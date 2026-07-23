//! WP-KERNEL-012 MT-072 (E12) — editor Settings persistence proofs (PT-001).
//!
//! These proofs drive the REAL `HandshakeApp` headlessly via egui_kittest and prove the Editor settings
//! sections persist THROUGH the SAME WP-011 PostgreSQL-backed `GET`/`PUT /workspaces/:id/settings`
//! surface — there is NO new persistence system, NO SQLite, NO new endpoint (AC-009). A scriptable
//! `StubSettingsTransport` records the PUT blob + serves a scripted GET, so the open -> change -> persist
//! round-trip is provable with no live server. The managed proof in this file additionally exercises
//! the real PostgreSQL GET/PUT, a real HTTP 503 first-save failure with exact Retry, and a fresh-app reopen.
//!
//! - AC-001: every current Editor preference, every syntax swatch, and code/rich keymap overrides issue
//!   a PUT carrying those values; the GET-on-open path reloads identical values.
//! - AC-002: editor_font_size is a SEPARATE field from the chrome appearance (theme) — the persisted blob
//!   carries them as distinct keys and changing one does not change the other.
//! - AC-006: a legacy WP-011-era settings doc (no editor keys) loads cleanly via the GET path (the dialog
//!   opens against it with the editor defaults — no hard-fail).
//! - AC-009: the ONLY persistence calls are the existing WP-011 GET/PUT — the stub transport is the sole
//!   I/O surface; no other save path is exercised.

mod pg_proof_support;

use std::sync::{Arc, Condvar, Mutex};
use std::{io::Read, io::Write};

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;
use handshake_native::accessibility::{UiNodeBounds, UiTreeNode, UiTreeSnapshot};
use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend_client::HealthInfo;
use handshake_native::code_editor::HighlightScope;
use handshake_native::settings_dialog::{SettingsOutcome, SETTINGS_SEARCH_AUTHOR_ID};
use handshake_native::settings_editor_section::{
    editor_keybind_row_author_id, EDITOR_BRACKET_MATCHING_AUTHOR_ID, EDITOR_FONT_SIZE_AUTHOR_ID,
    EDITOR_INDENT_GUIDES_AUTHOR_ID, EDITOR_INSERT_SPACES_AUTHOR_ID, EDITOR_LINE_HEIGHT_AUTHOR_ID,
    EDITOR_LINE_NUMBERS_AUTHOR_ID, EDITOR_MINIMAP_AUTHOR_ID, EDITOR_READING_MODE_DEFAULT_AUTHOR_ID,
    EDITOR_RENDER_WHITESPACE_AUTHOR_ID, EDITOR_STICKY_SCROLL_AUTHOR_ID, EDITOR_TAB_SIZE_AUTHOR_ID,
    EDITOR_WORD_WRAP_AUTHOR_ID, EDITOR_WRAP_COLUMN_AUTHOR_ID,
    FLIGHT_RECORDER_SETTINGS_POSTURE_AUTHOR_ID, FLIGHT_RECORDER_SETTINGS_POSTURE_NOTE,
    SYNTAX_PALETTE_MODE_AUTHOR_ID, SYNTAX_SWATCH_AUTHOR_IDS,
};
use handshake_native::theme::{HsTheme, MUTED_PALETTE, STANDARD_PALETTE};
use handshake_native::workspace_settings::{
    default_workspace_settings_state, normalize_workspace_settings_state, EditorPrefs,
    RenderWhitespaceMode, SettingsClient, SettingsTransport, SettingsTransportError, SyntaxPalette,
    SyntaxPaletteMode, WordWrapMode, SYNTAX_SCOPE_KEYS,
};
use serde_json::Value;

const ARGUS_PROBE_ACTIONS: &[egui::accesskit::Action] = &[
    egui::accesskit::Action::Click,
    egui::accesskit::Action::Focus,
    egui::accesskit::Action::SetValue,
    egui::accesskit::Action::ReplaceSelectedText,
    egui::accesskit::Action::ScrollIntoView,
];

fn ok_app() -> HandshakeApp {
    HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }))
}

/// A scriptable in-memory settings transport (the SAME pattern test_settings_dialog.rs uses): records
/// the last PUT blob + serves a scripted GET. The ONLY persistence surface — proving AC-009 (no new
/// save path; the editor fields ride the existing PUT/GET).
#[derive(Default)]
struct StubSettingsTransport {
    inner: Mutex<StubInner>,
}

#[derive(Default)]
struct StubInner {
    load_result: Option<Value>,
    saved: Option<Value>,
    save_calls: usize,
    load_calls: usize,
}

impl StubSettingsTransport {
    fn with_loaded(blob: Option<Value>) -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(StubInner {
                load_result: blob,
                ..Default::default()
            }),
        })
    }
    fn saved(&self) -> Option<Value> {
        self.inner.lock().unwrap().saved.clone()
    }
    fn save_calls(&self) -> usize {
        self.inner.lock().unwrap().save_calls
    }
    fn load_calls(&self) -> usize {
        self.inner.lock().unwrap().load_calls
    }
}

impl SettingsTransport for StubSettingsTransport {
    fn load(&self, _workspace_id: &str) -> Result<Option<Value>, SettingsTransportError> {
        let mut s = self.inner.lock().unwrap();
        s.load_calls += 1;
        Ok(s.load_result.clone())
    }
    fn save(
        &self,
        _workspace_id: &str,
        settings_state: Value,
    ) -> Result<(), SettingsTransportError> {
        let mut s = self.inner.lock().unwrap();
        s.save_calls += 1;
        s.saved = Some(settings_state);
        Ok(())
    }
}

/// Deterministic lifecycle transport: a test can hold an exact GET or PUT in the app's single I/O slot,
/// close Settings, then release the operation and prove the deferred save continues while the dialog is
/// closed. The bounded wait prevents a failing assertion from stranding a worker forever.
struct BlockingSettingsTransport {
    inner: Mutex<BlockingSettingsState>,
    wake: Condvar,
}

#[derive(Default)]
struct BlockingSettingsState {
    block_load: bool,
    block_save: bool,
    load_calls: usize,
    save_calls: usize,
    saved_workspaces: Vec<String>,
    saved_payloads: Vec<Value>,
}

impl BlockingSettingsTransport {
    fn new(block_load: bool, block_save: bool) -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(BlockingSettingsState {
                block_load,
                block_save,
                ..BlockingSettingsState::default()
            }),
            wake: Condvar::new(),
        })
    }

    fn load_calls(&self) -> usize {
        self.inner.lock().unwrap().load_calls
    }

    fn save_calls(&self) -> usize {
        self.inner.lock().unwrap().save_calls
    }

    fn saved_payloads(&self) -> Vec<Value> {
        self.inner.lock().unwrap().saved_payloads.clone()
    }

    fn saved_workspaces(&self) -> Vec<String> {
        self.inner.lock().unwrap().saved_workspaces.clone()
    }

    fn release_load(&self) {
        self.inner.lock().unwrap().block_load = false;
        self.wake.notify_all();
    }

    fn release_save(&self) {
        self.inner.lock().unwrap().block_save = false;
        self.wake.notify_all();
    }

    fn wait_while_blocked<'a>(
        &self,
        mut state: std::sync::MutexGuard<'a, BlockingSettingsState>,
        blocked: impl Fn(&BlockingSettingsState) -> bool,
        operation: &str,
    ) -> Result<std::sync::MutexGuard<'a, BlockingSettingsState>, SettingsTransportError> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while blocked(&state) {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                return Err(SettingsTransportError(format!(
                    "timed out waiting to release blocked settings {operation}"
                )));
            }
            let (next, _) = self.wake.wait_timeout(state, remaining).unwrap();
            state = next;
        }
        Ok(state)
    }
}

impl SettingsTransport for BlockingSettingsTransport {
    fn load(&self, _workspace_id: &str) -> Result<Option<Value>, SettingsTransportError> {
        let mut state = self.inner.lock().unwrap();
        state.load_calls += 1;
        self.wake.notify_all();
        let _state = self.wait_while_blocked(state, |s| s.block_load, "GET")?;
        Ok(None)
    }

    fn save(
        &self,
        workspace_id: &str,
        settings_state: Value,
    ) -> Result<(), SettingsTransportError> {
        let mut state = self.inner.lock().unwrap();
        state.save_calls += 1;
        state.saved_workspaces.push(workspace_id.to_owned());
        state.saved_payloads.push(settings_state);
        self.wake.notify_all();
        let _state = self.wait_while_blocked(state, |s| s.block_save, "PUT")?;
        Ok(())
    }
}

/// Holds workspace A and then workspace B independently so a test can inspect the exact frame between
/// a stale A delivery and B's canonical load result.
struct WorkspaceSwitchSettingsTransport {
    inner: Mutex<WorkspaceSwitchSettingsState>,
    wake: Condvar,
}

#[derive(Default)]
struct WorkspaceSwitchSettingsState {
    block_a: bool,
    block_b: bool,
    load_calls: Vec<String>,
}

