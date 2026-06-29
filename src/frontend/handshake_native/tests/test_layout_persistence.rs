//! WP-KERNEL-011 MT-009 — per-project layout persistence, end-to-end through the REAL `HandshakeApp`.
//!
//! These tests drive the actual shell (not just the `layout_persistence` module's own unit tests) to
//! prove the full capture -> save -> load -> apply round trip preserves every composed MT's state:
//! split weights (MT-006), per-pane tab order/active/pinned (MT-007), pop-out geometry + open state
//! (MT-008), and the pane registry records (MT-005). They also prove the MT-008-deferred restore
//! clamp: a pop-out saved off all monitors reopens at the fallback position, while a legitimate
//! second-monitor position survives.
//!
//! ## No live backend needed (transport stub)
//!
//! The real persistence path is the backend's PostgreSQL-authoritative
//! `GET`/`PUT /workspaces/:id/workbench/layout` REST endpoint (see
//! `layout_persistence::WorkbenchLayoutClient`). To prove the app's capture/apply/lifecycle logic
//! WITHOUT a running `handshake_core`, these tests inject an in-memory [`MemoryTransport`] (a public
//! `LayoutTransport` impl backed by a shared `HashMap`) via `HandshakeApp::set_layout_manager`. The
//! two "shells" share ONE backing map, so a save in shell #1 and a load in shell #2 mirror a real app
//! restart talking to the same backend. The genuine live-backend round trip is the cfg-gated test at
//! the bottom of this file.

use egui_kittest::Harness;
use handshake_native::app::{
    HandshakeApp, HealthDisplayState, DEFAULT_PROJECT_ID, LAYOUT_SAVE_DEBOUNCE,
};
use handshake_native::backend_client::HealthInfo;
use handshake_native::layout_persistence::{
    LayoutError, LayoutPersistenceManager, LayoutTransport,
};
use handshake_native::pane_registry::{PaneId, PaneType};
use handshake_native::split_layout::SplitWeights;
use handshake_native::tab_bar::TabState;
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

fn pid(s: &str) -> PaneId {
    Arc::from(s)
}

/// A generous full-desktop extent so the round-trip tests' geometries are never clamped (the clamp
/// itself is exercised by the dedicated restore-clamp test below).
fn big_desktop() -> egui::Rect {
    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(10_000.0, 10_000.0))
}

/// In-memory `LayoutTransport` backed by a shared map keyed by workspace id. Stands in for the
/// backend's PostgreSQL layout table so two shells sharing one map mirror a real app restart against
/// the same backend, with no live server.
#[derive(Clone, Default)]
struct MemoryTransport {
    store: Arc<Mutex<HashMap<String, Value>>>,
}

impl MemoryTransport {
    fn new() -> Self {
        Self::default()
    }
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

/// Build a fresh shell harness whose layout manager uses `transport` (shared across shells). A near-
/// zero debounce so a single `save_layout_now`/flush resolves immediately in tests.
fn shell(transport: MemoryTransport) -> Harness<'static, HandshakeApp> {
    let mut app = ok_app();
    app.set_layout_manager(LayoutPersistenceManager::new(
        Box::new(transport),
        std::time::Duration::ZERO,
    ));
    Harness::builder().build_state(|ctx, a: &mut HandshakeApp| a.ui(ctx), app)
}

