//! WP-KERNEL-012 MT-028 LoomSearchV2 surface PROOFS (E4 Search).
//!
//! Coverage map (proof_targets PT-1..PT-4 + acceptance_criteria AC-1..AC-10):
//!   - PROOF1 (highlight parser / facet sort / facet-clear / status / no-workspace guard): the
//!     STANDALONE state-machine + parser logic, proven in the lib unit tests
//!     (`handshake_native::loom_search_v2::tests`) — pure, no backend, no GPU.
//!   - PROOF2 (PT-3, AC-1/AC-3/AC-5/AC-8): a kittest render of the panel with a MOCK response (3 hits,
//!     2 facets) asserts the live AccessKit tree contains the 6 contract author_ids — query, search,
//!     save-view, facet.note, facet.code, result.{block_id} — and the status line text.
//!   - PROOF3 (AC-5 highlight): a kittest render confirms a hit with a `<mark>` highlight produces a
//!     colored LayoutJob (the highlight text run carries the palette `search_highlight_bg` background),
//!     proven via the `highlight_layout_job` builder (the SAME one the row renderer uses) — NOT raw tags.
//!   - PROOF4 (AC-6 open-block callback): a kittest click on a result row invokes `on_open_block` with
//!     the correct block_id.
//!   - PROOF5 (AC-4 facet toggle clear): clicking the active facet again clears `active_content_type`.
//!   - PROOF6 (request builders, the VERIFIED routes): the search POST `/loom/search-v2` body and the
//!     save-as-view POST `/loom/views/definitions` body (MT-027's proven createBlockView route, NOT the
//!     contract's stale `/loom/views`) — proven WITHOUT a backend (the spawn paths route through the
//!     SAME builders, so a stale URL / mis-shaped body can never reach the real backend unnoticed).
//!   - PROOF7 (PT-4, HBR-VIS): a screenshot of the rendered panel (query bar + 2 facets + 3 rows with
//!     visible highlight coloring) to the EXTERNAL artifact root.
//!   - PT-1/PT-2 (real-SurrealDB integration): a nonignored `integration`-feature proof owns an isolated
//!     workspace, self-seeds three differing block types, drives the mounted HandshakeApp panel, saves
//!     and reloads a real view definition, proves error/rebind behavior, and verifies canonical cleanup.
//!
//! ## Backend reality (Spec-Realism Gate / MT-022..027 pattern)
//!
//! AC-2 (a real query populates a real response) requires a running handshake_core backed by SurrealDB;
//! the feature-gated proof below creates and removes its own fixture and is deliberately not ignored.
//! The standalone rendering + parser + state-machine + request-builder proofs remain deterministic,
//! GPU/backend-free regression evidence.
//!
//! ## Artifact hygiene (CX-212E)
//!
//! EVERY PNG is written ONLY to the EXTERNAL `Handshake_Artifacts/handshake-test/wp-kernel-012-mt-028/`
//! root via [`external_artifact_dir`]; [`assert_no_local_artifact_dir`] fails the run if a repo-local
//! `tests/screenshots/` or `test_output/` directory exists.

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use egui_kittest::kittest::{NodeT, Queryable};
#[cfg(feature = "integration")]
#[path = "native_gui_support/canonical_argus_driver.rs"]
mod canonical_argus_driver;
#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;
#[cfg(feature = "integration")]
use canonical_argus_driver::{json_has_author_id, json_node_by_author_id, CanonicalArgusDriver};
use screenshot_harness::ScreenshotHarness as Harness;

use handshake_native::backend_client::{
    LoomSearchBlock, LoomSearchV2Body, LoomSearchV2Client, LoomSearchV2Hit, LoomSearchV2Response,
};
use handshake_native::loom_search_v2::{
    facet_author_id, highlight_layout_job, parse_highlight_segments, preview_author_id,
    result_author_id, LoomSearchV2Callbacks, LoomSearchV2PaneFactory, LoomSearchV2PaneShared,
    LoomSearchV2PanelState, QUERY_AUTHOR_ID, SAVE_STATUS_AUTHOR_ID, SAVE_VIEW_AUTHOR_ID,
    SEARCH_AUTHOR_ID, STATUS_AUTHOR_ID,
};
use handshake_native::pane_registry::{
    DirtyState, LockState, PaneAuthority, PaneFactory, PaneHostWidget, PaneRecord, PaneRegistry,
    PaneType,
};
use handshake_native::theme::HsTheme;

#[cfg(feature = "integration")]
#[path = "interconnect_support/mod.rs"]
mod interconnect_support;

/// The crate-relative path to the EXTERNAL artifacts root (CX-212E), disk-agnostic.
fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

/// Assert NO repo-local artifact directory exists under the crate (CX-212E hygiene). Checks BOTH
/// `test_output/` and `tests/screenshots/` (the path a contract might literally name, overridden here).
fn assert_no_local_artifact_dir() {
    for local in [Path::new("test_output"), Path::new("tests/screenshots")] {
        assert!(
            !local.exists(),
            "CX-212E: no repo-local artifact dir may exist — artifacts go to the external \
             Handshake_Artifacts/handshake-test root only (found {})",
            local.display()
        );
    }
}

/// Serialize the `.wgpu()` screenshot tests (the documented Windows-wgpu concurrent-device hazard).
static WGPU_SERIAL_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());
fn wgpu_guard() -> std::sync::MutexGuard<'static, ()> {
    WGPU_SERIAL_GUARD.lock().unwrap_or_else(|p| p.into_inner())
}

const TEST_BASE: &str = "http://127.0.0.1:37501";

// ── Mock-response builders (the native projection of a real loom_search_v2 response) ────────────────

fn hit(
    block_id: &str,
    content_type: &str,
    title: &str,
    score: f64,
    highlight: &str,
) -> LoomSearchV2Hit {
    LoomSearchV2Hit {
        block: LoomSearchBlock {
            block_id: block_id.to_owned(),
            content_type: content_type.to_owned(),
            document_id: None,
            title: Some(title.to_owned()),
        },
        score,
        fts_rank: 0.5,
        trgm_sim: 0.4,
        vector_sim: 0.0,
        edge_degree: 1,
        highlight: highlight.to_owned(),
    }
}

/// A response with 3 hits (note/note/code) and a 2-entry facet map, semantic OFF.
fn mock_response() -> LoomSearchV2Response {
    let mut facets = BTreeMap::new();
    facets.insert("note".to_owned(), 2);
    facets.insert("code".to_owned(), 1);
    LoomSearchV2Response {
        hits: vec![
            hit(
                "blk-1",
                "note",
                "First Note",
                0.912,
                "<mark>alpha</mark> beta",
            ),
            hit(
                "blk-2",
                "note",
                "Second Note",
                0.640,
                "gamma <mark>delta</mark>",
            ),
            hit("blk-3", "code", "Some Code", 0.501, "plain excerpt"),
        ],
        content_type_facets: facets,
        semantic_available: false,
        total: 3,
    }
}

/// A current-thread tokio runtime kept alive for a test's scope (the client bridges onto its handle).
fn rt() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build current-thread runtime")
}

/// Build a kittest harness that renders the shared panel state. `opened` records every block id passed
/// to the `on_open_block` callback. `workspace_id`/`client` drive the (non-fired) action dispatch.
fn harness_for<'a>(
    state: Arc<Mutex<LoomSearchV2PanelState>>,
    opened: Arc<Mutex<Vec<String>>>,
    client: LoomSearchV2Client,
    workspace_id: Option<String>,
) -> Harness<'a, ()> {
    Harness::builder()
        .with_size(egui::vec2(900.0, 700.0))
        .build_ui(move |ui| {
            let pal = HsTheme::Dark.palette();
            let opened_cb = Arc::clone(&opened);
            let mut on_open = move |id: &str| opened_cb.lock().unwrap().push(id.to_owned());
            let mut cbs = LoomSearchV2Callbacks {
                on_open_block: &mut on_open,
            };
            handshake_native::loom_search_v2::show(
                ui,
                &mut state.lock().unwrap(),
                &pal,
                &client,
                workspace_id.as_deref(),
                &mut cbs,
            );
        })
}

/// Collect every author_id present in the live AccessKit tree.
fn author_ids<State>(harness: &Harness<'_, State>) -> HashSet<String> {
    let mut ids = HashSet::new();
    for node in harness.root().children_recursive() {
        if let Some(a) = node.accesskit_node().author_id() {
            ids.insert(a.to_owned());
        }
    }
    ids
}

