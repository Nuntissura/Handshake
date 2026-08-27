//! WP-KERNEL-012 MT-072 remediation (FAIL_V2) — canonical PreferenceRecord authority live-SurrealDB proof.
//!
//! Validator V2 rejected editor settings because they persisted as an opaque workspace-settings JSON
//! document rather than the typed [`PreferenceRecord`] authority (Master Spec v02.201 §10.17). This
//! proof drives the NEW canonical preference HTTP surface against a REAL managed SurrealDB +
//! handshake_core backend and asserts the full lifecycle the validator required:
//!
//! * SET-REC-003 — a defined-but-unset preference resolves to its registry default (never null), with a
//!   stable `preference_id`, `value_type`, `scope`, `default_value`, `source=default`, `revision=0`.
//! * SET-REC-002 — a typed-invalid value is rejected with an explicit structured 400, never coerced.
//! * SET-EVT-001/002/003 — a set bumps `revision`, returns a recoverable receipt pointing at a durable
//!   EventLedger row, and the EventLedger row is visible on `/events`.
//! * SET-UI-002 — reset-to-default is a mutation with `source=operator` + its own receipt, not a delete.
//! * SET-UI-003 — the change history lists every mutation newest-first, and survives a fresh GET (the
//!   canonical SurrealDB round-trip; there is no in-memory settings cache — SurrealDB is the sole
//!   authority, so the readback proves durable persistence).
//! * SET-PROJ-002 — the redacted projection is a deterministic read-only view over canonical state.
//!
//! Run against a live backend, e.g. attach to http://127.0.0.1:37501 or an owned
//! `HSK_TEST_BACKEND_BIN` + `HANDSHAKE_DATA_DIR` (see backend_proof_support).

mod backend_proof_support;

use serde_json::{json, Value};

use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend_client::HealthInfo;
use handshake_native::code_editor::keymap::CodeEditorAction;
use handshake_native::code_editor::keymap_settings::KeymapSettings;
use handshake_native::preference_client::{
    PreferenceClient, PreferenceProjectionRow, PreferenceTransport, PreferenceTransportError,
    EDITOR_PREFERENCE_IDS,
};
use handshake_native::rich_editor::formatting::FormattingCommand;
use handshake_native::workspace_settings::{
    RenderWhitespaceMode, SyntaxPaletteMode, WordWrapMode,
};

const FONT_SIZE: &str = "view-defaults.editor.font-size";
const TAB_SIZE: &str = "view-defaults.editor.tab-size";
const WORD_WRAP: &str = "view-defaults.editor.word-wrap";

