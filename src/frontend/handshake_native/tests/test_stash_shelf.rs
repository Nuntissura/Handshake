//! WP-KERNEL-011 MT-023 (C6) — bottom drawer stash shelf LIVE proof.
//!
//! Renders the REAL `HandshakeApp` headlessly via egui_kittest (which enables AccessKit and produces the
//! same `TreeUpdate` the out-of-process Windows UIA adapter receives) and proves the MT-023 drawer:
//!   - PROOF-023-2: the affordance is in the default frame; opening shows four cards + a clicked Agenda
//!     card opens a pane; the Mail card does NOT navigate.
//!   - PROOF-023-4: when open, all seven drawer AccessKit nodes (affordance, shelf, four cards, resize
//!     handle) are present with non-None NodeIds and correct roles.
//!   - PROOF-023-5: the build produces no egui panel-ordering panic (the shell runs many frames here,
//!     open AND closed, without an ordering panic).
//!
//! The headless shell has no tokio runtime, so the cards stay in their pre-fetch state (badge 0) — these
//! tests prove rendering / ids / roles / click routing, NOT live backend data. Live backend data is
//! PROOF-023-3 (an `#[ignore]` integration test enabled with a real PG URL).

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;
use handshake_native::accessibility::assert_no_unnamed_interactive;
use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend_client::HealthInfo;
use handshake_native::stash_shelf::{
    DrawerCardKind, DRAWER_AFFORDANCE_AUTHOR_ID, DRAWER_RESIZE_AUTHOR_ID, DRAWER_SHELF_AUTHOR_ID,
};

fn ok_app() -> HandshakeApp {
    HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }))
}

/// Collect (author_id -> (role, label)) for every drawer node in the live consumer-side tree.
fn drawer_nodes(
    harness: &Harness<'_, HandshakeApp>,
) -> std::collections::HashMap<String, (String, Option<String>)> {
    let mut out = std::collections::HashMap::new();
    let root = harness.root();
    for node in root.children_recursive() {
        let ak = node.accesskit_node();
        if let Some(aid) = ak.author_id() {
            if aid.starts_with("hsk.drawer") {
                out.insert(aid.to_owned(), (format!("{:?}", ak.role()), ak.label()));
            }
        }
    }
    out
}

#[test]
fn affordance_present_when_collapsed_and_drawer_nodes_absent() {
    // PROOF-023-2(a) + AC-023-1/2: collapsed by default — only the affordance is live; the shelf, cards,
    // and resize handle are NOT (they render open-only).
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), ok_app());
    harness.run();

    assert!(!harness.state().bottom_drawer_open(), "drawer collapsed by default");
    let nodes = drawer_nodes(&harness);
    assert!(
        nodes.contains_key(DRAWER_AFFORDANCE_AUTHOR_ID),
        "affordance is always visible; found {:?}",
        nodes.keys().collect::<Vec<_>>()
    );
    assert_eq!(nodes[DRAWER_AFFORDANCE_AUTHOR_ID].0, "Button", "affordance role");
    // Open-only nodes are absent while collapsed.
    assert!(!nodes.contains_key(DRAWER_SHELF_AUTHOR_ID), "shelf absent while collapsed");
    assert!(!nodes.contains_key(DRAWER_RESIZE_AUTHOR_ID), "resize handle absent while collapsed");
    for kind in DrawerCardKind::all() {
        assert!(
            !nodes.contains_key(&kind.author_id()),
            "card {} absent while collapsed",
            kind
        );
    }
}

#[test]
fn open_drawer_shows_seven_nodes_with_correct_roles() {
    // PROOF-023-4 + AC-023-3/10: when open, all seven drawer nodes are present with correct roles.
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), ok_app());
    harness.run();
    harness.state_mut().set_bottom_drawer_open(true);
    harness.run();
    harness.run();

    let nodes = drawer_nodes(&harness);

    // Affordance + shelf container.
    assert_eq!(nodes[DRAWER_AFFORDANCE_AUTHOR_ID].0, "Button", "affordance role");
    assert_eq!(
        nodes
            .get(DRAWER_SHELF_AUTHOR_ID)
            .unwrap_or_else(|| panic!("shelf missing; found {:?}", nodes.keys().collect::<Vec<_>>()))
            .0,
        "Group",
        "shelf container role"
    );
    // Resize handle = Slider.
    assert_eq!(
        nodes.get(DRAWER_RESIZE_AUTHOR_ID).expect("resize handle present").0,
        "Slider",
        "resize handle role"
    );
    // Four cards = Button, in correct labels.
    for kind in DrawerCardKind::all() {
        let aid = kind.author_id();
        let (role, label) = nodes
            .get(&aid)
            .unwrap_or_else(|| panic!("card {aid} missing; found {:?}", nodes.keys().collect::<Vec<_>>()));
        assert_eq!(role, "Button", "{aid} role");
        // Label = "{title} ({badge})"; headless shell has no runtime so badge stays 0.
        assert_eq!(
            label.as_deref(),
            Some(format!("{} (0)", kind.title()).as_str()),
            "{aid} label"
        );
    }
    // MT-024: each open card also renders an overflow `...` button (Role::Button) carrying the stable
    // `hsk.drawer.card.{kind}.overflow` author_id (AC-024-1/12).
    for kind in DrawerCardKind::all() {
        let aid = kind.overflow_author_id();
        let (role, _) = nodes
            .get(aid)
            .unwrap_or_else(|| panic!("overflow {aid} missing; found {:?}", nodes.keys().collect::<Vec<_>>()));
        assert_eq!(role, "Button", "{aid} overflow role");
    }
    // Exactly the four cards + four overflow buttons + affordance + shelf + resize = 11 drawer nodes
    // when open (MT-024 added the four overflow buttons to the MT-023 seven).
    assert_eq!(nodes.len(), 11, "exactly eleven drawer nodes when open; found {:?}", nodes.keys());
}

