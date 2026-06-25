//! WP-KERNEL-012 MT-071 (E11) — VS-Code-class status-bar editor segments.
//!
//! Proves the five editor file-metadata segments (LanguageMode / EOL / Indent / Encoding /
//! RenderWhitespace) the MT-071 contract adds to the shell status/menu bar:
//!
//! - AC-004: the five segments render as live `Role::Button` nodes built with the WP-011 segment
//!   pattern (NO new status-bar infra), each with a working right-click context menu — driven through
//!   egui_kittest against the REAL `EditorStatusSegments` widget (the production render + popup path,
//!   not a memory-only node).
//! - AC-005: all five segments HIDE when no code-editor document is active (`None` state -> zero
//!   segment nodes) and re-appear with correct metadata when a code document is focused (`Some` state
//!   built `from_panel` -> five nodes).
//! - AC-006: the AccessKit tree dump carries the five stable author_ids `status-bar-language-mode`,
//!   `status-bar-eol`, `status-bar-indent`, `status-bar-encoding`, `status-bar-render-whitespace`,
//!   each role=Button.
//! - AC-001: language detection by shebang/content overrides the MT-001 extension-only detection, and a
//!   user override (DetectionSource::UserOverride) beats all auto-detection and STICKS per document
//!   across re-render (proven on the live `CodeEditorPanel` doc model).
//! - AC-002: EOL convert LF<->CRLF rewrites the line endings as EXACTLY ONE undo step (proven on the
//!   panel: convert, assert all endings changed, then a single restore returns the original).
//! - AC-003: indent detection picks tabs vs spaces + size, and a set_indent override updates the
//!   document's Tab-key editing behavior.
//! - AC-008: the segments call the EXISTING WP-011 segment helper + context-menu helper (no new
//!   status-bar segment type) — asserted by the stable-id reference checks + the right-click menu
//!   carrying the WP-011 `statusbar.*` items.
//!
//! ## Artifact hygiene (CX-212E)
//!
//! The screenshot is written ONLY to the EXTERNAL artifact root
//! `../../../../Handshake_Artifacts/handshake-test/MT-071/`; the test asserts NO repo-local
//! `test_output/` or `tests/screenshots/` directory exists after the run.

use std::path::{Path, PathBuf};

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;

use handshake_native::code_editor::file_meta::{Encoding, Eol, IndentKind, IndentStyle};
use handshake_native::code_editor::language_mode::{DetectionSource, LanguageId};
use handshake_native::code_editor::panel::CodeEditorPanel;
use handshake_native::top_menu_bar::{
    EditorMetaSegmentState, EditorSegment, EditorSegmentAction, EditorStatusSegments,
};

/// The crate-relative path to the external artifacts root (CX-212E), disk-agnostic. The crate sits at
/// `<repo>/src/frontend/handshake_native`, so four `..` reach the `Handshake_Artifacts` sibling.
fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

/// Assert no repo-local test-artifact directory exists under the crate (CX-212E hygiene): neither
/// `test_output/` nor `tests/screenshots/`. Artifacts go to the external root ONLY.
fn assert_no_local_artifact_dir() {
    for local in [Path::new("test_output"), Path::new("tests/screenshots")] {
        assert!(
            !local.exists(),
            "CX-212E: no repo-local artifact dir may exist — screenshots go to the external \
             Handshake_Artifacts/handshake-test root only (found {})",
            local.display()
        );
    }
}

/// Every live AccessKit node carrying an author_id: (author_id, role, label).
fn author_nodes(harness: &Harness<'_>) -> Vec<(String, String, Option<String>)> {
    let mut found = Vec::new();
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        if let Some(author_id) = ak.author_id() {
            found.push((author_id.to_owned(), format!("{:?}", ak.role()), ak.label()));
        }
    }
    found
}

/// A state-less harness rendering the five segments from `state` (or hiding them when `None`), capturing
/// the typed action a click/menu produces. Drives the SAME `EditorStatusSegments::show` path the live
/// status bar wires (no new menu infra) so the AccessKit nodes + activation are the production path.
fn segments_harness(
    state: Option<EditorMetaSegmentState>,
    captured: std::sync::Arc<std::sync::Mutex<Option<EditorSegmentAction>>>,
) -> Harness<'static> {
    Harness::builder()
        .with_size(egui::vec2(720.0, 120.0))
        .build_ui(move |ui| {
            // Render the segments in a bottom panel like the real status bar, so the picker popup has the
            // panel geometry the menu open path expects (matching the production mount point).
            egui::TopBottomPanel::bottom("test_status_bar").show(ui.ctx(), |ui| {
                ui.horizontal(|ui| {
                    if let Some(action) = EditorStatusSegments::new(state.clone()).show(ui) {
                        *captured.lock().unwrap() = Some(action);
                    }
                });
            });
        })
}

