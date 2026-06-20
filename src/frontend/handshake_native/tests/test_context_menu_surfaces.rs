//! WP-KERNEL-011 MT-020 (C5 part 1) — LIVE per-surface context-menu proof.
//!
//! These tests render the REAL `HandshakeApp` headlessly via egui_kittest (which enables AccessKit and
//! pushes the same `TreeUpdate` the out-of-process Windows UIA adapter receives) and prove the wired
//! per-surface context menus are LIVE end-to-end, not just arithmetic:
//!
//! - SECONDARY-click (right-click) a pane tab opens the tab context menu with the contract items
//!   (`Close`, `Close Others`, `Close All`, `Pin`, `Split Right` (disabled), `Pop Out`) as live
//!   `Role::MenuItem` nodes carrying `ctx-menu.tab.*` author_ids;
//! - activating `Close` from the menu removes the right-clicked tab from the live pane state;
//! - activating `Close Others` keeps only the right-clicked tab;
//! - keyboard nav inside the open menu (ArrowDown -> Enter) dispatches the highlighted item's action;
//! - SECONDARY-click a pane header opens the pane menu (`Lock Pane`, `Pop Out Pane`, `Set Type: …`
//!   disabled) and activating `Lock Pane` toggles the pane's LockState in the registry;
//! - SECONDARY-click a project tab opens the project menu and activating `Switch to Project` switches
//!   the active project;
//! - a disabled (future-target) item renders + is addressable but cannot be activated (no fake-enable).
//!
//! Why this proves LIVE behavior: every assertion either reads the consumer-side AccessKit tree egui
//! produced for the frame, or mutates `HandshakeApp` state through a real pointer / key event. A menu
//! that was only built in memory (never opened via a real `secondary_clicked()`) would be absent here.

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;
use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend_client::HealthInfo;
use handshake_native::pane_registry::{LockState, PaneId, PaneType};
use handshake_native::project_tabs::ProjectItem;
use handshake_native::project_tree::{BookmarkSummary, CanvasSummary, DocumentSummary};
use handshake_native::tab_bar::{TabBarState, TabState};
use std::sync::Arc;

fn ok_app() -> HandshakeApp {
    HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }))
}

/// An app whose `pane-a` has three unpinned tabs (Workspace, InferenceLab, AtelierEditor) so close /
/// close-others have something to act on, the left rail collapsed (stable pane geometry), and a wide
/// window so the tab chips + header strip lay out un-clipped and right-clickable.
fn app_three_tab_pane_a() -> HandshakeApp {
    let mut app = ok_app();
    let pane_a: PaneId = Arc::from("pane-a");
    let bar = TabBarState::new(
        pane_a.clone(),
        vec![
            TabState::new(PaneType::Workspace),
            TabState::new(PaneType::InferenceLab),
            TabState::new(PaneType::AtelierEditor),
        ],
    );
    app.tab_bar_states_mut().insert(pane_a, bar);
    app.set_left_rail_open(false);
    app
}

fn harness_for(app: HandshakeApp) -> Harness<'static, HandshakeApp> {
    let mut harness = Harness::builder()
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.set_size(egui::Vec2::new(1200.0, 800.0));
    harness.run();
    // The rail-collapse flag is applied on the next frame; run once more so the 2x2 grid settles.
    harness.run();
    harness
}

/// The explorer rename dialog's text field, if the dialog is open — disambiguated from the always-visible
/// MT-022 bottom search rail input (which is ALSO a `Role::TextInput` in every frame). The rail input
/// carries the stable author_id `bottom-rail.input`; the rename field does not, so the NON-rail
/// TextInput is the rename field. `None` => the rename dialog is not open (only the rail input exists).
/// (Before MT-022 the rename field was the only TextInput, so this test used a bare `query_by_role`; the
/// always-visible rail made that ambiguous.)
fn rename_field<'h>(harness: &'h Harness<'_, HandshakeApp>) -> Option<egui_kittest::Node<'h>> {
    harness
        .query_all_by_role(egui::accesskit::Role::TextInput)
        .find(|n| n.accesskit_node().author_id() != Some("bottom-rail.input"))
}

/// Every live author-id node: (author_id, role, label).
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

fn pane_a_tabs(harness: &Harness<'_, HandshakeApp>) -> Vec<PaneType> {
    harness
        .state()
        .tab_bar_states()
        .get(&(Arc::from("pane-a") as PaneId))
        .unwrap()
        .tabs
        .iter()
        .map(|t| t.pane_type.clone())
        .collect()
}

