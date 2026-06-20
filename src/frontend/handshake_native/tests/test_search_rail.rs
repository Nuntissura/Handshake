//! WP-KERNEL-011 MT-022 — Bottom search rail, end-to-end through the REAL `HandshakeApp`.
//!
//! These tests drive the actual shell (not only the `search_rail` module unit tests) with real kittest
//! input — the same out-of-process path a swarm agent uses. Per AC-022-9 the rail makes NO backend call
//! and renders NO results; it only EMITS a parsed `RailQuery` intent into the lock-guarded
//! `search_rail_query` slot, which a downstream search-results consumer (a future MT) reads to execute
//! the search and display results. These tests therefore assert the EMITTED INTENT, not any result set:
//!
//! - the rail renders as an always-visible bottom strip whose nine scope pills + query input + clear +
//!   Loom shortcut are LIVE AccessKit nodes (TextInput / Buttons) addressable by stable author_id;
//! - clicking the `file:` pill changes the active scope (but does NOT emit an intent — AC-022-3);
//! - typing a query + pressing Enter EMITS a scoped intent into the slot with the correct scope +
//!   free-text (AC-022-5);
//! - a typed scope-prefix (`project:hello world`) overrides the active pill scope (AC-022-4);
//! - the Loom shortcut forces the `project:` scope and emits;
//! - the clear `x` button empties the input + resets the scope + writes `None` into the slot (AC-022-6);
//! - the shared slot is observable off the same lock by a concurrent reader (HBR-SWARM).
//!
//! No live backend, no transport, no PostgreSQL: the rail performs no I/O, so these tests need none.

use std::time::{Duration, Instant};

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;
use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend_client::HealthInfo;
use handshake_native::search_rail::{
    RailQuery, SearchScope, RAIL_CLEAR_AUTHOR_ID, RAIL_INPUT_AUTHOR_ID, RAIL_LOOM_AUTHOR_ID,
};

// The rail query input is the only TextInput in the default frame (palette/switcher/settings closed),
// so kittest's `get_by_role(TextInput)` resolves it. The clear + Loom buttons are located by their
// labels (kittest 0.3 has no `get_by_author_id`; the controls carry author_ids for out-of-process
// steering but are located here by role/label, which is the same UIA-style locate path).
const CLEAR_LABEL: &str = "Clear the search";
const LOOM_LABEL: &str = "Search the whole project (Loom)";

fn ok_app() -> HandshakeApp {
    HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }))
}

/// Build a kittest harness over the headless shell. The rail makes no backend call, so no runtime/
/// transport injection is needed — the emitted intent is written straight into the shared slot.
fn shell_harness() -> Harness<'static, HandshakeApp> {
    Harness::builder().build_state(
        move |ctx, a: &mut HandshakeApp| a.ui(ctx),
        ok_app(),
    )
}

