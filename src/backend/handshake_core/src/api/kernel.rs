use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::flight_recorder::{EventFilter, FlightRecorderEvent, FlightRecorderEventType};
use crate::kernel::product_screenshot_capture::{
    capture_product_screenshot_from_browser_adapter, ProductScreenshotArtifactV1,
    ProductScreenshotBrowserAdapterConfigV1, ProductScreenshotDurableReceiptV1,
    ProductScreenshotExecutionProofV1, ProductScreenshotExecutionReceiptV1,
    ProductScreenshotRequestV1,
};
use crate::kernel::session_spawn_tree_dcc::{
    project_session_spawn_tree_dcc, SessionAnnounceBackBadgeV1, SessionRuntimeState,
    SessionSpawnMode, SessionSpawnRuntimeRecordV1, SessionSpawnTreeDccV1,
};
use crate::kernel::{
    action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
    build_pre_use_dcc_mvp_runtime_surface,
    dcc_mvp_runtime_surface::preview_dcc_governed_action,
    dcc_mvp_runtime_surface::validate_dcc_mvp_runtime_surface,
    DccMvpRuntimeSurfaceV1, KernelError, KernelTraceInspector, TraceProjection,
};
use crate::storage::{ModelSession, ModelSessionState};
use crate::swarm_orchestration::state_recovery::{
    validate_swarm_dashboard_projection, AgentLaneIdentity, AgentLaneKind, AttributionMode,
    LocalCloudAttribution, ParallelSwarmDashboardProjectionV1, ParallelSwarmStateRecoveryStore,
    StateRecoveryError, SwarmDashboardProjectionRequest,
};
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct TraceProjectionQuery {
    pub kernel_task_run_id: String,
    pub session_run_id: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub code: &'static str,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct DccGovernedActionTriggerRequest {
    pub work_id: String,
    pub action_id: String,
    pub approval_preview_id: Option<String>,
    pub same_turn_approval: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct DccGovernedActionTriggerResponse {
    pub schema_id: &'static str,
    pub work_id: String,
    pub action_id: String,
    pub triggered: bool,
    pub catalog_checked: bool,
    pub preview_checked: bool,
    pub gate_enforced: bool,
    pub approval_preview_id: Option<String>,
    pub authority_effect: crate::kernel::action_envelope::AuthorityEffect,
    pub approval_posture: crate::kernel::action_envelope::ApprovalPosture,
    pub expected_write_box_kinds: Vec<String>,
    pub receipt_ref: String,
}

#[derive(Debug, Deserialize)]
pub struct ProductScreenshotCaptureExecuteRequest {
    pub request: ProductScreenshotRequestV1,
    pub source_url: String,
    pub adapter_script_path: Option<String>,
    pub node_binary: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ProductScreenshotCaptureExecuteResponse {
    pub schema_id: &'static str,
    pub artifact: ProductScreenshotArtifactV1,
    pub durable_receipt: ProductScreenshotDurableReceiptV1,
    pub proof: ProductScreenshotExecutionProofV1,
    pub receipt: ProductScreenshotExecutionReceiptV1,
}

#[derive(Debug, Deserialize)]
pub struct ParallelSwarmDashboardProjectionQuery {
    pub workspace_id: String,
    pub wp_id: Option<String>,
    pub mt_id: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct SessionSpawnTreeNodeApiProjection {
    pub session_id: String,
    pub parent_session_id: Option<String>,
    pub role_id: String,
    pub depth: usize,
    pub child_count: usize,
    pub active_child_count: usize,
    pub spawn_mode: String,
    pub runtime_state: String,
    pub cascade_cancel_available: bool,
    pub announce_back_badges: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SessionSpawnTreeDccApiProjection {
    pub schema_id: String,
    pub tree_id: String,
    pub panel_id: String,
    pub visible_fields: Vec<String>,
    pub nodes: Vec<SessionSpawnTreeNodeApiProjection>,
    pub max_depth: usize,
    pub cascade_cancel_session_ids: Vec<String>,
    pub announce_back_badge_count: usize,
    pub runtime_record_refs: Vec<String>,
    pub mutates_runtime_records: bool,
}

#[derive(Debug, Serialize)]
pub struct KernelDccProjectionApiSurface {
    #[serde(flatten)]
    pub surface: DccMvpRuntimeSurfaceV1,
    pub session_spawn_runtime_records: Vec<SessionSpawnRuntimeRecordV1>,
}

type ApiResult<T> = Result<Json<T>, (StatusCode, Json<ErrorResponse>)>;

fn api_error(
    status: StatusCode,
    code: &'static str,
    message: impl Into<String>,
) -> (StatusCode, Json<ErrorResponse>) {
    (
        status,
        Json(ErrorResponse {
            code,
            message: message.into(),
        }),
    )
}

fn map_kernel_error(err: KernelError) -> (StatusCode, Json<ErrorResponse>) {
    match err {
        KernelError::InvalidEvent(_)
        | KernelError::InvalidEventType(_)
        | KernelError::InvalidSessionTransition { .. } => api_error(
            StatusCode::BAD_REQUEST,
            "kernel_trace_invalid",
            err.to_string(),
        ),
        _ => api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "kernel_trace_inspection_failed",
            err.to_string(),
        ),
    }
}

fn map_state_recovery_error(err: StateRecoveryError) -> (StatusCode, Json<ErrorResponse>) {
    match err {
        StateRecoveryError::InvalidInput(message) => api_error(
            StatusCode::BAD_REQUEST,
            "parallel_swarm_dashboard_invalid_request",
            message,
        ),
        other => api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "parallel_swarm_dashboard_projection_failed",
            other.to_string(),
        ),
    }
}

pub async fn inspect_trace_projection(
    State(state): State<AppState>,
    Query(query): Query<TraceProjectionQuery>,
) -> ApiResult<TraceProjection> {
    if query.kernel_task_run_id.trim().is_empty() || query.session_run_id.trim().is_empty() {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "kernel_trace_missing_ids",
            "kernel_task_run_id and session_run_id are required",
        ));
    }

    let projection = KernelTraceInspector::new(state.storage.clone())
        .inspect_session(&query.kernel_task_run_id, &query.session_run_id)
        .await
        .map_err(map_kernel_error)?;
    Ok(Json(projection))
}

