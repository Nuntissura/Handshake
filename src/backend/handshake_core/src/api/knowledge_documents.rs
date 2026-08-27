//! WP-KERNEL-009 RichDocumentCore (MT-145..MT-160): the backend HTTP surface
//! for the RichDocument authority model, wiring the editor to single-store +
//! EventLedger authority (NO mocks, no SQLite).
//!
//! WP-KERNEL-012 MT-136: the HTTP surface is backed by `SurrealDatabase` over
//! Handshake's embedded SurrealDB store. There is no relational fallback.
//!
//! This is the keystone API for the group:
//!   * MT-145 identity + MT-149 save/load: create / load / save a RichDocument
//!     against `knowledge_rich_documents` (build on MT-059 optimistic-concurrency
//!     save), each save leaving a `KNOWLEDGE_RICH_DOCUMENT_SAVED` EventLedger
//!     receipt.
//!   * MT-146/147/148 block tree: load returns the typed block tree (block ids,
//!     Raw/Derived/Display) so the frontend renders stable blocks.
//!   * MT-150 projection export + MT-151 import: render a document to a chosen
//!     projection format, or import a snippet into a new document.
//!   * MT-152/153 embeds: list/repair the typed embed references and the
//!     broken-embed repair queue.
//!   * MT-154/155 search-index + backlinks: extract + persist the document's
//!     backlinks (stable relationship ids) and expose forward/reverse lookups.
//!   * MT-156 history: the append-only revision history + receipts.
//!   * MT-157 batch ops: safe batch rename / move (project/folder) / set owner.
//!   * MT-158 permission boundary: every write/index is gated server-side.
//!
//! Backend-navigation receipt law (spec 2.3.13.11): a read is attributable; a
//! write/promotion leaves a receipt. Every endpoint REQUIRES the identity
//! headers (400 otherwise) — `x-hsk-actor-id`, `x-hsk-kernel-task-run-id`,
//! `x-hsk-session-run-id`, plus optional `x-hsk-actor-kind`,
//! `x-hsk-correlation-id`. The actor-kind drives the MT-158 permission
//! boundary and FAILS CLOSED (adversarial-v2 hardening): a missing
//! `x-hsk-actor-kind` is the least-privileged read-only actor, an unknown
//! token is a 400 — privilege must be explicitly asserted and is validated
//! server-side, never inferred.
//!
//! Conventions mirror `api/knowledge_memory.rs`.

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post, put},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::kernel::{KernelActor, KernelEventType, NewKernelEvent};
use crate::knowledge_document::backlink::DocumentLinkReferences;
use crate::knowledge_document::block_tree::BlockTree;
use crate::knowledge_document::embed::{validate_block_embeds, ValidatedBlockEmbed};
use crate::knowledge_document::import::{import_snippet, ImportFormat};
use crate::knowledge_document::permission::{
    DocumentAction, DocumentActorKind, DocumentPermission,
};
use crate::knowledge_document::projection::{render_projection, ProjectionFormat};
use crate::storage::knowledge::{
    KnowledgeEntityKind, KnowledgePermissionScope, KnowledgeRedactionState, KnowledgeRichDocument,
    KnowledgeSourceKind, KnowledgeStore, NewKnowledgeEntity, NewKnowledgeRichDocument,
    NewKnowledgeSource, UpsertKnowledgeDocumentBacklink, UpsertKnowledgeDocumentEmbed,
    UpsertKnowledgeRichDocumentDraft,
};
use crate::storage::surreal::SurrealDatabase;
use crate::storage::{Database, StorageError};
use crate::AppState;

const HSK_HEADER_ACTOR_KIND: &str = "x-hsk-actor-kind";
const HSK_HEADER_ACTOR_ID: &str = "x-hsk-actor-id";
const HSK_HEADER_KERNEL_TASK_RUN_ID: &str = "x-hsk-kernel-task-run-id";
const HSK_HEADER_SESSION_RUN_ID: &str = "x-hsk-session-run-id";
const HSK_HEADER_CORRELATION_ID: &str = "x-hsk-correlation-id";
/// The native-MCP session credential. Same spelling as `api::stage`'s private constant — that module
/// owns the validation, this module only decides whether to present the headers to it.
const HSK_HEADER_SESSION_TOKEN: &str = "x-hsk-session-token";

/// WP-KERNEL-012 MT-120: the SERVER-WRITTEN field in a `KNOWLEDGE_RICH_DOCUMENT_SAVED` receipt payload
/// naming the authenticated native principal that minted the receipt.
///
/// It exists because the ledger `actor_id` column is the CLIENT-declared per-agent attribution (two
/// swarm agents saving the same document must remain individually attributable), while the Flight
/// Recorder derives its own process principal from `stage::capture_context`. Those are deliberately
/// different values, so receipt ownership needs its own server-written anchor: this field. It is
/// written ONLY from an authenticated session and is never accepted from the request body.
pub const SAVE_RECEIPT_MINTED_BY_PRINCIPAL_FIELD: &str = "minted_by_principal";

/// The actor-id namespace `stage::capture_context` mints (`handshake-native:{pid}:{fingerprint}`).
/// A client may not declare an id in this namespace unless it authenticated AS that exact principal.
const RESERVED_NATIVE_PRINCIPAL_PREFIX: &str = "handshake-native:";

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route(
            "/knowledge/documents",
            get(list_documents).post(create_document),
        )
        .route("/knowledge/documents/import", post(import_document))
        .route(
            "/knowledge/documents/:document_id",
            get(load_document).delete(delete_document),
        )
        .route(
            "/knowledge/documents/:document_id/draft",
            get(load_document_draft)
                .put(upsert_document_draft)
                .delete(clear_document_draft),
        )
        .route("/knowledge/documents/:document_id/save", put(save_document))
        .route("/knowledge/documents/:document_id/blocks", get(load_blocks))
        .route(
            "/knowledge/documents/:document_id/history",
            get(load_history),
        )
        .route(
            "/knowledge/documents/:document_id/history/:doc_version",
            get(load_history_version),
        )
        .route(
            "/knowledge/documents/:document_id/projection",
            get(export_projection),
        )
        .route("/knowledge/documents/:document_id/embeds", get(list_embeds))
        .route(
            "/knowledge/documents/:document_id/embeds/broken",
            get(list_broken_embeds),
        )
        .route(
            "/knowledge/documents/embeds/:embed_id/repair",
            post(repair_embed),
        )
        .route(
            "/knowledge/documents/:document_id/backlinks",
            get(list_backlinks).post(rebuild_backlinks),
        )
        .route(
            "/knowledge/documents/:document_id/rename",
            post(rename_document),
        )
        .route(
            "/knowledge/documents/:document_id/move",
            post(move_document),
        )
        .route("/knowledge/documents/batch", post(batch_documents))
        .with_state(state)
}

type ApiError = (StatusCode, Json<Value>);

fn db_for(state: &AppState) -> SurrealDatabase {
    SurrealDatabase::new(state.surreal.clone())
}

fn header_str<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn bad_request(detail: impl Into<String>) -> ApiError {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({"error": "bad_request", "detail": detail.into()})),
    )
}

fn canonical_rich_document_crdt_document_id(rich_document_id: &str) -> String {
    rich_document_id
        .strip_prefix("KRD-")
        .map(|suffix| format!("KCRDT-{suffix}"))
        .unwrap_or_else(|| format!("KCRDT-{rich_document_id}"))
}

async fn validated_save_crdt_document_id(
    db: &dyn KnowledgeStore,
    rich_document_id: &str,
    requested_crdt_document_id: Option<&str>,
) -> Result<Option<String>, ApiError> {
    let Some(requested) = requested_crdt_document_id.map(str::trim) else {
        return Ok(None);
    };
    if requested.is_empty() {
        return Err(bad_request(
            "crdt_document_id must be non-empty when supplied",
        ));
    }

    let document = db
        .get_knowledge_rich_document(rich_document_id)
        .await
        .map_err(storage_error)?
        .ok_or_else(|| not_found("knowledge rich document"))?;
    let allowed = document
        .crdt_document_id
        .unwrap_or_else(|| canonical_rich_document_crdt_document_id(rich_document_id));
    if requested != allowed {
        return Err(bad_request(format!(
            "crdt_document_id '{requested}' does not belong to rich document '{rich_document_id}'"
        )));
    }

    Ok(Some(requested.to_string()))
}

fn not_found(detail: impl Into<String>) -> ApiError {
    (
        StatusCode::NOT_FOUND,
        Json(json!({"error": "not_found", "detail": detail.into()})),
    )
}

