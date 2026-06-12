//! WP-KERNEL-009 RichDocumentCore (MT-145..MT-160): the backend HTTP surface
//! for the RichDocument authority model, wiring the editor to PostgreSQL +
//! EventLedger authority (NO mocks, no SQLite).
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
    KnowledgeStore, NewKnowledgeRichDocument, UpsertKnowledgeDocumentBacklink,
    UpsertKnowledgeDocumentEmbed,
};
use crate::storage::postgres::PostgresDatabase;
use crate::storage::{Database, StorageError};
use crate::AppState;

const HSK_HEADER_ACTOR_KIND: &str = "x-hsk-actor-kind";
const HSK_HEADER_ACTOR_ID: &str = "x-hsk-actor-id";
const HSK_HEADER_KERNEL_TASK_RUN_ID: &str = "x-hsk-kernel-task-run-id";
const HSK_HEADER_SESSION_RUN_ID: &str = "x-hsk-session-run-id";
const HSK_HEADER_CORRELATION_ID: &str = "x-hsk-correlation-id";

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/knowledge/documents", post(create_document))
        .route("/knowledge/documents/import", post(import_document))
        .route("/knowledge/documents/:document_id", get(load_document))
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
        .with_state(state)
}

type ApiError = (StatusCode, Json<Value>);

fn db_for(state: &AppState) -> PostgresDatabase {
    PostgresDatabase::new(state.postgres_pool.clone())
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
}

fn doc_context(headers: &HeaderMap) -> Result<DocContext, ApiError> {
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
    db: &PostgresDatabase,
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
    db: &PostgresDatabase,
    ctx: &DocContext,
    event_type: KernelEventType,
    rich_document_id: &str,
    payload: Value,
) -> Result<String, ApiError> {
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
    let event = builder.build().map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "receipt_build_failed", "detail": err.to_string()})),
        )
    })?;
    let stored = db.append_kernel_event(event).await.map_err(storage_error)?;
    Ok(stored.event_id)
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
}

#[derive(Debug, Deserialize)]
struct MoveBody {
    #[serde(default)]
    project_ref: Option<String>,
    #[serde(default)]
    folder_ref: Option<String>,
}

// ---------------------------------------------------------------------------
// Handlers.
// ---------------------------------------------------------------------------

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

    let created = db
        .create_knowledge_rich_document(NewKnowledgeRichDocument {
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
        })
        .await
        .map_err(storage_error)?;

    // ---- post-commit (MT-149): the create above is committed; the steps
    // below are best-effort and RECORDED, never an error for a committed write.
    let (receipt, receipt_error) = record_receipt_non_fatal(
        &db,
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

    Ok(Json(json!({
        "document": created,
        "save_receipt_event_id": receipt,
        "receipt_error": receipt_error,
        "embeds_persisted": embeds_persisted,
        "embeds_error": embeds_error,
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

    let saved = db
        .save_knowledge_rich_document_version(
            &document_id,
            body.expected_version,
            body.content_json.clone(),
            None,
        )
        .await
        .map_err(storage_error)?;

    // ---- post-commit (MT-149): nothing below may error a committed save. ----
    let (receipt, receipt_error) = record_receipt_non_fatal(
        &db,
        &ctx,
        KernelEventType::KnowledgeRichDocumentSaved,
        &saved.rich_document_id,
        json!({"event": "saved", "doc_version": saved.doc_version}),
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
    match ctx.require(DocumentAction::Index) {
        Ok(()) => {
            let refs = DocumentLinkReferences::extract(&tree);
            let upserts: Vec<UpsertKnowledgeDocumentBacklink> = refs
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
        &db,
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
        &db,
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

/// GET /knowledge/documents/:document_id/backlinks — forward backlinks (MT-155).
async fn list_backlinks(
    State(state): State<AppState>,
    Path(document_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let ctx = doc_context(&headers)?;
    ctx.require(DocumentAction::Read)?;
    let db = db_for(&state);
    let backlinks = db
        .list_knowledge_document_backlinks_from(&document_id)
        .await
        .map_err(storage_error)?;
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
        .rename_knowledge_rich_document(&document_id, &title)
        .await
        .map_err(storage_error)?;
    let (receipt, receipt_error) = record_receipt_non_fatal(
        &db,
        &ctx,
        KernelEventType::KnowledgeRichDocumentSaved,
        &updated.rich_document_id,
        json!({"event": "renamed", "title": updated.title}),
    )
    .await;
    Ok(Json(json!({
        "document": updated,
        "save_receipt_event_id": receipt,
        "receipt_error": receipt_error,
    })))
}

/// POST /knowledge/documents/:document_id/move — batch-safe move to a project /
/// folder (MT-157).
async fn move_document(
    State(state): State<AppState>,
    Path(document_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<MoveBody>,
) -> Result<Json<Value>, ApiError> {
    let ctx = doc_context(&headers)?;
    ctx.require(DocumentAction::Write)?;
    let db = db_for(&state);

    let updated = db
        .move_knowledge_rich_document(
            &document_id,
            body.project_ref.as_deref(),
            body.folder_ref.as_deref(),
        )
        .await
        .map_err(storage_error)?;
    let (receipt, receipt_error) = record_receipt_non_fatal(
        &db,
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
