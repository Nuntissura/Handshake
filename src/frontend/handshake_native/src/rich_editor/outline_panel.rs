//! Heading Outline / Table-of-Contents navigator for the native rich-text editor (WP-KERNEL-012 MT-056).
//!
//! This is the rich-text analog of the MT-006 CODE-editor outline ([`crate::code_editor::outline`]) and
//! is deliberately DISTINCT from it (RISK-005 / MC-005):
//! - the code outline walks tree-sitter SYMBOLS; this one walks the MT-011 block [`DocModel`] for
//!   `Heading` nodes;
//! - the code outline lives in `code_editor/outline.rs`; this lives here;
//! - the code outline's AccessKit ids are its own; this one uses the `outline.heading.{block_id}`
//!   namespace, which cannot collide with the code outline's ids NOR with the editor's per-block
//!   `re-block-{hash}` nodes.
//!
//! ## What it does
//!
//! [`build_outline`] is a PURE function over `&BlockNode` (the `doc` root) — no egui, no IO — so the
//! tree-construction logic is unit-testable in isolation (AC-001 / AC-002). It collects every top-level
//! `Heading` block at level 1..=3 in document order and folds the flat ordered list into a nested tree
//! by a standard heading-stack algorithm.
//!
//! [`OutlinePanel::show`] renders that tree as an indented, collapsible, clickable list inside the
//! editor's side tile, REUSING:
//! - the WP-011 theme tokens ([`crate::theme::HsPalette`]) for every color (no hardcoded hex);
//! - the WP-011 AccessKit live-emission path (`Context::accesskit_node_builder`) for the
//!   `rich-editor-outline` (Role::Tree) container + `outline.heading.{block_id}` (Role::TreeItem)
//!   entries, each carrying an `Action::Click` (the "Press" invoke verb) so a swarm agent can invoke
//!   scroll-to-heading deterministically (AC-005);
//! - the editor's EXISTING block-addressing ([`crate::rich_editor::renderer::block_author_id`]) for the
//!   `block_id`, the EXISTING selection model + scroll area for the navigation path
//!   ([`RichEditorWidget::scroll_to_block`] / [`RichEditorWidget::select_block`]) — NO second selection
//!   or scroll mechanism (RISK-002 / MC-002).
//!
//! ## Coordinate-space reconciliation (the contract's `byte_offset`)
//!
//! The MT contract names a per-entry `byte_offset` "from the DocModel block index (the same offset basis
//! MT-012 uses for caret positioning)". The real MT-011/MT-012 model is CHAR-indexed (see
//! `document_model/position.rs` — `absolute_offset` returns a flat CHAR offset), and a byte offset that
//! landed inside a multi-byte UTF-8 char would corrupt addressing. So the recorded offset is the flat
//! absolute CHAR offset of the heading's first text position — MT-012's actual coordinate space — and
//! the field is named [`OutlineNode::char_offset`] to be honest about the unit.
//!
//! ## block_id reconciliation
//!
//! The MT contract names a `block_id: String`, but the real DocModel has NO stable per-block string id —
//! blocks are addressed by their child-index PATH (`Vec<usize>`), which already backs the editor's
//! `re-block-{hash}` AccessKit ids. So `OutlineNode::block_id` is the editor's canonical block address:
//! the `re-block-{hash}` string ([`block_author_id`]) of the heading's top-level path. This keeps the
//! outline and the editor's block nodes on ONE addressing scheme (a swarm agent that reads a
//! `re-block-{hash}` node and an `outline.heading.re-block-{hash}` entry knows they are the same block).

use std::sync::{Arc, Mutex};

use egui::accesskit;

use crate::rich_editor::document_model::node::{BlockNode, Child};
use crate::rich_editor::document_model::position::{absolute_offset, DocPosition};
use crate::rich_editor::renderer::block_author_id;
use crate::rich_editor::renderer::rich_editor_widget::{RichEditorState, RichEditorWidget};
use crate::theme::HsPalette;