impl WorkspaceSwitchSettingsTransport {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(WorkspaceSwitchSettingsState {
                block_a: true,
                block_b: true,
                load_calls: Vec::new(),
            }),
            wake: Condvar::new(),
        })
    }

    fn load_calls(&self) -> Vec<String> {
        self.inner.lock().unwrap().load_calls.clone()
    }

    fn release(&self, workspace: &str) {
        let mut state = self.inner.lock().unwrap();
        match workspace {
            "workspace-a" => state.block_a = false,
            "workspace-b" => state.block_b = false,
            other => panic!("unexpected workspace release: {other}"),
        }
        self.wake.notify_all();
    }
}

impl SettingsTransport for WorkspaceSwitchSettingsTransport {
    fn load(&self, workspace_id: &str) -> Result<Option<Value>, SettingsTransportError> {
        let mut state = self.inner.lock().unwrap();
        state.load_calls.push(workspace_id.to_owned());
        self.wake.notify_all();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while match workspace_id {
            "workspace-a" => state.block_a,
            "workspace-b" => state.block_b,
            other => {
                return Err(SettingsTransportError(format!(
                    "unexpected workspace {other}"
                )))
            }
        } {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                return Err(SettingsTransportError(format!(
                    "timed out waiting to release {workspace_id} GET"
                )));
            }
            (state, _) = self.wake.wait_timeout(state, remaining).unwrap();
        }
        let mut settings = default_workspace_settings_state();
        if workspace_id == "workspace-a" {
            settings.theme = handshake_native::workspace_settings::WorkspaceTheme::Light;
            settings.editor_prefs.editor_font_size = 31.0;
        } else {
            settings.editor_prefs.editor_font_size = 23.0;
        }
        Ok(Some(settings.to_settings_state()))
    }

    fn save(
        &self,
        _workspace_id: &str,
        _settings_state: Value,
    ) -> Result<(), SettingsTransportError> {
        Ok(())
    }
}

#[derive(Default)]
struct CommitThenLoseResponseTransport {
    durable: Mutex<std::collections::HashMap<String, Value>>,
    saves: Mutex<Vec<String>>,
}

impl CommitThenLoseResponseTransport {
    fn save_calls(&self) -> Vec<String> {
        self.saves.lock().unwrap().clone()
    }
}

impl SettingsTransport for CommitThenLoseResponseTransport {
    fn load(&self, workspace_id: &str) -> Result<Option<Value>, SettingsTransportError> {
        Ok(self.durable.lock().unwrap().get(workspace_id).cloned())
    }

    fn save(
        &self,
        workspace_id: &str,
        settings_state: Value,
    ) -> Result<(), SettingsTransportError> {
        self.durable
            .lock()
            .unwrap()
            .insert(workspace_id.to_owned(), settings_state);
        let mut saves = self.saves.lock().unwrap();
        saves.push(workspace_id.to_owned());
        if workspace_id == "workspace-a"
            && saves.iter().filter(|id| *id == workspace_id).count() == 1
        {
            Err(SettingsTransportError(
                "response lost after durable commit".to_owned(),
            ))
        } else {
            Ok(())
        }
    }
}

#[derive(Default)]
struct AlwaysFailSettingsTransport {
    save_calls: std::sync::atomic::AtomicUsize,
}

impl SettingsTransport for AlwaysFailSettingsTransport {
    fn load(&self, _workspace_id: &str) -> Result<Option<Value>, SettingsTransportError> {
        Ok(None)
    }

    fn save(
        &self,
        _workspace_id: &str,
        _settings_state: Value,
    ) -> Result<(), SettingsTransportError> {
        self.save_calls
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Err(SettingsTransportError(
            "deterministic repeated PUT failure".to_owned(),
        ))
    }
}

struct FailFirstLiveSettingsTransport {
    client: SettingsClient,
    failing_client: SettingsClient,
    fail_next_save: std::sync::atomic::AtomicBool,
    load_calls: std::sync::atomic::AtomicUsize,
    save_calls: std::sync::atomic::AtomicUsize,
    successful_saves: std::sync::atomic::AtomicUsize,
    last_successful_payload: Mutex<Option<Value>>,
}

impl FailFirstLiveSettingsTransport {
    fn new(client: SettingsClient, failing_client: SettingsClient) -> Arc<Self> {
        Arc::new(Self {
            client,
            failing_client,
            fail_next_save: std::sync::atomic::AtomicBool::new(true),
            load_calls: std::sync::atomic::AtomicUsize::new(0),
            save_calls: std::sync::atomic::AtomicUsize::new(0),
            successful_saves: std::sync::atomic::AtomicUsize::new(0),
            last_successful_payload: Mutex::new(None),
        })
    }
}

impl SettingsTransport for FailFirstLiveSettingsTransport {
    fn load(&self, workspace_id: &str) -> Result<Option<Value>, SettingsTransportError> {
        self.load_calls
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.client.load(workspace_id)
    }

    fn save(
        &self,
        workspace_id: &str,
        settings_state: Value,
    ) -> Result<(), SettingsTransportError> {
        self.save_calls
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if self
            .fail_next_save
            .swap(false, std::sync::atomic::Ordering::SeqCst)
        {
            return self.failing_client.save(workspace_id, settings_state);
        }
        let request_payload = settings_state.clone();
        self.client.save(workspace_id, settings_state)?;
        *self.last_successful_payload.lock().unwrap() = Some(request_payload);
        self.successful_saves
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }
}

fn spawn_one_http_503() -> (String, std::thread::JoinHandle<()>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind one-shot HTTP 503");
    let address = listener.local_addr().expect("one-shot HTTP address");
    listener
        .set_nonblocking(true)
        .expect("bound one-shot accept timeout");
    let worker = std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(
                        std::time::Instant::now() < deadline,
                        "timed out waiting for failing settings PUT"
                    );
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(error) => panic!("accept settings PUT: {error}"),
            }
        };
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .expect("bound one-shot read timeout");
        let mut request_prefix = [0u8; 4096];
        let read_deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        let read = loop {
            match stream.read(&mut request_prefix) {
                Ok(0) => panic!("failing settings PUT closed before sending a request"),
                Ok(read) => break read,
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) =>
                {
                    assert!(
                        std::time::Instant::now() < read_deadline,
                        "timed out reading failing settings PUT"
                    );
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(error) => panic!("read settings PUT: {error}"),
            }
        };
        assert!(
            std::str::from_utf8(&request_prefix[..read])
                .unwrap_or_default()
                .starts_with("PUT "),
            "failure proof must receive a real HTTP PUT"
        );
        stream
            .write_all(
                b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 20\r\nConnection: close\r\n\r\nsettings unavailable",
            )
            .expect("write HTTP 503");
    });
    (format!("http://{address}"), worker)
}

fn test_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("runtime")
}

fn shared_runtime_handle() -> tokio::runtime::Handle {
    static RUNTIME: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();
    RUNTIME.get_or_init(test_runtime).handle().clone()
}

