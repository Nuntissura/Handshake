//! WP-KERNEL-009 MT-126 MemoryGraphBackendApi: the backend HTTP surface for the
//! MemoryGraphAndClaims group (MT-113..MT-128).
//!
//! Purpose: let a no-context agent navigate the memory graph WITHOUT scraping a
//! generated wiki or relying on chat history — look up a claim and its evidence,
//! review open conflicts (the repair queue), trace a fact's S/P/O + backing
//! claim, walk an entity's graph neighborhood, and pull the visual-debug
//! payload. All reads go through `storage::knowledge` /
//! `storage::knowledge_memory` over the shared `postgres_pool` — PostgreSQL +
//! EventLedger authority only, no SQLite.
//!
//! Backend-navigation receipt law (spec 2.3.13.11): a navigation query is a
//! retrieval action and MUST be attributable. Every endpoint REQUIRES the
//! identity headers (400 otherwise) and appends a
//! `KNOWLEDGE_RETRIEVAL_TRACE_RECORDED` EventLedger receipt carrying the
//! actor/session/correlation identity and the resolved query:
//! * `x-hsk-actor-id` — who navigates
//! * `x-hsk-kernel-task-run-id` — the kernel task run this nav belongs to
//! * `x-hsk-session-run-id` — the session run within that task
//! and optionally `x-hsk-actor-kind`, `x-hsk-correlation-id`.
//!
//! Conventions mirror `api/knowledge_code_nav.rs`: a `routes(state)` builder,
//! handlers over `AppState`, JSON errors with typed `error` codes, reads bounded
//! by `LIST_CAP`.
//!
//! Routes (all read-only; each leaves a retrieval receipt):
//! * `GET /knowledge/memory/claims/:claim_id` — a claim, its evidence span ids,
//!   its conflicts, and the memory fact it backs (if any)
//! * `GET /knowledge/memory/conflicts?workspace_id=&open_only=&limit=` — the
//!   conflict review / repair queue for a workspace
//! * `GET /knowledge/memory/facts/:fact_id` — a fact (S/P/O, label) + its
//!   backing claim id (evidence trace entry point)
//! * `GET /knowledge/memory/entities/:entity_id/neighborhood` — the edges
//!   touching an entity (graph neighborhood), with evidence span ids
//! * `GET /knowledge/memory/visual-debug?workspace_id=&trusted_only=&limit=` —
//!   the MT-127 memory-graph visual-debug payload

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::kernel::{KernelActor, KernelEventType, NewKernelEvent};
use crate::knowledge_memory::visual_debug::build_memory_graph_visual_debug;
use crate::storage::knowledge::KnowledgeStore;
use crate::storage::knowledge_memory::{get_memory_fact, get_memory_fact_by_claim};
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
            "/knowledge/memory/claims/:claim_id",
            get(get_claim_with_evidence),
        )
        .route("/knowledge/memory/conflicts", get(list_conflicts))
        .route("/knowledge/memory/facts/:fact_id", get(get_fact))
        .route(
            "/knowledge/memory/entities/:entity_id/neighborhood",
            get(entity_neighborhood),
        )
        .route("/knowledge/memory/visual-debug", get(visual_debug))
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
                target: "handshake_core::knowledge_memory_api",
                error = %other,
                "memory_graph_api_internal_error"
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "internal_error"})),
            )
        }
    }
}

/// The backend-navigation identity required on every memory-graph query.
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

