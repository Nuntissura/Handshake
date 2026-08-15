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

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;
use handshake_native::app::{HandshakeApp, HealthDisplayState, ViewMode};
use handshake_native::backend_client::HealthInfo;
use handshake_native::pane_registry::PaneType;
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
        harness.run();
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
    harness.run();

    assert!(harness.state().settings_open(), "dialog open");
    // The Theme row + its ComboBox are in the live tree (findable by label).
    let _theme_label = harness.get_by_label("Theme / appearance");

    // Drive the wired change directly through the outcome path (a kittest cannot reliably click into a
    // ComboBox popup item; the dialog's wiring is what AC3 requires — selecting Dark applies + persists).
    harness.state_mut().apply_settings_outcome_for_test(
        handshake_native::settings_dialog::SettingsOutcome::ThemeChanged(WorkspaceTheme::Dark),
    );
    // Next frame applies the pending theme at the top of ui().
    harness.run();

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
    harness.run();

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
    harness.run();
    harness
        .get_by_label("Search settings")
        .type_text("keybinding");
    harness.run();
    harness.run();

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
    let stored_conflict = serde_json::json!({
        "schema_id": "hsk.workspace_settings_state@1",
        "theme": "dark",
        "custom_theme_tokens": {},
        "keybindings": {
            "app.quick_switcher.open": "Mod-Alt-p",
            "app.command_palette.open": "Mod-Alt-p"
        },
        "settings": {
            "view_mode": "NSFW",
            "swarm_board_default_open": false,
            "swarm_lane_diagnostics_default_open": false,
            "operator_chat_default_open": false
        }
    });
    let transport = StubSettingsTransport::with_loaded(Some(stored_conflict));
    let handle = leak_runtime_handle();
    let mut app = ok_app();
    app.set_runtime_handle(handle);
    app.set_settings_transport(transport.clone());
    // Seed the same deliberately-conflicting state in memory as well as the scripted durable load. The
    // app autoloads settings before the modal opens, so a `None` load would correctly replace any local
    // test-only seed with first-run defaults and make this proof scheduler-dependent.
    app.set_keybinding_for_test("app.quick_switcher.open", "Mod-Alt-p");
    app.set_keybinding_for_test("app.command_palette.open", "Mod-Alt-p");

    // Keep the complete expanded settings surface inside the headless desktop viewport. The dialog
    // now includes Cloud Models and Model Runtime sections, so the kittest default viewport can clip
    // lower AccessKit nodes even though the live desktop surface is scrollable.
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(1440.0, 940.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.state_mut().open_settings();
    harness.run();
    harness.run();

    // AC6: the conflict banner appears naming both actions + the shared chord.
    let conflict_label = harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some("settings.keybinding-conflict"))
        .and_then(|node| node.accesskit_node().label());
    assert_eq!(
        conflict_label.as_deref(),
        Some("Quick Switcher and Command Palette both use Mod-Alt-p."),
        "AC6: addressable conflict banner names both actions + the shared chord"
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
    harness.run();
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

    // Match the production desktop-sized proof surface used by the Model Runtime settings route.
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(1440.0, 940.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.state_mut().open_settings();
    harness.run();

    assert!(
        !harness.state().reset_layout_pending(),
        "no reset armed initially"
    );

    // Click the Reset panes & drawers button (findable by its visible label).
    harness
        .get_by_label("Reset panes & drawers")
        .click_accesskit();
    harness.run();

    assert!(
        harness.state().reset_layout_pending(),
        "AC10: Reset panes & drawers arms the layout reset (same as VIEW > Reset Layout)"
    );
}

#[test]
fn swarm_lane_diagnostics_setting_persists() {
    let transport = StubSettingsTransport::with_loaded(None);
    let handle = leak_runtime_handle();
    let mut app = ok_app();
    app.set_runtime_handle(handle);
    app.set_settings_transport(transport.clone());

    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.state_mut().open_settings();
    harness.run();
    harness.run();

    let nodes: Vec<String> = harness
        .root()
        .children_recursive()
        .filter_map(|node| node.accesskit_node().author_id().map(str::to_owned))
        .collect();
    assert!(
        nodes
            .iter()
            .any(|id| id == "settings.swarm-lane-diagnostics-default-open"),
        "lane diagnostics checkbox is addressable in the live settings tree: {nodes:?}"
    );

    harness.state_mut().apply_settings_outcome_for_test(
        handshake_native::settings_dialog::SettingsOutcome::SwarmLaneDiagnosticsDefaultOpenChanged(
            true,
        ),
    );
    harness.run();
    assert!(
        harness
            .state()
            .workspace_settings()
            .swarm_lane_diagnostics_default_open,
        "settings state flips the diagnostics default flag"
    );
    assert!(
        run_until(&mut harness, 60, |_| transport.save_calls() >= 1),
        "diagnostics setting persisted through PUT /workspaces/{{id}}/settings"
    );
    let blob = transport.saved().expect("a settings_state blob was PUT");
    assert_eq!(
        blob.pointer("/settings/swarm_lane_diagnostics_default_open")
            .and_then(serde_json::Value::as_bool),
        Some(true),
        "persisted blob carries diagnostics default flag"
    );
}

#[test]
fn operator_chat_swarm_setting_persists() {
    let transport = StubSettingsTransport::with_loaded(None);
    let handle = leak_runtime_handle();
    let mut app = ok_app();
    app.set_runtime_handle(handle);
    app.set_settings_transport(transport.clone());

    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.state_mut().open_settings();
    harness.run();
    harness.run();

    let nodes: Vec<String> = harness
        .root()
        .children_recursive()
        .filter_map(|node| node.accesskit_node().author_id().map(str::to_owned))
        .collect();
    assert!(
        nodes
            .iter()
            .any(|id| id == "settings.swarm-operator-chat-default-open"),
        "operator chat checkbox is addressable in the live settings tree: {nodes:?}"
    );

    harness.state_mut().apply_settings_outcome_for_test(
        handshake_native::settings_dialog::SettingsOutcome::OperatorChatDefaultOpenChanged(true),
    );
    harness.run();
    assert!(
        harness
            .state()
            .workspace_settings()
            .operator_chat_default_open,
        "settings state flips the operator chat default flag"
    );
    assert!(
        run_until(&mut harness, 60, |_| transport.save_calls() >= 1),
        "operator chat setting persisted through PUT /workspaces/{{id}}/settings"
    );
    let blob = transport.saved().expect("a settings_state blob was PUT");
    assert_eq!(
        blob.pointer("/settings/operator_chat_default_open")
            .and_then(serde_json::Value::as_bool),
        Some(true),
        "persisted blob carries operator chat default flag"
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
    harness.run();
    assert_eq!(harness.state().view_mode(), ViewMode::Nsfw, "default NSFW");

    harness.state_mut().apply_settings_outcome_for_test(
        handshake_native::settings_dialog::SettingsOutcome::ViewModeChanged(
            handshake_native::workspace_settings::SettingsViewMode::Sfw,
        ),
    );
    harness.run();
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
    harness.run();
    harness.run();

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
        harness.run();
        let node = harness
            .query_all_by(|n: &egui_kittest::kittest::AccessKitNode<'_>| {
                n.author_id() == Some(author_id.as_str())
            })
            .next()
            .expect("not-wired node addressable for the type-attempt");
        node.focus(); // disabled => Focus action is rejected; the control never gains keyboard focus.
        harness.run();
        let node = harness
            .query_all_by(|n: &egui_kittest::kittest::AccessKitNode<'_>| {
                n.author_id() == Some(author_id.as_str())
            })
            .next()
            .expect("not-wired node still addressable");
        node.type_text("XYZ-should-not-stick");
        harness.run();
        harness.run();

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
    harness.run();
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
    harness.run();
    harness.run();
    assert!(
        egui::Popup::is_any_open(&harness.ctx),
        "precondition: the Theme ComboBox popup is open before Escape"
    );

    // Escape #1: closes ONLY the popup; the dialog stays open (FIX-C).
    harness.key_press(egui::Key::Escape);
    harness.run();
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
    harness.run();
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
    harness.run();
    assert!(!harness.state().settings_open(), "dialog closed by default");
    // No settings nodes in the default tree.
    assert!(
        harness.query_by_label("Settings").is_none(),
        "no settings dialog node in the default-seed live tree"
    );

    harness.state_mut().open_settings();
    harness.run();
    assert!(harness.state().settings_open(), "dialog opened");

    // Press Escape -> the dialog requests close.
    harness.key_press(egui::Key::Escape);
    harness.run();
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
        "settings": { "view_mode": "NSFW", "swarm_board_default_open": false, "swarm_lane_diagnostics_default_open": false, "operator_chat_default_open": false }
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

#[test]
fn persisted_swarm_defaults_open_runtime_tabs() {
    let stored = serde_json::json!({
        "schema_id": "hsk.workspace_settings_state@1",
        "theme": "dark",
        "custom_theme_tokens": {},
        "keybindings": { "app.quick_switcher.open": "Mod-p", "app.command_palette.open": "Mod-Shift-p" },
        "settings": { "view_mode": "NSFW", "swarm_board_default_open": true, "swarm_lane_diagnostics_default_open": true, "operator_chat_default_open": true }
    });
    let transport = StubSettingsTransport::with_loaded(Some(stored));
    let handle = leak_runtime_handle();
    let mut app = ok_app();
    app.set_runtime_handle(handle);
    app.set_settings_transport(transport);

    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);

    let loaded = run_until(&mut harness, 60, |app| {
        app.tab_bar_states().values().any(|bar| {
            bar.tabs
                .iter()
                .any(|tab| tab.pane_type == PaneType::SwarmLaneDiagnostics)
        }) && app
            .tab_bar_states()
            .values()
            .any(|bar| bar.tabs.iter().any(|tab| tab.pane_type == PaneType::Swarm))
            && app.tab_bar_states().values().any(|bar| {
                bar.tabs
                    .iter()
                    .any(|tab| tab.pane_type == PaneType::OperatorChatLaunch)
            })
    });
    assert!(
        loaded,
        "stored Swarm defaults open Swarm, Lane Diagnostics, and Operator Chat runtime tabs"
    );
}

