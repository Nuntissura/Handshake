use super::*;
use crate::rich_editor::hs_embeds::{Cache,new_block};
use crate::rich_editor::block_identity::references::Target;
use crate::rich_editor::document_owner::DocumentEditOwner;
use crate::rich_editor::document_model::ReferenceTargetKind;

#[test]
fn nested_embed_uses_exact_measured_target_and_cell_clip_without_a_false_caret() {
    let mut embed=new_block("source",&Target {block_id:"target".into(),kind:ReferenceTargetKind::Block},"Alias").unwrap();
    embed.attrs.insert("block_id".into(),serde_json::json!("occurrence"));
    let cell=|children:Vec<BlockNode>|BlockNode::with_children(NodeKind::TableCell,
        children.into_iter().map(Child::Block).collect());
    let row=BlockNode::with_children(NodeKind::TableRow,vec![Child::Block(cell(vec![embed])),
        Child::Block(cell(vec![BlockNode::paragraph("neighbor")]))]);
    let table=BlockNode::with_children(NodeKind::Table,vec![Child::Block(row)]);
    let document=BlockNode::doc(vec![table.clone()]);
    let owner=DocumentEditOwner::new(document.clone());
    let endpoint="http://127.0.0.1:9911";
    let account=crate::local_account::mock_account_context(endpoint);
    owner.bind_context("workspace","destination",endpoint,None,Some(account.clone())).unwrap();
    let mut cache=Cache::default();
    let mut frame=cache.frame(&owner,Some(account),endpoint,&document,24.0);
    let ctx=egui::Context::default();
    crate::app::HandshakeApp::install_fonts(&ctx);
    let mut target=None;
    let output=ctx.run(Default::default(),|ctx| {
        let painted=paint_block_with_embeds(&ctx.layer_painter(egui::LayerId::background()),&table,
            egui::pos2(20.0,30.0),300.0,&HsPalette::dark(),Some(0),
            line_layout::bold_family_available(ctx),16.0,line_layout::AtomPaint::Visible,
            Some(&[0,0,0]),Some(&[0,0,0]),Some(&mut frame));
        assert!(painted.paint.caret_galley.is_none());
        let rect=painted.target_rect.unwrap();
        assert!(rect.height()>24.0);assert!(rect.max.x<=170.0);
        assert!(painted.paint.height>rect.height());
        target=Some(rect);
    });
    let target=target.unwrap();
    fn previews(shape:&egui::Shape,clip:Rect,out:&mut Vec<Rect>) {
        match shape {
            egui::Shape::Text(text) if text.galley.job.text=="Preview not loaded"=>out.push(clip),
            egui::Shape::Vec(shapes)=>for shape in shapes {previews(shape,clip,out)},
            _=>(),
        }
    }
    let mut clips=Vec::new();
    for shape in output.shapes {previews(&shape.shape,shape.clip_rect,&mut clips)}
    assert_eq!(clips.len(),1);
    assert!(target.contains_rect(clips[0]));
}
