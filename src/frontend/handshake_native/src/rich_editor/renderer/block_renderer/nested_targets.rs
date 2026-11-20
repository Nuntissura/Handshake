use super::*;
use crate::theme::HsTheme;

fn quote(children: Vec<BlockNode>) -> BlockNode {
    BlockNode::with_children(NodeKind::Blockquote, children.into_iter().map(Child::Block).collect())
}

fn render(block: &BlockNode, target: &[usize], caret_path: Option<&[usize]>, width: f32)
    -> (TargetedBlockPaint, Vec<(String, egui::Pos2)>) {
    let ctx = egui::Context::default();
    crate::app::HandshakeApp::install_fonts(&ctx);
    let mut painted = None;
    let output = ctx.run(Default::default(), |ctx| {
        painted = Some(paint_block_with_target(&ctx.layer_painter(egui::LayerId::background()),
            block, egui::pos2(20.0, 30.0), width, &HsTheme::Dark.palette(), caret_path.map(|_| 2),
            line_layout::bold_family_available(ctx), 16.0, line_layout::AtomPaint::ChipCovered, caret_path, Some(target)));
    });
    fn collect(shape: &egui::Shape, out: &mut Vec<(String, egui::Pos2)>) {
        match shape {
            egui::Shape::Text(text) => out.push((text.galley.job.text.clone(), text.pos)),
            egui::Shape::Vec(shapes) => for shape in shapes { collect(shape, out); },
            _ => (),
        }
    }
    let mut text = Vec::new();
    for shape in &output.shapes { collect(&shape.shape, &mut text); }
    (painted.unwrap(), text)
}

#[test]
fn every_quote_child_is_painted_and_later_target_uses_its_own_rect_and_caret() {
    let document = quote(vec![BlockNode::paragraph("first"), BlockNode::paragraph("later-target")]);
    let (painted, text) = render(&document, &[1], Some(&[1, 0]), 240.0);
    assert!(text.iter().any(|(text, _)| text == "first"));
    let later = text.iter().find(|(text, _)| text == "later-target").unwrap().1;
    let target = painted.target_rect.unwrap();
    assert_eq!(target.min, later);
    assert!(target.min.y > 30.0);
    let (galley, origin) = painted.paint.caret_galley.unwrap();
    assert_eq!(galley.job.text, "later-target");
    assert_eq!(origin, later);
    assert!(painted.paint.height > target.height());
}

#[test]
fn nested_quote_target_geometry_tracks_real_wrapped_layout_and_indent() {
    let document = quote(vec![BlockNode::paragraph("first child wraps over several lines at narrow widths"),
        quote(vec![BlockNode::paragraph("inside-first"), BlockNode::paragraph("inside-target")])]);
    let (wide, _) = render(&document, &[1, 1], None, 420.0);
    let (narrow, text) = render(&document, &[1, 1], None, 120.0);
    let target = narrow.target_rect.unwrap();
    let text_origin = text.iter().find(|(text, _)| text == "inside-target").unwrap().1;
    assert_eq!(target.min, text_origin);
    assert_eq!(target.min.x, 20.0 + 2.0 * BLOCKQUOTE_INDENT_PTS);
    assert!(target.min.y > wide.target_rect.unwrap().min.y);
    assert!(narrow.paint.height > wide.paint.height);
    assert!(narrow.paint.caret_galley.is_none());
}

#[test]
fn missing_inline_and_unpainted_targets_never_receive_the_parent_rectangle() {
    let list = BlockNode::with_children(NodeKind::BulletList, vec![Child::Block(
        BlockNode::with_children(NodeKind::ListItem, vec![Child::Block(BlockNode::paragraph("list child"))]))]);
    let table = BlockNode::with_children(NodeKind::Table, vec![Child::Block(
        BlockNode::with_children(NodeKind::TableRow, vec![Child::Block(
            BlockNode::with_children(NodeKind::TableCell, vec![Child::Block(BlockNode::paragraph("first cell child")),
                Child::Block(BlockNode::paragraph("later cell child"))]))]))]);
    let document = quote(vec![BlockNode::paragraph("visible"), list, table]);
    for path in [vec![9], vec![0,0], vec![1,0,9], vec![2,0,0,9]] {
        assert!(render(&document, &path, None, 200.0).0.target_rect.is_none(), "resolved {path:?}");
    }
    let mut embed = BlockNode::new(NodeKind::HsEmbed);
    embed.attrs.insert("refValue".into(), serde_json::json!("owning-note"));
    let document = quote(vec![embed]);
    crate::rich_editor::document_model::schema::validate_tree(&BlockNode::doc(vec![document.clone()])).unwrap();
    assert!(render(&document, &[0], None, 200.0).0.target_rect.is_none());
}

#[test]
fn later_nested_reference_labels_keep_visible_glyphs_without_a_chip_overlay() {
    let mut paragraph=BlockNode::paragraph("");
    paragraph.children=vec![Child::HsLink(HsLinkNode::new("note","owner-note","Visible link"))];
    let document=quote(vec![BlockNode::paragraph("first"),paragraph]);
    let (painted,_) = render(&document,&[1],Some(&[1,0]),220.0);
    let (galley,_) = painted.paint.caret_galley.unwrap();
    assert!(galley.job.text.contains("Visible link"));
    assert!(galley.job.sections.iter().all(|section|section.format.color.a()>0));
    assert!(painted.target_rect.is_some());
}
