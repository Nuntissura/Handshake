//! WP-KERNEL-009 MT-098/099/100 Rust/TypeScript/JavaScript symbol extractors.
//!
//! Master Spec anchor: 2.3.13.11 KnowledgeEntity ("a typed symbol ... detected
//! from one or more spans") + KnowledgeSpan ("a byte, text, AST, ... range
//! anchored to a KnowledgeSource ... the minimum citeable evidence unit").
//!
//! These extractors consume the typed AST node stream from
//! [`super::parser::ParsedTree`] (MT-097) and produce [`ExtractedSymbol`]
//! records: pure data, no DB. The orchestrator ([`super::engine`]) turns each
//! symbol into an `ast`-kind span + a `symbol`-kind entity through the storage
//! layer.
//!
//! Symbol kind coverage (per MT contracts):
//! * Rust (MT-098): fn, struct, enum, trait, impl, mod, const/static, type
//!   alias, macro; plus `#[test]`/`#[cfg(test)]` function detection.
//! * TypeScript (MT-099): function, class, interface, type alias, enum, exported
//!   const, React component / hook heuristics, JSX is handled by the TSX grammar.
//! * JavaScript (MT-100): function, class, exported const/function, method.
//!
//! The stable `entity_key` for a code symbol is
//! `{language}:{relative_path}#{symbol_path}` where `symbol_path` is the
//! dotted scope (e.g. `Beta::render` / `MyClass.method`). That key is stable
//! across re-index runs (it never includes byte offsets or run ids), so the
//! entity upsert (MT-053) keeps `entity_id` stable and the deterministic
//! relationship_id derivation (MT-054) can hash it.

use serde::{Deserialize, Serialize};

use super::parser::{AstNode, CodeLanguage, ParsedTree};

/// The kind of a detected code symbol. Maps to a `symbol` KnowledgeEntity with
/// this value carried in `detection_provenance.symbol_kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    Function,
    Method,
    Struct,
    Enum,
    Trait,
    Impl,
    Module,
    Constant,
    TypeAlias,
    Macro,
    Class,
    Interface,
    Component,
    Hook,
    Test,
}

impl SymbolKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::Method => "method",
            Self::Struct => "struct",
            Self::Enum => "enum",
            Self::Trait => "trait",
            Self::Impl => "impl",
            Self::Module => "module",
            Self::Constant => "constant",
            Self::TypeAlias => "type_alias",
            Self::Macro => "macro",
            Self::Class => "class",
            Self::Interface => "interface",
            Self::Component => "component",
            Self::Hook => "hook",
            Self::Test => "test",
        }
    }
}

/// One extracted symbol. Pure data; the engine maps it to span + entity rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedSymbol {
    pub kind: SymbolKind,
    /// Simple identifier, e.g. `render`.
    pub name: String,
    /// Dotted scope path, e.g. `Beta::render` (Rust) or `MyClass.method` (TS).
    pub symbol_path: String,
    /// AST node kind that produced this symbol (provenance), e.g.
    /// `function_item`.
    pub node_kind: String,
    /// Byte range `[start, end)` of the whole symbol definition in source.
    pub start_byte: usize,
    pub end_byte: usize,
    /// 1-based inclusive line range.
    pub start_line: u32,
    pub end_line: u32,
    /// Whether the Tree-sitter subtree for this symbol carried a syntax error.
    pub has_error: bool,
}

impl ExtractedSymbol {
    /// Stable entity key for this symbol within a file.
    pub fn entity_key(&self, language: CodeLanguage, relative_path: &str) -> String {
        format!(
            "{}:{}#{}",
            language.as_str(),
            relative_path,
            self.symbol_path
        )
    }
}

/// Extract symbols from a parsed tree for the tree's language. Dispatches to
/// the per-language extractor. Deterministic; ordered by start byte.
pub fn extract_symbols(tree: &ParsedTree, source: &str) -> Vec<ExtractedSymbol> {
    let mut symbols = match tree.language {
        CodeLanguage::Rust => extract_rust(tree, source),
        CodeLanguage::TypeScript | CodeLanguage::Tsx => extract_typescript(tree, source),
        CodeLanguage::JavaScript => extract_javascript(tree, source),
    };
    symbols.sort_by(|a, b| {
        a.start_byte
            .cmp(&b.start_byte)
            .then(a.symbol_path.cmp(&b.symbol_path))
    });
    symbols
}

