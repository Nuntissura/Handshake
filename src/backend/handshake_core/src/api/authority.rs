//! Shared authenticated identity and exact protected-resource authorization.
//!
//! Native process binding proves only the local transport channel. Persisted account, principal,
//! session, access-space, capability, and ResourceGrant state are resolved independently by the
//! Surreal-backed ResourceBroker before any protected product query is allowed.

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::post,
    Json, Router,
};
use chrono::Duration;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
#[cfg(test)]
use sha2::{Digest, Sha256};

use crate::kernel::KernelActor;
use crate::storage::surreal::resource_authority::{
    AuthorizationRequest, IssuedSession, ProvisionedPrincipal, RecordUserScope, ResourceAction,
    ResourceGrantSpec, ResourceKind,
};
use crate::AppState;

const HSK_HEADER_SESSION_TOKEN: &str = "x-hsk-session-token";
const LOCAL_SESSION_TTL_HOURS: i64 = 12;

#[derive(Clone, Debug)]
pub(crate) struct AuthorizedResourceContext {
    pub(crate) decision_id: String,
    pub(crate) account_id: String,
    pub(crate) principal_id: String,
    pub(crate) session_id: String,
    pub(crate) access_space_id: String,
    pub(crate) resource_id: String,
    pub(crate) actor_kind: String,
    pub(crate) actor_id: String,
    pub(crate) capability_profile_id: String,
    pub(crate) capability_id: String,
    pub(crate) delegation_chain: Vec<String>,
    pub(crate) record_user_scope: RecordUserScope,
}

#[derive(Clone, Debug)]
pub(crate) struct ReconciliationAuthority {
    session: IssuedSession,
    pub(crate) record_user_scope: RecordUserScope,
}

pub(crate) async fn reconciliation_authority(
    state: &AppState,
    capability_id: &'static str,
) -> Result<ReconciliationAuthority, String> {
    let principal = state
        .surreal
        .issue_reconciliation_session()
        .await
        .map_err(|error| error.to_string())?;
    let decision = state
        .surreal
        .authorize_protected_resource(AuthorizationRequest {
            session_token: principal.session.token.clone(),
            channel_binding_hash: None,
            capability_id: capability_id.to_owned(),
            resource_kind: ResourceKind::ReconciliationQueue,
            external_resource_id: "mt109-protected-reconciliation".to_owned(),
            action: ResourceAction::Reconcile,
        })
        .await
        .map_err(|error| error.to_string())?;
    Ok(ReconciliationAuthority {
        record_user_scope: RecordUserScope {
            workspace_id: None,
            session_token: principal.session.token.clone(),
            channel_binding_hash: None,
            resource_id: decision.resource_id,
            session_id: decision.session_id,
            capability_id: capability_id.to_owned(),
            action: ResourceAction::Reconcile,
        },
        session: principal.session,
    })
}

pub(crate) async fn authorize_reconciliation_workspace(
    state: &AppState,
    authority: &ReconciliationAuthority,
    workspace_id: &str,
    capability_id: &'static str,
) -> Result<RecordUserScope, String> {
    state
        .surreal
        .authorize_protected_resource(AuthorizationRequest {
            session_token: authority.session.token.clone(),
            channel_binding_hash: None,
            capability_id: capability_id.to_owned(),
            resource_kind: ResourceKind::ReconciliationQueue,
            external_resource_id: format!(
                "{}:{workspace_id}",
                crate::storage::surreal::resource_authority::RECONCILIATION_QUEUE_ID
            ),
            action: ResourceAction::Reconcile,
        })
        .await
        .map(|decision| RecordUserScope {
            workspace_id: Some(workspace_id.to_owned()),
            session_token: authority.session.token.clone(),
            channel_binding_hash: None,
            resource_id: decision.resource_id,
            session_id: decision.session_id,
            capability_id: capability_id.to_owned(),
            action: ResourceAction::Reconcile,
        })
        .map_err(|error| error.to_string())
}

impl AuthorizedResourceContext {
    pub(crate) fn capture_context(&self) -> crate::api::stage::CaptureContext {
        crate::api::stage::CaptureContext {
            actor_kind: self.actor_kind.clone(),
            actor_id: self.actor_id.clone(),
            limiter_principal: self.principal_id.clone(),
            actor: KernelActor::Operator(self.actor_id.clone()),
            kernel_task_run_id: format!("authority-task:{}", self.decision_id),
            session_run_id: self.session_id.clone(),
            // A channel credential is never copied into the authenticated identity context.
            binding_token: String::new(),
        }
    }
}

