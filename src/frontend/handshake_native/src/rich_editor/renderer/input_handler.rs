//! Keyboard + text input -> document Transactions (WP-KERNEL-012 MT-012).
//!
//! Converts egui input events (`egui::Event::Text` for printable chars, `egui::Event::Key`
//! for editing/navigation keys) into MT-011 [`Step`]s / [`Transaction`]s applied to the
//! [`BlockNode`] doc through [`apply_transaction`], with undo/redo via the MT-011
//! [`UndoManager`]. The contract maps:
//! - printable chars  -> `InsertText` at the caret
//! - Backspace        -> `DeleteText` one char left
//! - Delete           -> `DeleteText` one char right (forward)
//! - ArrowLeft/Right  -> caret move by one char (selection collapse/extend with Shift)
//! - Home/End         -> caret to line/leaf start/end
//! - Ctrl+A           -> select-all over the caret's leaf (vertical-slice scope)
//! - Ctrl+Z / Ctrl+Shift+Z -> undo / redo via the UndoManager
//!
//! Caret-bound discipline (red-team MC caret validation): every produced offset is
//! validated/clamped against the addressed leaf length HERE, at the input layer, so an
//! off-by-one is caught at the source rather than masked by the rope's silent clamp.

use crate::rich_editor::document_model::node::{BlockNode, Child, NodeKind};
use crate::rich_editor::document_model::position::DocPosition;
use crate::rich_editor::document_model::selection::Selection;
use crate::rich_editor::document_model::transform::{apply_transaction, Step, Transaction};
use crate::rich_editor::document_model::history::UndoManager;
use crate::rich_editor::formatting::commands::{self, CommandContext, FormattingCommand};
use crate::rich_editor::formatting::keymap;

/// The mutable editor state the input handler drives: the document tree, the current
/// selection, and the undo manager. Owned by the widget; passed by `&mut` so one input
/// pass can apply a batch of events in order.
pub struct EditContext<'a> {
    /// The document being edited (the `doc` root).
    pub doc: &'a mut BlockNode,
    /// The current selection (caret = collapsed selection).
    pub selection: &'a mut Selection,
    /// The undo/redo history.
    pub undo: &'a mut UndoManager,
    /// Actor id for transaction provenance (operator vs agent). The widget passes
    /// `"operator"` for keyboard input.
    pub actor_id: &'a str,
}

/// One decoded editor action, produced from an egui event. Kept as an explicit enum (not
/// applied inline) so the mapping is unit-testable WITHOUT a live egui context: a test
/// decodes a synthetic event list to `Vec<EditAction>` and asserts the mapping, and a
/// separate test applies actions to a doc and asserts the model result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditAction {
    /// Insert a string at the caret (a printable char or an IME commit chunk).
    Insert(String),
    /// Delete one char to the LEFT of the caret (Backspace).
    DeleteBackward,
    /// Delete one char to the RIGHT of the caret (Delete).
    DeleteForward,
    /// Move the caret one char left; `extend` keeps the anchor (Shift).
    MoveLeft { extend: bool },
    /// Move the caret one char right; `extend` keeps the anchor (Shift).
    MoveRight { extend: bool },
    /// Move the caret to the start of its leaf (Home).
    MoveHome { extend: bool },
    /// Move the caret to the end of its leaf (End).
    MoveEnd { extend: bool },
    /// Select the whole current leaf (Ctrl+A, vertical-slice scope).
    SelectAll,
    /// Undo the last transaction (Ctrl+Z).
    Undo,
    /// Redo (Ctrl+Shift+Z).
    Redo,
}