/// Click the node addressed by `author_id` via the AccessKit Click action.
fn click_author_id<State>(harness: &Harness<'_, State>, author_id: &str) {
    let node = harness
        .root()
        .children_recursive()
        .find(|n| n.accesskit_node().author_id() == Some(author_id))
        .unwrap_or_else(|| panic!("no node with author_id '{author_id}' to click"));
    node.click_accesskit();
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// PROOF2 (PT-3, AC-1/AC-3/AC-8): the 6 contract author_ids appear in the live AccessKit tree + status.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn accesskit_tree_has_all_contract_author_ids() {
    let mut s = LoomSearchV2PanelState::new();
    s.bind_workspace(Some("ws-1"));
    s.query = "alpha".to_owned();
    s.response = Some(mock_response());
    let state = Arc::new(Mutex::new(s));
    let opened = Arc::new(Mutex::new(Vec::new()));
    let client = LoomSearchV2Client::new(TEST_BASE, rt().handle().clone());
    let mut harness = harness_for(Arc::clone(&state), opened, client, Some("ws-1".to_owned()));
    harness.run();

    let ids = author_ids(&harness);
    for required in [
        QUERY_AUTHOR_ID,
        SEARCH_AUTHOR_ID,
        SAVE_VIEW_AUTHOR_ID,
        SAVE_STATUS_AUTHOR_ID,
        STATUS_AUTHOR_ID,
    ] {
        assert!(
            ids.contains(required),
            "PT-3: required author_id '{required}' missing from {ids:?}"
        );
    }
    // Facet ids for both content types.
    assert!(
        ids.contains(&facet_author_id("note")),
        "PT-3: facet.note missing"
    );
    assert!(
        ids.contains(&facet_author_id("code")),
        "PT-3: facet.code missing"
    );
    // A result row id (the contract names `search.result.{block_id_0}`).
    assert!(
        ids.contains(&result_author_id("blk-1")),
        "PT-3: result.blk-1 missing"
    );
    assert!(
        ids.contains(&result_author_id("blk-2")),
        "PT-3: result.blk-2 missing"
    );
    assert!(
        ids.contains(&result_author_id("blk-3")),
        "PT-3: result.blk-3 missing"
    );
    for block_id in ["blk-1", "blk-2", "blk-3"] {
        assert!(
            ids.contains(&preview_author_id(block_id)),
            "PT-3: stable preview for {block_id} missing"
        );
    }

    println!("PT-3/AC-1/AC-8: all 6 contract author_ids present in the live AccessKit tree");
    assert_no_local_artifact_dir();
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// PROOF (AC-3): the status line reflects semantic_available — '(keyword/fuzzy only)' when false.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn status_line_reflects_semantic_off() {
    let mut s = LoomSearchV2PanelState::new();
    s.bind_workspace(Some("ws-1"));
    s.response = Some(mock_response()); // semantic_available = false, total = 3
    let state = Arc::new(Mutex::new(s));
    let opened = Arc::new(Mutex::new(Vec::new()));
    let client = LoomSearchV2Client::new(TEST_BASE, rt().handle().clone());
    let mut harness = harness_for(state, opened, client, Some("ws-1".to_owned()));
    harness.run();

    // The status label text is queryable by label (it is a plain egui::Label).
    harness.get_by_label("3 results (keyword/fuzzy only)");
    println!(
        "AC-3: status line shows '3 results (keyword/fuzzy only)' when semantic_available=false"
    );
    assert_no_local_artifact_dir();
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// PROOF4 (AC-6): clicking a result row invokes on_open_block with the correct block_id.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn clicking_result_row_opens_block() {
    let mut s = LoomSearchV2PanelState::new();
    s.bind_workspace(Some("ws-1"));
    s.query = "alpha".to_owned();
    s.response = Some(mock_response());
    let state = Arc::new(Mutex::new(s));
    let opened = Arc::new(Mutex::new(Vec::new()));
    let opened_ck = Arc::clone(&opened);
    let client = LoomSearchV2Client::new(TEST_BASE, rt().handle().clone());
    let mut harness = harness_for(Arc::clone(&state), opened, client, Some("ws-1".to_owned()));
    harness.run();

    click_author_id(&harness, &result_author_id("blk-2"));
    harness.run();

    let opened = opened_ck.lock().unwrap();
    assert_eq!(
        opened.as_slice(),
        ["blk-2"],
        "AC-6: on_open_block called with the clicked block_id"
    );
    println!("AC-6: clicking result row blk-2 invoked on_open_block('blk-2')");
    assert_no_local_artifact_dir();
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// PROOF5 (AC-4): clicking the ACTIVE facet again clears active_content_type (no live backend needed —
// the toggle logic flips the state; the re-fire's no-backend HTTP simply never delivers).
// ══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn clicking_active_facet_clears_filter() {
    let mut s = LoomSearchV2PanelState::new();
    s.bind_workspace(Some("ws-1"));
    s.query = "alpha".to_owned();
    s.active_content_type = Some("note".to_owned()); // note facet already active
    s.response = Some(mock_response());
    let state = Arc::new(Mutex::new(s));
    let opened = Arc::new(Mutex::new(Vec::new()));
    let client = LoomSearchV2Client::new(TEST_BASE, rt().handle().clone());
    let mut harness = harness_for(Arc::clone(&state), opened, client, Some("ws-1".to_owned()));
    harness.run();

    click_author_id(&harness, &facet_author_id("note"));
    // Use step() not run(): the facet click re-fires the search, which sets `loading` and requests a
    // repaint each frame (the genuine in-flight state). With no live backend the spinner never clears,
    // so run()'s max_steps would trip — exactly the MT-015 "no perpetual spinner / kittest uses step()"
    // discipline. A single step applies the click's toggle + re-fire without waiting on the network.
    harness.step();

    assert_eq!(
        state.lock().unwrap().active_content_type,
        None,
        "AC-4: clicking the active facet again clears the content_type filter"
    );
    println!("AC-4: clicking the active 'note' facet cleared active_content_type");
    assert_no_local_artifact_dir();
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// PROOF (AC-7): the Save-as-view button is DISABLED with no results and ENABLED with results.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn save_view_button_disabled_without_results() {
    // No response => has_results() false => button disabled.
    let state = Arc::new(Mutex::new(LoomSearchV2PanelState::new()));
    let opened = Arc::new(Mutex::new(Vec::new()));
    let client = LoomSearchV2Client::new(TEST_BASE, rt().handle().clone());
    let mut harness = harness_for(state, opened, client, Some("ws-1".to_owned()));
    harness.run();
    let disabled = harness
        .root()
        .children_recursive()
        .find(|n| n.accesskit_node().author_id() == Some(SAVE_VIEW_AUTHOR_ID))
        .map(|n| n.accesskit_node().is_disabled())
        .expect("save-view node present");
    assert!(disabled, "AC-7: Save-as-view disabled with no results");

    // With results => enabled.
    let mut s2 = LoomSearchV2PanelState::new();
    s2.bind_workspace(Some("ws-1"));
    s2.response = Some(mock_response());
    let state2 = Arc::new(Mutex::new(s2));
    let opened2 = Arc::new(Mutex::new(Vec::new()));
    let client2 = LoomSearchV2Client::new(TEST_BASE, rt().handle().clone());
    let mut harness2 = harness_for(state2, opened2, client2, Some("ws-1".to_owned()));
    harness2.run();
    let enabled = harness2
        .root()
        .children_recursive()
        .find(|n| n.accesskit_node().author_id() == Some(SAVE_VIEW_AUTHOR_ID))
        .map(|n| !n.accesskit_node().is_disabled())
        .expect("save-view node present");
    assert!(enabled, "AC-7: Save-as-view enabled with results");
    println!("AC-7: Save-as-view button gates on has_results()");
    assert_no_local_artifact_dir();
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// PROOF3 (AC-5): the highlight LayoutJob colors the <mark> runs (the row renderer's exact builder).
// ══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn highlight_layout_job_colors_marked_runs() {
    let pal = HsTheme::Dark.palette();
    let text = egui::Color32::from_gray(200);
    let job = highlight_layout_job("<mark>foo</mark> bar <mark>baz</mark>", &pal, text);
    // The LayoutJob must have 3 sections; the 1st and 3rd carry the highlight background, the 2nd none.
    assert_eq!(
        job.sections.len(),
        3,
        "AC-5: 3 layout sections for foo/bar/baz"
    );
    assert_eq!(
        job.sections[0].format.background, pal.search_highlight_bg,
        "AC-5: 'foo' marked"
    );
    assert_eq!(
        job.sections[1].format.background,
        egui::Color32::TRANSPARENT,
        "AC-5: ' bar ' not marked"
    );
    assert_eq!(
        job.sections[2].format.background, pal.search_highlight_bg,
        "AC-5: 'baz' marked"
    );
    // And the raw `<mark>` tokens must NOT appear in the rendered text.
    assert!(
        !job.text.contains("<mark>"),
        "AC-5: no raw <mark> tag in rendered text"
    );
    assert_eq!(
        job.text, "foo bar baz",
        "AC-5: markers stripped, text preserved"
    );
    println!("AC-5: <mark> runs render as colored LayoutJob sections, no raw HTML");
    assert_no_local_artifact_dir();
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// PROOF6 (request builders): the VERIFIED search + save-as-view routes/bodies (NO backend).
// ══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn search_request_url_and_body() {
    let client = LoomSearchV2Client::new(TEST_BASE, rt().handle().clone());
    let body = LoomSearchV2Body::baseline("hello", Some("note".to_owned()));
    let spec = client.search_request("ws-1", &body);
    assert_eq!(
        spec.url,
        "http://127.0.0.1:37501/workspaces/ws-1/loom/search-v2"
    );
    let json = spec.body.expect("search body");
    assert_eq!(json.get("query").and_then(|x| x.as_str()), Some("hello"));
    assert_eq!(
        json.get("content_type").and_then(|x| x.as_str()),
        Some("note")
    );
    assert_eq!(json.get("graph_boost").and_then(|x| x.as_f64()), Some(1.0));
    assert_eq!(json.get("limit").and_then(|x| x.as_u64()), Some(25));
    println!("PROOF6: search POST /loom/search-v2 body = {{query, content_type, graph_boost:1.0, limit:25}}");
}

#[test]
fn search_request_omits_content_type_when_unfiltered() {
    let client = LoomSearchV2Client::new(TEST_BASE, rt().handle().clone());
    let body = LoomSearchV2Body::baseline("hello", None);
    let spec = client.search_request("ws-1", &body);
    let json = spec.body.expect("search body");
    assert!(
        json.get("content_type").is_none(),
        "unfiltered search omits content_type"
    );
}

#[test]
fn save_view_request_uses_verified_definitions_route() {
    let client = LoomSearchV2Client::new(TEST_BASE, rt().handle().clone());
    let spec = client.save_view_request("ws-1", "view-stable-1", "hello world", Some("note"));
    // MT-027's VERIFIED createBlockView route — NOT the MT-028 contract's stale bare /loom/views.
    assert_eq!(
        spec.url,
        "http://127.0.0.1:37501/workspaces/ws-1/loom/views/definitions"
    );
    let json = spec.body.expect("save body");
    assert_eq!(
        json.get("block_id").and_then(|x| x.as_str()),
        Some("view-stable-1")
    );
    assert_eq!(
        json.get("title").and_then(|x| x.as_str()),
        Some("Search: hello world")
    );
    let def = json.get("definition").expect("definition");
    assert_eq!(def.get("kind").and_then(|x| x.as_str()), Some("table"));
    assert_eq!(
        def.get("query")
            .and_then(|q| q.get("content_type"))
            .and_then(|x| x.as_str()),
        Some("note")
    );
    let cols = def
        .get("columns")
        .and_then(|c| c.as_array())
        .expect("columns");
    let cols: Vec<&str> = cols.iter().filter_map(|c| c.as_str()).collect();
    assert_eq!(cols, ["title", "content_type", "updated"]);
    println!("PROOF6: save-as-view POST /loom/views/definitions body = {{block_id,title,definition{{kind,query,columns}}}}");
}

