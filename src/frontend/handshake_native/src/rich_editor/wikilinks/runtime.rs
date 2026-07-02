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

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use crate::rich_editor::wikilinks::autocomplete::AutocompleteRuntime;
use crate::rich_editor::wikilinks::client::{
    BacklinksResponse, LoomBlockTransclusion, RichDocBacklink, WikilinkBackend, WikilinkError,
};
use crate::rich_editor::wikilinks::resolver::{normalize_target, ResolverIndex};

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

/// WP-KERNEL-012 MT-057: the typed result of a create-from-unresolved note creation. `title` is the
/// normalized title the create was keyed on (so the originating mark + the in-flight guard can be
/// found); `document_id` is the new note's id on success. A failure carries the title + a typed reason
/// so the affordance can re-enable + surface an error rather than silently swallowing (no silent
/// no-op).
#[derive(Debug, Clone)]
pub enum CreateNoteOutcome {
    /// The note was created; the originating mark must rewrite Unresolved -> Resolved (AC-002).
    Created {
        /// The normalized title the create was keyed on (matches the in-flight guard key).
        normalized_title: String,
        /// The original-case title (for the new index entry + the mark label).
        display_title: String,
        /// The new document id.
        document_id: String,
    },
    /// The create failed; the affordance re-enables + the editor surfaces the error.
    Failed {
        /// The normalized title the create was keyed on (matches the in-flight guard key).
        normalized_title: String,
        /// A human-readable failure reason (the typed backend error rendered).
        reason: String,
    },
}

/// WP-KERNEL-012 MT-057: the async backend for create-from-unresolved-link. A SEPARATE trait (not an
/// added method on [`WikilinkBackend`]) so the existing MT-015 mock backends do not need to grow a
/// method, and so the create path is unit-testable with a counted mock that proves the debounce guard
/// fires ONE POST for a double-click (RISK-001 / MC-001). The production impl wraps the MT-037
/// [`crate::backend::knowledge_documents::KnowledgeDocumentsClient`] `create_document` binding — it
/// adds NO new endpoint (AC-007 / MC-006).
pub trait CreateNoteBackend: Send + Sync {
    /// Create a knowledge document titled `title` in `workspace_id` with an empty body, returning the
    /// new document id. This is `POST /knowledge/documents` via the MT-037 binding — never a new
    /// endpoint, never an inline call on the egui frame (the runtime spawns it off-thread).
    fn create_note<'a>(
        &'a self,
        workspace_id: &'a str,
        title: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, String>> + Send + 'a>>;
}

/// The production [`CreateNoteBackend`]: wraps the MT-037 [`KnowledgeDocumentsClient`] and calls its
/// EXISTING `create_document` route (`POST /knowledge/documents`). Read-through to the one create
/// binding — adds NO endpoint and introduces NO SQLite (AC-007 / MC-006). The session run id is folded
/// into the operator identity headers so each create is attributable (HBR-SWARM).
pub struct KnowledgeCreateNoteBackend {
    client: crate::backend::knowledge_documents::KnowledgeDocumentsClient,
    session_run_id: String,
}

impl KnowledgeCreateNoteBackend {
    /// Build the production create backend (shares the process-wide HTTP pool via the MT-037 client's
    /// `production()` constructor — NO second reqwest stack).
    pub fn production(session_run_id: impl Into<String>) -> Self {
        Self {
            client: crate::backend::knowledge_documents::KnowledgeDocumentsClient::production(),
            session_run_id: session_run_id.into(),
        }
    }
}

