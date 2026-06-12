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
use super::embed::{url_scheme, EmbedTarget};

/// URL schemes a projection may emit into a link target (adversarial-v2
/// MT-150 hardening): web links, mail links, and internal `hsk:` refs.
/// Everything else — `javascript:`, `data:`, `vbscript:`, `file:` — is
/// neutralized. Scheme detection runs on the obfuscation-stripped value
/// (see [`url_scheme`]) so case/tab/newline tricks cannot smuggle a scheme.
const SAFE_LINK_SCHEMES: [&str; 4] = ["http", "https", "mailto", "hsk"];

/// Sanitize a link target for rendering (MT-150): returns the target when it
/// is scheme-less (relative/internal ref) or carries an allowlisted scheme,
/// else `None` (the renderer neutralizes the link instead of emitting it).
fn sanitize_link_target(target: &str) -> Option<&str> {
    match url_scheme(target) {
        None => Some(target),
        Some(scheme) if SAFE_LINK_SCHEMES.contains(&scheme.as_str()) => Some(target),
        Some(_) => None,
    }
}

/// Resolve a block's embed target through the typed [`EmbedTarget`] law
/// (MT-150/152 hardening: projections no longer trust raw `attrs` — the same
/// validation that guards the side table governs what renders). Returns the
/// validated value, or `None` when the target is absent or fails validation
/// (absolute path, non-http URL, scheme-bearing pseudo-id).
fn validated_embed_target(block: &Block) -> Option<String> {
    let raw = embed_target(block)?;
    EmbedTarget::parse_raw(&raw).ok().map(|target| target.value)
}

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
        BlockKind::ImportedRaw => {
            // MT-151: a repairable imported node renders as a fenced block so
            // its captured source text stays INERT in markdown renderers
            // (raw HTML inside markdown would otherwise execute as markup).
            let lang = imported_raw_source_format(block);
            format!("```{lang}\n{text}\n```")
        }
        kind if kind.is_typed_link() => {
            let target = typed_link_target(block).unwrap_or_default();
            if wiki {
                format!("[[{target}]]")
            } else {
                let label = if text.is_empty() { &target } else { text };
                match sanitize_link_target(&target) {
                    // MT-150: only allowlisted schemes become markdown links;
                    // a javascript:/data: target degrades to the plain label.
                    Some(safe) => format!("[{label}]({safe})"),
                    None => label.clone(),
                }
            }
        }
        kind if kind.is_embed() => {
            let alt = if text.is_empty() { "embed" } else { text };
            match validated_embed_target(block) {
                // MT-150/152: embed targets render only through the typed
                // EmbedTarget law; an invalid target degrades to the alt text.
                Some(target) => format!("![{alt}]({target})"),
                None => alt.to_string(),
            }
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
        BlockKind::ImportedRaw => {
            // MT-151: the captured source renders escaped inside a pre/code
            // block tagged as repairable — visible, inert, never executed.
            let lang = escape_html(&imported_raw_source_format(block));
            format!("<pre data-hsk-imported-raw=\"{lang}\"><code>{text}</code></pre>")
        }
        kind if kind.is_typed_link() => {
            let raw_target = typed_link_target(block).unwrap_or_default();
            // MT-150 (adversarial-v2): only allowlisted schemes may become an
            // href. A javascript:/data:/vbscript: target — including case,
            // tab/newline, and entity obfuscations — is neutralized into a
            // non-link span carrying the label only (stored XSS closed).
            match sanitize_link_target(&raw_target) {
                Some(safe) => {
                    let target = escape_html(safe);
                    let label = if text.is_empty() {
                        target.clone()
                    } else {
                        text
                    };
                    format!("<a href=\"{target}\">{label}</a>")
                }
                None => {
                    let label = if text.is_empty() {
                        escape_html("blocked link")
                    } else {
                        text
                    };
                    format!("<span data-hsk-blocked-link=\"true\">{label}</span>")
                }
            }
        }
        kind if kind.is_embed() => {
            // MT-150/152 (adversarial-v2): embeds render only through the
            // typed EmbedTarget law — never raw attrs. An invalid target
            // renders as a repairable placeholder span, not a live src.
            match validated_embed_target(block) {
                Some(valid) => {
                    let target = escape_html(&valid);
                    format!("<img src=\"{target}\" alt=\"{text}\" />")
                }
                None => {
                    let alt = if text.is_empty() {
                        escape_html("broken embed")
                    } else {
                        text
                    };
                    format!("<span data-hsk-broken-embed=\"true\">{alt}</span>")
                }
            }
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

/// The `attrs.source_format` of an importedRaw block (MT-151), used to tag the
/// rendered fence/pre so the original format stays visible for repair.
fn imported_raw_source_format(block: &Block) -> String {
    block
        .content
        .raw
        .as_object()
        .and_then(|o| o.get("attrs"))
        .and_then(|a| a.as_object())
        .and_then(|a| a.get("source_format"))
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_string()
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

    #[test]
    fn imported_raw_blocks_render_inert_in_markdown_and_html() {
        // Adversarial-v2 MT-151: importedRaw blocks must render (the review
        // found they 400'd) and the captured source must stay inert.
        let doc = json!({
            "type": "doc",
            "content": [
                { "type": "importedRaw",
                  "attrs": { "source_format": "html", "repairable": true },
                  "content": [{ "type": "text", "text": "<script>alert(1)</script>" }] }
            ]
        });
        let tree = BlockTree::from_document_json("KRD-x", DOCUMENT_SCHEMA_VERSION, &doc).unwrap();
        let md = render_projection("T", &tree, ProjectionFormat::Markdown);
        assert!(
            md.content.contains("```html\n<script>alert(1)</script>\n```"),
            "markdown renders the source fenced (inert): {}",
            md.content
        );
        let html = render_projection("T", &tree, ProjectionFormat::Html);
        assert!(html.content.contains("data-hsk-imported-raw=\"html\""));
        assert!(
            html.content.contains("&lt;script&gt;"),
            "html renders the source escaped: {}",
            html.content
        );
        assert!(!html.content.contains("<script>"));
    }

    // -- adversarial-v2 MT-150: stored-XSS scheme allowlist -------------------

    fn link_doc(target: &str) -> BlockTree {
        let doc = json!({
            "type": "doc",
            "content": [
                { "type": "fileLink", "attrs": { "target": target },
                  "content": [{ "type": "text", "text": "click me" }] }
            ]
        });
        BlockTree::from_document_json("KRD-x", DOCUMENT_SCHEMA_VERSION, &doc).unwrap()
    }

    fn embed_doc(target: &str) -> BlockTree {
        let doc = json!({
            "type": "doc",
            "content": [
                { "type": "image", "attrs": { "target": target },
                  "content": [{ "type": "text", "text": "pic" }] }
            ]
        });
        BlockTree::from_document_json("KRD-x", DOCUMENT_SCHEMA_VERSION, &doc).unwrap()
    }

    #[test]
    fn html_projection_neutralizes_dangerous_href_schemes() {
        // Plain and OBFUSCATED payloads: case tricks, embedded tab/newline/CR
        // (browsers strip these in URLs), leading whitespace, and friends.
        for payload in [
            "javascript:alert(1)",
            "JaVaScRiPt:alert(1)",
            "jav\tascript:alert(1)",
            "jav\nascript:alert(1)",
            "jav\rascript:alert(1)",
            "  javascript:alert(1)",
            "\u{1}javascript:alert(1)",
            "vbscript:msgbox(1)",
            "VBScript:msgbox(1)",
            "data:text/html,<script>alert(1)</script>",
            "DATA:text/html;base64,PHNjcmlwdD4=",
            "file:///etc/passwd",
        ] {
            let html = render_projection("T", &link_doc(payload), ProjectionFormat::Html);
            let lower = html.content.to_lowercase().replace(['\t', '\n', '\r'], "");
            assert!(
                !lower.contains("href=\"javascript")
                    && !lower.contains("href=\"vbscript")
                    && !lower.contains("href=\"data:")
                    && !lower.contains("href=\"file:"),
                "payload `{payload}` must never become an href: {}",
                html.content
            );
            assert!(
                html.content.contains("data-hsk-blocked-link=\"true\""),
                "payload `{payload}` must render as a neutralized non-link"
            );
            assert!(
                html.content.contains("click me"),
                "the label text survives neutralization"
            );
        }
    }

    #[test]
    fn html_projection_keeps_allowlisted_and_relative_link_targets() {
        for safe in [
            "https://example.com/page",
            "http://example.com",
            "mailto:ops@example.com",
            "hsk:spec/2.3.13.11",
            "WP-KERNEL-009",
            "docs/runbook.md",
            "src/main.rs",
        ] {
            let html = render_projection("T", &link_doc(safe), ProjectionFormat::Html);
            assert!(
                html.content.contains("<a href=\""),
                "safe target `{safe}` must stay a link: {}",
                html.content
            );
            assert!(!html.content.contains("data-hsk-blocked-link"));
        }
    }

    #[test]
    fn entity_encoded_scheme_stays_inert_through_escaping() {
        // `&#106;avascript:` carries no real scheme (the `&` breaks scheme
        // parsing) and the ampersand is escaped to `&amp;#106;` so a browser
        // decodes it back to LITERAL text `&#106;...` — an inert relative URL,
        // never a decoded javascript: scheme.
        let html = render_projection(
            "T",
            &link_doc("&#106;avascript:alert(1)"),
            ProjectionFormat::Html,
        );
        assert!(
            html.content.contains("href=\"&amp;#106;avascript:alert(1)\""),
            "entity payload must be emitted double-escaped: {}",
            html.content
        );
        assert!(!html.content.contains("href=\"&#106;"));
    }

    #[test]
    fn html_projection_routes_embeds_through_embed_target_validation() {
        // Valid typed targets render as <img src>.
        for valid in ["KMED-abc123", "https://cdn.example/x.png"] {
            let html = render_projection("T", &embed_doc(valid), ProjectionFormat::Html);
            assert!(
                html.content.contains("<img src=\""),
                "valid embed `{valid}` renders: {}",
                html.content
            );
        }
        // Invalid targets (the MT-152 bypass the review found) are neutralized:
        // data:/javascript: URIs, absolute paths, UNC, file:.
        for invalid in [
            "data:image/svg+xml,<svg onload=alert(1)>",
            "javascript:alert(1)",
            "JaVa\tScRiPt:alert(1)",
            "/var/secrets/x.png",
            "C:\\secrets\\x.png",
            "\\\\host\\share\\x.png",
            "file:///etc/passwd",
            "ftp://host/x.png",
        ] {
            let html = render_projection("T", &embed_doc(invalid), ProjectionFormat::Html);
            assert!(
                !html.content.contains("<img"),
                "invalid embed `{invalid}` must not render an img: {}",
                html.content
            );
            assert!(
                html.content.contains("data-hsk-broken-embed=\"true\""),
                "invalid embed `{invalid}` renders the repairable placeholder"
            );
        }
    }

    #[test]
    fn markdown_projection_neutralizes_dangerous_targets_too() {
        let md = render_projection(
            "T",
            &link_doc("javascript:alert(1)"),
            ProjectionFormat::Markdown,
        );
        assert!(
            !md.content.contains("](javascript:"),
            "markdown must not emit a javascript: link: {}",
            md.content
        );
        assert!(md.content.contains("click me"), "label text survives");
        let md = render_projection(
            "T",
            &embed_doc("data:text/html,<script>"),
            ProjectionFormat::Markdown,
        );
        assert!(!md.content.contains("](data:"));
        // Safe targets still render as links/embeds in markdown.
        let md = render_projection(
            "T",
            &link_doc("https://example.com"),
            ProjectionFormat::Markdown,
        );
        assert!(md.content.contains("[click me](https://example.com)"));
        let md = render_projection("T", &embed_doc("KMED-1"), ProjectionFormat::Markdown);
        assert!(md.content.contains("![pic](KMED-1)"));
    }
}
