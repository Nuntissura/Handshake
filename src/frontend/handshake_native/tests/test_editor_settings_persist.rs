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

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;
use handshake_native::accessibility::{UiNodeBounds, UiTreeNode, UiTreeSnapshot};
use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend_client::HealthInfo;
use handshake_native::code_editor::HighlightScope;
use handshake_native::settings_dialog::SettingsOutcome;
use handshake_native::settings_editor_section::{
    EDITOR_FONT_SIZE_AUTHOR_ID, EDITOR_TAB_SIZE_AUTHOR_ID,
    FLIGHT_RECORDER_SETTINGS_POSTURE_AUTHOR_ID, FLIGHT_RECORDER_SETTINGS_POSTURE_NOTE,
};
use handshake_native::theme::{HsTheme, MUTED_PALETTE, STANDARD_PALETTE};
use handshake_native::workspace_settings::{
    default_workspace_settings_state, EditorPrefs, RenderWhitespaceMode, SettingsTransport,
    SettingsTransportError, SyntaxPalette, SyntaxPaletteMode, WordWrapMode,
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

// MT-072 (FAIL_V2) — retry-after-failure on the canonical PreferenceRecord authority. This SUPERSEDES the
// retired `editor_settings_persist_managed_postgres_all_fields_retry_and_reopen_round_trip`, which drove
// the editor widgets but asserted persistence through the opaque `/settings` PUT/GET blob — dead routing
// after editor settings migrated to per-id preference PUTs (the `/settings` save is never called by an
// editor edit, so that test could not pass against a live backend). The behaviors it uniquely proved are
// now covered against the correct authority:
//   * live set/reset/history/receipt/EventLedger round-trip → test_editor_preference_records.rs (live PG)
//   * Argus widget → canonical PUT + AccessKit ids → argus_set_value_on_mounted_font_size_reaches_...
//   * close/reopen hydrate from canonical state → opening_settings_hydrates_editor_prefs_from_the_projection
//   * transient failure surfaces visibly + edit retained → backend_unavailable_preference_write_degrades_...
//   * structured 400 surfaces → structured_validation_rejection_surfaces_on_status_row
//   * chrome/editor font separation → workspace_settings::tests::editor_font_size_is_separate_from_chrome_appearance
// This test adds the missing piece: an explicit, addressable Retry that RE-DISPATCHES the exact retained
// editor edit after a transient backend failure, so the operator's change is recoverable (no data loss).
#[test]
fn failed_editor_preference_write_retries_and_redispatches_the_exact_edit() {
    let stub = StubPreferenceTransport::failing(PreferenceTransportError::Unavailable(
        "connection refused".to_owned(),
    ));
    let (app, stub) = pref_wired_app(stub);
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    // The dialog stays CLOSED (as in backend_unavailable_preference_write_degrades_visibly_without_freeze):
    // the shell drives + drains the preference write queue while closed, and a failed write surfaces the
    // addressable Retry via the closed-dialog persistence overlay. Not opening the dialog also keeps this
    // test off the separate /settings load path (whose fresh-workspace None response is unrelated here).

    // Edit the editor font size; the first PUT hits the unavailable backend.
    let mut prefs = harness.state().workspace_settings().editor_prefs;
    prefs.editor_font_size = 20.0;
    harness
        .state_mut()
        .apply_settings_outcome_for_test(SettingsOutcome::EditorPrefsChanged(prefs));

    // The transient failure surfaces a visible, addressable Retry. Its label ("Retry saving preference")
    // is emitted ONLY for the typed Preference retry lane, so its presence proves the lane is armed.
    assert!(
        run_until(&mut harness, 120, |app| app
            .settings_persist_error()
            .is_some()),
        "a transient preference-write failure surfaces a visible persist error"
    );
    harness.run_steps(3);
    assert!(
        harness.query_by_label("Retry saving preference").is_some(),
        "the failed preference write arms the typed 'Retry saving preference' control"
    );
    assert!(
        harness
            .root()
            .children_recursive()
            .any(|node| node.accesskit_node().author_id()
                == Some(handshake_native::settings_dialog::SETTINGS_PERSIST_RETRY_AUTHOR_ID)),
        "the Retry control is addressable by its stable author_id"
    );
    // The optimistic edit is retained (no data loss) and nothing reached the record store yet.
    assert_eq!(
        harness
            .state()
            .workspace_settings()
            .editor_prefs
            .editor_font_size,
        20.0
    );
    assert!(
        stub.sets().is_empty(),
        "the failed write never committed to the backend record store"
    );

    // Backend recovers; Retry re-dispatches the EXACT retained edit to the canonical font-size route.
    stub.set_fail_write(None);
    harness
        .state_mut()
        .apply_settings_outcome_for_test(SettingsOutcome::RetryPersistence);
    assert!(
        run_until(&mut harness, 120, |app| {
            app.settings_persist_error().is_none()
                && stub
                    .sets()
                    .iter()
                    .any(|(id, v)| id == PREF_EDITOR_FONT_SIZE && v == &serde_json::json!(20.0))
        }),
        "Retry re-dispatched the exact retained font-size edit and cleared the error; sets={:?}",
        stub.sets()
    );
    // The Retry control clears once the write commits (the lane is disarmed).
    harness.run_steps(3);
    assert!(
        harness.query_by_label("Retry saving preference").is_none(),
        "the Retry control clears after a successful re-dispatch"
    );
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

    // MT-072 FAIL_V2: editor prefs migrated OFF the opaque /settings PUT, so this generic opaque-doc
    // retry-mechanism proof is now driven by a NON-editor setting (theme) that still rides /settings.
    let next_theme = if harness.state().workspace_settings().theme
        == handshake_native::workspace_settings::WorkspaceTheme::Dark
    {
        handshake_native::workspace_settings::WorkspaceTheme::Light
    } else {
        handshake_native::workspace_settings::WorkspaceTheme::Dark
    };
    harness
        .state_mut()
        .apply_settings_outcome_for_test(SettingsOutcome::ThemeChanged(next_theme));
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
            app.workspace_settings().theme == next_theme
        }),
        "reopen and its remote/default GET preserve the exact unsaved local (theme) value"
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

