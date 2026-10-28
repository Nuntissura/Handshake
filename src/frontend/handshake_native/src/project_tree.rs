//! Left-rail PROJECT TREE for the native work surface (WP-KERNEL-011 MT-014, section 2).
//!
//! ## What this provides
//!
//! A collapsible tree of the active project's **documents** and **canvases**, rendered inside the
//! left activity rail ([`crate::left_rail`]). It replaces (and upgrades) the React
//! `WorkspaceSidebar` document/canvas list (`app/src/components/WorkspaceSidebar.tsx` lines
//! 426-465) with a real, paper-strip tree: each row's label background is only as wide as the label
//! text (variable width, NOT a full-row fill), with the label right-aligned within its strip and a
//! folder chevron at the left edge of each group header. Clicking a document/canvas row reports an
//! [`ProjectTreeEvent`] the caller turns into an "open in the active pane" action.
//!
//! ## Data sources (reuse the existing backend over HTTP, never re-implement it)
//!
//! - documents: `GET /knowledge/documents?workspace_id={id}` (RichDocument authority id + token);
//! - canvases:  `GET /workspaces/{id}/canvases`  (React `api.ts` `listCanvases`).
//!
//! Loading is asynchronous: a `tokio` task per `workspace_id` change sends its result back over an
//! `std::sync::mpsc` channel that [`ProjectTree::poll`] drains each frame with `try_recv()` — the
//! render thread is never blocked (red-team RISK: a slow/absent backend must not stall the shell).
//! A monotonic `load_id` is stamped on every request and the result is discarded unless it matches
//! the current counter, so a workspace switch mid-flight can never let the OLD workspace's documents
//! overwrite the NEW workspace's list (red-team RISK: stale-result clobber).
//!
//! ## Stable AccessKit ids (out-of-process steering)
//!
//! The tree CONTAINER gets a fixed `NodeId` ([`PROJECT_TREE_NODE_ID`] = 89, `Role::Tree`). The two
//! group HEADERS (Documents / Canvases) and each leaf ROW are DYNAMIC (their count varies as the
//! project's content changes), so each derives its `egui::Id` from its stable author_id STRING
//! (`project-tree.doc.{slug}` / `project-tree.canvas.{slug}` / `project-tree.group.{documents|
//! canvases}`) via [`egui::Id::new`] — the same dynamic-count pattern the per-pane tabs and project
//! tabs use. The leaf slug strips a content id to `[a-z0-9-]` so a slash/space/UTF-8 id can never
//! produce a malformed author_id (red-team RISK: invalid id chars). Each leaf is a `Role::TreeItem`
//! with `Action::Click`; each group header is a `Role::TreeItem` with `Action::Click` (expand/
//! collapse). The container Tree node is non-interactive.

use std::sync::mpsc::{Receiver, Sender};

use egui::accesskit;

use crate::backend_client::BACKEND_BASE_URL;
use crate::context_menu::ContextMenu;
use crate::context_menu_surfaces::{
    explorer_action_for_id, explorer_context_items, ExplorerMenuAction, ExplorerRowKind,
};
use crate::error::AppError;

/// Fixed AccessKit/egui `NodeId` for the project-tree CONTAINER node (`Role::Tree`).
///
/// Occupies the FRESH band slot 89 — disjoint from every other declared identity: theme toggle (10),
/// chrome (20/21), dividers (30/31), scrollbar rails (40..43), project-tab strip (50), module
/// buttons (51..56), tab-bar containers (60..63), merge-back (64..67), pane locks (70..73), pane
/// titles (74..77), the left-rail fixed band (80..88), the quick-links container (90), and the pane
/// id space (>= 100). The collision test in `accessibility::registry` proves the disjointness.
pub const PROJECT_TREE_NODE_ID: u64 = 89;

/// Stable out-of-process author_id for the project-tree container.
pub const PROJECT_TREE_AUTHOR_ID: &str = "project-tree";

/// Fixed AccessKit/egui `NodeId` for the BOOKMARKS group CONTAINER node (`Role::Tree`, MT-014 FIX-A).
///
/// Occupies the FRESH band slot 91 — above the quick-links container (90) and strictly below the pane
/// id base (100). The Bookmarks group renders below the Documents/Canvases groups inside the Files
/// panel; its container carries this stable id so an out-of-process model can address the bookmarks
/// region directly. Individual bookmark ROWS are DYNAMIC (count varies with pins) and derive their
/// `egui::Id` from `project-tree.bookmark.{slug}`. The collision test in `accessibility::registry`
/// proves disjointness.
pub const BOOKMARKS_NODE_ID: u64 = 91;

/// Stable out-of-process author_id for the bookmarks-group container.
pub const BOOKMARKS_AUTHOR_ID: &str = "project-tree.bookmarks";

/// One project RichDocument, reduced to the fields the tree and optimistic rename need.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentSummary {
    pub id: String,
    pub title: String,
    /// Backend concurrency token captured with the explorer row.
    pub updated_at: Option<String>,
}

impl DocumentSummary {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            updated_at: None,
        }
    }
}

/// One project canvas, reduced to the two fields the tree needs (mirrors the React `CanvasSummary`
/// shape mapped from `GET /workspaces/{id}/canvases`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasSummary {
    pub id: String,
    pub title: String,
    /// Backend concurrency token used by canvas title rename. `None` only for directly-seeded legacy
    /// tests; production list responses always provide it.
    pub updated_at: Option<String>,
}

impl CanvasSummary {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            updated_at: None,
        }
    }
}

/// One BOOKMARK (pinned Loom block) shown in the Bookmarks group (MT-014 FIX-A). Mirrors the React
/// `WorkspaceSidebar` bookmark rows backed by `GET /workspaces/{id}/loom/views/pins`
/// (`LoomViewResponse::Pins { blocks }`). The fields are the minimum the row needs to render + open:
/// the block id (the open target when there is no document), the display title, the kind badge text
/// (`document` / `file` / `tag_hub` / `journal` / `block`, computed exactly like the React
/// `bookmarkKind()` helper), and the optional owning `document_id` (when present, the bookmark opens as
/// that document, matching React `handleOpenBookmark`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookmarkSummary {
    /// The pinned block id (`LoomBlock.block_id`); the open target when `document_id` is `None`.
    pub block_id: String,
    /// Display title (`LoomBlock.title` -> `original_filename` -> `block_id`, like the React `blockTitle`).
    pub title: String,
    /// Kind badge text (the React `bookmarkKind()` result): `document` | `file` | `tag_hub` |
    /// `journal` | `block`.
    pub kind: String,
    /// The owning document id when this pin is a document pin; opening it opens that document.
    pub document_id: Option<String>,
    /// Loom block concurrency token captured with the pinned row.
    pub updated_at: Option<String>,
}

impl BookmarkSummary {
    pub fn new(
        block_id: impl Into<String>,
        title: impl Into<String>,
        kind: impl Into<String>,
        document_id: Option<String>,
    ) -> Self {
        Self {
            block_id: block_id.into(),
            title: title.into(),
            kind: kind.into(),
            document_id,
            updated_at: None,
        }
    }
}

/// What the operator clicked in the tree this frame: open a document or a canvas (by id) in the
/// active pane. Returned to [`crate::left_rail::LeftRail`], which the app turns into a pane action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectTreeEvent {
    OpenDocument(String),
    OpenCanvas(String),
    /// A Bookmarks-group row was clicked (MT-014 FIX-A). Carries the (document_id, block_id) the
    /// caller opens: when `document_id` is `Some`, open that document (matching React
    /// `handleOpenBookmark`); otherwise open the pinned Loom block by `block_id`.
    OpenBookmark {
        document_id: Option<String>,
        block_id: String,
    },
    /// MT-020 explorer-row context menu: "Copy Path" — copy the row's stable id/path to the clipboard.
    /// Carries the id the caller writes to the clipboard (the document/canvas id or the bookmark block
    /// id), so no backend call is involved.
    CopyPath(String),
    /// MT-020 explorer-row context menu: rename a bookmark's Loom block through its typed handler.
    RenameBlock {
        block_id: String,
        current_title: String,
        expected_updated_at: Option<String>,
    },
    /// Rename a document through the knowledge-document rename route. Kept distinct from
    /// [`Self::RenameBlock`] so a document id can never reach the Loom-block PATCH.
    RenameDocument {
        document_id: String,
        current_title: String,
        expected_updated_at: Option<String>,
    },
    /// Rename a canvas through the dedicated canvas title route with optimistic concurrency.
    RenameCanvas {
        canvas_id: String,
        current_title: String,
        expected_updated_at: Option<String>,
    },
    /// WP-KERNEL-012 MT-033 (E5 — route-to-Stage): "Route to Stage" on a Document row. Carries the
    /// document id + title so the app routes it to the Stage pane via the MT-031 Route-to-Stage command
    /// (`InteractionBus::route_to_stage(StageContent::Document(..))`). Only produced for a Document row
    /// (the Stage pane displays a routed document); canvas/bookmark rows' item is disabled.
    RouteToStage {
        document_id: String,
        title: String,
    },
}

/// The loaded content payload for one workspace: documents, canvases, and bookmarks (pinned Loom
/// blocks). Bundled so a single async load delivers all three over one channel message.
type LoadedContent = (
    Vec<DocumentSummary>,
    Vec<CanvasSummary>,
    Vec<BookmarkSummary>,
);

/// The async load result delivered over the channel: which `load_id` it belongs to (for staleness
/// rejection) and the loaded documents + canvases + bookmarks, or an error string.
struct LoadResult {
    load_id: u64,
    payload: Result<LoadedContent, String>,
}

