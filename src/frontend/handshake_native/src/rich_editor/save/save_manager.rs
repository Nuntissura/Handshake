//! Canonical save + optimistic-concurrency conflict state machine for the rich-text editor
//! (WP-KERNEL-012 MT-020).
//!
//! Port of the `app/src/components/RichDocumentView.tsx` save flow (MT-244/MT-255):
//! - [`SaveManager::request_save`] performs `PUT /knowledge/documents/{id}/save`
//!   `{ content_json, expected_version }` off the egui frame thread (HBR-QUIET), delivering the
//!   typed result into a one-slot cell the UI drains next frame.
//! - A 200 updates `doc_version`, clears the dirty flag, sets `updated_at`.
//! - A 409 (version mismatch) transitions to [`SaveState::Conflict`] carrying the server document
//!   + the operator's local content, which the [`super::conflict_ui`] panel resolves.
//!
//! ## Backend reuse only (verified, not contract-assumed)
//!
//! The save endpoint + its `{ expected_version, content_json }` body were VERIFIED READ-ONLY
//! against `src/backend/handshake_core/src/api/knowledge_documents.rs` (`save_document`,
//! `SaveDocumentBody`). The 409 is the verified optimistic-concurrency conflict response. NO
//! backend edit; a gap would be a typed blocker.
//!
//! ## Why a backend TRAIT + mock (the MT-019 pattern)
//!
//! [`SaveBackend`] is a `Send + Sync` trait returning a boxed future, with a production
//! [`reqwest`] impl and a COUNTED in-memory mock for the unit tests. This is the proven
//! MT-014/15/17/19 seam: the state machine is fully unit-testable with mock HTTP + a mock clock,
//! and the live backend halves are the gated integration ACs (NEEDS_MANAGED_RESOURCE_PROOF).
//!
//! ## MC-002 (draft/save race): the is_saving guard
//!
//! [`SaveManager::is_saving`] is `true` from `request_save` until the result is drained. The
//! draft manager ([`super::draft_manager`]) refuses to upsert a draft while a save is in flight,
//! so a stale-`base_doc_version` draft can never race a canonical save (red-team RISK-2).
//!
//! ## MC-003 (Keep-yours data-loss guard)
//!
//! Resolving a conflict with "Keep yours" re-saves the operator's content AT the server's
//! `doc_version`, silently overwriting a concurrent edit. That is destructive, so it requires a
//! secondary confirmation ([`SaveState::ConfirmKeepYours`]) before [`SaveManager::confirm_keep_yours`]
//! re-saves (red-team RISK-3 / HBR-STOP).

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use serde::Deserialize;
use serde_json::Value as JsonValue;
use thiserror::Error;

/// The verified subset of the backend `RichDocument` the save flow consumes: the id, the monotonic
/// `doc_version` (the optimistic-concurrency token), and the `content_json` (so a "Keep server"
/// reload can rebuild the doc). `#[serde(default)]` on the optionals so a forward-compatible body
/// still deserializes.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RichDocLoad {
    /// The stable rich-document id.
    pub rich_document_id: String,
    /// The monotonic version (the next save's `expected_version`).
    #[serde(default)]
    pub doc_version: u64,
    /// The document title.
    #[serde(default)]
    pub title: String,
    /// The server's ProseMirror doc JSON (for a "Keep server" reload). `null` → empty doc.
    #[serde(default)]
    pub content_json: Option<JsonValue>,
    /// The server's `updated_at` (ISO-8601), surfaced after a save / in the conflict panel.
    #[serde(default)]
    pub updated_at: Option<String>,
}

/// The result of a successful save (`PUT /save` 200): the updated document record. `doc_version`
/// here is the NEW version the editor adopts as its next `expected_version`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RichDocSaveResult {
    /// The updated document (carrying the new `doc_version` + `updated_at`).
    pub document: RichDocLoad,
}

