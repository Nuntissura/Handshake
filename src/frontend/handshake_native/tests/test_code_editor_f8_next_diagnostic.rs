//! MT-052 kittest proofs (WP-KERNEL-012 — E1 GO-menu navigation): F8 diagnostic traversal through the
//! LIVE keymap path + the GO-menu AccessKit MenuItem nodes.
//!
//! - AC-006 / PT-005 (`code_editor_f8_next_diagnostic`): a REAL F8 key event pushed into the live panel
//!   input loop (the SAME path the keymap takes — `process_keymap` -> resolve -> dispatch_action ->
//!   go_to_next_diagnostic) moves the caret to the next diagnostic line; pressing it again advances;
//!   pressing it at the last marker wraps to the first. Plus a screenshot to the EXTERNAL artifact root.
//! - AC-007 / PT-006 (`go_menu_editor_nav_accesskit_nodes`): the four GO-menu items render as
//!   `Role::MenuItem` AccessKit nodes (`menu-go-next-diagnostic` / `menu-go-prev-diagnostic` /
//!   `menu-go-back` / `menu-go-forward`) and are DISABLED until the editor is host-mounted (E11), the
//!   MT-050 disabled-until-mounted precedent.
//!
//! ## Artifact hygiene (CX-212E)
//!
//! Every screenshot is written to the EXTERNAL `../../../../Handshake_Artifacts/handshake-test/<subdir>/`
//! root via [`external_artifact_dir`], NEVER repo-local. [`assert_no_local_artifact_dir`] guards both
//! `test_output/` and `tests/screenshots/` so a stray repo-local artifact dir fails the test.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use egui::Key;
use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;

use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend_client::HealthInfo;
use handshake_native::code_editor::{
    CodeEditorPanel, DiagnosticSeverity, GutterMarker,
};
use handshake_native::top_menu_bar::{
    GO_BACK_AUTHOR_ID, GO_FORWARD_AUTHOR_ID, GO_NEXT_DIAGNOSTIC_AUTHOR_ID, GO_PREV_DIAGNOSTIC_AUTHOR_ID,
};

fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

/// CX-212E guard: NO repo-local artifact directory may exist (checks BOTH `test_output/` and
/// `tests/screenshots/`). Screenshots go only to the external Handshake_Artifacts root.
fn assert_no_local_artifact_dir() {
    for local in ["test_output", "tests/screenshots"] {
        let p = Path::new(local);
        assert!(
            !p.exists(),
            "no repo-local artifact dir may exist — artifacts go to the external \
             Handshake_Artifacts/handshake-test root only (found {})",
            p.display()
        );
    }
}

fn diag(line: usize) -> GutterMarker {
    GutterMarker::diagnostic(line, DiagnosticSeverity::Error, "boom")
}

fn numbered_buffer(lines: usize) -> String {
    (0..lines).map(|i| format!("line{i}\n")).collect()
}

fn caret_line(panel: &CodeEditorPanel) -> usize {
    let buffer = panel.buffer();
    let cursors = panel.cursors();
    handshake_native::code_editor::byte_to_line_col(cursors.primary().head, &buffer).0
}

/// Push one F8 key-down (with `shift` optionally) and run two frames so `process_keymap` services it.
fn press_f8(harness: &mut Harness<'static>, shift: bool) {
    harness.event(egui::Event::Key {
        key: Key::F8,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers { shift, ..Default::default() },
    });
    harness.run();
    harness.run();
}

// ── AC-006 / PT-005: F8 through the LIVE keymap path moves the caret + wraps + screenshot ───────────

