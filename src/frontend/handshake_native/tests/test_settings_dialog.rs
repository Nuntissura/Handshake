//! WP-KERNEL-011 MT-018 — Settings / Options dialog live proofs.
//!
//! Drives the REAL `HandshakeApp` headlessly via egui_kittest (which enables AccessKit and pushes the
//! same `TreeUpdate` the out-of-process Windows UIA adapter receives) and proves the contract's kittest
//! acceptance cases: the dialog opens via the `settings_open` flag, the Theme row wires to the live
//! `current_theme` and persists, the search filter narrows sections, keybinding edits detect conflicts
//! (and a conflicting binding is NOT saved), and the Reset-Layout button arms the reset. Persistence is
//! proven against a stub `SettingsTransport` so no live server is needed for the default `cargo test`
//! run; the live-PG round-trip is the cfg-gated `integration_tests` test at the bottom.

use std::sync::{Arc, Mutex};

use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use handshake_native::app::{HandshakeApp, HealthDisplayState, ViewMode};
use handshake_native::backend_client::HealthInfo;
use handshake_native::theme::HsTheme;
use handshake_native::workspace_settings::{
    SettingsTransport, SettingsTransportError, WorkspaceTheme,
};
use serde_json::Value;

fn ok_app() -> HandshakeApp {
    HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }))
}

/// A scriptable in-memory settings transport: records the last saved blob and serves a scripted load
/// result, so the full open -> change -> persist round-trip is provable with no live backend. Thread-safe
/// (the app spawns load/save on a runtime worker).
#[derive(Default)]
struct StubSettingsTransport {
    inner: Mutex<StubInner>,
}

