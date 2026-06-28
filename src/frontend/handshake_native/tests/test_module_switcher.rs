//! WP-KERNEL-011 MT-012 — top-right MODULE switcher, end-to-end through the REAL `HandshakeApp`.
//!
//! These tests drive the actual shell (not only the `module_switcher` module's own unit tests) to prove
//! the C3 MODULE navigation behavior the MT-012 contract asks for:
//!
//! - the six module buttons (MAIN, Atelier, INGEST, STAGE, LAB, STUDIO) render in the header as live
//!   selectable `Role::Button` nodes, in MODULE_DEFINITIONS order (proof_target #3 / acceptance #1);
//! - the active module button is `selected`, the others are not (proof_target #3 / acceptance #3);
//! - clicking a non-active module button switches `active_module` AND rewrites the active pane's tab set
//!   to the module's canonical list with the module's default tab active (proof_target #2 / acceptance
//!   #2);
//! - clicking the already-active module is a no-op — no state change, no layout save scheduled
//!   (acceptance #5);
//! - every module button carries a stable AccessKit `author_id` = its `data_id` (`module-main`, …),
//!   role `Button`, click action, in the LIVE tree an out-of-process model reads (proof_target #4 /
//!   acceptance #4);
//! - the active module survives a project-tab switch + restore and serializes into the layout blob
//!   (acceptance #6).
//!
//! ## No live backend needed
//!
//! Layout persistence uses an in-memory `LayoutTransport` injected via `set_layout_manager` (mirroring
//! the MT-009 / MT-011 tests), so the full switch -> save -> restore path is provable with no running
//! `handshake_core`. Module clicks are driven by real kittest pointer clicks on the labelled Button
//! nodes (the same path an out-of-process agent uses), not synthetic state pokes.

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;
use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend_client::HealthInfo;
use handshake_native::layout_persistence::{
    LayoutError, LayoutPersistenceManager, LayoutTransport,
};
use handshake_native::module_switcher::{ModuleId, ModuleSwitcher, ModuleSwitcherColors};
use handshake_native::pane_registry::PaneType;
use handshake_native::project_tabs::ProjectItem;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

fn ok_app() -> HandshakeApp {
    HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }))
}

#[derive(Clone, Default)]
struct MemoryTransport {
    store: Arc<Mutex<HashMap<String, Value>>>,
}

impl LayoutTransport for MemoryTransport {
    fn load(&self, workspace_id: &str) -> Result<Option<Value>, LayoutError> {
        Ok(self.store.lock().unwrap().get(workspace_id).cloned())
    }
    fn save(&self, workspace_id: &str, layout_state: Value) -> Result<(), LayoutError> {
        self.store
            .lock()
            .unwrap()
            .insert(workspace_id.to_owned(), layout_state);
        Ok(())
    }
}

fn shell_with_transport(transport: MemoryTransport) -> HandshakeApp {
    let mut app = ok_app();
    app.set_layout_manager(LayoutPersistenceManager::new(
        Box::new(transport),
        std::time::Duration::ZERO,
    ));
    app
}

fn test_colors() -> ModuleSwitcherColors {
    ModuleSwitcherColors {
        active_bg: egui::Color32::from_rgb(0x7A, 0x7A, 0xFF),
        inactive_bg: egui::Color32::from_gray(30),
        hover_bg: egui::Color32::from_gray(40),
        text: egui::Color32::from_gray(200),
        active_text: egui::Color32::BLACK,
    }
}

/// Collect every live AccessKit node carrying an author_id: (author_id, role, label, selected).
fn live_author_nodes(
    harness: &Harness<'_, HandshakeApp>,
) -> Vec<(String, String, Option<String>, bool)> {
    let mut found = Vec::new();
    let root = harness.root();
    for node in root.children_recursive() {
        let ak = node.accesskit_node();
        if let Some(author_id) = ak.author_id() {
            found.push((
                author_id.to_owned(),
                format!("{:?}", ak.role()),
                ak.label(),
                ak.is_selected().unwrap_or(false),
            ));
        }
    }
    found
}

// ── proof_target #3 (widget-level): 6 nodes, MAIN selected, LAB not ────────────────────────────────

