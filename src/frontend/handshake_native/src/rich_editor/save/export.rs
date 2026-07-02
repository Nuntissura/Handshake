//! Save-to-format EXPORT projections for the native rich-text editor (WP-KERNEL-012 MT-020).
//!
//! Port of `app/src/lib/editor/export_formats.ts`: one-way DocNode-tree walkers that emit a
//! byte payload + filename + MIME for each export format. There is NO comrak here — per the
//! KERNEL_BUILDER gate, comrak/egui_commonmark are the SECONDARY tools for markdown IMPORT +
//! reading-mode RENDER (MT-055), never export. ProseMirrorJson reuses the MT-011 `doc_json`
//! serializer with a typed projection envelope.
//!
//! ## Formats
//!
//! - **HtmlSelfContained**: HTML5 markup; image embeds are base64-inlined as `data:` URLs,
//!   subject to the size guards (per-image <= 10 MB, cumulative <= 50 MB, video NEVER inlined).
//!   An over-limit / unresolvable image emits a VISIBLE `<img ... data-hs-export-error="…">`
//!   placeholder pointing at the backend content URL (fail-OPEN to a reference link, never a
//!   silent blank — RISK-1).
//! - **HtmlReferenceLinked**: identical structure, but media `src` is always the backend
//!   content URL (no inlining, no size guard needed).
//! - **Markdown**: lossy CommonMark. Unsupported/future node kinds emit a
//!   `<!-- unsupported: {kind} -->` comment and never panic (red-team table div-by-zero guard).
//! - **PlainText**: every text leaf's rope content concatenated, blocks newline-separated.
//! - **ProseMirrorJson**: `{ schema_version, projection_disclaimer, content }` envelope over
//!   the MT-011 `to_content_json_value`.
//!
//! ## XSS-safe HTML assembly (impl note 1)
//!
//! HTML is built by a recursive `walk_node_to_html` writing into a `String` with strict entity
//! escaping (`&`, `<`, `>`, `"`, `'`). NEVER raw string concatenation of document text into
//! markup. The `html_escape_*` fns + the `script_text_is_escaped_in_html` test prove it.
//!
//! ## Asset fetching is the CALLER's job (HBR-QUIET / off-thread)
//!
//! The HTML self-contained walker does NOT perform network I/O itself. The caller resolves every
//! image asset's bytes FIRST (on a background thread — see [`AssetByteSource`]) into a map, then
//! passes that map in. This keeps the export walker a pure, synchronous, fully-unit-testable
//! function (no tokio, no blocking) and keeps all network off the egui frame thread (MC-004).

use std::collections::HashMap;

use serde_json::{json, Value as JsonValue};

use crate::rich_editor::document_model::doc_json::{
    to_content_json_value, RICH_DOCUMENT_SCHEMA_VERSION,
};
use crate::rich_editor::document_model::node::{BlockNode, Child, Mark, NodeKind, TextLeaf};

use base64::Engine as _;

/// Per-image inline cap for HTML self-contained (mirrors `export_formats.ts`
/// `HTML_INLINE_IMAGE_MAX_BYTES`): an image larger than this is NOT base64-inlined; it falls
/// back to a reference link carrying `data-hs-export-error="size_exceeded"`.
pub const HTML_INLINE_IMAGE_MAX_BYTES: usize = 10 * 1024 * 1024;

/// Cumulative inline cap across ALL images in one HTML self-contained export (mirrors
/// `HTML_INLINE_TOTAL_MAX_BYTES`): once inlining one more image would push the running total
/// past this, that image and every later one fall back to a reference link
/// (`data-hs-export-error="total_size_exceeded"`). The CHECK is done BEFORE fetching/inlining so
/// a 500 MB document can never be assembled (RISK-1 / red-team total-cap control).
pub const HTML_INLINE_TOTAL_MAX_BYTES: usize = 50 * 1024 * 1024;

/// The five export formats (matches `export_formats.ts`). `HtmlReferenceLiked` keeps the
/// contract's exact (mis)spelling in the enum NAME-FACING string is irrelevant; the variant is
/// spelled correctly here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// HTML5 with images base64-inlined as `data:` URLs (size-guarded).
    HtmlSelfContained,
    /// HTML5 with media `src` pointing at the backend asset content URL.
    HtmlReferenceLinked,
    /// Lossy CommonMark.
    Markdown,
    /// Plain text (all leaf text concatenated).
    PlainText,
    /// The ProseMirror JSON projection envelope.
    ProseMirrorJson,
}

impl ExportFormat {
    /// The file extension (no dot) for this format's default filename.
    pub fn extension(self) -> &'static str {
        match self {
            ExportFormat::HtmlSelfContained | ExportFormat::HtmlReferenceLinked => "html",
            ExportFormat::Markdown => "md",
            ExportFormat::PlainText => "txt",
            ExportFormat::ProseMirrorJson => "json",
        }
    }

    /// The MIME type for this format (used by the file dialog + any future HTTP serving).
    pub fn mime(self) -> &'static str {
        match self {
            ExportFormat::HtmlSelfContained | ExportFormat::HtmlReferenceLinked => {
                "text/html;charset=utf-8"
            }
            ExportFormat::Markdown => "text/markdown;charset=utf-8",
            ExportFormat::PlainText => "text/plain;charset=utf-8",
            ExportFormat::ProseMirrorJson => "application/json",
        }
    }

    /// A short human label for the format-picker popup.
    pub fn label(self) -> &'static str {
        match self {
            ExportFormat::HtmlSelfContained => "HTML (self-contained)",
            ExportFormat::HtmlReferenceLinked => "HTML (reference-linked)",
            ExportFormat::Markdown => "Markdown",
            ExportFormat::PlainText => "Plain text",
            ExportFormat::ProseMirrorJson => "ProseMirror JSON",
        }
    }

    /// All five formats in picker order.
    pub fn all() -> [ExportFormat; 5] {
        [
            ExportFormat::HtmlSelfContained,
            ExportFormat::HtmlReferenceLinked,
            ExportFormat::Markdown,
            ExportFormat::PlainText,
            ExportFormat::ProseMirrorJson,
        ]
    }
}