/// The left-rail project tree widget + its state.
///
/// Owns the loaded document/canvas lists, the per-workspace load lifecycle (load_id + channel),
/// the expand/collapse flags for the two groups, and a width cache for the paper-strip labels. It
/// does NOT own pane state — clicking a leaf is reported as a [`ProjectTreeEvent`] the app applies.
pub struct ProjectTree {
    cabinet: cabinet_host::State,
    cabinet_generation: u64,
    cabinet_source_revision: u64,
    cabinet_active_resource: Option<String>,
    cabinet_rename_identity: Option<(u64, u64, String)>,
    /// Backend root used for asynchronous project-content loads. Production keeps
    /// [`BACKEND_BASE_URL`]; managed-runtime tests rebind it together with the shell's other clients.
    backend_base_url: String,
    authenticated_context: Option<std::sync::Arc<crate::local_account::AuthenticatedContext>>,
    /// The workspace whose content is loaded / loading. `None` until the first workspace is set.
    workspace_id: Option<String>,
    documents: Vec<DocumentSummary>,
    canvases: Vec<CanvasSummary>,
    /// Pinned Loom blocks shown in the Bookmarks group (MT-014 FIX-A), loaded from
    /// `GET /workspaces/{id}/loom/views/pins`.
    bookmarks: Vec<BookmarkSummary>,
    /// Whether a load is in flight for the current workspace.
    loading: bool,
    /// The last load's error, shown inline with a Retry button.
    error: Option<String>,
    /// Group expand/collapse flags (chevron state).
    documents_open: bool,
    canvases_open: bool,
    /// Bookmarks-group expand/collapse flag (chevron state).
    bookmarks_open: bool,
    /// Monotonic load request id. Stamped on every spawn; a delivered result is applied ONLY when
    /// its `load_id` equals this value, so a stale (pre-switch) result is discarded.
    load_id: u64,
    /// The receiving end of the async load channel; drained each frame by [`poll`](Self::poll).
    rx: Option<Receiver<LoadResult>>,
    /// Cached paper-strip label widths, keyed by the row's author_id, recomputed once per loaded list
    /// (not per frame) so a large project does not pay the `ui.fonts` cost every repaint (red-team
    /// RISK: per-frame width computation on 100+ rows).
    width_cache: std::collections::HashMap<String, f32>,
    /// One-shot flag set when the inline Retry button is clicked. `show` does not hold the tokio
    /// runtime handle, so it records the click here; the caller (which DOES hold the handle) reads +
    /// clears it via [`take_retry_request`](Self::take_retry_request) and calls [`retry`](Self::retry).
    retry_requested: bool,
}

impl Default for ProjectTree {
    fn default() -> Self {
        Self::new()
    }
}

impl ProjectTree {
    pub fn new() -> Self {
        Self {
            cabinet: cabinet_host::State::default(),
            cabinet_generation: 0,
            cabinet_source_revision: 0,
            cabinet_active_resource: None,
            cabinet_rename_identity: None,
            backend_base_url: BACKEND_BASE_URL.to_owned(),
            authenticated_context: None,
            workspace_id: None,
            documents: Vec::new(),
            canvases: Vec::new(),
            bookmarks: Vec::new(),
            loading: false,
            error: None,
            documents_open: true,
            canvases_open: true,
            bookmarks_open: true,
            load_id: 0,
            rx: None,
            width_cache: std::collections::HashMap::new(),
            retry_requested: false,
        }
    }

    fn sync_cabinet(&mut self) -> Result<(), String> {
        use cabinet::{Entry, EntryKind};
        let workspace = self.workspace_id.as_deref().unwrap_or("unbound");
        let scope = match self.authenticated_context.as_deref() {
            Some(account) => format!("project:{:?}", [workspace, account.account_id.as_str(), account.principal_id.as_str(), account.session_id.as_str(), account.access_space_id.as_str()]),
            None => format!("project:unbound:{workspace:?}"),
        };
        if self.cabinet.source_is_current(&scope, self.cabinet_generation, self.cabinet_source_revision) { return Ok(()); }
        let mut entries = Vec::new();
        for (index, (slug, title)) in [("documents", "Documents"), ("canvases", "Canvases"), ("bookmarks", "Bookmarks")].into_iter().enumerate() {
            let mut entry = Entry::new(format!("project-tree.group.{slug}"), None, title, EntryKind::Folder);
            entry.order = index as i64;
            entry.capabilities = cabinet_host::capabilities(false, false, false);
            entries.push(entry);
        }
        for (index, doc) in self.documents.iter().enumerate() {
            let mut entry = Entry::new(format!("document:{}", doc.id), Some("project-tree.group.documents".into()), &doc.title, EntryKind::Leaf);
            entry.resource_id = format!("rich-document:{}", doc.id);
            entry.order = index as i64;
            entry.capabilities = cabinet_host::capabilities(true, true, false);
            entries.push(entry);
        }
        for (index, canvas) in self.canvases.iter().enumerate() {
            let mut entry = Entry::new(format!("canvas:{}", canvas.id), Some("project-tree.group.canvases".into()), &canvas.title, EntryKind::Leaf);
            entry.resource_id = format!("canvas:{}", canvas.id);
            entry.order = index as i64;
            entry.capabilities = cabinet_host::capabilities(true, true, false);
            entries.push(entry);
        }
        for (index, bookmark) in self.bookmarks.iter().enumerate() {
            let mut entry = Entry::new(format!("bookmark:{}", bookmark.block_id), Some("project-tree.group.bookmarks".into()), &bookmark.title, EntryKind::Leaf);
            entry.resource_id = format!("loom-block:{}", bookmark.block_id);
            entry.order = index as i64;
            entry.capabilities = cabinet_host::capabilities(true, true, false);
            entries.push(entry);
        }
        let initial = self.cabinet.model().is_none();
        self.cabinet.sync(scope, self.cabinet_generation, self.cabinet_source_revision, entries, true)?;
        if initial {
            for group in ["documents", "canvases", "bookmarks"] {
                self.cabinet.projection_expand(&format!("project-tree.group.{group}"), true)?;
            }
        }
        Ok(())
    }

    pub(crate) fn cabinet_prepare_rename(&mut self, entity_key: &str, title: &str, operation_id: u64) -> Result<(), String> {
        self.sync_cabinet()?;
        let id = self.cabinet.model().and_then(|model| model.snapshot().entries.iter().find(|entry| entry.resource_id == entity_key).map(|entry| entry.id.clone())).ok_or("Rename target is no longer in the project tree")?;
        self.cabinet.prepare_rename(&id, title)?;
        if !self.cabinet.bind(self.cabinet_generation, operation_id) { return Err("Project tree changed before rename dispatch".to_owned()); }
        self.cabinet_rename_identity = Some((self.cabinet_generation, operation_id, entity_key.to_owned()));
        Ok(())
    }

    pub(crate) fn cabinet_accepts_rename_delivery(&self, operation_id: u64, entity_key: &str) -> bool {
        self.cabinet_rename_identity.as_ref().is_some_and(|(generation, operation, entity)| {
            *generation == self.cabinet_generation && *operation == operation_id && entity == entity_key
                && matches!(self.cabinet.bound_operation(*generation, *operation), Some(cabinet::Operation::Rename { .. }))
        })
    }

    pub(crate) fn cabinet_rename_terminal(&mut self, operation_id: u64, result: Result<(), String>) {
        self.cabinet.terminal(self.cabinet_generation, operation_id, result);
    }

    pub(crate) fn cabinet_rename_unstarted(&mut self, operation_id: u64, error: String) {
        self.cabinet.bound_unstarted(self.cabinet_generation, operation_id, error);
    }

    pub(crate) fn cabinet_acknowledge_read(&mut self) { self.cabinet.acknowledge_read(); }

    pub(crate) fn cabinet_reveal_resource(&mut self, resource: Option<&str>) {
        if resource == self.cabinet_active_resource.as_deref() { return; }
        let Some(resource) = resource else { self.cabinet_active_resource = None; return; };
        if self.sync_cabinet().is_err() { return; }
        let id = self.cabinet.model().and_then(|model| model.snapshot().entries.iter().find(|entry| entry.resource_id == resource).map(|entry| entry.id.clone()));
        if let Some(id) = id {
            if self.cabinet.reveal(&id).is_ok() { self.cabinet_active_resource = Some(resource.to_owned()); }
        }
    }

    /// The workspace currently loaded / loading, if any.
    pub fn workspace_id(&self) -> Option<&str> {
        self.workspace_id.as_deref()
    }

    /// Read-only document list (for tests / the caller).
    pub fn documents(&self) -> &[DocumentSummary] {
        &self.documents
    }

    /// Read-only canvas list (for tests / the caller).
    pub fn canvases(&self) -> &[CanvasSummary] {
        &self.canvases
    }

    /// Read-only bookmark list (for tests / the caller).
    pub fn bookmarks(&self) -> &[BookmarkSummary] {
        &self.bookmarks
    }

    /// Whether a load is in flight.
    pub fn is_loading(&self) -> bool {
        self.loading
    }

    /// The current inline error, if any.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Put the tree into the inline-error state with `message`, clearing the loading flag (used by the
    /// app to surface a non-load failure, and by tests to render the error + Retry affordance without a
    /// live backend). Mirrors the state `poll` reaches on a failed [`LoadResult`].
    pub fn set_error(&mut self, message: impl Into<String>) {
        self.error = Some(message.into());
        self.loading = false;
    }

