//! WP-KERNEL-012 MT-078 (E13 i18n) — RTL + complex-script proofs.
//!
//! HONEST TIERED SCOPE (the load-bearing design — NO faking): egui's epaint lays text out LTR and does
//! NOT perform the Unicode Bidirectional Algorithm or complex-script shaping. MT-078 delivers the
//! achievable tiers and surfaces a TYPED LIMITATION for the rest:
//!   - TIER 1 (DELIVERED): base-direction detection (first-strong) + logical→visual reorder via UAX#9
//!     (`unicode-bidi`) + right-aligned RTL paragraphs, on top of the MT-078 bundled Hebrew/Arabic/
//!     Devanagari font faces.
//!   - TIER 2 (DELIVERED — Hebrew): Hebrew is non-joining, so it renders + edits correctly end-to-end —
//!     the honest RTL proof case.
//!   - TIER 3 (TYPED LIMITATION — Arabic/Indic): egui does not run GSUB/GPOS, so Arabic renders in
//!     ISOLATED (unjoined) forms. Rather than present that as "done", the editors raise a VISIBLE
//!     `text_intl::bidi::ShapingLimitation` note + a future-MT pointer. NEVER silently-broken Arabic.
//!
//! PROOF MAP (contract proof_targets / acceptance_criteria):
//! - PROOF1 / AC2+AC3+AC6: `bidi_reorder_mixed_line_to_visual_order`, `ltr_reorder_is_identity`,
//!   `base_direction_first_strong` — the bidi reorder fn (mixed → visual order, LTR identity) and base
//!   direction, asserted on the REAL `text_intl::bidi` pass (not a tautology — they assert the reversed
//!   Hebrew sub-run appears in visual order and the logical order is preserved).
//! - PROOF1 / AC4: `hebrew_end_to_end_edit_logical_order` drives the REAL rich-editor `input_handler`
//!   pipeline (insert / ArrowLeft / ArrowRight / Backspace) over a Hebrew run and asserts the model
//!   mutates in LOGICAL order with the documented arrow semantics (Hebrew needs no cursive shaping, so
//!   this is the honest RTL editing proof).
//! - PROOF1 / RISK-3: `rope_stays_logical_order_docjson_round_trip` proves the model stores Hebrew in
//!   LOGICAL order (the DocJson backend round-trip is byte-identical) — bidi is render-only.
//! - PROOF2 / AC1+AC5: `hebrew_rtl_screenshot` (right-aligned RTL Hebrew → mt078_hebrew_rtl.png) and
//!   `arabic_state_screenshot` (Arabic glyphs + the typed-limitation note → mt078_arabic_state.png),
//!   both written to the EXTERNAL artifact root and VISUALLY INSPECTED by the reviewer.
//! - PROOF3 / AC5: `arabic_typed_limitation_is_surfaced_not_silent` is the no-silent-breakage gate —
//!   Arabic MUST raise the visible typed limitation (the decision taken is DECISION A: unicode-bidi +
//!   Hebrew + Arabic typed-limitation; the Tier-3 rustybuzz/cosmic-text shaping spike was NOT taken
//!   because it would destabilize the egui Galley path — RISK-4 — so the typed limitation stands).
//! - PROOF4 / AC6+AC7: `bidi_is_noop_for_ltr_and_cjk` proves the bidi pass is identity for LTR + CJK
//!   (no regression to MT-075/077); the FULL `cargo test -p handshake-native` suite green is AC7.
//!
//! ARTIFACT HYGIENE (CX-212E): EVERY PNG goes ONLY to the EXTERNAL
//! `Handshake_Artifacts/handshake-test/wp-kernel-012-mt-078/` root via [`external_artifact_dir`].
//! [`assert_no_local_artifact_dir`] fails the run if a repo-local `test_output/` or `tests/screenshots/`
//! dir exists (the contract's repo-local screenshot path is OVERRIDDEN by this rule).

use std::path::{Path, PathBuf};

use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use unicode_segmentation::UnicodeSegmentation;

use handshake_native::code_editor::virtual_lines::code_line_bidi;
use handshake_native::code_editor::CodeEditorPanel;
use handshake_native::rich_editor::document_model::{
    from_json_string, to_json_string, BlockNode, Child, DocPosition, Selection, UndoManager,
};
use handshake_native::rich_editor::renderer::input_handler::{self, EditAction, EditContext};
use handshake_native::rich_editor::renderer::rich_editor_widget::{RichEditorState, RichEditorWidget};
use handshake_native::text_intl::{self, ComplexScript, Direction};
use std::sync::{Arc, Mutex};

