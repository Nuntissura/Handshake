//! WP-KERNEL-009 MT-097 TreeSitterParserAdapter.
//!
//! Master Spec anchor: 02-system-architecture.md section 2.3.13.11 "Project
//! Knowledge Index and Rich Document Authority" [ADD v02.192] — code navigation
//! "must use Handshake-managed parsers/adapters and store spans/citations in
//! PostgreSQL. Tree-sitter is the first parser family".
//!
//! This adapter is the single reusable Tree-sitter entry point for the whole
//! CodeIndexingAndNavigation group (MT-098..MT-112). It wraps the same grammar
//! family the ai_ready_data chunker already links statically
//! (`tree_sitter_rust`/`tree_sitter_javascript`/`tree_sitter_typescript`,
//! proven by MT-028) and exposes:
//!
//! * deterministic LANGUAGE DETECTION from a repo-relative path
//!   ([`detect_code_language`]),
//! * a parser-version RECEIPT string carried onto every span/entity/edge
//!   ([`CodeParserAdapter::parser_version`]), so re-index staleness (MT-107)
//!   can compare parser versions, and
//! * a typed AST NODE STREAM ([`AstNode`]) — a flattened, owned projection of
//!   the named Tree-sitter nodes with byte ranges, 1-based line ranges, the
//!   immediate field name a node occupies in its parent, and a `name`/path
//!   helper — so the per-language symbol extractors (MT-098..MT-100) and the
//!   relationship builder (MT-104) never touch raw `tree_sitter::Node`
//!   lifetimes.
//!
//! No external service, no LSP daemon: this is a pure in-process parse. A parse
//! that the grammar cannot complete is surfaced as a typed
//! [`CodeParseError`], never a panic and never a silent empty result, so the
//! partial-failure handler (MT-108) can record a precise receipt.

use std::str::FromStr;

use serde::{Deserialize, Serialize};

use super::{CodeIndexError, CodeIndexResult};

/// Code languages the adapter can parse. Mirrors the grammar set linked in
/// `Cargo.toml` (Rust, JavaScript, TypeScript, TSX); TSX is a distinct
/// Tree-sitter grammar from plain TypeScript.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CodeLanguage {
    Rust,
    JavaScript,
    TypeScript,
    Tsx,
}

impl CodeLanguage {
    /// Stable string key (used in provenance + entity keys).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::JavaScript => "javascript",
            Self::TypeScript => "typescript",
            Self::Tsx => "tsx",
        }
    }

    /// The Tree-sitter language for this code language.
    pub fn tree_sitter_language(&self) -> tree_sitter::Language {
        match self {
            Self::Rust => tree_sitter_rust::LANGUAGE.into(),
            Self::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            Self::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Self::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
        }
    }
}

impl FromStr for CodeLanguage {
    type Err = CodeIndexError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "rust" => Ok(Self::Rust),
            "javascript" => Ok(Self::JavaScript),
            "typescript" => Ok(Self::TypeScript),
            "tsx" => Ok(Self::Tsx),
            other => Err(CodeIndexError::Validation(format!(
                "unknown code language '{other}'"
            ))),
        }
    }
}

/// Detect the code language of a repo-relative path by extension. Returns
/// `None` for non-code paths (config/markdown/binary) so callers can route
/// them to the config extractor (MT-101) or skip them. `.d.ts` is treated as
/// TypeScript.
pub fn detect_code_language(relative_path: &str) -> Option<CodeLanguage> {
    let lower = relative_path.to_ascii_lowercase();
    // Order matters: the most specific compound suffixes first.
    if lower.ends_with(".d.ts") {
        return Some(CodeLanguage::TypeScript);
    }
    if lower.ends_with(".tsx") {
        return Some(CodeLanguage::Tsx);
    }
    if lower.ends_with(".ts") || lower.ends_with(".mts") || lower.ends_with(".cts") {
        return Some(CodeLanguage::TypeScript);
    }
    if lower.ends_with(".jsx") {
        // JSX is parsed by the JavaScript grammar (it accepts JSX).
        return Some(CodeLanguage::JavaScript);
    }
    if lower.ends_with(".js") || lower.ends_with(".mjs") || lower.ends_with(".cjs") {
        return Some(CodeLanguage::JavaScript);
    }
    if lower.ends_with(".rs") {
        return Some(CodeLanguage::Rust);
    }
    None
}

/// Parser-version receipt component for a language. Bumped when extractor or
/// adapter behavior changes so MT-107 can detect parser-version staleness.
const ADAPTER_VERSION: &str = "treesitter_adapter_v1";

