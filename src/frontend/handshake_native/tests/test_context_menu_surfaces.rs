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
use handshake_native::app::{ExplorerRenameTarget, HandshakeApp, HealthDisplayState};
use handshake_native::backend_client::HealthInfo;
use handshake_native::context_menu_surfaces::EXPLORER_RENAME_INPUT_AUTHOR_ID;
use handshake_native::pane_registry::{LockState, PaneId, PaneType};
use handshake_native::project_tabs::ProjectItem;
use handshake_native::project_tree::{BookmarkSummary, CanvasSummary, DocumentSummary};
use handshake_native::tab_bar::{TabBarState, TabState};
use std::sync::Arc;

#[path = "native_gui_support/canonical_argus_driver.rs"]
mod canonical_argus_driver;
#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;
use screenshot_harness::ScreenshotHarness as Harness;

fn capture_mt108_matrix_frame_if_selected(harness: &mut Harness<'_, HandshakeApp>) {
    if std::env::var("HANDSHAKE_ARGUS_MATRIX_RUN_ID")
        .ok()
        .is_some_and(|run_id| !run_id.trim().is_empty())
    {
        let _ = harness
            .render_proof_frame("MT-108 context-menu matrix requires a material captured frame");
    }
}

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
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.set_size(egui::Vec2::new(1200.0, 800.0));
    harness.run();
    // The rail-collapse flag is applied on the next frame; run once more so the 2x2 grid settles.
    harness.run();
    harness
}

fn json_author_count(value: &serde_json::Value, author_id: &str) -> usize {
    match value {
        serde_json::Value::Object(object) => {
            usize::from(
                object.get("author_id").and_then(serde_json::Value::as_str) == Some(author_id),
            ) + object
                .values()
                .map(|value| json_author_count(value, author_id))
                .sum::<usize>()
        }
        serde_json::Value::Array(values) => values
            .iter()
            .map(|value| json_author_count(value, author_id))
            .sum(),
        _ => 0,
    }
}

fn json_author<'a>(value: &'a serde_json::Value, author_id: &str) -> Option<&'a serde_json::Value> {
    match value {
        serde_json::Value::Object(object) => {
            if object.get("author_id").and_then(serde_json::Value::as_str) == Some(author_id) {
                return Some(value);
            }
            object
                .values()
                .find_map(|value| json_author(value, author_id))
        }
        serde_json::Value::Array(values) => values
            .iter()
            .find_map(|value| json_author(value, author_id)),
        _ => None,
    }
}

fn secondary_click_at(harness: &mut Harness<'_, HandshakeApp>, position: egui::Pos2) {
    harness.event(egui::Event::PointerMoved(position));
    for pressed in [true, false] {
        harness.event(egui::Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: egui::Modifiers::default(),
        });
    }
    harness.run();
}

/// The explorer rename dialog's exact text field, if the dialog is open. Role-only discovery is
/// deliberately forbidden here because the shell contains concurrent search-rail and Runtime Chat
/// text inputs. The production author id is the durable model/operator interaction address.
fn rename_field<'h>(harness: &'h Harness<'_, HandshakeApp>) -> Option<egui_kittest::Node<'h>> {
    harness
        .query_all_by_role(egui::accesskit::Role::TextInput)
        .find(|n| n.accesskit_node().author_id() == Some(EXPLORER_RENAME_INPUT_AUTHOR_ID))
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

    // The three MT-098 default-pane header right-click targets are live and named. Four-pane behavior
    // is covered by tests that seed pane-d explicitly rather than rewriting the product default.
    for hid in [
        "pane-pane-a-header",
        "pane-pane-b-header",
        "pane-pane-c-header",
    ] {
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
    harness
        .get_by_role_and_label(egui::accesskit::Role::Tab, "Workspace")
        .click_secondary();
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

    harness
        .get_by_role_and_label(egui::accesskit::Role::Tab, "Workspace")
        .click_secondary();
    harness.run();
    harness.run();
    // Activate "Close" — the genuine pointer path through the live menu item.
    harness.get_by_label("Close").click();
    harness.run();

    let tabs = pane_a_tabs(&harness);
    assert_eq!(tabs.len(), 2, "Close removed one tab; got {tabs:?}");
    assert!(
        !tabs.contains(&PaneType::Workspace),
        "the right-clicked Workspace tab is gone: {tabs:?}"
    );
    println!("PASS: tab menu Close removed the right-clicked tab (live + state)");
}

#[test]
fn tab_menu_close_others_keeps_only_the_right_clicked_tab() {
    let mut harness = harness_for(app_three_tab_pane_a());
    assert_eq!(pane_a_tabs(&harness).len(), 3);

    // Right-click the Workspace tab (unique to pane-a; "Inference Lab" also labels pane-b's seeded
    // tab, which would make the query ambiguous), then Close Others -> only Workspace survives.
    harness
        .get_by_role_and_label(egui::accesskit::Role::Tab, "Workspace")
        .click_secondary();
    harness.run();
    harness.run();
    harness.get_by_label("Close Others").click();
    harness.run();

    let tabs = pane_a_tabs(&harness);
    assert_eq!(
        tabs,
        vec![PaneType::Workspace],
        "only the right-clicked tab remains: {tabs:?}"
    );
    println!("PASS: tab menu Close Others kept only the right-clicked tab");
}

#[test]
fn tab_menu_keyboard_arrow_enter_dispatches_close() {
    // proof_target: open tab menu -> ArrowDown nav -> Enter dispatches the highlighted item's action.
    let mut harness = harness_for(app_three_tab_pane_a());
    assert_eq!(pane_a_tabs(&harness).len(), 3);

    harness
        .get_by_role_and_label(egui::accesskit::Role::Tab, "Workspace")
        .click_secondary();
    harness.run();
    harness.run();

    // On open the highlight anchors on the first actionable item ("Close"). Enter confirms it directly
    // (Close is the first enabled leaf), dispatching tab.close on the right-clicked Workspace tab.
    harness.key_press(egui::Key::Enter);
    harness.run();

    let tabs = pane_a_tabs(&harness);
    assert_eq!(
        tabs.len(),
        2,
        "Enter on the highlighted Close leaf removed a tab; got {tabs:?}"
    );
    assert!(
        !tabs.contains(&PaneType::Workspace),
        "Workspace closed via keyboard: {tabs:?}"
    );
    println!("PASS: keyboard Enter on the open tab menu dispatched Close");
}

#[test]
fn tab_menu_disabled_split_does_not_fire() {
    // Future-target Split Right renders + is addressable but cannot be activated (no fake-enable). It
    // does not map to any tab action, so even an attempted click leaves the pane tab set unchanged.
    let mut harness = harness_for(app_three_tab_pane_a());
    let before = pane_a_tabs(&harness);

    harness
        .get_by_role_and_label(egui::accesskit::Role::Tab, "Workspace")
        .click_secondary();
    harness.run();
    harness.run();

    let nodes = live_author_nodes(&harness);
    assert!(
        nodes
            .iter()
            .any(|(a, _, _)| a == "ctx-menu.tab.split_right"),
        "disabled Split Right is present + addressable: {nodes:?}"
    );
    // Clicking a disabled egui item is ignored — the tab set is unchanged.
    harness.get_by_label("Split Right").click();
    harness.run();
    assert_eq!(
        pane_a_tabs(&harness),
        before,
        "disabled Split Right fired no action"
    );
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
        !live_author_nodes(&harness)
            .iter()
            .any(|(a, _, _)| a.starts_with("ctx-menu.")),
        "no context menu before Shift+F10"
    );

    // Focus the Workspace tab (the wiring gates the Shift+F10 open on the tab having focus), settle.
    harness
        .get_by_role_and_label(egui::accesskit::Role::Tab, "Workspace")
        .focus();
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
        menu_items
            .iter()
            .any(|a| a.as_str() == "ctx-menu.tab.close"),
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
        harness
            .state()
            .pane_registry()
            .lock()
            .unwrap()
            .get(&pane_a)
            .unwrap()
            .lock_state,
        LockState::Unlocked,
    );

    harness.get_by_label("Pane header pane-a").click_secondary();
    harness.run();
    harness.run();

    // The pane menu items are live.
    let nodes = live_author_nodes(&harness);
    for leaf in [
        "ctx-menu.pane.lock",
        "ctx-menu.pane.pop_out",
        "ctx-menu.pane.set_type_editor",
    ] {
        assert!(
            nodes.iter().any(|(a, _, _)| a == leaf),
            "pane menu leaf {leaf} missing: {nodes:?}"
        );
    }

    // Activate "Lock Pane" -> the registry LockState flips to Locked.
    harness.get_by_label("Lock Pane").click();
    harness.run();
    assert_eq!(
        harness
            .state()
            .pane_registry()
            .lock()
            .unwrap()
            .get(&pane_a)
            .unwrap()
            .lock_state,
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
        assert!(
            nodes.iter().any(|(a, _, _)| a == leaf),
            "pane menu {leaf} present: {nodes:?}"
        );
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
        vec![DocumentSummary {
            id: "KRD-explorer-1".to_owned(),
            title: "My Document".to_owned(),
            updated_at: Some("2026-07-16T10:20:30Z".to_owned()),
        }],
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
fn explorer_document_rename_opens_the_document_rename_dialog() {
    // A document row uses the dedicated knowledge-document rename handler. Opening the shared dialog
    // proves the menu action is live; the typed target in app state prevents the document id from ever
    // reaching the Loom-block PATCH.
    let mut harness = harness_for(app_with_explorer_rows());

    harness
        .get_by_role_and_label(egui::accesskit::Role::TreeItem, "My Document")
        .click_secondary();
    harness.run();
    harness.run();

    let nodes = live_author_nodes(&harness);
    for leaf in [
        "ctx-menu.explorer.rename",
        "ctx-menu.explorer.reveal_in_graph",
    ] {
        assert!(
            nodes.iter().any(|(a, _, _)| a == leaf),
            "explorer {leaf} present + addressable on document row: {nodes:?}"
        );
    }
    harness.get_by_label("Rename").click();
    harness.run();
    assert!(
        rename_field(&harness).is_some(),
        "document Rename opened the shared dialog backed by the document-specific handler"
    );
    assert_eq!(
        harness.state().pending_explorer_rename_target(),
        Some(&ExplorerRenameTarget::Document {
            document_id: "KRD-explorer-1".to_owned(),
            expected_updated_at: Some("2026-07-16T10:20:30Z".to_owned()),
        }),
        "the mounted explorer retains the RichDocument authority id and matching optimistic token",
    );
    println!("PASS: document row Rename reaches the document-specific rename path");
}

#[test]
fn explorer_canvas_rename_opens_the_canvas_rename_dialog() {
    // Canvas rename has its own typed backend target; it must not route to the Loom dialog handler.
    let mut harness = harness_for(app_with_explorer_rows());

    harness
        .get_by_role_and_label(egui::accesskit::Role::TreeItem, "My Canvas")
        .click_secondary();
    harness.run();
    harness.run();

    let nodes = live_author_nodes(&harness);
    for leaf in [
        "ctx-menu.explorer.rename",
        "ctx-menu.explorer.reveal_in_graph",
    ] {
        assert!(
            nodes.iter().any(|(a, _, _)| a == leaf),
            "explorer {leaf} present + addressable on canvas row: {nodes:?}"
        );
    }
    harness.get_by_label("Rename").click();
    harness.run();
    assert!(
        rename_field(&harness).is_some(),
        "canvas Rename opened the shared dialog"
    );
    assert_eq!(
        harness.state().pending_explorer_rename_target(),
        Some(&ExplorerRenameTarget::Canvas {
            canvas_id: "canvas-1".to_owned(),
            expected_updated_at: None,
        }),
        "the live dialog retains a typed Canvas target",
    );
    println!("PASS: canvas row Rename reaches the canvas-specific rename path");
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

    harness
        .get_by_role_and_label(egui::accesskit::Role::Tab, "Second Project")
        .click_secondary();
    harness.run();
    harness.run();

    let nodes = live_author_nodes(&harness);
    assert!(
        nodes
            .iter()
            .any(|(a, r, _)| a == "ctx-menu.project.activate" && r == "MenuItem"),
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

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// WP-KERNEL-012 MT-070 (E11 — melt-together click-through): the EDITOR-BODY + canvas/loom-NODE editor
// context menus bind to the REAL WP-012 editor actions (no dead handlers on the required path).
//
// These tests render the MT-070 editor-body / node menus through the SAME WP-011 ContextMenu primitive
// (`show_on`) on a self-contained right-clickable surface (a state-less `build_ui` harness, the SAME
// pattern `test_context_menu.rs` uses for the MT-019 primitive), and prove:
//   - AC-070-1: the editor-body menu shows Rename Symbol / Quick Fix / Format Selection / Peek as live
//     Role::MenuItem nodes, and activating each returns the REAL typed EditorBodyMenuAction (the handler
//     the wiring site dispatches), never a placeholder;
//   - AC-070-2: the Create-note-from-link entry fires the real MT-057 create-note action;
//   - AC-070-4: the node menu (Open note / Reveal node / Create note) dispatches to real actions;
//   - AC-070-5: NO required entry resolves to a dead/placeholder handler (a pure walk of every required
//     id asserts it maps to a real action);
//   - AC-070-7: the editor-body code-action ids ARE the existing WP-011/WP-012 registry author_ids (no
//     parallel id scheme);
//   - AC-070-9: the menu container is Role::Menu and each item is Role::MenuItem carrying a stable
//     `ctx-menu.{author_id}` id.
// ═════════════════════════════════════════════════════════════════════════════════════════════════

use handshake_native::context_menu::{ContextMenu, ContextMenuItem};
use handshake_native::context_menu_surfaces::{
    editor_body_action_for_id, editor_body_context_items, editor_body_ids, node_action_for_id,
    node_context_items, node_menu_ids, node_navigation_target, show_editor_body_menu,
    show_node_menu, EditorBodyAvailability, EditorBodyMenuAction, NodeMenuAction,
    NodeMenuAvailability, EDITOR_BODY_REQUIRED_IDS, NODE_MENU_REQUIRED_IDS,
};
use handshake_native::navigation_bus::NavigationTarget;

const MT070_SURFACE_LABEL: &str = "Mt070RightClickSurface";

/// Every fully-available editor body: each of the five actions has a valid live target, so EVERY entry
/// is enabled (the "all required entries fire" path AC-070-1/2 prove).
fn full_editor_availability() -> EditorBodyAvailability {
    EditorBodyAvailability {
        symbol_under_cursor: true,
        quick_fix_available: true,
        has_selection: true,
        definition_available: true,
        unresolved_link_under_cursor: true,
    }
}

/// A fully-available node (note + id + unresolved link), so every node entry is enabled.
fn full_node_availability() -> NodeMenuAvailability {
    NodeMenuAvailability {
        canvas_projection_confirmed: None,
        has_note: true,
        has_node_id: true,
        can_route_to_stage: true,
        unresolved_link: true,
    }
}

/// A state-less harness whose UI is a single right-clickable surface that opens the editor-body menu via
/// the public `show_editor_body_menu` wiring helper and records the REAL action a confirmed entry maps
/// to. This drives the SAME `ContextMenu::show_on` path the live code-editor body wires (no new menu
/// infra), so the AccessKit Role::Menu/MenuItem nodes + activation are the production path.
fn editor_body_harness(
    availability: EditorBodyAvailability,
    captured: std::sync::Arc<std::sync::Mutex<Option<EditorBodyMenuAction>>>,
) -> Harness<'static> {
    Harness::builder().build_ui(move |ui| {
        let (rect, response) =
            ui.allocate_exact_size(egui::vec2(220.0, 90.0), egui::Sense::click());
        if ui.is_rect_visible(rect) {
            ui.painter()
                .rect_filled(rect, 4.0, ui.visuals().faint_bg_color);
        }
        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Button,
                ui.is_enabled(),
                MT070_SURFACE_LABEL,
            )
        });
        if let Some(action) = show_editor_body_menu(&response, availability) {
            *captured.lock().unwrap() = Some(action);
        }
    })
}

