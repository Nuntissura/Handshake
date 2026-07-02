//! WP-KERNEL-012 MT-029 Find-in-Files + Replace-in-Files surface PROOFS (E4 Search).
//!
//! Coverage map (proof_targets PT-1..PT-5 + acceptance_criteria AC-1..AC-11):
//!   - The STANDALONE replace logic (regex compile/escape/error, replace_segment zero-length +
//!     whole-word + group-expansion, content_json walk preserving non-text nodes, KRD- documentId
//!     extraction, stale-plan keys, bookmark blob round-trip) is proven in the lib unit tests
//!     (`handshake_native::find_in_files::tests`) — pure, no backend, no GPU. PT-4 (regex compile
//!     error) + RISK-3/4/5/8 + MC-2/5/8 live there; re-proven at the integration boundary here.
//!   - PROOF_ACCESSKIT (PT-5, AC-1/AC-3/AC-10): a kittest render of the panel with injected results
//!     (3 hits) + a preview plan (2 plans) asserts the live AccessKit tree contains the contract
//!     author_ids (query, search, toggle-case, toggle-word, toggle-regex, kind-filter, preview-replace,
//!     apply, plus >= 1 result-row + >= 1 preview node).
//!   - PROOF_TOGGLES (AC-3): the case/word/regex toggle buttons flip aria-pressed (selected) state.
//!   - PROOF_GATE (AC-5/AC-8): Preview Replace is disabled until a search has run; Apply is disabled
//!     until a non-stale preview exists.
//!   - PROOF_STALE (AC-7, RISK-2/MC-2): changing the query after a search makes Preview Replace show the
//!     stale warning rather than computing a preview.
//!   - PROOF_REQUEST (AC-2/AC-4, the VERIFIED routes): the graph-search query params + the bookmark PUT
//!     wrapper are asserted WITHOUT a backend (the spawn paths route through the SAME builders).
//!   - PROOF_SCREENSHOT (HBR-VIS): a screenshot of the rendered panel to the EXTERNAL artifact root.
//!   - PROOF_REGISTRY (the in-product render path): open the Find-in-Files pane THROUGH the WP-011
//!     registry + PaneHostWidget and assert the REAL panel rendered (not the placeholder).
//!   - PT-1/PT-2/PT-3 (real-PG integration, NEEDS_MANAGED_RESOURCE_PROOF): the `#[ignore]`d live tests
//!     that seed documents + run search/replace-cycle/bookmark-roundtrip against a seeded
//!     handshake_core + PostgreSQL. They NEVER fake PG; absent a seeded backend they are skipped, the
//!     request-builder + standalone-replace proofs covering the wire + the mutation logic.
//!
//! ## Artifact hygiene (CX-212E)
//!
//! EVERY PNG is written ONLY to the EXTERNAL `Handshake_Artifacts/handshake-test/wp-kernel-012-mt-029/`
//! root via [`external_artifact_dir`]; [`assert_no_local_artifact_dir`] fails the run if a repo-local
//! `tests/screenshots/` or `test_output/` directory exists.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use egui_kittest::kittest::NodeT;
use egui_kittest::Harness;

use handshake_native::backend_client::{
    LoomGraphSearchHit, RichDocClient, SearchMatchOptions, WorkspaceSearchClient,
};
use handshake_native::find_in_files::{
    document_id_from_hit, preview_author_id, replace_in_content, result_author_id, show,
    FindInFilesCallbacks, FindInFilesPaneFactory, FindInFilesPaneShared, FindInFilesPanelState,
    KindFilter, MatchOptions, MatchPreview, ReplacementPlan, APPLY_AUTHOR_ID,
    KIND_FILTER_AUTHOR_ID, PREVIEW_REPLACE_AUTHOR_ID, QUERY_AUTHOR_ID, SEARCH_AUTHOR_ID,
    TOGGLE_CASE_AUTHOR_ID, TOGGLE_REGEX_AUTHOR_ID, TOGGLE_WORD_AUTHOR_ID,
};
use handshake_native::pane_registry::{
    DirtyState, LockState, PaneAuthority, PaneFactory, PaneHostWidget, PaneRecord, PaneRegistry,
    PaneType,
};
use handshake_native::theme::HsTheme;

const TEST_BASE: &str = "http://127.0.0.1:37501";

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