pub async fn dcc_projection(
    State(state): State<AppState>,
) -> ApiResult<KernelDccProjectionApiSurface> {
    let surface = build_pre_use_dcc_mvp_runtime_surface();
    validate_dcc_mvp_runtime_surface(&surface).map_err(|errors| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "kernel_dcc_projection_invalid",
            format!("backend DCC projection failed validation: {errors:?}"),
        )
    })?;
    let session_spawn_runtime_records = session_spawn_runtime_records_from_state(&state).await;
    Ok(Json(KernelDccProjectionApiSurface {
        surface,
        session_spawn_runtime_records,
    }))
}

pub async fn parallel_swarm_dashboard_projection(
    State(state): State<AppState>,
    Query(query): Query<ParallelSwarmDashboardProjectionQuery>,
) -> ApiResult<ParallelSwarmDashboardProjectionV1> {
    let lane = AgentLaneIdentity::new(
        "lane-parallel-swarm-dashboard-api",
        "parallel-swarm-dashboard-api",
        AgentLaneKind::Validator,
        LocalCloudAttribution {
            mode: AttributionMode::System,
            provider: None,
            runtime: Some("handshake_core_api".to_string()),
            model_label: "parallel-swarm-dashboard-projection".to_string(),
            credential_ref: None,
            provider_metadata: serde_json::json!({}),
        },
    )
    .map_err(map_state_recovery_error)?;
    let store =
        ParallelSwarmStateRecoveryStore::new(state.postgres_pool.clone(), state.storage.clone());
    let projection = store
        .project_swarm_dashboard(SwarmDashboardProjectionRequest {
            lane,
            workspace_id: query.workspace_id,
            wp_id: query.wp_id,
            mt_id: query.mt_id,
            limit: query.limit.unwrap_or(100),
        })
        .await
        .map_err(map_state_recovery_error)?;
    validate_swarm_dashboard_projection(&projection).map_err(|errors| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "parallel_swarm_dashboard_projection_invalid",
            format!("parallel swarm dashboard projection failed validation: {errors:?}"),
        )
    })?;
    Ok(Json(projection))
}

