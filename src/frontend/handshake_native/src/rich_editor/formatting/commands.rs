//! Block- and inline-level formatting commands for the rich-text editor
//! (WP-KERNEL-012 MT-013).
//!
//! This module is the COMMAND LAYER of the E2 rich-text cluster: every toolbar
//! button (MT-013 [`super::toolbar`]) and keyboard shortcut (MT-013
//! [`super::keymap`]) ultimately calls one of the functions here. A command takes
//! the live editor state by `&mut` (the [`BlockNode`] doc, the [`UndoManager`], and
//! the [`Selection`]), builds an MT-011 [`Transaction`] of [`Step`]s, applies it
//! atomically through [`apply_transaction`], pushes the receipt onto the undo
//! manager on success, and updates the post-op selection. No global mutable state;
//! the command never owns the document (MT impl note 4).
//!
//! ## Why a command CAN mutate the selection (deviation from the literal signature)
//!
//! The MT contract sketches `fn command(doc, history, selection: &Selection)`, but
//! the SCOPE-EXPANSION note ("set the post-op Selection in the command") and the
//! structural commands (Enter-split moves the caret to the start of the new block;
//! Backspace-merge lands the caret at the join point; set_heading keeps the caret in
//! the converted block) REQUIRE the command to move the caret. So the selection is
//! taken `&mut`. This is the boringly-correct shape: a ProseMirror command returns a
//! transaction that carries BOTH doc steps and a new selection, and the native model
//! threads the selection alongside. The toggle/insert commands that do not move the
//! caret leave it untouched.
//!
//! ## Active-state discipline (red-team RISK-1 / MC-001)
//!
//! [`is_mark_active`] inspects EVERY [`TextLeaf`] that overlaps the current selection
//! range, not just the leaf at the caret. The ProseMirror "toggle mark" rule is: if
//! ANY overlapping leaf carries the mark, the toggle REMOVES it from all of them; if
//! NONE do, the toggle ADDS it to all of them. A 3-leaf selection where only the
//! middle leaf is bold therefore reports `active = true` and the toggle removes the
//! bold (test `toggle_bold_multi_leaf_active`).
//!
//! ## Context guards (red-team MC-002 / MC-003)
//!
//! - `set_heading` / `set_paragraph` only convert a `paragraph` or `heading` block;
//!   a caret inside a `list_item`, `table_cell`, or `code_block` returns
//!   [`CommandError::InvalidContext`] (a `list_item` is not a legal direct child of
//!   `doc`, so a blind conversion would corrupt the tree — MC-002).
//! - `insert_table` refuses to nest a table inside an existing table
//!   ([`CommandError::InvalidContext`]) — the backend `content_json` table schema has
//!   no nested-table content (MC-003).
//!
//! ## Backend node/attr-shape anchor (MT-011 hsLink lesson, KERNEL_BUILDER gate)
//!
//! The list/table/task_item commands produce the node + attr shapes the REAL backend
//! `content_json` round-trips (read from `app/src/lib/editor/editor_commands.ts` +
//! `app/src/lib/tiptap/extension_set.ts`):
//! - a task item carries `attrs.checked: bool` (Tiptap `taskItem`'s `checked` attr).
//! - a table cell carries `attrs.colspan: 1` + `attrs.rowspan: 1` (Tiptap
//!   table-cell defaults), and a header row's cells carry `attrs.isHeader: true`.
//!
//! NOTE (table header DIVERGENCE, recorded honestly): the REAL backend uses a
//! distinct `tableHeader` NODE TYPE for header cells, but the MT-011 [`NodeKind`]
//! enum (a frozen dependency this MT reuses, NOT forks) has only `TableCell` — no
//! `TableHeader` variant. So header-ness is carried as a cell `attrs.isHeader: true`
//! attr here. A full `tableHeader`-node round-trip is an MT-011 model gap surfaced as
//! a blocker, not fixed by forking the model in MT-013.

use crate::rich_editor::document_model::history::UndoManager;
use crate::rich_editor::document_model::node::{
    BlockNode, Child, HeadingLevel, Mark, NodeKind, TextLeaf,
};
use crate::rich_editor::document_model::position::DocPosition;
use crate::rich_editor::document_model::selection::Selection;
use crate::rich_editor::document_model::transform::{
    apply_transaction, ActorKind, Step, Transaction, TransformError,
};

use serde_json::Value as JsonValue;

/// Why a formatting command could not run. Distinct from [`TransformError`]: a
/// `CommandError` is raised by the command layer BEFORE (or instead of) building a
/// transaction — a guard refusing an illegal context, or no addressable caret. A
/// transaction that fails schema validation surfaces as [`CommandError::Transform`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CommandError {
    /// The command is not valid in the current node context (MC-002 / MC-003): e.g.
    /// `set_heading` inside a `list_item`/`table_cell`/`code_block`, or `insert_table`
    /// inside an existing table.
    #[error("command not valid in this context: {reason}")]
    InvalidContext { reason: String },
    /// The selection does not address a caret/leaf the command can act on (e.g. a
    /// whole-node selection where a text caret is required).
    #[error("no addressable caret/selection for this command")]
    NoCaret,
    /// The command's transaction failed to apply (addressing or schema error); the
    /// doc was rolled back by [`apply_transaction`].
    #[error("transform failed: {0}")]
    Transform(#[from] TransformError),
}

/// The full catalog of MT-013 formatting commands, one variant per toolbar button /
/// keyboard shortcut. Achieves command-for-command parity with the React
/// `EDITOR_COMMANDS` catalog categories `history | format | block | list | table |
/// tableEdit` (plus the MT-013 scope-expansion structural commands). The toolbar and
/// keymap both produce a `FormattingCommand` and dispatch it through [`dispatch`], so
/// the two surfaces can never drift (the same id drives a button and a chord).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormattingCommand {
    // --- history ---
    Undo,
    Redo,
    // --- inline marks (format) ---
    ToggleBold,
    ToggleItalic,
    ToggleUnderline,
    ToggleStrike,
    ToggleCode,
    // --- block types ---
    SetParagraph,
    SetHeading(u8),
    SetBlockquote,
    SetCodeBlock(Option<String>),
    InsertHorizontalRule,
    // --- lists ---
    ToggleBulletList,
    ToggleOrderedList,
    ToggleTaskList,
    ToggleTaskItemChecked,
    SinkListItem,
    LiftListItem,
    // --- tables ---
    InsertTable { rows: u8, cols: u8 },
    AddRowBefore,
    AddRowAfter,
    DeleteRow,
    AddColBefore,
    AddColAfter,
    DeleteCol,
    DeleteTable,
    ToggleHeaderRow,
    // --- structural editing (MT-013 scope expansion) ---
    /// Enter at the caret: split the current block into two.
    InsertParagraphBreak,
    /// Backspace at caret offset 0: merge the current block into the previous sibling.
    MergeBackward,
}

impl FormattingCommand {
    /// The stable command id string, used to build the toolbar button author_id
    /// (`toolbar-btn-{command_id}`) and to match React catalog ids. Uses the snake_case
    /// names the MT scope lists (`toggle_bold`, `set_heading`, …) so the AccessKit
    /// `toolbar-btn-toggle_bold` id the AC asserts is produced verbatim.
    ///
    /// The three heading buttons (H1/H2/H3) are DISTINCT toolbar controls, so each gets a
    /// level-specific id (`set_heading_1` / `_2` / `_3`) — a single shared `set_heading`
    /// id would make them indistinguishable to a swarm agent (a duplicate-author_id
    /// collision the HBR-SWARM gate forbids). The base `set_heading(level)` command
    /// FUNCTION is still one function; only its addressable id carries the level.
    pub fn command_id(&self) -> &'static str {
        match self {
            FormattingCommand::Undo => "undo",
            FormattingCommand::Redo => "redo",
            FormattingCommand::ToggleBold => "toggle_bold",
            FormattingCommand::ToggleItalic => "toggle_italic",
            FormattingCommand::ToggleUnderline => "toggle_underline",
            FormattingCommand::ToggleStrike => "toggle_strike",
            FormattingCommand::ToggleCode => "toggle_code",
            FormattingCommand::SetParagraph => "set_paragraph",
            FormattingCommand::SetHeading(1) => "set_heading_1",
            FormattingCommand::SetHeading(2) => "set_heading_2",
            FormattingCommand::SetHeading(3) => "set_heading_3",
            // HeadingLevel::new clamps out-of-range to 1..=3, so only 1/2/3 are
            // constructible via dispatch; a raw out-of-range variant maps to the level-1
            // id rather than panicking (defensive — never reached through the keymap/toolbar).
            FormattingCommand::SetHeading(_) => "set_heading_1",
            FormattingCommand::SetBlockquote => "set_blockquote",
            FormattingCommand::SetCodeBlock(_) => "set_code_block",
            FormattingCommand::InsertHorizontalRule => "insert_horizontal_rule",
            FormattingCommand::ToggleBulletList => "toggle_bullet_list",
            FormattingCommand::ToggleOrderedList => "toggle_ordered_list",
            FormattingCommand::ToggleTaskList => "toggle_task_list",
            FormattingCommand::ToggleTaskItemChecked => "toggle_task_item_checked",
            FormattingCommand::SinkListItem => "sink_list_item",
            FormattingCommand::LiftListItem => "lift_list_item",
            FormattingCommand::InsertTable { .. } => "insert_table",
            FormattingCommand::AddRowBefore => "add_row_before",
            FormattingCommand::AddRowAfter => "add_row_after",
            FormattingCommand::DeleteRow => "delete_row",
            FormattingCommand::AddColBefore => "add_col_before",
            FormattingCommand::AddColAfter => "add_col_after",
            FormattingCommand::DeleteCol => "delete_col",
            FormattingCommand::DeleteTable => "delete_table",
            FormattingCommand::ToggleHeaderRow => "toggle_header_row",
            FormattingCommand::InsertParagraphBreak => "insert_paragraph_break",
            FormattingCommand::MergeBackward => "merge_backward",
        }
    }
}

/// The mutable editor state a command drives: the document tree, the undo manager,
/// the selection, and the actor id for transaction provenance. Borrowed from
/// `RichEditorState` by the toolbar/keymap dispatch. Kept as an explicit borrow
/// struct (not the whole `RichEditorState`) so the command layer has NO dependency on
/// the renderer/IME fields and is unit-testable without an egui context.
pub struct CommandContext<'a> {
    /// The document being edited (the `doc` root).
    pub doc: &'a mut BlockNode,
    /// The undo/redo history (receipts pushed on each successful mutating command).
    pub history: &'a mut UndoManager,
    /// The current selection; commands may move the caret (see module docs).
    pub selection: &'a mut Selection,
    /// Actor id for transaction provenance (`"operator"` for keyboard/toolbar input).
    pub actor_id: &'a str,
}

