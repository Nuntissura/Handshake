//! The static slash-command catalog + the filter/ranking algorithm (WP-KERNEL-012 MT-016).
//!
//! This is the DATA layer of the slash-command surface: a `&'static` slice of
//! [`SlashCommand`]s covering every insertable block type, embed kind, wikilink action,
//! template, and the advanced manual-insert command (Obsidian / Notion parity). The menu
//! widget ([`super::menu`]) renders the filtered catalog and the executor
//! ([`super::executor`]) dispatches the selected command's [`SlashAction`] to the right
//! MT-013/MT-014/MT-015 handler.
//!
//! ## Why `&'static` everything (MT impl note 1)
//!
//! Every string field is `&'static str` and the catalog itself is a top-level `const`
//! slice, so iterating + filtering it allocates NOTHING per frame (the menu re-filters on
//! every keystroke). The one place a heap value is produced is [`SlashAction::InsertNode`]
//! / [`SlashAction::InsertTemplate`], where a builder fn synthesizes the node(s) ONLY when
//! the command actually executes — never on the hot filter path.
//!
//! ## Catalog source-of-truth (parity anchor)
//!
//! The command set is derived from the React `EDITOR_COMMANDS` categories
//! (`app/src/lib/editor/editor_commands.ts`: `block | embed | link | template`) so the
//! native slash menu offers the same insertable surface the Tiptap slash extension did.
//! Each block command reuses the MT-013 [`crate::rich_editor::formatting::commands`]
//! command set; each embed/wikilink command reuses the MT-014/MT-015 inline-atom logic.
//!
//! ## Catalog growth guard (red-team RISK-2 / MC-002)
//!
//! The filter is O(n) per keystroke. That is fine while `n` stays small, so a compile-time
//! [`SLASH_COMMANDS_MAX_COUNT`] assertion (a `const { assert!(...) }`, the same guard the
//! renderer uses for its node-id bands — no new crate dependency) FAILS THE BUILD if the
//! catalog grows past 100 entries without a deliberate review of the filter cost. This is
//! the field-correct, dependency-free equivalent of the contract's `static_assertions`
//! suggestion.

use crate::rich_editor::document_model::node::{BlockNode, Child, NodeKind, TextLeaf};

/// The grouping category a slash command renders under in the menu (the menu lists items
/// grouped by category, matching the React slash menu's section headers).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlashCategory {
    /// Block-level structures (paragraph, headings, lists, quote, code, table, rule).
    Blocks,
    /// CKC media embeds (image / slideshow / album / video).
    Embeds,
    /// Wikilinks + transclusion (MT-015 inline atoms).
    Wikilinks,
    /// Predefined document templates (hardcoded DocJson snippets in this MT).
    Templates,
    /// Advanced manual-insert (raw node JSON — for swarm-agent use).
    Manual,
}

impl SlashCategory {
    /// The human-readable section header the menu renders above this category's items.
    pub fn header(self) -> &'static str {
        match self {
            SlashCategory::Blocks => "Blocks",
            SlashCategory::Embeds => "Embeds",
            SlashCategory::Wikilinks => "Links",
            SlashCategory::Templates => "Templates",
            SlashCategory::Manual => "Advanced",
        }
    }

    /// A stable display order index (Blocks first, Advanced last) so the grouped menu
    /// renders categories in a deterministic top-to-bottom order regardless of the order
    /// items appear in [`SLASH_COMMANDS`].
    pub fn order(self) -> u8 {
        match self {
            SlashCategory::Blocks => 0,
            SlashCategory::Embeds => 1,
            SlashCategory::Wikilinks => 2,
            SlashCategory::Templates => 3,
            SlashCategory::Manual => 4,
        }
    }
}

/// The CKC media-embed kind a [`SlashAction::OpenEmbedPrompt`] command targets. Maps 1:1
/// onto the MT-014 [`crate::rich_editor::embeds::asset_resolver::MEDIA_EMBED_REF_KINDS`]
/// backend `ref_kind` strings (`images`/`slideshow`/`album`/`video`), so an inserted embed
/// is the same `hsLink` atom MT-014 renders — no new node kind invented here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbedKind {
    /// A single image embed (`ref_kind = "images"`).
    Image,
    /// A slideshow embed (`ref_kind = "slideshow"`).
    Slideshow,
    /// An album embed (`ref_kind = "album"`).
    Album,
    /// A video embed (`ref_kind = "video"`).
    Video,
}

