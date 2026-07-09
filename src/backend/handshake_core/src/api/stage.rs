//! WP-KERNEL-012 MT-066 (Stage capture): the Stage artifact store HTTP surface —
//! the backend behind the frontend embed-back leg
//! `GET /workspaces/:ws/stage/artifacts/:artifact_id`
//! (`stage_interop::StageClient::fetch_stage_artifact`).
//!
//! Routes (workspace-scoped, P1):
//!   * `POST /workspaces/:workspace_id/stage/artifacts`
//!       `{content_kind, label?, content_type, content_json, source_ref?}`
//!       -> `201 StageArtifactRef` (the created provenance descriptor)
//!   * `GET  /workspaces/:workspace_id/stage/artifacts/:artifact_id`
//!       -> `200 StageArtifactRef` (404 if absent)
//!
//! The response is the EVIDENCE-GRADE provenance descriptor the frontend decodes
//! (`StageArtifactRef` / `StageManifest`): `sha256` (lowercase 64-hex) and
//! `manifest.manifest_ref` are ALWAYS non-empty — the backend twin of
//! `stage_interop::StageManifest::is_evidence_grade`. An artifact missing either
//! is refused as `ProvenanceMissing` on the frontend, so the store never emits
//! one. Inline-text captures only (MT-066); binary/blob capture is deferred (the
//! frontend GET is metadata-only). PostgreSQL authority, no SQLite.

use crate::models::ErrorResponse;
use crate::storage::{
    NewStageCaptureArtifact, StageArtifactStore, StageCaptureArtifact, StorageError,
};
use crate::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

type ApiError = (StatusCode, Json<ErrorResponse>);
type ApiResult<T> = Result<T, ApiError>;

fn bad_request(code: &'static str) -> ApiError {
    (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: code }))
}

fn not_found(code: &'static str) -> ApiError {
    (StatusCode::NOT_FOUND, Json(ErrorResponse { error: code }))
}

fn internal_error(err: impl std::fmt::Display) -> ApiError {
    tracing::error!(target: "handshake_core::stage_api", error = %err, "stage_api_error");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "HSK-500-STAGE",
        }),
    )
}

fn map_storage_error(err: StorageError) -> ApiError {
    match err {
        StorageError::NotFound(code) => not_found(code),
        StorageError::Validation(_) => bad_request("HSK-400-STAGE"),
        other => internal_error(other),
    }
}

async fn ensure_workspace_exists(state: &AppState, workspace_id: &str) -> ApiResult<()> {
    match state.storage.get_workspace(workspace_id).await {
        Ok(Some(_)) => Ok(()),
        Ok(None) => Err(not_found("workspace_not_found")),
        Err(err) => Err(map_storage_error(err)),
    }
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route(
            "/workspaces/:workspace_id/stage/artifacts",
            post(create_stage_artifact),
        )
        .route(
            "/workspaces/:workspace_id/stage/artifacts/:artifact_id",
            get(get_stage_artifact),
        )
        .with_state(state)
}

/// The SHA-256 manifest descriptor carried inside the response. Both `sha256`
/// and `manifest_ref` are always non-empty (the evidence-grade contract).
#[derive(Debug, Serialize)]
struct StageManifestWire {
    sha256: String,
    manifest_ref: String,
    content_type: String,
}

/// The provenance-descriptor wire shape the native `StageArtifactRef` decoder
/// expects (metadata, not content bytes).
#[derive(Debug, Serialize)]
struct StageArtifactRefWire {
    artifact_id: String,
    workspace_id: String,
    /// The hoisted SHA-256 (lowercase 64-hex; also inside `manifest`).
    sha256: String,
    manifest: StageManifestWire,
    label: String,
}

/// Project a stored artifact to the evidence-grade wire shape.
fn artifact_to_wire(artifact: StageCaptureArtifact) -> StageArtifactRefWire {
    StageArtifactRefWire {
        artifact_id: artifact.artifact_id,
        workspace_id: artifact.workspace_id,
        sha256: artifact.content_sha256.clone(),
        manifest: StageManifestWire {
            sha256: artifact.content_sha256,
            manifest_ref: artifact.manifest_ref,
            content_type: artifact.content_type,
        },
        label: artifact.label,
    }
}

/// Body for `POST /workspaces/:ws/stage/artifacts`.
#[derive(Debug, Deserialize)]
struct CreateStageArtifactBody {
    content_kind: String,
    #[serde(default)]
    label: String,
    content_type: String,
    content_json: Value,
    #[serde(default)]
    source_ref: Option<String>,
}

/// `POST /workspaces/:ws/stage/artifacts` — capture an inline-text Stage
/// artifact and return its evidence-grade provenance descriptor.
async fn create_stage_artifact(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    Json(body): Json<CreateStageArtifactBody>,
) -> ApiResult<(StatusCode, Json<StageArtifactRefWire>)> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let store = StageArtifactStore::new(state.postgres_pool.clone());
    let artifact = store
        .insert_stage_artifact(NewStageCaptureArtifact {
            workspace_id: workspace_id.clone(),
            content_kind: body.content_kind,
            label: body.label,
            content_type: body.content_type,
            content_json: body.content_json,
            source_ref: body.source_ref,
        })
        .await
        .map_err(map_storage_error)?;
    Ok((StatusCode::CREATED, Json(artifact_to_wire(artifact))))
}

/// `GET /workspaces/:ws/stage/artifacts/:artifact_id` — resolve a Stage capture
/// artifact to its provenance descriptor (404 when absent).
async fn get_stage_artifact(
    State(state): State<AppState>,
    Path((workspace_id, artifact_id)): Path<(String, String)>,
) -> ApiResult<Json<StageArtifactRefWire>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let store = StageArtifactStore::new(state.postgres_pool.clone());
    let artifact = store
        .get_stage_artifact(&workspace_id, &artifact_id)
        .await
        .map_err(map_storage_error)?;
    match artifact {
        Some(artifact) => Ok(Json(artifact_to_wire(artifact))),
        None => Err(not_found("stage_artifact_not_found")),
    }
}
