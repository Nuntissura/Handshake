//! WP-KERNEL-012 MT-037 (E6 — backend reuse wiring): the CONSOLIDATED typed native Rust client for the
//! EXISTING handshake_core `/knowledge/documents/*` HTTP surface (WP-KERNEL-009 RichDocumentCore,
//! `api::knowledge_documents`). This is FRONTEND WIRING ONLY — it binds the routes the backend already
//! serves; it does NOT change the backend (a backend gap is a typed blocker, never a backend edit).
//!
//! ## Why this module exists (and the SINGLE load/save wire path)
//!
//! Prior MTs bound TWO knowledge-document routes through [`crate::backend_client::RichDocClient`]:
//!   * `GET  /knowledge/documents/{id}`        — the MT-029 find/replace load primitive (returns the
//!     narrow [`crate::backend_client::RichDocBody`]).
//!   * `PUT  /knowledge/documents/{id}/save`   — the MT-020/029 optimistic-concurrency save (409 ->
//!     [`crate::backend_client::DocSaveOutcome::Conflict`]).
//!
//! This MT is the CONSOLIDATED, single owner of the `/knowledge/documents/*` load + save wire path.
//! There is NO second forked load/save: [`KnowledgeDocumentsClient::load_document`] and
//! [`KnowledgeDocumentsClient::save_document`] are the ONE wire implementation, and
//! `RichDocClient::load_document` / `RichDocClient::save_document` now DELEGATE here (mapping this
//! client's [`DocumentLoadResponse`] -> `RichDocBody` and [`KnowledgeDocumentsError::SaveConflict`] ->
//! `DocSaveOutcome::Conflict`), so the find/replace pipeline and the editor share ONE save path with
//! ONE conflict semantic (the REUSE-NOT-DUPLICATE gate). The 409 -> [`KnowledgeDocumentsError::
//! SaveConflict`] mapping IS the MT-020 conflict pattern, not a divergent re-implementation.
//!
//! This module also REUSES the canonical identity-header constants and base URL `backend_client.rs`
//! owns ([`crate::backend_client::HSK_HEADER_ACTOR_ID`] etc., [`crate::backend_client::BACKEND_BASE_URL`],
//! [`crate::backend_client::DOC_ACTOR_ID`] / [`crate::backend_client::DOC_ACTOR_KIND`]) so the wire
//! identity is constructed in ONE place, and it shares the ONE process-wide
//! [`crate::backend_client::shared_http_client`] connection pool (production() does NOT mint a second
//! reqwest stack). On top of the shared transport it ADDS the 17 routes no prior MT bound (create /
//! import / draft GET-PUT-DELETE / blocks / history + version / projection / embeds + broken + repair /
//! backlinks list + rebuild / rename / move / batch) plus the richer typed response projections, the
//! [`HskDocumentHeaders`] struct, and the typed [`KnowledgeDocumentsError`] enum.
//!
//! ## Stateless adapter
//!
//! This module holds NO document state. It is a stateless HTTP adapter: every function takes typed
//! request arguments and returns a typed response or a typed [`KnowledgeDocumentsError`]. State (the
//! open document, the draft, the undo stack) lives in the editor layer that calls this.
//!
//! ## Verification provenance (SPEC-REALISM GATE)
//!
//! Every route + request/response shape + header + status-code mapping below was VERIFIED READ-ONLY
//! against the REAL running backend source `src/backend/handshake_core/src/api/knowledge_documents.rs`
//! (the `routes()` table + the per-handler bodies), NOT taken from the MT contract prose. The verified
//! facts that shape this client:
//!   * The three identity headers `x-hsk-actor-id` / `x-hsk-kernel-task-run-id` / `x-hsk-session-run-id`
//!     are REQUIRED (a missing one is `doc_context`'s hard `bad_request("<header> header is required")`
//!     -> HTTP 400). `x-hsk-actor-kind` is OPTIONAL but a MISSING kind is the least-privileged read-only
//!     actor server-side, so a WRITE with no kind 403s; an UNKNOWN kind is a 400. `x-hsk-correlation-id`
//!     is optional.
//!   * The save 409 body is `{"error":"conflict","detail":"..."}` — it does NOT carry a server version
//!     field, so [`KnowledgeDocumentsError::SaveConflict::server_version`] is parsed best-effort and is
//!     `None` against the current backend (the field is kept so a future backend can populate it without
//!     a client API break; the editor still gets a DISTINCT conflict variant, never a generic error).
//!   * `MoveBody` uses the serde double-Option idiom (`#[serde(default, deserialize_with = double_option)]`):
//!     an ABSENT field leaves that membership unchanged, an explicit `null` clears it, a string sets it
//!     (adversarial-v2 MT-157). This client mirrors it with `Option<Option<String>>` + a custom serializer
//!     ([`serialize_double_option`]) so absent omits the key, inner-`None` emits `null`, inner-`Some` emits
//!     the string.
//!   * The batch route caps at 100 operations (`BATCH_MAX_OPERATIONS`), rejects an empty list, and tags
//!     each op with `op` = `rename` | `move` | `set_authority_label` (snake_case). This client enforces
//!     the 1..=100 bound CLIENT-SIDE before sending so a too-large batch is a typed
//!     [`KnowledgeDocumentsError::BatchTooLarge`] (and an empty batch a [`KnowledgeDocumentsError::BatchEmpty`])
//!     BEFORE a wasted round-trip.
//!   * The history list clamps `limit` to 1..=200 (default 50) and `offset` to >= 0 server-side; this
//!     client clamps the same range CLIENT-SIDE so the request never relies on server clamping.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::backend_client::{
    shared_http_client, BACKEND_BASE_URL, DOC_ACTOR_ID, DOC_ACTOR_KIND, HSK_HEADER_ACTOR_ID,
    HSK_HEADER_ACTOR_KIND, HSK_HEADER_KERNEL_TASK_RUN_ID, HSK_HEADER_SESSION_RUN_ID,
};

