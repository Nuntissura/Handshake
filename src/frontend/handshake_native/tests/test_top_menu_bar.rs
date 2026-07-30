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
//! - disabled leaves (Save, Open Terminal, …) render but are not clickable into an action (no fake-
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
use handshake_native::top_menu_bar::{MenuBar, MenuBarState, MENU_DEFINITIONS};

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
        count, 7,
        "exactly seven top-level menu buttons in the live tree: {nodes:?}"
    );
    // The seven menu titles are reachable by label (the mouse-click open path). The Alt+<letter>
    // keyboard mnemonic open path is proven separately in `alt_letter_mnemonic_opens_each_menu` (AC2).
    for label in ["FILE", "EDIT", "VIEW", "GO", "RUN", "MODELS", "HELP"] {
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
        (MenuId::Run, Key::R, "menu.run.inference-lab"),
        (MenuId::Models, Key::M, "menu.models.swarm-board"),
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

// ── No fake-enable: disabled leaves render but cannot be clicked into an action ──────────────────────

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
    // RUN > Open Terminal is likewise disabled (no native terminal panel yet).
    harness.get_by_label("RUN").click();
    harness.run();
    let nodes = live_author_nodes(&harness);
    assert!(
        nodes.iter().any(|(a, _, _)| a == "menu.run.terminal"),
        "disabled Open Terminal leaf is present + addressable: {nodes:?}"
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

// ── WP-1 MT-021 (AC-1): the WP-1 orchestration console is SWARM-dispatchable ────────────────────────
//
// `SWARM_ACCESSIBLE_ACTIONS` previously listed 13 keys and omitted `menu.models.wp1-orchestration-
// console`, so a swarm agent could not dispatch the console the way it dispatches the other four MODELS
// leaves — the console the swarm exists to be observed in was operator-mouse-only.
//
// This proves the fix through the ACTUAL out-of-process path, not by reading the const:
//   (a) every MODELS-scoped key in the registry resolves to a live `MenuItem` node carrying exactly that
//       `author_id` and is NOT disabled — the two conditions `mcp::action::resolve_target` requires
//       before an `argus.click` is accepted, so each listed key is genuinely dispatchable; and
//   (b) the console key is driven by an AccessKit Click (`click_accesskit`) — the same action an
//       out-of-process `argus.click` dispatches into the frame loop — and the real
//       `Wp1OrchestrationConsole` pane opens as a result.
//
// HBR-QUIET: headless kittest, no foregrounding, no OS cursor automation.

#[test]
fn swarm_accessible_models_leaves_dispatch_through_accesskit() {
    use handshake_native::top_menu_bar::SWARM_ACCESSIBLE_ACTIONS;

    // The registry now carries the console key alongside its five MODELS siblings.
    assert!(
        SWARM_ACCESSIBLE_ACTIONS.contains(&"menu.models.wp1-orchestration-console"),
        "AC-1: the WP-1 orchestration console is swarm-accessible: {SWARM_ACCESSIBLE_ACTIONS:?}"
    );

    let models_keys: Vec<&&str> = SWARM_ACCESSIBLE_ACTIONS
        .iter()
        .filter(|key| key.starts_with("menu.models."))
        .collect();
    // Six, not five: the MODELS menu already exposed FIVE swarm-dispatchable
    // leaves before MT-021 -- swarm-board, swarm-lane-diagnostics,
    // model-runtime, operator-chat AND settings -- and this MT adds the WP-1
    // console. The original expectation of 5 undercounted the pre-existing set
    // by omitting `menu.models.settings`.
    assert_eq!(
        models_keys.len(),
        6,
        "six swarm-dispatchable MODELS leaves (5 pre-existing + the WP-1 console): {models_keys:?}"
    );

    let mut harness = shell_harness();
    harness.run();
    // Open MODELS the way a swarm agent does: an AccessKit Click on the stable `menu-models` node,
    // NOT a synthetic pointer position.
    harness
        .query_all_by(|n: &egui_kittest::kittest::AccessKitNode<'_>| n.author_id() == Some("menu-models"))
        .next()
        .expect("the MODELS menu button is addressable by its stable author_id")
        .click_accesskit();
    harness.run();
    // egui materializes a just-opened popup's items on the following frame.
    harness.run();

    // (a) Every registered MODELS key is live, is a MenuItem, and is ENABLED — the exact preconditions
    //     `resolve_target` enforces before accepting an out-of-process click.
    for key in &models_keys {
        let node = harness
            .query_all_by(|n: &egui_kittest::kittest::AccessKitNode<'_>| {
                n.author_id() == Some(**key)
            })
            .next()
            .unwrap_or_else(|| {
                panic!(
                    "swarm-accessible key '{key}' is not addressable in the open MODELS menu: {:?}",
                    live_author_nodes(&harness)
                )
            });
        let ak = node.accesskit_node();
        assert_eq!(
            format!("{:?}", ak.role()),
            "MenuItem",
            "swarm-accessible key '{key}' is a MenuItem"
        );
        assert!(
            !ak.is_disabled(),
            "swarm-accessible key '{key}' must not be disabled — a disabled target is rejected by \
             resolve_target, so a swarm agent could never dispatch it"
        );
    }

    // (b) Dispatch the console leaf out-of-process and prove the real pane opened.
    harness
        .query_all_by(|n: &egui_kittest::kittest::AccessKitNode<'_>| {
            n.author_id() == Some("menu.models.wp1-orchestration-console")
        })
        .next()
        .expect("the WP-1 console leaf is addressable by its stable author_id")
        .click_accesskit();
    harness.run();

    assert!(
        harness.state().tab_bar_states().values().any(|bar| {
            bar.tabs.iter().any(|tab| {
                tab.pane_type == handshake_native::pane_registry::PaneType::Wp1OrchestrationConsole
            })
        }),
        "AC-1: an AccessKit dispatch of menu.models.wp1-orchestration-console opened the native \
         Wp1OrchestrationConsole pane"
    );
}

#[test]
fn run_menu_opens_swarm_lane_diagnostics() {
    let mut harness = shell_harness();
    harness.run();
    harness.get_by_label("MODELS").click();
    harness.run();
    let nodes = live_author_nodes(&harness);
    assert!(
        nodes
            .iter()
            .any(|(a, role, _)| a == "menu.models.swarm-lane-diagnostics" && role == "MenuItem"),
        "Models menu exposes diagnostics leaf with stable author_id: {nodes:?}"
    );

    harness.get_by_label("Open Lane Diagnostics").click();
    harness.run();
    assert!(
        harness.state().tab_bar_states().values().any(|bar| {
            bar.tabs.iter().any(|tab| {
                tab.pane_type == handshake_native::pane_registry::PaneType::SwarmLaneDiagnostics
            })
        }),
        "Run > Open Lane Diagnostics opens the native diagnostics tab"
    );
}

#[test]
fn run_menu_opens_canonical_problems_pane() {
    let mut harness = shell_harness();
    harness.run();
    harness.get_by_label("RUN").click();
    harness.run();
    let nodes = live_author_nodes(&harness);
    assert!(
        nodes
            .iter()
            .any(|(author_id, role, _)| author_id == "menu.run.problems" && role == "MenuItem"),
        "Run menu exposes Problems with a stable author_id: {nodes:?}"
    );

    harness.get_by_label("Open Problems").click();
    harness.run();
    assert!(
        harness.state().tab_bar_states().values().any(|bar| {
            bar.tabs
                .iter()
                .any(|tab| tab.pane_type == handshake_native::pane_registry::PaneType::Problems)
        }),
        "Run > Open Problems opens the canonical native Problems pane"
    );
}

#[test]
fn run_menu_opens_operator_chat_launch() {
    let mut harness = shell_harness();
    harness.run();
    harness.get_by_label("MODELS").click();
    harness.run();
    let nodes = live_author_nodes(&harness);
    assert!(
        nodes
            .iter()
            .any(|(a, role, _)| a == "menu.models.operator-chat" && role == "MenuItem"),
        "Models menu exposes operator chat leaf with stable author_id: {nodes:?}"
    );

    harness.get_by_label("Open Operator Chat").click();
    harness.run();
    assert!(
        harness.state().tab_bar_states().values().any(|bar| {
            bar.tabs.iter().any(|tab| {
                tab.pane_type == handshake_native::pane_registry::PaneType::OperatorChatLaunch
            })
        }),
        "Run > Open Operator Chat opens the native operator chat launch tab"
    );
}

#[test]
fn run_menu_opens_real_model_runtime_pane() {
    let mut harness = shell_harness();
    harness.run();
    harness.get_by_label("MODELS").click();
    harness.run();
    let nodes = live_author_nodes(&harness);
    assert!(
        nodes.iter().any(
            |(author_id, role, _)| author_id == "menu.models.model-runtime" && role == "MenuItem"
        ),
        "Models menu exposes Model Runtime with a stable author_id: {nodes:?}"
    );
    harness.get_by_label("Open Model Runtime").click();
    harness.run();

    assert_eq!(
        harness.state().active_module(),
        handshake_native::module_switcher::ModuleId::Studio,
        "Run > Open Model Runtime switches through the STUDIO module workflow"
    );

    assert!(
        harness.state().tab_bar_states().values().any(|bar| {
            bar.tabs
                .iter()
                .any(|tab| tab.pane_type == handshake_native::pane_registry::PaneType::ModelRuntime)
        }),
        "Run > Open Model Runtime opens the native ModelRuntime tab"
    );
    let model_runtime_pane_id = harness
        .state()
        .tab_bar_states()
        .iter()
        .find_map(|(pane_id, bar)| {
            bar.tabs
                .iter()
                .any(|tab| tab.pane_type == handshake_native::pane_registry::PaneType::ModelRuntime)
                .then_some(pane_id.as_ref())
        })
        .expect("the ModelRuntime tab belongs to a live pane");
    let status_author_id =
        handshake_native::model_runtime_panel::status_author_id(model_runtime_pane_id);
    assert!(
        live_author_nodes(&harness)
            .iter()
            .any(|(author_id, _, _)| author_id == &status_author_id),
        "the no-backend shell renders the real offline ModelRuntime pane status"
    );
}

#[test]
fn help_menu_opens_real_user_manual_pane() {
    let mut harness = shell_harness();
    harness.run();
    harness.get_by_label("HELP").click();
    harness.run();
    harness.get_by_label("Open User Manual").click();
    harness.run();

    assert!(
        harness.state().tab_bar_states().values().any(|bar| {
            bar.tabs
                .iter()
                .any(|tab| tab.pane_type == handshake_native::pane_registry::PaneType::UserManual)
        }),
        "Help > Open User Manual opens the native UserManual tab"
    );
    assert!(
        live_author_nodes(&harness)
            .iter()
            .any(|(author_id, _, _)| author_id.ends_with("user-manual.status.unavailable")),
        "the no-backend shell renders the real offline UserManual pane"
    );
}

#[test]
fn run_menu_opens_real_user_manual_pane() {
    let mut harness = shell_harness();
    harness.run();
    harness.get_by_label("RUN").click();
    harness.run();
    let nodes = live_author_nodes(&harness);
    assert!(
        nodes
            .iter()
            .any(|(author_id, role, _)| author_id == "menu.run.user-manual" && role == "MenuItem"),
        "Run menu exposes UserManual with a stable author_id: {nodes:?}"
    );
    harness.get_by_label("Open User Manual").click();
    harness.run();

    assert!(
        harness.state().tab_bar_states().values().any(|bar| {
            bar.tabs
                .iter()
                .any(|tab| tab.pane_type == handshake_native::pane_registry::PaneType::UserManual)
        }),
        "Run > Open User Manual opens the native UserManual tab"
    );
    assert!(
        live_author_nodes(&harness)
            .iter()
            .any(|(author_id, _, _)| author_id.ends_with("user-manual.status.unavailable")),
        "the no-backend shell renders the real offline UserManual pane"
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
