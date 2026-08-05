//! WP-KERNEL-012 MT-072 (E12) — editor Keybindings table proofs (PT-003 / AC-005).
//!
//! The Keybindings settings section is EXTENDED in place (not a 2nd section — RISK-005) to list ALL
//! editor-specific actions: the MT-010 code-editor chords (from `CodeEditorAction::all`) AND the
//! rich-editor formatting commands. A custom binding overrides the built-in default for that action,
//! resolved as "custom if present in the editor_keybindings list else the built-in default". These
//! proofs assert:
//!
//! - AC-005 (completeness): the catalog the table renders includes EVERY code-editor action AND the
//!   rich-editor commands, sourced from the editor action catalogs (not a hand-listed subset).
//! - AC-005 (override semantics): a custom binding overrides the default for that action, and resetting
//!   reverts to the default.
//! - RISK-005: there is exactly ONE keybindings store extension point — the editor catalog ids are
//!   prefix-namespaced (code./rich.) and persist into the SEPARATE editor_keybindings list, never the
//!   WP-011 app keybindings map.

use std::sync::{Arc, Mutex};

use egui_kittest::Harness;
use handshake_native::app::{HandshakeApp, HealthDisplayState, DEFAULT_PROJECT_ID};
use handshake_native::backend_client::HealthInfo;
use handshake_native::code_editor::keymap::{CodeEditorAction, KeyChord};
use handshake_native::code_editor::keymap_settings::KeymapSettings;
use handshake_native::pane_registry::{
    DirtyState, LockState, PaneAuthority, PaneId, PaneRecord, PaneType,
};
use handshake_native::preference_client::{
    PreferenceProjectionRow, PREF_EDITOR_KEYBINDING_OVERRIDES,
};
use handshake_native::rich_editor::document_model::history::UndoManager;
use handshake_native::rich_editor::document_model::node::{BlockNode, Mark};
use handshake_native::rich_editor::document_model::position::DocPosition;
use handshake_native::rich_editor::document_model::selection::Selection;
use handshake_native::rich_editor::formatting::FormattingCommand;
use handshake_native::rich_editor::renderer::rich_editor_widget::RichEditorState;
use handshake_native::settings_dialog::SettingsOutcome;
use handshake_native::settings_editor_section::{
    editor_action_catalog, EditorActionSurface, CODE_ACTION_ID_PREFIX, RICH_ACTION_ID_PREFIX,
};
use handshake_native::workspace_settings::default_workspace_settings_state;

/// AC-005 (completeness): the table's catalog lists EVERY code-editor action and the rich-editor
/// commands. Every `CodeEditorAction::all()` id appears (prefixed) so no editor chord is unreachable.
#[test]
fn editor_keybindings_table_lists_every_code_action_and_rich_commands() {
    let catalog = editor_action_catalog();

    // Every code-editor action from the live catalog is present (prefixed) — none dropped.
    for action in CodeEditorAction::all() {
        let expected_id = format!("{CODE_ACTION_ID_PREFIX}{}", action.name());
        assert!(
            catalog.iter().any(|a| a.id == expected_id),
            "AC-005: code action '{}' is in the editor keybindings catalog",
            action.name()
        );
    }

    // Rich-editor commands are present (a representative spread — the table groups them under Rich).
    for rich_bare in [
        "toggle_bold",
        "toggle_italic",
        "set_heading_1",
        "toggle_bullet_list",
        "undo",
    ] {
        let expected_id = format!("{RICH_ACTION_ID_PREFIX}{rich_bare}");
        assert!(
            catalog.iter().any(|a| a.id == expected_id),
            "AC-005: rich command '{rich_bare}' is in the editor keybindings catalog"
        );
    }

    // Both surfaces are represented.
    assert!(catalog
        .iter()
        .any(|a| a.surface == EditorActionSurface::Code));
    assert!(catalog
        .iter()
        .any(|a| a.surface == EditorActionSurface::Rich));

    // The catalog count is at least all code actions + the rich command set.
    assert!(
        catalog.len() >= CodeEditorAction::all().len() + 15,
        "AC-005: catalog covers all code actions + the rich commands (got {})",
        catalog.len()
    );
}

