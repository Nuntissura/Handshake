//! WP-KERNEL-009 MT-095 SourceIngestionApi: the backend HTTP surface for the
//! SourceIngestionAndEvidence group (MT-081..MT-096).
//!
//! Purpose: let a no-context agent drive and inspect source ingestion without
//! reading product source — register roots (allowlist-enforced), list roots
//! and sources, trigger an ingestion pass, read extraction receipts, and work
//! the repair queue (list + retry).
//!
//! Conventions mirror `api/atelier.rs`: a `routes(state)` builder, handlers
//! over the shared `AppState`, JSON errors with typed `error` codes. The
//! ingestion engine writes through `storage::knowledge::KnowledgeStore` and
//! `knowledge_ingestion::store::KnowledgeIngestionStore` over the shared
//! shared storage handle — single-store + EventLedger authority only, no SQLite.
//!
//! The ingestion store and `KnowledgeStore` share the application's embedded
//! SurrealDB handle, so API reads, ingestion writes, and EventLedger receipts
//! all use the same durable authority.
//!
//! Backend-navigation law (spec 2.3.13.11): every MUTATION must carry actor,
//! session, and correlation identity into its EventLedger receipts. Mutating
//! routes therefore REQUIRE these headers (400 otherwise):
//! * `x-hsk-actor-id` — who acts (operator name, model session id, ...)
//! * `x-hsk-kernel-task-run-id` — the kernel task run this action belongs to
//! * `x-hsk-session-run-id` — the session run within that task
//!
//! and accept optionally:
//!
//! * `x-hsk-actor-kind` — operator | system | session_broker | model_adapter
//!   | toolgate | validation_runner | promotion_gate (default `system`)
//! * `x-hsk-correlation-id` — correlation chain id
//!
//! Filesystem anchoring: ingestion runs and repair retries need the
//! machine-local checkout root (`fs_anchor`) as REQUEST input. It is runtime
//! configuration, used for this one walk and never stored — stored paths stay
//! repo-relative POSIX ([GLOBAL-PORTABILITY], chk_*_path_portable). MT-154: the
//! anchor is only read after the root's workspace is authorized, must be an
//! absolute existing directory, and the registered root must resolve (after
//! symlink resolution) inside the canonical anchor.
//!
//! MT-154 authority (Master Spec 02-system-architecture.md:2758/2773/2776,
//! LM-RLS-001/002): every route names its workspace (`workspace_id` in the
//! body or query), authorizes it through the ResourceBroker BEFORE any table
//! or filesystem access (Read+memory.read for reads, Create+memory.propose for
//! writes, matching the MT-120 ingestion table predicates), and runs every
//! read/write as the account's record user with receipts attributed to the
//! session principal. Every failure, including a silently dropped record-user
//! write, is the constant denial.
//!
//! Routes:
//! * `POST /knowledge/ingestion/roots` — register a root (403 typed denial
//!   with the durable decision id when the allowlist rejects it)
//! * `GET  /knowledge/ingestion/roots?workspace_id=` — list roots
//! * `GET  /knowledge/ingestion/roots/:root_id/sources?workspace_id=` — list sources
//! * `POST /knowledge/ingestion/runs` — run an ingestion pass over a root
//!   (`{workspace_id, root_id, fs_anchor}`)
//! * `GET  /knowledge/ingestion/sources/:source_id/receipts?workspace_id=&limit=` —
//!   extraction-attempt receipts, newest first
//! * `GET  /knowledge/ingestion/repairs?workspace_id=&state=&limit=` — repair
//!   queue entries
//! * `POST /knowledge/ingestion/repairs/:repair_id/retry` — budgeted retry
//!   (`{workspace_id, fs_anchor}`)

use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