fn run_until(
    harness: &mut Harness<'_, HandshakeApp>,
    max: usize,
    pred: impl Fn(&HandshakeApp) -> bool,
) -> bool {
    for _ in 0..max {
        // Bounded frame pump instead of idle-wait `run()`: when a focused text field / mounted code panel
        // keeps requesting repaints (egui's blinking-cursor animation, text_selection/visuals), `run()`
        // exceeds its default max_steps and PANICS. `run_steps` pumps a fixed number of frames without that
        // panic — the same harness-regression fix the MT-104 handoff applied to test_settings_dialog.
        harness.run_steps(2);
        if pred(harness.state()) {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    pred(harness.state())
}

fn snapshot_harness<S>(harness: &mut Harness<'_, S>) -> UiTreeSnapshot {
    let mut children = Vec::new();
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        let author_id = ak.author_id().map(str::to_owned);
        let node_id = ak.id().0;
        let actions = ARGUS_PROBE_ACTIONS
            .iter()
            .filter(|action| ak.data().supports_action(**action))
            .map(|action| format!("{action:?}"))
            .collect();
        children.push(UiTreeNode {
            id: author_id
                .clone()
                .unwrap_or_else(|| format!("node:{node_id}")),
            author_id,
            node_id,
            role: format!("{:?}", ak.role()),
            label: ak.label(),
            value: ak.value(),
            disabled: ak.is_disabled(),
            actions,
            bounds: None::<UiNodeBounds>,
            children: Vec::new(),
        });
    }
    let widget_count = children.len() + 1;
    UiTreeSnapshot {
        root: UiTreeNode {
            id: "node:settings-argus-proof-root".to_owned(),
            author_id: None,
            node_id: 0,
            role: "Window".to_owned(),
            label: None,
            value: None,
            disabled: false,
            actions: Vec::new(),
            bounds: None,
            children,
        },
        captured_at_utc: "0.000000000Z".to_owned(),
        widget_count,
    }
}

fn drive_argus_control(
    harness: &mut Harness<'_, HandshakeApp>,
    method: &str,
    target: &str,
    value: Option<&str>,
) {
    let snapshot = snapshot_harness(harness);
    let live = snapshot
        .find_by_author_id(target)
        .unwrap_or_else(|| panic!("actual mounted Settings control '{target}' is absent"));
    assert!(
        !live.disabled,
        "actual mounted Settings control '{target}' is disabled"
    );
    let token = handshake_native::mcp::SessionToken::from_hex("settings-argus-proof");
    let mut params = serde_json::json!({"target": target});
    if let Some(value) = value {
        params["value"] = Value::String(value.to_owned());
    }
    let request = handshake_native::mcp::McpRequest {
        id: serde_json::json!(73),
        method: method.to_owned(),
        params,
        session_token: "settings-argus-proof".to_owned(),
    };
    let mut channel = handshake_native::mcp::ActionChannel::new();
    let response =
        handshake_native::mcp::dispatch_request(&request, &token, &snapshot, &mut channel, || {
            Err(handshake_native::mcp::ScreenshotError(
                "not used".to_owned(),
            ))
        });
    assert_eq!(
        response.to_json()["result"]["queued"],
        true,
        "canonical Argus operation failed for actual mounted control '{target}': {}",
        response.to_json()
    );
    let receipt_id = response.to_json()["result"]["receipt_id"]
        .as_u64()
        .expect("queued Argus action has receipt id");
    for event in channel.drain_revalidated_into_events(&snapshot) {
        harness.event(event);
    }
    harness.run_steps(3);
    let observed = snapshot_harness(harness);
    channel.acknowledge_after_render(&observed);
    let receipt = channel
        .receipts()
        .into_iter()
        .find(|receipt| receipt.receipt_id == receipt_id)
        .expect("Argus action receipt retained");
    if method == handshake_native::mcp::ARGUS_SET_VALUE_METHOD {
        assert_eq!(
            receipt.status,
            handshake_native::mcp::ActionReceiptStatus::Indeterminate,
            "set-value must expose exact mounted readback without claiming causal attribution: {receipt:?}"
        );
    } else {
        assert!(
            matches!(
                receipt.status,
                handshake_native::mcp::ActionReceiptStatus::Applied
                    | handshake_native::mcp::ActionReceiptStatus::Indeterminate
            ),
            "click must be terminal without fabricating success: {receipt:?}"
        );
    }
    if target.starts_with("settings-syntax-swatch-") {
        assert_eq!(
            receipt.observed_value.as_deref(),
            value,
            "Argus receipt must expose the exact mounted swatch value"
        );
    }
}

#[test]
fn editor_settings_persist_managed_postgres_all_fields_retry_and_reopen_round_trip() {
    use handshake_native::code_editor::keymap::{CodeEditorAction, KeyChord};
    use handshake_native::rich_editor::formatting::commands::FormattingCommand;
    use handshake_native::settings_dialog::SETTINGS_PERSIST_RETRY_AUTHOR_ID;

    let mut backend = pg_proof_support::require_live_backend();
    let runtime = test_runtime();
    let production_client = SettingsClient::new(backend.base.clone(), runtime.handle().clone());
    let (failure_base, failure_server) = spawn_one_http_503();
    let failing_client = SettingsClient::new(failure_base, runtime.handle().clone());
    // A WP-011-era row still carried the complete backend-validated shell keybinding map; it simply
    // predated the three MT-072 editor keys. Build that real legacy shape from the canonical defaults
    // and remove only the fields that did not exist yet.
    let mut legacy_settings = default_workspace_settings_state().to_settings_state();
    let legacy = legacy_settings
        .as_object_mut()
        .expect("canonical workspace settings serialize as an object");
    legacy.remove("editor_prefs");
    legacy.remove("syntax_palette");
    legacy.remove("editor_keybindings");
    production_client
        .save(&backend.workspace_id, legacy_settings.clone())
        .expect("PUT legacy/default settings into managed workspace");
    assert_eq!(
        production_client
            .load(&backend.workspace_id)
            .expect("GET managed legacy/default settings"),
        Some(legacy_settings),
        "managed proof starts from an actual legacy document without editor keys"
    );

    let flaky = FailFirstLiveSettingsTransport::new(production_client.clone(), failing_client);
    let mut app = ok_app();
    app.set_runtime_handle(runtime.handle().clone());
    app.bind_active_project_for_integration_test(backend.workspace_id.clone());
    app.set_settings_transport(flaky.clone());
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.state_mut().open_settings();
    assert!(
        run_until(&mut harness, 120, |_| {
            flaky.load_calls.load(std::sync::atomic::Ordering::SeqCst) >= 1
        }),
        "real GET /settings completed on first open"
    );
    let defaults = default_workspace_settings_state();
    assert_eq!(
        harness.state().workspace_settings().editor_prefs,
        defaults.editor_prefs,
        "legacy managed row supplies every missing Editor preference default"
    );
    assert_eq!(
        harness.state().workspace_settings().syntax_palette,
        defaults.syntax_palette,
        "legacy managed row supplies the missing syntax palette default"
    );
    assert!(
        harness
            .state()
            .workspace_settings()
            .editor_keybindings
            .is_empty(),
        "legacy managed row supplies no invented editor keybinding overrides"
    );
    let chrome_theme_before = harness.state().workspace_settings().theme;

    let prefs = EditorPrefs {
        editor_font_size: 19.5,
        tab_size: 3,
        insert_spaces: false,
        word_wrap: WordWrapMode::BoundedColumn(96),
        render_whitespace: RenderWhitespaceMode::Boundary,
        minimap_enabled: false,
        sticky_scroll: false,
        line_numbers: false,
        line_height: 1.4,
        bracket_matching: false,
        indent_guides: false,
        reading_mode_default: true,
    };
    let mut syntax = SyntaxPalette {
        mode: SyntaxPaletteMode::Custom,
        custom: Default::default(),
    };
    for (index, scope) in SYNTAX_SCOPE_KEYS.iter().enumerate() {
        syntax.set_custom(
            scope,
            [10 + index as u8, 40 + index as u8, 90 + index as u8, 255],
        );
    }
    // Drive every declared mutable Editor field through its actual mounted widget. Combo boxes and
    // text/swatch inputs consume addressed SetValue data; DragValues consume NumericValue; checkboxes
    // consume Click. No SettingsOutcome is injected by this managed proof.
    for (target, value) in [
        (EDITOR_FONT_SIZE_AUTHOR_ID, "19.5"),
        (EDITOR_TAB_SIZE_AUTHOR_ID, "3"),
    ] {
        drive_argus_control(
            &mut harness,
            handshake_native::mcp::ARGUS_SET_VALUE_METHOD,
            target,
            Some(value),
        );
    }
    for target in [
        EDITOR_INSERT_SPACES_AUTHOR_ID,
        EDITOR_MINIMAP_AUTHOR_ID,
        EDITOR_STICKY_SCROLL_AUTHOR_ID,
        EDITOR_LINE_NUMBERS_AUTHOR_ID,
        EDITOR_BRACKET_MATCHING_AUTHOR_ID,
        EDITOR_INDENT_GUIDES_AUTHOR_ID,
        EDITOR_READING_MODE_DEFAULT_AUTHOR_ID,
    ] {
        drive_argus_control(
            &mut harness,
            handshake_native::mcp::ARGUS_CLICK_METHOD,
            target,
            None,
        );
    }
    drive_argus_control(
        &mut harness,
        handshake_native::mcp::ARGUS_SET_VALUE_METHOD,
        EDITOR_WORD_WRAP_AUTHOR_ID,
        Some("bounded"),
    );
    drive_argus_control(
        &mut harness,
        handshake_native::mcp::ARGUS_SET_VALUE_METHOD,
        EDITOR_WRAP_COLUMN_AUTHOR_ID,
        Some("96"),
    );
    drive_argus_control(
        &mut harness,
        handshake_native::mcp::ARGUS_SET_VALUE_METHOD,
        EDITOR_RENDER_WHITESPACE_AUTHOR_ID,
        Some("boundary"),
    );
    drive_argus_control(
        &mut harness,
        handshake_native::mcp::ARGUS_SET_VALUE_METHOD,
        EDITOR_LINE_HEIGHT_AUTHOR_ID,
        Some("1.4"),
    );
    drive_argus_control(
        &mut harness,
        handshake_native::mcp::ARGUS_SET_VALUE_METHOD,
        SYNTAX_PALETTE_MODE_AUTHOR_ID,
        Some("custom"),
    );
    for (index, target) in SYNTAX_SWATCH_AUTHOR_IDS.iter().enumerate() {
        let rgba = format!(
            "#{:02x}{:02x}{:02x}ff",
            10 + index as u8,
            40 + index as u8,
            90 + index as u8
        );
        drive_argus_control(
            &mut harness,
            handshake_native::mcp::ARGUS_SET_VALUE_METHOD,
            target,
            Some(&rgba),
        );
    }

    drive_argus_control(
        &mut harness,
        handshake_native::mcp::ARGUS_SET_VALUE_METHOD,
        SETTINGS_SEARCH_AUTHOR_ID,
        Some("keybinding"),
    );
    drive_argus_control(
        &mut harness,
        handshake_native::mcp::ARGUS_CLICK_METHOD,
        "settings.section.keybindings-editor",
        None,
    );
    let keybind_target = editor_keybind_row_author_id("code.open_find");
    drive_argus_control(
        &mut harness,
        handshake_native::mcp::ARGUS_SET_VALUE_METHOD,
        &keybind_target,
        Some("Ctrl+Alt+F"),
    );
    let rich_keybind_target = editor_keybind_row_author_id("rich.toggle_bold");
    drive_argus_control(
        &mut harness,
        handshake_native::mcp::ARGUS_SET_VALUE_METHOD,
        &rich_keybind_target,
        Some("Ctrl+Alt+B"),
    );
    assert_eq!(harness.state().workspace_settings().editor_prefs, prefs);
    assert_eq!(harness.state().workspace_settings().syntax_palette, syntax);
    assert_eq!(
        harness.state().workspace_settings().theme,
        chrome_theme_before,
        "editor widget changes do not mutate the separate chrome theme"
    );
    let expected = harness.state().workspace_settings().clone();

    let code_chord = KeyChord::new(egui::Key::F, true, true, false, false);
    assert_eq!(
        harness
            .state()
            .mounted_code_panel()
            .keymap()
            .resolve(code_chord),
        Some(CodeEditorAction::OpenFind),
        "code override is live before persistence"
    );
    let rich_modifiers = egui::Modifiers {
        ctrl: true,
        command: true,
        alt: true,
        ..Default::default()
    };
    assert_eq!(
        harness
            .state()
            .mounted_rich_state()
            .lock()
            .unwrap()
            .rich_keymap()
            .resolve(&rich_modifiers, egui::Key::B),
        Some(FormattingCommand::ToggleBold),
        "rich override is live before persistence"
    );

    assert!(
        run_until(&mut harness, 160, |app| {
            flaky.save_calls.load(std::sync::atomic::Ordering::SeqCst) >= 1
                && app.settings_persist_error().is_some()
        }),
        "first PUT failure becomes visible without losing live edits"
    );
    failure_server
        .join()
        .expect("one-shot real HTTP failure server reaped");
    assert_eq!(harness.state().workspace_settings(), &expected);
    assert!(
        harness
            .root()
            .children_recursive()
            .any(|node| node.accesskit_node().author_id() == Some(SETTINGS_PERSIST_RETRY_AUTHOR_ID)),
        "typed settings retry control is live after failed PUT"
    );
    drive_argus_control(
        &mut harness,
        handshake_native::mcp::ARGUS_CLICK_METHOD,
        SETTINGS_PERSIST_RETRY_AUTHOR_ID,
        None,
    );
    assert!(
        run_until(&mut harness, 160, |_| {
            flaky
                .successful_saves
                .load(std::sync::atomic::Ordering::SeqCst)
                >= 1
        }),
        "Retry re-dispatched the retained settings PUT"
    );

    let persisted = production_client
        .load(&backend.workspace_id)
        .expect("GET settings after retry")
        .expect("retry created managed settings row");
    assert_eq!(
        normalize_workspace_settings_state(&persisted, &default_workspace_settings_state()),
        expected,
        "every editor field, palette swatch, and code/rich override survived real PUT->GET"
    );
    let successful_put = flaky
        .last_successful_payload
        .lock()
        .unwrap()
        .clone()
        .expect("successful retry captured the exact PUT request payload");
    assert_eq!(
        persisted, successful_put,
        "the managed GET response must exactly match the successful retry PUT request"
    );
    println!("MT-072 managed workspace_id={}", backend.workspace_id);
    println!(
        "MT-072 successful PUT /workspaces/{}/settings payload={}",
        backend.workspace_id, successful_put
    );
    println!(
        "MT-072 GET /workspaces/{}/settings payload={}",
        backend.workspace_id, persisted
    );

    let mut reopened = ok_app();
    reopened.set_runtime_handle(runtime.handle().clone());
    reopened.bind_active_project_for_integration_test(backend.workspace_id.clone());
    reopened.set_settings_transport(Arc::new(production_client));
    let mut reopened_harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), reopened);
    reopened_harness.state_mut().open_settings();
    assert!(
        run_until(&mut reopened_harness, 160, |app| {
            app.workspace_settings() == &expected
        }),
        "fresh app GET/reopen restored every editor setting"
    );
    assert_eq!(
        reopened_harness
            .state()
            .mounted_code_panel()
            .keymap()
            .resolve(code_chord),
        Some(CodeEditorAction::OpenFind),
        "reopen reapplied code keymap"
    );
    assert_eq!(
        reopened_harness
            .state()
            .mounted_rich_state()
            .lock()
            .unwrap()
            .rich_keymap()
            .resolve(&rich_modifiers, egui::Key::B),
        Some(FormattingCommand::ToggleBold),
        "reopen reapplied rich keymap"
    );
    let current_head = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("read current git head for managed evidence");
    assert!(current_head.status.success(), "git rev-parse HEAD failed");
    println!(
        "MT-072 current-head={} fresh-reopen=verified",
        String::from_utf8(current_head.stdout).unwrap().trim()
    );
    drop(reopened_harness);
    drop(harness);
    drop(flaky);
    backend.assert_cleanup();
}

