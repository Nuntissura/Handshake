//! WP-1 MT-012: Operator chat/launch work-surface HTTP routes.
//!
//! This is the NEW backend launch seam the MT-012 pre-implementation review (F9)
//! requires: no prior `api/*` route reached the sanctioned coordinator launch
//! authority (only read routes + the forbidden legacy Tauri IPC). The launch route
//! resolves ONLY through [`OperatorChatLaunchService`]: process-backed lanes call
//! `SwarmCoordinator::spawn_session`, and SUBAGENT calls the coordinator no-OS
//! subagent lane helper. Both fail closed when the coordinator has no
//! `ModelLaneStore`.
//!
//! Like `model_access`, this router owns a dedicated state (not `AppState`) so it
//! is route-testable without a full `AppState`. The launch service (which needs a
//! live `SwarmCoordinator`) is injected via [`OperatorChatState::with_launch_service`];
//! when absent the launch route reports `503 launch_not_wired` (production wires
//! the live coordinator singleton — a follow-on to this MT).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::api::account_scope::RequestAccountScope;
use crate::flight_recorder::{EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError};
use crate::model_runtime::catalog::ModelCatalog;
use crate::model_runtime::cloud::{
    enumerate_cli_bridge_with_probe, enumerate_cloud_access, CliBridgeAuthStatus,
    CliBridgeAuthStatusProbe, CliBridgeProvider, InMemoryAccessRegistry, ProviderAccessRegistry,
    ProviderAccessStatus,
};
use crate::storage::ModelSessionState;
use crate::swarm_orchestration::operator_chat::{
    OperatorChatCloudRow, OperatorChatError, OperatorChatLaunchRequest, OperatorChatLaunchService,
    OperatorChatModelInventory, OperatorChatModelRow, OperatorChatRoutingAuthorityRequest,
    OperatorChatRoutingCancelRequest, OperatorChatRoutingLifecycleRequest, OperatorChatSessionRow,
    OperatorChatSingleRunCloudLaunchRequest, OperatorChatSingleRunCloudRevokeRequest,
    OperatorChatSubagentRow,
};
use crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution;
use crate::workflows::{SessionRegistry, SessionSchedulerConfig};

type ApiError = (StatusCode, Json<Value>);

/// Dedicated router state for the operator chat/launch surface.
#[derive(Clone)]
pub struct OperatorChatState {
    launch_service: Option<Arc<OperatorChatLaunchService>>,
    catalog: Arc<ModelCatalog>,
    cloud_registry: Arc<dyn ProviderAccessRegistry>,
    cli_auth_probe: Arc<dyn CliBridgeAuthStatusProbe>,
    cli_bridge_launchable_providers: BTreeSet<String>,
    recorder: Arc<dyn FlightRecorder>,
    session_registry: Arc<SessionRegistry>,
}

#[derive(Clone, Copy, Debug, Default)]
struct UnavailableOperatorChatCliAuthProbe;

impl CliBridgeAuthStatusProbe for UnavailableOperatorChatCliAuthProbe {
    fn auth_status(&self, _provider: CliBridgeProvider) -> CliBridgeAuthStatus {
        CliBridgeAuthStatus::Unavailable
    }
}

impl OperatorChatState {
    /// Production default: enumeration/selection serve from an empty catalog and
    /// an in-memory (all-unavailable) cloud registry until the app wires the live
    /// model registry, cloud vault, and coordinator singleton. Launch is not wired
    /// here (`503 launch_not_wired`) because it needs the live `SwarmCoordinator`.
    pub fn production() -> Self {
        Self {
            launch_service: None,
            catalog: ModelCatalog::empty(),
            cloud_registry: Arc::new(InMemoryAccessRegistry::new()),
            cli_auth_probe: Arc::new(UnavailableOperatorChatCliAuthProbe),
            cli_bridge_launchable_providers: BTreeSet::new(),
            recorder: Arc::new(NoopOperatorChatRecorder),
            session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
        }
    }

    pub fn with_launch_service(mut self, service: Arc<OperatorChatLaunchService>) -> Self {
        self.launch_service = Some(service);
        self
    }

    pub fn with_catalog(mut self, catalog: Arc<ModelCatalog>) -> Self {
        self.catalog = catalog;
        self
    }

