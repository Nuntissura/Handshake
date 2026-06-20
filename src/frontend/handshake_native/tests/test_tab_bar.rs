//! WP-KERNEL-011 MT-007 — LIVE tab-bar interaction proof.
//!
//! These tests render the REAL `HandshakeApp` headlessly via egui_kittest (which enables AccessKit
//! and pushes the same `TreeUpdate` the out-of-process Windows UIA adapter receives) and prove the
//! MT-007 tab surface is LIVE, not just arithmetic:
//!
//! - a 3-tab pane emits one `Role::TabList` container + three `Role::Tab` children with the contract
//!   author_ids (`tabbar-pane-a`, `tab-pane-a-0..2`);
//! - the active tab is marked selected in the live tree;
//! - a dirty tab carries the dirty indicator in the live tree (no pixel scraping);
//! - a pinned tab has NO close button node (close is hidden for pinned tabs);
//! - clicking a tab activates it (live, via AccessKit click);
//! - clicking a tab's close button removes it from the live tree;
//! - dragging tab-pane-a-1 onto pane-b moves it across panes exactly once (source loses it, target
//!   gains it) — driven by real pointer drag/drop events through the egui DragAndDrop path.
//!
//! Why this proves LIVE behavior: every assertion reads from the consumer-side AccessKit tree egui
//! produced for the frame, or mutates `HandshakeApp` state through a real pointer/click event. A
//! widget that was only built in memory (never emitted via `Context::accesskit_node_builder`) would
//! be absent here.

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;
use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend_client::HealthInfo;
use handshake_native::pane_registry::{PaneId, PaneType};
use handshake_native::tab_bar::{TabBarState, TabState};
use std::sync::Arc;

fn ok_app() -> HandshakeApp {
    HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }))
}

/// Build an app whose `pane-a` has three tabs: Workspace(clean), InferenceLab(dirty),
/// AtelierEditor(pinned, clean) — the exact set the MT-007 proof_target screenshot specifies.
fn app_with_three_tabs_on_pane_a() -> HandshakeApp {
    let mut app = ok_app();
    let pane_a: PaneId = Arc::from("pane-a");
    let mut workspace = TabState::new(PaneType::Workspace);
    workspace.dirty = false;
    let mut inference = TabState::new(PaneType::InferenceLab);
    inference.dirty = true; // dirty-dot
    let mut atelier = TabState::new(PaneType::AtelierEditor);
    atelier.pinned = true; // no close button; will be stabilized to the front

    // NOTE: stabilize_pins moves the pinned Atelier tab to the FRONT, so the live order becomes
    // [Atelier(pinned), Workspace, InferenceLab(dirty)]. The test reads author_ids by their live
    // index, so it follows the stabilized order rather than the insertion order.
    let bar = TabBarState::new(
        pane_a.clone(),
        vec![workspace, inference, atelier],
    );
    app.tab_bar_states_mut().insert(pane_a, bar);
    app
}

/// Every (author_id, role, label, selected, description) tuple in the live AccessKit tree.
fn live_nodes(
    harness: &Harness<'_, HandshakeApp>,
) -> Vec<(String, String, Option<String>, bool, Option<String>)> {
    let mut found = Vec::new();
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        if let Some(author_id) = ak.author_id() {
            found.push((
                author_id.to_owned(),
                format!("{:?}", ak.role()),
                ak.label(),
                ak.is_selected().unwrap_or(false),
                ak.description(),
            ));
        }
    }
    found
}

#[test]
fn live_tab_bar_emits_tablist_and_tab_nodes_with_contract_author_ids() {
    let mut harness = Harness::builder()
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app_with_three_tabs_on_pane_a());
    harness.run();

    let nodes = live_nodes(&harness);
    let author_ids: Vec<&str> = nodes.iter().map(|(a, _, _, _, _)| a.as_str()).collect();
    println!("LIVE tab nodes: {author_ids:?}");

    // One TabList container for pane-a.
    let tablist = nodes
        .iter()
        .find(|(a, _, _, _, _)| a == "tabbar-pane-a")
        .expect("tabbar-pane-a TabList node present in LIVE tree");
    assert_eq!(tablist.1, "TabList", "tab bar container role is TabList");

    // Three Tab children with the contract author_ids.
    for i in 0..3 {
        let id = format!("tab-pane-a-{i}");
        let tab = nodes
            .iter()
            .find(|(a, _, _, _, _)| *a == id)
            .unwrap_or_else(|| panic!("{id} missing from LIVE tree; found {author_ids:?}"));
        assert_eq!(tab.1, "Tab", "{id} role is Tab");
    }
    println!("PASS: TabList + 3 Tab nodes live with contract author_ids");
}