#[derive(Debug, Serialize)]
struct LocalSessionResponse {
    schema_version: &'static str,
    session_token: String,
    account_id: String,
    principal_id: String,
    session_id: String,
    access_space_id: String,
    expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LocalSessionExchange {
    account_id: String,
    principal_id: String,
    access_space_id: String,
    authentication_token: String,
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/authority/session", post(exchange_local_session))
        .with_state(state)
}

pub(crate) fn constant_denial() -> (StatusCode, Json<Value>) {
    (
        StatusCode::FORBIDDEN,
        Json(json!({"error": "HSK-403-PROTECTED-RESOURCE"})),
    )
}

pub(crate) async fn authorize_request(
    state: &AppState,
    headers: &HeaderMap,
    capability_id: &'static str,
    resource_kind: ResourceKind,
    external_resource_id: &str,
    action: ResourceAction,
) -> Result<AuthorizedResourceContext, (StatusCode, Json<Value>)> {
    let session_token = headers
        .get(HSK_HEADER_SESSION_TOKEN)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .ok_or_else(constant_denial)?
        .to_owned();
    let channel =
        crate::api::stage::capture_channel_binding(headers).map_err(|_| constant_denial())?;
    let channel_binding_hash = channel.binding_hash;
    let decision = state
        .surreal
        .authorize_protected_resource(AuthorizationRequest {
            session_token: session_token.clone(),
            channel_binding_hash: Some(channel_binding_hash.clone()),
            capability_id: capability_id.to_owned(),
            resource_kind,
            external_resource_id: external_resource_id.to_owned(),
            action,
        })
        .await
        .map_err(|error| {
            tracing::warn!(
                target: "handshake_core::resource_authority",
                error = %error,
                capability_id,
                "protected_resource_authorization_denied"
            );
            constant_denial()
        })?;

    if !state
        .capability_registry
        .profile_can(&decision.capability_profile_id, capability_id)
        .unwrap_or(false)
    {
        return Err(constant_denial());
    }

    Ok(AuthorizedResourceContext {
        decision_id: decision.decision_id,
        account_id: decision.account_id,
        principal_id: decision.principal_id,
        session_id: decision.session_id.clone(),
        access_space_id: decision.access_space_id,
        resource_id: decision.resource_id.clone(),
        actor_kind: decision.actor_kind,
        actor_id: decision.actor_id,
        capability_profile_id: decision.capability_profile_id,
        capability_id: capability_id.to_owned(),
        delegation_chain: decision.delegation_chain,
        record_user_scope: RecordUserScope {
            workspace_id: Some(external_resource_id.to_owned()),
            session_token,
            channel_binding_hash: Some(channel_binding_hash),
            resource_id: decision.resource_id,
            session_id: decision.session_id,
            capability_id: capability_id.to_owned(),
            action,
        },
    })
}

async fn exchange_local_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(exchange): Json<LocalSessionExchange>,
) -> Result<Json<LocalSessionResponse>, (StatusCode, Json<Value>)> {
    let channel =
        crate::api::stage::capture_channel_binding(&headers).map_err(|_| constant_denial())?;
    let session = state
        .surreal
        .exchange_session_credential(
            &exchange.account_id,
            &exchange.principal_id,
            &exchange.access_space_id,
            &exchange.authentication_token,
            &channel.binding_hash,
            std::time::Duration::from_secs(LOCAL_SESSION_TTL_HOURS as u64 * 60 * 60),
        )
        .await
        .map_err(|error| {
            tracing::error!(
                target: "handshake_core::resource_authority",
                error = %error,
                "local_authority_session_bootstrap_failed"
            );
            constant_denial()
        })?;
    Ok(Json(LocalSessionResponse {
        schema_version: "hsk.authenticated_session@1",
        session_token: session.token,
        account_id: session.account_id,
        principal_id: session.principal_id,
        session_id: session.session_id,
        access_space_id: session.access_space_id,
        expires_at: session.expires_at,
    }))
}