#[test]
fn deferred_settings_save_continues_after_close_during_get() {
    let transport = BlockingSettingsTransport::new(true, false);
    let mut app = ok_app();
    app.set_runtime_handle(shared_runtime_handle());
    app.set_settings_transport(transport.clone());
    app.open_settings();

    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    assert!(
        run_until(&mut harness, 80, |_| transport.load_calls() == 1),
        "the exact settings GET entered the shared I/O slot"
    );

    harness
        .state_mut()
        .apply_settings_outcome_for_test(SettingsOutcome::ThemeChanged(
            handshake_native::workspace_settings::WorkspaceTheme::Light,
        ));
    harness.state_mut().close_settings();
    assert!(!harness.state().settings_open(), "Settings is closed");
    assert_eq!(
        transport.save_calls(),
        0,
        "the pending PUT is deferred while the GET owns the I/O slot"
    );

    transport.release_load();
    assert!(
        run_until(&mut harness, 120, |_| transport.save_calls() == 1),
        "the deferred PUT continues after the closed dialog's GET completes"
    );
    let payloads = transport.saved_payloads();
    assert_eq!(payloads.len(), 1, "exactly one deferred PUT is issued");
    assert_eq!(
        payloads[0].get("theme").and_then(Value::as_str),
        Some("light"),
        "the close-during-GET PUT preserves the operator's in-memory change"
    );
}

#[test]
fn workspace_switch_drops_stale_a_load_and_waits_for_b_canonical_load() {
    let transport = WorkspaceSwitchSettingsTransport::new();
    let mut app = ok_app();
    app.set_runtime_handle(shared_runtime_handle());
    app.set_settings_transport(transport.clone());
    app.bind_active_project_for_integration_test("workspace-a");
    app.open_settings();
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);

    assert!(
        run_until(&mut harness, 80, |_| {
            transport.load_calls().iter().any(|id| id == "workspace-a")
        }),
        "workspace A GET started"
    );
    harness
        .state_mut()
        .bind_active_project_for_integration_test("workspace-b");
    harness.run_steps(2);
    transport.release("workspace-a");
    assert!(
        run_until(&mut harness, 80, |_| {
            transport.load_calls().iter().any(|id| id == "workspace-b")
        }),
        "workspace B GET starts after the stale A delivery releases the single I/O slot"
    );
    assert_eq!(
        harness.state().workspace_settings().theme,
        handshake_native::workspace_settings::WorkspaceTheme::Dark,
        "A's delayed Light theme never becomes visible in workspace B"
    );
    assert_eq!(
        harness
            .state()
            .workspace_settings()
            .editor_prefs
            .editor_font_size,
        default_workspace_settings_state()
            .editor_prefs
            .editor_font_size,
        "B retains its workspace-local default while its own GET is blocked"
    );
    transport.release("workspace-b");
    assert!(
        run_until(&mut harness, 80, |app| {
            app.workspace_settings().editor_prefs.editor_font_size == 23.0
        }),
        "B's exact canonical settings apply after B releases"
    );
}

#[test]
fn queued_saves_for_two_workspaces_are_both_dispatched_in_workspace_order() {
    let transport = BlockingSettingsTransport::new(false, true);
    let mut app = ok_app();
    app.set_runtime_handle(shared_runtime_handle());
    app.set_settings_transport(transport.clone());
    app.bind_active_project_for_integration_test("workspace-a");
    app.open_settings();
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);

    harness
        .state_mut()
        .apply_settings_outcome_for_test(SettingsOutcome::ThemeChanged(
            handshake_native::workspace_settings::WorkspaceTheme::Light,
        ));
    harness.state_mut().close_settings();
    assert!(
        run_until(&mut harness, 80, |_| transport.save_calls() == 1),
        "workspace A PUT owns the I/O slot"
    );

    harness
        .state_mut()
        .bind_active_project_for_integration_test("workspace-b");
    harness.run_steps(2);
    harness
        .state_mut()
        .apply_settings_outcome_for_test(SettingsOutcome::ViewModeChanged(
            handshake_native::workspace_settings::SettingsViewMode::Sfw,
        ));
    harness.state_mut().close_settings();
    assert_eq!(transport.save_calls(), 1, "B is queued while A is blocked");

    transport.release_save();
    assert!(
        run_until(&mut harness, 160, |_| transport.save_calls() == 2),
        "completion of A arms and dispatches the queued B PUT"
    );
    assert_eq!(
        transport.saved_workspaces(),
        vec!["workspace-a".to_owned(), "workspace-b".to_owned()],
        "the queue preserves exact workspace ownership"
    );
    assert_eq!(
        transport.saved_payloads()[1]
            .pointer("/settings/view_mode")
            .and_then(Value::as_str),
        Some("SFW"),
        "B's PUT carries B's exact snapshot"
    );
}