    /// Directly seed the document + canvas lists (tests / a non-HTTP caller). Clears the loading /
    /// error state and refreshes the width cache so the tree renders immediately with no backend.
    /// Leaves the bookmark list unchanged (use [`set_content_with_bookmarks`](Self::set_content_with_bookmarks)
    /// to seed bookmarks too).
    pub fn set_content(&mut self, documents: Vec<DocumentSummary>, canvases: Vec<CanvasSummary>) {
        self.cabinet_source_revision = self.cabinet_source_revision.wrapping_add(1);
        self.documents = documents;
        self.canvases = canvases;
        self.loading = false;
        self.error = None;
        self.width_cache.clear();
    }

    /// Directly seed documents + canvases + bookmarks (tests / a non-HTTP caller). The full-content
    /// twin of [`set_content`](Self::set_content) used when a caller has all three lists (the same
    /// shape the async load delivers).
    pub fn set_content_with_bookmarks(
        &mut self,
        documents: Vec<DocumentSummary>,
        canvases: Vec<CanvasSummary>,
        bookmarks: Vec<BookmarkSummary>,
    ) {
        self.cabinet_source_revision = self.cabinet_source_revision.wrapping_add(1);
        self.documents = documents;
        self.canvases = canvases;
        self.bookmarks = bookmarks;
        self.loading = false;
        self.error = None;
        self.width_cache.clear();
    }

    /// Point the tree at `workspace_id`: if it differs from the current one, clear the old content and
    /// start an async load via `runtime`. A re-set to the SAME workspace is a no-op (no reload storm).
    /// Bumping `load_id` here is what makes any in-flight prior load's result stale.
    pub fn set_workspace(&mut self, workspace_id: &str, runtime: &tokio::runtime::Handle) {
        if self.workspace_id.as_deref() == Some(workspace_id) {
            return;
        }
        self.workspace_id = Some(workspace_id.to_owned());
        self.cabinet_generation = self.cabinet_generation.saturating_add(1);
        self.cabinet = cabinet_host::State::default();
        self.cabinet_active_resource = None;
        self.cabinet_rename_identity = None;
        self.documents.clear();
        self.canvases.clear();
        self.bookmarks.clear();
        self.width_cache.clear();
        self.error = None;
        self.spawn_load(runtime);
    }

    /// Rebind project-tree loading to the managed backend and immediately reload the current
    /// workspace. Replacing the receiver and incrementing `load_id` through [`Self::spawn_load`]
    /// makes any completion from the former backend stale and unable to restore its error/content.
    pub fn bind_authenticated_context(
        &mut self,
        context: Option<std::sync::Arc<crate::local_account::AuthenticatedContext>>,
    ) {
        self.authenticated_context = context;
        self.cabinet_generation = self.cabinet_generation.saturating_add(1);
        self.cabinet = cabinet_host::State::default();
        self.cabinet_active_resource = None;
        self.cabinet_rename_identity = None;
        self.rx = None;
        self.load_id = self.load_id.wrapping_add(1);
        self.documents.clear();
        self.canvases.clear();
        self.bookmarks.clear();
    }

    pub fn set_backend_base_url_for_test(
        &mut self,
        base_url: &str,
        runtime: &tokio::runtime::Handle,
    ) {
        self.backend_base_url = base_url.trim_end_matches('/').to_owned();
        self.error = None;
        self.retry_requested = false;
        self.rx = None;
        if self.workspace_id.is_some() {
            self.spawn_load(runtime);
        }
    }

    /// Spawn the async document + canvas fetch for the current workspace on `runtime`, stamping a
    /// fresh `load_id`. Results arrive on `self.rx`; [`poll`](Self::poll) applies them only if the
    /// `load_id` still matches (staleness rejection).
    fn spawn_load(&mut self, runtime: &tokio::runtime::Handle) {
        let Some(workspace_id) = self.workspace_id.clone() else {
            return;
        };
        self.load_id += 1;
        let load_id = self.load_id;
        self.loading = true;
        self.error = None;
        let backend_base_url = self.backend_base_url.clone();
        let account = self.authenticated_context.clone();

        let (tx, rx): (Sender<LoadResult>, Receiver<LoadResult>) = std::sync::mpsc::channel();
        self.rx = Some(rx);
        runtime.spawn(async move {
            let payload =
                load_project_content_authenticated(&backend_base_url, &workspace_id, account)
                    .await
                    .map_err(|e| e.to_string());
            // The receiver may have been dropped (the tree was reset again); a send error is benign.
            let _ = tx.send(LoadResult { load_id, payload });
        });
    }

    /// Retry the current workspace's load (the Retry button / an explicit refresh). No-op when no
    /// workspace is set.
    pub fn retry(&mut self, runtime: &tokio::runtime::Handle) {
        if self.workspace_id.is_some() {
            self.spawn_load(runtime);
        }
    }

    /// Remove a document from the loaded list by id (mirrors the React `handshake:document-deleted`
    /// event listener so a delete from another surface does not leave a stale tree row). Returns
    /// `true` if a row was removed.
    pub fn remove_document(&mut self, document_id: &str) -> bool {
        let before = self.documents.len();
        self.documents.retain(|d| d.id != document_id);
        let removed = self.documents.len() != before;
        if removed {
            self.cabinet_source_revision = self.cabinet_source_revision.wrapping_add(1);
            self.width_cache.clear();
        }
        removed
    }

    /// Remove a canvas from the loaded list by id (mirrors `handshake:canvas-deleted`). Returns
    /// `true` if a row was removed.
    pub fn remove_canvas(&mut self, canvas_id: &str) -> bool {
        let before = self.canvases.len();
        self.canvases.retain(|c| c.id != canvas_id);
        let removed = self.canvases.len() != before;
        if removed {
            self.cabinet_source_revision = self.cabinet_source_revision.wrapping_add(1);
            self.width_cache.clear();
        }
        removed
    }

    /// Remove a bookmark from the loaded list by its block id (mirrors the React
    /// `handshake:loom-bookmarks-changed` removal). Returns `true` if a row was removed.
    pub fn remove_bookmark(&mut self, block_id: &str) -> bool {
        let before = self.bookmarks.len();
        self.bookmarks.retain(|b| b.block_id != block_id);
        let removed = self.bookmarks.len() != before;
        if removed {
            self.cabinet_source_revision = self.cabinet_source_revision.wrapping_add(1);
            self.width_cache.clear();
        }
        removed
    }

    /// Drain any delivered async load result (non-blocking `try_recv`). A result whose `load_id` does
    /// not match the current counter is discarded (stale, from a workspace that has since changed).
    /// Call once per frame before [`show`](Self::show).
    pub fn poll(&mut self) {
        let Some(rx) = &self.rx else {
            return;
        };
        // Drain every queued message; keep only the one matching the current load_id.
        let mut latest: Option<LoadResult> = None;
        while let Ok(msg) = rx.try_recv() {
            latest = Some(msg);
        }
        if let Some(result) = latest {
            if result.load_id != self.load_id {
                return; // stale result from a superseded workspace load
            }
            self.loading = false;
            match result.payload {
                Ok((documents, canvases, bookmarks)) => {
                    self.cabinet.canonical_refetch();
                    self.cabinet_source_revision = self.cabinet_source_revision.wrapping_add(1);
                    self.documents = documents;
                    self.canvases = canvases;
                    self.bookmarks = bookmarks;
                    self.error = None;
                    self.width_cache.clear();
                }
                Err(e) => {
                    self.error = Some(e);
                }
            }
        }
    }