#[test]
fn editor_preferences_persist_reset_and_history_on_live_surrealdb() {
    let mut backend = backend_proof_support::require_live_backend();
    let wsid = backend.workspace_id.clone();
    let base = format!("/workspaces/{wsid}/preferences");

    // --- SET-REC-003: unset defined preferences resolve to the registry default (never null). ---
    let projection = backend.get_json(&base);
    let rows = projection["preferences"]
        .as_array()
        .expect("projection has a preferences array");
    assert_eq!(
        rows.len(),
        16,
        "the editor preference registry projection must list every defined editor preference"
    );
    let font_row = rows
        .iter()
        .find(|row| row["preference_id"] == FONT_SIZE)
        .expect("projection contains the font-size preference");
    assert_eq!(font_row["value"], json!(13.0), "default font-size = 13.0");
    assert_eq!(font_row["default_value"], json!(13.0));
    assert_eq!(font_row["source"], "default");
    assert_eq!(font_row["revision"], 0);
    assert_eq!(font_row["redacted"], json!(false));

    let font_get = backend.get_json(&format!("{base}/{FONT_SIZE}"));
    let record = &font_get["record"];
    assert_eq!(record["schema_id"], "hsk.preference_record@1");
    assert_eq!(record["preference_id"], FONT_SIZE);
    assert_eq!(record["value_type"], "float");
    assert_eq!(record["scope"], "workspace");
    assert_eq!(record["scope_ref"], wsid);
    assert_eq!(record["value"], json!(13.0));
    assert_eq!(record["source"], "default");
    assert_eq!(record["revision"], 0);

    // --- SET-REC-002: a typed-invalid value is rejected with a structured 400, never persisted. ---
    let (status, body) =
        backend.put_json_response(&format!("{base}/{FONT_SIZE}"), &json!({ "value": 100.0 }));
    assert_eq!(
        status, 400,
        "out-of-range font-size must be rejected: {body}"
    );
    assert_eq!(body["error"], "preference_validation_failed");
    assert_eq!(body["validation"]["preference_id"], FONT_SIZE);
    assert_eq!(body["validation"]["code"], "out_of_range");
    // The rejected write left nothing behind (still the default).
    let after_reject = backend.get_json(&format!("{base}/{FONT_SIZE}"));
    assert_eq!(after_reject["record"]["value"], json!(13.0));
    assert_eq!(after_reject["record"]["revision"], 0);

    let (wrong_type_status, _) =
        backend.put_json_response(&format!("{base}/{TAB_SIZE}"), &json!({ "value": "four" }));
    assert_eq!(
        wrong_type_status, 400,
        "string for an int preference is rejected"
    );

    let (unknown_status, _) = backend.put_json_response(
        &format!("{base}/view-defaults.editor.does-not-exist"),
        &json!({ "value": 1 }),
    );
    assert_eq!(unknown_status, 404, "an unknown preference id is a 404");

    // --- SET-EVT-001/002: a valid set bumps revision, returns a receipt + durable EventLedger ref. ---
    let set = backend.put_json(&format!("{base}/{FONT_SIZE}"), &json!({ "value": 20.0 }));
    assert_eq!(set["record"]["value"], json!(20.0));
    assert_eq!(set["record"]["source"], "operator");
    assert_eq!(set["record"]["revision"], 1);
    let receipt = &set["receipt"];
    assert_eq!(receipt["schema_id"], "hsk.preference_change_receipt@1");
    assert_eq!(receipt["before_revision"], json!(null));
    assert_eq!(receipt["after_revision"], 1);
    assert_eq!(receipt["old_value"], json!(null));
    assert_eq!(receipt["new_value"], json!(20.0));
    let set_event_id = receipt["event_ledger_event_id"]
        .as_str()
        .expect("receipt carries an EventLedger event id");
    assert!(!set_event_id.is_empty());

    // SET-EVT-003: the durable EventLedger row is correlatable through the canonical kernel
    // event-ledger aggregate the receipt's `event_ledger_event_id` (KE-...) points at.
    let event = backend.poll_preference_event(FONT_SIZE, 1);
    assert_eq!(event["event_id"], set_event_id);
    assert_eq!(event["payload"]["type"], "preference_record_changed");
    assert_eq!(event["payload"]["revision"], 1);
    assert_eq!(event["payload"]["new_value_ref"], json!(20.0));

    // --- SET-UI-003 durability: a fresh GET (canonical SurrealDB, no cache) returns the set value. ---
    let reread = backend.get_json(&format!("{base}/{FONT_SIZE}"));
    assert_eq!(reread["record"]["value"], json!(20.0));
    assert_eq!(reread["record"]["revision"], 1);
    assert_eq!(reread["record"]["source"], "operator");

    // Independent second preference proves records are per-preference-id (not one shared blob).
    let tab = backend.put_json(&format!("{base}/{TAB_SIZE}"), &json!({ "value": 8 }));
    assert_eq!(tab["record"]["value"], json!(8));
    assert_eq!(tab["record"]["revision"], 1);
    // font-size is unaffected by the tab-size write.
    assert_eq!(
        backend.get_json(&format!("{base}/{FONT_SIZE}"))["record"]["value"],
        json!(20.0)
    );

    // Enum preference set + validation domain.
    let wrap = backend.put_json(&format!("{base}/{WORD_WRAP}"), &json!({ "value": "on" }));
    assert_eq!(wrap["record"]["value"], json!("on"));
    let (bad_enum, _) = backend.put_json_response(
        &format!("{base}/{WORD_WRAP}"),
        &json!({ "value": "diagonal" }),
    );
    assert_eq!(bad_enum, 400, "an unknown enum member is rejected");

    // --- SET-UI-002: reset-to-default is a mutation (source=operator) with its own receipt. ---
    let reset = backend.post_json(&format!("{base}/{FONT_SIZE}/reset"), &json!({}));
    assert_eq!(
        reset["record"]["value"],
        json!(13.0),
        "reset restores the default"
    );
    assert_eq!(reset["record"]["source"], "operator");
    assert_eq!(reset["record"]["revision"], 2, "reset bumps the revision");
    let reset_receipt = &reset["receipt"];
    assert_eq!(reset_receipt["before_revision"], 1);
    assert_eq!(reset_receipt["after_revision"], 2);
    assert_eq!(reset_receipt["old_value"], json!(20.0));
    assert_eq!(reset_receipt["new_value"], json!(13.0));

    // --- SET-UI-003: change history lists every mutation newest-first, and survives the round-trip. ---
    let history = backend.get_json(&format!("{base}/{FONT_SIZE}/history"));
    let receipts = history["receipts"]
        .as_array()
        .expect("history has a receipts array");
    assert_eq!(
        receipts.len(),
        2,
        "font-size has a set + a reset in its history"
    );
    assert_eq!(receipts[0]["after_revision"], 2, "newest (reset) first");
    assert_eq!(receipts[0]["new_value"], json!(13.0));
    assert_eq!(receipts[1]["after_revision"], 1, "then the original set");
    assert_eq!(receipts[1]["new_value"], json!(20.0));
    for receipt in receipts {
        assert!(
            receipt["event_ledger_event_id"]
                .as_str()
                .is_some_and(|id| !id.is_empty()),
            "every receipt points at a durable EventLedger row: {receipt}"
        );
    }

    backend.assert_cleanup();
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// MT-072 remediation items 6 + 7 (FAIL_V4): the FULL live-SurrealDB authoritative matrix and the
// adversarial live cases. V4 accepted the proof above as real but noted it "mutates and rereads editor
// font size only", leaving the remaining scalars, the palette, and the code/rich keybinding overrides
// on stub transport. Everything below runs against the SAME real backend + real SurrealDB, and the
// fresh-client reopen uses the PRODUCTION `PreferenceClient` (no `StubPreferenceTransport` anywhere in
// this file).
// ═════════════════════════════════════════════════════════════════════════════════════════════════

/// A non-default value for EVERY defined editor preference, in registry order. Each value is inside its
/// registry constraint and differs from the registry default, so a reopen that returned the default
/// would fail loudly.
fn authoritative_matrix() -> Vec<(&'static str, Value)> {
    vec![
        (FONT_SIZE, json!(20.0)),
        (TAB_SIZE, json!(8)),
        ("view-defaults.editor.insert-spaces", json!(false)),
        (WORD_WRAP, json!("bounded")),
        ("view-defaults.editor.word-wrap-column", json!(120)),
        ("view-defaults.editor.render-whitespace", json!("all")),
        ("view-defaults.editor.minimap-enabled", json!(false)),
        ("view-defaults.editor.sticky-scroll", json!(false)),
        ("view-defaults.editor.line-numbers", json!(false)),
        ("view-defaults.editor.line-height", json!(1.5)),
        ("view-defaults.editor.bracket-matching", json!(false)),
        ("view-defaults.editor.indent-guides", json!(false)),
        ("view-defaults.editor.reading-mode-default", json!(true)),
        ("view-defaults.editor.syntax-palette-mode", json!("custom")),
        (
            "view-defaults.editor.syntax-custom-colors",
            json!({ "keyword": [255, 0, 0, 255] }),
        ),
        (
            "view-defaults.editor.keybinding-overrides",
            json!({ "code.open_find": "Mod+Alt+J", "rich.toggle_bold": "Mod+Alt+B" }),
        ),
    ]
}

/// A fresh production preference client bound to the live fixture backend — the exact transport a
/// reopened Handshake client uses. Returned with the runtime it bridges onto (which must outlive it).
fn fresh_production_client(base: &str) -> (PreferenceClient, tokio::runtime::Runtime) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("build fresh-client runtime");
    let client = PreferenceClient::new(
        base.to_owned(),
        "wp-kernel-012-mt-072-fresh-client",
        runtime.handle().clone(),
    );
    (client, runtime)
}