// ── AC-001 / AC-002 / AC-009 (MT-072 FAIL_V2 authority): editor prefs persist via the canonical
//    PreferenceRecord PUT (view-defaults.editor.*), NOT the opaque /settings document; distinct from
//    chrome. Retargeted from the superseded opaque-doc assertion validator V2 rejected. ─────────────
#[test]
fn editor_prefs_change_persists_via_existing_put_and_reloads() {
    let (app, stub) = pref_wired_app(StubPreferenceTransport::new());
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.state_mut().open_settings();
    // Pump a few frames so the Editor section renders + the open-hydrate flushes.
    run_until(&mut harness, 80, |_| stub.list_calls() >= 1);

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
    // the AC requires — the section returns EditorPrefsChanged, the shell stores it + PUTs each changed
    // preference id).
    let new_prefs = EditorPrefs {
        editor_font_size: 22.0,
        tab_size: 8,
        insert_spaces: false,
        word_wrap: WordWrapMode::BoundedColumn(100),
        render_whitespace: RenderWhitespaceMode::All,
        ..EditorPrefs::default()
    };
    harness
        .state_mut()
        .apply_settings_outcome_for_test(SettingsOutcome::EditorPrefsChanged(new_prefs));
    // At least the 5 core scalar changes + the bounded-wrap column = 6 targeted PUTs.
    run_until(&mut harness, 120, |_| stub.sets().len() >= 6);

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

    // AC-001 / AC-009: the change persists via the canonical PreferenceRecord PUTs (the ONLY editor save
    // surface — no opaque /settings write, no SQLite, no new endpoint).
    let sets: std::collections::HashMap<String, Value> = stub.sets().into_iter().collect();
    assert_eq!(
        sets.get(PREF_EDITOR_FONT_SIZE).and_then(Value::as_f64),
        Some(22.0),
        "AC-001: font size PUT carries the typed value on its stable id"
    );
    assert_eq!(
        sets.get(PREF_EDITOR_TAB_SIZE).and_then(Value::as_u64),
        Some(8)
    );
    assert_eq!(
        sets.get("view-defaults.editor.insert-spaces")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        sets.get("view-defaults.editor.render-whitespace")
            .and_then(Value::as_str),
        Some("all")
    );
    assert_eq!(
        sets.get("view-defaults.editor.word-wrap")
            .and_then(Value::as_str),
        Some("bounded")
    );
    assert_eq!(
        sets.get("view-defaults.editor.word-wrap-column")
            .and_then(Value::as_u64),
        Some(100),
        "AC-001: the bounded word-wrap column is its own canonical preference"
    );
    // AC-002: font-size is a distinct preference id, never a chrome key.
    assert!(
        PREF_EDITOR_FONT_SIZE.starts_with("view-defaults.editor."),
        "AC-002: editor font size is its own editor-namespaced preference id"
    );

    // AC-001 (reload side): a NEW app hydrating the projection on open reloads identical editor prefs.
    let reload_rows = vec![
        PreferenceProjectionRow {
            preference_id: PREF_EDITOR_FONT_SIZE.to_owned(),
            value: serde_json::json!(22.0),
            default_value: serde_json::json!(13.0),
            source: "operator".to_owned(),
            revision: 1,
        },
        PreferenceProjectionRow {
            preference_id: PREF_EDITOR_TAB_SIZE.to_owned(),
            value: serde_json::json!(8),
            default_value: serde_json::json!(4),
            source: "operator".to_owned(),
            revision: 1,
        },
    ];
    let (mut app2, reload_stub) =
        pref_wired_app(StubPreferenceTransport::with_projection(reload_rows));
    app2.open_settings();
    let mut harness2 =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app2);
    let loaded = run_until(&mut harness2, 120, |app| {
        reload_stub.list_calls() >= 1
            && app.workspace_settings().editor_prefs.editor_font_size == 22.0
            && app.workspace_settings().editor_prefs.tab_size == 8
    });
    assert!(
        loaded,
        "AC-001: reopening (GET projection) reloads the SAME editor prefs (got {:?})",
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

// ── AC-005 (persistence side, MT-072 FAIL_V2 authority): the editor keybinding override persists as the
//    canonical `view-defaults.editor.keybinding-overrides` json-object preference (action_id -> chord),
//    a dedicated editor namespace that never touches the WP-011 app keybindings map. Retargeted from the
//    superseded opaque-doc `editor_keybindings` list assertion. ─────────────────────────────────────
#[test]
fn editor_keybinding_override_persists_outside_the_app_keybindings_map() {
    let (app, stub) = pref_wired_app(StubPreferenceTransport::new());
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.state_mut().open_settings();
    run_until(&mut harness, 80, |_| stub.list_calls() >= 1);

    harness
        .state_mut()
        .apply_settings_outcome_for_test(SettingsOutcome::EditorKeybindingChanged {
            action_id: "code.open_find".to_owned(),
            chord: "Mod+Alt+F".to_owned(),
        });
    run_until(&mut harness, 80, |_| {
        stub.sets()
            .iter()
            .any(|(id, _)| id == PREF_EDITOR_KEYBINDING_OVERRIDES)
    });

    // The override persists as the dedicated editor keybinding-overrides preference (its own namespaced
    // id), carrying action_id -> chord. It never rides the WP-011 app keybindings map (a separate
    // surface the backend deny-unknown-validates).
    let sets: std::collections::HashMap<String, Value> = stub.sets().into_iter().collect();
    let overrides = sets
        .get(PREF_EDITOR_KEYBINDING_OVERRIDES)
        .expect("the editor keybinding-overrides preference was PUT");
    assert_eq!(
        overrides.get("code.open_find").and_then(Value::as_str),
        Some("Mod+Alt+F"),
        "the override map carries the edited action id -> chord"
    );
    assert!(
        overrides.get("app.quick_switcher.open").is_none()
            && overrides.get("app.command_palette.open").is_none(),
        "RISK-001: editor overrides are a distinct editor-namespaced preference, not the app keybindings map"
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

// ===========================================================================
// MT-072 FAIL_V2 remediation: editor settings are now authoritative on the
// canonical PreferenceRecord surface (Master Spec §10.17), NOT the opaque
// /settings document. These proofs drive the REAL HandshakeApp headlessly and
// capture at the frontend preference-client boundary (a stub PreferenceTransport)
// that:
//  * a control edit issues a targeted PUT to the stable view-defaults.editor.* id;
//  * reset-to-default issues POST .../reset (SET-UI-002);
//  * a backend-unavailable / structured-400 write degrades VISIBLY (no freeze);
//  * hydrate-on-open reads resolved values from the projection (SET-REC-003);
//  * a canonical Argus set_value on a real mounted control reaches the PUT boundary.
//
// The LIVE managed-PostgreSQL round-trip for this surface is the separate proof
// test_editor_preference_records.rs (require_live_backend). Here the stub is the
// sole I/O surface so the UI wiring is provable with no live server.
// ===========================================================================

use handshake_native::preference_client::{
    PreferenceProjectionRow, PreferenceRecord, PreferenceTransport, PreferenceTransportError,
    PreferenceValidationError, PREF_EDITOR_FONT_SIZE, PREF_EDITOR_KEYBINDING_OVERRIDES,
    PREF_EDITOR_SYNTAX_PALETTE_MODE, PREF_EDITOR_TAB_SIZE,
};

/// A scriptable in-memory preference transport: records every set/reset/list, and can be scripted to
/// return a structured validation rejection or an unavailable error so the degradation paths are
/// provable with no live server.
#[derive(Default)]
struct StubPreferenceTransport {
    inner: Mutex<StubPrefInner>,
}

#[derive(Default)]
struct StubPrefInner {
    /// Every (preference_id, value) PUT captured, in order.
    sets: Vec<(String, Value)>,
    /// Every preference_id reset, in order.
    resets: Vec<String>,
    /// Number of list (hydrate) calls.
    list_calls: usize,
    /// Scripted projection rows returned by `list`.
    projection: Vec<PreferenceProjectionRow>,
    /// When set, `set`/`reset` fail with this error (degradation scripting).
    fail_write_with: Option<PreferenceTransportError>,
}

impl StubPreferenceTransport {
    fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }
    fn with_projection(rows: Vec<PreferenceProjectionRow>) -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(StubPrefInner {
                projection: rows,
                ..Default::default()
            }),
        })
    }
    fn failing(err: PreferenceTransportError) -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(StubPrefInner {
                fail_write_with: Some(err),
                ..Default::default()
            }),
        })
    }
    fn sets(&self) -> Vec<(String, Value)> {
        self.inner.lock().unwrap().sets.clone()
    }
    fn resets(&self) -> Vec<String> {
        self.inner.lock().unwrap().resets.clone()
    }
    fn list_calls(&self) -> usize {
        self.inner.lock().unwrap().list_calls
    }
    /// Script the write-failure state at runtime so a test can fail the FIRST write (transient backend
    /// outage) then clear it to prove Retry re-dispatches the exact retained write and succeeds.
    fn set_fail_write(&self, err: Option<PreferenceTransportError>) {
        self.inner.lock().unwrap().fail_write_with = err;
    }
}

