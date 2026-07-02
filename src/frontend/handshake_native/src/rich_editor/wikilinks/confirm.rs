//! Autocomplete confirm / cancel doc mutations (WP-KERNEL-012 MT-015; MT-020 undo rewire).
//!
//! On Enter/Tab in the autocomplete popup, the selected result becomes an inserted `hsLink` atom
//! (NOT a `Mark` via `AddMark` — the MT-011-reconciled shape); the `[[query` trigger text is removed.
//! On Escape, the popup closes and the `[[query` text is removed (AC: Escape closes + removes the
//! trigger).
//!
//! ## Transactional insert (the MT-020 inline-atom undo rewire)
//!
//! `transform.rs` DOES carry inline-atom steps ([`Step::InsertInlineChild`] /
//! [`Step::DeleteInlineChild`] — the MT-020 amendment carried from MT-015), so the confirm goes
//! through [`apply_transaction`] as ONE atomic transaction: the `[[query` trigger-text removal
//! (a `DeleteText` step), the `hsLink` atom insert (an `InsertInlineChild` step), and — when
//! needed — the trailing caret-host text leaf (a second `InsertInlineChild`). The receipt is
//! pushed onto the caller's [`UndoManager`], so a confirmed wikilink/tag chip is ON the undo
//! stack like every other edit (one undo restores the pre-confirm doc, trigger text included)
//! and no direct children mutation can position-corrupt earlier undo receipts. The cancel path
//! (Escape) is the same discipline: a transactional `DeleteText` + `history.push`.
//!
//! These functions are pure over `(doc, history, leaf_path, …)` so they are unit-testable with no
//! egui/runtime.

use crate::rich_editor::document_model::history::UndoManager;
use crate::rich_editor::document_model::node::{BlockNode, Child, HsLinkNode, TextLeaf};
use crate::rich_editor::document_model::position::DocPosition;
use crate::rich_editor::document_model::selection::Selection;
use crate::rich_editor::document_model::transform::{
    apply_transaction, ActorKind, Step, Transaction,
};

/// The clamped `[start, end)` CHAR range of the `[[query` trigger text inside the text leaf at
/// `leaf_path` (from `trigger_start_char` to `caret_char`, both clamped into the leaf's length).
/// `None` when the path does not resolve to a text leaf or the clamped range is empty/inverted.
///
/// This is the shared "where is the `[[…` typed-so-far text" computation used by BOTH confirm
/// (removing the trigger before inserting the chip) and cancel (Escape just removes the trigger).
fn trigger_text_range(
    doc: &BlockNode,
    leaf_path: &[usize],
    trigger_start_char: usize,
    caret_char: usize,
) -> Option<(usize, usize)> {
    let leaf = leaf_at(doc, leaf_path)?;
    let len = leaf.text.len_chars();
    let start = trigger_start_char.min(len);
    let end = caret_char.min(len).max(start);
    (end > start).then_some((start, end))
}