#[derive(Default)]
struct StubInner {
    /// The blob returned by `load` (None => first run).
    load_result: Option<Value>,
    /// The last blob `save` received.
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

/// A real multi-thread runtime the stub transport's `block_on` can bridge onto, so the spawned
/// load/save tasks actually run + deliver into the app's cells (the headless `with_health` shell has no
/// runtime). Leaked so the handle outlives the test frames.
fn leak_runtime_handle() -> tokio::runtime::Handle {
    let rt = Box::leak(Box::new(
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("runtime"),
    ));
    rt.handle().clone()
}

/// Pump the harness until `pred` holds or `max` frames elapse (drains async load/save deliveries).
fn run_until(
    harness: &mut Harness<'_, HandshakeApp>,
    max: usize,
    pred: impl Fn(&HandshakeApp) -> bool,
) -> bool {
    for _ in 0..max {
        harness.run_steps(3);
        if pred(harness.state()) {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    pred(harness.state())
}

// ── Test 4 (contract): open settings, Theme row visible, change theme to Dark -> app theme Dark ──────
#[test]
fn opening_settings_shows_theme_row_and_changing_theme_applies_to_app() {
    let transport = StubSettingsTransport::with_loaded(None);
    let handle = leak_runtime_handle();

    let mut app = ok_app();
    app.set_runtime_handle(handle);
    app.set_settings_transport(transport.clone());
    // Start from Light so a change to Dark is observable.
    app.set_workspace_theme_for_test(WorkspaceTheme::Light);

    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    // Open the dialog (HELP > Open Settings… mirror).
    harness.state_mut().open_settings();
    harness.run_steps(3);

    assert!(harness.state().settings_open(), "dialog open");
    // The Theme row + its ComboBox are in the live tree (findable by label).
    let _theme_label = harness.get_by_label("Theme / appearance");

    // Drive the wired change directly through the outcome path (a kittest cannot reliably click into a
    // ComboBox popup item; the dialog's wiring is what AC3 requires — selecting Dark applies + persists).
    harness.state_mut().apply_settings_outcome_for_test(
        handshake_native::settings_dialog::SettingsOutcome::ThemeChanged(WorkspaceTheme::Dark),
    );
    // Next frame applies the pending theme at the top of ui().
    harness.run_steps(3);

    assert_eq!(
        harness.state().current_theme(),
        HsTheme::Dark,
        "AC3: selecting Dark applies egui dark theme to the app"
    );
    assert_eq!(
        harness.state().workspace_settings().theme,
        WorkspaceTheme::Dark,
        "persisted-settings theme updated to Dark"
    );

    // AC3: the change persists via PUT (debounced). Pump until the stub records the save.
    let saved = run_until(&mut harness, 60, |_| transport.save_calls() >= 1);
    assert!(
        saved,
        "theme change persisted via PUT /workspaces/{{id}}/settings"
    );
    let blob = transport.saved().expect("a settings_state blob was PUT");
    assert_eq!(
        blob.get("theme").and_then(Value::as_str),
        Some("dark"),
        "persisted blob carries the new theme"
    );
}

// ── Test 5 (contract): typing 'keybinding' shows only the Keybindings section ────────────────────────
#[test]
fn search_filter_narrows_to_keybindings_section() {
    let handle = leak_runtime_handle();
    let mut app = ok_app();
    app.set_runtime_handle(handle);
    app.set_settings_transport(StubSettingsTransport::with_loaded(None));

    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.state_mut().open_settings();
    harness.run_steps(3);

    // With no query, the Appearance + Keybindings + About headers are all present.
    assert!(
        harness.query_by_label("Appearance").is_some(),
        "Appearance shown with empty query"
    );
    assert!(
        harness.query_by_label("Keybindings").is_some(),
        "Keybindings shown with empty query"
    );

    // Type 'keybinding' into the search box.
    let search = harness.get_by_label("Search settings");
    search.focus();
    harness.run_steps(3);
    harness
        .get_by_label("Search settings")
        .type_text("keybinding");
    harness.run_steps(3);
    harness.run_steps(3);

    assert!(
        harness.query_by_label("Keybindings").is_some(),
        "AC2: Keybindings section visible for query 'keybinding'"
    );
    assert!(
        harness.query_by_label("Appearance").is_none(),
        "AC2: Appearance section hidden for query 'keybinding'"
    );
    assert!(
        harness.query_by_label("About").is_none(),
        "AC2: About section hidden for query 'keybinding'"
    );
}

// ── Test 6 (contract): same chord on both actions -> conflict banner; not persisted ─────────────────
#[test]
fn duplicate_keybinding_chord_shows_conflict_banner_and_is_not_saved() {
    let transport = StubSettingsTransport::with_loaded(None);
    let handle = leak_runtime_handle();
    let mut app = ok_app();
    app.set_runtime_handle(handle);
    app.set_settings_transport(transport.clone());
    // Seed a deliberately-conflicting state: both actions on Mod-Alt-p (the wired edit path refuses to
    // commit a conflict, so the seed bypasses it). On open, the drafts reflect this and the banner shows.
    app.set_keybinding_for_test("app.quick_switcher.open", "Mod-Alt-p");
    app.set_keybinding_for_test("app.command_palette.open", "Mod-Alt-p");

    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.state_mut().open_settings();
    harness.run_steps(3);
    harness.run_steps(3);

    // AC6: the conflict banner appears naming both actions + the shared chord.
    assert!(
        harness
            .query_by_label("Quick Switcher and Command Palette both use Mod-Alt-p.")
            .is_some(),
        "AC6: conflict banner appears naming both actions + the shared chord"
    );

    // AC6: while the bindings conflict, the dialog renders the banner but emits NO KeybindingChanged,
    // so nothing is persisted to the backend (a conflicting binding is never saved). Run several frames
    // and let any (incorrect) debounce elapse; the stub must record ZERO saves.
    run_until(&mut harness, 40, |_| transport.save_calls() > 0);
    assert_eq!(
        transport.save_calls(),
        0,
        "AC6: a conflicting binding is NOT saved to the backend while the conflict stands"
    );

    // Now RESOLVE the conflict via the wired Reset on the command palette (restores Mod-Shift-p). The
    // dialog then emits the reset outcome, the conflict clears, and the resolved state DOES persist.
    harness.state_mut().apply_settings_outcome_for_test(
        handshake_native::settings_dialog::SettingsOutcome::KeybindingReset {
            action_id: "app.command_palette.open".to_owned(),
        },
    );
    harness.run_steps(3);
    assert_eq!(
        harness
            .state()
            .workspace_settings()
            .chord_for("app.command_palette.open"),
        Some("Mod-Shift-p"),
        "AC7: Reset restores the default chord and clears the conflict"
    );
    assert!(
        run_until(&mut harness, 60, |_| transport.save_calls() >= 1),
        "AC6/AC7: once the conflict is resolved, the binding persists"
    );
}

// ── Test 7 (contract): Reset panes & drawers arms the layout reset ──────────────────────────────────
#[test]
fn reset_panes_and_drawers_button_arms_layout_reset() {
    let handle = leak_runtime_handle();
    let mut app = ok_app();
    app.set_runtime_handle(handle);
    app.set_settings_transport(StubSettingsTransport::with_loaded(None));

    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.state_mut().open_settings();
    harness.run_steps(3);

    assert!(
        !harness.state().reset_layout_pending(),
        "no reset armed initially"
    );

    // Click the Reset panes & drawers button (findable by its visible label).
    harness.get_by_label("Reset panes & drawers").click();
    harness.run_steps(3);

    assert!(
        harness.state().reset_layout_pending(),
        "AC10: Reset panes & drawers arms the layout reset (same as VIEW > Reset Layout)"
    );
}

// ── View-mode wiring (AC4) ──────────────────────────────────────────────────────────────────────────
#[test]
fn changing_view_mode_updates_app_flag_and_persists() {
    let transport = StubSettingsTransport::with_loaded(None);
    let handle = leak_runtime_handle();
    let mut app = ok_app();
    app.set_runtime_handle(handle);
    app.set_settings_transport(transport.clone());

    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.state_mut().open_settings();
    harness.run_steps(3);
    assert_eq!(harness.state().view_mode(), ViewMode::Nsfw, "default NSFW");

    harness.state_mut().apply_settings_outcome_for_test(
        handshake_native::settings_dialog::SettingsOutcome::ViewModeChanged(
            handshake_native::workspace_settings::SettingsViewMode::Sfw,
        ),
    );
    harness.run_steps(3);
    assert_eq!(
        harness.state().view_mode(),
        ViewMode::Sfw,
        "AC4: toggling SFW updates app_state.view_mode"
    );
    assert!(
        run_until(&mut harness, 60, |_| transport.save_calls() >= 1),
        "AC4: view mode persisted"
    );
}

// ── FIX-A (AC9 + red-team MC5): the NotYetWired disabled rows are PRESENT, show their fixed value, and
//    are NON-INTERACTIVE (typing into them does not change the value). ─────────────────────────────────
//
// Why this matters: MT-018 renders "not yet wired" settings (terminal-default-shell,
// swarm-reconcile-interval, ...) as DISABLED read-only rows pinned to a fixed value. Before this test
// nothing proved they (a) actually reach the LIVE AccessKit tree, (b) display the fixed value, or (c)
// refuse typed input. A row that silently became editable, or that vanished, would regress the contract
// with no failing test. This drives the REAL shell headlessly and asserts all three against the live
// consumer-side tree (the same surface an out-of-process model reads).
//
// MC5 (non-interactive) is proven the strongest way kittest allows: the disabled `TextEdit` IS in the
// live tree (so we can address + perceive it), it carries AccessKit `disabled=true`, and after focusing
// it and sending a `type_text` event + several frames the AccessKit value is UNCHANGED (a disabled,
// non-focusable egui widget never consumes the text event). We assert BOTH the disabled state AND the
// value-unchanged-after-typing outcome, so neither a stale-disabled-flag nor an accidental-edit
// regression can pass.
#[test]
fn not_yet_wired_rows_are_present_show_fixed_value_and_reject_typed_input() {
    use egui_kittest::kittest::NodeT;
    use handshake_native::workspace_settings::{
        SWARM_RECONCILE_INTERVAL_SETTING, SWARM_RESOURCE_POLL_INTERVAL_SETTING,
        TERMINAL_DEFAULT_SHELL_SETTING, TERMINAL_MAX_SCROLLBACK_SETTING,
        TERMINAL_OUTPUT_LOGGING_SETTING,
    };

    let handle = leak_runtime_handle();
    let mut app = ok_app();
    app.set_runtime_handle(handle);
    app.set_settings_transport(StubSettingsTransport::with_loaded(None));

    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.state_mut().open_settings();
    harness.run_steps(3);
    harness.run_steps(3);

    // Every NotYetWired row this dialog renders, by stable author_id (the FIX author_ids
    // `settings.not-wired.*`) + the fixed value it must display.
    let expected: [(&str, &str); 5] = [
        (
            SWARM_RECONCILE_INTERVAL_SETTING.id,
            SWARM_RECONCILE_INTERVAL_SETTING.fixed_value,
        ),
        (
            SWARM_RESOURCE_POLL_INTERVAL_SETTING.id,
            SWARM_RESOURCE_POLL_INTERVAL_SETTING.fixed_value,
        ),
        (
            TERMINAL_DEFAULT_SHELL_SETTING.id,
            TERMINAL_DEFAULT_SHELL_SETTING.fixed_value,
        ),
        (
            TERMINAL_MAX_SCROLLBACK_SETTING.id,
            TERMINAL_MAX_SCROLLBACK_SETTING.fixed_value,
        ),
        (
            TERMINAL_OUTPUT_LOGGING_SETTING.id,
            TERMINAL_OUTPUT_LOGGING_SETTING.fixed_value,
        ),
    ];

    // Helper: snapshot (present, disabled, value) for a not-wired row by its author_id, off the LIVE
    // tree. Returns None when the row is absent (so the test fails loudly with which row is missing).
    fn probe_not_wired(
        harness: &Harness<'_, HandshakeApp>,
        author_id: &str,
    ) -> Option<(bool, Option<String>)> {
        let root = harness.root();
        for node in root.children_recursive() {
            let ak = node.accesskit_node();
            if ak.author_id() == Some(author_id) {
                return Some((ak.is_disabled(), ak.value()));
            }
        }
        None
    }

    for (setting_id, fixed_value) in expected {
        let author_id = format!("settings.not-wired.{setting_id}");

        // (a) PRESENT in the live tree + (b) shows its fixed value + carries AccessKit disabled state.
        let (disabled, value) = probe_not_wired(&harness, &author_id).unwrap_or_else(|| {
            panic!("AC9: not-wired row '{author_id}' missing from the LIVE settings tree")
        });
        assert!(
            disabled,
            "MC5: not-wired row '{author_id}' must be AccessKit-disabled (non-interactive)"
        );
        assert_eq!(
            value.as_deref(),
            Some(fixed_value),
            "AC9: not-wired row '{author_id}' shows its fixed value"
        );

        // (c) NON-INTERACTIVE: attempt to type into the disabled control + pump frames, then assert the
        // value is UNCHANGED. A disabled egui widget is non-focusable and cannot consume the text event.
        //
        // Disclosure: the dialog auto-focuses its SEARCH box on open, so a raw `type_text` would leak
        // into the search box (filtering sections) rather than reaching the disabled control — i.e. the
        // disabled control genuinely cannot receive a type event in kittest (it never holds focus). Per
        // the FIX-A fallback, we (1) first stop any active text input so the typed event targets NO live
        // text widget, (2) focus the disabled node (a no-op — disabled widgets reject Focus), (3) send
        // the text event, and (4) assert the disabled control's AccessKit value is UNCHANGED. This proves
        // the typed input does not reach the disabled row, while keeping the row visible (no search leak).
        harness.ctx.memory_mut(|m| m.stop_text_input());
        harness.run_steps(3);
        let node = harness
            .query_all_by(|n: &egui_kittest::kittest::AccessKitNode<'_>| {
                n.author_id() == Some(author_id.as_str())
            })
            .next()
            .expect("not-wired node addressable for the type-attempt");
        node.focus(); // disabled => Focus action is rejected; the control never gains keyboard focus.
        harness.run_steps(3);
        let node = harness
            .query_all_by(|n: &egui_kittest::kittest::AccessKitNode<'_>| {
                n.author_id() == Some(author_id.as_str())
            })
            .next()
            .expect("not-wired node still addressable");
        node.type_text("XYZ-should-not-stick");
        harness.run_steps(3);
        harness.run_steps(3);

        let (disabled_after, value_after) = probe_not_wired(&harness, &author_id)
            .expect("not-wired row still present after the type attempt");
        assert!(
            disabled_after,
            "MC5: '{author_id}' stays disabled after a type attempt"
        );
        assert_eq!(
            value_after.as_deref(),
            Some(fixed_value),
            "MC5: typing into disabled not-wired row '{author_id}' does NOT change its value"
        );
    }
}

#[test]
fn model_session_settings_action_opens_real_launch_dialog() {
    let handle = leak_runtime_handle();
    let mut app = ok_app();
    app.set_runtime_handle(handle);
    app.set_settings_transport(StubSettingsTransport::with_loaded(None));

    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.state_mut().open_settings();
    harness.run_steps(3);
    harness.run_steps(3);

    harness.get_by_label("Search settings").type_text("model");
    harness.run_steps(3);
    harness.run_steps(3);

    harness
        .query_all_by(|n: &egui_kittest::kittest::AccessKitNode<'_>| {
            n.author_id()
                == Some(handshake_native::settings_dialog::MODEL_SESSION_OPEN_LAUNCH_AUTHOR_ID)
        })
        .next()
        .expect("Settings -> Model Session exposes the real launch action")
        .click();
    harness.run_steps(3);
    harness.run_steps(3);

