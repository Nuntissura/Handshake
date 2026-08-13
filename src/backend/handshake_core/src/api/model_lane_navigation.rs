use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::api::account_scope::RequestAccountScope;
use crate::swarm_orchestration::model_lane::{
    ModelLaneError, ModelLaneNavigationLookup, ModelLaneNavigationProjection, ModelLaneStore,
};
use crate::AppState;

#[derive(Debug, Deserialize, Default)]
pub struct ArtifactNavigationQuery {
    pub artifact_ref: Option<String>,
    pub artifact_binding_id: Option<String>,
    pub artifact_manifest_ref: Option<String>,
    pub artifact_payload_ref: Option<String>,
    pub artifact_sha256: Option<String>,
    pub content_hash: Option<String>,
    pub context_bundle_id: Option<String>,
    pub run_id: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct TraceNavigationQuery {
    pub span_id: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct DiagnosticNavigationQuery {
    pub behavior_id: Option<String>,
    pub tier: Option<String>,
    pub mt_id: Option<String>,
}

/// Build a store from the server-owned exact scope, retaining all five durable
/// attribution dimensions through SQL and post-decode authorization.
fn store(state: &AppState, scope: &RequestAccountScope) -> ModelLaneStore {
    ModelLaneStore::new_scoped(state.postgres_pool.clone(), scope.resource_scope())
}

type ApiError = (StatusCode, Json<Value>);

fn model_lane_api_error(err: ModelLaneError) -> ApiError {
    match err {
        ModelLaneError::InvalidInput(detail) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "bad_request", "detail": detail})),
        ),
        ModelLaneError::NotFound(_) => (StatusCode::NOT_FOUND, Json(json!({"error": "not_found"}))),
        ModelLaneError::AmbiguousLookup(detail) => (
            StatusCode::CONFLICT,
            Json(json!({"error": "ambiguous_lookup", "detail": detail})),
        ),
        // HBR-PRIV-002/004. The SQL predicate normally makes a cross-account row
        // simply absent (404, which is also what keeps existence from being an
        // oracle). Reaching here means the second, post-deserialization layer
        // caught something the predicate did not, so it is reported explicitly —
        // as the stable reason code only, with no identifiers and no row
        // contents, so the denial cannot become a metadata side channel.
        ModelLaneError::ScopeDenied(_) => {
            (StatusCode::NOT_FOUND, Json(json!({"error": "not_found"})))
        }
        ModelLaneError::AuthorityDenied(detail)
            if detail == "ModelLane navigation authority unavailable" =>
        {
            tracing::error!(target: "handshake_core::model_lane_navigation", error = %detail, "model_lane_navigation_integrity_failure");
            (StatusCode::NOT_FOUND, Json(json!({"error": "not_found"})))
        }
        other => {
            tracing::error!(target: "handshake_core::model_lane_navigation", error = %other, "model_lane_navigation_api_error");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "internal_error"})),
            )
        }
    }
}

async fn navigation_run(
    State(state): State<AppState>,
    scope: RequestAccountScope,
    Path(run_id): Path<String>,
) -> Result<Json<ModelLaneNavigationProjection>, ApiError> {
    store(&state, &scope)
        .navigation_by_run(&run_id)
        .await
        .map(Json)
        .map_err(model_lane_api_error)
}

async fn navigation_lane(
    State(state): State<AppState>,
    scope: RequestAccountScope,
    Path(lane_id): Path<String>,
) -> Result<Json<ModelLaneNavigationProjection>, ApiError> {
    store(&state, &scope)
        .navigation_by_lane(&lane_id)
        .await
        .map(Json)
        .map_err(model_lane_api_error)
}

async fn navigation_message(
    State(state): State<AppState>,
    scope: RequestAccountScope,
    Path(message_id): Path<String>,
) -> Result<Json<ModelLaneNavigationProjection>, ApiError> {
    store(&state, &scope)
        .navigation_by_message(&message_id)
        .await
        .map(Json)
        .map_err(model_lane_api_error)
}