/// Mutate a fresh shell into a NON-default layout, save it, then load+apply it into a SECOND fresh
/// shell. The second shell must end up with the first one's split weights, tab state, and pop-out —
/// proving the full work-surface layout persists across an app restart (the headline MT-009 goal).
#[test]
fn full_layout_round_trips_through_the_app() {
    let transport = MemoryTransport::new();

    // ── Shell #1: change the layout, then save. ─────────────────────────────────────────────────
    let mut h1 = shell(transport.clone());
    h1.run();

    // MT-006: change the split weights away from the default.
    let changed_weights = SplitWeights {
        vertical: 0.31,
        horizontal: 0.74,
    };
    *h1.state_mut().split_weights_mut() = changed_weights;

    // MT-007: add a pinned second tab to pane-a so tab order + pinned + active all carry meaning.
    {
        let app = h1.state_mut();
        let bars = app.tab_bar_states_mut();
        let bar = bars.get_mut(&pid("pane-a")).expect("pane-a tab bar");
        bar.tabs.push(TabState::new(PaneType::Problems));
        let last = bar.tabs.len() - 1;
        bar.tabs[last].pinned = true;
        bar.stabilize_pins();
    }

    // MT-008: pop pane-b out. request_pop_out is consumed at the top of the next ui() frame.
    h1.state_mut().request_pop_out(pid("pane-b"));
    h1.run();
    assert!(
        h1.state().is_popped_out(&pid("pane-b")),
        "pane-b should be popped out before save"
    );

    let saved_json = serde_json::to_value(h1.state().capture_layout_snapshot()).unwrap();
    h1.state().save_layout_now();

    // The blob is now in the shared backing store under the default workspace id.
    assert!(
        transport
            .store
            .lock()
            .unwrap()
            .contains_key(DEFAULT_PROJECT_ID),
        "expected layout blob stored for {DEFAULT_PROJECT_ID}"
    );

    // ── Shell #2: fresh defaults, then load+apply the saved layout. ─────────────────────────────
    let mut h2 = shell(transport.clone());
    h2.run();

    // Sanity: shell #2 starts at defaults (different from what we saved). (The lifecycle's first-frame
    // load runs against DEFAULT_PROJECT_ID; assert the EXPLICIT load below applies the saved blob.)
    let applied = h2
        .state_mut()
        .load_layout(DEFAULT_PROJECT_ID, big_desktop());
    assert!(applied, "a stored snapshot should have been applied");

    // The restored shell's captured snapshot must equal the saved one (serialized-form equality, the
    // persisted contract — see layout_persistence.rs PaneRecord/Instant note).
    let restored_json = serde_json::to_value(h2.state().capture_layout_snapshot()).unwrap();
    assert_eq!(
        saved_json, restored_json,
        "restored layout must equal saved layout"
    );

    // Spot-check the individual composed pieces too (so a future serde change can't make the whole
    // blob match by coincidence while a sub-piece is wrong).
    assert_eq!(
        h2.state().split_weights(),
        changed_weights,
        "MT-006 split weights restored"
    );
    assert!(
        h2.state().is_popped_out(&pid("pane-b")),
        "MT-008 pop-out restored"
    );
    let app2 = h2.state();
    let bar_a = app2
        .tab_bar_states()
        .get(&pid("pane-a"))
        .expect("pane-a bar");
    assert_eq!(
        bar_a.tabs.len(),
        2,
        "MT-007 pane-a has its second tab restored"
    );
    assert!(
        bar_a.tabs.iter().any(|t| t.pinned),
        "MT-007 pinned flag restored"
    );
}

/// First run for a project (no blob) keeps the seeded default layout and reports `false` (default
/// used), without error — proving the missing-blob path is not treated as corruption.
#[test]
fn first_run_keeps_default_layout() {
    let transport = MemoryTransport::new();
    let mut h = shell(transport);
    h.run();

    let applied = h
        .state_mut()
        .load_layout("brand-new-project", big_desktop());
    assert!(!applied, "no stored snapshot -> default layout used");
    assert_eq!(h.state().split_weights(), SplitWeights::default());
}

/// Structurally-corrupt-but-schema-valid blob through the app (MT-009 AC#3): a blob that passes the
/// schema_id + version checks but is MISSING a canonical pane (pane-b) must NOT be applied. With no
/// last-known-good held, the load path keeps the seeded default layout and reports `false` (did not
/// apply persisted) — the validate-before-restore pane-completeness gate. Mirrors the corrupt-blob /
/// fallback assertion style of the module-level `load_corrupt_blob_*` tests.
#[test]
fn load_blob_missing_pane_falls_back_to_default() {
    let transport = MemoryTransport::new();

    // Shell #1: capture a VALID snapshot, then drop pane-b so the stored blob is schema-valid but
    // structurally corrupt, and store it directly under the default workspace id.
    let mut h1 = shell(transport.clone());
    h1.run();
    let mut snap = h1.state().capture_layout_snapshot();
    assert!(
        snap.panes.contains_key(&pid("pane-b")),
        "captured snapshot should seed pane-b before we remove it"
    );
    snap.panes.remove(&pid("pane-b"));
    transport
        .store
        .lock()
        .unwrap()
        .insert(DEFAULT_PROJECT_ID.to_owned(), snap.to_layout_state());

    // Shell #2 (fresh, no LKG): loading the corrupt blob must NOT apply the 3-pane layout.
    let mut h2 = shell(transport.clone());
    h2.run();
    let applied = h2
        .state_mut()
        .load_layout(DEFAULT_PROJECT_ID, big_desktop());
    assert!(
        !applied,
        "a schema-valid but pane-incomplete blob must fall back to default, not be applied"
    );
    // The default seed has all canonical panes; prove the corrupt short layout was not applied.
    let app2 = h2.state();
    for id in ["pane-a", "pane-b", "pane-c"] {
        assert!(
            app2.tab_bar_states().contains_key(&pid(id)),
            "default layout (with {id}) kept, corrupt one-pane layout rejected"
        );
    }
    assert_eq!(
        app2.split_weights(),
        SplitWeights::default(),
        "fallback keeps the seeded default split weights"
    );
}