#[cfg(test)]
async fn provision_existing_workspace_grants(
    state: &AppState,
    principal: &ProvisionedPrincipal,
) -> Result<(), (StatusCode, Json<Value>)> {
    let workspaces = state.storage.list_workspaces().await.map_err(|error| {
        tracing::error!(target: "handshake_core::resource_authority", error = %error, "workspace_authority_enumeration_failed");
        constant_denial()
    })?;
    for workspace in workspaces {
        let workspace_resource = state
            .surreal
            .register_workspace_resource(&principal.identity, &workspace.id)
            .await
            .map_err(|error| {
                tracing::error!(target: "handshake_core::resource_authority", error = %error, "workspace_resource_registration_failed");
                constant_denial()
            })?;
        grant(
            state,
            principal,
            workspace_resource.resource_id.clone(),
            vec![
                ResourceAction::Read,
                ResourceAction::Create,
                ResourceAction::Update,
            ],
            vec![
                "fr.read",
                "fr.ingest.runtime_chat",
                "fr.ingest.native_editor",
                "memory.read",
                "memory.propose",
                "memory.review",
                "memory.commit",
            ],
        )
        .await?;

        let surfaces = [
            (
                ResourceKind::FlightRecorder,
                vec![ResourceAction::Read, ResourceAction::Create],
                vec![
                    "fr.read",
                    "fr.ingest.runtime_chat",
                    "fr.ingest.native_editor",
                ],
            ),
            (
                ResourceKind::MemoryPack,
                vec![ResourceAction::Read],
                vec!["memory.read"],
            ),
            (
                ResourceKind::MemoryProposal,
                vec![
                    ResourceAction::Read,
                    ResourceAction::Create,
                    ResourceAction::Update,
                ],
                vec![
                    "memory.read",
                    "memory.propose",
                    "memory.review",
                    "memory.commit",
                ],
            ),
            (
                ResourceKind::MemoryCommitReport,
                vec![ResourceAction::Read, ResourceAction::Create],
                vec!["memory.read", "memory.commit"],
            ),
            (
                ResourceKind::MemoryItem,
                vec![
                    ResourceAction::Read,
                    ResourceAction::Create,
                    ResourceAction::Update,
                ],
                vec!["memory.read", "memory.commit"],
            ),
            (
                ResourceKind::MemoryItemCount,
                vec![ResourceAction::Read, ResourceAction::Create],
                vec!["memory.read", "memory.commit"],
            ),
        ];
        for (kind, actions, capabilities) in surfaces {
            let resource = state
                .surreal
                .register_protected_resource(
                    &principal.identity,
                    kind,
                    &workspace.id,
                    Some(&workspace_resource.resource_id),
                    "account_private",
                )
                .await
                .map_err(|error| {
                    tracing::error!(target: "handshake_core::resource_authority", error = %error, "protected_resource_registration_failed");
                    constant_denial()
                })?;
            grant(
                state,
                principal,
                resource.resource_id,
                actions,
                capabilities,
            )
            .await?;
        }
    }
    Ok(())
}

#[cfg(test)]
pub(crate) async fn test_principal_for_binding(
    state: &AppState,
    binding_token: &str,
) -> Result<ProvisionedPrincipal, String> {
    let binding_hash = hex::encode(Sha256::digest(binding_token.as_bytes()));
    let principal = state
        .surreal
        .provision_local_operator(Some(&binding_hash))
        .await
        .map_err(|error| error.to_string())?;
    provision_existing_workspace_grants(state, &principal)
        .await
        .map_err(|(_, body)| body.0.to_string())?;
    Ok(principal)
}

#[cfg(test)]
pub(crate) async fn test_session_for_binding(
    state: &AppState,
    binding_token: &str,
) -> Result<String, String> {
    Ok(test_principal_for_binding(state, binding_token)
        .await?
        .session
        .token)
}

#[cfg(test)]
async fn grant(
    state: &AppState,
    principal: &ProvisionedPrincipal,
    resource_id: String,
    actions: Vec<ResourceAction>,
    capability_ids: Vec<&str>,
) -> Result<(), (StatusCode, Json<Value>)> {
    state
        .surreal
        .grant_resource(
            &principal.identity.account_id,
            &principal.identity.access_space_id,
            ResourceGrantSpec {
                principal_id: principal.identity.principal_id.clone(),
                resource_id,
                actions,
                capability_ids: capability_ids.into_iter().map(str::to_owned).collect(),
                expires_at: Some(chrono::Utc::now() + Duration::hours(LOCAL_SESSION_TTL_HOURS)),
                delegation_chain: Vec::new(),
            },
        )
        .await
        .map(|_| ())
        .map_err(|error| {
            tracing::error!(target: "handshake_core::resource_authority", error = %error, "resource_grant_provision_failed");
            constant_denial()
        })
}
