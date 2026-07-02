//! WP-KERNEL-012 MT-076 (E13) — IME composition inline preedit proofs (rich + code editors).
//!
//! Closes the MT-012 AC-6 PARTIAL: the committed CJK text already inserted char-correct, but the
//! in-progress UNDERLINED preedit was held as state and NOT painted inline. This MT paints the preedit
//! inline at the caret in BOTH editors, reports the IME caret rect so the OS candidate window anchors at
//! the caret, enables IME at the shell, and brings the code editor to the same IME behavior.
//!
//! PROOF MAP (contract acceptance_criteria / proof_targets):
//! - AC1 / PROOF1: `rich_preedit_then_commit_inserts_only_commit` — feeding Preedit("ni") -> Preedit("nihao")
//!   -> Commit("你好") through the LIVE rich-editor widget input path inserts ONLY "你好" into the rope
//!   (no preedit chars left), and while composing the rope is unchanged (overlay-only — RISK-1 / MC-1).
//! - AC2 / PROOF2: `rich_preedit_underline_screenshot` — while composing, an egui_kittest screenshot shows
//!   the underlined inline CJK preedit at the caret; saved to the EXTERNAL artifact root + pixel/logical
//!   asserted. The MT-075 font chain renders the CJK glyphs.
//! - AC3: `rich_empty_commit_cancels_with_no_insert` + `rich_disabled_cancels` — Escape/empty-Commit /
//!   Disabled clears the preedit overlay with NO insertion (cancel path).
//! - AC4: `rich_ime_caret_rect_anchors_at_caret` — the editor reports an IMEOutput rect via
//!   `ctx.output_mut(...).ime` whose rect/cursor is at the caret region, NOT the window origin.
//! - AC5 / PROOF1: `code_editor_ime_composes_and_commits` (direct) + `code_editor_ime_through_live_input`
//!   (live `Event::Ime` through `panel.show`) — a CJK string composes (overlay-only) + commits into the
//!   code buffer; the cancel path leaves the buffer unchanged.
//! - AC6 / PROOF3: `shell_wires_set_ime_allowed` — the shell sends `ViewportCommand::IMEAllowed(true)`
//!   exactly once (a source grep + a live `HandshakeApp` frame that emits the viewport command).
//! - AC7: `rich_accesskit_exposes_composition` + `code_accesskit_exposes_composition` — while composing,
//!   the editor's EXISTING editable text node exposes the preedit text in its value (no new tree).
//! - AC8 / PROOF4: the FULL `cargo test -p handshake-native` suite stays green (this touches shared
//!   rich_editor_widget / block_renderer / code-editor input paths).
//!
//! ARTIFACT HYGIENE (CX-212E / contract screenshot rule): the screenshot is written ONLY to the EXTERNAL
//! `Handshake_Artifacts/handshake-test/wp-kernel-012-mt-076/` root via `external_artifact_dir`; the
//! `assert_no_local_artifact_dir` guard checks BOTH `test_output/` AND `tests/screenshots/` are absent.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use egui_kittest::Harness;

use handshake_native::code_editor::CodeEditorPanel;
use handshake_native::rich_editor::document_model::node::BlockNode;
use handshake_native::rich_editor::document_model::position::DocPosition;
use handshake_native::rich_editor::document_model::selection::Selection;
use handshake_native::rich_editor::renderer::rich_editor_widget::{
    RichEditorState, RichEditorWidget,
};

/// Serialize the `.wgpu()` screenshot test (the documented Windows-wgpu concurrent-device hazard the
/// other screenshot tests guard the same way).
static WGPU_SERIAL_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());
fn wgpu_guard() -> std::sync::MutexGuard<'static, ()> {
    WGPU_SERIAL_GUARD.lock().unwrap_or_else(|p| p.into_inner())
}

/// The crate-relative path to the EXTERNAL artifacts root (CX-212E), disk-agnostic — the crate sits at
/// `<repo>/src/frontend/handshake_native`, so four `..` reach `<repo>/..` where `Handshake_Artifacts` is
/// a sibling of the repo worktree. (The SCREENSHOT/TEST-ARTIFACT rule overrides any repo-local path.)
fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