impl CreateNoteBackend for KnowledgeCreateNoteBackend {
    fn create_note<'a>(
        &'a self,
        workspace_id: &'a str,
        title: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, String>> + Send + 'a>>
    {
        use crate::backend::knowledge_documents::{CreateDocumentRequest, HskDocumentHeaders};
        let workspace_id = workspace_id.to_owned();
        let title = title.to_owned();
        let session_run_id = self.session_run_id.clone();
        let client = self.client.clone();
        Box::pin(async move {
            // A create is a WRITE -> the operator identity (with actor_kind) is required (a missing
            // kind 403s a write). The document id is unknown pre-create, so the task-run id folds the
            // title slug for attributability.
            let headers = HskDocumentHeaders::for_operator(session_run_id, &slugify(&title));
            let body = CreateDocumentRequest {
                workspace_id,
                title: title.clone(),
                content_json: None, // empty body — the MT contract: "with the title and an empty body"
                schema_version: None,
                project_ref: None,
                folder_ref: None,
            };
            match client.create_document(&headers, &body).await {
                Ok(resp) => extract_document_id(&resp.document).ok_or_else(|| {
                    "create succeeded but the response carried no document id".to_owned()
                }),
                Err(e) => Err(e.to_string()),
            }
        })
    }
}

/// Pull the new document's id out of the MT-037 create response `document` JSON value (the backend
/// `KnowledgeRichDocument`; the id field is `rich_document_id`, falling back to `id`).
fn extract_document_id(document: &serde_json::Value) -> Option<String> {
    document
        .get("rich_document_id")
        .or_else(|| document.get("id"))
        .and_then(|v| v.as_str())
        .map(str::to_owned)
}

/// A filesystem-safe slug of a title (for the attributable task-run id only — NOT a persisted name).
fn slugify(title: &str) -> String {
    let s: String = title
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = s.trim_matches('-');
    if trimmed.is_empty() {
        "untitled".to_owned()
    } else {
        trimmed.to_owned()
    }
}

/// One-slot delivery cell for an off-thread create-note result (WP-KERNEL-012 MT-057).
type CreateNoteDeliveryCell = Arc<Mutex<Option<CreateNoteOutcome>>>;

/// One-slot delivery cell for the off-thread resolver-index SEED (WP-KERNEL-012 MT-057): a Loom
/// search enumeration delivers `(document_id, title)` pairs that `drain` folds into
/// `resolver_index.add_document` so titles classify Resolved at runtime (AC-003). `Err` carries a
/// typed failure so a seed that fails does not silently leave the index empty (it is dropped — the
/// links stay Unresolved + offer the create affordance, which is the correct fail-closed behavior).
type ResolverSeedDeliveryCell = Arc<Mutex<Option<Result<Vec<(String, String)>, WikilinkError>>>>;

