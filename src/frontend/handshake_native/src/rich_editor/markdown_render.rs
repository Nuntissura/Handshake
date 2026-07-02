//! Shared CommonMark markdown -> displayable-blocks adapter (WP-KERNEL-012 MT-059).
//!
//! ## What this is and why it is shared
//!
//! This module is the SINGLE markdown-string-to-rendered-blocks rendering path the project reuses for
//! every surface that displays an already-rendered markdown STRING (as opposed to the live editable
//! [`crate::rich_editor::document_model`] tree, which the MT-012 WYSIWYG renderer owns). Its first
//! consumer is the read-only Loom wiki-page projection ([`crate::graph::wiki_page_panel`]): MT-025
//! shipped that read-only view as a single raw [`egui::Label`] that printed the
//! `LoomWikiProjection.rendered_content` string verbatim — so a page whose content is `"# Title\n- item"`
//! showed the literal `#` and `-` characters instead of a formatted heading and bullet. This MT resolves
//! that explicit MT-025 deferral by parsing `rendered_content` as CommonMark and painting headings,
//! lists, tables, blockquotes, fenced code blocks, and links with the SAME visual rules the MT-012 block
//! renderer uses.
//!
//! ## Two layers
//!
//! 1. **Parse** ([`parse_markdown`]): runs `pulldown-cmark` (GFM tables + strikethrough enabled) over the
//!    source string and folds its FLAT event stream into a typed [`MdBlock`]/[`MdSpan`] tree. pulldown's
//!    events are a flat `Start(Tag)`/`End(TagEnd)`/inline stream, NOT a tree, so this keeps an explicit
//!    CONTAINER STACK (list items, quotes, table cells) and pops/attaches on each `End` (RISK-2 / MC-2 —
//!    correct nesting of e.g. a list-in-quote or a table).
//! 2. **Render** ([`render_blocks`]): walks the `[MdBlock]` and paints each block into an [`egui::Ui`],
//!    REUSING the MT-012 styling source of truth: the block-level look constants
//!    (`HEADING_SCALE` / `BLOCK_GAP_PTS` / `BLOCKQUOTE_BAR_WIDTH_PTS` / `BLOCKQUOTE_INDENT_PTS` /
//!    `CODE_PADDING_PTS` / `LIST_INDENT_PTS` / `BASE_FONT_SIZE`) come straight from
//!    [`crate::rich_editor::renderer::line_layout`], and the per-span inline styling comes from the
//!    [`crate::rich_editor::renderer::block_renderer::md_span_text_format`] shim (which itself delegates to
//!    the editor's `text_format_for_run`). NOTHING here re-declares a heading scale, quote-bar width, code
//!    frame, or table stroke — keeping ONE look for wiki pages, reading mode, and the editor (RISK-1 / MC-1
//!    / AC5).
//!
//! ## Best-effort, panic-free contract (RISK-3 / MC-3 / AC3)
//!
//! Wiki pages are USER-authored, so the source can be malformed: an unterminated code fence, a table with
//! ragged column counts, a heading with no text, pathologically deep nesting. Parse + render are
//! best-effort and MUST NOT panic: heading levels are clamped to `1..=6`, every table row/cell access is
//! bounds-checked, and any inline run left over when the stream ends is folded into a trailing
//! [`MdBlock::Paragraph`] rather than dropped.
//!
//! ## Theme + links
//!
//! Every color comes from the shared [`crate::theme::HsPalette`] (no `Color32` literal). Link spans render
//! via [`egui::Ui::hyperlink_to`] so they are clickable and inherit egui's `Link` AccessKit role
//! automatically (the contract's link rule).

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use crate::rich_editor::renderer::block_renderer::{md_span_text_format, MdSpanStyle};
use crate::rich_editor::renderer::line_layout::{
    bold_family_available, BASE_FONT_SIZE, BLOCKQUOTE_BAR_WIDTH_PTS, BLOCKQUOTE_INDENT_PTS,
    BLOCK_GAP_PTS, CODE_PADDING_PTS, HEADING_SCALE, LIST_INDENT_PTS,
};
use crate::theme::HsPalette;

/// One inline run of text inside a block, carrying the active style flags. Mirrors the editor's inline
/// mark set; `link` holds the destination URL when the run is a link (so [`render_blocks`] can route it
/// through [`egui::Ui::hyperlink_to`]).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MdSpan {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub code: bool,
    pub strikethrough: bool,
    /// `Some(href)` when this run is a link; `None` for plain text.
    pub link: Option<String>,
}

impl MdSpan {
    /// A plain-text span (no marks).
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            ..Default::default()
        }
    }

    /// The [`MdSpanStyle`] flag set for this span (drops the link href, keeps the link FLAG so the styling
    /// shim colors a link label correctly even when it is painted as a non-clickable label).
    fn style(&self) -> MdSpanStyle {
        MdSpanStyle {
            bold: self.bold,
            italic: self.italic,
            code: self.code,
            strikethrough: self.strikethrough,
            link: self.link.is_some(),
        }
    }
}

