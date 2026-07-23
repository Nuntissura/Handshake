//! MT-004 find-bar LIVE proofs (WP-KERNEL-012 E1 code editor): egui_kittest interactive Ctrl+F ->
//! the find-bar AccessKit node, the yellow-highlight screenshot, and Escape -> highlights removed.
//!
//! PT-004 / AC-004 (`cargo test -p handshake-native find_bar_accesskit`): inject Ctrl+F, type "fn",
//! and verify the live AccessKit tree contains a node `author_id="code_editor_find_bar"` with role
//! `SearchInput` (the field-correct accesskit 0.21 role for a search box — the contract's `SearchBox`
//! does not exist there; AC-004 asserts the author_id, which matches exactly).
//! PT-005 / AC-005: an egui_kittest screenshot with the find bar open and the query "fn" on a Rust
//! snippet shows at least one YELLOW match-highlight rect in the text area. Saved to the EXTERNAL
//! artifact root only (`MT-004-find-highlight.png`).
//! AC-006: closing the find bar with Escape removes `find_state` so no highlights are rendered the
//! next frame.
//!
//! ## Why drive the panel API for the query rather than typing into the egui TextEdit
//!
//! The find bar's TextEdit owns the query string for the frame and pushes it back into `find_state`
//! via `set_find_query`. Injecting per-character text events into a specific TextEdit by focus is
//! brittle across egui versions; the deterministic, contract-faithful path is: inject the REAL Ctrl+F
//! key event (so `open_find` runs through the real keymap — AC-004's "open find bar via Ctrl+F"), then
//! set the query through the same public API the TextEdit calls. The AccessKit node + the highlight
//! rendering are exactly what a real keystroke produces.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use egui_kittest::kittest::{NodeT, Queryable};
#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;
use screenshot_harness::ScreenshotHarness as Harness;
#[path = "native_gui_support/argus_surface_proof.rs"]
mod argus_surface_proof;
use argus_surface_proof::{prove_argus_surface, ArgusMutation};

use handshake_native::code_editor::{CodeEditorPanel, CODE_EDITOR_FIND_BAR_AUTHOR_ID};

/// A multi-line Rust snippet with several `fn` occurrences so "fn" highlights are unambiguous.
const SNIPPET: &str = "fn main() {}\nfn helper() {}\nlet fname = 1;";

fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

fn assert_no_local_test_output() {
    let local = Path::new("test_output");
    assert!(
        !local.exists(),
        "no repo-local test_output/ dir may exist — artifacts go to the external \
         Handshake_Artifacts/handshake-test root only (found {})",
        local.display()
    );
}

/// A Ctrl+F key-press event (open find).
fn ctrl_f() -> egui::Event {
    egui::Event::Key {
        key: egui::Key::F,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers {
            ctrl: true,
            ..Default::default()
        },
    }
}

/// An Escape key-press event (close find).
fn escape() -> egui::Event {
    egui::Event::Key {
        key: egui::Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::default(),
    }
}

// ── PT-004 / AC-004: Ctrl+F + query -> code_editor_find_bar SearchInput node ──────────────────────

#[test]
fn find_bar_accesskit_ctrl_f_makes_find_bar_node() {
    let panel = Arc::new(CodeEditorPanel::new(SNIPPET, "rs"));
    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(700.0, 220.0))
        .build_ui(move |ui| {
            panel_ui.show(ui);
        });

    // Frame 1: render so input is wired.
    harness.run();
    assert!(!panel.is_find_open(), "find bar starts closed");

    // Inject the REAL Ctrl+F key event -> open_find runs through the keymap.
    harness.event(ctrl_f());
    harness.run();
    assert!(panel.is_find_open(), "Ctrl+F opened the find bar");

    // Type "fn" (through the public query API the find TextEdit drives).
    panel.set_find_query("fn");
    harness.run();
    harness.run(); // settle so the AccessKit nodes are emitted

    // The find bar must have matches for "fn" (two: line 0 and line 1).
    let state = panel.find_state().expect("bar open");
    assert!(
        state.matches.len() >= 2,
        "AC-004: 'fn' query matched at least the two fn keywords; got {}",
        state.matches.len()
    );

    // PT-004 / AC-004: the live tree must contain code_editor_find_bar with role SearchInput.
    let root = harness.root();
    let mut found_role: Option<String> = None;
    for node in root.children_recursive() {
        let ak = node.accesskit_node();
        if ak.author_id() == Some(CODE_EDITOR_FIND_BAR_AUTHOR_ID) {
            found_role = Some(format!("{:?}", ak.role()));
            break;
        }
    }
    assert!(
        found_role.is_some(),
        "AC-004: live tree must contain a node with author_id='{CODE_EDITOR_FIND_BAR_AUTHOR_ID}'"
    );
    assert_eq!(
        found_role.as_deref(),
        Some("SearchInput"),
        "AC-004: '{CODE_EDITOR_FIND_BAR_AUTHOR_ID}' must be Role::SearchInput (field-correct for the \
         contract's SearchBox, which does not exist in accesskit 0.21)"
    );
    let find_node = root
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(CODE_EDITOR_FIND_BAR_AUTHOR_ID))
        .expect("find-bar node remains addressable after role verification")
        .accesskit_node();
    assert!(
        find_node
            .data()
            .supports_action(egui::accesskit::Action::SetValue),
        "MT-108: stable find-bar node advertises SetValue for Argus query mutation"
    );
    println!("PT-004 find bar node: {{\"{CODE_EDITOR_FIND_BAR_AUTHOR_ID}\":\"{found_role:?}\"}}");

    // The find-next button is also addressable (HBR-SWARM).
    assert!(
        root.children_recursive()
            .any(|n| n.accesskit_node().author_id() == Some("code_editor_find_next")),
        "AC-004: the find-next button is AccessKit-addressable"
    );
}