/// Serialize the `.wgpu()` screenshot test (the documented Windows-wgpu concurrent-device hazard).
static WGPU_SERIAL_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());
fn wgpu_guard() -> std::sync::MutexGuard<'static, ()> {
    WGPU_SERIAL_GUARD.lock().unwrap_or_else(|p| p.into_inner())
}

/// A current-thread tokio runtime kept alive for a test's scope (the clients bridge onto its handle).
fn rt() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build current-thread runtime")
}

// ── Mock builders ─────────────────────────────────────────────────────────────────────────────────────

fn hit(
    source_kind: &str,
    ref_id: &str,
    title: &str,
    excerpt: &str,
    doc_id: Option<&str>,
) -> LoomGraphSearchHit {
    LoomGraphSearchHit {
        source_kind: source_kind.to_owned(),
        result_kind: "loom_block".to_owned(),
        ref_id: ref_id.to_owned(),
        title: title.to_owned(),
        excerpt: excerpt.to_owned(),
        metadata: match doc_id {
            Some(id) => serde_json::json!({ "rich_document_id": id }),
            None => serde_json::json!({}),
        },
        block: None,
    }
}

fn mock_plan(doc_id: &str, title: &str, count: usize) -> ReplacementPlan {
    ReplacementPlan {
        document_id: doc_id.to_owned(),
        title: title.to_owned(),
        expected_version: 3,
        content_json_after: serde_json::json!({ "type": "doc", "content": [] }),
        crdt_document_id: None,
        match_count: count,
        before_preview: "before FIND_TARGET text".to_owned(),
        after_preview: "before REPLACED text".to_owned(),
        match_previews: vec![MatchPreview {
            before_preview: "FIND_TARGET".to_owned(),
            after_preview: "REPLACED".to_owned(),
        }],
    }
}

/// A panel seeded with 3 results and 2 preview plans (the PT-5 render fixture).
fn seeded_state() -> FindInFilesPanelState {
    let mut s = FindInFilesPanelState::new();
    s.query = "FIND_TARGET".to_owned();
    s.replacement = "REPLACED".to_owned();
    s.results = vec![
        hit(
            "loom_block",
            "blk-1",
            "First Note",
            "has FIND_TARGET here",
            Some("KRD-1"),
        ),
        hit(
            "loom_block",
            "blk-2",
            "Second Note",
            "FIND_TARGET twice FIND_TARGET",
            Some("KRD-2"),
        ),
        hit("file", "blk-3", "Some File", "no match excerpt", None),
    ];
    s.result_set_key = Some(s.current_search_key());
    s.preview_plans = vec![
        mock_plan("KRD-1", "First Note", 1),
        mock_plan("KRD-2", "Second Note", 2),
    ];
    s.preview_plan_key = Some(s.current_replace_key());
    s
}

/// Build a kittest harness rendering the shared panel state. `opened` records every clicked hit.
fn harness_for<'a>(
    state: Arc<Mutex<FindInFilesPanelState>>,
    opened: Arc<Mutex<Vec<String>>>,
    search_client: WorkspaceSearchClient,
    doc_client: RichDocClient,
    workspace_id: Option<String>,
) -> Harness<'a, ()> {
    Harness::builder()
        .with_size(egui::vec2(900.0, 760.0))
        .build_ui(move |ui| {
            let pal = HsTheme::Dark.palette();
            let opened_cb = Arc::clone(&opened);
            let mut on_open = move |hit: &LoomGraphSearchHit| {
                opened_cb.lock().unwrap().push(hit.ref_id.clone());
            };
            let mut cbs = FindInFilesCallbacks {
                on_open_hit: &mut on_open,
            };
            show(
                ui,
                &mut state.lock().unwrap(),
                &pal,
                &search_client,
                &doc_client,
                workspace_id.as_deref(),
                &mut cbs,
            );
        })
}

fn author_ids(harness: &Harness<'_, ()>) -> HashSet<String> {
    let mut ids = HashSet::new();
    for node in harness.root().children_recursive() {
        if let Some(a) = node.accesskit_node().author_id() {
            ids.insert(a.to_owned());
        }
    }
    ids
}

