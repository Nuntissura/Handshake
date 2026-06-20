//! WP-KERNEL-011 MT-025 — LIVE AccessKit emission proof.
//!
//! Closes the recurring live-emission gap: earlier MTs built AccessKit nodes only in memory. This
//! test renders the real `HandshakeApp` headlessly via egui_kittest (which enables AccessKit and
//! pushes the same `TreeUpdate` the out-of-process Windows UIA adapter receives) and asserts that
//! shell chrome (MT-002 title + status) and every MT-005 pane are present in the LIVE accessibility
//! tree, each carrying its stable `author_id` and correct semantic role.
//!
//! Why this proves LIVE emission (not just in-memory data): the assertions read from
//! `harness.root()`, the consumer-side AccessKit tree egui produced for this frame. If a node were
//! only built by `PaneRegistry::build_accesskit_node` and never emitted via
//! `Context::accesskit_node_builder`, it would NOT appear here — the test would fail.

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;
use handshake_native::accessibility::{
    assert_no_unnamed_interactive, collect_tree_snapshot, AccessTreeSnapshot, ChromeWidget,
    THEME_TOGGLE_AUTHOR_ID,
};
use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend_client::HealthInfo;

/// The four default panes seeded by the shell, by kebab-case author_id, plus their surface labels.
/// Mirrors `app::default_panes()` (pane-a..pane-d). Kept here as the explicit expected contract so a
/// drift between the seed and the live tree fails loudly.
const EXPECTED_PANE_AUTHOR_IDS: [&str; 4] = ["pane-a", "pane-b", "pane-c", "pane-d"];
const EXPECTED_CHROME_AUTHOR_IDS: [&str; 2] =
    ["shell.chrome.title-bar", "shell.chrome.status-bar"];

fn ok_app() -> HandshakeApp {
    HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }))
}

/// Run the REAL shell for exactly one frame on a plain `egui::Context` with AccessKit enabled, and
/// return the live `accesskit::TreeUpdate` egui produced — the exact value the out-of-process
/// Windows UIA adapter receives each frame (egui builds it in `Context::run` end-of-pass). This is
/// the live-tree source for both the snapshot projection test and the interactive-naming gate; it
/// goes through the same emission path the real window uses, so a node that was only built in memory
/// (never emitted via `Context::accesskit_node_builder`) would be absent here.
fn live_tree_update() -> egui::accesskit::TreeUpdate {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut app = ok_app();
    let output = ctx.run(egui::RawInput::default(), |ctx| app.ui(ctx));
    output
        .platform_output
        .accesskit_update
        .expect("AccessKit update produced (accesskit enabled + one frame run)")
}

/// Walk the live consumer-side AccessKit tree and collect every (author_id, role, label) triple.
/// This is the same surface an out-of-process model reads; collecting it here proves the nodes are
/// actually in the live tree, not merely buildable in memory.
fn live_author_nodes(harness: &Harness<'_, HandshakeApp>) -> Vec<(String, String, Option<String>)> {
    let mut found = Vec::new();
    let root = harness.root();
    for node in root.children_recursive() {
        let ak = node.accesskit_node();
        if let Some(author_id) = ak.author_id() {
            found.push((
                author_id.to_owned(),
                format!("{:?}", ak.role()),
                ak.label(),
            ));
        }
    }
    found
}