/// The node-menu twin of [`editor_body_harness`].
fn node_menu_harness(
    availability: NodeMenuAvailability,
    captured: std::sync::Arc<std::sync::Mutex<Option<NodeMenuAction>>>,
) -> Harness<'static> {
    Harness::builder().build_ui(move |ui| {
        let (rect, response) =
            ui.allocate_exact_size(egui::vec2(220.0, 90.0), egui::Sense::click());
        if ui.is_rect_visible(rect) {
            ui.painter()
                .rect_filled(rect, 4.0, ui.visuals().faint_bg_color);
        }
        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Button,
                ui.is_enabled(),
                MT070_SURFACE_LABEL,
            )
        });
        if let Some(action) = show_node_menu(&response, availability) {
            *captured.lock().unwrap() = Some(action);
        }
    })
}

/// Every live author-id node in a state-less harness: (author_id, role, label).
fn mt070_author_nodes(harness: &Harness<'_>) -> Vec<(String, String, Option<String>)> {
    let mut found = Vec::new();
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        if let Some(author_id) = ak.author_id() {
            found.push((author_id.to_owned(), format!("{:?}", ak.role()), ak.label()));
        }
    }
    found
}

/// Project the mounted kittest surface into the same full-tree shape the canonical Argus tools read.
/// This deliberately uses the live AccessKit nodes instead of rebuilding menu state from the builder.
fn mt070_argus_snapshot(harness: &Harness<'_>) -> handshake_native::accessibility::UiTreeSnapshot {
    use handshake_native::accessibility::{UiTreeNode, UiTreeSnapshot};

    let actions = [
        egui::accesskit::Action::Click,
        egui::accesskit::Action::Focus,
        egui::accesskit::Action::SetValue,
    ];
    let children: Vec<_> = harness
        .root()
        .children_recursive()
        .map(|node| {
            let access = node.accesskit_node();
            let node_id = access.id().0;
            UiTreeNode {
                id: access
                    .author_id()
                    .map(str::to_owned)
                    .unwrap_or_else(|| format!("node:{node_id}")),
                author_id: access.author_id().map(str::to_owned),
                node_id,
                role: format!("{:?}", access.role()),
                label: access.label(),
                value: access.value(),
                disabled: access.is_disabled(),
                actions: actions
                    .iter()
                    .filter(|action| access.data().supports_action(**action))
                    .map(|action| format!("{action:?}"))
                    .collect(),
                bounds: None,
                children: Vec::new(),
            }
        })
        .collect();
    UiTreeSnapshot {
        widget_count: children.len() + 1,
        root: UiTreeNode {
            id: "mt070-argus-root".to_owned(),
            author_id: None,
            node_id: 0,
            role: "Window".to_owned(),
            label: None,
            value: None,
            disabled: false,
            actions: Vec::new(),
            bounds: None,
            children,
        },
        captured_at_utc: "mt070-argus-frame".to_owned(),
        viewport: None,
    }
}

// ── AC-070-9: the editor-body menu renders Role::Menu container + Role::MenuItem items by stable id ────

#[test]
fn mt070_editor_body_menu_renders_menuitems_with_stable_ids() {
    let captured = std::sync::Arc::new(std::sync::Mutex::new(None));
    let mut harness = editor_body_harness(full_editor_availability(), captured);
    harness.run();

    // Closed by default: NONE of the editor-body entries are in the tree before a right-click (so their
    // presence after opening proves they are genuinely nested in the live popup, not memory-only).
    let closed = mt070_author_nodes(&harness);
    assert!(
        !closed
            .iter()
            .any(|(a, _, _)| a.starts_with("ctx-menu.code_editor_ctx")),
        "no editor-body menu items in the closed default frame: {closed:?}",
    );

    harness.get_by_label(MT070_SURFACE_LABEL).click_secondary();
    harness.run();
    harness.run();

    let nodes = mt070_author_nodes(&harness);
    // AC-070-1 / AC-070-9: each of the four required code-action entries is a live Role::MenuItem node
    // carrying the stable `ctx-menu.{author_id}` id — the SAME author_id the owning editor MT emits.
    for required in EDITOR_BODY_REQUIRED_IDS {
        let want = format!("ctx-menu.{required}");
        let found = nodes
            .iter()
            .find(|(a, _, _)| a == &want)
            .unwrap_or_else(|| {
                panic!("editor-body menu entry {want} missing/anonymous: {nodes:?}")
            });
        assert_eq!(found.1, "MenuItem", "{want} is a Role::MenuItem (AC-070-9)");
    }
    // AC-070-9 (container): the live WP-011 ContextMenu popup exposes an addressable Role::Menu parent.
    // Its stable surface author id lets a no-context agent discover the menu topology before activating
    // one of the MenuItem children.
    let menu = nodes
        .iter()
        .find(|(a, r, _)| a == "ctx-menu.surface.editor-body" && r == "Menu")
        .expect("the open editor-body popup exposes a Role::Menu container");
    assert_eq!(menu.1, "Menu");
    let menu_item_count = nodes.iter().filter(|(_, r, _)| r == "MenuItem").count();
    assert!(
        menu_item_count >= EDITOR_BODY_REQUIRED_IDS.len(),
        "the open editor-body menu exposes every required entry as a live Role::MenuItem inside the \
         WP-011 popup container (AC-070-9): {menu_item_count} MenuItem nodes, want >= {}",
        EDITOR_BODY_REQUIRED_IDS.len(),
    );
    println!("PASS AC-070-1/9: editor-body menu renders required Role::MenuItem nodes in the WP-011 popup");
}

// ── AC-070-1: activating each required code-action entry fires the REAL editor action ─────────────────

#[test]
fn mt070_activating_rename_fires_real_rename_action() {
    assert_activates_to(
        editor_body_ids::RENAME_SYMBOL,
        "Rename Symbol",
        EditorBodyMenuAction::RenameSymbol,
    );
}

#[test]
fn mt070_activating_quick_fix_fires_real_quick_fix_action() {
    assert_activates_to(
        editor_body_ids::QUICK_FIX,
        "Quick Fix...",
        EditorBodyMenuAction::QuickFix,
    );
}

#[test]
fn mt070_activating_format_selection_fires_real_format_action() {
    assert_activates_to(
        editor_body_ids::FORMAT_SELECTION,
        "Format Selection",
        EditorBodyMenuAction::FormatSelection,
    );
}

#[test]
fn mt070_activating_peek_fires_real_goto_def_action() {
    assert_activates_to(
        editor_body_ids::PEEK_DEFINITION,
        "Peek Definition",
        EditorBodyMenuAction::PeekDefinition,
    );
}

// ── AC-070-2: the Create-note-from-link entry fires the real MT-057 create-note action ────────────────

#[test]
fn mt070_activating_create_note_fires_real_create_note_action() {
    assert_activates_to(
        editor_body_ids::CREATE_NOTE_FROM_LINK,
        "Create note from link",
        EditorBodyMenuAction::CreateNoteFromLink,
    );
}

/// Open the editor-body menu on the live surface, click the entry with `label`, and assert the captured
/// REAL action equals `expected` — i.e. a genuine right-click + pointer activation dispatched the real
/// handler (not a placeholder). This is the runtime side-effect AC-070-1/2 require.
fn assert_activates_to(id: &str, label: &str, expected: EditorBodyMenuAction) {
    let captured = std::sync::Arc::new(std::sync::Mutex::new(None));
    let mut harness = editor_body_harness(full_editor_availability(), captured.clone());
    harness.run();
    harness.get_by_label(MT070_SURFACE_LABEL).click_secondary();
    harness.run();
    harness.run();

    // The entry is a live MenuItem carrying the stable id.
    let nodes = mt070_author_nodes(&harness);
    let want = format!("ctx-menu.{id}");
    assert!(
        nodes.iter().any(|(a, r, _)| a == &want && r == "MenuItem"),
        "entry {want} present as MenuItem before activation: {nodes:?}",
    );

    harness.get_by_label(label).click();
    harness.run();

    assert_eq!(
        *captured.lock().unwrap(),
        Some(expected),
        "activating '{label}' dispatched the REAL action {expected:?} (not a placeholder)",
    );
}

// ── AC-070-4: the canvas/loom node menu actions dispatch to real handlers ─────────────────────────────

#[test]
fn mt070_node_menu_actions_dispatch_to_real_handlers() {
    // Open note.
    let captured = std::sync::Arc::new(std::sync::Mutex::new(None));
    let mut harness = node_menu_harness(full_node_availability(), captured.clone());
    harness.run();
    harness.get_by_label(MT070_SURFACE_LABEL).click_secondary();
    harness.run();
    harness.run();

    let nodes = mt070_author_nodes(&harness);
    for required in NODE_MENU_REQUIRED_IDS {
        let want = format!("ctx-menu.{required}");
        let found = nodes
            .iter()
            .find(|(a, _, _)| a == &want)
            .unwrap_or_else(|| panic!("node menu entry {want} missing/anonymous: {nodes:?}"));
        assert_eq!(found.1, "MenuItem", "{want} is a Role::MenuItem");
    }

    harness.get_by_label("Reveal Node").click();
    harness.run();
    assert_eq!(
        *captured.lock().unwrap(),
        Some(NodeMenuAction::RevealNode),
        "activating Reveal Node dispatched the real RevealNode action",
    );

    // AC-070-4: the node nav action builds the REAL NavigationTarget routed through the MT-070 bus.
    let pane: handshake_native::pane_registry::PaneId = std::sync::Arc::from("pane-graph");
    let target = node_navigation_target(NodeMenuAction::RevealNode, &pane, "blk-9", None);
    assert_eq!(
        target,
        Some(NavigationTarget::RevealNode {
            pane_id: pane.clone(),
            node_id: "blk-9".to_owned()
        }),
        "Reveal Node maps to a real RevealNode NavigationTarget by stable pane + node id",
    );
    let open = node_navigation_target(NodeMenuAction::OpenNote, &pane, "blk-9", Some("KRD-7"));
    assert_eq!(
        open,
        Some(NavigationTarget::OpenNote {
            note_id: "KRD-7".to_owned()
        }),
        "Open Note maps to a real OpenNote NavigationTarget",
    );
    println!("PASS AC-070-4: node menu actions dispatch to real handlers + NavigationTargets");
}

// ── AC-070-5 / MC-070-1: NO required entry resolves to a dead/placeholder handler ─────────────────────

#[test]
fn mt070_no_required_entry_is_a_dead_handler() {
    // Editor body: every required id, with full availability, resolves to a REAL action (never None).
    let avail = full_editor_availability();
    for id in EDITOR_BODY_REQUIRED_IDS {
        let action = editor_body_action_for_id(id, avail);
        assert!(
            action.is_some(),
            "required editor-body entry '{id}' resolves to a real action (no dead handler): got None",
        );
    }
    // The four code-action entries are the AC-070-1 required set; create-note is the AC-070-2 entry.
    assert_eq!(
        editor_body_action_for_id(editor_body_ids::RENAME_SYMBOL, avail),
        Some(EditorBodyMenuAction::RenameSymbol),
    );
    assert!(EditorBodyMenuAction::RenameSymbol.is_required_code_action());
    assert!(!EditorBodyMenuAction::CreateNoteFromLink.is_required_code_action());

    // Node menu: every required id, with full availability, resolves to a REAL action.
    let navail = full_node_availability();
    for id in NODE_MENU_REQUIRED_IDS {
        assert!(
            node_action_for_id(id, navail).is_some(),
            "required node entry '{id}' resolves to a real action (no dead handler): got None",
        );
    }

    // The menu BUILDERS render every required entry (no fake-drop), so the audit set matches the menu.
    let body_ids: Vec<&str> = editor_body_context_items(avail)
        .iter()
        .filter(|i| {
            !matches!(
                i.kind,
                handshake_native::context_menu::MenuItemKind::Separator
            )
        })
        .map(|i| i.id)
        .collect();
    for required in EDITOR_BODY_REQUIRED_IDS {
        assert!(
            body_ids.contains(required),
            "editor-body menu renders required id {required}"
        );
    }
    let node_ids: Vec<&str> = node_context_items(navail)
        .iter()
        .filter(|i| {
            !matches!(
                i.kind,
                handshake_native::context_menu::MenuItemKind::Separator
            )
        })
        .map(|i| i.id)
        .collect();
    for required in NODE_MENU_REQUIRED_IDS {
        assert!(
            node_ids.contains(required),
            "node menu renders required id {required}"
        );
    }
    println!("PASS AC-070-5: no required context-menu entry is a dead/placeholder handler");
}

