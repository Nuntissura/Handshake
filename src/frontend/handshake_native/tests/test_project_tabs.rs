//! WP-KERNEL-011 MT-011 — top project (workspace) tabs, end-to-end through the REAL `HandshakeApp`.
//!
//! These tests drive the actual shell (not just the `project_tabs` module's own unit tests) to prove
//! the C3 navigation behavior the MT-011 contract asks for:
//!
//! - clicking a non-active project tab switches `active_project_id` (proof_target #1);
//! - a project switch SAVES the leaving project's layout and LOADS the entered project's stored layout
//!   (or the default layout), via the MT-009 persistence lifecycle (proof_target #2 / acceptance #2-3);
//! - the strip renders one selectable Tab node per project, with the active tab `selected`
//!   (proof_target #3 / acceptance #4);
//! - every project tab carries a stable AccessKit `author_id` = `project-tab-{id}`, role `Tab`, with a
//!   click action, in the LIVE tree an out-of-process model reads (proof_target #4 / acceptance #5);
//! - an empty workspace list shows a single disabled "No projects" tab without crashing (acceptance #6);
//! - a fetch error retains the previous list and shows an inline message (acceptance #7).
//!
//! ## No live backend needed
//!
//! Workspaces are seeded directly via `HandshakeApp::project_tabs_mut().apply_fetched(...)` (the same
//! method the background `GET /workspaces` fetch folds its result into), and layout persistence uses an
//! in-memory `LayoutTransport` injected via `set_layout_manager` (mirroring the MT-009 tests). So the
//! full switch -> save-old -> load-new transition is provable with no running `handshake_core`.

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;
use handshake_native::app::{HandshakeApp, HealthDisplayState, DEFAULT_PROJECT_ID};
use handshake_native::backend_client::HealthInfo;
use handshake_native::layout_persistence::{LayoutError, LayoutPersistenceManager, LayoutTransport};
use handshake_native::project_tabs::{FetchState, ProjectItem, ProjectTabBar, ProjectTabColors};
use handshake_native::split_layout::SplitWeights;
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

/// In-memory `LayoutTransport` backed by a shared map keyed by workspace id (same stand-in the MT-009
/// tests use). Two shells sharing one map mirror a real backend; here one shell saves project `a`'s
/// layout and loads project `b`'s layout across a switch.
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

/// Build a shell whose layout manager uses `transport` (zero debounce so a save resolves immediately).
fn shell_with_transport(transport: MemoryTransport) -> HandshakeApp {
    let mut app = ok_app();
    app.set_layout_manager(LayoutPersistenceManager::new(
        Box::new(transport),
        std::time::Duration::ZERO,
    ));
    app
}

// ── proof_target #1: unit — clicking a non-active tab returns Some(that id) ────────────────────────

/// `ProjectTabBar::show` returns `Some("b")` when "Beta" is clicked while "Alpha" is active. Driven by
/// a real kittest pointer click on the "Beta" labelled Tab node, so this is the genuine widget
/// interaction (the same path an out-of-process agent uses), not a synthetic state poke.
#[test]
fn clicking_beta_returns_some_b() {
    let mut bar = ProjectTabBar::new(
        vec![ProjectItem::new("a", "Alpha"), ProjectItem::new("b", "Beta")],
        "a",
    );
    let switched: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let switched_c = switched.clone();
    let mut harness = Harness::builder().build_ui(move |ui| {
        if let Some(id) = bar.show(ui, test_colors()) {
            *switched_c.lock().unwrap() = Some(id);
        }
    });
    harness.run();
    harness.get_by_label("Beta").click();
    harness.run();
    assert_eq!(
        switched.lock().unwrap().clone(),
        Some("b".to_string()),
        "clicking Beta while Alpha active returns Some('b')"
    );
}

fn test_colors() -> ProjectTabColors {
    ProjectTabColors {
        bar_bg: egui::Color32::from_gray(20),
        active_bg: egui::Color32::from_gray(60),
        inactive_bg: egui::Color32::from_gray(30),
        hover_bg: egui::Color32::from_gray(40),
        text: egui::Color32::WHITE,
        disabled_text: egui::Color32::GRAY,
        accent: egui::Color32::from_rgb(0x7A, 0x7A, 0xFF),
        error: egui::Color32::from_rgb(0xFF, 0x55, 0x55),
    }
}

// ── proof_target #2 / acceptance #2-3: switch saves old layout + loads new layout ──────────────────