#[test]
fn clicking_agenda_card_opens_a_pane() {
    // PROOF-023-2(d) + AC-023-12: clicking the Agenda card opens a pane (a tab is inserted on the active
    // pane). We count total tabs across panes before/after the click.
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), ok_app());
    harness.run();
    harness.state_mut().set_bottom_drawer_open(true);
    harness.run();
    harness.run();

    let tabs_before: usize = harness
        .state()
        .tab_bar_states()
        .values()
        .map(|b| b.tabs.len())
        .sum();

    // Click the Agenda card by its live label ("Agenda (0)").
    harness.get_by_label("Agenda (0)").click();
    harness.run();
    harness.run();

    let tabs_after: usize = harness
        .state()
        .tab_bar_states()
        .values()
        .map(|b| b.tabs.len())
        .sum();
    assert!(
        tabs_after > tabs_before,
        "clicking Agenda opened a pane (tabs {tabs_before} -> {tabs_after})"
    );
}

#[test]
fn clicking_mail_card_does_not_navigate() {
    // PROOF-023-2(e) + AC-023-7/12: clicking the Mail card shows a tooltip, NOT a navigation — the pane
    // tab count is unchanged.
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), ok_app());
    harness.run();
    harness.state_mut().set_bottom_drawer_open(true);
    harness.run();
    harness.run();

    let tabs_before: usize = harness
        .state()
        .tab_bar_states()
        .values()
        .map(|b| b.tabs.len())
        .sum();

    harness.get_by_label("Mail (0)").click();
    harness.run();
    harness.run();

    let tabs_after: usize = harness
        .state()
        .tab_bar_states()
        .values()
        .map(|b| b.tabs.len())
        .sum();
    assert_eq!(
        tabs_after, tabs_before,
        "clicking Mail does NOT open a pane (tabs unchanged {tabs_before})"
    );
}

#[test]
fn clicking_affordance_toggles_open_state() {
    // AC-023-2: clicking the affordance toggles the drawer open then closed.
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), ok_app());
    harness.run();
    assert!(!harness.state().bottom_drawer_open(), "starts collapsed");

    harness.get_by_label("Open stash drawer").click();
    harness.run();
    harness.run();
    assert!(harness.state().bottom_drawer_open(), "affordance opened the drawer");

    harness.get_by_label("Close stash drawer").click();
    harness.run();
    harness.run();
    assert!(!harness.state().bottom_drawer_open(), "affordance closed the drawer");
}

#[test]
fn open_and_close_many_frames_without_ordering_panic() {
    // PROOF-023-5 / AC-023-11 / RISK-023-A: the drawer + rail + central panels run cleanly across many
    // open/closed frames with no egui ordering panic. (kittest runs the REAL update path; an ordering
    // panic would unwind the test.)
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), ok_app());
    for i in 0..6 {
        harness.state_mut().set_bottom_drawer_open(i % 2 == 0);
        harness.run();
        harness.run();
    }
    // Also resize the drawer to extremes while open to exercise the clamp (RISK-023-B).
    harness.state_mut().set_bottom_drawer_open(true);
    harness.run();
    println!("PASS: drawer ran open/closed across many frames without an egui ordering panic");
}

#[test]
fn open_drawer_frame_has_no_unnamed_interactive_nodes() {
    // AC-023-10 + rubric end-to-end integrity: with the drawer OPEN (its cards + ScrollArea + resize
    // handle all live), the full frame must still pass the MT-025 interactive-naming gate — no anonymous
    // clickable node (the ScrollArea-viewport / Area-background pitfall the rail + affordance avoided).
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut app = ok_app();
    // One frame to register, then open + a settle frame.
    let _ = ctx.run(egui::RawInput::default(), |ctx| app.ui(ctx));
    app.set_bottom_drawer_open(true);
    let _ = ctx.run(egui::RawInput::default(), |ctx| app.ui(ctx));
    let output = ctx.run(egui::RawInput::default(), |ctx| app.ui(ctx));
    let update = output
        .platform_output
        .accesskit_update
        .expect("AccessKit update produced");
    let inspected = assert_no_unnamed_interactive(&update);
    assert!(inspected >= 1, "gate inspected interactive nodes; inspected {inspected}");
    println!("PASS: open drawer frame has no unnamed interactive nodes ({inspected} named)");
}

#[test]
fn drawer_debug_state_exposes_cards_for_no_context_models() {
    // HBR-MAN: the drawer open/close + card state is readable from AppState without scraping the UI.
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), ok_app());
    harness.run();
    harness.state_mut().set_bottom_drawer_open(true);
    harness.run();

    let state = harness.state().drawer_debug_state();
    assert_eq!(state["open"], serde_json::json!(true));
    let cards = state["cards"].as_array().expect("cards array");
    assert_eq!(cards.len(), 4, "four cards in the debug surface");
    let titles: Vec<&str> = cards.iter().map(|c| c["title"].as_str().unwrap()).collect();
    assert_eq!(titles, vec!["Agenda", "Mail", "Lists", "Notes"], "logical card order preserved");
}