/// The optional `x-hsk-correlation-id` header constant. The other four document identity headers
/// (`x-hsk-actor-id` / `x-hsk-kernel-task-run-id` / `x-hsk-session-run-id` / `x-hsk-actor-kind`) are
/// reused from [`crate::backend_client`] so the required names live in ONE place (the MT-020
/// missing-headers fix). The correlation-id constant is NOT yet declared in `backend_client.rs` (no
/// prior MT sent it), so it is defined here, matching the backend's `HSK_HEADER_CORRELATION_ID`
/// (verified against `api/knowledge_documents.rs`). Defining it locally avoids editing the large
/// shared `backend_client.rs` for one optional constant.
pub const HSK_HEADER_CORRELATION_ID: &str = "x-hsk-correlation-id";

/// Per-request timeout. A document call must not hang the caller's worker; on timeout the client
/// returns a [`KnowledgeDocumentsError::Transport`] the editor layer surfaces as a transient error.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// History pagination bounds, mirrored from the backend (`HISTORY_DEFAULT_LIMIT` / `HISTORY_MAX_LIMIT`)
/// so the client clamps the same range before sending (RISK: over-fetch / truncation confusion).
pub const HISTORY_DEFAULT_LIMIT: i64 = 50;
pub const HISTORY_MAX_LIMIT: i64 = 200;

/// Max operations in one batch request, mirrored from the backend (`BATCH_MAX_OPERATIONS`) so the
/// client rejects a too-large batch BEFORE sending (RISK: runtime 400 on large batches).
pub const BATCH_MAX_OPERATIONS: usize = 100;

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Identity headers.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The backend-navigation identity the editor presents on EVERY `/knowledge/documents/*` request. The
/// three `*_run_id` / `actor_id` fields are REQUIRED (a missing one is a hard backend 400); `actor_kind`
/// is optional on the wire but MUST be asserted for a WRITE (a missing kind defaults to least-privileged
/// read-only server-side and a write then 403s); `correlation_id` is optional.
///
/// Field names map 1:1 to the backend header constants:
///   * `actor_id`        -> `x-hsk-actor-id`            (required)
///   * `kernel_task_run_id` -> `x-hsk-kernel-task-run-id` (required)
///   * `session_run_id`  -> `x-hsk-session-run-id`      (required)
///   * `actor_kind`      -> `x-hsk-actor-kind`          (optional; omit for least-privileged read-only)
///   * `correlation_id`  -> `x-hsk-correlation-id`      (optional)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HskDocumentHeaders {
    /// `x-hsk-actor-id` (required).
    pub actor_id: String,
    /// `x-hsk-kernel-task-run-id` (required).
    pub kernel_task_run_id: String,
    /// `x-hsk-session-run-id` (required).
    pub session_run_id: String,
    /// `x-hsk-actor-kind` (optional; `None` = least-privileged read-only server-side).
    pub actor_kind: Option<String>,
    /// `x-hsk-correlation-id` (optional).
    pub correlation_id: Option<String>,
}

impl HskDocumentHeaders {
    /// Build the identity for an OPERATOR document edit (the `operator` kind the MT-158 permission
    /// matrix grants `Write`), reusing the canonical [`crate::backend_client::DOC_ACTOR_ID`] /
    /// [`crate::backend_client::DOC_ACTOR_KIND`] so the wire identity matches every other document
    /// transport in the app. The `document_id` is folded into the task run id so each document action
    /// is individually attributable (HBR-SWARM). This is the helper the editor session context uses to
    /// populate the headers (the contract's `build_headers(ctx)` role).
    pub fn for_operator(session_run_id: impl Into<String>, document_id: &str) -> Self {
        Self {
            actor_id: DOC_ACTOR_ID.to_string(),
            kernel_task_run_id: format!("native-editor-doc-{document_id}"),
            session_run_id: session_run_id.into(),
            actor_kind: Some(DOC_ACTOR_KIND.to_string()),
            correlation_id: None,
        }
    }

    /// Build a READ-ONLY identity (no `x-hsk-actor-kind`; the backend defaults a missing kind to the
    /// least-privileged read-only actor, which is exactly what a GET needs — least privilege by
    /// default). Used for the pure-read routes (load / draft GET / blocks / history / projection /
    /// embeds / backlinks list).
    pub fn for_read(session_run_id: impl Into<String>, document_id: &str) -> Self {
        Self {
            actor_id: DOC_ACTOR_ID.to_string(),
            kernel_task_run_id: format!("native-editor-doc-{document_id}"),
            session_run_id: session_run_id.into(),
            actor_kind: None,
            correlation_id: None,
        }
    }