    pub fn with_cloud_registry(mut self, registry: Arc<dyn ProviderAccessRegistry>) -> Self {
        self.cloud_registry = registry;
        self
    }

    pub fn with_cli_bridge_auth_probe(mut self, probe: Arc<dyn CliBridgeAuthStatusProbe>) -> Self {
        self.cli_auth_probe = probe;
        self
    }

    pub fn with_cli_bridge_launchable_providers(
        mut self,
        providers: impl IntoIterator<Item = String>,
    ) -> Self {
        self.cli_bridge_launchable_providers = providers.into_iter().collect();
        self
    }

    pub fn with_recorder(mut self, recorder: Arc<dyn FlightRecorder>) -> Self {
        self.recorder = recorder;
        self
    }

    pub fn with_session_registry(mut self, registry: Arc<SessionRegistry>) -> Self {
        self.session_registry = registry;
        self
    }
}

/// Selection-decision audit request (spec 4.3.9.4.4). Distinct from launch.
#[derive(Debug, Deserialize)]
pub struct SelectionDecisionRequest {
    pub selected_model_id: String,
    #[serde(default)]
    pub lane_kind: Option<String>,
    #[serde(default)]
    pub model_id: Option<String>,
    #[serde(default)]
    pub cloud_provider: Option<String>,
    #[serde(default)]
    pub cli_provider: Option<String>,
    #[serde(default)]
    pub working_dir: Option<String>,
    #[serde(default)]
    pub worktree_id: Option<String>,
    #[serde(default = "default_actor")]
    pub actor: String,
    #[serde(default = "default_reason")]
    pub reason: String,
}

fn default_actor() -> String {
    "operator".to_string()
}

fn default_reason() -> String {
    "operator_chat_launch_selection".to_string()
}

pub fn routes(state: OperatorChatState) -> Router {
    Router::new()
        .route("/operator-chat/models", get(enumerate_models))
        .route("/operator-chat/selection", post(record_selection))
        .route("/operator-chat/launch", post(launch_session))
        .route(
            "/operator-chat/cloud/single-run/grant-launch",
            post(launch_single_run_cloud_consent),
        )
        .route(
            "/operator-chat/cloud/single-run/revoke",
            post(revoke_single_run_cloud_consent),
        )
        .route("/operator-chat/transcript/:run_id", get(fetch_transcript))
        .route(
            "/operator-chat/routing/lifecycle",
            post(execute_routing_lifecycle),
        )
        .route(
            "/operator-chat/routing/recover",
            post(recover_routing_lifecycle),
        )
        .route(
            "/operator-chat/routing/authority",
            post(complete_routing_authority),
        )
        .route(
            "/operator-chat/routing/cancel",
            post(cancel_routing_lifecycle),
        )
        // WP-1 MT-021 AC-3: read/set the LIVE model-session concurrency cap.
        // Mounted here because the coordinator singleton is only reachable
        // through this module's launch service.
        .route(
            "/operator-chat/swarm/max-concurrent",
            get(get_swarm_max_concurrent).put(set_swarm_max_concurrent),
        )
        .with_state(state)
}

/// Product router: every resource-bearing operation first extracts the exact
/// server-owned scope. Optional HTTP scope headers can only assert equality;
/// missing server authority and mismatches fail before handler side effects.
pub fn scoped_routes(state: OperatorChatState) -> Router {
    Router::new()
        .route("/operator-chat/models", get(enumerate_models))
        .route("/operator-chat/selection", post(scoped_record_selection))
        .route("/operator-chat/launch", post(scoped_launch_session))
        .route(
            "/operator-chat/cloud/single-run/grant-launch",
            post(scoped_launch_single_run_cloud_consent),
        )
        .route(
            "/operator-chat/cloud/single-run/revoke",
            post(scoped_revoke_single_run_cloud_consent),
        )
        .route(
            "/operator-chat/transcript/:run_id",
            get(scoped_fetch_transcript),
        )
        .route(
            "/operator-chat/routing/lifecycle",
            post(scoped_execute_routing_lifecycle),
        )
        .route(
            "/operator-chat/routing/recover",
            post(scoped_recover_routing_lifecycle),
        )
        .route(
            "/operator-chat/routing/authority",
            post(scoped_complete_routing_authority),
        )
        .route(
            "/operator-chat/routing/cancel",
            post(scoped_cancel_routing_lifecycle),
        )
        .route(
            "/operator-chat/swarm/max-concurrent",
            get(scoped_get_swarm_max_concurrent).put(scoped_set_swarm_max_concurrent),
        )
        .with_state(state)
}

