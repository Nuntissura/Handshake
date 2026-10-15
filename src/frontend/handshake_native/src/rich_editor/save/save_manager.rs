//! Owner-gated canonical document saving and explicit uncertain-outcome recovery.
//!
//! Admission and local completion effects run under the canonical document gate.
//! Opaque attempts launch the existing PUT or authenticated recovery GET after unlocking.
//! One active save serializes the latest queued snapshot; only a current-edit ACK clears dirty.
//! Unknown outcomes retain content until explicit reload/conflict recovery. A GET never proves
//! the earlier save outcome. Draft network ordering remains a separate acceptance obligation.

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use serde::Deserialize;
use serde_json::Value as JsonValue;
use thiserror::Error;
use crate::rich_editor::document_owner::{DocumentIdentity, DocumentSaveGate, DocumentSaveSnapshot, EditorSaveContext};

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
    /// Number of backlinks persisted by the server's post-commit index pass.
    pub backlinks_persisted: usize,
    /// A recorded post-commit index failure. The document save is still canonical.
    pub backlinks_error: Option<String>,
    /// Why indexing was intentionally skipped, when applicable.
    pub backlinks_skipped_reason: Option<String>,
    /// Canonical EventLedger receipt returned by the knowledge-document save endpoint.
    #[serde(default)]
    pub save_receipt_event_id: Option<String>,
    /// Exact request identity that caused this receipt. Production transports populate it from the
    /// immutable headers sent on the successful request; test backends may leave it absent.
    #[serde(default)]
    pub attribution: Option<SaveAttribution>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SaveAttribution {
    pub actor_id: String,
    pub actor_kind: String,
    pub kernel_task_run_id: String,
    pub session_run_id: String,
    pub correlation_id: Option<String>,
}

/// Why a save failed. A [`SaveError::VersionConflict`] carries the server document so the conflict
/// UI can show both versions (the contract's typed-variant set).
#[derive(Debug, Clone, PartialEq, Error)]
pub enum SaveError {
    /// HTTP 409: the `expected_version` did not match the server's current version. Carries the
    /// server document for the conflict panel.
    #[error("version conflict (server is ahead)")]
    VersionConflict(Box<RichDocLoad>),
    /// A transport failure can leave the canonical save outcome unknown; recovery precedes retry.
    #[error("network error: {0}")]
    Network(String),
    /// An HTTP failure or an unreadable success body; a 2xx parse error does not prove rejection.
    #[error("server error {0}: {1}")]
    Server(u16, String),
    /// The server rejected the content_json (e.g. malformed block tree) — a 400.
    #[error("schema rejected: {0}")]
    SchemaRejected(String),
}

/// A boxed future returned by [`SaveBackend`] (the MT-019 transport-trait shape).
pub type SaveFuture = Pin<Box<dyn Future<Output = Result<RichDocSaveResult, SaveError>> + Send>>;

pub type RecoveryLoadFuture = Pin<
    Box<
        dyn Future<
                Output = Result<
                    crate::backend::knowledge_documents::DocumentLoadResponse,
                    SaveError,
                >,
            > + Send,
    >,
>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SaveDispatchStamp {
    pub owner: crate::rich_editor::document_owner::EditorSaveContext,
    pub save_id: u64,
}

impl SaveDispatchStamp {
    fn accepts_context(
        self,
        current: Option<crate::rich_editor::document_owner::EditorSaveContext>,
    ) -> bool {
        current.is_some_and(|current| {
            current.document_incarnation == self.owner.document_incarnation
                && current.context_generation == self.owner.context_generation
        })
    }
}

struct PendingSave {
    snapshot: crate::rich_editor::document_owner::DocumentSaveSnapshot,
    stamp: SaveDispatchStamp,
    expected_version: u64,
    workspace_id: String,
}

struct SaveDelivery {
    stamp: SaveDispatchStamp,
    result: Result<RichDocSaveResult, SaveError>,
}

pub struct AdmittedSaveAttempt {
    backend: Arc<dyn SaveBackend>,
    runtime: Option<tokio::runtime::Handle>,
    cell: Arc<Mutex<Option<SaveDelivery>>>,
    document_id: String,
    content_json: JsonValue,
    expected_version: u64,
    stamp: SaveDispatchStamp,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RecoveryLoadStamp {
    recovery_id: u64,
    unknown_dispatch: SaveDispatchStamp,
    owner: crate::rich_editor::document_owner::EditorSaveContext,
    document_id: String,
    kind: RecoveryKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RecoveryKind {
    UnknownOutcome,
    ConflictRefresh,
}

struct RecoveryDelivery {
    stamp: RecoveryLoadStamp,
    result: Result<crate::backend::knowledge_documents::DocumentLoadResponse, SaveError>,
}

pub struct AdmittedRecoveryLoad {
    backend: Arc<dyn SaveBackend>,
    runtime: Option<tokio::runtime::Handle>,
    cell: Arc<Mutex<Option<RecoveryDelivery>>>,
    stamp: RecoveryLoadStamp,
}

fn decode_recovery_document(
    document: JsonValue,
    document_id: &str,
    known_version: u64,
) -> Result<RichDocLoad, SaveError> {
    let invalid = |detail: &str| SaveError::Server(200, format!("invalid recovery document: {detail}"));
    if document.get("rich_document_id").and_then(JsonValue::as_str) != Some(document_id) {
        return Err(invalid("document identity does not match the recovery target"));
    }
    let version = document
        .get("doc_version")
        .and_then(JsonValue::as_u64)
        .ok_or_else(|| invalid("an explicit nonnegative document version is required"))?;
    if version < known_version {
        return Err(invalid("document version is older than the confirmed version"));
    }
    let content = document
        .get("content_json")
        .ok_or_else(|| invalid("canonical content is missing"))?;
    crate::rich_editor::document_model::doc_json::from_json_value(content)
        .map_err(|_| invalid("canonical content is malformed"))?;
    serde_json::from_value(document).map_err(|error| invalid(&error.to_string()))
}

/// The async save transport. A `Send + Sync` trait so the production reqwest impl and the unit
/// mock are interchangeable; the manager spawns the returned future on the app runtime.
pub trait SaveBackend: Send + Sync {
    fn load_document(&self, document_id: &str) -> RecoveryLoadFuture;

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
    authenticated_context: Option<Arc<crate::local_account::AuthenticatedContext>>,
    /// A per-backend session id folded into the per-request run ids so every save is individually
    /// traceable (HBR-SWARM attribution). Stable for the lifetime of one editor's save manager.
    session_run_id: String,
}

impl ReqwestSaveBackend {
    /// Build a backend against `base_url` (e.g. [`crate::backend_client::BACKEND_BASE_URL`]).
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: crate::backend_client::shared_http_client(),
            base_url: base_url.into(),
            session_run_id: new_session_run_id(),
            authenticated_context: None,
        }
    }

