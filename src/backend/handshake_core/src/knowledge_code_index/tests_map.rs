//! WP-KERNEL-009 MT-102 TestMappingExtractor.
//!
//! Master Spec anchor: 2.3.13.11 KnowledgeEdge (`validates` relationship).
//! Links test symbols to the product symbols they exercise, where detectable,
//! so an agent can answer "which tests cover this function" / "what does this
//! test exercise".
//!
//! Pure data; no DB. Produces [`TestMapping`] candidates: a test symbol path
//! plus the simple names it references inside its body. The relationship
//! builder ([`super::relationships`]) / engine resolves those names against the
//! workspace's `symbol` entities and writes `validates` edges (test -> tested
//! symbol) with the test's span as evidence. Names that do not resolve are
//! simply dropped (no false edges).
//!
//! Detection is deliberately conservative and based on identifiers actually
//! present in the test body (call expressions and paths), so a mapping only
//! exists when there is concrete in-source evidence.

use std::collections::BTreeSet;

use super::parser::{AstNode, CodeLanguage, ParsedTree};
use super::symbols::{ExtractedSymbol, SymbolKind};

/// One test -> referenced-name mapping candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestMapping {
    /// The test symbol's path (e.g. `tests::it_works`).
    pub test_symbol_path: String,
    /// The symbol discriminator assigned by the extractor, when the test path is
    /// not unique (e.g. duplicate JS/TS test titles).
    pub test_disambiguator: Option<String>,
    /// Byte range of the test (evidence span source).
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: u32,
    pub end_line: u32,
    /// Distinct simple identifier names referenced in the test body, sorted.
    /// The engine resolves these against `symbol` entities.
    pub referenced_names: Vec<String>,
}

/// Build test mappings from a parsed tree and its extracted symbols.
/// Deterministic.
pub fn extract_test_mappings(
    tree: &ParsedTree,
    source: &str,
    symbols: &[ExtractedSymbol],
) -> Vec<TestMapping> {
    let mut out = Vec::new();
    for symbol in symbols {
        let is_test = match tree.language {
            CodeLanguage::Rust
            | CodeLanguage::TypeScript
            | CodeLanguage::Tsx
            | CodeLanguage::JavaScript => symbol.kind == SymbolKind::Test,
        };
        if !is_test {
            continue;
        }
        // Find the AST node for this test by byte range to scan its body.
        let Some(node) = tree
            .nodes
            .iter()
            .find(|n| n.start_byte == symbol.start_byte && n.end_byte == symbol.end_byte)
        else {
            continue;
        };
        let names = referenced_names_in_subtree(tree, node, source, &symbol.name);
        if names.is_empty() {
            continue;
        }
        out.push(TestMapping {
            test_symbol_path: symbol.symbol_path.clone(),
            test_disambiguator: symbol.disambiguator.clone(),
            start_byte: symbol.start_byte,
            end_byte: symbol.end_byte,
            start_line: symbol.start_line,
            end_line: symbol.end_line,
            referenced_names: names,
        });
    }
    out
}

/// Collect distinct identifier names that appear as call targets or path
/// segments inside a subtree (excluding the test's own name and common test
/// macros / assertions).
fn referenced_names_in_subtree(
    tree: &ParsedTree,
    root: &AstNode,
    source: &str,
    own_name: &str,
) -> Vec<String> {
    let mut names: BTreeSet<String> = BTreeSet::new();
    // All nodes whose subtree-range is inside the root range and that are
    // call/path identifiers.
    for node in &tree.nodes {
        if node.start_byte < root.start_byte || node.end_byte > root.end_byte {
            continue;
        }
        match node.kind.as_str() {
            "call_expression" => {
                if let Some(name) = call_callee_name(tree, node, source) {
                    if !is_ignored_name(&name) && name != own_name {
                        names.insert(name);
                    }
                }
            }
            // Rust path: `module::function` — take the final segment.
            "identifier" if tree.language == CodeLanguage::Rust => {
                if let Some(text) = tree.node_text(node, source) {
                    if !is_ignored_name(text) && text != own_name && looks_like_symbol(text) {
                        names.insert(text.to_string());
                    }
                }
            }
            _ => {}
        }
    }
    names.into_iter().collect()
}

/// The callee identifier of a call expression (`foo(...)` -> `foo`,
/// `a::b::foo(...)` -> `foo`, `obj.method(...)` -> `method`).
fn call_callee_name(tree: &ParsedTree, call: &AstNode, source: &str) -> Option<String> {
    // The `function` field holds the callee.
    let callee = tree
        .children_of(call.index)
        .find(|c| c.field_name.as_deref() == Some("function"))
        .or_else(|| tree.children_of(call.index).next())?;
    match callee.kind.as_str() {
        "identifier" => tree.node_text(callee, source).map(|s| s.to_string()),
        "scoped_identifier" | "field_expression" | "member_expression" => {
            let ids: Vec<&str> = tree
                .children_of(callee.index)
                .filter(|c| {
                    matches!(
                        c.kind.as_str(),
                        "identifier" | "property_identifier" | "field_identifier"
                    )
                })
                .filter_map(|c| tree.node_text(c, source))
                .collect();
            if tree.language != CodeLanguage::Rust
                && ids.len() == 2
                && matches!(ids[0], "describe" | "it" | "test")
                && matches!(ids[1], "only" | "skip" | "todo")
            {
                return Some(ids[0].to_string());
            }
            // Take the last identifier child as the simple name.
            ids.into_iter().last().map(|s| s.to_string())
        }
        _ => None,
    }
}