    /// Render the tree into `ui` and return `Some(event)` when a leaf row was clicked this frame.
    ///
    /// Rendering, top to bottom:
    /// - a "Loading..." label while a load is in flight and no content has arrived yet;
    /// - an inline error label + a Retry button when the last load failed;
    /// - the Documents group (chevron header + paper-strip leaf rows);
    /// - the Canvases group (chevron header + paper-strip leaf rows).
    ///
    /// `colors` carries the active-theme tokens so the tree flips dark<->light with the shell.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        colors: ProjectTreeColors,
    ) -> Option<ProjectTreeEvent> {
        let mut event: Option<ProjectTreeEvent> = None;
        if let Err(error) = self.sync_cabinet() {
            self.error = Some(format!("Project tree unavailable: {error}"));
        }
        let mut visible: std::collections::HashSet<String> = self.cabinet.model().map(|model| model.projected_rows().iter().map(|row| row.id.clone()).collect()).unwrap_or_default();
        if let Some(model) = self.cabinet.model() {
            self.documents_open = model.expanded("project-tree.group.documents");
            self.canvases_open = model.expanded("project-tree.group.canvases");
            self.bookmarks_open = model.expanded("project-tree.group.bookmarks");
        }

        // Container Tree node: register a fixed id on the tree's rect so its live AccessKit node
        // attaches under the correct parent, then enrich it (Role::Tree + author_id).
        let container_rect = ui.available_rect_before_wrap();
        let container_id = unsafe { egui::Id::from_high_entropy_bits(PROJECT_TREE_NODE_ID) };

        // Loading affordance (only while in-flight with nothing yet to show).
        if self.loading
            && self.documents.is_empty()
            && self.canvases.is_empty()
            && self.error.is_none()
        {
            ui.colored_label(colors.muted_text, "Loading\u{2026}");
        }

        // Inline error + Retry button. The Retry button is built by interacting at a FIXED egui::Id
        // (derived from its author_id string) so the Response, its widget_info (Role::Button +
        // Action::Click), the bounding box, and the author_id ALL land on the SAME live node — the
        // same id discipline the tab/module buttons use. This keeps the button addressable
        // out-of-process AND avoids creating a second anonymous clickable node that would trip the
        // MT-025 `assert_no_unnamed_interactive` gate.
        if let Some(err) = self.error.clone() {
            ui.colored_label(colors.error, format!("Load failed: {err}"));
            let retry_id = egui::Id::new(PROJECT_TREE_RETRY_AUTHOR_ID);
            let label = "Retry";
            let font = egui::FontId::proportional(13.0);
            let galley = ui
                .painter()
                .layout_no_wrap(label.to_owned(), font, colors.row_text);
            let pad = 6.0;
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(galley.size().x + pad * 2.0, 22.0),
                egui::Sense::hover(),
            );
            let retry_resp = ui.interact(rect, retry_id, egui::Sense::click());
            if ui.is_rect_visible(rect) {
                let bg = if retry_resp.hovered() {
                    colors.row_hover_bg
                } else {
                    colors.row_bg
                };
                ui.painter().rect_filled(rect, 3.0, bg);
                ui.painter().galley(
                    egui::pos2(rect.left() + pad, rect.center().y - galley.size().y * 0.5),
                    galley,
                    colors.row_text,
                );
            }
            retry_resp.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
            });
            ui.ctx().accesskit_node_builder(retry_id, |node| {
                node.set_role(accesskit::Role::Button);
                node.set_author_id(PROJECT_TREE_RETRY_AUTHOR_ID.to_owned());
                node.set_label(label.to_owned());
            });
            if retry_resp.clicked() {
                // The runtime handle is not held here; the caller re-spawns via `retry` after reading
                // this one-shot flag through `take_retry_request`.
                self.retry_requested = true;
            }
        }

        // Documents group.
        if Self::group_header(
            ui,
            "Documents",
            "documents",
            &mut self.documents_open,
            self.documents.len(),
            colors,
            &mut self.cabinet,
        ) {
            let _ = self.cabinet.projection_expand("project-tree.group.documents", self.documents_open);
            if let Some(model) = self.cabinet.model() { visible = model.projected_rows().iter().map(|row| row.id.clone()).collect(); }
        }
        if self.documents_open {
            for doc in &self.documents {
                let author_id = format!("project-tree.doc.{}", stable_part(&doc.id));
                let entry_id = format!("document:{}", doc.id);
                if !visible.contains(&entry_id) { continue; }
                let width = *self
                    .width_cache
                    .entry(author_id.clone())
                    .or_insert_with(|| measure_label(ui, &doc.title));
                let row_resp = ui.add_enabled_ui(self.cabinet.navigation_available(), |ui| paper_strip_row(ui, &doc.title, &author_id, width, colors)).inner;
                self.cabinet.row_navigation(ui, &row_resp, &entry_id);
                if row_resp.clicked() && self.cabinet.select_for_open(ui, &row_resp, &entry_id) && self.cabinet.request(cabinet::Operation::Open { id: entry_id.clone() }).is_ok() {
                    row_resp.request_focus();
                    event = Some(ProjectTreeEvent::OpenDocument(doc.id.clone()));
                }
                let target = ExplorerRowTarget {
                    kind: ExplorerRowKind::Document,
                    id: doc.id.clone(),
                    title: doc.title.clone(),
                    document_id: None,
                    expected_updated_at: doc.updated_at.clone(),
                };
                if let Some(menu_event) = row_context_menu(ui, &row_resp, &target, self.cabinet.navigation_available()) {
                    event = Some(menu_event);
                }
            }
        }

        // Canvases group.
        if Self::group_header(
            ui,
            "Canvases",
            "canvases",
            &mut self.canvases_open,
            self.canvases.len(),
            colors,
            &mut self.cabinet,
        ) {
            let _ = self.cabinet.projection_expand("project-tree.group.canvases", self.canvases_open);
            if let Some(model) = self.cabinet.model() { visible = model.projected_rows().iter().map(|row| row.id.clone()).collect(); }
        }
        if self.canvases_open {
            for canvas in &self.canvases {
                let author_id = format!("project-tree.canvas.{}", stable_part(&canvas.id));
                let entry_id = format!("canvas:{}", canvas.id);
                if !visible.contains(&entry_id) { continue; }
                let width = *self
                    .width_cache
                    .entry(author_id.clone())
                    .or_insert_with(|| measure_label(ui, &canvas.title));
                let row_resp = ui.add_enabled_ui(self.cabinet.navigation_available(), |ui| paper_strip_row(ui, &canvas.title, &author_id, width, colors)).inner;
                self.cabinet.row_navigation(ui, &row_resp, &entry_id);
                if row_resp.clicked() && self.cabinet.select_for_open(ui, &row_resp, &entry_id) && self.cabinet.request(cabinet::Operation::Open { id: entry_id.clone() }).is_ok() {
                    row_resp.request_focus();
                    event = Some(ProjectTreeEvent::OpenCanvas(canvas.id.clone()));
                }
                let target = ExplorerRowTarget {
                    kind: ExplorerRowKind::Canvas,
                    id: canvas.id.clone(),
                    title: canvas.title.clone(),
                    document_id: None,
                    expected_updated_at: canvas.updated_at.clone(),
                };
                if let Some(menu_event) = row_context_menu(ui, &row_resp, &target, self.cabinet.navigation_available()) {
                    event = Some(menu_event);
                }
            }
        }

        // Bookmarks group (MT-014 FIX-A): pinned Loom blocks, rendered below the document/canvas tree
        // inside the Files panel. Each row shows the bookmark title + a kind badge ([document] /
        // [file] / [tag_hub] / [journal] / [block]) and, when clicked, opens the owning document (if
        // any) or the pinned block (matching React `handleOpenBookmark`).
        let bookmarks_rect_start = ui.available_rect_before_wrap();
        let bookmarks_container_id = unsafe { egui::Id::from_high_entropy_bits(BOOKMARKS_NODE_ID) };
        if Self::group_header(
            ui,
            "Bookmarks",
            "bookmarks",
            &mut self.bookmarks_open,
            self.bookmarks.len(),
            colors,
            &mut self.cabinet,
        ) {
            let _ = self.cabinet.projection_expand("project-tree.group.bookmarks", self.bookmarks_open);
            if let Some(model) = self.cabinet.model() { visible = model.projected_rows().iter().map(|row| row.id.clone()).collect(); }
        }
        if self.bookmarks_open {
            for bookmark in &self.bookmarks {
                let author_id =
                    format!("project-tree.bookmark.{}", stable_part(&bookmark.block_id));
                let entry_id = format!("bookmark:{}", bookmark.block_id);
                if !visible.contains(&entry_id) { continue; }
                let label_text = format!("{}  [{}]", bookmark.title, bookmark.kind);
                let width = *self
                    .width_cache
                    .entry(author_id.clone())
                    .or_insert_with(|| measure_label(ui, &label_text));
                let row_resp = ui.add_enabled_ui(self.cabinet.navigation_available(), |ui| paper_strip_row(ui, &label_text, &author_id, width, colors)).inner;
                self.cabinet.row_navigation(ui, &row_resp, &entry_id);
                if row_resp.clicked() && self.cabinet.select_for_open(ui, &row_resp, &entry_id) && self.cabinet.request(cabinet::Operation::Open { id: entry_id.clone() }).is_ok() {
                    row_resp.request_focus();
                    event = Some(ProjectTreeEvent::OpenBookmark {
                        document_id: bookmark.document_id.clone(),
                        block_id: bookmark.block_id.clone(),
                    });
                }
                // The rename target is the bookmark's BLOCK id (the Loom block the PATCH renames), and
                // the rename seed is the bookmark TITLE (not the badge-suffixed label).
                let target = ExplorerRowTarget {
                    kind: ExplorerRowKind::Bookmark,
                    id: bookmark.block_id.clone(),
                    title: bookmark.title.clone(),
                    document_id: bookmark.document_id.clone(),
                    expected_updated_at: bookmark.updated_at.clone(),
                };
                if let Some(menu_event) = row_context_menu(ui, &row_resp, &target, self.cabinet.navigation_available()) {
                    event = Some(menu_event);
                }
            }
        }
        // Enrich the bookmarks container node (Role::Tree) so the bookmarks region is addressable.
        // The start rect (available space at the group's top) spans the rendered rows downward, the
        // same convention the project-tree container uses for its own rect.
        ui.interact(
            bookmarks_rect_start,
            bookmarks_container_id,
            egui::Sense::focusable_noninteractive(),
        );
        ui.ctx()
            .accesskit_node_builder(bookmarks_container_id, |node| {
                node.set_role(accesskit::Role::Tree);
                node.set_author_id(BOOKMARKS_AUTHOR_ID.to_owned());
                node.set_label("Bookmarks".to_owned());
            });

        // Enrich the container node last so its rect spans everything rendered above.
        ui.interact(
            container_rect,
            container_id,
            egui::Sense::focusable_noninteractive(),
        );
        ui.ctx().accesskit_node_builder(container_id, |node| {
            node.set_role(accesskit::Role::Tree);
            node.set_author_id(PROJECT_TREE_AUTHOR_ID.to_owned());
            node.set_label("Project tree".to_owned());
        });

        self.cabinet.take_navigation_changes();
        event
    }

    /// Render a group header (chevron + label + count) as a `Role::TreeItem` and toggle `open` on a
    /// click. Returns `true` if the open state changed this frame.
    fn group_header(
        ui: &mut egui::Ui,
        label: &str,
        slug: &str,
        open: &mut bool,
        count: usize,
        colors: ProjectTreeColors,
        cabinet: &mut cabinet_host::State,
    ) -> bool {
        let author_id = format!("project-tree.group.{slug}");
        let id = egui::Id::new(&author_id);
        let chevron = if *open { "\u{25BE}" } else { "\u{25B8}" }; // ▾ / ▸
        let text = format!("{chevron} {label} ({count})");

        let font = egui::FontId::proportional(13.0);
        let galley = ui
            .painter()
            .layout_no_wrap(text.clone(), font, colors.group_text);
        let pad = 4.0;
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(galley.size().x + pad * 2.0, 20.0),
            egui::Sense::hover(),
        );
        let resp = ui.interact(rect, id, egui::Sense::click());
        cabinet.row_navigation(ui, &resp, &author_id);
        if ui.is_rect_visible(rect) {
            ui.painter().galley(
                egui::pos2(rect.left() + pad, rect.center().y - galley.size().y * 0.5),
                galley,
                colors.group_text,
            );
        }
        // AccessKit: a TreeItem with a click action (expand/collapse).
        resp.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), &text)
        });
        ui.ctx().accesskit_node_builder(id, |node| {
            node.set_role(accesskit::Role::TreeItem);
            node.set_author_id(author_id.clone());
            node.set_label(format!("{label} group"));
            node.set_expanded(*open);
        });

        let mut changed = false;
        if resp.clicked() {
            resp.request_focus();
            let _ = cabinet.select(&author_id, cabinet::SelectionModifiers::default());
            *open = !*open;
            changed = true;
        }
        changed
    }
}