use super::knowledge_crdt::{is_record_user_denial, KnowledgeAccount};
use crate::kernel::KernelActor;
use crate::knowledge_ingestion::backpressure::IngestionLimits;
use crate::knowledge_ingestion::engine::{
    FileIngestOutcome, IngestionContext, IngestionEngine, IngestionPassSummary,
    RootRegistrationRequest,
};
use crate::knowledge_ingestion::repair::RepairState;
use crate::knowledge_ingestion::IngestionError;
use crate::storage::knowledge::{KnowledgeRootKind, KnowledgeStore};
use crate::storage::surreal::resource_authority::ResourceAction;
use crate::storage::surreal::SurrealDatabase;
use crate::storage::StorageError;
use crate::AppState;

/// Capability of every ingestion read (MT-120 `knowledge_*` select predicates).
const INGESTION_READ: &str = "memory.read";
/// Capability of every ingestion write (MT-120 create predicates + `fn::mt120_index_receipt`).
const INGESTION_WRITE: &str = "memory.propose";

const HSK_HEADER_ACTOR_KIND: &str = "x-hsk-actor-kind";
const HSK_HEADER_ACTOR_ID: &str = "x-hsk-actor-id";
const HSK_HEADER_KERNEL_TASK_RUN_ID: &str = "x-hsk-kernel-task-run-id";
const HSK_HEADER_SESSION_RUN_ID: &str = "x-hsk-session-run-id";
const HSK_HEADER_CORRELATION_ID: &str = "x-hsk-correlation-id";

/// Cap for list endpoints (matches the atelier API convention of bounded
/// reads; callers page by lowering `limit`).
const LIST_CAP: i64 = 500;

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route(
            "/knowledge/ingestion/roots",
            get(list_roots).post(register_root),
        )
        .route(
            "/knowledge/ingestion/roots/:root_id/sources",
            get(list_sources),
        )
        .route("/knowledge/ingestion/runs", post(trigger_run))
        .route(
            "/knowledge/ingestion/sources/:source_id/receipts",
            get(list_receipts),
        )
        .route("/knowledge/ingestion/repairs", get(list_repairs))
        .route(
            "/knowledge/ingestion/repairs/:repair_id/retry",
            post(retry_repair),
        )
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Shared plumbing.
// ---------------------------------------------------------------------------

type ApiError = (StatusCode, Json<Value>);

