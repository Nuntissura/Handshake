use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::kernel::session_spawn_tree_dcc::{
    project_session_spawn_tree_dcc, SessionSpawnTreeDccV1,
};
use crate::kernel::{
    action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
    build_pre_use_dcc_mvp_runtime_surface,
    dcc_mvp_runtime_surface::preview_dcc_governed_action,
    dcc_mvp_runtime_surface::validate_dcc_mvp_runtime_surface,
    DccMvpRuntimeSurfaceV1, KernelError, KernelTraceInspector, TraceProjection,
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

pub async fn dcc_projection() -> ApiResult<DccMvpRuntimeSurfaceV1> {
    let surface = build_pre_use_dcc_mvp_runtime_surface();
    validate_dcc_mvp_runtime_surface(&surface).map_err(|errors| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "kernel_dcc_projection_invalid",
            format!("backend DCC projection failed validation: {errors:?}"),
        )
    })?;
    Ok(Json(surface))
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

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/kernel/trace_projection", get(inspect_trace_projection))
        .route("/kernel/dcc_projection", get(dcc_projection))
        .route(
            "/kernel/dcc_actions/trigger",
            post(trigger_dcc_governed_action),
        )
        .route(
            "/kernel/session_spawn_tree_dcc_projection",
            post(session_spawn_tree_dcc_projection),
        )
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
