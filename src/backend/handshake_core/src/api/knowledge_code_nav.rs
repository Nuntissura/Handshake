//! WP-KERNEL-009 MT-106 CodeNavigationApi: the backend HTTP surface for the
//! CodeIndexingAndNavigation group (MT-097..MT-112).
//!
//! Purpose: let a no-context agent navigate indexed code WITHOUT an external LSP
//! server and without reading product source — look up a symbol by name, jump to
//! its definition span, list its references and callers, find the tests that
//! exercise it, and read the source spans (citations) behind every answer. The
//! Monaco code-lens payload (MT-109) and the bounded context bundle (MT-110) are
//! served here too.
//!
//! Conventions mirror `api/knowledge_ingestion.rs`: a `routes(state)` builder,
//! handlers over the shared `AppState`, JSON errors with typed `error` codes,
//! reads bounded by `LIST_CAP`. All graph reads go through
//! `storage::knowledge::KnowledgeStore` over the shared `postgres_pool` —
//! PostgreSQL + EventLedger authority only, no SQLite, no re-parsing.
//!
//! Backend-navigation receipt law (spec 2.3.13.11): a navigation query is a
//! retrieval action and MUST be attributable. Every nav endpoint therefore
//! REQUIRES the identity headers (400 otherwise) and appends a
//! `KNOWLEDGE_RETRIEVAL_TRACE_RECORDED` EventLedger receipt carrying the
//! actor/session/correlation identity and the resolved query, so who-navigated-
//! to-what is auditable:
//! * `x-hsk-actor-id` — who navigates
//! * `x-hsk-kernel-task-run-id` — the kernel task run this nav belongs to
//! * `x-hsk-session-run-id` — the session run within that task
//!
//! and accepts optionally:
//!
//! * `x-hsk-actor-kind` — operator | system | session_broker | model_adapter |
//!   toolgate | validation_runner | promotion_gate (default `system`)
//! * `x-hsk-correlation-id` — correlation chain id
//!
//! Routes (all read-only graph queries; each leaves a retrieval receipt):
//! * `GET /knowledge/code/symbols?workspace_id=&name=&path=&limit=` — symbol
//!   lookup by simple name and/or file path
//! * `GET /knowledge/code/symbols/:entity_id` — one symbol with definition span,
//!   doc, owning file, and staleness
//! * `GET /knowledge/code/symbols/:entity_id/references` — outgoing references
//!   (callees) + incoming references (callers), with evidence spans
//! * `GET /knowledge/code/symbols/:entity_id/tests` — tests that `validate` this
//!   symbol
//! * `GET /knowledge/code/symbols/:entity_id/spans` — the source spans
//!   (citations) the symbol was detected from
//! * `GET /knowledge/code/files/:path/lens?workspace_id=&content_hash=&parser_version=`
//!   — the Monaco code-lens payload (MT-109) for a file
//!
//! The `:path` segment is a repo-relative POSIX path; `/` is URL-encoded by the
//! caller and decoded here. `owning_wp` is surfaced when the symbol carries a
//! `wp`/`work_packet` provenance hint (the index records what it knows; the WP
//! linkage graph is populated by other groups).

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::kernel::{KernelActor, KernelEventType, NewKernelEvent};
use crate::knowledge_code_index::monaco_bridge::build_monaco_payload;
use crate::storage::knowledge::{
    KnowledgeCodeParseStatus, KnowledgeEdgeType, KnowledgeEntity, KnowledgeEntityKind,
    KnowledgeSpanKind, KnowledgeStore,
};
use crate::storage::postgres::PostgresDatabase;
use crate::storage::{Database, StorageError};
use crate::swarm_orchestration::state_recovery::{
    AgentLaneIdentity, AgentLaneKind, AttributionMode, LocalCloudAttribution,
    ParallelSwarmStateRecoveryStore, QuietBackgroundPolicy, QuietBackgroundWorkKind,
    QuietBackgroundWorkRequest,
};
use crate::AppState;