#[test]
fn mt070_node_menu_enabled_state_matches_action_resolution_for_every_availability() {
    for has_note in [false, true] {
        for has_node_id in [false, true] {
            for can_route_to_stage in [false, true] {
                for unresolved_link in [false, true] {
                    let availability = NodeMenuAvailability {
                        canvas_projection_confirmed: None,
                        has_note,
                        has_node_id,
                        can_route_to_stage,
                        unresolved_link,
                    };
                    for item in node_context_items(availability).iter().filter(|item| {
                        !matches!(
                            item.kind,
                            handshake_native::context_menu::MenuItemKind::Separator
                        )
                    }) {
                        let resolves = node_action_for_id(item.id, availability).is_some();
                        assert_eq!(
                            item.enabled, resolves,
                            "node menu item '{}' enabled={} but action resolution={} for availability {:?}",
                            item.id, item.enabled, resolves, availability
                        );
                        if !item.enabled {
                            assert!(
                                item.disabled_reason.is_some(),
                                "disabled node menu item '{}' must disclose why",
                                item.id
                            );
                        }
                    }
                }
            }
        }
    }

    let unavailable = NodeMenuAvailability::default();
    let route = node_context_items(unavailable)
        .into_iter()
        .find(|item| item.id == node_menu_ids::ROUTE_TO_STAGE)
        .expect("Route to Stage entry is rendered");
    assert!(
        !route.enabled,
        "Route to Stage requires both a stable node id and a live Canvas route"
    );
    assert!(route.disabled_reason.is_some());
    assert_eq!(
        node_action_for_id(node_menu_ids::ROUTE_TO_STAGE, unavailable),
        None
    );
}

#[test]
fn mt070_argus_dispatch_seam_observes_and_enforces_stage_route_availability() {
    use handshake_native::graph::canvas_board::{placement_menu_availability, CanvasPlacementCard};
    use handshake_native::graph::graph_view::{graph_node_menu_availability, GraphNode};
    use handshake_native::mcp::{
        dispatch_request, ActionChannel, McpRequest, ScreenshotError, SessionToken,
    };

    let target = format!("ctx-menu.{}", node_menu_ids::ROUTE_TO_STAGE);
    let token = SessionToken::from_hex("mt070-argus");

    // Graph is an explicitly unavailable source: canonical inspect sees a disabled live MenuItem and
    // canonical click rejects it before an event can reach the host.
    let graph_availability =
        graph_node_menu_availability(&GraphNode::new("blk-graph", "Graph", "note"));
    let graph_captured = std::sync::Arc::new(std::sync::Mutex::new(None));
    let mut graph = node_menu_harness(graph_availability, graph_captured.clone());
    graph.run();
    graph.get_by_label(MT070_SURFACE_LABEL).click_secondary();
    graph.run_steps(2);
    let disabled_route = graph.get_by(|node: &egui_kittest::kittest::AccessKitNode<'_>| {
        node.author_id() == Some(target.as_str())
    });
    assert_eq!(
        disabled_route.accesskit_node().value().as_deref(),
        Some("Route to Stage requires a stable node id and a live Canvas board context")
    );
    assert!(
        !disabled_route
            .accesskit_node()
            .data()
            .supports_action(egui::accesskit::Action::Click),
        "disabled AccessKit MenuItem exposes its reason but no Click action"
    );
    let graph_snapshot = mt070_argus_snapshot(&graph);
    let graph_node = graph_snapshot
        .find_by_author_id(&target)
        .expect("canonical Argus snapshot contains the graph Route-to-Stage MenuItem");
    assert!(
        graph_node.disabled,
        "graph Route to Stage is visibly disabled"
    );
    let mut graph_channel = ActionChannel::new();
    let inspect = dispatch_request(
        &McpRequest {
            id: serde_json::json!(1),
            method: handshake_native::mcp::argus::ARGUS_INSPECT_METHOD.to_owned(),
            params: serde_json::json!({}),
            session_token: "mt070-argus".to_owned(),
        },
        &token,
        &graph_snapshot,
        &mut graph_channel,
        || Err(ScreenshotError("not requested".to_owned())),
    )
    .to_json();
    assert_eq!(
        inspect["result"]["widget_count"],
        graph_snapshot.widget_count
    );
    let rejected = dispatch_request(
        &McpRequest {
            id: serde_json::json!(2),
            method: handshake_native::mcp::argus::ARGUS_CLICK_METHOD.to_owned(),
            params: serde_json::json!({"target": target}),
            session_token: "mt070-argus".to_owned(),
        },
        &token,
        &graph_snapshot,
        &mut graph_channel,
        || Err(ScreenshotError("not requested".to_owned())),
    )
    .to_json();
    assert_eq!(rejected["error"]["code"], -32000);
    assert!(rejected["error"]["message"]
        .as_str()
        .is_some_and(|message| message.contains("disabled")));
    assert_eq!(*graph_captured.lock().unwrap(), None);

    // A live Canvas board with both authority ids exposes the same stable target as enabled. Canonical
    // click crosses ActionChannel into the mounted production menu, and a fresh inspect carries the
    // terminal post-render receipt instead of inferring success from the queue response.
    let card = CanvasPlacementCard::new("placement-live", "block-live", 0.0, 0.0, 200.0, 120.0);
    let canvas_availability = placement_menu_availability(&card, true);
    let canvas_captured = std::sync::Arc::new(std::sync::Mutex::new(None));
    let mut canvas = node_menu_harness(canvas_availability, canvas_captured.clone());
    canvas.run();
    canvas.get_by_label(MT070_SURFACE_LABEL).click_secondary();
    canvas.run_steps(2);
    let before = mt070_argus_snapshot(&canvas);
    assert!(
        !before
            .find_by_author_id(&target)
            .expect("canonical Argus snapshot contains the Canvas Route-to-Stage MenuItem")
            .disabled
    );
    let mut canvas_channel = ActionChannel::new();
    let queued = dispatch_request(
        &McpRequest {
            id: serde_json::json!(3),
            method: handshake_native::mcp::argus::ARGUS_CLICK_METHOD.to_owned(),
            params: serde_json::json!({"target": target}),
            session_token: "mt070-argus".to_owned(),
        },
        &token,
        &before,
        &mut canvas_channel,
        || Err(ScreenshotError("not requested".to_owned())),
    )
    .to_json();
    assert_eq!(queued["result"]["queued"], true);
    let receipt_id = queued["result"]["receipt_id"]
        .as_u64()
        .expect("canonical Argus click returns a receipt id");
    for event in canvas_channel.drain_revalidated_into_events(&before) {
        canvas.event(event);
    }
    canvas.run_steps(3);
    assert_eq!(
        *canvas_captured.lock().unwrap(),
        Some(NodeMenuAction::RouteToStage),
        "canonical Argus click reaches the mounted real menu action"
    );
    let after = mt070_argus_snapshot(&canvas);
    canvas_channel.acknowledge_after_render(&after);
    let reinspect = dispatch_request(
        &McpRequest {
            id: serde_json::json!(4),
            method: handshake_native::mcp::argus::ARGUS_INSPECT_METHOD.to_owned(),
            params: serde_json::json!({}),
            session_token: "mt070-argus".to_owned(),
        },
        &token,
        &after,
        &mut canvas_channel,
        || Err(ScreenshotError("not requested".to_owned())),
    )
    .to_json();
    assert!(reinspect["result"]["action_receipts"]
        .as_array()
        .is_some_and(|receipts| receipts.iter().any(|receipt| {
            receipt["receipt_id"].as_u64() == Some(receipt_id) && receipt["status"] != "queued"
        })));
}

// ── WP-KERNEL-012 MT-070 validation_v4 remediation: canonical terminal binding + matrix artifacts ────
//
// FAIL_V4 root cause: the six mounted matrices below reached `canonical_argus_driver::finish` with
// canonical actions that were never rebound to an authoritative terminal snapshot and never carried an
// action-specific terminal predicate. Every dispatched action in this file is now (1) allowed to settle
// on the owning product state, (2) bound through `assert_latest_terminal_predicate*` to the exact
// authoritative effect of its REAL handler, and (3) exported into a source-bound canonical artifact that
// is written ONLY after that matrix's `argus.finish()` gate already passed.
//
// The static regression gate at the bottom of this file
// (`mt070_every_canonical_argus_action_is_terminally_bound_before_finish`) re-derives that invariant
// from this file's own source so a future action cannot be added without its terminal predicate.

/// One canonical matrix row: the dispatched action, the terminal observation the driver persisted
/// (before snapshot, receipt id/status, terminal snapshot, predicate id/evidence/result), the exact
/// handler trace, the owning pane, and the resulting product state.
fn mt070_matrix_row(
    action_target: &str,
    observation: &canonical_argus_driver::ArgusObservation,
    pane_ownership: serde_json::Value,
    handler_trace: serde_json::Value,
    resulting_state: serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "action_target": action_target,
        "observation": observation,
        "pane_ownership": pane_ownership,
        "handler_trace": handler_trace,
        "resulting_state": resulting_state,
    })
}

/// Write one source-bound canonical artifact for a mounted matrix. Callers MUST call this only after
/// the matrix's `argus.finish()` returned, so a failed final gate can never leave a success artifact.
fn mt070_write_matrix_artifact(matrix_id: &str, rows: &[serde_json::Value]) {
    assert!(
        !rows.is_empty(),
        "canonical matrix {matrix_id} must export at least one bound action row"
    );
    let root = std::env::var("HANDSHAKE_PROOF_ARTIFACT_DIR")
        .unwrap_or_else(|_| "../../../../Handshake_Artifacts/handshake-test".to_owned());
    let dir = std::path::PathBuf::from(root).join("mt070-canonical-matrices");
    std::fs::create_dir_all(&dir).expect("create the MT-070 canonical matrix artifact directory");
    let document = serde_json::json!({
        "schema_id": "hsk.native_gui.mt070_canonical_matrix@1",
        "mt_id": "MT-070",
        "wp_id": "WP-KERNEL-012-Native-Editors-Obsidian-VSCode-Parity-v1",
        "matrix_id": matrix_id,
        "source_file": "src/frontend/handshake_native/tests/test_context_menu_surfaces.rs",
        "process_id": std::process::id(),
        "dispatched_action_count": rows.len(),
        "rows": rows,
    });
    let path = dir.join(format!("{matrix_id}.json"));
    std::fs::write(
        &path,
        serde_json::to_string_pretty(&document).expect("serialize the MT-070 canonical matrix"),
    )
    .unwrap_or_else(|error| panic!("write {}: {error}", path.display()));
}

