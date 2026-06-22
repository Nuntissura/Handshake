//! Handshake backend code-navigation client (WP-KERNEL-012 MT-008 — E1 code editor).
//!
//! This is the NATIVE port of `app/src/lib/monaco/code_intelligence.ts`. It is the FALLBACK
//! intelligence source the editor uses when no language server is attached (and the always-on
//! enrichment source for symbol staleness): it binds the EXISTING handshake_core code-nav HTTP API
//!
//! - `GET /knowledge/code/symbols?workspace_id=&prefix=&name=&limit=` (completion + hover lookup),
//! - `GET /knowledge/code/symbols/:entity_id` (one symbol's definition span + staleness),
//! - `GET /knowledge/code/symbols/:entity_id/references` (callers + callees), and
//! - `GET /knowledge/code/files/:path/lens?workspace_id=&content_hash=&parser_version=` (file lens).
//!
//! ## REUSE, not a second HTTP stack
//!
//! Every request is sent through [`crate::backend_client::code_nav_get`], the SAME reqwest transport
//! the WP-KERNEL-011 [`crate::backend_client`] clients use (one `reqwest::Client`, the same
//! `BACKEND_BASE_URL`, the same 5s timeout, the same `serde_json::Value` deserialization so this never
//! depends on the `handshake_core` crate's types). The code-nav routes require the four backend-nav
//! identity headers (`x-hsk-actor-id`, `x-hsk-kernel-task-run-id`, `x-hsk-session-run-id`,
//! `x-hsk-actor-kind`); [`code_nav_get`](crate::backend_client::code_nav_get) attaches them.
//!
//! ## Off the egui thread (HBR-QUIET / implementation note 1 + 5)
//!
//! The egui render loop NEVER blocks on the network. The editor spawns a [`CodeNavClient`] request on
//! the app's tokio runtime in response to an input event (a keypress, cursor dwell) and the spawned
//! task delivers the typed result into an `Arc<Mutex<Option<..>>>` cell the UI drains on the next
//! frame — the exact `LoomBlockClient`/`SourceControlClient` delivery-cell shape. The methods here are
//! `async fn`s; the caller owns the spawn + cell.
//!
//! ## Debounce + cache (RISK-002 / MC-004 / implementation note 2 + 4)
//!
//! Flooding the backend on every keypress is RISK-002. The completion trigger is debounced at
//! [`COMPLETION_DEBOUNCE_MS`] (the panel checks `last_edit_instant`), and [`CodeNavCache`] memoizes
//! `lookup_symbols(prefix)` results for [`LOOKUP_CACHE_TTL`] so repeated lookups of the same prefix in
//! a short window reuse the cached result instead of re-hitting the backend.

use std::time::{Duration, Instant};

use serde::Deserialize;

use crate::backend_client::{code_nav_get, BACKEND_BASE_URL};
use crate::error::AppError;

use super::gutter::{DiagnosticSeverity, GutterMarker};

/// The completion trigger debounce window (RISK-002 / MC-004). The panel only fires a `lookup_symbols`
/// completion request when the user has not edited for this long, so fast typing does not flood the
/// backend with a request per keystroke. Named + documented exactly as MC-004 requires.
pub const COMPLETION_DEBOUNCE_MS: u64 = 200;

/// The cursor-dwell window before a hover request fires (implementation note 3). The panel tracks a
/// `(byte_offset, Instant)` dwell and only requests hover once the cursor has rested this long.
pub const HOVER_DWELL_MS: u64 = 500;

/// How long a `lookup_symbols(prefix)` result is reused before a fresh backend round-trip (RISK-002 /
/// implementation note 4). Short enough that new index state is picked up quickly; long enough that a
/// burst of completion triggers for the same prefix hits the backend once.
pub const LOOKUP_CACHE_TTL: Duration = Duration::from_secs(2);

/// The default lookup cap mirrored from the React `SYMBOL_LOOKUP_LIMIT` (the completion list length).
pub const SYMBOL_LOOKUP_LIMIT: usize = 20;