const HSK_HEADER_ACTOR_KIND: &str = "x-hsk-actor-kind";
const HSK_HEADER_ACTOR_ID: &str = "x-hsk-actor-id";
const HSK_HEADER_KERNEL_TASK_RUN_ID: &str = "x-hsk-kernel-task-run-id";
const HSK_HEADER_SESSION_RUN_ID: &str = "x-hsk-session-run-id";
const HSK_HEADER_CORRELATION_ID: &str = "x-hsk-correlation-id";

/// Cap for list endpoints (matches the ingestion API convention).
const LIST_CAP: i64 = 500;

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/knowledge/code/symbols", get(lookup_symbols))
        .route("/knowledge/code/symbols/:entity_id", get(get_symbol))
        .route(
            "/knowledge/code/symbols/:entity_id/references",
            get(symbol_references),
        )
        .route(
            "/knowledge/code/symbols/:entity_id/tests",
            get(symbol_tests),
        )
        .route(
            "/knowledge/code/symbols/:entity_id/spans",
            get(symbol_spans),
        )
        .route("/knowledge/code/files/:path/lens", get(file_lens))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Shared plumbing.
// ---------------------------------------------------------------------------

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
                target: "handshake_core::knowledge_code_nav",
                error = %other,
                "code_nav_api_internal_error"
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "internal_error"})),
            )
        }
    }
}

/// The backend-navigation identity required on every nav query.
struct NavContext {
    actor: KernelActor,
    kernel_task_run_id: String,
    session_run_id: String,
    correlation_id: Option<String>,
}

/// Build the nav identity from the required headers (400 if any is missing).
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

/// Append the navigation retrieval-trace receipt (spec 2.3.13.11). The receipt
/// carries the actor/session/correlation identity and the resolved query so the
/// navigation is auditable. A receipt failure is surfaced (the nav is not served
/// silently without its trace).
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
    .aggregate("knowledge_code_nav", query_kind)
    .source_component("knowledge_code_nav")
    .payload(json!({
        "kind": "code_nav_query",
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

async fn record_quiet_nav_receipt(
    state: &AppState,
    ctx: &NavContext,
    workspace_id: &str,
    nav_receipt_event_id: &str,
) -> Result<String, ApiError> {
    let actor_token = safe_lane_token(ctx.actor.actor_id());
    let lane = AgentLaneIdentity::new(
        format!("lane-backend-nav-{actor_token}"),
        format!("nav-{actor_token}"),
        AgentLaneKind::System,
        LocalCloudAttribution {
            mode: AttributionMode::System,
            provider: None,
            runtime: Some("backend_navigation_api".to_string()),
            model_label: "backend-navigation".to_string(),
            credential_ref: None,
            provider_metadata: json!({
                "actor_kind": ctx.actor.actor_kind(),
            }),
        },
    )
    .map_err(|err| bad_request(format!("invalid navigation quiet lane: {err}")))?;
    let store =
        ParallelSwarmStateRecoveryStore::new(state.postgres_pool.clone(), state.storage.clone());
    let record = store
        .record_quiet_background_work(QuietBackgroundWorkRequest {
            lane,
            workspace_id: workspace_id.to_string(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: "MT-219".to_string(),
            work_kind: QuietBackgroundWorkKind::BackendNavigation,
            subject_id: nav_receipt_event_id.to_string(),
            session_id: ctx.session_run_id.clone(),
            policy: QuietBackgroundPolicy::quiet_for(QuietBackgroundWorkKind::BackendNavigation),
            evidence_ref: format!("event://{nav_receipt_event_id}"),
        })
        .await
        .map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "quiet_receipt_failed", "detail": err.to_string()})),
            )
        })?;
    Ok(record.receipt_id)
}

