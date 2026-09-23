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
//! shared storage handle — single-store + EventLedger authority only, no SQLite.
//!
//! Both stores are backed by the embedded SurrealDB authority.
//!
//! Backend-navigation receipt law (spec 2.3.13.11): a navigation query is a
//! retrieval action and MUST be attributable. Every endpoint REQUIRES the
//! identity headers (400 otherwise) and appends a
//! `KNOWLEDGE_RETRIEVAL_TRACE_RECORDED` EventLedger receipt carrying the
//! actor/session/correlation identity and the resolved query. Conventions
//! mirror `api/knowledge_memory.rs`.
//!
//! MT-154 authority (Master Spec 02-system-architecture.md:2758/2773/2776,
//! LM-RLS-001/002): every route names its workspace (`workspace_id` query),
//! authorizes it through the ResourceBroker BEFORE any table access
//! (Read+fs.read for reads, Create+fs.write for the repair action), runs every
//! read/write as the account's record user, and appends its receipt inside that
//! scope attributed to the session principal (never a header actor). A bundle
//! outside the authorized workspace is a 404; every authority failure,
//! including a silently dropped record-user write, is the constant denial.
//!
//! Routes (each leaves a retrieval receipt):
//! * `GET /knowledge/retrieval/bundles/:bundle_id?workspace_id=` — a bundle, its items
//!   (with retrieval decisions + citations), and its replayable traces
//!   (QueryPlan + RetrievalTrace in `decisions`) — the explanation + reproduction
//!   surface.
//! * `GET /knowledge/retrieval/bundles/:bundle_id/export?workspace_id=` — the bounded AI-ready
//!   evidence export manifest for the bundle (provenance + retention +
//!   reconstructable).
//! * `GET /knowledge/retrieval/bundles/:bundle_id/staleness?workspace_id=` — the EXPLICIT
//!   stale-reason / missing-evidence surface (adversarial-v2 MT-143): every
//!   bundle item's backing record is re-checked against the live index
//!   (missing span/passage/source/entity, stale source) and reported per item.
//! * `POST /knowledge/retrieval/bundles/:bundle_id/repair?workspace_id=` — the repair action
//!   (adversarial-v2 MT-143): re-executes the bundle's recorded query through
//!   the executed retrieval pipeline, producing a FRESH bundle + trace linked
//!   to the stale one.
//! * `GET /knowledge/retrieval/catalog?workspace_id=&limit=` — the active
//!   SemanticCatalog routing contracts.

use std::collections::BTreeSet;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

use super::knowledge_crdt::{is_record_user_denial, KnowledgeAccount};
use crate::kernel::{KernelActor, KernelEventType, NewKernelEvent};
use crate::knowledge_retrieval::ai_ready_export::build_evidence_manifest;
use crate::knowledge_retrieval::compiler::BundleTargetKind;
use crate::knowledge_retrieval::executor::execute_retrieval;
use crate::knowledge_retrieval::graph_planner::GraphTraversalPolicy;
use crate::knowledge_retrieval::planner::RetrievalRequest;
use crate::storage::knowledge::{
    KnowledgeBundleItemRefKind, KnowledgeContextBundle, KnowledgeContextBundleItem, KnowledgeStore,
};
use crate::storage::knowledge_retrieval::list_semantic_catalog_entries;
use crate::storage::surreal::resource_authority::ResourceAction;
use crate::storage::surreal::SurrealDatabase;
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
        .route(
            "/knowledge/retrieval/bundles/:bundle_id/staleness",
            get(bundle_staleness),
        )
        .route(
            "/knowledge/retrieval/bundles/:bundle_id/repair",
            post(repair_bundle),
        )
        .route("/knowledge/retrieval/catalog", get(list_catalog))
        .with_state(state)
}

type ApiError = (StatusCode, Json<Value>);

/// Build the knowledge facade over the single embedded SurrealDB authority.
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

fn not_found(detail: impl Into<String>) -> ApiError {
    (
        StatusCode::NOT_FOUND,
        Json(json!({"error": "not_found", "detail": detail.into()})),
    )
}

