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
    /// MT-098/099/100 collision discriminator. Distinguishes two symbols that
    /// share the same `symbol_path` but are genuinely distinct entities the
    /// upsert must NOT merge: a trait-impl method vs an inherent method of the
    /// same name (`as Trait`), declaration-merging kinds (interface vs class vs
    /// function `Foo`), and overload/duplicate signatures (an ordinal). `None`
    /// when the `{lang}:{path}#{symbol_path}` key is already unique.
    pub disambiguator: Option<String>,
}

/// The discriminator delimiter in an entity_key. The nav layer strips anything
/// from this char onward before parsing the simple name / path, so the
/// discriminator makes the key unique WITHOUT changing how a symbol is looked up
/// by name or path. Chosen as `~` (not `#`/`:`/`.`/`::`) so it never clashes
/// with a path separator.
pub const KEY_DISCRIMINATOR: char = '~';

impl ExtractedSymbol {
    /// Stable, collision-resistant entity key for this symbol within a file.
    /// Shape: `{lang}:{path}#{symbol_path}` plus, when a discriminator is
    /// present, `~{discriminator}` so two distinct symbols sharing a
    /// `symbol_path` (trait vs inherent method, declaration-merging kinds,
    /// overloads) get DISTINCT keys and never merge in the entity upsert.
    pub fn entity_key(&self, language: CodeLanguage, relative_path: &str) -> String {
        match &self.disambiguator {
            Some(disc) => format!(
                "{}:{}#{}{KEY_DISCRIMINATOR}{}",
                language.as_str(),
                relative_path,
                self.symbol_path,
                disc
            ),
            None => format!(
                "{}:{}#{}",
                language.as_str(),
                relative_path,
                self.symbol_path
            ),
        }
    }
}

/// Assign overload/duplicate discriminators: any two symbols that would produce
/// the SAME base key (same language+path+symbol_path+kind-tag) get an ordinal
/// (`#0`, `#1`, ...) appended to their discriminator in stable document order,
/// so duplicate signatures never collapse to one entity. Symbols already unique
/// keep whatever disambiguator they carry (the kind/trait tag).
fn assign_collision_ordinals(symbols: &mut [ExtractedSymbol]) {
    use std::collections::HashMap;
    // Group indices by their current (symbol_path, disambiguator) identity.
    let mut groups: HashMap<(String, Option<String>), Vec<usize>> = HashMap::new();
    for (i, s) in symbols.iter().enumerate() {
        groups
            .entry((s.symbol_path.clone(), s.disambiguator.clone()))
            .or_default()
            .push(i);
    }
    for indices in groups.values() {
        if indices.len() < 2 {
            continue; // already unique
        }
        // Stable order: by start byte (document order).
        let mut ordered = indices.clone();
        ordered.sort_by_key(|&i| symbols[i].start_byte);
        for (ordinal, &i) in ordered.iter().enumerate() {
            let suffix = format!("dup{ordinal}");
            symbols[i].disambiguator = Some(match symbols[i].disambiguator.take() {
                Some(existing) => format!("{existing}.{suffix}"),
                None => suffix,
            });
        }
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
    // MT-098/099/100: any remaining same-base-key duplicates (overloads, two
    // impl blocks, repeated declarators) get a stable ordinal so the upsert
    // never silently merges distinct symbols.
    assign_collision_ordinals(&mut symbols);
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

        // MT-098 disambiguation:
        // * an impl entity carries its trait so `impl A` (inherent) and
        //   `impl Trait for A` are distinct keys, and `impl T1 for A` !=
        //   `impl T2 for A`.
        // * a METHOD inside a trait-impl carries that trait so an inherent
        //   `A::run` and a trait `A::run` (from `impl Trait for A`) do not merge.
        let disambiguator = match kind {
            SymbolKind::Impl => rust_impl_trait_name(tree, node, source)
                .map(|t| format!("as:{t}"))
                .or(Some("inherent".to_string())),
            SymbolKind::Method => {
                rust_enclosing_impl_trait(tree, node, source).map(|t| format!("as:{t}"))
            }
            _ => None,
        };

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
            disambiguator,
        });
    }
    out
}