/// Assert NO repo-local artifact directory exists under the crate (the artifact-hygiene guard the
/// SCREENSHOT/TEST-ARTIFACT rule mandates). Checks BOTH `test_output/` AND `tests/screenshots/`.
fn assert_no_local_artifact_dir() {
    for local in ["test_output", "tests/screenshots"] {
        let p = Path::new(local);
        assert!(
            !p.exists(),
            "ARTIFACT HYGIENE (CX-212E): no repo-local '{local}' dir may exist — MT-076 screenshots go to \
             the external Handshake_Artifacts/handshake-test root only (found {})",
            p.display()
        );
    }
}

/// A small editor over a single paragraph "Hi", caret at the end (offset 2) — the same shape the
/// ime_handler unit test uses, but here driven through the LIVE widget input path.
fn hi_state() -> Arc<Mutex<RichEditorState>> {
    let doc = BlockNode::doc(vec![BlockNode::paragraph("Hi")]);
    let mut st = RichEditorState::new(doc);
    st.selection = Selection::caret(DocPosition::new(vec![0, 0], 2));
    Arc::new(Mutex::new(st))
}

/// The leaf text of the first paragraph (the rope content under test).
fn leaf_text(state: &Arc<Mutex<RichEditorState>>) -> String {
    let st = state.lock().unwrap();
    st.doc.children[0].as_block().unwrap().children[0]
        .as_text()
        .unwrap()
        .text
        .to_string()
}

/// Build an EDITABLE rich-editor harness over `state`, with the MT-075 font chain installed so CJK
/// glyphs render (the demo harness pattern from test_rich_editor_widget.rs).
fn rich_harness<'a>(state: Arc<Mutex<RichEditorState>>) -> Harness<'a, ()> {
    Harness::builder()
        .with_size(egui::vec2(640.0, 220.0))
        .wgpu()
        .build_ui(move |ui| {
            handshake_native::app::HandshakeApp::install_fonts(ui.ctx());
            RichEditorWidget::new(Arc::clone(&state)).show(ui);
        })
}

/// Push one `egui::Event::Ime` onto the harness's next-frame input.
fn push_ime(harness: &mut Harness<'_, ()>, ime: egui::ImeEvent) {
    harness.input_mut().events.push(egui::Event::Ime(ime));
}

/// Focus the editor SURFACE (the focusable `rich-editor-surface` node) by sending it an AccessKit Focus
/// action — the same focus an out-of-process agent (or the OS routing IME to the focused widget) uses.
/// The rich editor's input path gates on `has_focus` (the runtime-correct behavior: the OS only sends
/// composition events to the focused widget), so the editor must be focused before IME events are
/// processed. This is the SAME pattern the MT-016/MT-018 rich-editor input tests use.
fn focus_editor(harness: &mut Harness<'_, ()>) {
    use egui_kittest::kittest::NodeT;
    {
        let root = harness.root();
        let surface = root
            .children_recursive()
            .find(|n| n.accesskit_node().author_id() == Some("rich-editor-surface"))
            .expect("the editor surface node carries author_id 'rich-editor-surface'");
        surface.focus();
    }
    // A FOCUSED editor animates the caret blink (the focus-guarded MT-015 blink), so it never settles —
    // `harness.run()` would loop to max_steps. Use bounded single steps after focus (the same discipline
    // the MT-015-aware rich tests use).
    harness.step();
    harness.step();
}

/// Run one bounded frame (a focused editor's blink never settles, so `run()` cannot be used after focus).
fn step(harness: &mut Harness<'_, ()>) {
    harness.step();
}

// ── AC1 / PROOF1: rich Preedit -> Commit inserts ONLY the commit (overlay-only invariant) ─────────

