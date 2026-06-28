//! WP-KERNEL-011 MT-013 — LIVE pane-local tabs / header binding proof.
//!
//! These tests render the REAL `HandshakeApp` headlessly via egui_kittest (which enables AccessKit
//! and pushes the same `TreeUpdate` the out-of-process Windows UIA adapter receives) and prove the
//! MT-013 binding header is LIVE, not just arithmetic:
//!
//! - each pane renders a header whose TITLE is bound to the pane's ACTIVE tab label (and re-titles
//!   when the active tab changes via a live tab click);
//! - each tab chip carries a module/type BADGE in its AccessKit description (module_label_for_tab);
//! - the per-pane LOCK button is a live `Role::Button` with the stable `pane-{pane_id}-lock`
//!   author_id, and a live click toggles the pane record's LockState (Lock <-> Unlock);
//! - all four panes (pane-a..pane-d) render independent headers + lock buttons with the correct
//!   pane-id prefix in their ids;
//! - the pane-a User-Manual tab carries the `hs-usermanual-diagnostics-tab` override id.
//!
//! Why this proves LIVE behavior: every assertion reads from the consumer-side AccessKit tree egui
//! produced for the frame, or mutates `HandshakeApp` state through a real click event.

use egui_kittest::kittest::NodeT;
use egui_kittest::Harness;
use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend_client::HealthInfo;
use handshake_native::module_switcher::ModuleId;
use handshake_native::pane_header::module_label_for_tab;
use handshake_native::pane_registry::{LockState, PaneId, PaneType};
use handshake_native::tab_bar::{TabBarState, TabState, USERMANUAL_DIAGNOSTICS_TAB_STABLE_ID};
use std::sync::Arc;

fn ok_app() -> HandshakeApp {
    HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }))
}

/// One live AccessKit node summary: (author_id, role, label, selected, description).
type LiveNode = (String, String, Option<String>, bool, Option<String>);

/// Every (author_id, role, label, selected, description) tuple in the live AccessKit tree.
fn live_nodes(harness: &Harness<'_, HandshakeApp>) -> Vec<LiveNode> {
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
fn live_all_four_panes_emit_independent_lock_buttons_with_pane_prefix() {
    // AC: all four panes render their own independent strips/headers; each lock button carries the
    // correct `pane-{pane_id}-lock` id (the pane prefix), proving headers are not shared/duplicated.
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), ok_app());
    harness.run();

    let nodes = live_nodes(&harness);
    for pane in ["pane-a", "pane-b", "pane-c", "pane-d"] {
        let lock_id = format!("pane-{pane}-lock");
        let lock = nodes
            .iter()
            .find(|(a, _, _, _, _)| *a == lock_id)
            .unwrap_or_else(|| {
                panic!(
                    "lock button '{lock_id}' missing from LIVE tree; found {:?}",
                    nodes.iter().map(|(a, _, _, _, _)| a).collect::<Vec<_>>()
                )
            });
        assert_eq!(lock.1, "Button", "{lock_id} role is Button");
        assert_eq!(
            lock.2.as_deref(),
            Some("Lock"),
            "{lock_id} default (unlocked) label"
        );
        assert!(
            lock.0.contains(pane),
            "{lock_id} carries the pane id prefix '{pane}'"
        );
    }
    println!("PASS: four panes each emit an independent lock button with the pane-id prefix");
}

/// The live (role, label) of the header-title node for `pane_id`, addressed by its stable
/// `pane-{pane_id}-title` author_id.
fn title_node(
    harness: &Harness<'_, HandshakeApp>,
    pane_id: &str,
) -> Option<(String, Option<String>)> {
    let want = format!("pane-{pane_id}-title");
    harness.root().children_recursive().find_map(|n| {
        let ak = n.accesskit_node();
        if ak.author_id() == Some(want.as_str()) {
            Some((format!("{:?}", ak.role()), ak.label()))
        } else {
            None
        }
    })
}

