use super::super::{RichEditorState, RichEditorWidget};
use crate::rich_editor::document_model::{BlockNode, Child, NodeKind};
use crate::theme::HsTheme;

fn document(repeats: usize) -> BlockNode {
    let mut blocks: Vec<_> = (0..8)
        .map(|_| BlockNode::paragraph(&"offscreen words with different widths ".repeat(repeats)))
        .collect();
    blocks.push(BlockNode::with_children(
        NodeKind::Blockquote,
        vec![Child::Block(BlockNode::paragraph("layout-target"))],
    ));
    blocks.extend((0..6).map(|_| BlockNode::paragraph(&"trailing content ".repeat(20))));
    BlockNode::doc(blocks)
}

fn frame(
    ctx: &egui::Context,
    state: &mut RichEditorState,
    width: f32,
    reading: bool,
    time: f64,
    measure_all: bool,
) -> egui::FullOutput {
    if measure_all {
        state.measure_content_on_next_render();
    }
    ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(width, 180.0),
            )),
            time: Some(time),
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                RichEditorWidget::render_blocks(ui, state, &HsTheme::Dark.palette(), false, reading);
            });
        },
    )
}

fn warm_layout(ctx: &egui::Context, state: &mut RichEditorState) {
    for index in 0..6 {
        frame(ctx, state, 360.0, false, index as f64 * 0.1, false);
    }
}

fn text_origins(output: &egui::FullOutput, label: &str) -> Vec<egui::Pos2> {
    fn collect(shape: &egui::Shape, label: &str, positions: &mut Vec<egui::Pos2>) {
        match shape {
            egui::Shape::Text(text) if text.galley.job.text == label => positions.push(text.pos),
            egui::Shape::Vec(shapes) => {
                for shape in shapes {
                    collect(shape, label, positions);
                }
            }
            _ => (),
        }
    }
    let mut positions = Vec::new();
    for shape in &output.shapes {
        collect(&shape.shape, label, &mut positions);
    }
    positions
}

fn assert_matches_full_layout(
    ctx: &egui::Context,
    state: &mut RichEditorState,
    width: f32,
    reading: bool,
    label: &str,
) {
    let cached = frame(ctx, state, width, reading, 1.0, false);
    let complete = frame(ctx, state, width, reading, 1.1, true);
    let actual = text_origins(&cached, "layout-target");
    let expected = text_origins(&complete, "layout-target");
    assert_eq!(actual.len(), 1, "missing target after {label}");
    assert_eq!(expected.len(), 1, "missing full-layout target after {label}");
    assert!(
        actual[0].distance(expected[0]) < 0.5,
        "cached extents misplaced target after {label}: {actual:?} versus {expected:?}"
    );
}

#[test]
fn cached_offscreen_paragraphs_preserve_real_document_geometry() {
    let ctx = egui::Context::default();
    crate::app::HandshakeApp::install_fonts(&ctx);
    let mut state = RichEditorState::new(document(8));
    warm_layout(&ctx, &mut state);
    let cached = frame(&ctx, &mut state, 360.0, false, 0.6, false);
    let complete = frame(&ctx, &mut state, 360.0, false, 0.7, true);
    let paragraph = "offscreen words with different widths ".repeat(8);
    assert!(text_origins(&cached, &paragraph).len() < text_origins(&complete, &paragraph).len());
    assert_eq!(text_origins(&cached, "layout-target"), text_origins(&complete, "layout-target"));
}

#[test]
fn layout_changes_keep_cached_and_full_document_geometry_aligned() {
    for change in ["width", "font-size", "dpi", "reading", "font-family", "font-data"] {
        let ctx = egui::Context::default();
        crate::app::HandshakeApp::install_fonts(&ctx);
        let mut state = RichEditorState::new(document(8));
        warm_layout(&ctx, &mut state);
        let mut width = 360.0;
        let mut reading = false;
        match change {
            "width" => width = 210.0,
            "font-size" => { state.set_editor_font_size(25.0); }
            "dpi" => ctx.set_pixels_per_point(1.75),
            "reading" => reading = true,
            "font-family" | "font-data" => {
                let mut fonts = ctx.fonts(|fonts| fonts.definitions().clone());
                let defaults = egui::FontDefinitions::default();
                let mono = defaults.families[&egui::FontFamily::Monospace][0].clone();
                if change == "font-family" {
                    fonts.families.insert(egui::FontFamily::Proportional, vec![mono]);
                } else {
                    let proportional = fonts.families[&egui::FontFamily::Proportional][0].clone();
                    let replacement = defaults.font_data[&mono].clone();
                    fonts.font_data.insert(proportional, replacement);
                }
                ctx.set_fonts(fonts);
            }
            _ => unreachable!(),
        }
        assert_matches_full_layout(&ctx, &mut state, width, reading, change);
    }
}

#[test]
fn edits_undo_and_owner_replacement_invalidate_offscreen_extents() {
    use crate::rich_editor::document_model::transform::{apply_transaction, ActorKind, Step, Transaction};

    for change in ["text", "insert", "undo", "owner"] {
        let ctx = egui::Context::default();
        crate::app::HandshakeApp::install_fonts(&ctx);
        let mut state = RichEditorState::new(document(8));
        warm_layout(&ctx, &mut state);
        if change == "owner" {
            state.open_document_owner(document(18));
        } else {
            let step = if change == "text" {
                Step::InsertText {
                    path: vec![3, 0],
                    char_offset: 0,
                    text: "longer offscreen paragraph ".repeat(35),
                }
            } else {
                Step::InsertNode {
                    parent_path: vec![],
                    index: 3,
                    node: BlockNode::paragraph(&"inserted offscreen content ".repeat(35)),
                }
            };
            state.edit_document(|document, history| {
                let receipt = apply_transaction(document, Transaction::new(
                    vec![step], ActorKind::Operator, "cache-invalidation"))?;
                history.push(receipt);
                Ok::<_, crate::rich_editor::document_model::transform::TransformError>(())
            }).unwrap();
            if change == "undo" {
                frame(&ctx, &mut state, 360.0, false, 0.6, false);
                frame(&ctx, &mut state, 360.0, false, 0.7, false);
                state.edit_document(|document, history| history.undo(document)).unwrap();
            }
        }
        assert_matches_full_layout(&ctx, &mut state, 360.0, false, change);
    }
}