// ---------------------------------------------------------------------------
// Query params.
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct LookupParams {
    workspace_id: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    prefix: Option<String>,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct LensParams {
    workspace_id: String,
    content_hash: String,
    parser_version: String,
}

// ---------------------------------------------------------------------------
// Handlers.
// ---------------------------------------------------------------------------

/// MT-106 (spec 2.3.13.11 "mark stale, never serve stale silently"): the
/// served-symbol staleness flag attached to EVERY symbol the nav API returns.
///
/// A symbol nav query has no live editor buffer to hash against (unlike the
/// MT-109 lens route, which is given the buffer's content_hash), so freshness is
/// taken from the PERSISTED code-file index health for the symbol's source:
/// * no code-file row  -> `unindexed` (the symbol predates/escapes the index)
/// * row marked stale  -> `marked_stale` (MT-107/ingestion flagged it)
/// * parse_status≠parsed-> `failed`/`partial` (the file did not fully index)
/// * otherwise         -> `fresh`
///
/// This guarantees a stale/failed symbol is FLAGGED rather than served as
/// authoritative. The flag is intentionally conservative: anything not provably
/// fresh is surfaced as not-fresh so a consumer never mistakes it for current.
async fn served_staleness(db: &PostgresDatabase, source_id: Option<&str>) -> Value {
    let Some(source_id) = source_id else {
        return json!({"state": "unindexed", "fresh": false,
            "detail": "symbol has no primary source"});
    };
    match db.get_knowledge_code_file_by_source(source_id).await {
        Ok(Some(code_file)) => {
            if code_file.stale {
                json!({"state": "marked_stale", "fresh": false,
                    "indexed_content_hash": code_file.indexed_content_hash,
                    "indexed_parser_version": code_file.parser_version})
            } else if code_file.parse_status != KnowledgeCodeParseStatus::Parsed {
                json!({"state": code_file.parse_status.as_str(), "fresh": false,
                    "detail": "source did not fully index"})
            } else {
                json!({"state": "fresh", "fresh": true,
                    "indexed_content_hash": code_file.indexed_content_hash,
                    "indexed_parser_version": code_file.parser_version})
            }
        }
        // No code-file row: the symbol is not backed by a current index pass.
        Ok(None) => json!({"state": "unindexed", "fresh": false,
            "detail": "no code-file index state for source"}),
        // A staleness lookup failure must not be served as "fresh": fail closed.
        Err(_) => json!({"state": "unknown", "fresh": false,
            "detail": "staleness lookup failed"}),
    }
}

/// The JSON projection of a symbol entity + its definition span + staleness.
async fn symbol_to_json(db: &PostgresDatabase, symbol: &KnowledgeEntity) -> Value {
    let symbol_kind = symbol
        .detection_provenance
        .get("symbol_kind")
        .and_then(|v| v.as_str())
        .unwrap_or("symbol");
    let owning_wp = symbol
        .detection_provenance
        .get("wp")
        .or_else(|| symbol.detection_provenance.get("work_packet"))
        .and_then(|v| v.as_str());
    // Definition = the first `ast` span.
    let mut definition = Value::Null;
    if let Ok(span_ids) = db.list_knowledge_entity_span_ids(&symbol.entity_id).await {
        for span_id in span_ids {
            if let Ok(Some(span)) = db.get_knowledge_span(&span_id).await {
                if matches!(span.span_kind, KnowledgeSpanKind::Ast) {
                    definition = json!({
                        "span_id": span.span_id,
                        "source_id": span.source_id,
                        "line_start": span.line_start,
                        "line_end": span.line_end,
                        "range_start": span.range_start,
                        "range_end": span.range_end,
                        "section_path": span.section_path,
                    });
                    break;
                }
            }
        }
    }
    let staleness = served_staleness(db, symbol.primary_source_id.as_deref()).await;
    json!({
        "symbol_entity_id": symbol.entity_id,
        "symbol_key": symbol.entity_key,
        "display_name": symbol.display_name,
        "symbol_kind": symbol_kind,
        "owning_wp": owning_wp,
        "primary_source_id": symbol.primary_source_id,
        "lifecycle_state": symbol.lifecycle_state.as_str(),
        "definition": definition,
        "staleness": staleness,
    })
}

/// `GET /knowledge/code/symbols` — symbol lookup by name and/or path.
async fn lookup_symbols(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<LookupParams>,
) -> Result<Json<Value>, ApiError> {
    let db = db_for(&state);
    let ctx = nav_context(&headers)?;
    let name = params
        .name
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty());
    let prefix = params
        .prefix
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty());
    let path = params
        .path
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty());
    if name.is_none() && prefix.is_none() && path.is_none() {
        return Err(bad_request(
            "at least one of name, prefix, or path is required",
        ));
    }
    let limit = params.limit.unwrap_or(LIST_CAP).clamp(1, LIST_CAP);

    // DB-side path/name pushdown (MT-106 DoS fix): SQL cuts the candidate set to
    // the matching path segment / name suffix instead of loading every symbol in
    // the workspace. We over-fetch (limit*4, capped) because the SQL name-suffix
    // LIKE (`%name`) is broader than the exact simple-name match, then refine to
    // exactness in Rust on the small candidate set.
    let candidates = db
        .lookup_code_symbols(
            &params.workspace_id,
            name,
            path,
            prefix,
            (limit.saturating_mul(4)).clamp(limit, 10_000),
        )
        .await
        .map_err(storage_error)?;

    let mut matched: Vec<&KnowledgeEntity> = candidates
        .iter()
        .filter(|s| {
            let name_ok = match name {
                Some(n) => symbol_simple_name(&s.entity_key) == n || s.display_name == n,
                None => true,
            };
            let path_ok = match path {
                Some(p) => symbol_path_segment(&s.entity_key) == p,
                None => true,
            };
            let prefix_ok = match prefix {
                Some(p) => {
                    let prefix = p.to_ascii_lowercase();
                    symbol_simple_name(&s.entity_key)
                        .to_ascii_lowercase()
                        .starts_with(&prefix)
                        || s.display_name.to_ascii_lowercase().starts_with(&prefix)
                }
                None => true,
            };
            name_ok && path_ok && prefix_ok
        })
        .collect();
    matched.sort_by(|a, b| a.entity_key.cmp(&b.entity_key));
    matched.truncate(limit as usize);

    let mut results = Vec::with_capacity(matched.len());
    for symbol in &matched {
        results.push(symbol_to_json(&db, symbol).await);
    }

    let receipt = record_nav_receipt(
        &db,
        &ctx,
        "symbol_lookup",
        json!({"workspace_id": params.workspace_id, "name": name, "prefix": prefix, "path": path, "matches": results.len()}),
    )
    .await?;
    let quiet_receipt =
        record_quiet_nav_receipt(&state, &ctx, &params.workspace_id, &receipt).await?;

    Ok(Json(json!({
        "workspace_id": params.workspace_id,
        "matches": results,
        "nav_receipt_event_id": receipt,
        "quiet_background_work_receipt_id": quiet_receipt,
    })))
}

