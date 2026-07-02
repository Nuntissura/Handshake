//! WP-KERNEL-012 MT-029 — native Find-in-Files + Replace-in-Files surface (E4 Search).
//!
//! The native Rust/egui port of the React `WorkspaceSearchPanel`
//! (`app/src/components/WorkspaceSearchPanel.tsx`, MT-250). This is a **workspace-wide** search across
//! all rich documents and Loom blocks with a two-phase replace-preview-then-apply flow. It is DISTINCT
//! from the single-document `FindReplacePanel` (MT-004, code-editor-local).
//!
//! ## Backend reality (Spec-Realism Gate — VERIFIED READ-ONLY, NOT contract-assumed)
//!
//! The MT-029 contract body names `GET /loom/search` and `PATCH /knowledge/rich-documents/{id}`, but
//! BOTH were verified against the real backend + the React reference and ALIGNED:
//!
//! - **Search** binds `GET /workspaces/{ws}/loom/graph-search` (handler `search_loom_graph` →
//!   `Vec<LoomGraphSearchResult>` carrying `{source_kind, result_kind, ref_id, title, excerpt, metadata,
//!   block?}`). This is the endpoint the React `searchLoomGraph()` actually calls
//!   (`app/src/lib/api.ts:1320-1341`) and the ONLY one returning the `source_kind`/`ref_id` shape this
//!   panel needs. The plain `/loom/search` (handler `search_loom_blocks`) returns `Vec<{block, score}>`
//!   with no `source_kind`/`ref_id`, so it CANNOT satisfy the documentId-from-hit logic — it is the
//!   wrong endpoint despite the contract naming it. Verified params (api/loom.rs `LoomSearchQueryParams`
//!   + api.test.ts:771): `q, source_kinds (comma-joined), tag_ids, mention_ids, case_sensitive,
//!   whole_word, regex (NOTE: `regex`, not `isRegex`), path, limit, offset`.
//! - **Rich-document load/save** binds `GET /knowledge/documents/{id}` + `PUT /knowledge/documents/
//!   {id}/save` `{expected_version, content_json}` (the MT-017/020 VERIFIED routes the React
//!   `loadRichDocument`/`saveRichDocument` use — `app/src/lib/api.ts:3199-3263`), NOT the contract's
//!   `/knowledge/rich-documents/{id}` PATCH. Save 200 → `{document:{doc_version,..}, save_receipt_event_id}`;
//!   save 409 → optimistic-concurrency conflict (NEVER a silent overwrite — RISK-2 data-loss control).
//! - **Bookmarks** bind `GET/PUT /workspaces/{ws}/search-bookmarks` with the body shape
//!   `{schema_id:"hsk.workspace_search_bookmark_state@1", bookmarks:[..]}` carried INSIDE the verified
//!   `bookmark_state` blob field (`api/workspaces.rs:806-869`). The backend stores `bookmark_state`
//!   opaquely; the schema_id lives in the blob (RISK-6: a wrong schema_id silently breaks the React
//!   reader, so the const is asserted in tests).
//!
//! ## Two-phase replace + data-loss safety (HBR-STOP — the replace MUTATES documents)
//!
//! Replace is preview-then-apply with three guards mirroring the React reference:
//! 1. STALE-RESULT guard: `result_set_key = hash(search params)`. Preview Replace refuses if the live
//!    `result_set_key` no longer matches the params the results were fetched under (RISK-2/MC-2).
//! 2. STALE-PLAN guard: `preview_plan_key = hash(result_set_key + replacement)`. Apply refuses if the
//!    preview is stale vs the current search+replacement (prevents applying a stale plan to a
//!    since-edited doc = silent data loss).
//! 3. OPTIMISTIC CONCURRENCY: each save carries the `expected_version` captured at preview; a 409 is
//!    surfaced and the OTHER receipts are preserved — never a silent overwrite (PARTIAL-FAILURE,
//!    RISK-1/MC-1).
//!
//! The content_json walk mutates ONLY `node.text` + `node.attrs.code` and round-trips every other node
//! VERBATIM (RISK-4: hsLink/embed/table nodes are preserved). Zero-length regex matches advance by 1
//! (RISK-3, no infinite loop); a non-regex query is `regex::escape`'d (RISK-8); only `KRD-`-prefixed
//! document ids are loaded (RISK-5).
//!
//! ## Async / HBR-QUIET + AccessKit (HBR-SWARM / HBR-VIS)
//!
//! Every HTTP call runs off the egui UI thread on the app's tokio runtime, delivering into
//! `Arc<Mutex<Option<..>>>` cells the panel drains each frame; the loading spinner animates ONLY while a
//! request is genuinely in flight (never a perpetual spinner). Every interactive widget carries a stable
//! kebab-case `author_id` under the `find-in-files.` namespace via
//! [`crate::accessibility::emit_interactive_node`].

use std::sync::{Arc, Mutex};

use regex::Regex;

use crate::accessibility;
use crate::backend_client::{
    BookmarkStateCell, FindReplaceCell, GraphSearchCell, LoomGraphSearchHit, RichDocClient,
    WorkspaceSearchClient,
};
use crate::pane_registry::{PaneFactory, PaneRenderContext, PaneType};
use crate::theme::HsPalette;

// ── Stable AccessKit author_ids (the MT-029 naming contract) ─────────────────────────────────────────

/// The query `TextEdit`.
pub const QUERY_AUTHOR_ID: &str = "find-in-files.query";
/// The replacement `TextEdit`.
pub const REPLACE_AUTHOR_ID: &str = "find-in-files.replace";
/// The case-sensitive toggle (`Aa`).
pub const TOGGLE_CASE_AUTHOR_ID: &str = "find-in-files.toggle-case";
/// The whole-word toggle (`W`).
pub const TOGGLE_WORD_AUTHOR_ID: &str = "find-in-files.toggle-word";
/// The regex toggle (`.*`).
pub const TOGGLE_REGEX_AUTHOR_ID: &str = "find-in-files.toggle-regex";
/// The kind-filter `ComboBox`.
pub const KIND_FILTER_AUTHOR_ID: &str = "find-in-files.kind-filter";
/// The tag-filter `TextEdit`.
pub const TAG_FILTER_AUTHOR_ID: &str = "find-in-files.tag-filter";
/// The path-filter `TextEdit`.
pub const PATH_FILTER_AUTHOR_ID: &str = "find-in-files.path-filter";
/// The `Search` button.
pub const SEARCH_AUTHOR_ID: &str = "find-in-files.search";
/// The `Preview Replace` button.
pub const PREVIEW_REPLACE_AUTHOR_ID: &str = "find-in-files.preview-replace";
/// The `Apply` button.
pub const APPLY_AUTHOR_ID: &str = "find-in-files.apply";
/// The `Cancel`/clear button.
pub const CANCEL_AUTHOR_ID: &str = "find-in-files.cancel";
/// The `Bookmark Search` button.
pub const SAVE_BOOKMARK_AUTHOR_ID: &str = "find-in-files.save-bookmark";
/// Prefix for a per-result row (`find-in-files.result.{source_kind}.{stable_ref_id}`).
pub const RESULT_AUTHOR_ID_PREFIX: &str = "find-in-files.result.";
/// Prefix for a per-preview item (`find-in-files.preview.{document_id}`).
pub const PREVIEW_AUTHOR_ID_PREFIX: &str = "find-in-files.preview.";

/// 24-char context window each side of a match preview (the React `MATCH_PREVIEW_CONTEXT_CHARS`).
pub const MATCH_PREVIEW_CONTEXT_CHARS: usize = 24;

/// Max bookmarks persisted (the React `MAX_WORKSPACE_SEARCH_BOOKMARKS`).
pub const MAX_WORKSPACE_SEARCH_BOOKMARKS: usize = 20;

/// The backend-validated bookmark blob schema id (RISK-6: a wrong value silently fails the React
/// reader). Asserted in the bookmark-blob test.
pub const WORKSPACE_SEARCH_BOOKMARK_SCHEMA_ID: &str = "hsk.workspace_search_bookmark_state@1";

/// The result-row author_id for a hit: `find-in-files.result.{source_kind}.{stable_ref_id}` with every
/// non-alphanumeric char in the ref id replaced by `-` (the contract's stable-ref rule). Mirrors the
/// React stable-key convention so the AccessKit tree + screenshot are reproducible.
pub fn result_author_id(source_kind: &str, ref_id: &str) -> String {
    let stable: String = ref_id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    format!("{RESULT_AUTHOR_ID_PREFIX}{source_kind}.{stable}")
}

/// The preview-item author_id for a planned document: `find-in-files.preview.{document_id}`.
pub fn preview_author_id(document_id: &str) -> String {
    format!("{PREVIEW_AUTHOR_ID_PREFIX}{document_id}")
}

// ── Kind filter ──────────────────────────────────────────────────────────────────────────────────────

/// One selectable source-kind filter. `All` omits `source_kinds` entirely; every other variant passes
/// exactly one `source_kind` to the backend. The labels + wire values mirror the React `SEARCHABLE_KINDS`
/// table (`WorkspaceSearchPanel.tsx:17-28`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KindFilter {
    All,
    Document,
    LoomBlock,
    File,
    TagHub,
    Symbol,
    WorkPacket,
    MicroTask,
    UserManualPage,
    WikiPage,
}

impl KindFilter {
    /// Every variant in display order (the order the ComboBox lists them).
    pub const ALL: [KindFilter; 10] = [
        KindFilter::All,
        KindFilter::Document,
        KindFilter::LoomBlock,
        KindFilter::File,
        KindFilter::TagHub,
        KindFilter::Symbol,
        KindFilter::WorkPacket,
        KindFilter::MicroTask,
        KindFilter::UserManualPage,
        KindFilter::WikiPage,
    ];