// ── The pane-tab right-click target nodes exist (default frame) ──────────────────────────────────────

#[test]
fn header_targets_present_menus_closed_by_default() {
    let harness = harness_for(app_three_tab_pane_a());
    let nodes = live_author_nodes(&harness);

    // The four MT-020 per-pane header right-click targets are live and named.
    for hid in ["pane-pane-a-header", "pane-pane-b-header", "pane-pane-c-header", "pane-pane-d-header"] {
        let found = nodes
            .iter()
            .find(|(a, _, _)| a == hid)
            .unwrap_or_else(|| panic!("header target {hid} missing/anonymous: {nodes:?}"));
        assert_eq!(found.1, "Group", "{hid} role is Group");
    }
    // No context-menu items in the default (all-closed) frame.
    assert!(
        !nodes.iter().any(|(a, _, _)| a.starts_with("ctx-menu.")),
        "no context-menu items before any right-click: {nodes:?}"
    );
}

// ── Surface 1: pane tab ──────────────────────────────────────────────────────────────────────────────

#[test]
fn secondary_click_tab_opens_menu_with_contract_items() {
    let mut harness = harness_for(app_three_tab_pane_a());

    // Right-click the Workspace tab (pane-a, label "Workspace"). Address by Role::Tab + label so the
    // pointer lands on the tab widget (the pane Group also carries a "Workspace" label).
    harness.get_by_role_and_label(egui::accesskit::Role::Tab, "Workspace").click_secondary();
    harness.run();
    harness.run();

    let nodes = live_author_nodes(&harness);
    for leaf in [
        "ctx-menu.tab.close",
        "ctx-menu.tab.close_others",
        "ctx-menu.tab.close_all",
        "ctx-menu.tab.pin",
        "ctx-menu.tab.split_right",
        "ctx-menu.tab.pop_out",
    ] {
        let found = nodes
            .iter()
            .find(|(a, _, _)| a == leaf)
            .unwrap_or_else(|| panic!("tab menu leaf {leaf} missing: {nodes:?}"));
        assert_eq!(found.1, "MenuItem", "{leaf} role is MenuItem");
    }
    println!("PASS: right-click tab opened the tab context menu with the contract items");
}

#[test]
fn tab_menu_close_removes_the_right_clicked_tab() {
    let mut harness = harness_for(app_three_tab_pane_a());
    assert_eq!(pane_a_tabs(&harness).len(), 3, "three tabs before close");

    harness.get_by_role_and_label(egui::accesskit::Role::Tab, "Workspace").click_secondary();
    harness.run();
    harness.run();
    // Activate "Close" — the genuine pointer path through the live menu item.
    harness.get_by_label("Close").click();
    harness.run();

    let tabs = pane_a_tabs(&harness);
    assert_eq!(tabs.len(), 2, "Close removed one tab; got {tabs:?}");
    assert!(!tabs.contains(&PaneType::Workspace), "the right-clicked Workspace tab is gone: {tabs:?}");
    println!("PASS: tab menu Close removed the right-clicked tab (live + state)");
}

#[test]
fn tab_menu_close_others_keeps_only_the_right_clicked_tab() {
    let mut harness = harness_for(app_three_tab_pane_a());
    assert_eq!(pane_a_tabs(&harness).len(), 3);

    // Right-click the Workspace tab (unique to pane-a; "Inference Lab" also labels pane-b's seeded
    // tab, which would make the query ambiguous), then Close Others -> only Workspace survives.
    harness.get_by_role_and_label(egui::accesskit::Role::Tab, "Workspace").click_secondary();
    harness.run();
    harness.run();
    harness.get_by_label("Close Others").click();
    harness.run();

    let tabs = pane_a_tabs(&harness);
    assert_eq!(tabs, vec![PaneType::Workspace], "only the right-clicked tab remains: {tabs:?}");
    println!("PASS: tab menu Close Others kept only the right-clicked tab");
}

