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
    for local in [Path::new("test_output"), Path::new("tests/screenshots")] {
        assert!(
            !local.exists(),
            "no repo-local artifact dir may exist — artifacts go to the external \
             Handshake_Artifacts/handshake-test root only (found {})",
            local.display()
        );
    }
}

fn pixel_bounds(
    rect: egui::Rect,
    pixels_per_point: f32,
    width: u32,
    height: u32,
) -> (u32, u32, u32, u32) {
    let x0 = (rect.left() * pixels_per_point).floor().max(0.0) as u32;
    let y0 = (rect.top() * pixels_per_point).floor().max(0.0) as u32;
    let x1 = (rect.right() * pixels_per_point).ceil().min(width as f32) as u32;
    let y1 = (rect.bottom() * pixels_per_point).ceil().min(height as f32) as u32;
    assert!(
        x1 > x0 && y1 > y0,
        "pixel proof rect must intersect the rendered image ({rect:?}, image={width}x{height}, ppp={pixels_per_point})"
    );
    (x0, y0, x1, y1)
}

fn non_background_pixels_in_rect(
    image: &image::RgbaImage,
    rect: egui::Rect,
    pixels_per_point: f32,
) -> usize {
    let (x0, y0, x1, y1) = pixel_bounds(rect, pixels_per_point, image.width(), image.height());
    let mut histogram = std::collections::BTreeMap::<[u8; 4], usize>::new();
    for y in y0..y1 {
        for x in x0..x1 {
            *histogram.entry(image.get_pixel(x, y).0).or_default() += 1;
        }
    }
    let (bg, _) = histogram
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .expect("non-empty pixel proof rect");

    let mut stray = 0usize;
    for y in y0..y1 {
        for x in x0..x1 {
            let px = image.get_pixel(x, y).0;
            let delta = px[0].abs_diff(bg[0]) as u16
                + px[1].abs_diff(bg[1]) as u16
                + px[2].abs_diff(bg[2]) as u16
                + px[3].abs_diff(bg[3]) as u16;
            if delta > 12 {
                stray += 1;
            }
        }
    }
    stray
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

#[test]
fn fold_accesskit_expand_and_collapse_requests_mutate_live_panel() {
    let panel = Arc::new(CodeEditorPanel::new(RUST_FN, "rs"));
    assert!(panel.toggle_fold(0), "precondition: line-0 region folded");

    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(640.0, 320.0))
        .build_ui(move |ui| {
            panel_ui.show(ui);
        });
    harness.run_steps(2);

    let fold0 = harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some("code_editor_fold_0"))
        .expect("fold node code_editor_fold_0 present while folded");
    assert!(
        fold0
            .accesskit_node()
            .data()
            .supports_action(egui::accesskit::Action::Expand),
        "folded region advertises Expand"
    );
    harness.event(egui::Event::AccessKitActionRequest(
        egui::accesskit::ActionRequest {
            action: egui::accesskit::Action::Expand,
            target: fold0.accesskit_node().id(),
            data: None,
        },
    ));
    harness.run_steps(2);
    assert!(
        panel
            .fold_set()
            .region_starting_at(0)
            .is_some_and(|r| !r.folded),
        "Action::Expand dispatched at code_editor_fold_0 unfolds the region"
    );

    let fold0 = harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some("code_editor_fold_0"))
        .expect("fold node code_editor_fold_0 present while unfolded");
    assert!(
        fold0
            .accesskit_node()
            .data()
            .supports_action(egui::accesskit::Action::Collapse),
        "unfolded region advertises Collapse"
    );
    harness.event(egui::Event::AccessKitActionRequest(
        egui::accesskit::ActionRequest {
            action: egui::accesskit::Action::Collapse,
            target: fold0.accesskit_node().id(),
            data: None,
        },
    ));
    harness.run_steps(2);
    assert!(
        panel
            .fold_set()
            .region_starting_at(0)
            .is_some_and(|r| r.folded),
        "Action::Collapse dispatched at code_editor_fold_0 folds the region"
    );
}

#[test]
fn stale_fold_action_target_does_not_reuse_visible_slot_for_another_region() {
    let src = "\
fn first() {
    let a = 1;
    let b = 2;
    a + b
}






