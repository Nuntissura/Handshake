//! Slash-command executor: dispatch a selected command to the right MT handler
//! (WP-KERNEL-012 MT-016).
//!
//! [`execute_slash_command`] is the single entry the menu calls when the operator selects a
//! command (Enter or click). It:
//!   1. REMOVES the `/`+filter trigger text from the doc, computing the delete range from the
//!      SNAPSHOTTED trigger position + filter length in [`super::SlashMenuState`] — NOT a
//!      live (possibly stale) selection (red-team RISK-3 / MC-003);
//!   2. dispatches the command's [`super::registry::SlashAction`] to the correct existing
//!      handler:
//!      - `SetBlock`  -> MT-013 `formatting::commands::set_block_kind` (via `dispatch`),
//!      - `InsertNode(block)` -> a transactional MT-011 `Step::InsertNode` (undoable),
//!      - `OpenEmbedPrompt` / `OpenTransclusionPrompt` / `OpenManualInsertPrompt` -> open a
//!        modal prompt (the deferred-confirm path returns [`SlashExecOutcome::OpenPrompt`]),
//!      - `OpenWikilinkAutocomplete` -> return [`SlashExecOutcome::OpenWikilinkAutocomplete`]
//!        so the widget activates the MT-015 autocomplete state at the caret,
//!      - `InsertTemplate` -> parse the const DocJson snippet and insert its blocks.
//!
//! ## Transactionality split (KERNEL_BUILDER gate impl note)
//!
//! `SetBlock` and `InsertNode(block)` go through the MT-011 transaction system (atomic +
//! undoable). The embed/transclusion/manual atom inserts reuse the MT-014/MT-015 inline-atom
//! DIRECT-children-mutation pattern (`Child::HsLink` / `Child::Transclusion`), which is NOT
//! a transform `Step` (MT-011 has no inline-atom insert step, and `transform.rs` is out of
//! this MT's `allowed_paths`). The KERNEL_BUILDER gate explicitly carries that
//! transform-completeness fix to MT-020; MT-016 reuses the existing atom-insert logic as-is.
//!
//! The trigger-text removal is ALWAYS a transactional `DeleteText` (it edits a text leaf, a
//! shape MT-011 fully supports), so the `/`+filter removal is undoable even for the atom paths.

use crate::rich_editor::document_model::history::UndoManager;
use crate::rich_editor::document_model::node::{
    BlockNode, Child, HsLinkNode, NodeKind, TransclusionNode,
};
use crate::rich_editor::document_model::position::DocPosition;
use crate::rich_editor::document_model::selection::Selection;
use crate::rich_editor::document_model::transform::{
    apply_transaction, ActorKind, Step, Transaction,
};
use crate::rich_editor::formatting::commands::{self, CommandContext};

use super::registry::{EmbedKind, SlashAction, SlashCommand, TemplateId};
use super::{SlashMenuState, SlashPrompt, SlashPromptKind};

/// What the widget must do after [`execute_slash_command`] runs. Most commands complete the
/// edit immediately (`Done`); the prompt commands need a follow-up modal (`OpenPrompt`); the
/// wikilink command activates the MT-015 autocomplete (`OpenWikilinkAutocomplete`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlashExecOutcome {
    /// The command ran and the menu should close. Carries `true` when the doc actually
    /// changed (so a no-op guard-rejected command is reported honestly).
    Done { changed: bool },
    /// A prompt modal must open with this state (embed/transclusion/manual insert). The
    /// `/`+filter trigger text has ALREADY been removed; the menu's list is hidden while the
    /// prompt is active (the menu stays `Some` carrying the prompt).
    OpenPrompt(SlashPrompt),
    /// Activate the MT-015 wikilink autocomplete at the caret. The `/`+filter trigger text
    /// has already been removed and a `[[` has been inserted so the autocomplete opens. The
    /// widget closes the slash menu and lets the existing autocomplete refresh take over.
    OpenWikilinkAutocomplete,
}

/// The mutable editor state the executor drives (the same borrow shape the formatting command
/// layer + input handler use). `actor_id` threads transaction provenance.
pub struct SlashExecContext<'a> {
    /// The document being edited (the `doc` root).
    pub doc: &'a mut BlockNode,
    /// The undo/redo history (receipts pushed on each transactional step).
    pub history: &'a mut UndoManager,
    /// The current selection; the executor moves the caret after inserting.
    pub selection: &'a mut Selection,
    /// Actor id for transaction provenance.
    pub actor_id: &'a str,
}

