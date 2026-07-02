//! Integration tests for the MT-001 tree-sitter highlight pipeline (WP-KERNEL-012).
//!
//! AC-002: a ~10-line Rust snippet yields >= 1 Keyword span AND >= 1 Function span.
//! AC-003: a JS snippet yields >= 1 String span.
//! PT-002 runs these via `cargo test -p handshake-native highlighter`; the test fn names contain
//! `highlighter` so that filter selects exactly this proof set.

use handshake_native::code_editor::{HighlightScope, LanguageRegistry};

#[test]
fn highlighter_rust_snippet_has_keyword_and_function_spans() {
    // A 10-line Rust snippet (AC-002).
    let src = "\
use std::fmt;

// compute the sum of two ints
fn compute(x: i32, y: i32) -> i32 {
    let total = add(x, y);
    return total;
}

fn add(a: i32, b: i32) -> i32 {
    a + b
}
";
    let mut hl = LanguageRegistry::with_bundled_languages()
        .highlighter_for_extension("rs")
        .expect("bundled rust highlighter");
    let spans = hl.highlight(src.as_bytes());

    assert!(
        spans.iter().any(|s| s.scope == HighlightScope::Keyword),
        "AC-002: expected at least one Keyword span; spans={spans:?}"
    );
    assert!(
        spans.iter().any(|s| s.scope == HighlightScope::Function),
        "AC-002: expected at least one Function span; spans={spans:?}"
    );

    println!(
        "PASS highlighter_rust: {} spans (keywords={}, functions={})",
        spans.len(),
        spans
            .iter()
            .filter(|s| s.scope == HighlightScope::Keyword)
            .count(),
        spans
            .iter()
            .filter(|s| s.scope == HighlightScope::Function)
            .count(),
    );
}

#[test]
fn highlighter_js_snippet_has_string_span() {
    let src = "\
const msg = \"hello, world\";
function greet(name) {
  console.log(msg);
  return name;
}
";
    let mut hl = LanguageRegistry::with_bundled_languages()
        .highlighter_for_extension("js")
        .expect("bundled js highlighter");
    let spans = hl.highlight(src.as_bytes());

    assert!(
        spans.iter().any(|s| s.scope == HighlightScope::String),
        "AC-003: expected at least one String span; spans={spans:?}"
    );

    println!(
        "PASS highlighter_js: {} spans (strings={})",
        spans.len(),
        spans
            .iter()
            .filter(|s| s.scope == HighlightScope::String)
            .count(),
    );
}

#[test]
fn highlighter_spans_align_to_real_tokens() {
    // The byte ranges must index real keyword/string text in the source (RISK-002 alignment).
    let src = "fn main() { let s = \"hi\"; }";
    let mut hl = LanguageRegistry::with_bundled_languages()
        .highlighter_for_extension("rs")
        .expect("rust highlighter");
    let spans = hl.highlight(src.as_bytes());
    let bytes = src.as_bytes();

    let kw = spans
        .iter()
        .find(|s| s.scope == HighlightScope::Keyword)
        .unwrap();
    let kw_text = std::str::from_utf8(&bytes[kw.byte_range.clone()]).unwrap();
    assert!(
        matches!(kw_text, "fn" | "let"),
        "keyword span must cover a keyword token, got {kw_text:?}"
    );

    let st = spans
        .iter()
        .find(|s| s.scope == HighlightScope::String)
        .unwrap();
    let st_text = std::str::from_utf8(&bytes[st.byte_range.clone()]).unwrap();
    assert_eq!(
        st_text, "\"hi\"",
        "string span must cover the quoted literal"
    );
}

#[test]
fn highlighter_unknown_extension_returns_none() {
    let reg = LanguageRegistry::with_bundled_languages();
    assert!(reg.highlighter_for_extension("unknownext").is_none());
}
