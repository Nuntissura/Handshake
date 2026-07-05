//! WP-CKC-posekit-overhaul MT-041 — operator-facing Atelier menu-bar dropdown + the Argus menu-leaf
//! expansion, proven end-to-end through the REAL `HandshakeApp` shell.
//!
//! ## What this proves (the MT-041 contract)
//!
//! 1. **Atelier menu-bar entry** — the GO menu carries an operator-facing "Atelier" group whose four
//!    leaves (`menu.go.atelier`, `.ckc`, `.posekit`, `.ingest`) are present as stable `Role::MenuItem`
//!    author_id nodes while the GO menu is open, and clicking each drives the shell `set_module` +
//!    the MT-006 internal-tab `set_active_tab` deep-link (CKC / Posekit / Ingest jumps).
//! 2a. **Static leaf-id discovery registry** — [`menu_leaf_ids`] / [`menu_leaf_catalog`] enumerate every
//!    menu leaf id WITHOUT opening the menu; the live drift gate opens each menu in the real shell and
//!    asserts the rendered leaf set matches the registry exactly (no drift in either direction).
//! 2b/2c. **Open-then-steer, and the popup HOLDS across the action-drain frame** — driven through the
//!    REAL MCP [`ActionChannel`] (an AccessKit `Action::Click`, the exact Argus path): `argus.click
//!    menu-go` opens the GO menu, the popup stays open across the drain + repaint frames (egui popups can
//!    close on a synthesized pointer event — this proves the AccessKit-driven open does not), the leaf
//!    then enters the tree carrying a Click action, and `argus.click menu.go.atelier` steers the shell.
//!
//! No live backend is needed — the shell is built with `HandshakeApp::with_health(...)` and menu
//! interactions run through real kittest clicks + the real action channel (the out-of-process path).

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;

use handshake_native::accessibility::{collect_ui_tree_snapshot, UiTreeSnapshot};
use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::atelier_panel::AtelierPanelTab;
use handshake_native::backend_client::HealthInfo;
use handshake_native::mcp::{ActionChannel, UiAction};
use handshake_native::module_switcher::ModuleId;
use handshake_native::top_menu_bar::{
    menu_leaf_catalog, menu_leaf_ids, MenuId, MENU_DEFINITIONS, MENU_GO_ATELIER_AUTHOR_ID,
    MENU_GO_ATELIER_CKC_AUTHOR_ID, MENU_GO_ATELIER_INGEST_AUTHOR_ID,
    MENU_GO_ATELIER_POSEKIT_AUTHOR_ID,
};

/// The four operator-facing Atelier leaf author_ids the MT-041 contract names, in render order.
const ATELIER_LEAVES: [&str; 4] = [
    MENU_GO_ATELIER_AUTHOR_ID,
    MENU_GO_ATELIER_CKC_AUTHOR_ID,
    MENU_GO_ATELIER_POSEKIT_AUTHOR_ID,
    MENU_GO_ATELIER_INGEST_AUTHOR_ID,
];

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

/// Collect every live AccessKit node carrying an author_id, as `(author_id, role)`.
fn live_author_nodes(harness: &Harness<'_, HandshakeApp>) -> Vec<(String, String)> {
    let mut found = Vec::new();
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        if let Some(author_id) = ak.author_id() {
            found.push((author_id.to_owned(), format!("{:?}", ak.role())));
        }
    }
    found
}

// ── Deliverable 1 + 4: the GO > Atelier group leaves exist (only) while the menu is open ──────────────

#[test]
fn go_menu_exposes_atelier_group_leaves_when_open() {
    let mut harness = shell_harness();
    harness.run();

    // Closed: the Atelier leaves are absent (menu leaves are dynamic — present only while GO is open).
    let before = live_author_nodes(&harness);
    for leaf in ATELIER_LEAVES {
        assert!(
            !before.iter().any(|(a, _)| a == leaf),
            "{leaf} must be absent while the GO menu is closed: {before:?}"
        );
    }

    // Open GO (kittest mouse click — the out-of-process open path). Run twice so the popup materializes.
    harness.get_by_label("GO").click();
    harness.run();
    harness.run();

    let nodes = live_author_nodes(&harness);
    for leaf in ATELIER_LEAVES {
        let found = nodes
            .iter()
            .find(|(a, _)| a == leaf)
            .unwrap_or_else(|| panic!("open GO menu missing Atelier leaf {leaf}: {nodes:?}"));
        assert_eq!(found.1, "MenuItem", "{leaf} role is MenuItem");
    }

    // The static registry lists exactly these four Atelier leaves under GO.
    let go = menu_leaf_ids(MenuId::Go);
    for leaf in ATELIER_LEAVES {
        assert!(
            go.contains(&leaf),
            "static registry GO catalog lists {leaf}"
        );
    }
}

// ── Deliverable 2a: the static leaf-id registry matches the rendered menus exactly (drift gate) ───────