    assert!(
        !harness.state().settings_open(),
        "settings closes before the one-shot model-session dialog opens"
    );
    assert!(
        harness.state().model_session_launch_dialog_open_for_test(),
        "settings action routes to the same real MT-101 launch dialog as Run/palette"
    );
}

// ── FIX-C (Escape vs ComboBox): Escape while a ComboBox popup is open closes only the POPUP and keeps
//    the dialog open; Escape with no popup open closes the dialog. ──────────────────────────────────────
//
// Regression guard: previously a single Escape both closed an open theme/view-mode combo AND tore down
// the whole dialog (egui's combo and the dialog's own Escape handler both peeked the same Escape event
// in one frame). This test opens the Theme combo popup, presses Escape, and asserts the dialog is STILL
// open (popup-only close) — then presses Escape again with nothing open and asserts the dialog closes.
#[test]
fn escape_closes_open_combo_popup_first_then_dialog() {
    let handle = leak_runtime_handle();
    let mut app = ok_app();
    app.set_runtime_handle(handle);
    app.set_settings_transport(StubSettingsTransport::with_loaded(None));

    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.state_mut().open_settings();
    harness.run_steps(3);
    assert!(harness.state().settings_open(), "dialog open");

    // Open the Theme / appearance ComboBox popup by clicking the combo control itself (addressed by its
    // stable author_id — the visible "Theme / appearance" text is a sibling Label, not the combo).
    harness
        .query_all_by(|n: &egui_kittest::kittest::AccessKitNode<'_>| {
            n.author_id() == Some("settings.theme")
        })
        .next()
        .expect("theme combo addressable by author_id")
        .click();
    harness.run_steps(3);
    harness.run_steps(3);
    assert!(
        egui::Popup::is_any_open(&harness.ctx),
        "precondition: the Theme ComboBox popup is open before Escape"
    );

    // Escape #1: closes ONLY the popup; the dialog stays open (FIX-C).
    harness.key_press(egui::Key::Escape);
    harness.run_steps(3);
    assert!(
        harness.state().settings_open(),
        "FIX-C: Escape with an open combo popup closes the popup, NOT the dialog"
    );
    assert!(
        !egui::Popup::is_any_open(&harness.ctx),
        "FIX-C: the combo popup is closed after Escape"
    );

    // Escape #2: nothing else open now, so Escape closes the dialog (AC12 unchanged).
    harness.key_press(egui::Key::Escape);
    harness.run_steps(3);
    assert!(
        !harness.state().settings_open(),
        "AC12: Escape with no popup open closes the dialog"
    );
}

