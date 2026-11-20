use super::*;
use crate::rich_editor::document_model::{
    history::UndoManager,
    node::{HsLinkNode, NodeKind},
    transform::{apply_transaction_with_trace, ActorKind},
};
use serde_json::json;

fn paragraph(text: &str, id: &str) -> BlockNode {
    let mut node = BlockNode::paragraph(text);
    node.attrs.insert("block_id".into(), json!(id));
    node
}

fn assert_undo_redo(before: &BlockNode, edited: &mut BlockNode, receipt: TransactionReceipt) {
    let expected = edited.clone();
    assert_eq!(receipt.actor_kind, ActorKind::Agent);
    assert_eq!(receipt.actor_id, "identity-author");
    let mut history = UndoManager::new();
    history.push(receipt);
    assert_eq!(history.len(), 1);
    assert!(history.undo(edited).unwrap());
    assert_eq!(edited, before);
    assert!(history.redo(edited).unwrap());
    assert_eq!(*edited, expected);
}

fn apply(before: &BlockNode, steps: Vec<Step>) -> (BlockNode, TracedTransactionReceipt) {
    let mut edited = before.clone();
    let receipt = apply_transaction_with_trace(&mut edited,
        Transaction::new(steps, ActorKind::Agent, "identity-author")).unwrap();
    (edited, receipt)
}

#[test]
fn copied_nested_blocks_get_fresh_ids_without_changing_targets_or_source() {
    let mut body = paragraph("body", "body-id");
    body.attrs.insert("target_block_id".into(), json!("target-kept"));
    body.children.push(Child::HsLink(HsLinkNode::new("note", "external-note", "link")));
    let mut quote = BlockNode::with_children(NodeKind::Blockquote, vec![Child::Block(body)]);
    quote.attrs.insert("block_id".into(), json!("quote-id"));
    let template = BlockNode::doc(vec![quote]);
    let source = template.clone();
    let destination = BlockNode::doc(vec![paragraph("destination", "occupied")]);
    let mut values = ["occupied", "quote-id", "copy-quote", "copy-body"].into_iter();
    let copy = copy_with_generator(&template, &destination, &mut || values.next().unwrap().into()).unwrap();
    let quote = copy.children[0].as_block().unwrap();
    let body = quote.children[0].as_block().unwrap();
    assert_eq!(block_id(quote).unwrap(), Some("copy-quote"));
    assert_eq!(block_id(body).unwrap(), Some("copy-body"));
    assert_eq!(body.attrs["target_block_id"], json!("target-kept"));
    assert_eq!(body.children[1], source.children[0].as_block().unwrap().children[0].as_block().unwrap().children[1]);
    assert_eq!(template, source);
}

#[test]
fn inserted_duplicate_before_original_keeps_mapped_original_identity() {
    let before = BlockNode::doc(vec![paragraph("original", "keep-id")]);
    let (mut edited, receipt) = apply(&before, vec![Step::InsertNode {
        parent_path: vec![], index: 0, node: paragraph("copy", "keep-id"),
    }]);
    let receipt = finish_with_generator(&before, &mut edited, receipt, &mut || "copy-id".into()).unwrap();
    assert_eq!(block_id(edited.children[0].as_block().unwrap()).unwrap(), Some("copy-id"));
    assert_eq!(block_id(edited.children[1].as_block().unwrap()).unwrap(), Some("keep-id"));
    assert_undo_redo(&before, &mut edited, receipt);
}

#[test]
fn missing_nested_ids_are_part_of_the_same_attributed_undo_entry() {
    let before = BlockNode::doc(vec![BlockNode::with_children(NodeKind::Blockquote,
        vec![Child::Block(BlockNode::paragraph("a"))])]);
    let (mut edited, receipt) = apply(&before, vec![Step::InsertText {
        path: vec![0, 0, 0], char_offset: 1, text: "b".into(),
    }]);
    let mut values = ["new-quote", "new-body"].into_iter();
    let receipt = finish_with_generator(&before, &mut edited, receipt, &mut || values.next().unwrap().into()).unwrap();
    let quote = edited.children[0].as_block().unwrap();
    assert_eq!(block_id(quote).unwrap(), Some("new-quote"));
    assert_eq!(block_id(quote.children[0].as_block().unwrap()).unwrap(), Some("new-body"));
    assert_undo_redo(&before, &mut edited, receipt);
}

