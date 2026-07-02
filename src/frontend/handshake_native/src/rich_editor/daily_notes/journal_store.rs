//! Async backend transport + state machine for the daily-notes / journal surface
//! (WP-KERNEL-012 MT-019).
//!
//! ## Backend reuse only (no backend edits — typed blocker if a gap)
//!
//! This reuses the PROVEN MT-014/MT-015/MT-017 backend-trait pattern (a `Send + Sync` trait
//! returning boxed futures, a production [`reqwest`] impl wrapping the existing reqwest 0.12 +
//! rustls stack, and a COUNTED in-memory mock for the unit tests — ZERO new dependency family).
//! Every endpoint already exists in `handshake_core` and was VERIFIED READ-ONLY against the React
//! client (`app/src/lib/api.ts`), NOT taken from the contract body:
//!   - open/get-or-create today's journal: `PUT /workspaces/{ws}/loom/journals/{date}` →
//!     [`JournalBlock`] (the verified `open_daily_journal` returns a single `LoomBlock`).
//!   - load the linked document: `GET /knowledge/documents/{document_id}` → [`JournalDocLoad`].
//!   - create a document for a blank journal block: `POST /knowledge/documents`
//!     (`{ workspace_id, title, content_json }`) → `{ document, save_receipt_event_id }`.
//!
//! ## VERIFIED CORRECTION: the contract's `document_ref` field does NOT exist
//!
//! The MT contract repeatedly names a LoomBlock `document_ref` field for the linked knowledge
//! document. The REAL `LoomBlock` (api.ts lines 1008-1025) has NO `document_ref` — the linked
//! knowledge-document id is `document_id: string | null`. Binding `document_ref` would silently
//! never resolve a document. This module binds the VERIFIED `document_id` field and documents the
//! correction (the contract's own impl-note: "don't assume document_ref exists; check").
//!
//! ## NO PERPETUAL SPINNER (KERNEL_BUILDER gate / MT-015 spinner-regression lesson)
//!
//! [`JournalState::Loading`] is only ever ENTERED when a runtime actually spawns the fetch
//! (`runtime.is_some()`). Headless / no-runtime never enters a stuck perpetual `Loading`: it
//! records a neutral non-animating state instead, so a headless harness using `step()`/`run_steps`
//! never loops forever on an animating `Spinner`. The unit tests stage deliveries directly.

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use serde::Deserialize;
use thiserror::Error;

// ── Verified backend shapes ──────────────────────────────────────────────────────────────────────

/// The verified mirror of the backend `LoomBlock` for a journal entry (`app/src/lib/api.ts` lines
/// 1008-1025). Only the fields the journal panel uses are kept; `#[serde(default)]` on the optionals
/// so a forward-compatible body still deserializes.
///
/// NOTE the field is `document_id` (NOT the contract's nonexistent `document_ref`): the linked
/// knowledge document, `None` when the journal block has no document yet ("Start writing").
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct JournalBlock {
    /// The stable Loom block id (the `block_id` badge).
    pub block_id: String,
    /// The workspace the block lives in.
    pub workspace_id: String,
    /// The block content type (expected `"journal"`).
    #[serde(default)]
    pub content_type: Option<String>,
    /// The linked knowledge document id, if any (VERIFIED field name — NOT `document_ref`).
    #[serde(default)]
    pub document_id: Option<String>,
    /// The block title (e.g. "Daily Note 2026-06-19").
    #[serde(default)]
    pub title: Option<String>,
    /// The journal date this block is for (`YYYY-MM-DD`).
    #[serde(default)]
    pub journal_date: Option<String>,
}

impl JournalBlock {
    /// The display title for this block on `date`, falling back to a "Daily Note {date}" default when
    /// the backend block has no (or a blank) title — mirrors the React `blockTitle` helper.
    pub fn display_title(&self, date: &str) -> String {
        match self.title.as_deref().map(str::trim) {
            Some(t) if !t.is_empty() => t.to_owned(),
            _ => format!("Daily Note {date}"),
        }
    }
}

/// The `RichDocLoad` envelope (`GET /knowledge/documents/{id}` returns `{ document, tree, code_nodes }`).
/// The journal panel needs the document id + version (for save) + the content_json (to render).
#[derive(Debug, Clone, Deserialize)]
struct RichDocLoadEnvelope {
    document: RichDocumentBody,
}

/// The create-document response envelope (`POST /knowledge/documents` → `{ document, save_receipt_event_id }`).
#[derive(Debug, Clone, Deserialize)]
struct CreateDocEnvelope {
    document: RichDocumentBody,
}

/// The subset of the backend `RichDocument` (api.ts lines 3028-3048) the journal panel consumes: the
/// id, the monotonic version (the MT-020 save seam's `expected_version`), and the `content_json` the
/// MT-012 renderer paints. `#[serde(default)]` on `content_json` so a never-saved document (null body)
/// still deserializes.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RichDocumentBody {
    /// The stable rich-document id (the MT-012 renderer + the save seam target it).
    pub rich_document_id: String,
    /// The document title.
    #[serde(default)]
    pub title: String,
    /// The monotonic version (the save seam's `expected_version`).
    #[serde(default)]
    pub doc_version: u64,
    /// The ProseMirror/Tiptap doc JSON the MT-012 renderer paints. `null`/absent → an empty doc.
    #[serde(default)]
    pub content_json: Option<serde_json::Value>,
}

/// The loaded journal document: the backend body plus the [`crate::rich_editor::document_model`] doc
/// the MT-012 renderer paints, parsed once on load (so a render never re-parses each frame).
#[derive(Debug, Clone)]
pub struct JournalDocLoad {
    /// The backend body (id + version for the save seam).
    pub body: RichDocumentBody,
}

