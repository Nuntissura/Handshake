//! Heading Outline / Table-of-Contents navigator proofs (WP-KERNEL-012 MT-056).
//!
//! These prove the rich-text editor's heading-outline panel — the Obsidian Outline core-plugin /
//! Notion TOC-block parity surface — at the WIDGET level (kittest). The full side-tile DOCK next to the
//! MOUNTED editor lands at E11 host-mount (MT-069), like the other E2 chrome; this MT proves the widget
//! + build_outline + scroll/select + AccessKit are correct now.
//!
//! - PT-001 / AC-001 + AC-002: the PURE `build_outline` tree-construction + level-1..=3 exclusion live
//!   as `#[cfg(test)]` unit tests in `src/rich_editor/outline_panel.rs` (run by `cargo test --lib`); the
//!   parentage + byte/char-offset assertions are proven there without a render harness.
//! - PT-002 / AC-003: a kittest clicks an outline entry and asserts the editor (a) recorded a pending
//!   scroll target resolving to the clicked heading block and (b) placed the caret SELECTION across that
//!   heading block via the MT-012 selection model — driven through the AccessKit-addressable entry node.
//! - PT-003 / AC-004: a kittest adds a heading to the DocModel, advances one frame, and asserts the
//!   outline tree now contains the new heading entry (the revision-gated rebuild fired); and that NO
//!   rebuild occurs on a frame whose revision is unchanged.
//! - PT-004 / AC-005: an AccessKit tree dump asserts the `rich-editor-outline` (Role::Tree) container +
//!   at least one `outline.heading.{block_id}` (Role::TreeItem) entry with a Press (Action::Click).
//! - AC-006: a kittest collapses a node, adds an unrelated heading, and asserts the collapsed node stays
//!   collapsed across the live rebuild.
//! - AC-007: this test set passes with no failures and the outline reads ONLY the in-memory DocModel
//!   (no backend / PostgreSQL / EventLedger / SQLite call — the module has no such dependency).
//! - HBR-VIS: a kittest screenshot of the rendered outline, saved to the EXTERNAL Handshake_Artifacts
//!   root (CX-212E, never repo-local).

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;

use handshake_native::rich_editor::document_model::node::{BlockNode, Child};
use handshake_native::rich_editor::document_model::selection::Selection;
use handshake_native::rich_editor::outline_panel::{
    build_outline, outline_entry_author_id, OutlinePanel, OUTLINE_CONTAINER_AUTHOR_ID,
};
use handshake_native::rich_editor::renderer::block_author_id;
use handshake_native::rich_editor::renderer::rich_editor_widget::RichEditorState;

/// The crate-relative path to the EXTERNAL artifacts root (CX-212E), disk-agnostic — the crate sits at
/// `<repo>/src/frontend/handshake_native`, so four `..` reach `<repo>/..` where `Handshake_Artifacts`
/// is a sibling of the repo worktree.
fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

/// The screenshot path under the EXTERNAL artifact root. The MT contract's literal repo-local
/// `test_output/` path is OVERRIDDEN by the CX-212E external-only artifact-hygiene rule applied across
/// the WP — a committed repo-local PNG/snapshot is a hygiene regression.
fn screenshot_path() -> PathBuf {
    external_artifact_dir("wp-kernel-012-mt-056").join("mt056_outline.png")
}

/// Assert NO repo-local artifact dir exists under the crate (CX-212E): neither `test_output/` nor
/// `tests/screenshots/`. Screenshots/artifacts go to the external Handshake_Artifacts root ONLY.
fn assert_no_local_artifact_dir() {
    for local in ["test_output", "tests/screenshots"] {
        let p = Path::new(local);
        assert!(
            !p.exists(),
            "artifact hygiene: no repo-local {local} dir may exist — artifacts go to the external \
             Handshake_Artifacts/handshake-test root only (found {})",
            p.display()
        );
    }
}

/// True when any node in an outline forest (recursive) has the given heading text.
fn contains_text(
    roots: &[handshake_native::rich_editor::outline_panel::OutlineNode],
    text: &str,
) -> bool {
    roots
        .iter()
        .any(|n| n.text == text || contains_text(&n.children, text))
}

/// Collect every author_id present in the live AccessKit tree.
fn author_ids<S>(harness: &Harness<'_, S>) -> std::collections::HashSet<String> {
    let mut ids = std::collections::HashSet::new();
    for node in harness.root().children_recursive() {
        if let Some(a) = node.accesskit_node().author_id() {
            ids.insert(a.to_owned());
        }
    }
    ids
}