/// Filter out test framework noise so mappings point at product symbols.
fn is_ignored_name(name: &str) -> bool {
    matches!(
        name,
        "assert"
            | "assert_eq"
            | "assert_ne"
            | "expect"
            | "unwrap"
            | "panic"
            | "println"
            | "eprintln"
            | "format"
            | "vec"
            | "describe"
            | "it"
            | "test"
            | "beforeEach"
            | "afterEach"
            | "toBe"
            | "toEqual"
    )
}

/// A bare identifier "looks like a symbol" if it is not a keyword-ish short
/// token. Conservative: requires length >= 2.
fn looks_like_symbol(name: &str) -> bool {
    name.len() >= 2
        && name
            .chars()
            .next()
            .map(|c| c.is_alphabetic() || c == '_')
            .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::super::parser::CodeParserAdapter;
    use super::super::symbols::extract_symbols;
    use super::*;

    #[test]
    fn maps_rust_test_to_called_function() {
        let src = r#"
pub fn add(a: i32, b: i32) -> i32 { a + b }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn adds() {
        assert_eq!(add(1, 2), 3);
    }
}
"#;
        let tree = CodeParserAdapter::new(CodeLanguage::Rust)
            .parse(src)
            .unwrap();
        let symbols = extract_symbols(&tree, src);
        let mappings = extract_test_mappings(&tree, src, &symbols);
        assert_eq!(mappings.len(), 1, "{mappings:?}");
        assert_eq!(mappings[0].test_symbol_path, "tests::adds");
        assert!(
            mappings[0].referenced_names.contains(&"add".to_string()),
            "{:?}",
            mappings[0].referenced_names
        );
        // assertion macro filtered out.
        assert!(!mappings[0]
            .referenced_names
            .contains(&"assert_eq".to_string()));
    }

    #[test]
    fn test_with_no_product_calls_yields_no_mapping() {
        let src = r#"
#[cfg(test)]
mod tests {
    #[test]
    fn trivial() { assert!(true); }
}
"#;
        let tree = CodeParserAdapter::new(CodeLanguage::Rust)
            .parse(src)
            .unwrap();
        let symbols = extract_symbols(&tree, src);
        let mappings = extract_test_mappings(&tree, src, &symbols);
        assert!(mappings.is_empty(), "{mappings:?}");
    }

    #[test]
    fn maps_typescript_it_and_test_blocks_to_called_functions() {
        let src = r#"
import { describe, it, test, expect } from "vitest";

export function compute(): number { return 1; }
export function helper(): number { return 2; }

describe("math", () => {
    it("computes value", () => {
        expect(compute()).toBe(1);
    });
    test("uses helper", () => {
        helper();
    });
});
"#;
        let tree = CodeParserAdapter::new(CodeLanguage::TypeScript)
            .parse(src)
            .unwrap();
        let symbols = extract_symbols(&tree, src);
        let mappings = extract_test_mappings(&tree, src, &symbols);
        assert_eq!(mappings.len(), 2, "{mappings:?}");

        let compute = mappings
            .iter()
            .find(|m| m.test_symbol_path == "test.math.computes value")
            .expect("compute test mapping");
        assert_eq!(compute.referenced_names, vec!["compute".to_string()]);

        let helper = mappings
            .iter()
            .find(|m| m.test_symbol_path == "test.math.uses helper")
            .expect("helper test mapping");
        assert_eq!(helper.referenced_names, vec!["helper".to_string()]);
    }

    #[test]
    fn maps_typescript_runner_modifiers_without_modifier_false_edges() {
        let src = r#"
import { describe, it, expect } from "vitest";

export function compute(): number { return 1; }
export function only(): number { return 2; }

describe("math", () => {
    it.only("focused", () => {
        expect(compute()).toBe(1);
    });
});
"#;
        let tree = CodeParserAdapter::new(CodeLanguage::TypeScript)
            .parse(src)
            .unwrap();
        let symbols = extract_symbols(&tree, src);
        let mappings = extract_test_mappings(&tree, src, &symbols);
        assert_eq!(mappings.len(), 1, "{mappings:?}");
        assert_eq!(
            mappings[0].referenced_names,
            vec!["compute".to_string()],
            "runner modifier names must not resolve as product coverage: {mappings:?}"
        );
    }

    #[test]
    fn maps_javascript_test_runner_blocks_to_called_functions() {
        let src = r#"
import { describe, test } from "vitest";

export function compute() { return 1; }

describe("math", () => {
    test("uses compute", () => {
        compute();
    });
});
"#;
        let tree = CodeParserAdapter::new(CodeLanguage::JavaScript)
            .parse(src)
            .unwrap();
        let symbols = extract_symbols(&tree, src);
        let mappings = extract_test_mappings(&tree, src, &symbols);
        assert_eq!(mappings.len(), 1, "{mappings:?}");
        assert_eq!(mappings[0].test_symbol_path, "test.math.uses compute");
        assert_eq!(mappings[0].referenced_names, vec!["compute".to_string()]);
    }

    #[test]
    fn maps_tsx_test_runner_blocks_to_called_components() {
        let src = r#"
import { describe, it } from "vitest";

export function Button() { return <button />; }

describe("ui", () => {
    it("renders button", () => {
        Button();
    });
});
"#;
        let tree = CodeParserAdapter::new(CodeLanguage::Tsx)
            .parse(src)
            .unwrap();
        let symbols = extract_symbols(&tree, src);
        let mappings = extract_test_mappings(&tree, src, &symbols);
        assert_eq!(mappings.len(), 1, "{mappings:?}");
        assert_eq!(mappings[0].test_symbol_path, "test.ui.renders button");
        assert_eq!(mappings[0].referenced_names, vec!["Button".to_string()]);
    }
}