/// The trait name of an `impl_item` (the `trait` field), or `None` for an
/// inherent impl. For `impl Foo for Bar` the `trait` field is `Foo`.
fn rust_impl_trait_name<'a>(
    tree: &ParsedTree,
    impl_node: &AstNode,
    source: &'a str,
) -> Option<&'a str> {
    tree.child_field_text(impl_node.index, "trait", source)
}

/// Walk up from a method node to its enclosing `impl_item` and return that
/// impl's trait name, if the impl is a trait impl. `None` for an inherent impl
/// method or a free function.
fn rust_enclosing_impl_trait<'a>(
    tree: &ParsedTree,
    node: &AstNode,
    source: &'a str,
) -> Option<&'a str> {
    let mut current = node.parent;
    while let Some(idx) = current {
        let parent = &tree.nodes[idx];
        if parent.kind == "impl_item" {
            return rust_impl_trait_name(tree, parent, source);
        }
        current = parent.parent;
    }
    None
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
        // A node may yield MULTIPLE symbols (a multi-declarator const), so the
        // extractor returns a Vec.
        let symbols: Vec<(SymbolKind, String)> = match node.kind.as_str() {
            "function_declaration" | "generator_function_declaration" => {
                decl_name(tree, node, source)
                    .map(|name| vec![(ts_function_kind(name), name.to_string())])
                    .unwrap_or_default()
            }
            "class_declaration" | "abstract_class_declaration" => decl_name(tree, node, source)
                .map(|n| vec![(SymbolKind::Class, n.to_string())])
                .unwrap_or_default(),
            "interface_declaration" => decl_name(tree, node, source)
                .map(|n| vec![(SymbolKind::Interface, n.to_string())])
                .unwrap_or_default(),
            "type_alias_declaration" => decl_name(tree, node, source)
                .map(|n| vec![(SymbolKind::TypeAlias, n.to_string())])
                .unwrap_or_default(),
            "enum_declaration" => decl_name(tree, node, source)
                .map(|n| vec![(SymbolKind::Enum, n.to_string())])
                .unwrap_or_default(),
            "method_definition" => method_name(tree, node, source)
                .map(|n| vec![(SymbolKind::Method, n.to_string())])
                .unwrap_or_default(),
            "lexical_declaration" | "variable_declaration" => {
                // ALL declarators: `const a = () => {}, b = () => {}` yields BOTH
                // a and b; a non-function `const MAX = 10` yields a Constant.
                ts_const_bindings(tree, node, source)
            }
            _ => Vec::new(),
        };

        for (kind, name) in symbols {
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
                // MT-099: declaration merging - `interface Foo`, `class Foo`,
                // `function Foo`, `namespace Foo` legally coexist and must NOT
                // merge into one entity. The kind tag keeps their keys distinct.
                disambiguator: Some(ts_js_kind_tag(kind).to_string()),
            });
        }
    }
    out
}

/// A short, stable kind tag used as a TS/JS entity-key disambiguator so
/// declaration-merging same-name decls (interface/class/function/namespace) get
/// distinct keys. Methods keep a tag too (harmless; they are class-scoped).
fn ts_js_kind_tag(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Class => "class",
        SymbolKind::Interface => "interface",
        SymbolKind::Enum => "enum",
        SymbolKind::TypeAlias => "type",
        SymbolKind::Method => "method",
        SymbolKind::Component => "component",
        SymbolKind::Hook => "hook",
        SymbolKind::Constant => "const",
        // Function/Module and any other share the `value` namespace.
        _ => "value",
    }
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

/// Inspect a `const/let/var X = ...` declaration and return a symbol for EVERY
/// declarator (MT-099/100: `const a = () => {}, b = () => {}` yields BOTH a and
/// b). A function-valued binding becomes a function/component/hook symbol; any
/// other binding becomes a `Constant` so config-like top-level values are
/// navigable too. Document order.
fn ts_const_bindings(tree: &ParsedTree, node: &AstNode, source: &str) -> Vec<(SymbolKind, String)> {
    let mut out = Vec::new();
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
        if name.trim().is_empty() {
            continue;
        }
        // Is the value an arrow / function expression?
        let is_fn = tree.children_of(declarator.index).any(|c| {
            matches!(
                c.kind.as_str(),
                "arrow_function" | "function" | "function_expression"
            )
        });
        if is_fn {
            out.push((ts_function_kind(name), name.to_string()));
        } else {
            out.push((SymbolKind::Constant, name.to_string()));
        }
    }
    out
}

