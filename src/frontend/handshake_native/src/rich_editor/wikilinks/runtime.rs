//! The per-editor wikilink async runtime: transclusion-resolution cache, backlinks state with
//! generation-counter cancellation, autocomplete runtime, and the off-thread delivery cells
//! (WP-KERNEL-012 MT-015).
//!
//! This mirrors MT-014's `EmbedRuntime`: it owns everything that must survive across frames so a
//! re-render reuses resolved transclusions/backlinks (no re-fetch storm) and remembers the popup
//! state. The editor (`RichEditorState`) owns one `WikilinkRuntime`; a render call borrows it `&mut`.
//!
//! ## Caching + cancellation
//!
//! - Transclusions are cached per `ref_value` ([`TransclusionState`]); a terminal state (Resolved /
//!   Failed) is never re-fetched (mirrors the AC-9 embed caching).
//! - Backlinks use a GENERATION COUNTER (MC-004): when the document id changes (doc switching), the
//!   generation bumps and an older in-flight backlinks response that lands late is dropped — only the
//!   latest document's backlinks are applied. This prevents the "N concurrent in-flight requests on
//!   rapid doc switching" red-team failure.
//! - Backlinks are fetched ONCE on document load and refreshed only on an explicit refresh action
//!   (no per-frame background polling — red-team RISK-4 / impl note 3).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::rich_editor::wikilinks::autocomplete::AutocompleteRuntime;
use crate::rich_editor::wikilinks::client::{
    BacklinksResponse, LoomBlockTransclusion, RichDocBacklink, WikilinkBackend, WikilinkError,
};

/// The resolution state of one transclusion target, cached per `ref_value`. A terminal state
/// (Resolved/Failed) is never re-fetched.
#[derive(Debug, Clone)]
pub enum TransclusionState {
    /// The resolve is in flight (the view shows a spinner).
    Resolving,
    /// Resolved to a live source document — the read-through renders `content_json`.
    Resolved(LoomBlockTransclusion),
    /// The block did not resolve to a source (e.g. `unresolved_reason`); the view shows the typed
    /// reason (NOT an error — this is a clean "not yet a source" state).
    Unresolved(String),
    /// The fetch failed with a typed error; the view shows the error chip. A 404
    /// ([`WikilinkError::is_not_found`]) additionally offers a "Remove embed" action (MC-003).
    Failed(WikilinkError),
}

impl TransclusionState {
    /// True when the state is terminal (will not be re-fetched). `Resolving` is non-terminal.
    pub fn is_terminal(&self) -> bool {
        !matches!(self, TransclusionState::Resolving)
    }
}

/// The backlinks-panel state for the current document. Carries the generation it was fetched for so a
/// stale response (an older document's) is dropped (MC-004).
#[derive(Debug, Clone)]
pub enum BacklinksState {
    /// No fetch issued yet (the panel shows nothing until the first load).
    Idle,
    /// A fetch is in flight.
    Loading,
    /// The backlinks loaded for the current document.
    Loaded(Vec<RichDocBacklink>),
    /// The fetch failed with a typed error (the panel shows a small inline error).
    Failed(WikilinkError),
}

/// One-slot delivery cell for an off-thread transclusion resolution: `(ref_value, result)`.
type TransclusionDeliveryCell = Arc<Mutex<Option<(String, Result<LoomBlockTransclusion, WikilinkError>)>>>;

/// One-slot delivery cell for an off-thread backlinks fetch, tagged with the generation it was issued
/// for (MC-004 cancellation): `(generation, result)`.
type BacklinksDeliveryCell = Arc<Mutex<Option<(u64, Result<BacklinksResponse, WikilinkError>)>>>;

