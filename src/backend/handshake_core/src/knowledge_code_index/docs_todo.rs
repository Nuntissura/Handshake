//! WP-KERNEL-009 MT-103 DocCommentTodoExtractor.
//!
//! Master Spec anchor: 2.3.13.11 KnowledgeClaim ("an assertion about a source,
//! product behavior, task, or operator workflow") + KnowledgeSpan. Extracts
//! doc comments, TODO/FIXME/HACK/SAFETY markers, and operator-facing strings as
//! labeled, searchable passages with source spans.
//!
//! Pure data; no DB. The engine maps each [`DocPassage`] to a `text`-kind span
//! and (for markers/strings) a `KnowledgeClaim`, so an agent can search "what
//! TODOs touch this file", "what does the doc comment on this symbol say", or
//! "where is this user-visible message printed".
//!
//! Two extractors:
//! * [`extract_doc_passages`] — a LINE scanner over the raw source
//!   (language-agnostic for the comment syntaxes Rust/TS/JS share): Rust doc
//!   comments `///`/`//!` and `/** ... */`, line `//` and block `/* ... */`
//!   comments that carry a marker keyword. Markers are case-sensitive keywords
//!   at a word boundary: TODO, FIXME, HACK, XXX, SAFETY, NOTE, BUG, OPTIMIZE.
//!   Consecutive doc-comment lines are merged into one passage.
//! * [`extract_operator_strings`] (MT-103 string coverage) — an AST walk over
//!   the parsed tree that pulls USER-VISIBLE string literals out of output
//!   sinks (Rust `println!`/`eprintln!`/`panic!`/log macros; JS/TS
//!   `console.log`/`alert`/...). These are indexed as a DISTINCT
//!   [`DocPassageKind::OperatorString`] passage — a separate span kind/claim
//!   from doc comments and TODO markers — so an operator-facing message is
//!   searchable as its own thing and never conflated with a code comment.

use serde::{Deserialize, Serialize};

use super::parser::{AstNode, CodeLanguage, ParsedTree};

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
    /// A user-visible string literal pulled from an output sink (a
    /// println!/log/console.log/UI message). Indexed separately from comment
    /// and marker passages so operator-facing text is its own searchable kind.
    OperatorString,
}

impl DocPassageKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::DocComment => "doc_comment",
            Self::Todo => "todo",
            Self::Safety => "safety",
            Self::OperatorString => "operator_string",
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

// ---------------------------------------------------------------------------
// MT-103 operator-facing string extraction (AST-based, distinct from comments).
// ---------------------------------------------------------------------------

/// Rust output-sink macros whose first string literal is user-visible.
const RUST_STRING_SINKS: &[&str] = &[
    "println",
    "eprintln",
    "print",
    "eprint",
    "panic",
    "unreachable",
    "todo",
    "unimplemented",
    "assert",
    "assert_eq",
    "assert_ne",
    "format",
    "write",
    "writeln",
    "info",
    "warn",
    "error",
    "debug",
    "trace",
];

/// JS/TS member-call output sinks (object.method) whose string arguments are
/// user-visible.
const JS_STRING_SINK_OBJECTS: &[&str] = &["console"];
const JS_STRING_SINK_FUNCS: &[&str] = &["alert", "prompt", "confirm"];

/// Extract operator-facing string literals from the parsed tree as
/// [`DocPassageKind::OperatorString`] passages. These are the user-visible
/// messages a program prints/logs/shows — indexed SEPARATELY from doc comments
/// and TODO markers so "where is this message shown" is its own searchable
/// passage kind. Pure data; deterministic, document order by byte offset.
///
/// Heuristic (no false-positive comment capture — strings only, from output
/// sinks): a string literal node that is an argument to a recognised sink call.
/// Rust: `macro_invocation` named in [`RUST_STRING_SINKS`]. JS/TS:
/// `call_expression` whose callee is `console.*` or a bare `alert`/`prompt`/
/// `confirm`.
pub fn extract_operator_strings(tree: &ParsedTree, source: &str) -> Vec<DocPassage> {
    let mut out: Vec<DocPassage> = Vec::new();
    let offsets = line_offsets(source);
    match tree.language {
        CodeLanguage::Rust => extract_rust_operator_strings(tree, source, &offsets, &mut out),
        CodeLanguage::JavaScript | CodeLanguage::TypeScript | CodeLanguage::Tsx => {
            extract_js_operator_strings(tree, source, &offsets, &mut out)
        }
    }
    out.sort_by(|a, b| a.byte_start.cmp(&b.byte_start).then(a.text.cmp(&b.text)));
    out.dedup_by(|a, b| a.byte_start == b.byte_start && a.byte_end == b.byte_end);
    out
}

