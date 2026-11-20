use super::*;
use super::super::tests::embed;
use crate::rich_editor::document_owner::DocumentEditOwner;

#[test]
fn pending_ready_missing_and_cyclic_previews_keep_stable_extents_and_clipped_controls() {
    let ctx=egui::Context::default();
    crate::app::HandshakeApp::install_fonts(&ctx);
    let node=embed("occurrence","source","target");
    let reference=Reference::from_block(&node).unwrap();
    let owner=DocumentEditOwner::new(BlockNode::doc(vec![node.clone()]));
    let context=owner.current_context().unwrap();
    let clip=egui::Rect::from_min_max(egui::pos2(30.0,35.0),egui::pos2(170.0,400.0));
    let mut heights=Vec::new();
    let _=ctx.run(Default::default(),|ctx| {
        let painter=ctx.layer_painter(egui::LayerId::background()).with_clip_rect(clip);
        for status in [Status::Idle,Status::Loading,Status::Missing,Status::Denied,Status::Failed,
            Status::Ready {text:"a long preview ".repeat(40),source_version:8},
            Status::Ready {text:String::new(),source_version:8}] {
            let ready=matches!(status,Status::Ready {..});
            let mut frame=Frame::new(HashMap::from([(reference.key.clone(),(reference.clone(),status))]),Some(context),24.0);
            heights.push(frame.paint(&painter,&node,egui::pos2(20.0,30.0),180.0,&HsPalette::dark(),16.0));
            assert_eq!(frame.visible.len(),1);
            let painted=&frame.painted[0];
            assert_eq!(painted.ready,ready);
            assert_eq!(painted.clip,painted.rect.intersect(clip));
            assert!(painted.rect.contains_rect(painted.controls));
            assert!(painted.reference.is_some(),"self/cyclic empty previews retain navigation/removal identity");
        }
    });
    assert!(heights.windows(2).all(|pair|(pair[0]-pair[1]).abs()<0.01));
}

#[test]
fn malformed_target_has_removal_only_and_offscreen_embeds_do_not_dispatch() {
    let ctx=egui::Context::default();
    crate::app::HandshakeApp::install_fonts(&ctx);
    let mut node=embed("occurrence","source","target");
    node.attrs.remove("target_kind");
    let owner=DocumentEditOwner::new(BlockNode::doc(vec![node.clone()]));
    let _=ctx.run(Default::default(),|ctx| {
        let painter=ctx.layer_painter(egui::LayerId::background()).with_clip_rect(
            egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(300.0,300.0)));
        let mut frame=Frame::new(HashMap::new(),Some(owner.current_context().unwrap()),24.0);
        frame.paint(&painter,&node,egui::pos2(10.0,10.0),200.0,&HsPalette::dark(),16.0);
        assert!(frame.visible.is_empty());
        assert_eq!(frame.painted.len(),1);
        assert!(!frame.painted[0].ready);
        assert!(frame.painted[0].reference.is_none());
        assert_eq!(frame.painted[0].node,node);
        frame.painted.clear();
        frame.paint(&painter,&node,egui::pos2(10.0,400.0),200.0,&HsPalette::dark(),16.0);
        assert!(frame.visible.is_empty());assert!(frame.painted.is_empty());
    });
}