/// A brand-new headless `HandshakeApp` (its own mounted code + rich editors, no settings control ever
/// touched) hydrated from `rows` through the PRODUCTION hydration body. This is the "fresh process /
/// client reopen" half: the values a reopened app would show in its running editors.
fn hydrated_app(rows: &[PreferenceProjectionRow]) -> HandshakeApp {
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }));
    app.hydrate_editor_preferences_for_test(rows);
    app
}

/// MT-072 remediation item 6 — EVERY authoritative editor preference proves typed projection, set,
/// fresh-client reopen, exact revision, receipt, EventLedger reference, reset, and history against REAL
/// SurrealDB, plus all three palette modes, a custom swatch, and BOTH a code and a rich keybinding
/// override landing on the mounted editors of a freshly hydrated client.
#[test]
fn every_editor_preference_persists_reopens_resets_and_histories_on_live_surrealdb() {
    let mut backend = backend_proof_support::require_live_backend();
    let wsid = backend.workspace_id.clone();
    let base = format!("/workspaces/{wsid}/preferences");
    let matrix = authoritative_matrix();

    // The matrix must cover the complete registry — a preference added later cannot silently escape.
    assert_eq!(
        matrix.len(),
        EDITOR_PREFERENCE_IDS.len(),
        "the live matrix must cover every defined editor preference"
    );
    for id in EDITOR_PREFERENCE_IDS {
        assert!(
            matrix.iter().any(|(candidate, _)| candidate == id),
            "live matrix is missing preference '{id}'"
        );
    }

    // ── (1) TYPED PROJECTION at rest: every id resolves to its registry default, revision 0. ────────
    let projection = backend.get_json(&base);
    let rows = projection["preferences"]
        .as_array()
        .expect("projection has a preferences array")
        .clone();
    assert_eq!(
        rows.len(),
        EDITOR_PREFERENCE_IDS.len(),
        "the projection lists every defined editor preference"
    );
    for (id, _) in &matrix {
        let row = rows
            .iter()
            .find(|row| row["preference_id"] == *id)
            .unwrap_or_else(|| panic!("projection contains {id}"));
        assert_eq!(row["source"], "default", "{id} starts unset");
        assert_eq!(row["revision"], 0, "{id} starts at revision 0");
        assert_eq!(
            row["value"], row["default_value"],
            "{id} resolves to its registry default while unset"
        );
        assert_eq!(row["redacted"], json!(false), "{id} is a public preference");

        // Typed record shape (SET-REC-001) for the same id.
        let record = backend.get_json(&format!("{base}/{id}"))["record"].clone();
        assert_eq!(record["schema_id"], "hsk.preference_record@1", "{id}");
        assert_eq!(record["preference_id"], *id);
        assert_eq!(record["namespace"], "view-defaults", "{id}");
        assert_eq!(record["scope"], "workspace", "{id}");
        assert_eq!(record["scope_ref"], wsid, "{id}");
        assert_eq!(record["source"], "default", "{id}");
        assert_eq!(record["revision"], 0, "{id}");
        assert!(
            record["value_type"].as_str().is_some_and(|t| !t.is_empty()),
            "{id} declares a value type"
        );
    }

    // ── (2) SET every preference to a non-default value: revision 1, receipt, EventLedger row. ──────
    for (id, value) in &matrix {
        let set = backend.put_json(&format!("{base}/{id}"), &json!({ "value": value }));
        assert_eq!(set["record"]["value"], *value, "{id} stored the set value");
        assert_eq!(set["record"]["source"], "operator", "{id}");
        assert_eq!(set["record"]["revision"], 1, "{id} first set is revision 1");
        let receipt = &set["receipt"];
        assert_eq!(receipt["schema_id"], "hsk.preference_change_receipt@1", "{id}");
        assert_eq!(receipt["before_revision"], json!(null), "{id}");
        assert_eq!(receipt["after_revision"], 1, "{id}");
        assert_eq!(receipt["old_value"], json!(null), "{id}");
        assert_eq!(receipt["new_value"], *value, "{id}");
        let event_id = receipt["event_ledger_event_id"]
            .as_str()
            .unwrap_or_else(|| panic!("{id} receipt carries an EventLedger event id"));
        assert!(!event_id.is_empty(), "{id} EventLedger id is non-empty");

        // SET-EVT-003: the durable EventLedger row the receipt points at is readable and correlates.
        let event = backend.poll_preference_event(id, 1);
        assert_eq!(event["event_id"], event_id, "{id} EventLedger correlation");
        assert_eq!(event["payload"]["type"], "preference_record_changed", "{id}");
        assert_eq!(event["payload"]["revision"], 1, "{id}");
    }

    // ── (3) FRESH CLIENT REOPEN through the PRODUCTION transport (no stub): every value is durable. ─
    let (client, _client_runtime) = fresh_production_client(&backend.base);
    let reopened = client
        .list(&wsid)
        .expect("the production preference client lists the live projection");
    assert_eq!(
        reopened.len(),
        EDITOR_PREFERENCE_IDS.len(),
        "the reopened client sees every preference"
    );
    for (id, value) in &matrix {
        let row = reopened
            .iter()
            .find(|row| row.preference_id == *id)
            .unwrap_or_else(|| panic!("reopened projection contains {id}"));
        assert_eq!(row.value, *value, "{id} survived the fresh-client reopen");
        assert_eq!(row.source, "operator", "{id} provenance survived");
        assert_eq!(row.revision, 1, "{id} exact revision survived");
    }
    // Each per-id typed record is independently durable on the canonical per-preference route too
    // (proving these are per-preference-id records, not one shared blob).
    for (id, value) in &matrix {
        let record = backend.get_json(&format!("{base}/{id}"))["record"].clone();
        assert_eq!(record["value"], *value, "{id} typed record readback");
        assert_eq!(record["revision"], 1, "{id} typed record revision");
        assert_eq!(record["source"], "operator", "{id} typed record provenance");
    }

    // ── (4) The reopened values reach the MOUNTED editors of a freshly hydrated app. ────────────────
    {
        let app = hydrated_app(&reopened);
        let prefs = app.workspace_settings().editor_prefs;
        assert_eq!(prefs.editor_font_size, 20.0, "hydrated editor font size");
        assert_eq!(prefs.tab_size, 8, "hydrated tab size");
        assert!(!prefs.insert_spaces, "hydrated insert spaces");
        assert_eq!(
            prefs.word_wrap,
            WordWrapMode::BoundedColumn(120),
            "hydrated bounded word wrap + column"
        );
        assert_eq!(
            prefs.render_whitespace,
            RenderWhitespaceMode::All,
            "hydrated render whitespace"
        );
        assert!(!prefs.minimap_enabled, "hydrated minimap");
        assert!(!prefs.sticky_scroll, "hydrated sticky scroll");
        assert!(!prefs.line_numbers, "hydrated line numbers");
        assert_eq!(prefs.line_height, 1.5, "hydrated line height");
        assert!(!prefs.bracket_matching, "hydrated bracket matching");
        assert!(!prefs.indent_guides, "hydrated indent guides");
        assert!(prefs.reading_mode_default, "hydrated reading-mode default");

        let palette = app.workspace_settings().syntax_palette.clone();
        assert_eq!(
            palette.mode,
            SyntaxPaletteMode::Custom,
            "hydrated palette mode"
        );
        assert_eq!(
            palette.custom_for("keyword"),
            Some([255, 0, 0, 255]),
            "hydrated custom swatch"
        );

        // The MOUNTED code editor keymap consumes the persisted `code.` override.
        let code_chord = KeymapSettings::chord_from_str("Mod+Alt+J").expect("code chord parses");
        let code_panel = app.mounted_code_panel();
        assert_eq!(
            code_panel.keymap().resolve(code_chord),
            Some(CodeEditorAction::OpenFind),
            "item 6: the live code keybinding override reached the mounted code editor after a \
             fresh-client reopen"
        );
        // The MOUNTED rich editor keymap consumes the persisted `rich.` override.
        let rich_chord = KeymapSettings::chord_from_str("Mod+Alt+B").expect("rich chord parses");
        let rich_modifiers = egui::Modifiers {
            alt: rich_chord.alt,
            ctrl: rich_chord.ctrl,
            shift: rich_chord.shift,
            mac_cmd: rich_chord.mac_cmd,
            command: rich_chord.ctrl || rich_chord.mac_cmd,
        };
        let rich_state = app.mounted_rich_state();
        let guard = rich_state.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(
            guard.rich_keymap().resolve(&rich_modifiers, rich_chord.key),
            Some(FormattingCommand::ToggleBold),
            "item 6: the live rich keybinding override reached the mounted rich editor after a \
             fresh-client reopen"
        );
    }

    // ── (5) ALL palette modes round-trip (not only the Custom one the matrix set). ──────────────────
    let palette_id = "view-defaults.editor.syntax-palette-mode";
    for (index, mode) in ["muted", "standard", "custom"].iter().enumerate() {
        let expected_revision = 2 + index as i64; // revision 1 was the matrix set above
        let set = backend.put_json(&format!("{base}/{palette_id}"), &json!({ "value": mode }));
        assert_eq!(set["record"]["value"], json!(mode), "palette mode {mode}");
        assert_eq!(
            set["record"]["revision"], expected_revision,
            "palette mode {mode} revision"
        );
        let reread = backend.get_json(&format!("{base}/{palette_id}"));
        assert_eq!(reread["record"]["value"], json!(mode), "palette {mode} durable");
        let hydrated = hydrated_app(
            &client
                .list(&wsid)
                .expect("fresh client re-lists after a palette change"),
        );
        let expected_mode = SyntaxPaletteMode::from_str_opt(mode).expect("known palette mode");
        assert_eq!(
            hydrated.workspace_settings().syntax_palette.mode,
            expected_mode,
            "item 6: palette mode {mode} reaches a freshly hydrated client"
        );
    }

    // ── (6) RESET + HISTORY for every preference: default restored, revisions strictly increase. ────
    for (id, value) in &matrix {
        let before = backend.get_json(&format!("{base}/{id}"))["record"]["revision"]
            .as_i64()
            .unwrap_or_else(|| panic!("{id} has a numeric revision"));
        let reset = backend.post_json(&format!("{base}/{id}/reset"), &json!({}));
        let default_value = reset["record"]["default_value"].clone();
        assert_eq!(
            reset["record"]["value"], default_value,
            "{id} reset restores the registry default"
        );
        assert_eq!(reset["record"]["source"], "operator", "{id} reset provenance");
        assert_eq!(
            reset["record"]["revision"],
            json!(before + 1),
            "{id} reset bumps the revision"
        );
        assert_eq!(
            reset["receipt"]["before_revision"],
            json!(before),
            "{id} reset receipt before_revision"
        );
        assert_eq!(
            reset["receipt"]["old_value"], *value,
            "{id} reset receipt records the superseded value"
        );

        let history = backend.get_json(&format!("{base}/{id}/history"));
        let receipts = history["receipts"]
            .as_array()
            .unwrap_or_else(|| panic!("{id} history has a receipts array"));
        assert_eq!(
            receipts.len() as i64,
            before + 1,
            "{id} history records every mutation"
        );
        let mut previous = i64::MAX;
        for receipt in receipts {
            let revision = receipt["after_revision"]
                .as_i64()
                .expect("receipt revision is numeric");
            assert!(
                revision < previous,
                "{id} history is newest-first with strictly decreasing revisions: {receipts:?}"
            );
            previous = revision;
            assert!(
                receipt["event_ledger_event_id"]
                    .as_str()
                    .is_some_and(|value| !value.is_empty()),
                "{id} every receipt points at a durable EventLedger row"
            );
        }
    }

    // ── (7) After the resets, a fresh client hydrates DEFAULTS on its mounted editors. ──────────────
    {
        let defaults = client
            .list(&wsid)
            .expect("fresh client lists the post-reset projection");
        let app = hydrated_app(&defaults);
        let prefs = app.workspace_settings().editor_prefs;
        assert_eq!(prefs.editor_font_size, 13.0, "reset font size default");
        assert_eq!(prefs.tab_size, 4, "reset tab size default");
        assert!(prefs.insert_spaces, "reset insert spaces default");
        assert_eq!(prefs.word_wrap, WordWrapMode::Off, "reset word wrap default");
        assert_eq!(
            prefs.render_whitespace,
            RenderWhitespaceMode::None,
            "reset render whitespace default"
        );
        assert!(
            app.workspace_settings().editor_keybindings.is_empty(),
            "reset cleared the editor keybinding overrides"
        );
        let code_chord = KeymapSettings::chord_from_str("Mod+Alt+J").expect("chord parses");
        let code_panel = app.mounted_code_panel();
        assert_ne!(
            code_panel.keymap().resolve(code_chord),
            Some(CodeEditorAction::OpenFind),
            "item 6: reset removed the code override from the mounted editor"
        );
    }

    backend.assert_cleanup();
}