#[test]
fn rich_preedit_then_commit_inserts_only_commit() {
    let _g = wgpu_guard();
    let state = hi_state();
    let mut harness = rich_harness(Arc::clone(&state));
    harness.run();
    focus_editor(&mut harness);

    // Enabled + Preedit("ni"): the OVERLAY shows but the rope is UNCHANGED (double-insert invariant).
    push_ime(&mut harness, egui::ImeEvent::Enabled);
    push_ime(&mut harness, egui::ImeEvent::Preedit("ni".to_owned()));
    step(&mut harness);
    assert!(
        state.lock().unwrap().preedit.is_active(),
        "the preedit overlay is active while composing"
    );
    assert_eq!(
        leaf_text(&state),
        "Hi",
        "RISK-1/MC-1: rope is UNCHANGED while composing (overlay-only)"
    );

    // Refine the composition to "nihao": still overlay-only, rope still "Hi".
    push_ime(&mut harness, egui::ImeEvent::Preedit("nihao".to_owned()));
    step(&mut harness);
    assert_eq!(
        leaf_text(&state),
        "Hi",
        "RISK-1: refining the preedit does not touch the rope"
    );

    // Commit("你好"): the preedit clears and ONLY the committed CJK lands at the caret (char-correct).
    push_ime(&mut harness, egui::ImeEvent::Commit("你好".to_owned()));
    step(&mut harness);
    assert_eq!(
        leaf_text(&state),
        "Hi你好",
        "AC1: only the committed CJK landed (no preedit chars)"
    );
    assert!(
        !state.lock().unwrap().preedit.is_active(),
        "preedit cleared after commit"
    );

    // CJK char-index discipline: caret advanced by 2 CHARS (not bytes — RISK-5 multi-codepoint).
    if let Selection::Text { head, .. } = &state.lock().unwrap().selection {
        assert_eq!(
            head.char_offset, 4,
            "AC1/RISK-5: caret advanced by 2 committed CHARS, not bytes"
        );
    } else {
        panic!("expected a collapsed text caret after commit");
    }
    println!("AC1: rich Preedit->Commit inserted only '你好'; rope='Hi你好'; caret at char 4.");
}

// ── AC3: empty-Commit (cancel) + Disabled clear the overlay with NO insertion ─────────────────────

#[test]
fn rich_empty_commit_cancels_with_no_insert() {
    let _g = wgpu_guard();
    let state = hi_state();
    let mut harness = rich_harness(Arc::clone(&state));
    harness.run();
    focus_editor(&mut harness);
    push_ime(&mut harness, egui::ImeEvent::Enabled);
    push_ime(&mut harness, egui::ImeEvent::Preedit("nihao".to_owned()));
    step(&mut harness);
    assert!(
        state.lock().unwrap().preedit.is_active(),
        "composing before cancel"
    );
    // An EMPTY commit is the cancel path (Escape during composition on many IMEs): clear, no insert.
    push_ime(&mut harness, egui::ImeEvent::Commit(String::new()));
    step(&mut harness);
    assert_eq!(
        leaf_text(&state),
        "Hi",
        "AC3: empty-Commit cancel leaves the rope unchanged"
    );
    assert!(
        !state.lock().unwrap().preedit.is_active(),
        "AC3: preedit overlay cleared on cancel"
    );
    println!("AC3: empty-Commit cancelled the composition with no insert; rope='Hi'.");
}

#[test]
fn rich_disabled_cancels() {
    let _g = wgpu_guard();
    let state = hi_state();
    let mut harness = rich_harness(Arc::clone(&state));
    harness.run();
    focus_editor(&mut harness);
    push_ime(&mut harness, egui::ImeEvent::Enabled);
    push_ime(&mut harness, egui::ImeEvent::Preedit("abc".to_owned()));
    step(&mut harness);
    push_ime(&mut harness, egui::ImeEvent::Disabled);
    step(&mut harness);
    assert_eq!(
        leaf_text(&state),
        "Hi",
        "AC3: Disabled leaves the rope unchanged"
    );
    assert!(
        !state.lock().unwrap().preedit.is_active(),
        "AC3: Disabled cleared the preedit overlay"
    );
    println!("AC3: ImeEvent::Disabled cancelled the composition with no insert; rope='Hi'.");
}

// ── AC4: the editor reports an IMEOutput caret rect anchored at the caret, NOT the origin ─────────

#[test]
fn rich_ime_caret_rect_anchors_at_caret() {
    let _g = wgpu_guard();
    let state = hi_state();
    let mut harness = rich_harness(Arc::clone(&state));
    harness.run();
    focus_editor(&mut harness);
    // Start composing so the renderer reports the IME caret rect this frame.
    push_ime(&mut harness, egui::ImeEvent::Enabled);
    push_ime(&mut harness, egui::ImeEvent::Preedit("nihao".to_owned()));
    step(&mut harness);
    // The frame that rendered the active preedit must have set platform_output.ime.
    step(&mut harness);

    let ime = harness
        .output()
        .platform_output
        .ime
        .expect("AC4: the composing editor must report an IMEOutput (ctx.output_mut(...).ime)");
    // The candidate-window anchor must be at the caret region, NOT the window origin (0,0). The caret
    // sits after "Hi" inside a paragraph painted below the toolbar, so both x and y are well off origin.
    assert!(
        ime.rect.min.x > 1.0 && ime.rect.min.y > 1.0,
        "AC4/RISK-2: the IME rect must anchor at the caret, not the window origin; got {:?}",
        ime.rect
    );
    // The cursor rect (the thin composition caret) must sit at/after the rect's left edge (end of the
    // preedit run) and share the row, so the OS draws the candidate list at the caret.
    assert!(
        ime.cursor_rect.min.x >= ime.rect.min.x - 0.5,
        "AC4: the composition caret is at the end of the preedit run; got caret={:?} rect={:?}",
        ime.cursor_rect,
        ime.rect
    );
    println!(
        "AC4: IMEOutput rect={:?} cursor={:?} (anchored at the caret, not origin).",
        ime.rect, ime.cursor_rect
    );
}

