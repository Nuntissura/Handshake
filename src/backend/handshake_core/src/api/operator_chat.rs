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

use std::collections::BTreeMap;
use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::flight_recorder::{EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError};
use crate::model_runtime::catalog::ModelCatalog;
use crate::model_runtime::cloud::{
    enumerate_cloud_access, InMemoryAccessRegistry, ProviderAccessRegistry, ProviderAccessStatus,
};
use crate::swarm_orchestration::operator_chat::{
    OperatorChatCloudRow, OperatorChatError, OperatorChatLaunchService, OperatorChatModelInventory,
    OperatorChatModelRow, OperatorChatSelection, OperatorChatSubagentRow,
};

type ApiError = (StatusCode, Json<Value>);

/// Dedicated router state for the operator chat/launch surface.
#[derive(Clone)]
pub struct OperatorChatState {
    launch_service: Option<Arc<OperatorChatLaunchService>>,
    catalog: Arc<ModelCatalog>,
    cloud_registry: Arc<dyn ProviderAccessRegistry>,
    cli_bridge_statuses: BTreeMap<String, ProviderAccessStatus>,
    recorder: Arc<dyn FlightRecorder>,
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
            cli_bridge_statuses: BTreeMap::new(),
            recorder: Arc::new(NoopOperatorChatRecorder),
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

    pub fn with_cli_bridge_provider_status(
        mut self,
        provider: impl Into<String>,
        status: ProviderAccessStatus,
    ) -> Self {
        self.cli_bridge_statuses.insert(provider.into(), status);
        self
    }

    pub fn with_recorder(mut self, recorder: Arc<dyn FlightRecorder>) -> Self {
        self.recorder = recorder;
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
        .route("/operator-chat/transcript/:run_id", get(fetch_transcript))
        .with_state(state)
}

/// GET the non-secret picker inventory: local models (MT-014) + cloud rows (MT-015).
/// Enumeration needs no launch authority; it reads the catalog + cloud registry.
async fn enumerate_models(State(state): State<OperatorChatState>) -> Json<Value> {
    let inventory = enumerate_inventory(&state);
    Json(serde_json::to_value(inventory).unwrap_or_else(|_| json!({})))
}

fn enumerate_inventory(state: &OperatorChatState) -> OperatorChatModelInventory {
    let local = state
        .catalog
        .list()
        .into_iter()
        .map(|entry| OperatorChatModelRow {
            model_id: entry.model_id,
            display_name: entry.display_name,
            runtime_binding: entry.runtime_binding,
            ready: entry.ready,
        })
        .collect();
    let cloud = enumerate_cloud_access(state.cloud_registry.as_ref());
    let status_label = |status: ProviderAccessStatus| match status {
        ProviderAccessStatus::Configured => "configured".to_string(),
        ProviderAccessStatus::Unavailable => "unavailable".to_string(),
    };
    OperatorChatModelInventory {
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
                status: status_label(
                    state
                        .cli_bridge_statuses
                        .get(row.provider)
                        .copied()
                        .unwrap_or(ProviderAccessStatus::Unavailable),
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
    Json(selection): Json<OperatorChatSelection>,
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
    let launched = service.launch(&selection).await.map_err(launch_api_error)?;
    Ok(Json(
        serde_json::to_value(launched).unwrap_or_else(|_| json!({})),
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