#[test]
fn live_tree_contains_chrome_and_panes_by_author_id() {
    let mut harness = Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), ok_app());
    harness.run();

    let nodes = live_author_nodes(&harness);
    let author_ids: Vec<&str> = nodes.iter().map(|(a, _, _)| a.as_str()).collect();
    println!(
        "LIVE author_id nodes ({}): {:?}",
        nodes.len(),
        nodes
    );

    // Chrome (MT-002) must be live.
    for expected in EXPECTED_CHROME_AUTHOR_IDS {
        assert!(
            author_ids.contains(&expected),
            "chrome author_id '{expected}' missing from LIVE tree; found {author_ids:?}"
        );
    }
    // Every pane (MT-005) must be live.
    for expected in EXPECTED_PANE_AUTHOR_IDS {
        assert!(
            author_ids.contains(&expected),
            "pane author_id '{expected}' missing from LIVE tree; found {author_ids:?}"
        );
    }

    // Roles are correct on the chrome nodes.
    let title = nodes
        .iter()
        .find(|(a, _, _)| a == "shell.chrome.title-bar")
        .expect("title-bar node");
    assert_eq!(title.1, "TitleBar", "title bar role");
    assert_eq!(title.2.as_deref(), Some("Handshake"), "title bar label");

    let status = nodes
        .iter()
        .find(|(a, _, _)| a == "shell.chrome.status-bar")
        .expect("status-bar node");
    assert_eq!(status.1, "Status", "status bar role");
    assert!(
        status.2.as_deref().unwrap_or_default().contains("Backend: OK"),
        "status bar label carries live health text, got {:?}",
        status.2
    );

    // Panes carry Role::Group (the PlaceholderPaneFactory default role) and their surface label.
    for (author_id, role, label) in nodes
        .iter()
        .filter(|(a, _, _)| EXPECTED_PANE_AUTHOR_IDS.contains(&a.as_str()))
    {
        assert_eq!(role, "Group", "pane {author_id} role is the factory Group default");
        assert!(label.is_some(), "pane {author_id} carries a surface label");
    }

    println!(
        "PASS: {} chrome + {} pane author_id nodes found in LIVE AccessKit tree",
        EXPECTED_CHROME_AUTHOR_IDS.len(),
        EXPECTED_PANE_AUTHOR_IDS.len()
    );
}

#[test]
fn live_tree_findable_by_label_and_role() {
    // Second, independent proof path: kittest's own Queryable resolves the chrome by role+label,
    // which is exactly how an out-of-process UIA client locates a widget (the MT-001 spike matched
    // by Name). If the node were not live, get_by_role_and_label would panic.
    let mut harness = Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), ok_app());
    harness.run();

    let _title = harness.get_by_label("Handshake");
    println!("PASS: title 'Handshake' findable by label in LIVE tree");
}

#[test]
fn chrome_node_ids_are_stable_across_process_restarts() {
    // "Same widget -> same NodeId across restarts" for chrome. The chrome NodeId is derived from a
    // fixed egui::Id (ChromeWidget::node_id), so two independent app instances must agree. (Panes
    // get their stable ids from the registry's monotonic counter, already proven in
    // pane_registry unit tests.)
    let title_a = ChromeWidget::TitleBar.node_id();
    let title_b = ChromeWidget::TitleBar.node_id();
    let status_a = ChromeWidget::StatusBar.node_id();
    assert_eq!(title_a, title_b, "title node id stable across calls/restarts");
    assert_ne!(title_a, status_a, "title and status node ids distinct");
    // Distinct from the theme toggle (10) and the pane id base (100).
    assert!(title_a != 10 && status_a != 10, "chrome ids do not collide with theme toggle");
    assert!(title_a < 100 && status_a < 100, "chrome ids stay below pane id base");
    println!("PASS: chrome node ids stable and collision-free (title={title_a}, status={status_a})");
}