/// Confirm an autocomplete selection into the document: remove the `[[query` trigger text from the
/// leaf at `leaf_path` (between `trigger_start_char` and `caret_char`), then insert `link` as an
/// inline `Child::HsLink` atom into the PARENT block immediately AFTER the trigger leaf — as ONE
/// atomic transaction whose receipt is pushed onto `history` (MT-020: the insert is UNDOABLE).
/// Updates the selection to a caret just after the inserted atom. Returns `true` when the
/// insertion happened (on any transform/schema failure the doc is rolled back untouched).
///
/// The atom is inserted as a SIBLING of the trigger text leaf (a paragraph holds a flat list of
/// `Text` runs + inline atoms), matching how the React editor inserts a wikilink via `insertHsLink`
/// (an inline atom in the paragraph's content). The trigger leaf is left in place (now without the
/// `[[query` text), so existing text before the trigger is preserved. `actor_id` threads
/// transaction provenance (HBR-SWARM attribution).
#[allow(clippy::too_many_arguments)]
pub fn confirm_wikilink(
    doc: &mut BlockNode,
    history: &mut UndoManager,
    selection: &mut Selection,
    leaf_path: &[usize],
    trigger_start_char: usize,
    caret_char: usize,
    link: HsLinkNode,
    actor_id: &str,
) -> bool {
    // 1) Resolve the PARENT block + the trigger leaf's index within it (pre-state addressing).
    let Some((leaf_idx, parent_path)) = leaf_path.split_last() else {
        return false;
    };
    // Pre-state facts needed to shape the steps: the leaf must exist in the parent, and the child
    // CURRENTLY following the trigger leaf decides whether a trailing caret-host leaf is needed
    // (post-insert it sits one index further right).
    let needs_trailing = {
        let Some(parent) = block_at(doc, parent_path) else {
            return false;
        };
        if *leaf_idx >= parent.children.len() {
            return false;
        }
        parent
            .children
            .get(*leaf_idx + 1)
            .map(|c| c.as_text().is_none())
            .unwrap_or(true)
    };

    // 2) Build the ONE transaction: trigger-text removal, atom insert, optional trailing leaf.
    let mut steps = Vec::with_capacity(3);
    if let Some((start, end)) = trigger_text_range(doc, leaf_path, trigger_start_char, caret_char) {
        steps.push(Step::DeleteText {
            path: leaf_path.to_vec(),
            start,
            end,
        });
    }
    let insert_at = *leaf_idx + 1;
    steps.push(Step::InsertInlineChild {
        parent_path: parent_path.to_vec(),
        index: insert_at,
        child: Child::HsLink(link),
    });
    // 3) A new empty text leaf is added after the atom if there is no following text leaf, so the
    //    caret has a text position to land on (a paragraph must end with addressable inline text
    //    for the caret model). The caret sits at offset 0 of that trailing leaf.
    let trailing_idx = insert_at + 1;
    if needs_trailing {
        steps.push(Step::InsertInlineChild {
            parent_path: parent_path.to_vec(),
            index: trailing_idx,
            child: Child::Text(TextLeaf::new("")),
        });
    }

    // 4) Apply atomically + push the receipt so Ctrl+Z undoes the WHOLE confirm (MT-020).
    let tx = Transaction::new(steps, ActorKind::Operator, actor_id);
    match apply_transaction(doc, tx) {
        Ok(receipt) => {
            history.push(receipt);
            let mut caret_path = parent_path.to_vec();
            caret_path.push(trailing_idx);
            *selection = Selection::caret(DocPosition::new(caret_path, 0));
            true
        }
        Err(_) => false, // rolled back — the doc is untouched, nothing is on the undo stack.
    }
}

/// WP-KERNEL-012 MT-057: rewrite every UNRESOLVED `hsLink` atom whose `ref_value` (normalized title)
/// equals `normalized_title` to a RESOLVED note link targeting `document_id` (AC-002 — after a
/// create-from-unresolved succeeds, the originating mark re-renders LIVE without a document reload).
/// Walks the whole tree (an unresolved title may appear in more than one place); each match becomes a
/// resolved `note` link with `ref_value = document_id` and `resolved = true`, so the next render paints
/// it with the live-link affordance and a click navigates to the document. The label is preserved when
/// the link had an explicit one, otherwise set to `display_title`. Returns the count of marks rewritten
/// (0 when none matched — never a panic). Pure over `(doc, …)` so it is unit-testable with no egui.
pub fn rewrite_mark_to_resolved(
    doc: &mut BlockNode,
    normalized_title: &str,
    document_id: &str,
    display_title: &str,
) -> usize {
    fn norm(s: &str) -> String {
        crate::rich_editor::wikilinks::resolver::normalize_target(s)
    }
    let mut rewritten = 0usize;
    rewrite_in_block(doc, &mut |link| {
        // Only an UNRESOLVED link whose target normalizes to the created title is rewritten. A code
        // ref / already-resolved link is left untouched (it is not the create-from-unresolved subject).
        if !link.resolved && norm(&link.ref_value) == normalized_title {
            link.ref_kind = "note".to_owned();
            link.ref_value = document_id.to_owned();
            link.resolved = true;
            if link.label.trim().is_empty() {
                link.label = display_title.to_owned();
            }
            rewritten += 1;
        }
    });
    rewritten
}