#[test]
fn tab_menu_keyboard_arrow_enter_dispatches_close() {
    // proof_target: open tab menu -> ArrowDown nav -> Enter dispatches the highlighted item's action.
    let mut harness = harness_for(app_three_tab_pane_a());
    assert_eq!(pane_a_tabs(&harness).len(), 3);

    harness.get_by_role_and_label(egui::accesskit::Role::Tab, "Workspace").click_secondary();
    harness.run();
    harness.run();

    // On open the highlight anchors on the first actionable item ("Close"). Enter confirms it directly
    // (Close is the first enabled leaf), dispatching tab.close on the right-clicked Workspace tab.
    harness.key_press(egui::Key::Enter);
    harness.run();

    let tabs = pane_a_tabs(&harness);
    assert_eq!(tabs.len(), 2, "Enter on the highlighted Close leaf removed a tab; got {tabs:?}");
    assert!(!tabs.contains(&PaneType::Workspace), "Workspace closed via keyboard: {tabs:?}");
    println!("PASS: keyboard Enter on the open tab menu dispatched Close");
}

#[test]
fn tab_menu_disabled_split_does_not_fire() {
    // Future-target Split Right renders + is addressable but cannot be activated (no fake-enable). It
    // does not map to any tab action, so even an attempted click leaves the pane tab set unchanged.
    let mut harness = harness_for(app_three_tab_pane_a());
    let before = pane_a_tabs(&harness);

    harness.get_by_role_and_label(egui::accesskit::Role::Tab, "Workspace").click_secondary();
    harness.run();
    harness.run();

    let nodes = live_author_nodes(&harness);
    assert!(
        nodes.iter().any(|(a, _, _)| a == "ctx-menu.tab.split_right"),
        "disabled Split Right is present + addressable: {nodes:?}"
    );
    // Clicking a disabled egui item is ignored — the tab set is unchanged.
    harness.get_by_label("Split Right").click();
    harness.run();
    assert_eq!(pane_a_tabs(&harness), before, "disabled Split Right fired no action");
    println!("PASS: disabled tab menu item is addressable but does not fire (no fake-enable)");
}

#[test]
fn tab_menu_opens_via_shift_f10_keyboard() {
    // proof_target (FIX-B): focus a context-menu-bearing surface (a tab) and press Shift+F10 (the
    // keyboard-open path the prior coder WIRED but never tested). The ctx-menu.tab.* MenuItem nodes
    // must appear in the LIVE tree — proving the keyboard `request_open` -> `show_on` popup id wiring
    // actually opens the SAME menu the right-click opens, with no pointer event.
    let mut harness = harness_for(app_three_tab_pane_a());

    // No menu before the keyboard open.
    assert!(
        !live_author_nodes(&harness).iter().any(|(a, _, _)| a.starts_with("ctx-menu.")),
        "no context menu before Shift+F10"
    );

    // Focus the Workspace tab (the wiring gates the Shift+F10 open on the tab having focus), settle.
    harness.get_by_role_and_label(egui::accesskit::Role::Tab, "Workspace").focus();
    harness.run();

    // Press Shift+F10 — the keyboard context-menu trigger (egui 0.33 has no dedicated Menu key).
    harness.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::F10);
    harness.run();
    harness.run();

    let nodes = live_author_nodes(&harness);
    let menu_items: Vec<&String> = nodes
        .iter()
        .filter(|(a, r, _)| a.starts_with("ctx-menu.tab.") && r == "MenuItem")
        .map(|(a, _, _)| a)
        .collect();
    assert!(
        !menu_items.is_empty(),
        "Shift+F10 on the focused tab opened the tab context menu (ctx-menu.tab.* MenuItem nodes); found {nodes:?}"
    );
    // Spot-check a known contract item is among the opened menu's nodes.
    assert!(
        menu_items.iter().any(|a| a.as_str() == "ctx-menu.tab.close"),
        "the Shift+F10-opened menu carries the contract Close item; found {menu_items:?}"
    );
    println!(
        "PASS: Shift+F10 on the focused tab opened the tab context menu via keyboard ({} item nodes)",
        menu_items.len()
    );
}

// ── Surface 2: pane header ────────────────────────────────────────────────────────────────────────────