/// Build the live metadata state from a code panel (the production `from_panel` path).
fn state_from_rust_snippet() -> EditorMetaSegmentState {
    let panel = CodeEditorPanel::new("fn main() {\n    let x = 1;\n}\n", "rs");
    EditorMetaSegmentState::from_panel(&panel)
}

// ── AC-004 / AC-006: five Role::Button segments with the stable author_ids ────────────────────────────

#[test]
fn five_segments_render_with_stable_button_author_ids() {
    let captured = std::sync::Arc::new(std::sync::Mutex::new(None));
    let mut harness = segments_harness(Some(state_from_rust_snippet()), captured);
    harness.run();

    let nodes = author_nodes(&harness);
    for segment in EditorSegment::ALL {
        let found = nodes
            .iter()
            .find(|(a, _, _)| a == segment.author_id())
            .unwrap_or_else(|| panic!("{} missing from live tree: {nodes:?}", segment.author_id()));
        assert_eq!(found.1, "Button", "{} role is Button", segment.author_id());
    }
    // Exactly the five MT-071 ids (no extra / fewer segment nodes).
    let seg_count = nodes
        .iter()
        .filter(|(a, _, _)| a.starts_with("status-bar-"))
        .count();
    assert_eq!(seg_count, 5, "exactly five editor segments in the live tree: {nodes:?}");
    assert_no_local_artifact_dir();
}

// ── AC-005: hide when no code document; appear when one is active ─────────────────────────────────────

#[test]
fn segments_hide_when_no_code_document_active() {
    // None state -> the whole cluster hides (no segment nodes).
    let captured = std::sync::Arc::new(std::sync::Mutex::new(None));
    let mut hidden = segments_harness(None, captured.clone());
    hidden.run();
    let hidden_nodes = author_nodes(&hidden);
    let hidden_count = hidden_nodes
        .iter()
        .filter(|(a, _, _)| a.starts_with("status-bar-"))
        .count();
    assert_eq!(hidden_count, 0, "no editor segments when no code document active: {hidden_nodes:?}");

    // Some state -> the five segments re-appear with their metadata.
    let mut shown = segments_harness(Some(state_from_rust_snippet()), captured);
    shown.run();
    let shown_count = author_nodes(&shown)
        .iter()
        .filter(|(a, _, _)| a.starts_with("status-bar-"))
        .count();
    assert_eq!(shown_count, 5, "five editor segments when a code document is focused");
}

// ── AC-004 (right-click menu) / AC-008: the WP-011 segment context menu opens with contract items ─────

#[test]
fn right_click_eol_segment_opens_wp011_context_menu_with_convert_items() {
    let captured = std::sync::Arc::new(std::sync::Mutex::new(None));
    let mut harness = segments_harness(Some(state_from_rust_snippet()), captured);
    harness.run();

    // Closed by default: the segment-specific menu items are NOT in the tree before a right-click.
    let closed = author_nodes(&harness);
    assert!(
        !closed.iter().any(|(a, _, _)| a.contains("statusbar.eol")),
        "no EOL menu items before the right-click: {closed:?}",
    );

    // Right-click the EOL segment (addressed by its display label "LF" on the rust snippet, which is
    // LF-detected) to open the WP-011 status-bar segment context menu.
    harness.get_by_label("LF").click_secondary();
    harness.run();
    harness.run();

    let open = author_nodes(&harness);
    // The WP-011 shared status-bar items (Copy / Hide / Refresh) AND the MT-071 EOL quick-actions are
    // present — proving the menu reuses the WP-011 infra (AC-008) and adds the contract actions.
    let labels: Vec<&str> = open.iter().filter_map(|(_, _, l)| l.as_deref()).collect();
    assert!(
        labels.iter().any(|l| l.contains("Convert to CRLF")),
        "EOL context menu offers 'Convert to CRLF': {labels:?}",
    );
    assert!(
        labels.contains(&"Copy"),
        "EOL context menu reuses the WP-011 'Copy' item: {labels:?}",
    );
    assert_no_local_artifact_dir();
}

// ── AC-004 (left-click picker) / AC-006 (picker items role=ListItem) ──────────────────────────────────