// ---------------------------------------------------------------------------
// Shared helpers over the node stream.
// ---------------------------------------------------------------------------

/// The name (`name` field) of a declaration node, falling back to the first
/// `type_identifier`/`identifier` child.
fn decl_name<'a>(tree: &ParsedTree, node: &AstNode, source: &'a str) -> Option<&'a str> {
    if let Some(name) = tree.child_field_text(node.index, "name", source) {
        return Some(name);
    }
    // Fallback: first identifier-ish child.
    tree.children_of(node.index)
        .find(|c| {
            matches!(
                c.kind.as_str(),
                "identifier" | "type_identifier" | "property_identifier"
            )
        })
        .and_then(|c| tree.node_text(c, source))
}

/// Build the dotted scope path of a node by walking parent declaration nodes
/// that contribute a named scope.
fn scope_path(
    tree: &ParsedTree,
    node: &AstNode,
    source: &str,
    name: &str,
    separator: &str,
    scope_kinds: &[&str],
) -> String {
    let mut prefix: Vec<String> = Vec::new();
    let mut current = node.parent;
    while let Some(idx) = current {
        let parent = &tree.nodes[idx];
        if scope_kinds.contains(&parent.kind.as_str()) {
            if let Some(pname) = scope_name_for(tree, parent, source) {
                prefix.push(pname.to_string());
            }
        }
        current = parent.parent;
    }
    prefix.reverse();
    if prefix.is_empty() {
        name.to_string()
    } else {
        format!("{}{separator}{name}", prefix.join(separator))
    }
}

/// The scope-contributing name of a container node (impl target type, struct
/// name, class name, module name).
fn scope_name_for<'a>(tree: &ParsedTree, node: &AstNode, source: &'a str) -> Option<&'a str> {
    match node.kind.as_str() {
        // Rust `impl Foo` / `impl Trait for Foo`: use the `type` field (the
        // implementing type), which is the navigable scope.
        "impl_item" => tree
            .child_field_text(node.index, "type", source)
            .or_else(|| decl_name(tree, node, source)),
        _ => decl_name(tree, node, source),
    }
}

// ---------------------------------------------------------------------------
// MT-098 Rust.
// ---------------------------------------------------------------------------

const RUST_SCOPE_KINDS: &[&str] = &["impl_item", "mod_item", "trait_item"];

fn extract_rust(tree: &ParsedTree, source: &str) -> Vec<ExtractedSymbol> {
    let mut out = Vec::new();
    for node in &tree.nodes {
        let kind = match node.kind.as_str() {
            "function_item" => {
                // Distinguish a #[test] function and methods inside impls.
                if rust_has_test_attribute(tree, node, source) {
                    SymbolKind::Test
                } else if rust_is_in_impl_or_trait(tree, node) {
                    SymbolKind::Method
                } else {
                    SymbolKind::Function
                }
            }
            "struct_item" => SymbolKind::Struct,
            "enum_item" => SymbolKind::Enum,
            "trait_item" => SymbolKind::Trait,
            "impl_item" => SymbolKind::Impl,
            "mod_item" => SymbolKind::Module,
            "const_item" | "static_item" => SymbolKind::Constant,
            "type_item" => SymbolKind::TypeAlias,
            "macro_definition" => SymbolKind::Macro,
            _ => continue,
        };

        let name = match kind {
            SymbolKind::Impl => scope_name_for(tree, node, source).map(|s| s.to_string()),
            _ => decl_name(tree, node, source).map(|s| s.to_string()),
        };
        let Some(name) = name else { continue };
        if name.trim().is_empty() {
            continue;
        }

        // An `impl Type` block names the SAME identifier as its `struct Type`;
        // give the impl a distinct path (`impl Type`) so the two never collide
        // on `entity_key` and merge in the upsert. Methods inside still scope
        // under the bare type (`Type::method`) via `scope_name_for`.
        let path_name = if kind == SymbolKind::Impl {
            format!("impl {name}")
        } else {
            name.clone()
        };
        let symbol_path = scope_path(tree, node, source, &path_name, "::", RUST_SCOPE_KINDS);
        out.push(ExtractedSymbol {
            kind,
            name,
            symbol_path,
            node_kind: node.kind.clone(),
            start_byte: node.start_byte,
            end_byte: node.end_byte,
            start_line: node.start_line,
            end_line: node.end_line,
            has_error: node.has_error,
        });
    }
    out
}

