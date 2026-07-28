//! Editors <-> Calendar (Pillar 2) interop proofs — WP-KERNEL-012 MT-067 (cluster E10).
//!
//! This suite proves the editors <-> Calendar edge through unit fixtures, a counted MT-019 backend mock,
//! an in-process mock HTTP server, egui_kittest, and an opt-in live backend/PostgreSQL proof.
//!
//! ## Backend reality
//!
//! handshake_core exposes live `GET /calendar/events` and `GET /calendar/activity-spans` routes. The
//! service still maps an unavailable route to `InteropError::EndpointUnavailable`, preserving the typed
//! empty-state path. `open_or_create_daily_note` delegates to the MT-019 daily-note service and remains
//! idempotent for one canonical document per workspace/date.
//!
//! Proof map:
//! - AC-1 / PT-2: `open_or_create_is_idempotent_and_delegates` — calling it twice for a date returns the
//!   SAME DocId and zero duplicate documents, proven against the MT-019 backend (no re-implemented creation).
//! - AC-2 / PT-3: `event_chip_click_emits_focus_calendar_event_on_bus` — when an event resolves, the panel
//!   renders the clickable CalendarEvent chip and a click emits `loom.daily-note.focus-calendar-event` on
//!   the WP-011 command bus.
//! - AC-3: `activity_strip_renders_read_only_chips_and_no_write` — the activity strip renders edited doc ids
//!   as read-only chips; a chip-click emits navigation only; the panel holds no ActivitySpan write path.
//! - AC-4 / PT-4: `activity_spans_404_is_typed_blocker_and_panel_stays_alive` — a simulated 404 on
//!   `/calendar/activity-spans` returns `InteropError::EndpointUnavailable` and the panel renders the typed
//!   empty-state while the daily-note binding stays functional; `events_404_is_typed_blocker` covers events.
//! - AC-5: `no_sqlite_no_backend_edit` — the production source has no sqlite/rusqlite/diesel and is GET-only,
//!   reusing the shared backend pool; `assert_no_local_artifact_dir` guards artifact hygiene (CX-212E).
//! - AC-6: `daily_journal_panel_accesskit_nodes_present` (+ screenshot) — the live AccessKit tree carries
//!   `daily-journal-panel` (GenericContainer), `daily-journal-date-header` (Label),
//!   `daily-journal-calendar-event-chip` (Button), and `daily-journal-activity-strip` (List) with the right
//!   roles + nesting, plus the reused MT-019 widget under its collision-free `daily-journal-*` address
//!   set; saves a screenshot to the EXTERNAL artifact root.
//! - AC-6 (command surface): `daily_note_command_ids_registered` — the three `loom.daily-note.*` /
//!   `loom.activity.*` command ids are present in the palette catalog exactly once each.
//! - PT-5: covered by `daily_journal_panel_accesskit_nodes_present` (the AccessKit tree snapshot).

use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;

use chrono::{NaiveDate, TimeZone, Utc};
use egui_kittest::kittest::{NodeT, Queryable};
#[path = "native_gui_support/canonical_argus_driver.rs"]
mod canonical_argus_driver;
#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;
use screenshot_harness::ScreenshotHarness as Harness;

use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend_client::HealthInfo;
use handshake_native::graph::daily_journal_panel::{
    activity_item_author_id, ActivityCorrelation, CalendarProjectionState, CalendarReadFailure,
    DailyJournalEvent, DailyJournalPanel, DailyJournalState,
    DAILY_JOURNAL_ACTIVITY_STRIP_AUTHOR_ID, DAILY_JOURNAL_CALENDAR_EVENT_CHIP_AUTHOR_ID,
    DAILY_JOURNAL_DATE_HEADER_AUTHOR_ID, DAILY_JOURNAL_LEGACY_BADGE_AUTHOR_ID,
    DAILY_JOURNAL_PANEL_AUTHOR_ID,
};
use handshake_native::interop::calendar_interop::{
    CalendarEventTemporal, CMD_OPEN_DOCUMENT as CMD_ACTIVITY_OPEN_DOCUMENT,
};
use handshake_native::interop::{
    ActivitySpan, CalendarEvent, CalendarInteropService, DocId, InteropError,
    CMD_FOCUS_CALENDAR_EVENT, CMD_OPEN_DAILY_NOTE_FOR_DATE,
};
use handshake_native::pane_registry::{
    DirtyState, LockState, PaneAuthority, PaneId, PaneRecord, PaneType,
};
use handshake_native::rich_editor::daily_notes::date_nav::{
    DateNav, DAILY_JOURNAL_DATE_NAV_AUTHOR_IDS, NEXT_DAY_ID,
};
use handshake_native::rich_editor::daily_notes::journal_store::{
    JournalBackend, JournalBlock, JournalDocLoad, JournalError, JournalFuture,
};
use handshake_native::tab_bar::TabState;
use handshake_native::theme::HsTheme;

#[path = "interconnect_support/mod.rs"]
mod interconnect_support;

struct ForcedOwnedBackendEnv {
    prior_binding_root: Option<std::ffi::OsString>,
    root: PathBuf,
}

impl ForcedOwnedBackendEnv {
    fn install() -> Self {
        let root = std::env::temp_dir().join(format!(
            "mt067-owned-backend-{}",
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir(&root).expect("create MT-067 owned-backend binding root");
        let prior_binding_root = std::env::var_os("HANDSHAKE_TEST_STAGE_BINDING_ROOT");
        std::env::set_var("HANDSHAKE_TEST_STAGE_BINDING_ROOT", &root);
        Self {
            prior_binding_root,
            root,
        }
    }
}

impl Drop for ForcedOwnedBackendEnv {
    fn drop(&mut self) {
        match self.prior_binding_root.take() {
            Some(value) => std::env::set_var("HANDSHAKE_TEST_STAGE_BINDING_ROOT", value),
            None => std::env::remove_var("HANDSHAKE_TEST_STAGE_BINDING_ROOT"),
        }
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn sha256_file(path: &Path) -> String {
    let bytes = std::fs::read(path).unwrap_or_else(|error| {
        panic!("read current-source proof file {}: {error}", path.display())
    });
    format!("{:x}", Sha256::digest(bytes))
}

struct LiveWorkspaceGuard<'a> {
    backend: &'a interconnect_support::LiveBackend,
    workspace_id: String,
    native_fr_event_ids: Vec<String>,
    cleaned: bool,
}

impl LiveWorkspaceGuard<'_> {
    fn track_native_fr(&mut self, row: &serde_json::Value) {
        let event_id = row["event_id"]
            .as_str()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| panic!("MT-067 native FR row lacks event_id: {row}"));
        uuid::Uuid::parse_str(event_id).expect("MT-067 native FR event_id is a UUID");
        if !self.native_fr_event_ids.iter().any(|id| id == event_id) {
            self.native_fr_event_ids.push(event_id.to_owned());
        }
    }

    fn cleanup_native_fr_ledger(&mut self) {
        let rows = self
            .backend
            .get_json(&format!("/api/flight_recorder?wsid={}", self.workspace_id));
        if let Some(rows) = rows.as_array() {
            for row in rows {
                if row["event_id"].as_str().is_some() {
                    self.track_native_fr(row);
                }
            }
        }
        if !self.native_fr_event_ids.is_empty() {
            let keys = self
                .native_fr_event_ids
                .iter()
                .flat_map(|event_id| {
                    [
                        format!("native-editor-fr-pending:{event_id}"),
                        format!("native-editor-fr-complete:{event_id}"),
                    ]
                })
                .map(|key| sql_literal(&key))
                .collect::<Vec<_>>()
                .join(", ");
            let sql = format!(
                "BEGIN; \
                 DELETE FROM kernel_event_ledger WHERE idempotency_key IN ({keys}); \
                 DO $$ BEGIN \
                   IF EXISTS (SELECT 1 FROM kernel_event_ledger WHERE idempotency_key IN ({keys})) THEN \
                     RAISE EXCEPTION 'MT-067 native FR EventLedger cleanup left exact rows behind'; \
                   END IF; \
                 END $$; \
                 COMMIT;"
            );
            self.backend
                .run_fixture_sql("mt067-native-fr-ledger-cleanup", &sql);
        }
        self.native_fr_event_ids.clear();
    }

    fn assert_cleanup(&mut self) {
        if self.cleaned {
            return;
        }
        let ledger_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.cleanup_native_fr_ledger();
        }));
        let status = self.backend.delete_workspace(&self.workspace_id);
        self.cleaned = true;
        assert!(
            (200..300).contains(&status) || status == 404,
            "MT-067 workspace cleanup {} returned {status}",
            self.workspace_id
        );
        if let Err(payload) = ledger_result {
            std::panic::resume_unwind(payload);
        }
    }
}

impl Drop for LiveWorkspaceGuard<'_> {
    fn drop(&mut self) {
        if self.cleaned {
            return;
        }
        if std::thread::panicking() {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                self.assert_cleanup();
            }));
            if result.is_err() {
                eprintln!("WARN(MT-067 cleanup): best-effort cleanup failed during unwind");
            }
        } else {
            self.assert_cleanup();
        }
    }
}

fn sql_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn exact_pg_journal_rows(workspace_id: &str, journal_date: NaiveDate) -> String {
    let database_url = [
        "HANDSHAKE_TEST_PG_DSN",
        "HSK_PROOF_DATABASE_URL",
        "POSTGRES_TEST_URL",
        "DATABASE_URL",
    ]
    .into_iter()
    .find_map(|name| {
        std::env::var(name)
            .ok()
            .filter(|value| !value.trim().is_empty())
    })
    .expect("MT-067 journal diagnostics require the managed PostgreSQL DSN");
    let query = format!(
        "SELECT COUNT(*)::text || '|' || COALESCE(string_agg(block_id, ',' ORDER BY block_id), '') \
         FROM loom_blocks WHERE workspace_id = {} AND content_type = 'journal' AND journal_date = {};",
        sql_literal(workspace_id),
        sql_literal(&journal_date.format("%Y-%m-%d").to_string()),
    );
    let psql = std::env::var_os("HSK_PSQL_BIN").unwrap_or_else(|| "psql".into());
    let mut command = Command::new(psql);
    command
        .arg("--no-psqlrc")
        .arg("--set")
        .arg("ON_ERROR_STOP=1")
        .arg("--tuples-only")
        .arg("--no-align")
        .arg("--dbname")
        .arg(database_url)
        .arg("--command")
        .arg(query)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("PGCONNECT_TIMEOUT", "5");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let mut child = command
        .spawn()
        .expect("start bounded MT-067 journal diagnostic query");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if std::time::Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                panic!("MT-067 journal diagnostic query exceeded ten seconds and was reaped");
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                panic!("poll MT-067 journal diagnostic query: {error}");
            }
        }
    };
    let mut stdout = String::new();
    child
        .stdout
        .take()
        .expect("capture MT-067 journal diagnostic stdout")
        .read_to_string(&mut stdout)
        .expect("read MT-067 journal diagnostic stdout");
    let mut stderr = String::new();
    child
        .stderr
        .take()
        .expect("capture MT-067 journal diagnostic stderr")
        .read_to_string(&mut stderr)
        .expect("read MT-067 journal diagnostic stderr");
    assert!(
        status.success(),
        "MT-067 journal diagnostic query failed with {status}: {stderr}"
    );
    stdout.trim().to_owned()
}

fn assert_exact_pg_journal_identity(
    workspace_id: &str,
    journal_date: NaiveDate,
    expected_block_id: &DocId,
) {
    let rows = exact_pg_journal_rows(workspace_id, journal_date);
    let (count, block_ids) = rows
        .split_once('|')
        .unwrap_or_else(|| panic!("invalid count|block_ids PostgreSQL proof: {rows}"));
    assert_eq!(count, "1", "exactly one durable journal row: {rows}");
    assert_eq!(
        block_ids,
        expected_block_id.as_str(),
        "the sole durable journal row is the returned binding"
    );
}