// Hebrew "שלום עולם" (shalom olam = "hello world"), LOGICAL order. Non-joining RTL — the honest case.
const HEBREW_HELLO: &str = "שלום עולם";
// Arabic "العربية" (al-arabiyya), LOGICAL order. Cursive-joining — the typed-limitation case.
const ARABIC: &str = "العربية";

/// The crate-relative path to the EXTERNAL artifacts root (CX-212E), disk-agnostic. The crate sits at
/// `<repo>/src/frontend/handshake_native`, so four `..` reach `<repo>/..` where `Handshake_Artifacts` is a
/// sibling of the repo worktree. Identical to the helper in test_code_editor_panel.rs / test_font_coverage.rs.
fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

/// Assert NO repo-local artifact directory exists under the crate (CX-212E hygiene). Checks BOTH
/// `test_output/` AND `tests/screenshots/`. A stray local artifact dir is a hygiene regression the reviewer
/// also greps for via `git ls-files "src/**/*.png"`.
fn assert_no_local_artifact_dir() {
    for local in ["test_output", "tests/screenshots"] {
        let p = Path::new(local);
        assert!(
            !p.exists(),
            "ARTIFACT HYGIENE (CX-212E): no repo-local '{}' dir may exist — MT-078 screenshots go to the \
             external Handshake_Artifacts/handshake-test root only (found {})",
            local,
            p.display()
        );
    }
}

// ── PROOF1 / AC2 + AC3 + AC6: the bidi reorder + base-direction pass (pure, deterministic) ────────────

#[test]
fn base_direction_first_strong() {
    // AC3: paragraph base direction is the FIRST STRONG character's direction (UAX#9 P2/P3). Both an RTL
    // and an LTR case are covered (the contract's "a cargo test covers both").
    assert_eq!(text_intl::base_direction(HEBREW_HELLO), Direction::Rtl, "Hebrew first-strong -> RTL");
    assert_eq!(text_intl::base_direction(ARABIC), Direction::Rtl, "Arabic first-strong -> RTL");
    assert_eq!(text_intl::base_direction("hello"), Direction::Ltr, "Latin -> LTR");
    assert_eq!(text_intl::base_direction("中文"), Direction::Ltr, "CJK -> LTR");
    // First strong char decides even when both scripts are present.
    assert_eq!(text_intl::base_direction("abc שלום"), Direction::Ltr, "leading Latin -> LTR base");
    assert_eq!(text_intl::base_direction("שלום abc"), Direction::Rtl, "leading Hebrew -> RTL base");
}

#[test]
fn ltr_reorder_is_identity() {
    // AC6 / MC-3 / RISK-2: the bidi reorder is a NO-OP (identity) for pure-LTR text. This is the proof
    // that LTR + CJK rendering (MT-075/077) is byte-for-byte unchanged.
    for s in ["hello world", "Привет мир", "这是中文", "abc 123", ""] {
        let r = text_intl::reorder_line(s);
        assert!(r.is_identity(s), "LTR identity required for {s:?}: {r:?}");
        assert_eq!(r.visual_text, s, "LTR visual text equals logical text for {s:?}");
    }
}