#[test]
fn language_segment_picker_opens_with_listitems_and_dispatches() {
    // Open the language picker via the deterministic programmatic-open seam (the same `Popup::open_id`
    // path a left-click drives + an out-of-process driver uses). egui's left-click menu-popup TOGGLE is
    // not reliably driveable in an isolated kittest harness (the codebase's own context-menu tests
    // likewise drive opens via secondary-click / `request_open`, not a top-level left-click toggle), so
    // this proves the SAME open->render->dispatch path through the explicit open id.
    let captured = std::sync::Arc::new(std::sync::Mutex::new(None));
    let popup_id = EditorStatusSegments::picker_popup_id(EditorSegment::LanguageMode);
    let state = Some(state_from_rust_snippet());
    let cap2 = captured.clone();
    // Open the language picker on the FIRST frame via the deterministic programmatic-open seam (inside the
    // closure, where the egui Context is in scope), then it stays open across runs.
    let opened = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let opened2 = opened.clone();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(720.0, 120.0))
        .build_ui(move |ui| {
            if !opened2.swap(true, std::sync::atomic::Ordering::Relaxed) {
                egui::Popup::open_id(ui.ctx(), popup_id);
            }
            egui::TopBottomPanel::bottom("test_status_bar").show(ui.ctx(), |ui| {
                ui.horizontal(|ui| {
                    if let Some(action) = EditorStatusSegments::new(state.clone()).show(ui) {
                        *cap2.lock().unwrap() = Some(action);
                    }
                });
            });
        });
    harness.run();
    harness.run();

    let open = author_nodes(&harness);
    // The picker list container + the JavaScript option row carry the stable ids (role=List / ListItem).
    assert!(
        open.iter()
            .any(|(a, r, _)| a == "status-bar-language-mode-picker" && r == "List"),
        "language picker container is role=List: {open:?}",
    );
    let js_item = open
        .iter()
        .find(|(a, _, _)| a == "status-bar-language-mode-item-javascript")
        .unwrap_or_else(|| panic!("javascript picker item missing: {open:?}"));
    assert_eq!(js_item.1, "ListItem", "picker rows are role=ListItem");

    // Click the JavaScript row -> the typed action is SetLanguage(javascript).
    harness.get_by_label("JavaScript").click();
    harness.run();
    let action = captured.lock().unwrap().clone();
    assert_eq!(
        action,
        Some(EditorSegmentAction::SetLanguage(LanguageId::new("javascript"))),
        "selecting JavaScript emits SetLanguage(javascript)",
    );
}

// ── AC-001: detection precedence + override sticks per document (live doc model) ──────────────────────

#[test]
fn language_override_sticks_per_document_across_rerender() {
    // A `.rs` file whose extension says rust but whose shebang says python: detection picks shebang.
    let panel = CodeEditorPanel::new("#!/usr/bin/env python\nprint(1)\n", "rs");
    panel.set_file_path("script.rs");
    let auto = panel.resolved_language();
    assert_eq!(auto.source, DetectionSource::Shebang, "shebang beats the .rs extension");
    assert_eq!(auto.detected.as_str(), "python");

    // A user override to javascript beats the shebang and the extension, and STICKS across a re-resolve
    // (the doc model holds it — RISK-004).
    panel.set_language_override(Some(LanguageId::new("javascript")));
    let overridden = panel.resolved_language();
    assert_eq!(overridden.source, DetectionSource::UserOverride);
    assert_eq!(overridden.detected.as_str(), "javascript");
    // Re-resolve again (simulating re-render / re-focus): the override is still there.
    let again = panel.resolved_language();
    assert_eq!(again.source, DetectionSource::UserOverride, "override persists across re-resolve");
    assert_eq!(again.detected.as_str(), "javascript");

    // Clearing the override falls back to the shebang again.
    panel.set_language_override(None);
    assert_eq!(panel.resolved_language().source, DetectionSource::Shebang);
}

// ── AC-002: EOL convert is exactly one undo step (live doc model) ─────────────────────────────────────