#[test]
fn leaf_discovery_registry_matches_rendered_menus() {
    let top_level: std::collections::HashSet<&str> =
        MENU_DEFINITIONS.iter().map(|m| m.author_id()).collect();

    // The whole static catalog (every leaf id the registry claims exists).
    let mut all_catalog: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for (_menu, leaves) in menu_leaf_catalog() {
        for &leaf in leaves {
            all_catalog.insert(leaf);
        }
    }

    // Every menu leaf actually rendered by an open menu (accumulated across all six menus). A live
    // MenuItem node that is not one of the six top-level buttons is a leaf.
    let mut all_live_leaves: std::collections::HashSet<String> = std::collections::HashSet::new();

    for menu in MENU_DEFINITIONS {
        let mut harness = shell_harness();
        harness.run();
        harness.get_by_label(menu.title()).click();
        harness.run();
        harness.run();

        let nodes = live_author_nodes(&harness);
        // Forward: every catalog leaf for THIS menu is present as a live MenuItem node (catches a leaf
        // dropped from the render, or a bogus registry id the menu never renders).
        for &leaf in menu_leaf_ids(menu) {
            let found = nodes.iter().find(|(a, _)| a == leaf).unwrap_or_else(|| {
                panic!(
                    "open {} menu is missing registry leaf {leaf}: {nodes:?}",
                    menu.title()
                )
            });
            assert_eq!(found.1, "MenuItem", "leaf {leaf} role is MenuItem");
        }

        for (author_id, role) in &nodes {
            if role == "MenuItem" && !top_level.contains(author_id.as_str()) {
                all_live_leaves.insert(author_id.clone());
            }
        }
    }

    // Reverse: no rendered leaf exists outside the registry (catches a leaf added to the render but never
    // registered — the exact drift the static catalog must prevent).
    for live in &all_live_leaves {
        assert!(
            all_catalog.contains(live.as_str()),
            "live menu leaf {live} is NOT in the static leaf-id registry (drift); registry={all_catalog:?}"
        );
    }
    // And the registry has no phantom leaf the menus never render.
    let live_ref: std::collections::HashSet<&str> =
        all_live_leaves.iter().map(|s| s.as_str()).collect();
    for cat in &all_catalog {
        assert!(
            live_ref.contains(cat),
            "registry leaf {cat} was never rendered by any open menu (phantom); live={live_ref:?}"
        );
    }
}

// ── Deliverable 1: clicking each Atelier leaf triggers the module/tab jump ────────────────────────────

#[test]
fn atelier_leaves_jump_to_module_and_tab() {
    // CKC leaf -> the CKC full-window Atelier module, landing on the Castkit Codex internal tab.
    {
        let mut h = shell_harness();
        h.run();
        assert_ne!(
            h.state().active_module(),
            ModuleId::Ckc,
            "default module is not the Atelier CKC module"
        );
        h.get_by_label("GO").click();
        h.run();
        h.get_by_label("Atelier: Castkit Codex").click();
        h.run();
        h.run();
        assert_eq!(
            h.state().active_module(),
            ModuleId::Ckc,
            "menu.go.atelier.ckc opened the CKC full-window Atelier module"
        );
        assert_eq!(
            h.state().atelier_active_tab(),
            AtelierPanelTab::CastkitCodex,
            "the CKC leaf landed on the Castkit Codex internal tab"
        );
    }

    // Posekit leaf -> the CKC module (shared full-window surface) but the Posekit INTERNAL tab (the tab is
    // what distinguishes the Posekit jump from the Castkit Codex jump — same module).
    {
        let mut h = shell_harness();
        h.run();
        h.get_by_label("GO").click();
        h.run();
        h.get_by_label("Atelier: Posekit").click();
        h.run();
        h.run();
        assert_eq!(
            h.state().active_module(),
            ModuleId::Ckc,
            "menu.go.atelier.posekit opened the Atelier (CKC full-window module)"
        );
        assert_eq!(
            h.state().atelier_active_tab(),
            AtelierPanelTab::Posekit,
            "the Posekit leaf selected the Posekit internal tab (not the CastkitCodex default)"
        );

        h.get_by_label("GO").click();
        h.run();
        h.get_by_label("Atelier: Castkit Codex").click();
        h.run();
        h.run();
        assert_eq!(
            h.state().active_module(),
            ModuleId::Ckc,
            "the CKC leaf keeps the shared CKC module active when returning from Posekit"
        );
        assert_eq!(
            h.state().atelier_active_tab(),
            AtelierPanelTab::CastkitCodex,
            "the CKC leaf resets the shared Atelier surface from Posekit to Castkit Codex"
        );
    }

    // Ingest leaf -> the Ingest full-window Atelier module, landing on the Ingest internal tab.
    {
        let mut h = shell_harness();
        h.run();
        h.get_by_label("GO").click();
        h.run();
        h.get_by_label("Atelier: Ingest").click();
        h.run();
        h.run();
        assert_eq!(
            h.state().active_module(),
            ModuleId::Ingest,
            "menu.go.atelier.ingest opened the Ingest full-window Atelier module"
        );
        assert_eq!(
            h.state().atelier_active_tab(),
            AtelierPanelTab::Ingest,
            "the Ingest leaf landed on the Ingest internal tab"
        );
    }
}