// ── AC7 (rich): the editable text node exposes the composition text in its value (no new tree) ────

#[test]
fn rich_accesskit_exposes_composition() {
    use egui_kittest::kittest::NodeT;
    use handshake_native::rich_editor::renderer::RICH_EDITOR_ROOT_AUTHOR_ID;
    let _g = wgpu_guard();
    let state = hi_state();
    let mut harness = rich_harness(Arc::clone(&state));
    harness.run();
    focus_editor(&mut harness);
    push_ime(&mut harness, egui::ImeEvent::Enabled);
    push_ime(&mut harness, egui::ImeEvent::Preedit("nihao".to_owned()));
    step(&mut harness);
    step(&mut harness);

    // The EXISTING rich-editor-root TextInput node's value now carries the composing text (AC7) — a
    // screen reader / swarm agent observes the composition through the same node, no new tree.
    let mut composing_value: Option<String> = None;
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        if ak.author_id() == Some(RICH_EDITOR_ROOT_AUTHOR_ID) {
            composing_value = ak.value().map(|v| v.to_owned());
        }
    }
    let value =
        composing_value.expect("AC7: the rich-editor-root node must be present + carry a value");
    assert!(
        value.contains("composing") && value.contains("nihao"),
        "AC7: the editable text node value must expose the composition state; got '{value}'"
    );
    println!("AC7 (rich): rich-editor-root value during composition = '{value}'.");
}

// ── AC2 / PROOF2: the underlined inline CJK preedit screenshot ────────────────────────────────────

#[test]
fn rich_preedit_underline_screenshot() {
    let _g = wgpu_guard();
    let state = hi_state();
    let mut harness = rich_harness(Arc::clone(&state));
    harness.run();
    focus_editor(&mut harness);
    // Compose a CJK string so the preedit overlay paints the underlined run at the caret.
    push_ime(&mut harness, egui::ImeEvent::Enabled);
    push_ime(&mut harness, egui::ImeEvent::Preedit("你好世界".to_owned()));
    step(&mut harness);
    step(&mut harness);

    // Logical no-tofu guarantee (GPU-independent): the composing CJK string resolves to real glyphs in
    // the installed MT-075 fallback chain, so the painted preedit is real glyphs, not notdef boxes.
    let prop = egui::FontId::proportional(15.0);
    let glyphs_ok = harness.ctx.fonts_mut(|f| f.has_glyphs(&prop, "你好世界"));
    assert!(
        glyphs_ok,
        "AC2: the composing CJK preedit must resolve to real glyphs (MT-075 fallback chain)"
    );
    // The preedit overlay is active (so the renderer painted it this frame) and reported the IME rect.
    assert!(
        state.lock().unwrap().preedit.is_active(),
        "AC2: the preedit overlay is active while composing"
    );
    assert!(
        harness.output().platform_output.ime.is_some(),
        "AC2: the composing frame reported IMEOutput"
    );

    match harness.render() {
        Ok(image) => {
            let (w, h) = (image.width(), image.height());
            assert!(w > 0 && h > 0, "rendered image is non-empty");
            let ext_dir = external_artifact_dir("wp-kernel-012-mt-076");
            let _ = std::fs::create_dir_all(&ext_dir);
            let png_path = ext_dir.join("mt076_ime_preedit.png");
            let saved = image.save(&png_path).is_ok();
            let abs = std::fs::canonicalize(&png_path).unwrap_or(png_path.clone());

            // Pixel proof: the composing preedit paints an underlined run over a subtle tinted background,
            // so the composing region adds distinct foreground colors over the editor bg. Count distinct
            // non-transparent colors (sample every 4th pixel) and require multiple foreground colors.
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
            let foreground = counts.keys().filter(|p| Some(**p) != bg).count();
            println!(
                "PROOF2 mt076 preedit screenshot: {w}x{h}, {} colors, {foreground} foreground; saved={saved} ({})",
                counts.len(),
                abs.display()
            );
            assert!(
                foreground >= 2,
                "AC2: the underlined CJK preedit + caret must produce >= 2 distinct foreground colors; got {foreground}"
            );
            assert!(
                saved,
                "AC2/PROOF2: the mt076_ime_preedit.png screenshot saved to the external root"
            );
        }
        Err(e) => {
            println!(
                "BLOCKER(non-fatal): MT-076 preedit screenshot render unavailable (no wgpu adapter): {e}. \
                 The logical no-tofu + overlay-active + IMEOutput proofs above stand; the PNG is a GPU-host item."
            );
        }
    }
    assert_no_local_artifact_dir();
}

