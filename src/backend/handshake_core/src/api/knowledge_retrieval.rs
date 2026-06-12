//! WP-KERNEL-009 MT-143 RetrievalDebugApi: the backend HTTP surface for the
//! RetrievalContextAndRanking group (MT-129..MT-144).
//!
//! Purpose (spec 2.3.13.11): let a no-context agent EXPLAIN and REPRODUCE a
//! retrieval WITHOUT scraping a generated wiki or relying on chat history —
//! pull a bundle and its replayable QueryPlan + RetrievalTrace, see why a mode
//! was chosen (and why broader retrieval was skipped), list the bundle's cited
//! items + which were dropped and why, get a bounded AI-ready export manifest,
//! and read the active SemanticCatalog routing contracts. All reads go through
//! `storage/knowledge` + `storage/knowledge_retrieval` over the shared
//! `postgres_pool` — PostgreSQL + EventLedger authority only, no SQLite.
//!
//! Backend-navigation receipt law (spec 2.3.13.11): a navigation query is a
//! retrieval action and MUST be attributable. Every endpoint REQUIRES the
//! identity headers (400 otherwise) and appends a
//! `KNOWLEDGE_RETRIEVAL_TRACE_RECORDED` EventLedger receipt carrying the
//! actor/session/correlation identity and the resolved query. Conventions
//! mirror `api/knowledge_memory.rs`.
//!
//! Routes (all read-only; each leaves a retrieval receipt):
//! * `GET /knowledge/retrieval/bundles/:bundle_id` — a bundle, its items
//!   (with retrieval decisions + citations), and its replayable traces
//!   (QueryPlan + RetrievalTrace in `decisions`) — the explanation + reproduction
//!   surface.
//! * `GET /knowledge/retrieval/bundles/:bundle_id/export` — the bounded AI-ready
//!   evidence export manifest for the bundle (provenance + retention +
//!   reconstructable).
//! * `GET /knowledge/retrieval/catalog?workspace_id=&limit=` — the active
//!   SemanticCatalog routing contracts.

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::kernel::{KernelActor, KernelEventType, NewKernelEvent};
use crate::knowledge_retrieval::ai_ready_export::build_evidence_manifest;
use crate::storage::knowledge::KnowledgeStore;
use crate::storage::knowledge_retrieval::list_semantic_catalog_entries;
use crate::storage::postgres::PostgresDatabase;
use crate::storage::{Database, StorageError};
use crate::AppState;

const HSK_HEADER_ACTOR_KIND: &str = "x-hsk-actor-kind";
const HSK_HEADER_ACTOR_ID: &str = "x-hsk-actor-id";
const HSK_HEADER_KERNEL_TASK_RUN_ID: &str = "x-hsk-kernel-task-run-id";
const HSK_HEADER_SESSION_RUN_ID: &str = "x-hsk-session-run-id";
const HSK_HEADER_CORRELATION_ID: &str = "x-hsk-correlation-id";

/// Bound on list reads so a single nav query cannot pull an unbounded result.
const LIST_CAP: i64 = 500;

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route(
            "/knowledge/retrieval/bundles/:bundle_id",
            get(explain_bundle),
        )
        .route(
            "/knowledge/retrieval/bundles/:bundle_id/export",
            get(export_bundle_evidence),
        )
        .route("/knowledge/retrieval/catalog", get(list_catalog))
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

fn storage_error(err: StorageError) -> ApiError {
    match err {
        StorageError::NotFound(what) => not_found(what),
        StorageError::Validation(detail) => bad_request(detail),
        other => {
            tracing::error!(
                target: "handshake_core::knowledge_retrieval_api",
                error = %other,
                "retrieval_debug_api_internal_error"
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "internal_error"})),
            )
        }
    }
}

/// The backend-navigation identity required on every retrieval-debug query.
struct NavContext {
    actor: KernelActor,
    kernel_task_run_id: String,
    session_run_id: String,
    correlation_id: Option<String>,
}

fn nav_context(headers: &HeaderMap) -> Result<NavContext, ApiError> {
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
    Ok(NavContext {
        actor,
        kernel_task_run_id,
        session_run_id,
        correlation_id: header_str(headers, HSK_HEADER_CORRELATION_ID).map(ToOwned::to_owned),
    })
}

