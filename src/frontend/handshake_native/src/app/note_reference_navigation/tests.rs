use super::*;
use crate::rich_editor::document_model::{Child, ReferenceTargetKind, selection::Selection};
use crate::rich_editor::renderer::rich_editor_widget::RichEditorState;
use serde_json::json;

fn fixture() -> (PendingReference, RichEditorState, backend_client::RichDocBody) {
    let content = json!({"type":"doc","content":[{"type":"blockquote","content":[
        {"type":"heading","attrs":{"level":2,"block_id":"nested-heading"},"content":[{"type":"text","text":"Exact target"}]}
    ]}]});
    let document = crate::rich_editor::document_model::doc_json::from_json_value(&content).unwrap();
    let target = Target { block_id:"nested-heading".into(), kind:ReferenceTargetKind::Heading };
    let link = target.link("destination", "Alias");
    let mut paragraph = BlockNode::paragraph("");
    paragraph.children = vec![Child::HsLink(link.clone())];
    let source = DocumentEditOwner::new(BlockNode::doc(vec![paragraph]));
    let endpoint = "http://127.0.0.1:9911";
    let account = crate::local_account::mock_account_context(endpoint);
    source.bind_context("workspace", "source", endpoint, None, Some(account.clone())).unwrap();
    let state = RichEditorState::new(BlockNode::doc(vec![BlockNode::paragraph("")]));
    let destination = state.document_owner();
    destination.bind_context("workspace", "destination", endpoint, None, Some(account.clone())).unwrap();
    let token = destination.begin_load("pane", uuid::Uuid::nil(), 1).unwrap();
    destination.apply_loaded_document(token, document, 4, |_, _| Ok(())).unwrap();
    let pending = PendingReference { reference:link, embed:None, target, workspace:"workspace".into(),
        endpoint:endpoint.into(), account, source_context:source.current_context().unwrap(), source,
        destination };
    let body = backend_client::RichDocBody { document_id:"destination".into(), block_id:None,
        workspace_id:"workspace".into(), doc_version:4, title:"Note".into(), content_json:content,
        crdt_document_id:None, authority_label:"AUTHORITATIVE".into(), owner_actor_kind:None,
        owner_actor_id:None, project_ref:None, folder_ref:None, created_at:"created".into(), updated_at:"four".into() };
    (pending, state, body)
}

#[test]
fn loaded_reference_selects_exact_nested_identity_without_edit_or_history() {
    let (pending, mut state, body) = fixture();
    let before = state.document_snapshot().unwrap();
    let context = state.document_owner().current_context().unwrap();
    let version = pending.validate_loaded(&body, &before).unwrap();
    pending.select_loaded(&mut state, "destination", version).unwrap();
    assert_eq!(state.selection, Selection::node(vec![0, 0]));
    assert_eq!(state.pending_scroll_block, Some(vec![0, 0]));
    assert_eq!(state.document_snapshot().unwrap(), before);
    assert_eq!(state.document_owner().current_context().unwrap(), context);
    assert!(state.history_snapshot().unwrap().is_empty());
    assert!(state.pending_bus_undo.is_empty());
}

#[test]
fn load_owner_and_missing_target_fail_before_selection() {
    let (pending, state, mut body) = fixture();
    let content = state.document_snapshot().unwrap();
    body.document_id = "wrong-owner".into();
    assert!(pending.validate_loaded(&body, &content).is_err());
    body.document_id = "destination".into();
    body.workspace_id = "wrong-workspace".into();
    assert!(pending.validate_loaded(&body, &content).is_err());
    body.workspace_id = "workspace".into();
    assert!(pending.validate_loaded(&body, &BlockNode::doc(vec![BlockNode::paragraph("Exact target")])).is_err());
}

#[test]
fn stale_source_replaced_destination_ime_version_and_revocation_preserve_selection() {
    for change in ["source", "destination", "ime", "version", "revoked"] {
        let (pending, mut state, _) = fixture();
        let before = state.selection.clone();
        match change {
            "source" => { pending.source.advance_context().unwrap(); },
            "destination" => { state.open_document_owner(BlockNode::doc(vec![BlockNode::paragraph("other")])); },
            "ime" => { state.preedit.anchor = Some(crate::rich_editor::document_model::position::DocPosition::new(vec![0,0,0],0)); },
            "revoked" => { pending.account.invalidate(); },
            _ => (),
        }
        let expected = if change == "version" { 5 } else { 4 };
        assert!(pending.select_loaded(&mut state, "destination", expected).is_err(), "accepted {change}");
        if change != "destination" { assert_eq!(state.selection, before); }
        assert!(state.pending_scroll_block.is_none());
        assert!(state.pending_bus_undo.is_empty());
    }
}

#[test]
fn embedded_reference_navigation_uses_exact_occurrence_and_persisted_destination() {
    let (mut pending,mut state,body)=fixture();
    let mut embed=crate::rich_editor::hs_embeds::new_block("destination",&pending.target,"Alias").unwrap();
    embed.attrs.insert("block_id".into(),json!("source-occurrence"));
    let mut candidate=pending.source.prepare_edit().unwrap();
    *candidate.parts_mut().0=BlockNode::doc(vec![embed.clone()]);
    pending.source.commit_edit(candidate).unwrap();
    pending.source_context=pending.source.current_context().unwrap();
    pending.embed=Some(embed.clone());
    let document=state.document_snapshot().unwrap();
    let version=pending.validate_loaded(&body,&document).unwrap();
    pending.select_loaded(&mut state,"destination",version).unwrap();
    assert_eq!(state.selection,Selection::node(vec![0,0]));
    assert_eq!(state.pending_scroll_block,Some(vec![0,0]));
    assert!(!references::contains_link(&pending.source.document_snapshot().unwrap(),&pending.reference));
    let mut changed=embed;
    changed.attrs.insert("target_block_id".into(),json!("other"));
    assert!(!pending.contains_source(&BlockNode::doc(vec![changed])));
    assert!(state.pending_bus_undo.is_empty());
}