fn engine_for(state: &AppState) -> IngestionEngine {
    IngestionEngine::from_database(Arc::new(SurrealDatabase::new(state.surreal.clone())))
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

/// Build the backend-navigation context from the required mutation headers, attributed to the
/// authenticated session: the actor is the session principal and the session run is the
/// authenticated session (`fn::mt120_index_receipt` requires both), never a header value.
fn account_mutation_context(
    headers: &HeaderMap,
    account: &KnowledgeAccount,
) -> Result<IngestionContext, ApiError> {
    let mut ctx = mutation_context(headers)?;
    ctx.actor = account.session_actor();
    ctx.session_run_id = account.session_id().to_owned();
    Ok(ctx)
}

/// MT-154: the caller-supplied checkout anchor must be an absolute, existing directory, and the
/// registered root (repo-relative) must resolve — after symlink resolution — inside it. Returns the
/// verified anchor the engine walks. No product-defined workspace filesystem root exists, so the
/// anchor stays runtime input of an account authorized for the root's workspace.
fn authorized_fs_anchor(fs_anchor: &str, repo_relative_path: &str) -> Result<PathBuf, ApiError> {
    let raw = PathBuf::from(fs_anchor.trim());
    if fs_anchor.trim().is_empty() || !raw.is_absolute() {
        return Err(bad_request("fs_anchor must be an absolute directory"));
    }
    let anchor = std::fs::canonicalize(&raw)
        .map_err(|_| bad_request("fs_anchor must be an existing directory"))?;
    if !anchor.is_dir() {
        return Err(bad_request("fs_anchor must be an existing directory"));
    }
    if !repo_relative_path.is_empty() {
        // Resolve through the non-verbatim path (the repo-relative path uses `/` separators).
        if let Ok(root_dir) = std::fs::canonicalize(raw.join(repo_relative_path)) {
            if !root_dir.starts_with(&anchor) {
                return Err(crate::api::authority::constant_denial());
            }
        }
    }
    // The engine walks the caller's (verified) spelling: a canonical Windows verbatim path would
    // stop `/`-separated repo-relative joins from resolving.
    Ok(raw)
}

/// Build the backend-navigation context from required mutation headers.
fn mutation_context(headers: &HeaderMap) -> Result<IngestionContext, ApiError> {
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

    let actor = match header_str(headers, HSK_HEADER_ACTOR_KIND).unwrap_or("system") {
        "operator" => KernelActor::Operator(actor_id),
        "system" => KernelActor::System(actor_id),
        "session_broker" => KernelActor::SessionBroker(actor_id),
        "model_adapter" => KernelActor::ModelAdapter(actor_id),
        "toolgate" => KernelActor::ToolGate(actor_id),
        "validation_runner" => KernelActor::ValidationRunner(actor_id),
        "promotion_gate" => KernelActor::PromotionGate(actor_id),
        other => {
            return Err(bad_request(format!(
                "unknown {HSK_HEADER_ACTOR_KIND} '{other}'"
            )))
        }
    };

    Ok(IngestionContext {
        actor,
        kernel_task_run_id,
        session_run_id,
        correlation_id: header_str(headers, HSK_HEADER_CORRELATION_ID).map(ToOwned::to_owned),
    })
}

/// Map a typed ingestion error to HTTP. Policy denials are 403 WITH the
/// durable decision id so the caller can replay the verdict.
fn ingestion_error(err: IngestionError) -> ApiError {
    if is_record_user_denial(&err.to_string()) {
        return crate::api::authority::constant_denial();
    }
    match err {
        IngestionError::PolicyDenied {
            verdict,
            candidate_path,
            matched_pattern,
            decision_id,
        } => (
            StatusCode::FORBIDDEN,
            Json(json!({
                "error": "policy_denied",
                "verdict": verdict.as_str(),
                "candidate_path": candidate_path,
                "matched_pattern": matched_pattern,
                "decision_id": decision_id,
            })),
        ),
        IngestionError::Validation(detail) => bad_request(detail),
        IngestionError::Io { path, detail } => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "io_error", "path": path, "detail": detail})),
        ),
        IngestionError::Storage(StorageError::NotFound(what)) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "not_found", "detail": what})),
        ),
        IngestionError::Storage(
            StorageError::Conflict(detail) | StorageError::ConflictDetails { code: detail, .. },
        ) => (
            StatusCode::CONFLICT,
            Json(json!({"error": "conflict", "detail": detail})),
        ),
        IngestionError::Storage(StorageError::Validation(detail)) => bad_request(detail),
        other => {
            tracing::error!(
                target: "handshake_core::knowledge_ingestion",
                error = %other,
                "ingestion_api_internal_error"
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "internal_error"})),
            )
        }
    }
}

fn storage_error(err: StorageError) -> ApiError {
    ingestion_error(IngestionError::Storage(err))
}

/// Request-supplied limit overrides onto the compiled defaults (MT-092).
#[derive(Clone, Copy, Debug, Default, Deserialize)]
struct LimitOverrides {
    max_bytes: Option<u64>,
    max_pdf_bytes: Option<u64>,
    max_lines: Option<u64>,
}

impl LimitOverrides {
    fn resolve(self) -> IngestionLimits {
        let defaults = IngestionLimits::default();
        IngestionLimits {
            max_bytes: self.max_bytes.unwrap_or(defaults.max_bytes),
            max_pdf_bytes: self.max_pdf_bytes.unwrap_or(defaults.max_pdf_bytes),
            max_lines: self.max_lines.unwrap_or(defaults.max_lines),
        }
    }
}