#[test]
fn committed_put_with_lost_response_is_reconciled_without_flushing_another_workspace() {
    let transport = Arc::new(CommitThenLoseResponseTransport::default());
    let mut app = ok_app();
    app.set_runtime_handle(shared_runtime_handle());
    app.set_settings_transport(transport.clone());
    app.bind_active_project_for_integration_test("workspace-a");
    app.open_settings();
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(3);

    harness
        .state_mut()
        .apply_settings_outcome_for_test(SettingsOutcome::ThemeChanged(
            handshake_native::workspace_settings::WorkspaceTheme::Light,
        ));
    harness.state_mut().close_settings();
    assert!(
        run_until(&mut harness, 120, |app| app
            .settings_persist_error()
            .is_some()),
        "the lost response records an ambiguous save failure"
    );
    assert_eq!(transport.save_calls(), vec!["workspace-a".to_owned()]);

    harness.state_mut().open_settings();
    assert!(
        run_until(&mut harness, 120, |app| {
            app.settings_persist_error().is_none()
                && app.workspace_settings().theme
                    == handshake_native::workspace_settings::WorkspaceTheme::Light
        }),
        "exact GET readback retires the false failure and preserves the committed value"
    );

    harness
        .state_mut()
        .bind_active_project_for_integration_test("workspace-b");
    harness.run_steps(2);
    harness
        .state_mut()
        .apply_settings_outcome_for_test(SettingsOutcome::ViewModeChanged(
            handshake_native::workspace_settings::SettingsViewMode::Sfw,
        ));
    harness
        .state_mut()
        .bind_active_project_for_integration_test("workspace-a");
    let retry_applied = harness
        .state_mut()
        .apply_settings_outcome_for_test(SettingsOutcome::RetryPersistence);
    assert!(
        !retry_applied,
        "reconciled A exposes no stale Retry Save operation"
    );
    assert_eq!(
        transport.save_calls(),
        vec!["workspace-a".to_owned()],
        "A's stale retry cannot flush queued workspace B"
    );
}

#[test]
fn repeated_failed_put_survives_close_reopen_and_keeps_retry_addressable() {
    let transport = Arc::new(AlwaysFailSettingsTransport::default());
    let mut app = ok_app();
    app.set_runtime_handle(shared_runtime_handle());
    app.set_settings_transport(transport.clone());
    app.bind_active_project_for_integration_test("workspace-retry");
    app.open_settings();
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    assert!(
        run_until(&mut harness, 80, |_| true),
        "initial settings frame rendered"
    );

    let mut changed = harness.state().workspace_settings().editor_prefs.clone();
    changed.editor_font_size = 27.0;
    harness
        .state_mut()
        .apply_settings_outcome_for_test(SettingsOutcome::EditorPrefsChanged(changed));
    harness.state_mut().close_settings();
    assert!(
        run_until(&mut harness, 120, |app| {
            app.settings_persist_error().is_some()
        }),
        "the failed close-flushed PUT is surfaced while Settings is closed"
    );
    let closed_snapshot = snapshot_harness(&mut harness);
    assert!(
        closed_snapshot
            .find_by_author_id(handshake_native::settings_dialog::SETTINGS_PERSIST_ERROR_AUTHOR_ID)
            .is_some(),
        "closed Settings exposes a stable error status"
    );
    assert!(
        closed_snapshot
            .find_by_author_id(handshake_native::settings_dialog::SETTINGS_PERSIST_RETRY_AUTHOR_ID)
            .is_some(),
        "closed Settings exposes a stable retry control"
    );
    drive_argus_control(
        &mut harness,
        handshake_native::mcp::ARGUS_CLICK_METHOD,
        handshake_native::settings_dialog::SETTINGS_PERSIST_RETRY_AUTHOR_ID,
        None,
    );
    assert!(
        run_until(&mut harness, 120, |app| {
            transport
                .save_calls
                .load(std::sync::atomic::Ordering::SeqCst)
                >= 2
                && app.settings_persist_error().is_some()
        }),
        "a second failed retry returns to the same addressable error state"
    );
    harness.state_mut().open_settings();
    assert!(
        run_until(&mut harness, 120, |app| {
            app.workspace_settings().editor_prefs.editor_font_size == 27.0
        }),
        "reopen and its remote/default GET preserve the exact unsaved local editor value"
    );
}

#[test]
fn deferred_settings_save_continues_after_close_during_put() {
    let transport = BlockingSettingsTransport::new(false, true);
    let mut app = ok_app();
    app.set_runtime_handle(shared_runtime_handle());
    app.set_settings_transport(transport.clone());
    app.open_settings();

    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    assert!(
        run_until(&mut harness, 80, |_| transport.load_calls() == 1),
        "the initial settings GET completed"
    );
    harness.run_steps(4);

    harness
        .state_mut()
        .apply_settings_outcome_for_test(SettingsOutcome::ThemeChanged(
            handshake_native::workspace_settings::WorkspaceTheme::Light,
        ));
    assert!(
        run_until(&mut harness, 120, |_| transport.save_calls() == 1),
        "the first PUT is in flight"
    );

    // Change a second field while PUT #1 is blocked, then close. The close flush finds the I/O slot
    // busy and must retain a deferred PUT for the new snapshot after the dialog disappears.
    harness
        .state_mut()
        .apply_settings_outcome_for_test(SettingsOutcome::ViewModeChanged(
            handshake_native::workspace_settings::SettingsViewMode::Sfw,
        ));
    harness.state_mut().close_settings();
    assert!(!harness.state().settings_open(), "Settings is closed");
    assert_eq!(
        transport.save_calls(),
        1,
        "PUT #2 waits while PUT #1 owns the I/O slot"
    );

    transport.release_save();
    assert!(
        run_until(&mut harness, 120, |_| transport.save_calls() == 2),
        "the deferred second PUT continues after the first PUT completes with Settings closed"
    );
    let payloads = transport.saved_payloads();
    assert_eq!(payloads.len(), 2, "exactly two serialized PUTs are issued");
    assert_eq!(
        payloads[1].get("theme").and_then(Value::as_str),
        Some("light")
    );
    assert_eq!(
        payloads[1]
            .pointer("/settings/view_mode")
            .and_then(Value::as_str),
        Some("SFW"),
        "the deferred snapshot contains the change made during PUT #1"
    );
}

// ── AC-001 / AC-002 / AC-009: editor prefs persist via the existing PUT; distinct from chrome ────────
#[test]
fn editor_prefs_change_persists_via_existing_put_and_reloads() {
    let transport = StubSettingsTransport::with_loaded(None);
    let handle = shared_runtime_handle();

    let mut app = ok_app();
    app.set_runtime_handle(handle);
    app.set_settings_transport(transport.clone());

    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.state_mut().open_settings();
    harness.run();

    // The Editor section renders (its header is in the live tree).
    assert!(
        harness.query_by_label("Editor").is_some(),
        "AC-008/AC-001: the Editor settings section renders"
    );
    let flight_recorder_posture = harness.get_by_label(FLIGHT_RECORDER_SETTINGS_POSTURE_NOTE);
    assert_eq!(
        flight_recorder_posture.accesskit_node().author_id(),
        Some(FLIGHT_RECORDER_SETTINGS_POSTURE_AUTHOR_ID),
        "Flight Recorder runtime-derived/no-dedicated-preference posture is directly discoverable in Editor Settings"
    );

    let chrome_theme_before = harness.state().workspace_settings().theme;

    // Apply a full editor-prefs change through the SAME outcome path the live controls produce (a kittest
    // cannot reliably drag an egui DragValue / click a ComboBox popup item; the dialog's WIRING is what
    // the AC requires — the section returns EditorPrefsChanged, the shell stores it + schedules the PUT).
    let new_prefs = EditorPrefs {
        editor_font_size: 22.0,
        tab_size: 8,
        insert_spaces: false,
        word_wrap: WordWrapMode::BoundedColumn(100),
        render_whitespace: RenderWhitespaceMode::All,
        // MT-035: the minimap / sticky-scroll / line-number toggles default to `true`; this case keeps them
        // at their defaults (their live-wiring is proven in the dedicated MT-035 toggle test).
        ..EditorPrefs::default()
    };
    harness
        .state_mut()
        .apply_settings_outcome_for_test(SettingsOutcome::EditorPrefsChanged(new_prefs));
    harness.run();

    // The live settings now hold the new prefs.
    assert_eq!(
        harness.state().workspace_settings().editor_prefs,
        new_prefs,
        "AC-001: the editor prefs change is held in the live settings"
    );
    // AC-002: editor font size change did NOT change the chrome theme (separate surfaces).
    assert_eq!(
        harness.state().workspace_settings().theme,
        chrome_theme_before,
        "AC-002: editor font size is a separate field from the chrome appearance"
    );

    // AC-001 / AC-009: the change persists via the existing debounced PUT (the ONLY save surface).
    let saved = run_until(&mut harness, 80, |_| transport.save_calls() >= 1);
    assert!(
        saved,
        "AC-001/AC-009: editor prefs persisted via PUT /workspaces/{{id}}/settings"
    );

    let blob = transport.saved().expect("a settings_state blob was PUT");
    let obj = blob.as_object().expect("settings_state is an object");

    // AC-001: the PUT blob carries all five editor pref values under editor_prefs.
    let ep = obj
        .get("editor_prefs")
        .and_then(Value::as_object)
        .expect("editor_prefs key");
    assert_eq!(
        ep.get("editor_font_size").and_then(Value::as_f64),
        Some(22.0)
    );
    assert_eq!(ep.get("tab_size").and_then(Value::as_u64), Some(8));
    assert_eq!(
        ep.get("insert_spaces").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        ep.get("render_whitespace").and_then(Value::as_str),
        Some("all")
    );
    assert_eq!(
        ep.get("word_wrap")
            .and_then(|w| w.get("boundedColumn"))
            .and_then(Value::as_u64),
        Some(100),
        "AC-001: bounded word-wrap column round-trips through the PUT blob"
    );

    // AC-002: editor_font_size is under editor_prefs, NOT a top-level chrome key; theme is its own key.
    assert!(
        !obj.contains_key("editor_font_size"),
        "AC-002: editor font size is NOT a chrome top-level key"
    );
    assert!(
        obj.contains_key("theme"),
        "AC-002: chrome appearance (theme) is its own top-level key"
    );

    // AC-001 (reload side): a NEW app GET-loading this exact blob reloads identical editor prefs.
    let reload_transport = StubSettingsTransport::with_loaded(Some(blob));
    let handle2 = shared_runtime_handle();
    let mut app2 = ok_app();
    app2.set_runtime_handle(handle2);
    app2.set_settings_transport(reload_transport.clone());
    let mut harness2 =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app2);
    harness2.state_mut().open_settings();
    let loaded = run_until(&mut harness2, 80, |app| {
        reload_transport.load_calls() >= 1 && app.workspace_settings().editor_prefs == new_prefs
    });
    assert!(
        loaded,
        "AC-001: reopening (GET) reloads the SAME editor prefs that were PUT (got {:?})",
        harness2.state().workspace_settings().editor_prefs
    );
}