fn second() {
    let c = 3;
    let d = 4;
    c + d
}
";
    let panel = Arc::new(CodeEditorPanel::new(src, "rs"));
    let starts: Vec<usize> = panel
        .fold_set()
        .regions
        .iter()
        .map(|r| r.start_line)
        .collect();
    assert!(
        starts.len() >= 2,
        "precondition: two fold regions exist; got {starts:?}"
    );
    let first_start = starts[0];
    let second_start = starts[1];

    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(520.0, 120.0))
        .build_ui(move |ui| {
            panel_ui.show(ui);
        });
    harness.run_steps(2);

    let first_author = format!("code_editor_fold_{first_start}");
    let first_node_id = harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(first_author.as_str()))
        .expect("first fold node is visible before scrolling")
        .accesskit_node()
        .id();

    panel.scroll_to_line(second_start);
    harness.run_steps(3);
    let second_author = format!("code_editor_fold_{second_start}");
    assert!(
        harness
            .root()
            .children_recursive()
            .any(|node| node.accesskit_node().author_id() == Some(second_author.as_str())),
        "second fold node is visible after scrolling"
    );

    harness.event(egui::Event::AccessKitActionRequest(
        egui::accesskit::ActionRequest {
            action: egui::accesskit::Action::Collapse,
            target: first_node_id,
            data: None,
        },
    ));
    harness.run_steps(2);

    let fold_set = panel.fold_set();
    assert!(
        fold_set
            .region_starting_at(first_start)
            .is_some_and(|r| !r.folded),
        "stale target for the offscreen first fold is ignored once that node is no longer emitted"
    );
    assert!(
        fold_set
            .region_starting_at(second_start)
            .is_some_and(|r| !r.folded),
        "stale slot-0 target must not collapse the currently visible second fold"
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

    assert!(
        harness.query_by_label(&folded_label).is_some(),
        "AC-004: the live tree exposes the folded summary label {folded_label:?}"
    );
    for hidden_body_line in [
        "    let mut total = 0;",
        "        if *item > 0 {",
        "    scaled",
        "}",
    ] {
        assert!(
            harness.query_by_label(hidden_body_line).is_none(),
            "AC-004: hidden folded body line must be absent from the live tree: {hidden_body_line:?}"
        );
    }

    // Render the screenshot and pixel-assert the collapsed body's next row is background-only. This is
    // the MT-005-local proof that hidden braces, bracket coloring, and indent guides do not leak below
    // the folded summary row.
    let image = harness
        .render()
        .expect("PT-004 requires a rendered folded screenshot on this host");
    let (w, h) = (image.width(), image.height());
    assert!(w > 0 && h > 0, "rendered folded image is non-empty");
    let label_rect = harness.get_by_label(&folded_label).rect();
    let collapsed_band = egui::Rect::from_min_max(
        egui::pos2(label_rect.left(), label_rect.bottom() + 3.0),
        egui::pos2(label_rect.left() + 220.0, label_rect.bottom() + 14.0),
    );
    let stray_pixels =
        non_background_pixels_in_rect(&image, collapsed_band, harness.ctx.pixels_per_point());
    assert_eq!(
        stray_pixels, 0,
        "AC-004 pixel proof: the row below the folded label must be background-only; stray_pixels={stray_pixels}"
    );

    let ext_dir = external_artifact_dir("wp-kernel-012-mt-005");
    let _ = std::fs::create_dir_all(&ext_dir);
    let png_path = ext_dir.join("MT-005-folded.png");
    let saved = image.save(&png_path).is_ok();
    let abs = std::fs::canonicalize(&png_path).unwrap_or(png_path.clone());
    println!(
        "PT-004 folded screenshot: {w}x{h}, folded_painted={} (<{} unfolded, <{} buffer), \
         label={folded_label:?}, stray_pixels={stray_pixels}, saved={saved} ({})",
        folded_stats.frame_lines_rendered,
        unfolded_painted,
        buffer_lines,
        abs.display()
    );
    assert!(
        saved,
        "PT-004: the folded screenshot PNG saved to the external artifact root"
    );

    assert_no_local_test_output();
}