/// Decode an egui input snapshot's events into ordered [`EditAction`]s. Pure mapping;
/// does NOT mutate the doc (the widget applies the actions). `consume` is the set of keys
/// the caller should mark consumed so egui does not double-handle them (the widget calls
/// `ui.input_mut(|i| i.consume_key(..))` for these); we return the decoded actions and let
/// the widget consume — keeping this fn context-free for testing.
pub fn decode_events(events: &[egui::Event]) -> Vec<EditAction> {
    let mut out = Vec::new();
    for ev in events {
        match ev {
            egui::Event::Text(s) if !s.is_empty() => out.push(EditAction::Insert(s.clone())),
            egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } => {
                let shift = modifiers.shift;
                let ctrl = modifiers.command || modifiers.ctrl;
                match key {
                    egui::Key::Backspace => out.push(EditAction::DeleteBackward),
                    egui::Key::Delete => out.push(EditAction::DeleteForward),
                    egui::Key::ArrowLeft => out.push(EditAction::MoveLeft { extend: shift }),
                    egui::Key::ArrowRight => out.push(EditAction::MoveRight { extend: shift }),
                    egui::Key::Home => out.push(EditAction::MoveHome { extend: shift }),
                    egui::Key::End => out.push(EditAction::MoveEnd { extend: shift }),
                    egui::Key::A if ctrl => out.push(EditAction::SelectAll),
                    egui::Key::Z if ctrl && shift => out.push(EditAction::Redo),
                    egui::Key::Z if ctrl => out.push(EditAction::Undo),
                    // Up/Down are intentionally NOT decoded in the MT-012 vertical slice
                    // (single-paragraph caret); cross-paragraph vertical motion is a
                    // later pass. They simply produce no action here.
                    _ => {}
                }
            }
            _ => {}
        }
    }
    out
}

/// Decode the plain text/nav events EXCLUDING any key event a formatting chord claims
/// (so a chord recognized by BOTH layers — e.g. `Ctrl+Z` is `Undo` in the formatting
/// keymap AND in [`decode_events`] — fires exactly once, in the formatting pass). A Key
/// event whose `(modifiers, key)` resolves to a [`FormattingCommand`] (respecting the
/// list-conditional Tab gate) is dropped before the plain decode runs. Non-key events
/// (Text, Ime, …) and unclaimed keys pass through unchanged.
pub fn decode_events_excluding_formatting(
    events: &[egui::Event],
    caret_in_list: bool,
) -> Vec<EditAction> {
    let filtered: Vec<egui::Event> = events
        .iter()
        .filter(|ev| {
            if let egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } = ev
            {
                if let Some(cmd) = keymap::resolve_shortcut(modifiers, *key) {
                    // A list-conditional chord outside a list is NOT claimed by the
                    // formatting pass, so it must remain available to the plain decode
                    // (though Tab has no EditAction, this keeps the gate consistent).
                    if keymap::is_list_conditional(&cmd) && !caret_in_list {
                        return true;
                    }
                    return false; // claimed by the formatting pass -> drop here
                }
            }
            true
        })
        .cloned()
        .collect();
    decode_events(&filtered)
}

/// Apply one [`EditAction`] to the edit context, returning `true` when the document or
/// selection changed. Text mutations go through [`apply_transaction`] and are pushed onto
/// the undo manager; caret moves only update the selection.
pub fn apply_action(ctx: &mut EditContext<'_>, action: EditAction) -> bool {
    match action {
        EditAction::Insert(text) => insert_text(ctx, &text),
        EditAction::DeleteBackward => delete(ctx, true),
        EditAction::DeleteForward => delete(ctx, false),
        EditAction::MoveLeft { extend } => move_horizontal(ctx, -1, extend),
        EditAction::MoveRight { extend } => move_horizontal(ctx, 1, extend),
        EditAction::MoveHome { extend } => move_boundary(ctx, true, extend),
        EditAction::MoveEnd { extend } => move_boundary(ctx, false, extend),
        EditAction::SelectAll => select_all(ctx),
        EditAction::Undo => ctx.undo.undo(ctx.doc).unwrap_or(false),
        EditAction::Redo => ctx.undo.redo(ctx.doc).unwrap_or(false),
    }
}

/// Decode the formatting/structural chords from this frame's events into ordered
/// [`FormattingCommand`]s via the MT-013 [`keymap`]. Pure mapping (no doc mutation), so
/// it is unit-testable without a live egui context. The widget calls this BEFORE the
/// plain text/nav decode (MT impl note 3) so a chord like `Ctrl+B` toggles bold instead
/// of inserting "b", and Enter splits the block instead of inserting a newline char.
///
/// `caret_in_list` lets the caller suppress the list-conditional Tab/Shift+Tab chords
/// when the caret is NOT in a list (so Tab keeps its focus-traversal behavior outside a
/// list — RISK-4 / MC-004); the widget passes the live list-context flag.
pub fn decode_formatting_commands(
    events: &[egui::Event],
    caret_in_list: bool,
) -> Vec<FormattingCommand> {
    let mut out = Vec::new();
    for ev in events {
        if let egui::Event::Key {
            key,
            pressed: true,
            modifiers,
            ..
        } = ev
        {
            if let Some(cmd) = keymap::resolve_shortcut(modifiers, *key) {
                // Suppress list-conditional indent/dedent outside a list (Tab traverses
                // focus there instead of indenting).
                if keymap::is_list_conditional(&cmd) && !caret_in_list {
                    continue;
                }
                out.push(cmd);
            }
        }
    }
    out
}