    /// Attach all present identity headers to a request builder. The three required headers are always
    /// attached; `actor_kind` / `correlation_id` are attached only when present (an absent kind is the
    /// least-privileged read-only actor server-side — exactly the least-privilege default a read wants).
    fn apply(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        let mut builder = builder
            .header(HSK_HEADER_ACTOR_ID, &self.actor_id)
            .header(HSK_HEADER_KERNEL_TASK_RUN_ID, &self.kernel_task_run_id)
            .header(HSK_HEADER_SESSION_RUN_ID, &self.session_run_id);
        if let Some(kind) = &self.actor_kind {
            builder = builder.header(HSK_HEADER_ACTOR_KIND, kind);
        }
        if let Some(correlation) = &self.correlation_id {
            builder = builder.header(HSK_HEADER_CORRELATION_ID, correlation);
        }
        builder
    }
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Typed error.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The typed result of one `/knowledge/documents/*` call. HTTP status codes are mapped to DISTINCT
/// variants so the editor layer can react correctly — in particular [`Self::SaveConflict`] is its own
/// variant (NEVER folded into a generic error) so a 409 optimistic-concurrency conflict can drive a
/// conflict UI instead of a silent data loss, and [`Self::BadRequest`] / [`Self::Forbidden`] /
/// [`Self::NotFound`] are distinct so a missing-header 400 or a permission 403 is not mistaken for a
/// transport failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KnowledgeDocumentsError {
    /// HTTP 400 — a malformed request (e.g. a missing required identity header, an unknown actor-kind,
    /// a malformed block tree, or an out-of-range batch the SERVER rejected). Carries the backend
    /// `detail` when present.
    BadRequest(String),
    /// HTTP 403 — the actor-kind is not permitted the requested action (a missing `x-hsk-actor-kind`
    /// on a write defaults to the least-privileged read-only actor and 403s). Carries the `reason`.
    Forbidden(String),
    /// HTTP 404 — the document (or version / embed) does not exist.
    NotFound(String),
    /// HTTP 409 on `PUT /save` — the document changed since `expected_version` was read (optimistic-
    /// concurrency conflict). The DISTINCT variant the editor uses to show a conflict UI. `server_version`
    /// is the current server version when the backend supplies one in the 409 body; the current backend
    /// 409 body is `{"error":"conflict","detail":...}` (no version), so this is `None` today — kept so a
    /// future backend can populate it without an API break.
    SaveConflict { server_version: Option<i64> },
    /// HTTP 5xx — the backend failed internally. Carries the status + any body detail.
    Server(String),
    /// A non-success status that is none of the above mapped codes. Carries the status + body.
    UnexpectedStatus { status: u16, body: String },
    /// A transport failure (connect / timeout / TLS) — the request never reached a status.
    Transport(String),
    /// The response body could not be parsed into the expected typed shape.
    Parse(String),
    /// A CLIENT-SIDE guard rejected the request before sending: the batch exceeded
    /// [`BATCH_MAX_OPERATIONS`] (RISK: runtime 400 on large batches — caught before the round-trip).
    BatchTooLarge { len: usize, max: usize },
    /// A CLIENT-SIDE guard rejected the request before sending: the batch was empty (the backend
    /// rejects an empty batch with a 400 — caught before the round-trip).
    BatchEmpty,
}

impl std::fmt::Display for KnowledgeDocumentsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadRequest(d) => write!(f, "bad request (400): {d}"),
            Self::Forbidden(r) => write!(f, "forbidden (403): {r}"),
            Self::NotFound(d) => write!(f, "not found (404): {d}"),
            Self::SaveConflict { server_version } => write!(
                f,
                "save conflict (409): document changed since the expected version (server_version={server_version:?})"
            ),
            Self::Server(d) => write!(f, "server error (5xx): {d}"),
            Self::UnexpectedStatus { status, body } => {
                write!(f, "unexpected status {status}: {body}")
            }
            Self::Transport(e) => write!(f, "transport error: {e}"),
            Self::Parse(e) => write!(f, "parse error: {e}"),
            Self::BatchTooLarge { len, max } => {
                write!(f, "batch too large: {len} operations (max {max})")
            }
            Self::BatchEmpty => write!(f, "batch is empty (at least one operation is required)"),
        }
    }
}

impl std::error::Error for KnowledgeDocumentsError {}

/// A typed result alias for this client.
pub type DocResult<T> = Result<T, KnowledgeDocumentsError>;

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Request bodies (mirror the backend `*Body` structs — VERIFIED against api/knowledge_documents.rs).
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// `POST /knowledge/documents` body. `content_json` is a `serde_json::Value` (the ProseMirror doc node)
/// to avoid schema coupling; the optional fields are omitted from the wire when `None`.
#[derive(Debug, Clone, Serialize)]
pub struct CreateDocumentRequest {
    pub workspace_id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_json: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_ref: Option<String>,
}

/// `PUT /knowledge/documents/:id/save` body. `expected_version` is the optimistic-concurrency token
/// (a stale value -> 409 -> [`KnowledgeDocumentsError::SaveConflict`]). The optional CRDT/promotion
/// fields are omitted when `None`.
#[derive(Debug, Clone, Serialize)]
pub struct SaveDocumentRequest {
    pub expected_version: i64,
    pub content_json: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crdt_document_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crdt_snapshot_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub promotion_receipt_event_id: Option<String>,
}

/// `PUT /knowledge/documents/:id/draft` body. The base version + content hash gate the draft against
/// the current document server-side (a newer base or a hash mismatch -> 409).
#[derive(Debug, Clone, Serialize)]
pub struct UpsertDraftRequest {
    pub base_doc_version: i64,
    pub base_content_sha256: String,
    pub content_json: Value,
}

/// `POST /knowledge/documents/import` body. `format` is `markdown` | `plain_text` | `html`.
#[derive(Debug, Clone, Serialize)]
pub struct ImportDocumentRequest {
    pub workspace_id: String,
    pub title: String,
    pub format: String,
    pub snippet: String,
}

/// `POST /knowledge/documents/:id/move` body with the VERIFIED double-Option (absent != null) semantics
/// (adversarial-v2 MT-157): an ABSENT field leaves that membership unchanged, an explicit `null` clears
/// it, a string sets it. Serialized via [`serialize_double_option`]: outer-`None` omits the key,
/// inner-`None` emits `null`, inner-`Some(s)` emits the string.
#[derive(Debug, Clone, Default, Serialize)]
pub struct MoveDocumentRequest {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_double_option"
    )]
    pub project_ref: Option<Option<String>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_double_option"
    )]
    pub folder_ref: Option<Option<String>>,
}

/// `POST /knowledge/documents/:id/rename` body.
#[derive(Debug, Clone, Serialize)]
pub struct RenameDocumentRequest {
    pub title: String,
}

/// `POST /knowledge/documents/embeds/:embed_id/repair` body. `action` is the recorded intent
/// (`relink` | `reresolve` | `remove`); `broken_reason` marks the embed broken with a reason when
/// present, or repairs it back to ok when absent.
#[derive(Debug, Clone, Default, Serialize)]
pub struct RepairEmbedRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub broken_reason: Option<String>,
}

/// One operation in a [`BatchRequest`] — the VERIFIED serde tagged union (`#[serde(tag = "op",
/// rename_all = "snake_case")]`) the backend `BatchOperation` deserializes. `Move` carries the same
/// double-Option (absent != null) semantics as [`MoveDocumentRequest`].
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum BatchOperation {
    Rename {
        document_id: String,
        title: String,
    },
    Move {
        document_id: String,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            serialize_with = "serialize_double_option"
        )]
        project_ref: Option<Option<String>>,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            serialize_with = "serialize_double_option"
        )]
        folder_ref: Option<Option<String>>,
    },
    SetAuthorityLabel {
        document_id: String,
        authority_label: String,
    },
}

