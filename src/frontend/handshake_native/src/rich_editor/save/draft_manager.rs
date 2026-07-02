//! Draft / crash-recovery state machine for the rich-text editor (WP-KERNEL-012 MT-020).
//!
//! Port of the `app/src/components/RichDocumentView.tsx` draft flow (MT-255):
//! - On document mount: `GET /knowledge/documents/{id}/draft` (load). If a draft exists, its
//!   `base_doc_version` matches the current `doc_version`, AND its content differs from the
//!   server content, transition to [`DraftState::Available`] so the editor shows a "Draft
//!   recovery" banner ([Restore draft] / [Discard]).
//! - On every dirty change: a debounced (5s) `PUT /knowledge/documents/{id}/draft`
//!   `{ base_doc_version, base_content_sha256, content_json }` (upsert) — BLOCKED while a canonical
//!   save is in flight (MC-002 / red-team RISK-2: a stale-`base_doc_version` draft must never race
//!   a save).
//! - On a successful canonical save: `DELETE /knowledge/documents/{id}/draft` (clear).
//! - On "Discard": clear + [`DraftState::Discarded`].
//!
//! ## The SHA256 is the backend's canonical hash (MC-005)
//!
//! `base_content_sha256` is computed by [`super::canonical_hash::canonical_content_sha256`], which
//! is a byte-for-byte port of the backend's `knowledge_canonical_json_sha256`. If it diverged, the
//! backend would reject the upsert with HTTP 409. This is the verified-against-the-real-backend
//! seam (the MT-011 hsLink lesson), NOT the contract's stale "serde_json::to_vec" wording.
//!
//! ## Mock clock (testable debounce)
//!
//! The 5s debounce is driven by an injected "now" [`std::time::Instant`] so a unit test advances a
//! mock clock to prove the upsert fires at exactly 5s — never `std::thread::sleep`. The state
//! machine + the SHA256-in-the-request-body are fully unit-testable headless.

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Deserialize;
use serde_json::Value as JsonValue;
use thiserror::Error;

use super::canonical_hash::canonical_content_sha256;

/// The debounce window before a dirty change is upserted as a draft (the contract's 5 seconds).
pub const DRAFT_DEBOUNCE: Duration = Duration::from_secs(5);

/// The verified backend draft record (`GET /draft` returns
/// `{ rich_document_id, current_doc_version, current_content_sha256, draft }` where `draft` is this
/// shape or `null`). Field names VERIFIED READ-ONLY against
/// `src/backend/handshake_core/src/storage/knowledge.rs` (`KnowledgeRichDocumentDraft`):
/// `base_doc_version`, `base_content_sha256`, `draft_content_sha256`, `content_json`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RichDocumentDraft {
    /// The doc version the draft was based on (must match the current `doc_version` to be offered).
    #[serde(default)]
    pub base_doc_version: u64,
    /// The hash of the base (server) content the draft forked from.
    #[serde(default)]
    pub base_content_sha256: String,
    /// The hash of the draft content itself.
    #[serde(default)]
    pub draft_content_sha256: String,
    /// The draft's unsaved ProseMirror content (restored on "Restore draft").
    #[serde(default)]
    pub content_json: Option<JsonValue>,
}

/// The `GET /draft` response envelope (the verified `load_document_draft` body): the current server
/// version + hash, plus the optional `draft` (already filtered server-side to differ from the
/// current content — but the client re-checks `base_doc_version` defensively).
#[derive(Debug, Clone, Deserialize)]
pub struct RichDocumentDraftLoad {
    /// The server's current doc version (the draft is offerable only when its `base_doc_version`
    /// equals this).
    #[serde(default)]
    pub current_doc_version: u64,
    /// The persisted draft, or `None` when there is none to recover.
    #[serde(default)]
    pub draft: Option<RichDocumentDraft>,
}

/// Why a draft backend interaction failed (typed, surfaced without crashing the editor).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DraftError {
    /// A transport-layer failure (TRANSIENT).
    #[error("network error: {0}")]
    Network(String),
    /// A non-success HTTP status with the body text.
    #[error("server error {0}: {1}")]
    Server(u16, String),
}

/// A boxed future for the load op (the MT-019 transport-trait shape).
pub type DraftLoadFuture =
    Pin<Box<dyn Future<Output = Result<RichDocumentDraftLoad, DraftError>> + Send>>;