// A retry flag is needed because `show` does not have the runtime handle. Declared as a field via an
// impl extension below so the struct definition stays focused; instead store it on the struct.
impl ProjectTree {
    /// Whether the Retry button was clicked this frame. The caller (which holds the tokio runtime
    /// handle) reads + clears this and calls [`retry`](Self::retry). Kept as a one-shot flag rather
    /// than threading the runtime handle into `show` so the widget stays runtime-agnostic.
    pub fn take_retry_request(&mut self) -> bool {
        std::mem::take(&mut self.retry_requested)
    }
}

/// Stable author_id for the inline Retry button shown on a failed load.
pub const PROJECT_TREE_RETRY_AUTHOR_ID: &str = "project-tree.retry";

/// Measure a label's pixel width for the paper-strip allocation (the core variable-width visual). A
/// single `layout_no_wrap` call; the caller caches the result per row so this runs once per loaded
/// list, not every frame.
fn measure_label(ui: &egui::Ui, label: &str) -> f32 {
    // The galley color does not affect layout width; use egui's PLACEHOLDER sentinel (the "decided
    // later" color) rather than a hardcoded color literal so this measurement helper sources no real
    // color outside the palette module (theme-hygiene gate in test_theme.rs).
    let galley = ui.painter().layout_no_wrap(
        label.to_owned(),
        egui::FontId::proportional(13.0),
        egui::Color32::PLACEHOLDER,
    );
    galley.size().x
}

/// Render ONE paper-strip leaf row: an off-white label background ONLY as wide as the label text
/// (`label_width + padding`), with the label RIGHT-aligned within that strip via a
/// `right_to_left` layout. This is the core visual differentiator from a full-row-fill sidebar (the
/// MT-014 contract's paper-strip aesthetic).
///
/// The row is a `Role::TreeItem` with a click action, addressed by `author_id`. Returns the row's
/// [`egui::Response`] (built at the FIXED `author_id`-derived id) so the caller can read `.clicked()`
/// AND attach the MT-020 right-click context menu to the SAME named node (no new unnamed interactive
/// node, so the MT-025 gate stays green). The response carries `Sense::click()`, which egui derives
/// `secondary_clicked()` from for the context-menu open path.
fn paper_strip_row(
    ui: &mut egui::Ui,
    label: &str,
    author_id: &str,
    label_width: f32,
    colors: ProjectTreeColors,
) -> egui::Response {
    let id = egui::Id::new(author_id);
    let pad = 6.0;
    let strip_w = (label_width + pad * 2.0).min(ui.available_width().max(0.0));
    let height = 20.0;

    // Allocate exactly the strip width (NOT the full row) so the off-white background is variable-
    // width per the paper-strip design. Indent the strip from the left by a small chevron gutter so
    // it reads as a tree leaf under its group header.
    let indent = 16.0;
    let inner = ui.horizontal(|ui| {
        ui.add_space(indent);
        let (rect, _) = ui.allocate_exact_size(egui::vec2(strip_w, height), egui::Sense::hover());
        let resp = ui.interact(rect, id, egui::Sense::click());
        if ui.is_rect_visible(rect) {
            let bg = if resp.hovered() {
                colors.row_hover_bg
            } else {
                colors.row_bg
            };
            ui.painter().rect_filled(rect, 3.0, bg);
            // Right-aligned label within the strip.
            let galley = ui.painter().layout_no_wrap(
                label.to_owned(),
                egui::FontId::proportional(13.0),
                colors.row_text,
            );
            let text_pos = egui::pos2(
                rect.right() - pad - galley.size().x,
                rect.center().y - galley.size().y * 0.5,
            );
            ui.painter().galley(text_pos, galley, colors.row_text);
        }
        resp.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
        });
        ui.ctx().accesskit_node_builder(id, |node| {
            node.set_role(accesskit::Role::TreeItem);
            node.set_author_id(author_id.to_owned());
            node.set_label(label.to_owned());
        });
        resp
    });
    inner.inner
}

/// The identity of an explorer row for the MT-020 right-click menu: the row [`ExplorerRowKind`], the
/// id the menu copies/opens (`content_id` for a document/canvas, `block_id` for a bookmark), the title
/// for the rename seed, and the optional owning `document_id` (bookmark only) so the Open action can
/// reuse the existing `OpenBookmark` event semantics.
struct ExplorerRowTarget {
    kind: ExplorerRowKind,
    /// The stable id the menu copies + opens (document/canvas id or bookmark block id).
    id: String,
    /// The display title (rename seed).
    title: String,
    /// Bookmark-only: the owning document id (drives `OpenBookmark` open semantics).
    document_id: Option<String>,
    /// Optimistic concurrency token captured with the rendered row.
    expected_updated_at: Option<String>,
}

/// Attach the MT-020 explorer-row context menu to a project-tree row's `row_resp` (the named
/// `Role::TreeItem` node) and return the menu-driven [`ProjectTreeEvent`] when an enabled item is
/// confirmed this frame. Mirrors the tab/header surfaces: the menu is built from the SHARED MT-019
/// infra ([`ContextMenu::show_on`]) so it paints via egui's hardened popup, emits `Role::MenuItem`
/// AccessKit nodes (`ctx-menu.explorer.*`) for out-of-process steering, supports keyboard nav, and is
/// CLOSED by default (so the MT-025 default-frame snapshot is unchanged). `show_on` opens on the row's
/// `secondary_clicked()`; Shift+F10 opens the same popup when the row is focused (keyboard parity,
/// gated on `has_focus()` so it never leaks to global shortcuts).
fn row_context_menu(
    ui: &mut egui::Ui,
    row_resp: &egui::Response,
    target: &ExplorerRowTarget,
    available: bool,
) -> Option<ProjectTreeEvent> {
    let mut event = None;
    let items = explorer_context_items(target.kind);
    let items = if available { items } else { items.into_iter().map(|item| item.disabled("Wait for the current tree operation to be reconciled")).collect() };
    let menu = ContextMenu::new("explorer").items(items);
    if let Some(confirmed_id) = menu.show_on(row_resp) {
        if let Some(action) = explorer_action_for_id(confirmed_id, target.kind) {
            event = Some(match action {
                ExplorerMenuAction::Open => open_event_for(target),
                ExplorerMenuAction::CopyPath => ProjectTreeEvent::CopyPath(target.id.clone()),
                ExplorerMenuAction::Rename => ProjectTreeEvent::RenameBlock {
                    block_id: target.id.clone(),
                    current_title: target.title.clone(),
                    expected_updated_at: target.expected_updated_at.clone(),
                },
                ExplorerMenuAction::RenameDocument => ProjectTreeEvent::RenameDocument {
                    document_id: target.id.clone(),
                    current_title: target.title.clone(),
                    expected_updated_at: target.expected_updated_at.clone(),
                },
                ExplorerMenuAction::RenameCanvas => ProjectTreeEvent::RenameCanvas {
                    canvas_id: target.id.clone(),
                    current_title: target.title.clone(),
                    expected_updated_at: target.expected_updated_at.clone(),
                },
                ExplorerMenuAction::RouteToStage => ProjectTreeEvent::RouteToStage {
                    document_id: target.id.clone(),
                    title: target.title.clone(),
                },
            });
        }
    }
    // Shift+F10 keyboard-open parity (egui 0.33 has no dedicated Menu/ContextMenu key).
    if row_resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::F10) && i.modifiers.shift) {
        crate::context_menu::request_open(ui.ctx(), row_resp.id, row_resp.rect.left_bottom());
    }
    event
}

/// The open event for an explorer row — the SAME event a left-click produces, so the menu's "Open"
/// item reuses the existing open path (no new open semantics).
fn open_event_for(target: &ExplorerRowTarget) -> ProjectTreeEvent {
    match target.kind {
        ExplorerRowKind::Document => ProjectTreeEvent::OpenDocument(target.id.clone()),
        ExplorerRowKind::Canvas => ProjectTreeEvent::OpenCanvas(target.id.clone()),
        ExplorerRowKind::Bookmark => ProjectTreeEvent::OpenBookmark {
            document_id: target.document_id.clone(),
            block_id: target.id.clone(),
        },
    }
}