/// Apply a decoded [`FormattingCommand`] through the MT-013 command layer, returning
/// `true` when the doc/selection changed. The command builds + applies a `Transaction`
/// and pushes the undo receipt itself. `MergeBackward` is the one chord that may be a
/// benign no-op (caret not at block start) — its return reflects whether anything moved.
pub fn apply_formatting_command(ctx: &mut EditContext<'_>, cmd: &FormattingCommand) -> bool {
    let before_doc_ptr = ctx.doc.clone();
    let before_sel = ctx.selection.clone();
    let mut cctx = CommandContext::new(ctx.doc, ctx.undo, ctx.selection, ctx.actor_id);
    let ran = commands::dispatch(&mut cctx, cmd).is_ok();
    // A command that errored (e.g. a context guard) is reported as "no change". A command
    // that ran is a change iff the doc or selection actually moved (MergeBackward at a
    // non-boundary is an Ok no-op).
    ran && (before_doc_ptr != *ctx.doc || before_sel != *ctx.selection)
}

/// True when the caret sits inside a list (`bullet_list`/`ordered_list`) or a list/task
/// item — used to gate the Tab/Shift+Tab indent chords (RISK-4 / MC-004). Walks the
/// caret's block-path prefix looking for a list/list-item ancestor.
pub fn caret_in_list(doc: &BlockNode, selection: &Selection) -> bool {
    let Selection::Text { head, .. } = selection else {
        return false;
    };
    if head.path.is_empty() {
        return false;
    }
    let block_path = &head.path[..head.path.len() - 1];
    let mut node = doc;
    for &idx in block_path {
        let Some(next) = node.children.get(idx).and_then(Child::as_block) else {
            return false;
        };
        node = next;
        if matches!(
            node.kind,
            NodeKind::BulletList | NodeKind::OrderedList | NodeKind::ListItem | NodeKind::TaskItem
        ) {
            return true;
        }
    }
    false
}

/// The caret head position (collapsed selections use head==anchor). For a node selection
/// (no text caret) we fall back to the doc start.
fn head(ctx: &EditContext<'_>) -> DocPosition {
    match &*ctx.selection {
        Selection::Text { head, .. } => head.clone(),
        Selection::Node { .. } => DocPosition::new(vec![0, 0], 0),
    }
}

/// The char length of the text leaf addressed by `pos.path` (its LAST element addresses
/// the leaf). Returns 0 when the path does not resolve to a text leaf (defensive — the
/// caret-bound clamp then keeps offsets at 0).
fn leaf_len(doc: &BlockNode, pos: &DocPosition) -> usize {
    resolve_leaf(doc, &pos.path).map(|l| l.text.len_chars()).unwrap_or(0)
}

/// Resolve a `path` (block child indices then a final text-leaf index) to a shared
/// [`crate::rich_editor::document_model::node::TextLeaf`] reference.
fn resolve_leaf<'a>(
    doc: &'a BlockNode,
    path: &[usize],
) -> Option<&'a crate::rich_editor::document_model::node::TextLeaf> {
    let (leaf_idx, block_path) = path.split_last()?;
    let mut node = doc;
    for &idx in block_path {
        node = node.children.get(idx)?.as_block()?;
    }
    node.children.get(*leaf_idx)?.as_text()
}

