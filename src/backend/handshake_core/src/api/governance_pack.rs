use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use serde::Serialize;

use crate::governance_pack::GovernancePackExportRequest;
use crate::jobs::create_job;
use crate::models::JobKind;
use crate::storage::EntityRef;
use crate::workflows::start_workflow_for_job;
use crate::AppState;

#[derive(Debug, Serialize)]
pub struct GovernancePackExportResponse {
    pub export_job_id: String,
    pub status: String,
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/api/governance_pack/export", post(export_governance_pack))
        .with_state(state)
}

async fn export_governance_pack(
    State(state): State<AppState>,
    Json(request): Json<GovernancePackExportRequest>,
) -> Result<(StatusCode, Json<GovernancePackExportResponse>), (StatusCode, String)> {
    let job_kind = JobKind::GovernancePackExport;
    let capability_profile = state
        .capability_registry
        .profile_for_job(job_kind.as_str())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let job_inputs =
        serde_json::to_value(&request).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let job = create_job(
        &state,
        job_kind,
        "hsk.governance_pack.export.v0",
        // Server-enforced capability profile to prevent client-side escalation.
        capability_profile.id.as_str(),
        Some(job_inputs),
        Vec::<EntityRef>::new(),
    )
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let export_job_id = job.job_id.to_string();
    let state_clone = state.clone();
    tokio::spawn(async move {
        let _ = start_workflow_for_job(&state_clone, job).await;
    });

    Ok((
        StatusCode::ACCEPTED,
        Json(GovernancePackExportResponse {
            export_job_id,
            status: "queued".to_string(),
        }),
    ))
}