#[test]
fn model_runtime_settings_action_opens_production_registry_pane() {
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(1440.0, 940.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), ok_app());
    harness.state_mut().open_settings();
    harness.run();

    let author_ids = harness
        .root()
        .children_recursive()
        .filter_map(|node| node.accesskit_node().author_id().map(str::to_owned))
        .collect::<Vec<_>>();
    assert!(
        author_ids
            .iter()
            .any(|id| id == "settings.section.model-runtime"),
        "Model Runtime settings section must be addressable: {author_ids:?}"
    );
    assert!(
        author_ids
            .iter()
            .any(|id| id == "settings.model-runtime.open"),
        "Model Runtime settings action must be addressable: {author_ids:?}"
    );

    harness.get_by_label("Open Model Runtime").click_accesskit();
    harness.run();

    assert!(
        !harness.state().settings_open(),
        "navigation closes the modal settings overlay"
    );
    assert_eq!(
        harness.state().active_module(),
        handshake_native::module_switcher::ModuleId::Studio,
        "settings navigation uses the production STUDIO route"
    );
    assert!(
        harness.state().tab_bar_states().values().any(|bar| bar
            .tabs
            .iter()
            .any(|tab| tab.pane_type == PaneType::ModelRuntime)),
        "settings navigation opens the production Model Runtime pane"
    );
}