// ── Typed errors ──────────────────────────────────────────────────────────────────────────────────

/// The typed reasons a journal backend interaction failed. Each renders as a VISIBLE error chip
/// (fail-closed, never blank, never a panic) with a Retry button. `kind_str` is a stable kebab-case
/// token the chip + AccessKit label carry, so an out-of-process agent reads a stable failure vocabulary
/// (the same shape as MT-015's `WikilinkError` / MT-017's `MetadataError`).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum JournalError {
    /// `openDailyJournal` (the PUT) failed.
    #[error("open daily journal failed: {0}")]
    OpenFailed(String),
    /// loading the linked document (`GET /knowledge/documents/{id}`) failed.
    #[error("document load failed: {0}")]
    DocLoadFailed(String),
    /// the auto-save / MT-020 save dispatch failed.
    #[error("save failed: {0}")]
    SaveFailed(String),
    /// creating a new document for a blank journal block (`POST /knowledge/documents`) failed.
    #[error("create document failed: {0}")]
    CreateFailed(String),
}

impl JournalError {
    /// Stable kebab-case kind token (the chip text + AccessKit label vocabulary).
    pub fn kind_str(&self) -> &'static str {
        match self {
            JournalError::OpenFailed(_) => "open_failed",
            JournalError::DocLoadFailed(_) => "doc_load_failed",
            JournalError::SaveFailed(_) => "save_failed",
            JournalError::CreateFailed(_) => "create_failed",
        }
    }
}

/// A boxed `Send` future yielding a Result, returned by the [`JournalBackend`] methods. Spelled out
/// (not `async-trait`) so this module adds ZERO new dependency families.
pub type JournalFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, JournalError>> + Send + 'a>>;

// ── Backend transport ──────────────────────────────────────────────────────────────────────────────

/// The backend transport for the MT-019 bindings. A trait (not hard reqwest calls) so the state
/// machine, date nav, and auto-save logic are unit-testable with a counted mock and NO backend.
pub trait JournalBackend: Send + Sync {
    /// `PUT /workspaces/{workspace_id}/loom/journals/{journal_date}` → the journal [`JournalBlock`]
    /// (idempotent get-or-create — the verified `open_daily_journal`).
    fn open_daily_journal<'a>(
        &'a self,
        workspace_id: &'a str,
        journal_date: &'a str,
    ) -> JournalFuture<'a, JournalBlock>;

    /// `GET /knowledge/documents/{document_id}` → the document body (`document_id` is the VERIFIED
    /// LoomBlock field, NOT `document_ref`).
    fn load_document<'a>(&'a self, document_id: &'a str) -> JournalFuture<'a, JournalDocLoad>;

    /// `POST /knowledge/documents` with `{ workspace_id, title, content_json: null }` → the created
    /// document body (the "Start writing" path for a blank journal block).
    fn create_document<'a>(
        &'a self,
        workspace_id: &'a str,
        title: &'a str,
    ) -> JournalFuture<'a, JournalDocLoad>;
}

/// The production [`JournalBackend`]: a thin wrapper over a `reqwest::Client` against the verified
/// backend endpoints, mapping HTTP status to the typed [`JournalError`] vocabulary. REUSES the existing
/// reqwest 0.12 + rustls stack from `backend_client` — NO new HTTP crate. Read/write of the EXISTING
/// loom-journal + knowledge-document APIs only; no backend code is touched.
#[derive(Clone)]
pub struct ReqwestJournalBackend {
    client: reqwest::Client,
    base_url: String,
}

impl ReqwestJournalBackend {
    /// Build a backend client against `base_url` (e.g. `backend_client::BACKEND_BASE_URL`).
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
        }
    }

    /// The production client against the hardcoded backend base URL.
    pub fn production() -> Self {
        Self::new(crate::backend_client::BACKEND_BASE_URL)
    }

    /// The REST base this client talks to.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    fn journal_url(&self, workspace_id: &str, journal_date: &str) -> String {
        format!(
            "{}/workspaces/{}/loom/journals/{}",
            self.base_url, workspace_id, journal_date
        )
    }

    fn document_url(&self, document_id: &str) -> String {
        format!("{}/knowledge/documents/{}", self.base_url, document_id)
    }

    fn create_document_url(&self) -> String {
        format!("{}/knowledge/documents", self.base_url)
    }
}

/// Map a non-success HTTP status to a short `"HTTP {code}"` reason carried by the typed error.
fn status_reason(status: u16, what: &str) -> String {
    format!("{what} returned HTTP {status}")
}