fn forbidden(reason: impl Into<String>) -> ApiError {
    (
        StatusCode::FORBIDDEN,
        Json(json!({"error": "forbidden", "reason": reason.into()})),
    )
}

/// WP-KERNEL-012 MT-120: an `x-hsk-session-token` was PRESENTED and did not authenticate (absent,
/// forged, or stale binding). This is deliberately a hard 401 and NEVER a downgrade to the header
/// identity: silently continuing as the client-declared actor would let a failed credential buy the
/// unauthenticated path, which is the single most dangerous failure mode of an optional credential.
fn doc_session_unauthenticated() -> ApiError {
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({
            "error": "HSK-401-DOC-SESSION",
            "detail": "x-hsk-session-token was presented but did not authenticate a live native-MCP session"
        })),
    )
}

/// WP-KERNEL-012 MT-120: the caller declared an `x-hsk-actor-id` inside the reserved
/// `handshake-native:` principal namespace without an authenticated session that owns exactly that
/// id. Without this guard an unauthenticated caller forges the server-derived principal by header.
fn doc_actor_spoof_denied() -> ApiError {
    (
        StatusCode::FORBIDDEN,
        Json(json!({
            "error": "HSK-403-DOC-ACTOR-SPOOF",
            "reason": "x-hsk-actor-id claims the reserved handshake-native: principal namespace without an authenticated session for that principal"
        })),
    )
}

fn conflict(detail: impl Into<String>) -> ApiError {
    (
        StatusCode::CONFLICT,
        Json(json!({"error": "conflict", "detail": detail.into()})),
    )
}

fn storage_error(err: StorageError) -> ApiError {
    match err {
        StorageError::NotFound(what) => not_found(what),
        StorageError::Validation(detail) => bad_request(detail),
        StorageError::Conflict(detail) => conflict(detail),
        other => {
            tracing::error!(
                target: "handshake_core::knowledge_documents_api",
                error = %other,
                "rich_document_api_internal_error"
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "internal_error"})),
            )
        }
    }
}

/// The backend-navigation identity required on every document request.
struct DocContext {
    actor: KernelActor,
    actor_kind: DocumentActorKind,
    kernel_task_run_id: String,
    session_run_id: String,
    correlation_id: Option<String>,
    /// WP-KERNEL-012 MT-120: the SERVER-DERIVED native principal when the caller presented a valid
    /// `x-hsk-session-token`, otherwise `None`. It never replaces `actor` (per-agent attribution),
    /// it is an ADDITIONAL server-written anchor consumed only by the save receipt.
    minted_by_principal: Option<String>,
}

/// WP-KERNEL-012 MT-120 — OPTIONAL-BUT-VERIFIED session resolution.
///
/// * header ABSENT  -> `Ok(None)`; the route behaves exactly as it did before this MT (no filesystem
///   read, no process probe, no behavior change for any existing caller).
/// * header PRESENT and valid -> `Ok(Some(server-derived actor id))`.
/// * header PRESENT and invalid/stale -> `Err(401 HSK-401-DOC-SESSION)`. NEVER a silent downgrade to
///   the client-declared header identity.
fn authenticated_native_principal(headers: &HeaderMap) -> Result<Option<String>, ApiError> {
    if header_str(headers, HSK_HEADER_SESSION_TOKEN).is_none() {
        return Ok(None);
    }
    match crate::api::stage::capture_context(headers) {
        Ok(ctx) => Ok(Some(ctx.actor_id)),
        Err(_) => Err(doc_session_unauthenticated()),
    }
}

fn doc_context(headers: &HeaderMap) -> Result<DocContext, ApiError> {
    // Resolve the credential FIRST: a presented-but-invalid token must fail closed regardless of
    // which other headers are missing, so a broken credential can never be laundered into a 400.
    let minted_by_principal = authenticated_native_principal(headers)?;
    let actor_id = header_str(headers, HSK_HEADER_ACTOR_ID)
        .ok_or_else(|| bad_request(format!("{HSK_HEADER_ACTOR_ID} header is required")))?
        .to_string();
    let kernel_task_run_id = header_str(headers, HSK_HEADER_KERNEL_TASK_RUN_ID)
        .ok_or_else(|| {
            bad_request(format!(
                "{HSK_HEADER_KERNEL_TASK_RUN_ID} header is required"
            ))
        })?
        .to_string();
    let session_run_id = header_str(headers, HSK_HEADER_SESSION_RUN_ID)
        .ok_or_else(|| bad_request(format!("{HSK_HEADER_SESSION_RUN_ID} header is required")))?
        .to_string();
    // WP-KERNEL-012 MT-120 RESERVED-NAMESPACE GUARD (every document route, not just save): the
    // `handshake-native:` namespace belongs to `stage::capture_context`. A caller may only declare an
    // id inside it when it authenticated as EXACTLY that principal. Without this an unauthenticated
    // request forges the derived principal with a single header.
    if actor_id.starts_with(RESERVED_NATIVE_PRINCIPAL_PREFIX)
        && minted_by_principal.as_deref() != Some(actor_id.as_str())
    {
        return Err(doc_actor_spoof_denied());
    }
    // MT-158 hardening (adversarial-v2): the actor kind is a free client
    // string, so it is validated STRICTLY server-side and privilege is never
    // inferred. A missing header is the LEAST-privileged kind (read-only),
    // never `system`; an unknown token is a 400, never a coercion.
    let actor_kind = match header_str(headers, HSK_HEADER_ACTOR_KIND) {
        None => DocumentActorKind::least_privileged(),
        Some(value) => DocumentActorKind::from_wire(value)
            .ok_or_else(|| bad_request(format!("unknown {HSK_HEADER_ACTOR_KIND} '{value}'")))?,
    };
    // Map the document actor-kind to the KernelActor used for receipts. The
    // binding is SERVER-derived from the validated kind — a caller can never
    // pick an arbitrary KernelActor.
    let actor = match actor_kind {
        DocumentActorKind::Operator => KernelActor::Operator(actor_id),
        DocumentActorKind::System => KernelActor::System(actor_id),
        DocumentActorKind::Validator => KernelActor::ValidationRunner(actor_id),
        // Unauthenticated callers attribute as the least-trusted adapter
        // bucket; they can never reach a receipt-recording path because every
        // write/index action is denied for them (permission.rs MT-158 matrix).
        DocumentActorKind::LocalModel
        | DocumentActorKind::CloudModel
        | DocumentActorKind::Unauthenticated => KernelActor::ModelAdapter(actor_id),
    };
    Ok(DocContext {
        actor,
        actor_kind,
        kernel_task_run_id,
        session_run_id,
        correlation_id: header_str(headers, HSK_HEADER_CORRELATION_ID).map(ToOwned::to_owned),
        minted_by_principal,
    })
}

impl DocContext {
    /// Enforce the MT-158 permission boundary; returns a 403 on denial.
    fn require(&self, action: DocumentAction) -> Result<(), ApiError> {
        let decision = DocumentPermission::decide(self.actor_kind, action);
        if decision.allowed {
            Ok(())
        } else {
            Err(forbidden(decision.reason))
        }
    }
}

/// Render an ApiError into a short diagnostic string for the non-fatal
/// post-commit recording path (MT-149).
fn api_error_detail(err: &ApiError) -> String {
    format!("{} {}", err.0.as_u16(), err.1 .0)
}

/// Append a receipt for a write that has ALREADY committed (adversarial-v2
/// MT-149): a receipt failure must never turn a committed write into an error
/// response — it is recorded in the response (and the log) instead.
async fn record_receipt_non_fatal(
    db: &dyn Database,
    ctx: &DocContext,
    event_type: KernelEventType,
    rich_document_id: &str,
    payload: Value,
) -> (Option<String>, Option<String>) {
    match record_receipt(db, ctx, event_type, rich_document_id, payload).await {
        Ok(event_id) => (Some(event_id), None),
        Err(err) => {
            let detail = api_error_detail(&err);
            tracing::error!(
                target: "handshake_core::knowledge_documents_api",
                rich_document_id,
                error = %detail,
                "rich_document_receipt_failed_post_commit"
            );
            (None, Some(detail))
        }
    }
}