/// The AccessKit author_id of the outline panel container (`Role::Tree`). A swarm agent addresses the
/// whole outline by this stable key (AC-005).
pub const OUTLINE_CONTAINER_AUTHOR_ID: &str = "rich-editor-outline";

/// The author_id prefix for one outline heading entry (`Role::TreeItem`). The full id is
/// `outline.heading.{block_id}` where `{block_id}` is the heading's `re-block-{hash}` block address. This
/// namespace is distinct from the MT-006 code-outline ids and from the editor's `re-block-{hash}` block
/// nodes (RISK-005 / MC-005), so addressing is never ambiguous.
pub const OUTLINE_ENTRY_AUTHOR_ID_PREFIX: &str = "outline.heading.";

/// Horizontal indent (points) per heading level. A level-N heading is indented `(level-1) * INDENT_PX`.
const INDENT_PX: f32 = 14.0;

/// The fixed AccessKit/egui id band base for the outline panel's nodes. Disjoint from the editor root
/// (1_000_000) and the per-block band (2_000_000+) by sitting in a higher band, so an outline node can
/// never collide with an editor block node or shell chrome.
const OUTLINE_NODE_ID_BASE: u64 = 3_000_000;

/// Build the stable AccessKit author_id for the outline entry of the heading addressed by `block_id`:
/// `outline.heading.{block_id}` (AC-005). `block_id` is the heading's `re-block-{hash}` block address.
pub fn outline_entry_author_id(block_id: &str) -> String {
    format!("{OUTLINE_ENTRY_AUTHOR_ID_PREFIX}{block_id}")
}

/// One node in the heading outline tree.
///
/// `block_id` is the heading's stable editor block address (`re-block-{hash}` — see module docs);
/// `char_offset` is the flat absolute CHAR offset of the heading's first text position (MT-012's
/// coordinate space — see module docs). `children` are deeper-level headings nested under this one by
/// the heading-stack fold. `collapsed` is the user's expand/collapse choice, carried forward by
/// `block_id` across revision-gated rebuilds (RISK-004 / MC-004 / AC-006).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutlineNode {
    /// The heading's stable editor block address (`re-block-{hash}`), reconciling the contract's
    /// `block_id` to the editor's real path-based addressing.
    pub block_id: String,
    /// The heading level (1..=3).
    pub level: u8,
    /// The heading's plain text (the entry label).
    pub text: String,
    /// The flat absolute CHAR offset of the heading's first text position (the contract's `byte_offset`,
    /// reconciled to the real char-indexed MT-012 coordinate space).
    pub char_offset: usize,
    /// Headings nested under this one (deeper levels).
    pub children: Vec<OutlineNode>,
    /// Whether this node is collapsed in the panel (user choice, carried forward across rebuilds).
    pub collapsed: bool,
}

impl OutlineNode {
    /// True when this node has child headings (so it renders a collapse caret).
    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }
}

/// A flat heading record collected from the DocModel before the stack-fold builds the tree.
struct FlatHeading {
    block_id: String,
    level: u8,
    text: String,
    char_offset: usize,
}