/// Explicit legacy-row fixture. This intentionally bypasses Calendar Workflow
/// only to prove the typed legacy recovery UI; canonical ingest authority is
/// covered by the backend Calendar Workflow managed-PG suite.
fn seed_explicit_legacy_calendar_fixture(
    backend: &interconnect_support::LiveBackend,
    workspace_id: &str,
    source_id: &str,
    event_id: &str,
    title: &str,
    date: NaiveDate,
) {
    let start = format!("{} 09:00:00", date.format("%Y-%m-%d"));
    let end = format!("{} 10:00:00", date.format("%Y-%m-%d"));
    let sql = format!(
        "BEGIN; \
         DO $$ BEGIN \
           IF NOT EXISTS (SELECT 1 FROM workspaces WHERE id = {workspace}) THEN \
             RAISE EXCEPTION 'MT-067 proof DSN is not the database attached to the live backend'; \
           END IF; \
         END $$; \
         INSERT INTO calendar_sources \
           (id, workspace_id, display_name, provider_type, write_policy, default_tzid, config_json) \
         VALUES ({source}, {workspace}, 'MT-067 live fixture', 'local', 'read_only_import', 'UTC', '{{}}') \
         ON CONFLICT (id) DO NOTHING; \
         INSERT INTO calendar_events \
           (id, workspace_id, source_id, title, start_ts_utc, end_ts_utc, tzid, status, visibility, export_mode, temporal_contract_version) \
         VALUES ({event}, {workspace}, {source}, {title}, \
                 TIMESTAMP {start}, TIMESTAMP {end}, \
                 'UTC', 'confirmed', 'private', 'full_export', NULL) \
         ON CONFLICT (id) DO NOTHING; \
         COMMIT;",
        source = sql_literal(source_id),
        workspace = sql_literal(workspace_id),
        event = sql_literal(event_id),
        title = sql_literal(title),
        start = sql_literal(&start),
        end = sql_literal(&end),
    );
    backend.run_fixture_sql("mt067-calendar-event", &sql);
}

fn wait_for_calendar_fr(
    backend: &interconnect_support::LiveBackend,
    workspace_id: &str,
    kind: &str,
    matches_fixture: impl Fn(&serde_json::Value) -> bool,
) -> serde_json::Value {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let rows = backend.get_json(&format!("/api/flight_recorder?wsid={workspace_id}"));
        if let Some(row) = rows.as_array().and_then(|rows| {
            rows.iter()
                .find(|row| row["payload"]["kind"].as_str() == Some(kind) && matches_fixture(row))
        }) {
            assert!(row["event_id"].as_str().is_some());
            assert_eq!(row["payload"]["workspace_id"].as_str(), Some(workspace_id));
            return row.clone();
        }
        assert!(
            std::time::Instant::now() < deadline,
            "automatic {kind} Flight Recorder row did not arrive within five seconds"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

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
// In-process mock HTTP server (the PROVEN MT-066 TcpListener pattern — no new dependency).
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// Spin up a one-shot mock server that replies with `status_line` + `body` to the FIRST request, and
/// captures that request's line. Returns (base_url, join handle delivering the request line).
fn spawn_mock(
    status_line: &'static str,
    body: serde_json::Value,
) -> (String, std::thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock server");
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{addr}");
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let request_line = read_request_line(&mut stream);
        let body_str = body.to_string();
        let response = format!(
            "{status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body_str}",
            body_str.len()
        );
        let _ = stream.write_all(response.as_bytes());
        let _ = stream.flush();
        request_line
    });
    (base_url, handle)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MountedEventMode {
    TransientThenEvent,
    NormalizedOverlap,
    Empty,
    AlwaysNotFound,
    AlwaysUnavailable,
    TransientThenEmpty,
    EventThenActivityNotFound,
    EventThenActivityUnavailable,
    MalformedEvent,
    JournalUnavailable,
}

static MOUNTED_SERVER_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn mounted_server_test_guard() -> std::sync::MutexGuard<'static, ()> {
    MOUNTED_SERVER_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

struct MountedServerCounts {
    event_reads: Arc<std::sync::atomic::AtomicUsize>,
    journal_puts: Arc<std::sync::atomic::AtomicUsize>,
    activity_reads: Arc<std::sync::atomic::AtomicUsize>,
    native_fr_posts: Arc<std::sync::atomic::AtomicUsize>,
    event_bound_fr_posts: Arc<std::sync::atomic::AtomicUsize>,
    activity_fr_posts: Arc<std::sync::atomic::AtomicUsize>,
    request_lines: Arc<std::sync::Mutex<Vec<String>>>,
}

fn assert_counter_settles_exact(
    counter: &std::sync::atomic::AtomicUsize,
    expected: usize,
    label: &str,
) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while counter.load(std::sync::atomic::Ordering::Acquire) < expected {
        assert!(
            std::time::Instant::now() < deadline,
            "{label} did not settle to {expected}; observed {}",
            counter.load(std::sync::atomic::Ordering::Acquire)
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(
        counter.load(std::sync::atomic::Ordering::Acquire),
        expected,
        "{label} exact settled count"
    );
    std::thread::sleep(std::time::Duration::from_millis(75));
    assert_eq!(
        counter.load(std::sync::atomic::Ordering::Acquire),
        expected,
        "{label} must remain stable after terminal state"
    );
}

fn spawn_transient_mounted_calendar_server(
    workspace_id: &str,
    date: NaiveDate,
    mode: MountedEventMode,
) -> (
    String,
    Arc<std::sync::atomic::AtomicBool>,
    MountedServerCounts,
    std::thread::JoinHandle<()>,
) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind transient calendar server");
    listener
        .set_nonblocking(true)
        .expect("set transient calendar server nonblocking");
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let event_reads = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let journal_puts = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let activity_reads = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let native_fr_posts = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let event_bound_fr_posts = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let activity_fr_posts = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let request_lines = Arc::new(std::sync::Mutex::new(Vec::new()));
    let server_stop = Arc::clone(&stop);
    let server_event_reads = Arc::clone(&event_reads);
    let server_journal_puts = Arc::clone(&journal_puts);
    let server_activity_reads = Arc::clone(&activity_reads);
    let server_native_fr_posts = Arc::clone(&native_fr_posts);
    let server_event_bound_fr_posts = Arc::clone(&event_bound_fr_posts);
    let server_activity_fr_posts = Arc::clone(&activity_fr_posts);
    let server_request_lines = Arc::clone(&request_lines);
    let workspace_id = workspace_id.to_owned();
    let date_label = date.format("%Y-%m-%d").to_string();
    let handle = std::thread::spawn(move || {
        // The mounted app can open unrelated backend connections while the Calendar request is in
        // flight. Handle accepted sockets concurrently so one slow/partial request cannot head-of-line
        // block a later Calendar retry in the listener backlog and turn a deterministic HTTP-503 probe
        // into an unrelated client transport failure.
        std::thread::scope(|scope| {
            while !server_stop.load(std::sync::atomic::Ordering::Acquire) {
                let (mut stream, _) = match listener.accept() {
                    Ok(accepted) => accepted,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(std::time::Duration::from_millis(5));
                        continue;
                    }
                    Err(error) => panic!("accept transient calendar request: {error}"),
                };
                let server_event_reads = Arc::clone(&server_event_reads);
                let server_journal_puts = Arc::clone(&server_journal_puts);
                let server_activity_reads = Arc::clone(&server_activity_reads);
                let server_native_fr_posts = Arc::clone(&server_native_fr_posts);
                let server_event_bound_fr_posts = Arc::clone(&server_event_bound_fr_posts);
                let server_activity_fr_posts = Arc::clone(&server_activity_fr_posts);
                let server_request_lines = Arc::clone(&server_request_lines);
                let workspace_id = workspace_id.clone();
                let date_label = date_label.clone();
                scope.spawn(move || {
                    stream
                        .set_read_timeout(Some(std::time::Duration::from_millis(250)))
                        .expect("bound transient calendar request read");
                    stream
                        .set_write_timeout(Some(std::time::Duration::from_secs(2)))
                        .expect("bound transient calendar response write");
                    let (request_line, request_body) = read_http_request(&mut stream);
                    if request_line.is_empty() {
                        return;
                    }
                    server_request_lines
                        .lock()
                        .unwrap()
                .push(request_line.clone());
            let (status_line, body) = if request_line.starts_with("PUT ")
                && request_line.contains("/loom/journals/")
            {
                server_journal_puts.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
                if mode == MountedEventMode::JournalUnavailable {
                    ("HTTP/1.1 503 Service Unavailable", serde_json::json!({}))
                } else {
                    (
                        "HTTP/1.1 200 OK",
                        serde_json::json!({
                            "block_id": "DOC-TRANSIENT-DATE-B",
                            "workspace_id": workspace_id,
                            "content_type": "journal",
                            "document_id": null,
                            "title": format!("Daily Note {date_label}"),
                            "journal_date": date_label,
                        }),
                    )
                }
            } else if request_line.starts_with("GET ") && request_line.contains("/calendar/events?")
            {
                let read = server_event_reads.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
                match mode {
                    MountedEventMode::Empty => ("HTTP/1.1 200 OK", serde_json::json!([])),
                    MountedEventMode::AlwaysNotFound => {
                        ("HTTP/1.1 404 Not Found", serde_json::json!({}))
                    }
                    MountedEventMode::AlwaysUnavailable => {
                        ("HTTP/1.1 503 Service Unavailable", serde_json::json!({}))
                    }
                    MountedEventMode::TransientThenEmpty => {
                        if read == 0 {
                            // Hold the first response after its counted arrival so the mounted test can
                            // deterministically navigate before the worker observes 503 and enters retry
                            // backoff. This attacks generation cancellation instead of scheduler speed.
                            std::thread::sleep(std::time::Duration::from_secs(2));
                            ("HTTP/1.1 503 Service Unavailable", serde_json::json!({}))
                        } else {
                            ("HTTP/1.1 200 OK", serde_json::json!([]))
                        }
                    }
                    MountedEventMode::TransientThenEvent => {
                        if read == 0 {
                            ("HTTP/1.1 503 Service Unavailable", serde_json::json!({}))
                        } else {
                            (
                                "HTTP/1.1 200 OK",
                                serde_json::json!([{
                                    "id": "EVENT-TRANSIENT-DATE-B",
                                    "title": "Transient retry event",
                                    "temporal": {
                                        "kind": "timed",
                                        "start_utc": format!("{date_label}T09:00:00Z"),
                                        "end_utc": format!("{date_label}T10:00:00Z"),
                                        "start_local": format!("{date_label}T09:00:00"),
                                        "end_local": format!("{date_label}T10:00:00"),
                                        "tzid": "UTC",
                                        "was_floating": false,
                                        "normalization_note": null
                                    },
                                    "daily_note_doc_id": "DOC-STALE-START-DATE",
                                }]),
                            )
                        }
                    }
                    MountedEventMode::NormalizedOverlap => (
                        "HTTP/1.1 200 OK",
                        serde_json::json!([{
                            "id": "EVENT-TRANSIENT-DATE-B",
                            "title": "DST overlap proof",
                            "temporal": {
                                "kind": "timed",
                                "start_utc": "2026-10-25T00:30:00Z",
                                "end_utc": "2026-10-25T02:30:00Z",
                                "start_local": "2026-10-25T02:30:00",
                                "end_local": "2026-10-25T03:30:00",
                                "tzid": "Europe/Brussels",
                                "was_floating": false,
                                "normalization_note": {
                                    "boundaries": [{
                                        "boundary": "start",
                                        "original_local": "2026-10-25T02:30:00",
                                        "resolution": "earlier_offset",
                                        "resolved_utc": "2026-10-25T00:30:00Z"
                                    }]
                                }
                            },
                            "daily_note_doc_id": "DOC-STALE-START-DATE"
                        }]),
                    ),
                    MountedEventMode::EventThenActivityNotFound
                    | MountedEventMode::EventThenActivityUnavailable => (
                        "HTTP/1.1 200 OK",
                        serde_json::json!([{
                            "id": "EVENT-ACTIVITY-FAIL",
                            "title": "Preserved activity failure event",
                            "temporal": {
                                "kind": "timed",
                                "start_utc": format!("{date_label}T09:00:00Z"),
                                "end_utc": format!("{date_label}T10:00:00Z"),
                                "start_local": format!("{date_label}T09:00:00"),
                                "end_local": format!("{date_label}T10:00:00"),
                                "tzid": "UTC",
                                "was_floating": false,
                                "normalization_note": null
                            },
                            "daily_note_doc_id": "DOC-STALE-START-DATE",
                        }]),
                    ),
                    MountedEventMode::MalformedEvent => {
                        ("HTTP/1.1 200 OK", serde_json::json!("not-an-event-list"))
                    }
                    MountedEventMode::JournalUnavailable => {
                        ("HTTP/1.1 500 Internal Server Error", serde_json::json!({}))
                    }
                }
            } else if request_line.starts_with("GET ")
                && request_line.contains("/calendar/activity-spans?")
            {
                server_activity_reads.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
                match mode {
                    MountedEventMode::EventThenActivityNotFound => {
                        ("HTTP/1.1 404 Not Found", serde_json::json!({}))
                    }
                    MountedEventMode::EventThenActivityUnavailable => {
                        ("HTTP/1.1 503 Service Unavailable", serde_json::json!({}))
                    }
                    _ => ("HTTP/1.1 200 OK", serde_json::json!([])),
                }
            } else if request_line.starts_with("POST ")
                && request_line.contains("/flight_recorder/native_editor_event")
            {
                server_native_fr_posts.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
                if request_body.contains("\"kind\":\"calendar_event_bound\"") {
                    server_event_bound_fr_posts.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
                }
                if request_body.contains("\"kind\":\"activity_span_correlated\"") {
                    server_activity_fr_posts.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
                }
                ("HTTP/1.1 200 OK", serde_json::json!({"ok": true}))
            } else {
                ("HTTP/1.1 200 OK", serde_json::json!({}))
                    };
                    let body = body.to_string();
                    // Keep the counted retry contract independent of host scheduling. A short
                    // keep-alive read timeout can otherwise turn the next planned HTTP response
                    // into an uncounted transport error when the machine is under heavy load.
                    let response = format!(
                        "{status_line}\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}",
                        body.len()
                    );
                    let _ = stream
                        .write_all(response.as_bytes())
                        .and_then(|()| stream.flush());
                });
            }
        });
    });
    (
        base_url,
        stop,
        MountedServerCounts {
            event_reads,
            journal_puts,
            activity_reads,
            native_fr_posts,
            event_bound_fr_posts,
            activity_fr_posts,
            request_lines,
        },
        handle,
    )
}