#[test]
fn live_header_title_binds_to_active_tab_label() {
    // AC: the pane header title matches TAB_LABEL_BY_ID[active_tab]. The title is an addressable
    // Role::Label (author_id `pane-{pane_id}-title`) carrying the active tab's short label. Seed:
    // pane-a active tab = Workspace, pane-c = MediaDownloader -> "Media Downloader". Read the title
    // node by its stable id (unambiguous, the way a model would).
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), ok_app());
    harness.set_size(egui::Vec2::new(1200.0, 800.0));
    harness.run();

    let (role_a, label_a) = title_node(&harness, "pane-a").expect("pane-a title node live");
    assert_eq!(role_a, "Label", "pane-a title is a Role::Label");
    assert_eq!(
        label_a.as_deref(),
        Some("Workspace"),
        "pane-a title bound to active tab Workspace"
    );

    let (_role_c, label_c) = title_node(&harness, "pane-c").expect("pane-c title node live");
    assert_eq!(
        label_c.as_deref(),
        Some("Media Downloader"),
        "pane-c title bound to active tab Media Downloader"
    );
    println!("PASS: header title nodes bound to active-tab labels (Workspace, Media Downloader)");
}

#[test]
fn live_header_title_retitles_on_tab_click() {
    // AC: the header title updates to match the newly-active tab when the active tab changes. Seed
    // pane-a with two tabs (Workspace active, Problems second); click the Problems tab; assert the
    // app's active tab is Problems and the live title label "Problems" is present.
    let mut app = ok_app();
    let pane_a: PaneId = Arc::from("pane-a");
    let bar = TabBarState::new(
        pane_a.clone(),
        vec![
            TabState::new(PaneType::Workspace),
            TabState::new(PaneType::Problems),
        ],
    );
    app.tab_bar_states_mut().insert(pane_a.clone(), bar);

    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.set_size(egui::Vec2::new(1200.0, 800.0));
    harness.run();

    // Before: active tab is Workspace.
    {
        let app = harness.state();
        let bar = app.tab_bar_states().get(&pane_a).unwrap();
        assert_eq!(
            bar.active().map(|t| t.pane_type.clone()),
            Some(PaneType::Workspace)
        );
    }

    // Click the Problems tab (tab-pane-a-1) by its stable author_id.
    harness
        .root()
        .children_recursive()
        .find(|n| n.accesskit_node().author_id() == Some("tab-pane-a-1"))
        .expect("tab-pane-a-1 (Problems) present")
        .click();
    harness.run();

    // After: active tab is Problems, and the header title label "Problems" is live.
    let app = harness.state();
    let bar = app.tab_bar_states().get(&pane_a).unwrap();
    assert_eq!(
        bar.active().map(|t| t.pane_type.clone()),
        Some(PaneType::Problems),
        "clicking the Problems tab made it active (drives the title binding)"
    );
    let _ = app; // release the &-borrow of harness.state() before the next harness query
                 // The pane-a header title node re-titled to "Problems" (the new active tab), read by its stable id.
    let (_role, label) = title_node(&harness, "pane-a").expect("pane-a title node live");
    assert_eq!(
        label.as_deref(),
        Some("Problems"),
        "header title re-bound to the newly-active tab (Workspace -> Problems)"
    );
    println!("PASS: header title re-titled to the newly-active tab (Workspace -> Problems)");
}

#[test]
fn live_tab_chip_carries_module_badge_in_description() {
    // AC: each tab chip shows a module/type badge. The badge is mirrored into the tab node's AccessKit
    // description as `module: <LABEL>`. The seed shell starts on the MAIN module; pane-a's Workspace
    // tab is in MAIN, so its description must contain "module: MAIN".
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), ok_app());
    harness.run();

    let nodes = live_nodes(&harness);
    let workspace_tab = nodes
        .iter()
        .find(|(a, role, _, _, _)| a == "tab-pane-a-0" && role == "Tab")
        .expect("pane-a tab 0 (Workspace) present");
    let desc = workspace_tab.4.as_deref().unwrap_or_default();
    assert!(
        desc.contains("module: MAIN"),
        "Workspace tab description carries its MAIN module badge; got {desc:?}"
    );
    println!("PASS: tab chip carries the module badge in its description ({desc:?})");
}

