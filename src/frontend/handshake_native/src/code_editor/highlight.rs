//! tree-sitter syntax-highlight pipeline for the native code editor (WP-KERNEL-012 MT-001).
//!
//! [`Highlighter`] parses a source buffer with tree-sitter and projects the syntax tree into a flat
//! list of [`HighlightSpan`]s (byte range + semantic [`HighlightScope`]). The panel maps each scope
//! to a theme color via [`scope_to_color`] (no hardcoded hex — theme layer owns the palette).
//!
//! ## Why a query, not the C `highlights.scm` highlighter API
//!
//! tree-sitter ships a `tree_sitter_highlight` crate, but it pulls a separate config + injection
//! machinery this MT does not need. Instead we run a small per-language `Query` of node patterns and
//! map each capture name to a [`HighlightScope`]. The capture-name vocabulary (`keyword`, `string`,
//! `comment`, `number`, `function`, `type`, `operator`) is the standard tree-sitter highlight
//! capture set, so the queries here are a minimal subset of each grammar's own `highlights.scm`.
//!
//! ## Incremental re-parse (implementation note 2 / RISK perf)
//!
//! [`Highlighter`] caches the previous `Tree`. After the first parse, [`highlight`](Highlighter::highlight)
//! passes `Some(&old_tree)` to `Parser::parse`, so an edit re-parses in O(edit) rather than
//! re-parsing the whole document.
//!
//! ## Send + Sync `Language` (RISK-005)
//!
//! tree-sitter `Language` is `Send + Sync` in the 0.25 line, but the egui event loop may move work
//! across threads; to be robust against a grammar/version where that is not guaranteed, the language
//! is wrapped behind [`SafeLanguage`] (an `Arc`-shareable newtype) so the highlighter is `Send`. The
//! `Parser` itself is NOT `Sync` (it holds mutable parse state), which is correct: a `Highlighter`
//! owns its parser and is used from one place at a time.
//!
//! ## tree-sitter 0.25 + `tree-sitter-language` LanguageFn shim (research-corrected stack)
//!
//! Grammars are loaded through the `tree-sitter-language` `LanguageFn` shim: each grammar crate
//! exports `LANGUAGE: LanguageFn`, converted to a [`Language`] via `LANGUAGE.into()` — NOT the
//! deprecated `tree_sitter_*::language()` fn. The shim decouples grammar version from the single
//! pinned `tree-sitter` 0.25 core (PT-005), which is what lets more Monaco-parity languages be added
//! later without a duplicate-core/ABI wall. In 0.25 `QueryCursor::matches` returns a
//! [`StreamingIterator`] (the C cursor mutates on each step), so spans are collected with a
//! `while let Some(m) = matches.next()` loop rather than a plain `for`.

use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;

use tree_sitter::{Language, Parser, Query, QueryCursor, StreamingIterator, Tree};

use crate::theme::{SyntaxPaletteEntry, MUTED_PALETTE, STANDARD_PALETTE};

/// Semantic highlight class for a span of source text. The panel maps each variant to a theme color
/// (`theme::HsSyntaxTokens`) — variants are deliberately a small, stable set rather than raw
/// tree-sitter capture strings so the renderer's color table is exhaustive and theme-driven.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HighlightScope {
    Keyword,
    String,
    Comment,
    Number,
    Function,
    Type,
    Operator,
    Other,
}

impl HighlightScope {
    /// Map a tree-sitter capture name (e.g. `"keyword"`, `"string.special"`, `"function.method"`)
    /// to a scope. Matches on the leading dotted segment so sub-captures
    /// (`function.method`, `string.escape`) fold into their base scope. Unknown captures -> `Other`.
    pub fn from_capture_name(name: &str) -> Self {
        let base = name.split('.').next().unwrap_or(name);
        match base {
            "keyword" => HighlightScope::Keyword,
            "string" => HighlightScope::String,
            "comment" => HighlightScope::Comment,
            "number" => HighlightScope::Number,
            // tree-sitter uses both "function" and "constructor"/"method" sub-captures for callables.
            "function" | "constructor" | "method" => HighlightScope::Function,
            "type" => HighlightScope::Type,
            "operator" => HighlightScope::Operator,
            _ => HighlightScope::Other,
        }
    }