#[test]
fn load_mt097_two_pane_snapshot_rejects_and_restores_runtime_chat_default() {
    let transport = MemoryTransport::new();

    let mut h1 = shell(transport.clone());
    h1.run();
    let mut snap = h1.state().capture_layout_snapshot();
    assert!(
        snap.panes.contains_key(&pid("pane-c")),
        "MT-098 snapshot should seed Runtime Chat before we simulate an MT-097 layout"
    );
    snap.panes.remove(&pid("pane-c"));
    snap.tab_bars.remove(&pid("pane-c"));
    transport
        .store
        .lock()
        .unwrap()
        .insert(DEFAULT_PROJECT_ID.to_owned(), snap.to_layout_state());

    let mut h2 = shell(transport.clone());
    h2.run();
    let applied = h2
        .state_mut()
        .load_layout(DEFAULT_PROJECT_ID, big_desktop());
    assert!(
        !applied,
        "stale MT-097 two-pane snapshot must be rejected, not applied over Runtime Chat"
    );
    assert!(
        h2.state().tab_bar_states().contains_key(&pid("pane-c")),
        "fallback default restores pane-c Runtime Chat"
    );
}

#[test]
fn load_snapshot_missing_runtime_chat_tab_bar_rejects_and_restores_default() {
    let transport = MemoryTransport::new();

    let mut h1 = shell(transport.clone());
    h1.run();
    let mut snap = h1.state().capture_layout_snapshot();
    assert!(
        snap.panes.contains_key(&pid("pane-c")),
        "test setup: pane-c Runtime Chat remains present"
    );
    assert!(
        snap.tab_bars.remove(&pid("pane-c")).is_some(),
        "test setup removes only the pane-c tab/status surface"
    );
    transport
        .store
        .lock()
        .unwrap()
        .insert(DEFAULT_PROJECT_ID.to_owned(), snap.to_layout_state());

    let mut h2 = shell(transport.clone());
    h2.run();
    let applied = h2
        .state_mut()
        .load_layout(DEFAULT_PROJECT_ID, big_desktop());
    assert!(
        !applied,
        "a pane-c snapshot missing tabbar-pane-c must be rejected, not silently restored"
    );
    assert!(
        h2.state().tab_bar_states().contains_key(&pid("pane-c")),
        "fallback default restores pane-c tab/status state"
    );
}

/// Restore clamp through the app: a pop-out saved OFF all monitors reopens at the fallback position;
/// one on a legitimate second monitor survives. This is the MT-008-deferred restore-time clamp,
/// applied once in `apply_layout_snapshot`.
#[test]
fn restore_clamps_off_monitor_pop_out_through_the_app() {
    let transport = MemoryTransport::new();

    // Shell #1: pop two panes out, then hand-edit their saved geometries and store the blob directly.
    let mut h1 = shell(transport.clone());
    h1.run();
    h1.state_mut().request_pop_out(pid("pane-a"));
    h1.run();
    h1.state_mut().request_pop_out(pid("pane-b"));
    h1.run();
    assert!(h1.state().is_popped_out(&pid("pane-a")));
    assert!(h1.state().is_popped_out(&pid("pane-b")));

    // Hand-edit the captured snapshot's geometries, then store the blob directly so we control the
    // exact saved positions (the live pop_out path uses pointer/fallback positions).
    let mut snap = h1.state().capture_layout_snapshot();
    snap.pop_outs.get_mut(&pid("pane-a")).unwrap().geometry.pos = egui::pos2(50_000.0, 50_000.0); // off all monitors
    snap.pop_outs.get_mut(&pid("pane-b")).unwrap().geometry.pos = egui::pos2(2200.0, 300.0); // second monitor
    transport
        .store
        .lock()
        .unwrap()
        .insert(DEFAULT_PROJECT_ID.to_owned(), snap.to_layout_state());

    // Two monitors: primary 0..1920, secondary 1920..3840; both 0..1080.
    let desktop = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(3840.0, 1080.0));

    // Shell #2: load+apply with that desktop extent.
    let mut h2 = shell(transport.clone());
    h2.run();
    assert!(h2.state_mut().load_layout(DEFAULT_PROJECT_ID, desktop));

    let app2 = h2.state();
    let mgr = app2.popout_manager();
    let off = mgr.get(&pid("pane-a")).expect("pane-a pop-out restored");
    assert_eq!(
        off.geometry.pos,
        handshake_native::popout_window::FALLBACK_POPOUT_POS,
        "off-monitor pop-out must reopen at fallback position"
    );
    let second = mgr.get(&pid("pane-b")).expect("pane-b pop-out restored");
    assert_eq!(
        second.geometry.pos,
        egui::pos2(2200.0, 300.0),
        "second-monitor pop-out must be preserved (not snapped)"
    );
}

