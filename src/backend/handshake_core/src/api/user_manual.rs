//! WP-KERNEL-009 MT-201 UserManualBackendApi (+ MT-199 quickstart bundles,
//! MT-200 access points, MT-203 legacy bridge, MT-204 freshness, MT-205
//! projections): the canonical `/usermanual/*` HTTP surface.
//!
//! Identity stance (differs from the knowledge surfaces ON PURPOSE): the
//! UserManual is the no-context BOOTSTRAP surface — it must be readable
//! before a caller knows the identity-header contract, because the manual is
//! where that contract is documented. Reads therefore accept anonymous
//! callers; page opens, the legacy bridge, freshness checks, and resyncs are
//! still attributable: they append `KNOWLEDGE_USER_MANUAL_ENTRY_RECORDED`
//! receipts (synthesizing bootstrap identity when headers are absent) and
//! RETURN the receipt id, so even anonymous discovery is auditable (spec
//! 2.3.13.11 receipt law; spec 10.15.8 compatibility receipts).
//!
//! Write stance: the ONLY write route is `POST /usermanual/resync`, gated on
//! an explicitly asserted actor kind (`operator` | `system` | `local_model`).
//! `cloud_model`, `validator`, and unauthenticated callers are DENIED (403,
//! stable reason codes); unknown tokens are 400 (privilege is asserted and
//! validated, never inferred — same fail-closed law as the documents API).
//! Manual content itself comes only from the compiled-in seed corpus, so
//! manual text cannot be injected at runtime through this surface.

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::knowledge_document::permission::DocumentActorKind;
use crate::storage::postgres::PostgresDatabase;
use crate::storage::StorageError;
use crate::user_manual::freshness::check_freshness;
use crate::user_manual::migration_plan::naming_migration_plan;
use crate::user_manual::projection::{render_page_html, render_page_markdown};
use crate::user_manual::registry::{user_manual_access_points, wp009_surface_registry};
use crate::user_manual::seed::{ensure_seeded, QUICKSTART_AREAS};
use crate::user_manual::spec_seed::spec_enrichment_seed;
use crate::user_manual::store::{UserManualStore, LIST_CAP};
use crate::user_manual::{ROUTE_NAMESPACE, USER_MANUAL_VERSION};
use crate::AppState;

const HSK_HEADER_ACTOR_KIND: &str = "x-hsk-actor-kind";
const HSK_HEADER_ACTOR_ID: &str = "x-hsk-actor-id";
const HSK_HEADER_KERNEL_TASK_RUN_ID: &str = "x-hsk-kernel-task-run-id";
const HSK_HEADER_SESSION_RUN_ID: &str = "x-hsk-session-run-id";

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/usermanual/pages", get(list_pages))
        .route("/usermanual/pages/:slug", get(get_page))
        .route("/usermanual/pages/:slug/links", get(page_links))
        .route("/usermanual/pages/:slug/projection", get(page_projection))
        .route("/usermanual/search", get(search))
        .route("/usermanual/tools", get(list_tools))
        .route("/usermanual/tools/:tool_id", get(get_tool))
        .route("/usermanual/features", get(list_features))
        .route("/usermanual/quickstarts/:area", get(quickstart))
        .route("/usermanual/freshness", get(freshness))
        .route("/usermanual/access-points", get(access_points))
        .route("/usermanual/legacy/model-manual", get(legacy_model_manual))
        .route("/usermanual/legacy/aliases", get(legacy_aliases))
        .route("/usermanual/migration-plan", get(migration_plan))
        .route(
            "/usermanual/spec-enrichment-seed",
            get(spec_enrichment_seed_rows),
        )
        .route("/usermanual/resync", post(resync))
        .with_state(state)
}

type ApiError = (StatusCode, Json<Value>);

fn db_for(state: &AppState) -> PostgresDatabase {
    PostgresDatabase::new(state.postgres_pool.clone())
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

fn forbidden(reason: &str) -> ApiError {
    (
        StatusCode::FORBIDDEN,
        Json(json!({"error": "forbidden", "reason": reason})),
    )
}

fn storage_error(err: StorageError) -> ApiError {
    tracing::error!(target: "handshake_core::user_manual", error = %err, "user_manual_api_error");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({"error": "internal_error"})),
    )
}

fn header_str<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

/// Caller identity for receipts: real headers when present, synthesized
/// bootstrap identity otherwise (the manual must be readable with NO prior
/// context — the bootstrap receipt teaches the caller its first ids).
struct ManualIdentity {
    actor_id: String,
    kernel_task_run_id: String,
    session_run_id: String,
    bootstrap: bool,
}