async fn navigation_artifact_context(
    State(state): State<AppState>,
    scope: RequestAccountScope,
    Query(query): Query<ArtifactNavigationQuery>,
) -> Result<Json<ModelLaneNavigationProjection>, ApiError> {
    let artifact_ref = artifact_selector(&query).map_err(model_lane_api_error)?;
    store(&state, &scope)
        .navigation_by_artifact_or_context(
            artifact_ref.as_deref(),
            query.context_bundle_id.as_deref(),
            query.run_id.as_deref(),
        )
        .await
        .map(Json)
        .map_err(model_lane_api_error)
}

async fn navigation_trace(
    State(state): State<AppState>,
    scope: RequestAccountScope,
    Path(trace_id): Path<String>,
    Query(query): Query<TraceNavigationQuery>,
) -> Result<Json<ModelLaneNavigationProjection>, ApiError> {
    store(&state, &scope)
        .navigation_by_trace(&trace_id, query.span_id.as_deref())
        .await
        .map(Json)
        .map_err(model_lane_api_error)
}

async fn navigation_diagnostics(
    State(state): State<AppState>,
    scope: RequestAccountScope,
    Path(run_id): Path<String>,
    Query(query): Query<DiagnosticNavigationQuery>,
) -> Result<Json<ModelLaneNavigationProjection>, ApiError> {
    store(&state, &scope)
        .navigation_by_diagnostics(
            &run_id,
            query.behavior_id.as_deref(),
            query.tier.as_deref(),
            query.mt_id.as_deref(),
        )
        .await
        .map(Json)
        .map_err(model_lane_api_error)
}

async fn navigation_recovery(
    State(state): State<AppState>,
    scope: RequestAccountScope,
    Path(run_id): Path<String>,
) -> Result<Json<ModelLaneNavigationProjection>, ApiError> {
    store(&state, &scope)
        .navigation_by_recovery(&run_id)
        .await
        .map(Json)
        .map_err(model_lane_api_error)
}

async fn navigation_lookup(
    State(state): State<AppState>,
    scope: RequestAccountScope,
    Query(query): Query<ModelLaneNavigationLookup>,
) -> Result<Json<ModelLaneNavigationProjection>, ApiError> {
    store(&state, &scope)
        .navigation_by_lookup(query)
        .await
        .map(Json)
        .map_err(model_lane_api_error)
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route(
            "/swarm/model-lanes/navigation/runs/:run_id",
            get(navigation_run),
        )
        .route(
            "/swarm/model-lanes/navigation/lanes/:lane_id",
            get(navigation_lane),
        )
        .route(
            "/swarm/model-lanes/navigation/messages/:message_id",
            get(navigation_message),
        )
        .route(
            "/swarm/model-lanes/navigation/artifacts",
            get(navigation_artifact_context),
        )
        .route(
            "/swarm/model-lanes/navigation/traces/:trace_id",
            get(navigation_trace),
        )
        .route(
            "/swarm/model-lanes/navigation/diagnostics/:run_id",
            get(navigation_diagnostics),
        )
        .route(
            "/swarm/model-lanes/navigation/recovery/:run_id",
            get(navigation_recovery),
        )
        .route(
            "/swarm/model-lanes/navigation/lookup",
            get(navigation_lookup),
        )
        .with_state(state)
}

fn artifact_selector(query: &ArtifactNavigationQuery) -> Result<Option<String>, ModelLaneError> {
    let values = [
        query.artifact_ref.as_deref(),
        query.artifact_binding_id.as_deref(),
        query.artifact_manifest_ref.as_deref(),
        query.artifact_payload_ref.as_deref(),
        query.artifact_sha256.as_deref(),
        query.content_hash.as_deref(),
    ]
    .into_iter()
    .flatten()
    .filter(|value| !value.trim().is_empty())
    .map(str::to_owned)
    .collect::<Vec<_>>();
    let unique = values
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>();
    match unique.len() {
        0 => Ok(None),
        1 => Ok(unique.into_iter().next()),
        _ => Err(ModelLaneError::InvalidInput(
            "artifact navigation accepts one artifact selector value; use artifact_ref, artifact_binding_id, artifact_manifest_ref, artifact_payload_ref, artifact_sha256, or content_hash"
                .into(),
        )),
    }
}