pub async fn trigger_dcc_governed_action(
    Json(request): Json<DccGovernedActionTriggerRequest>,
) -> ApiResult<DccGovernedActionTriggerResponse> {
    let work_id = request.work_id.trim();
    let action_id = request.action_id.trim();
    if work_id.is_empty() || action_id.is_empty() {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "kernel_dcc_action_missing_ids",
            "work_id and action_id are required",
        ));
    }

    let surface = build_pre_use_dcc_mvp_runtime_surface();
    validate_dcc_mvp_runtime_surface(&surface).map_err(|errors| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "kernel_dcc_projection_invalid",
            format!("backend DCC projection failed validation: {errors:?}"),
        )
    })?;

    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).map_err(|errors| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "kernel_dcc_catalog_invalid",
            format!("kernel action catalog failed validation: {errors:?}"),
        )
    })?;

    let preview =
        preview_dcc_governed_action(&surface, &catalog, action_id, work_id).map_err(|errors| {
            api_error(
                StatusCode::BAD_REQUEST,
                "kernel_dcc_action_preview_rejected",
                format!("DCC governed action preview rejected request: {errors:?}"),
            )
        })?;

    let requires_same_turn_approval = preview
        .approval_preview_id
        .as_deref()
        .and_then(|preview_id| {
            surface
                .approval_previews
                .iter()
                .find(|approval| approval.preview_id == preview_id)
        })
        .is_some_and(|approval| approval.requires_same_turn_approval);
    if requires_same_turn_approval
        && (request.approval_preview_id.as_deref() != preview.approval_preview_id.as_deref()
            || request.same_turn_approval != Some(true))
    {
        return Err(api_error(
            StatusCode::CONFLICT,
            "kernel_dcc_same_turn_approval_required",
            "governed DCC action requires matching same-turn approval preview",
        ));
    }

    Ok(Json(DccGovernedActionTriggerResponse {
        schema_id: "hsk.kernel.dcc_governed_action_trigger_result@1",
        work_id: preview.work_id,
        action_id: preview.action_id,
        triggered: true,
        catalog_checked: true,
        preview_checked: true,
        gate_enforced: true,
        approval_preview_id: preview.approval_preview_id,
        authority_effect: preview.authority_effect,
        approval_posture: preview.approval_posture,
        expected_write_box_kinds: preview.expected_write_box_kinds,
        receipt_ref: format!("receipt://kernel-dcc/action-trigger/{work_id}/{action_id}"),
    }))
}

pub async fn session_spawn_tree_dcc_projection(
    Json(tree): Json<SessionSpawnTreeDccV1>,
) -> ApiResult<SessionSpawnTreeDccApiProjection> {
    let projection = project_session_spawn_tree_dcc(&tree).map_err(|errors| {
        api_error(
            StatusCode::BAD_REQUEST,
            "kernel_session_spawn_tree_dcc_invalid",
            format!("session spawn tree DCC projection rejected runtime records: {errors:?}"),
        )
    })?;

    Ok(Json(SessionSpawnTreeDccApiProjection {
        schema_id: projection.schema_id,
        tree_id: projection.tree_id,
        panel_id: projection.panel_id,
        visible_fields: projection
            .visible_fields
            .iter()
            .map(|field| format!("{field:?}"))
            .collect(),
        nodes: projection
            .nodes
            .into_iter()
            .map(|node| SessionSpawnTreeNodeApiProjection {
                session_id: node.session_id,
                parent_session_id: node.parent_session_id,
                role_id: node.role_id,
                depth: node.depth,
                child_count: node.child_count,
                active_child_count: node.active_child_count,
                spawn_mode: format!("{:?}", node.spawn_mode),
                runtime_state: format!("{:?}", node.runtime_state),
                cascade_cancel_available: node.cascade_cancel_available,
                announce_back_badges: node.announce_back_badges,
            })
            .collect(),
        max_depth: projection.max_depth,
        cascade_cancel_session_ids: projection.cascade_cancel_session_ids,
        announce_back_badge_count: projection.announce_back_badge_count,
        runtime_record_refs: projection.runtime_record_refs,
        mutates_runtime_records: projection.mutates_runtime_records,
    }))
}

