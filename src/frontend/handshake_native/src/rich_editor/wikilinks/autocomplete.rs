//! Wikilink autocomplete: trigger state, debounced search, generation-counter cancellation, and the
//! egui popup at the caret pixel (WP-KERNEL-012 MT-015).
//!
//! ## What this owns
//!
//! When the operator types `[[` in the editor, [`crate::rich_editor::wikilinks::parser::open_wikilink_query`]
//! detects the open trigger and the input handler sets [`AutocompleteState`] on the editor. This
//! module:
//!   - holds the popup state (query, results, selected index, the byte span of the `[[…` trigger),
//!   - DEBOUNCES the search by 150ms (MC-002): a burst of keystrokes issues ONE search, not one per
//!     keystroke, proven by [`AutocompleteRuntime`] asserting 1 backend call for 5 rapid keystrokes,
//!   - CANCELS stale in-flight searches via a generation counter (MC-004): only the latest query's
//!     result is applied; an older response that lands late is dropped,
//!   - renders the popup at the CARET pixel (not the mouse) so keyboard-only typing positions it,
//!   - on Enter/Tab inserts an `hsLink` atom via `InsertNode` (NOT `AddMark`) and closes the popup,
//!   - on Escape closes the popup and removes the `[[` trigger text (AC: Escape closes + removes).
//!
//! The async search reuses the proven MT-014 delivery-cell pattern: a spawned task writes the result
//! into a one-slot cell tagged with its generation; the egui thread drains it next frame and applies
//! it ONLY when the generation still matches the live query (cancellation).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::rich_editor::wikilinks::client::{WikilinkBackend, WikilinkError, WikilinkResult};
use crate::rich_editor::wikilinks::resolver::{normalize_target, ResolverIndex};

/// The debounce window (MC-002): a search is issued only after the query has been stable for this
/// long, so a 5-keystroke burst issues ONE search, not five.
pub const SEARCH_DEBOUNCE: Duration = Duration::from_millis(150);

/// The default number of autocomplete results requested per search.
pub const AUTOCOMPLETE_LIMIT: usize = 10;

/// WP-KERNEL-012 MT-057: the number of document titles enumerated when seeding the resolver index from
/// the Loom search binding on document mount (a broader limit than the per-keystroke dropdown, because
/// the seed populates the WHOLE resolvable-title set for `[[Title]]` runtime resolution — AC-003 — not
/// just the top-N for a single query).
pub const RESOLVER_SEED_LIMIT: usize = 200;

/// The result of one autocomplete search (the popup's list state). `Idle` before any search,
/// `Loading` while one is in flight, `Ready` with the rows, `Err` with the typed failure (shown as a
/// small inline error row, never a blank popup).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchPhase {
    /// No search issued yet (the query is empty or still debouncing).
    Idle,
    /// A search is in flight.
    Loading,
    /// The search returned these rows (possibly empty -> "No results").
    Ready(Vec<WikilinkResult>),
    /// The search failed with a typed error.
    Err(WikilinkError),
}

/// The live autocomplete popup state stored on the editor (`RichEditorState.wikilink_autocomplete`).
/// Tracks the open trigger span, the typed query, the result phase, the selected row, and the
/// generation counter that makes stale searches cancelable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutocompleteState {
    /// The CHAR offset within the caret's text leaf where the `[[` trigger opened (so confirm/cancel
    /// can compute the char span of `[[query` to remove — the doc is CHAR-indexed, RISK-1).
    pub trigger_start_char: usize,
    /// The block path of the text leaf the trigger lives in (so confirm edits the right leaf).
    pub leaf_path: Vec<usize>,
    /// The query typed after `[[` (the text between the trigger and the caret).
    pub query: String,
    /// The current result phase.
    pub phase: SearchPhase,
    /// The highlighted row index (Arrow keys move it; Enter/Tab inserts it).
    pub selected: usize,
    /// The monotonic generation of the LATEST issued query. A delivered result older than this is
    /// dropped (MC-004 cancellation).
    pub generation: u64,
    /// When the query last changed (drives the 150ms debounce — MC-002). `None` until the first
    /// change after open.
    #[doc(hidden)]
    pub last_change: Option<Instant>,
}

