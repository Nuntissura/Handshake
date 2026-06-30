//! WP-KERNEL-011 MT-016 — Command Palette overlay, end-to-end through the REAL `HandshakeApp`.
//!
//! These tests drive the actual shell (not only the `command_palette` / `command_registry` module unit
//! tests) to prove the MT-016 contract behavior with real kittest input — the same out-of-process path a
//! swarm agent uses, not synthetic state pokes:
//!
//! - AC1/AC10: opening via the flag (the GO-menu / chord seam) renders a centred always-on-top overlay
//!   with a Dialog root, a SearchBox/TextInput, and a ListBox in the live AccessKit tree (AC11);
//! - AC2: the search input receives focus automatically on open;
//! - AC3: typing "manual" filters the list to the two UserManual rows;
//! - AC4: Enter on the (default) selection runs the command and closes the palette — proven by the
//!   active pane gaining the UserManual tab AND the open flag clearing;
//! - AC5: Escape closes the palette without running a command;
//! - AC6: the Close button closes the palette;
//! - AC8: running `usermanual.open` navigates the active pane to the user-manual tab.
//!
//! ## No live backend needed
//!
//! The shell is built with `HandshakeApp::with_health(...)` (no runtime spawn, no network).

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;
use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend_client::HealthInfo;
use handshake_native::command_palette::{
    PALETTE_DIALOG_AUTHOR_ID, PALETTE_LIST_AUTHOR_ID, PALETTE_SEARCH_AUTHOR_ID,
};
use handshake_native::pane_registry::PaneType;

fn ok_app() -> HandshakeApp {
    HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }))
}

fn shell_harness() -> Harness<'static, HandshakeApp> {
    Harness::builder().build_state(|ctx, a: &mut HandshakeApp| a.ui(ctx), ok_app())
}

/// The palette's search box, disambiguated from the always-visible bottom search rail and the mounted
/// code editor, which are also `Role::TextInput` nodes in the default shell.
fn palette_search<'h>(harness: &'h Harness<'_, HandshakeApp>) -> egui_kittest::Node<'h> {
    harness
        .query_all_by_role(egui::accesskit::Role::TextInput)
        .find(|n| n.accesskit_node().author_id() == Some(PALETTE_SEARCH_AUTHOR_ID))
        .expect("the palette search TextInput (the non-rail one)")
}

/// Collect every live AccessKit node carrying an author_id: (author_id, role, label).
fn live_author_nodes(harness: &Harness<'_, HandshakeApp>) -> Vec<(String, String, Option<String>)> {
    let mut found = Vec::new();
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        if let Some(author_id) = ak.author_id() {
            found.push((author_id.to_owned(), format!("{:?}", ak.role()), ak.label()));
        }
    }
    found
}

/// True if any pane's tab bar currently has a UserManual tab open (the result of `usermanual.open`).
fn any_pane_has_usermanual_tab(app: &HandshakeApp) -> bool {
    app.tab_bar_states()
        .values()
        .any(|bar| bar.tabs.iter().any(|t| t.pane_type == PaneType::UserManual))
}

// ── AC1 / AC10 / AC11: opening via the flag renders the Dialog/SearchBox/ListBox in the live tree ─────

#[test]
fn opening_palette_renders_dialog_searchbox_listbox() {
    let mut harness = shell_harness();
    harness.run();
    // Closed initially: none of the palette container nodes are in the live tree.
    let before = live_author_nodes(&harness);
    assert!(
        !before.iter().any(|(a, _, _)| a == PALETTE_DIALOG_AUTHOR_ID),
        "palette dialog absent while closed: {before:?}"
    );

    // Open via the public seam (the same call the GO menu + the Ctrl+Shift+P chord route through).
    harness.state_mut().open_command_palette();
    harness.run();
    // The Window's text edit requests focus on the first open frame; run once more so it settles.
    harness.run();

    let nodes = live_author_nodes(&harness);
    let dialog = nodes
        .iter()
        .find(|(a, _, _)| a == PALETTE_DIALOG_AUTHOR_ID)
        .unwrap_or_else(|| panic!("palette dialog missing from live tree: {nodes:?}"));
    assert_eq!(dialog.1, "Dialog", "palette root role is Dialog");

    let search = nodes
        .iter()
        .find(|(a, _, _)| a == PALETTE_SEARCH_AUTHOR_ID)
        .unwrap_or_else(|| panic!("palette search box missing: {nodes:?}"));
    assert_eq!(search.1, "TextInput", "palette search role is TextInput");

    let list = nodes
        .iter()
        .find(|(a, _, _)| a == PALETTE_LIST_AUTHOR_ID)
        .unwrap_or_else(|| panic!("palette list missing: {nodes:?}"));
    assert_eq!(list.1, "ListBox", "palette list role is ListBox");

    // The full app command set renders as ListBoxOption rows (AC: list real commands). UserManual: Open
    // is present and addressable.
    assert!(
        nodes.iter().any(
            |(a, r, _)| a == "command-palette.option.hs-usermanual-palette-open"
                && r == "ListBoxOption"
        ),
        "UserManual: Open row present as ListBoxOption: {nodes:?}"
    );
}

// ── AC2: the search input is focused automatically on open ───────────────────────────────────────────

#[test]
fn search_input_is_focused_on_open() {
    let mut harness = shell_harness();
    harness.run();
    harness.state_mut().open_command_palette();
    harness.run();
    harness.run();

    let search = palette_search(&harness);
    // The palette's search box is the only TextInput on screen and it requested focus on open.
    assert!(
        search.is_focused(),
        "the search input has keyboard focus on open"
    );
}

// ── AC3: typing "manual" filters the list to the UserManual commands ─────────────────────────────────

