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
//!   - PT-1/PT-2/PT-3: the `integration`-feature test creates an isolated managed workspace and its own
//!     documents, proves search/replace/conflict/bookmark persistence, writes a receipt, and deletes the
//!     workspace. It is not ignored and never depends on operator-preseeded state.
//!
//! ## Artifact hygiene (CX-212E)
//!
//! EVERY PNG is written ONLY to the EXTERNAL `Handshake_Artifacts/handshake-test/wp-kernel-012-mt-029/`
//! root via [`external_artifact_dir`]; [`assert_no_local_artifact_dir`] fails the run if a repo-local
//! `tests/screenshots/` or `test_output/` directory exists.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
#[cfg(feature = "integration")]
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use egui_kittest::kittest::NodeT;
#[cfg(feature = "integration")]
#[path = "native_gui_support/canonical_argus_driver.rs"]
mod canonical_argus_driver;
#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;
#[cfg(feature = "integration")]
use canonical_argus_driver::{json_has_author_id, json_node_by_author_id, CanonicalArgusDriver};
use screenshot_harness::ScreenshotHarness as Harness;

use handshake_native::backend_client::{
    FindInFilesOperation, FindInFilesStamp, LoomGraphSearchHit, RichDocClient, SearchMatchOptions,
    WorkspaceSearchClient,
};
#[cfg(feature = "integration")]
use handshake_native::editor_pane_factories::{
    placeholder_pane_type, BLOCK_COLLECTIONS_PANE_LABEL, WIKI_PAGE_PANE_LABEL,
};
use handshake_native::find_in_files::{
    bookmark_remove_author_id, bookmark_restore_author_id, document_id_from_hit,
    hit_identity_from_result_author_id, pane_scoped_author_id, preview_after_author_id,
    preview_author_id, preview_before_author_id, replace_in_content, result_author_id,
    shell_open_target_from_hit, show, FindInFilesCallbacks, FindInFilesOpenRequest,
    FindInFilesOpenTarget, FindInFilesPaneFactory, FindInFilesPaneShared, FindInFilesPanelState,
    KindFilter, MatchOptions, MatchPreview, ReplacementPlan, SearchBookmark, APPLY_AUTHOR_ID,
    KIND_FILTER_AUTHOR_ID, PREVIEW_REPLACE_AUTHOR_ID, QUERY_AUTHOR_ID, SEARCH_AUTHOR_ID,
    STATUS_AUTHOR_ID, TOGGLE_CASE_AUTHOR_ID, TOGGLE_REGEX_AUTHOR_ID, TOGGLE_WORD_AUTHOR_ID,
};
#[cfg(feature = "integration")]
use handshake_native::find_in_files::{
    BOOKMARK_RETRY_AUTHOR_ID, BOOKMARK_STATUS_AUTHOR_ID, CANCEL_AUTHOR_ID, SAVE_BOOKMARK_AUTHOR_ID,
    TOGGLE_REGEX_STATE_AUTHOR_ID,
};
use handshake_native::find_in_files::{
    PATH_FILTER_AUTHOR_ID, REPLACE_AUTHOR_ID, TAG_FILTER_AUTHOR_ID,
};
use handshake_native::pane_registry::{
    DirtyState, LockState, PaneAuthority, PaneFactory, PaneHostWidget, PaneRecord, PaneRegistry,
    PaneType,
};
use handshake_native::theme::HsTheme;

#[cfg(feature = "integration")]
#[path = "interconnect_support/mod.rs"]
mod interconnect_support;

const TEST_BASE: &str = "http://127.0.0.1:37501";

fn operation_stamp(
    workspace_id: &str,
    operation: FindInFilesOperation,
    sequence: u64,
) -> FindInFilesStamp {
    FindInFilesStamp {
        workspace_id: workspace_id.to_owned(),
        operation,
        epoch: 1,
        sequence,
    }
}

#[test]
fn graph_search_paginates_past_ten_thousand_until_the_canonical_short_page() {
    use std::io::{Read, Write};

    let listener =
        std::net::TcpListener::bind("127.0.0.1:0").expect("bind find-in-files pagination server");
    let base_url = format!(
        "http://{}",
        listener.local_addr().expect("pagination server address")
    );
    let server = std::thread::spawn(move || {
        let mut request_lines = Vec::new();
        for page_index in 0..=20usize {
            let (mut stream, _) = listener.accept().expect("accept graph-search page");
            let mut request = [0u8; 4096];
            let read = stream
                .read(&mut request)
                .expect("read graph-search request");
            request_lines.push(
                String::from_utf8_lossy(&request[..read])
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .to_owned(),
            );

            let row_count = if page_index < 20 { 500 } else { 1 };
            let rows: Vec<_> = (0..row_count)
                .map(|row_index| {
                    let ordinal = page_index * 500 + row_index;
                    serde_json::json!({
                        "source_kind": "document",
                        "result_kind": "knowledge_entity",
                        "ref_id": format!("KRD-pagination-{ordinal:05}"),
                        "title": format!("Pagination result {ordinal:05}"),
                        "excerpt": "canonical find-in-files page",
                        "score": 1.0,
                        "metadata": {}
                    })
                })
                .collect();
            let body = serde_json::to_string(&rows).expect("serialize graph-search page");
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}",
                body.len(), body
            );
            stream
                .write_all(response.as_bytes())
                .expect("write graph-search response");
        }
        request_lines
    });

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("build pagination runtime");
    let runtime_guard = runtime.enter();
    let client = WorkspaceSearchClient::new(base_url, runtime.handle().clone());
    drop(runtime_guard);
    let cell: handshake_native::backend_client::GraphSearchCell =
        Arc::new(Mutex::new(std::collections::VecDeque::new()));
    client.search_paginated(
        "ws-pagination",
        "canonical",
        Some("document"),
        "",
        "",
        SearchMatchOptions::default(),
        "pagination-key".to_owned(),
        operation_stamp("ws-pagination", FindInFilesOperation::Search, 1),
        Arc::clone(&cell),
    );

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let (hits, result_set_key) = loop {
        if let Some(delivery) = cell.lock().expect("pagination delivery queue").pop_front() {
            break delivery
                .outcome
                .expect("pagination must continue past 10,000 results");
        }
        assert!(
            std::time::Instant::now() < deadline,
            "graph-search pagination did not reach its canonical short page"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    };
    assert_eq!(result_set_key, "pagination-key");
    assert_eq!(
        hits.len(),
        10_001,
        "twenty full pages plus the final short page are all retained"
    );
    assert_eq!(hits.last().unwrap().ref_id, "KRD-pagination-10000");

    let request_lines = server.join().expect("pagination server joins");
    assert_eq!(request_lines.len(), 21);
    assert!(request_lines[0].contains("limit=500&offset=0"));
    assert!(request_lines[20].contains("limit=500&offset=10000"));
}

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

fn producer_block(block_id: &str, content_type: &str, title: &str) -> serde_json::Value {
    serde_json::json!({
        "block_id": block_id,
        "workspace_id": "ws-producer-fixture",
        "content_type": content_type,
        "document_id": null,
        "asset_id": null,
        "title": title,
        "original_filename": null,
        "content_hash": null,
        "pinned": false,
        "favorite": false,
        "journal_date": null,
        "created_at": "2026-07-15T00:00:00Z",
        "updated_at": "2026-07-15T00:00:00Z",
        "imported_at": null,
        "derived": {
            "backlink_count": 0,
            "mention_count": 0,
            "tag_count": 0,
            "preview_status": "none"
        }
    })
}

fn producer_hit(
    source_kind: &str,
    result_kind: &str,
    ref_id: &str,
    title: &str,
    metadata: serde_json::Value,
    block: Option<serde_json::Value>,
) -> LoomGraphSearchHit {
    LoomGraphSearchHit {
        source_kind: source_kind.to_owned(),
        result_kind: result_kind.to_owned(),
        ref_id: ref_id.to_owned(),
        title: title.to_owned(),
        excerpt: format!("{title} producer-shaped excerpt"),
        metadata,
        block,
    }
}

fn producer_block_hit(
    source_kind: &str,
    ref_id: &str,
    title: &str,
    content_type: &str,
) -> LoomGraphSearchHit {
    producer_hit(
        source_kind,
        "loom_block",
        ref_id,
        title,
        serde_json::json!({
            "authority_table": "loom_blocks",
            "retrieval_bias_schema_id": "hsk.loom_retrieval_bias@1",
            "retrieval_bias_score": 0.0,
            "retrieval_bias_reasons": []
        }),
        Some(producer_block(ref_id, content_type, title)),
    )
}

