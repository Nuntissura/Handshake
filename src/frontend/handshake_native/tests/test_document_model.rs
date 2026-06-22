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
    absolute_offset, apply_transaction, from_json_string, resolve, to_content_json_value,
    to_json_string, to_rich_document, ActorKind, BlockNode, Child, DocPosition, HeadingLevel,
    HsLinkNode, Mark, NodeKind, Selection, Step, TextLeaf, Transaction, TransformError,
    UndoManager, DEFAULT_HISTORY_CAP, RICH_DOCUMENT_SCHEMA_VERSION,
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
    ];
    for (mark, expected) in marks {
        assert_eq!(mark.json_type(), expected, "mark {mark:?}");
    }
    // The wikilink is an inline hsLink NODE (not a mark) — it serializes to the
    // backend `hsLink` node type, matching app/src/lib/tiptap/hs_link_node.ts.
    let link = HsLinkNode::new("wp", "WP-1", "");
    let v = to_content_json_value(&BlockNode::doc(vec![{
        let mut p = BlockNode::new(NodeKind::Paragraph);
        p.children.push(Child::HsLink(link));
        p
    }]));
    assert_eq!(v["content"][0]["content"][0]["type"], "hsLink");
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
    // The wire content_json is a BARE doc node (no schema_version key inside it).
    let doc = multi_paragraph_doc();
    let v: JsonValue = to_content_json_value(&doc);
    assert_eq!(v["type"], "doc");
    assert!(v.get("schema_version").is_none(), "content_json must NOT embed schema_version");
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
    // AC: schema_version in the RECORD envelope equals 'rich_document_v1', carried as
    // a SIBLING of content_json (matching the backend RichDocument record).
    assert_eq!(RICH_DOCUMENT_SCHEMA_VERSION, "rich_document_v1");
    let v: JsonValue = serde_json::to_value(to_rich_document(&multi_paragraph_doc())).unwrap();
    assert_eq!(v["schema_version"], "rich_document_v1");
    assert_eq!(v["content_json"]["type"], "doc");
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
fn link_mark_and_hs_link_node_round_trip_via_docjson() {
    // RISK-3 control: link href survives the DocJson round-trip, and a typed wikilink
    // round-trips as an inline hsLink NODE (MUST-FIX #1: node, not mark).
    let mut para = BlockNode::new(NodeKind::Paragraph);
    para.children.push(Child::Text(TextLeaf::with_marks(
        "site",
        vec![Mark::Link { href: "https://handshake.dev".into() }],
    )));
    para.children.push(Child::HsLink(HsLinkNode::new(
        "wp",
        "WP-KERNEL-012",
        "the WP",
    )));
    let doc = BlockNode::doc(vec![para]);
    let json = to_json_string(&doc).unwrap();
    let back = from_json_string(&json).unwrap();
    assert_eq!(doc, back);
}

/// A captured REAL backend `content_json` value (a BARE ProseMirror doc node, the
/// shape `loadRichDocument` returns and `createRichDocument` POSTs). It contains a
/// MULTI-RUN paragraph (bold + plain runs) and a typed `hsLink` inline atom node —
/// exactly the custom Handshake nodes that are the actual backend-compat risk. This
/// is hand-authored to match app/src/lib/tiptap/hs_link_node.ts + export_formats.ts,
/// NOT produced by this module's own serializer (MUST-FIX #3: assert against a real
/// captured shape, not the model's own output).
const REAL_BACKEND_CONTENT_JSON: &str = r#"{
  "type": "doc",
  "content": [
    {
      "type": "heading",
      "attrs": { "level": 1 },
      "content": [ { "type": "text", "text": "Release Notes" } ]
    },
    {
      "type": "paragraph",
      "content": [
        { "type": "text", "text": "See ", "marks": [ { "type": "bold" } ] },
        { "type": "text", "text": "the ticket: " },
        {
          "type": "hsLink",
          "attrs": {
            "refKind": "wp",
            "refValue": "WP-KERNEL-012",
            "label": "Native Editors",
            "resolved": true
          }
        },
        { "type": "text", "text": " now." }
      ]
    }
  ]
}"#;