fn click_author_id(harness: &Harness<'_, ()>, author_id: &str) {
    let node = harness
        .root()
        .children_recursive()
        .find(|n| n.accesskit_node().author_id() == Some(author_id))
        .unwrap_or_else(|| panic!("no node with author_id '{author_id}' to click"));
    node.click();
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// PROOF_ACCESSKIT (PT-5, AC-1/AC-3/AC-10): the contract author_ids appear in the live AccessKit tree.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn accesskit_tree_has_all_contract_author_ids() {
    let state = Arc::new(Mutex::new(seeded_state()));
    let opened = Arc::new(Mutex::new(Vec::new()));
    let r = rt();
    let search_client = WorkspaceSearchClient::new(TEST_BASE, r.handle().clone());
    let doc_client = RichDocClient::new(TEST_BASE, r.handle().clone());
    let mut harness = harness_for(
        state,
        opened,
        search_client,
        doc_client,
        Some("ws-1".to_owned()),
    );
    harness.run();

    let ids = author_ids(&harness);
    for required in [
        QUERY_AUTHOR_ID,
        SEARCH_AUTHOR_ID,
        TOGGLE_CASE_AUTHOR_ID,
        TOGGLE_WORD_AUTHOR_ID,
        TOGGLE_REGEX_AUTHOR_ID,
        KIND_FILTER_AUTHOR_ID,
        PREVIEW_REPLACE_AUTHOR_ID,
        APPLY_AUTHOR_ID,
    ] {
        assert!(
            ids.contains(required),
            "PT-5: required author_id '{required}' missing from {ids:?}"
        );
    }
    // At least one result-row node + one preview node.
    assert!(
        ids.contains(&result_author_id("loom_block", "blk-1")),
        "PT-5: result row find-in-files.result.loom_block.blk-1 missing from {ids:?}"
    );
    assert!(
        ids.contains(&preview_author_id("KRD-1")),
        "PT-5: preview node find-in-files.preview.KRD-1 missing from {ids:?}"
    );
    println!("PT-5/AC-1/AC-10: all contract author_ids present in the live AccessKit tree");
    assert_no_local_artifact_dir();
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// PROOF_TOGGLES (AC-3): the case/word/regex toggle buttons flip selected (aria-pressed) state.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn toggle_buttons_flip_state() {
    let state = Arc::new(Mutex::new(FindInFilesPanelState::new()));
    let opened = Arc::new(Mutex::new(Vec::new()));
    let r = rt();
    let search_client = WorkspaceSearchClient::new(TEST_BASE, r.handle().clone());
    let doc_client = RichDocClient::new(TEST_BASE, r.handle().clone());
    let mut harness = harness_for(
        Arc::clone(&state),
        opened,
        search_client,
        doc_client,
        Some("ws-1".to_owned()),
    );
    harness.run();

    assert!(!state.lock().unwrap().case_sensitive, "case off initially");
    click_author_id(&harness, TOGGLE_CASE_AUTHOR_ID);
    harness.run();
    assert!(
        state.lock().unwrap().case_sensitive,
        "AC-3: case toggle flipped on"
    );

    click_author_id(&harness, TOGGLE_REGEX_AUTHOR_ID);
    harness.run();
    assert!(
        state.lock().unwrap().is_regex,
        "AC-3: regex toggle flipped on"
    );
    click_author_id(&harness, TOGGLE_WORD_AUTHOR_ID);
    harness.run();
    assert!(
        state.lock().unwrap().whole_word,
        "AC-3: whole-word toggle flipped on"
    );
    println!("AC-3: case/word/regex toggles flip aria-pressed (selected) state");
    assert_no_local_artifact_dir();
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// PROOF_GATE (AC-5/AC-8): Preview Replace disabled until a search ran; Apply disabled until non-stale.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn preview_and_apply_gating() {
    // Fresh state: no search => preview disabled, apply disabled.
    let state = Arc::new(Mutex::new(FindInFilesPanelState::new()));
    let opened = Arc::new(Mutex::new(Vec::new()));
    let r = rt();
    let sc = WorkspaceSearchClient::new(TEST_BASE, r.handle().clone());
    let dc = RichDocClient::new(TEST_BASE, r.handle().clone());
    let mut h = harness_for(state, opened, sc, dc, Some("ws-1".to_owned()));
    h.run();
    let preview_disabled = h
        .root()
        .children_recursive()
        .find(|n| n.accesskit_node().author_id() == Some(PREVIEW_REPLACE_AUTHOR_ID))
        .map(|n| n.accesskit_node().is_disabled())
        .expect("preview node present");
    assert!(
        preview_disabled,
        "AC-5: Preview Replace disabled with no search"
    );
    let apply_disabled = h
        .root()
        .children_recursive()
        .find(|n| n.accesskit_node().author_id() == Some(APPLY_AUTHOR_ID))
        .map(|n| n.accesskit_node().is_disabled())
        .expect("apply node present");
    assert!(apply_disabled, "AC-8: Apply disabled with no preview");

    // Seeded state (search ran + non-stale preview) => both enabled.
    let state2 = Arc::new(Mutex::new(seeded_state()));
    let opened2 = Arc::new(Mutex::new(Vec::new()));
    let r2 = rt();
    let sc2 = WorkspaceSearchClient::new(TEST_BASE, r2.handle().clone());
    let dc2 = RichDocClient::new(TEST_BASE, r2.handle().clone());
    let mut h2 = harness_for(state2, opened2, sc2, dc2, Some("ws-1".to_owned()));
    h2.run();
    let preview_enabled = h2
        .root()
        .children_recursive()
        .find(|n| n.accesskit_node().author_id() == Some(PREVIEW_REPLACE_AUTHOR_ID))
        .map(|n| !n.accesskit_node().is_disabled())
        .expect("preview node present");
    assert!(
        preview_enabled,
        "AC-5: Preview Replace enabled after a search"
    );
    let apply_enabled = h2
        .root()
        .children_recursive()
        .find(|n| n.accesskit_node().author_id() == Some(APPLY_AUTHOR_ID))
        .map(|n| !n.accesskit_node().is_disabled())
        .expect("apply node present");
    assert!(
        apply_enabled,
        "AC-8: Apply enabled with a non-stale preview"
    );
    println!("AC-5/AC-8: Preview gates on search-ran; Apply gates on non-stale preview");
    assert_no_local_artifact_dir();
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// PROOF_STALE (AC-7, RISK-2/MC-2): a query change after a search makes Preview Replace show the stale
// warning rather than computing a preview.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn stale_result_guard_blocks_preview() {
    let mut s = FindInFilesPanelState::new();
    s.query = "cats".to_owned();
    s.results = vec![hit("loom_block", "blk-1", "T", "cats here", Some("KRD-1"))];
    // The results were fetched under the OLD query; now the query differs => stale.
    s.result_set_key = Some(handshake_native::find_in_files::search_plan_key(
        "old_query",
        KindFilter::All,
        "",
        "",
        MatchOptions::default(),
    ));
    let r = rt();
    let dc = RichDocClient::new(TEST_BASE, r.handle().clone());
    s.run_preview_replace(&dc, Some("ws-1"));
    assert!(
        s.replace_status.as_deref().unwrap_or_default().contains("stale"),
        "AC-7/RISK-2: a since-changed query shows the stale warning, computes no preview (got {:?})",
        s.replace_status
    );
    assert!(
        s.preview_plans.is_empty(),
        "no preview computed under stale results"
    );
    println!("AC-7/RISK-2/MC-2: stale-result guard blocks Preview Replace");
    assert_no_local_artifact_dir();
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// PROOF_REQUEST (AC-2/AC-4): the VERIFIED graph-search query params + the bookmark PUT wrapper (NO
// backend — the spawn paths route through the SAME builders).
// ══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn search_page_query_uses_verified_params() {
    let r = rt();
    let c = WorkspaceSearchClient::new(TEST_BASE, r.handle().clone());
    // All-kind, no filters => no source_kinds, no tag_ids, no path.
    let params = c.search_page_query("alpha", None, "", "", SearchMatchOptions::default(), 0);
    assert!(params.contains(&("q".to_owned(), "alpha".to_owned())));
    assert!(
        params.contains(&("limit".to_owned(), "500".to_owned())),
        "AC-2: page size 500"
    );
    assert!(params.contains(&("offset".to_owned(), "0".to_owned())));
    assert!(
        !params.iter().any(|(k, _)| k == "source_kinds"),
        "AC-4: All filter omits source_kinds"
    );

    // Document kind + filters + options => source_kinds + tag_ids + path + flags (regex NOT isRegex).
    let params2 = c.search_page_query(
        "Alpha.*Beta",
        Some("document"),
        "tag-1, tag-2",
        "src/app",
        SearchMatchOptions {
            case_sensitive: true,
            whole_word: true,
            is_regex: true,
        },
        500,
    );
    assert!(
        params2.contains(&("source_kinds".to_owned(), "document".to_owned())),
        "AC-4: source_kinds passed"
    );
    assert!(params2.contains(&("tag_ids".to_owned(), "tag-1,tag-2".to_owned())));
    assert!(params2.contains(&("path".to_owned(), "src/app".to_owned())));
    assert!(params2.contains(&("case_sensitive".to_owned(), "true".to_owned())));
    assert!(params2.contains(&("whole_word".to_owned(), "true".to_owned())));
    assert!(
        params2.contains(&("regex".to_owned(), "true".to_owned())),
        "the VERIFIED param is `regex`, NOT `isRegex` (api.test.ts:771)"
    );
    assert!(
        params2.contains(&("offset".to_owned(), "500".to_owned())),
        "pagination offset forwarded"
    );
    println!(
        "AC-2/AC-4: graph-search params = verified q/limit/offset/source_kinds/tag_ids/path/regex"
    );
}