/// Index a RichDocument into the Project Knowledge Index (adversarial-v2
/// MT-154): the document becomes a first-class knowledge SOURCE (kind
/// `rich_document`, content-hash tracked — a changed document marks its
/// source STALE for the indexing pipeline, exactly like file sources) and a
/// first-class ENTITY (kind `rich_document`, key = the document id, display
/// name = the title) anchored on that source. Links + tags are already
/// indexed as backlink edges (MT-155) and embeds in the typed side table
/// (MT-152); this closes the title/blocks half: the title is queryable
/// through the entity surface and the blocks' bytes are the source's
/// content-hash-tracked indexing unit.
async fn index_document_into_knowledge_index(
    db: &dyn KnowledgeStore,
    document: &KnowledgeRichDocument,
) -> Result<(), StorageError> {
    let source = match db
        .get_knowledge_source_by_document_id(&document.workspace_id, &document.rich_document_id)
        .await?
    {
        Some(existing) => {
            if existing.content_hash != document.content_sha256 && !existing.stale {
                // The document changed since the source was indexed: stale is
                // the truthful index state until the pipeline re-indexes it.
                db.mark_knowledge_source_stale(&existing.source_id).await?
            } else {
                existing
            }
        }
        None => {
            db.upsert_knowledge_source(NewKnowledgeSource {
                workspace_id: document.workspace_id.clone(),
                root_id: None,
                source_kind: KnowledgeSourceKind::RichDocument,
                relative_path: None,
                asset_id: None,
                loom_block_id: None,
                // The schema's document_id column FKs the LEGACY documents
                // table; the KRD linkage is provenance-keyed (see
                // get_knowledge_source_by_document_id).
                document_id: None,
                content_hash: document.content_sha256.clone(),
                size_bytes: Some(document.content_json.to_string().len() as i64),
                provenance: json!({
                    "discovered_by": "knowledge_documents_api",
                    "rich_document_id": document.rich_document_id,
                    "schema_version": document.schema_version,
                }),
                permission_scope: KnowledgePermissionScope::Workspace,
                redaction_state: KnowledgeRedactionState::None,
                source_modified_at: None,
            })
            .await?
        }
    };

    db.upsert_knowledge_entity(NewKnowledgeEntity {
        workspace_id: document.workspace_id.clone(),
        entity_kind: KnowledgeEntityKind::RichDocument,
        entity_key: document.rich_document_id.clone(),
        display_name: document.title.clone(),
        detection_provenance: json!({
            "extractor": "knowledge_documents_api",
            "content_sha256": document.content_sha256,
            "doc_version": document.doc_version,
        }),
        primary_source_id: Some(source.source_id),
        detected_in_run: None,
        evidence_span_ids: vec![],
    })
    .await?;
    Ok(())
}

/// Run the MT-154 index step post-commit and RECORD a failure instead of
/// erroring a committed write (MT-149 law). Returns (indexed, error).
async fn index_document_non_fatal(
    db: &dyn KnowledgeStore,
    document: &KnowledgeRichDocument,
) -> (bool, Option<String>) {
    match index_document_into_knowledge_index(db, document).await {
        Ok(()) => (true, None),
        Err(err) => {
            tracing::error!(
                target: "handshake_core::knowledge_documents_api",
                rich_document_id = %document.rich_document_id,
                error = %err,
                "rich_document_knowledge_index_failed_post_commit"
            );
            (false, Some(err.to_string()))
        }
    }
}

/// Map validated content embeds (MT-152) to side-table upserts.
fn embed_upserts(
    rich_document_id: &str,
    validated: &[ValidatedBlockEmbed],
) -> Vec<UpsertKnowledgeDocumentEmbed> {
    validated
        .iter()
        .map(|embed| UpsertKnowledgeDocumentEmbed {
            rich_document_id: rich_document_id.to_string(),
            block_id: embed.block_id.clone(),
            ref_kind: embed.target.kind.as_str().to_string(),
            ref_value: embed.target.value.clone(),
            caption: embed.caption.clone(),
        })
        .collect()
}

/// Append a document EventLedger receipt (save/promotion/nav) and return its id.
async fn record_receipt(
    db: &dyn Database,
    ctx: &DocContext,
    event_type: KernelEventType,
    rich_document_id: &str,
    payload: Value,
) -> Result<String, ApiError> {
    let event = build_receipt_event(ctx, event_type, rich_document_id, payload)?;
    let stored = db.append_kernel_event(event).await.map_err(storage_error)?;
    Ok(stored.event_id)
}

fn build_receipt_event(
    ctx: &DocContext,
    event_type: KernelEventType,
    rich_document_id: &str,
    payload: Value,
) -> Result<NewKernelEvent, ApiError> {
    let mut builder = NewKernelEvent::builder(
        ctx.kernel_task_run_id.clone(),
        ctx.session_run_id.clone(),
        event_type,
        ctx.actor.clone(),
    )
    .aggregate("knowledge_rich_document", rich_document_id.to_string())
    .source_component("knowledge_documents_api")
    .payload(payload);
    if let Some(correlation_id) = &ctx.correlation_id {
        builder = builder.correlation_id(correlation_id.clone());
    }
    builder.build().map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "receipt_build_failed", "detail": err.to_string()})),
        )
    })
}

/// Parse + serialize a document into the typed block-tree view used by load and
/// blocks endpoints (MT-146/147/148). A schema mismatch is surfaced, not
/// silently coerced (spec 7.1.1.8).
fn block_tree_view(
    rich_document_id: &str,
    schema_version: &str,
    content_json: &Value,
) -> Result<Value, ApiError> {
    let tree = BlockTree::from_document_json(rich_document_id, schema_version, content_json)
        .map_err(|err| bad_request(format!("document block tree is malformed: {err}")))?;
    Ok(json!({
        "schema_version": schema_version,
        "schema_matches": tree.schema_matches(),
        "block_ids": tree.block_ids(),
        "blocks": tree.blocks,
    }))
}