/// One named AST node, flattened from the Tree-sitter tree into an owned,
/// lifetime-free record the rest of the group consumes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstNode {
    /// Index of this node in [`ParsedTree::nodes`] (stable within one parse).
    pub index: usize,
    /// Index of the parent node in [`ParsedTree::nodes`], or `None` for the
    /// root's direct children whose parent is the (unrecorded) root.
    pub parent: Option<usize>,
    /// Tree-sitter node kind, e.g. `function_item`, `struct_item`,
    /// `call_expression`, `use_declaration`.
    pub kind: String,
    /// The field name this node occupies in its parent, when it is a named
    /// field (e.g. `name`, `body`, `type`). Empty when the node is an
    /// anonymous child.
    pub field_name: Option<String>,
    /// Byte range `[start, end)` in the source.
    pub start_byte: usize,
    pub end_byte: usize,
    /// 1-based inclusive line range.
    pub start_line: u32,
    pub end_line: u32,
    /// Depth from the root (root children are depth 0).
    pub depth: u32,
    /// True when the Tree-sitter parser flagged this subtree as containing a
    /// syntax error (used by the partial-failure handler).
    pub has_error: bool,
}

/// The result of a parse: the source-derived line table plus the flattened
/// named-node stream.
#[derive(Debug, Clone)]
pub struct ParsedTree {
    pub language: CodeLanguage,
    pub parser_version: String,
    /// Whole-tree error flag from Tree-sitter (`root.has_error()`).
    pub root_has_error: bool,
    /// Flattened named nodes in pre-order (parents before children).
    pub nodes: Vec<AstNode>,
    /// Source length in bytes (for range validation by consumers).
    pub source_len: usize,
}

impl ParsedTree {
    /// Slice the exact source text of a node. Returns `None` if the byte range
    /// is not a UTF-8 boundary slice (never panics).
    pub fn node_text<'a>(&self, node: &AstNode, source: &'a str) -> Option<&'a str> {
        source.get(node.start_byte..node.end_byte)
    }

    /// Direct children of a node, in source order.
    pub fn children_of(&self, parent_index: usize) -> impl Iterator<Item = &AstNode> {
        self.nodes
            .iter()
            .filter(move |n| n.parent == Some(parent_index))
    }

    /// The first direct child matching `field_name` (e.g. the `name` node of a
    /// declaration), returning its text.
    pub fn child_field_text<'a>(
        &self,
        parent_index: usize,
        field_name: &str,
        source: &'a str,
    ) -> Option<&'a str> {
        self.children_of(parent_index)
            .find(|n| n.field_name.as_deref() == Some(field_name))
            .and_then(|n| self.node_text(n, source))
    }

    /// All nodes whose kind is in `kinds` (top-level helper for extractors).
    pub fn nodes_of_kinds<'a>(&'a self, kinds: &'a [&'a str]) -> impl Iterator<Item = &'a AstNode> {
        self.nodes
            .iter()
            .filter(move |n| kinds.contains(&n.kind.as_str()))
    }
}

/// The reusable Tree-sitter parser adapter. Cheap to construct; one per parse
/// call (Tree-sitter `Parser` is not `Sync`).
pub struct CodeParserAdapter {
    language: CodeLanguage,
}

impl CodeParserAdapter {
    pub fn new(language: CodeLanguage) -> Self {
        Self { language }
    }

    /// Build an adapter for a path, or `None` if the path is not code.
    pub fn for_path(relative_path: &str) -> Option<Self> {
        detect_code_language(relative_path).map(Self::new)
    }

    pub fn language(&self) -> CodeLanguage {
        self.language
    }

    /// The parser-version receipt for this language, e.g.
    /// `treesitter_adapter_v1/rust/ts0.24`. Stored on every span/entity/edge so
    /// staleness detection can compare parser versions across runs.
    pub fn parser_version(&self) -> String {
        format!(
            "{ADAPTER_VERSION}/{}/ts{}",
            self.language.as_str(),
            tree_sitter::MIN_COMPATIBLE_LANGUAGE_VERSION
        )
    }

