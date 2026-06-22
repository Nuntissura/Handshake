//! IME composition handling via `egui::Event::Ime` (WP-KERNEL-012 MT-012).
//!
//! ## egui IME, NOT raw winit (contract KERNEL_BUILDER gate)
//!
//! The contract OVERRIDES its own scope text: do NOT add `winit` as a direct dep and do
//! NOT handle raw `winit::event::Ime` — that fights eframe's event loop. egui already
//! surfaces winit IME through the egui-winit bridge as
//! [`egui::ImeEvent`] (`Enabled | Preedit(String) | Commit(String) | Disabled`) inside
//! `egui::Event::Ime`, read from `ui.input()`. The shell enables IME via the
//! eframe/egui-winit setup (`set_ime_allowed`) — documented as a shell contract; this
//! module consumes `egui::Event::Ime`, never winit.
//!
//! ## egui-0.33 shape note (verified deviation from the contract text)
//!
//! The contract text models `Ime::Preedit(text, cursor_range)`. The egui 0.33 bridge
//! variant is `ImeEvent::Preedit(String)` with NO cursor range (verified in
//! `egui-0.33.3/src/data/input.rs:570`). The preedit caret is therefore rendered at the
//! END of the preedit run (the standard composition caret position), which is the
//! field-correct behavior for the egui shape.
//!
//! ## Double-insert guard (red-team RISK-2 / MC-002)
//!
//! The handler holds a [`PreeditState`] describing the inline region currently showing
//! the in-progress composition. On `Commit`, it CLEARS that preedit region BEFORE
//! inserting the committed text, so the preedit characters and the commit are NEVER both
//! applied (which would corrupt the rope). The `preedit_then_commit_inserts_only_commit`
//! test proves the rope ends with only the committed text.

use crate::rich_editor::document_model::node::BlockNode;
use crate::rich_editor::document_model::position::DocPosition;
use crate::rich_editor::document_model::selection::Selection;
use crate::rich_editor::document_model::transform::{apply_transaction, ActorKind, Step, Transaction};
use crate::rich_editor::document_model::history::UndoManager;

/// The in-progress IME composition region. `anchor` is the caret position where the
/// composition began; `text` is the current preedit string shown (underlined) inline.
/// While composing, the preedit text is NOT part of the committed document model — it is
/// an overlay the renderer draws underlined and the model rope does not yet contain it.
/// This is the key to the double-insert guard: because preedit is overlay-only, a commit
/// simply inserts the committed text at the anchor; there is no preedit text in the rope
/// to remove. If a future pass writes preedit into the rope, [`clear_preedit_region`]
/// removes it before commit.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PreeditState {
    /// Where the composition started (the commit insert point).
    pub anchor: Option<DocPosition>,
    /// The current preedit (composition) text, shown underlined inline. Empty when no
    /// composition is active.
    pub text: String,
}

impl PreeditState {
    /// True when a composition is in progress (preedit text is showing).
    pub fn is_active(&self) -> bool {
        self.anchor.is_some() && !self.text.is_empty()
    }

    /// Reset to no active composition.
    pub fn clear(&mut self) {
        self.anchor = None;
        self.text.clear();
    }
}

/// The result of feeding one IME event, so the widget can request a repaint / update its
/// caret without re-reading the doc.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImeOutcome {
    /// Composition started or the preedit text changed (redraw the underlined overlay).
    PreeditChanged,
    /// Text was committed into the document (the rope changed; push undo done).
    Committed,
    /// Composition ended with no commit (preedit cleared).
    Cleared,
    /// The event was not an IME event we handle (no-op).
    Ignored,
}

/// The mutable state an IME pass drives.
pub struct ImeContext<'a> {
    pub doc: &'a mut BlockNode,
    pub selection: &'a mut Selection,
    pub undo: &'a mut UndoManager,
    pub preedit: &'a mut PreeditState,
    pub actor_id: &'a str,
}