impl EmbedKind {
    /// The backend `ref_kind` string the inserted `hsLink` atom carries (the exact value
    /// MT-014's `MediaEmbedKind::from_ref_kind` recognizes). NOTE the image kind is the
    /// plural `"images"` — the backend's image embed ref kind (verified against
    /// `asset_resolver::MEDIA_EMBED_REF_KINDS`).
    pub fn ref_kind(self) -> &'static str {
        match self {
            EmbedKind::Image => "images",
            EmbedKind::Slideshow => "slideshow",
            EmbedKind::Album => "album",
            EmbedKind::Video => "video",
        }
    }
}

/// A predefined document template id (MT impl note 5: hardcoded DocJson snippets in this
/// MT; a future MT makes them user-editable). The two required templates are the daily
/// note and the meeting notes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateId {
    /// "Daily Note Template" — a heading + 3 bullet items.
    DailyNote,
    /// "Meeting Notes Template" — a heading + agenda list + action-items table.
    MeetingNotes,
}

impl TemplateId {
    /// The const DocJson snippet (a BARE `{type:"doc",content:[...]}` node string) this
    /// template inserts. The executor parses it with
    /// [`crate::rich_editor::document_model::doc_json::from_json_string`] and inserts the
    /// resulting doc's block children at the caret. The snippets are written in the exact
    /// Tiptap `content_json` shape the backend round-trips (heading `attrs.level`, taskItem
    /// `attrs.checked`, table cells with `colspan`/`rowspan`), so a template-built document
    /// saves and reloads unchanged.
    pub fn doc_json(self) -> &'static str {
        match self {
            TemplateId::DailyNote => DAILY_NOTE_TEMPLATE_JSON,
            TemplateId::MeetingNotes => MEETING_NOTES_TEMPLATE_JSON,
        }
    }
}

/// "Daily Note Template": an H1 heading followed by a 3-item bullet list. Bare doc node.
const DAILY_NOTE_TEMPLATE_JSON: &str = r#"{
  "type": "doc",
  "content": [
    { "type": "heading", "attrs": { "level": 1 }, "content": [ { "type": "text", "text": "Daily Note" } ] },
    { "type": "bulletList", "content": [
      { "type": "listItem", "content": [ { "type": "paragraph", "content": [ { "type": "text", "text": "Priorities" } ] } ] },
      { "type": "listItem", "content": [ { "type": "paragraph", "content": [ { "type": "text", "text": "Notes" } ] } ] },
      { "type": "listItem", "content": [ { "type": "paragraph", "content": [ { "type": "text", "text": "Reflections" } ] } ] }
    ] }
  ]
}"#;

/// "Meeting Notes Template": an H1 heading, an "Agenda" bullet list, and a 2x2 action-items
/// table (a header row + one body row). Bare doc node, exact backend shape.
const MEETING_NOTES_TEMPLATE_JSON: &str = r#"{
  "type": "doc",
  "content": [
    { "type": "heading", "attrs": { "level": 1 }, "content": [ { "type": "text", "text": "Meeting Notes" } ] },
    { "type": "bulletList", "content": [
      { "type": "listItem", "content": [ { "type": "paragraph", "content": [ { "type": "text", "text": "Agenda item" } ] } ] }
    ] },
    { "type": "table", "content": [
      { "type": "tableRow", "content": [
        { "type": "tableCell", "attrs": { "colspan": 1, "rowspan": 1, "isHeader": true }, "content": [ { "type": "paragraph", "content": [ { "type": "text", "text": "Action" } ] } ] },
        { "type": "tableCell", "attrs": { "colspan": 1, "rowspan": 1, "isHeader": true }, "content": [ { "type": "paragraph", "content": [ { "type": "text", "text": "Owner" } ] } ] }
      ] },
      { "type": "tableRow", "content": [
        { "type": "tableCell", "attrs": { "colspan": 1, "rowspan": 1 }, "content": [ { "type": "paragraph", "content": [ { "type": "text", "text": "" } ] } ] },
        { "type": "tableCell", "attrs": { "colspan": 1, "rowspan": 1 }, "content": [ { "type": "paragraph", "content": [ { "type": "text", "text": "" } ] } ] }
      ] }
    ] }
  ]
}"#;

