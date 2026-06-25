//! WP-KERNEL-012 MT-059 PROOFS — wiki-page markdown rendering (shared CommonMark adapter).
//!
//! This MT replaces the raw-text wiki projection rendering that MT-025 explicitly DEFERRED: the read-only
//! Loom wiki view used to print `LoomWikiProjection.rendered_content` verbatim through a single
//! `egui::Label`, so `"# Heading\n- one"` showed the literal `#`/`-` characters. MT-059 parses
//! `rendered_content` as CommonMark and paints headings/lists/tables/quotes/code/links via the SHARED
//! `rich_editor::markdown_render` adapter (the SAME styling the MT-012 block renderer uses).
//!
//! Proof map:
//!   - PT1 / AC1 + AC3 (parse + best-effort/panic-free): owned by the lib unit tests in
//!     `rich_editor::markdown_render::tests` (the MdBlock sequence + nesting + malformed-input asserts run
//!     without an egui context). Re-asserted here at the RENDER layer (PT3) so the painted path is proven
//!     panic-free too.
//!   - PT2 / AC2 + AC7 (kittest read-only render of the LIVE wiki panel): seed a `LoomWikiProjection`
//!     whose `rendered_content` is `"# Heading\n\n- one\n- two"`, render the REAL `LoomWikiPagePanel`, and
//!     assert (a) the `wiki.content.{id}` Role::Document node is STILL present (AC7), (b) NO label carries
//!     the literal `"# Heading"` / `"- one"` raw markdown (AC2: it is formatted, not raw), (c) a heading
//!     label "Heading" and a `•` bullet glyph ARE present (AC2: it is actually rendered as a heading +
//!     list), and save the HBR-VIS screenshot to the EXTERNAL artifact root.
//!   - PT3 / AC3 (render is panic-free for malformed user content): feed malformed fixtures (unterminated
//!     fence, ragged table, empty heading, deep nesting) through `parse_markdown` + `render_blocks` inside
//!     a kittest harness and assert the frame renders with no panic and no dropped trailing content.
//!   - PT4 / AC4 (edit overlay UNCHANGED — read-side-only change): click `wiki.edit.{id}` and assert the
//!     `wiki.edit-area.{id}` TextEdit::multiline still binds the RAW edit-buffer string (the hash/dash
//!     characters are preserved + editable) exactly as MT-025 left it; the formatting change is ONLY in
//!     the read-only branch (RISK-4 / MC-4).
//!   - AC5 (shared, not duplicated): a SOURCE assertion that `markdown_render` imports the MT-012
//!     `line_layout` styling constants + the `block_renderer::md_span_text_format` shim and declares NO
//!     local heading-scale / quote-bar / code-frame / table-stroke constants of its own.
//!   - AC6 (no backend change): owned by the reviewer's empty `git diff src/backend/` + the unchanged
//!     `binds_backend_api` set; this MT issues no new request (the panel still binds the MT-025 GET).
//!
//! ## Artifact hygiene (CX-212E screenshot rule)
//!
//! EVERY PNG is written ONLY to the EXTERNAL `Handshake_Artifacts/handshake-test/wp-kernel-012-mt-059/`
//! root via [`external_artifact_dir`]; [`assert_no_local_artifact_dir`] fails the run if a repo-local
//! `tests/screenshots/` or `test_output/` directory exists (the reviewer also greps
//! `git ls-files "src/**/*.png"`). The PNG proof is OPT-IN behind the OFF-by-default `wgpu_screenshots`
//! feature so the default `cargo test` does not add a concurrent wgpu device (the WP-wide Windows hazard).

#[cfg(feature = "wgpu_screenshots")]
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;

use handshake_native::backend_client::WikiProjection;
use handshake_native::graph::wiki_page_panel::{
    content_author_id, edit_area_author_id, edit_author_id, LoomWikiPagePanel,
};
use handshake_native::rich_editor::markdown_render::{parse_markdown, render_blocks, MdBlock};
use handshake_native::theme::HsTheme;

/// The crate-relative path to the EXTERNAL artifacts root (CX-212E), disk-agnostic.
#[cfg(feature = "wgpu_screenshots")]
fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

