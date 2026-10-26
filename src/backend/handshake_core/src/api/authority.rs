//! Shared authenticated identity and exact protected-resource authorization.
//!
//! Native process binding proves only the local transport channel. Persisted account, principal,
//! session, access-space, capability, and ResourceGrant state are resolved independently by the
//! Surreal-backed ResourceBroker before any protected product query is allowed.

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use chrono::Duration;
use handshake_account_activation::WorkspaceActivationInput;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
#[cfg(test)]
use sha2::{Digest, Sha256};

use crate::kernel::KernelActor;
use crate::storage::surreal::resource_authority::{
    AuthorizationRequest, IssuedSession, ProvisionedPrincipal, RecordUserScope, ResourceAction,
    ResourceGrantSpec, ResourceKind,
};
use crate::{AccountBootstrapState, AppState};

pub(crate) async fn configured_registry_context(
    State(state): State<AppState>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    crate::storage::surreal::with_configured_capability_registry(
        state.capability_registry.clone(), next.run(request),
    ).await
}

const HSK_HEADER_SESSION_TOKEN: &str = "x-hsk-session-token";
const LOCAL_SESSION_TTL_HOURS: i64 = 12;
#[cfg(feature = "os-keychain")]
static SESSION_VAULT_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[cfg(feature = "os-keychain")]
async fn run_session_vault_operation<T: Send + 'static>(
    operation: impl FnOnce() -> T + Send + 'static,
) -> Result<T, (StatusCode, Json<Value>)> {
    let guard = SESSION_VAULT_LOCK.lock().await;
    tokio::task::spawn_blocking(move || {
        let _guard = guard;
        operation()
    })
    .await
    .map_err(|error| {
        #[cfg(test)]
        eprintln!("LOCAL_ACCOUNT_TEST_FAILURE phase=vault_worker error={error}");
        tracing::error!(target: "handshake_core::resource_authority",
            phase = "vault_worker", cancelled = error.is_cancelled(), panicked = error.is_panic(),
            "local_authority_session_bootstrap_failed");
        constant_denial()
    })
}

#[cfg(test)]
struct AccountLifecycleProbe {
    path: std::path::PathBuf,
    phase: &'static str,
    entered: tokio::sync::oneshot::Sender<()>,
    release: tokio::sync::oneshot::Receiver<()>,
    fail: bool,
}
#[cfg(test)]
static ACCOUNT_LIFECYCLE_PROBE: std::sync::Mutex<Option<AccountLifecycleProbe>> =
    std::sync::Mutex::new(None);
#[cfg(test)]
async fn account_lifecycle_probe(
    state: &AccountBootstrapState,
    phase: &'static str,
) -> Result<(), (StatusCode, Json<Value>)> {
    let probe = {
        let mut slot = ACCOUNT_LIFECYCLE_PROBE.lock().unwrap();
        if slot.as_ref().is_some_and(|probe| {
            probe.path == state.surreal.config().path() && probe.phase == phase
        }) {
            slot.take()
        } else {
            None
        }
    };
    if let Some(probe) = probe {
        let _ = probe.entered.send(());
        let _ = probe.release.await;
        if probe.fail {
            return Err(constant_denial());
        }
    }
    Ok(())
}
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
            grant_id: Some(decision.grant_id.clone()),
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
            grant_id: Some(decision.grant_id.clone()),
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

#[derive(Serialize)]
struct LocalSessionResponse {
    schema_version: &'static str,
    session_token: String,
    account_id: String,
    principal_id: String,
    session_id: String,
    access_space_id: String,
    expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LocalSessionExchange {
    account_id: String,
    principal_id: String,
    access_space_id: String,
    authentication_token: String,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum LocalSessionOperation {
    Exchange(LocalSessionExchange),
    Activate(WorkspaceActivationInput),
}

pub struct WorkspaceActivationAcknowledgement {
    pub schema_version: &'static str,
    pub operation_id: uuid::Uuid,
    pub scope: crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution,
}

impl Serialize for WorkspaceActivationAcknowledgement {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let acknowledgement = handshake_account_activation::WorkspaceActivationAcknowledgement {
            operation_id: self.operation_id,
            owner_account_id: self.scope.owner_account_id.to_string(),
            actor_principal_id: self.scope.actor_principal_id.to_string(),
            authenticated_session_id: self.scope.authenticated_session_id.to_string(),
            access_space_id: self.scope.access_space_id.to_string(),
            workspace_id: self.scope.workspace_id.to_string(),
        };
        handshake_account_activation::serialize_workspace_activation_ack(&acknowledgement, self.schema_version, serializer)
    }
}

pub struct WorkspaceActivationRequest {
    state: AccountBootstrapState,
    headers: HeaderMap,
    scope: crate::api::account_scope::ProductLocalResourceScope,
    operation_id: uuid::Uuid,
    original_scope: RecordUserScope,
    activation_authority: crate::storage::surreal::resource_authority::CapturedActivationAuthority,
    association: crate::swarm_orchestration::resource_scope::ResourceAccessGrantAssociation,
    reply: tokio::sync::oneshot::Sender<Result<WorkspaceActivationAcknowledgement, (StatusCode, Json<Value>)>>,
}

impl WorkspaceActivationRequest {
    pub fn scope(&self) -> &crate::api::account_scope::ProductLocalResourceScope {
        &self.scope
    }

    pub fn operation_id(&self) -> uuid::Uuid {
        self.operation_id
    }

    pub async fn authorize_for_publication(&self) -> Result<(), (StatusCode, Json<Value>)> {
        let session = authenticated_session_credentials_bootstrap(&self.state, &self.headers).await?;
        let lifecycle = self.state.surreal.resource_access_lifecycle();
        let (scope, association) = crate::api::model_lane_navigation::resolve_navigation_authorization(
            &self.state.surreal, &self.state.capability_registry, session,
            self.scope.exact().workspace_id.as_str(), &lifecycle,
        ).await?;
        if association != self.association
            || scope.session_id != self.original_scope.session_id
            || scope.resource_id != self.original_scope.resource_id
            || scope.grant_id != self.original_scope.grant_id
            || scope.capability_id != self.original_scope.capability_id
            || scope.action != self.original_scope.action
        {
            return Err(constant_denial());
        }
        lifecycle.authorize_association(&self.association).map_err(|_| constant_denial())
    }

    pub async fn acknowledge_with<R, F>(self, runtime: &R, publication: F) -> Result<(), (StatusCode, Json<Value>)>
    where
        R: serde::Serialize + Sync,
        F: FnOnce() -> Result<(), (StatusCode, Json<Value>)> + Send,
    {
        use crate::storage::surreal::resource_authority::ResourceAuthorityError;
        let acknowledgement = WorkspaceActivationAcknowledgement {
            schema_version: "hsk.workspace_activation@1",
            operation_id: self.operation_id,
            scope: self.scope.exact().clone(),
        };
        let mut reply = Some(self.reply);
        let result = self.state.surreal.settle_activation_and_publish(
            &self.activation_authority, &self.state.capability_registry, runtime,
            || publication().map_err(|_| ResourceAuthorityError::InvalidInput("activation protected publication uncertain")),
            |_| {
                if let Some(reply) = reply.take() {
                    let _ = reply.send(Ok(acknowledgement));
                }
                Ok(())
            },
        ).await;
        if let Err(error) = result {
            tracing::warn!(error = ?error.cause, "workspace activation settlement remains uncertain");
            let error = activation_unavailable();
            if let Some(reply) = reply.take() { let _ = reply.send(Err(error.clone())); }
            return Err(error);
        }
        Ok(())
    }

    pub fn deny(self) {
        let _ = self.reply.send(Err(constant_denial()));
    }

    pub fn unavailable(self) {
        let _ = self.reply.send(Err(activation_unavailable()));
    }
}

fn activation_unavailable() -> (StatusCode, Json<Value>) {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({"error": "activation_unavailable"})),
    )
}

async fn session_operation(
    State(state): State<AccountBootstrapState>,
    axum::Extension(activation): axum::Extension<Option<tokio::sync::mpsc::Sender<WorkspaceActivationRequest>>>,
    headers: HeaderMap,
    Json(operation): Json<LocalSessionOperation>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match operation {
        LocalSessionOperation::Exchange(exchange) => {
            let Json(response) = exchange_local_session(State(state), headers, Json(exchange)).await?;
            serde_json::to_value(response).map(Json).map_err(|_| constant_denial())
        }
        LocalSessionOperation::Activate(input) => {
            if input.operation_id.is_nil() || input.workspace_id.trim().is_empty() {
                return Err(constant_denial());
            }
            let sender = activation.ok_or_else(activation_unavailable)?;
            let session = authenticated_session_credentials_bootstrap(&state, &headers).await?;
            let lifecycle = state.surreal.resource_access_lifecycle();
            let (original_scope, association) = crate::api::model_lane_navigation::resolve_navigation_authorization(
                &state.surreal, &state.capability_registry, session, &input.workspace_id, &lifecycle,
            ).await?;
            let scope = crate::api::account_scope::ProductLocalResourceScope::from_exact(association.exact.clone())
                .map_err(|_| constant_denial())?;
            let activation_authority = state.surreal.capture_activation_authority(
                input.operation_id, original_scope.clone(), association.clone(), &state.capability_registry,
            ).await.map_err(|error| match error {
                crate::storage::surreal::resource_authority::ResourceAuthorityError::Storage(_) => activation_unavailable(),
                _ => constant_denial(),
            })?;
            let (reply, result) = tokio::sync::oneshot::channel();
            sender.send(WorkspaceActivationRequest {
                state, headers, scope, operation_id: input.operation_id,
                original_scope, activation_authority, association, reply,
            }).await.map_err(|_| activation_unavailable())?;
            let acknowledgement = result.await.map_err(|_| activation_unavailable())??;
            serde_json::to_value(acknowledgement).map(Json).map_err(|_| constant_denial())
        }
    }
}

pub(crate) fn activation_observation_error_response(
    error: crate::storage::surreal::resource_authority::ActivationObservationError,
) -> (StatusCode, Json<Value>) {
    use crate::storage::surreal::resource_authority::ActivationObservationError;
    match error {
        ActivationObservationError::Authentication => constant_denial(),
        ActivationObservationError::NotFound => (StatusCode::NOT_FOUND, Json(json!({"error":"not_found"}))),
        ActivationObservationError::Unavailable => activation_unavailable(),
    }
}

async fn workspace_activation_observation(
    State(state): State<AccountBootstrapState>,
    Path((workspace_id, operation_id)): Path<(String, uuid::Uuid)>,
    mounted: Option<axum::Extension<crate::api::workspace_activation_readiness::MountedApiState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let session=authenticated_session_credentials_bootstrap(&state,&headers).await?;
    let axum::Extension(mounted)=mounted.ok_or_else(activation_unavailable)?;
    crate::api::workspace_activation_readiness::observe_workspace_activation(
        &mounted,&state,&session.session_token,&session.channel_binding_hash,&workspace_id,operation_id,
    ).await
}

pub fn routes(state: AppState) -> Router {
    account_routes(AccountBootstrapState::from(&state), None)
}

