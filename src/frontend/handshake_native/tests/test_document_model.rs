//! Integration tests for the WP-KERNEL-012 MT-011 block document model.
//!
//! These exercise the PUBLIC API of `handshake_native::rich_editor::document_model`
//! end-to-end (the same surface the renderer MT-012 and the backend bridge MT-037
//! consume), proving each MT-011 acceptance criterion + every red-team minimum
//! control with real runtime assertions (no tautological tests):
//!
//! - AC: all node kinds + all mark kinds compile/serialize (`all_node_and_mark_kinds_*`)
//! - AC: InsertText produces the correct rope state (`insert_text_char_level`)
//! - AC: AddMark then RemoveMark round-trips to zero marks (`add_then_remove_mark`)
//! - AC: schema violation returns Err + leaves the doc UNCHANGED (`schema_violation_rolls_back`)
//! - AC: 3 transactions, undo 3x restores original by DocJson equality (`undo_three_restores`)
//! - AC: UndoManager caps at 200 — 201 pushes drop the oldest (`undo_cap_at_201`)
//! - AC: DocJson multi-paragraph bold/italic round-trip equals original (`docjson_round_trip`)
//! - AC: DocJson shape matches Tiptap JSONContent (`docjson_tiptap_shape`)
//! - AC: schema_version == "rich_document_v1" (`schema_version_literal`)
//! - RISK-1 control: a 4-byte emoji inserted at char position 1 keeps char-count
//!   correctness (`emoji_char_index_is_not_byte_index`)
//! - RISK-4 control: undo of InsertText reproduces the exact pre-insert rope
//!   (covered by `undo_three_restores` + `insert_text_char_level`)

use handshake_native::rich_editor::document_model::{
    absolute_offset, apply_transaction, from_json_string, resolve, to_json_string,
    to_rich_document, ActorKind, BlockNode, Child, DocPosition, HeadingLevel, Mark, NodeKind,
    Selection, Step, TextLeaf, Transaction, TransformError, UndoManager,
    DEFAULT_HISTORY_CAP, RICH_DOCUMENT_SCHEMA_VERSION,
};
use serde_json::Value as JsonValue;

/// doc > [ heading(1,"Title"), paragraph with a bold run + an italic run ].
fn multi_paragraph_doc() -> BlockNode {
    let mut para = BlockNode::new(NodeKind::Paragraph);
    para.children
        .push(Child::Text(TextLeaf::with_marks("Bold ", vec![Mark::Bold])));
    para.children
        .push(Child::Text(TextLeaf::with_marks("italic", vec![Mark::Italic])));
    BlockNode::doc(vec![BlockNode::heading(1, "Title"), para])
}

#[test]
fn all_node_and_mark_kinds_compile_and_serialize() {
    // AC: every node kind + every mark kind exists and serializes to its Tiptap type.
    let node_kinds = [
        (NodeKind::Doc, "doc"),
        (NodeKind::Paragraph, "paragraph"),
        (NodeKind::Heading(HeadingLevel::new(1)), "heading"),
        (NodeKind::Heading(HeadingLevel::new(2)), "heading"),
        (NodeKind::Heading(HeadingLevel::new(3)), "heading"),
        (NodeKind::Blockquote, "blockquote"),
        (NodeKind::CodeBlock, "codeBlock"),
        (NodeKind::OrderedList, "orderedList"),
        (NodeKind::BulletList, "bulletList"),
        (NodeKind::ListItem, "listItem"),
        (NodeKind::Table, "table"),
        (NodeKind::TableRow, "tableRow"),
        (NodeKind::TableCell, "tableCell"),
        (NodeKind::TaskItem, "taskItem"),
        (NodeKind::HardBreak, "hardBreak"),
        (NodeKind::HorizontalRule, "horizontalRule"),
    ];
    for (kind, expected) in node_kinds {
        assert_eq!(kind.to_json_type(), expected, "node {kind:?}");
        // from_json_type round-trips (heading level defaults to 1).
        assert!(NodeKind::from_json_type(expected, 1).is_some(), "parse {expected}");
    }

    let marks = [
        (Mark::Bold, "bold"),
        (Mark::Italic, "italic"),
        (Mark::Underline, "underline"),
        (Mark::Strike, "strike"),
        (Mark::Code, "code"),
        (Mark::Link { href: "https://x".into() }, "link"),
        (
            Mark::Wikilink { kind: "wp".into(), value: "WP-1".into(), label: None },
            "wikilink",
        ),
    ];
    for (mark, expected) in marks {
        assert_eq!(mark.json_type(), expected, "mark {mark:?}");
    }
}