/// A symbol projection returned by the lookup + hover endpoints — the native equivalent of the React
/// `CodeSymbolNavProjection`. Deserialized from the backend's `symbol_to_json` shape via
/// `serde_json::Value` (only the fields the editor reads are modeled; unknown fields are ignored).
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct CodeSymbolNavProjection {
    /// The symbol's stable entity id (the key for `get_symbol` / `get_references`).
    #[serde(default)]
    pub symbol_entity_id: String,
    /// The symbol's full key (`<kind>:<path>#<name>`-ish), used for the file-path extraction.
    #[serde(default)]
    pub symbol_key: String,
    /// The human display name (the completion label + hover heading).
    #[serde(default)]
    pub display_name: String,
    /// The symbol kind string (`function`/`struct`/...), mapped to a [`CompletionKind`] for the icon.
    #[serde(default)]
    pub symbol_kind: String,
    /// The definition span (line range), present when the symbol is indexed; `None` otherwise.
    #[serde(default)]
    pub definition: Option<CodeSymbolDefinition>,
    /// The served-staleness flag (`{state, fresh, ...}`) — drives the AC-007 staleness gutter marker.
    #[serde(default)]
    pub staleness: Option<CodeStaleness>,
}

/// A symbol's definition span (1-based line range), as the backend's `definition` object carries it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct CodeSymbolDefinition {
    /// The 1-based first line of the definition (the editor converts to 0-based for navigation).
    #[serde(default)]
    pub line_start: Option<i64>,
    /// The 1-based last line of the definition.
    #[serde(default)]
    pub line_end: Option<i64>,
    /// The source id (file) the definition lives in (used for go-to-definition cross-file routing).
    #[serde(default)]
    pub source_id: Option<String>,
}

/// The served-staleness flag attached to every nav result (`served_staleness` in the backend). `fresh`
/// is the load-bearing field: a not-fresh symbol gets a warning gutter marker (AC-007), mirroring the
/// React `refreshHandshakeCodeIntelligenceMarkers` staleness path.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct CodeStaleness {
    /// The staleness state string (`fresh`/`marked_stale`/`unindexed`/`unknown`/...).
    #[serde(default)]
    pub state: Option<String>,
    /// Whether the index is provably fresh for this symbol. `false`/absent => stale warning marker.
    #[serde(default)]
    pub fresh: bool,
    /// The indexed content hash (used as a `get_file_lens` input when present).
    #[serde(default)]
    pub indexed_content_hash: Option<String>,
    /// The indexed parser version (used as a `get_file_lens` input when present).
    #[serde(default)]
    pub indexed_parser_version: Option<String>,
}

/// The lookup endpoint's response wrapper (`{ "matches": [...] }`).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct CodeSymbolLookupResponse {
    /// The matched symbol projections (capped at the request `limit`).
    #[serde(default)]
    pub matches: Vec<CodeSymbolNavProjection>,
}

/// The single-symbol endpoint's response wrapper (`{ "symbol": {...} }`).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct CodeSymbolResponse {
    /// The symbol projection, or a default (empty) projection when absent.
    #[serde(default)]
    pub symbol: CodeSymbolNavProjection,
}

/// One caller/callee reference returned by the references endpoint.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct CodeSymbolReference {
    /// The referencing symbol's entity id.
    #[serde(default)]
    pub symbol_entity_id: String,
    /// The referencing symbol's display name.
    #[serde(default)]
    pub display_name: String,
}

/// The references endpoint's response wrapper (`{ "callers": [...], "callees": [...] }`).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct CodeSymbolReferencesResponse {
    /// Symbols that reference (call) the queried symbol.
    #[serde(default)]
    pub callers: Vec<CodeSymbolReference>,
    /// Symbols the queried symbol references (calls).
    #[serde(default)]
    pub callees: Vec<CodeSymbolReference>,
}

impl CodeSymbolReferencesResponse {
    /// The total reference count (callers + callees) — AC-003 asserts at least one.
    pub fn total(&self) -> usize {
        self.callers.len() + self.callees.len()
    }
}