/// Execute the selected `command` against the editor state, having first removed the
/// `/`+filter trigger text using the snapshot in `menu`. Returns the [`SlashExecOutcome`] the
/// widget acts on.
pub fn execute_slash_command(
    ctx: &mut SlashExecContext<'_>,
    menu: &SlashMenuState,
    command: &SlashCommand,
) -> SlashExecOutcome {
    // 1) Remove the `/`+filter trigger text (RISK-3 snapshot): delete
    //    [trigger_char, trigger_char + trigger_delete_len) from the trigger leaf. This is a
    //    transactional DeleteText so it is undoable. After removal the caret sits at
    //    trigger_char (where the `/` was), which is where new content lands.
    let removed = remove_trigger_text_tx(ctx, menu);

    // 2) Dispatch the action.
    match command.action {
        SlashAction::SetParagraph => {
            let changed = set_block_command(ctx, commands::FormattingCommand::SetParagraph);
            SlashExecOutcome::Done {
                changed: changed || removed,
            }
        }
        SlashAction::SetHeading(level) => {
            let changed = set_block_command(ctx, commands::FormattingCommand::SetHeading(level));
            SlashExecOutcome::Done {
                changed: changed || removed,
            }
        }
        SlashAction::InsertNode(builder) => {
            let node = builder();
            let changed = insert_block_after_caret(ctx, node);
            SlashExecOutcome::Done {
                changed: changed || removed,
            }
        }
        SlashAction::OpenEmbedPrompt(kind) => {
            SlashExecOutcome::OpenPrompt(SlashPrompt::new(SlashPromptKind::Embed(kind)))
        }
        SlashAction::OpenTransclusionPrompt => {
            SlashExecOutcome::OpenPrompt(SlashPrompt::new(SlashPromptKind::Transclusion))
        }
        SlashAction::OpenManualInsertPrompt => {
            SlashExecOutcome::OpenPrompt(SlashPrompt::new(SlashPromptKind::ManualInsert))
        }
        SlashAction::OpenWikilinkAutocomplete => {
            // Insert a `[[` at the caret so the MT-015 autocomplete trigger fires; the widget
            // then opens the autocomplete popup (reusing MT-015's refresh path). This is a
            // transactional InsertText (a text-leaf edit MT-011 supports).
            insert_text_at_caret(ctx, "[[");
            SlashExecOutcome::OpenWikilinkAutocomplete
        }
        SlashAction::InsertTemplate(tid) => {
            let changed = insert_template(ctx, tid);
            SlashExecOutcome::Done {
                changed: changed || removed,
            }
        }
    }
}

/// Confirm a prompt modal's input into the document (the deferred path for
/// embed/transclusion/manual insert). Returns `true` when the insert happened (a blank /
/// invalid input is a no-op that the caller surfaces as a still-open prompt or a close).
pub fn confirm_prompt(ctx: &mut SlashExecContext<'_>, prompt: &SlashPrompt) -> bool {
    let trimmed = prompt.input.trim();
    if trimmed.is_empty() {
        return false; // blank input -> nothing inserted (caller keeps/abandons the modal).
    }
    match prompt.kind {
        SlashPromptKind::Embed(kind) => insert_embed_atom(ctx, kind, trimmed),
        SlashPromptKind::Transclusion => insert_transclusion_atom(ctx, trimmed),
        SlashPromptKind::ManualInsert => insert_manual_node(ctx, trimmed),
    }
}

// ── trigger-text removal (RISK-3 / MC-003) ───────────────────────────────────────────────

/// Remove the `/`+filter trigger text from the trigger leaf, transactionally. Computes the
/// delete range from the SNAPSHOTTED `menu.trigger_char` + `menu.trigger_delete_len()`, NOT
/// from the live selection (so it never deletes more than the operator typed). Places the
/// caret at `trigger_char` (where the `/` was). Returns `true` when a delete was applied.
fn remove_trigger_text_tx(ctx: &mut SlashExecContext<'_>, menu: &SlashMenuState) -> bool {
    let start = menu.trigger_char;
    let end = start + menu.trigger_delete_len();
    // Clamp to the leaf length defensively so a stale snapshot cannot over-delete.
    let leaf_len = leaf_len_at(ctx.doc, &menu.trigger_leaf_path);
    let start = start.min(leaf_len);
    let end = end.min(leaf_len).max(start);
    if end == start {
        // Nothing to remove (e.g. the `/` was already gone); still park the caret at start.
        *ctx.selection = Selection::caret(DocPosition::new(menu.trigger_leaf_path.clone(), start));
        return false;
    }
    let tx = Transaction::new(
        vec![Step::DeleteText {
            path: menu.trigger_leaf_path.clone(),
            start,
            end,
        }],
        ActorKind::Operator,
        ctx.actor_id,
    );
    match apply_transaction(ctx.doc, tx) {
        Ok(receipt) => {
            ctx.history.push(receipt);
            *ctx.selection =
                Selection::caret(DocPosition::new(menu.trigger_leaf_path.clone(), start));
            true
        }
        Err(_) => false,
    }
}