async fn scoped_get_swarm_max_concurrent(
    state: State<OperatorChatState>,
    _scope: RequestAccountScope,
) -> Result<Json<Value>, ApiError> {
    get_swarm_max_concurrent(state).await
}

async fn scoped_set_swarm_max_concurrent(
    state: State<OperatorChatState>,
    _scope: RequestAccountScope,
    body: Json<SetMaxConcurrentBody>,
) -> Result<Json<Value>, ApiError> {
    set_swarm_max_concurrent(state, body).await
}

async fn scoped_execute_routing_lifecycle(
    state: State<OperatorChatState>,
    _scope: RequestAccountScope,
    request: Json<OperatorChatRoutingLifecycleRequest>,
) -> Result<Json<Value>, ApiError> {
    execute_routing_lifecycle(state, request).await
}

async fn scoped_recover_routing_lifecycle(
    state: State<OperatorChatState>,
    _scope: RequestAccountScope,
    request: Json<OperatorChatRoutingLifecycleRequest>,
) -> Result<Json<Value>, ApiError> {
    recover_routing_lifecycle(state, request).await
}

async fn scoped_complete_routing_authority(
    state: State<OperatorChatState>,
    _scope: RequestAccountScope,
    request: Json<OperatorChatRoutingAuthorityRequest>,
) -> Result<Json<Value>, ApiError> {
    complete_routing_authority(state, request).await
}

async fn scoped_cancel_routing_lifecycle(
    state: State<OperatorChatState>,
    _scope: RequestAccountScope,
    request: Json<OperatorChatRoutingCancelRequest>,
) -> Result<Json<Value>, ApiError> {
    cancel_routing_lifecycle(state, request).await
}

async fn scoped_record_selection(
    State(state): State<OperatorChatState>,
    scope: RequestAccountScope,
    Json(request): Json<SelectionDecisionRequest>,
) -> Result<Json<Value>, ApiError> {
    let context = selection_context(&request);
    let scoped_recorder = ExactSelectionRecorder {
        inner: state.recorder.as_ref(),
        scope: scope.exact(),
    };
    state
        .catalog
        .record_selection_decision_with_context(
            &scoped_recorder,
            &request.selected_model_id,
            &request.actor,
            &request.reason,
            context,
        )
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "selection_audit_failed", "detail": error.to_string()})),
            )
        })?;
    Ok(Json(json!({
        "status": "recorded",
        "selected_model_id": request.selected_model_id,
    })))
}

struct ExactSelectionRecorder<'a> {
    inner: &'a dyn FlightRecorder,
    scope: &'a ExactResourceScopeAttribution,
}

#[async_trait]
impl FlightRecorder for ExactSelectionRecorder<'_> {
    async fn record_event(&self, mut event: FlightRecorderEvent) -> Result<(), RecorderError> {
        self.scope
            .stamp_json_object(&mut event.payload)
            .map_err(|error| RecorderError::SinkError(error.to_string()))?;
        self.inner.record_event(event).await
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
}

async fn scoped_launch_session(
    state: State<OperatorChatState>,
    _scope: RequestAccountScope,
    request: Json<OperatorChatLaunchRequest>,
) -> Result<Json<Value>, ApiError> {
    launch_session(state, request).await
}

async fn scoped_launch_single_run_cloud_consent(
    state: State<OperatorChatState>,
    scope: RequestAccountScope,
    request: Json<OperatorChatSingleRunCloudLaunchRequest>,
) -> Result<Json<Value>, ApiError> {
    let service = routing_service(&state)?;
    let launched = service
        .launch_single_run_cloud_consent_scoped(request.0, scope.exact())
        .await
        .map_err(launch_api_error)?;
    Ok(Json(
        serde_json::to_value(launched).unwrap_or_else(|_| json!({})),
    ))
}