/// What a selected slash command DOES when executed. Each variant maps to an existing MT
/// handler so the slash menu never re-implements block/embed/wikilink logic (it is purely a
/// keyboard-driven dispatcher onto the proven command surface).
#[derive(Clone, Copy)]
pub enum SlashAction {
    /// Convert the caret's block to a paragraph via MT-013
    /// `formatting::commands::set_paragraph`. Idempotent SET.
    SetParagraph,
    /// Convert the caret's block to a heading at this level (1..=3) via MT-013
    /// `formatting::commands::set_heading`. The raw `u8` (not a `HeadingLevel`) keeps the
    /// catalog `const`-constructible — `HeadingLevel::new` is non-const, and the MT-011 node
    /// model is a frozen dependency this MT may not edit; the executor clamps the level when
    /// it dispatches `FormattingCommand::SetHeading(level)`. Idempotent SET.
    SetHeading(u8),
    /// Build a fresh block node (via the `fn`) and insert it after the caret's block via an
    /// MT-011 `InsertNode` transaction (transactional/undoable — applies to BLOCK nodes).
    /// Used for the structures `set_block_kind` cannot express as a conversion (lists,
    /// task list, blockquote, code block, table, horizontal rule).
    InsertNode(fn() -> BlockNode),
    /// Open the embed-prompt modal for `EmbedKind`; on confirm, insert the embed `hsLink`
    /// atom reusing MT-014 logic.
    OpenEmbedPrompt(EmbedKind),
    /// Activate the MT-015 wikilink autocomplete (`[[…]]`) at the caret.
    OpenWikilinkAutocomplete,
    /// Insert a transclusion atom (the MT-015 `loomTransclusion`) at the caret (advanced;
    /// the operator supplies the ref via the embed-prompt modal reused for transclusions).
    OpenTransclusionPrompt,
    /// Insert a predefined template's DocJson blocks after the caret's block.
    InsertTemplate(TemplateId),
    /// Advanced: open the manual raw-node-JSON insert modal (swarm-agent surface). The
    /// operator/agent types a bare node JSON; the executor parses + inserts it.
    OpenManualInsertPrompt,
}

/// A single slash-menu command. All string fields are `&'static str` so the catalog is a
/// zero-allocation `const` (MT impl note 1).
#[derive(Clone, Copy)]
pub struct SlashCommand {
    /// Stable id (kebab/snake), used to build the AccessKit author_id `slash-item-{id}` and
    /// to match the React catalog ids. Unique across the catalog (proven by a unit test).
    pub id: &'static str,
    /// The menu row's primary label (e.g. "Heading 1").
    pub label: &'static str,
    /// The menu row's secondary description (smaller text).
    pub description: &'static str,
    /// Extra match keywords (case-insensitive) so e.g. "h1" matches "Heading 1" and "todo"
    /// matches the task list.
    pub keywords: &'static [&'static str],
    /// The grouping category.
    pub category: SlashCategory,
    /// What executing the command does.
    pub action: SlashAction,
    /// A short text glyph placeholder rendered at the row's left (no icon font dependency —
    /// a single unicode/ascii glyph the theme renders in the row text color).
    pub glyph: &'static str,
}

impl SlashCommand {
    /// True when this command matches `filter_lower` (already lowercased) as a SUBSTRING of
    /// its label OR any keyword. The empty filter matches everything.
    pub fn matches(&self, filter_lower: &str) -> bool {
        if filter_lower.is_empty() {
            return true;
        }
        if self.label.to_lowercase().contains(filter_lower) {
            return true;
        }
        self.keywords
            .iter()
            .any(|k| k.to_lowercase().contains(filter_lower))
    }

    /// True when this command's LABEL starts with `filter_lower` (the exact-prefix rank-1
    /// signal). Used by [`filter_slash_commands`] to float prefix matches above substring
    /// matches (MT impl note 2: 2-pass filter).
    pub fn label_prefix_matches(&self, filter_lower: &str) -> bool {
        !filter_lower.is_empty() && self.label.to_lowercase().starts_with(filter_lower)
    }
}