pub fn bootstrap_routes(
    state: AccountBootstrapState,
    activation: tokio::sync::mpsc::Sender<WorkspaceActivationRequest>,
) -> Router {
    account_routes(state, Some(activation))
}

fn account_routes(
    state: AccountBootstrapState,
    activation: Option<tokio::sync::mpsc::Sender<WorkspaceActivationRequest>>,
) -> Router {
    let router = Router::new()
        .route("/authority/setup", get(setup_status).post(setup_owner))
        .route("/authority/login", post(login_owner))
        .route("/authority/session", get(current_session).post(session_operation))
        .route("/authority/workspaces/{workspace_id}/activation/{operation_id}", get(workspace_activation_observation))
        .route("/authority/logout", post(logout))
        ;
    #[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
    let router = router
        .route("/authority/member-enrollments", post(issue_member_enrollment))
        .route("/authority/member-enrollments/:operation_id", get(observe_member_enrollment))
        .route("/authority/member-enrollments/:operation_id/redeem", post(redeem_member_enrollment))
        .route("/authority/member-enrollments/:operation_id/revoke", post(revoke_member_enrollment))
        .route("/authority/member-enrollment-administrator/confirmation", post(confirm_first_owner_administration));
    router.layer(axum::Extension(activation)).with_state(state)
}


// NEW authority.rs input boundary for the four selected enrollment routes.
// No route is mounted by this candidate. Account's server-current constructors,
// canonical recipient descriptor lookup and Update revocation are dependencies.
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
struct MemberEnrollmentSecret(zeroize::Zeroizing<String>);

#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl<'de> serde::Deserialize<'de> for MemberEnrollmentSecret {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = <String as serde::Deserialize>::deserialize(deserializer)?;
        Ok(Self(zeroize::Zeroizing::new(value)))
    }
}

#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl MemberEnrollmentSecret {
    fn into_zeroizing(self) -> zeroize::Zeroizing<String> {
        self.0
    }

    fn into_owned(self) -> String {
        let mut value = self.0;
        std::mem::take(&mut *value)
    }
}

#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct MemberEnrollmentIssueRequest {
    // Untrusted candidate operation UUID. Account admits the immutable
    // original once and compares complete selectors and credentials on replay.
    request_id: uuid::Uuid,
    account_name: String,
    workspace_id: String,
    administrator_resource: String,
    current_password: MemberEnrollmentSecret,
}

#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct MemberEnrollmentRedeemRequest {
    invitation_secret: MemberEnrollmentSecret,
    password: MemberEnrollmentSecret,
}

#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct MemberEnrollmentRevokeRequest {
    request_id: uuid::Uuid,
    workspace_id: String,
    administrator_resource: String,
    current_password: MemberEnrollmentSecret,
}

#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
fn member_enrollment_request<T>(
    request: Result<axum::Json<T>, axum::extract::rejection::JsonRejection>,
) -> Result<T, (axum::http::StatusCode, axum::Json<serde_json::Value>)> {
    match request {
        Ok(axum::Json(request)) => Ok(request),
        // Never display/debug/log decoder errors or reflect submitted values.
        Err(_) => Err(constant_denial()),
    }
}

// NEW retained HTTP caller owner for api::authority.rs.
// One instance per actual storage owner, supplied as a mandatory Extension.
// No public auth constructor, caller proof map, new credential or global registry.
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
mod member_enrollment_http_owner {
    use super::*;
    use crate::model_runtime::cloud::member_enrollment as invitation;
    use crate::storage::surreal::{SurrealStorage, resource_authority as account};
    use std::{collections::HashMap, future::Future, sync::{Arc, Mutex, OnceLock}};

    pub(crate) enum WorkerFailure<E> { Outcome(E), Join(tokio::task::JoinError), Unobserved }
    struct WorkerState<T, E> {
        handle: Option<tokio::task::JoinHandle<Result<T, E>>>,
        outcome: Option<Arc<Result<T, WorkerFailure<E>>>>,
    }
    pub(crate) struct Worker<T, E> { state: tokio::sync::Mutex<WorkerState<T, E>> }
    impl<T: Send + Sync + 'static, E: Send + Sync + 'static> Worker<T, E> {
        fn unstarted() -> Self {
            Self { state: tokio::sync::Mutex::new(WorkerState { handle: None, outcome: None }) }
        }
        fn start(&self, future: impl Future<Output = Result<T, E>> + Send + 'static) {
            let mut state = self.state.try_lock().expect("new retained HTTP worker");
            assert!(state.handle.is_none() && state.outcome.is_none());
            state.handle = Some(tokio::spawn(future));
        }
        pub(crate) async fn wait(&self) -> Arc<Result<T, WorkerFailure<E>>> {
            let mut state = self.state.lock().await;
            if let Some(outcome) = &state.outcome { return outcome.clone(); }
            let outcome = match state.handle.as_mut() {
                Some(handle) => match handle.await {
                    Ok(Ok(value)) => Ok(value),
                    Ok(Err(error)) => Err(WorkerFailure::Outcome(error)),
                    Err(error) => Err(WorkerFailure::Join(error)),
                },
                None => Err(WorkerFailure::Unobserved),
            };
            state.handle.take();
            let outcome = Arc::new(outcome);
            state.outcome = Some(outcome.clone());
            outcome
        }
    }

    struct IssueInput {
        operation: uuid::Uuid,
        account_name: String,
        workspace: String,
        administrator_resource: String,
        credentials: account::RoleAuthorityAuthInput,
        password: zeroize::Zeroizing<String>,
    }
    impl IssueInput {
        fn selectors(&self) -> account::MemberInvitationIssueSelectors {
            account::MemberInvitationIssueSelectors {
                operation_id: self.operation, account_name: self.account_name.clone(),
                workspace_id: self.workspace.clone(), administrator_resource: self.administrator_resource.clone(),
            }
        }
    }
    pub(crate) enum IssueFailure {
        Capture(account::MemberInvitationIssueSeedError),
        Admission(account::MemberInvitationIssueBoundaryError),
        DifferentOriginal { retained: invitation::Ticket },
        RegistryPoisoned { retained: invitation::Ticket },
    }
    pub(crate) struct IssueFrame {
        input: IssueInput,
        seed: Arc<account::MemberInvitationIssueSeed>,
        pub(crate) worker: Worker<invitation::Ticket, IssueFailure>,
        submission: OnceLock<invitation::OriginalSubmission>,
        delivery: OnceLock<Result<Arc<account::MemberInvitationDeliveryFrame>, Arc<invitation::AccountError>>>,
    }
    impl IssueFrame {
        pub(crate) fn operation_id(&self) -> uuid::Uuid { self.input.operation }
        fn matches(&self, storage: &SurrealStorage, input: &IssueInput) -> bool {
            self.seed.matches_redelivery(storage, &input.selectors(), &input.credentials)
                && self.input.password.as_str() == input.password.as_str()
        }
        async fn issue(self: Arc<Self>, owner: Arc<MemberEnrollmentHttpRuntime>)
            -> Result<invitation::Ticket, IssueFailure> {
            let captured = owner.storage.capture_member_invitation_submission(
                self.seed.clone(), owner.registry.clone(),
            ).await.map_err(IssueFailure::Capture)?;
            let original = captured.original().clone();
            let _ = self.submission.set(original.clone());
            let ticket = owner.storage.admit_original_member_invitation(
                &owner.lifecycle, owner.registry.clone(), self.input.credentials.clone(),
                original, self.input.password.as_str().to_owned(),
            ).await.map_err(IssueFailure::Admission)?;
            owner.note_issued(&self, &ticket)?;
            Ok(ticket)
        }
    }

    pub(crate) enum ReadOutput {
        Issuer(account::MemberInvitationMetadata),
        RevokeMetadata(account::MemberRevocationMetadata),
        RevokeReceipt(account::MemberRevocationObserved),
    }
    pub(crate) enum ReadFailure {
        Issuer(invitation::AccountError),
        Authority(account::ResourceAuthorityError),
        Deadline,
    }
    enum ReadInput {
        Issuer { operation: uuid::Uuid, credentials: account::RoleAuthorityAuthInput },
        RevokeMetadata { operation: uuid::Uuid, workspace: String, administrator_resource: String,
            credentials: account::RoleAuthorityAuthInput },
        RevokeReceipt { operation: uuid::Uuid, request: uuid::Uuid, workspace: String,
            administrator_resource: String, credentials: account::RoleAuthorityAuthInput },
    }
    pub(crate) struct ReadFrame {
        input: ReadInput,
        deadline: tokio::time::Instant,
        pub(crate) worker: Worker<ReadOutput, ReadFailure>,
    }
    impl ReadFrame {
        async fn read(self: Arc<Self>, owner: Arc<MemberEnrollmentHttpRuntime>)
            -> Result<ReadOutput, ReadFailure> {
            if tokio::time::Instant::now() >= self.deadline { return Err(ReadFailure::Deadline); }
            SurrealStorage::with_operation_deadline(self.deadline.into_std(), async {
                match &self.input {
                    ReadInput::Issuer { operation, credentials } =>
                        owner.storage.current_member_invitation_metadata(
                            *operation, owner.registry.clone(), credentials.clone(), self.deadline,
                        ).await.map(ReadOutput::Issuer).map_err(ReadFailure::Issuer),
                    ReadInput::RevokeMetadata { operation, workspace, administrator_resource, credentials } =>
                        owner.storage.current_member_revocation_metadata(
                            &owner.registry, credentials.clone(), *operation, workspace, administrator_resource,
                        ).await.map(ReadOutput::RevokeMetadata).map_err(ReadFailure::Authority),
                    ReadInput::RevokeReceipt { operation, request, workspace, administrator_resource, credentials } =>
                        owner.storage.observe_server_member_revocation(
                            &owner.registry, credentials.clone(), *request, *operation, workspace, administrator_resource,
                        ).await.map(ReadOutput::RevokeReceipt).map_err(ReadFailure::Authority),
                }
            }).await
        }
    }

