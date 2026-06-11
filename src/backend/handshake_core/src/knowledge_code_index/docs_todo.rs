//! WP-KERNEL-009 MT-103 DocCommentTodoExtractor.
//!
//! Master Spec anchor: 2.3.13.11 KnowledgeClaim ("an assertion about a source,
//! product behavior, task, or operator workflow") + KnowledgeSpan. Extracts
//! doc comments, TODO/FIXME/HACK/SAFETY markers, and operator-facing strings as
//! labeled, searchable passages with source spans.
//!
//! Pure data; no DB. The engine maps each [`DocPassage`] to a `text`-kind span
//! and (for markers) a `KnowledgeClaim`, so an agent can search "what TODOs
//! touch this file" or "what does the doc comment on this symbol say".
//!
//! Strategy: a line scanner over the raw source (language-agnostic for the
//! comment syntaxes Rust/TS/JS share). It recognises:
//! * Rust doc comments: `///` (outer) and `//!` (inner), plus `/** ... */`.
//! * Line comments `//` and block comments `/* ... */` that contain a marker
//!   keyword.
//!
//! Markers are case-sensitive keywords at a word boundary: TODO, FIXME, HACK,
//! XXX, SAFETY, NOTE, BUG, OPTIMIZE. Consecutive doc-comment lines are merged
//! into one passage.

use serde::{Deserialize, Serialize};

/// The kind of an extracted passage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocPassageKind {
    /// A documentation comment (`///`, `//!`, `/** */`).
    DocComment,
    /// A TODO/FIXME/HACK/XXX/BUG/OPTIMIZE actionable marker.
    Todo,
    /// A SAFETY/NOTE annotation.
    Safety,
}

impl DocPassageKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::DocComment => "doc_comment",
            Self::Todo => "todo",
            Self::Safety => "safety",
        }
    }
}

/// One extracted passage (merged doc block or a single marker line).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocPassage {
    pub kind: DocPassageKind,
    /// For markers, the keyword (e.g. `TODO`); empty for doc comments.
    pub marker: Option<String>,
    /// The passage text (comment body, markers/leading slashes stripped).
    pub text: String,
    /// 1-based inclusive line range.
    pub start_line: u32,
    pub end_line: u32,
    /// Byte range `[start, end)` covering the passage lines.
    pub byte_start: usize,
    pub byte_end: usize,
}

impl DocPassage {
    /// Stable-ish key for the passage entity/claim (path + start line + kind).
    /// Doc passages are not symbols; the key includes the line so re-runs of
    /// unchanged files reuse it, but an edit that shifts lines makes a new key
    /// (and the old one goes stale), which is the intended behavior.
    pub fn entity_key(&self, relative_path: &str) -> String {
        format!(
            "doc:{relative_path}#L{}:{}",
            self.start_line,
            self.kind.as_str()
        )
    }
}

const MARKERS: &[&str] = &[
    "TODO", "FIXME", "HACK", "XXX", "BUG", "OPTIMIZE", "SAFETY", "NOTE",
];

