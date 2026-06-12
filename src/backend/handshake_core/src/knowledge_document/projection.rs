//! MT-150 DocumentProjectionExport.
//!
//! Deterministic projection RENDERERS that derive markdown / HTML / plain text /
//! wiki-Loom / context-bundle views FROM the canonical block tree. Projections
//! are regenerable and NEVER authority (spec 2.3.13.11): the same block tree
//! always renders the same bytes, and a projection can be thrown away and
//! rebuilt. The persistence of a projection row (through
//! `knowledge_wiki_projections`) and the negative proof that deleting it never
//! mutates authority live in the storage/api layer; this module is the pure,
//! deterministic content renderer those use.

use serde::{Deserialize, Serialize};

use super::block_tree::{Block, BlockKind, BlockTree};

/// Supported projection output formats (MT-150).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionFormat {
    Markdown,
    Html,
    PlainText,
    /// Wiki / Loom view (markdown-with-wikilinks flavor).
    WikiLoom,
    /// A compact context-bundle text view (title + plain text blocks).
    ContextBundle,
}

impl ProjectionFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Markdown => "markdown",
            Self::Html => "html",
            Self::PlainText => "plain_text",
            Self::WikiLoom => "wiki_loom",
            Self::ContextBundle => "context_bundle",
        }
    }
}

/// A rendered projection of a document (MT-150). Regenerable; never authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderedProjection {
    pub format: ProjectionFormat,
    pub content: String,
}

/// Render a document title + block tree into the requested projection format
/// (MT-150). Deterministic: identical inputs always yield identical output.
pub fn render_projection(
    title: &str,
    tree: &BlockTree,
    format: ProjectionFormat,
) -> RenderedProjection {
    let content = match format {
        ProjectionFormat::Markdown => render_markdown(title, tree, false),
        ProjectionFormat::WikiLoom => render_markdown(title, tree, true),
        ProjectionFormat::Html => render_html(title, tree),
        ProjectionFormat::PlainText => render_plain_text(title, tree),
        ProjectionFormat::ContextBundle => render_context_bundle(title, tree),
    };
    RenderedProjection { format, content }
}

fn render_markdown(title: &str, tree: &BlockTree, wiki: bool) -> String {
    let mut out = String::new();
    out.push_str("# ");
    out.push_str(title);
    out.push_str("\n\n");
    for block in &tree.blocks {
        out.push_str(&markdown_block(block, wiki));
        out.push_str("\n\n");
    }
    out.trim_end().to_string()
}

fn markdown_block(block: &Block, wiki: bool) -> String {
    let text = &block.content.derived.plain_text;
    match block.kind {
        BlockKind::Heading => {
            let level = block.heading_level.unwrap_or(1).clamp(1, 6) as usize;
            format!("{} {}", "#".repeat(level), text)
        }
        BlockKind::Blockquote => text
            .lines()
            .map(|line| format!("> {line}"))
            .collect::<Vec<_>>()
            .join("\n"),
        BlockKind::CodeBlock => format!("```\n{text}\n```"),
        BlockKind::BulletList => text
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|line| format!("- {}", line.trim()))
            .collect::<Vec<_>>()
            .join("\n"),
        BlockKind::OrderedList => text
            .lines()
            .filter(|l| !l.trim().is_empty())
            .enumerate()
            .map(|(i, line)| format!("{}. {}", i + 1, line.trim()))
            .collect::<Vec<_>>()
            .join("\n"),
        BlockKind::TaskList => text
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|line| format!("- [ ] {}", line.trim()))
            .collect::<Vec<_>>()
            .join("\n"),
        kind if kind.is_typed_link() => {
            let target = typed_link_target(block).unwrap_or_default();
            if wiki {
                format!("[[{target}]]")
            } else {
                let label = if text.is_empty() { &target } else { text };
                format!("[{label}]({target})")
            }
        }
        kind if kind.is_embed() => {
            let target = embed_target(block).unwrap_or_default();
            let alt = if text.is_empty() { "embed" } else { text };
            format!("![{alt}]({target})")
        }
        _ => text.clone(),
    }
}

fn render_html(title: &str, tree: &BlockTree) -> String {
    let mut out = String::new();
    out.push_str("<h1>");
    out.push_str(&escape_html(title));
    out.push_str("</h1>");
    for block in &tree.blocks {
        out.push_str(&html_block(block));
    }
    out
}