/// The per-editor wikilink runtime (owned by `RichEditorState`). Holds the autocomplete runtime, the
/// transclusion cache, the backlinks state + generation, the document id the backlinks are for, the
/// backend transport, the tokio handle, and the delivery cells.
pub struct WikilinkRuntime {
    /// The workspace whose blocks/documents resolve.
    pub workspace_id: String,
    /// The current document id (drives the backlinks fetch; a change bumps the generation).
    pub document_id: String,
    /// The backend transport (production reqwest; tests: a mock).
    pub backend: Arc<dyn WikilinkBackend>,
    /// The tokio handle resolutions spawn onto (`None` in headless tests).
    pub runtime: Option<tokio::runtime::Handle>,
    /// The autocomplete runtime (debounce + cancellation + search delivery).
    pub autocomplete: AutocompleteRuntime,
    /// Per-`ref_value` transclusion resolution cache.
    pub transclusions: HashMap<String, TransclusionState>,
    /// The backlinks-panel state for the current document.
    pub backlinks: BacklinksState,
    /// The monotonic backlinks generation; bumped on document change so a stale response is dropped.
    pub backlinks_generation: u64,
    /// Whether the backlinks header is expanded (the CollapsingHeader open state, persisted).
    pub backlinks_expanded: bool,
    /// `ref_value`s whose transclusion the operator removed via "Remove embed" — the renderer drops
    /// the node via a DeleteNode transaction; this set guards against re-rendering a just-removed
    /// embed mid-frame.
    pub removed_transclusions: std::collections::HashSet<String>,
    transclusion_cell: TransclusionDeliveryCell,
    backlinks_cell: BacklinksDeliveryCell,
}

impl WikilinkRuntime {
    /// Build a runtime over `backend` for `workspace_id`, spawning onto `runtime` (pass `None` for a
    /// headless test). The document id starts empty (the shell installs it when a document loads).
    pub fn new(
        workspace_id: impl Into<String>,
        backend: Arc<dyn WikilinkBackend>,
        runtime: Option<tokio::runtime::Handle>,
    ) -> Self {
        let workspace_id = workspace_id.into();
        let autocomplete = AutocompleteRuntime::new(workspace_id.clone(), Arc::clone(&backend), runtime.clone());
        Self {
            workspace_id,
            document_id: String::new(),
            backend,
            runtime,
            autocomplete,
            transclusions: HashMap::new(),
            backlinks: BacklinksState::Idle,
            backlinks_generation: 0,
            backlinks_expanded: true,
            removed_transclusions: std::collections::HashSet::new(),
            transclusion_cell: Arc::new(Mutex::new(None)),
            backlinks_cell: Arc::new(Mutex::new(None)),
        }
    }

    /// A headless runtime (no tokio handle) over `backend` — the test/seed constructor.
    pub fn headless(backend: Arc<dyn WikilinkBackend>) -> Self {
        Self::new("ws", backend, None)
    }

    /// Set the active document id. When it CHANGES, bump the backlinks generation (so a stale
    /// in-flight response is dropped — MC-004), reset the backlinks state to `Idle`, and clear the
    /// transclusion cache (a different document has different transcluded sources). A no-op when the
    /// id is unchanged (so re-rendering does not reset state).
    pub fn set_document(&mut self, document_id: impl Into<String>) {
        let document_id = document_id.into();
        if document_id == self.document_id {
            return;
        }
        self.document_id = document_id;
        self.backlinks_generation = self.backlinks_generation.wrapping_add(1);
        self.backlinks = BacklinksState::Idle;
        self.transclusions.clear();
        self.removed_transclusions.clear();
    }

    /// Ensure a transclusion is being (or has been) resolved: if it has no terminal state and is not
    /// in flight, mark it `Resolving` and spawn the fetch. A terminal state is never re-fetched. A
    /// no-op when there is no runtime (headless: the test seeds the cache directly).
    pub fn ensure_transclusion(&mut self, ref_value: &str) {
        match self.transclusions.get(ref_value) {
            Some(state) if state.is_terminal() => return, // resolved/unresolved/failed -> keep.
            Some(TransclusionState::Resolving) => return, // already in flight.
            _ => {}
        }
        self.transclusions.insert(ref_value.to_owned(), TransclusionState::Resolving);
        let Some(runtime) = self.runtime.clone() else {
            return;
        };
        let backend = Arc::clone(&self.backend);
        let cell = Arc::clone(&self.transclusion_cell);
        let workspace_id = self.workspace_id.clone();
        let ref_value = ref_value.to_owned();
        runtime.spawn(async move {
            let result = backend.resolve_transclusion(&workspace_id, &ref_value).await;
            if let Ok(mut slot) = cell.lock() {
                *slot = Some((ref_value, result));
            }
        });
    }

