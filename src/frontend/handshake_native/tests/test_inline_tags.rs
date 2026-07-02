//! WP-KERNEL-012 MT-058 inline `#tag` authoring PROOFS: pure parser/boundary/normalization unit
//! tests, the convergence dedupe-builder proof, and kittest interaction proofs that mount the LIVE
//! `RichEditorWidget`, type a `#` trigger, commit through the LIVE menu, render the chip through the
//! LIVE MT-012/015 chip pipeline, and click it to emit a `TagActivated` event onto the LIVE bus + an
//! AccessKit dump asserting the `inline-tag-{name}` Role::Link node.
//!
//! Artifact hygiene (CX-212E): EVERY PNG is written ONLY to the EXTERNAL
//! `Handshake_Artifacts/handshake-test/wp-kernel-012-mt-058/` root via [`external_artifact_dir`];
//! [`assert_no_local_artifact_dir`] fails the run if a repo-local `tests/screenshots/` or
//! `test_output/` directory exists (the CX-212E artifact rule OVERRIDES any repo-local path the MT
//! contract might name — a tracked PNG under src/ is a hygiene failure the reviewer greps for with
//! `git ls-files "src/**/*.png"`).
//!
//! Backend reality (Spec-Realism Gate): the parser, normalization, chip render, `#` trigger detection,
//! menu commit, click->TagActivated, AccessKit dump, and the convergence dedupe are FULLY proven here
//! with NO live backend. The LIVE `POST /loom/edges` convergence round-trip is the gated
//! NEEDS_MANAGED_RESOURCE_PROOF item (it needs a live managed PostgreSQL + per-canonical tag_hub block
//! resolution — the backend tags an edge into a tag_hub BLOCK, not a name string); the deduped
//! edge-PAYLOAD builder is proven standalone here (AC-005, which the contract explicitly allows).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;

use handshake_native::rich_editor::document_model::node::{BlockNode, Child, NodeKind, TextLeaf};
use handshake_native::rich_editor::inline_tags::{
    build_tag_edge_payload, inline_tag_author_id, parse_inline_tags, tag_menu_items,
    tag_to_hs_link, Tag,
};
use handshake_native::rich_editor::renderer::rich_editor_widget::{
    RichEditorState, RichEditorWidget,
};
use handshake_native::rich_editor::wikilinks::inline_view::EditorEvent;

/// The crate-relative path to the EXTERNAL artifacts root (CX-212E), disk-agnostic. The crate sits at
/// `<repo>/src/frontend/handshake_native`, so four `..` reach `<repo>/..` where `Handshake_Artifacts`
/// is a sibling of the repo worktree.
fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

/// Assert NO repo-local artifact directory exists under the crate (CX-212E hygiene). Checks BOTH
/// `test_output/` and `tests/screenshots/` (the path any contract might name, which this rule
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

/// Serialize the `.wgpu()` screenshot tests (the documented Windows wgpu concurrency hazard — mirrors
/// the crate's `WGPU_SERIAL_GUARD` idiom in test_wikilinks.rs). Each wgpu test holds the lock for its
/// Harness lifetime so at most one wgpu device exists at a time.
static WGPU_SERIAL_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn wgpu_guard() -> std::sync::MutexGuard<'static, ()> {
    WGPU_SERIAL_GUARD.lock().unwrap_or_else(|p| p.into_inner())
}

/// A doc with one paragraph carrying a leading text run + a committed inline-tag hsLink atom (the
/// chip). The tag atom is built via the production `tag_to_hs_link` so the test exercises the real
/// node shape (ref_kind="tag", ref_value=canonical, label="#name").
fn doc_with_tag_chip(name: &str) -> BlockNode {
    let link = tag_to_hs_link(&Tag::new(name));
    BlockNode::doc(vec![BlockNode::with_children(
        NodeKind::Paragraph,
        vec![Child::Text(TextLeaf::new("learning ")), Child::HsLink(link)],
    )])
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// PT-001: pure parser/boundary/normalization (PT-001 / AC-001 / AC-002 / MC-001 / MC-002 / MC-003).
// These also run inside the crate as #[cfg(test)] units; duplicated here as an integration-level guard
// that the PUBLIC API extracts the contract's exact corpus.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn pt001_parse_extracts_rust_and_wip_in_order_ac002() {
    // AC-002: parse_inline_tags('learning #rust and #wip today') -> exactly two TagTokens [rust, wip],
    // each byte_range covering the FULL `#tag` literal including the leading `#`.
    let text = "learning #rust and #wip today";
    let toks = parse_inline_tags(text);
    assert_eq!(toks.len(), 2, "exactly two tags (got {toks:?})");
    assert_eq!(toks[0].tag.name, "rust");
    assert_eq!(toks[1].tag.name, "wip");
    assert_eq!(
        &text[toks[0].byte_range.clone()],
        "#rust",
        "byte_range includes the leading #"
    );
    assert_eq!(&text[toks[1].byte_range.clone()], "#wip");
}

