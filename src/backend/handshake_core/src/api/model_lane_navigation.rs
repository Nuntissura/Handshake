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
    ResourceAccessGrantAssociation, ResourceAccessLifecycleDecision, ResourceAccessLifecycleRegistry, ResourceAccessLifecycleState,
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
struct NavigationGrantBindings { resource_id: surrealdb::types::RecordId }
#[derive(SurrealValue)]
struct NavigationGrant { grant_id: String, resource_id: String, grant_version: i64, policy_version: i64, capability_profile_id: String }

// Select an existing canonical grant as the native record user, without the
// audit-writing authorization helper. The provider revalidates this original
// envelope inside the read transaction before and after acquiring the projection.
pub(crate) async fn resolve_navigation_authorization(
    surreal: &crate::storage::surreal::SurrealStorage,
    capability_registry: &crate::capabilities::CapabilityRegistry,
    session: crate::api::authority::AuthenticatedLocalSession,
    workspace_id: &str,
    lifecycle: &ResourceAccessLifecycleRegistry,
) -> Result<(RecordUserScope, ResourceAccessGrantAssociation), ApiError> {
    let unavailable = || (StatusCode::SERVICE_UNAVAILABLE, Json(json!({"error": "navigation_authority_unavailable"})));
    let denied = || (StatusCode::NOT_FOUND, Json(json!({"error": "not_found"})));
    if *lifecycle != surreal.resource_access_lifecycle() { return Err(unavailable()); }
    let locator = match surreal.canonical_resource_locator(ResourceKind::Workspace, workspace_id).await {
        Ok(Some(locator)) => locator, Ok(None) => return Err(denied()), Err(_) => return Err(unavailable()),
    };
    if locator.owner_account_id != surrealdb::types::RecordId::new("local_accounts", session.context.identity.account_id.clone())
        || locator.access_space_id != surrealdb::types::RecordId::new("access_spaces", session.context.identity.access_space_id.clone())
        || locator.external_resource_id != workspace_id {
        return Err(denied());
    }
    // Construct authority from the authenticated native identity and located
    // canonical workspace; the server selection above only asserts equality.
    let canonical = (|| {
        use crate::swarm_orchestration::resource_scope::{OwnerAccountId, ActorPrincipalId, AuthenticatedSessionRef, AccessSpaceRef, WorkspaceScopeRef, ResourceScope, ExactResourceScopeAttribution};
        let scope = ResourceScope::new(
            OwnerAccountId::from_uuid(uuid::Uuid::parse_str(&session.context.identity.account_id).ok()?),
            ActorPrincipalId::from_uuid(uuid::Uuid::parse_str(&session.context.identity.principal_id).ok()?),
        ).with_session(AuthenticatedSessionRef::from_uuid(uuid::Uuid::parse_str(&session.context.session_id).ok()?))
            .with_access_space(AccessSpaceRef::from_uuid(uuid::Uuid::parse_str(&session.context.identity.access_space_id).ok()?))
            .with_workspace(WorkspaceScopeRef::new(locator.external_resource_id.clone()).ok()?);
        ExactResourceScopeAttribution::try_from_resource_scope(&scope).ok()
    })();
    let Some(canonical) = canonical else { return Err(crate::api::authority::constant_denial()); };
    let exact = &canonical;
    match lifecycle.decision(exact) {
        Ok(ResourceAccessLifecycleDecision::Unknown) => {},
        Ok(ResourceAccessLifecycleDecision::Known { state: ResourceAccessLifecycleState::Active, .. }) => {},
        Ok(ResourceAccessLifecycleDecision::Known { .. }) => return Err(crate::api::authority::constant_denial()),
        Err(_) => return Err(unavailable()),
    }
    if !crate::api::authority::session_holds_capability(&session.context, "model_lane.navigation.read") {
        return Err(crate::api::authority::constant_denial());
    }
    let workspace_id = exact.workspace_id.as_str().to_owned();
    let mut original = RecordUserScope { grant_id: None, workspace_id: Some(workspace_id.clone()),
        resource_id: workspace_id.clone(), session_id: session.context.session_id,
        session_token: session.session_token, channel_binding_hash: Some(session.channel_binding_hash),
        capability_id: "model_lane.navigation.read".into(), action: ResourceAction::Read };
    let captured_grants = match lifecycle.capture_grants(exact, "navigation_read") {
        Ok(captured) => captured, Err(_) => return Err(unavailable()),
    };
    let rows = surreal.with_record_user_scope(original.clone(), surreal.with_data_operation(|database| Box::pin(async move {
        database.query_values::<NavigationGrant, _>(
            "SELECT record::id(id) AS grant_id, record::id(resource_id) AS resource_id, grant_version, policy_version, principal_id.capability_profile_id AS capability_profile_id FROM resource_grants WHERE account_id = $auth.account_id AND principal_id = $auth.principal_id AND access_space_id = $auth.access_space_id AND resource_id = $resource_id AND resource_id.lifecycle_state = 'active' AND resource_id.owner_account_id = $auth.account_id AND resource_id.access_space_id = $auth.access_space_id AND status = 'active' AND revoked_at = NONE AND (expires_at = NONE OR expires_at > time::now()) AND actions CONTAINS 'read' AND capability_ids CONTAINS 'model_lane.navigation.read' AND delegation_chain = $auth.delegation_chain AND policy_version <= $auth.policy_version AND resource_id.policy_version <= policy_version AND resource_id.created_by_principal_id.status = 'enabled' AND $auth.principal_id.delegated_capabilities CONTAINS 'model_lane.navigation.read' ORDER BY id ASC LIMIT 2;",
            NavigationGrantBindings { resource_id: locator.id },
        ).await
    }))).await;
    let mut rows = match rows {
        Ok(rows) => rows,
        Err(crate::storage::surreal::SurrealStorageError::StatementBudgetExhausted { .. }
            | crate::storage::surreal::SurrealStorageError::StatementTimeout { .. }
            | crate::storage::surreal::SurrealStorageError::Closed
            | crate::storage::surreal::SurrealStorageError::ResourceAuthorityUnavailable) => return Err(unavailable()),
        Err(crate::storage::surreal::SurrealStorageError::TransactionAuthorityRequired) =>
            return Err(crate::api::authority::constant_denial()),
        Err(crate::storage::surreal::SurrealStorageError::Database(error)) if matches!(
            error.not_allowed_details(),
            Some(surrealdb::types::NotAllowedError::Auth(
                surrealdb::types::AuthError::TokenExpired | surrealdb::types::AuthError::SessionExpired
                | surrealdb::types::AuthError::InvalidAuth | surrealdb::types::AuthError::MissingUserOrPass
                | surrealdb::types::AuthError::InvalidPass | surrealdb::types::AuthError::NotAllowed { .. }
            ))
        ) => return Err(crate::api::authority::constant_denial()),
        Err(error) => return Err(model_lane_api_error(ModelLaneError::Surreal(error))),
    };
    let eligible: Vec<_> = rows.iter().map(|grant| (grant.grant_id.clone(), grant.grant_version, grant.policy_version)).collect();
    // LIMIT 2 is an ambiguity probe; two rows are not complete membership evidence.
    if rows.len() < 2 && lifecycle.observe_grants(&captured_grants, &eligible).is_err() { return Err(unavailable()); }
    let grant = match rows.len() { 1 => rows.remove(0), _ => return Err(crate::api::authority::constant_denial()) };
    if !capability_registry.profile_can(&grant.capability_profile_id, "model_lane.navigation.read").unwrap_or(false) {
        return Err(crate::api::authority::constant_denial());
    }
    let association = ResourceAccessGrantAssociation { exact: exact.clone(), resource_id: grant.resource_id.clone(),
        grant_id: grant.grant_id.clone(), grant_version: grant.grant_version, policy_version: grant.policy_version,
        capability_id: "model_lane.navigation.read".into(), action: "read".into(), purpose: "navigation_read".into() };
    match lifecycle.register_authorization(association.clone()) {
        Ok(()) => {},
        Err(crate::swarm_orchestration::resource_scope::ResourceAccessLifecycleTransitionError::AuthorityUnavailable) => return Err(unavailable()),
        Err(_) => return Err(crate::api::authority::constant_denial()),
    }
    original.grant_id = Some(grant.grant_id); original.resource_id = grant.resource_id;
    Ok((original, association))
}

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
    let (original, association) = match resolve_navigation_authorization(
        &state.surreal,
        &state.capability_registry,
        session,
        exact.workspace_id.as_str(),
        &lifecycle,
    ).await {
        Ok(authorization) => authorization,
        Err(error) => return error.into_response(),
    };
    let response = crate::storage::surreal::RESOURCE_ACCESS_AUTHORIZATION.scope(association.clone(), crate::storage::surreal::with_model_lane_subject(
        crate::storage::surreal::ModelLaneOperationSubject { kind: ResourceKind::Workspace, external_id: association.exact.workspace_id.as_str().to_owned() },
        state.surreal.with_record_user_scope(original, next.run(request)),
    )).await;
    match lifecycle.authorize_association(&association) {
        Ok(()) => response,
        Err(crate::swarm_orchestration::resource_scope::ScopeDenied::LifecycleAuthorityUnavailable) => unavailable(),
        Err(_) => crate::api::authority::constant_denial().into_response(),
    }
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
        ModelLaneError::Surreal(crate::storage::surreal::SurrealStorageError::ResourceAuthorityUnavailable) =>
            (StatusCode::SERVICE_UNAVAILABLE, Json(json!({"error": "navigation_authority_unavailable"}))),
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