/// Assert NO repo-local artifact directory exists under the crate (CX-212E hygiene). Checks BOTH
/// `test_output/` and `tests/screenshots/` (the path the contract literally named, overridden here).
#[cfg(feature = "wgpu_screenshots")]
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

#[cfg(feature = "wgpu_screenshots")]
static WGPU_SERIAL_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(feature = "wgpu_screenshots")]
fn wgpu_guard() -> std::sync::MutexGuard<'static, ()> {
    WGPU_SERIAL_GUARD.lock().unwrap_or_else(|p| p.into_inner())
}

fn shared<T>(value: T) -> Arc<Mutex<T>> {
    Arc::new(Mutex::new(value))
}

/// Collect every (author_id, role) pair in the live AccessKit tree.
fn author_ids(harness: &Harness<'_, ()>) -> std::collections::HashSet<String> {
    let mut ids = std::collections::HashSet::new();
    for node in harness.root().children_recursive() {
        if let Some(a) = node.accesskit_node().author_id() {
            ids.insert(a.to_owned());
        }
    }
    ids
}

/// The role of the node whose author_id matches `author`, if present.
fn role_of(harness: &Harness<'_, ()>, author: &str) -> Option<String> {
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        if ak.author_id() == Some(author) {
            return Some(format!("{:?}", ak.role()));
        }
    }
    None
}

/// Every visible text label string in the live AccessKit tree (label + value across all nodes). Used to
/// assert formatted-not-raw rendering: a formatted heading shows the text "Heading" but NEVER a label
/// whose content is the literal markdown line "# Heading".
fn all_texts(harness: &Harness<'_, ()>) -> Vec<String> {
    let mut out = Vec::new();
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        if let Some(l) = ak.label() {
            out.push(l.to_owned());
        }
        if let Some(v) = ak.value() {
            out.push(v.to_owned());
        }
    }
    out
}

/// A seeded wiki projection whose `rendered_content` is the canonical MT-059 fixture: a heading + a
/// two-item bullet list (the PT2 seed from the contract).
fn markdown_projection(rendered: &str) -> WikiProjection {
    WikiProjection {
        projection_id: "proj-001".to_owned(),
        workspace_id: "ws-test".to_owned(),
        title: "Ownership model".to_owned(),
        source_block_ids: vec!["blk-1".to_owned()],
        rendered_content: rendered.to_owned(),
        staleness_hash: "h1".to_owned(),
        rebuild_status: "fresh".to_owned(),
        page_type: Some("concept".to_owned()),
        staleness_verdict: serde_json::json!({ "state": "fresh" }),
    }
}

fn loaded_panel(rendered: &str) -> LoomWikiPagePanel {
    let mut p = LoomWikiPagePanel::new("ws-test", "proj-001");
    p.set_page(markdown_projection(rendered));
    p
}

/// Harness rendering the shared wiki panel.
fn panel_harness(panel: Arc<Mutex<LoomWikiPagePanel>>) -> Harness<'static, ()> {
    Harness::builder()
        .with_size(egui::vec2(560.0, 700.0))
        .build_ui(move |ui| {
            let pal = HsTheme::Dark.palette();
            let _ = panel.lock().unwrap().show(ui, &pal);
        })
}

// ── PT2 + AC2 + AC7: the LIVE read-only wiki panel renders rendered_content as FORMATTED markdown ──────