#[test]
fn catalog_pins_code_folding_actions_and_default_chords() {
    let catalog = editor_action_catalog();
    for (id, label, default_chord) in [
        (
            "code.fold_at_cursor",
            "Fold region at cursor",
            "Ctrl+Shift+[",
        ),
        (
            "code.unfold_at_cursor",
            "Unfold region at cursor",
            "Ctrl+Shift+]",
        ),
        ("code.fold_all", "Fold all regions", "Ctrl+K Ctrl+0"),
        ("code.unfold_all", "Unfold all regions", "Ctrl+K Ctrl+J"),
    ] {
        let action = catalog
            .iter()
            .find(|action| action.id == id)
            .unwrap_or_else(|| panic!("folding action {id} is present in the settings catalog"));
        assert_eq!(action.label, label, "settings label for {id}");
        assert_eq!(
            action.default_chord, default_chord,
            "settings default chord for {id}"
        );
        assert_eq!(
            action.surface,
            EditorActionSurface::Code,
            "folding action {id} belongs to the code-editor settings surface"
        );
    }
}

/// AC-005 (override semantics): a custom binding overrides the default for that action; resetting
/// reverts. Resolution is "custom if present else default" against the SEPARATE editor_keybindings list.
#[test]
fn custom_binding_overrides_default_and_reset_reverts() {
    let catalog = editor_action_catalog();
    let find = catalog
        .iter()
        .find(|a| a.id == "code.open_find")
        .expect("code.open_find is in the catalog");
    let default_chord = find.default_chord.clone();
    assert!(
        !default_chord.is_empty(),
        "open_find has a real default chord"
    );

    let mut settings = default_workspace_settings_state();

    // No override yet => the resolved binding is the default (override returns None).
    assert_eq!(
        settings.editor_chord_override("code.open_find"),
        None,
        "with no override, the action uses its built-in default"
    );

    // A custom binding overrides the default for that action.
    settings.set_editor_chord("code.open_find", "Mod+Alt+F".to_owned());
    assert_eq!(
        settings.editor_chord_override("code.open_find"),
        Some("Mod+Alt+F"),
        "AC-005: a custom binding overrides the default for that action"
    );
    // Resolution "custom if present else default": the override is distinct from the default.
    assert_ne!(
        "Mod+Alt+F", default_chord,
        "the custom binding differs from the default"
    );

    // Reset reverts to the default (override removed).
    assert!(
        settings.clear_editor_chord("code.open_find"),
        "reset removed the override"
    );
    assert_eq!(
        settings.editor_chord_override("code.open_find"),
        None,
        "AC-005: resetting reverts the action to its built-in default"
    );
}

/// RISK-005: editor action ids are namespaced (code./rich.) and unique, so a code action and a rich
/// command sharing a bare name (e.g. `undo`) never collide in the ONE editor_keybindings store.
#[test]
fn editor_action_ids_are_namespaced_and_unique() {
    let catalog = editor_action_catalog();
    let mut ids = std::collections::HashSet::new();
    for action in &catalog {
        assert!(
            action.id.starts_with(CODE_ACTION_ID_PREFIX)
                || action.id.starts_with(RICH_ACTION_ID_PREFIX),
            "id '{}' is namespaced",
            action.id
        );
        assert!(
            ids.insert(action.id.clone()),
            "RISK-005: duplicate editor action id '{}'",
            action.id
        );
    }
    // The `undo` bare-name collision is resolved by the prefix — both exist, distinct.
    assert!(ids.contains("code.undo"), "code.undo present");
    assert!(ids.contains("rich.undo"), "rich.undo present");
}