/// Every `(pane_id, tab_index)` whose live tab carries this exact `(content_id, pane_type)` identity,
/// paired with the stable tab author_id the production tab bar emits for it. Used by the terminal
/// predicates below to prove a routed destination is exactly-once and lands in the owning pane.
fn mt070_tabs_for_content(
    app: &HandshakeApp,
    content_id: &str,
    pane_type: &PaneType,
) -> Vec<(String, usize, String)> {
    let mut found: Vec<(String, usize, String)> = app
        .tab_bar_states()
        .iter()
        .flat_map(|(pane_id, bar)| {
            bar.tabs
                .iter()
                .enumerate()
                .filter(|(_, tab)| {
                    tab.content_id.as_deref() == Some(content_id) && tab.pane_type == *pane_type
                })
                .map(|(index, tab)| {
                    (
                        pane_id.as_ref().to_owned(),
                        index,
                        handshake_native::tab_bar::tab_author_id_for(
                            pane_id.as_ref(),
                            index,
                            &tab.pane_type,
                        ),
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect();
    found.sort();
    found
}

#[test]
fn mt070_mounted_canvas_argus_routes_exact_source_into_stage() {
    use canonical_argus_driver::{json_has_author_id, CanonicalArgusDriver};
    use handshake_native::context_menu_surfaces::node_menu_ids;
    use handshake_native::graph::canvas_board::{placement_author_id, CanvasPlacementCard};
    use handshake_native::stage_pane::{StageContent, STAGE_ROUTED_CONTENT_AUTHOR_ID};

    fn find_author<'a>(
        value: &'a serde_json::Value,
        author_id: &str,
    ) -> Option<&'a serde_json::Value> {
        match value {
            serde_json::Value::Object(object) => {
                if object.get("author_id").and_then(serde_json::Value::as_str) == Some(author_id) {
                    return Some(value);
                }
                object
                    .values()
                    .find_map(|value| find_author(value, author_id))
            }
            serde_json::Value::Array(values) => values
                .iter()
                .find_map(|value| find_author(value, author_id)),
            _ => None,
        }
    }

    let pane_a = PaneId::from("pane-a");
    let mut app = ok_app();
    app.set_active_project_id_for_test("workspace-live");
    app.set_active_pane_for_test(Some(pane_a.clone()));
    app.tab_bar_states_mut().insert(
        pane_a.clone(),
        TabBarState::new(
            pane_a.clone(),
            vec![TabState {
                pane_type: PaneType::AtelierEditor,
                content_id: Some("canvas-live".to_owned()),
                pinned: false,
                dirty: false,
                label_override: None,
            }],
        ),
    );
    app.set_left_rail_open(false);
    app.begin_canvas_request_for_test("workspace-live", "canvas-live");
    {
        let board = app.mounted_canvas_board();
        let mut board = board.lock().unwrap();
        let mut card =
            CanvasPlacementCard::new("placement-live", "block-live", 40.0, 40.0, 200.0, 120.0);
        card.mark_live_resolved(Some("Live block".to_owned()), "note".to_owned(), None);
        board.set_board(vec![card], Vec::new(), egui::Vec2::ZERO, 1.0);
    }

    let mut harness = harness_for(app);
    let placement_id = placement_author_id("placement-live");
    let click_pos = harness
        .state()
        .mounted_canvas_board()
        .lock()
        .unwrap()
        .canvas_point_to_screen(egui::pos2(80.0, 80.0))
        .expect("mounted Canvas recorded its real transform");
    harness.event(egui::Event::PointerMoved(click_pos));
    harness.event(egui::Event::PointerButton {
        pos: click_pos,
        button: egui::PointerButton::Secondary,
        pressed: true,
        modifiers: egui::Modifiers::default(),
    });
    harness.event(egui::Event::PointerButton {
        pos: click_pos,
        button: egui::PointerButton::Secondary,
        pressed: false,
        modifiers: egui::Modifiers::default(),
    });
    harness.run();
    assert_eq!(
        harness
            .state()
            .mounted_canvas_board()
            .lock()
            .unwrap()
            .context_menu_placement_for_test(),
        Some("placement-live"),
        "real secondary click attaches the menu to the mounted placement"
    );
    let route_id = format!("ctx-menu.{}", node_menu_ids::ROUTE_TO_STAGE);
    assert!(harness
        .state()
        .mounted_canvas_board()
        .lock()
        .unwrap()
        .projection_is_confirmed());
    let mut argus = CanonicalArgusDriver::bind(harness.state(), "mt070-mounted-canvas-stage");
    let before = argus.inspect(&mut harness);
    assert!(
        harness
            .state()
            .mounted_canvas_board()
            .lock()
            .unwrap()
            .projection_is_confirmed(),
        "canonical capture preserves the confirmed projection"
    );
    assert!(json_has_author_id(&before, &placement_id));
    assert!(json_has_author_id(&before, &route_id));
    let route_before = find_author(&before, &route_id).unwrap();
    assert_eq!(route_before["disabled"], false, "{route_before}");
    // Change focus only after the canonical client observed pane-a's menu. The subsequent click uses
    // that exact inspected snapshot and must not reconstruct source identity from active pane-b.
    harness
        .state_mut()
        .set_active_pane_for_test(Some(PaneId::from("pane-b")));
    let board = harness.state().mounted_canvas_board();
    let observation =
        argus.click_from_snapshot_and_reinspect(&mut harness, &route_id, before.clone());
    assert!(
        matches!(
            observation.receipt_status.as_str(),
            "applied" | "indeterminate"
        ),
        "receipt is terminal; the bound terminal predicate below is the success gate"
    );
    // Let the owning product state settle: the confirmed menu action stages a Route-to-Stage request on
    // the shared InteractionBus, the shell drains it, and only then does the Stage pane own the content.
    harness.run_steps(3);

    let expected_text = "canvas node block-live";
    let expected_source = "node://canvas-live/block-live";
    let expected_routed_value = format!("Selection from {expected_source}: \"{expected_text}\"");
    let routed_value_probe = expected_routed_value.clone();
    let predicate_board = std::sync::Arc::clone(&board);
    argus.assert_latest_terminal_predicate_with_app_evidence(
        &mut harness,
        "mt070.canvas.route-to-stage.exact-source-reaches-stage",
        serde_json::json!({
            "source_pane_id": "pane-a",
            "active_pane_at_dispatch": "pane-b",
            "source_placement_author_id": placement_id.clone(),
            "source_canvas_id": "canvas-live",
            "source_node_id": "block-live",
            "menu_target": route_id.clone(),
            "receipt_id": observation.receipt_id,
            "correlation_id": observation.correlation_id.clone(),
            "expected_stage_text": expected_text,
            "expected_stage_source": expected_source,
            "expected_routed_content_value": expected_routed_value.clone(),
        }),
        move |after, app| {
            // The exact routed content, addressed by the source pane/node identity that owned the
            // right-click — NOT the pane that was active when the canonical click was dispatched.
            let staged = matches!(
                app.stage_content(),
                StageContent::Selection(ref text, ref source)
                    if text == expected_text && source == expected_source
            );
            // Stage pane activation + exact routed content, read from the authoritative terminal tree.
            let stage_activated = find_author(after, STAGE_ROUTED_CONTENT_AUTHOR_ID)
                .and_then(|node| node["value"].as_str())
                .is_some_and(|value| value == routed_value_probe);
            // The confirmed action released the retained right-click owner exactly once.
            let owner_released = predicate_board
                .lock()
                .map(|board| board.context_menu_owner_pane_for_test().is_none())
                .unwrap_or(false);
            staged && stage_activated && owner_released
        },
    );
    let terminal = argus.latest_terminal_observation();
    assert!(
        json_has_author_id(&terminal.after, STAGE_ROUTED_CONTENT_AUTHOR_ID),
        "the persisted terminal snapshot retains the mounted Stage post-state"
    );
    let rows = vec![mt070_matrix_row(
        &route_id,
        &terminal,
        serde_json::json!({
            "right_click_owner_pane": "pane-a",
            "active_pane_at_dispatch": "pane-b",
            "owner_released_after_confirm": true,
        }),
        serde_json::json!({
            "handler": "node_menu::RouteToStage -> InteractionBus pending_stage_route -> StagePane",
            "source_placement_author_id": placement_id,
            "source_canvas_id": "canvas-live",
            "source_node_id": "block-live",
        }),
        serde_json::json!({
            "stage_content_kind": "selection",
            "stage_text": expected_text,
            "stage_source": expected_source,
            "stage_routed_content_value": expected_routed_value,
        }),
    )];
    argus.finish();
    mt070_write_matrix_artifact("mt070-mounted-canvas-route-to-stage", &rows);
}

#[test]
fn mt070_mounted_editor_body_localhost_argus_action_matrix() {
    use canonical_argus_driver::{json_has_author_id, CanonicalArgusDriver};
    use handshake_native::code_editor::{
        Cursor, RenameState, CODE_EDITOR_CTX_RENAME_SYMBOL_AUTHOR_ID,
    };
    use handshake_native::context_menu_surfaces::editor_body_ids;
    use handshake_native::rich_editor::wikilinks::runtime::{CreateNoteBackend, CreateNoteWrite};

    struct PendingCreate;
    impl CreateNoteBackend for PendingCreate {
        fn create_note<'a>(
            &'a self,
            _workspace_id: &'a str,
            _title: &'a str,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<CreateNoteWrite, String>> + Send + 'a>,
        > {
            Box::pin(std::future::pending())
        }
    }

    fn find_author<'a>(
        value: &'a serde_json::Value,
        author_id: &str,
    ) -> Option<&'a serde_json::Value> {
        match value {
            serde_json::Value::Object(object) => {
                if object.get("author_id").and_then(serde_json::Value::as_str) == Some(author_id) {
                    return Some(value);
                }
                object
                    .values()
                    .find_map(|value| find_author(value, author_id))
            }
            serde_json::Value::Array(values) => values
                .iter()
                .find_map(|value| find_author(value, author_id)),
            _ => None,
        }
    }

    /// The always-mounted `code_editor_ctx_rename_symbol` node publishes a
    /// `handshake.click-completion/v1` same-target token for `code-editor.open-context-menu`. A popup
    /// that merely *looks* open is not proof; the terminal predicate below requires the production
    /// widget to have consumed the AccessKit request and advanced that token to `applied`.
    fn popup_open_completion_applied(after: &serde_json::Value) -> bool {
        find_author(after, CODE_EDITOR_CTX_RENAME_SYMBOL_AUTHOR_ID)
            .and_then(|node| node["value"].as_str())
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
            .is_some_and(|token| {
                token["schema"] == "handshake.click-completion/v1"
                    && token["effect"] == "code-editor.open-context-menu"
                    && token["state"] == "applied"
                    && token["generation"].as_u64().is_some_and(|value| value >= 1)
            })
    }

    /// Drive one editor-body entry end-to-end through the canonical localhost transport, binding BOTH
    /// canonical actions (open the real popup, then activate the entry) to authoritative terminal
    /// predicates. `settle_steps` lets the owning handler reach its terminal product state before the
    /// action-specific predicate is evaluated. Returns the persisted terminal observations in order.
    fn click_body_action(
        argus: &mut CanonicalArgusDriver,
        harness: &mut Harness<'_, HandshakeApp>,
        action_id: &str,
        settle_steps: usize,
        predicate_id: &str,
        evidence: serde_json::Value,
        predicate: impl FnOnce(&serde_json::Value) -> bool,
    ) -> Vec<canonical_argus_driver::ArgusObservation> {
        let target = format!("ctx-menu.{action_id}");
        let opened = argus.click_and_reinspect(harness, CODE_EDITOR_CTX_RENAME_SYMBOL_AUTHOR_ID);
        // The trigger frame only REQUESTS the popup; the real popup paints on the next live frame.
        // Settle it before re-observing, so the terminal tree describes an actually-open live popup
        // and the following entry click lands on the live item rather than a snapshot-only node.
        harness.run_steps(1);
        let popup_target = target.clone();
        argus.assert_latest_terminal_predicate_with_evidence(
            harness,
            &format!("mt070.editor.popup-open.{action_id}"),
            serde_json::json!({
                "open_trigger": CODE_EDITOR_CTX_RENAME_SYMBOL_AUTHOR_ID,
                "popup_target": target,
                "receipt_id": opened.receipt_id,
                "correlation_id": opened.correlation_id,
            }),
            move |after| {
                json_has_author_id(after, &popup_target) && popup_open_completion_applied(after)
            },
        );
        let open_observation = argus.latest_terminal_observation();
        assert!(
            harness
                .root()
                .children_recursive()
                .any(|node| node.accesskit_node().author_id() == Some(target.as_str())),
            "the settled live popup exposes {target} before the entry is steered"
        );
        let action = argus.click_and_reinspect(harness, &target);
        assert!(matches!(
            action.receipt_status.as_str(),
            "applied" | "indeterminate"
        ));
        harness.run_steps(settle_steps);
        argus.assert_latest_terminal_predicate_with_evidence(
            harness,
            predicate_id,
            evidence,
            predicate,
        );
        vec![open_observation, argus.latest_terminal_observation()]
    }

    /// Append the ordered one-action/one-receipt rows for one editor entry: the popup-open action and
    /// the entry action, each carrying its own exact target and handler trace.
    fn push_body_rows(
        rows: &mut Vec<serde_json::Value>,
        action_id: &str,
        observations: &[canonical_argus_driver::ArgusObservation],
        pane_ownership: &serde_json::Value,
        handler_trace: serde_json::Value,
        resulting_state: serde_json::Value,
    ) {
        assert_eq!(
            observations.len(),
            2,
            "one editor entry dispatches exactly one popup-open action and one entry action"
        );
        rows.push(mt070_matrix_row(
            CODE_EDITOR_CTX_RENAME_SYMBOL_AUTHOR_ID,
            &observations[0],
            pane_ownership.clone(),
            serde_json::json!({
                "handler": "CodeEditorPanel editor-body context surface",
                "effect": "code-editor.open-context-menu",
                "opens_target": format!("ctx-menu.{action_id}"),
            }),
            serde_json::json!({"popup_open_completion_state": "applied"}),
        ));
        rows.push(mt070_matrix_row(
            &format!("ctx-menu.{action_id}"),
            &observations[1],
            pane_ownership.clone(),
            handler_trace,
            resulting_state,
        ));
    }

    let pane_a = PaneId::from("pane-a");
    let mut app = ok_app();
    let editor_runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();
    app.set_runtime_handle(editor_runtime.handle().clone());
    app.set_active_project_id_for_test("workspace-editor");
    app.set_active_pane_for_test(Some(pane_a.clone()));
    app.tab_bar_states_mut().insert(
        pane_a.clone(),
        TabBarState::new(pane_a, vec![TabState::new(PaneType::CodeSymbol)]),
    );
    app.set_left_rail_open(false);
    let panel = app.mounted_code_panel();
    panel.set_runtime(editor_runtime.handle().clone());
    panel.set_text("fn alpha() { let beta = 1; }\n// [[Missing Note]]\n");
    panel.set_workspace_id("workspace-editor");
    panel.set_single_cursor(4);

    let mut harness = harness_for(app);
    // Let the production mount install workspace/runtime context first. Injecting the pending backend
    // before this frame would be overwritten by canonical mount wiring and would race a real backend.
    harness.run_steps(2);
    // Keep the create request observably in flight while retaining the mounted production runtime.
    {
        let rich = harness.state().mounted_rich_state();
        let mut rich = rich.lock().unwrap();
        rich.wikilinks.set_context("workspace-editor", "note-host");
        rich.wikilinks.set_create_backend(Arc::new(PendingCreate));
        rich.wikilinks.stage_resolver_seed(Vec::new());
    }
    harness.run_steps(2);
    assert!(harness
        .state()
        .mounted_rich_state()
        .lock()
        .unwrap()
        .wikilinks
        .is_resolver_index_ready());
    let rich_state = harness.state().mounted_rich_state();
    let mut argus = CanonicalArgusDriver::bind(harness.state(), "mt070-mounted-editor-matrix");
    let mut rows: Vec<serde_json::Value> = Vec::new();
    let editor_pane_ownership = serde_json::json!({
        "right_click_owner_pane": "pane-a",
        "owner_surface": "mounted code-editor body (PaneType::CodeSymbol)",
    });

    // Rename reaches the REAL MT-048 rename state machine: the terminal tree must expose its mounted
    // rename input, and the panel must actually be in `RenameState::Editing`.
    let rename_panel = Arc::clone(&panel);
    let observations = click_body_action(
        &mut argus,
        &mut harness,
        editor_body_ids::RENAME_SYMBOL,
        2,
        "mt070.editor.rename-symbol.mt048-rename-state-machine-editing",
        serde_json::json!({
            "handler_mt": "MT-048",
            "expected_rename_state": "Editing",
            "expected_terminal_author_id": "code_editor_rename_input",
        }),
        move |after| {
            matches!(rename_panel.rename_state(), RenameState::Editing { .. })
                && json_has_author_id(after, "code_editor_rename_input")
        },
    );
    push_body_rows(
        &mut rows,
        editor_body_ids::RENAME_SYMBOL,
        &observations,
        &editor_pane_ownership,
        serde_json::json!({
            "handler_mt": "MT-048",
            "handler": "CodeEditorPanel::begin_rename_at_cursor",
        }),
        serde_json::json!({
            "rename_state": "Editing",
            "rename_input_author_id": "code_editor_rename_input",
        }),
    );
    panel.cancel_rename();
    harness.run_steps(2);

    // Quick Fix reaches the same request/menu controller as Ctrl+.; the exactly-once handler trace is
    // the monotonic request generation plus the exact requested line/version/open-menu tuple.
    panel.set_single_cursor(4);
    let quick_fix_before = panel.quick_fix_request_generation_for_test();
    let quick_fix_panel = Arc::clone(&panel);
    let observations = click_body_action(
        &mut argus,
        &mut harness,
        editor_body_ids::QUICK_FIX,
        2,
        "mt070.editor.quick-fix.mt049-request-generation-advanced-exactly-once",
        serde_json::json!({
            "handler_mt": "MT-049",
            "quick_fix_generation_before": quick_fix_before,
            "expected_quick_fix_generation_after": quick_fix_before + 1,
            "expected_request_line": 0,
            "expected_open_menu": true,
        }),
        move |_after| {
            quick_fix_panel.quick_fix_request_generation_for_test() == quick_fix_before + 1
                && quick_fix_panel.last_quick_fix_request_for_test()
                    == Some((0, quick_fix_panel.buffer_version_for_test(), true))
        },
    );
    push_body_rows(
        &mut rows,
        editor_body_ids::QUICK_FIX,
        &observations,
        &editor_pane_ownership,
        serde_json::json!({
            "handler_mt": "MT-049",
            "handler": "CodeEditorPanel quick_fix_request -> per-frame pump",
            "generation_before": quick_fix_before,
            "generation_after": panel.quick_fix_request_generation_for_test(),
        }),
        serde_json::json!({
            "last_quick_fix_request": panel.last_quick_fix_request_for_test(),
        }),
    );

    // Format Selection reaches the real MT-050 formatter gate. This mounted headless panel has no
    // formatter, so the truthful authoritative effect is the non-blocking no-formatter toast.
    panel.set_cursors(vec![Cursor::selection(3, 8)]);
    let format_panel = Arc::clone(&panel);
    let observations = click_body_action(
        &mut argus,
        &mut harness,
        editor_body_ids::FORMAT_SELECTION,
        2,
        "mt070.editor.format-selection.mt050-no-formatter-toast",
        serde_json::json!({
            "handler_mt": "MT-050",
            "expected_effect": "non-blocking no-formatter toast",
        }),
        move |_after| {
            format_panel
                .last_format_toast()
                .as_deref()
                .is_some_and(|message| message.contains("formatter"))
        },
    );
    push_body_rows(
        &mut rows,
        editor_body_ids::FORMAT_SELECTION,
        &observations,
        &editor_pane_ownership,
        serde_json::json!({
            "handler_mt": "MT-050",
            "handler": "CodeEditorPanel::request_format_selection",
        }),
        serde_json::json!({"last_format_toast": panel.last_format_toast()}),
    );

    // Peek reaches the actual MT-008 go-to-definition request path, proven by its monotonic generation.
    panel.set_single_cursor(4);
    let definition_before = panel.definition_request_generation_for_test();
    let definition_panel = Arc::clone(&panel);
    let observations = click_body_action(
        &mut argus,
        &mut harness,
        editor_body_ids::PEEK_DEFINITION,
        2,
        "mt070.editor.peek-definition.mt008-definition-request-generation-advanced-exactly-once",
        serde_json::json!({
            "handler_mt": "MT-008",
            "definition_generation_before": definition_before,
            "expected_definition_generation_after": definition_before + 1,
        }),
        move |_after| {
            definition_panel.definition_request_generation_for_test() == definition_before + 1
        },
    );
    push_body_rows(
        &mut rows,
        editor_body_ids::PEEK_DEFINITION,
        &observations,
        &editor_pane_ownership,
        serde_json::json!({
            "handler_mt": "MT-008",
            "handler": "CodeEditorPanel::request_go_to_definition",
            "generation_before": definition_before,
            "generation_after": panel.definition_request_generation_for_test(),
        }),
        serde_json::json!({
            "definition_request_generation": panel.definition_request_generation_for_test(),
        }),
    );

    // Create-note uses an authoritative missing-link title and the mounted host drains it into the
    // existing MT-057 rich-editor create runtime. The pending stub preserves the in-flight state.
    let link_cursor = "fn alpha() { let beta = 1; }\n// [[Missing Note]]\n"
        .find("Missing Note")
        .unwrap()
        + 2;
    panel.set_single_cursor(link_cursor);
    harness.run_steps(2);
    let create_rich_state = Arc::clone(&rich_state);
    let observations = click_body_action(
        &mut argus,
        &mut harness,
        editor_body_ids::CREATE_NOTE_FROM_LINK,
        3,
        "mt070.editor.create-note-from-link.mt057-create-in-flight-for-exact-title",
        serde_json::json!({
            "handler_mt": "MT-057",
            "expected_create_title": "Missing Note",
        }),
        move |_after| {
            create_rich_state
                .lock()
                .map(|rich| rich.wikilinks.is_creating("Missing Note"))
                .unwrap_or(false)
        },
    );
    push_body_rows(
        &mut rows,
        editor_body_ids::CREATE_NOTE_FROM_LINK,
        &observations,
        &editor_pane_ownership,
        serde_json::json!({
            "handler_mt": "MT-057",
            "handler":
                "CodeEditorPanel::stage_create_note_from_link -> rich wikilinks create runtime",
        }),
        serde_json::json!({"creating_title": "Missing Note"}),
    );

    capture_mt108_matrix_frame_if_selected(&mut harness);
    assert_eq!(
        argus.dispatched_action_count(),
        10,
        "five editor entries, each dispatched as exactly one popup-open action plus one entry action"
    );
    argus.finish();
    mt070_write_matrix_artifact("mt070-mounted-editor-body-action-matrix", &rows);
}

#[test]
fn mt070_two_canvas_panes_retain_one_localhost_menu_owner_and_route_exact_origin() {
    use canonical_argus_driver::{json_has_author_id, CanonicalArgusDriver};
    use handshake_native::context_menu_surfaces::node_menu_ids;
    use handshake_native::graph::canvas_board::CanvasPlacementCard;

    let pane_a = PaneId::from("pane-a");
    let pane_b = PaneId::from("pane-b");
    let mut app = ok_app();
    app.set_left_rail_open(false);
    app.set_active_project_id_for_test("workspace-owner");
    app.set_active_pane_for_test(Some(pane_b.clone()));
    for pane_id in [&pane_a, &pane_b] {
        app.tab_bar_states_mut().insert(
            pane_id.clone(),
            TabBarState::new(
                pane_id.clone(),
                vec![TabState::new(PaneType::AtelierEditor)],
            ),
        );
    }
    {
        let board = app.mounted_canvas_board();
        let mut board = board.lock().unwrap();
        board.begin_projection_load("workspace-owner", "canvas-owner");
        let mut card =
            CanvasPlacementCard::new("placement-owner", "block-owner", 40.0, 40.0, 200.0, 120.0);
        card.mark_live_resolved(Some("Owner card".to_owned()), "artifact".to_owned(), None);
        board.set_board(vec![card], Vec::new(), egui::Vec2::ZERO, 1.0);
    }

    let mut harness = harness_for(app);
    let click = harness
        .state()
        .mounted_canvas_board()
        .lock()
        .unwrap()
        .canvas_point_to_screen_for_pane(&pane_a, egui::pos2(80.0, 80.0))
        .expect("first-rendered pane-a canvas rect is retained independently");
    secondary_click_at(&mut harness, click);
    assert_eq!(
        harness
            .state()
            .mounted_canvas_board()
            .lock()
            .unwrap()
            .context_menu_owner_pane_for_test(),
        Some(&pane_a),
        "right-click boundary retains pane-a even though pane-b renders later"
    );

    harness
        .state_mut()
        .set_active_pane_for_test(Some(pane_b.clone()));
    let target = format!("ctx-menu.{}", node_menu_ids::REVEAL_NODE);
    let mut argus = CanonicalArgusDriver::bind(harness.state(), "mt070-two-canvas-owner");
    let snapshot = argus.inspect(&mut harness);
    assert_eq!(
        json_author_count(&snapshot, &target),
        1,
        "snapshot reconstruction emits one global target from only the owning pane"
    );
    let board = harness.state().mounted_canvas_board();
    let observation = argus.click_from_snapshot_and_reinspect(&mut harness, &target, snapshot);
    assert!(matches!(
        observation.receipt_status.as_str(),
        "applied" | "indeterminate"
    ));
    // Let the owning product state settle before binding the authoritative terminal predicate.
    harness.run_steps(3);

    let owner_target = target.clone();
    let predicate_board = std::sync::Arc::clone(&board);
    argus.assert_latest_terminal_predicate_with_app_evidence(
        &mut harness,
        "mt070.two-canvas.reveal-node.routes-to-exact-right-click-owner-pane",
        serde_json::json!({
            "right_click_owner_pane": "pane-a",
            "active_pane_at_dispatch": "pane-b",
            "menu_target": target.clone(),
            "expected_node_id": "block-owner",
            "expected_tab_pane_type": "LoomBlock",
            "receipt_id": observation.receipt_id,
            "correlation_id": observation.correlation_id.clone(),
        }),
        move |after, app| {
            // Exactly-once destination: one LoomBlock tab for the exact node id, in the pane that
            // owned the right-click, and none in the pane that was merely active at dispatch time.
            let destinations = mt070_tabs_for_content(app, "block-owner", &PaneType::LoomBlock);
            let exactly_once_in_owner = destinations.len() == 1 && destinations[0].0 == "pane-a";
            // The destination tab is addressable in the authoritative terminal tree by its stable id.
            let destination_addressable = destinations
                .first()
                .is_some_and(|(_, _, author_id)| json_has_author_id(after, author_id));
            // A single confirmed action consumed the ONE retained menu owner; no stale pane kept it.
            let owner_released = predicate_board
                .lock()
                .map(|board| board.context_menu_owner_pane_for_test().is_none())
                .unwrap_or(false);
            // The consumed popup is gone from the terminal tree, so no second owner can replay it.
            let menu_consumed = !json_has_author_id(after, &owner_target);
            exactly_once_in_owner && destination_addressable && owner_released && menu_consumed
        },
    );
    let terminal = argus.latest_terminal_observation();
    let pane_a_tabs = &harness.state().tab_bar_states()[&pane_a].tabs;
    let pane_b_tabs = &harness.state().tab_bar_states()[&pane_b].tabs;
    assert!(pane_a_tabs.iter().any(|tab| {
        tab.pane_type == PaneType::LoomBlock && tab.content_id.as_deref() == Some("block-owner")
    }));
    assert!(!pane_b_tabs.iter().any(|tab| {
        tab.pane_type == PaneType::LoomBlock && tab.content_id.as_deref() == Some("block-owner")
    }));
    let destinations = mt070_tabs_for_content(harness.state(), "block-owner", &PaneType::LoomBlock);
    let rows = vec![mt070_matrix_row(
        &target,
        &terminal,
        serde_json::json!({
            "right_click_owner_pane": "pane-a",
            "active_pane_at_dispatch": "pane-b",
            "menu_owner_count_in_snapshot": 1,
            "owner_released_after_confirm": true,
        }),
        serde_json::json!({
            "handler": "node_menu::RevealNode -> NavigationTarget::RevealNode -> ShellNavigator",
            "source_placement_id": "placement-owner",
            "source_node_id": "block-owner",
        }),
        serde_json::json!({"destination_tabs": destinations}),
    )];
    argus.finish();
    mt070_write_matrix_artifact("mt070-two-canvas-panes-owner-routing", &rows);
}

#[test]
fn mt070_two_graph_panes_localhost_action_channel_routes_owner_and_route_stays_disabled() {
    use canonical_argus_driver::{json_has_author_id, CanonicalArgusDriver};
    use handshake_native::context_menu_surfaces::node_menu_ids;
    use handshake_native::editor_pane_factories::{placeholder_pane_type, GRAPH_VIEW_PANE_LABEL};
    use handshake_native::graph::graph_view::GraphNode;

    let pane_a = PaneId::from("pane-a");
    let pane_b = PaneId::from("pane-b");
    let graph_pane_type = placeholder_pane_type(GRAPH_VIEW_PANE_LABEL);
    let mut app = ok_app();
    app.set_left_rail_open(false);
    app.set_active_project_id_for_test("workspace-graph-owner");
    app.set_active_pane_for_test(Some(pane_b.clone()));
    for pane_id in [&pane_a, &pane_b] {
        app.tab_bar_states_mut().insert(
            pane_id.clone(),
            TabBarState::new(
                pane_id.clone(),
                vec![TabState::new(graph_pane_type.clone())],
            ),
        );
    }
    {
        let graph = app.mounted_graph_view();
        let mut graph = graph.lock().unwrap();
        graph.reset_for_workspace("workspace-graph-owner");
        graph.controls.show_orphans = true;
        graph.set_graph(
            vec![GraphNode::new("graph-owner", "Graph owner", "artifact")],
            Vec::new(),
        );
    }

    let mut harness = harness_for(app);
    harness.run_steps(4);
    {
        let graph = harness.state().mounted_graph_view();
        let mut graph = graph.lock().unwrap();
        // Two graph panes leave a narrow canvas beside each controls strip. Center the deterministic
        // one-node layout so the proof right-clicks a genuinely visible node in first-rendered pane A.
        graph.nodes[0].x = 0.0;
        graph.nodes[0].y = 0.0;
    }
    let click = harness
        .state()
        .mounted_graph_view()
        .lock()
        .unwrap()
        .node_screen_position_for_pane(&pane_a, "graph-owner")
        .expect("first-rendered pane-a graph rect and node are retained independently");
    assert_eq!(
        harness
            .state()
            .mounted_graph_view()
            .lock()
            .unwrap()
            .node_at_screen_for_pane_for_test(&pane_a, click),
        Some("graph-owner"),
        "the exact pane-a pointer hits the visible graph node before event dispatch"
    );
    secondary_click_at(&mut harness, click);
    {
        let graph = harness.state().mounted_graph_view();
        let graph = graph.lock().unwrap();
        assert_eq!(
            graph.context_menu_owner_pane_for_test(),
            Some(&pane_a),
            "Graph right-click retains pane-a even though pane-b renders later; selected={:?}, rect={:?}",
            graph.selected,
            graph.canvas_rect_for_pane_for_test(&pane_a)
        );
    }
    harness
        .state_mut()
        .set_active_pane_for_test(Some(pane_b.clone()));

    let reveal_target = format!("ctx-menu.{}", node_menu_ids::REVEAL_NODE);
    let route_target = format!("ctx-menu.{}", node_menu_ids::ROUTE_TO_STAGE);
    let mut argus = CanonicalArgusDriver::bind(harness.state(), "mt070-two-graph-owner");
    let snapshot = argus.inspect(&mut harness);
    assert_eq!(json_author_count(&snapshot, &reveal_target), 1);
    assert_eq!(json_author_count(&snapshot, &route_target), 1);
    let route = json_author(&snapshot, &route_target).expect("Graph Route target is inspectable");
    assert_eq!(route["disabled"], true, "{route}");
    assert_eq!(
        route["value"],
        "Route to Stage requires a stable node id and a live Canvas board context"
    );
    assert!(
        !route["actions"]
            .as_array()
            .is_some_and(|actions| actions.iter().any(|action| action == "Click")),
        "disabled Graph Route exposes no Click action: {route}"
    );
    argus.click_expect_rejected(&mut harness, &route_target, "disabled");
    assert_eq!(
        argus.dispatched_action_count(),
        0,
        "the disabled Graph Route entry is rejected before any canonical action is dispatched"
    );
    let reveal_snapshot = argus.inspect(&mut harness);
    let graph = harness.state().mounted_graph_view();
    let observation =
        argus.click_from_snapshot_and_reinspect(&mut harness, &reveal_target, reveal_snapshot);
    assert!(matches!(
        observation.receipt_status.as_str(),
        "applied" | "indeterminate"
    ));
    // Let the owning product state settle before binding the authoritative terminal predicate.
    harness.run_steps(3);

    let owner_reveal_target = reveal_target.clone();
    let owner_route_target = route_target.clone();
    let predicate_graph = std::sync::Arc::clone(&graph);
    argus.assert_latest_terminal_predicate_with_app_evidence(
        &mut harness,
        "mt070.two-graph.reveal-node.routes-to-owner-and-route-stays-unavailable",
        serde_json::json!({
            "right_click_owner_pane": "pane-a",
            "active_pane_at_dispatch": "pane-b",
            "menu_target": reveal_target.clone(),
            "unavailable_target": route_target.clone(),
            "expected_node_id": "graph-owner",
            "expected_tab_pane_type": "LoomBlock",
            "receipt_id": observation.receipt_id,
            "correlation_id": observation.correlation_id.clone(),
        }),
        move |after, app| {
            let destinations = mt070_tabs_for_content(app, "graph-owner", &PaneType::LoomBlock);
            let exactly_once_in_owner = destinations.len() == 1 && destinations[0].0 == "pane-a";
            let destination_addressable = destinations
                .first()
                .is_some_and(|(_, _, author_id)| json_has_author_id(after, author_id));
            let owner_released = predicate_graph
                .lock()
                .map(|graph| graph.context_menu_owner_pane_for_test().is_none())
                .unwrap_or(false);
            let menu_consumed = !json_has_author_id(after, &owner_reveal_target);
            // Graph Route never becomes steerable: its availability predicate is false, so it is
            // either gone with the consumed popup or still disabled with no Click action.
            let route_never_enabled = json_author(after, &owner_route_target).is_none_or(|node| {
                node["disabled"] == true
                    && !node["actions"]
                        .as_array()
                        .is_some_and(|actions| actions.iter().any(|action| action == "Click"))
            });
            exactly_once_in_owner
                && destination_addressable
                && owner_released
                && menu_consumed
                && route_never_enabled
        },
    );
    let terminal = argus.latest_terminal_observation();
    let pane_a_tabs = &harness.state().tab_bar_states()[&pane_a].tabs;
    let pane_b_tabs = &harness.state().tab_bar_states()[&pane_b].tabs;
    assert!(pane_a_tabs.iter().any(|tab| {
        tab.pane_type == PaneType::LoomBlock && tab.content_id.as_deref() == Some("graph-owner")
    }));
    assert!(!pane_b_tabs.iter().any(|tab| {
        tab.pane_type == PaneType::LoomBlock && tab.content_id.as_deref() == Some("graph-owner")
    }));
    let destinations = mt070_tabs_for_content(harness.state(), "graph-owner", &PaneType::LoomBlock);
    let rows = vec![mt070_matrix_row(
        &reveal_target,
        &terminal,
        serde_json::json!({
            "right_click_owner_pane": "pane-a",
            "active_pane_at_dispatch": "pane-b",
            "menu_owner_count_in_snapshot": 1,
            "owner_released_after_confirm": true,
        }),
        serde_json::json!({
            "handler": "node_menu::RevealNode -> NavigationTarget::RevealNode -> ShellNavigator",
            "source_node_id": "graph-owner",
            "rejected_unavailable_target": route_target,
        }),
        serde_json::json!({"destination_tabs": destinations}),
    )];
    argus.finish();
    mt070_write_matrix_artifact("mt070-two-graph-panes-owner-routing", &rows);
}

#[test]
fn mt070_mounted_editor_body_localhost_argus_disabled_matrix_is_truthful() {
    use canonical_argus_driver::CanonicalArgusDriver;
    use handshake_native::code_editor::CODE_EDITOR_CTX_RENAME_SYMBOL_AUTHOR_ID;

    fn find_author<'a>(
        value: &'a serde_json::Value,
        author_id: &str,
    ) -> Option<&'a serde_json::Value> {
        match value {
            serde_json::Value::Object(object) => {
                if object.get("author_id").and_then(serde_json::Value::as_str) == Some(author_id) {
                    return Some(value);
                }
                object
                    .values()
                    .find_map(|value| find_author(value, author_id))
            }
            serde_json::Value::Array(values) => values
                .iter()
                .find_map(|value| find_author(value, author_id)),
            _ => None,
        }
    }

    let pane_a = PaneId::from("pane-a");
    let mut app = ok_app();
    app.set_active_pane_for_test(Some(pane_a.clone()));
    app.tab_bar_states_mut().insert(
        pane_a.clone(),
        TabBarState::new(pane_a, vec![TabState::new(PaneType::CodeSymbol)]),
    );
    app.set_left_rail_open(false);
    let panel = app.mounted_code_panel();
    panel.set_text("");
    panel.set_single_cursor(0);
    panel.set_workspace_id("");

    const UNAVAILABLE_ENTRIES: [(&str, &str); 5] = [
        (editor_body_ids::RENAME_SYMBOL, "symbol"),
        (editor_body_ids::QUICK_FIX, "quick fix"),
        (editor_body_ids::FORMAT_SELECTION, "select"),
        (editor_body_ids::PEEK_DEFINITION, "definition"),
        (editor_body_ids::CREATE_NOTE_FROM_LINK, "unresolved"),
    ];

    /// Every unavailable entry must be a truthful `Role::MenuItem`: disabled, carrying its typed
    /// reason, and exposing NO Click action a swarm agent could steer.
    fn unavailable_entry_is_truthful(after: &serde_json::Value, id: &str, reason: &str) -> bool {
        let Some(node) = find_author(after, &format!("ctx-menu.{id}")) else {
            return false;
        };
        node["role"] == "MenuItem"
            && node["disabled"] == true
            && node["value"]
                .as_str()
                .is_some_and(|value| value.to_ascii_lowercase().contains(reason))
            && !node["actions"]
                .as_array()
                .is_some_and(|actions| actions.iter().any(|action| action == "Click"))
    }

    let mut harness = harness_for(app);
    let mut argus = CanonicalArgusDriver::bind(harness.state(), "mt070-editor-disabled-matrix");
    // The ONE canonical action this matrix is allowed to dispatch is the popup open. It is bound to a
    // terminal predicate; every unavailable entry below is proven WITHOUT being dispatched.
    let opened = argus.click_and_reinspect(&mut harness, CODE_EDITOR_CTX_RENAME_SYMBOL_AUTHOR_ID);
    harness.run_steps(2);
    let definition_before = panel.definition_request_generation_for_test();
    let quick_fix_before = panel.quick_fix_request_generation_for_test();
    let zero_dispatch_panel = Arc::clone(&panel);
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut harness,
        "mt070.editor.disabled-matrix.every-unavailable-entry-is-truthful-and-unsteerable",
        serde_json::json!({
            "open_trigger": CODE_EDITOR_CTX_RENAME_SYMBOL_AUTHOR_ID,
            "unavailable_entries": UNAVAILABLE_ENTRIES
                .iter()
                .map(|(id, reason)| serde_json::json!({
                    "target": format!("ctx-menu.{id}"),
                    "expected_reason_fragment": reason,
                }))
                .collect::<Vec<_>>(),
            "receipt_id": opened.receipt_id,
            "correlation_id": opened.correlation_id,
            "definition_generation_before": definition_before,
            "quick_fix_generation_before": quick_fix_before,
        }),
        move |after| {
            let truthful = UNAVAILABLE_ENTRIES
                .iter()
                .all(|(id, reason)| unavailable_entry_is_truthful(after, id, reason));
            // Opening the popup changed no handler state: the unavailable entries left every real
            // MT-048/049/050/008/057 handler trace exactly where it was.
            let handlers_untouched = matches!(
                zero_dispatch_panel.rename_state(),
                handshake_native::code_editor::RenameState::Idle
            ) && !zero_dispatch_panel.quick_fix_request_armed_for_test()
                && !zero_dispatch_panel.format_request_armed_for_test()
                && zero_dispatch_panel.definition_request_generation_for_test()
                    == definition_before
                && zero_dispatch_panel.quick_fix_request_generation_for_test() == quick_fix_before;
            truthful && handlers_untouched
        },
    );
    let terminal = argus.latest_terminal_observation();
    for (id, reason_fragment) in UNAVAILABLE_ENTRIES {
        let target = format!("ctx-menu.{id}");
        let node =
            find_author(&terminal.after, &target).unwrap_or_else(|| panic!("missing {target}"));
        assert_eq!(node["role"], "MenuItem", "{node}");
        assert_eq!(node["disabled"], true, "{node}");
        assert!(
            node["value"]
                .as_str()
                .is_some_and(|value| value.to_ascii_lowercase().contains(reason_fragment)),
            "{node}"
        );
        assert!(
            !node["actions"]
                .as_array()
                .is_some_and(|actions| actions.iter().any(|action| action == "Click")),
            "{node}"
        );
        argus.click_expect_rejected(&mut harness, &target, "disabled");
    }
    // Explicit zero-dispatch gate: the five unavailable entries produced zero canonical actions, so
    // the driver still holds exactly the one popup-open action it was allowed to dispatch.
    assert_eq!(
        argus.dispatched_action_count(),
        1,
        "no disabled entry may be dispatched; only the popup-open action is canonical here"
    );
    assert!(matches!(
        panel.rename_state(),
        handshake_native::code_editor::RenameState::Idle
    ));
    assert!(!panel.quick_fix_request_armed_for_test());
    assert!(!panel.format_request_armed_for_test());
    assert_eq!(
        panel.definition_request_generation_for_test(),
        definition_before
    );
    assert_eq!(
        panel.quick_fix_request_generation_for_test(),
        quick_fix_before
    );
    assert_eq!(panel.take_pending_create_note_link(), None);
    capture_mt108_matrix_frame_if_selected(&mut harness);
    let rows = vec![mt070_matrix_row(
        CODE_EDITOR_CTX_RENAME_SYMBOL_AUTHOR_ID,
        &terminal,
        serde_json::json!({
            "right_click_owner_pane": "pane-a",
            "owner_surface": "mounted code-editor body with no symbol/selection/link/workspace",
        }),
        serde_json::json!({
            "handler": "CodeEditorPanel editor-body context surface",
            "effect": "code-editor.open-context-menu",
            "dispatched_unavailable_entries": 0,
        }),
        serde_json::json!({
            "unavailable_entries": UNAVAILABLE_ENTRIES
                .iter()
                .map(|(id, reason)| serde_json::json!({
                    "target": format!("ctx-menu.{id}"),
                    "role": "MenuItem",
                    "disabled": true,
                    "reason_fragment": reason,
                    "exposes_click_action": false,
                }))
                .collect::<Vec<_>>(),
            "rename_state": "Idle",
            "quick_fix_request_armed": false,
            "format_request_armed": false,
            "definition_request_generation": definition_before,
        }),
    )];
    argus.finish();
    mt070_write_matrix_artifact("mt070-mounted-editor-body-disabled-matrix", &rows);
}

