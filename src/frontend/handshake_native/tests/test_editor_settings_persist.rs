//! WP-KERNEL-012 MT-072 (E12) — editor Settings persistence proofs (PT-001).
//!
//! These proofs drive the REAL `HandshakeApp` headlessly via egui_kittest and prove the Editor settings
//! sections persist THROUGH the SAME WP-011 PostgreSQL-backed `GET`/`PUT /workspaces/:id/settings`
//! surface — there is NO new persistence system, NO SQLite, NO new endpoint (AC-009). A scriptable
//! `StubSettingsTransport` records the PUT blob + serves a scripted GET, so the open -> change -> persist
//! round-trip is provable with no live server (the live-PG round-trip is the
//! NEEDS_MANAGED_RESOURCE_PROOF case the MT gates; the shape/serde round-trip is proven here + in the
//! `workspace_settings` unit tests).
//!
//! - AC-001: setting editor_font_size / tab_size / insert_spaces / word_wrap / render_whitespace then
//!   applying issues a PUT carrying those values; the GET-on-open path reloads identical values.
//! - AC-002: editor_font_size is a SEPARATE field from the chrome appearance (theme) — the persisted blob
//!   carries them as distinct keys and changing one does not change the other.
//! - AC-006: a legacy WP-011-era settings doc (no editor keys) loads cleanly via the GET path (the dialog
//!   opens against it with the editor defaults — no hard-fail).
//! - AC-009: the ONLY persistence calls are the existing WP-011 GET/PUT — the stub transport is the sole
//!   I/O surface; no other save path is exercised.

use std::sync::{Arc, Mutex};

use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend_client::HealthInfo;
use handshake_native::code_editor::HighlightScope;
use handshake_native::settings_dialog::SettingsOutcome;
use handshake_native::theme::{HsTheme, MUTED_PALETTE, STANDARD_PALETTE};
use handshake_native::workspace_settings::{
    EditorPrefs, RenderWhitespaceMode, SettingsTransport, SettingsTransportError, SyntaxPalette,
    SyntaxPaletteMode, WordWrapMode,
};
use serde_json::Value;

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

// ── AC-001 / AC-002 / AC-009: editor prefs persist via the existing PUT; distinct from chrome ────────
#[test]
fn editor_prefs_change_persists_via_existing_put_and_reloads() {
    let transport = StubSettingsTransport::with_loaded(None);
    let handle = leak_runtime_handle();

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
    let handle2 = leak_runtime_handle();
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
    let handle = leak_runtime_handle();
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
    let handle = leak_runtime_handle();
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
    assert!(panel.line_numbers_enabled(), "gutter line numbers default on");

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

#[test]
fn syntax_palette_change_drives_the_live_code_panel_immediately() {
    let transport = StubSettingsTransport::with_loaded(None);
    let handle = leak_runtime_handle();
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
    let handle = leak_runtime_handle();
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
    let handle = leak_runtime_handle();
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
    let handle = leak_runtime_handle();
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
    let handle = leak_runtime_handle();
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
    let handle = leak_runtime_handle();
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