/// Recurse the block tree, applying `f` to every `Child::HsLink`'s [`HsLinkNode`].
fn rewrite_in_block(block: &mut BlockNode, f: &mut impl FnMut(&mut HsLinkNode)) {
    for child in block.children.iter_mut() {
        match child {
            Child::HsLink(link) => f(link),
            Child::Block(b) => rewrite_in_block(b, f),
            _ => {}
        }
    }
}

/// Cancel the autocomplete (Escape): remove the `[[query` trigger text — transactionally, with the
/// receipt pushed onto `history` (MT-020: the removal is undoable and never bypasses the undo
/// manager) — and place the caret where the trigger started. Returns the number of chars removed.
pub fn cancel_wikilink(
    doc: &mut BlockNode,
    history: &mut UndoManager,
    selection: &mut Selection,
    leaf_path: &[usize],
    trigger_start_char: usize,
    caret_char: usize,
    actor_id: &str,
) -> usize {
    let removed = match trigger_text_range(doc, leaf_path, trigger_start_char, caret_char) {
        Some((start, end)) => {
            let tx = Transaction::new(
                vec![Step::DeleteText {
                    path: leaf_path.to_vec(),
                    start,
                    end,
                }],
                ActorKind::Operator,
                actor_id,
            );
            match apply_transaction(doc, tx) {
                Ok(receipt) => {
                    history.push(receipt);
                    end - start
                }
                Err(_) => 0, // rolled back — nothing removed.
            }
        }
        None => 0,
    };
    *selection = Selection::caret(DocPosition::new(leaf_path.to_vec(), trigger_start_char));
    removed
}

/// Resolve a `leaf_path` (block indices then a final text-leaf index) to a text leaf.
fn leaf_at<'a>(doc: &'a BlockNode, path: &[usize]) -> Option<&'a TextLeaf> {
    let (leaf_idx, block_path) = path.split_last()?;
    let node = block_at(doc, block_path)?;
    node.children.get(*leaf_idx)?.as_text()
}