/// Slugify a content id to `[a-z0-9-]` so a document/canvas id containing spaces, slashes, or UTF-8
/// can never produce a malformed AccessKit author_id (red-team RISK: invalid id chars). Mirrors the
/// React `stableIdPart()` (`app/src/App.tsx` lines 345-352): lowercase, replace every run of
/// non-`[a-z0-9]` with a single `-`, and trim leading/trailing `-`. An id that slugs to empty (all
/// punctuation) falls back to a stable hash so it is still unique and addressable.
pub fn stable_part(id: &str) -> String {
    let mut out = String::with_capacity(id.len());
    let mut last_dash = false;
    for ch in id.chars() {
        let c = ch.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() {
            out.push(c);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    let trimmed = out.trim_matches('-').to_owned();
    if trimmed.is_empty() {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        id.hash(&mut hasher);
        format!("id-{:x}", hasher.finish())
    } else {
        trimmed
    }
}

/// Colors the tree paints with, sourced from the active theme tokens by the caller so the tree never
/// reads egui's generic visuals (mirrors `tab_bar::TabBarColors` / `project_tabs::ProjectTabColors`).
#[derive(Debug, Clone, Copy)]
pub struct ProjectTreeColors {
    /// Off-white paper-strip background for a leaf row.
    pub row_bg: egui::Color32,
    /// Paper-strip background for a hovered leaf row.
    pub row_hover_bg: egui::Color32,
    /// Leaf row label text.
    pub row_text: egui::Color32,
    /// Group-header (Documents / Canvases) text.
    pub group_text: egui::Color32,
    /// Muted text for the "Loading..." affordance.
    pub muted_text: egui::Color32,
    /// Inline error text.
    pub error: egui::Color32,
}

/// Load a workspace's rich documents + canvases + bookmarks over the existing backend HTTP API.
/// Sequential reads (`/knowledge/documents?workspace_id={id}`, `/workspaces/{id}/canvases`,
/// `/workspaces/{id}/loom/views/pins`), each deserialized via `serde_json::Value` so this adds no
/// dependency on the `handshake_core` crate's types — the same pattern `project_tabs::fetch_workspaces`
/// uses. Rows missing an `id` are skipped (a malformed row must not fail the whole load); a missing
/// `title` falls back to the id.
pub async fn load_project_content(
    base_url: &str,
    workspace_id: &str,
) -> Result<LoadedContent, AppError> {
    load_project_content_authenticated(base_url, workspace_id, None).await
}

pub async fn load_project_content_authenticated(
    base_url: &str,
    workspace_id: &str,
    account: Option<std::sync::Arc<crate::local_account::AuthenticatedContext>>,
) -> Result<LoadedContent, AppError> {
    let client = crate::backend_client::shared_http_client();
    let documents = fetch_rich_document_summaries(base_url, workspace_id, account.clone()).await?;
    let canvases =
        fetch_summaries(&client, base_url, workspace_id, "canvases", account.clone()).await?;
    let bookmarks = fetch_bookmarks(&client, base_url, workspace_id, account).await?;
    let documents = documents
        .into_iter()
        .map(|(id, title, updated_at)| DocumentSummary {
            id,
            title,
            updated_at,
        })
        .collect();
    let canvases = canvases
        .into_iter()
        .map(|(id, title, updated_at)| CanvasSummary {
            id,
            title,
            updated_at,
        })
        .collect();
    Ok((documents, canvases, bookmarks))
}

async fn fetch_rich_document_summaries(
    base_url: &str,
    workspace_id: &str,
    account: Option<std::sync::Arc<crate::local_account::AuthenticatedContext>>,
) -> Result<Vec<(String, String, Option<String>)>, AppError> {
    let client = crate::backend::knowledge_documents::KnowledgeDocumentsClient::with_client(
        crate::backend_client::shared_http_client(),
        base_url,
    )
    .with_optional_authenticated_context(account);
    let headers = crate::backend::knowledge_documents::HskDocumentHeaders::for_read(
        "native-project-tree",
        workspace_id,
    );
    client
        .list_documents(&headers, workspace_id)
        .await
        .map(|documents| {
            documents
                .into_iter()
                .map(|document| {
                    (
                        document.rich_document_id,
                        document.title,
                        Some(document.updated_at),
                    )
                })
                .collect()
        })
        .map_err(|error| AppError::Http(error.to_string()))
}

/// Load every pins page through the authenticated Loom view. The backend caps pages at 500 and
/// applies the offset before returning `blocks`; a short raw page ends traversal. Malformed rows
/// without an id retain the existing skip behavior and do not alter pagination. A failed later page
/// or repeated block identity rejects the whole load, preserving the tree's previous set.
/// Offset responses carry no snapshot revision; concurrent server reordering can require a refresh.
async fn fetch_bookmarks(
    client: &reqwest::Client,
    base_url: &str,
    workspace_id: &str,
    account: Option<std::sync::Arc<crate::local_account::AuthenticatedContext>>,
) -> Result<Vec<BookmarkSummary>, AppError> {
    const PAGE_SIZE: u32 = 500;
    let mut offset = 0u32;
    let mut rows = Vec::new();
    let mut seen = std::collections::HashSet::new();
    loop {
        let url = format!(
            "{base_url}/workspaces/{workspace_id}/loom/views/pins?limit={PAGE_SIZE}&offset={offset}"
        );
        let resp = crate::local_account::AuthenticatedRequest::new(
            client.clone(),
            account.clone(),
            client.get(&url).timeout(std::time::Duration::from_secs(5)),
        )
        .send()
        .await
        .map_err(|error| AppError::Http(error.to_string()))?;
        if !resp.status().is_success() {
            return Err(AppError::Http(format!("non-success status {}", resp.status())));
        }
        let value = decode_bookmark_page(resp, account.as_ref()).await?;
        if value.get("view_type").and_then(serde_json::Value::as_str) != Some("pins") {
            return Err(AppError::Parse("expected the pins view".to_owned()));
        }
        let page = value.get("blocks").and_then(serde_json::Value::as_array)
            .ok_or_else(|| AppError::Parse("expected a 'blocks' array in the pins view".to_owned()))?;
        if page.len() > PAGE_SIZE as usize {
            return Err(AppError::Parse("pins page exceeds the requested limit".to_owned()));
        }
        for row in page {
            let Some(block_id) = row.get("block_id").and_then(serde_json::Value::as_str) else {
                continue;
            };
            if !seen.insert(block_id.to_owned()) {
                return Err(AppError::Parse(
                    "pins changed during pagination or contain a duplicate block; reload".to_owned(),
                ));
            }
            let document_id = row.get("document_id").and_then(serde_json::Value::as_str)
                .map(str::to_owned);
            let updated_at = row.get("updated_at").and_then(serde_json::Value::as_str)
                .map(str::to_owned);
            let title = row.get("title").and_then(serde_json::Value::as_str)
                .filter(|title| !title.trim().is_empty())
                .or_else(|| row.get("original_filename").and_then(serde_json::Value::as_str)
                    .filter(|name| !name.trim().is_empty()))
                .unwrap_or(block_id).to_owned();
            let content_type = row.get("content_type").and_then(serde_json::Value::as_str)
                .unwrap_or("");
            rows.push(BookmarkSummary {
                block_id: block_id.to_owned(),
                title,
                kind: bookmark_kind(document_id.as_deref(), content_type),
                document_id,
                updated_at,
            });
        }
        if page.len() < PAGE_SIZE as usize {
            return Ok(rows);
        }
        offset = offset.checked_add(PAGE_SIZE)
            .ok_or_else(|| AppError::Parse("pins pagination offset overflow".to_owned()))?;
    }
}

async fn decode_bookmark_page(
    response: reqwest::Response,
    account: Option<&std::sync::Arc<crate::local_account::AuthenticatedContext>>,
) -> Result<serde_json::Value, AppError> {
    let value = response.json().await
        .map_err(|error| AppError::Parse(error.to_string()))?;
    if !account.is_some_and(|context| context.is_active()) {
        return Err(AppError::Http("Account session is no longer active".to_owned()));
    }
    Ok(value)
}

/// Compute the bookmark kind badge exactly like the React `bookmarkKind()` helper
/// (`WorkspaceSidebar.tsx` lines 43-49): a pin with an owning `document_id` is a "document"; otherwise
/// a `file`/`tag_hub`/`journal` content_type uses that content_type verbatim; everything else is a
/// "block".
fn bookmark_kind(document_id: Option<&str>, content_type: &str) -> String {
    if document_id.map(|d| !d.trim().is_empty()).unwrap_or(false) {
        return "document".to_owned();
    }
    match content_type {
        "file" | "tag_hub" | "journal" => content_type.to_owned(),
        _ => "block".to_owned(),
    }
}

/// GET `/workspaces/{id}/{kind}` and map the JSON array to `(id, title)` pairs. `kind` is
/// `"documents"` or `"canvases"`. A non-success status or a parse failure is an error, never a panic.
async fn fetch_summaries(
    client: &reqwest::Client,
    base_url: &str,
    workspace_id: &str,
    kind: &str,
    account: Option<std::sync::Arc<crate::local_account::AuthenticatedContext>>,
) -> Result<Vec<(String, String, Option<String>)>, AppError> {
    let url = format!("{base_url}/workspaces/{workspace_id}/{kind}");
    let resp = crate::local_account::AuthenticatedRequest::new(
        client.clone(),
        account,
        client.get(&url).timeout(std::time::Duration::from_secs(5)),
    )
    .send()
    .await
    .map_err(|e| AppError::Http(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(AppError::Http(format!(
            "non-success status {}",
            resp.status()
        )));
    }
    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::Parse(e.to_string()))?;
    let arr = v
        .as_array()
        .ok_or_else(|| AppError::Parse(format!("expected a JSON array of {kind}")))?;
    let rows = arr
        .iter()
        .filter_map(|row| {
            let id = row.get("id").and_then(|x| x.as_str())?;
            let title = row
                .get("title")
                .and_then(|x| x.as_str())
                .or_else(|| row.get("name").and_then(|x| x.as_str()))
                .unwrap_or(id)
                .to_owned();
            let updated_at = row
                .get("updated_at")
                .and_then(|value| value.as_str())
                .map(ToOwned::to_owned);
            Some((id.to_owned(), title, updated_at))
        })
        .collect();
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cabinet_project_rename_waits_for_current_terminal_and_canonical_poll() {
        let mut tree = ProjectTree::new();
        tree.workspace_id = Some("workspace-one".to_owned());
        tree.set_content_with_bookmarks(
            vec![DocumentSummary::new("D-1", "Before")],
            vec![CanvasSummary::new("C-1", "Canvas")],
            vec![BookmarkSummary::new("B-1", "Bookmark", "document", Some("D-1".to_owned()))],
        );
        tree.sync_cabinet().unwrap();
        let model = tree.cabinet.model().unwrap();
        assert!(model.entry("document:D-1").is_some());
        assert!(model.entry("canvas:C-1").is_some());
        assert_eq!(model.entry("bookmark:B-1").unwrap().resource_id, "loom-block:B-1");
        tree.cabinet.projection_expand("project-tree.group.documents", false).unwrap();
        tree.cabinet_reveal_resource(Some("rich-document:D-1"));
        assert!(tree.cabinet.model().unwrap().projected_rows().iter().any(|row| row.id == "document:D-1"));

        tree.cabinet_prepare_rename("rich-document:D-1", "After", 17).unwrap();
        tree.cabinet_rename_terminal(16, Ok(()));
        assert!(tree.cabinet.model().unwrap().pending().is_some());
        tree.cabinet_rename_terminal(17, Ok(()));
        tree.sync_cabinet().unwrap();
        assert_eq!(tree.cabinet.model().unwrap().entry("document:D-1").unwrap().label, "Before");
        assert!(tree.cabinet.model().unwrap().pending().is_some());

        let (tx, rx) = std::sync::mpsc::channel();
        tree.rx = Some(rx);
        tx.send(LoadResult { load_id: tree.load_id, payload: Ok((vec![DocumentSummary::new("D-1", "After")], tree.canvases.clone(), tree.bookmarks.clone())) }).unwrap();
        tree.poll();
        tree.sync_cabinet().unwrap();
        let model = tree.cabinet.model().unwrap();
        assert!(model.pending().is_none());
        assert_eq!(model.entry("document:D-1").unwrap().label, "After");

        tree.cabinet_prepare_rename("rich-document:D-1", "Denied", 18).unwrap();
        tree.cabinet_rename_terminal(18, Err("denied".to_owned()));
        assert_eq!(tree.cabinet.model().unwrap().entry("document:D-1").unwrap().label, "After");
        tree.bind_authenticated_context(None);
        assert!(tree.cabinet.model().is_none());
        assert!(tree.documents().is_empty());
    }

    #[test]
    fn cabinet_rename_custody_outlives_dialog_and_local_projection_refresh() {
        let mut tree = ProjectTree::new();
        tree.workspace_id = Some("workspace-one".to_owned());
        tree.set_content(vec![DocumentSummary::new("D-1", "Before")], Vec::new());
        tree.cabinet_prepare_rename("rich-document:D-1", "After", 41).unwrap();
        // The host dialog may close independently. Only the retained operation and entity own
        // its delivery, including after another canonical load invalidates local pending state.
        tree.set_content(vec![DocumentSummary::new("D-1", "Before")], Vec::new());
        tree.sync_cabinet().unwrap();
        assert!(tree.cabinet.model().unwrap().pending().is_none());
        assert!(tree.cabinet_accepts_rename_delivery(41, "rich-document:D-1"));
        assert!(!tree.cabinet_accepts_rename_delivery(40, "rich-document:D-1"));
        assert!(!tree.cabinet_accepts_rename_delivery(41, "canvas:D-1"));
        assert!(!tree.cabinet.navigation_available());
        tree.cabinet_rename_terminal(41, Ok(()));
        tree.sync_cabinet().unwrap();
        assert!(!tree.cabinet.navigation_available());

        let (tx, rx) = std::sync::mpsc::channel();
        tree.rx = Some(rx);
        tx.send(LoadResult { load_id: tree.load_id, payload: Ok((vec![DocumentSummary::new("D-1", "After")], Vec::new(), Vec::new())) }).unwrap();
        tree.poll();
        tree.sync_cabinet().unwrap();
        assert!(tree.cabinet.navigation_available());
        assert!(!tree.cabinet_accepts_rename_delivery(41, "rich-document:D-1"));
        assert_eq!(tree.cabinet.model().unwrap().entry("document:D-1").unwrap().label, "After");
        tree.cabinet_prepare_rename("rich-document:D-1", "Again", 42).unwrap();
        tree.bind_authenticated_context(None);
        assert!(!tree.cabinet_accepts_rename_delivery(42, "rich-document:D-1"));
    }

    struct BookmarkPageServer {
        base: String,
        account: std::sync::Arc<crate::local_account::AuthenticatedContext>,
        stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
        worker: Option<std::thread::JoinHandle<Vec<String>>>,
        body_release: Option<std::sync::mpsc::Sender<()>>,
    }

    impl BookmarkPageServer {
        fn new(pages: Vec<(u16, serde_json::Value)>, hold_body: bool) -> Self {
            use std::io::{Read, Write};
            use std::sync::atomic::Ordering;
            use std::time::{Duration, Instant};

            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let base = format!("http://{}", listener.local_addr().unwrap());
            let account = crate::local_account::mock_account_context(&base);
            let (body_release, body_wait) = std::sync::mpsc::channel();
            let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let worker_stop = stop.clone();
            let worker = std::thread::spawn(move || {
                let mut requests = Vec::new();
                let deadline = Instant::now() + Duration::from_secs(15);
                for (status, value) in pages {
                    let mut stream = loop {
                        if worker_stop.load(Ordering::Acquire) {
                            return requests;
                        }
                        assert!(Instant::now() < deadline, "bookmark fixture request timed out");
                        match listener.accept() {
                            Ok((stream, _)) => break stream,
                            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                                std::thread::sleep(Duration::from_millis(1));
                            }
                            Err(error) => panic!("bookmark fixture accept: {error}"),
                        }
                    };
                    stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
                    stream.set_write_timeout(Some(Duration::from_secs(5))).unwrap();
                    let mut request = Vec::new();
                    while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                        let mut chunk = [0; 4096];
                        let count = stream.read(&mut chunk).unwrap();
                        assert!(count > 0 && request.len() + count <= 16_384);
                        request.extend_from_slice(&chunk[..count]);
                    }
                    requests.push(String::from_utf8(request).unwrap());
                    let body = serde_json::to_string(&value).unwrap();
                    let headers = format!(
                        "HTTP/1.1 {status} fixture\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n",
                        body.len(),
                    );
                    stream.write_all(headers.as_bytes()).unwrap();
                    stream.flush().unwrap();
                    if hold_body {
                        body_wait.recv_timeout(Duration::from_secs(5)).expect("release held response body");
                    }
                    stream.write_all(body.as_bytes()).unwrap();
                }
                requests
            });
            Self {
                base, account, stop, worker: Some(worker),
                body_release: hold_body.then_some(body_release),
            }
        }

        fn fetch(&self) -> Result<Vec<BookmarkSummary>, AppError> {
            tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(
                fetch_bookmarks(&reqwest::Client::new(), &self.base, "workspace-1", Some(self.account.clone())),
            )
        }

        fn release_body(&mut self) {
            if let Some(release) = self.body_release.take() {
                let _ = release.send(());
            }
        }

        fn finish(mut self) -> Vec<String> {
            self.release_body();
            self.worker.take().unwrap().join().unwrap()
        }
    }

    impl Drop for BookmarkPageServer {
        fn drop(&mut self) {
            self.release_body();
            self.stop.store(true, std::sync::atomic::Ordering::Release);
            if let Some(worker) = self.worker.take() {
                let _ = worker.join();
            }
        }
    }

    fn bookmark_page(rows: Vec<serde_json::Value>) -> serde_json::Value {
        serde_json::json!({ "view_type": "pins", "blocks": rows })
    }

    fn bookmark_rows(count: usize) -> Vec<serde_json::Value> {
        (0..count).map(|index| serde_json::json!({ "block_id": format!("bookmark-{index}") })).collect()
    }

    #[test]
    fn bookmarks_fetch_all_pages_with_auth_and_preserved_row_metadata() {
        let mut first = bookmark_rows(500);
        first[0] = serde_json::json!({
            "block_id": "bookmark-0", "title": "Document", "document_id": "document-1",
            "content_type": "file", "updated_at": "2026-10-25T10:00:00Z",
        });
        first[17] = serde_json::json!({ "title": "malformed row without identity" });
        let second = vec![
            serde_json::json!({ "block_id": "bookmark-500", "title": " ", "original_filename": "photo.png", "content_type": "file" }),
            serde_json::json!({ "block_id": "bookmark-501" }),
        ];
        let server = BookmarkPageServer::new(vec![(200, bookmark_page(first)), (200, bookmark_page(second))], false);
        let rows = server.fetch().expect("fetch the complete pins traversal");
        assert_eq!(rows.len(), 501, "only the malformed row is skipped, not a later page");
        assert_eq!(rows[0].title, "Document");
        assert_eq!(rows[0].kind, "document");
        assert_eq!(rows[0].document_id.as_deref(), Some("document-1"));
        assert_eq!(rows[0].updated_at.as_deref(), Some("2026-10-25T10:00:00Z"));
        assert_eq!(rows[499].block_id, "bookmark-500");
        assert_eq!(rows[499].title, "photo.png");
        assert_eq!(rows[499].kind, "file");
        assert_eq!(rows[500].title, "bookmark-501");
        let requests = server.finish();
        assert_eq!(requests.len(), 2, "the short second page terminates traversal");
        assert!(requests[0].lines().next().unwrap().contains("limit=500&offset=0"));
        assert!(requests[1].lines().next().unwrap().contains("limit=500&offset=500"));
        for request in requests {
            let headers = request.to_ascii_lowercase();
            assert!(headers.contains(&format!("x-hsk-session-token: {}", "a".repeat(64))));
            assert!(headers.contains(&format!("x-hsk-channel-binding-token: {}", "b".repeat(64))));
        }
    }

    #[test]
    fn bookmarks_exact_page_ends_on_empty_next_page() {
        let server = BookmarkPageServer::new(vec![
            (200, bookmark_page(bookmark_rows(500))), (200, bookmark_page(Vec::new())),
        ], false);
        assert_eq!(server.fetch().unwrap().len(), 500);
        let requests = server.finish();
        assert_eq!(requests.len(), 2);
        assert!(requests[1].lines().next().unwrap().contains("offset=500"));
    }

    #[test]
    fn bookmarks_later_failure_or_duplicate_rejects_the_whole_load() {
        for (status, second, expected) in [
            (503, serde_json::json!({ "error": "unavailable" }), "503"),
            (200, bookmark_page(bookmark_rows(1)), "duplicate"),
        ] {
            let server = BookmarkPageServer::new(vec![
                (200, bookmark_page(bookmark_rows(500))), (status, second),
            ], false);
            let error = server.fetch().expect_err("partial or repeated pages cannot become a complete tree");
            assert!(error.to_string().contains(expected));
            assert_eq!(server.finish().len(), 2);
        }
    }

    #[test]
    fn bookmarks_account_retirement_during_response_rejects_the_load() {
        use std::future::{poll_fn, Future};
        use std::task::Poll;

        let mut server = BookmarkPageServer::new(vec![(200, bookmark_page(bookmark_rows(1)))], true);
        let account = server.account.clone();
        let url = format!("{}/workspaces/workspace-1/loom/views/pins?limit=500&offset=0", server.base);
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let result = runtime.block_on(async {
            let client = reqwest::Client::new();
            let response = crate::local_account::AuthenticatedRequest::new(
                client.clone(), Some(account.clone()),
                client.get(&url).timeout(std::time::Duration::from_secs(5)),
            ).send().await.expect("headers complete while account is active");
            assert!(account.is_active(), "the send-stage guard already accepted this account");
            let decoding = decode_bookmark_page(response, Some(&account));
            tokio::pin!(decoding);
            poll_fn(|context| match decoding.as_mut().poll(context) {
                Poll::Pending => Poll::Ready(()),
                Poll::Ready(_) => panic!("the response body must still be held"),
            }).await;
            account.invalidate();
            server.release_body();
            decoding.await
        });
        assert!(matches!(result, Err(AppError::Http(message)) if message.contains("no longer active")),
            "only the post-body guard can reject this retired account after send succeeded");
        assert_eq!(server.finish().len(), 1);
    }

    #[test]
    fn stable_part_strips_to_safe_chars() {
        assert_eq!(stable_part("Doc 1/Sub"), "doc-1-sub");
        assert_eq!(stable_part("UPPER_case"), "upper-case");
        assert_eq!(stable_part("ws-123"), "ws-123");
        assert_eq!(stable_part("  spaced  "), "spaced");
        // All-punctuation falls back to a stable hash, never empty.
        let h = stable_part("///");
        assert!(
            h.starts_with("id-"),
            "all-punctuation slug falls back to a hash: {h}"
        );
        assert_eq!(
            stable_part("///"),
            stable_part("///"),
            "fallback hash is stable"
        );
    }

    #[test]
    fn stable_parts_do_not_collide_for_distinct_ids() {
        // 1000 distinct ids -> 1000 distinct slugs (red-team CONTROL: no two docs share a stable_part).
        let mut seen = std::collections::HashSet::new();
        for i in 0..1000 {
            let id = format!("doc-{i}");
            assert!(seen.insert(stable_part(&id)), "slug for {id} collided");
        }
        assert_eq!(seen.len(), 1000);
    }

    #[test]
    fn set_content_populates_lists_and_clears_loading() {
        let mut tree = ProjectTree::new();
        tree.set_content(
            vec![
                DocumentSummary::new("d1", "Foo"),
                DocumentSummary::new("d2", "Bar"),
            ],
            vec![CanvasSummary::new("c1", "Sketch")],
        );
        assert_eq!(tree.documents().len(), 2);
        assert_eq!(tree.canvases().len(), 1);
        assert!(!tree.is_loading());
        assert!(tree.error().is_none());
    }

    #[test]
    fn remove_document_drops_the_row() {
        let mut tree = ProjectTree::new();
        tree.set_content(
            vec![
                DocumentSummary::new("d1", "Foo"),
                DocumentSummary::new("d2", "Bar"),
            ],
            vec![],
        );
        assert!(tree.remove_document("d1"));
        assert_eq!(tree.documents().len(), 1);
        assert_eq!(tree.documents()[0].id, "d2");
        assert!(
            !tree.remove_document("nope"),
            "removing a missing id is a no-op"
        );
    }

    #[test]
    fn remove_canvas_drops_the_row() {
        let mut tree = ProjectTree::new();
        tree.set_content(vec![], vec![CanvasSummary::new("c1", "Sketch")]);
        assert!(tree.remove_canvas("c1"));
        assert!(tree.canvases().is_empty());
    }

    #[test]
    fn bookmark_kind_matches_react_helper() {
        // A pin with an owning document_id is a "document".
        assert_eq!(bookmark_kind(Some("doc-1"), "note"), "document");
        // An empty/blank document_id is treated as absent.
        assert_eq!(bookmark_kind(Some("  "), "file"), "file");
        // file/tag_hub/journal content types pass through verbatim.
        assert_eq!(bookmark_kind(None, "file"), "file");
        assert_eq!(bookmark_kind(None, "tag_hub"), "tag_hub");
        assert_eq!(bookmark_kind(None, "journal"), "journal");
        // Everything else (note, canvas, view_def, unknown) is "block".
        assert_eq!(bookmark_kind(None, "note"), "block");
        assert_eq!(bookmark_kind(None, "canvas"), "block");
        assert_eq!(bookmark_kind(None, ""), "block");
    }

    #[test]
    fn set_content_with_bookmarks_populates_and_remove_bookmark_drops_row() {
        let mut tree = ProjectTree::new();
        tree.set_content_with_bookmarks(
            vec![DocumentSummary::new("d1", "Foo")],
            vec![],
            vec![
                BookmarkSummary::new("b1", "Pinned Doc", "document", Some("d1".to_owned())),
                BookmarkSummary::new("b2", "Pinned Note", "block", None),
            ],
        );
        assert_eq!(tree.bookmarks().len(), 2);
        assert!(tree.remove_bookmark("b1"));
        assert_eq!(tree.bookmarks().len(), 1);
        assert_eq!(tree.bookmarks()[0].block_id, "b2");
        assert!(
            !tree.remove_bookmark("nope"),
            "removing a missing bookmark is a no-op"
        );
    }

    #[test]
    fn bookmarks_container_id_in_fresh_band() {
        assert_eq!(BOOKMARKS_NODE_ID, 91);
        // Above the quick-links container (90), below the pane id base (100).
        const { assert!(BOOKMARKS_NODE_ID > PROJECT_TREE_NODE_ID) };
        const { assert!(BOOKMARKS_NODE_ID < crate::accessibility::PANE_NODE_ID_BASE) };
    }

    #[test]
    fn stale_load_result_is_discarded() {
        // Simulate: load_id advances past a delivered result -> the result must be ignored.
        let mut tree = ProjectTree::new();
        let (tx, rx) = std::sync::mpsc::channel();
        tree.rx = Some(rx);
        tree.load_id = 5; // current
                          // Deliver a result tagged with an OLD load_id (3): it must not overwrite the lists.
        tx.send(LoadResult {
            load_id: 3,
            payload: Ok((vec![DocumentSummary::new("old", "Old")], vec![], vec![])),
        })
        .unwrap();
        tree.poll();
        assert!(
            tree.documents().is_empty(),
            "stale result must be discarded"
        );

        // Now deliver a matching result (5): it is applied.
        tx.send(LoadResult {
            load_id: 5,
            payload: Ok((vec![DocumentSummary::new("new", "New")], vec![], vec![])),
        })
        .unwrap();
        tree.poll();
        assert_eq!(tree.documents().len(), 1);
        assert_eq!(tree.documents()[0].id, "new");
    }
}