/// Compact per-file view of a [`FileIngestOutcome`].
fn outcome_json(outcome: &FileIngestOutcome) -> Value {
    json!({
        "source_id": outcome.source.source_id,
        "relative_path": outcome.source.relative_path,
        "status": outcome.receipt.status.as_str(),
        "error_class": outcome.receipt.error_class.map(|c| c.as_str()),
        "receipt_id": outcome.receipt.receipt_id,
        "receipt_event_id": outcome.receipt.receipt_event_id,
        "spans_produced": outcome.receipt.spans_produced,
        "spans_failed": outcome.receipt.spans_failed,
        "redaction_count": outcome.receipt.redaction_count,
        "redaction_state": outcome.source.redaction_state.as_str(),
        "repair_id": outcome.repair.as_ref().map(|r| r.repair_id.clone()),
    })
}

fn summary_json(summary: &IngestionPassSummary) -> Value {
    json!({
        "run_token": summary.run_token,
        "root_id": summary.root_id,
        "workspace_id": summary.workspace_id,
        "start_event_id": summary.start_event_id,
        "finish_event_id": summary.finish_event_id,
        "outcomes": summary.outcomes.iter().map(outcome_json).collect::<Vec<_>>(),
        "stale_marked": summary
            .stale_marked
            .iter()
            .map(|mark| json!({
                "source_id": mark.source_id,
                "relative_path": mark.relative_path,
                "disposition": mark.disposition,
                "moved_to": mark.moved_to,
                "event_id": mark.event_id,
            }))
            .collect::<Vec<_>>(),
        "skipped_by_allowlist": summary.skipped_by_allowlist,
        "invalid_paths": summary.invalid_paths,
        "walk_errors": summary.walk_errors,
    })
}

// ---------------------------------------------------------------------------
// Roots.
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct RegisterRootBody {
    workspace_id: String,
    display_name: String,
    /// `project_repo | governance | artifacts | media_library |
    /// external_import | operator_folder`.
    root_kind: String,
    repo_relative_path: String,
    /// Per-root FILE allowlist (`{"include": [...], "exclude": [...]}`);
    /// defaults to include-everything.
    #[serde(default)]
    file_allowlist_policy: Option<Value>,
    #[serde(default)]
    operator_approved: bool,
}

async fn register_root(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<RegisterRootBody>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let account = KnowledgeAccount::workspace(
        &state,
        &headers,
        &body.workspace_id,
        ResourceAction::Create,
        INGESTION_WRITE,
    )
    .await?;
    let ctx = account_mutation_context(&headers, &account)?;
    let root_kind: KnowledgeRootKind = body
        .root_kind
        .parse()
        .map_err(|_| bad_request(format!("invalid root_kind '{}'", body.root_kind)))?;

    let engine = engine_for(&state);
    // Roots are created with `created_in_session_id` by the scoped root upsert
    // (storage::surreal::knowledge::owned_root_upsert_rows), so record users can read them back.
    let (root, decision) = account
        .run(
            &state,
            engine.register_root(
                &ctx,
                RootRegistrationRequest {
                    workspace_id: body.workspace_id.clone(),
                    display_name: body.display_name,
                    root_kind,
                    repo_relative_path: body.repo_relative_path,
                    file_allowlist_policy: body
                        .file_allowlist_policy
                        .unwrap_or_else(|| json!({"include": ["**/*"], "exclude": []})),
                    operator_approved: body.operator_approved,
                },
            ),
        )
        .await
        .map_err(ingestion_error)?;
    if root.workspace_id != body.workspace_id {
        return Err(crate::api::authority::constant_denial());
    }

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "root": root,
            "decision": {
                "decision_id": decision.decision_id,
                "verdict": decision.verdict.as_str(),
                "matched_pattern": decision.matched_pattern,
                "receipt_event_id": decision.receipt_event_id,
            },
        })),
    ))
}

#[derive(Debug, Deserialize)]
struct ListRootsQuery {
    workspace_id: String,
}