/// Push an OperatorString passage for a string-literal node (the literal text
/// is stored with its surrounding quotes stripped; empty strings are skipped).
fn push_operator_string(
    out: &mut Vec<DocPassage>,
    offsets: &[usize],
    source: &str,
    literal: &AstNode,
) {
    let Some(raw) = source.get(literal.start_byte..literal.end_byte) else {
        return;
    };
    let text = strip_string_quotes(raw);
    if text.trim().is_empty() {
        return;
    }
    // Byte span is the literal itself (precise citation), not the whole line.
    let _ = offsets; // line numbers come from the node's own 1-based lines.
    out.push(DocPassage {
        kind: DocPassageKind::OperatorString,
        marker: None,
        text: text.trim().to_string(),
        start_line: literal.start_line,
        end_line: literal.end_line,
        byte_start: literal.start_byte,
        byte_end: literal.end_byte,
    });
}

/// Strip the surrounding quotes (and a Rust raw-string `r#"..."#` shell) from a
/// string-literal slice. Best-effort: returns the inner text.
fn strip_string_quotes(raw: &str) -> String {
    let t = raw.trim();
    // Rust raw string: r"...", r#"..."#, r##"..."## etc.
    if let Some(rest) = t.strip_prefix('r') {
        let hashes: String = rest.chars().take_while(|c| *c == '#').collect();
        let inner = &rest[hashes.len()..];
        if let Some(inner) = inner.strip_prefix('"') {
            let close = format!("\"{hashes}");
            if let Some(inner) = inner.strip_suffix(&close) {
                return inner.to_string();
            }
        }
    }
    // Ordinary single/double/back quoted string.
    for q in ['"', '\'', '`'] {
        if t.starts_with(q) && t.ends_with(q) && t.len() >= 2 {
            return t[1..t.len() - 1].to_string();
        }
    }
    t.to_string()
}

fn extract_rust_operator_strings(
    tree: &ParsedTree,
    source: &str,
    offsets: &[usize],
    out: &mut Vec<DocPassage>,
) {
    for node in &tree.nodes {
        if node.kind != "macro_invocation" {
            continue;
        }
        // The macro name is the `macro` field / first identifier child.
        let macro_name = tree
            .child_field_text(node.index, "macro", source)
            .or_else(|| {
                tree.children_of(node.index)
                    .find(|c| matches!(c.kind.as_str(), "identifier" | "scoped_identifier"))
                    .and_then(|c| tree.node_text(c, source))
            });
        let Some(macro_name) = macro_name else {
            continue;
        };
        // `log::info` / `tracing::warn` -> take the last path segment.
        let simple = macro_name.rsplit("::").next().unwrap_or(macro_name);
        if !RUST_STRING_SINKS.contains(&simple) {
            continue;
        }
        // Pull every string_literal anywhere under this macro invocation.
        for lit in descendants_of_kind(tree, node.index, &["string_literal", "raw_string_literal"])
        {
            push_operator_string(out, offsets, source, lit);
        }
    }
}

fn extract_js_operator_strings(
    tree: &ParsedTree,
    source: &str,
    offsets: &[usize],
    out: &mut Vec<DocPassage>,
) {
    for node in &tree.nodes {
        if node.kind != "call_expression" {
            continue;
        }
        let Some(callee) = tree
            .children_of(node.index)
            .find(|c| c.field_name.as_deref() == Some("function"))
        else {
            continue;
        };
        let is_sink = match callee.kind.as_str() {
            // `console.log(...)` etc.
            "member_expression" => {
                let object = tree
                    .child_field_text(callee.index, "object", source)
                    .unwrap_or("");
                JS_STRING_SINK_OBJECTS.contains(&object)
            }
            // bare `alert("...")`.
            "identifier" => tree
                .node_text(callee, source)
                .map(|name| JS_STRING_SINK_FUNCS.contains(&name))
                .unwrap_or(false),
            _ => false,
        };
        if !is_sink {
            continue;
        }
        // The argument list's string literals (string / template_string).
        for lit in descendants_of_kind(
            tree,
            node.index,
            &["string", "template_string", "string_fragment"],
        ) {
            // For a `string` node we want the node itself (quoted); for a
            // template_string the whole backtick span. A string_fragment is the
            // inner text of a template; skip it if its parent template_string is
            // already captured to avoid double counting.
            if lit.kind == "string_fragment" {
                continue;
            }
            push_operator_string(out, offsets, source, lit);
        }
    }
}