/// The caret head position used as the composition anchor / commit insert point.
fn head(sel: &Selection) -> DocPosition {
    match sel {
        Selection::Text { head, .. } => head.clone(),
        Selection::Node { .. } => DocPosition::new(vec![0, 0], 0),
    }
}

/// Handle one [`egui::ImeEvent`].
///
/// - `Enabled`  -> begin a composition anchored at the caret (no rope change).
/// - `Preedit(s)` -> update the overlay preedit text (no rope change; double-insert safe).
/// - `Commit(s)`  -> CLEAR the preedit region, then insert `s` at the anchor, push undo,
///   and advance the caret past it.
/// - `Disabled` -> clear the composition (no rope change).
pub fn handle_ime_event(ctx: &mut ImeContext<'_>, event: &egui::ImeEvent) -> ImeOutcome {
    match event {
        egui::ImeEvent::Enabled => {
            ctx.preedit.anchor = Some(head(ctx.selection));
            ctx.preedit.text.clear();
            ImeOutcome::PreeditChanged
        }
        egui::ImeEvent::Preedit(s) => {
            if ctx.preedit.anchor.is_none() {
                // A Preedit without a preceding Enabled (some platforms): anchor now.
                ctx.preedit.anchor = Some(head(ctx.selection));
            }
            ctx.preedit.text = s.clone();
            ImeOutcome::PreeditChanged
        }
        egui::ImeEvent::Commit(s) => {
            // RISK-2 / MC-002: clear the preedit region BEFORE applying the commit so the
            // preedit characters and the commit can never both land. Preedit is
            // overlay-only here, so clearing is just dropping the overlay; the helper is
            // defensive for a future in-rope preedit.
            let anchor = clear_preedit_region(ctx);
            commit_text(ctx, &anchor, s)
        }
        egui::ImeEvent::Disabled => {
            ctx.preedit.clear();
            ImeOutcome::Cleared
        }
    }
}

/// Clear the current preedit overlay and return the composition anchor (the commit insert
/// point). If preedit text had been written into the rope (it is not in this MT), this is
/// where it would be removed first. Returns the caret head when no anchor was recorded.
fn clear_preedit_region(ctx: &mut ImeContext<'_>) -> DocPosition {
    let anchor = ctx.preedit.anchor.clone().unwrap_or_else(|| head(ctx.selection));
    ctx.preedit.clear();
    anchor
}

/// Insert the committed `text` at `anchor`, push the undo receipt, and collapse the caret
/// just after it. The offset is clamped to the leaf length (caret-bound discipline).
fn commit_text(ctx: &mut ImeContext<'_>, anchor: &DocPosition, text: &str) -> ImeOutcome {
    if text.is_empty() {
        return ImeOutcome::Cleared;
    }
    let len = leaf_len(ctx.doc, anchor);
    let offset = anchor.char_offset.min(len);
    let tx = Transaction::new(
        vec![Step::InsertText {
            path: anchor.path.clone(),
            char_offset: offset,
            text: text.to_string(),
        }],
        ActorKind::Operator,
        ctx.actor_id,
    );
    match apply_transaction(ctx.doc, tx) {
        Ok(receipt) => {
            ctx.undo.push(receipt);
            let new_off = offset + text.chars().count();
            *ctx.selection = Selection::caret(DocPosition::new(anchor.path.clone(), new_off));
            ImeOutcome::Committed
        }
        Err(_) => ImeOutcome::Cleared,
    }
}