    /// The human/model-readable label (React parity).
    pub fn label(self) -> &'static str {
        match self {
            KindFilter::All => "All kinds",
            KindFilter::Document => "Documents",
            KindFilter::LoomBlock => "Loom blocks",
            KindFilter::File => "Files",
            KindFilter::TagHub => "Tags",
            KindFilter::Symbol => "Symbols",
            KindFilter::WorkPacket => "Work packets",
            KindFilter::MicroTask => "Microtasks",
            KindFilter::UserManualPage => "UserManual",
            KindFilter::WikiPage => "Wiki pages",
        }
    }

    /// The backend `source_kind` wire value for the single-kind filter, or `None` for `All` (which omits
    /// the `source_kinds` param entirely — AC-4). The values match the backend `LoomSearchSourceKind`
    /// snake_case enum.
    pub fn source_kind(self) -> Option<&'static str> {
        match self {
            KindFilter::All => None,
            KindFilter::Document => Some("document"),
            KindFilter::LoomBlock => Some("loom_block"),
            KindFilter::File => Some("file"),
            KindFilter::TagHub => Some("tag_hub"),
            KindFilter::Symbol => Some("symbol"),
            KindFilter::WorkPacket => Some("work_packet"),
            KindFilter::MicroTask => Some("micro_task"),
            KindFilter::UserManualPage => Some("user_manual_page"),
            KindFilter::WikiPage => Some("wiki_page"),
        }
    }

    /// The stable wire token used in a bookmark blob's `kind` field (round-trips through restore).
    pub fn wire(self) -> &'static str {
        self.source_kind().unwrap_or("all")
    }

    /// Parse a bookmark blob `kind` token back to a filter (`all` → `All`; unknown → `All`).
    pub fn from_wire(s: &str) -> KindFilter {
        KindFilter::ALL
            .into_iter()
            .find(|k| k.wire() == s)
            .unwrap_or(KindFilter::All)
    }
}

// ── Match options ─────────────────────────────────────────────────────────────────────────────────────

/// The three match toggles (case / whole-word / regex). Copied into every pure replace fn so the logic
/// is unit-testable standalone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MatchOptions {
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub is_regex: bool,
}

impl MatchOptions {
    /// Project into the transport-layer [`crate::backend_client::SearchMatchOptions`] the search client
    /// forwards as query params (kept as a separate type so backend_client does not depend on this
    /// module's `MatchOptions`).
    pub fn to_search(self) -> crate::backend_client::SearchMatchOptions {
        crate::backend_client::SearchMatchOptions {
            case_sensitive: self.case_sensitive,
            whole_word: self.whole_word,
            is_regex: self.is_regex,
        }
    }
}

// ── Regex compilation (PT-4, RISK-8) ──────────────────────────────────────────────────────────────────

/// Compile the search query into a `regex::Regex`. For non-regex mode the query is `regex::escape`'d
/// first so `.`/`*`/etc. are LITERAL (RISK-8). Case-insensitivity is the `(?i)` flag when not
/// case-sensitive. An empty query or an invalid pattern is an `Err(String)` (PT-4) — never a panic. The
/// Rust `regex` crate is linear-time (no catastrophic backtracking), so no thread-timeout is needed
/// (the MT-018 lesson).
pub fn compile_search_regex(query: &str, opts: MatchOptions) -> Result<Regex, String> {
    if query.trim().is_empty() {
        return Err("Search query is required".to_owned());
    }
    let base = if opts.is_regex {
        query.to_owned()
    } else {
        regex::escape(query)
    };
    let pattern = if opts.case_sensitive {
        base
    } else {
        format!("(?i){base}")
    };
    Regex::new(&pattern).map_err(|e| format!("Invalid regular expression: {e}"))
}

// ── Whole-word boundary (mirrors the React isWordBoundary) ────────────────────────────────────────────

/// A Unicode word char (letter, number, or underscore) — the React `WORD_CHAR` = `[\p{L}\p{N}_]`.
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Whether a match `[start, end)` (BYTE indices into `text`) sits on a whole-word boundary, mirroring
/// the React `isWordBoundary` (`WorkspaceSearchPanel.tsx:222-230`): if the match STARTS on a word char
/// and the char immediately before is also a word char, the boundary fails; symmetrically for the end.
fn is_word_boundary(text: &str, start: usize, end: usize) -> bool {
    let before = text[..start].chars().next_back();
    let after = text[end..].chars().next();
    let starts_on_word = text[start..].chars().next().is_some_and(is_word_char);
    let ends_on_word = text[..end].chars().next_back().is_some_and(is_word_char);
    if starts_on_word && before.is_some_and(is_word_char) {
        return false;
    }
    if ends_on_word && after.is_some_and(is_word_char) {
        return false;
    }
    true
}

// ── Replacement-group expansion (mirrors the React expandReplacement) ─────────────────────────────────

/// Expand `$1..$9`, `$&` (whole match), and `$$` (literal `$`) in a regex-mode replacement template,
/// mirroring the React `expandReplacement` (`WorkspaceSearchPanel.tsx:232-239`). `groups[i]` is the
/// i-th capture group's text (empty string for an unmatched optional group). For NON-regex mode the
/// caller passes the literal replacement and never calls this.
fn expand_replacement(template: &str, match_text: &str, groups: &[String]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut chars = template.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '$' {
            out.push(c);
            continue;
        }
        match chars.peek().copied() {
            Some('$') => {
                chars.next();
                out.push('$');
            }
            Some('&') => {
                chars.next();
                out.push_str(match_text);
            }
            Some(d @ '1'..='9') => {
                chars.next();
                let idx = (d as usize) - ('1' as usize);
                if let Some(g) = groups.get(idx) {
                    out.push_str(g);
                }
            }
            // A `$` not followed by a recognized token is emitted literally (React's regex only
            // matches `$($|&|[1-9])`, leaving any other `$x` untouched).
            _ => out.push('$'),
        }
    }
    out
}

// ── Per-match preview (24-char context, mirrors the React replacementMatchPreview) ────────────────────

/// One match's before/after preview snippet (24-char context each side).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchPreview {
    pub before_preview: String,
    pub after_preview: String,
}

/// Build a 24-char-context before/after preview for a single match `[start, end)` (BYTE indices) being
/// replaced by `inserted`, mirroring the React `replacementMatchPreview`. Slicing is done on CHAR
/// boundaries so a multibyte char near the window edge never panics.
fn match_preview(text: &str, start: usize, end: usize, inserted: &str) -> MatchPreview {
    let preview_start =
        floor_char_boundary(text, start.saturating_sub(MATCH_PREVIEW_CONTEXT_CHARS));
    let preview_end = ceil_char_boundary(text, (end + MATCH_PREVIEW_CONTEXT_CHARS).min(text.len()));
    let before_preview = text[preview_start..preview_end].to_owned();
    let after_preview = format!(
        "{}{}{}",
        &text[preview_start..start],
        inserted,
        &text[end..preview_end]
    );
    MatchPreview {
        before_preview,
        after_preview,
    }
}

/// Round `idx` DOWN to the nearest char boundary in `s` (std's `floor_char_boundary` is unstable, so
/// this is the stable equivalent the preview slicing needs to stay panic-free on multibyte text).
fn floor_char_boundary(s: &str, idx: usize) -> usize {
    if idx >= s.len() {
        return s.len();
    }
    let mut i = idx;
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// Round `idx` UP to the nearest char boundary in `s`.
fn ceil_char_boundary(s: &str, idx: usize) -> usize {
    if idx >= s.len() {
        return s.len();
    }
    let mut i = idx;
    while i < s.len() && !s.is_char_boundary(i) {
        i += 1;
    }
    i
}

// ── Segment replace (mirrors the React replaceSegment, RISK-3 zero-length guard) ──────────────────────

/// The result of replacing in ONE text segment: the new text, the match count, and per-match previews.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentReplaceResult {
    pub text: String,
    pub count: usize,
    pub match_previews: Vec<MatchPreview>,
}

/// Replace every (whole-word-respecting) match of `regex` in `text` with `replacement`, mirroring the
/// React `replaceSegment` (`WorkspaceSearchPanel.tsx:249-288`):
/// - A ZERO-LENGTH match advances the scan cursor by one CHAR and is never replaced (RISK-3 — no
///   infinite loop).
/// - When `whole_word`, a match that fails [`is_word_boundary`] is SKIPPED (left as-is).
/// - In regex mode the replacement is `$1..$9`/`$&`/`$$`-expanded per match; in literal mode the
///   replacement is inserted verbatim.
///
/// Returns the rebuilt text + the count + a 24-char preview per replaced match.
pub fn replace_segment(
    text: &str,
    regex: &Regex,
    replacement: &str,
    opts: MatchOptions,
) -> SegmentReplaceResult {
    let mut next = String::with_capacity(text.len());
    let mut last_index = 0usize; // byte index of the end of the last copied region
    let mut count = 0usize;
    let mut match_previews = Vec::new();
    let mut search_from = 0usize; // byte index the next find starts at

    loop {
        // Terminate once the scan cursor passes the end of the text (a find AT len can still match a
        // zero-length pattern, so the guard is `> len`, and the zero-length branch below advances past
        // len to break — RISK-3 no-infinite-loop).
        if search_from > text.len() {
            break;
        }
        let Some(m) = regex.find_at(text, search_from) else {
            break;
        };
        let start = m.start();
        let end = m.end();
        if start == end {
            // Zero-length match: advance by one char and never replace (RISK-3). When the match is at
            // the very end of the text, advancing past `len` makes the next loop iteration break — so a
            // zero-length-capable pattern like `a*` always terminates.
            search_from = if start >= text.len() {
                text.len() + 1
            } else {
                ceil_char_boundary(text, start + 1)
            };
            continue;
        }
        if opts.whole_word && !is_word_boundary(text, start, end) {
            // Not a whole-word boundary: leave this match as-is, continue scanning after it.
            search_from = end;
            continue;
        }
        let inserted = if opts.is_regex {
            let caps = regex.captures_at(text, start);
            let groups: Vec<String> = match caps {
                Some(caps) => (1..caps.len())
                    .map(|i| {
                        caps.get(i)
                            .map(|g| g.as_str().to_owned())
                            .unwrap_or_default()
                    })
                    .collect(),
                None => Vec::new(),
            };
            expand_replacement(replacement, &text[start..end], &groups)
        } else {
            replacement.to_owned()
        };
        next.push_str(&text[last_index..start]);
        next.push_str(&inserted);
        last_index = end;
        count += 1;
        match_previews.push(match_preview(text, start, end, &inserted));
        search_from = end;
    }

    if count == 0 {
        return SegmentReplaceResult {
            text: text.to_owned(),
            count: 0,
            match_previews: Vec::new(),
        };
    }
    next.push_str(&text[last_index..]);
    SegmentReplaceResult {
        text: next,
        count,
        match_previews,
    }
}

// ── content_json walk (mirrors the React replaceInContent, RISK-4) ────────────────────────────────────