impl JournalBackend for ReqwestJournalBackend {
    fn open_daily_journal<'a>(
        &'a self,
        workspace_id: &'a str,
        journal_date: &'a str,
    ) -> JournalFuture<'a, JournalBlock> {
        let url = self.journal_url(workspace_id, journal_date);
        let client = self.client.clone();
        let date = journal_date.to_owned();
        Box::pin(async move {
            let response = client
                .put(&url)
                .timeout(std::time::Duration::from_secs(5))
                .send()
                .await
                .map_err(|e| JournalError::OpenFailed(format!("open {date} failed: {e}")))?;
            let status = response.status();
            if !status.is_success() {
                return Err(JournalError::OpenFailed(status_reason(
                    status.as_u16(),
                    "open daily journal",
                )));
            }
            let block: JournalBlock = response
                .json()
                .await
                .map_err(|e| JournalError::OpenFailed(format!("journal body invalid: {e}")))?;
            Ok(block)
        })
    }

    fn load_document<'a>(&'a self, document_id: &'a str) -> JournalFuture<'a, JournalDocLoad> {
        let url = self.document_url(document_id);
        let client = self.client.clone();
        let document_id = document_id.to_owned();
        Box::pin(async move {
            let response = client
                .get(&url)
                .timeout(std::time::Duration::from_secs(5))
                .send()
                .await
                .map_err(|e| JournalError::DocLoadFailed(format!("load failed: {e}")))?;
            let status = response.status();
            if !status.is_success() {
                return Err(JournalError::DocLoadFailed(status_reason(
                    status.as_u16(),
                    &format!("document '{document_id}'"),
                )));
            }
            let parsed: RichDocLoadEnvelope = response
                .json()
                .await
                .map_err(|e| JournalError::DocLoadFailed(format!("load body invalid: {e}")))?;
            Ok(JournalDocLoad {
                body: parsed.document,
            })
        })
    }

    fn create_document<'a>(
        &'a self,
        workspace_id: &'a str,
        title: &'a str,
    ) -> JournalFuture<'a, JournalDocLoad> {
        let url = self.create_document_url();
        let client = self.client.clone();
        let workspace_id = workspace_id.to_owned();
        let title = title.to_owned();
        Box::pin(async move {
            let body = serde_json::json!({
                "workspace_id": workspace_id,
                "title": title,
                "content_json": serde_json::Value::Null,
            });
            let response = client
                .post(&url)
                .timeout(std::time::Duration::from_secs(5))
                .json(&body)
                .send()
                .await
                .map_err(|e| JournalError::CreateFailed(format!("create failed: {e}")))?;
            let status = response.status();
            if !status.is_success() {
                return Err(JournalError::CreateFailed(status_reason(
                    status.as_u16(),
                    "create document",
                )));
            }
            let parsed: CreateDocEnvelope = response
                .json()
                .await
                .map_err(|e| JournalError::CreateFailed(format!("create body invalid: {e}")))?;
            Ok(JournalDocLoad {
                body: parsed.document,
            })
        })
    }
}

// ── The save seam (MT-020, mockable — no hard dependency on unfinished MT-020) ────────────────────

/// The auto-save dispatch seam (MT-020). A trait so the dirty-tracking + 3-second debounce + the
/// save-dispatch call are FULLY testable against a mock with NO live backend and NO hard dependency on
/// the (possibly unfinished) MT-020 implementation. The production binding reuses MT-017's metadata
/// save path / is finalized by MT-020; here it routes `PUT /knowledge/documents/{id}/save`.
pub trait JournalSaveSeam: Send + Sync {
    /// Save `content_json` for `document_id` at `expected_version`. `Ok(new_version)` on success.
    fn save<'a>(
        &'a self,
        document_id: &'a str,
        expected_version: u64,
        content_json: serde_json::Value,
    ) -> JournalFuture<'a, u64>;
}

/// The production save seam: `PUT /knowledge/documents/{id}/save` with `{ expected_version, content_json }`
/// (the VERIFIED `saveRichDocument` body, api.ts lines 3245-3263). Reuses the existing reqwest stack.
#[derive(Clone)]
pub struct ReqwestSaveSeam {
    client: reqwest::Client,
    base_url: String,
}

impl ReqwestSaveSeam {
    /// Build a save seam against `base_url`.
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
        }
    }

    /// The production save seam against the hardcoded backend base URL.
    pub fn production() -> Self {
        Self::new(crate::backend_client::BACKEND_BASE_URL)
    }
}

impl JournalSaveSeam for ReqwestSaveSeam {
    fn save<'a>(
        &'a self,
        document_id: &'a str,
        expected_version: u64,
        content_json: serde_json::Value,
    ) -> JournalFuture<'a, u64> {
        let url = format!("{}/knowledge/documents/{}/save", self.base_url, document_id);
        let client = self.client.clone();
        Box::pin(async move {
            let body = serde_json::json!({
                "expected_version": expected_version,
                "content_json": content_json,
            });
            let response = client
                .put(&url)
                .timeout(std::time::Duration::from_secs(5))
                .json(&body)
                .send()
                .await
                .map_err(|e| JournalError::SaveFailed(format!("save failed: {e}")))?;
            let status = response.status();
            if !status.is_success() {
                return Err(JournalError::SaveFailed(status_reason(
                    status.as_u16(),
                    "save document",
                )));
            }
            let v: serde_json::Value = response
                .json()
                .await
                .map_err(|e| JournalError::SaveFailed(format!("save body invalid: {e}")))?;
            let new_version = v
                .get("document")
                .and_then(|d| d.get("doc_version"))
                .and_then(|x| x.as_u64())
                .unwrap_or(expected_version.saturating_add(1));
            Ok(new_version)
        })
    }
}

// ── The state machine ──────────────────────────────────────────────────────────────────────────────

/// The save-dispatch state of the auto-save seam. `InFlight` shows a non-animating "Saving…" hint
/// (NEVER a perpetual spinner — the no-spinner discipline); `Saved` shows the last-saved relative time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveStatus {
    /// No save in flight / never saved yet.
    Idle,
    /// A save is dispatched and in flight (MC-001: a new auto-save is skipped + re-armed while here).
    InFlight,
    /// The last save succeeded at the given frame (drives the "Saved …" footer text).
    Saved,
    /// The last save failed (a non-fatal footer chip; the editor stays usable).
    Failed(JournalError),
}