/// The char length of the text leaf addressed by `pos.path`.
fn leaf_len(doc: &BlockNode, pos: &DocPosition) -> usize {
    let Some((leaf_idx, block_path)) = pos.path.split_last() else {
        return 0;
    };
    let mut node = doc;
    for &idx in block_path {
        match node.children.get(idx).and_then(|c| c.as_block()) {
            Some(b) => node = b,
            None => return 0,
        }
    }
    node.children
        .get(*leaf_idx)
        .and_then(|c| c.as_text())
        .map(|l| l.text.len_chars())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rich_editor::document_model::node::BlockNode;

    fn doc_hi() -> BlockNode {
        BlockNode::doc(vec![BlockNode::paragraph("Hi")])
    }

    fn leaf_text(doc: &BlockNode) -> String {
        doc.children[0].as_block().unwrap().children[0].as_text().unwrap().text.to_string()
    }

    #[test]
    fn preedit_then_commit_inserts_only_commit() {
        // AC-6 / RISK-2 / MC-002: simulate Preedit("nihao") then Commit("你好"); the rope
        // must hold ONLY the committed text appended at the caret, never the preedit.
        let mut doc = doc_hi();
        let mut sel = Selection::caret(DocPosition::new(vec![0, 0], 2)); // end of "Hi"
        let mut undo = UndoManager::new();
        let mut pre = PreeditState::default();
        {
            let mut ctx = ImeContext {
                doc: &mut doc,
                selection: &mut sel,
                undo: &mut undo,
                preedit: &mut pre,
                actor_id: "operator",
            };
            assert_eq!(handle_ime_event(&mut ctx, &egui::ImeEvent::Enabled), ImeOutcome::PreeditChanged);
            assert_eq!(
                handle_ime_event(&mut ctx, &egui::ImeEvent::Preedit("nihao".into())),
                ImeOutcome::PreeditChanged
            );
            assert!(ctx.preedit.is_active(), "preedit overlay is showing");
            // Rope is UNCHANGED while composing (preedit is overlay-only).
            assert_eq!(leaf_text(ctx.doc), "Hi");
            assert_eq!(
                handle_ime_event(&mut ctx, &egui::ImeEvent::Commit("你好".into())),
                ImeOutcome::Committed
            );
        }
        // Only the committed text landed; the preedit "nihao" was never inserted.
        assert_eq!(leaf_text(&doc), "Hi你好");
        assert!(!pre.is_active(), "preedit cleared after commit");
        // CJK char-index discipline: caret advanced by 2 CHARS (not bytes).
        if let Selection::Text { head, .. } = &sel {
            assert_eq!(head.char_offset, 4, "caret advanced by 2 committed chars");
        }
    }

    #[test]
    fn disabled_clears_preedit_without_rope_change() {
        let mut doc = doc_hi();
        let mut sel = Selection::caret(DocPosition::new(vec![0, 0], 2));
        let mut undo = UndoManager::new();
        let mut pre = PreeditState::default();
        let mut ctx = ImeContext {
            doc: &mut doc,
            selection: &mut sel,
            undo: &mut undo,
            preedit: &mut pre,
            actor_id: "operator",
        };
        handle_ime_event(&mut ctx, &egui::ImeEvent::Enabled);
        handle_ime_event(&mut ctx, &egui::ImeEvent::Preedit("abc".into()));
        assert_eq!(handle_ime_event(&mut ctx, &egui::ImeEvent::Disabled), ImeOutcome::Cleared);
        assert!(!pre.is_active());
        assert_eq!(leaf_text(&doc), "Hi", "no rope change from a cancelled composition");
    }

    #[test]
    fn commit_undo_reverts_committed_text() {
        let mut doc = doc_hi();
        let mut sel = Selection::caret(DocPosition::new(vec![0, 0], 2));
        let mut undo = UndoManager::new();
        let mut pre = PreeditState::default();
        {
            let mut ctx = ImeContext {
                doc: &mut doc,
                selection: &mut sel,
                undo: &mut undo,
                preedit: &mut pre,
                actor_id: "operator",
            };
            handle_ime_event(&mut ctx, &egui::ImeEvent::Commit("X".into()));
        }
        assert_eq!(leaf_text(&doc), "HiX");
        // The commit is a normal transaction on the undo stack.
        assert!(undo.undo(&mut doc).unwrap());
        assert_eq!(leaf_text(&doc), "Hi");
    }
}