// ---------------------------------------------------------------------------
// Request bodies.
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct CreateDocumentBody {
    workspace_id: String,
    title: String,
    /// Wikilink create-note semantic: serialize concurrent callers at store authority and return
    /// the single existing title match instead of creating another document.
    #[serde(default)]
    create_if_title_absent: bool,
    /// ProseMirror doc node JSON. Defaults to an empty doc.
    #[serde(default)]
    content_json: Option<Value>,
    #[serde(default)]
    schema_version: Option<String>,
    #[serde(default)]
    project_ref: Option<String>,
    #[serde(default)]
    folder_ref: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SaveDocumentBody {
    expected_version: i64,
    content_json: Value,
    #[serde(default)]
    crdt_document_id: Option<String>,
    #[serde(default)]
    crdt_snapshot_id: Option<String>,
    #[serde(default)]
    promotion_receipt_event_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpsertDocumentDraftBody {
    base_doc_version: i64,
    base_content_sha256: String,
    content_json: Value,
}

#[derive(Debug, Deserialize)]
struct ImportDocumentBody {
    workspace_id: String,
    title: String,
    format: String,
    snippet: String,
}

#[derive(Debug, Deserialize)]
struct ProjectionParams {
    format: String,
}

/// Pagination for the history list (adversarial-v2 MT-156). Defaults bound the
/// response even when the caller passes nothing.
#[derive(Debug, Deserialize)]
struct HistoryParams {
    #[serde(default)]
    limit: Option<i64>,
    #[serde(default)]
    offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ListDocumentsParams {
    workspace_id: String,
    #[serde(default)]
    project_ref: Option<String>,
    #[serde(default)]
    folder_ref: Option<String>,
}

/// History pagination bounds (MT-156): a caller can never request an
/// unbounded page.
const HISTORY_DEFAULT_LIMIT: i64 = 50;
const HISTORY_MAX_LIMIT: i64 = 200;

#[derive(Debug, Deserialize)]
struct RepairEmbedBody {
    /// `relink` | `reresolve` | `remove` (intent), recorded in the receipt; the
    /// repair-state transition itself is broken<->ok.
    #[serde(default)]
    action: Option<String>,
    /// When provided, marks the embed broken with this reason; when absent,
    /// repairs it back to ok.
    #[serde(default)]
    broken_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RenameBody {
    title: String,
    /// Optimistic-concurrency token captured when the title was presented to the operator.
    /// Omitting it preserves the batch/API compatibility path; interactive explorer rename always sends it.
    #[serde(default)]
    expected_updated_at: Option<DateTime<Utc>>,
}

/// Move body with absent-vs-null semantics (adversarial-v2 MT-157): an ABSENT
/// field leaves that membership unchanged; an explicit `null` clears it; a
/// string sets it. Before this hardening an empty body silently cleared BOTH
/// memberships.
#[derive(Debug, Deserialize)]
struct MoveBody {
    #[serde(default, deserialize_with = "double_option")]
    project_ref: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    folder_ref: Option<Option<String>>,
}

/// Deserialize a present-but-possibly-null field into `Some(Option<T>)`,
/// leaving an absent field as `None` (the serde double-Option idiom).
fn double_option<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: serde::Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    serde::Deserialize::deserialize(deserializer).map(Some)
}

/// One operation in a batch request (MT-157): rename / move /
/// set_authority_label, applied per-document with per-item receipts and
/// partial-failure reporting.
#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
enum BatchOperation {
    Rename {
        document_id: String,
        title: String,
    },
    Move {
        document_id: String,
        #[serde(default, deserialize_with = "double_option")]
        project_ref: Option<Option<String>>,
        #[serde(default, deserialize_with = "double_option")]
        folder_ref: Option<Option<String>>,
    },
    SetAuthorityLabel {
        document_id: String,
        authority_label: String,
    },
}

impl BatchOperation {
    fn document_id(&self) -> &str {
        match self {
            Self::Rename { document_id, .. }
            | Self::Move { document_id, .. }
            | Self::SetAuthorityLabel { document_id, .. } => document_id,
        }
    }

    fn op_name(&self) -> &'static str {
        match self {
            Self::Rename { .. } => "rename",
            Self::Move { .. } => "move",
            Self::SetAuthorityLabel { .. } => "set_authority_label",
        }
    }
}

#[derive(Debug, Deserialize)]
struct BatchBody {
    operations: Vec<BatchOperation>,
}

/// Bound on one batch request (MT-157): keeps a batch a bounded, reviewable
/// unit instead of an unbounded sweep.
const BATCH_MAX_OPERATIONS: usize = 100;

// ---------------------------------------------------------------------------
// Handlers.
// ---------------------------------------------------------------------------

/// GET /knowledge/documents — enumerate the live RichDocument authority rows for one workspace.
/// This is the explorer's identity source: ids and optimistic tokens come from the same rows the
/// rename endpoint mutates, never from the legacy `documents` table.
async fn list_documents(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<ListDocumentsParams>,
) -> Result<Json<Value>, ApiError> {
    let ctx = doc_context(&headers)?;
    ctx.require(DocumentAction::Read)?;
    if params.workspace_id.trim().is_empty() {
        return Err(bad_request("workspace_id query parameter is required"));
    }
    let db = db_for(&state);
    let documents = db
        .list_knowledge_rich_documents(
            &params.workspace_id,
            params.project_ref.as_deref(),
            params.folder_ref.as_deref(),
        )
        .await
        .map_err(storage_error)?;
    let summaries: Vec<Value> = documents
        .into_iter()
        .map(|document| {
            json!({
                "rich_document_id": document.rich_document_id,
                "title": document.title,
                "updated_at": document.updated_at,
            })
        })
        .collect();
    Ok(Json(json!(summaries)))
}

/// POST /knowledge/documents — create a RichDocument (MT-145/149).
async fn create_document(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CreateDocumentBody>,
) -> Result<Json<Value>, ApiError> {
    let ctx = doc_context(&headers)?;
    ctx.require(DocumentAction::Write)?;
    let db = db_for(&state);

    let schema_version = body.schema_version.unwrap_or_else(|| {
        crate::knowledge_document::block_tree::DOCUMENT_SCHEMA_VERSION.to_string()
    });
    let content_json = body
        .content_json
        .unwrap_or_else(|| json!({"type": "doc", "content": []}));

    // MT-146/152 (adversarial-v2): the created content must be a valid block
    // tree AND every embed block must satisfy the typed EmbedTarget law BEFORE
    // anything commits — the same law that guards the side table governs the
    // authority content itself (no javascript:/data:/absolute-path targets).
    let tree = BlockTree::from_document_json("KRD-pending", &schema_version, &content_json)
        .map_err(|err| bad_request(format!("document block tree is malformed: {err}")))?;
    validate_block_embeds(&tree).map_err(|(block_id, err)| {
        bad_request(format!("embed block `{block_id}` target rejected: {err}"))
    })?;

    let new_document = NewKnowledgeRichDocument {
        workspace_id: body.workspace_id,
        document_id: None,
        title: body.title,
        schema_version,
        content_json,
        crdt_document_id: None,
        crdt_snapshot_id: None,
        promotion_receipt_event_id: None,
        project_ref: body.project_ref,
        folder_ref: body.folder_ref,
        authority_label: Some("promoted".to_string()),
        owner_actor_kind: Some(ctx.actor_kind.as_str().to_string()),
        owner_actor_id: Some(actor_id_of(&ctx.actor)),
    };
    let (created, document_created) = if body.create_if_title_absent {
        db.create_knowledge_rich_document_if_title_absent(new_document)
            .await
            .map_err(storage_error)?
    } else {
        (
            db.create_knowledge_rich_document(new_document)
                .await
                .map_err(storage_error)?,
            true,
        )
    };

    if !document_created {
        return Ok(Json(json!({
            "document": created,
            "created": false,
            "save_receipt_event_id": Value::Null,
            "receipt_error": Value::Null,
            "embeds_persisted": 0,
            "embeds_error": Value::Null,
            "knowledge_indexed": false,
            "knowledge_index_error": Value::Null,
        })));
    }

    // ---- post-commit (MT-149): the create above is committed; the steps
    // below are best-effort and RECORDED, never an error for a committed write.
    let (receipt, receipt_error) = record_receipt_non_fatal(
        state.storage.as_ref(),
        &ctx,
        KernelEventType::KnowledgeRichDocumentSaved,
        &created.rich_document_id,
        json!({"event": "created", "doc_version": created.doc_version}),
    )
    .await;

    // MT-152: sync the typed embed side table from the validated content.
    // Re-validate against the REAL document id so derived block ids match.
    let mut embeds_persisted = 0usize;
    let mut embeds_error: Option<String> = None;
    let created_tree = BlockTree::from_document_json(
        &created.rich_document_id,
        &created.schema_version,
        &created.content_json,
    )
    .ok();
    if let Some(created_tree) = created_tree {
        if let Ok(validated) = validate_block_embeds(&created_tree) {
            match db
                .replace_knowledge_document_embeds(
                    &created.rich_document_id,
                    embed_upserts(&created.rich_document_id, &validated),
                )
                .await
            {
                Ok(persisted) => embeds_persisted = persisted.len(),
                Err(err) => {
                    tracing::error!(
                        target: "handshake_core::knowledge_documents_api",
                        rich_document_id = %created.rich_document_id,
                        error = %err,
                        "rich_document_embed_sync_failed_post_commit"
                    );
                    embeds_error = Some(err.to_string());
                }
            }
        }
    }

    // MT-154: index the created document (source + title entity) when the
    // actor may index; denial just skips (read-only actors cannot create).
    let mut knowledge_indexed = false;
    let mut knowledge_index_error: Option<String> = None;
    if ctx.require(DocumentAction::Index).is_ok() {
        (knowledge_indexed, knowledge_index_error) = index_document_non_fatal(&db, &created).await;
    }

    Ok(Json(json!({
        "document": created,
        "created": true,
        "save_receipt_event_id": receipt,
        "receipt_error": receipt_error,
        "embeds_persisted": embeds_persisted,
        "embeds_error": embeds_error,
        "knowledge_indexed": knowledge_indexed,
        "knowledge_index_error": knowledge_index_error,
    })))
}

/// GET /knowledge/documents/:document_id — load a RichDocument + block tree
/// (MT-149 load, MT-146/147/148 tree).
async fn load_document(
    State(state): State<AppState>,
    Path(document_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let ctx = doc_context(&headers)?;
    ctx.require(DocumentAction::Read)?;
    let db = db_for(&state);

    let document = db
        .get_knowledge_rich_document(&document_id)
        .await
        .map_err(storage_error)?
        .ok_or_else(|| not_found("knowledge rich document"))?;
    let code_nodes = db
        .list_knowledge_editor_code_nodes(&document_id)
        .await
        .map_err(storage_error)?;
    let tree = block_tree_view(
        &document.rich_document_id,
        &document.schema_version,
        &document.content_json,
    )?;

    Ok(Json(json!({
        "document": document,
        "tree": tree,
        "code_nodes": code_nodes,
    })))
}

/// DELETE /knowledge/documents/:document_id — SOFT delete (tombstone).
///
/// Preserves EventLedger lineage: the authority row is marked deleted (never
/// dropped), the delete is recorded as a `KNOWLEDGE_RICH_DOCUMENT_DELETED`
/// EventLedger receipt (who/when/why is auditable), and the document's knowledge
/// SOURCE is marked stale so retrieval stops treating the deleted document's
/// blocks as fresh authority. The document's knowledge ENTITY has no stale/
/// lifecycle flag in the current schema, so the stale SOURCE (the index unit the
/// pipeline re-reads) is the marking surface. Requires `DocumentAction::Write`.
/// Additive: no frontend caller exists yet.
async fn delete_document(
    State(state): State<AppState>,
    Path(document_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let ctx = doc_context(&headers)?;
    ctx.require(DocumentAction::Write)?;
    let db = db_for(&state);
    let document = db
        .get_knowledge_rich_document(&document_id)
        .await
        .map_err(storage_error)?
        .ok_or_else(|| not_found("knowledge rich document"))?;
    let event = build_receipt_event(
        &ctx,
        KernelEventType::KnowledgeRichDocumentDeleted,
        &document.rich_document_id,
        json!({
            "event": "deleted",
            "workspace_id": document.workspace_id.clone(),
            "doc_version": document.doc_version,
            "title": document.title.clone(),
        }),
    )?;
    let outcome = db
        .delete_knowledge_rich_document_atomic(&document, event)
        .await
        .map_err(storage_error)?;

    Ok(Json(json!({
        "deleted": true,
        "rich_document_id": document.rich_document_id,
        "deleted_receipt_event_id": outcome.receipt_event_id,
        "source_marked_stale": outcome.source_marked_stale,
        "backlinks_deleted": outcome.backlinks_deleted,
        "loom_block_deleted": outcome.loom_block_deleted,
    })))
}

/// GET /knowledge/documents/:document_id/draft — load backend-persisted
/// unsaved editor content for crash recovery (MT-255).
async fn load_document_draft(
    State(state): State<AppState>,
    Path(document_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let ctx = doc_context(&headers)?;
    ctx.require(DocumentAction::Read)?;
    let db = db_for(&state);

    let document = db
        .get_knowledge_rich_document(&document_id)
        .await
        .map_err(storage_error)?
        .ok_or_else(|| not_found("knowledge rich document"))?;
    let draft = db
        .get_knowledge_rich_document_draft(&document_id)
        .await
        .map_err(storage_error)?;

    let visible_draft = draft.filter(|draft| {
        draft.draft_content_sha256 != document.content_sha256
            && draft.draft_content_json != document.content_json
    });

    Ok(Json(json!({
        "rich_document_id": document.rich_document_id,
        "current_doc_version": document.doc_version,
        "current_content_sha256": document.content_sha256,
        "draft": visible_draft,
    })))
}

/// PUT /knowledge/documents/:document_id/draft — persist unsaved editor
/// content to the durable store so a crash/reopen can offer restore/discard (MT-255).
async fn upsert_document_draft(
    State(state): State<AppState>,
    Path(document_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<UpsertDocumentDraftBody>,
) -> Result<Json<Value>, ApiError> {
    let ctx = doc_context(&headers)?;
    ctx.require(DocumentAction::Write)?;
    let db = db_for(&state);

    let document = db
        .get_knowledge_rich_document(&document_id)
        .await
        .map_err(storage_error)?
        .ok_or_else(|| not_found("knowledge rich document"))?;
    if body.base_doc_version > document.doc_version {
        return Err(conflict(
            "draft base version is newer than the current document",
        ));
    }
    if body.base_doc_version == document.doc_version
        && body.base_content_sha256 != document.content_sha256
    {
        return Err(conflict(
            "draft base content hash does not match the current document",
        ));
    }

    let tree = BlockTree::from_document_json(
        &document.rich_document_id,
        &document.schema_version,
        &body.content_json,
    )
    .map_err(|err| bad_request(format!("draft block tree is malformed: {err}")))?;
    validate_block_embeds(&tree).map_err(|(block_id, err)| {
        bad_request(format!(
            "draft embed block `{block_id}` target rejected: {err}"
        ))
    })?;

    if body.content_json == document.content_json {
        let cleared = db
            .clear_knowledge_rich_document_draft(&document.rich_document_id)
            .await
            .map_err(storage_error)?;
        let (receipt, receipt_error) = record_receipt_non_fatal(
            state.storage.as_ref(),
            &ctx,
            KernelEventType::KnowledgeCrdtRecoveryReceiptRecorded,
            &document.rich_document_id,
            json!({"event": "draft_noop_cleared", "cleared": cleared}),
        )
        .await;
        return Ok(Json(json!({
            "rich_document_id": document.rich_document_id,
            "draft": null,
            "cleared": cleared,
            "draft_receipt_event_id": receipt,
            "receipt_error": receipt_error,
        })));
    }

    let draft = db
        .upsert_knowledge_rich_document_draft(UpsertKnowledgeRichDocumentDraft {
            rich_document_id: document.rich_document_id.clone(),
            base_doc_version: body.base_doc_version,
            base_content_sha256: body.base_content_sha256,
            content_json: body.content_json,
            actor_kind: ctx.actor_kind.as_str().to_string(),
            actor_id: actor_id_of(&ctx.actor),
            kernel_task_run_id: ctx.kernel_task_run_id.clone(),
            session_run_id: ctx.session_run_id.clone(),
        })
        .await
        .map_err(storage_error)?;

    let (receipt, receipt_error) = record_receipt_non_fatal(
        state.storage.as_ref(),
        &ctx,
        KernelEventType::KnowledgeCrdtRecoveryReceiptRecorded,
        &document.rich_document_id,
        json!({
            "event": "draft_saved",
            "base_doc_version": draft.base_doc_version,
            "draft_content_sha256": draft.draft_content_sha256,
        }),
    )
    .await;

    Ok(Json(json!({
        "rich_document_id": document.rich_document_id,
        "draft": draft,
        "cleared": false,
        "draft_receipt_event_id": receipt,
        "receipt_error": receipt_error,
    })))
}

/// DELETE /knowledge/documents/:document_id/draft — explicit operator discard
/// for a persisted recovery draft (MT-255).
async fn clear_document_draft(
    State(state): State<AppState>,
    Path(document_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let ctx = doc_context(&headers)?;
    ctx.require(DocumentAction::Write)?;
    let db = db_for(&state);

    let document = db
        .get_knowledge_rich_document(&document_id)
        .await
        .map_err(storage_error)?
        .ok_or_else(|| not_found("knowledge rich document"))?;
    let cleared = db
        .clear_knowledge_rich_document_draft(&document.rich_document_id)
        .await
        .map_err(storage_error)?;
    let (receipt, receipt_error) = record_receipt_non_fatal(
        state.storage.as_ref(),
        &ctx,
        KernelEventType::KnowledgeCrdtRecoveryReceiptRecorded,
        &document.rich_document_id,
        json!({"event": "draft_cleared", "cleared": cleared}),
    )
    .await;

    Ok(Json(json!({
        "rich_document_id": document.rich_document_id,
        "cleared": cleared,
        "clear_receipt_event_id": receipt,
        "receipt_error": receipt_error,
    })))
}

/// PUT /knowledge/documents/:document_id/save — optimistic-concurrency save
/// (MT-149). Builds on MT-059 `save_knowledge_rich_document_version`; a stale
/// `expected_version` returns 409. Leaves a save receipt, re-extracts the
/// document's backlinks (MT-155), and syncs the typed embed side table from
/// the content (MT-152).
///
/// Atomicity law (adversarial-v2 MT-149): everything that can REJECT the save
/// (tree validation, embed-target validation, version conflict) runs BEFORE
/// the save commits; everything after the commit (receipt, backlink index,
/// embed sync) is best-effort and RECORDED in the response — a committed save
/// never returns an error.
async fn save_document(
    State(state): State<AppState>,
    Path(document_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<SaveDocumentBody>,
) -> Result<Json<Value>, ApiError> {
    let ctx = doc_context(&headers)?;
    ctx.require(DocumentAction::Write)?;
    let db = db_for(&state);

    // Validate the block tree before promoting (MT-146): a malformed doc is a
    // 400, never a silent bad save. The SAME parsed tree drives the post-save
    // index steps (no second parse with a different schema-version input).
    let tree = BlockTree::from_document_json(
        &document_id,
        crate::knowledge_document::block_tree::DOCUMENT_SCHEMA_VERSION,
        &body.content_json,
    )
    .map_err(|err| bad_request(format!("document block tree is malformed: {err}")))?;
    // MT-152 (adversarial-v2): every embed block in the content must satisfy
    // the typed EmbedTarget law BEFORE the save commits. A javascript:/data:/
    // absolute-path target rejects the whole save fail-closed.
    let validated_embeds = validate_block_embeds(&tree).map_err(|(block_id, err)| {
        bad_request(format!("embed block `{block_id}` target rejected: {err}"))
    })?;
    let document_link_references = DocumentLinkReferences::extract(&tree);
    let receipt_reference_targets: Vec<String> = document_link_references
        .references
        .iter()
        .map(|reference| reference.target.clone())
        .collect();
    let crdt_document_id =
        validated_save_crdt_document_id(&db, &document_id, body.crdt_document_id.as_deref())
            .await?;

    let saved = db
        .save_knowledge_rich_document_version(
            &document_id,
            body.expected_version,
            body.content_json.clone(),
            crdt_document_id.as_deref(),
            body.crdt_snapshot_id.as_deref(),
            body.promotion_receipt_event_id.as_deref(),
        )
        .await
        .map_err(storage_error)?;

    // ---- post-commit (MT-149): nothing below may error a committed save. ----
    // WP-KERNEL-012 MT-120: when (and only when) the caller authenticated a live native-MCP session,
    // stamp the SERVER-DERIVED principal into the receipt payload. This is the anchor the Flight
    // Recorder's `document_saved` receipt-ownership clause compares against. The ledger `actor_id`
    // column deliberately stays the CLIENT-declared per-agent id so two swarm agents saving the same
    // document remain individually attributable; ownership and attribution are different questions
    // and now have different fields. `ctx.actor`, the run ids and the correlation id are untouched —
    // the same clause compares those against the client-supplied Flight Recorder payload.
    let mut receipt_payload = json!({
        "event": "saved",
        "doc_version": saved.doc_version,
        "workspace_id": saved.workspace_id.clone(),
        "content_hash": saved.content_sha256.clone(),
        "reference_targets": receipt_reference_targets,
    });
    if let Some(principal) = ctx.minted_by_principal.as_deref() {
        if let Some(map) = receipt_payload.as_object_mut() {
            map.insert(
                SAVE_RECEIPT_MINTED_BY_PRINCIPAL_FIELD.to_owned(),
                Value::String(principal.to_owned()),
            );
        }
    }
    let (receipt, receipt_error) = record_receipt_non_fatal(
        state.storage.as_ref(),
        &ctx,
        KernelEventType::KnowledgeRichDocumentSaved,
        &saved.rich_document_id,
        receipt_payload,
    )
    .await;

    // MT-155 backlinks + MT-152 embeds: re-extract + persist from the new
    // content (the document content is the source of truth; both rebuilds are
    // idempotent). Index permission is checked, but a denial is non-fatal to
    // the save — it just skips the index step and reports it. A storage
    // failure in either step is RECORDED, never an error for the saved write.
    let mut backlinks_persisted = 0usize;
    let mut backlinks_error: Option<String> = None;
    let mut backlinks_skipped_reason: Option<String> = None;
    let mut embeds_persisted = 0usize;
    let mut embeds_error: Option<String> = None;
    let mut knowledge_indexed = false;
    let mut knowledge_index_error: Option<String> = None;
    match ctx.require(DocumentAction::Index) {
        Ok(()) => {
            let upserts: Vec<UpsertKnowledgeDocumentBacklink> = document_link_references
                .references
                .iter()
                .map(|r| UpsertKnowledgeDocumentBacklink {
                    workspace_id: saved.workspace_id.clone(),
                    relationship_id: r.relationship_id.clone(),
                    source_document_id: saved.rich_document_id.clone(),
                    link_kind: r.kind.as_str().to_string(),
                    target: r.target.clone(),
                    block_id: r.block_id.clone(),
                })
                .collect();
            match db
                .replace_knowledge_document_backlinks(&saved.rich_document_id, upserts)
                .await
            {
                Ok(persisted) => backlinks_persisted = persisted.len(),
                Err(err) => {
                    tracing::error!(
                        target: "handshake_core::knowledge_documents_api",
                        rich_document_id = %saved.rich_document_id,
                        error = %err,
                        "rich_document_backlink_index_failed_post_commit"
                    );
                    backlinks_error = Some(err.to_string());
                }
            }
            match db
                .replace_knowledge_document_embeds(
                    &saved.rich_document_id,
                    embed_upserts(&saved.rich_document_id, &validated_embeds),
                )
                .await
            {
                Ok(persisted) => embeds_persisted = persisted.len(),
                Err(err) => {
                    tracing::error!(
                        target: "handshake_core::knowledge_documents_api",
                        rich_document_id = %saved.rich_document_id,
                        error = %err,
                        "rich_document_embed_sync_failed_post_commit"
                    );
                    embeds_error = Some(err.to_string());
                }
            }
            // MT-154: the document is indexed into the Project Knowledge
            // Index (source row + title entity; staleness on content change).
            (knowledge_indexed, knowledge_index_error) =
                index_document_non_fatal(&db, &saved).await;
        }
        Err(_) => {
            backlinks_skipped_reason = Some(format!("{}_index_denied", ctx.actor_kind.as_str()));
        }
    }

    Ok(Json(json!({
        "document": saved,
        "save_receipt_event_id": receipt,
        "receipt_error": receipt_error,
        "backlinks_persisted": backlinks_persisted,
        "backlinks_error": backlinks_error,
        "backlinks_skipped_reason": backlinks_skipped_reason,
        "embeds_persisted": embeds_persisted,
        "embeds_error": embeds_error,
        "knowledge_indexed": knowledge_indexed,
        "knowledge_index_error": knowledge_index_error,
    })))
}

/// GET /knowledge/documents/:document_id/blocks — the typed block tree only
/// (MT-146/147/148).
async fn load_blocks(
    State(state): State<AppState>,
    Path(document_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let ctx = doc_context(&headers)?;
    ctx.require(DocumentAction::Read)?;
    let db = db_for(&state);

    let document = db
        .get_knowledge_rich_document(&document_id)
        .await
        .map_err(storage_error)?
        .ok_or_else(|| not_found("knowledge rich document"))?;
    let tree = block_tree_view(
        &document.rich_document_id,
        &document.schema_version,
        &document.content_json,
    )?;
    Ok(Json(tree))
}

/// GET /knowledge/documents/:document_id/history — append-only revision
/// history + receipts (MT-156).
///
/// Adversarial-v2 hardening: the list is PAGINATED (`?limit=&offset=`, default
/// 50, cap 200) and returns version METADATA only — no `content_json` bodies
/// (a long history could otherwise balloon the response into a DoS). A single
/// version body is lazily loaded via
/// `GET /knowledge/documents/:document_id/history/:doc_version`.
async fn load_history(
    State(state): State<AppState>,
    Path(document_id): Path<String>,
    Query(params): Query<HistoryParams>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let ctx = doc_context(&headers)?;
    ctx.require(DocumentAction::Read)?;
    let db = db_for(&state);

    let limit = params
        .limit
        .unwrap_or(HISTORY_DEFAULT_LIMIT)
        .clamp(1, HISTORY_MAX_LIMIT);
    let offset = params.offset.unwrap_or(0).max(0);

    let document = db
        .get_knowledge_rich_document(&document_id)
        .await
        .map_err(storage_error)?
        .ok_or_else(|| not_found("knowledge rich document"))?;
    let versions = db
        .list_knowledge_rich_document_version_metas(&document_id, limit, offset)
        .await
        .map_err(storage_error)?;
    let total_versions = db
        .count_knowledge_rich_document_versions(&document_id)
        .await
        .map_err(storage_error)?;

    Ok(Json(json!({
        "rich_document_id": document.rich_document_id,
        "current_version": document.doc_version,
        "authority_label": document.authority_label,
        "owner_actor_kind": document.owner_actor_kind,
        "owner_actor_id": document.owner_actor_id,
        "versions": versions,
        "total_versions": total_versions,
        "limit": limit,
        "offset": offset,
    })))
}

/// GET /knowledge/documents/:document_id/history/:doc_version — ONE revision
/// including its full content body (MT-156 lazy body load).
async fn load_history_version(
    State(state): State<AppState>,
    Path((document_id, doc_version)): Path<(String, i64)>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let ctx = doc_context(&headers)?;
    ctx.require(DocumentAction::Read)?;
    let db = db_for(&state);

    let version = db
        .get_knowledge_rich_document_version(&document_id, doc_version)
        .await
        .map_err(storage_error)?
        .ok_or_else(|| not_found("knowledge rich document version"))?;
    Ok(Json(json!({
        "rich_document_id": document_id,
        "version": version,
    })))
}

/// GET /knowledge/documents/:document_id/projection?format= — render a
/// regenerable projection (MT-150). Rendering NEVER mutates authority.
async fn export_projection(
    State(state): State<AppState>,
    Path(document_id): Path<String>,
    Query(params): Query<ProjectionParams>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let ctx = doc_context(&headers)?;
    ctx.require(DocumentAction::Read)?;
    let db = db_for(&state);

    let format = parse_projection_format(&params.format)?;
    let document = db
        .get_knowledge_rich_document(&document_id)
        .await
        .map_err(storage_error)?
        .ok_or_else(|| not_found("knowledge rich document"))?;
    let tree = BlockTree::from_document_json(
        &document.rich_document_id,
        &document.schema_version,
        &document.content_json,
    )
    .map_err(|err| bad_request(format!("block tree: {err}")))?;
    let rendered = render_projection(&document.title, &tree, format);

    Ok(Json(json!({
        "rich_document_id": document.rich_document_id,
        "projection": rendered,
    })))
}

/// POST /knowledge/documents/import — import a snippet into a new document
/// (MT-151).
async fn import_document(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ImportDocumentBody>,
) -> Result<Json<Value>, ApiError> {
    let ctx = doc_context(&headers)?;
    ctx.require(DocumentAction::Write)?;
    let db = db_for(&state);

    let format = parse_import_format(&body.format)?;
    let outcome = import_snippet(&body.snippet, format);

    let created = db
        .create_knowledge_rich_document(NewKnowledgeRichDocument {
            workspace_id: body.workspace_id,
            document_id: None,
            title: body.title,
            schema_version: crate::knowledge_document::block_tree::DOCUMENT_SCHEMA_VERSION
                .to_string(),
            content_json: outcome.document_json.clone(),
            crdt_document_id: None,
            crdt_snapshot_id: None,
            promotion_receipt_event_id: None,
            project_ref: None,
            folder_ref: None,
            authority_label: Some("promoted".to_string()),
            owner_actor_kind: Some(ctx.actor_kind.as_str().to_string()),
            owner_actor_id: Some(actor_id_of(&ctx.actor)),
        })
        .await
        .map_err(storage_error)?;

    // Post-commit receipt (MT-149): never an error for a committed import.
    let (receipt, receipt_error) = record_receipt_non_fatal(
        state.storage.as_ref(),
        &ctx,
        KernelEventType::KnowledgeRichDocumentSaved,
        &created.rich_document_id,
        json!({"event": "imported", "format": format.as_str(), "warnings": outcome.warnings.len()}),
    )
    .await;

    Ok(Json(json!({
        "document": created,
        "warnings": outcome.warnings,
        "save_receipt_event_id": receipt,
        "receipt_error": receipt_error,
    })))
}

/// GET /knowledge/documents/:document_id/embeds — typed embed refs (MT-152).
async fn list_embeds(
    State(state): State<AppState>,
    Path(document_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let ctx = doc_context(&headers)?;
    ctx.require(DocumentAction::Read)?;
    let db = db_for(&state);
    let embeds = db
        .list_knowledge_document_embeds(&document_id)
        .await
        .map_err(storage_error)?;
    Ok(Json(
        json!({"rich_document_id": document_id, "embeds": embeds}),
    ))
}

/// GET /knowledge/documents/:document_id/embeds/broken — broken-embed repair
/// queue (MT-153).
async fn list_broken_embeds(
    State(state): State<AppState>,
    Path(document_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let ctx = doc_context(&headers)?;
    ctx.require(DocumentAction::Read)?;
    let db = db_for(&state);
    let embeds = db
        .list_knowledge_document_broken_embeds(&document_id)
        .await
        .map_err(storage_error)?;
    let available_actions: Vec<&str> = crate::knowledge_document::embed::EmbedRepairAction::all()
        .iter()
        .map(|a| a.as_str())
        .collect();
    Ok(Json(json!({
        "rich_document_id": document_id,
        "broken_embeds": embeds,
        "available_actions": available_actions,
    })))
}

/// POST /knowledge/documents/embeds/:embed_id/repair — mark broken / repair an
/// embed (MT-153).
async fn repair_embed(
    State(state): State<AppState>,
    Path(embed_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<RepairEmbedBody>,
) -> Result<Json<Value>, ApiError> {
    let ctx = doc_context(&headers)?;
    ctx.require(DocumentAction::Write)?;
    let db = db_for(&state);

    let updated = db
        .set_knowledge_document_embed_repair_state(&embed_id, body.broken_reason.as_deref())
        .await
        .map_err(storage_error)?;
    let (receipt, receipt_error) = record_receipt_non_fatal(
        state.storage.as_ref(),
        &ctx,
        KernelEventType::KnowledgeRichDocumentSaved,
        &updated.rich_document_id,
        json!({
            "event": "embed_repair",
            "embed_id": embed_id,
            "action": body.action,
            "repair_state": updated.repair_state,
        }),
    )
    .await;
    Ok(Json(json!({
        "embed": updated,
        "save_receipt_event_id": receipt,
        "receipt_error": receipt_error,
    })))
}

/// GET /knowledge/documents/:document_id/backlinks — inbound backlinks (MT-155).
async fn list_backlinks(
    State(state): State<AppState>,
    Path(document_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let ctx = doc_context(&headers)?;
    ctx.require(DocumentAction::Read)?;
    let db = db_for(&state);
    let document = db
        .get_knowledge_rich_document(&document_id)
        .await
        .map_err(storage_error)?
        .ok_or_else(|| not_found("knowledge rich document"))?;
    let mut backlinks = db
        .list_knowledge_document_backlinks_to(
            &document.workspace_id,
            "wikilink",
            &document.rich_document_id,
        )
        .await
        .map_err(storage_error)?;
    // Wikilinks authored as `[[Title]]` store their human title as the
    // target, while structured hsLink nodes store the stable document id.
    // Both forms are inbound references to this document.
    let same_title_count = db
        .list_knowledge_rich_documents(&document.workspace_id, None, None)
        .await
        .map_err(storage_error)?
        .into_iter()
        .filter(|candidate| candidate.title == document.title)
        .count();
    if same_title_count == 1 && document.title != document.rich_document_id {
        let title_backlinks = db
            .list_knowledge_document_backlinks_to(
                &document.workspace_id,
                "wikilink",
                &document.title,
            )
            .await
            .map_err(storage_error)?;
        for backlink in title_backlinks {
            if !backlinks
                .iter()
                .any(|existing| existing.relationship_id == backlink.relationship_id)
            {
                backlinks.push(backlink);
            }
        }
    }
    Ok(Json(json!({
        "source_document_id": document_id,
        "backlinks": backlinks,
    })))
}

/// POST /knowledge/documents/:document_id/backlinks — re-extract + persist the
/// document's backlinks (MT-154/155 rebuild).
async fn rebuild_backlinks(
    State(state): State<AppState>,
    Path(document_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let ctx = doc_context(&headers)?;
    ctx.require(DocumentAction::Index)?;
    let db = db_for(&state);

    let document = db
        .get_knowledge_rich_document(&document_id)
        .await
        .map_err(storage_error)?
        .ok_or_else(|| not_found("knowledge rich document"))?;
    let tree = BlockTree::from_document_json(
        &document.rich_document_id,
        &document.schema_version,
        &document.content_json,
    )
    .map_err(|err| bad_request(format!("block tree: {err}")))?;
    let refs = DocumentLinkReferences::extract(&tree);
    let upserts: Vec<UpsertKnowledgeDocumentBacklink> = refs
        .references
        .iter()
        .map(|r| UpsertKnowledgeDocumentBacklink {
            workspace_id: document.workspace_id.clone(),
            relationship_id: r.relationship_id.clone(),
            source_document_id: document.rich_document_id.clone(),
            link_kind: r.kind.as_str().to_string(),
            target: r.target.clone(),
            block_id: r.block_id.clone(),
        })
        .collect();
    let persisted = db
        .replace_knowledge_document_backlinks(&document.rich_document_id, upserts)
        .await
        .map_err(storage_error)?;

    Ok(Json(json!({
        "source_document_id": document.rich_document_id,
        "backlinks": persisted,
        "tags": refs.tags(),
    })))
}

/// POST /knowledge/documents/:document_id/rename — batch-safe rename (MT-157).
async fn rename_document(
    State(state): State<AppState>,
    Path(document_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<RenameBody>,
) -> Result<Json<Value>, ApiError> {
    let ctx = doc_context(&headers)?;
    ctx.require(DocumentAction::Write)?;
    let db = db_for(&state);

    let title = body.title.trim().to_string();
    if title.is_empty() {
        return Err(bad_request("title must be non-empty"));
    }
    let updated = db
        .rename_knowledge_rich_document(&document_id, &title, body.expected_updated_at)
        .await
        .map_err(storage_error)?;
    let (receipt, receipt_error) = record_receipt_non_fatal(
        state.storage.as_ref(),
        &ctx,
        KernelEventType::KnowledgeRichDocumentSaved,
        &updated.rich_document_id,
        json!({"event": "renamed", "title": updated.title}),
    )
    .await;
    // MT-154: a rename refreshes the indexed title entity (non-fatal).
    let mut knowledge_indexed = false;
    let mut knowledge_index_error: Option<String> = None;
    if ctx.require(DocumentAction::Index).is_ok() {
        (knowledge_indexed, knowledge_index_error) = index_document_non_fatal(&db, &updated).await;
    }
    Ok(Json(json!({
        "document": updated,
        "save_receipt_event_id": receipt,
        "receipt_error": receipt_error,
        "knowledge_indexed": knowledge_indexed,
        "knowledge_index_error": knowledge_index_error,
    })))
}

/// POST /knowledge/documents/:document_id/move — batch-safe move to a project /
/// folder (MT-157).
///
/// Adversarial-v2 hardening: absent != explicit null. An absent field leaves
/// that membership UNCHANGED (an empty body is a no-op move, never a silent
/// clear); an explicit `null` clears; a string sets.
async fn move_document(
    State(state): State<AppState>,
    Path(document_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<MoveBody>,
) -> Result<Json<Value>, ApiError> {
    let ctx = doc_context(&headers)?;
    ctx.require(DocumentAction::Write)?;
    let db = db_for(&state);

    // Merge absent fields from the CURRENT membership so they stay unchanged.
    let current = db
        .get_knowledge_rich_document(&document_id)
        .await
        .map_err(storage_error)?
        .ok_or_else(|| not_found("knowledge rich document"))?;
    let project_ref = match &body.project_ref {
        None => current.project_ref.clone(),
        Some(explicit) => explicit.clone(),
    };
    let folder_ref = match &body.folder_ref {
        None => current.folder_ref.clone(),
        Some(explicit) => explicit.clone(),
    };

    let updated = db
        .move_knowledge_rich_document(&document_id, project_ref.as_deref(), folder_ref.as_deref())
        .await
        .map_err(storage_error)?;
    let (receipt, receipt_error) = record_receipt_non_fatal(
        state.storage.as_ref(),
        &ctx,
        KernelEventType::KnowledgeRichDocumentSaved,
        &updated.rich_document_id,
        json!({
            "event": "moved",
            "project_ref": updated.project_ref,
            "folder_ref": updated.folder_ref,
        }),
    )
    .await;
    Ok(Json(json!({
        "document": updated,
        "save_receipt_event_id": receipt,
        "receipt_error": receipt_error,
    })))
}

/// POST /knowledge/documents/batch — batch rename / move / set-authority-label
/// (adversarial-v2 MT-157): a bounded operation list applied per-document with
/// PER-ITEM receipts and partial-failure reporting. One failing item never
/// aborts the batch (each op is an independent metadata write); the response
/// reports every item's outcome so the caller can retry exactly the failures.
async fn batch_documents(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<BatchBody>,
) -> Result<Json<Value>, ApiError> {
    let ctx = doc_context(&headers)?;
    ctx.require(DocumentAction::Write)?;
    let db = db_for(&state);

    if body.operations.is_empty() {
        return Err(bad_request("batch requires at least one operation"));
    }
    if body.operations.len() > BATCH_MAX_OPERATIONS {
        return Err(bad_request(format!(
            "batch is limited to {BATCH_MAX_OPERATIONS} operations per request"
        )));
    }

    let mut results: Vec<Value> = Vec::with_capacity(body.operations.len());
    let mut succeeded = 0usize;
    let mut failed = 0usize;
    for operation in &body.operations {
        let document_id = operation.document_id().to_string();
        let op_name = operation.op_name();
        let outcome: Result<crate::storage::knowledge::KnowledgeRichDocument, StorageError> =
            match operation {
                BatchOperation::Rename { title, .. } => {
                    let title = title.trim().to_string();
                    if title.is_empty() {
                        Err(StorageError::Validation("title must be non-empty"))
                    } else {
                        db.rename_knowledge_rich_document(&document_id, &title, None)
                            .await
                    }
                }
                BatchOperation::Move {
                    project_ref,
                    folder_ref,
                    ..
                } => match db.get_knowledge_rich_document(&document_id).await {
                    Err(err) => Err(err),
                    Ok(None) => Err(StorageError::NotFound("knowledge rich document")),
                    Ok(Some(current)) => {
                        // Same absent != null law as the per-document move.
                        let project = match project_ref {
                            None => current.project_ref.clone(),
                            Some(explicit) => explicit.clone(),
                        };
                        let folder = match folder_ref {
                            None => current.folder_ref.clone(),
                            Some(explicit) => explicit.clone(),
                        };
                        db.move_knowledge_rich_document(
                            &document_id,
                            project.as_deref(),
                            folder.as_deref(),
                        )
                        .await
                    }
                },
                BatchOperation::SetAuthorityLabel {
                    authority_label, ..
                } => {
                    db.set_knowledge_rich_document_authority_label(&document_id, authority_label)
                        .await
                }
            };
        match outcome {
            Ok(updated) => {
                succeeded += 1;
                // Per-item receipt (post-commit, non-fatal per MT-149).
                let (receipt, receipt_error) = record_receipt_non_fatal(
                    state.storage.as_ref(),
                    &ctx,
                    KernelEventType::KnowledgeRichDocumentSaved,
                    &updated.rich_document_id,
                    json!({"event": "batch", "op": op_name}),
                )
                .await;
                results.push(json!({
                    "document_id": document_id,
                    "block_id": updated.block_id,
                    "op": op_name,
                    "ok": true,
                    "save_receipt_event_id": receipt,
                    "receipt_error": receipt_error,
                }));
            }
            Err(err) => {
                failed += 1;
                let (error_kind, detail) = match &err {
                    StorageError::NotFound(what) => ("not_found", what.to_string()),
                    StorageError::Validation(detail) => ("validation", detail.to_string()),
                    StorageError::Conflict(detail) => ("conflict", detail.to_string()),
                    other => ("internal", other.to_string()),
                };
                results.push(json!({
                    "document_id": document_id,
                    "op": op_name,
                    "ok": false,
                    "error": error_kind,
                    "detail": detail,
                }));
            }
        }
    }

    Ok(Json(json!({
        "results": results,
        "succeeded": succeeded,
        "failed": failed,
    })))
}

// ---------------------------------------------------------------------------
// Helpers.
// ---------------------------------------------------------------------------

fn parse_projection_format(value: &str) -> Result<ProjectionFormat, ApiError> {
    Ok(match value {
        "markdown" => ProjectionFormat::Markdown,
        "html" => ProjectionFormat::Html,
        "plain_text" => ProjectionFormat::PlainText,
        "wiki_loom" => ProjectionFormat::WikiLoom,
        "context_bundle" => ProjectionFormat::ContextBundle,
        other => return Err(bad_request(format!("unknown projection format '{other}'"))),
    })
}

fn parse_import_format(value: &str) -> Result<ImportFormat, ApiError> {
    Ok(match value {
        "markdown" => ImportFormat::Markdown,
        "plain_text" => ImportFormat::PlainText,
        "html" => ImportFormat::Html,
        other => return Err(bad_request(format!("unknown import format '{other}'"))),
    })
}

/// Extract the actor id string out of a KernelActor for the document owner.
fn actor_id_of(actor: &KernelActor) -> String {
    match actor {
        KernelActor::Operator(id)
        | KernelActor::System(id)
        | KernelActor::SessionBroker(id)
        | KernelActor::ModelAdapter(id)
        | KernelActor::ToolGate(id)
        | KernelActor::ValidationRunner(id)
        | KernelActor::PromotionGate(id) => id.clone(),
    }
}