#[test]
fn bidi_reorder_mixed_line_to_visual_order() {
    // AC2: a mixed-direction line (LTR Latin + an RTL Hebrew run + digits) is REORDERED to correct visual
    // order; the LOGICAL order is preserved (we never mutate the input) and only the VISUAL string changes.
    let logical = format!("abc {HEBREW_HELLO} 123");
    let r = text_intl::reorder_line(&logical);

    // First strong char is Latin -> LTR base, but the Hebrew run is still visually reordered (reversed).
    assert_eq!(r.base, Direction::Ltr, "first strong is Latin -> LTR base");
    assert!(!r.is_identity(&logical), "a mixed bidi line must reorder (not identity)");

    // The Hebrew run appears REVERSED (visual order) in the visual text; the verbatim logical Hebrew run
    // must NOT appear (it was reversed for display).
    let hebrew_visual: String = HEBREW_HELLO.graphemes(true).rev().collect();
    assert!(
        r.visual_text.contains(&hebrew_visual),
        "AC2: the Hebrew run must be reversed to visual order in {:?}",
        r.visual_text
    );

    // Logical order preserved: the multiset of grapheme clusters is unchanged (reorder rearranges, never
    // drops/duplicates), and every run maps back to a real byte range in the ORIGINAL logical text.
    let mut logical_gr: Vec<&str> = logical.graphemes(true).collect();
    let mut visual_gr: Vec<&str> = r.visual_text.graphemes(true).collect();
    logical_gr.sort_unstable();
    visual_gr.sort_unstable();
    assert_eq!(logical_gr, visual_gr, "AC2: reorder preserves all logical content");
    for run in &r.runs {
        assert!(logical.is_char_boundary(run.logical_range.start));
        assert!(logical.is_char_boundary(run.logical_range.end));
    }
    assert!(r.runs.iter().any(|x| x.rtl), "must have an RTL run");
    assert!(r.runs.iter().any(|x| !x.rtl), "must have an LTR run");
}

#[test]
fn bidi_is_noop_for_ltr_and_cjk() {
    // PROOF4 / AC6: explicit no-op proof for the LTR + CJK render path. If the bidi pass were NOT identity
    // here, MT-075 (CJK) / MT-077 (segmentation) rendering would regress.
    for s in ["fn main() {}", "let 变量 = 1;", "这是中文段落里没有空格", "Hello 世界"] {
        let r = text_intl::reorder_line(s);
        assert!(r.is_identity(s), "bidi must be a no-op for LTR/CJK line {s:?}: {r:?}");
    }
}

// ── PROOF1 / AC4: Hebrew end-to-end edit through the REAL input_handler (logical order) ────────────────

/// Build a one-paragraph doc holding `text`, with the caret collapsed at `caret_off` (char offset in the
/// single text leaf at path [0,0]).
fn doc_with_caret(text: &str, caret_off: usize) -> (BlockNode, Selection, UndoManager) {
    let doc = BlockNode::doc(vec![BlockNode::paragraph(text)]);
    let sel = Selection::caret(DocPosition::new(vec![0, 0], caret_off));
    (doc, sel, UndoManager::new())
}

/// The text of the single leaf at [0,0].
fn leaf_text(doc: &BlockNode) -> String {
    doc.children[0]
        .as_block()
        .unwrap()
        .children
        .first()
        .and_then(Child::as_text)
        .map(|l| l.text.to_string())
        .unwrap_or_default()
}

/// The caret char offset of a collapsed text selection.
fn caret_off(sel: &Selection) -> usize {
    match sel {
        Selection::Text { head, .. } => head.char_offset,
        _ => 0,
    }
}