    enum RejectionReason { Closing, Poisoned, DifferentOriginal, Seed(invitation::AccountError), Deadline }
    enum RejectedInput { Issue(IssueInput), Read(ReadInput) }
    pub(crate) struct Rejected { _input: RejectedInput, _reason: RejectionReason }
    struct Admissions {
        closing: bool,
        issues: HashMap<uuid::Uuid, Arc<IssueFrame>>,
        submissions: Vec<Arc<IssueFrame>>,
        reads: Vec<Arc<ReadFrame>>,
        deliveries: Vec<Arc<account::MemberInvitationDeliveryFrame>>,
        rejected: Vec<Arc<Rejected>>,
    }
    pub(crate) struct MemberEnrollmentHttpRuntime {
        storage: SurrealStorage,
        registry: Arc<crate::capabilities::CapabilityRegistry>,
        lifecycle: Arc<invitation::InvitationLifecycle>,
        admission: Mutex<Admissions>,
    }
    pub(crate) struct MemberEnrollmentHttpShutdownInventory {
        pub(crate) healthy: bool,
        pub(crate) issues: Vec<Arc<IssueFrame>>,
        pub(crate) reads: Vec<Arc<ReadFrame>>,
        pub(crate) deliveries: Vec<Arc<account::MemberInvitationDeliveryFrame>>,
    }
    impl MemberEnrollmentHttpRuntime {
        pub(crate) fn new(storage: SurrealStorage,
            registry: Arc<crate::capabilities::CapabilityRegistry>,
            lifecycle: Arc<invitation::InvitationLifecycle>) -> Arc<Self> {
            Arc::new(Self { storage, registry, lifecycle, admission: Mutex::new(Admissions {
                closing: false, issues: HashMap::new(), submissions: Vec::new(), reads: Vec::new(),
                deliveries: Vec::new(), rejected: Vec::new(),
            }) })
        }
        pub(crate) fn begin_shutdown(&self) -> MemberEnrollmentHttpShutdownInventory {
            let (mut gate, healthy) = match self.admission.lock() {
                Ok(gate) => (gate, true), Err(poisoned) => (poisoned.into_inner(), false),
            };
            gate.closing = true;
            MemberEnrollmentHttpShutdownInventory {
                healthy, issues: gate.submissions.clone(),
                reads: gate.reads.clone(), deliveries: gate.deliveries.clone(),
            }
        }
        pub(super) fn registry(&self) -> Arc<crate::capabilities::CapabilityRegistry> {
            self.registry.clone()
        }
        pub(super) fn begin_issue(self: &Arc<Self>, request: MemberEnrollmentIssueRequest,
            credentials: account::RoleAuthorityAuthInput) -> Result<Arc<IssueFrame>, Arc<Rejected>> {
            let input = IssueInput {
                operation: request.request_id, account_name: request.account_name, workspace: request.workspace_id,
                administrator_resource: request.administrator_resource, credentials,
                password: request.current_password.into_zeroizing(),
            };
            let (mut gate, poisoned) = match self.admission.lock() {
                Ok(gate) => (gate, false), Err(poisoned) => (poisoned.into_inner(), true),
            };
            let reason = if poisoned { Some(RejectionReason::Poisoned) }
                else if gate.closing { Some(RejectionReason::Closing) }
                else { None };
            if let Some(reason) = reason {
                gate.closing |= poisoned;
                let rejected = Arc::new(Rejected { _input: RejectedInput::Issue(input), _reason: reason });
                gate.rejected.push(rejected.clone()); return Err(rejected);
            }
            if let Some(frame) = gate.issues.get(&input.operation) {
                if frame.matches(&self.storage, &input) { return Ok(frame.clone()); }
                let rejected = Arc::new(Rejected {
                    _input: RejectedInput::Issue(input), _reason: RejectionReason::DifferentOriginal,
                });
                gate.rejected.push(rejected.clone()); return Err(rejected);
            }
            if let Some(frame) = gate.submissions.iter().find(|frame| frame.matches(&self.storage, &input)) {
                return Ok(frame.clone());
            }
            let seed = match account::MemberInvitationIssueSeed::new(
                &self.storage, input.selectors(), input.credentials.clone(),
            ) {
                Ok(seed) => seed,
                Err(error) => {
                    let rejected = Arc::new(Rejected {
                        _input: RejectedInput::Issue(input), _reason: RejectionReason::Seed(error),
                    });
                    gate.rejected.push(rejected.clone()); return Err(rejected);
                }
            };
            let frame = Arc::new(IssueFrame {
                input, seed, worker: Worker::unstarted(), submission: OnceLock::new(), delivery: OnceLock::new(),
            });
            gate.submissions.push(frame.clone());
            // The strong original frame is registered before the first worker poll.
            frame.worker.start(frame.clone().issue(self.clone()));
            Ok(frame)
        }

        fn note_issued(&self, frame: &Arc<IssueFrame>, ticket: &invitation::Ticket)
            -> Result<(), IssueFailure> {
            let mut gate = match self.admission.lock() {
                Ok(gate) => gate,
                Err(poisoned) => {
                    poisoned.into_inner().closing = true;
                    return Err(IssueFailure::RegistryPoisoned { retained: ticket.clone() });
                }
            };
            if !gate.submissions.iter().any(|old| Arc::ptr_eq(old, frame)) {
                return Err(IssueFailure::DifferentOriginal { retained: ticket.clone() });
            }
            if let Some(original) = gate.issues.get(&frame.operation_id()) {
                if !Arc::ptr_eq(original, frame) {
                    return Err(IssueFailure::DifferentOriginal { retained: ticket.clone() });
                }
            } else {
                // Only genuine Account admission can promote a canonical issue.
                // Unauthenticated/failed submissions never claim this UUID.
                gate.issues.insert(frame.operation_id(), frame.clone());
            }
            Ok(())
        }

        fn begin_read(self: &Arc<Self>, input: ReadInput) -> Result<Arc<ReadFrame>, Arc<Rejected>> {
            let (mut gate, poisoned) = match self.admission.lock() {
                Ok(gate) => (gate, false), Err(poisoned) => (poisoned.into_inner(), true),
            };
            let budget = self.storage.config().statement_timeout().min(std::time::Duration::from_secs(60));
            let deadline = tokio::time::Instant::now().checked_add(budget);
            let reason = if poisoned { Some(RejectionReason::Poisoned) }
                else if gate.closing { Some(RejectionReason::Closing) }
                else if deadline.is_none() { Some(RejectionReason::Deadline) } else { None };
            if let Some(reason) = reason {
                gate.closing |= poisoned;
                let rejected = Arc::new(Rejected { _input: RejectedInput::Read(input), _reason: reason });
                gate.rejected.push(rejected.clone()); return Err(rejected);
            }
            let frame = Arc::new(ReadFrame {
                input, deadline: deadline.expect("checked HTTP read deadline"), worker: Worker::unstarted(),
            });
            gate.reads.push(frame.clone());
            frame.worker.start(frame.clone().read(self.clone()));
            Ok(frame)
        }
        pub(super) fn issuer_metadata(self: &Arc<Self>, operation: uuid::Uuid,
            credentials: account::RoleAuthorityAuthInput) -> Result<Arc<ReadFrame>, Arc<Rejected>> {
            self.begin_read(ReadInput::Issuer { operation, credentials })
        }
        pub(super) fn revoke_metadata(self: &Arc<Self>, operation: uuid::Uuid, workspace: String,
            administrator_resource: String, credentials: account::RoleAuthorityAuthInput)
            -> Result<Arc<ReadFrame>, Arc<Rejected>> {
            self.begin_read(ReadInput::RevokeMetadata { operation, workspace, administrator_resource, credentials })
        }
        pub(super) fn revoke_receipt(self: &Arc<Self>, operation: uuid::Uuid, request: uuid::Uuid,
            workspace: String, administrator_resource: String, credentials: account::RoleAuthorityAuthInput)
            -> Result<Arc<ReadFrame>, Arc<Rejected>> {
            self.begin_read(ReadInput::RevokeReceipt { operation, request, workspace, administrator_resource, credentials })
        }
        pub(super) fn delivery(&self, frame: &Arc<IssueFrame>)
            -> Option<Result<Arc<account::MemberInvitationDeliveryFrame>, Arc<invitation::AccountError>>> {
            let mut gate = self.admission.lock().ok()?;
            if gate.closing || !gate.issues.get(&frame.operation_id()).is_some_and(|old| Arc::ptr_eq(old, frame)) {
                return None;
            }
            let original = frame.submission.get()?;
            let delivery = frame.delivery.get_or_init(|| account::MemberInvitationDeliveryFrame::start(
                &self.storage, self.registry.clone(), frame.input.credentials.clone(),
                frame.operation_id(), original.original_deadline,
            ).map_err(Arc::new));
            match delivery {
                Ok(delivery) => {
                    if !gate.deliveries.iter().any(|old| Arc::ptr_eq(old, delivery)) {
                        gate.deliveries.push(delivery.clone());
                    }
                    Some(Ok(delivery.clone()))
                }
                Err(error) => Some(Err(error.clone())),
            }
        }
    }
    pub(crate) struct MemberEnrollmentHttpShutdownReport {
        pub(crate) complete: bool,
        _issues: Vec<Arc<Result<invitation::Ticket, WorkerFailure<IssueFailure>>>>,
        _reads: Vec<Arc<Result<ReadOutput, WorkerFailure<ReadFailure>>>>,
        _deliveries: Vec<Result<(), Arc<tokio::task::JoinError>>>,
    }
    impl MemberEnrollmentHttpShutdownInventory {
        pub(crate) async fn drain(&self) -> Arc<MemberEnrollmentHttpShutdownReport> {
            let mut complete = self.healthy;
            let mut issues = Vec::with_capacity(self.issues.len());
            for issue in &self.issues {
                let outcome = issue.worker.wait().await;
                complete &= outcome.as_ref().is_ok();
                issues.push(outcome);
            }
            let mut reads = Vec::with_capacity(self.reads.len());
            for read in &self.reads {
                let outcome = read.worker.wait().await;
                complete &= outcome.as_ref().is_ok();
                reads.push(outcome);
            }
            let mut deliveries = Vec::with_capacity(self.deliveries.len());
            for delivery in &self.deliveries {
                // This joins the retained recovery worker; never publish a
                // secret or run fresh delivery authorization during shutdown.
                let outcome = delivery.drain_worker().await.clone();
                complete &= outcome.is_ok();
                deliveries.push(outcome);
            }
            Arc::new(MemberEnrollmentHttpShutdownReport {
                complete, _issues: issues, _reads: reads, _deliveries: deliveries,
            })
        }
    }

}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
pub(crate) use member_enrollment_http_owner::{
    MemberEnrollmentHttpRuntime, MemberEnrollmentHttpShutdownInventory, MemberEnrollmentHttpShutdownReport,
};


#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
fn member_enrollment_credentials(headers: &axum::http::HeaderMap)
    -> Result<crate::storage::surreal::resource_authority::RoleAuthorityAuthInput,
        (axum::http::StatusCode, axum::Json<serde_json::Value>)> {
    let channel = match crate::api::stage::capture_channel_binding(headers) {
        Ok(channel) => channel, Err(_) => return Err(constant_denial()),
    };
    let token = headers.get(HSK_HEADER_SESSION_TOKEN).and_then(|value| value.to_str().ok())
        .filter(|value| value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()));
    let Some(token) = token else { return Err(constant_denial()); };
    Ok(crate::storage::surreal::resource_authority::RoleAuthorityAuthInput::new(
        token.to_owned(), channel.binding_hash,
    ))
}