#[test]
fn live_lock_button_click_toggles_pane_lock_state() {
    // AC: clicking the lock button toggles the pane record's LockState (Unlock <-> Lock). Driven via a
    // live AccessKit click on pane-a's lock button, the same path an out-of-process model uses.
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), ok_app());
    harness.set_size(egui::Vec2::new(1200.0, 800.0));
    harness.run();

    let pane_a: PaneId = Arc::from("pane-a");
    // Before: seed pane is Unlocked.
    {
        let app = harness.state();
        let reg = app.pane_registry();
        let guard = reg.lock().unwrap();
        assert_eq!(guard.get(&pane_a).unwrap().lock_state, LockState::Unlocked);
    }

    // Click "Lock" on pane-a via its stable author_id.
    harness
        .root()
        .children_recursive()
        .find(|n| n.accesskit_node().author_id() == Some("pane-pane-a-lock"))
        .expect("pane-a lock button present")
        .click();
    harness.run();

    // After one click: Locked, and the button now reads "Unlock".
    {
        let app = harness.state();
        let reg = app.pane_registry();
        let guard = reg.lock().unwrap();
        assert_eq!(
            guard.get(&pane_a).unwrap().lock_state,
            LockState::Locked,
            "one lock click toggled pane-a to Locked"
        );
    }
    let nodes = live_nodes(&harness);
    let lock = nodes
        .iter()
        .find(|(a, _, _, _, _)| a == "pane-pane-a-lock")
        .unwrap();
    assert_eq!(
        lock.2.as_deref(),
        Some("Unlock"),
        "locked pane's button reads 'Unlock'"
    );
    assert!(
        lock.3,
        "locked pane's button is marked selected in the live tree"
    );

    // Click again -> back to Unlocked.
    harness
        .root()
        .children_recursive()
        .find(|n| n.accesskit_node().author_id() == Some("pane-pane-a-lock"))
        .expect("pane-a lock button present")
        .click();
    harness.run();
    {
        let app = harness.state();
        let reg = app.pane_registry();
        let guard = reg.lock().unwrap();
        assert_eq!(
            guard.get(&pane_a).unwrap().lock_state,
            LockState::Unlocked,
            "second lock click toggled pane-a back to Unlocked"
        );
    }
    println!("PASS: lock button click toggles LockState Unlocked->Locked->Unlocked (live)");
}

#[test]
fn live_locking_one_pane_does_not_affect_other_panes() {
    // AC (independence): locking pane-a must leave pane-b/c/d Unlocked — proves lock_requests target
    // the clicked pane only, not all panes.
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), ok_app());
    harness.set_size(egui::Vec2::new(1200.0, 800.0));
    harness.run();

    harness
        .root()
        .children_recursive()
        .find(|n| n.accesskit_node().author_id() == Some("pane-pane-a-lock"))
        .expect("pane-a lock button present")
        .click();
    harness.run();

    let app = harness.state();
    let reg = app.pane_registry();
    let guard = reg.lock().unwrap();
    assert_eq!(
        guard
            .get(&(Arc::from("pane-a") as PaneId))
            .unwrap()
            .lock_state,
        LockState::Locked
    );
    for other in ["pane-b", "pane-c", "pane-d"] {
        assert_eq!(
            guard.get(&(Arc::from(other) as PaneId)).unwrap().lock_state,
            LockState::Unlocked,
            "{other} must stay Unlocked when only pane-a was locked"
        );
    }
    println!("PASS: locking pane-a left pane-b/c/d Unlocked (per-pane independence)");
}