/// Lifecycle BLOCKER proof: driving the real `ui()` loop must (a) load the active project's layout on
/// the first frame, and (b) persist a layout-affecting change automatically (debounced) WITHOUT an
/// explicit `save_layout_now` call. Uses a near-zero debounce so the change flushes within the test's
/// frame loop; the flush runs on a worker thread, so the test waits for the shared store to populate.
#[test]
fn lifecycle_loads_then_autosaves_a_change() {
    let transport = MemoryTransport::new();
    let mut h = shell(transport.clone());
    h.run(); // first frame: lifecycle loads (no blob yet -> default), baselines change detection

    // No blob should have been written yet (nothing changed).
    assert!(
        !transport
            .store
            .lock()
            .unwrap()
            .contains_key(DEFAULT_PROJECT_ID),
        "no autosave before any change"
    );

    // Make a layout-affecting change (split weights), then run frames so change detection fires,
    // marks dirty, and the (zero) debounce flushes on a worker.
    *h.state_mut().split_weights_mut() = SplitWeights {
        vertical: 0.2,
        horizontal: 0.8,
    };
    // Several frames: frame N detects the change vs frame N-1, marks dirty; the next frame's
    // due_to_flush is true (zero debounce) and spawns the worker.
    for _ in 0..3 {
        h.run();
    }

    // The worker thread does the actual store insert; poll briefly for it to land.
    let mut saved = false;
    for _ in 0..200 {
        if transport
            .store
            .lock()
            .unwrap()
            .contains_key(DEFAULT_PROJECT_ID)
        {
            saved = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(
        saved,
        "a layout-affecting change autosaved via the debounced lifecycle"
    );

    // The autosaved blob reflects the changed split weights.
    let blob = transport
        .store
        .lock()
        .unwrap()
        .get(DEFAULT_PROJECT_ID)
        .cloned()
        .unwrap();
    // Compare against the f32-widened JSON the snapshot actually serializes (f32 0.2 -> a slightly
    // longer f64 in JSON), not the literal 0.2_f64, so the assertion is exact, not float-fuzzy.
    assert_eq!(
        blob["split_weights"]["vertical"],
        serde_json::json!(0.2_f32)
    );
    assert_eq!(
        blob["split_weights"]["horizontal"],
        serde_json::json!(0.8_f32)
    );

    // Sanity on the debounce constant the app uses (documents the wired value).
    assert!(
        LAYOUT_SAVE_DEBOUNCE.as_millis() > 0,
        "production debounce is non-zero"
    );
}

// ── Live-backend round trip (cfg-gated; needs managed-postgres + handshake_core running) ─────────
//
// This is the ONLY test that exercises the REAL `WorkbenchLayoutClient` against a running
// `handshake_core` (which must be started with managed PostgreSQL, migration 0323 applied) listening
// on 127.0.0.1:37501, with a workspace whose id is set in the HSK_LIVE_WORKSPACE_ID env var.
//
// It is gated behind the `integration_tests` feature (NOT run in the default `cargo test`) because it
// needs out-of-process infrastructure that cannot be stood up headlessly in unit CI. Run with:
//   cargo test --features integration_tests --test test_layout_persistence live_backend_ -- --ignored --nocapture
//
// The unit-level mapping + manager logic (above and in `layout_persistence.rs`) is the REAL,
// always-run proof; this test documents + exercises the genuine PostgreSQL path when the operator
// provides the infrastructure.
#[cfg(feature = "integration_tests")]
#[test]
#[ignore = "needs managed-postgres + handshake_core on 127.0.0.1:37501 and HSK_LIVE_WORKSPACE_ID"]
fn live_backend_layout_round_trips_through_postgres() {
    use handshake_native::backend_client::WorkbenchLayoutClient;

    let workspace_id = std::env::var("HSK_LIVE_WORKSPACE_ID")
        .expect("set HSK_LIVE_WORKSPACE_ID to an existing workspace id");

    // A real runtime for the blocking transport to bridge onto.
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("runtime");
    let client = WorkbenchLayoutClient::production(rt.handle().clone());

    // Build a non-default snapshot, PUT it, GET it back, assert the layout_state round-trips.
    let mut app = ok_app();
    app.set_layout_manager(LayoutPersistenceManager::new(
        Box::new(WorkbenchLayoutClient::production(rt.handle().clone())),
        std::time::Duration::ZERO,
    ));
    *app.split_weights_mut() = SplitWeights {
        vertical: 0.37,
        horizontal: 0.63,
    };
    // Re-stamp the snapshot's project_id to the live workspace id so the load's project check passes.
    let mut snap = app.capture_layout_snapshot();
    snap.project_id = workspace_id.clone();
    let expected = snap.to_layout_state();

    client
        .save(&workspace_id, expected.clone())
        .expect("PUT layout to live backend");
    let got = client
        .load(&workspace_id)
        .expect("GET layout from live backend")
        .expect("backend returned a stored layout");
    assert_eq!(
        got, expected,
        "live PostgreSQL layout_state round-trips identically"
    );
}