/// `GET /knowledge/code/symbols/:entity_id` — one symbol with its definition.
async fn get_symbol(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(entity_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let db = db_for(&state);
    let ctx = nav_context(&headers)?;
    let symbol = require_symbol(&db, &entity_id).await?;
    let body = symbol_to_json(&db, &symbol).await;
    let receipt =
        record_nav_receipt(&db, &ctx, "symbol_get", json!({"entity_id": entity_id})).await?;
    let quiet_receipt =
        record_quiet_nav_receipt(&state, &ctx, &symbol.workspace_id, &receipt).await?;
    Ok(Json(
        json!({"symbol": body, "nav_receipt_event_id": receipt, "quiet_background_work_receipt_id": quiet_receipt}),
    ))
}

/// `GET /knowledge/code/symbols/:entity_id/references` — callers + callees.
async fn symbol_references(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(entity_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let db = db_for(&state);
    let ctx = nav_context(&headers)?;
    let symbol = require_symbol(&db, &entity_id).await?;
    let edges = db
        .list_knowledge_edges_for_entity(&symbol.entity_id)
        .await
        .map_err(storage_error)?;

    let mut callers = Vec::new();
    let mut callees = Vec::new();
    for edge in &edges {
        if edge.edge_type != KnowledgeEdgeType::References {
            continue;
        }
        let spans = edge_span_refs(&db, &edge.edge_id).await;
        if edge.target_entity_id == symbol.entity_id {
            // Incoming reference: source calls this symbol.
            if let Ok(Some(src)) = db.get_knowledge_entity(&edge.source_entity_id).await {
                let staleness = served_staleness(&db, src.primary_source_id.as_deref()).await;
                callers.push(json!({
                    "symbol_entity_id": src.entity_id,
                    "symbol_key": src.entity_key,
                    "display_name": src.display_name,
                    "confidence": edge.confidence,
                    "evidence_spans": spans,
                    "staleness": staleness,
                }));
            }
        } else if edge.source_entity_id == symbol.entity_id {
            // Outgoing reference: this symbol calls target.
            if let Ok(Some(tgt)) = db.get_knowledge_entity(&edge.target_entity_id).await {
                let staleness = served_staleness(&db, tgt.primary_source_id.as_deref()).await;
                callees.push(json!({
                    "symbol_entity_id": tgt.entity_id,
                    "symbol_key": tgt.entity_key,
                    "display_name": tgt.display_name,
                    "confidence": edge.confidence,
                    "evidence_spans": spans,
                    "staleness": staleness,
                }));
            }
        }
    }

    // The queried symbol's own staleness is surfaced alongside its relations.
    let self_staleness = served_staleness(&db, symbol.primary_source_id.as_deref()).await;
    let receipt = record_nav_receipt(
        &db,
        &ctx,
        "symbol_references",
        json!({"entity_id": entity_id, "callers": callers.len(), "callees": callees.len()}),
    )
    .await?;
    let quiet_receipt =
        record_quiet_nav_receipt(&state, &ctx, &symbol.workspace_id, &receipt).await?;

    Ok(Json(json!({
        "symbol_entity_id": symbol.entity_id,
        "staleness": self_staleness,
        "callers": callers,
        "callees": callees,
        "nav_receipt_event_id": receipt,
        "quiet_background_work_receipt_id": quiet_receipt,
    })))
}

/// `GET /knowledge/code/symbols/:entity_id/tests` — tests validating a symbol.
async fn symbol_tests(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(entity_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let db = db_for(&state);
    let ctx = nav_context(&headers)?;
    let symbol = require_symbol(&db, &entity_id).await?;
    let edges = db
        .list_knowledge_edges_for_entity(&symbol.entity_id)
        .await
        .map_err(storage_error)?;

    let mut tests = Vec::new();
    for edge in &edges {
        // A `validates` edge points test -> tested symbol; we want edges whose
        // TARGET is this symbol (the tests that exercise it).
        if edge.edge_type == KnowledgeEdgeType::Validates
            && edge.target_entity_id == symbol.entity_id
        {
            let spans = edge_span_refs(&db, &edge.edge_id).await;
            if let Ok(Some(test)) = db.get_knowledge_entity(&edge.source_entity_id).await {
                let staleness = served_staleness(&db, test.primary_source_id.as_deref()).await;
                tests.push(json!({
                    "test_entity_id": test.entity_id,
                    "test_symbol_key": test.entity_key,
                    "display_name": test.display_name,
                    "confidence": edge.confidence,
                    "evidence_spans": spans,
                    "staleness": staleness,
                }));
            }
        }
    }

    let self_staleness = served_staleness(&db, symbol.primary_source_id.as_deref()).await;
    let receipt = record_nav_receipt(
        &db,
        &ctx,
        "symbol_tests",
        json!({"entity_id": entity_id, "tests": tests.len()}),
    )
    .await?;
    let quiet_receipt =
        record_quiet_nav_receipt(&state, &ctx, &symbol.workspace_id, &receipt).await?;

    Ok(Json(json!({
        "symbol_entity_id": symbol.entity_id,
        "staleness": self_staleness,
        "tests": tests,
        "nav_receipt_event_id": receipt,
        "quiet_background_work_receipt_id": quiet_receipt,
    })))
}

/// `GET /knowledge/code/symbols/:entity_id/spans` — citation spans of a symbol.
async fn symbol_spans(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(entity_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let db = db_for(&state);
    let ctx = nav_context(&headers)?;
    let symbol = require_symbol(&db, &entity_id).await?;
    let span_ids = db
        .list_knowledge_entity_span_ids(&symbol.entity_id)
        .await
        .map_err(storage_error)?;
    let mut spans = Vec::new();
    for span_id in span_ids {
        if let Some(span) = db
            .get_knowledge_span(&span_id)
            .await
            .map_err(storage_error)?
        {
            spans.push(json!({
                "span_id": span.span_id,
                "source_id": span.source_id,
                "span_kind": span.span_kind.as_str(),
                "line_start": span.line_start,
                "line_end": span.line_end,
                "range_start": span.range_start,
                "range_end": span.range_end,
                "section_path": span.section_path,
                "content_sha256": span.content_sha256,
                "parser_version": span.parser_version,
            }));
        }
    }

    let self_staleness = served_staleness(&db, symbol.primary_source_id.as_deref()).await;
    let receipt = record_nav_receipt(
        &db,
        &ctx,
        "symbol_spans",
        json!({"entity_id": entity_id, "spans": spans.len()}),
    )
    .await?;
    let quiet_receipt =
        record_quiet_nav_receipt(&state, &ctx, &symbol.workspace_id, &receipt).await?;

    Ok(Json(json!({
        "symbol_entity_id": symbol.entity_id,
        "staleness": self_staleness,
        "spans": spans,
        "nav_receipt_event_id": receipt,
        "quiet_background_work_receipt_id": quiet_receipt,
    })))
}

/// `GET /knowledge/code/files/:path/lens` — Monaco code-lens payload (MT-109).
async fn file_lens(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(path): Path<String>,
    Query(params): Query<LensParams>,
) -> Result<Json<Value>, ApiError> {
    let db = db_for(&state);
    let ctx = nav_context(&headers)?;
    let relative_path = decode_path(&path);
    // Reject path traversal / absolute paths on the lookup key. The path only
    // feeds a parameterised DB lookup today (not the filesystem), so this is
    // defence in depth, but an indexed file is always a repo-relative POSIX path
    // with no `..`/`.` segment, so anything else is a guaranteed miss anyway.
    if !is_safe_relative_path(&relative_path) {
        return Err(bad_request(
            "path must be a repo-relative POSIX path with no '..'/'.' segments",
        ));
    }
    let payload = build_monaco_payload(
        &db,
        &params.workspace_id,
        &relative_path,
        &params.content_hash,
        &params.parser_version,
    )
    .await
    .map_err(code_index_error)?;

    let receipt = record_nav_receipt(
        &db,
        &ctx,
        "file_lens",
        json!({"workspace_id": params.workspace_id, "relative_path": relative_path, "entries": payload.entries.len()}),
    )
    .await?;
    let quiet_receipt =
        record_quiet_nav_receipt(&state, &ctx, &params.workspace_id, &receipt).await?;

    let mut body = serde_json::to_value(&payload).map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "serialize_failed", "detail": err.to_string()})),
        )
    })?;
    if let Value::Object(map) = &mut body {
        map.insert("nav_receipt_event_id".to_string(), json!(receipt));
        map.insert(
            "quiet_background_work_receipt_id".to_string(),
            json!(quiet_receipt),
        );
    }
    Ok(Json(body))
}