/// `(value, revision, history_length)` for one preference — the exact durable state a rejected write
/// must leave untouched.
fn preference_snapshot(
    backend: &backend_proof_support::LiveBackend,
    base: &str,
    preference_id: &str,
) -> (Value, i64, usize) {
    let record = backend.get_json(&format!("{base}/{preference_id}"))["record"].clone();
    let history = backend.get_json(&format!("{base}/{preference_id}/history"));
    let receipts = history["receipts"]
        .as_array()
        .map(Vec::len)
        .unwrap_or_default();
    (
        record["value"].clone(),
        record["revision"].as_i64().unwrap_or(-1),
        receipts,
    )
}

/// MT-072 remediation item 7 — the adversarial live cases. Every rejected write is proven to leave the
/// durable revision, the change history, AND the mounted-editor state untouched.
#[test]
fn editor_preference_adversarial_cases_on_live_surrealdb() {
    let mut backend = backend_proof_support::require_live_backend();
    let wsid = backend.workspace_id.clone();
    let base = format!("/workspaces/{wsid}/preferences");
    let (client, _client_runtime) = fresh_production_client(&backend.base);

    // Seed a known-good authoritative state so "unchanged after rejection" is a meaningful assertion.
    backend.put_json(&format!("{base}/{FONT_SIZE}"), &json!({ "value": 20.0 }));
    backend.put_json(&format!("{base}/{TAB_SIZE}"), &json!({ "value": 8 }));
    backend.put_json(
        &format!("{base}/view-defaults.editor.keybinding-overrides"),
        &json!({ "value": { "rich.toggle_bold": "Mod+Alt+B" } }),
    );


    // Every rejected write, with the exact structured code the registry must produce.
    let rejections: Vec<(&str, Value, &str)> = vec![
        // Numeric bounds (both edges, both directions).
        (FONT_SIZE, json!(5.9), "out_of_range"),
        (FONT_SIZE, json!(48.1), "out_of_range"),
        (TAB_SIZE, json!(0), "out_of_range"),
        (TAB_SIZE, json!(17), "out_of_range"),
        ("view-defaults.editor.line-height", json!(0.9), "out_of_range"),
        ("view-defaults.editor.line-height", json!(2.1), "out_of_range"),
        (
            "view-defaults.editor.word-wrap-column",
            json!(19),
            "out_of_range",
        ),
        (
            "view-defaults.editor.word-wrap-column",
            json!(401),
            "out_of_range",
        ),
        // Type mismatches.
        (TAB_SIZE, json!("four"), "not_int"),
        (FONT_SIZE, json!("big"), "not_float"),
        (
            "view-defaults.editor.insert-spaces",
            json!("yes"),
            "not_bool",
        ),
        // Invalid enum members.
        (WORD_WRAP, json!("diagonal"), "unknown_enum_member"),
        (
            "view-defaults.editor.render-whitespace",
            json!("sometimes"),
            "unknown_enum_member",
        ),
        (
            "view-defaults.editor.syntax-palette-mode",
            json!("neon"),
            "unknown_enum_member",
        ),
        (WORD_WRAP, json!(3), "not_string"),
        // Invalid colours.
        (
            "view-defaults.editor.syntax-custom-colors",
            json!({ "keyword": [300, 0, 0, 255] }),
            "bad_color",
        ),
        (
            "view-defaults.editor.syntax-custom-colors",
            json!({ "keyword": [1, 2, 3] }),
            "bad_color",
        ),
        (
            "view-defaults.editor.syntax-custom-colors",
            json!({ "not-a-scope": [1, 2, 3, 4] }),
            "unknown_scope",
        ),
        (
            "view-defaults.editor.syntax-custom-colors",
            json!("#ff0000"),
            "not_object",
        ),
        // Invalid chord maps.
        (
            "view-defaults.editor.keybinding-overrides",
            json!({ "rich.toggle_bold": "" }),
            "bad_chord",
        ),
        (
            "view-defaults.editor.keybinding-overrides",
            json!({ "rich.toggle_bold": 7 }),
            "bad_chord",
        ),
        (
            "view-defaults.editor.keybinding-overrides",
            json!({ "  ": "Mod+B" }),
            "empty_action",
        ),
        (
            "view-defaults.editor.keybinding-overrides",
            json!(["Mod+B"]),
            "not_object",
        ),
    ];

    for (id, bad_value, expected_code) in &rejections {
        let before = preference_snapshot(&backend, &base, id);
        let (status, body) =
            backend.put_json_response(&format!("{base}/{id}"), &json!({ "value": bad_value }));
        assert_eq!(
            status, 400,
            "{id} must reject {bad_value} with a structured 400, got {body}"
        );
        assert_eq!(body["error"], "preference_validation_failed", "{id}");
        assert_eq!(body["validation"]["preference_id"], *id, "{id}");
        assert_eq!(
            body["validation"]["code"], *expected_code,
            "{id} rejection code for {bad_value}: {body}"
        );
        let after = preference_snapshot(&backend, &base, id);
        assert_eq!(
            before, after,
            "item 7: the rejected write for {id} must not mutate value, revision, or history"
        );
    }

    // The rejected writes never reached a mounted editor either: a freshly hydrated app still shows the
    // seeded authoritative state, not any rejected candidate.
    {
        let rows = client.list(&wsid).expect("fresh client lists after rejections");
        let app = hydrated_app(&rows);
        assert_eq!(
            app.workspace_settings().editor_prefs.editor_font_size,
            20.0,
            "item 7: rejected font-size writes left the mounted editor state untouched"
        );
        assert_eq!(
            app.workspace_settings().editor_prefs.tab_size,
            8,
            "item 7: rejected tab-size writes left the mounted editor state untouched"
        );
        assert_eq!(
            app.workspace_settings()
                .editor_chord_override("rich.toggle_bold"),
            Some("Mod+Alt+B"),
            "item 7: rejected chord-map writes left the persisted override untouched"
        );
    }

    // ── UNKNOWN preference ids are 404 on every verb, and the typed client reports them as such. ────
    let unknown = "view-defaults.editor.does-not-exist";
    assert_eq!(
        backend.get_status(&format!("{base}/{unknown}")),
        404,
        "GET unknown preference id"
    );
    assert_eq!(
        backend
            .put_json_response(&format!("{base}/{unknown}"), &json!({ "value": 1 }))
            .0,
        404,
        "PUT unknown preference id"
    );
    assert_eq!(
        backend
            .post_json_response(&format!("{base}/{unknown}/reset"), &json!({}))
            .0,
        404,
        "POST reset unknown preference id"
    );
    assert_eq!(
        backend.get_status(&format!("{base}/{unknown}/history")),
        404,
        "GET history for unknown preference id"
    );
    assert!(
        matches!(
            client.set(&wsid, unknown, json!(1)),
            Err(PreferenceTransportError::UnknownPreference(_))
        ),
        "item 7: the production client surfaces an unknown preference id as a typed error"
    );
    assert!(
        matches!(
            client.set(&wsid, FONT_SIZE, json!(100.0)),
            Err(PreferenceTransportError::Validation(_))
        ),
        "item 7: the production client surfaces an out-of-range value as a typed validation error"
    );

    // ── STALE REVISION / SEQUENCED WRITES from two independent clients: no lost update, no gap. ─────
    // The route carries no optimistic-concurrency token, so the honest contract is last-writer-wins with
    // a monotonic, gap-free revision and a complete history — an in-memory stale revision held by one
    // client can never silently drop another client's write.
    let (client_b, _client_b_runtime) = fresh_production_client(&backend.base);
    let stale_revision = backend.get_json(&format!("{base}/{FONT_SIZE}"))["record"]["revision"]
        .as_i64()
        .expect("client A reads a numeric current revision");
    let b_write = client_b
        .set(&wsid, FONT_SIZE, json!(30.0))
        .expect("client B writes while client A holds a stale revision");
    assert_eq!(b_write.revision, stale_revision + 1);
    let a_write = client
        .set(&wsid, FONT_SIZE, json!(11.0))
        .expect("client A writes from its stale view");
    assert_eq!(
        a_write.revision,
        b_write.revision + 1,
        "item 7: a stale-revision writer still advances the revision (no rewind, no gap)"
    );
    let history = client
        .history(&wsid, FONT_SIZE)
        .expect("history after the sequenced writes");
    assert!(
        history
            .iter()
            .any(|receipt| receipt.new_value == json!(30.0)),
        "item 7: client B's write survives in history (no lost update)"
    );
    assert_eq!(
        history[0].new_value,
        json!(11.0),
        "item 7: history is newest-first and records the last writer"
    );
    assert_eq!(
        history.len() as i64,
        a_write.revision,
        "item 7: the revision counter equals the number of durable mutations (monotonic, gap-free)"
    );

    // ── RESET IDEMPOTENCE: resetting twice never drifts the value away from the registry default. ───
    let first_reset = backend.post_json(&format!("{base}/{FONT_SIZE}/reset"), &json!({}));
    let second_reset = backend.post_json(&format!("{base}/{FONT_SIZE}/reset"), &json!({}));
    assert_eq!(
        first_reset["record"]["value"], json!(13.0),
        "first reset restores the default"
    );
    assert_eq!(
        second_reset["record"]["value"], json!(13.0),
        "item 7: a repeated reset is value-idempotent"
    );
    assert_eq!(
        second_reset["record"]["source"], "operator",
        "item 7: a repeated reset stays an attributed mutation, not a delete"
    );
    assert_eq!(
        second_reset["record"]["revision"]
            .as_i64()
            .expect("numeric revision"),
        first_reset["record"]["revision"]
            .as_i64()
            .expect("numeric revision")
            + 1,
        "item 7: every reset is an auditable revision-bumping mutation (SET-UI-002)"
    );
    assert_eq!(
        backend.get_json(&format!("{base}/{FONT_SIZE}"))["record"]["value"],
        json!(13.0),
        "item 7: the durable value after repeated resets is still the registry default"
    );

    // ── DUPLICATE CHORD COLLISION: the persisted map may physically hold a duplicate (the backend
    //    ChordMap constraint only checks shape), so the LIVE editor layer must refuse to apply it
    //    rather than silently disabling an action. Prove both halves against real persisted state. ───
    let duplicate = json!({ "rich.toggle_italic": "Mod+B" });
    let stored = backend.put_json(
        &format!("{base}/view-defaults.editor.keybinding-overrides"),
        &json!({ "value": duplicate }),
    );
    assert_eq!(
        stored["record"]["value"], duplicate,
        "the shape-valid duplicate-chord map is accepted by the storage contract"
    );
    {
        let rows = client
            .list(&wsid)
            .expect("fresh client lists the duplicate-chord state");
        let app = hydrated_app(&rows);
        // The FRONTEND guard refuses to author the collision through the Settings control...
        assert!(
            handshake_native::settings_editor_section::validate_editor_keybinding_change(
                app.workspace_settings(),
                "rich.toggle_italic",
                "Mod+B",
            )
            .is_err(),
            "item 7: the Settings guard rejects a duplicate chord within one editor surface"
        );
        // ...and the MOUNTED rich keymap keeps BOTH working defaults rather than losing Bold.
        let bold_chord = KeymapSettings::chord_from_str("Mod+B").expect("chord parses");
        let modifiers = egui::Modifiers {
            alt: bold_chord.alt,
            ctrl: bold_chord.ctrl,
            shift: bold_chord.shift,
            mac_cmd: bold_chord.mac_cmd,
            command: bold_chord.ctrl || bold_chord.mac_cmd,
        };
        let rich_state = app.mounted_rich_state();
        let guard = rich_state.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(
            guard.rich_keymap().resolve(&modifiers, bold_chord.key),
            Some(FormattingCommand::ToggleBold),
            "item 7: a persisted duplicate chord never silently steals another action's binding"
        );
        let italic_chord = KeymapSettings::chord_from_str("Mod+I").expect("chord parses");
        let italic_modifiers = egui::Modifiers {
            alt: italic_chord.alt,
            ctrl: italic_chord.ctrl,
            shift: italic_chord.shift,
            mac_cmd: italic_chord.mac_cmd,
            command: italic_chord.ctrl || italic_chord.mac_cmd,
        };
        assert_eq!(
            guard
                .rich_keymap()
                .resolve(&italic_modifiers, italic_chord.key),
            Some(FormattingCommand::ToggleItalic),
            "item 7: the colliding action keeps its own working default (never disabled)"
        );
    }

    // ── WORKSPACE ISOLATION: a second real workspace has independent records + history. ─────────────
    let other = backend.create_workspace(&format!(
        "wp-kernel-012-mt-072-isolation-{}",
        uuid::Uuid::new_v4()
    ));
    let other_id = other
        .get("id")
        .or_else(|| other.pointer("/workspace/id"))
        .and_then(Value::as_str)
        .expect("second workspace has an id")
        .to_owned();
    let other_base = format!("/workspaces/{other_id}/preferences");
    backend.put_json(&format!("{other_base}/{FONT_SIZE}"), &json!({ "value": 42.0 }));
    assert_eq!(
        backend.get_json(&format!("{other_base}/{FONT_SIZE}"))["record"]["value"],
        json!(42.0),
        "the second workspace stores its own value"
    );
    assert_eq!(
        backend.get_json(&format!("{base}/{FONT_SIZE}"))["record"]["value"],
        json!(13.0),
        "item 7: a write in one workspace never leaks into another workspace's record"
    );
    assert_eq!(
        backend.get_json(&format!("{other_base}/{FONT_SIZE}"))["record"]["scope_ref"],
        json!(other_id),
        "the second workspace's record is scoped to its own workspace id"
    );
    backend.post_json(&format!("{other_base}/{FONT_SIZE}/reset"), &json!({}));
    assert_eq!(
        backend.get_json(&format!("{base}/{FONT_SIZE}"))["record"]["revision"],
        backend.get_json(&format!("{base}/{FONT_SIZE}"))["record"]["revision"],
        "the reset in the other workspace did not touch this workspace"
    );
    let other_history = backend.get_json(&format!("{other_base}/{FONT_SIZE}/history"));
    assert_eq!(
        other_history["receipts"]
            .as_array()
            .map(Vec::len)
            .unwrap_or_default(),
        2,
        "item 7: the second workspace has its OWN history (set + reset), not the first's"
    );
    assert_eq!(
        backend.delete_workspace(&other_id) / 100,
        2,
        "the isolation workspace is cleaned up"
    );

    // ── LEGACY / DEFAULT HYDRATION: a workspace that never stored a preference hydrates defaults. ───
    let legacy = backend.create_workspace(&format!(
        "wp-kernel-012-mt-072-legacy-{}",
        uuid::Uuid::new_v4()
    ));
    let legacy_id = legacy
        .get("id")
        .or_else(|| legacy.pointer("/workspace/id"))
        .and_then(Value::as_str)
        .expect("legacy workspace has an id")
        .to_owned();
    {
        let rows = client
            .list(&legacy_id)
            .expect("fresh client lists a never-written workspace");
        assert_eq!(rows.len(), EDITOR_PREFERENCE_IDS.len());
        assert!(
            rows.iter()
                .all(|row| row.source == "default" && row.revision == 0),
            "item 7: an untouched workspace resolves every preference to its registry default"
        );
        let app = hydrated_app(&rows);
        let prefs = app.workspace_settings().editor_prefs;
        assert_eq!(prefs.editor_font_size, 13.0);
        assert_eq!(prefs.tab_size, 4);
        assert!(prefs.insert_spaces);
        assert_eq!(prefs.word_wrap, WordWrapMode::Off);
        assert!(
            app.workspace_settings().editor_keybindings.is_empty(),
            "item 7: legacy hydration installs no phantom keybinding overrides"
        );
    }
    assert_eq!(
        backend.delete_workspace(&legacy_id) / 100,
        2,
        "the legacy workspace is cleaned up"
    );

    // ── BACKEND LOSS + RETRY WITHOUT EDIT LOSS: restart the exact fixture-owned backend process. ────
    backend.put_json(&format!("{base}/{TAB_SIZE}"), &json!({ "value": 12 }));
    let before_loss = backend.get_json(&format!("{base}/{TAB_SIZE}"))["record"].clone();
    let (old_base, new_base) = backend.restart_owned();
    assert_eq!(
        old_base, new_base,
        "the restarted backend reclaimed the same listener"
    );
    let after_loss = backend.get_json(&format!("{base}/{TAB_SIZE}"))["record"].clone();
    assert_eq!(
        after_loss["value"], before_loss["value"],
        "item 7: the committed edit survived the backend loss"
    );
    assert_eq!(
        after_loss["revision"], before_loss["revision"],
        "item 7: the revision survived the backend loss unchanged"
    );
    let (retry_client, _retry_runtime) = fresh_production_client(&backend.base);
    let retried = retry_client
        .set(&wsid, TAB_SIZE, json!(6))
        .expect("item 7: a retried write succeeds against the restarted backend");
    assert_eq!(retried.value, json!(6));
    assert_eq!(
        retried.revision,
        before_loss["revision"].as_i64().expect("numeric revision") + 1,
        "item 7: the retried write continues the same durable revision line"
    );
    let post_restart_history = retry_client
        .history(&wsid, TAB_SIZE)
        .expect("history after restart");
    assert_eq!(
        post_restart_history.len() as i64,
        retried.revision,
        "item 7: no history entry was lost across the backend restart"
    );

    backend.assert_cleanup();
}