/// A completed export: the byte payload, a suggested filename, and the MIME type. The bytes are
/// what the file-save sink writes; the filename seeds the save dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportOutput {
    /// The encoded document bytes (UTF-8 text for every current format).
    pub content: Vec<u8>,
    /// The suggested filename (`{slug(title)}.{ext}`).
    pub filename: String,
    /// The MIME type.
    pub mime: String,
}

impl ExportOutput {
    /// The payload as a UTF-8 string (every current format is UTF-8 text), for assertions.
    pub fn as_str(&self) -> std::borrow::Cow<'_, str> {
        String::from_utf8_lossy(&self.content)
    }
}

/// Why an export could not be produced. Today the walkers never fail for a well-formed doc (an
/// unsupported node degrades to a placeholder, never an error), so this is reserved for future
/// hard-failure cases; it keeps `export_document` a `Result` so a caller has one error channel.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ExportError {
    /// Serializing the ProseMirror JSON envelope failed (should be impossible for a valid tree).
    #[error("failed to serialize ProseMirror JSON: {0}")]
    JsonSerialize(String),
}

/// A resolved image asset's bytes + MIME, supplied by the caller for HTML self-contained
/// inlining. The caller fetches `GET /workspaces/{ws}/assets/{asset_id}/content` off the egui
/// frame thread (HBR-QUIET) and fills this map keyed by `asset_id` BEFORE calling
/// [`export_document`]; the walker never does network I/O.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedAsset {
    /// The asset's raw bytes (for base64 inlining).
    pub bytes: Vec<u8>,
    /// The asset's MIME (e.g. `image/png`); used in the `data:{mime};base64,…` URL.
    pub mime: String,
}

/// The asset-resolution input for an HTML self-contained export: a map from `asset_id` to its
/// resolved bytes. An asset id absent from the map (resolution failed / not fetched) degrades to
/// a reference-link placeholder carrying `data-hs-export-error="unresolved"` — never a silent
/// blank (RISK-1). For every non-HTML-self-contained format this is ignored.
pub type AssetByteSource = HashMap<String, ResolvedAsset>;

/// The backend base URL used to build reference-linked media `src` URLs. Kept as a parameter
/// (defaulting to [`crate::backend_client::BACKEND_BASE_URL`] at the call site) so a test can
/// assert a stable URL without depending on a running backend.
pub fn asset_content_url(base_url: &str, workspace_id: &str, asset_id: &str) -> String {
    format!("{base_url}/workspaces/{workspace_id}/assets/{asset_id}/content")
}

/// Export `doc` to `format`. `workspace_id` + `base_url` build reference media URLs; `title`
/// seeds the filename; `assets` supplies resolved image bytes for HTML self-contained inlining
/// (ignored by the other formats).
///
/// The walkers never panic on any node kind: an unsupported markdown node emits a comment, an
/// over-limit / unresolved image emits a visible error placeholder, an empty table emits an empty
/// table comment (red-team div-by-zero control).
pub fn export_document(
    doc: &BlockNode,
    format: ExportFormat,
    workspace_id: &str,
    base_url: &str,
    title: &str,
    assets: &AssetByteSource,
) -> Result<ExportOutput, ExportError> {
    let content: Vec<u8> = match format {
        ExportFormat::PlainText => export_plain_text(doc).into_bytes(),
        ExportFormat::Markdown => export_markdown(doc).into_bytes(),
        ExportFormat::ProseMirrorJson => export_prosemirror_json(doc)?.into_bytes(),
        ExportFormat::HtmlSelfContained => export_html(
            doc,
            HtmlMediaMode::SelfContained { assets },
            workspace_id,
            base_url,
            title,
        )
        .into_bytes(),
        ExportFormat::HtmlReferenceLinked => export_html(
            doc,
            HtmlMediaMode::ReferenceLinked,
            workspace_id,
            base_url,
            title,
        )
        .into_bytes(),
    };
    Ok(ExportOutput {
        content,
        filename: format!("{}.{}", slugify(title), format.extension()),
        mime: format.mime().to_string(),
    })
}

/// Slugify a title into a filename-safe stem (no spaces — GLOBAL-NAMING). Lowercases, replaces
/// any non-alphanumeric run with a single `-`, trims leading/trailing `-`, and falls back to
/// `untitled` for an empty result.
pub fn slugify(title: &str) -> String {
    let mut out = String::new();
    let mut last_dash = true; // avoids a leading dash
    for ch in title.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "untitled".to_string()
    } else {
        out
    }
}

// ──────────────────────────────────────────────────────────────────────────────────────────────
// PlainText
// ──────────────────────────────────────────────────────────────────────────────────────────────

/// Concatenate every text leaf's content, separating top-level blocks (and recursively, list
/// items / table rows / quote bodies) by newlines. An inline hsLink/transclusion atom contributes
/// its short reference label so the text reads sensibly.
pub fn export_plain_text(doc: &BlockNode) -> String {
    let mut blocks: Vec<String> = Vec::new();
    for child in &doc.children {
        if let Child::Block(b) = child {
            blocks.push(block_plain_text(b));
        }
    }
    // Join top-level blocks with a single newline; trim a trailing newline so the output ends
    // cleanly (a doc with one paragraph "hi" exports "hi", not "hi\n").
    blocks.join("\n")
}