pub async fn execute_product_screenshot_capture_api(
    Json(request): Json<ProductScreenshotCaptureExecuteRequest>,
) -> ApiResult<ProductScreenshotCaptureExecuteResponse> {
    let result = capture_product_screenshot_from_browser_adapter(
        &request.request,
        ProductScreenshotBrowserAdapterConfigV1 {
            source_url: request.source_url,
            adapter_script_path: request
                .adapter_script_path
                .unwrap_or_else(|| "app/scripts/handshake-screenshot-capture.mjs".to_string()),
            node_binary: request.node_binary.unwrap_or_else(|| "node".to_string()),
            command_or_api_ref: "api://kernel.product_screenshot_capture.execute".to_string(),
        },
        "../Handshake_Artifacts/handshake-product/screenshots",
    )
    .map_err(|err| {
        api_error(
            StatusCode::BAD_REQUEST,
            "kernel_product_screenshot_capture_execute_failed",
            format!("{err:?}"),
        )
    })?;

    Ok(Json(ProductScreenshotCaptureExecuteResponse {
        schema_id: "hsk.kernel.product_screenshot_capture_execute_result@1",
        artifact: result.artifact,
        durable_receipt: result.durable_receipt,
        proof: result.proof,
        receipt: result.receipt,
    }))
}

async fn session_spawn_runtime_records_from_state(
    state: &AppState,
) -> Vec<SessionSpawnRuntimeRecordV1> {
    let snapshot = state.session_registry.snapshot().await;
    let mut sessions: Vec<ModelSession> = snapshot.active_sessions.into_values().collect();
    sessions.sort_by(|left, right| {
        left.spawn_depth
            .cmp(&right.spawn_depth)
            .then_with(|| left.session_id.cmp(&right.session_id))
    });

    if !sessions
        .iter()
        .any(|session| session.parent_session_id.is_some())
    {
        return Vec::new();
    }

    let evidence = session_spawn_runtime_evidence_from_state(state, &sessions)
        .await
        .unwrap_or_default();
    session_spawn_runtime_records_from_sessions(&sessions, &evidence)
}

#[derive(Debug, Default)]
pub struct SessionSpawnRuntimeEvidence {
    pub flight_recorder_refs: HashMap<String, String>,
    pub announce_back_badges: HashMap<String, Vec<SessionAnnounceBackBadgeV1>>,
    pub cascade_cancel_session_ids: HashSet<String>,
}

async fn session_spawn_runtime_evidence_from_state(
    state: &AppState,
    sessions: &[ModelSession],
) -> Result<SessionSpawnRuntimeEvidence, crate::flight_recorder::RecorderError> {
    let events = state
        .flight_recorder
        .list_events(EventFilter::default())
        .await?;
    Ok(derive_session_spawn_runtime_evidence(sessions, &events))
}