/// Read one HTTP request's request line off the stream (a GET has no body).
fn read_request_line(stream: &mut std::net::TcpStream) -> String {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 4096];
    loop {
        let n = stream.read(&mut tmp).unwrap_or(0);
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
        if String::from_utf8_lossy(&buf).contains("\r\n\r\n") {
            break;
        }
    }
    let text = String::from_utf8_lossy(&buf).to_string();
    text.lines().next().unwrap_or("").to_string()
}

fn read_http_request(stream: &mut std::net::TcpStream) -> (String, String) {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 4096];
    let mut header_end = None;
    let mut content_length = 0usize;
    loop {
        let n = stream.read(&mut tmp).unwrap_or(0);
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
        if header_end.is_none() {
            if let Some(offset) = buf.windows(4).position(|window| window == b"\r\n\r\n") {
                let end = offset + 4;
                let headers = String::from_utf8_lossy(&buf[..end]);
                content_length = headers
                    .lines()
                    .find_map(|line| {
                        line.strip_prefix("Content-Length: ")
                            .or_else(|| line.strip_prefix("content-length: "))
                    })
                    .and_then(|value| value.trim().parse().ok())
                    .unwrap_or(0);
                header_end = Some(end);
            }
        }
        if header_end.is_some_and(|end| buf.len() >= end + content_length) {
            break;
        }
    }
    let request = String::from_utf8_lossy(&buf);
    let line = request.lines().next().unwrap_or_default().to_owned();
    let body = header_end
        .map(|end| String::from_utf8_lossy(&buf[end..]).into_owned())
        .unwrap_or_default();
    (line, body)
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

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

// ── A counted MT-019 backend mock (proves delegation + idempotency, RISK-1). ───────────────────────

/// A counted mock MT-019 backend: `open_daily_journal` returns the SAME deterministic block for a given
/// date (the real backend's get-or-create idempotency) and counts how many times it was called. NEVER
/// creates a second block for the same date.
struct CountingJournalBackend {
    opens: std::sync::atomic::AtomicUsize,
    document_id: Option<String>,
}

impl CountingJournalBackend {
    fn new(document_id: Option<&str>) -> Self {
        Self {
            opens: std::sync::atomic::AtomicUsize::new(0),
            document_id: document_id.map(|s| s.to_owned()),
        }
    }
}

impl JournalBackend for CountingJournalBackend {
    fn open_daily_journal<'a>(
        &'a self,
        workspace_id: &'a str,
        journal_date: &'a str,
    ) -> JournalFuture<'a, JournalBlock> {
        self.opens.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let ws = workspace_id.to_owned();
        let date = journal_date.to_owned();
        let document_id = self.document_id.clone();
        Box::pin(async move {
            Ok(JournalBlock {
                block_id: format!("journal-{date}"),
                workspace_id: ws,
                content_type: Some("journal".to_owned()),
                document_id,
                title: Some(format!("Daily Note {date}")),
                journal_date: Some(date),
            })
        })
    }

    fn load_document<'a>(&'a self, _document_id: &'a str) -> JournalFuture<'a, JournalDocLoad> {
        Box::pin(async move { Err(JournalError::DocLoadFailed("unused".into())) })
    }

    fn create_document<'a>(
        &'a self,
        _workspace_id: &'a str,
        _title: &'a str,
    ) -> JournalFuture<'a, JournalDocLoad> {
        Box::pin(async move { Err(JournalError::CreateFailed("unused".into())) })
    }
}

fn event(id: &str, title: &str) -> CalendarEvent {
    CalendarEvent {
        id: id.to_owned(),
        title: title.to_owned(),
        temporal: CalendarEventTemporal::Timed {
            start_utc: Utc.with_ymd_and_hms(2026, 6, 21, 9, 0, 0).unwrap(),
            end_utc: Utc.with_ymd_and_hms(2026, 6, 21, 10, 0, 0).unwrap(),
            start_local: "2026-06-21T09:00:00".into(),
            end_local: "2026-06-21T10:00:00".into(),
            tzid: "UTC".into(),
            was_floating: false,
            normalization_note: None,
        },
        daily_note_doc_id: None,
        view_tzid: "UTC".into(),
    }
}

