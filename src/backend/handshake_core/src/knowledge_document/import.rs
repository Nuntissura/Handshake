//! MT-151 DocumentProjectionImport.
//!
//! Import markdown / plain-text / HTML snippets into a RichDocument block tree
//! (a ProseMirror doc node JSON), surfacing TYPED unsupported-feature warnings
//! rather than silently dropping content, and turning content that cannot be
//! faithfully represented into a repairable node carrying the original source
//! text. The output is a document-json `Value` ready to seed a new RichDocument
//! plus the list of warnings; nothing here writes to storage.
//!
//! Scope: a deliberately small, deterministic importer that covers the common
//! markdown block constructs (ATX headings, blockquotes, fenced code, bullet/
//! ordered lists, paragraphs). HTML and anything unrecognized is captured as a
//! repairable `importedRaw` node so no input is lost. This is the data plumbing;
//! the rich editing UX is a later group.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// The source format of an import snippet (MT-151).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportFormat {
    Markdown,
    PlainText,
    Html,
}

impl ImportFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Markdown => "markdown",
            Self::PlainText => "plain_text",
            Self::Html => "html",
        }
    }
}

/// A typed warning raised during import (MT-151): an unsupported feature or a
/// fragment captured as a repairable node instead of being dropped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportWarning {
    /// Stable machine code (e.g. `html_captured_as_raw`, `table_not_supported`).
    pub code: String,
    /// Human-readable detail.
    pub detail: String,
}

/// The outcome of an import (MT-151): the produced document JSON plus warnings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportOutcome {
    /// A ProseMirror doc node JSON ready to seed a RichDocument.
    pub document_json: Value,
    pub warnings: Vec<ImportWarning>,
}

/// Import a snippet into a document block tree (MT-151).
pub fn import_snippet(snippet: &str, format: ImportFormat) -> ImportOutcome {
    match format {
        ImportFormat::Markdown => import_markdown(snippet),
        ImportFormat::PlainText => import_plain_text(snippet),
        ImportFormat::Html => import_html(snippet),
    }
}

fn doc(content: Vec<Value>) -> Value {
    json!({ "type": "doc", "content": content })
}

fn paragraph(text: &str) -> Value {
    if text.is_empty() {
        json!({ "type": "paragraph" })
    } else {
        json!({ "type": "paragraph", "content": [{ "type": "text", "text": text }] })
    }
}

fn heading(level: i64, text: &str) -> Value {
    json!({
        "type": "heading",
        "attrs": { "level": level.clamp(1, 6) },
        "content": if text.is_empty() { json!([]) } else { json!([{ "type": "text", "text": text }]) }
    })
}

/// A repairable node that preserves source text we could not faithfully convert
/// (MT-151). The editor renders it as a repairable block; nothing is lost.
fn imported_raw(source_format: &str, text: &str) -> Value {
    json!({
        "type": "importedRaw",
        "attrs": { "source_format": source_format, "repairable": true },
        "content": [{ "type": "text", "text": text }]
    })
}

fn import_plain_text(snippet: &str) -> ImportOutcome {
    let content: Vec<Value> = snippet
        .split("\n\n")
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(|p| paragraph(&p.replace('\n', " ")))
        .collect();
    ImportOutcome {
        document_json: doc(if content.is_empty() {
            vec![paragraph("")]
        } else {
            content
        }),
        warnings: Vec::new(),
    }
}

fn import_html(snippet: &str) -> ImportOutcome {
    // A faithful HTML parser is the rich editing UX group's job; here we
    // capture the HTML as a single repairable node so the content is never lost
    // and the operator can repair it later.
    ImportOutcome {
        document_json: doc(vec![imported_raw("html", snippet.trim())]),
        warnings: vec![ImportWarning {
            code: "html_captured_as_raw".to_string(),
            detail: "HTML import is captured as a repairable importedRaw node; structured HTML conversion is handled by the rich editor group.".to_string(),
        }],
    }
}