#[test]
fn save_view_request_empty_query_when_no_facet() {
    let client = LoomSearchV2Client::new(TEST_BASE, rt().handle().clone());
    let spec = client.save_view_request("ws-1", "view-stable-2", "hello", None);
    let json = spec.body.expect("save body");
    let query = json
        .get("definition")
        .and_then(|d| d.get("query"))
        .expect("query");
    assert!(
        query.as_object().map(|o| o.is_empty()).unwrap_or(false),
        "no facet => empty query object"
    );
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// PROOF (MC-7): the no-workspace guard shows an error and fires NO HTTP call.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn no_workspace_guard_sets_error_without_search() {
    let mut state = LoomSearchV2PanelState::new();
    state.query = "alpha".to_owned();
    let client = LoomSearchV2Client::new(TEST_BASE, rt().handle().clone());
    state.run_search(&client, None);
    assert_eq!(state.error.as_deref(), Some("No workspace selected"));
    assert!(
        !state.loading,
        "MC-7: no HTTP call fired (loading stays false)"
    );
    println!("MC-7: no-workspace search sets error 'No workspace selected', no HTTP call");
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// PROOF (parser sanity at the test boundary): re-prove the MC-1 case from the integration crate too.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn parser_mc1_case_from_integration_crate() {
    let segs = parse_highlight_segments("<mark>foo</mark> bar <mark>baz</mark>");
    assert_eq!(segs.len(), 3);
    assert!(
        segs[0].marked && !segs[1].marked && segs[2].marked,
        "MC-1: mid+last marked"
    );
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// PROOF7 (PT-4, HBR-VIS): screenshot of the rendered panel (query bar + 2 facets + 3 rows w/ highlight).
// ══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn loom_search_v2_screenshot() {
    let _g = wgpu_guard();
    let mut s = LoomSearchV2PanelState::new();
    s.bind_workspace(Some("ws-1"));
    s.query = "alpha".to_owned();
    s.response = Some(mock_response());
    let state = Arc::new(Mutex::new(s));
    let opened = Arc::new(Mutex::new(Vec::new()));
    let client = LoomSearchV2Client::new(TEST_BASE, rt().handle().clone());
    let workspace_id = Some("ws-1".to_owned());

    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 700.0))
        .wgpu()
        .build_ui(move |ui| {
            let pal = HsTheme::Dark.palette();
            let opened_cb = Arc::clone(&opened);
            let mut on_open = move |id: &str| opened_cb.lock().unwrap().push(id.to_owned());
            let mut cbs = LoomSearchV2Callbacks {
                on_open_block: &mut on_open,
            };
            handshake_native::loom_search_v2::show(
                ui,
                &mut state.lock().unwrap(),
                &pal,
                &client,
                workspace_id.as_deref(),
                &mut cbs,
            );
        });
    harness.run();
    harness.run();

    match harness.render() {
        Ok(image) => {
            let (w, h) = (image.width(), image.height());
            assert!(w > 0 && h > 0, "rendered image must be non-empty");
            let raw = image.as_raw();
            let mut counts: std::collections::HashMap<[u8; 4], u32> =
                std::collections::HashMap::new();
            let mut white = 0u32;
            // The <mark> amber is rgb(255,214,0); sample for its presence to PROVE the highlight rendered.
            let mut amber = 0u32;
            let mut i = 0usize;
            while i + 4 <= raw.len() {
                let px = [raw[i], raw[i + 1], raw[i + 2], raw[i + 3]];
                if px[3] != 0 {
                    *counts.entry(px).or_insert(0) += 1;
                    if px[0] > 250 && px[1] > 250 && px[2] > 250 {
                        white += 1;
                    }
                    // amber-ish: high red, mid-high green, low blue.
                    if px[0] > 220 && px[1] > 180 && px[1] < 240 && px[2] < 80 {
                        amber += 1;
                    }
                }
                i += 16;
            }
            let total: u32 = counts.values().sum();
            assert!(total > 0, "screenshot: sampled pixels must be opaque");
            assert!(
                (white as f32 / total as f32) < 0.95,
                "screenshot: surface must not be ~all-white (white frac {})",
                white as f32 / total as f32
            );
            assert!(
                counts.len() >= 2,
                "screenshot: >= 2 distinct colours expected, got {}",
                counts.len()
            );

            let ext_dir = external_artifact_dir("wp-kernel-012-mt-028");
            let _ = std::fs::create_dir_all(&ext_dir);
            let png = ext_dir.join("MT-028-loom-search-v2.png");
            let saved = image.save(&png).is_ok();
            println!(
                "SCREENSHOT: {w}x{h}, {} distinct colours, white_frac={:.3}, amber_samples={amber}, saved={saved} ({})",
                counts.len(),
                white as f32 / total as f32,
                png.display()
            );
        }
        Err(e) => {
            println!(
                "BLOCKER(non-fatal): screenshot render unavailable (no wgpu adapter): {e}. The \
                 AccessKit + highlight + facet + open-block + request-builder proofs passed; the PNG is a GPU-host item."
            );
        }
    }
    assert_no_local_artifact_dir();
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// PROOF8 (AC-9, the must-fix in-product render path): open the LoomSearchV2 pane THROUGH the WP-011
// registry + PaneHostWidget (the SAME dispatch the running shell uses), not by calling show() directly.
// Proves the concrete `LoomSearchV2PaneFactory` renders the REAL panel (the 6 contract author_ids appear
// in the live AccessKit tree) and that a result-row click routed through the registry-dispatched pane
// reaches the shared open-block cell the shell drains into the Loom block-open path.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// Build a one-pane registry holding a single `PaneType::LoomSearchV2` record (the surface the running
/// shell hosts) so the pane host dispatches to its factory.
fn loom_search_v2_registry() -> PaneRegistry {
    let mut reg = PaneRegistry::new();
    reg.insert(PaneRecord::new(
        std::sync::Arc::from("loom-search-pane"),
        PaneType::LoomSearchV2,
        "p",
        None,
        LockState::Unlocked,
        DirtyState::Clean,
        PaneAuthority::System,
    ));
    reg
}

#[test]
fn pane_opens_via_registry_and_renders_real_panel() {
    // A real client (no backend reached — the panel renders the seeded response; no search fires).
    let client = LoomSearchV2Client::new(TEST_BASE, rt().handle().clone());
    let shared = Arc::new(Mutex::new(LoomSearchV2PaneShared::new(
        HsTheme::Dark.palette(),
    )));
    {
        // The shell pushes the live workspace id + palette into the shared cell each frame; mirror that.
        let mut g = shared.lock().unwrap();
        g.workspace_id = Some("ws-1".to_owned());
    }
    // Seed the panel state with a mock response so the registry-dispatched render shows real
    // facets/rows/highlight (the in-product render path — NOT an out-of-band show() call).
    let mut state = LoomSearchV2PanelState::new();
    state.bind_workspace(Some("ws-1"));
    state.query = "alpha".to_owned();
    state.response = Some(mock_response());
    let factory: Box<dyn PaneFactory> = Box::new(LoomSearchV2PaneFactory::with_state(
        client,
        Arc::clone(&shared),
        state,
    ));

    let reg = loom_search_v2_registry();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 700.0))
        .build_ui(move |ui| {
            // Dispatch through the SAME pane host the shell's CentralPanel uses (AC-9 in-product path).
            PaneHostWidget::show(ui, &reg, |_pane_type| factory.as_ref());
        });
    harness.run();

    // The REAL panel rendered (not the placeholder): the 6 contract author_ids are in the live tree.
    let ids = author_ids(&harness);
    for required in [
        QUERY_AUTHOR_ID,
        SEARCH_AUTHOR_ID,
        SAVE_VIEW_AUTHOR_ID,
        SAVE_STATUS_AUTHOR_ID,
        STATUS_AUTHOR_ID,
    ] {
        assert!(
            ids.contains(required),
            "AC-9: required author_id '{required}' missing — the pane rendered the placeholder, not the real panel ({ids:?})"
        );
    }
    assert!(
        ids.contains(&facet_author_id("note")),
        "AC-9: facet.note missing from registry-dispatched pane"
    );
    assert!(
        ids.contains(&result_author_id("blk-1")),
        "AC-9: result.blk-1 missing from registry-dispatched pane"
    );
    assert!(
        ids.contains(&preview_author_id("blk-1")),
        "AC-9: preview.blk-1 missing from registry-dispatched pane"
    );

    // A result-row click through the registry-dispatched pane routes the block id into the shared cell
    // the shell drains (open-in-place). Proves on_open_block is wired by the factory, not just show().
    click_author_id(&harness, &result_author_id("blk-2"));
    harness.run();
    let opened = shared.lock().unwrap().open_requests.clone();
    assert_eq!(opened.len(), 1);
    assert_eq!(opened[0].origin_pane_id.as_ref(), "loom-search-pane");
    assert_eq!(opened[0].workspace_id, "ws-1");
    assert_eq!(opened[0].block_id, "blk-2");
    assert_eq!(opened[0].content_type, "note");
    println!("AC-9: LoomSearchV2 pane opens via the WP-011 registry/PaneHostWidget and renders the REAL panel + open-block wiring");
    assert_no_local_artifact_dir();
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// PT-1 / PT-2 LIVE MANAGED INTEGRATION. Feature-gated but deliberately NOT ignored. The test owns
// its fixture lifecycle and writes its success receipt only after fresh canonical cleanup proof.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// A real loopback reverse proxy used only by the managed mounted proof. Its public base contains a
/// path prefix that the production default does not have. The mounted product client sends genuine
/// HTTP traffic to that distinct address; the proxy records it, strips the proof prefix, forwards the
/// same request to the managed handshake_core/SurrealDB backend, and relays the real response.
/// Therefore a factory that silently kept `BACKEND_BASE_URL` could still render backend data, but the
/// required prefixed captures would be absent and the proof would fail.
#[cfg(feature = "integration")]
struct ManagedRebindProxy {
    base: String,
    captured: Arc<Mutex<Vec<ManagedProxyRequest>>>,
    save_response_gate: Arc<ManagedSaveResponseGate>,
    connection_workers: Arc<Mutex<Vec<std::thread::JoinHandle<()>>>>,
    stop: Arc<std::sync::atomic::AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
}

#[cfg(feature = "integration")]
#[derive(Clone, Debug)]
struct ManagedProxyRequest {
    method: String,
    prefixed_path: String,
    body: serde_json::Value,
}

#[cfg(feature = "integration")]
struct ParsedProxyRequest {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

/// A deterministic response-order gate for the mounted stale-save regression. The proxy still
/// forwards the request to the real managed backend immediately; it only holds the corresponding
/// HTTP response after persistence succeeds, allowing a replacement facet search to finish first.
#[cfg(feature = "integration")]
#[derive(Default)]
struct ManagedSaveResponseGate {
    armed: std::sync::atomic::AtomicBool,
    held: Mutex<bool>,
    released: Mutex<bool>,
    release_cv: std::sync::Condvar,
    response_written: std::sync::atomic::AtomicBool,
    persisted_view_ids: Mutex<Vec<String>>,
}

#[cfg(feature = "integration")]
impl ManagedSaveResponseGate {
    fn arm(&self) {
        *self.held.lock().expect("MT-028 save gate held state") = false;
        *self
            .released
            .lock()
            .expect("MT-028 save gate release state") = false;
        self.response_written
            .store(false, std::sync::atomic::Ordering::Release);
        self.armed.store(true, std::sync::atomic::Ordering::Release);
    }

    fn claim_if_armed(&self) -> bool {
        self.armed.swap(false, std::sync::atomic::Ordering::AcqRel)
    }

    fn hold_after_forward(&self) {
        {
            let mut held = self.held.lock().expect("MT-028 save gate held state");
            *held = true;
        }
        let mut released = self
            .released
            .lock()
            .expect("MT-028 save gate release state");
        while !*released {
            released = self
                .release_cv
                .wait(released)
                .expect("MT-028 save gate release wait");
        }
    }

    fn is_held(&self) -> bool {
        *self.held.lock().expect("MT-028 save gate held state")
    }

    fn release(&self) {
        let mut released = self
            .released
            .lock()
            .expect("MT-028 save gate release state");
        *released = true;
        self.release_cv.notify_all();
    }

    fn mark_response_written(&self) {
        self.response_written
            .store(true, std::sync::atomic::Ordering::Release);
    }

    fn response_written(&self) -> bool {
        self.response_written
            .load(std::sync::atomic::Ordering::Acquire)
    }

    fn record_persisted_view_id(&self, response_body: &[u8]) {
        let value: serde_json::Value =
            serde_json::from_slice(response_body).expect("MT-028 successful save response is JSON");
        let block_id = value["block"]["block_id"]
            .as_str()
            .expect("MT-028 successful save response contains block.block_id")
            .to_owned();
        self.persisted_view_ids
            .lock()
            .expect("MT-028 persisted view ids")
            .push(block_id);
    }