/// AC-005 (LIVE side): a CODE keybinding override rebinds the running MT-079 code-editor keymap, not
/// only the Settings table. Driving an `EditorKeybindingChanged` for `code.open_find` to a chord that is
/// UNBOUND by default makes the live panel's keymap resolve that chord to `OpenFind` in the same frame;
/// resetting reverts it. Proven against the live `CodeEditorPanel::keymap()`, not the persisted struct.
///
/// The RICH side is proven separately (and more strongly — through the real mounted input route rather
/// than only the resolved keymap) by [`rich_keybinding_override_dispatches_through_the_mounted_input_route`]
/// below. The previous "rich-editor overrides have no live keymap seam" note recorded in this file is
/// obsolete: `RichEditorState::rich_keymap` is now the map the mounted widget's per-frame
/// `apply_frame_input` decode reads, and `HandshakeApp::sync_editor_keymap_to_panel` reloads it from the
/// persisted `view-defaults.editor.keybinding-overrides` record.
#[test]
fn code_keybinding_override_rebinds_the_live_panel_keymap() {
    let app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }));
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.state_mut().open_settings();
    harness.run();

    // A chord that is NOT bound to anything in the default VS Code table (so a default-keymap hit can't
    // masquerade as the override taking effect).
    let override_chord_str = "Mod+Alt+J";
    let chord = KeymapSettings::chord_from_str(override_chord_str).expect("parseable test chord");

    // Baseline: the default keymap does NOT resolve this chord to OpenFind (it is unbound).
    let panel0 = harness.state().mounted_code_panel();
    assert_ne!(
        panel0.keymap().resolve(chord),
        Some(CodeEditorAction::OpenFind),
        "baseline: {override_chord_str} is not the default OpenFind binding"
    );

    // Apply the override through the same wired outcome the live control produces.
    harness
        .state_mut()
        .apply_settings_outcome_for_test(SettingsOutcome::EditorKeybindingChanged {
            action_id: "code.open_find".to_owned(),
            chord: override_chord_str.to_owned(),
        });
    harness.run();

    // LIVE EFFECT: the running panel's keymap now resolves the override chord to OpenFind.
    let panel = harness.state().mounted_code_panel();
    assert_eq!(
        panel.keymap().resolve(chord),
        Some(CodeEditorAction::OpenFind),
        "AC-005 (live): the custom chord rebinds OpenFind on the running code-editor keymap"
    );

    // Reset reverts the live keymap: the override chord no longer resolves to OpenFind.
    harness
        .state_mut()
        .apply_settings_outcome_for_test(SettingsOutcome::EditorKeybindingReset {
            action_id: "code.open_find".to_owned(),
        });
    harness.run();
    let panel = harness.state().mounted_code_panel();
    assert_ne!(
        panel.keymap().resolve(chord),
        Some(CodeEditorAction::OpenFind),
        "AC-005 (live): resetting reverts the live keymap to the default binding"
    );
}

/// The default chord column is sourced from the SAME default keymap the live editor uses (honest
/// default), not a re-typed guess: open_find's default is the VS Code default for that action.
#[test]
fn default_chords_come_from_the_live_default_keymap() {
    use handshake_native::code_editor::Keymap;
    let catalog = editor_action_catalog();
    let default_keymap = Keymap::default_vscode();

    let find_default = default_keymap
        .bindings_for_action(CodeEditorAction::OpenFind)
        .first()
        .map(|b| {
            handshake_native::code_editor::keymap_settings::KeymapSettings::chord_to_str(&b.chord)
        })
        .expect("open_find has a default binding");

    let catalog_find = catalog.iter().find(|a| a.id == "code.open_find").unwrap();
    assert_eq!(
        catalog_find.default_chord, find_default,
        "the catalog's default chord for open_find matches the live default keymap"
    );
}

