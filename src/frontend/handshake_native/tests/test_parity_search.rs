//! WP-KERNEL-012 MT-044 — E8 Parity Proof Suite, cluster E4 (Search surfaces). Features #37-#43.
//!
//! ## CTRL-2: every proof exercises a REAL native impl by its fully-qualified Rust path
//!
//! Each E4 feature has TWO proofs:
//!
//!  1. `parity_<feature>_native` — a NON-ignored proof that runs IN-PROCESS (no PostgreSQL). It drives
//!     the REAL native search code: the `backend_client::LoomSearchV2Client` request builders +
//!     `LoomSearchV2Body::baseline`, the `loom_search_v2` panel consume path (facet ordering, `<mark>`
//!     highlight parsing, semantic status), the `find_in_files` regex match engine, and the
//!     `quick_switcher` recents-first ordering — asserting on the NATIVE output. These PASS today with
//!     no backend.
//!  2. `parity_<feature>` — the `#[ignore = "requires_pg"]` live round-trip. It calls the SAME native
//!     builder/consume path (CTRL-2 for the manifest `proof_fn`), then exercises the seeded
//!     handshake_core loom/search-v2 (+ quick-switcher / views) route. Gated `requires_pg`; with no env
//!     + no backend it panics with a `requires_pg` message, never fake-passes.
//!
//! There is NO sqlite, NO in-process backend substitute, and NO hard-coded result: the native half runs
//! the ported search code and the live half runs real PostgreSQL behind handshake_core.

mod parity_manifest_support;
mod pg_proof_support;

use std::collections::BTreeMap;

use parity_manifest_support::mark_pass;
use pg_proof_support::{require_live_backend, LiveBackend};

use handshake_native::backend_client::{
    HttpMethod, LoomGraphSearchHit as BcSearchHit, LoomSearchV2Body, LoomSearchV2Client,
    LoomSearchV2Response, RequestSpec,
};
use handshake_native::find_in_files::{compile_search_regex, hit_matches_regex, MatchOptions};
use handshake_native::loom_search_v2::{parse_highlight_segments, sorted_facets, LoomSearchV2PanelState};
use handshake_native::quick_switcher::{hit_key, ordered_results, LoomGraphSearchHit};

const NATIVE_BASE: &str = "http://native-proof";

fn native_rt() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("native proof runtime")
}

fn search_client() -> (tokio::runtime::Runtime, LoomSearchV2Client) {
    let rt = native_rt();
    let client = LoomSearchV2Client::new(NATIVE_BASE, rt.handle().clone());
    (rt, client)
}

/// The native search request the production panel sends for `query` + optional facet, built through
/// `LoomSearchV2Body::baseline` + `LoomSearchV2Client::search_request`.
fn search_spec(query: &str, content_type: Option<String>) -> RequestSpec {
    let (_rt, client) = search_client();
    client.search_request("ws-1", &LoomSearchV2Body::baseline(query, content_type))
}

/// A synthetic quick-switcher / find-in-files hit (all fields, defaults filled) for the pure native
/// ordering + match proofs.
fn hit(source_kind: &str, ref_id: &str, title: &str, excerpt: &str) -> LoomGraphSearchHit {
    LoomGraphSearchHit {
        result_kind: "loom_block".to_string(),
        source_kind: source_kind.to_string(),
        ref_id: ref_id.to_string(),
        title: title.to_string(),
        excerpt: excerpt.to_string(),
        block: serde_json::Value::Null,
        score: 0.0,
        metadata: serde_json::Value::Null,
    }
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// E4-37: full-text search — native LoomSearchV2Client::search_request + <mark> highlight parse
// ═════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn parity_full_text_search_native() {
    let spec = search_spec("the", None);
    assert_eq!(spec.method, HttpMethod::Post);
    assert!(spec.url.ends_with("/workspaces/ws-1/loom/search-v2"), "E4-37: native search URL (got {})", spec.url);
    let body = spec.body.expect("E4-37: search carries a body");
    assert_eq!(body["query"], "the", "E4-37: the FTS query is sent in the LoomSearchV2Body");
    assert!(body.get("content_type").is_none(), "E4-37: an unfaceted FTS omits content_type");
    // Consume path: the native highlight parser turns the backend `<mark>` runs into colored segments.
    let segs = parse_highlight_segments("<mark>the</mark> cat");
    assert!(segs.iter().any(|s| s.marked && s.text == "the"), "E4-37: the native highlighter marks the matched run");
    println!("E4-37 NATIVE PASS: LoomSearchV2Client::search_request built POST {} + native <mark> highlight parse", spec.url);
}