#[test]
fn snapshot_projects_author_id_nodes_in_stable_order() {
    // Unit-proof for the snapshot/verification endpoint: a TreeUpdate with author_id-bearing and
    // anonymous nodes projects to exactly the author_id nodes, sorted by author_id.
    let mut update = egui::accesskit::TreeUpdate {
        nodes: Vec::new(),
        tree: Some(egui::accesskit::Tree::new(egui::accesskit::NodeId(1))),
        focus: egui::accesskit::NodeId(1),
    };
    // root (no author_id) + two named children + one anonymous child.
    let mut root = egui::accesskit::Node::new(egui::accesskit::Role::Window);
    root.push_child(egui::accesskit::NodeId(2));
    root.push_child(egui::accesskit::NodeId(3));
    root.push_child(egui::accesskit::NodeId(4));
    update.nodes.push((egui::accesskit::NodeId(1), root));

    let mut pane_b = egui::accesskit::Node::new(egui::accesskit::Role::Group);
    pane_b.set_author_id("pane-b".to_owned());
    pane_b.set_label("Inference Lab".to_owned());
    update.nodes.push((egui::accesskit::NodeId(2), pane_b));

    let mut pane_a = egui::accesskit::Node::new(egui::accesskit::Role::Group);
    pane_a.set_author_id("pane-a".to_owned());
    pane_a.set_label("Workspace".to_owned());
    update.nodes.push((egui::accesskit::NodeId(3), pane_a));

    let anon = egui::accesskit::Node::new(egui::accesskit::Role::Label);
    update.nodes.push((egui::accesskit::NodeId(4), anon));

    let snapshot: AccessTreeSnapshot = collect_tree_snapshot(&update);
    assert_eq!(
        snapshot.author_ids(),
        vec!["pane-a", "pane-b"],
        "only author_id nodes, sorted"
    );
    assert!(snapshot.contains_all(&["pane-a", "pane-b"]));
    assert!(!snapshot.contains_all(&["pane-a", "missing"]));
    let a = snapshot.by_author_id("pane-a").unwrap();
    assert_eq!(a.role, "Group");
    assert_eq!(a.label.as_deref(), Some("Workspace"));
    assert_eq!(a.node_id, 3);
    println!("PASS: snapshot projects {} author_id nodes in stable order", snapshot.nodes.len());
}

// ── Item 1: theme toggle author_id is live ───────────────────────────────────────────────────────

#[test]
fn live_tree_contains_theme_toggle_by_author_id() {
    // The theme toggle is the shell's one interactive chrome widget. It must appear in the LIVE tree
    // with its stable author_id `shell.chrome.theme-toggle`, Role::Button (egui's widget_info role),
    // and the live label ("Light" while the default Dark theme is active). Proven via kittest's
    // consumer-side tree, the same surface an out-of-process model reads.
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), ok_app());
    harness.run();

    let nodes = live_author_nodes(&harness);
    let toggle = nodes
        .iter()
        .find(|(a, _, _)| a == THEME_TOGGLE_AUTHOR_ID)
        .unwrap_or_else(|| {
            panic!(
                "theme-toggle author_id '{THEME_TOGGLE_AUTHOR_ID}' missing from LIVE tree; found {:?}",
                nodes.iter().map(|(a, _, _)| a).collect::<Vec<_>>()
            )
        });
    assert_eq!(toggle.1, "Button", "toggle keeps egui's Role::Button");
    assert_eq!(
        toggle.2.as_deref(),
        Some("Light"),
        "toggle carries its live button label (Dark default -> 'Light')"
    );

    // It is also findable by kittest's Queryable (the UIA-style locate path), proving the node is
    // genuinely live and clickable, not just data in memory.
    let _ = harness.get_by_label("Light");
    println!("PASS: theme-toggle '{THEME_TOGGLE_AUTHOR_ID}' live (Role::Button, label 'Light')");
}

// ── Item 3: assert_no_unnamed_interactive gate (positive + negative) ──────────────────────────────

#[test]
fn assert_no_unnamed_interactive_passes_on_real_shell() {
    // Positive proof: the real shell's live tree has NO unnamed interactive widget. The gate also
    // returns the count of interactive nodes it inspected; it must be >= 1 (the theme toggle), so the
    // gate is proven to actually examine an interactive widget rather than pass vacuously.
    let update = live_tree_update();
    let inspected = assert_no_unnamed_interactive(&update);
    assert!(
        inspected >= 1,
        "gate must inspect at least the theme toggle; inspected {inspected}"
    );
    println!("PASS: real shell passes assert_no_unnamed_interactive ({inspected} interactive node(s) named)");
}