fn import_markdown(snippet: &str) -> ImportOutcome {
    let mut content: Vec<Value> = Vec::new();
    let mut warnings: Vec<ImportWarning> = Vec::new();
    let lines: Vec<&str> = snippet.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim_end();

        // Blank line: skip.
        if trimmed.trim().is_empty() {
            i += 1;
            continue;
        }

        // Fenced code block.
        if let Some(fence) = trimmed.trim_start().strip_prefix("```") {
            let _lang = fence.trim();
            let mut code_lines: Vec<&str> = Vec::new();
            i += 1;
            let mut closed = false;
            while i < lines.len() {
                if lines[i].trim_start().starts_with("```") {
                    closed = true;
                    i += 1;
                    break;
                }
                code_lines.push(lines[i]);
                i += 1;
            }
            let code = code_lines.join("\n");
            content.push(json!({
                "type": "codeBlock",
                "content": if code.is_empty() { json!([]) } else { json!([{ "type": "text", "text": code }]) }
            }));
            if !closed {
                warnings.push(ImportWarning {
                    code: "unterminated_code_fence".to_string(),
                    detail: "A fenced code block was not closed; imported through end of input."
                        .to_string(),
                });
            }
            continue;
        }

        // ATX heading.
        if let Some(level) = atx_level(trimmed) {
            let text = trimmed.trim_start_matches('#').trim();
            content.push(heading(level, text));
            i += 1;
            continue;
        }

        // Blockquote (one or more consecutive `>` lines).
        if trimmed.trim_start().starts_with('>') {
            let mut quote_lines: Vec<String> = Vec::new();
            while i < lines.len() && lines[i].trim_start().starts_with('>') {
                let q = lines[i]
                    .trim_start()
                    .trim_start_matches('>')
                    .trim_start()
                    .to_string();
                quote_lines.push(q);
                i += 1;
            }
            content.push(json!({
                "type": "blockquote",
                "content": [paragraph(&quote_lines.join(" "))]
            }));
            continue;
        }

        // Bullet list.
        if is_bullet(trimmed) {
            let mut items: Vec<Value> = Vec::new();
            while i < lines.len() && is_bullet(lines[i].trim_end()) {
                let text = strip_bullet(lines[i].trim_end());
                items.push(json!({ "type": "listItem", "content": [paragraph(&text)] }));
                i += 1;
            }
            content.push(json!({ "type": "bulletList", "content": items }));
            continue;
        }

        // Ordered list.
        if is_ordered(trimmed) {
            let mut items: Vec<Value> = Vec::new();
            while i < lines.len() && is_ordered(lines[i].trim_end()) {
                let text = strip_ordered(lines[i].trim_end());
                items.push(json!({ "type": "listItem", "content": [paragraph(&text)] }));
                i += 1;
            }
            content.push(json!({ "type": "orderedList", "content": items }));
            continue;
        }

        // Markdown table (header row with pipes followed by a separator row):
        // not faithfully supported here — capture as repairable raw.
        if trimmed.contains('|') && i + 1 < lines.len() && is_table_separator(lines[i + 1]) {
            let mut table_lines: Vec<&str> = Vec::new();
            while i < lines.len() && lines[i].contains('|') {
                table_lines.push(lines[i]);
                i += 1;
            }
            content.push(imported_raw("markdown_table", &table_lines.join("\n")));
            warnings.push(ImportWarning {
                code: "table_captured_as_raw".to_string(),
                detail: "Markdown tables are captured as a repairable importedRaw node; structured table import is handled by the rich editor group.".to_string(),
            });
            continue;
        }

        // Otherwise: a paragraph that runs to the next blank line.
        let mut para_lines: Vec<&str> = Vec::new();
        while i < lines.len() && !lines[i].trim().is_empty() && !starts_block(lines[i].trim_end()) {
            para_lines.push(lines[i].trim());
            i += 1;
        }
        if para_lines.is_empty() {
            // Defensive: avoid an infinite loop if a line both is non-blank and
            // starts a block we didn't consume above.
            para_lines.push(lines[i].trim());
            i += 1;
        }
        content.push(paragraph(&para_lines.join(" ")));
    }

    if content.is_empty() {
        content.push(paragraph(""));
    }
    ImportOutcome {
        document_json: doc(content),
        warnings,
    }
}

fn atx_level(line: &str) -> Option<i64> {
    let t = line.trim_start();
    if !t.starts_with('#') {
        return None;
    }
    let hashes = t.chars().take_while(|c| *c == '#').count();
    if (1..=6).contains(&hashes) && t.chars().nth(hashes) == Some(' ') {
        Some(hashes as i64)
    } else {
        None
    }
}

fn is_bullet(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("- ") || t.starts_with("* ") || t.starts_with("+ ")
}

fn strip_bullet(line: &str) -> String {
    let t = line.trim_start();
    t[2..].trim().to_string()
}

fn is_ordered(line: &str) -> bool {
    let t = line.trim_start();
    let digits: String = t.chars().take_while(|c| c.is_ascii_digit()).collect();
    !digits.is_empty() && t[digits.len()..].starts_with(". ")
}

fn strip_ordered(line: &str) -> String {
    let t = line.trim_start();
    let digits = t.chars().take_while(|c| c.is_ascii_digit()).count();
    t[digits + 2..].trim().to_string()
}