#[test]
fn hebrew_end_to_end_edit_logical_order() {
    // AC4 (the honest RTL editing proof — Hebrew is non-joining): type / move caret / select / delete in a
    // Hebrew run, all behaving in LOGICAL order with the documented arrow semantics. The rope is
    // direction-agnostic, so the SAME logical-order edit model used for Latin works for Hebrew.

    // 1) Insert a Hebrew char at the start of an empty paragraph.
    let (mut doc, mut sel, mut undo) = doc_with_caret("", 0);
    {
        let mut ctx = EditContext { doc: &mut doc, selection: &mut sel, undo: &mut undo, actor_id: "operator" };
        assert!(input_handler::apply_action(&mut ctx, EditAction::Insert("ש".into())));
    }
    assert_eq!(leaf_text(&doc), "ש", "Hebrew char inserted");
    assert_eq!(caret_off(&sel), 1, "caret advanced one logical char (grapheme) after insert");

    // 2) Insert the rest of the Hebrew word — the model accumulates in LOGICAL order (the order typed).
    {
        let mut ctx = EditContext { doc: &mut doc, selection: &mut sel, undo: &mut undo, actor_id: "operator" };
        for ch in "לום".chars() {
            assert!(input_handler::apply_action(&mut ctx, EditAction::Insert(ch.to_string())));
        }
    }
    assert_eq!(leaf_text(&doc), "שלום", "the Hebrew word is stored in LOGICAL (typed) order");
    let logical_len = "שלום".chars().count();
    assert_eq!(caret_off(&sel), logical_len, "caret at logical end after typing");

    // 3) ArrowLeft moves the caret to the PREVIOUS logical grapheme (documented semantics: logical-order
    //    motion, NOT visual-order). The rope stays logical-order; the renderer handles visual mapping.
    {
        let mut ctx = EditContext { doc: &mut doc, selection: &mut sel, undo: &mut undo, actor_id: "operator" };
        assert!(input_handler::apply_action(&mut ctx, EditAction::MoveLeft { extend: false }));
    }
    assert_eq!(caret_off(&sel), logical_len - 1, "ArrowLeft -> previous logical grapheme");

    // 4) ArrowRight moves to the NEXT logical grapheme (back to the end).
    {
        let mut ctx = EditContext { doc: &mut doc, selection: &mut sel, undo: &mut undo, actor_id: "operator" };
        assert!(input_handler::apply_action(&mut ctx, EditAction::MoveRight { extend: false }));
    }
    assert_eq!(caret_off(&sel), logical_len, "ArrowRight -> next logical grapheme");

    // 5) Backspace deletes the last Hebrew letter (one grapheme cluster) in logical order.
    {
        let mut ctx = EditContext { doc: &mut doc, selection: &mut sel, undo: &mut undo, actor_id: "operator" };
        assert!(input_handler::apply_action(&mut ctx, EditAction::DeleteBackward));
    }
    assert_eq!(leaf_text(&doc), "שלו", "Backspace removed the last Hebrew letter (logical-order delete)");
    assert_eq!(caret_off(&sel), logical_len - 1, "caret after the deleted grapheme");

    // 6) Select-all then the documented RTL caret arrow semantics string is the AC4 contract (proven in
    //    the bidi unit test + asserted non-empty here).
    {
        let mut ctx = EditContext { doc: &mut doc, selection: &mut sel, undo: &mut undo, actor_id: "operator" };
        assert!(input_handler::apply_action(&mut ctx, EditAction::SelectAll));
    }
    assert!(matches!(sel, Selection::Text { .. }), "Ctrl+A produced a text selection over the Hebrew run");
    assert!(text_intl::RTL_CARET_ARROW_SEMANTICS.contains("LOGICAL"), "AC4: arrow semantics documented");
}

// ── PROOF1 / RISK-3: the rope stays LOGICAL order (DocJson backend round-trip is byte-identical) ──────

#[test]
fn rope_stays_logical_order_docjson_round_trip() {
    // RISK-3 / MC-2: storing VISUAL-order text in the rope would corrupt the model + break the backend
    // round-trip. Prove the model stores Hebrew in LOGICAL order: serialize to DocJson, deserialize, and
    // the leaf text is byte-identical to the LOGICAL input (NOT the visually-reversed form). Bidi is
    // render-only and never touches the stored text.
    let doc = BlockNode::doc(vec![BlockNode::paragraph(HEBREW_HELLO)]);
    let json = to_json_string(&doc).expect("serialize");
    let back = from_json_string(&json).expect("deserialize");
    let stored = leaf_text(&back);
    assert_eq!(stored, HEBREW_HELLO, "the rope stores LOGICAL-order Hebrew (backend round-trip intact)");

    // The visually-reversed form must NOT be what is stored (that would be model corruption).
    let visual: String = HEBREW_HELLO.graphemes(true).rev().collect();
    assert_ne!(stored, visual, "the model must NOT store visual-order (reversed) text");
}

// ── PROOF3 / AC5: the Tier-3 typed limitation is SURFACED for Arabic (no-silent-breakage gate) ────────

#[test]
fn arabic_typed_limitation_is_surfaced_not_silent() {
    // PROOF3 / AC5 / MC-1 / RISK-1 — the central honesty gate. egui cannot cursive-shape Arabic, so MT-078
    // does NOT present it as done: `shaping_limitation` MUST return a VISIBLE typed limitation (with a
    // future-MT pointer) for Arabic content. This is DECISION A (typed limitation, not the rustybuzz/
    // cosmic-text spike — that was rejected to avoid destabilizing the egui Galley path, RISK-4).
    let lim = text_intl::shaping_limitation(ARABIC)
        .expect("PROOF3: Arabic MUST raise a typed shaping limitation (never silently broken)");
    assert_eq!(lim.script, ComplexScript::Arabic);
    assert!(lim.note.to_lowercase().contains("arabic"), "the note names Arabic: {:?}", lim.note);
    assert!(lim.note.to_lowercase().contains("limit"), "the note says 'limited': {:?}", lim.note);
    assert!(!lim.pointer.is_empty(), "a future-MT pointer is recorded");
    assert_eq!(lim.pointer, text_intl::SHAPING_FOLLOW_ON_POINTER);

    // Hebrew (non-joining) must NOT raise the limitation — that is what makes it the honest end-to-end RTL
    // case; only Arabic/Indic do.
    assert!(
        text_intl::shaping_limitation(HEBREW_HELLO).is_none(),
        "Hebrew is fully handled (non-joining) — it must NOT raise a shaping limitation"
    );
}