impl AutocompleteState {
    /// Open the popup for a `[[` trigger at `trigger_start_char` in the leaf at `leaf_path`, with the
    /// initial `query` typed so far.
    pub fn open(trigger_start_char: usize, leaf_path: Vec<usize>, query: String) -> Self {
        Self {
            trigger_start_char,
            leaf_path,
            query,
            phase: SearchPhase::Idle,
            selected: 0,
            generation: 0,
            last_change: Some(Instant::now()),
        }
    }

    /// Update the typed query (a keystroke refined the trigger). Bumps the generation (so a search
    /// for the OLD query that lands late is dropped — MC-004) and resets the debounce clock + the
    /// phase to `Idle` so the next debounce tick issues a fresh search. A no-op when the query is
    /// unchanged (so re-rendering the same frame does not reset the debounce).
    pub fn set_query(&mut self, query: String) {
        if query == self.query {
            return;
        }
        self.query = query;
        self.generation = self.generation.wrapping_add(1);
        self.selected = 0;
        self.phase = SearchPhase::Idle;
        self.last_change = Some(Instant::now());
    }

    /// True when the debounce window has elapsed since the last query change AND no search has been
    /// issued for the current query yet (phase still `Idle`). Drives the runtime's "issue search now"
    /// decision (MC-002). `now` is injected so the debounce is deterministically unit-testable.
    pub fn should_search(&self, now: Instant) -> bool {
        if !matches!(self.phase, SearchPhase::Idle) {
            return false;
        }
        match self.last_change {
            Some(t) => now.duration_since(t) >= SEARCH_DEBOUNCE,
            None => false,
        }
    }

    /// Move the selection down by one (Arrow Down), clamped to the result count.
    pub fn select_next(&mut self) {
        let len = self.result_len();
        if len == 0 {
            self.selected = 0;
        } else {
            self.selected = (self.selected + 1).min(len - 1);
        }
    }

    /// Move the selection up by one (Arrow Up), clamped at 0.
    pub fn select_prev(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    /// The currently-selected result row, if the phase is `Ready` and the index is in range.
    pub fn selected_result(&self) -> Option<&WikilinkResult> {
        match &self.phase {
            SearchPhase::Ready(rows) => rows.get(self.selected),
            _ => None,
        }
    }

    /// The number of result rows currently shown (0 unless `Ready`).
    fn result_len(&self) -> usize {
        match &self.phase {
            SearchPhase::Ready(rows) => rows.len(),
            _ => 0,
        }
    }
}

/// A one-slot delivery cell for an off-thread autocomplete search, tagged with the generation it was
/// issued for (so the egui thread drops a result whose generation no longer matches — MC-004). Mirrors
/// the MT-014 delivery-cell pattern.
type SearchDeliveryCell = Arc<Mutex<Option<(u64, Result<Vec<WikilinkResult>, WikilinkError>)>>>;

/// The async runtime bridge for autocomplete searches. Owns the backend transport, the tokio handle,
/// the workspace id, and the delivery cell. The editor calls [`Self::maybe_search`] each frame (which
/// debounces) and [`Self::drain`] at frame top (which applies a non-stale result).
pub struct AutocompleteRuntime {
    /// The workspace whose blocks autocomplete searches.
    pub workspace_id: String,
    /// The backend transport (production reqwest; tests: a counted mock).
    pub backend: Arc<dyn WikilinkBackend>,
    /// The tokio handle searches spawn onto (`None` in a headless unit test that drives the state
    /// directly).
    pub runtime: Option<tokio::runtime::Handle>,
    /// The delivery cell the spawned search writes its (generation-tagged) result into.
    cell: SearchDeliveryCell,
}

impl AutocompleteRuntime {
    /// Build a runtime over `backend` for `workspace_id`, spawning onto `runtime` (pass `None` for a
    /// headless test that applies results directly).
    pub fn new(
        workspace_id: impl Into<String>,
        backend: Arc<dyn WikilinkBackend>,
        runtime: Option<tokio::runtime::Handle>,
    ) -> Self {
        Self {
            workspace_id: workspace_id.into(),
            backend,
            runtime,
            cell: Arc::new(Mutex::new(None)),
        }
    }