fn html_block(block: &Block) -> String {
    let text = escape_html(&block.content.derived.plain_text);
    match block.kind {
        BlockKind::Heading => {
            let level = block.heading_level.unwrap_or(1).clamp(1, 6);
            format!("<h{level}>{text}</h{level}>")
        }
        BlockKind::Blockquote => format!("<blockquote>{text}</blockquote>"),
        BlockKind::CodeBlock => format!("<pre><code>{text}</code></pre>"),
        BlockKind::BulletList | BlockKind::TaskList => {
            list_html("ul", &block.content.derived.plain_text)
        }
        BlockKind::OrderedList => list_html("ol", &block.content.derived.plain_text),
        kind if kind.is_typed_link() => {
            let target = escape_html(&typed_link_target(block).unwrap_or_default());
            let label = if text.is_empty() {
                target.clone()
            } else {
                text
            };
            format!("<a href=\"{target}\">{label}</a>")
        }
        kind if kind.is_embed() => {
            let target = escape_html(&embed_target(block).unwrap_or_default());
            format!("<img src=\"{target}\" alt=\"{text}\" />")
        }
        _ => format!("<p>{text}</p>"),
    }
}

fn list_html(tag: &str, text: &str) -> String {
    let items: String = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|line| format!("<li>{}</li>", escape_html(line.trim())))
        .collect();
    format!("<{tag}>{items}</{tag}>")
}

fn render_plain_text(title: &str, tree: &BlockTree) -> String {
    let mut out = String::from(title);
    out.push_str("\n\n");
    out.push_str(&tree.plain_text());
    out.trim_end().to_string()
}

fn render_context_bundle(title: &str, tree: &BlockTree) -> String {
    // A compact view for context bundles: the title, then each non-empty
    // block's preview line, so a retrieval bundle gets a stable digest of the
    // document without the full body.
    let mut out = format!("DOCUMENT: {title}\n");
    for block in &tree.blocks {
        let preview = &block.content.derived.preview;
        if !preview.is_empty() {
            out.push_str(&format!("- [{}] {}\n", block.kind.as_node_type(), preview));
        }
    }
    out.trim_end().to_string()
}

fn typed_link_target(block: &Block) -> Option<String> {
    block
        .content
        .raw
        .as_object()
        .and_then(|o| o.get("attrs"))
        .and_then(|a| a.as_object())
        .and_then(|a| a.get("target"))
        .and_then(|t| t.as_str())
        .map(ToOwned::to_owned)
}

fn embed_target(block: &Block) -> Option<String> {
    let attrs = block
        .content
        .raw
        .as_object()
        .and_then(|o| o.get("attrs"))
        .and_then(|a| a.as_object())?;
    // Embeds carry their typed target under attrs.target (id or url) or
    // attrs.src (legacy/url). Prefer target.
    attrs
        .get("target")
        .and_then(|t| t.as_str())
        .or_else(|| attrs.get("src").and_then(|t| t.as_str()))
        .map(ToOwned::to_owned)
}

fn escape_html(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge_document::block_tree::DOCUMENT_SCHEMA_VERSION;
    use serde_json::json;

    fn tree() -> BlockTree {
        let doc = json!({
            "type": "doc",
            "content": [
                { "type": "heading", "attrs": { "level": 2 }, "content": [{ "type": "text", "text": "Heading" }] },
                { "type": "paragraph", "content": [{ "type": "text", "text": "Body text." }] },
                { "type": "codeBlock", "content": [{ "type": "text", "text": "let x = 1;" }] },
                { "type": "wpLink", "attrs": { "target": "WP-1" }, "content": [{ "type": "text", "text": "wp" }] }
            ]
        });
        BlockTree::from_document_json("KRD-x", DOCUMENT_SCHEMA_VERSION, &doc).unwrap()
    }

    #[test]
    fn markdown_projection_is_deterministic_and_structured() {
        let a = render_projection("Doc", &tree(), ProjectionFormat::Markdown);
        let b = render_projection("Doc", &tree(), ProjectionFormat::Markdown);
        assert_eq!(a.content, b.content, "deterministic");
        assert!(a.content.starts_with("# Doc"));
        assert!(a.content.contains("## Heading"));
        assert!(a.content.contains("```\nlet x = 1;\n```"));
        assert!(a.content.contains("[wp](WP-1)"));
    }

    #[test]
    fn html_projection_escapes_and_structures() {
        let doc = json!({
            "type": "doc",
            "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "a < b & c" }] }]
        });
        let tree = BlockTree::from_document_json("KRD-x", DOCUMENT_SCHEMA_VERSION, &doc).unwrap();
        let html = render_projection("T", &tree, ProjectionFormat::Html);
        assert!(html.content.contains("<h1>T</h1>"));
        assert!(html.content.contains("a &lt; b &amp; c"));
    }

    #[test]
    fn wiki_loom_renders_typed_links_as_wikilinks() {
        let wiki = render_projection("Doc", &tree(), ProjectionFormat::WikiLoom);
        assert!(wiki.content.contains("[[WP-1]]"));
    }

    #[test]
    fn all_formats_render_without_panicking() {
        for format in [
            ProjectionFormat::Markdown,
            ProjectionFormat::Html,
            ProjectionFormat::PlainText,
            ProjectionFormat::WikiLoom,
            ProjectionFormat::ContextBundle,
        ] {
            let rendered = render_projection("Doc", &tree(), format);
            assert!(!rendered.content.is_empty());
        }
    }
}