/// Resolve a block `path` (child indices from the doc root) to a block node.
fn block_at<'a>(doc: &'a BlockNode, path: &[usize]) -> Option<&'a BlockNode> {
    let mut node = doc;
    for &idx in path {
        node = node.children.get(idx)?.as_block()?;
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
    fn cancel_removes_the_open_token_transactionally_and_undo_restores_it() {
        // "see [[wp:WP-" with the `[[` at char 4, caret at the end (char 12): cancel removes
        // "[[wp:WP-" via a DeleteText TRANSACTION pushed on the history (MT-020) — undo restores it.
        let mut doc = doc_with_trigger("see [[wp:WP-");
        let before = doc.clone();
        let mut history = UndoManager::new();
        let mut sel = Selection::caret(DocPosition::new(vec![0, 0], 12));
        let removed = cancel_wikilink(&mut doc, &mut history, &mut sel, &[0, 0], 4, 12, "operator");
        assert_eq!(removed, 8, "removed `[[wp:WP-` (8 chars)");
        let leaf = doc.children[0].as_block().unwrap().children[0]
            .as_text()
            .unwrap();
        assert_eq!(
            leaf.text.to_string(),
            "see ",
            "the text before the trigger is preserved"
        );
        assert_eq!(history.len(), 1, "the cancel pushed ONE receipt");
        assert!(history.undo(&mut doc).unwrap(), "undo applies");
        assert_eq!(doc, before, "undo restores the `[[wp:WP-` trigger text");
    }

    #[test]
    fn confirm_inserts_hs_link_atom_and_removes_trigger() {
        // Confirm `[[wp:WP-` -> an hsLink(wp, WP-KERNEL-012) atom; the trigger text is gone.
        let mut doc = doc_with_trigger("see [[wp:WP-");
        let mut history = UndoManager::new();
        let mut sel = Selection::caret(DocPosition::new(vec![0, 0], 12));
        let link = HsLinkNode::new("wp", "WP-KERNEL-012", "My WP");
        assert!(confirm_wikilink(
            &mut doc,
            &mut history,
            &mut sel,
            &[0, 0],
            4,
            12,
            link.clone(),
            "operator",
        ));

        let para = doc.children[0].as_block().unwrap();
        // children: [ Text("see "), HsLink(wp:WP-KERNEL-012), Text("") ]
        assert_eq!(para.children[0].as_text().unwrap().text.to_string(), "see ");
        let inserted = para.children[1].as_hs_link().unwrap();
        assert_eq!(inserted, &link, "the hsLink atom is inserted, NOT a mark");
        assert!(
            para.children[2].as_text().is_some(),
            "a trailing text leaf hosts the caret"
        );
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
    fn confirm_is_one_undoable_transaction_restoring_the_pre_confirm_doc() {
        // MT-020: the WHOLE confirm (trigger removal + atom insert + trailing leaf) is ONE receipt
        // on the undo manager; one undo restores the exact pre-confirm doc (trigger text included),
        // and redo re-applies the confirmed doc — no position corruption either way.
        let mut doc = doc_with_trigger("see [[wp:WP-");
        let before = doc.clone();
        let mut history = UndoManager::new();
        let mut sel = Selection::caret(DocPosition::new(vec![0, 0], 12));
        let link = HsLinkNode::new("wp", "WP-KERNEL-012", "My WP");
        assert!(confirm_wikilink(
            &mut doc,
            &mut history,
            &mut sel,
            &[0, 0],
            4,
            12,
            link,
            "operator",
        ));
        let confirmed = doc.clone();
        assert_eq!(history.len(), 1, "the confirm pushed exactly ONE receipt");
        assert!(history.undo(&mut doc).unwrap(), "undo applies");
        assert_eq!(
            doc, before,
            "one undo restores the exact pre-confirm doc (atom gone, `[[wp:WP-` back)"
        );
        assert!(history.redo(&mut doc).unwrap(), "redo applies");
        assert_eq!(doc, confirmed, "redo restores the confirmed doc exactly");
    }

    #[test]
    fn confirm_reuses_following_text_leaf_when_present() {
        // A paragraph "x[[wp:" followed by more text " y" (two leaves): confirm reuses the following
        // leaf as the caret target rather than inserting a redundant empty leaf.
        let mut doc = BlockNode::doc(vec![BlockNode::with_children(
            NodeKind::Paragraph,
            vec![
                Child::Text(TextLeaf::new("x[[wp:")),
                Child::Text(TextLeaf::new(" y")),
            ],
        )]);
        let mut history = UndoManager::new();
        let mut sel = Selection::caret(DocPosition::new(vec![0, 0], 6));
        let link = HsLinkNode::new("wp", "W", "W");
        assert!(confirm_wikilink(
            &mut doc,
            &mut history,
            &mut sel,
            &[0, 0],
            1,
            6,
            link,
            "operator",
        ));
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
        let mut history = UndoManager::new();
        let mut sel = Selection::caret(DocPosition::new(vec![0, 0], 6));
        let removed = cancel_wikilink(&mut doc, &mut history, &mut sel, &[0, 0], 2, 6, "operator");
        assert_eq!(removed, 4, "removed `[[fi`");
        assert_eq!(
            doc.children[0].as_block().unwrap().children[0]
                .as_text()
                .unwrap()
                .text
                .to_string(),
            "a "
        );
        assert!(matches!(&sel, Selection::Text { head, .. } if head.char_offset == 2));
    }

    #[test]
    fn rewrite_mark_to_resolved_flips_unresolved_link_live_ac002() {
        // AC-002: an UNRESOLVED `[[My New Note]]` mark (ref_kind=note, resolved=false, ref_value=the
        // title) becomes Resolved{document_id} after a create — re-rendered live, no reload.
        use crate::rich_editor::wikilinks::resolver::normalize_target;
        let mut unresolved = HsLinkNode::new("note", "My New Note", "My New Note");
        unresolved.resolved = false;
        let mut doc = BlockNode::doc(vec![BlockNode::with_children(
            NodeKind::Paragraph,
            vec![
                Child::Text(TextLeaf::new("see ")),
                Child::HsLink(unresolved),
            ],
        )]);
        let n = rewrite_mark_to_resolved(
            &mut doc,
            &normalize_target("My New Note"),
            "DOC-NEW",
            "My New Note",
        );
        assert_eq!(n, 1, "exactly one unresolved mark rewritten");
        let link = doc.children[0].as_block().unwrap().children[1]
            .as_hs_link()
            .unwrap();
        assert!(link.resolved, "AC-002: the mark is now Resolved");
        assert_eq!(
            link.ref_value, "DOC-NEW",
            "the mark targets the new document id"
        );
        assert_eq!(link.ref_kind, "note");
    }

    #[test]
    fn rewrite_leaves_already_resolved_and_nonmatching_marks_untouched() {
        use crate::rich_editor::wikilinks::resolver::normalize_target;
        // A RESOLVED link to a different doc + an unresolved link to a DIFFERENT title must be left
        // alone (only the matching unresolved title is rewritten).
        let resolved = HsLinkNode::new("wp", "WP-1", "WP One"); // resolved=true by default
        let mut other_unresolved = HsLinkNode::new("note", "Different Title", "Different Title");
        other_unresolved.resolved = false;
        let mut doc = BlockNode::doc(vec![BlockNode::with_children(
            NodeKind::Paragraph,
            vec![Child::HsLink(resolved), Child::HsLink(other_unresolved)],
        )]);
        let n = rewrite_mark_to_resolved(
            &mut doc,
            &normalize_target("My New Note"),
            "DOC-NEW",
            "My New Note",
        );
        assert_eq!(
            n, 0,
            "no mark matches the created title -> nothing rewritten"
        );
        let para = doc.children[0].as_block().unwrap();
        assert!(
            para.children[0].as_hs_link().unwrap().resolved,
            "the already-resolved link is untouched"
        );
        assert!(
            !para.children[1].as_hs_link().unwrap().resolved,
            "the non-matching unresolved link stays unresolved"
        );
    }

    #[test]
    fn rewrite_matches_case_insensitively_via_normalized_title() {
        use crate::rich_editor::wikilinks::resolver::normalize_target;
        // The mark's ref_value differs in case/whitespace from the create title; the normalized match
        // still rewrites it (Obsidian-default).
        let mut unresolved = HsLinkNode::new("note", "  my   NEW note ", "");
        unresolved.resolved = false;
        let mut doc = BlockNode::doc(vec![BlockNode::with_children(
            NodeKind::Paragraph,
            vec![Child::HsLink(unresolved)],
        )]);
        let n = rewrite_mark_to_resolved(
            &mut doc,
            &normalize_target("My New Note"),
            "DOC-X",
            "My New Note",
        );
        assert_eq!(n, 1, "a case/whitespace-different unresolved title still matches the normalized created title");
        let link = doc.children[0].as_block().unwrap().children[0]
            .as_hs_link()
            .unwrap();
        assert_eq!(link.ref_value, "DOC-X");
        assert_eq!(
            link.label, "My New Note",
            "a blank label is filled from the display title"
        );
    }

    #[test]
    fn confirmed_doc_round_trips_through_doc_json() {
        // The confirmed hsLink atom is a valid model node that serializes to the backend content_json
        // (proves InsertNode-of-atom produces a round-trippable doc, the MT contract requirement).
        use crate::rich_editor::document_model::doc_json::{from_json_string, to_json_string};
        let mut doc = doc_with_trigger("[[wp:");
        let mut history = UndoManager::new();
        let mut sel = Selection::caret(DocPosition::new(vec![0, 0], 5));
        confirm_wikilink(
            &mut doc,
            &mut history,
            &mut sel,
            &[0, 0],
            0,
            5,
            HsLinkNode::new("wp", "WP-1", "One"),
            "operator",
        );
        let json = to_json_string(&doc).unwrap();
        let back = from_json_string(&json).unwrap();
        assert_eq!(
            doc, back,
            "the confirmed wikilink doc round-trips through DocJson"
        );
        // The hsLink node is present in the serialized content_json.
        assert!(
            json.contains("\"hsLink\""),
            "the serialized doc carries an hsLink node"
        );
        assert!(json.contains("WP-1"));
    }
}