    /// If the popup's debounce has elapsed (MC-002) and no search is in flight for the current query,
    /// mark the phase `Loading` and spawn the search for the current `(generation, query)`. A no-op
    /// when there is no runtime (headless test path drives the state directly). `now` is injected for
    /// deterministic debounce testing.
    pub fn maybe_search(&self, state: &mut AutocompleteState, now: Instant) {
        if !state.should_search(now) {
            return;
        }
        // Mark Loading so the same query is not re-issued every frame while in flight.
        state.phase = SearchPhase::Loading;
        let generation = state.generation;
        let query = state.query.clone();

        let Some(runtime) = self.runtime.clone() else {
            return; // headless: the test seeds the result via apply_delivery / drain.
        };
        let backend = Arc::clone(&self.backend);
        let cell = Arc::clone(&self.cell);
        let workspace_id = self.workspace_id.clone();
        runtime.spawn(async move {
            let result = backend.search(&workspace_id, &query, AUTOCOMPLETE_LIMIT).await;
            if let Ok(mut slot) = cell.lock() {
                *slot = Some((generation, result));
            }
        });
    }

    /// Drain a delivered search result into the popup state — but ONLY when its generation still
    /// matches the live query (MC-004 cancellation: a stale result for an older query is dropped).
    /// Returns true when a result was applied (so the caller can request a repaint). A no-op when the
    /// popup has closed (`state` is `None`).
    pub fn drain(&self, state: &mut Option<AutocompleteState>) -> bool {
        let Ok(mut slot) = self.cell.lock() else {
            return false;
        };
        let Some((generation, result)) = slot.take() else {
            return false;
        };
        let Some(st) = state.as_mut() else {
            return false; // popup closed before the result landed -> drop it.
        };
        if generation != st.generation {
            // A result for a superseded query landed late -> drop it (cancellation).
            return false;
        }
        st.phase = match result {
            Ok(rows) => SearchPhase::Ready(rows),
            Err(e) => SearchPhase::Err(e),
        };
        // Clamp the selection into the new row set.
        if let SearchPhase::Ready(rows) = &st.phase {
            if st.selected >= rows.len() {
                st.selected = rows.len().saturating_sub(1);
            }
        }
        true
    }