/// One file-lens entry (doc + staleness for a symbol), from the file-lens endpoint.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct CodeFileLensEntry {
    /// The entry's symbol entity id.
    #[serde(default)]
    pub symbol_entity_id: String,
    /// The rendered documentation for the symbol (shown in the hover tooltip when present).
    #[serde(default)]
    pub doc: Option<String>,
}

/// The file-lens endpoint's response wrapper (`{ "entries": [...] }`).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct CodeFileLensResponse {
    /// The per-symbol lens entries for the file.
    #[serde(default)]
    pub entries: Vec<CodeFileLensEntry>,
}

/// The completion-item kind icon, the native port of the React `completionKind` mapping
/// (`app/src/lib/monaco/code_intelligence.ts:106`). The completion popup renders [`CompletionKind::icon`]
/// before the label; nothing about the kind reaches the backend (it is purely presentational).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionKind {
    Class,
    Enum,
    Field,
    Module,
    Variable,
    Function,
}

impl CompletionKind {
    /// Map a backend `symbol_kind` string to a kind, mirroring `completionKind` exactly (the default,
    /// like the React `default:`, is `Function`).
    pub fn from_symbol_kind(kind: &str) -> Self {
        match kind {
            "class" | "struct" => CompletionKind::Class,
            "enum" => CompletionKind::Enum,
            "field" | "property" => CompletionKind::Field,
            "module" | "namespace" => CompletionKind::Module,
            "variable" => CompletionKind::Variable,
            _ => CompletionKind::Function,
        }
    }

    /// A short glyph shown as the completion item's kind icon (a monospace-friendly single char so it
    /// aligns in the popup list — the native analog of Monaco's kind icon).
    pub fn icon(self) -> &'static str {
        match self {
            CompletionKind::Class => "C",
            CompletionKind::Enum => "E",
            CompletionKind::Field => "f",
            CompletionKind::Module => "M",
            CompletionKind::Variable => "v",
            CompletionKind::Function => "\u{0192}", // ƒ
        }
    }
}

/// One completion suggestion shown in the popup. Built from a [`CodeSymbolNavProjection`] exactly the
/// way the React provider maps a match to a Monaco `CompletionItem` (label = display_name, insertText =
/// display_name, detail = symbol_kind, documentation = `markdown_for_symbol`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionItem {
    /// The label shown in the list (the symbol's display name).
    pub label: String,
    /// The text inserted at the cursor when the item is accepted.
    pub insert_text: String,
    /// The kind icon/detail (the symbol kind).
    pub kind: CompletionKind,
    /// The raw kind string (the `detail` line in the popup).
    pub detail: String,
    /// The markdown documentation shown alongside (hover body / popup detail line).
    pub documentation: String,
    /// The symbol's entity id, so accepting/inspecting an item can resolve its definition.
    pub symbol_entity_id: String,
}

impl CompletionItem {
    /// Build a completion item from a symbol projection (the React `suggestions.map(...)` body).
    pub fn from_symbol(symbol: &CodeSymbolNavProjection) -> Self {
        Self {
            label: symbol.display_name.clone(),
            insert_text: symbol.display_name.clone(),
            kind: CompletionKind::from_symbol_kind(&symbol.symbol_kind),
            detail: symbol.symbol_kind.clone(),
            documentation: markdown_for_symbol(symbol, None),
            symbol_entity_id: symbol.symbol_entity_id.clone(),
        }
    }
}

/// The hover content shown when the cursor dwells on an identifier — the data the React
/// `CodeSymbolPanel` + the hover provider render: the symbol heading, kind, key, staleness, and any
/// file-lens doc.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoverResult {
    /// The display name (the hover heading + the AC-006 text content the test asserts contains the
    /// identifier).
    pub display_name: String,
    /// The full markdown body (heading + kind + key + staleness + optional doc).
    pub markdown: String,
    /// The symbol's definition span line (0-based), if the symbol is indexed — the go-to-definition
    /// target the hover's "Go to definition" link uses.
    pub definition_line: Option<usize>,
    /// The symbol's entity id (for the references / go-to-definition follow-ups).
    pub symbol_entity_id: String,
}