// ── AC-001 (LIVE side) / MT-072 note 87: editor prefs WIRE INTO the mounted editors ────────────────
//
// Persistence (above) proves the blob is PUT. This proves the WIRE-INTO-LIVE half: applying an
// EditorPrefsChanged outcome (and loading prefs from a stored blob) drives the live mounted
// `CodeEditorPanel` and rich editor state — tab size / insert-spaces / render-whitespace / word-wrap /
// editor_font_size reflect the new values in the same frame, NOT only the persisted struct.
#[test]
fn editor_prefs_change_drives_the_live_mounted_editors() {
    let transport = StubSettingsTransport::with_loaded(None);
    let handle = shared_runtime_handle();
    let mut app = ok_app();
    app.set_runtime_handle(handle);
    app.set_settings_transport(transport);

    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.state_mut().open_settings();
    harness.run();

    // Baseline: the mounted panel holds the seeded defaults (tab 4, spaces on, no whitespace glyphs, no
    // wrap) BEFORE any settings change reaches it.
    let panel0 = harness.state().mounted_code_panel();
    assert_eq!(
        panel0.indent_settings(),
        (4, true),
        "baseline indent = default (4, spaces)"
    );
    assert!(
        !panel0.render_whitespace(),
        "baseline render-whitespace OFF"
    );
    assert!(!panel0.is_wrap_enabled(), "baseline word-wrap OFF");
    {
        let expected = harness
            .state()
            .workspace_settings()
            .editor_prefs
            .editor_font_size;
        let rich = harness.state().mounted_rich_state();
        let rich = rich.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(
            rich.editor_font_size(),
            expected,
            "baseline rich editor font size follows workspace settings"
        );
    }

    // Apply a full editor-prefs change through the same wired outcome the live controls produce.
    let new_prefs = EditorPrefs {
        editor_font_size: 18.0,
        tab_size: 8,
        insert_spaces: false,
        word_wrap: WordWrapMode::BoundedColumn(100),
        render_whitespace: RenderWhitespaceMode::All,
        ..EditorPrefs::default()
    };
    harness
        .state_mut()
        .apply_settings_outcome_for_test(SettingsOutcome::EditorPrefsChanged(new_prefs));
    harness.run();

    // LIVE EFFECT: the SAME mounted panel now reflects the new prefs — proven against the panel's own
    // public state, not the persisted blob.
    let panel = harness.state().mounted_code_panel();
    assert_eq!(
        panel.indent_settings(),
        (8, false),
        "MT-072 note 87: tab_size + insert_spaces wired into the live code panel"
    );
    assert!(
        panel.render_whitespace(),
        "MT-072 note 87: render_whitespace=All draws whitespace on the live panel"
    );
    assert!(
        panel.is_wrap_enabled(),
        "MT-072 note 87: word_wrap enabled on the live panel"
    );
    assert_eq!(
        panel.wrap_config().wrap_column,
        Some(100),
        "MT-072 note 87: BoundedColumn(100) sets the live wrap column"
    );
    assert_eq!(
        panel.font_size(),
        18.0,
        "wave-6 S6 item 3: editor_font_size resizes the live code panel, not only the saved blob"
    );
    {
        let rich = harness.state().mounted_rich_state();
        let rich = rich.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(
            rich.editor_font_size(),
            18.0,
            "wave-6 S6 item 3: editor_font_size resizes the live rich editor, not only the saved blob"
        );
    }
}

// ── WP-KERNEL-012 MT-035: minimap / sticky-scroll / line-number toggles + render-whitespace 3-way ────────

/// Each MT-035 code-editor toggle is FULLY live-wired: changing the setting through the wired
/// `EditorPrefsChanged` outcome changes the MOUNTED code panel's OWN public state (proven against the
/// panel, not the saved blob). No dead toggles: minimap -> `is_minimap_shown`, sticky-scroll ->
/// `sticky_scroll_enabled`, line numbers -> `line_numbers_enabled` (the MT-007 GutterConfig flag), and the
/// render-whitespace mode threads the FULL None/Boundary/All enum (the old Boundary-vs-All lossiness is
/// fixed) into `render_whitespace_mode`.
#[test]
fn mt035_visibility_and_whitespace_toggles_drive_live_code_panel() {
    let transport = StubSettingsTransport::with_loaded(None);
    let handle = shared_runtime_handle();
    let mut app = ok_app();
    app.set_runtime_handle(handle);
    app.set_settings_transport(transport);

    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.state_mut().open_settings();
    harness.run();

    // Baseline: the three visibility features default ON (matching the always-on pre-MT-035 behavior).
    let panel = harness.state().mounted_code_panel();
    assert!(panel.is_minimap_shown(), "minimap defaults on");
    assert!(panel.sticky_scroll_enabled(), "sticky scroll defaults on");
    assert!(
        panel.line_numbers_enabled(),
        "gutter line numbers default on"
    );

    // Flip all three OFF + set render-whitespace to Boundary through the SAME wired outcome the live
    // controls produce.
    let prefs = EditorPrefs {
        render_whitespace: RenderWhitespaceMode::Boundary,
        minimap_enabled: false,
        sticky_scroll: false,
        line_numbers: false,
        ..EditorPrefs::default()
    };
    harness
        .state_mut()
        .apply_settings_outcome_for_test(SettingsOutcome::EditorPrefsChanged(prefs));
    harness.run();

    let panel = harness.state().mounted_code_panel();
    assert!(
        !panel.is_minimap_shown(),
        "MT-035: minimap=false disabled the LIVE minimap (set_show_minimap)"
    );
    assert!(
        !panel.sticky_scroll_enabled(),
        "MT-035: sticky_scroll=false disabled the LIVE sticky band (set_sticky_scroll_enabled)"
    );
    assert!(
        !panel.line_numbers_enabled(),
        "MT-035: line_numbers=false disabled LIVE gutter numbers (GutterConfig::show_line_numbers)"
    );
    assert_eq!(
        panel.render_whitespace_mode(),
        RenderWhitespaceMode::Boundary,
        "MT-035: the full Boundary mode threads to the panel (Boundary-vs-All lossiness fixed)"
    );
    assert!(
        panel.render_whitespace(),
        "Boundary still draws glyphs (the bool draw-gate stays true for a non-None mode)"
    );

    // Move the toggles the OTHER direction: re-enable minimap + set render-whitespace to All.
    let prefs2 = EditorPrefs {
        render_whitespace: RenderWhitespaceMode::All,
        minimap_enabled: true,
        ..EditorPrefs::default()
    };
    harness
        .state_mut()
        .apply_settings_outcome_for_test(SettingsOutcome::EditorPrefsChanged(prefs2));
    harness.run();
    let panel = harness.state().mounted_code_panel();
    assert!(panel.is_minimap_shown(), "minimap re-enabled");
    assert_eq!(
        panel.render_whitespace_mode(),
        RenderWhitespaceMode::All,
        "MT-035: All mode threads distinctly from Boundary"
    );

    // None mode: the draw-gate bool goes false (no glyphs) — proving None/Boundary/All are all distinct.
    let prefs3 = EditorPrefs {
        render_whitespace: RenderWhitespaceMode::None,
        ..EditorPrefs::default()
    };
    harness
        .state_mut()
        .apply_settings_outcome_for_test(SettingsOutcome::EditorPrefsChanged(prefs3));
    harness.run();
    let panel = harness.state().mounted_code_panel();
    assert_eq!(
        panel.render_whitespace_mode(),
        RenderWhitespaceMode::None,
        "MT-035: None mode threads through"
    );
    assert!(
        !panel.render_whitespace(),
        "None disables whitespace drawing (the bool draw-gate is false)"
    );
}