/// Append the retrieval-debug navigation receipt (spec 2.3.13.11).
async fn record_nav_receipt(
    db: &PostgresDatabase,
    ctx: &NavContext,
    query_kind: &str,
    query: Value,
) -> Result<String, ApiError> {
    let mut builder = NewKernelEvent::builder(
        ctx.kernel_task_run_id.clone(),
        ctx.session_run_id.clone(),
        KernelEventType::KnowledgeRetrievalTraceRecorded,
        ctx.actor.clone(),
    )
    .aggregate("knowledge_retrieval_nav", query_kind)
    .source_component("knowledge_retrieval_api")
    .payload(json!({
        "kind": "retrieval_debug_query",
        "query_kind": query_kind,
        "query": query,
    }));
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

#[derive(Debug, Deserialize)]
struct CatalogParams {
    workspace_id: String,
    #[serde(default)]
    limit: Option<i64>,
}

fn clamp_limit(requested: Option<i64>) -> i64 {
    requested.unwrap_or(LIST_CAP).clamp(1, LIST_CAP)
}

/// GET /knowledge/retrieval/bundles/:bundle_id
///
/// The explanation + reproduction surface: the bundle (bounded allowed_context),
/// its items with per-item retrieval decisions + citations, and the replayable
/// traces. The trace `decisions` JSONB embeds the full QueryPlan + RetrievalTrace
/// (mode, non_hybrid_reason, candidates, selected) — everything needed to see
/// why a mode was chosen and to reproduce the run.
async fn explain_bundle(
    State(state): State<AppState>,
    Path(bundle_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let ctx = nav_context(&headers)?;
    let db = db_for(&state);

    let (bundle, items) = db
        .get_knowledge_context_bundle(&bundle_id)
        .await
        .map_err(storage_error)?
        .ok_or_else(|| not_found("knowledge context bundle"))?;
    let traces = db
        .list_knowledge_retrieval_traces_for_bundle(&bundle_id)
        .await
        .map_err(storage_error)?;

    let receipt =
        record_nav_receipt(&db, &ctx, "explain_bundle", json!({"bundle_id": bundle_id})).await?;

    Ok(Json(json!({
        "bundle": bundle,
        "items": items,
        "traces": traces,
        "retrieval_receipt_event_id": receipt,
    })))
}

/// GET /knowledge/retrieval/bundles/:bundle_id/export
///
/// The bounded AI-ready evidence export manifest for the bundle (provenance,
/// retention, reconstructability) — reuses the canonical AI-ready export dialect
/// (MT-141) so retrieval evidence speaks one export contract.
async fn export_bundle_evidence(
    State(state): State<AppState>,
    Path(bundle_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let ctx = nav_context(&headers)?;
    let db = db_for(&state);

    let (bundle, _items) = db
        .get_knowledge_context_bundle(&bundle_id)
        .await
        .map_err(storage_error)?
        .ok_or_else(|| not_found("knowledge context bundle"))?;
    let traces = db
        .list_knowledge_retrieval_traces_for_bundle(&bundle_id)
        .await
        .map_err(storage_error)?;

    let manifest = build_evidence_manifest(&bundle, &traces);

    let receipt = record_nav_receipt(
        &db,
        &ctx,
        "export_bundle_evidence",
        json!({"bundle_id": bundle_id}),
    )
    .await?;

    Ok(Json(json!({
        "manifest": manifest,
        "retrieval_receipt_event_id": receipt,
    })))
}

/// GET /knowledge/retrieval/catalog?workspace_id=&limit=
///
/// The active SemanticCatalog routing contracts (MT-140) — backend-queryable
/// routing, not prompt-only helper text.
async fn list_catalog(
    State(state): State<AppState>,
    Query(params): Query<CatalogParams>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let ctx = nav_context(&headers)?;
    let db = db_for(&state);
    let limit = clamp_limit(params.limit);

    let entries = list_semantic_catalog_entries(&state.postgres_pool, &params.workspace_id, limit)
        .await
        .map_err(storage_error)?;

    let receipt = record_nav_receipt(
        &db,
        &ctx,
        "list_catalog",
        json!({"workspace_id": params.workspace_id}),
    )
    .await?;

    Ok(Json(json!({
        "workspace_id": params.workspace_id,
        "entries": entries,
        "count": entries.len(),
        "retrieval_receipt_event_id": receipt,
    })))
}