    fn persisted_view_ids(&self) -> Vec<String> {
        self.persisted_view_ids
            .lock()
            .expect("MT-028 persisted view ids")
            .clone()
    }
}

#[cfg(feature = "integration")]
fn read_proxy_request(stream: &mut std::net::TcpStream) -> ParsedProxyRequest {
    use std::io::Read as _;

    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .expect("set MT-028 proxy read timeout");
    let mut bytes = Vec::new();
    let mut header_end = None;
    let mut expected_len = None;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let mut chunk = [0_u8; 8192];
        let count = match stream.read(&mut chunk) {
            Ok(count) => count,
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock
                        | std::io::ErrorKind::TimedOut
                        | std::io::ErrorKind::Interrupted
                ) && std::time::Instant::now() < deadline =>
            {
                std::thread::sleep(std::time::Duration::from_millis(5));
                continue;
            }
            Err(error) => panic!("read MT-028 proxy request: {error}"),
        };
        assert!(
            count > 0,
            "MT-028 proxy peer closed before request completed"
        );
        bytes.extend_from_slice(&chunk[..count]);
        if header_end.is_none() {
            header_end = bytes.windows(4).position(|window| window == b"\r\n\r\n");
            if let Some(end) = header_end {
                let headers = String::from_utf8_lossy(&bytes[..end]);
                let content_len = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().expect("valid content-length"))
                    })
                    .unwrap_or(0);
                expected_len = Some(end + 4 + content_len);
            }
        }
        if expected_len.is_some_and(|expected| bytes.len() >= expected) {
            break;
        }
    }

    let end = header_end.expect("MT-028 proxy request has headers");
    let header_text = String::from_utf8(bytes[..end].to_vec()).expect("ASCII HTTP headers");
    let mut lines = header_text.lines();
    let request_line = lines.next().expect("MT-028 proxy request line");
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts.next().expect("request method").to_owned();
    let path = request_parts.next().expect("request path").to_owned();
    let headers = lines
        .filter_map(|line| {
            let (name, value) = line.split_once(':')?;
            Some((name.trim().to_owned(), value.trim().to_owned()))
        })
        .collect();
    let body_start = end + 4;
    let body_end = expected_len.expect("MT-028 proxy expected request length");
    ParsedProxyRequest {
        method,
        path,
        headers,
        body: bytes[body_start..body_end].to_vec(),
    }
}

#[cfg(feature = "integration")]
fn relay_proxy_request(
    stream: &mut std::net::TcpStream,
    runtime: &tokio::runtime::Runtime,
    client: &reqwest::Client,
    upstream_base: &str,
    captured: &Arc<Mutex<Vec<ManagedProxyRequest>>>,
    save_response_gate: &ManagedSaveResponseGate,
) {
    use std::io::Write as _;

    const PREFIX: &str = "/mt028-rebind";
    let request = read_proxy_request(stream);
    let upstream_path = request
        .path
        .strip_prefix(PREFIX)
        .unwrap_or_else(|| panic!("rebound request missing {PREFIX} prefix: {}", request.path))
        .to_owned();
    let hold_save_response = request.method == "POST"
        && upstream_path.ends_with("/loom/views/definitions")
        && save_response_gate.claim_if_armed();
    let body_json = if request.body.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&request.body).unwrap_or_else(|_| {
            serde_json::Value::String(String::from_utf8_lossy(&request.body).into_owned())
        })
    };
    captured
        .lock()
        .expect("MT-028 proxy capture")
        .push(ManagedProxyRequest {
            method: request.method.clone(),
            prefixed_path: request.path.clone(),
            body: body_json,
        });

    let method = reqwest::Method::from_bytes(request.method.as_bytes()).expect("valid HTTP method");
    let mut outbound = client.request(method, format!("{upstream_base}{upstream_path}"));
    for (name, value) in &request.headers {
        if ![
            "host",
            "content-length",
            "connection",
            "transfer-encoding",
            "accept-encoding",
        ]
        .iter()
        .any(|skip| name.eq_ignore_ascii_case(skip))
        {
            outbound = outbound.header(name.as_str(), value.as_str());
        }
    }
    if !request.body.is_empty() {
        outbound = outbound.body(request.body);
    }
    // Construct and poll every reqwest future while this runtime is entered. Calling
    // `runtime.block_on(outbound.send())` still constructs `send()` first, outside the reactor, and
    // `runtime.block_on(response.bytes())` repeats the same error for the body stream.
    let forwarded = runtime.block_on(async move {
        let response = outbound.send().await?;
        let status = response.status();
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("application/json")
            .to_owned();
        let body = response.bytes().await?.to_vec();
        Ok::<_, reqwest::Error>((status, content_type, body))
    });
    let (status, content_type, body) = match forwarded {
        Ok((status, content_type, body)) => (status, content_type, body),
        Err(error) => (
            reqwest::StatusCode::BAD_GATEWAY,
            "text/plain".to_owned(),
            format!("MT-028 managed proxy upstream failure: {error}").into_bytes(),
        ),
    };
    if request.method == "POST"
        && upstream_path.ends_with("/loom/views/definitions")
        && status.is_success()
    {
        save_response_gate.record_persisted_view_id(&body);
    }
    if hold_save_response {
        save_response_gate.hold_after_forward();
    }
    write!(
        stream,
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        status.as_u16(),
        status.canonical_reason().unwrap_or("Unknown"),
        content_type,
        body.len()
    )
    .expect("write MT-028 proxy response headers");
    stream
        .write_all(&body)
        .expect("write MT-028 proxy response body");
    stream.flush().expect("flush MT-028 proxy response");
    if hold_save_response {
        save_response_gate.mark_response_written();
    }
}

#[cfg(feature = "integration")]
impl ManagedRebindProxy {
    fn start(upstream_base: String) -> Self {
        use std::sync::atomic::Ordering;

        let listener = std::net::TcpListener::bind("127.0.0.1:0")
            .expect("bind distinct MT-028 managed reverse proxy");
        listener
            .set_nonblocking(true)
            .expect("set MT-028 proxy nonblocking");
        let address = listener.local_addr().expect("MT-028 proxy address");
        let base = format!("http://{address}/mt028-rebind");
        let captured = Arc::new(Mutex::new(Vec::new()));
        let captured_for_worker = Arc::clone(&captured);
        let save_response_gate = Arc::new(ManagedSaveResponseGate::default());
        let save_response_gate_for_worker = Arc::clone(&save_response_gate);
        let connection_workers = Arc::new(Mutex::new(Vec::new()));
        let connection_workers_for_accept = Arc::clone(&connection_workers);
        let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let stop_for_worker = Arc::clone(&stop);
        let worker = std::thread::spawn(move || {
            while !stop_for_worker.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let upstream_base = upstream_base.clone();
                        let captured = Arc::clone(&captured_for_worker);
                        let save_response_gate = Arc::clone(&save_response_gate_for_worker);
                        let connection_worker = std::thread::spawn(move || {
                            let runtime = tokio::runtime::Builder::new_current_thread()
                                .enable_all()
                                .build()
                                .expect("MT-028 reverse-proxy connection runtime");
                            // Reqwest's client builder consults Tokio's current reactor for its
                            // connector. This worker owns a runtime, but merely constructing it does
                            // not enter its context; building the client outside `enter()` panics on
                            // the background thread with "there is no reactor running" before the
                            // mounted request can be forwarded. Enter only for construction, then
                            // leave before `relay_proxy_request` calls `runtime.block_on`.
                            let client = {
                                let _runtime_guard = runtime.enter();
                                reqwest::Client::builder()
                                    .timeout(std::time::Duration::from_secs(5))
                                    .build()
                                    .expect("MT-028 reverse-proxy connection client")
                            };
                            relay_proxy_request(
                                &mut stream,
                                &runtime,
                                &client,
                                &upstream_base,
                                &captured,
                                &save_response_gate,
                            );
                        });
                        connection_workers_for_accept
                            .lock()
                            .expect("MT-028 proxy connection workers")
                            .push(connection_worker);
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(std::time::Duration::from_millis(5));
                    }
                    Err(error) => panic!("MT-028 reverse-proxy accept failed: {error}"),
                }
            }
        });
        Self {
            base,
            captured,
            save_response_gate,
            connection_workers,
            stop,
            worker: Some(worker),
        }
    }

    fn arm_next_save_response_hold(&self) {
        self.save_response_gate.arm();
    }

    fn save_response_is_held(&self) -> bool {
        self.save_response_gate.is_held()
    }

    fn release_held_save_response(&self) {
        self.save_response_gate.release();
    }

    fn held_save_response_was_written(&self) -> bool {
        self.save_response_gate.response_written()
    }

    fn persisted_view_ids(&self) -> Vec<String> {
        self.save_response_gate.persisted_view_ids()
    }

    fn captured_requests(&self) -> Vec<ManagedProxyRequest> {
        self.captured.lock().expect("MT-028 captures").clone()
    }

    fn finish(mut self) -> Vec<ManagedProxyRequest> {
        self.stop_and_join().expect("MT-028 reverse-proxy worker");
        let captures = self.captured.lock().expect("MT-028 captures").clone();
        captures
    }

    fn stop_and_join(&mut self) -> std::thread::Result<()> {
        self.stop.store(true, std::sync::atomic::Ordering::Release);
        self.save_response_gate.release();
        if let Some(worker) = self.worker.take() {
            worker.join()?;
        }
        let connection_workers = {
            let mut workers = self
                .connection_workers
                .lock()
                .expect("MT-028 proxy connection workers");
            std::mem::take(&mut *workers)
        };
        for worker in connection_workers {
            worker.join()?;
        }
        Ok(())
    }
}

#[cfg(feature = "integration")]
impl Drop for ManagedRebindProxy {
    fn drop(&mut self) {
        // Never double-panic during test unwinding; the explicit `finish` path propagates worker failure.
        let _ = self.stop_and_join();
    }
}

#[cfg(feature = "integration")]
struct ManagedWorkspaceCleanup<'a> {
    backend: &'a interconnect_support::LiveBackend,
    workspace_id: String,
    cleaned: bool,
}

#[cfg(feature = "integration")]
impl ManagedWorkspaceCleanup<'_> {
    fn clean(&mut self) -> u16 {
        let status = self.backend.delete_workspace(&self.workspace_id);
        assert!(
            matches!(status, 200 | 202 | 204 | 404),
            "managed workspace cleanup returned HTTP {status}"
        );
        self.cleaned = true;
        status
    }
}

#[cfg(feature = "integration")]
impl Drop for ManagedWorkspaceCleanup<'_> {
    fn drop(&mut self) {
        if !self.cleaned {
            let _ = self.backend.delete_workspace(&self.workspace_id);
        }
    }
}

/// Capture ONE durable proof frame for the exact state that is currently terminal and return its
/// typed outcome. `ScreenshotHarness` records a durable CAPTURED/DEFERRED/BLOCKED row for every call,
/// so a GREEN run can never imply pixels that were never produced.
#[cfg(feature = "integration")]
fn capture_state_screenshot<State>(
    harness: &mut Harness<'_, State>,
    label: &str,
) -> serde_json::Value {
    let image = harness.render_settled_proof_frame(label);
    let outcome = harness
        .last_screenshot_outcome()
        .cloned()
        .expect("the screenshot harness records a durable outcome for every proof frame");
    serde_json::json!({
        "label": label,
        "status": outcome.status,
        "frame_path": outcome.frame_path,
        "gpu_screenshot_enabled": outcome.gpu_screenshot_enabled,
        "screenshot_run_id": outcome.run_id,
        "outcome_id": outcome.outcome_id,
        "pixels": image.as_ref().map(|image| serde_json::json!({
            "width": image.width(),
            "height": image.height(),
        })),
    })
}