fn mock_plan(doc_id: &str, title: &str, count: usize) -> ReplacementPlan {
    ReplacementPlan {
        workspace_id: "ws-1".to_owned(),
        document_id: doc_id.to_owned(),
        title: title.to_owned(),
        expected_version: 3,
        content_json_after: serde_json::json!({ "type": "doc", "content": [] }),
        before_sha256: "0".repeat(64),
        after_sha256: "1".repeat(64),
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

#[test]
fn accesskit_result_ids_are_utf8_injective_and_exactly_reversible() {
    let identities = [
        ("loom_block", "blk/1:x"),
        ("loom_block", "blk-1-x"),
        ("文档", "résumé/東京"),
    ];
    let ids: HashSet<_> = identities
        .iter()
        .map(|(kind, reference)| result_author_id(kind, reference))
        .collect();
    assert_eq!(
        ids.len(),
        identities.len(),
        "distinct identities must not collapse"
    );
    for (kind, reference) in identities {
        let author_id = result_author_id(kind, reference);
        assert_eq!(
            hit_identity_from_result_author_id(&author_id),
            Some((kind.to_owned(), reference.to_owned()))
        );
    }
    assert_eq!(
        result_author_id("document", "KRD-1:/foo?x=1"),
        "find-in-files.result.646f63756d656e74.4b52442d313a2f666f6f3f783d31",
        "ASCII punctuation fixture is literal lowercase two-digit byte hex"
    );
    assert_eq!(
        result_author_id("文档", "résumé/東京"),
        "find-in-files.result.e69687e6a1a3.72c3a973756dc3a92fe69db1e4baac",
        "Unicode fixture is literal lowercase UTF-8 byte hex"
    );
    assert_eq!(
        preview_author_id("KRD-文/1"),
        "find-in-files.preview.4b52442de696872f31",
        "preview routes encode the exact document id with the same codec"
    );
    assert_eq!(
        bookmark_restore_author_id("saved:文/1"),
        "find-in-files.bookmark-restore.73617665643ae696872f31"
    );
    assert_eq!(
        bookmark_remove_author_id("saved:文/1"),
        "find-in-files.bookmark-remove.73617665643ae696872f31"
    );

    let bookmark = |query: &str, case_sensitive: bool| SearchBookmark {
        id: String::new(),
        label: query.to_owned(),
        query: query.to_owned(),
        kind: KindFilter::All,
        tag_filter: String::new(),
        path_filter: String::new(),
        case_sensitive,
        whole_word: false,
        is_regex: false,
        saved_at: "2026-07-15T00:00:00Z".to_owned(),
    };
    assert_eq!(
        bookmark("Foo", true).stable_id(),
        "bookmark-v1.3-466f6f.3-616c6c.0-.0-.4-74727565.5-66616c7365.5-66616c7365"
    );
    assert_ne!(
        bookmark("Foo", true).stable_id(),
        bookmark("foo", true).stable_id(),
        "case-sensitive searches must retain case-distinct identities"
    );
    assert_ne!(
        bookmark("文", false).stable_id(),
        bookmark("東", false).stable_id(),
        "Unicode-only searches must retain byte-distinct identities"
    );
}

/// A panel seeded with 3 results and 2 preview plans (the PT-5 render fixture).
fn seeded_state() -> FindInFilesPanelState {
    let mut s = FindInFilesPanelState::new();
    s.bind_workspace(Some("ws-1"), 0);
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
    s.bookmarks = vec![SearchBookmark {
        id: "saved:文/1".to_owned(),
        label: "Saved fixture".to_owned(),
        query: "FIND_TARGET".to_owned(),
        kind: KindFilter::Document,
        tag_filter: "tag-1".to_owned(),
        path_filter: "notes".to_owned(),
        case_sensitive: true,
        whole_word: true,
        is_regex: false,
        saved_at: "2026-07-15T00:00:00Z".to_owned(),
    }];
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
        .proof_mt_id("MT-029")
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

fn author_ids<State>(harness: &Harness<'_, State>) -> HashSet<String> {
    let mut ids = HashSet::new();
    for node in harness.root().children_recursive() {
        if let Some(a) = node.accesskit_node().author_id() {
            ids.insert(a.to_owned());
        }
    }
    ids
}

fn click_author_id<State>(harness: &Harness<'_, State>, author_id: &str) {
    let node = harness
        .root()
        .children_recursive()
        .find(|n| n.accesskit_node().author_id() == Some(author_id))
        .unwrap_or_else(|| panic!("no node with author_id '{author_id}' to click"));
    node.click_accesskit();
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
        STATUS_AUTHOR_ID,
    ] {
        assert!(
            ids.contains(required),
            "PT-5: required author_id '{required}' missing from {ids:?}"
        );
    }
    // At least one result-row node + one preview node.
    assert!(
        ids.contains(&result_author_id("loom_block", "blk-1")),
        "PT-5: encoded result-row author_id missing from {ids:?}"
    );
    assert!(
        ids.contains(&preview_author_id("KRD-1")),
        "PT-5: encoded preview author_id missing from {ids:?}"
    );
    assert!(ids.contains(&bookmark_restore_author_id("saved:文/1")));
    assert!(ids.contains(&bookmark_remove_author_id("saved:文/1")));
    let status = harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(STATUS_AUTHOR_ID))
        .expect("stable Find in Files status node");
    assert_eq!(
        status.accesskit_node().role(),
        egui::accesskit::Role::Status
    );
    println!("PT-5/AC-1/AC-10: all contract author_ids present in the live AccessKit tree");
    assert_no_local_artifact_dir();
}

#[test]
fn text_inputs_advertise_and_apply_canonical_set_value() {
    fn dispatch_set_value(harness: &mut Harness<'_, ()>, author_id: &str, value: &str) {
        let node = harness
            .root()
            .children_recursive()
            .find(|node| node.accesskit_node().author_id() == Some(author_id))
            .unwrap_or_else(|| panic!("missing Find in Files text input {author_id}"));
        assert!(
            node.accesskit_node()
                .data()
                .supports_action(egui::accesskit::Action::SetValue),
            "{author_id} must advertise canonical SetValue"
        );
        let node_id = node.accesskit_node().id();
        harness.event(egui::Event::AccessKitActionRequest(
            egui::accesskit::ActionRequest {
                action: egui::accesskit::Action::SetValue,
                target: node_id,
                data: Some(egui::accesskit::ActionData::Value(value.into())),
            },
        ));
        harness.run_steps(2);
    }

    fn seed_stale_sensitive_state(state: &mut FindInFilesPanelState) {
        state.preview_plans = vec![mock_plan("KRD-sentinel", "Sentinel", 1)];
        state.preview_plan_key = Some("sentinel-preview-key".to_owned());
        state.replace_status = Some("sentinel replace status".to_owned());
        state.error = Some("sentinel error".to_owned());
    }

    let state = Arc::new(Mutex::new(seeded_state()));
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
    harness.run_steps(2);

    for (author_id, value) in [
        (QUERY_AUTHOR_ID, "canonical query"),
        (TAG_FILTER_AUTHOR_ID, "tag-canonical"),
        (PATH_FILTER_AUTHOR_ID, "notes/canonical"),
    ] {
        {
            let mut guard = state.lock().expect("Find in Files state");
            seed_stale_sensitive_state(&mut guard);
        }
        let generation_before = state
            .lock()
            .expect("Find in Files state")
            .results_generation();
        dispatch_set_value(&mut harness, author_id, value);
        let guard = state.lock().expect("Find in Files state");
        assert!(guard.preview_plans.is_empty(), "{author_id}");
        assert!(guard.preview_plan_key.is_none(), "{author_id}");
        assert!(guard.replace_status.is_none(), "{author_id}");
        assert!(guard.error.is_none(), "{author_id}");
        assert_ne!(
            guard.results_generation(),
            generation_before,
            "{author_id} must invalidate the search generation"
        );
    }

    {
        let mut guard = state.lock().expect("Find in Files state");
        seed_stale_sensitive_state(&mut guard);
    }
    let generation_before = state
        .lock()
        .expect("Find in Files state")
        .results_generation();
    dispatch_set_value(&mut harness, REPLACE_AUTHOR_ID, "canonical replacement");
    let guard = state.lock().expect("Find in Files state");
    assert_eq!(guard.query, "canonical query");
    assert_eq!(guard.replacement, "canonical replacement");
    assert_eq!(guard.tag_filter, "tag-canonical");
    assert_eq!(guard.path_filter, "notes/canonical");
    assert!(guard.preview_plans.is_empty());
    assert!(guard.preview_plan_key.is_none());
    assert_eq!(
        guard.replace_status.as_deref(),
        Some("Preview is stale; run Preview Replace again before applying.")
    );
    assert_eq!(guard.error.as_deref(), Some("sentinel error"));
    assert_eq!(
        guard.results_generation(),
        generation_before,
        "replacement invalidation must preserve the current search result generation"
    );
}

#[test]
fn replacement_set_value_is_rejected_while_apply_is_in_flight() {
    let mut initial = seeded_state();
    initial.set_apply_in_flight_for_test(true);
    let expected_replacement = initial.replacement.clone();
    let expected_preview_key = initial.preview_plan_key.clone();
    let expected_preview_count = initial.preview_plans.len();
    let state = Arc::new(Mutex::new(initial));
    let opened = Arc::new(Mutex::new(Vec::new()));
    let r = rt();
    let mut harness = harness_for(
        Arc::clone(&state),
        opened,
        WorkspaceSearchClient::new(TEST_BASE, r.handle().clone()),
        RichDocClient::new(TEST_BASE, r.handle().clone()),
        Some("ws-1".to_owned()),
    );
    harness.run_steps(2);

    let node = harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(REPLACE_AUTHOR_ID))
        .expect("replacement input");
    assert!(node.accesskit_node().is_disabled());
    assert!(
        !node
            .accesskit_node()
            .data()
            .supports_action(egui::accesskit::Action::SetValue),
        "disabled replacement must not advertise SetValue"
    );
    let node_id = node.accesskit_node().id();
    harness.event(egui::Event::AccessKitActionRequest(
        egui::accesskit::ActionRequest {
            action: egui::accesskit::Action::SetValue,
            target: node_id,
            data: Some(egui::accesskit::ActionData::Value("must-not-apply".into())),
        },
    ));
    harness.run_steps(2);

    let guard = state.lock().expect("Find in Files state");
    assert_eq!(guard.replacement, expected_replacement);
    assert_eq!(guard.preview_plan_key, expected_preview_key);
    assert_eq!(guard.preview_plans.len(), expected_preview_count);
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

#[test]
fn every_find_in_files_result_kind_resolves_to_its_dedicated_production_route() {
    let document = producer_hit(
        "document",
        "knowledge_entity",
        "KRD-7",
        "Doc",
        serde_json::json!({
            "authority_table": "knowledge_rich_documents",
            "rich_document_id": "KRD-7",
            "document_id": null,
            "schema_version": "hsk.knowledge.rich_document@1",
            "doc_version": 1,
            "authority_label": "operator"
        }),
        None,
    );
    let loom_block = producer_block_hit("loom_block", "BLK-1", "Block", "note");
    let file = producer_block_hit("file", "BLK-FILE", "File", "file");
    let tag_hub = producer_block_hit("tag_hub", "BLK-TAG", "Tag", "tag_hub");
    let symbol = producer_hit(
        "symbol",
        "knowledge_entity",
        "SYM-1",
        "Symbol",
        serde_json::json!({
            "authority_table": "knowledge_entities",
            "entity_key": "symbol::SYM-1",
            "detection_provenance": {"source": "producer-fixture"}
        }),
        None,
    );
    let work_packet = producer_hit(
        "work_packet",
        "knowledge_entity",
        "entity-wp",
        "WP",
        serde_json::json!({
            "authority_table": "knowledge_entities",
            "entity_key": "WP-KERNEL-012",
            "detection_provenance": {"source": "producer-fixture"}
        }),
        None,
    );
    let micro_task = producer_hit(
        "micro_task",
        "knowledge_entity",
        "entity-mt",
        "MT",
        serde_json::json!({
            "authority_table": "knowledge_entities",
            "entity_key": "WP-KERNEL-012/MT-029",
            "detection_provenance": {"source": "producer-fixture"}
        }),
        None,
    );
    let user_manual = producer_hit(
        "user_manual_page",
        "user_manual_page",
        "fallback-slug",
        "Manual",
        serde_json::json!({
            "authority_table": "user_manual_pages",
            "page_slug": "native-editors"
        }),
        None,
    );
    let wiki = producer_hit(
        "wiki_page",
        "wiki_page",
        "WIKI-PROJECTION-1",
        "Wiki",
        serde_json::json!({
            "authority_table": "knowledge_wiki_projections",
            "projection_id": "WIKI-PROJECTION-1",
            "page_type": "reference",
            "rebuild_status": "current"
        }),
        None,
    );
    let collection = producer_block_hit("loom_block", "BLK-VIEW", "View", "view_def");

    let cases = [
        (
            document,
            FindInFilesOpenTarget::Document {
                document_id: "KRD-7".into(),
            },
        ),
        (
            loom_block,
            FindInFilesOpenTarget::LoomBlock {
                block_id: "BLK-1".into(),
            },
        ),
        (
            file,
            FindInFilesOpenTarget::LoomBlock {
                block_id: "BLK-FILE".into(),
            },
        ),
        (
            tag_hub,
            FindInFilesOpenTarget::LoomBlock {
                block_id: "BLK-TAG".into(),
            },
        ),
        (
            symbol,
            FindInFilesOpenTarget::CodeSymbol {
                symbol_entity_id: "SYM-1".into(),
            },
        ),
        (
            work_packet,
            FindInFilesOpenTarget::WorkPacket {
                wp_id: "WP-KERNEL-012".into(),
            },
        ),
        (
            micro_task,
            FindInFilesOpenTarget::MicroTask {
                mt_id: "MT-029".into(),
                wp_id: Some("WP-KERNEL-012".into()),
            },
        ),
        (
            user_manual,
            FindInFilesOpenTarget::UserManual {
                slug: "native-editors".into(),
            },
        ),
        (
            wiki,
            FindInFilesOpenTarget::WikiPage {
                projection_id: "WIKI-PROJECTION-1".into(),
            },
        ),
        (
            collection,
            FindInFilesOpenTarget::BlockCollectionView {
                view_block_id: "BLK-VIEW".into(),
            },
        ),
    ];

    for (search_hit, expected) in cases {
        assert_eq!(shell_open_target_from_hit(&search_hit), Some(expected));
    }
    assert_eq!(
        shell_open_target_from_hit(&hit("future_kind", "", "Unknown", "", None)),
        None,
        "unsupported or identity-less future kinds stay non-navigable"
    );
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
        .proof_mt_id("MT-029")
        .with_size(egui::vec2(900.0, 760.0))
        .build_ui(move |ui| {
            PaneHostWidget::show(ui, &reg, |_pane_type| factory.as_ref());
        });
    // The real pane mount legitimately starts an asynchronous bookmark GET and requests another
    // repaint while that request is in flight. This registry proof only needs one deterministic
    // mounted frame; `run()` incorrectly requires repaint quiescence and therefore races the
    // current-thread runtime that this test intentionally does not drive.
    harness.step();

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

#[test]
fn two_registry_find_panes_keep_state_and_author_ids_isolated() {
    let r = rt();
    let sc = WorkspaceSearchClient::new(TEST_BASE, r.handle().clone());
    let dc = RichDocClient::new(TEST_BASE, r.handle().clone());
    let pane_a: handshake_native::pane_registry::PaneId = Arc::from("find-pane-a");
    let pane_b: handshake_native::pane_registry::PaneId = Arc::from("find-pane-b");
    let shared = Arc::new(Mutex::new(FindInFilesPaneShared::new(
        HsTheme::Dark.palette(),
    )));
    {
        let mut guard = shared.lock().expect("shared Find state");
        guard.workspace_id = Some("ws-1".to_owned());
        guard.active_pane_id = Some(pane_a.clone());
    }
    let factory = FindInFilesPaneFactory::with_state(sc, dc, Arc::clone(&shared), seeded_state());
    let states = factory.states_handle();
    let factory: Box<dyn PaneFactory> = Box::new(factory);
    let mut registry = PaneRegistry::new();
    for pane_id in [pane_a.clone(), pane_b.clone()] {
        registry.insert(PaneRecord::new(
            pane_id,
            PaneType::FindInFiles,
            "p",
            None,
            LockState::Unlocked,
            DirtyState::Clean,
            PaneAuthority::System,
        ));
    }
    let mut harness = Harness::builder()
        .proof_mt_id("MT-029")
        .with_size(egui::vec2(900.0, 760.0))
        .build_ui(move |ui| {
            PaneHostWidget::show(ui, &registry, |_pane_type| factory.as_ref());
        });
    harness.step();

    let rendered_author_ids = harness
        .root()
        .children_recursive()
        .filter_map(|node| node.accesskit_node().author_id().map(str::to_owned))
        .collect::<Vec<_>>();
    let unique = rendered_author_ids.iter().collect::<HashSet<_>>();
    assert_eq!(
        unique.len(),
        rendered_author_ids.len(),
        "two mounted Find panes must expose a globally unique author-id tree"
    );
    assert!(rendered_author_ids
        .iter()
        .any(|author_id| author_id == QUERY_AUTHOR_ID));
    assert!(rendered_author_ids.iter().any(|author_id| {
        author_id == &pane_scoped_author_id(QUERY_AUTHOR_ID, Some(pane_b.as_ref()))
    }));
    let leased_control_ids = |harness: &Harness<'_, ()>| {
        author_ids(harness)
            .into_iter()
            .filter(|author_id| {
                [
                    QUERY_AUTHOR_ID,
                    REPLACE_AUTHOR_ID,
                    SEARCH_AUTHOR_ID,
                    PREVIEW_REPLACE_AUTHOR_ID,
                    APPLY_AUTHOR_ID,
                    STATUS_AUTHOR_ID,
                ]
                .iter()
                .any(|base| author_id == base || author_id.starts_with(&format!("{base}.pane-")))
            })
            .collect::<HashSet<_>>()
    };
    let initial_author_ids = leased_control_ids(&harness);
    {
        shared.lock().expect("shared Find state").active_pane_id = Some(pane_b.clone());
    }
    harness.step();
    let focus_b_author_ids = leased_control_ids(&harness);
    assert_eq!(
        focus_b_author_ids, initial_author_ids,
        "canonical/scoped Find author ids cannot swap when focus moves to the sibling Find pane"
    );
    {
        shared.lock().expect("shared Find state").active_pane_id = Some(Arc::from("non-find-pane"));
    }
    harness.step();
    assert_eq!(
        leased_control_ids(&harness),
        initial_author_ids,
        "canonical/scoped Find author ids remain stable when global focus leaves Find panes"
    );

    let pane_b_query_author_id = pane_scoped_author_id(QUERY_AUTHOR_ID, Some(pane_b.as_ref()));
    let pane_b_replace_author_id = pane_scoped_author_id(REPLACE_AUTHOR_ID, Some(pane_b.as_ref()));
    for (author_id, value) in [
        (&pane_b_query_author_id, "secondary-query"),
        (&pane_b_replace_author_id, "secondary-replacement"),
    ] {
        let node = harness
            .root()
            .children_recursive()
            .find(|node| node.accesskit_node().author_id() == Some(author_id.as_str()))
            .unwrap_or_else(|| panic!("secondary pane target missing: {author_id}"));
        let node_id = node.accesskit_node().id();
        harness.event(egui::Event::AccessKitActionRequest(
            egui::accesskit::ActionRequest {
                action: egui::accesskit::Action::SetValue,
                target: node_id,
                data: Some(egui::accesskit::ActionData::Value(value.into())),
            },
        ));
        harness.run_steps(2);
    }

    let states = states.lock().expect("pane-keyed Find states");
    assert_eq!(states.len(), 2);
    let primary = states.get(&pane_a).expect("primary pane state");
    let secondary = states.get(&pane_b).expect("secondary pane state");
    assert_eq!(primary.query, "FIND_TARGET");
    assert_eq!(primary.replacement, "REPLACED");
    assert_eq!(secondary.query, "secondary-query");
    assert_eq!(secondary.replacement, "secondary-replacement");
}

#[test]
fn typed_find_open_request_uses_origin_and_rejects_stale_authority() {
    let workspace_id = "mt029-route-workspace";
    let mut app = handshake_native::app::HandshakeApp::with_health(
        handshake_native::app::HealthDisplayState::Loading,
    );
    app.bind_active_project_for_integration_test(workspace_id);
    let pane_ids = app
        .tab_bar_states()
        .keys()
        .take(2)
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(pane_ids.len(), 2, "seeded shell has two target panes");
    let origin = pane_ids[0].clone();
    let wrong_focus = pane_ids[1].clone();
    app.set_active_pane_for_test(Some(wrong_focus.clone()));

    let exact_block = "BLK-MT029-ORIGIN";
    app.enqueue_find_in_files_open_request_for_test(FindInFilesOpenRequest {
        origin_pane_id: origin.clone(),
        workspace_id: workspace_id.to_owned(),
        hit: producer_block_hit("loom_block", exact_block, "Exact origin", "note"),
    });
    assert_eq!(app.drain_find_in_files_open_requests_for_test(), 1);
    assert!(app
        .tab_bar_states()
        .get(&origin)
        .is_some_and(|bar| bar.tabs.iter().any(|tab| {
            tab.pane_type == PaneType::LoomBlock && tab.content_id.as_deref() == Some(exact_block)
        })));
    assert!(!app
        .tab_bar_states()
        .get(&wrong_focus)
        .is_some_and(|bar| bar.tabs.iter().any(|tab| {
            tab.pane_type == PaneType::LoomBlock && tab.content_id.as_deref() == Some(exact_block)
        })));

    app.enqueue_find_in_files_open_request_for_test(FindInFilesOpenRequest {
        origin_pane_id: origin,
        workspace_id: "stale-workspace".to_owned(),
        hit: producer_block_hit("loom_block", "BLK-MT029-STALE", "Stale", "note"),
    });
    app.enqueue_find_in_files_open_request_for_test(FindInFilesOpenRequest {
        origin_pane_id: Arc::from("missing-pane"),
        workspace_id: workspace_id.to_owned(),
        hit: producer_block_hit("loom_block", "BLK-MT029-MISSING", "Missing", "note"),
    });
    assert_eq!(
        app.drain_find_in_files_open_requests_for_test(),
        0,
        "stale-workspace and missing-origin requests fail closed"
    );
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
        .proof_mt_id("MT-029")
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
    assert_no_local_artifact_dir();

    let Some(image) =
        harness.render_proof_frame("MT-029 standalone Find-in-Files screenshot frame")
    else {
        assert!(
            harness
                .last_screenshot_outcome()
                .is_some_and(|outcome| outcome.status == "DEFERRED"),
            "headless screenshot path must retain a typed DEFERRED MT-029 marker"
        );
        return;
    };
    let (w, h) = (image.width(), image.height());
    assert!(w > 0 && h > 0, "rendered image must be non-empty");
    let raw = image.as_raw();
    let mut counts: std::collections::HashMap<[u8; 4], u32> = std::collections::HashMap::new();
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
    std::fs::create_dir_all(&ext_dir).expect("create MT-029 external screenshot directory");
    let png = ext_dir.join("MT-029-find-in-files.png");
    image
        .save(&png)
        .unwrap_or_else(|error| panic!("save MT-029 screenshot {}: {error}", png.display()));
    assert!(
        png.is_file(),
        "screenshot PNG was not created at {}",
        png.display()
    );
    println!(
        "SCREENSHOT: {w}x{h}, {} distinct colours, white_frac={:.3}, saved=true ({})",
        counts.len(),
        white as f32 / total as f32,
        png.display()
    );
    assert_no_local_artifact_dir();
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// PT-1 / PT-2 / PT-3 LIVE MANAGED INTEGRATION. Feature-gated but deliberately NOT ignored.
// This proof owns its fixture lifecycle: isolated workspace + documents + bookmark are created through
// production HTTP routes and the workspace is deleted deterministically after the receipt is written.

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

#[cfg(feature = "integration")]
fn wait_search(
    cell: &handshake_native::backend_client::GraphSearchCell,
) -> Result<(Vec<LoomGraphSearchHit>, String), String> {
    for _ in 0..200 {
        if let Some(delivery) = cell.lock().unwrap().pop_front() {
            return delivery.outcome;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    panic!("managed graph-search completion exceeded 10 seconds")
}

#[cfg(feature = "integration")]
fn wait_replace(
    cell: &handshake_native::backend_client::FindReplaceCell,
) -> handshake_native::find_in_files::ReplaceDelivery {
    for _ in 0..200 {
        if let Some(delivery) = cell.lock().unwrap().pop_front() {
            return delivery.outcome;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    panic!("managed replace completion exceeded 10 seconds")
}

#[cfg(feature = "integration")]
fn wait_bookmark(
    cell: &handshake_native::backend_client::BookmarkStateCell,
) -> Result<(serde_json::Value, Option<String>, Option<String>), String> {
    for _ in 0..200 {
        if let Some(delivery) = cell.lock().unwrap().pop_front() {
            return delivery.outcome;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    panic!("managed bookmark completion exceeded 10 seconds")
}

#[cfg(feature = "integration")]
#[allow(clippy::too_many_arguments)]
fn managed_search(
    client: &WorkspaceSearchClient,
    workspace_id: &str,
    query: &str,
    source_kind: Option<&str>,
    tag_filter: &str,
    path_filter: &str,
    options: SearchMatchOptions,
    sequence: u64,
) -> Vec<LoomGraphSearchHit> {
    let cell = Arc::new(Mutex::new(std::collections::VecDeque::new()));
    client.search_paginated(
        workspace_id,
        query,
        source_kind,
        tag_filter,
        path_filter,
        options,
        format!("managed-{sequence}"),
        operation_stamp(workspace_id, FindInFilesOperation::Search, sequence),
        Arc::clone(&cell),
    );
    wait_search(&cell)
        .unwrap_or_else(|error| panic!("managed graph-search {sequence} failed: {error}"))
        .0
}

#[cfg(feature = "integration")]
fn wait_panel_idle(
    state: &mut FindInFilesPanelState,
    search_client: &WorkspaceSearchClient,
    workspace_id: &str,
) {
    for _ in 0..400 {
        state.poll_with_search_refresh(search_client, Some(workspace_id));
        if !state.loading {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    panic!("managed mounted panel state did not become idle within 20 seconds");
}

/// PT-1/PT-2/PT-3: self-seeded graph search, preview/apply with an induced optimistic-concurrency
/// conflict and exact hash receipts, persisted reload, bookmark round-trip, and deterministic cleanup.
#[test]
#[cfg(feature = "integration")]
fn find_in_files_search_find_in_files_replace_cycle_find_in_files_bookmark_roundtrip() {
    use handshake_native::backend_client::{BookmarkStateCell, FindReplaceCell};
    use handshake_native::find_in_files::{
        content_json_sha256, parse_bookmark_state, ReplaceAuditOutcome, ReplaceDelivery,
    };

    let live = interconnect_support::require_reachable_backend();
    let unique = format!(
        "mt029-{}-{}",
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
    let other_workspace = live.create_workspace(&format!("{unique}-cross-workspace"));
    let other_workspace_id = other_workspace["id"]
        .as_str()
        .expect("cross-workspace create returns id")
        .to_owned();
    let mut other_cleanup = ManagedWorkspaceCleanup {
        backend: &live,
        workspace_id: other_workspace_id.clone(),
        cleaned: false,
    };

    let needle = format!("MT029_FIND_{unique}");
    let replacement = format!("MT029_REPLACED_{unique}");
    let create_doc = |title: &str, content_json: serde_json::Value| {
        live.post_json(
            "/knowledge/documents",
            &serde_json::json!({
                "workspace_id": workspace_id,
                "title": title,
                "content_json": content_json
            }),
        )
    };
    let preserved_image = serde_json::json!({
        "type": "image",
        "attrs": {
            "target": "https://example.invalid/mt029-preserved.png",
            "alt": "MT-029 preserved image",
            "width": 320,
            "preserve": {"nested": [true, 29]}
        }
    });
    let preserved_table = serde_json::json!({
        "type": "table",
        "attrs": {"preserve": "table-marker"},
        "content": [{
            "type": "tableRow",
            "content": [{
                "type": "tableCell",
                "attrs": {"colspan": 1, "rowspan": 1},
                "content": [{
                    "type": "paragraph",
                    "content": [{"type": "text", "text": "UNCHANGED_TABLE_CONTENT"}]
                }]
            }]
        }]
    });
    let first_content = serde_json::json!({
        "type": "doc",
        "content": [
            {"type":"paragraph","content":[{"type":"text","text":needle}]},
            {"type":"codeBlock","attrs":{"code":needle,"language":"rust","preserve":"attrs-marker"}},
            preserved_image.clone(),
            preserved_table.clone()
        ]
    });
    let ordinary_content = || {
        serde_json::json!({
            "type": "doc",
            "content": [{"type":"paragraph","content":[{"type":"text","text":needle}]}]
        })
    };
    let mut created = vec![
        create_doc("MT-029 first path-scope", first_content),
        create_doc("MT-029 conflict", ordinary_content()),
    ];
    for index in 2..501 {
        created.push(create_doc(
            &format!("MT-029 pagination {index:03}"),
            ordinary_content(),
        ));
    }
    let document_ids: Vec<String> = created
        .iter()
        .map(|value| {
            value["document"]["rich_document_id"]
                .as_str()
                .expect("document create returns rich_document_id")
                .to_owned()
        })
        .collect();
    let decoy_needle = needle.replacen("MT029_FIND_", "MT029_FIND.+é_", 1);
    assert_ne!(
        decoy_needle, needle,
        "the persisted near-match must differ from the positive literal"
    );
    let decoy_created = create_doc(
        "MT-029 same-kind Unicode regex-metachar decoy",
        serde_json::json!({
            "type": "doc",
            "content": [{
                "type": "paragraph",
                "content": [{"type": "text", "text": decoy_needle}]
            }]
        }),
    );
    let decoy_document_id = decoy_created["document"]["rich_document_id"]
        .as_str()
        .expect("decoy create returns rich_document_id")
        .to_owned();

    // Positive tag-filter fixture: a real note -> tag_hub tag edge. Documents are intentionally not
    // treated as tagged Loom blocks, so this proves the backend graph-search tag path on its own terms.
    let tag_hub = live.post_json(
        &format!("/workspaces/{workspace_id}/loom/blocks"),
        &serde_json::json!({"content_type":"tag_hub","title":"MT-029 managed tag"}),
    );
    let tag_hub_id = tag_hub["block_id"]
        .as_str()
        .expect("tag hub block_id")
        .to_owned();
    let tagged_block = live.post_json(
        &format!("/workspaces/{workspace_id}/loom/blocks"),
        &serde_json::json!({"content_type":"note","title":format!("tagged {needle}")}),
    );
    let tagged_block_id = tagged_block["block_id"]
        .as_str()
        .expect("tagged note block_id")
        .to_owned();
    live.post_json(
        &format!("/workspaces/{workspace_id}/loom/edges"),
        &serde_json::json!({
            "source_block_id": tagged_block_id,
            "target_block_id": tag_hub_id,
            "edge_type":"tag",
            "created_by":"user"
        }),
    );

    let cross_workspace_doc = live.post_json(
        "/knowledge/documents",
        &serde_json::json!({
            "workspace_id": other_workspace_id,
            "title":"MT-029 cross-workspace reject",
            "content_json": ordinary_content()
        }),
    );
    let cross_workspace_document_id = cross_workspace_doc["document"]["rich_document_id"]
        .as_str()
        .expect("cross-workspace rich_document_id")
        .to_owned();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("managed MT-029 runtime");
    let search_client = WorkspaceSearchClient::new(live.base.clone(), runtime.handle().clone());
    let doc_client = RichDocClient::new(live.base.clone(), runtime.handle().clone());

    let cross_preview_cell: FindReplaceCell =
        Arc::new(Mutex::new(std::collections::VecDeque::new()));
    doc_client.preview_replace(
        &workspace_id,
        vec![cross_workspace_document_id],
        handshake_native::find_in_files::compile_search_regex(&needle, MatchOptions::default())
            .expect("cross-workspace literal regex"),
        replacement.clone(),
        MatchOptions::default(),
        "cross-workspace-preview".to_owned(),
        operation_stamp(&workspace_id, FindInFilesOperation::Preview, 90),
        Arc::clone(&cross_preview_cell),
    );
    match wait_replace(&cross_preview_cell) {
        ReplaceDelivery::PreviewError(error) => {
            assert!(error.contains("cross-workspace"));
            assert!(error.contains(&workspace_id));
            assert!(error.contains(&other_workspace_id));
        }
        other => panic!("cross-workspace KRD hit must fail preview, got {other:?}"),
    }

    let hits = managed_search(
        &search_client,
        &workspace_id,
        &needle,
        Some("document"),
        "",
        "",
        SearchMatchOptions::default(),
        1,
    );
    assert_eq!(
        hits.len(),
        501,
        "two-page search returns the complete 501-document canonical set"
    );
    let hit_document_ids: HashSet<String> = hits.iter().filter_map(document_id_from_hit).collect();
    assert!(
        !hit_document_ids.contains(&decoy_document_id),
        "the persisted same-kind Unicode/metachar near-match is not a false positive"
    );
    for document_id in &document_ids {
        assert!(
            hit_document_ids.contains(document_id),
            "self-seeded document {document_id} appears in graph-search"
        );
    }
    let decoy_hits = managed_search(
        &search_client,
        &workspace_id,
        &decoy_needle,
        Some("document"),
        "",
        "",
        SearchMatchOptions::default(),
        62,
    );
    assert_eq!(decoy_hits.len(), 1, "the decoy is itself searchable");
    assert_eq!(
        document_id_from_hit(&decoy_hits[0]).as_deref(),
        Some(decoy_document_id.as_str()),
        "the negative control is a persisted document, not an absent fixture"
    );
    let lower = needle.to_lowercase();
    assert_eq!(
        managed_search(
            &search_client,
            &workspace_id,
            &lower,
            Some("document"),
            "",
            "",
            SearchMatchOptions::default(),
            2,
        )
        .len(),
        501,
        "case-insensitive document-kind search finds the full set"
    );
    assert!(managed_search(
        &search_client,
        &workspace_id,
        &lower,
        Some("document"),
        "",
        "",
        SearchMatchOptions {
            case_sensitive: true,
            ..Default::default()
        },
        3,
    )
    .is_empty());
    let positive_tag_hits = managed_search(
        &search_client,
        &workspace_id,
        &needle,
        Some("loom_block"),
        &tag_hub_id,
        "",
        SearchMatchOptions::default(),
        61,
    );
    assert_eq!(
        positive_tag_hits.len(),
        1,
        "real tag filter returns its tagged note"
    );
    assert_eq!(positive_tag_hits[0].ref_id, tagged_block_id);
    assert_eq!(
        managed_search(
            &search_client,
            &workspace_id,
            &needle,
            Some("document"),
            "",
            "",
            SearchMatchOptions {
                whole_word: true,
                ..Default::default()
            },
            4,
        )
        .len(),
        501
    );
    assert_eq!(
        managed_search(
            &search_client,
            &workspace_id,
            &format!("MT029_FIND_.*{}", regex::escape(&unique)),
            Some("document"),
            "",
            "",
            SearchMatchOptions {
                is_regex: true,
                ..Default::default()
            },
            5,
        )
        .len(),
        501
    );
    assert!(managed_search(
        &search_client,
        &workspace_id,
        &needle,
        Some("document"),
        "missing-tag-id",
        "",
        SearchMatchOptions::default(),
        6,
    )
    .is_empty());
    assert_eq!(
        managed_search(
            &search_client,
            &workspace_id,
            &needle,
            Some("document"),
            "",
            "path-scope",
            SearchMatchOptions::default(),
            7,
        )
        .len(),
        1
    );
    let first_hit = hits
        .iter()
        .find(|hit| document_id_from_hit(hit).as_ref() == Some(&document_ids[0]))
        .expect("first self-seeded hit exists");
    let route = result_author_id(&first_hit.source_kind, &first_hit.ref_id);
    assert_eq!(
        hit_identity_from_result_author_id(&route),
        Some((first_hit.source_kind.clone(), first_hit.ref_id.clone()))
    );

    // Real mounted factory/UI proof: type Search and Replace through AccessKit, click the real backend
    // hit into open_requests and the exact shell resolver, then Preview -> Apply through UI controls.
    let ui_needle = format!("MT029_UI_{unique}");
    let ui_replacement = format!("MT029_UI_REPLACED_{unique}");
    let ui_title = "MT-029 mounted UI";
    let ui_content = serde_json::json!({
        "type":"doc",
        "content":[{"type":"paragraph","content":[{"type":"text","text":ui_needle}]}]
    });
    let ui_created = create_doc(ui_title, ui_content.clone());
    let ui_document_id = ui_created["document"]["rich_document_id"]
        .as_str()
        .expect("mounted UI document id")
        .to_owned();
    let ui_document_version = ui_created["document"]["doc_version"]
        .as_u64()
        .expect("mounted UI document version");
    let ui_hits = managed_search(
        &search_client,
        &workspace_id,
        &ui_needle,
        Some("document"),
        "",
        "",
        SearchMatchOptions::default(),
        8,
    );
    let ui_hit = ui_hits
        .iter()
        .find(|hit| document_id_from_hit(hit).as_deref() == Some(ui_document_id.as_str()))
        .expect("fresh managed search returns the mounted UI document");
    assert_eq!(ui_hit.source_kind, "document");
    assert_eq!(ui_hit.ref_id, ui_document_id);
    let ui_result_author_id = result_author_id(&ui_hit.source_kind, &ui_hit.ref_id);
    let mut production_app = handshake_native::app::HandshakeApp::with_health(
        handshake_native::app::HealthDisplayState::Ok(
            handshake_native::backend_client::HealthInfo {
                status: "ok".to_owned(),
                db_status: "ok".to_owned(),
                migration_version: Some(1),
            },
        ),
    );
    production_app.set_backend_base_url_for_test(&live.base, runtime.handle().clone());
    production_app.bind_active_project_for_integration_test(workspace_id.clone());
    assert!(production_app.dispatch_palette_action_for_test(
        handshake_native::command_registry::CMD_VIEW_FIND_IN_FILES
    ));
    let mut argus = CanonicalArgusDriver::bind(&production_app, "mt029-find-in-files");
    let _managed_wgpu_guard = wgpu_guard();
    // The mounted proof surface is deliberately WIDER than the focused unit harnesses: the production
    // shell lays the Find pane out as ONE COLUMN of the live pane grid (Find | Wiki | Chat), so the
    // pane is roughly a quarter of the viewport. At 900px the query / toggles / Search / Preview /
    // Apply / Cancel / Bookmark controls were cropped by the neighbouring pane, and at 1800px Cancel
    // and Bookmark Search were still clipped at the pane boundary. A validator must be able to SEE the
    // requested surface in the captured frame, so the managed run uses a viewport whose Find column is
    // wide enough to render every requested control inside its REAL production pane — the panel is not
    // detached, resized, or given a special capture-only layout.
    let mut ui_harness = Harness::builder()
        .proof_mt_id("MT-029")
        .with_size(egui::vec2(2560.0, 1200.0))
        .wgpu()
        .build_state(
            |ctx, app: &mut handshake_native::app::HandshakeApp| app.ui(ctx),
            production_app,
        );
    ui_harness.run_steps(2);
    let initial_tree = argus.inspect(&mut ui_harness);
    for stable in [
        QUERY_AUTHOR_ID,
        REPLACE_AUTHOR_ID,
        SEARCH_AUTHOR_ID,
        PREVIEW_REPLACE_AUTHOR_ID,
        APPLY_AUTHOR_ID,
        STATUS_AUTHOR_ID,
    ] {
        assert!(
            json_has_author_id(&initial_tree, stable),
            "canonical Argus initial inspection missing {stable}"
        );
    }
    argus.set_value_and_reinspect(&mut ui_harness, QUERY_AUTHOR_ID, &ui_needle);
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut ui_harness,
        "query-value-visible",
        serde_json::json!({"query": ui_needle.clone()}),
        |tree| {
            json_node_by_author_id(tree, QUERY_AUTHOR_ID)
                .and_then(|node| node.get("value"))
                .and_then(serde_json::Value::as_str)
                == Some(ui_needle.as_str())
        },
    );
    argus.click_and_reinspect(&mut ui_harness, SEARCH_AUTHOR_ID);
    for _ in 0..400 {
        ui_harness.run_steps(1);
        if author_ids(&ui_harness).contains(&ui_result_author_id) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    assert!(
        author_ids(&ui_harness).contains(&ui_result_author_id),
        "production-mounted HandshakeApp Search renders the real managed-backend result"
    );
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut ui_harness,
        "real-search-result-visible",
        serde_json::json!({
            "result_author_id": ui_result_author_id.clone(),
            "document_id": ui_document_id.clone()
        }),
        |tree| {
            json_has_author_id(tree, &ui_result_author_id)
                && json_node_by_author_id(tree, STATUS_AUTHOR_ID)
                    .and_then(|node| node.get("value"))
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|value| value.contains("result"))
        },
    );
    // ── UNOBSCURED VISUAL PROOF (V4 remediation 1) ──────────────────────────────────────────────────
    // A managed-workspace bind starts the canonical FEMS review refresh. When that unrelated transport
    // terminalises as a failure it paints a "Propose to Memory" notice OVER the surface under
    // inspection, which is exactly why the previous frame was rejected as primary visual evidence.
    // Settle it through the product seam that clears ONLY the incidental notice: it REFUSES (returns
    // false) while any operator-owned proposal, emitter, operation, submission, review target or
    // in-flight refresh exists, so this can never hide real state and never disables FEMS globally.
    interconnect_support::settle_incidental_fems_for_capture(
        &mut ui_harness,
        "wp-kernel-012-mt-029-mounted-find-in-files",
    );
    // Give the surface under inspection the full central region: collapse the unrelated chrome that
    // would otherwise crop the query/results/preview/apply/bookmark controls out of the frame.
    ui_harness.state_mut().set_left_rail_open(false);
    ui_harness.state_mut().set_atelier_panel_open(false);
    ui_harness.state_mut().set_bottom_drawer_open(false);
    ui_harness.run_steps(3);

    // Prove the frame is UNOBSCURED against the authoritative tree captured at the same instant:
    // every unrelated FEMS overlay node is absent AND every requested Find-in-Files control is
    // present. This is a structural occlusion check, not a narrative claim about the PNG.
    let capture_tree = argus.inspect(&mut ui_harness);
    for obscuring in [
        handshake_native::fems::memory_proposal::FEMS_PROPOSE_DIALOG_AUTHOR_ID,
        handshake_native::fems::memory_proposal::FEMS_PROPOSE_CONFIRM_AUTHOR_ID,
        handshake_native::fems::memory_proposal::FEMS_PROPOSE_CANCEL_AUTHOR_ID,
        handshake_native::fems::memory_proposal::FEMS_PROPOSE_STATUS_AUTHOR_ID,
        handshake_native::fems::memory_proposal::FEMS_REVIEW_STATUS_AUTHOR_ID,
        handshake_native::fems::memory_proposal::FEMS_REVIEW_REFRESH_RETRY_AUTHOR_ID,
    ] {
        assert!(
            !json_has_author_id(&capture_tree, obscuring),
            "MT-029 visual proof must not be covered by the unrelated FEMS overlay node {obscuring}"
        );
    }
    for required in [
        QUERY_AUTHOR_ID,
        REPLACE_AUTHOR_ID,
        TOGGLE_CASE_AUTHOR_ID,
        TOGGLE_WORD_AUTHOR_ID,
        TOGGLE_REGEX_AUTHOR_ID,
        KIND_FILTER_AUTHOR_ID,
        TAG_FILTER_AUTHOR_ID,
        PATH_FILTER_AUTHOR_ID,
        SEARCH_AUTHOR_ID,
        PREVIEW_REPLACE_AUTHOR_ID,
        APPLY_AUTHOR_ID,
        CANCEL_AUTHOR_ID,
        SAVE_BOOKMARK_AUTHOR_ID,
        STATUS_AUTHOR_ID,
    ] {
        assert!(
            json_has_author_id(&capture_tree, required),
            "MT-029 visual proof must expose the requested control {required}"
        );
    }
    assert!(
        json_has_author_id(&capture_tree, &ui_result_author_id),
        "MT-029 visual proof must expose the real managed-backend result row"
    );

    let managed_png_dir = external_artifact_dir("wp-kernel-012-mt-029");
    std::fs::create_dir_all(&managed_png_dir).expect("create managed MT-029 screenshot directory");
    let managed_png = managed_png_dir.join("MT-029-managed-mounted-runtime.png");
    let managed_png = ui_harness
        .render_proof_frame("MT-029 mounted Find-in-Files runtime frame")
        .map(|managed_image| {
            managed_image.save(&managed_png).unwrap_or_else(|error| {
                panic!(
                    "save managed mounted MT-029 screenshot {}: {error}",
                    managed_png.display()
                )
            });
            assert!(managed_png.is_file(), "managed screenshot PNG must exist");
            managed_png
        });

    argus.click_expect_applied_and_reinspect(&mut ui_harness, &ui_result_author_id);
    let has_tab = |pane_type: PaneType, content_id: &str| {
        ui_harness.state().tab_bar_states().values().any(|bar| {
            bar.tabs.iter().any(|tab| {
                tab.pane_type == pane_type && tab.content_id.as_deref() == Some(content_id)
            })
        })
    };
    assert!(has_tab(PaneType::LoomWikiPage, &ui_document_id));
    let mut rich_editor_loaded = false;
    for _ in 0..400 {
        ui_harness.run_steps(1);
        let rich = ui_harness.state().mounted_rich_state();
        let state = rich.lock().expect("mounted rich editor state");
        let metadata_matches = state.properties.as_ref().is_some_and(|properties| {
            properties.doc_metadata.rich_document_id == ui_document_id
                && properties.doc_metadata.title == ui_title
                && properties.doc_metadata.doc_version == ui_document_version
        });
        let save_matches = state.save.as_ref().is_some_and(|save| {
            save.document_id() == ui_document_id && save.doc_version == ui_document_version
        });
        rich_editor_loaded = metadata_matches
            && save_matches
            && state.current_content_json() == ui_content
            && author_ids(&ui_harness)
                .contains(handshake_native::rich_editor::renderer::RICH_EDITOR_ROOT_AUTHOR_ID)
            && author_ids(&ui_harness).contains(
                &handshake_native::rich_editor::renderer::block_author_id(&[0]),
            );
        drop(state);
        if rich_editor_loaded {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    assert!(
        rich_editor_loaded,
        "result click must mount and backend-load the exact seeded rich document (id/title/content/version) with stable editor root/block author_ids"
    );
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut ui_harness,
        "exact-rich-document-opened",
        serde_json::json!({
            "document_id": ui_document_id.clone(),
            "title": ui_title,
            "doc_version": ui_document_version
        }),
        |tree| {
            json_has_author_id(
                tree,
                handshake_native::rich_editor::renderer::RICH_EDITOR_ROOT_AUTHOR_ID,
            ) && json_has_author_id(
                tree,
                &handshake_native::rich_editor::renderer::block_author_id(&[0]),
            )
        },
    );
    let shell_target = FindInFilesOpenTarget::Document {
        document_id: ui_document_id.clone(),
    };
    assert!(ui_harness.state_mut().dispatch_palette_action_for_test(
        handshake_native::command_registry::CMD_VIEW_FIND_IN_FILES
    ));
    ui_harness.run_steps(2);
    assert!(
        author_ids(&ui_harness).contains(&ui_result_author_id),
        "returning to the production Find pane preserves the managed result state"
    );

    let stale_query = format!("{ui_needle}_STALE");
    argus.set_value_and_reinspect(&mut ui_harness, QUERY_AUTHOR_ID, &stale_query);
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut ui_harness,
        "stale-query-value-visible",
        serde_json::json!({"query": stale_query.clone()}),
        |tree| {
            json_node_by_author_id(tree, QUERY_AUTHOR_ID)
                .and_then(|node| node.get("value"))
                .and_then(serde_json::Value::as_str)
                == Some(stale_query.as_str())
                && json_has_author_id(tree, PREVIEW_REPLACE_AUTHOR_ID)
        },
    );
    // The stale-result guard is a REAL terminal outcome of the Preview action with no HTTP request, so
    // the canonical receipt is a typed, causally bound `rejected` — never `indeterminate` and never a
    // silent no-op. The product-owned error envelope makes the failing effect externally checkable.
    argus.click_expect_typed_rejected_and_reinspect(
        &mut ui_harness,
        PREVIEW_REPLACE_AUTHOR_ID,
        "find-in-files.preview-replace failed",
    );
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut ui_harness,
        "stale-preview-blocked-visible",
        serde_json::json!({"stale_query": stale_query.clone()}),
        |tree| {
            json_node_by_author_id(tree, STATUS_AUTHOR_ID)
                .and_then(|node| node.get("value"))
                .and_then(serde_json::Value::as_str)
                .is_some_and(|value| value.contains("stale") && value.contains("Search again"))
        },
    );
    argus.set_value_and_reinspect(&mut ui_harness, QUERY_AUTHOR_ID, &ui_needle);
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut ui_harness,
        "query-restored-for-preview",
        serde_json::json!({"query": ui_needle.clone()}),
        |tree| {
            json_node_by_author_id(tree, QUERY_AUTHOR_ID)
                .and_then(|node| node.get("value"))
                .and_then(serde_json::Value::as_str)
                == Some(ui_needle.as_str())
        },
    );
    argus.click_and_reinspect(&mut ui_harness, SEARCH_AUTHOR_ID);
    for _ in 0..400 {
        ui_harness.run_steps(1);
        if author_ids(&ui_harness).contains(&ui_result_author_id)
            && ui_harness.state().find_in_files_diagnostics_for_test().1 == false
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut ui_harness,
        "stale-recovery-search-refreshed",
        serde_json::json!({"result_author_id": ui_result_author_id.clone()}),
        |tree| {
            json_has_author_id(tree, &ui_result_author_id)
                && json_node_by_author_id(tree, STATUS_AUTHOR_ID)
                    .and_then(|node| node.get("value"))
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|value| value.contains("result") && !value.contains("stale"))
        },
    );
    argus.set_value_and_reinspect(&mut ui_harness, REPLACE_AUTHOR_ID, &ui_replacement);
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut ui_harness,
        "replacement-value-visible",
        serde_json::json!({"replacement": ui_replacement.clone()}),
        |tree| {
            json_node_by_author_id(tree, REPLACE_AUTHOR_ID)
                .and_then(|node| node.get("value"))
                .and_then(serde_json::Value::as_str)
                == Some(ui_replacement.as_str())
        },
    );
    argus.click_and_reinspect(&mut ui_harness, PREVIEW_REPLACE_AUTHOR_ID);
    for _ in 0..400 {
        ui_harness.run_steps(1);
        let apply_enabled = ui_harness
            .root()
            .children_recursive()
            .find(|node| node.accesskit_node().author_id() == Some(APPLY_AUTHOR_ID))
            .is_some_and(|node| !node.accesskit_node().is_disabled());
        if apply_enabled {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    let apply_enabled = ui_harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(APPLY_AUTHOR_ID))
        .is_some_and(|node| !node.accesskit_node().is_disabled());
    assert!(
        apply_enabled,
        "mounted Preview produces an applicable real plan"
    );
    let ui_preview_author_id = preview_author_id(&ui_document_id);
    let ui_preview_before_author_id = preview_before_author_id(&ui_document_id);
    let ui_preview_after_author_id = preview_after_author_id(&ui_document_id);
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut ui_harness,
        "persisted-preview-visible",
        serde_json::json!({
            "preview_author_id": ui_preview_author_id.clone(),
            "document_id": ui_document_id.clone(),
            "replacement": ui_replacement.clone()
        }),
        |tree| {
            json_has_author_id(tree, &ui_preview_author_id)
                && json_node_by_author_id(tree, STATUS_AUTHOR_ID)
                    .and_then(|node| node.get("value"))
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|value| value.contains("Previewed 1"))
        },
    );
    argus.click_and_reinspect(&mut ui_harness, &ui_preview_author_id);
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut ui_harness,
        "preview-before-after-visible",
        serde_json::json!({
            "before_author_id": ui_preview_before_author_id.clone(),
            "after_author_id": ui_preview_after_author_id.clone(),
            "needle": ui_needle.clone(),
            "replacement": ui_replacement.clone(),
        }),
        |tree| {
            json_node_by_author_id(tree, &ui_preview_before_author_id)
                .and_then(|node| node.get("value"))
                .and_then(serde_json::Value::as_str)
                .is_some_and(|value| value.contains(&ui_needle))
                && json_node_by_author_id(tree, &ui_preview_after_author_id)
                    .and_then(|node| node.get("value"))
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|value| value.contains(&ui_replacement))
        },
    );

    // Editing replacement invalidates the exact preview and leaves destructive Apply unreachable.
    // The producer is refetched before/after to prove that this stale UI path performs no mutation.
    let stale_apply_before = live.get_json(&format!("/knowledge/documents/{ui_document_id}"));
    let stale_replacement = format!("{ui_replacement}_STALE");
    argus.set_value_and_reinspect(&mut ui_harness, REPLACE_AUTHOR_ID, &stale_replacement);
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut ui_harness,
        "stale-apply-disabled-visible",
        serde_json::json!({"stale_replacement": stale_replacement}),
        |tree| {
            json_node_by_author_id(tree, STATUS_AUTHOR_ID)
                .and_then(|node| node.get("value"))
                .and_then(serde_json::Value::as_str)
                .is_some_and(|value| value.contains("Preview is stale"))
                && !json_has_author_id(tree, &ui_preview_author_id)
        },
    );
    let stale_apply_enabled = ui_harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(APPLY_AUTHOR_ID))
        .is_some_and(|node| !node.accesskit_node().is_disabled());
    assert!(!stale_apply_enabled, "stale Apply must remain disabled");
    let stale_apply_after = live.get_json(&format!("/knowledge/documents/{ui_document_id}"));
    assert_eq!(
        stale_apply_after["document"]["doc_version"], stale_apply_before["document"]["doc_version"],
        "editing replacement after preview cannot mutate producer version"
    );
    assert_eq!(
        stale_apply_after["document"]["content_json"],
        stale_apply_before["document"]["content_json"],
        "editing replacement after preview cannot mutate producer content"
    );

    argus.set_value_and_reinspect(&mut ui_harness, REPLACE_AUTHOR_ID, &ui_replacement);
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut ui_harness,
        "fresh-replacement-restored",
        serde_json::json!({"replacement": ui_replacement.clone()}),
        |tree| {
            json_node_by_author_id(tree, REPLACE_AUTHOR_ID)
                .and_then(|node| node.get("value"))
                .and_then(serde_json::Value::as_str)
                == Some(ui_replacement.as_str())
        },
    );
    argus.click_and_reinspect(&mut ui_harness, PREVIEW_REPLACE_AUTHOR_ID);
    for _ in 0..400 {
        ui_harness.run_steps(1);
        let apply_enabled = ui_harness
            .root()
            .children_recursive()
            .find(|node| node.accesskit_node().author_id() == Some(APPLY_AUTHOR_ID))
            .is_some_and(|node| !node.accesskit_node().is_disabled());
        if apply_enabled {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut ui_harness,
        "fresh-preview-restored-before-apply",
        serde_json::json!({"preview_author_id": ui_preview_author_id.clone()}),
        |tree| {
            json_has_author_id(tree, &ui_preview_author_id)
                && json_node_by_author_id(tree, STATUS_AUTHOR_ID)
                    .and_then(|node| node.get("value"))
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|value| value.contains("Previewed 1"))
        },
    );
    argus.click_expect_applied_and_reinspect(&mut ui_harness, APPLY_AUTHOR_ID);
    let mut ui_apply_persisted = false;
    for _ in 0..400 {
        ui_harness.run_steps(1);
        let persisted = live.get_json(&format!("/knowledge/documents/{ui_document_id}"));
        if persisted["document"]["content_json"]
            .to_string()
            .contains(&ui_replacement)
        {
            ui_apply_persisted = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    assert!(
        ui_apply_persisted,
        "mounted UI Apply persisted through the real save route"
    );
    for _ in 0..400 {
        ui_harness.run_steps(1);
        if !author_ids(&ui_harness).contains(&ui_result_author_id) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    assert!(
        !author_ids(&ui_harness).contains(&ui_result_author_id),
        "mounted Apply terminal delivery auto-refreshes the visible result"
    );
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut ui_harness,
        "apply-receipt-persisted-and-result-refreshed",
        serde_json::json!({
            "document_id": ui_document_id.clone(),
            "old_result_author_id": ui_result_author_id.clone()
        }),
        |tree| {
            !json_has_author_id(tree, &ui_result_author_id)
                && json_node_by_author_id(tree, STATUS_AUTHOR_ID)
                    .and_then(|node| node.get("value"))
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|value| value.contains("Applied 1") && value.contains("receipt"))
        },
    );

    // Canonical mounted error and empty-result states, followed by a clean recovery. Invalid regex is
    // local and bounded; the guaranteed miss still traverses the real managed search route.
    argus.click_and_reinspect(&mut ui_harness, TOGGLE_REGEX_AUTHOR_ID);
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut ui_harness,
        "regex-toggle-enabled",
        serde_json::json!({"regex": true, "state_author_id": TOGGLE_REGEX_STATE_AUTHOR_ID}),
        |tree| {
            json_node_by_author_id(tree, TOGGLE_REGEX_STATE_AUTHOR_ID)
                .and_then(|node| node.get("value"))
                .and_then(serde_json::Value::as_str)
                == Some("true")
        },
    );
    argus.set_value_and_reinspect(&mut ui_harness, QUERY_AUTHOR_ID, "[");
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut ui_harness,
        "invalid-regex-query-visible",
        serde_json::json!({"query": "["}),
        |tree| {
            json_node_by_author_id(tree, QUERY_AUTHOR_ID)
                .and_then(|node| node.get("value"))
                .and_then(serde_json::Value::as_str)
                == Some("[")
        },
    );
    // An invalid regex is a REAL terminal failure of the Search action with no backend round-trip, so
    // the canonical receipt is a typed, causally bound `rejected`.
    argus.click_expect_typed_rejected_and_reinspect(
        &mut ui_harness,
        SEARCH_AUTHOR_ID,
        "find-in-files.search failed",
    );
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut ui_harness,
        "invalid-regex-error-visible",
        serde_json::json!({"query": "[", "regex": true}),
        |tree| {
            json_node_by_author_id(tree, STATUS_AUTHOR_ID)
                .and_then(|node| node.get("value"))
                .and_then(serde_json::Value::as_str)
                .is_some_and(|value| value.to_ascii_lowercase().contains("regex"))
        },
    );
    argus.click_and_reinspect(&mut ui_harness, TOGGLE_REGEX_AUTHOR_ID);
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut ui_harness,
        "regex-toggle-disabled",
        serde_json::json!({"regex": false, "state_author_id": TOGGLE_REGEX_STATE_AUTHOR_ID}),
        |tree| {
            json_node_by_author_id(tree, TOGGLE_REGEX_STATE_AUTHOR_ID)
                .and_then(|node| node.get("value"))
                .and_then(serde_json::Value::as_str)
                == Some("false")
        },
    );
    let guaranteed_miss = format!("MT029_NO_MATCH_{unique}");
    argus.set_value_and_reinspect(&mut ui_harness, QUERY_AUTHOR_ID, &guaranteed_miss);
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut ui_harness,
        "managed-empty-query-visible",
        serde_json::json!({"query": guaranteed_miss.clone()}),
        |tree| {
            json_node_by_author_id(tree, QUERY_AUTHOR_ID)
                .and_then(|node| node.get("value"))
                .and_then(serde_json::Value::as_str)
                == Some(guaranteed_miss.as_str())
        },
    );
    argus.click_and_reinspect(&mut ui_harness, SEARCH_AUTHOR_ID);
    for _ in 0..400 {
        ui_harness.run_steps(1);
        let (_, loading, _, count) = ui_harness.state().find_in_files_diagnostics_for_test();
        if !loading && count == 0 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut ui_harness,
        "managed-empty-result-visible",
        serde_json::json!({"query": guaranteed_miss.clone()}),
        |tree| {
            json_node_by_author_id(tree, STATUS_AUTHOR_ID)
                .and_then(|node| node.get("value"))
                .and_then(serde_json::Value::as_str)
                .is_some_and(|value| value.contains("0 result"))
        },
    );

    // ── Canonical Argus bookmark save / restore / remove + cancellation (V4 remediation 2) ──────────
    // Every one of these is driven through the SAME production-mounted HandshakeApp and the same
    // localhost Argus transport, so each emits a matrix row bound to a product-side terminal receipt.
    // The saved bookmark id is derived from the live panel fields, exactly as the production
    // `SearchBookmark::stable_id()` derives it, so nothing is seeded behind the UI.
    //
    // The bookmark row author ids are `find-in-files.bookmark-{restore,remove}.<hex(stable_id)>`, and the
    // stable id hex-encodes the query. The canonical `handshake.click-completion/v1` boundary bounds an
    // acknowledgement's `pending_target` at 256 bytes (`MAX_CLICK_COMPLETION_AUTHOR_BYTES`), so a
    // bookmark whose query is long enough pushes its row id past that limit and its observer token is
    // rejected as malformed — an `indeterminate` receipt. This proof therefore bookmarks a REALISTIC
    // short query (the operator case), and the residual long-id limit is reported as a bounded
    // HBR-VIS gap rather than papered over by weakening the boundary.
    let bookmark_query = format!("M29B{:x}", std::process::id());
    assert!(
        bookmark_query.len() <= 22,
        "the bookmarked query must keep its derived row author ids inside the 256-byte canonical          completion target bound"
    );
    argus.set_value_and_reinspect(&mut ui_harness, QUERY_AUTHOR_ID, &bookmark_query);
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut ui_harness,
        "bookmark-query-visible",
        serde_json::json!({"query": bookmark_query.clone()}),
        |tree| {
            json_node_by_author_id(tree, QUERY_AUTHOR_ID)
                .and_then(|node| node.get("value"))
                .and_then(serde_json::Value::as_str)
                == Some(bookmark_query.as_str())
        },
    );
    let mounted_bookmark = {
        let mut bookmark = SearchBookmark {
            id: String::new(),
            label: String::new(),
            query: bookmark_query.clone(),
            kind: KindFilter::All,
            tag_filter: String::new(),
            path_filter: String::new(),
            case_sensitive: false,
            whole_word: false,
            is_regex: false,
            saved_at: "2026-08-05T00:00:00Z".to_owned(),
        };
        bookmark.id = bookmark.stable_id();
        bookmark.label = bookmark.display_label();
        bookmark
    };
    let mounted_restore_author_id = bookmark_restore_author_id(&mounted_bookmark.id);
    let mounted_remove_author_id = bookmark_remove_author_id(&mounted_bookmark.id);
    assert!(
        mounted_remove_author_id.len() <= 256 && mounted_restore_author_id.len() <= 256,
        "derived bookmark row ids must fit the canonical completion target bound: restore={} remove={}",
        mounted_restore_author_id.len(),
        mounted_remove_author_id.len()
    );
    argus.click_expect_applied_and_reinspect(&mut ui_harness, SAVE_BOOKMARK_AUTHOR_ID);
    for _ in 0..400 {
        ui_harness.run_steps(1);
        let ids = author_ids(&ui_harness);
        if ids.contains(&mounted_restore_author_id) && ids.contains(&mounted_remove_author_id) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut ui_harness,
        "mounted-bookmark-saved-with-receipt",
        serde_json::json!({
            "bookmark_id": mounted_bookmark.id.clone(),
            "restore_author_id": mounted_restore_author_id.clone(),
            "remove_author_id": mounted_remove_author_id.clone(),
        }),
        |tree| {
            json_has_author_id(tree, &mounted_restore_author_id)
                && json_has_author_id(tree, &mounted_remove_author_id)
                && json_node_by_author_id(tree, BOOKMARK_STATUS_AUTHOR_ID)
                    .and_then(|node| node.get("value"))
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|value| {
                        value
                            .split_once("receipt: ")
                            .map(|(_, tail)| tail.trim())
                            .is_some_and(|receipt| !receipt.is_empty())
                    })
        },
    );
    // A real backend GET must independently see the bookmark the mounted Save persisted.
    let mounted_bookmark_cell: BookmarkStateCell =
        Arc::new(Mutex::new(std::collections::VecDeque::new()));
    search_client.load_bookmarks(
        &workspace_id,
        operation_stamp(&workspace_id, FindInFilesOperation::BookmarkLoad, 81),
        Arc::clone(&mounted_bookmark_cell),
    );
    let (mounted_saved_blob, _, _) =
        wait_bookmark(&mounted_bookmark_cell).expect("mounted bookmark save backend GET");
    let mounted_saved_state =
        parse_bookmark_state(&mounted_saved_blob).expect("strict mounted bookmark payload");
    assert!(
        mounted_saved_state
            .iter()
            .any(|saved| saved.id == mounted_bookmark.id),
        "the mounted Bookmark Search action persisted the exact semantic bookmark id"
    );

    // Move the query away so Restore has something real to repopulate, then restore through the UI.
    let bookmark_scratch_query = format!("MT029_BOOKMARK_SCRATCH_{unique}");
    argus.set_value_and_reinspect(&mut ui_harness, QUERY_AUTHOR_ID, &bookmark_scratch_query);
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut ui_harness,
        "bookmark-scratch-query-visible",
        serde_json::json!({"query": bookmark_scratch_query.clone()}),
        |tree| {
            json_node_by_author_id(tree, QUERY_AUTHOR_ID)
                .and_then(|node| node.get("value"))
                .and_then(serde_json::Value::as_str)
                == Some(bookmark_scratch_query.as_str())
        },
    );
    argus.click_expect_applied_and_reinspect(&mut ui_harness, &mounted_restore_author_id);
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut ui_harness,
        "mounted-bookmark-restored",
        serde_json::json!({
            "bookmark_id": mounted_bookmark.id.clone(),
            "restored_query": bookmark_query.clone(),
        }),
        |tree| {
            json_node_by_author_id(tree, QUERY_AUTHOR_ID)
                .and_then(|node| node.get("value"))
                .and_then(serde_json::Value::as_str)
                == Some(bookmark_query.as_str())
        },
    );
    argus.click_expect_applied_and_reinspect(&mut ui_harness, &mounted_remove_author_id);
    for _ in 0..400 {
        ui_harness.run_steps(1);
        if !author_ids(&ui_harness).contains(&mounted_remove_author_id) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut ui_harness,
        "mounted-bookmark-removed",
        serde_json::json!({"bookmark_id": mounted_bookmark.id.clone()}),
        |tree| {
            !json_has_author_id(tree, &mounted_remove_author_id)
                && !json_has_author_id(tree, &mounted_restore_author_id)
        },
    );
    let mounted_absence_cell: BookmarkStateCell =
        Arc::new(Mutex::new(std::collections::VecDeque::new()));
    search_client.load_bookmarks(
        &workspace_id,
        operation_stamp(&workspace_id, FindInFilesOperation::BookmarkLoad, 82),
        Arc::clone(&mounted_absence_cell),
    );
    let (mounted_absence_blob, _, _) =
        wait_bookmark(&mounted_absence_cell).expect("mounted bookmark remove backend GET");
    let mounted_absence_state =
        parse_bookmark_state(&mounted_absence_blob).expect("strict mounted absence payload");
    assert!(
        mounted_absence_state
            .iter()
            .all(|saved| saved.id != mounted_bookmark.id),
        "the mounted Remove action persisted exact-bookmark absence"
    );

    // Cancellation through the production control. No destructive save is in flight here, so the whole
    // terminal effect is the local preview clear — and the completion observer says exactly that
    // instead of pretending a mutation was cancelled.
    argus.click_expect_applied_and_reinspect(&mut ui_harness, CANCEL_AUTHOR_ID);
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut ui_harness,
        "mounted-cancel-cleared-preview",
        serde_json::json!({"expected_status": "Replacement preview cleared."}),
        |tree| {
            json_node_by_author_id(tree, STATUS_AUTHOR_ID)
                .and_then(|node| node.get("value"))
                .and_then(serde_json::Value::as_str)
                .is_some_and(|value| value.contains("Replacement preview cleared"))
        },
    );

    // Exhaust every supported result target as a REAL rendered row in the production-mounted app.
    // The seam only supplies backend-shaped result state; each route still requires AccessKit row
    // discovery, an operator-equivalent click, the production frame drain, and a resulting shell tab.
    let production_hits = vec![
        producer_hit(
            "document",
            "knowledge_entity",
            &ui_document_id,
            "Managed document",
            serde_json::json!({
                "authority_table": "knowledge_rich_documents",
                "rich_document_id": ui_document_id,
                "document_id": null,
                "schema_version": "hsk.knowledge.rich_document@1",
                "doc_version": 1,
                "authority_label": "operator"
            }),
            None,
        ),
        producer_block_hit("loom_block", "BLK-BLOCK-MT029", "Block", "note"),
        producer_block_hit("file", "BLK-FILE-MT029", "File", "file"),
        producer_block_hit("tag_hub", "BLK-TAG-MT029", "Tag", "tag_hub"),
        producer_hit(
            "symbol",
            "knowledge_entity",
            "SYM-MT029",
            "Symbol",
            serde_json::json!({
                "authority_table": "knowledge_entities",
                "entity_key": "symbol::SYM-MT029",
                "detection_provenance": {"source": "producer-fixture"}
            }),
            None,
        ),
        producer_hit(
            "work_packet",
            "knowledge_entity",
            "wp-entity",
            "WP",
            serde_json::json!({
                "authority_table": "knowledge_entities",
                "entity_key": "WP-KERNEL-012",
                "detection_provenance": {"source": "producer-fixture"}
            }),
            None,
        ),
        producer_hit(
            "micro_task",
            "knowledge_entity",
            "mt-entity",
            "MT",
            serde_json::json!({
                "authority_table": "knowledge_entities",
                "entity_key": "WP-KERNEL-012/MT-029",
                "detection_provenance": {"source": "producer-fixture"}
            }),
            None,
        ),
        producer_hit(
            "user_manual_page",
            "user_manual_page",
            "native-editors",
            "UserManual",
            serde_json::json!({
                "authority_table": "user_manual_pages",
                "page_slug": "native-editors"
            }),
            None,
        ),
        producer_hit(
            "wiki_page",
            "wiki_page",
            "WIKI-MT029",
            "Wiki projection",
            serde_json::json!({
                "authority_table": "knowledge_wiki_projections",
                "projection_id": "WIKI-MT029",
                "page_type": "reference",
                "rebuild_status": "current"
            }),
            None,
        ),
        producer_block_hit("loom_block", "BLK-VIEW-MT029", "View", "view_def"),
    ];
    let mut production_app_route_count = 0usize;
    for search_hit in production_hits {
        let author_id = result_author_id(&search_hit.source_kind, &search_hit.ref_id);
        assert_eq!(
            hit_identity_from_result_author_id(&author_id),
            Some((search_hit.source_kind.clone(), search_hit.ref_id.clone())),
            "every mounted production result retains its exact reversible row identity"
        );
        ui_harness
            .state_mut()
            .mount_find_in_files_results_for_test(vec![search_hit]);
        assert!(ui_harness.state_mut().dispatch_palette_action_for_test(
            handshake_native::command_registry::CMD_VIEW_FIND_IN_FILES
        ));
        ui_harness.run_steps(2);
        assert!(
            author_ids(&ui_harness).contains(&author_id),
            "production-mounted Find pane exposes row {author_id}"
        );
        click_author_id(&ui_harness, &author_id);
        ui_harness.run_steps(1);
        production_app_route_count += 1;
    }
    assert_eq!(production_app_route_count, 10);
    let has_production_tab = |pane_type: PaneType, content_id: &str| {
        ui_harness.state().tab_bar_states().values().any(|bar| {
            bar.tabs.iter().any(|tab| {
                tab.pane_type == pane_type && tab.content_id.as_deref() == Some(content_id)
            })
        })
    };
    assert!(has_production_tab(PaneType::LoomWikiPage, &ui_document_id));
    assert!(has_production_tab(PaneType::LoomBlock, "BLK-BLOCK-MT029"));
    assert!(has_production_tab(PaneType::LoomBlock, "BLK-FILE-MT029"));
    assert!(has_production_tab(PaneType::LoomBlock, "BLK-TAG-MT029"));
    assert!(
        ui_harness.state().tab_bar_states().values().any(|bar| bar
            .tabs
            .iter()
            .any(|tab| tab.pane_type == PaneType::CodeSymbol)),
        "a symbol hit routes to the mounted code surface"
    );
    assert!(
        !has_production_tab(PaneType::CodeSymbol, "SYM-MT029"),
        "symbol routing must not leave the transient symbol-id tab removed by MT-034"
    );
    assert!(has_production_tab(PaneType::KernelDcc, "WP:WP-KERNEL-012"));
    assert!(has_production_tab(
        PaneType::KernelDcc,
        "MT:WP-KERNEL-012:MT-029"
    ));
    assert!(has_production_tab(PaneType::UserManual, "native-editors"));
    assert!(has_production_tab(
        placeholder_pane_type(BLOCK_COLLECTIONS_PANE_LABEL),
        "BLK-VIEW-MT029"
    ));
    assert!(has_production_tab(
        placeholder_pane_type(WIKI_PAGE_PANE_LABEL),
        "WIKI-MT029"
    ));
    let regex =
        handshake_native::find_in_files::compile_search_regex(&needle, MatchOptions::default())
            .expect("literal search regex");
    let preview_cell: FindReplaceCell = Arc::new(Mutex::new(std::collections::VecDeque::new()));
    doc_client.preview_replace(
        &workspace_id,
        document_ids[..2].to_vec(),
        regex,
        replacement.clone(),
        MatchOptions::default(),
        "preview-key".to_owned(),
        operation_stamp(&workspace_id, FindInFilesOperation::Preview, 2),
        Arc::clone(&preview_cell),
    );
    let plans = match wait_replace(&preview_cell) {
        ReplaceDelivery::Preview { plans, .. } => plans,
        other => panic!("expected managed preview, got {other:?}"),
    };
    assert_eq!(plans.len(), 2);
    assert_eq!(
        plans[0].match_count, 2,
        "the first rich document has one text-node match plus one attrs.code match"
    );
    assert_eq!(
        plans[1].match_count, 1,
        "the second rich document has exactly one text-node match"
    );
    for plan in &plans {
        assert_eq!(plan.before_sha256.len(), 64);
        assert_eq!(plan.after_sha256.len(), 64);
        assert_ne!(plan.before_sha256, plan.after_sha256);
        assert!(
            !plan.before_preview.is_empty() && plan.before_preview.contains(&needle),
            "plan {} exposes meaningful before context",
            plan.document_id
        );
        assert!(
            !plan.after_preview.is_empty() && plan.after_preview.contains(&replacement),
            "plan {} exposes the replacement in after context",
            plan.document_id
        );
        assert_eq!(
            plan.match_previews.len(),
            plan.match_count,
            "plan {} emits one preview per actual match",
            plan.document_id
        );
        for preview in &plan.match_previews {
            assert!(
                !preview.before_preview.is_empty() && preview.before_preview.contains(&needle),
                "each match preview exposes nonempty before context"
            );
            assert!(
                !preview.after_preview.is_empty() && preview.after_preview.contains(&replacement),
                "each match preview exposes the replacement in nonempty after context"
            );
        }
    }

    // Advance the second document after preview. Apply must save plan 0, then retain a typed conflict
    // receipt for plan 1 without overwriting the externally persisted content.
    let conflict_plan = &plans[1];
    let conflict_content = serde_json::json!({
        "type":"doc",
        "content":[{"type":"paragraph","content":[{"type":"text","text":"external concurrent edit"}]}]
    });
    live.put_json(
        &format!("/knowledge/documents/{}/save", conflict_plan.document_id),
        &serde_json::json!({
            "expected_version": conflict_plan.expected_version,
            "content_json": conflict_content
        }),
    );

    let apply_cell: FindReplaceCell = Arc::new(Mutex::new(std::collections::VecDeque::new()));
    doc_client.apply_plans(
        &workspace_id,
        plans.clone(),
        operation_stamp(&workspace_id, FindInFilesOperation::Apply, 3),
        Arc::clone(&apply_cell),
        Arc::new(AtomicBool::new(false)),
    );
    let (saved_receipts, audit_receipts) = match wait_replace(&apply_cell) {
        ReplaceDelivery::AppliedPartial {
            receipts,
            audit_receipts,
            error,
        } => {
            assert!(error.contains("conflict"));
            (receipts, audit_receipts)
        }
        other => panic!("expected managed partial conflict, got {other:?}"),
    };
    assert_eq!(saved_receipts.len(), 1);
    assert!(
        saved_receipts
            .iter()
            .all(|receipt| !receipt.trim().is_empty()),
        "committed save receipt ids are never blank"
    );
    assert_eq!(audit_receipts.len(), 2);
    assert_eq!(audit_receipts[0].outcome, ReplaceAuditOutcome::Saved);
    assert_eq!(audit_receipts[1].outcome, ReplaceAuditOutcome::Conflict);
    assert_eq!(audit_receipts[0].before_sha256, plans[0].before_sha256);
    assert_eq!(audit_receipts[0].after_sha256, plans[0].after_sha256);
    assert_eq!(
        audit_receipts[0].save_receipt_event_id.as_deref(),
        Some(saved_receipts[0].as_str()),
        "audit row carries the same nonblank producer receipt id"
    );
    let saved_event = interconnect_support::event_ledger_payload(
        &live,
        "knowledge_rich_document",
        &plans[0].document_id,
        &saved_receipts[0],
    );
    assert_eq!(saved_event["_event_id"], saved_receipts[0]);
    assert_eq!(
        saved_event["_event_type"], "KNOWLEDGE_RICH_DOCUMENT_SAVED",
        "receipt resolves to the canonical rich-document save event family"
    );
    assert_eq!(
        saved_event["_aggregate_type"], "knowledge_rich_document",
        "receipt resolves to the canonical aggregate type"
    );
    assert_eq!(
        saved_event["_aggregate_id"], plans[0].document_id,
        "receipt resolves to the exact document saved by this plan"
    );
    assert_eq!(
        saved_event["workspace_id"], workspace_id,
        "receipt remains scoped to the managed workspace"
    );
    assert_eq!(
        saved_event["content_hash"], plans[0].after_sha256,
        "EventLedger content hash equals the applied plan and persisted reload"
    );
    assert_eq!(audit_receipts[1].before_sha256, plans[1].before_sha256);
    assert_eq!(audit_receipts[1].after_sha256, plans[1].after_sha256);
    assert!(
        audit_receipts[1].save_receipt_event_id.is_none(),
        "a conflict must never invent a successful save receipt"
    );

    // Canonical mounted partial Apply: Search and Preview are driven through Argus, the second exact
    // production plan is externally advanced, and Apply must accept its stamped delivery through the
    // normal mounted poller while preserving the first save receipt beside the conflict.
    let mounted_partial_needle = format!("MT029_MOUNTED_PARTIAL_{unique}");
    let mounted_partial_replacement = format!("MT029_MOUNTED_PARTIAL_REPLACED_{unique}");
    let mounted_partial_a = create_doc(
        "MT-029 mounted partial A",
        serde_json::json!({
            "type":"doc",
            "content":[{"type":"paragraph","content":[{"type":"text","text":mounted_partial_needle}]}]
        }),
    );
    let mounted_partial_b = create_doc(
        "MT-029 mounted partial B",
        serde_json::json!({
            "type":"doc",
            "content":[{"type":"paragraph","content":[{"type":"text","text":mounted_partial_needle}]}]
        }),
    );
    let mounted_partial_ids = [
        mounted_partial_a["document"]["rich_document_id"]
            .as_str()
            .expect("mounted partial A id")
            .to_owned(),
        mounted_partial_b["document"]["rich_document_id"]
            .as_str()
            .expect("mounted partial B id")
            .to_owned(),
    ];
    assert!(ui_harness.state_mut().dispatch_palette_action_for_test(
        handshake_native::command_registry::CMD_VIEW_FIND_IN_FILES
    ));
    ui_harness.run_steps(2);
    argus.set_value_and_reinspect(&mut ui_harness, QUERY_AUTHOR_ID, &mounted_partial_needle);
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut ui_harness,
        "mounted-partial-query-visible",
        serde_json::json!({"query": mounted_partial_needle.clone()}),
        |tree| {
            json_node_by_author_id(tree, QUERY_AUTHOR_ID)
                .and_then(|node| node.get("value"))
                .and_then(serde_json::Value::as_str)
                == Some(mounted_partial_needle.as_str())
        },
    );
    argus.click_and_reinspect(&mut ui_harness, SEARCH_AUTHOR_ID);
    let mounted_partial_result_ids = mounted_partial_ids
        .iter()
        .map(|document_id| result_author_id("document", document_id))
        .collect::<Vec<_>>();
    for _ in 0..400 {
        ui_harness.run_steps(1);
        let ids = author_ids(&ui_harness);
        if mounted_partial_result_ids
            .iter()
            .all(|author_id| ids.contains(author_id))
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut ui_harness,
        "mounted-partial-results-visible",
        serde_json::json!({"result_author_ids": mounted_partial_result_ids.clone()}),
        |tree| {
            mounted_partial_result_ids
                .iter()
                .all(|author_id| json_has_author_id(tree, author_id))
        },
    );
    argus.set_value_and_reinspect(
        &mut ui_harness,
        REPLACE_AUTHOR_ID,
        &mounted_partial_replacement,
    );
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut ui_harness,
        "mounted-partial-replacement-visible",
        serde_json::json!({"replacement": mounted_partial_replacement.clone()}),
        |tree| {
            json_node_by_author_id(tree, REPLACE_AUTHOR_ID)
                .and_then(|node| node.get("value"))
                .and_then(serde_json::Value::as_str)
                == Some(mounted_partial_replacement.as_str())
        },
    );
    argus.click_and_reinspect(&mut ui_harness, PREVIEW_REPLACE_AUTHOR_ID);
    for _ in 0..400 {
        ui_harness.run_steps(1);
        if ui_harness
            .state()
            .find_in_files_preview_document_ids_for_test()
            .len()
            == 2
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    let mounted_preview_order = ui_harness
        .state()
        .find_in_files_preview_document_ids_for_test();
    assert_eq!(
        mounted_preview_order.len(),
        2,
        "mounted Preview must produce the exact two action-driven plans"
    );
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut ui_harness,
        "mounted-partial-preview-visible",
        serde_json::json!({"preview_document_ids": mounted_preview_order.clone()}),
        |tree| {
            mounted_preview_order
                .iter()
                .all(|document_id| json_has_author_id(tree, &preview_author_id(document_id)))
        },
    );
    let mounted_saved_id = mounted_preview_order[0].clone();
    let mounted_conflict_id = mounted_preview_order[1].clone();
    let mounted_conflict_before =
        live.get_json(&format!("/knowledge/documents/{mounted_conflict_id}"));
    let mounted_conflict_version = mounted_conflict_before["document"]["doc_version"]
        .as_u64()
        .expect("mounted conflict version");
    let mounted_conflict_content = serde_json::json!({
        "type":"doc",
        "content":[{"type":"paragraph","content":[{"type":"text","text":"mounted external concurrent edit"}]}]
    });
    live.put_json(
        &format!("/knowledge/documents/{mounted_conflict_id}/save"),
        &serde_json::json!({
            "expected_version": mounted_conflict_version,
            "content_json": mounted_conflict_content
        }),
    );
    argus.click_expect_applied_and_reinspect(&mut ui_harness, APPLY_AUTHOR_ID);
    for _ in 0..400 {
        ui_harness.run_steps(1);
        let partial_terminal_visible = ui_harness
            .root()
            .children_recursive()
            .find(|node| node.accesskit_node().author_id() == Some(STATUS_AUTHOR_ID))
            .and_then(|node| node.accesskit_node().value())
            .is_some_and(|value| {
                let lower = value.to_ascii_lowercase();
                lower.contains("receipts:")
                    && lower.contains("failure")
                    && lower.contains("conflict")
            });
        if partial_terminal_visible {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut ui_harness,
        "mounted-partial-receipt-and-conflict-visible",
        serde_json::json!({
            "saved_document_id": mounted_saved_id.clone(),
            "conflict_document_id": mounted_conflict_id.clone()
        }),
        |tree| {
            json_node_by_author_id(tree, STATUS_AUTHOR_ID)
                .and_then(|node| node.get("value"))
                .and_then(serde_json::Value::as_str)
                .is_some_and(|value| {
                    let lower = value.to_ascii_lowercase();
                    let receipt = value
                        .split_once("receipts:")
                        .and_then(|(_, tail)| tail.split(';').next())
                        .map(str::trim)
                        .unwrap_or_default();
                    !receipt.is_empty()
                        && receipt != "<none>"
                        && lower.contains("failure")
                        && lower.contains("conflict")
                })
        },
    );
    let mounted_saved = live.get_json(&format!("/knowledge/documents/{mounted_saved_id}"));
    assert!(
        mounted_saved["document"]["content_json"]
            .to_string()
            .contains(&mounted_partial_replacement),
        "the first mounted plan persisted before the later conflict"
    );
    let mounted_conflict = live.get_json(&format!("/knowledge/documents/{mounted_conflict_id}"));
    assert_eq!(
        mounted_conflict["document"]["content_json"], mounted_conflict_content,
        "the action-driven mounted conflict never overwrites the concurrent edit"
    );
    let reloaded_saved = live.get_json(&format!("/knowledge/documents/{}", plans[0].document_id));
    let persisted_saved = &reloaded_saved["document"]["content_json"];
    assert_eq!(content_json_sha256(persisted_saved), plans[0].after_sha256);
    assert!(persisted_saved.to_string().contains(&replacement));
    assert_eq!(
        persisted_saved["content"][1]["attrs"]["code"], replacement,
        "attrs.code participates in replacement"
    );
    assert_eq!(
        persisted_saved["content"][1]["attrs"]["preserve"], "attrs-marker",
        "non-code attrs remain byte-for-byte semantically preserved"
    );
    assert_eq!(
        persisted_saved["content"][2], preserved_image,
        "backend-valid typed image nodes remain verbatim across replacement"
    );
    assert_eq!(
        persisted_saved["content"][3], preserved_table,
        "backend-valid structured table nodes remain verbatim across replacement"
    );
    let reloaded_conflict =
        live.get_json(&format!("/knowledge/documents/{}", plans[1].document_id));
    assert_eq!(
        reloaded_conflict["document"]["content_json"], conflict_content,
        "conflicting document is never overwritten"
    );

    // Drive the production panel state machine through a complete successful Apply. The query is first
    // changed after search to prove stale Preview rejection against real backend results, then recovered.
    // Apply completion must automatically issue the same search and finish with zero old-needle hits.
    let full_needle = format!("MT029_FULL_{unique}");
    let full_replacement = format!("MT029_FULL_REPLACED_{unique}");
    let full_created = create_doc(
        "MT-029 full apply",
        serde_json::json!({
            "type":"doc",
            "content":[{"type":"paragraph","content":[{"type":"text","text":full_needle}]}]
        }),
    );
    let full_document_id = full_created["document"]["rich_document_id"]
        .as_str()
        .expect("full document id")
        .to_owned();
    let mut mounted_state = FindInFilesPanelState::new();
    mounted_state.bind_workspace(Some(&workspace_id), 1);
    mounted_state.kind = KindFilter::Document;
    mounted_state.query = full_needle.clone();
    mounted_state.replacement = full_replacement.clone();
    mounted_state.run_search(&search_client, Some(&workspace_id));
    wait_panel_idle(&mut mounted_state, &search_client, &workspace_id);
    assert_eq!(mounted_state.results.len(), 1);
    mounted_state.query.push_str("_STALE");
    mounted_state.run_preview_replace(&doc_client, Some(&workspace_id));
    assert!(
        mounted_state
            .replace_status
            .as_deref()
            .is_some_and(|status| status.contains("stale")),
        "changed real search input blocks stale preview"
    );
    mounted_state.query = full_needle.clone();
    mounted_state.run_search(&search_client, Some(&workspace_id));
    wait_panel_idle(&mut mounted_state, &search_client, &workspace_id);
    mounted_state.run_preview_replace(&doc_client, Some(&workspace_id));
    wait_panel_idle(&mut mounted_state, &search_client, &workspace_id);
    assert_eq!(mounted_state.preview_plans.len(), 1);
    mounted_state.run_apply(&doc_client, Some(&workspace_id));
    wait_panel_idle(&mut mounted_state, &search_client, &workspace_id);
    assert!(
        mounted_state.results.is_empty(),
        "successful Apply automatically refreshes and removes the old search hit"
    );
    let refreshed_search_key = mounted_state.current_search_key();
    assert_eq!(
        mounted_state.result_set_key.as_deref(),
        Some(refreshed_search_key.as_str())
    );
    let full_apply_status = mounted_state
        .replace_status
        .clone()
        .expect("full Apply status remains visible after automatic refresh");
    assert!(full_apply_status.contains("Applied 1 document replacement plan"));
    let full_reloaded = live.get_json(&format!("/knowledge/documents/{full_document_id}"));
    assert!(full_reloaded["document"]["content_json"]
        .to_string()
        .contains(&full_replacement));

    // Cancellation after Apply actually begins: use 100 real plans, observe the first SurrealDB
    // commit, then set the cooperative token. The terminal delivery must preserve committed receipts
    // and prove at least one unsent plan stayed unchanged.
    let cancel_document_ids = document_ids[2..102].to_vec();
    let cancel_first_id = cancel_document_ids[0].clone();
    let cancel_last_id = cancel_document_ids.last().unwrap().clone();
    let cancel_last_before = live.get_json(&format!("/knowledge/documents/{cancel_last_id}"));
    let cancel_regex =
        handshake_native::find_in_files::compile_search_regex(&needle, MatchOptions::default())
            .expect("cancel literal regex");
    let cancel_preview_cell: FindReplaceCell =
        Arc::new(Mutex::new(std::collections::VecDeque::new()));
    doc_client.preview_replace(
        &workspace_id,
        cancel_document_ids,
        cancel_regex,
        format!("MT029_CANCELLED_AFTER_START_{unique}"),
        MatchOptions::default(),
        "cancel-preview".to_owned(),
        operation_stamp(&workspace_id, FindInFilesOperation::Preview, 8),
        Arc::clone(&cancel_preview_cell),
    );
    let cancel_plans = match wait_replace(&cancel_preview_cell) {
        ReplaceDelivery::Preview { plans, .. } => plans,
        other => panic!("expected cancel preview, got {other:?}"),
    };
    assert_eq!(cancel_plans.len(), 100);
    let cancel_token = Arc::new(AtomicBool::new(false));
    let cancel_apply_cell: FindReplaceCell =
        Arc::new(Mutex::new(std::collections::VecDeque::new()));
    doc_client.apply_plans(
        &workspace_id,
        cancel_plans,
        operation_stamp(&workspace_id, FindInFilesOperation::Apply, 9),
        Arc::clone(&cancel_apply_cell),
        Arc::clone(&cancel_token),
    );
    let mut observed_commit_before_cancel = false;
    for _ in 0..400 {
        let first = live.get_json(&format!("/knowledge/documents/{cancel_first_id}"));
        if first["document"]["content_json"]
            .to_string()
            .contains("MT029_CANCELLED_AFTER_START_")
        {
            observed_commit_before_cancel = true;
            cancel_token.store(true, std::sync::atomic::Ordering::Release);
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(
        observed_commit_before_cancel,
        "cancellation token is set only after a real Apply commit is visible"
    );
    let (cancel_committed_count, cancel_skipped_count) = match wait_replace(&cancel_apply_cell) {
        ReplaceDelivery::Cancelled {
            receipts,
            audit_receipts,
            skipped_plan_count,
        } => {
            assert!(!audit_receipts.is_empty());
            assert!(receipts.iter().all(|receipt| !receipt.trim().is_empty()));
            assert!(skipped_plan_count > 0);
            (audit_receipts.len(), skipped_plan_count)
        }
        other => panic!("expected managed cancellation, got {other:?}"),
    };
    let cancel_last_after = live.get_json(&format!("/knowledge/documents/{cancel_last_id}"));
    assert_eq!(
        cancel_last_after["document"]["content_json"],
        cancel_last_before["document"]["content_json"],
        "a skipped tail plan never mutates SurrealDB"
    );

    // Bounded backend-loss + recovery through the mounted production action path. Rebinding replaces
    // the real pane factory; Argus then types and clicks Search so neither error nor recovery can be
    // satisfied by injecting detached state.
    let backend_recovery_needle = format!("MT029_BACKEND_RECOVERY_{unique}");
    let backend_recovery_created = create_doc(
        "MT-029 backend recovery",
        serde_json::json!({
            "type":"doc",
            "content":[{"type":"paragraph","content":[{"type":"text","text":backend_recovery_needle}]}]
        }),
    );
    let backend_recovery_document_id = backend_recovery_created["document"]["rich_document_id"]
        .as_str()
        .expect("backend recovery document id")
        .to_owned();
    let backend_recovery_result_id = result_author_id("document", &backend_recovery_document_id);
    ui_harness
        .state_mut()
        .set_backend_base_url_for_test("http://127.0.0.1:9", runtime.handle().clone());
    // Force the production mount-effect bookmark GET to re-issue against the unreachable backend by
    // advancing the shell's workspace binding generation through the normal project-switch path.
    ui_harness
        .state_mut()
        .bind_active_project_for_integration_test(other_workspace_id.clone());
    ui_harness
        .state_mut()
        .bind_active_project_for_integration_test(workspace_id.clone());
    assert!(ui_harness.state_mut().dispatch_palette_action_for_test(
        handshake_native::command_registry::CMD_VIEW_FIND_IN_FILES
    ));
    ui_harness.run_steps(2);
    for _ in 0..400 {
        ui_harness.run_steps(1);
        if author_ids(&ui_harness).contains(BOOKMARK_RETRY_AUTHOR_ID) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    assert!(
        author_ids(&ui_harness).contains(BOOKMARK_RETRY_AUTHOR_ID),
        "a failed mount-time bookmark GET must expose the stable Retry control"
    );
    argus.set_value_and_reinspect(&mut ui_harness, QUERY_AUTHOR_ID, &backend_recovery_needle);
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut ui_harness,
        "backend-loss-query-visible",
        serde_json::json!({"query": backend_recovery_needle.clone()}),
        |tree| {
            json_node_by_author_id(tree, QUERY_AUTHOR_ID)
                .and_then(|node| node.get("value"))
                .and_then(serde_json::Value::as_str)
                == Some(backend_recovery_needle.as_str())
        },
    );
    // Backend loss is a REAL terminal transport failure of the Search action: the observer publishes a
    // typed Failed transition bound to the same target/semantic, so the canonical receipt is `rejected`
    // rather than an unprovable `indeterminate`.
    argus.click_expect_typed_rejected_and_reinspect(
        &mut ui_harness,
        SEARCH_AUTHOR_ID,
        "find-in-files.search failed",
    );
    for _ in 0..400 {
        ui_harness.run_steps(1);
        let (_, loading, error, _) = ui_harness.state().find_in_files_diagnostics_for_test();
        if !loading && error.is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut ui_harness,
        "mounted-backend-loss-visible",
        serde_json::json!({"unavailable_base_url": "http://127.0.0.1:9"}),
        |tree| {
            json_node_by_author_id(tree, STATUS_AUTHOR_ID)
                .and_then(|node| node.get("value"))
                .and_then(serde_json::Value::as_str)
                .is_some_and(|value| {
                    let lower = value.to_ascii_lowercase();
                    lower.contains("error")
                        || lower.contains("connect")
                        || lower.contains("refused")
                })
        },
    );

    // ── Canonical Argus RETRY (V4 remediation 2) ────────────────────────────────────────────────────
    // Retry re-issues the real bookmark GET against the still-unreachable backend. Success would remove
    // the control; a typed terminal FAILURE leaves it mounted. The completion observer therefore
    // publishes a causally bound Failed transition, and the canonical receipt is terminal `rejected` —
    // never `indeterminate`, and never a silent "nothing changed".
    argus.click_expect_typed_rejected_and_reinspect(
        &mut ui_harness,
        BOOKMARK_RETRY_AUTHOR_ID,
        "find-in-files.bookmark-load failed",
    );
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut ui_harness,
        "mounted-bookmark-retry-typed-failure",
        serde_json::json!({
            "retry_author_id": BOOKMARK_RETRY_AUTHOR_ID,
            "unavailable_base_url": "http://127.0.0.1:9",
        }),
        |tree| {
            json_has_author_id(tree, BOOKMARK_RETRY_AUTHOR_ID)
                && json_node_by_author_id(tree, BOOKMARK_STATUS_AUTHOR_ID)
                    .and_then(|node| node.get("value"))
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|value| !value.trim().is_empty())
        },
    );

    ui_harness
        .state_mut()
        .set_backend_base_url_for_test(&live.base, runtime.handle().clone());
    assert!(ui_harness.state_mut().dispatch_palette_action_for_test(
        handshake_native::command_registry::CMD_VIEW_FIND_IN_FILES
    ));
    ui_harness.run_steps(2);
    argus.set_value_and_reinspect(&mut ui_harness, QUERY_AUTHOR_ID, &backend_recovery_needle);
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut ui_harness,
        "backend-recovery-query-visible",
        serde_json::json!({"query": backend_recovery_needle.clone()}),
        |tree| {
            json_node_by_author_id(tree, QUERY_AUTHOR_ID)
                .and_then(|node| node.get("value"))
                .and_then(serde_json::Value::as_str)
                == Some(backend_recovery_needle.as_str())
        },
    );
    argus.click_and_reinspect(&mut ui_harness, SEARCH_AUTHOR_ID);
    for _ in 0..400 {
        ui_harness.run_steps(1);
        if author_ids(&ui_harness).contains(&backend_recovery_result_id) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut ui_harness,
        "mounted-backend-recovery-visible",
        serde_json::json!({
            "result_author_id": backend_recovery_result_id.clone(),
            "document_id": backend_recovery_document_id.clone()
        }),
        |tree| {
            json_has_author_id(tree, &backend_recovery_result_id)
                && json_node_by_author_id(tree, STATUS_AUTHOR_ID)
                    .and_then(|node| node.get("value"))
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|value| value.contains("result"))
        },
    );
    // STRICT close-out: every canonical action in this proof must carry a terminal, causally bound
    // receipt. `finish_require_no_indeterminate` rejects the run if ANY action terminalised as
    // `indeterminate` (the exact V4 failure), on top of `finish`'s existing requirement that every
    // action be rebound to an authoritative terminal snapshot with at least one passing predicate.
    argus.finish_require_no_indeterminate();
    drop(ui_harness);
    drop(_managed_wgpu_guard);

    let expected_bookmark = |query: String,
                             kind: KindFilter,
                             tag_filter: String,
                             path_filter: String,
                             case_sensitive: bool,
                             whole_word: bool,
                             is_regex: bool| {
        let mut bookmark = SearchBookmark {
            id: String::new(),
            label: String::new(),
            query,
            kind,
            tag_filter,
            path_filter,
            case_sensitive,
            whole_word,
            is_regex,
            saved_at: "2026-07-15T00:00:00Z".to_owned(),
        };
        bookmark.id = bookmark.stable_id();
        bookmark.label = bookmark.display_label();
        bookmark
    };
    let case_query = format!("CaseSensitive{unique}");
    let expected_bookmarks = vec![
        expected_bookmark(
            case_query.clone(),
            KindFilter::Document,
            tag_hub_id.clone(),
            "path-scope".to_owned(),
            true,
            true,
            true,
        ),
        expected_bookmark(
            case_query.to_lowercase(),
            KindFilter::Document,
            tag_hub_id.clone(),
            "path-scope".to_owned(),
            true,
            true,
            true,
        ),
        expected_bookmark(
            "文".to_owned(),
            KindFilter::All,
            String::new(),
            String::new(),
            false,
            false,
            false,
        ),
        expected_bookmark(
            "東".to_owned(),
            KindFilter::All,
            String::new(),
            String::new(),
            false,
            false,
            false,
        ),
    ];
    let expected_ids: HashSet<_> = expected_bookmarks
        .iter()
        .map(|bookmark| bookmark.id.clone())
        .collect();
    assert_eq!(
        expected_ids.len(),
        expected_bookmarks.len(),
        "case-sensitive case variants and Unicode-only variants have distinct semantic ids"
    );

    // Drive the real mounted producer path. Every bookmark is saved by clicking the production
    // find-in-files.save-bookmark control; no backend/client seed bypasses state shaping or id creation.
    let producer_shared = Arc::new(Mutex::new(FindInFilesPaneShared::new(
        HsTheme::Dark.palette(),
    )));
    {
        let mut shared = producer_shared.lock().expect("bookmark producer shared");
        shared.workspace_id = Some(workspace_id.clone());
        shared.workspace_generation = 1;
    }
    let producer_factory = FindInFilesPaneFactory::new(
        search_client.clone(),
        doc_client.clone(),
        Arc::clone(&producer_shared),
    );
    let producer_state = producer_factory.state_handle();
    let producer_registry = find_in_files_registry();
    let mut producer_harness = Harness::builder()
        .proof_mt_id("MT-029")
        .with_size(egui::vec2(900.0, 760.0))
        .build_ui(move |ui| {
            PaneHostWidget::show(ui, &producer_registry, |_pane_type| &producer_factory);
        });
    for _ in 0..400 {
        producer_harness.run_steps(1);
        let state = producer_state
            .lock()
            .expect("initial bookmark producer state");
        let loaded = state.bookmark_load_attempt_count == 1 && !state.bookmark_in_flight();
        drop(state);
        if loaded {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    assert!(
        producer_state
            .lock()
            .expect("loaded bookmark producer state")
            .bookmarks
            .is_empty(),
        "isolated managed workspace begins with no persisted bookmarks"
    );
    assert!(author_ids(&producer_harness).contains(SAVE_BOOKMARK_AUTHOR_ID));

    for (index, expected) in expected_bookmarks.iter().enumerate() {
        {
            let mut state = producer_state.lock().expect("set bookmark producer fields");
            state.query = expected.query.clone();
            state.kind = expected.kind;
            state.tag_filter = expected.tag_filter.clone();
            state.path_filter = expected.path_filter.clone();
            state.case_sensitive = expected.case_sensitive;
            state.whole_word = expected.whole_word;
            state.is_regex = expected.is_regex;
        }
        producer_harness.run_steps(1);
        click_author_id(&producer_harness, SAVE_BOOKMARK_AUTHOR_ID);
        for _ in 0..400 {
            producer_harness.run_steps(1);
            let state = producer_state.lock().expect("bookmark producer save state");
            let persisted = !state.bookmark_in_flight()
                && state.bookmarks.len() == index + 1
                && state
                    .bookmarks
                    .iter()
                    .any(|bookmark| bookmark.id == expected.id);
            drop(state);
            if persisted {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        let state = producer_state
            .lock()
            .expect("saved bookmark producer state");
        assert_eq!(state.bookmarks.len(), index + 1);
        assert!(state
            .bookmarks
            .iter()
            .any(|bookmark| bookmark.id == expected.id));
        assert!(
            state
                .last_bookmark_save_receipt_id
                .as_deref()
                .is_some_and(|receipt| !receipt.trim().is_empty()),
            "production Bookmark Search save {index} preserves a nonblank producer receipt"
        );
    }
    assert_eq!(
        producer_state
            .lock()
            .expect("coexisting producer bookmarks")
            .bookmarks
            .iter()
            .map(|bookmark| bookmark.id.clone())
            .collect::<HashSet<_>>(),
        expected_ids,
        "four distinct production saves coexist without silent eviction"
    );
    let bookmark_save_receipt = producer_state
        .lock()
        .expect("terminal production bookmark receipt")
        .last_bookmark_save_receipt_id
        .clone();

    let bookmark_cell: BookmarkStateCell = Arc::new(Mutex::new(std::collections::VecDeque::new()));
    search_client.load_bookmarks(
        &workspace_id,
        operation_stamp(&workspace_id, FindInFilesOperation::BookmarkLoad, 5),
        Arc::clone(&bookmark_cell),
    );
    let (bookmark_blob, _, _) = wait_bookmark(&bookmark_cell).expect("managed bookmark load");
    let persisted_bookmarks =
        parse_bookmark_state(&bookmark_blob).expect("strict bookmark payload");
    assert_eq!(persisted_bookmarks.len(), expected_bookmarks.len());
    for expected in &expected_bookmarks {
        let persisted = persisted_bookmarks
            .iter()
            .find(|bookmark| bookmark.id == expected.id)
            .expect("production-saved bookmark persisted by exact semantic id");
        assert_eq!(persisted.query, expected.query);
        assert_eq!(persisted.kind, expected.kind);
        assert_eq!(persisted.tag_filter, expected.tag_filter);
        assert_eq!(persisted.path_filter, expected.path_filter);
        assert_eq!(persisted.case_sensitive, expected.case_sensitive);
        assert_eq!(persisted.whole_word, expected.whole_word);
        assert_eq!(persisted.is_regex, expected.is_regex);
    }
    drop(producer_harness);

    // True production-panel remount after the production Save actions. Its mount effect must issue the
    // backend GET, retain all collision probes, expose stable per-bookmark actions, and Restore must
    // repopulate every query/filter/option field through the actual UI callback.
    let bookmark_shared = Arc::new(Mutex::new(FindInFilesPaneShared::new(
        HsTheme::Dark.palette(),
    )));
    {
        let mut shared = bookmark_shared.lock().expect("bookmark remount shared");
        shared.workspace_id = Some(workspace_id.clone());
        shared.workspace_generation = 2;
    }
    let bookmark_factory = FindInFilesPaneFactory::new(
        search_client.clone(),
        doc_client.clone(),
        Arc::clone(&bookmark_shared),
    );
    let bookmark_mounted_state = bookmark_factory.state_handle();
    let bookmark_registry = find_in_files_registry();
    let mut bookmark_harness = Harness::builder()
        .proof_mt_id("MT-029")
        .with_size(egui::vec2(900.0, 760.0))
        .build_ui(move |ui| {
            PaneHostWidget::show(ui, &bookmark_registry, |_pane_type| &bookmark_factory);
        });
    let bookmark = &expected_bookmarks[0];
    let restore_id = bookmark_restore_author_id(&bookmark.id);
    let remove_id = bookmark_remove_author_id(&bookmark.id);
    let expected_action_ids: Vec<_> = expected_bookmarks
        .iter()
        .map(|bookmark| {
            (
                bookmark_restore_author_id(&bookmark.id),
                bookmark_remove_author_id(&bookmark.id),
            )
        })
        .collect();
    for _ in 0..400 {
        bookmark_harness.run_steps(1);
        let ids = author_ids(&bookmark_harness);
        if expected_action_ids
            .iter()
            .all(|(restore, remove)| ids.contains(restore) && ids.contains(remove))
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    let remounted_ids = author_ids(&bookmark_harness);
    assert!(
        expected_action_ids
            .iter()
            .all(|(restore, remove)| remounted_ids.contains(restore) && remounted_ids.contains(remove)),
        "fresh production remount retains every case/Unicode bookmark with stable Restore/Remove ids"
    );
    {
        let mut mounted = bookmark_mounted_state
            .lock()
            .expect("production bookmark mounted state");
        mounted.query = "different".to_owned();
        mounted.kind = KindFilter::All;
        mounted.tag_filter.clear();
        mounted.path_filter.clear();
        mounted.case_sensitive = false;
        mounted.whole_word = false;
        mounted.is_regex = false;
    }
    click_author_id(&bookmark_harness, &restore_id);
    bookmark_harness.run_steps(2);
    {
        let mounted = bookmark_mounted_state
            .lock()
            .expect("restored production bookmark state");
        assert_eq!(mounted.query, bookmark.query);
        assert_eq!(mounted.kind, bookmark.kind);
        assert_eq!(mounted.tag_filter, bookmark.tag_filter);
        assert_eq!(mounted.path_filter, bookmark.path_filter);
        assert_eq!(mounted.case_sensitive, bookmark.case_sensitive);
        assert_eq!(mounted.whole_word, bookmark.whole_word);
        assert_eq!(mounted.is_regex, bookmark.is_regex);
    }
    for (author_id, expected) in [
        (QUERY_AUTHOR_ID, bookmark.query.as_str()),
        (TAG_FILTER_AUTHOR_ID, bookmark.tag_filter.as_str()),
        (PATH_FILTER_AUTHOR_ID, bookmark.path_filter.as_str()),
    ] {
        let value = bookmark_harness
            .root()
            .children_recursive()
            .find(|node| node.accesskit_node().author_id() == Some(author_id))
            .and_then(|node| node.accesskit_node().value())
            .map(|value| value.to_owned())
            .unwrap_or_else(|| panic!("restored input {author_id} exposes an AccessKit value"));
        assert_eq!(value, expected, "restored {author_id} value");
    }
    let kind_label = bookmark_harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(KIND_FILTER_AUTHOR_ID))
        .and_then(|node| node.accesskit_node().label())
        .map(|value| value.to_owned())
        .expect("restored kind filter exposes an AccessKit label");
    assert!(kind_label.contains("Documents"), "restored kind label");
    for toggle_author_id in [
        TOGGLE_CASE_AUTHOR_ID,
        TOGGLE_WORD_AUTHOR_ID,
        TOGGLE_REGEX_AUTHOR_ID,
    ] {
        let toggled = bookmark_harness
            .root()
            .children_recursive()
            .find(|node| node.accesskit_node().author_id() == Some(toggle_author_id))
            .and_then(|node| node.accesskit_node().toggled());
        assert_eq!(
            toggled,
            Some(egui::accesskit::Toggled::True),
            "restored {toggle_author_id} pressed state"
        );
    }

    click_author_id(&bookmark_harness, &remove_id);
    for _ in 0..400 {
        bookmark_harness.run_steps(1);
        if !bookmark_mounted_state
            .lock()
            .expect("bookmark remove state")
            .bookmark_in_flight()
            && !author_ids(&bookmark_harness).contains(&remove_id)
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    assert!(
        !author_ids(&bookmark_harness).contains(&remove_id),
        "UI Remove clears the mounted persisted row"
    );
    let fresh_absence_cell: BookmarkStateCell =
        Arc::new(Mutex::new(std::collections::VecDeque::new()));
    search_client.load_bookmarks(
        &workspace_id,
        operation_stamp(&workspace_id, FindInFilesOperation::BookmarkLoad, 71),
        Arc::clone(&fresh_absence_cell),
    );
    let (fresh_absence_blob, _, _) =
        wait_bookmark(&fresh_absence_cell).expect("fresh backend bookmark absence GET");
    let fresh_absence =
        parse_bookmark_state(&fresh_absence_blob).expect("strict fresh absence payload");
    assert_eq!(fresh_absence.len(), expected_bookmarks.len() - 1);
    assert!(
        fresh_absence.iter().all(|saved| saved.id != bookmark.id),
        "fresh backend GET proves UI Remove persisted exact-bookmark absence"
    );

    drop(bookmark_harness);
    let absence_shared = Arc::new(Mutex::new(FindInFilesPaneShared::new(
        HsTheme::Dark.palette(),
    )));
    {
        let mut shared = absence_shared.lock().expect("absence remount shared");
        shared.workspace_id = Some(workspace_id.clone());
        shared.workspace_generation = 3;
    }
    let absence_factory = FindInFilesPaneFactory::new(
        search_client.clone(),
        doc_client.clone(),
        Arc::clone(&absence_shared),
    );
    let absence_state = absence_factory.state_handle();
    let absence_registry = find_in_files_registry();
    let mut absence_harness = Harness::builder()
        .proof_mt_id("MT-029")
        .with_size(egui::vec2(900.0, 760.0))
        .build_ui(move |ui| {
            PaneHostWidget::show(ui, &absence_registry, |_pane_type| &absence_factory);
        });
    for _ in 0..400 {
        absence_harness.run_steps(1);
        let state = absence_state.lock().expect("absence remount state");
        let loaded = state.bookmark_load_attempt_count == 1 && !state.bookmark_in_flight();
        drop(state);
        if loaded {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    let absence_ids = author_ids(&absence_harness);
    let absence_bookmarks = absence_state
        .lock()
        .expect("fresh absence mounted state")
        .bookmarks
        .clone();
    assert_eq!(absence_bookmarks.len(), expected_bookmarks.len() - 1);
    assert!(absence_bookmarks
        .iter()
        .all(|saved| saved.id != bookmark.id));
    assert!(!absence_ids.contains(&restore_id) && !absence_ids.contains(&remove_id));
    assert!(
        expected_action_ids[1..]
            .iter()
            .all(|(restore, remove)| absence_ids.contains(restore) && absence_ids.contains(remove)),
        "a second fresh production mount confirms exact removed-bookmark absence without evicting case/Unicode siblings"
    );

    // Mount-time load failure is recoverable from a stable Retry control. Use a real managed-backend
    // 404 workspace so both attempts are bounded real HTTP requests rather than staged deliveries.
    let missing_workspace_id = format!("missing-mt029-{unique}");
    let retry_shared = Arc::new(Mutex::new(FindInFilesPaneShared::new(
        HsTheme::Dark.palette(),
    )));
    {
        let mut shared = retry_shared.lock().expect("retry shared");
        shared.workspace_id = Some(missing_workspace_id);
        shared.workspace_generation = 1;
    }
    let retry_factory = FindInFilesPaneFactory::new(
        search_client.clone(),
        doc_client.clone(),
        Arc::clone(&retry_shared),
    );
    let retry_state = retry_factory.state_handle();
    let retry_registry = find_in_files_registry();
    let mut retry_harness = Harness::builder()
        .proof_mt_id("MT-029")
        .with_size(egui::vec2(900.0, 760.0))
        .build_ui(move |ui| {
            PaneHostWidget::show(ui, &retry_registry, |_pane_type| &retry_factory);
        });
    for _ in 0..400 {
        retry_harness.run_steps(1);
        if author_ids(&retry_harness).contains(BOOKMARK_RETRY_AUTHOR_ID) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    assert_eq!(
        retry_state
            .lock()
            .expect("initial retry state")
            .bookmark_load_attempt_count,
        1
    );
    assert!(author_ids(&retry_harness).contains(BOOKMARK_RETRY_AUTHOR_ID));
    click_author_id(&retry_harness, BOOKMARK_RETRY_AUTHOR_ID);
    for _ in 0..400 {
        retry_harness.run_steps(1);
        let state = retry_state.lock().expect("retried bookmark state");
        if state.bookmark_load_attempt_count == 2
            && state.bookmark_load_failed
            && !state.bookmark_in_flight()
        {
            break;
        }
        drop(state);
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    let retried = retry_state.lock().expect("terminal retry state");
    assert_eq!(retried.bookmark_load_attempt_count, 2);
    assert!(retried.bookmark_load_failed);
    drop(retried);
    assert!(
        author_ids(&retry_harness).contains(BOOKMARK_RETRY_AUTHOR_ID),
        "a failed Retry remains visibly retryable instead of entering a perpetual spinner"
    );

    let other_cleanup_status = other_cleanup.clean();
    let other_workspace_list = live.get_json("/workspaces");
    let other_workspace_absent_from_fresh_list = other_workspace_list
        .as_array()
        .expect("workspace list is an array")
        .iter()
        .all(|workspace| workspace["id"].as_str() != Some(other_workspace_id.as_str()));
    assert!(
        other_workspace_absent_from_fresh_list,
        "fresh GET /workspaces proves cross-workspace fixture cleanup"
    );
    let cleanup_status = cleanup.clean();
    let workspace_list = live.get_json("/workspaces");
    let workspace_absent_from_fresh_list = workspace_list
        .as_array()
        .expect("workspace list is an array")
        .iter()
        .all(|workspace| workspace["id"].as_str() != Some(workspace_id.as_str()));
    assert!(
        workspace_absent_from_fresh_list,
        "fresh GET /workspaces proves workspace deletion"
    );
    let deleted_refetch_cell: handshake_native::backend_client::GraphSearchCell =
        Arc::new(Mutex::new(std::collections::VecDeque::new()));
    search_client.search_paginated(
        &workspace_id,
        &needle,
        Some("document"),
        "",
        "",
        SearchMatchOptions::default(),
        "deleted-workspace-refetch".to_owned(),
        operation_stamp(&workspace_id, FindInFilesOperation::Search, 99),
        Arc::clone(&deleted_refetch_cell),
    );
    let deleted_refetch_error = wait_search(&deleted_refetch_cell)
        .expect_err("deleted workspace refetch must fail instead of serving cached results");
    assert!(
        deleted_refetch_error.contains("404") || deleted_refetch_error.contains("not found"),
        "deleted workspace refetch reports backend absence: {deleted_refetch_error}"
    );
    let receipt_dir = external_artifact_dir("wp-kernel-012-mt-029");
    std::fs::create_dir_all(&receipt_dir).expect("create external receipt directory");
    let receipt = serde_json::json!({
        "schema_id": "hsk.mt029.managed_receipt@1",
        "workspace_id": workspace_id,
        "document_ids": document_ids,
        "pagination": {"page_size": 500, "seeded_hit_count": 501, "all_pages_returned": true},
        "search_options": {
            "case_insensitive": true,
            "case_sensitive_negative": true,
            "whole_word": true,
            "regex": true,
            "source_kind_document": true,
            "tag_filter_positive": true,
            "tag_filter_negative": true,
            "path_filter": true
        },
        "navigation": {
            "author_id": route,
            "document_id": document_id_from_hit(first_hit),
            "round_trip_identity": true,
            "mounted_open_request_shell_target": format!("{shell_target:?}"),
            "rich_editor_backend_mount": {
                "loaded": rich_editor_loaded,
                "document_id": ui_document_id,
                "title": ui_title,
                "doc_version": ui_document_version,
                "content_json": ui_content,
                "root_author_id": handshake_native::rich_editor::renderer::RICH_EDITOR_ROOT_AUTHOR_ID,
                "first_block_author_id": handshake_native::rich_editor::renderer::block_author_id(&[0])
            }
        },
        "mounted_ui_search_preview_apply": true,
        "mounted_ui_screenshot_png": managed_png,
        "production_handshake_app_route_count": production_app_route_count,
        "bookmark_save_receipt": bookmark_save_receipt,
        "bookmark_production_remount_restore_remove": {
            "producer_action_author_id": SAVE_BOOKMARK_AUTHOR_ID,
            "case_and_unicode_bookmark_ids": expected_bookmarks.iter().map(|bookmark| bookmark.id.clone()).collect::<Vec<_>>(),
            "coexisting_bookmark_count_before_remove": expected_bookmarks.len(),
            "restore_author_id": restore_id,
            "remove_author_id": remove_id,
            "all_fields_restored": true,
            "fresh_backend_absence": true,
            "second_fresh_mount_absence": true,
            "load_failure_retry_attempt_count": 2
        },
        "cross_workspace_krd_rejected": true,
        "backend_loss_visible_error_and_recovery": true,
        "stale_preview_blocked": true,
        "full_apply_status": full_apply_status,
        "automatic_refresh_removed_old_hit": true,
        "partial_conflict_preserved_receipts": true,
        "cancel_after_apply_started": {
            "observed_commit_before_cancel": observed_commit_before_cancel,
            "committed_audit_count": cancel_committed_count,
            "skipped_plan_count": cancel_skipped_count,
            "skipped_tail_unchanged": true
        },
        "attrs_code_replaced_and_non_text_nodes_preserved": true,
        "saved_receipts": saved_receipts,
        "saved_receipts_all_nonblank": true,
        "audit_receipts": audit_receipts.iter().map(|entry| serde_json::json!({
            "document_id": entry.document_id,
            "before_sha256": entry.before_sha256,
            "after_sha256": entry.after_sha256,
            "outcome": format!("{:?}", entry.outcome),
            "save_receipt_event_id": entry.save_receipt_event_id,
            "error": entry.error,
        })).collect::<Vec<_>>(),
        "workspace_cleanup_http_status": cleanup_status,
        "workspace_cleanup_absent_from_fresh_list": workspace_absent_from_fresh_list,
        "workspace_cleanup_refetch_error": deleted_refetch_error,
        "other_workspace_cleanup_http_status": other_cleanup_status,
        "other_workspace_cleanup_absent_from_fresh_list": other_workspace_absent_from_fresh_list,
        "cleanup_verified": matches!(cleanup_status, 200 | 202 | 204 | 404)
            && workspace_absent_from_fresh_list
            && matches!(other_cleanup_status, 200 | 202 | 204 | 404)
            && other_workspace_absent_from_fresh_list
            && (deleted_refetch_error.contains("404")
                || deleted_refetch_error.contains("not found"))
    });
    std::fs::write(
        receipt_dir.join("MT-029-managed-search-replace-bookmark-receipt.json"),
        serde_json::to_vec_pretty(&receipt).expect("serialize receipt"),
    )
    .expect("write external managed receipt");
    assert_no_local_artifact_dir();
}