/// A go-to-definition / reference location: a 0-based buffer line plus the optional source id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    /// The 0-based target line in the file.
    pub line: usize,
    /// The source id (file) the location lives in, when the backend supplied one.
    pub source_id: Option<String>,
}

/// The human staleness label, the native port of `codeSymbolStalenessLabel`
/// (`app/src/lib/monaco/code_intelligence.ts:46`): `"{state} ({fresh|not fresh})"`.
pub fn code_symbol_staleness_label(staleness: Option<&CodeStaleness>) -> String {
    match staleness {
        None => "unknown".to_owned(),
        Some(s) => {
            let state = s.state.as_deref().unwrap_or("unknown");
            let fresh = if s.fresh { "fresh" } else { "not fresh" };
            format!("{state} ({fresh})")
        }
    }
}

/// The hover/completion markdown for a symbol — the native port of `markdownForSymbol`
/// (`app/src/lib/monaco/code_intelligence.ts:234`): a bold name, the kind, the key, the staleness, and
/// an optional doc body.
pub fn markdown_for_symbol(symbol: &CodeSymbolNavProjection, doc: Option<&str>) -> String {
    let mut lines = vec![
        format!("**{}**", symbol.display_name),
        String::new(),
        format!("Kind: `{}`", symbol.symbol_kind),
        format!("Symbol: `{}`", symbol.symbol_key),
        format!("Staleness: `{}`", code_symbol_staleness_label(symbol.staleness.as_ref())),
    ];
    if let Some(doc) = doc.filter(|d| !d.is_empty()) {
        lines.push(String::new());
        lines.push(doc.to_owned());
    }
    lines.join("\n")
}

/// Extract the file path from a symbol key (`<kind>:<path>#<name>`), the port of `symbolFilePath`
/// (`code_intelligence.ts:127`). Returns `None` when the key has no `:` separator.
pub fn symbol_file_path(symbol_key: &str) -> Option<String> {
    let before_hash = symbol_key.split('#').next().unwrap_or("");
    let separator = before_hash.find(':')?;
    let path = before_hash[separator + 1..].trim();
    if path.is_empty() {
        None
    } else {
        Some(path.to_owned())
    }
}

/// Convert a not-fresh symbol projection into a warning gutter marker on its definition line — the
/// native port of `refreshHandshakeCodeIntelligenceMarkers`'s staleness branch (AC-007). A symbol with
/// no definition span, or one that is provably fresh, yields `None` (no marker). The marker line is
/// 0-based (the gutter's coordinate space); the symbol's `definition.line_start` is 1-based.
pub fn staleness_marker_for(symbol: &CodeSymbolNavProjection) -> Option<GutterMarker> {
    // Fresh symbols never produce a staleness marker.
    let staleness = symbol.staleness.as_ref()?;
    if staleness.fresh {
        return None;
    }
    let def = symbol.definition.as_ref()?;
    let line_start = def.line_start?;
    if line_start < 1 {
        return None;
    }
    let line = (line_start - 1) as usize; // 1-based -> 0-based gutter line.
    let label = code_symbol_staleness_label(Some(staleness));
    Some(GutterMarker::diagnostic(
        line,
        DiagnosticSeverity::Warning,
        format!("Stale code intelligence: {} is {label}", symbol.display_name),
    ))
}

/// A short-lived cache of `lookup_symbols(prefix)` results keyed by the exact prefix string, with a
/// [`LOOKUP_CACHE_TTL`] expiry (RISK-002 / MC-004). A single-entry cache (the editor only ever has one
/// active prefix at a time) keeps it trivial + lock-cheap. Stored behind a `Mutex` by the panel.
#[derive(Debug, Clone, Default)]
pub struct CodeNavCache {
    /// The cached `(prefix, matches, fetched_at)`. `None` until the first lookup is cached.
    entry: Option<(String, Vec<CodeSymbolNavProjection>, Instant)>,
}

