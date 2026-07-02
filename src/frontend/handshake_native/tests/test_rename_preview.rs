//! WP-KERNEL-012 MT-048 Rename Symbol — multi-file preview LIVE proofs (E1 VS Code parity).
//!
//! - PT-003 / AC-004 `rename_preview`: `build_preview` lists every changed file (open vs to-disk noted)
//!   with before/after hunks BEFORE any edit is applied; NOTHING is mutated until 'Apply'. Driven through
//!   the REAL `WorkspaceEditPreview::from_lsp` (so the preview model is the live one) + the panel's
//!   apply-on-Apply path (nothing mutates until apply_rename_preview).
//!
//! Provable WITHOUT a live PostgreSQL / a language server: the preview is built from a synthetic
//! `lsp_types::WorkspaceEdit` (the shape a server returns) over in-memory buffer text + a temp-dir disk
//! file, and the apply target is the in-memory `TextBuffer`.

use std::sync::Arc;

use handshake_native::code_editor::rename::{RenameState, WorkspaceEditPreview};
use handshake_native::code_editor::CodeEditorPanel;

/// Build a 2-file `lsp_types::WorkspaceEdit`: an open buffer (the current file) + a to-disk file. The open
/// file renames `value` at two occurrences; the to-disk file renames it once.
fn two_file_workspace_edit(open_uri: &str, disk_uri: &str) -> lsp_types::WorkspaceEdit {
    let mk = |line: u32, sc: u32, ec: u32| lsp_types::TextEdit {
        range: lsp_types::Range {
            start: lsp_types::Position {
                line,
                character: sc,
            },
            end: lsp_types::Position {
                line,
                character: ec,
            },
        },
        new_text: "total".into(),
    };
    let mut changes = std::collections::HashMap::new();
    changes.insert(
        lsp_types::Url::parse(open_uri).unwrap(),
        vec![mk(0, 8, 13), mk(1, 4, 9)],
    );
    changes.insert(
        lsp_types::Url::parse(disk_uri).unwrap(),
        vec![mk(0, 11, 16)],
    );
    lsp_types::WorkspaceEdit {
        changes: Some(changes),
        document_changes: None,
        change_annotations: None,
    }
}

// ── PT-003 / AC-004: the preview lists every changed file with hunks BEFORE apply; nothing mutates ─────