// ---------------------------------------------------------------------------
// Helpers.
// ---------------------------------------------------------------------------

/// Resolve an entity id, 404 if missing, 400 if it is not a code symbol.
async fn require_symbol(
    db: &PostgresDatabase,
    entity_id: &str,
) -> Result<KnowledgeEntity, ApiError> {
    let entity = db
        .get_knowledge_entity(entity_id)
        .await
        .map_err(storage_error)?
        .ok_or_else(|| not_found(format!("symbol '{entity_id}' not found")))?;
    if entity.entity_kind != KnowledgeEntityKind::Symbol {
        return Err(bad_request(format!(
            "entity '{entity_id}' is not a code symbol (kind {})",
            entity.entity_kind.as_str()
        )));
    }
    Ok(entity)
}

/// The evidence span refs of an edge as JSON (id + line range), best-effort.
async fn edge_span_refs(db: &PostgresDatabase, edge_id: &str) -> Vec<Value> {
    let mut out = Vec::new();
    if let Ok(span_ids) = db.list_knowledge_edge_span_ids(edge_id).await {
        for span_id in span_ids {
            if let Ok(Some(span)) = db.get_knowledge_span(&span_id).await {
                out.push(json!({
                    "span_id": span.span_id,
                    "line_start": span.line_start,
                    "line_end": span.line_end,
                }));
            }
        }
    }
    out
}

