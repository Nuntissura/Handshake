//! Bidirectional code<->note cross-reference resolution (WP-KERNEL-012 MT-034, cluster E5).
//!
//! ## What this is (the two directions of the code<->note edge)
//!
//! This module is the resolution SERVICE behind the two MT-034 directions:
//!
//! - (A) note -> code: a `[[code:path/to/file.rs#MyStruct]]` reference in a note is the EXISTING
//!   `hsLink` inline atom with `ref_kind="code"` (parsed by the MT-015 wikilink parser — the `code:`
//!   prefix is registered in `wikilinks/parser.rs`, NOT a new node type). Clicking the chip dispatches
//!   `open-code-symbol` on the MT-031 [`crate::interop::InteractionBus`]; the shell routes it through
//!   the MT-030 [`crate::quick_switcher::ShellNavigator`] `open_code_symbol` seam. [`resolve_code_ref`]
//!   turns the staged `symbol_entity_id` into a [`CodeRef`] (file path + line span) via the EXISTING
//!   code-nav backend so the editor can jump-to-line (the actual jump lands when the code pane mounts
//!   at E11/MT-069 — until then the navigator returns `EditorPaneNotMounted`, never a faked jump).
//!
//! - (B) code -> notes: from the code pane, [`find_notes_referencing_symbol`] lists the rich documents
//!   that mention the focused symbol. The [`NoteRefsPanel`](crate::code_editor::note_refs_panel) renders
//!   the result; clicking a row dispatches the EXISTING `open-document` command on the same bus.
//!
//! ## Backend reuse only (no backend edits — typed blocker if a gap)
//!
//! - [`resolve_code_ref`] reuses [`crate::code_editor::code_nav::CodeNavClient::get_symbol`]
//!   (`GET /knowledge/code/symbols/:entity_id`, the VERIFIED real path — the MT contract's bare
//!   `/code/symbols/{id}` is the React `api.ts` shorthand; the live backend route is the
//!   `/knowledge/code/...` family the MT-008 client already binds).
//! - [`find_notes_referencing_symbol`] reuses the VERIFIED hybrid-search route
//!   `POST /workspaces/{ws}/loom/search-v2` (the same route MT-015 wikilink autocomplete + MT-028 search
//!   use), querying the symbol key and restricting to rich-document content types.
//!
//!   ENDPOINT CHOICE (RISK-1 / MC-1, the KERNEL_BUILDER "verify the route" gate): the MT contract
//!   preferred a backlink/`ref_value` index over naive full-text. The DEDICATED backlinks route
//!   (`GET /workspaces/{ws}/loom/blocks/{id}/backlinks`, MT-178) is keyed on a BLOCK id, not on an
//!   arbitrary `ref_value` / symbol key — there is no verified `GET /knowledge/backlinks?ref_value=…`
//!   route in the live backend (confirmed read-only against `backend_client` + the React `api.ts`
//!   surface). Search-v2 therefore supplies only a bounded candidate set. The production path then
//!   reads each candidate document and accepts it only when persisted `content_json` contains an exact
//!   structured code `hsLink` identity. Plain mentions and same-name symbols in other files cannot
//!   become reverse references. If a dedicated
//!   ref_value/backlink index endpoint is added later, [`find_notes_referencing_symbol`] swaps to it
//!   with no caller change. A missing endpoint is a typed [`CrossRefError`] (visible empty state),
//!   NEVER a backend edit.
//!
//! ## Off the egui thread (HBR-QUIET)
//!
//! The methods here are `async fn`s; the caller spawns them on the app tokio runtime and drains the
//! typed result into a delivery cell the UI reads next frame (the MT-008/MT-015 delivery-cell shape).
//! The [`SymbolDwellTracker`] enforces the 800ms dwell debounce so a cursor move does NOT spam the
//! backend (RISK-3 / MC-3): the search fires ONCE per dwell crossing, and the timer resets on every
//! cursor move.
//!
//! ## URL key encoding (RISK-2 / MC-2)
//!
//! Symbol keys contain `::`, `/`, and `#`. [`percent_encode_symbol`] percent-encodes them so they
//! embed in a URL path/query segment without breaking routing (a missed encode causes a 404). The
//! encoder is the same dependency-free byte-walk the MT-008 `code_nav::urlencode` uses (reqwest does
//! NOT re-export `percent_encoding`, so adding a crate would be unjustified churn for a handful of
//! chars). A unit test covers a key containing `/`, `#`, and `::`.

use std::time::{Duration, Instant};

use thiserror::Error;

use crate::backend_client::{
    LoomSearchV2Body, LoomSearchV2Hit, LoomSearchV2Response, BACKEND_BASE_URL,
};
use crate::code_editor::code_nav::{CodeNavClient, CodeSymbolNavProjection};
use crate::error::AppError;

/// The backend `ref_kind` a code cross-reference `hsLink` atom carries (the discriminator the
/// note->code dispatch keys on). Registered in `wikilinks/parser.rs` so `[[code:…]]` parses to it.
pub const CODE_REF_KIND: &str = "code";

/// The dwell window (ms) the cursor must rest on a symbol before the code pane fires a
/// `find_notes_referencing_symbol` search (RISK-3 / MC-3). The timer RESETS on every cursor move, so a
/// scan across many symbols fires zero searches; the search fires ONCE when the cursor settles.
pub const NOTE_REFS_DWELL_MS: u64 = 800;

/// The result cap for a `find_notes_referencing_symbol` search (keeps the NoteRefsPanel list bounded).
pub const NOTE_REFS_SEARCH_LIMIT: u32 = 25;

/// Exact reverse lookup may need to scan beyond the operator-facing 25-row panel page before it finds
/// the persisted structured `hsLink`. Fail closed at the same bounded find-all ceiling used by the
/// native workspace search instead of returning a silently truncated exact result.
pub const NOTE_REFS_MAX_CANDIDATES: usize = 10_000;

/// The rich-document content types a code->notes search restricts to (RISK-1 / MC-1: a code symbol is
/// referenced from NOTES, so a search filtered to these content types excludes unrelated block kinds
/// and cuts false positives).
///
/// BACKEND-SHAPE INVARIANT (verified read-only): the backend `content_type` filter deserializes into
/// `LoomBlockContentType` (`src/backend/handshake_core/src/storage/loom.rs:41-69`), a snake_case enum
/// with NO `#[serde(other)]` fallback — so an unknown value makes Axum's `Json` extractor return HTTP
/// 422 and the WHOLE search fails. Every value here MUST therefore be a real
/// `LoomBlockContentType::as_str()` token. The verified set is:
/// `note, file, annotated_file, tag_hub, journal, canvas, view_def` (loom.rs:58-69) — there is NO
/// `document` variant (an earlier draft used `"document"`, which 422'd against real PG; the `Document`
/// at loom.rs:502 belongs to the UNRELATED `LoomSearchSourceKind` hit-source enum, not this filter).
/// A code reference in a note lives in a `note` block, and daily-note coverage lives in a `journal`
/// block — both are real tokens (asserted by the `note_ref_content_types_are_valid_backend_tokens`
/// unit test against [`BACKEND_LOOM_CONTENT_TYPE_TOKENS`] so a mock can never hide an invalid value).
pub const NOTE_REF_CONTENT_TYPES: &[&str] = &["note", "journal"];

/// The complete verified `LoomBlockContentType::as_str()` allow-list, mirrored from the real backend
/// (`src/backend/handshake_core/src/storage/loom.rs:58-69`) for a compile-adjacent guard: the unit test
/// `note_ref_content_types_are_valid_backend_tokens` asserts every [`NOTE_REF_CONTENT_TYPES`] value is in
/// this set, so a code->notes search can NEVER send a value the backend would reject with HTTP 422 (the
/// drift the `MockSearch` happy-path would otherwise mask). This list is a verification fixture, NOT a
/// second source of authority — the backend enum stays canonical; if it changes, update both together.
pub const BACKEND_LOOM_CONTENT_TYPE_TOKENS: &[&str] = &[
    "note",
    "file",
    "annotated_file",
    "tag_hub",
    "journal",
    "canvas",
    "view_def",
];

/// Why a code<->note cross-reference resolution failed. Every variant renders as a VISIBLE state
/// (empty / typed error chip), never a silent no-op or a panic. `kind_str` is a stable kebab-case
/// token the error UI + AccessKit label carry so an out-of-process agent reads a stable vocabulary.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CrossRefError {
    /// No workspace bound — a notes search resolves workspace state and needs a workspace id.
    #[error("no workspace context: code<->note cross-reference needs a workspace id")]
    NoWorkspace,
    /// The symbol reference was empty (nothing to resolve).
    #[error("empty symbol ref: no code symbol reference to resolve")]
    EmptySymbol,
    /// The backend resolved the symbol but it carried no definition span (no file/line to jump to).
    /// The chip renders as `unresolved` (greyed) without crashing — AC-4 / RISK pt(e).
    #[error("symbol has no definition span: {0}")]
    NoDefinition(String),
    /// The target symbol/document was not found (HTTP 404 / empty projection). Drives the
    /// greyed-out `unresolved` chip (AC-4) — a deleted symbol must NOT crash or panic.
    #[error("not found: {0}")]
    NotFound(String),
    /// A supposedly identity-bound backend response returned a different/empty identity. Treating the
    /// projection as the requested symbol would navigate to unrelated code, so it is rejected.
    #[error("backend identity mismatch: requested '{requested}', returned '{returned}'")]
    IdentityMismatch { requested: String, returned: String },
    /// The backend transport failed (down / non-2xx / parse). Surfaced as a typed error state.
    #[error("backend error: {0}")]
    Backend(String),
}