// ── block dispatch (MT-013 reuse) ────────────────────────────────────────────────────────

/// Run a block-conversion MT-013 command (`SetParagraph` / `SetHeading`) against the caret's
/// block (idempotent SET). Returns `true` when the block changed. A guard rejection (e.g. the
/// caret is in a list/table cell where conversion is illegal) is reported as `false`, not a
/// panic. The MT-013 `SetHeading` clamps the level into 1..=3 itself.
fn set_block_command(ctx: &mut SlashExecContext<'_>, cmd: commands::FormattingCommand) -> bool {
    let before = ctx.doc.clone();
    let mut cctx = CommandContext::new(ctx.doc, ctx.history, ctx.selection, ctx.actor_id);
    let ran = commands::dispatch(&mut cctx, &cmd).is_ok();
    ran && before != *ctx.doc
}

/// Insert `node` as a new sibling block immediately AFTER the caret's block, via a
/// transactional MT-011 `Step::InsertNode` (atomic + undoable). Moves the caret to the start
/// of the inserted block's first inline-content leaf when it has one (else leaves it at the
/// trigger position). Returns `true` when the insert applied.
fn insert_block_after_caret(ctx: &mut SlashExecContext<'_>, node: BlockNode) -> bool {
    let Some((parent_path, index)) = caret_block_parent_and_index(ctx.selection) else {
        return false;
    };
    let insert_at = index + 1;
    let inserted_kind = node.kind;
    let tx = Transaction::new(
        vec![Step::InsertNode {
            parent_path: parent_path.clone(),
            index: insert_at,
            node,
        }],
        ActorKind::Operator,
        ctx.actor_id,
    );
    match apply_transaction(ctx.doc, tx) {
        Ok(receipt) => {
            ctx.history.push(receipt);
            move_caret_into_inserted(ctx, &parent_path, insert_at, inserted_kind);
            true
        }
        Err(_) => false,
    }
}

/// Parse a template's const DocJson snippet and insert ALL its top-level blocks after the
/// caret's block, in order, via one transaction (atomic: either all blocks insert or none).
/// Returns `true` when at least one block was inserted.
fn insert_template(ctx: &mut SlashExecContext<'_>, tid: TemplateId) -> bool {
    let doc = match crate::rich_editor::document_model::doc_json::from_json_string(tid.doc_json()) {
        Ok(d) => d,
        Err(_) => return false, // a malformed const template would be a build-time bug.
    };
    let blocks = super::doc_block_children(&doc);
    if blocks.is_empty() {
        return false;
    }
    insert_blocks_after_caret(ctx, blocks)
}

/// Insert a manual raw node JSON: parse it as either a bare `{type:"doc",content:[…]}` (then
/// insert its block children) or a single bare block node (then insert that one block). The
/// advanced swarm-agent surface. Returns `true` when at least one block was inserted; a
/// non-block / unparseable input is a no-op (false).
fn insert_manual_node(ctx: &mut SlashExecContext<'_>, json: &str) -> bool {
    use crate::rich_editor::document_model::doc_json::{from_json_string, from_json_value};
    // Try a full doc first (the model only exposes a doc-rooted parser publicly).
    if let Ok(doc) = from_json_string(json) {
        let blocks = super::doc_block_children(&doc);
        if !blocks.is_empty() {
            return insert_blocks_after_caret(ctx, blocks);
        }
    }
    // Else try wrapping a single bare block node in a doc and parsing that, so an agent can
    // paste a `{type:"paragraph",…}` directly.
    let wrapped = format!(r#"{{"type":"doc","content":[{json}]}}"#);
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&wrapped) {
        if let Ok(doc) = from_json_value(&value) {
            let blocks = super::doc_block_children(&doc);
            if !blocks.is_empty() {
                return insert_blocks_after_caret(ctx, blocks);
            }
        }
    }
    false
}