impl PreferenceTransport for StubPreferenceTransport {
    fn list(
        &self,
        _workspace_id: &str,
    ) -> Result<Vec<PreferenceProjectionRow>, PreferenceTransportError> {
        let mut s = self.inner.lock().unwrap();
        s.list_calls += 1;
        Ok(s.projection.clone())
    }
    fn set(
        &self,
        _workspace_id: &str,
        preference_id: &str,
        value: Value,
    ) -> Result<PreferenceRecord, PreferenceTransportError> {
        let mut s = self.inner.lock().unwrap();
        if let Some(err) = s.fail_write_with.clone() {
            return Err(err);
        }
        s.sets.push((preference_id.to_owned(), value.clone()));
        Ok(PreferenceRecord {
            preference_id: preference_id.to_owned(),
            value,
            default_value: Value::Null,
            source: "operator".to_owned(),
            revision: 1,
        })
    }
    fn reset(
        &self,
        _workspace_id: &str,
        preference_id: &str,
    ) -> Result<PreferenceRecord, PreferenceTransportError> {
        let mut s = self.inner.lock().unwrap();
        if let Some(err) = s.fail_write_with.clone() {
            return Err(err);
        }
        s.resets.push(preference_id.to_owned());
        Ok(PreferenceRecord {
            preference_id: preference_id.to_owned(),
            value: Value::Null,
            default_value: Value::Null,
            source: "operator".to_owned(),
            revision: 1,
        })
    }
    fn history(
        &self,
        _workspace_id: &str,
        _preference_id: &str,
    ) -> Result<
        Vec<handshake_native::preference_client::PreferenceChangeReceipt>,
        PreferenceTransportError,
    > {
        Ok(Vec::new())
    }
}

