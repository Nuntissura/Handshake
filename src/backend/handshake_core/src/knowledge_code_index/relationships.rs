//! WP-KERNEL-009 MT-104 CallImportRelationshipBuilder.
//!
//! Master Spec anchor: 2.3.13.11 KnowledgeEdge — "a typed relationship between
//! entities or sources. Every edge MUST carry source span refs, extractor
//! version, lifecycle state, confidence, and stable relationship_id."
//!
//! This module extracts call and import RELATIONSHIP CANDIDATES from a parsed
//! tree: pure data, no DB. The engine ([`super::engine`]) resolves each
//! candidate against the workspace's `symbol` entities and writes the edge
//! through `KnowledgeStore::upsert_knowledge_edge`, which derives the stable
//! `relationship_id` (via `derive_knowledge_relationship_id`), enforces the
//! required evidence span, and records the extractor version + confidence.
//!
//! Candidate kinds (per MT contract: import/calls/implements/tests/documents):
//! * [`RelationshipKind::Calls`]  -> `references` edge (caller symbol ->
//!   callee name), evidence = the call-site span.
//! * [`RelationshipKind::Imports`] -> `depends_on` edge (file -> imported
//!   module path), evidence = the import-statement span.
//! * [`RelationshipKind::Implements`] -> `implements` edge (Rust `impl Trait
//!   for Type`: Type -> Trait), evidence = the impl header span.
//!
//! (`tests` edges come from MT-102 TestMappingExtractor; `documents` edges from
//! MT-103 doc passages — both are resolved by the engine. This builder owns the
//! call/import/implements families.)
//!
//! Confidence is conservative and fixed per candidate kind: structural facts
//! present verbatim in the AST (imports, impl headers) are high-confidence;
//! call edges resolved only by simple name are medium-confidence because a bare
//! name can collide across modules.

use serde::{Deserialize, Serialize};

use super::parser::{AstNode, CodeLanguage, ParsedTree};
use super::symbols::ExtractedSymbol;

/// The kind of an extracted relationship candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipKind {
    Calls,
    Imports,
    Implements,
}

impl RelationshipKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Calls => "calls",
            Self::Imports => "imports",
            Self::Implements => "implements",
        }
    }

    /// Conservative default confidence for this relationship kind.
    pub fn default_confidence(&self) -> f64 {
        match self {
            // Verbatim structural facts.
            Self::Imports => 0.95,
            Self::Implements => 0.9,
            // Resolved by simple name -> can collide.
            Self::Calls => 0.6,
        }
    }
}

/// One relationship candidate: a source endpoint (a symbol path within this
/// file, or the file itself), a target NAME to resolve, and the evidence span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationshipCandidate {
    pub kind: RelationshipKind,
    /// The enclosing symbol path that owns the relationship, when the source
    /// endpoint is a symbol (e.g. the caller function). `None` means the file
    /// itself is the source endpoint (imports).
    pub source_symbol_path: Option<String>,
    /// The target to resolve: a simple symbol name (calls/implements) or a
    /// module path string (imports).
    pub target_name: String,
    /// Evidence span byte range `[start, end)`.
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: u32,
    pub end_line: u32,
}

/// Extract call/import/implements candidates from a parsed tree. `symbols` is
/// used to attribute call sites to their enclosing symbol. Deterministic.
pub fn extract_relationships(
    tree: &ParsedTree,
    source: &str,
    symbols: &[ExtractedSymbol],
) -> Vec<RelationshipCandidate> {
    let mut out = Vec::new();
    extract_imports(tree, source, &mut out);
    extract_calls(tree, source, symbols, &mut out);
    if tree.language == CodeLanguage::Rust {
        extract_rust_impls(tree, source, &mut out);
    }
    out.sort_by(|a, b| {
        a.start_byte
            .cmp(&b.start_byte)
            .then(a.target_name.cmp(&b.target_name))
            .then((a.kind as u8).cmp(&(b.kind as u8)))
    });
    out.dedup();
    out
}

// ---------------------------------------------------------------------------
// Imports.
// ---------------------------------------------------------------------------