/// The journal load state machine (the contract's `JournalState`). `Loading` is only entered when a
/// runtime actually spawns the fetch (no perpetual headless spinner); `Error` always renders a typed
/// chip + Retry.
#[derive(Debug, Clone)]
pub enum JournalState {
    /// Nothing requested yet (the neutral non-animating headless state). NOT a spinner.
    Idle,
    /// An `openDailyJournal` fetch is in flight for `date` (only entered with a live runtime).
    Loading { date: String },
    /// The journal block (and, if linked, its document) loaded. Boxed because [`JournalReady`] carries
    /// the full document body (much larger than the other variants — the clippy `large_enum_variant`
    /// fix); a journal panel holds one state, so the box is free.
    Ready(Box<JournalReady>),
    /// A typed failure for `date`; renders the error chip + Retry.
    Error { date: String, kind: JournalError },
}

impl JournalState {
    /// True while a genuine fetch is in flight (the ONLY state that renders the animating spinner).
    pub fn is_loading(&self) -> bool {
        matches!(self, JournalState::Loading { .. })
    }

    /// The error kind when in [`JournalState::Error`], else `None`.
    pub fn error(&self) -> Option<&JournalError> {
        match self {
            JournalState::Error { kind, .. } => Some(kind),
            _ => None,
        }
    }

    /// The ready payload when in [`JournalState::Ready`], else `None`.
    pub fn ready(&self) -> Option<&JournalReady> {
        match self {
            JournalState::Ready(r) => Some(r),
            _ => None,
        }
    }
}

/// The Ready payload: the journal block, the optionally-loaded document, the save status, and the
/// `is_creating` debounce flag for the "Start writing" path (MC-003).
#[derive(Debug, Clone)]
pub struct JournalReady {
    /// The journal date (`YYYY-MM-DD`) this entry is for.
    pub date: String,
    /// The journal Loom block.
    pub block: JournalBlock,
    /// The loaded document body, `None` for a blank journal block (the "Start writing" state).
    pub doc: Option<RichDocumentBody>,
    /// The save-dispatch status (auto-save + MT-020 seam).
    pub save: SaveStatus,
    /// MC-003: true while a "Start writing" `createRichDocument` request is in flight (the button is
    /// replaced with a spinner so a second click cannot fire a duplicate create).
    pub is_creating: bool,
}

impl JournalReady {
    /// Build a Ready payload for `date` with `block` + optional `doc`, save Idle, not creating. A
    /// public seam (mirroring `RichEditorState::with_properties`) so a screenshot test in the external
    /// integration-test crate can construct a deterministic Ready state WITHOUT a live backend or async
    /// timing (the `#[cfg(test)]` `stage_*` seams are only visible to in-crate unit tests).
    pub fn new(
        date: impl Into<String>,
        block: JournalBlock,
        doc: Option<RichDocumentBody>,
    ) -> Self {
        Self {
            date: date.into(),
            block,
            doc,
            save: SaveStatus::Idle,
            is_creating: false,
        }
    }

    /// True when the journal block has NO linked document yet (the "Start writing" empty state).
    pub fn needs_document(&self) -> bool {
        self.doc.is_none()
    }
}

/// The result delivered into the load cell when an `openDailyJournal` (+ optional doc load) completes,
/// tagged with the generation it was issued for (MC-002 cancellation): `(generation, date, result)`.
type LoadResult = (
    u64,
    String,
    Result<(JournalBlock, Option<RichDocumentBody>), JournalError>,
);

/// One-slot delivery cell for the off-thread journal load.
type LoadCell = Arc<Mutex<Option<LoadResult>>>;

/// The result delivered when a "Start writing" `createRichDocument` completes: `(generation, result)`.
type CreateResult = (u64, Result<RichDocumentBody, JournalError>);
type CreateCell = Arc<Mutex<Option<CreateResult>>>;

/// The result delivered when an auto-save completes: `(generation, result)` (new version or error).
type SaveResult = (u64, Result<u64, JournalError>);
type SaveCell = Arc<Mutex<Option<SaveResult>>>;

/// The per-panel journal runtime: the backend transport, the save seam, the tokio handle, the active
/// workspace, the [`JournalState`], the MC-002 generation counter, and the three delivery cells.
///
/// MC-002 (rapid date-nav): every navigation bumps `generation`; an in-flight load / create / save
/// tagged with a stale generation is DROPPED on drain, so clicking quickly through dates never lets an
/// older response overwrite the newest date's state (the same generation-counter cancellation MT-015's
/// backlinks fix uses).
pub struct JournalStore {
    /// The backend transport (production reqwest; tests: a counted mock).
    pub backend: Arc<dyn JournalBackend>,
    /// The auto-save seam (production reqwest; tests: a counted mock). Mockable so there is NO hard
    /// dependency on the (possibly unfinished) MT-020 implementation.
    pub save_seam: Arc<dyn JournalSaveSeam>,
    /// The tokio handle dispatches spawn onto (`None` headless — never enters Loading; no spinner).
    pub runtime: Option<tokio::runtime::Handle>,
    /// The workspace journals are opened against.
    pub workspace_id: String,
    /// The current load state machine.
    pub state: JournalState,
    /// The monotonic generation; bumped on each navigation so a stale in-flight response is dropped.
    pub generation: u64,
    load_cell: LoadCell,
    create_cell: CreateCell,
    save_cell: SaveCell,
}

impl JournalStore {
    /// Build a store over `backend` + `save_seam`, spawning onto `runtime` (`None` headless).
    pub fn new(
        backend: Arc<dyn JournalBackend>,
        save_seam: Arc<dyn JournalSaveSeam>,
        runtime: Option<tokio::runtime::Handle>,
    ) -> Self {
        Self {
            backend,
            save_seam,
            runtime,
            workspace_id: String::new(),
            state: JournalState::Idle,
            generation: 0,
            load_cell: Arc::new(Mutex::new(None)),
            create_cell: Arc::new(Mutex::new(None)),
            save_cell: Arc::new(Mutex::new(None)),
        }
    }

