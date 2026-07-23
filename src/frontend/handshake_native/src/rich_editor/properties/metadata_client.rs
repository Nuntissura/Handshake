//! Async backend transport for the document properties panel (WP-KERNEL-012 MT-017).
//!
//! ## Why a backend TRAIT (reuse the proven MT-014/MT-015 pattern, do not fork it)
//!
//! MT-014's `AssetMetadataFetcher` and MT-015's `WikilinkBackend` proved the right shape for this
//! crate: a `Send + Sync` trait returning boxed futures (NOT `async-trait`, so ZERO new dependency
//! families), a production impl wrapping the existing consolidated `/knowledge/documents/*` client
//! (including the required `x-hsk-*` document headers), and a COUNTED in-memory mock for the unit tests.
//! This module reuses that exact pattern for the THREE MT-017 backend bindings, anchored to the VERIFIED
//! real backend
//! (`src/backend/handshake_core/src/api/knowledge_documents.rs`):
//!   - title rename: `POST /knowledge/documents/{id}/rename` with `{ title }` -> `{ document, … }`
//!     (verified `RenameBody { title }` + the `move`/`rename` handlers return the updated document),
//!   - project/folder move: `POST /knowledge/documents/{id}/move` with `{ project_ref?, folder_ref? }`
//!     (verified `MoveBody`: absent = unchanged, explicit null = clear, string = set),
//!   - metadata load: `GET /knowledge/documents/{id}` -> `RichDocLoad.document` ([`DocMetadata`]),
//!   - backlinks count: `GET /knowledge/documents/{id}/backlinks` -> the length of `backlinks`.
//!
//! ## Backend reuse only (no backend edits — typed blocker if a gap)
//!
//! Every endpoint above ALREADY exists in `handshake_core` (proven by the route table in
//! `knowledge_documents.rs` and the React `api.ts` client). This module only CONSUMES them; a missing
//! endpoint / failure is a typed [`MetadataError`] (a visible banner), never a backend edit.
//!
//! ## The MT contract's `/save`-for-title assumption was WRONG (verified correction)
//!
//! The contract said title saves through `PUT /save` with the full content_json. The REAL `/save`
//! handler (`SaveDocumentBody { expected_version, content_json }`) does NOT accept a title; the title
//! lives behind the dedicated `/rename` route. Binding `/rename` is the correct, persisting path AND
//! it sidesteps the contract's MC-001 stale-content-clobber risk entirely: a rename never sends a
//! content body, so it cannot overwrite the operator's live edits.

use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use serde::Deserialize;
use thiserror::Error;

/// The verified mirror of the backend `RichDocument` (`app/src/lib/api.ts` lines 3028-3048 +
/// `knowledge_documents.rs`). Only the fields the properties panel displays are kept; `#[serde(default)]`
/// on the optionals so a forward-compatible backend body still deserializes.
///
/// NOTE the ABSENT `tags` field: the real `RichDocument` has none, and the knowledge-document API has no
/// tag endpoint, so the panel keeps a LOCAL-ONLY tag list + a visible backend-gap banner (MC-002) rather
/// than fabricating a `tags` field here.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct DocMetadata {
    /// The stable rich-document id (the read-only monospace, click-to-copy field).
    pub rich_document_id: String,
    /// The workspace the document lives in.
    pub workspace_id: String,
    /// The editable title (the persisted value; the edit buffer lives in `PropertiesState`).
    pub title: String,
    /// The monotonic document version (`#version` badge).
    pub doc_version: u64,
    /// The authority label badge (`promoted` / `draft`).
    pub authority_label: String,
    /// The owning actor kind (read-only).
    #[serde(default)]
    pub owner_actor_kind: Option<String>,
    /// The owning actor id (read-only).
    #[serde(default)]
    pub owner_actor_id: Option<String>,
    /// The project association (editable via `/move`).
    #[serde(default)]
    pub project_ref: Option<String>,
    /// The folder association (editable via `/move`).
    #[serde(default)]
    pub folder_ref: Option<String>,
    /// The CRDT document id (read-only; displayed if present).
    #[serde(default)]
    pub crdt_document_id: Option<String>,
    /// The ISO-8601 creation timestamp (rendered local).
    pub created_at: String,
    /// The ISO-8601 last-update timestamp (rendered local; refreshed after each save).
    pub updated_at: String,
}

/// The typed reasons a metadata backend interaction failed. Each variant renders as a VISIBLE banner
/// (fail-closed, never blank, never a panic). `kind_str` is a stable kebab-case token the error UI +
/// AccessKit label carry, so an out-of-process agent reads a stable failure vocabulary (same shape as
/// MT-015's `WikilinkError`).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MetadataError {
    /// The title was empty — the backend `/rename` rejects an empty title (validated client-side too).
    #[error("title must be non-empty")]
    EmptyTitle,
    /// The document was not found (HTTP 404).
    #[error("not found: {0}")]
    NotFound(String),
    /// The target is not accessible (HTTP 401/403).
    #[error("not accessible: {0}")]
    Forbidden(String),
    /// The backend returned 5xx / a malformed body.
    #[error("server error: {0}")]
    ServerError(String),
    /// The fetch itself failed (backend unreachable / transport error).
    #[error("network error: {0}")]
    NetworkError(String),
}