/// The result of walking a whole document's content_json: the replaced tree, the total match count,
/// the first match's before/after snapshot (whole-text), and the flattened per-match previews.
#[derive(Debug, Clone, PartialEq)]
pub struct ContentReplaceResult {
    pub content: serde_json::Value,
    pub count: usize,
    pub before_preview: String,
    pub after_preview: String,
    pub match_previews: Vec<MatchPreview>,
}

/// Recursively replace in a ProseMirror-style content_json tree (`serde_json::Value`), mirroring the
/// React `replaceInContent` (`WorkspaceSearchPanel.tsx:290-342`). ONLY two string fields are mutated —
/// `node.text` and `node.attrs.code` (code-block content) — and EVERY other node/field is round-tripped
/// VERBATIM (RISK-4: hsLink/embed/table/transclusion nodes are preserved). Recurses into `node.content`.
/// `before_preview`/`after_preview` capture the FIRST mutated text segment's whole-text before/after (the
/// React `??=` first-set semantics).
pub fn replace_in_content(
    content: &serde_json::Value,
    regex: &Regex,
    replacement: &str,
    opts: MatchOptions,
) -> ContentReplaceResult {
    let mut count = 0usize;
    let mut before_preview: Option<String> = None;
    let mut after_preview: Option<String> = None;
    let mut match_previews: Vec<MatchPreview> = Vec::new();
    let new_content = visit_node(
        content,
        regex,
        replacement,
        opts,
        &mut count,
        &mut before_preview,
        &mut after_preview,
        &mut match_previews,
    );
    ContentReplaceResult {
        content: new_content,
        count,
        before_preview: before_preview.unwrap_or_default(),
        after_preview: after_preview.unwrap_or_default(),
        match_previews,
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_node(
    node: &serde_json::Value,
    regex: &Regex,
    replacement: &str,
    opts: MatchOptions,
    count: &mut usize,
    before_preview: &mut Option<String>,
    after_preview: &mut Option<String>,
    match_previews: &mut Vec<MatchPreview>,
) -> serde_json::Value {
    // Non-object nodes (string/number/array elements handled by their parent) round-trip verbatim.
    let serde_json::Value::Object(map) = node else {
        return node.clone();
    };
    let mut next = map.clone();

    // 1) text node: mutate `node.text`.
    if let Some(serde_json::Value::String(text)) = map.get("text") {
        let replaced = replace_segment(text, regex, replacement, opts);
        if replaced.count > 0 {
            if before_preview.is_none() {
                *before_preview = Some(text.clone());
                *after_preview = Some(replaced.text.clone());
            }
            *count += replaced.count;
            match_previews.extend(replaced.match_previews);
            next.insert("text".to_owned(), serde_json::Value::String(replaced.text));
        }
    }

    // 2) code-block: mutate `node.attrs.code` (RISK-4 — code-block content must be searched too).
    if let Some(serde_json::Value::Object(attrs)) = map.get("attrs") {
        if let Some(serde_json::Value::String(code)) = attrs.get("code") {
            let replaced = replace_segment(code, regex, replacement, opts);
            if replaced.count > 0 {
                if before_preview.is_none() {
                    *before_preview = Some(code.clone());
                    *after_preview = Some(replaced.text.clone());
                }
                *count += replaced.count;
                match_previews.extend(replaced.match_previews);
                let mut new_attrs = attrs.clone();
                new_attrs.insert("code".to_owned(), serde_json::Value::String(replaced.text));
                next.insert("attrs".to_owned(), serde_json::Value::Object(new_attrs));
            }
        }
    }

    // 3) recurse into `node.content` (every child round-trips, mutated children replaced).
    if let Some(serde_json::Value::Array(children)) = map.get("content") {
        let new_children: Vec<serde_json::Value> = children
            .iter()
            .map(|child| {
                visit_node(
                    child,
                    regex,
                    replacement,
                    opts,
                    count,
                    before_preview,
                    after_preview,
                    match_previews,
                )
            })
            .collect();
        next.insert("content".to_owned(), serde_json::Value::Array(new_children));
    }

    serde_json::Value::Object(next)
}

// ── documentId-from-hit (mirrors the React documentIdFromLoomSearchHit, RISK-5) ───────────────────────

/// Extract the editable rich-document id for a search hit, mirroring the React
/// `documentIdFromLoomSearchHit` (`loom_search_open_target.ts:15-22`): try `metadata.rich_document_id`,
/// then `metadata.document_id`, then `block.document_id`, then (when `source_kind == "document"`) the
/// `ref_id`; accept the candidate ONLY if it starts with `KRD-` (RISK-5 — a non-rich-document id would
/// 404 the load). Returns `None` when no `KRD-` id is found.
pub fn document_id_from_hit(hit: &LoomGraphSearchHit) -> Option<String> {
    let metadata_str = |key: &str| -> Option<String> {
        hit.metadata
            .get(key)
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
    };
    let block_document_id = || -> Option<String> {
        hit.block
            .as_ref()
            .and_then(|b| b.get("document_id"))
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
    };
    let candidate = metadata_str("rich_document_id")
        .or_else(|| metadata_str("document_id"))
        .or_else(block_document_id)
        .or_else(|| {
            if hit.source_kind == "document" {
                let r = hit.ref_id.trim();
                (!r.is_empty()).then(|| r.to_owned())
            } else {
                None
            }
        });
    candidate.filter(|c| c.starts_with("KRD-"))
}

// ── Client-side option filter (mirrors the React hitMatchesClientOptions) ─────────────────────────────

/// Whether a hit passes the client-side option filter, mirroring the React `hitMatchesClientOptions`
/// (`WorkspaceSearchPanel.tsx:344-361`): when NONE of case/word/regex is set, every hit passes; otherwise
/// the compiled regex must match the `title\nexcerpt` haystack respecting the whole-word boundary. A
/// query that fails to compile passes everything (the backend already filtered).
pub fn hit_matches_client_options(
    hit: &LoomGraphSearchHit,
    query: &str,
    opts: MatchOptions,
) -> bool {
    if query.trim().is_empty() {
        return true;
    }
    if !opts.case_sensitive && !opts.whole_word && !opts.is_regex {
        return true;
    }
    let Ok(regex) = compile_search_regex(query, opts) else {
        return true;
    };
    hit_matches_regex(hit, &regex, opts)
}

/// Whether a hit's `title\nexcerpt` haystack matches an ALREADY-COMPILED `regex`, respecting the
/// whole-word boundary. Split out from [`hit_matches_client_options`] so the render path can compile the
/// regex ONCE per (query, options) change and reuse it across all hits (perf hygiene) instead of
/// recompiling per hit per frame.
pub fn hit_matches_regex(hit: &LoomGraphSearchHit, regex: &Regex, opts: MatchOptions) -> bool {
    let haystack = format!("{}\n{}", hit.title, hit.excerpt);
    let mut search_from = 0usize;
    loop {
        if search_from > haystack.len() {
            return false;
        }
        let Some(m) = regex.find_at(&haystack, search_from) else {
            return false;
        };
        let (start, end) = (m.start(), m.end());
        if start == end {
            // Zero-length match: advance past it (and past `len` at the end) so the scan terminates.
            search_from = if start >= haystack.len() {
                haystack.len() + 1
            } else {
                ceil_char_boundary(&haystack, start + 1)
            };
            continue;
        }
        if !opts.whole_word || is_word_boundary(&haystack, start, end) {
            return true;
        }
        search_from = end;
    }
}

// ── State keys (RISK-2/MC-2 stale guards) ─────────────────────────────────────────────────────────────

/// A deterministic string key for the current SEARCH params (query + kind + filters + options). Two
/// searches with identical params yield the same key; any change yields a different key. Used as
/// `result_set_key` so Preview Replace can detect a since-changed query (RISK-2). Built from a normalized
/// tuple serialized to JSON (stable field order).
pub fn search_plan_key(
    query: &str,
    kind: KindFilter,
    tag_filter: &str,
    path_filter: &str,
    opts: MatchOptions,
) -> String {
    let normalized = serde_json::json!({
        "query": query.trim(),
        "kind": kind.wire(),
        "tag": tag_filter.trim(),
        "path": path_filter.trim(),
        "case": opts.case_sensitive,
        "word": opts.whole_word,
        "regex": opts.is_regex,
    });
    normalized.to_string()
}

/// A deterministic key for a REPLACE plan: the search key + the replacement text. Used as
/// `preview_plan_key` so Apply can detect a since-changed search-or-replacement (RISK-2/MC-2).
pub fn replace_plan_key(search_key: &str, replacement: &str) -> String {
    serde_json::json!({ "search": search_key, "replacement": replacement }).to_string()
}

// ── Replacement plan ──────────────────────────────────────────────────────────────────────────────────

/// One document's planned replacement (the preview unit). Carries the `expected_version` captured at
/// preview so Apply's save uses optimistic concurrency (a 409 = the doc changed since preview → NO
/// overwrite, RISK-2 data-loss control).
#[derive(Debug, Clone, PartialEq)]
pub struct ReplacementPlan {
    pub document_id: String,
    pub title: String,
    pub expected_version: u64,
    pub content_json_after: serde_json::Value,
    pub crdt_document_id: Option<String>,
    pub match_count: usize,
    pub before_preview: String,
    pub after_preview: String,
    pub match_previews: Vec<MatchPreview>,
}

// ── Bookmark ──────────────────────────────────────────────────────────────────────────────────────────

/// One saved search bookmark (the React `WorkspaceSearchBookmark`). Round-trips through the
/// `bookmark_state` blob.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchBookmark {
    pub id: String,
    pub label: String,
    pub query: String,
    pub kind: KindFilter,
    pub tag_filter: String,
    pub path_filter: String,
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub is_regex: bool,
    pub saved_at: String,
}

impl SearchBookmark {
    /// A stable id derived from the search params (the React `bookmarkIdForSearch`): re-saving the same
    /// search replaces (dedups) the prior bookmark.
    pub fn stable_id(&self) -> String {
        let parts = [
            if self.query.trim().is_empty() {
                "empty"
            } else {
                self.query.trim()
            },
            self.kind.wire(),
            self.tag_filter.trim(),
            self.path_filter.trim(),
            if self.case_sensitive { "case" } else { "" },
            if self.whole_word { "word" } else { "" },
            if self.is_regex { "regex" } else { "" },
        ];
        let joined = parts
            .iter()
            .filter(|s| !s.is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join(" ");
        let stable: String = joined
            .to_lowercase()
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                    c
                } else {
                    '-'
                }
            })
            .collect();
        let trimmed = stable.trim_matches('-');
        if trimmed.is_empty() {
            "item".to_owned()
        } else {
            trimmed.to_owned()
        }
    }

    /// The display label (the React `bookmarkLabelForSearch`): the query if any, else the kind/filters.
    pub fn display_label(&self) -> String {
        let q = self.query.trim();
        if !q.is_empty() {
            return q.to_owned();
        }
        let kind_label = if self.kind == KindFilter::All {
            ""
        } else {
            self.kind.label()
        };
        let parts: Vec<&str> = [kind_label, self.tag_filter.trim(), self.path_filter.trim()]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect();
        if parts.is_empty() {
            "Filtered search".to_owned()
        } else {
            parts.join(" / ")
        }
    }

    /// Serialize to the per-bookmark JSON shape the `bookmark_state.bookmarks[]` blob carries (React
    /// `WorkspaceSearchBookmark` field names — camelCase, since the React reader keys on them).
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "label": self.label,
            "query": self.query,
            "kind": self.kind.wire(),
            "tagFilter": self.tag_filter,
            "pathFilter": self.path_filter,
            "caseSensitive": self.case_sensitive,
            "wholeWord": self.whole_word,
            "isRegex": self.is_regex,
            "savedAt": self.saved_at,
        })
    }

    /// Parse one bookmark entry from the blob; returns `None` if any required field is missing/mistyped
    /// (the React `isWorkspaceSearchBookmark` guard).
    pub fn from_json(v: &serde_json::Value) -> Option<SearchBookmark> {
        let s = |k: &str| v.get(k).and_then(|x| x.as_str()).map(str::to_owned);
        let b = |k: &str| v.get(k).and_then(|x| x.as_bool());
        let id = s("id")?;
        let label = s("label")?;
        if id.trim().is_empty() || label.trim().is_empty() {
            return None;
        }
        Some(SearchBookmark {
            id,
            label,
            query: s("query")?,
            kind: KindFilter::from_wire(&s("kind")?),
            tag_filter: s("tagFilter")?,
            path_filter: s("pathFilter")?,
            case_sensitive: b("caseSensitive")?,
            whole_word: b("wholeWord")?,
            is_regex: b("isRegex")?,
            saved_at: s("savedAt")?,
        })
    }
}

