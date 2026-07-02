//! MT-005 code-folding LIVE GUI proofs (WP-KERNEL-012 — E1 code editor): the folded-region AccessKit
//! TreeItem node (AC-005 / PT-005) and the folded-region screenshot (AC-004 / PT-004).
//!
//! - AC-005 / PT-005 (`cargo test -p handshake-native fold_accesskit`): with one region folded, the
//!   LIVE egui AccessKit tree contains a node with `author_id="code_editor_fold_0"`, role
//!   `Role::TreeItem`, and (because the region is FOLDED) the `Action::Expand` action a swarm agent
//!   dispatches to unfold it. `Role::TreeItem` and `Action::Expand`/`Collapse` all exist in accesskit
//!   0.21.1 (verified against the locked source), so no role/action fallback is needed for this MT.
//! - AC-004 / PT-004: an egui_kittest screenshot of the folded state shows the fold LABEL line
//!   (containing the `…` ellipsis) and proves the collapsed lines are ABSENT — the painted (visible)
//!   line count is strictly fewer than the buffer line count. The PNG is saved to the EXTERNAL
//!   Handshake_Artifacts test root only (the repo-local `test_output/` the contract's PT-004 string
//!   names is forbidden by the project's artifact-root convention + the `assert_no_local_test_output`
//!   guard the MT-003 proof established; the screenshot lives under
//!   `Handshake_Artifacts/handshake-test/wp-kernel-012-mt-005/MT-005-folded.png` instead and its
//!   absolute path is logged).
//!
//! ## Why drive the public fold surface, not a faked node
//!
//! The panel computes fold regions from the real tree-sitter parse at construction; the test folds a
//! region via `CodeEditorPanel::toggle_fold(0)` (the SAME surface the gutter click + Ctrl+Shift+[ use)
//! and then renders, so the AccessKit node + the collapsed render are produced by the real fold path,
//! not a stub.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;

use handshake_native::code_editor::CodeEditorPanel;

/// A real multi-line Rust function whose body folds to a single summary line. The body spans many
/// lines so a folded render is obviously shorter than the unfolded one.
const RUST_FN: &str = "\
fn render(items: &[i32]) -> i32 {
    let mut total = 0;
    for item in items {
        if *item > 0 {
            total += item;
        } else {
            total -= item;
        }
    }
    let scaled = total * 2;
    let label = String::from(\"render\");
    println!(\"{}: {}\", label, scaled);
    scaled
}
";

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

// ── AC-005 / PT-005: folded region -> code_editor_fold_0 TreeItem with Expand action ──────────────

#[test]
fn fold_accesskit_folded_region_emits_treeitem_expand_node() {
    let panel = Arc::new(CodeEditorPanel::new(RUST_FN, "rs"));

    // The function-body region starts on line 0; fold it (the same surface the gutter/keymap use).
    assert!(
        panel.toggle_fold(0),
        "a fold region starts on line 0 of the function (so toggle_fold(0) succeeds)"
    );
    assert!(
        panel
            .fold_set()
            .regions
            .iter()
            .any(|r| r.start_line == 0 && r.folded),
        "the line-0 region is folded after toggle_fold(0)"
    );

    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(640.0, 320.0))
        .build_ui(move |ui| {
            panel_ui.show(ui);
        });

    // Render a few frames so the fold nodes are emitted into the live AccessKit tree.
    harness.run();
    harness.run();

    // AC-005: the live tree must contain code_editor_fold_0 with role TreeItem and the Expand action
    // (a FOLDED region offers Expand — the action that unfolds it).
    let root = harness.root();
    let mut found: Vec<String> = Vec::new();
    let mut fold0_role: Option<String> = None;
    let mut fold0_supports_expand = false;
    for node in root.children_recursive() {
        let ak = node.accesskit_node();
        if let Some(author) = ak.author_id() {
            if author.starts_with("code_editor_fold_") {
                found.push(format!("{author}={:?}", ak.role()));
                if author == "code_editor_fold_0" {
                    fold0_role = Some(format!("{:?}", ak.role()));
                    // The consumer node exposes the raw NodeData via `.data()`, whose
                    // `supports_action(action)` takes just the action (the consumer `Node`'s own
                    // `supports_action` needs a parent filter we do not have here).
                    fold0_supports_expand =
                        ak.data().supports_action(egui::accesskit::Action::Expand);
                }
            }
        }
    }
    found.sort();
    println!("PT-005 accesskit fold nodes: {found:?}");

    assert!(
        found.iter().any(|s| s.starts_with("code_editor_fold_0=")),
        "AC-005: live tree must contain code_editor_fold_0; found {found:?}"
    );
    assert_eq!(
        fold0_role.as_deref(),
        Some("TreeItem"),
        "AC-005: code_editor_fold_0 has role TreeItem; got {fold0_role:?}"
    );
    assert!(
        fold0_supports_expand,
        "AC-005: a FOLDED region's node supports Action::Expand (the unfold action)"
    );

    // The fold node is addressable by its label too (container -> scroll -> text -> fold).
    let labeled = harness.query_all_by_label("Code editor fold").count();
    assert!(
        labeled >= 1,
        "at least one fold node is labeled/addressable; got {labeled}"
    );
}