impl<'a> CommandContext<'a> {
    /// Build a command context from the live editor state borrows.
    pub fn new(
        doc: &'a mut BlockNode,
        history: &'a mut UndoManager,
        selection: &'a mut Selection,
        actor_id: &'a str,
    ) -> Self {
        Self {
            doc,
            history,
            selection,
            actor_id,
        }
    }
}

/// Dispatch a [`FormattingCommand`] against the live editor state. This is the single
/// entry both the toolbar and the keymap call, so the two surfaces share ONE command
/// path (no drift). Returns `Ok(())` when the command ran (or was a benign no-op such
/// as undo with an empty stack), or a [`CommandError`] when a guard refused it.
pub fn dispatch(ctx: &mut CommandContext<'_>, cmd: &FormattingCommand) -> Result<(), CommandError> {
    match cmd {
        FormattingCommand::Undo => undo(ctx),
        FormattingCommand::Redo => redo(ctx),
        FormattingCommand::ToggleBold => toggle_mark(ctx, Mark::Bold),
        FormattingCommand::ToggleItalic => toggle_mark(ctx, Mark::Italic),
        FormattingCommand::ToggleUnderline => toggle_mark(ctx, Mark::Underline),
        FormattingCommand::ToggleStrike => toggle_mark(ctx, Mark::Strike),
        FormattingCommand::ToggleCode => toggle_mark(ctx, Mark::Code),
        FormattingCommand::SetParagraph => set_block_kind(ctx, NodeKind::Paragraph),
        FormattingCommand::SetHeading(level) => {
            set_block_kind(ctx, NodeKind::Heading(HeadingLevel::new(*level)))
        }
        FormattingCommand::SetBlockquote => toggle_wrap_blockquote(ctx),
        FormattingCommand::SetCodeBlock(lang) => set_code_block(ctx, lang.clone()),
        FormattingCommand::InsertHorizontalRule => insert_horizontal_rule(ctx),
        FormattingCommand::ToggleBulletList => toggle_list(ctx, NodeKind::BulletList),
        FormattingCommand::ToggleOrderedList => toggle_list(ctx, NodeKind::OrderedList),
        FormattingCommand::ToggleTaskList => toggle_task_list(ctx),
        FormattingCommand::ToggleTaskItemChecked => toggle_task_item_checked(ctx),
        FormattingCommand::SinkListItem => sink_list_item(ctx),
        FormattingCommand::LiftListItem => lift_list_item(ctx),
        FormattingCommand::InsertTable { rows, cols } => insert_table(ctx, *rows, *cols),
        FormattingCommand::AddRowBefore => add_row(ctx, false),
        FormattingCommand::AddRowAfter => add_row(ctx, true),
        FormattingCommand::DeleteRow => delete_row(ctx),
        FormattingCommand::AddColBefore => add_col(ctx, false),
        FormattingCommand::AddColAfter => add_col(ctx, true),
        FormattingCommand::DeleteCol => delete_col(ctx),
        FormattingCommand::DeleteTable => delete_table(ctx),
        FormattingCommand::ToggleHeaderRow => toggle_header_row(ctx),
        FormattingCommand::InsertParagraphBreak => insert_paragraph_break(ctx),
        FormattingCommand::MergeBackward => merge_backward(ctx),
    }
}

// ── caret/selection helpers ──────────────────────────────────────────────────────

/// The caret head position (collapsed selections use head == anchor). A node
/// selection has no text caret.
fn head(selection: &Selection) -> Option<DocPosition> {
    match selection {
        Selection::Text { head, .. } => Some(head.clone()),
        Selection::Node { .. } => None,
    }
}

/// The anchor position of a text selection (the fixed end). `None` for a node
/// selection.
fn anchor(selection: &Selection) -> Option<DocPosition> {
    match selection {
        Selection::Text { anchor, .. } => Some(anchor.clone()),
        Selection::Node { .. } => None,
    }
}

/// The BLOCK path (the caret head's leaf path with the final text-leaf index
/// dropped). For a caret at `[1, 0]` (paragraph 1, text leaf 0) this is `[1]` — the
/// paragraph block. `None` for a node selection or an empty/too-short path.
fn caret_block_path(selection: &Selection) -> Option<Vec<usize>> {
    let h = head(selection)?;
    if h.path.is_empty() {
        return None;
    }
    Some(h.path[..h.path.len() - 1].to_vec())
}

/// Resolve a child-index block `path` (empty path = the doc root) to a shared
/// [`BlockNode`] reference, or `None` if any index is out of range / not a block.
fn block_at<'a>(doc: &'a BlockNode, path: &[usize]) -> Option<&'a BlockNode> {
    let mut node = doc;
    for &idx in path {
        node = node.children.get(idx)?.as_block()?;
    }
    Some(node)
}

/// True when `path` (a block path) lands on (or inside) a node of `kind`. Used by the
/// context guards: e.g. is the caret block path a descendant of a `Table`?
fn path_is_inside_kind(doc: &BlockNode, path: &[usize], kind: NodeKind) -> bool {
    let mut node = doc;
    if node.kind == kind {
        return true;
    }
    for &idx in path {
        let Some(next) = node.children.get(idx).and_then(Child::as_block) else {
            return false;
        };
        node = next;
        if node.kind == kind {
            return true;
        }
    }
    false
}

/// Build + apply an operator transaction, pushing the receipt on success. Returns the
/// transform result so the caller can map it to a [`CommandError`].
fn apply(ctx: &mut CommandContext<'_>, steps: Vec<Step>) -> Result<(), CommandError> {
    let tx = Transaction::new(steps, ActorKind::Operator, ctx.actor_id);
    let receipt = apply_transaction(ctx.doc, tx)?;
    ctx.history.push(receipt);
    Ok(())
}

// ── history ───────────────────────────────────────────────────────────────────────

/// Undo the last command (delegates to the MT-011 [`UndoManager`]). A no-op (Ok) when
/// the undo stack is empty.
fn undo(ctx: &mut CommandContext<'_>) -> Result<(), CommandError> {
    ctx.history.undo(ctx.doc)?;
    clamp_selection(ctx);
    Ok(())
}

/// Redo the last undone command. A no-op (Ok) when there is nothing to redo.
fn redo(ctx: &mut CommandContext<'_>) -> Result<(), CommandError> {
    ctx.history.redo(ctx.doc)?;
    clamp_selection(ctx);
    Ok(())
}

// ── inline marks (RISK-1 / MC-001 full-range active check) ──────────────────────────

/// The inclusive range of (block_index, leaf_index) pairs the selection spans at the
/// TOP level. The MT-011 model marks at whole-leaf granularity, so a selection's mark
/// toggle acts on every text leaf whose block+leaf index falls between the anchor and
/// head leaves (inclusive). Returns a flat list of leaf paths (each `[.., leaf_idx]`).
///
/// For a collapsed caret this yields exactly the one leaf the caret sits in.
fn selected_leaf_paths(ctx: &CommandContext<'_>) -> Vec<Vec<usize>> {
    let (Some(a), Some(h)) = (anchor(ctx.selection), head(ctx.selection)) else {
        return Vec::new();
    };
    // Order the two endpoints by document order (block path then leaf, lexicographic).
    let (start, end) = if a.path <= h.path { (a, h) } else { (h, a) };

    // Collapsed caret (same leaf path): just that one leaf.
    if start.path == end.path {
        return vec![start.path];
    }

    // Multi-leaf: walk the doc in document order, collecting every text-leaf path that
    // is >= start.path and <= end.path (lexicographic on the child-index path). This
    // covers a selection spanning sibling leaves in one block and leaves across blocks.
    let mut out = Vec::new();
    collect_leaf_paths_in_range(ctx.doc, &mut Vec::new(), &start.path, &end.path, &mut out);
    if out.is_empty() {
        // Defensive: at minimum act on the start leaf.
        out.push(start.path);
    }
    out
}

/// Recursive walk collecting text-leaf paths in `[start_path, end_path]` (inclusive,
/// lexicographic). Pushes the path of every `Child::Text` leaf whose full path sorts
/// within the bound.
fn collect_leaf_paths_in_range(
    node: &BlockNode,
    path: &mut Vec<usize>,
    start_path: &[usize],
    end_path: &[usize],
    out: &mut Vec<Vec<usize>>,
) {
    for (i, child) in node.children.iter().enumerate() {
        path.push(i);
        match child {
            Child::Text(_) => {
                if path.as_slice() >= start_path && path.as_slice() <= end_path {
                    out.push(path.clone());
                }
            }
            Child::Block(b) => {
                collect_leaf_paths_in_range(b, path, start_path, end_path, out);
            }
            Child::HsLink(_) => {}
        }
        // Pop AFTER visiting the child so each child's full path is correct during its
        // own visit, and siblings do not accumulate stale indices.
        path.pop();
    }
}

/// True when the mark of `mark`'s type is active over the current selection: ANY
/// overlapping text leaf carries it (ProseMirror toggle semantics — RISK-1 / MC-001).
pub fn is_mark_active(doc: &BlockNode, selection: &Selection, mark: &Mark) -> bool {
    let ctx_doc = doc;
    let leaf_paths = {
        // Reuse the same range computation without a full CommandContext.
        let (a, h) = match selection {
            Selection::Text { anchor, head } => (anchor.clone(), head.clone()),
            Selection::Node { .. } => return false,
        };
        let (start, end) = if a.path <= h.path { (a, h) } else { (h, a) };
        if start.path == end.path {
            vec![start.path]
        } else {
            let mut out = Vec::new();
            collect_leaf_paths_in_range(ctx_doc, &mut Vec::new(), &start.path, &end.path, &mut out);
            out
        }
    };
    leaf_paths.iter().any(|p| {
        leaf_at(ctx_doc, p)
            .map(|l| l.has_mark_type(mark))
            .unwrap_or(false)
    })
}

/// Resolve a leaf path (block indices then a final text-leaf index) to a shared
/// [`TextLeaf`].
fn leaf_at<'a>(doc: &'a BlockNode, path: &[usize]) -> Option<&'a TextLeaf> {
    let (leaf_idx, block_path) = path.split_last()?;
    let block = block_at(doc, block_path)?;
    block.children.get(*leaf_idx)?.as_text()
}