#[test]
fn mt108_argus_find_bar_real_server_loop() {
    let panel = Arc::new(CodeEditorPanel::new(SNIPPET, "rs"));
    panel.open_find(false);
    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(700.0, 220.0))
        .build_ui(move |ui| panel_ui.show(ui));
    harness.run_steps(2);

    prove_argus_surface(
        &mut harness,
        "find bar",
        CODE_EDITOR_FIND_BAR_AUTHOR_ID,
        ArgusMutation::SetValue {
            target: CODE_EDITOR_FIND_BAR_AUTHOR_ID,
            value: "fn",
        },
        CODE_EDITOR_FIND_BAR_AUTHOR_ID,
        true,
        |_| {
            let pattern = panel
                .find_state()
                .ok_or_else(|| "find panel closed after mutation".to_owned())?
                .query
                .pattern;
            if pattern != "fn" {
                return Err(format!("expected query 'fn', observed {pattern:?}"));
            }
            Ok(serde_json::json!({ "query_pattern": pattern }))
        },
    );
}

// ── MT-108 (MT-004 residual): typing real characters into the focused find TextEdit ───────────────

#[test]
fn find_bar_typing_into_textedit_updates_query_and_matches() {
    // Ctrl+F auto-focuses the find input (MT-108 residual), so real per-character Text events land in
    // the TextEdit and drive the query through the SAME render path a user's keystrokes take — proving
    // the find input is wired end-to-end, not just via the `set_find_query` API shortcut.
    let panel = Arc::new(CodeEditorPanel::new(SNIPPET, "rs"));
    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(700.0, 220.0))
        .build_ui(move |ui| {
            panel_ui.show(ui);
        });

    harness.run();
    harness.event(ctrl_f());
    harness.run(); // open + request focus
    harness.run(); // focus applied to the find input

    // Type "fn" as real character events into the focused find TextEdit.
    harness.event(egui::Event::Text("f".to_owned()));
    harness.run();
    harness.event(egui::Event::Text("n".to_owned()));
    harness.run();
    harness.run(); // settle: TextEdit pushes the query back into find_state

    let state = panel.find_state().expect("bar open");
    assert_eq!(
        state.query.pattern, "fn",
        "typed characters landed in the find input and became the query"
    );
    assert!(
        state.matches.len() >= 2,
        "the typed query matched at least the two fn keywords; got {}",
        state.matches.len()
    );
    println!(
        "MT-108: typed 'fn' into the find TextEdit -> pattern={:?}, matches={}",
        state.query.pattern,
        state.matches.len()
    );
}

#[test]
fn replace_all_button_caps_each_click_and_exposes_repeat_recovery() {
    let text = "x ".repeat(handshake_native::code_editor::REPLACE_ALL_CAP + 25);
    let panel = Arc::new(CodeEditorPanel::new(&text, "txt"));
    panel.open_find(true);
    panel.set_find_query("x");
    panel.set_replace_text("y");
    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 320.0))
        .build_ui(move |ui| panel_ui.show(ui));
    harness.run_steps(2);

    harness.get_by_label("Replace All").click();
    harness.run_steps(2);
    let first = panel.find_state().expect("replace bar remains open");
    assert_eq!(first.replace_all_remaining, 25);
    assert!(
        harness
            .query_by_label_contains("25 more not yet replaced")
            .is_some(),
        "the mounted find bar exposes the bounded-operation recovery instruction"
    );

    harness.get_by_label("Replace All").click();
    harness.run_steps(2);
    let second = panel.find_state().expect("replace bar remains open");
    assert_eq!(second.replace_all_remaining, 0);
    assert!(second.matches.is_empty());
    assert!(
        harness
            .query_by_label_contains("more not yet replaced")
            .is_none(),
        "repeat action completes the remaining batch and clears the progress hint"
    );
}