fn span(id: &str, docs: &[&str]) -> ActivitySpan {
    ActivitySpan {
        span_id: id.to_owned(),
        calendar_event_id: Some("E-1".to_owned()),
        started_utc: Utc.with_ymd_and_hms(2026, 6, 21, 9, 5, 0).unwrap(),
        ended_utc: Some(Utc.with_ymd_and_hms(2026, 6, 21, 9, 45, 0).unwrap()),
        edited_doc_ids: docs.iter().map(|s| DocId((*s).to_owned())).collect(),
    }
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// AC-1 / PT-2 — open-or-create is idempotent and DELEGATES to the MT-019 daily-note service.
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn open_or_create_is_idempotent_and_delegates() {
    let backend = Arc::new(CountingJournalBackend::new(Some("DOC-2026-06-21")));
    let svc = CalendarInteropService::with_base_url("http://unused", "WS-1", backend.clone());
    let date = d(2026, 6, 21);
    let (a, b) = rt().block_on(async {
        let a = svc
            .open_or_create_daily_note(date)
            .await
            .expect("first open");
        let b = svc
            .open_or_create_daily_note(date)
            .await
            .expect("second open");
        (a, b)
    });
    // AC-1: same date -> same DocId, zero duplicate documents (idempotent get-or-create).
    assert_eq!(a.doc_id, b.doc_id, "AC-1: same date -> same DocId");
    assert_eq!(a.doc_id, DocId("DOC-2026-06-21".to_owned()));
    assert_eq!(a.date, date);
    // PT-2 / RISK-1: the MT-019 backend was the creation path (delegated, not re-implemented) — called
    // exactly twice (once per open), never spawning a second block for the date.
    assert_eq!(
        backend.opens.load(std::sync::atomic::Ordering::SeqCst),
        2,
        "PT-2: open-or-create delegated to the MT-019 daily-note service both times"
    );
    println!("AC-1/PT-2 OK: idempotent open-or-create delegates to MT-019, single doc/date");
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// AC-1 / PT-2 (LIVE) — the REAL daily-note PUT round-trip against managed PG.
//
// The contract's "REAL and FULLY PROVABLE" daily-note half (MT-067.json implementation_notes line 84:
// "the LIVE daily-note PUT round-trip (real PG)") must touch the REAL
// MT-019 resource — the `PUT /workspaces/{ws}/loom/journals/{date}` route backed by
// PostgreSQL/EventLedger — not only the implementer-authored CountingJournalBackend mock above
// (Spec-Realism Gate Sub-rule 2: "a trait abstraction plus an in-memory impl this role also authored
// does not count as touching the resource"). This test builds the SAME CalendarInteropService with a
// REAL `ReqwestJournalBackend` (the production MT-019 transport) pointed at a managed handshake_core +
// PostgreSQL, calls `open_or_create_daily_note` TWICE for one date, and asserts identical DocId and a
// single durable journal block per date — the idempotent get-or-create against the real route. It is
// The WP validation lane supplies the managed backend + exact proof DSN. Fixture setup uses that DSN only
// after proving the HTTP-created workspace exists there; all measured behavior uses production routes.
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn open_or_create_daily_note_is_idempotent_against_real_pg_live() {
    use handshake_native::rich_editor::daily_notes::journal_store::ReqwestJournalBackend;

    let _server_guard = mounted_server_test_guard();
    let _owned_backend_env = ForcedOwnedBackendEnv::install();
    let backend_binary = PathBuf::from(
        std::env::var_os("HSK_TEST_BACKEND_BIN")
            .expect("exact-source proof requires HSK_TEST_BACKEND_BIN"),
    );
    let binary_hash_before = sha256_file(&backend_binary);
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("native crate repository root");
    let source_paths = [
        repo_root.join("src/backend/handshake_core/src/storage/calendar.rs"),
        repo_root.join("src/backend/handshake_core/src/storage/postgres.rs"),
        repo_root.join("src/backend/handshake_core/src/workflows.rs"),
        repo_root.join("src/backend/handshake_core/src/api/calendar.rs"),
        repo_root.join(
            "src/backend/handshake_core/migrations/0353_calendar_lossless_temporal_contract.sql",
        ),
    ];
    let source_hashes_before = source_paths
        .iter()
        .map(|path| (path.clone(), sha256_file(path)))
        .collect::<Vec<_>>();
    let live = interconnect_support::require_reachable_backend();
    assert_eq!(
        sha256_file(&backend_binary),
        binary_hash_before,
        "the spawned backend executable identity cannot change during proof startup"
    );
    for (path, expected_hash) in source_hashes_before {
        assert_eq!(
            sha256_file(&path),
            expected_hash,
            "current-source hash drift: {}",
            path.display()
        );
    }
    let workspace = live.create_workspace(&format!(
        "mt067-live-calendar-{}",
        uuid::Uuid::new_v4().simple()
    ));
    let workspace_id = workspace["id"]
        .as_str()
        .expect("workspace create returns id")
        .to_owned();
    let mut cleanup = LiveWorkspaceGuard {
        backend: &live,
        workspace_id: workspace_id.clone(),
        native_fr_event_ids: Vec::new(),
        cleaned: false,
    };
    let source_id = format!("cal-src-{}", uuid::Uuid::new_v4().simple());
    let event_id = format!("cal-evt-{}", uuid::Uuid::new_v4().simple());
    let event_title = "MT-067 persisted CalendarEvent";
    // Deliberately avoid today: the former host hardcoded `Utc::now().date_naive()`, so a live proof
    // seeded only for today could pass while every operator-selected date remained broken.
    let date = Utc::now()
        .date_naive()
        .checked_add_days(chrono::Days::new(7))
        .expect("non-today MT-067 date");
    seed_explicit_legacy_calendar_fixture(
        &live,
        &workspace_id,
        &source_id,
        &event_id,
        event_title,
        date,
    );

    // The REAL MT-019 daily-note transport issues PUT /loom/journals/:date against this isolated workspace.
    let journal_backend = Arc::new(ReqwestJournalBackend::new(live.base.clone()));
    let svc =
        CalendarInteropService::with_base_url(live.base.clone(), &workspace_id, journal_backend);
    // Call the SAME production open_or_create_daily_note twice for one date against the REAL route.
    let (a, b) = rt().block_on(async {
        let a = svc.open_or_create_daily_note(date).await.expect(
            "AC-1 LIVE: first open against the real PUT /loom/journals/:date route succeeds",
        );
        let b = svc
            .open_or_create_daily_note(date)
            .await
            .expect("AC-1 LIVE: second open against the real route succeeds");
        (a, b)
    });

    // Idempotency against managed PG: the same date maps to exactly ONE durable journal block/doc id,
    // so the two real round-trips return the SAME DocId (no duplicate journal block was created).
    assert_eq!(
        a.doc_id, b.doc_id,
        "AC-1 LIVE: open_or_create_daily_note twice for one date returns the SAME DocId from real PG \
         (single durable journal block per date — idempotent get-or-create, no duplicate)"
    );
    assert_eq!(
        a.date, date,
        "AC-1 LIVE: the binding carries the requested date"
    );
    assert!(
        !a.doc_id.as_str().trim().is_empty(),
        "AC-1 LIVE: the real route returns a non-empty stable doc/block id for the date"
    );
    assert_exact_pg_journal_identity(&workspace_id, date, &a.doc_id);

    // Resolve the exact persisted CalendarEvent through the production frontend service. The backend
    // projects the daily-note document id at read time, proving the bidirectional date/event binding.
    let resolved = rt()
        .block_on(svc.resolve_event_for_daily_note(date))
        .expect("real calendar event query succeeds")
        .expect("seeded real CalendarEvent resolves for the daily note");
    assert_eq!(resolved.id, event_id);
    assert_eq!(resolved.title, event_title);
    assert!(
        resolved.is_legacy_incomplete(),
        "the direct-SQL fixture is explicitly typed legacy, never canonical ingest proof"
    );
    assert_eq!(resolved.daily_note_doc_id.as_ref(), Some(&a.doc_id));

    let span_id = format!("CAS-{}", uuid::Uuid::new_v4().simple());
    let first_span_id = span_id.clone();
    let span_started = format!("{}T09:05:00Z", date.format("%Y-%m-%d"));
    let span_ended = format!("{}T09:45:00Z", date.format("%Y-%m-%d"));
    let created_span = live.post_json(
        &format!("/workspaces/{workspace_id}/calendar/activity-spans"),
        &serde_json::json!({
            "calendar_event_id": event_id,
            "span_id": span_id,
            "started_utc": span_started,
            "ended_utc": span_ended,
            "edited_doc_ids": [a.doc_id.as_str(), "DOC-MT067-SECONDARY"],
        }),
    );
    assert_eq!(created_span["span_id"].as_str(), Some(span_id.as_str()));
    let spans = rt()
        .block_on(svc.activity_spans_for_event(&event_id))
        .expect("production activity-span read succeeds");
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].span_id, span_id);
    assert_eq!(
        spans[0].edited_doc_ids,
        vec![a.doc_id.clone(), DocId("DOC-MT067-SECONDARY".to_owned())]
    );

    // Extend the same explicit legacy fixture across the adjacent day. The backend projects its
    // selected-date journal id without inventing missing local intent, so
    // this is the live counterexample for the mounted host: date B must replace that projection with date
    // B's exact session binding rather than retain date A's document.
    let second_date = date
        .checked_add_days(chrono::Days::new(1))
        .expect("adjacent MT-067 date");
    let second_event_id = event_id.clone();
    let multi_day_end = format!("{} 10:00:00", second_date.format("%Y-%m-%d"));
    live.run_fixture_sql(
        "mt067-multi-day-calendar-event",
        &format!(
            "UPDATE calendar_events SET end_ts_utc = TIMESTAMP {multi_day_end} \
             WHERE workspace_id = {workspace} AND id = {event};",
            multi_day_end = sql_literal(&multi_day_end),
            workspace = sql_literal(&workspace_id),
            event = sql_literal(&event_id),
        ),
    );
    let direct_second_date_events = rt()
        .block_on(svc.events_for_range(second_date, second_date))
        .expect("direct date-B CalendarEvent range query succeeds after extending the event");
    assert!(
        direct_second_date_events
            .iter()
            .any(|event| event.id == second_event_id),
        "direct date-B CalendarEvent range query must contain the same multi-day event: {direct_second_date_events:?}"
    );
    let second_date_label = second_date.format("%Y-%m-%d").to_string();
    live.run_fixture_sql(
        "mt067-date-b-absent-before-ui",
        &format!(
            "DO $$ BEGIN \
             IF EXISTS (SELECT 1 FROM loom_blocks WHERE workspace_id = {workspace} \
                        AND content_type = 'journal' AND journal_date = {journal_date}) THEN \
               RAISE EXCEPTION 'MT-067 date-B journal must be absent before mounted navigation'; \
             END IF; \
             END $$;",
            workspace = sql_literal(&workspace_id),
            journal_date = sql_literal(&second_date_label),
        ),
    );
    let second_span_id = format!("CAS-{}", uuid::Uuid::new_v4().simple());
    let second_span_started = format!("{}T11:05:00Z", second_date.format("%Y-%m-%d"));
    let second_span_ended = format!("{}T11:45:00Z", second_date.format("%Y-%m-%d"));
    live.post_json(
        &format!("/workspaces/{workspace_id}/calendar/activity-spans"),
        &serde_json::json!({
            "calendar_event_id": second_event_id,
            "span_id": second_span_id,
            "started_utc": second_span_started,
            "ended_utc": second_span_ended,
            "edited_doc_ids": ["DOC-MT067-DAY-B"],
        }),
    );

    // Mount the actual journal pane in the production shell. The shell performs its own production
    // daily-note/event/span reads, populates the shared panel state, and automatically dispatches both
    // native-editor events. The test only drives the visible CalendarEvent chip and reads durable rows.
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("MT-067 mounted runtime");
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }));
    app.set_backend_base_url_for_test(&live.base, runtime.handle().clone());
    app.bind_active_project_for_integration_test(workspace_id.clone());
    let pane_id = PaneId::from("pane-a");
    app.pane_registry().lock().unwrap().insert(PaneRecord::new(
        pane_id.clone(),
        PaneType::LoomDailyJournal,
        workspace_id.clone(),
        Some(a.doc_id.as_str().to_owned()),
        LockState::Unlocked,
        DirtyState::Clean,
        PaneAuthority::System,
    ));
    let mut tab = TabState::new(PaneType::LoomDailyJournal);
    tab.content_id = Some(a.doc_id.as_str().to_owned());
    let bar = app
        .tab_bar_states_mut()
        .get_mut(&pane_id)
        .expect("default pane-a has a tab bar");
    bar.tabs = vec![tab];
    bar.active_index = 0;
    app.set_active_pane_for_test(Some(pane_id));
    let mounted = app.mounted_daily_journal();
    mounted.lock().unwrap().prepare_date(date);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1100.0, 760.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    let mounted_deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    loop {
        harness.run_steps(1);
        let loaded = mounted.lock().unwrap().clone();
        if loaded.event.as_ref().is_some_and(|event| {
            event.id == event_id && event.daily_note_doc_id.as_ref() == Some(&a.doc_id)
        }) && matches!(loaded.activity, ActivityCorrelation::Spans(ref rows) if rows.iter().any(|row| row.span_id == span_id))
        {
            break;
        }
        assert!(
            std::time::Instant::now() < mounted_deadline,
            "mounted journal did not load the exact CalendarEvent and ActivitySpan within fifteen seconds; last state: {loaded:?}"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    // Capture the date-A receipts before navigation. The same multi-day event/span is emitted again for
    // date B, so the final proof excludes these exact causal rows rather than accepting the first match.
    let first_date_label = date.format("%Y-%m-%d").to_string();
    let initial_bound_fr =
        wait_for_calendar_fr(&live, &workspace_id, "calendar_event_bound", |row| {
            row["payload"]["native_payload"]["calendar_event_id"].as_str()
                == Some(event_id.as_str())
                && row["payload"]["native_payload"]["date"].as_str()
                    == Some(first_date_label.as_str())
        });
    let initial_first_span_fr =
        wait_for_calendar_fr(&live, &workspace_id, "activity_span_correlated", |row| {
            row["payload"]["native_payload"]["calendar_event_id"].as_str()
                == Some(event_id.as_str())
                && row["payload"]["native_payload"]["activity_span_id"].as_str()
                    == Some(first_span_id.as_str())
        });
    let initial_second_span_fr =
        wait_for_calendar_fr(&live, &workspace_id, "activity_span_correlated", |row| {
            row["payload"]["native_payload"]["calendar_event_id"].as_str()
                == Some(event_id.as_str())
                && row["payload"]["native_payload"]["activity_span_id"].as_str()
                    == Some(second_span_id.as_str())
        });
    cleanup.track_native_fr(&initial_bound_fr);
    cleanup.track_native_fr(&initial_first_span_fr);
    cleanup.track_native_fr(&initial_second_span_fr);
    let initial_first_span_fr_id = initial_first_span_fr["event_id"]
        .as_str()
        .expect("initial date-A first activity event id")
        .to_owned();
    let initial_second_span_fr_id = initial_second_span_fr["event_id"]
        .as_str()
        .expect("initial date-A activity event id")
        .to_owned();

    // Both mounted DateNavWidgets have distinct, registry-backed identities. Each exact address occurs
    // once, so the proof never depends on AccessKit tree order.
    for author_id in [DAILY_JOURNAL_DATE_NAV_AUTHOR_IDS.next_day, NEXT_DAY_ID] {
        assert_eq!(
            harness
                .query_all_by(|node: &egui_kittest::kittest::AccessKitNode<'_>| {
                    node.author_id() == Some(author_id)
                })
                .count(),
            1,
            "mounted date-nav author id {author_id} must be collision-free"
        );
    }
    {
        let mut argus = canonical_argus_driver::CanonicalArgusDriver::bind(
            harness.state(),
            "mt067-live-date-navigation",
        );
        let before = argus.inspect(&mut harness);
        let observation = argus.click_from_snapshot_and_reinspect(
            &mut harness,
            DAILY_JOURNAL_DATE_NAV_AUTHOR_IDS.next_day,
            before,
        );
        assert!(
            matches!(
                observation.receipt_status.as_str(),
                "applied" | "indeterminate"
            ),
            "canonical date-navigation receipt is terminal: {observation:?}"
        );
        argus.finish();
    }
    let second_deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    loop {
        harness.run_steps(1);
        let loaded = mounted.lock().unwrap().clone();
        if loaded.nav.current == second_date
            && loaded.event.as_ref().is_some_and(|event| {
                event.id == second_event_id
                    && event
                        .daily_note_doc_id
                        .as_ref()
                        .is_some_and(|doc_id| doc_id != &a.doc_id)
            })
            && matches!(loaded.activity, ActivityCorrelation::Spans(ref rows) if rows.iter().any(|row| row.span_id == second_span_id))
        {
            break;
        }
        if std::time::Instant::now() >= second_deadline {
            let date_b_journal_rows = exact_pg_journal_rows(&workspace_id, second_date);
            panic!(
                "Next-day navigation did not replace date A with date B's exact event/span within fifteen seconds; \
                 last state: {loaded:?}; direct date-B CalendarEvent result: {direct_second_date_events:?}; \
                 exact PostgreSQL date-B journal count|block_ids: {date_b_journal_rows}"
            );
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    let ui_created_doc_id = mounted
        .lock()
        .unwrap()
        .event
        .as_ref()
        .and_then(|event| event.daily_note_doc_id.clone())
        .expect("mounted date-B navigation creates and binds the previously absent journal");
    let second_binding = rt()
        .block_on(svc.open_or_create_daily_note(second_date))
        .expect("date-B journal reopens idempotently after the mounted UI created it");
    assert_eq!(second_binding.doc_id, ui_created_doc_id);
    assert_exact_pg_journal_identity(&workspace_id, second_date, &second_binding.doc_id);
    assert_ne!(
        a.doc_id, second_binding.doc_id,
        "different dates retain distinct durable journal identities"
    );

    // The remainder of the live interaction proves the content-addressed destination and durable FR
    // receipts for the newly selected second date, not for the initial date or for today.
    let event_id = second_event_id;
    let span_id = second_span_id.clone();
    let date = second_date;
    let a = second_binding;
    // The loop can observe the async delivery immediately after the frame rendered its cleared
    // in-flight state. Render once more so AccessKit reflects the newly delivered date-B chip before
    // driving it.
    harness.run_steps(1);
    harness.get_by(|node| node.author_id() == Some(DAILY_JOURNAL_LEGACY_BADGE_AUTHOR_ID));
    harness
        .get_by(|node| node.author_id() == Some(DAILY_JOURNAL_CALENDAR_EVENT_CHIP_AUTHOR_ID))
        .click();
    harness.run_steps(2);

    let active_pane = harness
        .state()
        .active_pane()
        .cloned()
        .expect("CalendarEvent activation keeps an active pane");
    let active_tab = harness
        .state()
        .tab_bar_states()
        .get(&active_pane)
        .and_then(|bar| bar.tabs.get(bar.active_index))
        .expect("CalendarEvent activation produces an active tab");
    assert_eq!(active_tab.pane_type, PaneType::CalendarEvent);
    assert_eq!(active_tab.content_id.as_deref(), Some(event_id.as_str()));
    let details = harness.get_by(|node| {
        node.author_id()
            == Some(handshake_native::graph::daily_journal_panel::CALENDAR_EVENT_DETAILS_AUTHOR_ID)
    });
    assert!(
        details
            .accesskit_node()
            .value()
            .is_some_and(|value| value.contains(&event_id)),
        "the CalendarEvent Details destination exposes the exact clicked event id"
    );
    harness
        .get_by(|node| {
            node.author_id()
                == Some(
                    handshake_native::graph::daily_journal_panel::CALENDAR_EVENT_ACTIVITY_TAB_AUTHOR_ID,
                )
        })
        .click();
    harness.run_steps(2);
    let span_author_id =
        handshake_native::graph::daily_journal_panel::calendar_event_span_author_id(&span_id);
    harness.get_by(|node| node.author_id() == Some(span_author_id.as_str()));

    let calendar_event_fr =
        wait_for_calendar_fr(&live, &workspace_id, "calendar_event_bound", |row| {
            row["payload"]["native_payload"]["calendar_event_id"].as_str()
                == Some(event_id.as_str())
                && row["payload"]["native_payload"]["date"].as_str()
                    == Some(second_date_label.as_str())
        });
    let first_span_activity_fr =
        wait_for_calendar_fr(&live, &workspace_id, "activity_span_correlated", |row| {
            row["payload"]["native_payload"]["calendar_event_id"].as_str()
                == Some(event_id.as_str())
                && row["payload"]["native_payload"]["activity_span_id"].as_str()
                    == Some(first_span_id.as_str())
                && row["event_id"].as_str() != Some(initial_first_span_fr_id.as_str())
        });
    let activity_fr =
        wait_for_calendar_fr(&live, &workspace_id, "activity_span_correlated", |row| {
            row["payload"]["native_payload"]["calendar_event_id"].as_str()
                == Some(event_id.as_str())
                && row["payload"]["native_payload"]["activity_span_id"].as_str()
                    == Some(span_id.as_str())
                && row["event_id"].as_str() != Some(initial_second_span_fr_id.as_str())
        });
    cleanup.track_native_fr(&calendar_event_fr);
    cleanup.track_native_fr(&first_span_activity_fr);
    cleanup.track_native_fr(&activity_fr);
    assert_ne!(calendar_event_fr["event_id"], activity_fr["event_id"]);
    let bound_payload = &calendar_event_fr["payload"]["native_payload"];
    let date_label = date.format("%Y-%m-%d").to_string();
    assert_eq!(bound_payload["date"].as_str(), Some(date_label.as_str()));
    assert_eq!(
        bound_payload["calendar_event_id"].as_str(),
        Some(event_id.as_str())
    );
    let span_payload = &activity_fr["payload"]["native_payload"];
    assert_eq!(
        span_payload["activity_span_id"].as_str(),
        Some(span_id.as_str())
    );
    assert_eq!(
        span_payload["calendar_event_id"].as_str(),
        Some(event_id.as_str())
    );
    let bound_ts = chrono::DateTime::parse_from_rfc3339(
        calendar_event_fr["payload"]["ts_utc"]
            .as_str()
            .expect("calendar_event_bound ts_utc"),
    )
    .unwrap();
    let correlated_ts = chrono::DateTime::parse_from_rfc3339(
        activity_fr["payload"]["ts_utc"]
            .as_str()
            .expect("activity_span_correlated ts_utc"),
    )
    .unwrap();
    assert!(
        correlated_ts > bound_ts,
        "the exact ActivitySpan correlation must be strictly later than its exact CalendarEvent binding"
    );
    let all_fr = live.get_json(&format!("/api/flight_recorder?wsid={workspace_id}"));
    let all_fr = all_fr.as_array().expect("workspace FR rows are an array");
    let bound_rows = all_fr
        .iter()
        .filter(|row| {
            row["payload"]["kind"].as_str() == Some("calendar_event_bound")
                && row["payload"]["native_payload"]["calendar_event_id"].as_str()
                    == Some(event_id.as_str())
        })
        .collect::<Vec<_>>();
    assert_eq!(
        bound_rows.len(),
        2,
        "one accepted CalendarEvent binding per selected date A/B, no extras: {bound_rows:?}"
    );
    let bound_dates = bound_rows
        .iter()
        .filter_map(|row| row["payload"]["native_payload"]["date"].as_str())
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(
        bound_dates,
        std::collections::HashSet::from([first_date_label.as_str(), second_date_label.as_str()])
    );
    let activity_rows = all_fr
        .iter()
        .filter(|row| {
            row["payload"]["kind"].as_str() == Some("activity_span_correlated")
                && row["payload"]["native_payload"]["calendar_event_id"].as_str()
                    == Some(event_id.as_str())
        })
        .collect::<Vec<_>>();
    assert_eq!(
        activity_rows.len(),
        4,
        "two seeded spans each emit one accepted correlation per selected date A/B, no extras: {activity_rows:?}"
    );
    let activity_rows_by_span = activity_rows.iter().fold(
        std::collections::HashMap::<&str, usize>::new(),
        |mut counts, row| {
            let span_id = row["payload"]["native_payload"]["activity_span_id"]
                .as_str()
                .expect("every activity correlation has a span id");
            *counts.entry(span_id).or_default() += 1;
            counts
        },
    );
    assert_eq!(
        activity_rows_by_span,
        std::collections::HashMap::from([
            (first_span_id.as_str(), 2usize),
            (second_span_id.as_str(), 2usize),
        ]),
        "each exact seeded span must have one A receipt and one B receipt, with no other span ids"
    );
    cleanup.assert_cleanup();
    println!(
        "AC-1..3 LIVE OK: daily note {} idempotent; CalendarEvent {} resolved and navigated; \
         ActivitySpan {} reloaded read-only; both interop FR rows persisted",
        a.doc_id, resolved.id, spans[0].span_id
    );
}

#[test]
fn mounted_host_retries_first_transient_calendar_read_without_unavailable_state() {
    let _server_guard = mounted_server_test_guard();
    let workspace_id = "WS-MT067-TRANSIENT";
    let date = d(2026, 7, 26);
    let (base_url, stop, counts, server) = spawn_transient_mounted_calendar_server(
        workspace_id,
        date,
        MountedEventMode::TransientThenEvent,
    );
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("MT-067 transient mounted runtime");
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }));
    app.set_backend_base_url_for_test(&base_url, runtime.handle().clone());
    app.bind_active_project_for_integration_test(workspace_id.to_owned());
    let pane_id = PaneId::from("pane-a");
    app.pane_registry().lock().unwrap().insert(PaneRecord::new(
        pane_id.clone(),
        PaneType::LoomDailyJournal,
        workspace_id,
        None,
        LockState::Unlocked,
        DirtyState::Clean,
        PaneAuthority::System,
    ));
    let mut tab = TabState::new(PaneType::LoomDailyJournal);
    tab.content_id = Some("DOC-TRANSIENT-DATE-B".to_owned());
    let bar = app
        .tab_bar_states_mut()
        .get_mut(&pane_id)
        .expect("default pane-a has a tab bar");
    bar.tabs = vec![tab];
    bar.active_index = 0;
    app.set_active_pane_for_test(Some(pane_id));
    let mounted = app.mounted_daily_journal();
    mounted.lock().unwrap().prepare_date(date);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 620.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        harness.run_steps(1);
        let state = mounted.lock().unwrap().clone();
        assert!(
            !matches!(state.projection, CalendarProjectionState::Failed(_)),
            "a retryable first Calendar read must not publish an intermediate failure state: {state:?}"
        );
        if state.event.as_ref().is_some_and(|event| {
            event.id == "EVENT-TRANSIENT-DATE-B"
                && event.daily_note_doc_id.as_ref()
                    == Some(&DocId("DOC-TRANSIENT-DATE-B".to_owned()))
        }) && matches!(state.activity, ActivityCorrelation::Spans(ref spans) if spans.is_empty())
        {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "mounted host did not recover from the first transient Calendar read: {state:?}"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(
        counts
            .event_reads
            .load(std::sync::atomic::Ordering::Acquire),
        2,
        "the mounted host retries the first 503 exactly once before accepting success"
    );
    assert_eq!(
        counts
            .journal_puts
            .load(std::sync::atomic::Ordering::Acquire),
        1,
        "one mounted generation has exactly one journal open/create mutation authority"
    );
    stop.store(true, std::sync::atomic::Ordering::Release);
    server.join().expect("join transient calendar server");
}

