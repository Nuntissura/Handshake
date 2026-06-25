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
use handshake_native::settings_dialog::SettingsOutcome;
use handshake_native::workspace_settings::{
    EditorPrefs, RenderWhitespaceMode, SettingsTransport, SettingsTransportError, SyntaxPalette,
    WordWrapMode,
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
    fn save(&self, _workspace_id: &str, settings_state: Value) -> Result<(), SettingsTransportError> {
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
        harness.run();
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
    };
    harness
        .state_mut()
        .apply_settings_outcome_for_test(SettingsOutcome::EditorPrefsChanged(new_prefs));
    harness.run();

    // The live settings now hold the new prefs.
    assert_eq!(
        harness.state().workspace_settings().editor_prefs, new_prefs,
        "AC-001: the editor prefs change is held in the live settings"
    );
    // AC-002: editor font size change did NOT change the chrome theme (separate surfaces).
    assert_eq!(
        harness.state().workspace_settings().theme, chrome_theme_before,
        "AC-002: editor font size is a separate field from the chrome appearance"
    );

    // AC-001 / AC-009: the change persists via the existing debounced PUT (the ONLY save surface).
    let saved = run_until(&mut harness, 80, |_| transport.save_calls() >= 1);
    assert!(saved, "AC-001/AC-009: editor prefs persisted via PUT /workspaces/{{id}}/settings");

    let blob = transport.saved().expect("a settings_state blob was PUT");
    let obj = blob.as_object().expect("settings_state is an object");

    // AC-001: the PUT blob carries all five editor pref values under editor_prefs.
    let ep = obj.get("editor_prefs").and_then(Value::as_object).expect("editor_prefs key");
    assert_eq!(ep.get("editor_font_size").and_then(Value::as_f64), Some(22.0));
    assert_eq!(ep.get("tab_size").and_then(Value::as_u64), Some(8));
    assert_eq!(ep.get("insert_spaces").and_then(Value::as_bool), Some(false));
    assert_eq!(ep.get("render_whitespace").and_then(Value::as_str), Some("all"));
    assert_eq!(
        ep.get("word_wrap").and_then(|w| w.get("boundedColumn")).and_then(Value::as_u64),
        Some(100),
        "AC-001: bounded word-wrap column round-trips through the PUT blob"
    );

    // AC-002: editor_font_size is under editor_prefs, NOT a top-level chrome key; theme is its own key.
    assert!(!obj.contains_key("editor_font_size"), "AC-002: editor font size is NOT a chrome top-level key");
    assert!(obj.contains_key("theme"), "AC-002: chrome appearance (theme) is its own top-level key");

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

// ── AC-001 (LIVE side) / MT-072 note 87: editor prefs WIRE INTO the running MT-079 code panel ────────
//
// Persistence (above) proves the blob is PUT. This proves the WIRE-INTO-LIVE half: applying an
// EditorPrefsChanged outcome (and loading prefs from a stored blob) drives the live mounted
// `CodeEditorPanel` — tab size / insert-spaces / render-whitespace / word-wrap reflect the new values in
// the same frame, NOT only the persisted struct. (editor_font_size has no panel slot today — typed
// follow-up blocker — so it is intentionally NOT asserted on the panel here.)
#[test]
fn editor_prefs_change_drives_the_live_code_panel() {
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
    assert_eq!(panel0.indent_settings(), (4, true), "baseline indent = default (4, spaces)");
    assert!(!panel0.render_whitespace(), "baseline render-whitespace OFF");
    assert!(!panel0.is_wrap_enabled(), "baseline word-wrap OFF");

    // Apply a full editor-prefs change through the same wired outcome the live controls produce.
    let new_prefs = EditorPrefs {
        editor_font_size: 18.0,
        tab_size: 8,
        insert_spaces: false,
        word_wrap: WordWrapMode::BoundedColumn(100),
        render_whitespace: RenderWhitespaceMode::All,
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
    assert!(panel.is_wrap_enabled(), "MT-072 note 87: word_wrap enabled on the live panel");
    assert_eq!(
        panel.wrap_config().wrap_column,
        Some(100),
        "MT-072 note 87: BoundedColumn(100) sets the live wrap column"
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
    assert!(panel.render_whitespace(), "loaded render_whitespace=boundary draws on the live panel");
    assert!(panel.is_wrap_enabled(), "loaded word_wrap=on enabled wrap on the live panel");
    assert_eq!(panel.wrap_config().wrap_column, None, "word_wrap=on wraps at the viewport edge (no column)");
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
    assert!(harness.state().settings_open(), "AC-006: the dialog stayed open against a legacy doc");
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
    assert!(harness.query_by_label("Editor").is_some(), "AC-006: Editor section renders after legacy load");
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
    let editor_kb = obj.get("editor_keybindings").and_then(Value::as_array).expect("editor_keybindings");
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
    assert!(kb.contains_key("app.quick_switcher.open") && kb.contains_key("app.command_palette.open"));
    assert!(
        !kb.contains_key("code.open_find"),
        "RISK-001: the editor binding did NOT leak into the backend-validated keybindings map"
    );
}
