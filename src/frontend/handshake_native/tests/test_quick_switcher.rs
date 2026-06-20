//! WP-KERNEL-011 MT-017 — Quick Switcher overlay, end-to-end through the REAL `HandshakeApp`.
//!
//! These tests drive the actual shell (not only the `quick_switcher` module unit tests) to prove the
//! MT-017 behavior with real kittest input — the same out-of-process path a swarm agent uses:
//!
//! - opening via the flag (the GO-menu / Ctrl+P seam) renders a centred always-on-top overlay with a
//!   Dialog root, a SearchBox/TextInput, and a ListBox in the live AccessKit tree;
//! - the search input receives focus automatically on open;
//! - typing a query fires a Loom-graph search and the returned hits render as ListBoxOption rows with
//!   the source-kind chip + title;
//! - Enter on the selection OPENS the hit's typed target (a tab on the active pane) and closes the
//!   switcher;
//! - Escape closes the switcher without navigating.
//!
//! ## Stub transport, no live backend
//!
//! The switcher's backend I/O goes through a [`LoomGraphSearchTransport`] seam. These tests inject an
//! in-memory `StubTransport` (canned hits + recents) and a real multi-thread tokio runtime handle so
//! the spawned async tasks actually run and deliver — proving the full open→search→render→open path
//! with NO live PostgreSQL/handshake_core. The genuine PostgreSQL path is exercised by the
//! `#[cfg(feature = "integration_tests")]` live test below when the operator provides the backend.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;
use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend_client::HealthInfo;
use handshake_native::pane_registry::PaneType;
use handshake_native::quick_switcher::{
    LoomGraphSearchHit, LoomGraphSearchTransport, SearchTransportError, SWITCHER_DIALOG_AUTHOR_ID,
    SWITCHER_LIST_AUTHOR_ID, SWITCHER_SEARCH_AUTHOR_ID,
};
use serde_json::{json, Value};

fn ok_app() -> HandshakeApp {
    HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }))
}

/// The switcher's search box, disambiguated from the always-visible MT-022 bottom search rail input
/// (which is ALSO a `Role::TextInput` in every frame). The rail input carries the stable author_id
/// `bottom-rail.input`; the switcher search box does not, so we pick the non-rail TextInput. (Before
/// MT-022 the switcher search was the only TextInput on screen; the rail made that ambiguous.)
fn switcher_search<'h>(harness: &'h Harness<'_, HandshakeApp>) -> egui_kittest::Node<'h> {
    harness
        .query_all_by_role(egui::accesskit::Role::TextInput)
        .find(|n| n.accesskit_node().author_id() != Some("bottom-rail.input"))
        .expect("the switcher search TextInput (the non-rail one)")
}

/// A canned, in-memory transport: returns the same hits for any query, a fixed recents list, and
/// echoes the picked hit's key from `record_recent`. No network, deterministic.
struct StubTransport {
    hits: Vec<LoomGraphSearchHit>,
    recents: Vec<String>,
}

impl LoomGraphSearchTransport for StubTransport {
    fn search(&self, _ws: &str, _q: &str) -> Result<Vec<LoomGraphSearchHit>, SearchTransportError> {
        Ok(self.hits.clone())
    }
    fn list_recents(&self, _ws: &str) -> Result<Vec<String>, SearchTransportError> {
        Ok(self.recents.clone())
    }
    fn record_recent(
        &self,
        _ws: &str,
        hit: &LoomGraphSearchHit,
    ) -> Result<String, SearchTransportError> {
        Ok(format!("{}:{}", hit.source_kind, hit.ref_id))
    }
}

fn make_hit(source_kind: &str, ref_id: &str, title: &str, metadata: Value) -> LoomGraphSearchHit {
    LoomGraphSearchHit {
        result_kind: "knowledge_entity".to_owned(),
        source_kind: source_kind.to_owned(),
        ref_id: ref_id.to_owned(),
        title: title.to_owned(),
        excerpt: String::new(),
        block: Value::Null,
        score: 1.0,
        metadata,
    }
}