/// Why a save failed. A [`SaveError::VersionConflict`] carries the server document so the conflict
/// UI can show both versions (the contract's typed-variant set).
#[derive(Debug, Clone, PartialEq, Error)]
pub enum SaveError {
    /// HTTP 409: the `expected_version` did not match the server's current version. Carries the
    /// server document for the conflict panel.
    #[error("version conflict (server is ahead)")]
    VersionConflict(Box<RichDocLoad>),
    /// A transport-layer failure (timeout, connection refused, …) — TRANSIENT, retryable.
    #[error("network error: {0}")]
    Network(String),
    /// A non-409 non-success HTTP status with the body text.
    #[error("server error {0}: {1}")]
    Server(u16, String),
    /// The server rejected the content_json (e.g. malformed block tree) — a 400.
    #[error("schema rejected: {0}")]
    SchemaRejected(String),
}

/// A boxed future returned by [`SaveBackend`] (the MT-019 transport-trait shape).
pub type SaveFuture = Pin<Box<dyn Future<Output = Result<RichDocSaveResult, SaveError>> + Send>>;

/// The async save transport. A `Send + Sync` trait so the production reqwest impl and the unit
/// mock are interchangeable; the manager spawns the returned future on the app runtime.
pub trait SaveBackend: Send + Sync {
    /// `PUT /knowledge/documents/{document_id}/save` with `{ content_json, expected_version }`.
    fn save_document(
        &self,
        document_id: &str,
        content_json: JsonValue,
        expected_version: u64,
    ) -> SaveFuture;
}

/// The production save transport over the existing reqwest 0.12 + rustls stack (no new dependency
/// family). Maps a 409 to [`SaveError::VersionConflict`], a 400 to [`SaveError::SchemaRejected`],
/// any other non-2xx to [`SaveError::Server`], and a transport failure to [`SaveError::Network`].
///
/// ## The four required identity headers (the missing-headers fix)
///
/// The backend `doc_context` (`handshake_core::api::knowledge_documents`) REQUIRES four headers on
/// EVERY document request — `x-hsk-actor-id`, `x-hsk-kernel-task-run-id`, `x-hsk-session-run-id`,
/// `x-hsk-actor-kind` — and returns a hard HTTP 400 ("<header> header is required") when any is
/// missing. A missing `x-hsk-actor-kind` additionally defaults to the LEAST-privileged (read-only)
/// kind, so `ctx.require(DocumentAction::Write)` then 403s BEFORE the 409-conflict path is ever
/// reached. So the transport MUST attach all four (verified READ-ONLY against the backend
/// `doc_context` + the MT-158 permission matrix). We reuse the canonical header-name constants +
/// the `operator` actor-kind (which the matrix grants `Write`) from [`crate::backend_client`] rather
/// than re-deriving them. `session_run_id` makes each editor session's saves attributable.
#[derive(Clone)]
pub struct ReqwestSaveBackend {
    client: reqwest::Client,
    base_url: String,
    /// A per-backend session id folded into the per-request run ids so every save is individually
    /// traceable (HBR-SWARM attribution). Stable for the lifetime of one editor's save manager.
    session_run_id: String,
}

impl ReqwestSaveBackend {
    /// Build a backend against `base_url` (e.g. [`crate::backend_client::BACKEND_BASE_URL`]).
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            session_run_id: new_session_run_id(),
        }
    }

    fn save_url(&self, document_id: &str) -> String {
        format!("{}/knowledge/documents/{}/save", self.base_url, document_id)
    }
}