/// A parsed top-level (or nested) markdown block. A flat-ish sequence: containers (lists, quotes, tables)
/// hold child blocks/spans so nesting folds correctly, but inline-only blocks (paragraph, heading) hold a
/// flat span vector.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MdBlock {
    /// A heading; `level` is clamped to `1..=6` at parse time (RISK-3).
    Heading { level: u8, spans: Vec<MdSpan> },
    /// A paragraph of inline runs.
    Paragraph { spans: Vec<MdSpan> },
    /// A bullet list; each item is its own block sequence (so an item can hold nested lists/quotes).
    BulletList { items: Vec<Vec<MdBlock>> },
    /// An ordered list starting at `start`; each item is its own block sequence.
    OrderedList {
        start: u64,
        items: Vec<Vec<MdBlock>>,
    },
    /// A blockquote wrapping child blocks (paragraphs, nested lists, nested quotes).
    Quote { children: Vec<MdBlock> },
    /// A fenced/indented code block; `lang` is the fence info string when present.
    CodeBlock { lang: Option<String>, code: String },
    /// A GFM table: a header row of cells (each a span vector) + body rows (each a vector of cells).
    Table {
        headers: Vec<Vec<MdSpan>>,
        rows: Vec<Vec<Vec<MdSpan>>>,
    },
    /// A horizontal rule.
    Rule,
}

/// Parse `src` as CommonMark (GFM tables + strikethrough enabled) into a typed [`MdBlock`] sequence.
///
/// Folds pulldown-cmark's FLAT event stream with an explicit container stack so nested lists / quotes /
/// table cells attach to the correct parent (RISK-2 / MC-2). Best-effort and panic-free for arbitrary
/// user input (RISK-3 / MC-3): heading levels are clamped `1..=6`, and any inline run still open when the
/// stream ends is flushed into a trailing [`MdBlock::Paragraph`] rather than dropped (the "trailing
/// remainder" rule). An empty input yields an empty `Vec`.
pub fn parse_markdown(src: &str) -> Vec<MdBlock> {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);

    let mut folder = Folder::new();
    for event in Parser::new_ext(src, opts) {
        folder.on_event(event);
    }
    folder.finish()
}

/// Active inline style flags while folding text events (a small bit-set tracked as counts so nested
/// emphasis — e.g. bold inside a link — is handled by depth, not a single bool).
#[derive(Clone, Copy, Debug, Default)]
struct InlineStyle {
    bold: u32,
    italic: u32,
    strikethrough: u32,
}

/// A frame on the container stack: a block container currently being filled. Inline content accumulates
/// into `spans`; child blocks accumulate into `blocks`; the [`Folder`] folds a frame into a concrete
/// [`MdBlock`] (or list item / table parts) when its matching `End` arrives.
#[derive(Clone, Debug)]
enum Frame {
    Paragraph {
        spans: Vec<MdSpan>,
    },
    Heading {
        level: u8,
        spans: Vec<MdSpan>,
    },
    /// A code block accumulates raw text (NOT spans — code is verbatim).
    CodeBlock {
        lang: Option<String>,
        code: String,
    },
    Quote {
        children: Vec<MdBlock>,
    },
    List {
        ordered: bool,
        start: u64,
        items: Vec<Vec<MdBlock>>,
    },
    /// A single list item collecting its own child blocks.
    Item {
        children: Vec<MdBlock>,
    },
    Table {
        headers: Vec<Vec<MdSpan>>,
        rows: Vec<Vec<Vec<MdSpan>>>,
        in_head: bool,
        current_row: Vec<Vec<MdSpan>>,
    },
    /// A single table cell collecting its inline spans.
    TableCell {
        spans: Vec<MdSpan>,
    },
    /// A link wraps inline content; its spans inherit the href on flush.
    Link {
        href: String,
        spans: Vec<MdSpan>,
    },
}

/// The event-fold state machine that turns the flat pulldown stream into the [`MdBlock`] tree.
struct Folder {
    /// Finished top-level blocks.
    out: Vec<MdBlock>,
    /// The open-container stack (innermost last).
    stack: Vec<Frame>,
    /// Active inline emphasis depth (bold/italic/strike), applied to every text run while open.
    inline: InlineStyle,
}

impl Folder {
    fn new() -> Self {
        Self {
            out: Vec::new(),
            stack: Vec::new(),
            inline: InlineStyle::default(),
        }
    }

    /// Process one pulldown event.
    fn on_event(&mut self, event: Event<'_>) {
        match event {
            Event::Start(tag) => self.on_start(tag),
            Event::End(tag_end) => self.on_end(tag_end),
            Event::Text(t) => self.push_text(&t, false),
            Event::Code(t) => self.push_text(&t, true),
            // Inline math / HTML: render their raw text best-effort as plain text rather than dropping it.
            Event::InlineMath(t) | Event::DisplayMath(t) => self.push_text(&t, true),
            Event::Html(t) | Event::InlineHtml(t) => self.push_text(&t, false),
            // A footnote reference shows its label inline (best-effort; full footnote defs are out of scope).
            Event::FootnoteReference(label) => self.push_text(&format!("[^{label}]"), false),
            Event::SoftBreak => self.push_text(" ", false),
            Event::HardBreak => self.push_text("\n", false),
            Event::Rule => self.push_block(MdBlock::Rule),
            // A task-list marker renders as a checkbox glyph prefix on its list item's first line.
            Event::TaskListMarker(checked) => {
                let glyph = if checked { "\u{2611} " } else { "\u{2610} " };
                self.push_text(glyph, false);
            }
        }
    }