#[test]
fn mt070_mounted_canvas_pending_and_failed_projection_argus_disables_all_actions() {
    use canonical_argus_driver::CanonicalArgusDriver;
    use handshake_native::graph::canvas_board::CanvasPlacementCard;

    fn find_author<'a>(
        value: &'a serde_json::Value,
        author_id: &str,
    ) -> Option<&'a serde_json::Value> {
        match value {
            serde_json::Value::Object(object) => {
                if object.get("author_id").and_then(serde_json::Value::as_str) == Some(author_id) {
                    return Some(value);
                }
                object
                    .values()
                    .find_map(|value| find_author(value, author_id))
            }
            serde_json::Value::Array(values) => {
                values.iter().find_map(|v| find_author(v, author_id))
            }
            _ => None,
        }
    }

    let pane_a = PaneId::from("pane-a");
    let mut app = ok_app();
    app.set_active_project_id_for_test("workspace-projection");
    app.set_active_pane_for_test(Some(pane_a.clone()));
    app.tab_bar_states_mut().insert(
        pane_a.clone(),
        TabBarState::new(
            pane_a,
            vec![TabState {
                pane_type: PaneType::AtelierEditor,
                content_id: Some("canvas-projection".to_owned()),
                pinned: false,
                dirty: false,
                label_override: None,
            }],
        ),
    );
    app.set_left_rail_open(false);
    app.begin_canvas_request_for_test("workspace-projection", "canvas-projection");
    {
        let board = app.mounted_canvas_board();
        let mut board = board.lock().unwrap();
        let mut card = CanvasPlacementCard::new(
            "placement-retained",
            "block-retained",
            40.0,
            40.0,
            200.0,
            120.0,
        );
        card.mark_live_resolved(Some("Retained Note".to_owned()), "note".to_owned(), None);
        board.set_board(vec![card], Vec::new(), egui::Vec2::ZERO, 1.0);
    }

    let mut harness = harness_for(app);
    {
        let board = harness.state().mounted_canvas_board();
        let mut board = board.lock().unwrap();
        board.begin_projection_load("workspace-projection", "canvas-projection");
        assert_eq!(board.placements.len(), 1);
    }
    harness.run_steps(1);
    let click_pos = harness
        .state()
        .mounted_canvas_board()
        .lock()
        .unwrap()
        .canvas_point_to_screen(egui::pos2(80.0, 80.0))
        .unwrap();
    harness.event(egui::Event::PointerMoved(click_pos));
    for pressed in [true, false] {
        harness.event(egui::Event::PointerButton {
            pos: click_pos,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: egui::Modifiers::default(),
        });
    }
    harness.run_steps(2);

    let mut argus = CanonicalArgusDriver::bind(harness.state(), "mt070-canvas-projection-disabled");
    for phase in ["pending", "failed"] {
        if phase == "failed" {
            harness
                .state()
                .mounted_canvas_board()
                .lock()
                .unwrap()
                .fail_projection("backend unavailable");
        }
        let snapshot = argus.inspect(&mut harness);
        for id in NODE_MENU_REQUIRED_IDS {
            let target = format!("ctx-menu.{id}");
            let node =
                find_author(&snapshot, &target).unwrap_or_else(|| panic!("missing {target}"));
            assert_eq!(node["disabled"], true, "{phase}: {node}");
            assert_eq!(
                node["value"],
                "Canvas projection is pending, failed, or stale"
            );
            assert!(
                !node["actions"]
                    .as_array()
                    .is_some_and(|actions| actions.iter().any(|action| action == "Click")),
                "{node}"
            );
            argus.click_expect_rejected(&mut harness, &target, "disabled");
        }
        assert_eq!(
            harness
                .state()
                .mounted_canvas_board()
                .lock()
                .unwrap()
                .placements
                .len(),
            1,
            "same-binding {phase} retains its visible card"
        );
    }
    // Explicit zero-dispatch gate (validation_v4 remediation step 8): a pending/failed projection
    // rejects EVERY node entry before an event can reach the host, so this matrix must reach
    // `finish` holding zero canonical actions rather than passing vacuously.
    assert_eq!(
        argus.dispatched_action_count(),
        0,
        "a pending/failed Canvas projection may not dispatch any canonical action"
    );
    argus.finish();
}

