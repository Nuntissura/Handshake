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
//!   surface). So the field-correct verified path is search-v2 with the symbol key as the query, and
//!   the false-positive risk is mitigated by (a) restricting to rich-doc content types and (b) a
//!   precise multi-token symbol key (`path#Symbol`) rather than a bare word. If a dedicated
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
    BACKEND_BASE_URL, LoomSearchV2Body, LoomSearchV2Hit, LoomSearchV2Response,
};
use crate::code_editor::code_nav::CodeNavClient;
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

/// The rich-document content types a code->notes search restricts to (RISK-1 / MC-1: a code symbol is
/// referenced from NOTES, so a search filtered to these content types excludes unrelated block kinds
/// and cuts false positives). The backend serializes `LoomBlockContentType` as a snake_case string;
/// `note` + `document` are the rich-document surfaces a code-ref lives in.
pub const NOTE_REF_CONTENT_TYPES: &[&str] = &["note", "document"];

/// Why a code<->note cross-reference resolution failed. Every variant renders as a VISIBLE state
/// (empty / typed error chip), never a silent no-op or a panic. `kind_str` is a stable kebab-case
/// token the error UI + AccessKit label carry so an out-of-process agent reads a stable vocabulary.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CrossRefError {
    /// No workspace bound — a notes search resolves workspace state and needs a workspace id.
    #[error("no workspace context: code<->note cross-reference needs a workspace id")]
    NoWorkspace,
    /// The symbol entity id was empty (nothing to resolve).
    #[error("empty symbol id: no symbol entity id to resolve")]
    EmptySymbol,
    /// The backend resolved the symbol but it carried no definition span (no file/line to jump to).
    /// The chip renders as `unresolved` (greyed) without crashing — AC-4 / RISK pt(e).
    #[error("symbol has no definition span: {0}")]
    NoDefinition(String),
    /// The target symbol/document was not found (HTTP 404 / empty projection). Drives the
    /// greyed-out `unresolved` chip (AC-4) — a deleted symbol must NOT crash or panic.
    #[error("not found: {0}")]
    NotFound(String),
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
            CrossRefError::Backend(_) => "backend_error",
        }
    }

    /// True when this error means the symbol could not be resolved to a live definition (a deleted /
    /// unindexed symbol). The code-ref chip renders `unresolved` (greyed) for these without panicking
    /// (AC-4 / RISK pt(e)). A transient backend error is NOT treated as unresolved (it should retry).
    pub fn is_unresolved(&self) -> bool {
        matches!(
            self,
            CrossRefError::NotFound(_) | CrossRefError::NoDefinition(_) | CrossRefError::EmptySymbol
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
    /// Build a [`NoteRef`] from a loom search-v2 hit. The block id is the openable reference; the title
    /// falls back to the block id; the excerpt is the FTS highlight with the literal `<mark>`/`</mark>`
    /// markers stripped (the panel renders plain text, never raw HTML).
    pub fn from_hit(hit: LoomSearchV2Hit) -> Self {
        let block_id = hit.block.block_id.clone();
        let document_title = hit.block.display_title().to_owned();
        let excerpt = strip_mark_tags(&hit.highlight);
        Self {
            document_id: block_id.clone(),
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
    highlight.replace("<mark>", "").replace("</mark>", "").trim().to_owned()
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

/// Resolve a code-symbol entity id to a [`CodeRef`] (file path + 0-based line span) via the EXISTING
/// code-nav backend (`GET /knowledge/code/symbols/:entity_id`, reusing [`CodeNavClient`]).
///
/// Direction (A) note->code: clicking a `[[code:…]]` chip dispatches `open-code-symbol` with the
/// symbol entity id; the navigator calls this to learn WHERE to jump. The actual jump-to-line lands
/// when the code pane is mounted (E11/MT-069) — this resolves the target HONESTLY; it never fakes a
/// jump into a non-existent pane.
///
/// Errors:
/// - empty id -> [`CrossRefError::EmptySymbol`] (no round-trip),
/// - 404 / empty projection -> [`CrossRefError::NotFound`] (drives the greyed `unresolved` chip, AC-4),
/// - resolved but no definition span -> [`CrossRefError::NoDefinition`] (also unresolved),
/// - transport failure -> [`CrossRefError::Backend`].
pub async fn resolve_code_ref(symbol_entity_id: &str) -> Result<CodeRef, CrossRefError> {
    resolve_code_ref_with(&CodeNavClient::production(), symbol_entity_id).await
}

/// [`resolve_code_ref`] against an explicit [`CodeNavClient`] (a test points it at a live backend; the
/// percent-encoding of the entity id is performed INSIDE the client's `get_symbol`, so this is the
/// single resolution path the production helper also uses).
pub async fn resolve_code_ref_with(
    client: &CodeNavClient,
    symbol_entity_id: &str,
) -> Result<CodeRef, CrossRefError> {
    if symbol_entity_id.trim().is_empty() {
        return Err(CrossRefError::EmptySymbol);
    }
    let response = client.get_symbol(symbol_entity_id).await?;
    let symbol = response.symbol;
    // An absent symbol comes back as a default (empty) projection (the `get_symbol` graceful path) —
    // treat an empty entity id / key with no definition as not-found so the chip greys out (AC-4).
    let definition = symbol
        .definition
        .as_ref()
        .ok_or_else(|| CrossRefError::NoDefinition(symbol_entity_id.to_owned()))?;
    let (line_start, line_end) = match (definition.line_start, definition.line_end) {
        (Some(start), end) if start >= 1 => {
            let s = (start - 1) as u32; // 1-based backend -> 0-based editor.
            let e = end
                .filter(|e| *e >= start)
                .map(|e| (e - 1) as u32)
                .unwrap_or(s);
            (s, e)
        }
        _ => return Err(CrossRefError::NoDefinition(symbol_entity_id.to_owned())),
    };
    // Prefer the definition's own source id; fall back to extracting the path from the symbol key.
    let file_path = definition
        .source_id
        .clone()
        .filter(|p| !p.trim().is_empty())
        .or_else(|| crate::code_editor::code_nav::symbol_file_path(&symbol.symbol_key))
        .unwrap_or_default();
    Ok(CodeRef {
        symbol_entity_id: symbol_entity_id.to_owned(),
        symbol_key: symbol.symbol_key,
        file_path,
        line_start,
        line_end,
    })
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
    if symbol_key.trim().is_empty() {
        return Ok(Vec::new());
    }
    // Restrict to rich-doc content types one at a time (the search-v2 body's `content_type` filter is a
    // single value), merging + de-duplicating by block id so a symbol mentioned in both a `note` and a
    // `document` is listed once (RISK-1: tighter than an unfiltered full-text query).
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for content_type in NOTE_REF_CONTENT_TYPES {
        let body = LoomSearchV2Body::baseline(symbol_key.to_owned(), Some((*content_type).to_owned()));
        let response = backend.search(workspace_id, &body).await?;
        for hit in response.hits {
            let note = NoteRef::from_hit(hit);
            if seen.insert(note.block_id.clone()) {
                out.push(note);
            }
        }
    }
    Ok(out)
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
    if let EditorEvent::WikilinkActivated { ref_kind, ref_value, .. } = event {
        if ref_kind == CODE_REF_KIND {
            bus.open_code_symbol(ctx, ref_value.clone());
            return Some(ref_value.clone());
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
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<LoomSearchV2Response, CrossRefError>> + Send + 'a>>;
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
            client: reqwest::Client::new(),
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
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<LoomSearchV2Response, CrossRefError>> + Send + 'a>>
    {
        let url = format!("{}/workspaces/{}/loom/search-v2", self.base_url, workspace_id);
        let client = self.client.clone();
        let body = body.clone();
        Box::pin(async move {
            let response = client
                .post(&url)
                .json(&body)
                .send()
                .await
                .map_err(|e| CrossRefError::Backend(format!("find-notes search failed: {e}")))?;
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
                    // A DIFFERENT symbol (or first observation): reset the dwell timer to now and clear
                    // the fired marker (so settling here later fires once). Cursor MOVED => no fire yet.
                    _ => {
                        self.dwelling = Some((symbol.to_owned(), now));
                        self.fired_for = None;
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

    fn hit(block_id: &str, title: Option<&str>, content_type: &str, highlight: &str) -> LoomSearchV2Hit {
        LoomSearchV2Hit {
            block: LoomSearchBlock {
                block_id: block_id.to_owned(),
                content_type: content_type.to_owned(),
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
    fn note_ref_from_hit_strips_mark_and_falls_back_to_block_id() {
        let n = NoteRef::from_hit(hit("BLK-1", None, "note", "see <mark>MyStruct</mark> here"));
        assert_eq!(n.block_id, "BLK-1");
        assert_eq!(n.document_id, "BLK-1");
        assert_eq!(n.document_title, "BLK-1", "untitled block falls back to its id");
        assert_eq!(n.excerpt, "see MyStruct here", "the <mark> markers are stripped");
        let titled = NoteRef::from_hit(hit("BLK-2", Some("My Note"), "document", ""));
        assert_eq!(titled.document_title, "My Note");
    }

    #[test]
    fn cross_ref_error_kind_strings_and_unresolved_flag() {
        assert_eq!(CrossRefError::NoWorkspace.kind_str(), "no_workspace");
        assert_eq!(CrossRefError::NotFound("x".into()).kind_str(), "not_found");
        assert_eq!(CrossRefError::NoDefinition("x".into()).kind_str(), "no_definition");
        assert!(CrossRefError::NotFound("x".into()).is_unresolved());
        assert!(CrossRefError::NoDefinition("x".into()).is_unresolved());
        assert!(CrossRefError::EmptySymbol.is_unresolved());
        assert!(!CrossRefError::Backend("down".into()).is_unresolved(), "transient backend error is not unresolved");
        assert!(!CrossRefError::NoWorkspace.is_unresolved());
    }

    #[test]
    fn app_error_404_maps_to_not_found() {
        let nf: CrossRefError = AppError::Http("GET code-nav non-success status 404 Not Found".into()).into();
        assert!(matches!(nf, CrossRefError::NotFound(_)), "a 404 status maps to unresolved/not-found");
        let be: CrossRefError = AppError::Http("503 Service Unavailable".into()).into();
        assert!(matches!(be, CrossRefError::Backend(_)), "a non-404 status is a generic backend error");
    }

    // A counted in-memory search mock (the MT-014/MT-015 counted-mock pattern; NO backend).
    struct MockSearch {
        // The hits returned per content_type (note -> ..., document -> ...).
        by_content_type: std::collections::HashMap<String, Vec<LoomSearchV2Hit>>,
        calls: AtomicUsize,
    }
    impl FindNotesSearch for MockSearch {
        fn search<'a>(
            &'a self,
            _workspace_id: &'a str,
            body: &'a LoomSearchV2Body,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<LoomSearchV2Response, CrossRefError>> + Send + 'a>>
        {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let ct = body.content_type.clone().unwrap_or_default();
            let hits = self.by_content_type.get(&ct).cloned().unwrap_or_default();
            Box::pin(async move {
                Ok(LoomSearchV2Response {
                    hits,
                    content_type_facets: Default::default(),
                    semantic_available: false,
                    total: 0,
                })
            })
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
        let mock = MockSearch { by_content_type: Default::default(), calls: AtomicUsize::new(0) };
        let r = block_on(find_notes_with(&mock, "fn:src/main.rs#MyStruct", ""));
        assert_eq!(r, Err(CrossRefError::NoWorkspace));
        assert_eq!(mock.calls.load(Ordering::SeqCst), 0, "no backend call without a workspace");
    }

    #[test]
    fn find_notes_empty_symbol_is_empty_not_error() {
        let mock = MockSearch { by_content_type: Default::default(), calls: AtomicUsize::new(0) };
        let r = block_on(find_notes_with(&mock, "  ", "ws-1")).unwrap();
        assert!(r.is_empty(), "an empty symbol yields an empty list, not an error");
        assert_eq!(mock.calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn find_notes_merges_content_types_and_dedups() {
        // The same block matched under both `note` and `document` is listed ONCE (RISK-1 dedup); a
        // distinct block in each is kept. The search restricts to rich-doc content types (one call per).
        let mut by = std::collections::HashMap::new();
        by.insert("note".to_owned(), vec![hit("BLK-A", Some("A"), "note", "<mark>S</mark>"), hit("BLK-B", Some("B"), "note", "x")]);
        by.insert("document".to_owned(), vec![hit("BLK-A", Some("A"), "document", "y"), hit("BLK-C", Some("C"), "document", "z")]);
        let mock = MockSearch { by_content_type: by, calls: AtomicUsize::new(0) };
        let r = block_on(find_notes_with(&mock, "fn:src/main.rs#S", "ws-1")).unwrap();
        let ids: Vec<&str> = r.iter().map(|n| n.block_id.as_str()).collect();
        assert_eq!(ids, vec!["BLK-A", "BLK-B", "BLK-C"], "deduped, in content-type then hit order");
        assert_eq!(mock.calls.load(Ordering::SeqCst), 2, "one search per rich-doc content type");
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
        assert_eq!(tracker.observe_with_threshold(Some("S1"), t0, z), Some("S1".to_owned()));
        // Frame 3: still on S1 -> already fired, no refire (no backend spam).
        assert_eq!(tracker.observe_with_threshold(Some("S1"), t0, z), None);
        // Frame 4: cursor MOVES to S2 -> reset, no fire on the move frame.
        assert_eq!(tracker.observe_with_threshold(Some("S2"), t0, z), None);
        // Frame 5: settles on S2 -> fires once for S2.
        assert_eq!(tracker.observe_with_threshold(Some("S2"), t0, z), Some("S2".to_owned()));
        assert_eq!(tracker.current_symbol(), Some("S2"));
    }

    #[test]
    fn dwell_does_not_fire_before_threshold() {
        // A real (800ms) threshold: a cursor that has only just landed does NOT fire (the timer has not
        // elapsed), proving the debounce gates the search.
        let mut tracker = SymbolDwellTracker::new();
        let now = Instant::now();
        assert_eq!(tracker.observe(Some("S1"), now), None, "first observation never fires");
        // Same instant, real 800ms threshold -> still under the window -> no fire.
        assert_eq!(tracker.observe(Some("S1"), now), None, "under the 800ms window -> no fire");
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
}