    fn on_start(&mut self, tag: Tag<'_>) {
        match tag {
            Tag::Paragraph => self.stack.push(Frame::Paragraph { spans: Vec::new() }),
            Tag::Heading { level, .. } => self.stack.push(Frame::Heading {
                level: clamp_heading(level),
                spans: Vec::new(),
            }),
            Tag::BlockQuote(_) => self.stack.push(Frame::Quote {
                children: Vec::new(),
            }),
            Tag::CodeBlock(kind) => {
                let lang = match kind {
                    CodeBlockKind::Fenced(info) => {
                        let s = info.trim().to_string();
                        if s.is_empty() {
                            None
                        } else {
                            Some(s)
                        }
                    }
                    CodeBlockKind::Indented => None,
                };
                self.stack.push(Frame::CodeBlock {
                    lang,
                    code: String::new(),
                })
            }
            Tag::List(start) => self.stack.push(Frame::List {
                ordered: start.is_some(),
                start: start.unwrap_or(1),
                items: Vec::new(),
            }),
            Tag::Item => self.stack.push(Frame::Item {
                children: Vec::new(),
            }),
            Tag::Table(_) => self.stack.push(Frame::Table {
                headers: Vec::new(),
                rows: Vec::new(),
                in_head: false,
                current_row: Vec::new(),
            }),
            Tag::TableHead => {
                if let Some(Frame::Table {
                    in_head,
                    current_row,
                    ..
                }) = self.stack.last_mut()
                {
                    *in_head = true;
                    current_row.clear();
                }
            }
            Tag::TableRow => {
                if let Some(Frame::Table { current_row, .. }) = self.stack.last_mut() {
                    current_row.clear();
                }
            }
            Tag::TableCell => self.stack.push(Frame::TableCell { spans: Vec::new() }),
            Tag::Emphasis => self.inline.italic += 1,
            Tag::Strong => self.inline.bold += 1,
            Tag::Strikethrough => self.inline.strikethrough += 1,
            Tag::Link { dest_url, .. } => self.stack.push(Frame::Link {
                href: dest_url.into_string(),
                spans: Vec::new(),
            }),
            // Images: render the alt text (collected as inline text) inline; the destination is dropped
            // (no image fetch in this read-only markdown path). The Start opens nothing; alt text flows as
            // normal Text events into the current frame, which is the best-effort behavior.
            Tag::Image { .. } => { /* alt text flows into the current inline frame */ }
            // HTML blocks / metadata / footnote defs: open a paragraph so their inner text is not dropped.
            Tag::HtmlBlock | Tag::MetadataBlock(_) | Tag::FootnoteDefinition(_) => {
                self.stack.push(Frame::Paragraph { spans: Vec::new() })
            }
        }
    }

    fn on_end(&mut self, tag_end: TagEnd) {
        match tag_end {
            TagEnd::Emphasis => self.inline.italic = self.inline.italic.saturating_sub(1),
            TagEnd::Strong => self.inline.bold = self.inline.bold.saturating_sub(1),
            TagEnd::Strikethrough => {
                self.inline.strikethrough = self.inline.strikethrough.saturating_sub(1)
            }
            TagEnd::Link => self.close_link(),
            // Image end: nothing was pushed on Start, so nothing to pop.
            TagEnd::Image => {}
            // A TableHead/TableRow opens NO stack frame (only TableCell does), so their End must commit the
            // accumulated row state WITHOUT popping the enclosing Table frame.
            TagEnd::TableHead | TagEnd::TableRow => self.on_end_table_row(),
            // The block-container ends (Item / BlockQuote / List). In a TIGHT list (no blank line between
            // items) pulldown emits item text DIRECTLY as `Text` inside the `Item` with NO surrounding
            // Start/End(Paragraph), so `push_text` auto-opens a `Frame::Paragraph` (RISK-2). Before closing
            // the container we must flush that dangling auto-opened inline frame into the container, else
            // the container's End would pop the stray Paragraph instead of the container (mis-nesting every
            // tight list item into one). This makes tight + loose lists fold identically.
            TagEnd::Item | TagEnd::BlockQuote | TagEnd::List(_) => {
                self.auto_close_dangling_inline();
                self.close_frame();
            }
            _ => self.close_frame(),
        }
    }

    /// Flush a dangling auto-opened inline frame (a `Paragraph`/`Heading` opened by loose `Text` inside a
    /// block container in a tight list) into its parent container, so the subsequent container `End` pops
    /// the CONTAINER, not the stray inline frame. Only pops an inline frame; a real block container is left
    /// for its own `End`.
    fn auto_close_dangling_inline(&mut self) {
        if matches!(
            self.stack.last(),
            Some(Frame::Paragraph { .. }) | Some(Frame::Heading { .. })
        ) {
            self.close_frame();
        }
    }

    /// Close the innermost link frame, merging its accumulated spans (tagged with the href) into the
    /// parent inline context.
    fn close_link(&mut self) {
        if let Some(Frame::Link { href, spans }) = pop_if_link(&mut self.stack) {
            for mut span in spans {
                // Preserve any inner emphasis already baked into the span; set the link href.
                span.link = Some(href.clone());
                self.push_span(span);
            }
        }
    }

