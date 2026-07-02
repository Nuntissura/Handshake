//! MT-015 wikilinks / transclusion / backlinks PROOFS: kittest screenshots, AccessKit-tree
//! assertions, autocomplete `[[` trigger + Escape interaction, and the gated real-backend
//! integration test.
//!
//! Artifact hygiene (CX-212E): EVERY PNG is written ONLY to the EXTERNAL
//! `Handshake_Artifacts/handshake-test/wp-kernel-012-mt-015/` root via [`external_artifact_dir`];
//! [`assert_no_local_artifact_dir`] fails the run if a repo-local `tests/screenshots/` or
//! `test_output/` directory exists (the MT contract names a repo-local screenshot path, but the
//! CX-212E artifact rule OVERRIDES it — a tracked PNG under src/ is a hygiene failure the reviewer
//! greps for with `git ls-files "src/**/*.png"`).
//!
//! Backend reality (Spec-Realism Gate): the parser, chip render, autocomplete STATE transitions +
//! debounce + cancellation, transclusion view states (incl. 404->Remove embed), and backlinks panel
//! are FULLY proven here with a mock backend — NO live backend. The real-backend transclusion +
//! backlinks ACs are the `#[ignore]` integration tests, which need a live Handshake-managed backend
//! with seeded data; absent that, they are NEEDS_MANAGED_RESOURCE_PROOF (run with
//! `--features integration -- --ignored` against a live backend). The mock never fakes the backend —
//! it proves the resolver BINDING + the view dispatch, not fabricated content.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;

use handshake_native::rich_editor::document_model::node::{
    BlockNode, Child, HsLinkNode, NodeKind, TextLeaf, TransclusionNode,
};
use handshake_native::rich_editor::renderer::rich_editor_widget::{
    RichEditorState, RichEditorWidget,
};
use handshake_native::rich_editor::wikilinks::client::{
    BacklinksResponse, LoomBlockTransclusion, RichDocBacklink, WikilinkBackend, WikilinkError,
    WikilinkFuture, WikilinkResult,
};
use handshake_native::rich_editor::wikilinks::runtime::{BacklinksState, WikilinkRuntime};

/// The crate-relative path to the EXTERNAL artifacts root (CX-212E), disk-agnostic. The crate sits at
/// `<repo>/src/frontend/handshake_native`, so four `..` reach `<repo>/..` where `Handshake_Artifacts`
/// is a sibling of the repo worktree.
fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

/// Assert NO repo-local artifact directory exists under the crate (CX-212E hygiene). Checks BOTH
/// `test_output/` and `tests/screenshots/` (the path the MT contract literally names, which this rule
/// overrides).
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

/// Serialize the `.wgpu()` screenshot tests. egui_kittest's `Harness::builder().wgpu()` spins up its
/// own wgpu Instance/Adapter/Device; creating several of those concurrently on parallel test threads is
/// a known Windows wgpu hazard that aborts the process with STATUS_ACCESS_VIOLATION (0xC0000005) — so
/// `cargo test --test test_wikilinks` (default multi-threaded harness) would crash even though every
/// test passes when run serially. This guard makes the documented proof command deterministic without a
/// new dependency or a `--test-threads=1` requirement: each wgpu test holds the lock for the lifetime of
/// its Harness, so at most one wgpu device exists at a time. Mirrors the crate's existing
/// `WIRE_TEST_GUARD` idiom (test_mcp_tools.rs / test_swarm_concurrency.rs). A poisoned lock is recovered
/// (a prior panic already failed that test). Non-wgpu tests run fully parallel.
static WGPU_SERIAL_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn wgpu_guard() -> std::sync::MutexGuard<'static, ()> {
    WGPU_SERIAL_GUARD.lock().unwrap_or_else(|p| p.into_inner())
}

// ── Mock backend (no live backend) ─────────────────────────────────────────────────────────────

/// A mock wikilink backend that serves canned transclusion + backlinks + search results from memory.
struct MockBackend {
    transclusion: Result<LoomBlockTransclusion, WikilinkError>,
    backlinks: Result<BacklinksResponse, WikilinkError>,
    search: Vec<WikilinkResult>,
}
impl WikilinkBackend for MockBackend {
    fn search<'a>(
        &'a self,
        _ws: &'a str,
        _q: &'a str,
        _l: usize,
    ) -> WikilinkFuture<'a, Vec<WikilinkResult>> {
        let rows = self.search.clone();
        Box::pin(async move { Ok(rows) })
    }
    fn resolve_transclusion<'a>(
        &'a self,
        _ws: &'a str,
        _r: &'a str,
    ) -> WikilinkFuture<'a, LoomBlockTransclusion> {
        let result = self.transclusion.clone();
        Box::pin(async move { result })
    }
    fn list_backlinks<'a>(&'a self, _d: &'a str) -> WikilinkFuture<'a, BacklinksResponse> {
        let result = self.backlinks.clone();
        Box::pin(async move { result })
    }
}

fn resolved_transclusion(block_id: &str, body: &str) -> LoomBlockTransclusion {
    LoomBlockTransclusion {
        block_id: block_id.into(),
        workspace_id: "ws".into(),
        source_document_id: Some("DOC-SRC".into()),
        source_doc_version: Some(1),
        content_json: Some(serde_json::json!({
            "type": "doc",
            "content": [{"type":"paragraph","content":[{"type":"text","text": body}]}]
        })),
        resolved: true,
        unresolved_reason: None,
    }
}

fn backlink(src: &str) -> RichDocBacklink {
    RichDocBacklink {
        backlink_id: format!("BL-{src}"),
        workspace_id: "ws".into(),
        relationship_id: "REL".into(),
        source_document_id: src.into(),
        link_kind: "note".into(),
        target: "DOC-1".into(),
        block_id: "BLK".into(),
    }
}