/// True when the function node is preceded by a `#[test]` or
/// `#[tokio::test]`-style attribute, or sits in a `#[cfg(test)]` module.
fn rust_has_test_attribute(tree: &ParsedTree, node: &AstNode, source: &str) -> bool {
    // Attribute items are siblings preceding the function in Tree-sitter Rust.
    let siblings: Vec<&AstNode> = match node.parent {
        Some(p) => tree.children_of(p).collect(),
        None => tree.nodes.iter().filter(|n| n.parent.is_none()).collect(),
    };
    let pos = siblings.iter().position(|n| n.index == node.index);
    if let Some(pos) = pos {
        for prev in siblings[..pos].iter().rev() {
            match prev.kind.as_str() {
                "attribute_item" => {
                    if let Some(text) = tree.node_text(prev, source) {
                        if text.contains("test") {
                            return true;
                        }
                    }
                }
                // Stop scanning once we hit a non-attribute sibling.
                _ => break,
            }
        }
    }
    false
}

fn rust_is_in_impl_or_trait(tree: &ParsedTree, node: &AstNode) -> bool {
    let mut current = node.parent;
    while let Some(idx) = current {
        let parent = &tree.nodes[idx];
        match parent.kind.as_str() {
            "impl_item" | "trait_item" => return true,
            _ => {}
        }
        current = parent.parent;
    }
    false
}

// ---------------------------------------------------------------------------
// MT-099 TypeScript / TSX.
// ---------------------------------------------------------------------------

const TS_SCOPE_KINDS: &[&str] = &["class_declaration", "internal_module", "module"];

fn extract_typescript(tree: &ParsedTree, source: &str) -> Vec<ExtractedSymbol> {
    let mut out = Vec::new();
    for node in &tree.nodes {
        let symbol = match node.kind.as_str() {
            "function_declaration" | "generator_function_declaration" => {
                decl_name(tree, node, source).map(|name| (ts_function_kind(name), name.to_string()))
            }
            "class_declaration" | "abstract_class_declaration" => {
                decl_name(tree, node, source).map(|n| (SymbolKind::Class, n.to_string()))
            }
            "interface_declaration" => {
                decl_name(tree, node, source).map(|n| (SymbolKind::Interface, n.to_string()))
            }
            "type_alias_declaration" => {
                decl_name(tree, node, source).map(|n| (SymbolKind::TypeAlias, n.to_string()))
            }
            "enum_declaration" => {
                decl_name(tree, node, source).map(|n| (SymbolKind::Enum, n.to_string()))
            }
            "method_definition" => {
                method_name(tree, node, source).map(|n| (SymbolKind::Method, n.to_string()))
            }
            "lexical_declaration" | "variable_declaration" => {
                // `const Foo = () => {}` / `const useThing = () => {}`:
                // arrow-function or function-expression bound consts become
                // component/hook/function symbols.
                ts_const_binding(tree, node, source)
            }
            _ => None,
        };

        let Some((kind, name)) = symbol else { continue };
        if name.trim().is_empty() {
            continue;
        }
        let symbol_path = scope_path(tree, node, source, &name, ".", TS_SCOPE_KINDS);
        out.push(ExtractedSymbol {
            kind,
            name,
            symbol_path,
            node_kind: node.kind.clone(),
            start_byte: node.start_byte,
            end_byte: node.end_byte,
            start_line: node.start_line,
            end_line: node.end_line,
            has_error: node.has_error,
        });
    }
    out
}