impl CrossRefError {
    /// Stable kebab-case kind token (the chip text + AccessKit label vocabulary).
    pub fn kind_str(&self) -> &'static str {
        match self {
            CrossRefError::NoWorkspace => "no_workspace",
            CrossRefError::EmptySymbol => "empty_symbol",
            CrossRefError::NoDefinition(_) => "no_definition",
            CrossRefError::NotFound(_) => "not_found",
            CrossRefError::IdentityMismatch { .. } => "identity_mismatch",
            CrossRefError::Backend(_) => "backend_error",
        }
    }

    /// True when this error means the symbol could not be resolved to a live definition (a deleted /
    /// unindexed symbol). The code-ref chip renders `unresolved` (greyed) for these without panicking
    /// (AC-4 / RISK pt(e)). A transient backend error is NOT treated as unresolved (it should retry).
    pub fn is_unresolved(&self) -> bool {
        matches!(
            self,
            CrossRefError::NotFound(_)
                | CrossRefError::NoDefinition(_)
                | CrossRefError::EmptySymbol
        )
    }
}

impl From<AppError> for CrossRefError {
    fn from(e: AppError) -> Self {
        // The code-nav transport returns a non-success status as `AppError::Http(...)`; a 404 there is
        // a missing symbol (drives the unresolved chip). We can only see the status text, so a body
        // containing "404" maps to NotFound; everything else is a generic backend error.
        match &e {
            AppError::Http(m) if m.contains("404") => CrossRefError::NotFound(m.clone()),
            AppError::Http(m) => CrossRefError::Backend(m.clone()),
            AppError::Parse(m) => CrossRefError::Backend(m.clone()),
        }
    }
}

/// A resolved code-symbol target: the file the symbol is defined in and its 0-based line span (the
/// editor jumps to `line_start`). Built from the backend `getCodeSymbol` definition projection. The
/// MT contract names `{symbol_key, file_path, line_start, line_end}`; the entity id is carried too so
/// the resolved target round-trips back to the navigator without re-deriving it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeRef {
    /// The symbol's stable entity id (the key the navigator + backend use).
    pub symbol_entity_id: String,
    /// The full symbol key (`<kind>:<path>#<name>`), preserved for display + the find-notes query.
    pub symbol_key: String,
    /// The canonical PostgreSQL `KnowledgeSource` identity for the file. This is provenance, not a
    /// filesystem path: callers may bind it to a loaded code tab but must never try to open it.
    pub source_id: String,
    /// The file path the symbol is defined in (extracted from the definition source / symbol key).
    pub file_path: String,
    /// The 0-based first line of the definition (the editor scroll/jump target). The backend serves a
    /// 1-based `line_start`; this is converted to 0-based here (the editor's coordinate space).
    pub line_start: u32,
    /// The 0-based last line of the definition (>= `line_start`).
    pub line_end: u32,
}

/// A note (rich document) that mentions a code symbol — the code->notes direction result row. Built
/// from a loom search-v2 hit. The MT contract names `{document_id, document_title, block_id, excerpt}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteRef {
    /// The referencing block id (the loom block the hit matched — the open-document staging id falls
    /// back to this when the hit carries no separate document id).
    pub block_id: String,
    /// The rich-document id the row opens (`open-document` target). For a loom search hit the block id
    /// IS the openable reference; a future document-join endpoint can populate a distinct doc id.
    pub document_id: String,
    /// The note's display title (or the block id as a fallback when the block has no title).
    pub document_title: String,
    /// A short excerpt centered on the symbol mention (the search highlight, `<mark>` markers stripped).
    pub excerpt: String,
}

impl NoteRef {
    /// Build a [`NoteRef`] from a loom search-v2 hit. Prefer the backend `document_id` when it is
    /// present because `open-document` targets rich documents, not arbitrary matched block ids. The
    /// block id remains the hit/de-dupe key; the title falls back to the block id; the excerpt is the
    /// FTS highlight with literal `<mark>`/`</mark>` markers stripped.
    pub fn from_hit(hit: LoomSearchV2Hit) -> Self {
        let block_id = hit.block.block_id.clone();
        let document_id = hit
            .block
            .document_id
            .clone()
            .filter(|id| !id.trim().is_empty())
            .unwrap_or_else(|| block_id.clone());
        let document_title = hit.block.display_title().to_owned();
        let excerpt = strip_mark_tags(&hit.highlight);
        Self {
            document_id,
            block_id,
            document_title,
            excerpt,
        }
    }
}

/// Strip the literal `<mark>` / `</mark>` highlight markers a ts_headline excerpt carries, leaving
/// plain text (the NoteRefsPanel renders plain text, never raw HTML — the same rule the MT-028 search
/// panel follows by parsing the markers into colored runs; here we only need the bare text).
fn strip_mark_tags(highlight: &str) -> String {
    highlight
        .replace("<mark>", "")
        .replace("</mark>", "")
        .trim()
        .to_owned()
}

/// Percent-encode a symbol key (or any value) for embedding in a URL path/query segment (RISK-2 /
/// MC-2). Symbol keys contain `::`, `/`, and `#`, all of which break URL routing unencoded. This is
/// the same dependency-free unreserved-char allow-list the MT-008 `code_nav::urlencode` uses; every
/// other byte is `%XX`-encoded. (`reqwest` does NOT re-export `percent_encoding`, so a new crate would
/// be unjustified for a handful of chars — the local encoder is the established pattern in this crate.)
pub fn percent_encode_symbol(s: &str) -> String {
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

/// Validate the exact `path#symbol` payload shared by the code-editor producer and rich-editor
/// consumer for `[[code:...]]` references. Both components must be losslessly representable by the
/// wikilink grammar; in particular, a bare `]` is rejected (not only the closing `]]` pair) because
/// the parser's target capture stops at either bracket.
pub fn is_encodable_code_reference_target(target: &str) -> bool {
    let Some((path, symbol)) = target.split_once('#') else {
        return false;
    };
    let mut symbol_chars = symbol.chars();
    let symbol_is_identifier = symbol_chars
        .next()
        .is_some_and(|ch| ch == '_' || ch == '$' || ch.is_alphabetic())
        && symbol_chars.all(|ch| ch == '_' || ch == '$' || ch.is_alphanumeric());
    !path.is_empty()
        && !symbol.is_empty()
        && symbol_is_identifier
        && path.trim() == path
        && symbol.trim() == symbol
        && !path.contains('#')
        && !symbol.contains('#')
        && !target
            .chars()
            .any(|ch| matches!(ch, ']' | '|' | '\r' | '\n'))
}

/// Format one canonical code-to-note reference, failing closed when either component would be
/// truncated or reinterpreted by the rich-editor wikilink parser.
pub fn format_code_note_reference(path: &str, symbol: &str) -> Option<String> {
    let target = format!("{path}#{symbol}");
    is_encodable_code_reference_target(&target).then(|| format!("[[code:{target}]]"))
}

fn parse_path_symbol_ref(symbol_ref: &str) -> Option<(&str, &str)> {
    let (raw_path, raw_name) = symbol_ref.rsplit_once('#')?;
    let raw_path = raw_path.trim();
    let bytes = raw_path.as_bytes();
    let is_windows_drive_absolute = bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'\\' | b'/');
    let is_absolute_or_uri = is_windows_drive_absolute
        || raw_path.starts_with('/')
        || raw_path.starts_with('\\')
        || raw_path
            .get(..7)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("file://"));
    // Backend symbol keys may prefix a relative path with a language (`rust:src/lib.rs`). An authored
    // Windows drive/verbatim/UNC path or file URI also contains a colon, but that colon is path syntax
    // and must never be stripped. Use only the first colon and only a conservative language token.
    let path = if is_absolute_or_uri {
        raw_path
    } else if let Some((prefix, remainder)) = raw_path.split_once(':') {
        let is_language_prefix = !prefix.is_empty()
            && prefix
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'));
        if is_language_prefix && !remainder.trim().is_empty() {
            remainder.trim()
        } else {
            raw_path
        }
    } else {
        raw_path
    };
    let name = raw_name.trim();
    if path.is_empty() || name.is_empty() {
        None
    } else {
        Some((path, name))
    }
}

fn normalized_symbol_path(path: &str) -> String {
    let mut normalized = path.trim().replace('\\', "/");
    while let Some(stripped) = normalized.strip_prefix("./") {
        normalized = stripped.to_owned();
    }
    #[cfg(windows)]
    normalized.make_ascii_lowercase();
    normalized
}

fn symbol_matches_exact_path_name(
    symbol: &CodeSymbolNavProjection,
    requested_path: &str,
    requested_name: &str,
) -> bool {
    let Some(path) = crate::code_editor::code_nav::symbol_file_path(&symbol.symbol_key) else {
        return false;
    };
    let key_name = symbol
        .symbol_key
        .rsplit_once('#')
        .map(|(_, name)| name.trim());
    normalized_symbol_path(&path) == normalized_symbol_path(requested_path)
        && key_name == Some(requested_name)
        && symbol.display_name == requested_name
        && !symbol.symbol_entity_id.trim().is_empty()
}