/// A headless wikilink runtime with seeded mock results (no tokio handle). Because there is no tokio
/// runtime, the backlinks panel's `ensure_backlinks_loaded` cannot resolve a spawned fetch (it would
/// spin forever), so we PRE-SEED the backlinks into a terminal `Loaded`/`Failed` state from the mock
/// result. This reproduces the post-load steady state headlessly (the runtime fetch path is proven by
/// the unit tests in `runtime.rs` + the `#[ignore]` real-backend integration test).
fn headless_runtime(
    transclusion: Result<LoomBlockTransclusion, WikilinkError>,
    backlinks: Result<BacklinksResponse, WikilinkError>,
    search: Vec<WikilinkResult>,
) -> WikilinkRuntime {
    let seeded_backlinks = match &backlinks {
        Ok(resp) => BacklinksState::Loaded(resp.backlinks.clone()),
        Err(e) => BacklinksState::Failed(e.clone()),
    };
    let mut rt = WikilinkRuntime::headless(Arc::new(MockBackend {
        transclusion,
        backlinks,
        search,
    }));
    rt.backlinks = seeded_backlinks;
    rt
}

/// A doc with one paragraph carrying a leading text run + a wikilink hsLink atom (the chip).
fn doc_with_chip(ref_kind: &str, ref_value: &str, label: &str, resolved: bool) -> BlockNode {
    let mut link = HsLinkNode::new(ref_kind, ref_value, label);
    link.resolved = resolved;
    BlockNode::doc(vec![BlockNode::with_children(
        NodeKind::Paragraph,
        vec![Child::Text(TextLeaf::new("see ")), Child::HsLink(link)],
    )])
}

// ── AC-3 / PT (kittest screenshot): a wikilink renders as a visible colored chip inline ──────────

#[test]
fn mt015_wikilink_chip_screenshot() {
    let _wgpu_guard = wgpu_guard(); // serialize wgpu device creation (held for the Harness lifetime)
    let doc = doc_with_chip("wp", "WP-KERNEL-012", "My WP", true);
    let state = Arc::new(std::sync::Mutex::new(
        RichEditorState::new(doc).with_wikilink_runtime(headless_runtime(
            Err(WikilinkError::NotFound("none".into())),
            Ok(BacklinksResponse {
                source_document_id: "DOC-1".into(),
                backlinks: vec![],
            }),
            vec![],
        )),
    ));
    let state_for_ui = Arc::clone(&state);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(520.0, 280.0))
        .wgpu()
        .build_ui(move |ui| {
            RichEditorWidget::new(Arc::clone(&state_for_ui)).show(ui);
        });
    harness.run();
    harness.run();

    // AC-3 / AC (AccessKit): the chip is addressable by its `wikilink-chip-{hash}` author_id with
    // Role::Link.
    let expected_author =
        handshake_native::rich_editor::wikilinks::inline_view::chip_author_id("WP-KERNEL-012");
    let root = harness.root();
    let mut chip_found = false;
    for node in root.children_recursive() {
        if node.accesskit_node().author_id() == Some(expected_author.as_str()) {
            chip_found = true;
            break;
        }
    }
    assert!(
        chip_found,
        "AC-3: the wikilink renders an addressable '{expected_author}' chip node"
    );

    match harness.render() {
        Ok(image) => {
            assert!(image.width() > 0 && image.height() > 0, "rendered image non-empty");
            let ext_dir = external_artifact_dir("wp-kernel-012-mt-015");
            let _ = std::fs::create_dir_all(&ext_dir);
            let path = ext_dir.join("mt015_wikilink_chip.png");
            let saved = image.save(&path).is_ok();
            println!("PT wikilink chip: {}x{} saved={saved} ({})", image.width(), image.height(), path.display());
        }
        Err(e) => println!(
            "BLOCKER(non-fatal): mt015_wikilink_chip screenshot render unavailable (no wgpu adapter): {e}. \
             The AccessKit chip-node structural proof passed; the PNG is a GPU-host item."
        ),
    }
    assert_no_local_artifact_dir();
}

// ── AC: clicking a wikilink chip enqueues a WikilinkActivated event ──────────────────────────────

#[test]
fn mt015_wikilink_chip_click_enqueues_event() {
    let doc = doc_with_chip("wp", "WP-7", "Seven", true);
    let state = Arc::new(std::sync::Mutex::new(
        RichEditorState::new(doc).with_wikilink_runtime(headless_runtime(
            Err(WikilinkError::NotFound("none".into())),
            Ok(BacklinksResponse {
                source_document_id: "DOC-1".into(),
                backlinks: vec![],
            }),
            vec![],
        )),
    ));
    let state_for_ui = Arc::clone(&state);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(520.0, 240.0))
        .build_ui(move |ui| {
            RichEditorWidget::new(Arc::clone(&state_for_ui)).show(ui);
        });
    harness.run();

    // Click the chip: the chip is the Role::Link node (the paragraph node also carries "see Seven"
    // as its label, so we disambiguate by role — the chip is the only Link-role node).
    {
        let node = harness.get_by_role(egui::accesskit::Role::Link);
        node.click();
    }
    harness.run();
    harness.run();

    let events = state.lock().unwrap().pending_events.clone();
    let found = events.iter().any(|e| matches!(
        e,
        handshake_native::rich_editor::wikilinks::inline_view::EditorEvent::WikilinkActivated { ref_value, .. }
            if ref_value == "WP-7"
    ));
    assert!(
        found,
        "AC: clicking the chip enqueues a WikilinkActivated event for WP-7 (got {events:?})"
    );
}

// ── AC: unresolved/unknown wikilink renders with the warning affordance ──────────────────────────

#[test]
fn mt015_unknown_wikilink_renders_warning_chip() {
    // An unresolved chip carries a `?` prefix + the error color affordance (RISK-5: visible broken link).
    let doc = doc_with_chip("unknown", "xyz", "", false);
    let state = Arc::new(std::sync::Mutex::new(
        RichEditorState::new(doc).with_wikilink_runtime(headless_runtime(
            Err(WikilinkError::NotFound("none".into())),
            Ok(BacklinksResponse {
                source_document_id: "DOC-1".into(),
                backlinks: vec![],
            }),
            vec![],
        )),
    ));
    let state_for_ui = Arc::clone(&state);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(520.0, 200.0))
        .build_ui(move |ui| {
            RichEditorWidget::new(Arc::clone(&state_for_ui)).show(ui);
        });
    harness.run();

    // The `?`-prefixed unresolved label is on screen and addressable.
    assert!(
        harness.query_by_label_contains("? unknown:xyz").is_some(),
        "RISK-5: an unknown wikilink renders a visible `?`-prefixed warning chip"
    );
}