#[test]
fn canonical_localhost_argus_inspects_navigates_and_freshly_reobserves_calendar_event() {
    use canonical_argus_driver::{json_has_author_id, CanonicalArgusDriver};

    let _server_guard = mounted_server_test_guard();
    let workspace_id = "WS-MT067-CANONICAL-ARGUS";
    let date = d(2026, 10, 25);
    let (base_url, stop, _counts, server) = spawn_transient_mounted_calendar_server(
        workspace_id,
        date,
        MountedEventMode::NormalizedOverlap,
    );
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("MT-067 canonical Argus mounted runtime");
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }));
    app.set_backend_base_url_for_test(&base_url, runtime.handle().clone());
    app.bind_active_project_for_integration_test(workspace_id.to_owned());
    let pane_id = PaneId::from("pane-a");
    app.pane_registry().lock().unwrap().insert(PaneRecord::new(
        pane_id.clone(),
        PaneType::LoomDailyJournal,
        workspace_id,
        None,
        LockState::Unlocked,
        DirtyState::Clean,
        PaneAuthority::System,
    ));
    let mut tab = TabState::new(PaneType::LoomDailyJournal);
    tab.content_id = Some("DOC-TRANSIENT-DATE-B".to_owned());
    let bar = app
        .tab_bar_states_mut()
        .get_mut(&pane_id)
        .expect("default pane-a has a tab bar");
    bar.tabs = vec![tab];
    bar.active_index = 0;
    app.set_active_pane_for_test(Some(pane_id));
    let mounted = app.mounted_daily_journal();
    mounted.lock().unwrap().prepare_date(date);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 620.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        harness.run_steps(1);
        if mounted
            .lock()
            .unwrap()
            .event
            .as_ref()
            .is_some_and(|event| event.id == "EVENT-TRANSIENT-DATE-B")
        {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "mounted Calendar event was not visible before canonical Argus inspection"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    harness.run_steps(1);

    let mut argus = CanonicalArgusDriver::bind(harness.state(), "mt067-calendar-temporal");
    let before = argus.inspect(&mut harness);
    assert!(
        json_has_author_id(&before, DAILY_JOURNAL_CALENDAR_EVENT_CHIP_AUTHOR_ID),
        "canonical argus.inspect sees the mounted CalendarEvent chip"
    );
    assert!(
        json_has_author_id(
            &before,
            handshake_native::graph::daily_journal_panel::DAILY_JOURNAL_NORMALIZATION_BADGE_AUTHOR_ID,
        ),
        "canonical argus.inspect sees the stable normalization badge"
    );
    let observation = argus.click_from_snapshot_and_reinspect(
        &mut harness,
        DAILY_JOURNAL_CALENDAR_EVENT_CHIP_AUTHOR_ID,
        before,
    );
    let details_id = handshake_native::graph::daily_journal_panel::CALENDAR_EVENT_DETAILS_AUTHOR_ID;
    assert!(
        json_has_author_id(&observation.after, details_id),
        "fresh canonical argus.inspect re-observes the content-addressed CalendarEvent destination"
    );
    assert!(
        json_has_author_id(
            &observation.after,
            handshake_native::graph::daily_journal_panel::CALENDAR_EVENT_NORMALIZATION_BADGE_AUTHOR_ID,
        ),
        "fresh canonical argus.inspect re-observes the detail normalization badge"
    );
    assert_eq!(
        harness
            .state()
            .tab_bar_states()
            .get(harness.state().active_pane().expect("active Calendar pane"))
            .and_then(|bar| bar.tabs.get(bar.active_index))
            .map(|tab| (&tab.pane_type, tab.content_id.as_deref())),
        Some((&PaneType::CalendarEvent, Some("EVENT-TRANSIENT-DATE-B")))
    );
    argus.finish();

    stop.store(true, std::sync::atomic::Ordering::Release);
    server.join().expect("join canonical Argus Calendar server");
}

