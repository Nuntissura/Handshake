//! WP-1 MT-012: Operator chat/launch work-surface HTTP routes.
//!
//! This is the NEW backend launch seam the MT-012 pre-implementation review (F9)
//! requires: no prior `api/*` route reached [`SwarmCoordinator::spawn_session`]
//! (only read routes + the forbidden legacy Tauri IPC). The launch route resolves
//! ONLY through the sanctioned [`OperatorChatLaunchService`] (which calls
//! `spawn_session`) and fails closed when the coordinator has no `ModelLaneStore`.
//!
//! Like `model_access`, this router owns a dedicated state (not `AppState`) so it
//! is route-testable without a full `AppState`. The launch service (which needs a
//! live `SwarmCoordinator`) is injected via [`OperatorChatState::with_launch_service`];
//! when absent the launch route reports `503 launch_not_wired` (production wires
//! the live coordinator singleton — a follow-on to this MT).

use std::sync::Arc;

use axum::{
    extract::State,
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
    OperatorChatModelRow, OperatorChatSelection,
};

type ApiError = (StatusCode, Json<Value>);

/// Dedicated router state for the operator chat/launch surface.
#[derive(Clone)]
pub struct OperatorChatState {
    launch_service: Option<Arc<OperatorChatLaunchService>>,
    catalog: Arc<ModelCatalog>,
    cloud_registry: Arc<dyn ProviderAccessRegistry>,
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

    pub fn with_recorder(mut self, recorder: Arc<dyn FlightRecorder>) -> Self {
        self.recorder = recorder;
        self
    }
}

/// Selection-decision audit request (spec 4.3.9.4.4). Distinct from launch.
#[derive(Debug, Deserialize)]
pub struct SelectionDecisionRequest {
    pub selected_model_id: String,
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
                label: row.label.to_string(),
                status: status_label(row.status),
            })
            .collect(),
        cloud_cli_bridge: cloud
            .cli_bridge
            .into_iter()
            .map(|row| OperatorChatCloudRow {
                provider: row.provider.to_string(),
                label: row.label.to_string(),
                status: "offered".to_string(),
            })
            .collect(),
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
        .record_selection_decision(
            state.recorder.as_ref(),
            &req.selected_model_id,
            &req.actor,
            &req.reason,
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
    Ok(Json(serde_json::to_value(launched).unwrap_or_else(|_| json!({}))))
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