#[test]
fn deletion_then_insertion_maps_nested_original_over_an_earlier_duplicate() {
    let mut quote = BlockNode::with_children(NodeKind::Blockquote, vec![
        Child::Block(paragraph("remove", "remove-id")),
        Child::Block(paragraph("keep", "keep-id")),
    ]);
    quote.attrs.insert("block_id".into(), json!("quote-id"));
    let before = BlockNode::doc(vec![quote]);
    let (mut edited, receipt) = apply(&before, vec![
        Step::DeleteNode { parent_path: vec![0], index: 0 },
        Step::InsertNode { parent_path: vec![0], index: 0, node: paragraph("copy", "keep-id") },
    ]);
    let receipt = finish_with_generator(&before, &mut edited, receipt, &mut || "copy-id".into()).unwrap();
    let quote = edited.children[0].as_block().unwrap();
    assert_eq!(block_id(quote.children[0].as_block().unwrap()).unwrap(), Some("copy-id"));
    assert_eq!(block_id(quote.children[1].as_block().unwrap()).unwrap(), Some("keep-id"));
    assert_undo_redo(&before, &mut edited, receipt);
}

#[test]
fn split_preserves_head_identity_and_rekeys_tail_in_one_undo() {
    let before = BlockNode::doc(vec![paragraph("abcd", "head-id")]);
    let (mut edited, receipt) = apply(&before, vec![Step::SplitNode { path: vec![0], char_offset: 2 }]);
    let receipt = finish_with_generator(&before, &mut edited, receipt, &mut || "tail-id".into()).unwrap();
    assert_eq!(block_id(edited.children[0].as_block().unwrap()).unwrap(), Some("head-id"));
    assert_eq!(block_id(edited.children[1].as_block().unwrap()).unwrap(), Some("tail-id"));
    assert_undo_redo(&before, &mut edited, receipt);
}

#[test]
fn merge_undo_restores_both_original_block_identities() {
    let before = BlockNode::doc(vec![paragraph("ab", "head-id"), paragraph("cd", "tail-id")]);
    let (mut edited, receipt) = apply(&before, vec![Step::MergeNodes { parent_path: vec![], index: 1 }]);
    let receipt = finish_with_generator(&before, &mut edited, receipt, &mut || panic!("no ID allocation expected")).unwrap();
    assert_eq!(block_id(edited.children[0].as_block().unwrap()).unwrap(), Some("head-id"));
    assert_undo_redo(&before, &mut edited, receipt);
}

#[test]
fn exhausted_id_allocation_does_not_mutate_the_prepared_edit() {
    let before = BlockNode::doc(vec![paragraph("original", "occupied")]);
    let (mut edited, receipt) = apply(&before, vec![Step::InsertNode {
        parent_path: vec![], index: 0, node: BlockNode::paragraph("new"),
    }]);
    let expected = edited.clone();
    assert!(finish_with_generator(&before, &mut edited, receipt, &mut || "occupied".into()).is_err());
    assert_eq!(edited, expected);
}

#[test]
fn empty_transaction_preserves_missing_ids_without_a_new_revision() {
    let before = BlockNode::doc(vec![BlockNode::paragraph("")]);
    let (mut edited, receipt) = apply(&before, vec![]);
    let receipt = finish_with_generator(&before, &mut edited, receipt, &mut || panic!("no ID allocation expected")).unwrap();
    assert_eq!(edited, before);
    assert!(receipt.forward.is_empty());
    assert!(receipt.inverse.is_empty());
}


#[test]
fn merged_container_keeps_moved_child_identity_over_an_earlier_copy() {
    for temporary_inline_child in [false, true] {
        let mut left = BlockNode::with_children(NodeKind::Blockquote,
            vec![Child::Block(paragraph("left", "left-child"))]);
        left.attrs.insert("block_id".into(), json!("left-container"));
        let mut right = BlockNode::with_children(NodeKind::Blockquote,
            vec![Child::Block(paragraph("original", "moved-child"))]);
        right.attrs.insert("block_id".into(), json!("right-container"));
        let before = BlockNode::doc(vec![left, right]);
        let mut steps = Vec::new();
        if temporary_inline_child {
            // The model validates the complete batch, not every intermediate tree.
            steps.push(Step::InsertInlineChild { parent_path: vec![], index: 0,
                child: Child::Text(crate::rich_editor::document_model::node::TextLeaf::new("temporary")) });
            steps.push(Step::DeleteInlineChild { parent_path: vec![], index: 0 });
        }
        steps.extend([
            Step::MergeNodes { parent_path: vec![], index: 1 },
            Step::InsertNode { parent_path: vec![0], index: 0,
                node: paragraph("copy", "moved-child") },
        ]);
        let (mut edited, receipt) = apply(&before, steps);
        let receipt = finish_with_generator(&before, &mut edited, receipt,
            &mut || "fresh-copy".into()).unwrap();
        let merged = edited.children[0].as_block().unwrap();
        assert_eq!(block_id(merged).unwrap(), Some("left-container"));
        assert_eq!(block_id(merged.children[0].as_block().unwrap()).unwrap(), Some("fresh-copy"));
        assert_eq!(block_id(merged.children[1].as_block().unwrap()).unwrap(), Some("left-child"));
        assert_eq!(block_id(merged.children[2].as_block().unwrap()).unwrap(), Some("moved-child"));
        assert_undo_redo(&before, &mut edited, receipt);
    }
}