// ── MT-072 remediation item 5 (FAIL_V4): MOUNTED RICH-EDITOR custom-chord dispatch ───────────────────
//
// The V4 validator's root failure was that no proof drove a persisted CUSTOM chord through the real
// mounted rich editor: the live override test above exercises `CodeEditorPanel::keymap` only. These
// proofs close that gap end-to-end against the running `HandshakeApp`:
//
//   * a non-default chord is persisted through the SAME wired outcome the Settings control produces,
//   * a FRESH app/client hydrates that persisted override through the production preference-hydration
//     body (`HandshakeApp::apply_preference_hydration`, the same code the drained GET delivery runs),
//   * the chord is sent as a REAL key event into the harness, decoded by the mounted widget's per-frame
//     `apply_frame_input` -> `decode_formatting_commands_with_keymap(&state.rich_keymap)`,
//   * the EXACT formatting effect is asserted on the mounted document (the Bold mark), together with the
//     attributable `TransactionReceipt` (actor id + forward/inverse steps) it recorded,
//   * the OLD default chord is proven inert while the override is active, and
//   * reset removes ONLY that override and the built-in default works again.

/// A live shell whose `pane-b` slot hosts the mounted Notes/rich editor (the MT-079 mount the real app
/// renders). The runtime is returned so it OUTLIVES the harness — a dropped runtime unbinds the editors
/// mid-test. Mirrors the proven `test_app_host_mount::editor_shell` construction.
fn rich_editor_shell() -> (HandshakeApp, tokio::runtime::Runtime) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("build multi-thread runtime");
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }));
    app.set_runtime_handle(runtime.handle().clone());
    {
        let registry = app.pane_registry();
        let mut guard = registry.lock().unwrap_or_else(|e| e.into_inner());
        guard.insert(PaneRecord::new(
            PaneId::from("pane-b"),
            PaneType::LoomWikiPage,
            DEFAULT_PROJECT_ID,
            None,
            LockState::Unlocked,
            DirtyState::Clean,
            PaneAuthority::System,
        ));
    }
    (app, runtime)
}

/// The exact `egui::Modifiers` a live keyboard produces for `chord` (the inverse of
/// `KeyChord::from_modifiers`, which the mounted decode calls on every real key event).
fn modifiers_for(chord: &KeyChord) -> egui::Modifiers {
    egui::Modifiers {
        alt: chord.alt,
        ctrl: chord.ctrl,
        shift: chord.shift,
        mac_cmd: chord.mac_cmd,
        command: chord.ctrl || chord.mac_cmd,
    }
}

/// A `view-defaults.editor.keybinding-overrides` projection row carrying `overrides`, shaped exactly
/// like the row the backend returns from `GET /workspaces/:id/preferences`.
fn keybinding_overrides_projection_row(
    overrides: &[(String, String)],
) -> Vec<PreferenceProjectionRow> {
    let mut map = serde_json::Map::new();
    for (action_id, chord) in overrides {
        map.insert(action_id.clone(), serde_json::Value::String(chord.clone()));
    }
    vec![PreferenceProjectionRow {
        preference_id: PREF_EDITOR_KEYBINDING_OVERRIDES.to_owned(),
        value: serde_json::Value::Object(map),
        default_value: serde_json::json!({}),
        source: "operator".to_owned(),
        revision: 1,
    }]
}

/// Seed the mounted rich document with one paragraph whose whole text leaf is selected, an empty undo
/// history, and a named actor, then request editor focus so the next rendered frame runs the real input
/// path.
fn seed_rich_document(state: &Arc<Mutex<RichEditorState>>, actor_id: &str) {
    let mut guard = state.lock().unwrap_or_else(|e| e.into_inner());
    guard.doc = BlockNode::doc(vec![BlockNode::paragraph("hello")]);
    guard.selection = Selection::text(
        DocPosition::new(vec![0, 0], 0),
        DocPosition::new(vec![0, 0], 5),
    );
    guard.undo = UndoManager::new();
    guard.actor_id = actor_id.to_owned();
    guard.request_editor_focus();
}

