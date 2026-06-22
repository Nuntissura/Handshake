//! Autocomplete confirm / cancel doc mutations (WP-KERNEL-012 MT-015).
//!
//! On Enter/Tab in the autocomplete popup, the selected result becomes an inserted `hsLink` atom
//! (NOT a `Mark` via `AddMark` — the MT-011-reconciled shape); the `[[query` trigger text is removed.
//! On Escape, the popup closes and the `[[query` text is removed (AC: Escape closes + removes the
//! trigger).
//!
//! ## Why a direct children mutation (not a transform Step)
//!
//! MT-011's `transform::Step::InsertNode` inserts a `Child::Block` only — there is NO step that
//! inserts an inline `Child::HsLink` atom, and `transform.rs` is OUT of this MT's `allowed_paths`
//! (the model is MT-011's; a new inline-insert step would be an out-of-scope model change). So the
//! confirm performs the insert by directly editing the paragraph's `children` vec (the model layer
//! makes `Child::HsLink` a first-class variant, so this produces a schema-valid tree that
//! round-trips through DocJson). The `[[query` text removal uses the rope's char-indexed `remove`.
//! The operation is wrapped so the editor records it; backspace over the chip (the existing
//! `DeleteText`/`DeleteNode` paths) removes it.
//!
//! These functions are pure over `(doc, leaf_path, …)` so they are unit-testable with no egui/runtime.

use crate::rich_editor::document_model::node::{BlockNode, Child, HsLinkNode};
use crate::rich_editor::document_model::position::DocPosition;
use crate::rich_editor::document_model::selection::Selection;

/// Remove the `[[query` trigger text from the text leaf at `leaf_path`, starting at char offset
/// `trigger_start_char`, up to `caret_char` (the caret position when the popup closed). Returns the
/// number of chars removed (so the caller can adjust the caret). A no-op (returns 0) when the path
/// does not resolve to a text leaf or the range is empty/inverted.
///
/// This is the shared "remove the `[[…` typed-so-far text" step used by BOTH confirm (before
/// inserting the chip) and cancel (Escape just removes the trigger text).
pub fn remove_trigger_text(
    doc: &mut BlockNode,
    leaf_path: &[usize],
    trigger_start_char: usize,
    caret_char: usize,
) -> usize {
    let Some(leaf) = leaf_at_mut(doc, leaf_path) else {
        return 0;
    };
    let len = leaf.text.len_chars();
    let start = trigger_start_char.min(len);
    let end = caret_char.min(len).max(start);
    if end == start {
        return 0;
    }
    leaf.text.remove(start, end);
    end - start
}

/// Confirm an autocomplete selection into the document: remove the `[[query` trigger text from the
/// leaf at `leaf_path` (between `trigger_start_char` and `caret_char`), then insert `link` as an
/// inline `Child::HsLink` atom into the PARENT block immediately AFTER the trigger leaf. Updates the
/// selection to a caret just after the inserted atom. Returns `true` when the insertion happened.
///
/// The atom is inserted as a SIBLING of the trigger text leaf (a paragraph holds a flat list of
/// `Text` runs + inline atoms), matching how the React editor inserts a wikilink via `insertHsLink`
/// (an inline atom in the paragraph's content). The trigger leaf is left in place (now without the
/// `[[query` text), so existing text before the trigger is preserved.
pub fn confirm_wikilink(
    doc: &mut BlockNode,
    selection: &mut Selection,
    leaf_path: &[usize],
    trigger_start_char: usize,
    caret_char: usize,
    link: HsLinkNode,
) -> bool {
    // 1) Remove the `[[query` text from the trigger leaf.
    remove_trigger_text(doc, leaf_path, trigger_start_char, caret_char);

    // 2) Resolve the PARENT block + the trigger leaf's index within it.
    let Some((leaf_idx, parent_path)) = leaf_path.split_last() else {
        return false;
    };
    let Some(parent) = block_at_mut(doc, parent_path) else {
        return false;
    };
    if *leaf_idx >= parent.children.len() {
        return false;
    }
    // 3) Insert the hsLink atom right AFTER the trigger leaf.
    let insert_at = *leaf_idx + 1;
    parent.children.insert(insert_at, Child::HsLink(link));

    // 4) Place the caret just after the inserted atom: a new empty text leaf is added after the atom
    //    if there is no following text leaf, so the caret has a text position to land on (a paragraph
    //    must end with addressable inline text for the caret model). The caret sits at offset 0 of
    //    that trailing leaf.
    let trailing_idx = insert_at + 1;
    let needs_trailing = parent
        .children
        .get(trailing_idx)
        .map(|c| c.as_text().is_none())
        .unwrap_or(true);
    if needs_trailing {
        parent.children.insert(
            trailing_idx,
            Child::Text(crate::rich_editor::document_model::node::TextLeaf::new("")),
        );
    }
    let mut caret_path = parent_path.to_vec();
    caret_path.push(trailing_idx);
    *selection = Selection::caret(DocPosition::new(caret_path, 0));
    true
}