#[test]
fn pt001_word_boundary_adversarial_corpus_mc002() {
    // MC-002 adversarial corpus: '#rust' yes / 'C#' no / 'a#b' no / '#' alone no / '# foo' no /
    // 'see #wip,' yes(trailing punct) / '#area/sub' nested yes / UTF-8 emoji before '#tag' byte-safe.
    assert_eq!(parse_inline_tags("#rust").len(), 1, "#rust is a tag");
    assert!(parse_inline_tags("C#").is_empty(), "C# is NOT a tag");
    assert!(parse_inline_tags("a#b").is_empty(), "a#b is NOT a tag");
    assert!(parse_inline_tags("#").is_empty(), "bare # is NOT a tag");
    assert!(
        parse_inline_tags("# foo").is_empty(),
        "# foo (empty body) is NOT a tag"
    );

    let trailing = parse_inline_tags("see #wip, today");
    assert_eq!(trailing.len(), 1, "trailing punctuation ends the tag");
    assert_eq!(trailing[0].tag.name, "wip");

    let nested = parse_inline_tags("#area/sub");
    assert_eq!(nested.len(), 1);
    assert_eq!(nested[0].tag.name, "area/sub", "nested tag keeps the slash");

    // MC-003: UTF-8 emoji before the tag is byte-safe (no mis-slice).
    let utf8 = parse_inline_tags("🚀 #rust");
    assert_eq!(utf8.len(), 1, "the tag after an emoji is found byte-safely");
    assert_eq!(utf8[0].tag.name, "rust");
}

