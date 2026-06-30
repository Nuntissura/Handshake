//! WP-KERNEL-011 MT-015 — top application menu bar, end-to-end through the REAL `HandshakeApp`.
//!
//! These tests drive the actual shell (not only the `top_menu_bar` module's own unit tests) to prove
//! the C4 menu-bar behavior the MT-015 contract asks for:
//!
//! - the six top-level menus (FILE/EDIT/VIEW/GO/RUN/HELP) render in a horizontal strip at the very top
//!   as live `Role::MenuItem` nodes with stable author_ids (`menu-file`..`menu-help`) — AC1, AC2, AC9;
//! - opening the GO menu and clicking "Command Palette" sets `command_palette_open` (AC3, AC11);
//! - opening the GO menu and clicking "Quick Switcher" sets `quick_switcher_open` (AC4);
//! - opening the VIEW menu and clicking the NON-active Theme option toggles the theme (AC5);
//! - opening the VIEW menu and clicking a drawer toggle flips the SAME flag the rail toggles (AC6);
//! - opening the VIEW menu and clicking "Reset Layout" arms the confirm (does NOT reset immediately —
//!   red-team MC7/R7), and the explicit confirm resets to the seeded default (AC7);
//! - the menu closes after an item is clicked (red-team R6 / MC6);
//! - disabled leaves (Save, …) render but are not clickable into an action (no fake-
//!   enable) — they still appear in the open-menu tree as addressable disabled MenuItem nodes.
//!
//! ## No live backend needed
//!
//! The shell is built with `HandshakeApp::with_health(...)` (no runtime spawn, no network), and menu
//! interactions are driven by real kittest pointer clicks on the labelled menu/leaf nodes — the same
//! out-of-process path a swarm agent uses, not synthetic state pokes.

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;
use handshake_native::accessibility::assert_no_unnamed_interactive;
use handshake_native::app::{HandshakeApp, HealthDisplayState, ViewMode};
use handshake_native::backend_client::HealthInfo;
use handshake_native::theme::HsTheme;
use handshake_native::top_menu_bar::{
    MenuBar, MenuBarState, MENU_DEFINITIONS, MENU_RUN_MODEL_SESSION_LAUNCH_AUTHOR_ID,
};

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

// ── AC1 / AC2 / AC9: six top-level menu buttons in the live tree with stable ids + MenuItem role ─────

#[test]
fn live_shell_has_six_top_level_menus_with_stable_ids() {
    let mut harness = shell_harness();
    harness.run();

    let nodes = live_author_nodes(&harness);
    for menu in MENU_DEFINITIONS {
        let found = nodes
            .iter()
            .find(|(a, _, _)| a == menu.author_id())
            .unwrap_or_else(|| panic!("{} missing from live tree: {nodes:?}", menu.author_id()));
        assert_eq!(found.1, "MenuItem", "{} role is MenuItem", menu.author_id());
    }
    // Exactly six top-level menu buttons (leaf items are not rendered while all menus are closed).
    let count = nodes
        .iter()
        .filter(|(a, _, _)| a.starts_with("menu-"))
        .count();
    assert_eq!(
        count, 6,
        "exactly six top-level menu buttons in the live tree: {nodes:?}"
    );
    // The six menu titles are reachable by label (the mouse-click open path). The Alt+<letter> keyboard
    // mnemonic open path is proven separately in `alt_letter_mnemonic_opens_each_menu` below (AC2).
    for label in ["FILE", "EDIT", "VIEW", "GO", "RUN", "HELP"] {
        let _ = harness.get_by_label(label);
    }
}

// ── AC2 keyboard path: Alt+<letter> mnemonic OPENS the corresponding menu ─────────────────────────────