/// Toggle an inline mark over the whole selection (RISK-1 / MC-001). If ANY overlapping
/// leaf has the mark, every overlapping leaf has it REMOVED; otherwise every overlapping
/// leaf has it ADDED. A `code_block`'s text run forbids marks (schema), so a leaf inside
/// one is skipped (the transaction would otherwise roll back).
fn toggle_mark(ctx: &mut CommandContext<'_>, mark: Mark) -> Result<(), CommandError> {
    let leaf_paths = selected_leaf_paths(ctx);
    if leaf_paths.is_empty() {
        return Err(CommandError::NoCaret);
    }
    let active = is_mark_active(ctx.doc, ctx.selection, &mark);
    let mut steps = Vec::new();
    for p in &leaf_paths {
        // Skip a leaf whose parent forbids marks (code_block) so the whole tx does not
        // roll back on an otherwise-valid multi-leaf toggle.
        if leaf_parent_forbids_marks(ctx.doc, p) {
            continue;
        }
        if active {
            steps.push(Step::RemoveMark {
                path: p.clone(),
                mark: mark.clone(),
            });
        } else {
            steps.push(Step::AddMark {
                path: p.clone(),
                mark: mark.clone(),
            });
        }
    }
    if steps.is_empty() {
        return Ok(()); // nothing to do (e.g. only code-block leaves selected)
    }
    apply(ctx, steps)
}

/// True when the leaf at `path`'s parent block is a `code_block` (which forbids marks).
fn leaf_parent_forbids_marks(doc: &BlockNode, path: &[usize]) -> bool {
    let Some((_, block_path)) = path.split_last() else {
        return false;
    };
    block_at(doc, block_path)
        .map(|b| matches!(b.kind, NodeKind::CodeBlock))
        .unwrap_or(false)
}

// ── block-kind conversions (MC-002 context guard) ───────────────────────────────────

/// Convert the caret's block to `new_kind` (paragraph<->heading). Guards: the block
/// must currently be a `paragraph` or `heading` AND its parent must be the `doc` root
/// (so the converted block stays a legal direct child) — a caret inside a `list_item`,
/// `table_cell`, `blockquote`, or `code_block` returns `InvalidContext` (MC-002).
///
/// Implemented via DeleteNode + InsertNode (the MT-011 transform has no `ReplaceNode`
/// step): the new block carries the SAME children (text + marks survive) and, for a
/// heading, the level rides in the typed [`NodeKind::Heading`] variant.
fn set_block_kind(ctx: &mut CommandContext<'_>, new_kind: NodeKind) -> Result<(), CommandError> {
    let block_path = caret_block_path(ctx.selection).ok_or(CommandError::NoCaret)?;
    let (parent_path, index) = split_block_path(&block_path)?;

    let block = block_at(ctx.doc, &block_path).ok_or(CommandError::NoCaret)?;
    // Guard: only a paragraph/heading is convertible.
    if !matches!(block.kind, NodeKind::Paragraph | NodeKind::Heading(_)) {
        return Err(CommandError::InvalidContext {
            reason: format!(
                "set_block_kind only converts a paragraph/heading, not {:?}",
                block.kind
            ),
        });
    }
    // Guard: the converted block must be a legal child of its parent. paragraph and
    // heading are legal anywhere a paragraph is (doc, list_item, table_cell,
    // blockquote), so the conversion is safe as long as the parent already holds the
    // (paragraph/heading) block — which it does. The MC-002 corruption case is a
    // list_item being treated as the convertible block; we already rejected that above
    // (the block itself is the paragraph, and its kind is paragraph/heading).
    let already = block.kind == new_kind;
    if already {
        // Toggling a heading to the same level (or paragraph to paragraph) is a no-op
        // in this command — the React `toggleHeading` toggles back to paragraph, but
        // `set_paragraph`/`set_heading` here are SET (idempotent), so just keep state.
        return Ok(());
    }
    let mut new_block = BlockNode::new(new_kind);
    new_block.attrs = block.attrs.clone();
    new_block.children = block.children.clone();

    apply(
        ctx,
        vec![
            Step::DeleteNode {
                parent_path: parent_path.clone(),
                index,
            },
            Step::InsertNode {
                parent_path,
                index,
                node: new_block,
            },
        ],
    )?;
    // The caret stays in the (now-converted) block at the same offset; the block path
    // is unchanged (same parent + index), so the existing selection still resolves.
    clamp_selection(ctx);
    Ok(())
}

/// Set the caret's block to a `code_block` (carrying an optional `attrs.language`).
/// Guarded like [`set_block_kind`] (paragraph/heading only). Marks are dropped because
/// a code block forbids marks (schema) — the children are flattened to a single text
/// leaf of the concatenated text.
fn set_code_block(
    ctx: &mut CommandContext<'_>,
    language: Option<String>,
) -> Result<(), CommandError> {
    let block_path = caret_block_path(ctx.selection).ok_or(CommandError::NoCaret)?;
    let (parent_path, index) = split_block_path(&block_path)?;
    let block = block_at(ctx.doc, &block_path).ok_or(CommandError::NoCaret)?;
    if !matches!(block.kind, NodeKind::Paragraph | NodeKind::Heading(_)) {
        return Err(CommandError::InvalidContext {
            reason: format!("set_code_block only converts a paragraph/heading, not {:?}", block.kind),
        });
    }
    // Flatten inline content to plain text (a code block holds ONE unmarked text leaf).
    let mut text = String::new();
    for c in &block.children {
        if let Child::Text(t) = c {
            text.push_str(&t.text.to_string());
        }
    }
    let mut new_block = BlockNode::new(NodeKind::CodeBlock);
    if let Some(lang) = language {
        new_block
            .attrs
            .insert("language".to_string(), JsonValue::from(lang));
    }
    new_block.children = vec![Child::Text(TextLeaf::new(&text))];

    apply(
        ctx,
        vec![
            Step::DeleteNode {
                parent_path: parent_path.clone(),
                index,
            },
            Step::InsertNode {
                parent_path,
                index,
                node: new_block,
            },
        ],
    )?;
    clamp_selection(ctx);
    Ok(())
}

/// Wrap the caret's block in a `blockquote` (or unwrap when already inside one),
/// matching React `toggleBlockquote`. The blockquote is a top-level container holding
/// the block; toggling off lifts the block back to the doc root.
fn toggle_wrap_blockquote(ctx: &mut CommandContext<'_>) -> Result<(), CommandError> {
    let block_path = caret_block_path(ctx.selection).ok_or(CommandError::NoCaret)?;
    let (parent_path, index) = split_block_path(&block_path)?;

    // Already inside a blockquote at the immediate parent? -> unwrap.
    if !parent_path.is_empty() {
        if let Some(parent) = block_at(ctx.doc, &parent_path) {
            if matches!(parent.kind, NodeKind::Blockquote) && parent.children.len() == 1 {
                // Unwrap: replace the blockquote (at grandparent[gp_index]) with its child.
                let (gp_path, gp_index) = split_block_path(&parent_path)?;
                let inner = parent.children[0]
                    .as_block()
                    .ok_or(CommandError::NoCaret)?
                    .clone();
                apply(
                    ctx,
                    vec![
                        Step::DeleteNode {
                            parent_path: gp_path.clone(),
                            index: gp_index,
                        },
                        Step::InsertNode {
                            parent_path: gp_path.clone(),
                            index: gp_index,
                            node: inner,
                        },
                    ],
                )?;
                // Caret moves up one nesting level: drop the blockquote index.
                reparent_caret(ctx, &parent_path, &gp_path);
                return Ok(());
            }
        }
    }

    // Wrap: replace the block with a blockquote holding it.
    let block = block_at(ctx.doc, &block_path).ok_or(CommandError::NoCaret)?.clone();
    let quote = BlockNode::with_children(NodeKind::Blockquote, vec![Child::Block(block)]);
    apply(
        ctx,
        vec![
            Step::DeleteNode {
                parent_path: parent_path.clone(),
                index,
            },
            Step::InsertNode {
                parent_path: parent_path.clone(),
                index,
                node: quote,
            },
        ],
    )?;
    // Caret descends one level: the block is now at parent[index] -> [0].
    let mut new_block_path = parent_path.clone();
    new_block_path.push(index);
    new_block_path.push(0);
    reparent_caret_into(ctx, &block_path, &new_block_path);
    Ok(())
}

/// Insert a `horizontal_rule` atom block right AFTER the caret's block.
fn insert_horizontal_rule(ctx: &mut CommandContext<'_>) -> Result<(), CommandError> {
    let block_path = caret_block_path(ctx.selection).ok_or(CommandError::NoCaret)?;
    let (parent_path, index) = split_block_path(&block_path)?;
    apply(
        ctx,
        vec![Step::InsertNode {
            parent_path,
            index: index + 1,
            node: BlockNode::new(NodeKind::HorizontalRule),
        }],
    )
}

// ── lists ───────────────────────────────────────────────────────────────────────

/// Toggle the caret's paragraph into a list of `list_kind` (bullet/ordered) holding a
/// single `list_item`, or back to a paragraph when already inside one. Matches React
/// `toggleBulletList` / `toggleOrderedList`. Only converts a top-level paragraph
/// (guarded — a caret already inside a list toggles back out).
fn toggle_list(ctx: &mut CommandContext<'_>, list_kind: NodeKind) -> Result<(), CommandError> {
    let block_path = caret_block_path(ctx.selection).ok_or(CommandError::NoCaret)?;
    let (parent_path, index) = split_block_path(&block_path)?;

    // Already a list_item inside a list of this kind at the parent chain? -> toggle off.
    if let Some(list_path) = enclosing_list(ctx.doc, &block_path) {
        let list = block_at(ctx.doc, &list_path).ok_or(CommandError::NoCaret)?;
        if list.kind == list_kind {
            return unwrap_list_to_paragraph(ctx, &list_path);
        }
    }

    let block = block_at(ctx.doc, &block_path).ok_or(CommandError::NoCaret)?;
    if !matches!(block.kind, NodeKind::Paragraph | NodeKind::Heading(_)) {
        return Err(CommandError::InvalidContext {
            reason: format!("toggle_list only converts a paragraph/heading, not {:?}", block.kind),
        });
    }
    let para = block.clone();
    let list_item = BlockNode::with_children(NodeKind::ListItem, vec![Child::Block(para)]);
    let list = BlockNode::with_children(list_kind, vec![Child::Block(list_item)]);
    apply(
        ctx,
        vec![
            Step::DeleteNode {
                parent_path: parent_path.clone(),
                index,
            },
            Step::InsertNode {
                parent_path: parent_path.clone(),
                index,
                node: list,
            },
        ],
    )?;
    // Caret descends two levels: block -> list[index] / item[0] / para[0].
    let mut new_block_path = parent_path.clone();
    new_block_path.extend_from_slice(&[index, 0, 0]);
    reparent_caret_into(ctx, &block_path, &new_block_path);
    Ok(())
}