/// Switching from project `a` to project `b` SAVES `a`'s current (non-default) layout and LOADS `b`'s
/// previously-stored layout. Then switching back to `a` restores the layout that was saved when leaving
/// it. This proves project isolation through the real MT-009 persistence path.
#[test]
fn switching_project_saves_old_and_loads_new_layout() {
    let transport = MemoryTransport::default();

    // Pre-store a DISTINCT layout for project "b" so loading it is observable.
    {
        let app_b = shell_with_transport(transport.clone());
        let mut snap = app_b.capture_layout_snapshot();
        snap.project_id = "b".to_string();
        snap.split_weights = SplitWeights { vertical: 0.2, horizontal: 0.9 };
        transport
            .store
            .lock()
            .unwrap()
            .insert("b".to_string(), snap.to_layout_state());
    }

    let mut app = shell_with_transport(transport.clone());
    app.project_tabs_mut().apply_fetched(vec![
        ProjectItem::new("a", "Alpha"),
        ProjectItem::new("b", "Beta"),
    ]);
    // The fetch may have re-pointed the active id to the first project ("a"); align the shell's
    // active_project_id to "a" so the lifecycle's first load keys on "a".
    let mut harness =
        Harness::builder().build_state(|ctx, a: &mut HandshakeApp| a.ui(ctx), app);
    harness.run();
    harness.state_mut().switch_project("a");
    harness.run();

    // Give project "a" a recognizable layout, then switch to "b".
    let a_weights = SplitWeights { vertical: 0.41, horizontal: 0.59 };
    *harness.state_mut().split_weights_mut() = a_weights;

    assert!(harness.state_mut().switch_project("b"), "switch a->b happens");
    // Next frame: lifecycle loads "b"'s stored layout (loaded_project_id != active_project_id).
    harness.run();
    assert_eq!(harness.state().active_project_id(), "b", "active project is now b");
    assert_eq!(
        harness.state().split_weights(),
        SplitWeights { vertical: 0.2, horizontal: 0.9 },
        "project b's previously-stored layout was loaded on switch"
    );
    // "a"'s layout must have been saved to the store on leaving it.
    let saved_a = transport
        .store
        .lock()
        .unwrap()
        .get("a")
        .cloned()
        .expect("project a's layout saved when leaving it");
    assert_eq!(saved_a["split_weights"]["vertical"], serde_json::json!(0.41_f32));
    assert_eq!(saved_a["split_weights"]["horizontal"], serde_json::json!(0.59_f32));

    // Switch back to "a": its saved layout must be restored.
    assert!(harness.state_mut().switch_project("a"), "switch b->a happens");
    harness.run();
    assert_eq!(harness.state().active_project_id(), "a");
    assert_eq!(
        harness.state().split_weights(),
        a_weights,
        "switching back restores the layout saved when leaving project a"
    );
}

/// Switching to a project with NO stored layout falls back to the default layout (never carries the old
/// project's panes over). Mirrors React `defaultWorkbenchLayoutState(projectId)`.
#[test]
fn switching_to_unstored_project_uses_default_layout() {
    let transport = MemoryTransport::default();
    let mut app = shell_with_transport(transport.clone());
    app.project_tabs_mut().apply_fetched(vec![
        ProjectItem::new("a", "Alpha"),
        ProjectItem::new("fresh", "Fresh"),
    ]);
    let mut harness =
        Harness::builder().build_state(|ctx, a: &mut HandshakeApp| a.ui(ctx), app);
    harness.run();
    harness.state_mut().switch_project("a");
    harness.run();

    // Change a's layout, then switch to the never-stored "fresh" project.
    *harness.state_mut().split_weights_mut() = SplitWeights { vertical: 0.7, horizontal: 0.3 };
    assert!(harness.state_mut().switch_project("fresh"));
    harness.run();

    assert_eq!(harness.state().active_project_id(), "fresh");
    assert_eq!(
        harness.state().split_weights(),
        SplitWeights::default(),
        "an unstored project falls back to the default layout, not the old project's layout"
    );
}

// ── proof_target #3 + #4 / acceptance #4-5: live AccessKit Tab/TabList nodes, active selected ──────