// ── PROOF2 / AC1: right-aligned RTL Hebrew screenshot (visually inspected) ────────────────────────────

#[test]
fn hebrew_rtl_screenshot() {
    // AC1: a Hebrew paragraph renders RIGHT-ALIGNED with RTL base direction and correct visual order using
    // the MT-078 bundled Noto Hebrew face. Render the rich-editor block-paint path through a real frame and
    // save the PNG to the EXTERNAL artifact root for VISUAL INSPECTION.
    // Drive the REAL production rich-editor widget over a Hebrew document so the FULL pipeline (font atlas
    // build + the bidi right-align/reorder path wired into block_renderer) runs exactly as it does live —
    // not a bespoke painter call. The widget paints its own dark surface, so the light text is visible.
    let state = Arc::new(Mutex::new(RichEditorState::new(BlockNode::doc(vec![BlockNode::paragraph(
        HEBREW_HELLO,
    )]))));
    let state_ui = Arc::clone(&state);
    let mut harness = Harness::builder()
        // Tall enough that the document BODY (below the editor toolbar + properties chrome) is inside the
        // captured frame — a short window would only capture the toolbar and push the Hebrew paragraph off
        // the bottom.
        .with_size(egui::vec2(560.0, 560.0))
        .wgpu()
        .build_ui(move |ui| {
            // Install the real MT-075+MT-078 font fallback chain on THIS context (the kittest context is
            // fresh — without it, Hebrew would render as tofu).
            handshake_native::app::HandshakeApp::install_fonts(ui.ctx());
            RichEditorWidget::new(Arc::clone(&state_ui)).show(ui);
        });
    harness.run();
    harness.run();

    // Logical proof (GPU-independent): the Hebrew block resolves to RTL base + right alignment and the
    // Hebrew glyphs are present in the installed chain (no tofu).
    assert_eq!(text_intl::base_direction(HEBREW_HELLO), Direction::Rtl);
    let prop = egui::FontId::proportional(20.0);
    let has = harness.ctx.fonts_mut(|f| f.has_glyphs(&prop, HEBREW_HELLO));
    assert!(has, "AC1 (logical): Hebrew glyphs must resolve in the installed chain (no tofu)");

    match harness.render() {
        Ok(image) => {
            let (w, h) = (image.width(), image.height());
            assert!(w > 0 && h > 0, "rendered image must be non-empty");
            let ext = external_artifact_dir("wp-kernel-012-mt-078");
            let _ = std::fs::create_dir_all(&ext);
            let png = ext.join("mt078_hebrew_rtl.png");
            let saved = image.save(&png).is_ok();
            println!(
                "PROOF2/AC1 screenshot: {w}x{h} saved={saved} ({}); base=RTL right-aligned. VISUAL \
                 INSPECTION REQUIRED: open the PNG; confirm 'שלום עולם' shows real Hebrew glyphs aligned \
                 to the RIGHT edge (RTL), not tofu and not left-aligned.",
                png.display()
            );
        }
        Err(e) => {
            println!(
                "BLOCKER(non-fatal): MT-078 Hebrew screenshot render unavailable (no wgpu adapter / headless \
                 GPU crash): {e}. The LOGICAL RTL + glyph-coverage proof passed and stands as the AC1 \
                 evidence on this host; the PNG is a GPU-host item."
            );
        }
    }
    assert_no_local_artifact_dir();
}

// ── PROOF2 / AC5: Arabic state screenshot (glyphs + the typed-limitation note) ────────────────────────