fn manual_identity(headers: &HeaderMap) -> ManualIdentity {
    let actor_id = header_str(headers, HSK_HEADER_ACTOR_ID).map(str::to_string);
    let task = header_str(headers, HSK_HEADER_KERNEL_TASK_RUN_ID).map(str::to_string);
    let session = header_str(headers, HSK_HEADER_SESSION_RUN_ID).map(str::to_string);
    let bootstrap = actor_id.is_none() || task.is_none() || session.is_none();
    ManualIdentity {
        actor_id: actor_id.unwrap_or_else(|| "anonymous".to_string()),
        kernel_task_run_id: task.unwrap_or_else(|| "UM-BOOTSTRAP".to_string()),
        session_run_id: session.unwrap_or_else(|| format!("UMB-{}", Uuid::now_v7())),
        bootstrap,
    }
}

async fn append_read_receipt(
    db: &PostgresDatabase,
    identity: &ManualIdentity,
    action: &str,
    subject: &str,
) -> Result<String, ApiError> {
    let store = UserManualStore::new(db);
    store
        .append_manual_receipt(
            action,
            subject,
            json!({
                "actor_id": identity.actor_id,
                "kernel_task_run_id": identity.kernel_task_run_id,
                "session_run_id": identity.session_run_id,
                "bootstrap_identity": identity.bootstrap,
            }),
        )
        .await
        .map_err(storage_error)
}

// ---------------------------------------------------------------------------
// Pages.
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct ListPagesQuery {
    kind: Option<String>,
    audience: Option<String>,
    limit: Option<i64>,
}

async fn list_pages(
    State(state): State<AppState>,
    Query(params): Query<ListPagesQuery>,
) -> Result<Json<Value>, ApiError> {
    let db = db_for(&state);
    let store = UserManualStore::new(&db);
    let pages = store
        .list_pages(
            params.kind.as_deref(),
            params.audience.as_deref(),
            params.limit.unwrap_or(LIST_CAP),
        )
        .await
        .map_err(storage_error)?;
    Ok(Json(json!({
        "manual_version": USER_MANUAL_VERSION,
        "route_namespace": ROUTE_NAMESPACE,
        "count": pages.len(),
        "pages": pages.iter().map(|p| json!({
            "slug": p.slug,
            "title": p.title,
            "page_kind": p.page_kind,
            "audience": p.audience,
            "manual_version": p.manual_version,
            "content_hash": p.content_hash,
            "status": p.status,
            "updated_at": p.updated_at,
        })).collect::<Vec<_>>(),
    })))
}

async fn get_page(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let db = db_for(&state);
    let store = UserManualStore::new(&db);
    let Some((page, sections, anchors)) =
        store.get_page_by_slug(&slug).await.map_err(storage_error)?
    else {
        return Err(not_found(format!("no UserManual page with slug '{slug}'")));
    };
    let identity = manual_identity(&headers);
    let receipt = append_read_receipt(&db, &identity, "page_opened", &slug).await?;
    Ok(Json(json!({
        "page": page,
        "sections": sections,
        "anchors": anchors,
        "bootstrap_receipt_event_id": receipt,
        "bootstrap_identity_used": identity.bootstrap,
    })))
}

async fn page_links(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let db = db_for(&state);
    let store = UserManualStore::new(&db);
    let Some((outbound, inbound)) = store.page_links(&slug).await.map_err(storage_error)? else {
        return Err(not_found(format!("no UserManual page with slug '{slug}'")));
    };
    Ok(Json(json!({
        "slug": slug,
        "outbound": outbound,
        "inbound": inbound,
    })))
}

#[derive(Debug, Deserialize)]
struct ProjectionQuery {
    format: Option<String>,
}

async fn page_projection(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(params): Query<ProjectionQuery>,
) -> Result<Json<Value>, ApiError> {
    let format = params.format.unwrap_or_else(|| "html".to_string());
    if !matches!(format.as_str(), "html" | "markdown") {
        return Err(bad_request(format!(
            "unsupported projection format '{format}' (html | markdown)"
        )));
    }
    let db = db_for(&state);
    let store = UserManualStore::new(&db);
    let Some((page, sections, anchors)) =
        store.get_page_by_slug(&slug).await.map_err(storage_error)?
    else {
        return Err(not_found(format!("no UserManual page with slug '{slug}'")));
    };
    let rendered = match format.as_str() {
        "html" => render_page_html(&page, &sections, &anchors),
        _ => render_page_markdown(&page, &sections, &anchors),
    };
    Ok(Json(json!({
        "slug": slug,
        "format": format,
        "manual_version": page.manual_version,
        "content_hash": page.content_hash,
        "rendered": rendered,
        "note": "projection only — the PostgreSQL UserManual rows remain canonical",
    })))
}

// ---------------------------------------------------------------------------
// Search / tools / features.
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct SearchQuery {
    q: Option<String>,
    limit: Option<i64>,
}