    /// All eight scope variants, in declaration order. Lets a palette-completeness test iterate every
    /// scope (AC-004) without re-listing the variants.
    pub const ALL: &'static [HighlightScope] = &[
        HighlightScope::Keyword,
        HighlightScope::String,
        HighlightScope::Comment,
        HighlightScope::Number,
        HighlightScope::Function,
        HighlightScope::Type,
        HighlightScope::Operator,
        HighlightScope::Other,
    ];

    /// The stable lowercase string key for this scope, used in the persisted
    /// `syntax_palette.custom` map (see `crate::workspace_settings::SYNTAX_SCOPE_KEYS`). Round-trips
    /// with [`from_scope_key`](HighlightScope::from_scope_key).
    pub fn scope_key(self) -> &'static str {
        match self {
            HighlightScope::Keyword => "keyword",
            HighlightScope::String => "string",
            HighlightScope::Comment => "comment",
            HighlightScope::Number => "number",
            HighlightScope::Function => "function",
            HighlightScope::Type => "type",
            HighlightScope::Operator => "operator",
            HighlightScope::Other => "other",
        }
    }

    /// Parse a scope key (the [`scope_key`](HighlightScope::scope_key) form) back to a scope; `None`
    /// for an unknown key.
    pub fn from_scope_key(key: &str) -> Option<Self> {
        HighlightScope::ALL
            .iter()
            .copied()
            .find(|s| s.scope_key() == key)
    }

    /// Index a built-in [`SyntaxPaletteEntry`] color by this scope (MT-072). The mapping is total —
    /// every variant resolves to a concrete field, so there is NO missing scope and NO panic on lookup
    /// (AC-004).
    pub fn builtin_color(self, entry: &SyntaxPaletteEntry) -> egui::Color32 {
        match self {
            HighlightScope::Keyword => entry.keyword,
            HighlightScope::String => entry.string,
            HighlightScope::Comment => entry.comment,
            HighlightScope::Number => entry.number,
            HighlightScope::Function => entry.function,
            HighlightScope::Type => entry.type_name,
            HighlightScope::Operator => entry.operator,
            HighlightScope::Other => entry.other,
        }
    }
}

/// Resolve the color the code editor should paint for `scope` given the active [`SyntaxPalette`]
/// (WP-KERNEL-012 MT-072). This is the LIVE palette path: the code editor calls it every frame, so a
/// Custom swatch edit takes effect in the SAME frame with no restart (AC-003).
///
/// Resolution:
/// - `Muted`    -> the [`MUTED_PALETTE`] built-in color for the scope;
/// - `Standard` -> the [`STANDARD_PALETTE`] built-in color for the scope;
/// - `Custom`   -> the user's per-scope sRGBA override if present (converted via
///   `Color32::from_rgba_unmultiplied` — the CONTROL-4-sanctioned RGBA form), ELSE the
///   [`STANDARD_PALETTE`] color for that scope (so an un-overridden scope is never missing — AC-004).
///
/// Every mode resolves EVERY scope to a concrete color: there is no gap and no panic on any
/// `HighlightScope` lookup (AC-004 — a test iterates all variants for all three modes).
///
/// TYPED FOLLOW-UP BLOCKER (panel.rs + minimap.rs are OUTSIDE MT-072's allowed_paths, and unlike the
/// editor-prefs + code-keymap wiring there is NO public `&self` seam on the mounted panel to inject the
/// palette into the per-frame render): the live render call sites
/// `code_editor::panel::scope_to_color(scope, &HsSyntaxTokens)` (panel.rs single-line + multi-line draw
/// paths) and `minimap::scope_to_color` still read the theme's fixed `HsSyntaxTokens`, so a Custom
/// swatch edit changes THIS seam's resolved color (proven live in the MT tests) but does NOT yet repaint
/// the running editor. Routing those render sites through `resolve_scope_color(scope, &syntax_palette)`
/// requires editing panel.rs/minimap.rs (a render-call-site change with no settings-flow seam to route
/// through), which is the editor-highlight wiring follow-up packet recorded in the MT handoff. AC-003 is
/// therefore SEAM-PROVEN (this function reacts live), NOT live-proven at the editor surface; the WP
/// validator must record AC-003 as seam-only, not GREEN. This function is the seam that follow-up routes
/// through, and it is directly proven live here + in the MT tests.
///
/// (Contrast: editor PREFS — tab size / insert-spaces / render-whitespace / word-wrap — and the CODE
/// keybinding overrides ARE wired into the live editor in `app.rs` via the panel's existing public
/// `set_indent_settings` / `set_render_whitespace` / `set_wrap_*` / `reload_keymap_from_settings`
/// setters, so those are live-proven. Only the syntax-color render path, the editor FONT SIZE — no panel
/// slot — and RICH keybinding overrides — no live rich keymap seam — remain typed follow-up blockers.)
pub fn resolve_scope_color(
    scope: HighlightScope,
    palette: &crate::workspace_settings::SyntaxPalette,
) -> egui::Color32 {
    use crate::workspace_settings::SyntaxPaletteMode;
    match palette.mode {
        SyntaxPaletteMode::Muted => scope.builtin_color(&MUTED_PALETTE),
        SyntaxPaletteMode::Standard => scope.builtin_color(&STANDARD_PALETTE),
        SyntaxPaletteMode::Custom => match palette.custom_for(scope.scope_key()) {
            // A Custom override: sRGBA [u8;4] -> Color32 (the sanctioned RGBA construction form).
            Some([r, g, b, a]) => egui::Color32::from_rgba_unmultiplied(r, g, b, a),
            // No override for this scope -> fall back to Standard (no missing scope — AC-004).
            None => scope.builtin_color(&STANDARD_PALETTE),
        },
    }
}

