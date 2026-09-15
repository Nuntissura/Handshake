//! WP-KERNEL-012 MT-109 FEMS memory routes: the backend surfaces the native editors
//! read (relevant-memory pack) and write (review-gated proposal) against.
//!
//! * `GET  /workspaces/{workspace_id}/memory/pack` (AC-109-2) — returns the REAL
//!   [`crate::ace::MemoryPack`] shape (`items[].memory_id` / `memory_class` /
//!   `source_refs`). The native `MemoryClient` (frontend MT-063) reads this; the
//!   response is a pinned contract so the client model can be aligned.
//! * `POST /workspaces/{workspace_id}/memory/proposals` (AC-109-3) — stores a
//!   review-gated memory-write PROPOSAL (`document_id` + selection-range + content-hash
//!   provenance REQUIRED, fail-closed on missing provenance). The proposal lands as
//!   `status='pending_review'`; there is NO path from this route to a committed memory
//!   item — a proposal can NEVER mutate memory directly.
//! * `GET /workspaces/{workspace_id}/memory/proposals` lists an explicitly bounded,
//!   deterministic workspace projection of pending-review proposals.
//! * `GET /workspaces/{workspace_id}/memory/proposals/{proposal_id}` reads the exact
//!   pending-review row back for operator/model diagnostics and correlation proof.
//! * `POST /workspaces/{workspace_id}/memory/proposals/{proposal_id}/review` performs the
//!   auditable pending-review -> approved/rejected transition.
//! * `POST /workspaces/{workspace_id}/memory/proposals/{proposal_id}/commit` accepts only an
//!   approved proposal and atomically writes the canonical `MemoryItem`, `MemoryCommitReport`,
//!   strict `MemoryPack`, and EventLedger commit receipt; FR-EVT-MEM-003 is projected from it.
//!
//! All durable writes use the embedded SurrealDB FEMS store plus durable kernel EventLedger
//! receipts; review decisions are mirrored to Flight Recorder.

use axum::{
    extract::Request,
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue, Method, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::ace::{
    FemsEntityRef, FemsSourceRef, FemsSourceRefKind, MemoryCommitReport, MemoryMutationOp,
    MemoryPack, MemoryPackBudgets, MemoryPackDeterminismMode, MemoryPolicy, MemoryWriteOp,
    MemoryWritePolicy, MemoryWriteProposal, PartialMemoryItem,
};
use crate::flight_recorder::{
    EventFilter, FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType, RecorderError,
};
use crate::kernel::{KernelActor, KernelEventType, NewKernelEvent};
use crate::storage::knowledge::{KnowledgeSourceKind, KnowledgeStore};
use crate::storage::surreal::SurrealDatabase;
use crate::storage::{fems_memory, StorageError};
use crate::AppState;

const HSK_HEADER_ACTOR_KIND: &str = "x-hsk-actor-kind";
const HSK_HEADER_ACTOR_ID: &str = "x-hsk-actor-id";
const HSK_HEADER_KERNEL_TASK_RUN_ID: &str = "x-hsk-kernel-task-run-id";
const HSK_HEADER_SESSION_RUN_ID: &str = "x-hsk-session-run-id";

const MEMORY_READ_CAPABILITY: &str = "memory.read";
const MEMORY_PROPOSE_CAPABILITY: &str = "memory.propose";
const MEMORY_REVIEW_CAPABILITY: &str = "memory.review";
const MEMORY_COMMIT_CAPABILITY: &str = "memory.commit";

const PROPOSAL_STATUS_PENDING_REVIEW: &str = "pending_review";
const PACK_SCHEMA_VERSION: &str = "hsk.memory_pack@0.1";

type ApiError = (StatusCode, Json<Value>);

pub fn routes(state: AppState) -> Router {
    routes_with_reconciler(state).0
}

fn routes_with_reconciler(state: AppState) -> (Router, Option<tokio::task::JoinHandle<()>>) {
    let reconciler = spawn_memory_commit_reconciler(state.clone());
    let middleware_state = state.clone();
    let router = Router::new()
        .route(
            "/workspaces/:workspace_id/memory/pack",
            get(get_memory_pack),
        )
        .route(
            "/workspaces/:workspace_id/memory/proposals",
            get(list_memory_proposals).post(create_memory_proposal),
        )
        .route(
            "/workspaces/:workspace_id/memory/proposals/:proposal_id",
            get(get_memory_proposal),
        )
        .route(
            "/workspaces/:workspace_id/memory/proposals/:proposal_id/artifact",
            get(get_memory_proposal_artifact),
        )
        .route(
            "/workspaces/:workspace_id/memory/proposals/:proposal_id/review",
            axum::routing::post(review_memory_proposal),
        )
        .route(
            "/workspaces/:workspace_id/memory/proposals/:proposal_id/commit",
            axum::routing::post(commit_memory_proposal),
        )
        .route(
            "/workspaces/:workspace_id/memory/commits/:commit_id/report",
            get(get_memory_commit_report),
        )
        .route(
            "/workspaces/:workspace_id/memory/items/count",
            get(get_committed_memory_count),
        )
        .layer(middleware::from_fn_with_state(
            middleware_state,
            authorize_memory_request,
        ))
        .with_state(state);
    (router, reconciler)
}

fn capability_for_request(method: &Method, path: &str) -> &'static str {
    if method == Method::POST && path.ends_with("/review") {
        MEMORY_REVIEW_CAPABILITY
    } else if method == Method::POST && path.ends_with("/commit") {
        MEMORY_COMMIT_CAPABILITY
    } else if method == Method::POST && path.ends_with("/memory/proposals") {
        MEMORY_PROPOSE_CAPABILITY
    } else {
        MEMORY_READ_CAPABILITY
    }
}

fn protected_resource_for_request(
    method: &Method,
    path: &str,
) -> (
    crate::storage::surreal::resource_authority::ResourceKind,
    crate::storage::surreal::resource_authority::ResourceAction,
) {
    use crate::storage::surreal::resource_authority::{ResourceAction, ResourceKind};
    if path.ends_with("/memory/pack") {
        (ResourceKind::MemoryPack, ResourceAction::Read)
    } else if path.ends_with("/report") {
        (ResourceKind::MemoryCommitReport, ResourceAction::Read)
    } else if path.ends_with("/items/count") {
        (ResourceKind::MemoryItemCount, ResourceAction::Read)
    } else if method == Method::POST && path.ends_with("/memory/proposals") {
        (ResourceKind::MemoryProposal, ResourceAction::Create)
    } else if method == Method::POST {
        (ResourceKind::MemoryProposal, ResourceAction::Update)
    } else {
        (ResourceKind::MemoryProposal, ResourceAction::Read)
    }
}

async fn record_memory_capability_decision(
    state: &AppState,
    ctx: Option<&crate::api::stage::CaptureContext>,
    capability_id: &'static str,
    decision_outcome: &'static str,
    workspace_id: Option<String>,
) -> Result<(), RecorderError> {
    let trace_id = Uuid::now_v7();
    let policy_decision_id = format!("native-fems-capability:{trace_id}");
    let actor_id = ctx
        .map(|ctx| ctx.actor_id.as_str())
        .unwrap_or("unauthenticated-native-client");
    let actor = if ctx.is_some() {
        FlightRecorderActor::Human
    } else {
        FlightRecorderActor::System
    };
    let mut event = FlightRecorderEvent::new(
        FlightRecorderEventType::CapabilityAction,
        actor,
        trace_id,
        json!({
            "capability_id": capability_id,
            "actor_id": actor_id,
            "job_id": null,
            "decision_outcome": decision_outcome,
        }),
    )
    .with_actor_id(actor_id)
    .with_capability_id(capability_id)
    .with_policy_decision_id(policy_decision_id);
    if let Some(workspace_id) = workspace_id {
        event = event.with_wsids(vec![workspace_id]);
    }
    state.flight_recorder.record_event(event).await
}

fn workspace_id_from_memory_path(path: &str) -> Option<String> {
    let mut segments = path.trim_start_matches('/').split('/');
    match (segments.next(), segments.next(), segments.next()) {
        (Some("workspaces"), Some(workspace_id), Some("memory")) if !workspace_id.is_empty() => {
            Some(workspace_id.to_owned())
        }
        _ => None,
    }
}

async fn authorize_memory_request(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let capability_id = capability_for_request(request.method(), request.uri().path());
    let workspace_id = workspace_id_from_memory_path(request.uri().path());
    let Some(workspace_id) = workspace_id else {
        return crate::api::authority::constant_denial().into_response();
    };
    let (resource_kind, action) =
        protected_resource_for_request(request.method(), request.uri().path());
    let authority = match crate::api::authority::authorize_request(
        &state,
        request.headers(),
        capability_id,
        resource_kind,
        &workspace_id,
        action,
    )
    .await
    {
        Ok(authority) => authority,
        Err(error) => {
            if record_memory_capability_decision(
                &state,
                None,
                capability_id,
                "deny",
                Some(workspace_id.clone()),
            )
            .await
            .is_err()
            {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": "memory capability audit failed closed"})),
                )
                    .into_response();
            }
            return error.into_response();
        }
    };
    if request.method() == Method::POST && request.uri().path().ends_with("/commit") {
        use crate::storage::surreal::resource_authority::{ResourceAction, ResourceKind};
        for resource_kind in [
            ResourceKind::MemoryPack,
            ResourceKind::MemoryItem,
            ResourceKind::MemoryCommitReport,
        ] {
            if crate::api::authority::authorize_request(
                &state,
                request.headers(),
                capability_id,
                resource_kind,
                &workspace_id,
                ResourceAction::Create,
            )
            .await
            .is_err()
            {
                let _ = record_memory_capability_decision(
                    &state,
                    None,
                    capability_id,
                    "deny",
                    Some(workspace_id.clone()),
                )
                .await;
                return crate::api::authority::constant_denial().into_response();
            }
        }
    }
    let ctx = authority.capture_context();
    if record_memory_capability_decision(
        &state,
        Some(&ctx),
        capability_id,
        "allow",
        Some(workspace_id),
    )
    .await
    .is_err()
    {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "memory capability audit failed closed"})),
        )
            .into_response();
    }

    let canonical_headers = [
        (HSK_HEADER_ACTOR_KIND, ctx.actor_kind.as_str()),
        (HSK_HEADER_ACTOR_ID, ctx.actor_id.as_str()),
        (
            HSK_HEADER_KERNEL_TASK_RUN_ID,
            ctx.kernel_task_run_id.as_str(),
        ),
        (HSK_HEADER_SESSION_RUN_ID, ctx.session_run_id.as_str()),
    ];
    for (name, value) in canonical_headers {
        let Ok(value) = HeaderValue::from_str(value) else {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "native session identity could not be encoded"})),
            )
                .into_response();
        };
        request.headers_mut().insert(name, value);
    }
    state
        .surreal
        .with_record_user_scope(authority.record_user_scope, next.run(request))
        .await
}

fn spawn_memory_commit_reconciler(state: AppState) -> Option<tokio::task::JoinHandle<()>> {
    let runtime = tokio::runtime::Handle::try_current().ok()?;
    Some(runtime.spawn(async move {
        if let Err(error) = reconcile_all_memory_commit_events(&state).await {
            tracing::error!(
                target: "handshake_core::memory_api",
                error = ?error,
                "fems_memory_commit_startup_reconciliation_failed"
            );
        }
    }))
}

const MAX_MEMORY_OUTBOX_RECONCILIATION_PASSES: usize = 4;

async fn project_lifecycle_event(
    state: &AppState,
    workspace_id: &str,
    event: FlightRecorderEvent,
) -> Result<(), ApiError> {
    if let Err(error) = record_review_event_idempotent(state, event.clone()).await {
        fems_memory::record_memory_lifecycle_event_failure(
            &state.surreal,
            workspace_id,
            event.event_id,
            &format!("{error:?}"),
            false,
        )
        .await
        .map_err(storage_error)?;
        return Err(error);
    }
    fems_memory::mark_memory_lifecycle_event_published(&state.surreal, workspace_id, event.event_id)
        .await
        .map_err(storage_error)
}

async fn project_commit_event(
    state: &AppState,
    workspace_id: &str,
    event: FlightRecorderEvent,
) -> Result<(), ApiError> {
    // WP-KERNEL-012 MT-146 D-146-3. The pending listing now reports every durable un-published
    // commit-outbox row, so the causal rule that a pack-built event never precedes its commit
    // event is enforced here. Skipping is not a delivery failure: the row keeps its attempt
    // count and stays visible as pending rather than being driven toward quarantine.
    if !fems_memory::commit_event_dispatch_ready(&state.surreal, workspace_id, event.event_id)
        .await
        .map_err(storage_error)?
    {
        return Ok(());
    }
    if let Err(error) = record_review_event_idempotent(state, event.clone()).await {
        fems_memory::record_memory_commit_event_failure(
            &state.surreal,
            workspace_id,
            event.event_id,
            &format!("{error:?}"),
            false,
        )
        .await
        .map_err(storage_error)?;
        return Err(error);
    }
    fems_memory::mark_memory_commit_event_published(&state.surreal, workspace_id, event.event_id)
        .await
        .map_err(storage_error)
}

async fn reconcile_all_memory_commit_events(state: &AppState) -> Result<(), ApiError> {
    let authority = crate::api::authority::reconciliation_authority(
        state,
        MEMORY_COMMIT_CAPABILITY,
    )
    .await
    .map_err(|error| {
        tracing::error!(target: "handshake_core::memory_api", error = %error, "memory reconciliation authority denied");
        crate::api::authority::constant_denial()
    })?;
    state
        .surreal
        .with_record_user_scope(
            authority.record_user_scope.clone(),
            reconcile_all_memory_commit_events_authorized(state, &authority),
        )
        .await
}

async fn reconcile_all_memory_commit_events_authorized(
    state: &AppState,
    authority: &crate::api::authority::ReconciliationAuthority,
) -> Result<(), ApiError> {
    fems_memory::recover_missing_memory_lifecycle_outbox_events(&state.surreal)
        .await
        .map_err(storage_error)?;
    fems_memory::recover_missing_memory_commit_outbox_events(&state.surreal)
        .await
        .map_err(storage_error)?;
    let mut first_error = None;
    for _ in 0..MAX_MEMORY_OUTBOX_RECONCILIATION_PASSES {
        let lifecycle = fems_memory::list_all_pending_memory_lifecycle_events(&state.surreal, 200)
            .await
            .map_err(storage_error)?;
        let lifecycle_len = lifecycle.len();
        for (workspace_id, event) in lifecycle {
            if crate::api::authority::authorize_reconciliation_workspace(
                state,
                authority,
                &workspace_id,
                MEMORY_COMMIT_CAPABILITY,
            )
            .await
            .is_err()
            {
                continue;
            }
            if let Err(error) = project_lifecycle_event(state, &workspace_id, event).await {
                tracing::error!(
                    target: "handshake_core::memory_api",
                    workspace_id,
                    error = ?error,
                    "fems_memory_lifecycle_outbox_projection_failed"
                );
                first_error.get_or_insert(error);
            }
        }
        let pending = fems_memory::list_all_pending_memory_commit_events(&state.surreal, 200)
            .await
            .map_err(storage_error)?;
        if pending.is_empty() && lifecycle_len == 0 {
            return first_error.map_or(Ok(()), Err);
        }
        let batch_len = pending.len();
        for (workspace_id, event) in pending {
            if crate::api::authority::authorize_reconciliation_workspace(
                state,
                authority,
                &workspace_id,
                MEMORY_COMMIT_CAPABILITY,
            )
            .await
            .is_err()
            {
                continue;
            }
            if let Err(error) = project_commit_event(state, &workspace_id, event).await {
                tracing::error!(
                    target: "handshake_core::memory_api",
                    workspace_id,
                    error = ?error,
                    "fems_memory_commit_outbox_projection_failed"
                );
                first_error.get_or_insert(error);
            }
        }
        if batch_len == 0 && lifecycle_len == 0 {
            break;
        }
    }
    first_error.map_or(Ok(()), Err)
}

async fn reconcile_workspace_memory_commit_events(
    state: &AppState,
    workspace_id: &str,
) -> Result<(), ApiError> {
    let mut first_error = None;
    for _ in 0..MAX_MEMORY_OUTBOX_RECONCILIATION_PASSES {
        let lifecycle =
            fems_memory::list_pending_memory_lifecycle_events(&state.surreal, workspace_id, 200)
                .await
                .map_err(storage_error)?;
        let lifecycle_len = lifecycle.len();
        for event in lifecycle {
            if let Err(error) = project_lifecycle_event(state, workspace_id, event).await {
                first_error.get_or_insert(error);
            }
        }
        let pending =
            fems_memory::list_pending_memory_commit_events(&state.surreal, workspace_id, 200)
                .await
                .map_err(storage_error)?;
        let pending_len = pending.len();
        for event in pending {
            if let Err(error) = project_commit_event(state, workspace_id, event).await {
                first_error.get_or_insert(error);
            }
        }
        if lifecycle_len == 0 && pending_len == 0 {
            break;
        }
    }
    first_error.map_or(Ok(()), Err)
}

async fn require_published_proposal_event(
    state: &AppState,
    workspace_id: &str,
    proposal_id: &str,
) -> Result<(), ApiError> {
    let proposal = fems_memory::get_memory_proposal(&state.surreal, proposal_id)
        .await
        .map_err(storage_error)?
        .ok_or_else(|| storage_error(StorageError::NotFound("memory proposal")))?;
    if proposal.workspace_id != workspace_id {
        return Err(storage_error(StorageError::NotFound("memory proposal")));
    }
    reconcile_workspace_memory_commit_events(state, workspace_id).await?;
    match fems_memory::memory_lifecycle_publication_state(
        &state.surreal,
        proposal_id,
        "FR-EVT-MEM-001",
    )
    .await
    .map_err(storage_error)?
    {
        fems_memory::MemoryLifecyclePublicationState::Published => Ok(()),
        publication_state => Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "error": "proposal_event_not_published",
                "detail": format!(
                    "review and commit are blocked until FR-EVT-MEM-001 is durably published ({publication_state:?})"
                ),
                "proposal_id": proposal_id,
            })),
        )),
    }
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct CommittedMemoryCount {
    workspace_id: String,
    count: i64,
}

#[derive(Debug, Default, Deserialize)]
struct ProposalListQuery {
    limit: Option<u32>,
}

async fn list_memory_proposals(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    Query(query): Query<ProposalListQuery>,
) -> Result<Json<Vec<fems_memory::StoredMemoryProposal>>, ApiError> {
    if state
        .storage
        .get_workspace(&workspace_id)
        .await
        .map_err(storage_error)?
        .is_none()
    {
        return Err(storage_error(StorageError::NotFound("workspace")));
    }
    reconcile_workspace_memory_commit_events(&state, &workspace_id).await?;
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let proposals = fems_memory::list_memory_proposals(&state.surreal, &workspace_id, limit as i64)
        .await
        .map_err(storage_error)?;
    let mut visible = Vec::with_capacity(proposals.len());
    for proposal in proposals {
        if memory_proposal_sources_visible(&state, &proposal).await? {
            visible.push(proposal);
        }
    }
    Ok(Json(visible))
}

async fn get_memory_commit_report(
    State(state): State<AppState>,
    Path((workspace_id, commit_id)): Path<(String, String)>,
) -> Result<Json<MemoryCommitReport>, ApiError> {
    let report = fems_memory::get_memory_commit_report(&state.surreal, &workspace_id, &commit_id)
        .await
        .map_err(storage_error)?
        .ok_or_else(|| storage_error(StorageError::NotFound("memory commit report artifact")))?;
    let proposal = fems_memory::get_memory_proposal(&state.surreal, &report.source_proposal_id)
        .await
        .map_err(storage_error)?
        .filter(|proposal| proposal.workspace_id == workspace_id)
        .ok_or_else(|| storage_error(StorageError::NotFound("memory commit report artifact")))?;
    if !memory_proposal_sources_visible(&state, &proposal).await? {
        return Err(storage_error(StorageError::NotFound(
            "memory commit report artifact",
        )));
    }
    Ok(Json(report))
}

async fn get_committed_memory_count(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
) -> Result<Json<CommittedMemoryCount>, ApiError> {
    if state
        .storage
        .get_workspace(&workspace_id)
        .await
        .map_err(storage_error)?
        .is_none()
    {
        return Err(storage_error(StorageError::NotFound("workspace")));
    }
    let items = fems_memory::list_memory_items(&state.surreal, &workspace_id)
        .await
        .map_err(storage_error)?;
    let mut count = 0i64;
    for item in items {
        let item = serde_json::from_value::<crate::ace::MemoryPackItem>(item)
            .map_err(|error| storage_error(StorageError::Serialization(error.to_string())))?;
        if memory_source_refs_visible(&state, &workspace_id, &item.source_refs).await? {
            count += 1;
        }
    }
    Ok(Json(CommittedMemoryCount {
        workspace_id,
        count,
    }))
}

async fn get_memory_proposal(
    State(state): State<AppState>,
    Path((workspace_id, proposal_id)): Path<(String, String)>,
) -> Result<Json<fems_memory::StoredMemoryProposal>, ApiError> {
    let proposal = fems_memory::get_memory_proposal(&state.surreal, &proposal_id)
        .await
        .map_err(storage_error)?
        .ok_or_else(|| storage_error(StorageError::NotFound("memory proposal")))?;
    if proposal.workspace_id != workspace_id {
        return Err(storage_error(StorageError::NotFound(
            "memory proposal in workspace",
        )));
    }
    if !memory_proposal_sources_visible(&state, &proposal).await? {
        return Err(storage_error(StorageError::NotFound(
            "memory proposal in workspace",
        )));
    }
    Ok(Json(proposal))
}

async fn get_memory_proposal_artifact(
    State(state): State<AppState>,
    Path((workspace_id, proposal_id)): Path<(String, String)>,
) -> Result<Json<Value>, ApiError> {
    let proposal = fems_memory::get_memory_proposal(&state.surreal, &proposal_id)
        .await
        .map_err(storage_error)?
        .filter(|proposal| proposal.workspace_id == workspace_id)
        .ok_or_else(|| storage_error(StorageError::NotFound("memory proposal in workspace")))?;
    if !memory_proposal_sources_visible(&state, &proposal).await? {
        return Err(storage_error(StorageError::NotFound(
            "memory proposal in workspace",
        )));
    }
    // MT-118: a row read back from storage is by definition pre-existing, so this is the
    // read side of the same recovery the retry path performs. Resolving the artifact through
    // ONE definition keeps the existing invariant true for pre-hardening rows as well: the
    // `artifact://sha256/<hash>` that FR-EVT-MEM-001 publishes is the hash of exactly the
    // artifact this endpoint returns. Nothing is written back; the heal stays in memory.
    Ok(Json(
        fems_memory::proposal_canonical_artifact(&proposal, fems_memory::LegacyArtifactHeal::Allow)
            .value,
    ))
}

fn bad_request(detail: impl Into<String>) -> ApiError {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({"error": "bad_request", "detail": detail.into()})),
    )
}

fn storage_error(err: StorageError) -> ApiError {
    match err {
        StorageError::Guard("HSK-MEM-SOURCE-AUTHORITY") => crate::api::authority::constant_denial(),
        StorageError::NotFound(what) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "not_found", "detail": what})),
        ),
        StorageError::Validation(detail) => bad_request(detail),
        StorageError::Conflict(detail) | StorageError::ConflictDetails { code: detail, .. } => (
            StatusCode::CONFLICT,
            Json(json!({"error": "conflict", "detail": detail})),
        ),
        other => {
            tracing::error!(
                target: "handshake_core::memory_api",
                error = %other,
                "fems_memory_api_internal_error"
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "internal_error"})),
            )
        }
    }
}

fn memory_proposal_source_refs(
    proposal: &fems_memory::StoredMemoryProposal,
) -> Result<Vec<FemsSourceRef>, ApiError> {
    let artifact =
        fems_memory::proposal_canonical_artifact(proposal, fems_memory::LegacyArtifactHeal::Allow)
            .value;
    let proposal = serde_json::from_value::<MemoryWriteProposal>(artifact)
        .map_err(|error| storage_error(StorageError::Serialization(error.to_string())))?;
    if proposal.source_refs.is_empty() {
        return Err(storage_error(StorageError::Conflict(
            "memory proposal has no source references",
        )));
    }
    Ok(proposal.source_refs)
}

async fn memory_proposal_sources_visible(
    state: &AppState,
    proposal: &fems_memory::StoredMemoryProposal,
) -> Result<bool, ApiError> {
    let source_refs = memory_proposal_source_refs(proposal)?;
    memory_source_refs_visible(state, &proposal.workspace_id, &source_refs).await
}

async fn memory_source_refs_visible(
    state: &AppState,
    workspace_id: &str,
    source_refs: &[FemsSourceRef],
) -> Result<bool, ApiError> {
    if source_refs.is_empty() {
        return Ok(false);
    }
    for source_ref in source_refs {
        if source_ref.kind != FemsSourceRefKind::DocBlock
            || !memory_document_source_visible(state, workspace_id, &source_ref.id).await?
        {
            return Ok(false);
        }
    }
    Ok(true)
}

async fn memory_document_source_visible(
    state: &AppState,
    workspace_id: &str,
    document_id: &str,
) -> Result<bool, ApiError> {
    let database = SurrealDatabase::new(state.surreal.clone());
    if let Some(document) = database
        .get_knowledge_rich_document(document_id)
        .await
        .map_err(storage_error)?
    {
        return Ok(document.workspace_id == workspace_id);
    }
    if let Some(source) = database
        .get_knowledge_source(document_id)
        .await
        .map_err(storage_error)?
    {
        if source.workspace_id != workspace_id || source.source_kind != KnowledgeSourceKind::File {
            return Ok(false);
        }
        return Ok(database
            .get_knowledge_code_file_by_source(document_id)
            .await
            .map_err(storage_error)?
            .is_some_and(|file| file.workspace_id == workspace_id));
    }
    match state
        .storage
        .get_loom_block(workspace_id, document_id)
        .await
    {
        Ok(block) => Ok(block.workspace_id == workspace_id),
        Err(StorageError::NotFound(_)) => Ok(false),
        Err(error) => Err(storage_error(error)),
    }
}

fn header_str<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

#[derive(Debug, Clone)]
struct CanonicalActorIdentity {
    actor_id: String,
    actor_kind: &'static str,
    kernel_actor: KernelActor,
}

fn canonical_actor_identity(
    headers: &HeaderMap,
    body_actor_id: Option<&str>,
    fallback_actor_id: Option<&str>,
) -> Result<CanonicalActorIdentity, ApiError> {
    let actor_id = body_actor_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| header_str(headers, HSK_HEADER_ACTOR_ID))
        .or(fallback_actor_id)
        .ok_or_else(|| bad_request("x-hsk-actor-id is required"))?;
    if actor_id.len() > 200
        || !actor_id
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':'))
    {
        return Err(bad_request(
            "actor_id must be 1..=200 safe identifier characters",
        ));
    }
    let actor_id = actor_id.to_owned();
    match header_str(headers, HSK_HEADER_ACTOR_KIND).unwrap_or("human") {
        "operator" | "human" => Ok(CanonicalActorIdentity {
            actor_id: actor_id.clone(),
            actor_kind: "operator",
            kernel_actor: KernelActor::Operator(actor_id),
        }),
        "system" | "policy" => Ok(CanonicalActorIdentity {
            actor_id: actor_id.clone(),
            actor_kind: "system",
            kernel_actor: KernelActor::System(actor_id),
        }),
        _ => Err(bad_request(
            "x-hsk-actor-kind must be human, operator, system, or policy",
        )),
    }
}

// ---------------------------------------------------------------------------
// GET /workspaces/{workspace_id}/memory/pack  (AC-109-2)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Default)]
struct PackQuery {
    /// The retrieval context key the native client keys the capsule on (echoed by the
    /// frontend `MemoryClient`). Used as the scope key when present.
    #[serde(default)]
    context: Option<String>,
    /// Explicit scope key override (takes precedence over `context`).
    #[serde(default)]
    scope_key: Option<String>,
    /// Context detail supplied by the native editor. These fields shape future retrieval but never
    /// become persistence keys and never cause a write on this GET route.
    #[serde(default)]
    document_id: Option<String>,
    #[serde(default)]
    selection_text: Option<String>,
    #[serde(default)]
    cursor_byte: Option<u64>,
}

fn validated_pack_scope(query: &PackQuery) -> Result<Option<&str>, ApiError> {
    const MAX_CONTEXT_BYTES: usize = 1_024;
    const MAX_SCOPE_KEY_BYTES: usize = 256;
    const MAX_DOCUMENT_ID_BYTES: usize = 256;
    const MAX_SELECTION_TEXT_BYTES: usize = 2_048;

    let context = query.context.as_deref().map(str::trim);
    if context.is_some_and(|value| value.len() > MAX_CONTEXT_BYTES || value.contains('\0')) {
        return Err(bad_request(
            "memory pack context must be at most 1024 bytes and contain no NUL",
        ));
    }
    let scope_key = query.scope_key.as_deref().map(str::trim);
    if scope_key.is_some_and(|value| value.len() > MAX_SCOPE_KEY_BYTES || value.contains('\0')) {
        return Err(bad_request(
            "memory pack scope_key must be at most 256 bytes and contain no NUL",
        ));
    }
    let document_id = query.document_id.as_deref().map(str::trim);
    if document_id.is_some_and(|value| value.len() > MAX_DOCUMENT_ID_BYTES || value.contains('\0'))
    {
        return Err(bad_request(
            "memory pack document_id must be at most 256 bytes and contain no NUL",
        ));
    }
    if query
        .selection_text
        .as_deref()
        .is_some_and(|value| value.len() > MAX_SELECTION_TEXT_BYTES || value.contains('\0'))
    {
        return Err(bad_request(
            "memory pack selection_text must be at most 2048 bytes and contain no NUL",
        ));
    }
    let _ = query.cursor_byte;
    Ok(scope_key
        .filter(|value| !value.is_empty())
        .or_else(|| context.filter(|value| !value.is_empty())))
}

/// Return the REAL `ace::MemoryPack` for a workspace. When no pack has been stored yet,
/// return a well-formed EMPTY pack (200) rather than a 404 so the native client never
/// mistakes an empty capsule for a missing route.
async fn get_memory_pack(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    Query(query): Query<PackQuery>,
) -> Result<Json<MemoryPack>, ApiError> {
    let scope = validated_pack_scope(&query)?;

    let workspace = state
        .storage
        .get_workspace(&workspace_id)
        .await
        .map_err(storage_error)?
        .ok_or_else(|| storage_error(StorageError::NotFound("workspace")))?;

    let pack = fems_memory::get_latest_memory_pack(&state.surreal, &workspace_id, scope)
        .await
        .map_err(storage_error)?;
    if let Some(mut pack) = pack {
        let stored_hash = pack
            .compute_hash()
            .map_err(|error| storage_error(StorageError::Serialization(error.to_string())))?;
        if stored_hash != pack.memory_pack_hash {
            return Err(storage_error(StorageError::Conflict(
                "stored memory pack failed canonical hash validation",
            )));
        }
        if pack.schema_version == "fems.memory_pack@0.1" {
            // Compatibility is a response-only projection. GET is side-effect-free: migrations own
            // stored-envelope repair, so a caller cannot turn reads into UPDATE amplification.
            pack.schema_version = PACK_SCHEMA_VERSION.to_owned();
            pack.memory_pack_hash = pack
                .compute_hash()
                .map_err(|error| storage_error(StorageError::Serialization(error.to_string())))?;
        } else if pack.schema_version != PACK_SCHEMA_VERSION {
            return Err(storage_error(StorageError::Conflict(
                "stored memory pack uses an unsupported schema version",
            )));
        }
        let original_item_count = pack.items.len();
        let mut visible_items = Vec::with_capacity(original_item_count);
        for item in pack.items {
            if memory_source_refs_visible(&state, &workspace_id, &item.source_refs).await? {
                visible_items.push(item);
            }
        }
        if visible_items.len() != original_item_count {
            if visible_items.is_empty() {
                let scope_key = scope
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .unwrap_or("");
                return Ok(Json(empty_memory_pack(&workspace, scope_key)?));
            }
            pack.items = visible_items;
            pack.token_estimate = pack
                .items
                .iter()
                .map(|item| {
                    ((item.summary.chars().count() + item.content.chars().count() + 3) / 4) as u32
                })
                .sum::<u32>()
                .min(pack.budgets.max_tokens);
            pack.warnings.clear();
            let visible_ids = pack
                .items
                .iter()
                .map(|item| item.memory_id.as_str())
                .collect::<Vec<_>>()
                .join("\0");
            pack.pack_id = deterministic_uuid_from_seed(&format!(
                "fems-visible-memory-pack:{workspace_id}:{}:{visible_ids}",
                pack.pack_id
            ))
            .to_string();
            pack.memory_pack_hash.clear();
            pack.memory_pack_hash = pack
                .compute_hash()
                .map_err(|error| storage_error(StorageError::Serialization(error.to_string())))?;
        } else {
            pack.items = visible_items;
        }
        return Ok(Json(pack));
    }

    // Return one deterministic empty projection without persisting it. Repeated reads and restarts
    // derive the same value from the canonical workspace identity, while attacker-chosen contexts
    // cannot amplify embedded-store rows through a GET.
    let scope_key = scope
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("");
    let pack = empty_memory_pack(&workspace, scope_key)?;
    Ok(Json(pack))
}

/// A well-formed empty `ace::MemoryPack` (no items) for the neutral "no relevant memory"
/// state. Carries the real shape so the client decode path is identical to a full pack.
fn empty_memory_pack(
    workspace: &crate::storage::Workspace,
    scope_key: &str,
) -> Result<MemoryPack, ApiError> {
    let workspace_uuid = Uuid::parse_str(&workspace.id)
        .map_err(|_| bad_request("workspace id is not a canonical UUID"))?;
    let scope_ref = FemsEntityRef {
        artefact_type: "workspace".to_owned(),
        artefact_id: workspace_uuid,
        selector: if scope_key.is_empty() {
            "workspace".to_owned()
        } else {
            format!("scope_key:{scope_key}")
        },
    };
    let mut pack = MemoryPack {
        schema_version: PACK_SCHEMA_VERSION.to_string(),
        pack_id: deterministic_uuid_from_seed(&format!(
            "fems-empty-pack:{}:{scope_key}",
            workspace.id
        ))
        .to_string(),
        generated_at: workspace.created_at.to_rfc3339(),
        determinism_mode: MemoryPackDeterminismMode::Strict,
        memory_policy: MemoryPolicy::WorkspaceScoped,
        scope_refs: vec![scope_ref],
        budgets: MemoryPackBudgets {
            max_tokens: 500,
            max_items: 24,
            max_items_per_type: std::collections::BTreeMap::new(),
        },
        items: Vec::new(),
        token_estimate: 0,
        memory_pack_hash: String::new(),
        warnings: Vec::new(),
    };
    pack.memory_pack_hash = pack.compute_hash().map_err(|error| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "memory_pack_hash_failed", "detail": error.to_string()})),
        )
    })?;
    Ok(pack)
}