// ── WP-KERNEL-012 MT-035 wave-7: line-height / bracket-matching / indent-guides / reading-mode-default ──

/// Each MT-035 wave-7 editor setting is FULLY live-wired: changing it through the wired `EditorPrefsChanged`
/// outcome drives the MOUNTED editor state BOTH directions. line_height -> the code panel's
/// `line_height_multiplier`; bracket_matching -> `bracket_matching_enabled`; indent_guides ->
/// `indent_guides_enabled`; reading_mode_default -> the mounted rich state's `reading_mode_default`. No dead
/// toggles — the feature-effect gating is proven per-panel in the `code_editor::panel` + `reading_mode` unit
/// tests; this proves the settings->mounted-state wiring end to end.
#[test]
fn mt035_wave7_settings_drive_live_mounted_editors() {
    let transport = StubSettingsTransport::with_loaded(None);
    let handle = shared_runtime_handle();
    let mut app = ok_app();
    app.set_runtime_handle(handle);
    app.set_settings_transport(transport);

    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.state_mut().open_settings();
    harness.run();

    // Baseline: the wave-7 features default to their always-on / single-spaced / editable state.
    let panel = harness.state().mounted_code_panel();
    assert!(
        (panel.line_height_multiplier() - 1.0).abs() < 1e-4,
        "line height defaults to 1.0 (single-spaced)"
    );
    assert!(
        panel.bracket_matching_enabled(),
        "bracket matching defaults on"
    );
    assert!(panel.indent_guides_enabled(), "indent guides default on");
    assert!(
        !harness
            .state()
            .mounted_rich_state()
            .lock()
            .unwrap()
            .reading_mode_default(),
        "reading-mode default is off (docs open editable)"
    );

    // Change all four through the SAME wired outcome the live controls produce.
    let prefs = EditorPrefs {
        line_height: 1.8,
        bracket_matching: false,
        indent_guides: false,
        reading_mode_default: true,
        ..EditorPrefs::default()
    };
    harness
        .state_mut()
        .apply_settings_outcome_for_test(SettingsOutcome::EditorPrefsChanged(prefs));
    harness.run();

    let panel = harness.state().mounted_code_panel();
    assert!(
        (panel.line_height_multiplier() - 1.8).abs() < 1e-4,
        "wave-7: line_height=1.8 reached the LIVE code panel (set_line_height)"
    );
    assert!(
        !panel.bracket_matching_enabled(),
        "wave-7: bracket_matching=false disabled the LIVE matching-bracket highlight"
    );
    assert!(
        !panel.indent_guides_enabled(),
        "wave-7: indent_guides=false disabled the LIVE indent guides"
    );
    assert!(
        harness
            .state()
            .mounted_rich_state()
            .lock()
            .unwrap()
            .reading_mode_default(),
        "wave-7: reading_mode_default=true reached the LIVE rich state (set_reading_mode_default)"
    );

    // The OTHER direction: restore single-spaced + re-enable the chrome + turn reading-default back off.
    let prefs2 = EditorPrefs {
        line_height: 1.0,
        bracket_matching: true,
        indent_guides: true,
        reading_mode_default: false,
        ..EditorPrefs::default()
    };
    harness
        .state_mut()
        .apply_settings_outcome_for_test(SettingsOutcome::EditorPrefsChanged(prefs2));
    harness.run();

    let panel = harness.state().mounted_code_panel();
    assert!(
        (panel.line_height_multiplier() - 1.0).abs() < 1e-4,
        "wave-7: line height reset to single-spaced"
    );
    assert!(
        panel.bracket_matching_enabled(),
        "wave-7: bracket matching re-enabled"
    );
    assert!(
        panel.indent_guides_enabled(),
        "wave-7: indent guides re-enabled"
    );
    assert!(
        !harness
            .state()
            .mounted_rich_state()
            .lock()
            .unwrap()
            .reading_mode_default(),
        "wave-7: reading-mode default turned back off"
    );
}

#[test]
fn syntax_palette_change_drives_the_live_code_panel_immediately() {
    let transport = StubSettingsTransport::with_loaded(None);
    let handle = shared_runtime_handle();
    let mut app = ok_app();
    app.set_runtime_handle(handle);
    app.set_settings_transport(transport);

    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.state_mut().open_settings();
    harness.run();

    let syntax = HsTheme::Dark.palette().syntax;
    let panel = harness.state().mounted_code_panel();
    assert_eq!(
        panel.resolve_highlight_color(HighlightScope::Keyword, &syntax),
        syntax.keyword,
        "baseline keyword color comes from the active theme before a Custom palette is applied"
    );

    let mut custom = SyntaxPalette {
        mode: SyntaxPaletteMode::Custom,
        custom: Default::default(),
    };
    custom.set_custom(HighlightScope::Keyword.scope_key(), [200, 30, 30, 255]);
    harness
        .state_mut()
        .apply_settings_outcome_for_test(SettingsOutcome::SyntaxPaletteChanged(custom));
    harness.run();
    let panel = harness.state().mounted_code_panel();
    assert_eq!(
        panel.resolve_highlight_color(HighlightScope::Keyword, &syntax),
        egui::Color32::from_rgba_unmultiplied(200, 30, 30, 255),
        "wave-6 S6 item 3: SyntaxPaletteChanged repaints the mounted panel immediately"
    );

    harness
        .state_mut()
        .apply_settings_outcome_for_test(SettingsOutcome::SyntaxPaletteChanged(
            SyntaxPalette::default(),
        ));
    harness.run();
    let panel = harness.state().mounted_code_panel();
    assert_eq!(
        panel.resolve_highlight_color(HighlightScope::Keyword, &syntax),
        syntax.keyword,
        "Custom -> Standard clears the live Custom override immediately"
    );
}

// ── AC-001 (LIVE side, load path): editor prefs from a STORED blob apply to the live panel on load ───
#[test]
fn loaded_editor_prefs_apply_to_the_live_code_panel() {
    // A stored blob carrying non-default editor prefs (tab 2, hard tabs, whitespace boundary, wrap on).
    let stored = serde_json::json!({
        "schema_id": "hsk.workspace_settings_state@1",
        "theme": "dark",
        "custom_theme_tokens": {},
        "keybindings": {},
        "settings": { "view_mode": "NSFW", "swarm_board_default_open": false },
        "editor_prefs": {
            "editor_font_size": 15.0,
            "tab_size": 2,
            "insert_spaces": false,
            "word_wrap": "on",
            "render_whitespace": "boundary",
        },
    });
    let transport = StubSettingsTransport::with_loaded(Some(stored));
    let handle = shared_runtime_handle();
    let mut app = ok_app();
    app.set_runtime_handle(handle);
    app.set_settings_transport(transport.clone());

    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.state_mut().open_settings();
    let loaded = run_until(&mut harness, 80, |app| {
        transport.load_calls() >= 1 && app.workspace_settings().editor_prefs.tab_size == 2
    });
    assert!(loaded, "the stored blob loaded via GET");

    // The load drain pushed the stored prefs into the live mounted panel (parity with theme/view_mode,
    // which the load drain also applies live).
    let panel = harness.state().mounted_code_panel();
    assert_eq!(
        panel.indent_settings(),
        (2, false),
        "loaded editor prefs (tab 2, hard tabs) applied to the live code panel"
    );
    assert!(
        panel.render_whitespace(),
        "loaded render_whitespace=boundary draws on the live panel"
    );
    assert!(
        panel.is_wrap_enabled(),
        "loaded word_wrap=on enabled wrap on the live panel"
    );
    assert_eq!(
        panel.wrap_config().wrap_column,
        None,
        "word_wrap=on wraps at the viewport edge (no column)"
    );
}

// ── AC-006: a legacy WP-011-era settings doc (no editor keys) loads cleanly via GET ──────────────────
#[test]
fn legacy_settings_doc_loads_cleanly_without_editor_keys() {
    // A WP-011-era blob: valid schema + theme + keybindings + settings, but NO editor_* keys.
    let legacy = serde_json::json!({
        "schema_id": "hsk.workspace_settings_state@1",
        "theme": "dark",
        "custom_theme_tokens": {},
        "keybindings": {
            "app.quick_switcher.open": "Mod-p",
            "app.command_palette.open": "Mod-Shift-p",
        },
        "settings": { "view_mode": "NSFW", "swarm_board_default_open": false },
    });
    let transport = StubSettingsTransport::with_loaded(Some(legacy));
    let handle = shared_runtime_handle();
    let mut app = ok_app();
    app.set_runtime_handle(handle);
    app.set_settings_transport(transport.clone());

    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.state_mut().open_settings();

    // The GET completes and the dialog opens against the legacy doc WITHOUT a hard-fail; the editor
    // fields are the defaults (AC-006).
    let loaded = run_until(&mut harness, 80, |_app| transport.load_calls() >= 1);
    assert!(loaded, "AC-006: the legacy settings doc loaded via GET");
    assert!(
        harness.state().settings_open(),
        "AC-006: the dialog stayed open against a legacy doc"
    );
    assert_eq!(
        harness.state().workspace_settings().editor_prefs,
        EditorPrefs::default(),
        "AC-006: a legacy doc yields the default editor prefs"
    );
    assert_eq!(
        harness.state().workspace_settings().syntax_palette,
        SyntaxPalette::default(),
        "AC-006: a legacy doc yields the default syntax palette"
    );
    assert!(
        harness.state().settings_persist_error().is_none(),
        "AC-006: loading a legacy doc produced no persistence error"
    );
    // And the Editor section still renders (the legacy load did not break the dialog body).
    harness.run();
    assert!(
        harness.query_by_label("Editor").is_some(),
        "AC-006: Editor section renders after legacy load"
    );
}