async fn scoped_revoke_single_run_cloud_consent(
    state: State<OperatorChatState>,
    scope: RequestAccountScope,
    request: Json<OperatorChatSingleRunCloudRevokeRequest>,
) -> Result<Json<Value>, ApiError> {
    let service = routing_service(&state)?;
    let cancelled = service
        .revoke_single_run_cloud_consent_scoped(
            &request.consent_receipt_id,
            &request.revoked_by_ref,
            &request.reason,
            scope.exact(),
        )
        .await
        .map_err(launch_api_error)?;
    Ok(Json(json!({
        "consent_receipt_id": request.consent_receipt_id,
        "cancelled_lanes": cancelled,
    })))
}

async fn scoped_fetch_transcript(
    state: State<OperatorChatState>,
    _scope: RequestAccountScope,
    run_id: Path<String>,
) -> Result<Json<Value>, ApiError> {
    fetch_transcript(state, run_id).await
}

/// Body for `PUT /operator-chat/swarm/max-concurrent`.
#[derive(serde::Deserialize)]
struct SetMaxConcurrentBody {
    max_concurrent: usize,
}

/// `GET /operator-chat/swarm/max-concurrent` - the cap currently IN FORCE.
///
/// Reports the live semaphore cap, not the value the coordinator was built with,
/// so an operator surface can never display a limit the runtime is not honouring
/// (the misleading-control defect MT-021 AC-3 exists to remove).
async fn get_swarm_max_concurrent(
    State(state): State<OperatorChatState>,
) -> Result<Json<Value>, ApiError> {
    let service = routing_service(&state)?;
    let coordinator = service.coordinator();
    let in_force = coordinator.max_concurrent();
    let requested = coordinator.requested_max_concurrent();
    Ok(Json(json!({
        "max_concurrent": in_force,
        // A lowering that is still draining is a real state the operator can sit
        // in, so GET has to be able to express it. Without these two fields a
        // reader cannot tell "the cap is 3" from "the cap is 3 on its way to 1".
        "requested": requested,
        "fully_applied": in_force == requested,
        "live_sessions": coordinator.live_session_count(),
    })))
}

/// `PUT /operator-chat/swarm/max-concurrent` - change the live cap.
///
/// Returns the cap ACTUALLY in force after the change, which may be higher than
/// requested when lowering: reducing the cap is cooperative, reclaiming only
/// permits that are free right now and retiring the rest as running sessions
/// finish. Model sessions already admitted are never killed to satisfy a
/// settings change - that would destroy operator work and orphan processes
/// (HBR-QUIET-003). Reporting the requested number instead of the enforced one
/// would recreate exactly the lie this route exists to remove.
async fn set_swarm_max_concurrent(
    State(state): State<OperatorChatState>,
    Json(body): Json<SetMaxConcurrentBody>,
) -> Result<Json<Value>, ApiError> {
    let service = routing_service(&state)?;
    let coordinator = service.coordinator();
    let in_force = coordinator.set_max_concurrent(body.max_concurrent);
    // Report the CLAMPED target, not the raw body: a request of 0 is held at 1,
    // and echoing the 0 back would make `fully_applied` false forever.
    let requested = coordinator.requested_max_concurrent();
    Ok(Json(json!({
        "requested": requested,
        "max_concurrent": in_force,
        "fully_applied": in_force == requested,
        "live_sessions": coordinator.live_session_count(),
    })))
}

fn routing_service(state: &OperatorChatState) -> Result<Arc<OperatorChatLaunchService>, ApiError> {
    state.launch_service.clone().ok_or_else(|| (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({"error":"routing_not_wired","detail":"operator-chat routing requires the production coordinator"})),
    ))
}

async fn execute_routing_lifecycle(
    State(state): State<OperatorChatState>,
    Json(request): Json<OperatorChatRoutingLifecycleRequest>,
) -> Result<Json<Value>, ApiError> {
    let batch = routing_service(&state)?
        .execute_routing_lifecycle(request)
        .await
        .map_err(launch_api_error)?;
    Ok(Json(
        serde_json::to_value(batch).unwrap_or_else(|_| json!({})),
    ))
}

async fn recover_routing_lifecycle(
    State(state): State<OperatorChatState>,
    Json(request): Json<OperatorChatRoutingLifecycleRequest>,
) -> Result<Json<Value>, ApiError> {
    let batch = routing_service(&state)?
        .recover_routing_lifecycle(request)
        .await
        .map_err(launch_api_error)?;
    Ok(Json(
        serde_json::to_value(batch).unwrap_or_else(|_| json!({})),
    ))
}