#[test]
fn live_pane_a_user_manual_tab_carries_diagnostics_override_id() {
    // AC / red-team CONTROL: pane-a's User-Manual tab gets the `hs-usermanual-diagnostics-tab`
    // override id instead of `tab-pane-a-{index}`. Seed pane-a with [Workspace, UserManual]; assert
    // the override id is live with Role::Tab and the index-based id for that tab is ABSENT.
    let mut app = ok_app();
    let pane_a: PaneId = Arc::from("pane-a");
    let bar = TabBarState::new(
        pane_a.clone(),
        vec![
            TabState::new(PaneType::Workspace),
            TabState::new(PaneType::UserManual),
        ],
    );
    app.tab_bar_states_mut().insert(pane_a, bar);

    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.set_size(egui::Vec2::new(1200.0, 800.0));
    harness.run();

    let nodes = live_nodes(&harness);
    let author_ids: Vec<&str> = nodes.iter().map(|(a, _, _, _, _)| a.as_str()).collect();

    // The override id is present as a Role::Tab.
    let override_tab = nodes
        .iter()
        .find(|(a, _, _, _, _)| a == USERMANUAL_DIAGNOSTICS_TAB_STABLE_ID)
        .unwrap_or_else(|| {
            panic!(
                "override id '{USERMANUAL_DIAGNOSTICS_TAB_STABLE_ID}' missing; found {author_ids:?}"
            )
        });
    assert_eq!(
        override_tab.1, "Tab",
        "the User-Manual diagnostics tab is a Role::Tab"
    );
    assert_eq!(
        override_tab.2.as_deref(),
        Some("User Manual"),
        "override tab label"
    );

    // The index-based id for the User-Manual tab (index 1) must NOT also be present (no double-id).
    assert!(
        !author_ids.contains(&"tab-pane-a-1"),
        "pane-a User-Manual tab uses the override id, NOT tab-pane-a-1; found {author_ids:?}"
    );
    // The non-override Workspace tab keeps its index-based id.
    assert!(
        author_ids.contains(&"tab-pane-a-0"),
        "the Workspace tab keeps its index-based id tab-pane-a-0"
    );
    println!(
        "PASS: pane-a User-Manual tab carries the diagnostics override id (and not the index id)"
    );
}

#[test]
fn live_module_switch_rebadges_active_pane_tabs() {
    // BINDING AC: switching the module re-tabs the active pane AND the badges reflect the new module.
    // Click the LAB module button; the active pane's default tab becomes Inference Lab, badged LAB.
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), ok_app());
    harness.set_size(egui::Vec2::new(1400.0, 900.0));
    harness.run();

    // Make pane-a the active pane by clicking its lock then unlocking (focus side-effect) — simplest is
    // to click a tab in pane-a so it becomes active, then switch module (module targets the active pane).
    harness
        .root()
        .children_recursive()
        .find(|n| n.accesskit_node().author_id() == Some("tab-pane-a-0"))
        .expect("pane-a tab 0 present")
        .click();
    harness.run();

    // Click the LAB module button (author_id module-lab).
    harness
        .root()
        .children_recursive()
        .find(|n| n.accesskit_node().author_id() == Some("module-lab"))
        .expect("module-lab button present")
        .click();
    harness.run();

    assert_eq!(
        harness.state().active_module(),
        ModuleId::Lab,
        "module switched to LAB"
    );
    let pane_a: PaneId = Arc::from("pane-a");
    let app = harness.state();
    let bar = app.tab_bar_states().get(&pane_a).unwrap();
    assert_eq!(
        bar.active().map(|t| t.pane_type.clone()),
        Some(PaneType::InferenceLab),
        "LAB module activated its default tab (Inference Lab) on the active pane"
    );
    let _ = app; // release the &-borrow of harness.state() before the next harness query

    // The Inference Lab tab on pane-a now badges LAB in its description.
    let nodes = live_nodes(&harness);
    let lab_tab = nodes
        .iter()
        .find(|(a, role, label, _, _)| {
            role == "Tab"
                && a.starts_with("tab-pane-a-")
                && label.as_deref() == Some("Inference Lab")
        })
        .expect("pane-a Inference Lab tab present after module switch");
    let desc = lab_tab.4.as_deref().unwrap_or_default();
    assert!(
        desc.contains("module: LAB"),
        "after switching to LAB, the Inference Lab tab badges LAB; got {desc:?}"
    );
    println!("PASS: module switch re-tabbed the active pane and re-badged its tabs to LAB");
}

#[test]
fn module_label_for_tab_unit_matches_contract() {
    // Unit re-statement of the contract proof_target at the integration boundary: covers the active-
    // module preference + home-module fallback for representative tabs.
    assert_eq!(
        module_label_for_tab(&PaneType::InferenceLab, ModuleId::Lab),
        "LAB"
    );
    assert_eq!(
        module_label_for_tab(&PaneType::Workspace, ModuleId::Main),
        "MAIN"
    );
    assert_eq!(
        module_label_for_tab(&PaneType::AtelierEditor, ModuleId::Main),
        "Atelier"
    );
    assert_eq!(
        PaneType::AtelierEditor.default_label(),
        "CKC",
        "the default tab label inside the Atelier module is CKC"
    );
    println!("PASS: module_label_for_tab integration boundary checks");
}