fn code_ref_from_symbol(
    requested_ref: &str,
    symbol: CodeSymbolNavProjection,
) -> Result<CodeRef, CrossRefError> {
    if symbol.symbol_entity_id.trim().is_empty() {
        return Err(CrossRefError::IdentityMismatch {
            requested: requested_ref.to_owned(),
            returned: symbol.symbol_entity_id,
        });
    }
    let definition = symbol
        .definition
        .as_ref()
        .ok_or_else(|| CrossRefError::NoDefinition(requested_ref.to_owned()))?;
    let source_id = definition
        .source_id
        .as_deref()
        .map(str::trim)
        .filter(|source_id| !source_id.is_empty())
        .ok_or_else(|| CrossRefError::NoDefinition(requested_ref.to_owned()))?
        .to_owned();
    let (line_start, line_end) = match (definition.line_start, definition.line_end) {
        (Some(start), end) if start >= 1 => {
            let s = (start - 1) as u32; // 1-based backend -> 0-based editor.
            let e = end
                .filter(|e| *e >= start)
                .map(|e| (e - 1) as u32)
                .unwrap_or(s);
            (s, e)
        }
        _ => return Err(CrossRefError::NoDefinition(requested_ref.to_owned())),
    };
    // The backend definition's `source_id` is the opaque KnowledgeSource identity (for example
    // `KSRC-...`), NOT a filesystem path. The canonical repo-relative path is encoded in the symbol
    // key (`<language>:<path>#<symbol>`). Never reinterpret an opaque source identity as a path: a
    // malformed/missing key path is an unresolved definition, not authority to open a `KSRC-*` file.
    let file_path = crate::code_editor::code_nav::symbol_file_path(&symbol.symbol_key)
        .filter(|path| !path.trim().is_empty())
        .ok_or_else(|| CrossRefError::NoDefinition(requested_ref.to_owned()))?;
    Ok(CodeRef {
        symbol_entity_id: symbol.symbol_entity_id,
        symbol_key: symbol.symbol_key,
        source_id,
        file_path,
        line_start,
        line_end,
    })
}

/// Resolve a code-symbol entity id to a [`CodeRef`] (file path + 0-based line span) via the EXISTING
/// code-nav backend (`GET /knowledge/code/symbols/:entity_id`, reusing [`CodeNavClient`]). For
/// hand-authored `path#Symbol` refs, callers that have workspace context should use
/// [`resolve_code_ref_with_workspace`] so the backend lookup route can bind `workspace_id`, `path`, and
/// `name`.
///
/// Direction (A) note->code: clicking a `[[code:…]]` chip dispatches `open-code-symbol` with the
/// symbol reference; the navigator calls this to learn WHERE to jump. The actual jump-to-line lands
/// when the code pane is mounted (E11/MT-069) — this resolves the target HONESTLY; it never fakes a
/// jump into a non-existent pane.
///
/// Errors:
/// - empty ref -> [`CrossRefError::EmptySymbol`] (no round-trip),
/// - 404 / empty projection -> [`CrossRefError::NotFound`] (drives the greyed `unresolved` chip, AC-4),
/// - resolved but no definition span -> [`CrossRefError::NoDefinition`] (also unresolved),
/// - transport failure -> [`CrossRefError::Backend`].
pub async fn resolve_code_ref(symbol_ref: &str) -> Result<CodeRef, CrossRefError> {
    resolve_code_ref_with(&CodeNavClient::production(), symbol_ref).await
}

/// [`resolve_code_ref`] against an explicit [`CodeNavClient`] (a test points it at a live backend; the
/// percent-encoding of the entity id is performed INSIDE the client's `get_symbol`, so this is the
/// single resolution path the production helper also uses).
pub async fn resolve_code_ref_with(
    client: &CodeNavClient,
    symbol_ref: &str,
) -> Result<CodeRef, CrossRefError> {
    resolve_code_ref_with_workspace(client, "", symbol_ref).await
}

/// Resolve a code ref with workspace context. Entity ids keep using `get_symbol`; hand-authored
/// `path#Symbol` refs use the existing lookup route's `workspace_id` + `path` + `name` filters and then
/// resolve to the returned symbol projection.
pub async fn resolve_code_ref_with_workspace(
    client: &CodeNavClient,
    workspace_id: &str,
    symbol_ref: &str,
) -> Result<CodeRef, CrossRefError> {
    let symbol_ref = symbol_ref.trim();
    if symbol_ref.is_empty() {
        return Err(CrossRefError::EmptySymbol);
    }
    if let Some((path, name)) = parse_path_symbol_ref(symbol_ref) {
        if workspace_id.trim().is_empty() {
            return Err(CrossRefError::NoWorkspace);
        }
        let matches = client
            .lookup_symbols_by_name_path(workspace_id, name, path, 20)
            .await?;
        let mut exact = matches
            .into_iter()
            .filter(|symbol| symbol_matches_exact_path_name(symbol, path, name));
        let symbol = exact
            .next()
            .ok_or_else(|| CrossRefError::NotFound(symbol_ref.to_owned()))?;
        if exact.next().is_some() {
            return Err(CrossRefError::Backend(format!(
                "ambiguous exact code symbol projection for {symbol_ref}"
            )));
        }
        return code_ref_from_symbol(symbol_ref, symbol);
    }

    let response = client.get_symbol(symbol_ref).await?;
    if response.symbol.symbol_entity_id != symbol_ref {
        return Err(CrossRefError::IdentityMismatch {
            requested: symbol_ref.to_owned(),
            returned: response.symbol.symbol_entity_id,
        });
    }
    code_ref_from_symbol(symbol_ref, response.symbol)
}

/// Find the notes (rich documents) that reference a code symbol, the code->notes direction. Reuses the
/// VERIFIED `POST /workspaces/{ws}/loom/search-v2` route (see the module-level ENDPOINT CHOICE note),
/// querying the `symbol_key` restricted to rich-document content types to cut false positives
/// (RISK-1 / MC-1). The result feeds the [`NoteRefsPanel`](crate::code_editor::note_refs_panel).
///
/// Errors: an empty workspace -> [`CrossRefError::NoWorkspace`]; a backend failure ->
/// [`CrossRefError::Backend`]. An empty (zero-hit) result is `Ok(vec![])`, NOT an error (the panel
/// shows an honest "No notes reference this symbol" empty state).
pub async fn find_notes_referencing_symbol(
    symbol_key: &str,
    workspace_id: &str,
) -> Result<Vec<NoteRef>, CrossRefError> {
    find_notes_with(&FindNotesHttp::production(), symbol_key, workspace_id).await
}

/// [`find_notes_referencing_symbol`] against an explicit [`FindNotesSearch`] backend (a counted mock in
/// the unit tests; the reqwest impl in production). This is the single search path both the production
/// helper and the test mock drive — the content-type restriction + result mapping live HERE so they
/// are unit-tested without a backend.
pub async fn find_notes_with(
    backend: &dyn FindNotesSearch,
    symbol_key: &str,
    workspace_id: &str,
) -> Result<Vec<NoteRef>, CrossRefError> {
    if workspace_id.trim().is_empty() {
        return Err(CrossRefError::NoWorkspace);
    }
    let symbol_key = symbol_key.trim();
    if symbol_key.is_empty() {
        return Ok(Vec::new());
    }
    // RichDocument search projection indexes an hsLink's operator-facing label. `/code-ref` stores the
    // stable symbol entity id in `refValue` and the code symbol's display name in `label`; therefore a
    // full `<language>:<path>#<symbol>` key is not present in the searchable plain-text projection.
    // Search the exact simple symbol name extracted from the canonical key so the code pane can recover
    // notes authored through `/code-ref`. A non-key input (the backend-loss/raw-word fallback) is used
    // unchanged. The content-type filters and workspace boundary below still bound the candidate set.
    let search_query = note_ref_search_query(symbol_key);
    // Restrict to rich-doc content types one at a time (the search-v2 body's `content_type` filter is a
    // single value), merging + de-duplicating by block id so a symbol mentioned in both a `note` and a
    // `journal` is listed once (RISK-1: tighter than an unfiltered full-text query).
    //
    // PARTIAL-FAILURE ROBUSTNESS (must-fix hardening): do NOT propagate a single content-type query
    // failure with `?` (that would discard the hits already collected from the other content types).
    // Collect every successful query's hits and remember the LAST error; surface an error ONLY when
    // EVERY query failed (so one flaky content type cannot blank a panel that the other type populated).
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    let mut last_err: Option<CrossRefError> = None;
    let mut any_ok = false;
    for content_type in NOTE_REF_CONTENT_TYPES {
        let body =
            LoomSearchV2Body::baseline(search_query.to_owned(), Some((*content_type).to_owned()));
        match backend.search(workspace_id, &body).await {
            Ok(response) => {
                any_ok = true;
                for hit in response.hits {
                    let note = NoteRef::from_hit(hit);
                    if seen.insert(note.block_id.clone()) {
                        out.push(note);
                    }
                }
            }
            Err(e) => last_err = Some(e),
        }
    }
    // Every query failed (e.g. the backend is down) -> surface the typed error. At least one succeeded
    // -> return the merged hits (an empty vec is the honest "no notes" state, never an error).
    if !any_ok {
        return Err(last_err
            .unwrap_or_else(|| CrossRefError::Backend("no content-type queries ran".to_owned())));
    }
    Ok(out)
}