/// The simple name of a symbol key `{lang}:{path}#{symbol_path}[~{disc}]` (last
/// segment of the symbol_path after the final `.`/`::`). The optional `~{disc}`
/// collision discriminator (MT-098/099/100) is stripped first so the name match
/// is unaffected by it.
fn symbol_simple_name(entity_key: &str) -> &str {
    let after_hash = entity_key.rsplit('#').next().unwrap_or(entity_key);
    // Drop the collision discriminator suffix (`~as:Trait`, `~class`, `~dup1`).
    let after_hash = after_hash.split('~').next().unwrap_or(after_hash);
    after_hash
        .rsplit("::")
        .next()
        .unwrap_or(after_hash)
        .rsplit('.')
        .next()
        .unwrap_or(after_hash)
}

/// The file path segment of a symbol key `{lang}:{path}#{symbol_path}`.
fn symbol_path_segment(entity_key: &str) -> &str {
    let before_hash = entity_key.split('#').next().unwrap_or(entity_key);
    before_hash
        .split_once(':')
        .map(|x| x.1)
        .unwrap_or(before_hash)
}

/// Decode a URL-encoded repo-relative path segment.
fn decode_path(raw: &str) -> String {
    // Axum already percent-decodes a single path segment; this also accepts a
    // caller that encoded `/` as `%2F` by replacing the literal back.
    raw.replace("%2F", "/").replace("%2f", "/")
}