/// Insert a sequence of blocks after the caret's block in ONE transaction (each at a
/// successive index after the caret), so a multi-block template is atomic + a single undo.
/// Moves the caret into the FIRST inserted block.
fn insert_blocks_after_caret(ctx: &mut SlashExecContext<'_>, blocks: Vec<BlockNode>) -> bool {
    let Some((parent_path, index)) = caret_block_parent_and_index(ctx.selection) else {
        return false;
    };
    let first_kind = blocks.first().map(|b| b.kind);
    let mut steps = Vec::with_capacity(blocks.len());
    for (i, node) in blocks.into_iter().enumerate() {
        steps.push(Step::InsertNode {
            parent_path: parent_path.clone(),
            index: index + 1 + i,
            node,
        });
    }
    if steps.is_empty() {
        return false;
    }
    let tx = Transaction::new(steps, ActorKind::Operator, ctx.actor_id);
    match apply_transaction(ctx.doc, tx) {
        Ok(receipt) => {
            ctx.history.push(receipt);
            if let Some(kind) = first_kind {
                move_caret_into_inserted(ctx, &parent_path, index + 1, kind);
            }
            true
        }
        Err(_) => false,
    }
}

// ── inline-atom inserts (MT-014 / MT-015 reuse — direct children mutation) ───────────────

/// Insert a CKC media-embed `hsLink` atom (the MT-014 node shape: `ref_kind` ∈
/// {images,slideshow,album,video}, `ref_value` = the asset id(s)) as an inline atom after the
/// caret's text leaf, reusing the MT-015 `confirm`-style direct-children insert. The atom is
/// NOT transactional (MT-011 has no inline-atom step; carried to MT-020), but the trigger-text
/// removal already was. Returns `true` when inserted.
fn insert_embed_atom(ctx: &mut SlashExecContext<'_>, kind: EmbedKind, ref_value: &str) -> bool {
    let link = HsLinkNode::new(kind.ref_kind(), ref_value.to_string(), String::new());
    insert_inline_atom(ctx, Child::HsLink(link))
}

/// Insert a `loomTransclusion` atom (MT-015) referencing `ref_value` as an inline atom after
/// the caret's text leaf. Returns `true` when inserted.
fn insert_transclusion_atom(ctx: &mut SlashExecContext<'_>, ref_value: &str) -> bool {
    insert_inline_atom(ctx, Child::Transclusion(TransclusionNode::new(ref_value.to_string())))
}

/// Insert `atom` (an `hsLink` or `loomTransclusion` inline atom) immediately AFTER the caret's
/// text leaf, mirroring `wikilinks::confirm::confirm_wikilink`'s insert mechanics (a paragraph
/// holds a flat list of text runs + inline atoms; insert as a sibling, then ensure a trailing
/// text leaf hosts the caret). Returns `true` when inserted.
fn insert_inline_atom(ctx: &mut SlashExecContext<'_>, atom: Child) -> bool {
    let Selection::Text { head, .. } = ctx.selection.clone() else {
        return false;
    };
    let Some((leaf_idx, parent_path)) = head.path.split_last() else {
        return false;
    };
    let Some(parent) = block_at_mut(ctx.doc, parent_path) else {
        return false;
    };
    if *leaf_idx >= parent.children.len() {
        return false;
    }
    let insert_at = *leaf_idx + 1;
    parent.children.insert(insert_at, atom);
    // Ensure a trailing text leaf hosts the caret (the same rule confirm_wikilink uses).
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
    *ctx.selection = Selection::caret(DocPosition::new(caret_path, 0));
    true
}

/// Insert `text` at the caret transactionally (used by `OpenWikilinkAutocomplete` to type the
/// `[[` that opens the MT-015 autocomplete). Moves the caret past the inserted text.
fn insert_text_at_caret(ctx: &mut SlashExecContext<'_>, text: &str) -> bool {
    let Selection::Text { head, .. } = ctx.selection.clone() else {
        return false;
    };
    let len = leaf_len_at(ctx.doc, &head.path);
    let offset = head.char_offset.min(len);
    let tx = Transaction::new(
        vec![Step::InsertText {
            path: head.path.clone(),
            char_offset: offset,
            text: text.to_string(),
        }],
        ActorKind::Operator,
        ctx.actor_id,
    );
    match apply_transaction(ctx.doc, tx) {
        Ok(receipt) => {
            ctx.history.push(receipt);
            let new_off = offset + text.chars().count();
            *ctx.selection = Selection::caret(DocPosition::new(head.path, new_off));
            true
        }
        Err(_) => false,
    }
}