#[test]
fn mounted_empty_event_list_finishes_as_no_event_with_one_journal_put() {
    let _server_guard = mounted_server_test_guard();
    let workspace_id = "WS-MT067-NO-EVENT";
    let date = d(2026, 7, 27);
    let (base_url, stop, counts, server) =
        spawn_transient_mounted_calendar_server(workspace_id, date, MountedEventMode::Empty);
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("MT-067 empty mounted runtime");
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }));
    app.set_backend_base_url_for_test(&base_url, runtime.handle().clone());
    app.bind_active_project_for_integration_test(workspace_id.to_owned());
    let pane_id = PaneId::from("pane-a");
    app.pane_registry().lock().unwrap().insert(PaneRecord::new(
        pane_id.clone(),
        PaneType::LoomDailyJournal,
        workspace_id,
        None,
        LockState::Unlocked,
        DirtyState::Clean,
        PaneAuthority::System,
    ));
    let mut tab = TabState::new(PaneType::LoomDailyJournal);
    tab.content_id = Some("DOC-TRANSIENT-DATE-B".to_owned());
    let bar = app
        .tab_bar_states_mut()
        .get_mut(&pane_id)
        .expect("default pane-a has a tab bar");
    bar.tabs = vec![tab];
    bar.active_index = 0;
    app.set_active_pane_for_test(Some(pane_id));
    let mounted = app.mounted_daily_journal();
    mounted.lock().unwrap().prepare_date(date);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 620.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        harness.run_steps(1);
        let state = mounted.lock().unwrap().clone();
        if state.projection == CalendarProjectionState::NoEvent {
            assert!(state.event.is_none());
            assert_eq!(state.activity, ActivityCorrelation::NoEvent);
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "mounted empty event response never left Loading: {state:?}"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(
        counts
            .event_reads
            .load(std::sync::atomic::Ordering::Acquire),
        1
    );
    assert_eq!(
        counts
            .journal_puts
            .load(std::sync::atomic::Ordering::Acquire),
        1
    );
    stop.store(true, std::sync::atomic::Ordering::Release);
    server.join().expect("join empty calendar server");
}

#[test]
fn mounted_terminal_404_is_one_get_and_endpoint_unavailable() {
    let _server_guard = mounted_server_test_guard();
    let workspace_id = "WS-MT067-404";
    let date = d(2026, 7, 28);
    let (base_url, stop, counts, server) = spawn_transient_mounted_calendar_server(
        workspace_id,
        date,
        MountedEventMode::AlwaysNotFound,
    );
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".into(),
        db_status: "ok".into(),
        migration_version: Some(1),
    }));
    app.set_backend_base_url_for_test(&base_url, runtime.handle().clone());
    app.bind_active_project_for_integration_test(workspace_id.to_owned());
    let pane_id = PaneId::from("pane-a");
    app.pane_registry().lock().unwrap().insert(PaneRecord::new(
        pane_id.clone(),
        PaneType::LoomDailyJournal,
        workspace_id,
        None,
        LockState::Unlocked,
        DirtyState::Clean,
        PaneAuthority::System,
    ));
    let mut tab = TabState::new(PaneType::LoomDailyJournal);
    tab.content_id = Some("DOC-TRANSIENT-DATE-B".to_owned());
    let bar = app.tab_bar_states_mut().get_mut(&pane_id).unwrap();
    bar.tabs = vec![tab];
    bar.active_index = 0;
    app.set_active_pane_for_test(Some(pane_id));
    let mounted = app.mounted_daily_journal();
    mounted.lock().unwrap().prepare_date(date);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 620.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        harness.run_steps(1);
        let projection = mounted.lock().unwrap().projection.clone();
        if projection == CalendarProjectionState::Failed(CalendarReadFailure::EndpointUnavailable) {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "404 did not reach typed endpoint state: {projection:?}"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(
        counts
            .event_reads
            .load(std::sync::atomic::Ordering::Acquire),
        1
    );
    stop.store(true, std::sync::atomic::Ordering::Release);
    server.join().unwrap();
}

#[test]
fn mounted_three_503s_are_retry_exhausted_after_exactly_three_gets() {
    let _server_guard = mounted_server_test_guard();
    let workspace_id = "WS-MT067-503";
    let date = d(2026, 7, 29);
    let (base_url, stop, counts, server) = spawn_transient_mounted_calendar_server(
        workspace_id,
        date,
        MountedEventMode::AlwaysUnavailable,
    );
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".into(),
        db_status: "ok".into(),
        migration_version: Some(1),
    }));
    app.set_backend_base_url_for_test(&base_url, runtime.handle().clone());
    app.bind_active_project_for_integration_test(workspace_id.to_owned());
    let pane_id = PaneId::from("pane-a");
    app.pane_registry().lock().unwrap().insert(PaneRecord::new(
        pane_id.clone(),
        PaneType::LoomDailyJournal,
        workspace_id,
        None,
        LockState::Unlocked,
        DirtyState::Clean,
        PaneAuthority::System,
    ));
    let mut tab = TabState::new(PaneType::LoomDailyJournal);
    tab.content_id = Some("DOC-TRANSIENT-DATE-B".to_owned());
    let bar = app.tab_bar_states_mut().get_mut(&pane_id).unwrap();
    bar.tabs = vec![tab];
    bar.active_index = 0;
    app.set_active_pane_for_test(Some(pane_id));
    let mounted = app.mounted_daily_journal();
    mounted.lock().unwrap().prepare_date(date);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 620.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        harness.run_steps(1);
        let projection = mounted.lock().unwrap().projection.clone();
        if projection == CalendarProjectionState::Failed(CalendarReadFailure::RetryExhausted) {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "503 exhaustion did not reach typed state: {projection:?}"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_counter_settles_exact(&counts.event_reads, 3, "calendar 503 GET attempts");
    stop.store(true, std::sync::atomic::Ordering::Release);
    server.join().unwrap();
}

#[test]
fn mounted_navigation_while_old_get_is_in_flight_cancels_without_fr_residue() {
    let _server_guard = mounted_server_test_guard();
    let workspace_id = "WS-MT067-CANCEL";
    let first_date = chrono::Local::now().date_naive();
    let second_date = first_date
        .checked_add_days(chrono::Days::new(1))
        .expect("next local day");
    let (base_url, stop, counts, server) = spawn_transient_mounted_calendar_server(
        workspace_id,
        first_date,
        MountedEventMode::TransientThenEmpty,
    );
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".into(),
        db_status: "ok".into(),
        migration_version: Some(1),
    }));
    app.set_backend_base_url_for_test(&base_url, runtime.handle().clone());
    app.bind_active_project_for_integration_test(workspace_id.to_owned());
    let pane_id = PaneId::from("pane-a");
    app.pane_registry().lock().unwrap().insert(PaneRecord::new(
        pane_id.clone(),
        PaneType::LoomDailyJournal,
        workspace_id,
        None,
        LockState::Unlocked,
        DirtyState::Clean,
        PaneAuthority::System,
    ));
    let mut tab = TabState::new(PaneType::LoomDailyJournal);
    tab.content_id = Some("DOC-TRANSIENT-DATE-B".to_owned());
    let bar = app.tab_bar_states_mut().get_mut(&pane_id).unwrap();
    bar.tabs = vec![tab];
    bar.active_index = 0;
    app.set_active_pane_for_test(Some(pane_id));
    let mounted = app.mounted_daily_journal();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1100.0, 760.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    let first_request_deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while counts
        .event_reads
        .load(std::sync::atomic::Ordering::Acquire)
        == 0
    {
        harness.run_steps(1);
        assert!(
            std::time::Instant::now() < first_request_deadline,
            "first transient GET never arrived through the mounted operator path"
        );
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert_eq!(
        harness
            .query_all_by(|node: &egui_kittest::kittest::AccessKitNode<'_>| {
                node.author_id() == Some(NEXT_DAY_ID)
            })
            .count(),
        1,
        "the mounted journal-editor next-day control must be collision-free"
    );
    // Render one stable frame after the off-thread request has reached the held 503 server. This keeps
    // the AccessKit target current while the first response is still blocked, then the ordinary click
    // and next-frame host drain must atomically supersede that request.
    harness.run_steps(1);
    harness
        .get_by(|node| node.author_id() == Some(NEXT_DAY_ID))
        .click();
    harness.run_steps(2);
    let second_deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        harness.run_steps(1);
        let state = mounted.lock().unwrap().clone();
        if state.nav.current == second_date && state.projection == CalendarProjectionState::NoEvent
        {
            break;
        }
        assert!(
            std::time::Instant::now() < second_deadline,
            "new date did not settle after cancellation: {state:?}; requests={:?}",
            counts.request_lines.lock().unwrap()
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let old_label = first_date.format("%Y-%m-%d").to_string();
    let old_end_label = second_date.format("%Y-%m-%d").to_string();
    let request_lines = counts.request_lines.lock().unwrap();
    let old_reads = request_lines
        .iter()
        .filter_map(|line| line.split_ascii_whitespace().nth(1))
        .filter_map(|target| reqwest::Url::parse(&format!("http://localhost{target}")).ok())
        .filter(|url| {
            if !url.path().ends_with("/calendar/events") {
                return false;
            }
            let query = url
                .query_pairs()
                .collect::<std::collections::HashMap<_, _>>();
            query
                .get("from_date")
                .is_some_and(|value| value == &old_label)
                && query
                    .get("to_date_exclusive")
                    .is_some_and(|value| value == &old_end_label)
        })
        .count();
    assert_eq!(
        old_reads, 1,
        "a superseded in-flight date must not issue a later retry"
    );
    assert_eq!(
        counts
            .native_fr_posts
            .load(std::sync::atomic::Ordering::Acquire),
        0
    );
    stop.store(true, std::sync::atomic::Ordering::Release);
    server.join().unwrap();
}

fn assert_mounted_activity_failure(
    mode: MountedEventMode,
    expected_failure: CalendarReadFailure,
    expected_activity_reads: usize,
) {
    let workspace_id = match mode {
        MountedEventMode::EventThenActivityNotFound => "WS-MT067-ACTIVITY-404",
        MountedEventMode::EventThenActivityUnavailable => "WS-MT067-ACTIVITY-503",
        _ => panic!("activity failure helper received wrong mode"),
    };
    let date = d(2026, 8, 1);
    let (base_url, stop, counts, server) =
        spawn_transient_mounted_calendar_server(workspace_id, date, mode);
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".into(),
        db_status: "ok".into(),
        migration_version: Some(1),
    }));
    app.set_backend_base_url_for_test(&base_url, runtime.handle().clone());
    app.bind_active_project_for_integration_test(workspace_id.to_owned());
    let pane_id = PaneId::from("pane-a");
    app.pane_registry().lock().unwrap().insert(PaneRecord::new(
        pane_id.clone(),
        PaneType::LoomDailyJournal,
        workspace_id,
        None,
        LockState::Unlocked,
        DirtyState::Clean,
        PaneAuthority::System,
    ));
    let mut tab = TabState::new(PaneType::LoomDailyJournal);
    tab.content_id = Some("DOC-TRANSIENT-DATE-B".to_owned());
    let bar = app.tab_bar_states_mut().get_mut(&pane_id).unwrap();
    bar.tabs = vec![tab];
    bar.active_index = 0;
    app.set_active_pane_for_test(Some(pane_id));
    let mounted = app.mounted_daily_journal();
    mounted.lock().unwrap().prepare_date(date);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 620.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        harness.run_steps(1);
        let state = mounted.lock().unwrap().clone();
        if state
            .event
            .as_ref()
            .is_some_and(|event| event.id == "EVENT-ACTIVITY-FAIL")
            && state.activity == ActivityCorrelation::Failed(expected_failure.clone())
        {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "mounted activity failure did not preserve event/typed state: {state:?}"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    while counts
        .event_bound_fr_posts
        .load(std::sync::atomic::Ordering::Acquire)
        < 1
    {
        harness.run_steps(1);
        assert!(
            std::time::Instant::now() < deadline,
            "event-bound FR receipt missing"
        );
    }
    assert_counter_settles_exact(&counts.event_reads, 1, "activity-failure event GET");
    assert_counter_settles_exact(
        &counts.activity_reads,
        expected_activity_reads,
        "activity failure GET attempts",
    );
    assert_eq!(
        counts
            .event_bound_fr_posts
            .load(std::sync::atomic::Ordering::Acquire),
        1
    );
    assert_eq!(
        counts
            .activity_fr_posts
            .load(std::sync::atomic::Ordering::Acquire),
        0
    );
    stop.store(true, std::sync::atomic::Ordering::Release);
    server.join().unwrap();
}

#[test]
fn mounted_activity_404_and_503_preserve_event_without_activity_fr() {
    let _server_guard = mounted_server_test_guard();
    assert_mounted_activity_failure(
        MountedEventMode::EventThenActivityNotFound,
        CalendarReadFailure::EndpointUnavailable,
        1,
    );
    assert_mounted_activity_failure(
        MountedEventMode::EventThenActivityUnavailable,
        CalendarReadFailure::RetryExhausted,
        3,
    );
}