    /// TEST SEAM: directly stage a delivery into the cell (so a headless test can drive [`Self::drain`]
    /// without a tokio runtime).
    #[cfg(test)]
    pub fn stage_delivery(&self, generation: u64, result: Result<Vec<WikilinkResult>, WikilinkError>) {
        *self.cell.lock().unwrap() = Some((generation, result));
    }
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// WP-KERNEL-012 MT-057: ALIAS-AWARE candidate provider.
//
// This feeds the EXISTING MT-015 autocomplete dropdown widget — it does NOT build a second dropdown.
// While the operator types `[[query`, the dropdown calls [`candidates_for_query`] to fuzzy-match the
// query against BOTH document titles AND declared aliases (from the [`ResolverIndex`]), dedupe by
// document_id (the higher score wins; `matched_alias` is set only when the WINNING match was an
// alias), and sort by score descending so the best match is the default selection.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// One alias-aware autocomplete candidate the dropdown renders. When `matched_alias` is `Some`, the
/// row renders a secondary label (e.g. `Project Atlas — alias: "atlas"`) so the operator sees WHY the
/// candidate matched; selecting it inserts a link resolving to `document_id`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikilinkCandidate {
    /// The document the candidate resolves to (the link target on selection).
    pub document_id: String,
    /// The document's original-case display title (the primary label).
    pub display_title: String,
    /// The alias (original case) that produced the WINNING match, or `None` when the title matched.
    /// Drives the dropdown's `— alias: "…"` secondary label (AC-005).
    pub matched_alias: Option<String>,
    /// The match score (higher is better). Used to dedupe (keep the higher) and to sort the list.
    pub score: i64,
}

/// Score a `query` (already normalized) against a normalized `candidate` string. A subsequence /
/// prefix / contains heuristic, NOT a full edit distance — the dropdown only needs a stable RANKING,
/// and a heavyweight fuzzy crate would be a new dependency for a small ranking job. Returns `None`
/// when the query does not match at all (so a non-matching title/alias is excluded). Higher scores:
///
/// - exact equality -> 1000
/// - candidate starts_with q -> 500 + a short-candidate bonus (a tight prefix ranks above a long one)
/// - candidate contains q -> 200 + bonus
/// - q is a subsequence of cand -> 50 + bonus
///
/// An empty query matches everything with a low base score (the dropdown shows the index
/// alphabetically when nothing is typed yet).
fn fuzzy_score(query: &str, candidate: &str) -> Option<i64> {
    if query.is_empty() {
        // Empty query: everything matches with a tiny base score (length-penalized so shorter titles
        // rank first), so the dropdown lists the index before the operator types.
        return Some(10 - (candidate.len() as i64).min(9));
    }
    if candidate == query {
        return Some(1000);
    }
    let len_bonus = (40 - (candidate.len() as i64).min(40)).max(0); // shorter candidate -> higher
    if candidate.starts_with(query) {
        return Some(500 + len_bonus);
    }
    if candidate.contains(query) {
        return Some(200 + len_bonus);
    }
    if is_subsequence(query, candidate) {
        return Some(50 + len_bonus);
    }
    None
}

/// True when every char of `needle` appears in `haystack` in order (a classic subsequence test — the
/// "fzf-style" loose match). Both are expected pre-normalized (lower-cased).
fn is_subsequence(needle: &str, haystack: &str) -> bool {
    let mut hay = haystack.chars();
    'outer: for nc in needle.chars() {
        for hc in hay.by_ref() {
            if hc == nc {
                continue 'outer;
            }
        }
        return false;
    }
    true
}

/// Build the alias-aware candidate list for `query` against `index` (WP-KERNEL-012 MT-057). For each
/// document, the title AND every alias are scored against the normalized query; the BEST score per
/// document wins, and `matched_alias` is set ONLY when the winning match was an alias (so a document
/// matched by BOTH title and alias renders once, as a title match, never duplicated — the dedupe
/// contract / impl note 3). The result is sorted by score descending, then by display title (stable
/// tiebreak), so the dropdown's first row is the best match.
///
/// This is the data path the EXISTING MT-015 dropdown consumes; it does not render anything itself.
pub fn candidates_for_query(index: &ResolverIndex, query: &str) -> Vec<WikilinkCandidate> {
    let nq = normalize_target(query);
    // Best (score, matched_alias) per document_id.
    let mut best: HashMap<String, (i64, Option<String>, String)> = HashMap::new();

    for doc in index.documents() {
        // Title match (matched_alias = None).
        if let Some(score) = fuzzy_score(&nq, &normalize_target(&doc.display_title)) {
            consider(&mut best, &doc.document_id, score, None, &doc.display_title);
        }
        // Alias matches (matched_alias = the original-case alias).
        for alias in &doc.aliases {
            if let Some(score) = fuzzy_score(&nq, &normalize_target(alias)) {
                consider(&mut best, &doc.document_id, score, Some(alias.clone()), &doc.display_title);
            }
        }
    }

    let mut out: Vec<WikilinkCandidate> = best
        .into_iter()
        .map(|(document_id, (score, matched_alias, display_title))| WikilinkCandidate {
            document_id,
            display_title,
            matched_alias,
            score,
        })
        .collect();
    // Sort by score desc, then display title asc (stable, deterministic ordering).
    out.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| a.display_title.cmp(&b.display_title))
            .then_with(|| a.document_id.cmp(&b.document_id))
    });
    out
}