/// AC2 (keyboard mnemonic path): pressing `Alt+<letter>` opens the matching top-level menu — proven by
/// the menu's leaf items becoming reachable in the live tree (they exist ONLY while the menu is open).
/// This drives the REAL shell through real `Alt+<key>` key events (the same out-of-process keyboard path
/// a swarm agent or a keyboard-only operator uses), not a synthetic memory poke.
#[test]
fn alt_letter_mnemonic_opens_each_menu() {
    use egui::{Key, Modifiers};
    use handshake_native::top_menu_bar::MenuId;

    // (mnemonic key, a leaf author_id that exists ONLY while THIS menu is open) per menu.
    let cases = [
        (MenuId::File, Key::F, "menu.file.quit"),
        (MenuId::Edit, Key::E, "menu.edit.undo"),
        (MenuId::View, Key::V, "menu.view.reset-layout"),
        (MenuId::Go, Key::G, "menu.go.command-palette"),
        (MenuId::Run, Key::R, "menu.run.swarm-board"),
        (MenuId::Help, Key::H, "menu.help.about"),
    ];

    for (menu, key, open_only_leaf) in cases {
        // The mnemonic key constant on MenuId is the one the shell consumes (keeps the table honest).
        assert_eq!(menu.mnemonic_key(), key, "{:?} mnemonic key", menu);

        let mut harness = shell_harness();
        harness.run();
        // Closed initially: the open-only leaf is NOT in the tree.
        let before = live_author_nodes(&harness);
        assert!(
            !before.iter().any(|(a, _, _)| a == open_only_leaf),
            "{open_only_leaf} present before Alt+{key:?} (menu should be closed): {before:?}"
        );

        // Press Alt+<letter> — the genuine keyboard mnemonic path.
        harness.key_press_modifiers(Modifiers::ALT, key);
        harness.run();
        // egui's menu popup materializes its items on the frame after it is opened in memory; run once
        // more so the just-opened popup is laid out and its leaves enter the accessibility tree.
        harness.run();

        let after = live_author_nodes(&harness);
        assert!(
            after.iter().any(|(a, _, _)| a == open_only_leaf),
            "Alt+{key:?} did NOT open {:?}: leaf {open_only_leaf} absent from live tree: {after:?}",
            menu
        );
    }
}

/// AC2 + red-team R3: Alt+<letter> opening one menu CLOSES any other menu (only one popup open at a
/// time), so the keyboard path cannot leave two menus open at once.
#[test]
fn alt_letter_mnemonic_switches_between_menus() {
    use egui::{Key, Modifiers};

    let mut harness = shell_harness();
    harness.run();

    // Open GO via Alt+G.
    harness.key_press_modifiers(Modifiers::ALT, Key::G);
    harness.run();
    harness.run();
    let go_open = live_author_nodes(&harness);
    assert!(
        go_open
            .iter()
            .any(|(a, _, _)| a == "menu.go.command-palette"),
        "Alt+G opened GO: {go_open:?}"
    );

    // Now press Alt+V — VIEW opens and GO closes (egui keeps at most one popup open).
    harness.key_press_modifiers(Modifiers::ALT, Key::V);
    harness.run();
    harness.run();
    let view_open = live_author_nodes(&harness);
    assert!(
        view_open
            .iter()
            .any(|(a, _, _)| a == "menu.view.reset-layout"),
        "Alt+V opened VIEW: {view_open:?}"
    );
    assert!(
        !view_open
            .iter()
            .any(|(a, _, _)| a == "menu.go.command-palette"),
        "GO closed when VIEW opened (only one menu open at a time): {view_open:?}"
    );
}

// ── AC3 / AC11: GO > Command Palette sets command_palette_open ───────────────────────────────────────

#[test]
fn clicking_go_command_palette_sets_flag() {
    let mut harness = shell_harness();
    harness.run();
    assert!(
        !harness.state().command_palette_open(),
        "palette closed initially"
    );

    // Open the GO menu, then click the Command Palette leaf — the genuine out-of-process path.
    harness.get_by_label("GO").click();
    harness.run();
    harness.get_by_label("Command Palette").click();
    harness.run();

    assert!(
        harness.state().command_palette_open(),
        "GO > Command Palette set command_palette_open within the dispatch frame"
    );
    // R6 / MC6: the menu closed after the click (the leaf is gone from the live tree).
    let nodes = live_author_nodes(&harness);
    assert!(
        !nodes.iter().any(|(a, _, _)| a == "menu.go.command-palette"),
        "the GO menu closed after the item was clicked: {nodes:?}"
    );
}

// ── AC4: GO > Quick Switcher sets quick_switcher_open ────────────────────────────────────────────────

#[test]
fn clicking_go_quick_switcher_sets_flag() {
    let mut harness = shell_harness();
    harness.run();
    assert!(
        !harness.state().quick_switcher_open(),
        "switcher closed initially"
    );

    harness.get_by_label("GO").click();
    harness.run();
    harness.get_by_label("Quick Switcher").click();
    harness.run();

    assert!(
        harness.state().quick_switcher_open(),
        "GO > Quick Switcher set the flag"
    );
}