#[test]
fn deserializes_real_backend_content_json_with_hs_link_and_multi_run() {
    // MUST-FIX #1 + #3: a REAL backend content_json containing an hsLink node and a
    // multi-run paragraph must deserialize (previously this hit UnknownNodeType on
    // hsLink and the editor could not open any Notes doc with a wikilink — RISK-5).
    let doc = from_json_string(REAL_BACKEND_CONTENT_JSON)
        .expect("real backend content_json with hsLink must deserialize");
    assert_eq!(doc.kind, NodeKind::Doc);
    // heading + paragraph.
    assert_eq!(doc.children.len(), 2);
    let para = doc.children[1].as_block().unwrap();
    assert_eq!(para.kind, NodeKind::Paragraph);
    // The multi-run paragraph kept all four inline children (bold run, plain run,
    // hsLink atom, trailing run) — nothing collapsed or dropped.
    assert_eq!(para.children.len(), 4);
    // The bold run preserved its mark.
    let first = para.children[0].as_text().unwrap();
    assert_eq!(first.text.to_string(), "See ");
    assert_eq!(first.marks, vec![Mark::Bold]);
    // The hsLink atom deserialized with its typed backend attrs.
    let link = para.children[2].as_hs_link().unwrap();
    assert_eq!(link.ref_kind, "wp");
    assert_eq!(link.ref_value, "WP-KERNEL-012");
    assert_eq!(link.label, "Native Editors");
    assert!(link.resolved);
}

#[test]
fn real_backend_content_json_reserializes_byte_shape_compatibly() {
    // MUST-FIX #3: deserialize the captured REAL backend value, re-serialize, and
    // assert the re-serialized JSON is STRUCTURALLY equal to the captured fixture
    // (parsed as serde_json::Value so whitespace differences do not matter). This
    // proves the wire shape matches the backend, not just that the serializer is the
    // inverse of its own deserializer.
    let doc = from_json_string(REAL_BACKEND_CONTENT_JSON).unwrap();
    let reserialized = to_content_json_value(&doc);
    let captured: JsonValue = serde_json::from_str(REAL_BACKEND_CONTENT_JSON).unwrap();
    assert_eq!(reserialized, captured, "re-serialized content_json must match the captured backend shape");
}

#[test]
fn split_multi_run_paragraph_carries_all_runs() {
    // RISK FIX: splitting a multi-run paragraph must not drop runs after the first.
    // doc > [ paragraph[ Text("Bold ",bold), Text("plain") ] ]; split at char 8 (3
    // chars into the second run) keeps "Bold pla" in the head and "in" in the tail,
    // both halves of the second run keeping NO marks, the first run keeping bold.
    let mut para = BlockNode::new(NodeKind::Paragraph);
    para.children.push(Child::Text(TextLeaf::with_marks("Bold ", vec![Mark::Bold])));
    para.children.push(Child::Text(TextLeaf::new("plain")));
    let mut doc = BlockNode::doc(vec![para]);
    apply_transaction(
        &mut doc,
        Transaction::operator(vec![Step::SplitNode { path: vec![0], char_offset: 8 }]),
    )
    .unwrap();
    assert_eq!(doc.children.len(), 2);
    let head = doc.children[0].as_block().unwrap();
    // head: Text("Bold ",bold) + Text("pla") — both runs preserved.
    assert_eq!(head.children.len(), 2);
    assert_eq!(head.children[0].as_text().unwrap().text.to_string(), "Bold ");
    assert_eq!(head.children[0].as_text().unwrap().marks, vec![Mark::Bold]);
    assert_eq!(head.children[1].as_text().unwrap().text.to_string(), "pla");
    let tail = doc.children[1].as_block().unwrap();
    assert_eq!(tail.children[0].as_text().unwrap().text.to_string(), "in");
}