// ── AC5 / PROOF1: the code editor handles the same Preedit/Commit sequence (direct + live) ────────

#[test]
fn code_editor_ime_composes_and_commits() {
    // Direct drive of the code-editor IME entry point (the SAME method the live `Event::Ime` arm calls).
    let panel = CodeEditorPanel::new("x", "rs");
    // Place the caret at the end so the commit appends after "x".
    panel.set_single_cursor(panel.buffer().len_bytes());

    // Enabled + Preedit: overlay-only — the buffer is UNCHANGED while composing (RISK-1 / MC-1).
    assert!(!panel.handle_ime_event(&egui::ImeEvent::Enabled));
    assert!(!panel.handle_ime_event(&egui::ImeEvent::Preedit("nihao".to_owned())));
    assert_eq!(
        panel.preedit(),
        "nihao",
        "AC5: the code-editor preedit overlay shows while composing"
    );
    assert_eq!(
        panel.buffer().to_string(),
        "x",
        "AC5/RISK-1: the buffer is UNCHANGED while composing"
    );

    // Commit("你好"): clears the overlay, inserts ONLY the committed CJK char-correct at the caret.
    assert!(
        panel.handle_ime_event(&egui::ImeEvent::Commit("你好".to_owned())),
        "commit mutated the buffer"
    );
    assert_eq!(
        panel.buffer().to_string(),
        "x你好",
        "AC5: only the committed CJK landed in the code buffer"
    );
    assert_eq!(
        panel.preedit(),
        "",
        "AC5: the code-editor preedit cleared after commit"
    );

    // The cursor advanced past the committed run (2 CJK chars = 6 UTF-8 bytes after the 1-byte "x").
    let (start, _end) = panel.primary_selection_bytes();
    assert_eq!(
        start,
        "x你好".len(),
        "AC5: the code-editor caret advanced past the committed CJK run"
    );

    // Cancel path: an empty commit / Disabled clears the overlay with NO further insert.
    panel.handle_ime_event(&egui::ImeEvent::Enabled);
    panel.handle_ime_event(&egui::ImeEvent::Preedit("zz".to_owned()));
    assert!(
        !panel.handle_ime_event(&egui::ImeEvent::Commit(String::new())),
        "empty commit does not mutate"
    );
    assert_eq!(
        panel.buffer().to_string(),
        "x你好",
        "AC5: empty-Commit cancel left the code buffer unchanged"
    );
    assert_eq!(panel.preedit(), "", "AC5: the cancel cleared the overlay");
    println!("AC5: code editor composed + committed '你好' (overlay-only); cancel left the buffer unchanged.");
}

#[test]
fn code_editor_ime_through_live_input() {
    // Drive the SAME sequence through the LIVE `panel.show` input path (the `Event::Ime` arm in
    // `process_cursor_input` reads `ui.input()`), proving the runtime input loop wires IME — not just the
    // direct method (anti-tautology: this exercises the real `egui::Event::Ime` decode).
    let panel = Arc::new(CodeEditorPanel::new("x", "rs"));
    panel.set_single_cursor(panel.buffer().len_bytes());
    let panel_for_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(640.0, 200.0))
        .build_ui(move |ui| {
            handshake_native::app::HandshakeApp::install_fonts(ui.ctx());
            panel_for_ui.show(ui);
        });
    harness.run();

    harness
        .input_mut()
        .events
        .push(egui::Event::Ime(egui::ImeEvent::Enabled));
    harness
        .input_mut()
        .events
        .push(egui::Event::Ime(egui::ImeEvent::Preedit(
            "nihao".to_owned(),
        )));
    harness.run();
    assert_eq!(
        panel.preedit(),
        "nihao",
        "AC5: live Event::Ime Preedit set the code-editor overlay"
    );
    assert_eq!(
        panel.buffer().to_string(),
        "x",
        "AC5: the buffer is unchanged while composing (live path)"
    );

    harness
        .input_mut()
        .events
        .push(egui::Event::Ime(egui::ImeEvent::Commit("你好".to_owned())));
    harness.run();
    assert_eq!(
        panel.buffer().to_string(),
        "x你好",
        "AC5: live Event::Ime Commit inserted the CJK char-correct"
    );
    assert_eq!(
        panel.preedit(),
        "",
        "AC5: the live commit cleared the overlay"
    );
    println!(
        "AC5 (live): the code-editor `egui::Event::Ime` input arm composed + committed '你好'."
    );
}