/// Whether the mounted document's first text leaf currently carries the Bold mark.
fn mounted_text_is_bold(state: &Arc<Mutex<RichEditorState>>) -> bool {
    let guard = state.lock().unwrap_or_else(|e| e.into_inner());
    guard.doc.children[0]
        .as_block()
        .expect("seeded doc has a paragraph block")
        .children[0]
        .as_text()
        .expect("seeded paragraph has a text leaf")
        .has_mark_type(&Mark::Bold)
}

/// Number of recorded transactions on the mounted document (one per applied formatting command).
fn mounted_undo_len(state: &Arc<Mutex<RichEditorState>>) -> usize {
    state.lock().unwrap_or_else(|e| e.into_inner()).undo.len()
}

/// The chord the rich `toggle_bold` command is bound to by default, straight from the shipped catalog
/// (so this proof cannot drift away from the real default).
fn rich_bold_default_chord() -> String {
    editor_action_catalog()
        .into_iter()
        .find(|action| action.id == "rich.toggle_bold")
        .expect("rich.toggle_bold is in the editor keybindings catalog")
        .default_chord
}

/// MT-072 remediation item 5 — the mounted rich editor consumes a PERSISTED CUSTOM chord through the
/// real input/command route, the superseded default goes inert, and reset restores the default.
#[test]
fn rich_keybinding_override_dispatches_through_the_mounted_input_route() {
    const OVERRIDE_CHORD: &str = "Mod+Alt+B";
    const ACTOR: &str = "wp-kernel-012-mt-072-rich-override";

    let default_chord_text = rich_bold_default_chord();
    assert_eq!(
        default_chord_text, "Mod+B",
        "the shipped rich Bold default is the chord this proof suppresses"
    );
    let default_chord =
        KeymapSettings::chord_from_str(&default_chord_text).expect("default chord parses");
    let custom_chord = KeymapSettings::chord_from_str(OVERRIDE_CHORD).expect("custom chord parses");
    assert_ne!(
        default_chord, custom_chord,
        "the override chord must differ from the default it replaces"
    );

    let (app, _runtime) = rich_editor_shell();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(3);
    let rich = harness.state().mounted_rich_state();

    // ── (1) BASELINE: the BUILT-IN default chord bolds through the real mounted input route. ────────
    seed_rich_document(&rich, ACTOR);
    harness.run_steps(2);
    assert!(
        !mounted_text_is_bold(&rich),
        "baseline: the seeded document starts unbolded"
    );
    harness.key_press_modifiers(modifiers_for(&default_chord), default_chord.key);
    harness.run_steps(2);
    assert!(
        mounted_text_is_bold(&rich),
        "baseline: the built-in {default_chord_text} chord toggles Bold on the MOUNTED rich editor \
         through the real per-frame input decode"
    );

    // ── (2) Persist the CUSTOM chord through the same wired outcome the Settings control produces. ──
    assert!(
        harness
            .state_mut()
            .apply_settings_outcome_for_test(SettingsOutcome::EditorKeybindingChanged {
                action_id: "rich.toggle_bold".to_owned(),
                chord: OVERRIDE_CHORD.to_owned(),
            }),
        "the wired keybinding-change outcome is accepted (no validation rejection)"
    );
    harness.run_steps(2);
    assert_eq!(
        harness
            .state()
            .workspace_settings()
            .editor_chord_override("rich.toggle_bold"),
        Some(OVERRIDE_CHORD),
        "the override is persisted in the canonical editor keybinding-override store"
    );

    // ── (3) FRESH panel/client hydration: a brand-new app + mounted rich editor that has never seen a
    //        settings control loads the SAME persisted override map through the PRODUCTION hydration
    //        body (`apply_preference_hydration`), exactly as a reopened client does after its GET. ───
    let persisted_overrides: Vec<(String, String)> = harness
        .state()
        .workspace_settings()
        .editor_keybindings
        .iter()
        .map(|binding| (binding.action_id.clone(), binding.chord.clone()))
        .collect();
    assert!(
        persisted_overrides
            .iter()
            .any(|(action_id, chord)| action_id == "rich.toggle_bold" && chord == OVERRIDE_CHORD),
        "the persisted override map carries the rich override a fresh client would hydrate"
    );

    let (fresh_app, _fresh_runtime) = rich_editor_shell();
    let mut fresh = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), fresh_app);
    fresh.run_steps(3);
    fresh
        .state_mut()
        .hydrate_editor_preferences_for_test(&keybinding_overrides_projection_row(
            &persisted_overrides,
        ));
    fresh.run_steps(2);
    let fresh_rich = fresh.state().mounted_rich_state();
    seed_rich_document(&fresh_rich, ACTOR);
    fresh.run_steps(2);

    // (3a) The OLD default is INERT on the freshly hydrated mounted editor while the override is active.
    let undo_before_default = mounted_undo_len(&fresh_rich);
    fresh.key_press_modifiers(modifiers_for(&default_chord), default_chord.key);
    fresh.run_steps(2);
    assert!(
        !mounted_text_is_bold(&fresh_rich),
        "item 5: the superseded default {default_chord_text} must NOT toggle Bold while \
         rich.toggle_bold is overridden to {OVERRIDE_CHORD}"
    );
    assert_eq!(
        mounted_undo_len(&fresh_rich),
        undo_before_default,
        "item 5: the superseded default must record NO transaction on the mounted document"
    );

    // (3b) The CUSTOM chord DOES invoke the command through the real mounted input route.
    fresh.key_press_modifiers(modifiers_for(&custom_chord), custom_chord.key);
    fresh.run_steps(2);
    assert!(
        mounted_text_is_bold(&fresh_rich),
        "item 5: the persisted custom chord {OVERRIDE_CHORD} toggles Bold on the freshly hydrated \
         MOUNTED rich editor through the real input/command route"
    );
    assert_eq!(
        mounted_undo_len(&fresh_rich),
        undo_before_default + 1,
        "item 5: exactly ONE transaction was recorded (the chord fired once, not twice)"
    );

    // (3c) ATTRIBUTABLE RECEIPT + trace: the recorded transaction names the actor and the mark step,
    //      and the mounted widget really ran its per-frame input path.
    {
        let guard = fresh_rich.lock().unwrap_or_else(|e| e.into_inner());
        let receipt = guard
            .undo
            .last_receipt()
            .expect("the custom-chord dispatch recorded a transaction receipt");
        assert_eq!(
            receipt.actor_id, ACTOR,
            "item 5: the receipt attributes the mutation to the dispatching actor"
        );
        assert!(
            !receipt.forward.is_empty() && !receipt.inverse.is_empty(),
            "item 5: the receipt carries recoverable forward + inverse steps: {receipt:?}"
        );
        assert!(
            format!("{:?}", receipt.forward).contains("Bold"),
            "item 5: the receipt's forward steps record the Bold mark the chord applied: {:?}",
            receipt.forward
        );
        assert!(
            guard.doc_snapshot_count() > 0,
            "item 5: the mounted rich editor ran its real per-frame input path"
        );
        // The LIVE runtime map (not a settings struct) is what resolved the chord.
        assert_eq!(
            guard
                .rich_keymap()
                .resolve(&modifiers_for(&custom_chord), custom_chord.key),
            Some(FormattingCommand::ToggleBold),
            "item 5: the MOUNTED rich keymap resolves the custom chord to ToggleBold"
        );
        assert_eq!(
            guard
                .rich_keymap()
                .resolve(&modifiers_for(&default_chord), default_chord.key),
            None,
            "item 5: the MOUNTED rich keymap no longer resolves the superseded default"
        );
    }

    // ── (4) RESET removes ONLY the selected override; the built-in default works again. ─────────────
    let sibling_chord =
        KeymapSettings::chord_from_str("Mod+Alt+I").expect("sibling override chord parses");
    assert!(
        fresh
            .state_mut()
            .apply_settings_outcome_for_test(SettingsOutcome::EditorKeybindingChanged {
                action_id: "rich.toggle_italic".to_owned(),
                chord: "Mod+Alt+I".to_owned(),
            }),
        "a second rich override is accepted"
    );
    fresh.run_steps(2);
    assert!(
        fresh
            .state_mut()
            .apply_settings_outcome_for_test(SettingsOutcome::EditorKeybindingReset {
                action_id: "rich.toggle_bold".to_owned(),
            }),
        "resetting the bold override is accepted"
    );
    fresh.run_steps(2);
    assert_eq!(
        fresh
            .state()
            .workspace_settings()
            .editor_chord_override("rich.toggle_bold"),
        None,
        "item 5: reset removed the bold override"
    );
    assert_eq!(
        fresh
            .state()
            .workspace_settings()
            .editor_chord_override("rich.toggle_italic"),
        Some("Mod+Alt+I"),
        "item 5: reset removed ONLY the selected override — the sibling override survives"
    );

    seed_rich_document(&fresh_rich, ACTOR);
    fresh.run_steps(2);
    fresh.key_press_modifiers(modifiers_for(&default_chord), default_chord.key);
    fresh.run_steps(2);
    assert!(
        mounted_text_is_bold(&fresh_rich),
        "item 5: after reset the built-in {default_chord_text} default toggles Bold on the mounted \
         rich editor again"
    );
    {
        let guard = fresh_rich.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(
            guard
                .rich_keymap()
                .resolve(&modifiers_for(&custom_chord), custom_chord.key),
            None,
            "item 5: the reset chord is no longer bound on the mounted rich keymap"
        );
        assert_eq!(
            guard
                .rich_keymap()
                .resolve(&modifiers_for(&sibling_chord), sibling_chord.key),
            Some(FormattingCommand::ToggleItalic),
            "item 5: the untouched sibling override is still live on the mounted rich keymap"
        );
    }
}