/// One highlighted span: a half-open BYTE range and its semantic scope. Byte-addressed so it aligns
/// directly with tree-sitter (which is byte-native) and with [`super::buffer::TextBuffer`]'s public
/// byte API; the renderer converts to char offsets via `TextBuffer::byte_to_char` before slicing
/// (RISK-002).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HighlightSpan {
    pub byte_range: Range<usize>,
    pub scope: HighlightScope,
}

/// A `Send + Sync` wrapper over a tree-sitter `Language` (RISK-005). `Language` is `Clone` and cheap
/// to clone (it is an `Arc`-like handle internally); wrapping it in `Arc` here makes the registry and
/// highlighter trivially shareable across threads even if a future grammar/version weakens the
/// auto-trait guarantees.
#[derive(Clone)]
pub struct SafeLanguage(Arc<Language>);

impl SafeLanguage {
    pub fn new(language: Language) -> Self {
        Self(Arc::new(language))
    }

    /// The underlying tree-sitter language (cloned cheaply for `Parser::set_language`).
    pub fn language(&self) -> Language {
        (*self.0).clone()
    }
}

// `Language` is already Send + Sync in tree-sitter 0.25; the explicit wrapper + Arc keeps the
// highlighter Send-robust without relying on that holding in every grammar version (RISK-005).

/// Maps a file extension (or language id) to a tree-sitter language + its highlight query source.
/// Keyed on the lowercased extension; built once at startup and shared. Adding a language is a single
/// `register` call (extensible seam for the wider language set ported from the React registry).
pub struct LanguageRegistry {
    by_ext: HashMap<String, RegisteredLanguage>,
}

/// A stable, language-family id used by downstream syntax-tree consumers (MT-005 folding's
/// per-language foldable-node table) to select language-specific behavior without re-deriving it from
/// the file extension. Maps the bundled grammars to the tree-sitter grammar family name (`"rust"` /
/// `"javascript"`).
pub fn language_id_for_extension(ext: &str) -> Option<&'static str> {
    match ext.to_ascii_lowercase().as_str() {
        "rs" => Some("rust"),
        "js" | "jsx" | "mjs" | "cjs" => Some("javascript"),
        _ => None,
    }
}

/// A language + the highlight query text used to derive spans for it.
#[derive(Clone)]
struct RegisteredLanguage {
    language: SafeLanguage,
    /// tree-sitter highlight query source (a minimal subset of the grammar's `highlights.scm`).
    query_src: Arc<str>,
    /// The stable language-family id ([`language_id_for_extension`]) so consumers that need the
    /// language (folding's node-type table — MT-005) can read it off the highlighter.
    language_id: &'static str,
}