async fn list_roots(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ListRootsQuery>,
) -> Result<Json<Value>, ApiError> {
    let account = KnowledgeAccount::workspace(
        &state,
        &headers,
        &query.workspace_id,
        ResourceAction::Read,
        INGESTION_READ,
    )
    .await?;
    let engine = engine_for(&state);
    let roots = account
        .run(
            &state,
            engine
                .knowledge()
                .list_knowledge_source_roots(&query.workspace_id),
        )
        .await
        .map_err(storage_error)?;
    Ok(Json(json!({"roots": roots})))
}

#[derive(Debug, Deserialize)]
struct WorkspaceQuery {
    workspace_id: String,
}

/// Reads `root_id` as the record user and requires it to belong to the authorized workspace; an
/// invisible or foreign root is a 404 (existence is not disclosed across workspaces).
async fn authorized_root(
    state: &AppState,
    account: &KnowledgeAccount,
    engine: &IngestionEngine,
    root_id: &str,
) -> Result<crate::storage::knowledge::KnowledgeSourceRoot, ApiError> {
    account
        .run(state, engine.knowledge().get_knowledge_source_root(root_id))
        .await
        .map_err(storage_error)?
        .filter(|root| root.workspace_id == account.workspace_id)
        .ok_or_else(|| storage_error(StorageError::NotFound("knowledge source root")))
}

async fn list_sources(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(root_id): Path<String>,
    Query(query): Query<WorkspaceQuery>,
) -> Result<Json<Value>, ApiError> {
    let account = KnowledgeAccount::workspace(
        &state,
        &headers,
        &query.workspace_id,
        ResourceAction::Read,
        INGESTION_READ,
    )
    .await?;
    let engine = engine_for(&state);
    authorized_root(&state, &account, &engine, &root_id).await?;
    let sources = account
        .run(
            &state,
            engine.knowledge().list_knowledge_sources_for_root(&root_id),
        )
        .await
        .map_err(storage_error)?
        .into_iter()
        .filter(|source| source.workspace_id == query.workspace_id)
        .collect::<Vec<_>>();
    Ok(Json(json!({"sources": sources})))
}

// ---------------------------------------------------------------------------
// Ingestion runs.
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct TriggerRunBody {
    /// The workspace the root belongs to (authorized before any access).
    workspace_id: String,
    root_id: String,
    /// Machine-local checkout root the registered repo-relative root path is
    /// resolved against. Runtime input for THIS run only — never stored.
    fs_anchor: String,
    #[serde(default)]
    limits: LimitOverrides,
}

async fn trigger_run(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<TriggerRunBody>,
) -> Result<Json<Value>, ApiError> {
    let account = KnowledgeAccount::workspace(
        &state,
        &headers,
        &body.workspace_id,
        ResourceAction::Create,
        INGESTION_WRITE,
    )
    .await?;
    let ctx = account_mutation_context(&headers, &account)?;
    if body.fs_anchor.trim().is_empty() {
        return Err(bad_request("fs_anchor is required"));
    }
    let engine = engine_for(&state);
    let root = authorized_root(&state, &account, &engine, &body.root_id).await?;
    let anchor = authorized_fs_anchor(&body.fs_anchor, &root.repo_relative_path)?;
    let summary = account
        .run(
            &state,
            engine.run_ingestion_pass(&ctx, &root.root_id, &anchor, &body.limits.resolve()),
        )
        .await
        .map_err(ingestion_error)?;
    Ok(Json(summary_json(&summary)))
}

// ---------------------------------------------------------------------------
// Receipts.
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct ListReceiptsQuery {
    workspace_id: String,
    limit: Option<i64>,
}