#[test]
fn live_active_tab_is_selected_and_dirty_tab_carries_indicator() {
    let mut harness = Harness::builder()
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app_with_three_tabs_on_pane_a());
    harness.run();

    let nodes = live_nodes(&harness);

    // Stabilized order: [Atelier(pinned) @0, Workspace @1, InferenceLab(dirty) @2]. The active tab is
    // the one TabBarState::new leaves active after stabilization (the previously-active index 0 ->
    // follows to wherever it landed). Exactly one Tab must be selected.
    let selected: Vec<&str> = nodes
        .iter()
        .filter(|(a, role, _, sel, _)| role == "Tab" && *sel && a.starts_with("tab-pane-a-"))
        .map(|(a, _, _, _, _)| a.as_str())
        .collect();
    assert_eq!(selected.len(), 1, "exactly one tab is selected (active), got {selected:?}");

    // The dirty InferenceLab tab carries the "dirty" indicator in its AccessKit description, so a
    // model reads unsaved-state without pixels. Find the dirty tab by its label. NOTE (MT-013): the
    // description now ALSO carries the module/type badge (e.g. "module: STAGE; dirty"), so the dirty
    // indicator is asserted by `contains("dirty")` rather than an exact match — the dirty signal is
    // still present and machine-readable, alongside the new module badge.
    let dirty_tab = nodes
        .iter()
        .find(|(a, role, label, _, _)| {
            role == "Tab"
                && a.starts_with("tab-pane-a-")
                && label.as_deref() == Some("Inference Lab")
        })
        .expect("Inference Lab tab present");
    assert!(
        dirty_tab.4.as_deref().unwrap_or_default().contains("dirty"),
        "dirty tab carries the 'dirty' indicator in the live AccessKit tree (description: {:?})",
        dirty_tab.4
    );

    // A clean tab (Workspace) carries NO dirty indicator (its description may carry the module badge
    // but never the dirty token).
    let clean_tab = nodes
        .iter()
        .find(|(a, role, label, _, _)| {
            role == "Tab" && a.starts_with("tab-pane-a-") && label.as_deref() == Some("Workspace")
        })
        .expect("Workspace tab present");
    assert!(
        !clean_tab.4.as_deref().unwrap_or_default().contains("dirty"),
        "clean tab has no dirty indicator (description: {:?})",
        clean_tab.4
    );
    println!("PASS: active tab selected; dirty tab flagged, clean tab not");
}

#[test]
fn live_pinned_tab_has_no_close_button_node() {
    let mut harness = Harness::builder()
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app_with_three_tabs_on_pane_a());
    harness.run();

    let nodes = live_nodes(&harness);
    let author_ids: Vec<&str> = nodes.iter().map(|(a, _, _, _, _)| a.as_str()).collect();

    // Atelier is pinned and (after stabilization) sits at index 0 -> tab-pane-a-0. Its close button
    // (tab-close-pane-a-0) must be ABSENT. The two unpinned tabs (index 1, 2) must HAVE close buttons.
    assert!(
        !author_ids.contains(&"tab-close-pane-a-0"),
        "pinned tab (index 0) must have NO close button; found {author_ids:?}"
    );
    assert!(
        author_ids.contains(&"tab-close-pane-a-1"),
        "unpinned tab (index 1) must have a close button"
    );
    assert!(
        author_ids.contains(&"tab-close-pane-a-2"),
        "unpinned tab (index 2) must have a close button"
    );
    println!("PASS: pinned tab has no close button; unpinned tabs do");
}