    /// A headless store (no tokio handle) over a backend + save seam — the test/seed constructor.
    pub fn headless(backend: Arc<dyn JournalBackend>, save_seam: Arc<dyn JournalSaveSeam>) -> Self {
        Self::new(backend, save_seam, None)
    }

    /// Install the production reqwest backend + save seam against the standard base, spawning onto
    /// `runtime`. The shell calls this when it mounts the journal panel (the production wiring point).
    pub fn production(workspace_id: impl Into<String>, runtime: tokio::runtime::Handle) -> Self {
        let base = crate::backend_client::BACKEND_BASE_URL;
        let mut store = Self::new(
            Arc::new(ReqwestJournalBackend::new(base)),
            Arc::new(ReqwestSaveSeam::new(base)),
            Some(runtime),
        );
        store.workspace_id = workspace_id.into();
        store
    }

    /// Open (or re-open) the journal for `date`. Bumps the generation (MC-002 cancels any in-flight
    /// load for a prior date), and — ONLY when a runtime can dispatch the fetch — enters `Loading` and
    /// spawns the `openDailyJournal` (+ optional doc load). Headless stays neutral (no perpetual
    /// spinner): the generation still bumps (so a staged test delivery matches), but the state does NOT
    /// enter `Loading` because nothing would ever resolve it.
    pub fn open(&mut self, date: impl Into<String>) {
        let date = date.into();
        self.generation = self.generation.wrapping_add(1);
        let generation = self.generation;

        let Some(runtime) = self.runtime.clone() else {
            // Headless: do not enter a perpetual Loading. Stay Idle (the neutral non-animating state).
            // The unit tests stage the result via `stage_load`; production always has a runtime.
            return;
        };
        self.state = JournalState::Loading { date: date.clone() };
        let backend = Arc::clone(&self.backend);
        let cell = Arc::clone(&self.load_cell);
        let workspace_id = self.workspace_id.clone();
        runtime.spawn(async move {
            let result = match backend.open_daily_journal(&workspace_id, &date).await {
                Ok(block) => match block.document_id.clone() {
                    // The block links a document → load it (so the renderer paints the journal content).
                    Some(doc_id) if !doc_id.trim().is_empty() => {
                        match backend.load_document(&doc_id).await {
                            Ok(load) => Ok((block, Some(load.body))),
                            Err(e) => Err(e),
                        }
                    }
                    // No linked document → Ready with no doc (the "Start writing" state).
                    _ => Ok((block, None)),
                },
                Err(e) => Err(e),
            };
            if let Ok(mut slot) = cell.lock() {
                *slot = Some((generation, date, result));
            }
        });
    }

    /// Dispatch the "Start writing" `createRichDocument` for the CURRENT Ready journal block (MC-003:
    /// a no-op while a create is already in flight, so a double-click cannot fire a duplicate create).
    /// Sets `is_creating` and spawns the POST; the created document lands in the create cell.
    pub fn start_writing(&mut self, title: impl Into<String>) {
        // Only valid from a Ready state with no document, and not while already creating (MC-003).
        let workspace_id = self.workspace_id.clone();
        let title = title.into();
        let JournalState::Ready(ready) = &mut self.state else {
            return;
        };
        if ready.doc.is_some() || ready.is_creating {
            return;
        }
        let Some(runtime) = self.runtime.clone() else {
            return; // headless: tests stage the create result directly.
        };
        ready.is_creating = true;
        let generation = self.generation;
        let backend = Arc::clone(&self.backend);
        let cell = Arc::clone(&self.create_cell);
        runtime.spawn(async move {
            let result = backend
                .create_document(&workspace_id, &title)
                .await
                .map(|load| load.body);
            if let Ok(mut slot) = cell.lock() {
                *slot = Some((generation, result));
            }
        });
    }

    /// Dispatch an auto-save of `content_json` for the CURRENT Ready document (MC-001: a no-op when no
    /// document is loaded OR a save is already in flight — the caller re-arms the debounce). Sets the
    /// save status to `InFlight` and spawns the save through the mockable seam.
    pub fn dispatch_save(&mut self, content_json: serde_json::Value) {
        let JournalState::Ready(ready) = &mut self.state else {
            return;
        };
        // MC-001: skip if a save is already in flight (the caller re-arms the 3s timer afterward).
        if matches!(ready.save, SaveStatus::InFlight) {
            return;
        }
        let Some(doc) = ready.doc.clone() else {
            return; // no document yet → nothing to save (the "Start writing" state).
        };
        let Some(runtime) = self.runtime.clone() else {
            return; // headless: tests stage the save result directly.
        };
        ready.save = SaveStatus::InFlight;
        let generation = self.generation;
        let seam = Arc::clone(&self.save_seam);
        let cell = Arc::clone(&self.save_cell);
        runtime.spawn(async move {
            let result = seam
                .save(&doc.rich_document_id, doc.doc_version, content_json)
                .await;
            if let Ok(mut slot) = cell.lock() {
                *slot = Some((generation, result));
            }
        });
    }