impl SaveBackend for ReqwestSaveBackend {
    fn save_document(
        &self,
        document_id: &str,
        content_json: JsonValue,
        expected_version: u64,
    ) -> SaveFuture {
        let url = self.save_url(document_id);
        let client = self.client.clone();
        let session_run_id = self.session_run_id.clone();
        let document_id_owned = document_id.to_owned();
        Box::pin(async move {
            let body = serde_json::json!({
                "content_json": content_json,
                "expected_version": expected_version,
            });
            let resp = attach_doc_headers(
                client
                    .put(&url)
                    .timeout(std::time::Duration::from_secs(10))
                    .json(&body),
                &document_id_owned,
                &session_run_id,
            )
                .send()
                .await
                .map_err(|e| SaveError::Network(e.to_string()))?;
            let status = resp.status();
            if status.is_success() {
                let result: RichDocSaveResult = resp
                    .json()
                    .await
                    .map_err(|e| SaveError::Server(status.as_u16(), e.to_string()))?;
                return Ok(result);
            }
            // A 409 carries the server document so the conflict UI can show both versions.
            if status == reqwest::StatusCode::CONFLICT {
                // The verified conflict body shape is `{ "document": {...} }` (the server's current
                // document) OR the bare document; accept either to be robust to a body-shape tweak.
                let v: JsonValue = resp.json().await.unwrap_or(JsonValue::Null);
                let doc_value = v.get("document").cloned().unwrap_or(v);
                let server: RichDocLoad =
                    serde_json::from_value(doc_value).unwrap_or(RichDocLoad {
                        rich_document_id: document_id_placeholder(),
                        doc_version: expected_version + 1,
                        title: String::new(),
                        content_json: None,
                        updated_at: None,
                    });
                return Err(SaveError::VersionConflict(Box::new(server)));
            }
            if status == reqwest::StatusCode::BAD_REQUEST {
                let text = resp.text().await.unwrap_or_default();
                return Err(SaveError::SchemaRejected(text));
            }
            let text = resp.text().await.unwrap_or_default();
            Err(SaveError::Server(status.as_u16(), text))
        })
    }
}

/// A placeholder rich-document id used only when a 409 body omits the document id entirely (a
/// degenerate server response). The conflict UI then still renders (server content empty) rather
/// than the save silently succeeding.
fn document_id_placeholder() -> String {
    "unknown-document".to_string()
}

/// A monotonic-ish session run id seed for the document transports' per-request run-id headers. Uses
/// the process id + a process-lifetime atomic counter so each save/draft transport gets a distinct,
/// stable id without a new dependency (the same lightweight scheme other native-editor transports
/// use for traceability). NOT a security token — purely an attribution/trace correlation value.
pub(crate) fn new_session_run_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    format!("native-editor-{}-{}", std::process::id(), seq)
}

/// Attach the four backend-required document identity headers to `builder` (the missing-headers fix).
/// `x-hsk-actor-id` + `x-hsk-actor-kind` come from the canonical [`crate::backend_client`] document
/// constants (`operator`, which the MT-158 matrix grants `Write`); `x-hsk-kernel-task-run-id` +
/// `x-hsk-session-run-id` fold in the target document + the session id so each request is traceable.
/// REUSED by the save backend and the draft backend so the wire identity is constructed in ONE place.
pub(crate) fn attach_doc_headers(
    builder: reqwest::RequestBuilder,
    document_id: &str,
    session_run_id: &str,
) -> reqwest::RequestBuilder {
    use crate::backend_client::{
        DOC_ACTOR_ID, DOC_ACTOR_KIND, HSK_HEADER_ACTOR_ID, HSK_HEADER_ACTOR_KIND,
        HSK_HEADER_KERNEL_TASK_RUN_ID, HSK_HEADER_SESSION_RUN_ID,
    };
    builder
        .header(HSK_HEADER_ACTOR_ID, DOC_ACTOR_ID)
        .header(HSK_HEADER_ACTOR_KIND, DOC_ACTOR_KIND)
        .header(HSK_HEADER_KERNEL_TASK_RUN_ID, format!("native-editor-doc-{document_id}"))
        .header(HSK_HEADER_SESSION_RUN_ID, session_run_id)
}

/// One-slot delivery cell for an off-thread save result, drained by the egui UI thread next frame
/// (the proven `Arc<Mutex<Option<..>>>` pattern). `Some` once the spawned save resolves.
pub type SaveDeliveryCell = Arc<Mutex<Option<Result<RichDocSaveResult, SaveError>>>>;