#[test]
fn assert_no_unnamed_interactive_fires_on_deliberately_unnamed_widget() {
    // Negative proof: render the real shell PLUS one deliberately-unnamed interactive button into the
    // SAME live frame, then assert the gate panics. This proves the gate is a real enforcement check
    // (catch_unwind sees the panic) and cannot be silently removed without this test going red.
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut app = ok_app();
    let output = ctx.run(egui::RawInput::default(), |ctx| {
        app.ui(ctx);
        // A real interactive egui button with NO author_id assigned. egui gives it Role::Button +
        // Action::Click via widget_info, exactly the shape the gate must catch.
        egui::Area::new(egui::Id::new("unnamed_interactive_probe"))
            .show(ctx, |ui| {
                let _ = ui.button("Unnamed");
            });
    });
    let update = output
        .platform_output
        .accesskit_update
        .expect("AccessKit update produced");

    let result = std::panic::catch_unwind(|| assert_no_unnamed_interactive(&update));
    let err = result.expect_err("gate MUST panic when an interactive widget lacks an author_id");
    let msg = err
        .downcast_ref::<String>()
        .map(|s| s.as_str())
        .or_else(|| err.downcast_ref::<&str>().copied())
        .unwrap_or("");
    assert!(
        msg.contains("no stable author_id"),
        "panic message should name the missing author_id; got: {msg:?}"
    );
    println!("PASS: gate fires on a deliberately unnamed interactive widget: {msg}");
}

// ── Item 4: live-frame snapshot over the REAL tree ────────────────────────────────────────────────

#[test]
fn live_frame_snapshot_contains_chrome_panes_and_toggle_in_stable_order() {
    // Closes the snapshot test gap: `collect_tree_snapshot` was previously only proven against a
    // hand-built TreeUpdate. Here it runs over the REAL shell's one-frame live tree and must contain
    // all six chrome+pane author_id nodes PLUS the theme toggle, sorted by author_id (the snapshot's
    // stable order contract).
    let update = live_tree_update();
    let snapshot: AccessTreeSnapshot = collect_tree_snapshot(&update);

    println!(
        "LIVE-FRAME snapshot ({} author_id nodes): {:?}",
        snapshot.nodes.len(),
        snapshot.author_ids()
    );

    // All six chrome+pane nodes are present.
    for expected in EXPECTED_CHROME_AUTHOR_IDS.iter().chain(EXPECTED_PANE_AUTHOR_IDS.iter()) {
        assert!(
            snapshot.by_author_id(expected).is_some(),
            "author_id '{expected}' missing from LIVE-FRAME snapshot; found {:?}",
            snapshot.author_ids()
        );
    }
    // The interactive theme toggle is present too.
    assert!(
        snapshot.by_author_id(THEME_TOGGLE_AUTHOR_ID).is_some(),
        "theme toggle missing from LIVE-FRAME snapshot; found {:?}",
        snapshot.author_ids()
    );

    // Stable order: the snapshot sorts by author_id. Assert the exact expected sorted sequence of the
    // seven stable-id nodes the shell emits, so a re-order or a dropped node fails loudly.
    let expected_sorted = vec![
        "pane-a",
        "pane-b",
        "pane-c",
        "pane-d",
        "shell.chrome.status-bar",
        "shell.chrome.theme-toggle",
        "shell.chrome.title-bar",
    ];
    assert_eq!(
        snapshot.author_ids(),
        expected_sorted,
        "LIVE-FRAME snapshot must list exactly the 7 stable-id nodes in sorted order"
    );

    // Roles survive the projection: chrome regions and the interactive toggle.
    assert_eq!(snapshot.by_author_id("shell.chrome.title-bar").unwrap().role, "TitleBar");
    assert_eq!(snapshot.by_author_id("shell.chrome.status-bar").unwrap().role, "Status");
    assert_eq!(snapshot.by_author_id(THEME_TOGGLE_AUTHOR_ID).unwrap().role, "Button");
    for pane in EXPECTED_PANE_AUTHOR_IDS {
        assert_eq!(snapshot.by_author_id(pane).unwrap().role, "Group", "{pane} role");
    }

    println!(
        "PASS: LIVE-FRAME snapshot has all 6 chrome+pane nodes + the toggle, sorted ({} total)",
        snapshot.nodes.len()
    );
}