#[test]
fn model_runtime_settings_action_opens_canonical_problems_pane() {
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(1440.0, 940.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), ok_app());
    harness.state_mut().open_settings();
    harness.run();

    let author_ids = harness
        .root()
        .children_recursive()
        .filter_map(|node| node.accesskit_node().author_id().map(str::to_owned))
        .collect::<Vec<_>>();
    assert!(
        author_ids
            .iter()
            .any(|id| id == "settings.model-runtime.open-problems"),
        "Problems settings action must be addressable: {author_ids:?}"
    );

    harness.get_by_label("Open Problems").click_accesskit();
    harness.run();

    assert!(!harness.state().settings_open());
    assert!(
        harness.state().tab_bar_states().values().any(|bar| bar
            .tabs
            .iter()
            .any(|tab| tab.pane_type == PaneType::Problems)),
        "settings navigation opens the canonical Problems pane"
    );
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// WP-1 MT-021 — Settings operator-surface gaps
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// AC-3: the Swarm section now carries a REAL concurrency control. This proves the whole chain, not a
/// rendered widget: the control is addressable out-of-process, the outcome the live ComboBox produces
/// mutates the persisted setting, PUSHES the value into the same `ActionChannel` the running MCP/Argus
/// transport drains (so concurrent-agent admission actually changes), and persists through
/// `PUT /workspaces/{id}/settings`. It also proves the setting can only TIGHTEN the flood ceiling.
#[test]
fn swarm_admission_budget_control_drives_the_live_action_channel_and_persists() {
    let transport = StubSettingsTransport::with_loaded(None);
    let handle = leak_runtime_handle();
    let mut app = ok_app();
    app.set_runtime_handle(handle);
    app.set_settings_transport(transport.clone());

    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.state_mut().open_settings();
    harness.run();
    harness.run();

    // Addressable out-of-process by its stable author_id (AC-4).
    let ids = settings_author_ids(&harness);
    assert!(
        ids.iter()
            .any(|id| id == "settings.swarm-max-actions-per-frame"),
        "AC-3/AC-4: the swarm admission-budget control is addressable in the live tree: {ids:?}"
    );

    // Baseline: the live channel runs at the compiled-in ceiling (no extra throttle).
    let channel = harness.state().mcp_action_channel();
    let baseline = channel.lock().unwrap().burst_limit();
    assert_eq!(
        baseline,
        handshake_native::mcp::MAX_ACTIONS_PER_BURST,
        "default admission budget is the compiled-in flood ceiling"
    );

    // Drive the visible control through AccessKit/Argus exactly as an out-of-process model does:
    // open the real ComboBox, then select its stable option node. No SettingsOutcome injection.
    click_settings_author_id(&mut harness, "settings.swarm-max-actions-per-frame");
    harness.run();
    click_settings_author_id(
        &mut harness,
        &handshake_native::settings_dialog::swarm_action_budget_option_author_id(1),
    );
    harness.run();

    assert_eq!(
        harness
            .state()
            .workspace_settings()
            .swarm_max_actions_per_frame,
        1,
        "the persisted setting holds the configured budget"
    );
    assert_eq!(
        channel.lock().unwrap().burst_limit(),
        1,
        "AC-3: the LIVE action channel the MCP/Argus transport drains now admits 1 action per frame"
    );

    // The control cannot widen the compiled-in ceiling (a UI control must never loosen a safety bound).
    harness.state_mut().apply_settings_outcome_for_test(
        handshake_native::settings_dialog::SettingsOutcome::SwarmMaxActionsPerFrameChanged(
            usize::MAX,
        ),
    );
    harness.run();
    assert_eq!(
        channel.lock().unwrap().burst_limit(),
        handshake_native::mcp::MAX_ACTIONS_PER_BURST,
        "an out-of-band budget is clamped down to the compiled-in ceiling, never above it"
    );

    // Persisted through the real settings PUT.
    assert!(
        run_until(&mut harness, 60, |_| transport.save_calls() >= 1),
        "admission budget persisted through PUT /workspaces/{{id}}/settings"
    );
    let blob = transport.saved().expect("a settings_state blob was PUT");
    assert_eq!(
        blob.pointer("/settings/swarm_max_actions_per_frame")
            .and_then(Value::as_u64),
        Some(handshake_native::mcp::MAX_ACTIONS_PER_BURST as u64),
        "persisted blob carries the admission budget"
    );
}