    /// Trigger a backlinks fetch for the current document (on load or on an explicit refresh — NOT
    /// per frame). Bumps the generation, marks `Loading`, and spawns the fetch. A no-op when the
    /// document id is empty (no document loaded) or there is no runtime (headless seeds directly).
    pub fn refresh_backlinks(&mut self) {
        if self.document_id.trim().is_empty() {
            return;
        }
        // Only enter the Loading (spinner) state when a runtime can actually dispatch the fetch.
        // Headless (no runtime) must NOT enter a perpetual Loading: nothing would ever resolve it, so
        // the egui::Spinner would request a repaint every frame forever (idle-CPU burn + harness.run()
        // max_steps in any full-widget test). Tests stage results directly via stage_backlinks.
        let Some(runtime) = self.runtime.clone() else {
            return;
        };
        self.backlinks_generation = self.backlinks_generation.wrapping_add(1);
        self.backlinks = BacklinksState::Loading;
        let generation = self.backlinks_generation;
        let backend = Arc::clone(&self.backend);
        let cell = Arc::clone(&self.backlinks_cell);
        let document_id = self.document_id.clone();
        runtime.spawn(async move {
            let result = backend.list_backlinks(&document_id).await;
            if let Ok(mut slot) = cell.lock() {
                *slot = Some((generation, result));
            }
        });
    }

    /// Fetch the backlinks ONCE on document load: if the state is still `Idle`, trigger a fetch. This
    /// is the "fetch on load, not on every frame" guard (red-team RISK-4 / impl note 3).
    pub fn ensure_backlinks_loaded(&mut self) {
        if matches!(self.backlinks, BacklinksState::Idle) {
            self.refresh_backlinks();
        }
    }

    /// Drain any off-thread transclusion/backlinks results delivered since the last frame into the
    /// caches. A backlinks result whose generation no longer matches `backlinks_generation` is
    /// DROPPED (MC-004). Returns true when something was applied (the caller can request a repaint).
    pub fn drain(&mut self) -> bool {
        let mut applied = false;
        if let Ok(mut slot) = self.transclusion_cell.lock() {
            if let Some((ref_value, result)) = slot.take() {
                let state = match result {
                    Ok(t) if t.resolved => TransclusionState::Resolved(t),
                    Ok(t) => TransclusionState::Unresolved(
                        t.unresolved_reason.unwrap_or_else(|| "source_unresolved".to_owned()),
                    ),
                    Err(e) => TransclusionState::Failed(e),
                };
                self.transclusions.insert(ref_value, state);
                applied = true;
            }
        }
        if let Ok(mut slot) = self.backlinks_cell.lock() {
            if let Some((generation, result)) = slot.take() {
                if generation == self.backlinks_generation {
                    self.backlinks = match result {
                        Ok(resp) => BacklinksState::Loaded(resp.backlinks),
                        Err(e) => BacklinksState::Failed(e),
                    };
                    applied = true;
                }
                // else: a stale (older-generation) backlinks response landed late -> dropped (MC-004).
            }
        }
        // Drain the autocomplete search delivery too (so all wikilink async results land in one place).
        applied
    }

    /// Mark a transclusion as removed by the operator (the renderer issued a DeleteNode); the embed is
    /// not re-resolved/re-rendered this frame.
    pub fn mark_removed(&mut self, ref_value: &str) {
        self.removed_transclusions.insert(ref_value.to_owned());
        self.transclusions.remove(ref_value);
    }

    // ── Test seams (headless: stage a delivery without a tokio runtime) ──────────────────────────

    /// Stage a transclusion delivery into the cell (test seam).
    #[cfg(test)]
    pub fn stage_transclusion(&self, ref_value: &str, result: Result<LoomBlockTransclusion, WikilinkError>) {
        *self.transclusion_cell.lock().unwrap() = Some((ref_value.to_owned(), result));
    }