#[test]
fn insert_text_char_level() {
    // AC: apply_transaction with InsertText produces the correct rope state, verified
    // by a char-level assertion.
    let mut doc = BlockNode::doc(vec![BlockNode::paragraph("hello")]);
    let tx = Transaction::operator(vec![Step::InsertText {
        path: vec![0, 0],
        char_offset: 5,
        text: " world".into(),
    }]);
    let receipt = apply_transaction(&mut doc, tx).unwrap();
    let leaf = doc.children[0].as_block().unwrap().children[0].as_text().unwrap();
    assert_eq!(leaf.text.to_string(), "hello world");
    assert_eq!(leaf.text.len_chars(), 11);

    // RISK-4: undo via the receipt's inverse reproduces the exact pre-insert rope.
    apply_transaction(&mut doc, Transaction::operator(receipt.inverse)).unwrap();
    let leaf = doc.children[0].as_block().unwrap().children[0].as_text().unwrap();
    assert_eq!(leaf.text.to_string(), "hello");
}

#[test]
fn delete_text_char_level() {
    let mut doc = BlockNode::doc(vec![BlockNode::paragraph("hello world")]);
    let tx = Transaction::operator(vec![Step::DeleteText {
        path: vec![0, 0],
        start: 5,
        end: 11,
    }]);
    apply_transaction(&mut doc, tx).unwrap();
    let leaf = doc.children[0].as_block().unwrap().children[0].as_text().unwrap();
    assert_eq!(leaf.text.to_string(), "hello");
}

#[test]
fn add_then_remove_mark_zero_marks() {
    // AC: AddMark then RemoveMark over a text range round-trips with zero marks.
    let mut doc = BlockNode::doc(vec![BlockNode::paragraph("text")]);
    apply_transaction(
        &mut doc,
        Transaction::operator(vec![Step::AddMark {
            path: vec![0, 0],
            mark: Mark::Bold,
        }]),
    )
    .unwrap();
    assert_eq!(
        doc.children[0].as_block().unwrap().children[0].as_text().unwrap().marks.len(),
        1
    );
    apply_transaction(
        &mut doc,
        Transaction::operator(vec![Step::RemoveMark {
            path: vec![0, 0],
            mark: Mark::Bold,
        }]),
    )
    .unwrap();
    assert_eq!(
        doc.children[0].as_block().unwrap().children[0].as_text().unwrap().marks.len(),
        0
    );
}

#[test]
fn schema_violation_rolls_back() {
    // AC: apply_transaction returns Err and leaves the doc UNCHANGED on a schema
    // violation (paragraph inside a paragraph's inline content).
    let mut doc = BlockNode::doc(vec![BlockNode::paragraph("hi")]);
    let before = doc.clone();
    let tx = Transaction::operator(vec![Step::InsertNode {
        parent_path: vec![0], // the paragraph (inline-content only)
        index: 1,
        node: BlockNode::paragraph("nested"),
    }]);
    let err = apply_transaction(&mut doc, tx).unwrap_err();
    assert!(matches!(err, TransformError::Schema(_)), "got {err:?}");
    assert_eq!(doc, before, "doc must be byte-for-byte unchanged after a rejected tx");
}

#[test]
fn undo_three_restores_by_docjson_equality() {
    // AC: after 3 transactions, undo 3 times restores the original, verified by
    // DocJson equality.
    let mut doc = BlockNode::doc(vec![BlockNode::paragraph("")]);
    let original_json = to_json_string(&doc).unwrap();
    let mut um = UndoManager::new();
    for chunk in ["a", "b", "c"] {
        let tx = Transaction::operator(vec![Step::InsertText {
            path: vec![0, 0],
            char_offset: usize::MAX, // append at end
            text: chunk.into(),
        }]);
        let r = apply_transaction(&mut doc, tx).unwrap();
        um.push(r);
    }
    assert_eq!(
        doc.children[0].as_block().unwrap().children[0].as_text().unwrap().text.to_string(),
        "abc"
    );
    assert!(um.undo(&mut doc).unwrap());
    assert!(um.undo(&mut doc).unwrap());
    assert!(um.undo(&mut doc).unwrap());
    let restored_json = to_json_string(&doc).unwrap();
    assert_eq!(restored_json, original_json, "3 undos must restore by DocJson equality");
    assert!(!um.can_undo());

    // Redo all three to confirm the forward path too.
    assert!(um.redo(&mut doc).unwrap());
    assert!(um.redo(&mut doc).unwrap());
    assert!(um.redo(&mut doc).unwrap());
    assert_eq!(
        doc.children[0].as_block().unwrap().children[0].as_text().unwrap().text.to_string(),
        "abc"
    );
}