fn extract_imports(tree: &ParsedTree, source: &str, out: &mut Vec<RelationshipCandidate>) {
    for node in &tree.nodes {
        let module = match (tree.language, node.kind.as_str()) {
            // Rust: `use a::b::c;` -> module path `a::b::c`.
            (CodeLanguage::Rust, "use_declaration") => rust_use_path(tree, node, source),
            // JS/TS: `import ... from "mod"` / `export ... from "mod"`.
            (_, "import_statement") | (_, "export_statement") => {
                js_import_source(tree, node, source)
            }
            _ => None,
        };
        let Some(module) = module else { continue };
        if module.trim().is_empty() {
            continue;
        }
        out.push(RelationshipCandidate {
            kind: RelationshipKind::Imports,
            source_symbol_path: None,
            target_name: module,
            start_byte: node.start_byte,
            end_byte: node.end_byte,
            start_line: node.start_line,
            end_line: node.end_line,
        });
    }
}

/// The path of a Rust `use_declaration` (the text after `use`, before `;`,
/// normalized of whitespace). For grouped uses (`use a::{b, c}`) the common
/// prefix `a` is recorded (a navigable module fact).
fn rust_use_path(tree: &ParsedTree, node: &AstNode, source: &str) -> Option<String> {
    let text = tree.node_text(node, source)?;
    let inner = text
        .trim()
        .strip_prefix("use")
        .unwrap_or(text)
        .trim()
        .trim_end_matches(';')
        .trim();
    // Strip a trailing group / glob to the common prefix.
    let prefix = inner.split("::{").next().unwrap_or(inner);
    let prefix = prefix.trim_end_matches("::*").trim();
    if prefix.is_empty() {
        None
    } else {
        Some(prefix.to_string())
    }
}

/// The module string literal of a JS/TS import/export-from statement.
fn js_import_source(tree: &ParsedTree, node: &AstNode, source: &str) -> Option<String> {
    // The `source` field is a string node; otherwise the last string child.
    let string_node = tree
        .children_of(node.index)
        .find(|c| c.field_name.as_deref() == Some("source") && c.kind == "string")
        .or_else(|| tree.children_of(node.index).find(|c| c.kind == "string"))?;
    let raw = tree.node_text(string_node, source)?;
    Some(raw.trim_matches(['"', '\'', '`']).to_string())
}

// ---------------------------------------------------------------------------
// Calls.
// ---------------------------------------------------------------------------

fn extract_calls(
    tree: &ParsedTree,
    source: &str,
    symbols: &[ExtractedSymbol],
    out: &mut Vec<RelationshipCandidate>,
) {
    for node in &tree.nodes {
        if node.kind != "call_expression" {
            continue;
        }
        let Some(callee) = call_callee_name(tree, node, source) else {
            continue;
        };
        if callee.trim().is_empty() {
            continue;
        }
        let enclosing = enclosing_symbol_path(symbols, node.start_byte, node.end_byte);
        out.push(RelationshipCandidate {
            kind: RelationshipKind::Calls,
            source_symbol_path: enclosing,
            target_name: callee,
            start_byte: node.start_byte,
            end_byte: node.end_byte,
            start_line: node.start_line,
            end_line: node.end_line,
        });
    }
}

/// The callee simple name of a call expression.
fn call_callee_name(tree: &ParsedTree, call: &AstNode, source: &str) -> Option<String> {
    let callee = tree
        .children_of(call.index)
        .find(|c| c.field_name.as_deref() == Some("function"))
        .or_else(|| tree.children_of(call.index).next())?;
    match callee.kind.as_str() {
        "identifier" => tree.node_text(callee, source).map(|s| s.to_string()),
        "scoped_identifier" | "field_expression" | "member_expression" => tree
            .children_of(callee.index)
            .filter(|c| {
                matches!(
                    c.kind.as_str(),
                    "identifier" | "property_identifier" | "field_identifier"
                )
            })
            .last()
            .and_then(|c| tree.node_text(c, source))
            .map(|s| s.to_string()),
        _ => None,
    }
}

/// The smallest symbol whose byte range encloses `[start, end)`.
fn enclosing_symbol_path(symbols: &[ExtractedSymbol], start: usize, end: usize) -> Option<String> {
    symbols
        .iter()
        .filter(|s| s.start_byte <= start && s.end_byte >= end)
        .min_by_key(|s| s.end_byte - s.start_byte)
        .map(|s| s.symbol_path.clone())
}