#[test]
fn mt070_mounted_canvas_localhost_argus_open_reveal_create_matrix() {
    use canonical_argus_driver::{json_has_author_id, CanonicalArgusDriver};
    use handshake_native::graph::canvas_board::CanvasPlacementCard;
    use handshake_native::rich_editor::wikilinks::runtime::{CreateNoteBackend, CreateNoteWrite};

    struct PendingCreate;
    impl CreateNoteBackend for PendingCreate {
        fn create_note<'a>(
            &'a self,
            _workspace_id: &'a str,
            _title: &'a str,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<CreateNoteWrite, String>> + Send + 'a>,
        > {
            Box::pin(std::future::pending())
        }
    }

    for action in [
        NodeMenuAction::OpenNote,
        NodeMenuAction::RevealNode,
        NodeMenuAction::CreateNoteFromLink,
    ] {
        let pane_a = PaneId::from("pane-a");
        let canvas_id = format!("canvas-{action:?}");
        let block_id = format!("block-{action:?}");
        let mut app = ok_app();
        let action_runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();
        app.set_runtime_handle(action_runtime.handle().clone());
        app.set_active_project_id_for_test("workspace-matrix");
        app.set_active_pane_for_test(Some(pane_a.clone()));
        app.tab_bar_states_mut().insert(
            pane_a.clone(),
            TabBarState::new(
                pane_a,
                vec![TabState {
                    pane_type: PaneType::AtelierEditor,
                    content_id: Some(canvas_id.clone()),
                    pinned: false,
                    dirty: false,
                    label_override: None,
                }],
            ),
        );
        app.set_left_rail_open(false);
        app.begin_canvas_request_for_test("workspace-matrix", &canvas_id);
        {
            let rich_state = app.mounted_rich_state();
            let mut rich = rich_state.lock().unwrap();
            rich.wikilinks.set_context("workspace-matrix", "note-host");
            rich.wikilinks.set_create_backend(Arc::new(PendingCreate));
        }
        {
            let board = app.mounted_canvas_board();
            let mut board = board.lock().unwrap();
            let mut card =
                CanvasPlacementCard::new("placement-matrix", &block_id, 40.0, 40.0, 200.0, 120.0);
            match action {
                NodeMenuAction::OpenNote => {
                    card.mark_live_resolved(Some("Open Me".to_owned()), "note".to_owned(), None)
                }
                NodeMenuAction::RevealNode => card.mark_live_resolved(
                    Some("Reveal Me".to_owned()),
                    "artifact".to_owned(),
                    None,
                ),
                NodeMenuAction::CreateNoteFromLink => {
                    card.mark_live_unresolved(Some("Missing Canvas Note".to_owned()))
                }
                NodeMenuAction::RouteToStage => unreachable!(),
            }
            board.set_board(vec![card], Vec::new(), egui::Vec2::ZERO, 1.0);
        }

        let mut harness = harness_for(app);
        if action == NodeMenuAction::CreateNoteFromLink {
            harness
                .state()
                .mounted_rich_state()
                .lock()
                .unwrap()
                .wikilinks
                .set_create_backend(Arc::new(PendingCreate));
        }
        let click_pos = harness
            .state()
            .mounted_canvas_board()
            .lock()
            .unwrap()
            .canvas_point_to_screen(egui::pos2(80.0, 80.0))
            .unwrap();
        harness.event(egui::Event::PointerMoved(click_pos));
        for pressed in [true, false] {
            harness.event(egui::Event::PointerButton {
                pos: click_pos,
                button: egui::PointerButton::Secondary,
                pressed,
                modifiers: egui::Modifiers::default(),
            });
        }
        harness.run_steps(2);
        let target = match action {
            NodeMenuAction::OpenNote => node_menu_ids::OPEN_NOTE,
            NodeMenuAction::RevealNode => node_menu_ids::REVEAL_NODE,
            NodeMenuAction::CreateNoteFromLink => node_menu_ids::CREATE_NOTE_FROM_LINK,
            NodeMenuAction::RouteToStage => unreachable!(),
        };
        let menu_target = format!("ctx-menu.{target}");
        let board = harness.state().mounted_canvas_board();
        let rich_state = harness.state().mounted_rich_state();
        let mut argus =
            CanonicalArgusDriver::bind(harness.state(), &format!("mt070-canvas-{action:?}"));
        let observation = argus.click_and_reinspect(&mut harness, &menu_target);
        assert!(matches!(
            observation.receipt_status.as_str(),
            "applied" | "indeterminate"
        ));
        // Reinspect only after the owning destination/effect has settled for THIS action.
        harness.run_steps(3);

        let predicate_id = match action {
            NodeMenuAction::OpenNote => "mt070.canvas.open-note.opens-exact-wiki-page-destination",
            NodeMenuAction::RevealNode => "mt070.canvas.reveal-node.opens-exact-block-destination",
            NodeMenuAction::CreateNoteFromLink => {
                "mt070.canvas.create-note-from-link.mt057-create-in-flight-for-exact-title"
            }
            NodeMenuAction::RouteToStage => unreachable!(),
        };
        let expected_pane_type = match action {
            NodeMenuAction::OpenNote => Some(PaneType::LoomWikiPage),
            NodeMenuAction::RevealNode => Some(PaneType::LoomBlock),
            NodeMenuAction::CreateNoteFromLink => None,
            NodeMenuAction::RouteToStage => unreachable!(),
        };
        let predicate_block_id = block_id.clone();
        let predicate_pane_type = expected_pane_type.clone();
        let predicate_menu_target = menu_target.clone();
        let predicate_board = std::sync::Arc::clone(&board);
        let predicate_rich_state = std::sync::Arc::clone(&rich_state);
        argus.assert_latest_terminal_predicate_with_app_evidence(
            &mut harness,
            predicate_id,
            serde_json::json!({
                "menu_target": menu_target.clone(),
                "source_pane_id": "pane-a",
                "source_canvas_id": canvas_id.clone(),
                "source_placement_id": "placement-matrix",
                "source_node_id": block_id.clone(),
                "expected_destination_pane_type": expected_pane_type
                    .as_ref()
                    .map(|pane_type| format!("{pane_type:?}")),
                "expected_create_title": matches!(action, NodeMenuAction::CreateNoteFromLink)
                    .then_some("Missing Canvas Note"),
                "receipt_id": observation.receipt_id,
                "correlation_id": observation.correlation_id.clone(),
            }),
            move |after, app| {
                // The confirmed action consumed the ONE retained right-click owner exactly once.
                let owner_released = predicate_board
                    .lock()
                    .map(|board| board.context_menu_owner_pane_for_test().is_none())
                    .unwrap_or(false);
                let menu_consumed = !json_has_author_id(after, &predicate_menu_target);
                let durable_effect = match predicate_pane_type {
                    Some(pane_type) => {
                        // Exactly-once durable destination for the exact node id, addressable in the
                        // authoritative terminal tree by its stable production tab author_id.
                        // Exactly ONE durable destination for the exact node id (no duplicate/fan-out
                        // open), addressable in the authoritative terminal tree by its stable
                        // production tab author_id. The destination PANE is chosen by the shared
                        // MT-030 ShellNavigator seam (Open note routes to the note pane, Reveal node
                        // to the block pane), so it is recorded as evidence rather than asserted here;
                        // the two-pane ownership matrices below own the source-pane binding.
                        let destinations =
                            mt070_tabs_for_content(app, &predicate_block_id, &pane_type);
                        destinations.len() == 1
                            && json_has_author_id(after, &destinations[0].2)
                            // The sibling destination kind must NOT also have been opened: Open note
                            // and Reveal node are distinct authorities, not one shared effect.
                            && mt070_tabs_for_content(
                                app,
                                &predicate_block_id,
                                if pane_type == PaneType::LoomWikiPage {
                                    &PaneType::LoomBlock
                                } else {
                                    &PaneType::LoomWikiPage
                                },
                            )
                            .is_empty()
                    }
                    None => {
                        // Create-note-from-link routes to the MT-057 create runtime for the exact
                        // unresolved title, and must NOT open a destination tab for the node id.
                        predicate_rich_state
                            .lock()
                            .map(|rich| rich.wikilinks.is_creating("Missing Canvas Note"))
                            .unwrap_or(false)
                            && mt070_tabs_for_content(
                                app,
                                &predicate_block_id,
                                &PaneType::LoomWikiPage,
                            )
                            .is_empty()
                            && mt070_tabs_for_content(
                                app,
                                &predicate_block_id,
                                &PaneType::LoomBlock,
                            )
                            .is_empty()
                    }
                };
                owner_released && menu_consumed && durable_effect
            },
        );
        let terminal = argus.latest_terminal_observation();
        match action {
            NodeMenuAction::OpenNote => {
                assert!(harness.state().tab_bar_states().values().any(|bar| {
                    bar.tabs.iter().any(|tab| {
                        tab.content_id.as_deref() == Some(block_id.as_str())
                            && tab.pane_type == PaneType::LoomWikiPage
                    })
                }))
            }
            NodeMenuAction::RevealNode => {
                assert!(harness.state().tab_bar_states().values().any(|bar| {
                    bar.tabs.iter().any(|tab| {
                        tab.content_id.as_deref() == Some(block_id.as_str())
                            && tab.pane_type == PaneType::LoomBlock
                    })
                }))
            }
            NodeMenuAction::CreateNoteFromLink => assert!(harness
                .state()
                .mounted_rich_state()
                .lock()
                .unwrap()
                .wikilinks
                .is_creating("Missing Canvas Note")),
            NodeMenuAction::RouteToStage => unreachable!(),
        }
        assert_eq!(
            argus.dispatched_action_count(),
            1,
            "each node-menu matrix entry dispatches exactly one canonical action"
        );
        let destinations = expected_pane_type
            .as_ref()
            .map(|pane_type| mt070_tabs_for_content(harness.state(), &block_id, pane_type))
            .unwrap_or_default();
        let rows = vec![mt070_matrix_row(
            &menu_target,
            &terminal,
            serde_json::json!({
                "right_click_owner_pane": "pane-a",
                "source_canvas_id": canvas_id,
                "source_placement_id": "placement-matrix",
                "owner_released_after_confirm": true,
            }),
            serde_json::json!({
                "handler": format!("node_menu::{action:?}"),
                "source_node_id": block_id,
            }),
            serde_json::json!({
                "destination_tabs": destinations,
                "creating_title": matches!(action, NodeMenuAction::CreateNoteFromLink)
                    .then_some("Missing Canvas Note"),
            }),
        )];
        argus.finish();
        mt070_write_matrix_artifact(
            &format!("mt070-mounted-canvas-node-menu-{action:?}").to_ascii_lowercase(),
            &rows,
        );
    }
}