/// Build the `bookmark_state` blob from a bookmark list (the React `workspaceSearchBookmarkBlob`): the
/// REQUIRED `schema_id` (RISK-6) + the `bookmarks` array, capped at [`MAX_WORKSPACE_SEARCH_BOOKMARKS`].
pub fn bookmark_state_blob(bookmarks: &[SearchBookmark]) -> serde_json::Value {
    let capped: Vec<serde_json::Value> = bookmarks
        .iter()
        .take(MAX_WORKSPACE_SEARCH_BOOKMARKS)
        .map(SearchBookmark::to_json)
        .collect();
    serde_json::json!({
        "schema_id": WORKSPACE_SEARCH_BOOKMARK_SCHEMA_ID,
        "bookmarks": capped,
    })
}

/// Parse the `bookmark_state` blob into a bookmark list (the React `parseWorkspaceSearchBookmarks`):
/// reads the `bookmarks` array, filters out malformed entries, caps at the max. A null/absent/mis-shaped
/// blob yields an empty list (never an error).
pub fn parse_bookmark_state(blob: &serde_json::Value) -> Vec<SearchBookmark> {
    blob.get("bookmarks")
        .and_then(|b| b.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(SearchBookmark::from_json)
                .take(MAX_WORKSPACE_SEARCH_BOOKMARKS)
                .collect()
        })
        .unwrap_or_default()
}

// ── Panel state machine ───────────────────────────────────────────────────────────────────────────────

/// The memoized client-side visible-hit filter (perf hygiene): the compiled regex + the filtered hit
/// index list, valid only while `key` matches the live (query + options + results-generation) key. This
/// hoists the per-hit `compile_search_regex` AND the per-frame filter+clone out of the render hot path —
/// without it, `show()` recompiled one [`Regex`] PER HIT and rebuilt the visible Vec EVERY frame
/// (typing/hover/scroll all repaint) for a paginated result set of hundreds-to-thousands of hits.
struct VisibleCache {
    /// `query + options + results_generation` digest; a mismatch invalidates the cache.
    key: String,
    /// The 0-based indices into `results` that pass the client-side option filter, in result order.
    indices: Vec<usize>,
}

/// All Find-in-Files panel state (the React component's `useState` hooks as one struct), plus the
/// off-thread delivery cells. Mirrors the MT-029 AC-1 required field set.
pub struct FindInFilesPanelState {
    pub query: String,
    pub replacement: String,
    pub kind: KindFilter,
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub is_regex: bool,
    pub tag_filter: String,
    pub path_filter: String,
    /// The collected, paginated hits from the last search.
    pub results: Vec<LoomGraphSearchHit>,
    /// `true` while a search / preview / apply request is genuinely in flight (drives the loading
    /// indicator ONLY while pending — never a perpetual spinner).
    pub loading: bool,
    /// The last error string, or `None`.
    pub error: Option<String>,
    /// The last replace/preview/apply status string, or `None`.
    pub replace_status: Option<String>,
    /// The current preview plans (empty until Preview Replace runs).
    pub preview_plans: Vec<ReplacementPlan>,
    /// The replace-plan key the current `preview_plans` were computed under (stale-apply guard).
    pub preview_plan_key: Option<String>,
    /// The search-plan key the current `results` were fetched under (stale-preview guard).
    pub result_set_key: Option<String>,
    /// The saved-search bookmarks.
    pub bookmarks: Vec<SearchBookmark>,
    /// The last bookmark op status string, or `None`.
    pub bookmark_status: Option<String>,

    /// Bumps every time `results` is replaced (in [`poll`](Self::poll)); part of the visible-cache key so
    /// a new result set invalidates the memoized filter even when query+options are unchanged.
    results_generation: u64,
    /// Memoized client-side visible-hit filter (perf hygiene — see [`VisibleCache`]). Interior mutability
    /// lets the `&self` render/status path refresh it lazily without taking `&mut self`.
    visible_cache: std::cell::RefCell<Option<VisibleCache>>,

    // ── Off-thread delivery cells ──
    search_cell: GraphSearchCell,
    /// The replace pipeline (preview document loads + apply saves) runs on a background task and
    /// delivers a typed [`ReplaceDelivery`] into this cell.
    replace_cell: FindReplaceCell,
    bookmark_cell: BookmarkStateCell,
}

impl Default for FindInFilesPanelState {
    fn default() -> Self {
        Self::new()
    }
}