impl CodeNavCache {
    /// An empty cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the cached matches for `prefix` iff the cache holds that exact prefix and the entry has
    /// not expired (RISK-002). A miss / expiry returns `None` so the caller hits the backend.
    pub fn get(&self, prefix: &str) -> Option<Vec<CodeSymbolNavProjection>> {
        match &self.entry {
            Some((p, matches, at))
                if p == prefix && at.elapsed() < LOOKUP_CACHE_TTL =>
            {
                Some(matches.clone())
            }
            _ => None,
        }
    }

    /// Store `matches` for `prefix`, stamped now. Replaces any previous entry (single-slot cache).
    pub fn put(&mut self, prefix: impl Into<String>, matches: Vec<CodeSymbolNavProjection>) {
        self.entry = Some((prefix.into(), matches, Instant::now()));
    }
}

/// The native code-navigation client. Wraps the four read-only backend code-nav routes through the
/// reused [`crate::backend_client`] transport, returning typed projections. Cheaply cloneable (it
/// holds only a base-URL string); the caller spawns its `async fn`s on the app's tokio runtime and
/// drains the result into a delivery cell (HBR-QUIET).
#[derive(Debug, Clone)]
pub struct CodeNavClient {
    base_url: String,
}

impl Default for CodeNavClient {
    fn default() -> Self {
        Self::production()
    }
}

impl CodeNavClient {
    /// Build a client against an explicit `base_url` (for tests pointing at a live backend).
    pub fn new(base_url: impl Into<String>) -> Self {
        Self { base_url: base_url.into() }
    }

    /// The production client against the hardcoded backend base URL.
    pub fn production() -> Self {
        Self::new(BACKEND_BASE_URL)
    }

    /// The base URL this client targets (for diagnostics / tests).
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// `GET /knowledge/code/symbols?workspace_id=&prefix=&limit=` — the completion + hover lookup. Ports
    /// `lookupCodeSymbols` (`api.ts:2158`). Returns the matched projections (possibly empty). A backend
    /// error (down / non-2xx / parse) is returned as [`AppError`] so the caller degrades gracefully
    /// (empty completion) rather than panicking.
    pub async fn lookup_symbols(
        &self,
        workspace_id: &str,
        prefix: &str,
        limit: usize,
    ) -> Result<Vec<CodeSymbolNavProjection>, AppError> {
        let url = format!("{}/knowledge/code/symbols", self.base_url);
        let query = vec![
            ("workspace_id".to_owned(), workspace_id.to_owned()),
            ("prefix".to_owned(), prefix.to_owned()),
            ("limit".to_owned(), limit.to_string()),
        ];
        let v = code_nav_get(&url, &query, &format!("lookup-{workspace_id}")).await?;
        let parsed: CodeSymbolLookupResponse =
            serde_json::from_value(v).map_err(|e| AppError::Parse(e.to_string()))?;
        Ok(parsed.matches)
    }

    /// `GET /knowledge/code/symbols/:entity_id` — one symbol's definition + staleness. Ports
    /// `getCodeSymbol` (`api.ts:2152`). Returns `None` when the symbol is absent / the backend errors
    /// (graceful: hover shows nothing rather than crashing).
    pub async fn get_symbol(
        &self,
        entity_id: &str,
    ) -> Result<CodeSymbolResponse, AppError> {
        let url = format!(
            "{}/knowledge/code/symbols/{}",
            self.base_url,
            urlencode(entity_id)
        );
        let v = code_nav_get(&url, &[], &format!("symbol-{entity_id}")).await?;
        serde_json::from_value(v).map_err(|e| AppError::Parse(e.to_string()))
    }