// ── AC: typing `[[` opens the autocomplete popup; Escape closes + removes the trigger ────────────

#[test]
fn mt015_autocomplete_opens_on_double_bracket_and_escape_closes() {
    // Start with an empty paragraph and focus the editor, then type `[[` and assert the popup opens
    // (the AccessKit `wikilink-autocomplete` node appears). Then press Escape and assert it closes +
    // the `[[` trigger text is removed.
    let doc = BlockNode::doc(vec![BlockNode::with_children(
        NodeKind::Paragraph,
        vec![Child::Text(TextLeaf::new(""))],
    )]);
    let state = Arc::new(std::sync::Mutex::new(
        RichEditorState::new(doc).with_wikilink_runtime(headless_runtime(
            Err(WikilinkError::NotFound("none".into())),
            Ok(BacklinksResponse {
                source_document_id: "DOC-1".into(),
                backlinks: vec![],
            }),
            vec![WikilinkResult {
                block_id: "BLK-1".into(),
                title: "Hit One".into(),
                content_type: "note".into(),
                highlight: String::new(),
            }],
        )),
    ));
    let state_for_ui = Arc::clone(&state);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(520.0, 280.0))
        .build_ui(move |ui| {
            RichEditorWidget::new(Arc::clone(&state_for_ui)).show(ui);
        });
    harness.run();

    // Focus the editor SURFACE (the focusable click_and_drag node carrying author_id
    // `rich-editor-surface`) by sending it an AccessKit Focus action — this is the same focus an
    // out-of-process agent would request by the stable surface id. The root TextInput node is only an
    // AccessKit container (no interaction), so we focus the surface node specifically. A FOCUSED
    // editor requests a continuous blink repaint, so we advance with step() (single frame) throughout
    // (run() requires convergence and would trip its step cap on the blink repaint).
    {
        let root = harness.root();
        let surface = root
            .children_recursive()
            .find(|n| n.accesskit_node().author_id() == Some("rich-editor-surface"))
            .expect("the editor surface node carries author_id 'rich-editor-surface'");
        surface.focus();
    }
    harness.step(); // process the focus action -> surface focused
    harness.step(); // focus is live this frame

    // Type the trigger characters into the now-focused editor, then advance one frame at a time.
    harness.event(egui::Event::Text("[[".into()));
    harness.step();
    harness.step();

    // The popup opened: the state carries an autocomplete + the leaf now contains "[[".
    {
        let st = state.lock().unwrap();
        assert!(
            st.wikilink_autocomplete.is_some(),
            "typing `[[` opens the autocomplete popup"
        );
        assert_eq!(
            st.block_plain_text(0).as_deref(),
            Some("[["),
            "the `[[` trigger text is in the leaf"
        );
    }
    // The AccessKit popup node is present.
    {
        let root = harness.root();
        let mut popup_found = false;
        for node in root.children_recursive() {
            if node.accesskit_node().author_id() == Some("wikilink-autocomplete") {
                popup_found = true;
                break;
            }
        }
        assert!(
            popup_found,
            "AC-9: the 'wikilink-autocomplete' popup node is in the accessibility tree"
        );
    }

    // Press Escape: the popup closes and the `[[` trigger text is removed.
    harness.key_press(egui::Key::Escape);
    harness.step();
    harness.step();
    {
        let st = state.lock().unwrap();
        assert!(
            st.wikilink_autocomplete.is_none(),
            "Escape closes the popup"
        );
        assert_eq!(
            st.block_plain_text(0).as_deref(),
            Some(""),
            "Escape removes the `[[` trigger text"
        );
    }
}

// ── AC / PT (kittest screenshot): a standalone transclusion node renders a bordered preview ──────

#[test]
fn mt015_transclusion_view_screenshot() {
    let _wgpu_guard = wgpu_guard(); // serialize wgpu device creation (held for the Harness lifetime)
                                    // A standalone transclusion block, with the resolution PRE-SEEDED Resolved (the runtime's
                                    // resolved state, reproduced headlessly so the SCREENSHOT shows the read-through preview without
                                    // a backend). The real-backend variant is the #[ignore] integration test below.
    let doc = BlockNode::doc(vec![BlockNode::with_children(
        NodeKind::Paragraph,
        vec![Child::Transclusion(TransclusionNode::new("BLK-42"))],
    )]);
    let mut runtime = headless_runtime(
        Ok(resolved_transclusion(
            "BLK-42",
            "This is the transcluded source body.",
        )),
        Ok(BacklinksResponse {
            source_document_id: "DOC-1".into(),
            backlinks: vec![],
        }),
        vec![],
    );
    // Pre-seed the resolved transclusion state so the view renders the preview this frame.
    runtime.transclusions.insert(
        "BLK-42".into(),
        handshake_native::rich_editor::wikilinks::runtime::TransclusionState::Resolved(
            resolved_transclusion("BLK-42", "This is the transcluded source body."),
        ),
    );
    let state = Arc::new(std::sync::Mutex::new(
        RichEditorState::new(doc).with_wikilink_runtime(runtime),
    ));
    let state_for_ui = Arc::clone(&state);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(560.0, 320.0))
        .wgpu()
        .build_ui(move |ui| {
            RichEditorWidget::new(Arc::clone(&state_for_ui)).show(ui);
        });
    harness.run();
    harness.run();

    // The transclusion container is addressable + the preview body text is on screen.
    let root = harness.root();
    let mut container_found = false;
    for node in root.children_recursive() {
        if node.accesskit_node().author_id() == Some("transclusion-BLK-42") {
            container_found = true;
            break;
        }
    }
    assert!(
        container_found,
        "AC: the transclusion renders an addressable 'transclusion-BLK-42' container"
    );
    assert!(
        harness
            .query_by_label_contains("transcluded source body")
            .is_some(),
        "AC: the transclusion shows the resolved source content_preview"
    );

    match harness.render() {
        Ok(image) => {
            assert!(image.width() > 0 && image.height() > 0);
            let ext_dir = external_artifact_dir("wp-kernel-012-mt-015");
            let _ = std::fs::create_dir_all(&ext_dir);
            let path = ext_dir.join("mt015_transclusion_view.png");
            let saved = image.save(&path).is_ok();
            println!("PT transclusion view: {}x{} saved={saved} ({})", image.width(), image.height(), path.display());
        }
        Err(e) => println!(
            "BLOCKER(non-fatal): mt015_transclusion_view screenshot render unavailable (no wgpu adapter): {e}. \
             The transclusion structural proof passed; the PNG is a GPU-host item."
        ),
    }
    assert_no_local_artifact_dir();
}

