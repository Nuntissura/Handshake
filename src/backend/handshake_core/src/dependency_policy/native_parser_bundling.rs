//! WP-KERNEL-009 / MT-028 — NativeParserBundling proof.
//!
//! Proves the tree-sitter grammars (rust/javascript/typescript) are
//! STATICALLY LINKED into the product binary: the grammar crates compile
//! their C parser tables into the crate at build time (each crate's build.rs
//! invokes cc on the vendored parser.c), so parsing works with
//!   - no network access,
//!   - no external grammar files (.so/.dll/.dylib/.wasm),
//!   - no runtime grammar registry or download path.
//!
//! The proof drives the EXISTING product chunking path
//! (`ai_ready_data::chunking::chunk_code_treesitter`, strategy
//! `code_ast_treesitter_v1`) over in-memory rust/js/ts samples inside a plain
//! `cargo test` process. AST-derived multi-chunk output is only possible when
//! the compiled-in grammar actually parsed the source: the no-AST fallback
//! produces exactly ONE whole-file chunk, and a parse failure returns an
//! error. The negative test proves real parsing (a stub parser would not
//! reject broken syntax).
//!
//! Bundled-library governance for the grammar crates (names, licenses,
//! no-runtime-download reason) lives in the runtime dependency allowlist
//! (`bundled_libraries`, family "tree-sitter") — MT-017/MT-029.

#[cfg(test)]
mod tests {
    use crate::ai_ready_data::chunking::{chunk_code_treesitter, CodeLanguage};
    use crate::dependency_policy::{repo_root_from_manifest_dir, RuntimeDependencyAllowlist};

    const PIPELINE: &str = "test_pipeline_v1";
    const EMBED_ID: &str = "none";
    const EMBED_VERSION: &str = "0";

    const RUST_SAMPLE: &str = r#"use std::collections::HashMap;

pub struct KnowledgeEntry {
    pub id: String,
    pub title: String,
}

pub fn index_entry(map: &mut HashMap<String, KnowledgeEntry>, entry: KnowledgeEntry) {
    map.insert(entry.id.clone(), entry);
}

pub fn entry_count(map: &HashMap<String, KnowledgeEntry>) -> usize {
    map.len()
}
"#;

    const TS_SAMPLE: &str = r#"import { join } from "node:path";

export interface KnowledgeEntry {
  id: string;
  title: string;
}

export function indexEntry(entries: KnowledgeEntry[], entry: KnowledgeEntry): number {
  entries.push(entry);
  return entries.length;
}

export function entryPath(root: string, entry: KnowledgeEntry): string {
  return join(root, entry.id);
}
"#;

    const JS_SAMPLE: &str = r#"const REGISTRY = new Map();

class KnowledgeEntry {
  constructor(id, title) {
    this.id = id;
    this.title = title;
  }
}

function indexEntry(entry) {
  REGISTRY.set(entry.id, entry);
  return REGISTRY.size;
}
"#;

    fn assert_ast_chunked(language: CodeLanguage, source: &str, min_chunks: usize) {
        let chunks = chunk_code_treesitter(
            "bronze_test_ref",
            source,
            language,
            PIPELINE,
            EMBED_ID,
            EMBED_VERSION,
        )
        .expect("statically linked grammar must parse the in-memory sample");

        // AST-derived chunking: more than the single whole-file fallback chunk
        // is only possible when the compiled-in grammar produced a real tree
        // with selected named nodes (fn/struct/class/interface/...).
        assert!(
            chunks.len() >= min_chunks,
            "{language:?}: expected >= {min_chunks} AST chunks, got {} — \
             grammar did not parse (fallback or empty selection)",
            chunks.len()
        );
        for chunk in &chunks {
            assert_eq!(chunk.strategy_id, "code_ast_treesitter_v1");
            assert!(chunk.byte_end > chunk.byte_start);
            assert_eq!(chunk.text, &source[chunk.byte_start..chunk.byte_end]);
        }
        // Imports ride with the first chunk: proves real node byte ranges were
        // post-processed, not a line-split heuristic.
        assert_eq!(chunks[0].byte_start, 0);
    }

    #[test]
    fn rust_grammar_is_statically_linked_and_parses() {
        // 1 struct + 2 fns → >= 3 selected nodes.
        assert_ast_chunked(CodeLanguage::Rust, RUST_SAMPLE, 3);
    }

    #[test]
    fn typescript_grammar_is_statically_linked_and_parses() {
        // 1 interface + 2 exported fns → >= 3 selected nodes.
        assert_ast_chunked(CodeLanguage::TypeScript, TS_SAMPLE, 3);
    }

    #[test]
    fn javascript_grammar_is_statically_linked_and_parses() {
        // 1 class + 1 fn → >= 2 selected nodes.
        assert_ast_chunked(CodeLanguage::JavaScript, JS_SAMPLE, 2);
    }

    #[test]
    fn grammars_reject_broken_syntax_proving_real_parsing() {
        // A stub or pass-through "parser" would happily chunk this; the real
        // compiled-in grammar must surface a parse error.
        let broken = "pub fn unterminated(((( {";
        let result = chunk_code_treesitter(
            "bronze_test_ref",
            broken,
            CodeLanguage::Rust,
            PIPELINE,
            EMBED_ID,
            EMBED_VERSION,
        );
        assert!(result.is_err(), "broken syntax must fail the AST parse");
    }

    #[test]
    fn compiled_in_grammars_expose_real_node_tables() {
        // The grammar tables live in the binary itself: each Language exposes
        // a populated node-kind table without any file or network access.
        for (name, language) in [
            (
                "rust",
                tree_sitter::Language::from(tree_sitter_rust::LANGUAGE),
            ),
            (
                "javascript",
                tree_sitter::Language::from(tree_sitter_javascript::LANGUAGE),
            ),
            (
                "typescript",
                tree_sitter::Language::from(tree_sitter_typescript::LANGUAGE_TYPESCRIPT),
            ),
        ] {
            assert!(
                language.node_kind_count() > 50,
                "{name}: compiled-in grammar table looks empty"
            );
        }
    }

    #[test]
    fn no_external_grammar_artifacts_in_product_scan_roots() {
        // No runtime-loadable grammar binaries may ship in product source
        // trees; grammars exist only as compiled-in tables.
        let repo_root = repo_root_from_manifest_dir();
        let allowlist =
            RuntimeDependencyAllowlist::load_from_repo_root(&repo_root).expect("allowlist loads");
        let mut offenders = Vec::new();
        for root in &allowlist.product_scan_roots {
            let root_path = repo_root.join(root.replace('/', std::path::MAIN_SEPARATOR_STR));
            let mut stack = vec![root_path];
            while let Some(dir) = stack.pop() {
                let Ok(entries) = std::fs::read_dir(&dir) else {
                    continue;
                };
                for entry in entries.flatten() {
                    let path = entry.path();
                    let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
                    if path.is_dir() {
                        if name != "node_modules" && name != "target" && name != "dist" {
                            stack.push(path);
                        }
                        continue;
                    }
                    let is_loadable_binary = [".so", ".dll", ".dylib", ".wasm"]
                        .iter()
                        .any(|ext| name.ends_with(ext));
                    if is_loadable_binary && (name.contains("tree") || name.contains("grammar")) {
                        offenders.push(path.display().to_string());
                    }
                }
            }
        }
        assert!(
            offenders.is_empty(),
            "runtime-loadable grammar artifacts found in product trees: {offenders:?}"
        );
    }
}