    /// `GET /knowledge/code/symbols/:entity_id/references` — callers + callees. Ports
    /// `getCodeSymbolReferences` (`api.ts:2169`).
    pub async fn get_references(
        &self,
        entity_id: &str,
    ) -> Result<CodeSymbolReferencesResponse, AppError> {
        let url = format!(
            "{}/knowledge/code/symbols/{}/references",
            self.base_url,
            urlencode(entity_id)
        );
        let v = code_nav_get(&url, &[], &format!("references-{entity_id}")).await?;
        serde_json::from_value(v).map_err(|e| AppError::Parse(e.to_string()))
    }

    /// `GET /knowledge/code/files/:path/lens?workspace_id=&content_hash=&parser_version=` — the file
    /// lens (per-symbol docs). Ports `getCodeFileLens` (`api.ts:2175`).
    pub async fn get_file_lens(
        &self,
        workspace_id: &str,
        path: &str,
        content_hash: &str,
        parser_version: &str,
    ) -> Result<CodeFileLensResponse, AppError> {
        let url = format!(
            "{}/knowledge/code/files/{}/lens",
            self.base_url,
            urlencode(path)
        );
        let query = vec![
            ("workspace_id".to_owned(), workspace_id.to_owned()),
            ("content_hash".to_owned(), content_hash.to_owned()),
            ("parser_version".to_owned(), parser_version.to_owned()),
        ];
        let v = code_nav_get(&url, &query, &format!("lens-{workspace_id}")).await?;
        serde_json::from_value(v).map_err(|e| AppError::Parse(e.to_string()))
    }
}