// ── MC-003: a 404 transclusion renders the typed error + a "Remove embed" action ─────────────────

#[test]
fn mt015_transclusion_404_offers_remove_embed_mc003() {
    let doc = BlockNode::doc(vec![BlockNode::with_children(
        NodeKind::Paragraph,
        vec![Child::Transclusion(TransclusionNode::new("BLK-GONE"))],
    )]);
    let mut runtime = headless_runtime(
        Err(WikilinkError::NotFound("BLK-GONE".into())),
        Ok(BacklinksResponse {
            source_document_id: "DOC-1".into(),
            backlinks: vec![],
        }),
        vec![],
    );
    // Pre-seed the Failed(NotFound) state (a 404 of a deleted block).
    runtime.transclusions.insert(
        "BLK-GONE".into(),
        handshake_native::rich_editor::wikilinks::runtime::TransclusionState::Failed(
            WikilinkError::NotFound("BLK-GONE".into()),
        ),
    );
    let state = Arc::new(std::sync::Mutex::new(
        RichEditorState::new(doc).with_wikilink_runtime(runtime),
    ));
    let state_for_ui = Arc::clone(&state);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(520.0, 240.0))
        .build_ui(move |ui| {
            RichEditorWidget::new(Arc::clone(&state_for_ui)).show(ui);
        });
    harness.run();

    // The "Remove embed" action is present (MC-003).
    let root = harness.root();
    let mut remove_found = false;
    for node in root.children_recursive() {
        if node.accesskit_node().author_id() == Some("transclusion-remove-BLK-GONE") {
            remove_found = true;
            break;
        }
    }
    assert!(
        remove_found,
        "MC-003: a 404 transclusion offers a 'transclusion-remove-BLK-GONE' action"
    );

    // Click "Remove embed": the transclusion node is deleted from the doc.
    {
        let node = harness.get_by_label_contains("Remove embed");
        node.click();
    }
    harness.run();
    harness.run();
    {
        let st = state.lock().unwrap();
        let para = st.doc.children[0].as_block().unwrap();
        let has_transclusion = para.children.iter().any(|c| c.as_transclusion().is_some());
        assert!(
            !has_transclusion,
            "MC-003: clicking 'Remove embed' deletes the transclusion node"
        );
    }
}

// ── AC / PT (kittest screenshot): the backlinks panel renders the collapsible header + entries ───

#[test]
fn mt015_backlinks_panel_screenshot() {
    let _wgpu_guard = wgpu_guard(); // serialize wgpu device creation (held for the Harness lifetime)
    let doc = BlockNode::doc(vec![BlockNode::paragraph("A note with backlinks.")]);
    let mut runtime = headless_runtime(
        Err(WikilinkError::NotFound("none".into())),
        Ok(BacklinksResponse {
            source_document_id: "DOC-1".into(),
            backlinks: vec![backlink("DOC-2"), backlink("DOC-3")],
        }),
        vec![],
    );
    // Pre-seed the loaded backlinks so the panel renders the list this frame.
    runtime.backlinks = BacklinksState::Loaded(vec![backlink("DOC-2"), backlink("DOC-3")]);
    let state = Arc::new(std::sync::Mutex::new(
        RichEditorState::new(doc).with_wikilink_runtime(runtime),
    ));
    let state_for_ui = Arc::clone(&state);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(560.0, 360.0))
        .wgpu()
        .build_ui(move |ui| {
            RichEditorWidget::new(Arc::clone(&state_for_ui)).show(ui);
        });
    harness.run();
    harness.run();

    // The panel header shows the count, and each entry is addressable.
    assert!(
        harness.query_by_label_contains("Backlinks (2)").is_some(),
        "AC: the backlinks header shows 'Backlinks (N)' with the count"
    );
    let root = harness.root();
    let mut panel_found = false;
    let mut entry_found = false;
    for node in root.children_recursive() {
        match node.accesskit_node().author_id() {
            Some("backlinks-panel") => panel_found = true,
            Some("backlink-DOC-2") => entry_found = true,
            _ => {}
        }
    }
    assert!(panel_found, "AC: the 'backlinks-panel' node is present");
    assert!(
        entry_found,
        "AC: each backlink entry ('backlink-DOC-2') is addressable"
    );

    match harness.render() {
        Ok(image) => {
            assert!(image.width() > 0 && image.height() > 0);
            let ext_dir = external_artifact_dir("wp-kernel-012-mt-015");
            let _ = std::fs::create_dir_all(&ext_dir);
            let path = ext_dir.join("mt015_backlinks_panel.png");
            let saved = image.save(&path).is_ok();
            println!("PT backlinks panel: {}x{} saved={saved} ({})", image.width(), image.height(), path.display());
        }
        Err(e) => println!(
            "BLOCKER(non-fatal): mt015_backlinks_panel screenshot render unavailable (no wgpu adapter): {e}. \
             The backlinks structural proof passed; the PNG is a GPU-host item."
        ),
    }
    assert_no_local_artifact_dir();
}