/// Cancel the autocomplete (Escape): remove the `[[query` trigger text and place the caret where the
/// trigger started. Returns the number of chars removed.
pub fn cancel_wikilink(
    doc: &mut BlockNode,
    selection: &mut Selection,
    leaf_path: &[usize],
    trigger_start_char: usize,
    caret_char: usize,
) -> usize {
    let removed = remove_trigger_text(doc, leaf_path, trigger_start_char, caret_char);
    *selection = Selection::caret(DocPosition::new(leaf_path.to_vec(), trigger_start_char));
    removed
}

/// Resolve a `leaf_path` (block indices then a final text-leaf index) to a mutable text leaf.
fn leaf_at_mut<'a>(
    doc: &'a mut BlockNode,
    path: &[usize],
) -> Option<&'a mut crate::rich_editor::document_model::node::TextLeaf> {
    let (leaf_idx, block_path) = path.split_last()?;
    let node = block_at_mut(doc, block_path)?;
    node.children.get_mut(*leaf_idx)?.as_text_mut()
}

/// Resolve a block `path` (child indices from the doc root) to a mutable block node.
fn block_at_mut<'a>(doc: &'a mut BlockNode, path: &[usize]) -> Option<&'a mut BlockNode> {
    let mut node = doc;
    for &idx in path {
        node = node.children.get_mut(idx)?.as_block_mut()?;
    }
    Some(node)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rich_editor::document_model::node::{NodeKind, TextLeaf};

    fn doc_with_trigger(text: &str) -> BlockNode {
        // doc > paragraph(text) — text contains the `[[query` trigger.
        BlockNode::doc(vec![BlockNode::with_children(
            NodeKind::Paragraph,
            vec![Child::Text(TextLeaf::new(text))],
        )])
    }

    #[test]
    fn remove_trigger_text_removes_the_open_token() {
        // "see [[wp:WP-" with the `[[` at char 4, caret at the end (char 12): removes "[[wp:WP-".
        let mut doc = doc_with_trigger("see [[wp:WP-");
        let removed = remove_trigger_text(&mut doc, &[0, 0], 4, 12);
        assert_eq!(removed, 8, "removed `[[wp:WP-` (8 chars)");
        let leaf = doc.children[0].as_block().unwrap().children[0].as_text().unwrap();
        assert_eq!(leaf.text.to_string(), "see ", "the text before the trigger is preserved");
    }

    #[test]
    fn confirm_inserts_hs_link_atom_and_removes_trigger() {
        // Confirm `[[wp:WP-` -> an hsLink(wp, WP-KERNEL-012) atom; the trigger text is gone.
        let mut doc = doc_with_trigger("see [[wp:WP-");
        let mut sel = Selection::caret(DocPosition::new(vec![0, 0], 12));
        let link = HsLinkNode::new("wp", "WP-KERNEL-012", "My WP");
        assert!(confirm_wikilink(&mut doc, &mut sel, &[0, 0], 4, 12, link.clone()));

        let para = doc.children[0].as_block().unwrap();
        // children: [ Text("see "), HsLink(wp:WP-KERNEL-012), Text("") ]
        assert_eq!(para.children[0].as_text().unwrap().text.to_string(), "see ");
        let inserted = para.children[1].as_hs_link().unwrap();
        assert_eq!(inserted, &link, "the hsLink atom is inserted, NOT a mark");
        assert!(para.children[2].as_text().is_some(), "a trailing text leaf hosts the caret");
        // The caret sits at the trailing leaf, offset 0.
        match &sel {
            Selection::Text { head, .. } => {
                assert_eq!(head.path, vec![0, 2]);
                assert_eq!(head.char_offset, 0);
            }
            _ => panic!("expected a caret"),
        }
    }

    #[test]
    fn confirm_reuses_following_text_leaf_when_present() {
        // A paragraph "x[[wp:" followed by more text " y" (two leaves): confirm reuses the following
        // leaf as the caret target rather than inserting a redundant empty leaf.
        let mut doc = BlockNode::doc(vec![BlockNode::with_children(
            NodeKind::Paragraph,
            vec![Child::Text(TextLeaf::new("x[[wp:")), Child::Text(TextLeaf::new(" y"))],
        )]);
        let mut sel = Selection::caret(DocPosition::new(vec![0, 0], 6));
        let link = HsLinkNode::new("wp", "W", "W");
        assert!(confirm_wikilink(&mut doc, &mut sel, &[0, 0], 1, 6, link));
        let para = doc.children[0].as_block().unwrap();
        // children: [ Text("x"), HsLink, Text(" y") ] — the existing following leaf is the caret host.
        assert_eq!(para.children.len(), 3);
        assert_eq!(para.children[0].as_text().unwrap().text.to_string(), "x");
        assert!(para.children[1].as_hs_link().is_some());
        assert_eq!(para.children[2].as_text().unwrap().text.to_string(), " y");
        assert!(matches!(&sel, Selection::Text { head, .. } if head.path == vec![0, 2]));
    }

    #[test]
    fn cancel_removes_trigger_and_places_caret() {
        let mut doc = doc_with_trigger("a [[fi");
        let mut sel = Selection::caret(DocPosition::new(vec![0, 0], 6));
        let removed = cancel_wikilink(&mut doc, &mut sel, &[0, 0], 2, 6);
        assert_eq!(removed, 4, "removed `[[fi`");
        assert_eq!(doc.children[0].as_block().unwrap().children[0].as_text().unwrap().text.to_string(), "a ");
        assert!(matches!(&sel, Selection::Text { head, .. } if head.char_offset == 2));
    }

    #[test]
    fn confirmed_doc_round_trips_through_doc_json() {
        // The confirmed hsLink atom is a valid model node that serializes to the backend content_json
        // (proves InsertNode-of-atom produces a round-trippable doc, the MT contract requirement).
        use crate::rich_editor::document_model::doc_json::{from_json_string, to_json_string};
        let mut doc = doc_with_trigger("[[wp:");
        let mut sel = Selection::caret(DocPosition::new(vec![0, 0], 5));
        confirm_wikilink(&mut doc, &mut sel, &[0, 0], 0, 5, HsLinkNode::new("wp", "WP-1", "One"));
        let json = to_json_string(&doc).unwrap();
        let back = from_json_string(&json).unwrap();
        assert_eq!(doc, back, "the confirmed wikilink doc round-trips through DocJson");
        // The hsLink node is present in the serialized content_json.
        assert!(json.contains("\"hsLink\""), "the serialized doc carries an hsLink node");
        assert!(json.contains("WP-1"));
    }
}