// ---------------------------------------------------------------------------
// POST /workspaces/{workspace_id}/memory/proposals  (AC-109-3)
// ---------------------------------------------------------------------------

/// The Pillar 12 memory class a proposal targets (bounded vocabulary — unknown classes
/// are rejected at decode time). Mirrors the native `MemoryClass` wire strings.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum ProposalClass {
    Episodic,
    Semantic,
    Procedural,
}

impl ProposalClass {
    fn wire(self) -> &'static str {
        match self {
            ProposalClass::Episodic => "episodic",
            ProposalClass::Semantic => "semantic",
            ProposalClass::Procedural => "procedural",
        }
    }
}

/// The source provenance a proposal MUST carry. Mirrors the native
/// `MemorySourceProvenance` exactly (closed field set — `deny_unknown_fields`).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ProposalSource {
    document_id: String,
    selection_start: u64,
    selection_end: u64,
    content_hash: String,
    #[serde(default)]
    document_content_hash: Option<String>,
    #[serde(default)]
    pane_id: Option<String>,
    #[serde(default)]
    workspace_id: Option<String>,
}

/// The review-gated proposal write body. Closed field set (`deny_unknown_fields`) matching
/// the native `submit_proposal` body so unknown/free-text smuggling is rejected.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ProposalRequest {
    /// Stable client request identity. Current native clients that predate this field get a deterministic
    /// identity derived from the closed request body; explicit identities allow retries across processes.
    #[serde(default)]
    request_id: Option<String>,
    class: ProposalClass,
    content: String,
    source: ProposalSource,
    /// Transient full code buffer used only to authenticate a `KSRC-*` source and selected slice.
    /// This field is deliberately omitted from the stored proposal payload below.
    #[serde(default)]
    source_document_content: Option<String>,
    #[serde(default)]
    review_gated: Option<bool>,
    #[serde(default)]
    actor_id: Option<String>,
}

/// The server acknowledgement of a stored proposal (matches the native `ProposalAck`).
///
/// `PartialEq` is derived so an idempotent retry can be compared field-for-field against the first
/// acknowledgement (WP-KERNEL-012 MT-144); comparing the serialized forms instead would let a field
/// reordering pass as convergence.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
struct ProposalAck {
    proposal_id: String,
    status: String,
    created_at: chrono::DateTime<chrono::Utc>,
    flight_recorder_event_id: Uuid,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ProposalReviewDecision {
    Approved,
    Rejected,
}

impl ProposalReviewDecision {
    fn wire(self) -> &'static str {
        match self {
            Self::Approved => "approved",
            Self::Rejected => "rejected",
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ProposalReviewerKind {
    User,
    Policy,
}

impl ProposalReviewerKind {
    fn wire(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Policy => "policy",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProposalReviewRequest {
    decision: ProposalReviewDecision,
    reviewer_kind: ProposalReviewerKind,
    #[serde(default)]
    reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct ProposalReviewAck {
    proposal_id: String,
    status: String,
    decision: ProposalReviewDecision,
    reviewer_kind: ProposalReviewerKind,
    actor_id: String,
    correlation_id: String,
    event_ledger_event_id: String,
    flight_recorder_event_id: Uuid,
    reviewed_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize)]
struct ProposalCommitAck {
    proposal_id: String,
    status: String,
    commit_id: String,
    memory_id: String,
    memory_pack_id: String,
    memory_pack_hash: String,
    commit_report: MemoryCommitReport,
    commit_report_hash: String,
    event_ledger_event_id: String,
    flight_recorder_event_id: Uuid,
    committed_at: String,
}

/// THE canonical FEMS memory-proposal request identity (WP-KERNEL-012 MT-112, AC-112-1).
///
/// SHA-256 over the domain tag `fems-memory-proposal-request-v2` + NUL, followed by
/// ELEVEN length-prefixed components (big-endian u64 byte length, then UTF-8 bytes) in
/// exactly this order:
///
///  1. `workspace_id`                            (route workspace, not trimmed)
///  2. `class.wire()`                            (not trimmed)
///  3. `content`                                 (not trimmed)
///  4. `source.document_id`                      (`str::trim`)
///  5. `source.selection_start`                  (decimal text)
///  6. `source.selection_end`                    (decimal text)
///  7. `source.content_hash`                     (`str::trim`)
///  8. `source.document_content_hash`            (`normalized_optional`)
///  9. `source.pane_id`                          (`normalized_optional`)
/// 10. `source.workspace_id`                     (`normalized_optional`)
/// 11. `sha256_hex(source_document_content)`     (absent => "")
///
/// `actor_id` is DELIBERATELY EXCLUDED. `same_logical_proposal` in
/// `storage::fems_memory` strips `actor_id` from intake replay equality because the
/// router derives it from the live native binding, so an exact retry from a later
/// authenticated session must converge on the same row. Hashing `actor_id` into the
/// identity would fork that retry into a duplicate proposal and contradict the retry
/// contract this function exists to serve. Attribution is not weakened: `actor_id`
/// stays in the stored proposal payload and in the immutable `ARTIFACT_PROPOSED`
/// EventLedger receipt (`KernelActor`), which is where attribution is authoritative.
///
/// AC-112-3 - components 8 and 11 always carry the SAME value, and that is intentional,
/// not a duplicated component. The intake gate below makes it an invariant: the
/// canonical-code branch rejects any proposal where
/// `sha256_hex(source_document_content) != source.document_content_hash`, and the
/// rich-document and Loom-reference branches reject a proposal that carries either
/// field, leaving both components "". The declarative schema contract hashes
/// `document_content_hash`
/// twice for exactly this reason - a stored proposal row never persists
/// `source_document_content`, so `document_content_hash` is the only SQL-derivable
/// expression of component 11. That second occurrence is load-bearing and is retained;
/// only the extra `actor_id` component was dropped when 0365 superseded 0345.
///
/// The canonical identity contract lives with this Rust implementation and the
/// matching fields in `storage/surreal/schema.surql`. Do NOT hand-copy this expression
/// into a third site.
fn stable_proposal_request_id(
    workspace_id: &str,
    request: &ProposalRequest,
) -> Result<String, ApiError> {
    if let Some(explicit) = request.request_id.as_deref() {
        let explicit = explicit.trim();
        if explicit.is_empty()
            || explicit.len() > 200
            || !explicit
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':'))
        {
            return Err(bad_request(
                "request_id must be 1..=200 safe identifier characters",
            ));
        }
        return Ok(explicit.to_owned());
    }
    let mut hasher = Sha256::new();
    hasher.update(b"fems-memory-proposal-request-v2\0");
    let selection_start = request.source.selection_start.to_string();
    let selection_end = request.source.selection_end.to_string();
    let source_document_hash = request
        .source_document_content
        .as_deref()
        .map(|content| hex::encode(Sha256::digest(content.as_bytes())))
        .unwrap_or_default();
    for component in [
        workspace_id,
        request.class.wire(),
        request.content.as_str(),
        request.source.document_id.trim(),
        selection_start.as_str(),
        selection_end.as_str(),
        request.source.content_hash.trim(),
        normalized_optional(request.source.document_content_hash.as_deref()),
        normalized_optional(request.source.pane_id.as_deref()),
        normalized_optional(request.source.workspace_id.as_deref()),
        source_document_hash.as_str(),
    ] {
        let bytes = component.as_bytes();
        hasher.update((bytes.len() as u64).to_be_bytes());
        hasher.update(bytes);
    }
    Ok(format!("derived-sha256:{}", hex::encode(hasher.finalize())))
}

fn normalized_optional(value: Option<&str>) -> &str {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("")
}

fn canonical_content_hash(content: &str) -> Result<String, ApiError> {
    let canonical =
        crate::kernel::context_bundle::canonical_json_bytes(&Value::String(content.to_owned()));
    Ok(hex::encode(Sha256::digest(canonical)))
}

fn stable_proposal_id(workspace_id: &str, request_id: &str) -> String {
    deterministic_uuid_from_seed(&format!(
        "fems-memory-proposal-v2:{workspace_id}:{request_id}"
    ))
    .to_string()
}

/// Store a review-gated proposal. Fail-closed on missing provenance (`document_id` +
/// selection range + 64-hex `content_hash`). The proposal is stored as `pending_review`
/// and a durable `ARTIFACT_PROPOSED` EventLedger receipt is appended. There is NO path
/// from here to a committed memory item.
async fn create_memory_proposal(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    headers: HeaderMap,
    Json(mut request): Json<ProposalRequest>,
) -> Result<Json<ProposalAck>, ApiError> {
    if state
        .storage
        .get_workspace(&workspace_id)
        .await
        .map_err(storage_error)?
        .is_none()
    {
        return Err(storage_error(StorageError::NotFound("workspace")));
    }

    // The router middleware replaces all caller-authored identity headers with the identity derived
    // from the live native MCP binding. The body actor is display metadata only and must never
    // override that authenticated principal.
    let actor = canonical_actor_identity(&headers, None, Some("native_editor"))?;
    request.actor_id = Some(actor.actor_id.clone());

    // Fail-closed provenance gate (AC-109-3).
    let document_id = request.source.document_id.trim();
    if document_id.is_empty() {
        return Err(bad_request(
            "proposal provenance requires a non-empty document_id",
        ));
    }
    if request.review_gated != Some(true) {
        return Err(bad_request(
            "memory proposals require review_gated=true; false or omitted gating is rejected",
        ));
    }
    if request.content.is_empty() {
        return Err(bad_request("memory proposal content must not be empty"));
    }
    if request.source.selection_end <= request.source.selection_start {
        return Err(bad_request(
            "proposal provenance selection range is invalid (selection_end <= selection_start)",
        ));
    }
    let selected_byte_len = request.source.selection_end - request.source.selection_start;
    if selected_byte_len != request.content.len() as u64 {
        return Err(bad_request(
            "proposal selection byte range must exactly match the UTF-8 content byte length",
        ));
    }
    let selection_start = i64::try_from(request.source.selection_start)
        .map_err(|_| bad_request("proposal selection_start exceeds the supported range"))?;
    let selection_end = i64::try_from(request.source.selection_end)
        .map_err(|_| bad_request("proposal selection_end exceeds the supported range"))?;
    if request.source.workspace_id.as_deref() != Some(workspace_id.as_str()) {
        return Err(bad_request(
            "proposal source.workspace_id is required and must exactly match the route workspace_id",
        ));
    }
    let content_hash = request.source.content_hash.trim();
    if !is_content_hash(content_hash) {
        return Err(bad_request(
            "proposal provenance requires a 64-char lowercase hex content_hash",
        ));
    }
    if canonical_content_hash(&request.content)? != content_hash {
        return Err(bad_request(
            "proposal content_hash does not match the canonical JSON-string SHA-256 of content",
        ));
    }
    let db = SurrealDatabase::new(state.surreal.clone());
    let document = db
        .get_knowledge_rich_document(document_id)
        .await
        .map_err(storage_error)?;
    match document {
        Some(document) => {
            if request.source.document_content_hash.is_some()
                || request.source_document_content.is_some()
            {
                return Err(bad_request(
                    "rich-document proposals must not include code-only document snapshot fields",
                ));
            }
            if document.workspace_id != workspace_id {
                return Err(bad_request(
                    "proposal provenance document does not belong to the route workspace",
                ));
            }
            let document_text =
                crate::knowledge_document::block_tree::extract_plain_text(&document.content_json);
            let start = usize::try_from(request.source.selection_start).map_err(|_| {
                bad_request("proposal selection_start exceeds the document text range")
            })?;
            let end = usize::try_from(request.source.selection_end).map_err(|_| {
                bad_request("proposal selection_end exceeds the document text range")
            })?;
            let Some(selected) = document_text.get(start..end) else {
                return Err(bad_request(
                    "proposal selection range is not a valid UTF-8 range in the canonical document",
                ));
            };
            if selected != request.content {
                return Err(bad_request(
                    "proposal content does not match the canonical document selection",
                ));
            }
        }
        None => {
            let source = db
                .get_knowledge_source(document_id)
                .await
                .map_err(storage_error)?;
            match source {
                Some(source) => {
                    if source.workspace_id != workspace_id {
                        return Err(bad_request(
                            "proposal provenance code source does not belong to the route workspace",
                        ));
                    }
                    if source.source_kind != KnowledgeSourceKind::File {
                        return Err(bad_request(
                            "proposal provenance source is not a canonical code file",
                        ));
                    }
                    if source.stale {
                        return Err(bad_request(
                            "proposal provenance code source is stale and must be re-indexed",
                        ));
                    }
                    let code_file = db
                        .get_knowledge_code_file_by_source(document_id)
                        .await
                        .map_err(storage_error)?
                        .ok_or_else(|| {
                            bad_request(
                                "proposal provenance source has no canonical code-file index record",
                            )
                        })?;
                    if code_file.workspace_id != workspace_id
                        || code_file.stale
                        || code_file.parse_status.as_str() == "failed"
                    {
                        return Err(bad_request(
                            "proposal provenance code-file index is not current and usable",
                        ));
                    }
                    if code_file.indexed_content_hash != source.content_hash {
                        return Err(bad_request(
                            "proposal provenance code source and code-file index hashes disagree",
                        ));
                    }
                    let document_content_hash = request
                        .source
                        .document_content_hash
                        .as_deref()
                        .map(str::trim)
                        .filter(|hash| is_content_hash(hash))
                        .ok_or_else(|| {
                            bad_request(
                                "canonical code proposals require source.document_content_hash",
                            )
                        })?;
                    let document_content =
                        request.source_document_content.as_deref().ok_or_else(|| {
                            bad_request("canonical code proposals require source_document_content")
                        })?;
                    let submitted_document_hash =
                        hex::encode(Sha256::digest(document_content.as_bytes()));
                    if submitted_document_hash != document_content_hash
                        || submitted_document_hash != source.content_hash
                    {
                        return Err(bad_request(
                            "proposal code snapshot hash does not match canonical KnowledgeSource content",
                        ));
                    }
                    let start = usize::try_from(request.source.selection_start).map_err(|_| {
                        bad_request("proposal selection_start exceeds the code document range")
                    })?;
                    let end = usize::try_from(request.source.selection_end).map_err(|_| {
                        bad_request("proposal selection_end exceeds the code document range")
                    })?;
                    let Some(selected) = document_content.get(start..end) else {
                        return Err(bad_request(
                            "proposal selection range is not a valid UTF-8 range in the canonical code document",
                        ));
                    };
                    if selected != request.content {
                        return Err(bad_request(
                            "proposal content does not match the canonical code document selection",
                        ));
                    }
                }
                None => {
                    // BlockRef/NodeRef selections deliberately carry a canonical Loom address rather
                    // than materialized block text. Accept that address only when the referenced block
                    // exists in this exact workspace; arbitrary loom:// strings remain rejected.
                    // AC-112-5: `document_id` is CLIENT-SUPPLIED PROVENANCE being
                    // validated, not the route resource. The route resource is the
                    // workspace, and it is checked at the top of this handler where a
                    // missing workspace is (and stays) 404. Once we are here the
                    // document_id has resolved to no rich document, no canonical code
                    // source and no Loom block, which is the same fail-closed
                    // provenance rejection every other gate in this handler answers
                    // with 400. Letting `StorageError::NotFound` reach `storage_error`
                    // returned 404 and made "your provenance is bad" indistinguishable
                    // from "this workspace is gone", so the provenance rejection is
                    // normalized to 400 here. Fail-closed behavior is unchanged -
                    // nothing is stored either way; only the status contract is fixed.
                    let loom_block = match state
                        .storage
                        .get_loom_block(&workspace_id, document_id)
                        .await
                    {
                        Ok(block) => block,
                        Err(StorageError::NotFound(_)) => {
                            // WP-KERNEL-012 MT-146 D-146-1. Provenance can also fail to resolve
                            // because the route workspace was deleted while this request was in
                            // flight: the cascade takes the workspace's rich documents, code
                            // sources and Loom blocks with it, so every provenance lookup above
                            // misses. That is the route resource disappearing, not bad
                            // provenance, and this handler's contract answers a missing workspace
                            // with 404. Re-check before falling back to the 400.
                            if state
                                .storage
                                .get_workspace(&workspace_id)
                                .await
                                .map_err(storage_error)?
                                .is_none()
                            {
                                return Err(storage_error(StorageError::NotFound("workspace")));
                            }
                            return Err(bad_request(
                                "proposal provenance document_id does not resolve to a rich document, a canonical code source, or a Loom block in this workspace",
                            ));
                        }
                        Err(other) => return Err(storage_error(other)),
                    };
                    let canonical_ref = format!("loom://{document_id}");
                    if loom_block.workspace_id != workspace_id
                        || request.source_document_content.is_some()
                        || request.source.document_content_hash.is_some()
                        || request.source.selection_start != 0
                        || request.source.selection_end != canonical_ref.len() as u64
                        || request.content != canonical_ref
                    {
                        return Err(bad_request(
                            "Loom reference proposal must carry the exact canonical whole-block address",
                        ));
                    }
                }
            }
        }
    }

    let request_id = stable_proposal_request_id(&workspace_id, &request)?;
    let proposal_id = stable_proposal_id(&workspace_id, &request_id);
    // The commit is downstream + review-gated: the server ALWAYS records the proposal as
    // pending review and never trusts a client flag to bypass the gate.
    let review_gated = true;

    // The embedded store persists microseconds. Normalize before hashing/storing so the
    // content-addressed proposal artifact and durable proposal row retain one byte-identical instant.
    let created_at = chrono::DateTime::from_timestamp_micros(chrono::Utc::now().timestamp_micros())
        .ok_or_else(|| {
            storage_error(StorageError::Serialization("invalid proposal time".into()))
        })?;
    let kernel_task_run_id = header_str(&headers, HSK_HEADER_KERNEL_TASK_RUN_ID)
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("native-editor-fems-propose-{workspace_id}"));
    let session_run_id = header_str(&headers, HSK_HEADER_SESSION_RUN_ID)
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| "native-editor-session".to_string());
    let workspace_uuid =
        Uuid::parse_str(&workspace_id).map_err(|_| bad_request("workspace id is not a UUID"))?;
    let scope_refs = vec![FemsEntityRef {
        artefact_type: "workspace".to_owned(),
        artefact_id: workspace_uuid,
        selector: "self".to_owned(),
    }];
    let source_refs = vec![FemsSourceRef {
        kind: FemsSourceRefKind::DocBlock,
        id: document_id.to_owned(),
        hash: Some(content_hash.to_owned()),
        selector: Some(format!("bytes:{selection_start}-{selection_end}")),
        created_at: Some(created_at.to_rfc3339()),
        classification: Some("low".to_owned()),
    }];
    let canonical_proposal = MemoryWriteProposal {
        schema_version: "hsk.memory_write_proposal@0.1".to_owned(),
        proposal_id: proposal_id.clone(),
        created_at: created_at.to_rfc3339(),
        created_by_job_id: kernel_task_run_id.clone(),
        scope_refs: scope_refs.clone(),
        source_refs: source_refs.clone(),
        policy: MemoryWritePolicy {
            allow_procedural: request.class == ProposalClass::Procedural,
            require_human_review: true,
            max_ops: 1,
        },
        ops: vec![MemoryWriteOp {
            op: MemoryMutationOp::Add,
            temp_id: Some("m1".to_owned()),
            memory_id: None,
            item: PartialMemoryItem {
                memory_class: Some(request.class.wire().to_owned()),
                item_type: Some(
                    match request.class {
                        ProposalClass::Procedural => "tool_protocol",
                        ProposalClass::Episodic => "intent",
                        ProposalClass::Semantic => "fact",
                    }
                    .to_owned(),
                ),
                scope_refs: Some(scope_refs),
                content: Some(request.content.clone()),
                confidence: Some(1.0),
                trust_level: Some("user_asserted".to_owned()),
                provenance: Some(crate::ace::MemoryItemProvenance {
                    source_refs,
                    created_by_job_id: kernel_task_run_id.clone(),
                }),
                classification: Some("low".to_owned()),
                ..PartialMemoryItem::default()
            },
            rationale: "Editor selection proposed from source_refs[0]".to_owned(),
            confidence: 1.0,
            requires_review: true,
        }],
    };
    let canonical_artifact = serde_json::to_value(&canonical_proposal)
        .map_err(|error| storage_error(StorageError::Serialization(error.to_string())))?;
    let proposal_payload = json!({
        "_canonical_artifact": canonical_artifact,
        "proposal_id": proposal_id,
        "request_id": request_id,
        "workspace_id": workspace_id,
        "class": request.class.wire(),
        "content": request.content,
        "source": request.source,
        "review_gated": review_gated,
        "status": PROPOSAL_STATUS_PENDING_REVIEW,
        "actor_id": actor.actor_id,
    });

    let stored = fems_memory::StoredMemoryProposal {
        proposal_id: proposal_id.clone(),
        request_id: request_id.clone(),
        workspace_id: workspace_id.clone(),
        document_id: document_id.to_string(),
        selection_start,
        selection_end,
        content_hash: content_hash.to_string(),
        memory_class: request.class.wire().to_string(),
        status: PROPOSAL_STATUS_PENDING_REVIEW.to_string(),
        review_gated,
        created_at,
        proposal: proposal_payload.clone(),
    };

    // Durable EventLedger receipt (embedded SurrealDB authority path). A review-gated
    // proposal is an ARTIFACT_PROPOSED event — it is explicitly NOT a commit.
    let correlation_id = format!("fems-memory-proposal:{proposal_id}");
    let receipt = NewKernelEvent::builder(
        kernel_task_run_id,
        session_run_id,
        KernelEventType::ArtifactProposed,
        actor.kernel_actor,
    )
    .aggregate("fems_memory_proposal", proposal_id.clone())
    .idempotency_key(format!("fems-memory-proposal:{proposal_id}"))
    .correlation_id(correlation_id)
    .source_component("fems_memory_proposal_intake")
    .payload(json!({
        "receipt_kind": "fems_memory_write_proposal",
        "proposal_id": proposal_id,
        "workspace_id": workspace_id,
        "document_id": document_id,
        "selection_start": selection_start,
        "selection_end": selection_end,
        "content_hash": content_hash,
        "memory_class": request.class.wire(),
        "review_gated": review_gated,
        "status": PROPOSAL_STATUS_PENDING_REVIEW,
        "never_editor_direct": true,
    }))
    .build()
    .map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "receipt_build_failed", "detail": err.to_string()})),
        )
    })?;

    let stored = fems_memory::insert_memory_proposal_with_receipt(&state.surreal, &stored, receipt)
        .await
        .map_err(storage_error)?;
    // An identical user retry is the explicit recovery signal for a quarantined projection. The
    // immutable proposal/outbox identity remains unchanged; only its bounded delivery attempts reset.
    fems_memory::requeue_quarantined_memory_lifecycle_event(
        &state.surreal,
        &stored.proposal_id,
        "FR-EVT-MEM-001",
    )
    .await
    .map_err(storage_error)?;
    reconcile_workspace_memory_commit_events(&state, &workspace_id).await?;
    match fems_memory::memory_lifecycle_publication_state(
        &state.surreal,
        &stored.proposal_id,
        "FR-EVT-MEM-001",
    )
    .await
    .map_err(storage_error)?
    {
        fems_memory::MemoryLifecyclePublicationState::Published => {}
        publication_state => {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({
                    "error": "flight_recorder_event_not_published",
                    "detail": format!(
                        "FR-EVT-MEM-001 is not durably published ({publication_state:?})"
                    ),
                    "proposal_id": stored.proposal_id,
                })),
            ));
        }
    }

    Ok(Json(ProposalAck {
        proposal_id: stored.proposal_id.clone(),
        status: stored.status,
        created_at: stored.created_at,
        flight_recorder_event_id: deterministic_uuid_from_seed(&format!(
            "fems-memory-proposal-event:{}",
            stored.proposal_id
        )),
    }))
}

async fn review_memory_proposal(
    State(state): State<AppState>,
    Path((workspace_id, proposal_id)): Path<(String, String)>,
    headers: HeaderMap,
    Json(request): Json<ProposalReviewRequest>,
) -> Result<Json<ProposalReviewAck>, ApiError> {
    let actor = canonical_actor_identity(&headers, None, None)?;
    match (request.reviewer_kind, actor.actor_kind) {
        (ProposalReviewerKind::User, "operator") | (ProposalReviewerKind::Policy, "system") => {}
        (ProposalReviewerKind::User, _) => {
            return Err(bad_request(
                "reviewer_kind=user requires a human/operator actor",
            ))
        }
        (ProposalReviewerKind::Policy, _) => {
            return Err(bad_request(
                "reviewer_kind=policy requires a system/policy actor",
            ))
        }
    }
    let reason = match request.reason {
        Some(reason) => {
            let reason = reason.trim();
            if reason.is_empty() || reason.len() > 1000 {
                return Err(bad_request(
                    "review reason, when present, must be 1..=1000 bytes",
                ));
            }
            Some(reason.to_owned())
        }
        None => None,
    };
    require_published_proposal_event(&state, &workspace_id, &proposal_id).await?;
    let correlation_id = format!("fems-memory-proposal-review:{proposal_id}");
    let kernel_task_run_id = header_str(&headers, HSK_HEADER_KERNEL_TASK_RUN_ID)
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("native-editor-fems-review-{workspace_id}"));
    let session_run_id = header_str(&headers, HSK_HEADER_SESSION_RUN_ID)
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| "native-editor-session".to_owned());
    let receipt = NewKernelEvent::builder(
        kernel_task_run_id,
        session_run_id,
        match request.decision {
            ProposalReviewDecision::Approved => KernelEventType::PromotionAccepted,
            ProposalReviewDecision::Rejected => KernelEventType::PromotionRejected,
        },
        actor.kernel_actor.clone(),
    )
    .aggregate("fems_memory_proposal", proposal_id.clone())
    .idempotency_key(format!("fems-memory-proposal-review:{proposal_id}"))
    .correlation_id(correlation_id.clone())
    .source_component("fems_memory_proposal_review")
    .payload(json!({"proposal_id": proposal_id, "decision": request.decision.wire()}))
    .build()
    .map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "receipt_build_failed", "detail": err.to_string()})),
        )
    })?;
    let review = fems_memory::MemoryProposalReview {
        decision: request.decision.wire().to_owned(),
        reviewer_kind: request.reviewer_kind.wire().to_owned(),
        actor_kind: actor.actor_kind.to_owned(),
        actor_id: actor.actor_id.clone(),
        reason,
        correlation_id: correlation_id.clone(),
    };
    let transition = fems_memory::review_memory_proposal_with_receipt(
        &state.surreal,
        &workspace_id,
        &proposal_id,
        &review,
        receipt,
    )
    .await
    .map_err(storage_error)?;

    let event_id =
        deterministic_uuid_from_seed(&format!("fems-memory-proposal-review-event:{proposal_id}"));
    reconcile_workspace_memory_commit_events(&state, &workspace_id).await?;

    Ok(Json(ProposalReviewAck {
        proposal_id,
        status: transition.proposal.status,
        decision: request.decision,
        reviewer_kind: request.reviewer_kind,
        actor_id: actor.actor_id,
        correlation_id,
        event_ledger_event_id: transition.receipt.event_id,
        flight_recorder_event_id: event_id,
        reviewed_at: transition.reviewed_at,
    }))
}

async fn commit_memory_proposal(
    State(state): State<AppState>,
    Path((workspace_id, proposal_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<Json<ProposalCommitAck>, ApiError> {
    let actor = canonical_actor_identity(&headers, None, None)?;
    require_published_proposal_event(&state, &workspace_id, &proposal_id).await?;
    let kernel_task_run_id = header_str(&headers, HSK_HEADER_KERNEL_TASK_RUN_ID)
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("native-editor-fems-commit-{workspace_id}"));
    let session_run_id = header_str(&headers, HSK_HEADER_SESSION_RUN_ID)
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| "native-editor-session".to_owned());
    let receipt = NewKernelEvent::builder(
        kernel_task_run_id,
        session_run_id,
        KernelEventType::ArtifactStored,
        actor.kernel_actor.clone(),
    )
    .aggregate("fems_memory_commit", proposal_id.clone())
    .idempotency_key(format!("fems-memory-commit:{proposal_id}"))
    .correlation_id(format!("fems-memory-proposal:{proposal_id}"))
    .source_component("fems_memory_proposal_commit")
    .payload(json!({"proposal_id": proposal_id}))
    .build()
    .map_err(|error| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "receipt_build_failed", "detail": error.to_string()})),
        )
    })?;
    let committed = fems_memory::commit_memory_proposal_with_receipt(
        &state.surreal,
        &workspace_id,
        &proposal_id,
        receipt,
    )
    .await
    .map_err(storage_error)?;
    let memory_id = committed
        .commit_report
        .applied_ops
        .first()
        .map(|operation| operation.memory_id.clone())
        .ok_or_else(|| {
            storage_error(StorageError::Conflict(
                "memory commit report has no applied operation",
            ))
        })?;
    let event_id = committed.flight_recorder_event.event_id;
    let committed_at = committed.flight_recorder_event.timestamp;
    // A successful response means the durable outbox has been projected. If Flight Recorder is
    // unavailable, the authoritative SurrealDB commit remains recoverable and this request fails
    // honestly; the startup projector retries it after process restart.
    reconcile_workspace_memory_commit_events(&state, &workspace_id).await?;

    Ok(Json(ProposalCommitAck {
        proposal_id,
        status: committed.proposal.status,
        commit_id: committed.commit_report.commit_id.clone(),
        memory_id,
        memory_pack_id: committed.memory_pack.pack_id.clone(),
        memory_pack_hash: committed.memory_pack.memory_pack_hash.clone(),
        commit_report: committed.commit_report,
        commit_report_hash: committed.commit_report_hash,
        event_ledger_event_id: committed.receipt.event_id,
        flight_recorder_event_id: event_id,
        committed_at: committed_at.to_rfc3339(),
    }))
}

fn deterministic_uuid_from_seed(seed: &str) -> Uuid {
    let digest = Sha256::digest(seed.as_bytes());
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}

fn same_review_event(left: &FlightRecorderEvent, right: &FlightRecorderEvent) -> bool {
    left.event_id == right.event_id
        && left.trace_id == right.trace_id
        // SurrealDB/DuckDB persist timestamps at microsecond precision. Compare the exact persisted
        // instant rather than chrono's unused sub-microsecond component so a read-back retry is stable.
        && left.timestamp.timestamp_micros() == right.timestamp.timestamp_micros()
        && left.actor == right.actor
        && left.actor_id == right.actor_id
        && left.event_type == right.event_type
        && left.job_id == right.job_id
        && left.workflow_id == right.workflow_id
        && left.model_id == right.model_id
        && left.model_session_id == right.model_session_id
        && left.wsids == right.wsids
        && left.activity_span_id == right.activity_span_id
        && left.session_span_id == right.session_span_id
        && left.capability_id == right.capability_id
        && left.policy_decision_id == right.policy_decision_id
        && left.payload == right.payload
}

async fn record_review_event_idempotent(
    state: &AppState,
    event: FlightRecorderEvent,
) -> Result<(), ApiError> {
    let existing = state
        .flight_recorder
        .list_events(EventFilter {
            event_id: Some(event.event_id),
            ..EventFilter::default()
        })
        .await
        .map_err(flight_recorder_error)?;
    if let Some(existing) = existing.first() {
        return if same_review_event(existing, &event) {
            Ok(())
        } else {
            Err(storage_error(StorageError::Conflict(
                "flight recorder review event identity is bound to different evidence",
            )))
        };
    }
    if let Err(write_error) = state.flight_recorder.record_event(event.clone()).await {
        let existing = state
            .flight_recorder
            .list_events(EventFilter {
                event_id: Some(event.event_id),
                ..EventFilter::default()
            })
            .await
            .map_err(flight_recorder_error)?;
        if existing
            .first()
            .is_some_and(|existing| same_review_event(existing, &event))
        {
            return Ok(());
        }
        return Err(flight_recorder_error(write_error));
    }
    Ok(())
}

fn flight_recorder_error(error: RecorderError) -> ApiError {
    tracing::error!(
        target: "handshake_core::memory_api",
        error = %error,
        "fems_memory_review_flight_recorder_error"
    );
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({"error": "flight_recorder_error"})),
    )
}

/// A content hash is provenance-valid iff it is exactly 64 lowercase hex chars (the loom
/// canonical SHA-256 primitive the native editor produces).
fn is_content_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
}

#[cfg(all(test, feature = "duckdb-flight-recorder"))]
mod tests {
    use super::*;
    use crate::ace::{FemsSourceRef, FemsSourceRefKind, MemoryPackItem};
    use crate::capabilities::CapabilityRegistry;
    use crate::flight_recorder::duckdb::DuckDbFlightRecorder;
    use crate::flight_recorder::FlightRecorder;
    use crate::llm::{
        CompletionRequest, CompletionResponse, LlmClient, LlmError, ModelProfile, TokenUsage,
    };
    use crate::storage::knowledge::NewKnowledgeRichDocument;
    use crate::storage::tests::embedded_test_backend;
    use crate::storage::{
        LoomBlockContentType, LoomBlockDerived, NewLoomBlock, NewWorkspace, WriteContext,
    };
    use crate::workflows::{SessionRegistry, SessionSchedulerConfig};
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    use surrealdb::types::{Datetime, RecordId, SurrealValue};
    use uuid::Uuid;

    #[derive(Clone, Default, SurrealValue)]
    struct MemoryTestBindings {
        workspace: Option<RecordId>,
        proposal: Option<RecordId>,
        value: Option<String>,
        other: Option<String>,
        hash: Option<String>,
        values: Vec<String>,
        at: Option<Datetime>,
    }

    #[derive(SurrealValue)]
    struct MemoryTestCountRow {
        count: i64,
    }

    #[derive(SurrealValue)]
    struct MemoryTestBoolRow {
        value: bool,
    }

    #[derive(SurrealValue)]
    struct MemoryTestStringRow {
        value: String,
    }

    #[derive(SurrealValue)]
    struct ReconciliationPrincipalProfileRow {
        capability_profile_id: String,
        actor_id: String,
    }

    #[derive(SurrealValue)]
    struct ReconciliationPrincipalProfileBinding {
        principal: RecordId,
    }