/// Extract doc/TODO passages from raw source text. Language-agnostic over the
/// C-family comment syntaxes used by Rust/TS/JS. Deterministic, document order.
pub fn extract_doc_passages(text: &str) -> Vec<DocPassage> {
    let offsets = line_offsets(text);
    let lines: Vec<&str> = text.split('\n').collect();
    let mut out: Vec<DocPassage> = Vec::new();

    let mut i = 0usize;
    let mut in_block = false;
    let mut block_start_line = 0usize;
    let mut block_text = String::new();
    let mut block_is_doc = false;

    while i < lines.len() {
        let raw = lines[i];
        let trimmed = raw.trim_start();

        if in_block {
            block_text.push('\n');
            let body = strip_block_inner(raw);
            block_text.push_str(&body);
            if raw.contains("*/") {
                in_block = false;
                push_block(
                    &mut out,
                    &offsets,
                    text,
                    block_is_doc,
                    block_start_line,
                    i,
                    std::mem::take(&mut block_text),
                );
            }
            i += 1;
            continue;
        }

        // Block comment start.
        if let Some(pos) = trimmed.find("/*") {
            let is_doc = trimmed[pos..].starts_with("/**");
            let one_line_end = trimmed.find("*/");
            if let Some(end) = one_line_end {
                // Single-line block comment.
                let inner = &trimmed[pos + 2..end];
                let inner = inner.trim_start_matches('*').trim();
                record_text_passage(&mut out, &offsets, text, is_doc, i, i, inner);
                i += 1;
                continue;
            } else {
                in_block = true;
                block_is_doc = is_doc;
                block_start_line = i;
                block_text = strip_block_inner(&trimmed[pos..]);
                i += 1;
                continue;
            }
        }

        // Rust doc comments (/// outer, //! inner): merge consecutive lines.
        if trimmed.starts_with("///") || trimmed.starts_with("//!") {
            let start = i;
            let mut body = String::new();
            while i < lines.len() {
                let t = lines[i].trim_start();
                if t.starts_with("///") || t.starts_with("//!") {
                    let content = t
                        .trim_start_matches('/')
                        .strip_prefix(' ')
                        .unwrap_or_else(|| t.trim_start_matches('/'));
                    if !body.is_empty() {
                        body.push('\n');
                    }
                    body.push_str(content);
                    i += 1;
                } else {
                    break;
                }
            }
            record_text_passage(&mut out, &offsets, text, true, start, i - 1, body.trim());
            continue;
        }

        // Plain line comment: only emit if it carries a marker.
        if let Some(pos) = trimmed.find("//") {
            let comment = &trimmed[pos + 2..];
            if let Some((marker, _)) = find_marker(comment) {
                let kind = marker_kind(marker);
                record_marker(&mut out, &offsets, text, kind, marker, comment.trim(), i, i);
            }
            i += 1;
            continue;
        }

        i += 1;
    }

    out
}

/// Strip a leading `/**`, `/*`, and a leading ` * ` from a block-comment line.
fn strip_block_inner(line: &str) -> String {
    let t = line.trim();
    let t = t.strip_prefix("/**").unwrap_or(t);
    let t = t.strip_prefix("/*").unwrap_or(t);
    let t = t.trim_start();
    let t = t
        .strip_prefix("* ")
        .or_else(|| t.strip_prefix('*'))
        .unwrap_or(t);
    let t = t.strip_suffix("*/").unwrap_or(t);
    t.trim_end().to_string()
}

fn push_block(
    out: &mut Vec<DocPassage>,
    offsets: &[usize],
    text: &str,
    is_doc: bool,
    start_line: usize,
    end_line: usize,
    body: String,
) {
    let body = body.trim();
    // A block comment containing a marker is classified as a marker passage.
    if let Some((marker, _)) = find_marker(body) {
        let kind = marker_kind(marker);
        record_marker(out, offsets, text, kind, marker, body, start_line, end_line);
    } else if is_doc && !body.is_empty() {
        record_text_passage(out, offsets, text, true, start_line, end_line, body);
    }
}

fn record_text_passage(
    out: &mut Vec<DocPassage>,
    offsets: &[usize],
    text: &str,
    is_doc: bool,
    start_line: usize,
    end_line: usize,
    body: &str,
) {
    if body.trim().is_empty() {
        return;
    }
    // A doc comment that contains a marker is recorded as the marker.
    if let Some((marker, _)) = find_marker(body) {
        let kind = marker_kind(marker);
        record_marker(out, offsets, text, kind, marker, body, start_line, end_line);
        return;
    }
    if !is_doc {
        return;
    }
    let (byte_start, byte_end) = byte_span(offsets, text, start_line, end_line);
    out.push(DocPassage {
        kind: DocPassageKind::DocComment,
        marker: None,
        text: body.trim().to_string(),
        start_line: (start_line + 1) as u32,
        end_line: (end_line + 1) as u32,
        byte_start,
        byte_end,
    });
}

#[allow(clippy::too_many_arguments)]
fn record_marker(
    out: &mut Vec<DocPassage>,
    offsets: &[usize],
    text: &str,
    kind: DocPassageKind,
    marker: &str,
    body: &str,
    start_line: usize,
    end_line: usize,
) {
    let (byte_start, byte_end) = byte_span(offsets, text, start_line, end_line);
    out.push(DocPassage {
        kind,
        marker: Some(marker.to_string()),
        text: body.trim().to_string(),
        start_line: (start_line + 1) as u32,
        end_line: (end_line + 1) as u32,
        byte_start,
        byte_end,
    });
}