/// A boxed future for the upsert/clear ops (the body is not needed by the UI — success/failure).
pub type DraftWriteFuture = Pin<Box<dyn Future<Output = Result<(), DraftError>> + Send>>;

/// The async draft transport (load / upsert / clear). A `Send + Sync` trait so the production
/// reqwest impl and the unit mock are interchangeable.
pub trait DraftBackend: Send + Sync {
    /// `GET /knowledge/documents/{id}/draft`.
    fn load_draft(&self, document_id: &str) -> DraftLoadFuture;
    /// `PUT /knowledge/documents/{id}/draft` `{ base_doc_version, base_content_sha256, content_json }`.
    fn upsert_draft(
        &self,
        document_id: &str,
        base_doc_version: u64,
        base_content_sha256: String,
        content_json: JsonValue,
    ) -> DraftWriteFuture;
    /// `DELETE /knowledge/documents/{id}/draft`.
    fn clear_draft(&self, document_id: &str) -> DraftWriteFuture;
}

/// The production draft transport over the existing reqwest 0.12 + rustls stack.
///
/// ## The four required identity headers (the missing-headers fix)
///
/// Every draft request (`GET` / `PUT` / `DELETE /draft`) goes through the backend `doc_context`,
/// which REQUIRES `x-hsk-actor-id`, `x-hsk-kernel-task-run-id`, `x-hsk-session-run-id`, and
/// `x-hsk-actor-kind` or returns a hard 400; the upsert + clear additionally `require(Write)`, which
/// a missing `x-hsk-actor-kind` (defaulting to read-only) would 403. So all three methods attach the
/// four headers via the shared [`super::save_manager::attach_doc_headers`] helper (the SAME canonical
/// header names + `operator` actor-kind as the save transport — constructed in ONE place).
#[derive(Clone)]
pub struct ReqwestDraftBackend {
    client: reqwest::Client,
    base_url: String,
    /// A per-backend session id folded into the per-request run ids (HBR-SWARM attribution).
    session_run_id: String,
}

impl ReqwestDraftBackend {
    /// Build a backend against `base_url`.
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            session_run_id: super::save_manager::new_session_run_id(),
        }
    }
    fn draft_url(&self, document_id: &str) -> String {
        format!(
            "{}/knowledge/documents/{}/draft",
            self.base_url, document_id
        )
    }
}

impl DraftBackend for ReqwestDraftBackend {
    fn load_draft(&self, document_id: &str) -> DraftLoadFuture {
        let url = self.draft_url(document_id);
        let client = self.client.clone();
        let session_run_id = self.session_run_id.clone();
        let document_id_owned = document_id.to_owned();
        Box::pin(async move {
            let resp = super::save_manager::attach_doc_headers(
                client.get(&url).timeout(Duration::from_secs(10)),
                &document_id_owned,
                &session_run_id,
            )
            .send()
            .await
            .map_err(|e| DraftError::Network(e.to_string()))?;
            let status = resp.status();
            if !status.is_success() {
                let text = resp.text().await.unwrap_or_default();
                return Err(DraftError::Server(status.as_u16(), text));
            }
            resp.json::<RichDocumentDraftLoad>()
                .await
                .map_err(|e| DraftError::Server(status.as_u16(), e.to_string()))
        })
    }

    fn upsert_draft(
        &self,
        document_id: &str,
        base_doc_version: u64,
        base_content_sha256: String,
        content_json: JsonValue,
    ) -> DraftWriteFuture {
        let url = self.draft_url(document_id);
        let client = self.client.clone();
        let session_run_id = self.session_run_id.clone();
        let document_id_owned = document_id.to_owned();
        Box::pin(async move {
            let body = serde_json::json!({
                "base_doc_version": base_doc_version,
                "base_content_sha256": base_content_sha256,
                "content_json": content_json,
            });
            let resp = super::save_manager::attach_doc_headers(
                client
                    .put(&url)
                    .timeout(Duration::from_secs(10))
                    .json(&body),
                &document_id_owned,
                &session_run_id,
            )
            .send()
            .await
            .map_err(|e| DraftError::Network(e.to_string()))?;
            let status = resp.status();
            if !status.is_success() {
                let text = resp.text().await.unwrap_or_default();
                return Err(DraftError::Server(status.as_u16(), text));
            }
            Ok(())
        })
    }