impl FindInFilesPanelState {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            replacement: String::new(),
            kind: KindFilter::All,
            case_sensitive: false,
            whole_word: false,
            is_regex: false,
            tag_filter: String::new(),
            path_filter: String::new(),
            results: Vec::new(),
            loading: false,
            error: None,
            replace_status: None,
            preview_plans: Vec::new(),
            preview_plan_key: None,
            result_set_key: None,
            bookmarks: Vec::new(),
            bookmark_status: None,
            results_generation: 0,
            visible_cache: std::cell::RefCell::new(None),
            search_cell: Arc::new(Mutex::new(None)),
            replace_cell: Arc::new(Mutex::new(None)),
            bookmark_cell: Arc::new(Mutex::new(None)),
        }
    }

    /// The current match options as a [`MatchOptions`].
    pub fn options(&self) -> MatchOptions {
        MatchOptions {
            case_sensitive: self.case_sensitive,
            whole_word: self.whole_word,
            is_regex: self.is_regex,
        }
    }

    /// The current search-plan key (for the stale-result guard).
    pub fn current_search_key(&self) -> String {
        search_plan_key(
            &self.query,
            self.kind,
            &self.tag_filter,
            &self.path_filter,
            self.options(),
        )
    }

    /// The current replace-plan key (for the stale-preview guard).
    pub fn current_replace_key(&self) -> String {
        replace_plan_key(&self.current_search_key(), &self.replacement)
    }

    /// The current visible-cache key: live query + options + results generation. A change to ANY of these
    /// (re-search, query edit, toggle flip) invalidates the memoized visible-hit filter.
    fn visible_cache_key(&self) -> String {
        let opts = self.options();
        format!(
            "{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}",
            self.results_generation,
            self.query,
            opts.case_sensitive as u8,
            opts.whole_word as u8,
            opts.is_regex as u8,
        )
    }

    /// Recompute (and cache) the indices of `results` that pass the client-side option filter, but ONLY
    /// when the visible-cache key changed. The compiled [`Regex`] is built ONCE here per (query, options)
    /// change and reused across all hits — never per hit, never per frame (perf hygiene). Runs the filtered
    /// closure with the cached index slice borrowed from the [`RefCell`].
    fn with_visible_indices<R>(&self, f: impl FnOnce(&[usize]) -> R) -> R {
        let key = self.visible_cache_key();
        {
            let cache = self.visible_cache.borrow();
            if cache.as_ref().is_some_and(|c| c.key == key) {
                return f(&cache.as_ref().expect("checked is_some_and above").indices);
            }
        }
        // Cache miss: rebuild the index list. Compile the option regex ONCE (or None when the query is
        // empty / no match-option is active / it fails to compile — in those cases every hit passes,
        // mirroring `hit_matches_client_options`).
        let opts = self.options();
        let regex = if self.query.trim().is_empty()
            || (!opts.case_sensitive && !opts.whole_word && !opts.is_regex)
        {
            None
        } else {
            compile_search_regex(&self.query, opts).ok()
        };
        let indices: Vec<usize> = match &regex {
            None => (0..self.results.len()).collect(),
            Some(re) => self
                .results
                .iter()
                .enumerate()
                .filter(|(_, h)| hit_matches_regex(h, re, opts))
                .map(|(i, _)| i)
                .collect(),
        };
        *self.visible_cache.borrow_mut() = Some(VisibleCache { key, indices });
        let cache = self.visible_cache.borrow();
        f(&cache.as_ref().expect("just stored above").indices)
    }

    /// The hits passing the client-side option filter (the React `visibleResults`), via the memoized
    /// index cache (no per-frame regex recompile or full-set clone — perf hygiene).
    pub fn visible_results(&self) -> Vec<&LoomGraphSearchHit> {
        self.with_visible_indices(|idx| idx.iter().map(|&i| &self.results[i]).collect())
    }

    /// The number of hits passing the client-side option filter, via the memoized cache (cheap — no clone).
    pub fn visible_result_count(&self) -> usize {
        self.with_visible_indices(<[usize]>::len)
    }

    /// `true` when a non-stale preview with plans exists (gates the Apply button — AC-8).
    pub fn can_apply(&self) -> bool {
        !self.preview_plans.is_empty()
            && self.preview_plan_key.as_deref() == Some(&self.current_replace_key())
    }

    /// Drain the off-thread delivery cells, folding any arrived result into state. Returns `true` if
    /// anything was delivered (so the caller can request a repaint).
    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        if let Ok(mut slot) = self.search_cell.lock() {
            if let Some(result) = slot.take() {
                self.loading = false;
                match result {
                    Ok((hits, key)) => {
                        self.results = hits;
                        self.result_set_key = Some(key);
                        self.error = None;
                    }
                    Err(msg) => {
                        self.results = Vec::new();
                        self.result_set_key = None;
                        self.error = Some(msg);
                    }
                }
                // A new (or cleared) result set invalidates the memoized visible-hit filter.
                self.results_generation = self.results_generation.wrapping_add(1);
                changed = true;
            }
        }
        let replace_delivery = self.replace_cell.lock().ok().and_then(|mut s| s.take());
        if let Some(delivery) = replace_delivery {
            self.loading = false;
            self.apply_replace_delivery(delivery);
            changed = true;
        }
        if let Ok(mut slot) = self.bookmark_cell.lock() {
            if let Some(result) = slot.take() {
                self.loading = false;
                match result {
                    Ok((blob, status)) => {
                        self.bookmarks = parse_bookmark_state(&blob);
                        if let Some(s) = status {
                            self.bookmark_status = Some(s);
                        }
                    }
                    Err(msg) => self.bookmark_status = Some(msg),
                }
                changed = true;
            }
        }
        changed
    }

    /// Fold a delivered replace-pipeline result into state.
    fn apply_replace_delivery(&mut self, delivery: ReplaceDelivery) {
        match delivery {
            ReplaceDelivery::Preview { plans, key } => {
                let plan_count = plans.len();
                self.preview_plans = plans;
                self.preview_plan_key = Some(key);
                self.replace_status = Some(if plan_count == 0 {
                    "No replacements matched in editable rich documents.".to_owned()
                } else {
                    format!("Previewed {plan_count} document replacement plan(s).")
                });
                self.error = None;
            }
            ReplaceDelivery::PreviewError(msg) => {
                self.preview_plans = Vec::new();
                self.preview_plan_key = None;
                self.error = Some(msg);
            }
            ReplaceDelivery::Applied {
                receipts,
                plan_count,
            } => {
                self.replace_status = Some(format!(
                    "Applied {plan_count} document replacement plan(s); receipts: {}",
                    receipts.join(", ")
                ));
                self.preview_plans = Vec::new();
                self.preview_plan_key = None;
                self.error = None;
            }
            ReplaceDelivery::AppliedPartial { receipts, error } => {
                // RISK-1 / MC-1: a partial failure NEVER loses the receipts already collected.
                self.replace_status = Some(format!(
                    "Applied {} document replacement plan(s) before failure; receipts: {}",
                    receipts.len(),
                    receipts.join(", ")
                ));
                self.preview_plans = Vec::new();
                self.preview_plan_key = None;
                self.error = Some(error);
            }
        }
    }

    /// Fire a workspace-wide search against `workspace_id` with the current query + filters + options.
    /// Guards no-workspace (MC-7, NO HTTP), empty-query, and regex-mode compile errors (PT-4) — each
    /// shows an error and fires no request. On a real fire, sets `loading`, clears the prior error, and
    /// resets the preview (a fresh search invalidates any stale plan).
    pub fn run_search(&mut self, client: &WorkspaceSearchClient, workspace_id: Option<&str>) {
        let Some(ws) = workspace_id else {
            self.error = Some("No workspace selected".to_owned());
            return;
        };
        let trimmed = self.query.trim();
        if trimmed.is_empty() {
            self.error = Some("Search query is required".to_owned());
            return;
        }
        // Regex-mode pre-validation so a bad pattern shows the error WITHOUT a backend round-trip (PT-4).
        if self.is_regex {
            if let Err(e) = compile_search_regex(trimmed, self.options()) {
                self.error = Some(e);
                return;
            }
        }
        let key = self.current_search_key();
        self.loading = true;
        self.error = None;
        self.preview_plans = Vec::new();
        self.preview_plan_key = None;
        self.result_set_key = None;
        if let Ok(mut slot) = self.search_cell.lock() {
            *slot = None;
        }
        client.search_paginated(
            ws,
            trimmed,
            self.kind.source_kind(),
            &self.tag_filter,
            &self.path_filter,
            self.options().to_search(),
            key,
            Arc::clone(&self.search_cell),
        );
    }

    /// Begin the Preview Replace pipeline: stale-result guard (RISK-2/MC-2 — a since-changed query
    /// shows the stale warning and computes NOTHING), regex-compile guard, then load each
    /// `KRD-`-prefixed hit document off-thread, walk its content_json, and accumulate the plans into the
    /// replace cell. No-workspace/no-document cases set a status and fire nothing.
    pub fn run_preview_replace(&mut self, client: &RichDocClient, workspace_id: Option<&str>) {
        let Some(ws) = workspace_id else {
            self.replace_status = Some("No workspace selected".to_owned());
            return;
        };
        let opts = self.options();
        let regex = match compile_search_regex(self.query.trim(), opts) {
            Ok(r) => r,
            Err(e) => {
                self.error = Some(e);
                self.preview_plans = Vec::new();
                self.preview_plan_key = None;
                return;
            }
        };
        // STALE-RESULT guard: the results must have been fetched under the CURRENT search params.
        if self.result_set_key.as_deref() != Some(&self.current_search_key()) {
            self.replace_status = Some(
                "Search results are stale; run Search again before previewing replacements."
                    .to_owned(),
            );
            self.preview_plans = Vec::new();
            self.preview_plan_key = None;
            return;
        }
        // Unique KRD- document ids from the live result set (RISK-5).
        let mut seen = std::collections::HashSet::new();
        let document_ids: Vec<String> = self
            .results
            .iter()
            .filter_map(document_id_from_hit)
            .filter(|id| seen.insert(id.clone()))
            .collect();
        if document_ids.is_empty() {
            self.replace_status =
                Some("No editable rich documents in the backend result set.".to_owned());
            self.preview_plans = Vec::new();
            self.preview_plan_key = None;
            return;
        }
        let key = self.current_replace_key();
        self.loading = true;
        self.error = None;
        self.replace_status = None;
        if let Ok(mut slot) = self.replace_cell.lock() {
            *slot = None;
        }
        client.preview_replace(
            ws,
            document_ids,
            regex,
            self.replacement.clone(),
            opts,
            key,
            Arc::clone(&self.replace_cell),
        );
    }

    /// Apply the current preview plans: stale-plan guard (RISK-2/MC-2 — a since-changed search or
    /// replacement shows the stale warning and applies NOTHING), then save each plan off-thread with its
    /// captured `expected_version` (optimistic concurrency; a 409 stops with partial receipts preserved).
    pub fn run_apply(&mut self, client: &RichDocClient, workspace_id: Option<&str>) {
        let Some(ws) = workspace_id else {
            self.replace_status = Some("No workspace selected".to_owned());
            return;
        };
        if self.preview_plans.is_empty() {
            return;
        }
        // STALE-PLAN guard: the preview must match the current search+replacement.
        if self.preview_plan_key.as_deref() != Some(&self.current_replace_key()) {
            self.replace_status =
                Some("Preview is stale; run Preview Replace again before applying.".to_owned());
            return;
        }
        self.loading = true;
        self.error = None;
        if let Ok(mut slot) = self.replace_cell.lock() {
            *slot = None;
        }
        client.apply_plans(
            ws,
            self.preview_plans.clone(),
            Arc::clone(&self.replace_cell),
        );
    }

    /// Load the saved bookmarks for `workspace_id` (called when the panel mounts). No-op when no
    /// workspace; clears any stale list.
    pub fn load_bookmarks(&mut self, client: &WorkspaceSearchClient, workspace_id: Option<&str>) {
        let Some(ws) = workspace_id else {
            self.bookmarks = Vec::new();
            return;
        };
        self.bookmark_status = None;
        if let Ok(mut slot) = self.bookmark_cell.lock() {
            *slot = None;
        }
        client.load_bookmarks(ws, Arc::clone(&self.bookmark_cell));
    }

    /// Save the current search as a bookmark (dedup by stable id, cap at 20), persisting the whole list.
    /// Refuses an empty search (no query + All kind + no filters).
    pub fn save_bookmark(&mut self, client: &WorkspaceSearchClient, workspace_id: Option<&str>) {
        let Some(ws) = workspace_id else {
            self.bookmark_status = Some("No workspace selected".to_owned());
            return;
        };
        if self.query.trim().is_empty()
            && self.kind == KindFilter::All
            && self.tag_filter.trim().is_empty()
            && self.path_filter.trim().is_empty()
        {
            self.bookmark_status = Some("Add a query or filter before bookmarking.".to_owned());
            return;
        }
        let mut bookmark = SearchBookmark {
            id: String::new(),
            label: String::new(),
            query: self.query.clone(),
            kind: self.kind,
            tag_filter: self.tag_filter.clone(),
            path_filter: self.path_filter.clone(),
            case_sensitive: self.case_sensitive,
            whole_word: self.whole_word,
            is_regex: self.is_regex,
            saved_at: now_iso8601(),
        };
        bookmark.id = bookmark.stable_id();
        bookmark.label = bookmark.display_label();
        // Dedup by id, newest first, cap at 20.
        let mut next: Vec<SearchBookmark> = vec![bookmark.clone()];
        next.extend(
            self.bookmarks
                .iter()
                .filter(|b| b.id != bookmark.id)
                .cloned(),
        );
        next.truncate(MAX_WORKSPACE_SEARCH_BOOKMARKS);
        self.persist_bookmarks(
            client,
            ws,
            next,
            format!("Saved search bookmark {}", bookmark.label),
        );
    }

    /// Restore a bookmark into the live query/filter/option fields (purely local — no HTTP). Clears the
    /// stale result/preview state.
    pub fn restore_bookmark(&mut self, bookmark: &SearchBookmark) {
        self.query = bookmark.query.clone();
        self.kind = bookmark.kind;
        self.tag_filter = bookmark.tag_filter.clone();
        self.path_filter = bookmark.path_filter.clone();
        self.case_sensitive = bookmark.case_sensitive;
        self.whole_word = bookmark.whole_word;
        self.is_regex = bookmark.is_regex;
        self.results = Vec::new();
        self.preview_plans = Vec::new();
        self.preview_plan_key = None;
        self.result_set_key = None;
        self.replace_status = None;
        self.error = None;
        self.bookmark_status = Some(format!("Restored search bookmark {}", bookmark.label));
    }

    /// Remove a bookmark (persisting the shortened list).
    pub fn remove_bookmark(
        &mut self,
        client: &WorkspaceSearchClient,
        workspace_id: Option<&str>,
        bookmark_id: &str,
    ) {
        let Some(ws) = workspace_id else {
            return;
        };
        let next: Vec<SearchBookmark> = self
            .bookmarks
            .iter()
            .filter(|b| b.id != bookmark_id)
            .cloned()
            .collect();
        self.persist_bookmarks(client, ws, next, "Removed search bookmark".to_owned());
    }

    /// Persist a bookmark list to the backend off-thread, delivering the saved (re-parsed) list +
    /// `status` into the bookmark cell.
    fn persist_bookmarks(
        &mut self,
        client: &WorkspaceSearchClient,
        ws: &str,
        bookmarks: Vec<SearchBookmark>,
        status: String,
    ) {
        self.bookmark_status = None;
        if let Ok(mut slot) = self.bookmark_cell.lock() {
            *slot = None;
        }
        let blob = bookmark_state_blob(&bookmarks);
        client.save_bookmarks(ws, blob, status, Arc::clone(&self.bookmark_cell));
    }

    /// The honest loading/status text for the header line.
    pub fn header_status(&self) -> String {
        if self.loading {
            return "Working…".to_owned();
        }
        if let Some(err) = &self.error {
            return err.clone();
        }
        if let Some(status) = &self.replace_status {
            return status.clone();
        }
        let n = self.visible_result_count();
        if self.result_set_key.is_some() {
            let plural = if n == 1 { "" } else { "s" };
            return format!("{n} result{plural}");
        }
        "Enter a query".to_owned()
    }
}