    pub fn with_authenticated_context(
        mut self,
        context: Option<Arc<crate::local_account::AuthenticatedContext>>,
    ) -> Self {
        self.authenticated_context = context;
        self
    }

    fn save_url(&self, document_id: &str) -> String {
        format!("{}/knowledge/documents/{}/save", self.base_url, document_id)
    }
}

impl SaveBackend for ReqwestSaveBackend {
    fn load_document(&self, document_id: &str) -> RecoveryLoadFuture {
        crate::backend_client::RichDocSaveBackend::new(self.base_url.clone())
            .with_authenticated_context(self.authenticated_context.clone())
            .load_document(document_id)
    }

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
                let mut result: RichDocSaveResult = resp
                    .json()
                    .await
                    .map_err(|e| SaveError::Server(status.as_u16(), e.to_string()))?;
                result.attribution = Some(SaveAttribution {
                    actor_id: crate::backend_client::DOC_ACTOR_ID.to_owned(),
                    actor_kind: crate::backend_client::DOC_ACTOR_KIND.to_owned(),
                    kernel_task_run_id: format!("native-editor-doc-{document_id_owned}"),
                    session_run_id: session_run_id.clone(),
                    correlation_id: Some(format!(
                        "native-editor-save-{document_id_owned}-{session_run_id}"
                    )),
                });
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
    use crate::backend::knowledge_documents::HSK_HEADER_CORRELATION_ID;
    use crate::backend_client::{
        DOC_ACTOR_ID, DOC_ACTOR_KIND, HSK_HEADER_ACTOR_ID, HSK_HEADER_ACTOR_KIND,
        HSK_HEADER_KERNEL_TASK_RUN_ID, HSK_HEADER_SESSION_RUN_ID,
    };
    builder
        .header(HSK_HEADER_ACTOR_ID, DOC_ACTOR_ID)
        .header(HSK_HEADER_ACTOR_KIND, DOC_ACTOR_KIND)
        .header(
            HSK_HEADER_KERNEL_TASK_RUN_ID,
            format!("native-editor-doc-{document_id}"),
        )
        .header(HSK_HEADER_SESSION_RUN_ID, session_run_id)
        .header(
            HSK_HEADER_CORRELATION_ID,
            format!("native-editor-save-{document_id}-{session_run_id}"),
        )
}

type SaveDeliveryCell = Arc<Mutex<Option<SaveDelivery>>>;

#[derive(Debug, Clone, PartialEq)]
pub enum SaveState {
    Idle,
    Saving { expected_version: u64 },
    Conflict { server: Box<RichDocLoad>, local_content: JsonValue },
    ConfirmKeepYours { server: Box<RichDocLoad>, local_content: JsonValue },
    Error(String),
}

pub struct SaveManager {
    backend: Arc<dyn SaveBackend>,
    runtime: Option<tokio::runtime::Handle>,
    document_id: String,
    pub doc_version: u64,
    pub dirty: bool,
    pub updated_at: Option<String>,
    pub last_save_receipt_event_id: Option<String>,
    pub last_save_attribution: Option<SaveAttribution>,
    pub state: SaveState,
    cell: SaveDeliveryCell,
    workspace_id: String,
    owner_binding: Option<(DocumentIdentity, EditorSaveContext)>,
    next_save_id: u64,
    active: Option<PendingSave>,
    queued: Option<DocumentSaveSnapshot>,
    recovery_snapshot: Option<PendingSave>,
    uncertain: bool,
    choice_context: Option<EditorSaveContext>,
    recovery_choice: bool,
    next_recovery_id: u64,
    recovery_active: Option<RecoveryLoadStamp>,
    recovery_cell: Arc<Mutex<Option<RecoveryDelivery>>>,
    recovery_local_snapshot: Option<DocumentSaveSnapshot>,
}

pub struct SaveTransition {
    pub outcome: Option<SaveOutcome>,
    pub next_attempt: Option<AdmittedSaveAttempt>,
}

impl SaveTransition {
    fn none() -> Self { Self { outcome: None, next_attempt: None } }
    fn outcome(outcome: SaveOutcome) -> Self {
        Self { outcome: Some(outcome), next_attempt: None }
    }
}

pub enum SaveOutcome {
    Saved {
        doc_version: u64,
        backlinks_persisted: usize,
        backlinks_warning: Option<String>,
        saved_content: JsonValue,
        saved_snapshot: DocumentSaveSnapshot,
        workspace_id: String,
        save_receipt_event_id: Option<String>,
        attribution: Option<SaveAttribution>,
        dispatch_stamp: SaveDispatchStamp,
        current_edit_saved: bool,
    },
    Conflict { dispatch_stamp: SaveDispatchStamp },
    Failed { error: SaveError, dispatch_stamp: SaveDispatchStamp, uncertain: bool },
    Stale { dispatch_stamp: SaveDispatchStamp },
}

impl std::fmt::Debug for SaveOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Saved { doc_version, dispatch_stamp, current_edit_saved, .. } =>
                f.debug_struct("Saved").field("doc_version", doc_version)
                    .field("dispatch_stamp", dispatch_stamp)
                    .field("current_edit_saved", current_edit_saved).finish_non_exhaustive(),
            Self::Conflict { dispatch_stamp } =>
                f.debug_struct("Conflict").field("dispatch_stamp", dispatch_stamp).finish(),
            Self::Failed { error, dispatch_stamp, uncertain } =>
                f.debug_struct("Failed").field("error", error)
                    .field("dispatch_stamp", dispatch_stamp).field("uncertain", uncertain).finish(),
            Self::Stale { dispatch_stamp } =>
                f.debug_struct("Stale").field("dispatch_stamp", dispatch_stamp).finish(),
        }
    }
}

#[derive(Debug)]
pub enum RecoveryOutcome {
    Ready { dispatch_stamp: SaveDispatchStamp, server_version: u64 },
    Failed(SaveError),
    Stale,
}