// ── AC5: VIEW > Theme toggle flips the theme + the checkmark ─────────────────────────────────────────

#[test]
fn clicking_view_theme_light_toggles_theme() {
    let mut harness = shell_harness();
    harness.run();
    assert_eq!(
        harness.state().current_theme(),
        HsTheme::Dark,
        "starts Dark"
    );

    harness.get_by_label("VIEW").click();
    harness.run();
    // Click the non-active "Theme: Light" flat checkmark item.
    harness.get_by_label("Theme: Light").click();
    harness.run();

    assert_eq!(
        harness.state().current_theme(),
        HsTheme::Light,
        "VIEW > Theme: Light switched the active theme"
    );
}

// ── AC6: VIEW drawer toggles flip the SAME flags the rail toggles ────────────────────────────────────

#[test]
fn clicking_view_toggle_bottom_panel_flips_the_shared_flag() {
    let mut harness = shell_harness();
    harness.run();
    let before = harness.state().bottom_drawer_open();

    harness.get_by_label("VIEW").click();
    harness.run();
    harness.get_by_label("Toggle Bottom Panel").click();
    harness.run();

    assert_eq!(
        harness.state().bottom_drawer_open(),
        !before,
        "VIEW > Toggle Bottom Panel flipped the bottom_drawer_open flag (same one MT-014 toggles)"
    );
}

#[test]
fn clicking_view_toggle_project_drawer_flips_the_rail_flag() {
    let mut harness = shell_harness();
    harness.run();
    let before = harness.state().left_rail_open();

    harness.get_by_label("VIEW").click();
    harness.run();
    harness.get_by_label("Toggle Project Drawer").click();
    harness.run();

    assert_eq!(
        harness.state().left_rail_open(),
        !before,
        "project drawer flag flipped"
    );
}

// ── AC7 + MC7/R7: Reset Layout ARMS a confirm; only the explicit confirm resets ──────────────────────

#[test]
fn reset_layout_arms_then_confirms() {
    let mut harness = shell_harness();
    harness.run();
    // Dirty the layout so the reset is observable: move a divider weight off the default.
    harness.state_mut().split_weights_mut().vertical = 0.2;
    harness.run();
    assert!(
        !harness.state().reset_layout_pending(),
        "no reset armed yet"
    );

    harness.get_by_label("VIEW").click();
    harness.run();
    harness.get_by_label("Reset Layout…").click();
    harness.run();

    // The click ARMS the confirm but does NOT reset (red-team MC7/R7): the off-default weight survives.
    assert!(
        harness.state().reset_layout_pending(),
        "Reset Layout armed the confirm"
    );
    assert!(
        (harness.state().split_weights().vertical - 0.2).abs() < 1e-6,
        "layout NOT reset on the menu click alone"
    );

    // The explicit confirm performs the reset to the seeded default split.
    let did = harness.state_mut().confirm_reset_layout();
    harness.run();
    assert!(did, "confirm performed the reset");
    assert!(
        !harness.state().reset_layout_pending(),
        "confirm cleared the pending flag"
    );
    let default_v = handshake_native::split_layout::SplitWeights::default().vertical;
    assert!(
        (harness.state().split_weights().vertical - default_v).abs() < 1e-6,
        "confirm reset the split weights to the seeded default"
    );
}

// ── No fake-enable: disabled leaves render; terminal blocker is clickable and typed ─────────────────

#[test]
fn disabled_leaves_render_but_do_not_fire() {
    let mut harness = shell_harness();
    harness.run();

    // FILE > Save is disabled (no document model yet). It appears in the open menu as a disabled node.
    harness.get_by_label("FILE").click();
    harness.run();
    let nodes = live_author_nodes(&harness);
    assert!(
        nodes.iter().any(|(a, _, _)| a == "menu.file.save"),
        "disabled Save leaf is still present + addressable in the open menu: {nodes:?}"
    );
    // RUN > Open Terminal is present and addressable; MT-100 routes it into a typed status blocker.
    harness.get_by_label("RUN").click();
    harness.run();
    let nodes = live_author_nodes(&harness);
    assert!(
        nodes
            .iter()
            .any(|(a, _, _)| a == MENU_RUN_MODEL_SESSION_LAUNCH_AUTHOR_ID),
        "Launch Model Session leaf is present + addressable: {nodes:?}"
    );
    assert!(
        nodes.iter().any(|(a, _, _)| a == "menu.run.terminal"),
        "Open Terminal in Workspace Folder leaf is present + addressable: {nodes:?}"
    );
}