async fn complete_routing_authority(
    State(state): State<OperatorChatState>,
    Json(request): Json<OperatorChatRoutingAuthorityRequest>,
) -> Result<Json<Value>, ApiError> {
    let batch = routing_service(&state)?
        .complete_routing_authority(request)
        .await
        .map_err(launch_api_error)?;
    Ok(Json(
        serde_json::to_value(batch).unwrap_or_else(|_| json!({})),
    ))
}

async fn cancel_routing_lifecycle(
    State(state): State<OperatorChatState>,
    Json(request): Json<OperatorChatRoutingCancelRequest>,
) -> Result<Json<Value>, ApiError> {
    let execution = routing_service(&state)?
        .cancel_routing_lifecycle(request)
        .await
        .map_err(launch_api_error)?;
    Ok(Json(
        serde_json::to_value(execution).unwrap_or_else(|_| json!({})),
    ))
}

/// GET the non-secret picker inventory: local models (MT-014) + cloud rows (MT-015).
/// Enumeration needs no launch authority; it reads the catalog + cloud registry.
async fn enumerate_models(State(state): State<OperatorChatState>) -> Json<Value> {
    let inventory = enumerate_inventory(&state).await;
    Json(serde_json::to_value(inventory).unwrap_or_else(|_| json!({})))
}