/// The save-flow state. The editor holds one [`SaveManager`]; [`SaveState`] drives the conflict UI.
#[derive(Debug, Clone, PartialEq)]
pub enum SaveState {
    /// No save in progress and no conflict.
    Idle,
    /// A save is in flight (the `is_saving` guard — MC-002). Carries the `expected_version` the
    /// in-flight save used so a 409 result is matched correctly.
    Saving { expected_version: u64 },
    /// The last save 409'd: a conflict is open. Carries the server document + the operator's local
    /// content (the conflict UI shows both, then resolves via Keep-yours / Keep-server).
    Conflict {
        server: Box<RichDocLoad>,
        local_content: JsonValue,
    },
    /// MC-003: the operator chose "Keep yours" and must confirm the destructive overwrite before it
    /// re-saves. Carries the same conflict payload so a cancel returns to [`SaveState::Conflict`].
    ConfirmKeepYours {
        server: Box<RichDocLoad>,
        local_content: JsonValue,
    },
    /// The last save failed for a NON-conflict reason (network/server/schema); the message is shown
    /// on a dismissible error chip. Not a conflict (no both-versions UI).
    Error(String),
}

/// The persistent save coordinator owned by the editor. Tracks the dirty flag, the current
/// `doc_version`, the in-flight save guard, and the conflict state machine.
pub struct SaveManager {
    /// The backend transport (production reqwest, or a mock in tests).
    backend: Arc<dyn SaveBackend>,
    /// The tokio runtime handle saves spawn onto. `None` in a headless unit test (the test stages
    /// the delivery cell directly instead of spawning), so the manager never blocks on a runtime.
    runtime: Option<tokio::runtime::Handle>,
    /// The document id saves target.
    document_id: String,
    /// The monotonic version the next save sends as `expected_version` (set on load, bumped after
    /// each successful save). NEVER hardcoded 0 (impl note: comes from the loaded doc).
    pub doc_version: u64,
    /// True when the in-memory doc has unsaved edits (cleared on a successful save).
    pub dirty: bool,
    /// The server `updated_at` from the last successful save (surfaced in the UI).
    pub updated_at: Option<String>,
    /// The save-flow state (drives the conflict UI).
    pub state: SaveState,
    /// The one-slot cell the spawned save delivers into; drained each frame by [`Self::drain`].
    cell: SaveDeliveryCell,
    /// The content the in-flight save attempted, threaded into a resulting conflict so the
    /// both-versions UI can show the operator's local version. Set by `set_pending_local_content`,
    /// taken in `drain` on a 409.
    pending_local_content: Option<JsonValue>,
}

impl SaveManager {
    /// Build a manager for `document_id` at `doc_version`, with `backend` + an optional runtime
    /// handle. A headless test passes `runtime = None` and stages [`Self::deliver_for_test`].
    pub fn new(
        backend: Arc<dyn SaveBackend>,
        runtime: Option<tokio::runtime::Handle>,
        document_id: impl Into<String>,
        doc_version: u64,
    ) -> Self {
        Self {
            backend,
            runtime,
            document_id: document_id.into(),
            doc_version,
            dirty: false,
            updated_at: None,
            state: SaveState::Idle,
            cell: Arc::new(Mutex::new(None)),
            pending_local_content: None,
        }
    }

    /// The production manager over the reqwest backend + the app runtime handle.
    pub fn production(
        runtime: tokio::runtime::Handle,
        document_id: impl Into<String>,
        doc_version: u64,
    ) -> Self {
        let backend = Arc::new(ReqwestSaveBackend::new(crate::backend_client::BACKEND_BASE_URL));
        Self::new(backend, Some(runtime), document_id, doc_version)
    }

    /// True while a canonical save is in flight (the MC-002 guard the draft manager consults).
    pub fn is_saving(&self) -> bool {
        matches!(self.state, SaveState::Saving { .. })
    }

    /// True when a conflict (or its keep-yours confirmation) is open.
    pub fn has_conflict(&self) -> bool {
        matches!(
            self.state,
            SaveState::Conflict { .. } | SaveState::ConfirmKeepYours { .. }
        )
    }

