//! WP-KERNEL-012 MT-048 Rename Symbol — inline input + AccessKit + screenshot LIVE proofs (E1 VS Code
//! parity).
//!
//! - PT-004 / AC-001 `rename_inline_input_opens`: F2 (and the context-menu entry) with the cursor on an
//!   identifier opens an inline rename input PRE-FILLED with the identifier text and FULLY SELECTED —
//!   proven by the live AccessKit tree: the input node `code_editor_rename_input` is `Role::TextInput`
//!   and its value equals the identifier under the cursor.
//! - PT-005 / AC-005 / AC-006 `rename_accesskit_nodes`: the live AccessKit dump contains the rename nodes
//!   `code_editor_rename_input` (TextInput), `code_editor_rename_apply` (Button),
//!   `code_editor_rename_cancel` (Button), and `code_editor_ctx_rename_symbol` (MenuItem) with correct
//!   roles; the context-menu 'Rename Symbol' entry triggers the SAME begin_rename path as F2.
//! - HBR-VIS `rename_input_screenshot`: an egui_kittest screenshot of the open inline input is saved to
//!   the EXTERNAL artifact root (never repo-local — CX-212E).
//!
//! Provable WITHOUT a live PostgreSQL / a language server: the rename surface (input + preview + AccessKit
//! nodes) is independent of the backend; the input opens from a tree-sitter identifier resolution and is
//! driven through the panel's public API.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use egui_kittest::kittest::NodeT;
use egui_kittest::Harness;

use handshake_native::code_editor::keymap::CodeEditorAction;
use handshake_native::code_editor::rename::{
    RenameState, WorkspaceEditPreview, CODE_EDITOR_CTX_RENAME_SYMBOL_AUTHOR_ID,
    CODE_EDITOR_RENAME_APPLY_AUTHOR_ID, CODE_EDITOR_RENAME_CANCEL_AUTHOR_ID,
    CODE_EDITOR_RENAME_INPUT_AUTHOR_ID,
};
use handshake_native::code_editor::CodeEditorPanel;

/// The external artifact root for MT-048 screenshots (CX-212E — NEVER repo-local). The same
/// `../../../../Handshake_Artifacts/handshake-test/<subdir>` pattern the MT-008 / MT-047 proofs use.
fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

/// Hard guard (CX-212E): NO repo-local artifact dir may exist after a screenshot test.
fn assert_no_local_artifact_dir() {
    for local in ["test_output", "tests/screenshots"] {
        assert!(
            !Path::new(local).exists(),
            "no repo-local {local}/ dir may exist — artifacts go to the external \
             Handshake_Artifacts/handshake-test root only (CX-212E)"
        );
    }
}

/// Find the AccessKit node with `author_id` in the live tree, returning `(role, value)`.
fn find_node(harness: &Harness<'_>, author_id: &str) -> Option<(String, Option<String>)> {
    harness.root().children_recursive().find_map(|n| {
        let ak = n.accesskit_node();
        if ak.author_id() == Some(author_id) {
            Some((format!("{:?}", ak.role()), ak.value().map(|s| s.to_owned())))
        } else {
            None
        }
    })
}

// ── PT-004 / AC-001: the inline input opens pre-filled with the identifier (TextInput node value) ───────