    /// Parse source into the flattened typed node stream. Fails closed with a
    /// typed [`CodeParseError`] (never a panic) when the grammar cannot be
    /// initialised or the parse returns no tree. A tree WITH syntax errors is
    /// returned (with `root_has_error = true`) rather than rejected: the
    /// symbol extractors still recover whatever well-formed nodes exist
    /// (MT-108 partial indexing), and the caller decides how to record it.
    pub fn parse(&self, source: &str) -> CodeIndexResult<ParsedTree> {
        let mut parser = tree_sitter::Parser::new();
        let ts_language = self.language.tree_sitter_language();
        parser.set_language(&ts_language).map_err(|err| {
            CodeIndexError::Parse(CodeParseError {
                language: self.language,
                reason: format!("tree-sitter language init failed: {err}"),
            })
        })?;

        let tree = parser.parse(source, None).ok_or_else(|| {
            CodeIndexError::Parse(CodeParseError {
                language: self.language,
                reason: "tree-sitter parse returned no tree".to_string(),
            })
        })?;

        let root = tree.root_node();
        let root_has_error = root.has_error();
        let line_offsets = compute_line_offsets(source);

        let mut nodes: Vec<AstNode> = Vec::new();
        // Iterative pre-order walk over NAMED nodes, recording parent links by
        // index. We push the root's named children with parent = None and
        // recurse; the root itself is not recorded (it is the whole file).
        let mut stack: Vec<(tree_sitter::Node, Option<usize>, u32)> = Vec::new();
        {
            let mut cursor = root.walk();
            // Seed with the root's direct named children (depth 0).
            let children: Vec<tree_sitter::Node> = root.named_children(&mut cursor).collect();
            // Push in reverse so the stack pops them in source order.
            for child in children.into_iter().rev() {
                stack.push((child, None, 0));
            }
        }

        while let Some((node, parent_index, depth)) = stack.pop() {
            let start_byte = node.start_byte();
            let end_byte = node.end_byte();
            let (start_line, end_line) =
                byte_range_to_line_range(&line_offsets, start_byte, end_byte);
            let field_name = node_field_name(&node);
            let this_index = nodes.len();
            nodes.push(AstNode {
                index: this_index,
                parent: parent_index,
                kind: node.kind().to_string(),
                field_name,
                start_byte,
                end_byte,
                start_line,
                end_line,
                depth,
                has_error: node.has_error(),
            });

            let mut cursor = node.walk();
            let children: Vec<tree_sitter::Node> = node.named_children(&mut cursor).collect();
            for child in children.into_iter().rev() {
                stack.push((child, Some(this_index), depth + 1));
            }
        }

        // The pre-order property (parent before child) is required by
        // consumers that resolve enclosing scopes; restore it because the
        // stack walk above emits a valid-but-not-strictly-preorder sequence.
        // We re-sort by start_byte then by range length (outer before inner)
        // and remap parent indices.
        let sorted = reorder_preorder(nodes);

        Ok(ParsedTree {
            language: self.language,
            parser_version: self.parser_version(),
            root_has_error,
            nodes: sorted,
            source_len: source.len(),
        })
    }
}

/// A typed parse failure (carries the language + a human reason for the
/// receipt). Distinct from a successful parse that merely contains syntax
/// errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeParseError {
    pub language: CodeLanguage,
    pub reason: String,
}

impl std::fmt::Display for CodeParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} parse error: {}", self.language.as_str(), self.reason)
    }
}

impl std::error::Error for CodeParseError {}