/// Build a harness bound to a workspace + runtime + the stub preference transport, ready to apply
/// editor outcomes and pump the off-thread preference write queue.
fn pref_wired_app(
    stub: Arc<StubPreferenceTransport>,
) -> (HandshakeApp, Arc<StubPreferenceTransport>) {
    let mut app = ok_app();
    app.set_runtime_handle(shared_runtime_handle());
    app.bind_active_project_for_integration_test("workspace-pref");
    app.set_preference_transport(stub.clone());
    // A settings transport is still needed for non-editor settings; a no-op stub suffices here.
    app.set_settings_transport(StubSettingsTransport::with_loaded(None));
    (app, stub)
}

#[test]
fn editor_pref_edit_puts_to_canonical_preference_route() {
    let (app, stub) = pref_wired_app(StubPreferenceTransport::new());
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    // Edit only tab-size via the same outcome path the mounted DragValue produces.
    let mut prefs = harness.state().workspace_settings().editor_prefs;
    prefs.tab_size = 8;
    harness
        .state_mut()
        .apply_settings_outcome_for_test(SettingsOutcome::EditorPrefsChanged(prefs));
    // Pump frames so the off-thread PUT flushes + delivers.
    run_until(&mut harness, 120, |_| !stub.sets().is_empty());
    let sets = stub.sets();
    assert_eq!(
        sets.len(),
        1,
        "exactly one targeted PUT for the single edited field, got {sets:?}"
    );
    assert_eq!(
        sets[0].0, PREF_EDITOR_TAB_SIZE,
        "PUT targets the stable tab-size id"
    );
    assert_eq!(
        sets[0].1,
        serde_json::json!(8),
        "PUT carries the typed value"
    );
}

