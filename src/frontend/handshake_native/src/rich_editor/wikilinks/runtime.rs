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

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex};

use crate::rich_editor::wikilinks::autocomplete::AutocompleteRuntime;
use crate::rich_editor::wikilinks::client::{
    BacklinksResponse, LoomBlockTransclusion, RichDocBacklink, WikilinkBackend, WikilinkError,
};
use crate::rich_editor::wikilinks::resolver::{normalize_target, ResolverIndex};

const BACKLINKS_INVALIDATION_ID: &str = "handshake.backlinks-invalidation";
const BACKLINKS_INVALIDATION_WINDOW: usize = 256;

#[derive(Clone, Default)]
struct BacklinksInvalidationLog {
    revision: u64,
    events: VecDeque<u64>,
    current_warnings: HashMap<(String, String), String>,
    latest_revision_by_workspace: HashMap<String, u64>,
}

/// Broadcast one backlink-projection invalidation through egui's shared context data. Every mounted
/// rich editor observes the same revision on its next frame, so saving source A refreshes an already
/// mounted target B panel. A post-commit indexing warning is broadcast as a visible failure instead
/// of allowing a pane to present stale rows as current.
pub fn publish_backlinks_invalidation(
    ctx: &egui::Context,
    workspace_id: impl Into<String>,
    source_document_id: impl Into<String>,
    warning: Option<String>,
) -> u64 {
    ctx.data_mut(|data| {
        let id = egui::Id::new(BACKLINKS_INVALIDATION_ID);
        let mut log = data
            .get_temp::<BacklinksInvalidationLog>(id)
            .unwrap_or_default();
        log.revision = log.revision.wrapping_add(1);
        let revision = log.revision;
        let workspace_id = workspace_id.into();
        let source_document_id = source_document_id.into();
        let warning_key = (workspace_id.clone(), source_document_id.clone());
        log.latest_revision_by_workspace
            .insert(workspace_id.clone(), revision);
        match &warning {
            Some(message) => {
                log.current_warnings.insert(warning_key, message.clone());
            }
            None => {
                log.current_warnings.remove(&warning_key);
            }
        }
        log.events.push_back(revision);
        while log.events.len() > BACKLINKS_INVALIDATION_WINDOW {
            log.events.pop_front();
        }
        data.insert_temp(id, log);
        revision
    })
}

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
        /// Backend authority: true when this request inserted the document, false when the
        /// idempotent create route opened/reused an existing document.
        created: bool,
    },
    /// The create failed; the affordance re-enables + the editor surfaces the error.
    Failed {
        /// The normalized title the create was keyed on (matches the in-flight guard key).
        normalized_title: String,
        /// A human-readable failure reason (the typed backend error rendered).
        reason: String,
    },
}

/// Successful create-route projection retained across the async runtime boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateNoteWrite {
    pub document_id: String,
    pub created: bool,
}

/// Host-owned create completion stamped with the context that originated the request. The shell
/// drains every completion even after the mounted widget changes document/workspace; the origin
/// identity decides whether the old document mark may be rewritten, while navigation/failure still
/// surfaces exactly once.
#[derive(Debug, Clone)]
pub struct CreateNoteCompletion {
    pub context_generation: u64,
    pub workspace_id: String,
    pub document_id: String,
    pub outcome: CreateNoteOutcome,
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
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<CreateNoteWrite, String>> + Send + 'a>,
    >;
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

    /// Build the same production create adapter against an explicitly selected Handshake backend.
    /// Managed-runtime proofs use this to keep the canonical create path aligned with the backend URL
    /// selected by the fixture; the adapter still delegates to the one MT-037 document client and the
    /// real `POST /knowledge/documents` route.
    pub fn with_base_url(base_url: impl Into<String>, session_run_id: impl Into<String>) -> Self {
        Self {
            client: crate::backend::knowledge_documents::KnowledgeDocumentsClient::with_base_url(
                base_url,
            ),
            session_run_id: session_run_id.into(),
        }
    }
}