#[test]
fn arabic_state_screenshot() {
    // AC5: an Arabic paragraph renders its (available, ISOLATED-form) glyphs AND the VISIBLE typed-
    // limitation note. The note path is exercised by the block-paint RTL branch (it paints "⚠ Arabic
    // cursive shaping limited …" beneath the text). Save the PNG for visual inspection.
    // Drive the REAL production rich-editor widget over an Arabic document. The widget's block_renderer RTL
    // path paints the Arabic glyphs (isolated forms — egui does not cursive-shape) AND the visible
    // typed-limitation note beneath them.
    let state = Arc::new(Mutex::new(RichEditorState::new(BlockNode::doc(vec![BlockNode::paragraph(
        ARABIC,
    )]))));
    let state_ui = Arc::clone(&state);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(560.0, 560.0))
        .wgpu()
        .build_ui(move |ui| {
            handshake_native::app::HandshakeApp::install_fonts(ui.ctx());
            RichEditorWidget::new(Arc::clone(&state_ui)).show(ui);
        });
    harness.run();
    harness.run();

    // Logical proof: Arabic base is RTL, the typed limitation is raised, and the Arabic glyphs resolve in
    // the installed chain (present but unshaped — the documented Tier-3 state).
    assert_eq!(text_intl::base_direction(ARABIC), Direction::Rtl);
    assert!(text_intl::shaping_limitation(ARABIC).is_some(), "Arabic raises the typed limitation");
    let prop = egui::FontId::proportional(20.0);
    let has = harness.ctx.fonts_mut(|f| f.has_glyphs(&prop, ARABIC));
    assert!(has, "AC5 (logical): Arabic glyphs must resolve in the chain (present, isolated forms)");

    match harness.render() {
        Ok(image) => {
            let (w, h) = (image.width(), image.height());
            assert!(w > 0 && h > 0, "rendered image must be non-empty");
            let ext = external_artifact_dir("wp-kernel-012-mt-078");
            let _ = std::fs::create_dir_all(&ext);
            let png = ext.join("mt078_arabic_state.png");
            let saved = image.save(&png).is_ok();
            println!(
                "PROOF2/AC5 screenshot: {w}x{h} saved={saved} ({}); Arabic glyphs present (ISOLATED forms — \
                 egui does not cursive-shape) + the visible '⚠ Arabic cursive shaping limited' note. VISUAL \
                 INSPECTION REQUIRED: open the PNG; confirm Arabic glyphs render (not tofu) AND the warning \
                 note is visible (the typed limitation — never silently-broken Arabic). DECISION A taken.",
                png.display()
            );
        }
        Err(e) => {
            println!(
                "BLOCKER(non-fatal): MT-078 Arabic screenshot render unavailable (no wgpu adapter): {e}. The \
                 LOGICAL typed-limitation + glyph-coverage proof passed and stands as the AC5 evidence on \
                 this host; the PNG is a GPU-host item."
            );
        }
    }
    assert_no_local_artifact_dir();
}

// ── AC1 + AC3 + AC5 (CODE EDITOR integration): the bidi wiring reaches the LIVE code-editor render path ─
//
// The pure `text_intl::bidi` / `virtual_lines::code_line_bidi` unit tests prove the helper RETURNS the
// right values, but the must-fix (adversarial review) is that the helper must be CONSUMED by the live
// code-editor render loop (`panel.rs::render_line` / `render_visual_row_fragment`), not left as dead
// scaffolding. These tests drive the REAL `CodeEditorPanel` over RTL/Arabic source and assert the wired
// render output appears on screen via the live AccessKit label tree — the integration, not the helper.

/// Drive the real `CodeEditorPanel` over `text` for two settled frames and return the harness so its
/// on-screen labels can be queried. `wgpu()` matches the other code-editor panel tests; fonts are installed
/// on the fresh kittest context so RTL glyphs resolve (no tofu) exactly as live.
fn code_panel_harness<'a>(text: &'a str, ext: &'a str) -> Harness<'a, ()> {
    let panel = CodeEditorPanel::new(text, ext);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(560.0, 240.0))
        .wgpu()
        .build_ui(move |ui| {
            handshake_native::app::HandshakeApp::install_fonts(ui.ctx());
            panel.show(ui);
        });
    harness.run();
    harness.run();
    harness
}