impl SaveManager {
    pub fn new(
        backend: Arc<dyn SaveBackend>,
        runtime: Option<tokio::runtime::Handle>,
        document_id: impl Into<String>,
        doc_version: u64,
    ) -> Self {
        Self {
            backend, runtime, document_id: document_id.into(), doc_version,
            dirty: false, updated_at: None, last_save_receipt_event_id: None,
            last_save_attribution: None, state: SaveState::Idle,
            cell: Arc::new(Mutex::new(None)), workspace_id: String::new(),
            owner_binding: None, next_save_id: 1, active: None, queued: None,
            recovery_snapshot: None, uncertain: false, choice_context: None,
            recovery_choice: false, next_recovery_id: 1, recovery_active: None,
            recovery_cell: Arc::new(Mutex::new(None)),
            recovery_local_snapshot: None,
        }
    }

    pub fn production(
        runtime: tokio::runtime::Handle,
        document_id: impl Into<String>,
        doc_version: u64,
    ) -> Self {
        Self::new(
            Arc::new(ReqwestSaveBackend::new(crate::backend_client::BACKEND_BASE_URL)),
            Some(runtime), document_id, doc_version,
        )
    }

    pub fn bind_owner(&mut self, gate: &mut DocumentSaveGate<'_>) -> bool {
        let (Some(identity), Some(context)) = (gate.document_identity(), gate.current_context()) else {
            return false;
        };
        if identity.document_id() != self.document_id { return false; }
        if let Some((bound, original)) = &self.owner_binding {
            return *bound == identity && same_binding(*original, context);
        }
        self.workspace_id = identity.workspace_id().to_owned();
        self.doc_version = self.doc_version.max(gate.known_server_version());
        self.dirty = gate.is_dirty();
        self.owner_binding = Some((identity, context));
        true
    }

    fn accepts_gate(&self, gate: &DocumentSaveGate<'_>) -> bool {
        match (&self.owner_binding, gate.document_identity(), gate.current_context()) {
            (Some((identity, context)), Some(live_identity), Some(current)) =>
                *identity == live_identity && same_binding(*context, current),
            _ => false,
        }
    }

    fn accepts_snapshot(&self, snapshot: &DocumentSaveSnapshot, gate: &DocumentSaveGate<'_>) -> bool {
        self.accepts_gate(gate)
            && gate.document_identity().as_ref() == Some(snapshot.document_identity())
            && gate.current_context().is_some_and(|current|
                same_binding(snapshot.context(), current)
                    && snapshot.context().edit_revision <= current.edit_revision)
    }

    pub fn is_saving(&self) -> bool { matches!(self.state, SaveState::Saving { .. }) }
    pub fn has_conflict(&self) -> bool {
        matches!(self.state, SaveState::Conflict { .. } | SaveState::ConfirmKeepYours { .. })
    }
    pub fn requires_recovery(&self) -> bool { self.uncertain }
    pub fn mark_dirty(&mut self) { self.dirty = true; }
    pub fn set_workspace_id(&mut self, workspace_id: impl Into<String>) {
        self.workspace_id = workspace_id.into();
    }
    pub fn document_id(&self) -> &str { &self.document_id }

    pub fn request_save(
        &mut self,
        snapshot: DocumentSaveSnapshot,
        gate: &mut DocumentSaveGate<'_>,
    ) -> Option<AdmittedSaveAttempt> {
        if !self.accepts_snapshot(&snapshot, gate)
            || gate.current_context() != Some(snapshot.context())
            || self.uncertain || self.has_conflict()
        { return None; }
        self.dirty = true;
        if self.active.is_some() {
            self.queued = Some(snapshot);
            return None;
        }
        self.queued = None;
        self.admit(snapshot, gate)
    }