/// All descendant nodes (any depth) of `ancestor_index` whose kind is in
/// `kinds`. Walks the flattened node stream following the `parent` chain.
fn descendants_of_kind<'a>(
    tree: &'a ParsedTree,
    ancestor_index: usize,
    kinds: &'a [&'a str],
) -> Vec<&'a AstNode> {
    let mut out = Vec::new();
    for node in &tree.nodes {
        if !kinds.contains(&node.kind.as_str()) {
            continue;
        }
        // Is `node` a descendant of `ancestor_index`?
        let mut cur = node.parent;
        while let Some(idx) = cur {
            if idx == ancestor_index {
                out.push(node);
                break;
            }
            cur = tree.nodes[idx].parent;
        }
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

    // -- MT-103 operator-facing string extraction (separate from markers) ------

    use super::super::parser::CodeParserAdapter;

    fn parse(lang: CodeLanguage, src: &str) -> ParsedTree {
        CodeParserAdapter::new(lang).parse(src).expect("parse")
    }

    #[test]
    fn rust_operator_strings_extracted_from_output_sinks() {
        let src = r#"
/// Doc comment, not an operator string.
pub fn run() {
    // TODO: not an operator string either
    println!("Starting the export now");
    let x = "plain binding not in a sink";
    eprintln!("export failed for {}", path);
    log::warn!("low disk space");
}
"#;
        let tree = parse(CodeLanguage::Rust, src);
        let strings = extract_operator_strings(&tree, src);
        let texts: Vec<&str> = strings.iter().map(|p| p.text.as_str()).collect();
        assert!(
            texts.contains(&"Starting the export now"),
            "println! string must be captured: {texts:?}"
        );
        assert!(
            texts.iter().any(|t| t.contains("export failed for")),
            "eprintln! string must be captured: {texts:?}"
        );
        assert!(
            texts.contains(&"low disk space"),
            "log::warn! string must be captured: {texts:?}"
        );
        // The plain binding string is NOT in an output sink -> not captured.
        assert!(
            !texts.contains(&"plain binding not in a sink"),
            "a non-sink string must not be captured: {texts:?}"
        );
        // Every captured passage is the operator_string kind.
        assert!(strings
            .iter()
            .all(|p| p.kind == DocPassageKind::OperatorString));
    }

    #[test]
    fn operator_strings_are_separate_from_doc_and_todo_passages() {
        // The CRUX of the MT-103 remediation: the SAME file's doc comment, TODO
        // marker, and println! string land in THREE different passage streams /
        // kinds, never conflated.
        let src = r#"
/// This is documentation.
pub fn go() {
    // TODO: wire the retry
    println!("user sees this message");
}
"#;
        let tree = parse(CodeLanguage::Rust, src);
        let doc_passages = extract_doc_passages(src);
        let operator_strings = extract_operator_strings(&tree, src);

        // Doc passages contain the doc comment + the TODO, but NO operator string.
        assert!(doc_passages
            .iter()
            .any(|p| p.kind == DocPassageKind::DocComment && p.text.contains("documentation")));
        assert!(doc_passages
            .iter()
            .any(|p| p.kind == DocPassageKind::Todo && p.text.contains("wire the retry")));
        assert!(
            !doc_passages
                .iter()
                .any(|p| p.kind == DocPassageKind::OperatorString),
            "the comment scanner must NOT emit operator strings"
        );
        assert!(
            !doc_passages
                .iter()
                .any(|p| p.text.contains("user sees this message")),
            "the println! string must not appear among doc/TODO passages"
        );

        // Operator strings contain ONLY the println! message, no doc/TODO text.
        assert_eq!(operator_strings.len(), 1, "{operator_strings:?}");
        assert_eq!(operator_strings[0].kind, DocPassageKind::OperatorString);
        assert_eq!(operator_strings[0].text, "user sees this message");

        // Their entity keys are distinct kinds, so the upsert never merges them.
        let todo = doc_passages
            .iter()
            .find(|p| p.kind == DocPassageKind::Todo)
            .unwrap();
        assert_ne!(
            todo.entity_key("src/lib.rs"),
            operator_strings[0].entity_key("src/lib.rs"),
            "operator-string and TODO passages must have distinct entity keys"
        );
        assert!(operator_strings[0]
            .entity_key("src/lib.rs")
            .contains("operator_string"));
    }

    #[test]
    fn js_console_strings_extracted_as_operator_strings() {
        let src = r#"
// a comment
function go() {
  console.log("hello from the app");
  console.error("something broke");
  const unused = "not a sink string";
  alert("popup message");
}
"#;
        let tree = parse(CodeLanguage::JavaScript, src);
        let strings = extract_operator_strings(&tree, src);
        let texts: Vec<&str> = strings.iter().map(|p| p.text.as_str()).collect();
        assert!(texts.contains(&"hello from the app"), "{texts:?}");
        assert!(texts.contains(&"something broke"), "{texts:?}");
        assert!(texts.contains(&"popup message"), "{texts:?}");
        assert!(!texts.contains(&"not a sink string"), "{texts:?}");
        assert!(strings
            .iter()
            .all(|p| p.kind == DocPassageKind::OperatorString));
    }
}