/// MT-072 remediation item 5 (coverage half): EVERY rich formatting command the settings catalog lists
/// is reboundable on the MOUNTED rich keymap seam, and a rebind never silently drops a command. The
/// dispatch proof above covers the real input route for a representative command; this covers the set.
#[test]
fn every_rich_catalog_command_is_reboundable_on_the_mounted_keymap() {
    use handshake_native::rich_editor::formatting::RichKeymap;

    let rich_actions: Vec<String> = editor_action_catalog()
        .into_iter()
        .filter(|action| action.surface == EditorActionSurface::Rich)
        .map(|action| action.id)
        .collect();
    assert!(
        rich_actions.len() >= 30,
        "the rich catalog covers the full formatting command set (got {})",
        rich_actions.len()
    );

    // Each command rebinds to its OWN otherwise-unbound chord (`Mod+Alt+Shift+<key>` is unbound by every
    // rich default), so the whole set applies at once without a collision masking a dropped command.
    let unbound_keys: Vec<String> = ('A'..='Z')
        .map(|letter| letter.to_string())
        .chain((1..=12).map(|n| format!("F{n}")))
        .collect();
    assert!(
        unbound_keys.len() >= rich_actions.len(),
        "the proof needs one distinct unbound chord per rich command"
    );
    let chords: Vec<(String, String)> = rich_actions
        .iter()
        .enumerate()
        .map(|(index, action_id)| {
            let bare = action_id
                .strip_prefix(RICH_ACTION_ID_PREFIX)
                .expect("rich action id is namespaced")
                .to_owned();
            (bare, format!("Mod+Alt+Shift+{}", unbound_keys[index]))
        })
        .collect();
    let (keymap, errors) = RichKeymap::from_overrides(
        chords
            .iter()
            .map(|(bare, chord)| (bare.as_str(), chord.as_str())),
    );
    assert!(
        errors.is_empty(),
        "every rich catalog command must be reboundable through the mounted keymap seam: {errors:?}"
    );
    for (bare, chord_text) in &chords {
        assert!(
            keymap.is_overridden(bare.as_str()),
            "rich command '{bare}' did not take its override chord '{chord_text}'"
        );
    }
}