impl LanguageRegistry {
    /// An empty registry. Use [`with_bundled_languages`](Self::with_bundled_languages) for the
    /// default Rust + JavaScript set.
    pub fn new() -> Self {
        Self { by_ext: HashMap::new() }
    }

    /// The default registry bundling `tree-sitter-rust` and `tree-sitter-javascript`, mapped to the
    /// extensions the React language registry used for those languages (`rs`; `js`/`jsx`/`mjs`/`cjs`).
    /// More languages plug in later through [`register`](Self::register).
    ///
    /// Grammars are loaded through the `tree-sitter-language` LanguageFn shim: each grammar crate
    /// exports `LANGUAGE: LanguageFn`, converted to a [`Language`] via `LANGUAGE.into()` (the
    /// research-corrected 0.25 stack — NOT the deprecated `tree_sitter_*::language()` fn).
    ///
    /// Each language uses the grammar crate's OWN shipped `highlights.scm`
    /// (`tree_sitter_rust::HIGHLIGHTS_QUERY` / `tree_sitter_javascript::HIGHLIGHT_QUERY`) rather than a
    /// hand-listed token query. The shipped query is guaranteed to compile against the exact grammar
    /// version pinned in `Cargo.lock` (a hand-listed anonymous-token query breaks across versions — e.g.
    /// `tree-sitter-rust` represents `mut` as a named `mutable_specifier`, not an anonymous token,
    /// so `"mut"` in a literal-keyword list would fail `Query::new` with a `NodeType` error). The shipped
    /// queries use the standard tree-sitter capture vocabulary ([`HighlightScope::from_capture_name`]
    /// maps `keyword`/`string`/`function`/`type`/... and folds the rest to [`HighlightScope::Other`]).
    pub fn with_bundled_languages() -> Self {
        let mut reg = Self::new();
        reg.register(
            &["rs"],
            SafeLanguage::new(tree_sitter_rust::LANGUAGE.into()),
            tree_sitter_rust::HIGHLIGHTS_QUERY,
        );
        reg.register(
            &["js", "jsx", "mjs", "cjs"],
            SafeLanguage::new(tree_sitter_javascript::LANGUAGE.into()),
            tree_sitter_javascript::HIGHLIGHT_QUERY,
        );
        reg
    }

    /// Register `language` (with its highlight `query_src`) for one or more file extensions.
    /// Extensions are stored lowercased so lookup is case-insensitive. The language-family id is
    /// derived from the first extension via [`language_id_for_extension`] (falling back to `""` for an
    /// unmapped extension), so the highlighter can report its language to folding (MT-005) without a
    /// second lookup.
    pub fn register(&mut self, extensions: &[&str], language: SafeLanguage, query_src: &str) {
        let language_id = extensions
            .first()
            .and_then(|ext| language_id_for_extension(ext))
            .unwrap_or("");
        let entry = RegisteredLanguage {
            language,
            query_src: Arc::from(query_src),
            language_id,
        };
        for ext in extensions {
            self.by_ext.insert(ext.to_ascii_lowercase(), entry.clone());
        }
    }

    /// Look up a language by file extension (case-insensitive). Returns `None` for an unknown
    /// extension so the panel can fall back to plain (unhighlighted) text rather than guessing.
    fn get(&self, ext: &str) -> Option<&RegisteredLanguage> {
        self.by_ext.get(&ext.to_ascii_lowercase())
    }

    /// Build a [`Highlighter`] for `ext`, or `None` if the extension is unregistered or the language
    /// fails to load into a parser (never panics — a bad grammar degrades to no highlighting).
    pub fn highlighter_for_extension(&self, ext: &str) -> Option<Highlighter> {
        let entry = self.get(ext)?;
        Highlighter::with_language_id(entry.language.clone(), &entry.query_src, entry.language_id)
    }
}

impl Default for LanguageRegistry {
    fn default() -> Self {
        Self::with_bundled_languages()
    }
}