fn is_table_separator(line: &str) -> bool {
    let t = line.trim();
    !t.is_empty()
        && t.chars().all(|c| matches!(c, '|' | '-' | ':' | ' '))
        && t.contains('-')
        && t.contains('|')
}

fn starts_block(line: &str) -> bool {
    atx_level(line).is_some()
        || line.trim_start().starts_with('>')
        || line.trim_start().starts_with("```")
        || is_bullet(line)
        || is_ordered(line)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge_document::block_tree::{BlockKind, BlockTree, DOCUMENT_SCHEMA_VERSION};

    fn kinds(doc: &Value) -> Vec<BlockKind> {
        BlockTree::from_document_json("KRD-x", DOCUMENT_SCHEMA_VERSION, doc)
            .unwrap()
            .blocks
            .iter()
            .map(|b| b.kind)
            .collect()
    }

    #[test]
    fn markdown_import_recognizes_block_constructs() {
        let snippet =
            "# H1\n\nA paragraph\nspanning two lines.\n\n> a quote\n\n- a\n- b\n\n1. one\n2. two\n\n```\ncode();\n```";
        let outcome = import_snippet(snippet, ImportFormat::Markdown);
        assert!(outcome.warnings.is_empty());
        let k = kinds(&outcome.document_json);
        assert!(k.contains(&BlockKind::Heading));
        assert!(k.contains(&BlockKind::Blockquote));
        assert!(k.contains(&BlockKind::BulletList));
        assert!(k.contains(&BlockKind::OrderedList));
        assert!(k.contains(&BlockKind::CodeBlock));
        // The parsed doc is itself a valid block tree (no lost content).
        assert!(BlockTree::from_document_json(
            "KRD-x",
            DOCUMENT_SCHEMA_VERSION,
            &outcome.document_json
        )
        .is_ok());
    }

    #[test]
    fn html_is_captured_as_repairable_raw_node_with_warning() {
        let outcome = import_snippet("<b>hi</b>", ImportFormat::Html);
        assert_eq!(outcome.warnings.len(), 1);
        assert_eq!(outcome.warnings[0].code, "html_captured_as_raw");
        assert_eq!(
            outcome.document_json["content"][0]["type"],
            serde_json::json!("importedRaw")
        );
        assert_eq!(
            outcome.document_json["content"][0]["attrs"]["repairable"],
            serde_json::json!(true)
        );
        // Adversarial-v2 MT-151: the imported document must be LOADABLE as a
        // typed block tree (the review found importedRaw made it 400).
        let tree = BlockTree::from_document_json(
            "KRD-x",
            DOCUMENT_SCHEMA_VERSION,
            &outcome.document_json,
        )
        .expect("imported HTML document parses as a block tree");
        assert_eq!(tree.blocks[0].kind, BlockKind::ImportedRaw);
        assert_eq!(
            tree.to_document_json(),
            outcome.document_json,
            "import -> load -> save round-trip is lossless"
        );
    }

    #[test]
    fn markdown_table_is_captured_as_repairable_raw_with_warning() {
        let snippet = "| a | b |\n| - | - |\n| 1 | 2 |";
        let outcome = import_snippet(snippet, ImportFormat::Markdown);
        assert!(outcome
            .warnings
            .iter()
            .any(|w| w.code == "table_captured_as_raw"));
        assert_eq!(
            outcome.document_json["content"][0]["type"],
            serde_json::json!("importedRaw")
        );
        // Adversarial-v2 MT-151: table imports load + round-trip too.
        let tree = BlockTree::from_document_json(
            "KRD-x",
            DOCUMENT_SCHEMA_VERSION,
            &outcome.document_json,
        )
        .expect("imported markdown-table document parses as a block tree");
        assert_eq!(tree.blocks[0].kind, BlockKind::ImportedRaw);
        assert!(tree.blocks[0].content.derived.plain_text.contains("| a | b |"));
        assert_eq!(tree.to_document_json(), outcome.document_json);
    }

    #[test]
    fn plain_text_splits_paragraphs() {
        let outcome = import_snippet("para one\n\npara two", ImportFormat::PlainText);
        assert_eq!(
            kinds(&outcome.document_json),
            vec![BlockKind::Paragraph, BlockKind::Paragraph]
        );
    }

    #[test]
    fn empty_input_yields_an_empty_paragraph_doc() {
        let outcome = import_snippet("", ImportFormat::Markdown);
        assert_eq!(kinds(&outcome.document_json), vec![BlockKind::Paragraph]);
    }
}