impl MetadataError {
    /// Stable kebab-case kind token (the banner text + AccessKit label vocabulary).
    pub fn kind_str(&self) -> &'static str {
        match self {
            MetadataError::EmptyTitle => "empty_title",
            MetadataError::NotFound(_) => "not_found",
            MetadataError::Forbidden(_) => "forbidden",
            MetadataError::ServerError(_) => "server_error",
            MetadataError::NetworkError(_) => "network_error",
        }
    }
}

/// A boxed `Send` future yielding a Result, returned by the [`KnowledgeMetadataBackend`] methods.
/// Spelled out (not `async-trait`) so this module adds ZERO new dependency families — the same
/// boxed-future pattern MT-014/MT-015 use.
pub type MetadataFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, MetadataError>> + Send + 'a>>;

/// The backend transport for the MT-017 bindings. A trait (not hard reqwest calls) so the save / load /
/// backlinks-count logic is unit-testable with a counted mock and NO backend. The production impl
/// ([`ReqwestMetadataBackend`]) wraps the existing reqwest stack against the VERIFIED routes.
pub trait KnowledgeMetadataBackend: Send + Sync {
    /// Rename: `POST /knowledge/documents/{id}/rename` with `{ title }`. Returns the updated document
    /// (refreshed `title` + `updated_at`). This is the REAL title-update path (NOT `/save`).
    fn rename<'a>(
        &'a self,
        document_id: &'a str,
        title: &'a str,
    ) -> MetadataFuture<'a, DocMetadata>;

    /// Move: `POST /knowledge/documents/{id}/move` with `{ project_ref?, folder_ref? }`. An absent
    /// (`None`) field leaves that membership unchanged; an explicit `Some(None)` clears it; `Some(Some)`
    /// sets it (the verified absent-vs-null backend semantics). Returns the updated document.
    fn move_doc<'a>(
        &'a self,
        document_id: &'a str,
        project_ref: Option<Option<String>>,
        folder_ref: Option<Option<String>>,
    ) -> MetadataFuture<'a, DocMetadata>;

    /// Load: `GET /knowledge/documents/{id}` -> the document metadata (the panel's source of truth).
    fn load<'a>(&'a self, document_id: &'a str) -> MetadataFuture<'a, DocMetadata>;

    /// Backlinks count: `GET /knowledge/documents/{id}/backlinks` -> the count of linking documents.
    fn backlinks_count<'a>(&'a self, document_id: &'a str) -> MetadataFuture<'a, usize>;
}

/// The production [`KnowledgeMetadataBackend`]: a thin wrapper over the consolidated
/// [`crate::backend::knowledge_documents::KnowledgeDocumentsClient`] against the verified backend
/// endpoints, mapping typed document-client errors to the panel's [`MetadataError`] vocabulary. REUSES
/// the existing reqwest 0.12 + rustls stack and the required `x-hsk-*` identity headers from
/// `backend_client` / `backend::knowledge_documents` — NO new HTTP crate and NO duplicate header path.
/// Read/write of the EXISTING knowledge-document API only; no backend code is touched.
#[derive(Clone)]
pub struct ReqwestMetadataBackend {
    client: crate::backend::knowledge_documents::KnowledgeDocumentsClient,
    base_url: String,
    session_run_id: String,
}

impl ReqwestMetadataBackend {
    /// Build a backend client against `base_url` (e.g. `backend_client::BACKEND_BASE_URL`).
    pub fn new(base_url: impl Into<String>) -> Self {
        let base_url = base_url.into();
        Self {
            client: crate::backend::knowledge_documents::KnowledgeDocumentsClient::with_base_url(
                base_url.clone(),
            ),
            base_url,
            session_run_id: crate::rich_editor::save::save_manager::new_session_run_id(),
        }
    }

    /// The production client against the hardcoded backend base URL.
    pub fn production() -> Self {
        Self {
            client: crate::backend::knowledge_documents::KnowledgeDocumentsClient::production(),
            base_url: crate::backend_client::BACKEND_BASE_URL.to_owned(),
            session_run_id: crate::rich_editor::save::save_manager::new_session_run_id(),
        }
    }

    /// The REST base this client talks to.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    fn read_headers(
        &self,
        document_id: &str,
    ) -> crate::backend::knowledge_documents::HskDocumentHeaders {
        crate::backend::knowledge_documents::HskDocumentHeaders::for_read(
            self.session_run_id.clone(),
            document_id,
        )
    }

    fn write_headers(
        &self,
        document_id: &str,
    ) -> crate::backend::knowledge_documents::HskDocumentHeaders {
        crate::backend::knowledge_documents::HskDocumentHeaders::for_operator(
            self.session_run_id.clone(),
            document_id,
        )
    }
}