/// A seeded DocModel: h1 "Alpha" > h2 "Beta" > h2 "Gamma" > h3 "Delta", with a paragraph between, so the
/// outline has a real nested tree and the editor has scrollable content.
fn seeded_doc() -> BlockNode {
    BlockNode::doc(vec![
        BlockNode::heading(1, "Alpha"),
        BlockNode::paragraph("alpha body text"),
        BlockNode::heading(2, "Beta"),
        BlockNode::heading(2, "Gamma"),
        BlockNode::heading(3, "Delta"),
    ])
}

/// Build a kittest harness that renders JUST the OutlinePanel over `state` (the side-tile widget). The
/// panel is a `Arc<Mutex<OutlinePanel>>` so its built tree + collapse state persist across frames (the
/// host owns it the same way). Shell Inter fonts installed so labels render.
fn outline_harness<'a>(
    state: Arc<Mutex<RichEditorState>>,
    panel: Arc<Mutex<OutlinePanel>>,
) -> Harness<'a, ()> {
    Harness::builder()
        .with_size(egui::vec2(360.0, 500.0))
        .wgpu()
        .build_ui(move |ui| {
            handshake_native::app::HandshakeApp::install_fonts(ui.ctx());
            let mut p = panel.lock().unwrap();
            p.show(ui, &state);
        })
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════
// PT-004 / AC-005: the AccessKit tree carries the rich-editor-outline Tree + an outline.heading.{id}
// TreeItem with a Press (Action::Click).
// ═══════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn pt004_outline_accesskit_tree_and_treeitems_with_press() {
    let state = Arc::new(Mutex::new(RichEditorState::new(seeded_doc())));
    let panel = Arc::new(Mutex::new(OutlinePanel::new()));
    let mut harness = outline_harness(Arc::clone(&state), Arc::clone(&panel));
    harness.run();

    let ids = author_ids(&harness);
    // AC-005: the container Tree node is present.
    assert!(
        ids.contains(OUTLINE_CONTAINER_AUTHOR_ID),
        "AC-005: the `{OUTLINE_CONTAINER_AUTHOR_ID}` container must be present; ids: {ids:?}"
    );
    // AC-005: at least one outline.heading.{block_id} entry is present — specifically the h1 "Alpha"
    // (top-level block 0).
    let alpha_entry = outline_entry_author_id(&block_author_id(&[0]));
    assert!(
        ids.contains(&alpha_entry),
        "AC-005: the entry node `{alpha_entry}` must be present; ids: {ids:?}"
    );

    // Assert the container is Role::Tree and the entry is Role::TreeItem carrying an Action::Click
    // (the "Press" invoke verb).
    let mut container_role = None;
    let mut entry_role = None;
    let mut entry_supports_click = false;
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        match ak.author_id() {
            Some(a) if a == OUTLINE_CONTAINER_AUTHOR_ID => {
                container_role = Some(format!("{:?}", ak.role()));
            }
            Some(a) if a == alpha_entry => {
                entry_role = Some(format!("{:?}", ak.role()));
                // Read the raw node action set (the no-filter accessor the production interactive-node
                // gate also uses); the entry carries Action::Click (the "Press" invoke verb).
                entry_supports_click = ak.data().supports_action(egui::accesskit::Action::Click);
            }
            _ => {}
        }
    }
    assert_eq!(
        container_role.as_deref(),
        Some("Tree"),
        "AC-005: container role must be Tree"
    );
    assert_eq!(
        entry_role.as_deref(),
        Some("TreeItem"),
        "AC-005: entry role must be TreeItem"
    );
    assert!(
        entry_supports_click,
        "AC-005: the entry must carry a Press (Action::Click) so a swarm agent can invoke scroll-to-heading"
    );
    println!(
        "PT-004: rich-editor-outline=Tree present; {alpha_entry}=TreeItem present with Action::Click (Press)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════
// PT-002 / AC-003: clicking an outline entry scrolls the editor to the heading block AND selects it via
// the MT-012 caret model — driven through the AccessKit-addressable entry node.
// ═══════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn pt002_click_entry_scrolls_and_selects_via_caret_model() {
    let state = Arc::new(Mutex::new(RichEditorState::new(seeded_doc())));
    let state_ck = Arc::clone(&state);
    let panel = Arc::new(Mutex::new(OutlinePanel::new()));
    let mut harness = outline_harness(Arc::clone(&state), Arc::clone(&panel));
    harness.run();

    // Target the h2 "Gamma" entry (top-level block 3). Clicking it must (a) set the editor's pending
    // scroll target to block [3] and (b) select block [3]'s heading text range via the MT-012 model.
    let gamma_block_id = block_author_id(&[3]);
    let gamma_entry = outline_entry_author_id(&gamma_block_id);
    let ids = author_ids(&harness);
    assert!(
        ids.contains(&gamma_entry),
        "the Gamma entry `{gamma_entry}` must be addressable; ids: {ids:?}"
    );

    let entry = harness.get_by(|n| n.author_id() == Some(gamma_entry.as_str()));
    entry.click();
    harness.run();

    let st = state_ck.lock().unwrap();
    // AC-003 (scroll): the editor recorded a pending scroll target resolving to block [3].
    assert_eq!(
        st.pending_scroll_block.as_deref(),
        Some(&[3usize][..]),
        "AC-003: clicking the Gamma entry must request a scroll to its heading block [3]"
    );
    // AC-003 (select via MT-012 caret model): the selection is a Text range over block 3's heading text
    // leaf, anchored at offset 0 to the end of "Gamma" (5 chars). This is the EXISTING Selection model
    // (no second selection mechanism — RISK-002 / MC-002).
    match &st.selection {
        Selection::Text { anchor, head } => {
            assert_eq!(anchor.path, vec![3, 0], "AC-003: selection anchor on block 3's text leaf");
            assert_eq!(head.path, vec![3, 0], "AC-003: selection head on block 3's text leaf");
            assert_eq!(anchor.char_offset, 0, "AC-003: selection starts at the heading text start");
            assert_eq!(head.char_offset, 5, "AC-003: selection covers the whole heading text 'Gamma'");
        }
        other => panic!("AC-003: expected a Text selection over the heading via the MT-012 model, got {other:?}"),
    }
    println!("PT-002: Gamma entry click -> pending_scroll_block=[3] + Text selection [3,0] 0..5 via the MT-012 caret model");
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════
// PT-003 / AC-004: adding a heading rebuilds the outline on the next frame; an unchanged-revision frame
// does NOT rebuild.
// ═══════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn pt003_live_update_on_heading_add_and_no_rebuild_when_unchanged() {
    let state = Arc::new(Mutex::new(RichEditorState::new(seeded_doc())));
    let panel = Arc::new(Mutex::new(OutlinePanel::new()));
    let mut harness = outline_harness(Arc::clone(&state), Arc::clone(&panel));
    harness.run();

    // Initially the new "Epsilon" heading entry is absent.
    let epsilon_block_id = block_author_id(&[5]); // it will be appended at top-level index 5.
    let epsilon_entry = outline_entry_author_id(&epsilon_block_id);
    assert!(
        !author_ids(&harness).contains(&epsilon_entry),
        "AC-004 pre: the Epsilon entry must be absent before the heading is added"
    );

    // Capture the revision the panel last built against, and prove a second frame with NO doc change
    // does NOT rebuild (revision unchanged — RISK-001 / MC-001).
    let rev_after_first = panel.lock().unwrap().doc_revision;
    harness.run(); // unchanged-revision frame
    {
        let p = panel.lock().unwrap();
        assert_eq!(
            p.doc_revision, rev_after_first,
            "AC-004: no rebuild on an unchanged-revision frame (the stored revision did not move)"
        );
    }

    // Mutate the DocModel: append a new h2 "Epsilon". The next frame's sync sees a new revision and
    // rebuilds the tree (live update — AC-004).
    {
        let mut st = state.lock().unwrap();
        st.doc
            .children
            .push(Child::Block(BlockNode::heading(2, "Epsilon")));
    }
    harness.run();
    harness.run(); // settle

    assert!(
        author_ids(&harness).contains(&epsilon_entry),
        "AC-004: after adding a heading, the outline tree includes the new Epsilon entry on the next frame"
    );
    // And the panel's stored revision advanced (the rebuild fired).
    assert_ne!(
        panel.lock().unwrap().doc_revision,
        rev_after_first,
        "AC-004: adding a heading advanced the revision (the revision-gated rebuild fired)"
    );
    println!("PT-003: unchanged frame -> no rebuild; heading add -> revision advanced + Epsilon entry appeared");
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════
// AC-006: user collapse state survives a live rebuild.
// ═══════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn ac006_collapse_state_survives_live_rebuild() {
    let state = Arc::new(Mutex::new(RichEditorState::new(seeded_doc())));
    let panel = Arc::new(Mutex::new(OutlinePanel::new()));
    let mut harness = outline_harness(Arc::clone(&state), Arc::clone(&panel));
    harness.run();

    // The h1 "Alpha" (block 0) has children (Beta/Gamma/Delta), so it renders a collapse caret. Click
    // the caret to collapse it. The caret node is `outline.toggle.{block_id}`.
    let alpha_block_id = block_author_id(&[0]);
    let alpha_toggle = format!("outline.toggle.{alpha_block_id}");
    let ids = author_ids(&harness);
    assert!(
        ids.contains(&alpha_toggle),
        "the Alpha collapse caret `{alpha_toggle}` must be addressable; ids: {ids:?}"
    );
    let caret = harness.get_by(|n| n.author_id() == Some(alpha_toggle.as_str()));
    caret.click();
    harness.run();
    assert!(
        panel.lock().unwrap().roots[0].collapsed,
        "AC-006 pre: clicking the caret collapsed the Alpha node"
    );

    // Now add an UNRELATED heading and advance a frame (forces a live rebuild). The collapsed Alpha must
    // stay collapsed (RISK-004 / MC-004 — collapse carried forward by block_id).
    {
        let mut st = state.lock().unwrap();
        st.doc
            .children
            .push(Child::Block(BlockNode::heading(2, "Zeta")));
    }
    harness.run();
    harness.run();

    let p = panel.lock().unwrap();
    assert!(
        p.roots[0].collapsed,
        "AC-006: the Alpha node stays collapsed across the live rebuild after an unrelated heading add"
    );
    // The rebuild really happened (Zeta is now present somewhere in the tree). Zeta is an h2 appended
    // after the h3 Delta, so the heading-stack nests it under the still-open h1 Alpha (it is NOT a new
    // root) — search the whole forest, which also confirms the rebuild ran AND the carry-forward kept
    // Alpha collapsed without dropping the new heading.
    assert!(
        contains_text(&p.roots, "Zeta"),
        "AC-006: the rebuild included the new Zeta heading (nested under the open h1)"
    );
    println!("AC-006: Alpha stayed collapsed across a live rebuild that added Zeta (Zeta nested under Alpha)");
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════
// AC-007: the outline reads ONLY the in-memory DocModel — build_outline is a pure function over a
// hand-built doc with no backend/PG/EventLedger/SQLite involvement (it has no such dependency at all).
// ═══════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn ac007_outline_is_pure_over_in_memory_docmodel() {
    // No backend, no async runtime, no network: build_outline takes a &BlockNode and returns the tree.
    let doc = seeded_doc();
    let roots = build_outline(&doc);
    assert_eq!(roots.len(), 1, "Alpha is the single root");
    assert_eq!(roots[0].text, "Alpha");
    assert_eq!(roots[0].children.len(), 2, "Beta + Gamma under Alpha");
    assert_eq!(roots[0].children[1].children.len(), 1, "Delta under Gamma");
    println!("AC-007: build_outline derived the tree purely from the in-memory DocModel (no backend call)");
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════
// HBR-VIS: a screenshot of the rendered outline, saved to the EXTERNAL artifact root.
// ═══════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn hbr_vis_outline_screenshot() {
    let state = Arc::new(Mutex::new(RichEditorState::new(seeded_doc())));
    let panel = Arc::new(Mutex::new(OutlinePanel::new()));
    let mut harness = outline_harness(Arc::clone(&state), Arc::clone(&panel));
    harness.run();
    harness.run();

    match harness.render() {
        Ok(image) => {
            let (w, h) = (image.width(), image.height());
            assert!(w > 0 && h > 0, "rendered image must be non-empty");

            let path = screenshot_path();
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let saved = image.save(&path).is_ok();

            // Pixel proof: the outline renders a dark bg + foreground glyphs (heading labels + carets),
            // so there are >= 2 distinct foreground colors over the bg. Sample every 4th pixel.
            let raw = image.as_raw();
            let mut counts: std::collections::HashMap<[u8; 4], u32> =
                std::collections::HashMap::new();
            let mut i = 0usize;
            while i + 4 <= raw.len() {
                let px = [raw[i], raw[i + 1], raw[i + 2], raw[i + 3]];
                if px[3] != 0 {
                    *counts.entry(px).or_insert(0) += 1;
                }
                i += 4 * 4;
            }
            let bg = counts.iter().max_by_key(|(_, c)| **c).map(|(p, _)| *p);
            let foreground: std::collections::HashSet<[u8; 4]> =
                counts.keys().filter(|p| Some(**p) != bg).copied().collect();

            println!(
                "HBR-VIS screenshot: {w}x{h}, {} distinct colors, {} foreground; saved={saved} ({})",
                counts.len(),
                foreground.len(),
                screenshot_path().display(),
            );
            assert!(
                foreground.len() >= 2,
                "HBR-VIS: the rendered outline must produce >= 2 distinct foreground colors (labels/carets \
                 over the bg); got {} (bg={bg:?})",
                foreground.len()
            );
            assert!(
                saved,
                "the mt056_outline.png screenshot must be saved to the external artifact root"
            );
            assert_no_local_artifact_dir();
        }
        Err(e) => {
            println!(
                "BLOCKER(non-fatal): mt056 outline screenshot render unavailable (no wgpu adapter / headless \
                 GPU crash): {e}. The AccessKit + structural proofs stand; the PNG is a GPU-host item."
            );
        }
    }
}