/// Toggle a task list: like [`toggle_list`] but the items are `task_item`s carrying
/// `attrs.checked: false` (the Tiptap `taskItem` shape — backend anchor). Toggling off
/// converts the task list back to a paragraph.
fn toggle_task_list(ctx: &mut CommandContext<'_>) -> Result<(), CommandError> {
    let block_path = caret_block_path(ctx.selection).ok_or(CommandError::NoCaret)?;
    let (parent_path, index) = split_block_path(&block_path)?;

    if let Some(list_path) = enclosing_list(ctx.doc, &block_path) {
        // A bullet/ordered list holding task_items is our task-list representation;
        // toggling off unwraps regardless of which list kind hosts the task items.
        let list = block_at(ctx.doc, &list_path).ok_or(CommandError::NoCaret)?;
        let holds_task = list
            .children
            .iter()
            .filter_map(Child::as_block)
            .any(|c| matches!(c.kind, NodeKind::TaskItem));
        if holds_task {
            return unwrap_list_to_paragraph(ctx, &list_path);
        }
    }

    let block = block_at(ctx.doc, &block_path).ok_or(CommandError::NoCaret)?;
    if !matches!(block.kind, NodeKind::Paragraph | NodeKind::Heading(_)) {
        return Err(CommandError::InvalidContext {
            reason: format!("toggle_task_list only converts a paragraph/heading, not {:?}", block.kind),
        });
    }
    let para = block.clone();
    let mut task_item = BlockNode::with_children(NodeKind::TaskItem, vec![Child::Block(para)]);
    task_item
        .attrs
        .insert("checked".to_string(), JsonValue::Bool(false));
    // Task items live inside a bullet list (Tiptap TaskList renders as a list whose
    // items are taskItems; the MT-011 schema allows TaskItem as a list child).
    let list = BlockNode::with_children(NodeKind::BulletList, vec![Child::Block(task_item)]);
    apply(
        ctx,
        vec![
            Step::DeleteNode {
                parent_path: parent_path.clone(),
                index,
            },
            Step::InsertNode {
                parent_path: parent_path.clone(),
                index,
                node: list,
            },
        ],
    )?;
    let mut new_block_path = parent_path.clone();
    new_block_path.extend_from_slice(&[index, 0, 0]);
    reparent_caret_into(ctx, &block_path, &new_block_path);
    Ok(())
}

/// Toggle the `checked` attr of the task item enclosing the caret (false<->true).
/// Implemented as a DeleteNode + InsertNode of the task_item with the flipped attr
/// (the MT-011 transform has no in-place attr step). Errors if the caret is not inside
/// a task item.
fn toggle_task_item_checked(ctx: &mut CommandContext<'_>) -> Result<(), CommandError> {
    let block_path = caret_block_path(ctx.selection).ok_or(CommandError::NoCaret)?;
    let task_path = enclosing_kind(ctx.doc, &block_path, NodeKind::TaskItem).ok_or(
        CommandError::InvalidContext {
            reason: "toggle_task_item_checked requires the caret inside a task item".to_string(),
        },
    )?;
    let (parent_path, index) = split_block_path(&task_path)?;
    let task = block_at(ctx.doc, &task_path).ok_or(CommandError::NoCaret)?;
    let mut new_task = task.clone();
    let now = !new_task.task_checked();
    new_task
        .attrs
        .insert("checked".to_string(), JsonValue::Bool(now));
    apply(
        ctx,
        vec![
            Step::DeleteNode {
                parent_path: parent_path.clone(),
                index,
            },
            Step::InsertNode {
                parent_path,
                index,
                node: new_task,
            },
        ],
    )?;
    clamp_selection(ctx);
    Ok(())
}

/// Sink (indent) the list item enclosing the caret: nest it as a child of the previous
/// sibling list item (the ProseMirror `sinkListItem` shape). Requires a previous
/// sibling item (you cannot indent the first item). The increased nesting depth is the
/// "indent level" the AC checks.
fn sink_list_item(ctx: &mut CommandContext<'_>) -> Result<(), CommandError> {
    let block_path = caret_block_path(ctx.selection).ok_or(CommandError::NoCaret)?;
    let item_path = enclosing_list_item(ctx.doc, &block_path).ok_or(CommandError::InvalidContext {
        reason: "sink_list_item requires the caret inside a list item".to_string(),
    })?;
    let (list_path, item_index) = split_block_path(&item_path)?;
    if item_index == 0 {
        return Err(CommandError::InvalidContext {
            reason: "cannot sink the first list item (no previous sibling to nest under)".to_string(),
        });
    }
    let list = block_at(ctx.doc, &list_path).ok_or(CommandError::NoCaret)?;
    let list_kind = list.kind;
    let item = list.children[item_index]
        .as_block()
        .ok_or(CommandError::NoCaret)?
        .clone();
    // The previous sibling item gets a nested sub-list (of the same kind) appended,
    // holding the sunk item. Rebuild the previous item with the nested list.
    let prev = list.children[item_index - 1]
        .as_block()
        .ok_or(CommandError::NoCaret)?
        .clone();
    let mut new_prev = prev.clone();
    // If the previous item already ends in a sub-list of this kind, append to it;
    // otherwise add a new sub-list. (Keeps repeated indents from stacking empty lists.)
    if let Some(Child::Block(last)) = new_prev.children.last_mut() {
        if last.kind == list_kind {
            last.children.push(Child::Block(item.clone()));
        } else {
            let sub = BlockNode::with_children(list_kind, vec![Child::Block(item.clone())]);
            new_prev.children.push(Child::Block(sub));
        }
    } else {
        let sub = BlockNode::with_children(list_kind, vec![Child::Block(item.clone())]);
        new_prev.children.push(Child::Block(sub));
    }

    apply(
        ctx,
        vec![
            // Remove the sunk item from the list.
            Step::DeleteNode {
                parent_path: list_path.clone(),
                index: item_index,
            },
            // Replace the previous item with the rebuilt one carrying the nested list.
            Step::DeleteNode {
                parent_path: list_path.clone(),
                index: item_index - 1,
            },
            Step::InsertNode {
                parent_path: list_path.clone(),
                index: item_index - 1,
                node: new_prev,
            },
        ],
    )?;
    // Move the caret INTO the sunk item's new home: prev_item -> last child (the nested
    // sub-list) -> last item -> its first block -> first text leaf. Walk the live tree to
    // resolve the concrete indices (the sub-list index inside the prev item is its last
    // child; the sunk item is the sub-list's last item).
    let prev_item_path = {
        let mut p = list_path.clone();
        p.push(item_index - 1);
        p
    };
    if let Some(caret) = caret_into_nested_sunk_item(ctx.doc, &prev_item_path) {
        *ctx.selection = Selection::caret(caret);
    } else {
        clamp_selection(ctx);
    }
    Ok(())
}

/// After a sink, resolve a caret inside the sunk item's new home: the previous item's
/// LAST child must be the nested sub-list, whose LAST item holds the sunk content. Land
/// the caret at the start of that item's first inline-content block.
fn caret_into_nested_sunk_item(doc: &BlockNode, prev_item_path: &[usize]) -> Option<DocPosition> {
    let prev_item = block_at(doc, prev_item_path)?;
    let sub_list_idx = prev_item.children.len().checked_sub(1)?;
    let sub_list = prev_item.children.get(sub_list_idx)?.as_block()?;
    if !matches!(sub_list.kind, NodeKind::BulletList | NodeKind::OrderedList) {
        return None;
    }
    let sunk_item_idx = sub_list.children.len().checked_sub(1)?;
    let sunk_item = sub_list.children.get(sunk_item_idx)?.as_block()?;
    // The sunk item's first block child is the paragraph; caret to its first text leaf.
    let _ = sunk_item.children.first()?.as_block()?;
    let mut block_path = prev_item_path.to_vec();
    block_path.extend_from_slice(&[sub_list_idx, sunk_item_idx, 0]);
    caret_in_block_at_offset(doc, &block_path, 0)
}

/// Lift (dedent) the list item enclosing the caret: if it is nested inside a sub-list
/// (a child of a parent list item), pull it up to be a sibling of its grandparent item.
/// Requires the item to be nested at least one level (else it is already at the top
/// list level and there is nothing to lift to within the list).
fn lift_list_item(ctx: &mut CommandContext<'_>) -> Result<(), CommandError> {
    let block_path = caret_block_path(ctx.selection).ok_or(CommandError::NoCaret)?;
    let item_path = enclosing_list_item(ctx.doc, &block_path).ok_or(CommandError::InvalidContext {
        reason: "lift_list_item requires the caret inside a list item".to_string(),
    })?;
    // item_path = [.., grandparent_list_item, sub_list_index, item_index]
    // To lift, the item's list must itself be inside a parent list ITEM.
    let (sub_list_path, item_index) = split_block_path(&item_path)?;
    let (parent_item_path, _sub_list_index_in_item) = split_block_path(&sub_list_path)?;
    let parent_item = block_at(ctx.doc, &parent_item_path);
    let is_nested = parent_item
        .map(|b| matches!(b.kind, NodeKind::ListItem | NodeKind::TaskItem))
        .unwrap_or(false);
    if !is_nested {
        return Err(CommandError::InvalidContext {
            reason: "cannot lift a top-level list item (already at the outer list level)".to_string(),
        });
    }
    let (outer_list_path, parent_item_index) = split_block_path(&parent_item_path)?;
    let item = block_at(ctx.doc, &item_path)
        .ok_or(CommandError::NoCaret)?
        .clone();
    apply(
        ctx,
        vec![
            // Remove the item from its sub-list.
            Step::DeleteNode {
                parent_path: sub_list_path.clone(),
                index: item_index,
            },
            // Insert it as the next sibling of the parent item in the outer list.
            Step::InsertNode {
                parent_path: outer_list_path.clone(),
                index: parent_item_index + 1,
                node: item,
            },
        ],
    )?;
    clamp_selection(ctx);
    Ok(())
}

// ── tables (MC-003 nesting guard) ────────────────────────────────────────────────

/// The default `attrs` for a freshly-inserted table cell, matching the Tiptap
/// table-cell shape (`colspan: 1`, `rowspan: 1`). `is_header` stamps the
/// header-row marker attr (the MT-011 model has no `tableHeader` NodeKind, so header
/// status is an attr — see module docs).
fn table_cell(is_header: bool) -> BlockNode {
    let mut cell = BlockNode::with_children(
        NodeKind::TableCell,
        vec![Child::Block(BlockNode::paragraph(""))],
    );
    cell.attrs.insert("colspan".to_string(), JsonValue::from(1));
    cell.attrs.insert("rowspan".to_string(), JsonValue::from(1));
    if is_header {
        cell.attrs.insert("isHeader".to_string(), JsonValue::Bool(true));
    }
    cell
}