#[test]
#[ignore = "requires_pg: live handshake_core + PostgreSQL + HSK_TEST_WORKSPACE_ID + HSK_TEST_BLOCK_ID (POST /loom/search-v2)"]
fn parity_full_text_search() {
    let be: LiveBackend = require_live_backend();
    let block_id = be.require_block_id();
    let query = std::env::var("HSK_TEST_QUERY").unwrap_or_else(|_| "the".to_owned());
    // CTRL-2: build + send the native FTS request.
    let spec = search_spec(&query, None);
    let path = spec.url.strip_prefix(NATIVE_BASE).unwrap_or(&spec.url).replace("ws-1", &be.workspace_id);
    let resp = be.post_json(&path, &spec.body.clone().unwrap_or(serde_json::Value::Null));
    assert!(serde_json::to_string(&resp).unwrap().contains(&block_id), "E4-37: the indexed block {block_id} must appear in FTS hits");
    println!("E4-37 PASS: native FTS request surfaced block {block_id} for query '{query}'");
    mark_pass("E4-37");
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// E4-38: fuzzy search — native LoomSearchV2Body::baseline carries the typo'd query
// ═════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn parity_fuzzy_search_native() {
    let typo = inject_typo("parity");
    let spec = search_spec(&typo, None);
    let body = spec.body.expect("E4-38: fuzzy search carries a body");
    assert_eq!(body["query"], typo, "E4-38: the typo'd query is sent verbatim (pg_trgm fuzzy matches server-side)");
    println!("E4-38 NATIVE PASS: LoomSearchV2Body::baseline carried the typo'd query '{typo}'");
}

#[test]
#[ignore = "requires_pg: live handshake_core + PostgreSQL + HSK_TEST_WORKSPACE_ID + HSK_TEST_BLOCK_ID (POST /loom/search-v2 fuzzy)"]
fn parity_fuzzy_search() {
    let be = require_live_backend();
    let block_id = be.require_block_id();
    let base_query = std::env::var("HSK_TEST_QUERY").unwrap_or_else(|_| "parity".to_owned());
    let typo_query = inject_typo(&base_query);
    let spec = search_spec(&typo_query, None);
    let path = spec.url.strip_prefix(NATIVE_BASE).unwrap_or(&spec.url).replace("ws-1", &be.workspace_id);
    let resp = be.post_json(&path, &spec.body.clone().unwrap_or(serde_json::Value::Null));
    assert!(serde_json::to_string(&resp).unwrap().contains(&block_id), "E4-38: fuzzy search for the typo'd query '{typo_query}' must still surface block {block_id}");
    println!("E4-38 PASS: native fuzzy request surfaced block {block_id} despite the typo '{typo_query}'");
    mark_pass("E4-38");
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// E4-39: semantic search (pgvector) — native panel semantic status consume
// ═════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn parity_semantic_search_native() {
    // The native panel reads `semantic_available` HONESTLY: it shows "(semantic on)" ONLY when the
    // pgvector modality contributed, else "(keyword/fuzzy only)". Build a response with semantic on and
    // confirm the native status consume path (LoomSearchV2PanelState::status_text) reports it.
    let mut state = LoomSearchV2PanelState::new();
    state.response = Some(LoomSearchV2Response { hits: vec![], content_type_facets: BTreeMap::new(), semantic_available: true, total: 3 });
    assert_eq!(state.status_text(), "3 results (semantic on)", "E4-39: the native panel reports semantic ON when pgvector contributed");
    state.response = Some(LoomSearchV2Response { hits: vec![], content_type_facets: BTreeMap::new(), semantic_available: false, total: 1 });
    assert_eq!(state.status_text(), "1 result (keyword/fuzzy only)", "E4-39: the native panel NEVER claims semantic when it is off");
    println!("E4-39 NATIVE PASS: LoomSearchV2PanelState::status_text honestly reports semantic on/off");
}