    fn admit(&mut self, snapshot: DocumentSaveSnapshot, gate: &DocumentSaveGate<'_>)
        -> Option<AdmittedSaveAttempt>
    {
        if !self.accepts_snapshot(&snapshot, gate) { return None; }
        let Some(next) = self.next_save_id.checked_add(1) else {
            self.state = SaveState::Error("save identity counter exhausted".to_owned());
            return None;
        };
        let stamp = SaveDispatchStamp { owner: snapshot.context(), save_id: self.next_save_id };
        self.next_save_id = next;
        let expected_version = self.doc_version.max(gate.known_server_version());
        let content_json = snapshot.content_json().clone();
        let workspace_id = snapshot.document_identity().workspace_id().to_owned();
        self.cell = Arc::new(Mutex::new(None));
        self.state = SaveState::Saving { expected_version };
        self.active = Some(PendingSave { snapshot, stamp, expected_version, workspace_id });
        Some(AdmittedSaveAttempt {
            backend: Arc::clone(&self.backend), runtime: self.runtime.clone(),
            cell: Arc::clone(&self.cell), document_id: self.document_id.clone(),
            content_json, expected_version, stamp,
        })
    }

    pub fn launch_admitted(attempt: AdmittedSaveAttempt) {
        let AdmittedSaveAttempt { backend, runtime, cell, document_id,
            content_json, expected_version, stamp } = attempt;
        if let Some(runtime) = runtime {
            runtime.spawn(async move {
                let result = backend.save_document(&document_id, content_json, expected_version).await;
                if let Ok(mut delivery) = cell.lock() { *delivery = Some(SaveDelivery { stamp, result }); }
            });
        }
    }

    pub fn drain(&mut self, gate: &mut DocumentSaveGate<'_>) -> SaveTransition {
        let delivered = self.cell.lock().ok().and_then(|mut cell| cell.take());
        let Some(delivered) = delivered else { return SaveTransition::none(); };
        let stamp = delivered.stamp;
        if self.active.as_ref().map(|pending| pending.stamp) != Some(stamp) {
            return SaveTransition::outcome(SaveOutcome::Stale { dispatch_stamp: stamp });
        }
        if !self.accepts_gate(gate) || !stamp.accepts_context(gate.current_context()) {
            self.recovery_snapshot = self.active.take();
            self.queued = None;
            self.uncertain = true;
            self.state = SaveState::Error("save completed for an inactive document context; recovery retained".to_owned());
            return SaveTransition::outcome(SaveOutcome::Stale { dispatch_stamp: stamp });
        }
        let pending = self.active.take().expect("full active dispatch was matched");
        match delivered.result {
            Ok(result) => {
                let floor = self.doc_version.max(gate.known_server_version()).max(pending.expected_version);
                if result.document.rich_document_id != self.document_id
                    || result.document.doc_version < floor
                {
                    return self.retain_failure(pending,
                        SaveError::Server(200, "save acknowledgement did not match the target/current version".to_owned()), true);
                }
                let current_edit_saved = gate.record_save_commit(stamp.owner, result.document.doc_version);
                self.doc_version = result.document.doc_version;
                self.dirty = !current_edit_saved;
                self.updated_at = result.document.updated_at.clone();
                self.last_save_receipt_event_id = result.save_receipt_event_id.clone();
                self.last_save_attribution = result.attribution.clone();
                self.state = SaveState::Idle;
                self.recovery_snapshot = None;
                self.uncertain = false;
                let saved_content = pending.snapshot.content_json().clone();
                let outcome = SaveOutcome::Saved {
                    doc_version: self.doc_version, backlinks_persisted: result.backlinks_persisted,
                    backlinks_warning: result.backlinks_error.or(result.backlinks_skipped_reason),
                    saved_content, saved_snapshot: pending.snapshot,
                    workspace_id: pending.workspace_id, save_receipt_event_id: result.save_receipt_event_id,
                    attribution: result.attribution, dispatch_stamp: stamp, current_edit_saved,
                };
                let next_attempt = self.queued.take().and_then(|snapshot| self.admit(snapshot, gate));
                SaveTransition { outcome: Some(outcome), next_attempt }
            }
            Err(SaveError::VersionConflict(server)) => {
                if server.rich_document_id != self.document_id
                    || server.doc_version < gate.known_server_version()
                {
                    return self.retain_failure(pending,
                        SaveError::Server(409, "conflict response requires authenticated recovery".to_owned()), true);
                }
                self.state = SaveState::Conflict {
                    server, local_content: pending.snapshot.content_json().clone(),
                };
                self.choice_context = gate.current_context();
                self.recovery_choice = false;
                self.recovery_snapshot = Some(pending);
                self.queued = None;
                self.dirty = true;
                SaveTransition::outcome(SaveOutcome::Conflict { dispatch_stamp: stamp })
            }
            Err(error) => {
                let uncertain = !matches!(error, SaveError::SchemaRejected(_));
                self.retain_failure(pending, error, uncertain)
            }
        }
    }

    fn retain_failure(&mut self, pending: PendingSave, error: SaveError, uncertain: bool) -> SaveTransition {
        let stamp = pending.stamp;
        self.state = SaveState::Error(if uncertain {
            format!("Save outcome is uncertain; reload before another save: {error}")
        } else { error.to_string() });
        self.recovery_snapshot = Some(pending);
        self.uncertain = uncertain;
        self.dirty = true;
        SaveTransition::outcome(SaveOutcome::Failed { error, dispatch_stamp: stamp, uncertain })
    }

    pub fn request_recovery_load(&mut self, gate: &mut DocumentSaveGate<'_>)
        -> Option<AdmittedRecoveryLoad>
    {
        if !self.uncertain || !self.accepts_gate(gate)
            || self.active.is_some() || self.recovery_active.is_some()
        { return None; }
        let unknown = self.recovery_snapshot.as_ref()?;
        let owner = gate.current_context()?;
        if !same_binding(unknown.stamp.owner, owner) { return None; }
        let next = self.next_recovery_id.checked_add(1)?;
        let stamp = RecoveryLoadStamp {
            recovery_id: self.next_recovery_id, unknown_dispatch: unknown.stamp,
            owner, document_id: self.document_id.clone(), kind: RecoveryKind::UnknownOutcome,
        };
        self.next_recovery_id = next;
        self.recovery_cell = Arc::new(Mutex::new(None));
        self.recovery_active = Some(stamp.clone());
        self.recovery_local_snapshot = None;
        Some(AdmittedRecoveryLoad {
            backend: Arc::clone(&self.backend), runtime: self.runtime.clone(),
            cell: Arc::clone(&self.recovery_cell), stamp,
        })
    }

    pub fn conflict_refresh_required(&self, gate: &DocumentSaveGate<'_>) -> bool {
        self.accepts_gate(gate) && !self.uncertain && self.has_conflict()
            && self.choice_context.is_some() && self.choice_context != gate.current_context()
    }

    pub fn request_conflict_refresh(
        &mut self,
        snapshot: DocumentSaveSnapshot,
        gate: &mut DocumentSaveGate<'_>,
    ) -> Option<AdmittedRecoveryLoad> {
        if !self.conflict_refresh_required(gate) || self.active.is_some()
            || self.recovery_active.is_some() || !self.accepts_snapshot(&snapshot, gate)
            || gate.current_context() != Some(snapshot.context())
        { return None; }
        let original = self.recovery_snapshot.as_ref()?;
        let next = self.next_recovery_id.checked_add(1)?;
        let stamp = RecoveryLoadStamp {
            recovery_id: self.next_recovery_id,
            unknown_dispatch: original.stamp,
            owner: snapshot.context(),
            document_id: self.document_id.clone(),
            kind: RecoveryKind::ConflictRefresh,
        };
        self.next_recovery_id = next;
        self.recovery_cell = Arc::new(Mutex::new(None));
        self.recovery_active = Some(stamp.clone());
        self.recovery_local_snapshot = Some(snapshot);
        Some(AdmittedRecoveryLoad {
            backend: Arc::clone(&self.backend), runtime: self.runtime.clone(),
            cell: Arc::clone(&self.recovery_cell), stamp,
        })
    }

    pub fn launch_recovery_load(attempt: AdmittedRecoveryLoad) {
        let AdmittedRecoveryLoad { backend, runtime, cell, stamp } = attempt;
        if let Some(runtime) = runtime {
            runtime.spawn(async move {
                let result = backend.load_document(&stamp.document_id).await;
                if let Ok(mut delivery) = cell.lock() { *delivery = Some(RecoveryDelivery { stamp, result }); }
            });
        }
    }

    pub fn drain_recovery(&mut self, gate: &mut DocumentSaveGate<'_>) -> Option<RecoveryOutcome> {
        let delivery = self.recovery_cell.lock().ok().and_then(|mut cell| cell.take())?;
        if self.recovery_active.as_ref() != Some(&delivery.stamp) {
            return Some(RecoveryOutcome::Stale);
        }
        self.recovery_active = None;
        if !self.accepts_gate(gate) || gate.current_context() != Some(delivery.stamp.owner)
            || self.recovery_snapshot.as_ref().map(|pending| pending.stamp) != Some(delivery.stamp.unknown_dispatch)
            || delivery.stamp.document_id != self.document_id
            || match delivery.stamp.kind {
                RecoveryKind::UnknownOutcome => !self.uncertain,
                RecoveryKind::ConflictRefresh => self.uncertain || !self.has_conflict(),
            }
        { return Some(RecoveryOutcome::Stale); }
        let response = match delivery.result {
            Ok(response) => response,
            Err(error) => {
                if delivery.stamp.kind == RecoveryKind::UnknownOutcome {
                    self.state = SaveState::Error(format!("Recovery load failed; submitted content retained: {error}"));
                }
                return Some(RecoveryOutcome::Failed(error));
            }
        };
        let server = match decode_recovery_document(
            response.document, &self.document_id, self.doc_version.max(gate.known_server_version()),
        ) {
            Ok(server) => server,
            Err(error) => {
                if delivery.stamp.kind == RecoveryKind::UnknownOutcome {
                    self.state = SaveState::Error(error.to_string());
                }
                return Some(RecoveryOutcome::Failed(error));
            }
        };
        if gate.observe_server_version(delivery.stamp.owner, server.doc_version).is_err() {
            return Some(RecoveryOutcome::Stale);
        }
        let local_content = match delivery.stamp.kind {
            RecoveryKind::UnknownOutcome => self.recovery_snapshot.as_ref()?.snapshot.content_json().clone(),
            RecoveryKind::ConflictRefresh => self.recovery_local_snapshot.take()?.content_json().clone(),
        };
        let server_version = server.doc_version;
        self.state = SaveState::Conflict { server: Box::new(server), local_content };
        self.choice_context = Some(delivery.stamp.owner);
        self.recovery_choice = delivery.stamp.kind == RecoveryKind::UnknownOutcome;
        self.queued = None;
        Some(RecoveryOutcome::Ready { dispatch_stamp: delivery.stamp.unknown_dispatch, server_version })
    }

    fn accepts_choice(&self, gate: &DocumentSaveGate<'_>) -> bool {
        self.accepts_gate(gate) && self.choice_context.is_some()
            && gate.current_context() == self.choice_context
    }

    pub fn recovery_choice_ready(&self, gate: &DocumentSaveGate<'_>) -> bool {
        self.uncertain && self.recovery_choice && self.has_conflict() && self.accepts_choice(gate)
    }

    pub fn request_keep_yours(&mut self, gate: &mut DocumentSaveGate<'_>) -> bool {
        if !self.accepts_choice(gate) { return false; }
        let SaveState::Conflict { server, local_content } = &self.state else { return false; };
        self.state = SaveState::ConfirmKeepYours { server: server.clone(), local_content: local_content.clone() };
        true
    }

    pub fn confirm_keep_yours(
        &mut self, snapshot: DocumentSaveSnapshot, gate: &mut DocumentSaveGate<'_>,
    ) -> Option<AdmittedSaveAttempt> {
        if !self.accepts_choice(gate) || !self.accepts_snapshot(&snapshot, gate)
            || gate.current_context() != Some(snapshot.context())
        { return None; }
        let SaveState::ConfirmKeepYours { server, .. } = &self.state else { return None; };
        let version = server.doc_version;
        gate.observe_server_version(snapshot.context(), version).ok()?;
        self.doc_version = version;
        let recovery_choice = self.recovery_choice;
        self.state = SaveState::Idle;
        self.uncertain = false;
        self.recovery_snapshot = None;
        self.queued = None;
        self.choice_context = None;
        self.recovery_choice = false;
        self.dirty = true;
        if recovery_choice { None } else { self.admit(snapshot, gate) }
    }

    pub fn cancel_keep_yours(&mut self, gate: &mut DocumentSaveGate<'_>) {
        if !self.accepts_choice(gate) { return; }
        if let SaveState::ConfirmKeepYours { server, local_content } = &self.state {
            self.state = SaveState::Conflict { server: server.clone(), local_content: local_content.clone() };
        }
    }

    pub fn keep_server(&mut self, gate: &mut DocumentSaveGate<'_>) -> Option<JsonValue> {
        if !self.accepts_choice(gate) { return None; }
        let SaveState::Conflict { server, .. } = &self.state else { return None; };
        let content = server.content_json.clone()?;
        let document = crate::rich_editor::document_model::doc_json::from_json_value(&content).ok()?;
        gate.accept_server_document(self.choice_context?, document, server.doc_version).ok()?;
        self.doc_version = server.doc_version;
        self.updated_at = server.updated_at.clone();
        self.dirty = false;
        self.state = SaveState::Idle;
        self.recovery_snapshot = None;
        self.uncertain = false;
        self.queued = None;
        self.choice_context = None;
        self.recovery_choice = false;
        Some(content)
    }

    pub fn dismiss_error(&mut self) {
        if matches!(self.state, SaveState::Error(_)) { self.state = SaveState::Idle; }
    }

    pub fn deliver_for_test(&self, result: Result<RichDocSaveResult, SaveError>) {
        if let Some(pending) = &self.active {
            if let Ok(mut cell) = self.cell.lock() { *cell = Some(SaveDelivery { stamp: pending.stamp, result }); }
        }
    }

    pub fn deliver_recovery_for_test(
        &self, result: Result<crate::backend::knowledge_documents::DocumentLoadResponse, SaveError>,
    ) {
        if let Some(stamp) = &self.recovery_active {
            if let Ok(mut cell) = self.recovery_cell.lock() {
                *cell = Some(RecoveryDelivery { stamp: stamp.clone(), result });
            }
        }
    }
}

fn same_binding(left: EditorSaveContext, right: EditorSaveContext) -> bool {
    left.document_incarnation == right.document_incarnation
        && left.context_generation == right.context_generation
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rich_editor::document_owner::DocumentEditOwner;
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct MockBackend {
        result: Mutex<Option<Result<RichDocSaveResult, SaveError>>>,
        recovery: Mutex<Option<Result<crate::backend::knowledge_documents::DocumentLoadResponse, SaveError>>>,
        calls: AtomicUsize,
    }

    impl MockBackend {
        fn new(result: Result<RichDocSaveResult, SaveError>) -> Self {
            Self { result: Mutex::new(Some(result)), recovery: Mutex::new(None), calls: AtomicUsize::new(0) }
        }
    }

    impl SaveBackend for MockBackend {
        fn save_document(&self, _id: &str, _content: JsonValue, _version: u64) -> SaveFuture {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let result = self.result.lock().unwrap().take().expect("stage the save result");
            Box::pin(async move { result })
        }

        fn load_document(&self, _document_id: &str) -> RecoveryLoadFuture {
            let result = self.recovery.lock().unwrap().take().expect("stage the recovery result");
            Box::pin(async move { result })
        }
    }

    fn content(text: &str) -> JsonValue {
        json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":text}]}]})
    }

    fn owner(content: &JsonValue) -> DocumentEditOwner {
        let document = crate::rich_editor::document_model::doc_json::from_json_value(content).unwrap();
        let owner = DocumentEditOwner::new(document);
        owner.bind_context("workspace-a", "DOC-1", "http://test", None, None).unwrap();
        owner.with_save_gate(|gate| {
            gate.observe_server_version(gate.current_context().unwrap(), 3).unwrap();
        });
        owner
    }

    fn edit(owner: &DocumentEditOwner, value: &JsonValue) {
        let expected = owner.current_context().unwrap();
        let replacement = crate::rich_editor::document_model::doc_json::from_json_value(value).unwrap();
        owner.edit(expected, |document, _history| {
            *document = replacement;
            Ok::<_, ()>(())
        }).unwrap();
    }

    fn server_doc(version: u64) -> RichDocLoad {
        RichDocLoad { rich_document_id: "DOC-1".to_owned(), doc_version: version,
            title: "T".to_owned(), content_json: Some(content("server")),
            updated_at: Some("2026-06-22T01:00:00Z".to_owned()) }
    }

    fn ok_result(version: u64) -> RichDocSaveResult {
        RichDocSaveResult { document: server_doc(version), backlinks_persisted: 0,
            backlinks_error: None, backlinks_skipped_reason: None,
            save_receipt_event_id: None, attribution: None }
    }

    fn raw_load(version: u64) -> crate::backend::knowledge_documents::DocumentLoadResponse {
        use crate::backend::knowledge_documents::{BlockTreeResponse, DocumentLoadResponse};
        DocumentLoadResponse {
            document: json!({"rich_document_id":"DOC-1","doc_version":version,"title":"T","content_json":content("server")}),
            tree: BlockTreeResponse { schema_version: "test".to_owned(), schema_matches: true,
                block_ids: vec![], blocks: json!([]) },
            code_nodes: json!([]),
        }
    }

    fn mgr(value: &JsonValue) -> (SaveManager, DocumentEditOwner) {
        let owner = owner(value);
        let mut manager = SaveManager::new(Arc::new(MockBackend::new(Ok(ok_result(4)))), None, "DOC-1", 3);
        assert!(owner.with_save_gate(|gate| manager.bind_owner(gate)));
        (manager, owner)
    }

    fn dispatch(manager: &mut SaveManager, owner: &DocumentEditOwner) -> SaveDispatchStamp {
        let snapshot = owner.save_snapshot().unwrap();
        let attempt = owner.with_save_gate(|gate| manager.request_save(snapshot, gate)).unwrap();
        let stamp = attempt.stamp;
        SaveManager::launch_admitted(attempt);
        stamp
    }

    fn drain(manager: &mut SaveManager, owner: &DocumentEditOwner) -> Option<SaveOutcome> {
        let transition = owner.with_save_gate(|gate| manager.drain(gate));
        if let Some(attempt) = transition.next_attempt { SaveManager::launch_admitted(attempt); }
        transition.outcome
    }

    fn conflict(manager: &mut SaveManager, owner: &DocumentEditOwner) {
        dispatch(manager, owner);
        manager.deliver_for_test(Err(SaveError::VersionConflict(Box::new(server_doc(5)))));
        assert!(matches!(drain(manager, owner), Some(SaveOutcome::Conflict { .. })));
    }

    fn uncertain(manager: &mut SaveManager, owner: &DocumentEditOwner) {
        dispatch(manager, owner);
        manager.deliver_for_test(Err(SaveError::Network("response was lost".to_owned())));
        assert!(matches!(drain(manager, owner), Some(SaveOutcome::Failed { uncertain: true, .. })));
    }

    fn recover(manager: &mut SaveManager, owner: &DocumentEditOwner) {
        let attempt = owner.with_save_gate(|gate| manager.request_recovery_load(gate)).unwrap();
        SaveManager::launch_recovery_load(attempt);
        manager.deliver_recovery_for_test(Ok(raw_load(5)));
        assert!(matches!(owner.with_save_gate(|gate| manager.drain_recovery(gate)),
            Some(RecoveryOutcome::Ready { server_version: 5, .. })));
    }

    #[test]
    fn successful_save_bumps_version_and_clears_dirty() {
        let value = content("mine");
        let (mut manager, owner) = mgr(&value);
        manager.mark_dirty();
        let stamp = dispatch(&mut manager, &owner);
        assert!(manager.is_saving());
        manager.deliver_for_test(Ok(ok_result(4)));
        match drain(&mut manager, &owner).unwrap() {
            SaveOutcome::Saved { doc_version, backlinks_persisted, backlinks_warning, saved_content,
                saved_snapshot, workspace_id, save_receipt_event_id, attribution,
                dispatch_stamp, current_edit_saved } => {
                assert_eq!(doc_version, 4);
                assert_eq!(backlinks_persisted, 0);
                assert_eq!(backlinks_warning, None);
                assert_eq!(saved_content, value);
                assert_eq!(saved_snapshot.content_json(), &value);
                assert_eq!(workspace_id, "workspace-a");
                assert_eq!(save_receipt_event_id, None);
                assert_eq!(attribution, None);
                assert_eq!(dispatch_stamp, stamp);
                assert!(current_edit_saved);
            }
            other => panic!("expected Saved, got {other:?}"),
        }
        assert_eq!(manager.doc_version, 4);
        assert!(!manager.dirty);
        assert_eq!(manager.state, SaveState::Idle);
        assert!(!owner.with_save_gate(|gate| gate.is_dirty()));
    }

    #[test]
    fn save_completion_keeps_workspace_captured_at_dispatch() {
        let (mut manager, owner) = mgr(&content("mine"));
        dispatch(&mut manager, &owner);
        manager.set_workspace_id("workspace-b");
        manager.deliver_for_test(Ok(ok_result(4)));
        assert!(matches!(drain(&mut manager, &owner),
            Some(SaveOutcome::Saved { workspace_id, .. }) if workspace_id == "workspace-a"));
    }

    #[test]
    fn conflict_409_sets_conflict_state_with_both_versions() {
        let local = content("mine");
        let (mut manager, owner) = mgr(&local);
        conflict(&mut manager, &owner);
        match &manager.state {
            SaveState::Conflict { server, local_content } => {
                assert_eq!(server.doc_version, 5);
                assert_eq!(local_content, &local);
            }
            other => panic!("expected Conflict, got {other:?}"),
        }
        assert!(manager.has_conflict());
    }

    #[test]
    fn keep_yours_requires_confirmation_before_resave() {
        let (mut manager, owner) = mgr(&content("mine"));
        conflict(&mut manager, &owner);
        assert!(owner.with_save_gate(|gate| manager.request_keep_yours(gate)));
        assert!(matches!(manager.state, SaveState::ConfirmKeepYours { .. }));
        assert!(!manager.is_saving());
        let snapshot = owner.save_snapshot().unwrap();
        let attempt = owner.with_save_gate(|gate| manager.confirm_keep_yours(snapshot, gate)).unwrap();
        assert_eq!(attempt.expected_version, 5);
        SaveManager::launch_admitted(attempt);
        assert_eq!(manager.doc_version, 5);
        assert!(manager.is_saving());
    }

    #[test]
    fn cancel_keep_yours_returns_to_conflict() {
        let (mut manager, owner) = mgr(&content("mine"));
        conflict(&mut manager, &owner);
        owner.with_save_gate(|gate| { manager.request_keep_yours(gate); manager.cancel_keep_yours(gate); });
        assert!(matches!(manager.state, SaveState::Conflict { .. }));
    }

    #[test]
    fn keep_server_reloads_server_content_and_clears_conflict() {
        let (mut manager, owner) = mgr(&content("mine"));
        manager.mark_dirty();
        conflict(&mut manager, &owner);
        let server_content = owner.with_save_gate(|gate| manager.keep_server(gate)).unwrap();
        assert_eq!(server_content["content"][0]["content"][0]["text"], "server");
        assert_eq!(owner.save_snapshot().unwrap().content_json(), &server_content);
        assert_eq!(manager.doc_version, 5);
        assert!(!manager.dirty);
        assert_eq!(manager.state, SaveState::Idle);
    }

    #[test]
    fn second_save_blocked_while_in_flight() {
        let (mut manager, owner) = mgr(&content("mine"));
        dispatch(&mut manager, &owner);
        let snapshot = owner.save_snapshot().unwrap();
        assert!(owner.with_save_gate(|gate| manager.request_save(snapshot, gate)).is_none());
        assert!(manager.is_saving());
        assert!(manager.queued.is_some());
    }

    #[test]
    fn rapid_save_uses_latest_snapshot_and_confirmed_version_without_clearing_newer_edits() {
        let (mut manager, owner) = mgr(&content("first"));
        let first_stamp = dispatch(&mut manager, &owner);
        edit(&owner, &content("second"));
        let snapshot = owner.save_snapshot().unwrap();
        assert!(owner.with_save_gate(|gate| manager.request_save(snapshot, gate)).is_none());
        edit(&owner, &content("third"));
        let latest = owner.save_snapshot().unwrap();
        assert!(owner.with_save_gate(|gate| manager.request_save(latest, gate)).is_none());
        manager.deliver_for_test(Ok(ok_result(4)));
        let transition = owner.with_save_gate(|gate| manager.drain(gate));
        assert!(matches!(transition.outcome, Some(SaveOutcome::Saved { current_edit_saved: false, .. })));
        let next = transition.next_attempt.unwrap();
        assert_eq!(next.expected_version, 4);
        assert_eq!(next.content_json, content("third"));
        assert!(next.stamp.save_id > first_stamp.save_id);
        assert!(manager.dirty);
        assert!(owner.with_save_gate(|gate| gate.is_dirty()));
        SaveManager::launch_admitted(next);
        manager.deliver_for_test(Ok(ok_result(5)));
        assert!(matches!(drain(&mut manager, &owner), Some(SaveOutcome::Saved { current_edit_saved: true, .. })));
        assert!(!manager.dirty);
    }

    #[test]
    fn mismatched_callback_does_not_consume_active_attempt() {
        let (mut manager, owner) = mgr(&content("mine"));
        let stamp = dispatch(&mut manager, &owner);
        let wrong = SaveDispatchStamp { save_id: stamp.save_id + 1, ..stamp };
        *manager.cell.lock().unwrap() = Some(SaveDelivery { stamp: wrong, result: Ok(ok_result(4)) });
        assert!(matches!(drain(&mut manager, &owner), Some(SaveOutcome::Stale { .. })));
        assert_eq!(manager.active.as_ref().unwrap().stamp, stamp);
        assert_eq!(manager.doc_version, 3);
    }

    #[test]
    fn reopened_or_rebound_context_cannot_accept_old_completion() {
        let (mut manager, owner) = mgr(&content("mine"));
        dispatch(&mut manager, &owner);
        owner.bind_context("workspace-a", "DOC-1", "http://replacement", None, None).unwrap();
        manager.deliver_for_test(Ok(ok_result(4)));
        assert!(matches!(drain(&mut manager, &owner), Some(SaveOutcome::Stale { .. })));
        assert_eq!(manager.doc_version, 3);
        assert!(!manager.is_saving());
        assert!(manager.recovery_snapshot.is_some());
        assert!(!owner.with_save_gate(|gate| manager.bind_owner(gate)));
    }

    #[test]
    fn unknown_save_and_dismiss_preserve_snapshot_without_automatic_resend() {
        let (mut manager, owner) = mgr(&content("mine"));
        uncertain(&mut manager, &owner);
        manager.dismiss_error();
        assert!(!manager.is_saving());
        assert!(manager.requires_recovery());
        let snapshot = owner.save_snapshot().unwrap();
        assert!(owner.with_save_gate(|gate| manager.request_save(snapshot, gate)).is_none());
        assert_eq!(manager.recovery_snapshot.as_ref().unwrap().snapshot.content_json(), &content("mine"));
        assert_eq!(owner.save_snapshot().unwrap().content_json(), &content("mine"));
    }

    #[test]
    fn recovery_get_and_keep_yours_never_resend_old_queue() {
        let (mut manager, owner) = mgr(&content("first"));
        dispatch(&mut manager, &owner);
        edit(&owner, &content("second"));
        let snapshot = owner.save_snapshot().unwrap();
        owner.with_save_gate(|gate| { manager.request_save(snapshot, gate); });
        manager.deliver_for_test(Err(SaveError::Server(200, "body parse failed".to_owned())));
        drain(&mut manager, &owner);
        recover(&mut manager, &owner);
        assert!(manager.requires_recovery(), "GET does not prove the old save outcome");
        assert!(manager.dirty);
        assert!(owner.with_save_gate(|gate| gate.is_dirty()));
        assert!(manager.queued.is_none());
        owner.with_save_gate(|gate| { manager.request_keep_yours(gate); });
        let snapshot = owner.save_snapshot().unwrap();
        assert!(owner.with_save_gate(|gate| manager.confirm_keep_yours(snapshot, gate)).is_none());
        assert!(!manager.is_saving());
        assert!(manager.dirty);
        assert_eq!(manager.doc_version, 5);
        let snapshot = owner.save_snapshot().unwrap();
        let attempt = owner.with_save_gate(|gate| manager.request_save(snapshot, gate)).unwrap();
        assert_eq!(attempt.expected_version, 5);
        assert_eq!(attempt.content_json, content("second"));
        assert_eq!(attempt.stamp.save_id, 2);
        SaveManager::launch_admitted(attempt);
    }

    #[test]
    fn later_edit_rejects_recovery_delivery_without_losing_unknown_snapshot() {
        let (mut manager, owner) = mgr(&content("first"));
        uncertain(&mut manager, &owner);
        let attempt = owner.with_save_gate(|gate| manager.request_recovery_load(gate)).unwrap();
        SaveManager::launch_recovery_load(attempt);
        edit(&owner, &content("later"));
        manager.deliver_recovery_for_test(Ok(raw_load(5)));
        assert!(matches!(owner.with_save_gate(|gate| manager.drain_recovery(gate)), Some(RecoveryOutcome::Stale)));
        assert!(manager.requires_recovery());
        assert_eq!(manager.doc_version, 3);
        assert_eq!(owner.save_snapshot().unwrap().content_json(), &content("later"));
        assert!(owner.with_save_gate(|gate| manager.request_recovery_load(gate)).is_some());
    }

    #[test]
    fn recovery_rejects_missing_wrong_older_or_malformed_authority_fields() {
        let valid = raw_load(5).document;
        let mut missing_version = valid.clone();
        missing_version.as_object_mut().unwrap().remove("doc_version");
        let mut wrong_id = valid.clone(); wrong_id["rich_document_id"] = json!("OTHER");
        let mut negative = valid.clone(); negative["doc_version"] = json!(-1);
        let mut missing_content = valid.clone(); missing_content.as_object_mut().unwrap().remove("content_json");
        let mut malformed = valid.clone(); malformed["content_json"] = json!({"type":"invalid-node"});
        let mut older = valid.clone(); older["doc_version"] = json!(2);
        for value in [missing_version, wrong_id, negative, missing_content, malformed, older] {
            assert!(decode_recovery_document(value, "DOC-1", 3).is_err());
        }
        assert_eq!(decode_recovery_document(valid, "DOC-1", 3).unwrap().doc_version, 5);
    }

    #[test]
    fn projection_failure_does_not_undo_canonical_save() {
        let (mut manager, owner) = mgr(&content("mine"));
        dispatch(&mut manager, &owner);
        let mut result = ok_result(4); result.backlinks_error = Some("index unavailable".to_owned());
        manager.deliver_for_test(Ok(result));
        assert!(matches!(drain(&mut manager, &owner),
            Some(SaveOutcome::Saved { backlinks_warning: Some(_), current_edit_saved: true, .. })));
        assert_eq!(manager.doc_version, 4);
        assert!(!manager.requires_recovery());
    }

    #[test]
    fn edit_after_conflict_requires_explicit_refresh_before_confirmed_keep_yours() {
        let (mut manager, owner) = mgr(&content("first"));
        conflict(&mut manager, &owner);
        edit(&owner, &content("newer local edit"));
        assert!(!owner.with_save_gate(|gate| manager.request_keep_yours(gate)));
        assert!(owner.with_save_gate(|gate| manager.keep_server(gate)).is_none());
        assert!(owner.with_save_gate(|gate| manager.conflict_refresh_required(gate)));

        let fresh = owner.save_snapshot().unwrap();
        let attempt = owner.with_save_gate(|gate| manager.request_conflict_refresh(fresh, gate)).unwrap();
        SaveManager::launch_recovery_load(attempt);
        manager.deliver_recovery_for_test(Err(SaveError::Network("reload unavailable".to_owned())));
        assert!(matches!(owner.with_save_gate(|gate| manager.drain_recovery(gate)), Some(RecoveryOutcome::Failed(_))));
        assert!(manager.has_conflict());
        let fresh = owner.save_snapshot().unwrap();
        assert!(owner.with_save_gate(|gate| manager.request_save(fresh, gate)).is_none());

        let fresh = owner.save_snapshot().unwrap();
        let attempt = owner.with_save_gate(|gate| manager.request_conflict_refresh(fresh, gate)).unwrap();
        SaveManager::launch_recovery_load(attempt);
        manager.deliver_recovery_for_test(Ok(raw_load(5)));
        assert!(matches!(owner.with_save_gate(|gate| manager.drain_recovery(gate)), Some(RecoveryOutcome::Ready { .. })));
        match &manager.state {
            SaveState::Conflict { local_content, .. } => assert_eq!(local_content, &content("newer local edit")),
            _ => panic!("refresh must return to the explicit conflict choices"),
        }
        assert!(!manager.is_saving());
        assert!(owner.with_save_gate(|gate| manager.request_keep_yours(gate)));
        assert!(!manager.is_saving(), "secondary confirmation still precedes the overwrite");
        let fresh = owner.save_snapshot().unwrap();
        let attempt = owner.with_save_gate(|gate| manager.confirm_keep_yours(fresh, gate)).unwrap();
        assert_eq!(attempt.expected_version, 5);
        assert_eq!(attempt.content_json, content("newer local edit"));
        SaveManager::launch_admitted(attempt);
    }
}