/// The plain text of one block (recursing into container children). Inline leaves concatenate;
/// block children are newline-joined.
fn block_plain_text(node: &BlockNode) -> String {
    if node.kind.is_atom() {
        return String::new();
    }
    // Inline-content blocks: concatenate their inline children with no separator.
    if node.kind.holds_inline_content() {
        let mut s = String::new();
        for child in &node.children {
            match child {
                Child::Text(t) => s.push_str(&t.text.to_string()),
                Child::HsLink(l) => s.push_str(&hs_link_text(l)),
                Child::Transclusion(t) => s.push_str(&format!("[[{}]]", t.ref_value)),
                Child::Block(_) => {}
            }
        }
        return s;
    }
    // Container blocks: newline-join the plain text of each block child.
    let mut parts: Vec<String> = Vec::new();
    for child in &node.children {
        if let Child::Block(b) = child {
            parts.push(block_plain_text(b));
        }
    }
    parts.join("\n")
}

/// The display text of an hsLink atom: its label, or `refKind:refValue` when the label is blank
/// (matching the React node's blank-label fallback).
fn hs_link_text(l: &crate::rich_editor::document_model::node::HsLinkNode) -> String {
    if l.label.is_empty() {
        format!("{}:{}", l.ref_kind, l.ref_value)
    } else {
        l.label.clone()
    }
}

// ──────────────────────────────────────────────────────────────────────────────────────────────
// ProseMirror JSON envelope
// ──────────────────────────────────────────────────────────────────────────────────────────────

/// The read-only projection disclaimer embedded in the ProseMirror JSON export envelope (the
/// contract text verbatim) — a reminder that the canonical authority is the Postgres record.
pub const PROJECTION_DISCLAIMER: &str =
    "This is a read-only export projection. The canonical authority is the Postgres RichDocument record.";

/// Serialize `doc` to the ProseMirror JSON projection envelope:
/// `{ schema_version, projection_disclaimer, content }`, where `content` is the bare
/// `content_json` doc node from the MT-011 serializer.
pub fn export_prosemirror_json(doc: &BlockNode) -> Result<String, ExportError> {
    let envelope = json!({
        "schema_version": RICH_DOCUMENT_SCHEMA_VERSION,
        "projection_disclaimer": PROJECTION_DISCLAIMER,
        "content": to_content_json_value(doc),
    });
    serde_json::to_string_pretty(&envelope).map_err(|e| ExportError::JsonSerialize(e.to_string()))
}

// ──────────────────────────────────────────────────────────────────────────────────────────────
// Markdown (lossy CommonMark)
// ──────────────────────────────────────────────────────────────────────────────────────────────

/// Export `doc` to lossy CommonMark. Block-level walker; an unsupported node kind emits a
/// `<!-- unsupported: {kind} -->` comment and continues (never panics — red-team control).
pub fn export_markdown(doc: &BlockNode) -> String {
    let mut out = String::new();
    for child in &doc.children {
        if let Child::Block(b) = child {
            md_block(&mut out, b, 0);
        }
    }
    out
}

/// Append the markdown for one block to `out`. `list_depth` tracks nested-list indentation.
fn md_block(out: &mut String, node: &BlockNode, list_depth: usize) {
    match node.kind {
        NodeKind::Paragraph => {
            out.push_str(&md_inline(node));
            out.push_str("\n\n");
        }
        NodeKind::Heading(level) => {
            for _ in 0..level.get() {
                out.push('#');
            }
            out.push(' ');
            out.push_str(&md_inline(node));
            out.push_str("\n\n");
        }
        NodeKind::Blockquote => {
            // Prefix each produced line with "> ".
            let mut inner = String::new();
            for child in &node.children {
                if let Child::Block(b) = child {
                    md_block(&mut inner, b, list_depth);
                }
            }
            for line in inner.trim_end_matches('\n').lines() {
                out.push_str("> ");
                out.push_str(line);
                out.push('\n');
            }
            out.push('\n');
        }
        NodeKind::CodeBlock => {
            let lang = node
                .attrs
                .get("language")
                .and_then(JsonValue::as_str)
                .unwrap_or("");
            out.push_str("```");
            out.push_str(lang);
            out.push('\n');
            out.push_str(&md_inline(node));
            out.push_str("\n```\n\n");
        }
        NodeKind::OrderedList | NodeKind::BulletList => {
            let ordered = matches!(node.kind, NodeKind::OrderedList);
            let mut n = 1usize;
            for child in &node.children {
                if let Some(item) = child.as_block() {
                    let indent = "  ".repeat(list_depth);
                    let marker = if ordered {
                        format!("{n}. ")
                    } else {
                        "- ".to_string()
                    };
                    // A task item carries `checked`; render as a GFM task checkbox.
                    let checkbox = if matches!(item.kind, NodeKind::TaskItem) {
                        if item.task_checked() {
                            "[x] "
                        } else {
                            "[ ] "
                        }
                    } else {
                        ""
                    };
                    out.push_str(&indent);
                    out.push_str(&marker);
                    out.push_str(checkbox);
                    // The item's first inline-content child is its text; nested lists recurse.
                    let mut item_text = String::new();
                    let mut nested = String::new();
                    for ic in &item.children {
                        match ic {
                            Child::Block(b) if b.kind.holds_inline_content() => {
                                item_text.push_str(&md_inline(b));
                            }
                            Child::Block(b) => md_block(&mut nested, b, list_depth + 1),
                            Child::Text(t) => item_text.push_str(&t.text.to_string()),
                            Child::HsLink(l) => item_text.push_str(&hs_link_text(l)),
                            Child::Transclusion(t) => {
                                item_text.push_str(&format!("[[{}]]", t.ref_value))
                            }
                        }
                    }
                    out.push_str(&item_text);
                    out.push('\n');
                    out.push_str(&nested);
                    n += 1;
                }
            }
            out.push('\n');
        }
        NodeKind::TaskItem => {
            // A bare task item not under a list (defensive): render as a single checkbox line.
            let checkbox = if node.task_checked() {
                "- [x] "
            } else {
                "- [ ] "
            };
            out.push_str(checkbox);
            out.push_str(&md_inline(node));
            out.push_str("\n\n");
        }
        NodeKind::Table => md_table(out, node),
        NodeKind::HorizontalRule => out.push_str("---\n\n"),
        NodeKind::HardBreak => out.push('\n'),
        // Doc never appears as a child; rows/cells/headers are handled inside md_table. Anything
        // else as a top-level block is an unsupported/future kind: emit a comment, never panic.
        NodeKind::Doc
        | NodeKind::TableRow
        | NodeKind::TableCell
        | NodeKind::TableHeader
        | NodeKind::ListItem => {
            out.push_str(&format!(
                "<!-- unsupported: {} -->\n\n",
                node.kind.to_json_type()
            ));
        }
    }
}