#[test]
fn editor_keybinding_edit_puts_overrides_map_to_preference_route() {
    let (app, stub) = pref_wired_app(StubPreferenceTransport::new());
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness
        .state_mut()
        .apply_settings_outcome_for_test(SettingsOutcome::EditorKeybindingChanged {
            action_id: "code.open_find".to_owned(),
            chord: "Mod+Alt+F".to_owned(),
        });
    run_until(&mut harness, 120, |_| !stub.sets().is_empty());
    let sets = stub.sets();
    assert_eq!(sets.len(), 1);
    assert_eq!(
        sets[0].0, PREF_EDITOR_KEYBINDING_OVERRIDES,
        "editor keybinding overrides persist as the canonical json-object preference"
    );
    assert!(
        sets[0].1.get("code.open_find").is_some(),
        "the override map carries the edited action id, got {:?}",
        sets[0].1
    );
}

#[test]
fn editor_prefs_reset_posts_reset_route_for_every_scalar() {
    let (app, stub) = pref_wired_app(StubPreferenceTransport::new());
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness
        .state_mut()
        .apply_settings_outcome_for_test(SettingsOutcome::EditorPrefsReset);
    run_until(&mut harness, 200, |_| stub.resets().len() >= 13);
    let resets = stub.resets();
    assert!(
        resets.contains(&PREF_EDITOR_FONT_SIZE.to_owned()),
        "reset-to-default POSTs .../reset for the font-size id (SET-UI-002); got {resets:?}"
    );
    assert!(
        resets.contains(&PREF_EDITOR_TAB_SIZE.to_owned()),
        "reset-to-default covers every editor scalar preference"
    );
}

#[test]
fn syntax_palette_reset_posts_reset_route() {
    let (app, stub) = pref_wired_app(StubPreferenceTransport::new());
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness
        .state_mut()
        .apply_settings_outcome_for_test(SettingsOutcome::SyntaxPaletteReset);
    run_until(&mut harness, 120, |_| {
        stub.resets()
            .contains(&PREF_EDITOR_SYNTAX_PALETTE_MODE.to_owned())
    });
    assert!(stub
        .resets()
        .contains(&PREF_EDITOR_SYNTAX_PALETTE_MODE.to_owned()));
}

#[test]
fn backend_unavailable_preference_write_degrades_visibly_without_freeze() {
    let stub = StubPreferenceTransport::failing(PreferenceTransportError::Unavailable(
        "connection refused".to_owned(),
    ));
    let (app, _stub) = pref_wired_app(stub);
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    let mut prefs = harness.state().workspace_settings().editor_prefs;
    prefs.tab_size = 6;
    harness
        .state_mut()
        .apply_settings_outcome_for_test(SettingsOutcome::EditorPrefsChanged(prefs));
    let degraded = run_until(&mut harness, 120, |app| {
        app.settings_persist_error().is_some()
    });
    assert!(
        degraded,
        "an unreachable preference backend surfaces a visible persist error (no freeze)"
    );
    // The optimistic local edit is retained so the UI does not lose the operator's change.
    assert_eq!(
        harness.state().workspace_settings().editor_prefs.tab_size,
        6
    );
}