/// React heuristics: PascalCase => Component, `use` prefix => Hook.
fn ts_function_kind(name: &str) -> SymbolKind {
    if is_hook_name(name) {
        SymbolKind::Hook
    } else if is_component_name(name) {
        SymbolKind::Component
    } else {
        SymbolKind::Function
    }
}

fn is_hook_name(name: &str) -> bool {
    name.len() > 3
        && name.starts_with("use")
        && name[3..]
            .chars()
            .next()
            .map(|c| c.is_ascii_uppercase())
            .unwrap_or(false)
}

fn is_component_name(name: &str) -> bool {
    name.chars()
        .next()
        .map(|c| c.is_ascii_uppercase())
        .unwrap_or(false)
}

fn method_name<'a>(tree: &ParsedTree, node: &AstNode, source: &'a str) -> Option<&'a str> {
    tree.child_field_text(node.index, "name", source)
        .or_else(|| {
            tree.children_of(node.index)
                .find(|c| matches!(c.kind.as_str(), "property_identifier" | "identifier"))
                .and_then(|c| tree.node_text(c, source))
        })
}

/// Inspect a `const X = ...` declaration: only bindings whose value is a
/// function (arrow/function expression) become symbols. Returns the first such
/// binding (declarations usually have one).
fn ts_const_binding(
    tree: &ParsedTree,
    node: &AstNode,
    source: &str,
) -> Option<(SymbolKind, String)> {
    for declarator in tree.children_of(node.index) {
        if declarator.kind != "variable_declarator" {
            continue;
        }
        let name = tree
            .child_field_text(declarator.index, "name", source)
            .or_else(|| {
                tree.children_of(declarator.index)
                    .find(|c| c.kind == "identifier")
                    .and_then(|c| tree.node_text(c, source))
            });
        let Some(name) = name else { continue };
        // Is the value an arrow / function expression?
        let is_fn = tree.children_of(declarator.index).any(|c| {
            matches!(
                c.kind.as_str(),
                "arrow_function" | "function" | "function_expression"
            )
        });
        if is_fn {
            return Some((ts_function_kind(name), name.to_string()));
        }
    }
    None
}

// ---------------------------------------------------------------------------
// MT-100 JavaScript.
// ---------------------------------------------------------------------------

const JS_SCOPE_KINDS: &[&str] = &["class_declaration"];