/// Build the heading outline tree from the document (`doc` root). PURE — no egui, no IO — so it is
/// directly unit-testable (AC-001 / AC-002).
///
/// Walks the top-level blocks in document order; for each `Heading` at level 1..=3 it records the
/// heading's `block_id` (the `re-block-{hash}` of its top-level path), level, plain text, and first-text
/// absolute char offset. Headings deeper than level 3 and every non-heading block are excluded (AC-002).
/// The flat ordered list is then folded into a nested tree by a heading-stack algorithm (MT impl note):
/// a heading of level N becomes a child of the most recent heading of level < N still on the stack;
/// headings that appear before any shallower heading attach at the root.
///
/// The MT scope walks "the DocModel block list" — the editor's blocks are the top-level children of the
/// `doc` root (the same units the editor renders + addresses as `re-block-{hash}`), so the walk is over
/// `doc.children` at the top level, matching the editor's block-addressing exactly. Headings nested
/// inside list items / blockquotes are not top-level editor blocks and are out of this MT's scope (the
/// React `RichDocumentView` TOC likewise lists top-level headings).
pub fn build_outline(doc: &BlockNode) -> Vec<OutlineNode> {
    // 1) Collect the flat ordered list of in-scope headings.
    let mut flat: Vec<FlatHeading> = Vec::new();
    for (idx, child) in doc.children.iter().enumerate() {
        let Some(block) = child.as_block() else {
            continue;
        };
        let Some(level) = block.heading_level() else {
            continue;
        };
        // heading_level() is always 1..=3 (HeadingLevel clamps), but guard explicitly so the 1..=3
        // contract is enforced HERE rather than relied upon (AC-002 belt-and-braces).
        if !(1..=3).contains(&level) {
            continue;
        }
        let path = [idx];
        let block_id = block_author_id(&path);
        let text = block_plain_text(block);
        // The heading's first text position is `[idx, 0]` offset 0 (the start of its first child). Its
        // flat absolute CHAR offset in the document is `absolute_offset(doc, that_pos)` — MT-012's
        // coordinate space (RISK-002 / MC-002: ONE coordinate space shared with the caret model).
        let first_pos = DocPosition::new(vec![idx, 0], 0);
        let char_offset = absolute_offset(doc, &first_pos);
        flat.push(FlatHeading {
            block_id,
            level,
            text,
            char_offset,
        });
    }

    // 2) Fold the flat list into a nested tree by the heading-stack algorithm. The stack holds the PATH
    //    (sequence of child indices) into `roots` of each currently-open ancestor heading, newest last.
    let mut roots: Vec<OutlineNode> = Vec::new();
    // Each stack entry is (level, path-into-the-tree-of-the-open-node).
    let mut stack: Vec<(u8, Vec<usize>)> = Vec::new();
    for h in flat {
        let node = OutlineNode {
            block_id: h.block_id,
            level: h.level,
            text: h.text,
            char_offset: h.char_offset,
            children: Vec::new(),
            collapsed: false,
        };
        // Pop every open ancestor whose level is >= this heading's level (a same-or-shallower heading
        // closes them), so the new top is a STRICTLY shallower heading (level < N) or empty (root).
        while stack.last().is_some_and(|(lvl, _)| *lvl >= h.level) {
            stack.pop();
        }
        let new_path = match stack.last() {
            // Attach under the most recent strictly-shallower open heading.
            Some((_, parent_path)) => {
                let parent = node_at_path_mut(&mut roots, parent_path)
                    .expect("stack path must address a live node");
                parent.children.push(node);
                let mut p = parent_path.clone();
                p.push(parent.children.len() - 1);
                p
            }
            // No shallower heading open — attach at the root.
            None => {
                roots.push(node);
                vec![roots.len() - 1]
            }
        };
        stack.push((h.level, new_path));
    }
    roots
}

/// Resolve a path (sequence of child indices) into a mutable node within the `roots` forest. The first
/// index selects a root; each subsequent index descends into `children`. Returns `None` for an
/// out-of-range path (never panics).
fn node_at_path_mut<'a>(
    roots: &'a mut [OutlineNode],
    path: &[usize],
) -> Option<&'a mut OutlineNode> {
    let (first, rest) = path.split_first()?;
    let mut node = roots.get_mut(*first)?;
    for &i in rest {
        node = node.children.get_mut(i)?;
    }
    Some(node)
}

/// The plain text of a block (concatenated text leaves). Headings hold a single text leaf, but this
/// concatenates all leaves so a marked-run-split heading ("**bold** title") reads correctly. Pure.
fn block_plain_text(block: &BlockNode) -> String {
    let mut s = String::new();
    for c in &block.children {
        if let Child::Text(t) = c {
            s.push_str(&t.text.to_string());
        }
    }
    s
}

/// Carry forward user collapse state across a rebuild (RISK-004 / MC-004 / AC-006). For every node in
/// `new_roots` whose `block_id` was collapsed in `old_roots`, set its `collapsed` flag. Recurses into
/// children. A heading that no longer exists simply drops out (its collapse state is forgotten); a new
/// heading defaults to expanded.
fn carry_forward_collapse(
    new_roots: &mut [OutlineNode],
    collapsed_ids: &std::collections::HashSet<String>,
) {
    for node in new_roots.iter_mut() {
        if collapsed_ids.contains(&node.block_id) {
            node.collapsed = true;
        }
        carry_forward_collapse(&mut node.children, collapsed_ids);
    }
}