/// A typed result delivered by the off-thread replace pipeline.
#[derive(Debug, Clone, PartialEq)]
pub enum ReplaceDelivery {
    /// Preview computed `plans` under `key`.
    Preview {
        plans: Vec<ReplacementPlan>,
        key: String,
    },
    /// Preview failed (a document load failed).
    PreviewError(String),
    /// All `plan_count` plans applied; `receipts` are the per-document save receipt ids.
    Applied {
        receipts: Vec<String>,
        plan_count: usize,
    },
    /// Apply failed partway: `receipts` of the docs already saved are preserved; `error` is the failure.
    AppliedPartial {
        receipts: Vec<String>,
        error: String,
    },
}

/// A monotonic ISO-8601-ish timestamp for the bookmark `savedAt` field. Uses `chrono` (already a
/// transitive dep) so the value is a real UTC instant.
fn now_iso8601() -> String {
    chrono::Utc::now().to_rfc3339()
}

// ── Callbacks ─────────────────────────────────────────────────────────────────────────────────────────

/// Callbacks the host wires into the panel.
pub struct FindInFilesCallbacks<'a> {
    /// Open a hit's target (document/loom-block/etc.) in place — routed by the shell. The
    /// `(source_kind, ref_id, document_id?)` tuple lets the shell pick the open path.
    pub on_open_hit: &'a mut dyn FnMut(&LoomGraphSearchHit),
}

// ── Render ────────────────────────────────────────────────────────────────────────────────────────────

/// Render the panel: query/replace bars, match toggles, kind/tag/path filters, action buttons, the
/// results list, and the preview list. Drains the async cells first; dispatches actions through the two
/// clients + the callbacks. `workspace_id` is the active workspace (the no-workspace guards show an error
/// rather than 404ing).
#[allow(clippy::too_many_arguments)]
pub fn show(
    ui: &mut egui::Ui,
    state: &mut FindInFilesPanelState,
    palette: &HsPalette,
    search_client: &WorkspaceSearchClient,
    doc_client: &RichDocClient,
    workspace_id: Option<&str>,
    callbacks: &mut FindInFilesCallbacks<'_>,
) {
    state.poll();
    if state.loading {
        ui.ctx().request_repaint();
    }

    ui.heading("Find in Files");
    ui.label(egui::RichText::new("Workspace-wide search + replace").weak());
    ui.add_space(4.0);

    // Deferred action flags (dispatched after the immutable borrows end).
    let mut fire_search = false;
    let mut fire_preview = false;
    let mut fire_apply = false;
    let mut fire_cancel = false;
    let mut fire_save_bookmark = false;

    // ── Query + match toggles ──
    ui.horizontal(|ui| {
        let edit = egui::TextEdit::singleline(&mut state.query)
            .hint_text("Search workspace")
            .desired_width(220.0);
        let resp = ui.add(edit);
        accessibility::emit_interactive_node(ui.ctx(), resp.id, QUERY_AUTHOR_ID);
        if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            fire_search = true;
        }

        let case_btn = ui.add(egui::Button::new("Aa").selected(state.case_sensitive));
        accessibility::emit_interactive_node(ui.ctx(), case_btn.id, TOGGLE_CASE_AUTHOR_ID);
        if case_btn.clicked() {
            state.case_sensitive = !state.case_sensitive;
        }
        let word_btn = ui.add(egui::Button::new("W").selected(state.whole_word));
        accessibility::emit_interactive_node(ui.ctx(), word_btn.id, TOGGLE_WORD_AUTHOR_ID);
        if word_btn.clicked() {
            state.whole_word = !state.whole_word;
        }
        let regex_btn = ui.add(egui::Button::new(".*").selected(state.is_regex));
        accessibility::emit_interactive_node(ui.ctx(), regex_btn.id, TOGGLE_REGEX_AUTHOR_ID);
        if regex_btn.clicked() {
            state.is_regex = !state.is_regex;
        }

        let search_btn = ui.button("Search");
        accessibility::emit_interactive_node(ui.ctx(), search_btn.id, SEARCH_AUTHOR_ID);
        if search_btn.clicked() {
            fire_search = true;
        }
    });

    // ── Replacement + replace actions ──
    ui.horizontal(|ui| {
        let edit = egui::TextEdit::singleline(&mut state.replacement)
            .hint_text("Replace with")
            .desired_width(220.0);
        let resp = ui.add(edit);
        accessibility::emit_interactive_node(ui.ctx(), resp.id, REPLACE_AUTHOR_ID);

        let preview_enabled = state.result_set_key.is_some();
        let preview_btn = ui.add_enabled(preview_enabled, egui::Button::new("Preview Replace"));
        accessibility::emit_interactive_node(ui.ctx(), preview_btn.id, PREVIEW_REPLACE_AUTHOR_ID);
        if preview_btn.clicked() {
            fire_preview = true;
        }

        let apply_btn = ui.add_enabled(state.can_apply(), egui::Button::new("Apply"));
        accessibility::emit_interactive_node(ui.ctx(), apply_btn.id, APPLY_AUTHOR_ID);
        if apply_btn.clicked() {
            fire_apply = true;
        }

        let cancel_btn = ui.button("Cancel");
        accessibility::emit_interactive_node(ui.ctx(), cancel_btn.id, CANCEL_AUTHOR_ID);
        if cancel_btn.clicked() {
            fire_cancel = true;
        }
    });

    // ── Kind / tag / path filters ──
    ui.horizontal(|ui| {
        let combo = egui::ComboBox::from_id_salt(KIND_FILTER_AUTHOR_ID)
            .selected_text(state.kind.label())
            .show_ui(ui, |ui| {
                for kind in KindFilter::ALL {
                    ui.selectable_value(&mut state.kind, kind, kind.label());
                }
            });
        accessibility::emit_interactive_node(ui.ctx(), combo.response.id, KIND_FILTER_AUTHOR_ID);

        let tag = egui::TextEdit::singleline(&mut state.tag_filter)
            .hint_text("tag ids")
            .desired_width(120.0);
        let tag_resp = ui.add(tag);
        accessibility::emit_interactive_node(ui.ctx(), tag_resp.id, TAG_FILTER_AUTHOR_ID);

        let path = egui::TextEdit::singleline(&mut state.path_filter)
            .hint_text("path")
            .desired_width(120.0);
        let path_resp = ui.add(path);
        accessibility::emit_interactive_node(ui.ctx(), path_resp.id, PATH_FILTER_AUTHOR_ID);

        let bm_btn = ui.button("Bookmark Search");
        accessibility::emit_interactive_node(ui.ctx(), bm_btn.id, SAVE_BOOKMARK_AUTHOR_ID);
        if bm_btn.clicked() {
            fire_save_bookmark = true;
        }
    });

    // ── Status line ──
    ui.add_space(2.0);
    ui.label(state.header_status());
    if let Some(bm_status) = &state.bookmark_status {
        ui.label(egui::RichText::new(bm_status).weak());
    }

    // ── Saved searches (bookmarks) ──
    let mut restore_bookmark: Option<SearchBookmark> = None;
    let mut remove_bookmark_id: Option<String> = None;
    if !state.bookmarks.is_empty() {
        ui.separator();
        ui.label(egui::RichText::new("Saved searches").strong());
        for bm in &state.bookmarks {
            ui.horizontal(|ui| {
                ui.label(&bm.label);
                if ui.small_button("Restore").clicked() {
                    restore_bookmark = Some(bm.clone());
                }
                if ui.small_button("Remove").clicked() {
                    remove_bookmark_id = Some(bm.id.clone());
                }
            });
        }
    }

    // ── Results list (VIRTUALIZED — perf hygiene) ──
    // `open_hit_index` is a position into `visible_indices` (the on-screen visible list), resolved back
    // to `state.results` after the borrows end. `visible_indices` is the memoized client-side filter
    // result (cheap `Vec<usize>`, no per-frame regex recompile and no per-frame clone of the hits).
    let mut open_hit_index: Option<usize> = None;
    let visible_indices: Vec<usize> = state.with_visible_indices(<[usize]>::to_vec);
    if !visible_indices.is_empty() {
        ui.separator();
        // Borrow `results` (not all of `state`) so the row closure only holds the shared read it needs.
        let results = &state.results;
        // Uniform slot height so `show_rows` lays out ONLY the on-screen rows (title line + excerpt line +
        // Frame::group padding) instead of materializing every row in a large paginated result set.
        let row_height = ui.text_style_height(&egui::TextStyle::Body) * 2.0 + 18.0;
        egui::ScrollArea::vertical()
            .id_salt("find-in-files.results")
            .max_height(220.0)
            .auto_shrink([false, false])
            .show_rows(ui, row_height, visible_indices.len(), |ui, range| {
                for vi in range {
                    let hit = &results[visible_indices[vi]];
                    let frame = egui::Frame::group(ui.style());
                    let inner = frame.show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.strong(&hit.title);
                                ui.label(
                                    egui::RichText::new(format!("[{}]", hit.source_kind))
                                        .color(ui.visuals().weak_text_color())
                                        .small(),
                                );
                            });
                            if !hit.excerpt.is_empty() {
                                ui.label(egui::RichText::new(&hit.excerpt).small());
                            }
                        });
                    });
                    let row = inner.response.interact(egui::Sense::click());
                    accessibility::emit_interactive_node(
                        ui.ctx(),
                        row.id,
                        &result_author_id(&hit.source_kind, &hit.ref_id),
                    );
                    if row.clicked() {
                        open_hit_index = Some(vi);
                    }
                }
            });
    }

    // ── Preview list ──
    if !state.preview_plans.is_empty() {
        ui.separator();
        ui.label(egui::RichText::new("Replacement preview").strong());
        let text_color = ui.visuals().text_color();
        egui::ScrollArea::vertical()
            .id_salt("find-in-files.preview")
            .max_height(220.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for plan in &state.preview_plans {
                    let header = egui::CollapsingHeader::new(format!(
                        "{} ({})",
                        plan.title, plan.match_count
                    ))
                    .id_salt(preview_author_id(&plan.document_id))
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(format!("before: {}", plan.before_preview))
                                .small()
                                .weak(),
                        );
                        // Render the after-preview with the matched replacement highlighted via
                        // the theme `search_highlight_bg` token (NO Color32 literal — the theme
                        // guard). Each per-match after_preview is a small highlighted chip.
                        for mp in &plan.match_previews {
                            let mut job = egui::text::LayoutJob::default();
                            job.append(
                                &mp.after_preview,
                                0.0,
                                egui::TextFormat {
                                    color: text_color,
                                    background: palette.search_highlight_bg,
                                    ..Default::default()
                                },
                            );
                            ui.label(job);
                        }
                    });
                    accessibility::emit_interactive_node(
                        ui.ctx(),
                        header.header_response.id,
                        &preview_author_id(&plan.document_id),
                    );
                }
            });
    }

    // ── Dispatch deferred actions (after immutable borrows end) ──
    if let Some(vi) = open_hit_index {
        if let Some(hit) = visible_indices
            .get(vi)
            .and_then(|&ri| state.results.get(ri))
        {
            (callbacks.on_open_hit)(hit);
        }
    }
    if let Some(bm) = restore_bookmark {
        state.restore_bookmark(&bm);
    }
    if let Some(id) = remove_bookmark_id {
        state.remove_bookmark(search_client, workspace_id, &id);
    }
    if fire_cancel {
        state.preview_plans = Vec::new();
        state.preview_plan_key = None;
        state.replace_status = None;
        state.error = None;
    }
    if fire_search {
        state.run_search(search_client, workspace_id);
    }
    if fire_preview {
        state.run_preview_replace(doc_client, workspace_id);
    }
    if fire_apply {
        state.run_apply(doc_client, workspace_id);
    }
    if fire_save_bookmark {
        state.save_bookmark(search_client, workspace_id);
    }
}