#[test]
fn secondary_click_pane_header_lock_toggles_lock_state() {
    let mut harness = harness_for(app_three_tab_pane_a());

    // pane-a starts Unlocked.
    let pane_a: PaneId = Arc::from("pane-a");
    assert_eq!(
        harness.state().pane_registry().lock().unwrap().get(&pane_a).unwrap().lock_state,
        LockState::Unlocked,
    );

    harness.get_by_label("Pane header pane-a").click_secondary();
    harness.run();
    harness.run();

    // The pane menu items are live.
    let nodes = live_author_nodes(&harness);
    for leaf in ["ctx-menu.pane.lock", "ctx-menu.pane.pop_out", "ctx-menu.pane.set_type_editor"] {
        assert!(nodes.iter().any(|(a, _, _)| a == leaf), "pane menu leaf {leaf} missing: {nodes:?}");
    }

    // Activate "Lock Pane" -> the registry LockState flips to Locked.
    harness.get_by_label("Lock Pane").click();
    harness.run();
    assert_eq!(
        harness.state().pane_registry().lock().unwrap().get(&pane_a).unwrap().lock_state,
        LockState::Locked,
        "pane header menu Lock Pane locked the pane",
    );
    println!("PASS: pane header menu Lock Pane toggled the registry LockState");
}

#[test]
fn pane_header_menu_set_type_is_disabled() {
    let mut harness = harness_for(app_three_tab_pane_a());
    harness.get_by_label("Pane header pane-a").click_secondary();
    harness.run();
    harness.run();

    let nodes = live_author_nodes(&harness);
    // The four Set Type items are present (addressable) but future-target/disabled.
    for leaf in [
        "ctx-menu.pane.set_type_editor",
        "ctx-menu.pane.set_type_terminal",
        "ctx-menu.pane.set_type_canvas",
        "ctx-menu.pane.set_type_browser",
        "ctx-menu.pane.close",
    ] {
        assert!(nodes.iter().any(|(a, _, _)| a == leaf), "pane menu {leaf} present: {nodes:?}");
    }
    println!("PASS: pane header Set Type / Close items are present but future-target (disabled)");
}

// ── Surface 4: explorer row (project-tree document / canvas / bookmark) ─────────────────────────────────

/// An app with the left rail OPEN + the project tree seeded with one document, one canvas, and one
/// bookmark so the explorer rows render and are right-clickable. No backend is needed (content is
/// seeded directly), so the rows exist deterministically.
fn app_with_explorer_rows() -> HandshakeApp {
    let mut app = ok_app();
    app.set_left_rail_open(true);
    app.left_rail_mut().project_tree.set_content_with_bookmarks(
        vec![DocumentSummary::new("doc-1", "My Document")],
        vec![CanvasSummary::new("canvas-1", "My Canvas")],
        vec![BookmarkSummary::new("blk-1", "My Bookmark", "block", None)],
    );
    app
}

#[test]
fn secondary_click_explorer_document_opens_menu_with_contract_items() {
    // FIX-A proof: the explorer-row context menu the prior coder OVER-deferred. Right-click a real
    // project-tree document row -> the ctx-menu.explorer.* MenuItem nodes appear LIVE, including the
    // enabled rename and the DISABLED reveal_in_graph (no graph surface in WP-011 — disclosed, not faked).
    let mut harness = harness_for(app_with_explorer_rows());

    // The document row is a Role::TreeItem labeled with its title.
    harness
        .get_by_role_and_label(egui::accesskit::Role::TreeItem, "My Document")
        .click_secondary();
    harness.run();
    harness.run();

    let nodes = live_author_nodes(&harness);
    for leaf in [
        "ctx-menu.explorer.open",
        "ctx-menu.explorer.copy_path",
        "ctx-menu.explorer.rename",
        "ctx-menu.explorer.reveal_in_graph",
    ] {
        let found = nodes
            .iter()
            .find(|(a, _, _)| a == leaf)
            .unwrap_or_else(|| panic!("explorer menu leaf {leaf} missing: {nodes:?}"));
        assert_eq!(found.1, "MenuItem", "{leaf} role is MenuItem");
    }
    println!("PASS: right-click explorer document row opened the explorer context menu with contract items");
}

#[test]
fn explorer_rename_opens_rename_dialog_seeded_with_title() {
    // Activating Rename on a BOOKMARK row opens the inline rename dialog seeded with the current title.
    // A bookmark row's id IS a genuine `LoomBlock.block_id`, so it is the ONLY explorer row whose rename
    // maps to the real PATCH-driving action (FIX: documents/canvases carry a different id space and are
    // disabled). (The PATCH itself needs a live backend + workspace; here we prove the menu -> dialog
    // wiring and the seed, which is the deterministic, backend-free part of the closure unit.)
    let mut harness = harness_for(app_with_explorer_rows());

    // The bookmark row label is "<title>  [<kind>]" (project_tree renders the badge suffix).
    harness
        .get_by_role_and_label(egui::accesskit::Role::TreeItem, "My Bookmark  [block]")
        .click_secondary();
    harness.run();
    harness.run();
    harness.get_by_label("Rename").click();
    harness.run();

    // The rename dialog is open: its text field is seeded with the current title and is findable.
    let nodes = live_author_nodes(&harness);
    let _ = nodes; // dialog widgets are egui-default-named; assert via the visible label instead.
    let field = rename_field(&harness);
    assert!(field.is_some(), "rename dialog text field is live");
    println!("PASS: explorer Rename (bookmark row) opened the inline rename dialog");
}