/// The `ModuleSwitcher` widget paints exactly six selectable Button nodes; with `active = Main`, the
/// `MAIN` node is `selected` and `LAB` is not. Driven through a real kittest render of the widget.
#[test]
fn switcher_paints_six_buttons_active_selected() {
    let mut switcher = ModuleSwitcher::new(ModuleId::Main);
    let mut harness = Harness::builder().build_ui(move |ui| {
        switcher.show(ui, test_colors());
    });
    harness.run();

    // Six module buttons by label.
    for label in ["MAIN", "Atelier", "INGEST", "STAGE", "LAB", "STUDIO"] {
        let _ = harness.get_by_label(label);
    }

    let nodes: Vec<_> = harness
        .root()
        .children_recursive()
        .map(|n| {
            let ak = n.accesskit_node();
            (
                ak.author_id().map(|s| s.to_owned()),
                format!("{:?}", ak.role()),
                ak.is_selected().unwrap_or(false),
            )
        })
        .filter(|(id, _, _)| {
            id.as_deref()
                .map(|s| s.starts_with("module-"))
                .unwrap_or(false)
        })
        .collect();

    assert_eq!(nodes.len(), 6, "six module buttons painted: {nodes:?}");
    for (id, role, _) in &nodes {
        assert_eq!(role, "Button", "{id:?} role is Button");
    }
    let selected = |id: &str| {
        nodes
            .iter()
            .find(|(a, _, _)| a.as_deref() == Some(id))
            .map(|(_, _, s)| *s)
            .unwrap_or(false)
    };
    assert!(selected("module-main"), "MAIN selected when active=Main");
    assert!(!selected("module-lab"), "LAB not selected when active=Main");
}

// ── proof_target #2 / acceptance #2: set_module updates the active pane ─────────────────────────────

/// `set_module(Lab)` on a shell whose active pane shows Workspace activates the LAB default tab
/// (inference-lab) and rewrites the pane's tab list to the LAB canonical set (with existing tabs kept).
#[test]
fn set_module_updates_active_pane_tabs() {
    let mut app = ok_app();
    // The seeded pane-a is the MAIN module with Workspace active. Make pane-a the active pane.
    let pane_a: handshake_native::pane_registry::PaneId = app
        .tab_bar_states()
        .keys()
        .min()
        .cloned()
        .expect("a seeded pane exists");
    assert_eq!(app.active_module(), ModuleId::Main, "starts on MAIN");

    // Switch to LAB.
    let changed = app.set_module(ModuleId::Lab);
    assert!(changed, "switching to a different module reports a change");
    assert_eq!(
        app.active_module(),
        ModuleId::Lab,
        "active module is now LAB"
    );

    let bar = app
        .tab_bar_states()
        .get(&pane_a)
        .expect("active pane has a tab bar");
    // LAB default tab is inference-lab, and it is active.
    assert_eq!(
        bar.active().map(|t| &t.pane_type),
        Some(&PaneType::InferenceLab),
        "LAB default tab (inference-lab) is active"
    );
    // The pane now contains the LAB canonical tabs.
    for expected in [
        PaneType::InferenceLab,
        PaneType::ModelRuntime,
        PaneType::Swarm,
        PaneType::FontManager,
        PaneType::KernelDcc,
        PaneType::UserManual,
    ] {
        assert!(
            bar.tabs.iter().any(|t| t.pane_type == expected),
            "LAB tab set contains {expected:?}"
        );
    }
    // inference-lab appears exactly once (dedup), not duplicated by the default-tab prepend.
    assert_eq!(
        bar.tabs
            .iter()
            .filter(|t| t.pane_type == PaneType::InferenceLab)
            .count(),
        1,
        "the module default tab is not duplicated"
    );
}

// ── acceptance #5: clicking the already-active module is a no-op ────────────────────────────────────

#[test]
fn switching_to_active_module_is_a_noop() {
    let mut app = ok_app();
    let before: Value = serde_json::to_value(app.capture_layout_snapshot()).unwrap();
    let changed = app.set_module(ModuleId::Main); // already MAIN
    assert!(
        !changed,
        "switching to the already-active module reports no change"
    );
    let after: Value = serde_json::to_value(app.capture_layout_snapshot()).unwrap();
    assert_eq!(
        before, after,
        "no state change when re-selecting the active module"
    );
}

// ── proof_target #3 + #4 / acceptance #1,#3,#4: live tree through the real shell ────────────────────