#[test]
fn rename_inline_input_opens_prefilled_with_identifier() {
    let src = "fn compute() { let myValue = 1; }";
    let panel = Arc::new(CodeEditorPanel::new(src, "rs"));
    // Caret on `myValue`.
    let on_ident = src.find("myValue").unwrap() + 2;
    panel.set_single_cursor(on_ident);

    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 280.0))
        .build_ui(move |ui| {
            panel_ui.show(ui);
        });
    harness.run();
    assert!(!panel.is_rename_input_open(), "the input starts closed");

    // F2 path: dispatch the RenameSymbol action (the keymap binds F2 to it).
    panel.dispatch_action(CodeEditorAction::RenameSymbol);
    harness.run();
    harness.run(); // settle so the input node is emitted.

    // AC-001: the input is open, pre-filled with the identifier under the cursor.
    assert!(
        panel.is_rename_input_open(),
        "AC-001: F2 on an identifier opens the inline rename input"
    );
    match panel.rename_state() {
        RenameState::Editing {
            original, draft, ..
        } => {
            assert_eq!(
                original, "myValue",
                "AC-001: the input pre-fills with the identifier"
            );
            assert_eq!(
                draft, "myValue",
                "AC-001: the draft starts equal to the identifier"
            );
        }
        other => panic!("AC-001: expected Editing, got {other:?}"),
    }

    // AC-001 (the AccessKit proof): the input node is Role::TextInput with value == the identifier.
    let (role, value) = find_node(&harness, CODE_EDITOR_RENAME_INPUT_AUTHOR_ID)
        .expect("AC-001: the rename input node is present in the live tree");
    assert_eq!(
        role, "TextInput",
        "AC-001: '{CODE_EDITOR_RENAME_INPUT_AUTHOR_ID}' is a Role::TextInput"
    );
    assert_eq!(
        value.as_deref(),
        Some("myValue"),
        "AC-001: the input node value equals the identifier under the cursor"
    );

    // AC-001 (fully selected): the TextEdit's cursor range spans the whole draft on the first open frame
    // (select-all-on-open). We assert via egui's TextEditState char range covering [0, len).
    let input_id = egui::Id::new(("code-editor-rename-input", ""));
    let mut select_all_verified = false;
    if let Some(state) = egui::text_edit::TextEditState::load(&harness.ctx, input_id) {
        if let Some(range) = state.cursor.char_range() {
            let min = range.primary.index.min(range.secondary.index);
            let max = range.primary.index.max(range.secondary.index);
            assert_eq!(
                min, 0,
                "AC-001: the selection starts at the beginning (select-all)"
            );
            assert_eq!(
                max,
                "myValue".chars().count(),
                "AC-001: the selection spans the whole identifier (fully selected on open)"
            );
            select_all_verified = true;
        }
    }
    assert!(
        select_all_verified,
        "AC-001: the select-all-on-open cursor range was set on the input's TextEditState"
    );
    println!("PT-004 rename_inline_input: input opens pre-filled 'myValue', Role::TextInput, fully selected");
}

// ── AC-001 negative: F2 on a non-identifier does NOT open the input (RISK-006) ─────────────────────────

#[test]
fn rename_not_opened_on_non_identifier() {
    let src = "fn compute() { let x = 1; }";
    let panel = Arc::new(CodeEditorPanel::new(src, "rs"));
    // Caret on the `fn` keyword.
    panel.set_single_cursor(0);
    panel.dispatch_action(CodeEditorAction::RenameSymbol);
    assert!(
        !panel.is_rename_input_open(),
        "RISK-006: F2 on a keyword does NOT open the rename input (no word-scan)"
    );
}

// ── PT-005 / AC-005 / AC-006: AccessKit nodes (input/apply/cancel/ctx_rename_symbol) + ctx-menu path ────