    fn clear_draft(&self, document_id: &str) -> DraftWriteFuture {
        let url = self.draft_url(document_id);
        let client = self.client.clone();
        let session_run_id = self.session_run_id.clone();
        let document_id_owned = document_id.to_owned();
        Box::pin(async move {
            let resp = super::save_manager::attach_doc_headers(
                client.delete(&url).timeout(Duration::from_secs(10)),
                &document_id_owned,
                &session_run_id,
            )
            .send()
            .await
            .map_err(|e| DraftError::Network(e.to_string()))?;
            let status = resp.status();
            if !status.is_success() {
                let text = resp.text().await.unwrap_or_default();
                return Err(DraftError::Server(status.as_u16(), text));
            }
            Ok(())
        })
    }
}

/// One-slot delivery cell for the off-thread draft LOAD result.
pub type DraftLoadCell = Arc<Mutex<Option<Result<RichDocumentDraftLoad, DraftError>>>>;

/// The draft-recovery state. The editor holds one [`DraftManager`]; [`DraftState`] drives the
/// recovery banner.
#[derive(Debug, Clone, PartialEq)]
pub enum DraftState {
    /// No draft known yet and none being checked.
    None,
    /// A `GET /draft` is in flight (entered only when a runtime actually spawns it — no perpetual
    /// spinner headless, the MT-019 spinner-trap discipline).
    Checking,
    /// A recoverable draft exists; the banner is shown until Restore/Discard. `shown` lets the UI
    /// dismiss the banner without discarding the draft (e.g. the operator ignores it and keeps
    /// editing — the draft stays on the server).
    Available {
        draft: Box<RichDocumentDraft>,
        shown: bool,
    },
    /// The operator discarded the draft (the banner is gone and the server draft was cleared).
    Discarded,
}

/// The recorded draft-upsert request (for the test seam + debounce assertion): the exact
/// `{ base_doc_version, base_content_sha256, content_json }` that was/would be sent. The
/// `base_content_sha256` is the canonical hash of the BASE (server) content the draft forked from.
#[derive(Debug, Clone, PartialEq)]
pub struct DraftUpsertRequest {
    pub base_doc_version: u64,
    pub base_content_sha256: String,
    pub content_json: JsonValue,
}

/// The persistent draft coordinator owned by the editor.
pub struct DraftManager {
    backend: Arc<dyn DraftBackend>,
    runtime: Option<tokio::runtime::Handle>,
    document_id: String,
    /// The recovery state (drives the banner).
    pub state: DraftState,
    /// When the document first became dirty in the current debounce window (`None` when clean). The
    /// debounced upsert fires once `now - dirty_since >= DRAFT_DEBOUNCE`.
    dirty_since: Option<Instant>,
    /// The base content hash the next draft upsert reports — the canonical hash of the SERVER
    /// content the current edits forked from (NOT the draft content). Set on load.
    base_content_sha256: String,
    /// The base doc version the next draft upsert reports (the server version the edits forked
    /// from). Set on load; equals the save manager's `doc_version` at load time.
    base_doc_version: u64,
    /// The load-result delivery cell.
    load_cell: DraftLoadCell,
    /// TEST SEAM: the last upsert the manager dispatched (so a headless test asserts the exact
    /// `base_content_sha256` matches the canonical hash without a live backend).
    pub last_upsert: Option<DraftUpsertRequest>,
}

impl DraftManager {
    /// Build a manager for `document_id` with `backend` + an optional runtime. `base_doc_version` +
    /// `base_content` seed the upsert base (the server content the edits fork from).
    pub fn new(
        backend: Arc<dyn DraftBackend>,
        runtime: Option<tokio::runtime::Handle>,
        document_id: impl Into<String>,
        base_doc_version: u64,
        base_content: &JsonValue,
    ) -> Self {
        Self {
            backend,
            runtime,
            document_id: document_id.into(),
            state: DraftState::None,
            dirty_since: None,
            base_content_sha256: canonical_content_sha256(base_content),
            base_doc_version,
            load_cell: Arc::new(Mutex::new(None)),
            last_upsert: None,
        }
    }

    /// The production manager over the reqwest backend + the app runtime handle.
    pub fn production(
        runtime: tokio::runtime::Handle,
        document_id: impl Into<String>,
        base_doc_version: u64,
        base_content: &JsonValue,
    ) -> Self {
        let backend = Arc::new(ReqwestDraftBackend::new(
            crate::backend_client::BACKEND_BASE_URL,
        ));
        Self::new(
            backend,
            Some(runtime),
            document_id,
            base_doc_version,
            base_content,
        )
    }