/// Minimal percent-encoding for a path segment (the chars that would break a URL path/query). The
/// backend route matches `:entity_id` / `:path` as a single segment; symbol ids are slug-like but a
/// path may contain `/`, which must be encoded so it stays one segment. Keeps the dependency surface
/// small (no `urlencoding` crate) since only a handful of chars matter here.
fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completion_kind_maps_like_react() {
        // Mirror the React `completionKind` switch, including the Function default.
        assert_eq!(CompletionKind::from_symbol_kind("struct"), CompletionKind::Class);
        assert_eq!(CompletionKind::from_symbol_kind("class"), CompletionKind::Class);
        assert_eq!(CompletionKind::from_symbol_kind("enum"), CompletionKind::Enum);
        assert_eq!(CompletionKind::from_symbol_kind("field"), CompletionKind::Field);
        assert_eq!(CompletionKind::from_symbol_kind("property"), CompletionKind::Field);
        assert_eq!(CompletionKind::from_symbol_kind("module"), CompletionKind::Module);
        assert_eq!(CompletionKind::from_symbol_kind("namespace"), CompletionKind::Module);
        assert_eq!(CompletionKind::from_symbol_kind("variable"), CompletionKind::Variable);
        assert_eq!(CompletionKind::from_symbol_kind("function"), CompletionKind::Function);
        assert_eq!(CompletionKind::from_symbol_kind("anything-else"), CompletionKind::Function);
    }

    #[test]
    fn staleness_label_matches_react_format() {
        let fresh = CodeStaleness { state: Some("fresh".into()), fresh: true, ..Default::default() };
        assert_eq!(code_symbol_staleness_label(Some(&fresh)), "fresh (fresh)");
        let stale =
            CodeStaleness { state: Some("marked_stale".into()), fresh: false, ..Default::default() };
        assert_eq!(code_symbol_staleness_label(Some(&stale)), "marked_stale (not fresh)");
        assert_eq!(code_symbol_staleness_label(None), "unknown");
    }

    #[test]
    fn markdown_for_symbol_has_name_kind_key_staleness() {
        let symbol = CodeSymbolNavProjection {
            display_name: "add".into(),
            symbol_kind: "function".into(),
            symbol_key: "fn:src/math.rs#add".into(),
            staleness: Some(CodeStaleness {
                state: Some("fresh".into()),
                fresh: true,
                ..Default::default()
            }),
            ..Default::default()
        };
        let md = markdown_for_symbol(&symbol, Some("Adds two numbers."));
        assert!(md.contains("**add**"), "bold name");
        assert!(md.contains("Kind: `function`"));
        assert!(md.contains("Symbol: `fn:src/math.rs#add`"));
        assert!(md.contains("Staleness: `fresh (fresh)`"));
        assert!(md.contains("Adds two numbers."), "doc appended");
    }

    #[test]
    fn symbol_file_path_extracts_path_segment() {
        assert_eq!(
            symbol_file_path("fn:src/math.rs#add"),
            Some("src/math.rs".to_owned())
        );
        assert_eq!(symbol_file_path("noseparator"), None);
        assert_eq!(symbol_file_path("fn:#add"), None, "empty path -> None");
    }

    #[test]
    fn staleness_marker_only_for_not_fresh_with_definition() {
        // Fresh symbol -> no marker.
        let fresh = CodeSymbolNavProjection {
            display_name: "ok".into(),
            definition: Some(CodeSymbolDefinition { line_start: Some(10), ..Default::default() }),
            staleness: Some(CodeStaleness { fresh: true, ..Default::default() }),
            ..Default::default()
        };
        assert!(staleness_marker_for(&fresh).is_none(), "fresh -> no marker");

        // Not-fresh with a definition -> a Warning marker on the 0-based def line.
        let stale = CodeSymbolNavProjection {
            display_name: "old".into(),
            definition: Some(CodeSymbolDefinition { line_start: Some(10), ..Default::default() }),
            staleness: Some(CodeStaleness {
                state: Some("marked_stale".into()),
                fresh: false,
                ..Default::default()
            }),
            ..Default::default()
        };
        let marker = staleness_marker_for(&stale).expect("not-fresh -> marker");
        assert_eq!(marker.line, 9, "1-based line 10 -> 0-based gutter line 9");
        assert!(matches!(
            marker.kind,
            super::super::gutter::GutterMarkerKind::Diagnostic(DiagnosticSeverity::Warning)
        ));

        // Not-fresh but no definition span -> no marker (nothing to anchor it to).
        let stale_no_def = CodeSymbolNavProjection {
            staleness: Some(CodeStaleness { fresh: false, ..Default::default() }),
            ..Default::default()
        };
        assert!(staleness_marker_for(&stale_no_def).is_none());
    }

    #[test]
    fn references_total_counts_callers_and_callees() {
        let refs = CodeSymbolReferencesResponse {
            callers: vec![CodeSymbolReference::default()],
            callees: vec![CodeSymbolReference::default(), CodeSymbolReference::default()],
        };
        assert_eq!(refs.total(), 3);
    }

    #[test]
    fn lookup_response_deserializes_backend_shape() {
        // The exact backend `lookup_symbols` body shape (symbol_to_json under "matches").
        let body = serde_json::json!({
            "workspace_id": "ws1",
            "matches": [{
                "symbol_entity_id": "ent-1",
                "symbol_key": "fn:src/math.rs#add",
                "display_name": "add",
                "symbol_kind": "function",
                "definition": { "line_start": 12, "line_end": 14, "source_id": "src-1" },
                "staleness": { "state": "fresh", "fresh": true }
            }]
        });
        let parsed: CodeSymbolLookupResponse = serde_json::from_value(body).unwrap();
        assert_eq!(parsed.matches.len(), 1);
        let m = &parsed.matches[0];
        assert_eq!(m.display_name, "add");
        assert_eq!(m.symbol_kind, "function");
        assert_eq!(m.definition.as_ref().unwrap().line_start, Some(12));
        assert!(m.staleness.as_ref().unwrap().fresh);
    }

    #[test]
    fn cache_hits_within_ttl_and_respects_prefix() {
        let mut cache = CodeNavCache::new();
        assert!(cache.get("ad").is_none(), "empty cache misses");
        let matches = vec![CodeSymbolNavProjection { display_name: "add".into(), ..Default::default() }];
        cache.put("ad", matches.clone());
        assert_eq!(cache.get("ad").map(|m| m.len()), Some(1), "same prefix hits");
        assert!(cache.get("xyz").is_none(), "different prefix misses");
    }
}