/// Insert `text` at the caret, then move the caret to just after the inserted text. The
/// offset is validated against the leaf length at this layer (caret-bound discipline).
fn insert_text(ctx: &mut EditContext<'_>, text: &str) -> bool {
    if text.is_empty() {
        return false;
    }
    let pos = head(ctx);
    let len = leaf_len(ctx.doc, &pos);
    let offset = pos.char_offset.min(len);
    let tx = Transaction::new(
        vec![Step::InsertText {
            path: pos.path.clone(),
            char_offset: offset,
            text: text.to_string(),
        }],
        crate::rich_editor::document_model::transform::ActorKind::Operator,
        ctx.actor_id,
    );
    match apply_transaction(ctx.doc, tx) {
        Ok(receipt) => {
            ctx.undo.push(receipt);
            let new_off = offset + text.chars().count();
            *ctx.selection = Selection::caret(DocPosition::new(pos.path, new_off));
            true
        }
        Err(_) => false,
    }
}

/// Delete one char backward (Backspace) or forward (Delete) from the caret. A backward
/// delete at block offset 0 now routes to the MT-013 structural `merge_backward` command
/// (merge this block into the previous sibling) — closing MT-012's "Backspace at offset 0
/// is a no-op" deferral. A forward delete at the leaf end is still a no-op here (forward
/// cross-block merge is a later E2 pass).
fn delete(ctx: &mut EditContext<'_>, backward: bool) -> bool {
    let pos = head(ctx);
    let len = leaf_len(ctx.doc, &pos);
    let offset = pos.char_offset.min(len);
    let (start, end, new_off) = if backward {
        if offset == 0 {
            // At the very start of the block's text -> structural merge into the previous
            // sibling (MT-013 scope expansion), via the command layer (which sets the
            // post-merge caret at the join point). A no-op when there is no previous
            // sibling / not at a block boundary.
            return apply_formatting_command(ctx, &FormattingCommand::MergeBackward);
        }
        (offset - 1, offset, offset - 1)
    } else {
        if offset >= len {
            return false;
        }
        (offset, offset + 1, offset)
    };
    let tx = Transaction::new(
        vec![Step::DeleteText { path: pos.path.clone(), start, end }],
        crate::rich_editor::document_model::transform::ActorKind::Operator,
        ctx.actor_id,
    );
    match apply_transaction(ctx.doc, tx) {
        Ok(receipt) => {
            ctx.undo.push(receipt);
            *ctx.selection = Selection::caret(DocPosition::new(pos.path, new_off));
            true
        }
        Err(_) => false,
    }
}

/// Move the caret horizontally by `delta` chars within its leaf, clamped to
/// `0..=leaf_len`. `extend` keeps the anchor (Shift+Arrow builds a range); otherwise the
/// selection collapses to the new head. Cross-leaf motion is out of the vertical-slice
/// scope: the caret clamps at the leaf boundary.
fn move_horizontal(ctx: &mut EditContext<'_>, delta: i64, extend: bool) -> bool {
    let pos = head(ctx);
    let len = leaf_len(ctx.doc, &pos);
    let cur = pos.char_offset.min(len) as i64;
    let next = (cur + delta).clamp(0, len as i64) as usize;
    if next == pos.char_offset && !extend {
        // Already collapsed at the boundary; nothing visible changes.
        if matches!(ctx.selection, Selection::Text { anchor, head } if anchor == head) {
            return false;
        }
    }
    set_head(ctx, DocPosition::new(pos.path, next), extend);
    true
}

/// Move the caret to the start (`to_start`) or end of its leaf (Home/End in the
/// single-paragraph vertical slice = leaf boundary). `extend` keeps the anchor.
fn move_boundary(ctx: &mut EditContext<'_>, to_start: bool, extend: bool) -> bool {
    let pos = head(ctx);
    let len = leaf_len(ctx.doc, &pos);
    let target = if to_start { 0 } else { len };
    set_head(ctx, DocPosition::new(pos.path, target), extend);
    true
}

/// Select the whole current leaf (Ctrl+A vertical-slice scope): anchor at offset 0, head
/// at the leaf end.
fn select_all(ctx: &mut EditContext<'_>) -> bool {
    let pos = head(ctx);
    let len = leaf_len(ctx.doc, &pos);
    *ctx.selection = Selection::text(
        DocPosition::new(pos.path.clone(), 0),
        DocPosition::new(pos.path, len),
    );
    true
}