    /// Close the innermost (non-link) container frame and attach the resulting block(s) to its parent.
    fn close_frame(&mut self) {
        let Some(frame) = self.stack.pop() else {
            return;
        };
        match frame {
            Frame::Paragraph { spans } => self.push_block(MdBlock::Paragraph { spans }),
            Frame::Heading { level, spans } => self.push_block(MdBlock::Heading { level, spans }),
            Frame::CodeBlock { lang, code } => self.push_block(MdBlock::CodeBlock { lang, code }),
            Frame::Quote { children } => self.push_block(MdBlock::Quote { children }),
            Frame::List {
                ordered,
                start,
                items,
            } => {
                let block = if ordered {
                    MdBlock::OrderedList { start, items }
                } else {
                    MdBlock::BulletList { items }
                };
                self.push_block(block);
            }
            Frame::Item { children } => {
                if let Some(Frame::List { items, .. }) = self.stack.last_mut() {
                    items.push(children);
                } else {
                    // An Item with no enclosing List (malformed): keep its children at the parent level
                    // rather than dropping them.
                    for child in children {
                        self.push_block(child);
                    }
                }
            }
            Frame::Table { headers, rows, .. } => self.push_block(MdBlock::Table { headers, rows }),
            Frame::TableCell { spans } => {
                // Attach the finished cell to the enclosing table's head or current row.
                if let Some(Frame::Table {
                    headers,
                    current_row,
                    in_head,
                    ..
                }) = self.stack.last_mut()
                {
                    if *in_head {
                        headers.push(spans);
                    } else {
                        current_row.push(spans);
                    }
                }
            }
            // A link should be closed by close_link; reaching here means an unbalanced stream — flush its
            // spans into the parent so nothing is dropped.
            Frame::Link { spans, .. } => {
                for span in spans {
                    self.push_span(span);
                }
            }
        }
    }

    /// Push a finished block to the current container (an open item/quote) or to the top-level output.
    fn push_block(&mut self, block: MdBlock) {
        match self.stack.last_mut() {
            Some(Frame::Item { children }) => children.push(block),
            Some(Frame::Quote { children }) => children.push(block),
            _ => self.out.push(block),
        }
    }

    /// Push a styled span into the current inline frame (paragraph/heading/table-cell/link). If no inline
    /// frame is open (loose inline text at the top level — malformed), open a paragraph so the run is not
    /// dropped (the trailing-remainder rule applied eagerly).
    fn push_span(&mut self, span: MdSpan) {
        match self.stack.last_mut() {
            Some(Frame::Paragraph { spans })
            | Some(Frame::Heading { spans, .. })
            | Some(Frame::TableCell { spans })
            | Some(Frame::Link { spans, .. }) => spans.push(span),
            _ => {
                self.stack.push(Frame::Paragraph { spans: vec![span] });
            }
        }
    }

    /// Push raw text into the current frame. Code blocks accumulate verbatim text; everything else
    /// becomes an [`MdSpan`] carrying the active emphasis flags (or an inline-code flag when `is_code`).
    fn push_text(&mut self, text: &str, is_code: bool) {
        // A code BLOCK swallows text verbatim (no span styling).
        if let Some(Frame::CodeBlock { code, .. }) = self.stack.last_mut() {
            code.push_str(text);
            return;
        }
        let span = MdSpan {
            text: text.to_string(),
            bold: self.inline.bold > 0,
            italic: self.inline.italic > 0,
            code: is_code,
            strikethrough: self.inline.strikethrough > 0,
            link: None,
        };
        self.push_span(span);
    }

    /// Flush any still-open frames when the stream ends (RISK-3 trailing-remainder rule): pop every
    /// remaining frame so no content is dropped. Returns the finished top-level block sequence.
    fn finish(mut self) -> Vec<MdBlock> {
        // Commit a final partial body row if a table was left open mid-row.
        while let Some(frame) = self.stack.last() {
            match frame {
                Frame::Link { .. } => self.close_link(),
                _ => self.close_frame(),
            }
            // close_frame/close_link pop one frame each iteration; guard against an Item/Cell that
            // re-pushes nothing by checking length monotonically.
            // (Both close_* always pop exactly one frame, so the loop terminates.)
        }
        self.out
    }
}

/// Commit the table's accumulated `current_row` into `rows` and reset it — called on a `TableRow` end.
/// Kept separate from [`Folder::close_frame`] because a `TableRow` opens no stack frame (cells do).
impl Folder {
    fn on_end_table_row(&mut self) {
        if let Some(Frame::Table {
            rows,
            current_row,
            in_head,
            ..
        }) = self.stack.last_mut()
        {
            if *in_head {
                // The header row is captured cell-by-cell into `headers`; nothing to commit here.
                *in_head = false;
            } else if !current_row.is_empty() {
                rows.push(std::mem::take(current_row));
            }
        }
    }
}

/// Pop the innermost frame ONLY if it is a [`Frame::Link`], returning it; otherwise leave the stack and
/// return `None`. Avoids accidentally popping a non-link frame on an unbalanced `Link` end.
fn pop_if_link(stack: &mut Vec<Frame>) -> Option<Frame> {
    if matches!(stack.last(), Some(Frame::Link { .. })) {
        stack.pop()
    } else {
        None
    }
}

/// Clamp a pulldown [`HeadingLevel`] to a `1..=6` `u8` (RISK-3: never an out-of-range index into
/// the heading-scale table downstream).
fn clamp_heading(level: HeadingLevel) -> u8 {
    let n = match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    };
    n.clamp(1, 6)
}