/// Step single frames in a loop until `pred` holds or a timeout elapses. The rail does no async I/O,
/// but Enter/click effects settle over a couple of frames, so a short bounded loop keeps the assertion
/// stable without depending on exact frame counts.
fn step_until(harness: &mut Harness<'_, HandshakeApp>, pred: impl Fn(&HandshakeApp) -> bool) -> bool {
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        harness.step();
        if pred(harness.state()) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    harness.step();
    pred(harness.state())
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

// ── The rail renders 9 pills + input + clear + Loom as live nodes; no intent before a fire ───────────

#[test]
fn rail_renders_pills_input_and_buttons_live() {
    let mut harness = shell_harness();
    harness.run();

    let nodes = live_author_nodes(&harness);

    // The input is a live TextInput.
    let input = nodes
        .iter()
        .find(|(a, _, _)| a == RAIL_INPUT_AUTHOR_ID)
        .unwrap_or_else(|| panic!("rail input missing: {nodes:?}"));
    assert_eq!(input.1, "TextInput", "rail input role");

    // The clear + Loom controls are live Buttons.
    for btn in [RAIL_CLEAR_AUTHOR_ID, RAIL_LOOM_AUTHOR_ID] {
        let n = nodes
            .iter()
            .find(|(a, _, _)| a == btn)
            .unwrap_or_else(|| panic!("rail button {btn} missing: {nodes:?}"));
        assert_eq!(n.1, "Button", "{btn} role");
    }

    // All nine scope pills are present as Buttons (PROOF-022-2a).
    let pills: Vec<&str> = nodes
        .iter()
        .filter(|(a, _, _)| a.starts_with("bottom-rail.scope."))
        .map(|(a, _, _)| a.as_str())
        .collect();
    assert_eq!(pills.len(), 9, "all nine scope pills render: {pills:?}");
    for scope in SearchScope::all() {
        assert!(
            pills.contains(&scope.author_id().as_str()),
            "pill for {scope:?} present: {pills:?}"
        );
    }

    // The rail renders NO result rows (AC-022-9: execution + display are deferred to a downstream
    // consumer) and no intent has been emitted before a fire.
    assert!(
        !nodes.iter().any(|(a, _, _)| a.starts_with("bottom-rail.result.")),
        "the rail renders no result rows (results deferred to the downstream consumer): {nodes:?}"
    );
    assert!(harness.state().search_rail_query().is_none(), "no emitted intent before a fire");
}

// ── Clicking the file: pill changes the active scope but does NOT emit an intent (AC-022-3) ──────────

#[test]
fn clicking_file_pill_sets_scope_without_emitting() {
    let mut harness = shell_harness();
    harness.run();

    harness.get_by_label(format!("Scope {}", SearchScope::File).as_str()).click();
    harness.run();
    harness.run();

    // No intent emitted from a pill click (AC-022-3).
    assert!(
        harness.state().search_rail_query().is_none(),
        "clicking a pill does not emit an intent"
    );

    // Now type + Enter -> the intent is emitted with the File scope (the pill selection took effect).
    harness.get_by_role(egui::accesskit::Role::TextInput).focus();
    harness.run();
    harness.get_by_role(egui::accesskit::Role::TextInput).type_text("readme");
    harness.key_press(egui::Key::Enter);
    let emitted = step_until(&mut harness, |app| app.search_rail_query().is_some());
    assert!(emitted, "Enter emitted the scoped intent");
    let q = harness.state().search_rail_query().expect("intent recorded");
    assert_eq!(q.scope, SearchScope::File, "the File pill scoped the emitted intent");
    assert_eq!(q.free_text, "readme");
}

// ── Typing project:hello world + Enter emits the project: scope with the residual free-text (AC-4/5) ──

#[test]
fn typed_scope_prefix_overrides_in_emitted_intent() {
    let mut harness = shell_harness();
    harness.run();

    // Type a query with a typed scope-prefix into the focused input, then Enter.
    harness.get_by_role(egui::accesskit::Role::TextInput).focus();
    harness.run();
    harness.get_by_role(egui::accesskit::Role::TextInput).type_text("project:hello world");
    harness.key_press(egui::Key::Enter);

    let emitted = step_until(&mut harness, |app| app.search_rail_query().is_some());
    assert!(emitted, "the typed-prefix intent was emitted");

    // The emitted intent: scope=Project (typed prefix won), free_text="hello world" (prefix stripped).
    let q = harness.state().search_rail_query().expect("emitted intent");
    assert_eq!(q.scope, SearchScope::Project, "typed project: overrode the pill scope");
    assert_eq!(q.free_text, "hello world", "the prefix was stripped from the free-text");

    // The slot is observable off the SAME lock by a concurrent reader (HBR-SWARM): a downstream
    // consumer holds an Arc clone and clones-and-reads the latest emitted intent.
    let slot = harness.state().search_rail_query_slot();
    let read_back: RailQuery = slot.lock().unwrap().clone().expect("reader sees the emitted intent");
    assert_eq!(read_back.scope, SearchScope::Project);
    assert_eq!(read_back.free_text, "hello world");
}

// ── The Loom shortcut forces the project: scope and emits ─────────────────────────────────────────────

#[test]
fn loom_shortcut_forces_project_scope_and_emits() {
    let mut harness = shell_harness();
    harness.run();

    // Select a NON-project pill first, type, then click Loom: the Loom shortcut must override to Project.
    harness.get_by_label(format!("Scope {}", SearchScope::File).as_str()).click();
    harness.run();
    harness.get_by_role(egui::accesskit::Role::TextInput).focus();
    harness.run();
    harness.get_by_role(egui::accesskit::Role::TextInput).type_text("anything");
    harness.run();
    harness.get_by_label(LOOM_LABEL).click();

    let emitted = step_until(&mut harness, |app| app.search_rail_query().is_some());
    assert!(emitted, "the Loom shortcut emitted an intent");
    let q = harness.state().search_rail_query().expect("intent recorded");
    assert_eq!(q.scope, SearchScope::Project, "Loom forced the project: scope despite the File pill");
    assert_eq!(q.free_text, "anything");
}

// ── The clear (x) button empties the input, resets the scope to Project, clears the slot (AC-022-6) ──

#[test]
fn clear_button_resets_input_scope_and_slot() {
    let mut harness = shell_harness();
    harness.run();

    // Emit a File-scoped intent so there is state to clear.
    harness.get_by_label(format!("Scope {}", SearchScope::File).as_str()).click();
    harness.run();
    harness.get_by_role(egui::accesskit::Role::TextInput).focus();
    harness.run();
    harness.get_by_role(egui::accesskit::Role::TextInput).type_text("query");
    harness.key_press(egui::Key::Enter);
    let emitted = step_until(&mut harness, |app| app.search_rail_query().is_some());
    assert!(emitted, "an intent is present before clear");

    // Click the clear button.
    harness.get_by_label(CLEAR_LABEL).click();
    harness.run();
    harness.run();

    // The slot was reset to None (AC-022-6).
    assert!(
        harness.state().search_rail_query().is_none(),
        "clear wrote None into the emitted-intent slot"
    );

    // The input is empty + the scope reset to Project: a fresh emit (after clear) carries no leftover.
    // Type a bare query (no scope prefix) and Enter; the emitted scope must be the default Project,
    // proving the clear reset the active pill from File back to Project (AC-022-6).
    harness.get_by_role(egui::accesskit::Role::TextInput).focus();
    harness.run();
    harness.get_by_role(egui::accesskit::Role::TextInput).type_text("fresh");
    harness.key_press(egui::Key::Enter);
    let fired = step_until(&mut harness, |app| {
        app.search_rail_query().map(|q| q.free_text) == Some("fresh".to_owned())
    });
    assert!(fired, "a fresh intent was emitted after clear");
    let emitted = harness.state().search_rail_query().expect("emitted");
    assert_eq!(emitted.scope, SearchScope::Project, "clear reset the scope to Project");
    assert_eq!(emitted.free_text, "fresh");
}