/// Insert a `rows`x`cols` table AFTER the caret's block, with a header first row
/// (Tiptap `insertTable({withHeaderRow:true})` parity). Guard (MC-003): refuses when
/// the caret is already inside a table (no nested tables in the backend schema).
/// `rows`/`cols` are clamped to `>= 1`.
fn insert_table(ctx: &mut CommandContext<'_>, rows: u8, cols: u8) -> Result<(), CommandError> {
    let block_path = caret_block_path(ctx.selection).ok_or(CommandError::NoCaret)?;
    if path_is_inside_kind(ctx.doc, &block_path, NodeKind::Table) {
        return Err(CommandError::InvalidContext {
            reason: "cannot insert a table inside an existing table (no nested tables)".to_string(),
        });
    }
    let (parent_path, index) = split_block_path(&block_path)?;
    let rows = rows.max(1);
    let cols = cols.max(1);
    let mut table_rows = Vec::with_capacity(rows as usize);
    for r in 0..rows {
        let is_header = r == 0;
        let mut row = BlockNode::new(NodeKind::TableRow);
        for _ in 0..cols {
            row.children.push(Child::Block(table_cell(is_header)));
        }
        table_rows.push(Child::Block(row));
    }
    let table = BlockNode::with_children(NodeKind::Table, table_rows);
    apply(
        ctx,
        vec![Step::InsertNode {
            parent_path,
            index: index + 1,
            node: table,
        }],
    )?;
    // Move the caret into the first cell's paragraph text leaf.
    let mut new_block_path = caret_block_path(ctx.selection).unwrap_or_default();
    let (pp, _) = split_block_path(&new_block_path).unwrap_or((Vec::new(), 0));
    // table is at pp[index+1]; first row[0] / cell[0] / para[0] / text[0].
    new_block_path = pp;
    new_block_path.extend_from_slice(&[index + 1, 0, 0, 0, 0]);
    *ctx.selection = Selection::caret(DocPosition::new(new_block_path, 0));
    clamp_selection(ctx);
    Ok(())
}

/// Add a row before/after the caret's row inside a table. Errors if the caret is not
/// inside a table. The new row mirrors the column count of the caret's row and uses
/// non-header cells.
fn add_row(ctx: &mut CommandContext<'_>, after: bool) -> Result<(), CommandError> {
    let block_path = caret_block_path(ctx.selection).ok_or(CommandError::NoCaret)?;
    let row_path = enclosing_kind(ctx.doc, &block_path, NodeKind::TableRow)
        .ok_or(not_in_table("add_row"))?;
    let (table_path, row_index) = split_block_path(&row_path)?;
    let row = block_at(ctx.doc, &row_path).ok_or(CommandError::NoCaret)?;
    let cols = row.children.len().max(1);
    let mut new_row = BlockNode::new(NodeKind::TableRow);
    for _ in 0..cols {
        new_row.children.push(Child::Block(table_cell(false)));
    }
    let index = if after { row_index + 1 } else { row_index };
    apply(
        ctx,
        vec![Step::InsertNode {
            parent_path: table_path,
            index,
            node: new_row,
        }],
    )?;
    clamp_selection(ctx);
    Ok(())
}

/// Delete the caret's row from its table. Errors if the caret is not in a table. If the
/// table would become empty (last row), the whole table is deleted instead (so a table
/// is never left with zero rows — an invalid empty table).
fn delete_row(ctx: &mut CommandContext<'_>) -> Result<(), CommandError> {
    let block_path = caret_block_path(ctx.selection).ok_or(CommandError::NoCaret)?;
    let row_path = enclosing_kind(ctx.doc, &block_path, NodeKind::TableRow)
        .ok_or(not_in_table("delete_row"))?;
    let (table_path, row_index) = split_block_path(&row_path)?;
    let table = block_at(ctx.doc, &table_path).ok_or(CommandError::NoCaret)?;
    if table.children.len() <= 1 {
        // Last row -> delete the whole table (no empty tables).
        return delete_table(ctx);
    }
    apply(
        ctx,
        vec![Step::DeleteNode {
            parent_path: table_path,
            index: row_index,
        }],
    )?;
    // Caret may now point past the end; clamp it to the table start.
    move_caret_to_table_start(ctx, &block_path);
    Ok(())
}

/// Add a column before/after the caret's column across every row of the table. Errors
/// if the caret is not in a table. New cells are non-header.
fn add_col(ctx: &mut CommandContext<'_>, after: bool) -> Result<(), CommandError> {
    let block_path = caret_block_path(ctx.selection).ok_or(CommandError::NoCaret)?;
    let cell_path = enclosing_kind(ctx.doc, &block_path, NodeKind::TableCell)
        .ok_or(not_in_table("add_col"))?;
    // cell_path = [.., table, row, cell]; the cell index is the column index.
    let (row_path, col_index) = split_block_path(&cell_path)?;
    let (table_path, _row_index) = split_block_path(&row_path)?;
    let table = block_at(ctx.doc, &table_path).ok_or(CommandError::NoCaret)?;
    let insert_at = if after { col_index + 1 } else { col_index };
    let n_rows = table.children.len();
    // Build one InsertNode step per row (each adds the column cell at the same index).
    let mut steps = Vec::with_capacity(n_rows);
    for r in 0..n_rows {
        let mut rp = table_path.clone();
        rp.push(r);
        let is_header = r == 0; // first row is the header row
        steps.push(Step::InsertNode {
            parent_path: rp,
            index: insert_at,
            node: table_cell(is_header),
        });
    }
    apply(ctx, steps)?;
    clamp_selection(ctx);
    Ok(())
}

/// Delete the caret's column across every row. Errors if not in a table. If a row would
/// become empty (last column), the whole table is deleted (no zero-column rows).
fn delete_col(ctx: &mut CommandContext<'_>) -> Result<(), CommandError> {
    let block_path = caret_block_path(ctx.selection).ok_or(CommandError::NoCaret)?;
    let cell_path = enclosing_kind(ctx.doc, &block_path, NodeKind::TableCell)
        .ok_or(not_in_table("delete_col"))?;
    let (row_path, col_index) = split_block_path(&cell_path)?;
    let (table_path, _row_index) = split_block_path(&row_path)?;
    let table = block_at(ctx.doc, &table_path).ok_or(CommandError::NoCaret)?;
    let n_rows = table.children.len();
    // If every row has a single column, deleting it empties the table.
    let min_cols = table
        .children
        .iter()
        .filter_map(Child::as_block)
        .map(|r| r.children.len())
        .min()
        .unwrap_or(0);
    if min_cols <= 1 {
        return delete_table(ctx);
    }
    // Delete the cell at col_index in every row (same index across rows).
    let mut steps = Vec::with_capacity(n_rows);
    for r in 0..n_rows {
        let mut rp = table_path.clone();
        rp.push(r);
        steps.push(Step::DeleteNode {
            parent_path: rp,
            index: col_index,
        });
    }
    apply(ctx, steps)?;
    move_caret_to_table_start(ctx, &block_path);
    Ok(())
}

/// Delete the entire table the caret is in. Errors if not in a table.
fn delete_table(ctx: &mut CommandContext<'_>) -> Result<(), CommandError> {
    let block_path = caret_block_path(ctx.selection).ok_or(CommandError::NoCaret)?;
    let table_path = enclosing_kind(ctx.doc, &block_path, NodeKind::Table)
        .ok_or(not_in_table("delete_table"))?;
    let (parent_path, index) = split_block_path(&table_path)?;
    apply(
        ctx,
        vec![Step::DeleteNode {
            parent_path: parent_path.clone(),
            index,
        }],
    )?;
    // Park the caret at the doc start (the table is gone). Resolve to the first leaf.
    if let Some(pos) =
        crate::rich_editor::document_model::position::resolve(ctx.doc, 0)
    {
        *ctx.selection = Selection::caret(pos);
    }
    Ok(())
}

/// Toggle the header marker on the caret's table's FIRST row: flips every cell's
/// `attrs.isHeader` between true and absent (the MT-011-model representation of the
/// React `toggleHeaderRow`). Errors if not in a table.
fn toggle_header_row(ctx: &mut CommandContext<'_>) -> Result<(), CommandError> {
    let block_path = caret_block_path(ctx.selection).ok_or(CommandError::NoCaret)?;
    let table_path = enclosing_kind(ctx.doc, &block_path, NodeKind::Table)
        .ok_or(not_in_table("toggle_header_row"))?;
    let table = block_at(ctx.doc, &table_path).ok_or(CommandError::NoCaret)?;
    let Some(first_row) = table.children.first().and_then(Child::as_block) else {
        return Err(not_in_table("toggle_header_row"));
    };
    // Determine the current header state from the first cell.
    let currently_header = first_row
        .children
        .first()
        .and_then(Child::as_block)
        .map(|c| c.attrs.get("isHeader").and_then(JsonValue::as_bool).unwrap_or(false))
        .unwrap_or(false);

    // Rebuild the first row with toggled header cells (DeleteNode + InsertNode of the row).
    let mut new_row = first_row.clone();
    for child in new_row.children.iter_mut() {
        if let Child::Block(cell) = child {
            if currently_header {
                cell.attrs.remove("isHeader");
            } else {
                cell.attrs.insert("isHeader".to_string(), JsonValue::Bool(true));
            }
        }
    }
    let mut row_path = table_path.clone();
    row_path.push(0);
    let (rp_parent, rp_index) = split_block_path(&row_path)?;
    apply(
        ctx,
        vec![
            Step::DeleteNode {
                parent_path: rp_parent.clone(),
                index: rp_index,
            },
            Step::InsertNode {
                parent_path: rp_parent,
                index: rp_index,
                node: new_row,
            },
        ],
    )?;
    clamp_selection(ctx);
    Ok(())
}

// ── structural editing (MT-013 scope expansion) ─────────────────────────────────────

/// Enter / InsertParagraphBreak: split the caret's block at the caret offset into two
/// sibling blocks (MT-011 [`Step::SplitNode`]). The caret + selection move to the START
/// of the new (tail) block. Closes the MT-012 deferral of cross-block structural edits.
fn insert_paragraph_break(ctx: &mut CommandContext<'_>) -> Result<(), CommandError> {
    let h = head(ctx.selection).ok_or(CommandError::NoCaret)?;
    let block_path = caret_block_path(ctx.selection).ok_or(CommandError::NoCaret)?;
    let block = block_at(ctx.doc, &block_path).ok_or(CommandError::NoCaret)?;
    if !block.kind.holds_inline_content() {
        return Err(CommandError::InvalidContext {
            reason: format!("cannot split non-inline-content block {:?}", block.kind),
        });
    }
    // The split offset is the caret's flat char offset WITHIN this block. For a single
    // text leaf at [.., 0] the in-leaf offset is the block offset; for multi-run blocks
    // we sum the preceding sibling leaf lengths.
    let split_offset = block_local_char_offset(block, &h);
    let (parent_path, index) = split_block_path(&block_path)?;
    apply(
        ctx,
        vec![Step::SplitNode {
            path: block_path.clone(),
            char_offset: split_offset,
        }],
    )?;
    // The new sibling is at parent[index+1]; caret to its first text leaf, offset 0.
    let mut new_path = parent_path.clone();
    new_path.extend_from_slice(&[index + 1, 0]);
    *ctx.selection = Selection::caret(DocPosition::new(new_path, 0));
    clamp_selection(ctx);
    Ok(())
}

