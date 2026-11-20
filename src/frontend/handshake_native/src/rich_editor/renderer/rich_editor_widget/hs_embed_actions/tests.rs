use super::*;
use crate::rich_editor::hs_embeds::{new_block,Reference};
use crate::rich_editor::block_identity::references::Target;
use crate::rich_editor::document_model::ReferenceTargetKind;
use serde_json::json;

fn fixture() -> (RichEditorState, BlockNode) {
    let mut embed = new_block("source", &Target {block_id:"target".into(),kind:ReferenceTargetKind::Block},"Alias").unwrap();
    embed.attrs.insert("block_id".into(),json!("occurrence"));
    let mut state = RichEditorState::new(BlockNode::doc(vec![BlockNode::paragraph("before"),embed.clone(),BlockNode::paragraph("after")]));
    let endpoint = "http://127.0.0.1:9911";
    let account = crate::local_account::mock_account_context(endpoint);
    state.bind_authenticated_context(Some(account.clone()),endpoint);
    state.document_owner().bind_context("workspace","destination",endpoint,None,Some(account)).unwrap();
    (state,embed)
}

#[test]
fn open_keeps_exact_source_context_and_remove_is_one_undoable_edit() {
    let (mut state,embed) = fixture();
    let before = state.document_snapshot().unwrap();
    let context = state.document_owner().current_context().unwrap();
    state.commit_hs_embed_action(Action::Open(Reference::from_block(&embed).unwrap()),Some(context),true).unwrap();
    assert!(matches!(&state.pending_events[0],crate::rich_editor::wikilinks::inline_view::EditorEvent::NoteEmbedActivated {
        embed:wire,context:captured } if *captured==context && wire["content"][0]["attrs"]["block_id"]=="occurrence"));
    assert_eq!(state.document_snapshot().unwrap(),before);
    assert!(state.history_snapshot().unwrap().is_empty());
    state.commit_hs_embed_action(Action::Remove(embed),Some(context),false).unwrap();
    let after = state.document_snapshot().unwrap();
    assert_eq!(after.children.len(),2);
    assert_eq!(state.selection,Selection::node(vec![1]));
    assert_eq!(state.pending_bus_undo.len(),1);
    assert_eq!(state.history_snapshot().unwrap().len(),1);
    state.edit_document(|document,history|history.undo(document)).unwrap();
    assert_eq!(state.document_snapshot().unwrap(),before);
    state.edit_document(|document,history|history.redo(document)).unwrap();
    assert_eq!(state.document_snapshot().unwrap(),after);
}

#[test]
fn readonly_ime_stale_owner_revoked_account_and_changed_occurrence_do_not_remove_or_navigate() {
    for change in ["readonly","ime","context","owner","revoked","occurrence"] {
        let (mut state,mut embed) = fixture();
        let context = state.document_owner().current_context().unwrap();
        match change {
            "ime" => state.preedit.anchor=Some(DocPosition::new(vec![0,0],0)),
            "context" => {state.document_owner().advance_context().unwrap();},
            "owner" => state.open_document_owner(BlockNode::doc(vec![BlockNode::paragraph("replacement")])),
            "revoked" => state.authenticated_context.as_ref().unwrap().invalidate(),
            "occurrence" => {embed.attrs.insert("refValue".into(),json!("different"));},
            _ => (),
        }
        let before=state.document_snapshot().ok();
        assert!(state.commit_hs_embed_action(Action::Remove(embed.clone()),Some(context),change=="readonly").is_err(),"{change}");
        if change!="readonly" {
            assert!(state.commit_hs_embed_action(Action::Open(Reference::from_block(&embed).unwrap()),Some(context),false).is_err(),"{change}");
        }
        assert_eq!(state.document_snapshot().ok(),before);
        assert!(state.pending_bus_undo.is_empty());
        assert!(state.pending_events.is_empty());
    }
}

#[test]
fn malformed_target_can_be_removed_by_its_exact_occurrence_without_preview_or_navigation() {
    let (mut state,mut embed) = fixture();
    embed.attrs.remove("target_block_id");
    let owner=state.document_owner();
    let mut candidate=owner.prepare_edit().unwrap();
    candidate.parts_mut().0.children[1]=Child::Block(embed.clone());
    owner.commit_edit(candidate).unwrap();
    assert!(Reference::from_block(&embed).is_err());
    let context=owner.current_context().unwrap();
    state.commit_hs_embed_action(Action::Remove(embed),Some(context),false).unwrap();
    assert_eq!(state.document_snapshot().unwrap().children.len(),2);
    assert!(state.pending_events.is_empty());
    assert_eq!(state.pending_bus_undo.len(),1);
}