    /// Drain any off-thread load / create / save results since the last frame, applying the ones whose
    /// generation still matches (MC-002 drops stale ones). Returns true when ANYTHING was applied (the
    /// caller requests a repaint). A load result transitions the state machine; a create result fills the
    /// Ready document (+ clears `is_creating`); a save result updates the save status (+ bumps the
    /// document version on success so the next save sends the correct `expected_version`).
    pub fn drain(&mut self) -> bool {
        let mut applied = false;

        // Load delivery → drive the state machine.
        if let Ok(mut slot) = self.load_cell.lock() {
            if let Some((generation, date, result)) = slot.take() {
                if generation == self.generation {
                    self.state = match result {
                        Ok((block, doc)) => JournalState::Ready(Box::new(JournalReady {
                            date,
                            block,
                            doc,
                            save: SaveStatus::Idle,
                            is_creating: false,
                        })),
                        Err(kind) => JournalState::Error { date, kind },
                    };
                    applied = true;
                }
                // else: a stale-generation load landed late → dropped (MC-002).
            }
        }

        // Create delivery → fill the Ready document.
        if let Ok(mut slot) = self.create_cell.lock() {
            if let Some((generation, result)) = slot.take() {
                if generation == self.generation {
                    if let JournalState::Ready(ready) = &mut self.state {
                        ready.is_creating = false;
                        match result {
                            Ok(body) => {
                                ready.block.document_id = Some(body.rich_document_id.clone());
                                ready.doc = Some(body);
                            }
                            Err(kind) => {
                                self.state = JournalState::Error {
                                    date: self.current_date().unwrap_or_default(),
                                    kind,
                                };
                            }
                        }
                        applied = true;
                    }
                }
            }
        }

        // Save delivery → update the save status.
        if let Ok(mut slot) = self.save_cell.lock() {
            if let Some((generation, result)) = slot.take() {
                if generation == self.generation {
                    if let JournalState::Ready(ready) = &mut self.state {
                        match result {
                            Ok(new_version) => {
                                if let Some(doc) = ready.doc.as_mut() {
                                    doc.doc_version = new_version;
                                }
                                ready.save = SaveStatus::Saved;
                            }
                            Err(kind) => ready.save = SaveStatus::Failed(kind),
                        }
                        applied = true;
                    }
                }
            }
        }

        applied
    }

    /// Seed the state machine directly into Ready (a public test seam: a screenshot/AccessKit test in
    /// the external integration crate sets a deterministic Ready state with no live backend / async
    /// timing). The production path reaches Ready only through `open` + `drain`.
    pub fn seed_ready(&mut self, ready: JournalReady) {
        self.state = JournalState::Ready(Box::new(ready));
    }

    /// Seed the state machine directly into Error for `date` with `kind` (public test seam).
    pub fn seed_error(&mut self, date: impl Into<String>, kind: JournalError) {
        self.state = JournalState::Error {
            date: date.into(),
            kind,
        };
    }

    /// Seed the state machine directly into Loading for `date` (public test seam — renders the spinner
    /// deterministically without a live runtime; the production path enters Loading only inside `open`
    /// when a runtime can dispatch the fetch, so this seam never causes a perpetual headless spinner in
    /// production code — it is only ever called by a single-frame `step()` screenshot test).
    pub fn seed_loading(&mut self, date: impl Into<String>) {
        self.state = JournalState::Loading { date: date.into() };
    }

    /// The date currently displayed by the state machine (Loading / Ready / Error all carry one).
    pub fn current_date(&self) -> Option<String> {
        match &self.state {
            JournalState::Idle => None,
            JournalState::Loading { date } => Some(date.clone()),
            JournalState::Ready(r) => Some(r.date.clone()),
            JournalState::Error { date, .. } => Some(date.clone()),
        }
    }

    // ── Test seams (headless: stage a delivery without a tokio runtime) ──────────────────────────

    /// Stage a load delivery into the cell tagged with the CURRENT generation (test seam). The journal
    /// block + optional document are delivered as if `openDailyJournal` (+ doc load) had completed.
    #[cfg(test)]
    pub fn stage_load(
        &self,
        date: &str,
        result: Result<(JournalBlock, Option<RichDocumentBody>), JournalError>,
    ) {
        *self.load_cell.lock().unwrap() = Some((self.generation, date.to_owned(), result));
    }

    /// Stage a load delivery tagged with an EXPLICIT generation (test seam for MC-002 stale-drop).
    #[cfg(test)]
    pub fn stage_load_gen(
        &self,
        generation: u64,
        date: &str,
        result: Result<(JournalBlock, Option<RichDocumentBody>), JournalError>,
    ) {
        *self.load_cell.lock().unwrap() = Some((generation, date.to_owned(), result));
    }

    /// Stage a create-document delivery (test seam).
    #[cfg(test)]
    pub fn stage_create(&self, result: Result<RichDocumentBody, JournalError>) {
        *self.create_cell.lock().unwrap() = Some((self.generation, result));
    }