#[test]
fn structured_validation_rejection_surfaces_on_status_row() {
    let stub = StubPreferenceTransport::failing(PreferenceTransportError::Validation(
        PreferenceValidationError {
            preference_id: PREF_EDITOR_FONT_SIZE.to_owned(),
            code: "out_of_range".to_owned(),
            message: "number 100 is outside the allowed range [6, 48]".to_owned(),
        },
    ));
    let (app, _stub) = pref_wired_app(stub);
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    let mut prefs = harness.state().workspace_settings().editor_prefs;
    prefs.editor_font_size = 40.0;
    harness
        .state_mut()
        .apply_settings_outcome_for_test(SettingsOutcome::EditorPrefsChanged(prefs));
    run_until(&mut harness, 120, |app| {
        app.settings_persist_error().is_some()
    });
    let err = harness
        .state()
        .settings_persist_error()
        .unwrap_or("")
        .to_owned();
    assert!(
        err.contains("out of the allowed range") || err.contains("outside the allowed range"),
        "the structured 400 validation message is surfaced verbatim: {err}"
    );
    assert!(
        err.contains(PREF_EDITOR_FONT_SIZE),
        "names the rejected preference id"
    );
}

#[test]
fn opening_settings_hydrates_editor_prefs_from_the_projection() {
    let rows = vec![
        PreferenceProjectionRow {
            preference_id: PREF_EDITOR_FONT_SIZE.to_owned(),
            value: serde_json::json!(24.0),
            default_value: serde_json::json!(13.0),
            source: "operator".to_owned(),
            revision: 3,
        },
        PreferenceProjectionRow {
            preference_id: PREF_EDITOR_TAB_SIZE.to_owned(),
            value: serde_json::json!(2),
            default_value: serde_json::json!(4),
            source: "operator".to_owned(),
            revision: 1,
        },
    ];
    let (mut app, stub) = pref_wired_app(StubPreferenceTransport::with_projection(rows));
    app.open_settings();
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    let hydrated = run_until(&mut harness, 200, |app| {
        app.workspace_settings().editor_prefs.editor_font_size == 24.0
    });
    assert!(
        hydrated && stub.list_calls() >= 1,
        "the dialog reads resolved editor values from the PreferenceRecord projection on open"
    );
    assert_eq!(
        harness.state().workspace_settings().editor_prefs.tab_size,
        2
    );
}

#[test]
fn argus_set_value_on_mounted_font_size_reaches_preference_put_and_tree_has_ids() {
    // Canonical Argus (list_widgets/set_value/re-observe) on a REAL mounted control, proving the
    // migrated write path end-to-end at the client boundary. AccessKit-tree evidence acceptable on a
    // headless host (there is no display to screenshot in CI).
    let stub = StubPreferenceTransport::new();
    let (mut app, stub) = pref_wired_app(stub);
    app.open_settings();
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(3);
    // AccessKit-tree evidence: the migrated controls are addressable by their stable author_ids.
    let tree = snapshot_harness(&mut harness);
    for required in [EDITOR_FONT_SIZE_AUTHOR_ID, EDITOR_TAB_SIZE_AUTHOR_ID] {
        assert!(
            tree.find_by_author_id(required).is_some(),
            "AccessKit tree exposes the migrated control '{required}' for Argus steering"
        );
    }
    // Steer the real mounted font-size DragValue; the edit flows through EditorPrefsChanged -> the
    // canonical PUT boundary.
    drive_argus_control(
        &mut harness,
        handshake_native::mcp::ARGUS_SET_VALUE_METHOD,
        EDITOR_FONT_SIZE_AUTHOR_ID,
        Some("18"),
    );
    let put = run_until(&mut harness, 200, |_| {
        stub.sets()
            .iter()
            .any(|(id, _)| id == PREF_EDITOR_FONT_SIZE)
    });
    assert!(
        put,
        "an Argus-steered font-size edit issues a PUT to the canonical font-size preference id; captured PUTs = {:?}",
        stub.sets()
    );
}