/// Append the memory-graph navigation retrieval-trace receipt (spec 2.3.13.11).
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
    .aggregate("knowledge_memory_nav", query_kind)
    .source_component("knowledge_memory_api")
    .payload(json!({
        "kind": "memory_graph_query",
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

// ---------------------------------------------------------------------------
// Query params.
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct ConflictsParams {
    workspace_id: String,
    #[serde(default)]
    open_only: Option<bool>,
    #[serde(default)]
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct VisualDebugParams {
    workspace_id: String,
    #[serde(default)]
    trusted_only: Option<bool>,
    #[serde(default)]
    limit: Option<i64>,
}

fn clamp_limit(requested: Option<i64>) -> i64 {
    requested.unwrap_or(LIST_CAP).clamp(1, LIST_CAP)
}

// ---------------------------------------------------------------------------
// Handlers.
// ---------------------------------------------------------------------------

/// GET /knowledge/memory/claims/:claim_id
async fn get_claim_with_evidence(
    State(state): State<AppState>,
    Path(claim_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let ctx = nav_context(&headers)?;
    let db = db_for(&state);

    let claim = db
        .get_knowledge_claim(&claim_id)
        .await
        .map_err(storage_error)?
        .ok_or_else(|| not_found("knowledge claim"))?;
    let span_ids = db
        .list_knowledge_claim_span_ids(&claim_id)
        .await
        .map_err(storage_error)?;
    let conflicts = db
        .list_knowledge_claim_conflicts(&claim_id)
        .await
        .map_err(storage_error)?;
    let fact = get_memory_fact_by_claim(&state.postgres_pool, &claim_id)
        .await
        .map_err(storage_error)?;

    let receipt = record_nav_receipt(
        &db,
        &ctx,
        "claim_with_evidence",
        json!({"claim_id": claim_id}),
    )
    .await?;

    Ok(Json(json!({
        "claim": claim,
        "evidence_span_ids": span_ids,
        "conflicts": conflicts,
        "backing_fact": fact,
        "retrieval_receipt_event_id": receipt,
    })))
}

/// GET /knowledge/memory/conflicts?workspace_id=&open_only=&limit=
async fn list_conflicts(
    State(state): State<AppState>,
    Query(params): Query<ConflictsParams>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let ctx = nav_context(&headers)?;
    let db = db_for(&state);
    let open_only = params.open_only.unwrap_or(true);
    let limit = clamp_limit(params.limit);

    // The conflict review / repair queue: open conflicts for the workspace.
    // Reads knowledge_claim_conflicts joined to claims for workspace scoping.
    let rows = sqlx::query_as::<_, ConflictRow>(
        r#"
        SELECT kcc.conflict_id, kcc.claim_id, kcc.conflicting_claim_id,
               kcc.conflict_reason,
               kcc.resolution_receipt_event_id,
               kcc.detected_at, kcc.resolved_at
        FROM knowledge_claim_conflicts kcc
        JOIN knowledge_claims kc ON kc.claim_id = kcc.claim_id
        WHERE kc.workspace_id = $1
          AND ($2 = false OR kcc.resolved_at IS NULL)
        ORDER BY kcc.detected_at DESC, kcc.conflict_id DESC
        LIMIT $3
        "#,
    )
    .bind(&params.workspace_id)
    .bind(open_only)
    .bind(limit)
    .fetch_all(&state.postgres_pool)
    .await
    .map_err(|err| storage_error(StorageError::from(err)))?;

    let receipt = record_nav_receipt(
        &db,
        &ctx,
        "conflict_review",
        json!({"workspace_id": params.workspace_id, "open_only": open_only}),
    )
    .await?;

    Ok(Json(json!({
        "workspace_id": params.workspace_id,
        "open_only": open_only,
        "conflicts": rows,
        "count": rows.len(),
        "retrieval_receipt_event_id": receipt,
    })))
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
struct ConflictRow {
    conflict_id: String,
    claim_id: String,
    conflicting_claim_id: String,
    conflict_reason: String,
    resolution_receipt_event_id: Option<String>,
    detected_at: chrono::DateTime<chrono::Utc>,
    resolved_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// GET /knowledge/memory/facts/:fact_id
async fn get_fact(
    State(state): State<AppState>,
    Path(fact_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let ctx = nav_context(&headers)?;
    let db = db_for(&state);

    let fact = get_memory_fact(&state.postgres_pool, &fact_id)
        .await
        .map_err(storage_error)?
        .ok_or_else(|| not_found("memory fact"))?;

    let receipt = record_nav_receipt(&db, &ctx, "fact_trace", json!({"fact_id": fact_id})).await?;

    Ok(Json(json!({
        "fact": fact,
        "backing_claim_id": fact.claim_id,
        "retrieval_receipt_event_id": receipt,
    })))
}

/// GET /knowledge/memory/entities/:entity_id/neighborhood
async fn entity_neighborhood(
    State(state): State<AppState>,
    Path(entity_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let ctx = nav_context(&headers)?;
    let db = db_for(&state);

    let edges = db
        .list_knowledge_edges_for_entity(&entity_id)
        .await
        .map_err(storage_error)?;

    // Attach the evidence span ids for each edge (citations).
    let mut edge_views = Vec::with_capacity(edges.len());
    for edge in &edges {
        let span_ids = db
            .list_knowledge_edge_span_ids(&edge.edge_id)
            .await
            .map_err(storage_error)?;
        edge_views.push(json!({
            "edge": edge,
            "evidence_span_ids": span_ids,
        }));
    }

    let receipt = record_nav_receipt(
        &db,
        &ctx,
        "entity_neighborhood",
        json!({"entity_id": entity_id}),
    )
    .await?;

    Ok(Json(json!({
        "entity_id": entity_id,
        "edges": edge_views,
        "count": edge_views.len(),
        "retrieval_receipt_event_id": receipt,
    })))
}

/// GET /knowledge/memory/visual-debug?workspace_id=&trusted_only=&limit=
async fn visual_debug(
    State(state): State<AppState>,
    Query(params): Query<VisualDebugParams>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let ctx = nav_context(&headers)?;
    let db = db_for(&state);
    let trusted_only = params.trusted_only.unwrap_or(false);
    let limit = clamp_limit(params.limit);

    let payload = build_memory_graph_visual_debug(
        &db,
        &state.postgres_pool,
        &params.workspace_id,
        trusted_only,
        limit,
    )
    .await
    .map_err(storage_error)?;

    let receipt = record_nav_receipt(
        &db,
        &ctx,
        "visual_debug",
        json!({"workspace_id": params.workspace_id, "trusted_only": trusted_only}),
    )
    .await?;

    Ok(Json(json!({
        "payload": payload,
        "retrieval_receipt_event_id": receipt,
    })))
}