// ---------------------------------------------------------------------------
// Rust impls.
// ---------------------------------------------------------------------------

fn extract_rust_impls(tree: &ParsedTree, source: &str, out: &mut Vec<RelationshipCandidate>) {
    for node in &tree.nodes {
        if node.kind != "impl_item" {
            continue;
        }
        // `impl Trait for Type`: the `trait` field is the trait, `type` is the
        // implementing type. We record Type --implements--> Trait.
        let trait_name = tree.child_field_text(node.index, "trait", source);
        let type_name = tree.child_field_text(node.index, "type", source);
        let (Some(trait_name), Some(type_name)) = (trait_name, type_name) else {
            continue;
        };
        out.push(RelationshipCandidate {
            kind: RelationshipKind::Implements,
            source_symbol_path: Some(simple_type(type_name).to_string()),
            target_name: simple_type(trait_name).to_string(),
            start_byte: node.start_byte,
            end_byte: node.end_byte,
            start_line: node.start_line,
            end_line: node.end_line,
        });
    }
}

/// Strip generic args / path qualifiers from a type for name resolution.
fn simple_type(text: &str) -> &str {
    let t = text.trim();
    let t = t.split('<').next().unwrap_or(t).trim();
    t.rsplit("::").next().unwrap_or(t).trim()
}

#[cfg(test)]
mod tests {
    use super::super::parser::CodeParserAdapter;
    use super::super::symbols::extract_symbols;
    use super::*;

    fn build(lang: CodeLanguage, src: &str) -> Vec<RelationshipCandidate> {
        let tree = CodeParserAdapter::new(lang).parse(src).unwrap();
        let symbols = extract_symbols(&tree, src);
        extract_relationships(&tree, src, &symbols)
    }

    #[test]
    fn rust_extracts_use_imports() {
        let src = "use crate::storage::knowledge;\nuse std::collections::{HashMap, HashSet};\n";
        let rels = build(CodeLanguage::Rust, src);
        let imports: Vec<&str> = rels
            .iter()
            .filter(|r| r.kind == RelationshipKind::Imports)
            .map(|r| r.target_name.as_str())
            .collect();
        assert!(
            imports.contains(&"crate::storage::knowledge"),
            "{imports:?}"
        );
        assert!(imports.contains(&"std::collections"), "{imports:?}");
    }

    #[test]
    fn rust_extracts_call_with_enclosing_symbol() {
        let src = r#"
fn helper() -> i32 { 1 }
fn caller() -> i32 { helper() }
"#;
        let rels = build(CodeLanguage::Rust, src);
        let call = rels
            .iter()
            .find(|r| r.kind == RelationshipKind::Calls && r.target_name == "helper")
            .expect("call edge");
        assert_eq!(call.source_symbol_path.as_deref(), Some("caller"));
        assert!((call.kind.default_confidence() - 0.6).abs() < 1e-9);
    }

    #[test]
    fn rust_extracts_impl_implements_edge() {
        let src = "struct W; trait R {} impl R for W {}\n";
        let rels = build(CodeLanguage::Rust, src);
        let imp = rels
            .iter()
            .find(|r| r.kind == RelationshipKind::Implements)
            .expect("implements edge");
        assert_eq!(imp.source_symbol_path.as_deref(), Some("W"));
        assert_eq!(imp.target_name, "R");
    }

    #[test]
    fn ts_extracts_import_from_module() {
        let src = "import { foo } from \"./bar\";\nimport React from 'react';\n";
        let rels = build(CodeLanguage::TypeScript, src);
        let imports: Vec<&str> = rels
            .iter()
            .filter(|r| r.kind == RelationshipKind::Imports)
            .map(|r| r.target_name.as_str())
            .collect();
        assert!(imports.contains(&"./bar"), "{imports:?}");
        assert!(imports.contains(&"react"), "{imports:?}");
    }

    #[test]
    fn js_extracts_call_edges() {
        let src = "function a() { return 1; }\nfunction b() { return a(); }\n";
        let rels = build(CodeLanguage::JavaScript, src);
        assert!(rels
            .iter()
            .any(|r| r.kind == RelationshipKind::Calls && r.target_name == "a"));
    }
}