/// Render a table to a GFM pipe table. Guards against an empty table (0 rows or 0 cols): emits
/// `<!-- empty table -->` and returns, so the column-width / header logic never divides by zero
/// (red-team table div-by-zero control).
fn md_table(out: &mut String, table: &BlockNode) {
    let rows: Vec<&BlockNode> = table
        .children
        .iter()
        .filter_map(Child::as_block)
        .filter(|b| matches!(b.kind, NodeKind::TableRow))
        .collect();
    let col_count = rows
        .iter()
        .map(|r| r.children.iter().filter_map(Child::as_block).count())
        .max()
        .unwrap_or(0);
    if rows.is_empty() || col_count == 0 {
        out.push_str("<!-- empty table -->\n\n");
        return;
    }
    for (ri, row) in rows.iter().enumerate() {
        let cells: Vec<&BlockNode> = row.children.iter().filter_map(Child::as_block).collect();
        out.push('|');
        for ci in 0..col_count {
            let text = cells
                .get(ci)
                .map(|c| md_inline(c).replace('|', "\\|"))
                .unwrap_or_default();
            out.push(' ');
            out.push_str(&text);
            out.push_str(" |");
        }
        out.push('\n');
        // After the first (header) row, emit the GFM header separator row.
        if ri == 0 {
            out.push('|');
            for _ in 0..col_count {
                out.push_str(" --- |");
            }
            out.push('\n');
        }
    }
    out.push('\n');
}

/// Render an inline-content block's children to markdown inline text: marked text runs become
/// `**bold**` / `*italic*` / `~~strike~~` / `` `code` `` / `[text](href)`, hsLinks become
/// `[[refKind:refValue|label]]` wikilink tokens, transclusions `[[refValue]]`. Underline has no
/// CommonMark form, so it is dropped (lossy — the contract names this explicitly).
fn md_inline(node: &BlockNode) -> String {
    let mut s = String::new();
    for child in &node.children {
        match child {
            Child::Text(t) => s.push_str(&md_text_run(t)),
            Child::HsLink(l) => {
                // Convert wikilinks to a [[kind:value]] token (the contract's lossy mapping).
                if l.label.is_empty() {
                    s.push_str(&format!("[[{}:{}]]", l.ref_kind, l.ref_value));
                } else {
                    s.push_str(&format!("[[{}:{}|{}]]", l.ref_kind, l.ref_value, l.label));
                }
            }
            Child::Transclusion(t) => s.push_str(&format!("[[{}]]", t.ref_value)),
            Child::Block(_) => {}
        }
    }
    s
}

/// Render a single marked text run to markdown. A code-marked run uses backticks (and is NOT
/// further wrapped by other marks — code is opaque); otherwise bold/italic/strike/link wrap the
/// text in CommonMark order. Underline is silently dropped (lossy).
fn md_text_run(t: &TextLeaf) -> String {
    let raw = t.text.to_string();
    if t.has_mark_type(&Mark::Code) {
        return format!("`{raw}`");
    }
    let mut text = raw;
    if t.has_mark_type(&Mark::Bold) {
        text = format!("**{text}**");
    }
    if t.has_mark_type(&Mark::Italic) {
        text = format!("*{text}*");
    }
    if t.has_mark_type(&Mark::Strike) {
        text = format!("~~{text}~~");
    }
    // A link wraps last so `[**bold**](href)` is valid.
    for m in &t.marks {
        if let Mark::Link { href } = m {
            text = format!("[{text}]({href})");
        }
    }
    text
}

// ──────────────────────────────────────────────────────────────────────────────────────────────
// HTML (self-contained + reference-linked)
// ──────────────────────────────────────────────────────────────────────────────────────────────

/// How an HTML export resolves media `src`: self-contained inlines image bytes as `data:` URLs
/// (size-guarded), reference-linked points at the backend content URL.
enum HtmlMediaMode<'a> {
    SelfContained { assets: &'a AssetByteSource },
    ReferenceLinked,
}