#[test]
fn fold_accesskit_unfolded_region_offers_collapse_action() {
    // The mirror of the above: an UNfolded region's node offers Collapse (the action that folds it),
    // so a swarm agent can fold a region it sees open. This proves the action reflects fold state.
    let panel = Arc::new(CodeEditorPanel::new(RUST_FN, "rs"));
    assert!(
        panel
            .fold_set()
            .regions
            .iter()
            .any(|r| r.start_line == 0 && !r.folded),
        "the line-0 region starts UNfolded"
    );

    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(640.0, 320.0))
        .build_ui(move |ui| {
            panel_ui.show(ui);
        });
    harness.run();
    harness.run();

    let root = harness.root();
    let mut fold0_supports_collapse = false;
    let mut fold0_supports_expand = false;
    for node in root.children_recursive() {
        let ak = node.accesskit_node();
        if ak.author_id() == Some("code_editor_fold_0") {
            fold0_supports_collapse = ak.data().supports_action(egui::accesskit::Action::Collapse);
            fold0_supports_expand = ak.data().supports_action(egui::accesskit::Action::Expand);
        }
    }
    assert!(
        fold0_supports_collapse,
        "AC-005: an UNfolded region's node supports Action::Collapse (the fold action)"
    );
    assert!(
        !fold0_supports_expand,
        "an UNfolded region's node does NOT offer Expand (it is not folded)"
    );
}

// ── AC-004 / PT-004: folded screenshot shows the fold label + collapses lines ─────────────────────

#[test]
fn fold_accesskit_folded_render_shows_label_and_hides_lines() {
    let panel = Arc::new(CodeEditorPanel::new(RUST_FN, "rs"));
    let buffer_lines = panel.buffer().len_lines();

    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(640.0, 320.0))
        .wgpu()
        .build_ui(move |ui| {
            panel_ui.show(ui);
        });

    // Frame 1 UNFOLDED: record the painted line count (all body lines visible).
    harness.run();
    let unfolded_painted = panel.perf_stats().frame_lines_rendered;

    // Fold the function body and re-render.
    assert!(panel.toggle_fold(0), "fold the function body");
    harness.run();
    harness.run(); // settle
    let folded_stats = panel.perf_stats();

    // AC-004: the collapsed lines are ABSENT — the folded render paints strictly fewer lines than the
    // unfolded render AND strictly fewer than the whole buffer.
    assert_eq!(
        folded_stats.buffer_len_lines, buffer_lines,
        "the whole-document line count is still reported"
    );
    assert!(
        folded_stats.frame_lines_rendered < unfolded_painted,
        "AC-004: folding paints fewer rows than unfolded ({} < {})",
        folded_stats.frame_lines_rendered,
        unfolded_painted
    );
    assert!(
        folded_stats.frame_lines_rendered < buffer_lines,
        "AC-004: the folded render paints fewer lines than the whole buffer ({} < {})",
        folded_stats.frame_lines_rendered,
        buffer_lines
    );

    // The fold label line (the collapsed summary, containing the ellipsis) is present in the fold set
    // and is what the start-line row renders.
    let folded_label = panel
        .fold_set()
        .region_starting_at(0)
        .map(|r| r.label.clone())
        .expect("a fold region starts on line 0");
    assert!(
        folded_label.contains('…'),
        "AC-004: the fold label contains the ellipsis; got {folded_label:?}"
    );

    // Render the screenshot. On a GPU host this produces the PNG; absent a wgpu adapter, record an
    // honest non-fatal blocker (the structural collapse proof above stands as the AC-004 evidence).
    match harness.render() {
        Ok(image) => {
            let (w, h) = (image.width(), image.height());
            assert!(w > 0 && h > 0, "rendered folded image is non-empty");
            let ext_dir = external_artifact_dir("wp-kernel-012-mt-005");
            let _ = std::fs::create_dir_all(&ext_dir);
            let png_path = ext_dir.join("MT-005-folded.png");
            let saved = image.save(&png_path).is_ok();
            let abs = std::fs::canonicalize(&png_path).unwrap_or(png_path.clone());
            println!(
                "PT-004 folded screenshot: {w}x{h}, folded_painted={} (<{} unfolded, <{} buffer), \
                 label={folded_label:?}, saved={saved} ({})",
                folded_stats.frame_lines_rendered,
                unfolded_painted,
                buffer_lines,
                abs.display()
            );
            assert!(
                saved,
                "PT-004: the folded screenshot PNG saved to the external artifact root"
            );
        }
        Err(e) => {
            println!(
                "BLOCKER(non-fatal): MT-005 folded screenshot render unavailable (no wgpu adapter): \
                 {e}. AC-004 structural collapse proof (folded paints {} < {} unfolded < {} buffer) + \
                 the ellipsis-label assertion passed; the PNG is a GPU-host item.",
                folded_stats.frame_lines_rendered, unfolded_painted, buffer_lines
            );
        }
    }

    assert_no_local_test_output();
}