    /// Stage a save delivery (test seam).
    #[cfg(test)]
    pub fn stage_save(&self, result: Result<u64, JournalError>) {
        *self.save_cell.lock().unwrap() = Some((self.generation, result));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(date: &str, doc_id: Option<&str>) -> JournalBlock {
        JournalBlock {
            block_id: format!("LB-{date}"),
            workspace_id: "ws-1".into(),
            content_type: Some("journal".into()),
            document_id: doc_id.map(|s| s.to_owned()),
            title: Some(format!("Daily Note {date}")),
            journal_date: Some(date.to_owned()),
        }
    }

    fn doc_body(id: &str, version: u64) -> RichDocumentBody {
        RichDocumentBody {
            rich_document_id: id.into(),
            title: "Daily Note".into(),
            doc_version: version,
            content_json: Some(serde_json::json!({ "type": "doc", "content": [] })),
        }
    }

    /// A counted mock backend: open returns a journal block (with/without a doc), load returns a fixed
    /// document, create returns a fresh document. Records call counts for the debounce/cancel proofs.
    struct MockBackend {
        link_document: bool,
        open_calls: Mutex<u32>,
        create_calls: Mutex<u32>,
    }
    impl MockBackend {
        fn new(link_document: bool) -> Self {
            Self {
                link_document,
                open_calls: Mutex::new(0),
                create_calls: Mutex::new(0),
            }
        }
    }
    impl JournalBackend for MockBackend {
        fn open_daily_journal<'a>(
            &'a self,
            _ws: &'a str,
            date: &'a str,
        ) -> JournalFuture<'a, JournalBlock> {
            *self.open_calls.lock().unwrap() += 1;
            let date = date.to_owned();
            let doc_id = if self.link_document {
                Some("KRD-1".to_owned())
            } else {
                None
            };
            Box::pin(async move {
                Ok(JournalBlock {
                    block_id: format!("LB-{date}"),
                    workspace_id: "ws-1".into(),
                    content_type: Some("journal".into()),
                    document_id: doc_id,
                    title: Some(format!("Daily Note {date}")),
                    journal_date: Some(date),
                })
            })
        }
        fn load_document<'a>(&'a self, id: &'a str) -> JournalFuture<'a, JournalDocLoad> {
            let id = id.to_owned();
            Box::pin(async move {
                Ok(JournalDocLoad {
                    body: doc_body(&id, 3),
                })
            })
        }
        fn create_document<'a>(
            &'a self,
            _ws: &'a str,
            title: &'a str,
        ) -> JournalFuture<'a, JournalDocLoad> {
            *self.create_calls.lock().unwrap() += 1;
            let title = title.to_owned();
            Box::pin(async move {
                let mut body = doc_body("KRD-NEW", 1);
                body.title = title;
                Ok(JournalDocLoad { body })
            })
        }
    }

    struct MockSaveSeam {
        calls: Mutex<u32>,
    }
    impl MockSaveSeam {
        fn new() -> Self {
            Self {
                calls: Mutex::new(0),
            }
        }
    }
    impl JournalSaveSeam for MockSaveSeam {
        fn save<'a>(
            &'a self,
            _id: &'a str,
            expected: u64,
            _c: serde_json::Value,
        ) -> JournalFuture<'a, u64> {
            *self.calls.lock().unwrap() += 1;
            Box::pin(async move { Ok(expected + 1) })
        }
    }

    fn store(link_document: bool) -> JournalStore {
        JournalStore::headless(
            Arc::new(MockBackend::new(link_document)),
            Arc::new(MockSaveSeam::new()),
        )
    }

    #[test]
    fn document_id_is_the_verified_field_not_document_ref() {
        // The contract's `document_ref` does NOT exist on the real LoomBlock; we bind `document_id`.
        let json = serde_json::json!({
            "block_id": "LB-1",
            "workspace_id": "ws-1",
            "content_type": "journal",
            "document_id": "KRD-77",
            "title": "Daily Note 2026-06-19",
            "journal_date": "2026-06-19"
        });
        let b: JournalBlock = serde_json::from_value(json).unwrap();
        assert_eq!(b.document_id.as_deref(), Some("KRD-77"));
        assert_eq!(b.display_title("2026-06-19"), "Daily Note 2026-06-19");
    }

    #[test]
    fn block_with_no_document_id_deserializes_and_needs_document() {
        // A blank journal block (no document_id) deserializes and triggers the "Start writing" path.
        let json = serde_json::json!({ "block_id": "LB-2", "workspace_id": "ws-1" });
        let b: JournalBlock = serde_json::from_value(json).unwrap();
        assert_eq!(b.document_id, None);
        // Empty/whitespace title → the "Daily Note {date}" fallback.
        assert_eq!(b.display_title("2026-06-19"), "Daily Note 2026-06-19");
    }

    #[test]
    fn headless_open_does_not_enter_perpetual_loading() {
        // No runtime → must NOT enter Loading (nothing would resolve it → perpetual spinner). Stays Idle.
        let mut s = store(true);
        s.open("2026-06-19");
        assert!(
            matches!(s.state, JournalState::Idle),
            "headless stays Idle (no perpetual spinner)"
        );
        assert_eq!(
            s.generation, 1,
            "the generation still bumps so a staged delivery matches"
        );
    }

    #[test]
    fn drain_load_transitions_idle_to_ready_with_doc() {
        // AC-2 + state-machine: staging a successful load (block + doc) drains to Ready with the doc.
        let mut s = store(true);
        s.open("2026-06-19");
        s.stage_load(
            "2026-06-19",
            Ok((
                block("2026-06-19", Some("KRD-1")),
                Some(doc_body("KRD-1", 3)),
            )),
        );
        assert!(s.drain());
        let ready = s.state.ready().expect("Ready after a successful load");
        assert_eq!(ready.date, "2026-06-19");
        assert_eq!(ready.block.document_id.as_deref(), Some("KRD-1"));
        assert!(
            !ready.needs_document(),
            "a linked document means no 'Start writing'"
        );
    }

    #[test]
    fn drain_load_with_no_doc_is_ready_needing_document() {
        let mut s = store(false);
        s.open("2026-06-19");
        s.stage_load("2026-06-19", Ok((block("2026-06-19", None), None)));
        assert!(s.drain());
        let ready = s.state.ready().expect("Ready");
        assert!(
            ready.needs_document(),
            "no document → the 'Start writing' state"
        );
    }

    #[test]
    fn drain_load_error_transitions_to_error_state() {
        // AC-7: a simulated openDailyJournal failure → Error state with a typed kind (chip + Retry).
        let mut s = store(true);
        s.open("2026-06-19");
        s.stage_load(
            "2026-06-19",
            Err(JournalError::OpenFailed("HTTP 500".into())),
        );
        assert!(s.drain());
        let err = s.state.error().expect("Error after a failed open");
        assert_eq!(err.kind_str(), "open_failed");
    }

    #[test]
    fn stale_generation_load_is_dropped_mc002() {
        // MC-002: a rapid date-nav bumps the generation; an OLDER date's load landing late is dropped.
        let mut s = store(true);
        s.open("2026-06-18");
        let stale_gen = s.generation;
        s.open("2026-06-19"); // navigate forward → newer generation
        let fresh_gen = s.generation;
        assert_ne!(stale_gen, fresh_gen);

        // The stale (2026-06-18) load lands late → dropped, state unchanged (still Idle headless).
        s.stage_load_gen(
            stale_gen,
            "2026-06-18",
            Ok((block("2026-06-18", None), None)),
        );
        assert!(!s.drain(), "MC-002: a stale-generation load is dropped");
        assert!(
            s.state.ready().is_none(),
            "the stale load did not become Ready"
        );

        // The fresh (2026-06-19) load lands → applied.
        s.stage_load_gen(
            fresh_gen,
            "2026-06-19",
            Ok((block("2026-06-19", None), None)),
        );
        assert!(s.drain());
        assert_eq!(s.state.ready().unwrap().date, "2026-06-19");
    }

    #[test]
    fn start_writing_then_drain_create_fills_the_document_mc003() {
        // The "Start writing" create fills the Ready document; is_creating gates a duplicate (MC-003).
        let mut s = store(false);
        s.open("2026-06-19");
        s.stage_load("2026-06-19", Ok((block("2026-06-19", None), None)));
        assert!(s.drain());
        assert!(s.state.ready().unwrap().needs_document());

        // Headless start_writing does not spawn, but staging a create + drain fills the doc.
        s.stage_create(Ok(doc_body("KRD-NEW", 1)));
        assert!(s.drain());
        let ready = s.state.ready().unwrap();
        assert!(
            !ready.needs_document(),
            "the created document fills the Ready state"
        );
        assert_eq!(ready.block.document_id.as_deref(), Some("KRD-NEW"));
        assert!(!ready.is_creating);
    }

    #[test]
    fn dispatch_save_skips_when_already_in_flight_mc001() {
        // MC-001: dispatch_save is a no-op while a save is already in flight (the caller re-arms).
        let mut s = store(true);
        s.open("2026-06-19");
        s.stage_load(
            "2026-06-19",
            Ok((
                block("2026-06-19", Some("KRD-1")),
                Some(doc_body("KRD-1", 3)),
            )),
        );
        assert!(s.drain());
        // Force the save status to InFlight, then dispatch → should be skipped (no panic, stays InFlight).
        if let JournalState::Ready(r) = &mut s.state {
            r.save = SaveStatus::InFlight;
        }
        s.dispatch_save(serde_json::json!({ "type": "doc" }));
        assert!(matches!(
            s.state.ready().unwrap().save,
            SaveStatus::InFlight
        ));
    }

    #[test]
    fn drain_save_success_bumps_doc_version() {
        // A successful save updates the status to Saved AND bumps the doc version (so the next save
        // sends the correct expected_version — the optimistic-concurrency contract).
        let mut s = store(true);
        s.open("2026-06-19");
        s.stage_load(
            "2026-06-19",
            Ok((
                block("2026-06-19", Some("KRD-1")),
                Some(doc_body("KRD-1", 3)),
            )),
        );
        assert!(s.drain());
        s.stage_save(Ok(4));
        assert!(s.drain());
        let ready = s.state.ready().unwrap();
        assert!(matches!(ready.save, SaveStatus::Saved));
        assert_eq!(
            ready.doc.as_ref().unwrap().doc_version,
            4,
            "the version bumped after the save"
        );
    }

    #[test]
    fn drain_save_failure_records_typed_error_chip() {
        let mut s = store(true);
        s.open("2026-06-19");
        s.stage_load(
            "2026-06-19",
            Ok((
                block("2026-06-19", Some("KRD-1")),
                Some(doc_body("KRD-1", 3)),
            )),
        );
        assert!(s.drain());
        s.stage_save(Err(JournalError::SaveFailed("HTTP 409".into())));
        assert!(s.drain());
        assert!(matches!(
            s.state.ready().unwrap().save,
            SaveStatus::Failed(_)
        ));
    }

    #[test]
    fn error_kind_strings_are_stable() {
        assert_eq!(
            JournalError::OpenFailed("x".into()).kind_str(),
            "open_failed"
        );
        assert_eq!(
            JournalError::DocLoadFailed("x".into()).kind_str(),
            "doc_load_failed"
        );
        assert_eq!(
            JournalError::SaveFailed("x".into()).kind_str(),
            "save_failed"
        );
        assert_eq!(
            JournalError::CreateFailed("x".into()).kind_str(),
            "create_failed"
        );
    }

    #[test]
    fn production_request_urls_are_verified() {
        // The production reqwest backend builds the VERIFIED endpoint URLs.
        let b = ReqwestJournalBackend::new("http://h");
        assert_eq!(
            b.journal_url("ws", "2026-06-19"),
            "http://h/workspaces/ws/loom/journals/2026-06-19"
        );
        assert_eq!(
            b.document_url("KRD-1"),
            "http://h/knowledge/documents/KRD-1"
        );
        assert_eq!(b.create_document_url(), "http://h/knowledge/documents");
    }
}