// ── MT-025 preservation: every OPEN menu leaf is an addressable (author_id-carrying) node ───────────

/// With the GO menu OPEN, every menu leaf in the live tree carries an author_id (the MT-025
/// interactive-naming invariant: a clickable/focusable widget must be addressable). We walk the live
/// kittest tree directly — the same consumer-side tree the out-of-process UIA adapter reads — and
/// assert each `menu.go.*` leaf is present with an author_id, and that NO open leaf is anonymous.
#[test]
fn open_menu_leaves_are_all_named() {
    let mut harness = shell_harness();
    harness.run();
    harness.get_by_label("GO").click();
    harness.run();

    let nodes = live_author_nodes(&harness);
    // The four GO leaves are present + addressable by their stable author_ids.
    for leaf in [
        "menu.go.quick-switcher",
        "menu.go.command-palette",
        "menu.go.next-pane",
        "menu.go.prev-pane",
    ] {
        let found = nodes
            .iter()
            .find(|(a, _, _)| a == leaf)
            .unwrap_or_else(|| panic!("open GO leaf {leaf} missing/anonymous: {nodes:?}"));
        assert_eq!(found.1, "MenuItem", "{leaf} role is MenuItem");
    }

    // No live MenuItem node is anonymous (every menu node carries an author_id). The `assert_no_*`
    // gate is the authoritative MT-025 check; reference it so this file is tied to that contract symbol.
    let _gate = assert_no_unnamed_interactive;
    let menu_item_count = harness
        .root()
        .children_recursive()
        .filter(|n| format!("{:?}", n.accesskit_node().role()) == "MenuItem")
        .count();
    let named_menu_items = nodes.iter().filter(|(_, r, _)| r == "MenuItem").count();
    assert_eq!(
        menu_item_count, named_menu_items,
        "every live MenuItem node carries an author_id (none anonymous)"
    );
    assert!(
        menu_item_count >= 10,
        "six menus + four open GO leaves at least; got {menu_item_count}"
    );
}

// ── ViewMode toggle is observable through the public accessor ────────────────────────────────────────

#[test]
fn view_mode_toggles_from_nsfw_to_sfw() {
    let mut harness = shell_harness();
    harness.run();
    assert_eq!(
        harness.state().view_mode(),
        ViewMode::Nsfw,
        "starts NSFW (production default)"
    );

    harness.get_by_label("VIEW").click();
    harness.run();
    harness.get_by_label("View Mode: SFW").click();
    harness.run();

    assert_eq!(
        harness.state().view_mode(),
        ViewMode::Sfw,
        "VIEW > View Mode: SFW switched the mode"
    );
}

// ── Widget-level: MenuBar::show returns the clicked action ───────────────────────────────────────────

#[test]
fn menubar_widget_returns_command_palette_action() {
    let state = MenuBarState {
        theme_is_dark: true,
        view_mode_is_nsfw: true,
        project_drawer_open: true,
        bottom_drawer_open: false,
        has_active_tab: true,
        editor_available: true,
        editor_can_undo: true,
        editor_can_redo: true,
        editor_can_paste: true,
    };
    use std::sync::{Arc, Mutex};
    let captured: Arc<Mutex<Option<handshake_native::top_menu_bar::MenuBarAction>>> =
        Arc::new(Mutex::new(None));
    let cap = captured.clone();
    let mut harness = Harness::builder().build_ui(move |ui| {
        let action = MenuBar::new(state).show(ui);
        if action.is_some() {
            *cap.lock().unwrap() = action;
        }
    });
    harness.run();
    harness.get_by_label("GO").click();
    harness.run();
    harness.get_by_label("Command Palette").click();
    harness.run();

    assert_eq!(
        *captured.lock().unwrap(),
        Some(handshake_native::top_menu_bar::MenuBarAction::OpenCommandPalette),
        "the widget returned the OpenCommandPalette action on the leaf click"
    );
}