/// True when `path` is a repo-relative POSIX path with no traversal: no leading
/// `/`, no backslash, and no `.`/`..` segment. Indexed file keys always satisfy
/// this, so a path that does not is rejected rather than silently missing.
fn is_safe_relative_path(path: &str) -> bool {
    if path.is_empty() || path.starts_with('/') || path.contains('\\') {
        return false;
    }
    path.split('/')
        .all(|seg| !seg.is_empty() && seg != "." && seg != "..")
}

fn safe_lane_token(value: &str) -> String {
    let mut token: String = value
        .chars()
        .filter_map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | ':' | '.' | '/' | '#') {
                Some(c)
            } else if c.is_whitespace() {
                Some('_')
            } else {
                None
            }
        })
        .take(96)
        .collect();
    if token.is_empty() {
        token = "unknown".to_string();
    }
    token
}

/// Map a code-index error to HTTP.
fn code_index_error(err: crate::knowledge_code_index::CodeIndexError) -> ApiError {
    use crate::knowledge_code_index::CodeIndexError;
    match err {
        CodeIndexError::Validation(detail) => bad_request(detail),
        CodeIndexError::Storage(storage) => storage_error(storage),
        other => {
            tracing::error!(
                target: "handshake_core::knowledge_code_nav",
                error = %other,
                "code_nav_api_code_index_error"
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "internal_error"})),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_name_extraction() {
        assert_eq!(symbol_simple_name("rust:src/lib.rs#Foo::bar"), "bar");
        assert_eq!(
            symbol_simple_name("typescript:app/x.ts#useThing"),
            "useThing"
        );
        assert_eq!(symbol_simple_name("rust:src/lib.rs#alpha"), "alpha");
        assert_eq!(symbol_simple_name("rust:a.rs#A.b.c"), "c");
    }

    #[test]
    fn path_segment_extraction() {
        assert_eq!(symbol_path_segment("rust:src/lib.rs#Foo"), "src/lib.rs");
        assert_eq!(symbol_path_segment("typescript:app/x.ts#y"), "app/x.ts");
    }

    #[test]
    fn decode_path_handles_encoded_slash() {
        assert_eq!(decode_path("src%2Flib.rs"), "src/lib.rs");
        assert_eq!(decode_path("src/lib.rs"), "src/lib.rs");
    }
}
