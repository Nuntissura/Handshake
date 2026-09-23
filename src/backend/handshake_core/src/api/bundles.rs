use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::Response,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::bundles::{
    bundle_path, BundleExportError, BundleScope, DebugBundleExporter, DefaultDebugBundleExporter,
    ExportableFilter, ExportableInventory, RedactionMode,
};
use crate::jobs::create_job;
use crate::models::JobKind;
use crate::storage::{AiJob, EntityRef, JobState};
use crate::workflows::start_workflow_for_job;
use crate::AppState;

#[derive(Debug, Deserialize, Clone)]
pub struct ExportRequest {
    pub scope: ExportScope,
    pub redaction_mode: RedactionMode,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ExportScope {
    pub kind: String,
    #[serde(default)]
    pub problem_id: Option<String>,
    #[serde(default)]
    pub job_id: Option<String>,
    #[serde(default)]
    pub time_range: Option<TimeRangeRequest>,
    #[serde(default)]
    pub wsid: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TimeRangeRequest {
    pub start: String,
    pub end: String,
}

#[derive(Debug, Serialize)]
pub struct ExportResponse {
    pub export_job_id: String,
    pub status: String,
    pub estimated_size_bytes: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct BundleStatus {
    pub bundle_id: String,
    pub status: String,
    pub manifest: Option<serde_json::Value>,
    pub error: Option<String>,
    pub expires_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ValidationResponse {
    pub valid: bool,
    pub findings: Vec<crate::bundles::ValidationFinding>,
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/api/bundles/debug/export", post(export_bundle))
        .route("/api/bundles/debug/exportable", get(list_exportable))
        .route("/api/bundles/debug/:bundle_id", get(bundle_status))
        .route(
            "/api/bundles/debug/:bundle_id/validate",
            post(validate_bundle),
        )
        .route(
            "/api/bundles/debug/:bundle_id/download",
            get(download_bundle),
        )
        // MT-156 (Master Spec 02-system-architecture.md:2758): deny by default before any handler,
        // table, filesystem or DuckDB access.
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::api::authority::require_authenticated_session,
        ))
        .with_state(state)
}

type BundleApiError = (StatusCode, String);

/// The constant denial of every bundle route (never discloses whether a bundle, job or workspace
/// exists for another account).
fn bundle_denied() -> BundleApiError {
    (
        StatusCode::FORBIDDEN,
        json!({"error": "HSK-403-PROTECTED-RESOURCE"}).to_string(),
    )
}

/// MT-156 AC-156-2/4: the caller's Workspace Read + fr.read authority on `wsid` (Flight Recorder /
/// diagnostics are protected resources, 02-system-architecture.md:2774).
async fn bundle_workspace_authority(
    state: &AppState,
    headers: &HeaderMap,
    wsid: &str,
) -> Result<crate::api::authority::AuthorizedResourceContext, BundleApiError> {
    use crate::storage::surreal::resource_authority::{ResourceAction, ResourceKind};
    if wsid.trim().is_empty() {
        return Err(bundle_denied());
    }
    crate::api::authority::authorize_request(
        state,
        headers,
        "fr.read",
        ResourceKind::Workspace,
        wsid,
        ResourceAction::Read,
    )
    .await
    .map_err(|_| bundle_denied())
}

/// A session-only record-user scope for resolving a job/bundle row: the `ai_jobs` select predicate
/// admits only jobs of workspaces the session can read, so another account's job is invisible.
async fn session_read_scope(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<
    (
        crate::api::authority::AuthenticatedLocalSession,
        crate::storage::surreal::resource_authority::RecordUserScope,
    ),
    BundleApiError,
> {
    let session = crate::api::authority::authenticated_session_credentials(state, headers)
        .await
        .map_err(|_| bundle_denied())?;
    let scope = crate::storage::surreal::resource_authority::RecordUserScope {
        grant_id: None,
        workspace_id: None,
        session_token: session.session_token.clone(),
        channel_binding_hash: Some(session.channel_binding_hash.clone()),
        resource_id: String::new(),
        session_id: session.context.session_id.clone(),
        capability_id: "fr.read".to_owned(),
        action: crate::storage::surreal::resource_authority::ResourceAction::Read,
    };
    Ok((session, scope))
}

/// The single workspace a job names (its `workspace` entity ref, else job_inputs wsid/workspace_id).
fn job_workspace(job: &AiJob) -> Option<String> {
    let refs = job
        .entity_refs
        .iter()
        .filter(|entity| entity.entity_kind == "workspace")
        .map(|entity| entity.entity_id.clone())
        .collect::<Vec<_>>();
    match refs.as_slice() {
        [only] => Some(only.clone()),
        [] => job.job_inputs.as_ref().and_then(|inputs| {
            inputs
                .get("wsid")
                .or_else(|| inputs.get("workspace_id"))
                .and_then(|value| value.as_str())
                .map(str::to_owned)
        }),
        _ => None,
    }
}

/// MT-156 AC-156-6: a bundle id is the export job's UUID; anything else never reaches a path join.
fn validated_bundle_id(bundle_id: &str) -> Result<uuid::Uuid, BundleApiError> {
    uuid::Uuid::parse_str(bundle_id)
        .ok()
        .filter(|parsed| parsed.hyphenated().to_string() == bundle_id)
        .ok_or((
            StatusCode::BAD_REQUEST,
            "HSK-400-BUNDLE-ID: bundle id must be a canonical UUID".to_string(),
        ))
}

/// MT-156 AC-156-4: the bundle's export job, readable by the caller's record user, recorded as
/// requested by the caller's account, and whose workspace the caller still reads with fr.read.
/// Every failure (absent, foreign, not an export, revoked) is the constant denial.
async fn authorized_bundle_job(
    state: &AppState,
    headers: &HeaderMap,
    bundle_id: &str,
) -> Result<AiJob, BundleApiError> {
    validated_bundle_id(bundle_id)?;
    let (session, scope) = session_read_scope(state, headers).await?;
    let job = state
        .surreal
        .with_record_user_scope(scope, state.storage.get_ai_job(bundle_id))
        .await
        .map_err(|_| bundle_denied())?;
    if job.job_kind != JobKind::DebugBundleExport {
        return Err(bundle_denied());
    }
    let requested_by = job
        .job_inputs
        .as_ref()
        .and_then(|inputs| inputs.get("requested_by"))
        .and_then(|requester| requester.get("account_id"))
        .and_then(|account| account.as_str());
    if requested_by != Some(session.context.identity.account_id.as_str()) {
        return Err(bundle_denied());
    }
    let wsid = job_workspace(&job).ok_or_else(bundle_denied)?;
    bundle_workspace_authority(state, headers, &wsid).await?;
    Ok(job)
}

fn parse_scope(scope: ExportScope) -> Result<BundleScope, BundleExportError> {
    match scope.kind.as_str() {
        "problem" => scope
            .problem_id
            .map(|id| BundleScope::Problem { diagnostic_id: id })
            .ok_or_else(|| BundleExportError::InvalidScope("problem_id required".to_string())),
        "job" => scope
            .job_id
            .map(|id| BundleScope::Job { job_id: id })
            .ok_or_else(|| BundleExportError::InvalidScope("job_id required".to_string())),
        "time_window" => {
            if let Some(range) = scope.time_range {
                let start = chrono::DateTime::parse_from_rfc3339(&range.start)
                    .map_err(|e| {
                        BundleExportError::InvalidScope(format!(
                            "invalid time_range.start (expected RFC3339): {e}"
                        ))
                    })?
                    .with_timezone(&chrono::Utc);
                let end = chrono::DateTime::parse_from_rfc3339(&range.end)
                    .map_err(|e| {
                        BundleExportError::InvalidScope(format!(
                            "invalid time_range.end (expected RFC3339): {e}"
                        ))
                    })?
                    .with_timezone(&chrono::Utc);
                Ok(BundleScope::TimeWindow {
                    start,
                    end,
                    wsid: scope.wsid,
                })
            } else {
                Err(BundleExportError::InvalidScope(
                    "time_range required for time_window scope".to_string(),
                ))
            }
        }
        "workspace" => scope
            .wsid
            .map(|wsid| BundleScope::Workspace { wsid })
            .ok_or_else(|| BundleExportError::InvalidScope("wsid required".to_string())),
        _ => Err(BundleExportError::InvalidScope(format!(
            "invalid scope.kind: {}",
            &scope.kind
        ))),
    }
}

async fn export_bundle(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ExportRequest>,
) -> Result<(StatusCode, Json<ExportResponse>), (StatusCode, String)> {
    // Validate the request is well-formed per spec before queuing the job.
    let _parsed_scope =
        parse_scope(request.scope.clone()).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    // MT-156 AC-156-2: the scope must resolve to exactly one workspace the caller reads with
    // fr.read; an unresolvable scope (time_window/problem without wsid, a foreign job) is denied
    // before any job row exists.
    let wsid = match (request.scope.kind.as_str(), request.scope.wsid.clone()) {
        (_, Some(wsid)) => wsid,
        ("job", None) => {
            let job_id = request.scope.job_id.clone().ok_or_else(bundle_denied)?;
            let (_, scope) = session_read_scope(&state, &headers).await?;
            let job = state
                .surreal
                .with_record_user_scope(scope, state.storage.get_ai_job(&job_id))
                .await
                .map_err(|_| bundle_denied())?;
            job_workspace(&job).ok_or_else(bundle_denied)?
        }
        _ => return Err(bundle_denied()),
    };
    let authority = bundle_workspace_authority(&state, &headers, &wsid).await?;
    if let Some(job_id) = request.scope.job_id.as_deref() {
        // A job scope must name a job of the authorized workspace, readable by this account.
        let job = state
            .surreal
            .with_record_user_scope(
                authority.record_user_scope.clone(),
                state.storage.get_ai_job(job_id),
            )
            .await
            .map_err(|_| bundle_denied())?;
        if job_workspace(&job).as_deref() != Some(wsid.as_str()) {
            return Err(bundle_denied());
        }
    }
    let scope_value = serde_json::to_value(&request.scope)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let job_kind = JobKind::DebugBundleExport;
    let capability_profile = state
        .capability_registry
        .profile_for_job(job_kind.as_str())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let entity_refs: Vec<EntityRef> = vec![EntityRef {
        entity_id: wsid.clone(),
        entity_kind: "workspace".to_string(),
    }];

    // MT-156 AC-156-3: the job records the requesting account and session principal.
    let job_inputs = json!({
        "scope": scope_value,
        "redaction_mode": request.redaction_mode,
        "include_artifacts": false,
        "requested_by": {
            "account_id": authority.account_id,
            "principal_id": authority.principal_id,
            "actor_id": authority.actor_id,
        },
    });

    let record_user_scope = authority.record_user_scope.clone();
    let job = state
        .surreal
        .with_record_user_scope(
            record_user_scope.clone(),
            create_job(
                &state,
                job_kind,
                "hsk.bundle.export.v0",
                // Server-enforced capability profile to prevent client-side escalation.
                capability_profile.id.as_str(),
                Some(job_inputs),
                entity_refs,
            ),
        )
        .await
        .map_err(|error| match error {
            crate::jobs::JobError::Storage(crate::storage::StorageError::Guard(
                "HSK-403-PROTECTED-RESOURCE",
            )) => bundle_denied(),
            other => (StatusCode::INTERNAL_SERVER_ERROR, other.to_string()),
        })?;

    let export_job_id = job.job_id.to_string();
    let state_clone = state.clone();
    // The export runs as the requesting account's record user, never as root (F5: a spawned task
    // does not inherit the task-local scope, so it is re-entered here).
    tokio::spawn(async move {
        let _ = state_clone
            .surreal
            .clone()
            .with_record_user_scope(record_user_scope, start_workflow_for_job(&state_clone, job))
            .await;
    });

    Ok((
        StatusCode::ACCEPTED,
        Json(ExportResponse {
            export_job_id,
            status: "queued".to_string(),
            estimated_size_bytes: None,
        }),
    ))
}

async fn bundle_status(
    State(state): State<AppState>,
    Path(bundle_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<BundleStatus>, (StatusCode, String)> {
    let mut status = "pending".to_string();
    let mut error: Option<String> = None;

    {
        let job = authorized_bundle_job(&state, &headers, &bundle_id).await?;
        status = match job.state {
            JobState::Queued
            | JobState::Running
            | JobState::AwaitingUser
            | JobState::AwaitingValidation
            | JobState::Stalled => "pending".to_string(),
            JobState::Completed | JobState::CompletedWithIssues => "ready".to_string(),
            JobState::Failed | JobState::Poisoned | JobState::Cancelled => {
                error = job
                    .error_message
                    .clone()
                    .or_else(|| Some(job.state.as_str().to_string()));
                "failed".to_string()
            }
        };
    }

    let default_dir = crate::bundles::exporter::default_bundle_dir(&bundle_id);
    let path = bundle_path(&bundle_id).unwrap_or(default_dir);

    let manifest_path = path.join("bundle_manifest.json");
    if status == "pending" && manifest_path.exists() {
        status = "ready".to_string();
    }
    let mut manifest_value = if status == "ready" && manifest_path.exists() {
        let content = std::fs::read_to_string(&manifest_path)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        Some(
            serde_json::from_str::<serde_json::Value>(&content)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?,
        )
    } else {
        None
    };

    let expires_at = if let Some(manifest_value) = manifest_value.as_ref() {
        let manifest: crate::bundles::schemas::BundleManifest =
            serde_json::from_value(manifest_value.clone())
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        Some((manifest.created_at + chrono::Duration::hours(24)).to_rfc3339())
    } else {
        None
    };

    if let Some(ref expires_at_str) = expires_at {
        if let Ok(expires_at_dt) = chrono::DateTime::parse_from_rfc3339(expires_at_str) {
            if chrono::Utc::now() > expires_at_dt.with_timezone(&chrono::Utc) {
                status = "expired".to_string();
                manifest_value = None;
            }
        }
    }

    Ok(Json(BundleStatus {
        bundle_id,
        status,
        manifest: manifest_value,
        error,
        expires_at,
    }))
}

async fn download_bundle(
    State(state): State<AppState>,
    Path(bundle_id): Path<String>,
    headers: HeaderMap,
) -> Result<Response, (StatusCode, String)> {
    authorized_bundle_job(&state, &headers, &bundle_id).await?;
    let default_dir = crate::bundles::exporter::default_bundle_dir(&bundle_id);
    let path = bundle_path(&bundle_id).unwrap_or(default_dir);
    if !path.exists() {
        return Err((StatusCode::NOT_FOUND, "bundle not found".to_string()));
    }
    let zip_path = path.join(format!("{}.zip", bundle_id));
    let bytes = std::fs::read(&zip_path).map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;

    let mut response = Response::new(axum::body::Body::from(bytes));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/zip"),
    );
    response.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        header::HeaderValue::from_str(&format!("attachment; filename=\"{}.zip\"", bundle_id))
            .unwrap_or_else(|_| header::HeaderValue::from_static("attachment")),
    );
    Ok(response)
}

async fn list_exportable(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Query(filter): axum::extract::Query<ExportableFilter>,
) -> Result<Json<ExportableInventory>, (StatusCode, String)> {
    // MT-156 AC-156-4: the inventory is scoped to one workspace the caller reads with fr.read and
    // is read as the caller's record user; unscoped enumeration is denied.
    let wsid = filter.wsid.clone().ok_or_else(bundle_denied)?;
    let authority = bundle_workspace_authority(&state, &headers, &wsid).await?;
    let exporter = DefaultDebugBundleExporter::new(state.clone());
    let inventory = state
        .surreal
        .with_record_user_scope(
            authority.record_user_scope,
            exporter.list_exportable(filter),
        )
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(inventory))
}

async fn validate_bundle(
    State(state): State<AppState>,
    Path(bundle_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<ValidationResponse>, (StatusCode, String)> {
    authorized_bundle_job(&state, &headers, &bundle_id).await?;
    let default_dir = crate::bundles::exporter::default_bundle_dir(&bundle_id);
    let path = bundle_path(&bundle_id).unwrap_or(default_dir);
    if !path.exists() {
        return Err((StatusCode::NOT_FOUND, "bundle not found".to_string()));
    }
    let exporter = DefaultDebugBundleExporter::new(state);
    let report = exporter
        .validate(&path)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(ValidationResponse {
        valid: report.valid,
        findings: report
            .findings
            .into_iter()
            .map(|f| crate::bundles::ValidationFinding {
                severity: f.severity,
                code: f.code,
                message: f.message,
                file: f.file,
                path: f.path,
            })
            .collect(),
    }))
}