async fn search(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<Value>, ApiError> {
    let query = params.q.unwrap_or_default();
    if query.trim().is_empty() {
        return Err(bad_request("query parameter 'q' is required and non-empty"));
    }
    let db = db_for(&state);
    let store = UserManualStore::new(&db);
    let hits = store
        .search(&query, params.limit.unwrap_or(50))
        .await
        .map_err(storage_error)?;
    Ok(Json(json!({
        "query": query.trim(),
        "count": hits.len(),
        "results": hits,
    })))
}

#[derive(Debug, Deserialize)]
struct ListToolsQuery {
    status: Option<String>,
    origin: Option<String>,
    limit: Option<i64>,
}

async fn list_tools(
    State(state): State<AppState>,
    Query(params): Query<ListToolsQuery>,
) -> Result<Json<Value>, ApiError> {
    let db = db_for(&state);
    let store = UserManualStore::new(&db);
    let tools = store
        .list_tool_entries(
            params.status.as_deref(),
            params.origin.as_deref(),
            params.limit.unwrap_or(LIST_CAP),
        )
        .await
        .map_err(storage_error)?;
    Ok(Json(json!({"count": tools.len(), "tools": tools})))
}

async fn get_tool(
    State(state): State<AppState>,
    Path(tool_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let db = db_for(&state);
    let store = UserManualStore::new(&db);
    let Some(tool) = store.get_tool_entry(&tool_id).await.map_err(storage_error)? else {
        return Err(not_found(format!("no UserManual tool entry '{tool_id}'")));
    };
    Ok(Json(json!({"tool": tool})))
}

async fn list_features(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let db = db_for(&state);
    let store = UserManualStore::new(&db);
    let features = store
        .list_feature_entries(LIST_CAP)
        .await
        .map_err(storage_error)?;
    Ok(Json(json!({"count": features.len(), "features": features})))
}

// ---------------------------------------------------------------------------
// MT-199 quickstart bundles.
// ---------------------------------------------------------------------------

async fn quickstart(
    State(state): State<AppState>,
    Path(area): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    if !QUICKSTART_AREAS.contains(&area.as_str()) {
        return Err(not_found(format!(
            "unknown quickstart area '{}' (known: {})",
            area,
            QUICKSTART_AREAS.join(", ")
        )));
    }
    let db = db_for(&state);
    let store = UserManualStore::new(&db);
    let quickstart_slug = format!("quickstart-{area}");
    let Some((page, sections, anchors)) = store
        .get_page_by_slug(&quickstart_slug)
        .await
        .map_err(storage_error)?
    else {
        return Err(not_found(format!(
            "quickstart page '{quickstart_slug}' is not seeded — POST /usermanual/resync"
        )));
    };

    // Bundle the quickstart page PLUS every page it links to (one fetch for a
    // no-context model; bounded by the page's own link list).
    let mut bundled = Vec::new();
    for anchor in &anchors {
        if anchor.anchor_kind == "page_link" {
            if let Some((linked_page, linked_sections, _)) = store
                .get_page_by_slug(&anchor.anchor_value)
                .await
                .map_err(storage_error)?
            {
                bundled.push(json!({
                    "slug": linked_page.slug,
                    "title": linked_page.title,
                    "sections": linked_sections,
                }));
            }
        }
    }
    let identity = manual_identity(&headers);
    let receipt = append_read_receipt(&db, &identity, "quickstart_opened", &quickstart_slug).await?;
    Ok(Json(json!({
        "area": area,
        "manual_version": page.manual_version,
        "quickstart": {
            "slug": page.slug,
            "title": page.title,
            "sections": sections,
            "anchors": anchors,
        },
        "linked_pages": bundled,
        "bootstrap_receipt_event_id": receipt,
    })))
}

// ---------------------------------------------------------------------------
// MT-204 freshness.
// ---------------------------------------------------------------------------

async fn freshness(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let db = db_for(&state);
    let report = check_freshness(&db).await.map_err(storage_error)?;
    let identity = manual_identity(&headers);
    let store = UserManualStore::new(&db);
    let receipt = store
        .append_manual_receipt(
            "freshness_checked",
            USER_MANUAL_VERSION,
            json!({
                "fresh": report.fresh,
                "current_count": report.current_count,
                "problem_count": report.problem_count,
                "actor_id": identity.actor_id,
                "session_run_id": identity.session_run_id,
            }),
        )
        .await
        .map_err(storage_error)?;
    Ok(Json(json!({
        "report": report,
        "receipt_event_id": receipt,
    })))
}

// ---------------------------------------------------------------------------
// MT-200 access points + registry projections.
// ---------------------------------------------------------------------------

async fn access_points(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    // Verify targets resolve against the LIVE database (an access point to a
    // missing page is a defect surfaced here, not in the UI).
    let db = db_for(&state);
    let store = UserManualStore::new(&db);
    let mut rows = Vec::new();
    for ap in user_manual_access_points() {
        let resolves = store
            .get_page_by_slug(ap.target_page_slug)
            .await
            .map_err(storage_error)?
            .is_some();
        rows.push(json!({
            "access_point_id": ap.access_point_id,
            "host_surface": ap.host_surface,
            "entry_kind": ap.entry_kind,
            "target_page_slug": ap.target_page_slug,
            "ui_wiring_route": ap.ui_wiring_route,
            "stable_element_id": ap.stable_element_id,
            "note": ap.note,
            "target_resolves": resolves,
        }));
    }
    Ok(Json(json!({"count": rows.len(), "access_points": rows})))
}

async fn migration_plan() -> Json<Value> {
    let plan = naming_migration_plan();
    Json(json!({
        "plan_id": plan.plan_id,
        "spec_anchor": plan.spec_anchor,
        "canonical_term": plan.canonical_term,
        "rows": plan.rows,
        "aliases": plan.aliases,
    }))
}

async fn spec_enrichment_seed_rows() -> Json<Value> {
    Json(json!({
        "manual_version": USER_MANUAL_VERSION,
        "rows": spec_enrichment_seed(),
    }))
}

// ---------------------------------------------------------------------------
// MT-203 legacy bridge.
// ---------------------------------------------------------------------------

async fn legacy_model_manual(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let db = db_for(&state);
    let store = UserManualStore::new(&db);
    let identity = manual_identity(&headers);
    // Spec 10.15.8: a legacy path "emits a compatibility receipt when used".
    let receipt = store
        .append_manual_receipt(
            "legacy_bridge_used",
            "model_manual",
            json!({
                "legacy_surface": "model_manual",
                "canonical_namespace": ROUTE_NAMESPACE,
                "actor_id": identity.actor_id,
                "session_run_id": identity.session_run_id,
            }),
        )
        .await
        .map_err(storage_error)?;
    let pages = store
        .list_pages(None, None, LIST_CAP)
        .await
        .map_err(storage_error)?;
    let aliases = store.list_legacy_aliases().await.map_err(storage_error)?;
    Ok(Json(json!({
        "deprecated": true,
        "deprecation_note": "ModelManual is a deprecated legacy name. The canonical surface is UserManual: GET /usermanual/pages, /usermanual/tools, /usermanual/search. Mapping: GET /usermanual/legacy/aliases.",
        "canonical": {
            "route_namespace": ROUTE_NAMESPACE,
            "manual_version": USER_MANUAL_VERSION,
            "pages": pages.iter().map(|p| json!({"slug": p.slug, "title": p.title})).collect::<Vec<_>>(),
        },
        "aliases": aliases,
        "compatibility_receipt_event_id": receipt,
    })))
}

async fn legacy_aliases(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let db = db_for(&state);
    let store = UserManualStore::new(&db);
    let aliases = store.list_legacy_aliases().await.map_err(storage_error)?;
    Ok(Json(json!({"count": aliases.len(), "aliases": aliases})))
}

// ---------------------------------------------------------------------------
// Gated resync.
// ---------------------------------------------------------------------------

async fn resync(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    // Fail-closed actor gate (mirrors the documents permission law): absent
    // kind = unauthenticated (deny), unknown token = 400, cloud_model and
    // validator = deny.
    let actor_kind = match header_str(&headers, HSK_HEADER_ACTOR_KIND) {
        None => DocumentActorKind::least_privileged(),
        Some(value) => DocumentActorKind::from_wire(value)
            .ok_or_else(|| bad_request(format!("unknown x-hsk-actor-kind '{value}'")))?,
    };
    match actor_kind {
        DocumentActorKind::Operator | DocumentActorKind::System | DocumentActorKind::LocalModel => {}
        DocumentActorKind::CloudModel => return Err(forbidden("cloud_model_resync_denied")),
        DocumentActorKind::Validator => return Err(forbidden("validator_resync_denied")),
        DocumentActorKind::Unauthenticated => {
            return Err(forbidden("unauthenticated_resync_denied"))
        }
    }
    let db = db_for(&state);
    let report = ensure_seeded(&db).await.map_err(storage_error)?;
    Ok(Json(json!({"resync": report})))
}

// ---------------------------------------------------------------------------
// Registry self-check helper exposed for tests (not a route).
// ---------------------------------------------------------------------------

/// The registry rows for THIS module's own routes — used by the
/// doc-vs-runtime test to confirm the manual documents itself accurately.
pub fn own_registry_rows() -> Vec<&'static crate::user_manual::registry::SurfaceDescriptor> {
    wp009_surface_registry()
        .iter()
        .filter(|s| s.route.starts_with("/usermanual"))
        .collect()
}