    // `Send` is required by `with_data_operation`, which drives the query on the storage runtime
    // (WP-KERNEL-012 MT-144).
    async fn memory_test_query_first<R: SurrealValue + Send + 'static>(
        state: &AppState,
        statement: &'static str,
        bindings: MemoryTestBindings,
    ) -> Result<Option<R>, Box<dyn std::error::Error>> {
        Ok(state
            .surreal
            .with_data_operation(move |database| {
                Box::pin(async move { database.query_first(statement, bindings).await })
            })
            .await?)
    }

    async fn memory_test_count(
        state: &AppState,
        statement: &'static str,
        bindings: MemoryTestBindings,
    ) -> Result<i64, Box<dyn std::error::Error>> {
        Ok(
            memory_test_query_first::<MemoryTestCountRow>(state, statement, bindings)
                .await?
                .map_or(0, |row| row.count),
        )
    }

    async fn memory_test_execute(
        state: &AppState,
        statement: &'static str,
        bindings: MemoryTestBindings,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        Ok(state
            .surreal
            .with_data_operation(move |database| {
                Box::pin(async move { database.execute_returning(statement, bindings).await })
            })
            .await?)
    }

    async fn mt109_workspace_ids_in_reconciliation_scope(
        state: &AppState,
        scope: crate::storage::surreal::resource_authority::RecordUserScope,
        workspace_ids: Vec<String>,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let bindings = MemoryTestBindings {
            values: workspace_ids,
            ..Default::default()
        };
        let rows = state
            .surreal
            .with_record_user_scope(
                scope,
                state.surreal.with_data_operation(move |database| {
                    Box::pin(async move {
                        database
                            .query_values::<MemoryTestStringRow, _>(
                                "SELECT record::id(id) AS value FROM workspaces WHERE record::id(id) IN $values ORDER BY value ASC;",
                                bindings,
                            )
                            .await
                    })
                }),
            )
            .await?;
        Ok(rows.into_iter().map(|row| row.value).collect())
    }

    async fn memory_test_ledger_event(
        state: &AppState,
        idempotency_key: &str,
    ) -> Result<Option<crate::kernel::KernelEvent>, Box<dyn std::error::Error>> {
        Ok(crate::storage::surreal::event_ledger::get_by_idempotency(
            &state.surreal,
            idempotency_key,
        )
        .await?)
    }

    // One shared guard for the process-global test binding env var: `api::flight_recorder`'s
    // authorization suite installs the same binding, and two suites racing on it would
    // authenticate against each other's token.
    use crate::api::stage::NATIVE_BINDING_ENV_LOCK as MEMORY_AUTH_ENV_LOCK;

    struct BindingEnvGuard {
        previous: Option<std::ffi::OsString>,
        path: std::path::PathBuf,
    }

    impl Drop for BindingEnvGuard {
        fn drop(&mut self) {
            if let Some(previous) = self.previous.take() {
                std::env::set_var("HANDSHAKE_STAGE_BINDING_FILE", previous);
            } else {
                std::env::remove_var("HANDSHAKE_STAGE_BINDING_FILE");
            }
            let _ = std::fs::remove_file(&self.path);
        }
    }

    struct TestLlmClient {
        profile: ModelProfile,
    }

    struct FailNextRecordFlightRecorder {
        inner: Arc<dyn FlightRecorder>,
        fail_next: AtomicBool,
    }

    struct FailAllRecordFlightRecorder {
        inner: Arc<dyn FlightRecorder>,
    }

    #[async_trait::async_trait]
    impl FlightRecorder for FailNextRecordFlightRecorder {
        async fn record_event(&self, event: FlightRecorderEvent) -> Result<(), RecorderError> {
            if self.fail_next.swap(false, Ordering::SeqCst) {
                return Err(RecorderError::SinkError(
                    "injected post-commit/pre-flight-recorder crash window".to_owned(),
                ));
            }
            self.inner.record_event(event).await
        }

        async fn delete_workspace_events(&self, workspace_id: &str) -> Result<u64, RecorderError> {
            self.inner.delete_workspace_events(workspace_id).await
        }

        fn duckdb_connection(&self) -> Option<Arc<std::sync::Mutex<::duckdb::Connection>>> {
            self.inner.duckdb_connection()
        }

        async fn enforce_retention(&self) -> Result<u64, RecorderError> {
            self.inner.enforce_retention().await
        }

        async fn list_events(
            &self,
            filter: EventFilter,
        ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
            self.inner.list_events(filter).await
        }

        async fn list_session_scoped_events(
            &self,
            session_id: &str,
            from: Option<chrono::DateTime<chrono::Utc>>,
            to: Option<chrono::DateTime<chrono::Utc>>,
        ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
            self.inner
                .list_session_scoped_events(session_id, from, to)
                .await
        }
    }

    #[async_trait::async_trait]
    impl FlightRecorder for FailAllRecordFlightRecorder {
        async fn record_event(&self, _event: FlightRecorderEvent) -> Result<(), RecorderError> {
            Err(RecorderError::SinkError(
                "injected persistent flight-recorder failure".to_owned(),
            ))
        }

        async fn delete_workspace_events(&self, workspace_id: &str) -> Result<u64, RecorderError> {
            self.inner.delete_workspace_events(workspace_id).await
        }

        fn duckdb_connection(&self) -> Option<Arc<std::sync::Mutex<::duckdb::Connection>>> {
            self.inner.duckdb_connection()
        }

        async fn enforce_retention(&self) -> Result<u64, RecorderError> {
            self.inner.enforce_retention().await
        }

        async fn list_events(
            &self,
            filter: EventFilter,
        ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
            self.inner.list_events(filter).await
        }

        async fn list_session_scoped_events(
            &self,
            session_id: &str,
            from: Option<chrono::DateTime<chrono::Utc>>,
            to: Option<chrono::DateTime<chrono::Utc>>,
        ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
            self.inner
                .list_session_scoped_events(session_id, from, to)
                .await
        }
    }

    impl TestLlmClient {
        fn new() -> Self {
            Self {
                profile: ModelProfile::new("fems-memory-api-test".to_string(), 4096),
            }
        }
    }

    #[async_trait::async_trait]
    impl LlmClient for TestLlmClient {
        async fn completion(
            &self,
            _req: CompletionRequest,
        ) -> Result<CompletionResponse, LlmError> {
            Ok(CompletionResponse {
                text: "ok".to_string(),
                usage: TokenUsage {
                    prompt_tokens: 1,
                    completion_tokens: 1,
                    total_tokens: 2,
                },
                latency_ms: 0,
            })
        }

        fn profile(&self) -> &ModelProfile {
            &self.profile
        }
    }

    /// WP-KERNEL-012 MT-144: the `EmbeddedTestBackend` is RETURNED, not dropped here. It owns the
    /// store's cleanup guard, so letting it fall out of scope at the end of this function shut the
    /// store down and every caller then failed with `embedded database is closed`. Callers bind the
    /// store before the state (`let (_store, state) = setup_state().await?;`) so reverse local-drop
    /// order releases every state-owned database handle before the store cleanup guard runs.
    async fn setup_state(
    ) -> Result<(crate::storage::tests::EmbeddedTestBackend, AppState), Box<dyn std::error::Error>>
    {
        let backend = embedded_test_backend().await?;
        let recorder = match DuckDbFlightRecorder::new_in_memory(32) {
            Ok(recorder) => Arc::new(recorder),
            Err(error) => {
                let cleanup = backend.close_and_remove().await;
                return match cleanup {
                    Ok(()) => Err(error.into()),
                    Err(cleanup) => Err(std::io::Error::other(format!(
                        "memory setup failed: {error}; cleanup also failed: {cleanup}"
                    ))
                    .into()),
                };
            }
        };
        let state = AppState {
            storage: backend.database.clone(),
            surreal: backend.storage.clone(),
            flight_recorder: recorder.clone(),
            diagnostics: recorder,
            llm_client: Arc::new(TestLlmClient::new()),
            capability_registry: Arc::new(CapabilityRegistry::new()),
            session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
        };
        Ok((backend, state))
    }

    async fn mt109_memory_residue(
        state: &AppState,
        workspace_id: &str,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        let bindings = MemoryTestBindings {
            workspace: Some(RecordId::new("workspaces", workspace_id)),
            value: Some(workspace_id.to_owned()),
            ..Default::default()
        };
        let mut counts = serde_json::Map::new();
        for (name, query) in [
            ("packs", "SELECT count() AS count FROM fems_memory_packs WHERE workspace_id = $workspace GROUP ALL;"),
            ("proposals", "SELECT count() AS count FROM fems_memory_proposals WHERE workspace_id = $workspace GROUP ALL;"),
            ("items", "SELECT count() AS count FROM fems_memory_items WHERE workspace_id = $workspace GROUP ALL;"),
            ("reports", "SELECT count() AS count FROM fems_memory_commit_reports WHERE workspace_id = $workspace GROUP ALL;"),
            ("commit_outbox", "SELECT count() AS count FROM fems_memory_commit_fr_outbox WHERE workspace_id = $workspace GROUP ALL;"),
            ("lifecycle_outbox", "SELECT count() AS count FROM fems_memory_lifecycle_fr_outbox WHERE workspace_id = $workspace GROUP ALL;"),
            ("anchors", "SELECT count() AS count FROM fems_workspace_write_anchors WHERE workspace_key = $value GROUP ALL;"),
            ("ledger", "SELECT count() AS count FROM kernel_event_ledger WHERE payload.workspace_id = $value AND source_component IN ['fems_memory_proposal_intake', 'fems_memory_proposal_review', 'fems_memory_proposal_commit'] GROUP ALL;"),
        ] {
            counts.insert(name.to_owned(), json!(memory_test_count(state, query, bindings.clone()).await?));
        }
        Ok(Value::Object(counts))
    }

    async fn mt109_memory_ok(
        response: reqwest::Response,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        let path = response.url().path().to_owned();
        let status = response.status();
        let body = response.text().await?;
        assert_eq!(
            status,
            StatusCode::OK,
            "authorized memory route {path}: {body}"
        );
        Ok(serde_json::from_str(&body)?)
    }

    async fn mt109_memory_rows(
        state: &AppState,
    ) -> Result<
        std::collections::BTreeMap<&'static str, Vec<surrealdb::types::Value>>,
        Box<dyn std::error::Error>,
    > {
        let mut rows = std::collections::BTreeMap::new();
        for (name, statement) in [
            ("packs", "SELECT * FROM fems_memory_packs ORDER BY id ASC;"),
            ("proposals", "SELECT * FROM fems_memory_proposals ORDER BY id ASC;"),
            ("items", "SELECT * FROM fems_memory_items ORDER BY id ASC;"),
            ("reports", "SELECT * FROM fems_memory_commit_reports ORDER BY id ASC;"),
            ("commit_outbox", "SELECT * FROM fems_memory_commit_fr_outbox ORDER BY id ASC;"),
            ("lifecycle_outbox", "SELECT * FROM fems_memory_lifecycle_fr_outbox ORDER BY id ASC;"),
            ("anchors", "SELECT * FROM fems_workspace_write_anchors ORDER BY id ASC;"),
            ("ledger", "SELECT * FROM kernel_event_ledger WHERE source_component IN ['fems_memory_proposal_intake', 'fems_memory_proposal_review', 'fems_memory_proposal_commit'] ORDER BY id ASC;"),
        ] {
            let values = state.surreal.with_data_operation(move |database| {
                Box::pin(async move { database.query_values::<surrealdb::types::Value, _>(statement, MemoryTestBindings::default()).await })
            }).await?;
            rows.insert(name, values);
        }
        Ok(rows)
    }

    async fn finish_mt109_memory_test(
        body: Result<Result<(), Box<dyn std::error::Error>>, Box<dyn std::any::Any + Send>>,
        store: crate::storage::tests::EmbeddedTestBackend,
        data_dir: std::path::PathBuf,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let body = match body {
            Ok(result) => result,
            Err(payload) => Err(std::io::Error::other(format!(
                "memory body panicked: {}",
                payload
                    .downcast_ref::<&str>()
                    .copied()
                    .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
                    .unwrap_or("non-string panic")
            ))
            .into()),
        };
        let cleanup = store.close_and_remove().await;
        match (body, cleanup) {
            (Ok(()), Ok(())) => {
                assert!(
                    !data_dir.try_exists()?,
                    "memory store survived cleanup: {}",
                    data_dir.display()
                );
                Ok(())
            }
            (Err(error), Ok(())) => Err(error),
            (Ok(()), Err(error)) => Err(error.into()),
            (Err(body), Err(cleanup)) => Err(std::io::Error::other(format!(
                "memory body failed: {body}; cleanup also failed: {cleanup}"
            ))
            .into()),
        }
    }

    async fn serve_test_router(
        app: axum::Router,
    ) -> (String, reqwest::Client, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind memory test router");
        let address = listener.local_addr().expect("memory test address");
        let server = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("serve memory test router");
        });
        (format!("http://{address}"), reqwest::Client::new(), server)
    }

    #[derive(Clone)]
    struct Mt109MemoryResources {
        workspace: String,
        pack: String,
        proposal: String,
        item: String,
        report: String,
        count: String,
    }

    #[derive(Clone, Copy)]
    enum Mt109GrantMode {
        Full,
        MemoryOnly,
        WorkspaceReadOnly,
        WorkspaceWritesOnly,
        WrongAction,
        WrongCapability,
        MismatchedDelegation,
    }

    fn mt109_memory_capabilities() -> Vec<String> {
        [
            MEMORY_READ_CAPABILITY,
            MEMORY_PROPOSE_CAPABILITY,
            MEMORY_REVIEW_CAPABILITY,
            MEMORY_COMMIT_CAPABILITY,
        ]
        .map(str::to_owned)
        .to_vec()
    }

    async fn mt109_memory_principal(
        state: &AppState,
        account: &str,
        principal: &str,
        space: &str,
        capabilities: &[String],
    ) -> Result<
        crate::storage::surreal::resource_authority::ProvisionedPrincipal,
        Box<dyn std::error::Error>,
    > {
        Ok(state
            .surreal
            .provision_principal(
                account,
                principal,
                "human_account",
                principal,
                "Operator",
                capabilities,
                space,
                None,
                std::time::Duration::from_secs(300),
            )
            .await?)
    }

    async fn mt109_register_memory_resources(
        state: &AppState,
        principal: &crate::storage::surreal::resource_authority::ProvisionedPrincipal,
        workspace_id: &str,
    ) -> Result<Mt109MemoryResources, Box<dyn std::error::Error>> {
        use crate::storage::surreal::resource_authority::ResourceKind;
        let register = |kind| {
            state.surreal.register_protected_resource(
                &principal.identity,
                kind,
                workspace_id,
                None,
                "account_private",
            )
        };
        Ok(Mt109MemoryResources {
            workspace: register(ResourceKind::Workspace).await?.resource_id,
            pack: register(ResourceKind::MemoryPack).await?.resource_id,
            proposal: register(ResourceKind::MemoryProposal).await?.resource_id,
            item: register(ResourceKind::MemoryItem).await?.resource_id,
            report: register(ResourceKind::MemoryCommitReport)
                .await?
                .resource_id,
            count: register(ResourceKind::MemoryItemCount).await?.resource_id,
        })
    }

    async fn mt109_grant_memory_resources(
        state: &AppState,
        principal: &crate::storage::surreal::resource_authority::ProvisionedPrincipal,
        resources: &Mt109MemoryResources,
        mode: Mt109GrantMode,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        use crate::storage::surreal::resource_authority::{ResourceAction, ResourceGrantSpec};
        let cases = [
            (
                resources.workspace.as_str(),
                vec![ResourceAction::Read],
                vec![MEMORY_READ_CAPABILITY],
            ),
            (
                resources.workspace.as_str(),
                vec![ResourceAction::Create],
                vec![MEMORY_PROPOSE_CAPABILITY],
            ),
            (
                resources.workspace.as_str(),
                vec![ResourceAction::Create],
                vec![MEMORY_COMMIT_CAPABILITY],
            ),
            (
                resources.workspace.as_str(),
                vec![ResourceAction::Update],
                vec![MEMORY_REVIEW_CAPABILITY],
            ),
            (
                resources.workspace.as_str(),
                vec![ResourceAction::Update],
                vec![MEMORY_COMMIT_CAPABILITY],
            ),
            (
                resources.pack.as_str(),
                vec![ResourceAction::Read, ResourceAction::Create],
                vec![MEMORY_READ_CAPABILITY, MEMORY_COMMIT_CAPABILITY],
            ),
            (
                resources.proposal.as_str(),
                vec![
                    ResourceAction::Read,
                    ResourceAction::Create,
                    ResourceAction::Update,
                ],
                vec![
                    MEMORY_READ_CAPABILITY,
                    MEMORY_PROPOSE_CAPABILITY,
                    MEMORY_REVIEW_CAPABILITY,
                    MEMORY_COMMIT_CAPABILITY,
                ],
            ),
            (
                resources.item.as_str(),
                vec![ResourceAction::Read, ResourceAction::Create],
                vec![MEMORY_READ_CAPABILITY, MEMORY_COMMIT_CAPABILITY],
            ),
            (
                resources.report.as_str(),
                vec![ResourceAction::Read, ResourceAction::Create],
                vec![MEMORY_READ_CAPABILITY, MEMORY_COMMIT_CAPABILITY],
            ),
            (
                resources.count.as_str(),
                vec![ResourceAction::Read],
                vec![MEMORY_READ_CAPABILITY],
            ),
        ];
        let mut grant_ids = Vec::new();
        for (resource_id, expected_actions, expected_capabilities) in cases {
            let is_workspace = resource_id == resources.workspace;
            let is_read = expected_capabilities == vec![MEMORY_READ_CAPABILITY];
            if match mode {
                Mt109GrantMode::MemoryOnly => is_workspace,
                Mt109GrantMode::WorkspaceReadOnly => !is_workspace || !is_read,
                Mt109GrantMode::WorkspaceWritesOnly => !is_workspace || is_read,
                _ => false,
            } {
                continue;
            }
            let actions = match mode {
                Mt109GrantMode::WrongAction => vec![ResourceAction::Delete],
                _ => expected_actions,
            };
            let capability_ids = match mode {
                Mt109GrantMode::WrongCapability => vec!["fr.read".to_owned()],
                _ => expected_capabilities
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
            };
            let delegation_chain = match mode {
                Mt109GrantMode::MismatchedDelegation => vec!["forged-delegation".to_owned()],
                _ => Vec::new(),
            };
            let grant = state
                .surreal
                .grant_resource(
                    &principal.identity.account_id,
                    &principal.identity.access_space_id,
                    ResourceGrantSpec {
                        principal_id: principal.identity.principal_id.clone(),
                        resource_id: resource_id.to_owned(),
                        actions,
                        capability_ids,
                        expires_at: None,
                        delegation_chain,
                    },
                )
                .await?;
            grant_ids.push(grant.grant_id);
        }
        Ok(grant_ids)
    }

    async fn mt109_exchange_memory_session(
        state: &AppState,
        base: &str,
        client: &reqwest::Client,
        principal: &crate::storage::surreal::resource_authority::ProvisionedPrincipal,
        binding_token: &str,
    ) -> Result<(String, String), Box<dyn std::error::Error>> {
        let credential = state
            .surreal
            .provision_session_credential(&principal.identity, std::time::Duration::from_secs(300))
            .await?;
        let response = client
            .post(format!("{base}/authority/session"))
            .header("x-hsk-channel-binding-token", binding_token)
            .json(&json!({
                "account_id": principal.identity.account_id.clone(),
                "principal_id": principal.identity.principal_id.clone(),
                "access_space_id": principal.identity.access_space_id.clone(),
                "authentication_token": credential.token,
            }))
            .send()
            .await?;
        assert_eq!(response.status(), StatusCode::OK, "credential exchange");
        let body: Value = response.json().await?;
        Ok((
            body["session_token"]
                .as_str()
                .expect("session token")
                .to_owned(),
            body["session_id"].as_str().expect("session id").to_owned(),
        ))
    }

    #[tokio::test]
    async fn memory_routes_require_live_binding_ignore_spoofed_actor_and_audit_decisions(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let _env_lock = MEMORY_AUTH_ENV_LOCK.lock().expect("memory auth env lock");
        let token = "a".repeat(64);
        let binding_path =
            std::env::temp_dir().join(format!("hsk-stage-binding-{}.json", Uuid::now_v7()));
        std::fs::write(
            &binding_path,
            serde_json::to_vec(&crate::api::stage::current_process_native_binding(&token))?,
        )?;
        let _binding_guard = BindingEnvGuard {
            previous: std::env::var_os("HANDSHAKE_STAGE_BINDING_FILE"),
            path: binding_path.clone(),
        };
        std::env::set_var("HANDSHAKE_STAGE_BINDING_FILE", &binding_path);

        let (store, state) = setup_state().await?;
        let data_dir = store.data_dir.clone();
        let mut server_task = None;
        let mut reconciler_task = None;
        let body = futures::FutureExt::catch_unwind(std::panic::AssertUnwindSafe(async {
        let workspace_id = create_test_workspace(&state, "memory-route-auth").await?;
        let source = create_test_rich_source(
            &state,
            &workspace_id,
            "memory-route-auth-source",
            "authenticated proposal",
        )
        .await?;
        let principal = crate::api::authority::test_principal_for_binding(&state, &token).await?;
        let workspace_resource = state
            .surreal
            .register_workspace_resource(&principal.identity, &workspace_id)
            .await?;
        let source_resource = state
            .surreal
            .register_protected_resource(
                &principal.identity,
                crate::storage::surreal::resource_authority::ResourceKind::RichDocument,
                &source.document_id,
                Some(&workspace_resource.resource_id),
                "account_private",
            )
            .await?;
        state
            .surreal
            .grant_resource(
                &principal.identity.account_id,
                &principal.identity.access_space_id,
                crate::storage::surreal::resource_authority::ResourceGrantSpec {
                    principal_id: principal.identity.principal_id.clone(),
                    resource_id: source_resource.resource_id,
                    actions: vec![
                        crate::storage::surreal::resource_authority::ResourceAction::Read,
                    ],
                    capability_ids: vec![MEMORY_PROPOSE_CAPABILITY.to_owned()],
                    expires_at: None,
                    delegation_chain: Vec::new(),
                },
            )
            .await?;
        let session_token = principal.session.token;
        let (memory_router, reconciler) = routes_with_reconciler(state.clone());
        reconciler_task = reconciler;
        let (base, client, server) = serve_test_router(memory_router).await;
        server_task = Some(server);
        let list_url = format!("{base}/workspaces/{workspace_id}/memory/proposals");

        let missing = client.get(&list_url).send().await?;
        assert_eq!(missing.status(), StatusCode::FORBIDDEN);
        let forged = client
            .get(&list_url)
            .header("x-hsk-session-token", "b".repeat(64))
            .header("x-hsk-channel-binding-token", &token)
            .send()
            .await?;
        assert_eq!(forged.status(), StatusCode::FORBIDDEN);
        let allowed = client
            .get(&list_url)
            .header("x-hsk-session-token", &session_token)
            .header("x-hsk-channel-binding-token", &token)
            .send()
            .await?;
        assert_eq!(allowed.status(), StatusCode::OK);

        let proposal = ProposalRequest {
            request_id: Some(format!("route-auth-{}", Uuid::now_v7())),
            class: ProposalClass::Semantic,
            content: "authenticated proposal".to_owned(),
            source,
            source_document_content: None,
            review_gated: Some(true),
            actor_id: Some("spoofed-body-actor".to_owned()),
        };
        let response = client
            .post(format!("{base}/workspaces/{workspace_id}/memory/proposals"))
            .header("x-hsk-session-token", &session_token)
            .header("x-hsk-channel-binding-token", &token)
            .header(HSK_HEADER_ACTOR_ID, "spoofed-header-actor")
            .header(HSK_HEADER_ACTOR_KIND, "system")
            .json(&proposal)
            .send()
            .await?;
        let response_status = response.status();
        let response_body = response.text().await?;
        assert_eq!(
            response_status,
            StatusCode::OK,
            "authorized proposal response: {response_body}"
        );
        let ack: ProposalAck = serde_json::from_str(&response_body)?;
        let stored = fems_memory::get_memory_proposal(&state.surreal, &ack.proposal_id)
            .await?
            .expect("authenticated proposal stored");
        let actor_id = stored.proposal["actor_id"]
            .as_str()
            .expect("stored proposal actor id");
        assert_eq!(actor_id, "local_operator");
        assert_ne!(actor_id, "spoofed-body-actor");
        assert_ne!(actor_id, "spoofed-header-actor");

        let decisions = state
            .flight_recorder
            .list_events(EventFilter {
                event_type: Some("capability_action".to_owned()),
                wsid: Some(workspace_id.clone()),
                ..EventFilter::default()
            })
            .await?;
        let read_denies = decisions
            .iter()
            .filter(|event| {
                event.payload["capability_id"] == MEMORY_READ_CAPABILITY
                    && event.payload["decision_outcome"] == "deny"
            })
            .count();
        assert_eq!(read_denies, 2, "missing and forged tokens are audited");
        assert!(decisions.iter().any(|event| {
            event.payload["capability_id"] == MEMORY_READ_CAPABILITY
                && event.payload["decision_outcome"] == "allow"
        }));
        assert!(decisions.iter().any(|event| {
            event.payload["capability_id"] == MEMORY_PROPOSE_CAPABILITY
                && event.payload["decision_outcome"] == "allow"
                && event.actor_id == actor_id
        }));

        Ok::<(), Box<dyn std::error::Error>>(())
        })).await;
        if let Some(server) = server_task {
            server.abort();
            let _ = server.await;
        }
        if let Some(reconciler) = reconciler_task {
            reconciler.abort();
            let _ = reconciler.await;
        }
        drop(state);
        finish_mt109_memory_test(body, store, data_dir).await
    }

    #[tokio::test]
    async fn mt109_every_memory_route_uses_persisted_credentials_and_full_denial_matrix(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (store, state) = setup_state().await?;
        let data_dir = store.data_dir.clone();
        let mut server_task = None;
        let mut reconciler_task = None;
        let mut body = futures::FutureExt::catch_unwind(std::panic::AssertUnwindSafe(async {
            let _env_lock = MEMORY_AUTH_ENV_LOCK.lock().expect("memory auth env lock");
            let binding_token = "d".repeat(64);
            let binding_path =
                std::env::temp_dir().join(format!("hsk-stage-binding-{}.json", Uuid::now_v7()));
            std::fs::write(
                &binding_path,
                serde_json::to_vec(&crate::api::stage::current_process_native_binding(
                    &binding_token,
                ))?,
            )?;
            let _binding_guard = BindingEnvGuard {
                previous: std::env::var_os("HANDSHAKE_STAGE_BINDING_FILE"),
                path: binding_path.clone(),
            };
            std::env::set_var("HANDSHAKE_STAGE_BINDING_FILE", &binding_path);

            let workspace_id = create_test_workspace(&state, "memory-mounted-matrix").await?;
            let content = "mounted route matrix";
            let source = create_test_rich_source(
                &state,
                &workspace_id,
                "memory-mounted-matrix-source",
                content,
            )
            .await?;
            let capabilities = mt109_memory_capabilities();
            let owner = mt109_memory_principal(
                &state,
                "memory-matrix-account",
                "memory-matrix-owner",
                "memory-matrix-space-a",
                &capabilities,
            )
            .await?;
            let resources = mt109_register_memory_resources(&state, &owner, &workspace_id).await?;
            mt109_grant_memory_resources(&state, &owner, &resources, Mt109GrantMode::MemoryOnly)
                .await?;
            let foreign_workspace = create_test_workspace(&state, "memory-mounted-foreign").await?;
            let (memory_router, reconciler) = routes_with_reconciler(state.clone());
            reconciler_task = reconciler;
            let app = crate::api::authority::routes(state.clone()).merge(memory_router);
            let (base, client, server) = serve_test_router(app).await;
            server_task = Some(server);
            let (session_token, _session_id) =
                mt109_exchange_memory_session(&state, &base, &client, &owner, &binding_token)
                    .await?;
            let authenticated = |builder: reqwest::RequestBuilder| {
                builder
                    .header("x-hsk-session-token", &session_token)
                    .header("x-hsk-channel-binding-token", &binding_token)
            };

            let before_workspace_grant = mt109_memory_residue(&state, &workspace_id).await?;
            let initial_rows = mt109_memory_rows(&state).await?;
            assert!(
                initial_rows.values().all(Vec::is_empty),
                "initial global FEMS rows: {initial_rows:?}"
            );
            assert!(
                before_workspace_grant
                    .as_object()
                    .expect("residue counts object")
                    .values()
                    .all(|count| count == &json!(0)),
                "initial FEMS residue: {before_workspace_grant}"
            );
            let missing_workspace = authenticated(
                client.get(format!("{base}/workspaces/{workspace_id}/memory/proposals")),
            )
            .send()
            .await?;
            let denied_status = missing_workspace.status();
            let denied_body: Value = missing_workspace.json().await?;
            assert_eq!(
                denied_status,
                StatusCode::NOT_FOUND,
                "workspace prerequisite denial: {denied_body}"
            );
            assert_eq!(
                denied_body,
                json!({"error": "not_found", "detail": "workspace"})
            );
            assert_eq!(
                mt109_memory_residue(&state, &workspace_id).await?,
                before_workspace_grant,
                "workspace-denied read changed FEMS state"
            );
            mt109_grant_memory_resources(
                &state,
                &owner,
                &resources,
                Mt109GrantMode::WorkspaceReadOnly,
            )
            .await?;
            let workspace_retry = authenticated(
                client.get(format!("{base}/workspaces/{workspace_id}/memory/proposals")),
            )
            .send()
            .await?;
            let retry_status = workspace_retry.status();
            let retry_body: Value = workspace_retry.json().await?;
            assert_eq!(
                retry_status,
                StatusCode::OK,
                "same read after exact Workspace/read/memory.read grant: {retry_body}"
            );
            assert_eq!(
                mt109_memory_residue(&state, &workspace_id).await?,
                before_workspace_grant
            );

            let denied_paths =
                |workspace: &str| {
                    vec![
                (
                    reqwest::Method::GET,
                    format!("/workspaces/{workspace}/memory/pack"),
                ),
                (
                    reqwest::Method::GET,
                    format!("/workspaces/{workspace}/memory/proposals"),
                ),
                (
                    reqwest::Method::POST,
                    format!("/workspaces/{workspace}/memory/proposals"),
                ),
                (
                    reqwest::Method::GET,
                    format!("/workspaces/{workspace}/memory/proposals/foreign-proposal"),
                ),
                (
                    reqwest::Method::GET,
                    format!("/workspaces/{workspace}/memory/proposals/foreign-proposal/artifact"),
                ),
                (
                    reqwest::Method::POST,
                    format!("/workspaces/{workspace}/memory/proposals/foreign-proposal/review"),
                ),
                (
                    reqwest::Method::POST,
                    format!("/workspaces/{workspace}/memory/proposals/foreign-proposal/commit"),
                ),
                (
                    reqwest::Method::GET,
                    format!("/workspaces/{workspace}/memory/commits/foreign-commit/report"),
                ),
                (
                    reqwest::Method::GET,
                    format!("/workspaces/{workspace}/memory/items/count"),
                ),
            ]
                };
            for (method, path) in denied_paths(&foreign_workspace) {
                let response = authenticated(client.request(method, format!("{base}{path}")))
                    .json(&json!({}))
                    .send()
                    .await?;
                assert_eq!(response.status(), StatusCode::FORBIDDEN, "{path}");
                assert_eq!(
                    response.json::<Value>().await?,
                    json!({"error": "HSK-403-PROTECTED-RESOURCE"}),
                    "{path} must not disclose whether the selector exists"
                );
            }

            let member = mt109_memory_principal(
                &state,
                "memory-matrix-account",
                "memory-matrix-member",
                "memory-matrix-space-a",
                &capabilities,
            )
            .await?;
            assert_eq!(member.identity.account_id, owner.identity.account_id);
            assert_eq!(
                member.identity.access_space_id,
                owner.identity.access_space_id
            );
            let wrong_space = mt109_memory_principal(
                &state,
                "memory-matrix-account",
                "memory-matrix-space-member",
                "memory-matrix-space-b",
                &capabilities,
            )
            .await?;
            assert_eq!(wrong_space.identity.account_id, owner.identity.account_id);
            assert_ne!(
                wrong_space.identity.access_space_id,
                owner.identity.access_space_id
            );
            let foreign_account = mt109_memory_principal(
                &state,
                "memory-matrix-foreign-account",
                "memory-matrix-foreign-principal",
                "memory-matrix-foreign-space",
                &capabilities,
            )
            .await?;
            let without_capability = mt109_memory_principal(
                &state,
                "memory-matrix-account",
                "memory-matrix-no-capability",
                "memory-matrix-space-a",
                &[],
            )
            .await?;
            mt109_grant_memory_resources(
                &state,
                &without_capability,
                &resources,
                Mt109GrantMode::Full,
            )
            .await?;
            let wrong_action = mt109_memory_principal(
                &state,
                "memory-matrix-account",
                "memory-matrix-wrong-action",
                "memory-matrix-space-a",
                &capabilities,
            )
            .await?;
            mt109_grant_memory_resources(
                &state,
                &wrong_action,
                &resources,
                Mt109GrantMode::WrongAction,
            )
            .await?;
            let wrong_capability = mt109_memory_principal(
                &state,
                "memory-matrix-account",
                "memory-matrix-wrong-capability",
                "memory-matrix-space-a",
                &capabilities,
            )
            .await?;
            mt109_grant_memory_resources(
                &state,
                &wrong_capability,
                &resources,
                Mt109GrantMode::WrongCapability,
            )
            .await?;
            let mismatched_delegation = mt109_memory_principal(
                &state,
                "memory-matrix-account",
                "memory-matrix-bad-delegation",
                "memory-matrix-space-a",
                &capabilities,
            )
            .await?;
            mt109_grant_memory_resources(
                &state,
                &mismatched_delegation,
                &resources,
                Mt109GrantMode::MismatchedDelegation,
            )
            .await?;
            let revoked_grant = mt109_memory_principal(
                &state,
                "memory-matrix-account",
                "memory-matrix-revoked-grant",
                "memory-matrix-space-a",
                &capabilities,
            )
            .await?;
            let revoked_grant_ids = mt109_grant_memory_resources(
                &state,
                &revoked_grant,
                &resources,
                Mt109GrantMode::Full,
            )
            .await?;
            let stale_space = mt109_memory_principal(
                &state,
                "memory-matrix-account",
                "memory-matrix-stale-space",
                "memory-matrix-space-a",
                &capabilities,
            )
            .await?;
            mt109_grant_memory_resources(&state, &stale_space, &resources, Mt109GrantMode::Full)
                .await?;

            let mut denied_sessions = Vec::new();
            for (label, principal) in [
                ("same-account-member-without-grant", &member),
                ("wrong-access-space", &wrong_space),
                ("cross-account", &foreign_account),
                ("grant-without-capability", &without_capability),
                ("wrong-action", &wrong_action),
                ("wrong-capability", &wrong_capability),
                ("mismatched-delegation", &mismatched_delegation),
            ] {
                let (token, _) = mt109_exchange_memory_session(
                    &state,
                    &base,
                    &client,
                    principal,
                    &binding_token,
                )
                .await?;
                denied_sessions.push((label, token));
            }
            let (revoked_grant_token, _) = mt109_exchange_memory_session(
                &state,
                &base,
                &client,
                &revoked_grant,
                &binding_token,
            )
            .await?;
            for grant_id in revoked_grant_ids {
                state.surreal.revoke_grant(&grant_id).await?;
            }
            denied_sessions.push(("revoked-grants", revoked_grant_token));
            let (stale_token, stale_session_id) =
                mt109_exchange_memory_session(&state, &base, &client, &stale_space, &binding_token)
                    .await?;
            let switched = mt109_memory_principal(
                &state,
                "memory-matrix-account",
                "memory-matrix-stale-space",
                "memory-matrix-space-b",
                &capabilities,
            )
            .await?;
            state
                .surreal
                .switch_session_access_space(&stale_session_id, &switched.identity.access_space_id)
                .await?;
            denied_sessions.push(("stale-post-space-switch", stale_token));

            assert!(owner.session.expires_at > chrono::Utc::now(), "earliest ordinary session expired before negative phase");
            eprintln!("MT109_MEMORY_ORDINARY_REMAINING_MS={}", (owner.session.expires_at - chrono::Utc::now()).num_milliseconds());
            let state_before_denials = mt109_memory_residue(&state, &workspace_id).await?;
            for (label, token) in denied_sessions {
                for (method, path) in denied_paths(&workspace_id) {
                    let response = client
                        .request(method, format!("{base}{path}"))
                        .header("x-hsk-session-token", &token)
                        .header("x-hsk-channel-binding-token", &binding_token)
                        .json(&json!({"workspace_id": foreign_workspace, "proposal_id": "forged"}))
                        .send()
                        .await?;
                    assert_eq!(response.status(), StatusCode::FORBIDDEN, "{label}: {path}");
                    assert_eq!(
                        response.json::<Value>().await?,
                        json!({"error": "HSK-403-PROTECTED-RESOURCE"}),
                        "{label}: {path} constant denial"
                    );
                }
            }
            assert_eq!(
                mt109_memory_residue(&state, &workspace_id).await?,
                state_before_denials,
                "denied mounted routes changed FEMS state"
            );
            assert_eq!(
                mt109_memory_residue(&state, &foreign_workspace).await?,
                state_before_denials,
                "foreign workspace gained FEMS residue"
            );

            let disabled_workspace =
                create_test_workspace(&state, "memory-mounted-disabled").await?;
            let disabled = mt109_memory_principal(
                &state,
                "memory-matrix-disabled-account",
                "memory-matrix-disabled-principal",
                "memory-matrix-disabled-space",
                &capabilities,
            )
            .await?;
            let disabled_resources =
                mt109_register_memory_resources(&state, &disabled, &disabled_workspace).await?;
            mt109_grant_memory_resources(
                &state,
                &disabled,
                &disabled_resources,
                Mt109GrantMode::Full,
            )
            .await?;
            let (disabled_token, _) =
                mt109_exchange_memory_session(&state, &base, &client, &disabled, &binding_token)
                    .await?;
            state
                .surreal
                .disable_account(&disabled.identity.account_id)
                .await?;
            for (method, path) in denied_paths(&disabled_workspace) {
                let response = client
                    .request(method, format!("{base}{path}"))
                    .header("x-hsk-session-token", &disabled_token)
                    .header("x-hsk-channel-binding-token", &binding_token)
                    .json(&json!({}))
                    .send()
                    .await?;
                assert_eq!(response.status(), StatusCode::FORBIDDEN, "disabled: {path}");
                assert_eq!(
                    response.json::<Value>().await?,
                    json!({"error": "HSK-403-PROTECTED-RESOURCE"})
                );
            }

            assert_eq!(
                mt109_memory_residue(&state, &disabled_workspace).await?,
                state_before_denials,
                "disabled workspace gained FEMS residue"
            );
            assert_eq!(
                mt109_memory_rows(&state).await?,
                initial_rows,
                "denied routes changed global protected FEMS rows"
            );
            mt109_grant_memory_resources(
                &state,
                &owner,
                &resources,
                Mt109GrantMode::WorkspaceWritesOnly,
            )
            .await?;
            mt109_memory_ok(
                authenticated(client.get(format!("{base}/workspaces/{workspace_id}/memory/pack")))
                    .send()
                    .await?,
            )
            .await?;
            mt109_memory_ok(
                authenticated(
                    client.get(format!("{base}/workspaces/{workspace_id}/memory/proposals")),
                )
                .send()
                .await?,
            )
            .await?;
            assert!(owner.session.expires_at > chrono::Utc::now(), "ordinary positive control session expired");
            let source_id = source.document_id.clone();
            let proposal_request = ProposalRequest {
                request_id: Some(format!("mounted-matrix-{}", Uuid::now_v7())),
                class: ProposalClass::Semantic,
                content: content.to_owned(),
                source,
                source_document_content: None,
                review_gated: Some(true),
                actor_id: Some("forged-client-actor".to_owned()),
            };
            let denied_source = authenticated(client.post(format!("{base}/workspaces/{workspace_id}/memory/proposals")))
                .json(&proposal_request).send().await?;
            let denied_source_status = denied_source.status();
            let denied_source_body: Value = denied_source.json().await?;
            assert_eq!(denied_source_status, StatusCode::BAD_REQUEST, "source grant missing: {denied_source_body}");
            assert_eq!(denied_source_body, json!({"error": "bad_request", "detail": "proposal provenance document_id does not resolve to a rich document, a canonical code source, or a Loom block in this workspace"}));
            assert_eq!(mt109_memory_rows(&state).await?, initial_rows, "source-denied proposal changed FEMS rows");
            let source_resource = state.surreal.register_protected_resource(
                &owner.identity,
                crate::storage::surreal::resource_authority::ResourceKind::RichDocument,
                &source_id,
                Some(&resources.workspace),
                "account_private",
            ).await?;
            state.surreal.grant_resource(&owner.identity.account_id, &owner.identity.access_space_id,
                crate::storage::surreal::resource_authority::ResourceGrantSpec {
                    principal_id: owner.identity.principal_id.clone(),
                    resource_id: source_resource.resource_id,
                    actions: vec![crate::storage::surreal::resource_authority::ResourceAction::Read],
                    capability_ids: vec![MEMORY_PROPOSE_CAPABILITY.to_owned()],
                    expires_at: None,
                    delegation_chain: Vec::new(),
                }).await?;
            let proposal_response = authenticated(client.post(format!("{base}/workspaces/{workspace_id}/memory/proposals")))
                .json(&proposal_request).send().await?;
            let proposal_status = proposal_response.status();
            let proposal_body = proposal_response.text().await?;
            assert_eq!(
                proposal_status,
                StatusCode::OK,
                "authorized proposal POST: {proposal_body}"
            );
            let proposal: ProposalAck = serde_json::from_str(&proposal_body)?;
            let proposal_url = format!(
                "{base}/workspaces/{workspace_id}/memory/proposals/{}",
                proposal.proposal_id
            );
            mt109_memory_ok(authenticated(client.get(&proposal_url)).send().await?).await?;
            mt109_memory_ok(
                authenticated(client.get(format!("{proposal_url}/artifact")))
                    .send()
                    .await?,
            )
            .await?;
            let review = authenticated(client.post(format!("{proposal_url}/review")))
                .json(&json!({
                    "decision": "approved",
                    "reviewer_kind": "user",
                    "reason": "mounted route proof"
                }))
                .send()
                .await?;
            let review_status = review.status();
            let review_body = review.text().await?;
            assert_eq!(
                review_status,
                StatusCode::OK,
                "authorized review POST: {review_body}"
            );
            let commit = authenticated(client.post(format!("{proposal_url}/commit")))
                .json(&json!({}))
                .send()
                .await?;
            let commit_status = commit.status();
            let commit_body = commit.text().await?;
            assert_eq!(
                commit_status,
                StatusCode::OK,
                "authorized commit POST: {commit_body}"
            );
            let commit: Value = serde_json::from_str(&commit_body)?;
            let commit_id = commit["commit_id"]
                .as_str()
                .expect("commit route returns commit_id");
            mt109_memory_ok(
                authenticated(client.get(format!(
                    "{base}/workspaces/{workspace_id}/memory/commits/{commit_id}/report"
                )))
                .send()
                .await?,
            )
            .await?;
            mt109_memory_ok(
                authenticated(client.get(format!(
                    "{base}/workspaces/{workspace_id}/memory/items/count"
                )))
                .send()
                .await?,
            )
            .await?;

            let state_after_success = mt109_memory_residue(&state, &workspace_id).await?;
            assert_eq!(
                state_after_success["ledger"],
                json!(3),
                "proposal/review/commit must each produce a scoped receipt"
            );
            let rows_after_success = mt109_memory_rows(&state).await?;
            assert_eq!(
                rows_after_success["ledger"].len(),
                3,
                "all FEMS receipts must match the scoped positive control"
            );
            let binding_hash = hex::encode(Sha256::digest(binding_token.as_bytes()));
            let expired = state
                .surreal
                .provision_principal(
                    "memory-matrix-account",
                    "memory-matrix-expired",
                    "human_account",
                    "memory-matrix-expired",
                    "Operator",
                    &capabilities,
                    "memory-matrix-space-a",
                    Some(&binding_hash),
                    std::time::Duration::from_millis(1),
                )
                .await?;
            mt109_grant_memory_resources(&state, &expired, &resources, Mt109GrantMode::Full)
                .await?;
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            for (method, path) in denied_paths(&workspace_id) {
                let response = client
                    .request(method, format!("{base}{path}"))
                    .header("x-hsk-session-token", &expired.session.token)
                    .header("x-hsk-channel-binding-token", &binding_token)
                    .json(&json!({}))
                    .send()
                    .await?;
                assert_eq!(response.status(), StatusCode::FORBIDDEN, "expired: {path}");
            }

            let revocation_decision = state
                .surreal
                .authorize_protected_resource(
                    crate::storage::surreal::resource_authority::AuthorizationRequest {
                        session_token: session_token.clone(),
                        channel_binding_hash: Some(hex::encode(Sha256::digest(
                            binding_token.as_bytes(),
                        ))),
                        capability_id: MEMORY_READ_CAPABILITY.to_owned(),
                        resource_kind:
                            crate::storage::surreal::resource_authority::ResourceKind::MemoryPack,
                        external_resource_id: workspace_id.clone(),
                        action: crate::storage::surreal::resource_authority::ResourceAction::Read,
                    },
                )
                .await?;
            state
                .surreal
                .revoke_session(&revocation_decision.session_id)
                .await?;
            for (method, path) in denied_paths(&workspace_id) {
                let response = authenticated(client.request(method, format!("{base}{path}")))
                    .json(&json!({}))
                    .send()
                    .await?;
                assert_eq!(response.status(), StatusCode::FORBIDDEN, "{path}");
                assert_eq!(
                    response.json::<Value>().await?,
                    json!({"error": "HSK-403-PROTECTED-RESOURCE"}),
                    "{path} revoked-session denial"
                );
            }
            assert_eq!(
                mt109_memory_residue(&state, &workspace_id).await?,
                state_after_success,
                "expired/revoked routes changed successful FEMS state"
            );
            assert_eq!(
                mt109_memory_rows(&state).await?,
                rows_after_success,
                "expired/revoked routes mutated protected rows"
            );
            Ok::<(), Box<dyn std::error::Error>>(())
        }))
        .await;
        if let Some(server) = server_task {
            server.abort();
            let _ = server.await;
        }
        if let Some(reconciler) = reconciler_task {
            if let Err(join_error) = reconciler.await {
                body = match body {
                    Ok(Ok(())) => Ok(Err(join_error.into())),
                    Ok(Err(error)) => Ok(Err(std::io::Error::other(format!(
                        "memory body failed: {error}; reconciler join also failed: {join_error}"
                    ))
                    .into())),
                    Err(panic) => {
                        eprintln!("memory reconciler join also failed: {join_error}");
                        Err(panic)
                    }
                };
            }
        }
        drop(state);
        finish_mt109_memory_test(body, store, data_dir).await
    }

    async fn mt109_source_rows_for_reader(
        state: &AppState,
        token: &str,
        binding: &str,
        workspace: &str,
        document: &str,
    ) -> Result<Vec<surrealdb::types::Value>, Box<dyn std::error::Error>> {
        use crate::storage::surreal::resource_authority::{ResourceAction, ResourceKind};
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("x-hsk-session-token", token.parse()?);
        headers.insert("x-hsk-channel-binding-token", binding.parse()?);
        let authority = crate::api::authority::authorize_request(
            state,
            &headers,
            MEMORY_READ_CAPABILITY,
            ResourceKind::MemoryProposal,
            workspace,
            ResourceAction::Read,
        )
        .await
        .map_err(|(status, body)| {
            std::io::Error::other(format!("source read outer authority: {status}; {}", body.0))
        })?;
        let bindings = MemoryTestBindings {
            value: Some(document.to_owned()),
            ..Default::default()
        };
        Ok(state.surreal.with_record_user_scope(authority.record_user_scope,
            state.surreal.with_data_operation(move |database| Box::pin(async move {
                database.query_values::<surrealdb::types::Value, _>(
                    "SELECT * FROM knowledge_rich_documents WHERE rich_document_id = $value;", bindings).await
            }))).await?)
    }

    async fn mt109_proposal_rows_for_producer(
        state: &AppState,
        principal: &crate::storage::surreal::resource_authority::ProvisionedPrincipal,
        workspace: &str,
        proposal: &str,
    ) -> Result<Vec<surrealdb::types::Value>, Box<dyn std::error::Error>> {
        use crate::storage::surreal::resource_authority::{
            AuthorizationRequest, RecordUserScope, ResourceAction, ResourceKind,
        };
        let decision = state
            .surreal
            .authorize_protected_resource(AuthorizationRequest {
                session_token: principal.session.token.clone(),
                channel_binding_hash: None,
                capability_id: MEMORY_PROPOSE_CAPABILITY.to_owned(),
                resource_kind: ResourceKind::MemoryProposal,
                external_resource_id: workspace.to_owned(),
                action: ResourceAction::Create,
            })
            .await?;
        let scope = RecordUserScope {
            workspace_id: Some(workspace.to_owned()),
            session_token: principal.session.token.clone(),
            channel_binding_hash: None,
            resource_id: decision.resource_id,
            session_id: decision.session_id,
            capability_id: MEMORY_PROPOSE_CAPABILITY.to_owned(),
            action: ResourceAction::Create,
        };
        let bindings = MemoryTestBindings {
            value: Some(proposal.to_owned()),
            ..Default::default()
        };
        Ok(state
            .surreal
            .with_record_user_scope(
                scope,
                state.surreal.with_data_operation(move |database| {
                    Box::pin(async move {
                        database
                            .query_values::<surrealdb::types::Value, _>(
                                "SELECT * FROM fems_memory_proposals WHERE proposal_id = $value;",
                                bindings,
                            )
                            .await
                    })
                }),
            )
            .await?)
    }

    async fn mt109_source_refs_visible_for_reader(
        state: &AppState,
        token: &str,
        binding: &str,
        workspace: &str,
        source_refs: &[FemsSourceRef],
    ) -> Result<bool, Box<dyn std::error::Error>> {
        use crate::storage::surreal::resource_authority::{ResourceAction, ResourceKind};
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("x-hsk-session-token", token.parse()?);
        headers.insert("x-hsk-channel-binding-token", binding.parse()?);
        let authority = crate::api::authority::authorize_request(
            state,
            &headers,
            MEMORY_READ_CAPABILITY,
            ResourceKind::MemoryProposal,
            workspace,
            ResourceAction::Read,
        )
        .await
        .map_err(|(status, body)| {
            std::io::Error::other(format!("mixed-source outer authority: {status}; {}", body.0))
        })?;
        state
            .surreal
            .with_record_user_scope(
                authority.record_user_scope,
                memory_source_refs_visible(state, workspace, source_refs),
            )
            .await
            .map_err(|(status, body)| {
                std::io::Error::other(format!("mixed-source visibility: {status}; {}", body.0))
                    .into()
            })
    }

    #[tokio::test]
    async fn mt109_memory_derivatives_do_not_widen_source_read_authority(
    ) -> Result<(), Box<dyn std::error::Error>> {
        use crate::storage::surreal::resource_authority::{
            ResourceAction, ResourceGrantSpec, ResourceKind,
        };
        let (store, state) = setup_state().await?;
        let data_dir = store.data_dir.clone();
        let mut shutdown_tx = None;
        let mut server_task = None;
        let mut reconciler_task = None;
        let body = futures::FutureExt::catch_unwind(std::panic::AssertUnwindSafe(async {
            let _env_lock = MEMORY_AUTH_ENV_LOCK.lock().expect("memory auth env lock");
            let binding_token = "e".repeat(64);
            let binding_path = std::env::temp_dir().join(format!("hsk-stage-binding-{}.json", Uuid::now_v7()));
            std::fs::write(&binding_path, serde_json::to_vec(&crate::api::stage::current_process_native_binding(&binding_token))?)?;
            let _binding_guard = BindingEnvGuard { previous: std::env::var_os("HANDSHAKE_STAGE_BINDING_FILE"), path: binding_path.clone() };
            std::env::set_var("HANDSHAKE_STAGE_BINDING_FILE", &binding_path);
            let workspace_id = create_test_workspace(&state, "memory-source-derivative").await?;
            let content = format!("private-source-{}", Uuid::now_v7());
            let source = create_test_rich_source(&state, &workspace_id, "derivative", &content).await?;
            let second_content = format!("private-second-source-{}", Uuid::now_v7());
            let second_source = create_test_rich_source(
                &state,
                &workspace_id,
                "derivative-second",
                &second_content,
            )
            .await?;
            let owner = mt109_memory_principal(&state, "derivative-account", "derivative-owner", "derivative-space", &mt109_memory_capabilities()).await?;
            let reader = mt109_memory_principal(&state, "derivative-account", "derivative-reader", "derivative-space", &[MEMORY_READ_CAPABILITY.to_owned()]).await?;
            assert_ne!(owner.identity.principal_id, reader.identity.principal_id);
            assert_eq!(owner.identity.account_id, reader.identity.account_id);
            assert_eq!(owner.identity.access_space_id, reader.identity.access_space_id);
            let resources = mt109_register_memory_resources(&state, &owner, &workspace_id).await?;
            mt109_grant_memory_resources(&state, &owner, &resources, Mt109GrantMode::Full).await?;
            for resource_id in [&resources.workspace, &resources.pack, &resources.proposal, &resources.item, &resources.report, &resources.count] {
                state.surreal.grant_resource(&reader.identity.account_id, &reader.identity.access_space_id, ResourceGrantSpec {
                    principal_id: reader.identity.principal_id.clone(), resource_id: resource_id.clone(), actions: vec![ResourceAction::Read],
                    capability_ids: vec![MEMORY_READ_CAPABILITY.to_owned()], expires_at: None, delegation_chain: Vec::new(),
                }).await?;
            }
            let source_resource = state.surreal.register_protected_resource(&owner.identity, ResourceKind::RichDocument, &source.document_id, Some(&resources.workspace), "account_private").await?;
            let _second_source_resource = state.surreal.register_protected_resource(&owner.identity, ResourceKind::RichDocument, &second_source.document_id, Some(&resources.workspace), "account_private").await?;
            let source_grant = state.surreal.grant_resource(&owner.identity.account_id, &owner.identity.access_space_id, ResourceGrantSpec {
                principal_id: owner.identity.principal_id.clone(), resource_id: source_resource.resource_id.clone(), actions: vec![ResourceAction::Read],
                capability_ids: vec![MEMORY_PROPOSE_CAPABILITY.to_owned()], expires_at: None, delegation_chain: Vec::new(),
            }).await?;
            let (memory_router, reconciler) = routes_with_reconciler(state.clone());
            reconciler_task = reconciler;
            let app = crate::api::authority::routes(state.clone()).merge(memory_router);
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
            let base = format!("http://{}", listener.local_addr()?);
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(300))
                .build()?;
            let (tx, rx) = tokio::sync::oneshot::channel::<()>();
            shutdown_tx = Some(tx);
            server_task = Some(tokio::spawn(async move {
                axum::serve(listener, app).with_graceful_shutdown(async move { let _ = rx.await; }).await
            }));
            let (owner_token, _) = mt109_exchange_memory_session(&state, &base, &client, &owner, &binding_token).await?;
            let (reader_token, _) = mt109_exchange_memory_session(&state, &base, &client, &reader, &binding_token).await?;
            assert_ne!(owner_token, reader_token);
            let auth = |request: reqwest::RequestBuilder, token: &str| request.header("x-hsk-session-token", token).header("x-hsk-channel-binding-token", &binding_token);
            assert_eq!(mt109_source_rows_for_reader(&state, &owner_token, &binding_token, &workspace_id, &source.document_id).await?.len(), 1, "owner source control");
            assert!(mt109_source_rows_for_reader(&state, &reader_token, &binding_token, &workspace_id, &source.document_id).await?.is_empty(), "reader must initially lack canonical source");
            let request = ProposalRequest { request_id: Some(format!("derivative-{}", Uuid::now_v7())), class: ProposalClass::Semantic, content: content.clone(), source: source.clone(), source_document_content: None, review_gated: Some(true), actor_id: None };
            eprintln!("MT109_DERIVATIVE_PHASE=owner_proposal");
            let owner_proposal_started = tokio::time::Instant::now();
            let response = auth(client.post(format!("{base}/workspaces/{workspace_id}/memory/proposals")), &owner_token)
                .json(&request)
                .send()
                .await
                .map_err(|error| {
                    eprintln!(
                        "MT109_DERIVATIVE_OWNER_PROPOSAL_ERROR elapsed_ms={} server_finished={} reconciler_finished={} leases_in_flight={} error={error}",
                        owner_proposal_started.elapsed().as_millis(),
                        server_task.as_ref().is_some_and(tokio::task::JoinHandle::is_finished),
                        reconciler_task.as_ref().is_some_and(tokio::task::JoinHandle::is_finished),
                        state.surreal.leases_in_flight(),
                    );
                    error
                })?;
            eprintln!(
                "MT109_DERIVATIVE_OWNER_PROPOSAL_OK elapsed_ms={}",
                owner_proposal_started.elapsed().as_millis()
            );
            let ack: ProposalAck = serde_json::from_value(mt109_memory_ok(response).await?)?;
            let proposal_url = format!("{base}/workspaces/{workspace_id}/memory/proposals/{}", ack.proposal_id);
            eprintln!("MT109_DERIVATIVE_PHASE=owner_review");
            mt109_memory_ok(auth(client.post(format!("{proposal_url}/review")), &owner_token).json(&json!({"decision":"approved","reviewer_kind":"user","reason":"source-bound derivative"})).send().await?).await?;
            eprintln!("MT109_DERIVATIVE_PHASE=owner_commit");
            let commit = mt109_memory_ok(auth(client.post(format!("{proposal_url}/commit")), &owner_token).json(&json!({})).send().await?).await?;
            let commit_id = commit["commit_id"].as_str().ok_or("missing commit id")?;
            let paths = [
                ("list", format!("{base}/workspaces/{workspace_id}/memory/proposals")),
                ("proposal", proposal_url.clone()), ("artifact", format!("{proposal_url}/artifact")),
                ("pack", format!("{base}/workspaces/{workspace_id}/memory/pack")),
                ("count", format!("{base}/workspaces/{workspace_id}/memory/items/count")),
                ("report", format!("{base}/workspaces/{workspace_id}/memory/commits/{commit_id}/report")),
            ];
            let mut canonical = std::collections::BTreeMap::new();
            for (name, url) in &paths { canonical.insert(*name, mt109_memory_ok(auth(client.get(url), &owner_token).send().await?).await?); }
            assert_eq!(canonical["proposal"]["document_id"], source.document_id);
            assert_eq!(canonical["proposal"]["selection_start"], source.selection_start);
            assert_eq!(canonical["proposal"]["selection_end"], source.selection_end);
            assert_eq!(canonical["proposal"]["content_hash"], source.content_hash);
            assert_eq!(canonical["proposal"]["proposal"]["content"], content);
            let artifact = &canonical["artifact"];
            assert_eq!(artifact["source_refs"][0]["id"], source.document_id);
            assert_eq!(artifact["source_refs"][0]["hash"], source.content_hash);
            assert_eq!(artifact["source_refs"][0]["selector"], format!("bytes:{}-{}", source.selection_start, source.selection_end));
            assert_eq!(artifact["ops"][0]["item"]["content"], content);
            assert_eq!(artifact["ops"][0]["item"]["provenance"]["source_refs"], artifact["source_refs"]);
            assert_eq!(canonical["pack"]["items"].as_array().ok_or("pack items missing")?.len(), 1);
            assert_eq!(canonical["pack"]["items"][0]["content"], content);
            assert_eq!(canonical["pack"]["items"][0]["source_refs"], artifact["source_refs"]);
            assert_eq!(canonical["count"]["count"], 1);
            let producer_canonical = mt109_proposal_rows_for_producer(
                &state,
                &owner,
                &workspace_id,
                &ack.proposal_id,
            )
            .await?;
            assert_eq!(producer_canonical.len(), 1, "the exact producer sees its source-backed proposal");
            let other_producer = mt109_memory_principal(
                &state,
                "derivative-account",
                "derivative-other-producer",
                "derivative-space",
                &[MEMORY_PROPOSE_CAPABILITY.to_owned()],
            )
            .await?;
            state.surreal.grant_resource(&other_producer.identity.account_id, &other_producer.identity.access_space_id, ResourceGrantSpec {
                principal_id: other_producer.identity.principal_id.clone(), resource_id: resources.proposal.clone(), actions: vec![ResourceAction::Create],
                capability_ids: vec![MEMORY_PROPOSE_CAPABILITY.to_owned()], expires_at: None, delegation_chain: Vec::new(),
            }).await?;
            state.surreal.grant_resource(&other_producer.identity.account_id, &other_producer.identity.access_space_id, ResourceGrantSpec {
                principal_id: other_producer.identity.principal_id.clone(), resource_id: source_resource.resource_id.clone(), actions: vec![ResourceAction::Read],
                capability_ids: vec![MEMORY_PROPOSE_CAPABILITY.to_owned()], expires_at: None, delegation_chain: Vec::new(),
            }).await?;
            assert!(
                mt109_proposal_rows_for_producer(&state, &other_producer, &workspace_id, &ack.proposal_id).await?.is_empty(),
                "proposal-create and source-read grants must not expose another actor's proposal"
            );
            state.surreal.revoke_grant(&source_grant.grant_id).await?;
            assert!(
                mt109_proposal_rows_for_producer(&state, &owner, &workspace_id, &ack.proposal_id).await?.is_empty(),
                "the producer loses direct proposal visibility when current source authority is revoked"
            );
            state.surreal.grant_resource(&owner.identity.account_id, &owner.identity.access_space_id, ResourceGrantSpec {
                principal_id: owner.identity.principal_id.clone(), resource_id: source_resource.resource_id.clone(), actions: vec![ResourceAction::Read],
                capability_ids: vec![MEMORY_PROPOSE_CAPABILITY.to_owned()], expires_at: None, delegation_chain: Vec::new(),
            }).await?;
            assert_eq!(
                mt109_proposal_rows_for_producer(&state, &owner, &workspace_id, &ack.proposal_id).await?,
                producer_canonical,
                "restoring the same source grant exposes the identical canonical proposal"
            );
            let before_reads = mt109_memory_rows(&state).await?;
            let mut failures = Vec::new();
            eprintln!("MT109_DERIVATIVE_PHASE=source_ungranted_reads");
            for (name, url) in &paths {
                let response = auth(client.get(url), &reader_token).send().await?;
                let status = response.status(); let body: Value = response.json().await?;
                let hidden = match *name {
                    "list" => status == StatusCode::OK && body.as_array().is_some_and(Vec::is_empty),
                    "pack" => status == StatusCode::OK && body["items"].as_array().is_some_and(Vec::is_empty),
                    "count" => status == StatusCode::OK && body["count"] == json!(0),
                    _ => status == StatusCode::NOT_FOUND,
                };
                let encoded = serde_json::to_string(&body)?;
                let disclosed = [&content, &source.document_id, &source.content_hash, &ack.proposal_id].iter().any(|private| encoded.contains(private.as_str()));
                if !hidden || disclosed { failures.push(format!("{name}: status={status}; body={body}")); }
            }
            eprintln!("MT109_DERIVATIVE_UNGRANTED_FAILURES={}", failures.join("\n"));
            assert_eq!(mt109_memory_rows(&state).await?, before_reads, "derivative GETs mutated protected rows");
            state.surreal.grant_resource(&reader.identity.account_id, &reader.identity.access_space_id, ResourceGrantSpec {
                principal_id: reader.identity.principal_id.clone(), resource_id: source_resource.resource_id.clone(), actions: vec![ResourceAction::Read],
                capability_ids: vec![MEMORY_READ_CAPABILITY.to_owned()], expires_at: None, delegation_chain: Vec::new(),
            }).await?;
            assert!(mt109_source_rows_for_reader(&state, &reader_token, &binding_token, &workspace_id, &second_source.document_id).await?.is_empty(), "reader must lack the second canonical source");
            let mixed_source_refs = vec![
                FemsSourceRef {
                    kind: FemsSourceRefKind::DocBlock,
                    id: source.document_id.clone(),
                    hash: Some(source.content_hash.clone()),
                    selector: Some(format!("bytes:{}-{}", source.selection_start, source.selection_end)),
                    created_at: None,
                    classification: Some("low".to_owned()),
                },
                FemsSourceRef {
                    kind: FemsSourceRefKind::DocBlock,
                    id: second_source.document_id.clone(),
                    hash: Some(second_source.content_hash.clone()),
                    selector: Some(format!("bytes:{}-{}", second_source.selection_start, second_source.selection_end)),
                    created_at: None,
                    classification: Some("low".to_owned()),
                },
            ];
            assert!(
                !mt109_source_refs_visible_for_reader(
                    &state,
                    &reader_token,
                    &binding_token,
                    &workspace_id,
                    &mixed_source_refs,
                )
                .await?,
                "a mixed-source derivative must fail closed when any referenced source is inaccessible"
            );
            for (name, url) in &paths {
                let readable = mt109_memory_ok(auth(client.get(url), &reader_token).send().await?).await?;
                assert_eq!(readable, canonical[*name], "source-granted same reader {name}");
            }
            assert_eq!(mt109_memory_rows(&state).await?, before_reads, "source-granted GETs mutated protected rows");
            let source_visible = mt109_source_rows_for_reader(&state, &reader_token, &binding_token, &workspace_id, &source.document_id).await?.len();
            assert!(owner.session.expires_at > chrono::Utc::now() && reader.session.expires_at > chrono::Utc::now(), "ordinary read probes must precede expiry");
            assert!(failures.is_empty(), "source-ungranted derivative disclosure: {}", failures.join("\n"));
            assert_eq!(source_visible, 1, "Read/memory.read source grant must suffice for a memory-only reader");
            Ok::<(), Box<dyn std::error::Error>>(())
        })).await;
        if let Some(tx) = shutdown_tx {
            let _ = tx.send(());
        }
        let mut drain_errors = Vec::new();
        if let Some(mut server) = server_task {
            match tokio::time::timeout(std::time::Duration::from_secs(30), &mut server).await {
                Ok(Ok(Ok(()))) => {}
                Ok(result) => drain_errors.push(format!("graceful server join failed: {result:?}")),
                Err(error) => {
                    server.abort();
                    let _ = server.await;
                    drain_errors.push(format!("graceful server drain timed out: {error}"));
                }
            }
        }
        if let Some(mut reconciler) = reconciler_task {
            match tokio::time::timeout(std::time::Duration::from_secs(30), &mut reconciler).await {
                Ok(Ok(())) => {}
                Ok(Err(error)) => drain_errors.push(format!("reconciler join failed: {error}")),
                Err(error) => {
                    reconciler.abort();
                    let _ = reconciler.await;
                    drain_errors.push(format!("reconciler drain timed out: {error}"));
                }
            }
        }
        drop(state);
        if !drain_errors.is_empty() {
            let body_detail = match &body {
                Ok(Ok(())) => "body passed".to_owned(),
                Ok(Err(error)) => error.to_string(),
                Err(payload) => payload
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| payload.downcast_ref::<&str>().map(|s| (*s).to_owned()))
                    .unwrap_or_else(|| "body panicked".to_owned()),
            };
            std::mem::forget(store);
            return Err(std::io::Error::other(format!(
                "{body_detail}; cleanup unproven, store preserved at {}: {}",
                data_dir.display(),
                drain_errors.join("; ")
            ))
            .into());
        }
        finish_mt109_memory_test(body, store, data_dir).await
    }

    #[tokio::test]
    async fn mt109_source_revocation_before_proposal_transaction_leaves_zero_residue(
    ) -> Result<(), Box<dyn std::error::Error>> {
        use crate::storage::surreal::keyed_lock::race_test_support;
        use crate::storage::surreal::resource_authority::{
            AuthorizationRequest, RecordUserScope, ResourceAction, ResourceGrantSpec, ResourceKind,
        };

        eprintln!("MT109_SOURCE_REVOCATION_PHASE=setup_start");
        let (store, state) = setup_state().await?;
        eprintln!("MT109_SOURCE_REVOCATION_PHASE=setup_complete");
        let data_dir = store.data_dir.clone();
        let body = futures::FutureExt::catch_unwind(std::panic::AssertUnwindSafe(async {
            eprintln!("MT109_SOURCE_REVOCATION_PHASE=fixture_start");
            let workspace_id = create_test_workspace(&state, "memory-source-revocation").await?;
            let content = format!("revoked-source-{}", Uuid::now_v7());
            let source = create_test_rich_source(
                &state,
                &workspace_id,
                "source-revocation",
                &content,
            )
            .await?;
            let owner = mt109_memory_principal(
                &state,
                "source-revocation-account",
                "source-revocation-owner",
                "source-revocation-space",
                &mt109_memory_capabilities(),
            )
            .await?;
            let resources =
                mt109_register_memory_resources(&state, &owner, &workspace_id).await?;
            mt109_grant_memory_resources(&state, &owner, &resources, Mt109GrantMode::Full)
                .await?;
            let source_resource = state
                .surreal
                .register_protected_resource(
                    &owner.identity,
                    ResourceKind::RichDocument,
                    &source.document_id,
                    Some(&resources.workspace),
                    "account_private",
                )
                .await?;
            let source_grant = state
                .surreal
                .grant_resource(
                    &owner.identity.account_id,
                    &owner.identity.access_space_id,
                    ResourceGrantSpec {
                        principal_id: owner.identity.principal_id.clone(),
                        resource_id: source_resource.resource_id,
                        actions: vec![ResourceAction::Read],
                        capability_ids: vec![MEMORY_PROPOSE_CAPABILITY.to_owned()],
                        expires_at: None,
                        delegation_chain: Vec::new(),
                    },
                )
                .await?;
            let decision = state
                .surreal
                .authorize_protected_resource(AuthorizationRequest {
                    session_token: owner.session.token.clone(),
                    channel_binding_hash: None,
                    capability_id: MEMORY_PROPOSE_CAPABILITY.to_owned(),
                    resource_kind: ResourceKind::MemoryProposal,
                    external_resource_id: workspace_id.clone(),
                    action: ResourceAction::Create,
                })
                .await?;
            eprintln!("MT109_SOURCE_REVOCATION_PHASE=authority_ready");
            let scope = RecordUserScope {
                workspace_id: Some(workspace_id.clone()),
                session_token: owner.session.token.clone(),
                channel_binding_hash: None,
                resource_id: decision.resource_id,
                session_id: decision.session_id,
                capability_id: MEMORY_PROPOSE_CAPABILITY.to_owned(),
                action: ResourceAction::Create,
            };
            let request = ProposalRequest {
                request_id: Some(format!("source-revocation-{}", Uuid::now_v7())),
                class: ProposalClass::Semantic,
                content,
                source,
                source_document_content: None,
                review_gated: Some(true),
                actor_id: None,
            };
            let before = mt109_memory_rows(&state).await?;
            assert!(
                before.values().all(Vec::is_empty),
                "revocation proof must start without FEMS residue: {before:?}"
            );
            eprintln!("MT109_SOURCE_REVOCATION_PHASE=operation_construct");

            let reached = Arc::new(tokio::sync::Barrier::new(2));
            let release = Arc::new(tokio::sync::Barrier::new(2));
            let handler = Box::pin(create_memory_proposal(
                State(state.clone()),
                Path(workspace_id),
                HeaderMap::new(),
                Json(request),
            ));
            let paused = Box::pin(
                race_test_support::with_pause_after_decision_until_released(
                    Arc::clone(&reached),
                    Arc::clone(&release),
                    handler,
                ),
            );
            let operation = Box::pin(state.surreal.with_record_user_scope(scope, paused));
            let operation_finished = Arc::new(AtomicBool::new(false));
            let operation_finished_after = Arc::clone(&operation_finished);
            let operation = async move {
                let result = operation.await;
                operation_finished_after.store(true, Ordering::SeqCst);
                result
            };
            let revoke_finished = Arc::new(AtomicBool::new(false));
            let revoke_finished_after = Arc::clone(&revoke_finished);
            let revoke = async {
                reached.wait().await;
                let result = state.surreal.revoke_grant(&source_grant.grant_id).await;
                release.wait().await;
                revoke_finished_after.store(true, Ordering::SeqCst);
                result
            };
            let joined = tokio::time::timeout(
                std::time::Duration::from_secs(300),
                async { tokio::join!(operation, revoke) },
            )
            .await;
            if joined.is_err() {
                eprintln!(
                    "MT109_SOURCE_REVOCATION_TIMEOUT operation_finished={} revoke_finished={} leases_in_flight={}",
                    operation_finished.load(Ordering::SeqCst),
                    revoke_finished.load(Ordering::SeqCst),
                    state.surreal.leases_in_flight()
                );
            }
            let (proposal_result, revoke_result) = joined.map_err(|error| {
                std::io::Error::other(format!("revocation race timed out: {error}"))
            })?;
            eprintln!("MT109_SOURCE_REVOCATION_PHASE=operation_joined");
            revoke_result?;
            let (status, body) =
                proposal_result.expect_err("revoked source must fail before proposal transaction");
            assert_eq!(status, StatusCode::FORBIDDEN);
            assert_eq!(body.0, json!({"error": "HSK-403-PROTECTED-RESOURCE"}));
            assert_eq!(
                mt109_memory_rows(&state).await?,
                before,
                "revocation before the authoritative transaction left FEMS, ledger, or outbox residue"
            );
            eprintln!("MT109_SOURCE_REVOCATION_BEFORE_TRANSACTION=DENIED_ZERO_RESIDUE");
            Ok::<(), Box<dyn std::error::Error>>(())
        }))
        .await;
        drop(state);
        finish_mt109_memory_test(body, store, data_dir).await
    }

    #[tokio::test]
    async fn mt109_memory_reconciler_processes_only_explicitly_granted_pending_workspace(
    ) -> Result<(), Box<dyn std::error::Error>> {
        use crate::storage::surreal::resource_authority::{
            AuthorizationRequest, ResourceAction, ResourceKind,
        };
        let (store, state) = setup_state().await?;
        let data_dir = store.data_dir.clone();
        let body = futures::FutureExt::catch_unwind(std::panic::AssertUnwindSafe(async {
            let granted_workspace = create_test_workspace(&state, "reconcile-granted").await?;
            let denied_workspace = create_test_workspace(&state, "reconcile-denied").await?;
            let (granted_proposal, _) = create_and_approve_test_proposal(
                &state,
                &granted_workspace,
                "reconcile-granted",
                "granted pending memory",
            )
            .await?;
            let (denied_proposal, _) = create_and_approve_test_proposal(
                &state,
                &denied_workspace,
                "reconcile-denied",
                "denied pending memory",
            )
            .await?;

            for (workspace_id, proposal_id) in [
                (&granted_workspace, &granted_proposal.proposal_id),
                (&denied_workspace, &denied_proposal.proposal_id),
            ] {
                let receipt = NewKernelEvent::builder(
                    format!("reconcile-proof-{workspace_id}"),
                    "reconcile-proof-session".to_owned(),
                    KernelEventType::ArtifactStored,
                    KernelActor::System("reconcile-proof-seeder".to_owned()),
                )
                .aggregate("fems_memory_commit", proposal_id.clone())
                .idempotency_key(format!("fems-memory-commit:{proposal_id}"))
                .correlation_id(format!("fems-memory-proposal:{proposal_id}"))
                .source_component("fems_memory_proposal_commit")
                .payload(json!({"proposal_id": proposal_id}))
                .build()?;
                fems_memory::commit_memory_proposal_with_receipt(
                    &state.surreal,
                    workspace_id,
                    proposal_id,
                    receipt,
                )
                .await?;
            }
            assert_eq!(
                fems_memory::list_pending_memory_commit_events(
                    &state.surreal,
                    &granted_workspace,
                    200,
                )
                .await?
                .len(),
                2
            );
            assert_eq!(
                fems_memory::list_pending_memory_commit_events(
                    &state.surreal,
                    &denied_workspace,
                    200,
                )
                .await?
                .len(),
                2
            );
            let denied_trace = deterministic_uuid_from_seed(&format!(
                "fems-memory-proposal:{}",
                denied_proposal.proposal_id
            ));
            let denied_residue_before =
                mt109_memory_residue(&state, &denied_workspace).await?;
            let denied_events_before = state
                .flight_recorder
                .list_events(EventFilter {
                    trace_id: Some(denied_trace),
                    ..EventFilter::default()
                })
                .await?;
            let denied_event_snapshot_before = denied_events_before
                .iter()
                .map(|event| {
                    (
                        event.event_id,
                        event.event_type.clone(),
                        event.payload.clone(),
                    )
                })
                .collect::<Vec<_>>();
            eprintln!(
                "MT109_DENIED_TRACE_BASELINE_TYPES={:?}",
                denied_event_snapshot_before
                    .iter()
                    .map(|(_, event_type, _)| event_type)
                    .collect::<Vec<_>>()
            );

            let service = state
                .surreal
                .provision_reconciliation_principal(std::slice::from_ref(&granted_workspace), None)
                .await?;
            let workspace_queue_decision = state
                .surreal
                .authorize_protected_resource(AuthorizationRequest {
                    session_token: service.session.token.clone(),
                    channel_binding_hash: None,
                    capability_id: MEMORY_COMMIT_CAPABILITY.to_owned(),
                    resource_kind: ResourceKind::ReconciliationQueue,
                    external_resource_id: format!(
                        "{}:{granted_workspace}",
                        crate::storage::surreal::resource_authority::RECONCILIATION_QUEUE_ID
                    ),
                    action: ResourceAction::Reconcile,
                })
                .await?;
            let service_principal =
                RecordId::new("principals", service.identity.principal_id.as_str());
            let persisted_profile = state
                .surreal
                .with_data_operation(move |database| {
                    Box::pin(async move {
                        database
                            .query_first::<ReconciliationPrincipalProfileRow, _>(
                                "SELECT capability_profile_id, actor_id FROM ONLY $principal;",
                                ReconciliationPrincipalProfileBinding {
                                    principal: service_principal,
                                },
                            )
                            .await
                    })
                })
                .await?
                .expect("persisted reconciliation Principal");
            assert_eq!(
                persisted_profile.capability_profile_id,
                crate::storage::surreal::resource_authority::RECONCILIATION_PROFILE_ID
            );
            assert_eq!(persisted_profile.actor_id, "mt109_reconciler");
            reconcile_all_memory_commit_events(&state)
                .await
                .map_err(|(status, body)| format!("service reconcile failed: {status} {body:?}"))?;

            assert!(
                fems_memory::list_pending_memory_commit_events(
                    &state.surreal,
                    &granted_workspace,
                    200,
                )
                .await?
                .is_empty(),
                "the explicitly granted service principal drains its workspace"
            );
            assert_eq!(
                fems_memory::list_pending_memory_commit_events(
                    &state.surreal,
                    &denied_workspace,
                    200,
                )
                .await?
                .len(),
                2,
                "the same worker must neither observe nor mutate an ungranted pending workspace"
            );
            let granted_trace = deterministic_uuid_from_seed(&format!(
                "fems-memory-proposal:{}",
                granted_proposal.proposal_id
            ));
            assert!(!state
                .flight_recorder
                .list_events(EventFilter {
                    trace_id: Some(granted_trace),
                    ..EventFilter::default()
                })
                .await?
                .is_empty());
            let denied_event_snapshot_after = state
                .flight_recorder
                .list_events(EventFilter {
                    trace_id: Some(denied_trace),
                    ..EventFilter::default()
                })
                .await?
                .into_iter()
                .map(|event| (event.event_id, event.event_type, event.payload))
                .collect::<Vec<_>>();
            eprintln!(
                "MT109_DENIED_TRACE_AFTER_TYPES={:?}",
                denied_event_snapshot_after
                    .iter()
                    .map(|(_, event_type, _)| event_type)
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                denied_event_snapshot_after, denied_event_snapshot_before,
                "reconciliation emitted or changed Flight Recorder events for the ungranted workspace"
            );
            assert_eq!(
                mt109_memory_residue(&state, &denied_workspace).await?,
                denied_residue_before,
                "reconciliation changed proposal, memory, outbox, anchor, or ledger state for the ungranted workspace"
            );
            let visibility_authority = crate::api::authority::reconciliation_authority(
                &state,
                MEMORY_COMMIT_CAPABILITY,
            )
            .await
            .map_err(std::io::Error::other)?;
            let visibility_session_token = visibility_authority
                .record_user_scope
                .session_token
                .clone();
            let workspace_queue_scope = visibility_authority.record_user_scope;
            assert_eq!(
                mt109_workspace_ids_in_reconciliation_scope(
                    &state,
                    workspace_queue_scope.clone(),
                    vec![granted_workspace.clone(), denied_workspace.clone()],
                )
                .await?,
                vec![granted_workspace.clone()],
                "the service identity must enumerate only its exactly granted workspace"
            );
            state
                .surreal
                .revoke_grant(&workspace_queue_decision.grant_id)
                .await?;
            assert!(
                state
                    .surreal
                    .authorize_protected_resource(AuthorizationRequest {
                        session_token: visibility_session_token,
                        channel_binding_hash: None,
                        capability_id: MEMORY_COMMIT_CAPABILITY.to_owned(),
                        resource_kind: ResourceKind::ReconciliationQueue,
                        external_resource_id: format!(
                            "{}:{granted_workspace}",
                            crate::storage::surreal::resource_authority::RECONCILIATION_QUEUE_ID
                        ),
                        action: ResourceAction::Reconcile,
                    })
                    .await
                    .is_err(),
                "revoked exact reconciliation grant must fail authorization"
            );
            assert!(
                mt109_workspace_ids_in_reconciliation_scope(
                    &state,
                    workspace_queue_scope,
                    vec![granted_workspace, denied_workspace],
                )
                .await?
                .is_empty(),
                "the same service session must lose workspace visibility immediately after exact queue revocation"
            );
            Ok::<(), Box<dyn std::error::Error>>(())
        }))
        .await;
        drop(state);
        finish_mt109_memory_test(body, store, data_dir).await
    }

    #[tokio::test]
    async fn concurrent_workspace_commits_publish_a_pack_containing_both_items(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (store, state) = setup_state().await?;
        let data_dir = store.data_dir.clone();
        let body = futures::FutureExt::catch_unwind(std::panic::AssertUnwindSafe(async {
        let workspace_id = create_test_workspace(&state, "memory-concurrent-commit").await?;
        let (proposal_a, _) = create_and_approve_test_proposal(
            &state,
            &workspace_id,
            "concurrent-shared-owner",
            "first concurrent memory",
        )
        .await?;
        let (proposal_b, _) = create_and_approve_test_proposal(
            &state,
            &workspace_id,
            "concurrent-shared-owner",
            "second concurrent memory",
        )
        .await?;
        let mut headers = HeaderMap::new();
        headers.insert(HSK_HEADER_ACTOR_ID, "concurrent-operator".parse()?);
        headers.insert(HSK_HEADER_ACTOR_KIND, "operator".parse()?);
        let commit_a = commit_memory_proposal(
            State(state.clone()),
            Path((workspace_id.clone(), proposal_a.proposal_id)),
            headers.clone(),
        );
        let commit_b = commit_memory_proposal(
            State(state.clone()),
            Path((workspace_id.clone(), proposal_b.proposal_id)),
            headers,
        );
        let (commit_a, commit_b) = tokio::join!(commit_a, commit_b);
        let commit_a = commit_a
            .map_err(|(status, body)| format!("concurrent commit A failed: {status} {body:?}"))?
            .0;
        let commit_b = commit_b
            .map_err(|(status, body)| format!("concurrent commit B failed: {status} {body:?}"))?
            .0;
        assert_ne!(commit_a.memory_id, commit_b.memory_id);

        let latest_pack =
            fems_memory::get_latest_memory_pack(&state.surreal, &workspace_id, Some("workspace"))
                .await?
                .expect("latest concurrent pack");
        let ids = latest_pack
            .items
            .iter()
            .map(|item| item.memory_id.as_str())
            .collect::<std::collections::HashSet<_>>();
        assert!(ids.contains(commit_a.memory_id.as_str()));
        assert!(ids.contains(commit_b.memory_id.as_str()));
        assert_eq!(
            fems_memory::count_memory_items(&state.surreal, &workspace_id).await?,
            2
        );
        Ok::<(), Box<dyn std::error::Error>>(())
        }))
        .await;
        drop(state);
        finish_mt109_memory_test(body, store, data_dir).await
    }

    fn seeded_pack(workspace_id: &str) -> MemoryPack {
        let item = MemoryPackItem {
            memory_id: "MEM-EP-001".to_string(),
            memory_class: "episodic".to_string(),
            item_type: "note".to_string(),
            summary: "Aria refuses the summons".to_string(),
            content: "The protagonist Aria refuses the royal summons.".to_string(),
            structured: None,
            trust_level: "medium".to_string(),
            confidence: 0.9,
            scope_refs: Vec::new(),
            source_refs: vec![FemsSourceRef {
                kind: FemsSourceRefKind::DocBlock,
                id: "doc-block-42".to_string(),
                hash: Some("a".repeat(64)),
                selector: Some("0:47".to_string()),
                created_at: None,
                classification: None,
            }],
            pinned: false,
            last_verified_at: None,
        };
        let mut pack = MemoryPack {
            schema_version: PACK_SCHEMA_VERSION.to_string(),
            pack_id: format!("PACK-{workspace_id}"),
            generated_at: chrono::Utc::now().to_rfc3339(),
            determinism_mode: MemoryPackDeterminismMode::Strict,
            memory_policy: MemoryPolicy::WorkspaceScoped,
            scope_refs: Vec::new(),
            budgets: MemoryPackBudgets {
                max_tokens: 500,
                max_items: 24,
                max_items_per_type: std::collections::BTreeMap::new(),
            },
            items: vec![item],
            token_estimate: 12,
            memory_pack_hash: String::new(),
            warnings: Vec::new(),
        };
        pack.memory_pack_hash = pack.compute_hash().expect("hash");
        pack
    }

    async fn create_test_workspace(state: &AppState, name: &str) -> Result<String, StorageError> {
        state
            .storage
            .create_workspace(
                &WriteContext::human(Some("fems-memory-test".to_owned())),
                NewWorkspace {
                    name: format!("{name}-{}", Uuid::now_v7()),
                },
            )
            .await
            .map(|workspace| workspace.id)
    }

    fn valid_source(workspace_id: &str, document_id: &str, content: &str) -> ProposalSource {
        ProposalSource {
            document_id: document_id.to_string(),
            selection_start: 3,
            selection_end: 3 + content.len() as u64,
            content_hash: canonical_content_hash(content).expect("canonical content hash"),
            document_content_hash: None,
            pane_id: Some("pane-rich".to_string()),
            workspace_id: Some(workspace_id.to_string()),
        }
    }

    async fn create_test_rich_source(
        state: &AppState,
        workspace_id: &str,
        label: &str,
        content: &str,
    ) -> Result<ProposalSource, StorageError> {
        let document_text = format!("xxx{content}");
        let db = SurrealDatabase::new(state.surreal.clone());
        let document = db
            .create_knowledge_rich_document(NewKnowledgeRichDocument {
                workspace_id: workspace_id.to_owned(),
                document_id: None,
                title: format!("{label}-{}", Uuid::now_v7()),
                schema_version: "hsk_richdoc_v1".to_owned(),
                content_json: json!({
                    "type": "doc",
                    "content": [{
                        "type": "paragraph",
                        "content": [{"type": "text", "text": document_text}]
                    }]
                }),
                crdt_document_id: None,
                crdt_snapshot_id: None,
                promotion_receipt_event_id: None,
                ..Default::default()
            })
            .await?;
        Ok(valid_source(
            workspace_id,
            &document.rich_document_id,
            content,
        ))
    }

    async fn proposal_create_test_scope(
        state: &AppState,
        workspace_id: &str,
        document_id: &str,
        label: &str,
        actor_id: &str,
    ) -> Result<crate::storage::surreal::resource_authority::RecordUserScope, Box<dyn std::error::Error>> {
        use crate::storage::surreal::resource_authority::{
            AuthorizationRequest, RecordUserScope, ResourceAction, ResourceGrantSpec,
            ResourceKind,
        };
        let principal = mt109_memory_principal(
            state,
            &format!("{label}-account"),
            actor_id,
            &format!("{label}-space"),
            &[
                MEMORY_READ_CAPABILITY.to_owned(),
                MEMORY_PROPOSE_CAPABILITY.to_owned(),
            ],
        )
        .await?;
        let workspace_resource = state
            .surreal
            .register_protected_resource(
                &principal.identity,
                ResourceKind::Workspace,
                workspace_id,
                None,
                "account_private",
            )
            .await?;
        let proposal_resource = state
            .surreal
            .register_protected_resource(
                &principal.identity,
                ResourceKind::MemoryProposal,
                workspace_id,
                Some(&workspace_resource.resource_id),
                "account_private",
            )
            .await?;
        for resource_id in [
            workspace_resource.resource_id.as_str(),
            proposal_resource.resource_id.as_str(),
        ] {
            state
                .surreal
                .grant_resource(
                    &principal.identity.account_id,
                    &principal.identity.access_space_id,
                    ResourceGrantSpec {
                        principal_id: principal.identity.principal_id.clone(),
                        resource_id: resource_id.to_owned(),
                        actions: vec![ResourceAction::Create],
                        capability_ids: vec![MEMORY_PROPOSE_CAPABILITY.to_owned()],
                        expires_at: None,
                        delegation_chain: Vec::new(),
                    },
                )
                .await?;
        }
        state
            .surreal
            .grant_resource(
                &principal.identity.account_id,
                &principal.identity.access_space_id,
                ResourceGrantSpec {
                    principal_id: principal.identity.principal_id.clone(),
                    resource_id: workspace_resource.resource_id.clone(),
                    actions: vec![ResourceAction::Read],
                    capability_ids: vec![MEMORY_READ_CAPABILITY.to_owned()],
                    expires_at: None,
                    delegation_chain: Vec::new(),
                },
            )
            .await?;
        let source_resource = state
            .surreal
            .register_protected_resource(
                &principal.identity,
                ResourceKind::RichDocument,
                document_id,
                Some(&workspace_resource.resource_id),
                "account_private",
            )
            .await?;
        state
            .surreal
            .grant_resource(
                &principal.identity.account_id,
                &principal.identity.access_space_id,
                ResourceGrantSpec {
                    principal_id: principal.identity.principal_id.clone(),
                    resource_id: source_resource.resource_id,
                    actions: vec![ResourceAction::Read],
                    capability_ids: vec![MEMORY_PROPOSE_CAPABILITY.to_owned()],
                    expires_at: None,
                    delegation_chain: Vec::new(),
                },
            )
            .await?;
        let decision = state
            .surreal
            .authorize_protected_resource(AuthorizationRequest {
                session_token: principal.session.token.clone(),
                channel_binding_hash: None,
                capability_id: MEMORY_PROPOSE_CAPABILITY.to_owned(),
                resource_kind: ResourceKind::MemoryProposal,
                external_resource_id: workspace_id.to_owned(),
                action: ResourceAction::Create,
            })
            .await?;
        Ok(RecordUserScope {
            workspace_id: Some(workspace_id.to_owned()),
            session_token: principal.session.token,
            channel_binding_hash: None,
            resource_id: decision.resource_id,
            session_id: decision.session_id,
            capability_id: MEMORY_PROPOSE_CAPABILITY.to_owned(),
            action: ResourceAction::Create,
        })
    }

    async fn create_and_approve_test_proposal(
        state: &AppState,
        workspace_id: &str,
        label: &str,
        content: &str,
    ) -> Result<(ProposalAck, ProposalReviewAck), Box<dyn std::error::Error>> {
        let source = create_test_rich_source(state, workspace_id, label, content).await?;
        let actor_id = format!("{label}-principal");
        let proposal_scope =
            proposal_create_test_scope(state, workspace_id, &source.document_id, label, &actor_id)
                .await?;
        let mut headers = HeaderMap::new();
        headers.insert(HSK_HEADER_ACTOR_ID, actor_id.parse()?);
        headers.insert(HSK_HEADER_ACTOR_KIND, "operator".parse()?);
        let create = create_memory_proposal(
                State(state.clone()),
                Path(workspace_id.to_owned()),
                headers.clone(),
                Json(ProposalRequest {
                    request_id: Some(format!("{label}-{}", Uuid::now_v7())),
                    class: ProposalClass::Semantic,
                    content: content.to_owned(),
                    source,
                    source_document_content: None,
                    review_gated: Some(true),
                    actor_id: Some(actor_id),
                }),
            );
        let Json(proposal) = state
            .surreal
            .with_record_user_scope(proposal_scope, create)
            .await
        .map_err(|(status, body)| format!("proposal failed: {status} {body:?}"))?;
        let Json(review) = review_memory_proposal(
            State(state.clone()),
            Path((workspace_id.to_owned(), proposal.proposal_id.clone())),
            headers,
            Json(ProposalReviewRequest {
                decision: ProposalReviewDecision::Approved,
                reviewer_kind: ProposalReviewerKind::User,
                reason: Some("test source verified".to_owned()),
            }),
        )
        .await
        .map_err(|(status, body)| format!("review failed: {status} {body:?}"))?;
        Ok((proposal, review))
    }

    #[test]
    fn content_hash_matches_frontend_canonical_json_string_bytes() {
        let content = "line\n\"quoted\" ü\u{0000}\u{0008}\u{000c}";
        let hash = canonical_content_hash(content).expect("canonical content hash");
        assert_eq!(
            hash,
            crate::kernel::context_bundle::sha256_hex(
                &crate::kernel::context_bundle::canonical_json_bytes(&Value::String(
                    content.to_owned()
                ))
            )
        );
        assert_ne!(
            hash,
            hex::encode(Sha256::digest(
                serde_json::to_vec(&Value::String(content.to_owned())).expect("serde JSON string")
            )),
            "the shared Loom writer intentionally differs from serde for nonstandard controls"
        );
    }

    #[tokio::test]
    async fn surreal_identity_and_exact_retry_converge() -> Result<(), Box<dyn std::error::Error>> {
        let (store, state) = setup_state().await?;
        let data_dir = store.data_dir.clone();
        let body = futures::FutureExt::catch_unwind(std::panic::AssertUnwindSafe(async {
        let workspace_id = create_test_workspace(&state, "surreal-identity-retry").await?;
        let content = "unicode identity\u{2003}";
        let source =
            create_test_rich_source(&state, &workspace_id, "surreal-identity-source", content)
                .await?;
        let proposal_scope = proposal_create_test_scope(
            &state,
            &workspace_id,
            &source.document_id,
            "surreal-identity",
            "native_editor",
        )
        .await?;
        let request = ProposalRequest {
            request_id: None,
            class: ProposalClass::Semantic,
            content: content.to_owned(),
            source,
            source_document_content: None,
            review_gated: Some(true),
            actor_id: Some("identity-actor".to_owned()),
        };
        let expected_request_id =
            stable_proposal_request_id(&workspace_id, &request).expect("stable request identity");

        let first_call = create_memory_proposal(
            State(state.clone()),
            Path(workspace_id.clone()),
            HeaderMap::new(),
            Json(request.clone()),
        );
        let Json(first) = state
            .surreal
            .with_record_user_scope(proposal_scope.clone(), first_call)
            .await
        .map_err(|(status, body)| format!("first proposal failed: {status} {body:?}"))?;
        let retry_call = create_memory_proposal(
            State(state.clone()),
            Path(workspace_id.clone()),
            HeaderMap::new(),
            Json(request),
        );
        let Json(retry) = state
            .surreal
            .with_record_user_scope(proposal_scope, retry_call)
            .await
        .map_err(|(status, body)| format!("proposal retry failed: {status} {body:?}"))?;

        assert_eq!(retry, first, "an exact SurrealDB retry must converge");
        let stored = fems_memory::get_memory_proposal(&state.surreal, &first.proposal_id)
            .await?
            .expect("proposal stored");
        assert_eq!(stored.request_id, expected_request_id);
        assert_eq!(
            fems_memory::list_memory_proposals(&state.surreal, &workspace_id, 200)
                .await?
                .len(),
            1,
            "exact retries must not duplicate the durable proposal"
        );
            Ok::<(), Box<dyn std::error::Error>>(())
        }))
        .await;
        drop(state);
        finish_mt109_memory_test(body, store, data_dir).await
    }
    #[tokio::test]
    async fn canonical_request_identity_matches_embedded_contract_across_actor_and_hash_edges(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let workspace_id = format!("WS-MT112-{}", Uuid::now_v7());
        let content = "identity edge content";
        let mut without_actor = ProposalRequest {
            request_id: None,
            class: ProposalClass::Semantic,
            content: content.to_owned(),
            source: valid_source(&workspace_id, "doc-mt112", content),
            source_document_content: None,
            review_gated: Some(true),
            actor_id: None,
        };
        let first = stable_proposal_request_id(&workspace_id, &without_actor)
            .expect("canonical identity without actor");
        without_actor.actor_id = Some("different-authenticated-session".to_owned());
        let actor_retry = stable_proposal_request_id(&workspace_id, &without_actor)
            .expect("canonical identity with actor");
        assert_eq!(
            actor_retry, first,
            "authenticated actor changes must not fork an otherwise exact retry"
        );

        without_actor.content.push('!');
        without_actor.source.content_hash =
            canonical_content_hash(&without_actor.content).expect("changed content hash");
        let changed = stable_proposal_request_id(&workspace_id, &without_actor)
            .expect("changed-content identity");
        assert_ne!(
            changed, first,
            "a changed durable proposal payload must not collide with the exact-retry identity"
        );

        let explicit = ProposalRequest {
            request_id: Some("explicit-retry-id".to_owned()),
            ..without_actor
        };
        assert_eq!(
            stable_proposal_request_id(&workspace_id, &explicit)
                .expect("an explicit request_id derives a stable identity"),
            "explicit-retry-id"
        );
        Ok(())
    }

    /// AC-109-2: GET returns the REAL ace::MemoryPack shape; asserted field-by-field so
    /// the native client alignment has a pinned contract.
    #[tokio::test]
    async fn get_memory_pack_returns_real_ace_shape() -> Result<(), Box<dyn std::error::Error>> {
        let (store, state) = setup_state().await?;
        let data_dir = store.data_dir.clone();
        let body = futures::FutureExt::catch_unwind(std::panic::AssertUnwindSafe(async {
        let workspace_id = create_test_workspace(&state, "pack").await?;
        let source = create_test_rich_source(
            &state,
            &workspace_id,
            "pack-source",
            "The protagonist Aria refuses the royal summons.",
        )
        .await?;
        let mut pack = seeded_pack(&workspace_id);
        pack.items[0].source_refs[0].id = source.document_id.clone();
        pack.items[0].source_refs[0].hash = Some(source.content_hash.clone());
        pack.items[0].source_refs[0].selector = Some(format!(
            "bytes:{}-{}",
            source.selection_start, source.selection_end
        ));
        pack.memory_pack_hash = pack.compute_hash()?;
        fems_memory::upsert_memory_pack(&state.surreal, &workspace_id, "", &pack).await?;

        let Json(got) = get_memory_pack(
            State(state.clone()),
            Path(workspace_id.clone()),
            Query(PackQuery::default()),
        )
        .await
        .map_err(|(code, body)| format!("get pack failed: {code} {body:?}"))?;

        assert_eq!(
            got, pack,
            "the SurrealDB route must return every MemoryPack field unchanged"
        );
        assert_eq!(
            serde_json::to_value(&got)?,
            serde_json::to_value(&pack)?,
            "the complete serialized MemoryPack response shape must match authority"
        );

        // Pack-level shape.
        assert_eq!(got.pack_id, format!("PACK-{workspace_id}"));
        assert_eq!(got.schema_version, PACK_SCHEMA_VERSION);
        assert_eq!(got.items.len(), 1);

        // Item-level shape: the exact fields the native client aligns to.
        let item = &got.items[0];
        assert_eq!(item.memory_id, "MEM-EP-001");
        assert_eq!(item.memory_class, "episodic");
        assert_eq!(item.summary, "Aria refuses the summons");
        assert_eq!(item.source_refs.len(), 1);
        assert_eq!(item.source_refs[0].kind, FemsSourceRefKind::DocBlock);
        assert_eq!(item.source_refs[0].id, source.document_id);
        assert_eq!(
            item.source_refs[0].hash.as_deref(),
            Some(source.content_hash.as_str())
        );
        Ok::<(), Box<dyn std::error::Error>>(())
        })).await;
        drop(state);
        finish_mt109_memory_test(body, store, data_dir).await
    }

    /// GET on a real workspace with no stored pack returns one deterministic, ephemeral empty pack;
    /// repeated attacker-chosen contexts cannot amplify persisted rows, and an unknown workspace fails
    /// closed instead of fabricating workspace authority.
    #[tokio::test]
    async fn get_memory_pack_empty_is_deterministic_and_unknown_workspace_is_not_found(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (store, state) = setup_state().await?;
        let data_dir = store.data_dir.clone();
        let body = futures::FutureExt::catch_unwind(std::panic::AssertUnwindSafe(async {
        let workspace_id = create_test_workspace(&state, "empty-pack").await?;
        let query = PackQuery {
            context: Some("mounted-editor-context".to_owned()),
            ..PackQuery::default()
        };
        let Json(first) = get_memory_pack(
            State(state.clone()),
            Path(workspace_id.clone()),
            Query(query),
        )
        .await
        .map_err(|(code, body)| format!("get pack failed: {code} {body:?}"))?;
        let Json(second) = get_memory_pack(
            State(state.clone()),
            Path(workspace_id.clone()),
            Query(PackQuery {
                context: Some("mounted-editor-context".to_owned()),
                ..PackQuery::default()
            }),
        )
        .await
        .map_err(|(code, body)| format!("retry get pack failed: {code} {body:?}"))?;
        assert!(first.items.is_empty());
        assert_eq!(first.schema_version, PACK_SCHEMA_VERSION);
        assert_eq!(
            second, first,
            "empty-pack retries are byte-shape deterministic"
        );
        assert_eq!(
            fems_memory::get_latest_memory_pack(
                &state.surreal,
                &workspace_id,
                Some("mounted-editor-context")
            )
            .await?,
            None,
            "a GET miss must not persist an empty projection"
        );

        let count_pack_rows = || {
            memory_test_count(
                &state,
                "SELECT count() AS count FROM fems_memory_packs \
                 WHERE workspace_id = $workspace GROUP ALL;",
                MemoryTestBindings {
                    workspace: Some(RecordId::new("workspaces", workspace_id.as_str())),
                    ..MemoryTestBindings::default()
                },
            )
        };
        let before_rows = count_pack_rows().await?;
        for index in 0..2_000_u32 {
            let Json(pack) = get_memory_pack(
                State(state.clone()),
                Path(workspace_id.clone()),
                Query(PackQuery {
                    context: Some(format!("attacker-context-{index}")),
                    ..PackQuery::default()
                }),
            )
            .await
            .map_err(|(code, body)| format!("amplification read failed: {code} {body:?}"))?;
            assert!(pack.items.is_empty());
        }
        let after_rows = count_pack_rows().await?;
        assert_eq!(
            after_rows, before_rows,
            "caller-controlled GET contexts must not create or update memory-pack rows"
        );

        let oversized = get_memory_pack(
            State(state.clone()),
            Path(workspace_id.clone()),
            Query(PackQuery {
                context: Some("x".repeat(1_025)),
                ..PackQuery::default()
            }),
        )
        .await
        .expect_err("oversized context must fail before a database lookup");
        assert_eq!(oversized.0, StatusCode::BAD_REQUEST);

        let unknown = get_memory_pack(
            State(state.clone()),
            Path(Uuid::now_v7().to_string()),
            Query(PackQuery::default()),
        )
        .await
        .expect_err("unknown workspace must not receive a fabricated empty pack");
        assert_eq!(unknown.0, StatusCode::NOT_FOUND);
        Ok::<(), Box<dyn std::error::Error>>(())
        })).await;
        drop(state);
        finish_mt109_memory_test(body, store, data_dir).await
    }

    #[tokio::test]
    async fn memory_pack_and_item_ids_cannot_be_reassigned_across_workspaces(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (store, state) = setup_state().await?;
        let data_dir = store.data_dir.clone();
        let body = futures::FutureExt::catch_unwind(std::panic::AssertUnwindSafe(async {
        let owner = create_test_workspace(&state, "memory-id-owner").await?;
        let intruder = create_test_workspace(&state, "memory-id-intruder").await?;

        let pack = seeded_pack(&owner);
        fems_memory::upsert_memory_pack(&state.surreal, &owner, "", &pack).await?;
        let pack_error = fems_memory::upsert_memory_pack(&state.surreal, &intruder, "", &pack)
            .await
            .expect_err("a pack id cannot move to another workspace");
        assert!(matches!(pack_error, StorageError::Conflict(_)));

        let memory_id = Uuid::now_v7().to_string();
        let item = serde_json::json!({
            "memory_id": memory_id,
            "memory_class": "semantic",
            "type": "fact",
            "summary": "workspace-bound item",
            "content": "workspace-bound item",
            "source_refs": [],
            "status": "active"
        });
        fems_memory::upsert_memory_item(&state.surreal, &owner, &memory_id, &item).await?;
        let item_error =
            fems_memory::upsert_memory_item(&state.surreal, &intruder, &memory_id, &item)
                .await
                .expect_err("a memory item id cannot move to another workspace");
        assert!(matches!(item_error, StorageError::Conflict(_)));
        assert_eq!(
            fems_memory::get_memory_item(&state.surreal, &owner, &memory_id).await?,
            Some(item)
        );
        assert_eq!(
            fems_memory::get_memory_item(&state.surreal, &intruder, &memory_id).await?,
            None
        );
        Ok::<(), Box<dyn std::error::Error>>(())
        })).await;
        drop(state);
        finish_mt109_memory_test(body, store, data_dir).await
    }

    /// AC-109-3: a valid proposal is stored as pending_review + leaves a durable
    /// ARTIFACT_PROPOSED EventLedger receipt.
    #[tokio::test]
    async fn create_proposal_stores_pending_review_and_receipt(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (store, state) = setup_state().await?;
        let data_dir = store.data_dir.clone();
        let body = futures::FutureExt::catch_unwind(std::panic::AssertUnwindSafe(async {
        let workspace_id = create_test_workspace(&state, "proposal").await?;
        let content = "durable fact";
        let content_hash = canonical_content_hash(content).expect("canonical content hash");
        let source =
            create_test_rich_source(&state, &workspace_id, "proposal-source", content).await?;
        let source_document_id = source.document_id.clone();
        let proposal_scope = proposal_create_test_scope(
            &state,
            &workspace_id,
            &source_document_id,
            "proposal-pending",
            "native_editor",
        )
        .await?;
        let expected_selection_start = i64::try_from(source.selection_start)?;
        let expected_selection_end = i64::try_from(source.selection_end)?;
        let request = ProposalRequest {
            request_id: None,
            class: ProposalClass::Semantic,
            content: content.to_string(),
            source,
            source_document_content: None,
            review_gated: Some(true),
            actor_id: Some("actor-7".to_string()),
        };

        let create = create_memory_proposal(
                State(state.clone()),
                Path(workspace_id.clone()),
                HeaderMap::new(),
                Json(request),
            );
        let Json(ack) = state
            .surreal
            .with_record_user_scope(proposal_scope, create)
            .await
        .map_err(|(code, body)| format!("proposal failed: {code} {body:?}"))?;

        assert_eq!(ack.status, PROPOSAL_STATUS_PENDING_REVIEW);
        assert!(
            Uuid::parse_str(&ack.proposal_id).is_ok_and(|proposal_id| !proposal_id.is_nil()),
            "proposal id is a non-nil UUID"
        );

        // The proposal is durably stored as pending_review with its provenance.
        let stored = fems_memory::get_memory_proposal(&state.surreal, &ack.proposal_id)
            .await?
            .expect("proposal stored");
        assert_eq!(stored.status, PROPOSAL_STATUS_PENDING_REVIEW);
        assert_eq!(stored.document_id, source_document_id);
        assert_eq!(stored.selection_start, expected_selection_start);
        assert_eq!(stored.selection_end, expected_selection_end);
        assert_eq!(stored.content_hash, content_hash);
        assert_eq!(stored.memory_class, "semantic");
        assert!(stored.review_gated);

        let Json(readback) = get_memory_proposal(
            State(state.clone()),
            Path((workspace_id.clone(), ack.proposal_id.clone())),
        )
        .await
        .map_err(|(code, body)| format!("proposal readback failed: {code} {body:?}"))?;
        assert_eq!(readback, stored);
        let wrong_workspace = get_memory_proposal(
            State(state.clone()),
            Path(("WS-OTHER".to_owned(), ack.proposal_id.clone())),
        )
        .await
        .expect_err("cross-workspace proposal readback must fail closed");
        assert_eq!(wrong_workspace.0, StatusCode::NOT_FOUND);

        // A durable ARTIFACT_PROPOSED EventLedger receipt was appended.
        let receipt =
            memory_test_ledger_event(&state, &format!("fems-memory-proposal:{}", ack.proposal_id))
                .await?
                .expect("exactly one durable proposal receipt");
        assert_eq!(receipt.event_type, KernelEventType::ArtifactProposed);
        assert_eq!(receipt.payload["document_id"], source_document_id);
        assert_eq!(receipt.payload["selection_start"], expected_selection_start);
        assert_eq!(receipt.payload["selection_end"], expected_selection_end);
        assert_eq!(receipt.payload["content_hash"], content_hash);

        let events = state
            .flight_recorder
            .list_events(EventFilter {
                event_id: Some(ack.flight_recorder_event_id),
                ..EventFilter::default()
            })
            .await?;
        assert_eq!(events.len(), 1, "proposal ack names one canonical FR event");
        let event = &events[0];
        assert_eq!(
            event.event_type,
            FlightRecorderEventType::MemoryWriteProposed
        );
        assert_eq!(event.payload["event_code"], "FR-EVT-MEM-001");
        assert_eq!(event.payload["proposal_id"], ack.proposal_id);
        assert_eq!(event.payload["op_count"], 1);
        assert_eq!(event.payload["requires_review_count"], 1);
        assert!(event.payload.get("content").is_none());
        let Json(artifact) = get_memory_proposal_artifact(
            State(state.clone()),
            Path((workspace_id.clone(), ack.proposal_id.clone())),
        )
        .await
        .map_err(|(code, body)| format!("proposal artifact failed: {code} {body:?}"))?;
        assert_eq!(artifact["proposal_id"], ack.proposal_id);
        assert_eq!(artifact["schema_version"], "hsk.memory_write_proposal@0.1");
        assert_eq!(
            artifact["created_at"],
            stored.created_at.to_rfc3339(),
            "canonical artifact and durable proposal row share one creation instant"
        );
        assert_eq!(artifact["policy"]["require_human_review"], true);
        assert_eq!(artifact["ops"].as_array().map(Vec::len), Some(1));
        assert_eq!(artifact["ops"][0]["requires_review"], true);
        assert!(artifact.get("content").is_none());
        let canonical_artifact: MemoryWriteProposal =
            serde_json::from_value(artifact.clone()).expect("canonical proposal artifact decodes");
        assert_eq!(
            event.payload["proposal_hash"],
            canonical_artifact
                .compute_hash()
                .expect("canonical proposal artifact hashes")
        );
        assert_eq!(
            event.payload["artifact_ref"],
            format!(
                "artifact://sha256/{}",
                event.payload["proposal_hash"].as_str().unwrap()
            )
        );
        let published = memory_test_query_first::<MemoryTestBoolRow>(
            &state,
            "SELECT published_at != NONE AS value FROM fems_memory_lifecycle_fr_outbox \
             WHERE proposal_id = $proposal AND event_code = 'FR-EVT-MEM-001' LIMIT 1;",
            MemoryTestBindings {
                proposal: Some(RecordId::new(
                    "fems_memory_proposals",
                    ack.proposal_id.as_str(),
                )),
                ..MemoryTestBindings::default()
            },
        )
        .await?
        .expect("proposal outbox row")
        .value;
        assert!(
            published,
            "API acknowledgement follows durable FR projection"
        );
        Ok::<(), Box<dyn std::error::Error>>(())
        }))
        .await;
        eprintln!(
            "MT109_CREATE_PROPOSAL_LEASES_AFTER_BODY={}",
            state.surreal.leases_in_flight()
        );
        drop(state);
        finish_mt109_memory_test(body, store, data_dir).await
    }

    #[tokio::test]
    async fn list_proposals_is_bounded_deterministic_and_workspace_scoped(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (store, state) = setup_state().await?;
        let data_dir = store.data_dir.clone();
        let body = futures::FutureExt::catch_unwind(std::panic::AssertUnwindSafe(async {
        let target = create_test_workspace(&state, "proposal-list-target").await?;
        let control = create_test_workspace(&state, "proposal-list-control").await?;

        let submit = |workspace_id: String, request_id: String| {
            let state = state.clone();
            async move {
                let content = format!("content-{request_id}");
                let source = create_test_rich_source(
                    &state,
                    &workspace_id,
                    "proposal-list-source",
                    &content,
                )
                .await
                .map_err(storage_error)?;
                let scope = proposal_create_test_scope(
                    &state,
                    &workspace_id,
                    &source.document_id,
                    "proposal-list",
                    "native_editor",
                )
                .await
                .map_err(|error| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"error": error.to_string()})),
                    )
                })?;
                let request_state = state.clone();
                state
                    .surreal
                    .with_record_user_scope(
                        scope,
                        create_memory_proposal(
                            State(request_state),
                            Path(workspace_id.clone()),
                            HeaderMap::new(),
                            Json(ProposalRequest {
                                request_id: Some(request_id),
                                class: ProposalClass::Semantic,
                                content: content.clone(),
                                source,
                                source_document_content: None,
                                review_gated: Some(true),
                                actor_id: Some("proposal-list-test".to_owned()),
                            }),
                        ),
                    )
                    .await
            }
        };
        let first = submit(target.clone(), "list-first".to_owned())
            .await
            .map_err(|(code, body)| format!("first proposal failed: {code} {body:?}"))?
            .0;
        let second = submit(target.clone(), "list-second".to_owned())
            .await
            .map_err(|(code, body)| format!("second proposal failed: {code} {body:?}"))?
            .0;
        let control_ack = submit(control.clone(), "list-control".to_owned())
            .await
            .map_err(|(code, body)| format!("control proposal failed: {code} {body:?}"))?
            .0;

        for (proposal_id, created_at) in [
            (
                &first.proposal_id,
                chrono::Utc::now() - chrono::Duration::minutes(1),
            ),
            (&second.proposal_id, chrono::Utc::now()),
        ] {
            assert_eq!(
                memory_test_execute(
                    &state,
                    "UPDATE fems_memory_proposals SET created_at = $at \
                     WHERE id = $proposal RETURN AFTER;",
                    MemoryTestBindings {
                        proposal: Some(RecordId::new(
                            "fems_memory_proposals",
                            proposal_id.as_str(),
                        )),
                        at: Some(Datetime::from(created_at)),
                        ..MemoryTestBindings::default()
                    },
                )
                .await?,
                1
            );
        }

        let Json(all) = list_memory_proposals(
            State(state.clone()),
            Path(target.clone()),
            Query(ProposalListQuery { limit: Some(200) }),
        )
        .await
        .map_err(|(code, body)| format!("list proposals failed: {code} {body:?}"))?;
        assert_eq!(
            all.iter()
                .map(|proposal| proposal.proposal_id.as_str())
                .collect::<Vec<_>>(),
            vec![second.proposal_id.as_str(), first.proposal_id.as_str()]
        );
        assert!(all.iter().all(|proposal| proposal.workspace_id == target));
        assert!(!all
            .iter()
            .any(|proposal| proposal.proposal_id == control_ack.proposal_id));

        assert_eq!(
            memory_test_execute(
                &state,
                "UPDATE fems_memory_proposals SET status = 'approved' \
                 WHERE id = $proposal RETURN AFTER;",
                MemoryTestBindings {
                    proposal: Some(RecordId::new(
                        "fems_memory_proposals",
                        first.proposal_id.as_str()
                    )),
                    ..MemoryTestBindings::default()
                },
            )
            .await?,
            1
        );
        let Json(actionable_after_review) = list_memory_proposals(
            State(state.clone()),
            Path(target.clone()),
            Query(ProposalListQuery { limit: Some(200) }),
        )
        .await
        .map_err(|(code, body)| format!("post-review pending list failed: {code} {body:?}"))?;
        assert_eq!(
            actionable_after_review
                .iter()
                .map(|proposal| proposal.proposal_id.as_str())
                .collect::<Vec<_>>(),
            vec![first.proposal_id.as_str(), second.proposal_id.as_str()],
            "approved proposals must lead the actionable projection for crash-safe commit recovery"
        );

        let Json(limited) = list_memory_proposals(
            State(state.clone()),
            Path(target),
            Query(ProposalListQuery { limit: Some(1) }),
        )
        .await
        .map_err(|(code, body)| format!("limited list failed: {code} {body:?}"))?;
        assert_eq!(limited, vec![actionable_after_review[0].clone()]);

        let missing = list_memory_proposals(
            State(state.clone()),
            Path("WS-MISSING-PROPOSAL-LIST".to_owned()),
            Query(ProposalListQuery::default()),
        )
        .await
        .expect_err("missing workspace must not masquerade as an empty proposal list");
        assert_eq!(missing.0, StatusCode::NOT_FOUND);
        Ok::<(), Box<dyn std::error::Error>>(())
        })).await;
        drop(state);
        finish_mt109_memory_test(body, store, data_dir).await
    }

    /// A client retry is the same logical operation: the stable request identity maps to
    /// one proposal row and one canonical EventLedger receipt. Reusing that identity for
    /// a different authoritative payload fails closed instead of silently aliasing two proposals.
    /// Caller-supplied actor metadata is not authoritative and therefore cannot create drift.
    #[tokio::test]
    async fn proposal_retry_is_idempotent_and_payload_drift_conflicts(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (store, state) = setup_state().await?;
        let data_dir = store.data_dir.clone();
        let body = futures::FutureExt::catch_unwind(std::panic::AssertUnwindSafe(async {
        let workspace_id = create_test_workspace(&state, "retry").await?;
        let request_id = format!("native-retry-{}", Uuid::now_v7());
        let content = "same logical proposal";
        let source =
            create_test_rich_source(&state, &workspace_id, "retry-source", content).await?;
        let proposal_scope = proposal_create_test_scope(
            &state,
            &workspace_id,
            &source.document_id,
            "retry",
            "native_editor",
        )
        .await?;
        let request = ProposalRequest {
            request_id: Some(request_id.clone()),
            class: ProposalClass::Semantic,
            content: content.to_string(),
            source,
            source_document_content: None,
            review_gated: Some(true),
            actor_id: Some("actor-retry".to_string()),
        };

        let first_call = create_memory_proposal(
            State(state.clone()),
            Path(workspace_id.clone()),
            HeaderMap::new(),
            Json(request.clone()),
        );
        let Json(first) = state
            .surreal
            .with_record_user_scope(proposal_scope.clone(), first_call)
            .await
        .map_err(|(code, body)| format!("first proposal failed: {code} {body:?}"))?;
        let replay_call = create_memory_proposal(
            State(state.clone()),
            Path(workspace_id.clone()),
            HeaderMap::new(),
            Json(request.clone()),
        );
        let Json(replay) = state
            .surreal
            .with_record_user_scope(proposal_scope.clone(), replay_call)
            .await
        .map_err(|(code, body)| format!("replay failed: {code} {body:?}"))?;
        assert_eq!(replay.proposal_id, first.proposal_id);
        assert_eq!(replay.status, first.status);

        let proposal_count = fems_memory::list_memory_proposals(&state.surreal, &workspace_id, 200)
            .await?
            .into_iter()
            .filter(|proposal| proposal.request_id == request_id)
            .count();
        assert_eq!(proposal_count, 1, "retry must not duplicate proposal rows");
        let receipt_count = usize::from(
            memory_test_ledger_event(
                &state,
                &format!("fems-memory-proposal:{}", first.proposal_id),
            )
            .await?
            .is_some(),
        );
        assert_eq!(
            receipt_count, 1,
            "retry must not duplicate canonical receipts"
        );

        let mut drifted = request.clone();
        drifted.actor_id = Some("different-actor".to_string());
        let spoof_call = create_memory_proposal(
            State(state.clone()),
            Path(workspace_id.clone()),
            HeaderMap::new(),
            Json(drifted),
        );
        let Json(spoof_replay) = state
            .surreal
            .with_record_user_scope(proposal_scope.clone(), spoof_call)
            .await
        .map_err(|(code, body)| format!("spoof metadata replay failed: {code} {body:?}"))?;
        assert_eq!(spoof_replay.proposal_id, first.proposal_id);

        let mut class_drift = request.clone();
        class_drift.class = ProposalClass::Episodic;
        let conflict_call = create_memory_proposal(
            State(state.clone()),
            Path(workspace_id),
            HeaderMap::new(),
            Json(class_drift),
        );
        let conflict = state
            .surreal
            .with_record_user_scope(proposal_scope, conflict_call)
            .await
        .expect_err("request identity authoritative payload drift must fail closed");
        assert_eq!(conflict.0, StatusCode::CONFLICT);
        Ok::<(), Box<dyn std::error::Error>>(())
        }))
        .await;
        drop(state);
        finish_mt109_memory_test(body, store, data_dir).await
    }

    #[tokio::test]
    async fn rich_proposal_replay_rejects_code_only_snapshot_drift(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (store, state) = setup_state().await?;
        let data_dir = store.data_dir.clone();
        let body = futures::FutureExt::catch_unwind(std::panic::AssertUnwindSafe(async {
        let workspace_id = create_test_workspace(&state, "rich-snapshot-boundary").await?;
        let content = "rich replay selection";
        let document_text = format!("xxx{content}");
        let db = SurrealDatabase::new(state.surreal.clone());
        let document = db
            .create_knowledge_rich_document(NewKnowledgeRichDocument {
                workspace_id: workspace_id.clone(),
                document_id: None,
                title: "Rich snapshot boundary".to_owned(),
                schema_version: "hsk_richdoc_v1".to_owned(),
                content_json: json!({
                    "type": "doc",
                    "content": [{
                        "type": "paragraph",
                        "content": [{"type": "text", "text": document_text}]
                    }]
                }),
                crdt_document_id: None,
                crdt_snapshot_id: None,
                promotion_receipt_event_id: None,
                ..Default::default()
            })
            .await?;
        let proposal_scope = proposal_create_test_scope(
            &state,
            &workspace_id,
            &document.rich_document_id,
            "rich-snapshot-boundary",
            "native_editor",
        )
        .await?;
        let request_id = format!("rich-snapshot-boundary-{}", Uuid::now_v7());
        let request = ProposalRequest {
            request_id: Some(request_id.clone()),
            class: ProposalClass::Semantic,
            content: content.to_owned(),
            source: valid_source(&workspace_id, &document.rich_document_id, content),
            source_document_content: None,
            review_gated: Some(true),
            actor_id: Some("rich-boundary-actor".to_owned()),
        };

        let first_call = create_memory_proposal(
            State(state.clone()),
            Path(workspace_id.clone()),
            HeaderMap::new(),
            Json(request.clone()),
        );
        let Json(first) = state
            .surreal
            .with_record_user_scope(proposal_scope.clone(), first_call)
            .await
        .map_err(|(code, body)| format!("valid rich proposal failed: {code} {body:?}"))?;

        let mut snapshot_drift = request.clone();
        snapshot_drift.source_document_content = Some("untrusted code snapshot".to_owned());
        let snapshot_call = create_memory_proposal(
            State(state.clone()),
            Path(workspace_id.clone()),
            HeaderMap::new(),
            Json(snapshot_drift),
        );
        let snapshot_error = state
            .surreal
            .with_record_user_scope(proposal_scope.clone(), snapshot_call)
            .await
        .expect_err("rich replay must reject code-only source_document_content");
        assert_eq!(snapshot_error.0, StatusCode::BAD_REQUEST);

        let mut hash_drift = request;
        hash_drift.source.document_content_hash = Some("a".repeat(64));
        let hash_call = create_memory_proposal(
            State(state.clone()),
            Path(workspace_id.clone()),
            HeaderMap::new(),
            Json(hash_drift),
        );
        let hash_error = state
            .surreal
            .with_record_user_scope(proposal_scope, hash_call)
            .await
        .expect_err("rich replay must reject code-only document_content_hash");
        assert_eq!(hash_error.0, StatusCode::BAD_REQUEST);

        let proposal_count = fems_memory::list_memory_proposals(&state.surreal, &workspace_id, 200)
            .await?
            .into_iter()
            .filter(|proposal| proposal.request_id == request_id)
            .count();
        assert_eq!(proposal_count, 1, "rejected rich drift must add no row");
        let receipt_count = usize::from(
            memory_test_ledger_event(
                &state,
                &format!("fems-memory-proposal:{}", first.proposal_id),
            )
            .await?
            .is_some(),
        );
        assert_eq!(receipt_count, 1, "rejected rich drift must add no receipt");
            Ok::<(), Box<dyn std::error::Error>>(())
        }))
        .await;
        drop(state);
        finish_mt109_memory_test(body, store, data_dir).await
    }

    #[tokio::test]
    async fn proposal_retry_rejects_corrupt_receipt_but_allows_new_retry_headers(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (store, state) = setup_state().await?;
        let data_dir = store.data_dir.clone();
        let body = futures::FutureExt::catch_unwind(std::panic::AssertUnwindSafe(async {
        let workspace_id = create_test_workspace(&state, "receipt-authenticity").await?;
        let content = "receipt authenticity proposal";
        let source = create_test_rich_source(
            &state,
            &workspace_id,
            "receipt-authenticity-source",
            content,
        )
        .await?;
        let proposal_scope = proposal_create_test_scope(
            &state,
            &workspace_id,
            &source.document_id,
            "receipt-authenticity",
            "original-operator",
        )
        .await?;
        let request = ProposalRequest {
            request_id: Some(format!("receipt-authenticity-{}", Uuid::now_v7())),
            class: ProposalClass::Semantic,
            content: content.to_owned(),
            source,
            source_document_content: None,
            review_gated: Some(true),
            actor_id: Some("body-actor".to_owned()),
        };
        let mut original_headers = HeaderMap::new();
        original_headers.insert(HSK_HEADER_ACTOR_KIND, "operator".parse()?);
        original_headers.insert(HSK_HEADER_ACTOR_ID, "original-operator".parse()?);
        original_headers.insert(HSK_HEADER_KERNEL_TASK_RUN_ID, "original-task".parse()?);
        original_headers.insert(HSK_HEADER_SESSION_RUN_ID, "original-session".parse()?);
        let first_call = create_memory_proposal(
            State(state.clone()),
            Path(workspace_id.clone()),
            original_headers,
            Json(request.clone()),
        );
        let Json(first) = state
            .surreal
            .with_record_user_scope(proposal_scope.clone(), first_call)
            .await
        .map_err(|(code, body)| format!("first proposal failed: {code} {body:?}"))?;
        let receipt_key = format!("fems-memory-proposal:{}", first.proposal_id);
        let canonical_hash = memory_test_ledger_event(&state, &receipt_key)
            .await?
            .expect("proposal receipt")
            .payload_hash;
        let mutate_receipt_hash = |hash: String| {
            memory_test_execute(
                &state,
                "UPDATE kernel_event_ledger SET payload_hash = $hash \
                 WHERE idempotency_key = $value RETURN AFTER;",
                MemoryTestBindings {
                    value: Some(receipt_key.clone()),
                    hash: Some(hash),
                    ..MemoryTestBindings::default()
                },
            )
        };
        assert_eq!(mutate_receipt_hash("0".repeat(64)).await?, 1);

        let mut retry_headers = HeaderMap::new();
        retry_headers.insert(HSK_HEADER_ACTOR_KIND, "system".parse()?);
        retry_headers.insert(HSK_HEADER_ACTOR_ID, "retry-system".parse()?);
        retry_headers.insert(HSK_HEADER_KERNEL_TASK_RUN_ID, "retry-task".parse()?);
        retry_headers.insert(HSK_HEADER_SESSION_RUN_ID, "retry-session".parse()?);
        let corrupt_call = create_memory_proposal(
            State(state.clone()),
            Path(workspace_id.clone()),
            retry_headers.clone(),
            Json(request.clone()),
        );
        let corrupt_status = state
            .surreal
            .with_record_user_scope(proposal_scope.clone(), corrupt_call)
            .await
        .err()
        .map(|error| error.0);
        assert_eq!(mutate_receipt_hash(canonical_hash).await?, 1);
        assert_eq!(
            corrupt_status,
            Some(StatusCode::CONFLICT),
            "corrupt existing receipt did not fail closed as a conflict"
        );

        let retry_call = create_memory_proposal(
            State(state.clone()),
            Path(workspace_id),
            retry_headers,
            Json(request),
        );
        let Json(retry) = state
            .surreal
            .with_record_user_scope(proposal_scope, retry_call)
            .await
        .map_err(|(code, body)| format!("header-independent retry failed: {code} {body:?}"))?;
        assert_eq!(retry.proposal_id, first.proposal_id);
            Ok::<(), Box<dyn std::error::Error>>(())
        }))
        .await;
        drop(state);
        finish_mt109_memory_test(body, store, data_dir).await
    }

    #[tokio::test]
    async fn simultaneous_proposal_retries_converge_to_one_row_and_receipt(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (store, state) = setup_state().await?;
        let data_dir = store.data_dir.clone();
        let body = futures::FutureExt::catch_unwind(std::panic::AssertUnwindSafe(async {
        let workspace_id = create_test_workspace(&state, "simultaneous-retry").await?;
        let content = "simultaneous logical proposal";
        let source =
            create_test_rich_source(&state, &workspace_id, "simultaneous-retry-source", content)
                .await?;
        let proposal_scope = proposal_create_test_scope(
            &state,
            &workspace_id,
            &source.document_id,
            "simultaneous-retry",
            "native_editor",
        )
        .await?;
        let request = ProposalRequest {
            request_id: Some(format!("simultaneous-retry-{}", Uuid::now_v7())),
            class: ProposalClass::Semantic,
            content: content.to_owned(),
            source,
            source_document_content: None,
            review_gated: Some(true),
            actor_id: Some("actor-simultaneous".to_owned()),
        };

        let first_call = create_memory_proposal(
            State(state.clone()),
            Path(workspace_id.clone()),
            HeaderMap::new(),
            Json(request.clone()),
        );
        let second_call = create_memory_proposal(
            State(state.clone()),
            Path(workspace_id.clone()),
            HeaderMap::new(),
            Json(request.clone()),
        );
        let first = state
            .surreal
            .with_record_user_scope(proposal_scope.clone(), first_call);
        let second = state
            .surreal
            .with_record_user_scope(proposal_scope, second_call);
        let (first, second) = tokio::join!(first, second);
        let first = first
            .map_err(|(code, body)| format!("first concurrent retry failed: {code} {body:?}"))?
            .0;
        let second = second
            .map_err(|(code, body)| format!("second concurrent retry failed: {code} {body:?}"))?
            .0;
        assert_eq!(first.proposal_id, second.proposal_id);

        let proposal_count = fems_memory::list_memory_proposals(&state.surreal, &workspace_id, 200)
            .await?
            .into_iter()
            .filter(|proposal| proposal.request_id == request.request_id.as_deref().unwrap())
            .count();
        assert_eq!(proposal_count, 1);
        let receipt_count = usize::from(
            memory_test_ledger_event(
                &state,
                &format!("fems-memory-proposal:{}", first.proposal_id),
            )
            .await?
            .is_some(),
        );
        assert_eq!(receipt_count, 1);
            Ok::<(), Box<dyn std::error::Error>>(())
        }))
        .await;
        drop(state);
        finish_mt109_memory_test(body, store, data_dir).await
    }

    #[tokio::test]
    async fn legacy_retry_heals_missing_receipt_once_and_converges(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (store, state) = setup_state().await?;
        let data_dir = store.data_dir.clone();
        let body = futures::FutureExt::catch_unwind(std::panic::AssertUnwindSafe(async {
        let workspace_id = create_test_workspace(&state, "legacy-retry").await?;
        let content = "legacy proposal retry";
        let source =
            create_test_rich_source(&state, &workspace_id, "legacy-retry-source", content).await?;
        let request = ProposalRequest {
            request_id: None,
            class: ProposalClass::Episodic,
            content: content.to_owned(),
            source,
            source_document_content: None,
            review_gated: Some(true),
            actor_id: Some("legacy-actor".to_owned()),
        };
        let derived_request_id =
            stable_proposal_request_id(&workspace_id, &request).expect("derived request identity");

        let legacy_proposal_id = format!("PROP-LEGACY-{}", Uuid::now_v7());
        let proposal_payload = json!({
            "proposal_id": legacy_proposal_id,
            "request_id": derived_request_id,
            "workspace_id": workspace_id,
            "class": request.class.wire(),
            "content": request.content,
            "source": request.source,
            "review_gated": true,
            "status": PROPOSAL_STATUS_PENDING_REVIEW,
            "actor_id": request.actor_id,
        });
        let legacy = fems_memory::StoredMemoryProposal {
            proposal_id: legacy_proposal_id.clone(),
            request_id: derived_request_id.clone(),
            workspace_id: workspace_id.clone(),
            document_id: request.source.document_id.clone(),
            selection_start: i64::try_from(request.source.selection_start)
                .expect("legacy fixture selection_start fits i64"),
            selection_end: i64::try_from(request.source.selection_end)
                .expect("legacy fixture selection_end fits i64"),
            content_hash: canonical_content_hash(content).expect("canonical hash"),
            memory_class: "episodic".to_owned(),
            status: PROPOSAL_STATUS_PENDING_REVIEW.to_owned(),
            review_gated: true,
            created_at: chrono::Utc::now(),
            proposal: proposal_payload,
        };
        #[derive(SurrealValue)]
        struct LegacyProposalContent {
            proposal_id: String,
            request_id: String,
            workspace_id: RecordId,
            document_id: String,
            selection_start: i64,
            selection_end: i64,
            content_hash: String,
            memory_class: String,
            status: String,
            review_gated: bool,
            proposal: Value,
            created_at: Datetime,
        }
        let legacy_id = legacy.proposal_id.clone();
        let legacy_content = LegacyProposalContent {
            proposal_id: legacy.proposal_id.clone(),
            request_id: legacy.request_id.clone(),
            workspace_id: RecordId::new("workspaces", legacy.workspace_id.as_str()),
            document_id: legacy.document_id.clone(),
            selection_start: legacy.selection_start,
            selection_end: legacy.selection_end,
            content_hash: legacy.content_hash.clone(),
            memory_class: legacy.memory_class.clone(),
            status: legacy.status.clone(),
            review_gated: legacy.review_gated,
            proposal: legacy.proposal.clone(),
            created_at: Datetime::from(legacy.created_at),
        };
        let inserted: Option<Value> = state
            .surreal
            .with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .create_if_absent("fems_memory_proposals", &legacy_id, legacy_content)
                        .await
                })
            })
            .await?;
        assert!(
            inserted.is_some(),
            "legacy proposal fixture must be inserted"
        );
        let receipt_key = format!("fems-memory-proposal:{legacy_proposal_id}");
        let before = usize::from(
            memory_test_ledger_event(&state, &receipt_key)
                .await?
                .is_some(),
        );
        assert_eq!(before, 0, "fixture must model the pre-upgrade crash window");

        let mut first_headers = HeaderMap::new();
        first_headers.insert(HSK_HEADER_ACTOR_KIND, "operator".parse()?);
        first_headers.insert(HSK_HEADER_ACTOR_ID, "legacy-actor".parse()?);
        first_headers.insert(HSK_HEADER_KERNEL_TASK_RUN_ID, "retry-task-a".parse()?);
        first_headers.insert(HSK_HEADER_SESSION_RUN_ID, "retry-session-a".parse()?);
        let mut second_headers = HeaderMap::new();
        second_headers.insert(HSK_HEADER_ACTOR_KIND, "operator".parse()?);
        second_headers.insert(HSK_HEADER_ACTOR_ID, "legacy-actor".parse()?);
        second_headers.insert(HSK_HEADER_KERNEL_TASK_RUN_ID, "retry-task-b".parse()?);
        second_headers.insert(HSK_HEADER_SESSION_RUN_ID, "retry-session-b".parse()?);
        let first = create_memory_proposal(
            State(state.clone()),
            Path(workspace_id.clone()),
            first_headers,
            Json(request.clone()),
        );
        let second = create_memory_proposal(
            State(state.clone()),
            Path(workspace_id.clone()),
            second_headers,
            Json(request),
        );
        let (first, second) = tokio::join!(first, second);
        let first = first
            .map_err(|(code, body)| format!("first legacy retry failed: {code} {body:?}"))?
            .0;
        let second = second
            .map_err(|(code, body)| format!("second legacy retry failed: {code} {body:?}"))?
            .0;
        assert_eq!(first.proposal_id, legacy_proposal_id);
        assert_eq!(second.proposal_id, legacy_proposal_id);
        let count = fems_memory::list_memory_proposals(&state.surreal, &workspace_id, 200)
            .await?
            .into_iter()
            .filter(|proposal| proposal.request_id == derived_request_id)
            .count();
        assert_eq!(count, 1);
        let receipt = memory_test_ledger_event(&state, &receipt_key)
            .await?
            .expect("concurrent healing appends exactly once");
        assert_eq!(receipt.aggregate_id, legacy_proposal_id);
        assert_eq!(
            receipt.payload["proposal_id"],
            Value::String(legacy_proposal_id)
        );
        assert_eq!(
            receipt.kernel_task_run_id,
            format!("native-editor-fems-propose-{workspace_id}")
        );
        assert_eq!(receipt.session_run_id, "native-editor-session");
        assert_eq!(receipt.actor.actor_kind(), "operator");
        assert_eq!(receipt.actor.actor_id(), "legacy-actor");
        Ok::<(), Box<dyn std::error::Error>>(())
        })).await;
        drop(state);
        finish_mt109_memory_test(body, store, data_dir).await
    }

    /// MT-118 AC-118-2 TRIPWIRE. The legacy non-UUID `proposal_id` must be admitted ONLY on
    /// the heal path; if it is admitted anywhere else the canonical proposal contract has
    /// been weakened for every proposal in the system.
    ///
    /// This is the differential CONTROL for
    /// `legacy_retry_heals_missing_receipt_once_and_converges`. Both tests seed the same kind
    /// of pre-existing row, with the same non-UUID `PROP-LEGACY-{uuid}` id, and reach the same
    /// canonical artifact BYTES - here obtained from the production heal itself rather than
    /// hand-copied, because MT-112 recorded that a test which hand-copies production logic
    /// cannot detect production drift. The ONLY difference is that here those bytes are
    /// PERSISTED in `_canonical_artifact`, which puts them on the durable path. Same bytes,
    /// different origin, and the durable one must still be rejected.
    ///
    /// The second half proves the heal cannot be MINTED: a FIRST-TIME insert whose payload
    /// carries no `_canonical_artifact` still fails closed with the exact serialization error
    /// MT-112 could previously only obtain by temporarily instrumenting `storage_error`. That
    /// error text is pinned here permanently.
    #[tokio::test]
    async fn non_uuid_proposal_id_is_admitted_only_on_the_legacy_heal_path(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (store, state) = setup_state().await?;
        let data_dir = store.data_dir.clone();
        let body = futures::FutureExt::catch_unwind(std::panic::AssertUnwindSafe(async {
        let workspace_id = create_test_workspace(&state, "heal-tripwire").await?;
        let content = "tripwire proposal retry";
        let source =
            create_test_rich_source(&state, &workspace_id, "heal-tripwire-source", content).await?;
        let request = ProposalRequest {
            request_id: None,
            class: ProposalClass::Episodic,
            content: content.to_owned(),
            source,
            source_document_content: None,
            review_gated: Some(true),
            actor_id: Some("tripwire-actor".to_owned()),
        };
        let derived_request_id =
            stable_proposal_request_id(&workspace_id, &request).expect("derived request identity");

        let legacy_proposal_id = format!("PROP-LEGACY-{}", Uuid::now_v7());
        // Same microsecond normalization the intake route uses, so the row's durable
        // `created_at` and the artifact rebuilt from it are one byte-identical instant.
        let created_at =
            chrono::DateTime::from_timestamp_micros(chrono::Utc::now().timestamp_micros())
                .expect("normalized fixture instant");
        let base_payload = json!({
            "proposal_id": legacy_proposal_id,
            "request_id": derived_request_id,
            "workspace_id": workspace_id,
            "class": request.class.wire(),
            "content": request.content,
            "source": request.source,
            "review_gated": true,
            "status": PROPOSAL_STATUS_PENDING_REVIEW,
            "actor_id": request.actor_id,
        });
        let legacy = fems_memory::StoredMemoryProposal {
            proposal_id: legacy_proposal_id.clone(),
            request_id: derived_request_id.clone(),
            workspace_id: workspace_id.clone(),
            document_id: request.source.document_id.clone(),
            selection_start: i64::try_from(request.source.selection_start)
                .expect("tripwire fixture selection_start fits i64"),
            selection_end: i64::try_from(request.source.selection_end)
                .expect("tripwire fixture selection_end fits i64"),
            content_hash: canonical_content_hash(content).expect("canonical hash"),
            memory_class: "episodic".to_owned(),
            status: PROPOSAL_STATUS_PENDING_REVIEW.to_owned(),
            review_gated: true,
            created_at,
            proposal: base_payload.clone(),
        };

        // The artifact the heal would rebuild for this exact row - taken from production, not
        // reconstructed by the test.
        let healed = fems_memory::proposal_canonical_artifact(
            &legacy,
            fems_memory::LegacyArtifactHeal::Allow,
        );
        assert_eq!(
            healed.origin,
            fems_memory::ProposalArtifactOrigin::HealedFromDurableColumns,
            "the control must start from a row the heal path really does accept"
        );
        assert_eq!(
            healed.value["proposal_id"],
            Value::String(legacy_proposal_id.clone()),
            "the rebuilt artifact carries the row's own non-UUID id"
        );
        assert!(
            Uuid::parse_str(&legacy_proposal_id).is_err(),
            "the fixture id must genuinely be non-UUID or this proves nothing"
        );

        // CASE 1: identical row, identical artifact BYTES, but PERSISTED -> durable path.
        let mut persisted_payload = base_payload.clone();
        persisted_payload
            .as_object_mut()
            .expect("fixture payload is an object")
            .insert("_canonical_artifact".to_owned(), healed.value.clone());
        #[derive(SurrealValue)]
        struct PersistedProposalContent {
            proposal_id: String,
            request_id: String,
            workspace_id: RecordId,
            document_id: String,
            selection_start: i64,
            selection_end: i64,
            content_hash: String,
            memory_class: String,
            status: String,
            review_gated: bool,
            created_at: Datetime,
            proposal: Value,
        }
        let persisted_id = legacy.proposal_id.clone();
        let inserted: Option<Value> = state
            .surreal
            .with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .create_if_absent(
                            "fems_memory_proposals",
                            &persisted_id,
                            PersistedProposalContent {
                                proposal_id: legacy.proposal_id,
                                request_id: legacy.request_id,
                                workspace_id: RecordId::new("workspaces", legacy.workspace_id),
                                document_id: legacy.document_id,
                                selection_start: legacy.selection_start,
                                selection_end: legacy.selection_end,
                                content_hash: legacy.content_hash,
                                memory_class: legacy.memory_class,
                                status: legacy.status,
                                review_gated: legacy.review_gated,
                                created_at: Datetime::from(legacy.created_at),
                                proposal: persisted_payload,
                            },
                        )
                        .await
                })
            })
            .await?;
        assert!(
            inserted.is_some(),
            "persisted tripwire fixture must be inserted"
        );

        let mut headers = HeaderMap::new();
        headers.insert(HSK_HEADER_ACTOR_KIND, "operator".parse()?);
        headers.insert(HSK_HEADER_ACTOR_ID, "tripwire-actor".parse()?);
        let (code, body) = create_memory_proposal(
            State(state.clone()),
            Path(workspace_id.clone()),
            headers,
            Json(request.clone()),
        )
        .await
        .err()
        .expect("a durable artifact with a non-UUID proposal_id must be rejected");
        assert_eq!(
            code,
            StatusCode::CONFLICT,
            "rejection must be the canonical-contract conflict, not an unrelated failure"
        );
        assert_eq!(
            body.0["detail"], "memory proposal artifact violates the canonical proposal contract",
            "the ONLY thing wrong with this artifact is its non-UUID proposal_id"
        );
        let receipt_key = format!("fems-memory-proposal:{legacy_proposal_id}");
        let receipts = usize::from(
            memory_test_ledger_event(&state, &receipt_key)
                .await?
                .is_some(),
        );
        assert_eq!(receipts, 0, "the rejected retry must append no receipt");

        // CASE 2: the heal cannot be MINTED. A first-time insert with no `_canonical_artifact`
        // stays rejected, so the relaxed id check is unreachable from the creation path.
        let minted_workspace = create_test_workspace(&state, "heal-mint").await?;
        let minted_proposal_id = format!("PROP-LEGACY-{}", Uuid::now_v7());
        let minted_request_id = format!("mint-{}", Uuid::now_v7());
        let minted = fems_memory::StoredMemoryProposal {
            proposal_id: minted_proposal_id.clone(),
            request_id: minted_request_id,
            workspace_id: minted_workspace.clone(),
            document_id: "doc-mint".to_owned(),
            selection_start: 0,
            selection_end: 1,
            content_hash: "c".repeat(64),
            memory_class: "episodic".to_owned(),
            status: PROPOSAL_STATUS_PENDING_REVIEW.to_owned(),
            review_gated: true,
            created_at,
            proposal: json!({
                "proposal_id": minted_proposal_id,
                "workspace_id": minted_workspace,
                "class": "episodic",
                "content": "minted without a canonical artifact",
                "review_gated": true,
                "status": PROPOSAL_STATUS_PENDING_REVIEW,
            }),
        };
        let receipt = NewKernelEvent::builder(
            "mint-task",
            "mint-session",
            KernelEventType::ArtifactProposed,
            KernelActor::Operator("mint-actor".to_owned()),
        )
        .aggregate("fems_memory_proposal", minted_proposal_id.clone())
        .idempotency_key(format!("fems-memory-proposal:{minted_proposal_id}"))
        .source_component("fems_memory_proposal_intake")
        .payload(json!({"proposal_id": &minted_proposal_id}))
        .build()?;
        let error =
            fems_memory::insert_memory_proposal_with_receipt(&state.surreal, &minted, receipt)
                .await
                .expect_err("a first-time insert with no canonical artifact must fail closed");
        assert!(
            matches!(&error, StorageError::Serialization(detail)
                if detail == "memory proposal artifact is not hsk.memory_write_proposal@0.1: missing field `schema_version`"),
            "the mint path must keep failing with the exact pre-MT-118 error, got {error:?}"
        );
        assert!(
            fems_memory::get_memory_proposal(&state.surreal, &minted_proposal_id)
                .await?
                .is_none(),
            "the rejected mint must leave no durable proposal row"
        );
        let minted_receipts = usize::from(
            memory_test_ledger_event(
                &state,
                &format!("fems-memory-proposal:{minted_proposal_id}"),
            )
            .await?
            .is_some(),
        );
        assert_eq!(
            minted_receipts, 0,
            "the rejected mint must append no receipt"
        );
        Ok::<(), Box<dyn std::error::Error>>(())
        }))
        .await;
        drop(state);
        finish_mt109_memory_test(body, store, data_dir).await
    }

    /// Counterfactual partial failure: if execution stops after the proposal INSERT but
    /// before receipt append, the enclosing transaction rolls the proposal back.
    #[tokio::test]
    async fn proposal_insert_rolls_back_when_receipt_phase_fails(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (store, state) = setup_state().await?;
        let data_dir = store.data_dir.clone();
        let body = futures::FutureExt::catch_unwind(std::panic::AssertUnwindSafe(async {
        let workspace_id = create_test_workspace(&state, "rollback").await?;
        let request_id = format!("forced-failure-{}", Uuid::now_v7());
        let proposal_id = stable_proposal_id(&workspace_id, &request_id);
        let content = "rollback after a valid proposal insert";
        let source =
            create_test_rich_source(&state, &workspace_id, "rollback-source", content).await?;
        let proposal_scope = proposal_create_test_scope(
            &state,
            &workspace_id,
            &source.document_id,
            "rollback",
            "native_editor",
        )
        .await?;
        let created_at =
            chrono::DateTime::from_timestamp_micros(chrono::Utc::now().timestamp_micros())
                .expect("normalized rollback fixture instant");
        let mut proposal = fems_memory::StoredMemoryProposal {
            proposal_id: proposal_id.clone(),
            request_id: request_id.clone(),
            workspace_id: workspace_id.clone(),
            document_id: source.document_id.clone(),
            selection_start: i64::try_from(source.selection_start)?,
            selection_end: i64::try_from(source.selection_end)?,
            content_hash: source.content_hash.clone(),
            memory_class: "semantic".to_string(),
            status: PROPOSAL_STATUS_PENDING_REVIEW.to_string(),
            review_gated: true,
            created_at,
            proposal: json!({
                "proposal_id": &proposal_id,
                "request_id": &request_id,
                "workspace_id": &workspace_id,
                "class": "semantic",
                "content": content,
                "source": source,
                "review_gated": true,
                "status": PROPOSAL_STATUS_PENDING_REVIEW,
                "actor_id": "native_editor",
            }),
        };
        let canonical = fems_memory::proposal_canonical_artifact(
            &proposal,
            fems_memory::LegacyArtifactHeal::Allow,
        );
        assert_eq!(
            canonical.origin,
            fems_memory::ProposalArtifactOrigin::HealedFromDurableColumns
        );
        proposal
            .proposal
            .as_object_mut()
            .expect("rollback proposal payload is an object")
            .insert("_canonical_artifact".to_owned(), canonical.value);
        let receipt = NewKernelEvent::builder(
            "forced-failure-task",
            "forced-failure-session",
            KernelEventType::ArtifactProposed,
            KernelActor::System("forced-failure-test".to_string()),
        )
        .aggregate("fems_memory_proposal", proposal_id.clone())
        .idempotency_key(format!("fems-memory-proposal:{proposal_id}"))
        .source_component("fems_memory_proposal_intake")
        .payload(json!({"proposal_id": &proposal_id, "workspace_id": &workspace_id}))
        .build()?;

        let forced = fems_memory::insert_memory_proposal_with_receipt_forced_failure(
            &state.surreal,
            &proposal,
            receipt,
        );
        let error = state
            .surreal
            .with_record_user_scope(proposal_scope, forced)
            .await
            .expect_err("forced receipt-phase failure must surface");
        assert!(
            error.to_string().contains("forced failure after proposal insert"),
            "rollback proof must reach the injected post-insert failure, got {error}"
        );
        assert!(
            fems_memory::get_memory_proposal(&state.surreal, &proposal_id)
                .await?
                .is_none(),
            "partial proposal insert must roll back"
        );
        let receipt_count = usize::from(
            memory_test_ledger_event(&state, &format!("fems-memory-proposal:{proposal_id}"))
                .await?
                .is_some(),
        );
        assert_eq!(
            receipt_count, 0,
            "failed transaction must append no receipt"
        );
        Ok::<(), Box<dyn std::error::Error>>(())
        }))
        .await;
        drop(state);
        finish_mt109_memory_test(body, store, data_dir).await
    }

    /// AC-109-3 NEGATIVE: submitting a proposal can NOT mutate committed memory. A
    /// pre-seeded committed item is byte-unchanged and the committed-item count does not
    /// grow after the proposal is submitted.
    #[tokio::test]
    async fn proposal_cannot_mutate_committed_memory() -> Result<(), Box<dyn std::error::Error>> {
        let (store, state) = setup_state().await?;
        let data_dir = store.data_dir.clone();
        let body = futures::FutureExt::catch_unwind(std::panic::AssertUnwindSafe(async {
        let workspace_id = create_test_workspace(&state, "negative").await?;
        let memory_id = "MEM-COMMITTED-1";
        let committed_content = "committed truth";
        let committed_source = create_test_rich_source(
            &state,
            &workspace_id,
            "negative-committed-source",
            committed_content,
        )
        .await?;
        let committed = serde_json::to_value(crate::ace::MemoryPackItem {
            memory_id: memory_id.to_owned(),
            memory_class: "semantic".to_owned(),
            item_type: "fact".to_owned(),
            summary: committed_content.to_owned(),
            content: committed_content.to_owned(),
            structured: None,
            trust_level: "user_asserted".to_owned(),
            confidence: 1.0,
            scope_refs: Vec::new(),
            source_refs: vec![FemsSourceRef {
                kind: FemsSourceRefKind::DocBlock,
                id: committed_source.document_id,
                hash: Some(committed_source.content_hash),
                selector: Some(format!(
                    "bytes:{}-{}",
                    committed_source.selection_start, committed_source.selection_end
                )),
                created_at: None,
                classification: Some("low".to_owned()),
            }],
            pinned: false,
            last_verified_at: None,
        })?;
        fems_memory::upsert_memory_item(&state.surreal, &workspace_id, memory_id, &committed)
            .await?;
        let before = fems_memory::count_memory_items(&state.surreal, &workspace_id).await?;
        assert_eq!(before, 1);
        let Json(before_exact) =
            get_committed_memory_count(State(state.clone()), Path(workspace_id.clone()))
                .await
                .map_err(|(code, body)| format!("committed count failed: {code} {body:?}"))?;
        assert_eq!(before_exact.count, 1);

        let other_workspace = create_test_workspace(&state, "negative-control").await?;
        fems_memory::upsert_memory_item(
            &state.surreal,
            &other_workspace,
            "MEM-OTHER-WORKSPACE",
            &json!({"content": "must not leak into target count"}),
        )
        .await?;

        // Submit a proposal that names the committed memory in its content.
        let content = format!("overwrite {memory_id} with attacker text");
        let source =
            create_test_rich_source(&state, &workspace_id, "negative-source", &content).await?;
        let proposal_scope = proposal_create_test_scope(
            &state,
            &workspace_id,
            &source.document_id,
            "negative",
            "native_editor",
        )
        .await?;
        let request = ProposalRequest {
            request_id: None,
            class: ProposalClass::Procedural,
            content: content.clone(),
            source,
            source_document_content: None,
            review_gated: Some(true),
            actor_id: Some("attacker".to_string()),
        };
        let create = create_memory_proposal(
            State(state.clone()),
            Path(workspace_id.clone()),
            HeaderMap::new(),
            Json(request),
        );
        let Json(ack) = state
            .surreal
            .with_record_user_scope(proposal_scope, create)
            .await
        .map_err(|(code, body)| format!("proposal failed: {code} {body:?}"))?;

        // The committed item is byte-for-byte unchanged.
        let after_item = fems_memory::get_memory_item(&state.surreal, &workspace_id, memory_id)
            .await?
            .expect("committed item still present");
        assert_eq!(
            after_item, committed,
            "proposal must not mutate committed memory"
        );

        // No new committed item was created; the count is unchanged.
        let after = fems_memory::count_memory_items(&state.surreal, &workspace_id).await?;
        assert_eq!(after, 1, "proposal must not create a committed memory item");
        let Json(after_exact) =
            get_committed_memory_count(State(state.clone()), Path(workspace_id.clone()))
                .await
                .map_err(|(code, body)| format!("committed count failed: {code} {body:?}"))?;
        assert_eq!(
            after_exact.count, 1,
            "canonical endpoint must remain unchanged"
        );
        assert_eq!(after_exact.workspace_id, workspace_id);

        // The proposal itself is only pending_review.
        let stored = fems_memory::get_memory_proposal(&state.surreal, &ack.proposal_id)
            .await?
            .expect("proposal stored");
        assert_eq!(stored.status, PROPOSAL_STATUS_PENDING_REVIEW);
        Ok::<(), Box<dyn std::error::Error>>(())
        }))
        .await;
        drop(state);
        finish_mt109_memory_test(body, store, data_dir).await
    }

    /// Workspace teardown removes only the target's mutable FEMS projections. The
    /// control workspace remains intact and canonical EventLedger receipts are retained.
    #[tokio::test]
    async fn workspace_memory_cleanup_is_scoped_and_preserves_receipts(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (store, state) = setup_state().await?;
        let data_dir = store.data_dir.clone();
        let body = futures::FutureExt::catch_unwind(std::panic::AssertUnwindSafe(async {
        let target = create_test_workspace(&state, "cleanup-target").await?;
        let control = create_test_workspace(&state, "cleanup-control").await?;

        fems_memory::upsert_memory_pack(&state.surreal, &target, "", &seeded_pack(&target)).await?;
        fems_memory::upsert_memory_pack(&state.surreal, &control, "", &seeded_pack(&control))
            .await?;
        fems_memory::upsert_memory_item(
            &state.surreal,
            &target,
            &format!("MEM-TARGET-{}", Uuid::now_v7()),
            &json!({"content": "target"}),
        )
        .await?;
        fems_memory::upsert_memory_item(
            &state.surreal,
            &control,
            &format!("MEM-CONTROL-{}", Uuid::now_v7()),
            &json!({"content": "control"}),
        )
        .await?;

        let target_content = "target pending proposal";
        let target_source =
            create_test_rich_source(&state, &target, "cleanup-target-source", target_content)
                .await?;
        let target_scope = proposal_create_test_scope(
            &state,
            &target,
            &target_source.document_id,
            "cleanup",
            "native_editor",
        )
        .await?;
        let target_request = ProposalRequest {
            request_id: Some(format!("cleanup-target-{}", Uuid::now_v7())),
            class: ProposalClass::Semantic,
            content: target_content.to_string(),
            source: target_source,
            source_document_content: None,
            review_gated: Some(true),
            actor_id: Some("cleanup-test".to_string()),
        };
        let control_content = "control pending proposal";
        let control_source =
            create_test_rich_source(&state, &control, "cleanup-control-source", control_content)
                .await?;
        let control_scope = proposal_create_test_scope(
            &state,
            &control,
            &control_source.document_id,
            "cleanup",
            "native_editor",
        )
        .await?;
        let control_request = ProposalRequest {
            request_id: Some(format!("cleanup-control-{}", Uuid::now_v7())),
            class: ProposalClass::Semantic,
            content: control_content.to_string(),
            source: control_source,
            source_document_content: None,
            review_gated: Some(true),
            actor_id: Some("cleanup-test".to_string()),
        };
        let target_call = create_memory_proposal(
            State(state.clone()),
            Path(target.clone()),
            HeaderMap::new(),
            Json(target_request),
        );
        let Json(target_ack) = state
            .surreal
            .with_record_user_scope(target_scope, target_call)
            .await
        .map_err(|(code, body)| format!("target proposal failed: {code} {body:?}"))?;
        let control_call = create_memory_proposal(
            State(state.clone()),
            Path(control.clone()),
            HeaderMap::new(),
            Json(control_request),
        );
        let Json(control_ack) = state
            .surreal
            .with_record_user_scope(control_scope, control_call)
            .await
        .map_err(|(code, body)| format!("control proposal failed: {code} {body:?}"))?;

        state
            .storage
            .delete_workspace(
                &WriteContext::human(Some("fems-memory-test".to_owned())),
                &target,
            )
            .await?;

        assert!(
            fems_memory::get_latest_memory_pack(&state.surreal, &target, None)
                .await?
                .is_none()
        );
        assert_eq!(
            fems_memory::count_memory_items(&state.surreal, &target).await?,
            0
        );
        assert!(
            fems_memory::get_memory_proposal(&state.surreal, &target_ack.proposal_id)
                .await?
                .is_none()
        );

        assert!(
            fems_memory::get_latest_memory_pack(&state.surreal, &control, None)
                .await?
                .is_some()
        );
        assert_eq!(
            fems_memory::count_memory_items(&state.surreal, &control).await?,
            1
        );
        assert!(
            fems_memory::get_memory_proposal(&state.surreal, &control_ack.proposal_id)
                .await?
                .is_some()
        );
        let target_receipt_count = usize::from(
            memory_test_ledger_event(
                &state,
                &format!("fems-memory-proposal:{}", target_ack.proposal_id),
            )
            .await?
            .is_some(),
        );
        assert_eq!(
            target_receipt_count, 1,
            "cleanup must preserve EventLedger receipt"
        );
            Ok::<(), Box<dyn std::error::Error>>(())
        }))
        .await;
        drop(state);
        finish_mt109_memory_test(body, store, data_dir).await
    }

    #[tokio::test]
    async fn proposal_delete_race_never_leaves_workspace_orphans(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (store, state) = setup_state().await?;
        let data_dir = store.data_dir.clone();
        let body = futures::FutureExt::catch_unwind(std::panic::AssertUnwindSafe(async {
        let workspace_id = create_test_workspace(&state, "delete-race").await?;
        let content = "proposal racing workspace deletion";
        let source =
            create_test_rich_source(&state, &workspace_id, "delete-race-source", content).await?;
        let proposal_scope = proposal_create_test_scope(
            &state,
            &workspace_id,
            &source.document_id,
            "delete-race",
            "native_editor",
        )
        .await?;
        let request = ProposalRequest {
            request_id: Some(format!("delete-race-{}", Uuid::now_v7())),
            class: ProposalClass::Semantic,
            content: content.to_owned(),
            source,
            source_document_content: None,
            review_gated: Some(true),
            actor_id: Some("delete-race-test".to_owned()),
        };
        let ctx = WriteContext::human(Some("delete-race-test".to_owned()));

        let deletion = state.storage.delete_workspace(&ctx, &workspace_id);
        let proposal_call = create_memory_proposal(
            State(state.clone()),
            Path(workspace_id.clone()),
            HeaderMap::new(),
            Json(request.clone()),
        );
        let proposal = state
            .surreal
            .with_record_user_scope(proposal_scope, proposal_call);
        let (deletion, proposal) = tokio::join!(deletion, proposal);
        deletion?;
        if let Err((status, _)) = proposal {
            assert_eq!(status, StatusCode::NOT_FOUND);
        }

        assert!(state.storage.get_workspace(&workspace_id).await?.is_none());
        let orphan_count = memory_test_count(
            &state,
            "SELECT count() AS count FROM fems_memory_proposals \
             WHERE workspace_id = $workspace GROUP ALL;",
            MemoryTestBindings {
                workspace: Some(RecordId::new("workspaces", workspace_id.as_str())),
                ..MemoryTestBindings::default()
            },
        )
        .await?;
        assert_eq!(orphan_count, 0, "workspace deletion must cascade proposals");

        let after_delete = create_memory_proposal(
            State(state.clone()),
            Path(workspace_id),
            HeaderMap::new(),
            Json(request),
        )
        .await
        .expect_err("proposal route must reject a deleted workspace");
        assert_eq!(after_delete.0, StatusCode::NOT_FOUND);
        Ok::<(), Box<dyn std::error::Error>>(())
        }))
        .await;
        drop(state);
        finish_mt109_memory_test(body, store, data_dir).await
    }

    /// AC-109-3 fail-closed: missing/invalid provenance is rejected with 400 and nothing
    /// is stored.
    #[tokio::test]
    async fn proposal_missing_provenance_fails_closed() -> Result<(), Box<dyn std::error::Error>> {
        let (store, state) = setup_state().await?;
        let data_dir = store.data_dir.clone();
        let body = futures::FutureExt::catch_unwind(std::panic::AssertUnwindSafe(async {
        let workspace_id = create_test_workspace(&state, "fail-closed").await?;

        // Empty content_hash.
        let bad_hash = ProposalRequest {
            request_id: None,
            class: ProposalClass::Episodic,
            content: "x".to_string(),
            source: ProposalSource {
                content_hash: String::new(),
                ..valid_source(&workspace_id, "doc-1", "x")
            },
            source_document_content: None,
            review_gated: Some(true),
            actor_id: None,
        };
        let err = create_memory_proposal(
            State(state.clone()),
            Path(workspace_id.clone()),
            HeaderMap::new(),
            Json(bad_hash),
        )
        .await
        .expect_err("empty content_hash must be rejected");
        assert_eq!(err.0, StatusCode::BAD_REQUEST);

        let valid = ProposalRequest {
            request_id: None,
            class: ProposalClass::Episodic,
            content: "x".to_owned(),
            source: valid_source(&workspace_id, "doc-1", "x"),
            source_document_content: None,
            review_gated: Some(true),
            actor_id: None,
        };

        let missing_source = create_memory_proposal(
            State(state.clone()),
            Path(workspace_id.clone()),
            HeaderMap::new(),
            Json(valid.clone()),
        )
        .await
        .expect_err("nonexistent rich/code source must fail closed");
        assert_eq!(missing_source.0, StatusCode::BAD_REQUEST);

        for untrusted_gate in [None, Some(false)] {
            let mut ungated = valid.clone();
            ungated.review_gated = untrusted_gate;
            let err = create_memory_proposal(
                State(state.clone()),
                Path(workspace_id.clone()),
                HeaderMap::new(),
                Json(ungated),
            )
            .await
            .expect_err("false or omitted review gating must fail before persistence");
            assert_eq!(err.0, StatusCode::BAD_REQUEST);
        }

        let mut empty_content = valid.clone();
        empty_content.content.clear();
        empty_content.source.selection_end = empty_content.source.selection_start;
        empty_content.source.content_hash =
            canonical_content_hash("").map_err(|_| "failed to build empty-content hash fixture")?;
        let err = create_memory_proposal(
            State(state.clone()),
            Path(workspace_id.clone()),
            HeaderMap::new(),
            Json(empty_content),
        )
        .await
        .expect_err("empty selection must fail before persistence");
        assert_eq!(err.0, StatusCode::BAD_REQUEST);

        let mut mismatched_range = valid.clone();
        mismatched_range.source.selection_end = mismatched_range.source.selection_start + 2;
        let err = create_memory_proposal(
            State(state.clone()),
            Path(workspace_id.clone()),
            HeaderMap::new(),
            Json(mismatched_range),
        )
        .await
        .expect_err("selection byte range/content mismatch must fail before persistence");
        assert_eq!(err.0, StatusCode::BAD_REQUEST);

        let unicode_content = "é🙂";
        let unicode_source = create_test_rich_source(
            &state,
            &workspace_id,
            "unicode-byte-source",
            unicode_content,
        )
        .await?;
        let unicode_request = ProposalRequest {
            request_id: Some(format!("unicode-bytes-{}", Uuid::now_v7())),
            class: ProposalClass::Semantic,
            content: unicode_content.to_owned(),
            source: unicode_source,
            source_document_content: None,
            review_gated: Some(true),
            actor_id: Some("unicode-byte-test".to_owned()),
        };
        let unicode_scope = proposal_create_test_scope(
            &state,
            &workspace_id,
            &unicode_request.source.document_id,
            "unicode-byte",
            "native_editor",
        )
        .await?;
        let unicode_create = create_memory_proposal(
            State(state.clone()),
            Path(workspace_id.clone()),
            HeaderMap::new(),
            Json(unicode_request),
        );
        let Json(unicode_ack) = state
            .surreal
            .with_record_user_scope(unicode_scope, unicode_create)
            .await
        .map_err(|(code, body)| format!("valid UTF-8 byte range failed: {code} {body:?}"))?;
        let unicode_stored =
            fems_memory::get_memory_proposal(&state.surreal, &unicode_ack.proposal_id)
                .await?
                .expect("valid UTF-8 proposal persisted");
        assert_eq!(
            unicode_stored.selection_end - unicode_stored.selection_start,
            unicode_content.len() as i64
        );

        let mut missing_workspace = valid.clone();
        missing_workspace.source.workspace_id = None;
        let err = create_memory_proposal(
            State(state.clone()),
            Path(workspace_id.clone()),
            HeaderMap::new(),
            Json(missing_workspace),
        )
        .await
        .expect_err("missing source.workspace_id must be rejected");
        assert_eq!(err.0, StatusCode::BAD_REQUEST);

        let mut mismatched_workspace = valid.clone();
        mismatched_workspace.source.workspace_id = Some(format!("{workspace_id}-other"));
        let err = create_memory_proposal(
            State(state.clone()),
            Path(workspace_id.clone()),
            HeaderMap::new(),
            Json(mismatched_workspace),
        )
        .await
        .expect_err("mismatched source.workspace_id must be rejected");
        assert_eq!(err.0, StatusCode::BAD_REQUEST);

        let mut overflowed_selection = valid.clone();
        overflowed_selection.source.selection_end = u64::MAX;
        let err = create_memory_proposal(
            State(state.clone()),
            Path(workspace_id.clone()),
            HeaderMap::new(),
            Json(overflowed_selection),
        )
        .await
        .expect_err("selection offsets beyond i64 must be rejected");
        assert_eq!(err.0, StatusCode::BAD_REQUEST);

        let mut wrong_canonical_hash = valid;
        wrong_canonical_hash.source.content_hash = "a".repeat(64);
        let err = create_memory_proposal(
            State(state.clone()),
            Path(workspace_id.clone()),
            HeaderMap::new(),
            Json(wrong_canonical_hash),
        )
        .await
        .expect_err("well-shaped hash for different content must be rejected");
        assert_eq!(err.0, StatusCode::BAD_REQUEST);

        // Empty document_id.
        let bad_doc = ProposalRequest {
            request_id: None,
            class: ProposalClass::Episodic,
            content: "x".to_string(),
            source: valid_source(&workspace_id, "", "x"),
            source_document_content: None,
            review_gated: Some(true),
            actor_id: None,
        };
        let err = create_memory_proposal(
            State(state.clone()),
            Path(workspace_id.clone()),
            HeaderMap::new(),
            Json(bad_doc),
        )
        .await
        .expect_err("empty document_id must be rejected");
        assert_eq!(err.0, StatusCode::BAD_REQUEST);

        // Exactly the valid Unicode byte-range control was stored. Every rejected request above must
        // leave no proposal or EventLedger residue.
        fems_memory::ensure_fems_memory_schema(&state.surreal).await?;
        let stored = fems_memory::list_memory_proposals(&state.surreal, &workspace_id, 200).await?;
        let count = stored.len();
        assert_eq!(
            count, 1,
            "only the valid Unicode byte-range control may be stored"
        );
        let stored_ids: Vec<String> = stored
            .into_iter()
            .map(|proposal| proposal.proposal_id)
            .collect();
        assert_eq!(stored_ids, vec![unicode_ack.proposal_id.clone()]);
        let receipt_count = state
            .storage
            .list_kernel_events_for_aggregate("fems_memory_proposal", &unicode_ack.proposal_id)
            .await?
            .len();
        assert_eq!(receipt_count, 1, "only the valid control emits a receipt");
        Ok::<(), Box<dyn std::error::Error>>(())
        }))
        .await;
        drop(state);
        finish_mt109_memory_test(body, store, data_dir).await
    }

    #[tokio::test]
    async fn loom_block_reference_proposal_accepts_only_an_existing_exact_canonical_address(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (store, state) = setup_state().await?;
        let data_dir = store.data_dir.clone();
        let body = futures::FutureExt::catch_unwind(std::panic::AssertUnwindSafe(async {
        let workspace_id = create_test_workspace(&state, "loom-reference").await?;
        let block = state
            .storage
            .create_loom_block(
                &WriteContext::human(Some("fems-loom-reference-test".to_owned())),
                NewLoomBlock {
                    block_id: None,
                    workspace_id: workspace_id.clone(),
                    content_type: LoomBlockContentType::Note,
                    document_id: None,
                    asset_id: None,
                    title: Some("Canonical Loom reference".to_owned()),
                    original_filename: None,
                    content_hash: None,
                    pinned: false,
                    journal_date: None,
                    imported_at: None,
                    derived: LoomBlockDerived::default(),
                },
            )
            .await?;
        let canonical_ref = format!("loom://{}", block.block_id);
        let canonical = ProposalRequest {
            request_id: Some(format!("loom-reference-{}", Uuid::now_v7())),
            class: ProposalClass::Semantic,
            content: canonical_ref.clone(),
            source: ProposalSource {
                document_id: block.block_id.clone(),
                selection_start: 0,
                selection_end: canonical_ref.len() as u64,
                content_hash: canonical_content_hash(&canonical_ref)
                    .expect("canonical Loom reference hash"),
                document_content_hash: None,
                pane_id: Some("pane-loom".to_owned()),
                workspace_id: Some(workspace_id.clone()),
            },
            source_document_content: None,
            review_gated: Some(true),
            actor_id: Some("loom-reference-test".to_owned()),
        };

        let Json(ack) = create_memory_proposal(
            State(state.clone()),
            Path(workspace_id.clone()),
            HeaderMap::new(),
            Json(canonical.clone()),
        )
        .await
        .map_err(|(status, body)| format!("canonical Loom reference failed: {status} {body:?}"))?;
        assert_eq!(ack.status, PROPOSAL_STATUS_PENDING_REVIEW);

        let mut fabricated = canonical;
        fabricated.request_id = Some(format!("loom-fabricated-{}", Uuid::now_v7()));
        fabricated.source.document_id = Uuid::now_v7().to_string();
        fabricated.content = format!("loom://{}", fabricated.source.document_id);
        fabricated.source.selection_end = fabricated.content.len() as u64;
        fabricated.source.content_hash =
            canonical_content_hash(&fabricated.content).expect("fabricated Loom reference hash");
        let rejected = create_memory_proposal(
            State(state.clone()),
            Path(workspace_id),
            HeaderMap::new(),
            Json(fabricated),
        )
        .await
        .expect_err("a fabricated Loom address must fail closed");
        assert!(
            matches!(rejected.0, StatusCode::BAD_REQUEST | StatusCode::NOT_FOUND),
            "fabricated Loom address returned unexpected status {}",
            rejected.0
        );
        Ok::<(), Box<dyn std::error::Error>>(())
        })).await;
        drop(state);
        finish_mt109_memory_test(body, store, data_dir).await
    }

    /// Typed rejection: an unknown top-level field or unknown class is rejected at decode
    /// (deny_unknown_fields + closed class vocabulary).
    #[test]
    fn proposal_body_rejects_unknown_field_and_unknown_class() {
        // Unknown top-level field.
        let unknown_field = json!({
            "class": "episodic",
            "content": "x",
            "source": {
                "document_id": "doc-1",
                "selection_start": 0,
                "selection_end": 1,
                "content_hash": "a".repeat(64),
                "pane_id": "p",
                "workspace_id": "w"
            },
            "review_gated": true,
            "actor_id": "a",
            "smuggled": "free text"
        });
        assert!(serde_json::from_value::<ProposalRequest>(unknown_field).is_err());

        // Unknown memory class.
        let unknown_class = json!({
            "class": "working",
            "content": "x",
            "source": {
                "document_id": "doc-1",
                "selection_start": 0,
                "selection_end": 1,
                "content_hash": "a".repeat(64),
                "pane_id": "p",
                "workspace_id": "w"
            }
        });
        assert!(serde_json::from_value::<ProposalRequest>(unknown_class).is_err());
    }

    #[tokio::test]
    async fn proposal_route_rejects_unknown_field_and_class_without_durable_residue(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (store, state) = setup_state().await?;
        let data_dir = store.data_dir.clone();
        let body = futures::FutureExt::catch_unwind(std::panic::AssertUnwindSafe(async {
        let workspace_id = create_test_workspace(&state, "strict-route").await?;
        let content = "strict proposal";
        let source =
            create_test_rich_source(&state, &workspace_id, "strict-route-source", content).await?;
        let valid = json!({
            "class": "semantic",
            "content": content,
            "source": source,
            "review_gated": true,
            "actor_id": "strict-route-actor"
        });
        let before_proposals =
            fems_memory::list_memory_proposals(&state.surreal, &workspace_id, 200)
                .await?
                .len();
        let count_workspace_receipts = || {
            memory_test_count(
                &state,
                "SELECT count() AS count FROM kernel_event_ledger \
                 WHERE event_type = $other AND payload.workspace_id = $value GROUP ALL;",
                MemoryTestBindings {
                    value: Some(workspace_id.clone()),
                    other: Some(KernelEventType::ArtifactProposed.as_str().to_owned()),
                    ..MemoryTestBindings::default()
                },
            )
        };
        let before_ledger = count_workspace_receipts().await?;
        let before_flight_recorder = state
            .flight_recorder
            .list_events(EventFilter {
                event_type: Some("memory_write_proposed".to_owned()),
                wsid: Some(workspace_id.clone()),
                ..EventFilter::default()
            })
            .await?
            .len();
        let (base, http, server) = serve_test_router(routes(state.clone())).await;
        let endpoint = format!("{base}/workspaces/{workspace_id}/memory/proposals");

        let mut unknown_field = valid.clone();
        unknown_field["smuggled"] = json!("free text");
        let response = http.post(&endpoint).json(&unknown_field).send().await?;
        assert!(
            response.status().is_client_error(),
            "unknown proposal field must be rejected by the mounted route, got {}",
            response.status()
        );

        let mut unknown_class = valid;
        unknown_class["class"] = json!("working");
        let response = http.post(&endpoint).json(&unknown_class).send().await?;
        assert!(
            response.status().is_client_error(),
            "unknown proposal class must be rejected by the mounted route, got {}",
            response.status()
        );

        let after_proposals =
            fems_memory::list_memory_proposals(&state.surreal, &workspace_id, 200)
                .await?
                .len();
        let after_flight_recorder = state
            .flight_recorder
            .list_events(EventFilter {
                event_type: Some("memory_write_proposed".to_owned()),
                wsid: Some(workspace_id.clone()),
                ..EventFilter::default()
            })
            .await?
            .len();
        let after_ledger = count_workspace_receipts().await?;
        assert_eq!(
            after_proposals, before_proposals,
            "rejected bodies must persist no proposal"
        );
        assert_eq!(
            after_ledger, before_ledger,
            "rejected bodies must emit no EventLedger receipt"
        );
        assert_eq!(
            after_flight_recorder, before_flight_recorder,
            "rejected bodies must emit no memory-write-proposed Flight Recorder event"
        );
        server.abort();
            Ok::<(), Box<dyn std::error::Error>>(())
        }))
        .await;
        drop(state);
        finish_mt109_memory_test(body, store, data_dir).await
    }

    #[test]
    fn proposal_review_body_is_closed_and_typed() {
        assert!(serde_json::from_value::<ProposalReviewRequest>(json!({
            "decision": "approved",
            "reviewer_kind": "user",
            "smuggled": true
        }))
        .is_err());
        assert!(serde_json::from_value::<ProposalReviewRequest>(json!({
            "decision": "partial",
            "reviewer_kind": "user"
        }))
        .is_err());
        assert!(serde_json::from_value::<ProposalReviewRequest>(json!({
            "decision": "rejected",
            "reviewer_kind": "agent"
        }))
        .is_err());
    }

    #[tokio::test]
    async fn proposal_review_transitions_are_audited_idempotent_and_conflict_safe(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (store, state) = setup_state().await?;
        let data_dir = store.data_dir.clone();
        let body = futures::FutureExt::catch_unwind(std::panic::AssertUnwindSafe(async {
        let workspace_id = create_test_workspace(&state, "proposal-review").await?;
        let content = "review this durable fact";
        let source =
            create_test_rich_source(&state, &workspace_id, "review-source", content).await?;
        let proposal_scope = proposal_create_test_scope(
            &state,
            &workspace_id,
            &source.document_id,
            "proposal-review-create",
            "canonical-proposer",
        )
        .await?;
        let mut proposal_headers = HeaderMap::new();
        proposal_headers.insert(HSK_HEADER_ACTOR_ID, "canonical-proposer".parse()?);
        proposal_headers.insert(HSK_HEADER_ACTOR_KIND, "human".parse()?);
        let proposal_call = create_memory_proposal(
            State(state.clone()),
            Path(workspace_id.clone()),
            proposal_headers.clone(),
            Json(ProposalRequest {
                request_id: Some(format!("review-request-{}", Uuid::now_v7())),
                class: ProposalClass::Semantic,
                content: content.to_owned(),
                source,
                source_document_content: None,
                review_gated: Some(true),
                actor_id: Some("canonical-proposer".to_owned()),
            }),
        );
        let Json(proposal_ack) = state
            .surreal
            .with_record_user_scope(proposal_scope, proposal_call)
            .await
        .map_err(|(code, body)| format!("proposal failed: {code} {body:?}"))?;

        let pending = fems_memory::get_memory_proposal(&state.surreal, &proposal_ack.proposal_id)
            .await?
            .expect("proposal stored");
        assert_eq!(pending.proposal["actor_id"], "canonical-proposer");
        assert!(pending.created_at <= chrono::Utc::now());
        assert!(
            serde_json::to_value(&pending)?["created_at"].is_string(),
            "proposal read APIs expose the authoritative SurrealDB creation timestamp"
        );
        let proposal_receipt = memory_test_ledger_event(
            &state,
            &format!("fems-memory-proposal:{}", proposal_ack.proposal_id),
        )
        .await?
        .expect("proposal receipt");
        assert_eq!(proposal_receipt.actor.actor_id(), "canonical-proposer");
        assert_eq!(
            proposal_receipt.correlation_id,
            Some(format!("fems-memory-proposal:{}", proposal_ack.proposal_id))
        );

        let mut review_headers = HeaderMap::new();
        review_headers.insert(HSK_HEADER_ACTOR_ID, "reviewer-1".parse()?);
        review_headers.insert(HSK_HEADER_ACTOR_KIND, "operator".parse()?);
        let review_request = ProposalReviewRequest {
            decision: ProposalReviewDecision::Approved,
            reviewer_kind: ProposalReviewerKind::User,
            reason: Some("source checked".to_owned()),
        };
        let Json(first) = review_memory_proposal(
            State(state.clone()),
            Path((workspace_id.clone(), proposal_ack.proposal_id.clone())),
            review_headers.clone(),
            Json(review_request.clone()),
        )
        .await
        .map_err(|(code, body)| format!("review failed: {code} {body:?}"))?;
        assert_eq!(first.status, "approved");
        let approved = fems_memory::get_memory_proposal(&state.surreal, &proposal_ack.proposal_id)
            .await?
            .expect("reviewed proposal stored");
        assert_eq!(approved.status, "approved");
        assert_eq!(approved.proposal["review"]["actor_id"], "reviewer-1");
        assert_eq!(approved.proposal["review"]["decision"], "approved");

        let review_receipt = memory_test_ledger_event(
            &state,
            &format!("fems-memory-proposal-review:{}", proposal_ack.proposal_id),
        )
        .await?
        .expect("review receipt");
        assert_eq!(
            review_receipt.event_type,
            KernelEventType::PromotionAccepted
        );
        assert_eq!(review_receipt.actor.actor_id(), "reviewer-1");
        assert_eq!(
            review_receipt.correlation_id,
            Some(first.correlation_id.clone())
        );
        assert_eq!(review_receipt.payload["content_hash"], pending.content_hash);
        assert_eq!(review_receipt.payload["reason_present"], true);
        assert!(review_receipt.payload.get("content").is_none());

        let events = state
            .flight_recorder
            .list_events(EventFilter {
                event_id: Some(first.flight_recorder_event_id),
                ..EventFilter::default()
            })
            .await?;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].actor_id, "reviewer-1");
        assert_eq!(
            events[0].trace_id,
            deterministic_uuid_from_seed(&first.correlation_id)
        );
        assert_eq!(events[0].payload["decision"], "approved");
        assert!(
            events[0].payload.get("commit_report_ref").is_none(),
            "review events omit the optional commit report until a commit exists"
        );

        let Json(replay) = review_memory_proposal(
            State(state.clone()),
            Path((workspace_id.clone(), proposal_ack.proposal_id.clone())),
            review_headers.clone(),
            Json(review_request),
        )
        .await
        .map_err(|(code, body)| format!("review replay failed: {code} {body:?}"))?;
        assert_eq!(replay, first, "exact review retry must converge");
        let receipt_count = usize::from(
            memory_test_ledger_event(
                &state,
                &format!("fems-memory-proposal-review:{}", proposal_ack.proposal_id),
            )
            .await?
            .is_some(),
        );
        assert_eq!(receipt_count, 1);

        let opposite = review_memory_proposal(
            State(state.clone()),
            Path((workspace_id.clone(), proposal_ack.proposal_id.clone())),
            review_headers,
            Json(ProposalReviewRequest {
                decision: ProposalReviewDecision::Rejected,
                reviewer_kind: ProposalReviewerKind::User,
                reason: Some("source checked".to_owned()),
            }),
        )
        .await
        .expect_err("opposite decision must conflict");
        assert_eq!(opposite.0, StatusCode::CONFLICT);
        assert_eq!(
            fems_memory::count_memory_items(&state.surreal, &workspace_id).await?,
            0,
            "review transition alone must not silently commit memory"
        );

        let rejected_content = "reject this durable fact";
        let rejected_source = create_test_rich_source(
            &state,
            &workspace_id,
            "rejected-review-source",
            rejected_content,
        )
        .await?;
        let rejected_scope = proposal_create_test_scope(
            &state,
            &workspace_id,
            &rejected_source.document_id,
            "proposal-review-create",
            "canonical-proposer",
        )
        .await?;
        let rejected_call = create_memory_proposal(
            State(state.clone()),
            Path(workspace_id.clone()),
            proposal_headers,
            Json(ProposalRequest {
                request_id: Some(format!("reject-request-{}", Uuid::now_v7())),
                class: ProposalClass::Episodic,
                content: rejected_content.to_owned(),
                source: rejected_source,
                source_document_content: None,
                review_gated: Some(true),
                actor_id: Some("proposer-2".to_owned()),
            }),
        );
        let Json(rejected_proposal) = state
            .surreal
            .with_record_user_scope(rejected_scope, rejected_call)
            .await
        .map_err(|(code, body)| format!("rejected proposal failed: {code} {body:?}"))?;
        let mut policy_headers = HeaderMap::new();
        policy_headers.insert(HSK_HEADER_ACTOR_ID, "policy-reviewer".parse()?);
        policy_headers.insert(HSK_HEADER_ACTOR_KIND, "policy".parse()?);
        let Json(rejected) = review_memory_proposal(
            State(state.clone()),
            Path((workspace_id.clone(), rejected_proposal.proposal_id.clone())),
            policy_headers,
            Json(ProposalReviewRequest {
                decision: ProposalReviewDecision::Rejected,
                reviewer_kind: ProposalReviewerKind::Policy,
                reason: None,
            }),
        )
        .await
        .map_err(|(code, body)| format!("reject review failed: {code} {body:?}"))?;
        assert_eq!(rejected.status, "rejected");
        let rejected_event = memory_test_ledger_event(
            &state,
            &format!(
                "fems-memory-proposal-review:{}",
                rejected_proposal.proposal_id
            ),
        )
        .await?
        .expect("rejected review receipt");
        assert_eq!(
            rejected_event.event_type,
            KernelEventType::PromotionRejected
        );
        let rejected_events = state
            .flight_recorder
            .list_events(EventFilter {
                event_id: Some(rejected.flight_recorder_event_id),
                ..EventFilter::default()
            })
            .await?;
        assert_eq!(rejected_events.len(), 1);
        assert_eq!(rejected_events[0].actor, FlightRecorderActor::System);
        assert_eq!(rejected_events[0].actor_id, "policy-reviewer");
        assert_eq!(rejected_events[0].payload["decision"], "rejected");
        assert_eq!(
            fems_memory::count_memory_items(&state.surreal, &workspace_id).await?,
            0
        );
            Ok::<(), Box<dyn std::error::Error>>(())
        }))
        .await;
        drop(state);
        finish_mt109_memory_test(body, store, data_dir).await
    }

    #[tokio::test]
    async fn commit_outbox_recovers_on_restart_and_preserves_original_evidence_across_later_commits(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (store, state) = setup_state().await?;
        let data_dir = store.data_dir.clone();
        let mut reconciler_task = None;
        let body = futures::FutureExt::catch_unwind(std::panic::AssertUnwindSafe(async {
        let workspace_id = create_test_workspace(&state, "commit-recovery").await?;
        let (proposal_a, review_a) = create_and_approve_test_proposal(
            &state,
            &workspace_id,
            "commit-recovery-owner",
            "first durable memory",
        )
        .await?;
        tokio::time::sleep(std::time::Duration::from_millis(2)).await;

        let durable_recorder = state.flight_recorder.clone();
        let mut commit_headers = HeaderMap::new();
        commit_headers.insert(HSK_HEADER_ACTOR_ID, "commit-operator".parse()?);
        commit_headers.insert(HSK_HEADER_ACTOR_KIND, "operator".parse()?);

        // Faithful crash simulation. A real crash durably commits the proposal transaction
        // (proposal -> committed plus the FR-EVT-MEM-003 commit and FR-EVT-MEM-004 pack-built
        // outbox rows, both inserted un-published) and then the process dies BEFORE the
        // in-request Flight Recorder projection runs. It does NOT execute the handler's
        // self-healing multi-pass reconcile, so the outbox rows are left genuinely un-published
        // and non-quarantined. Driving the storage-layer commit directly reproduces that durable
        // crash window; routing through commit_memory_proposal instead lets the same-request
        // reconcile self-heal the outbox, leaving nothing for the restart projector to recover.
        let crash_actor = canonical_actor_identity(&commit_headers, None, None)
            .map_err(|(code, body)| format!("crash actor identity failed: {code} {body:?}"))?;
        let crash_receipt = NewKernelEvent::builder(
            format!("native-editor-fems-commit-{workspace_id}"),
            "native-editor-session".to_owned(),
            KernelEventType::ArtifactStored,
            crash_actor.kernel_actor.clone(),
        )
        .aggregate("fems_memory_commit", proposal_a.proposal_id.clone())
        .idempotency_key(format!("fems-memory-commit:{}", proposal_a.proposal_id))
        .correlation_id(format!("fems-memory-proposal:{}", proposal_a.proposal_id))
        .source_component("fems_memory_proposal_commit")
        .payload(json!({"proposal_id": proposal_a.proposal_id}))
        .build()
        .map_err(|error| format!("crash receipt build failed: {error}"))?;
        fems_memory::commit_memory_proposal_with_receipt(
            &state.surreal,
            &workspace_id,
            &proposal_a.proposal_id,
            crash_receipt,
        )
        .await
        .map_err(|error| format!("crash-window storage commit failed: {error}"))?;

        let committed_status =
            fems_memory::get_memory_proposal(&state.surreal, &proposal_a.proposal_id)
                .await?
                .expect("committed proposal")
                .status;
        assert_eq!(committed_status, "committed");
        // WP-KERNEL-012 MT-146 D-146-3. SELECTOR CORRECTION ONLY, authorised by the Operator on
        // 2026-09-04 and recorded at lifecycle.operator_authorisation_2026_09_04. The expected
        // count and the assertion message below are deliberately unchanged.
        //
        // The previous selector filtered on payload["proposal_id"], which FR-EVT-MEM-004 cannot
        // carry: validate_memory_pack_built_payload enforces a closed key allowlist that omits
        // proposal_id and the builder validates, so the pack-built event could never match and
        // the assertion was unsatisfiable by construction rather than merely failing.
        //
        // The authorisation named activity_span_id, but required verifying first that BOTH
        // events carry it. They do not: only the pack builder sets activity_span_id, and
        // FlightRecorderEvent::new defaults it to None, so selecting on it alone would drop
        // FR-EVT-MEM-003 and be wrong in the opposite direction. trace_id is the linkage both
        // events genuinely share -- the commit event is built with
        // stable_uuid("fems-memory-proposal:{proposal_id}") as its trace_id and the pack event
        // copies that same value -- and it is scoped to this proposal, so it selects both
        // events of this commit and nothing else.
        let proposal_a_trace_id = deterministic_uuid_from_seed(&format!(
            "fems-memory-proposal:{}",
            proposal_a.proposal_id
        ));
        let pending_outbox =
            fems_memory::list_pending_memory_commit_events(&state.surreal, &workspace_id, 200)
                .await?
                .into_iter()
                .filter(|event| event.trace_id == proposal_a_trace_id)
                .count();
        assert_eq!(
            pending_outbox, 2,
            "crash leaves the commit (FR-EVT-MEM-003) and pack-built (FR-EVT-MEM-004) events durably un-published and non-quarantined"
        );
        let expected_event_id = deterministic_uuid_from_seed(&format!(
            "fems-memory-proposal-commit-event:{}",
            proposal_a.proposal_id
        ));
        assert!(durable_recorder
            .list_events(EventFilter {
                event_id: Some(expected_event_id),
                ..EventFilter::default()
            })
            .await?
            .is_empty());

        // Constructing the mounted routes is the production startup-owned projector hook. No memory
        // request is made after this point before the Flight Recorder row appears.
        state
            .surreal
            .provision_reconciliation_principal(std::slice::from_ref(&workspace_id), None)
            .await?;
        let restarted_state = AppState {
            storage: state.storage.clone(),
            flight_recorder: durable_recorder.clone(),
            diagnostics: state.diagnostics.clone(),
            llm_client: state.llm_client.clone(),
            capability_registry: state.capability_registry.clone(),
            session_registry: state.session_registry.clone(),
            surreal: state.surreal.clone(),
        };
        let (startup_routes, reconciler) = routes_with_reconciler(restarted_state.clone());
        reconciler_task = reconciler;
        let _startup_routes = startup_routes;
        reconciler_task
            .take()
            .expect("mounted routes start the one-shot memory reconciler")
            .await
            .map_err(|error| std::io::Error::other(format!(
                "startup memory reconciler join failed: {error}"
            )))?;
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
        let recovered_event = loop {
            let events = durable_recorder
                .list_events(EventFilter {
                    event_id: Some(expected_event_id),
                    ..EventFilter::default()
                })
                .await?;
            let published = memory_test_query_first::<MemoryTestBoolRow>(
                &state,
                "SELECT published_at != NONE AS value FROM fems_memory_commit_fr_outbox \
                 WHERE proposal_id = $proposal AND event_code = 'FR-EVT-MEM-003' LIMIT 1;",
                MemoryTestBindings {
                    proposal: Some(RecordId::new(
                        "fems_memory_proposals",
                        proposal_a.proposal_id.as_str(),
                    )),
                    ..MemoryTestBindings::default()
                },
            )
            .await?
            .expect("commit outbox row")
            .value;
            if let (Some(event), true) = (events.into_iter().next(), published) {
                break event;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "startup projector did not recover FR-EVT-MEM-003"
            );
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        };

        let commit_id = recovered_event.payload["commit_id"]
            .as_str()
            .expect("commit id")
            .to_owned();
        let Json(report_artifact) = get_memory_commit_report(
            State(restarted_state.clone()),
            Path((workspace_id.clone(), commit_id.clone())),
        )
        .await
        .map_err(|(status, body)| format!("report artifact failed: {status} {body:?}"))?;
        let artifact_hash = report_artifact.compute_hash()?;
        assert_eq!(
            artifact_hash,
            recovered_event.payload["commit_report_hash"]
                .as_str()
                .expect("report hash")
        );
        assert_eq!(
            recovered_event.payload["artifact_ref"],
            format!("artifact://sha256/{artifact_hash}")
        );
        let committed_at = chrono::DateTime::parse_from_rfc3339(&report_artifact.created_at)?
            .with_timezone(&chrono::Utc);
        assert!(
            committed_at > review_a.reviewed_at,
            "commit chronology must be later than review chronology"
        );

        let Json(first_a_retry) = commit_memory_proposal(
            State(restarted_state.clone()),
            Path((workspace_id.clone(), proposal_a.proposal_id.clone())),
            commit_headers.clone(),
        )
        .await
        .map_err(|(status, body)| format!("A retry failed: {status} {body:?}"))?;
        let (proposal_b, _) = create_and_approve_test_proposal(
            &restarted_state,
            &workspace_id,
            "commit-recovery-owner",
            "second durable memory",
        )
        .await?;
        let Json(commit_b) = commit_memory_proposal(
            State(restarted_state.clone()),
            Path((workspace_id.clone(), proposal_b.proposal_id)),
            commit_headers.clone(),
        )
        .await
        .map_err(|(status, body)| format!("B commit failed: {status} {body:?}"))?;
        let Json(second_a_retry) = commit_memory_proposal(
            State(restarted_state.clone()),
            Path((workspace_id.clone(), proposal_a.proposal_id.clone())),
            commit_headers,
        )
        .await
        .map_err(|(status, body)| format!("A-after-B retry failed: {status} {body:?}"))?;
        assert_eq!(first_a_retry.memory_pack_id, second_a_retry.memory_pack_id);
        assert_eq!(
            first_a_retry.memory_pack_hash,
            second_a_retry.memory_pack_hash
        );
        assert_ne!(
            first_a_retry.memory_pack_id, commit_b.memory_pack_id,
            "B advances the workspace pack without rewriting A's original pack evidence"
        );
        let receipt_count = usize::from(
            memory_test_ledger_event(
                &state,
                &format!("fems-memory-commit:{}", proposal_a.proposal_id),
            )
            .await?
            .is_some(),
        );
        assert_eq!(receipt_count, 1, "A retries keep one EventLedger receipt");
        Ok::<(), Box<dyn std::error::Error>>(())
        })).await;
        if let Some(reconciler) = reconciler_task {
            reconciler.abort();
            let _ = reconciler.await;
        }
        drop(state);
        finish_mt109_memory_test(body, store, data_dir).await
    }

    #[tokio::test]
    async fn proposal_outbox_retries_post_commit_recorder_failure_without_duplicate_event(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (store, state) = setup_state().await?;
        let data_dir = store.data_dir.clone();
        let body = futures::FutureExt::catch_unwind(std::panic::AssertUnwindSafe(async {
        let workspace_id = create_test_workspace(&state, "proposal-recovery").await?;
        let content = "proposal survives recorder crash";
        let source =
            create_test_rich_source(&state, &workspace_id, "proposal-recovery-source", content)
                .await?;
        let proposal_scope = proposal_create_test_scope(
            &state,
            &workspace_id,
            &source.document_id,
            "proposal-recovery",
            "native_editor",
        )
        .await?;
        let request_id = format!("proposal-recovery-{}", Uuid::now_v7());
        let proposal_id = stable_proposal_id(&workspace_id, &request_id);
        let durable_recorder = state.flight_recorder.clone();
        let failed_state = AppState {
            flight_recorder: Arc::new(FailNextRecordFlightRecorder {
                inner: durable_recorder.clone(),
                fail_next: AtomicBool::new(true),
            }),
            ..state.clone()
        };
        let failed_create = create_memory_proposal(
            State(failed_state),
            Path(workspace_id.clone()),
            HeaderMap::new(),
            Json(ProposalRequest {
                request_id: Some(request_id),
                class: ProposalClass::Semantic,
                content: content.to_owned(),
                source,
                source_document_content: None,
                review_gated: Some(true),
                actor_id: Some("spoofed-proposal-actor".to_owned()),
            }),
        );
        let failed = state
            .surreal
            .with_record_user_scope(proposal_scope, failed_create)
            .await
        .expect_err("injected recorder failure must fail the response honestly");
        eprintln!("MT109_PROPOSAL_OUTBOX_INJECTED_FAILURE={failed:?}");
        assert_eq!(failed.0, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(
            fems_memory::get_memory_proposal(&state.surreal, &proposal_id)
                .await?
                .is_some(),
            "proposal transaction remains authoritative after projection failure"
        );
        let pending =
            fems_memory::list_pending_memory_lifecycle_events(&state.surreal, &workspace_id, 200)
                .await?
                .into_iter()
                .filter(|event| event.payload["proposal_id"] == proposal_id)
                .count();
        assert_eq!(
            pending, 0,
            "the bounded reconciler retries the transient first failure in the same call"
        );
        let expected_event_id =
            deterministic_uuid_from_seed(&format!("fems-memory-proposal-event:{proposal_id}"));
        assert_eq!(
            durable_recorder
                .list_events(EventFilter {
                    event_id: Some(expected_event_id),
                    ..EventFilter::default()
                })
                .await?
                .len(),
            1,
            "the in-call retry publishes exactly one canonical event"
        );

        state
            .surreal
            .provision_reconciliation_principal(std::slice::from_ref(&workspace_id), None)
            .await?;
        reconcile_all_memory_commit_events(&state)
            .await
            .map_err(|(status, body)| format!("startup recovery failed: {status} {body:?}"))?;
        let recovered = durable_recorder
            .list_events(EventFilter {
                event_id: Some(expected_event_id),
                ..EventFilter::default()
            })
            .await?;
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].payload["event_code"], "FR-EVT-MEM-001");
        assert_eq!(recovered[0].payload["proposal_id"], proposal_id);
        reconcile_all_memory_commit_events(&state)
            .await
            .map_err(|(status, body)| format!("idempotent recovery failed: {status} {body:?}"))?;
        assert_eq!(
            durable_recorder
                .list_events(EventFilter {
                    event_id: Some(expected_event_id),
                    ..EventFilter::default()
                })
                .await?
                .len(),
            1,
            "repeated recovery never duplicates the canonical event"
        );
        Ok::<(), Box<dyn std::error::Error>>(())
        }))
        .await;
        drop(state);
        finish_mt109_memory_test(body, store, data_dir).await
    }

    #[tokio::test]
    async fn quarantined_proposal_outbox_never_returns_a_false_success_ack(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (store, state) = setup_state().await?;
        let data_dir = store.data_dir.clone();
        let body = futures::FutureExt::catch_unwind(std::panic::AssertUnwindSafe(async {
        let workspace_id = create_test_workspace(&state, "proposal-quarantine").await?;
        let content = "proposal whose recorder projection is quarantined";
        let source =
            create_test_rich_source(&state, &workspace_id, "proposal-quarantine-source", content)
                .await?;
        let proposal_scope = proposal_create_test_scope(
            &state,
            &workspace_id,
            &source.document_id,
            "proposal-quarantine",
            "native-process-a",
        )
        .await?;
        let request = ProposalRequest {
            request_id: None,
            class: ProposalClass::Semantic,
            content: content.to_owned(),
            source,
            source_document_content: None,
            review_gated: Some(true),
            actor_id: Some("spoofed-body-actor".to_owned()),
        };
        let request_id = stable_proposal_request_id(&workspace_id, &request)
            .map_err(|(status, body)| format!("derive request id failed: {status} {body:?}"))?;
        let proposal_id = stable_proposal_id(&workspace_id, &request_id);
        let mut first_process_headers = HeaderMap::new();
        first_process_headers.insert(HSK_HEADER_ACTOR_ID, "native-process-a".parse()?);
        first_process_headers.insert(HSK_HEADER_ACTOR_KIND, "operator".parse()?);
        let initial_call = create_memory_proposal(
            State(state.clone()),
            Path(workspace_id.clone()),
            first_process_headers,
            Json(request.clone()),
        );
        let Json(initial_ack) = state
            .surreal
            .with_record_user_scope(proposal_scope.clone(), initial_call)
            .await
        .map_err(|(status, body)| format!("canonical proposal setup failed: {status} {body:?}"))?;
        assert_eq!(initial_ack.proposal_id, proposal_id);

        let now = Datetime::from(chrono::Utc::now());
        let updated = memory_test_execute(
            &state,
            "UPDATE fems_memory_lifecycle_fr_outbox SET published_at = NONE, \
             attempt_count = 3, last_error = 'injected persistent projection failure', \
             last_error_at = $at, quarantined_at = $at \
             WHERE proposal_id = $proposal AND event_code = 'FR-EVT-MEM-001' RETURN AFTER;",
            MemoryTestBindings {
                proposal: Some(RecordId::new("fems_memory_proposals", proposal_id.as_str())),
                at: Some(now),
                ..MemoryTestBindings::default()
            },
        )
        .await?;
        assert_eq!(updated, 1, "the exact proposed-event row is quarantined");
        state
            .flight_recorder
            .delete_workspace_events(&workspace_id)
            .await?;

        let mut operator_headers = HeaderMap::new();
        operator_headers.insert(HSK_HEADER_ACTOR_ID, "quarantine-reviewer".parse()?);
        operator_headers.insert(HSK_HEADER_ACTOR_KIND, "operator".parse()?);
        let review = review_memory_proposal(
            State(state.clone()),
            Path((workspace_id.clone(), proposal_id.clone())),
            operator_headers.clone(),
            Json(ProposalReviewRequest {
                decision: ProposalReviewDecision::Approved,
                reviewer_kind: ProposalReviewerKind::User,
                reason: Some("must remain blocked without proposal event".to_owned()),
            }),
        )
        .await
        .expect_err("review must fail closed while FR-EVT-MEM-001 is quarantined");
        assert_eq!(review.0, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(review.1["error"], "proposal_event_not_published");

        let commit = commit_memory_proposal(
            State(state.clone()),
            Path((workspace_id.clone(), proposal_id.clone())),
            operator_headers,
        )
        .await
        .expect_err("commit must fail closed while FR-EVT-MEM-001 is quarantined");
        assert_eq!(commit.0, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(commit.1["error"], "proposal_event_not_published");
        let still_pending = fems_memory::get_memory_proposal(&state.surreal, &proposal_id)
            .await?
            .expect("quarantined proposal remains authoritative");
        assert_eq!(still_pending.status, PROPOSAL_STATUS_PENDING_REVIEW);
        assert_eq!(
            fems_memory::count_memory_items(&state.surreal, &workspace_id).await?,
            0,
            "neither review nor commit mutates memory behind an unpublished proposal event"
        );

        let persistently_failed_state = AppState {
            flight_recorder: Arc::new(FailAllRecordFlightRecorder {
                inner: state.flight_recorder.clone(),
            }),
            ..state.clone()
        };
        let mut restarted_process_headers = HeaderMap::new();
        restarted_process_headers.insert(HSK_HEADER_ACTOR_ID, "native-process-b".parse()?);
        restarted_process_headers.insert(HSK_HEADER_ACTOR_KIND, "operator".parse()?);
        let retry_call = create_memory_proposal(
            State(persistently_failed_state),
            Path(workspace_id.clone()),
            restarted_process_headers,
            Json(request.clone()),
        );
        let retry = state
            .surreal
            .with_record_user_scope(proposal_scope.clone(), retry_call)
            .await
        .expect_err("persistent recorder failure must never return a success acknowledgement");
        assert!(
            matches!(
                retry.0,
                StatusCode::INTERNAL_SERVER_ERROR | StatusCode::SERVICE_UNAVAILABLE
            ),
            "persistent projection failure returns an honest server error: {}",
            retry.0
        );
        assert!(
            state
                .flight_recorder
                .list_events(EventFilter {
                    event_id: Some(deterministic_uuid_from_seed(&format!(
                        "fems-memory-proposal-event:{proposal_id}"
                    ))),
                    ..EventFilter::default()
                })
                .await?
                .is_empty(),
            "no canonical recorder row exists behind the failed acknowledgement"
        );

        let publication_state = fems_memory::memory_lifecycle_publication_state(
            &state.surreal,
            &proposal_id,
            "FR-EVT-MEM-001",
        )
        .await?;
        assert_eq!(
            publication_state,
            fems_memory::MemoryLifecyclePublicationState::Quarantined,
            "the bounded persistent retry returns to an explicit quarantine state"
        );

        let mut healthy_restart_headers = HeaderMap::new();
        healthy_restart_headers.insert(HSK_HEADER_ACTOR_ID, "native-process-c".parse()?);
        healthy_restart_headers.insert(HSK_HEADER_ACTOR_KIND, "operator".parse()?);
        let recovered_call = create_memory_proposal(
            State(state.clone()),
            Path(workspace_id),
            healthy_restart_headers,
            Json(request),
        );
        let Json(recovered) = state
            .surreal
            .with_record_user_scope(proposal_scope, recovered_call)
            .await
        .map_err(|(status, body)| {
            format!("healthy identical resubmission failed to recover: {status} {body:?}")
        })?;
        assert_eq!(recovered.proposal_id, proposal_id);
        assert_eq!(
            fems_memory::memory_lifecycle_publication_state(
                &state.surreal,
                &proposal_id,
                "FR-EVT-MEM-001",
            )
            .await?,
            fems_memory::MemoryLifecyclePublicationState::Published,
            "identical resubmission explicitly requeues and publishes the exact quarantined event"
        );
        assert_eq!(
            state
                .flight_recorder
                .list_events(EventFilter {
                    event_id: Some(deterministic_uuid_from_seed(&format!(
                        "fems-memory-proposal-event:{proposal_id}"
                    ))),
                    ..EventFilter::default()
                })
                .await?
                .len(),
            1,
            "recovery publishes exactly one canonical FR-EVT-MEM-001"
        );
            Ok::<(), Box<dyn std::error::Error>>(())
        }))
        .await;
        drop(state);
        finish_mt109_memory_test(body, store, data_dir).await
    }
}