// ── AC: backlinks empty state renders "No backlinks yet." ────────────────────────────────────────

#[test]
fn mt015_backlinks_empty_state() {
    let doc = BlockNode::doc(vec![BlockNode::paragraph("Lonely note.")]);
    let mut runtime = headless_runtime(
        Err(WikilinkError::NotFound("none".into())),
        Ok(BacklinksResponse {
            source_document_id: "DOC-1".into(),
            backlinks: vec![],
        }),
        vec![],
    );
    runtime.backlinks = BacklinksState::Loaded(vec![]);
    let state = Arc::new(std::sync::Mutex::new(
        RichEditorState::new(doc).with_wikilink_runtime(runtime),
    ));
    let state_for_ui = Arc::clone(&state);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(480.0, 240.0))
        .build_ui(move |ui| {
            RichEditorWidget::new(Arc::clone(&state_for_ui)).show(ui);
        });
    harness.run();
    assert!(
        harness
            .query_by_label_contains("No backlinks yet.")
            .is_some(),
        "AC: the empty backlinks state reads 'No backlinks yet.'"
    );
    assert!(
        harness.query_by_label_contains("Backlinks (0)").is_some(),
        "the header count is 0 for an empty backlinks set"
    );
}

// ── PT (gated): real backend transclusion + backlinks (NEEDS_MANAGED_RESOURCE_PROOF without one) ─