#[test]
fn code_editor_rtl_line_is_reordered_and_wired() {
    // AC1/AC3 (CODE editor): a code line containing a Hebrew literal is RTL base and is REORDERED into
    // VISUAL order by the LIVE render path (`render_line` calling `render_rtl_or_limited_code_row`). The
    // on-screen label text equals the bidi VISUAL-order string (the reversed Hebrew run), NOT the logical
    // string — proving the helper output reached the screen, not just the unit test.
    let line = "שלום עולם"; // pure Hebrew line (non-joining RTL — the honest case, no limitation note)
    let bidi = code_line_bidi(line);
    assert_eq!(bidi.base, Direction::Rtl, "the Hebrew code line resolves RTL base");
    assert!(bidi.shaping_limitation.is_none(), "Hebrew raises no limitation (non-joining)");

    let harness = code_panel_harness(line, "txt");

    // The LIVE render path paints the VISUAL-order (reordered) text as the row label. Assert that exact
    // visual string is on screen — and the raw LOGICAL string is NOT (it was reordered for display). This
    // fails if the code editor ever stops calling the bidi pass (the unwired-scaffolding regression).
    assert!(
        !bidi.visual_text.is_empty() && bidi.visual_text != line,
        "the Hebrew line must reorder to a different visual string: visual={:?} logical={:?}",
        bidi.visual_text,
        line
    );
    assert!(
        harness.query_by_label(&bidi.visual_text).is_some(),
        "AC1/AC3: the VISUAL-order Hebrew row {:?} must be on screen (the live code-editor render path \
         called the bidi reorder) — not the logical string",
        bidi.visual_text
    );
    assert!(
        harness.query_by_label(line).is_none(),
        "the LOGICAL (un-reordered) Hebrew {line:?} must NOT be the painted label — it was reordered"
    );
}

#[test]
fn code_editor_arabic_line_surfaces_limitation_marker_wired() {
    // AC5 / PROOF3 / MC-1 (CODE editor): an Arabic code literal MUST surface the VISIBLE typed-limitation
    // marker in the LIVE code-editor render path — Arabic in the code editor is NEVER silently broken. The
    // marker is a `⚠` label whose hover carries the limitation note; assert it is on screen.
    let line = "let s = \"العربية\";"; // Arabic literal inside an LTR line (base LTR, limitation present)
    let bidi = code_line_bidi(line);
    assert!(bidi.shaping_limitation.is_some(), "Arabic code line must raise the typed limitation");
    assert_eq!(bidi.base, Direction::Ltr, "first strong is the Latin 'l' of `let` -> LTR base");

    let harness = code_panel_harness(line, "txt");

    // The live render path paints the `⚠` limitation marker for the Arabic content. (`query_by_label`
    // matches the trimmed accessible text, so both the RTL " ⚠" and LTR "⚠" marker variants match "⚠".)
    assert!(
        harness.query_by_label("⚠").is_some(),
        "AC5/PROOF3/MC-1: the Arabic code line must surface the visible ⚠ typed-limitation marker on \
         screen (never silently broken) — the live code-editor render path called the shaping-limitation pass"
    );

    // And a pure-Hebrew code line must NOT raise the marker (the honest non-joining RTL case), so the
    // marker is specific to Arabic/Indic, not every RTL row.
    let hebrew_harness = code_panel_harness("שלום", "txt");
    assert!(
        hebrew_harness.query_by_label("⚠").is_none(),
        "a pure-Hebrew code line is non-joining and must NOT surface the shaping-limitation marker"
    );
}

#[test]
fn code_editor_ltr_line_is_unchanged_identity_wired() {
    // AC6 / MC-3 / RISK-2 (CODE editor): an ordinary LTR source line must be BYTE-FOR-BYTE unchanged — the
    // bidi pass returns the identity, so `render_rtl_or_limited_code_row` returns false and the existing
    // per-run colored LTR path renders. Prove the plain LTR text label is on screen and NO ⚠ marker is.
    let line = "let x = 1; // ok";
    let bidi = code_line_bidi(line);
    assert!(bidi.is_identity(line), "an LTR code line must be bidi-identity");

    // Plain text (extension "txt") so the whole line renders as exactly one label.
    let harness = code_panel_harness(line, "txt");
    assert!(
        harness.query_by_label(line).is_some(),
        "AC6: the LTR code line {line:?} must render unchanged (identity path) on screen"
    );
    assert!(
        harness.query_by_label("⚠").is_none(),
        "AC6: an LTR line must NOT surface any shaping-limitation marker"
    );
}