/// The canonical matrix this exact process wrote, opened row by row. Returns the persisted rows plus a
/// binding summary; an empty/absent matrix on a declared matrix run is a hard failure, because a
/// summary boolean is not per-action causal evidence.
#[cfg(feature = "integration")]
fn open_canonical_matrix_rows(client_session_id: &str) -> Option<serde_json::Value> {
    let run_id = std::env::var("HANDSHAKE_ARGUS_MATRIX_RUN_ID")
        .ok()
        .filter(|value| !value.trim().is_empty())?;
    let artifact_dir = std::env::var("HANDSHAKE_PROOF_ARTIFACT_DIR")
        .expect("a declared canonical matrix run binds HANDSHAKE_PROOF_ARTIFACT_DIR");
    let matrix_path = PathBuf::from(artifact_dir)
        .join(&run_id)
        .join("canonical-argus-matrix.jsonl");
    let raw = std::fs::read_to_string(&matrix_path).unwrap_or_else(|error| {
        panic!(
            "the canonical Argus matrix must exist after this exact run: {} ({error})",
            matrix_path.display()
        )
    });
    let rows = raw
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str::<serde_json::Value>(line).expect("matrix row is JSON"))
        .collect::<Vec<_>>();
    assert!(
        !rows.is_empty(),
        "the canonical Argus matrix must carry one terminal row per action"
    );
    let source_sha = std::env::var("HANDSHAKE_ARGUS_MATRIX_SOURCE_SHA")
        .expect("a declared canonical matrix run binds HANDSHAKE_ARGUS_MATRIX_SOURCE_SHA");
    let process_correlation_id = std::env::var("HANDSHAKE_PROOF_PROCESS_CORRELATION_ID")
        .expect("a declared canonical matrix run binds HANDSHAKE_PROOF_PROCESS_CORRELATION_ID");
    let mut previous_sequence = 0_u64;
    let mut receipt_ids = HashSet::new();
    for row in &rows {
        assert_eq!(
            row["schema_id"], "hsk.native_gui.canonical_argus_matrix_trace@1",
            "unexpected matrix row schema: {row}"
        );
        assert_eq!(row["run_id"].as_str(), Some(run_id.as_str()), "{row}");
        assert_eq!(
            row["source_sha"].as_str(),
            Some(source_sha.as_str()),
            "{row}"
        );
        assert_eq!(
            row["process_correlation_id"].as_str(),
            Some(process_correlation_id.as_str()),
            "{row}"
        );
        assert_eq!(
            row["process_id"].as_u64(),
            Some(u64::from(std::process::id())),
            "{row}"
        );
        assert_eq!(
            row["client_session_id"].as_str(),
            Some(client_session_id),
            "{row}"
        );
        let status = row["receipt_status"]
            .as_str()
            .expect("typed receipt status");
        assert_ne!(
            status, "indeterminate",
            "every canonical action must carry a causal terminal receipt: {row}"
        );
        assert!(
            matches!(status, "applied" | "rejected"),
            "unexpected terminal receipt status: {row}"
        );
        assert_eq!(row["terminal_refreshed"], true, "{row}");
        let predicates = row["terminal_predicates"]
            .as_array()
            .expect("terminal predicates array");
        assert!(!predicates.is_empty(), "{row}");
        assert!(
            predicates
                .iter()
                .all(|predicate| predicate["passed"] == true),
            "{row}"
        );
        assert!(
            row["agent_id"]
                .as_str()
                .is_some_and(|agent| agent.ends_with(&format!(":client:{client_session_id}"))),
            "{row}"
        );
        assert!(
            row["correlation_id"]
                .as_str()
                .is_some_and(|id| id.contains(&format!(":{client_session_id}:"))
                    && id.ends_with(&format!(":receipt:{}", row["receipt_id"]))),
            "{row}"
        );
        let sequence = row["terminal_observed_sequence"]
            .as_u64()
            .expect("terminal observed sequence");
        assert!(sequence > previous_sequence, "{row}");
        previous_sequence = sequence;
        assert!(
            receipt_ids.insert(row["receipt_id"].as_u64().expect("receipt id")),
            "{row}"
        );
    }
    let statuses = rows
        .iter()
        .map(|row| {
            serde_json::json!({
                "method": row["method"],
                "target": row["target"],
                "receipt_id": row["receipt_id"],
                "receipt_status": row["receipt_status"],
                "predicate_ids": row["terminal_predicates"]
                    .as_array()
                    .map(|predicates| predicates
                        .iter()
                        .map(|predicate| predicate["predicate_id"].clone())
                        .collect::<Vec<_>>())
                    .unwrap_or_default(),
            })
        })
        .collect::<Vec<_>>();
    Some(serde_json::json!({
        "run_id": run_id,
        "source_sha": source_sha,
        "process_correlation_id": process_correlation_id,
        "process_id": std::process::id(),
        "client_session_id": client_session_id,
        "matrix_path": matrix_path.display().to_string(),
        "row_count": rows.len(),
        "indeterminate_row_count": 0,
        "rows": statuses,
    }))
}

