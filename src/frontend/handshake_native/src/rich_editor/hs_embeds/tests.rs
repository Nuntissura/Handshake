use super::*;

pub(super) fn embed(occurrence: &str, note: &str, block: &str) -> BlockNode {
    let mut node = new_block(note, &Target { block_id: block.into(), kind: ReferenceTargetKind::Block }, "Alias").unwrap();
    node.attrs.insert("block_id".into(), json!(occurrence));
    node
}

fn response(note: &str, blocks: Vec<BlockNode>) -> serde_json::Value {
    json!({"rich_document_id":note,"workspace_id":"workspace","doc_version":7,
        "content_json":super::super::document_model::to_content_json_value(&BlockNode::doc(blocks))})
}

#[test]
fn preview_contains_only_the_exact_persisted_target_and_reports_its_source_version() {
    let occurrence = embed("occurrence", "source", "target");
    let reference = Reference::from_block(&occurrence).unwrap();
    let mut target = BlockNode::paragraph("exact body");
    target.attrs.insert("block_id".into(), json!("target"));
    let result = source_preview(&reference, "workspace", &response("source", vec![
        BlockNode::paragraph("unrelated before"), target, BlockNode::paragraph("unrelated after")]));
    let Status::Ready { text, source_version } = result else { panic!("Expected exact preview") };
    assert_eq!(text, "exact body");
    assert_eq!(source_version, 7);
    let wire = super::super::document_model::to_content_json_value(&BlockNode::doc(vec![occurrence.clone()]));
    assert_eq!(super::super::document_model::from_json_value(&wire).unwrap().children[0].as_block(), Some(&occurrence));
    assert_eq!(wire["content"][0]["type"], "hsEmbed");
    assert_ne!(occurrence.attrs["block_id"], occurrence.attrs["target_block_id"]);
}

#[test]
fn missing_wrong_owner_wrong_workspace_malformed_and_ambiguous_sources_never_preview_other_content() {
    let reference = Reference::from_block(&embed("occurrence", "source", "target")).unwrap();
    let mut target = BlockNode::paragraph("private text");
    target.attrs.insert("block_id".into(), json!("target"));
    let mut body = response("other", vec![target.clone()]);
    assert!(matches!(source_preview(&reference, "workspace", &body), Status::Denied));
    body["rich_document_id"] = json!("source");
    assert!(matches!(source_preview(&reference, "other-workspace", &body), Status::Denied));
    body["content_json"] = json!({"type":"doc","content":[{"type":"unknown"}]});
    assert!(matches!(source_preview(&reference, "workspace", &body), Status::Failed));
    assert!(matches!(source_preview(&reference, "workspace", &response("source", vec![BlockNode::paragraph("private text")])), Status::Missing));
    assert!(matches!(source_preview(&reference, "workspace", &response("source", vec![target.clone(),target])), Status::Failed));
    let mut incomplete = embed("occurrence", "source", "target");
    incomplete.attrs.remove("target_block_id");
    assert!(Reference::from_block(&incomplete).is_err());
}

#[test]
fn self_and_cyclic_targets_project_once_without_recursive_fetch_or_expansion() {
    let self_ref = embed("self", "source", "self");
    let a = embed("a", "source", "b");
    let b = embed("b", "source", "a");
    for occurrence in [&self_ref, &a, &b] {
        let reference = Reference::from_block(occurrence).unwrap();
        let result = source_preview(&reference, "workspace", &response("source", vec![self_ref.clone(),a.clone(),b.clone()]));
        assert!(matches!(result, Status::Ready { ref text, source_version: 7 } if text.is_empty()));
        assert_eq!(reference.link().ref_value, "source");
        let mut document = BlockNode::doc(vec![self_ref.clone(), a.clone(), b.clone()]);
        let (receipt, _) = remove(&mut document, occurrence, "operator").unwrap();
        assert!(!receipt.forward.is_empty());
        assert!(reference.locate_occurrence(&document).is_err());
    }
}

#[test]
fn removal_targets_one_occurrence_and_rejects_changed_payload_even_with_the_same_id() {
    let first = embed("first", "source", "target");
    let second = embed("second", "source", "target");
    let mut document = BlockNode::doc(vec![first.clone(),second.clone()]);
    remove(&mut document, &second, "operator").unwrap();
    assert_eq!(document.children[0].as_block(), Some(&first));
    assert_eq!(document.children.len(), 1);
    let mut changed = first.clone();
    changed.attrs.insert("refValue".into(), json!("other"));
    let before = document.clone();
    assert!(remove(&mut document, &changed, "operator").is_err());
    assert_eq!(document, before);
    let (receipt, selection) = remove(&mut document, &first, "operator").unwrap();
    assert_eq!(selection, super::super::document_model::selection::Selection::node(vec![0]));
    assert_eq!(document.children[0].as_block().unwrap().kind, NodeKind::Paragraph);
    assert!(document.children[0].as_block().unwrap().attrs.contains_key("block_id"));
    super::super::document_model::transform::apply_transaction(&mut document,
        super::super::document_model::transform::Transaction::new(receipt.inverse,
            super::super::document_model::ActorKind::Operator, "operator")).unwrap();
    assert_eq!(document, before);
}