/// The font size (points) for a heading of `level` (1..=6), REUSING the MT-012 [`HEADING_SCALE`] table.
/// `line_layout::HEADING_SCALE` only defines h1..h3 explicitly (its values 1.8 / 1.5 / 1.25); h4..h6 step
/// down to 1.1 / 1.0 / 1.0 — extending, not re-declaring, the editor's table (the first three levels read
/// straight from `HEADING_SCALE`, so the shared look is preserved — AC5).
fn heading_size(level: u8) -> f32 {
    let scale = match level {
        1 => HEADING_SCALE[0],
        2 => HEADING_SCALE[1],
        3 => HEADING_SCALE[2],
        4 => 1.1,
        _ => 1.0,
    };
    BASE_FONT_SIZE * scale
}

/// Paint a parsed [`MdBlock`] sequence into `ui`, REUSING the MT-012 look (see module docs). Best-effort
/// and panic-free for any block shape (RISK-3 / MC-3). `palette` supplies every color (no `Color32`
/// literal). Links render via [`egui::Ui::hyperlink_to`] (clickable + egui `Link` AccessKit role).
pub fn render_blocks(ui: &mut egui::Ui, blocks: &[MdBlock]) {
    let palette = active_palette(ui);
    let bold_available = bold_family_available(ui.ctx());
    for block in blocks {
        render_block(ui, block, &palette, bold_available);
        ui.add_space(BLOCK_GAP_PTS);
    }
}

/// Resolve the active [`HsPalette`] from egui's current visuals so colors track the live theme without
/// threading a palette argument (matches the reading-mode precedent). The editor host seeds egui's
/// Visuals from the active `HsTheme`, so dark visuals -> the dark palette and vice versa.
fn active_palette(ui: &egui::Ui) -> HsPalette {
    if ui.visuals().dark_mode {
        HsPalette::dark()
    } else {
        HsPalette::light()
    }
}

/// Paint a single block.
fn render_block(ui: &mut egui::Ui, block: &MdBlock, palette: &HsPalette, bold_available: bool) {
    match block {
        MdBlock::Heading { level, spans } => {
            let size = heading_size(*level);
            // A heading is bold by convention (matches the editor's heading weight); fold the bold flag
            // into every span so the heading reads heavier than body text.
            render_span_line(
                ui,
                spans,
                size,
                palette,
                bold_available,
                /*force_bold=*/ true,
            );
        }
        MdBlock::Paragraph { spans } => {
            render_span_line(ui, spans, BASE_FONT_SIZE, palette, bold_available, false);
        }
        MdBlock::BulletList { items } => {
            render_list(ui, items, None, palette, bold_available);
        }
        MdBlock::OrderedList { start, items } => {
            render_list(ui, items, Some(*start), palette, bold_available);
        }
        MdBlock::Quote { children } => {
            render_quote(ui, children, palette, bold_available);
        }
        MdBlock::CodeBlock { lang, code } => {
            render_code_block(ui, lang.as_deref(), code, palette);
        }
        MdBlock::Table { headers, rows } => {
            render_table(ui, headers, rows, palette, bold_available);
        }
        MdBlock::Rule => {
            ui.separator();
        }
    }
}

/// Render one line of inline spans at `size`, wrapping. Each span is appended to a single horizontal,
/// wrapping run so bold/italic/code/strikethrough mix inline; link spans break out to
/// [`egui::Ui::hyperlink_to`] so they stay clickable.
fn render_span_line(
    ui: &mut egui::Ui,
    spans: &[MdSpan],
    size: f32,
    palette: &HsPalette,
    bold_available: bool,
    force_bold: bool,
) {
    // An empty heading/paragraph (RISK-3: "a heading with no text") still consumes a line so layout stays
    // sane; render a zero-width space label rather than nothing.
    if spans.is_empty() {
        ui.label(egui::RichText::new(" ").size(size));
        return;
    }
    ui.horizontal_wrapped(|ui| {
        // Tighten inter-span spacing so adjacent runs read as one line of prose.
        ui.spacing_mut().item_spacing.x = 0.0;
        for span in spans {
            if let Some(href) = &span.link {
                // Links are clickable + inherit egui's Link AccessKit role (the contract rule). Style the
                // label text via the shared shim so the link color/underline matches the editor.
                let mut style = span.style();
                if force_bold {
                    style.bold = true;
                }
                let fmt = md_span_text_format(style, size, palette, bold_available);
                let rich = rich_text_from_format(&span.text, &fmt);
                ui.hyperlink_to(rich, href);
            } else {
                let mut style = span.style();
                if force_bold {
                    style.bold = true;
                }
                let fmt = md_span_text_format(style, size, palette, bold_available);
                ui.label(rich_text_from_format(&span.text, &fmt));
            }
        }
    });
}

/// Build an [`egui::RichText`] from the editor's [`egui::text::TextFormat`] (the shim output) so the same
/// font family / size / color / italic-skew / underline / strikethrough the WYSIWYG editor uses are
/// applied here. Keeps ONE styling source (AC5).
fn rich_text_from_format(text: &str, fmt: &egui::text::TextFormat) -> egui::RichText {
    let mut rich = egui::RichText::new(text)
        .font(fmt.font_id.clone())
        .color(fmt.color);
    if fmt.italics {
        rich = rich.italics();
    }
    if fmt.underline != egui::Stroke::NONE {
        rich = rich.underline();
    }
    if fmt.strikethrough != egui::Stroke::NONE {
        rich = rich.strikethrough();
    }
    rich
}