#[test]
fn mounted_journal_503_exhaustion_clears_stale_projection_and_skips_calendar() {
    let _server_guard = mounted_server_test_guard();
    let workspace_id = "WS-MT067-JOURNAL-503";
    let date = d(2026, 8, 2);
    let (base_url, stop, counts, server) = spawn_transient_mounted_calendar_server(
        workspace_id,
        date,
        MountedEventMode::JournalUnavailable,
    );
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".into(),
        db_status: "ok".into(),
        migration_version: Some(1),
    }));
    app.set_backend_base_url_for_test(&base_url, runtime.handle().clone());
    app.bind_active_project_for_integration_test(workspace_id.to_owned());
    let mounted = app.mounted_daily_journal();
    let stale = event("STALE-EVENT", "must clear");
    mounted
        .lock()
        .unwrap()
        .set_event_with_spans(stale, Vec::new());
    let pane_id = PaneId::from("pane-a");
    app.pane_registry().lock().unwrap().insert(PaneRecord::new(
        pane_id.clone(),
        PaneType::LoomDailyJournal,
        workspace_id,
        None,
        LockState::Unlocked,
        DirtyState::Clean,
        PaneAuthority::System,
    ));
    let mut tab = TabState::new(PaneType::LoomDailyJournal);
    tab.content_id = Some("DOC-TRANSIENT-DATE-B".to_owned());
    let bar = app.tab_bar_states_mut().get_mut(&pane_id).unwrap();
    bar.tabs = vec![tab];
    bar.active_index = 0;
    app.set_active_pane_for_test(Some(pane_id));
    mounted.lock().unwrap().prepare_date(date);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 620.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        harness.run_steps(1);
        let state = mounted.lock().unwrap().clone();
        if state.projection == CalendarProjectionState::DailyNoteError {
            assert!(state.event.is_none(), "stale event must be cleared");
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "journal failure did not reach DailyNoteError: {state:?}"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_counter_settles_exact(&counts.journal_puts, 3, "journal 503 PUT attempts");
    assert_eq!(
        counts
            .event_reads
            .load(std::sync::atomic::Ordering::Acquire),
        0
    );
    assert_eq!(
        counts
            .native_fr_posts
            .load(std::sync::atomic::Ordering::Acquire),
        0
    );
    stop.store(true, std::sync::atomic::Ordering::Release);
    server.join().unwrap();
}