#[test]
fn pt001_normalization_is_the_one_identity_mc001() {
    // MC-001 / RISK-001: `#Rust` and a property tag 'Rust' resolve to the IDENTICAL canonical key, so
    // they collapse to one identity (the one-tag-one-hub invariant). The convergence builder proves the
    // edge dedupe; here we assert the underlying normalization agreement.
    assert_eq!(Tag::new("Rust").canonical(), Tag::new("rust").canonical());
    assert_eq!(Tag::new("Rust").canonical(), "rust");
    assert_eq!(
        Tag::new("#Rust").canonical(),
        "rust",
        "a leading # is stripped before normalization"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// PT-005 / AC-005: convergence — inline #rust + property tag 'rust' -> EXACTLY ONE deduped edge.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn pt005_convergence_dedupes_inline_and_property_to_one_edge_ac005() {
    // AC-005: a document with inline #rust + a property tag 'rust' persists EXACTLY ONE loom edge for
    // 'rust' (no duplicate) — proven by the deduped edge-PAYLOAD builder (the LIVE POST is gated).
    let payload = build_tag_edge_payload(["rust", "wip"], ["Rust"]);
    assert_eq!(
        payload.len(),
        2,
        "rust (inline+property) -> one edge; +wip -> {:?}",
        payload.canonical_set()
    );
    let rust_count = payload
        .edges
        .iter()
        .filter(|e| e.canonical == "rust")
        .count();
    assert_eq!(
        rust_count, 1,
        "AC-005: exactly ONE 'rust' edge despite inline + property occurrence"
    );
    assert!(payload.canonical_set().contains(&"wip".to_owned()));
}

#[test]
fn pt005_convergence_from_live_document_collects_inline_atoms() {
    // The state-level collector walks the committed tag atoms and unions them with the property tags;
    // a document with inline #rust + a property tag 'rust' yields exactly one edge end-to-end.
    let doc = doc_with_tag_chip("rust");
    let mut state = RichEditorState::new(doc);
    // Seed the MT-017 property-tag set with 'rust' (the local-only list) to force the convergence.
    state.properties = Some(make_properties_with_tags(&["rust", "design"]));

    let inline = state.collect_inline_tags();
    assert_eq!(
        inline,
        vec!["rust".to_owned()],
        "the committed inline #rust atom is collected"
    );

    let payload = state.build_tag_edge_payload_for_save();
    let rust_count = payload
        .edges
        .iter()
        .filter(|e| e.canonical == "rust")
        .count();
    assert_eq!(
        rust_count, 1,
        "AC-005 end-to-end: inline #rust + property 'rust' -> one edge"
    );
    assert!(
        payload.canonical_set().contains(&"design".to_owned()),
        "the property-only tag persists too"
    );
    assert_eq!(payload.len(), 2, "rust (deduped) + design");
}

/// Build a MT-017 PropertiesState carrying a local-only tag list (the convergence property half).
fn make_properties_with_tags(
    tags: &[&str],
) -> handshake_native::rich_editor::properties::PropertiesState {
    use handshake_native::rich_editor::properties::metadata_client::DocMetadata;
    use handshake_native::rich_editor::properties::PropertiesState;
    let meta = DocMetadata {
        rich_document_id: "KRD-1".into(),
        workspace_id: "ws".into(),
        title: "Doc".into(),
        doc_version: 1,
        authority_label: "draft".into(),
        owner_actor_kind: None,
        owner_actor_id: None,
        project_ref: None,
        folder_ref: None,
        crdt_document_id: None,
        created_at: "2026-06-25T00:00:00Z".into(),
        updated_at: "2026-06-25T00:00:00Z".into(),
    };
    let mut st = PropertiesState::new(meta);
    for t in tags {
        st.add_tag(*t);
    }
    st
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// AC-006: the menu lists existing tags + always allows a free-typed NEW tag.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn ac006_menu_lists_existing_and_offers_free_typed_new() {
    let available = vec!["rust".to_owned(), "rustaceans".to_owned()];
    // An existing prefix query lists the matches and does NOT offer 'rust' as new (it exists).
    let items = tag_menu_items("rust", &available);
    assert!(items
        .iter()
        .any(|i| i.tag.canonical() == "rust" && !i.is_new));
    assert!(items.iter().any(|i| i.tag.canonical() == "rustaceans"));
    // A brand-new tag offers a create row (AC-006).
    let items2 = tag_menu_items("brandnew", &available);
    assert!(
        items2
            .iter()
            .any(|i| i.is_new && i.tag.canonical() == "brandnew"),
        "AC-006: free-typed new tag committable"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// PT-004 / AC-004 (kittest AccessKit dump + screenshot): a committed inline tag renders as a clickable
// chip with an `inline-tag-{name}` Role::Link AccessKit node.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn pt004_inline_tag_chip_accesskit_and_screenshot_ac004() {
    let _wgpu_guard = wgpu_guard();
    let doc = doc_with_tag_chip("rust");
    let state = Arc::new(std::sync::Mutex::new(RichEditorState::new(doc)));
    let state_for_ui = Arc::clone(&state);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(520.0, 240.0))
        .wgpu()
        .build_ui(move |ui| {
            RichEditorWidget::new(Arc::clone(&state_for_ui)).show(ui);
        });
    harness.run();
    harness.run();

    // AC-004 / PT-004: the chip is addressable by `inline-tag-rust` with Role::Link in the LIVE tree.
    let expected_author = inline_tag_author_id(&Tag::new("rust"));
    assert_eq!(expected_author, "inline-tag-rust");
    let root = harness.root();
    let mut chip_found = false;
    let mut chip_is_link = false;
    for node in root.children_recursive() {
        if node.accesskit_node().author_id() == Some(expected_author.as_str()) {
            chip_found = true;
            chip_is_link = node.accesskit_node().role() == egui::accesskit::Role::Link;
            break;
        }
    }
    assert!(
        chip_found,
        "AC-004: the inline tag renders an addressable '{expected_author}' chip node"
    );
    assert!(
        chip_is_link,
        "AC-004: the inline tag chip node carries Role::Link"
    );

    match harness.render() {
        Ok(image) => {
            assert!(image.width() > 0 && image.height() > 0, "rendered image non-empty");
            let ext_dir = external_artifact_dir("wp-kernel-012-mt-058");
            let _ = std::fs::create_dir_all(&ext_dir);
            let path = ext_dir.join("mt058_inline_tag_chip.png");
            let saved = image.save(&path).is_ok();
            println!("PT-004 inline tag chip: {}x{} saved={saved} ({})", image.width(), image.height(), path.display());
        }
        Err(e) => println!(
            "BLOCKER(non-fatal): mt058_inline_tag_chip screenshot render unavailable (no wgpu adapter): {e}. \
             The AccessKit chip-node structural proof passed; the PNG is a GPU-host item."
        ),
    }
    assert_no_local_artifact_dir();
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// PT-003 / AC-003: clicking the rendered `#rust` chip emits a TagActivated event onto the LIVE bus.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn pt003_inline_tag_chip_click_emits_tag_activated_ac003() {
    let doc = doc_with_tag_chip("rust");
    let state = Arc::new(std::sync::Mutex::new(RichEditorState::new(doc)));
    let state_for_ui = Arc::clone(&state);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(520.0, 220.0))
        .build_ui(move |ui| {
            RichEditorWidget::new(Arc::clone(&state_for_ui)).show(ui);
        });
    harness.run();

    // Click the chip: the only Role::Link node in the tree is the tag chip.
    {
        let node = harness.get_by_role(egui::accesskit::Role::Link);
        node.click();
    }
    harness.run();
    harness.run();

    // AC-003 / MC-005: the click emits a TagActivated{canonical:'rust'} event onto the LIVE bus (NOT a
    // WikilinkActivated, and NOT a direct hub-window open).
    let events = state.lock().unwrap().pending_events.clone();
    let found = events.iter().any(|e| {
        matches!(
            e,
            EditorEvent::TagActivated { canonical, .. } if canonical == "rust"
        )
    });
    assert!(
        found,
        "AC-003: clicking the chip emits TagActivated{{canonical:'rust'}} (got {events:?})"
    );
    // RISK-005 / MC-005: it is NOT a WikilinkActivated event (the tag is not routed as a wikilink).
    let is_wikilink = events
        .iter()
        .any(|e| matches!(e, EditorEvent::WikilinkActivated { .. }));
    assert!(
        !is_wikilink,
        "MC-005: a tag chip emits TagActivated, never WikilinkActivated"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// PT-002 / AC-001 / AC-003 (kittest interaction): type `#ru`, select 'rust' from the LIVE menu, press
// Enter -> the LIVE doc gains a Tag mark + the LIVE render shows the chip. A mid-word `#` (C#) does NOT
// open the menu.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn pt002_type_hash_open_menu_select_commit_chip_live() {
    // Start with an empty paragraph, focus the editor, seed the cached tag list with 'rust', type
    // `#ru`, assert the LIVE menu opened, press Enter (selecting the top row 'rust'), assert the LIVE
    // doc now has a committed tag atom + the LIVE render shows an `inline-tag-rust` chip node.
    let doc = BlockNode::doc(vec![BlockNode::with_children(
        NodeKind::Paragraph,
        vec![Child::Text(TextLeaf::new(""))],
    )]);
    let state = Arc::new(std::sync::Mutex::new({
        let mut st = RichEditorState::new(doc);
        st.set_tag_list(vec!["rust".to_owned(), "rustaceans".to_owned()]);
        st
    }));
    let state_for_ui = Arc::clone(&state);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(520.0, 280.0))
        .build_ui(move |ui| {
            RichEditorWidget::new(Arc::clone(&state_for_ui)).show(ui);
        });
    harness.run();

    // Focus the editor surface (the same focus an out-of-process agent requests by the stable id).
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

    // Type `#ru` -> the LIVE `#` trigger opens the menu with the query 'ru'.
    harness.event(egui::Event::Text("#ru".into()));
    harness.step();
    harness.step();
    {
        let st = state.lock().unwrap();
        assert!(
            st.tag_autocomplete.is_some(),
            "AC-001: typing `#` at a word boundary opens the tag menu"
        );
        assert_eq!(
            st.tag_autocomplete.as_ref().unwrap().query,
            "ru",
            "the live query is the typed body"
        );
        assert_eq!(
            st.block_plain_text(0).as_deref(),
            Some("#ru"),
            "the `#ru` trigger text is in the leaf"
        );
    }
    // The LIVE menu AccessKit container is present.
    {
        let root = harness.root();
        let menu_found = root
            .children_recursive()
            .any(|n| n.accesskit_node().author_id() == Some("inline-tag-menu"));
        assert!(
            menu_found,
            "the LIVE 'inline-tag-menu' popup node is in the accessibility tree"
        );
    }

    // Press Enter -> commit the top row ('rust') as a tag atom; the menu closes.
    harness.key_press(egui::Key::Enter);
    harness.step();
    harness.step();
    {
        let st = state.lock().unwrap();
        assert!(
            st.tag_autocomplete.is_none(),
            "the menu closes after commit"
        );
        // The LIVE doc now has a committed inline-tag atom for 'rust'.
        let inline = st.collect_inline_tags();
        assert!(
            inline.iter().any(|t| Tag::new(t).canonical() == "rust"),
            "PT-002: the LIVE doc gained a committed #rust tag atom (got {inline:?})"
        );
    }
    // The LIVE render shows the chip (the `inline-tag-rust` Role::Link node). The editor is FOCUSED, so
    // it requests a continuous blink repaint — advance with step() (single frame), because run()
    // requires convergence and would trip its step cap on the blink repaint (the same reason the MT-015
    // wikilink interaction test uses step() throughout).
    harness.step();
    harness.step();
    {
        let root = harness.root();
        let chip_found = root
            .children_recursive()
            .any(|n| n.accesskit_node().author_id() == Some("inline-tag-rust"));
        assert!(
            chip_found,
            "PT-002: the LIVE render shows the committed `inline-tag-rust` chip"
        );
    }
}