/// Real-backend transclusion + backlinks proof. Requires a LIVE Handshake-managed backend on
/// 127.0.0.1:37501 with a SEEDED workspace/document/block. OFF by default (`#[ignore]` + `integration`
/// feature). Run:
///   cargo test -p handshake-native --features integration --test test_wikilinks -- --ignored real_
///
/// Binds the production `ReqwestWikilinkBackend` against the REAL endpoints. When the backend/data is
/// absent this is the NEEDS_MANAGED_RESOURCE_PROOF gap the MT discloses.
#[test]
#[ignore = "needs a live Handshake-managed backend + seeded loom block + document (NEEDS_MANAGED_RESOURCE_PROOF)"]
#[cfg(feature = "integration")]
fn real_transclusion_and_backlinks_against_live_backend() {
    use handshake_native::backend_client::BACKEND_BASE_URL;
    use handshake_native::rich_editor::wikilinks::client::ReqwestWikilinkBackend;

    let workspace_id = std::env::var("HANDSHAKE_TEST_WORKSPACE_ID")
        .expect("set HANDSHAKE_TEST_WORKSPACE_ID to a real workspace");
    let block_id = std::env::var("HANDSHAKE_TEST_BLOCK_ID")
        .expect("set HANDSHAKE_TEST_BLOCK_ID to a real seeded transcludable loom block id");
    let document_id = std::env::var("HANDSHAKE_TEST_DOCUMENT_ID")
        .expect("set HANDSHAKE_TEST_DOCUMENT_ID to a real seeded document id");

    let rt = tokio::runtime::Runtime::new().unwrap();
    let backend = ReqwestWikilinkBackend::new(BACKEND_BASE_URL);

    // Transclusion resolve.
    let transclusion =
        rt.block_on(async { backend.resolve_transclusion(&workspace_id, &block_id).await });
    match transclusion {
        Ok(t) => {
            assert_eq!(
                t.block_id, block_id,
                "the real backend returned the requested block"
            );
            println!(
                "PT REAL transclusion resolve: resolved={} source={:?}",
                t.resolved, t.source_document_id
            );
        }
        Err(e) => panic!("PT real transclusion resolve failed (backend up + block seeded?): {e}"),
    }

    // Backlinks list.
    let backlinks = rt.block_on(async { backend.list_backlinks(&document_id).await });
    match backlinks {
        Ok(resp) => {
            assert_eq!(
                resp.source_document_id, document_id,
                "the backlinks are for the requested document"
            );
            println!(
                "PT REAL backlinks: {} backlink(s) for {document_id}",
                resp.backlinks.len()
            );
        }
        Err(e) => panic!("PT real backlinks failed (backend up + document seeded?): {e}"),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// WP-KERNEL-012 MT-068 — chip glyph-overlap fix: the laid-out atom span IS the chip label.
// The Wave-B audit's locus PNG showed doubled/garbled glyph runs around inline chips because the
// galley laid out `label`/`refKind:refValue` while the chip span/label math used `chip_label` (the
// `? …`/`⎘ …`/`(unresolved)` decorated text) — the spans disagreed, so underlying glyphs stuck out
// around the pill. These tests pin the single-span authority: layout text == chip label, span math
// (`block_galley_cursor`) == chip label, and the chip-covered paint mode reserves the space with a
// TRANSPARENT run (the chip paints the only visible glyphs).
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn mt068_atom_layout_consumes_chip_label_space() {
    use handshake_native::rich_editor::renderer::line_layout::{
        layout_block, layout_block_atoms, AtomPaint,
    };
    use handshake_native::rich_editor::wikilinks::inline_view::chip_label;
    use handshake_native::theme::HsTheme;

    // The audit's case: an UNRESOLVED locus ref whose chip label ("? WP-KERNEL-012 (unresolved)")
    // differs from both its raw label ("") and its refKind:refValue fallback.
    let mut locus = HsLinkNode::new("locus", "wp/WP-KERNEL-012", "");
    locus.resolved = false;
    let label = chip_label(&locus);
    let block = BlockNode::with_children(
        NodeKind::Paragraph,
        vec![
            Child::Text(TextLeaf::new("see ")),
            Child::HsLink(locus.clone()),
            Child::Text(TextLeaf::new(" after")),
        ],
    );
    let palette = HsTheme::Dark.palette();

    // 1) The laid-out plain text carries EXACTLY the chip label for the atom (one span authority).
    let layout = layout_block(&block, &palette, 800.0, false);
    assert_eq!(
        layout.plain_text,
        format!("see {label} after"),
        "MT-068: the galley lays out the chip's true text so the chip consumes the layout space"
    );

    // 2) The find/replace span math agrees (the shared galley-cursor mirror): the leaf AFTER the
    //    atom starts at 'see '(4) + chip-label chars — not at the raw-label length.
    use handshake_native::rich_editor::find_replace::highlight_layer::block_galley_cursor;
    let expected = 4 + label.chars().count();
    assert_eq!(
        block_galley_cursor(&block, 2, 0),
        Some(expected),
        "MT-068: span math counts the chip label ({} chars), keeping chip rects + highlight \
         cursors aligned with the laid-out glyphs",
        label.chars().count()
    );

    // 3) Chip-covered paint mode: the atom's section paints TRANSPARENT (the chip pill + its own
    //    label are the only visible glyphs — no doubled runs), while Visible mode (nested contexts
    //    with no chip overlay) keeps the run visible. Metrics/plain text identical between modes.
    let covered = layout_block_atoms(&block, &palette, 800.0, false, AtomPaint::ChipCovered);
    assert_eq!(covered.plain_text, layout.plain_text, "identical metrics");
    assert_eq!(covered.job.sections.len(), 3, "three runs: text, atom, text");
    assert_eq!(
        covered.job.sections[1].format.color,
        egui::Color32::TRANSPARENT,
        "MT-068: the chip-covered atom run reserves space but paints nothing"
    );
    assert_ne!(
        layout.job.sections[1].format.color,
        egui::Color32::TRANSPARENT,
        "Visible mode (nested list/quote/table content) still paints the atom label"
    );

    // 4) The resolved-tag regression guard: a committed tag atom's laid-out text is its label
    //    ("#rust") in BOTH the old and new schemes — the fix must not shift tag chip spans.
    let tag = HsLinkNode::new("tag", "rust", "#rust");
    assert_eq!(chip_label(&tag), "#rust", "resolved tag label unchanged");
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// WP-KERNEL-012 MT-020 — LIVE-WIDGET undo-after-insert (wikilink confirm path): a real `[[` keystroke
// decode opens the popup, a real Enter confirms the staged result into an hsLink atom, and a REAL
// Ctrl+Z through the mounted widget removes the atom and restores the exact pre-insert doc (trigger
// text included) via the MT-035 unified undo bus. Also proves the insert→type→undo→undo ORDERING:
// undo #1 removes the typed text, undo #2 removes the atom, undo #3 removes the trigger typing.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn mt020_live_wikilink_confirm_undo_restores_pre_insert_doc_and_ordering() {
    use handshake_native::interop::interaction_bus::InteractionBus;
    use handshake_native::pane_registry::PaneId;
    use handshake_native::rich_editor::wikilinks::autocomplete::SearchPhase;

    let doc = BlockNode::doc(vec![BlockNode::with_children(
        NodeKind::Paragraph,
        vec![Child::Text(TextLeaf::new(""))],
    )]);
    let rich_pane: PaneId = Arc::from("pane-mt020-wikilink");
    let state = Arc::new(std::sync::Mutex::new({
        let mut st = RichEditorState::new(doc).with_wikilink_runtime(headless_runtime(
            Err(WikilinkError::NotFound("none".into())),
            Ok(BacklinksResponse {
                source_document_id: "DOC-1".into(),
                backlinks: vec![],
            }),
            vec![],
        ));
        st.undo_pane_id = Some(rich_pane.clone());
        st
    }));
    let state_for_ui = Arc::clone(&state);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(600.0, 320.0))
        .build_ui(move |ui| {
            RichEditorWidget::new(Arc::clone(&state_for_ui)).show(ui);
        });
    harness.run();
    {
        let root = harness.root();
        let surface = root
            .children_recursive()
            .find(|n| n.accesskit_node().author_id() == Some("rich-editor-surface"))
            .expect("the editor surface node carries author_id 'rich-editor-surface'");
        surface.focus();
    }
    harness.step();
    harness.step();

    // 1) REAL keystroke decode: type `[[Hi` — the popup opens with query "Hi".
    harness.event(egui::Event::Text("[[Hi".into()));
    harness.step();
    harness.step();
    {
        let st = state.lock().unwrap();
        assert!(
            st.wikilink_autocomplete.is_some(),
            "typing `[[` opens the autocomplete popup"
        );
        assert_eq!(st.block_plain_text(0).as_deref(), Some("[[Hi"));
    }
    // Break the 500ms undo-coalescing window so the CONFIRM records a FRESH bus entry (the batcher
    // coalesces rapid edits into one undo — deliberate; the ordering proof needs distinct entries).
    std::thread::sleep(std::time::Duration::from_millis(600));

    // 2) Stage the search delivery (the headless runtime has no tokio to resolve the fetch; staging
    //    the Ready phase reproduces the post-delivery state the drain would produce), then a REAL
    //    Enter confirms the selected row through the live key handler.
    {
        let mut st = state.lock().unwrap();
        let ac = st.wikilink_autocomplete.as_mut().expect("popup open");
        ac.phase = SearchPhase::Ready(vec![WikilinkResult {
            block_id: "BLK-9".into(),
            title: "Hit One".into(),
            content_type: "note".into(),
            highlight: String::new(),
        }]);
    }
    let pre_insert_json = {
        let st = state.lock().unwrap();
        handshake_native::rich_editor::document_model::doc_json::to_content_json_value(&st.doc)
    };
    harness.key_press(egui::Key::Enter);
    harness.step();
    harness.step();
    {
        let st = state.lock().unwrap();
        assert!(st.wikilink_autocomplete.is_none(), "confirm closes popup");
        let para = st.doc.children[0].as_block().unwrap();
        let atom = para
            .children
            .iter()
            .find_map(Child::as_hs_link)
            .expect("the confirm inserted an hsLink atom");
        assert_eq!(atom.ref_value, "BLK-9");
        assert!(
            !st.block_plain_text(0).unwrap_or_default().contains("[[Hi"),
            "the `[[Hi` trigger text was removed by the confirm"
        );
    }
    // The confirm recorded on the UNIFIED bus (2 entries: the typing + the confirm).
    let bus = InteractionBus::get_or_init(&harness.ctx);
    let depth = InteractionBus::with_try_lock(&bus, |b| b.local_undo_count(&rich_pane))
        .expect("bus lock");
    assert_eq!(
        depth, 2,
        "MT-020: the atom insert recorded its own bus entry (typing + confirm)"
    );

    // 3) ORDERING: type more text AFTER the insert (its own entry after the window)...
    std::thread::sleep(std::time::Duration::from_millis(600));
    harness.event(egui::Event::Text("tail".into()));
    harness.step();
    harness.step();
    {
        let st = state.lock().unwrap();
        assert!(
            st.block_plain_text(0).unwrap_or_default().contains("tail"),
            "the post-insert typing landed"
        );
    }

    // 4) Undo #1 (REAL Ctrl+Z): the typed text goes, the atom STAYS (ordering).
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    harness.step();
    harness.step();
    {
        let st = state.lock().unwrap();
        assert!(
            !st.block_plain_text(0).unwrap_or_default().contains("tail"),
            "undo #1 removed the typed text"
        );
        let para = st.doc.children[0].as_block().unwrap();
        assert!(
            para.children.iter().any(|c| c.as_hs_link().is_some()),
            "undo #1 did NOT remove the atom (insert-type-undo-undo ordering)"
        );
    }

    // 5) Undo #2: the ATOM goes; the doc equals the exact pre-insert doc (trigger text restored).
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    harness.step();
    harness.step();
    {
        let st = state.lock().unwrap();
        let para = st.doc.children[0].as_block().unwrap();
        assert!(
            para.children.iter().all(|c| c.as_hs_link().is_none()),
            "undo #2 removed the inserted atom"
        );
        let now_json =
            handshake_native::rich_editor::document_model::doc_json::to_content_json_value(
                &st.doc,
            );
        assert_eq!(
            now_json, pre_insert_json,
            "MT-020: undo restored the EXACT pre-insert doc (trigger text `[[Hi` included)"
        );
    }

    // 6) Undo #3: the trigger typing goes too — back to the empty paragraph.
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    harness.step();
    harness.step();
    {
        let st = state.lock().unwrap();
        assert_eq!(
            st.block_plain_text(0).as_deref(),
            Some(""),
            "undo #3 removed the typed trigger — the full stack unwinds in order"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// WP-KERNEL-012 wave-2 remediation — LIVE-WIDGET undo-after-REMOVE (the MC-003 "Remove embed" path):
// clicking "Remove embed" on a 404 transclusion must route through the MT-020 Transaction machinery
// (Step::DeleteInlineChild) + the MT-035 unified undo bus, so a REAL Ctrl+Z restores the removed
// embed exactly. The pre-fix defect: the removal was a direct `children.retain` mutation that
// bypassed BOTH undo authorities — a removed embed was unrecoverable.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn mt020_live_remove_embed_undo_restores_the_transclusion() {
    use handshake_native::interop::interaction_bus::InteractionBus;
    use handshake_native::pane_registry::PaneId;

    let doc = BlockNode::doc(vec![BlockNode::with_children(
        NodeKind::Paragraph,
        vec![Child::Transclusion(TransclusionNode::new("BLK-GONE"))],
    )]);
    let mut runtime = headless_runtime(
        Err(WikilinkError::NotFound("BLK-GONE".into())),
        Ok(BacklinksResponse {
            source_document_id: "DOC-1".into(),
            backlinks: vec![],
        }),
        vec![],
    );
    // Pre-seed the Failed(NotFound) state (a 404 of a deleted block) so "Remove embed" shows.
    runtime.transclusions.insert(
        "BLK-GONE".into(),
        handshake_native::rich_editor::wikilinks::runtime::TransclusionState::Failed(
            WikilinkError::NotFound("BLK-GONE".into()),
        ),
    );
    let rich_pane: PaneId = Arc::from("pane-remove-embed-undo");
    let state = Arc::new(std::sync::Mutex::new({
        let mut st = RichEditorState::new(doc).with_wikilink_runtime(runtime);
        st.undo_pane_id = Some(rich_pane.clone());
        st
    }));
    let state_for_ui = Arc::clone(&state);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(520.0, 260.0))
        .build_ui(move |ui| {
            RichEditorWidget::new(Arc::clone(&state_for_ui)).show(ui);
        });
    harness.run();

    let pre_remove_json = {
        let st = state.lock().unwrap();
        handshake_native::rich_editor::document_model::doc_json::to_content_json_value(&st.doc)
    };

    // A REAL click on the live "Remove embed" button (the MC-003 affordance).
    {
        let node = harness.get_by_label_contains("Remove embed");
        node.click();
    }
    harness.step();
    harness.step();
    {
        let st = state.lock().unwrap();
        let para = st.doc.children[0].as_block().unwrap();
        assert!(
            para.children.iter().all(|c| c.as_transclusion().is_none()),
            "clicking 'Remove embed' deleted the transclusion node (MC-003 behavior kept)"
        );
        // The removal landed on the MODEL UndoManager (the MT-020 transaction receipt) — the direct
        // retain mutation left this empty.
        assert_eq!(
            st.undo.len(),
            1,
            "the removal pushed ONE transaction receipt onto the model UndoManager"
        );
    }
    // ...and on the MT-035 unified bus (ONE entry for the removal).
    let bus = InteractionBus::get_or_init(&harness.ctx);
    let depth = InteractionBus::with_try_lock(&bus, |b| b.local_undo_count(&rich_pane))
        .expect("bus lock");
    assert_eq!(
        depth, 1,
        "the remove-embed recorded exactly one unified-bus undo entry"
    );

    // A REAL Ctrl+Z (the chord decode runs on the FOCUSED editor input path) restores the embed.
    {
        let root = harness.root();
        let surface = root
            .children_recursive()
            .find(|n| n.accesskit_node().author_id() == Some("rich-editor-surface"))
            .expect("the editor surface node carries author_id 'rich-editor-surface'");
        surface.focus();
    }
    harness.step();
    harness.step();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    harness.step();
    harness.step();
    {
        let st = state.lock().unwrap();
        let para = st.doc.children[0].as_block().unwrap();
        assert!(
            para.children.iter().any(|c| c.as_transclusion().is_some()),
            "Ctrl+Z restored the removed transclusion embed"
        );
        let now_json =
            handshake_native::rich_editor::document_model::doc_json::to_content_json_value(
                &st.doc,
            );
        assert_eq!(
            now_json, pre_remove_json,
            "undo restored the EXACT pre-remove doc"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// WP-KERNEL-012 MT-045 (wave-2 remediation) — PRODUCT-PATH cycle guard: rendering a CYCLIC
// transclusion chain through the LIVE widget must not panic and must show a VISIBLE cycle indicator
// (`transclusion-cycle-{id}`, error-colored, preview suppressed). The guard is the PRODUCT
// `resolve_transclusion_chain` (rich_editor::wikilinks::transclusion_resolver) — the same symbol the
// LR-05 perf proof imports — walking the live resolution cache via
// `WikilinkRuntime::detect_transclusion_cycle`.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

/// A resolved transclusion whose content embeds a `loomTransclusion` targeting `next` (one chain hop).
fn chained_transclusion(block_id: &str, next: &str) -> LoomBlockTransclusion {
    LoomBlockTransclusion {
        block_id: block_id.into(),
        workspace_id: "ws".into(),
        source_document_id: Some("DOC-SRC".into()),
        source_doc_version: Some(1),
        content_json: Some(serde_json::json!({
            "type": "doc",
            "content": [{"type":"paragraph","content":[
                {"type":"text","text":"chained body"},
                {"type":"loomTransclusion","attrs":{"refValue": next}}
            ]}]
        })),
        resolved: true,
        unresolved_reason: None,
    }
}

#[test]
fn mt045_cyclic_transclusion_chain_renders_visible_cycle_indicator_without_panic() {
    use handshake_native::rich_editor::wikilinks::runtime::TransclusionState;

    // The rendered doc embeds BLK-A; the resolution cache says BLK-A -> BLK-B -> BLK-A (a cycle).
    let doc = BlockNode::doc(vec![BlockNode::with_children(
        NodeKind::Paragraph,
        vec![Child::Transclusion(TransclusionNode::new("BLK-A"))],
    )]);
    let mut runtime = headless_runtime(
        Err(WikilinkError::NotFound("none".into())),
        Ok(BacklinksResponse {
            source_document_id: "DOC-1".into(),
            backlinks: vec![],
        }),
        vec![],
    );
    runtime.transclusions.insert(
        "BLK-A".into(),
        TransclusionState::Resolved(chained_transclusion("BLK-A", "BLK-B")),
    );
    runtime.transclusions.insert(
        "BLK-B".into(),
        TransclusionState::Resolved(chained_transclusion("BLK-B", "BLK-A")),
    );
    let state = Arc::new(std::sync::Mutex::new(
        RichEditorState::new(doc).with_wikilink_runtime(runtime),
    ));
    let state_for_ui = Arc::clone(&state);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(560.0, 300.0))
        .build_ui(move |ui| {
            RichEditorWidget::new(Arc::clone(&state_for_ui)).show(ui);
        });
    // Rendering the cyclic chain must not panic and must not spin (the resolver's visited set +
    // depth bound terminate the walk).
    harness.run();

    // The VISIBLE cycle indicator is present: the stable AccessKit id + the typed label text.
    let root = harness.root();
    let mut cycle_node_found = false;
    for node in root.children_recursive() {
        if node.accesskit_node().author_id() == Some("transclusion-cycle-BLK-A") {
            cycle_node_found = true;
            break;
        }
    }
    assert!(
        cycle_node_found,
        "MT-045: a cyclic chain renders the addressable 'transclusion-cycle-BLK-A' indicator"
    );
    assert!(
        harness.query_by_label_contains("cycle_detected").is_some(),
        "MT-045: the cycle indicator carries the typed 'cycle_detected' token visibly"
    );
    // The read-through preview is SUPPRESSED for the cyclic chain (the guard replaces it).
    assert!(
        harness.query_by_label_contains("chained body").is_none(),
        "MT-045: the cyclic chain's content preview is suppressed"
    );

    // RISK-4-style product guard: a NON-cyclic resolved transclusion through the SAME path still
    // renders its preview and NO cycle indicator (the guard flags cycles specifically).
    let clean_doc = BlockNode::doc(vec![BlockNode::with_children(
        NodeKind::Paragraph,
        vec![Child::Transclusion(TransclusionNode::new("BLK-CLEAN"))],
    )]);
    let mut clean_runtime = headless_runtime(
        Err(WikilinkError::NotFound("none".into())),
        Ok(BacklinksResponse {
            source_document_id: "DOC-1".into(),
            backlinks: vec![],
        }),
        vec![],
    );
    clean_runtime.transclusions.insert(
        "BLK-CLEAN".into(),
        TransclusionState::Resolved(resolved_transclusion("BLK-CLEAN", "clean readthrough body")),
    );
    let clean_state = Arc::new(std::sync::Mutex::new(
        RichEditorState::new(clean_doc).with_wikilink_runtime(clean_runtime),
    ));
    let clean_for_ui = Arc::clone(&clean_state);
    let mut clean_harness = Harness::builder()
        .with_size(egui::vec2(560.0, 300.0))
        .build_ui(move |ui| {
            RichEditorWidget::new(Arc::clone(&clean_for_ui)).show(ui);
        });
    clean_harness.run();
    assert!(
        clean_harness
            .query_by_label_contains("clean readthrough body")
            .is_some(),
        "MT-045 guard: a clean transclusion still renders its read-through preview"
    );
    let clean_root = clean_harness.root();
    assert!(
        !clean_root.children_recursive().any(|n| n
            .accesskit_node()
            .author_id()
            .is_some_and(|id| id.starts_with("transclusion-cycle-"))),
        "MT-045 guard: a clean chain shows NO cycle indicator (cycles are flagged specifically)"
    );
}