/// MT-021: the model-session cap is a distinct real control. This exercises the live AccessKit popup
/// option, persists the desired project value, proves the exact production GET/PUT request contract,
/// and confirms the UI continues to expose backend-returned requested/in-force/draining truth.
#[test]
fn model_session_concurrency_control_uses_accesskit_and_preserves_runtime_truth() {
    use handshake_native::backend_client::{
        HttpMethod, OperatorChatClient, SwarmConcurrencySnapshot,
    };

    let transport = StubSettingsTransport::with_loaded(None);
    let handle = leak_runtime_handle();
    let client = OperatorChatClient::new("http://127.0.0.1:37501", handle.clone());
    let get = client.swarm_concurrency_get_request();
    assert_eq!(get.method, HttpMethod::Get);
    assert_eq!(
        get.url,
        "http://127.0.0.1:37501/operator-chat/swarm/max-concurrent"
    );
    assert_eq!(get.body, None);
    let put = client.swarm_concurrency_put_request(4);
    assert_eq!(put.method, HttpMethod::Put);
    assert_eq!(put.url, get.url);
    assert_eq!(put.body, Some(serde_json::json!({ "max_concurrent": 4 })));

    let mut app = ok_app();
    app.set_runtime_handle(handle);
    app.set_settings_transport(transport.clone());
    app.set_swarm_concurrency_snapshot_for_test(SwarmConcurrencySnapshot {
        requested: 2,
        max_concurrent: 5,
        fully_applied: false,
        live_sessions: 5,
    });
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.state_mut().open_settings();
    harness.run();
    harness.run();
    let search = harness.get_by_label("Search settings");
    search.focus();
    harness.run();
    harness.get_by_label("Search settings").type_text("session");
    harness.run();
    harness.run();
    let nodes = settings_author_nodes(&harness);
    let status = nodes
        .iter()
        .find(|(id, _, _)| id == "settings.swarm-model-sessions-max-concurrent.status")
        .and_then(|(_, _, label)| label.clone())
        .unwrap_or_else(|| panic!("live coordinator status node; published nodes: {nodes:?}"));
    for truth in [
        "Requested: 2",
        "In force: 5",
        "Fully applied: false",
        "Live sessions: 5",
    ] {
        assert!(status.contains(truth), "status exposes `{truth}`: {status}");
    }

    click_settings_author_id(&mut harness, "settings.swarm-model-sessions-max-concurrent");
    harness.run();
    click_settings_author_id(
        &mut harness,
        &handshake_native::settings_dialog::swarm_model_session_option_author_id(4),
    );
    harness.run();

    assert_eq!(
        harness
            .state()
            .workspace_settings()
            .swarm_model_sessions_max_concurrent,
        Some(4),
        "desired cap is stored separately from backend-reported in-force truth"
    );
    assert_eq!(
        harness.state().last_swarm_concurrency_request_for_test(),
        Some(4),
        "the live UI selection dispatches the coordinator request"
    );
    let status_after_failed_update = settings_author_nodes(&harness)
        .into_iter()
        .find(|(id, _, _)| id == "settings.swarm-model-sessions-max-concurrent.status")
        .and_then(|(_, _, label)| label)
        .expect("model-session status remains accessible after a failed update");
    assert!(
        status_after_failed_update.contains("Update failed: backend unavailable"),
        "the prior coordinator snapshot must not hide the failed update: {status_after_failed_update}"
    );
    assert!(
        run_until(&mut harness, 60, |_| transport.save_calls() >= 1),
        "desired cap persisted through the project settings authority"
    );
    assert_eq!(
        transport.saved().and_then(|blob| {
            blob.pointer("/settings/swarm_model_sessions_max_concurrent")
                .and_then(Value::as_u64)
        }),
        Some(4)
    );
}

