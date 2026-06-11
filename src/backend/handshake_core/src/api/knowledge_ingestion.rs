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
//! `postgres_pool` — PostgreSQL + EventLedger authority only, no SQLite.
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
//! repo-relative POSIX ([GLOBAL-PORTABILITY], chk_*_path_portable).
//!
//! Routes:
//! * `POST /knowledge/ingestion/roots` — register a root (403 typed denial
//!   with the durable decision id when the allowlist rejects it)
//! * `GET  /knowledge/ingestion/roots?workspace_id=` — list roots
//! * `GET  /knowledge/ingestion/roots/:root_id/sources` — list sources
//! * `POST /knowledge/ingestion/runs` — run an ingestion pass over a root
//! * `GET  /knowledge/ingestion/sources/:source_id/receipts?limit=` —
//!   extraction-attempt receipts, newest first
//! * `GET  /knowledge/ingestion/repairs?workspace_id=&state=&limit=` — repair
//!   queue entries
//! * `POST /knowledge/ingestion/repairs/:repair_id/retry` — budgeted retry

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

use crate::kernel::KernelActor;
use crate::knowledge_ingestion::backpressure::IngestionLimits;
use crate::knowledge_ingestion::engine::{
    FileIngestOutcome, IngestionContext, IngestionEngine, IngestionPassSummary,
    RootRegistrationRequest,
};
use crate::knowledge_ingestion::repair::RepairState;
use crate::knowledge_ingestion::IngestionError;
use crate::storage::knowledge::{KnowledgeRootKind, KnowledgeStore};
use crate::storage::postgres::PostgresDatabase;
use crate::storage::StorageError;
use crate::AppState;

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
    // The shared pool is Arc-backed: this wraps it, it does NOT reconnect.
    IngestionEngine::from_database(Arc::new(PostgresDatabase::new(state.postgres_pool.clone())))
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
        IngestionError::Storage(StorageError::Conflict(detail)) => (
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
    let ctx = mutation_context(&headers)?;
    let root_kind: KnowledgeRootKind = body
        .root_kind
        .parse()
        .map_err(|_| bad_request(format!("invalid root_kind '{}'", body.root_kind)))?;

    let engine = engine_for(&state);
    let (root, decision) = engine
        .register_root(
            &ctx,
            RootRegistrationRequest {
                workspace_id: body.workspace_id,
                display_name: body.display_name,
                root_kind,
                repo_relative_path: body.repo_relative_path,
                file_allowlist_policy: body
                    .file_allowlist_policy
                    .unwrap_or_else(|| json!({"include": ["**/*"], "exclude": []})),
                operator_approved: body.operator_approved,
            },
        )
        .await
        .map_err(ingestion_error)?;

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
    Query(query): Query<ListRootsQuery>,
) -> Result<Json<Value>, ApiError> {
    let engine = engine_for(&state);
    let roots = engine
        .knowledge()
        .list_knowledge_source_roots(&query.workspace_id)
        .await
        .map_err(storage_error)?;
    Ok(Json(json!({"roots": roots})))
}

async fn list_sources(
    State(state): State<AppState>,
    Path(root_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let engine = engine_for(&state);
    let sources = engine
        .knowledge()
        .list_knowledge_sources_for_root(&root_id)
        .await
        .map_err(storage_error)?;
    Ok(Json(json!({"sources": sources})))
}

// ---------------------------------------------------------------------------
// Ingestion runs.
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct TriggerRunBody {
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
    let ctx = mutation_context(&headers)?;
    if body.fs_anchor.trim().is_empty() {
        return Err(bad_request("fs_anchor is required"));
    }
    let engine = engine_for(&state);
    let summary = engine
        .run_ingestion_pass(
            &ctx,
            &body.root_id,
            &PathBuf::from(&body.fs_anchor),
            &body.limits.resolve(),
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
    limit: Option<i64>,
}

async fn list_receipts(
    State(state): State<AppState>,
    Path(source_id): Path<String>,
    Query(query): Query<ListReceiptsQuery>,
) -> Result<Json<Value>, ApiError> {
    let engine = engine_for(&state);
    let limit = query.limit.unwrap_or(50).clamp(1, LIST_CAP);
    let receipts = engine
        .store()
        .list_extraction_receipts(&source_id, limit)
        .await
        .map_err(ingestion_error)?;
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
    Query(query): Query<ListRepairsQuery>,
) -> Result<Json<Value>, ApiError> {
    let repair_state = query
        .state
        .as_deref()
        .map(str::parse::<RepairState>)
        .transpose()
        .map_err(ingestion_error)?;
    let engine = engine_for(&state);
    let limit = query.limit.unwrap_or(100).clamp(1, LIST_CAP);
    let entries = engine
        .store()
        .list_repair_entries(&query.workspace_id, repair_state, limit)
        .await
        .map_err(ingestion_error)?;
    Ok(Json(json!({"repairs": entries})))
}

#[derive(Debug, Deserialize)]
struct RetryRepairBody {
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
    let ctx = mutation_context(&headers)?;
    if body.fs_anchor.trim().is_empty() {
        return Err(bad_request("fs_anchor is required"));
    }
    let engine = engine_for(&state);
    let (entry, outcome) = engine
        .retry_repair(
            &ctx,
            &repair_id,
            &PathBuf::from(&body.fs_anchor),
            &body.limits.resolve(),
        )
        .await
        .map_err(ingestion_error)?;
    Ok(Json(json!({
        "repair": entry,
        "attempt": outcome.as_ref().map(outcome_json),
    })))
}