/// Collect the set of `block_id`s that are currently collapsed in a forest (recursively). Used to seed
/// the carry-forward set before a rebuild.
fn collect_collapsed_ids(roots: &[OutlineNode], out: &mut std::collections::HashSet<String>) {
    for node in roots {
        if node.collapsed {
            out.insert(node.block_id.clone());
        }
        collect_collapsed_ids(&node.children, out);
    }
}

/// The heading Outline / Table-of-Contents navigator widget.
///
/// Owns the built tree + the last DocModel revision it built against. [`Self::show`] rebuilds the tree
/// ONLY when the editor state's revision advanced (RISK-001 / MC-001 / AC-004 — never per frame), then
/// renders the indented collapsible list. A click (or AccessKit `Action::Click`/"Press") on an entry
/// drives the editor's EXISTING scroll-to + select-block path (AC-003 / AC-005).
#[derive(Debug, Default)]
pub struct OutlinePanel {
    /// The built heading tree.
    pub roots: Vec<OutlineNode>,
    /// The DocModel revision the current `roots` were built against. The next `show` rebuilds only if
    /// the live revision differs (revision-gated rebuild — AC-004).
    pub doc_revision: u64,
    /// Set once any tree has been built, so the first `show` always builds (a real document may hash to
    /// the `0` sentinel revision otherwise). Without this a doc whose revision happens to be 0 would
    /// never build on the first frame.
    built_once: bool,
}

impl OutlinePanel {
    /// A fresh, empty outline panel (no tree built yet).
    pub fn new() -> Self {
        Self::default()
    }

    /// Rebuild the heading tree from `state`'s document if the revision advanced (or this is the first
    /// build), carrying forward the user's collapse state by `block_id`. Returns `true` when a rebuild
    /// actually happened (the test for AC-004 asserts this is `false` on an unchanged-revision frame).
    ///
    /// Separated from [`Self::show`] so the revision-gating logic is unit/kittest-provable WITHOUT a
    /// render harness (the rebuild decision is the load-bearing AC-004 behavior).
    pub fn sync(&mut self, state: &RichEditorState) -> bool {
        let live = state.doc_revision();
        if self.built_once && live == self.doc_revision {
            return false; // RISK-001 / MC-001: revision unchanged -> NO rebuild this frame.
        }
        // Preserve the user's current collapse choices keyed by block_id, then rebuild and re-apply them
        // (RISK-004 / MC-004 / AC-006 — a live rebuild must not reset expand/collapse).
        let mut collapsed_ids = std::collections::HashSet::new();
        collect_collapsed_ids(&self.roots, &mut collapsed_ids);
        let mut roots = build_outline(&state.doc);
        carry_forward_collapse(&mut roots, &collapsed_ids);
        self.roots = roots;
        self.doc_revision = live;
        self.built_once = true;
        true
    }