/// The maximum number of slash commands rendered in the menu at once (MT scope: "Max 20
/// items shown; scrollable if more match"). The menu caps the visible list at this; the
/// scroll area shows the rest.
pub const SLASH_MENU_MAX_VISIBLE: usize = 20;

/// The hard upper bound on the catalog size (red-team RISK-2 / MC-002). A compile-time
/// assertion below fails the build if [`SLASH_COMMANDS`] grows past this without a review of
/// the O(n)-per-keystroke filter cost.
pub const SLASH_COMMANDS_MAX_COUNT: usize = 100;

/// Build a fresh bullet list holding one empty paragraph list item (the slash "Bullet
/// List" insert). The MT-013 toggle converts an EXISTING paragraph; the slash menu inserts
/// a NEW empty list after the caret, so it builds the node directly.
fn new_bullet_list() -> BlockNode {
    list_with_one_item(NodeKind::BulletList, false)
}

/// Build a fresh ordered list with one empty item.
fn new_ordered_list() -> BlockNode {
    list_with_one_item(NodeKind::OrderedList, false)
}

/// Build a fresh task list (a bullet list whose single item is a `taskItem` with
/// `attrs.checked = false`, matching the MT-013 task-list representation).
fn new_task_list() -> BlockNode {
    list_with_one_item(NodeKind::BulletList, true)
}

/// A list of `kind` containing exactly one item; the item is a `taskItem` (with
/// `checked: false`) when `task` is set, else a `listItem`. The item wraps one empty
/// paragraph so the caret has an inline-content leaf to land in.
fn list_with_one_item(kind: NodeKind, task: bool) -> BlockNode {
    let item_kind = if task { NodeKind::TaskItem } else { NodeKind::ListItem };
    let mut item = BlockNode::with_children(item_kind, vec![Child::Block(BlockNode::paragraph(""))]);
    if task {
        item.attrs
            .insert("checked".to_string(), serde_json::Value::Bool(false));
    }
    BlockNode::with_children(kind, vec![Child::Block(item)])
}

/// Build a fresh blockquote wrapping one empty paragraph.
fn new_blockquote() -> BlockNode {
    BlockNode::with_children(NodeKind::Blockquote, vec![Child::Block(BlockNode::paragraph(""))])
}

/// Build a fresh code block holding one empty unmarked text leaf.
fn new_code_block() -> BlockNode {
    BlockNode::with_children(NodeKind::CodeBlock, vec![Child::Text(TextLeaf::new(""))])
}

/// Build a fresh horizontal-rule atom block.
fn new_horizontal_rule() -> BlockNode {
    BlockNode::new(NodeKind::HorizontalRule)
}

/// Build a fresh 3x3 table with a header first row (matching the MT-013 `insert_table`
/// shape: header row cells carry `attrs.isHeader = true`, every cell carries
/// `colspan`/`rowspan = 1` and one empty paragraph).
fn new_table() -> BlockNode {
    let cell = |is_header: bool| {
        let mut c = BlockNode::with_children(
            NodeKind::TableCell,
            vec![Child::Block(BlockNode::paragraph(""))],
        );
        c.attrs.insert("colspan".to_string(), serde_json::Value::from(1));
        c.attrs.insert("rowspan".to_string(), serde_json::Value::from(1));
        if is_header {
            c.attrs
                .insert("isHeader".to_string(), serde_json::Value::Bool(true));
        }
        c
    };
    let mut rows = Vec::with_capacity(3);
    for r in 0..3 {
        let is_header = r == 0;
        let mut row = BlockNode::new(NodeKind::TableRow);
        for _ in 0..3 {
            row.children.push(Child::Block(cell(is_header)));
        }
        rows.push(Child::Block(row));
    }
    BlockNode::with_children(NodeKind::Table, rows)
}