// ── Pane factory (the in-product render path — AC, the WP-011 registry dispatch) ──────────────────────

/// Per-frame inputs the shell pushes to the [`FindInFilesPaneFactory`] (workspace id + palette IN) and
/// the open-hit requests it drains OUT. Mirrors the MT-028 `LoomSearchV2PaneShared` shape so the live
/// app threads the active workspace + theme through the `&self` `PaneFactory::render` without `&mut self`
/// on the factory map.
pub struct FindInFilesPaneShared {
    pub workspace_id: Option<String>,
    pub palette: HsPalette,
    /// Hits the operator/agent clicked this frame (FIFO), drained by the shell into the open path.
    pub open_requests: Vec<LoomGraphSearchHit>,
    /// Set true once the panel's bookmarks have been loaded for the active workspace, so the load fires
    /// exactly once per workspace (the React mount-effect equivalent).
    bookmarks_loaded_for: Option<String>,
}

impl FindInFilesPaneShared {
    pub fn new(palette: HsPalette) -> Self {
        Self {
            workspace_id: None,
            palette,
            open_requests: Vec::new(),
            bookmarks_loaded_for: None,
        }
    }
}

/// The CONCRETE `PaneFactory` for [`PaneType::FindInFiles`] — the in-product render path that makes the
/// "Find in Files" pane render the REAL panel instead of the placeholder. Mirrors the MT-028
/// `LoomSearchV2PaneFactory` exactly: panel state behind a `Mutex` (Send + Sync), the per-frame
/// workspace id + palette + open-hit drain flowing through [`FindInFilesPaneShared`], and the HTTP
/// transport reusing the real verified clients.
pub struct FindInFilesPaneFactory {
    state: Mutex<FindInFilesPanelState>,
    search_client: WorkspaceSearchClient,
    doc_client: RichDocClient,
    shared: Arc<Mutex<FindInFilesPaneShared>>,
}

impl FindInFilesPaneFactory {
    pub fn new(
        search_client: WorkspaceSearchClient,
        doc_client: RichDocClient,
        shared: Arc<Mutex<FindInFilesPaneShared>>,
    ) -> Self {
        Self::with_state(
            search_client,
            doc_client,
            shared,
            FindInFilesPanelState::new(),
        )
    }

    pub fn with_state(
        search_client: WorkspaceSearchClient,
        doc_client: RichDocClient,
        shared: Arc<Mutex<FindInFilesPaneShared>>,
        state: FindInFilesPanelState,
    ) -> Self {
        Self {
            state: Mutex::new(state),
            search_client,
            doc_client,
            shared,
        }
    }
}

impl PaneFactory for FindInFilesPaneFactory {
    fn pane_type(&self) -> PaneType {
        PaneType::FindInFiles
    }

    fn render(&self, ui: &mut egui::Ui, _ctx: &PaneRenderContext) {
        let (workspace_id, palette) = {
            let guard = self.shared.lock().unwrap_or_else(|p| p.into_inner());
            (guard.workspace_id.clone(), guard.palette.clone())
        };
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());

        // Load the workspace's bookmarks exactly once per workspace (the React mount-effect).
        let needs_bookmark_load = {
            let mut guard = self.shared.lock().unwrap_or_else(|p| p.into_inner());
            if guard.bookmarks_loaded_for != workspace_id {
                guard.bookmarks_loaded_for = workspace_id.clone();
                workspace_id.is_some()
            } else {
                false
            }
        };
        if needs_bookmark_load {
            state.load_bookmarks(&self.search_client, workspace_id.as_deref());
        }