#[test]
fn typing_manual_filters_to_usermanual_rows() {
    let mut harness = shell_harness();
    harness.run();
    harness.state_mut().open_command_palette();
    harness.run();
    harness.run();

    // Type into the focused search box (the genuine keyboard path).
    palette_search(&harness).type_text("manual");
    harness.run();
    harness.run();

    let nodes = live_author_nodes(&harness);
    let rows: Vec<&str> = nodes
        .iter()
        .filter(|(a, _, _)| a.starts_with("command-palette.option."))
        .map(|(_, _, label)| label.as_deref().unwrap_or(""))
        .collect();

    assert!(
        rows.contains(&"UserManual: Open"),
        "UserManual: Open shown for 'manual': {rows:?}"
    );
    assert!(
        rows.contains(&"UserManual: Search"),
        "UserManual: Search shown for 'manual': {rows:?}"
    );
    // A non-matching command is filtered out.
    assert!(
        !rows.contains(&"View: Toggle Theme"),
        "View: Toggle Theme filtered out for 'manual': {rows:?}"
    );
}

// ── AC4 / AC8: Enter runs the selected command (usermanual.open) and closes the palette ───────────────

#[test]
fn enter_runs_selected_command_and_closes() {
    let mut harness = shell_harness();
    harness.run();
    assert!(
        !any_pane_has_usermanual_tab(harness.state()),
        "no UserManual tab before"
    );

    harness.state_mut().open_command_palette();
    harness.run();
    harness.run();

    // Type "manual" so the first filtered row is UserManual: Open (selected_index defaults to 0).
    palette_search(&harness).type_text("manual");
    harness.run();
    harness.run();

    // Press Enter — runs the selected enabled command and closes the palette (AC4).
    harness.key_press(egui::Key::Enter);
    harness.run();
    harness.run();

    assert!(
        !harness.state().command_palette_open(),
        "Enter on a command closed the palette"
    );
    // AC8: usermanual.open navigated the active pane to the user-manual tab.
    assert!(
        any_pane_has_usermanual_tab(harness.state()),
        "Enter ran usermanual.open -> a UserManual tab is now open"
    );
}

// ── AC5: Escape closes the palette without running a command ──────────────────────────────────────────

#[test]
fn escape_closes_without_running() {
    let mut harness = shell_harness();
    harness.run();
    harness.state_mut().open_command_palette();
    harness.run();
    harness.run();
    assert!(
        harness.state().command_palette_open(),
        "palette open before Escape"
    );

    harness.key_press(egui::Key::Escape);
    harness.run();
    harness.run();

    assert!(
        !harness.state().command_palette_open(),
        "Escape closed the palette"
    );
    // Nothing ran: no UserManual tab was opened.
    assert!(
        !any_pane_has_usermanual_tab(harness.state()),
        "Escape ran no command"
    );
    // The palette container nodes are gone from the live tree.
    let nodes = live_author_nodes(&harness);
    assert!(
        !nodes.iter().any(|(a, _, _)| a == PALETTE_DIALOG_AUTHOR_ID),
        "palette dialog gone after Escape: {nodes:?}"
    );
}

// ── AC6: the Close button closes the palette ─────────────────────────────────────────────────────────

#[test]
fn close_button_closes_palette() {
    let mut harness = shell_harness();
    harness.run();
    harness.state_mut().open_command_palette();
    harness.run();
    harness.run();

    harness.get_by_label("Close").click();
    harness.run();
    harness.run();

    assert!(
        !harness.state().command_palette_open(),
        "Close button closed the palette"
    );
}

// ── Re-open resets the query (red-team R1/MC1) ───────────────────────────────────────────────────────

#[test]
fn reopen_resets_query() {
    let mut harness = shell_harness();
    harness.run();

    // Open, type "manual", then close.
    harness.state_mut().open_command_palette();
    harness.run();
    harness.run();
    palette_search(&harness).type_text("manual");
    harness.run();
    let first_open_count = harness.state().command_palette_open_count();
    harness.key_press(egui::Key::Escape);
    harness.run();
    harness.run();

    // Re-open: the open generation bumps and the query is cleared, so ALL commands show again (a
    // non-UserManual command like View: Toggle Theme is back in the list).
    harness.state_mut().open_command_palette();
    harness.run();
    harness.run();
    assert!(
        harness.state().command_palette_open_count() > first_open_count,
        "re-open bumped the open generation"
    );

    let nodes = live_author_nodes(&harness);
    let rows: Vec<&str> = nodes
        .iter()
        .filter(|(a, _, _)| a.starts_with("command-palette.option."))
        .map(|(_, _, label)| label.as_deref().unwrap_or(""))
        .collect();
    assert!(
        rows.contains(&"View: Toggle Theme"),
        "re-open cleared the stale 'manual' query (all commands visible again): {rows:?}"
    );
}

// ── Disabled (editor) rows cannot be run via Enter (AC7) ──────────────────────────────────────────────

#[test]
fn disabled_editor_row_cannot_run() {
    let mut harness = shell_harness();
    harness.run();
    harness.state_mut().open_command_palette();
    harness.run();
    harness.run();

    // Filter to an editor command ("Bold"), which is disabled (no native editor surface yet).
    palette_search(&harness).type_text("bold");
    harness.run();
    harness.run();

    // The Bold row is present and marked disabled.
    let bold = harness.get_by_label("Bold");
    assert!(
        bold.accesskit_node().is_disabled(),
        "editor Bold row is disabled"
    );

    // Pressing Enter on the disabled selection does NOT run it: the palette stays open (no Run outcome).
    harness.key_press(egui::Key::Enter);
    harness.run();
    harness.run();
    assert!(
        harness.state().command_palette_open(),
        "Enter on a disabled row did not run/close the palette"
    );
}