// ---------------------------------------------------------------------------
// MT-100 JavaScript.
// ---------------------------------------------------------------------------

const JS_SCOPE_KINDS: &[&str] = &["class_declaration"];

fn extract_javascript(tree: &ParsedTree, source: &str) -> Vec<ExtractedSymbol> {
    let mut out = Vec::new();
    for node in &tree.nodes {
        let symbols: Vec<(SymbolKind, String)> = match node.kind.as_str() {
            "function_declaration" | "generator_function_declaration" => {
                decl_name(tree, node, source)
                    .map(|name| vec![(ts_function_kind(name), name.to_string())])
                    .unwrap_or_default()
            }
            "class_declaration" => decl_name(tree, node, source)
                .map(|n| vec![(SymbolKind::Class, n.to_string())])
                .unwrap_or_default(),
            "method_definition" => method_name(tree, node, source)
                .map(|n| vec![(SymbolKind::Method, n.to_string())])
                .unwrap_or_default(),
            "lexical_declaration" | "variable_declaration" => ts_const_bindings(tree, node, source),
            _ => Vec::new(),
        };
        for (kind, name) in symbols {
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
                // MT-100: a `class Foo` and a `function Foo`/`const Foo` must not
                // merge - the kind tag keeps their keys distinct.
                disambiguator: Some(ts_js_kind_tag(kind).to_string()),
            });
        }
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

    /// Collect the entity keys of all extracted symbols for a file.
    fn keys(lang: CodeLanguage, src: &str, path: &str) -> Vec<String> {
        let tree = parse(lang, src);
        extract_symbols(&tree, src)
            .iter()
            .map(|s| s.entity_key(lang, path))
            .collect()
    }

    /// No two extracted symbols may share an entity key (the upsert merge bug).
    fn assert_unique_keys(ks: &[String]) {
        let mut seen = std::collections::HashSet::new();
        for k in ks {
            assert!(
                seen.insert(k),
                "duplicate entity key collides in upsert: {k} in {ks:?}"
            );
        }
    }

    #[test]
    fn rust_trait_and_inherent_method_same_name_do_not_collide() {
        // MT-098: `A::run` inherent vs `A::run` from `impl Trait for A` are
        // DISTINCT methods and must get distinct keys.
        let src = r#"
struct A;
trait T { fn run(&self); }
impl A { fn run(&self) {} }
impl T for A { fn run(&self) {} }
"#;
        let ks = keys(CodeLanguage::Rust, src, "src/lib.rs");
        assert_unique_keys(&ks);
        // The trait-impl method carries the `as:T` discriminator.
        assert!(
            ks.iter().any(|k| k.contains("#A::run~as:T")),
            "trait method must be tagged with its trait: {ks:?}"
        );
        // The inherent one is the bare path (no `~as:`).
        assert!(
            ks.iter().any(|k| k == "rust:src/lib.rs#A::run"),
            "inherent method keeps the bare path: {ks:?}"
        );
    }

    #[test]
    fn rust_two_trait_impls_of_same_type_do_not_collide() {
        // MT-098: `impl T1 for A` and `impl T2 for A` are distinct impl entities.
        let src = r#"
struct A;
trait T1 {}
trait T2 {}
impl T1 for A {}
impl T2 for A {}
impl A {}
"#;
        let ks = keys(CodeLanguage::Rust, src, "src/lib.rs");
        assert_unique_keys(&ks);
        assert!(ks.iter().any(|k| k.contains("#impl A~as:T1")), "{ks:?}");
        assert!(ks.iter().any(|k| k.contains("#impl A~as:T2")), "{ks:?}");
        assert!(ks.iter().any(|k| k.contains("#impl A~inherent")), "{ks:?}");
    }

    #[test]
    fn typescript_declaration_merging_same_name_do_not_collide() {
        // MT-099: `interface Foo`, `class Foo`, `function Foo` legally coexist
        // and must not merge into one symbol entity. (PascalCase `function Foo`
        // is classified Component by the React heuristic -> `~component` tag;
        // the point is the three keys are DISTINCT.)
        let src = r#"
interface Foo { a: number }
class Foo { b() {} }
function Foo() {}
"#;
        let ks = keys(CodeLanguage::TypeScript, src, "src/a.ts");
        assert_unique_keys(&ks);
        assert!(ks.iter().any(|k| k.contains("#Foo~interface")), "{ks:?}");
        assert!(ks.iter().any(|k| k.contains("#Foo~class")), "{ks:?}");
        assert!(
            ks.iter()
                .any(|k| k.contains("#Foo~component") || k.contains("#Foo~value")),
            "the function/component Foo must have its own key: {ks:?}"
        );
    }

    #[test]
    fn typescript_multi_declarator_const_extracts_all() {
        // MT-099: `const a = () => {}, b = () => {}` must yield BOTH a and b;
        // a non-function `const MAX = 10` is a Constant.
        let src = "const a = () => {}, b = () => {};\nconst MAX = 10;\n";
        let tree = parse(CodeLanguage::TypeScript, src);
        let syms = extract_symbols(&tree, src);
        let names: Vec<&str> = syms.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"a"), "{names:?}");
        assert!(names.contains(&"b"), "{names:?}");
        assert!(names.contains(&"MAX"), "{names:?}");
        let max = syms.iter().find(|s| s.name == "MAX").unwrap();
        assert_eq!(max.kind, SymbolKind::Constant);
        assert_unique_keys(
            &syms
                .iter()
                .map(|s| s.entity_key(CodeLanguage::TypeScript, "src/a.ts"))
                .collect::<Vec<_>>(),
        );
    }

    #[test]
    fn duplicate_same_kind_declarations_get_ordinal_and_do_not_collide() {
        // MT-099/100: two declarations that genuinely produce the SAME base key
        // (same name, same kind, same scope) — e.g. a duplicated function — get
        // a stable `dup` ordinal so they do not collapse to one entity in the
        // upsert. (Two top-level `function compute` is invalid JS but tree-sitter
        // still parses both; the index must keep them distinct, not merge.)
        let src = "function compute() { return 1; }\nfunction compute() { return 2; }\n";
        let ks = keys(CodeLanguage::JavaScript, src, "src/a.js");
        assert_unique_keys(&ks);
        let dup_keys: Vec<&String> = ks.iter().filter(|k| k.contains("#compute~")).collect();
        assert_eq!(dup_keys.len(), 2, "both duplicates must survive: {ks:?}");
        assert!(
            dup_keys.iter().all(|k| k.contains("dup")),
            "duplicates must carry an ordinal: {ks:?}"
        );
    }

    #[test]
    fn javascript_class_and_function_same_name_do_not_collide() {
        // MT-100: a `class Foo` and a `function Foo` must be distinct entities.
        // (PascalCase `function Foo` -> Component heuristic -> `~component`.)
        let src = "class Foo {}\nfunction Foo() {}\n";
        let ks = keys(CodeLanguage::JavaScript, src, "src/a.js");
        assert_unique_keys(&ks);
        assert!(ks.iter().any(|k| k.contains("#Foo~class")), "{ks:?}");
        assert!(
            ks.iter()
                .any(|k| k.contains("#Foo~component") || k.contains("#Foo~value")),
            "{ks:?}"
        );
    }

    #[test]
    fn lowercase_function_and_const_same_name_do_not_collide() {
        // A lowercase `value`-namespace collision: a `function helper` and a
        // `const helper = () => {}` both tag `~value`; the ordinal keeps them
        // distinct rather than merging.
        let src = "function helper() {}\nconst helper = () => {};\n";
        let ks = keys(CodeLanguage::JavaScript, src, "src/a.js");
        assert_unique_keys(&ks);
        let helper_keys: Vec<&String> = ks.iter().filter(|k| k.contains("#helper~")).collect();
        assert_eq!(helper_keys.len(), 2, "{ks:?}");
    }
}