async fn list_receipts(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(source_id): Path<String>,
    Query(query): Query<ListReceiptsQuery>,
) -> Result<Json<Value>, ApiError> {
    let account = KnowledgeAccount::workspace(
        &state,
        &headers,
        &query.workspace_id,
        ResourceAction::Read,
        INGESTION_READ,
    )
    .await?;
    let engine = engine_for(&state);
    let limit = query.limit.unwrap_or(50).clamp(1, LIST_CAP);
    account
        .run(&state, engine.knowledge().get_knowledge_source(&source_id))
        .await
        .map_err(storage_error)?
        .filter(|source| source.workspace_id == query.workspace_id)
        .ok_or_else(|| storage_error(StorageError::NotFound("knowledge source")))?;
    let receipts = account
        .run(
            &state,
            engine.store().list_extraction_receipts(&source_id, limit),
        )
        .await
        .map_err(ingestion_error)?
        .into_iter()
        .filter(|receipt| receipt.workspace_id == query.workspace_id)
        .collect::<Vec<_>>();
    Ok(Json(json!({"receipts": receipts})))
}

// ---------------------------------------------------------------------------
// Repair queue.
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct ListRepairsQuery {
    workspace_id: String,
    /// `queued | retrying | resolved | dead_letter`; omitted = all states.
    state: Option<String>,
    limit: Option<i64>,
}

async fn list_repairs(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ListRepairsQuery>,
) -> Result<Json<Value>, ApiError> {
    let account = KnowledgeAccount::workspace(
        &state,
        &headers,
        &query.workspace_id,
        ResourceAction::Read,
        INGESTION_READ,
    )
    .await?;
    let repair_state = query
        .state
        .as_deref()
        .map(str::parse::<RepairState>)
        .transpose()
        .map_err(ingestion_error)?;
    let engine = engine_for(&state);
    let limit = query.limit.unwrap_or(100).clamp(1, LIST_CAP);
    let entries = account
        .run(
            &state,
            engine
                .store()
                .list_repair_entries(&query.workspace_id, repair_state, limit),
        )
        .await
        .map_err(ingestion_error)?;
    Ok(Json(json!({"repairs": entries})))
}

#[derive(Debug, Deserialize)]
struct RetryRepairBody {
    /// The workspace the repair entry belongs to (authorized before any access).
    workspace_id: String,
    /// Machine-local checkout root (see [`TriggerRunBody::fs_anchor`]).
    fs_anchor: String,
    #[serde(default)]
    limits: LimitOverrides,
}

async fn retry_repair(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(repair_id): Path<String>,
    Json(body): Json<RetryRepairBody>,
) -> Result<Json<Value>, ApiError> {
    let account = KnowledgeAccount::workspace(
        &state,
        &headers,
        &body.workspace_id,
        ResourceAction::Create,
        INGESTION_WRITE,
    )
    .await?;
    let ctx = account_mutation_context(&headers, &account)?;
    if body.fs_anchor.trim().is_empty() {
        return Err(bad_request("fs_anchor is required"));
    }
    let engine = engine_for(&state);
    // The entry must be visible to the record user AND belong to the authorized workspace before
    // the retry claims an attempt or touches the filesystem.
    let entry = account
        .run(&state, engine.store().get_repair_entry(&repair_id))
        .await
        .map_err(ingestion_error)?
        .filter(|entry| entry.workspace_id == body.workspace_id)
        .ok_or_else(|| storage_error(StorageError::NotFound("knowledge ingestion repair entry")))?;
    let source = account
        .run(
            &state,
            engine.knowledge().get_knowledge_source(&entry.source_id),
        )
        .await
        .map_err(storage_error)?
        .filter(|source| source.workspace_id == body.workspace_id)
        .ok_or_else(|| {
            storage_error(StorageError::NotFound("knowledge source for repair entry"))
        })?;
    let root_id = source
        .root_id
        .clone()
        .ok_or_else(|| bad_request("repair source has no root"))?;
    let root = authorized_root(&state, &account, &engine, &root_id).await?;
    let anchor = authorized_fs_anchor(&body.fs_anchor, &root.repo_relative_path)?;
    let (entry, outcome) = account
        .run(
            &state,
            engine.retry_repair(&ctx, &repair_id, &anchor, &body.limits.resolve()),
        )
        .await
        .map_err(ingestion_error)?;
    Ok(Json(json!({
        "repair": entry,
        "attempt": outcome.as_ref().map(outcome_json),
    })))
}