#[test]
fn split_then_merge_multi_run_round_trips() {
    // RISK FIX: the SplitNode -> MergeNodes inverse must restore a MULTI-RUN node.
    let mut para = BlockNode::new(NodeKind::Paragraph);
    para.children.push(Child::Text(TextLeaf::with_marks("Bold ", vec![Mark::Bold])));
    para.children.push(Child::Text(TextLeaf::with_marks("italic", vec![Mark::Italic])));
    let mut doc = BlockNode::doc(vec![para]);
    let before = doc.clone();
    // Split mid-second-run (offset 8), then merge the tail back via the receipt inverse.
    let receipt = apply_transaction(
        &mut doc,
        Transaction::operator(vec![Step::SplitNode { path: vec![0], char_offset: 8 }]),
    )
    .unwrap();
    assert_eq!(doc.children.len(), 2);
    apply_transaction(&mut doc, Transaction::operator(receipt.inverse)).unwrap();
    assert_eq!(doc, before, "split then inverse-merge must restore the multi-run paragraph");
}

#[test]
fn merge_multi_run_paragraph_keeps_all_content() {
    // RISK FIX: merging a styled paragraph must keep every run + its marks, not just
    // the first text leaf.
    let mut p0 = BlockNode::new(NodeKind::Paragraph);
    p0.children.push(Child::Text(TextLeaf::new("head")));
    let mut p1 = BlockNode::new(NodeKind::Paragraph);
    p1.children.push(Child::Text(TextLeaf::with_marks("bold", vec![Mark::Bold])));
    p1.children.push(Child::Text(TextLeaf::with_marks("ital", vec![Mark::Italic])));
    let mut doc = BlockNode::doc(vec![p0, p1]);
    apply_transaction(
        &mut doc,
        Transaction::operator(vec![Step::MergeNodes { parent_path: vec![], index: 1 }]),
    )
    .unwrap();
    assert_eq!(doc.children.len(), 1);
    let merged = doc.children[0].as_block().unwrap();
    // "head" (no marks) + "bold" (bold) + "ital" (italic) all survive as distinct runs.
    assert_eq!(merged.children.len(), 3);
    assert_eq!(merged.children[0].as_text().unwrap().text.to_string(), "head");
    assert_eq!(merged.children[1].as_text().unwrap().marks, vec![Mark::Bold]);
    assert_eq!(merged.children[2].as_text().unwrap().marks, vec![Mark::Italic]);
}

#[test]
fn add_mark_on_code_block_is_rejected_and_rolls_back() {
    // COVERAGE GAP: pushing an AddMark onto a code_block's text leaf must fail schema
    // validation (code blocks carry no marks) and leave the doc unchanged.
    let mut cb = BlockNode::new(NodeKind::CodeBlock);
    cb.children.push(Child::Text(TextLeaf::new("let x = 1;")));
    let mut doc = BlockNode::doc(vec![cb]);
    let before = doc.clone();
    let err = apply_transaction(
        &mut doc,
        Transaction::operator(vec![Step::AddMark { path: vec![0, 0], mark: Mark::Bold }]),
    )
    .unwrap_err();
    assert!(matches!(err, TransformError::Schema(_)), "got {err:?}");
    assert_eq!(doc, before, "code_block mark rejection must roll back the doc");
}

#[test]
fn task_item_checked_and_code_block_language_attrs_survive_round_trip() {
    // COVERAGE GAP: attr-bearing nodes' payloads (task_item.checked, code_block.
    // language) must survive the DocJson round-trip, not just compile.
    let mut task = BlockNode::new(NodeKind::TaskItem);
    task.attrs.insert("checked".into(), JsonValue::Bool(true));
    task.children.push(Child::Block(BlockNode::paragraph("do it")));
    let mut code = BlockNode::new(NodeKind::CodeBlock);
    code.attrs.insert("language".into(), JsonValue::from("rust"));
    code.children.push(Child::Text(TextLeaf::new("fn main() {}")));
    let mut list = BlockNode::new(NodeKind::BulletList);
    list.children.push(Child::Block(task));
    let doc = BlockNode::doc(vec![list, code]);
    let json = to_json_string(&doc).unwrap();
    let back = from_json_string(&json).unwrap();
    assert_eq!(doc, back);
    // The attrs are present in the wire content_json.
    let v = to_content_json_value(&doc);
    assert_eq!(v["content"][0]["content"][0]["attrs"]["checked"], true);
    assert_eq!(v["content"][1]["attrs"]["language"], "rust");
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