#[test]
fn live_click_tab_activates_it() {
    let mut harness = Harness::builder()
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app_with_three_tabs_on_pane_a());
    // Wide+tall window so the pane has room for the MT-013 header strip ABOVE the tab strip and the
    // (now badge-widened) tab chips lay out with un-clipped, clickable bounding boxes.
    harness.set_size(egui::Vec2::new(1200.0, 800.0));
    harness.run();

    // Click the last tab (tab-pane-a-2 = InferenceLab after stabilization) by its stable author_id,
    // the same path an out-of-process model uses (label "Inference Lab" collides with pane-b's seed).
    harness
        .root()
        .children_recursive()
        .find(|n| n.accesskit_node().author_id() == Some("tab-pane-a-2"))
        .expect("tab-pane-a-2 present")
        .click();
    harness.run();

    // The clicked tab is now selected (active) in the live tree.
    let nodes = live_nodes(&harness);
    let active_label = nodes
        .iter()
        .find(|(a, role, _, sel, _)| role == "Tab" && *sel && a.starts_with("tab-pane-a-"))
        .and_then(|(_, _, label, _, _)| label.clone());
    assert_eq!(
        active_label.as_deref(),
        Some("Inference Lab"),
        "clicking the Inference Lab tab made it active"
    );

    // And the app's TabBarState reflects the activation.
    let app = harness.state();
    let bar = app
        .tab_bar_states()
        .get(&(Arc::from("pane-a") as PaneId))
        .expect("pane-a tab bar");
    assert_eq!(
        bar.active().map(|t| t.pane_type.clone()),
        Some(PaneType::InferenceLab),
        "TabBarState active tab is InferenceLab after the click"
    );
    println!("PASS: clicking a tab activates it (live + state)");
}

#[test]
fn live_click_close_button_removes_tab() {
    let mut harness = Harness::builder()
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app_with_three_tabs_on_pane_a());
    // Wide+tall window so the MT-013 header strip + badge-widened tab chips lay out with un-clipped,
    // clickable close-button bounding boxes (the default size is now too tight after the header add).
    harness.set_size(egui::Vec2::new(1200.0, 800.0));
    harness.run();

    let before = harness
        .state()
        .tab_bar_states()
        .get(&(Arc::from("pane-a") as PaneId))
        .unwrap()
        .tabs
        .len();
    assert_eq!(before, 3, "three tabs before close");

    // Close the Workspace tab (an unpinned tab) via its close button. The close button label is
    // "Close Workspace" (set in render_one_tab's widget_info).
    harness.get_by_label("Close Workspace").click();
    harness.run();

    let after = harness
        .state()
        .tab_bar_states()
        .get(&(Arc::from("pane-a") as PaneId))
        .unwrap();
    assert_eq!(after.tabs.len(), 2, "one tab removed by the close button");
    assert!(
        !after.tabs.iter().any(|t| t.pane_type == PaneType::Workspace),
        "Workspace tab is gone"
    );
    println!("PASS: clicking a close button removes the tab (live + state)");
}

#[test]
fn live_drag_tab_across_panes_moves_it_exactly_once() {
    // Red-team CONTROL via the LIVE pointer path: drag tab-pane-a-1 and drop it onto pane-b's tab
    // bar; assert pane-a loses that tab and pane-b gains it exactly once (no duplication, no loss).
    let mut harness = Harness::builder()
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app_with_three_tabs_on_pane_a());
    harness.set_size(egui::Vec2::new(1000.0, 700.0));
    harness.run();
    // Collapse the MT-014 left rail for this tab-drag test: an OPEN rail is a SidePanel::left that
    // narrows + shifts the 2x2 pane grid, which moves the tab + tab-bar rects under test and made the
    // live cross-pane drag gesture unreliable in the harness. The rail is irrelevant to the tab-drag
    // behavior being proven, so collapsing it restores stable pane geometry without changing what this
    // test exercises (the cross-pane tab move). The rail's own behavior is covered by test_left_rail.
    harness.state_mut().set_left_rail_open(false);
    harness.run();

    // Source counts before.
    let pane_a: PaneId = Arc::from("pane-a");
    let pane_b: PaneId = Arc::from("pane-b");
    let a_before = harness.state().tab_bar_states().get(&pane_a).unwrap().tabs.len();
    let b_before = harness.state().tab_bar_states().get(&pane_b).unwrap().tabs.len();
    assert_eq!(a_before, 3, "pane-a starts with 3 tabs");

    // Find a draggable tab in pane-a (Workspace is unpinned, index 1 after stabilization) and the
    // pane-b tab bar drop target. Address by stable author_id (label "Workspace" collides with
    // pane-a's pane Group label) so the pointer events land on the real tab widget.
    let source_center = harness
        .root()
        .children_recursive()
        .find(|n| n.accesskit_node().author_id() == Some("tab-pane-a-1"))
        .expect("tab-pane-a-1 (Workspace) present")
        .rect()
        .center();

    // Target the pane-b TAB-BAR container by its unambiguous author_id (its tab label "Inference Lab"
    // would collide with pane-a's Inference Lab tab).
    let target_center = harness
        .root()
        .children_recursive()
        .find(|n| n.accesskit_node().author_id() == Some("tabbar-pane-b"))
        .expect("pane-b tab bar present")
        .rect()
        .center();

    // Real pointer drag: press at the source tab, then move the pointer in STEPS toward the target
    // (egui only registers a drag once the pointer moves past its drag threshold; a press->release
    // with no intermediate PointerMoved is treated as a click, not a drag). Finally release over the
    // pane-b tab bar. egui's DragAndDrop carries the TabDragPayload; pane-b's drop zone consumes it.
    harness.drag_at(source_center);
    harness.run();
    let steps = 8;
    for s in 1..=steps {
        let t = s as f32 / steps as f32;
        let p = source_center + (target_center - source_center) * t;
        harness.hover_at(p);
        harness.run();
    }
    harness.drop_at(target_center);
    harness.run();
    harness.run(); // settle one more frame for the reconcile

    let a_after = harness.state().tab_bar_states().get(&pane_a).unwrap();
    let b_after = harness.state().tab_bar_states().get(&pane_b).unwrap();

    assert_eq!(
        a_after.tabs.len(),
        a_before - 1,
        "pane-a lost exactly one tab (had {a_before}, now {})",
        a_after.tabs.len()
    );
    assert_eq!(
        b_after.tabs.len(),
        b_before + 1,
        "pane-b gained exactly one tab (had {b_before}, now {})",
        b_after.tabs.len()
    );
    assert!(
        !a_after.tabs.iter().any(|t| t.pane_type == PaneType::Workspace),
        "moved Workspace tab no longer in pane-a"
    );
    assert_eq!(
        b_after.tabs.iter().filter(|t| t.pane_type == PaneType::Workspace).count(),
        1,
        "moved Workspace tab appears exactly once in pane-b (no duplication)"
    );
    println!(
        "PASS: live drag moved a tab pane-a -> pane-b exactly once (a:{a_before}->{}, b:{b_before}->{})",
        a_after.tabs.len(),
        b_after.tabs.len()
    );
}