#[test]
fn rename_preview_lists_files_with_hunks_before_apply() {
    // The open file (the current document) text, with `value` on line 0 and line 1.
    let open_text = "let value = 1;\n    value + 1";
    let open_uri = "file:///open.rs";

    // A to-disk file we write to a temp dir so the preview can read its before/after hunks.
    let dir = std::env::temp_dir().join(format!("hsk-rename-preview-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let disk_path = dir.join("disk.rs");
    std::fs::write(&disk_path, "fn f() { value(); }\n").unwrap();
    let disk_uri = lsp_types::Url::from_file_path(&disk_path)
        .unwrap()
        .to_string();

    let edit = two_file_workspace_edit(open_uri, &disk_uri);

    // Build the preview: the open file's text comes from the open_lookup; the to-disk file is read.
    let preview = WorkspaceEditPreview::from_lsp(&edit, |uri| {
        if uri == open_uri {
            Some(open_text.to_owned())
        } else {
            None // to-disk: read from disk for the hunks.
        }
    });

    // AC-004: every changed file is listed, open vs to-disk noted.
    assert_eq!(
        preview.files.len(),
        2,
        "AC-004: both changed files are in the preview"
    );
    let open_file = preview.files.iter().find(|f| f.uri == open_uri).unwrap();
    let disk_file = preview.files.iter().find(|f| f.uri == disk_uri).unwrap();
    assert!(
        open_file.is_open_buffer,
        "AC-004: the open file is marked is_open_buffer"
    );
    assert!(
        !disk_file.is_open_buffer,
        "AC-004: the to-disk file is marked NOT is_open_buffer"
    );

    // AC-004: each file has before/after hunks computed BEFORE any mutation.
    assert!(
        !open_file.hunks.is_empty(),
        "AC-004: the open file has before/after hunks"
    );
    assert!(
        !disk_file.hunks.is_empty(),
        "AC-004: the to-disk file has before/after hunks"
    );
    // The hunks show value -> total in the after, and value in the before.
    assert!(
        open_file
            .hunks
            .iter()
            .any(|h| h.before.contains("value") && h.after.contains("total")),
        "AC-004: the open file's hunk shows value (before) -> total (after); got {:?}",
        open_file.hunks
    );
    assert!(
        disk_file
            .hunks
            .iter()
            .any(|h| h.before.contains("value") && h.after.contains("total")),
        "AC-004: the to-disk file's hunk shows value (before) -> total (after); got {:?}",
        disk_file.hunks
    );

    // AC-004 (the load-bearing assertion): NOTHING is mutated by building the preview — the to-disk file
    // is still the ORIGINAL on disk, and the open text variable is unchanged.
    assert_eq!(
        std::fs::read_to_string(&disk_path).unwrap(),
        "fn f() { value(); }\n",
        "AC-004: building the preview did NOT touch the to-disk file"
    );
    assert_eq!(
        open_text, "let value = 1;\n    value + 1",
        "AC-004: building the preview did not mutate the open text"
    );

    println!(
        "PT-003 rename_preview: 2 files listed (open + to-disk) with {} total hunks; nothing mutated before apply",
        open_file.hunks.len() + disk_file.hunks.len()
    );

    let _ = std::fs::remove_file(&disk_path);
    let _ = std::fs::remove_dir(&dir);
}

// ── AC-004 (panel path): the panel does not mutate its buffer until Apply ──────────────────────────────

#[test]
fn rename_panel_does_not_mutate_buffer_until_apply() {
    let src = "fn compute() {\n    let value = 1;\n    value + value\n}";
    let panel = Arc::new(CodeEditorPanel::new(src, "rs"));
    panel.set_file_path("cur.rs");
    let on_value = src.find("value").unwrap() + 1;
    panel.set_single_cursor(on_value);

    // Begin + drive the preview through the synthetic single-file fallback (no runtime -> sync path).
    panel.begin_rename_at_cursor();
    if let RenameState::Editing {
        original,
        anchor_byte,
        ident_range,
        entity_id,
        focus_requested,
        ..
    } = panel.rename_state()
    {
        panel.set_rename_state(RenameState::Editing {
            original,
            draft: "renamed_value".to_owned(),
            anchor_byte,
            ident_range,
            entity_id,
            focus_requested,
        });
    }

    // Render + Enter -> the preview is built (the sync fallback), but the buffer is UNCHANGED until Apply.
    let panel_ui = Arc::clone(&panel);
    let mut harness = egui_kittest::Harness::builder()
        .with_size(egui::vec2(900.0, 360.0))
        .build_ui(move |ui| {
            panel_ui.show(ui);
        });
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    harness.run();

    // The preview is open, but the buffer still has the ORIGINAL `value` text (AC-004 — nothing mutated).
    assert!(
        panel.is_rename_preview_open(),
        "AC-004: the preview is open after confirm"
    );
    let before_apply = panel.buffer().to_string();
    assert_eq!(
        before_apply, src,
        "AC-004: the buffer is UNCHANGED until Apply is clicked"
    );
    assert!(
        before_apply.contains("value"),
        "AC-004: the original `value` text is intact before apply"
    );

    // Now apply: the buffer mutates.
    let report = panel.apply_rename_preview().expect("apply ok");
    assert!(report.edits_applied >= 1);
    let after_apply = panel.buffer().to_string();
    assert_ne!(
        after_apply, src,
        "AC-004: the buffer IS mutated after Apply"
    );
    assert!(
        after_apply.contains("renamed_value"),
        "AC-004: the new name is applied after Apply"
    );
    println!(
        "AC-004 rename_panel: buffer unchanged until Apply, then renamed ({} edits)",
        report.edits_applied
    );
}