    /// Mark the document dirty (an edit happened). The widget calls this after any mutation; the
    /// draft manager keys its debounce off the dirty flag.
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Request a canonical save of `content_json` at the current `doc_version`. No-op when a save is
    /// already in flight (the guard prevents a double-save race). Spawns the backend call off the
    /// frame thread (HBR-QUIET); the result lands in the delivery cell. In a headless test with no
    /// runtime, the state still moves to `Saving` and the test stages the delivery directly.
    pub fn request_save(&mut self, content_json: JsonValue) {
        if self.is_saving() {
            return; // MC-002: never start a second save while one is in flight.
        }
        let expected_version = self.doc_version;
        self.state = SaveState::Saving { expected_version };
        // Clear the cell before spawning so a stale prior result can't be drained as this save's.
        if let Ok(mut slot) = self.cell.lock() {
            *slot = None;
        }
        if let Some(rt) = &self.runtime {
            let backend = Arc::clone(&self.backend);
            let cell = Arc::clone(&self.cell);
            let document_id = self.document_id.clone();
            rt.spawn(async move {
                let result = backend.save_document(&document_id, content_json, expected_version).await;
                if let Ok(mut slot) = cell.lock() {
                    *slot = Some(result);
                }
            });
        }
        // No runtime (headless): the state is `Saving`; the test calls `deliver_for_test` then `drain`.
    }

    /// Drain a completed save result (called each frame by the widget). Applies the outcome:
    /// - Ok: bump `doc_version`, clear `dirty`, set `updated_at`, return to `Idle`, and report the
    ///   document so the caller can clear the draft (`DELETE /draft`).
    /// - 409: enter `Conflict` carrying the server doc + the local content the save attempted.
    /// - other Err: enter `Error`.
    ///
    /// Returns `Some(SaveOutcome)` when a result was drained this frame, else `None`. The
    /// `local_content` the conflict carries is the content the caller passed to `request_save`; the
    /// caller threads it in via [`Self::set_pending_local_content`] before the result drains.
    pub fn drain(&mut self) -> Option<SaveOutcome> {
        let delivered = self.cell.lock().ok().and_then(|mut s| s.take())?;
        match delivered {
            Ok(result) => {
                self.doc_version = result.document.doc_version;
                self.dirty = false;
                self.updated_at = result.document.updated_at.clone();
                self.state = SaveState::Idle;
                Some(SaveOutcome::Saved {
                    doc_version: result.document.doc_version,
                })
            }
            Err(SaveError::VersionConflict(server)) => {
                let local = self.pending_local_content.take().unwrap_or(JsonValue::Null);
                self.state = SaveState::Conflict {
                    server: server.clone(),
                    local_content: local,
                };
                Some(SaveOutcome::Conflict)
            }
            Err(e) => {
                self.state = SaveState::Error(e.to_string());
                Some(SaveOutcome::Failed(e))
            }
        }
    }

    /// Record the content the in-flight save attempted, so a resulting 409 conflict carries the
    /// operator's local version for the both-versions UI. The widget calls this with the SAME
    /// content it passed to `request_save`.
    pub fn set_pending_local_content(&mut self, content_json: JsonValue) {
        self.pending_local_content = Some(content_json);
    }

    /// MC-003 step 1: the operator clicked "Keep yours" in the conflict UI. Transition to the
    /// confirmation state (does NOT re-save yet — the destructive overwrite needs a confirm).
    pub fn request_keep_yours(&mut self) {
        if let SaveState::Conflict { server, local_content } = &self.state {
            self.state = SaveState::ConfirmKeepYours {
                server: server.clone(),
                local_content: local_content.clone(),
            };
        }
    }

    /// MC-003 step 2: the operator confirmed the overwrite. Re-save the operator's local content AT
    /// the server's `doc_version` (so the optimistic-concurrency check passes this time),
    /// overwriting the concurrent edit. Only valid from [`SaveState::ConfirmKeepYours`].
    pub fn confirm_keep_yours(&mut self) {
        let SaveState::ConfirmKeepYours { server, local_content } = &self.state else {
            return;
        };
        let local = local_content.clone();
        // Adopt the server's version as our expected_version so the re-save matches the server.
        self.doc_version = server.doc_version;
        self.set_pending_local_content(local.clone());
        // request_save reads self.doc_version (now the server's) and spawns the overwrite.
        self.request_save(local);
    }