#[test]
fn code_editor_f8_next_diagnostic() {
    let panel = Arc::new(CodeEditorPanel::new(&numbered_buffer(30), "rs"));
    panel.push_diagnostics(vec![diag(5), diag(10), diag(20)]);

    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 360.0))
        .build_ui(move |ui| {
            panel_ui.show(ui);
        });
    harness.run();
    assert_eq!(caret_line(&panel), 0, "caret starts at line 0");

    // A REAL F8 event routed through process_keymap -> resolve(F8) -> GoToNextDiagnostic.
    press_f8(&mut harness, false);
    assert_eq!(caret_line(&panel), 5, "AC-006: F8 moves the caret to the next diagnostic line (5)");
    press_f8(&mut harness, false);
    assert_eq!(caret_line(&panel), 10, "AC-006: F8 again advances to line 10");
    press_f8(&mut harness, false);
    assert_eq!(caret_line(&panel), 20, "AC-006: F8 again advances to line 20");
    press_f8(&mut harness, false);
    assert_eq!(caret_line(&panel), 5, "AC-006: F8 at the last marker wraps to the first (5)");

    // Shift+F8 (Go to Previous Problem) steps backward through the live path.
    press_f8(&mut harness, true);
    assert_eq!(caret_line(&panel), 20, "Shift+F8 steps to the previous marker (wraps to 20)");

    // HBR-VIS: screenshot the editor with diagnostics present. On a GPU host this saves a PNG to the
    // EXTERNAL artifact root; absent a wgpu adapter, record an honest non-fatal note.
    match harness.render() {
        Ok(image) => {
            let (w, h) = (image.width(), image.height());
            assert!(w > 0 && h > 0, "rendered image is non-empty");
            let ext_dir = external_artifact_dir("wp-kernel-012-mt-052");
            let _ = std::fs::create_dir_all(&ext_dir);
            let png_path = ext_dir.join("MT-052-f8-diagnostic-nav.png");
            let saved = image.save(&png_path).is_ok();
            let abs = std::fs::canonicalize(&png_path).unwrap_or(png_path.clone());
            println!("PT-005 F8-nav screenshot: {w}x{h}, saved={saved} ({})", abs.display());
            assert!(saved, "PT-005: the F8-nav screenshot PNG saved to the external root");
        }
        Err(e) => {
            println!(
                "BLOCKER(non-fatal): MT-052 F8-nav screenshot render unavailable (no wgpu adapter): \
                 {e}. The live-keymap F8 caret-move proof above passed; the PNG is a GPU-host item."
            );
        }
    }
    assert_no_local_artifact_dir();
}

// ── AC-007 / PT-006: GO-menu editor-navigation AccessKit MenuItem nodes (disabled-until-E11) ────────

fn ok_app() -> HandshakeApp {
    HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }))
}

#[test]
fn go_menu_editor_nav_accesskit_nodes() {
    // Drive the REAL shell so the GO menu renders through the live menu bar (the same out-of-process
    // path a swarm agent uses). Open the GO menu, then assert the four MT-052 leaf nodes are present as
    // disabled Role::MenuItem nodes with the contract-named author_ids.
    let mut harness =
        Harness::builder().build_state(|ctx, a: &mut HandshakeApp| a.ui(ctx), ok_app());
    harness.run();

    // Open the GO menu by clicking its top-level button (the same out-of-process click path
    // test_top_menu_bar.rs uses).
    harness.get_by_label("GO").click();
    harness.run();
    harness.run();

    // Walk the live tree for the four MT-052 GO-menu leaf nodes.
    let required = [
        GO_NEXT_DIAGNOSTIC_AUTHOR_ID,
        GO_PREV_DIAGNOSTIC_AUTHOR_ID,
        GO_BACK_AUTHOR_ID,
        GO_FORWARD_AUTHOR_ID,
    ];
    let mut found: Vec<(String, String, bool)> = Vec::new();
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        if let Some(author) = ak.author_id() {
            if required.contains(&author) {
                let role = format!("{:?}", ak.role());
                // A disabled node carries accesskit `is_disabled()` (egui add_enabled(false)).
                let disabled = ak.is_disabled();
                found.push((author.to_owned(), role, disabled));
            }
        }
    }
    found.sort();
    println!("PT-006 GO-menu MT-052 nodes: {found:?}");

    for id in required {
        let entry = found
            .iter()
            .find(|(a, _, _)| a == id)
            .unwrap_or_else(|| panic!("AC-007: GO-menu node {id} missing from the live tree: {found:?}"));
        assert_eq!(entry.1, "MenuItem", "AC-007: {id} is a Role::MenuItem node");
        assert!(
            entry.2,
            "AC-007: {id} is DISABLED until the editor is host-mounted (E11) — MT-050 precedent"
        );
    }
    assert_eq!(found.len(), 4, "all four MT-052 GO-menu nodes present exactly once");
    assert_no_local_artifact_dir();
}