#[test]
#[ignore = "requires_pg: live handshake_core + PostgreSQL + pgvector + embedding model + mt250 fixture + HSK_TEST_WORKSPACE_ID (POST /loom/search-v2, assert semantic_available)"]
fn parity_semantic_search() {
    let be = require_live_backend();
    let query = std::env::var("HSK_TEST_QUERY").unwrap_or_else(|_| "knowledge graph".to_owned());
    // CTRL-2: build + send the native search request; parse the response through the native model + panel.
    let spec = search_spec(&query, None);
    let path = spec.url.strip_prefix(NATIVE_BASE).unwrap_or(&spec.url).replace("ws-1", &be.workspace_id);
    let resp_json = be.post_json(&path, &spec.body.clone().unwrap_or(serde_json::Value::Null));
    let resp: LoomSearchV2Response = serde_json::from_value(resp_json).expect("E4-39: response deserializes through the native LoomSearchV2Response");
    assert!(resp.semantic_available, "E4-39: the pgvector path must actually contribute (semantic_available=true); configure the model + mt250 fixture/pgvector extension");
    let mut state = LoomSearchV2PanelState::new();
    let hit_count = resp.hits.len();
    state.response = Some(resp);
    assert!(state.status_text().contains("semantic on"), "E4-39: the native panel confirms semantic ON");
    assert!(hit_count > 0, "E4-39: semantic search (pgvector) must return a non-empty hits list (got 0)");
    println!("E4-39 PASS: semantic search (semantic_available=true) returned {hit_count} hits — non-empty");
    mark_pass("E4-39");
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// E4-40: faceted filter — native content_type facet in the body + native facet ordering
// ═════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn parity_faceted_filter_native() {
    let spec = search_spec("", Some("note".to_string()));
    let body = spec.body.expect("E4-40: faceted search carries a body");
    assert_eq!(body["content_type"], "note", "E4-40: the active facet is sent as the content_type filter");
    // Consume path: the native panel sorts facets by count DESC then name ASC.
    let mut facets = BTreeMap::new();
    facets.insert("note".to_string(), 2i64);
    facets.insert("code".to_string(), 5i64);
    facets.insert("image".to_string(), 2i64);
    let resp = LoomSearchV2Response { hits: vec![], content_type_facets: facets, semantic_available: false, total: 9 };
    let sorted = sorted_facets(&resp);
    assert_eq!(sorted[0], ("code".to_string(), 5), "E4-40: the native facet ordering is count-desc (code=5 first)");
    println!("E4-40 NATIVE PASS: native content_type facet in the body + native count-desc facet ordering");
}