#[test]
fn live_close_five_tabs_decreases_tab_node_count_by_five() {
    // Red-team CONTROL (CI): render with a 6-tab pane, close 5 tabs, assert the live Tab node count
    // drops by 5. Driven through the live close-button path each step.
    let mut app = ok_app();
    let pane_a: PaneId = Arc::from("pane-a");
    let types = [
        PaneType::Workspace,
        PaneType::InferenceLab,
        PaneType::AtelierEditor,
        PaneType::Swarm,
        PaneType::Problems,
        PaneType::Jobs,
    ];
    let bar = TabBarState::new(
        pane_a.clone(),
        types.iter().cloned().map(TabState::new).collect(),
    );
    app.tab_bar_states_mut().insert(pane_a.clone(), bar);

    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    // Wide window so all six tabs in pane-a's tab strip lay out with on-screen bounding boxes (a
    // pointer click needs the close button's AccessKit bounding box).
    harness.set_size(egui::Vec2::new(1600.0, 900.0));
    harness.run();

    let count_tabs = |h: &Harness<'_, HandshakeApp>| -> usize {
        h.root()
            .children_recursive()
            .filter(|n| {
                let ak = n.accesskit_node();
                format!("{:?}", ak.role()) == "Tab"
                    && ak
                        .author_id()
                        .is_some_and(|a| a.starts_with("tab-pane-a-"))
            })
            .count()
    };

    let initial = count_tabs(&harness);
    assert_eq!(initial, 6, "six Tab nodes initially");

    // Close 5 tabs: always close the FIRST remaining tab (index 0; none are pinned here). Target the
    // close button by its stable author_id (`tab-close-pane-a-0`) rather than by label, because tab
    // labels can collide with the OTHER seeded panes' tabs (e.g. pane-b also seeds an Inference Lab
    // tab) — the author_id is unambiguous, which is exactly why a model addresses by it.
    for _ in 0..5 {
        harness
            .root()
            .children_recursive()
            .find(|n| n.accesskit_node().author_id() == Some("tab-close-pane-a-0"))
            .expect("first tab's close button present")
            .click();
        harness.run();
    }

    let remaining = count_tabs(&harness);
    assert_eq!(remaining, 1, "five tabs closed -> one Tab node remains (was {initial})");
    println!("PASS: closing 5 tabs decreased live Tab node count from {initial} to {remaining}");
}