fn document_client_error_to_metadata_error(
    error: crate::backend::knowledge_documents::KnowledgeDocumentsError,
) -> MetadataError {
    use crate::backend::knowledge_documents::KnowledgeDocumentsError;
    match error {
        KnowledgeDocumentsError::NotFound(detail) => MetadataError::NotFound(detail),
        KnowledgeDocumentsError::Forbidden(reason) => MetadataError::Forbidden(reason),
        KnowledgeDocumentsError::Transport(e) => MetadataError::NetworkError(e),
        KnowledgeDocumentsError::BadRequest(detail) => MetadataError::ServerError(detail),
        KnowledgeDocumentsError::Server(detail) => MetadataError::ServerError(detail),
        KnowledgeDocumentsError::UnexpectedStatus { status, body } => {
            MetadataError::ServerError(format!("unexpected HTTP {status}: {body}"))
        }
        KnowledgeDocumentsError::Parse(e) => MetadataError::ServerError(e),
        KnowledgeDocumentsError::SaveConflict { server_version } => MetadataError::ServerError(
            format!("metadata write conflicted (server_version={server_version:?})"),
        ),
        KnowledgeDocumentsError::TitleAmbiguous { detail } => {
            MetadataError::ServerError(format!("ambiguous title: {detail}"))
        }
        KnowledgeDocumentsError::BatchTooLarge { len, max } => {
            MetadataError::ServerError(format!("batch too large: {len} > {max}"))
        }
        KnowledgeDocumentsError::BatchEmpty => MetadataError::ServerError("batch empty".into()),
    }
}

fn doc_metadata_from_value(
    value: serde_json::Value,
    what: &str,
) -> Result<DocMetadata, MetadataError> {
    serde_json::from_value(value)
        .map_err(|e| MetadataError::ServerError(format!("{what} body invalid: {e}")))
}

impl KnowledgeMetadataBackend for ReqwestMetadataBackend {
    fn rename<'a>(
        &'a self,
        document_id: &'a str,
        title: &'a str,
    ) -> MetadataFuture<'a, DocMetadata> {
        let client = self.client.clone();
        let headers = self.write_headers(document_id);
        let document_id = document_id.to_owned();
        let title = title.trim().to_owned();
        Box::pin(async move {
            if title.is_empty() {
                return Err(MetadataError::EmptyTitle);
            }
            let body = crate::backend::knowledge_documents::RenameDocumentRequest {
                title,
                expected_updated_at: None,
            };
            let response = client
                .rename_document(&headers, &document_id, &body)
                .await
                .map_err(document_client_error_to_metadata_error)?;
            doc_metadata_from_value(response.document, "rename")
        })
    }

    fn move_doc<'a>(
        &'a self,
        document_id: &'a str,
        project_ref: Option<Option<String>>,
        folder_ref: Option<Option<String>>,
    ) -> MetadataFuture<'a, DocMetadata> {
        let client = self.client.clone();
        let headers = self.write_headers(document_id);
        let document_id = document_id.to_owned();
        Box::pin(async move {
            let body = crate::backend::knowledge_documents::MoveDocumentRequest {
                project_ref,
                folder_ref,
            };
            let response = client
                .move_document(&headers, &document_id, &body)
                .await
                .map_err(document_client_error_to_metadata_error)?;
            doc_metadata_from_value(response.document, "move")
        })
    }

    fn load<'a>(&'a self, document_id: &'a str) -> MetadataFuture<'a, DocMetadata> {
        let client = self.client.clone();
        let headers = self.read_headers(document_id);
        let document_id = document_id.to_owned();
        Box::pin(async move {
            let response = client
                .load_document(&headers, &document_id)
                .await
                .map_err(document_client_error_to_metadata_error)?;
            doc_metadata_from_value(response.document, "load")
        })
    }

    fn backlinks_count<'a>(&'a self, document_id: &'a str) -> MetadataFuture<'a, usize> {
        let client = self.client.clone();
        let headers = self.read_headers(document_id);
        let document_id = document_id.to_owned();
        Box::pin(async move {
            let response = client
                .list_backlinks(&headers, &document_id)
                .await
                .map_err(document_client_error_to_metadata_error)?;
            match response.backlinks {
                serde_json::Value::Array(rows) => Ok(rows.len()),
                serde_json::Value::Null => Ok(0),
                other => Err(MetadataError::ServerError(format!(
                    "backlinks-count body invalid: expected array, got {other}"
                ))),
            }
        })
    }
}

/// The state of the title-save dispatch (the rename round-trip). Drives the title-field affordance: a
/// `Saving` state shows a non-animating "Saving…" hint (NEVER a perpetual Spinner — the impl-note
/// gate), `Saved` shows nothing, `Failed` shows the typed banner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveState {
    /// No save in flight / nothing saved yet.
    Idle,
    /// A rename is dispatched and in flight.
    Saving,
    /// The last save succeeded.
    Saved,
    /// The last save failed with a typed error (banner).
    Failed(MetadataError),
}

/// The state of the backlinks-count fetch (a SEPARATE async task that never touches the doc metadata —
/// MC-004). The count is shown as `↑ N backlinks`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BacklinksCountState {
    /// No fetch issued yet (the count chip shows nothing — NOT a spinner, the impl-note gate).
    Idle,
    /// A count fetch is in flight (only ever entered when a runtime can dispatch it).
    Loading,
    /// The count loaded.
    Loaded(usize),
    /// The count fetch failed (a small, non-fatal "backlinks ?" — the panel still renders everything).
    Failed(MetadataError),
}