#[test]
#[ignore = "requires_pg: live handshake_core + PostgreSQL + HSK_TEST_WORKSPACE_ID (POST /loom/search-v2 facet)"]
fn parity_faceted_filter() {
    let be = require_live_backend();
    let content_type = std::env::var("HSK_TEST_CONTENT_TYPE").unwrap_or_else(|_| "note".to_owned());
    // CTRL-2: build + send the native faceted request.
    let spec = search_spec("", Some(content_type.clone()));
    let path = spec.url.strip_prefix(NATIVE_BASE).unwrap_or(&spec.url).replace("ws-1", &be.workspace_id);
    let resp = be.post_json(&path, &spec.body.clone().unwrap_or(serde_json::Value::Null));
    let hits = resp.get("hits").and_then(|h| h.as_array()).cloned().unwrap_or_default();
    assert!(!hits.is_empty(), "E4-40: the faceted search must return >= 1 hit");
    for hit in &hits {
        let ct = hit
            .get("content_type")
            .and_then(|c| c.as_str())
            .or_else(|| hit.get("block").and_then(|b| b.get("content_type")).and_then(|c| c.as_str()))
            .unwrap_or("");
        assert_eq!(ct, content_type, "E4-40: every faceted hit must match content_type '{content_type}'");
    }
    println!("E4-40 PASS: native faceted request -> {} hits all match content_type '{content_type}'", hits.len());
    mark_pass("E4-40");
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// E4-41: save-results-as-view — native LoomSearchV2Client::save_view_request
// ═════════════════════════════════════════════════════════════════════════════════════════════════

fn save_view_spec(query: &str, content_type: Option<&str>) -> RequestSpec {
    let (_rt, client) = search_client();
    client.save_view_request("ws-1", query, content_type)
}

#[test]
fn parity_save_results_as_view_native() {
    let spec = save_view_spec("cats", Some("annotated_file"));
    assert_eq!(spec.method, HttpMethod::Post);
    assert!(spec.url.ends_with("/workspaces/ws-1/loom/views/definitions"), "E4-41: native save-view URL (got {})", spec.url);
    let body = spec.body.expect("E4-41: save-view carries a body");
    assert_eq!(body["definition"]["kind"], "table", "E4-41: the saved search is born as a table view_def");
    assert_eq!(body["definition"]["query"]["content_type"], "annotated_file", "E4-41: the search facet is embedded in the saved view query");
    println!("E4-41 NATIVE PASS: LoomSearchV2Client::save_view_request built a table view_def embedding the query");
}

#[test]
#[ignore = "requires_pg: live handshake_core + PostgreSQL + HSK_TEST_WORKSPACE_ID (POST /loom/views/definitions)"]
fn parity_save_results_as_view() {
    let be = require_live_backend();
    let saved_facet = "annotated_file";
    // CTRL-2: build + send the native save-view request.
    let spec = save_view_spec("parity-e4-41-saved-search", Some(saved_facet));
    let path = spec.url.strip_prefix(NATIVE_BASE).unwrap_or(&spec.url).replace("ws-1", &be.workspace_id);
    let view = be.post_json(&path, &spec.body.clone().unwrap_or(serde_json::Value::Null));
    let s = serde_json::to_string(&view).unwrap();
    assert!(s.contains("view_def"), "E4-41: the saved view block must be content_type='view_def' (got {s})");
    assert!(s.contains(saved_facet), "E4-41: the saved view must embed the query (facet '{saved_facet}')");
    println!("E4-41 PASS: native save-view request -> search saved as a view_def block with the query embedded");
    mark_pass("E4-41");
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// E4-42: find-in-files — native find_in_files regex engine matches across 3 files
// ═════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn parity_find_in_files_native() {
    let needle = "PARITY_FIND_MARKER";
    // Three file-class hits each carrying the needle. The native find_in_files engine compiles the query
    // ONCE (regex-escaped literal, RISK-8) and matches every hit's title\nexcerpt haystack.
    let bc_hit = |ref_id: &str, title: &str, excerpt: String| BcSearchHit {
        source_kind: "file".to_string(),
        result_kind: "loom_block".to_string(),
        ref_id: ref_id.to_string(),
        title: title.to_string(),
        excerpt,
        metadata: serde_json::Value::Null,
        block: None,
    };
    let files = [
        bc_hit("src/a.rs", "a.rs", format!("let a = {needle};")),
        bc_hit("src/b.rs", "b.rs", format!("fn b() {{ {needle} }}")),
        bc_hit("src/c.rs", "c.rs", format!("// {needle} here")),
    ];
    let opts = MatchOptions { is_regex: false, ..MatchOptions::default() };
    let regex = compile_search_regex(needle, opts).expect("E4-42: the native find engine compiles the query");
    let matched: Vec<&str> = files
        .iter()
        .filter(|&h| hit_matches_regex(h, &regex, opts))
        .map(|h| h.ref_id.as_str())
        .collect();
    assert_eq!(matched.len(), 3, "E4-42: the native find-in-files engine must match all 3 files (got {matched:?})");
    println!("E4-42 NATIVE PASS: find_in_files::compile_search_regex + hit_matches_regex surfaced 3 files for '{needle}'");
}

#[test]
#[ignore = "requires_pg: live handshake_core + PostgreSQL + HSK_TEST_WORKSPACE_ID (find-in-files across 3 files)"]
fn parity_find_in_files() {
    let be = require_live_backend();
    let needle = std::env::var("HSK_TEST_FIND_STRING").unwrap_or_else(|_| "PARITY_FIND_MARKER".to_owned());
    // CTRL-2: build + send the native file-faceted search request.
    let spec = search_spec(&needle, Some("file".to_string()));
    let path = spec.url.strip_prefix(NATIVE_BASE).unwrap_or(&spec.url).replace("ws-1", &be.workspace_id);
    let resp = be.post_json(&path, &spec.body.clone().unwrap_or(serde_json::Value::Null));
    let hits = resp.get("hits").and_then(|h| h.as_array()).cloned().unwrap_or_default();
    let distinct_paths: std::collections::HashSet<String> = hits
        .iter()
        .filter_map(|h| {
            h.get("path")
                .and_then(|p| p.as_str())
                .or_else(|| h.get("file_path").and_then(|p| p.as_str()))
                .or_else(|| h.get("block").and_then(|b| b.get("path")).and_then(|p| p.as_str()))
                .map(|s| s.to_owned())
        })
        .collect();
    assert!(distinct_paths.len() >= 3, "E4-42: find-in-files for '{needle}' must surface >= 3 distinct file paths (got {})", distinct_paths.len());
    println!("E4-42 PASS: native file-search request surfaced {} distinct file paths for '{needle}'", distinct_paths.len());
    mark_pass("E4-42");
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// E4-43: quick-switcher — native quick_switcher::ordered_results (recents-first)
// ═════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn parity_quick_switcher_native() {
    // Three hits; a recent selection of the 2nd hit must rank it FIRST (the native recents-first order).
    let hits = [
        hit("loom_block", "blk-1", "Alpha", ""),
        hit("loom_block", "blk-2", "Beta", ""),
        hit("loom_block", "blk-3", "Gamma", ""),
    ];
    let recent_key = hit_key(&hits[1]);
    assert_eq!(recent_key, "loom_block:blk-2", "E4-43: the native recents key is source_kind:ref_id");
    let ordered = ordered_results(&hits, std::slice::from_ref(&recent_key));
    assert_eq!(ordered[0].ref_id, "blk-2", "E4-43: the recorded recent surfaces FIRST in the native quick-switcher order");
    println!("E4-43 NATIVE PASS: quick_switcher::ordered_results ranked the recent block first");
}

#[test]
#[ignore = "requires_pg: live handshake_core + PostgreSQL + HSK_TEST_WORKSPACE_ID + HSK_TEST_QS_BLOCK_ID (GET /loom/quick-switcher/recents)"]
fn parity_quick_switcher() {
    let be = require_live_backend();
    let block_id = std::env::var("HSK_TEST_QS_BLOCK_ID")
        .or_else(|_| std::env::var("HSK_TEST_BLOCK_ID"))
        .expect("E4-43 requires_pg: set HSK_TEST_QS_BLOCK_ID (or HSK_TEST_BLOCK_ID) to a real block id");
    // CTRL-2: prove the native recents-first ordering surfaces the recorded block.
    let recorded = hit("loom_block", &block_id, "parity-e4-43", "");
    let recent_key = hit_key(&recorded);
    let ordered = ordered_results(std::slice::from_ref(&recorded), std::slice::from_ref(&recent_key));
    assert_eq!(ordered[0].ref_id, block_id, "E4-43: the native quick-switcher ranks the recorded block first");
    // Live round-trip: record a recent selection, then GET the recents and confirm the block surfaces.
    be.post_json(
        &format!("/workspaces/{}/loom/quick-switcher/recents", be.workspace_id),
        &serde_json::json!({ "result_kind": "loom_block", "source_kind": "loom_block", "ref_id": block_id, "title": "parity-e4-43" }),
    );
    let resp = be.get_json(&format!("/workspaces/{}/loom/quick-switcher/recents", be.workspace_id));
    let recents = resp.as_array().cloned().unwrap_or_default();
    assert!(serde_json::to_string(&resp).unwrap().contains(&block_id), "E4-43: the quick-switcher recents must surface the recorded block {block_id} (got {} recents)", recents.len());
    println!("E4-43 PASS: native recents ordering + live quick-switcher recents surfaced block {block_id}");
    mark_pass("E4-43");
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// helper (pure)
// ═════════════════════════════════════════════════════════════════════════════════════════════════

/// Inject a single-character typo into a query (swap the last char) for the fuzzy-search proof.
fn inject_typo(q: &str) -> String {
    let mut chars: Vec<char> = q.chars().collect();
    if let Some(last) = chars.last_mut() {
        *last = if *last == 'x' { 'y' } else { 'x' };
    }
    chars.into_iter().collect()
}