/// Mutable running state threaded through the HTML walker for the cumulative inline-size guard.
struct HtmlExportCtx<'a> {
    mode: HtmlMediaMode<'a>,
    workspace_id: &'a str,
    base_url: &'a str,
    /// Running total of bytes already inlined (self-contained mode); used for the 50 MB cap.
    inlined_total: usize,
}

/// Export `doc` to an HTML5 document. `mode` decides media handling.
fn export_html(
    doc: &BlockNode,
    mode: HtmlMediaMode<'_>,
    workspace_id: &str,
    base_url: &str,
    title: &str,
) -> String {
    let mut ctx = HtmlExportCtx {
        mode,
        workspace_id,
        base_url,
        inlined_total: 0,
    };
    let mut body = String::new();
    for child in &doc.children {
        if let Child::Block(b) = child {
            walk_node_to_html(&mut body, b, &mut ctx);
        }
    }
    format!(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<title>{}</title>\n</head>\n<body>\n{}</body>\n</html>\n",
        html_escape_text(title),
        body
    )
}

/// Recursively write the HTML for one block. NEVER concatenates raw document text into markup —
/// every text/attr value goes through `html_escape_*` (XSS-safe — impl note 1).
fn walk_node_to_html(out: &mut String, node: &BlockNode, ctx: &mut HtmlExportCtx<'_>) {
    match node.kind {
        NodeKind::Paragraph => {
            // A standalone media-embed paragraph (an hsLink with an `image`/`video` ref kind) is
            // rendered as a media element; an ordinary paragraph as <p>.
            if let Some(link) = node.children.iter().find_map(Child::as_hs_link) {
                if is_media_ref_kind(&link.ref_kind) {
                    write_media_html(out, link, ctx);
                    return;
                }
            }
            out.push_str("<p>");
            html_inline(out, node);
            out.push_str("</p>\n");
        }
        NodeKind::Heading(level) => {
            let h = level.get();
            out.push_str(&format!("<h{h}>"));
            html_inline(out, node);
            out.push_str(&format!("</h{h}>\n"));
        }
        NodeKind::Blockquote => {
            out.push_str("<blockquote>\n");
            for child in &node.children {
                if let Child::Block(b) = child {
                    walk_node_to_html(out, b, ctx);
                }
            }
            out.push_str("</blockquote>\n");
        }
        NodeKind::CodeBlock => {
            out.push_str("<pre><code>");
            // Code text is escaped but carries no inline marks.
            for child in &node.children {
                if let Child::Text(t) = child {
                    out.push_str(&html_escape_text(&t.text.to_string()));
                }
            }
            out.push_str("</code></pre>\n");
        }
        NodeKind::OrderedList | NodeKind::BulletList => {
            let tag = if matches!(node.kind, NodeKind::OrderedList) {
                "ol"
            } else {
                "ul"
            };
            out.push_str(&format!("<{tag}>\n"));
            for child in &node.children {
                if let Child::Block(item) = child {
                    out.push_str("<li>");
                    // Inline content directly, nested blocks recursively.
                    for ic in &item.children {
                        match ic {
                            Child::Block(b) if b.kind.holds_inline_content() => html_inline(out, b),
                            Child::Block(b) => walk_node_to_html(out, b, ctx),
                            Child::Text(t) => out.push_str(&html_escape_text(&t.text.to_string())),
                            Child::HsLink(l) => write_hs_link_html(out, l),
                            Child::Transclusion(t) => {
                                out.push_str(&html_escape_text(&format!("[[{}]]", t.ref_value)))
                            }
                        }
                    }
                    out.push_str("</li>\n");
                }
            }
            out.push_str(&format!("</{tag}>\n"));
        }
        NodeKind::Table => {
            out.push_str("<table>\n");
            for child in &node.children {
                if let Child::Block(row) = child {
                    if !matches!(row.kind, NodeKind::TableRow) {
                        continue;
                    }
                    out.push_str("<tr>\n");
                    for cell_child in &row.children {
                        if let Child::Block(cell) = cell_child {
                            // A TableHeader cell renders <th>, a TableCell <td> (MT-013/backend compat).
                            let tag = if matches!(cell.kind, NodeKind::TableHeader) {
                                "th"
                            } else {
                                "td"
                            };
                            out.push_str(&format!("<{tag}>"));
                            // Cells hold block children (usually a paragraph).
                            for cc in &cell.children {
                                match cc {
                                    Child::Block(b) if b.kind.holds_inline_content() => {
                                        html_inline(out, b)
                                    }
                                    Child::Block(b) => walk_node_to_html(out, b, ctx),
                                    Child::Text(t) => {
                                        out.push_str(&html_escape_text(&t.text.to_string()))
                                    }
                                    Child::HsLink(l) => write_hs_link_html(out, l),
                                    Child::Transclusion(t) => out.push_str(&html_escape_text(
                                        &format!("[[{}]]", t.ref_value),
                                    )),
                                }
                            }
                            out.push_str(&format!("</{tag}>"));
                        }
                    }
                    out.push_str("\n</tr>\n");
                }
            }
            out.push_str("</table>\n");
        }
        NodeKind::HorizontalRule => out.push_str("<hr>\n"),
        NodeKind::HardBreak => out.push_str("<br>\n"),
        // Rows/cells/items are handled inside their parents; a doc is never a child. Anything
        // unexpected emits a visible comment (never silent, never panic).
        NodeKind::Doc
        | NodeKind::TableRow
        | NodeKind::TableCell
        | NodeKind::TableHeader
        | NodeKind::ListItem
        | NodeKind::TaskItem => {
            out.push_str(&format!(
                "<!-- unsupported-block: {} -->\n",
                node.kind.to_json_type()
            ));
        }
    }
}