// ── AC-005 (persistence side) / RISK-001: editor keybinding override persists in the SEPARATE list ───
#[test]
fn editor_keybinding_override_persists_outside_the_app_keybindings_map() {
    let transport = StubSettingsTransport::with_loaded(None);
    let handle = shared_runtime_handle();
    let mut app = ok_app();
    app.set_runtime_handle(handle);
    app.set_settings_transport(transport.clone());

    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.state_mut().open_settings();
    harness.run();

    harness
        .state_mut()
        .apply_settings_outcome_for_test(SettingsOutcome::EditorKeybindingChanged {
            action_id: "code.open_find".to_owned(),
            chord: "Mod+Alt+F".to_owned(),
        });
    harness.run();

    let saved = run_until(&mut harness, 80, |_| transport.save_calls() >= 1);
    assert!(saved, "the editor keybinding override persisted via PUT");

    let blob = transport.saved().expect("a settings_state blob was PUT");
    let obj = blob.as_object().unwrap();

    // RISK-001: the override is in the SEPARATE editor_keybindings list...
    let editor_kb = obj
        .get("editor_keybindings")
        .and_then(Value::as_array)
        .expect("editor_keybindings");
    assert!(
        editor_kb.iter().any(|e| {
            e.get("action").and_then(Value::as_str) == Some("code.open_find")
                && e.get("chord").and_then(Value::as_str) == Some("Mod+Alt+F")
        }),
        "the editor binding is in the separate editor_keybindings list"
    );
    // ...and the WP-011 keybindings map STILL contains ONLY the two backend-allowed app action ids
    // (writing editor bindings there would hard-fail every PUT against the backend validator).
    let kb = obj.get("keybindings").and_then(Value::as_object).unwrap();
    assert_eq!(
        kb.len(),
        2,
        "RISK-001: the backend-validated keybindings map keeps EXACTLY the two app actions, got {:?}",
        kb.keys().collect::<Vec<_>>()
    );
    assert!(
        kb.contains_key("app.quick_switcher.open") && kb.contains_key("app.command_palette.open")
    );
    assert!(
        !kb.contains_key("code.open_find"),
        "RISK-001: the editor binding did NOT leak into the backend-validated keybindings map"
    );
}

// ── MT-072 Fix 1: selecting Muted or Standard recolors the LIVE code panel (not only the preview) ────
//
// Before the fix, `resolve_highlight_color` routed ONLY Custom through the palette resolver, so Muted /
// Standard changed only the Settings preview swatch — the running editor kept theme tokens. This proves
// the live render-path resolver now returns the Muted / Standard TABLE color for every mode, so the live
// editor and the preview swatch agree (mirroring the existing Custom same-frame proof above).
#[test]
fn muted_and_standard_palette_recolor_the_live_code_panel() {
    let transport = StubSettingsTransport::with_loaded(None);
    let handle = shared_runtime_handle();
    let mut app = ok_app();
    app.set_runtime_handle(handle);
    app.set_settings_transport(transport);

    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.state_mut().open_settings();
    harness.run();

    // The `syntax` arg is the theme-token fallback (used only when NO palette is installed); with a palette
    // installed the resolver ignores it and returns the palette-table color.
    let syntax = HsTheme::Dark.palette().syntax;

    // Select Muted: the running panel resolves EVERY scope to the Muted table color (same-frame recolor).
    harness
        .state_mut()
        .apply_settings_outcome_for_test(SettingsOutcome::SyntaxPaletteChanged(SyntaxPalette {
            mode: SyntaxPaletteMode::Muted,
            custom: Default::default(),
        }));
    harness.run();
    let panel = harness.state().mounted_code_panel();
    for scope in HighlightScope::ALL.iter().copied() {
        assert_eq!(
            panel.resolve_highlight_color(scope, &syntax),
            scope.builtin_color(&MUTED_PALETTE),
            "MT-072 Fix 1: Muted recolors the LIVE panel for {scope:?} (not only the preview swatch)"
        );
    }
    // Muted actually DIFFERS from the theme keyword token — proves the live editor recolored, not a no-op.
    assert_ne!(
        panel.resolve_highlight_color(HighlightScope::Keyword, &syntax),
        syntax.keyword,
        "MT-072 Fix 1: Muted keyword differs from the theme token on the live panel"
    );

    // Select Standard: the running panel resolves EVERY scope to the Standard table color.
    harness
        .state_mut()
        .apply_settings_outcome_for_test(SettingsOutcome::SyntaxPaletteChanged(SyntaxPalette {
            mode: SyntaxPaletteMode::Standard,
            custom: Default::default(),
        }));
    harness.run();
    let panel = harness.state().mounted_code_panel();
    for scope in HighlightScope::ALL.iter().copied() {
        assert_eq!(
            panel.resolve_highlight_color(scope, &syntax),
            scope.builtin_color(&STANDARD_PALETTE),
            "MT-072 Fix 1: Standard recolors the LIVE panel for {scope:?}"
        );
    }
}

// ── MT-072 Fix 3 (MT-054 wrap-persistence closeout): a USER Alt+Z / Wrap-button / editor-wrap-toggle
//    change writes back to editor_prefs, persists via the existing PUT, is NOT clobbered by a following
//    prefs->panel sync, and an explicit Settings change still flows prefs->panel. ──────────────────────
#[test]
fn user_wrap_toggle_persists_and_is_not_clobbered_by_sync() {
    let transport = StubSettingsTransport::with_loaded(None);
    let handle = shared_runtime_handle();
    let mut app = ok_app();
    app.set_runtime_handle(handle);
    app.set_settings_transport(transport.clone());

    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.state_mut().open_settings();
    harness.run();

    // Baseline: wrap OFF on both the persisted prefs and the live panel.
    assert_eq!(
        harness.state().workspace_settings().editor_prefs.word_wrap,
        WordWrapMode::Off,
        "baseline persisted word_wrap is Off"
    );
    assert!(
        !harness.state().mounted_code_panel().is_wrap_enabled(),
        "baseline live panel wrap OFF"
    );

    // A USER wrap toggle through the SAME mutation point Alt+Z / the "Wrap" button / the editor-wrap-toggle
    // node route through (proven equivalent to Alt+Z in test_word_wrap). One frame lets the app drain the
    // pending user toggle and write it back into editor_prefs.
    harness.state().mounted_code_panel().toggle_wrap();
    assert!(
        harness.state().mounted_code_panel().is_wrap_enabled(),
        "the user toggle enabled wrap on the live panel"
    );
    harness.run();

    // WRITE-BACK: the persisted editor_prefs now reflects the toggle (it did NOT before this fix).
    assert_eq!(
        harness.state().workspace_settings().editor_prefs.word_wrap,
        WordWrapMode::On,
        "MT-072 Fix 3: a user Alt+Z toggle wrote back to editor_prefs.word_wrap = On"
    );

    // PERSISTENCE: it rides the SAME debounced PUT (the only save surface — AC-009), so it survives restart.
    let saved = run_until(&mut harness, 80, |_| transport.save_calls() >= 1);
    assert!(saved, "the wrap toggle persisted via the existing PUT");
    let blob = transport.saved().expect("a settings_state blob was PUT");
    assert_eq!(
        blob.as_object()
            .and_then(|o| o.get("editor_prefs"))
            .and_then(|e| e.get("word_wrap"))
            .and_then(Value::as_str),
        Some("on"),
        "the PUT blob carries word_wrap = on"
    );

    // NO CLOBBER: a following prefs->panel sync (the EXACT path the bug reported reverting the toggle) must
    // NOT revert the live panel — editor_prefs already equals the panel state, so the sync is a no-op.
    harness.state().sync_editor_prefs_to_panel_for_test();
    harness.run();
    assert!(
        harness.state().mounted_code_panel().is_wrap_enabled(),
        "MT-072 Fix 3: a prefs->panel sync did NOT clobber the user wrap toggle"
    );
    assert_eq!(
        harness.state().workspace_settings().editor_prefs.word_wrap,
        WordWrapMode::On,
        "editor_prefs still On after the sync (no revert)"
    );

    // TWO-WAY: an explicit Settings change still flows prefs->panel (word wrap OFF via the Settings control).
    let mut off_prefs = harness.state().workspace_settings().editor_prefs;
    off_prefs.word_wrap = WordWrapMode::Off;
    harness
        .state_mut()
        .apply_settings_outcome_for_test(SettingsOutcome::EditorPrefsChanged(off_prefs));
    harness.run();
    assert!(
        !harness.state().mounted_code_panel().is_wrap_enabled(),
        "MT-072 Fix 3: an explicit Settings word_wrap=Off still flows prefs->panel (two-way sync intact)"
    );
}