// ── caret / path helpers ─────────────────────────────────────────────────────────────────

/// The caret block's (parent_path, index) — the parent block path and the caret block's index
/// within it — so an InsertNode can target the position right after the caret block. `None`
/// for a node selection or a too-short path.
fn caret_block_parent_and_index(selection: &Selection) -> Option<(Vec<usize>, usize)> {
    let Selection::Text { head, .. } = selection else {
        return None;
    };
    // The block path is the head path with the final text-leaf index dropped.
    if head.path.len() < 2 {
        // A top-level block leaf path is [block_idx, leaf_idx]; its block path is [block_idx],
        // whose parent is the doc root [] at index block_idx. A shorter path has no caret block.
        if head.path.len() == 1 {
            return Some((Vec::new(), head.path[0]));
        }
        return None;
    }
    let block_path = &head.path[..head.path.len() - 1];
    let (index, parent) = block_path.split_last()?;
    Some((parent.to_vec(), *index))
}

/// After inserting a block at `parent_path[insert_at]`, move the caret to the start of its
/// first inline-content (paragraph/heading) leaf so the operator can immediately type into the
/// new block. For a structural insert (list/table/quote/code) the caret descends into the first
/// editable leaf; for an atom block (horizontal rule) it stays put (no inline content).
fn move_caret_into_inserted(
    ctx: &mut SlashExecContext<'_>,
    parent_path: &[usize],
    insert_at: usize,
    kind: NodeKind,
) {
    // A horizontal rule (atom) has no caret interior — leave the caret where the trigger was.
    if kind.is_atom() {
        return;
    }
    let mut block_path = parent_path.to_vec();
    block_path.push(insert_at);
    if let Some(caret_path) = first_inline_leaf_path(ctx.doc, &block_path) {
        *ctx.selection = Selection::caret(DocPosition::new(caret_path, 0));
    }
}

/// Walk into the block at `block_path` to find the path of its FIRST inline-content text leaf
/// (descending through list/table/quote containers to the first paragraph/heading's text leaf).
/// Returns the full leaf path, or `None` when the block has no inline-content descendant.
fn first_inline_leaf_path(doc: &BlockNode, block_path: &[usize]) -> Option<Vec<usize>> {
    let block = block_at(doc, block_path)?;
    descend_to_inline_leaf(block, block_path.to_vec())
}

/// Recursive helper for [`first_inline_leaf_path`].
fn descend_to_inline_leaf(block: &BlockNode, mut path: Vec<usize>) -> Option<Vec<usize>> {
    if block.kind.holds_inline_content() {
        // Find the first text-leaf child.
        for (i, child) in block.children.iter().enumerate() {
            if child.as_text().is_some() {
                path.push(i);
                return Some(path);
            }
        }
        return None;
    }
    // A container: descend into the first block child.
    for (i, child) in block.children.iter().enumerate() {
        if let Some(b) = child.as_block() {
            let mut child_path = path.clone();
            child_path.push(i);
            if let Some(found) = descend_to_inline_leaf(b, child_path) {
                return Some(found);
            }
        }
    }
    None
}

/// The char length of the text leaf addressed by `path` (its last element is the leaf index).
fn leaf_len_at(doc: &BlockNode, path: &[usize]) -> usize {
    let Some((leaf_idx, block_path)) = path.split_last() else {
        return 0;
    };
    let Some(block) = block_at(doc, block_path) else {
        return 0;
    };
    block
        .children
        .get(*leaf_idx)
        .and_then(Child::as_text)
        .map(|t| t.text.len_chars())
        .unwrap_or(0)
}

/// Resolve a block `path` to a shared block reference (empty path = the doc root).
fn block_at<'a>(doc: &'a BlockNode, path: &[usize]) -> Option<&'a BlockNode> {
    let mut node = doc;
    for &idx in path {
        node = node.children.get(idx)?.as_block()?;
    }
    Some(node)
}