fn note_ref_search_query(symbol_key: &str) -> &str {
    symbol_key
        .rsplit_once('#')
        .map(|(_, name)| name.trim())
        .filter(|name| !name.is_empty())
        .unwrap_or(symbol_key)
}

/// Exhaust the backend-authoritative `limit`/`offset` candidate pages needed by an exact structured
/// reverse lookup. Unlike the operator-facing NoteRefs panel, this path cannot stop at the first 25
/// ranked text matches: a real `hsLink` may rank below many plain-text false positives. The backend's
/// `total` is checked on every page, duplicate/non-progressing pages fail closed, and the scan refuses
/// to cross [`NOTE_REFS_MAX_CANDIDATES`] instead of returning a false empty/partial exact result.
pub async fn find_all_note_candidates_with(
    backend: &dyn FindNotesSearch,
    symbol_key: &str,
    workspace_id: &str,
) -> Result<Vec<NoteRef>, CrossRefError> {
    if workspace_id.trim().is_empty() {
        return Err(CrossRefError::NoWorkspace);
    }
    let search_query = note_ref_search_query(symbol_key);
    let mut seen_across_types = std::collections::HashSet::new();
    let mut candidates = Vec::new();

    for content_type in NOTE_REF_CONTENT_TYPES {
        let mut seen_in_content_type = std::collections::HashSet::new();
        let mut offset = 0u32;
        let mut expected_total: Option<usize> = None;
        loop {
            let mut body = LoomSearchV2Body::baseline(
                search_query.to_owned(),
                Some((*content_type).to_owned()),
            );
            body.offset = offset;
            let response = backend.search(workspace_id, &body).await?;
            let total = usize::try_from(response.total).map_err(|_| {
                CrossRefError::Backend(format!(
                    "loom search-v2 returned invalid negative total {} for {content_type}",
                    response.total
                ))
            })?;
            if total > NOTE_REFS_MAX_CANDIDATES {
                return Err(CrossRefError::Backend(format!(
                    "exact code-reference candidate search returned {total} {content_type} hits; bounded maximum is {NOTE_REFS_MAX_CANDIDATES}"
                )));
            }
            if let Some(expected) = expected_total {
                if total != expected {
                    return Err(CrossRefError::Backend(format!(
                        "loom search-v2 total changed during exact {content_type} pagination ({expected} -> {total})"
                    )));
                }
            } else {
                expected_total = Some(total);
            }

            let page_len = response.hits.len();
            if page_len > NOTE_REFS_SEARCH_LIMIT as usize {
                return Err(CrossRefError::Backend(format!(
                    "loom search-v2 returned {page_len} rows for limit {NOTE_REFS_SEARCH_LIMIT}"
                )));
            }
            let offset_usize = offset as usize;
            if offset_usize > total || page_len > total.saturating_sub(offset_usize) {
                return Err(CrossRefError::Backend(format!(
                    "loom search-v2 page offset={offset} rows={page_len} exceeds reported total={total}"
                )));
            }
            for hit in response.hits {
                let note = NoteRef::from_hit(hit);
                if !seen_in_content_type.insert(note.block_id.clone()) {
                    return Err(CrossRefError::Backend(format!(
                        "loom search-v2 pagination repeated block_id {}",
                        note.block_id
                    )));
                }
                if seen_across_types.insert(note.block_id.clone()) {
                    candidates.push(note);
                }
            }

            let consumed = offset_usize + page_len;
            if consumed == total {
                break;
            }
            if page_len < NOTE_REFS_SEARCH_LIMIT as usize {
                return Err(CrossRefError::Backend(format!(
                    "loom search-v2 returned a short exact {content_type} page at offset {offset} before reported total {total}"
                )));
            }
            offset = offset.checked_add(NOTE_REFS_SEARCH_LIMIT).ok_or_else(|| {
                CrossRefError::Backend("exact code-reference pagination offset overflow".to_owned())
            })?;
        }
    }

    Ok(candidates)
}

fn content_has_exact_code_ref(
    node: &serde_json::Value,
    symbol_entity_id: &str,
    symbol_key: &str,
) -> bool {
    if node.get("type").and_then(serde_json::Value::as_str) == Some("hsLink") {
        let attrs = node.get("attrs");
        let ref_kind = attrs
            .and_then(|attrs| attrs.get("refKind"))
            .and_then(serde_json::Value::as_str);
        let ref_value = attrs
            .and_then(|attrs| attrs.get("refValue"))
            .and_then(serde_json::Value::as_str)
            .map(str::trim);
        if ref_kind == Some(CODE_REF_KIND) {
            if ref_value == Some(symbol_entity_id) || ref_value == Some(symbol_key) {
                return true;
            }
            if let (Some(value), Some((expected_path, expected_name))) =
                (ref_value, parse_path_symbol_ref(symbol_key))
            {
                if let Some((actual_path, actual_name)) = parse_path_symbol_ref(value) {
                    if actual_name == expected_name
                        && normalized_symbol_path(actual_path)
                            == normalized_symbol_path(expected_path)
                    {
                        return true;
                    }
                }
            }
        }
    }
    node.get("content")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|children| {
            children
                .iter()
                .any(|child| content_has_exact_code_ref(child, symbol_entity_id, symbol_key))
        })
}

/// Exact code-reference reverse lookup. Search-v2 is used only to produce a bounded candidate set;
/// every candidate rich document is then read through the existing document GET and retained only if
/// its persisted `content_json` contains `hsLink(refKind="code")` with the exact symbol entity id or
/// exact file-qualified symbol key. Plain mentions and same-name symbols in another file are excluded.
pub async fn find_code_ref_notes_with(
    backend: &dyn FindNotesSearch,
    symbol_entity_id: &str,
    symbol_key: &str,
    workspace_id: &str,
) -> Result<Vec<NoteRef>, CrossRefError> {
    let symbol_entity_id = symbol_entity_id.trim();
    let symbol_key = symbol_key.trim();
    if symbol_entity_id.is_empty() || symbol_key.is_empty() {
        return Err(CrossRefError::EmptySymbol);
    }
    let candidates = find_all_note_candidates_with(backend, symbol_key, workspace_id).await?;
    let mut verified = Vec::new();
    let mut verified_documents = std::collections::HashSet::new();
    let mut last_error = None;
    for candidate in candidates {
        match backend.load_document_content(&candidate.document_id).await {
            Ok(content) => {
                if content_has_exact_code_ref(&content, symbol_entity_id, symbol_key)
                    && verified_documents.insert(candidate.document_id.clone())
                {
                    verified.push(candidate);
                }
            }
            Err(error) => last_error = Some(error),
        }
    }
    // Candidate search is only a prefilter. If any candidate cannot be read back, the exact result is
    // unknowable: the failed document may be the one carrying the structured code hsLink. Returning a
    // successful empty/partial list would falsely claim completeness, so exact reverse lookup fails
    // closed even when another false-positive candidate loaded successfully.
    if let Some(error) = last_error {
        return Err(error);
    }
    Ok(verified)
}

/// Bridge a clicked code-ref chip ([`EditorEvent::WikilinkActivated`] with `ref_kind="code"`) to the
/// cross-pane Open-Code-Symbol command on the [`InteractionBus`](crate::interop::InteractionBus) — the
/// note->code dispatch (AC-2). The MT-015 chip renderer reports a clicked `hsLink` atom as a
/// `WikilinkActivated` event carrying `ref_kind`/`ref_value`; for a code ref the `ref_value` is the
/// symbol resolution key (the symbol entity id when inserted via `/code-ref`, or the `path#Symbol`
/// symbol key when authored as `[[code:…]]` syntax). This stages it on the bus and dispatches
/// [`CMD_OPEN_CODE_SYMBOL`](crate::interop::CMD_OPEN_CODE_SYMBOL), so the click fires the ONE named
/// cross-pane command (not a per-pane ad-hoc callback). The shell drains the staged id and routes it
/// through the MT-030 ShellNavigator `open_code_symbol` seam.
///
/// Returns `Some(symbol_ref)` when a code-ref was dispatched, `None` for a non-code event (a `wp`/
/// `note`/… wikilink routes through the open-document path instead — this bridge handles only code).
/// The caller must have run
/// [`InteractionBus::register_open_code_symbol_command`](crate::interop::InteractionBus::register_open_code_symbol_command)
/// once (the command is then always present).
pub fn dispatch_code_ref_open(
    ctx: &egui::Context,
    bus: &mut crate::interop::InteractionBus,
    event: &crate::rich_editor::wikilinks::inline_view::EditorEvent,
) -> Option<String> {
    use crate::rich_editor::wikilinks::inline_view::EditorEvent;
    if let EditorEvent::WikilinkActivated {
        ref_kind,
        ref_value,
        ..
    } = event
    {
        if ref_kind == CODE_REF_KIND {
            bus.open_code_symbol(ctx, ref_value.clone());
            return Some(ref_value.clone());
        }
    }
    None
}