    /// Stage a backlinks delivery into the cell tagged with `generation` (test seam).
    #[cfg(test)]
    pub fn stage_backlinks(&self, generation: u64, result: Result<BacklinksResponse, WikilinkError>) {
        *self.backlinks_cell.lock().unwrap() = Some((generation, result));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rich_editor::wikilinks::client::WikilinkFuture;
    use crate::rich_editor::wikilinks::client::WikilinkResult;

    /// A backend that always errors NotFound (drives the headless terminal-state paths).
    struct NotFoundBackend;
    impl WikilinkBackend for NotFoundBackend {
        fn search<'a>(&'a self, _ws: &'a str, _q: &'a str, _l: usize) -> WikilinkFuture<'a, Vec<WikilinkResult>> {
            Box::pin(async { Ok(vec![]) })
        }
        fn resolve_transclusion<'a>(&'a self, _ws: &'a str, r: &'a str) -> WikilinkFuture<'a, LoomBlockTransclusion> {
            let r = r.to_owned();
            Box::pin(async move { Err(WikilinkError::NotFound(r)) })
        }
        fn list_backlinks<'a>(&'a self, d: &'a str) -> WikilinkFuture<'a, BacklinksResponse> {
            let d = d.to_owned();
            Box::pin(async move { Err(WikilinkError::NotFound(d)) })
        }
    }

    fn rt() -> WikilinkRuntime {
        WikilinkRuntime::headless(Arc::new(NotFoundBackend))
    }

    fn resolved_transclusion(block_id: &str) -> LoomBlockTransclusion {
        LoomBlockTransclusion {
            block_id: block_id.into(),
            workspace_id: "ws".into(),
            source_document_id: Some("DOC-1".into()),
            source_doc_version: Some(1),
            content_json: Some(serde_json::json!({"type":"doc","content":[
                {"type":"paragraph","content":[{"type":"text","text":"transcluded body"}]}
            ]})),
            resolved: true,
            unresolved_reason: None,
        }
    }

    fn backlink(src: &str) -> RichDocBacklink {
        RichDocBacklink {
            backlink_id: format!("BL-{src}"),
            workspace_id: "ws".into(),
            relationship_id: "REL".into(),
            source_document_id: src.into(),
            link_kind: "note".into(),
            target: "DOC-1".into(),
            block_id: "BLK".into(),
        }
    }

    #[test]
    fn ensure_transclusion_is_idempotent_for_terminal_state() {
        let mut rt = rt();
        rt.transclusions.insert("BLK-1".into(), TransclusionState::Resolved(resolved_transclusion("BLK-1")));
        rt.ensure_transclusion("BLK-1");
        assert!(
            matches!(rt.transclusions.get("BLK-1"), Some(TransclusionState::Resolved(_))),
            "a terminal transclusion is not re-resolved"
        );
        // An absent one is marked Resolving (then would spawn in the runtime path).
        rt.ensure_transclusion("BLK-2");
        assert!(matches!(rt.transclusions.get("BLK-2"), Some(TransclusionState::Resolving)));
    }

    #[test]
    fn drain_applies_resolved_transclusion() {
        let mut rt = rt();
        rt.stage_transclusion("BLK-9", Ok(resolved_transclusion("BLK-9")));
        assert!(rt.drain());
        assert!(matches!(rt.transclusions.get("BLK-9"), Some(TransclusionState::Resolved(_))));
    }

    #[test]
    fn drain_maps_unresolved_to_unresolved_state() {
        let mut rt = rt();
        let mut t = resolved_transclusion("BLK-3");
        t.resolved = false;
        t.content_json = None;
        t.unresolved_reason = Some("source_deleted".into());
        rt.stage_transclusion("BLK-3", Ok(t));
        assert!(rt.drain());
        match rt.transclusions.get("BLK-3") {
            Some(TransclusionState::Unresolved(reason)) => assert_eq!(reason, "source_deleted"),
            other => panic!("expected Unresolved, got {other:?}"),
        }
    }

    #[test]
    fn drain_404_maps_to_failed_not_found_for_remove_affordance_mc003() {
        // MC-003: a 404 transclusion (deleted block) becomes Failed(NotFound) so the view can offer
        // a "Remove embed" action.
        let mut rt = rt();
        rt.stage_transclusion("BLK-X", Err(WikilinkError::NotFound("BLK-X".into())));
        assert!(rt.drain());
        match rt.transclusions.get("BLK-X") {
            Some(TransclusionState::Failed(e)) => assert!(e.is_not_found(), "404 -> NotFound -> Remove embed"),
            other => panic!("expected Failed(NotFound), got {other:?}"),
        }
    }

    #[test]
    fn backlinks_generation_cancels_stale_response_mc004() {
        // MC-004: switching documents bumps the generation; an older document's backlinks response
        // that lands late is dropped.
        let mut rt = rt();
        rt.set_document("DOC-A");
        let gen_a = rt.backlinks_generation;
        rt.backlinks = BacklinksState::Loading;
        // The operator switched to DOC-B before DOC-A's response arrived.
        rt.set_document("DOC-B");
        let gen_b = rt.backlinks_generation;
        assert_ne!(gen_a, gen_b);

        // DOC-A's STALE response lands -> dropped (generation mismatch).
        rt.stage_backlinks(gen_a, Ok(BacklinksResponse { source_document_id: "DOC-A".into(), backlinks: vec![backlink("X")] }));
        assert!(!rt.drain(), "MC-004: a stale-generation backlinks response is dropped");
        assert!(matches!(rt.backlinks, BacklinksState::Idle), "state unchanged by the stale response");

        // DOC-B's response lands -> applied.
        rt.stage_backlinks(gen_b, Ok(BacklinksResponse { source_document_id: "DOC-B".into(), backlinks: vec![backlink("Y"), backlink("Z")] }));
        assert!(rt.drain());
        match &rt.backlinks {
            BacklinksState::Loaded(links) => assert_eq!(links.len(), 2),
            other => panic!("expected Loaded(2), got {other:?}"),
        }
    }

    #[test]
    fn set_document_clears_transclusions_and_resets_backlinks() {
        let mut rt = rt();
        rt.set_document("DOC-A");
        rt.transclusions.insert("BLK-1".into(), TransclusionState::Resolved(resolved_transclusion("BLK-1")));
        rt.backlinks = BacklinksState::Loaded(vec![backlink("X")]);
        rt.set_document("DOC-B");
        assert!(rt.transclusions.is_empty(), "a new document clears the transclusion cache");
        assert!(matches!(rt.backlinks, BacklinksState::Idle), "a new document resets backlinks to Idle");
        // Re-setting the SAME document is a no-op (does not reset state).
        rt.backlinks = BacklinksState::Loaded(vec![backlink("Y")]);
        rt.set_document("DOC-B");
        assert!(matches!(rt.backlinks, BacklinksState::Loaded(_)), "same-document set_document is a no-op");
    }

    #[test]
    fn ensure_backlinks_loaded_stays_idle_without_runtime() {
        // Headless (no runtime) must NOT enter Loading: nothing would resolve it, so a Loading-state
        // egui::Spinner would repaint forever (idle-CPU + harness.run() max_steps). It stays Idle and
        // the panel renders a neutral non-animating "Backlinks not loaded." (tests stage state directly).
        let mut rt = rt();
        rt.set_document("DOC-A");
        assert!(matches!(rt.backlinks, BacklinksState::Idle));
        let gen = rt.backlinks_generation;
        rt.ensure_backlinks_loaded();
        assert!(
            matches!(rt.backlinks, BacklinksState::Idle),
            "headless (no runtime) stays Idle — no perpetual-spinner Loading"
        );
        assert_eq!(
            rt.backlinks_generation, gen,
            "no generation bump / fetch without a runtime to dispatch it (RISK-4 + no idle spinner)"
        );
    }

    #[test]
    fn mark_removed_drops_the_transclusion() {
        let mut rt = rt();
        rt.transclusions.insert("BLK-1".into(), TransclusionState::Failed(WikilinkError::NotFound("BLK-1".into())));
        rt.mark_removed("BLK-1");
        assert!(!rt.transclusions.contains_key("BLK-1"), "removed transclusion is dropped from the cache");
        assert!(rt.removed_transclusions.contains("BLK-1"));
    }
}