// ── Deliverable 2b/2c: Argus click OPENS the GO menu, the popup HOLDS across the drain frame, then the
//    leaf steers the shell — driven through the REAL MCP ActionChannel (an AccessKit Click) ────────────

/// One frame of the real shell on a persistent ctx, fed the given events, returning the live AccessKit
/// snapshot — the exact `list_widgets`/`argus.inspect` projection an out-of-process model reads.
fn run_frame(
    ctx: &egui::Context,
    app: &mut HandshakeApp,
    events: Vec<egui::Event>,
) -> UiTreeSnapshot {
    let raw = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 800.0),
        )),
        events,
        ..Default::default()
    };
    let output = ctx.run(raw, |ctx| app.ui(ctx));
    let update = output
        .platform_output
        .accesskit_update
        .expect("AccessKit update produced for the live shell frame");
    collect_ui_tree_snapshot(&update)
}

#[test]
fn argus_click_opens_and_holds_go_menu_then_steers_atelier_leaf() {
    // A persistent ctx + app so we control the frame-by-frame snapshots the real Argus drain path yields.
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut app = ok_app();

    // Warm-up frame, then capture the CLOSED-menu tree.
    let _ = run_frame(&ctx, &mut app, vec![]);
    let closed = run_frame(&ctx, &mut app, vec![]);
    assert!(
        closed.find_by_author_id("menu-go").is_some(),
        "the menu-go top-level button is present when closed"
    );
    assert!(
        closed
            .find_by_author_id(MENU_GO_ATELIER_AUTHOR_ID)
            .is_none(),
        "the Atelier leaf is ABSENT while the GO menu is closed (a model cannot one-shot a leaf)"
    );
    assert_ne!(
        app.active_module(),
        ModuleId::Ckc,
        "the shell does not start on the Atelier CKC module"
    );

    // Step 1 — argus.click menu-go through the REAL ActionChannel. `click_widget` resolves the author_id
    // to a NodeId and enqueues an AccessKit `Action::Click` (NOT a synthesized pointer event); the frame
    // loop drains it into an `AccessKitActionRequest` event.
    let mut channel = ActionChannel::new();
    channel
        .enqueue(&closed, "menu-go", UiAction::Click)
        .expect("argus.click on menu-go resolves + queues");
    let open_events = channel.drain_into_events();
    assert!(
        !open_events.is_empty(),
        "the click drained into an AccessKit action event the shell consumes"
    );

    // The action-drain frame (feed the click), then one repaint frame so egui materializes the popup's
    // leaves into the AccessKit tree.
    let _ = run_frame(&ctx, &mut app, open_events);
    let opened = run_frame(&ctx, &mut app, vec![]);
    assert!(
        opened
            .find_by_author_id(MENU_GO_ATELIER_AUTHOR_ID)
            .is_some(),
        "argus.click on menu-go OPENED the GO menu (the Atelier leaf entered the live tree)"
    );

    // HOLD proof: an ADDITIONAL idle frame must not close the popup. egui popups can close on the next
    // synthesized pointer event; this proves the AccessKit-driven open HOLDS across the drain/repaint so
    // the two-step open-then-steer contract is race-free.
    let held = run_frame(&ctx, &mut app, vec![]);
    let leaf = held.find_by_author_id(MENU_GO_ATELIER_AUTHOR_ID).expect(
        "the Atelier leaf HELD present across the drain/repaint frame (popup did not close)",
    );
    assert_eq!(leaf.role, "MenuItem", "the held Atelier leaf is a MenuItem");
    assert!(
        !leaf.disabled,
        "the Atelier leaf is ENABLED (its target surface exists), so argus.click can drive it"
    );
    assert!(
        leaf.actions.iter().any(|a| a == "Click"),
        "the Atelier leaf carries a Click action (steerable capability): {:?}",
        leaf.actions
    );

    // Step 2 — argus.click the leaf by author_id, resolved from the OPEN-menu snapshot, and observe the
    // shell steer: the parent Atelier leaf opens the CKC full-window Atelier at the Castkit Codex tab.
    let mut channel2 = ActionChannel::new();
    channel2
        .enqueue(&held, MENU_GO_ATELIER_AUTHOR_ID, UiAction::Click)
        .expect("argus.click on the Atelier leaf resolves + queues");
    let steer_events = channel2.drain_into_events();
    let _ = run_frame(&ctx, &mut app, steer_events);
    let _ = run_frame(&ctx, &mut app, vec![]);

    assert_eq!(
        app.active_module(),
        ModuleId::Ckc,
        "argus.click menu.go.atelier opened the Atelier (CKC full-window module) out-of-process"
    );
    assert_eq!(
        app.atelier_active_tab(),
        AtelierPanelTab::CastkitCodex,
        "the Atelier parent leaf lands on the Castkit Codex default internal tab"
    );

    // The menu closed after the leaf activation (the leaf is gone from the post-steer tree).
    let after = run_frame(&ctx, &mut app, vec![]);
    assert!(
        after.find_by_author_id(MENU_GO_ATELIER_AUTHOR_ID).is_none(),
        "the GO menu closed after the Atelier leaf was activated"
    );
}