#[test]
fn undo_cap_at_201_drops_oldest() {
    // AC + RISK-6 control: pushing 201 transactions caps history at exactly 200.
    let mut doc = BlockNode::doc(vec![BlockNode::paragraph("")]);
    let mut um = UndoManager::with_cap(DEFAULT_HISTORY_CAP);
    for _ in 0..201 {
        let tx = Transaction::operator(vec![Step::InsertText {
            path: vec![0, 0],
            char_offset: usize::MAX,
            text: "z".into(),
        }]);
        let r = apply_transaction(&mut doc, tx).unwrap();
        um.push(r);
    }
    assert_eq!(um.len(), 200, "history must be capped at 200 after 201 pushes");
}

#[test]
fn insert_and_delete_block_node() {
    // AC: insert/delete a block node by path.
    let mut doc = BlockNode::doc(vec![BlockNode::paragraph("first")]);
    apply_transaction(
        &mut doc,
        Transaction::operator(vec![Step::InsertNode {
            parent_path: vec![], // the doc root
            index: 1,
            node: BlockNode::paragraph("second"),
        }]),
    )
    .unwrap();
    assert_eq!(doc.children.len(), 2);
    apply_transaction(
        &mut doc,
        Transaction::operator(vec![Step::DeleteNode {
            parent_path: vec![],
            index: 0,
        }]),
    )
    .unwrap();
    assert_eq!(doc.children.len(), 1);
    assert_eq!(
        doc.children[0].as_block().unwrap().children[0].as_text().unwrap().text.to_string(),
        "second"
    );
}

#[test]
fn split_then_merge_paragraphs() {
    // AC: split a paragraph, then merge two paragraphs.
    let mut doc = BlockNode::doc(vec![BlockNode::paragraph("helloworld")]);
    apply_transaction(
        &mut doc,
        Transaction::operator(vec![Step::SplitNode { path: vec![0], char_offset: 5 }]),
    )
    .unwrap();
    assert_eq!(doc.children.len(), 2);
    assert_eq!(
        doc.children[0].as_block().unwrap().children[0].as_text().unwrap().text.to_string(),
        "hello"
    );
    assert_eq!(
        doc.children[1].as_block().unwrap().children[0].as_text().unwrap().text.to_string(),
        "world"
    );
    // Merge them back.
    apply_transaction(
        &mut doc,
        Transaction::operator(vec![Step::MergeNodes { parent_path: vec![], index: 1 }]),
    )
    .unwrap();
    assert_eq!(doc.children.len(), 1);
    assert_eq!(
        doc.children[0].as_block().unwrap().children[0].as_text().unwrap().text.to_string(),
        "helloworld"
    );
}

#[test]
fn docjson_round_trip_equals_original() {
    // AC: serialize a multi-paragraph doc with bold/italic to JSON and deserialize
    // back; the result equals the original doc.
    let doc = multi_paragraph_doc();
    let json = to_json_string(&doc).unwrap();
    let back = from_json_string(&json).unwrap();
    assert_eq!(doc, back);
}

#[test]
fn docjson_tiptap_shape() {
    // AC: DocJson output matches the Tiptap JSONContent shape (type field = node kind
    // string, content array, text field on text nodes, marks array with type field).
    let doc = multi_paragraph_doc();
    let v: JsonValue = serde_json::to_value(to_rich_document(&doc)).unwrap();
    assert_eq!(v["type"], "doc");
    assert!(v["content"].is_array());
    assert_eq!(v["content"][0]["type"], "heading");
    assert_eq!(v["content"][0]["attrs"]["level"], 1);
    assert_eq!(v["content"][0]["content"][0]["type"], "text");
    assert_eq!(v["content"][0]["content"][0]["text"], "Title");
    assert_eq!(v["content"][1]["content"][0]["text"], "Bold ");
    assert_eq!(v["content"][1]["content"][0]["marks"][0]["type"], "bold");
    assert_eq!(v["content"][1]["content"][1]["marks"][0]["type"], "italic");
}