/// Bridge a clicked LOCUS-ref chip ([`EditorEvent::WikilinkActivated`] with `ref_kind="locus"`) to the
/// cross-pane Open-Locus-Ref command on the [`InteractionBus`](crate::interop::InteractionBus) — the
/// editors->Locus dispatch (WP-KERNEL-012 MT-068, AC-003). The SIBLING of [`dispatch_code_ref_open`]: the
/// MT-015 chip renderer reports a clicked `hsLink` atom as a `WikilinkActivated` event carrying
/// `ref_kind`/`ref_value`; for a Locus ref the `ref_value` is the `locus://` ref (the WP/MT resolution
/// key). This stages the canonical original-case URI on the bus and dispatches
/// [`CMD_OPEN_LOCUS_REF`](crate::interop::CMD_OPEN_LOCUS_REF), so the click fires the ONE named cross-pane
/// command (not a per-pane ad-hoc callback). The shell drains the staged ref and routes it through the
/// SAME MT-030 nav seam the other cross-refs use (NO new navigation channel — RISK-007).
///
/// The staged value is the canonical original-case `locus://` URI (via
/// [`crate::interop::locus_interop::LocusRef::to_uri`]) when the `ref_value` parses. WP/MT identifiers are
/// case-significant record identities, so [`crate::interop::locus_interop::LocusRef::normalized`] remains
/// lookup-only and must never become a navigation payload. A non-parsing value still dispatches the raw
/// ref (the shell shows a typed "cannot resolve" state rather than silently dropping). Returns
/// `Some(staged_ref)` when a locus-ref was dispatched, `None` for a non-locus event (a
/// `code`/`wp`/`note`/… ref routes through its own path).
pub fn dispatch_locus_ref_open(
    ctx: &egui::Context,
    bus: &mut crate::interop::InteractionBus,
    event: &crate::rich_editor::wikilinks::inline_view::EditorEvent,
) -> Option<String> {
    use crate::rich_editor::wikilinks::inline_view::EditorEvent;
    if let EditorEvent::WikilinkActivated {
        ref_kind,
        ref_value,
        ..
    } = event
    {
        if ref_kind == crate::interop::locus_interop::LOCUS_REF_KIND {
            // Stage the canonical original-case `locus://` URI. `normalized` is a lookup/search key,
            // not a navigation identity: staging it would silently lowercase case-significant WP/MT ids.
            // A raw value that does not parse still dispatches (the shell renders a typed cannot-resolve
            // state).
            let staged = crate::interop::locus_interop::parse_locus_ref(ref_value)
                .map(|r| r.to_uri())
                .unwrap_or_else(|| ref_value.clone());
            bus.open_locus_ref(ctx, staged.clone());
            return Some(staged);
        }
    }
    None
}

/// The search transport behind [`find_notes_referencing_symbol`]. A trait (not hard reqwest calls) so
/// the content-type-restriction + hit-mapping + de-dup logic is unit-testable with a counted mock and
/// NO backend (the proven MT-014/MT-015 fetcher-trait pattern). The production impl
/// ([`FindNotesHttp`]) reuses the existing reqwest stack.
pub trait FindNotesSearch: Send + Sync {
    /// Run one loom search-v2 query (already carrying the content-type filter) and return the parsed
    /// response, or a typed [`CrossRefError`] on failure.
    fn search<'a>(
        &'a self,
        workspace_id: &'a str,
        body: &'a LoomSearchV2Body,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<LoomSearchV2Response, CrossRefError>>
                + Send
                + 'a,
        >,
    >;

    /// Read one candidate rich document's authoritative `content_json` for exact structured-link
    /// verification. Generic reverse-lookup users do not call this; MT-034's code-ref path does.
    fn load_document_content<'a>(
        &'a self,
        document_id: &'a str,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<serde_json::Value, CrossRefError>> + Send + 'a>,
    > {
        let document_id = document_id.to_owned();
        Box::pin(async move {
            Err(CrossRefError::Backend(format!(
                "exact rich-document readback is unavailable for {document_id}"
            )))
        })
    }

    /// Resolve one Loom search-hit block through the backend's canonical transclusion route.
    /// The returned content is the complete source rich document, not a block-local slice;
    /// callers must also enforce the canonical native projection identity (`block_id ==
    /// source_document_id`) before treating it as exact block evidence.
    fn load_block_transclusion<'a>(
        &'a self,
        workspace_id: &'a str,
        block_id: &'a str,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<(String, serde_json::Value), CrossRefError>>
                + Send
                + 'a,
        >,
    > {
        let workspace_id = workspace_id.to_owned();
        let block_id = block_id.to_owned();
        Box::pin(async move {
            Err(CrossRefError::Backend(format!(
                "exact Loom block readback is unavailable for {workspace_id}/{block_id}"
            )))
        })
    }
}

/// The production [`FindNotesSearch`]: a thin reqwest wrapper over the VERIFIED
/// `POST /workspaces/{ws}/loom/search-v2` route (the same route MT-015/MT-028 use). Read-only; no
/// backend code is touched. REUSES the existing reqwest 0.12 + rustls stack — NO new HTTP crate.
#[derive(Clone)]
pub struct FindNotesHttp {
    client: reqwest::Client,
    base_url: String,
}

impl FindNotesHttp {
    /// Build against an explicit base URL (a test points it at a live backend).
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: crate::backend_client::shared_http_client(),
            base_url: base_url.into(),
        }
    }

    /// The production client against the hardcoded backend base URL.
    pub fn production() -> Self {
        Self::new(BACKEND_BASE_URL)
    }
}

impl FindNotesSearch for FindNotesHttp {
    fn search<'a>(
        &'a self,
        workspace_id: &'a str,
        body: &'a LoomSearchV2Body,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<LoomSearchV2Response, CrossRefError>>
                + Send
                + 'a,
        >,
    > {
        let url = format!(
            "{}/workspaces/{}/loom/search-v2",
            self.base_url, workspace_id
        );
        let client = self.client.clone();
        let body = body.clone();
        Box::pin(async move {
            let response =
                client.post(&url).json(&body).send().await.map_err(|e| {
                    CrossRefError::Backend(format!("find-notes search failed: {e}"))
                })?;
            let status = response.status();
            if status.as_u16() == 404 {
                return Err(CrossRefError::NotFound("loom search-v2".to_owned()));
            }
            if !status.is_success() {
                return Err(CrossRefError::Backend(format!(
                    "loom search-v2 returned HTTP {status}"
                )));
            }
            response
                .json::<LoomSearchV2Response>()
                .await
                .map_err(|e| CrossRefError::Backend(format!("loom search-v2 body invalid: {e}")))
        })
    }

    fn load_document_content<'a>(
        &'a self,
        document_id: &'a str,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<serde_json::Value, CrossRefError>> + Send + 'a>,
    > {
        let client = crate::backend::knowledge_documents::KnowledgeDocumentsClient::with_client(
            self.client.clone(),
            self.base_url.clone(),
        );
        let headers = crate::backend::knowledge_documents::HskDocumentHeaders::for_read(
            "mt034-note-ref-readback",
            document_id,
        );
        Box::pin(async move {
            use crate::backend::knowledge_documents::KnowledgeDocumentsError;
            let response = client
                .load_document(&headers, document_id)
                .await
                .map_err(|error| match error {
                    KnowledgeDocumentsError::NotFound(detail) => CrossRefError::NotFound(detail),
                    other => CrossRefError::Backend(format!(
                        "exact rich-document readback failed: {other}"
                    )),
                })?;
            response
                .document
                .get("content_json")
                .cloned()
                .filter(|content| content.is_object())
                .ok_or_else(|| {
                    CrossRefError::Backend(format!(
                        "document {document_id} omitted object content_json"
                    ))
                })
        })
    }

    fn load_block_transclusion<'a>(
        &'a self,
        workspace_id: &'a str,
        block_id: &'a str,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<(String, serde_json::Value), CrossRefError>>
                + Send
                + 'a,
        >,
    > {
        #[derive(serde::Deserialize)]
        struct LoomTransclusionWire {
            source_document_id: Option<String>,
            content_json: Option<serde_json::Value>,
            resolved: bool,
        }

        let url = format!(
            "{}/workspaces/{}/loom/blocks/{}/transclusion",
            self.base_url,
            percent_encode_symbol(workspace_id),
            percent_encode_symbol(block_id)
        );
        let client = self.client.clone();
        let block_id = block_id.to_owned();
        Box::pin(async move {
            let response = client.get(&url).send().await.map_err(|error| {
                CrossRefError::Backend(format!(
                    "exact Loom block {block_id} readback failed: {error}"
                ))
            })?;
            let status = response.status();
            if status.as_u16() == 404 {
                return Err(CrossRefError::NotFound(format!("Loom block {block_id}")));
            }
            if !status.is_success() {
                return Err(CrossRefError::Backend(format!(
                    "exact Loom block {block_id} readback returned HTTP {status}"
                )));
            }
            let body = response
                .json::<LoomTransclusionWire>()
                .await
                .map_err(|error| {
                    CrossRefError::Backend(format!(
                        "exact Loom block {block_id} readback body invalid: {error}"
                    ))
                })?;
            if !body.resolved {
                return Err(CrossRefError::NotFound(format!(
                    "Loom block {block_id} rich document"
                )));
            }
            let document_id = body.source_document_id.ok_or_else(|| {
                CrossRefError::Backend(format!(
                    "exact Loom block {block_id} readback omitted source_document_id"
                ))
            })?;
            let content_json = body
                .content_json
                .filter(serde_json::Value::is_object)
                .ok_or_else(|| {
                    CrossRefError::Backend(format!(
                        "exact Loom block {block_id} readback omitted object content_json"
                    ))
                })?;
            Ok((document_id, content_json))
        })
    }
}