impl CreateNoteBackend for KnowledgeCreateNoteBackend {
    fn create_note<'a>(
        &'a self,
        workspace_id: &'a str,
        title: &'a str,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<CreateNoteWrite, String>> + Send + 'a>,
    > {
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
                create_if_title_absent: true,
                content_json: None, // empty body — the MT contract: "with the title and an empty body"
                schema_version: None,
                project_ref: None,
                folder_ref: None,
            };
            match client.create_document(&headers, &body).await {
                Ok(resp) => extract_document_id(&resp.document)
                    .map(|document_id| CreateNoteWrite {
                        document_id,
                        created: resp.created,
                    })
                    .ok_or_else(|| {
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

/// Context-stamped off-thread create-note result (WP-KERNEL-012 MT-057).
type CreateNoteDeliveryCell = Arc<Mutex<VecDeque<(u64, String, String, CreateNoteOutcome)>>>;

/// One-slot delivery cell for the off-thread resolver-index SEED (WP-KERNEL-012 MT-057): a Loom
/// search enumeration delivers `(document_id, title)` pairs that `drain` folds into
/// `resolver_index.add_document` so titles classify Resolved at runtime (AC-003). `Err` carries a
/// typed failure so a seed that fails does not masquerade as an authoritative empty index. The
/// resolver remains NOT READY and create-note consumers stay disabled, preventing duplicate notes.
type ResolverSeedDeliveryCell = Arc<
    Mutex<
        VecDeque<(
            u64,
            String,
            String,
            Result<Vec<(String, String)>, WikilinkError>,
        )>,
    >,
>;

/// Queue of off-thread transclusion resolutions. Every delivery is stamped with its context
/// generation plus workspace/document identity so a late old-workspace result cannot repopulate the
/// current cache. A queue also prevents concurrent transclusions overwriting one another.
type TransclusionDeliveryCell = Arc<
    Mutex<
        VecDeque<(
            u64,
            String,
            String,
            String,
            Result<LoomBlockTransclusion, WikilinkError>,
        )>,
    >,
>;

/// Completion queue for off-thread backlinks fetches. Every completion carries the generation and
/// exact workspace/document identity it was issued for. The identity stamp rejects stale empty
/// responses too: an empty response has no row whose workspace could otherwise reveal a context
/// crossover. A queue is required because explicit refreshes can overlap.
type BacklinksDeliveryCell = Arc<
    Mutex<
        VecDeque<(
            u64,
            String,
            String,
            Result<BacklinksResponse, WikilinkError>,
        )>,
    >,
>;

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
    /// Context epoch for transclusion delivery cancellation; changes only with workspace/document.
    context_generation: u64,
    /// Last cross-pane backlink invalidation revision applied by this editor.
    backlinks_invalidation_revision: u64,
    /// Save-time backlink indexing warnings keyed by the source document whose projection update
    /// failed. These remain sticky across read refreshes and are cleared only by a later successful
    /// indexing publication for the same source document.
    backlinks_index_warnings: HashMap<String, String>,
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
    pub creating_titles: HashSet<(String, String)>,
    /// Successfully created notes retained per workspace so an A -> B -> A workspace round-trip
    /// cannot expose the unresolved-create affordance again before the next backend seed completes.
    created_notes_by_workspace: HashMap<String, HashMap<String, (String, String)>>,
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
    /// True only after a successful complete resolver seed for the current workspace. Consumers that
    /// can create notes must fail closed until this is true; an empty or failed seed is not evidence
    /// that a title is unresolved.
    resolver_seed_ready: bool,
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
            context_generation: 0,
            backlinks_invalidation_revision: 0,
            backlinks_index_warnings: HashMap::new(),
            backlinks_expanded: true,
            removed_transclusions: HashSet::new(),
            // MT-057: the index starts empty (no aliases support — the backend payload has no
            // `aliases` field; AC-006). The shell populates titles from the MT-038 Loom enumeration
            // and aliases from the in-session local stub.
            resolver_index: ResolverIndex::new(),
            create_backend: None,
            creating_titles: HashSet::new(),
            created_notes_by_workspace: HashMap::new(),
            // The alias-backend gap is recognized lazily: it flips true the first time an alias path is
            // exercised while `resolver_index.aliases_supported` is false (so the banner shows only when
            // aliases are actually in play, not on every note). The shell may also set it on mount.
            alias_backend_gap: false,
            transclusion_cell: Arc::new(Mutex::new(VecDeque::new())),
            backlinks_cell: Arc::new(Mutex::new(VecDeque::new())),
            create_cell: Arc::new(Mutex::new(VecDeque::new())),
            resolver_seed_cell: Arc::new(Mutex::new(VecDeque::new())),
            resolver_seeding: false,
            resolver_seed_ready: false,
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
        self.autocomplete.reset_context(self.workspace_id.clone());
        self.document_id = document_id;
        self.backlinks_generation = self.backlinks_generation.wrapping_add(1);
        self.context_generation = self.context_generation.wrapping_add(1);
        self.backlinks = BacklinksState::Idle;
        self.transclusions.clear();
        self.removed_transclusions.clear();
    }

    /// Atomically replace the workspace/document context. Changing either identity cancels every
    /// in-flight backlinks completion and clears per-context caches. This is the production mount
    /// entry point; assigning `workspace_id` before `set_document` could otherwise accept an old
    /// empty response when the same document id exists in two workspaces.
    pub fn set_context(&mut self, workspace_id: impl Into<String>, document_id: impl Into<String>) {
        let workspace_id = workspace_id.into();
        let document_id = document_id.into();
        if workspace_id == self.workspace_id && document_id == self.document_id {
            return;
        }
        let workspace_changed = workspace_id != self.workspace_id;
        self.workspace_id = workspace_id.clone();
        self.autocomplete.reset_context(workspace_id);
        self.document_id = document_id;
        self.backlinks_generation = self.backlinks_generation.wrapping_add(1);
        self.context_generation = self.context_generation.wrapping_add(1);
        self.backlinks = BacklinksState::Idle;
        self.transclusions.clear();
        self.removed_transclusions.clear();
        if workspace_changed {
            self.resolver_index = ResolverIndex::new();
            if let Some(created) = self.created_notes_by_workspace.get(&self.workspace_id) {
                for (document_id, display_title) in created.values() {
                    self.resolver_index
                        .add_document(document_id.clone(), display_title.clone());
                }
            }
            self.alias_backend_gap = false;
            self.resolver_seeding = false;
            self.resolver_seed_ready = false;
            self.backlinks_index_warnings.clear();
            self.backlinks_invalidation_revision = 0;
        }
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
        let document_id = self.document_id.clone();
        let context_generation = self.context_generation;
        let ref_value = ref_value.to_owned();
        runtime.spawn(async move {
            let result = backend
                .resolve_transclusion(&workspace_id, &ref_value)
                .await;
            if let Ok(mut slot) = cell.lock() {
                slot.push_back((
                    context_generation,
                    workspace_id,
                    document_id,
                    ref_value,
                    result,
                ));
            }
        });
    }

    /// Trigger a backlinks fetch for the current document (on load or on an explicit refresh — NOT
    /// per frame). Bumps the generation, marks `Loading`, and spawns the fetch. A no-op when the
    /// document id is empty (no document loaded) or there is no runtime (headless seeds directly).
    pub fn refresh_backlinks(&mut self) {
        if !self.backlinks_index_warnings.is_empty() {
            return;
        }
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
        let workspace_id = self.workspace_id.clone();
        runtime.spawn(async move {
            let result = backend.list_backlinks(&document_id).await;
            if let Ok(mut slot) = cell.lock() {
                slot.push_back((generation, workspace_id, document_id, result));
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

    /// Apply the newest cross-pane save/index invalidation. Successful indexing invalidates and
    /// refetches this mounted document. A post-commit warning cancels in-flight work and exposes a
    /// typed failure until the operator retries with the existing Refresh control.
    pub fn observe_backlinks_invalidation(&mut self, ctx: &egui::Context) -> bool {
        let log = ctx.data(|data| {
            data.get_temp::<BacklinksInvalidationLog>(egui::Id::new(BACKLINKS_INVALIDATION_ID))
        });
        let Some(log) = log else {
            return false;
        };
        let workspace_revision = log
            .latest_revision_by_workspace
            .get(&self.workspace_id)
            .copied()
            .unwrap_or(0);
        let previous_warnings = self.backlinks_index_warnings.clone();
        self.backlinks_index_warnings = log
            .current_warnings
            .iter()
            .filter(|entry| (entry.0).0 == self.workspace_id)
            .map(|((_, source_document_id), warning)| (source_document_id.clone(), warning.clone()))
            .collect();
        let observed = workspace_revision != self.backlinks_invalidation_revision
            || self.backlinks_index_warnings != previous_warnings;
        self.backlinks_invalidation_revision = workspace_revision;
        if !observed {
            return false;
        }
        self.backlinks_generation = self.backlinks_generation.wrapping_add(1);
        if let Some((source_document_id, warning)) = self.backlinks_index_warnings.iter().next() {
            self.backlinks = BacklinksState::Failed(WikilinkError::ServerError(format!(
                "document '{source_document_id}' saved, but backlink indexing needs attention: {warning}"
            )));
        } else {
            self.backlinks = BacklinksState::Idle;
            self.refresh_backlinks();
        }
        true
    }

    /// Drain any off-thread transclusion/backlinks results delivered since the last frame into the
    /// caches. A backlinks result whose generation no longer matches `backlinks_generation` is
    /// DROPPED (MC-004). Returns true when something was applied (the caller can request a repaint).
    pub fn drain(&mut self) -> bool {
        let mut applied = false;
        if let Ok(mut slot) = self.transclusion_cell.lock() {
            while let Some((generation, workspace_id, document_id, ref_value, result)) =
                slot.pop_front()
            {
                if generation != self.context_generation
                    || workspace_id != self.workspace_id
                    || document_id != self.document_id
                {
                    continue;
                }
                let state = match result {
                    Ok(t) if t.workspace_id != self.workspace_id || t.block_id != ref_value => {
                        TransclusionState::Failed(WikilinkError::ServerError(format!(
                            "transclusion response identity mismatch: requested workspace '{}' block '{}', received workspace '{}' block '{}'",
                            self.workspace_id, ref_value, t.workspace_id, t.block_id
                        )))
                    }
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
            while let Some((generation, workspace_id, document_id, result)) = slot.pop_front() {
                if generation == self.backlinks_generation
                    && workspace_id == self.workspace_id
                    && document_id == self.document_id
                {
                    self.backlinks = match result {
                        Ok(resp) if resp.source_document_id != self.document_id => {
                            BacklinksState::Failed(WikilinkError::ServerError(format!(
                                "backlinks response document mismatch: requested '{}', received '{}'",
                                self.document_id, resp.source_document_id
                            )))
                        }
                        Ok(resp)
                            if resp
                                .backlinks
                                .iter()
                                .any(|row| row.workspace_id != self.workspace_id) =>
                        {
                            BacklinksState::Failed(WikilinkError::ServerError(format!(
                                "backlinks response crossed workspace boundary for document '{}'",
                                self.document_id
                            )))
                        }
                        Ok(resp) => BacklinksState::Loaded(resp.backlinks),
                        Err(e) => BacklinksState::Failed(e),
                    };
                    applied = true;
                }
                // else: a stale (older-generation) backlinks response landed late -> dropped (MC-004).
            }
        }
        // WP-KERNEL-012 MT-057: fold a delivered resolver-index SEED (a Loom search enumeration) into
        // the index so a `[[Title]]` classifies Resolved at runtime (AC-003). A failed seed leaves the
        // resolver NOT READY, so create-note consumers stay disabled instead of risking duplicates.
        if let Ok(mut slot) = self.resolver_seed_cell.lock() {
            while let Some((_generation, workspace_id, _document_id, result)) = slot.pop_front() {
                if workspace_id == self.workspace_id {
                    self.resolver_seeding = false;
                    if let Ok(pairs) = result {
                        self.resolver_seed_ready = true;
                        for (document_id, title) in pairs {
                            self.resolver_index.add_document(document_id, title);
                        }
                    }
                    applied = true;
                }
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

    /// WP-KERNEL-012 MT-045 (wave-2 remediation): detect a CYCLIC transclusion chain starting at
    /// `start`, walking the LIVE resolution cache with the product
    /// [`crate::rich_editor::wikilinks::transclusion_resolver::resolve_transclusion_chain`] (the
    /// resolver the LR-05 perf proof drives — one algorithm, not a test-only fork). Each hop is the
    /// FIRST `loomTransclusion` target embedded in the previous hop's resolved `content_json`.
    ///
    /// Returns `Some(repeated_block_id)` when the chain revisits an id (the render path then shows a
    /// visible cycle indicator instead of the read-through preview), `None` otherwise. The walk is
    /// evidence-based and fail-closed the honest way around:
    /// - a hop that is NOT yet resolved in the cache ends the walk for THIS frame (`None` — never a
    ///   cycle claim without evidence); the missing hop is handed to [`Self::ensure_transclusion`] so
    ///   the walk deepens on later frames once the fetch lands (cached, storm-safe, headless no-op);
    /// - a hop whose content embeds no transclusion ends the chain cleanly;
    /// - a non-cyclic over-deep chain trips the resolver's depth bound and reports no cycle (the
    ///   bound exists so a runaway chain can never spin the frame).
    pub fn detect_transclusion_cycle(&mut self, start: &str) -> Option<String> {
        use crate::rich_editor::wikilinks::transclusion_resolver::{
            next_transclusion_ref, resolve_transclusion_chain, TransclusionResolveError,
            MAX_TRANSCLUSION_CHAIN_DEPTH,
        };

        // The walk reads ONLY the cache; a next hop that is not cached yet is recorded so the fetch
        // can be scheduled AFTER the walk (the closure borrows the cache immutably).
        let mut missing_hop: Option<String> = None;
        let transclusions = &self.transclusions;
        let result = resolve_transclusion_chain(start, MAX_TRANSCLUSION_CHAIN_DEPTH, |id| {
            let next = match transclusions.get(id) {
                Some(TransclusionState::Resolved(t)) => {
                    t.content_json.as_ref().and_then(next_transclusion_ref)?
                }
                // Unresolved/Failed/Resolving/uncached hop: no onward evidence — clean end.
                _ => return None,
            };
            if transclusions.contains_key(&next) {
                // Cached (any state): continue — the resolver's visited set is what flags a repeat.
                Some(next)
            } else {
                // Not fetched yet: stop THIS frame's walk and schedule the hop so a deeper chain
                // becomes verifiable on later frames.
                missing_hop = Some(next);
                None
            }
        });
        if let Some(hop) = missing_hop {
            self.ensure_transclusion(&hop);
        }
        match result {
            Err(TransclusionResolveError::CycleDetected { at }) => Some(at),
            // A clean end or the depth bound: no cycle EVIDENCE — render normally.
            Ok(_) | Err(TransclusionResolveError::DepthExceeded { .. }) => None,
        }
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
        self.resolver_seed_ready = false;
        let backend = Arc::clone(&self.backend);
        let cell = Arc::clone(&self.resolver_seed_cell);
        let workspace_id = self.workspace_id.clone();
        let document_id = self.document_id.clone();
        let context_generation = self.context_generation;
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
                slot.push_back((context_generation, workspace_id, document_id, result));
            }
        });
    }

    /// TEST SEAM: stage a resolver-index seed delivery into the cell (so a headless test drives
    /// [`Self::drain`]'s seed-fold without a tokio runtime / real backend). Mirrors the other
    /// `stage_*` seams.
    #[doc(hidden)]
    pub fn stage_resolver_seed(&mut self, pairs: Vec<(String, String)>) {
        self.resolver_seeding = true;
        self.resolver_seed_cell.lock().unwrap().push_back((
            self.context_generation,
            self.workspace_id.clone(),
            self.document_id.clone(),
            Ok(pairs),
        ));
    }

    /// True while a resolver-index seed search is in flight (the idempotency guard; read by a test that
    /// proves a re-mount does not re-issue a seed while one is already running).
    pub fn is_seeding_resolver_index(&self) -> bool {
        self.resolver_seeding
    }

    /// Whether the current workspace's resolver seed completed successfully. A successful empty seed
    /// is ready; transport/server failure remains not-ready and may be retried.
    pub fn is_resolver_index_ready(&self) -> bool {
        self.resolver_seed_ready
    }

    /// True when a create POST is already in flight for `title` (normalized) — the affordance is
    /// DISABLED while true so a double-click cannot POST twice (RISK-001 / MC-001). The egui click
    /// handler checks this before emitting/dispatching.
    pub fn is_creating(&self, title: &str) -> bool {
        self.creating_titles
            .contains(&(self.workspace_id.clone(), normalize_target(title)))
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
        let guard_key = (self.workspace_id.clone(), normalized.clone());
        if self.creating_titles.contains(&guard_key) {
            return false;
        }
        let (Some(backend), Some(runtime)) = (self.create_backend.clone(), self.runtime.clone())
        else {
            return false; // headless / unwired: the test stages the outcome directly.
        };
        self.creating_titles.insert(guard_key);
        let workspace_id = self.workspace_id.clone();
        let document_id = self.document_id.clone();
        let context_generation = self.context_generation;
        let cell = Arc::clone(&self.create_cell);
        runtime.spawn(async move {
            let result = backend.create_note(&workspace_id, &display_title).await;
            let outcome = match result {
                Ok(write) => CreateNoteOutcome::Created {
                    normalized_title: normalized,
                    display_title,
                    document_id: write.document_id,
                    created: write.created,
                },
                Err(reason) => CreateNoteOutcome::Failed {
                    normalized_title: normalized,
                    reason,
                },
            };
            if let Ok(mut slot) = cell.lock() {
                slot.push_back((context_generation, workspace_id, document_id, outcome));
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
    fn apply_create_outcome(
        &mut self,
        workspace_id: &str,
        outcome: CreateNoteOutcome,
    ) -> CreateNoteOutcome {
        match &outcome {
            CreateNoteOutcome::Created {
                normalized_title,
                display_title,
                document_id,
                ..
            } => {
                self.creating_titles
                    .remove(&(workspace_id.to_owned(), normalized_title.clone()));
                self.created_notes_by_workspace
                    .entry(workspace_id.to_owned())
                    .or_default()
                    .insert(
                        normalized_title.clone(),
                        (document_id.clone(), display_title.clone()),
                    );
                if workspace_id == self.workspace_id {
                    // The new note is now resolvable by its title (live, no reload — AC-002).
                    self.resolver_index
                        .add_document(document_id.clone(), display_title.clone());
                }
            }
            CreateNoteOutcome::Failed {
                normalized_title, ..
            } => {
                self.creating_titles
                    .remove(&(workspace_id.to_owned(), normalized_title.clone()));
            }
        }
        outcome
    }

    fn pop_create_completion(&mut self) -> Option<CreateNoteCompletion> {
        let (context_generation, workspace_id, document_id, outcome) = self
            .create_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.pop_front())?;
        let outcome = self.apply_create_outcome(&workspace_id, outcome);
        Some(CreateNoteCompletion {
            context_generation,
            workspace_id,
            document_id,
            outcome,
        })
    }

    /// Drain the next completion for shell ownership without filtering it through the widget's
    /// current context. This is the production path for hidden/document-switched panes.
    pub fn drain_create_for_host(&mut self) -> Option<CreateNoteCompletion> {
        self.pop_create_completion()
    }

    /// Whether this completion originated in the context currently mounted by this runtime.
    pub fn create_completion_matches_current(&self, completion: &CreateNoteCompletion) -> bool {
        completion.context_generation == self.context_generation
            && completion.workspace_id == self.workspace_id
            && completion.document_id == self.document_id
    }

    /// Drain a delivered create-note outcome (if any) into the index + in-flight guard, returning it
    /// so the widget can rewrite the originating mark Unresolved -> Resolved (AC-002) or surface a
    /// failure. Separate from [`Self::drain`] because the create outcome must flow back to the WIDGET
    /// (to mutate the document mark), whereas transclusion/backlinks land entirely inside the runtime.
    pub fn drain_create(&mut self) -> Option<CreateNoteOutcome> {
        loop {
            let completion = self.pop_create_completion()?;
            if self.create_completion_matches_current(&completion) {
                return Some(completion.outcome);
            }
        }
    }

    /// TEST SEAM: stage a create-note outcome into the create cell (so a headless test drives
    /// [`Self::drain_create`] without a tokio runtime / real backend).
    #[doc(hidden)]
    pub fn stage_create(&self, outcome: CreateNoteOutcome) {
        self.create_cell.lock().unwrap().push_back((
            self.context_generation,
            self.workspace_id.clone(),
            self.document_id.clone(),
            outcome,
        ));
    }

    /// TEST SEAM: directly mark a title in-flight (so a test can prove the double-dispatch guard
    /// without a runtime).
    #[cfg(test)]
    pub fn mark_creating(&mut self, title: &str) {
        self.creating_titles
            .insert((self.workspace_id.clone(), normalize_target(title)));
    }

    // ── Test seams (headless: stage a delivery without a tokio runtime) ──────────────────────────

    /// Stage a transclusion delivery into the cell (test seam).
    #[cfg(test)]
    pub fn stage_transclusion(
        &self,
        ref_value: &str,
        result: Result<LoomBlockTransclusion, WikilinkError>,
    ) {
        self.transclusion_cell.lock().unwrap().push_back((
            self.context_generation,
            self.workspace_id.clone(),
            self.document_id.clone(),
            ref_value.to_owned(),
            result,
        ));
    }

    /// Stage a backlinks delivery into the cell tagged with `generation` (test seam).
    #[cfg(test)]
    pub fn stage_backlinks(
        &self,
        generation: u64,
        result: Result<BacklinksResponse, WikilinkError>,
    ) {
        self.backlinks_cell.lock().unwrap().push_back((
            generation,
            self.workspace_id.clone(),
            self.document_id.clone(),
            result,
        ));
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
    fn loom_address_backlinks_queue_preserves_newest_completion_before_stale_delivery() {
        let mut rt = rt();
        rt.set_document("DOC-A");
        let stale_generation = rt.backlinks_generation;
        rt.set_document("DOC-B");
        let current_generation = rt.backlinks_generation;
        rt.backlinks = BacklinksState::Loading;

        // The current response completes first, then the older request completes before the next
        // frame drains. A one-slot cell would lose the current response and leave Loading forever.
        rt.stage_backlinks(
            current_generation,
            Ok(BacklinksResponse {
                source_document_id: "DOC-B".into(),
                backlinks: vec![backlink("CURRENT")],
            }),
        );
        rt.stage_backlinks(
            stale_generation,
            Ok(BacklinksResponse {
                source_document_id: "DOC-A".into(),
                backlinks: vec![backlink("STALE")],
            }),
        );

        assert!(rt.drain(), "the current queued completion must be applied");
        match &rt.backlinks {
            BacklinksState::Loaded(rows) => {
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0].source_document_id, "CURRENT");
            }
            other => panic!("expected current Loaded response, got {other:?}"),
        }
    }

    #[test]
    fn loom_address_backlinks_response_document_mismatch_fails_closed() {
        let mut rt = rt();
        rt.set_document("DOC-B");
        rt.backlinks = BacklinksState::Loading;
        rt.stage_backlinks(
            rt.backlinks_generation,
            Ok(BacklinksResponse {
                source_document_id: "DOC-C".into(),
                backlinks: vec![],
            }),
        );
        assert!(rt.drain());
        assert!(
            matches!(
                rt.backlinks,
                BacklinksState::Failed(WikilinkError::ServerError(_))
            ),
            "a same-generation response for another document must not render"
        );
    }

    #[test]
    fn loom_address_backlinks_response_workspace_mismatch_fails_closed() {
        let mut rt = rt();
        rt.set_document("DOC-B");
        rt.backlinks = BacklinksState::Loading;
        let mut wrong_workspace = backlink("DOC-A");
        wrong_workspace.workspace_id = "other-workspace".into();
        rt.stage_backlinks(
            rt.backlinks_generation,
            Ok(BacklinksResponse {
                source_document_id: "DOC-B".into(),
                backlinks: vec![wrong_workspace],
            }),
        );
        assert!(rt.drain());
        assert!(
            matches!(
                rt.backlinks,
                BacklinksState::Failed(WikilinkError::ServerError(_))
            ),
            "a row from another workspace must not render"
        );
    }

    #[test]
    fn loom_address_same_document_workspace_swap_drops_stale_empty_response() {
        let mut rt = rt();
        rt.set_context("workspace-old", "DOC-B");
        let old_generation = rt.backlinks_generation;
        rt.stage_backlinks(
            old_generation,
            Ok(BacklinksResponse {
                source_document_id: "DOC-B".into(),
                backlinks: vec![],
            }),
        );

        rt.set_context("workspace-current", "DOC-B");
        let current_generation = rt.backlinks_generation;
        let mut current = backlink("CURRENT");
        current.workspace_id = "workspace-current".into();
        rt.stage_backlinks(
            current_generation,
            Ok(BacklinksResponse {
                source_document_id: "DOC-B".into(),
                backlinks: vec![current],
            }),
        );

        assert!(rt.drain());
        match &rt.backlinks {
            BacklinksState::Loaded(rows) => {
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0].source_document_id, "CURRENT");
            }
            other => panic!("expected current workspace response, got {other:?}"),
        }
    }

    #[test]
    fn workspace_swap_drops_stale_transclusion_delivery() {
        let mut rt = rt();
        rt.set_context("workspace-old", "DOC-B");
        let mut stale = resolved_transclusion("BLOCK-SHARED");
        stale.workspace_id = "workspace-old".into();
        rt.stage_transclusion("BLOCK-SHARED", Ok(stale));

        rt.set_context("workspace-current", "DOC-B");
        assert!(!rt.drain(), "old-workspace transclusion must be dropped");
        assert!(!rt.transclusions.contains_key("BLOCK-SHARED"));

        let mut current = resolved_transclusion("BLOCK-SHARED");
        current.workspace_id = "workspace-current".into();
        rt.stage_transclusion("BLOCK-SHARED", Ok(current));
        assert!(rt.drain());
        match rt.transclusions.get("BLOCK-SHARED") {
            Some(TransclusionState::Resolved(value)) => {
                assert_eq!(value.workspace_id, "workspace-current");
            }
            other => panic!("expected current transclusion, got {other:?}"),
        }
    }

    #[test]
    fn workspace_swap_drops_stale_resolver_seed_and_create_delivery() {
        let mut rt = rt();
        rt.set_context("workspace-old", "DOC-OLD");
        rt.stage_resolver_seed(vec![("DOC-OLD-TARGET".into(), "Old title".into())]);
        rt.stage_create(CreateNoteOutcome::Created {
            normalized_title: "old-created".into(),
            display_title: "Old Created".into(),
            document_id: "DOC-OLD-CREATED".into(),
            created: true,
        });

        rt.set_context("workspace-current", "DOC-CURRENT");
        assert!(!rt.drain(), "old resolver seed must be dropped");
        assert!(
            rt.drain_create().is_none(),
            "old create result must be dropped"
        );
        assert_eq!(rt.resolver_index.title_count(), 0);
        assert!(rt.creating_titles.is_empty());
    }

    #[test]
    fn backlinks_save_invalidation_fans_out_to_every_mounted_runtime() {
        let ctx = egui::Context::default();
        let mut target_b = rt();
        target_b.set_document("DOC-B");
        target_b.backlinks = BacklinksState::Loaded(vec![backlink("DOC-A")]);
        let mut target_c = rt();
        target_c.set_document("DOC-C");
        target_c.backlinks = BacklinksState::Loaded(vec![backlink("DOC-X")]);

        publish_backlinks_invalidation(&ctx, "ws", "DOC-A", None);
        assert!(target_b.observe_backlinks_invalidation(&ctx));
        assert!(target_c.observe_backlinks_invalidation(&ctx));
        assert!(matches!(target_b.backlinks, BacklinksState::Idle));
        assert!(matches!(target_c.backlinks, BacklinksState::Idle));
    }

    #[test]
    fn backlinks_post_commit_index_warning_is_visible_in_every_mounted_runtime() {
        let ctx = egui::Context::default();
        let mut target_b = rt();
        target_b.set_document("DOC-B");
        let mut target_c = rt();
        target_c.set_document("DOC-C");

        publish_backlinks_invalidation(&ctx, "ws", "DOC-A", Some("index unavailable".into()));
        assert!(target_b.observe_backlinks_invalidation(&ctx));
        assert!(target_c.observe_backlinks_invalidation(&ctx));
        for state in [&target_b.backlinks, &target_c.backlinks] {
            match state {
                BacklinksState::Failed(WikilinkError::ServerError(message)) => {
                    assert!(message.contains("index unavailable"));
                }
                other => panic!("index warning must be visible, got {other:?}"),
            }
        }
    }

    #[test]
    fn backlink_index_warning_is_queued_sticky_and_cleared_only_by_same_source_success() {
        let ctx = egui::Context::default();
        let mut target = rt();
        target.set_document("DOC-TARGET");

        publish_backlinks_invalidation(&ctx, "ws", "DOC-A", Some("index unavailable".into()));
        publish_backlinks_invalidation(&ctx, "ws", "DOC-B", None);
        assert!(target.observe_backlinks_invalidation(&ctx));
        assert!(matches!(
            &target.backlinks,
            BacklinksState::Failed(WikilinkError::ServerError(message))
                if message.contains("DOC-A") && message.contains("index unavailable")
        ));

        target.refresh_backlinks();
        assert!(
            matches!(target.backlinks, BacklinksState::Failed(_)),
            "a read refresh cannot repair or hide a failed save-time index update"
        );

        publish_backlinks_invalidation(&ctx, "ws", "DOC-A", None);
        assert!(target.observe_backlinks_invalidation(&ctx));
        assert!(matches!(target.backlinks, BacklinksState::Idle));
    }

    #[test]
    fn concurrent_create_deliveries_are_all_preserved() {
        let mut rt = rt();
        rt.set_context("ws", "DOC-A");
        rt.mark_creating("Alpha");
        rt.mark_creating("Beta");
        rt.stage_create(CreateNoteOutcome::Created {
            normalized_title: normalize_target("Alpha"),
            display_title: "Alpha".into(),
            document_id: "DOC-ALPHA".into(),
            created: true,
        });
        rt.stage_create(CreateNoteOutcome::Created {
            normalized_title: normalize_target("Beta"),
            display_title: "Beta".into(),
            document_id: "DOC-BETA".into(),
            created: false,
        });

        assert!(
            matches!(rt.drain_create(), Some(CreateNoteOutcome::Created { document_id, created: true, .. }) if document_id == "DOC-ALPHA")
        );
        assert!(
            matches!(rt.drain_create(), Some(CreateNoteOutcome::Created { document_id, created: false, .. }) if document_id == "DOC-BETA")
        );
        assert!(rt.drain_create().is_none());
        assert!(rt.creating_titles.is_empty());
        assert_eq!(rt.resolver_index.title_count(), 2);
    }

    #[test]
    fn same_workspace_document_switch_preserves_workspace_create_and_seed_guards() {
        let mut rt = rt();
        rt.set_context("ws", "DOC-OLD");
        rt.mark_creating("Shared");
        rt.resolver_seeding = true;
        let old_generation = rt.context_generation;

        rt.set_context("ws", "DOC-CURRENT");
        assert!(rt.is_creating("Shared"));
        assert!(rt.is_seeding_resolver_index());
        assert!(
            !rt.dispatch_create_note("Shared"),
            "document navigation must not re-enable a duplicate workspace/title create"
        );
        rt.create_cell.lock().unwrap().push_back((
            old_generation,
            "ws".into(),
            "DOC-OLD".into(),
            CreateNoteOutcome::Created {
                normalized_title: normalize_target("Shared"),
                display_title: "Shared".into(),
                document_id: "DOC-SHARED".into(),
                created: false,
            },
        ));
        rt.resolver_seed_cell.lock().unwrap().push_back((
            old_generation,
            "ws".into(),
            "DOC-OLD".into(),
            Ok(vec![("DOC-TARGET".into(), "Target".into())]),
        ));

        assert!(rt.drain());
        assert!(!rt.is_seeding_resolver_index());
        assert_eq!(rt.resolver_index.title_count(), 1);
        assert!(
            rt.drain_create().is_none(),
            "the old document receives no UI outcome"
        );
        assert!(!rt.is_creating("Shared"));
        assert_eq!(rt.resolver_index.title_count(), 2);
    }

    #[test]
    fn backlink_invalidation_is_workspace_scoped_and_history_is_bounded() {
        let ctx = egui::Context::default();
        let mut target = rt();
        target.set_context("workspace-a", "DOC-TARGET");
        publish_backlinks_invalidation(
            &ctx,
            "workspace-b",
            "DOC-SOURCE",
            Some("other workspace failure".into()),
        );
        assert!(!target.observe_backlinks_invalidation(&ctx));
        assert!(target.backlinks_index_warnings.is_empty());
        target.set_context("workspace-b", "DOC-TARGET");
        assert!(
            target.observe_backlinks_invalidation(&ctx),
            "workspace selection hydrates its existing warning snapshot without a new publication"
        );
        assert_eq!(target.backlinks_index_warnings.len(), 1);
        target.set_context("workspace-a", "DOC-TARGET");

        publish_backlinks_invalidation(
            &ctx,
            "workspace-a",
            "DOC-SOURCE",
            Some("current workspace failure".into()),
        );
        assert!(target.observe_backlinks_invalidation(&ctx));
        assert_eq!(target.backlinks_index_warnings.len(), 1);
        target.set_context("workspace-b", "DOC-TARGET");
        assert!(target.backlinks_index_warnings.is_empty());

        for index in 0..(BACKLINKS_INVALIDATION_WINDOW + 32) {
            publish_backlinks_invalidation(&ctx, "workspace-b", format!("DOC-{index}"), None);
        }
        let log = ctx
            .data(|data| {
                data.get_temp::<BacklinksInvalidationLog>(egui::Id::new(BACKLINKS_INVALIDATION_ID))
            })
            .expect("invalidation log exists");
        assert_eq!(log.events.len(), BACKLINKS_INVALIDATION_WINDOW);
    }

    #[test]
    fn successful_workspace_invalidation_survives_event_window_eviction() {
        let ctx = egui::Context::default();
        let mut target = rt();
        target.set_context("workspace-a", "DOC-TARGET");
        target.backlinks = BacklinksState::Loaded(vec![backlink("DOC-STALE")]);
        publish_backlinks_invalidation(&ctx, "workspace-a", "DOC-SOURCE", None);
        for index in 0..(BACKLINKS_INVALIDATION_WINDOW + 8) {
            publish_backlinks_invalidation(&ctx, "workspace-b", format!("DOC-{index}"), None);
        }
        assert!(target.observe_backlinks_invalidation(&ctx));
        assert!(
            matches!(target.backlinks, BacklinksState::Idle),
            "the compact per-workspace revision refreshes stale rows after the event was evicted"
        );
    }

    #[test]
    fn workspace_round_trip_never_reenables_duplicate_create() {
        use crate::rich_editor::wikilinks::resolver::{resolve_wikilink, WikilinkResolution};

        let mut rt = rt();
        rt.set_context("workspace-a", "DOC-A");
        rt.mark_creating("Shared");
        let generation = rt.context_generation;
        rt.set_context("workspace-b", "DOC-B");
        assert!(
            !rt.is_creating("Shared"),
            "workspace B has an independent title guard"
        );
        rt.set_context("workspace-a", "DOC-A");
        assert!(
            rt.is_creating("Shared"),
            "A -> B -> A retains the original A create guard"
        );
        assert!(!rt.dispatch_create_note("Shared"));

        rt.set_context("workspace-b", "DOC-B");
        rt.create_cell.lock().unwrap().push_back((
            generation,
            "workspace-a".into(),
            "DOC-A".into(),
            CreateNoteOutcome::Created {
                normalized_title: normalize_target("Shared"),
                display_title: "Shared".into(),
                document_id: "DOC-SHARED".into(),
                created: false,
            },
        ));
        assert!(rt.drain_create().is_none());
        rt.set_context("workspace-a", "DOC-A");
        assert!(!rt.is_creating("Shared"));
        assert!(matches!(
            resolve_wikilink(&rt.resolver_index, "Shared"),
            WikilinkResolution::Resolved { document_id, .. } if document_id == "DOC-SHARED"
        ));
    }

    #[test]
    fn transclusion_response_identity_mismatch_fails_closed() {
        let mut rt = rt();
        rt.set_context("workspace-current", "DOC-A");
        let mut mismatched = resolved_transclusion("BLOCK-OTHER");
        mismatched.workspace_id = "workspace-current".into();
        rt.stage_transclusion("BLOCK-REQUESTED", Ok(mismatched));
        assert!(rt.drain());
        assert!(matches!(
            rt.transclusions.get("BLOCK-REQUESTED"),
            Some(TransclusionState::Failed(WikilinkError::ServerError(message)))
                if message.contains("identity mismatch")
        ));
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

    /// A resolved transclusion whose content embeds a `loomTransclusion` pointing at `next` (the
    /// chain-hop shape [`WikilinkRuntime::detect_transclusion_cycle`] walks).
    fn chained_transclusion(block_id: &str, next: &str) -> LoomBlockTransclusion {
        let mut t = resolved_transclusion(block_id);
        t.content_json = Some(serde_json::json!({"type":"doc","content":[
            {"type":"paragraph","content":[
                {"type":"loomTransclusion","attrs":{"refValue": next}}
            ]}
        ]}));
        t
    }

    #[test]
    fn detect_transclusion_cycle_flags_a_seeded_two_hop_cycle_mt045() {
        // MT-045 (wave-2): BLK-A embeds BLK-B and BLK-B embeds BLK-A — the cache walk must report
        // the repeated id instead of spinning, using the PRODUCT resolver.
        let mut rt = rt();
        rt.transclusions.insert(
            "BLK-A".into(),
            TransclusionState::Resolved(chained_transclusion("BLK-A", "BLK-B")),
        );
        rt.transclusions.insert(
            "BLK-B".into(),
            TransclusionState::Resolved(chained_transclusion("BLK-B", "BLK-A")),
        );
        assert_eq!(
            rt.detect_transclusion_cycle("BLK-A").as_deref(),
            Some("BLK-A"),
            "the A->B->A walk flags the repeated id (BLK-A)"
        );
        // A SELF-cycle (A embeds A) is caught with no second hop needed.
        rt.transclusions.insert(
            "BLK-SELF".into(),
            TransclusionState::Resolved(chained_transclusion("BLK-SELF", "BLK-SELF")),
        );
        assert_eq!(
            rt.detect_transclusion_cycle("BLK-SELF").as_deref(),
            Some("BLK-SELF")
        );
    }

    #[test]
    fn detect_transclusion_cycle_clean_chain_and_unfetched_hop_report_no_cycle() {
        let mut rt = rt();
        // A clean chain: BLK-1 embeds BLK-2; BLK-2's content embeds nothing -> no cycle.
        rt.transclusions.insert(
            "BLK-1".into(),
            TransclusionState::Resolved(chained_transclusion("BLK-1", "BLK-2")),
        );
        rt.transclusions.insert(
            "BLK-2".into(),
            TransclusionState::Resolved(resolved_transclusion("BLK-2")),
        );
        assert_eq!(rt.detect_transclusion_cycle("BLK-1"), None);

        // An UNFETCHED next hop is never claimed as a cycle (fail-closed the honest way): the walk
        // ends this frame and the missing hop is scheduled (headless: marked Resolving, no spin).
        rt.transclusions.insert(
            "BLK-3".into(),
            TransclusionState::Resolved(chained_transclusion("BLK-3", "BLK-UNFETCHED")),
        );
        assert_eq!(rt.detect_transclusion_cycle("BLK-3"), None);
        assert!(
            matches!(
                rt.transclusions.get("BLK-UNFETCHED"),
                Some(TransclusionState::Resolving)
            ),
            "the missing hop was handed to ensure_transclusion so later frames can deepen the walk"
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
            created: true,
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
        assert!(
            rt.is_resolver_index_ready(),
            "a successful seed, including an empty one, authorizes unresolved classification"
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
    fn failed_resolver_seed_stays_not_ready_and_create_consumers_fail_closed() {
        let mut rt = rt();
        rt.resolver_seeding = true;
        rt.resolver_seed_cell.lock().unwrap().push_back((
            rt.context_generation,
            rt.workspace_id.clone(),
            rt.document_id.clone(),
            Err(WikilinkError::ServerError("resolver unavailable".into())),
        ));
        assert!(rt.drain());
        assert!(!rt.is_seeding_resolver_index());
        assert!(!rt.is_resolver_index_ready());
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