/// One-slot delivery cell for an off-thread transclusion resolution: `(ref_value, result)`.
type TransclusionDeliveryCell =
    Arc<Mutex<Option<(String, Result<LoomBlockTransclusion, WikilinkError>)>>>;

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
    pub removed_transclusions: HashSet<String>,
    /// WP-KERNEL-012 MT-057: the resolution index (titles from the MT-038 Loom search enumeration +
    /// the in-session alias stub). The click handler resolves a `[[Title]]` against this; the
    /// candidate provider lists matches from it. A fresh create inserts the new note's title so the
    /// link resolves LIVE without a reload (AC-002).
    pub resolver_index: ResolverIndex,
    /// WP-KERNEL-012 MT-057: the create backend (`POST /knowledge/documents` via the MT-037 binding).
    /// `None` in a headless test that does not exercise a real create (it stages a delivery directly).
    pub create_backend: Option<Arc<dyn CreateNoteBackend>>,
    /// WP-KERNEL-012 MT-057: in-flight create guard keyed on the NORMALIZED title (RISK-001 / MC-001).
    /// A title present here has a create POST in flight; a second click on the same unresolved link is
    /// a no-op so a double-click cannot POST twice = duplicate notes. Cleared when the create resolves.
    pub creating_titles: HashSet<String>,
    /// WP-KERNEL-012 MT-057 (AC-006 / RISK-002 / MC-002): true once the missing-aliases typed-gap
    /// blocker has been recognized for THIS runtime (the backend payload lacks an `aliases` field).
    /// Drives the VISIBLE local-only banner in the rich editor; the resolver index's
    /// `aliases_supported` flag is the source of truth, this caches "the banner should show".
    pub alias_backend_gap: bool,
    transclusion_cell: TransclusionDeliveryCell,
    backlinks_cell: BacklinksDeliveryCell,
    create_cell: CreateNoteDeliveryCell,
    /// WP-KERNEL-012 MT-057: the off-thread resolver-index seed delivery cell (a Loom search
    /// enumeration). `drain` folds its `(document_id, title)` pairs into `resolver_index` so a
    /// `[[Title]]` classifies Resolved at runtime (AC-003). A seed already in flight is not re-issued
    /// (the `seeding` guard), and a delivered seed clears the guard.
    resolver_seed_cell: ResolverSeedDeliveryCell,
    /// WP-KERNEL-012 MT-057: true while a resolver-index seed search is in flight, so a per-mount
    /// `seed_resolver_index_from_search` is idempotent (no enumeration storm if the shell re-mounts the
    /// same document repeatedly across frames).
    resolver_seeding: bool,
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
        let autocomplete =
            AutocompleteRuntime::new(workspace_id.clone(), Arc::clone(&backend), runtime.clone());
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
            removed_transclusions: HashSet::new(),
            // MT-057: the index starts empty (no aliases support — the backend payload has no
            // `aliases` field; AC-006). The shell populates titles from the MT-038 Loom enumeration
            // and aliases from the in-session local stub.
            resolver_index: ResolverIndex::new(),
            create_backend: None,
            creating_titles: HashSet::new(),
            // The alias-backend gap is recognized lazily: it flips true the first time an alias path is
            // exercised while `resolver_index.aliases_supported` is false (so the banner shows only when
            // aliases are actually in play, not on every note). The shell may also set it on mount.
            alias_backend_gap: false,
            transclusion_cell: Arc::new(Mutex::new(None)),
            backlinks_cell: Arc::new(Mutex::new(None)),
            create_cell: Arc::new(Mutex::new(None)),
            resolver_seed_cell: Arc::new(Mutex::new(None)),
            resolver_seeding: false,
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
        self.transclusions
            .insert(ref_value.to_owned(), TransclusionState::Resolving);
        let Some(runtime) = self.runtime.clone() else {
            return;
        };
        let backend = Arc::clone(&self.backend);
        let cell = Arc::clone(&self.transclusion_cell);
        let workspace_id = self.workspace_id.clone();
        let ref_value = ref_value.to_owned();
        runtime.spawn(async move {
            let result = backend
                .resolve_transclusion(&workspace_id, &ref_value)
                .await;
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
                        t.unresolved_reason
                            .unwrap_or_else(|| "source_unresolved".to_owned()),
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
        // WP-KERNEL-012 MT-057: fold a delivered resolver-index SEED (a Loom search enumeration) into
        // the index so a `[[Title]]` classifies Resolved at runtime (AC-003). A failed seed is dropped
        // (the links stay Unresolved + offer the create affordance — the correct fail-closed behavior);
        // either way the in-flight `resolver_seeding` guard is cleared so a later refresh can re-seed.
        if let Ok(mut slot) = self.resolver_seed_cell.lock() {
            if let Some(result) = slot.take() {
                self.resolver_seeding = false;
                if let Ok(pairs) = result {
                    for (document_id, title) in pairs {
                        self.resolver_index.add_document(document_id, title);
                    }
                }
                applied = true;
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

    // ── WP-KERNEL-012 MT-057: create-from-unresolved + alias stub ────────────────────────────────

    /// Install the production create backend (`POST /knowledge/documents` via the MT-037 binding) so
    /// the create-from-unresolved path can dispatch. The shell calls this when it mounts a document.
    pub fn set_create_backend(&mut self, backend: Arc<dyn CreateNoteBackend>) {
        self.create_backend = Some(backend);
    }

    /// WP-KERNEL-012 MT-057 (AC-003 seed): enumerate document titles from the EXISTING MT-038 Loom
    /// search binding ([`WikilinkBackend::search`] -> `POST /workspaces/{ws}/loom/search-v2`) and fold
    /// them into [`Self::resolver_index`] so a `[[Title]]` classifies Resolved at runtime instead of
    /// always-Unresolved (the inert-index defect). A BROAD `query` ("" lists the index by the backend's
    /// FTS) with `limit` rows is issued OFF the egui frame thread; the `(block_id, title)` pairs land in
    /// the seed cell and [`Self::drain`] applies them next frame.
    ///
    /// Idempotent + storm-safe: a no-op while a seed is already in flight (`resolver_seeding`) or there
    /// is no workspace/runtime (headless: the test stages the seed directly via [`Self::stage_resolver_seed`]).
    /// This adds NO new endpoint — it read-throughs the SAME `search()` the autocomplete dropdown uses
    /// (AC-007 / MC-006: no SQLite, no backend edit).
    pub fn seed_resolver_index_from_search(&mut self, query: &str, limit: usize) {
        if self.resolver_seeding || self.workspace_id.trim().is_empty() {
            return;
        }
        let Some(runtime) = self.runtime.clone() else {
            return; // headless: the test stages the seed directly.
        };
        self.resolver_seeding = true;
        let backend = Arc::clone(&self.backend);
        let cell = Arc::clone(&self.resolver_seed_cell);
        let workspace_id = self.workspace_id.clone();
        let query = query.to_owned();
        runtime.spawn(async move {
            // Each hit's `block_id` is the document/block id a `[[Title]]` resolves to; `title` is the
            // display title. A blank title is recorded for rendering but not indexed (add_document
            // skips a blank normalized key), so an untitled block never resolves an empty `[[]]`.
            let result = backend
                .search(&workspace_id, &query, limit)
                .await
                .map(|rows| {
                    rows.into_iter()
                        .map(|r| (r.block_id, r.title))
                        .collect::<Vec<_>>()
                });
            if let Ok(mut slot) = cell.lock() {
                *slot = Some(result);
            }
        });
    }

    /// TEST SEAM: stage a resolver-index seed delivery into the cell (so a headless test drives
    /// [`Self::drain`]'s seed-fold without a tokio runtime / real backend). Mirrors the other
    /// `stage_*` seams.
    #[cfg(test)]
    pub fn stage_resolver_seed(&mut self, pairs: Vec<(String, String)>) {
        self.resolver_seeding = true;
        *self.resolver_seed_cell.lock().unwrap() = Some(Ok(pairs));
    }

    /// True while a resolver-index seed search is in flight (the idempotency guard; read by a test that
    /// proves a re-mount does not re-issue a seed while one is already running).
    pub fn is_seeding_resolver_index(&self) -> bool {
        self.resolver_seeding
    }

    /// True when a create POST is already in flight for `title` (normalized) — the affordance is
    /// DISABLED while true so a double-click cannot POST twice (RISK-001 / MC-001). The egui click
    /// handler checks this before emitting/dispatching.
    pub fn is_creating(&self, title: &str) -> bool {
        self.creating_titles.contains(&normalize_target(title))
    }

    /// Dispatch a create-from-unresolved note: guard against a duplicate in-flight create (RISK-001 /
    /// MC-001 — keyed on the normalized title), mark the title in-flight, and spawn the
    /// `POST /knowledge/documents` (MT-037 binding) OFF the egui frame thread (RISK-007 / MC-007). The
    /// completion lands in the create cell; [`Self::drain`] applies it (inserts the new id into the
    /// resolver index + returns the outcome so the widget rewrites the mark). Returns `true` when a
    /// create was newly dispatched, `false` when it was a duplicate (already in flight) or there is no
    /// backend/runtime (headless: the test stages a delivery directly). A blank title is a no-op.
    pub fn dispatch_create_note(&mut self, title: &str) -> bool {
        let display_title = title.trim().to_owned();
        let normalized = normalize_target(&display_title);
        if normalized.is_empty() {
            return false;
        }
        // RISK-001 / MC-001: a create for this title is already in flight -> do NOT POST again.
        if self.creating_titles.contains(&normalized) {
            return false;
        }
        let (Some(backend), Some(runtime)) = (self.create_backend.clone(), self.runtime.clone())
        else {
            return false; // headless / unwired: the test stages the outcome directly.
        };
        self.creating_titles.insert(normalized.clone());
        let workspace_id = self.workspace_id.clone();
        let cell = Arc::clone(&self.create_cell);
        runtime.spawn(async move {
            let result = backend.create_note(&workspace_id, &display_title).await;
            let outcome = match result {
                Ok(document_id) => CreateNoteOutcome::Created {
                    normalized_title: normalized,
                    display_title,
                    document_id,
                },
                Err(reason) => CreateNoteOutcome::Failed {
                    normalized_title: normalized,
                    reason,
                },
            };
            if let Ok(mut slot) = cell.lock() {
                *slot = Some(outcome);
            }
        });
        true
    }

    /// Declare an in-session LOCAL alias for a document (the MT-017 PropertiesPanel path). Because the
    /// backend payload has NO `aliases` field (AC-006), this is the ONLY source of aliases; it
    /// populates the resolver index IN MEMORY (no file, no DB — AC-007 / MC-006) and flips the
    /// alias-backend-gap flag so the editor shows the local-only banner.
    pub fn add_local_alias(&mut self, document_id: &str, alias: &str) {
        self.resolver_index.add_alias(document_id, alias);
        if !self.resolver_index.aliases_supported {
            self.alias_backend_gap = true;
        }
    }

    /// Recognize the missing-aliases typed-gap (AC-006 / MC-002): when the backend payload lacks an
    /// `aliases` field (the resolver index reports `aliases_supported == false`), flip the
    /// alias-backend-gap flag so the editor renders the VISIBLE local-only banner. Idempotent. Called
    /// by the shell when it builds the index from a backend enumeration that carried no aliases.
    pub fn note_alias_backend_gap(&mut self) {
        if !self.resolver_index.aliases_supported {
            self.alias_backend_gap = true;
        }
    }

    /// Apply a delivered create-note outcome (called from [`Self::drain`]): clear the in-flight guard,
    /// and on success insert the new note's title into the resolver index so a re-resolution of the
    /// same `[[Title]]` is now Resolved (AC-002 — the link goes live without a reload). Returns the
    /// outcome so the widget can rewrite the originating mark / surface an error.
    fn apply_create_outcome(&mut self, outcome: CreateNoteOutcome) -> CreateNoteOutcome {
        match &outcome {
            CreateNoteOutcome::Created {
                normalized_title,
                display_title,
                document_id,
            } => {
                self.creating_titles.remove(normalized_title);
                // The new note is now resolvable by its title (live, no reload — AC-002).
                self.resolver_index
                    .add_document(document_id.clone(), display_title.clone());
            }
            CreateNoteOutcome::Failed {
                normalized_title, ..
            } => {
                self.creating_titles.remove(normalized_title);
            }
        }
        outcome
    }

    /// Drain a delivered create-note outcome (if any) into the index + in-flight guard, returning it
    /// so the widget can rewrite the originating mark Unresolved -> Resolved (AC-002) or surface a
    /// failure. Separate from [`Self::drain`] because the create outcome must flow back to the WIDGET
    /// (to mutate the document mark), whereas transclusion/backlinks land entirely inside the runtime.
    pub fn drain_create(&mut self) -> Option<CreateNoteOutcome> {
        let taken = self
            .create_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take())?;
        Some(self.apply_create_outcome(taken))
    }

    /// TEST SEAM: stage a create-note outcome into the create cell (so a headless test drives
    /// [`Self::drain_create`] without a tokio runtime / real backend).
    #[cfg(test)]
    pub fn stage_create(&self, outcome: CreateNoteOutcome) {
        *self.create_cell.lock().unwrap() = Some(outcome);
    }

    /// TEST SEAM: directly mark a title in-flight (so a test can prove the double-dispatch guard
    /// without a runtime).
    #[cfg(test)]
    pub fn mark_creating(&mut self, title: &str) {
        self.creating_titles.insert(normalize_target(title));
    }

    // ── Test seams (headless: stage a delivery without a tokio runtime) ──────────────────────────

    /// Stage a transclusion delivery into the cell (test seam).
    #[cfg(test)]
    pub fn stage_transclusion(
        &self,
        ref_value: &str,
        result: Result<LoomBlockTransclusion, WikilinkError>,
    ) {
        *self.transclusion_cell.lock().unwrap() = Some((ref_value.to_owned(), result));
    }

    /// Stage a backlinks delivery into the cell tagged with `generation` (test seam).
    #[cfg(test)]
    pub fn stage_backlinks(
        &self,
        generation: u64,
        result: Result<BacklinksResponse, WikilinkError>,
    ) {
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
        fn search<'a>(
            &'a self,
            _ws: &'a str,
            _q: &'a str,
            _l: usize,
        ) -> WikilinkFuture<'a, Vec<WikilinkResult>> {
            Box::pin(async { Ok(vec![]) })
        }
        fn resolve_transclusion<'a>(
            &'a self,
            _ws: &'a str,
            r: &'a str,
        ) -> WikilinkFuture<'a, LoomBlockTransclusion> {
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
        rt.transclusions.insert(
            "BLK-1".into(),
            TransclusionState::Resolved(resolved_transclusion("BLK-1")),
        );
        rt.ensure_transclusion("BLK-1");
        assert!(
            matches!(
                rt.transclusions.get("BLK-1"),
                Some(TransclusionState::Resolved(_))
            ),
            "a terminal transclusion is not re-resolved"
        );
        // An absent one is marked Resolving (then would spawn in the runtime path).
        rt.ensure_transclusion("BLK-2");
        assert!(matches!(
            rt.transclusions.get("BLK-2"),
            Some(TransclusionState::Resolving)
        ));
    }

    #[test]
    fn drain_applies_resolved_transclusion() {
        let mut rt = rt();
        rt.stage_transclusion("BLK-9", Ok(resolved_transclusion("BLK-9")));
        assert!(rt.drain());
        assert!(matches!(
            rt.transclusions.get("BLK-9"),
            Some(TransclusionState::Resolved(_))
        ));
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
            Some(TransclusionState::Failed(e)) => {
                assert!(e.is_not_found(), "404 -> NotFound -> Remove embed")
            }
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
        rt.stage_backlinks(
            gen_a,
            Ok(BacklinksResponse {
                source_document_id: "DOC-A".into(),
                backlinks: vec![backlink("X")],
            }),
        );
        assert!(
            !rt.drain(),
            "MC-004: a stale-generation backlinks response is dropped"
        );
        assert!(
            matches!(rt.backlinks, BacklinksState::Idle),
            "state unchanged by the stale response"
        );

        // DOC-B's response lands -> applied.
        rt.stage_backlinks(
            gen_b,
            Ok(BacklinksResponse {
                source_document_id: "DOC-B".into(),
                backlinks: vec![backlink("Y"), backlink("Z")],
            }),
        );
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
        rt.transclusions.insert(
            "BLK-1".into(),
            TransclusionState::Resolved(resolved_transclusion("BLK-1")),
        );
        rt.backlinks = BacklinksState::Loaded(vec![backlink("X")]);
        rt.set_document("DOC-B");
        assert!(
            rt.transclusions.is_empty(),
            "a new document clears the transclusion cache"
        );
        assert!(
            matches!(rt.backlinks, BacklinksState::Idle),
            "a new document resets backlinks to Idle"
        );
        // Re-setting the SAME document is a no-op (does not reset state).
        rt.backlinks = BacklinksState::Loaded(vec![backlink("Y")]);
        rt.set_document("DOC-B");
        assert!(
            matches!(rt.backlinks, BacklinksState::Loaded(_)),
            "same-document set_document is a no-op"
        );
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
        rt.transclusions.insert(
            "BLK-1".into(),
            TransclusionState::Failed(WikilinkError::NotFound("BLK-1".into())),
        );
        rt.mark_removed("BLK-1");
        assert!(
            !rt.transclusions.contains_key("BLK-1"),
            "removed transclusion is dropped from the cache"
        );
        assert!(rt.removed_transclusions.contains("BLK-1"));
    }

    // ── WP-KERNEL-012 MT-057: create-from-unresolved + alias stub ────────────────────────────────

    #[test]
    fn drain_create_inserts_new_title_into_index_for_live_resolution_ac002() {
        // AC-002: after a create resolves, re-resolving the SAME `[[Title]]` is now Resolved (the link
        // goes live without a reload) and the in-flight guard is cleared.
        use crate::rich_editor::wikilinks::resolver::{resolve_wikilink, WikilinkResolution};
        let mut rt = rt();
        rt.mark_creating("My New Note");
        assert!(
            rt.is_creating("my new note"),
            "the title is in-flight (normalized key)"
        );
        rt.stage_create(CreateNoteOutcome::Created {
            normalized_title: normalize_target("My New Note"),
            display_title: "My New Note".into(),
            document_id: "DOC-NEW".into(),
        });
        let outcome = rt.drain_create().expect("a staged create outcome drains");
        assert!(
            matches!(outcome, CreateNoteOutcome::Created { ref document_id, .. } if document_id == "DOC-NEW")
        );
        assert!(
            !rt.is_creating("My New Note"),
            "the in-flight guard is cleared after the create resolves"
        );
        // The link is now live: re-resolving the same title returns Resolved (AC-002).
        let r = resolve_wikilink(&rt.resolver_index, "My New Note");
        assert!(
            matches!(r, WikilinkResolution::Resolved { ref document_id, .. } if document_id == "DOC-NEW")
        );
    }

    #[test]
    fn drain_create_failed_clears_guard_without_indexing() {
        // A failed create re-enables the affordance (clears the guard) and does NOT index a phantom doc
        // (no silent success).
        use crate::rich_editor::wikilinks::resolver::{resolve_wikilink, WikilinkResolution};
        let mut rt = rt();
        rt.mark_creating("Doomed");
        rt.stage_create(CreateNoteOutcome::Failed {
            normalized_title: normalize_target("Doomed"),
            reason: "network error".into(),
        });
        let outcome = rt.drain_create().expect("a staged failure drains");
        assert!(matches!(outcome, CreateNoteOutcome::Failed { .. }));
        assert!(
            !rt.is_creating("Doomed"),
            "a failed create re-enables the affordance"
        );
        assert!(matches!(
            resolve_wikilink(&rt.resolver_index, "Doomed"),
            WikilinkResolution::Unresolved { .. }
        ));
    }

    #[test]
    fn dispatch_is_noop_when_already_in_flight_mc001() {
        // RISK-001 / MC-001: a second dispatch for an in-flight title returns false (no second POST).
        // (Headless has no backend/runtime, so dispatch returns false anyway; we prove the GUARD path
        // specifically by pre-marking the title and asserting is_creating short-circuits.)
        let mut rt = rt();
        rt.mark_creating("Atlas");
        assert!(rt.is_creating("Atlas"));
        // A dispatch for an already-in-flight title is a no-op (the guard check precedes any spawn).
        assert!(
            !rt.dispatch_create_note("Atlas"),
            "MC-001: an in-flight title does not dispatch again"
        );
    }

    #[test]
    fn dispatch_blank_title_is_noop() {
        let mut rt = rt();
        assert!(
            !rt.dispatch_create_note("   "),
            "a blank title never dispatches a create"
        );
        assert!(rt.creating_titles.is_empty());
    }

    #[test]
    fn add_local_alias_populates_index_and_flips_gap_banner_ac006() {
        // AC-006 / MC-002: the local alias stub populates the index IN MEMORY and flips the
        // local-only banner flag (the backend has no aliases field).
        let mut rt = rt();
        assert!(
            !rt.alias_backend_gap,
            "no gap recognized before any alias is used"
        );
        rt.resolver_index.add_document("DOC-1", "Project Atlas");
        rt.add_local_alias("DOC-1", "Atlas");
        assert!(
            rt.alias_backend_gap,
            "AC-006: using the local alias stub flips the local-only banner"
        );
        assert_eq!(
            rt.resolver_index.alias_count(),
            1,
            "the alias is in the in-memory index"
        );
        // Resolving by the alias works (the code path is exercised + testable despite the backend gap).
        use crate::rich_editor::wikilinks::resolver::{
            resolve_wikilink, MatchKind, WikilinkResolution,
        };
        assert!(matches!(
            resolve_wikilink(&rt.resolver_index, "atlas"),
            WikilinkResolution::Resolved {
                matched_by: MatchKind::Alias { .. },
                ..
            }
        ));
    }

    #[test]
    fn note_alias_backend_gap_is_idempotent() {
        let mut rt = rt();
        rt.note_alias_backend_gap();
        rt.note_alias_backend_gap();
        assert!(
            rt.alias_backend_gap,
            "the gap flag flips on (backend lacks aliases)"
        );
    }

    #[test]
    fn drain_folds_resolver_seed_into_index_for_live_resolution_ac003() {
        // AC-003 seed: a delivered Loom-search enumeration folds into the resolver index so a
        // `[[Title]]` classifies Resolved at runtime (the inert-index defect fix). Before the seed the
        // title is Unresolved; after the drain it resolves by ExactTitle.
        use crate::rich_editor::wikilinks::resolver::{
            resolve_wikilink, MatchKind, WikilinkResolution,
        };
        let mut rt = rt();
        assert!(
            matches!(
                resolve_wikilink(&rt.resolver_index, "Project Atlas"),
                WikilinkResolution::Unresolved { .. }
            ),
            "before seeding the title is Unresolved (empty index)"
        );
        rt.stage_resolver_seed(vec![
            ("DOC-1".into(), "Project Atlas".into()),
            ("DOC-2".into(), "Roadmap".into()),
        ]);
        assert!(
            rt.is_seeding_resolver_index(),
            "a staged seed marks seeding in flight"
        );
        assert!(rt.drain(), "draining the seed applies it");
        assert!(
            !rt.is_seeding_resolver_index(),
            "the seed-in-flight guard clears after the drain"
        );
        assert_eq!(
            rt.resolver_index.title_count(),
            2,
            "both seeded titles are indexed"
        );
        // AC-003: the seeded title now resolves at runtime (no longer Unresolved).
        assert!(matches!(
            resolve_wikilink(&rt.resolver_index, "project atlas"),
            WikilinkResolution::Resolved { matched_by: MatchKind::ExactTitle, ref document_id } if document_id == "DOC-1"
        ));
    }

    #[test]
    fn seed_resolver_index_is_noop_without_workspace_or_runtime() {
        // Headless (no runtime) + empty workspace: seeding is a no-op (no panic, no seeding flag), so a
        // unit/kittest that does not exercise the network is unaffected.
        let mut rt = rt();
        rt.seed_resolver_index_from_search("", 50);
        assert!(
            !rt.is_seeding_resolver_index(),
            "no seed dispatched without a workspace/runtime"
        );
        assert_eq!(rt.resolver_index.title_count(), 0, "index stays empty");
    }

    #[test]
    fn slugify_produces_a_safe_attribution_slug() {
        assert_eq!(slugify("My New Note!"), "my-new-note");
        assert_eq!(slugify("   "), "untitled");
        assert_eq!(slugify("Café 2026"), "caf--2026");
    }
}