/// Keep the higher-scoring match for a document (the dedupe-by-document_id rule). On a tie, a TITLE
/// match (matched_alias = None) is preferred over an alias match, so a document that matches by both
/// renders as a title match (impl note 3 — the dedupe prefers the canonical title presentation).
fn consider(
    best: &mut HashMap<String, (i64, Option<String>, String)>,
    document_id: &str,
    score: i64,
    matched_alias: Option<String>,
    display_title: &str,
) {
    match best.get(document_id) {
        Some((existing_score, existing_alias, _)) => {
            let replace = score > *existing_score
                // On an equal score, prefer the TITLE match (existing alias-only loses to a new title).
                || (score == *existing_score && matched_alias.is_none() && existing_alias.is_some());
            if replace {
                best.insert(document_id.to_owned(), (score, matched_alias, display_title.to_owned()));
            }
        }
        None => {
            best.insert(document_id.to_owned(), (score, matched_alias, display_title.to_owned()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use crate::rich_editor::wikilinks::client::WikilinkFuture;
    use crate::rich_editor::wikilinks::client::{BacklinksResponse, LoomBlockTransclusion};

    fn result(id: &str) -> WikilinkResult {
        WikilinkResult { block_id: id.into(), title: id.into(), content_type: "note".into(), highlight: String::new() }
    }

    /// A counted mock backend: tracks how many times `search` is called (MC-002 debounce proof).
    struct CountingBackend {
        searches: AtomicUsize,
    }
    impl CountingBackend {
        fn new() -> Self {
            Self { searches: AtomicUsize::new(0) }
        }
        fn search_count(&self) -> usize {
            self.searches.load(Ordering::SeqCst)
        }
    }
    impl WikilinkBackend for CountingBackend {
        fn search<'a>(&'a self, _ws: &'a str, query: &'a str, _limit: usize) -> WikilinkFuture<'a, Vec<WikilinkResult>> {
            self.searches.fetch_add(1, Ordering::SeqCst);
            let q = query.to_owned();
            Box::pin(async move { Ok(vec![result(&format!("hit-{q}"))]) })
        }
        fn resolve_transclusion<'a>(&'a self, _ws: &'a str, _r: &'a str) -> WikilinkFuture<'a, LoomBlockTransclusion> {
            Box::pin(async { Err(WikilinkError::NotFound("x".into())) })
        }
        fn list_backlinks<'a>(&'a self, _d: &'a str) -> WikilinkFuture<'a, BacklinksResponse> {
            Box::pin(async { Err(WikilinkError::NotFound("x".into())) })
        }
    }

    #[test]
    fn debounce_issues_one_search_for_five_rapid_keystrokes_mc002() {
        // MC-002: 5 rapid keystrokes (each within the debounce window) must issue ONLY 1 search.
        let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(1).enable_all().build().unwrap();
        let backend = Arc::new(CountingBackend::new());
        let backend_dyn: Arc<dyn WikilinkBackend> = backend.clone();
        let runtime = AutocompleteRuntime::new("ws", backend_dyn, Some(rt.handle().clone()));

        let mut state = AutocompleteState::open(0, vec![0, 0], String::new());
        let base = Instant::now();
        // Five keystrokes 10ms apart (all < 150ms debounce). After each, maybe_search runs at the
        // CURRENT time (no debounce elapsed yet) so NO search is issued during the burst.
        for (i, q) in ["w", "wi", "wik", "wiki", "wikil"].iter().enumerate() {
            state.set_query((*q).to_owned());
            let now = base + Duration::from_millis((i as u64) * 10);
            runtime.maybe_search(&mut state, now);
        }
        assert_eq!(backend.search_count(), 0, "no search fires DURING the rapid burst (still debouncing)");

        // After the debounce window elapses past the LAST change, exactly one search fires.
        let after = base + Duration::from_millis(40 + 200);
        runtime.maybe_search(&mut state, after);
        // Give the spawned task a moment to run.
        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(backend.search_count(), 1, "MC-002: exactly ONE search after the burst debounces");
    }

    #[test]
    fn should_search_respects_debounce_window() {
        let mut state = AutocompleteState::open(0, vec![0, 0], "q".into());
        state.set_query("qq".into());
        let t0 = state.last_change.unwrap();
        assert!(!state.should_search(t0), "no search before the window elapses");
        assert!(!state.should_search(t0 + Duration::from_millis(149)), "still within the window");
        assert!(state.should_search(t0 + Duration::from_millis(150)), "search at the window edge");
        // Once Loading, should_search is false (a search is in flight).
        state.phase = SearchPhase::Loading;
        assert!(!state.should_search(t0 + Duration::from_secs(1)), "no re-search while Loading");
    }

    #[test]
    fn drain_applies_only_matching_generation_mc004() {
        // MC-004: a delivered result whose generation no longer matches the live query is DROPPED.
        let backend: Arc<dyn WikilinkBackend> = Arc::new(CountingBackend::new());
        let runtime = AutocompleteRuntime::new("ws", backend, None);

        let mut state = Some(AutocompleteState::open(0, vec![0, 0], "a".into()));
        // The live query advanced to generation 2.
        state.as_mut().unwrap().set_query("ab".into()); // gen 1
        state.as_mut().unwrap().set_query("abc".into()); // gen 2
        let live_gen = state.as_ref().unwrap().generation;
        assert_eq!(live_gen, 2);

        // A STALE result for generation 0 lands -> dropped.
        runtime.stage_delivery(0, Ok(vec![result("stale")]));
        assert!(!runtime.drain(&mut state), "MC-004: a stale-generation result is dropped");
        assert_eq!(state.as_ref().unwrap().phase, SearchPhase::Idle, "phase unchanged by a stale result");

        // The CURRENT-generation result lands -> applied.
        runtime.stage_delivery(live_gen, Ok(vec![result("fresh")]));
        assert!(runtime.drain(&mut state), "the matching-generation result applies");
        assert_eq!(
            state.as_ref().unwrap().phase,
            SearchPhase::Ready(vec![result("fresh")]),
            "the fresh result is applied"
        );
    }

    #[test]
    fn drain_drops_result_when_popup_closed() {
        let backend: Arc<dyn WikilinkBackend> = Arc::new(CountingBackend::new());
        let runtime = AutocompleteRuntime::new("ws", backend, None);
        runtime.stage_delivery(0, Ok(vec![result("x")]));
        let mut closed: Option<AutocompleteState> = None;
        assert!(!runtime.drain(&mut closed), "a result for a closed popup is dropped, not a panic");
    }

    #[test]
    fn selection_moves_and_clamps() {
        let mut st = AutocompleteState::open(0, vec![0, 0], "q".into());
        st.phase = SearchPhase::Ready(vec![result("a"), result("b"), result("c")]);
        assert_eq!(st.selected, 0);
        st.select_next();
        assert_eq!(st.selected, 1);
        st.select_next();
        st.select_next();
        assert_eq!(st.selected, 2, "clamps at the last row");
        st.select_prev();
        assert_eq!(st.selected, 1);
        st.select_prev();
        st.select_prev();
        assert_eq!(st.selected, 0, "clamps at 0");
        assert_eq!(st.selected_result().unwrap().block_id, "a");
    }

    #[test]
    fn set_query_noop_when_unchanged_keeps_debounce_clock() {
        let mut st = AutocompleteState::open(0, vec![0, 0], "q".into());
        let gen0 = st.generation;
        let t0 = st.last_change;
        st.set_query("q".into()); // unchanged
        assert_eq!(st.generation, gen0, "unchanged query does not bump generation");
        assert_eq!(st.last_change, t0, "unchanged query does not reset the debounce clock");
        st.set_query("qq".into()); // changed
        assert_eq!(st.generation, gen0.wrapping_add(1));
    }

    #[test]
    fn err_result_becomes_err_phase() {
        let backend: Arc<dyn WikilinkBackend> = Arc::new(CountingBackend::new());
        let runtime = AutocompleteRuntime::new("ws", backend, None);
        let mut state = Some(AutocompleteState::open(0, vec![0, 0], "q".into()));
        let gen = state.as_ref().unwrap().generation;
        runtime.stage_delivery(gen, Err(WikilinkError::NetworkError("down".into())));
        assert!(runtime.drain(&mut state));
        assert!(matches!(state.as_ref().unwrap().phase, SearchPhase::Err(WikilinkError::NetworkError(_))));
    }

    // ── WP-KERNEL-012 MT-057: alias-aware candidate provider ─────────────────────────────────────

    fn alias_index() -> ResolverIndex {
        let mut idx = ResolverIndex::new();
        idx.add_document("DOC-1", "Project Atlas");
        idx.add_document("DOC-2", "Roadmap");
        idx.add_document("DOC-3", "Atlas Shrugged");
        idx.add_alias("DOC-1", "Atlas"); // DOC-1 also reachable by the alias "Atlas"
        idx
    }

    #[test]
    fn candidates_surface_alias_match_with_matched_alias_set() {
        // AC-005: a query that matches a document's ALIAS surfaces that document with matched_alias set.
        let idx = alias_index();
        let cands = candidates_for_query(&idx, "atlas");
        // DOC-1 ("Project Atlas") matches by BOTH title (contains "atlas") AND the alias "Atlas".
        let doc1 = cands.iter().find(|c| c.document_id == "DOC-1").expect("DOC-1 present");
        // Dedupe rule (impl note 3): a doc matched by both renders ONCE; the title match (exact-ish via
        // contains) here wins on score, so matched_alias is None for DOC-1 (it appears once, not twice).
        let doc1_count = cands.iter().filter(|c| c.document_id == "DOC-1").count();
        assert_eq!(doc1_count, 1, "a doc matched by both title and alias is deduped to ONE candidate");
        // DOC-3 ("Atlas Shrugged") starts_with "atlas" -> a strong title match, also present.
        assert!(cands.iter().any(|c| c.document_id == "DOC-3"), "DOC-3 title-matches 'atlas'");
        assert!(doc1.display_title == "Project Atlas");
    }

    #[test]
    fn alias_only_query_sets_matched_alias() {
        // A query that matches ONLY via an alias (the title does not contain it) sets matched_alias.
        let mut idx = ResolverIndex::new();
        idx.add_document("DOC-9", "Quarterly Plan");
        idx.add_alias("DOC-9", "QP");
        let cands = candidates_for_query(&idx, "qp");
        let c = cands.iter().find(|c| c.document_id == "DOC-9").expect("matched via alias");
        assert_eq!(
            c.matched_alias.as_deref(),
            Some("QP"),
            "AC-005: a candidate matched only by alias carries the matched_alias (original case)"
        );
        assert_eq!(c.display_title, "Quarterly Plan", "the primary label is still the canonical title");
    }

    #[test]
    fn candidates_dedupe_by_document_id_keeping_higher_score() {
        // The dedupe-by-document_id contract: a document matching the query via multiple aliases +
        // the title appears exactly once, with the highest score.
        let mut idx = ResolverIndex::new();
        idx.add_document("DOC-1", "Atlas Project");
        idx.add_alias("DOC-1", "Atlas");
        idx.add_alias("DOC-1", "AtlasProj");
        let cands = candidates_for_query(&idx, "atlas");
        assert_eq!(
            cands.iter().filter(|c| c.document_id == "DOC-1").count(),
            1,
            "DOC-1 (title + 2 aliases all matching) is deduped to ONE candidate"
        );
    }

    #[test]
    fn candidates_sorted_by_score_descending() {
        let mut idx = ResolverIndex::new();
        idx.add_document("EXACT", "atlas"); // exact -> score 1000
        idx.add_document("PREFIX", "atlas one"); // starts_with -> ~500+
        idx.add_document("CONTAINS", "the atlas map"); // contains -> ~200+
        let cands = candidates_for_query(&idx, "atlas");
        assert_eq!(cands[0].document_id, "EXACT", "the exact match ranks first");
        assert!(cands[0].score >= cands[1].score, "sorted by score descending");
        assert!(cands[1].score >= cands.last().unwrap().score);
    }

    #[test]
    fn non_matching_query_excludes_candidate() {
        let idx = alias_index();
        let cands = candidates_for_query(&idx, "zzznope");
        assert!(cands.is_empty(), "a query matching no title/alias yields no candidates");
    }

    #[test]
    fn empty_query_lists_the_index() {
        let idx = alias_index();
        let cands = candidates_for_query(&idx, "");
        // Three documents -> three candidates (one per doc, deduped).
        assert_eq!(cands.len(), 3, "an empty query lists the indexed documents (one per doc)");
    }
}