async fn enumerate_inventory(state: &OperatorChatState) -> OperatorChatModelInventory {
    let local = state
        .catalog
        .list()
        .into_iter()
        .filter(|entry| entry.default_selectable)
        .map(|entry| OperatorChatModelRow {
            model_id: entry.model_id,
            display_name: entry.display_name,
            runtime_binding: entry.runtime_binding,
            ready: entry.ready,
        })
        .collect();
    let cloud = enumerate_cloud_access(state.cloud_registry.as_ref());
    let cli_auth_probe = state.cli_auth_probe.clone();
    let cli_bridge_launchable_providers = state.cli_bridge_launchable_providers.clone();
    let cli_bridge_statuses: BTreeMap<String, CliBridgeAuthStatus> =
        tokio::task::spawn_blocking(move || {
            enumerate_cli_bridge_with_probe(cli_auth_probe.as_ref())
                .into_iter()
                .map(|row| (row.provider.to_string(), row.auth_status))
                .collect()
        })
        .await
        .unwrap_or_else(|_| {
            CliBridgeProvider::OFFERED
                .into_iter()
                .map(|provider| (provider.id().to_string(), CliBridgeAuthStatus::Unavailable))
                .collect()
        });
    let status_label = |status: ProviderAccessStatus| match status {
        ProviderAccessStatus::Configured => "configured".to_string(),
        ProviderAccessStatus::Unavailable => "unavailable".to_string(),
    };
    let cli_auth_status_label = |status: CliBridgeAuthStatus| match status {
        CliBridgeAuthStatus::LoggedIn => "logged_in".to_string(),
        CliBridgeAuthStatus::LoggedOut => "logged_out".to_string(),
        CliBridgeAuthStatus::Expired => "expired".to_string(),
        CliBridgeAuthStatus::Unavailable => "unavailable".to_string(),
    };
    let sessions = state.session_registry.snapshot().await;
    let mut session_rows = sessions
        .active_sessions
        .values()
        .map(|session| {
            let parent_active = session
                .parent_session_id
                .as_ref()
                .and_then(|parent| sessions.active_sessions.get(parent))
                .is_some_and(|parent| {
                    parent.state == ModelSessionState::Active
                        && session.spawn_depth == parent.spawn_depth.saturating_add(1)
                });
            OperatorChatSessionRow {
                session_id: session.session_id.clone(),
                parent_session_id: session.parent_session_id.clone(),
                label: format!("{} / {}", session.role, session.model_id),
                status: if session.state == ModelSessionState::Active && parent_active {
                    "available"
                } else {
                    "unavailable"
                }
                .to_string(),
            }
        })
        .collect::<Vec<_>>();
    session_rows.sort_by(|left, right| left.session_id.cmp(&right.session_id));
    OperatorChatModelInventory {
        inventory_source: "operator_chat_backend",
        sessions: session_rows,
        local,
        cloud_byok: cloud
            .byok
            .into_iter()
            .map(|row| OperatorChatCloudRow {
                provider: row.provider.to_string(),
                model_id: match row.provider {
                    "anthropic" => "claude-sonnet-4",
                    "openai" => "gpt-4o",
                    _ => "cloud-model",
                }
                .to_string(),
                label: row.label.to_string(),
                status: status_label(row.status),
            })
            .collect(),
        cloud_cli_bridge: cloud
            .cli_bridge
            .into_iter()
            .map(|row| OperatorChatCloudRow {
                provider: row.provider.to_string(),
                model_id: match row.provider {
                    "claude_code" => "claude-sonnet-4",
                    "codex" => "gpt-5-codex",
                    _ => "official-cli-model",
                }
                .to_string(),
                label: row.label.to_string(),
                status: cli_auth_status_label(
                    if cli_bridge_launchable_providers.contains(row.provider) {
                        cli_bridge_statuses
                            .get(row.provider)
                            .copied()
                            .unwrap_or(CliBridgeAuthStatus::Unavailable)
                    } else {
                        CliBridgeAuthStatus::Unavailable
                    },
                ),
            })
            .collect(),
        subagents: vec![OperatorChatSubagentRow {
            role: "subagent_coder".to_string(),
            model_id: "subagent://operator-chat/coder".to_string(),
            label: "Subagent Manager / Coder".to_string(),
            status: "available".to_string(),
        }],
        excluded: cloud.excluded.into_iter().map(|s| s.to_string()).collect(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedOperatorChatLineage {
    pub owner_session_id: String,
    pub parent_session_id: String,
}

pub async fn resolve_operator_chat_lineage(
    registry: &SessionRegistry,
    owner_session_id: &str,
) -> Result<ResolvedOperatorChatLineage, &'static str> {
    let owner_session_id = owner_session_id.trim();
    if owner_session_id.is_empty() {
        return Err("owner_session_id_required");
    }
    let snapshot = registry.snapshot().await;
    let owner = snapshot
        .active_sessions
        .get(owner_session_id)
        .ok_or("owner_session_not_registered")?;
    if owner.state != ModelSessionState::Active {
        return Err("owner_session_not_active");
    }
    let parent_session_id = owner
        .parent_session_id
        .as_deref()
        .filter(|parent| !parent.trim().is_empty())
        .ok_or("owner_session_lineage_missing")?;
    let parent = snapshot
        .active_sessions
        .get(parent_session_id)
        .ok_or("parent_session_not_registered")?;
    if parent.state != ModelSessionState::Active {
        return Err("parent_session_not_active");
    }
    if owner.spawn_depth != parent.spawn_depth.saturating_add(1) {
        return Err("owner_session_lineage_invalid");
    }
    Ok(ResolvedOperatorChatLineage {
        owner_session_id: owner.session_id.clone(),
        parent_session_id: parent.session_id.clone(),
    })
}

/// POST the selection-decision audit event (wires MT-014 record_selection_decision).
async fn record_selection(
    State(state): State<OperatorChatState>,
    Json(req): Json<SelectionDecisionRequest>,
) -> Result<Json<Value>, ApiError> {
    state
        .catalog
        .record_selection_decision_with_context(
            state.recorder.as_ref(),
            &req.selected_model_id,
            &req.actor,
            &req.reason,
            selection_context(&req),
        )
        .await
        .map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "selection_audit_failed", "detail": err.to_string()})),
            )
        })?;
    Ok(Json(json!({
        "status": "recorded",
        "selected_model_id": req.selected_model_id,
    })))
}

fn selection_context(req: &SelectionDecisionRequest) -> Value {
    let mut context = serde_json::Map::new();
    push_selection_context(&mut context, "lane_kind", req.lane_kind.as_deref());
    push_selection_context(&mut context, "model_id", req.model_id.as_deref());
    push_selection_context(
        &mut context,
        "cloud_provider",
        req.cloud_provider.as_deref(),
    );
    push_selection_context(&mut context, "cli_provider", req.cli_provider.as_deref());
    push_selection_context(&mut context, "working_dir", req.working_dir.as_deref());
    push_selection_context(&mut context, "worktree_id", req.worktree_id.as_deref());
    Value::Object(context)
}

fn push_selection_context(
    context: &mut serde_json::Map<String, Value>,
    key: &str,
    value: Option<&str>,
) {
    if let Some(value) = value.filter(|v| !v.trim().is_empty()) {
        context.insert(key.to_string(), Value::String(value.to_string()));
    }
}