/// The field name a node occupies in its parent, if any.
fn node_field_name(node: &tree_sitter::Node) -> Option<String> {
    // Tree-sitter exposes the field id via the parent's cursor; the node API
    // surfaces it through `Node::field_name_for_child` on the parent. The
    // simplest robust path is to ask the parent.
    let parent = node.parent()?;
    let mut cursor = parent.walk();
    if cursor.goto_first_child() {
        loop {
            if cursor.node().id() == node.id() {
                return cursor.field_name().map(|s| s.to_string());
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    None
}

/// Reorder a parent-indexed node list into strict pre-order (parent before
/// child, siblings by start byte), remapping parent indices to the new
/// positions. Deterministic.
fn reorder_preorder(nodes: Vec<AstNode>) -> Vec<AstNode> {
    if nodes.is_empty() {
        return nodes;
    }
    // Build children adjacency keyed by old parent index (None => roots).
    let mut roots: Vec<usize> = Vec::new();
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
    for (old_idx, node) in nodes.iter().enumerate() {
        match node.parent {
            Some(p) => children[p].push(old_idx),
            None => roots.push(old_idx),
        }
    }
    let key = |i: &usize| {
        let n = &nodes[*i];
        (n.start_byte, std::cmp::Reverse(n.end_byte))
    };
    roots.sort_by_key(key);
    for bucket in &mut children {
        bucket.sort_by_key(key);
    }

    // DFS emitting old indices in pre-order.
    let mut order: Vec<usize> = Vec::with_capacity(nodes.len());
    let mut stack: Vec<usize> = roots.into_iter().rev().collect();
    while let Some(old_idx) = stack.pop() {
        order.push(old_idx);
        for child in children[old_idx].iter().rev() {
            stack.push(*child);
        }
    }

    // old index -> new index.
    let mut new_index = vec![0usize; nodes.len()];
    for (new_idx, old_idx) in order.iter().enumerate() {
        new_index[*old_idx] = new_idx;
    }

    let mut out: Vec<AstNode> = Vec::with_capacity(nodes.len());
    for old_idx in &order {
        let mut node = nodes[*old_idx].clone();
        node.index = new_index[*old_idx];
        node.parent = node.parent.map(|p| new_index[p]);
        out.push(node);
    }
    out
}

/// Byte offset of the start of each line (line 0 starts at byte 0).
fn compute_line_offsets(source: &str) -> Vec<usize> {
    let mut offsets = vec![0usize];
    for (idx, ch) in source.char_indices() {
        if ch == '\n' {
            offsets.push(idx + 1);
        }
    }
    offsets
}

/// Map a byte range to a 1-based inclusive line range.
fn byte_range_to_line_range(
    line_offsets: &[usize],
    byte_start: usize,
    byte_end: usize,
) -> (u32, u32) {
    fn line_for_byte(line_offsets: &[usize], byte: usize) -> u32 {
        match line_offsets.binary_search(&byte) {
            Ok(idx) => idx as u32,
            Err(idx) => idx.saturating_sub(1) as u32,
        }
    }
    let start = line_for_byte(line_offsets, byte_start);
    let end = line_for_byte(line_offsets, byte_end.saturating_sub(1).max(byte_start));
    (start + 1, end + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_languages_by_extension() {
        assert_eq!(
            detect_code_language("src/main.rs"),
            Some(CodeLanguage::Rust)
        );
        assert_eq!(
            detect_code_language("app/x.ts"),
            Some(CodeLanguage::TypeScript)
        );
        assert_eq!(detect_code_language("app/x.tsx"), Some(CodeLanguage::Tsx));
        assert_eq!(
            detect_code_language("types/x.d.ts"),
            Some(CodeLanguage::TypeScript)
        );
        assert_eq!(
            detect_code_language("app/x.js"),
            Some(CodeLanguage::JavaScript)
        );
        assert_eq!(
            detect_code_language("app/x.jsx"),
            Some(CodeLanguage::JavaScript)
        );
        assert_eq!(detect_code_language("README.md"), None);
        assert_eq!(detect_code_language("Cargo.toml"), None);
    }

    #[test]
    fn parses_rust_into_preorder_named_nodes() {
        let src = "fn alpha() {}\nstruct Beta { x: i32 }\n";
        let adapter = CodeParserAdapter::new(CodeLanguage::Rust);
        let tree = adapter.parse(src).expect("parse");
        assert!(!tree.root_has_error);
        // Top-level fn + struct present.
        let kinds: Vec<&str> = tree.nodes.iter().map(|n| n.kind.as_str()).collect();
        assert!(kinds.contains(&"function_item"), "{kinds:?}");
        assert!(kinds.contains(&"struct_item"), "{kinds:?}");
        // Pre-order: a parent always appears before its children.
        for node in &tree.nodes {
            if let Some(p) = node.parent {
                assert!(
                    p < node.index,
                    "parent {p} must precede child {}",
                    node.index
                );
            }
        }
        // The fn name is reachable via the `name` field.
        let fn_node = tree
            .nodes
            .iter()
            .find(|n| n.kind == "function_item")
            .unwrap();
        assert_eq!(
            tree.child_field_text(fn_node.index, "name", src),
            Some("alpha")
        );
    }

    #[test]
    fn parser_version_is_stable_and_language_tagged() {
        let v = CodeParserAdapter::new(CodeLanguage::Rust).parser_version();
        assert!(v.starts_with("treesitter_adapter_v1/rust/ts"));
    }

    #[test]
    fn malformed_source_returns_tree_with_error_flag_not_panic() {
        let src = "fn broken( {";
        let adapter = CodeParserAdapter::new(CodeLanguage::Rust);
        let tree = adapter.parse(src).expect("parse still returns a tree");
        assert!(tree.root_has_error);
    }

    #[test]
    fn line_ranges_are_one_based() {
        let src = "fn a() {}\nfn b() {}\n";
        let tree = CodeParserAdapter::new(CodeLanguage::Rust)
            .parse(src)
            .unwrap();
        let b = tree
            .nodes
            .iter()
            .find(|n| {
                n.kind == "function_item"
                    && tree.child_field_text(n.index, "name", src) == Some("b")
            })
            .unwrap();
        assert_eq!(b.start_line, 2);
    }
}