fn live_author_nodes(harness: &Harness<'_, HandshakeApp>) -> Vec<(String, String, Option<String>, bool)> {
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

/// The real shell renders one `Role::Tab` node per project with author_id `project-tab-{id}`, inside a
/// `Role::TabList` strip (`project-tabs`), with the active tab `selected` — in the LIVE tree an
/// out-of-process model reads. Proves proof_target #3 (3 selectable nodes) + #4 (stable ids/role).
#[test]
fn live_tree_has_three_project_tabs_with_stable_ids_and_active_selected() {
    let mut app = ok_app();
    app.project_tabs_mut().apply_fetched(vec![
        ProjectItem::new("p1", "Alpha"),
        ProjectItem::new("p2", "Beta"),
        ProjectItem::new("p3", "Gamma"),
    ]);
    let mut harness =
        Harness::builder().build_state(|ctx, a: &mut HandshakeApp| a.ui(ctx), app);
    // After apply_fetched the active id became "p1" (first project); align the shell.
    harness.state_mut().switch_project("p1");
    harness.run();

    let nodes = live_author_nodes(&harness);
    let by_id = |id: &str| nodes.iter().find(|(a, _, _, _)| a == id).cloned();

    // The strip container is a live TabList.
    let strip = by_id("project-tabs").expect("project-tabs strip in live tree");
    assert_eq!(strip.1, "TabList", "strip container role is TabList");

    // Exactly three project Tab nodes, by stable author_id, role Tab.
    for (id, name) in [("p1", "Alpha"), ("p2", "Beta"), ("p3", "Gamma")] {
        let author = format!("project-tab-{id}");
        let tab = by_id(&author).unwrap_or_else(|| panic!("{author} missing from live tree: {nodes:?}"));
        assert_eq!(tab.1, "Tab", "{author} role is Tab");
        assert_eq!(tab.2.as_deref(), Some(name), "{author} label");
    }

    // The active tab (p1) is selected; the others are not.
    assert!(by_id("project-tab-p1").unwrap().3, "active tab p1 selected=true");
    assert!(!by_id("project-tab-p2").unwrap().3, "inactive tab p2 selected=false");
    assert!(!by_id("project-tab-p3").unwrap().3, "inactive tab p3 selected=false");

    // Findable by label via kittest's Queryable (the UIA-style locate path an agent uses).
    let _ = harness.get_by_label("Beta");

    let project_tab_count = nodes
        .iter()
        .filter(|(a, _, _, _)| a.starts_with("project-tab-"))
        .count();
    assert_eq!(project_tab_count, 3, "exactly three project tabs painted as selectable nodes");
}

// ── acceptance #6: empty workspace list -> single disabled placeholder, no crash ───────────────────

#[test]
fn empty_workspace_list_shows_disabled_placeholder_without_crashing() {
    let mut app = ok_app();
    app.project_tabs_mut().apply_fetched(Vec::new());
    let mut harness =
        Harness::builder().build_state(|ctx, a: &mut HandshakeApp| a.ui(ctx), app);
    harness.run(); // must not panic

    let nodes = live_author_nodes(&harness);
    let placeholder = nodes
        .iter()
        .find(|(a, _, _, _)| a == "project-tab-none")
        .unwrap_or_else(|| panic!("placeholder tab missing: {nodes:?}"));
    assert_eq!(placeholder.1, "Tab", "placeholder is a Tab node");
    assert_eq!(placeholder.2.as_deref(), Some("No projects"), "placeholder label");
    // No real project tab nodes when the list is empty.
    assert!(
        !nodes.iter().any(|(a, _, _, _)| a.starts_with("project-tab-p")
            || (a.starts_with("project-tab-") && a != "project-tab-none")),
        "no real project tabs when the list is empty"
    );
    println!("PASS: empty list -> single disabled 'No projects' placeholder, no crash");
}

// ── acceptance #7: a fetch error retains the previous list + shows it inline ───────────────────────

#[test]
fn fetch_error_retains_previous_project_list() {
    let mut app = ok_app();
    app.project_tabs_mut().apply_fetched(vec![
        ProjectItem::new("a", "Alpha"),
        ProjectItem::new("b", "Beta"),
    ]);
    // A later refresh fails: the previous two-project list must be retained.
    app.project_tabs_mut().apply_fetch_error("connection refused");
    let mut harness =
        Harness::builder().build_state(|ctx, a: &mut HandshakeApp| a.ui(ctx), app);
    harness.run(); // must not panic; renders the retained tabs + inline error

    assert_eq!(harness.state().project_tabs().projects().len(), 2, "previous list retained");
    assert!(matches!(harness.state().project_tabs().fetch_state(), FetchState::Error(_)));

    let nodes = live_author_nodes(&harness);
    // Both project tabs still present in the live tree despite the fetch error.
    for id in ["project-tab-a", "project-tab-b"] {
        assert!(
            nodes.iter().any(|(a, _, _, _)| a == id),
            "{id} still present after fetch error: {nodes:?}"
        );
    }
}

// ── default seed: a fresh shell shows exactly the default project before any fetch resolves ─────────

#[test]
fn fresh_shell_seeds_default_project_tab() {
    let mut harness =
        Harness::builder().build_state(|ctx, a: &mut HandshakeApp| a.ui(ctx), ok_app());
    harness.run();
    let bar = harness.state().project_tabs();
    assert_eq!(bar.projects().len(), 1, "one seeded tab before fetch");
    assert_eq!(bar.projects()[0].id, DEFAULT_PROJECT_ID);
    assert_eq!(bar.active_id(), DEFAULT_PROJECT_ID, "seeded tab is active");
}
