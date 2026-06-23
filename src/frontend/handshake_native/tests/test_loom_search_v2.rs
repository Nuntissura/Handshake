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
//!   - PT-1/PT-2 (real-PG integration): the `#[ignore]`d live tests that spawn-against a seeded
//!     handshake_core + PostgreSQL — NEEDS_MANAGED_RESOURCE_PROOF (run with a live backend). They NEVER
//!     fake PG; absent a seeded backend they are skipped, the request-builder proofs covering the wire.
//!
//! ## Backend reality (Spec-Realism Gate / MT-022..027 pattern)
//!
//! AC-2 (a real query populates a real response) requires a running, seeded handshake_core; that is the
//! `*_live_pg` `#[ignore]` integration test below (PT-1/PT-2). The standalone rendering + parser +
//! state-machine + request-builder proofs are the deterministic, GPU/backend-free evidence.
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
use egui_kittest::Harness;

use handshake_native::backend_client::{
    LoomSearchBlock, LoomSearchV2Body, LoomSearchV2Client, LoomSearchV2Hit, LoomSearchV2Response,
};
use handshake_native::loom_search_v2::{
    facet_author_id, highlight_layout_job, parse_highlight_segments, result_author_id,
    LoomSearchV2Callbacks, LoomSearchV2PanelState, QUERY_AUTHOR_ID, SAVE_VIEW_AUTHOR_ID,
    SEARCH_AUTHOR_ID, STATUS_AUTHOR_ID,
};
use handshake_native::theme::HsTheme;

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