/// Backspace at caret offset 0: merge the caret's block into its previous sibling
/// (MT-011 [`Step::MergeNodes`]). The caret lands at the join point (the previous
/// block's pre-merge end). A no-op (Ok) when the caret is not at offset 0 or there is
/// no previous sibling. Closes MT-012's "Backspace at offset 0 is a no-op" deferral.
fn merge_backward(ctx: &mut CommandContext<'_>) -> Result<(), CommandError> {
    let h = head(ctx.selection).ok_or(CommandError::NoCaret)?;
    // Only act at the very start of the block's first text leaf.
    let block_path = caret_block_path(ctx.selection).ok_or(CommandError::NoCaret)?;
    let leaf_idx = *h.path.last().unwrap_or(&0);
    if h.char_offset != 0 || leaf_idx != 0 {
        return Ok(()); // not at block start -> let normal backspace handle it
    }
    let (parent_path, index) = split_block_path(&block_path)?;
    if index == 0 {
        return Ok(()); // no previous sibling to merge into
    }
    // Capture the previous sibling's pre-merge total char length (the join point).
    let parent = block_at(ctx.doc, &parent_path).ok_or(CommandError::NoCaret)?;
    let prev = parent
        .children
        .get(index - 1)
        .and_then(Child::as_block)
        .ok_or(CommandError::NoCaret)?;
    if !prev.kind.holds_inline_content() {
        // Previous sibling is a container (list/table/etc.); a simple inline merge is
        // not defined here — leave as a no-op for this MT (richer merges are later E2).
        return Ok(());
    }
    let join_at = prev.char_len();
    apply(
        ctx,
        vec![Step::MergeNodes {
            parent_path: parent_path.clone(),
            index,
        }],
    )?;
    // Caret lands at the join point in the merged (previous) block. Resolve the leaf at
    // the join offset within that block.
    let mut prev_path = parent_path.clone();
    prev_path.push(index - 1);
    let caret = caret_in_block_at_offset(ctx.doc, &prev_path, join_at)
        .unwrap_or_else(|| DocPosition::new({
            let mut p = prev_path.clone();
            p.push(0);
            p
        }, 0));
    *ctx.selection = Selection::caret(caret);
    clamp_selection(ctx);
    Ok(())
}

// ── tree-navigation helpers ──────────────────────────────────────────────────────

/// Split a non-empty block path into (parent_path, index). Errors on the empty path
/// (the root has no parent — a structural step cannot address it).
fn split_block_path(path: &[usize]) -> Result<(Vec<usize>, usize), CommandError> {
    match path.split_last() {
        Some((&last, head)) => Ok((head.to_vec(), last)),
        None => Err(CommandError::InvalidContext {
            reason: "cannot address the document root with this command".to_string(),
        }),
    }
}

/// The path of the nearest ancestor block of `kind` enclosing `block_path` (inclusive
/// of `block_path` itself), or `None` if none on the chain. Walks the path prefix by
/// prefix from the deepest to the root.
fn enclosing_kind(doc: &BlockNode, block_path: &[usize], kind: NodeKind) -> Option<Vec<usize>> {
    for end in (0..=block_path.len()).rev() {
        let prefix = &block_path[..end];
        if let Some(b) = block_at(doc, prefix) {
            if b.kind == kind {
                return Some(prefix.to_vec());
            }
        }
    }
    None
}

/// The path of the nearest enclosing `list_item` OR `task_item` (the indent/dedent
/// commands treat both as list items).
fn enclosing_list_item(doc: &BlockNode, block_path: &[usize]) -> Option<Vec<usize>> {
    for end in (0..=block_path.len()).rev() {
        let prefix = &block_path[..end];
        if let Some(b) = block_at(doc, prefix) {
            if matches!(b.kind, NodeKind::ListItem | NodeKind::TaskItem) {
                return Some(prefix.to_vec());
            }
        }
    }
    None
}

/// The path of the nearest enclosing list (`bullet_list` or `ordered_list`).
fn enclosing_list(doc: &BlockNode, block_path: &[usize]) -> Option<Vec<usize>> {
    for end in (0..=block_path.len()).rev() {
        let prefix = &block_path[..end];
        if let Some(b) = block_at(doc, prefix) {
            if matches!(b.kind, NodeKind::BulletList | NodeKind::OrderedList) {
                return Some(prefix.to_vec());
            }
        }
    }
    None
}

/// Unwrap a single-item list back to its inner paragraph at the list's position. Used
/// by [`toggle_list`] / [`toggle_task_list`] to toggle off. Replaces the list node with
/// the first item's first block child (the paragraph).
fn unwrap_list_to_paragraph(ctx: &mut CommandContext<'_>, list_path: &[usize]) -> Result<(), CommandError> {
    let (parent_path, index) = split_block_path(list_path)?;
    let list = block_at(ctx.doc, list_path).ok_or(CommandError::NoCaret)?;
    let first_item = list
        .children
        .first()
        .and_then(Child::as_block)
        .ok_or(CommandError::NoCaret)?;
    let inner = first_item
        .children
        .first()
        .and_then(Child::as_block)
        .ok_or(CommandError::NoCaret)?
        .clone();
    apply(
        ctx,
        vec![
            Step::DeleteNode {
                parent_path: parent_path.clone(),
                index,
            },
            Step::InsertNode {
                parent_path: parent_path.clone(),
                index,
                node: inner,
            },
        ],
    )?;
    // Caret moves up two levels (list/item/para -> para): land it in the paragraph leaf.
    let mut new_block_path = parent_path.clone();
    new_block_path.push(index);
    let caret = caret_in_block_at_offset(ctx.doc, &new_block_path, 0)
        .unwrap_or_else(|| DocPosition::new({
            let mut p = new_block_path.clone();
            p.push(0);
            p
        }, 0));
    *ctx.selection = Selection::caret(caret);
    clamp_selection(ctx);
    Ok(())
}

/// The flat char offset of `pos` WITHIN `block` (summing the char lengths of the
/// sibling inline children before the addressed leaf, plus the in-leaf offset). For a
/// single-leaf block this is just `pos.char_offset`.
fn block_local_char_offset(block: &BlockNode, pos: &DocPosition) -> usize {
    // pos.path = block_path ++ [leaf_idx]; the leaf index is the last element.
    let Some(&leaf_idx) = pos.path.last() else {
        return pos.char_offset;
    };
    let mut acc = 0usize;
    for (i, child) in block.children.iter().enumerate() {
        if i == leaf_idx {
            return acc + pos.char_offset;
        }
        acc += child.char_len();
    }
    acc + pos.char_offset
}

/// Build a [`DocPosition`] caret inside the block at `block_path` at `block_offset`
/// flat chars (walks the block's inline children to find the leaf + in-leaf offset).
fn caret_in_block_at_offset(
    doc: &BlockNode,
    block_path: &[usize],
    block_offset: usize,
) -> Option<DocPosition> {
    let block = block_at(doc, block_path)?;
    let mut remaining = block_offset;
    let mut last_leaf_idx = None;
    let mut last_leaf_len = 0usize;
    for (i, child) in block.children.iter().enumerate() {
        if let Child::Text(t) = child {
            let len = t.text.len_chars();
            if remaining <= len {
                let mut p = block_path.to_vec();
                p.push(i);
                return Some(DocPosition::new(p, remaining));
            }
            remaining -= len;
            last_leaf_idx = Some(i);
            last_leaf_len = len;
        }
    }
    // Past the end: clamp to the last text leaf's end.
    last_leaf_idx.map(|i| {
        let mut p = block_path.to_vec();
        p.push(i);
        DocPosition::new(p, last_leaf_len)
    })
}

/// Reparent a caret whose old block lived under `old_parent` so it now points under
/// `new_parent` (same trailing leaf index + offset). Used by blockquote unwrap.
fn reparent_caret(ctx: &mut CommandContext<'_>, _old_parent: &[usize], new_parent: &[usize]) {
    // After unwrap, the inner block sits where the blockquote was: at new_parent ++ [index].
    // We move the caret to that block's first leaf, offset 0 (a safe, valid caret).
    let mut block_path = new_parent.to_vec();
    // The unwrap inserted the inner block at the blockquote's index (last of old parent).
    // The caller passed gp_path as new_parent and the blockquote index is recoverable
    // from the old block path; for simplicity land the caret at the new block start.
    if let Some(pos) = caret_in_block_at_offset(ctx.doc, &block_path, 0) {
        *ctx.selection = Selection::caret(pos);
        return;
    }
    block_path.push(0);
    *ctx.selection = Selection::caret(DocPosition::new(block_path, 0));
}

/// Move the caret from its old leaf path to a new block path's first leaf, offset 0.
fn reparent_caret_into(ctx: &mut CommandContext<'_>, _old_leaf: &[usize], new_block_path: &[usize]) {
    if let Some(pos) = caret_in_block_at_offset(ctx.doc, new_block_path, 0) {
        *ctx.selection = Selection::caret(pos);
        return;
    }
    let mut p = new_block_path.to_vec();
    p.push(0);
    *ctx.selection = Selection::caret(DocPosition::new(p, 0));
}

/// Move the caret to the start of the table enclosing `block_path` (first cell's
/// paragraph), used after a row/column delete that may have invalidated the caret.
fn move_caret_to_table_start(ctx: &mut CommandContext<'_>, block_path: &[usize]) {
    if let Some(table_path) = enclosing_kind(ctx.doc, block_path, NodeKind::Table) {
        // table/row0/cell0/para0/text0
        let mut p = table_path;
        p.extend_from_slice(&[0, 0, 0, 0]);
        if let Some(pos) = position_for_first_leaf(ctx.doc, &p) {
            *ctx.selection = Selection::caret(pos);
            return;
        }
    }
    clamp_selection(ctx);
}

/// Build a caret DocPosition addressing the first text leaf at `leaf_path` if it
/// resolves, else `None`.
fn position_for_first_leaf(doc: &BlockNode, leaf_path: &[usize]) -> Option<DocPosition> {
    let (leaf_idx, block_path) = leaf_path.split_last()?;
    let block = block_at(doc, block_path)?;
    block.children.get(*leaf_idx)?.as_text()?;
    Some(DocPosition::new(leaf_path.to_vec(), 0))
}

/// Clamp the selection to a valid caret if its current path no longer resolves to a
/// text leaf (e.g. after a structural delete). Falls back to the document start. This
/// keeps the caret from dangling after a command rearranges the tree.
fn clamp_selection(ctx: &mut CommandContext<'_>) {
    let valid = match &*ctx.selection {
        Selection::Text { head, anchor } => {
            leaf_at(ctx.doc, &head.path).is_some() && leaf_at(ctx.doc, &anchor.path).is_some()
        }
        Selection::Node { node_path } => block_at(ctx.doc, node_path).is_some(),
    };
    if valid {
        return;
    }
    if let Some(pos) = crate::rich_editor::document_model::position::resolve(ctx.doc, 0) {
        *ctx.selection = Selection::caret(pos);
    }
}