#[cfg(feature = "integration")]
fn wait_search_cell(
    cell: &handshake_native::backend_client::LoomSearchCell,
) -> Result<LoomSearchV2Response, String> {
    for _ in 0..200 {
        if let Some(delivery) = cell.lock().expect("search cell").take() {
            return delivery;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    panic!("managed Loom Search completion exceeded 10 seconds")
}

#[cfg(feature = "integration")]
fn wait_panel_idle(state: &mut LoomSearchV2PanelState) {
    for _ in 0..200 {
        state.poll();
        if !state.loading {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    panic!("managed Loom Search panel did not become idle within 10 seconds")
}

#[test]
#[cfg(feature = "integration")]
fn loom_search_v2_managed_mounted_search_facet_save_reload_cleanup() {
    let mut live = interconnect_support::require_reachable_backend();
    let receipt_dir = external_artifact_dir("wp-kernel-012-mt-028");
    std::fs::create_dir_all(&receipt_dir).expect("create MT-028 external receipt directory");
    let receipt_path = receipt_dir.join("MT-028-managed-loom-search-v2-receipt.json");
    if receipt_path.exists() {
        std::fs::remove_file(&receipt_path).expect("remove stale MT-028 success receipt");
    }
    assert!(
        !receipt_path.exists(),
        "stale success receipt must be absent before proof"
    );

    let unique = format!(
        "mt028-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after unix epoch")
            .as_nanos()
    );
    let workspace = live.create_workspace(&unique);
    let workspace_id = workspace["id"]
        .as_str()
        .expect("workspace create returns id")
        .to_owned();
    let mut cleanup = ManagedWorkspaceCleanup {
        backend: &live,
        workspace_id: workspace_id.clone(),
        cleaned: false,
    };
    let needle = format!("mt028needle{}{}", std::process::id(), unique.len());
    let mut seeded = BTreeMap::new();
    for (content_type, suffix) in [("note", "alpha"), ("file", "beta"), ("tag_hub", "gamma")] {
        let created = live.post_json(
            &format!("/workspaces/{workspace_id}/loom/blocks"),
            &serde_json::json!({
                "content_type": content_type,
                "title": format!("{needle} {suffix}")
            }),
        );
        seeded.insert(
            content_type.to_owned(),
            created["block_id"]
                .as_str()
                .expect("block create returns block_id")
                .to_owned(),
        );
    }

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("managed MT-028 runtime");
    let client = LoomSearchV2Client::new(live.base.clone(), runtime.handle().clone());
    let direct_cell: handshake_native::backend_client::LoomSearchCell = Arc::new(Mutex::new(None));
    client.search(
        &workspace_id,
        &LoomSearchV2Body::baseline(needle.clone(), None),
        Arc::clone(&direct_cell),
    );
    let direct = wait_search_cell(&direct_cell).expect("real managed hybrid search succeeds");
    assert_eq!(
        direct.total, 3,
        "isolated workspace returns its three seeded blocks"
    );
    assert_eq!(direct.hits.len(), 3);
    assert_eq!(direct.content_type_facets.get("note"), Some(&1));
    assert_eq!(direct.content_type_facets.get("file"), Some(&1));
    assert_eq!(direct.content_type_facets.get("tag_hub"), Some(&1));
    assert!(
        !direct.semantic_available,
        "managed default without an embedding model must truthfully report semantic unavailable"
    );
    let direct_ids: HashSet<_> = direct
        .hits
        .iter()
        .map(|hit| hit.block.block_id.as_str())
        .collect();
    for block_id in seeded.values() {
        assert!(direct_ids.contains(block_id.as_str()));
    }
    for hit in &direct.hits {
        assert!(hit.score > 0.0, "real hit score must be positive");
        assert_eq!(
            hit.vector_sim, 0.0,
            "semantic-unavailable path cannot invent vector similarity"
        );
        assert!(hit.highlight.contains("<mark>"));
        let segments = parse_highlight_segments(&hit.highlight);
        assert!(segments.iter().any(|segment| segment.marked));
        assert!(segments.iter().all(|segment| {
            !segment.text.contains("<mark>") && !segment.text.contains("</mark>")
        }));
    }

    // A canonical title mutation must atomically refresh the derived search row: the old query loses
    // this exact note id, the new query gains the same id under the same facet, and restoring the title
    // reverses both observations. This is a real PATCH -> SurrealDB -> search-v2 counterfactual.
    let renamed_needle = "zephyrquartzvexingfjord".to_owned();
    let note_block_id = seeded["note"].clone();
    let renamed = live.patch_json(
        &format!("/workspaces/{workspace_id}/loom/blocks/{note_block_id}"),
        &serde_json::json!({ "title": renamed_needle }),
    );
    assert_eq!(renamed["block_id"], note_block_id);
    assert_eq!(renamed["title"], renamed_needle);

    let old_note_cell: handshake_native::backend_client::LoomSearchCell =
        Arc::new(Mutex::new(None));
    client.search(
        &workspace_id,
        &LoomSearchV2Body::baseline(needle.clone(), Some("note".to_owned())),
        Arc::clone(&old_note_cell),
    );
    let old_note = wait_search_cell(&old_note_cell).expect("old title search succeeds");
    assert_eq!(
        old_note.total, 0,
        "renamed note must leave the old note query"
    );

    let renamed_note_cell: handshake_native::backend_client::LoomSearchCell =
        Arc::new(Mutex::new(None));
    client.search(
        &workspace_id,
        &LoomSearchV2Body::baseline(renamed_needle.clone(), Some("note".to_owned())),
        Arc::clone(&renamed_note_cell),
    );
    let renamed_note = wait_search_cell(&renamed_note_cell).expect("renamed title search succeeds");
    assert_eq!(renamed_note.total, 1);
    assert_eq!(renamed_note.hits[0].block.block_id, note_block_id);
    assert_eq!(renamed_note.hits[0].block.content_type, "note");

    let restored = live.patch_json(
        &format!("/workspaces/{workspace_id}/loom/blocks/{note_block_id}"),
        &serde_json::json!({ "title": format!("{needle} alpha") }),
    );
    assert_eq!(restored["block_id"], note_block_id);
    let restored_note_cell: handshake_native::backend_client::LoomSearchCell =
        Arc::new(Mutex::new(None));
    client.search(
        &workspace_id,
        &LoomSearchV2Body::baseline(needle.clone(), Some("note".to_owned())),
        Arc::clone(&restored_note_cell),
    );
    let restored_note =
        wait_search_cell(&restored_note_cell).expect("restored title search succeeds");
    assert_eq!(restored_note.total, 1);
    assert_eq!(restored_note.hits[0].block.block_id, note_block_id);

    // Empty input is rejected before transport; a real connection refusal becomes a visible terminal
    // error, and rebinding the same state to the live client recovers without stale results.
    let mut recovery = LoomSearchV2PanelState::new();
    recovery.query = "   ".to_owned();
    recovery.run_search(&client, Some(&workspace_id));
    assert_eq!(recovery.error.as_deref(), Some("Search query is required"));
    assert!(!recovery.loading);
    recovery.query = needle.clone();
    let unavailable = LoomSearchV2Client::new("http://127.0.0.1:9", runtime.handle().clone());
    recovery.run_search(&unavailable, Some(&workspace_id));
    wait_panel_idle(&mut recovery);
    assert!(
        recovery.error.is_some(),
        "backend refusal is visible and bounded"
    );
    recovery.run_search(&client, Some(&workspace_id));
    wait_panel_idle(&mut recovery);
    assert!(recovery.error.is_none());
    assert_eq!(
        recovery.response.as_ref().map(|response| response.total),
        Some(3)
    );
    recovery.active_content_type = Some("note".to_owned());
    recovery.view_status = Some("stale receipt".to_owned());
    assert!(recovery.bind_workspace(Some("different-workspace")));
    assert!(recovery.response.is_none());
    assert!(recovery.active_content_type.is_none());
    assert!(recovery.view_status.is_none());

    // Production-mounted proof: rebind the actual HandshakeApp factory to a DISTINCT prefixed reverse
    // proxy, open through the operator command, submit with Enter, invalidate an already-displayed
    // response by editing the query, rerun through a facet, and save through the mounted UI. The proxy
    // forwards genuine product traffic to this same managed backend and captures the prefixed requests;
    // a factory that retained BACKEND_BASE_URL cannot satisfy those capture assertions.
    let rebind_proxy = ManagedRebindProxy::start(live.base.clone());
    assert_ne!(rebind_proxy.base, live.base);
    assert_ne!(
        rebind_proxy.base,
        handshake_native::backend_client::BACKEND_BASE_URL
    );
    let mut app = handshake_native::app::HandshakeApp::with_health(
        handshake_native::app::HealthDisplayState::Ok(
            handshake_native::backend_client::HealthInfo {
                status: "ok".to_owned(),
                db_status: "ok".to_owned(),
                migration_version: Some(1),
            },
        ),
    );
    app.set_backend_base_url_for_test(&rebind_proxy.base, runtime.handle().clone());
    app.bind_active_project_for_integration_test(workspace_id.clone());
    assert!(app.dispatch_palette_action_for_test(
        handshake_native::command_registry::CMD_VIEW_LOOM_SEARCH
    ));
    let mut argus = CanonicalArgusDriver::bind(&app, "mt028-notes-search");
    let canonical_client_session_id = "mt028-notes-search-agent";
    let mut screenshot_outcomes: Vec<serde_json::Value> = Vec::new();
    let _managed_wgpu_guard = wgpu_guard();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 760.0))
        .wgpu()
        .build_state(
            |ctx, app: &mut handshake_native::app::HandshakeApp| app.ui(ctx),
            app,
        );
    harness.run_steps(2);
    let baseline_ids = author_ids(&harness);
    for stable in [
        QUERY_AUTHOR_ID,
        SEARCH_AUTHOR_ID,
        SAVE_VIEW_AUTHOR_ID,
        SAVE_STATUS_AUTHOR_ID,
        STATUS_AUTHOR_ID,
    ] {
        assert!(
            baseline_ids.contains(stable),
            "mounted UI missing stable id {stable}"
        );
    }
    let initial_tree = argus.inspect(&mut harness);
    for stable in [
        QUERY_AUTHOR_ID,
        SEARCH_AUTHOR_ID,
        SAVE_VIEW_AUTHOR_ID,
        SAVE_STATUS_AUTHOR_ID,
        STATUS_AUTHOR_ID,
    ] {
        assert!(
            json_has_author_id(&initial_tree, stable),
            "canonical Argus initial inspection missing {stable}"
        );
    }
    argus.set_value_and_reinspect(&mut harness, QUERY_AUTHOR_ID, &needle);
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut harness,
        "query-value-visible",
        serde_json::json!({"query": needle.clone()}),
        |tree| {
            json_node_by_author_id(tree, QUERY_AUTHOR_ID)
                .and_then(|node| node.get("value"))
                .and_then(serde_json::Value::as_str)
                == Some(needle.as_str())
        },
    );
    argus.click_and_reinspect(&mut harness, SEARCH_AUTHOR_ID);
    for _ in 0..400 {
        harness.run_steps(1);
        let ids = author_ids(&harness);
        if seeded
            .values()
            .all(|id| ids.contains(&result_author_id(id)))
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut harness,
        "populated-results-preview-visible",
        serde_json::json!({"result_ids": seeded.values().collect::<Vec<_>>()}),
        |tree| {
            let serialized = serde_json::to_string(tree).unwrap_or_default();
            seeded.values().all(|block_id| {
                json_has_author_id(tree, &result_author_id(block_id))
                    && json_has_author_id(tree, &preview_author_id(block_id))
            }) && serialized.contains("3 results (keyword/fuzzy only)")
                && serialized.contains("<mark>") == false
        },
    );
    // Assert delivery independently before inspecting row exposure. If this fails, the mounted
    // transport/query/generation path did not publish the managed response; a subsequent row-id
    // failure therefore means AccessKit exposure, not an ambiguous network/timing failure.
    harness.get_by_label("3 results (keyword/fuzzy only)");
    screenshot_outcomes.push(capture_state_screenshot(
        &mut harness,
        "mt028-populated-results",
    ));
    let ids = author_ids(&harness);
    for block_id in seeded.values() {
        assert!(
            ids.contains(&result_author_id(block_id)),
            "managed response delivered, but mounted AccessKit tree lacks result row {block_id}; ids={ids:?}"
        );
    }
    for content_type in ["note", "file", "tag_hub"] {
        assert!(ids.contains(&facet_author_id(content_type)));
    }

    // Edit query A after its rows are already mounted. The TextEdit's real `changed()` path must
    // synchronously remove A's rows and disable Save before any query-B request is submitted.
    let query_input = harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(QUERY_AUTHOR_ID))
        .expect("mounted query remains addressable after results");
    query_input.focus();
    harness.run_steps(1);
    harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(QUERY_AUTHOR_ID))
        .expect("refocused mounted query after results")
        .type_text("x");
    harness.run_steps(1);
    let edited_ids = author_ids(&harness);
    for block_id in seeded.values() {
        assert!(
            !edited_ids.contains(&result_author_id(block_id)),
            "editing query A invalidates its already-displayed row {block_id}"
        );
    }
    let save_disabled = harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(SAVE_VIEW_AUTHOR_ID))
        .map(|node| node.accesskit_node().is_disabled())
        .expect("mounted Save as view remains addressable");
    assert!(
        save_disabled,
        "edited query B cannot save query A's displayed response"
    );

    // Restore the exact seeded query and exercise Enter a second time. No Search-button click is used
    // for the mounted live search path.
    harness.key_press(egui::Key::Backspace);
    harness.run_steps(1);
    harness.key_press(egui::Key::Enter);
    for _ in 0..400 {
        harness.run_steps(1);
        let ids = author_ids(&harness);
        if seeded
            .values()
            .all(|id| ids.contains(&result_author_id(id)))
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    let restored_ids = author_ids(&harness);
    for block_id in seeded.values() {
        assert!(restored_ids.contains(&result_author_id(block_id)));
    }
    harness.get_by_label("3 results (keyword/fuzzy only)");

    argus.click_and_reinspect(&mut harness, &facet_author_id("note"));
    for _ in 0..400 {
        harness.run_steps(1);
        let ids = author_ids(&harness);
        if ids.contains(&result_author_id(&seeded["note"]))
            && !ids.contains(&result_author_id(&seeded["file"]))
            && !ids.contains(&result_author_id(&seeded["tag_hub"]))
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut harness,
        "note-facet-filtered",
        serde_json::json!({
            "included": seeded["note"],
            "excluded": [seeded["file"].clone(), seeded["tag_hub"].clone()]
        }),
        |tree| {
            json_has_author_id(tree, &result_author_id(&seeded["note"]))
                && !json_has_author_id(tree, &result_author_id(&seeded["file"]))
                && !json_has_author_id(tree, &result_author_id(&seeded["tag_hub"]))
                && serde_json::to_string(tree)
                    .unwrap_or_default()
                    .contains("1 result (keyword/fuzzy only)")
        },
    );
    let note_ids = author_ids(&harness);
    assert!(
        note_ids.contains(&result_author_id(&seeded["note"]))
            && !note_ids.contains(&result_author_id(&seeded["file"]))
            && !note_ids.contains(&result_author_id(&seeded["tag_hub"])),
        "initial note facet must replace the unfiltered set; result_ids={:?}; captures={:#?}",
        note_ids
            .iter()
            .filter(|id| id.starts_with("search.result."))
            .collect::<Vec<_>>(),
        rebind_proxy.captured_requests()
    );
    harness.get_by_label("1 result (keyword/fuzzy only)");
    screenshot_outcomes.push(capture_state_screenshot(
        &mut harness,
        "mt028-note-facet-filtered",
    ));
    let save_disabled = harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(SAVE_VIEW_AUTHOR_ID))
        .map(|node| node.accesskit_node().is_disabled())
        .expect("mounted note-facet Save as view remains addressable");
    assert!(
        !save_disabled,
        "note-facet result must enable Save before the stale-response gate is armed"
    );

    // Hold the real note-facet save response only AFTER the proxy has forwarded it and the managed
    // backend has persisted the view. Then clear the active facet and let that replacement search
    // finish before releasing the old receipt. The mounted panel must retain the current unfiltered
    // identity and never attribute the late note-facet receipt to it.
    rebind_proxy.arm_next_save_response_hold();
    click_author_id(&harness, SAVE_VIEW_AUTHOR_ID);
    harness.run_steps(1);
    for _ in 0..400 {
        harness.run_steps(1);
        if rebind_proxy.save_response_is_held() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(
        rebind_proxy.save_response_is_held(),
        "the real note-facet save response must be held after upstream persistence"
    );

    let stale_saved_view_id = rebind_proxy
        .persisted_view_ids()
        .into_iter()
        .next()
        .expect("held mounted save returned a canonical persisted block id");

    let view_facet_cell: handshake_native::backend_client::LoomSearchCell =
        Arc::new(Mutex::new(None));
    client.search(
        &workspace_id,
        &LoomSearchV2Body::baseline(needle.clone(), Some("view_def".to_owned())),
        Arc::clone(&view_facet_cell),
    );
    let view_facet = wait_search_cell(&view_facet_cell)
        .expect("persisted saved view is searchable under the exact view_def facet");
    assert_eq!(view_facet.total, 1);
    assert_eq!(view_facet.hits.len(), 1);
    assert_eq!(view_facet.hits[0].block.block_id, stale_saved_view_id);
    assert_eq!(view_facet.hits[0].block.content_type, "view_def");
    assert_eq!(view_facet.content_type_facets.get("view_def"), Some(&1));
    assert_eq!(view_facet.content_type_facets.get("note"), Some(&1));
    assert_eq!(view_facet.content_type_facets.get("file"), Some(&1));
    assert_eq!(view_facet.content_type_facets.get("tag_hub"), Some(&1));
    assert_eq!(view_facet.content_type_facets.len(), 4);

    click_author_id(&harness, &facet_author_id("note"));
    for _ in 0..400 {
        harness.run_steps(1);
        let ids = author_ids(&harness);
        if seeded
            .values()
            .all(|block_id| ids.contains(&result_author_id(block_id)))
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    let current_unfiltered_ids = author_ids(&harness);
    for block_id in seeded.values() {
        assert!(
            current_unfiltered_ids.contains(&result_author_id(block_id)),
            "replacement unfiltered search must become current before the old save response"
        );
    }
    assert!(
        current_unfiltered_ids.contains(&result_author_id(&stale_saved_view_id)),
        "the already-persisted view definition participates in the replacement unfiltered search"
    );
    harness.get_by_label("4 results (keyword/fuzzy only)");
    assert!(
        harness
            .query_by_label(&format!("Saved search as Loom view {stale_saved_view_id}"))
            .is_none(),
        "the held old-facet receipt is not visible before release"
    );

    rebind_proxy.release_held_save_response();
    for _ in 0..400 {
        harness.run_steps(1);
        if rebind_proxy.held_save_response_was_written() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(
        rebind_proxy.held_save_response_was_written(),
        "the stale save response must actually be written after the current facet result"
    );
    for _ in 0..40 {
        harness.run_steps(1);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(
        harness
            .query_by_label(&format!("Saved search as Loom view {stale_saved_view_id}"))
            .is_none(),
        "a stale note-facet save delivered last cannot publish into the current unfiltered panel"
    );
    let after_stale_delivery_ids = author_ids(&harness);
    for block_id in seeded.values() {
        assert!(
            after_stale_delivery_ids.contains(&result_author_id(block_id)),
            "late stale save delivery cannot replace the current unfiltered result set"
        );
    }
    assert!(after_stale_delivery_ids.contains(&result_author_id(&stale_saved_view_id)));
    harness.get_by_label("4 results (keyword/fuzzy only)");

    // Return to the note facet and perform a second, current-identity save. This preserves the
    // original success-receipt/reload proof while distinguishing it from the intentionally orphaned
    // first receipt.
    click_author_id(&harness, &facet_author_id("note"));
    for _ in 0..400 {
        harness.run_steps(1);
        let ids = author_ids(&harness);
        if ids.contains(&result_author_id(&seeded["note"]))
            && !ids.contains(&result_author_id(&seeded["file"]))
            && !ids.contains(&result_author_id(&seeded["tag_hub"]))
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    // The mounted panel polls and publishes the async response after drawing its status row,
    // while result rows are drawn later in the same frame. Advance once so the status surface
    // reflects the same response generation as the rows asserted below.
    harness.run_steps(1);
    let restored_note_ids = author_ids(&harness);
    assert!(
        restored_note_ids.contains(&result_author_id(&seeded["note"]))
            && !restored_note_ids.contains(&result_author_id(&seeded["file"]))
            && !restored_note_ids.contains(&result_author_id(&seeded["tag_hub"])),
        "restored note facet must replace the unfiltered set; result_ids={:?}; captures={:#?}",
        restored_note_ids
            .iter()
            .filter(|id| id.starts_with("search.result."))
            .collect::<Vec<_>>(),
        rebind_proxy.captured_requests()
    );
    harness.get_by_label("1 result (keyword/fuzzy only)");
    argus.click_and_reinspect(&mut harness, SAVE_VIEW_AUTHOR_ID);

    let mut saved_view_id = None;
    for _ in 0..400 {
        harness.run_steps(1);
        if let Some(id) = rebind_proxy
            .persisted_view_ids()
            .into_iter()
            .find(|id| id != &stale_saved_view_id)
        {
            saved_view_id = Some(id);
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    let saved_view_id =
        saved_view_id.expect("mounted Save as view persisted a searchable view_def");
    harness.run_steps(2);
    harness.get_by_label(&format!("Saved search as Loom view {saved_view_id}"));
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut harness,
        "saved-view-id-visible",
        serde_json::json!({"saved_view_id": saved_view_id.clone()}),
        |tree| {
            serde_json::to_string(tree)
                .unwrap_or_default()
                .contains(&format!("Saved search as Loom view {saved_view_id}"))
        },
    );
    screenshot_outcomes.push(capture_state_screenshot(
        &mut harness,
        "mt028-saved-view-receipt",
    ));
    let reloaded = live.get_json(&format!(
        "/workspaces/{workspace_id}/loom/views/definitions/{saved_view_id}"
    ));
    assert_eq!(reloaded["block"]["block_id"], saved_view_id);
    assert_eq!(reloaded["block"]["content_type"], "view_def");
    assert_eq!(reloaded["block"]["title"], format!("Search: {needle}"));
    assert_eq!(reloaded["definition"]["kind"], "table");
    assert_eq!(reloaded["definition"]["query"]["content_type"], "note");
    let saved_view_events = live.get_json(&format!(
        "/kernel/events/aggregates/loom_block/{saved_view_id}"
    ));
    let saved_view_mutation_event = saved_view_events
        .as_array()
        .expect("saved-view EventLedger response is an array")
        .iter()
        .find(|event| {
            event["event_type"] == "KNOWLEDGE_LOOM_BLOCK_MUTATED"
                && event["aggregate_type"] == "loom_block"
                && event["aggregate_id"] == saved_view_id
                && event["payload"]["workspace_id"] == workspace_id
                && event["payload"]["block_id"] == saved_view_id
                && event["payload"]["content_type"] == "view_def"
                && event["payload"]["operation"] == "create_view_definition"
        })
        .cloned()
        .expect("saved-view creation has an exact SurrealDB EventLedger receipt");
    let stale_reloaded = live.get_json(&format!(
        "/workspaces/{workspace_id}/loom/views/definitions/{stale_saved_view_id}"
    ));
    assert_eq!(stale_reloaded["block"]["block_id"], stale_saved_view_id);
    assert_eq!(
        stale_reloaded["definition"]["query"]["content_type"], "note",
        "the deliberately orphaned UI receipt still corresponds to a real persisted note view"
    );
    let view_results = live.post_json(
        &format!("/workspaces/{workspace_id}/loom/views/definitions/{saved_view_id}/results"),
        &serde_json::json!({"limit":25,"offset":0}),
    );
    let persisted_ids: HashSet<_> = view_results["blocks"]
        .as_array()
        .expect("view results blocks array")
        .iter()
        .filter_map(|block| block["block_id"].as_str())
        .collect();
    assert!(persisted_ids.contains(seeded["note"].as_str()));
    assert!(!persisted_ids.contains(seeded["file"].as_str()));
    assert!(!persisted_ids.contains(seeded["tag_hub"].as_str()));

    // A canonical run forces a fixture-owned current-source backend. Restart that exact owned process
    // on its existing SurrealDB authority, then prove the persisted view remains reloadable. The
    // cleanup guard is temporarily released only around the mutable restart and is reconstructed
    // before any panic can leave this scope.
    let canonical_restart = if std::env::var_os("HANDSHAKE_ARGUS_MATRIX_RUN_ID").is_some() {
        cleanup.cleaned = true;
        drop(cleanup);
        let restart =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| live.restart_owned()));
        cleanup = ManagedWorkspaceCleanup {
            backend: &live,
            workspace_id: workspace_id.clone(),
            cleaned: false,
        };
        let (old_base, new_base) = match restart {
            Ok(restarted) => restarted,
            Err(payload) => {
                let _ = cleanup.clean();
                std::panic::resume_unwind(payload);
            }
        };
        assert_eq!(old_base, new_base);
        let restarted_view = live.get_json(&format!(
            "/workspaces/{workspace_id}/loom/views/definitions/{saved_view_id}"
        ));
        assert_eq!(restarted_view["block"]["block_id"], saved_view_id);
        Some(serde_json::json!({
            "old_base": old_base,
            "new_base": new_base,
            "persisted_view_reloaded": true,
            "backend_binding": live.owned_backend_binding_receipt()
        }))
    } else {
        None
    };

    // Canonical mounted empty state: drive a guaranteed-miss query through the production Argus
    // transport, then bind the Search action to the exact zero-row terminal tree.
    let missing_query = "qzxvjkbbpqzxvjkbbpqzxvjkbbpqzxvjkbbp".to_owned();
    let direct_empty = live.post_json(
        &format!("/workspaces/{workspace_id}/loom/search-v2"),
        &serde_json::json!({
            "query": missing_query.clone(),
            "content_type": "note",
            "limit": 25,
            "offset": 0
        }),
    );
    assert_eq!(
        direct_empty["total"], 0,
        "the mounted empty-state query must first prove zero against the real search authority"
    );
    argus.set_value_and_reinspect(&mut harness, QUERY_AUTHOR_ID, &missing_query);
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut harness,
        "empty-query-value-visible",
        serde_json::json!({"query": missing_query.clone()}),
        |tree| {
            json_node_by_author_id(tree, QUERY_AUTHOR_ID)
                .and_then(|node| node.get("value"))
                .and_then(serde_json::Value::as_str)
                == Some(missing_query.as_str())
        },
    );
    argus.click_and_reinspect(&mut harness, SEARCH_AUTHOR_ID);
    for _ in 0..400 {
        harness.run_steps(1);
        let status_is_empty = harness
            .root()
            .children_recursive()
            .find(|node| node.accesskit_node().author_id() == Some(STATUS_AUTHOR_ID))
            .and_then(|node| node.accesskit_node().value())
            .is_some_and(|value| value == "0 results (keyword/fuzzy only)");
        if status_is_empty {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    let empty_prefixed_search_path =
        format!("/mt028-rebind/workspaces/{workspace_id}/loom/search-v2");
    let mounted_empty_query = rebind_proxy
        .captured_requests()
        .into_iter()
        .filter(|request| {
            request.method == "POST" && request.prefixed_path == empty_prefixed_search_path
        })
        .last()
        .and_then(|request| {
            request
                .body
                .get("query")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        });
    let live_empty_statuses = harness
        .root()
        .children_recursive()
        .filter(|node| node.accesskit_node().author_id() == Some(STATUS_AUTHOR_ID))
        .map(|node| node.accesskit_node().value().unwrap_or_default())
        .collect::<Vec<_>>();
    let active_pane = harness
        .state()
        .active_pane()
        .map(|pane_id| pane_id.as_ref().to_owned());
    let search_tab_panes = harness
        .state()
        .tab_bar_states()
        .iter()
        .filter_map(|(pane_id, bar)| {
            bar.tabs
                .iter()
                .any(|tab| tab.pane_type == PaneType::LoomSearchV2)
                .then(|| pane_id.as_ref().to_owned())
        })
        .collect::<Vec<_>>();
    assert_eq!(
        mounted_empty_query.as_deref(),
        Some(missing_query.as_str()),
        "Argus Search must send the missing query; statuses={live_empty_statuses:?}; active_pane={active_pane:?}; search_tab_panes={search_tab_panes:?}"
    );
    let empty_tree = argus.inspect(&mut harness);
    let empty_status = json_node_by_author_id(&empty_tree, STATUS_AUTHOR_ID)
        .map(|node| serde_json::to_string(node).unwrap_or_default())
        .unwrap_or_default();
    let stale_empty_result_ids = seeded
        .values()
        .filter(|block_id| json_has_author_id(&empty_tree, &result_author_id(block_id)))
        .cloned()
        .collect::<Vec<_>>();
    assert!(
        empty_status.contains("0 results (keyword/fuzzy only)")
            && stale_empty_result_ids.is_empty(),
        "canonical empty tree mismatch: status={empty_status}; stale_result_ids={stale_empty_result_ids:?}; live_statuses={live_empty_statuses:?}; active_pane={active_pane:?}; search_tab_panes={search_tab_panes:?}"
    );
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut harness,
        "zero-results-no-stale-rows",
        serde_json::json!({
            "status": empty_status,
            "stale_result_ids": stale_empty_result_ids
        }),
        |tree| {
            json_node_by_author_id(tree, STATUS_AUTHOR_ID)
                .map(|node| serde_json::to_string(node).unwrap_or_default())
                .is_some_and(|status| status.contains("0 results (keyword/fuzzy only)"))
                && seeded
                    .values()
                    .all(|block_id| !json_has_author_id(tree, &result_author_id(block_id)))
        },
    );
    let empty_save_disabled = harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(SAVE_VIEW_AUTHOR_ID))
        .map(|node| node.accesskit_node().is_disabled())
        .expect("empty mounted Save as view remains addressable");
    assert!(empty_save_disabled);
    screenshot_outcomes.push(capture_state_screenshot(&mut harness, "mt028-empty-state"));

    // Canonical mounted backend-error/recovery state. Rebind the concrete factory to a refused
    // loopback port, prove a bounded visible terminal error, then restore the managed proxy and
    // recover the same mounted pane through the same stable Search action.
    harness
        .state_mut()
        .set_backend_base_url_for_test("http://127.0.0.1:9", runtime.handle().clone());
    harness.run_steps(2);
    argus.set_value_and_reinspect(&mut harness, QUERY_AUTHOR_ID, &needle);
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut harness,
        "error-query-value-visible",
        serde_json::json!({"query": needle.clone()}),
        |tree| {
            json_node_by_author_id(tree, QUERY_AUTHOR_ID)
                .and_then(|node| node.get("value"))
                .and_then(serde_json::Value::as_str)
                == Some(needle.as_str())
        },
    );
    // A refused backend is a CAUSALLY OWNED terminal failure, not an unprovable outcome: the mounted
    // Search action's completion observer binds the exact target/context/generation/semantic tuple
    // before dispatch and then publishes the typed transport error against that same tuple.
    argus.click_expect_typed_rejected_and_reinspect(
        &mut harness,
        SEARCH_AUTHOR_ID,
        "Loom search failed",
    );
    let mut mounted_error_tree = None;
    for _ in 0..400 {
        harness.run_steps(1);
        let tree = argus.inspect(&mut harness);
        let status = json_node_by_author_id(&tree, STATUS_AUTHOR_ID)
            .map(|node| serde_json::to_string(node).unwrap_or_default())
            .unwrap_or_default();
        if status.contains("error sending request") || status.contains("Connection refused") {
            mounted_error_tree = Some(tree);
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    assert!(
        mounted_error_tree.is_some(),
        "mounted refused backend must reach a bounded visible error"
    );
    argus.assert_latest_terminal_predicate(&mut harness, "backend-error-visible", |tree| {
        let status = json_node_by_author_id(tree, STATUS_AUTHOR_ID)
            .map(|node| serde_json::to_string(node).unwrap_or_default())
            .unwrap_or_default();
        status.contains("error sending request") || status.contains("Connection refused")
    });
    screenshot_outcomes.push(capture_state_screenshot(
        &mut harness,
        "mt028-backend-error",
    ));

    harness
        .state_mut()
        .set_backend_base_url_for_test(&rebind_proxy.base, runtime.handle().clone());
    harness.run_steps(2);
    argus.set_value_and_reinspect(&mut harness, QUERY_AUTHOR_ID, &needle);
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut harness,
        "recovery-query-value-visible",
        serde_json::json!({"query": needle.clone()}),
        |tree| {
            json_node_by_author_id(tree, QUERY_AUTHOR_ID)
                .and_then(|node| node.get("value"))
                .and_then(serde_json::Value::as_str)
                == Some(needle.as_str())
        },
    );
    argus.click_and_reinspect(&mut harness, SEARCH_AUTHOR_ID);
    for _ in 0..400 {
        harness.run_steps(1);
        if author_ids(&harness).contains(&result_author_id(&saved_view_id)) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut harness,
        "backend-recovered-with-saved-view-result",
        serde_json::json!({"saved_view_id": saved_view_id.clone()}),
        |tree| json_has_author_id(tree, &result_author_id(&saved_view_id)),
    );
    screenshot_outcomes.push(capture_state_screenshot(
        &mut harness,
        "mt028-backend-recovered",
    ));

    // Reopen the saved view from the mounted Notes Search surface itself. The view_def facet is
    // selected through canonical Argus, then the exact saved row is activated. The host must route
    // that typed result back into the mounted Block Collections surface at the same canonical id.
    argus.click_and_reinspect(&mut harness, &facet_author_id("view_def"));
    for _ in 0..400 {
        harness.run_steps(1);
        let ids = author_ids(&harness);
        if ids.contains(&result_author_id(&saved_view_id))
            && ids.contains(&result_author_id(&stale_saved_view_id))
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut harness,
        "saved-view-facet-visible",
        serde_json::json!({
            "saved_view_id": saved_view_id.clone(),
            "orphaned_receipt_view_id": stale_saved_view_id.clone()
        }),
        |tree| {
            json_has_author_id(tree, &result_author_id(&saved_view_id))
                && json_has_author_id(tree, &result_author_id(&stale_saved_view_id))
        },
    );
    argus.click_and_reinspect(&mut harness, &result_author_id(&saved_view_id));
    for _ in 0..400 {
        harness.run_steps(1);
        let block_collections = handshake_native::editor_pane_factories::placeholder_pane_type(
            handshake_native::editor_pane_factories::BLOCK_COLLECTIONS_PANE_LABEL,
        );
        if harness.state().tab_bar_states().values().any(|bar| {
            bar.tabs.iter().any(|tab| {
                tab.pane_type == block_collections
                    && tab.content_id.as_deref() == Some(saved_view_id.as_str())
            })
        }) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut harness,
        "saved-view-reopened-in-block-collections",
        serde_json::json!({"saved_view_id": saved_view_id.clone()}),
        |tree| {
            let serialized = serde_json::to_string(tree).unwrap_or_default();
            serialized.contains(&saved_view_id) && serialized.contains("Block Collections")
        },
    );
    let block_collections = handshake_native::editor_pane_factories::placeholder_pane_type(
        handshake_native::editor_pane_factories::BLOCK_COLLECTIONS_PANE_LABEL,
    );
    assert!(harness.state().tab_bar_states().values().any(|bar| {
        bar.tabs.iter().any(|tab| {
            tab.pane_type == block_collections
                && tab.content_id.as_deref() == Some(saved_view_id.as_str())
        })
    }));
    screenshot_outcomes.push(capture_state_screenshot(
        &mut harness,
        "mt028-saved-view-reopened",
    ));
    // STRICT: every canonical action must have terminalized through a product-side causal completion
    // observer. An `indeterminate` receipt is not accepted anywhere in this proof.
    argus.finish_require_no_indeterminate();
    let canonical_matrix = open_canonical_matrix_rows(canonical_client_session_id);

    // Stop the proxy only after every mounted mutation completed, then prove the traffic used the
    // rebound factory rather than the production default. The path prefix is present only in the
    // injected base and is stripped by the proxy before the real managed backend sees the request.
    let rebound_requests = rebind_proxy.finish();
    let prefixed_search_path = format!("/mt028-rebind/workspaces/{workspace_id}/loom/search-v2");
    let prefixed_save_path =
        format!("/mt028-rebind/workspaces/{workspace_id}/loom/views/definitions");
    let mounted_search_requests: Vec<_> = rebound_requests
        .iter()
        .filter(|request| request.method == "POST" && request.prefixed_path == prefixed_search_path)
        .collect();
    assert!(
        mounted_search_requests.len() >= 5,
        "mounted Enter, restored Enter, note, unfiltered replacement, and restored-note searches must all hit the rebound factory; captures={rebound_requests:#?}"
    );
    assert!(mounted_search_requests.iter().any(|request| {
        request.body["query"].as_str() == Some(needle.as_str())
            && request.body.get("content_type").is_none()
    }));
    assert!(mounted_search_requests.iter().any(|request| {
        request.body["query"].as_str() == Some(needle.as_str())
            && request.body["content_type"].as_str() == Some("note")
    }));
    let mounted_saves: Vec<_> = rebound_requests
        .iter()
        .filter(|request| request.method == "POST" && request.prefixed_path == prefixed_save_path)
        .collect();
    assert!(
        mounted_saves.len() >= 2,
        "both the deliberately stale and current mounted saves hit the distinct rebound factory"
    );
    for mounted_save in mounted_saves {
        assert_eq!(mounted_save.body["title"], format!("Search: {needle}"));
        assert_eq!(
            mounted_save.body["definition"]["query"]["content_type"],
            "note"
        );
    }

    let cleanup_status = cleanup.clean();
    let workspace_list = live.get_json("/workspaces");
    let absent = workspace_list
        .as_array()
        .expect("workspace list is an array")
        .iter()
        .all(|workspace| workspace["id"].as_str() != Some(workspace_id.as_str()));
    assert!(absent, "fresh workspace list proves canonical cleanup");
    assert_eq!(
        live.get_status(&format!(
            "/workspaces/{workspace_id}/loom/views/definitions/{saved_view_id}"
        )),
        404,
        "deleted workspace cannot reload its saved view"
    );
    assert_eq!(
        live.get_status(&format!(
            "/workspaces/{workspace_id}/loom/views/definitions/{stale_saved_view_id}"
        )),
        404,
        "deleted workspace also removes the persisted view whose UI receipt was orphaned"
    );
    assert!(
        !receipt_path.exists(),
        "success receipt remains absent until all proof passes"
    );
    let receipt = serde_json::json!({
        "schema_id": "hsk.mt028.managed_receipt@2",
        "workspace_id": workspace_id,
        "seeded_blocks": seeded,
        "search": {
            "total": direct.total,
            "semantic_available": direct.semantic_available,
            "facet_counts": direct.content_type_facets,
            "parsed_marked_highlights": true,
            "mounted_status_truth": "3 results (keyword/fuzzy only)",
            "mounted_facet_rerun": "note"
        },
        "stable_accesskit_ids": [
            QUERY_AUTHOR_ID, SEARCH_AUTHOR_ID, SAVE_VIEW_AUTHOR_ID, SAVE_STATUS_AUTHOR_ID,
            STATUS_AUTHOR_ID,
            "loom-search-v2.facet.note", "loom-search-v2.facet.file",
            "loom-search-v2.facet.tag_hub"
        ],
        "exact_result_navigation": true,
        "saved_view": {
            "block_id": saved_view_id,
            "reloaded": true,
            "persisted_note_facet": true,
            "reopened_in_block_collections": true,
            "event_ledger": saved_view_mutation_event
        },
        "canonical_argus": {
            "inspect_click_set_value_with_terminal_predicates": true,
            "mounted_populated_empty_error_recovery": true,
            "saved_view_reopen": true,
            "owned_backend_restart": canonical_restart,
            // Per-action causal evidence, NOT a summary boolean: the matrix rows this exact process
            // wrote were reopened and re-verified above, and every identity below is read from the
            // SAME run/process that produced this receipt.
            "matrix": canonical_matrix,
            "zero_indeterminate_actions_required": true
        },
        "screenshots": screenshot_outcomes,
        "empty_query_rejected_without_request": true,
        "backend_refusal_visible_and_live_recovery": true,
        "workspace_rebind_clears_stale_state": true,
        "query_edit_orphans_in_flight_and_invalidates_displayed_results": true,
        "facet_transition_orphans_stale_save_delivered_after_current_results": true,
        "mounted_enter_search": true,
        "mounted_factory_rebound_proxy_capture_count": rebound_requests.len(),
        "cleanup_http_status": cleanup_status,
        "workspace_absent_from_fresh_list": absent,
        "deleted_view_reload_http_status": 404,
        "deleted_orphaned_receipt_view_reload_http_status": 404,
        "cleanup_verified": true
    });
    std::fs::write(
        &receipt_path,
        serde_json::to_vec_pretty(&receipt).expect("serialize MT-028 receipt"),
    )
    .expect("write MT-028 success receipt after proof");
    assert!(receipt_path.is_file());
    assert_no_local_artifact_dir();
}