#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
#[derive(serde::Serialize)]
struct MemberEnrollmentMetadataResponse {
    operation_id: uuid::Uuid,
    account_id: String,
    principal_id: String,
    access_space_id: String,
    account_name: String,
    status: String,
    vault_state: String,
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl From<&crate::storage::surreal::resource_authority::MemberInvitationMetadata>
    for MemberEnrollmentMetadataResponse {
    fn from(value: &crate::storage::surreal::resource_authority::MemberInvitationMetadata) -> Self {
        Self { operation_id: value.operation_id, account_id: value.account_id.clone(),
            principal_id: value.principal_id.clone(), access_space_id: value.access_space_id.clone(),
            account_name: value.account_name.clone(), status: value.status.clone(), vault_state: value.vault_state.clone() }
    }
}

#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
async fn observe_member_enrollment(
    owner: Option<axum::Extension<std::sync::Arc<MemberEnrollmentHttpRuntime>>>,
    path: Result<axum::extract::Path<uuid::Uuid>, axum::extract::rejection::PathRejection>,
    headers: axum::http::HeaderMap,
) -> Result<axum::Json<MemberEnrollmentMetadataResponse>,
    (axum::http::StatusCode, axum::Json<serde_json::Value>)> {
    let Some(axum::Extension(owner)) = owner else { return Err(constant_denial()); };
    let axum::extract::Path(operation) = match path {
        Ok(path) => path, Err(_) => return Err(constant_denial()),
    };
    let credentials = match member_enrollment_credentials(&headers) {
        Ok(credentials) => credentials, Err(denial) => return Err(denial),
    };
    let frame = match owner.issuer_metadata(operation, credentials) {
        Ok(frame) => frame, Err(_retained) => return Err(constant_denial()),
    };
    let result = frame.worker.wait().await;
    match result.as_ref() {
        Ok(member_enrollment_http_owner::ReadOutput::Issuer(metadata)) => Ok(axum::Json(metadata.into())),
        _ => Err(constant_denial()),
    }
}

#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
struct MemberEnrollmentIssuedResponse {
    operation_id: uuid::Uuid,
    delivery: crate::storage::surreal::resource_authority::MemberInvitationDelivery,
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl serde::Serialize for MemberEnrollmentIssuedResponse {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer {
        use serde::ser::SerializeStruct;
        let expiry = self.delivery.descriptor().expires_at();
        let mut value = serializer.serialize_struct("MemberEnrollmentIssuedResponse", 4)?;
        value.serialize_field("operation_id", &self.operation_id)?;
        value.serialize_field("invitation_secret", self.delivery.recipient_token())?;
        value.serialize_field("expires_at_unix_seconds", &expiry.unix_seconds())?;
        value.serialize_field("expires_at_nanoseconds", &expiry.nanoseconds())?;
        value.end()
    }
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
#[derive(serde::Serialize)]
#[serde(untagged)]
enum MemberEnrollmentIssueResponse {
    Metadata(MemberEnrollmentMetadataResponse),
    Issued(MemberEnrollmentIssuedResponse),
}

#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
async fn issue_member_enrollment(
    owner: Option<axum::Extension<std::sync::Arc<MemberEnrollmentHttpRuntime>>>,
    headers: axum::http::HeaderMap,
    request: Result<axum::Json<MemberEnrollmentIssueRequest>, axum::extract::rejection::JsonRejection>,
) -> Result<(axum::http::StatusCode, axum::Json<MemberEnrollmentIssueResponse>),
    (axum::http::StatusCode, axum::Json<serde_json::Value>)> {
    use crate::model_runtime::cloud::member_enrollment::CustodyObservation;
    let Some(axum::Extension(owner)) = owner else { return Err(constant_denial()); };
    let request = match member_enrollment_request(request) {
        Ok(request) => request, Err(denial) => return Err(denial),
    };
    let credentials = match member_enrollment_credentials(&headers) {
        Ok(credentials) => credentials, Err(denial) => return Err(denial),
    };
    let frame = match owner.begin_issue(request, credentials.clone()) {
        Ok(frame) => frame, Err(_retained) => return Err(constant_denial()),
    };
    let admitted = frame.worker.wait().await;
    let ticket = match admitted.as_ref() {
        Ok(ticket) => ticket, Err(_retained) => return Err(constant_denial()),
    };
    let custody = ticket.drain().await;
    let metadata_frame = match owner.issuer_metadata(frame.operation_id(), credentials) {
        Ok(frame) => frame, Err(_retained) => return Err(constant_denial()),
    };
    let metadata_result = metadata_frame.worker.wait().await;
    let metadata = match metadata_result.as_ref() {
        Ok(member_enrollment_http_owner::ReadOutput::Issuer(metadata)) => metadata,
        _ => return Err(constant_denial()),
    };
    match custody.observation {
        CustodyObservation::Unknown { .. } => Ok((axum::http::StatusCode::ACCEPTED,
            axum::Json(MemberEnrollmentIssueResponse::Metadata(metadata.into())))),
        CustodyObservation::Completed(_) => {
            let delivery_frame = match owner.delivery(&frame) {
                Some(Ok(frame)) => frame, _ => return Err(constant_denial()),
            };
            let delivery = match delivery_frame.deliver().await {
                Ok(delivery) => delivery, Err(_retained) => return Err(constant_denial()),
            };
            Ok((axum::http::StatusCode::OK, axum::Json(MemberEnrollmentIssueResponse::Issued(
                MemberEnrollmentIssuedResponse { operation_id: frame.operation_id(), delivery },
            ))))
        }
    }
}


#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
#[derive(serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum MemberEnrollmentRedemptionObservation {
    ConsumedNow,
    SameAttemptAlreadyConsumed,
}

#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
#[derive(serde::Serialize)]
struct MemberEnrollmentRedemptionResponse {
    operation_id: uuid::Uuid,
    outcome: MemberEnrollmentRedemptionObservation,
}

// Handler candidate only. Coherent four-route registration is separate.
// Actual runtime Extension is mandatory; never create/reload a per-request owner.
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
async fn redeem_member_enrollment(
    runtime: Option<axum::Extension<std::sync::Arc<
        crate::storage::surreal::resource_authority::MemberRecipientRedemptionRuntime,
    >>>,
    path: Result<axum::extract::Path<uuid::Uuid>, axum::extract::rejection::PathRejection>,
    request: Result<axum::Json<MemberEnrollmentRedeemRequest>, axum::extract::rejection::JsonRejection>,
) -> Result<
    (axum::http::StatusCode, axum::Json<MemberEnrollmentRedemptionResponse>),
    (axum::http::StatusCode, axum::Json<serde_json::Value>),
> {
    use crate::storage::surreal::resource_authority::MemberRecipientOutcome;
    let Some(axum::Extension(runtime)) = runtime else {
        return Err(constant_denial());
    };
    let axum::extract::Path(operation_id) = match path {
        Ok(path) => path,
        Err(_) => return Err(constant_denial()),
    };
    let request = match member_enrollment_request(request) {
        Ok(request) => request,
        Err(denial) => return Err(denial),
    };
    let retained = match runtime.begin_operation(
        operation_id, request.invitation_secret.into_zeroizing(), request.password.into_owned(),
    ) {
        Ok(retained) => retained,
        // This is a borrowed projection of an Arc also retained by the runtime.
        // The complete rejected input/error remains in the same runtime owner.
        Err(_retained_rejection) => return Err(constant_denial()),
    };
    let admitted = retained.wait().await;
    let controller = match admitted.as_ref() {
        Ok(controller) => controller,
        // The original request still owns the actual preparation/admission/join
        // outcome. No error payload is removed, formatted, logged or reissued.
        Err(_retained_failure) => return Err(constant_denial()),
    };
    let completed = controller.wait().await;
    let outcome = match completed.as_ref() {
        Ok(MemberRecipientOutcome::ConsumedNow) =>
            MemberEnrollmentRedemptionObservation::ConsumedNow,
        Ok(MemberRecipientOutcome::SameAttemptAlreadyConsumed) =>
            MemberEnrollmentRedemptionObservation::SameAttemptAlreadyConsumed,
        // Includes genuine unknown settlement. Denial asserts neither rollback
        // nor retry permission; the same controller retains the whole outcome.
        Err(_retained_failure) => return Err(constant_denial()),
    };
    Ok((axum::http::StatusCode::OK, axum::Json(MemberEnrollmentRedemptionResponse {
        operation_id, outcome,
    })))
}


#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
#[derive(serde::Serialize)]
struct MemberEnrollmentRevocationResponse {
    operation_id: uuid::Uuid,
    request_id: uuid::Uuid,
    event_id: String,
    event_sequence: i64,
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
async fn revoke_member_enrollment(
    owner: Option<axum::Extension<std::sync::Arc<MemberEnrollmentHttpRuntime>>>,
    revocations: Option<axum::Extension<std::sync::Arc<
        crate::storage::surreal::resource_authority::MemberRevocationRuntime,
    >>>,
    headers: axum::http::HeaderMap,
    path: Result<axum::extract::Path<uuid::Uuid>, axum::extract::rejection::PathRejection>,
    request: Result<axum::Json<MemberEnrollmentRevokeRequest>, axum::extract::rejection::JsonRejection>,
) -> Result<axum::Json<MemberEnrollmentRevocationResponse>,
    (axum::http::StatusCode, axum::Json<serde_json::Value>)> {
    use crate::storage::surreal::resource_authority::MemberRevocationServerInput;
    let Some(axum::Extension(owner)) = owner else { return Err(constant_denial()); };
    let Some(axum::Extension(revocations)) = revocations else { return Err(constant_denial()); };
    let axum::extract::Path(operation) = match path {
        Ok(path) => path, Err(_) => return Err(constant_denial()),
    };
    let request = match member_enrollment_request(request) {
        Ok(request) => request, Err(denial) => return Err(denial),
    };
    let credentials = match member_enrollment_credentials(&headers) {
        Ok(credentials) => credentials, Err(denial) => return Err(denial),
    };
    let metadata = match owner.revoke_metadata(operation, request.workspace_id.clone(),
        request.administrator_resource.clone(), credentials.clone()) {
        Ok(frame) => frame, Err(_retained) => return Err(constant_denial()),
    };
    let current = metadata.worker.wait().await;
    let status = match current.as_ref() {
        Ok(member_enrollment_http_owner::ReadOutput::RevokeMetadata(value)) => value.status(),
        _ => return Err(constant_denial()),
    };
    if status == "active" {
        let seed = match revocations.begin(owner.registry(), MemberRevocationServerInput {
            request_selector: request.request_id, enrollment_operation_id: operation,
            workspace: request.workspace_id.clone(), administrator_resource: request.administrator_resource.clone(),
            credentials: credentials.clone(), password: request.current_password.into_zeroizing(),
        }) {
            Ok(seed) => seed, Err(_retained) => return Err(constant_denial()),
        };
        let prepared = seed.wait().await;
        let controller = match prepared.as_ref() {
            Ok(controller) => controller, Err(_retained) => return Err(constant_denial()),
        };
        let result = controller.wait().await;
        if result.as_ref().is_err() { return Err(constant_denial()); }
    } else if status != "revoked" {
        return Err(constant_denial());
    }
    // Every outward observation uses current Update authorization. The
    // canonical receipt read also handles an already-revoked original on restart.
    let frame = match owner.revoke_receipt(operation, request.request_id, request.workspace_id,
        request.administrator_resource, credentials) {
        Ok(frame) => frame, Err(_retained) => return Err(constant_denial()),
    };
    let result = frame.worker.wait().await;
    match result.as_ref() {
        Ok(member_enrollment_http_owner::ReadOutput::RevokeReceipt(value)) => Ok(axum::Json(
            MemberEnrollmentRevocationResponse {
                operation_id: value.enrollment_operation_id(), request_id: value.request_id(),
                event_id: value.event_id().to_owned(), event_sequence: value.event_sequence(),
            },
        )),
        _ => Err(constant_denial()),
    }
}

#[cfg(all(any(test, feature = "test-utils"), feature = "duckdb-flight-recorder", feature = "os-keychain"))]
pub mod local_account_http_test_support {
    use super::*;
    use crate::api::MountedRequestExt;
    use axum::{body::{to_bytes, Body}, http::Request};
    use serde_json::{json, Value};
    use sha2::{Digest, Sha256};
    use std::sync::Arc;

// Move the existing BindingFile and request helper declarations here exactly once; visibility
// is narrowed to authority descendants so the unchanged sibling tests can continue using them.
    pub(in crate::api::authority) struct BindingFile {
        _lock: std::sync::MutexGuard<'static, ()>,
        _directory: tempfile::TempDir,
        previous: Option<std::ffi::OsString>,
        pub(in crate::api::authority) channel: String,
    }
    impl BindingFile {
        pub(in crate::api::authority) fn new() -> Self {
            let lock = crate::api::stage::NATIVE_BINDING_ENV_LOCK
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            let directory = tempfile::tempdir().unwrap();
            let channel = "e3".repeat(32);
            let path = directory.path().join("account-binding.json");
            std::fs::write(
                &path,
                serde_json::to_vec(&crate::api::stage::current_process_native_binding(&channel))
                    .unwrap(),
            )
            .unwrap();
            let previous = std::env::var_os("HANDSHAKE_STAGE_BINDING_FILE");
            std::env::set_var("HANDSHAKE_STAGE_BINDING_FILE", path);
            Self {
                _lock: lock,
                _directory: directory,
                previous,
                channel,
            }
        }
    }
    impl Drop for BindingFile {
        fn drop(&mut self) {
            if let Some(value) = &self.previous {
                std::env::set_var("HANDSHAKE_STAGE_BINDING_FILE", value);
            } else {
                std::env::remove_var("HANDSHAKE_STAGE_BINDING_FILE");
            }
        }
    }
    pub(in crate::api::authority) async fn request(
        router: &Router,
        method: &str,
        path: &str,
        channel: Option<&str>,
        session: Option<&str>,
        body: Value,
    ) -> (StatusCode, Value) {
        let mut request = Request::builder()
            .method(method)
            .uri(path)
            .header("content-type", "application/json");
        if let Some(channel) = channel {
            request = request.header("x-hsk-channel-binding-token", channel);
        }
        if let Some(session) = session {
            request = request.header(HSK_HEADER_SESSION_TOKEN, session);
        }
        let response = router
            .clone()
            .oneshot(
                request
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), 128 * 1024).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    // Shared runner copied from the original test; body unchanged, only test attr/name removed.
    pub async fn run_local_account_http_setup_login_vault_logout_and_constant_denials() {
        let binding = BindingFile::new();
        let backend = crate::storage::tests::embedded_test_backend()
            .await
            .unwrap();
        let recorder = Arc::new(
            crate::flight_recorder::duckdb::DuckDbFlightRecorder::new_in_memory(7).unwrap(),
        );
        let state = AppState {
            storage: backend.database.clone(),
            surreal: backend.storage.clone(),
            flight_recorder: recorder.clone(),
            diagnostics: recorder,
            llm_client: Arc::new(crate::llm::ollama::InMemoryLlmClient::new("ok".into())),
            capability_registry: Arc::new(crate::capabilities::CapabilityRegistry::new()),
            session_registry: Arc::new(crate::workflows::SessionRegistry::new(
                crate::workflows::SessionSchedulerConfig::default(),
            )),
        };
        let router = routes(state.clone());
        let denial = json!({"error":"HSK-403-PROTECTED-RESOURCE"});
        assert_eq!(
            request(
                &router,
                "GET",
                "/authority/session",
                None,
                None,
                Value::Null
            )
            .await,
            (StatusCode::FORBIDDEN, denial.clone())
        );
        let mut counts = state.surreal.test_admin_query("RETURN array::len(SELECT VALUE id FROM kernel_event_ledger WHERE source_component = 'local_account_authority');".into()).await.unwrap();
        assert_eq!(counts.take::<Option<i64>>(0).unwrap(), Some(0));
        let password = json!({"account_name":"Owner","password":"correct horse battery staple"});
        assert_eq!(
            request(
                &router,
                "POST",
                "/authority/setup",
                Some(&binding.channel),
                None,
                password.clone()
            )
            .await
            .0,
            StatusCode::OK
        );
        assert_eq!(
            request(
                &router,
                "POST",
                "/authority/setup",
                Some(&binding.channel),
                None,
                password.clone()
            )
            .await,
            (StatusCode::FORBIDDEN, denial.clone())
        );
        assert_eq!(
            request(
                &router,
                "POST",
                "/authority/login",
                Some(&binding.channel),
                None,
                json!({"account_name":"Owner","password":"wrong password value"})
            )
            .await,
            (StatusCode::FORBIDDEN, denial.clone())
        );
        let (status, credential) = request(
            &router,
            "POST",
            "/authority/login",
            Some(&binding.channel),
            None,
            password,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let exchange = json!({"account_id":credential["account_id"],"principal_id":credential["principal_id"],
            "access_space_id":credential["access_space_id"],"authentication_token":credential["token"]});
        let (status, session) = request(
            &router,
            "POST",
            "/authority/session",
            Some(&binding.channel),
            None,
            exchange.clone(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let token = session["session_token"].as_str().unwrap();
        assert_eq!(
            request(
                &router,
                "POST",
                "/authority/session",
                Some(&binding.channel),
                None,
                exchange
            )
            .await,
            (StatusCode::FORBIDDEN, denial.clone())
        );
        let (status, current) = request(
            &router,
            "GET",
            "/authority/session",
            Some(&binding.channel),
            Some(token),
            Value::Null,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(current["account_id"], credential["account_id"]);
        assert_eq!(
            request(
                &router,
                "GET",
                "/authority/session",
                Some(&"f4".repeat(32)),
                Some(token),
                Value::Null
            )
            .await,
            (StatusCode::FORBIDDEN, denial.clone())
        );
        use crate::model_runtime::cloud::secrets_vault::{OsKeychainSecretsVault, SecretsVault};
        let config = state.surreal.config();
        let installation = hex::encode(Sha256::digest(
            serde_json::to_vec(&(
                config.path().to_string_lossy(),
                config.namespace(),
                config.database(),
            ))
            .unwrap(),
        ));
        let vault = OsKeychainSecretsVault::new(format!("handshake-local-accounts-{installation}"));
        let session_id = session["session_id"].as_str().unwrap();
        assert!(vault.get(session_id).is_ok_and(|stored| stored.as_str() == token));
        let other_vault =
            OsKeychainSecretsVault::new(format!("handshake-local-accounts-{installation}-other"));
        assert!(
            matches!(other_vault.get(session_id), Err(crate::model_runtime::cloud::secrets_vault::SecretsVaultError::NoSecretForLane(missing)) if missing == session_id)
        );
        assert_eq!(
            request(
                &router,
                "POST",
                "/authority/logout",
                Some(&binding.channel),
                Some(token),
                Value::Null
            )
            .await
            .0,
            StatusCode::OK
        );
        assert!(
            matches!(vault.get(session_id), Err(crate::model_runtime::cloud::secrets_vault::SecretsVaultError::NoSecretForLane(missing)) if missing == session_id)
        );
        for path in ["/authority/session", "/authority/logout"] {
            let method = if path.ends_with("logout") {
                "POST"
            } else {
                "GET"
            };
            assert_eq!(
                request(
                    &router,
                    method,
                    path,
                    Some(&binding.channel),
                    Some(token),
                    Value::Null
                )
                .await,
                (StatusCode::FORBIDDEN, denial.clone())
            );
        }
        let mut counts = state.surreal.test_admin_query("RETURN {setup: array::len(SELECT VALUE id FROM kernel_event_ledger WHERE event_type = 'LOCAL_ACCOUNT_SETUP'), login: array::len(SELECT VALUE id FROM kernel_event_ledger WHERE event_type = 'LOCAL_ACCOUNT_LOGIN'), logout: array::len(SELECT VALUE id FROM kernel_event_ledger WHERE event_type = 'LOCAL_ACCOUNT_LOGOUT')};".into()).await.unwrap();
        assert_eq!(
            counts.take::<Option<Value>>(0).unwrap(),
            Some(json!({"setup":1,"login":1,"logout":1}))
        );
        state.surreal.shutdown().await.unwrap();
    }

    // End support module after the shared runner.
}

#[cfg(all(test, feature = "duckdb-flight-recorder", feature = "os-keychain"))]
mod local_account_http_tests {
    use super::*;
    use std::sync::Arc;
    use super::local_account_http_test_support::{request, BindingFile};

    #[tokio::test]
    async fn cancelled_vault_request_retains_serialization_until_blocking_exit() {
        let (entered, started) = tokio::sync::oneshot::channel();
        let (release, blocked) = std::sync::mpsc::channel();
        let (finished, exited) = tokio::sync::oneshot::channel();
        let request = tokio::spawn(run_session_vault_operation(move || {
            entered.send(()).unwrap();
            blocked.recv().unwrap();
            let _ = finished.send(());
        }));
        started.await.unwrap();
        request.abort();
        assert!(request.await.unwrap_err().is_cancelled());
        assert!(SESSION_VAULT_LOCK.try_lock().is_err());
        release.send(()).unwrap();
        exited.await.unwrap();
        let _guard =
            tokio::time::timeout(std::time::Duration::from_secs(2), SESSION_VAULT_LOCK.lock())
                .await
                .expect("blocking exit releases the vault lock");
    }

    fn lifecycle_pause(
        state: &AppState,
        phase: &'static str,
        fail: bool,
    ) -> (
        tokio::sync::oneshot::Receiver<()>,
        tokio::sync::oneshot::Sender<()>,
    ) {
        let (entered, observed) = tokio::sync::oneshot::channel();
        let (release, wait) = tokio::sync::oneshot::channel();
        let previous = ACCOUNT_LIFECYCLE_PROBE
            .lock()
            .unwrap()
            .replace(AccountLifecycleProbe {
                path: state.surreal.config().path().to_path_buf(),
                phase,
                entered,
                release: wait,
                fail,
            });
        assert!(previous.is_none());
        (observed, release)
    }

    #[tokio::test]
    async fn local_account_handler_cancellation_and_vault_cleanup_retry() {
        use crate::model_runtime::cloud::secrets_vault::{OsKeychainSecretsVault, SecretsVault};
        let binding = BindingFile::new();
        let backend = crate::storage::tests::embedded_test_backend()
            .await
            .unwrap();
        let recorder = Arc::new(
            crate::flight_recorder::duckdb::DuckDbFlightRecorder::new_in_memory(7).unwrap(),
        );
        let state = AppState {
            storage: backend.database.clone(),
            surreal: backend.storage.clone(),
            flight_recorder: recorder.clone(),
            diagnostics: recorder,
            llm_client: Arc::new(crate::llm::ollama::InMemoryLlmClient::new("ok".into())),
            capability_registry: Arc::new(crate::capabilities::CapabilityRegistry::new()),
            session_registry: Arc::new(crate::workflows::SessionRegistry::new(
                crate::workflows::SessionSchedulerConfig::default(),
            )),
        };
        let router = routes(state.clone());
        let config = state.surreal.config();
        let installation = hex::encode(Sha256::digest(
            serde_json::to_vec(&(
                config.path().to_string_lossy(),
                config.namespace(),
                config.database(),
            ))
            .unwrap(),
        ));
        let vault = OsKeychainSecretsVault::new(format!("handshake-local-accounts-{installation}"));
        let password = json!({"account_name":"Owner","password":"correct horse battery staple"});
        assert_eq!(
            request(
                &router,
                "POST",
                "/authority/setup",
                Some(&binding.channel),
                None,
                password.clone()
            )
            .await
            .0,
            StatusCode::OK
        );
        for (index, phase) in ["post_session", "post_vault"].into_iter().enumerate() {
            let (_, credential) = request(
                &router,
                "POST",
                "/authority/login",
                Some(&binding.channel),
                None,
                password.clone(),
            )
            .await;
            let principal_id = credential["principal_id"].as_str().unwrap().to_owned();
            let exchange = json!({"account_id":credential["account_id"],"principal_id":credential["principal_id"],
                "access_space_id":credential["access_space_id"],"authentication_token":credential["token"]});
            let (observed, release) = lifecycle_pause(&state, phase, false);
            let app = router.clone();
            let channel = binding.channel.clone();
            let handler = tokio::spawn(async move {
                request(
                    &app,
                    "POST",
                    "/authority/session",
                    Some(&channel),
                    None,
                    exchange,
                )
                .await
            });
            tokio::time::timeout(std::time::Duration::from_secs(10), observed)
                .await
                .unwrap()
                .unwrap();
            handler.abort();
            assert!(handler.await.unwrap_err().is_cancelled());
            release.send(()).unwrap();
            let id = tokio::time::timeout(std::time::Duration::from_secs(10), async {
                loop {
                    let mut response = state.surreal.test_admin_query(format!(
                        "SELECT VALUE record::id(id) FROM authenticated_sessions WHERE revoked_at = NONE AND principal_id = type::record('principals', '{principal_id}');"
                    )).await.unwrap();
                    let ids: Vec<String> = response.take(0).unwrap();
                    let mut audit = state.surreal.test_admin_query("RETURN array::len(SELECT VALUE id FROM kernel_event_ledger WHERE event_type = 'LOCAL_ACCOUNT_LOGIN');".into()).await.unwrap();
                    if audit.take::<Option<i64>>(0).unwrap() == Some((index + 1) as i64) && ids.len() == 1 {
                        if vault.get(&ids[0]).is_ok() { break ids[0].clone(); }
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                }
            }).await.unwrap();
            let token = vault.get(&id).unwrap();
            assert_eq!(
                request(
                    &router,
                    "GET",
                    "/authority/session",
                    Some(&binding.channel),
                    Some(&token),
                    Value::Null
                )
                .await
                .0,
                StatusCode::OK
            );
            let logout_phase = if index == 0 {
                "post_revoke"
            } else {
                "vault_delete"
            };
            let (observed, release) = lifecycle_pause(&state, logout_phase, index == 1);
            let app = router.clone();
            let channel = binding.channel.clone();
            let bearer = token.clone();
            let handler = tokio::spawn(async move {
                request(
                    &app,
                    "POST",
                    "/authority/logout",
                    Some(&channel),
                    Some(&bearer),
                    Value::Null,
                )
                .await
            });
            tokio::time::timeout(std::time::Duration::from_secs(10), observed)
                .await
                .unwrap()
                .unwrap();
            if index == 0 {
                handler.abort();
            }
            release.send(()).unwrap();
            if index == 0 {
                assert!(handler.await.unwrap_err().is_cancelled());
            } else {
                assert_eq!(
                    handler.await.unwrap().0,
                    StatusCode::OK,
                    "owned retry repairs injected transient vault deletion failure"
                );
            }
            tokio::time::timeout(std::time::Duration::from_secs(10), async {
                loop {
                    match vault.get(&id) {
                        Ok(_) => tokio::time::sleep(std::time::Duration::from_millis(20)).await,
                        Err(crate::model_runtime::cloud::secrets_vault::SecretsVaultError::NoSecretForLane(missing)) if missing == id => break,
                        Err(error) => panic!("vault cleanup lookup failed: {error}"),
                    }
                }
            }).await.unwrap();
            assert_eq!(
                request(
                    &router,
                    "GET",
                    "/authority/session",
                    Some(&binding.channel),
                    Some(&token),
                    Value::Null
                )
                .await
                .0,
                StatusCode::FORBIDDEN
            );
            // Recreate only the retired vault residue to exercise the recovery boundary itself.
            vault.put(&id, token.as_str()).unwrap();
            assert_eq!(
                request(
                    &router,
                    "POST",
                    "/authority/logout",
                    Some(&"f4".repeat(32)),
                    Some(&token),
                    Value::Null
                )
                .await
                .0,
                StatusCode::FORBIDDEN
            );
            assert!(vault.get(&id).is_ok());
            assert_eq!(
                request(
                    &router,
                    "POST",
                    "/authority/logout",
                    Some(&binding.channel),
                    Some(&"ab".repeat(32)),
                    Value::Null
                )
                .await
                .0,
                StatusCode::FORBIDDEN
            );
            assert!(vault.get(&id).is_ok());
            assert_eq!(
                request(
                    &router,
                    "POST",
                    "/authority/logout",
                    Some(&binding.channel),
                    Some(&token),
                    Value::Null
                )
                .await
                .0,
                StatusCode::FORBIDDEN
            );
            assert!(
                matches!(vault.get(&id), Err(crate::model_runtime::cloud::secrets_vault::SecretsVaultError::NoSecretForLane(missing)) if missing == id)
            );
        }
        let mut audit = state.surreal.test_admin_query("RETURN array::len(SELECT VALUE id FROM kernel_event_ledger WHERE event_type = 'LOCAL_ACCOUNT_LOGOUT');".into()).await.unwrap();
        assert_eq!(
            audit.take::<Option<i64>>(0).unwrap(),
            Some(2),
            "recovery never creates a second logout event"
        );
        state.surreal.shutdown().await.unwrap();
    }
    #[tokio::test]
    async fn incomplete_owner_setup_recovery_requires_password_and_preserves_one_time_gate() {
        let binding = BindingFile::new();
        let backend = crate::storage::tests::embedded_test_backend()
            .await
            .unwrap();
        let recorder = Arc::new(
            crate::flight_recorder::duckdb::DuckDbFlightRecorder::new_in_memory(7).unwrap(),
        );
        let state = AppState {
            storage: backend.database.clone(),
            surreal: backend.storage.clone(),
            flight_recorder: recorder.clone(),
            diagnostics: recorder,
            llm_client: Arc::new(crate::llm::ollama::InMemoryLlmClient::new("ok".into())),
            capability_registry: Arc::new(crate::capabilities::CapabilityRegistry::new()),
            session_registry: Arc::new(crate::workflows::SessionRegistry::new(
                crate::workflows::SessionSchedulerConfig::default(),
            )),
        };
        let router = routes(state.clone());
        let password =
            json!({"account_name":"Recovery Owner","password":"correct horse battery staple"});
        let capabilities = state
            .capability_registry
            .profile_by_id("Operator")
            .expect("Operator profile")
            .allowed
            .clone();
        let verifier = crate::storage::surreal::local_accounts::new_password_verifier(
            "correct horse battery staple".to_owned(),
        )
        .await
        .expect("test password verifier");
        state
            .surreal
            .setup_local_owner("Recovery Owner", verifier, capabilities)
            .await
            .expect("seed owner marker without reconciliation service");
        assert!(
            !state
                .surreal
                .reconciliation_principal_is_provisioned()
                .await
                .unwrap(),
            "seeded owner setup must be incomplete before recovery"
        );

        let denial = json!({"error":"HSK-403-PROTECTED-RESOURCE"});
        assert_eq!(
            request(
                &router,
                "POST",
                "/authority/setup",
                Some(&binding.channel),
                None,
                json!({"account_name":"Recovery Owner","password":"wrong password value"}),
            )
            .await,
            (StatusCode::FORBIDDEN, denial.clone()),
            "wrong password must not repair a partial setup"
        );
        assert!(
            !state
                .surreal
                .reconciliation_principal_is_provisioned()
                .await
                .unwrap(),
            "wrong recovery password must leave the service identity absent"
        );

        assert_eq!(
            request(
                &router,
                "POST",
                "/authority/setup",
                Some(&binding.channel),
                None,
                password.clone(),
            )
            .await
            .0,
            StatusCode::OK,
            "the matching owner password repairs only the missing service setup"
        );
        assert!(
            state
                .surreal
                .reconciliation_principal_is_provisioned()
                .await
                .unwrap(),
            "recovery must create the canonical reconciliation authority"
        );
        let mut rows = state
            .surreal
            .test_admin_query(
                "RETURN { principal: array::len(SELECT VALUE id FROM principals WHERE principal_key = 'mt109-reconciliation-service-principal' AND principal_kind = 'service_identity' AND status = 'enabled'), space: array::len(SELECT VALUE id FROM access_spaces WHERE space_key = 'mt109-reconciliation-space' AND status = 'active'), root: array::len(SELECT VALUE id FROM protected_resources WHERE resource_kind = 'reconciliation_queue' AND external_resource_id = 'mt109-protected-reconciliation' AND lifecycle_state = 'active'), root_grant: array::len(SELECT VALUE id FROM resource_grants WHERE resource_id.resource_kind = 'reconciliation_queue' AND resource_id.external_resource_id = 'mt109-protected-reconciliation' AND actions = ['reconcile'] AND capability_ids = ['fr.ingest.native_editor','memory.commit'] AND status = 'active' AND revoked_at = NONE AND expires_at = NONE) };".to_owned(),
            )
            .await
            .unwrap();
        assert_eq!(
            rows.take::<Option<Value>>(0).unwrap(),
            Some(json!({"principal": 1, "space": 1, "root": 1, "root_grant": 1})),
            "recovery must establish the exact service principal, root queue, and root grant"
        );
        assert_eq!(
            request(
                &router,
                "POST",
                "/authority/setup",
                Some(&binding.channel),
                None,
                password,
            )
            .await,
            (StatusCode::FORBIDDEN, denial),
            "completed setup remains one-time after recovery"
        );
        state.surreal.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn local_account_http_setup_login_vault_logout_and_constant_denials() {
        super::local_account_http_test_support::run_local_account_http_setup_login_vault_logout_and_constant_denials().await;
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PasswordLogin {
    account_name: String,
    password: String,
}

// Account-selected fixed confirmation caller; real controller implementation
// and atomic bootstrap/registration proof remain required before adoption.
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
#[derive(serde::Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
enum FirstOwnerAdministrationLifetimeSelection {
    // An empty struct variant enforces unknown-field rejection in Serde 1.0.228.
    NoExpiry {},
    ExpiresAt { expires_at: chrono::DateTime<chrono::Utc> },
}

#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct FirstOwnerAdministrationConfirmationBody {
    operation_id: uuid::Uuid,
    first_workspace_operation_id: uuid::Uuid,
    parent_workspace_external_id: String,
    lifetime: FirstOwnerAdministrationLifetimeSelection,
    password: MemberEnrollmentSecret,
}

#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
struct FirstOwnerAdministrationConfirmationResponse {
    outcome: std::sync::Arc<Result<
        crate::storage::surreal::resource_authority::IssuedSessionCredential,
        crate::storage::surreal::local_accounts::FirstOwnerAdministrationConfirmationFailure,
    >>,
}

#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl serde::Serialize for FirstOwnerAdministrationConfirmationResponse {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer {
        match self.outcome.as_ref() {
            // Serialize the actual immutable canonical credential by borrow.
            // No caller-provided IDs, profile, grants or nonce are constructed.
            Ok(credential) => serde::Serialize::serialize(credential, serializer),
            Err(_) => Err(<S::Error as serde::ser::Error>::custom("confirmation denied")),
        }
    }
}

#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
async fn confirm_first_owner_administration(
    State(state): State<AccountBootstrapState>,
    headers: HeaderMap,
    request: Result<Json<FirstOwnerAdministrationConfirmationBody>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<FirstOwnerAdministrationConfirmationResponse>, (StatusCode, Json<Value>)> {
    use crate::storage::surreal::local_accounts::{
        FirstOwnerAdministrationConfirmationLifetime as Lifetime,
        FirstOwnerAdministrationConfirmationObservation as Observation,
        FirstOwnerAdministrationConfirmationRequest,
    };
    let request = match member_enrollment_request(request) {
        Ok(request) => request,
        Err(denial) => return Err(denial),
    };
    let authenticated = match authenticated_session_credentials_bootstrap(&state, &headers).await {
        Ok(authenticated) => authenticated,
        Err(denial) => return Err(denial),
    };
    let lifetime = match request.lifetime {
        FirstOwnerAdministrationLifetimeSelection::NoExpiry {} => Lifetime::NoExpiry,
        FirstOwnerAdministrationLifetimeSelection::ExpiresAt { expires_at } =>
            Lifetime::ExpiresAt { expires_at },
    };
    let operation = state.surreal.begin_first_owner_administration_confirmation(
        FirstOwnerAdministrationConfirmationRequest {
            operation_id: request.operation_id,
            first_workspace_operation_id: request.first_workspace_operation_id,
            parent_workspace_external_id: request.parent_workspace_external_id,
            lifetime,
            // This genuine session read is still data. Account freshly
            // reauthenticates and compares the prospective origin in its frame.
            actor: authenticated.context,
            session_token: zeroize::Zeroizing::new(authenticated.session_token),
            channel_hash: authenticated.channel_binding_hash,
            password: request.password.into_zeroizing(),
        },
    );
    match operation.wait().await {
        Observation::Complete(outcome) => {
            if outcome.as_ref().is_err() { return Err(constant_denial()); }
            Ok(Json(FirstOwnerAdministrationConfirmationResponse { outcome }))
        }
        // The original controller retains the whole join failure and every
        // uncertain engine result. This response grants no retry authority.
        Observation::Join(_retained_join) => Err(constant_denial()),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TargetedPasswordLogin {
    account_name: String,
    password: String,
    target: Option<crate::storage::surreal::local_accounts::LocalAccountLoginTarget>,
}

async fn setup_status(
    State(state): State<AccountBootstrapState>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    crate::api::stage::capture_channel_binding(&headers).map_err(|_| constant_denial())?;
    let required = state
        .surreal
        .local_owner_setup_required()
        .await
        .map_err(|_| constant_denial())?;
    Ok(Json(
        json!({"schema_version": "hsk.local_account_setup@1", "setup_required": required}),
    ))
}

async fn setup_owner(
    State(state): State<AccountBootstrapState>,
    headers: HeaderMap,
    Json(input): Json<PasswordLogin>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    crate::api::stage::capture_channel_binding(&headers).map_err(|_| constant_denial())?;
    if state
        .surreal
        .local_owner_setup_required()
        .await
        .map_err(|_| constant_denial())?
    {
        let capabilities = state
            .capability_registry
            .profile_by_id("Operator")
            .map_err(|_| constant_denial())?
            .allowed
            .clone();
        let verifier =
            crate::storage::surreal::local_accounts::new_password_verifier(input.password.clone())
                .await
                .map_err(|_| constant_denial())?;
        state
            .surreal
            .setup_local_owner(&input.account_name, verifier, capabilities)
            .await
            .map_err(|_| constant_denial())?;
        state
            .surreal
            .provision_reconciliation_principal(&[], None)
            .await
            .map_err(|_| constant_denial())?;
    } else if !state
        .surreal
        .reconciliation_principal_is_provisioned()
        .await
        .map_err(|_| constant_denial())?
    {
        // Only an interrupted setup can be recovered. A completed setup stays
        // one-time and never mints a session credential on a repeat request.
        state
            .surreal
            .verify_local_owner_password(&input.account_name, input.password)
            .await
            .map_err(|_| constant_denial())?;
        state
            .surreal
            .provision_reconciliation_principal(&[], None)
            .await
            .map_err(|_| constant_denial())?;
    } else {
        return Err(constant_denial());
    }
    Ok(Json(
        json!({"schema_version": "hsk.local_account_setup@1", "setup_required": false}),
    ))
}

async fn login_owner(
    State(state): State<AccountBootstrapState>,
    headers: HeaderMap,
    request: Result<Json<TargetedPasswordLogin>, axum::extract::rejection::JsonRejection>,
) -> Result<
    Json<crate::storage::surreal::resource_authority::IssuedSessionCredential>,
    (StatusCode, Json<Value>),
> {
    crate::api::stage::capture_channel_binding(&headers).map_err(|_| constant_denial())?;
    let input = match request {
        Ok(Json(input)) => input,
        Err(_) => return Err(constant_denial()),
    };
    let target = input.target.unwrap_or(
        crate::storage::surreal::local_accounts::LocalAccountLoginTarget::Default,
    );
    match state
        .surreal
        .login_local_account_target(&input.account_name, input.password, target)
        .await
    {
        Ok(credential) => Ok(Json(credential)),
        Err(_) => {
            let attempt = uuid::Uuid::now_v7().to_string();
            let event = crate::kernel::NewKernelEvent::builder(format!("local-account:{attempt}"), attempt,
                crate::kernel::KernelEventType::LocalAccountLoginDenied,
                KernelActor::System("local_account_authority".into()))
                .aggregate("local_account", "unresolved").source_component("local_account_authority")
                .payload(json!({"schema_version":"hsk.local_account_event@1", "account_id":null,
                    "principal_id":null,"session_id":null,"access_space_id":null,"delegation_chain":[],
                    "resource_refs":[],"action":"LOCAL_ACCOUNT_LOGIN","result":"deny","policy_version":null}))
                .build().map_err(|_| constant_denial())?;
            state
                .storage
                .append_kernel_event(event)
                .await
                .map_err(|_| constant_denial())?;
            Err(constant_denial())
        }
    }
}

pub(crate) async fn authenticated_session(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<crate::storage::surreal::local_accounts::LocalSessionContext, (StatusCode, Json<Value>)>
{
    authenticated_session_credentials(state, headers)
        .await
        .map(|session| session.context)
}

/// MT-109 C2 (Master Spec 02-system-architecture:2758 deny by default): gate for routes that start
/// processes or touch repository state and have no protected-resource row. Only a persisted account
/// session bound to the live native channel passes; everything else gets the constant denial.
pub(crate) async fn require_authenticated_session(
    State(state): State<AppState>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    match authenticated_session(&state, request.headers()).await {
        Ok(_) => next.run(request).await,
        Err(denial) => denial.into_response(),
    }
}

/// MT-155 (Master Spec 02-system-architecture:2758 deny by default, :2776 capability checks): true
/// only when the authenticated session's delegated capabilities contain exactly `capability_id`.
/// An empty list, an empty id and the `*` wildcard never pass.
pub(crate) fn session_holds_capability(
    context: &crate::storage::surreal::local_accounts::LocalSessionContext,
    capability_id: &str,
) -> bool {
    !capability_id.is_empty()
        && capability_id != "*"
        && context
            .delegated_capabilities
            .iter()
            .any(|held| held == capability_id)
}

pub(crate) struct AuthenticatedLocalSession {
    pub context: crate::storage::surreal::local_accounts::LocalSessionContext,
    pub session_token: String,
    pub channel_binding_hash: String,
}

pub(crate) async fn authenticated_session_credentials(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<AuthenticatedLocalSession, (StatusCode, Json<Value>)> {
    authenticated_session_credentials_bootstrap(&AccountBootstrapState::from(state), headers).await
}

pub(crate) async fn authenticated_session_bootstrap(
    state: &AccountBootstrapState,
    headers: &HeaderMap,
) -> Result<crate::storage::surreal::local_accounts::LocalSessionContext, (StatusCode, Json<Value>)> {
    authenticated_session_credentials_bootstrap(state, headers).await.map(|session| session.context)
}

pub(crate) async fn authenticated_session_credentials_bootstrap(
    state: &AccountBootstrapState,
    headers: &HeaderMap,
) -> Result<AuthenticatedLocalSession, (StatusCode, Json<Value>)> {
    let channel =
        crate::api::stage::capture_channel_binding(headers).map_err(|_| constant_denial())?;
    let token = headers
        .get(HSK_HEADER_SESSION_TOKEN)
        .and_then(|value| value.to_str().ok())
        .filter(|value| value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .ok_or_else(constant_denial)?;
    let context = state
        .surreal
        .authenticate_local_session(token, &channel.binding_hash)
        .await
        .map_err(|_| constant_denial())?;
    Ok(AuthenticatedLocalSession {
        context,
        session_token: token.to_owned(),
        channel_binding_hash: channel.binding_hash,
    })
}

/// MT-154 (D-154-3; Master Spec 02-system-architecture.md:2740/:2751/:2758/:2773/:2776): authority
/// for account-owned surfaces that have no workspace and no protected-resource row (Atelier,
/// account-global/surface preferences). The live account session is authenticated through the
/// record access method, the capability is rechecked against the session's delegated capabilities,
/// and the request then runs as the account's record user, so the per-row
/// `owner_account_id = $auth.account_id` table permissions are the data boundary. The scope has no
/// workspace: receipts written inside it carry `wsids = []` and are accepted only by
/// `fn::mt154_account_receipt`.
#[derive(Clone, Debug)]
pub(crate) struct AccountSessionAuthority {
    pub(crate) actor_id: String,
    pub(crate) record_user_scope: RecordUserScope,
}

impl AccountSessionAuthority {
    /// The session principal every receipt written by this request carries.
    pub(crate) fn session_actor(&self) -> KernelActor {
        KernelActor::Operator(self.actor_id.clone())
    }

    /// Runs `operation` as the account's record user (table permissions apply).
    pub(crate) async fn run<T>(
        &self,
        state: &AppState,
        operation: impl std::future::Future<Output = T>,
    ) -> T {
        state
            .surreal
            .with_record_user_scope(self.record_user_scope.clone(), operation)
            .await
    }
}

/// MT-154: authenticates the account session and rechecks `capability_id` for an account-owned
/// surface. Every failure is the constant denial and happens before any product table is touched.
pub(crate) async fn authorize_account_session(
    state: &AppState,
    headers: &HeaderMap,
    capability_id: &'static str,
    action: ResourceAction,
) -> Result<AccountSessionAuthority, (StatusCode, Json<Value>)> {
    let session = authenticated_session_credentials(state, headers).await?;
    if !session_holds_capability(&session.context, capability_id) {
        return Err(constant_denial());
    }
    let record_user_scope = RecordUserScope {
        grant_id: None,
        workspace_id: None,
        // No protected-resource row exists for an account-owned surface; the receipt authority
        // anchor is the owning account (checked by `fn::mt154_account_receipt`).
        resource_id: session.context.identity.account_id.clone(),
        session_id: session.context.session_id.clone(),
        session_token: session.session_token,
        channel_binding_hash: Some(session.channel_binding_hash),
        capability_id: capability_id.to_owned(),
        action,
    };
    Ok(AccountSessionAuthority {
        actor_id: session.context.actor_id,
        record_user_scope,
    })
}

async fn logout(
    State(state): State<AccountBootstrapState>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Own completion before revocation so cancellation cannot drop vault cleanup.
    tokio::spawn(async move {
        let context = match authenticated_session_bootstrap(&state, &headers).await {
            Ok(context) => context,
            Err(denial) => {
                let channel = crate::api::stage::capture_channel_binding(&headers)
                    .map_err(|_| constant_denial())?;
                let token = headers
                    .get(HSK_HEADER_SESSION_TOKEN)
                    .and_then(|value| value.to_str().ok())
                    .filter(|value| {
                        value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
                    })
                    .ok_or_else(constant_denial)?;
                if let Some(id) = state
                    .surreal
                    .retired_session_cleanup_id(token, &channel.binding_hash)
                    .await
                    .map_err(|_| constant_denial())?
                {
                    delete_session_secret_with_retry(&state, &id).await?;
                }
                // This authority deletes one retired secret; it never authenticates content.
                return Err(denial);
            }
        };
        state
            .surreal
            .logout_local_account(&context)
            .await
            .map_err(|_| constant_denial())?;
        #[cfg(test)]
        account_lifecycle_probe(&state, "post_revoke").await?;
        delete_session_secret_with_retry(&state, &context.session_id).await?;
        Ok(Json(
            json!({"schema_version": "hsk.local_logout@1", "logged_out": true}),
        ))
    })
    .await
    .map_err(|_| constant_denial())?
}

async fn current_session(
    State(state): State<AccountBootstrapState>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let context = authenticated_session_bootstrap(&state, &headers).await?;
    Ok(Json(json!({"schema_version": "hsk.current_account@1",
        "account_id": context.identity.account_id, "principal_id": context.identity.principal_id,
        "access_space_id": context.identity.access_space_id, "session_id": context.session_id})))
}

/// Persist bearer material only in the existing OS-bound vault. The database stores hashes.
async fn record_login_event(
    state: &AccountBootstrapState,
    session: &IssuedSession,
    channel_hash: &str,
) -> Result<(), (StatusCode, Json<Value>)> {
    let context = state
        .surreal
        .authenticate_local_session(&session.token, channel_hash)
        .await
        .map_err(|error| {
            #[cfg(test)]
            eprintln!("LOCAL_ACCOUNT_TEST_FAILURE phase=login_audit_identity error={error}");
            tracing::error!(target: "handshake_core::resource_authority",
                phase = "login_event_identity", cause = session_authority_error_kind(&error),
                "local_authority_session_bootstrap_failed");
            constant_denial()
        })?;
    let event = crate::storage::surreal::local_accounts::account_event(
        crate::kernel::KernelEventType::LocalAccountLogin,
        &context.identity,
        Some(&context.session_id),
        context.policy_version,
        context.delegation_chain,
    )
    .map_err(|error| {
        tracing::error!(target: "handshake_core::resource_authority",
            phase = "login_event_build", cause = session_authority_error_kind(&error),
            "local_authority_session_bootstrap_failed");
        constant_denial()
    })?;
    state
        .storage
        .append_kernel_event(event)
        .await
        .map_err(|error| {
            #[cfg(test)]
            eprintln!("LOCAL_ACCOUNT_TEST_FAILURE phase=login_audit_append error={error}");
            tracing::error!(target: "handshake_core::resource_authority",
                phase = "login_event_append", cause_type = std::any::type_name_of_val(&error),
                cause = ?std::mem::discriminant(&error),
                "local_authority_session_bootstrap_failed");
            constant_denial()
        })?;
    Ok(())
}

// Revocation stays immediate; transient vault errors do not require a discarded UI context.
// Permanent failure remains an error and the exact retired-session cleanup route remains retryable.
async fn delete_session_secret_with_retry(
    state: &AccountBootstrapState,
    session_id: &str,
) -> Result<(), (StatusCode, Json<Value>)> {
    for attempt in 0..3 {
        match store_session_secret(state, session_id, None).await {
            Ok(()) => return Ok(()),
            Err(error) if attempt == 2 => return Err(error),
            Err(_) => tokio::time::sleep(std::time::Duration::from_millis(100)).await,
        }
    }
    Err(constant_denial())
}
async fn store_session_secret(
    state: &AccountBootstrapState,
    session_id: &str,
    token: Option<String>,
) -> Result<(), (StatusCode, Json<Value>)> {
    #[cfg(feature = "os-keychain")]
    {
        use crate::model_runtime::cloud::secrets_vault::{
            OsKeychainSecretsVault, SecretsVault, SecretsVaultError,
        };
        use sha2::{Digest, Sha256};
        // Namespace includes this installation's storage root; independent installations and
        // proof databases cannot overwrite each other's OS credentials.
        let config = state.surreal.config();
        let identity = serde_json::to_vec(&(
            config.path().to_string_lossy(),
            config.namespace(),
            config.database(),
        ))
        .map_err(|_| {
            tracing::error!(target: "handshake_core::resource_authority",
                phase = "vault_namespace", cause = "serialization",
                "local_authority_session_bootstrap_failed");
            constant_denial()
        })?;
        let installation = hex::encode(Sha256::digest(identity));
        let namespace = format!("handshake-local-accounts-{installation}");
        #[cfg(test)]
        if token.is_none() {
            account_lifecycle_probe(state, "vault_delete").await?;
        }
        let key = session_id.to_owned();
        let is_write = token.is_some();
        // Test builds record every written lane so the store teardown deletes it
        // (storage::tests::shutdown_and_remove_test_store).
        #[cfg(test)]
        if token.is_some() {
            crate::storage::tests::record_test_vault_lane(
                config.path().to_path_buf(),
                namespace.clone(),
                key.clone(),
            );
        }
        run_session_vault_operation(move || {
            let vault = OsKeychainSecretsVault::new(namespace);
            match token {
                Some(token) => vault.put(&key, &token),
                None => vault.delete(&key),
            }
        })
        .await?
        .map_err(|error| {
            if is_write {
                let windows_error_code = match &error {
                    SecretsVaultError::KeychainBackend(message) => message
                        .strip_prefix("Platform secure storage failure: Windows error code ")
                        .and_then(|code| code.parse::<u32>().ok()),
                    _ => None,
                };
                tracing::error!(
                    target: "handshake_core::local_account_vault",
                    phase = "session_exchange_vault_write",
                    keychain_backend = matches!(&error, SecretsVaultError::KeychainBackend(_)),
                    ?windows_error_code,
                    "local_account_session_vault_write_failed"
                );
            }
            #[cfg(test)]
            eprintln!("LOCAL_ACCOUNT_TEST_FAILURE phase=os_vault_operation error={error}");
            let _ = error;
            constant_denial()
        })
    }
    #[cfg(not(feature = "os-keychain"))]
    {
        let _ = (state, session_id, token);
        tracing::error!(target: "handshake_core::resource_authority",
            phase = "session_vault", cause = "os_keychain_feature_disabled",
            "local_authority_session_bootstrap_failed");
        Err(constant_denial())
    }
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
    authorize_resource_request(&state.surreal, &state.capability_registry, headers,
        capability_id, resource_kind, external_resource_id, action).await
}

pub(crate) async fn authorize_bootstrap_request(
    state: &AccountBootstrapState,
    headers: &HeaderMap,
    capability_id: &'static str,
    resource_kind: ResourceKind,
    external_resource_id: &str,
    action: ResourceAction,
) -> Result<AuthorizedResourceContext, (StatusCode, Json<Value>)> {
    authorize_resource_request(&state.surreal, &state.capability_registry, headers,
        capability_id, resource_kind, external_resource_id, action).await
}

async fn authorize_resource_request(
    storage: &crate::storage::surreal::SurrealStorage,
    registry: &crate::capabilities::CapabilityRegistry,
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
    let decision = storage
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

    if !registry
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
            grant_id: Some(decision.grant_id.clone()),
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

fn session_authority_error_kind(
    error: &crate::storage::surreal::resource_authority::ResourceAuthorityError,
) -> &'static str {
    use crate::storage::surreal::resource_authority::ResourceAuthorityError;
    match error {
        ResourceAuthorityError::Storage(_) => "Storage",
        ResourceAuthorityError::InvalidInput(_) => "InvalidInput",
        ResourceAuthorityError::Denied { .. } => "Denied",
        ResourceAuthorityError::Entropy(_) => "Entropy",
    }
}

async fn exchange_local_session(
    State(state): State<AccountBootstrapState>,
    headers: HeaderMap,
    Json(exchange): Json<LocalSessionExchange>,
) -> Result<Json<LocalSessionResponse>, (StatusCode, Json<Value>)> {
    let channel = crate::api::stage::capture_channel_binding(&headers).map_err(|error| {
        tracing::error!(target: "handshake_core::resource_authority",
            phase = "channel_binding", cause = ?error,
            "local_authority_session_bootstrap_failed");
        constant_denial()
    })?;
    // Own completion before consuming the credential or creating durable authority.
    tokio::spawn(async move {
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
                #[cfg(test)]
                eprintln!("LOCAL_ACCOUNT_TEST_FAILURE phase=session_exchange error={error}");
                tracing::error!(
                    target: "handshake_core::resource_authority",
                    phase = "credential_exchange", cause = session_authority_error_kind(&error),
                    "local_authority_session_bootstrap_failed"
                );
                constant_denial()
            })?;
        #[cfg(test)]
        account_lifecycle_probe(&state, "post_session").await?;
        if let Err(error) =
            store_session_secret(&state, &session.session_id, Some(session.token.clone())).await
        {
            let _ = state.surreal.revoke_session(&session.session_id).await;
            return Err(error);
        }
        #[cfg(test)]
        account_lifecycle_probe(&state, "post_vault").await?;
        if let Err(error) = record_login_event(&state, &session, &channel.binding_hash).await {
            let _ = state.surreal.revoke_session(&session.session_id).await;
            let _ = delete_session_secret_with_retry(&state, &session.session_id).await;
            return Err(error);
        }
        Ok(Json(LocalSessionResponse {
            schema_version: "hsk.authenticated_session@1",
            session_token: session.token,
            account_id: session.account_id,
            principal_id: session.principal_id,
            session_id: session.session_id,
            access_space_id: session.access_space_id,
            expires_at: session.expires_at,
        }))
    })
    .await
    .map_err(|error| {
        tracing::error!(target: "handshake_core::resource_authority",
            phase = "session_worker", cancelled = error.is_cancelled(), panicked = error.is_panic(),
            "local_authority_session_bootstrap_failed");
        constant_denial()
    })?
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