/// Derive the spawn-runtime evidence used by the DCC session-spawn-tree
/// projection from in-scope `ModelSession`s plus Flight Recorder events.
///
/// This is the pure path used by `session_spawn_runtime_evidence_from_state`
/// after it pulls the recorder events from the active state. It is exposed so
/// tests can drive announce-back, cascade-cancel, and flight-recorder pairing
/// behavior without standing up a Postgres-backed AppState — the test surface
/// must fail closed if a future change reverts to hardcoded synthesis.
pub fn derive_session_spawn_runtime_evidence(
    sessions: &[ModelSession],
    events: &[FlightRecorderEvent],
) -> SessionSpawnRuntimeEvidence {
    let session_ids: HashSet<&str> = sessions
        .iter()
        .map(|session| session.session_id.as_str())
        .collect();
    let mut evidence = SessionSpawnRuntimeEvidence::default();
    for session in sessions {
        if session_has_explicit_cascade_cancel_capability(session) {
            evidence
                .cascade_cancel_session_ids
                .insert(session.session_id.clone());
        }
    }
    for event in events {
        if let Some(session_id) = session_id_from_spawn_event(&event.event_type, &event.payload) {
            if session_ids.contains(session_id.as_str()) {
                evidence
                    .flight_recorder_refs
                    .entry(session_id)
                    .or_insert_with(|| format!("FR-EVT-SESSION-SPAWN-{}", event.event_id));
            }
        }

        if matches!(
            event.event_type,
            FlightRecorderEventType::SessionSpawnAnnounceBack
        ) {
            if let Some(badge) = announce_back_badge_from_event(&event.event_id, &event.payload) {
                if session_ids.contains(badge.session_id.as_str()) {
                    evidence
                        .announce_back_badges
                        .entry(badge.session_id.clone())
                        .or_default()
                        .push(badge);
                }
            }
        }

        if matches!(
            event.event_type,
            FlightRecorderEventType::SessionCascadeCancel
        ) {
            if let Some(root_session_id) = event
                .payload
                .get("root_session_id")
                .and_then(serde_json::Value::as_str)
            {
                if session_ids.contains(root_session_id) {
                    evidence
                        .cascade_cancel_session_ids
                        .insert(root_session_id.to_string());
                }
            }
        }
    }
    evidence
}

fn session_id_from_spawn_event(
    event_type: &FlightRecorderEventType,
    payload: &serde_json::Value,
) -> Option<String> {
    match event_type {
        FlightRecorderEventType::SessionSpawnAccepted => payload
            .get("child_session_id")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
        FlightRecorderEventType::SessionSpawnAnnounceBack => payload
            .get("child_session_id")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
        FlightRecorderEventType::SessionCreated => payload
            .get("session_id")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
        _ => None,
    }
}