/// Write an inline-content block's children as inline HTML (marked text runs -> `<strong>` /
/// `<em>` / `<u>` / `<s>` / `<code>` / `<a href>`; hsLinks -> `<a>` chips; transclusions -> a
/// labeled span).
fn html_inline(out: &mut String, node: &BlockNode) {
    for child in &node.children {
        match child {
            Child::Text(t) => write_text_run_html(out, t),
            Child::HsLink(l) => write_hs_link_html(out, l),
            Child::Transclusion(t) => {
                out.push_str("<span class=\"hs-transclusion\" data-hs-ref=\"");
                out.push_str(&html_escape_attr(&t.ref_value));
                out.push_str("\">");
                out.push_str(&html_escape_text(&format!("[[{}]]", t.ref_value)));
                out.push_str("</span>");
            }
            Child::Block(_) => {}
        }
    }
}

/// Write a marked text run as inline HTML, escaping the text and wrapping it in the mark tags.
/// Code is opaque (no nested mark tags). Link wraps last.
fn write_text_run_html(out: &mut String, t: &TextLeaf) {
    let escaped = html_escape_text(&t.text.to_string());
    if t.has_mark_type(&Mark::Code) {
        out.push_str("<code>");
        out.push_str(&escaped);
        out.push_str("</code>");
        return;
    }
    let mut html = escaped;
    if t.has_mark_type(&Mark::Bold) {
        html = format!("<strong>{html}</strong>");
    }
    if t.has_mark_type(&Mark::Italic) {
        html = format!("<em>{html}</em>");
    }
    if t.has_mark_type(&Mark::Underline) {
        html = format!("<u>{html}</u>");
    }
    if t.has_mark_type(&Mark::Strike) {
        html = format!("<s>{html}</s>");
    }
    for m in &t.marks {
        if let Mark::Link { href } = m {
            html = format!("<a href=\"{}\">{html}</a>", html_escape_attr(href));
        }
    }
    out.push_str(&html);
}

/// Write an hsLink atom as an `<a>` carrying its typed ref attrs (escaped). A non-media link is
/// an in-document reference; the href is a `hs:` scheme so it never resolves to a network URL on
/// its own (the reader app routes it).
fn write_hs_link_html(out: &mut String, l: &crate::rich_editor::document_model::node::HsLinkNode) {
    out.push_str("<a class=\"hs-link\" data-hs-ref-kind=\"");
    out.push_str(&html_escape_attr(&l.ref_kind));
    out.push_str("\" data-hs-ref-value=\"");
    out.push_str(&html_escape_attr(&l.ref_value));
    out.push_str("\" href=\"");
    out.push_str(&html_escape_attr(&format!(
        "hs:{}:{}",
        l.ref_kind, l.ref_value
    )));
    out.push_str("\">");
    out.push_str(&html_escape_text(&hs_link_text(l)));
    out.push_str("</a>");
}

/// True when an hsLink `ref_kind` denotes an inline-able media asset. An `image` is inline-able
/// (self-contained mode); a `video` is media but NEVER inlined.
fn is_media_ref_kind(ref_kind: &str) -> bool {
    matches!(ref_kind, "image" | "video" | "asset")
}

/// Write a media embed (`image`/`video`) as an HTML element. In reference-linked mode (or for
/// video, or when the per-image / total cap would be exceeded, or the asset is unresolved), the
/// `src` is the backend content URL and a `data-hs-export-error` attr explains any fallback. In
/// self-contained mode an inline-able image becomes a `data:` URL.
fn write_media_html(
    out: &mut String,
    link: &crate::rich_editor::document_model::node::HsLinkNode,
    ctx: &mut HtmlExportCtx<'_>,
) {
    let asset_id = &link.ref_value;
    let content_url = asset_content_url(ctx.base_url, ctx.workspace_id, asset_id);

    // Video is NEVER inlined (size + format): always a reference <video src> (RISK-1 control).
    if link.ref_kind == "video" {
        out.push_str("<video controls src=\"");
        out.push_str(&html_escape_attr(&content_url));
        out.push_str("\"></video>\n");
        return;
    }

    match &ctx.mode {
        HtmlMediaMode::ReferenceLinked => {
            out.push_str("<img src=\"");
            out.push_str(&html_escape_attr(&content_url));
            out.push_str("\" alt=\"");
            out.push_str(&html_escape_attr(&link.label));
            out.push_str("\">\n");
        }
        HtmlMediaMode::SelfContained { assets } => {
            // Resolve the asset bytes; absent => unresolved fallback (never a silent blank).
            let Some(asset) = assets.get(asset_id) else {
                write_img_error(out, &content_url, &link.label, "unresolved");
                return;
            };
            let size = asset.bytes.len();
            // Per-image cap (check BEFORE inlining — RISK-1).
            if size > HTML_INLINE_IMAGE_MAX_BYTES {
                write_img_error(out, &content_url, &link.label, "size_exceeded");
                return;
            }
            // Cumulative cap (check BEFORE inlining so a 500 MB doc can never be assembled).
            if ctx.inlined_total + size > HTML_INLINE_TOTAL_MAX_BYTES {
                write_img_error(out, &content_url, &link.label, "total_size_exceeded");
                return;
            }
            // Inline as a data: URL.
            let b64 = base64::engine::general_purpose::STANDARD.encode(&asset.bytes);
            ctx.inlined_total += size;
            out.push_str("<img src=\"data:");
            out.push_str(&html_escape_attr(&asset.mime));
            out.push_str(";base64,");
            // base64 STANDARD alphabet is URL/HTML-attr safe (A-Za-z0-9+/=), no escaping needed,
            // but run it through the attr escaper anyway for defense in depth (it is a no-op here).
            out.push_str(&b64);
            out.push_str("\" alt=\"");
            out.push_str(&html_escape_attr(&link.label));
            out.push_str("\">\n");
        }
    }
}