/// The real shell header renders the six module buttons as live `Role::Button` nodes with stable
/// author_ids (`module-main`..`module-studio`), MAIN selected by default. Proves proof_target #3 (6
/// selectable nodes, MAIN selected) + #4 (stable ids / role) in the LIVE tree an agent reads.
#[test]
fn live_shell_has_six_module_buttons_with_stable_ids() {
    let mut harness =
        Harness::builder().build_state(|ctx, a: &mut HandshakeApp| a.ui(ctx), ok_app());
    harness.run();

    let nodes = live_author_nodes(&harness);
    let by_id = |id: &str| nodes.iter().find(|(a, _, _, _)| a == id).cloned();

    for data_id in [
        "module-main",
        "module-ckc",
        "module-ingest",
        "module-stage",
        "module-lab",
        "module-studio",
    ] {
        let btn =
            by_id(data_id).unwrap_or_else(|| panic!("{data_id} missing from live tree: {nodes:?}"));
        assert_eq!(btn.1, "Button", "{data_id} role is Button");
    }
    assert_eq!(
        by_id("module-ckc").unwrap().2.as_deref(),
        Some("Atelier"),
        "module-ckc keeps its stable id but now displays the Atelier shell label"
    );
    // Default active module is MAIN.
    assert!(by_id("module-main").unwrap().3, "MAIN selected by default");
    assert!(
        !by_id("module-lab").unwrap().3,
        "LAB not selected by default"
    );

    let module_count = nodes
        .iter()
        .filter(|(a, _, _, _)| a.starts_with("module-"))
        .count();
    assert_eq!(
        module_count, 6,
        "exactly six module buttons in the live tree"
    );
}

/// Clicking the `LAB` module button in the REAL shell switches `active_module` to LAB and activates the
/// LAB default tab on the active pane — the genuine out-of-process interaction path.
#[test]
fn clicking_lab_button_switches_module_in_live_shell() {
    let mut harness =
        Harness::builder().build_state(|ctx, a: &mut HandshakeApp| a.ui(ctx), ok_app());
    harness.run();
    assert_eq!(
        harness.state().active_module(),
        ModuleId::Main,
        "starts on MAIN"
    );

    harness.get_by_label("LAB").click();
    harness.run();

    assert_eq!(
        harness.state().active_module(),
        ModuleId::Lab,
        "click switched to LAB"
    );
    // The active pane's active tab is now the LAB default (inference-lab).
    let pane = harness
        .state()
        .tab_bar_states()
        .keys()
        .min()
        .cloned()
        .unwrap();
    let bar = harness.state().tab_bar_states().get(&pane).unwrap();
    assert_eq!(
        bar.active().map(|t| &t.pane_type),
        Some(&PaneType::InferenceLab),
        "LAB default tab active after the click"
    );

    // The LAB button is now selected, MAIN is not.
    let nodes = live_author_nodes(&harness);
    let by_id = |id: &str| nodes.iter().find(|(a, _, _, _)| a == id).cloned();
    assert!(by_id("module-lab").unwrap().3, "LAB selected after click");
    assert!(
        !by_id("module-main").unwrap().3,
        "MAIN deselected after click"
    );
}

// ── acceptance #6: module survives a project switch + serializes into the layout blob ───────────────

#[test]
fn active_module_survives_project_switch_and_serializes() {
    let transport = MemoryTransport::default();
    let mut app = shell_with_transport(transport.clone());
    app.project_tabs_mut().apply_fetched(vec![
        ProjectItem::new("a", "Alpha"),
        ProjectItem::new("b", "Beta"),
    ]);
    let mut harness = Harness::builder().build_state(|ctx, a: &mut HandshakeApp| a.ui(ctx), app);
    harness.run();
    harness.state_mut().switch_project("a");
    harness.run();

    // On project "a", switch to STUDIO.
    assert!(harness.state_mut().set_module(ModuleId::Studio));
    harness.run();
    assert_eq!(harness.state().active_module(), ModuleId::Studio);

    // Switch to project "b" (no stored layout -> default MAIN). "a"'s STUDIO must be saved.
    assert!(harness.state_mut().switch_project("b"), "switch a->b");
    harness.run();
    assert_eq!(
        harness.state().active_module(),
        ModuleId::Main,
        "fresh project b defaults to MAIN"
    );

    // The saved blob for "a" carries active_module = STUDIO (serialized as the uppercase string).
    let saved_a = transport
        .store
        .lock()
        .unwrap()
        .get("a")
        .cloned()
        .expect("project a layout saved on leaving");
    assert_eq!(
        saved_a["active_module"],
        serde_json::json!("STUDIO"),
        "active_module serialized into the layout blob as the React uppercase string"
    );

    // Switch back to "a": its STUDIO module is restored from the saved blob.
    assert!(harness.state_mut().switch_project("a"), "switch b->a");
    harness.run();
    assert_eq!(
        harness.state().active_module(),
        ModuleId::Studio,
        "returning to project a restores its saved STUDIO module"
    );
}