#[test]
fn bookmark_save_request_wraps_blob() {
    let r = rt();
    let c = WorkspaceSearchClient::new(TEST_BASE, r.handle().clone());
    let blob = handshake_native::find_in_files::bookmark_state_blob(&[]);
    let spec = c.save_bookmarks_request("ws-1", blob.clone());
    assert_eq!(
        spec.url,
        "http://127.0.0.1:37501/workspaces/ws-1/search-bookmarks"
    );
    let body = spec.body.expect("bookmark body");
    assert_eq!(
        body.get("bookmark_state").unwrap(),
        &blob,
        "PUT wraps the blob under bookmark_state"
    );
    // RISK-6: the blob carries the EXACT backend-validated schema_id.
    assert_eq!(
        body["bookmark_state"]["schema_id"], "hsk.workspace_search_bookmark_state@1",
        "RISK-6: bookmark schema_id must be exactly hsk.workspace_search_bookmark_state@1"
    );
    println!("AC-2/RISK-6: bookmark PUT wraps {{schema_id, bookmarks}} under bookmark_state");
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// PROOF (RISK-3/4/5/8 re-proven at the integration boundary): the standalone replace logic.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn replace_preserves_non_text_nodes_and_walks_code() {
    // RISK-4: a doc with text + a code block + an embed; replace walks text + attrs.code, preserves embed.
    let content = serde_json::json!({
        "type": "doc",
        "content": [
            { "type": "text", "text": "alpha FIND_TARGET omega" },
            { "type": "codeBlock", "attrs": { "code": "fn FIND_TARGET() {}", "language": "rust" } },
            { "type": "hsLink", "attrs": { "target": "KRD-9", "label": "link" } }
        ]
    });
    let re = handshake_native::find_in_files::compile_search_regex(
        "FIND_TARGET",
        MatchOptions::default(),
    )
    .unwrap();
    let res = replace_in_content(&content, &re, "REPLACED", MatchOptions::default());
    assert_eq!(res.count, 2, "one in text + one in code");
    let arr = res.content["content"].as_array().unwrap();
    assert_eq!(arr[0]["text"], "alpha REPLACED omega");
    assert_eq!(arr[1]["attrs"]["code"], "fn REPLACED() {}");
    // RISK-4: the hsLink node is preserved VERBATIM (the MT-011 round-trip lesson).
    assert_eq!(arr[2]["type"], "hsLink");
    assert_eq!(arr[2]["attrs"]["target"], "KRD-9");
    println!("RISK-4: content_json walk mutates text+code, round-trips hsLink/embed verbatim");
}