/// Write a fail-OPEN `<img>` placeholder: a reference link to the backend content URL carrying a
/// `data-hs-export-error="{kind}"` attribute, so the failure is VISIBLE in the exported HTML and
/// the image can still be fetched by a reader with backend access (never a silent blank — RISK-1).
fn write_img_error(out: &mut String, content_url: &str, alt: &str, error_kind: &str) {
    out.push_str("<img data-hs-export-error=\"");
    out.push_str(&html_escape_attr(error_kind));
    out.push_str("\" src=\"");
    out.push_str(&html_escape_attr(content_url));
    out.push_str("\" alt=\"");
    out.push_str(&html_escape_attr(alt));
    out.push_str("\">\n");
}

/// HTML-escape text content (`&`, `<`, `>`). Used for element text/innerHTML positions.
fn html_escape_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            other => out.push(other),
        }
    }
    out
}

/// HTML-escape an attribute value (`&`, `<`, `>`, `"`, `'`). Attribute positions additionally
/// escape the quote chars so a value can never break out of a double- or single-quoted attr.
fn html_escape_attr(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rich_editor::document_model::node::{
        BlockNode, Child, HsLinkNode, Mark, NodeKind, TextLeaf,
    };

    fn heading_para_doc() -> BlockNode {
        let mut para = BlockNode::new(NodeKind::Paragraph);
        para.children.push(Child::Text(TextLeaf::new("Some ")));
        para.children
            .push(Child::Text(TextLeaf::with_marks("bold", vec![Mark::Bold])));
        para.children.push(Child::Text(TextLeaf::new(" text")));
        BlockNode::doc(vec![BlockNode::heading(1, "Heading"), para])
    }

    #[test]
    fn plain_text_concatenates_all_text() {
        let doc = heading_para_doc();
        let out = export_document(
            &doc,
            ExportFormat::PlainText,
            "ws",
            "http://127.0.0.1:37501",
            "Doc",
            &AssetByteSource::new(),
        )
        .unwrap();
        assert_eq!(out.as_str(), "Heading\nSome bold text");
        assert_eq!(out.mime, "text/plain;charset=utf-8");
        assert_eq!(out.filename, "doc.txt");
    }

    #[test]
    fn markdown_heading_paragraph_bold_exact() {
        // AC: '# Heading\n\nSome **bold** text\n' — the contract's exact assertion (with the
        // trailing paragraph blank line the walker emits).
        let doc = heading_para_doc();
        let md = export_markdown(&doc);
        assert_eq!(md, "# Heading\n\nSome **bold** text\n\n");
    }

    #[test]
    fn prosemirror_json_envelope_shape() {
        let doc = heading_para_doc();
        let s = export_prosemirror_json(&doc).unwrap();
        let v: JsonValue = serde_json::from_str(&s).unwrap();
        assert_eq!(v["schema_version"], "rich_document_v1");
        assert_eq!(v["projection_disclaimer"], PROJECTION_DISCLAIMER);
        assert_eq!(v["content"]["type"], "doc");
        assert_eq!(v["content"]["content"][0]["type"], "heading");
        assert_eq!(v["content"]["content"][0]["attrs"]["level"], 1);
    }

    #[test]
    fn html_self_contained_plain_paragraph_has_p_tag() {
        let doc = BlockNode::doc(vec![BlockNode::paragraph("hello world")]);
        let out = export_document(
            &doc,
            ExportFormat::HtmlSelfContained,
            "ws",
            "http://127.0.0.1:37501",
            "Doc",
            &AssetByteSource::new(),
        )
        .unwrap();
        let s = out.as_str();
        assert!(s.contains("<!DOCTYPE html>"), "valid HTML5 doctype");
        assert!(
            s.contains("<p>hello world</p>"),
            "paragraph rendered as <p>"
        );
        assert_eq!(out.filename, "doc.html");
    }

    #[test]
    fn script_text_is_escaped_in_html() {
        // XSS-safety: a paragraph containing markup-like text must be entity-escaped, NOT emitted
        // as raw tags (impl note 1).
        let doc = BlockNode::doc(vec![BlockNode::paragraph("<script>alert(1)</script>")]);
        let html = export_html(
            &doc,
            HtmlMediaMode::ReferenceLinked,
            "ws",
            "http://127.0.0.1:37501",
            "T",
        );
        assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
        assert!(!html.contains("<script>"), "raw <script> must never appear");
    }

    fn doc_with_image(asset_id: &str) -> BlockNode {
        // A standalone paragraph holding a single image hsLink atom.
        let mut para = BlockNode::new(NodeKind::Paragraph);
        para.children
            .push(Child::HsLink(HsLinkNode::new("image", asset_id, "pic")));
        BlockNode::doc(vec![para])
    }

    #[test]
    fn html_15mb_image_falls_back_to_reference_with_size_error() {
        // A 15 MB image exceeds the 10 MB per-image cap: it must fall back to a reference link
        // carrying data-hs-export-error="size_exceeded" (AC + RISK-1).
        let doc = doc_with_image("ASSET-1");
        let mut assets = AssetByteSource::new();
        assets.insert(
            "ASSET-1".to_string(),
            ResolvedAsset {
                bytes: vec![0u8; 15 * 1024 * 1024],
                mime: "image/png".to_string(),
            },
        );
        let html = export_html(
            &doc,
            HtmlMediaMode::SelfContained { assets: &assets },
            "ws",
            "http://127.0.0.1:37501",
            "T",
        );
        assert!(html.contains("data-hs-export-error=\"size_exceeded\""));
        assert!(
            html.contains("src=\"http://127.0.0.1:37501/workspaces/ws/assets/ASSET-1/content\""),
            "the fallback points at the backend content URL"
        );
        assert!(
            !html.contains("data:image"),
            "the over-cap image is NOT inlined"
        );
    }

    #[test]
    fn html_total_cap_stops_after_five_of_six_9mb_images() {
        // Red-team total-cap control: 6 x 9 MB images. Per-image (9 MB) is under the 10 MB cap, but
        // the 50 MB total cap means only the first 5 (45 MB) inline; the 6th (would be 54 MB) falls
        // back with data-hs-export-error="total_size_exceeded".
        let mut paras: Vec<BlockNode> = Vec::new();
        let mut assets = AssetByteSource::new();
        for i in 0..6 {
            let id = format!("IMG-{i}");
            let mut para = BlockNode::new(NodeKind::Paragraph);
            para.children
                .push(Child::HsLink(HsLinkNode::new("image", &id, "p")));
            paras.push(para);
            assets.insert(
                id,
                ResolvedAsset {
                    bytes: vec![1u8; 9 * 1024 * 1024],
                    mime: "image/png".to_string(),
                },
            );
        }
        let doc = BlockNode::doc(paras);
        let html = export_html(
            &doc,
            HtmlMediaMode::SelfContained { assets: &assets },
            "ws",
            "http://127.0.0.1:37501",
            "T",
        );
        let inlined = html.matches("src=\"data:image/png;base64,").count();
        let total_err = html
            .matches("data-hs-export-error=\"total_size_exceeded\"")
            .count();
        assert_eq!(
            inlined, 5,
            "exactly 5 of 6 images inline before the 50 MB cap"
        );
        assert_eq!(
            total_err, 1,
            "the 6th image falls back with the total-cap error"
        );
    }

    #[test]
    fn video_is_never_inlined() {
        // A video embed always becomes a reference <video src>, never a data: URL (RISK-1).
        let mut para = BlockNode::new(NodeKind::Paragraph);
        para.children
            .push(Child::HsLink(HsLinkNode::new("video", "VID-1", "clip")));
        let doc = BlockNode::doc(vec![para]);
        let mut assets = AssetByteSource::new();
        // Even if (wrongly) provided bytes, video is never inlined.
        assets.insert(
            "VID-1".to_string(),
            ResolvedAsset {
                bytes: vec![0u8; 10],
                mime: "video/mp4".to_string(),
            },
        );
        let html = export_html(
            &doc,
            HtmlMediaMode::SelfContained { assets: &assets },
            "ws",
            "http://127.0.0.1:37501",
            "T",
        );
        assert!(html.contains(
            "<video controls src=\"http://127.0.0.1:37501/workspaces/ws/assets/VID-1/content\""
        ));
        assert!(
            !html.contains("data:video"),
            "video is never base64-inlined"
        );
    }

    #[test]
    fn markdown_table_with_zero_cols_does_not_panic() {
        // Red-team div-by-zero control: an empty table (a table with one row but no cells) emits
        // the empty-table comment and never divides by the column count.
        let row = BlockNode::new(NodeKind::TableRow);
        let table = BlockNode::with_children(NodeKind::Table, vec![Child::Block(row)]);
        let doc = BlockNode::doc(vec![table]);
        let md = export_markdown(&doc);
        assert!(md.contains("<!-- empty table -->"));
    }

    #[test]
    fn markdown_handles_every_node_kind_without_panic() {
        // A document touching many kinds produces non-empty output and never panics (the
        // all-kinds robustness AC / impl note).
        let mut list = BlockNode::new(NodeKind::BulletList);
        let mut item = BlockNode::new(NodeKind::ListItem);
        item.children
            .push(Child::Block(BlockNode::paragraph("item")));
        list.children.push(Child::Block(item));

        let mut task = BlockNode::new(NodeKind::TaskItem);
        task.attrs
            .insert("checked".to_string(), JsonValue::Bool(true));
        task.children.push(Child::Text(TextLeaf::new("done")));

        let mut code = BlockNode::new(NodeKind::CodeBlock);
        code.attrs.insert(
            "language".to_string(),
            JsonValue::String("rust".to_string()),
        );
        code.children
            .push(Child::Text(TextLeaf::new("fn main(){}")));

        let mut quote = BlockNode::new(NodeKind::Blockquote);
        quote
            .children
            .push(Child::Block(BlockNode::paragraph("quoted")));

        let doc = BlockNode::doc(vec![
            BlockNode::heading(2, "H2"),
            BlockNode::paragraph("para"),
            list,
            code,
            quote,
            BlockNode::new(NodeKind::HorizontalRule),
        ]);
        let md = export_markdown(&doc);
        assert!(!md.is_empty());
        assert!(md.contains("## H2"));
        assert!(md.contains("```rust"));
        assert!(md.contains("> quoted"));
        assert!(md.contains("---"));
        // The standalone task item path (defensive) also runs without panic.
        let task_doc = BlockNode::doc(vec![task]);
        assert!(export_markdown(&task_doc).contains("- [x] done"));
    }

    #[test]
    fn slugify_strips_spaces_and_punctuation() {
        assert_eq!(slugify("My Doc Title!"), "my-doc-title");
        assert_eq!(slugify("   "), "untitled");
        assert_eq!(slugify("a/b\\c"), "a-b-c");
    }
}