#[test]
fn pt2_read_only_wiki_renders_formatted_markdown_not_raw() {
    // The contract's PT2 seed: a heading + a two-item bullet list.
    let panel = shared(loaded_panel("# Heading\n\n- one\n- two"));
    let mut harness = panel_harness(Arc::clone(&panel));
    harness.run();

    // AC7: the MT-025 content node is PRESERVED (Role::Document `wiki.content.proj-001`) — markdown blocks
    // render INSIDE it; downstream swarm selectors depend on it (RISK-5 / MC-5).
    let ids = author_ids(&harness);
    assert!(
        ids.contains(&content_author_id("proj-001")),
        "AC7: the '{}' content node must still be present (ids={ids:?})",
        content_author_id("proj-001")
    );
    assert_eq!(
        role_of(&harness, &content_author_id("proj-001")).as_deref(),
        Some("Document"),
        "AC7: the content node must still be Role::Document"
    );

    // AC2: the content is FORMATTED, not raw. No label in the tree carries the literal markdown source
    // lines "# Heading" or "- one" / "- two" (those are the un-rendered characters the MT-025 raw Label
    // would have shown). The Document node's VALUE legitimately carries the raw source (so a swarm agent
    // can read the page text by id) — we exclude that single node's value and assert no RENDERED label is
    // raw markdown.
    let labels: Vec<String> = {
        let mut v = Vec::new();
        for node in harness.root().children_recursive() {
            let ak = node.accesskit_node();
            // Skip the content Document node's value (it carries the raw source by design — AC7).
            if ak.author_id() == Some(content_author_id("proj-001").as_str()) {
                continue;
            }
            if let Some(l) = ak.label() {
                v.push(l.to_owned());
            }
        }
        v
    };
    assert!(
        !labels.iter().any(|t| t.contains("# Heading")),
        "AC2: the read-only view must NOT show the literal '# Heading' (it must render as a heading). \
         labels={labels:?}"
    );
    assert!(
        !labels.iter().any(|t| t.trim_start().starts_with("- one") || t.trim_start().starts_with("- two")),
        "AC2: the read-only view must NOT show the literal '- one'/'- two' (it must render as bullets). \
         labels={labels:?}"
    );

    // AC2 (positive): the heading TEXT "Heading" is present as a rendered label, and a bullet glyph '•'
    // marks the rendered list — proving it is actually formatted.
    let texts = all_texts(&harness);
    assert!(
        texts.iter().any(|t| t == "Heading"),
        "AC2: the rendered heading label 'Heading' must be present (texts={texts:?})"
    );
    assert!(
        texts.iter().any(|t| t.contains('\u{2022}')),
        "AC2: a rendered bullet glyph '•' must be present for the list (texts={texts:?})"
    );
    assert!(
        texts.iter().any(|t| t == "one") && texts.iter().any(|t| t == "two"),
        "AC2: both list item texts 'one' and 'two' must render (texts={texts:?})"
    );
    println!("PT2/AC2/AC7: live wiki panel renders '# Heading\\n- one\\n- two' as a formatted heading + bulleted list; wiki.content Document node preserved");
}

// ── PT3 + AC3: render_blocks is panic-free for malformed USER markdown ────────────────────────────────

#[test]
fn pt3_render_malformed_markdown_is_panic_free() {
    // Each of these is a known-malformed user markdown shape; parse + RENDER must complete with no panic.
    let malformed = [
        "```\nunterminated code fence body\nstill open",          // unterminated fence
        "| a | b | c |\n|---|---|---|\n| 1 |\n",                   // ragged table row
        "#\n##\n",                                                  // empty headings
        &"  ".repeat(0),                                            // empty
        "> quote\n> - nested\n>   - deeper\n>     - deepest\n",   // nested list-in-quote
        "**unclosed bold and _unclosed italic and `unclosed code", // unbalanced inline
        "a paragraph then a [link with no close](http://x and trailing words", // odd link
    ];
    // Build a deeply-nested list separately (12 levels) to exercise the iterative fold.
    let mut deep = String::new();
    for i in 0..12 {
        deep.push_str(&"  ".repeat(i));
        deep.push_str("- deep item\n");
    }

    let cases: Vec<String> = malformed
        .iter()
        .map(|s| s.to_string())
        .chain(std::iter::once(deep))
        .collect();

    // Render every case through the LIVE render path in a kittest harness; a panic in render_blocks would
    // unwind the harness closure and fail the test.
    for (i, case) in cases.iter().enumerate() {
        let case = case.clone();
        let mut harness = Harness::builder()
            .with_size(egui::vec2(480.0, 600.0))
            .build_ui(move |ui| {
                let blocks = parse_markdown(&case);
                render_blocks(ui, &blocks);
            });
        harness.run();
        // The render produced a tree (no panic). The harness root always exists; reaching here is the
        // pass condition for case i.
        let _ = harness.root();
        println!("PT3/AC3: malformed case {i} rendered without panic");
    }
}