fn announce_back_badge_from_event(
    event_id: &uuid::Uuid,
    payload: &serde_json::Value,
) -> Option<SessionAnnounceBackBadgeV1> {
    let session_id = payload
        .get("child_session_id")
        .and_then(serde_json::Value::as_str)?;
    let status = payload
        .get("status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("recorded");
    let mailbox_message_id = payload
        .get("mailbox_message_id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("message-unavailable")
        .replace(':', "/");

    Some(SessionAnnounceBackBadgeV1 {
        badge_id: format!("announce-back-{event_id}"),
        session_id: session_id.to_string(),
        label: format!("announce-back {status}"),
        mailbox_route: format!("role-mailbox://{session_id}/announce-back/{mailbox_message_id}"),
    })
}

fn session_has_explicit_cascade_cancel_capability(session: &ModelSession) -> bool {
    session.capability_grants.iter().any(|grant| {
        let normalized = grant.trim().to_ascii_lowercase();
        matches!(
            normalized.as_str(),
            "session.cascade_cancel"
                | "session.cascade-cancel"
                | "kernel.session.cascade_cancel"
                | "kernel.session.cascade-cancel"
                | "cascade_cancel"
                | "cascade-cancel"
        )
    })
}

fn session_spawn_runtime_records_from_sessions(
    sessions: &[ModelSession],
    evidence: &SessionSpawnRuntimeEvidence,
) -> Vec<SessionSpawnRuntimeRecordV1> {
    let runtime_record_refs: HashMap<&str, String> = sessions
        .iter()
        .filter_map(|session| {
            model_session_runtime_record_ref(session)
                .map(|runtime_ref| (session.session_id.as_str(), runtime_ref))
        })
        .collect();
    let eligible_sessions: Vec<&ModelSession> = sessions
        .iter()
        .filter(|session| {
            runtime_record_refs.contains_key(session.session_id.as_str())
                && evidence
                    .flight_recorder_refs
                    .contains_key(session.session_id.as_str())
                && session.parent_session_id.as_ref().map_or(true, |parent| {
                    runtime_record_refs.contains_key(parent.as_str())
                        && evidence.flight_recorder_refs.contains_key(parent.as_str())
                })
        })
        .collect();

    eligible_sessions
        .iter()
        .map(|session| SessionSpawnRuntimeRecordV1 {
            session_id: session.session_id.clone(),
            parent_session_id: session.parent_session_id.clone(),
            role_id: session.role.clone(),
            spawn_mode: session_spawn_mode_from_execution_mode(&session.execution_mode),
            runtime_state: session_runtime_state(&session.state),
            cascade_cancel_supported: evidence
                .cascade_cancel_session_ids
                .contains(session.session_id.as_str()),
            announce_back_badges: evidence
                .announce_back_badges
                .get(session.session_id.as_str())
                .cloned()
                .unwrap_or_default(),
            runtime_record_ref: runtime_record_refs[session.session_id.as_str()].clone(),
            flight_recorder_ref: evidence.flight_recorder_refs[session.session_id.as_str()].clone(),
        })
        .collect()
}

fn model_session_runtime_record_ref(session: &ModelSession) -> Option<String> {
    session
        .job_id
        .map(|job_id| format!("runtime://session-spawn/job/{job_id}"))
        .or_else(|| {
            session
                .checkpoint_artifact_id
                .as_ref()
                .map(|checkpoint_id| format!("runtime://session-spawn/checkpoint/{checkpoint_id}"))
        })
}

fn session_spawn_mode_from_execution_mode(execution_mode: &str) -> SessionSpawnMode {
    let normalized = execution_mode.trim().to_ascii_lowercase();
    if normalized.contains("persistent") {
        SessionSpawnMode::SessionPersistent
    } else {
        SessionSpawnMode::OneShot
    }
}

fn session_runtime_state(state: &ModelSessionState) -> SessionRuntimeState {
    match state {
        ModelSessionState::Completed => SessionRuntimeState::Completed,
        ModelSessionState::Cancelled => SessionRuntimeState::Cancelled,
        ModelSessionState::Failed => SessionRuntimeState::Failed,
        ModelSessionState::Created
        | ModelSessionState::Active
        | ModelSessionState::Paused
        | ModelSessionState::Blocked => SessionRuntimeState::Active,
    }
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/kernel/trace_projection", get(inspect_trace_projection))
        .route("/kernel/dcc_projection", get(dcc_projection))
        .route(
            "/kernel/parallel_swarm/dashboard_projection",
            get(parallel_swarm_dashboard_projection),
        )
        .route(
            "/kernel/dcc_actions/trigger",
            post(trigger_dcc_governed_action),
        )
        .route(
            "/kernel/session_spawn_tree_dcc_projection",
            post(session_spawn_tree_dcc_projection),
        )
        .route(
            "/kernel/product_screenshot_capture/execute",
            post(execute_product_screenshot_capture_api),
        )
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};
    use uuid::Uuid;

    fn spawn_tree_request() -> SessionSpawnTreeDccV1 {
        serde_json::from_value(serde_json::json!({
            "schema_id": "hsk.kernel.session_spawn_tree_dcc@1",
            "tree_id": "tree-api-test",
            "folded_stub_ids": ["WP-1-Session-Spawn-Tree-DCC-Visualization-v1"],
            "panel_id": "session-spawn-tree",
            "visible_fields": [
                "SpawnHierarchy",
                "ChildCounts",
                "SpawnDepth",
                "CascadeCancel",
                "SpawnMode",
                "AnnounceBackBadges"
            ],
            "runtime_records": [
                {
                    "session_id": "session-root",
                    "parent_session_id": null,
                    "role_id": "orchestrator",
                    "spawn_mode": "SessionPersistent",
                    "runtime_state": "Active",
                    "cascade_cancel_supported": true,
                    "announce_back_badges": [
                        {
                            "badge_id": "badge-root",
                            "session_id": "session-root",
                            "label": "announce-back-ready",
                            "mailbox_route": "role-mailbox://session-root"
                        }
                    ],
                    "runtime_record_ref": "runtime://session-spawn/session-root",
                    "flight_recorder_ref": "FR-EVT-SESSION-SPAWN-root"
                },
                {
                    "session_id": "session-child",
                    "parent_session_id": "session-root",
                    "role_id": "coder",
                    "spawn_mode": "OneShot",
                    "runtime_state": "Active",
                    "cascade_cancel_supported": false,
                    "announce_back_badges": [],
                    "runtime_record_ref": "runtime://session-spawn/session-child",
                    "flight_recorder_ref": "FR-EVT-SESSION-SPAWN-child"
                }
            ],
            "product_authority_refs": [
                "kernel.dcc_mvp_runtime_surface",
                "kernel.role_mailbox_inbox_evidence_bridge",
                "kernel.session_anti_pattern_registry",
                "flight_recorder.session_spawn"
            ],
            "folded_source_refs": [
                ".GOV/task_packets/stubs/WP-1-Session-Spawn-Tree-DCC-Visualization-v1.contract.json"
            ]
        }))
        .expect("valid spawn tree")
    }

    #[tokio::test]
    async fn trigger_dcc_action_requires_catalog_preview_gate() {
        let rejected = trigger_dcc_governed_action(Json(DccGovernedActionTriggerRequest {
            work_id: "work-kernel002-mt050-preuse".to_string(),
            action_id: "kernel.not_registered".to_string(),
            approval_preview_id: None,
            same_turn_approval: None,
        }))
        .await;

        assert!(rejected.is_err());
        let (status, Json(error)) = rejected.err().expect("catalog preview rejection");
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(error.code, "kernel_dcc_action_preview_rejected");

        let Json(accepted) = trigger_dcc_governed_action(Json(DccGovernedActionTriggerRequest {
            work_id: "work-kernel002-mt050-preuse".to_string(),
            action_id: "kernel.write_box.promote".to_string(),
            approval_preview_id: Some("approval-preuse-promote".to_string()),
            same_turn_approval: Some(true),
        }))
        .await
        .expect("same-turn approval should pass");

        assert!(accepted.triggered);
        assert!(accepted.preview_checked);
        assert!(accepted.gate_enforced);
        assert_eq!(accepted.action_id, "kernel.write_box.promote");
    }

    #[tokio::test]
    async fn projects_session_spawn_tree_runtime_records_for_dcc() {
        let Json(projection) = session_spawn_tree_dcc_projection(Json(spawn_tree_request()))
            .await
            .expect("spawn tree projection should pass");

        assert_eq!(
            projection.schema_id,
            "hsk.kernel.session_spawn_tree_dcc_projection@1"
        );
        assert_eq!(projection.max_depth, 1);
        assert_eq!(projection.announce_back_badge_count, 1);
        assert_eq!(projection.cascade_cancel_session_ids, vec!["session-root"]);
        assert!(!projection.mutates_runtime_records);

        let root = projection
            .nodes
            .iter()
            .find(|node| node.session_id == "session-root")
            .expect("root node projected");
        assert_eq!(root.child_count, 1);
        assert_eq!(root.active_child_count, 1);
        assert_eq!(root.spawn_mode, "SessionPersistent");
        assert_eq!(root.announce_back_badges, vec!["announce-back-ready"]);
    }

    #[test]
    fn derives_session_spawn_records_from_runtime_model_sessions() {
        let sessions = vec![
            model_session(
                "session-child",
                Some("session-root"),
                1,
                "one-shot",
                ModelSessionState::Active,
                "00000000-0000-0000-0000-000000000002",
            ),
            model_session(
                "session-root",
                None,
                0,
                "session-persistent",
                ModelSessionState::Active,
                "00000000-0000-0000-0000-000000000001",
            ),
        ];
        let runtime_event_evidence = SessionSpawnRuntimeEvidence {
            flight_recorder_refs: HashMap::from([
                (
                    "session-child".to_string(),
                    "FR-EVT-SESSION-SPAWN-11111111-1111-1111-1111-111111111111".to_string(),
                ),
                (
                    "session-root".to_string(),
                    "FR-EVT-SESSION-SPAWN-22222222-2222-2222-2222-222222222222".to_string(),
                ),
            ]),
            announce_back_badges: HashMap::new(),
            cascade_cancel_session_ids: HashSet::new(),
        };

        let records =
            session_spawn_runtime_records_from_sessions(&sessions, &runtime_event_evidence);
        assert!(
            session_spawn_runtime_records_from_sessions(
                &sessions,
                &SessionSpawnRuntimeEvidence::default()
            )
            .is_empty(),
            "DCC records must not synthesize Flight Recorder refs when runtime events are absent"
        );

        let child = records
            .iter()
            .find(|record| record.session_id == "session-child")
            .expect("child runtime record");
        assert_eq!(child.parent_session_id.as_deref(), Some("session-root"));
        assert_eq!(child.spawn_mode, SessionSpawnMode::OneShot);
        assert!(!child.cascade_cancel_supported);
        assert_eq!(
            child.flight_recorder_ref,
            "FR-EVT-SESSION-SPAWN-11111111-1111-1111-1111-111111111111"
        );
        assert_eq!(
            child.runtime_record_ref,
            "runtime://session-spawn/job/00000000-0000-0000-0000-000000000002"
        );
        assert!(
            child.announce_back_badges.is_empty(),
            "announce-back badges must come from explicit runtime or Flight Recorder records"
        );

        let root = records
            .iter()
            .find(|record| record.session_id == "session-root")
            .expect("root runtime record");
        assert_eq!(root.parent_session_id, None);
        assert_eq!(root.spawn_mode, SessionSpawnMode::SessionPersistent);
        assert!(
            !root.cascade_cancel_supported,
            "cascade cancel must not be inferred from active child count"
        );

        let explicit_badge = SessionAnnounceBackBadgeV1 {
            badge_id: "announce-back-event-child".to_string(),
            session_id: "session-child".to_string(),
            label: "announce-back completed".to_string(),
            mailbox_route: "role-mailbox://session-child/announce-back/mailbox/child/announce_back"
                .to_string(),
        };
        let explicit_runtime_evidence = SessionSpawnRuntimeEvidence {
            announce_back_badges: HashMap::from([(
                "session-child".to_string(),
                vec![explicit_badge.clone()],
            )]),
            cascade_cancel_session_ids: HashSet::from(["session-root".to_string()]),
            ..runtime_event_evidence
        };
        let explicit_records =
            session_spawn_runtime_records_from_sessions(&sessions, &explicit_runtime_evidence);
        let child = explicit_records
            .iter()
            .find(|record| record.session_id == "session-child")
            .expect("child runtime record");
        assert_eq!(child.announce_back_badges, vec![explicit_badge]);
        let root = explicit_records
            .iter()
            .find(|record| record.session_id == "session-root")
            .expect("root runtime record");
        assert!(root.cascade_cancel_supported);
    }

    fn model_session(
        session_id: &str,
        parent_session_id: Option<&str>,
        spawn_depth: i32,
        execution_mode: &str,
        state: ModelSessionState,
        job_id: &str,
    ) -> ModelSession {
        ModelSession {
            session_id: session_id.to_string(),
            parent_session_id: parent_session_id.map(str::to_string),
            spawn_depth,
            state,
            model_id: "gpt-test".to_string(),
            backend: "codex".to_string(),
            parameter_class: "standard".to_string(),
            role: "CODER".to_string(),
            wp_id: Some("WP-KERNEL-002".to_string()),
            mt_id: Some("MT-043".to_string()),
            work_profile_id: None,
            execution_mode: execution_mode.to_string(),
            memory_policy: "SESSION_SCOPED".to_string(),
            consent_receipt_id: None,
            capability_grants: Vec::new(),
            capability_token_ids: None,
            job_id: Some(Uuid::parse_str(job_id).expect("test job uuid")),
            checkpoint_artifact_id: None,
            last_checkpoint_at: None,
            checkpoint_count: 0,
            merge_back_artifact: None,
            agent: None,
            purpose: None,
            close_reason: None,
            closed_by_actor: None,
            closed_at: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }
}