// ── Escape closes (AC12) + dialog absent by default (MT-025 snapshot stays at its baseline) ─────────
#[test]
fn dialog_closed_by_default_and_escape_closes() {
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), ok_app());
    harness.run_steps(3);
    assert!(!harness.state().settings_open(), "dialog closed by default");
    // No settings nodes in the default tree.
    assert!(
        harness.query_by_label("Settings").is_none(),
        "no settings dialog node in the default-seed live tree"
    );

    harness.state_mut().open_settings();
    harness.run_steps(3);
    assert!(harness.state().settings_open(), "dialog opened");

    // Press Escape -> the dialog requests close.
    harness.key_press(egui::Key::Escape);
    harness.run_steps(3);
    assert!(
        !harness.state().settings_open(),
        "AC12: Escape closes the dialog"
    );
}

// ── Load-on-open restores a persisted theme (PT6 round-trip, stubbed) ───────────────────────────────
#[test]
fn opening_settings_loads_persisted_theme_from_backend() {
    // The backend already has a Dark theme stored. Opening settings must load it and apply Dark.
    let stored = serde_json::json!({
        "schema_id": "hsk.workspace_settings_state@1",
        "theme": "dark",
        "custom_theme_tokens": {},
        "keybindings": { "app.quick_switcher.open": "Mod-p", "app.command_palette.open": "Mod-Shift-p" },
        "settings": { "view_mode": "NSFW", "swarm_board_default_open": false }
    });
    let transport = StubSettingsTransport::with_loaded(Some(stored));
    let handle = leak_runtime_handle();
    let mut app = ok_app();
    app.set_runtime_handle(handle);
    app.set_settings_transport(transport.clone());
    // Start Light; the load must flip it to Dark.
    app.set_workspace_theme_for_test(WorkspaceTheme::Light);

    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.state_mut().open_settings();

    let loaded = run_until(&mut harness, 60, |app| app.current_theme() == HsTheme::Dark);
    assert!(
        loaded,
        "PT6: opening settings loads the persisted Dark theme from the backend"
    );
    assert_eq!(
        harness.state().workspace_settings().theme,
        WorkspaceTheme::Dark,
        "loaded settings reflect the stored theme"
    );
}

