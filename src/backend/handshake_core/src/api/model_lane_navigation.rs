use axum::{
    extract::{Path, Query, State, Request},
    http::StatusCode,
    routing::get,
    Json, Router, Extension,
    middleware::{self, Next},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::api::account_scope::{RequestAccountScope, ProductLocalResourceScope};
use crate::swarm_orchestration::resource_scope::{
    ResourceAccessLifecycleDecision, ResourceAccessLifecycleRegistry, ResourceAccessLifecycleState,
};
use crate::storage::surreal::resource_authority::{RecordUserScope, ResourceAction, ResourceKind};
use surrealdb::types::SurrealValue;
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
fn store(state: &AppState, scope: &RequestAccountScope, lifecycle: ResourceAccessLifecycleRegistry) -> ModelLaneStore {
    ModelLaneStore::new_scoped_with_lifecycle(state.surreal.clone(), scope.resource_scope(), lifecycle)
}

#[derive(SurrealValue)]
struct NavigationGrantBindings { workspace_id: String }
#[derive(SurrealValue)]
struct NavigationGrant { grant_id: String, resource_id: String }

// Select an existing canonical grant as the native record user, without the
// audit-writing authorization helper. The provider revalidates this original
// envelope inside the read transaction before and after acquiring the projection.
async fn navigation_authority(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let unavailable = || (StatusCode::SERVICE_UNAVAILABLE, Json(json!({"error": "navigation_authority_unavailable"}))).into_response();
    let denied = || (StatusCode::NOT_FOUND, Json(json!({"error": "not_found"}))).into_response();
    let Some(scope) = request.extensions().get::<ProductLocalResourceScope>().cloned() else { return unavailable(); };
    let Some(lifecycle) = request.extensions().get::<ResourceAccessLifecycleRegistry>().cloned() else { return unavailable(); };
    let session = match crate::api::authority::authenticated_session_credentials(&state, request.headers()).await {
        Ok(session) => session, Err(denial) => return denial.into_response(),
    };
    let exact = scope.exact();
    if session.context.identity.account_id != exact.owner_account_id.to_string()
        || session.context.identity.principal_id != exact.actor_principal_id.to_string()
        || session.context.session_id != exact.authenticated_session_id.to_string()
        || session.context.identity.access_space_id != exact.access_space_id.to_string() {
        return denied();
    }
    // The shared lifecycle distinguishes a foreign workspace from a denied
    // canonical grant; it never substitutes for durable grant validation.
    match lifecycle.decision(exact) {
        Ok(ResourceAccessLifecycleDecision::Unknown) => return denied(),
        Ok(ResourceAccessLifecycleDecision::Known { state: ResourceAccessLifecycleState::Active, .. }) => {},
        Ok(ResourceAccessLifecycleDecision::Known { .. }) => return crate::api::authority::constant_denial().into_response(),
        Err(_) => return unavailable(),
    }
    if !crate::api::authority::session_holds_capability(&session.context, "model_lane.navigation.read") {
        return crate::api::authority::constant_denial().into_response();
    }
    let workspace_id = exact.workspace_id.as_str().to_owned();
    let mut original = RecordUserScope { grant_id: None, workspace_id: Some(workspace_id.clone()),
        resource_id: workspace_id.clone(), session_id: session.context.session_id,
        session_token: session.session_token, channel_binding_hash: Some(session.channel_binding_hash),
        capability_id: "model_lane.navigation.read".into(), action: ResourceAction::Read };
    let rows = state.surreal.with_record_user_scope(original.clone(), state.surreal.with_data_operation(|database| Box::pin(async move {
        database.query_values::<NavigationGrant, _>(
            "SELECT record::id(id) AS grant_id, record::id(resource_id) AS resource_id FROM resource_grants WHERE account_id = $auth.account_id AND principal_id = $auth.principal_id AND access_space_id = $auth.access_space_id AND resource_id.resource_kind = 'workspace' AND resource_id.external_resource_id = $workspace_id AND resource_id.lifecycle_state = 'active' AND resource_id.owner_account_id = $auth.account_id AND resource_id.access_space_id = $auth.access_space_id AND status = 'active' AND revoked_at = NONE AND (expires_at = NONE OR expires_at > time::now()) AND actions CONTAINS 'read' AND capability_ids CONTAINS 'model_lane.navigation.read' AND delegation_chain = $auth.delegation_chain ORDER BY id ASC LIMIT 2;",
            NavigationGrantBindings { workspace_id },
        ).await
    }))).await;
    let grant = match rows { Ok(mut rows) if rows.len() == 1 => rows.remove(0), _ => return crate::api::authority::constant_denial().into_response() };
    original.grant_id = Some(grant.grant_id); original.resource_id = grant.resource_id;
    crate::storage::surreal::with_model_lane_subject(
        crate::storage::surreal::ModelLaneOperationSubject { kind: ResourceKind::Workspace, external_id: exact.workspace_id.as_str().to_owned() },
        state.surreal.with_record_user_scope(original, next.run(request)),
    ).await
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
        ModelLaneError::AuthorityDenied(_) =>
            (StatusCode::NOT_FOUND, Json(json!({"error": "not_found"}))),
        ModelLaneError::Surreal(crate::storage::surreal::SurrealStorageError::TransactionAuthorityRequired) =>
            crate::api::authority::constant_denial(),
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
    Extension(lifecycle): Extension<ResourceAccessLifecycleRegistry>,
    Path(run_id): Path<String>,
) -> Result<Json<ModelLaneNavigationProjection>, ApiError> {
    store(&state, &scope, lifecycle)
        .navigation_by_run(&run_id)
        .await
        .map(Json)
        .map_err(model_lane_api_error)
}

async fn navigation_lane(
    State(state): State<AppState>,
    scope: RequestAccountScope,
    Extension(lifecycle): Extension<ResourceAccessLifecycleRegistry>,
    Path(lane_id): Path<String>,
) -> Result<Json<ModelLaneNavigationProjection>, ApiError> {
    store(&state, &scope, lifecycle)
        .navigation_by_lane(&lane_id)
        .await
        .map(Json)
        .map_err(model_lane_api_error)
}

async fn navigation_message(
    State(state): State<AppState>,
    scope: RequestAccountScope,
    Extension(lifecycle): Extension<ResourceAccessLifecycleRegistry>,
    Path(message_id): Path<String>,
) -> Result<Json<ModelLaneNavigationProjection>, ApiError> {
    store(&state, &scope, lifecycle)
        .navigation_by_message(&message_id)
        .await
        .map(Json)
        .map_err(model_lane_api_error)
}

async fn navigation_artifact_context(
    State(state): State<AppState>,
    scope: RequestAccountScope,
    Extension(lifecycle): Extension<ResourceAccessLifecycleRegistry>,
    Query(query): Query<ArtifactNavigationQuery>,
) -> Result<Json<ModelLaneNavigationProjection>, ApiError> {
    let artifact_ref = artifact_selector(&query).map_err(model_lane_api_error)?;
    store(&state, &scope, lifecycle)
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
    Extension(lifecycle): Extension<ResourceAccessLifecycleRegistry>,
    Path(trace_id): Path<String>,
    Query(query): Query<TraceNavigationQuery>,
) -> Result<Json<ModelLaneNavigationProjection>, ApiError> {
    store(&state, &scope, lifecycle)
        .navigation_by_trace(&trace_id, query.span_id.as_deref())
        .await
        .map(Json)
        .map_err(model_lane_api_error)
}

async fn navigation_diagnostics(
    State(state): State<AppState>,
    scope: RequestAccountScope,
    Extension(lifecycle): Extension<ResourceAccessLifecycleRegistry>,
    Path(run_id): Path<String>,
    Query(query): Query<DiagnosticNavigationQuery>,
) -> Result<Json<ModelLaneNavigationProjection>, ApiError> {
    store(&state, &scope, lifecycle)
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
    Extension(lifecycle): Extension<ResourceAccessLifecycleRegistry>,
    Path(run_id): Path<String>,
) -> Result<Json<ModelLaneNavigationProjection>, ApiError> {
    store(&state, &scope, lifecycle)
        .navigation_by_recovery(&run_id)
        .await
        .map(Json)
        .map_err(model_lane_api_error)
}

async fn navigation_lookup(
    State(state): State<AppState>,
    scope: RequestAccountScope,
    Extension(lifecycle): Extension<ResourceAccessLifecycleRegistry>,
    Query(query): Query<ModelLaneNavigationLookup>,
) -> Result<Json<ModelLaneNavigationProjection>, ApiError> {
    store(&state, &scope, lifecycle)
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
        .route_layer(middleware::from_fn_with_state(state.clone(), navigation_authority))
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
