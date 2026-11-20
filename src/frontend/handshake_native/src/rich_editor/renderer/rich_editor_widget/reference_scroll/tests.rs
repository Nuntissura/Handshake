use super::*;
use crate::rich_editor::document_model::{BlockNode, Child, NodeKind, Selection, ReferenceTargetKind};
use crate::rich_editor::renderer::rich_editor_widget::RichEditorWidget;
use crate::rich_editor::block_identity::references::Target;
use crate::theme::HsTheme;

fn document() -> BlockNode {
    let children = (0..32).map(|index| {
        let mut child = BlockNode::paragraph(if index == 31 { "late-target" } else { "earlier" });
        child.attrs.insert("block_id".into(), serde_json::json!(format!("persisted-{index}")));
        Child::Block(child)
    }).collect();
    BlockNode::doc(vec![BlockNode::with_children(NodeKind::Blockquote, children)])
}

fn frame(ctx: &egui::Context, state: &mut RichEditorState, time: f64) -> egui::FullOutput {
    ctx.run(egui::RawInput { screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(360.0,180.0))),
        time:Some(time), ..Default::default() }, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            RichEditorWidget::render_blocks(ui,state,&HsTheme::Dark.palette(),false,true);
        });
    })
}

fn target_visible(shape: &egui::Shape, clip: egui::Rect) -> bool {
    match shape {
        egui::Shape::Text(text) if text.galley.job.text == "late-target" =>
            text.pos.y >= clip.min.y - 1.0 && text.pos.y < clip.max.y,
        egui::Shape::Vec(shapes) => shapes.iter().any(|shape| target_visible(shape,clip)),
        _ => false,
    }
}

#[test]
fn mounted_reference_scroll_stays_visible_after_wrapped_paragraphs_settle() {
    let ctx=egui::Context::default();
    crate::app::HandshakeApp::install_fonts(&ctx);
    let mut blocks:Vec<_>=(0..8).map(|_|BlockNode::paragraph(&"wrapped before target ".repeat(24))).collect();
    blocks.push(document().children[0].as_block().unwrap().clone());
    blocks.extend((0..8).map(|_|BlockNode::paragraph(&"trailing content ".repeat(24))));
    let mut state=RichEditorState::new(BlockNode::doc(blocks));
    let before=state.document_snapshot().unwrap();
    let context=state.document_owner().current_context().unwrap();
    state.selection=Selection::node(vec![8,31]);
    state.request_reference_scroll(vec![8,31],context);
    for frame_index in 0..16 {
        let output=frame(&ctx,&mut state,frame_index as f64 * 0.1);
        if frame_index>=12 {
            assert!(output.shapes.iter().any(|shape|target_visible(&shape.shape,shape.clip_rect)),
                "target left the actual content clip in settled frame {frame_index}");
        }
    }
    assert_eq!(state.last_consumed_scroll_block,Some(vec![8,31]));
    assert!(state.pending_scroll_block.is_none());
    assert_eq!(state.selection,Selection::node(vec![8,31]));
    assert_eq!(state.document_snapshot().unwrap(),before);
    assert_eq!(state.document_owner().current_context().unwrap(),context);
    assert!(state.history_snapshot().unwrap().is_empty());
    assert!(state.pending_bus_undo.is_empty());
}

#[test]
fn missing_target_clears_request_with_visible_error_without_parent_scroll() {
    let ctx=egui::Context::default();
    crate::app::HandshakeApp::install_fonts(&ctx);
    let mut state=RichEditorState::new(document());
    state.request_reference_scroll(vec![0,88],state.document_owner().current_context().unwrap());
    frame(&ctx,&mut state,0.0);
    assert!(state.pending_scroll_block.is_none());
    assert!(state.last_consumed_scroll_block.is_none());
    assert!(state.interop_error.as_deref().unwrap().contains("no rendered target"));
}

#[test]
fn stale_owner_ime_or_revocation_cancels_queued_reference_scroll() {
    for change in ["revision","owner","ime","revoked"] {
        let mut state=RichEditorState::new(document());
        let endpoint="http://127.0.0.1:9911";
        let account=crate::local_account::mock_account_context(endpoint);
        state.document_owner().bind_context("workspace","note",endpoint,None,Some(account.clone())).unwrap();
        state.request_reference_scroll(vec![0,31],state.document_owner().current_context().unwrap());
        match change {
            "revision" => { state.document_owner().advance_context().unwrap(); },
            "owner" => { state.open_document_owner(document()); },
            "ime" => { state.preedit.anchor=Some(crate::rich_editor::document_model::DocPosition::new(vec![0,31,0],0)); },
            "revoked" => account.invalidate(),
            _ => unreachable!(),
        }
        assert!(!state.validate_reference_scroll(),"accepted {change}");
        assert!(state.pending_scroll_block.is_none());
        assert!(state.last_consumed_scroll_block.is_none());
        assert!(state.pending_bus_undo.is_empty());
    }
}

#[test]
fn persisted_target_after_reload_and_undo_uses_its_current_model_path() {
    use crate::rich_editor::document_model::transform::{apply_transaction,ActorKind,Step,Transaction};
    let wire=crate::rich_editor::document_model::to_content_json_value(&document());
    let mut state=RichEditorState::new(crate::rich_editor::document_model::from_json_value(&wire).unwrap());
    let target=Target { block_id:"persisted-31".into(),kind:ReferenceTargetKind::Block };
    state.edit_document(|document,history| {
        let receipt=apply_transaction(document,Transaction::new(vec![Step::InsertNode {
            parent_path:vec![0],index:0,node:BlockNode::paragraph("inserted before")
        }],ActorKind::Operator,"test"))?;
        history.push(receipt);
        Ok::<_,crate::rich_editor::document_model::transform::TransformError>(())
    }).unwrap();
    assert_eq!(target.locate(&state.document_snapshot().unwrap()).unwrap(),vec![0,32]);
    state.edit_document(|document,history|history.undo(document)).unwrap();
    let path=target.locate(&state.document_snapshot().unwrap()).unwrap();
    assert_eq!(path,vec![0,31]);
    let ctx=egui::Context::default();
    crate::app::HandshakeApp::install_fonts(&ctx);
    state.request_reference_scroll(path.clone(),state.document_owner().current_context().unwrap());
    frame(&ctx,&mut state,0.0);
    assert_eq!(state.last_consumed_scroll_block,Some(path));
}