/// Owns a tree-sitter `Parser` + compiled highlight `Query` for one language, plus the cached
/// previous `Tree` for incremental re-parse. Created via [`LanguageRegistry::highlighter_for_extension`]
/// or [`Highlighter::new`] directly.
pub struct Highlighter {
    parser: Parser,
    query: Query,
    /// Cached previous parse tree; passed to `Parser::parse` for O(edit) incremental re-parse, and
    /// exposed via [`tree`](Highlighter::tree) so MT-005 folding can derive fold regions from the SAME
    /// syntax tree (no second parse).
    old_tree: Option<Tree>,
    /// The stable language-family id ([`language_id_for_extension`]) this highlighter parses, so a
    /// consumer that needs the language (folding's foldable-node table — MT-005) can read it off the
    /// highlighter rather than re-deriving it from the extension. `""` when the language is unmapped.
    language_id: &'static str,
}

impl Highlighter {
    /// Build a highlighter from a language + its highlight query source. Returns `None` (never
    /// panics) if the language cannot be set on the parser or the query fails to compile against the
    /// grammar — a defensive boundary so a grammar/query mismatch degrades to "no highlighting"
    /// rather than aborting (AC-006 spirit: fallible setup returns Option). The language id is unknown
    /// (`""`) on this path; use [`with_language_id`](Highlighter::with_language_id) to carry it.
    pub fn new(language: SafeLanguage, query_src: &str) -> Option<Self> {
        Self::with_language_id(language, query_src, "")
    }

    /// Like [`new`](Highlighter::new) but records the language-family id so folding (MT-005) can read
    /// it via [`language_id`](Highlighter::language_id).
    pub fn with_language_id(
        language: SafeLanguage,
        query_src: &str,
        language_id: &'static str,
    ) -> Option<Self> {
        let lang = language.language();
        let mut parser = Parser::new();
        parser.set_language(&lang).ok()?;
        let query = Query::new(&lang, query_src).ok()?;
        Some(Self {
            parser,
            query,
            old_tree: None,
            language_id,
        })
    }

    /// The most recent parse [`Tree`], or `None` before the first [`highlight`](Highlighter::highlight)
    /// call. MT-005 folding reads this to derive fold regions from the SAME tree the highlighter built
    /// (no second parse — the fold provider walks this tree with a `TreeCursor`).
    pub fn tree(&self) -> Option<&Tree> {
        self.old_tree.as_ref()
    }