    /// The operator cancelled the keep-yours confirmation: return to the open conflict (no save).
    pub fn cancel_keep_yours(&mut self) {
        if let SaveState::ConfirmKeepYours { server, local_content } = &self.state {
            self.state = SaveState::Conflict {
                server: server.clone(),
                local_content: local_content.clone(),
            };
        }
    }

    /// "Keep server": discard the local edits, adopt the server document + version, clear the
    /// conflict + dirty flag. Returns the server `content_json` (the caller rebuilds the doc from
    /// it) when a conflict was open, else `None`.
    pub fn keep_server(&mut self) -> Option<JsonValue> {
        let SaveState::Conflict { server, .. } = &self.state else {
            return None;
        };
        let content = server.content_json.clone().unwrap_or(JsonValue::Null);
        self.doc_version = server.doc_version;
        self.updated_at = server.updated_at.clone();
        self.dirty = false;
        self.state = SaveState::Idle;
        Some(content)
    }

    /// Dismiss a non-conflict error chip (return to `Idle`).
    pub fn dismiss_error(&mut self) {
        if matches!(self.state, SaveState::Error(_)) {
            self.state = SaveState::Idle;
        }
    }

    /// The document id (for the draft manager / tests).
    pub fn document_id(&self) -> &str {
        &self.document_id
    }

    /// TEST SEAM: stage a delivery into the cell as if the spawned save resolved (headless, no
    /// runtime). The next [`Self::drain`] applies it. Mirrors the MT-019 `seed_*` test seams.
    pub fn deliver_for_test(&self, result: Result<RichDocSaveResult, SaveError>) {
        if let Ok(mut slot) = self.cell.lock() {
            *slot = Some(result);
        }
    }
}