    /// On document mount: spawn `GET /draft`. Only enters `Checking` when a runtime actually
    /// spawns (no perpetual spinner headless). A headless test stages [`Self::deliver_load_for_test`].
    pub fn check_on_mount(&mut self) {
        if let Ok(mut slot) = self.load_cell.lock() {
            *slot = None;
        }
        if let Some(rt) = &self.runtime {
            self.state = DraftState::Checking;
            let backend = Arc::clone(&self.backend);
            let cell = Arc::clone(&self.load_cell);
            let document_id = self.document_id.clone();
            rt.spawn(async move {
                let result = backend.load_draft(&document_id).await;
                if let Ok(mut slot) = cell.lock() {
                    *slot = Some(result);
                }
            });
        }
    }

    /// Drain a completed load result (called each frame). If a draft exists, its `base_doc_version`
    /// matches the load's `current_doc_version`, and it carries content, enter `Available` (banner
    /// shows). Otherwise `None`. A failure is swallowed to `None` (no draft offered, no crash).
    /// Returns `true` when a draft became available this frame.
    pub fn drain_load(&mut self) -> bool {
        let Some(delivered) = self.load_cell.lock().ok().and_then(|mut s| s.take()) else {
            return false;
        };
        match delivered {
            Ok(load) => match load.draft {
                Some(draft)
                    // The draft must be based on the SAME server version we loaded, and carry content.
                    if draft.base_doc_version == load.current_doc_version
                        && draft.content_json.is_some() =>
                {
                    self.state = DraftState::Available {
                        draft: Box::new(draft),
                        shown: true,
                    };
                    true
                }
                _ => {
                    self.state = DraftState::None;
                    false
                }
            },
            Err(_) => {
                self.state = DraftState::None;
                false
            }
        }
    }

    /// Mark the document dirty (an edit happened). Starts the debounce window on the FIRST dirty
    /// change since the last upsert/clear; subsequent dirty marks within the window do not reset it
    /// (so continuous typing still upserts every ~5s, matching the React debounce).
    pub fn mark_dirty(&mut self, now: Instant) {
        if self.dirty_since.is_none() {
            self.dirty_since = Some(now);
        }
    }

    /// True when the debounce window has elapsed and a draft upsert is due.
    pub fn upsert_due(&self, now: Instant) -> bool {
        match self.dirty_since {
            Some(since) => now.duration_since(since) >= DRAFT_DEBOUNCE,
            None => false,
        }
    }

    /// Fire the debounced draft upsert for `content_json` IF due AND no canonical save is in flight
    /// (`save_in_flight` — MC-002 / RISK-2: a stale-base draft must never race a save). Computes
    /// `base_content_sha256` as the canonical hash of the BASE server content (set on load),
    /// records the request (test seam), spawns the PUT, and resets the debounce window. Returns
    /// `true` when an upsert was dispatched.
    pub fn maybe_upsert(
        &mut self,
        content_json: JsonValue,
        now: Instant,
        save_in_flight: bool,
    ) -> bool {
        if save_in_flight {
            return false; // MC-002: never upsert a draft during a canonical save.
        }
        if !self.upsert_due(now) {
            return false;
        }
        let request = DraftUpsertRequest {
            base_doc_version: self.base_doc_version,
            base_content_sha256: self.base_content_sha256.clone(),
            content_json: content_json.clone(),
        };
        self.last_upsert = Some(request.clone());
        if let Some(rt) = &self.runtime {
            let backend = Arc::clone(&self.backend);
            let document_id = self.document_id.clone();
            rt.spawn(async move {
                let _ = backend
                    .upsert_draft(
                        &document_id,
                        request.base_doc_version,
                        request.base_content_sha256,
                        request.content_json,
                    )
                    .await;
            });
        }
        self.dirty_since = None; // reset the window; the next dirty change starts a fresh one.
        true
    }

    /// Clear the draft after a successful canonical save (`DELETE /draft`). Resets the debounce
    /// window and re-bases on the new server version (the just-saved content). The caller passes the
    /// new `doc_version` + the saved content so the next draft (if any) bases correctly.
    pub fn clear_after_save(&mut self, new_doc_version: u64, saved_content: &JsonValue) {
        self.base_doc_version = new_doc_version;
        self.base_content_sha256 = canonical_content_sha256(saved_content);
        self.dirty_since = None;
        self.state = DraftState::None;
        self.spawn_clear();
    }