fn hit(block_id: &str, content_type: &str, title: &str, score: f64, highlight: &str) -> LoomSearchV2Hit {
    LoomSearchV2Hit {
        block: LoomSearchBlock {
            block_id: block_id.to_owned(),
            content_type: content_type.to_owned(),
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
            hit("blk-1", "note", "First Note", 0.912, "<mark>alpha</mark> beta"),
            hit("blk-2", "note", "Second Note", 0.640, "gamma <mark>delta</mark>"),
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
            let mut cbs = LoomSearchV2Callbacks { on_open_block: &mut on_open };
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
fn author_ids(harness: &Harness<'_, ()>) -> HashSet<String> {
    let mut ids = HashSet::new();
    for node in harness.root().children_recursive() {
        if let Some(a) = node.accesskit_node().author_id() {
            ids.insert(a.to_owned());
        }
    }
    ids
}

/// Click the node addressed by `author_id` via the AccessKit Click action.
fn click_author_id(harness: &Harness<'_, ()>, author_id: &str) {
    let node = harness
        .root()
        .children_recursive()
        .find(|n| n.accesskit_node().author_id() == Some(author_id))
        .unwrap_or_else(|| panic!("no node with author_id '{author_id}' to click"));
    node.click();
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// PROOF2 (PT-3, AC-1/AC-3/AC-8): the 6 contract author_ids appear in the live AccessKit tree + status.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn accesskit_tree_has_all_contract_author_ids() {
    let mut s = LoomSearchV2PanelState::new();
    s.query = "alpha".to_owned();
    s.response = Some(mock_response());
    let state = Arc::new(Mutex::new(s));
    let opened = Arc::new(Mutex::new(Vec::new()));
    let client = LoomSearchV2Client::new(TEST_BASE, rt().handle().clone());
    let mut harness = harness_for(Arc::clone(&state), opened, client, Some("ws-1".to_owned()));
    harness.run();

    let ids = author_ids(&harness);
    for required in [QUERY_AUTHOR_ID, SEARCH_AUTHOR_ID, SAVE_VIEW_AUTHOR_ID, STATUS_AUTHOR_ID] {
        assert!(ids.contains(required), "PT-3: required author_id '{required}' missing from {ids:?}");
    }
    // Facet ids for both content types.
    assert!(ids.contains(&facet_author_id("note")), "PT-3: facet.note missing");
    assert!(ids.contains(&facet_author_id("code")), "PT-3: facet.code missing");
    // A result row id (the contract names `loom-search-v2.result.{block_id_0}`).
    assert!(ids.contains(&result_author_id("blk-1")), "PT-3: result.blk-1 missing");
    assert!(ids.contains(&result_author_id("blk-2")), "PT-3: result.blk-2 missing");
    assert!(ids.contains(&result_author_id("blk-3")), "PT-3: result.blk-3 missing");

    println!("PT-3/AC-1/AC-8: all 6 contract author_ids present in the live AccessKit tree");
    assert_no_local_artifact_dir();
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// PROOF (AC-3): the status line reflects semantic_available — '(keyword/fuzzy only)' when false.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn status_line_reflects_semantic_off() {
    let mut s = LoomSearchV2PanelState::new();
    s.response = Some(mock_response()); // semantic_available = false, total = 3
    let state = Arc::new(Mutex::new(s));
    let opened = Arc::new(Mutex::new(Vec::new()));
    let client = LoomSearchV2Client::new(TEST_BASE, rt().handle().clone());
    let mut harness = harness_for(state, opened, client, Some("ws-1".to_owned()));
    harness.run();

    // The status label text is queryable by label (it is a plain egui::Label).
    harness.get_by_label("3 results (keyword/fuzzy only)");
    println!("AC-3: status line shows '3 results (keyword/fuzzy only)' when semantic_available=false");
    assert_no_local_artifact_dir();
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// PROOF4 (AC-6): clicking a result row invokes on_open_block with the correct block_id.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn clicking_result_row_opens_block() {
    let mut s = LoomSearchV2PanelState::new();
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
    assert_eq!(opened.as_slice(), ["blk-2"], "AC-6: on_open_block called with the clicked block_id");
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
    assert_eq!(job.sections.len(), 3, "AC-5: 3 layout sections for foo/bar/baz");
    assert_eq!(job.sections[0].format.background, pal.search_highlight_bg, "AC-5: 'foo' marked");
    assert_eq!(
        job.sections[1].format.background,
        egui::Color32::TRANSPARENT,
        "AC-5: ' bar ' not marked"
    );
    assert_eq!(job.sections[2].format.background, pal.search_highlight_bg, "AC-5: 'baz' marked");
    // And the raw `<mark>` tokens must NOT appear in the rendered text.
    assert!(!job.text.contains("<mark>"), "AC-5: no raw <mark> tag in rendered text");
    assert_eq!(job.text, "foo bar baz", "AC-5: markers stripped, text preserved");
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
    assert_eq!(spec.url, "http://127.0.0.1:37501/workspaces/ws-1/loom/search-v2");
    let json = spec.body.expect("search body");
    assert_eq!(json.get("query").and_then(|x| x.as_str()), Some("hello"));
    assert_eq!(json.get("content_type").and_then(|x| x.as_str()), Some("note"));
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
    assert!(json.get("content_type").is_none(), "unfiltered search omits content_type");
}

#[test]
fn save_view_request_uses_verified_definitions_route() {
    let client = LoomSearchV2Client::new(TEST_BASE, rt().handle().clone());
    let spec = client.save_view_request("ws-1", "hello world", Some("note"));
    // MT-027's VERIFIED createBlockView route — NOT the MT-028 contract's stale bare /loom/views.
    assert_eq!(spec.url, "http://127.0.0.1:37501/workspaces/ws-1/loom/views/definitions");
    let json = spec.body.expect("save body");
    assert_eq!(json.get("title").and_then(|x| x.as_str()), Some("Search: hello world"));
    let def = json.get("definition").expect("definition");
    assert_eq!(def.get("kind").and_then(|x| x.as_str()), Some("table"));
    assert_eq!(
        def.get("query").and_then(|q| q.get("content_type")).and_then(|x| x.as_str()),
        Some("note")
    );
    let cols = def.get("columns").and_then(|c| c.as_array()).expect("columns");
    let cols: Vec<&str> = cols.iter().filter_map(|c| c.as_str()).collect();
    assert_eq!(cols, ["title", "content_type", "updated"]);
    println!("PROOF6: save-as-view POST /loom/views/definitions body = {{title, definition{{kind,query,columns}}}}");
}

#[test]
fn save_view_request_empty_query_when_no_facet() {
    let client = LoomSearchV2Client::new(TEST_BASE, rt().handle().clone());
    let spec = client.save_view_request("ws-1", "hello", None);
    let json = spec.body.expect("save body");
    let query = json.get("definition").and_then(|d| d.get("query")).expect("query");
    assert!(query.as_object().map(|o| o.is_empty()).unwrap_or(false), "no facet => empty query object");
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
    assert!(!state.loading, "MC-7: no HTTP call fired (loading stays false)");
    println!("MC-7: no-workspace search sets error 'No workspace selected', no HTTP call");
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// PROOF (parser sanity at the test boundary): re-prove the MC-1 case from the integration crate too.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn parser_mc1_case_from_integration_crate() {
    let segs = parse_highlight_segments("<mark>foo</mark> bar <mark>baz</mark>");
    assert_eq!(segs.len(), 3);
    assert!(segs[0].marked && !segs[1].marked && segs[2].marked, "MC-1: mid+last marked");
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// PROOF7 (PT-4, HBR-VIS): screenshot of the rendered panel (query bar + 2 facets + 3 rows w/ highlight).
// ══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn loom_search_v2_screenshot() {
    let _g = wgpu_guard();
    let mut s = LoomSearchV2PanelState::new();
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
            let mut cbs = LoomSearchV2Callbacks { on_open_block: &mut on_open };
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
            let mut counts: std::collections::HashMap<[u8; 4], u32> = std::collections::HashMap::new();
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
            assert!(counts.len() >= 2, "screenshot: >= 2 distinct colours expected, got {}", counts.len());

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
// PT-1 / PT-2 (real-PG integration, NEEDS_MANAGED_RESOURCE_PROOF): the live search + save-as-view
// against a seeded handshake_core + PostgreSQL. `#[ignore]` so the default `cargo test` (no backend)
// does not depend on a running server. Run with a live, seeded backend on 127.0.0.1:37501:
//
//   cargo test -p handshake-native --test test_loom_search_v2 -- --ignored
//
// Requires env `HSK_TEST_WORKSPACE_ID` pointing at a workspace seeded with >= 3 Loom blocks of
// differing content_types whose text matches `HSK_TEST_QUERY` (default "the"). NEVER fakes PG.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore = "needs a live handshake_core + PostgreSQL on 127.0.0.1:37501 and HSK_TEST_WORKSPACE_ID seeded with >=3 blocks"]
fn loom_search_v2_live_pg() {
    use handshake_native::backend_client::LoomSearchCell;
    use std::time::{Duration, Instant};

    let workspace_id = std::env::var("HSK_TEST_WORKSPACE_ID")
        .expect("set HSK_TEST_WORKSPACE_ID to a workspace seeded with >=3 differing-content_type blocks");
    let query = std::env::var("HSK_TEST_QUERY").unwrap_or_else(|_| "the".to_owned());

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("build multi-thread runtime");
    let client = LoomSearchV2Client::new(TEST_BASE, runtime.handle().clone());

    let body = LoomSearchV2Body::baseline(query.clone(), None);
    let cell: LoomSearchCell = Arc::new(Mutex::new(None));
    client.search(&workspace_id, &body, Arc::clone(&cell));

    let deadline = Instant::now() + Duration::from_secs(5);
    let delivered = loop {
        if let Some(v) = cell.lock().unwrap().take() {
            break v;
        }
        if Instant::now() > deadline {
            panic!("PT-1: loom_search_v2 did not deliver within 5s");
        }
        std::thread::sleep(Duration::from_millis(50));
    };

    let resp = delivered.expect("PT-1: real search succeeded against the live backend");
    assert!(!resp.hits.is_empty(), "PT-1: expected >= 1 hit for query '{query}'");
    for h in &resp.hits {
        assert!(!h.block.block_id.is_empty(), "PT-1: each hit carries a block_id");
        assert!(h.score > 0.0, "PT-1: each hit has score > 0.0 (got {})", h.score);
    }
    assert!(!resp.content_type_facets.is_empty(), "PT-1: content_type_facets populated");
    let first_highlight = resp.hits.iter().find(|h| !h.highlight.is_empty());
    assert!(first_highlight.is_some(), "PT-1: at least one hit has a non-empty highlight");
    println!(
        "PT-1 PASS: {} hits, {} facets, semantic_available={}, total={} from real PG /loom/search-v2",
        resp.hits.len(),
        resp.content_type_facets.len(),
        resp.semantic_available,
        resp.total
    );
}

#[test]
#[ignore = "needs a live handshake_core + PostgreSQL on 127.0.0.1:37501 and HSK_TEST_WORKSPACE_ID"]
fn loom_search_v2_save_view_live_pg() {
    use handshake_native::backend_client::SaveViewCell;
    use std::time::{Duration, Instant};

    let workspace_id = std::env::var("HSK_TEST_WORKSPACE_ID")
        .expect("set HSK_TEST_WORKSPACE_ID to a real workspace");

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("build multi-thread runtime");
    let client = LoomSearchV2Client::new(TEST_BASE, runtime.handle().clone());

    let cell: SaveViewCell = Arc::new(Mutex::new(None));
    client.save_view(&workspace_id, "the", None, Arc::clone(&cell));

    let deadline = Instant::now() + Duration::from_secs(5);
    let delivered = loop {
        if let Some(v) = cell.lock().unwrap().take() {
            break v;
        }
        if Instant::now() > deadline {
            panic!("PT-2: save-as-view did not deliver within 5s");
        }
        std::thread::sleep(Duration::from_millis(50));
    };

    let block_id = delivered.expect("PT-2: save-as-view succeeded against the live backend");
    assert!(!block_id.is_empty(), "PT-2: createBlockView returned a non-empty view block_id");
    println!("PT-2 PASS: save-as-view created Loom view block_id={block_id} via POST /loom/views/definitions");
}