#[test]
fn schema_version_literal() {
    // AC: schema_version in DocJson output equals 'rich_document_v1'.
    assert_eq!(RICH_DOCUMENT_SCHEMA_VERSION, "rich_document_v1");
    let v: JsonValue = serde_json::to_value(to_rich_document(&multi_paragraph_doc())).unwrap();
    assert_eq!(v["schema_version"], "rich_document_v1");
}

#[test]
fn emoji_char_index_is_not_byte_index() {
    // RISK-1 mandatory control: insert a 4-byte emoji at CHAR position 1 and verify
    // char-count correctness. "a😀b" is 3 chars but 6 bytes (😀 = U+1F600 = 4 bytes).
    let mut doc = BlockNode::doc(vec![BlockNode::paragraph("ab")]);
    apply_transaction(
        &mut doc,
        Transaction::operator(vec![Step::InsertText {
            path: vec![0, 0],
            char_offset: 1, // CHAR index 1 (between 'a' and 'b'), NOT byte index
            text: "😀".into(),
        }]),
    )
    .unwrap();
    let leaf = doc.children[0].as_block().unwrap().children[0].as_text().unwrap();
    assert_eq!(leaf.text.to_string(), "a😀b");
    assert_eq!(leaf.text.len_chars(), 3, "char count must be 3, not the 6-byte length");
    // The emoji is addressable as a single char at char index 1.
    assert_eq!(leaf.text.char_at(1), Some('😀'));
    // Deleting char range [1,2) removes exactly the emoji (one char, four bytes).
    apply_transaction(
        &mut doc,
        Transaction::operator(vec![Step::DeleteText {
            path: vec![0, 0],
            start: 1,
            end: 2,
        }]),
    )
    .unwrap();
    let leaf = doc.children[0].as_block().unwrap().children[0].as_text().unwrap();
    assert_eq!(leaf.text.to_string(), "ab");
}

#[test]
fn position_resolve_round_trips_across_emoji() {
    // Positions must use char offsets too: an emoji is one char-unit in the flat
    // absolute offset, so resolve/absolute_offset round-trip across it.
    let doc = BlockNode::doc(vec![BlockNode::paragraph("a😀b"), BlockNode::paragraph("cd")]);
    // total chars: 3 + 2 = 5.
    for abs in 0..=5 {
        let pos = resolve(&doc, abs).unwrap();
        assert_eq!(absolute_offset(&doc, &pos), abs, "offset {abs}");
    }
    // Offset 2 is just after the emoji in the first paragraph.
    let pos = resolve(&doc, 2).unwrap();
    assert_eq!(pos, DocPosition::new(vec![0, 0], 2));
}

#[test]
fn selection_caret_is_collapsed() {
    let pos = DocPosition::new(vec![0, 0], 3);
    let sel = Selection::caret(pos);
    assert!(sel.is_collapsed());
}

#[test]
fn link_and_wikilink_marks_round_trip_via_docjson() {
    // RISK-3 control: link href + wikilink payload survive the DocJson round-trip.
    let mut para = BlockNode::new(NodeKind::Paragraph);
    para.children.push(Child::Text(TextLeaf::with_marks(
        "site",
        vec![Mark::Link { href: "https://handshake.dev".into() }],
    )));
    para.children.push(Child::Text(TextLeaf::with_marks(
        "ref",
        vec![Mark::Wikilink {
            kind: "wp".into(),
            value: "WP-KERNEL-012".into(),
            label: Some("the WP".into()),
        }],
    )));
    let doc = BlockNode::doc(vec![para]);
    let json = to_json_string(&doc).unwrap();
    let back = from_json_string(&json).unwrap();
    assert_eq!(doc, back);
}

#[test]
fn agent_actor_attribution_threads_through_receipt() {
    // HBR-SWARM: an agent-authored transaction carries its actor kind/id onto the
    // receipt so attribution survives for the event ledger (consumed in a later MT).
    let mut doc = BlockNode::doc(vec![BlockNode::paragraph("")]);
    let tx = Transaction::new(
        vec![Step::InsertText { path: vec![0, 0], char_offset: 0, text: "x".into() }],
        ActorKind::Agent,
        "agent-author-id-7",
    );
    let receipt = apply_transaction(&mut doc, tx).unwrap();
    assert_eq!(receipt.actor_kind, ActorKind::Agent);
    assert_eq!(receipt.actor_id, "agent-author-id-7");
}