        let shared_for_open = Arc::clone(&self.shared);
        let mut on_open = move |hit: &LoomGraphSearchHit| {
            if let Ok(mut guard) = shared_for_open.lock() {
                guard.open_requests.push(hit.clone());
            }
        };
        let mut callbacks = FindInFilesCallbacks {
            on_open_hit: &mut on_open,
        };
        show(
            ui,
            &mut state,
            &palette,
            &self.search_client,
            &self.doc_client,
            workspace_id.as_deref(),
            &mut callbacks,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn doc(text_nodes: &[&str], code: Option<&str>) -> serde_json::Value {
        let mut content: Vec<serde_json::Value> = text_nodes
            .iter()
            .map(|t| json!({ "type": "text", "text": t }))
            .collect();
        if let Some(code) = code {
            content.push(json!({
                "type": "codeBlock",
                "attrs": { "code": code, "language": "rust" }
            }));
        }
        json!({ "type": "doc", "content": content })
    }

    #[test]
    fn compile_regex_escapes_non_regex_query() {
        // RISK-8: `a.b` in non-regex mode must NOT match `acb` (the dot is escaped to literal).
        let re = compile_search_regex("a.b", MatchOptions::default()).unwrap();
        assert!(re.is_match("a.b"), "literal dot matches a.b");
        assert!(
            !re.is_match("acb"),
            "RISK-8: escaped dot does NOT match acb"
        );
    }

    #[test]
    fn compile_regex_invalid_pattern_is_err() {
        // PT-4: an invalid regex returns Err with a non-empty message (no panic).
        let err = compile_search_regex(
            "[invalid",
            MatchOptions {
                is_regex: true,
                ..Default::default()
            },
        )
        .unwrap_err();
        assert!(
            !err.is_empty(),
            "PT-4: invalid regex yields a non-empty error"
        );
    }

    #[test]
    fn compile_regex_empty_query_is_err() {
        assert!(compile_search_regex("   ", MatchOptions::default()).is_err());
    }

    #[test]
    fn case_insensitive_by_default() {
        let re = compile_search_regex("Foo", MatchOptions::default()).unwrap();
        assert!(
            re.is_match("foo") && re.is_match("FOO"),
            "case-insensitive when not case_sensitive"
        );
        let re2 = compile_search_regex(
            "Foo",
            MatchOptions {
                case_sensitive: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(
            re2.is_match("Foo") && !re2.is_match("foo"),
            "case-sensitive when set"
        );
    }

    #[test]
    fn replace_segment_zero_length_match_terminates() {
        // RISK-3: a pattern that can match empty (`a*`) must terminate (no infinite loop) and replace
        // the non-empty runs only.
        let re = compile_search_regex(
            "a*",
            MatchOptions {
                is_regex: true,
                ..Default::default()
            },
        )
        .unwrap();
        let res = replace_segment(
            "baaab",
            &re,
            "X",
            MatchOptions {
                is_regex: true,
                ..Default::default()
            },
        );
        // The non-empty `aaa` run is replaced; the zero-length matches at b-positions are skipped.
        assert!(res.text.contains('X'), "the aaa run was replaced");
        assert!(res.count >= 1, "at least one non-empty match replaced");
    }

    #[test]
    fn replace_segment_whole_word_skips_substring() {
        let re = compile_search_regex(
            "cat",
            MatchOptions {
                whole_word: true,
                ..Default::default()
            },
        )
        .unwrap();
        let res = replace_segment(
            "cat category",
            &re,
            "dog",
            MatchOptions {
                whole_word: true,
                ..Default::default()
            },
        );
        assert_eq!(
            res.count, 1,
            "only the standalone 'cat' replaced, not 'cat' inside 'category'"
        );
        assert_eq!(res.text, "dog category");
    }

    #[test]
    fn replace_segment_regex_group_expansion() {
        let re = compile_search_regex(
            r"(\w+)@(\w+)",
            MatchOptions {
                is_regex: true,
                ..Default::default()
            },
        )
        .unwrap();
        let res = replace_segment(
            "user@host",
            &re,
            "$2.$1",
            MatchOptions {
                is_regex: true,
                ..Default::default()
            },
        );
        assert_eq!(res.text, "host.user", "$1/$2 group expansion");
    }

    #[test]
    fn replace_segment_dollar_literals() {
        let re = compile_search_regex(
            "x",
            MatchOptions {
                is_regex: true,
                ..Default::default()
            },
        )
        .unwrap();
        let res = replace_segment(
            "x",
            &re,
            "$$$&",
            MatchOptions {
                is_regex: true,
                ..Default::default()
            },
        );
        assert_eq!(res.text, "$x", "$$ => literal $, $& => whole match");
    }

    #[test]
    fn replace_in_content_walks_text_and_code_preserves_other_nodes() {
        // RISK-4: text nodes AND attrs.code are replaced; a non-text node (an embed) round-trips verbatim.
        let mut content = doc(&["hello FIND_TARGET world"], Some("let FIND_TARGET = 1;"));
        // Inject an embed node that must be preserved untouched.
        content["content"].as_array_mut().unwrap().push(json!({
            "type": "hsEmbed",
            "attrs": { "asset_id": "AST-1", "kind": "image" }
        }));
        let re = compile_search_regex("FIND_TARGET", MatchOptions::default()).unwrap();
        let res = replace_in_content(&content, &re, "REPLACED", MatchOptions::default());
        assert_eq!(res.count, 2, "one match in text, one in code");
        let arr = res.content["content"].as_array().unwrap();
        assert_eq!(arr[0]["text"], "hello REPLACED world");
        assert_eq!(arr[1]["attrs"]["code"], "let REPLACED = 1;");
        // The embed node is preserved VERBATIM.
        assert_eq!(arr[2]["type"], "hsEmbed");
        assert_eq!(arr[2]["attrs"]["asset_id"], "AST-1");
        assert!(res.after_preview.contains("REPLACED"));
    }

    #[test]
    fn replace_in_content_no_match_returns_zero_and_unchanged() {
        let content = doc(&["nothing here"], None);
        let re = compile_search_regex("ABSENT", MatchOptions::default()).unwrap();
        let res = replace_in_content(&content, &re, "X", MatchOptions::default());
        assert_eq!(res.count, 0);
        assert_eq!(res.content, content, "no-match returns the tree unchanged");
    }

    #[test]
    fn document_id_from_hit_requires_krd_prefix() {
        // RISK-5: a non-KRD- document_id returns None.
        let hit_bad = LoomGraphSearchHit {
            source_kind: "loom_block".into(),
            result_kind: "loom_block".into(),
            ref_id: "blk-1".into(),
            title: "T".into(),
            excerpt: String::new(),
            metadata: json!({ "document_id": "DOC-1" }),
            block: None,
        };
        assert_eq!(
            document_id_from_hit(&hit_bad),
            None,
            "RISK-5: non-KRD id rejected"
        );

        let hit_good = LoomGraphSearchHit {
            source_kind: "loom_block".into(),
            result_kind: "loom_block".into(),
            ref_id: "blk-1".into(),
            title: "T".into(),
            excerpt: String::new(),
            metadata: json!({ "rich_document_id": "KRD-42" }),
            block: None,
        };
        assert_eq!(document_id_from_hit(&hit_good), Some("KRD-42".to_owned()));

        // source_kind == document falls back to ref_id (when KRD-).
        let hit_doc = LoomGraphSearchHit {
            source_kind: "document".into(),
            result_kind: "loom_block".into(),
            ref_id: "KRD-99".into(),
            title: "T".into(),
            excerpt: String::new(),
            metadata: json!({}),
            block: None,
        };
        assert_eq!(document_id_from_hit(&hit_doc), Some("KRD-99".to_owned()));
    }

    #[test]
    fn document_id_from_hit_block_document_id() {
        let hit = LoomGraphSearchHit {
            source_kind: "loom_block".into(),
            result_kind: "loom_block".into(),
            ref_id: "blk-1".into(),
            title: "T".into(),
            excerpt: String::new(),
            metadata: json!({}),
            block: Some(json!({ "document_id": "KRD-7" })),
        };
        assert_eq!(document_id_from_hit(&hit), Some("KRD-7".to_owned()));
    }

    #[test]
    fn stale_plan_keys_change_with_query_and_replacement() {
        // RISK-2/MC-2: a query change OR a replacement change yields a different key.
        let opts = MatchOptions::default();
        let k1 = search_plan_key("cats", KindFilter::All, "", "", opts);
        let k2 = search_plan_key("cats and dogs", KindFilter::All, "", "", opts);
        assert_ne!(k1, k2, "query change => different search key");
        let r1 = replace_plan_key(&k1, "X");
        let r2 = replace_plan_key(&k1, "Y");
        assert_ne!(r1, r2, "replacement change => different replace key");
    }

    #[test]
    fn can_apply_false_when_preview_stale() {
        let mut s = FindInFilesPanelState::new();
        s.query = "cats".into();
        s.results = vec![]; // unused for the key
        s.result_set_key = Some(s.current_search_key());
        s.preview_plans = vec![ReplacementPlan {
            document_id: "KRD-1".into(),
            title: "T".into(),
            expected_version: 1,
            content_json_after: json!({}),
            crdt_document_id: None,
            match_count: 1,
            before_preview: String::new(),
            after_preview: String::new(),
            match_previews: vec![],
        }];
        s.preview_plan_key = Some(s.current_replace_key());
        assert!(s.can_apply(), "fresh preview => can apply");
        // Change the query AFTER the preview => the plan key no longer matches => cannot apply.
        s.query = "dogs".into();
        assert!(
            !s.can_apply(),
            "RISK-2/MC-2: a since-changed query makes the preview stale"
        );
    }

    #[test]
    fn no_workspace_search_sets_error_without_loading() {
        let mut s = FindInFilesPanelState::new();
        s.query = "x".into();
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let client = WorkspaceSearchClient::new("http://test.local", rt.handle().clone());
        s.run_search(&client, None);
        assert_eq!(s.error.as_deref(), Some("No workspace selected"));
        assert!(!s.loading, "MC-7: no HTTP fired");
    }

    #[test]
    fn preview_stale_result_guard() {
        let mut s = FindInFilesPanelState::new();
        s.query = "cats".into();
        // result_set_key reflects an OLD query; the current query differs => stale.
        s.result_set_key = Some(search_plan_key(
            "old",
            KindFilter::All,
            "",
            "",
            MatchOptions::default(),
        ));
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let client = RichDocClient::new("http://test.local", rt.handle().clone());
        s.run_preview_replace(&client, Some("ws-1"));
        assert!(
            s.replace_status
                .as_deref()
                .unwrap_or_default()
                .contains("stale"),
            "RISK-2/MC-2: stale results show the warning, compute no preview"
        );
        assert!(s.preview_plans.is_empty());
    }

    #[test]
    fn bookmark_blob_round_trips_with_schema_id() {
        let bm = SearchBookmark {
            id: "alpha".into(),
            label: "alpha".into(),
            query: "alpha".into(),
            kind: KindFilter::Document,
            tag_filter: "t1".into(),
            path_filter: "src".into(),
            case_sensitive: true,
            whole_word: false,
            is_regex: true,
            saved_at: "2026-06-23T00:00:00Z".into(),
        };
        let blob = bookmark_state_blob(std::slice::from_ref(&bm));
        // RISK-6: the schema_id MUST be exactly the backend-validated value.
        assert_eq!(blob["schema_id"], WORKSPACE_SEARCH_BOOKMARK_SCHEMA_ID);
        let parsed = parse_bookmark_state(&blob);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0], bm, "bookmark round-trips through the blob");
    }

    #[test]
    fn bookmark_blob_caps_at_twenty() {
        let many: Vec<SearchBookmark> = (0..30)
            .map(|i| SearchBookmark {
                id: format!("b{i}"),
                label: format!("b{i}"),
                query: format!("q{i}"),
                kind: KindFilter::All,
                tag_filter: String::new(),
                path_filter: String::new(),
                case_sensitive: false,
                whole_word: false,
                is_regex: false,
                saved_at: "2026-06-23T00:00:00Z".into(),
            })
            .collect();
        let blob = bookmark_state_blob(&many);
        assert_eq!(
            blob["bookmarks"].as_array().unwrap().len(),
            MAX_WORKSPACE_SEARCH_BOOKMARKS
        );
    }

    #[test]
    fn result_author_id_sanitizes_ref_id() {
        assert_eq!(
            result_author_id("loom_block", "blk/1:x"),
            "find-in-files.result.loom_block.blk-1-x"
        );
    }

    #[test]
    fn kind_filter_all_omits_source_kind() {
        assert_eq!(KindFilter::All.source_kind(), None);
        assert_eq!(KindFilter::Document.source_kind(), Some("document"));
    }

    #[test]
    fn restore_bookmark_repopulates_fields() {
        let mut s = FindInFilesPanelState::new();
        let bm = SearchBookmark {
            id: "x".into(),
            label: "x".into(),
            query: "needle".into(),
            kind: KindFilter::WikiPage,
            tag_filter: "tag-1".into(),
            path_filter: "src/app".into(),
            case_sensitive: true,
            whole_word: true,
            is_regex: true,
            saved_at: "2026-06-23T00:00:00Z".into(),
        };
        s.restore_bookmark(&bm);
        assert_eq!(s.query, "needle");
        assert_eq!(s.kind, KindFilter::WikiPage);
        assert_eq!(s.tag_filter, "tag-1");
        assert_eq!(s.path_filter, "src/app");
        assert!(s.case_sensitive && s.whole_word && s.is_regex);
    }
}