#[test]
fn mounted_malformed_calendar_response_is_invalid_after_one_get() {
    let _server_guard = mounted_server_test_guard();
    let workspace_id = "WS-MT067-MALFORMED";
    let date = d(2026, 8, 3);
    let (base_url, stop, counts, server) = spawn_transient_mounted_calendar_server(
        workspace_id,
        date,
        MountedEventMode::MalformedEvent,
    );
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".into(),
        db_status: "ok".into(),
        migration_version: Some(1),
    }));
    app.set_backend_base_url_for_test(&base_url, runtime.handle().clone());
    app.bind_active_project_for_integration_test(workspace_id.to_owned());
    let pane_id = PaneId::from("pane-a");
    app.pane_registry().lock().unwrap().insert(PaneRecord::new(
        pane_id.clone(),
        PaneType::LoomDailyJournal,
        workspace_id,
        None,
        LockState::Unlocked,
        DirtyState::Clean,
        PaneAuthority::System,
    ));
    let mut tab = TabState::new(PaneType::LoomDailyJournal);
    tab.content_id = Some("DOC-TRANSIENT-DATE-B".to_owned());
    let bar = app.tab_bar_states_mut().get_mut(&pane_id).unwrap();
    bar.tabs = vec![tab];
    bar.active_index = 0;
    app.set_active_pane_for_test(Some(pane_id));
    let mounted = app.mounted_daily_journal();
    mounted.lock().unwrap().prepare_date(date);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 620.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        harness.run_steps(1);
        let projection = mounted.lock().unwrap().projection.clone();
        if projection == CalendarProjectionState::Failed(CalendarReadFailure::InvalidResponse) {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "malformed response did not reach InvalidResponse: {projection:?}"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(
        counts
            .event_reads
            .load(std::sync::atomic::Ordering::Acquire),
        1
    );
    assert_eq!(
        counts
            .native_fr_posts
            .load(std::sync::atomic::Ordering::Acquire),
        0
    );
    stop.store(true, std::sync::atomic::Ordering::Release);
    server.join().unwrap();
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// AC-2 / PT-3 — a resolved event renders a clickable chip; its click emits focus-calendar-event on the bus.
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn event_chip_click_emits_focus_calendar_event_on_bus() {
    use handshake_native::interop::InteractionBus;
    use std::cell::RefCell;
    use std::rc::Rc;

    let mut state = DailyJournalState::new(DateNav::new(d(2026, 6, 21), d(2026, 6, 21)));
    state.set_event_with_spans(event("E-1", "Sprint planning"), vec![]);

    let captured: Rc<RefCell<DailyJournalEvent>> = Rc::new(RefCell::new(DailyJournalEvent::None));
    let cap = captured.clone();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(420.0, 320.0))
        .build_ui(move |ui| {
            let ev = DailyJournalPanel::show(ui, &mut state, &dark());
            if !matches!(ev, DailyJournalEvent::None) {
                *cap.borrow_mut() = ev;
            }
        });
    harness.run();
    harness
        .get_by(|n| n.author_id() == Some(DAILY_JOURNAL_CALENDAR_EVENT_CHIP_AUTHOR_ID))
        .click();
    harness.run();

    match &*captured.borrow() {
        DailyJournalEvent::FocusCalendarEvent(id) => {
            assert_eq!(id, "E-1", "AC-2: the chip click carries the event id");
        }
        other => panic!("AC-2: chip click must emit FocusCalendarEvent, got {other:?}"),
    }

    // The real named command stages the exact payload that the shell consumes once. This rejects the old
    // placebo proof where an arbitrary handler ran without carrying the clicked event id.
    let ctx = egui::Context::default();
    let _ = ctx.run(Default::default(), |ctx| {
        let mut bus = InteractionBus::new();
        bus.register_focus_calendar_event_command();
        assert!(
            bus.focus_calendar_event(ctx, "E-1"),
            "AC-2: the focus-calendar-event bus command dispatches"
        );
        assert_eq!(bus.pending_calendar_event_focus(), Some("E-1"));
        assert_eq!(
            bus.take_pending_calendar_event_focus().as_deref(),
            Some("E-1")
        );
        assert!(
            bus.take_pending_calendar_event_focus().is_none(),
            "AC-2: the typed event payload is consumed exactly once"
        );
    });
    println!("AC-2/PT-3 OK: chip click -> FocusCalendarEvent -> loom.daily-note.focus-calendar-event bus dispatch");
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// AC-3 — the activity strip renders read-only doc chips; a chip-click navigates; NO write path exists.
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn activity_strip_renders_read_only_chips_and_no_write() {
    use std::cell::RefCell;
    use std::rc::Rc;

    let mut state = DailyJournalState::new(DateNav::new(d(2026, 6, 21), d(2026, 6, 21)));
    state.set_event_with_spans(
        event("E-1", "Block"),
        vec![span("S-1", &["DOC-A", "DOC-B"])],
    );

    let captured: Rc<RefCell<DailyJournalEvent>> = Rc::new(RefCell::new(DailyJournalEvent::None));
    let cap = captured.clone();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(420.0, 340.0))
        .build_ui(move |ui| {
            let ev = DailyJournalPanel::show(ui, &mut state, &dark());
            if let DailyJournalEvent::OpenDocument(_) = &ev {
                *cap.borrow_mut() = ev;
            }
        });
    harness.run();

    // The read-only doc chips are addressable by their per-item author_id; clicking one emits navigation.
    let chip_a = activity_item_author_id(&DocId("DOC-A".to_owned()));
    harness
        .get_by(|n| n.author_id() == Some(chip_a.as_str()))
        .click();
    harness.run();

    match &*captured.borrow() {
        DailyJournalEvent::OpenDocument(doc_id) => {
            assert_eq!(
                doc_id,
                &DocId("DOC-A".to_owned()),
                "AC-3: chip click navigates to the doc"
            );
        }
        other => panic!(
            "AC-3: a read-only chip click must emit OpenDocument (navigation), got {other:?}"
        ),
    }

    // AC-3 / RISK-5/MC-5: the panel source has NO mutation path on ActivitySpan data — the only outcome a
    // chip produces is the navigation OpenDocument event. Prove the panel never exposes a write API by
    // grepping its source for write verbs against the activity data (no `.post(`/`.put(`/etc., no
    // `edited_doc_ids` mutation). The activity strip is render-only.
    let panel_src = include_str!("../src/graph/daily_journal_panel.rs");
    for verb in [
        ".post(",
        ".put(",
        ".delete(",
        ".patch(",
        "push_to_span",
        "write_span",
    ] {
        assert!(
            !panel_src.contains(verb),
            "AC-3/RISK-5: the panel must have no ActivitySpan write path — found '{verb}'"
        );
    }
    println!(
        "AC-3 OK: read-only activity chips render, click navigates, zero write path on span data"
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// AC-4 / PT-4 — unavailable /calendar/ reads are typed blockers; the panel stays alive on the empty-state.
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn activity_spans_404_is_typed_blocker_and_panel_stays_alive() {
    // A simulated 404 on /calendar/activity-spans -> EndpointUnavailable (the typed blocker, DISTINCT from
    // a generic Http error). The daily-note and already resolved CalendarEvent stay functional when only
    // the ActivitySpan read is unavailable.
    let backend = Arc::new(CountingJournalBackend::new(Some("DOC-2026-06-21")));
    let (base_url, server) = spawn_mock(
        "HTTP/1.1 404 Not Found",
        serde_json::json!({"error": "not found"}),
    );
    let svc = CalendarInteropService::with_base_url(base_url, "WS-1", backend.clone());

    let result = rt().block_on(async { svc.activity_spans_for_event("E-1").await });
    let req_line = server.join().unwrap();

    // The probe is a read-only GET at the documented route.
    assert!(
        req_line.starts_with("GET "),
        "AC-4: activity-spans read must be a GET; got '{req_line}'"
    );
    assert!(
        req_line.contains("/workspaces/WS-1/calendar/activity-spans"),
        "AC-4: probes the documented activity-spans route; got '{req_line}'"
    );
    match result {
        Err(InteropError::EndpointUnavailable { probed_path }) => {
            assert!(
                probed_path.contains("/calendar/activity-spans"),
                "AC-4: EndpointUnavailable names the probed path; got '{probed_path}'"
            );
        }
        other => {
            panic!("AC-4: a 404 must map to EndpointUnavailable (typed blocker), got {other:?}")
        }
    }

    // The daily-note binding STILL works (the panel never dies when Calendar reads are unavailable): the MT-019
    // delegation still produces the single doc for the date.
    let binding = rt()
        .block_on(async { svc.open_or_create_daily_note(d(2026, 6, 21)).await })
        .expect(
            "AC-4: the daily-note binding stays functional alongside the typed calendar blocker",
        );
    assert_eq!(binding.doc_id, DocId("DOC-2026-06-21".to_owned()));

    // An ActivitySpan-only failure preserves the already resolved event and its daily-note binding while
    // rendering only the activity strip's typed unavailable state.
    let mut state = DailyJournalState::new(DateNav::new(d(2026, 6, 21), d(2026, 6, 21)));
    let mut resolved_event = event("E-1", "Sprint planning");
    resolved_event.daily_note_doc_id = Some(binding.doc_id.clone());
    state.set_activity_unavailable(resolved_event.clone());
    assert_eq!(state.event.as_ref(), Some(&resolved_event));
    assert_eq!(state.projection, CalendarProjectionState::Event);
    assert_eq!(
        state.activity,
        ActivityCorrelation::Failed(CalendarReadFailure::EndpointUnavailable)
    );
    let mut harness = Harness::builder()
        .with_size(egui::vec2(420.0, 300.0))
        .build_ui(move |ui| {
            let _ = DailyJournalPanel::show(ui, &mut state, &dark());
        });
    harness.run();
    // The panel container + the date header are still present (the panel is alive).
    let root = harness.root();
    assert!(
        role_of(&root, DAILY_JOURNAL_PANEL_AUTHOR_ID).is_some(),
        "AC-4: the panel container is alive on the typed blocker"
    );
    assert!(
        role_of(&root, DAILY_JOURNAL_DATE_HEADER_AUTHOR_ID).is_some(),
        "AC-4: the date header stays functional on the typed blocker"
    );
    assert!(
        role_of(&root, DAILY_JOURNAL_CALENDAR_EVENT_CHIP_AUTHOR_ID).is_some(),
        "AC-4: an ActivitySpan blocker must not suppress the resolved CalendarEvent chip"
    );
    println!("AC-4/PT-4 OK: 404 -> EndpointUnavailable, daily-note/event binding alive, activity renders its typed empty-state");
}

#[test]
fn mounted_host_preserves_event_when_activity_fetch_fails() {
    let date = d(2026, 7, 21);
    let mut state = DailyJournalState::new(DateNav::new(date, date));
    let resolved = event("EVENT-ACTIVITY-FAIL", "Preserved event");
    state.set_activity_failure(resolved.clone(), CalendarReadFailure::RetryExhausted);
    assert_eq!(state.event.as_ref(), Some(&resolved));
    assert_eq!(state.projection, CalendarProjectionState::Event);
    assert_eq!(
        state.activity,
        ActivityCorrelation::Failed(CalendarReadFailure::RetryExhausted)
    );

    let mut harness = Harness::builder()
        .with_size(egui::vec2(560.0, 320.0))
        .build_ui(move |ui| {
            let _ = DailyJournalPanel::show(ui, &mut state, &dark());
        });
    harness.run();
    let root = harness.root();
    assert!(
        role_of(&root, DAILY_JOURNAL_CALENDAR_EVENT_CHIP_AUTHOR_ID).is_some(),
        "typed activity failure must preserve the resolved event chip"
    );
    assert!(
        role_of(&root, DAILY_JOURNAL_ACTIVITY_STRIP_AUTHOR_ID).is_some(),
        "typed activity failure keeps the activity strip alive for its recovery message"
    );
}

#[test]
fn events_404_is_typed_blocker() {
    // BROAD detection (RISK-3/MC-3): the events route is ALSO the typed blocker (404 AND 501).
    let backend = Arc::new(CountingJournalBackend::new(Some("DOC-X")));
    for status in ["HTTP/1.1 404 Not Found", "HTTP/1.1 501 Not Implemented"] {
        let (base_url, server) = spawn_mock(status, serde_json::json!({"error": "absent"}));
        let svc = CalendarInteropService::with_base_url(base_url, "WS-1", backend.clone());
        let result =
            rt().block_on(async { svc.resolve_event_for_daily_note(d(2026, 6, 21)).await });
        let _ = server.join();
        assert!(
            matches!(result, Err(InteropError::EndpointUnavailable { .. })),
            "AC-4: events {status} must map to EndpointUnavailable, got {result:?}"
        );
    }
    println!("AC-4 OK: /calendar/events 404 AND 501 -> EndpointUnavailable (broad detection)");
}

#[test]
fn events_503_is_retryable_http_failure_on_wire() {
    let backend = Arc::new(CountingJournalBackend::new(Some("DOC-X")));
    let (base_url, server) = spawn_mock(
        "HTTP/1.1 503 Service Unavailable",
        serde_json::json!({"error": "transient"}),
    );
    let svc = CalendarInteropService::with_base_url(base_url, "WS-503", backend);
    let result =
        rt().block_on(async { svc.events_for_range(d(2026, 7, 29), d(2026, 7, 29)).await });
    server
        .join()
        .expect("503 mock server completed its one GET");
    assert!(
        matches!(&result, Err(InteropError::Http { status: 503 })),
        "wire 503 must remain an exact HTTP failure, got {result:?}"
    );
    assert!(
        result.unwrap_err().is_retryable(),
        "wire 503 must enter the bounded retry taxonomy"
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// AC-5 — no SQLite/DB anywhere, GET-only calendar reads, shared backend pool reused, no backend edit.
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn no_sqlite_no_backend_edit() {
    // Strip line-comments (// and //!) so the gate checks ACTUAL CODE, not the doc comments that explain
    // "NO SQLite anywhere" (a substring gate over the whole file would match its own prose — the rubric's
    // "prove behavior, not hide uncertainty"). Block comments are not used in these files.
    fn code_only(src: &str) -> String {
        src.lines()
            .map(|line| match line.find("//") {
                Some(idx) => &line[..idx],
                None => line,
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    let sources: [(&str, String); 2] = [
        (
            "calendar_interop.rs",
            code_only(include_str!("../src/interop/calendar_interop.rs")),
        ),
        (
            "daily_journal_panel.rs",
            code_only(include_str!("../src/graph/daily_journal_panel.rs")),
        ),
    ];
    for (name, src) in &sources {
        // No DB-driver USAGE in actual code (PostgreSQL/EventLedger is the only durable authority — AC-5).
        for store in ["sqlite", "rusqlite", "diesel", "Sqlite", "SQLite", "sqlx"] {
            assert!(
                !src.contains(store),
                "AC-5: {name} code must not reference '{store}' (PostgreSQL/EventLedger only)"
            );
        }
    }
    // The calendar reads are GET-only (read-only correlation + read-only events) — no write verbs in code.
    let interop_code = code_only(include_str!("../src/interop/calendar_interop.rs"));
    for verb in [".post(", ".put(", ".delete(", ".patch("] {
        assert!(
            !interop_code.contains(verb),
            "AC-5: calendar_interop reads must be GET-only — found write verb '{verb}'"
        );
    }
    // Whole-file checks for the REUSE evidence (these tokens legitimately appear in code).
    let interop_src = include_str!("../src/interop/calendar_interop.rs");
    // It reuses the shared backend pool + base url (no second HTTP stack).
    assert!(
        interop_src.contains("shared_http_client") && interop_src.contains("BACKEND_BASE_URL"),
        "AC-5: the calendar reads must reuse the shared backend_client pool + base url (no second stack)"
    );
    assert!(
        interop_src.contains(".get(&url)"),
        "AC-5: the calendar reads must issue a GET via the reqwest builder"
    );
    println!("AC-5 OK: no sqlite/rusqlite/diesel, GET-only calendar reads, shared client and live backend routes reused");
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// AC-6 / PT-5 — AccessKit nodes present with correct roles + nesting (+ screenshot).
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn daily_journal_panel_accesskit_nodes_present() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(440.0, 360.0))
        .wgpu()
        .build_ui(|ui| {
            let mut state = DailyJournalState::new(DateNav::new(d(2026, 6, 21), d(2026, 6, 21)));
            // Seed a resolved event + spans so the chip (Button) + the activity strip (List) both render.
            state.set_event_with_spans(
                event("E-1", "Sprint planning"),
                vec![span("S-1", &["DOC-A"])],
            );
            let _ = DailyJournalPanel::show(ui, &mut state, &dark());
        });
    harness.run();
    harness.run();

    let root = harness.root();

    // AC-6 / PT-5: the contract-named nodes are present with the right roles.
    assert_eq!(
        role_of(&root, DAILY_JOURNAL_PANEL_AUTHOR_ID).as_deref(),
        Some("GenericContainer"),
        "PT-5: 'daily-journal-panel' must be Role::GenericContainer"
    );
    assert_eq!(
        role_of(&root, DAILY_JOURNAL_DATE_HEADER_AUTHOR_ID).as_deref(),
        Some("Label"),
        "PT-5: 'daily-journal-date-header' must be Role::Label"
    );
    assert_eq!(
        role_of(&root, DAILY_JOURNAL_CALENDAR_EVENT_CHIP_AUTHOR_ID).as_deref(),
        Some("Button"),
        "PT-5: 'daily-journal-calendar-event-chip' must be Role::Button"
    );
    assert_eq!(
        role_of(&root, DAILY_JOURNAL_ACTIVITY_STRIP_AUTHOR_ID).as_deref(),
        Some("List"),
        "PT-5: 'daily-journal-activity-strip' must be Role::List"
    );
    // The reused MT-019 widget is present under this mounted surface's collision-free address set.
    assert!(
        role_of(&root, DAILY_JOURNAL_DATE_NAV_AUTHOR_IDS.prev_day).is_some(),
        "AC-6: reused MT-019 daily-journal-prev-day present"
    );
    assert!(
        role_of(&root, DAILY_JOURNAL_DATE_NAV_AUTHOR_IDS.next_day).is_some(),
        "AC-6: reused MT-019 daily-journal-next-day present"
    );

    // Nesting: the date header, the chip, and the activity strip are under the panel container.
    assert!(
        author_under(
            &root,
            DAILY_JOURNAL_DATE_HEADER_AUTHOR_ID,
            DAILY_JOURNAL_PANEL_AUTHOR_ID
        ),
        "AC-6: the date header nests under the panel container"
    );
    assert!(
        author_under(
            &root,
            DAILY_JOURNAL_CALENDAR_EVENT_CHIP_AUTHOR_ID,
            DAILY_JOURNAL_PANEL_AUTHOR_ID
        ),
        "AC-6: the calendar-event chip nests under the panel container"
    );
    assert!(
        author_under(
            &root,
            DAILY_JOURNAL_ACTIVITY_STRIP_AUTHOR_ID,
            DAILY_JOURNAL_PANEL_AUTHOR_ID
        ),
        "AC-6: the activity strip nests under the panel container"
    );

    println!(
        "PT-5 accesskit dump: {{\"daily-journal-panel\":\"{}\",\"daily-journal-date-header\":\"{}\",\"daily-journal-calendar-event-chip\":\"{}\",\"daily-journal-activity-strip\":\"{}\"}}",
        role_of(&root, DAILY_JOURNAL_PANEL_AUTHOR_ID).unwrap_or_default(),
        role_of(&root, DAILY_JOURNAL_DATE_HEADER_AUTHOR_ID).unwrap_or_default(),
        role_of(&root, DAILY_JOURNAL_CALENDAR_EVENT_CHIP_AUTHOR_ID).unwrap_or_default(),
        role_of(&root, DAILY_JOURNAL_ACTIVITY_STRIP_AUTHOR_ID).unwrap_or_default()
    );

    // Screenshot to the EXTERNAL root ONLY (best-effort pixel readback).
    if let Ok(image) = harness.render() {
        let ext_dir = external_artifact_dir("wp-kernel-012-mt-067");
        let _ = std::fs::create_dir_all(&ext_dir);
        let ext_path = ext_dir.join("MT-067-daily-journal-calendar-interop.png");
        let saved = image.save(&ext_path).is_ok();
        println!(
            "PT-5 screenshot: {}x{} saved_ext={saved} ({})",
            image.width(),
            image.height(),
            ext_path.display()
        );
    } else {
        println!(
            "PT-5 screenshot: GPU readback unavailable on this host (structural proof stands)"
        );
    }

    assert_no_local_artifact_dir();
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// AC-6 (command surface) — the three daily-note <-> Calendar bus command ids are registered exactly once.
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn daily_note_command_ids_registered() {
    use handshake_native::command_registry::all_commands;

    for id in [
        CMD_OPEN_DAILY_NOTE_FOR_DATE,
        CMD_FOCUS_CALENDAR_EVENT,
        CMD_ACTIVITY_OPEN_DOCUMENT,
    ] {
        let rows: Vec<_> = all_commands().iter().filter(|c| c.id == id).collect();
        assert_eq!(
            rows.len(),
            1,
            "AC-6: command id '{id}' must be present exactly once in the palette catalog"
        );
        assert!(
            !rows[0].disabled,
            "AC-6: command '{id}' is enabled (bus-driven)"
        );
    }
    assert_eq!(
        CMD_OPEN_DAILY_NOTE_FOR_DATE,
        "loom.daily-note.open-for-date"
    );
    assert_eq!(
        CMD_FOCUS_CALENDAR_EVENT,
        "loom.daily-note.focus-calendar-event"
    );
    assert_eq!(CMD_ACTIVITY_OPEN_DOCUMENT, "loom.activity.open-document");
    println!(
        "AC-6 command surface OK: 3 daily-note/calendar bus command ids registered exactly once"
    );
}

// ── small AccessKit tree helpers (the proven MT-066 helpers) ──────────────────────────────────────

/// The `{:?}` role string of the first node with `author_id`, if present.
fn role_of(root: &egui_kittest::Node<'_>, author_id: &str) -> Option<String> {
    for node in root.children_recursive() {
        let ak = node.accesskit_node();
        if ak.author_id() == Some(author_id) {
            return Some(format!("{:?}", ak.role()));
        }
    }
    None
}

/// True if a node addressed `child_author` has an ancestor addressed `ancestor_author`.
fn author_under(root: &egui_kittest::Node<'_>, child_author: &str, ancestor_author: &str) -> bool {
    for node in root.children_recursive() {
        if node.accesskit_node().author_id() != Some(child_author) {
            continue;
        }
        let mut cur = node.parent();
        while let Some(p) = cur {
            if p.accesskit_node().author_id() == Some(ancestor_author) {
                return true;
            }
            cur = p.parent();
        }
    }
    false
}