    /// Render the outline into `ui`, gated against the editor's document revision, and drive
    /// scroll-to-heading + select-block on a click/Press.
    ///
    /// `state_arc` is the SHARED editor state the outline navigates. The lock is held only briefly — to
    /// read the revision (the rebuild gate) and the document palette, and to render the entries — and is
    /// DROPPED before driving the editor navigation, because the navigation path
    /// ([`RichEditorWidget::scroll_to_block`] / [`RichEditorWidget::select_block`]) re-locks the SAME Arc;
    /// holding the guard across that call would deadlock. A clicked entry then calls
    /// `scroll_to_block(block_id)` + `select_block(block_id)` — the EXISTING editor navigation path
    /// (RISK-002 / MC-002). The SAME path runs for an out-of-process AccessKit `Action::Click` ("Press")
    /// request targeting an entry node, because egui synthesizes a widget click from that request
    /// (AC-005).
    pub fn show(&mut self, ui: &mut egui::Ui, state_arc: &Arc<Mutex<RichEditorState>>) {
        // Phase 1: render under a short-lived lock, collecting the click/toggle requests. The guard is
        // dropped at the end of this block (before phase 2 drives the editor) to avoid a re-lock deadlock.
        let clicked: Option<String> = {
            let state = state_arc.lock().unwrap_or_else(|e| e.into_inner());
            // Revision-gated rebuild (AC-004).
            self.sync(&state);
            let palette = state.palette();

            let mut clicked: Option<String> = None;
            let mut toggled: Vec<String> = Vec::new();

            // The outline container: a stable-id scope so the Tree node id is fixed across frames, with
            // the `rich-editor-outline` author_id + Role::Tree (AC-005). The container is non-interactive
            // (no Click action) — it is a navigable group, not a control.
            ui.scope_builder(
                egui::UiBuilder::new()
                    .id_salt(("rich-editor-outline-container", OUTLINE_NODE_ID_BASE)),
                |ui| {
                    let container_id = ui.unique_id();
                    ui.ctx().accesskit_node_builder(container_id, |node| {
                        node.set_role(accesskit::Role::Tree);
                        node.set_author_id(OUTLINE_CONTAINER_AUTHOR_ID.to_owned());
                        node.set_label("Document outline".to_owned());
                    });

                    egui::ScrollArea::vertical()
                        .id_salt("rich-editor-outline-scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            if self.roots.is_empty() {
                                // Honest empty state (no headings) — not a fake row.
                                ui.add_space(4.0);
                                ui.colored_label(palette.text_subtle, "No headings");
                                return;
                            }
                            render_nodes(ui, &self.roots, &palette, &mut clicked, &mut toggled);
                        });
                },
            );

            // Apply collapse toggles (carry by block_id) while we still hold the panel's own state.
            for id in toggled {
                toggle_collapsed(&mut self.roots, &id);
            }
            clicked
            // <- the state guard is dropped HERE.
        };

        // Phase 2 (guard released): drive the editor navigation for a clicked entry (AC-003). The
        // stale-id guard lives in the editor (block_path_from_id returns None for a gone block), so a
        // dangling entry click is a silent no-op (RISK-003 / MC-003). A transient widget over the SAME
        // Arc carries the scroll request + selection (the editor consumes the scroll on its next frame).
        if let Some(block_id) = clicked {
            let mut editor = RichEditorWidget::new(Arc::clone(state_arc));
            editor.scroll_to_block(&block_id);
            editor.select_block(&block_id);
            ui.ctx().request_repaint();
        }
    }
}

/// Recursively render the outline forest. Pushes a clicked entry's `block_id` into `clicked` and a
/// toggled (collapse-caret-clicked) entry's `block_id` into `toggled`. Skips the children of a collapsed
/// node (the collapse actually hides the subtree).
fn render_nodes(
    ui: &mut egui::Ui,
    nodes: &[OutlineNode],
    palette: &HsPalette,
    clicked: &mut Option<String>,
    toggled: &mut Vec<String>,
) {
    for node in nodes {
        render_one(ui, node, palette, clicked, toggled);
        if node.has_children() && !node.collapsed {
            render_nodes(ui, &node.children, palette, clicked, toggled);
        }
    }
}