    /// The stable language-family id this highlighter parses (`"rust"` / `"javascript"`, or `""` when
    /// unmapped). Selects folding's foldable-node set (MT-005).
    pub fn language_id(&self) -> &'static str {
        self.language_id
    }

    /// Parse `source` (UTF-8 bytes) and return its highlight spans in source order. The previous tree
    /// is reused for incremental re-parse (implementation note 2); the new tree is cached for the
    /// next call. Returns an empty `Vec` (never panics) if the parse fails.
    pub fn highlight(&mut self, source: &[u8]) -> Vec<HighlightSpan> {
        let tree = match self.parser.parse(source, self.old_tree.as_ref()) {
            Some(t) => t,
            None => return Vec::new(),
        };

        let mut spans: Vec<HighlightSpan> = Vec::new();
        let mut cursor = QueryCursor::new();
        let capture_names = self.query.capture_names();
        // tree-sitter 0.25: `QueryCursor::matches` returns a `StreamingIterator` of `QueryMatch`
        // (the underlying C cursor mutates on each step, so it cannot be a plain `Iterator`); walk it
        // with `while let Some(m) = matches.next()`. The `source` byte slice is the `TextProvider`.
        let mut matches = cursor.matches(&self.query, tree.root_node(), source);
        while let Some(m) = matches.next() {
            for cap in m.captures {
                let name = capture_names
                    .get(cap.index as usize)
                    .copied()
                    .unwrap_or("");
                let scope = HighlightScope::from_capture_name(name);
                let node = cap.node;
                spans.push(HighlightSpan {
                    byte_range: node.start_byte()..node.end_byte(),
                    scope,
                });
            }
        }

        // tree-sitter emits captures in match order, which is not strictly source order across
        // overlapping patterns; sort by start byte so the renderer can walk spans left-to-right and
        // resolve overlaps deterministically (a later, more specific capture wins on equal start).
        spans.sort_by_key(|s| (s.byte_range.start, s.byte_range.end));

        self.old_tree = Some(tree);
        spans
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rust_highlighter() -> Highlighter {
        LanguageRegistry::with_bundled_languages()
            .highlighter_for_extension("rs")
            .expect("rust highlighter from bundled registry")
    }

    fn js_highlighter() -> Highlighter {
        LanguageRegistry::with_bundled_languages()
            .highlighter_for_extension("js")
            .expect("js highlighter from bundled registry")
    }

    #[test]
    fn capture_name_maps_to_scope_including_sub_captures() {
        assert_eq!(HighlightScope::from_capture_name("keyword"), HighlightScope::Keyword);
        assert_eq!(HighlightScope::from_capture_name("string.special"), HighlightScope::String);
        assert_eq!(HighlightScope::from_capture_name("function.method"), HighlightScope::Function);
        assert_eq!(HighlightScope::from_capture_name("constructor"), HighlightScope::Function);
        assert_eq!(HighlightScope::from_capture_name("type.builtin"), HighlightScope::Type);
        assert_eq!(HighlightScope::from_capture_name("comment.line"), HighlightScope::Comment);
        assert_eq!(HighlightScope::from_capture_name("number"), HighlightScope::Number);
        assert_eq!(HighlightScope::from_capture_name("operator"), HighlightScope::Operator);
        assert_eq!(HighlightScope::from_capture_name("totally-unknown"), HighlightScope::Other);
    }

    #[test]
    fn rust_snippet_yields_keyword_and_function_spans() {
        let src = r#"
// a comment
fn compute(x: i32) -> i32 {
    let y = 42;
    return add(x, y);
}
fn add(a: i32, b: i32) -> i32 { a + b }
"#;
        let mut hl = rust_highlighter();
        let spans = hl.highlight(src.as_bytes());
        assert!(!spans.is_empty(), "expected highlight spans for a 10-line rust snippet");

        let has_keyword = spans.iter().any(|s| s.scope == HighlightScope::Keyword);
        let has_function = spans.iter().any(|s| s.scope == HighlightScope::Function);
        assert!(has_keyword, "AC-002: at least one Keyword span; got {spans:?}");
        assert!(has_function, "AC-002: at least one Function span; got {spans:?}");

        // Spot-check: the very first `fn` keyword span maps onto literal "fn" text.
        let kw = spans.iter().find(|s| s.scope == HighlightScope::Keyword).unwrap();
        let text = &src.as_bytes()[kw.byte_range.clone()];
        assert!(
            matches!(std::str::from_utf8(text), Ok("fn" | "let" | "return")),
            "a keyword span should cover a real keyword token, got {:?}",
            std::str::from_utf8(text)
        );
    }

    #[test]
    fn js_snippet_yields_string_span() {
        let src = r#"
const greeting = "hello world";
function greet(name) {
    return `hi ${name}`;
}
"#;
        let mut hl = js_highlighter();
        let spans = hl.highlight(src.as_bytes());
        let has_string = spans.iter().any(|s| s.scope == HighlightScope::String);
        assert!(has_string, "AC-003: at least one String span in JS; got {spans:?}");
    }

    #[test]
    fn spans_are_sorted_by_start_byte() {
        let mut hl = rust_highlighter();
        let spans = hl.highlight(b"fn a() {} fn b() {}");
        for w in spans.windows(2) {
            assert!(
                w[0].byte_range.start <= w[1].byte_range.start,
                "spans must be sorted by start byte"
            );
        }
    }

    #[test]
    fn incremental_reparse_after_caching_old_tree() {
        let mut hl = rust_highlighter();
        let first = hl.highlight(b"fn a() {}");
        assert!(!first.is_empty());
        // Second call reuses the cached tree (no panic, still produces spans).
        let second = hl.highlight(b"fn a() { let x = 1; }");
        assert!(second.iter().any(|s| s.scope == HighlightScope::Keyword));
    }

    #[test]
    fn unknown_extension_has_no_highlighter() {
        let reg = LanguageRegistry::with_bundled_languages();
        assert!(reg.highlighter_for_extension("xyz").is_none());
        // Case-insensitive ext match.
        assert!(reg.highlighter_for_extension("RS").is_some());
    }

    #[test]
    fn safe_language_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<SafeLanguage>();
        assert_send::<Highlighter>();
    }

    // ── MT-072 syntax-palette resolution (AC-003 live update / AC-004 completeness) ──────────────────

    use crate::workspace_settings::{SyntaxPalette, SyntaxPaletteMode};

    /// AC-004: Muted, Standard, and Custom modes each yield a color for EVERY HighlightScope variant —
    /// no missing scope, no panic on any lookup.
    #[test]
    fn every_mode_resolves_every_scope_with_no_gap() {
        for mode in [
            SyntaxPaletteMode::Muted,
            SyntaxPaletteMode::Standard,
            SyntaxPaletteMode::Custom,
        ] {
            let palette = SyntaxPalette { mode, custom: std::collections::HashMap::new() };
            for scope in HighlightScope::ALL.iter().copied() {
                // Each lookup returns a concrete Color32 (the call itself cannot panic — the mapping is
                // total). The assertion is that ALL eight scopes resolve under all three modes.
                let _color = resolve_scope_color(scope, &palette);
            }
        }
        // The built-in tables also cover all eight scopes by construction (the struct fields ARE the
        // scopes), proven by indexing each.
        for scope in HighlightScope::ALL.iter().copied() {
            let _muted = scope.builtin_color(&MUTED_PALETTE);
            let _standard = scope.builtin_color(&STANDARD_PALETTE);
        }
    }

    /// AC-003: a Custom swatch edit changes the resolved color in the SAME call (no caching / restart).
    /// Custom mode reads the override map LIVE: mutate the map, call resolve again, get the new color.
    #[test]
    fn custom_swatch_edit_changes_resolved_color_live() {
        let mut palette = SyntaxPalette {
            mode: SyntaxPaletteMode::Custom,
            custom: std::collections::HashMap::new(),
        };
        // With no override, Custom falls back to Standard for Keyword (no gap — AC-004).
        let before = resolve_scope_color(HighlightScope::Keyword, &palette);
        assert_eq!(
            before,
            HighlightScope::Keyword.builtin_color(&STANDARD_PALETTE),
            "un-overridden Custom scope falls back to Standard"
        );

        // Edit the Keyword swatch: the SAME-frame resolve returns the new color immediately.
        let new_rgba = [0xAB, 0xCD, 0xEF, 0xFF];
        palette.set_custom("keyword", new_rgba);
        let after = resolve_scope_color(HighlightScope::Keyword, &palette);
        assert_eq!(
            after,
            egui::Color32::from_rgba_unmultiplied(0xAB, 0xCD, 0xEF, 0xFF),
            "a Custom Keyword swatch edit changes the resolved color in the same frame (no restart)"
        );
        assert_ne!(after, before, "the color actually changed");

        // Other un-overridden scopes still resolve to Standard (only the edited scope changed).
        assert_eq!(
            resolve_scope_color(HighlightScope::String, &palette),
            HighlightScope::String.builtin_color(&STANDARD_PALETTE),
            "editing Keyword did not affect String"
        );
    }

    /// Muted and Standard are distinct palettes (the mode selector visibly changes colors).
    #[test]
    fn muted_and_standard_modes_differ() {
        let muted = SyntaxPalette { mode: SyntaxPaletteMode::Muted, custom: Default::default() };
        let standard = SyntaxPalette { mode: SyntaxPaletteMode::Standard, custom: Default::default() };
        let any_differ = HighlightScope::ALL.iter().copied().any(|s| {
            resolve_scope_color(s, &muted) != resolve_scope_color(s, &standard)
        });
        assert!(any_differ, "Muted and Standard must produce different colors for at least one scope");
    }

    /// `scope_key` round-trips with `from_scope_key`, and matches the persisted SYNTAX_SCOPE_KEYS list.
    #[test]
    fn scope_key_round_trips_and_matches_persisted_keys() {
        for scope in HighlightScope::ALL.iter().copied() {
            assert_eq!(HighlightScope::from_scope_key(scope.scope_key()), Some(scope));
            assert!(
                crate::workspace_settings::SYNTAX_SCOPE_KEYS.contains(&scope.scope_key()),
                "scope key '{}' is in the persisted vocabulary",
                scope.scope_key()
            );
        }
        // And every persisted key maps back to a scope (the two lists agree exactly).
        for key in crate::workspace_settings::SYNTAX_SCOPE_KEYS {
            assert!(HighlightScope::from_scope_key(key).is_some(), "persisted key '{key}' maps to a scope");
        }
    }
}