/// POST an operator launch. Resolves ONLY through the sanctioned launch service
/// (`spawn_session`). Fails closed (non-2xx) when the coordinator has no
/// ModelLaneStore, or `503` when the launch service is not wired.
async fn launch_session(
    State(state): State<OperatorChatState>,
    Json(request): Json<OperatorChatLaunchRequest>,
) -> Result<Json<Value>, ApiError> {
    let Some(service) = state.launch_service.clone() else {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "error": "launch_not_wired",
                "detail": "operator-chat launch requires a live SwarmCoordinator; not wired in this deployment",
            })),
        ));
    };
    let lineage = resolve_operator_chat_lineage(&state.session_registry, &request.owner_session_id)
        .await
        .map_err(|code| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "invalid_owner_session", "code": code})),
            )
        })?;
    let selection =
        request.into_governed_selection(lineage.owner_session_id, lineage.parent_session_id);
    let launched = service.launch(&selection).await.map_err(launch_api_error)?;
    Ok(Json(
        serde_json::to_value(launched).unwrap_or_else(|_| json!({})),
    ))
}

async fn launch_single_run_cloud_consent(
    State(_state): State<OperatorChatState>,
    Json(_request): Json<OperatorChatSingleRunCloudLaunchRequest>,
) -> Result<Json<Value>, ApiError> {
    Err((
        StatusCode::FORBIDDEN,
        Json(json!({
            "error": "resource_scope_unavailable",
            "detail": "operator-chat cloud launch requires the scoped product router"
        })),
    ))
}

async fn revoke_single_run_cloud_consent(
    State(_state): State<OperatorChatState>,
    Json(_request): Json<OperatorChatSingleRunCloudRevokeRequest>,
) -> Result<Json<Value>, ApiError> {
    Err((
        StatusCode::FORBIDDEN,
        Json(json!({
            "error": "resource_scope_unavailable",
            "detail": "operator-chat cloud revocation requires the scoped product router"
        })),
    ))
}

/// GET the captured transcript (ModelLaneMessage rows) for a launched run so the
/// pane can render the conversation/thought/tool-call turns (F8). Reuses the
/// launch service's `ModelLaneStore` (EventLedger authority); reports `503` when
/// the launch service (and therefore the store) is not wired.
async fn fetch_transcript(
    State(state): State<OperatorChatState>,
    Path(run_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let Some(service) = state.launch_service.clone() else {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "error": "transcript_not_wired",
                "detail": "operator-chat transcript requires a live ModelLaneStore; not wired in this deployment",
            })),
        ));
    };
    let rows = service
        .fetch_transcript(&run_id)
        .await
        .map_err(launch_api_error)?;
    Ok(Json(json!({
        "run_id": run_id,
        "rows": serde_json::to_value(rows).unwrap_or_else(|_| json!([])),
    })))
}

fn launch_api_error(err: OperatorChatError) -> ApiError {
    match err {
        OperatorChatError::Invalid(detail) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "bad_request", "detail": detail})),
        ),
        // Fail-closed launches (absent ModelLaneStore / bypass) surface here as a
        // SwarmError::LedgerFailed — a hard 500, never a partial success.
        OperatorChatError::Swarm(swarm) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "launch_failed_closed", "detail": swarm.to_string()})),
        ),
        OperatorChatError::ModelLane(model_lane) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "model_lane_error", "detail": model_lane.to_string()})),
        ),
        OperatorChatError::Recorder(rec) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "recorder_error", "detail": rec.to_string()})),
        ),
    }
}

/// Minimal no-op recorder for the `production()` default so enumeration/selection
/// routes compile without the app's real Flight Recorder wired. Tests inject a
/// capturing recorder to PROVE the selection-decision event.
#[derive(Debug, Default)]
struct NoopOperatorChatRecorder;

#[async_trait::async_trait]
impl FlightRecorder for NoopOperatorChatRecorder {
    async fn record_event(&self, _event: FlightRecorderEvent) -> Result<(), RecorderError> {
        Ok(())
    }

    async fn enforce_retention(&self) -> Result<u64, RecorderError> {
        Ok(0)
    }

    async fn list_events(
        &self,
        _filter: EventFilter,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        Ok(Vec::new())
    }
}