/// Set the caret head, either extending the current anchor (`extend`) or collapsing to a
/// caret at the new head.
fn set_head(ctx: &mut EditContext<'_>, new_head: DocPosition, extend: bool) {
    if extend {
        let anchor = match ctx.selection {
            Selection::Text { anchor, .. } => anchor.clone(),
            Selection::Node { .. } => new_head.clone(),
        };
        *ctx.selection = Selection::text(anchor, new_head);
    } else {
        *ctx.selection = Selection::caret(new_head);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rich_editor::document_model::node::{BlockNode, Child, TextLeaf};

    fn doc_hello() -> BlockNode {
        BlockNode::doc(vec![BlockNode::paragraph("Hello")])
    }

    fn ctx_at<'a>(
        doc: &'a mut BlockNode,
        sel: &'a mut Selection,
        undo: &'a mut UndoManager,
    ) -> EditContext<'a> {
        EditContext { doc, selection: sel, undo, actor_id: "operator" }
    }

    fn leaf_text(doc: &BlockNode) -> String {
        doc.children[0].as_block().unwrap().children[0].as_text().unwrap().text.to_string()
    }

    #[test]
    fn decode_printable_char_to_insert() {
        let ev = egui::Event::Text("a".into());
        assert_eq!(decode_events(&[ev]), vec![EditAction::Insert("a".into())]);
    }

    #[test]
    fn decode_editing_and_nav_keys() {
        let mk = |key, shift, ctrl| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers { shift, ctrl, command: ctrl, ..Default::default() },
        };
        let events = vec![
            mk(egui::Key::Backspace, false, false),
            mk(egui::Key::Delete, false, false),
            mk(egui::Key::ArrowLeft, false, false),
            mk(egui::Key::ArrowRight, true, false),
            mk(egui::Key::Home, false, false),
            mk(egui::Key::End, true, false),
            mk(egui::Key::A, false, true),
            mk(egui::Key::Z, false, true),
            mk(egui::Key::Z, true, true),
        ];
        assert_eq!(
            decode_events(&events),
            vec![
                EditAction::DeleteBackward,
                EditAction::DeleteForward,
                EditAction::MoveLeft { extend: false },
                EditAction::MoveRight { extend: true },
                EditAction::MoveHome { extend: false },
                EditAction::MoveEnd { extend: true },
                EditAction::SelectAll,
                EditAction::Undo,
                EditAction::Redo,
            ]
        );
    }

    #[test]
    fn typing_inserts_at_caret_and_advances() {
        // AC-2: typing ASCII inserts at caret and the paragraph holds the new text.
        let mut doc = doc_hello();
        let mut sel = Selection::caret(DocPosition::new(vec![0, 0], 5)); // end of "Hello"
        let mut undo = UndoManager::new();
        let mut ctx = ctx_at(&mut doc, &mut sel, &mut undo);
        assert!(apply_action(&mut ctx, EditAction::Insert("!".into())));
        assert_eq!(leaf_text(&doc), "Hello!");
        // Caret advanced past the inserted char.
        if let Selection::Text { head, .. } = &sel {
            assert_eq!(head.char_offset, 6);
        } else {
            panic!("expected a caret");
        }
    }

    #[test]
    fn backspace_deletes_left_of_caret() {
        // AC-3: backspace at offset > 0 deletes the char to the left.
        let mut doc = doc_hello();
        let mut sel = Selection::caret(DocPosition::new(vec![0, 0], 5));
        let mut undo = UndoManager::new();
        let mut ctx = ctx_at(&mut doc, &mut sel, &mut undo);
        assert!(apply_action(&mut ctx, EditAction::DeleteBackward));
        assert_eq!(leaf_text(&doc), "Hell");
        if let Selection::Text { head, .. } = &sel {
            assert_eq!(head.char_offset, 4);
        }
        // Backspace at offset 0 is a no-op in the vertical slice.
        let mut sel0 = Selection::caret(DocPosition::new(vec![0, 0], 0));
        let mut ctx0 = ctx_at(&mut doc, &mut sel0, &mut undo);
        assert!(!apply_action(&mut ctx0, EditAction::DeleteBackward));
        assert_eq!(leaf_text(&doc), "Hell");
    }

    #[test]
    fn delete_forward_removes_right_of_caret() {
        let mut doc = doc_hello();
        let mut sel = Selection::caret(DocPosition::new(vec![0, 0], 0));
        let mut undo = UndoManager::new();
        let mut ctx = ctx_at(&mut doc, &mut sel, &mut undo);
        assert!(apply_action(&mut ctx, EditAction::DeleteForward));
        assert_eq!(leaf_text(&doc), "ello");
    }

    #[test]
    fn arrows_move_caret_and_clamp() {
        // AC-4: arrows move the caret one char; clamps at the leaf boundary.
        let mut doc = doc_hello();
        let mut sel = Selection::caret(DocPosition::new(vec![0, 0], 2));
        let mut undo = UndoManager::new();
        let mut ctx = ctx_at(&mut doc, &mut sel, &mut undo);
        apply_action(&mut ctx, EditAction::MoveRight { extend: false });
        assert!(matches!(&sel, Selection::Text { head, .. } if head.char_offset == 3));
        let mut ctx = ctx_at(&mut doc, &mut sel, &mut undo);
        apply_action(&mut ctx, EditAction::MoveLeft { extend: false });
        assert!(matches!(&sel, Selection::Text { head, .. } if head.char_offset == 2));
        // Clamp at start.
        let mut sel0 = Selection::caret(DocPosition::new(vec![0, 0], 0));
        let mut ctx0 = ctx_at(&mut doc, &mut sel0, &mut undo);
        apply_action(&mut ctx0, EditAction::MoveLeft { extend: false });
        assert!(matches!(&sel0, Selection::Text { head, .. } if head.char_offset == 0));
    }

    #[test]
    fn shift_arrow_extends_selection() {
        let mut doc = doc_hello();
        let mut sel = Selection::caret(DocPosition::new(vec![0, 0], 1));
        let mut undo = UndoManager::new();
        let mut ctx = ctx_at(&mut doc, &mut sel, &mut undo);
        apply_action(&mut ctx, EditAction::MoveRight { extend: true });
        match &sel {
            Selection::Text { anchor, head } => {
                assert_eq!(anchor.char_offset, 1, "anchor stays");
                assert_eq!(head.char_offset, 2, "head extends");
                assert!(!sel.is_collapsed());
            }
            _ => panic!("expected a range"),
        }
    }

    #[test]
    fn ctrl_z_undo_reverts_then_redo_restores() {
        // AC-5: Ctrl+Z undoes the last typed char; Ctrl+Shift+Z redoes it.
        let mut doc = doc_hello();
        let mut sel = Selection::caret(DocPosition::new(vec![0, 0], 5));
        let mut undo = UndoManager::new();
        {
            let mut ctx = ctx_at(&mut doc, &mut sel, &mut undo);
            apply_action(&mut ctx, EditAction::Insert("X".into()));
        }
        assert_eq!(leaf_text(&doc), "HelloX");
        {
            let mut ctx = ctx_at(&mut doc, &mut sel, &mut undo);
            assert!(apply_action(&mut ctx, EditAction::Undo));
        }
        assert_eq!(leaf_text(&doc), "Hello", "undo reverts the typed char");
        {
            let mut ctx = ctx_at(&mut doc, &mut sel, &mut undo);
            assert!(apply_action(&mut ctx, EditAction::Redo));
        }
        assert_eq!(leaf_text(&doc), "HelloX", "redo restores it");
    }

    #[test]
    fn select_all_selects_the_leaf() {
        let mut doc = doc_hello();
        let mut sel = Selection::caret(DocPosition::new(vec![0, 0], 2));
        let mut undo = UndoManager::new();
        let mut ctx = ctx_at(&mut doc, &mut sel, &mut undo);
        apply_action(&mut ctx, EditAction::SelectAll);
        match &sel {
            Selection::Text { anchor, head } => {
                assert_eq!(anchor.char_offset, 0);
                assert_eq!(head.char_offset, 5);
            }
            _ => panic!("expected a range"),
        }
    }

    #[test]
    fn insert_into_empty_leaf_does_not_panic() {
        // RISK empty-leaf: typing into an empty paragraph works.
        let mut doc = BlockNode::doc(vec![BlockNode::with_children(
            crate::rich_editor::document_model::node::NodeKind::Paragraph,
            vec![Child::Text(TextLeaf::new(""))],
        )]);
        let mut sel = Selection::caret(DocPosition::new(vec![0, 0], 0));
        let mut undo = UndoManager::new();
        let mut ctx = ctx_at(&mut doc, &mut sel, &mut undo);
        assert!(apply_action(&mut ctx, EditAction::Insert("a".into())));
        assert_eq!(leaf_text(&doc), "a");
    }
}