fn extract_javascript(tree: &ParsedTree, source: &str) -> Vec<ExtractedSymbol> {
    let mut out = Vec::new();
    for node in &tree.nodes {
        let symbol = match node.kind.as_str() {
            "function_declaration" | "generator_function_declaration" => {
                decl_name(tree, node, source).map(|name| (ts_function_kind(name), name.to_string()))
            }
            "class_declaration" => {
                decl_name(tree, node, source).map(|n| (SymbolKind::Class, n.to_string()))
            }
            "method_definition" => {
                method_name(tree, node, source).map(|n| (SymbolKind::Method, n.to_string()))
            }
            "lexical_declaration" | "variable_declaration" => ts_const_binding(tree, node, source),
            _ => None,
        };
        let Some((kind, name)) = symbol else { continue };
        if name.trim().is_empty() {
            continue;
        }
        let symbol_path = scope_path(tree, node, source, &name, ".", JS_SCOPE_KINDS);
        out.push(ExtractedSymbol {
            kind,
            name,
            symbol_path,
            node_kind: node.kind.clone(),
            start_byte: node.start_byte,
            end_byte: node.end_byte,
            start_line: node.start_line,
            end_line: node.end_line,
            has_error: node.has_error,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::parser::CodeParserAdapter;
    use super::*;

    fn parse(lang: CodeLanguage, src: &str) -> ParsedTree {
        CodeParserAdapter::new(lang).parse(src).expect("parse")
    }

    #[test]
    fn rust_extracts_fns_structs_impls_methods_traits_tests() {
        let src = r#"
pub struct Widget { size: u32 }

pub trait Render { fn render(&self); }

impl Render for Widget {
    fn render(&self) {}
}

pub fn make_widget() -> Widget { Widget { size: 1 } }

mod inner {
    pub const MAX: u32 = 10;
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
"#;
        let tree = parse(CodeLanguage::Rust, src);
        let syms = extract_symbols(&tree, src);
        let by_path: std::collections::HashMap<&str, SymbolKind> = syms
            .iter()
            .map(|s| (s.symbol_path.as_str(), s.kind))
            .collect();
        assert_eq!(by_path.get("Widget"), Some(&SymbolKind::Struct));
        assert_eq!(by_path.get("Render"), Some(&SymbolKind::Trait));
        assert_eq!(by_path.get("Widget"), Some(&SymbolKind::Struct));
        assert_eq!(by_path.get("make_widget"), Some(&SymbolKind::Function));
        assert_eq!(by_path.get("inner"), Some(&SymbolKind::Module));
        assert_eq!(by_path.get("inner::MAX"), Some(&SymbolKind::Constant));
        // Method inside impl carries the impl-target scope.
        assert_eq!(by_path.get("Widget::render"), Some(&SymbolKind::Method));
        // #[test] fn is classified as a test.
        assert_eq!(by_path.get("tests::it_works"), Some(&SymbolKind::Test));
    }

    #[test]
    fn typescript_extracts_functions_classes_interfaces_components_hooks() {
        let src = r#"
export interface Props { title: string }
export class Service { run(): void {} }
export function helper(): number { return 1; }
export const Button = (props: Props) => { return null; };
export const useThing = () => { return 1; };
type Id = string;
enum Color { Red, Green }
"#;
        let tree = parse(CodeLanguage::TypeScript, src);
        let syms = extract_symbols(&tree, src);
        let by_path: std::collections::HashMap<&str, SymbolKind> = syms
            .iter()
            .map(|s| (s.symbol_path.as_str(), s.kind))
            .collect();
        assert_eq!(by_path.get("Props"), Some(&SymbolKind::Interface));
        assert_eq!(by_path.get("Service"), Some(&SymbolKind::Class));
        assert_eq!(by_path.get("Service.run"), Some(&SymbolKind::Method));
        assert_eq!(by_path.get("helper"), Some(&SymbolKind::Function));
        assert_eq!(by_path.get("Button"), Some(&SymbolKind::Component));
        assert_eq!(by_path.get("useThing"), Some(&SymbolKind::Hook));
        assert_eq!(by_path.get("Id"), Some(&SymbolKind::TypeAlias));
        assert_eq!(by_path.get("Color"), Some(&SymbolKind::Enum));
    }

    #[test]
    fn javascript_extracts_functions_classes_const_arrows() {
        let src = r#"
export function compute(a, b) { return a + b; }
class Store { put(k, v) {} }
export const handler = (req) => { return req; };
const PascalThing = () => null;
"#;
        let tree = parse(CodeLanguage::JavaScript, src);
        let syms = extract_symbols(&tree, src);
        let by_path: std::collections::HashMap<&str, SymbolKind> = syms
            .iter()
            .map(|s| (s.symbol_path.as_str(), s.kind))
            .collect();
        assert_eq!(by_path.get("compute"), Some(&SymbolKind::Function));
        assert_eq!(by_path.get("Store"), Some(&SymbolKind::Class));
        assert_eq!(by_path.get("Store.put"), Some(&SymbolKind::Method));
        assert_eq!(by_path.get("handler"), Some(&SymbolKind::Function));
        assert_eq!(by_path.get("PascalThing"), Some(&SymbolKind::Component));
    }

    #[test]
    fn entity_key_is_stable_and_path_scoped() {
        let src = "fn a() {}";
        let tree = parse(CodeLanguage::Rust, src);
        let sym = &extract_symbols(&tree, src)[0];
        assert_eq!(
            sym.entity_key(CodeLanguage::Rust, "src/lib.rs"),
            "rust:src/lib.rs#a"
        );
    }
}