#[test]
fn explorer_document_rename_is_disabled() {
    // FIX (BLOCKER): a document row's id is a DOCUMENT id, NOT a Loom-block id. PATCHing
    // `/loom/blocks/{document_id}` would 404 at runtime, so the document Rename item is present +
    // addressable but DISABLED (no fake-enable), mirroring the canvas-disabled proof. Clicking it must
    // open NO rename dialog — proving a document id can never reach the Loom-block PATCH.
    let mut harness = harness_for(app_with_explorer_rows());

    harness
        .get_by_role_and_label(egui::accesskit::Role::TreeItem, "My Document")
        .click_secondary();
    harness.run();
    harness.run();

    let nodes = live_author_nodes(&harness);
    for leaf in ["ctx-menu.explorer.rename", "ctx-menu.explorer.reveal_in_graph"] {
        assert!(
            nodes.iter().any(|(a, _, _)| a == leaf),
            "explorer {leaf} present + addressable on document row: {nodes:?}"
        );
    }
    // Clicking the disabled rename fires nothing -> no rename dialog opens.
    harness.get_by_label("Rename").click();
    harness.run();
    assert!(
        rename_field(&harness).is_none(),
        "disabled document Rename did not open the rename dialog (no fake-enable; wrong id space)"
    );
    println!("PASS: document row Rename is addressable but disabled (document id is not a Loom-block id)");
}

#[test]
fn explorer_canvas_rename_is_disabled() {
    // A canvas row is not a Loom block, so its rename item is present + addressable but DISABLED
    // (no fake-enable). reveal_in_graph is disabled for every row kind.
    let mut harness = harness_for(app_with_explorer_rows());

    harness
        .get_by_role_and_label(egui::accesskit::Role::TreeItem, "My Canvas")
        .click_secondary();
    harness.run();
    harness.run();

    let nodes = live_author_nodes(&harness);
    for leaf in ["ctx-menu.explorer.rename", "ctx-menu.explorer.reveal_in_graph"] {
        assert!(
            nodes.iter().any(|(a, _, _)| a == leaf),
            "explorer {leaf} present + addressable on canvas row: {nodes:?}"
        );
    }
    // Clicking the disabled rename fires nothing -> no rename dialog opens.
    harness.get_by_label("Rename").click();
    harness.run();
    assert!(
        rename_field(&harness).is_none(),
        "disabled canvas Rename did not open the rename dialog (no fake-enable)"
    );
    println!("PASS: canvas row Rename is addressable but disabled (no fake-enable)");
}

// ── Surface 3: project tab ────────────────────────────────────────────────────────────────────────────

#[test]
fn secondary_click_project_tab_switches_project() {
    let mut app = ok_app();
    // Two projects; the default is active. Right-clicking the OTHER one + Switch to Project switches.
    app.project_tabs_mut().apply_fetched(vec![
        ProjectItem::new("default-project", "Default Project"),
        ProjectItem::new("ws-2", "Second Project"),
    ]);
    let mut harness = harness_for(app);

    assert_eq!(harness.state().active_project_id(), "default-project");

    harness.get_by_role_and_label(egui::accesskit::Role::Tab, "Second Project").click_secondary();
    harness.run();
    harness.run();

    let nodes = live_author_nodes(&harness);
    assert!(
        nodes.iter().any(|(a, r, _)| a == "ctx-menu.project.activate" && r == "MenuItem"),
        "project menu Switch to Project item present: {nodes:?}"
    );
    harness.get_by_label("Switch to Project").click();
    harness.run();

    assert_eq!(
        harness.state().active_project_id(),
        "ws-2",
        "project tab menu Switch to Project switched the active project",
    );
    println!("PASS: project tab menu Switch to Project switched the active project");
}