/// AC-2 + red-team: cloud consent / export posture is VISIBLE per configured provider lane and is an
/// explicit not-wired state — never a fabricated posture, and never carrying restricted metadata.
///
/// The backend supplies no posture (`api/mod.rs` builds `CloudLaneObservability { consent: None }` and
/// `api/model_access.rs::routes` exposes no consent route), so the ONLY honest render is the explicit
/// unavailable state asserted here. If a future MT wires the route, this test is what forces the UI to
/// stop claiming "not wired" once real posture exists.
#[test]
fn cloud_consent_posture_is_visible_per_lane_and_explicitly_not_wired() {
    use handshake_native::settings_dialog::{
        cloud_consent_posture_author_id, CloudAccessSnapshot, CloudByokRow, CloudCliAuthStatus,
        CloudCliRow, CLOUD_CONSENT_NOT_WIRED_TOKEN, CLOUD_CONSENT_STATUS_AUTHOR_ID,
    };

    let mut app = ok_app();
    app.set_settings_transport(StubSettingsTransport::with_loaded(None));
    // Two configured lanes of each kind, so "each configured provider lane" is really exercised.
    app.set_cloud_snapshot_for_test(CloudAccessSnapshot {
        byok: vec![
            CloudByokRow {
                provider: "anthropic".to_owned(),
                label: "Anthropic (Claude)".to_owned(),
                configured: true,
            },
            CloudByokRow {
                provider: "openai".to_owned(),
                label: "OpenAI (GPT)".to_owned(),
                configured: false,
            },
        ],
        cli_bridge: vec![CloudCliRow {
            provider: "claude_code".to_owned(),
            label: "Claude Code".to_owned(),
            auth_status: CloudCliAuthStatus::LoggedIn,
            login_program: "claude".to_owned(),
            login_args: vec!["login".to_owned()],
            hint: String::new(),
        }],
    });

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(1440.0, 940.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.state_mut().open_settings();
    harness.run();
    harness.run();

    let nodes = settings_author_nodes(&harness);
    let label_for = |author_id: &str| -> String {
        nodes
            .iter()
            .find(|(a, _, _)| a == author_id)
            .unwrap_or_else(|| {
                panic!(
                    "AC-2: consent node '{author_id}' missing from the live settings tree: {:?}",
                    nodes.iter().map(|(a, _, _)| a).collect::<Vec<_>>()
                )
            })
            .2
            .clone()
            .unwrap_or_else(|| panic!("consent node '{author_id}' carries no readable label"))
    };

    // The section-level summary states the whole surface is unavailable.
    let summary = label_for(CLOUD_CONSENT_STATUS_AUTHOR_ID);
    assert!(
        summary.contains(CLOUD_CONSENT_NOT_WIRED_TOKEN),
        "AC-2: the consent summary is an explicit not-wired state: {summary}"
    );

    // One posture row per CONFIGURED lane (BYOK + CLI), each explicitly unavailable.
    for provider in ["anthropic", "openai", "claude_code"] {
        let author_id = cloud_consent_posture_author_id(provider);
        let line = label_for(&author_id);
        assert!(
            line.contains(CLOUD_CONSENT_NOT_WIRED_TOKEN),
            "AC-2: lane '{provider}' renders an explicit not-wired posture: {line}"
        );
        assert!(
            line.contains("none is assumed"),
            "AC-2: lane '{provider}' refuses to assume a posture: {line}"
        );
        // RED TEAM: no fabricated verdict, and no restricted resource metadata (HBR-PRIV-008).
        let lowered = line.to_lowercase();
        for forbidden in ["consented", "approved", "granted", "allowed", "denied"] {
            assert!(
                !lowered.contains(forbidden),
                "lane '{provider}' must not imply a consent verdict ('{forbidden}'): {line}"
            );
        }
        for leaked in [
            "workspace",
            "project",
            "account",
            "artifact",
            "sha256",
            "receipt",
        ] {
            assert!(
                !lowered.contains(leaked),
                "lane '{provider}' must not leak restricted metadata ('{leaked}'): {line}"
            );
        }
    }

    // No control in this block can grant or widen consent — the whole surface is display-only, so no
    // consent-scoped author_id is interactive.
    for (author_id, role, _) in &nodes {
        if author_id.starts_with("settings.cloud.consent.") {
            assert_ne!(
                role, "Button",
                "no consent control may exist: '{author_id}' rendered as a Button"
            );
            assert_ne!(
                role, "CheckBox",
                "no consent control may exist: '{author_id}' rendered as a CheckBox"
            );
            let live_node = harness
                .root()
                .children_recursive()
                .find(|node| node.accesskit_node().author_id() == Some(author_id.as_str()))
                .unwrap_or_else(|| {
                    panic!("consent node '{author_id}' disappeared from the live tree")
                });
            for forbidden_action in [
                egui::accesskit::Action::Click,
                egui::accesskit::Action::SetValue,
            ] {
                assert!(
                    !live_node
                        .accesskit_node()
                        .data()
                        .supports_action(forbidden_action),
                    "display-only consent node '{author_id}' exposed authority-widening action \
                     {forbidden_action:?}"
                );
            }
        }
    }
}

/// AC-5: the four WP-1 Settings author_ids that previously had ZERO test references anywhere in the
/// crate. Nothing proved they reached the live AccessKit tree, so a rename or an accidental drop would
/// have broken every out-of-process model addressing them with no failing test. This asserts they are
/// live and addressable in the real shell; the string values themselves are pinned by
/// `settings_dialog::tests::settings_author_ids_are_stable`.
#[test]
fn wp1_settings_author_ids_are_addressable_in_the_live_tree() {
    use handshake_native::settings_dialog::{
        DIAGNOSTICS_SUBSYSTEM_STATUS_AUTHOR_ID, OPEN_OPERATOR_CHAT_AUTHOR_ID,
        PALMISTRY_STATUS_AUTHOR_ID, RESOURCE_SAMPLING_CHECKBOX_AUTHOR_ID,
    };

    let mut harness = open_settings_harness();
    harness.run();

    let ids = settings_author_ids(&harness);
    for required in [
        OPEN_OPERATOR_CHAT_AUTHOR_ID,
        RESOURCE_SAMPLING_CHECKBOX_AUTHOR_ID,
        PALMISTRY_STATUS_AUTHOR_ID,
        DIAGNOSTICS_SUBSYSTEM_STATUS_AUTHOR_ID,
    ] {
        assert!(
            ids.iter().any(|id| id == required),
            "AC-5: WP-1 settings id '{required}' must be addressable in the live tree: {ids:?}"
        );
    }

    // The Operator Chat deep-link is a real navigation control, driven out-of-process.
    click_settings_author_id(&mut harness, OPEN_OPERATOR_CHAT_AUTHOR_ID);
    harness.run();
    assert!(
        harness.state().tab_bar_states().values().any(|bar| {
            bar.tabs
                .iter()
                .any(|tab| tab.pane_type == PaneType::OperatorChatLaunch)
        }),
        "AC-5: the Operator Chat deep-link opens the real OperatorChatLaunch pane"
    );
}

// ── MT-015 detached Settings window (pop-out / re-dock / close) ──────────────────────────────────────
//
// The MT-015 v4 fail report requires the Settings surface to be targetable as a DETACHED window, not
// only as a root-viewport modal. These drive the REAL shell through the same AccessKit path Argus uses
// out-of-process and prove:
//   * the modal header exposes `settings.popout`;
//   * clicking it detaches the surface into its own viewport whose ROOT node is
//     `popout-window-settings` (Role::Window, label "Handshake – Settings"), registered with the Argus
//     window registry as `popout-settings`, while the modal's `settings.dialog` root STOPS rendering
//     (no double UI) and every settings section still renders + stays addressable;
//   * `settings.redock` restores the modal; the detached Close control closes settings outright and a
//     later re-open comes back as the modal (modal availability restored).
//
// Headless scope (honest): on a plain kittest `egui::Context`, `embed_viewports()` is `true`, so
// `show_viewport_immediate` runs the SAME callback embedded in the current frame instead of raising a
// second OS window (eframe sets `embed_viewports == false` only on the live wgpu/winit backend). The
// content, the window-root node, the mutual exclusion, and the Argus registration are therefore fully
// proven here; the genuine "OS raised a second top-level window and the user clicked its native X" step
// needs a real winit event loop and is NOT faked — the close path is driven through the in-shell seam
// (`close_settings`, which is exactly what the viewport's `close_requested()` branch calls).

/// Every live node that carries a stable author_id, as owned `(author_id, role, label)` triples — the
/// same projection an out-of-process Argus client reads.
fn settings_author_nodes(
    harness: &Harness<'_, HandshakeApp>,
) -> Vec<(String, String, Option<String>)> {
    let mut found = Vec::new();
    let root = harness.root();
    for node in root.children_recursive() {
        let ak = node.accesskit_node();
        if let Some(author_id) = ak.author_id() {
            found.push((author_id.to_owned(), format!("{:?}", ak.role()), ak.label()));
        }
    }
    found
}

fn settings_author_ids(harness: &Harness<'_, HandshakeApp>) -> Vec<String> {
    settings_author_nodes(harness)
        .into_iter()
        .map(|(author_id, _, _)| author_id)
        .collect()
}

/// Click a live node by its stable author_id through AccessKit — the out-of-process steering path.
fn click_settings_author_id(harness: &mut Harness<'_, HandshakeApp>, author_id: &str) {
    harness
        .query_all_by(|n: &egui_kittest::kittest::AccessKitNode<'_>| {
            n.author_id() == Some(author_id)
        })
        .next()
        .unwrap_or_else(|| panic!("author_id '{author_id}' must be addressable in the live tree"))
        .click_accesskit();
}