/// Render one outline entry: an indented row with an optional collapse caret + a clickable label. The
/// label is a `Role::TreeItem` AccessKit node `outline.heading.{block_id}` carrying an `Action::Click`
/// (the "Press" invoke verb), so a mouse click OR an out-of-process Press both set `clicked` to this
/// entry's `block_id`. The collapse caret toggles the node's `collapsed` flag (recorded in `toggled`).
fn render_one(
    ui: &mut egui::Ui,
    node: &OutlineNode,
    palette: &HsPalette,
    clicked: &mut Option<String>,
    toggled: &mut Vec<String>,
) {
    let indent = (node.level.saturating_sub(1)) as f32 * INDENT_PX;
    ui.horizontal(|ui| {
        ui.add_space(indent);
        // Collapse caret for nodes with children (a manual triangle toggle). A leaf reserves the same
        // width so labels align.
        if node.has_children() {
            let caret = if node.collapsed {
                "\u{25B6}"
            } else {
                "\u{25BC}"
            }; // ▶ / ▼
            let caret_resp = ui.add(
                egui::Label::new(egui::RichText::new(caret).color(palette.text_subtle))
                    .sense(egui::Sense::click()),
            );
            // The caret carries its own stable author_id so a swarm agent can collapse/expand a subtree.
            crate::accessibility::emit_interactive_node(
                ui.ctx(),
                caret_resp.id,
                &format!("outline.toggle.{}", node.block_id),
            );
            if caret_resp.clicked() {
                toggled.push(node.block_id.clone());
            }
        } else {
            ui.add_space(INDENT_PX); // align leaf labels under their siblings' labels.
        }

        // The clickable heading entry. A selectable_label (the contract's named widget). It is not held
        // "selected" (the outline does not track a current entry — selection lives in the editor), so the
        // selected flag is always false.
        let label = egui::RichText::new(&node.text).color(palette.text);
        let entry = ui.selectable_label(false, label);
        // The stable TreeItem node: author_id `outline.heading.{block_id}`, role TreeItem, label = the
        // heading text, + an Action::Click ("Press") so the scroll+select path is invokable
        // out-of-process (AC-005). selectable_label already gave the node Click+Focus actions and a
        // role; we OVERRIDE the role to TreeItem and attach the stable author_id + add Click explicitly
        // (idempotent) so the contract's Role/Action/author_id are all guaranteed on the live node.
        let entry_id = entry.id;
        let author = outline_entry_author_id(&node.block_id);
        let text = node.text.clone();
        ui.ctx().accesskit_node_builder(entry_id, move |n| {
            n.set_role(accesskit::Role::TreeItem);
            n.set_author_id(author.clone());
            n.set_label(text.clone());
            n.add_action(accesskit::Action::Click);
        });
        // A mouse click OR an out-of-process AccessKit Click/"Press" request both fire the scroll+select.
        // `clicked()` reports both an egui pointer click and an AccessKit action request that egui routed
        // to this widget (egui synthesizes a click from an `Action::Click` request), so this single check
        // covers AC-003 (mouse) AND AC-005 (Press) — the SAME path the contract requires.
        if entry.clicked() {
            *clicked = Some(node.block_id.clone());
        }
    });
}