/// Build a kittest harness whose shell is wired with a stub transport + a real multi-thread runtime so
/// the switcher's async search/recents tasks actually run. The runtime is leaked (test-lifetime) so its
/// handle stays valid for the whole harness.
fn shell_harness_with_stub(stub: StubTransport) -> Harness<'static, HandshakeApp> {
    let rt = Box::leak(Box::new(
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("runtime"),
    ));
    let handle = rt.handle().clone();
    let transport: Arc<dyn LoomGraphSearchTransport> = Arc::new(stub);
    Harness::builder().build_state(
        move |ctx, a: &mut HandshakeApp| a.ui(ctx),
        {
            let mut app = ok_app();
            app.set_runtime_handle(handle.clone());
            app.set_quick_switcher_transport(transport.clone());
            app
        },
    )
}

/// Step single frames in a loop (with short sleeps to cross the 150ms debounce and let spawned tasks
/// deliver) until `pred` holds or a timeout elapses. Uses `step` (one frame) NOT `run` because the open
/// switcher requests a continuous repaint while loading/debouncing, which would make `run` (loop-until-
/// idle) exceed its step cap. Returns whether `pred` held.
fn step_until(harness: &mut Harness<'_, HandshakeApp>, pred: impl Fn(&HandshakeApp) -> bool) -> bool {
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        harness.step();
        if pred(harness.state()) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
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

// ── Opening renders the Dialog/SearchBox/ListBox; empty query shows no rows ───────────────────────────

#[test]
fn opening_switcher_renders_dialog_searchbox_listbox() {
    let mut harness = shell_harness_with_stub(StubTransport {
        hits: vec![],
        recents: vec![],
    });
    harness.run();
    let before = live_author_nodes(&harness);
    assert!(
        !before.iter().any(|(a, _, _)| a == SWITCHER_DIALOG_AUTHOR_ID),
        "switcher dialog absent while closed: {before:?}"
    );

    harness.state_mut().open_quick_switcher();
    harness.run();
    harness.run();

    let nodes = live_author_nodes(&harness);
    let dialog = nodes
        .iter()
        .find(|(a, _, _)| a == SWITCHER_DIALOG_AUTHOR_ID)
        .unwrap_or_else(|| panic!("switcher dialog missing: {nodes:?}"));
    assert_eq!(dialog.1, "Dialog", "switcher root role is Dialog");

    let search = nodes
        .iter()
        .find(|(a, _, _)| a == SWITCHER_SEARCH_AUTHOR_ID)
        .unwrap_or_else(|| panic!("switcher search box missing: {nodes:?}"));
    assert_eq!(search.1, "TextInput", "switcher search role is TextInput");

    let list = nodes
        .iter()
        .find(|(a, _, _)| a == SWITCHER_LIST_AUTHOR_ID)
        .unwrap_or_else(|| panic!("switcher list missing: {nodes:?}"));
    assert_eq!(list.1, "ListBox", "switcher list role is ListBox");

    // Empty query => no result rows.
    assert!(
        !nodes.iter().any(|(a, _, _)| a.starts_with("quick-switcher.option.")),
        "no result rows for an empty query: {nodes:?}"
    );
}

// ── The search input is focused automatically on open ────────────────────────────────────────────────

#[test]
fn search_input_is_focused_on_open() {
    let mut harness = shell_harness_with_stub(StubTransport {
        hits: vec![],
        recents: vec![],
    });
    harness.run();
    harness.state_mut().open_quick_switcher();
    harness.run();
    harness.run();

    let search = switcher_search(&harness);
    assert!(search.is_focused(), "the search input has keyboard focus on open");
}

// ── Typing a query fires the graph-search; the returned hits render as ListBoxOption rows ─────────────

#[test]
fn typing_fires_graph_search_and_renders_hits() {
    let stub = StubTransport {
        hits: vec![
            make_hit("document", "KRD-101", "Design Doc", json!({ "rich_document_id": "KRD-101" })),
            make_hit("work_packet", "WP-9", "Work Packet Nine", json!({ "work_packet_id": "WP-9" })),
        ],
        recents: vec![],
    };
    let mut harness = shell_harness_with_stub(stub);
    harness.run();
    harness.state_mut().open_quick_switcher();
    harness.run();
    harness.run();

    // Type into the focused search box (the genuine keyboard path).
    switcher_search(&harness).type_text("design");

    // Pump single frames + wall-clock past the 150ms debounce until the search delivers and rows render.
    let rendered = step_until(&mut harness, |app| !app.quick_switcher_search_results().is_empty());
    assert!(rendered, "graph-search delivered hits within the timeout");
    harness.step();
    harness.step();

    let nodes = live_author_nodes(&harness);
    let rows: Vec<(&str, Option<&str>)> = nodes
        .iter()
        .filter(|(a, _, _)| a.starts_with("quick-switcher.option."))
        .map(|(a, _, label)| (a.as_str(), label.as_deref()))
        .collect();

    assert!(
        rows.iter().any(|(a, label)| *a == "quick-switcher.option.document.krd-101"
            && *label == Some("Design Doc")),
        "Design Doc document row rendered: {rows:?}"
    );
    assert!(
        rows.iter().any(|(a, _)| *a == "quick-switcher.option.work_packet.wp-9"),
        "WP-9 work-packet row rendered: {rows:?}"
    );
}

// ── Enter on the selection OPENS the hit's typed target (a tab on the active pane) + closes ───────────

#[test]
fn enter_opens_hit_target_and_closes() {
    let stub = StubTransport {
        hits: vec![make_hit(
            "work_packet",
            "WP-KERNEL-011",
            "WorkSurface Shell",
            json!({ "work_packet_id": "WP-KERNEL-011" }),
        )],
        recents: vec![],
    };
    let mut harness = shell_harness_with_stub(stub);
    harness.run();
    assert!(harness.state().active_pane().is_none(), "no active pane before");

    harness.state_mut().open_quick_switcher();
    harness.run();
    harness.run();

    switcher_search(&harness).type_text("shell");
    // Cross the debounce + let the delivery arrive so the single row is selectable (step, not run,
    // because the open switcher repaints continuously while loading/debouncing).
    let rendered = step_until(&mut harness, |app| !app.quick_switcher_search_results().is_empty());
    assert!(rendered, "graph-search delivered the work-packet hit");
    harness.step();
    harness.step();

    // Press Enter — opens the selected hit's target (a Kernel DCC tab) and closes the switcher.
    harness.key_press(egui::Key::Enter);
    harness.step();
    harness.step();

    assert!(
        !harness.state().quick_switcher_open(),
        "Enter on a hit closed the switcher"
    );
    // The work-packet target opened a Kernel DCC tab on the active pane.
    let active = harness
        .state()
        .active_pane()
        .map(|p| p.to_string())
        .expect("active pane set after open");
    let bar = harness
        .state()
        .tab_bar_states()
        .get(active.as_str())
        .expect("active pane tab bar");
    let opened = bar.active().expect("an active tab");
    assert_eq!(
        opened.pane_type,
        PaneType::KernelDcc,
        "work-packet hit opened a Kernel DCC tab"
    );
    assert_eq!(
        opened.content_id.as_deref(),
        Some("WP:WP-KERNEL-011"),
        "the Kernel DCC tab focuses the picked work packet"
    );
    // The recent was recorded (optimistic local update from record_recent).
    assert_eq!(
        harness.state().quick_switcher_recents().first().map(|s| s.as_str()),
        Some("work_packet:WP-KERNEL-011"),
        "the open recorded the durable recent key"
    );
}

// ── Escape closes the switcher without navigating ────────────────────────────────────────────────────

#[test]
fn escape_closes_without_navigating() {
    let mut harness = shell_harness_with_stub(StubTransport {
        hits: vec![make_hit("symbol", "sym-1", "Symbol One", Value::Null)],
        recents: vec![],
    });
    harness.run();
    harness.state_mut().open_quick_switcher();
    harness.run();
    harness.run();
    assert!(harness.state().quick_switcher_open(), "switcher open before Escape");

    harness.key_press(egui::Key::Escape);
    harness.run();
    harness.run();

    assert!(!harness.state().quick_switcher_open(), "Escape closed the switcher");
    assert!(
        harness.state().active_pane().is_none(),
        "Escape did not change the active pane"
    );
    assert!(
        harness.state().quick_switcher_recents().is_empty(),
        "Escape recorded no recent"
    );
    let nodes = live_author_nodes(&harness);
    assert!(
        !nodes.iter().any(|(a, _, _)| a == SWITCHER_DIALOG_AUTHOR_ID),
        "switcher dialog gone after Escape: {nodes:?}"
    );
}

// ── The Close button closes the switcher ─────────────────────────────────────────────────────────────

#[test]
fn close_button_closes_switcher() {
    let mut harness = shell_harness_with_stub(StubTransport {
        hits: vec![],
        recents: vec![],
    });
    harness.run();
    harness.state_mut().open_quick_switcher();
    harness.run();
    harness.run();

    harness.get_by_label("Close").click();
    harness.run();
    harness.run();

    assert!(!harness.state().quick_switcher_open(), "Close button closed the switcher");
}

// ── Re-open bumps the open generation (transient state reset) ─────────────────────────────────────────

#[test]
fn reopen_bumps_open_generation() {
    let mut harness = shell_harness_with_stub(StubTransport {
        hits: vec![],
        recents: vec![],
    });
    harness.run();

    harness.state_mut().open_quick_switcher();
    harness.run();
    let first = harness.state().quick_switcher_open_count();
    harness.key_press(egui::Key::Escape);
    harness.run();
    harness.run();

    harness.state_mut().open_quick_switcher();
    harness.run();
    assert!(
        harness.state().quick_switcher_open_count() > first,
        "re-open bumped the open generation"
    );
}

// ── HBR-QUIET: the jump does NOT block on the recents `POST` (it runs off the UI thread) ─────────────
//
// This is the MT-017 redo regression guard. The earlier code called `record_recent` SYNCHRONOUSLY with
// `block_on` on the egui frame thread, freezing the UI for the full network round-trip. A `BlockingPost`
// transport gates `record_recent` on a channel the test does NOT release until AFTER it has asserted the
// jump landed (the tab opened + the switcher closed). Because the frame returns while the POST is still
// parked, the proof is: (1) the switcher closed and the target tab opened, and (2) the POST was observed
// dispatched (`post_started`) but had NOT yet returned (`post_finished == false`) at that point — i.e.
// the frame did not await the POST. We then release the POST and confirm it completes + reconciles.

struct BlockingPost {
    hits: Vec<LoomGraphSearchHit>,
    /// Set true the instant `record_recent` is entered (proves the POST was dispatched off-thread).
    post_started: Arc<AtomicBool>,
    /// Set true only after `record_recent` is allowed to return (proves the frame did not await it).
    post_finished: Arc<AtomicBool>,
    /// `record_recent` blocks on this until the test releases it, simulating a slow network round-trip.
    release: Mutex<Receiver<()>>,
    /// The durable recents the backend would return on a subsequent `GET recents` — a recorded POST
    /// prepends its key here, so `list_recents` reflects committed state (realistic backend behavior).
    recorded: Arc<Mutex<Vec<String>>>,
}

impl LoomGraphSearchTransport for BlockingPost {
    fn search(&self, _ws: &str, _q: &str) -> Result<Vec<LoomGraphSearchHit>, SearchTransportError> {
        Ok(self.hits.clone())
    }
    fn list_recents(&self, _ws: &str) -> Result<Vec<String>, SearchTransportError> {
        Ok(self.recorded.lock().unwrap().clone())
    }
    fn record_recent(
        &self,
        _ws: &str,
        hit: &LoomGraphSearchHit,
    ) -> Result<String, SearchTransportError> {
        self.post_started.store(true, Ordering::SeqCst);
        // Park here until the test releases the round-trip. If the egui frame thread were awaiting this
        // (the old block_on bug), the harness `step()` that triggers the jump would deadlock and the
        // test would hang instead of asserting — so completing the jump assertions IS the proof.
        let _ = self.release.lock().unwrap().recv();
        let key = format!("{}:{}", hit.source_kind, hit.ref_id);
        // Commit to the durable store so a later `GET recents` returns it (backend-realistic).
        {
            let mut store = self.recorded.lock().unwrap();
            store.retain(|k| k != &key);
            store.insert(0, key.clone());
        }
        self.post_finished.store(true, Ordering::SeqCst);
        Ok(key)
    }
}

#[test]
fn jump_does_not_block_on_recents_post() {
    let (tx, rx): (Sender<()>, Receiver<()>) = std::sync::mpsc::channel();
    let post_started = Arc::new(AtomicBool::new(false));
    let post_finished = Arc::new(AtomicBool::new(false));
    let stub = BlockingPost {
        hits: vec![make_hit(
            "work_packet",
            "WP-KERNEL-011",
            "WorkSurface Shell",
            json!({ "work_packet_id": "WP-KERNEL-011" }),
        )],
        post_started: post_started.clone(),
        post_finished: post_finished.clone(),
        release: Mutex::new(rx),
        recorded: Arc::new(Mutex::new(vec![])),
    };

    // Build the harness with a real multi-thread runtime so the spawned POST actually runs (and parks).
    let rt = Box::leak(Box::new(
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("runtime"),
    ));
    let handle = rt.handle().clone();
    let transport: Arc<dyn LoomGraphSearchTransport> = Arc::new(stub);
    let mut harness = Harness::builder().build_state(
        move |ctx, a: &mut HandshakeApp| a.ui(ctx),
        {
            let mut app = ok_app();
            app.set_runtime_handle(handle.clone());
            app.set_quick_switcher_transport(transport.clone());
            app
        },
    );

    harness.run();
    harness.state_mut().open_quick_switcher();
    harness.run();
    harness.run();

    switcher_search(&harness).type_text("shell");
    let rendered = step_until(&mut harness, |app| !app.quick_switcher_search_results().is_empty());
    assert!(rendered, "graph-search delivered the work-packet hit");
    harness.step();
    harness.step();

    // Press Enter — this dispatches the POST off-thread and performs the jump on THIS frame.
    harness.key_press(egui::Key::Enter);
    // If the POST were synchronous (block_on on the frame thread), this step() would deadlock on the
    // parked channel. It returns => the frame did not await the POST.
    harness.step();
    harness.step();

    // The jump landed: the switcher closed and the work-packet tab opened on the active pane.
    assert!(
        !harness.state().quick_switcher_open(),
        "Enter on a hit closed the switcher even though the recents POST is still parked"
    );
    let active = harness
        .state()
        .active_pane()
        .map(|p| p.to_string())
        .expect("active pane set after the jump");
    let bar = harness
        .state()
        .tab_bar_states()
        .get(active.as_str())
        .expect("active pane tab bar");
    let opened = bar.active().expect("an active tab opened by the jump");
    assert_eq!(opened.pane_type, PaneType::KernelDcc, "work-packet hit opened a Kernel DCC tab");
    assert_eq!(opened.content_id.as_deref(), Some("WP:WP-KERNEL-011"));

    // The optimistic local recents prepend is immediate (no wait for the POST).
    assert_eq!(
        harness.state().quick_switcher_recents().first().map(|s| s.as_str()),
        Some("work_packet:WP-KERNEL-011"),
        "the jump optimistically prepended the recent without awaiting the POST"
    );

    // The proof of non-blocking: the POST was dispatched but had NOT returned while the jump completed.
    assert!(
        post_started.load(Ordering::SeqCst),
        "the recents POST was dispatched off-thread"
    );
    assert!(
        !post_finished.load(Ordering::SeqCst),
        "the recents POST had NOT returned when the jump completed — the frame did not await it"
    );

    // Release the parked POST; it now completes + the next open drains the confirmed key (no crash).
    tx.send(()).expect("release the parked POST");
    let finished = {
        let deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < deadline && !post_finished.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(10));
        }
        post_finished.load(Ordering::SeqCst)
    };
    assert!(finished, "the released POST completed off-thread");

    // Re-open: the fresh `GET recents` load (now reflecting the committed POST) delivers the confirmed
    // key to the front. step_until tolerates the async delivery latency.
    harness.state_mut().open_quick_switcher();
    let reconciled = step_until(&mut harness, |app| {
        app.quick_switcher_recents().first().map(|s| s.as_str()) == Some("work_packet:WP-KERNEL-011")
    });
    assert!(
        reconciled,
        "the backend-confirmed recents key reconciled to the front after the POST completed; got {:?}",
        harness.state().quick_switcher_recents()
    );
    assert!(
        harness.state().quick_switcher_recents_error().is_none(),
        "a successful POST left no recents error"
    );
}