/// The full static slash-command catalog (parity with the React `EDITOR_COMMANDS` block /
/// embed / link / template categories). One `const` slice; no per-frame allocation.
pub const SLASH_COMMANDS: &[SlashCommand] = &[
    // ── Blocks ──────────────────────────────────────────────────────────────────────────
    SlashCommand {
        id: "paragraph",
        label: "Paragraph",
        description: "Plain body text",
        keywords: &["text", "body", "p"],
        category: SlashCategory::Blocks,
        action: SlashAction::SetParagraph,
        glyph: "¶",
    },
    SlashCommand {
        id: "heading-1",
        label: "Heading 1",
        description: "Large section heading",
        keywords: &["h1", "title", "head"],
        category: SlashCategory::Blocks,
        action: SlashAction::SetHeading(1),
        glyph: "H1",
    },
    SlashCommand {
        id: "heading-2",
        label: "Heading 2",
        description: "Medium section heading",
        keywords: &["h2", "subtitle", "head"],
        category: SlashCategory::Blocks,
        action: SlashAction::SetHeading(2),
        glyph: "H2",
    },
    SlashCommand {
        id: "heading-3",
        label: "Heading 3",
        description: "Small section heading",
        keywords: &["h3", "head"],
        category: SlashCategory::Blocks,
        action: SlashAction::SetHeading(3),
        glyph: "H3",
    },
    SlashCommand {
        id: "bullet-list",
        label: "Bullet List",
        description: "Unordered list",
        keywords: &["ul", "unordered", "bullets", "list"],
        category: SlashCategory::Blocks,
        action: SlashAction::InsertNode(new_bullet_list),
        glyph: "•",
    },
    SlashCommand {
        id: "ordered-list",
        label: "Ordered List",
        description: "Numbered list",
        keywords: &["ol", "numbered", "list"],
        category: SlashCategory::Blocks,
        action: SlashAction::InsertNode(new_ordered_list),
        glyph: "1.",
    },
    SlashCommand {
        id: "task-list",
        label: "Task List",
        description: "Checkbox to-do list",
        keywords: &["todo", "checkbox", "checklist", "tasks"],
        category: SlashCategory::Blocks,
        action: SlashAction::InsertNode(new_task_list),
        glyph: "☐",
    },
    SlashCommand {
        id: "blockquote",
        label: "Blockquote",
        description: "Quoted passage",
        keywords: &["quote", "citation"],
        category: SlashCategory::Blocks,
        action: SlashAction::InsertNode(new_blockquote),
        glyph: "❝",
    },
    SlashCommand {
        id: "code-block",
        label: "Code Block",
        description: "Fenced monospace code (prompts for language)",
        keywords: &["code", "fence", "pre", "monospace"],
        category: SlashCategory::Blocks,
        action: SlashAction::InsertNode(new_code_block),
        glyph: "</>",
    },
    SlashCommand {
        id: "table",
        label: "Table",
        description: "Insert a 3x3 table",
        keywords: &["grid", "rows", "cols", "cells"],
        category: SlashCategory::Blocks,
        action: SlashAction::InsertNode(new_table),
        glyph: "▦",
    },
    SlashCommand {
        id: "horizontal-rule",
        label: "Horizontal Rule",
        description: "A horizontal divider line",
        keywords: &["hr", "divider", "separator", "line"],
        category: SlashCategory::Blocks,
        action: SlashAction::InsertNode(new_horizontal_rule),
        glyph: "―",
    },
    // ── Embeds ──────────────────────────────────────────────────────────────────────────
    SlashCommand {
        id: "embed-image",
        label: "Image embed",
        description: "Embed a CKC image asset (prompts for asset id)",
        keywords: &["picture", "photo", "img", "media"],
        category: SlashCategory::Embeds,
        action: SlashAction::OpenEmbedPrompt(EmbedKind::Image),
        glyph: "🖼",
    },
    SlashCommand {
        id: "embed-slideshow",
        label: "Slideshow embed",
        description: "Embed a CKC slideshow (prompts for asset ids)",
        keywords: &["slides", "carousel", "media"],
        category: SlashCategory::Embeds,
        action: SlashAction::OpenEmbedPrompt(EmbedKind::Slideshow),
        glyph: "▷",
    },
    SlashCommand {
        id: "embed-album",
        label: "Album embed",
        description: "Embed a CKC album grid (prompts for asset ids)",
        keywords: &["gallery", "grid", "media"],
        category: SlashCategory::Embeds,
        action: SlashAction::OpenEmbedPrompt(EmbedKind::Album),
        glyph: "▦",
    },
    SlashCommand {
        id: "embed-video",
        label: "Video embed",
        description: "Embed a CKC video asset (prompts for asset id)",
        keywords: &["movie", "clip", "media"],
        category: SlashCategory::Embeds,
        action: SlashAction::OpenEmbedPrompt(EmbedKind::Video),
        glyph: "▶",
    },
    // ── Wikilinks ─────────────────────────────────────────────────────────────────────────
    SlashCommand {
        id: "insert-link",
        label: "Insert Link",
        description: "Insert a [[wikilink]] with autocomplete",
        keywords: &["wikilink", "[[", "reference", "ref"],
        category: SlashCategory::Wikilinks,
        action: SlashAction::OpenWikilinkAutocomplete,
        glyph: "🔗",
    },
    SlashCommand {
        id: "insert-transclusion",
        label: "Insert Transclusion",
        description: "Embed another note's content by reference",
        keywords: &["embed-note", "include", "transclude"],
        category: SlashCategory::Wikilinks,
        action: SlashAction::OpenTransclusionPrompt,
        glyph: "⟢",
    },
    // ── Templates ─────────────────────────────────────────────────────────────────────────
    SlashCommand {
        id: "template-daily-note",
        label: "Daily Note Template",
        description: "Heading + priorities/notes/reflections",
        keywords: &["journal", "daily", "template"],
        category: SlashCategory::Templates,
        action: SlashAction::InsertTemplate(TemplateId::DailyNote),
        glyph: "📅",
    },
    SlashCommand {
        id: "template-meeting-notes",
        label: "Meeting Notes Template",
        description: "Heading + agenda + action items table",
        keywords: &["meeting", "agenda", "minutes", "template"],
        category: SlashCategory::Templates,
        action: SlashAction::InsertTemplate(TemplateId::MeetingNotes),
        glyph: "🗒",
    },
    // ── Advanced ────────────────────────────────────────────────────────────────────────
    SlashCommand {
        id: "manual-insert",
        label: "Manual node insert",
        description: "Advanced: paste raw node JSON (swarm agents)",
        keywords: &["raw", "json", "advanced", "agent"],
        category: SlashCategory::Manual,
        action: SlashAction::OpenManualInsertPrompt,
        glyph: "{}",
    },
];