/// Toggle the `collapsed` flag of the node with `block_id` anywhere in the forest (recursive). No-op if
/// not found.
fn toggle_collapsed(roots: &mut [OutlineNode], block_id: &str) {
    for node in roots.iter_mut() {
        if node.block_id == block_id {
            node.collapsed = !node.collapsed;
            return;
        }
        toggle_collapsed(&mut node.children, block_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rich_editor::document_model::node::{BlockNode, Child, NodeKind, TextLeaf};

    /// doc with headings h1 > h2 > h2 > h3 (and the block_ids the editor would assign).
    fn h1_h2_h2_h3_doc() -> BlockNode {
        BlockNode::doc(vec![
            BlockNode::heading(1, "Alpha"),
            BlockNode::heading(2, "Beta"),
            BlockNode::heading(2, "Gamma"),
            BlockNode::heading(3, "Delta"),
        ])
    }

    #[test]
    fn build_outline_h1_h2_h2_h3() {
        // AC-001: h1 is a root; both h2 are its children in document order; h3 is a child of the 2nd h2.
        let doc = h1_h2_h2_h3_doc();
        let roots = build_outline(&doc);
        assert_eq!(roots.len(), 1, "exactly one root (the h1)");
        let h1 = &roots[0];
        assert_eq!(h1.level, 1);
        assert_eq!(h1.text, "Alpha");
        assert_eq!(
            h1.block_id,
            block_author_id(&[0]),
            "h1 carries its real re-block block_id"
        );
        assert_eq!(h1.children.len(), 2, "both h2 are children of the h1");
        assert_eq!(h1.children[0].text, "Beta");
        assert_eq!(h1.children[1].text, "Gamma");
        assert_eq!(h1.children[0].block_id, block_author_id(&[1]));
        assert_eq!(h1.children[1].block_id, block_author_id(&[2]));
        // The h3 is a child of the SECOND h2 (Gamma), in document order.
        assert_eq!(
            h1.children[1].children.len(),
            1,
            "the h3 nests under the 2nd h2"
        );
        assert_eq!(h1.children[1].children[0].text, "Delta");
        assert_eq!(h1.children[1].children[0].level, 3);
        assert_eq!(h1.children[1].children[0].block_id, block_author_id(&[3]));
        // The first h2 (Beta) has no children.
        assert!(h1.children[0].children.is_empty());
        // Byte/char offsets are the flat absolute CHAR offsets in document order: Alpha at 0, Beta after
        // "Alpha" (5), Gamma after "AlphaBeta" (9), Delta after "AlphaBetaGamma" (14).
        assert_eq!(h1.char_offset, 0);
        assert_eq!(h1.children[0].char_offset, 5);
        assert_eq!(h1.children[1].char_offset, 9);
        assert_eq!(h1.children[1].children[0].char_offset, 14);
    }

    #[test]
    fn build_outline_excludes_non_headings_and_deep_levels() {
        // AC-002: only headings 1..=3 appear; paragraphs/lists/code-blocks are excluded. (Headings >3
        // cannot be REPRESENTED — HeadingLevel clamps to 3 — but a clamped level-3 from a "level 5" attr
        // still legitimately appears as a level-3 heading; the exclusion that matters here is non-heading
        // blocks, which we assert directly.)
        let doc = BlockNode::doc(vec![
            BlockNode::heading(1, "Title"),
            BlockNode::paragraph("a paragraph"),
            BlockNode::with_children(
                NodeKind::CodeBlock,
                vec![Child::Text(TextLeaf::new("let x = 1;"))],
            ),
            BlockNode::with_children(
                NodeKind::BulletList,
                vec![Child::Block(BlockNode::with_children(
                    NodeKind::ListItem,
                    vec![Child::Block(BlockNode::paragraph("item"))],
                ))],
            ),
            BlockNode::heading(2, "Section"),
        ]);
        let roots = build_outline(&doc);
        // Two headings total: the h1 root + the h2 child.
        assert_eq!(roots.len(), 1, "one root heading");
        assert_eq!(roots[0].text, "Title");
        assert_eq!(
            roots[0].children.len(),
            1,
            "the h2 is the only other outline entry"
        );
        assert_eq!(roots[0].children[0].text, "Section");
        // No paragraph/code/list block leaked in as an outline node.
        let total = count_nodes(&roots);
        assert_eq!(
            total, 2,
            "exactly the 2 headings appear; paragraphs/code/lists are excluded"
        );
    }

    #[test]
    fn build_outline_heading_before_shallower_attaches_at_root() {
        // A level-2 heading appearing BEFORE any level-1 attaches at the root (no shallower ancestor).
        let doc = BlockNode::doc(vec![
            BlockNode::heading(2, "Orphan H2"),
            BlockNode::heading(1, "Later H1"),
            BlockNode::heading(2, "Child of H1"),
        ]);
        let roots = build_outline(&doc);
        assert_eq!(
            roots.len(),
            2,
            "the orphan h2 and the later h1 are both roots"
        );
        assert_eq!(roots[0].text, "Orphan H2");
        assert!(roots[0].children.is_empty());
        assert_eq!(roots[1].text, "Later H1");
        assert_eq!(roots[1].children.len(), 1);
        assert_eq!(roots[1].children[0].text, "Child of H1");
    }

    #[test]
    fn build_outline_empty_doc_is_empty() {
        let doc = BlockNode::doc(vec![BlockNode::paragraph("no headings here")]);
        assert!(build_outline(&doc).is_empty());
    }

    #[test]
    fn sync_is_revision_gated() {
        // AC-004 (pure side): sync rebuilds when the revision changed, and does NOT rebuild on an
        // unchanged-revision frame.
        let state = RichEditorState::new(h1_h2_h2_h3_doc());
        let mut panel = OutlinePanel::new();
        assert!(panel.sync(&state), "first sync always builds");
        assert_eq!(count_nodes(&panel.roots), 4);
        // A second sync with no document change MUST NOT rebuild (revision unchanged).
        assert!(
            !panel.sync(&state),
            "no rebuild on an unchanged-revision frame (RISK-001/MC-001)"
        );
    }

    #[test]
    fn sync_rebuilds_when_heading_added_and_carries_collapse() {
        // AC-004 + AC-006: adding a heading bumps the revision -> rebuild includes it; an unrelated
        // collapse choice survives the rebuild.
        let mut state = RichEditorState::new(h1_h2_h2_h3_doc());
        let mut panel = OutlinePanel::new();
        panel.sync(&state);
        assert_eq!(count_nodes(&panel.roots), 4);
        // Collapse the h1 root, then add a NEW heading at the end of the doc.
        let h1_id = block_author_id(&[0]);
        toggle_collapsed(&mut panel.roots, &h1_id);
        assert!(panel.roots[0].collapsed, "h1 collapsed by the user");
        state
            .doc
            .children
            .push(Child::Block(BlockNode::heading(2, "Epsilon")));
        // The revision advanced -> sync rebuilds and includes the new heading.
        assert!(
            panel.sync(&state),
            "adding a heading advances the revision -> rebuild"
        );
        assert_eq!(
            count_nodes(&panel.roots),
            5,
            "the new Epsilon heading appears"
        );
        // AC-006: the user's collapse of the h1 survived the rebuild.
        assert!(
            panel.roots[0].collapsed,
            "AC-006: the collapsed h1 stays collapsed across a live rebuild"
        );
    }

    #[test]
    fn non_heading_edit_does_not_change_revision() {
        // RISK-001: editing a PARAGRAPH (not a heading) leaves the heading-fingerprint revision
        // unchanged, so the outline does not rebuild.
        let mut state = RichEditorState::new(BlockNode::doc(vec![
            BlockNode::heading(1, "Title"),
            BlockNode::paragraph("body"),
        ]));
        let rev_before = state.doc_revision();
        // Mutate the paragraph text.
        if let Some(p) = state.doc.children.get_mut(1).and_then(Child::as_block_mut) {
            if let Some(t) = p.children.get_mut(0).and_then(Child::as_text_mut) {
                t.text = crate::rich_editor::document_model::rope_text::RopeText::from_str(
                    "body edited longer",
                );
            }
        }
        let rev_after = state.doc_revision();
        assert_eq!(
            rev_before, rev_after,
            "a non-heading edit must not change the heading revision"
        );
        // Retitling the HEADING, by contrast, MUST change it.
        if let Some(h) = state.doc.children.get_mut(0).and_then(Child::as_block_mut) {
            if let Some(t) = h.children.get_mut(0).and_then(Child::as_text_mut) {
                t.text =
                    crate::rich_editor::document_model::rope_text::RopeText::from_str("New Title");
            }
        }
        assert_ne!(
            rev_after,
            state.doc_revision(),
            "retitling a heading must change the revision"
        );
    }

    #[test]
    fn entry_author_id_namespace_is_distinct() {
        // RISK-005 / MC-005: the outline entry id is `outline.heading.{block_id}`, distinct from the
        // editor's `re-block-{hash}` block node ids AND from the container id.
        let bid = block_author_id(&[0]);
        let entry = outline_entry_author_id(&bid);
        assert!(entry.starts_with("outline.heading."));
        assert_ne!(
            entry, bid,
            "the entry id is namespaced, not the bare block id"
        );
        assert_ne!(entry, OUTLINE_CONTAINER_AUTHOR_ID);
        assert!(
            !entry.starts_with("re-block-"),
            "must not collide with the editor block-node namespace"
        );
    }

    /// Count every node in an outline forest (recursive) for the exclusion/total assertions.
    fn count_nodes(roots: &[OutlineNode]) -> usize {
        roots.iter().map(|n| 1 + count_nodes(&n.children)).sum()
    }
}