fn storage_error(err: StorageError) -> ApiError {
    if is_record_user_denial(&err.to_string()) {
        return crate::api::authority::constant_denial();
    }
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

//// Authorize the named workspace BEFORE any table access (Read+fs.read for reads, Create+fs.write
/// for the repair action), then build the navigation identity attributed to the session principal
/// (the header actor is ignored).
async fn retrieval_account(
    state: &AppState,
    headers: &HeaderMap,
    workspace_id: &str,
    write: bool,
) -> Result<(KnowledgeAccount, NavContext), ApiError> {
    let (action, capability) = if write {
        (ResourceAction::Create, "fs.write")
    } else {
        (ResourceAction::Read, "fs.read")
    };
    let account =
        KnowledgeAccount::workspace(state, headers, workspace_id, action, capability).await?;
    let mut ctx = nav_context(headers)?;
    ctx.actor = account.session_actor();
    Ok((account, ctx))
}

#[derive(Debug, Deserialize)]
struct WorkspaceParams {
    workspace_id: String,
}

/// Append the retrieval-debug navigation receipt (spec 2.3.13.11). Must run inside
/// [`KnowledgeAccount::run`]: the receipt is a record-user write bound to the authorized workspace
/// (`fn::mt154_knowledge_event`); a dropped write is the constant denial.
async fn record_nav_receipt(
    db: &dyn Database,
    ctx: &NavContext,
    workspace_id: &str,
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
        "workspace_id": workspace_id,
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

/// Loads a bundle as the record user and requires it to belong to the authorized workspace; an
/// invisible or foreign bundle is a 404.
async fn authorized_bundle(
    db: &SurrealDatabase,
    workspace_id: &str,
    bundle_id: &str,
) -> Result<(KnowledgeContextBundle, Vec<KnowledgeContextBundleItem>), ApiError> {
    db.get_knowledge_context_bundle(bundle_id)
        .await
        .map_err(storage_error)?
        .filter(|(bundle, _)| bundle.workspace_id == workspace_id)
        .ok_or_else(|| not_found("knowledge context bundle"))
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

/// GET /knowledge/retrieval/bundles/:bundle_id?workspace_id=
///
/// The explanation + reproduction surface: the bundle (bounded allowed_context),
/// its items with per-item retrieval decisions + citations, and the replayable
/// traces. The trace `decisions` JSONB embeds the full QueryPlan + RetrievalTrace
/// (mode, non_hybrid_reason, candidates, selected) — everything needed to see
/// why a mode was chosen and to reproduce the run.
async fn explain_bundle(
    State(state): State<AppState>,
    Path(bundle_id): Path<String>,
    Query(params): Query<WorkspaceParams>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let (account, ctx) = retrieval_account(&state, &headers, &params.workspace_id, false).await?;
    let db = db_for(&state);

    account
        .run(&state, async {
            let (bundle, items) = authorized_bundle(&db, &params.workspace_id, &bundle_id).await?;
            let traces = db
                .list_knowledge_retrieval_traces_for_bundle(&bundle_id)
                .await
                .map_err(storage_error)?;

            let receipt = record_nav_receipt(
                state.storage.as_ref(),
                &ctx,
                &params.workspace_id,
                "explain_bundle",
                json!({"bundle_id": bundle_id}),
            )
            .await?;

            Ok::<_, ApiError>(Json(json!({
                "bundle": bundle,
                "items": items,
                "traces": traces,
                "retrieval_receipt_event_id": receipt,
            })))
        })
        .await
}

/// GET /knowledge/retrieval/bundles/:bundle_id/export?workspace_id=
///
/// The bounded AI-ready evidence export manifest for the bundle (provenance,
/// retention, reconstructability) — reuses the canonical AI-ready export dialect
/// (MT-141) so retrieval evidence speaks one export contract.
async fn export_bundle_evidence(
    State(state): State<AppState>,
    Path(bundle_id): Path<String>,
    Query(params): Query<WorkspaceParams>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let (account, ctx) = retrieval_account(&state, &headers, &params.workspace_id, false).await?;
    let db = db_for(&state);

    account
        .run(&state, async {
            let (bundle, _items) = authorized_bundle(&db, &params.workspace_id, &bundle_id).await?;
            let traces = db
                .list_knowledge_retrieval_traces_for_bundle(&bundle_id)
                .await
                .map_err(storage_error)?;

            let manifest = build_evidence_manifest(&bundle, &traces);

            let receipt = record_nav_receipt(
                state.storage.as_ref(),
                &ctx,
                &params.workspace_id,
                "export_bundle_evidence",
                json!({"bundle_id": bundle_id}),
            )
            .await?;

            Ok::<_, ApiError>(Json(json!({
                "manifest": manifest,
                "retrieval_receipt_event_id": receipt,
            })))
        })
        .await
}

/// GET /knowledge/retrieval/bundles/:bundle_id/staleness?workspace_id=
///
/// The EXPLICIT stale-reason / missing-evidence surface (adversarial-v2
/// MT-143): every bundle item's backing record is re-checked against the live
/// index. Statuses per item: `ok`, `missing_evidence` (the cited record no
/// longer exists or is not readable by this account), `source_stale` (the
/// cited span's source changed since indexing). The bundle is `stale` when any
/// item is not `ok`.
async fn bundle_staleness(
    State(state): State<AppState>,
    Path(bundle_id): Path<String>,
    Query(params): Query<WorkspaceParams>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let (account, ctx) = retrieval_account(&state, &headers, &params.workspace_id, false).await?;
    let db = db_for(&state);

    account
        .run(&state, async {
            let (_bundle, items) = authorized_bundle(&db, &params.workspace_id, &bundle_id).await?;
            let (stale, item_reports) =
                bundle_item_reports(&db, &params.workspace_id, &items).await?;

            let receipt = record_nav_receipt(
                state.storage.as_ref(),
                &ctx,
                &params.workspace_id,
                "bundle_staleness",
                json!({"bundle_id": bundle_id}),
            )
            .await?;

            Ok::<_, ApiError>(Json(json!({
                "bundle_id": bundle_id,
                "stale": stale,
                "items": item_reports,
                "repair_available": true,
                "retrieval_receipt_event_id": receipt,
            })))
        })
        .await
}

/// Per-item staleness of a bundle, read as the record user. A cited record outside the authorized
/// workspace is reported as missing evidence (it is not this workspace's evidence).
async fn bundle_item_reports(
    db: &SurrealDatabase,
    workspace_id: &str,
    items: &[KnowledgeContextBundleItem],
) -> Result<(bool, Vec<Value>), ApiError> {
    let mut item_reports: Vec<Value> = Vec::with_capacity(items.len());
    let mut stale = false;
    for item in items {
        let (status, reason): (&str, Option<String>) = match item.ref_kind {
            KnowledgeBundleItemRefKind::Span => {
                match db
                    .get_knowledge_span(&item.ref_id)
                    .await
                    .map_err(storage_error)?
                {
                    None => (
                        "missing_evidence",
                        Some("cited span no longer exists".into()),
                    ),
                    Some(span) => {
                        match db
                            .get_knowledge_source(&span.source_id)
                            .await
                            .map_err(storage_error)?
                            .filter(|source| source.workspace_id == workspace_id)
                        {
                            None => (
                                "missing_evidence",
                                Some("the span's source no longer exists".into()),
                            ),
                            Some(source) if source.stale => (
                                "source_stale",
                                Some(format!(
                                    "source {} changed since indexing (stale)",
                                    source.source_id
                                )),
                            ),
                            Some(_) => ("ok", None),
                        }
                    }
                }
            }
            KnowledgeBundleItemRefKind::Passage => {
                match db
                    .get_knowledge_memory_passage(&item.ref_id)
                    .await
                    .map_err(storage_error)?
                    .filter(|passage| passage.workspace_id == workspace_id)
                {
                    None => (
                        "missing_evidence",
                        Some("cited passage no longer exists".into()),
                    ),
                    Some(_) => ("ok", None),
                }
            }
            KnowledgeBundleItemRefKind::Source => {
                match db
                    .get_knowledge_source(&item.ref_id)
                    .await
                    .map_err(storage_error)?
                    .filter(|source| source.workspace_id == workspace_id)
                {
                    None => (
                        "missing_evidence",
                        Some("cited source no longer exists".into()),
                    ),
                    Some(source) if source.stale => (
                        "source_stale",
                        Some("source changed since indexing (stale)".into()),
                    ),
                    Some(_) => ("ok", None),
                }
            }
            KnowledgeBundleItemRefKind::Entity => {
                // Entity candidates from the executed pipeline cite the stable
                // relationship_id (not a bare entity row id), so the entity
                // surface carries no per-item staleness check here.
                ("ok", None)
            }
            KnowledgeBundleItemRefKind::Claim => {
                match db
                    .get_knowledge_claim(&item.ref_id)
                    .await
                    .map_err(storage_error)?
                    .filter(|claim| claim.workspace_id == workspace_id)
                {
                    None => (
                        "missing_evidence",
                        Some("cited claim no longer exists".into()),
                    ),
                    Some(_) => ("ok", None),
                }
            }
        };
        if status != "ok" {
            stale = true;
        }
        item_reports.push(json!({
            "ref_kind": item.ref_kind.as_str(),
            "ref_id": item.ref_id,
            "status": status,
            "reason": reason,
        }));
    }
    Ok((stale, item_reports))
}

/// POST /knowledge/retrieval/bundles/:bundle_id/repair?workspace_id=
///
/// The repair action (adversarial-v2 MT-143): re-executes the bundle's
/// recorded query through the executed retrieval pipeline
/// (`knowledge_retrieval::executor`), producing a FRESH bundle + trace bound
/// to current index state. The response links old -> new so a consumer swaps
/// to the repaired bundle; the stale bundle stays (bundles are append-only
/// evidence, never silently rewritten). The re-execution runs as the account's
/// record user: it reads only this workspace's readable evidence and writes the
/// fresh bundle/items/trace under Create+fs.write.
async fn repair_bundle(
    State(state): State<AppState>,
    Path(bundle_id): Path<String>,
    Query(params): Query<WorkspaceParams>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let (account, ctx) = retrieval_account(&state, &headers, &params.workspace_id, true).await?;
    let db = db_for(&state);

    account
        .run(&state, async {
            let (_bundle, _items) =
                authorized_bundle(&db, &params.workspace_id, &bundle_id).await?;
            let traces = db
                .list_knowledge_retrieval_traces_for_bundle(&bundle_id)
                .await
                .map_err(storage_error)?;
            let Some(trace_row) = traces.first() else {
                return Err(bad_request(
                    "bundle has no recorded trace; the original query cannot be reproduced",
                ));
            };
            if trace_row.workspace_id != params.workspace_id {
                return Err(not_found("knowledge context bundle"));
            }
            let recorded_plan = &trace_row.decisions["query_plan"];
            let query_text = recorded_plan["query_text"]
                .as_str()
                .or(trace_row.query_text.as_deref())
                .ok_or_else(|| bad_request("recorded trace carries no query text to re-execute"))?
                .to_string();
            let recorded_mode = recorded_plan["retrieval_mode"].as_str().unwrap_or("");
            let target = trace_row.decisions["retrieval_trace"]["target"].clone();

            // Re-execute the recorded query against CURRENT index state.
            let mut request = RetrievalRequest::discovery(&params.workspace_id, &query_text);
            request.graph_neighborhood_expected = recorded_mode == "graph_traversal";
            let executed = execute_retrieval(
                &db,
                &state.surreal,
                &ctx.kernel_task_run_id,
                &ctx.session_run_id,
                BundleTargetKind::Task,
                &format!("repair:{bundle_id}"),
                &request,
                &BTreeSet::new(),
                GraphTraversalPolicy::default(),
            )
            .await
            .map_err(storage_error)?;

            let receipt = record_nav_receipt(
                state.storage.as_ref(),
                &ctx,
                &params.workspace_id,
                "repair_bundle",
                json!({
                    "bundle_id": bundle_id,
                    "repaired_bundle_id": executed.compiled.bundle_id,
                    "action": "reexecute",
                }),
            )
            .await?;

            Ok::<_, ApiError>(Json(json!({
                "bundle_id": bundle_id,
                "action": "reexecute",
                "repaired_bundle_id": executed.compiled.bundle_id,
                "repaired_trace_id": executed.compiled.trace_id,
                "fallback_reason": executed.fallback_reason,
                "ranked_candidates": executed.ranked.len(),
                "original_target": target,
                "retrieval_receipt_event_id": receipt,
            })))
        })
        .await
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
    let (account, ctx) = retrieval_account(&state, &headers, &params.workspace_id, false).await?;
    let limit = clamp_limit(params.limit);

    account
        .run(&state, async {
            let entries =
                list_semantic_catalog_entries(&state.surreal, &params.workspace_id, limit)
                    .await
                    .map_err(storage_error)?;

            let receipt = record_nav_receipt(
                state.storage.as_ref(),
                &ctx,
                &params.workspace_id,
                "list_catalog",
                json!({"workspace_id": params.workspace_id}),
            )
            .await?;

            Ok::<_, ApiError>(Json(json!({
                "workspace_id": params.workspace_id,
                "entries": entries,
                "count": entries.len(),
                "retrieval_receipt_event_id": receipt,
            })))
        })
        .await
}
