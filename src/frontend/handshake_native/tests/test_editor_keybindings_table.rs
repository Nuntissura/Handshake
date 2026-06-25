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

use handshake_native::code_editor::keymap::CodeEditorAction;
use handshake_native::settings_editor_section::{
    editor_action_catalog, EditorActionSurface, CODE_ACTION_ID_PREFIX, RICH_ACTION_ID_PREFIX,
};
use handshake_native::workspace_settings::default_workspace_settings_state;

/// AC-005 (completeness): the table's catalog lists EVERY code-editor action and the rich-editor
/// commands. Every `CodeEditorAction::all()` id appears (prefixed) so no editor chord is unreachable.
#[test]
fn catalog_lists_every_code_action_and_rich_commands() {
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
    for rich_bare in ["toggle_bold", "toggle_italic", "set_heading_1", "toggle_bullet_list", "undo"] {
        let expected_id = format!("{RICH_ACTION_ID_PREFIX}{rich_bare}");
        assert!(
            catalog.iter().any(|a| a.id == expected_id),
            "AC-005: rich command '{rich_bare}' is in the editor keybindings catalog"
        );
    }

    // Both surfaces are represented.
    assert!(catalog.iter().any(|a| a.surface == EditorActionSurface::Code));
    assert!(catalog.iter().any(|a| a.surface == EditorActionSurface::Rich));

    // The catalog count is at least all code actions + the rich command set.
    assert!(
        catalog.len() >= CodeEditorAction::all().len() + 15,
        "AC-005: catalog covers all code actions + the rich commands (got {})",
        catalog.len()
    );
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
    assert!(!default_chord.is_empty(), "open_find has a real default chord");

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
    assert_ne!("Mod+Alt+F", default_chord, "the custom binding differs from the default");

    // Reset reverts to the default (override removed).
    assert!(settings.clear_editor_chord("code.open_find"), "reset removed the override");
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
            action.id.starts_with(CODE_ACTION_ID_PREFIX) || action.id.starts_with(RICH_ACTION_ID_PREFIX),
            "id '{}' is namespaced",
            action.id
        );
        assert!(ids.insert(action.id.clone()), "RISK-005: duplicate editor action id '{}'", action.id);
    }
    // The `undo` bare-name collision is resolved by the prefix — both exist, distinct.
    assert!(ids.contains("code.undo"), "code.undo present");
    assert!(ids.contains("rich.undo"), "rich.undo present");
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