/// FIFO delivery queue for off-thread metadata mutation (rename/move) results, tagged with the mutation
/// generation + document id each request was issued for so stale responses can be dropped without
/// overwriting a fresher queued result.
type MutationDeliveryQueue = VecDeque<(u64, String, Result<DocMetadata, MetadataError>)>;
type MutationDeliveryCell = Arc<Mutex<MutationDeliveryQueue>>;

/// One-slot delivery cell for an off-thread backlinks-count fetch, tagged with the generation it was
/// issued for (MC-004 cancellation): `(generation, result)`.
type CountDeliveryCell = Arc<Mutex<Option<(u64, Result<usize, MetadataError>)>>>;

/// The per-editor properties runtime (owned by `RichEditorState`, mirroring `WikilinkRuntime`). Holds
/// the backend transport, the tokio handle, the title-save state + its delivery cell, and the
/// backlinks-count state + generation + its SEPARATE delivery cell (MC-004: the count NEVER overwrites
/// the doc metadata struct — it lands only in the dedicated count cell/state).
pub struct PropertiesRuntime {
    /// The backend transport (production reqwest; tests: a counted mock).
    pub backend: Arc<dyn KnowledgeMetadataBackend>,
    /// The tokio handle dispatches spawn onto (`None` in headless tests — no spinner, see below).
    pub runtime: Option<tokio::runtime::Handle>,
    /// The current document id (drives the backlinks-count fetch; a change bumps the generation).
    pub document_id: String,
    /// The title-save (rename) dispatch state.
    pub save_state: SaveState,
    /// The monotonic rename/move generation; bumped on each metadata mutation dispatch and document
    /// switch so stale same-document and prior-document mutation responses are dropped.
    pub mutation_generation: u64,
    /// The backlinks-count state (a SEPARATE async task — MC-004).
    pub backlinks_count: BacklinksCountState,
    /// The monotonic backlinks-count generation; bumped on document change so a stale response is
    /// dropped (MC-004) and never overwrites a newer document's count.
    pub count_generation: u64,
    mutation_cell: MutationDeliveryCell,
    count_cell: CountDeliveryCell,
}

impl PropertiesRuntime {
    /// Build a runtime over `backend`, spawning onto `runtime` (pass `None` for a headless test).
    pub fn new(
        backend: Arc<dyn KnowledgeMetadataBackend>,
        runtime: Option<tokio::runtime::Handle>,
    ) -> Self {
        Self {
            backend,
            runtime,
            document_id: String::new(),
            save_state: SaveState::Idle,
            mutation_generation: 0,
            backlinks_count: BacklinksCountState::Idle,
            count_generation: 0,
            mutation_cell: Arc::new(Mutex::new(VecDeque::new())),
            count_cell: Arc::new(Mutex::new(None)),
        }
    }

    /// A headless runtime (no tokio handle) over `backend` — the test/seed constructor.
    pub fn headless(backend: Arc<dyn KnowledgeMetadataBackend>) -> Self {
        Self::new(backend, None)
    }

    /// Set the active document id. When it CHANGES, bump the count generation (MC-004), reset the
    /// backlinks-count to `Idle`, and reset the save state. A no-op when unchanged.
    pub fn set_document(&mut self, document_id: impl Into<String>) {
        let document_id = document_id.into();
        if document_id == self.document_id {
            return;
        }
        self.document_id = document_id;
        self.mutation_generation = self.mutation_generation.wrapping_add(1);
        self.count_generation = self.count_generation.wrapping_add(1);
        self.backlinks_count = BacklinksCountState::Idle;
        self.save_state = SaveState::Idle;
    }

    /// Clear the active document binding unconditionally. Used when the shell invalidates or switches
    /// documents before the next metadata load lands, so stale save/count state cannot remain visible.
    pub fn clear_document(&mut self) {
        self.document_id.clear();
        self.mutation_generation = self.mutation_generation.wrapping_add(1);
        self.count_generation = self.count_generation.wrapping_add(1);
        self.backlinks_count = BacklinksCountState::Idle;
        self.save_state = SaveState::Idle;
    }

    /// Dispatch a title rename for `document_id`. Marks `Saving` and spawns the rename; the result lands
    /// in the mutation cell (drained next frame). A no-op (records `Failed(EmptyTitle)`) for a blank
    /// title. Without a runtime (headless) it leaves state unchanged rather than entering a perpetual
    /// `Saving` that nothing resolves (no-spinner discipline); tests stage results directly.
    pub fn dispatch_rename(&mut self, document_id: &str, title: &str) {
        let title = title.trim().to_owned();
        if title.is_empty() {
            self.save_state = SaveState::Failed(MetadataError::EmptyTitle);
            return;
        }
        if document_id != self.document_id {
            return;
        }
        let Some(runtime) = self.runtime.clone() else {
            // Headless: do not enter a perpetual Saving (nothing would resolve it). The unit tests
            // stage the result via `stage_mutation`; production always has a runtime.
            return;
        };
        self.mutation_generation = self.mutation_generation.wrapping_add(1);
        let generation = self.mutation_generation;
        self.save_state = SaveState::Saving;
        let backend = Arc::clone(&self.backend);
        let cell = Arc::clone(&self.mutation_cell);
        let document_id = document_id.to_owned();
        runtime.spawn(async move {
            let result = backend.rename(&document_id, &title).await;
            if let Ok(mut slot) = cell.lock() {
                slot.push_back((generation, document_id, result));
            }
        });
    }