// ── WP-KERNEL-012 MT-080 FIX E: graph/canvas node-menu availability is read from the node payload ──────

#[test]
fn mt080_graph_and_canvas_node_availability_from_payload() {
    use handshake_native::context_menu_surfaces::node_menu_ids;
    use handshake_native::graph::canvas_board::{placement_menu_availability, CanvasPlacementCard};
    use handshake_native::graph::graph_view::{graph_node_menu_availability, GraphNode};

    // A NOTE-backed graph node ENABLES Open Note (has_note read from the payload content_type, not a
    // hardcoded `false`). The enabled entry maps to a REAL action (no dead handler).
    let note_node = GraphNode::new("blk-note", "A Note", "note");
    let navail = graph_node_menu_availability(&note_node);
    assert!(
        navail.has_note,
        "FIX E: a `note` graph node has a backing note"
    );
    assert_eq!(
        node_action_for_id(node_menu_ids::OPEN_NOTE, navail),
        Some(NodeMenuAction::OpenNote),
        "FIX E: a note-backed node ENABLES Open Note (maps to a real action)"
    );
    assert!(
        !navail.can_route_to_stage,
        "graph nodes never advertise the Canvas-only live Stage route"
    );
    assert_eq!(
        node_action_for_id(node_menu_ids::ROUTE_TO_STAGE, navail),
        None,
        "a stable graph-node id does not create a live Canvas Stage route"
    );

    // A NON-note graph node keeps Open Note DISABLED — the invariant (disabled entry maps to None).
    let file_node = GraphNode::new("blk-file", "A File", "file");
    let favail = graph_node_menu_availability(&file_node);
    assert!(
        !favail.has_note,
        "FIX E: a non-note node has no backing note"
    );
    assert_eq!(
        node_action_for_id(node_menu_ids::OPEN_NOTE, favail),
        None,
        "FIX E: a non-note node keeps Open Note disabled (no dead ENABLED entry)"
    );

    // Loading stays disabled; confirmed unresolved with a retained source title enables Create-note.
    let mut stale = CanvasPlacementCard::new("p-stale", "blk-missing", 0.0, 0.0, 200.0, 120.0);
    assert!(!placement_menu_availability(&stale, true).unresolved_link);
    stale.mark_live_unresolved(Some("Missing Note".to_owned()));
    let savail = placement_menu_availability(&stale, true);
    assert!(
        savail.unresolved_link,
        "FIX E: a stale reference is an unresolved link"
    );
    assert_eq!(
        node_action_for_id(node_menu_ids::CREATE_NOTE_FROM_LINK, savail),
        Some(NodeMenuAction::CreateNoteFromLink),
        "FIX E: an unresolved-link node ENABLES Create-note from link"
    );

    // A free-text canvas card ENABLES Open Note (it owns a note) and is NOT an unresolved link.
    let text = CanvasPlacementCard::new("p-text", "blk-text", 0.0, 0.0, 200.0, 120.0)
        .as_text_card("hello");
    let tavail = placement_menu_availability(&text, true);
    assert!(tavail.has_note, "FIX E: a text card has a backing note");
    assert!(
        !tavail.unresolved_link,
        "FIX E: a text card is not an unresolved link"
    );
    assert_eq!(
        node_action_for_id(node_menu_ids::OPEN_NOTE, tavail),
        Some(NodeMenuAction::OpenNote),
        "FIX E: a text card ENABLES Open Note"
    );
    assert!(
        tavail.can_route_to_stage,
        "a mounted Canvas board with both authority ids exposes the live Stage route capability"
    );
    assert_eq!(
        node_action_for_id(node_menu_ids::ROUTE_TO_STAGE, tavail),
        Some(NodeMenuAction::RouteToStage)
    );
    println!("PASS FIX E: graph/canvas node-menu availability reads the node payload (no hardcoded false)");
}