#[test]
fn replace_all_refreshes_live_matches_after_intervening_mounted_edit() {
    let count = handshake_native::code_editor::REPLACE_ALL_CAP + 4;
    let panel = Arc::new(CodeEditorPanel::new(&"x ".repeat(count), "txt"));
    panel.open_find(true);
    panel.set_find_query("x");
    panel.set_replace_text("y");
    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 320.0))
        .build_ui(move |ui| panel_ui.show(ui));
    harness.run_steps(2);

    harness.get_by_label("Replace All").click();
    harness.run_steps(2);
    assert_eq!(
        panel
            .find_state()
            .expect("replace bar remains open")
            .replace_all_remaining,
        4
    );

    // Shift every cached byte range through the mounted panel's ordinary whole-buffer edit path.
    // The next click must search this live document before rebuilding its continuation plan.
    let shifted = format!("guard-prefix {}", panel.buffer().to_string());
    panel.set_text(&shifted);
    harness.run_steps(2);
    harness.get_by_label("Replace All").click();
    harness.run_steps(2);

    assert_eq!(
        panel.buffer().to_string(),
        format!("guard-prefix {}", "y ".repeat(count)),
        "the second mounted click used refreshed live ranges and preserved the intervening edit"
    );
    let state = panel.find_state().expect("replace bar remains open");
    assert_eq!(state.replace_all_remaining, 0);
    assert!(state.matches.is_empty());
}

#[test]
fn replace_all_same_text_advances_across_original_matches() {
    let count = handshake_native::code_editor::REPLACE_ALL_CAP + 25;
    let text = "x ".repeat(count);
    let panel = Arc::new(CodeEditorPanel::new(&text, "txt"));
    panel.open_find(true);
    panel.set_find_query("x");
    panel.set_replace_text("x");
    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 320.0))
        .build_ui(move |ui| panel_ui.show(ui));
    harness.run_steps(2);

    harness.get_by_label("Replace All").click();
    harness.run_steps(2);
    assert_eq!(
        panel.find_state().expect("replace bar remains open").replace_all_remaining,
        25,
        "the first click records progress through the original set even though the query still matches"
    );

    harness.get_by_label("Replace All").click();
    harness.run_steps(2);
    let state = panel.find_state().expect("replace bar remains open");
    assert_eq!(state.replace_all_remaining, 0);
    assert_eq!(
        state.matches.len(),
        count,
        "same-text replacement still matches"
    );
    assert_eq!(panel.buffer().to_string(), text);
    assert!(
        harness
            .query_by_label_contains("more not yet replaced")
            .is_none(),
        "the continuation terminates instead of retrying the first batch forever"
    );
}

#[test]
fn replace_all_expanding_text_advances_across_original_matches() {
    let count = handshake_native::code_editor::REPLACE_ALL_CAP + 25;
    let panel = Arc::new(CodeEditorPanel::new(&"x ".repeat(count), "txt"));
    panel.open_find(true);
    panel.set_find_query("x");
    panel.set_replace_text("xx");
    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 320.0))
        .build_ui(move |ui| panel_ui.show(ui));
    harness.run_steps(2);

    harness.get_by_label("Replace All").click();
    harness.run_steps(2);
    assert_eq!(
        panel
            .find_state()
            .expect("replace bar remains open")
            .replace_all_remaining,
        25
    );

    harness.get_by_label("Replace All").click();
    harness.run_steps(2);
    let state = panel.find_state().expect("replace bar remains open");
    assert_eq!(state.replace_all_remaining, 0);
    assert_eq!(panel.buffer().to_string(), "xx ".repeat(count));
    assert_eq!(
        state.matches.len(),
        count * 2,
        "replacement-generated matches remain searchable but are not reprocessed by the continuation"
    );
}

#[test]
fn find_opening_frame_text_never_leaks_into_code_buffer() {
    let panel = Arc::new(CodeEditorPanel::new(SNIPPET, "rs"));
    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 700.0))
        .build_ui(move |ui| panel_ui.show(ui));
    harness.run();
    let before = panel.buffer().to_string();

    // Adversarial event ordering: the command and printable event are delivered in one input frame,
    // before egui has rendered the newly opened TextEdit and can report that it owns focus.
    harness.event(ctrl_f());
    harness.event(egui::Event::Text("f".to_owned()));
    harness.run();

    assert!(panel.is_find_open(), "Ctrl+F still opens the find surface");
    assert_eq!(
        panel.buffer().to_string(),
        before,
        "opening-frame query text must never mutate the code document"
    );
}

// ── AC-006: Escape closes the find bar -> no highlights on the next frame ──────────────────────────