    /// Dispatch a project/folder move for `document_id`. `None` means leave that ref unchanged,
    /// `Some(None)` clears it, and `Some(Some(v))` sets it. The result lands in the same mutation cell as
    /// rename so the host applies the refreshed metadata and updated timestamp on the next frame.
    pub fn dispatch_move(
        &mut self,
        document_id: &str,
        project_ref: Option<Option<String>>,
        folder_ref: Option<Option<String>>,
    ) {
        if project_ref.is_none() && folder_ref.is_none() {
            return;
        }
        if document_id != self.document_id {
            return;
        }
        let Some(runtime) = self.runtime.clone() else {
            // Headless: do not enter a perpetual Saving state; tests can stage or use a real runtime.
            return;
        };
        self.mutation_generation = self.mutation_generation.wrapping_add(1);
        let generation = self.mutation_generation;
        self.save_state = SaveState::Saving;
        let backend = Arc::clone(&self.backend);
        let cell = Arc::clone(&self.mutation_cell);
        let document_id = document_id.to_owned();
        runtime.spawn(async move {
            let result = backend
                .move_doc(&document_id, project_ref, folder_ref)
                .await;
            if let Ok(mut slot) = cell.lock() {
                slot.push_back((generation, document_id, result));
            }
        });
    }

    /// Trigger a backlinks-count fetch for the current document (on load / explicit refresh — NOT per
    /// frame). Bumps the generation, marks `Loading`, and spawns the fetch into the dedicated count cell
    /// (MC-004: it NEVER writes the doc metadata). A no-op when the document id is empty or there is no
    /// runtime (headless stays `Idle` — no perpetual spinner).
    pub fn refresh_backlinks_count(&mut self) {
        if self.document_id.trim().is_empty() {
            return;
        }
        let Some(runtime) = self.runtime.clone() else {
            return;
        };
        self.count_generation = self.count_generation.wrapping_add(1);
        self.backlinks_count = BacklinksCountState::Loading;
        let generation = self.count_generation;
        let backend = Arc::clone(&self.backend);
        let cell = Arc::clone(&self.count_cell);
        let document_id = self.document_id.clone();
        runtime.spawn(async move {
            let result = backend.backlinks_count(&document_id).await;
            if let Ok(mut slot) = cell.lock() {
                *slot = Some((generation, result));
            }
        });
    }

    /// Fetch the backlinks count ONCE on document load (Idle -> Loading); the "fetch on load, not every
    /// frame" guard (RISK-4 / no-spinner discipline).
    pub fn ensure_backlinks_count_loaded(&mut self) {
        if matches!(self.backlinks_count, BacklinksCountState::Idle) {
            self.refresh_backlinks_count();
        }
    }