/// Render a list: a `•` prefix (bullet) or `N.` prefix (ordered), each item indented by [`LIST_INDENT_PTS`]
/// (the MT-012 list indent), with item children rendered recursively (so a nested list-in-item folds).
fn render_list(
    ui: &mut egui::Ui,
    items: &[Vec<MdBlock>],
    ordered_start: Option<u64>,
    palette: &HsPalette,
    bold_available: bool,
) {
    let mut number = ordered_start.unwrap_or(1);
    for item in items {
        ui.horizontal_top(|ui| {
            ui.add_space(LIST_INDENT_PTS * 0.25);
            let prefix = match ordered_start {
                Some(_) => format!("{number}."),
                None => "\u{2022}".to_string(), // bullet •
            };
            ui.label(
                egui::RichText::new(prefix)
                    .size(BASE_FONT_SIZE)
                    .color(palette.text_subtle),
            );
            ui.add_space(4.0);
            // The item body is its own block sequence; render it in an indented vertical group so nested
            // lists/quotes inside the item fold correctly (RISK-2 nesting).
            ui.vertical(|ui| {
                for child in item {
                    render_block(ui, child, palette, bold_available);
                }
            });
        });
        number = number.saturating_add(1);
    }
}

/// Render a blockquote: a [`BLOCKQUOTE_BAR_WIDTH_PTS`]-wide accent left bar + child blocks indented by
/// [`BLOCKQUOTE_INDENT_PTS`] (the exact MT-012 quote chrome). Children render recursively so a nested
/// quote / list inside the quote folds.
fn render_quote(
    ui: &mut egui::Ui,
    children: &[MdBlock],
    palette: &HsPalette,
    bold_available: bool,
) {
    ui.horizontal_top(|ui| {
        // The 3px accent left bar (BLOCKQUOTE_BAR_WIDTH_PTS): draw a thin filled rect spanning the quote.
        let (rect, _resp) = ui.allocate_exact_size(
            egui::vec2(BLOCKQUOTE_BAR_WIDTH_PTS, 1.0),
            egui::Sense::hover(),
        );
        // Defer the bar paint until we know the content height: paint a full-height bar by capturing the
        // group's rect. egui paints children first, so draw the bar over the group's min rect afterward.
        let bar_x = rect.min.x;
        ui.add_space(BLOCKQUOTE_INDENT_PTS - BLOCKQUOTE_BAR_WIDTH_PTS);
        let inner = ui.vertical(|ui| {
            for child in children {
                render_block(ui, child, palette, bold_available);
            }
        });
        let bar_rect = egui::Rect::from_min_max(
            egui::pos2(bar_x, inner.response.rect.min.y),
            egui::pos2(bar_x + BLOCKQUOTE_BAR_WIDTH_PTS, inner.response.rect.max.y),
        );
        ui.painter().rect_filled(bar_rect, 0.0, palette.accent);
    });
}

/// Render a fenced/indented code block: a rounded [`egui::Frame`] filled with the theme code background,
/// padded by [`CODE_PADDING_PTS`] (the MT-012 code-frame chrome), with monospace verbatim code.
fn render_code_block(ui: &mut egui::Ui, lang: Option<&str>, code: &str, palette: &HsPalette) {
    let pad = CODE_PADDING_PTS as i8;
    egui::Frame::new()
        .fill(palette.surface)
        .stroke(egui::Stroke::new(1.0, palette.border))
        .inner_margin(egui::Margin::same(pad))
        .corner_radius(6.0)
        .show(ui, |ui| {
            // A trailing newline from the fence is trimmed so the frame does not show an empty last line.
            let body = code.strip_suffix('\n').unwrap_or(code);
            if let Some(lang) = lang {
                if !lang.is_empty() {
                    ui.label(
                        egui::RichText::new(lang)
                            .monospace()
                            .size(BASE_FONT_SIZE * 0.85)
                            .color(palette.text_subtle),
                    );
                }
            }
            ui.label(
                egui::RichText::new(body)
                    .monospace()
                    .size(BASE_FONT_SIZE)
                    .color(palette.text_subtle),
            );
        });
}

/// Render a GFM table: header cells in bold + body rows, every cell border stroked via
/// [`egui::Painter::rect_stroke`] and clipped to its bounds (the MT-012 table chrome — a long cell never
/// overflows its neighbor). Column count is the MAX cell count across header + rows; a ragged row's
/// missing cells render empty (RISK-3 bounds-checked: never index past a row's cells).
fn render_table(
    ui: &mut egui::Ui,
    headers: &[Vec<MdSpan>],
    rows: &[Vec<Vec<MdSpan>>],
    palette: &HsPalette,
    bold_available: bool,
) {
    let cols = headers
        .len()
        .max(rows.iter().map(|r| r.len()).max().unwrap_or(0))
        .max(1);
    let avail_w = ui.available_width().max(1.0);
    let col_w = (avail_w / cols as f32).max(1.0);
    let row_h = BASE_FONT_SIZE + 10.0;

    // Allocate the whole table area, then paint cell borders + clipped content with the painter (the
    // MT-012 painter table path) so a long cell cannot overflow its neighbor (RISK-3 / MC-006 parity).
    let total_rows = if headers.is_empty() {
        rows.len()
    } else {
        rows.len() + 1
    };
    let total_h = row_h * total_rows as f32;
    let (rect, _resp) = ui.allocate_exact_size(
        egui::vec2(avail_w, total_h.max(row_h)),
        egui::Sense::hover(),
    );
    let painter = ui.painter();

    let mut y = rect.min.y;
    // Header row (bold).
    if !headers.is_empty() {
        paint_table_row(
            painter,
            headers,
            rect.min.x,
            y,
            col_w,
            row_h,
            cols,
            palette,
            bold_available,
            true,
        );
        y += row_h;
    }
    for row in rows {
        paint_table_row(
            painter,
            row,
            rect.min.x,
            y,
            col_w,
            row_h,
            cols,
            palette,
            bold_available,
            false,
        );
        y += row_h;
    }
}