fn open_settings_harness() -> Harness<'static, HandshakeApp> {
    let mut app = ok_app();
    app.set_settings_transport(StubSettingsTransport::with_loaded(None));
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(1440.0, 940.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.state_mut().open_settings();
    harness.run();
    harness
}

/// Detach an open settings surface through the LIVE `settings.popout` control and settle the frames.
fn detach_open_settings(harness: &mut Harness<'_, HandshakeApp>) {
    click_settings_author_id(harness, "settings.popout");
    harness.run();
    harness.run();
}

#[test]
fn settings_popout_control_detaches_into_its_own_argus_window_and_hides_the_modal() {
    let mut harness = open_settings_harness();

    // Docked (modal) host: the dialog root AND the pop-out control are addressable.
    let ids = settings_author_ids(&harness);
    assert!(
        ids.iter().any(|id| id == "settings.dialog"),
        "the modal host renders its Role::Dialog root: {ids:?}"
    );
    assert!(
        ids.iter().any(|id| id == "settings.popout"),
        "the modal header exposes the pop-out control by its stable author_id: {ids:?}"
    );
    assert!(
        !harness.state().settings_detached(),
        "settings starts docked"
    );

    detach_open_settings(&mut harness);

    assert!(
        harness.state().settings_detached(),
        "clicking settings.popout detaches the surface into its own window"
    );
    assert!(
        harness.state().settings_open(),
        "pop-out does not close settings; it only changes the host window"
    );

    // The detached window's ROOT node is live, with the shared pop-out identity + OS title.
    let nodes = settings_author_nodes(&harness);
    let window = nodes
        .iter()
        .find(|(author_id, _, _)| author_id == "popout-window-settings")
        .unwrap_or_else(|| {
            panic!(
                "popout-window-settings missing from the LIVE tree: {:?}",
                nodes.iter().map(|(a, _, _)| a).collect::<Vec<_>>()
            )
        });
    assert_eq!(
        window.1, "Window",
        "the detached settings root is Role::Window"
    );
    assert_eq!(
        window.2.as_deref(),
        Some("Handshake \u{2013} Settings"),
        "the detached window carries the shared 'Handshake – <label>' title"
    );

    let ids: Vec<String> = nodes.iter().map(|(a, _, _)| a.clone()).collect();
    // NO double UI: the modal's Role::Dialog root is gone while the surface is detached.
    assert!(
        !ids.iter().any(|id| id == "settings.dialog"),
        "the root-viewport modal must NOT render while the surface is detached: {ids:?}"
    );
    // ALL sections still render and stay addressable in the detached host (same render path).
    for expected in [
        "settings.search",
        "settings.list",
        "settings.section.appearance",
        "settings.section.keybindings",
        "settings.section.swarm",
        "settings.section.terminal",
        "settings.section.layout",
        "settings.section.cloud-models",
        "settings.section.model-runtime",
        "settings.section.diagnostics",
        "settings.section.about",
        "settings.theme",
        "settings.view-mode",
        "settings.reset-layout",
        "settings.cloud.byok.openai.key",
        "settings.cloud.byok.anthropic.key",
        "settings.redock",
        "settings.close",
    ] {
        assert!(
            ids.iter().any(|id| id == expected),
            "'{expected}' must stay addressable in the DETACHED settings window: {ids:?}"
        );
    }

    // Argus enumerates the detached window by its stable id, so an out-of-process driver can target it
    // (list_widgets / click / screenshot) without guessing viewport timing.
    let windows = harness.state().mcp_window_registry().list();
    let detached = windows
        .iter()
        .find(|w| w.window_id == "popout-settings")
        .unwrap_or_else(|| {
            panic!(
                "argus.list_windows must enumerate the detached settings window: {:?}",
                windows.iter().map(|w| &w.window_id).collect::<Vec<_>>()
            )
        });
    assert_eq!(detached.title, "Handshake \u{2013} Settings");
    assert!(
        windows.iter().any(|w| w.window_id == "main"),
        "the main window stays registered alongside the detached settings window"
    );
}