/// Tracks the cursor dwell on a code symbol so the NoteRefsPanel search fires ONCE per dwell crossing
/// the 800ms threshold, and NEVER on a cursor move (RISK-3 / MC-3 — no backend spam).
///
/// The code pane calls [`Self::observe`] each frame with the symbol entity id under the cursor (or
/// `None` when the cursor is not on a symbol) and the current [`Instant`]. When the cursor SETTLES on a
/// NEW symbol for >= [`NOTE_REFS_DWELL_MS`], [`Self::observe`] returns `Some(symbol_entity_id)` exactly
/// once (the search trigger); subsequent frames on the SAME symbol return `None` (already fired). Any
/// change of symbol RESETS the timer, so scanning across symbols fires zero searches.
#[derive(Debug, Clone, Default)]
pub struct SymbolDwellTracker {
    /// The symbol the cursor is currently dwelling on + when the dwell started. `None` when the cursor
    /// is off any symbol.
    dwelling: Option<(String, Instant)>,
    /// The last symbol a search was FIRED for (so the same dwell does not re-fire each frame).
    fired_for: Option<String>,
}

impl SymbolDwellTracker {
    /// A fresh tracker (no dwell, nothing fired).
    pub fn new() -> Self {
        Self::default()
    }

    /// Observe the symbol under the cursor this frame. Returns `Some(symbol_entity_id)` EXACTLY ONCE
    /// when the cursor has dwelled on a symbol distinct from the last-fired one for >= the dwell
    /// threshold; `None` otherwise. A `current` of `None` (cursor off any symbol) clears the dwell but
    /// keeps `fired_for` so re-entering the same symbol without a move does not refire spuriously.
    pub fn observe(&mut self, current: Option<&str>, now: Instant) -> Option<String> {
        self.observe_with_threshold(current, now, Duration::from_millis(NOTE_REFS_DWELL_MS))
    }

    /// [`Self::observe`] with an explicit threshold (the unit tests inject a tiny/zero window to prove
    /// the fire-once + reset-on-move + no-refire semantics deterministically without sleeping).
    pub fn observe_with_threshold(
        &mut self,
        current: Option<&str>,
        now: Instant,
        threshold: Duration,
    ) -> Option<String> {
        match current {
            None => {
                // Cursor left all symbols: drop the in-flight dwell (the timer resets), but keep
                // `fired_for` so a same-symbol re-entry without an intervening DIFFERENT symbol does
                // not immediately refire.
                self.dwelling = None;
                None
            }
            Some(symbol) => {
                match &self.dwelling {
                    // Same symbol still under the cursor: check whether it has now crossed the dwell
                    // threshold AND has not already fired.
                    Some((s, started)) if s == symbol => {
                        let crossed = now.duration_since(*started) >= threshold;
                        let already_fired = self.fired_for.as_deref() == Some(symbol);
                        if crossed && !already_fired {
                            self.fired_for = Some(symbol.to_owned());
                            Some(symbol.to_owned())
                        } else {
                            None
                        }
                    }
                    // A DIFFERENT symbol (or first observation / re-entry after the cursor left all
                    // symbols): reset the dwell timer to `now`. Cursor MOVED => no fire yet.
                    //
                    // NO-REFIRE on same-symbol re-entry (RISK-3 / MC-3 — the live backend-spam guard once
                    // wired): clear `fired_for` ONLY when re-entering a DIFFERENT symbol than the one we
                    // last fired for. Re-entering the SAME symbol (cursor left to `None`, then came back
                    // without crossing an intervening different symbol) keeps `fired_for`, so settling on
                    // it again does NOT refire the search. A genuine move to another symbol clears the
                    // marker so that symbol fires once.
                    _ => {
                        self.dwelling = Some((symbol.to_owned(), now));
                        if self.fired_for.as_deref() != Some(symbol) {
                            self.fired_for = None;
                        }
                        None
                    }
                }
            }
        }
    }