    /// "Restore draft": adopt the available draft's content. Returns the draft `content_json` (the
    /// caller rebuilds the doc from it) and transitions the state to `None` (the banner is gone; the
    /// restored content is now the live doc, dirty). Clears the server draft.
    pub fn restore_draft(&mut self) -> Option<JsonValue> {
        let DraftState::Available { draft, .. } = &self.state else {
            return None;
        };
        let content = draft.content_json.clone();
        self.state = DraftState::None;
        self.spawn_clear();
        content
    }

    /// "Discard": clear the server draft and transition to `Discarded` (the banner is gone, the
    /// live doc keeps the server content).
    pub fn discard_draft(&mut self) {
        self.state = DraftState::Discarded;
        self.spawn_clear();
    }

    /// Dismiss the recovery banner WITHOUT discarding (the operator ignores it; the draft stays on
    /// the server for a later session). Only meaningful in `Available`.
    pub fn dismiss_banner(&mut self) {
        if let DraftState::Available { draft, .. } = &self.state {
            self.state = DraftState::Available {
                draft: draft.clone(),
                shown: false,
            };
        }
    }

    /// True when the recovery banner should render (a draft is available AND not dismissed).
    pub fn banner_visible(&self) -> bool {
        matches!(self.state, DraftState::Available { shown: true, .. })
    }

    /// Spawn the `DELETE /draft` clear (off-thread; no result needed by the UI). No-op headless.
    fn spawn_clear(&self) {
        if let Some(rt) = &self.runtime {
            let backend = Arc::clone(&self.backend);
            let document_id = self.document_id.clone();
            rt.spawn(async move {
                let _ = backend.clear_draft(&document_id).await;
            });
        }
    }

    /// The document id (for tests).
    pub fn document_id(&self) -> &str {
        &self.document_id
    }