#[test]
fn traced_template_split_preserves_attributes_targets_and_one_undo() {
    use crate::rich_editor::document_model::{DocPosition, Selection, Mark, TextLeaf};
    let mut original_block = paragraph("abcd", "original-id");
    original_block.attrs.insert("custom".into(), json!({"retain":true}));
    let before = BlockNode::doc(vec![original_block]);
    let mut title = BlockNode::heading(2, "Heading");
    title.attrs.insert("block_id".into(), json!("template-heading"));
    let mut body = paragraph("", "template-body");
    body.children = vec![Child::Text(TextLeaf::with_marks("rich", vec![Mark::Bold])),
        Child::HsLink(HsLinkNode::new("note", "external-target", "Original label"))];
    body.attrs.insert("target_block_id".into(), json!("unchanged-reference-target"));
    let template = BlockNode::doc(vec![title, body]);
    let source = template.clone();
    let mut edited = before.clone();
    let result = fragment::replace_template(&mut edited,
        &Selection::caret(DocPosition::new(vec![0, 0], 2)), &template,
        ActorKind::Agent, "identity-author".into()).unwrap();
    let blocks: Vec<_> = edited.children.iter().map(|child| child.as_block().unwrap()).collect();
    assert_eq!(blocks.len(), 4);
    assert_eq!(block_id(blocks[0]).unwrap(), Some("original-id"));
    assert_ne!(block_id(blocks[1]).unwrap(), Some("template-heading"));
    assert_ne!(block_id(blocks[2]).unwrap(), Some("template-body"));
    assert_ne!(block_id(blocks[3]).unwrap(), Some("original-id"));
    assert_eq!(blocks[0].attrs["custom"], blocks[3].attrs["custom"]);
    assert_eq!(blocks[2].children, source.children[1].as_block().unwrap().children);
    assert_eq!(blocks[2].attrs["target_block_id"], json!("unchanged-reference-target"));
    let ids: HashSet<_> = blocks.iter().map(|block| block_id(block).unwrap().unwrap()).collect();
    assert_eq!(ids.len(), 4);
    assert_eq!(template, source);
    assert_undo_redo(&before, &mut edited, result.receipt);
}

#[test]
fn traced_fragment_identity_keeps_untouched_original_over_inserted_duplicate() {
    use crate::rich_editor::document_model::{FragmentReplaceOptions, RichFragment, Selection};
    let before = BlockNode::doc(vec![paragraph("replace", "replace"), paragraph("keep", "same-id")]);
    let mut edited = before.clone();
    let result = fragment::replace_fragment(&mut edited, &Selection::node(vec![0]),
        RichFragment::Blocks(vec![paragraph("copy", "same-id")]), FragmentReplaceOptions {
            actor_kind: ActorKind::Agent, actor_id: "identity-author".into(), split_tail_attrs: None,
        }).unwrap();
    assert_ne!(block_id(edited.children[0].as_block().unwrap()).unwrap(), Some("same-id"));
    assert_eq!(block_id(edited.children[1].as_block().unwrap()).unwrap(), Some("same-id"));
    assert_undo_redo(&before, &mut edited, result.receipt);
}

#[test]
fn empty_inline_fragment_keeps_missing_identity_and_empty_receipt() {
    use crate::rich_editor::document_model::{DocPosition, FragmentReplaceOptions, RichFragment, Selection};
    let before = BlockNode::doc(vec![BlockNode::new(NodeKind::Paragraph)]);
    let selection = Selection::caret(DocPosition::new(vec![0, 0], 0));
    let mut edited = before.clone();
    let result = fragment::replace_fragment(&mut edited, &selection, RichFragment::Inline(vec![]),
        FragmentReplaceOptions { actor_kind: ActorKind::Agent, actor_id: "identity-author".into(), split_tail_attrs: None }).unwrap();
    assert_eq!(edited, before);
    assert_eq!(result.selection, selection);
    assert!(result.receipt.forward.is_empty());
    assert!(result.receipt.inverse.is_empty());
}

#[test]
fn identity_failure_after_fragment_application_rolls_back_the_whole_prepared_edit() {
    use crate::rich_editor::document_model::{DocPosition, FragmentReplaceOptions, RichFragment, Selection, TextLeaf};
    let mut original = BlockNode::paragraph("keep");
    original.attrs.insert("block_id".into(), json!(42));
    let before = BlockNode::doc(vec![original]);
    let mut edited = before.clone();
    let result = fragment::replace_fragment(&mut edited,
        &Selection::caret(DocPosition::new(vec![0, 0], 2)), RichFragment::Inline(vec![Child::Text(TextLeaf::new("new"))]),
        FragmentReplaceOptions { actor_kind: ActorKind::Agent, actor_id: "identity-author".into(), split_tail_attrs: None });
    assert!(result.is_err());
    assert_eq!(edited, before);
}