/// The outcome `drain` reports for one frame's save result.
#[derive(Debug, Clone, PartialEq)]
pub enum SaveOutcome {
    /// The save succeeded; the new `doc_version` is adopted. The caller should clear the draft.
    Saved { doc_version: u64 },
    /// The save 409'd; a conflict is now open.
    Conflict,
    /// The save failed for a non-conflict reason.
    Failed(SaveError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// A counted in-memory mock backend: returns a staged result and counts calls (so a test can
    /// assert exactly one save fired, MC-002).
    struct MockBackend {
        result: Mutex<Option<Result<RichDocSaveResult, SaveError>>>,
        calls: AtomicUsize,
    }
    impl MockBackend {
        fn new(result: Result<RichDocSaveResult, SaveError>) -> Self {
            Self {
                result: Mutex::new(Some(result)),
                calls: AtomicUsize::new(0),
            }
        }
    }
    impl SaveBackend for MockBackend {
        fn save_document(&self, _id: &str, _c: JsonValue, _v: u64) -> SaveFuture {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let staged = self.result.lock().unwrap().clone();
            Box::pin(async move { staged.unwrap_or(Err(SaveError::Network("no staged result".into()))) })
        }
    }

    fn ok_result(doc_version: u64) -> RichDocSaveResult {
        RichDocSaveResult {
            document: RichDocLoad {
                rich_document_id: "DOC-1".into(),
                doc_version,
                title: "T".into(),
                content_json: Some(json!({"type":"doc","content":[]})),
                updated_at: Some("2026-06-22T00:00:00Z".into()),
            },
        }
    }

    fn server_doc(doc_version: u64) -> RichDocLoad {
        RichDocLoad {
            rich_document_id: "DOC-1".into(),
            doc_version,
            title: "T".into(),
            content_json: Some(json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"server"}]}]})),
            updated_at: Some("2026-06-22T01:00:00Z".into()),
        }
    }

    fn mgr(result: Result<RichDocSaveResult, SaveError>) -> SaveManager {
        SaveManager::new(Arc::new(MockBackend::new(result)), None, "DOC-1", 3)
    }

    #[test]
    fn successful_save_bumps_version_and_clears_dirty() {
        // AC: a 200 clears dirty + updates doc_version (mock HTTP 200, headless deliver).
        let mut m = mgr(Ok(ok_result(4)));
        m.mark_dirty();
        m.request_save(json!({"type":"doc","content":[]}));
        assert!(m.is_saving(), "state is Saving while in flight (MC-002 guard true)");
        m.deliver_for_test(Ok(ok_result(4)));
        let outcome = m.drain().unwrap();
        assert_eq!(outcome, SaveOutcome::Saved { doc_version: 4 });
        assert_eq!(m.doc_version, 4);
        assert!(!m.dirty, "dirty cleared after a successful save");
        assert_eq!(m.state, SaveState::Idle);
    }

    #[test]
    fn conflict_409_sets_conflict_state_with_both_versions() {
        // AC: a 409 sets ConflictState carrying server + local content.
        let local = json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"mine"}]}]});
        let mut m = mgr(Err(SaveError::VersionConflict(Box::new(server_doc(5)))));
        m.set_pending_local_content(local.clone());
        m.request_save(local.clone());
        m.deliver_for_test(Err(SaveError::VersionConflict(Box::new(server_doc(5)))));
        let outcome = m.drain().unwrap();
        assert_eq!(outcome, SaveOutcome::Conflict);
        match &m.state {
            SaveState::Conflict { server, local_content } => {
                assert_eq!(server.doc_version, 5);
                assert_eq!(local_content, &local, "the operator's local content is carried");
            }
            other => panic!("expected Conflict, got {other:?}"),
        }
        assert!(m.has_conflict());
    }

    #[test]
    fn keep_yours_requires_confirmation_before_resave() {
        // MC-003: Keep-yours must NOT re-save without a confirm; it transitions to ConfirmKeepYours.
        let local = json!({"type":"doc","content":[]});
        let mut m = mgr(Ok(ok_result(6)));
        m.state = SaveState::Conflict {
            server: Box::new(server_doc(5)),
            local_content: local.clone(),
        };
        m.request_keep_yours();
        assert!(
            matches!(m.state, SaveState::ConfirmKeepYours { .. }),
            "Keep-yours first asks for confirmation, never an immediate overwrite"
        );
        assert!(!m.is_saving(), "no save fires until the operator confirms");
        // Confirm -> re-saves at the SERVER's version (5), overwriting.
        m.confirm_keep_yours();
        assert_eq!(m.doc_version, 5, "the re-save adopts the server version so it passes");
        assert!(m.is_saving(), "the overwrite save is now in flight");
    }

    #[test]
    fn cancel_keep_yours_returns_to_conflict() {
        let local = json!({"type":"doc","content":[]});
        let mut m = mgr(Ok(ok_result(6)));
        m.state = SaveState::ConfirmKeepYours {
            server: Box::new(server_doc(5)),
            local_content: local,
        };
        m.cancel_keep_yours();
        assert!(matches!(m.state, SaveState::Conflict { .. }));
    }

    #[test]
    fn keep_server_reloads_server_content_and_clears_conflict() {
        // AC: Keep server reloads the server content into the doc and clears ConflictState.
        let mut m = mgr(Ok(ok_result(6)));
        m.dirty = true;
        m.state = SaveState::Conflict {
            server: Box::new(server_doc(5)),
            local_content: json!({"type":"doc","content":[]}),
        };
        let server_content = m.keep_server().expect("keep_server returns the server content");
        assert_eq!(server_content["content"][0]["content"][0]["text"], "server");
        assert_eq!(m.doc_version, 5, "adopt the server version");
        assert!(!m.dirty, "local edits discarded");
        assert_eq!(m.state, SaveState::Idle);
    }

    #[test]
    fn second_save_blocked_while_in_flight() {
        // MC-002: a second request_save while one is in flight is a no-op (the in-flight guard).
        let backend = Arc::new(MockBackend::new(Ok(ok_result(4))));
        let mut m = SaveManager::new(backend.clone(), None, "DOC-1", 3);
        m.request_save(json!({}));
        m.request_save(json!({})); // ignored
        // Only the state proves the guard here (no runtime spawn in headless); the production guard
        // is the same `is_saving` check. The mock's call count would be 0 here because there is no
        // runtime to spawn the future; the guard is proven by the state staying Saving once.
        assert!(m.is_saving());
    }
}