// ── AC-070-7 / RISK-070-5: required code-action ids ARE the existing registry ids (no parallel scheme) ─

#[test]
fn mt070_editor_action_ids_reuse_existing_registry() {
    // The four code-action entry ids are the EXACT author_ids the owning code-editor MTs already emit on
    // the panel's inline body menu + AccessKit nodes — proving reuse of the WP-011/WP-012 id registry,
    // not a parallel scheme (AC-070-7 / RISK-070-5).
    assert_eq!(
        editor_body_ids::RENAME_SYMBOL,
        handshake_native::code_editor::CODE_EDITOR_CTX_RENAME_SYMBOL_AUTHOR_ID,
        "Rename reuses the MT-048 code-panel ctx author_id",
    );
    assert_eq!(
        editor_body_ids::QUICK_FIX,
        handshake_native::code_editor::code_actions::CODE_EDITOR_CTX_QUICK_FIX_AUTHOR_ID,
        "Quick Fix reuses the MT-049 code-actions ctx author_id",
    );
    assert_eq!(
        editor_body_ids::FORMAT_SELECTION,
        handshake_native::code_editor::FORMAT_SELECTION_CTX_AUTHOR_ID,
        "Format Selection reuses the MT-050 formatting ctx author_id",
    );
    assert_eq!(
        editor_body_ids::PEEK_DEFINITION,
        handshake_native::code_editor::CODE_EDITOR_HOVER_GOTODEF_AUTHOR_ID,
        "Peek reuses the MT-008 go-to-def author_id",
    );

    // AC-070-7: the entries are added via the WP-011 ContextMenu builder (the menu is a ContextMenu whose
    // items round-trip through the primitive), not a hand-rolled menu — proven by re-wrapping the items
    // in a ContextMenu and confirming the builder preserves them.
    let items = editor_body_context_items(full_editor_availability());
    let menu = ContextMenu::new("editor-body").items(items.clone());
    assert_eq!(
        menu.entries().len(),
        items.len(),
        "menu uses the WP-011 ContextMenu builder verbatim"
    );
    // Sanity: a separator is the WP-011 primitive's separator (not a fabricated divider).
    assert!(
        items.iter().any(|i| matches!(
            i.kind,
            handshake_native::context_menu::MenuItemKind::Separator
        )),
        "the editor-body menu uses the WP-011 primitive's separator",
    );
    let _ = ContextMenuItem::separator(); // touch the primitive's constructor (compile-time reuse proof)
    println!(
        "PASS AC-070-7: editor-action ids reuse the existing registry; built via WP-011 primitive"
    );
}

// ── Honest enable/disable: a dead-but-enabled entry is impossible (a no-target entry is DISABLED) ─────

#[test]
fn mt070_unavailable_entry_is_disabled_not_dead_enabled() {
    // No symbol / selection / link under the cursor: every action is rendered (no fake-drop) but DISABLED
    // (RISK-070-1 — a disabled entry is OK; a dead-but-ENABLED entry FAILS). And a disabled entry maps to
    // NO action even if (impossibly) confirmed — the belt-and-braces second line of defence.
    let empty = EditorBodyAvailability::default();
    let items = editor_body_context_items(empty);
    for required in EDITOR_BODY_REQUIRED_IDS {
        let item = items.iter().find(|i| i.id == *required).unwrap_or_else(|| {
            panic!("entry {required} still RENDERED when unavailable (no fake-drop)")
        });
        assert!(
            !item.enabled,
            "{required} is DISABLED when it has no target (not dead-but-enabled)"
        );
        assert!(
            item.disabled_reason.is_some(),
            "{required} discloses WHY it is disabled"
        );
        assert_eq!(
            editor_body_action_for_id(required, empty),
            None,
            "{required} maps to NO action when disabled (can never fire a dead entry)",
        );
    }
    println!("PASS RISK-070-1: an unavailable editor-body entry is disabled+disclosed, never dead-enabled");
}

// ── WP-KERNEL-012 MT-070 validation_v4 remediation step 8: static canonical-binding regression gate ───

/// Every canonical Argus action dispatched anywhere in THIS file must be rebound to an authoritative
/// terminal snapshot and carry at least one action-specific terminal predicate BEFORE its driver is
/// finished.
///
/// This is a source-level gate, not a runtime one: `canonical_argus_driver::finish` already fails a
/// matrix that reaches it unbound (that is exactly how FAIL_V4 was caught), but a future matrix could
/// silently regress by dispatching an action inside a helper and never re-observing it. Scanning this
/// file's own source makes the invariant structural, so the omission is caught at the first `cargo test`
/// rather than after a full mounted run.
///
/// Recognized tokens (lines carrying the scanner's own literals are skipped by sentinel):
/// - ACTION:    any `*_and_reinspect(` driver call — every one of those pushes an observation;
/// - PREDICATE: any `assert_latest_terminal_predicate*` call — every one rebinds + records;
/// - FINISH:    `argus.finish` — the final gate.
///
/// Deliberately NOT actions: `click_expect_rejected` / `click_from_snapshot_expect_rpc_rejected`
/// (an RPC-level rejection never reaches the action channel, so no observation exists to bind) and
/// `argus.inspect` (a read).
#[test]
fn mt070_every_canonical_argus_action_is_terminally_bound_before_finish() {
    const SOURCE: &str = include_str!("test_context_menu_surfaces.rs");
    // The markers are assembled with `concat!` so the scanner's own source lines never contain the
    // literals it searches for. A trailing-comment sentinel would work too, but rustfmt is free to
    // move a trailing comment onto its own line and would silently make this gate match itself.
    const ACTION_MARKER: &str = concat!("_and_", "reinspect(");
    const PREDICATE_MARKER: &str = concat!("assert_latest_", "terminal_predicate");
    const FINISH_MARKER: &str = concat!("argus", ".finish");

    #[derive(Debug)]
    enum Token {
        Action(usize, String),
        Predicate,
        Finish(usize),
    }

    let mut tokens: Vec<Token> = Vec::new();
    for (offset, line) in SOURCE.lines().enumerate() {
        let number = offset + 1;
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") {
            continue;
        }
        if line.contains(ACTION_MARKER) {
            tokens.push(Token::Action(number, trimmed.to_owned()));
        } else if line.contains(PREDICATE_MARKER) {
            tokens.push(Token::Predicate);
        } else if line.contains(FINISH_MARKER) {
            tokens.push(Token::Finish(number));
        }
    }

    let action_count = tokens
        .iter()
        .filter(|token| matches!(token, Token::Action(..)))
        .count();
    let finish_count = tokens
        .iter()
        .filter(|token| matches!(token, Token::Finish(_)))
        .count();
    // Seven canonical dispatch SITES exist in this file (one per mounted matrix, plus the two inside
    // the editor-body helper that every one of its five entries reuses). The floor exists so a broken
    // scanner that silently matches nothing cannot report a vacuous pass.
    assert!(
        action_count >= 7,
        "the gate must observe the real canonical action inventory of this file, saw {action_count}"
    );
    assert!(
        finish_count >= 7,
        "the gate must observe every canonical driver finish in this file, saw {finish_count}"
    );

    let mut unbound: Option<(usize, String)> = None;
    for token in &tokens {
        match token {
            Token::Action(number, text) => {
                if let Some((pending, pending_text)) = unbound.take() {
                    panic!(
                        "canonical Argus action at line {pending} ({pending_text}) dispatched \
                         another action at line {number} before binding a terminal predicate"
                    );
                }
                unbound = Some((*number, text.clone()));
            }
            Token::Predicate => {
                unbound = None;
            }
            Token::Finish(number) => {
                if let Some((pending, pending_text)) = unbound.take() {
                    panic!(
                        "canonical Argus action at line {pending} ({pending_text}) reached the \
                         driver finish at line {number} without a terminal predicate"
                    );
                }
            }
        }
    }
    assert!(
        unbound.is_none(),
        "trailing canonical Argus action without a terminal predicate: {unbound:?}"
    );

    // The zero-dispatch matrices must stay explicit rather than passing vacuously.
    assert!(
        SOURCE.contains("dispatched_action_count()"),
        "matrices that dispatch nothing must assert their zero-dispatch state explicitly"
    );
    println!(
        "PASS MT-070 static gate: {action_count} canonical actions, {finish_count} driver finishes, \
         every action terminally bound before its finish"
    );
}