    /// TEST SEAM: stage a load result as if `GET /draft` resolved (headless). The next
    /// [`Self::drain_load`] applies it.
    pub fn deliver_load_for_test(&self, result: Result<RichDocumentDraftLoad, DraftError>) {
        if let Ok(mut slot) = self.load_cell.lock() {
            *slot = Some(result);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// A mock backend that records upsert/clear calls (so a test asserts the dispatched body). The
    /// futures resolve immediately; in headless tests we never run them (no runtime), but the
    /// recorded `last_upsert` on the manager proves the request shape.
    struct MockDraftBackend {
        load: Mutex<Option<Result<RichDocumentDraftLoad, DraftError>>>,
    }
    impl MockDraftBackend {
        fn new(load: Result<RichDocumentDraftLoad, DraftError>) -> Self {
            Self {
                load: Mutex::new(Some(load)),
            }
        }
    }
    impl DraftBackend for MockDraftBackend {
        fn load_draft(&self, _id: &str) -> DraftLoadFuture {
            let staged = self.load.lock().unwrap().clone();
            Box::pin(
                async move { staged.unwrap_or(Err(DraftError::Network("no staged load".into()))) },
            )
        }
        fn upsert_draft(&self, _id: &str, _v: u64, _h: String, _c: JsonValue) -> DraftWriteFuture {
            Box::pin(async { Ok(()) })
        }
        fn clear_draft(&self, _id: &str) -> DraftWriteFuture {
            Box::pin(async { Ok(()) })
        }
    }

    fn base_content() -> JsonValue {
        json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"server"}]}]})
    }

    fn mgr() -> DraftManager {
        let backend = Arc::new(MockDraftBackend::new(Ok(RichDocumentDraftLoad {
            current_doc_version: 3,
            draft: None,
        })));
        DraftManager::new(backend, None, "DOC-1", 3, &base_content())
    }

    #[test]
    fn draft_load_with_matching_base_version_sets_available() {
        // AC: a loadRichDocumentDraft response containing a draft sets DraftState::Available.
        let mut m = mgr();
        let draft = RichDocumentDraft {
            base_doc_version: 3,
            base_content_sha256: canonical_content_sha256(&base_content()),
            draft_content_sha256: "deadbeef".into(),
            content_json: Some(
                json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"draft text"}]}]}),
            ),
        };
        m.deliver_load_for_test(Ok(RichDocumentDraftLoad {
            current_doc_version: 3,
            draft: Some(draft),
        }));
        assert!(m.drain_load(), "a matching draft becomes Available");
        assert!(m.banner_visible(), "the recovery banner shows");
        assert!(matches!(m.state, DraftState::Available { shown: true, .. }));
    }

    #[test]
    fn draft_load_with_stale_base_version_is_ignored() {
        // A draft based on an older version than the loaded current_doc_version is NOT offered.
        let mut m = mgr();
        let draft = RichDocumentDraft {
            base_doc_version: 2, // stale: server is now at 3
            base_content_sha256: "x".into(),
            draft_content_sha256: "y".into(),
            content_json: Some(json!({"type":"doc","content":[]})),
        };
        m.deliver_load_for_test(Ok(RichDocumentDraftLoad {
            current_doc_version: 3,
            draft: Some(draft),
        }));
        assert!(!m.drain_load());
        assert_eq!(m.state, DraftState::None);
    }

    #[test]
    fn restore_draft_returns_content_and_clears_banner() {
        // AC: Restore draft loads the draft content into the DocModel.
        let mut m = mgr();
        let draft_content = json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"restored"}]}]});
        m.state = DraftState::Available {
            draft: Box::new(RichDocumentDraft {
                base_doc_version: 3,
                base_content_sha256: "a".into(),
                draft_content_sha256: "b".into(),
                content_json: Some(draft_content.clone()),
            }),
            shown: true,
        };
        let restored = m
            .restore_draft()
            .expect("restore returns the draft content");
        assert_eq!(restored, draft_content);
        assert_eq!(m.state, DraftState::None, "banner gone after restore");
    }

    #[test]
    fn discard_sets_discarded_state() {
        // AC: Discard sets DraftState::Discarded (and would DELETE the draft on a live backend).
        let mut m = mgr();
        m.state = DraftState::Available {
            draft: Box::new(RichDocumentDraft {
                base_doc_version: 3,
                base_content_sha256: "a".into(),
                draft_content_sha256: "b".into(),
                content_json: Some(json!({})),
            }),
            shown: true,
        };
        m.discard_draft();
        assert_eq!(m.state, DraftState::Discarded);
        assert!(!m.banner_visible());
    }

    #[test]
    fn upsert_fires_after_5s_and_sha_matches_canonical_hash() {
        // AC: draft upsert fires after 5s of dirty state; the SHA256 in the request body matches the
        // canonical hash of the base content (MC-005). Mock clock — no thread::sleep.
        let mut m = mgr();
        let t0 = Instant::now();
        m.mark_dirty(t0);
        // Not yet due at 4.9s.
        assert!(!m.maybe_upsert(
            json!({"type":"doc","content":[]}),
            t0 + Duration::from_millis(4900),
            false
        ));
        assert!(m.last_upsert.is_none());
        // Due at exactly 5s.
        let content = json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"edit"}]}]});
        let fired = m.maybe_upsert(content.clone(), t0 + Duration::from_secs(5), false);
        assert!(fired, "the upsert fires at the 5s debounce boundary");
        let req = m.last_upsert.clone().expect("an upsert was recorded");
        assert_eq!(req.base_doc_version, 3);
        assert_eq!(req.content_json, content);
        // The exact assertion: base_content_sha256 == the canonical hash of the base content.
        assert_eq!(
            req.base_content_sha256,
            canonical_content_sha256(&base_content())
        );
    }

    #[test]
    fn upsert_blocked_while_save_in_flight() {
        // MC-002 / RISK-2: a draft upsert never fires while a canonical save is in flight, even if
        // the debounce is due (a stale-base draft must not race a save).
        let mut m = mgr();
        let t0 = Instant::now();
        m.mark_dirty(t0);
        let fired = m.maybe_upsert(
            json!({}),
            t0 + Duration::from_secs(10),
            /* save_in_flight */ true,
        );
        assert!(!fired, "no draft upsert while a save is in flight");
        assert!(m.last_upsert.is_none());
    }

    #[test]
    fn clear_after_save_rebases_and_resets_debounce() {
        let mut m = mgr();
        m.mark_dirty(Instant::now());
        let saved = json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"saved"}]}]});
        m.clear_after_save(4, &saved);
        assert_eq!(m.base_doc_version, 4, "re-based on the new server version");
        assert_eq!(m.base_content_sha256, canonical_content_sha256(&saved));
        assert!(
            !m.upsert_due(Instant::now() + Duration::from_secs(10)),
            "debounce reset"
        );
        assert_eq!(m.state, DraftState::None);
    }
}