// =============================================================================
// Live PostgreSQL integration test (MT-017 PT2) — mirrors the MT-009 layout
// live test. Gated behind `integration_tests`; needs handshake_core + managed
// PostgreSQL on 127.0.0.1:37501 and an existing workspace id.
// =============================================================================
//
// Run with:
//   cargo test --features integration_tests --test test_quick_switcher live_backend_ -- --ignored --nocapture
//
// HSK_LIVE_WORKSPACE_ID must name a workspace with at least one searchable graph entity matching
// HSK_LIVE_QUICK_SWITCHER_QUERY (default "a"). This exercises the REAL LoomGraphSearchClient against
// the real graph-search + quick-switcher/recents endpoints over PostgreSQL.
#[cfg(feature = "integration_tests")]
#[test]
#[ignore = "needs managed-postgres + handshake_core on 127.0.0.1:37501 and HSK_LIVE_WORKSPACE_ID"]
fn live_backend_quick_switcher_searches_real_graph() {
    use handshake_native::quick_switcher::{
        ordered_results, LoomGraphSearchClient, LoomGraphSearchTransport,
    };

    let workspace_id = std::env::var("HSK_LIVE_WORKSPACE_ID")
        .expect("set HSK_LIVE_WORKSPACE_ID to an existing workspace id");
    let query = std::env::var("HSK_LIVE_QUICK_SWITCHER_QUERY").unwrap_or_else(|_| "a".to_owned());

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("runtime");
    let client = LoomGraphSearchClient::production(rt.handle().clone());

    // GET graph-search against the real backend.
    let hits = client
        .search(&workspace_id, &query)
        .expect("graph-search against live backend");
    println!("live graph-search returned {} hit(s)", hits.len());

    // GET recents against the real backend.
    let recents = client
        .list_recents(&workspace_id)
        .expect("recents GET against live backend");
    println!("live recents: {recents:?}");

    // ordered_results applies recents-first ordering over the live hits without panicking.
    let ordered = ordered_results(&hits, &recents);
    assert_eq!(ordered.len(), hits.len(), "ordering preserves the hit set");

    // If there is at least one hit, POST a recent for it and confirm the returned key round-trips.
    if let Some(first) = hits.first() {
        let key = client
            .record_recent(&workspace_id, first)
            .expect("recents POST against live backend");
        assert_eq!(
            key,
            format!("{}:{}", first.source_kind, first.ref_id),
            "POST recents returns the expected hit_key"
        );
    }
}