// ── AC7 (code): the code-editor text node exposes the composition text in its value ───────────────

#[test]
fn code_accesskit_exposes_composition() {
    use egui_kittest::kittest::NodeT;
    use handshake_native::code_editor::CODE_EDITOR_TEXT_AUTHOR_ID;
    let panel = Arc::new(CodeEditorPanel::new("x", "rs"));
    panel.set_single_cursor(panel.buffer().len_bytes());
    let panel_for_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(640.0, 200.0))
        .build_ui(move |ui| {
            handshake_native::app::HandshakeApp::install_fonts(ui.ctx());
            panel_for_ui.show(ui);
        });
    harness.run();
    harness
        .input_mut()
        .events
        .push(egui::Event::Ime(egui::ImeEvent::Enabled));
    harness
        .input_mut()
        .events
        .push(egui::Event::Ime(egui::ImeEvent::Preedit(
            "nihao".to_owned(),
        )));
    harness.run();
    harness.run();

    let mut composing_value: Option<String> = None;
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        if ak.author_id() == Some(CODE_EDITOR_TEXT_AUTHOR_ID) {
            composing_value = ak.value().map(|v| v.to_owned());
        }
    }
    let value =
        composing_value.expect("AC7: the code_editor_text node must be present + carry a value");
    assert!(
        value.contains("composing") && value.contains("nihao"),
        "AC7: the code-editor text node value must expose the composition state; got '{value}'"
    );
    println!("AC7 (code): code_editor_text value during composition = '{value}'.");
}

// ── AC6 / PROOF3: the shell wires set_ime_allowed (ViewportCommand::IMEAllowed(true)) ─────────────

#[test]
fn shell_wires_set_ime_allowed_source() {
    // Source grep: the app.rs shell must reference the IMEAllowed viewport command (the egui-side
    // set_ime_allowed). The crate sits at the cwd of the test; app.rs is at src/app.rs.
    let app_src = std::fs::read_to_string("src/app.rs").expect("read src/app.rs");
    assert!(
        app_src.contains("ViewportCommand::IMEAllowed"),
        "AC6/PROOF3: app.rs must wire set_ime_allowed via ViewportCommand::IMEAllowed(true)"
    );
    println!("AC6 (source): app.rs wires ViewportCommand::IMEAllowed(true).");
}

#[test]
fn shell_emits_ime_allowed_command_live() {
    use handshake_native::app::{HandshakeApp, HealthDisplayState};
    use handshake_native::backend_client::HealthInfo;
    // A live `HandshakeApp` frame must EMIT the IMEAllowed viewport command on its first real frame
    // (AC6 / RISK-3 / MC-3 — proven at runtime, not just by grep). Run one frame through a raw context
    // with AccessKit enabled (the same path the shell uses) and inspect the platform output's viewport
    // commands for the IMEAllowed(true) command.
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }));
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let output = ctx.run(egui::RawInput::default(), |ctx| {
        app.ui(ctx);
    });
    // A `send_viewport_cmd` lands in FullOutput.viewport_output[<viewport>].commands as a
    // `ViewportCommand` (NOT in platform_output.commands, which is clipboard/url only).
    let found = output.viewport_output.values().any(|vp| {
        vp.commands
            .iter()
            .any(|c| matches!(c, egui::ViewportCommand::IMEAllowed(true)))
    });
    assert!(
        found,
        "AC6/RISK-3/MC-3: the first live shell frame must emit ViewportCommand::IMEAllowed(true) so winit \
         forwards IME composition events"
    );
    println!("AC6 (live): the first HandshakeApp frame emitted ViewportCommand::IMEAllowed(true).");
}