/// Paint one table row of `cols` cells starting at `(x0, y)`. Each cell gets a stroked border and its
/// inline spans painted as a single clipped galley (RISK-3: a `cell_idx` past the row's cell count renders
/// an empty cell rather than panicking).
#[allow(clippy::too_many_arguments)]
fn paint_table_row(
    painter: &egui::Painter,
    cells: &[Vec<MdSpan>],
    x0: f32,
    y: f32,
    col_w: f32,
    row_h: f32,
    cols: usize,
    palette: &HsPalette,
    bold_available: bool,
    header: bool,
) {
    for c in 0..cols {
        let x = x0 + c as f32 * col_w;
        let cell_rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(col_w, row_h));
        painter.rect_stroke(
            cell_rect,
            0.0,
            egui::Stroke::new(1.0, palette.border),
            egui::StrokeKind::Inside,
        );
        // Bounds-checked cell access: a ragged row with fewer cells than `cols` paints an empty cell.
        let Some(spans) = cells.get(c) else { continue };
        // Build a single layout job for the cell's spans, clipped to the cell rect (no overflow).
        let cell_painter = painter.with_clip_rect(cell_rect);
        let mut job = egui::text::LayoutJob::default();
        job.wrap.max_width = (col_w - 6.0).max(1.0);
        for span in spans {
            let mut style = span.style();
            if header {
                style.bold = true;
            }
            let fmt = md_span_text_format(style, BASE_FONT_SIZE, palette, bold_available);
            job.append(&span.text, 0.0, fmt);
        }
        if job.sections.is_empty() {
            // An empty cell still lays out one (empty) section so the row height is consistent.
            let fmt = md_span_text_format(
                MdSpanStyle::default(),
                BASE_FONT_SIZE,
                palette,
                bold_available,
            );
            job.append("", 0.0, fmt);
        }
        let galley = cell_painter.layout_job(job);
        cell_painter.galley(egui::pos2(x + 3.0, y + 4.0), galley, palette.text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// AC1 / MC-2: the canonical fixture (h1, h2, bullet list, GFM table, blockquote, fenced code) folds
    /// into the expected MdBlock sequence with correct nesting.
    #[test]
    fn parse_canonical_fixture_block_sequence() {
        let src = "# H1\n\n## H2\n\n- one\n- two\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\n> quoted\n\n```rust\nfn x() {}\n```\n";
        let blocks = parse_markdown(src);

        // Heading h1.
        assert!(
            matches!(&blocks[0], MdBlock::Heading { level: 1, spans } if span_text(spans) == "H1"),
            "block[0] must be H1 (got {:?})",
            blocks[0]
        );
        // Heading h2.
        assert!(
            matches!(&blocks[1], MdBlock::Heading { level: 2, spans } if span_text(spans) == "H2"),
            "block[1] must be H2 (got {:?})",
            blocks[1]
        );
        // Bullet list of two items.
        match &blocks[2] {
            MdBlock::BulletList { items } => {
                assert_eq!(items.len(), 2, "two list items");
                assert_eq!(item_text(&items[0]), "one");
                assert_eq!(item_text(&items[1]), "two");
            }
            other => panic!("block[2] must be a BulletList (got {other:?})"),
        }
        // GFM table: 1 header row (2 cells), 1+ body rows.
        match &blocks[3] {
            MdBlock::Table { headers, rows } => {
                assert_eq!(headers.len(), 2, "table header has 2 cells");
                assert_eq!(span_text(&headers[0]), "a");
                assert_eq!(span_text(&headers[1]), "b");
                assert!(!rows.is_empty(), "table has at least one body row");
                assert_eq!(span_text(&rows[0][0]), "1");
                assert_eq!(span_text(&rows[0][1]), "2");
            }
            other => panic!("block[3] must be a Table (got {other:?})"),
        }
        // Blockquote.
        assert!(
            matches!(&blocks[4], MdBlock::Quote { children } if !children.is_empty()),
            "block[4] must be a Quote (got {:?})",
            blocks[4]
        );
        // Fenced code block with lang=rust.
        match &blocks[5] {
            MdBlock::CodeBlock { lang, code } => {
                assert_eq!(lang.as_deref(), Some("rust"), "fence lang is rust");
                assert!(
                    code.contains("fn x()"),
                    "code body preserved (got {code:?})"
                );
            }
            other => panic!("block[5] must be a CodeBlock (got {other:?})"),
        }
    }

    /// MC-2 nesting: a list inside a blockquote folds with correct structure (container stack works).
    #[test]
    fn nested_list_in_quote_folds() {
        let src = "> a quote\n>\n> - nested one\n> - nested two\n";
        let blocks = parse_markdown(src);
        let MdBlock::Quote { children } = &blocks[0] else {
            panic!("expected a Quote (got {:?})", blocks[0]);
        };
        // The quote holds a paragraph AND a nested bullet list.
        assert!(
            children
                .iter()
                .any(|b| matches!(b, MdBlock::BulletList { items } if items.len() == 2)),
            "the quote must contain a nested 2-item bullet list (got {children:?})"
        );
    }

    /// Inline marks: bold / italic / inline-code / strikethrough / link spans carry the right flags.
    #[test]
    fn inline_marks_parse() {
        let blocks = parse_markdown("**b** _i_ `c` ~~s~~ [t](http://x)");
        let MdBlock::Paragraph { spans } = &blocks[0] else {
            panic!("expected a paragraph (got {:?})", blocks[0]);
        };
        let find = |t: &str| {
            spans
                .iter()
                .find(|s| s.text == t)
                .cloned()
                .unwrap_or_default()
        };
        assert!(find("b").bold, "**b** is bold");
        assert!(find("i").italic, "_i_ is italic");
        assert!(find("c").code, "`c` is inline code");
        assert!(find("s").strikethrough, "~~s~~ is strikethrough");
        let link = find("t");
        assert_eq!(
            link.link.as_deref(),
            Some("http://x"),
            "[t](http://x) carries the href"
        );
    }

    /// AC3 / MC-3: malformed markdown (unterminated fence, ragged table, empty heading, deep nesting)
    /// parses best-effort with NO panic and NO dropped trailing content.
    #[test]
    fn malformed_markdown_is_panic_free_and_preserves_trailing() {
        // Unterminated code fence: the body after ``` must survive as a code block (trailing remainder).
        let unterminated = "intro\n\n```\nunclosed code body line\nstill in the fence";
        let blocks = parse_markdown(unterminated);
        assert!(
            blocks.iter().any(|b| matches!(b, MdBlock::CodeBlock { code, .. } if code.contains("unclosed code body line"))),
            "an unterminated fence keeps its body as a code block (got {blocks:?})"
        );

        // Ragged table: a body row with fewer cells than the header must not panic and must keep the cells
        // it has.
        let ragged = "| a | b | c |\n|---|---|---|\n| 1 |\n";
        let rblocks = parse_markdown(ragged);
        assert!(
            rblocks.iter().any(|b| matches!(b, MdBlock::Table { .. })),
            "a ragged table still parses to a Table (got {rblocks:?})"
        );

        // Empty heading: `#` with no text -> a Heading with empty spans, level clamped.
        let empty_heading = parse_markdown("#\n");
        assert!(
            empty_heading
                .iter()
                .any(|b| matches!(b, MdBlock::Heading { level, .. } if (1..=6).contains(level))),
            "an empty heading parses with a clamped level (got {empty_heading:?})"
        );

        // Deeply nested list: 10 levels deep must not stack-overflow the fold (iterative stack).
        let mut deep = String::new();
        for i in 0..10 {
            deep.push_str(&"  ".repeat(i));
            deep.push_str("- item\n");
        }
        let dblocks = parse_markdown(&deep);
        assert!(
            !dblocks.is_empty(),
            "deep nesting parses without panic (got {} blocks)",
            dblocks.len()
        );

        // Loose trailing inline text after a block with no closing structure is never dropped.
        let trailing = parse_markdown("# head\n\ntrailing words");
        assert!(
            trailing.iter().any(|b| matches!(b, MdBlock::Paragraph { spans } if span_text(spans).contains("trailing words"))),
            "trailing paragraph text is preserved (got {trailing:?})"
        );
    }

    /// Empty input yields an empty block sequence (no spurious blocks, no panic).
    #[test]
    fn empty_input_is_empty() {
        assert!(parse_markdown("").is_empty());
        assert!(parse_markdown("   \n  \n").is_empty());
    }

    /// heading_size reuses the MT-012 HEADING_SCALE for h1..h3 (the shared-look invariant, AC5) and steps
    /// down for h4..h6; every level is > the body size for h1..h4 and >= body for h5/h6.
    #[test]
    fn heading_size_reuses_mt012_scale() {
        assert_eq!(
            heading_size(1),
            BASE_FONT_SIZE * HEADING_SCALE[0],
            "h1 uses HEADING_SCALE[0]"
        );
        assert_eq!(
            heading_size(2),
            BASE_FONT_SIZE * HEADING_SCALE[1],
            "h2 uses HEADING_SCALE[1]"
        );
        assert_eq!(
            heading_size(3),
            BASE_FONT_SIZE * HEADING_SCALE[2],
            "h3 uses HEADING_SCALE[2]"
        );
        assert!(heading_size(1) > BASE_FONT_SIZE, "h1 is larger than body");
        assert!(heading_size(4) > BASE_FONT_SIZE, "h4 is larger than body");
        assert!(
            heading_size(6) >= BASE_FONT_SIZE,
            "h6 is at least body size"
        );
        // An out-of-range level (defensive — clamp_heading already bounds it) never panics.
        assert!(heading_size(99) >= BASE_FONT_SIZE);
    }

    // ── test helpers ──────────────────────────────────────────────────────────────────────────────────

    /// Concatenate a span vector's text.
    fn span_text(spans: &[MdSpan]) -> String {
        spans.iter().map(|s| s.text.as_str()).collect()
    }

    /// The concatenated text of a list item's first paragraph.
    fn item_text(item: &[MdBlock]) -> String {
        for block in item {
            if let MdBlock::Paragraph { spans } = block {
                return span_text(spans);
            }
        }
        String::new()
    }
}