// Catalog growth guard (red-team RISK-2 / MC-002): a compile-time assertion (no
// `static_assertions` crate needed — the same `const { assert!(...) }` pattern the renderer
// uses for its node-id bands). A catalog past 100 entries fails the build until the
// O(n)-per-keystroke filter cost is reviewed.
const _: () = assert!(
    SLASH_COMMANDS.len() <= SLASH_COMMANDS_MAX_COUNT,
    "SLASH_COMMANDS exceeded SLASH_COMMANDS_MAX_COUNT (100) — review the O(n)-per-keystroke filter cost before growing the catalog"
);

/// Filter + rank the catalog for `filter` (the text the operator typed after `/`). The
/// 2-pass algorithm (MT impl note 2):
///
/// - PASS 1: every command whose LABEL starts with the (lowercased) filter, in catalog order
///   — the exact-prefix matches rank FIRST.
/// - PASS 2: every REMAINING command that matches as a substring of its label OR any keyword,
///   in catalog order.
///
/// The two passes are concatenated and de-duplicated by command id (a prefix match never
/// reappears in the substring pass). An empty filter returns the whole catalog in catalog
/// order (every command matches; the prefix pass is empty, so pass 2 yields all).
pub fn filter_slash_commands(filter: &str) -> Vec<&'static SlashCommand> {
    let filter_lower = filter.to_lowercase();
    let mut out: Vec<&'static SlashCommand> = Vec::with_capacity(SLASH_COMMANDS.len());

    // PASS 1: exact label-prefix matches first (skipped entirely for an empty filter).
    if !filter_lower.is_empty() {
        for cmd in SLASH_COMMANDS {
            if cmd.label_prefix_matches(&filter_lower) {
                out.push(cmd);
            }
        }
    }
    // PASS 2: substring/keyword matches not already added by the prefix pass.
    for cmd in SLASH_COMMANDS {
        if cmd.matches(&filter_lower) && !out.iter().any(|c| c.id == cmd.id) {
            out.push(cmd);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn command_ids_are_unique() {
        // The AccessKit author_id `slash-item-{id}` must be collision-free, so every id is
        // distinct across the catalog (the HBR-SWARM duplicate-author_id gate forbids dupes).
        let mut ids = HashSet::new();
        for cmd in SLASH_COMMANDS {
            assert!(ids.insert(cmd.id), "duplicate slash command id '{}'", cmd.id);
        }
        assert_eq!(ids.len(), SLASH_COMMANDS.len());
    }

    #[test]
    fn catalog_covers_every_required_block_kind() {
        // The MT scope's BLOCKS list must all be present.
        let ids: HashSet<&str> = SLASH_COMMANDS.iter().map(|c| c.id).collect();
        for required in [
            "paragraph", "heading-1", "heading-2", "heading-3", "bullet-list",
            "ordered-list", "task-list", "blockquote", "code-block", "table",
            "horizontal-rule", "embed-image", "embed-slideshow", "embed-album",
            "embed-video", "insert-link", "insert-transclusion", "template-daily-note",
            "template-meeting-notes", "manual-insert",
        ] {
            assert!(ids.contains(required), "catalog missing required command '{required}'");
        }
    }

    #[test]
    fn filter_head_returns_only_headings() {
        // AC-3: filtering by "head" shows only the Heading 1/2/3 entries. (The keyword "head"
        // is on the three heading commands; no other label/keyword contains "head".)
        let rows = filter_slash_commands("head");
        let ids: Vec<&str> = rows.iter().map(|c| c.id).collect();
        assert_eq!(ids, vec!["heading-1", "heading-2", "heading-3"], "got {ids:?}");
    }

    #[test]
    fn filter_is_case_insensitive() {
        let rows = filter_slash_commands("HEAD");
        let ids: Vec<&str> = rows.iter().map(|c| c.id).collect();
        assert_eq!(ids, vec!["heading-1", "heading-2", "heading-3"]);
    }

    #[test]
    fn filter_prefix_ranks_before_substring() {
        // "table" — the "Table" command label is an exact prefix match; nothing else has
        // "table" as a substring of its label, so it is the single result and ranks first.
        let rows = filter_slash_commands("table");
        assert!(!rows.is_empty());
        assert_eq!(rows[0].id, "table", "exact-prefix label match ranks first");
    }

    #[test]
    fn filter_keyword_matches_h1() {
        // The keyword "h1" matches "Heading 1" even though the label does not contain "h1".
        let rows = filter_slash_commands("h1");
        assert!(rows.iter().any(|c| c.id == "heading-1"), "h1 keyword must match Heading 1");
    }

    #[test]
    fn filter_todo_matches_task_list() {
        let rows = filter_slash_commands("todo");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "task-list");
    }

    #[test]
    fn empty_filter_returns_whole_catalog_in_order() {
        let rows = filter_slash_commands("");
        assert_eq!(rows.len(), SLASH_COMMANDS.len());
        // Catalog order preserved (paragraph first).
        assert_eq!(rows[0].id, "paragraph");
    }

    #[test]
    fn no_match_returns_empty() {
        let rows = filter_slash_commands("zzzznotacommand");
        assert!(rows.is_empty());
    }

    #[test]
    fn embed_kind_ref_kinds_match_backend() {
        // The embed ref_kinds must be the exact MT-014 backend strings.
        use crate::rich_editor::embeds::asset_resolver::MEDIA_EMBED_REF_KINDS;
        for ek in [EmbedKind::Image, EmbedKind::Slideshow, EmbedKind::Album, EmbedKind::Video] {
            assert!(
                MEDIA_EMBED_REF_KINDS.contains(&ek.ref_kind()),
                "embed ref_kind '{}' not in MEDIA_EMBED_REF_KINDS",
                ek.ref_kind()
            );
        }
    }

    #[test]
    fn templates_parse_to_non_empty_docs() {
        // MT impl note 5 + AC-8: each template's DocJson parses to a doc with >= 1 block.
        use crate::rich_editor::document_model::doc_json::from_json_string;
        for tid in [TemplateId::DailyNote, TemplateId::MeetingNotes] {
            let doc = from_json_string(tid.doc_json())
                .unwrap_or_else(|e| panic!("template {tid:?} JSON must parse: {e:?}"));
            assert_eq!(doc.kind, NodeKind::Doc);
            assert!(!doc.children.is_empty(), "template {tid:?} must have >= 1 block");
        }
    }
}