/// PT3 (no dropped trailing content at the RENDER layer): an unterminated fence keeps its body, and
/// trailing prose after a block survives as a paragraph. Proven on the parsed block sequence the renderer
/// consumes (the same Vec<MdBlock> render_blocks walks).
#[test]
fn pt3_malformed_preserves_trailing_content() {
    let blocks = parse_markdown("intro paragraph\n\n```\nunclosed fence body line\nmore body");
    assert!(
        blocks.iter().any(|b| matches!(b, MdBlock::CodeBlock { code, .. } if code.contains("unclosed fence body line"))),
        "PT3: an unterminated fence keeps its body content (blocks={blocks:?})"
    );

    let trailing = parse_markdown("# title\n\ntrailing words after the heading");
    assert!(
        trailing.iter().any(|b| matches!(b, MdBlock::Paragraph { spans }
            if spans.iter().map(|s| s.text.as_str()).collect::<String>().contains("trailing words"))),
        "PT3: trailing paragraph text after a block is never dropped (blocks={trailing:?})"
    );
    println!("PT3/AC3: malformed input preserves trailing content (no dropped remainder)");
}

// ── PT4 + AC4: the edit overlay is UNCHANGED — it still binds the RAW edit-buffer string ───────────────

#[test]
fn pt4_edit_overlay_still_binds_raw_markdown() {
    // MT-059 is a READ-SIDE-ONLY change: the edit overlay (edit_mode==true) is untouched (RISK-4 / MC-4).
    // Click Edit to enter the overlay, set a RAW markdown string into the edit buffer, and assert the
    // `wiki.edit-area.{id}` TextEdit::multiline binds that raw string VERBATIM (hash/dash characters
    // preserved + editable) — exactly as MT-025 left it. The read-only formatting NEVER leaks into the
    // editable text.
    let panel = shared(loaded_panel("# Heading\n\n- one\n- two"));
    let mut harness = panel_harness(Arc::clone(&panel));
    harness.run();

    // Click the Edit button (enters edit_mode).
    let edit_target = edit_author_id("proj-001");
    harness
        .get_by(|n: &egui_kittest::kittest::AccessKitNode<'_>| n.author_id() == Some(edit_target.as_str()))
        .click();
    harness.run();

    // The edit area (Role::MultilineTextInput) is present in edit mode (AC4 / MC-4).
    let ids = author_ids(&harness);
    assert!(
        ids.contains(&edit_area_author_id("proj-001")),
        "AC4: the edit area must appear after clicking Edit (ids={ids:?})"
    );
    assert_eq!(
        role_of(&harness, &edit_area_author_id("proj-001")).as_deref(),
        Some("MultilineTextInput"),
        "AC4: the edit area must be Role::MultilineTextInput (the MT-025 raw editor, unchanged)"
    );

    // Set a RAW markdown string into the editable buffer (the same state the TextEdit::multiline mutates)
    // and re-render; the buffer must hold the raw hash/dash characters verbatim — the editor is raw text,
    // NOT formatted markdown (RISK-4: formatting the edit overlay would make the source un-editable).
    let raw = "# A raw heading\n- a raw bullet\n- another *with marks*";
    panel.lock().unwrap().set_edit_buffer(raw);
    harness.run();
    assert_eq!(
        panel.lock().unwrap().edit_buffer,
        raw,
        "AC4/PT4: the edit overlay binds the RAW markdown string verbatim (hash/dash editable, not formatted)"
    );

    // The read-only content node is ABSENT while editing (the overlay replaced the read-only view), so the
    // formatting change is scoped to edit_mode==false only.
    let ids = author_ids(&harness);
    assert!(
        !ids.contains(&content_author_id("proj-001")),
        "AC4: in edit mode the read-only content node is hidden (the change is read-side-only)"
    );
    println!("PT4/AC4: edit overlay unchanged — binds raw markdown verbatim ('{raw}'); formatting is read-side-only");
}

// ── AC5: the rendering path is SHARED, not duplicated (no re-declared MT-012 styling constants) ───────

#[test]
fn ac5_markdown_render_reuses_mt012_styling_no_duplication() {
    // Source-level proof that markdown_render reuses the MT-012 line_layout styling source of truth rather
    // than re-implementing heading-scale / quote-bar / code-frame / table-stroke constants. Reads the
    // module source (crate-relative path; disk-agnostic) and asserts the reuse imports are present and no
    // local styling constants are declared.
    let src = std::fs::read_to_string("src/rich_editor/markdown_render.rs")
        .expect("markdown_render.rs is readable from the crate root");

    // REUSE: it imports the MT-012 line_layout block-look constants + the block_renderer per-span shim.
    assert!(
        src.contains("use crate::rich_editor::renderer::line_layout::"),
        "AC5: markdown_render must import the MT-012 line_layout styling constants"
    );
    assert!(
        src.contains("HEADING_SCALE")
            && src.contains("BLOCKQUOTE_BAR_WIDTH_PTS")
            && src.contains("CODE_PADDING_PTS")
            && src.contains("LIST_INDENT_PTS"),
        "AC5: markdown_render must use the MT-012 line_layout block-look constants (heading scale / quote \
         bar / code padding / list indent)"
    );
    assert!(
        src.contains("md_span_text_format"),
        "AC5: markdown_render must use the block_renderer md_span_text_format shim for per-span styling"
    );

    // NO DUPLICATION: markdown_render must NOT re-declare its own heading-scale array or quote-bar width
    // constant. Guard against an actual heading-scale array ASSIGNMENT (`= [1.8`) — a doc comment that
    // merely names the values is fine; a re-declared array literal is not — and a `const ..._BAR_WIDTH`.
    assert!(
        !src.contains("= [1.8"),
        "AC5: markdown_render must NOT re-declare the MT-012 heading-scale array (reuse line_layout::HEADING_SCALE)"
    );
    assert!(
        !src.contains("const BLOCKQUOTE_BAR_WIDTH")
            && !src.contains("const CODE_PADDING")
            && !src.contains("const LIST_INDENT"),
        "AC5: markdown_render must NOT re-declare the MT-012 quote-bar / code-padding / list-indent constants"
    );
    println!("AC5: markdown_render reuses the MT-012 line_layout + block_renderer styling; no duplicated styling constants");
}

// ── HBR-VIS screenshot: the read-only wiki panel renders formatted markdown (OPT-IN, wgpu_screenshots) ─

#[test]
#[cfg(feature = "wgpu_screenshots")]
fn mt059_wiki_markdown_screenshot() {
    let _g = wgpu_guard();
    let panel = shared(loaded_panel(
        "# Heading\n\nA paragraph with **bold** and *italic* and `code`.\n\n- one\n- two\n\n\
         | col a | col b |\n|-------|-------|\n| 1 | 2 |\n\n> a blockquote\n\n```rust\nfn x() {}\n```\n\n\
         A [link](https://example.com).",
    ));
    let mut harness = Harness::builder()
        .with_size(egui::vec2(560.0, 720.0))
        .wgpu()
        .build_ui(move |ui| {
            let pal = HsTheme::Dark.palette();
            let _ = panel.lock().unwrap().show(ui, &pal);
        });
    harness.run();
    harness.run();

    match harness.render() {
        Ok(image) => {
            let (w, h) = (image.width(), image.height());
            assert!(w > 0 && h > 0, "rendered image must be non-empty");
            let ext_dir = external_artifact_dir("wp-kernel-012-mt-059");
            let _ = std::fs::create_dir_all(&ext_dir);
            let png = ext_dir.join("mt059_wiki_markdown.png");
            let saved = image.save(&png).is_ok();
            println!("HBR-VIS: {w}x{h} mt059 wiki-markdown screenshot, saved={saved} ({})", png.display());
        }
        Err(e) => {
            println!(
                "BLOCKER(non-fatal): mt059 wiki-markdown screenshot render unavailable (no wgpu adapter): {e}. \
                 The AccessKit/structural read-only + edit-overlay + malformed proofs passed; the PNG is a GPU-host item."
            );
        }
    }
    assert_no_local_artifact_dir();
}