/// Find the first marker keyword at a word boundary in `text`. Returns the
/// canonical keyword and its byte index.
fn find_marker(text: &str) -> Option<(&'static str, usize)> {
    let bytes = text.as_bytes();
    let mut best: Option<(&'static str, usize)> = None;
    for marker in MARKERS {
        let mut from = 0usize;
        while let Some(rel) = text[from..].find(marker) {
            let idx = from + rel;
            let before_ok = idx == 0 || !is_word_byte(bytes[idx - 1]);
            let after = idx + marker.len();
            let after_ok = after >= bytes.len() || !is_word_byte(bytes[after]);
            if before_ok && after_ok {
                match best {
                    Some((_, bidx)) if bidx <= idx => {}
                    _ => best = Some((marker, idx)),
                }
                break;
            }
            from = idx + marker.len();
        }
    }
    best
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn marker_kind(marker: &str) -> DocPassageKind {
    match marker {
        "SAFETY" | "NOTE" => DocPassageKind::Safety,
        _ => DocPassageKind::Todo,
    }
}

fn line_offsets(text: &str) -> Vec<usize> {
    let mut offsets = vec![0usize];
    for (idx, ch) in text.char_indices() {
        if ch == '\n' {
            offsets.push(idx + 1);
        }
    }
    offsets
}

fn byte_span(offsets: &[usize], text: &str, start_line: usize, end_line: usize) -> (usize, usize) {
    let start = offsets.get(start_line).copied().unwrap_or(0);
    let end = offsets.get(end_line + 1).copied().unwrap_or(text.len());
    (start, end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_rust_outer_doc_comment_block() {
        let src = "/// First line.\n/// Second line.\npub fn x() {}\n";
        let passages = extract_doc_passages(src);
        let docs: Vec<&DocPassage> = passages
            .iter()
            .filter(|p| p.kind == DocPassageKind::DocComment)
            .collect();
        assert_eq!(docs.len(), 1, "{passages:?}");
        assert_eq!(docs[0].text, "First line.\nSecond line.");
        assert_eq!(docs[0].start_line, 1);
        assert_eq!(docs[0].end_line, 2);
    }

    #[test]
    fn extracts_todo_and_fixme_markers() {
        let src = "fn a() {}\n// TODO: handle the edge case\nfn b() {}\n  // FIXME broken\n";
        let passages = extract_doc_passages(src);
        let todos: Vec<&DocPassage> = passages
            .iter()
            .filter(|p| p.kind == DocPassageKind::Todo)
            .collect();
        assert_eq!(todos.len(), 2, "{passages:?}");
        assert_eq!(todos[0].marker.as_deref(), Some("TODO"));
        assert!(todos[0].text.contains("edge case"));
        assert_eq!(todos[1].marker.as_deref(), Some("FIXME"));
    }

    #[test]
    fn safety_note_is_classified_safety() {
        let src = "// SAFETY: the pointer is valid here\nfn x() {}\n";
        let passages = extract_doc_passages(src);
        assert_eq!(passages.len(), 1);
        assert_eq!(passages[0].kind, DocPassageKind::Safety);
        assert_eq!(passages[0].marker.as_deref(), Some("SAFETY"));
    }

    #[test]
    fn plain_line_comment_without_marker_is_ignored() {
        let src = "// just a comment\nfn x() {}\n";
        assert!(extract_doc_passages(src).is_empty());
    }

    #[test]
    fn block_doc_comment_extracted() {
        let src = "/** A block doc.\n *  more text\n */\nfn x() {}\n";
        let passages = extract_doc_passages(src);
        let docs: Vec<&DocPassage> = passages
            .iter()
            .filter(|p| p.kind == DocPassageKind::DocComment)
            .collect();
        assert_eq!(docs.len(), 1, "{passages:?}");
        assert!(docs[0].text.contains("A block doc."));
    }

    #[test]
    fn marker_word_boundary_avoids_false_positive() {
        // "TODOLIST" is not a TODO marker.
        let src = "// TODOLIST is a variable name\nfn x() {}\n";
        assert!(extract_doc_passages(src).is_empty());
    }
}