    /// Drain any off-thread mutation (rename/move) + backlinks-count results since the last frame.
    /// Returns the freshly-completed mutation result (so the host applies the new metadata to
    /// `PropertiesState`) plus a bool that is true when ANYTHING was applied (the caller can request a
    /// repaint). MC-004: a count result whose generation no longer matches is DROPPED, and the count
    /// only ever lands in `backlinks_count` — never the doc metadata.
    pub fn drain(&mut self) -> (Option<DocMetadata>, bool) {
        let mut applied = false;
        let mut fresh_metadata: Option<DocMetadata> = None;

        let mutation_deliveries = if let Ok(mut slot) = self.mutation_cell.lock() {
            slot.drain(..).collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        for (generation, document_id, result) in mutation_deliveries {
            if generation == self.mutation_generation && document_id == self.document_id {
                match result {
                    Ok(meta) => {
                        if meta.rich_document_id == self.document_id {
                            self.save_state = SaveState::Saved;
                            fresh_metadata = Some(meta);
                            applied = true;
                        } else {
                            self.save_state =
                                SaveState::Failed(MetadataError::ServerError(format!(
                                    "metadata mutation returned document {} for active document {}",
                                    meta.rich_document_id, self.document_id
                                )));
                            applied = true;
                        }
                    }
                    Err(e) => {
                        self.save_state = SaveState::Failed(e);
                        applied = true;
                    }
                }
            }
            // else: a stale mutation landed late -> dropped.
        }
        if let Ok(mut slot) = self.count_cell.lock() {
            if let Some((generation, result)) = slot.take() {
                if generation == self.count_generation {
                    self.backlinks_count = match result {
                        Ok(n) => BacklinksCountState::Loaded(n),
                        Err(e) => BacklinksCountState::Failed(e),
                    };
                    applied = true;
                }
                // else: a stale (older-generation) count landed late -> dropped (MC-004).
            }
        }
        (fresh_metadata, applied)
    }

    // ── Test seams (headless: stage a delivery without a tokio runtime) ──────────────────────────

    /// Stage a mutation (rename/move) delivery into the cell (test seam).
    #[cfg(test)]
    pub fn stage_mutation(&self, result: Result<DocMetadata, MetadataError>) {
        self.mutation_cell.lock().unwrap().push_back((
            self.mutation_generation,
            self.document_id.clone(),
            result,
        ));
    }

    /// Stage a mutation (rename/move) delivery tagged with an explicit document id (test seam).
    #[cfg(test)]
    pub fn stage_mutation_for_document(
        &self,
        document_id: impl Into<String>,
        result: Result<DocMetadata, MetadataError>,
    ) {
        self.mutation_cell.lock().unwrap().push_back((
            self.mutation_generation,
            document_id.into(),
            result,
        ));
    }

    /// Stage a mutation delivery tagged with explicit generation + document id (test seam).
    #[cfg(test)]
    pub fn stage_mutation_for_generation(
        &self,
        generation: u64,
        document_id: impl Into<String>,
        result: Result<DocMetadata, MetadataError>,
    ) {
        self.mutation_cell
            .lock()
            .unwrap()
            .push_back((generation, document_id.into(), result));
    }

    /// Stage a backlinks-count delivery into the cell tagged with `generation` (test seam).
    #[cfg(test)]
    pub fn stage_count(&self, generation: u64, result: Result<usize, MetadataError>) {
        *self.count_cell.lock().unwrap() = Some((generation, result));
    }
}

/// A mockable clipboard sink so the document-id click-to-copy (AC-6) is unit-testable WITHOUT touching
/// the real OS clipboard in a headless test. The production impl ([`EguiClipboard`]) delegates to the
/// EXISTING `egui::Context::copy_text` surface the shell already uses (app.rs / debug_console.rs) — NO
/// `arboard` direct dependency (the impl-note "reuse any existing WP-011 clipboard surface if present").
pub trait ClipboardSink {
    /// Copy `text` to the clipboard.
    fn copy(&self, text: &str);
}

/// The production clipboard sink: copies through the egui context (the same `ctx.copy_text` call the
/// rest of the app uses). Holds the context by value (egui `Context` is a cheap `Arc` clone).
pub struct EguiClipboard {
    ctx: egui::Context,
}

impl EguiClipboard {
    /// Wrap an egui context as a clipboard sink.
    pub fn new(ctx: egui::Context) -> Self {
        Self { ctx }
    }
}

impl ClipboardSink for EguiClipboard {
    fn copy(&self, text: &str) {
        self.ctx.copy_text(text.to_owned());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta(title: &str) -> DocMetadata {
        DocMetadata {
            rich_document_id: "KRD-1".into(),
            workspace_id: "ws".into(),
            title: title.into(),
            doc_version: 2,
            authority_label: "draft".into(),
            owner_actor_kind: Some("operator".into()),
            owner_actor_id: Some("ilja".into()),
            project_ref: None,
            folder_ref: None,
            crdt_document_id: None,
            created_at: "2026-06-19T14:32:00Z".into(),
            updated_at: "2026-06-19T14:32:00Z".into(),
        }
    }

    fn meta_with_id(document_id: &str, title: &str) -> DocMetadata {
        let mut m = meta(title);
        m.rich_document_id = document_id.to_owned();
        m
    }

    /// A counted mock clipboard sink (AC-6: copy without touching the OS clipboard).
    struct MockClipboard {
        last: Mutex<Option<String>>,
    }
    impl MockClipboard {
        fn new() -> Self {
            Self {
                last: Mutex::new(None),
            }
        }
    }
    impl ClipboardSink for MockClipboard {
        fn copy(&self, text: &str) {
            *self.last.lock().unwrap() = Some(text.to_owned());
        }
    }

    #[test]
    fn doc_metadata_deserializes_real_backend_shape() {
        // The verified RichDocument shape (api.ts lines 3028-3048) round-trips into DocMetadata, and the
        // ABSENT `tags` field does not break deserialization (proving the MC-002 gap is real, not a
        // missing-field bug).
        let json = serde_json::json!({
            "rich_document_id": "KRD-9",
            "workspace_id": "ws1",
            "document_id": "DOC-9",
            "title": "My Doc",
            "schema_version": "rich_document_v1",
            "doc_version": 7,
            "content_json": { "type": "doc", "content": [] },
            "content_sha256": "abc",
            "crdt_document_id": "KCRDT-9",
            "crdt_snapshot_id": null,
            "promotion_receipt_event_id": null,
            "projection_refs": null,
            "project_ref": "PRJ-1",
            "folder_ref": null,
            "authority_label": "promoted",
            "owner_actor_kind": "operator",
            "owner_actor_id": "ilja",
            "created_at": "2026-06-19T14:32:00Z",
            "updated_at": "2026-06-20T08:00:00Z"
        });
        let m: DocMetadata = serde_json::from_value(json).unwrap();
        assert_eq!(m.rich_document_id, "KRD-9");
        assert_eq!(m.title, "My Doc");
        assert_eq!(m.doc_version, 7);
        assert_eq!(m.authority_label, "promoted");
        assert_eq!(m.project_ref.as_deref(), Some("PRJ-1"));
        assert_eq!(m.folder_ref, None);
        assert_eq!(m.crdt_document_id.as_deref(), Some("KCRDT-9"));
    }

    #[test]
    fn error_kind_strings_are_stable() {
        assert_eq!(MetadataError::EmptyTitle.kind_str(), "empty_title");
        assert_eq!(MetadataError::NotFound("x".into()).kind_str(), "not_found");
        assert_eq!(MetadataError::Forbidden("x".into()).kind_str(), "forbidden");
        assert_eq!(
            MetadataError::ServerError("x".into()).kind_str(),
            "server_error"
        );
        assert_eq!(
            MetadataError::NetworkError("x".into()).kind_str(),
            "network_error"
        );
    }

    #[test]
    fn document_client_errors_map_to_metadata_errors() {
        use crate::backend::knowledge_documents::KnowledgeDocumentsError;

        assert_eq!(
            document_client_error_to_metadata_error(KnowledgeDocumentsError::NotFound("x".into())),
            MetadataError::NotFound("x".into())
        );
        assert_eq!(
            document_client_error_to_metadata_error(KnowledgeDocumentsError::Forbidden("x".into()))
                .kind_str(),
            "forbidden"
        );
        assert_eq!(
            document_client_error_to_metadata_error(KnowledgeDocumentsError::Transport("x".into()))
                .kind_str(),
            "network_error"
        );
        assert_eq!(
            document_client_error_to_metadata_error(KnowledgeDocumentsError::Server("x".into()))
                .kind_str(),
            "server_error"
        );
    }

    struct NoopBackend;
    impl KnowledgeMetadataBackend for NoopBackend {
        fn rename<'a>(&'a self, _d: &'a str, t: &'a str) -> MetadataFuture<'a, DocMetadata> {
            let t = t.to_owned();
            Box::pin(async move { Ok(meta(&t)) })
        }
        fn move_doc<'a>(
            &'a self,
            _d: &'a str,
            _p: Option<Option<String>>,
            _f: Option<Option<String>>,
        ) -> MetadataFuture<'a, DocMetadata> {
            Box::pin(async { Ok(meta("moved")) })
        }
        fn load<'a>(&'a self, _d: &'a str) -> MetadataFuture<'a, DocMetadata> {
            Box::pin(async { Ok(meta("loaded")) })
        }
        fn backlinks_count<'a>(&'a self, _d: &'a str) -> MetadataFuture<'a, usize> {
            Box::pin(async { Ok(0) })
        }
    }

    fn rt() -> PropertiesRuntime {
        PropertiesRuntime::headless(Arc::new(NoopBackend))
    }

    #[test]
    fn dispatch_rename_rejects_blank_title_without_runtime() {
        let mut rt = rt();
        rt.dispatch_rename("KRD-1", "   ");
        assert_eq!(rt.save_state, SaveState::Failed(MetadataError::EmptyTitle));
    }

    #[test]
    fn drain_applies_rename_result_to_save_state_and_returns_metadata() {
        // Staging a rename result -> drain sets Saved + returns the fresh metadata (the host applies it
        // to PropertiesState). MC-001 documented: the metadata is the rename response (title-only), so a
        // content body is never involved and cannot clobber the live doc.
        let mut rt = rt();
        rt.set_document("KRD-1");
        rt.stage_mutation(Ok(meta("Renamed Doc")));
        let (fresh, applied) = rt.drain();
        assert!(applied);
        assert_eq!(rt.save_state, SaveState::Saved);
        assert_eq!(fresh.unwrap().title, "Renamed Doc");
    }

    #[test]
    fn drain_applies_rename_failure_to_save_state() {
        let mut rt = rt();
        rt.set_document("KRD-1");
        rt.stage_mutation(Err(MetadataError::ServerError("500".into())));
        let (fresh, applied) = rt.drain();
        assert!(applied);
        assert!(fresh.is_none());
        assert!(matches!(
            rt.save_state,
            SaveState::Failed(MetadataError::ServerError(_))
        ));
    }

    #[test]
    fn mutation_delivery_drops_stale_document_results() {
        // Rename/move responses are tagged with the document id they were issued for. If Doc A's
        // mutation lands after the editor has switched to Doc B, it must not overwrite Doc B metadata.
        let mut rt = rt();
        rt.set_document("KRD-A");
        rt.set_document("KRD-B");
        rt.stage_mutation_for_document("KRD-A", Ok(meta_with_id("KRD-A", "Stale A")));

        let (fresh, applied) = rt.drain();
        assert!(
            !applied,
            "stale Doc A mutation is dropped after switching to Doc B"
        );
        assert!(fresh.is_none());
        assert_eq!(
            rt.save_state,
            SaveState::Idle,
            "dropping a stale mutation does not mark the active Doc B save state"
        );

        rt.stage_mutation_for_document("KRD-B", Ok(meta_with_id("KRD-B", "Fresh B")));
        let (fresh, applied) = rt.drain();
        assert!(applied);
        assert_eq!(fresh.unwrap().title, "Fresh B");
        assert_eq!(rt.save_state, SaveState::Saved);
    }

    #[test]
    fn mutation_delivery_drops_stale_same_document_generation() {
        let mut rt = rt();
        rt.set_document("KRD-1");
        rt.mutation_generation = 7;

        rt.stage_mutation_for_generation(6, "KRD-1", Ok(meta_with_id("KRD-1", "Older")));
        let (fresh, applied) = rt.drain();
        assert!(
            !applied,
            "older same-document metadata mutation generation is dropped"
        );
        assert!(fresh.is_none());

        rt.stage_mutation_for_generation(7, "KRD-1", Ok(meta_with_id("KRD-1", "Latest")));
        let (fresh, applied) = rt.drain();
        assert!(applied);
        assert_eq!(fresh.unwrap().title, "Latest");
    }

    #[test]
    fn mutation_delivery_queue_preserves_current_result_when_stale_lands_after_it() {
        let mut rt = rt();
        rt.set_document("KRD-1");
        rt.mutation_generation = 2;
        rt.save_state = SaveState::Saving;

        rt.stage_mutation_for_generation(2, "KRD-1", Ok(meta_with_id("KRD-1", "Newest")));
        rt.stage_mutation_for_generation(1, "KRD-1", Ok(meta_with_id("KRD-1", "Old")));

        let (fresh, applied) = rt.drain();
        assert!(applied);
        assert_eq!(rt.save_state, SaveState::Saved);
        assert_eq!(
            fresh.unwrap().title,
            "Newest",
            "a late stale delivery must not overwrite or discard the current-generation result"
        );
    }

    #[test]
    fn mutation_delivery_wrong_return_document_fails_active_save() {
        let mut rt = rt();
        rt.set_document("KRD-1");
        rt.mutation_generation = 3;
        rt.save_state = SaveState::Saving;

        rt.stage_mutation_for_generation(
            3,
            "KRD-1",
            Ok(meta_with_id("KRD-OTHER", "Wrong document")),
        );

        let (fresh, applied) = rt.drain();
        assert!(applied);
        assert!(fresh.is_none());
        match &rt.save_state {
            SaveState::Failed(MetadataError::ServerError(message)) => {
                assert!(message.contains("KRD-OTHER"));
                assert!(message.contains("KRD-1"));
            }
            other => panic!("wrong returned document must fail the active save, got {other:?}"),
        }
    }

    #[test]
    fn dispatch_mutations_ignore_non_current_document_without_saving() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let mut rt = PropertiesRuntime::new(Arc::new(NoopBackend), Some(runtime.handle().clone()));
        rt.set_document("KRD-1");
        let generation = rt.mutation_generation;

        rt.dispatch_rename("KRD-OTHER", "Wrong document");
        assert_eq!(rt.save_state, SaveState::Idle);
        assert_eq!(
            rt.mutation_generation, generation,
            "wrong-document rename must not bump the mutation generation"
        );

        rt.dispatch_move(
            "KRD-OTHER",
            Some(Some("project-x".into())),
            Some(Some("folder-x".into())),
        );
        assert_eq!(rt.save_state, SaveState::Idle);
        assert_eq!(
            rt.mutation_generation, generation,
            "wrong-document move must not bump the mutation generation"
        );
    }

    #[test]
    fn backlinks_count_generation_cancels_stale_response_mc004() {
        // MC-004: switching documents bumps the count generation; an older document's count response
        // that lands late is dropped, and the count NEVER touches the doc metadata.
        let mut rt = rt();
        rt.set_document("DOC-A");
        let gen_a = rt.count_generation;
        rt.backlinks_count = BacklinksCountState::Loading;
        rt.set_document("DOC-B");
        let gen_b = rt.count_generation;
        assert_ne!(gen_a, gen_b);

        // DOC-A's stale count lands -> dropped.
        rt.stage_count(gen_a, Ok(5));
        let (fresh, applied) = rt.drain();
        assert!(!applied, "MC-004: a stale-generation count is dropped");
        assert!(fresh.is_none(), "a count drain never produces doc metadata");
        assert!(matches!(rt.backlinks_count, BacklinksCountState::Idle));

        // DOC-B's count lands -> applied.
        rt.stage_count(gen_b, Ok(3));
        let (_fresh, applied) = rt.drain();
        assert!(applied);
        assert_eq!(rt.backlinks_count, BacklinksCountState::Loaded(3));
    }

    #[test]
    fn ensure_backlinks_count_stays_idle_without_runtime() {
        // Headless (no runtime) must NOT enter Loading: nothing would resolve it, so a Loading-state
        // spinner would repaint forever (idle-CPU + harness max_steps). It stays Idle.
        let mut rt = rt();
        rt.set_document("DOC-A");
        let gen = rt.count_generation;
        rt.ensure_backlinks_count_loaded();
        assert!(
            matches!(rt.backlinks_count, BacklinksCountState::Idle),
            "headless stays Idle (no spinner)"
        );
        assert_eq!(
            rt.count_generation, gen,
            "no generation bump without a runtime to dispatch the fetch"
        );
    }

    #[test]
    fn clear_document_resets_runtime_state_even_when_id_is_already_empty() {
        let mut rt = rt();
        rt.save_state = SaveState::Saving;
        rt.backlinks_count = BacklinksCountState::Loaded(4);
        rt.clear_document();

        assert_eq!(rt.document_id, "");
        assert_eq!(rt.save_state, SaveState::Idle);
        assert_eq!(rt.backlinks_count, BacklinksCountState::Idle);
    }

    #[test]
    fn mock_clipboard_records_copied_text_ac6() {
        // AC-6: the document-id copy goes through the ClipboardSink trait; a mock records it without
        // touching the OS clipboard.
        let clip = MockClipboard::new();
        clip.copy("KRD-12345");
        assert_eq!(clip.last.lock().unwrap().as_deref(), Some("KRD-12345"));
    }
}