    /// The symbol currently being dwelled on, if any (for diagnostics / the panel's "loading for X").
    pub fn current_symbol(&self) -> Option<&str> {
        self.dwelling.as_ref().map(|(s, _)| s.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend_client::{LoomSearchBlock, LoomSearchV2Hit, LoomSearchV2Response};
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn hit(
        block_id: &str,
        title: Option<&str>,
        content_type: &str,
        highlight: &str,
    ) -> LoomSearchV2Hit {
        LoomSearchV2Hit {
            block: LoomSearchBlock {
                block_id: block_id.to_owned(),
                content_type: content_type.to_owned(),
                document_id: None,
                title: title.map(str::to_owned),
            },
            score: 1.0,
            fts_rank: 0.0,
            trgm_sim: 0.0,
            vector_sim: 0.0,
            edge_degree: 0,
            highlight: highlight.to_owned(),
        }
    }

    /// RISK-2 / MC-2: a symbol key with `/`, `#`, and `::` percent-encodes so it embeds in a URL
    /// without breaking routing. The unreserved chars (letters/digits/`-_.~`) pass through.
    #[test]
    fn percent_encode_handles_slash_hash_and_colons() {
        let key = "fn:src/main.rs#MyStruct::new";
        let encoded = percent_encode_symbol(key);
        assert!(!encoded.contains('/'), "slash must be encoded");
        assert!(!encoded.contains('#'), "hash must be encoded");
        assert!(!encoded.contains(':'), "colon must be encoded");
        assert_eq!(encoded, "fn%3Asrc%2Fmain.rs%23MyStruct%3A%3Anew");
        // The unreserved chars survive verbatim.
        assert_eq!(percent_encode_symbol("Abc-_.~9"), "Abc-_.~9");
    }

    #[test]
    fn path_symbol_parser_preserves_windows_absolute_paths_and_strips_only_language_prefixes() {
        assert_eq!(
            parse_path_symbol_ref(r"D:\code\src\main.rs#MyStruct"),
            Some((r"D:\code\src\main.rs", "MyStruct"))
        );
        assert_eq!(
            parse_path_symbol_ref(r"\\?\D:\code\src\main.rs#MyStruct"),
            Some((r"\\?\D:\code\src\main.rs", "MyStruct"))
        );
        assert_eq!(
            parse_path_symbol_ref("file:///D:/code/src/main.rs#MyStruct"),
            Some(("file:///D:/code/src/main.rs", "MyStruct"))
        );
        assert_eq!(
            parse_path_symbol_ref("rust:src/main.rs#MyStruct"),
            Some(("src/main.rs", "MyStruct"))
        );
    }

    #[test]
    fn note_ref_from_hit_strips_mark_and_falls_back_to_block_id() {
        let n = NoteRef::from_hit(hit("BLK-1", None, "note", "see <mark>MyStruct</mark> here"));
        assert_eq!(n.block_id, "BLK-1");
        assert_eq!(n.document_id, "BLK-1");
        assert_eq!(
            n.document_title, "BLK-1",
            "untitled block falls back to its id"
        );
        assert_eq!(
            n.excerpt, "see MyStruct here",
            "the <mark> markers are stripped"
        );
        let titled = NoteRef::from_hit(hit("BLK-2", Some("My Note"), "document", ""));
        assert_eq!(titled.document_title, "My Note");
    }

    #[test]
    fn note_ref_from_hit_uses_backend_document_id_when_present() {
        let hit: LoomSearchV2Hit = serde_json::from_value(serde_json::json!({
            "block": {
                "block_id": "BLK-7",
                "document_id": "DOC-7",
                "content_type": "note",
                "title": "Design notes"
            },
            "score": 1.0,
            "highlight": "uses <mark>MyStruct</mark> here"
        }))
        .expect("backend search-v2 hit JSON should deserialize");
        let note = NoteRef::from_hit(hit);
        assert_eq!(note.block_id, "BLK-7");
        assert_eq!(
            note.document_id, "DOC-7",
            "real search-v2 hits open the rich document id, not the matched block id"
        );
        assert_eq!(note.document_title, "Design notes");
        assert_eq!(note.excerpt, "uses MyStruct here");
    }

    #[test]
    fn cross_ref_error_kind_strings_and_unresolved_flag() {
        assert_eq!(CrossRefError::NoWorkspace.kind_str(), "no_workspace");
        assert_eq!(CrossRefError::NotFound("x".into()).kind_str(), "not_found");
        assert_eq!(
            CrossRefError::NoDefinition("x".into()).kind_str(),
            "no_definition"
        );
        assert_eq!(
            CrossRefError::IdentityMismatch {
                requested: "A".into(),
                returned: "B".into()
            }
            .kind_str(),
            "identity_mismatch"
        );
        assert!(CrossRefError::NotFound("x".into()).is_unresolved());
        assert!(CrossRefError::NoDefinition("x".into()).is_unresolved());
        assert!(CrossRefError::EmptySymbol.is_unresolved());
        assert!(
            !CrossRefError::Backend("down".into()).is_unresolved(),
            "transient backend error is not unresolved"
        );
        assert!(!CrossRefError::NoWorkspace.is_unresolved());
    }

    #[test]
    fn app_error_404_maps_to_not_found() {
        let nf: CrossRefError =
            AppError::Http("GET code-nav non-success status 404 Not Found".into()).into();
        assert!(
            matches!(nf, CrossRefError::NotFound(_)),
            "a 404 status maps to unresolved/not-found"
        );
        let be: CrossRefError = AppError::Http("503 Service Unavailable".into()).into();
        assert!(
            matches!(be, CrossRefError::Backend(_)),
            "a non-404 status is a generic backend error"
        );
    }

    #[test]
    fn literal_code_ref_exact_filter_rejects_same_name_wrong_file() {
        let wrong = CodeSymbolNavProjection {
            symbol_entity_id: "KEN-WRONG".to_owned(),
            symbol_key: "rust:src/other.rs#Symbol".to_owned(),
            display_name: "Symbol".to_owned(),
            ..Default::default()
        };
        let exact = CodeSymbolNavProjection {
            symbol_entity_id: "KEN-EXACT".to_owned(),
            symbol_key: "rust:src/target.rs#Symbol".to_owned(),
            display_name: "Symbol".to_owned(),
            ..Default::default()
        };
        assert!(!symbol_matches_exact_path_name(
            &wrong,
            "src/target.rs",
            "Symbol"
        ));
        assert!(symbol_matches_exact_path_name(
            &exact,
            "src/target.rs",
            "Symbol"
        ));
    }

    // A counted in-memory search mock (the MT-014/MT-015 counted-mock pattern; NO backend).
    struct MockSearch {
        // The hits returned per content_type (note -> ..., journal -> ...).
        by_content_type: std::collections::HashMap<String, Vec<LoomSearchV2Hit>>,
        // When true, EVERY query returns a typed backend error (the all-fail path).
        fail: bool,
        // When true, every query AFTER the first returns a typed backend error (the partial-failure path).
        #[allow(dead_code)]
        fail_after_first: bool,
        calls: AtomicUsize,
        documents: std::collections::HashMap<String, serde_json::Value>,
    }
    impl MockSearch {
        // Convenience ctor for the existing tests that do not exercise the failure paths.
        fn ok(by_content_type: std::collections::HashMap<String, Vec<LoomSearchV2Hit>>) -> Self {
            Self {
                by_content_type,
                fail: false,
                fail_after_first: false,
                calls: AtomicUsize::new(0),
                documents: Default::default(),
            }
        }
    }
    impl FindNotesSearch for MockSearch {
        fn search<'a>(
            &'a self,
            _workspace_id: &'a str,
            body: &'a LoomSearchV2Body,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<Output = Result<LoomSearchV2Response, CrossRefError>>
                    + Send
                    + 'a,
            >,
        > {
            let call_index = self.calls.fetch_add(1, Ordering::SeqCst);
            let fail = self.fail || (self.fail_after_first && call_index >= 1);
            let ct = body.content_type.clone().unwrap_or_default();
            let all_hits = self.by_content_type.get(&ct).cloned().unwrap_or_default();
            let total = all_hits.len() as i64;
            let hits = all_hits
                .into_iter()
                .skip(body.offset as usize)
                .take(body.limit as usize)
                .collect();
            Box::pin(async move {
                if fail {
                    return Err(CrossRefError::Backend(
                        "mock content-type query failed".to_owned(),
                    ));
                }
                Ok(LoomSearchV2Response {
                    hits,
                    content_type_facets: Default::default(),
                    semantic_available: false,
                    total,
                })
            })
        }

        fn load_document_content<'a>(
            &'a self,
            document_id: &'a str,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<Output = Result<serde_json::Value, CrossRefError>>
                    + Send
                    + 'a,
            >,
        > {
            let value = self.documents.get(document_id).cloned();
            Box::pin(
                async move { value.ok_or_else(|| CrossRefError::NotFound(document_id.to_owned())) },
            )
        }
    }

    fn block_on<F: std::future::Future>(f: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(f)
    }

    #[test]
    fn find_notes_requires_workspace() {
        let mock = MockSearch::ok(Default::default());
        let r = block_on(find_notes_with(&mock, "fn:src/main.rs#MyStruct", ""));
        assert_eq!(r, Err(CrossRefError::NoWorkspace));
        assert_eq!(
            mock.calls.load(Ordering::SeqCst),
            0,
            "no backend call without a workspace"
        );
    }

    #[test]
    fn find_notes_empty_symbol_is_empty_not_error() {
        let mock = MockSearch::ok(Default::default());
        let r = block_on(find_notes_with(&mock, "  ", "ws-1")).unwrap();
        assert!(
            r.is_empty(),
            "an empty symbol yields an empty list, not an error"
        );
        assert_eq!(mock.calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn find_notes_merges_content_types_and_dedups() {
        // The same block matched under both `note` and `journal` is listed ONCE (RISK-1 dedup); a
        // distinct block in each is kept. The search restricts to rich-doc content types (one call per).
        // Seeds the REAL backend tokens (`note`/`journal`) — never the invalid `document` that 422'd.
        let mut by = std::collections::HashMap::new();
        by.insert(
            "note".to_owned(),
            vec![
                hit("BLK-A", Some("A"), "note", "<mark>S</mark>"),
                hit("BLK-B", Some("B"), "note", "x"),
            ],
        );
        by.insert(
            "journal".to_owned(),
            vec![
                hit("BLK-A", Some("A"), "journal", "y"),
                hit("BLK-C", Some("C"), "journal", "z"),
            ],
        );
        let mock = MockSearch::ok(by);
        let r = block_on(find_notes_with(&mock, "fn:src/main.rs#S", "ws-1")).unwrap();
        let ids: Vec<&str> = r.iter().map(|n| n.block_id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["BLK-A", "BLK-B", "BLK-C"],
            "deduped, in content-type then hit order"
        );
        assert_eq!(
            mock.calls.load(Ordering::SeqCst),
            2,
            "one search per rich-doc content type"
        );
    }

    /// BACKEND-SHAPE GUARD (must-fix): every `NOTE_REF_CONTENT_TYPES` value MUST be a real
    /// `LoomBlockContentType::as_str()` token, so the code->notes search can never send a value the real
    /// backend would reject with HTTP 422 (the `document` drift the MockSearch happy-path previously hid).
    #[test]
    fn note_ref_content_types_are_valid_backend_tokens() {
        for ct in NOTE_REF_CONTENT_TYPES {
            assert!(
                BACKEND_LOOM_CONTENT_TYPE_TOKENS.contains(ct),
                "content_type {ct:?} is NOT a real LoomBlockContentType token (loom.rs:58-69) — it would \
                 422 against real PG; valid tokens: {BACKEND_LOOM_CONTENT_TYPE_TOKENS:?}"
            );
        }
        // The invalid value that 422'd against real PG must NOT have crept back in.
        assert!(
            !NOTE_REF_CONTENT_TYPES.contains(&"document"),
            "`document` is not a LoomBlockContentType variant (it 422s); use `note`/`journal`"
        );
    }

    /// PARTIAL-FAILURE ROBUSTNESS (must-fix hardening): if ONE content-type query fails but another
    /// succeeds, the merged hits from the successful query are returned (not discarded); the panel does
    /// not blank because one content type was flaky.
    #[test]
    fn find_notes_returns_partial_hits_when_one_content_type_fails() {
        let mut by = std::collections::HashMap::new();
        by.insert(
            "note".to_owned(),
            vec![hit("BLK-A", Some("A"), "note", "S")],
        );
        // `journal` returns no seeded hits AND the mock is set to fail the SECOND call below.
        let mock = MockSearch {
            by_content_type: by,
            fail_after_first: true,
            fail: false,
            calls: AtomicUsize::new(0),
            documents: Default::default(),
        };
        let r = block_on(find_notes_with(&mock, "fn:src/main.rs#S", "ws-1")).unwrap();
        let ids: Vec<&str> = r.iter().map(|n| n.block_id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["BLK-A"],
            "the successful query's hit survives a sibling query failure"
        );
    }

    /// PARTIAL-FAILURE ROBUSTNESS (must-fix hardening): if EVERY content-type query fails, the typed
    /// error is surfaced (the panel shows a fail-closed error chip, not a silent empty state).
    #[test]
    fn find_notes_errors_only_when_all_content_types_fail() {
        let mock = MockSearch {
            by_content_type: Default::default(),
            fail_after_first: false,
            fail: true,
            calls: AtomicUsize::new(0),
            documents: Default::default(),
        };
        let r = block_on(find_notes_with(&mock, "fn:src/main.rs#S", "ws-1"));
        assert!(
            matches!(r, Err(CrossRefError::Backend(_))),
            "all-fail surfaces the typed backend error"
        );
    }

    #[test]
    fn exact_reverse_lookup_excludes_plain_mentions_and_same_name_other_file() {
        let mut by = std::collections::HashMap::new();
        by.insert(
            "note".to_owned(),
            vec![
                hit("DOC-PLAIN", Some("Plain"), "note", "Symbol"),
                hit("DOC-WRONG", Some("Wrong file"), "note", "Symbol"),
                hit("DOC-EXACT", Some("Exact"), "note", "Symbol"),
            ],
        );
        let mut documents = std::collections::HashMap::new();
        documents.insert(
            "DOC-PLAIN".to_owned(),
            serde_json::json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"Symbol"}]}]}),
        );
        documents.insert(
            "DOC-WRONG".to_owned(),
            serde_json::json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"hsLink","attrs":{"refKind":"code","refValue":"fn:src/other.rs#Symbol","label":"Symbol","resolved":true}}]}]}),
        );
        documents.insert(
            "DOC-EXACT".to_owned(),
            serde_json::json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"hsLink","attrs":{"refKind":"code","refValue":"KEN-EXACT","label":"Symbol","resolved":true}}]}]}),
        );
        let mock = MockSearch {
            by_content_type: by,
            fail: false,
            fail_after_first: false,
            calls: AtomicUsize::new(0),
            documents,
        };
        let notes = block_on(find_code_ref_notes_with(
            &mock,
            "KEN-EXACT",
            "fn:src/target.rs#Symbol",
            "ws-1",
        ))
        .unwrap();
        assert_eq!(
            notes
                .iter()
                .map(|note| note.document_id.as_str())
                .collect::<Vec<_>>(),
            vec!["DOC-EXACT"],
            "candidate search must not promote plain text or a same-name symbol from another file"
        );
    }

    #[test]
    fn exact_reverse_lookup_fails_closed_when_any_candidate_readback_fails() {
        let mut by = std::collections::HashMap::new();
        by.insert(
            "note".to_owned(),
            vec![
                hit("DOC-PLAIN", Some("Plain"), "note", "Symbol"),
                hit("DOC-UNREADABLE", Some("Potential exact"), "note", "Symbol"),
            ],
        );
        let mut documents = std::collections::HashMap::new();
        documents.insert(
            "DOC-PLAIN".to_owned(),
            serde_json::json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"Symbol"}]}]}),
        );
        let mock = MockSearch {
            by_content_type: by,
            fail: false,
            fail_after_first: false,
            calls: AtomicUsize::new(0),
            documents,
        };

        let result = block_on(find_code_ref_notes_with(
            &mock,
            "KEN-EXACT",
            "fn:src/target.rs#Symbol",
            "ws-1",
        ));
        assert!(
            matches!(result, Err(CrossRefError::NotFound(_))),
            "one readable false positive cannot turn an unreadable exact candidate into Ok(empty): {result:?}"
        );
    }

    #[test]
    fn exact_reverse_lookup_exhausts_pages_past_twenty_five_false_positives() {
        let mut note_hits = (0..30)
            .map(|index| {
                hit(
                    &format!("DOC-FALSE-{index:02}"),
                    Some(&format!("False {index:02}")),
                    "note",
                    "Symbol",
                )
            })
            .collect::<Vec<_>>();
        note_hits.push(hit(
            "DOC-EXACT-PAGE-2",
            Some("Exact structured link"),
            "note",
            "Symbol",
        ));
        let mut by = std::collections::HashMap::new();
        by.insert("note".to_owned(), note_hits);

        let mut documents = std::collections::HashMap::new();
        for index in 0..30 {
            documents.insert(
                format!("DOC-FALSE-{index:02}"),
                serde_json::json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"Symbol"}]}]}),
            );
        }
        documents.insert(
            "DOC-EXACT-PAGE-2".to_owned(),
            serde_json::json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"hsLink","attrs":{"refKind":"code","refValue":"KEN-PAGED","label":"Symbol","resolved":true}}]}]}),
        );
        let mock = MockSearch {
            by_content_type: by,
            fail: false,
            fail_after_first: false,
            calls: AtomicUsize::new(0),
            documents,
        };

        let notes = block_on(find_code_ref_notes_with(
            &mock,
            "KEN-PAGED",
            "fn:src/target.rs#Symbol",
            "ws-1",
        ))
        .unwrap();
        assert_eq!(
            notes
                .iter()
                .map(|note| note.document_id.as_str())
                .collect::<Vec<_>>(),
            vec!["DOC-EXACT-PAGE-2"],
            "the exact hsLink after more than 25 higher-ranked false positives must be found"
        );
        assert_eq!(
            mock.calls.load(Ordering::SeqCst),
            3,
            "note requires two limit/offset pages and journal one empty page"
        );
    }

    /// RISK-3 / MC-3 (the live backend-spam guard, now that the tracker is WIRED): the cursor leaves all
    /// symbols (`None`) and re-enters the SAME symbol without an intervening DIFFERENT symbol. The search
    /// must NOT refire (it already fired for that symbol) — otherwise a hover-jiggle would spam the
    /// backend. A genuine move to a DIFFERENT symbol still fires once for the new symbol.
    #[test]
    fn dwell_does_not_refire_on_same_symbol_reentry() {
        let mut tracker = SymbolDwellTracker::new();
        let z = Duration::from_millis(0);
        let t = Instant::now();
        // Settle on S1 and fire once.
        assert_eq!(tracker.observe_with_threshold(Some("S1"), t, z), None);
        assert_eq!(
            tracker.observe_with_threshold(Some("S1"), t, z),
            Some("S1".to_owned())
        );
        // Cursor leaves all symbols.
        assert_eq!(tracker.observe_with_threshold(None, t, z), None);
        // Re-enter the SAME symbol: this is a move frame (no fire), and crucially `fired_for` is KEPT.
        assert_eq!(
            tracker.observe_with_threshold(Some("S1"), t, z),
            None,
            "re-entry move frame: no fire"
        );
        // Settling again must NOT refire (no backend spam from a hover-jiggle).
        assert_eq!(
            tracker.observe_with_threshold(Some("S1"), t, z),
            None,
            "same-symbol re-entry must not refire"
        );
        // But a genuine move to a DIFFERENT symbol DOES fire once.
        assert_eq!(
            tracker.observe_with_threshold(Some("S2"), t, z),
            None,
            "move to S2: no fire on move frame"
        );
        assert_eq!(
            tracker.observe_with_threshold(Some("S2"), t, z),
            Some("S2".to_owned()),
            "S2 fires once"
        );
    }

    #[test]
    fn dwell_fires_once_after_threshold_and_resets_on_move() {
        // RISK-3 / MC-3: with a zero threshold, the FIRST settle on a symbol fires once; staying on it
        // does NOT refire; moving to another symbol resets (no fire on the move frame), then settling
        // fires for the new symbol.
        let mut tracker = SymbolDwellTracker::new();
        let z = Duration::from_millis(0);
        let t0 = Instant::now();
        // Frame 1: cursor lands on S1 -> sets the dwell, does NOT fire (a move/first-observation frame).
        assert_eq!(tracker.observe_with_threshold(Some("S1"), t0, z), None);
        // Frame 2: still on S1, threshold crossed -> fires ONCE.
        assert_eq!(
            tracker.observe_with_threshold(Some("S1"), t0, z),
            Some("S1".to_owned())
        );
        // Frame 3: still on S1 -> already fired, no refire (no backend spam).
        assert_eq!(tracker.observe_with_threshold(Some("S1"), t0, z), None);
        // Frame 4: cursor MOVES to S2 -> reset, no fire on the move frame.
        assert_eq!(tracker.observe_with_threshold(Some("S2"), t0, z), None);
        // Frame 5: settles on S2 -> fires once for S2.
        assert_eq!(
            tracker.observe_with_threshold(Some("S2"), t0, z),
            Some("S2".to_owned())
        );
        assert_eq!(tracker.current_symbol(), Some("S2"));
    }

    #[test]
    fn dwell_does_not_fire_before_threshold() {
        // A real (800ms) threshold: a cursor that has only just landed does NOT fire (the timer has not
        // elapsed), proving the debounce gates the search.
        let mut tracker = SymbolDwellTracker::new();
        let now = Instant::now();
        assert_eq!(
            tracker.observe(Some("S1"), now),
            None,
            "first observation never fires"
        );
        // Same instant, real 800ms threshold -> still under the window -> no fire.
        assert_eq!(
            tracker.observe(Some("S1"), now),
            None,
            "under the 800ms window -> no fire"
        );
    }

    #[test]
    fn dwell_cursor_leaving_clears_in_flight_dwell() {
        let mut tracker = SymbolDwellTracker::new();
        let z = Duration::from_millis(0);
        let t = Instant::now();
        tracker.observe_with_threshold(Some("S1"), t, z);
        // Cursor leaves all symbols -> the in-flight dwell is dropped.
        assert_eq!(tracker.observe_with_threshold(None, t, z), None);
        assert_eq!(tracker.current_symbol(), None);
    }

    #[test]
    fn code_note_reference_formatter_matches_consumer_encoding_boundary() {
        assert_eq!(
            format_code_note_reference("src/lib.rs", "my_function").as_deref(),
            Some("[[code:src/lib.rs#my_function]]")
        );
        for target in [
            "src/lib.rs#",
            "#my_function",
            "src/li]b.rs#my_function",
            "src/lib.rs#bad]symbol",
            "src/lib.rs#two#symbols",
            "src/lib.rs# symbol ",
            "src/lib.rs#foo bar",
            "src/lib.rs#foo.bar",
        ] {
            assert!(
                !is_encodable_code_reference_target(target),
                "target must fail closed: {target:?}"
            );
        }
    }
}