#[test]
fn ac001_mid_word_hash_does_not_open_menu() {
    // AC-001: a `#` preceded by a word char (`C#`) does NOT open the menu (and does not produce a tag).
    let doc = BlockNode::doc(vec![BlockNode::with_children(
        NodeKind::Paragraph,
        vec![Child::Text(TextLeaf::new(""))],
    )]);
    let state = Arc::new(std::sync::Mutex::new(RichEditorState::new(doc)));
    let state_for_ui = Arc::clone(&state);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(420.0, 180.0))
        .build_ui(move |ui| {
            RichEditorWidget::new(Arc::clone(&state_for_ui)).show(ui);
        });
    harness.run();
    {
        let root = harness.root();
        let surface = root
            .children_recursive()
            .find(|n| n.accesskit_node().author_id() == Some("rich-editor-surface"))
            .expect("editor surface");
        surface.focus();
    }
    harness.step();
    harness.step();
    // Type `C#` — the `#` is preceded by the word char 'C', so it is NOT a tag trigger.
    harness.event(egui::Event::Text("C#".into()));
    harness.step();
    harness.step();
    {
        let st = state.lock().unwrap();
        assert!(
            st.tag_autocomplete.is_none(),
            "AC-001: a mid-word `#` (C#) does NOT open the tag menu"
        );
        assert!(
            st.collect_inline_tags().is_empty(),
            "AC-001: a mid-word `#` produces no tag atom"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// MC-006: two `#rust` occurrences in one doc render valid, non-colliding AccessKit nodes.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn mc006_repeated_tag_renders_valid_non_colliding_tree() {
    // Two committed #rust atoms in one paragraph: both carry the SAME `inline-tag-rust` author_id (a
    // match key, not a uniqueness key) but distinct egui NodeIds, so the AccessKit tree is valid (no
    // duplicate NodeId). We assert the shell's interactive-naming gate passes on the LIVE tree.
    let rust1 = tag_to_hs_link(&Tag::new("rust"));
    let rust2 = tag_to_hs_link(&Tag::new("rust"));
    let doc = BlockNode::doc(vec![BlockNode::with_children(
        NodeKind::Paragraph,
        vec![
            Child::Text(TextLeaf::new("a ")),
            Child::HsLink(rust1),
            Child::Text(TextLeaf::new(" and ")),
            Child::HsLink(rust2),
        ],
    )]);
    let state = Arc::new(std::sync::Mutex::new(RichEditorState::new(doc)));

    // Build the LIVE accesskit::TreeUpdate the out-of-process adapter receives by running one frame on a
    // real egui::Context (the same emission path the window uses — a node only built in memory would be
    // absent). The interactive-naming gate panics on a duplicate-NodeId / unnamed-interactive tree; a
    // clean pass over a doc with TWO `#rust` chips proves MC-006 (the repeated tag does not collide).
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let state_for_run = Arc::clone(&state);
    // Run twice so layout settles before the AccessKit emission frame.
    for _ in 0..2 {
        let st = Arc::clone(&state_for_run);
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                RichEditorWidget::new(Arc::clone(&st)).show(ui);
            });
        });
    }
    let st = Arc::clone(&state_for_run);
    let output = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            RichEditorWidget::new(Arc::clone(&st)).show(ui);
        });
    });
    let update = output
        .platform_output
        .accesskit_update
        .expect("AccessKit update produced (accesskit enabled + one frame run)");

    let inspected = handshake_native::accessibility::assert_no_unnamed_interactive(&update);
    assert!(inspected >= 2, "MC-006: at least the two tag chips are inspected as named interactive nodes (got {inspected})");

    // Both chips carry the same addressable author_id (a swarm agent addresses either by `inline-tag-rust`),
    // and their NodeIds are DISTINCT (a valid, non-colliding tree) — assert at least two such nodes exist.
    let tag_nodes = update
        .nodes
        .iter()
        .filter(|(_, node)| node.author_id() == Some("inline-tag-rust"))
        .count();
    assert!(
        tag_nodes >= 2,
        "MC-006: both #rust occurrences emit a distinct `inline-tag-rust` node (got {tag_nodes})"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// WP-KERNEL-012 MT-020 — LIVE-WIDGET undo-after-insert (tag commit path): a real `#ru` keystroke
// decode opens the LIVE menu, a real Enter commits the tag atom, and a REAL Ctrl+Z through the
// mounted widget removes the atom and restores the exact pre-commit doc (the `#ru` trigger text)
// via the MT-035 unified undo bus; a second Ctrl+Z unwinds the typing too.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn mt020_live_tag_commit_undo_restores_pre_insert_doc() {
    use handshake_native::interop::interaction_bus::InteractionBus;
    use handshake_native::pane_registry::PaneId;

    let doc = BlockNode::doc(vec![BlockNode::with_children(
        NodeKind::Paragraph,
        vec![Child::Text(TextLeaf::new(""))],
    )]);
    let rich_pane: PaneId = Arc::from("pane-mt020-tag");
    let state = Arc::new(std::sync::Mutex::new({
        let mut st = RichEditorState::new(doc);
        st.set_tag_list(vec!["rust".to_owned()]);
        st.undo_pane_id = Some(rich_pane.clone());
        st
    }));
    let state_for_ui = Arc::clone(&state);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(520.0, 280.0))
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

    // REAL keystroke decode: `#ru` opens the LIVE tag menu.
    harness.event(egui::Event::Text("#ru".into()));
    harness.step();
    harness.step();
    {
        let st = state.lock().unwrap();
        assert!(st.tag_autocomplete.is_some(), "the `#` trigger opened");
        assert_eq!(st.block_plain_text(0).as_deref(), Some("#ru"));
    }
    // Break the 500ms undo-coalescing window so the commit records its OWN bus entry.
    std::thread::sleep(std::time::Duration::from_millis(600));
    let pre_insert_json = {
        let st = state.lock().unwrap();
        handshake_native::rich_editor::document_model::doc_json::to_content_json_value(&st.doc)
    };

    // REAL Enter commits the top row ('rust') through the live key handler.
    harness.key_press(egui::Key::Enter);
    harness.step();
    harness.step();
    {
        let st = state.lock().unwrap();
        assert!(st.tag_autocomplete.is_none(), "the menu closed on commit");
        let inline = st.collect_inline_tags();
        assert!(
            inline.iter().any(|t| Tag::new(t).canonical() == "rust"),
            "the LIVE doc gained a committed #rust tag atom (got {inline:?})"
        );
    }
    let bus = InteractionBus::get_or_init(&harness.ctx);
    let depth = InteractionBus::with_try_lock(&bus, |b| b.local_undo_count(&rich_pane))
        .expect("bus lock");
    assert_eq!(
        depth, 2,
        "MT-020: the tag commit recorded its own unified-bus entry (typing + commit)"
    );

    // REAL Ctrl+Z: the atom goes; the doc equals the exact pre-commit doc (`#ru` restored).
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    harness.step();
    harness.step();
    {
        let st = state.lock().unwrap();
        assert!(
            st.collect_inline_tags().is_empty(),
            "MT-020: Ctrl+Z removed the committed tag atom"
        );
        let now_json =
            handshake_native::rich_editor::document_model::doc_json::to_content_json_value(
                &st.doc,
            );
        assert_eq!(
            now_json, pre_insert_json,
            "MT-020: undo restored the EXACT pre-commit doc (the `#ru` trigger text)"
        );
    }

    // A second Ctrl+Z unwinds the typing itself.
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    harness.step();
    harness.step();
    {
        let st = state.lock().unwrap();
        assert_eq!(
            st.block_plain_text(0).as_deref(),
            Some(""),
            "undo #2 removed the `#ru` typing — the stack unwinds in order"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// WP-KERNEL-012 MT-058 — the tag-edge SAVE HOOK: `build_tag_edge_payload_for_save` is called from the
// REAL save path (`request_save_for_host` — the Ctrl+S / host-menu / pane-save funnel) and the edge
// intent is QUEUED (dispatch stays gated on the tag_hub block-id typed blocker). MC-004: fires only
// at document commit/save (and only when the save actually dispatched), never per keystroke.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

/// A no-op save backend for the headless SaveManager (the queue test never resolves the HTTP call —
/// the queued intent is captured synchronously at dispatch time).
struct NoopSaveBackend;
impl handshake_native::rich_editor::save::save_manager::SaveBackend for NoopSaveBackend {
    fn save_document(
        &self,
        _id: &str,
        _c: serde_json::Value,
        _v: u64,
    ) -> handshake_native::rich_editor::save::save_manager::SaveFuture {
        Box::pin(async move {
            Err(
                handshake_native::rich_editor::save::save_manager::SaveError::Network(
                    "noop test backend".into(),
                ),
            )
        })
    }
}

#[test]
fn mt058_request_save_for_host_queues_tag_edge_intent() {
    use handshake_native::rich_editor::inline_tags::TAG_EDGE_DISPATCH_BLOCKER;
    use handshake_native::rich_editor::save::save_manager::{SaveManager, SaveState};

    // A doc with a committed inline #rust atom + property tags ['Rust', 'design'] (the dedupe case).
    let doc = doc_with_tag_chip("rust");
    let mut state = RichEditorState::new(doc);
    state.properties = Some(make_properties_with_tags(&["Rust", "design"]));
    state.save = Some(SaveManager::new(Arc::new(NoopSaveBackend), None, "KRD-1", 7));
    assert!(
        state.pending_tag_edge_intent.is_none(),
        "no intent before any save (MC-004: never per keystroke)"
    );

    // The REAL save funnel queues the intent for the dispatched save.
    assert!(state.request_save_for_host(), "the save dispatched");
    let intent = state
        .pending_tag_edge_intent
        .clone()
        .expect("MT-058: the save path queued a tag-edge intent");
    assert_eq!(intent.document_id, "KRD-1");
    assert_eq!(intent.doc_version, 7, "provenance: the dispatching version");
    assert_eq!(
        intent.dispatch_blocked_on, TAG_EDGE_DISPATCH_BLOCKER,
        "dispatch stays gated on the tag_hub block-id typed blocker (never silently POSTed)"
    );
    // The payload is the DEDUPED inline+property union: rust (inline #rust + property 'Rust' -> ONE
    // edge) + design.
    assert_eq!(intent.payload.len(), 2, "rust (deduped) + design");
    assert_eq!(
        intent
            .payload
            .edges
            .iter()
            .filter(|e| e.canonical == "rust")
            .count(),
        1,
        "AC-005/MC-004: inline #rust + property 'Rust' converge to exactly one edge"
    );
    assert!(
        intent.payload.canonical_set().contains(&"design".to_owned()),
        "the property-only tag is in the union"
    );

    // MC-002 mirror: while the first save is IN FLIGHT, a second request is a swallowed no-op and
    // must NOT queue an intent for content that never dispatched.
    state.pending_tag_edge_intent = None;
    assert!(
        state.save.as_ref().unwrap().is_saving(),
        "the first save is in flight"
    );
    assert!(state.request_save_for_host(), "save context still present");
    assert!(
        state.pending_tag_edge_intent.is_none(),
        "MT-058: a swallowed (in-flight-guarded) save queues NO intent"
    );

    // Latest-wins: once the save slot frees, a new dispatched save's tag set supersedes.
    state.save.as_mut().unwrap().state = SaveState::Idle;
    if let Some(props) = state.properties.as_mut() {
        props.add_tag("extra");
    }
    assert!(state.request_save_for_host());
    let intent2 = state
        .pending_tag_edge_intent
        .clone()
        .expect("the fresh dispatch re-queued");
    assert!(
        intent2.payload.canonical_set().contains(&"extra".to_owned()),
        "latest-wins: the newer save's tag union supersedes the older queued intent"
    );
}