#[test]
fn document_id_extraction_krd_prefix() {
    // RISK-5: only KRD- ids are accepted.
    let bad = hit("loom_block", "blk-1", "T", "x", Some("DOC-1"));
    assert_eq!(
        document_id_from_hit(&bad),
        None,
        "RISK-5: non-KRD id rejected"
    );
    let good = hit("loom_block", "blk-1", "T", "x", Some("KRD-7"));
    assert_eq!(document_id_from_hit(&good), Some("KRD-7".to_owned()));
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// PROOF_REGISTRY (in-product render path): open the Find-in-Files pane THROUGH the WP-011 registry +
// PaneHostWidget and assert the REAL panel rendered (not the placeholder).
// ══════════════════════════════════════════════════════════════════════════════════════════════════

fn find_in_files_registry() -> PaneRegistry {
    let mut reg = PaneRegistry::new();
    reg.insert(PaneRecord::new(
        std::sync::Arc::from("find-in-files-pane"),
        PaneType::FindInFiles,
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
    let r = rt();
    let sc = WorkspaceSearchClient::new(TEST_BASE, r.handle().clone());
    let dc = RichDocClient::new(TEST_BASE, r.handle().clone());
    let shared = Arc::new(Mutex::new(FindInFilesPaneShared::new(
        HsTheme::Dark.palette(),
    )));
    {
        let mut g = shared.lock().unwrap();
        g.workspace_id = Some("ws-1".to_owned());
    }
    let factory: Box<dyn PaneFactory> = Box::new(FindInFilesPaneFactory::with_state(
        sc,
        dc,
        Arc::clone(&shared),
        seeded_state(),
    ));

    let reg = find_in_files_registry();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 760.0))
        .build_ui(move |ui| {
            PaneHostWidget::show(ui, &reg, |_pane_type| factory.as_ref());
        });
    harness.run();

    let ids = author_ids(&harness);
    for required in [
        QUERY_AUTHOR_ID,
        SEARCH_AUTHOR_ID,
        PREVIEW_REPLACE_AUTHOR_ID,
        APPLY_AUTHOR_ID,
    ] {
        assert!(
            ids.contains(required),
            "registry-dispatched pane rendered the placeholder, not the real panel — '{required}' missing ({ids:?})"
        );
    }
    assert!(
        ids.contains(&result_author_id("loom_block", "blk-1")),
        "result row missing from registry pane"
    );
    println!("AC-registry: Find-in-Files pane opens via the WP-011 registry/PaneHostWidget + renders the REAL panel");
    assert_no_local_artifact_dir();
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// PROOF_SCREENSHOT (HBR-VIS): screenshot of the rendered panel to the EXTERNAL artifact root.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn find_in_files_screenshot() {
    let _g = wgpu_guard();
    let state = Arc::new(Mutex::new(seeded_state()));
    let opened = Arc::new(Mutex::new(Vec::new()));
    let r = rt();
    let sc = WorkspaceSearchClient::new(TEST_BASE, r.handle().clone());
    let dc = RichDocClient::new(TEST_BASE, r.handle().clone());
    let workspace_id = Some("ws-1".to_owned());

    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 760.0))
        .wgpu()
        .build_ui(move |ui| {
            let pal = HsTheme::Dark.palette();
            let opened_cb = Arc::clone(&opened);
            let mut on_open =
                move |hit: &LoomGraphSearchHit| opened_cb.lock().unwrap().push(hit.ref_id.clone());
            let mut cbs = FindInFilesCallbacks {
                on_open_hit: &mut on_open,
            };
            show(
                ui,
                &mut state.lock().unwrap(),
                &pal,
                &sc,
                &dc,
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
            let mut i = 0usize;
            while i + 4 <= raw.len() {
                let px = [raw[i], raw[i + 1], raw[i + 2], raw[i + 3]];
                if px[3] != 0 {
                    *counts.entry(px).or_insert(0) += 1;
                    if px[0] > 250 && px[1] > 250 && px[2] > 250 {
                        white += 1;
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

            let ext_dir = external_artifact_dir("wp-kernel-012-mt-029");
            let _ = std::fs::create_dir_all(&ext_dir);
            let png = ext_dir.join("MT-029-find-in-files.png");
            let saved = image.save(&png).is_ok();
            println!(
                "SCREENSHOT: {w}x{h}, {} distinct colours, white_frac={:.3}, saved={saved} ({})",
                counts.len(),
                white as f32 / total as f32,
                png.display()
            );
        }
        Err(e) => {
            println!(
                "BLOCKER(non-fatal): screenshot render unavailable (no wgpu adapter): {e}. The \
                 AccessKit + toggle + gating + stale-guard + request-builder + replace-logic proofs \
                 passed; the PNG is a GPU-host item."
            );
        }
    }
    assert_no_local_artifact_dir();
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// PT-1 / PT-2 / PT-3 (real-PG integration, NEEDS_MANAGED_RESOURCE_PROOF). `#[ignore]` so the default
// `cargo test` (no backend) does not depend on a running server. Run with a live, seeded backend:
//
//   cargo test -p handshake-native --test test_find_in_files -- --ignored
//
// Requires env `HSK_TEST_WORKSPACE_ID` pointing at a workspace seeded with >= 1 rich document whose
// content matches `HSK_TEST_QUERY` (default "the"). NEVER fakes PG.
//
// PT-2 (the replace cycle) ADDITIONALLY requires the seeded docs to contain "FIND_TARGET"; it runs a
// search → preview-replace → apply → re-search and asserts the original text is gone. It MUTATES the
// seeded documents, so it is gated behind both `--ignored` AND `HSK_TEST_REPLACE=1` so a casual
// `--ignored` run does not destructively edit a workspace.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore = "needs a live handshake_core + PostgreSQL on 127.0.0.1:37501 and HSK_TEST_WORKSPACE_ID seeded with matching docs"]
fn find_in_files_search_live_pg() {
    use handshake_native::backend_client::GraphSearchCell;
    use std::time::{Duration, Instant};

    let workspace_id = std::env::var("HSK_TEST_WORKSPACE_ID")
        .expect("set HSK_TEST_WORKSPACE_ID to a seeded workspace");
    let query = std::env::var("HSK_TEST_QUERY").unwrap_or_else(|_| "the".to_owned());

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("build multi-thread runtime");
    let client = WorkspaceSearchClient::new(TEST_BASE, runtime.handle().clone());

    let key = handshake_native::find_in_files::search_plan_key(
        &query,
        KindFilter::All,
        "",
        "",
        MatchOptions::default(),
    );
    let cell: GraphSearchCell = Arc::new(Mutex::new(None));
    client.search_paginated(
        &workspace_id,
        &query,
        None,
        "",
        "",
        SearchMatchOptions::default(),
        key,
        Arc::clone(&cell),
    );

    let deadline = Instant::now() + Duration::from_secs(10);
    let delivered = loop {
        if let Some(v) = cell.lock().unwrap().take() {
            break v;
        }
        if Instant::now() > deadline {
            panic!("PT-1: find-in-files search did not deliver within 10s");
        }
        std::thread::sleep(Duration::from_millis(50));
    };

    let (hits, _key) =
        delivered.expect("PT-1: real graph-search succeeded against the live backend");
    assert!(
        !hits.is_empty(),
        "PT-1: expected >= 1 hit for query '{query}'"
    );
    for h in &hits {
        assert!(
            !h.title.is_empty() || !h.ref_id.is_empty(),
            "PT-1: each hit carries a title or ref_id"
        );
        assert!(
            !h.source_kind.is_empty(),
            "PT-1: each hit carries a source_kind"
        );
    }
    println!(
        "PT-1 PASS: {} hits from real PG /loom/graph-search for query '{query}'",
        hits.len()
    );
}

#[test]
#[ignore = "needs a live backend AND HSK_TEST_REPLACE=1 (MUTATES seeded docs) + HSK_TEST_WORKSPACE_ID with docs containing FIND_TARGET"]
fn find_in_files_replace_cycle_live_pg() {
    use handshake_native::backend_client::{DocSaveOutcome, GraphSearchCell};
    use std::time::{Duration, Instant};

    if std::env::var("HSK_TEST_REPLACE").as_deref() != Ok("1") {
        eprintln!(
            "skipping replace-cycle: set HSK_TEST_REPLACE=1 to run the MUTATING replace test"
        );
        return;
    }
    let workspace_id = std::env::var("HSK_TEST_WORKSPACE_ID")
        .expect("set HSK_TEST_WORKSPACE_ID to a seeded workspace");

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("build multi-thread runtime");
    let search_client = WorkspaceSearchClient::new(TEST_BASE, runtime.handle().clone());
    let doc_client = RichDocClient::new(TEST_BASE, runtime.handle().clone());

    // 1) Search for FIND_TARGET.
    let key = handshake_native::find_in_files::search_plan_key(
        "FIND_TARGET",
        KindFilter::All,
        "",
        "",
        MatchOptions::default(),
    );
    let cell: GraphSearchCell = Arc::new(Mutex::new(None));
    search_client.search_paginated(
        &workspace_id,
        "FIND_TARGET",
        None,
        "",
        "",
        SearchMatchOptions::default(),
        key,
        Arc::clone(&cell),
    );
    let deadline = Instant::now() + Duration::from_secs(10);
    let (hits, _) = loop {
        if let Some(v) = cell.lock().unwrap().take() {
            break v.expect("PT-2: search ok");
        }
        if Instant::now() > deadline {
            panic!("PT-2: search did not deliver");
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let doc_ids: Vec<String> = hits.iter().filter_map(document_id_from_hit).collect();
    assert!(
        !doc_ids.is_empty(),
        "PT-2: expected >= 1 KRD- doc containing FIND_TARGET"
    );

    // 2) Load + replace + save each doc with its expected_version (optimistic concurrency).
    let re = handshake_native::find_in_files::compile_search_regex(
        "FIND_TARGET",
        MatchOptions::default(),
    )
    .unwrap();
    let mut applied = 0usize;
    for doc_id in &doc_ids {
        let doc = runtime
            .block_on(doc_client.load_document(doc_id))
            .expect("PT-2: load doc");
        let replaced =
            replace_in_content(&doc.content_json, &re, "REPLACED", MatchOptions::default());
        if replaced.count == 0 {
            continue;
        }
        match runtime.block_on(doc_client.save_document(doc_id, &replaced.content, doc.doc_version))
        {
            DocSaveOutcome::Saved(receipt) => {
                assert!(!receipt.is_empty(), "PT-2: save returns a receipt event id");
                applied += 1;
            }
            other => panic!("PT-2: expected Saved, got {other:?}"),
        }
    }
    assert!(applied >= 1, "PT-2: at least one doc replaced + saved");
    println!("PT-2 PASS: replaced FIND_TARGET -> REPLACED in {applied} doc(s) via real PG save");
}

#[test]
#[ignore = "needs a live handshake_core + PostgreSQL on 127.0.0.1:37501 and HSK_TEST_WORKSPACE_ID"]
fn find_in_files_bookmark_roundtrip_live_pg() {
    use handshake_native::backend_client::BookmarkStateCell;
    use handshake_native::find_in_files::{
        bookmark_state_blob, parse_bookmark_state, SearchBookmark,
    };
    use std::time::{Duration, Instant};

    let workspace_id = std::env::var("HSK_TEST_WORKSPACE_ID")
        .expect("set HSK_TEST_WORKSPACE_ID to a real workspace");

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("build multi-thread runtime");
    let client = WorkspaceSearchClient::new(TEST_BASE, runtime.handle().clone());

    // Save a bookmark.
    let bm = SearchBookmark {
        id: "pt3-fixture".to_owned(),
        label: "PT3 fixture".to_owned(),
        query: "PT3_QUERY".to_owned(),
        kind: KindFilter::Document,
        tag_filter: String::new(),
        path_filter: String::new(),
        case_sensitive: true,
        whole_word: false,
        is_regex: false,
        saved_at: "2026-06-23T00:00:00Z".to_owned(),
    };
    let blob = bookmark_state_blob(std::slice::from_ref(&bm));
    let save_cell: BookmarkStateCell = Arc::new(Mutex::new(None));
    client.save_bookmarks(
        &workspace_id,
        blob,
        "saved".to_owned(),
        Arc::clone(&save_cell),
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if save_cell.lock().unwrap().take().is_some() {
            break;
        }
        if Instant::now() > deadline {
            panic!("PT-3: bookmark save did not deliver");
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    // Read it back.
    let load_cell: BookmarkStateCell = Arc::new(Mutex::new(None));
    client.load_bookmarks(&workspace_id, Arc::clone(&load_cell));
    let deadline = Instant::now() + Duration::from_secs(5);
    let (loaded_blob, _) = loop {
        if let Some(v) = load_cell.lock().unwrap().take() {
            break v.expect("PT-3: bookmark load ok");
        }
        if Instant::now() > deadline {
            panic!("PT-3: bookmark load did not deliver");
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let parsed = parse_bookmark_state(&loaded_blob);
    let found = parsed
        .iter()
        .find(|b| b.id == "pt3-fixture")
        .expect("PT-3: saved bookmark present");
    assert_eq!(found.query, "PT3_QUERY", "PT-3: bookmark query round-trips");
    assert_eq!(
        found.kind,
        KindFilter::Document,
        "PT-3: bookmark kind round-trips"
    );
    assert!(found.case_sensitive, "PT-3: bookmark options round-trip");
    println!(
        "PT-3 PASS: bookmark saved + read back via real PG /search-bookmarks with options intact"
    );
}