#[test]
fn re_docking_the_detached_settings_window_restores_the_modal() {
    let mut harness = open_settings_harness();
    detach_open_settings(&mut harness);
    assert!(harness.state().settings_detached());

    click_settings_author_id(&mut harness, "settings.redock");
    harness.run();
    harness.run();

    assert!(
        !harness.state().settings_detached(),
        "settings.redock returns the surface to the modal host"
    );
    assert!(
        harness.state().settings_open(),
        "re-docking keeps settings open"
    );
    let ids = settings_author_ids(&harness);
    assert!(
        ids.iter().any(|id| id == "settings.dialog"),
        "the modal renders again after re-dock: {ids:?}"
    );
    assert!(
        !ids.iter().any(|id| id == "popout-window-settings"),
        "the detached window is gone after re-dock: {ids:?}"
    );
    assert!(
        !harness
            .state()
            .mcp_window_registry()
            .list()
            .iter()
            .any(|w| w.window_id == "popout-settings"),
        "the detached Argus window is unregistered on re-dock"
    );
}

#[test]
fn closing_the_detached_settings_window_restores_modal_availability() {
    let mut harness = open_settings_harness();
    detach_open_settings(&mut harness);

    // The detached header's Close control (the same author_id as the modal's, scoped to this window).
    click_settings_author_id(&mut harness, "settings.close");
    harness.run();
    harness.run();

    assert!(
        !harness.state().settings_open(),
        "closing the detached window closes settings"
    );
    assert!(
        !harness.state().settings_detached(),
        "the detached host is torn down on close"
    );
    let ids = settings_author_ids(&harness);
    assert!(
        !ids.iter().any(|id| id == "popout-window-settings"),
        "no detached window node survives the close: {ids:?}"
    );
    assert!(
        !ids.iter().any(|id| id == "settings.dialog"),
        "no modal is rendered either — settings is closed: {ids:?}"
    );
    assert!(
        !harness
            .state()
            .mcp_window_registry()
            .list()
            .iter()
            .any(|w| w.window_id == "popout-settings"),
        "the detached Argus window is unregistered on close"
    );

    // Modal availability is restored: the next open comes back as the root-viewport modal.
    harness.state_mut().open_settings();
    harness.run();
    assert!(harness.state().settings_open());
    assert!(
        !harness.state().settings_detached(),
        "a re-open after closing a detached window is docked again"
    );
    let ids = settings_author_ids(&harness);
    assert!(
        ids.iter().any(|id| id == "settings.dialog"),
        "the modal host is available again: {ids:?}"
    );
    assert!(
        ids.iter().any(|id| id == "settings.popout"),
        "and it can be popped out again: {ids:?}"
    );
}

#[test]
fn open_settings_while_detached_keeps_exactly_one_settings_host() {
    let mut harness = open_settings_harness();
    let generation = harness.state().settings_open_count();
    detach_open_settings(&mut harness);

    // A second OpenSettings (HELP menu / palette `settings.open`) while detached must not duplicate the
    // surface, re-dock it, or reset the operator's in-progress state.
    harness.state_mut().open_settings();
    harness.run();

    assert!(harness.state().settings_detached(), "still detached");
    assert_eq!(
        harness.state().settings_open_count(),
        generation,
        "an OpenSettings while already open does not bump the open generation"
    );
    let ids = settings_author_ids(&harness);
    assert_eq!(
        ids.iter()
            .filter(|id| *id == "popout-window-settings")
            .count(),
        1,
        "exactly ONE detached settings window is rendered: {ids:?}"
    );
    assert!(
        !ids.iter().any(|id| id == "settings.dialog"),
        "no modal appears alongside the detached window: {ids:?}"
    );
    assert_eq!(
        harness
            .state()
            .mcp_window_registry()
            .list()
            .iter()
            .filter(|w| w.window_id == "popout-settings")
            .count(),
        1,
        "the Argus registry holds exactly one detached settings window"
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