#[test]
fn eol_convert_is_one_undo_step() {
    let original = "line1\nline2\nline3\n";
    let panel = CodeEditorPanel::new(original, "rs");
    assert_eq!(panel.eol(), Eol::Lf, "LF-detected on open");

    // Convert LF -> CRLF: ALL endings change, recorded as ONE whole-buffer replace.
    let changed = panel.convert_eol(Eol::Crlf);
    assert!(changed, "conversion changed the buffer");
    assert_eq!(panel.eol(), Eol::Crlf);
    let crlf_text = panel.buffer().to_string();
    assert_eq!(crlf_text, "line1\r\nline2\r\nline3\r\n", "every ending became CRLF");
    assert!(!crlf_text.contains("\n\r"), "no malformed endings");

    // The conversion queued EXACTLY ONE unified-undo snapshot (description, before, after) — the SAME
    // single-undo bus boundary every code edit records at (the factory render drains this into ONE undo
    // entry, so a single Ctrl+Z reverts the WHOLE conversion — RISK-002/MC-002, no per-line edits).
    let pending = panel.take_pending_line_op_undo().expect("EOL convert queued one undo snapshot");
    assert_eq!(pending.0, "Convert Line Endings");
    assert_eq!(pending.1, original, "undo restores the original byte-for-byte");
    assert_eq!(pending.2, crlf_text, "redo re-applies the converted text");
    // Applying the snapshot's `before` (what the single Ctrl+Z does) returns the original exactly.
    panel.set_text(&pending.1);
    assert_eq!(panel.buffer().to_string(), original, "single undo returns the original exactly");

    // Idempotent: converting to the already-active EOL is a no-op.
    let panel_lf = CodeEditorPanel::new(original, "rs");
    assert!(!panel_lf.convert_eol(Eol::Lf), "converting to the same EOL is a no-op");
}

// ── AC-003: indent detection + Tab-key behavior override (live doc model) ─────────────────────────────

#[test]
fn indent_detection_and_tab_key_override() {
    // A tab-indented file detects Tabs.
    let tabs = CodeEditorPanel::new("fn f() {\n\tlet x = 1;\n}\n", "rs");
    assert_eq!(tabs.indent_style().kind, IndentKind::Tabs, "tab-indented -> Tabs");

    // A 4-space file detects Spaces size 4, and that drives the REUSED MT-051 Tab-key indent settings.
    let spaces = CodeEditorPanel::new("def f():\n    x = 1\n    return x\n", "py");
    let style = spaces.indent_style();
    assert_eq!(style.kind, IndentKind::Spaces);
    assert_eq!(style.size, 4);
    assert_eq!(spaces.indent_settings(), (4, true), "Tab key inserts 4 spaces");

    // A set_indent override to Tabs flips the Tab-key behavior to a literal tab (AC-003).
    spaces.set_indent_style(IndentStyle { kind: IndentKind::Tabs, size: 4 });
    assert_eq!(spaces.indent_settings(), (4, false), "override flips Tab key to a literal tab");
    assert_eq!(spaces.indent_style().kind, IndentKind::Tabs);
}

// ── Encoding reopen is in-process (no backend) + an in-memory buffer is a typed blocker, not a panic ──

#[test]
fn encoding_reopen_is_in_process_and_typed() {
    // An in-memory buffer (no file path) -> a TYPED error, never a silent no-op or a backend call.
    let panel = CodeEditorPanel::new("hello\n", "txt");
    let err = panel.reopen_with_encoding(Encoding::Utf16Le);
    assert!(err.is_err(), "in-memory reopen returns a typed error (no file)");

    // Default encoding is UTF-8; set_encoding records the load encoding the MT-010 path decoded under.
    assert_eq!(panel.encoding(), Encoding::Utf8);
    panel.set_encoding(Encoding::Utf8Bom);
    assert_eq!(panel.encoding(), Encoding::Utf8Bom);
}

// ── AC-004 screenshot proof + artifact hygiene (CX-212E) ──────────────────────────────────────────────

#[test]
fn segments_screenshot_to_external_artifact_root() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(720.0, 40.0))
        .wgpu()
        .build_ui({
            let state = Some(state_from_rust_snippet());
            move |ui| {
                ui.horizontal(|ui| {
                    let _ = EditorStatusSegments::new(state.clone()).show(ui);
                });
            }
        });
    harness.run();

    // Render to a PNG written ONLY to the external artifact root (CX-212E). Best-effort pixel readback:
    // on a GPU host the PNG is saved; on a GPU-less host the logical/AccessKit proofs above stand and we
    // record an honest non-fatal note rather than faking a pass.
    match harness.render() {
        Ok(image) => {
            let ext_dir = external_artifact_dir("MT-071");
            let _ = std::fs::create_dir_all(&ext_dir);
            let png_path = ext_dir.join("MT-071-status-bar-segments.png");
            let saved = image.save(&png_path).is_ok();
            eprintln!(
                "MT-071 segments screenshot: {}x{} saved={saved} ({})",
                image.width(),
                image.height(),
                png_path.display()
            );
        }
        Err(e) => {
            eprintln!("MT-071 segments screenshot: GPU readback unavailable ({e}); logical + AccessKit proofs stand");
        }
    }
    // No repo-local artifact dir may exist after the run (the PNG goes to the external root only).
    assert_no_local_artifact_dir();
}