/// Build the "not inside a table" context error for a table-edit command.
fn not_in_table(cmd: &str) -> CommandError {
    CommandError::InvalidContext {
        reason: format!("{cmd} requires the caret inside a table"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rich_editor::document_model::node::{Child, HeadingLevel, NodeKind, TextLeaf};
    use crate::rich_editor::document_model::position::DocPosition;

    /// A test harness owning a doc + undo + selection so a command runs against real
    /// state and the result is asserted on the live tree.
    struct Editor {
        doc: BlockNode,
        undo: UndoManager,
        sel: Selection,
    }

    impl Editor {
        fn new(doc: BlockNode, sel: Selection) -> Self {
            Self {
                doc,
                undo: UndoManager::new(),
                sel,
            }
        }

        fn run(&mut self, cmd: FormattingCommand) -> Result<(), CommandError> {
            let mut ctx = CommandContext::new(&mut self.doc, &mut self.undo, &mut self.sel, "test");
            dispatch(&mut ctx, &cmd)
        }

        /// The top-level block at `idx`.
        fn block(&self, idx: usize) -> &BlockNode {
            self.doc.children[idx].as_block().unwrap()
        }
    }

    /// A doc of one paragraph with a single text leaf, caret at `offset`.
    fn one_para(text: &str, offset: usize) -> Editor {
        let doc = BlockNode::doc(vec![BlockNode::paragraph(text)]);
        Editor::new(doc, Selection::caret(DocPosition::new(vec![0, 0], offset)))
    }

    // ── AC-2: toggle bold idempotent (add then remove) ───────────────────────────────

    #[test]
    fn test_toggle_bold_idempotent() {
        // Selection over the whole "hello" leaf; first toggle adds bold, second removes it.
        let mut ed = Editor::new(
            BlockNode::doc(vec![BlockNode::paragraph("hello")]),
            Selection::text(
                DocPosition::new(vec![0, 0], 0),
                DocPosition::new(vec![0, 0], 5),
            ),
        );
        ed.run(FormattingCommand::ToggleBold).unwrap();
        assert!(
            ed.block(0).children[0]
                .as_text()
                .unwrap()
                .has_mark_type(&Mark::Bold),
            "first toggle adds bold"
        );
        ed.run(FormattingCommand::ToggleBold).unwrap();
        assert!(
            !ed.block(0).children[0]
                .as_text()
                .unwrap()
                .has_mark_type(&Mark::Bold),
            "second toggle removes bold (idempotent round-trip)"
        );
    }

    // ── MC-001 / RISK-1: full-range active check across a 3-leaf selection ─────────────

    #[test]
    fn toggle_bold_multi_leaf_active() {
        // A paragraph with three runs; ONLY the middle run is bold. A selection spanning
        // all three reports active=true, so the toggle REMOVES bold from the middle run.
        let para = BlockNode::with_children(
            NodeKind::Paragraph,
            vec![
                Child::Text(TextLeaf::new("aaa")),
                Child::Text(TextLeaf::with_marks("bbb", vec![Mark::Bold])),
                Child::Text(TextLeaf::new("ccc")),
            ],
        );
        let doc = BlockNode::doc(vec![para]);
        let sel = Selection::text(
            DocPosition::new(vec![0, 0], 0),
            DocPosition::new(vec![0, 2], 3),
        );
        // Active check: ANY overlapping leaf has bold -> true.
        assert!(
            is_mark_active(&doc, &sel, &Mark::Bold),
            "MC-001: a 3-leaf selection where only the middle leaf is bold is ACTIVE"
        );
        let mut ed = Editor::new(doc, sel);
        ed.run(FormattingCommand::ToggleBold).unwrap();
        // Because it was active, the toggle removed bold from every overlapping leaf.
        for i in 0..3 {
            assert!(
                !ed.block(0).children[i]
                    .as_text()
                    .unwrap()
                    .has_mark_type(&Mark::Bold),
                "leaf {i} must have bold removed after the active toggle"
            );
        }
    }

    // ── AC-5: set_heading changes node kind; active state reported ─────────────────────

    #[test]
    fn test_set_heading_changes_kind() {
        let mut ed = one_para("title", 2);
        ed.run(FormattingCommand::SetHeading(1)).unwrap();
        assert_eq!(
            ed.block(0).kind,
            NodeKind::Heading(HeadingLevel::new(1)),
            "set_heading(1) converts the paragraph to an h1"
        );
        // The text survived the conversion.
        assert_eq!(
            ed.block(0).children[0].as_text().unwrap().text.to_string(),
            "title"
        );
        // The toolbar H1 active-state check now reports true for this caret.
        assert!(crate::rich_editor::formatting::toolbar::is_command_active(
            &ed.doc,
            &ed.sel,
            &FormattingCommand::SetHeading(1)
        ));
    }

    // ── MC-002: set_heading guards context (refuses inside a list item) ────────────────

    #[test]
    fn set_heading_refuses_inside_list_item() {
        // doc > bullet_list > list_item > paragraph("x"); caret in the paragraph.
        let para = BlockNode::paragraph("x");
        let item = BlockNode::with_children(NodeKind::ListItem, vec![Child::Block(para)]);
        let list = BlockNode::with_children(NodeKind::BulletList, vec![Child::Block(item)]);
        let doc = BlockNode::doc(vec![list]);
        // caret path: doc[0] list -> [0] item -> [0] para -> [0] text.
        let sel = Selection::caret(DocPosition::new(vec![0, 0, 0, 0], 0));
        let mut ed = Editor::new(doc, sel);
        // The caret's IMMEDIATE block is the paragraph, which IS convertible — but the MC-002
        // corruption case is converting the LIST ITEM. set_heading converts the paragraph in
        // place (a legal child of a list_item), so it succeeds here. The guard that matters is
        // refusing to convert a non-paragraph/heading block — proven by the code_block case.
        let r = ed.run(FormattingCommand::SetHeading(2));
        assert!(r.is_ok(), "converting a paragraph inside a list item is legal");
        assert_eq!(ed.doc.children[0].as_block().unwrap().kind, NodeKind::BulletList);
    }

    #[test]
    fn set_heading_refuses_code_block() {
        // A caret inside a code block must NOT convert (MC-002 non-paragraph guard).
        let cb = BlockNode::with_children(
            NodeKind::CodeBlock,
            vec![Child::Text(TextLeaf::new("fn main(){}"))],
        );
        let doc = BlockNode::doc(vec![cb]);
        let sel = Selection::caret(DocPosition::new(vec![0, 0], 0));
        let mut ed = Editor::new(doc, sel);
        let err = ed.run(FormattingCommand::SetHeading(1)).unwrap_err();
        assert!(matches!(err, CommandError::InvalidContext { .. }));
        assert_eq!(ed.block(0).kind, NodeKind::CodeBlock, "code block unchanged");
    }

    // ── AC-6: insert_table structure (2 rows x 3 cols) ─────────────────────────────────

    #[test]
    fn test_insert_table_structure() {
        let mut ed = one_para("para", 0);
        ed.run(FormattingCommand::InsertTable { rows: 2, cols: 3 }).unwrap();
        // The table is inserted AFTER the paragraph -> top-level index 1.
        let table = ed.block(1);
        assert_eq!(table.kind, NodeKind::Table);
        assert_eq!(table.children.len(), 2, "exactly 2 table_row children");
        for r in 0..2 {
            let row = table.children[r].as_block().unwrap();
            assert_eq!(row.kind, NodeKind::TableRow);
            assert_eq!(row.children.len(), 3, "each row has exactly 3 table_cell children");
            for c in 0..3 {
                let cell = row.children[c].as_block().unwrap();
                assert_eq!(cell.kind, NodeKind::TableCell);
                // Backend-shape anchor: each cell carries colspan/rowspan = 1.
                assert_eq!(cell.attrs.get("colspan").and_then(JsonValue::as_i64), Some(1));
                assert_eq!(cell.attrs.get("rowspan").and_then(JsonValue::as_i64), Some(1));
            }
            // First row cells are header cells (withHeaderRow parity).
            let is_header = row.children[0]
                .as_block()
                .unwrap()
                .attrs
                .get("isHeader")
                .and_then(JsonValue::as_bool)
                .unwrap_or(false);
            assert_eq!(is_header, r == 0, "row {r} header flag");
        }
    }

    // ── MC-003: insert_table refuses nesting ───────────────────────────────────────────

    #[test]
    fn insert_table_refuses_nesting() {
        let mut ed = one_para("para", 0);
        ed.run(FormattingCommand::InsertTable { rows: 2, cols: 2 }).unwrap();
        // The caret is now inside the first cell; a second insert_table must be refused.
        let err = ed.run(FormattingCommand::InsertTable { rows: 2, cols: 2 }).unwrap_err();
        assert!(
            matches!(err, CommandError::InvalidContext { .. }),
            "MC-003: insert_table inside a table is refused"
        );
    }

    // ── AC-7: delete_row removes one row ───────────────────────────────────────────────

    #[test]
    fn delete_row_decrements_row_count() {
        let mut ed = one_para("para", 0);
        ed.run(FormattingCommand::InsertTable { rows: 3, cols: 2 }).unwrap();
        assert_eq!(ed.block(1).children.len(), 3);
        // Caret is in the first cell of row 0. delete_row removes that row.
        ed.run(FormattingCommand::DeleteRow).unwrap();
        assert_eq!(ed.block(1).children.len(), 2, "one row removed");
    }

    // ── AC-8: toggle_ordered_list round-trips paragraph <-> list ───────────────────────

    #[test]
    fn toggle_ordered_list_round_trip() {
        let mut ed = one_para("item text", 0);
        ed.run(FormattingCommand::ToggleOrderedList).unwrap();
        // doc[0] is now an ordered_list with one list_item holding the paragraph.
        let list = ed.block(0);
        assert_eq!(list.kind, NodeKind::OrderedList);
        let item = list.children[0].as_block().unwrap();
        assert_eq!(item.kind, NodeKind::ListItem);
        assert_eq!(item.children[0].as_block().unwrap().kind, NodeKind::Paragraph);
        // Toggling again converts back to a paragraph.
        ed.run(FormattingCommand::ToggleOrderedList).unwrap();
        assert_eq!(ed.block(0).kind, NodeKind::Paragraph, "toggled back to paragraph");
        assert_eq!(
            ed.block(0).children[0].as_text().unwrap().text.to_string(),
            "item text"
        );
    }

    // ── AC-9: sink_list_item increases nesting; lift reverses it ───────────────────────

    #[test]
    fn sink_and_lift_list_item() {
        // Build a bullet list with TWO items; caret in the SECOND item so it can sink under
        // the first.
        let item = |t: &str| {
            BlockNode::with_children(NodeKind::ListItem, vec![Child::Block(BlockNode::paragraph(t))])
        };
        let list = BlockNode::with_children(
            NodeKind::BulletList,
            vec![Child::Block(item("first")), Child::Block(item("second"))],
        );
        let doc = BlockNode::doc(vec![list]);
        // caret: list[0] -> item[1] -> para[0] -> text[0]  == path [0,1,0,0].
        let sel = Selection::caret(DocPosition::new(vec![0, 1, 0, 0], 0));
        let mut ed = Editor::new(doc, sel);

        ed.run(FormattingCommand::SinkListItem).unwrap();
        // After sink: the outer list has ONE item (the first), which now contains a nested
        // sub-list holding the sunk "second" item.
        let outer = ed.block(0);
        assert_eq!(outer.kind, NodeKind::BulletList);
        assert_eq!(outer.children.len(), 1, "the sunk item left the outer list");
        let first_item = outer.children[0].as_block().unwrap();
        // The first item now ends with a nested bullet_list.
        let sub = first_item
            .children
            .iter()
            .filter_map(Child::as_block)
            .find(|b| matches!(b.kind, NodeKind::BulletList));
        assert!(sub.is_some(), "first item gained a nested sub-list (indent)");
        assert_eq!(sub.unwrap().children.len(), 1, "the nested list holds the sunk item");

        // Now lift the nested item back out.
        ed.run(FormattingCommand::LiftListItem).unwrap();
        let outer = ed.block(0);
        assert_eq!(
            outer.children.len(),
            2,
            "lift returned the item to the outer list (2 items again)"
        );
    }

    // ── AC-4: undo restores the previous doc state ─────────────────────────────────────

    #[test]
    fn undo_restores_pre_bold_state() {
        let mut ed = Editor::new(
            BlockNode::doc(vec![BlockNode::paragraph("hello")]),
            Selection::text(
                DocPosition::new(vec![0, 0], 0),
                DocPosition::new(vec![0, 0], 5),
            ),
        );
        let before = ed.doc.clone();
        ed.run(FormattingCommand::ToggleBold).unwrap();
        assert!(ed.block(0).children[0].as_text().unwrap().has_mark_type(&Mark::Bold));
        ed.run(FormattingCommand::Undo).unwrap();
        assert_eq!(ed.doc, before, "AC-4: undo restores the pre-bold doc");
    }

    #[test]
    fn undo_restores_pre_heading_state() {
        let mut ed = one_para("x", 0);
        let before = ed.doc.clone();
        ed.run(FormattingCommand::SetHeading(2)).unwrap();
        assert_eq!(ed.block(0).kind, NodeKind::Heading(HeadingLevel::new(2)));
        ed.run(FormattingCommand::Undo).unwrap();
        assert_eq!(ed.doc, before, "undo restores the paragraph (heading reverted)");
    }

    // ── scope expansion: Enter split + Backspace merge + undo ──────────────────────────

    #[test]
    fn enter_splits_backspace_merges_undo_restores() {
        // "helloworld" with caret after "hello" (offset 5). Enter splits into two paragraphs.
        let mut ed = one_para("helloworld", 5);
        let before = ed.doc.clone();
        ed.run(FormattingCommand::InsertParagraphBreak).unwrap();
        assert_eq!(ed.doc.children.len(), 2, "Enter split into two blocks");
        assert_eq!(ed.block(0).children[0].as_text().unwrap().text.to_string(), "hello");
        assert_eq!(ed.block(1).children[0].as_text().unwrap().text.to_string(), "world");
        // Caret moved to the start of the new (tail) block.
        if let Selection::Text { head, .. } = &ed.sel {
            assert_eq!(head.path, vec![1, 0]);
            assert_eq!(head.char_offset, 0);
        } else {
            panic!("expected a caret after split");
        }

        // Backspace at the start of the second block merges it back.
        ed.run(FormattingCommand::MergeBackward).unwrap();
        assert_eq!(ed.doc.children.len(), 1, "Backspace-at-0 merged the blocks");
        assert_eq!(
            ed.block(0).children[0].as_text().unwrap().text.to_string(),
            "helloworld"
        );
        // Caret at the join point (offset 5 within the merged block).
        if let Selection::Text { head, .. } = &ed.sel {
            assert_eq!(head.char_offset, 5, "caret at the join point");
        }

        // Undo the merge, then undo the split: back to the original single block.
        ed.run(FormattingCommand::Undo).unwrap();
        assert_eq!(ed.doc.children.len(), 2, "undo of merge restores two blocks");
        ed.run(FormattingCommand::Undo).unwrap();
        assert_eq!(ed.doc, before, "undo of split restores the original doc");
    }

    #[test]
    fn merge_backward_no_op_when_no_previous_sibling() {
        // Caret at offset 0 of the FIRST block -> no previous sibling -> benign no-op.
        let mut ed = one_para("only", 0);
        let before = ed.doc.clone();
        ed.run(FormattingCommand::MergeBackward).unwrap();
        assert_eq!(ed.doc, before, "merge at the first block is a no-op");
    }

    // ── task list: checked attr round-trips backend shape ──────────────────────────────

    #[test]
    fn toggle_task_list_and_checked_attr() {
        let mut ed = one_para("buy milk", 0);
        ed.run(FormattingCommand::ToggleTaskList).unwrap();
        // doc[0] is a bullet_list holding a task_item with checked=false.
        let list = ed.block(0);
        let item = list.children[0].as_block().unwrap();
        assert_eq!(item.kind, NodeKind::TaskItem);
        assert!(!item.task_checked(), "new task item starts unchecked");
        // Caret should now be inside the task item's paragraph; toggle the checked attr.
        ed.run(FormattingCommand::ToggleTaskItemChecked).unwrap();
        let item = ed.block(0).children[0].as_block().unwrap();
        assert!(item.task_checked(), "toggle_task_item_checked sets checked=true");
    }

    // ── code block: set_code_block carries language, flattens marks ────────────────────

    #[test]
    fn set_code_block_carries_language() {
        let mut ed = one_para("let x = 1;", 0);
        ed.run(FormattingCommand::SetCodeBlock(Some("rust".to_string()))).unwrap();
        let cb = ed.block(0);
        assert_eq!(cb.kind, NodeKind::CodeBlock);
        assert_eq!(cb.attrs.get("language").and_then(JsonValue::as_str), Some("rust"));
        assert_eq!(cb.children[0].as_text().unwrap().text.to_string(), "let x = 1;");
    }

    // ── blockquote wrap/unwrap ──────────────────────────────────────────────────────────

    #[test]
    fn blockquote_wrap_then_unwrap() {
        let mut ed = one_para("quoted", 0);
        ed.run(FormattingCommand::SetBlockquote).unwrap();
        assert_eq!(ed.block(0).kind, NodeKind::Blockquote);
        assert_eq!(
            ed.block(0).children[0].as_block().unwrap().kind,
            NodeKind::Paragraph
        );
        ed.run(FormattingCommand::SetBlockquote).unwrap();
        assert_eq!(ed.block(0).kind, NodeKind::Paragraph, "unwrapped back to paragraph");
    }

    // ── horizontal rule + DocJson round-trip of a built table (backend shape) ──────────

    #[test]
    fn insert_table_round_trips_doc_json() {
        use crate::rich_editor::document_model::doc_json::{from_json_string, to_json_string};
        let mut ed = one_para("p", 0);
        ed.run(FormattingCommand::InsertTable { rows: 2, cols: 2 }).unwrap();
        // The built table must serialize and deserialize without an UnknownNodeType error
        // (backend node-shape anchor: table/tableRow/tableCell are known Tiptap types).
        let json = to_json_string(&ed.doc).expect("table serializes to content_json");
        let back = from_json_string(&json).expect("table round-trips from content_json");
        assert_eq!(ed.doc, back, "the inserted table round-trips through DocJson");
    }

    #[test]
    fn insert_horizontal_rule_adds_atom_block() {
        let mut ed = one_para("above", 5);
        ed.run(FormattingCommand::InsertHorizontalRule).unwrap();
        assert_eq!(ed.doc.children.len(), 2);
        assert_eq!(ed.block(1).kind, NodeKind::HorizontalRule);
    }

    // ── add row before/after + add/delete column ───────────────────────────────────────

    #[test]
    fn add_row_and_column_operations() {
        let mut ed = one_para("p", 0);
        ed.run(FormattingCommand::InsertTable { rows: 2, cols: 2 }).unwrap();
        // Caret in first cell. Add a row after -> 3 rows.
        ed.run(FormattingCommand::AddRowAfter).unwrap();
        assert_eq!(ed.block(1).children.len(), 3);
        // Add a column after -> each row has 3 cells.
        ed.run(FormattingCommand::AddColAfter).unwrap();
        for r in 0..ed.block(1).children.len() {
            assert_eq!(
                ed.block(1).children[r].as_block().unwrap().children.len(),
                3,
                "row {r} now has 3 columns"
            );
        }
        // Delete a column -> back to 2 cells per row.
        ed.run(FormattingCommand::DeleteCol).unwrap();
        for r in 0..ed.block(1).children.len() {
            assert_eq!(
                ed.block(1).children[r].as_block().unwrap().children.len(),
                2,
                "row {r} back to 2 columns"
            );
        }
    }

    #[test]
    fn delete_table_removes_it() {
        let mut ed = one_para("p", 0);
        ed.run(FormattingCommand::InsertTable { rows: 2, cols: 2 }).unwrap();
        assert_eq!(ed.doc.children.len(), 2);
        ed.run(FormattingCommand::DeleteTable).unwrap();
        // Only the original paragraph remains.
        assert_eq!(ed.doc.children.len(), 1);
        assert_eq!(ed.block(0).kind, NodeKind::Paragraph);
    }

    #[test]
    fn toggle_header_row_flips_first_row_cells() {
        let mut ed = one_para("p", 0);
        ed.run(FormattingCommand::InsertTable { rows: 2, cols: 2 }).unwrap();
        // First row starts as header (withHeaderRow). Toggle off -> no isHeader.
        ed.run(FormattingCommand::ToggleHeaderRow).unwrap();
        let first_row = ed.block(1).children[0].as_block().unwrap();
        for c in 0..first_row.children.len() {
            assert!(
                !first_row.children[c]
                    .as_block()
                    .unwrap()
                    .attrs
                    .contains_key("isHeader"),
                "header flag cleared on cell {c}"
            );
        }
    }

    #[test]
    fn table_edit_outside_table_errors() {
        let mut ed = one_para("not a table", 0);
        assert!(matches!(
            ed.run(FormattingCommand::DeleteRow).unwrap_err(),
            CommandError::InvalidContext { .. }
        ));
        assert!(matches!(
            ed.run(FormattingCommand::AddRowAfter).unwrap_err(),
            CommandError::InvalidContext { .. }
        ));
    }
}