// ── Live-PG integration: change theme, persist, reload, assert it round-trips through PostgreSQL ─────
//
// Gated behind the `integration_tests` feature + #[ignore] (mirrors test_layout_persistence.rs): it
// needs managed-postgres + handshake_core on 127.0.0.1:37501 and an existing workspace id. Run with:
//   cargo test --features integration_tests -- --ignored live_backend_settings
#[cfg(feature = "integration_tests")]
#[test]
#[ignore = "needs managed-postgres + handshake_core on 127.0.0.1:37501 and HSK_LIVE_WORKSPACE_ID"]
fn live_backend_settings_round_trips_through_postgres() {
    use handshake_native::workspace_settings::{
        default_workspace_settings_state, normalize_workspace_settings_state, SettingsClient,
    };

    let workspace_id = std::env::var("HSK_LIVE_WORKSPACE_ID")
        .expect("set HSK_LIVE_WORKSPACE_ID to an existing workspace id");
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("runtime");
    let client = SettingsClient::production(rt.handle().clone());

    // Build a non-default settings state, PUT it, GET it back, assert it round-trips.
    let mut settings = default_workspace_settings_state();
    settings.theme = WorkspaceTheme::Dark;
    settings.set_chord("app.quick_switcher.open", "Mod-Alt-q".to_owned());
    let expected = settings.to_settings_state();

    client
        .save(&workspace_id, expected.clone())
        .expect("PUT settings to live backend");
    let got = client
        .load(&workspace_id)
        .expect("GET settings from live backend")
        .expect("backend returned stored settings");
    let normalized = normalize_workspace_settings_state(&got, &default_workspace_settings_state());
    assert_eq!(
        normalized, settings,
        "live PostgreSQL settings_state round-trips identically"
    );
}