/// Resolve a block `path` to a mutable block reference (empty path = the doc root).
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
    use crate::rich_editor::document_model::node::{HeadingLevel, TextLeaf};
    use crate::rich_editor::slash_commands::registry::{filter_slash_commands, SLASH_COMMANDS};

    /// A fresh `doc > paragraph(text)` plus a caret at `caret` in that paragraph's leaf, and an
    /// empty undo manager.
    fn fixture(text: &str, caret: usize) -> (BlockNode, UndoManager, Selection) {
        let doc = BlockNode::doc(vec![BlockNode::paragraph(text)]);
        let undo = UndoManager::new();
        let sel = Selection::caret(DocPosition::new(vec![0, 0], caret));
        (doc, undo, sel)
    }

    fn ctx<'a>(
        doc: &'a mut BlockNode,
        undo: &'a mut UndoManager,
        sel: &'a mut Selection,
    ) -> SlashExecContext<'a> {
        SlashExecContext { doc, history: undo, selection: sel, actor_id: "operator" }
    }

    fn cmd_by_id(id: &str) -> &'static SlashCommand {
        SLASH_COMMANDS.iter().find(|c| c.id == id).expect("command id exists")
    }

    fn para_text(doc: &BlockNode, idx: usize) -> String {
        doc.children[idx]
            .as_block()
            .unwrap()
            .children
            .iter()
            .filter_map(Child::as_text)
            .map(|t| t.text.to_string())
            .collect()
    }

    #[test]
    fn heading2_insertion() {
        // PT-4 / AC-4: selecting "Heading 2" sets the current paragraph to Heading(2) AND
        // removes the `/head` trigger text. Document: "/head" (caret at 5).
        let (mut doc, mut undo, mut sel) = fixture("/head", 5);
        let menu = SlashMenuState {
            trigger_leaf_path: vec![0, 0],
            trigger_char: 0,
            filter: "head".to_string(),
            selected: 0,
            prompt: None,
        };
        let cmd = cmd_by_id("heading-2");
        let outcome = execute_slash_command(&mut ctx(&mut doc, &mut undo, &mut sel), &menu, cmd);
        assert!(matches!(outcome, SlashExecOutcome::Done { changed: true }));
        // The block is now a Heading(2) and the `/head` text is gone.
        let block = doc.children[0].as_block().unwrap();
        assert_eq!(block.kind, NodeKind::Heading(HeadingLevel::new(2)));
        assert_eq!(para_text(&doc, 0), "", "the `/head` trigger text was removed");
    }

    #[test]
    fn set_paragraph_removes_trigger() {
        // "/p" -> Paragraph SET; the trigger text removed. (Already a paragraph, so SetBlock is
        // an idempotent no-op on KIND, but the trigger removal IS a change.)
        let (mut doc, mut undo, mut sel) = fixture("/p", 2);
        let menu = SlashMenuState { trigger_leaf_path: vec![0, 0], trigger_char: 0, filter: "p".into(), selected: 0, prompt: None };
        let cmd = cmd_by_id("paragraph");
        let outcome = execute_slash_command(&mut ctx(&mut doc, &mut undo, &mut sel), &menu, cmd);
        assert!(matches!(outcome, SlashExecOutcome::Done { changed: true }));
        assert_eq!(doc.children[0].as_block().unwrap().kind, NodeKind::Paragraph);
        assert_eq!(para_text(&doc, 0), "");
    }

    #[test]
    fn trigger_removal_only_deletes_slash_plus_filter() {
        // RISK-3: with text BEFORE the trigger ("note /head"), executing must delete ONLY the
        // `/head` (6 chars from offset 5), preserving "note ".
        let (mut doc, mut undo, mut sel) = fixture("note /head", 10);
        let menu = SlashMenuState { trigger_leaf_path: vec![0, 0], trigger_char: 5, filter: "head".into(), selected: 0, prompt: None };
        let cmd = cmd_by_id("heading-1");
        execute_slash_command(&mut ctx(&mut doc, &mut undo, &mut sel), &menu, cmd);
        assert_eq!(para_text(&doc, 0), "note ", "only `/head` removed, prefix preserved");
        assert_eq!(doc.children[0].as_block().unwrap().kind, NodeKind::Heading(HeadingLevel::new(1)));
    }

    #[test]
    fn insert_bullet_list_inserts_after_caret_block() {
        // "/" -> Bullet List inserts a NEW list after the (now-empty) paragraph.
        let (mut doc, mut undo, mut sel) = fixture("/", 1);
        let menu = SlashMenuState { trigger_leaf_path: vec![0, 0], trigger_char: 0, filter: String::new(), selected: 0, prompt: None };
        let cmd = cmd_by_id("bullet-list");
        let outcome = execute_slash_command(&mut ctx(&mut doc, &mut undo, &mut sel), &menu, cmd);
        assert!(matches!(outcome, SlashExecOutcome::Done { changed: true }));
        assert_eq!(doc.children.len(), 2, "a new list block was inserted after the paragraph");
        assert_eq!(doc.children[1].as_block().unwrap().kind, NodeKind::BulletList);
        // The caret descended into the list item's paragraph leaf.
        assert!(matches!(&sel, Selection::Text { head, .. } if head.path.len() >= 4));
    }

    #[test]
    fn insert_table_inserts_table_block() {
        let (mut doc, mut undo, mut sel) = fixture("/", 1);
        let menu = SlashMenuState { trigger_leaf_path: vec![0, 0], trigger_char: 0, filter: String::new(), selected: 0, prompt: None };
        execute_slash_command(&mut ctx(&mut doc, &mut undo, &mut sel), &menu, cmd_by_id("table"));
        assert_eq!(doc.children[1].as_block().unwrap().kind, NodeKind::Table);
        // 3 rows, 3 cols, first row header.
        let table = doc.children[1].as_block().unwrap();
        assert_eq!(table.children.len(), 3);
        let first_row = table.children[0].as_block().unwrap();
        assert_eq!(first_row.children.len(), 3);
        let header_cell = first_row.children[0].as_block().unwrap();
        assert_eq!(header_cell.attrs.get("isHeader").and_then(|v| v.as_bool()), Some(true));
    }

    #[test]
    fn insert_horizontal_rule_keeps_caret() {
        let (mut doc, mut undo, mut sel) = fixture("/", 1);
        let menu = SlashMenuState { trigger_leaf_path: vec![0, 0], trigger_char: 0, filter: String::new(), selected: 0, prompt: None };
        execute_slash_command(&mut ctx(&mut doc, &mut undo, &mut sel), &menu, cmd_by_id("horizontal-rule"));
        assert_eq!(doc.children[1].as_block().unwrap().kind, NodeKind::HorizontalRule);
        // Atom block: caret stays at the trigger paragraph (offset 0 after removal).
        assert!(matches!(&sel, Selection::Text { head, .. } if head.path == vec![0, 0]));
    }

    #[test]
    fn embed_command_opens_prompt() {
        let (mut doc, mut undo, mut sel) = fixture("/img", 4);
        let menu = SlashMenuState { trigger_leaf_path: vec![0, 0], trigger_char: 0, filter: "img".into(), selected: 0, prompt: None };
        let outcome = execute_slash_command(&mut ctx(&mut doc, &mut undo, &mut sel), &menu, cmd_by_id("embed-image"));
        match outcome {
            SlashExecOutcome::OpenPrompt(p) => {
                assert_eq!(p.kind, SlashPromptKind::Embed(EmbedKind::Image));
            }
            other => panic!("expected OpenPrompt, got {other:?}"),
        }
        // The trigger text was removed even though the insert is deferred to the prompt.
        assert_eq!(para_text(&doc, 0), "");
    }

    #[test]
    fn confirm_embed_prompt_inserts_hs_link_atom() {
        // AC-9: confirming a valid asset_id inserts an EmbedNode (an `hsLink` atom, ref_kind=images).
        let (mut doc, mut undo, mut sel) = fixture("", 0);
        let prompt = SlashPrompt { kind: SlashPromptKind::Embed(EmbedKind::Image), input: "asset-123".into() };
        assert!(confirm_prompt(&mut ctx(&mut doc, &mut undo, &mut sel), &prompt));
        let para = doc.children[0].as_block().unwrap();
        let atom = para.children.iter().find_map(Child::as_hs_link).expect("hsLink atom inserted");
        assert_eq!(atom.ref_kind, "images");
        assert_eq!(atom.ref_value, "asset-123");
    }

    #[test]
    fn confirm_blank_embed_prompt_is_noop() {
        let (mut doc, mut undo, mut sel) = fixture("", 0);
        let prompt = SlashPrompt { kind: SlashPromptKind::Embed(EmbedKind::Image), input: "   ".into() };
        assert!(!confirm_prompt(&mut ctx(&mut doc, &mut undo, &mut sel), &prompt));
        // No atom inserted.
        assert!(doc.children[0].as_block().unwrap().children.iter().all(|c| c.as_hs_link().is_none()));
    }

    #[test]
    fn confirm_transclusion_prompt_inserts_atom() {
        let (mut doc, mut undo, mut sel) = fixture("", 0);
        let prompt = SlashPrompt { kind: SlashPromptKind::Transclusion, input: "block-77".into() };
        assert!(confirm_prompt(&mut ctx(&mut doc, &mut undo, &mut sel), &prompt));
        let para = doc.children[0].as_block().unwrap();
        let t = para.children.iter().find_map(Child::as_transclusion).expect("transclusion atom");
        assert_eq!(t.ref_value, "block-77");
    }

    #[test]
    fn wikilink_command_opens_autocomplete_and_inserts_brackets() {
        let (mut doc, mut undo, mut sel) = fixture("/link", 5);
        let menu = SlashMenuState { trigger_leaf_path: vec![0, 0], trigger_char: 0, filter: "link".into(), selected: 0, prompt: None };
        let outcome = execute_slash_command(&mut ctx(&mut doc, &mut undo, &mut sel), &menu, cmd_by_id("insert-link"));
        assert_eq!(outcome, SlashExecOutcome::OpenWikilinkAutocomplete);
        // The `/link` trigger removed and a `[[` inserted so the MT-015 trigger fires.
        assert_eq!(para_text(&doc, 0), "[[");
    }

    #[test]
    fn daily_note_template_inserts_non_empty_blocks() {
        // AC-8: InsertTemplate for "Daily Note Template" inserts a predefined non-empty snippet.
        let (mut doc, mut undo, mut sel) = fixture("/", 1);
        let menu = SlashMenuState { trigger_leaf_path: vec![0, 0], trigger_char: 0, filter: String::new(), selected: 0, prompt: None };
        let outcome = execute_slash_command(&mut ctx(&mut doc, &mut undo, &mut sel), &menu, cmd_by_id("template-daily-note"));
        assert!(matches!(outcome, SlashExecOutcome::Done { changed: true }));
        // The original empty paragraph + the heading + the bullet list = >= 3 top-level blocks.
        assert!(doc.children.len() >= 3, "template inserted multiple blocks (got {})", doc.children.len());
        // The first inserted block is the heading.
        assert_eq!(doc.children[1].as_block().unwrap().heading_level(), Some(1));
    }

    #[test]
    fn manual_insert_parses_single_block() {
        let (mut doc, mut undo, mut sel) = fixture("", 0);
        let prompt = SlashPrompt {
            kind: SlashPromptKind::ManualInsert,
            input: r#"{"type":"paragraph","content":[{"type":"text","text":"agent block"}]}"#.into(),
        };
        assert!(confirm_prompt(&mut ctx(&mut doc, &mut undo, &mut sel), &prompt));
        // The pasted paragraph was inserted after the caret block.
        assert!(doc.children.iter().any(|c| {
            c.as_block()
                .map(|b| para_text_of(b) == "agent block")
                .unwrap_or(false)
        }));
    }

    fn para_text_of(b: &BlockNode) -> String {
        b.children.iter().filter_map(Child::as_text).map(|t| t.text.to_string()).collect()
    }

    #[test]
    fn undo_reverts_trigger_removal_and_block_change() {
        // The trigger removal + the SetBlock are both transactional; an undo of each reverts.
        let (mut doc, mut undo, mut sel) = fixture("/head", 5);
        let before = doc.clone();
        let menu = SlashMenuState { trigger_leaf_path: vec![0, 0], trigger_char: 0, filter: "head".into(), selected: 0, prompt: None };
        execute_slash_command(&mut ctx(&mut doc, &mut undo, &mut sel), &menu, cmd_by_id("heading-1"));
        // Two transactions pushed: the DeleteText (trigger) and the SetBlock (delete+insert).
        // Undo both to get back to the original "/head" paragraph.
        undo.undo(&mut doc).unwrap();
        undo.undo(&mut doc).unwrap();
        assert_eq!(doc, before, "undo restores the pre-command document");
    }

    #[test]
    fn execute_against_filtered_selection_matches_contract_flow() {
        // End-to-end shape: the widget filters by "head", the selected row (index 0) is
        // "heading-1", executing it sets the block. Proves the filter -> execute handoff.
        let rows = filter_slash_commands("head");
        let selected = rows[0];
        assert_eq!(selected.id, "heading-1");
        let (mut doc, mut undo, mut sel) = fixture("/head", 5);
        let menu = SlashMenuState { trigger_leaf_path: vec![0, 0], trigger_char: 0, filter: "head".into(), selected: 0, prompt: None };
        execute_slash_command(&mut ctx(&mut doc, &mut undo, &mut sel), &menu, selected);
        assert_eq!(doc.children[0].as_block().unwrap().heading_level(), Some(1));
    }

    // Reference the unused import so a future refactor keeps it.
    #[allow(unused_imports)]
    use crate::rich_editor::document_model::node::TextLeaf as _TextLeafRef;
    #[allow(dead_code)]
    fn _touch_textleaf() -> TextLeaf {
        TextLeaf::new("")
    }
}