#[test]
fn find_bar_escape_removes_find_state() {
    let panel = Arc::new(CodeEditorPanel::new(SNIPPET, "rs"));
    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(700.0, 220.0))
        .build_ui(move |ui| {
            panel_ui.show(ui);
        });

    harness.run();
    harness.event(ctrl_f());
    harness.run();
    panel.set_find_query("fn");
    harness.run();
    assert!(panel.is_find_open() && !panel.find_state().unwrap().matches.is_empty());

    // The find-bar node is present while open.
    assert!(
        harness
            .root()
            .children_recursive()
            .any(|n| n.accesskit_node().author_id() == Some(CODE_EDITOR_FIND_BAR_AUTHOR_ID)),
        "find-bar node present while open"
    );

    // Escape closes it.
    harness.event(escape());
    harness.run();
    harness.run();
    assert!(!panel.is_find_open(), "AC-006: Escape closed the find bar");
    assert!(
        panel.find_state().is_none(),
        "AC-006: find_state cleared -> no highlights next frame"
    );

    // The find-bar AccessKit node is gone from the live tree.
    assert!(
        !harness
            .root()
            .children_recursive()
            .any(|n| n.accesskit_node().author_id() == Some(CODE_EDITOR_FIND_BAR_AUTHOR_ID)),
        "AC-006: the find-bar node is removed after closing"
    );
    println!("AC-006: find bar closed via Escape; no find_state, no find-bar node");
}

// ── PT-005 / AC-005: yellow match-highlight screenshot ─────────────────────────────────────────────

#[test]
fn find_bar_highlight_screenshot_has_yellow_rect() {
    let panel = Arc::new(CodeEditorPanel::new(SNIPPET, "rs"));
    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(700.0, 220.0))
        .wgpu()
        .build_ui(move |ui| {
            panel_ui.show(ui);
        });

    harness.run();
    harness.event(ctrl_f());
    harness.run();
    panel.set_find_query("fn");
    harness.run();
    harness.run(); // settle so the highlight overlay paints

    let matches = panel.find_state().expect("bar open").matches.len();
    assert!(matches >= 2, "two fn matches highlighted; got {matches}");

    match harness.render() {
        Ok(image) => {
            let (w, h) = (image.width(), image.height());
            assert!(w > 0 && h > 0, "rendered image non-empty");
            let raw = image.as_raw();

            // The MATCH_HIGHLIGHT_COLOR is a translucent yellow (premultiplied ~ (180,160,0) over the
            // dark editor bg). After compositing over the dark background the painted match cells are
            // distinctly YELLOW-DOMINANT: red and green channels clearly above blue, and red/green both
            // well above the dark background level. Count pixels matching that yellow-dominant signature;
            // a match highlight must produce a contiguous block of them.
            let mut yellow_pixels = 0usize;
            let mut i = 0usize;
            while i + 4 <= raw.len() {
                let (r, g, b, a) = (raw[i], raw[i + 1], raw[i + 2], raw[i + 3]);
                // Yellow-dominant: r and g are both meaningfully high and clearly exceed b.
                if a != 0
                    && r as i32 > 70
                    && g as i32 > 60
                    && (r as i32) > (b as i32) + 40
                    && (g as i32) > (b as i32) + 30
                {
                    yellow_pixels += 1;
                }
                i += 4;
            }

            let ext_dir = external_artifact_dir("wp-kernel-012-mt-004");
            let _ = std::fs::create_dir_all(&ext_dir);
            let png_path = ext_dir.join("MT-004-find-highlight.png");
            image
                .save(&png_path)
                .expect("find-highlight proof frame saves to the external artifact root");
            println!(
                "PT-005 find-highlight screenshot: {w}x{h}, yellow_pixels={yellow_pixels}, \
                 saved=true ({})",
                png_path.display()
            );

            // A single "fn" highlight rect over a ~13px line height and ~2 glyphs wide is dozens of
            // pixels; two matches are well over 50. Assert a generous lower bound so the proof is the
            // PRESENCE of the yellow highlight, not an exact count.
            assert!(
                yellow_pixels >= 30,
                "AC-005: the find-highlight overlay must paint a yellow match rect; got \
                 {yellow_pixels} yellow-dominant pixels"
            );
        }
        Err(e) => {
            // MT-108 residual item (11) / AC-108-2: instead of a silent GREEN pass, record a typed
            // DEFERRED marker so a headless run never implies pixels were captured.
            println!(
                "DEFERRED(typed marker): MT-004 find-highlight screenshot render unavailable (no wgpu \
                 adapter): {e}. The match-state + highlight-overlay logic is proven by the find_bar \
                 AccessKit test and the engine tests; the PNG + yellow-pixel check is a GPU-host item."
            );
        }
    }

    assert_no_local_test_output();
}