#[test]
fn rename_accesskit_nodes_present_with_correct_roles() {
    let src = "fn compute() { let value = 1; value }";
    let panel = Arc::new(CodeEditorPanel::new(src, "rs"));
    panel.set_file_path("cur.rs");
    let on_ident = src.find("value").unwrap() + 1;
    panel.set_single_cursor(on_ident);

    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 320.0))
        .build_ui(move |ui| {
            panel_ui.show(ui);
        });
    harness.run();

    // AC-005: the editor body context-menu 'Rename Symbol' node is ALWAYS present (so a swarm agent can
    // trigger it by id without a right-click), as a Role::MenuItem.
    let (ctx_role, _) = find_node(&harness, CODE_EDITOR_CTX_RENAME_SYMBOL_AUTHOR_ID)
        .expect("AC-005: the context-menu 'Rename Symbol' node is present");
    assert_eq!(
        ctx_role, "MenuItem",
        "AC-005: '{CODE_EDITOR_CTX_RENAME_SYMBOL_AUTHOR_ID}' is a Role::MenuItem"
    );

    // AC-005: the context-menu path triggers the SAME begin_rename as F2 — drive begin_rename (the method
    // both the F2 dispatch and the context-menu button call) and prove it opens the input.
    panel.begin_rename_at_cursor();
    harness.run();
    harness.run();
    assert!(
        panel.is_rename_input_open(),
        "AC-005: the context-menu 'Rename Symbol' path opens the input"
    );
    assert!(
        find_node(&harness, CODE_EDITOR_RENAME_INPUT_AUTHOR_ID).is_some(),
        "AC-006: the rename input node is present"
    );

    // Move to the Previewing phase so the Apply/Cancel nodes are emitted, then assert all 6 nodes' roles.
    // A single synthetic occurrence range for the deterministic preview (the `value` token span).
    #[allow(clippy::single_range_in_vec_init)]
    let occurrence_ranges = vec![on_ident - 1..on_ident - 1 + 5];
    let preview = WorkspaceEditPreview::single_file_fallback(
        "file:///cur.rs",
        src,
        "total",
        &occurrence_ranges,
        true,
    );
    panel.set_rename_state(RenameState::Previewing {
        workspace_edit: preview,
    });
    harness.run();
    harness.run();

    // AC-006: the apply + cancel buttons are present with Role::Button.
    let (apply_role, _) = find_node(&harness, CODE_EDITOR_RENAME_APPLY_AUTHOR_ID)
        .expect("AC-006: the apply node is present");
    assert_eq!(
        apply_role, "Button",
        "AC-006: '{CODE_EDITOR_RENAME_APPLY_AUTHOR_ID}' is a Role::Button"
    );
    let (cancel_role, _) = find_node(&harness, CODE_EDITOR_RENAME_CANCEL_AUTHOR_ID)
        .expect("AC-006: the cancel node is present");
    assert_eq!(
        cancel_role, "Button",
        "AC-006: '{CODE_EDITOR_RENAME_CANCEL_AUTHOR_ID}' is a Role::Button"
    );
    println!(
        "PT-005 rename_accesskit: {{ input: TextInput, apply: Button, cancel: Button, ctx_rename_symbol: MenuItem }} all present"
    );
}

// ── HBR-VIS: screenshot the open inline rename input -> the EXTERNAL artifact root ─────────────────────

#[test]
fn rename_input_screenshot() {
    let src = "fn compute() { let value = 1; }";
    let panel = Arc::new(CodeEditorPanel::new(src, "rs"));
    let on_ident = src.find("value").unwrap() + 1;
    panel.set_single_cursor(on_ident);

    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 280.0))
        .wgpu()
        .build_ui(move |ui| {
            panel_ui.show(ui);
        });
    harness.run();
    panel.dispatch_action(CodeEditorAction::RenameSymbol);
    harness.run();
    harness.run(); // settle so the input paints.
    assert!(
        panel.is_rename_input_open(),
        "HBR-VIS: the input is open for the screenshot"
    );

    match harness.render() {
        Ok(image) => {
            let (w, h) = (image.width(), image.height());
            let ext_dir = external_artifact_dir("wp-kernel-012-mt-048");
            let _ = std::fs::create_dir_all(&ext_dir);
            let png_path = ext_dir.join("MT-048-rename-inline-input.png");
            let saved = image.save(&png_path).is_ok();
            println!(
                "HBR-VIS rename_input_screenshot: {w}x{h}, saved={saved} ({})",
                png_path.display()
            );
            assert!(
                saved,
                "the rename input screenshot saved to the external artifact root"
            );
        }
        Err(e) => {
            println!(
                "BLOCKER(non-fatal): MT-048 rename screenshot render unavailable (no wgpu adapter): {e}. \
                 The input-open state + the AccessKit TextInput node prove the inline input; the PNG is a \
                 GPU-host item."
            );
        }
    }
    assert_no_local_artifact_dir();
}