/// `POST /knowledge/documents/batch` body. Bounded to 1..=[`BATCH_MAX_OPERATIONS`] client-side
/// (see [`batch_documents`]).
#[derive(Debug, Clone, Serialize)]
pub struct BatchRequest {
    pub operations: Vec<BatchOperation>,
}

/// Serialize an `Option<Option<String>>` with the double-Option (absent != null) semantics: the OUTER
/// option is handled by `skip_serializing_if = "Option::is_none"` on the field (an absent field is
/// omitted entirely), so this serializer is only reached for `Some(inner)` and emits `null` for
/// `Some(None)` or the string for `Some(Some(s))`. This is the client mirror of the backend's
/// `double_option` deserializer (adversarial-v2 MT-157) — it guarantees an "absent" move never
/// silently clears a membership the caller meant to leave unchanged.
fn serialize_double_option<S>(value: &Option<Option<String>>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match value {
        // The outer-None arm is UNREACHABLE through the struct path: every field that uses this
        // serializer carries `skip_serializing_if = "Option::is_none"`, so serde never calls this fn
        // for an absent (outer-None) field — absence is realized by SKIPPING the key entirely, which
        // is the correct "leave unchanged" wire shape (absent != null). The arm exists only so the fn
        // is total; if it were ever reached it would emit `null` (serialize_none), matching the
        // explicit-clear shape rather than panicking.
        None => serializer.serialize_none(),
        Some(None) => serializer.serialize_none(),
        Some(Some(s)) => serializer.serialize_some(s),
    }
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Response projections (use serde_json::Value for nested document/embed/version bodies to avoid
// coupling the native crate to the handshake_core schema; the externally-meaningful top-level fields
// the editor consumes are typed).
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The typed block-tree view the backend returns under `tree` on load/blocks (the MT-146/147/148
/// shape). `blocks` is a `Value` to avoid coupling to the block schema.
#[derive(Debug, Clone, Deserialize)]
pub struct BlockTreeResponse {
    pub schema_version: String,
    pub schema_matches: bool,
    pub block_ids: Vec<String>,
    pub blocks: Value,
}

/// `GET /knowledge/documents/:id` response (`{document, tree, code_nodes}`). `document` is the full
/// `KnowledgeRichDocument` as a `Value` (avoids schema coupling); `tree` is the typed block tree;
/// `code_nodes` is the editor code-node list.
#[derive(Debug, Clone, Deserialize)]
pub struct DocumentLoadResponse {
    pub document: Value,
    pub tree: BlockTreeResponse,
    #[serde(default)]
    pub code_nodes: Value,
}

/// `GET /knowledge/documents/:id/draft` response. `draft` is the persisted recovery draft (a `Value`,
/// or `null` when there is no draft distinct from the current document).
#[derive(Debug, Clone, Deserialize)]
pub struct DocumentDraftResponse {
    pub rich_document_id: String,
    pub current_doc_version: i64,
    pub current_content_sha256: String,
    #[serde(default)]
    pub draft: Option<Value>,
}

/// `PUT /knowledge/documents/:id/save` 200 response. The post-commit index/embed/backlink counts are
/// best-effort server-side (a failure is RECORDED, never an error for a committed save — MT-149), so
/// the `*_error` fields surface a recorded post-commit failure without failing the save.
#[derive(Debug, Clone, Deserialize)]
pub struct SaveDocumentResponse {
    pub document: Value,
    #[serde(default)]
    pub save_receipt_event_id: Option<String>,
    #[serde(default)]
    pub receipt_error: Option<String>,
    #[serde(default)]
    pub backlinks_persisted: usize,
    #[serde(default)]
    pub backlinks_error: Option<String>,
    #[serde(default)]
    pub embeds_persisted: usize,
    #[serde(default)]
    pub embeds_error: Option<String>,
    #[serde(default)]
    pub knowledge_indexed: bool,
    #[serde(default)]
    pub knowledge_index_error: Option<String>,
}

/// `POST /knowledge/documents` create / `POST /knowledge/documents/import` / `POST .../rename`
/// response — a committed document write with its post-commit best-effort outcome.
#[derive(Debug, Clone, Deserialize)]
pub struct DocumentWriteResponse {
    pub document: Value,
    #[serde(default)]
    pub save_receipt_event_id: Option<String>,
    #[serde(default)]
    pub receipt_error: Option<String>,
    #[serde(default)]
    pub warnings: Value,
    #[serde(default)]
    pub embeds_persisted: usize,
    #[serde(default)]
    pub knowledge_indexed: bool,
}

/// `PUT /knowledge/documents/:id/draft` / `DELETE .../draft` response.
#[derive(Debug, Clone, Deserialize)]
pub struct DraftWriteResponse {
    pub rich_document_id: String,
    #[serde(default)]
    pub draft: Option<Value>,
    #[serde(default)]
    pub cleared: bool,
    #[serde(default)]
    pub draft_receipt_event_id: Option<String>,
    #[serde(default)]
    pub clear_receipt_event_id: Option<String>,
    #[serde(default)]
    pub receipt_error: Option<String>,
}

/// `GET /knowledge/documents/:id/history` response (paginated metadata; no content bodies — those load
/// lazily via [`load_history_version`]).
#[derive(Debug, Clone, Deserialize)]
pub struct HistoryListResponse {
    pub rich_document_id: String,
    pub current_version: i64,
    pub versions: Value,
    pub total_versions: i64,
    pub limit: i64,
    pub offset: i64,
    #[serde(default)]
    pub authority_label: Option<String>,
}

/// `GET /knowledge/documents/:id/history/:v` response (one revision incl. its content body).
#[derive(Debug, Clone, Deserialize)]
pub struct HistoryVersionResponse {
    pub rich_document_id: String,
    pub version: Value,
}

/// `GET /knowledge/documents/:id/projection?format=` response. `projection` is the rendered string in
/// the requested format (rendering never mutates authority).
#[derive(Debug, Clone, Deserialize)]
pub struct ProjectionResponse {
    pub rich_document_id: String,
    pub projection: Value,
}

/// `GET /knowledge/documents/:id/embeds` response.
#[derive(Debug, Clone, Deserialize)]
pub struct EmbedListResponse {
    pub rich_document_id: String,
    pub embeds: Value,
}

/// `GET /knowledge/documents/:id/embeds/broken` response.
#[derive(Debug, Clone, Deserialize)]
pub struct BrokenEmbedListResponse {
    pub rich_document_id: String,
    pub broken_embeds: Value,
    #[serde(default)]
    pub available_actions: Vec<String>,
}

/// `POST /knowledge/documents/embeds/:embed_id/repair` response.
#[derive(Debug, Clone, Deserialize)]
pub struct RepairEmbedResponse {
    pub embed: Value,
    #[serde(default)]
    pub save_receipt_event_id: Option<String>,
    #[serde(default)]
    pub receipt_error: Option<String>,
}

/// `GET /knowledge/documents/:id/backlinks` / `POST .../backlinks` (rebuild) response.
#[derive(Debug, Clone, Deserialize)]
pub struct BacklinksResponse {
    pub source_document_id: String,
    pub backlinks: Value,
    #[serde(default)]
    pub tags: Value,
}

/// `POST /knowledge/documents/:document_id/move` response.
#[derive(Debug, Clone, Deserialize)]
pub struct MoveDocumentResponse {
    pub document: Value,
    #[serde(default)]
    pub save_receipt_event_id: Option<String>,
    #[serde(default)]
    pub receipt_error: Option<String>,
}

/// `POST /knowledge/documents/batch` response (per-item results + counts).
#[derive(Debug, Clone, Deserialize)]
pub struct BatchResponse {
    pub results: Value,
    pub succeeded: usize,
    pub failed: usize,
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Client.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The stateless typed client for the `/knowledge/documents/*` surface. Holds ONLY a shared
/// [`reqwest::Client`] (cheaply cloneable; an `Arc` internally) and the base URL — NO document state.
/// The base URL is resolved from [`crate::backend_client::BACKEND_BASE_URL`] (config/environment via
/// the WP-011 backend client), NEVER hardcoded at a call site (GLOBAL-PORTABILITY-004).
#[derive(Clone)]
pub struct KnowledgeDocumentsClient {
    client: reqwest::Client,
    base_url: String,
}

impl Default for KnowledgeDocumentsClient {
    fn default() -> Self {
        Self::production()
    }
}

impl KnowledgeDocumentsClient {
    /// Construct against the production backend base URL (the same `BACKEND_BASE_URL` every other
    /// native client uses — config-resolved, not hardcoded here), sharing the ONE process-wide
    /// [`crate::backend_client::shared_http_client`] connection pool rather than minting a second
    /// reqwest stack (the REUSE-NOT-DUPLICATE pool concern). The find/replace `RichDocClient` delegates
    /// to this same client + pool, so the whole `/knowledge/documents/*` surface shares one transport.
    pub fn production() -> Self {
        Self::with_client(shared_http_client(), BACKEND_BASE_URL)
    }

    /// Construct against an explicit base URL on a FRESH client (used by tests to point at a mock
    /// server with an isolated pool; the production path uses [`Self::production`], which shares the
    /// process-wide pool via [`Self::with_client`]). The base URL is the authority for the host — a
    /// function never hardcodes one.
    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
        }
    }

    /// Reuse an already-constructed [`reqwest::Client`] (e.g. the one the WP-011 [`crate::backend_client`]
    /// owns) so the app shares ONE connection pool rather than minting a second HTTP stack.
    pub fn with_client(client: reqwest::Client, base_url: impl Into<String>) -> Self {
        Self {
            client,
            base_url: base_url.into(),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    // ── Route 1: POST /knowledge/documents — create a RichDocument. ─────────────────────────────
    /// Calls `POST /knowledge/documents` (create_document).
    pub async fn create_document(
        &self,
        headers: &HskDocumentHeaders,
        body: &CreateDocumentRequest,
    ) -> DocResult<DocumentWriteResponse> {
        let builder = self.client.post(self.url("/knowledge/documents")).json(body);
        self.send_json(headers.apply(builder)).await
    }

    // ── Route 2: POST /knowledge/documents/import — import a snippet into a new document. ────────
    /// Calls `POST /knowledge/documents/import` (import_document).
    pub async fn import_document(
        &self,
        headers: &HskDocumentHeaders,
        body: &ImportDocumentRequest,
    ) -> DocResult<DocumentWriteResponse> {
        let builder = self
            .client
            .post(self.url("/knowledge/documents/import"))
            .json(body);
        self.send_json(headers.apply(builder)).await
    }

    // ── Route 3: GET /knowledge/documents/:id — load document + block tree + code nodes. ────────
    /// Calls `GET /knowledge/documents/:document_id` (load_document).
    pub async fn load_document(
        &self,
        headers: &HskDocumentHeaders,
        document_id: &str,
    ) -> DocResult<DocumentLoadResponse> {
        let builder = self
            .client
            .get(self.url(&format!("/knowledge/documents/{document_id}")));
        self.send_json(headers.apply(builder)).await
    }

    // ── Route 4: GET /knowledge/documents/:id/draft — load persisted recovery draft. ────────────
    /// Calls `GET /knowledge/documents/:document_id/draft` (load_document_draft).
    pub async fn load_document_draft(
        &self,
        headers: &HskDocumentHeaders,
        document_id: &str,
    ) -> DocResult<DocumentDraftResponse> {
        let builder = self
            .client
            .get(self.url(&format!("/knowledge/documents/{document_id}/draft")));
        self.send_json(headers.apply(builder)).await
    }

    // ── Route 5: PUT /knowledge/documents/:id/draft — persist unsaved editor content. ───────────
    /// Calls `PUT /knowledge/documents/:document_id/draft` (upsert_document_draft).
    pub async fn upsert_document_draft(
        &self,
        headers: &HskDocumentHeaders,
        document_id: &str,
        body: &UpsertDraftRequest,
    ) -> DocResult<DraftWriteResponse> {
        let builder = self
            .client
            .put(self.url(&format!("/knowledge/documents/{document_id}/draft")))
            .json(body);
        self.send_json(headers.apply(builder)).await
    }

    // ── Route 6: DELETE /knowledge/documents/:id/draft — discard a persisted draft. ─────────────
    /// Calls `DELETE /knowledge/documents/:document_id/draft` (clear_document_draft).
    pub async fn clear_document_draft(
        &self,
        headers: &HskDocumentHeaders,
        document_id: &str,
    ) -> DocResult<DraftWriteResponse> {
        let builder = self
            .client
            .delete(self.url(&format!("/knowledge/documents/{document_id}/draft")));
        self.send_json(headers.apply(builder)).await
    }

    // ── Route 7: PUT /knowledge/documents/:id/save — optimistic-concurrency save. ───────────────
    /// Calls `PUT /knowledge/documents/:document_id/save` (save_document). A stale `expected_version`
    /// returns 409, surfaced as the DISTINCT [`KnowledgeDocumentsError::SaveConflict`] (never a generic
    /// error) so the editor can show a conflict UI (RISK: 409-as-generic-error data loss). This reuses
    /// the MT-020 conflict pattern; it does NOT fork a second divergent save.
    pub async fn save_document(
        &self,
        headers: &HskDocumentHeaders,
        document_id: &str,
        body: &SaveDocumentRequest,
    ) -> DocResult<SaveDocumentResponse> {
        let builder = self
            .client
            .put(self.url(&format!("/knowledge/documents/{document_id}/save")))
            .json(body);
        self.send_json(headers.apply(builder)).await
    }

    // ── Route 8: GET /knowledge/documents/:id/blocks — typed block tree only. ───────────────────
    /// Calls `GET /knowledge/documents/:document_id/blocks` (load_blocks).
    pub async fn load_blocks(
        &self,
        headers: &HskDocumentHeaders,
        document_id: &str,
    ) -> DocResult<BlockTreeResponse> {
        let builder = self
            .client
            .get(self.url(&format!("/knowledge/documents/{document_id}/blocks")));
        self.send_json(headers.apply(builder)).await
    }

    // ── Route 9: GET /knowledge/documents/:id/history — paginated revision metadata. ────────────
    /// Calls `GET /knowledge/documents/:document_id/history?limit=&offset=` (load_history). `limit` is
    /// clamped CLIENT-SIDE to 1..=[`HISTORY_MAX_LIMIT`] and `offset` to >= 0 so the request never
    /// relies on server clamping (RISK: over-fetch / truncation confusion).
    pub async fn load_history(
        &self,
        headers: &HskDocumentHeaders,
        document_id: &str,
        limit: i64,
        offset: i64,
    ) -> DocResult<HistoryListResponse> {
        let limit = limit.clamp(1, HISTORY_MAX_LIMIT);
        let offset = offset.max(0);
        let builder = self
            .client
            .get(self.url(&format!("/knowledge/documents/{document_id}/history")))
            .query(&[("limit", limit), ("offset", offset)]);
        self.send_json(headers.apply(builder)).await
    }

    // ── Route 10: GET /knowledge/documents/:id/history/:v — one revision incl. content. ─────────
    /// Calls `GET /knowledge/documents/:document_id/history/:doc_version` (load_history_version).
    pub async fn load_history_version(
        &self,
        headers: &HskDocumentHeaders,
        document_id: &str,
        doc_version: i64,
    ) -> DocResult<HistoryVersionResponse> {
        let builder = self.client.get(self.url(&format!(
            "/knowledge/documents/{document_id}/history/{doc_version}"
        )));
        self.send_json(headers.apply(builder)).await
    }

    // ── Route 11: GET /knowledge/documents/:id/projection?format= — render a projection. ────────
    /// Calls `GET /knowledge/documents/:document_id/projection?format=` (export_projection). `format`
    /// is one of `markdown` | `html` | `plain_text` | `wiki_loom` | `context_bundle`.
    pub async fn export_projection(
        &self,
        headers: &HskDocumentHeaders,
        document_id: &str,
        format: &str,
    ) -> DocResult<ProjectionResponse> {
        let builder = self
            .client
            .get(self.url(&format!("/knowledge/documents/{document_id}/projection")))
            .query(&[("format", format)]);
        self.send_json(headers.apply(builder)).await
    }

    // ── Route 12: GET /knowledge/documents/:id/embeds — typed embed refs. ───────────────────────
    /// Calls `GET /knowledge/documents/:document_id/embeds` (list_embeds).
    pub async fn list_embeds(
        &self,
        headers: &HskDocumentHeaders,
        document_id: &str,
    ) -> DocResult<EmbedListResponse> {
        let builder = self
            .client
            .get(self.url(&format!("/knowledge/documents/{document_id}/embeds")));
        self.send_json(headers.apply(builder)).await
    }

    // ── Route 13: GET /knowledge/documents/:id/embeds/broken — broken-embed repair queue. ───────
    /// Calls `GET /knowledge/documents/:document_id/embeds/broken` (list_broken_embeds).
    pub async fn list_broken_embeds(
        &self,
        headers: &HskDocumentHeaders,
        document_id: &str,
    ) -> DocResult<BrokenEmbedListResponse> {
        let builder = self.client.get(self.url(&format!(
            "/knowledge/documents/{document_id}/embeds/broken"
        )));
        self.send_json(headers.apply(builder)).await
    }

    // ── Route 14: POST /knowledge/documents/embeds/:embed_id/repair — mark/repair an embed. ─────
    /// Calls `POST /knowledge/documents/embeds/:embed_id/repair` (repair_embed).
    pub async fn repair_embed(
        &self,
        headers: &HskDocumentHeaders,
        embed_id: &str,
        body: &RepairEmbedRequest,
    ) -> DocResult<RepairEmbedResponse> {
        let builder = self
            .client
            .post(self.url(&format!("/knowledge/documents/embeds/{embed_id}/repair")))
            .json(body);
        self.send_json(headers.apply(builder)).await
    }

    // ── Route 15: GET /knowledge/documents/:id/backlinks — forward backlinks. ───────────────────
    /// Calls `GET /knowledge/documents/:document_id/backlinks` (list_backlinks).
    pub async fn list_backlinks(
        &self,
        headers: &HskDocumentHeaders,
        document_id: &str,
    ) -> DocResult<BacklinksResponse> {
        let builder = self
            .client
            .get(self.url(&format!("/knowledge/documents/{document_id}/backlinks")));
        self.send_json(headers.apply(builder)).await
    }

    // ── Route 16: POST /knowledge/documents/:id/backlinks — rebuild backlinks. ──────────────────
    /// Calls `POST /knowledge/documents/:document_id/backlinks` (rebuild_backlinks). Requires the
    /// `Index` permission server-side (a read-only kind 403s).
    pub async fn rebuild_backlinks(
        &self,
        headers: &HskDocumentHeaders,
        document_id: &str,
    ) -> DocResult<BacklinksResponse> {
        let builder = self
            .client
            .post(self.url(&format!("/knowledge/documents/{document_id}/backlinks")));
        self.send_json(headers.apply(builder)).await
    }

    // ── Route 17: POST /knowledge/documents/:id/rename — batch-safe rename. ─────────────────────
    /// Calls `POST /knowledge/documents/:document_id/rename` (rename_document).
    pub async fn rename_document(
        &self,
        headers: &HskDocumentHeaders,
        document_id: &str,
        body: &RenameDocumentRequest,
    ) -> DocResult<DocumentWriteResponse> {
        let builder = self
            .client
            .post(self.url(&format!("/knowledge/documents/{document_id}/rename")))
            .json(body);
        self.send_json(headers.apply(builder)).await
    }

    // ── Route 18: POST /knowledge/documents/:id/move — batch-safe move (double-option). ─────────
    /// Calls `POST /knowledge/documents/:document_id/move` (move_document). The double-Option body
    /// preserves absent != null: an absent ref leaves the membership unchanged, an explicit `null`
    /// clears it, a string sets it (RISK: a wrong serialization silently clears a membership).
    pub async fn move_document(
        &self,
        headers: &HskDocumentHeaders,
        document_id: &str,
        body: &MoveDocumentRequest,
    ) -> DocResult<MoveDocumentResponse> {
        let builder = self
            .client
            .post(self.url(&format!("/knowledge/documents/{document_id}/move")))
            .json(body);
        self.send_json(headers.apply(builder)).await
    }

    // ── Route 19: POST /knowledge/documents/batch — bounded batch ops. ──────────────────────────
    /// Calls `POST /knowledge/documents/batch` (batch_documents). The operation count is guarded
    /// CLIENT-SIDE to 1..=[`BATCH_MAX_OPERATIONS`] BEFORE sending: an empty list is
    /// [`KnowledgeDocumentsError::BatchEmpty`] and an over-large list is
    /// [`KnowledgeDocumentsError::BatchTooLarge`], so a runtime backend 400 on a bad batch size is
    /// caught before the round-trip (RISK: runtime errors on large batches).
    pub async fn batch_documents(
        &self,
        headers: &HskDocumentHeaders,
        body: &BatchRequest,
    ) -> DocResult<BatchResponse> {
        if body.operations.is_empty() {
            return Err(KnowledgeDocumentsError::BatchEmpty);
        }
        if body.operations.len() > BATCH_MAX_OPERATIONS {
            return Err(KnowledgeDocumentsError::BatchTooLarge {
                len: body.operations.len(),
                max: BATCH_MAX_OPERATIONS,
            });
        }
        let builder = self
            .client
            .post(self.url("/knowledge/documents/batch"))
            .json(body);
        self.send_json(headers.apply(builder)).await
    }

    // ── Shared send + status-mapping + parse path. ──────────────────────────────────────────────
    /// Send a built request (timeout attached), map the HTTP status to a typed
    /// [`KnowledgeDocumentsError`], and deserialize a success body into `T`. A 409 maps to
    /// [`KnowledgeDocumentsError::SaveConflict`] (parsing `server_version` from the body best-effort);
    /// 400/403/404 map to their distinct variants carrying the backend `detail`/`reason`; 5xx maps to
    /// [`KnowledgeDocumentsError::Server`]; any other non-success status to
    /// [`KnowledgeDocumentsError::UnexpectedStatus`]. The body is parsed exactly once.
    async fn send_json<T: serde::de::DeserializeOwned>(
        &self,
        builder: reqwest::RequestBuilder,
    ) -> DocResult<T> {
        let resp = builder
            .timeout(REQUEST_TIMEOUT)
            .send()
            .await
            .map_err(|e| KnowledgeDocumentsError::Transport(e.to_string()))?;
        let status = resp.status();
        if status.is_success() {
            return resp
                .json::<T>()
                .await
                .map_err(|e| KnowledgeDocumentsError::Parse(e.to_string()));
        }
        // Non-success: read the body text once for the typed error detail.
        let code = status.as_u16();
        let body = resp.text().await.unwrap_or_default();
        Err(map_error_status(code, &body))
    }
}

/// Map a non-success status + body into the typed [`KnowledgeDocumentsError`]. Pure (no IO) so the
/// status-to-variant contract is unit-provable without a live socket. The 409 body is parsed for a
/// `server_version` field best-effort; the current backend 409 body has none, so `server_version` is
/// `None` — but the caller STILL gets a distinct [`KnowledgeDocumentsError::SaveConflict`] (the
/// load-bearing data-loss control), never a generic error.
fn map_error_status(status: u16, body: &str) -> KnowledgeDocumentsError {
    let parsed: Option<Value> = serde_json::from_str(body).ok();
    let detail = parsed
        .as_ref()
        .and_then(|v| v.get("detail").or_else(|| v.get("reason")))
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| body.to_string());
    match status {
        400 => KnowledgeDocumentsError::BadRequest(detail),
        403 => KnowledgeDocumentsError::Forbidden(detail),
        404 => KnowledgeDocumentsError::NotFound(detail),
        409 => {
            let server_version = parsed
                .as_ref()
                .and_then(|v| v.get("server_version").or_else(|| v.get("current_version")))
                .and_then(Value::as_i64);
            KnowledgeDocumentsError::SaveConflict { server_version }
        }
        500..=599 => KnowledgeDocumentsError::Server(detail),
        other => KnowledgeDocumentsError::UnexpectedStatus {
            status: other,
            body: body.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    //! Pure unit proofs for the serialization + status-mapping contracts that DO NOT need a socket.
    //! The wire-level proofs (mock-server round-trips for load / 409 / 400 / double-option / batch /
    //! history clamp) live in `tests/test_knowledge_documents.rs` against real backend payload shapes.

    use super::*;
    use serde_json::json;

    #[test]
    fn move_request_double_option_serializes_three_ways() {
        // Absent (outer None) -> key OMITTED.
        let absent = MoveDocumentRequest::default();
        let v = serde_json::to_value(&absent).unwrap();
        assert!(
            v.get("project_ref").is_none() && v.get("folder_ref").is_none(),
            "an absent ref must be OMITTED from the JSON (absent != null): {v}"
        );

        // Explicit null (Some(None)) -> key present, value null.
        let clear = MoveDocumentRequest {
            project_ref: Some(None),
            folder_ref: None,
        };
        let v = serde_json::to_value(&clear).unwrap();
        assert_eq!(v["project_ref"], Value::Null, "Some(None) must serialize as null: {v}");
        assert!(v.get("folder_ref").is_none(), "absent folder_ref still omitted: {v}");

        // String (Some(Some)) -> key present, string value.
        let set = MoveDocumentRequest {
            project_ref: Some(Some("PROJ-1".into())),
            folder_ref: Some(Some("FOLDER-2".into())),
        };
        let v = serde_json::to_value(&set).unwrap();
        assert_eq!(v["project_ref"], json!("PROJ-1"));
        assert_eq!(v["folder_ref"], json!("FOLDER-2"));
    }

    #[test]
    fn batch_operation_tagged_union_matches_backend_op_tags() {
        let rename = BatchOperation::Rename {
            document_id: "KRD-1".into(),
            title: "T".into(),
        };
        assert_eq!(serde_json::to_value(&rename).unwrap()["op"], json!("rename"));

        let mv = BatchOperation::Move {
            document_id: "KRD-1".into(),
            project_ref: Some(None),
            folder_ref: None,
        };
        let v = serde_json::to_value(&mv).unwrap();
        assert_eq!(v["op"], json!("move"));
        assert_eq!(v["project_ref"], Value::Null, "batch move keeps double-option null: {v}");
        assert!(v.get("folder_ref").is_none(), "batch move keeps absent omitted: {v}");

        let label = BatchOperation::SetAuthorityLabel {
            document_id: "KRD-1".into(),
            authority_label: "promoted".into(),
        };
        assert_eq!(
            serde_json::to_value(&label).unwrap()["op"],
            json!("set_authority_label")
        );
    }

    #[test]
    fn status_map_409_is_distinct_save_conflict_not_generic() {
        let backend_409_body = json!({"error": "conflict", "detail": "version conflict"}).to_string();
        let err = map_error_status(409, &backend_409_body);
        assert!(
            matches!(err, KnowledgeDocumentsError::SaveConflict { server_version: None }),
            "the real backend 409 body (no server_version) must still be a DISTINCT SaveConflict, \
             not a generic error: {err:?}"
        );
        // A hypothetical future backend that DOES carry a version is parsed.
        let future = json!({"error": "conflict", "server_version": 7}).to_string();
        assert!(matches!(
            map_error_status(409, &future),
            KnowledgeDocumentsError::SaveConflict { server_version: Some(7) }
        ));
    }

    #[test]
    fn status_map_400_403_404_5xx_are_distinct_variants() {
        assert!(matches!(
            map_error_status(400, &json!({"detail": "x-hsk-actor-id header is required"}).to_string()),
            KnowledgeDocumentsError::BadRequest(_)
        ));
        assert!(matches!(
            map_error_status(403, &json!({"reason": "operator may not index"}).to_string()),
            KnowledgeDocumentsError::Forbidden(_)
        ));
        assert!(matches!(
            map_error_status(404, &json!({"detail": "not_found"}).to_string()),
            KnowledgeDocumentsError::NotFound(_)
        ));
        assert!(matches!(
            map_error_status(500, &json!({"error": "internal_error"}).to_string()),
            KnowledgeDocumentsError::Server(_)
        ));
        assert!(matches!(
            map_error_status(418, "teapot"),
            KnowledgeDocumentsError::UnexpectedStatus { status: 418, .. }
        ));
    }

    #[test]
    fn headers_omit_actor_kind_for_read_and_set_operator_for_write() {
        let read = HskDocumentHeaders::for_read("session-1", "KRD-1");
        assert!(read.actor_kind.is_none(), "a read omits actor-kind (least-privileged default)");
        assert_eq!(read.kernel_task_run_id, "native-editor-doc-KRD-1");

        let write = HskDocumentHeaders::for_operator("session-1", "KRD-1");
        assert_eq!(
            write.actor_kind.as_deref(),
            Some("operator"),
            "a write asserts the operator kind (a missing kind 403s a write)"
        );
    }

    #[test]
    fn header_field_names_match_backend_constants() {
        // The five header constants the AC names, sourced from backend_client (single source of truth).
        assert_eq!(HSK_HEADER_ACTOR_ID, "x-hsk-actor-id");
        assert_eq!(HSK_HEADER_KERNEL_TASK_RUN_ID, "x-hsk-kernel-task-run-id");
        assert_eq!(HSK_HEADER_SESSION_RUN_ID, "x-hsk-session-run-id");
        assert_eq!(HSK_HEADER_ACTOR_KIND, "x-hsk-actor-kind");
        assert_eq!(HSK_HEADER_CORRELATION_ID, "x-hsk-correlation-id");
    }
}
