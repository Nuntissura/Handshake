use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use surrealdb::{
    opt::auth::Record as RecordSignin,
    types::{Datetime, RecordId, RecordIdKey, SurrealValue},
};
use thiserror::Error;
use uuid::Uuid;

use super::{SurrealStorage, SurrealStorageError};

// One owning canonical grant predicate shared by public authorization and the
// private publication fence; the latter uses a canonical located resource ID.
// Resource table permission already enforces its creator; ordinary queries never
// re-read the hidden external locator through mt109_has_grant.
macro_rules! canonical_grant_query {
    ($projection:literal, $extra:literal, $limit:literal) => {
        concat!($projection, " FROM (SELECT * FROM resource_grants WHERE account_id = $auth.account_id AND principal_id = $auth.principal_id AND access_space_id = $auth.access_space_id AND resource_id = $resource) WHERE status = 'active' AND revoked_at = NONE AND (expires_at = NONE OR expires_at > time::now()) AND resource_id.lifecycle_state = 'active' AND resource_id.owner_account_id = $auth.account_id AND resource_id.access_space_id = $auth.access_space_id AND actions CONTAINS $action AND capability_ids CONTAINS $capability AND ($auth.delegated_capabilities CONTAINS '*' OR $auth.delegated_capabilities CONTAINS $capability) AND delegation_chain = $auth.delegation_chain AND ($kind != 'reconciliation_queue' OR (principal_id.principal_kind = 'service_identity' AND principal_id.capability_profile_id = 'MT109Reconciler'))", $extra, " LIMIT ", $limit, ";")
    };
}

#[derive(Clone, Debug, Eq, PartialEq, SurrealValue)]
struct RegistryPublicationAuthority {
    grant_id: RecordId,
    resource_id: RecordId,
    account_id: RecordId,
    principal_id: RecordId,
    actor_kind: String,
    actor_id: String,
    session_id: RecordId,
    access_space_id: RecordId,
    grant_version: i64,
    policy_versions: Vec<i64>,
    revocation_epochs: Vec<i64>,
    actions: Vec<String>,
    capability_ids: Vec<String>,
    delegation_chain: Vec<String>,
    session_capabilities: Vec<String>,
    principal_capabilities: Vec<String>,
    capability_profile_id: String,
    statuses: Vec<String>,
    resource_kind: String,
    grant_expires_at: Option<Datetime>,
    session_expires_at: Datetime,
}

// Navigation captures remain separate from registry write authority.
#[derive(Clone, Debug, Eq, PartialEq, SurrealValue)]
pub(super) struct NavigationPublicationFacts {
    pub(super) grant_id: RecordId,
    pub(super) resource_id: RecordId,
    pub(super) account_id: RecordId,
    pub(super) principal_id: RecordId,
    pub(super) actor_kind: String,
    pub(super) actor_id: String,
    pub(super) session_id: RecordId,
    pub(super) access_space_id: RecordId,
    pub(super) grant_version: i64,
    pub(super) policy_versions: Vec<i64>,
    pub(super) revocation_epochs: Vec<i64>,
    pub(super) actions: Vec<String>,
    pub(super) capability_ids: Vec<String>,
    pub(super) delegation_chain: Vec<String>,
    pub(super) session_capabilities: Vec<String>,
    pub(super) principal_capabilities: Vec<String>,
    pub(super) capability_profile_id: String,
    pub(super) statuses: Vec<String>,
    pub(super) resource_kind: String,
    pub(super) grant_expires_at: Option<Datetime>,
    pub(super) session_expires_at: Datetime,
}

#[derive(Clone)]
pub(crate) struct CapturedActivationAuthority {
    operation_id: Uuid,
    scope: RecordUserScope,
    association: crate::swarm_orchestration::resource_scope::ResourceAccessGrantAssociation,
    facts: NavigationPublicationFacts,
}

impl CapturedActivationAuthority {
    pub(super) fn operation_id(&self) -> Uuid { self.operation_id }
    pub(super) fn scope(&self) -> &RecordUserScope { &self.scope }
    pub(super) fn association(&self) -> &crate::swarm_orchestration::resource_scope::ResourceAccessGrantAssociation {
        &self.association
    }
    pub(super) fn facts(&self) -> &NavigationPublicationFacts { &self.facts }
}

#[derive(Debug)]
pub(crate) struct ActivationSettlementError {
    pub(crate) cause: ResourceAuthorityError,
}

type ActivationPublicationFence = std::sync::Arc<tokio::sync::OwnedRwLockReadGuard<()>>;

pub(crate) enum ActivationObservationError {
    Authentication,
    NotFound,
    Unavailable,
}

fn activation_observation_authority_error(error: ResourceAuthorityError) -> ActivationObservationError {
    match error {
        ResourceAuthorityError::Storage(SurrealStorageError::Database(error)) if matches!(
            error.not_allowed_details(),
            Some(surrealdb::types::NotAllowedError::Auth(
                surrealdb::types::AuthError::TokenExpired | surrealdb::types::AuthError::SessionExpired
                | surrealdb::types::AuthError::InvalidAuth | surrealdb::types::AuthError::MissingUserOrPass
                | surrealdb::types::AuthError::InvalidPass | surrealdb::types::AuthError::NotAllowed { .. }
            ))
        ) => ActivationObservationError::Authentication,
        ResourceAuthorityError::Storage(_) => ActivationObservationError::Unavailable,
        _ => ActivationObservationError::Authentication,
    }
}

pub(crate) struct AuthorizedActivationObservation<'a> {
    envelope: &'a handshake_account_activation::ActivationOriginalEnvelope,
    exact: &'a crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution,
    outcome: Option<&'a super::workspace_activation_outcome::StoredActivationOutcome>,
}

impl AuthorizedActivationObservation<'_> {
    pub(crate) fn exact(&self) -> &crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution { self.exact }
    pub(crate) fn encode<R: Serialize>(&self, readiness: Option<&R>) -> Result<serde_json::Value, ResourceAuthorityError> {
        use handshake_account_activation::{WorkspaceActivationObservation, WorkspaceActivationOutcome,
            WorkspaceActivationReadiness, WorkspaceActivationAcknowledgement, ActivationPublicationWitness};
        let invalid=||ResourceAuthorityError::InvalidInput("activation observation encoding failed");
        let readiness=match readiness {
            Some(descriptor)=>WorkspaceActivationReadiness::Current {
                descriptor:serde_json::to_value(descriptor).map_err(|_|invalid())? },
            None=>WorkspaceActivationReadiness::Unavailable,
        };
        let outcome=match self.outcome.and_then(|outcome|outcome.applied()) {
            Some(applied)=>WorkspaceActivationOutcome::Applied {
                result:serde_json::from_value::<WorkspaceActivationAcknowledgement>(applied.result().clone()).map_err(|_|invalid())?,
                publication_witness:serde_json::from_value::<ActivationPublicationWitness<serde_json::Value>>(
                    applied.publication_witness().clone()).map_err(|_|invalid())?,
            },
            None=>WorkspaceActivationOutcome::Unresolved,
        };
        let envelope=self.envelope;
        WorkspaceActivationObservation {
            schema_id:"hsk.workspace_activation.observation@1".into(),operation_id:envelope.operation_id,
            owner_account_id:envelope.owner_account_id.clone(),actor_principal_id:envelope.actor_principal_id.clone(),
            authenticated_session_id:envelope.authenticated_session_id.clone(),access_space_id:envelope.access_space_id.clone(),
            workspace_id:envelope.workspace_id.clone(),readiness,outcome,
        }.to_value().map_err(|_|invalid())
    }
}

#[derive(Clone, SurrealValue)]
struct NavigationActivationBindings {
    token_hash: String,
    channel_hash: String,
    session: RecordId,
    resource: RecordId,
    original_grant: Option<RecordId>,
    workspace: String,
}

// This server-only transaction never authenticates a client as a privileged user.
// Every bookkeeping statement is bracketed by the same canonical session/grant snapshot.
const NAVIGATION_ACTIVATION_FACTS: &str = "
LET $activation_session = (SELECT * FROM authenticated_sessions WHERE id = $session AND token_hash = $token_hash AND channel_binding_hash = $channel_hash AND revoked_at = NONE AND expires_at > time::now() AND account_id.status = 'enabled' AND principal_id.status = 'enabled' AND access_space_id.status = 'active' AND principal_id.account_id = account_id AND access_space_id.account_id = account_id AND account_revocation_epoch = account_id.revocation_epoch AND principal_revocation_epoch = principal_id.revocation_epoch AND space_revocation_epoch = access_space_id.revocation_epoch AND policy_version >= account_id.policy_version AND policy_version >= principal_id.policy_version AND policy_version >= access_space_id.policy_version LIMIT 1)[0];
SELECT id AS grant_id, resource_id, account_id, principal_id, principal_id.actor_kind AS actor_kind, principal_id.actor_id AS actor_id, $activation_session.id AS session_id, access_space_id, grant_version, [policy_version, resource_id.policy_version, account_id.policy_version, principal_id.policy_version, access_space_id.policy_version, $activation_session.policy_version] AS policy_versions, [account_id.revocation_epoch, principal_id.revocation_epoch, access_space_id.revocation_epoch, $activation_session.account_revocation_epoch, $activation_session.principal_revocation_epoch, $activation_session.space_revocation_epoch] AS revocation_epochs, actions, capability_ids, delegation_chain, $activation_session.delegated_capabilities AS session_capabilities, principal_id.delegated_capabilities AS principal_capabilities, principal_id.capability_profile_id AS capability_profile_id, [status, resource_id.lifecycle_state, account_id.status, principal_id.status, access_space_id.status] AS statuses, resource_id.resource_kind AS resource_kind, expires_at AS grant_expires_at, $activation_session.expires_at AS session_expires_at FROM resource_grants WHERE $activation_session.id != NONE AND account_id = $activation_session.account_id AND principal_id = $activation_session.principal_id AND access_space_id = $activation_session.access_space_id AND resource_id = $resource AND ($original_grant = NONE OR id = $original_grant) AND resource_id.resource_kind = 'workspace' AND resource_id.external_resource_id = $workspace AND resource_id.owner_account_id = account_id AND resource_id.access_space_id = access_space_id AND resource_id.lifecycle_state = 'active' AND resource_id.created_by_principal_id.status = 'enabled' AND status = 'active' AND revoked_at = NONE AND (expires_at = NONE OR expires_at > time::now()) AND actions CONTAINS 'read' AND capability_ids CONTAINS 'model_lane.navigation.read' AND delegation_chain = $activation_session.delegation_chain AND policy_version <= $activation_session.policy_version AND resource_id.policy_version <= policy_version AND $activation_session.delegated_capabilities CONTAINS 'model_lane.navigation.read' AND principal_id.delegated_capabilities CONTAINS 'model_lane.navigation.read' ORDER BY id ASC LIMIT 2;";

async fn navigation_activation_facts(
    transaction: &super::SurrealTransactionContext<'_>,
    bindings: NavigationActivationBindings,
    registry: &crate::capabilities::CapabilityRegistry,
) -> Result<NavigationPublicationFacts, ResourceAuthorityError> {
    let mut rows: Vec<NavigationPublicationFacts> = transaction.query_values_at(NAVIGATION_ACTIVATION_FACTS, bindings, 1).await?;
    if rows.len() != 1 { return Err(ResourceAuthorityError::InvalidInput("activation navigation authority unavailable")); }
    let facts = rows.remove(0);
    if !registry.profile_can(&facts.capability_profile_id, "model_lane.navigation.read").unwrap_or(false)
        || !facts.statuses.iter().map(String::as_str).eq(["active","active","enabled","enabled","active"])
        || facts.resource_kind != "workspace"
    { return Err(ResourceAuthorityError::InvalidInput("activation navigation authority denied")); }
    Ok(facts)
}

impl CapturedActivationAuthority {
    fn bindings(&self) -> Result<NavigationActivationBindings, ResourceAuthorityError> {
        Ok(NavigationActivationBindings {
            token_hash: sha256_hex(self.scope.session_token.as_bytes()),
            channel_hash: self.scope.channel_binding_hash.clone().ok_or(ResourceAuthorityError::InvalidInput("activation channel unavailable"))?,
            session: RecordId::new("authenticated_sessions", self.scope.session_id.clone()),
            resource: self.facts.resource_id.clone(), original_grant: Some(self.facts.grant_id.clone()),
            workspace: self.association.exact.workspace_id.as_str().to_owned(),
        })
    }

    fn envelope(&self) -> Result<handshake_account_activation::ActivationOriginalEnvelope, ResourceAuthorityError> {
        let facts = &self.facts;
        Ok(handshake_account_activation::ActivationOriginalEnvelope {
            operation_id:self.operation_id,
            owner_account_id:record_key(facts.account_id.clone())?,
            actor_principal_id:record_key(facts.principal_id.clone())?,
            authenticated_session_id:record_key(facts.session_id.clone())?,
            access_space_id:record_key(facts.access_space_id.clone())?,
            workspace_id:self.association.exact.workspace_id.as_str().to_owned(),
            resource_id:record_key(facts.resource_id.clone())?,grant_id:record_key(facts.grant_id.clone())?,
            grant_version:facts.grant_version,policy_version:self.association.policy_version,
            policy_versions:facts.policy_versions.clone().try_into().map_err(|_|ResourceAuthorityError::InvalidInput("activation policy shape"))?,
            revocation_epochs:facts.revocation_epochs.clone().try_into().map_err(|_|ResourceAuthorityError::InvalidInput("activation epoch shape"))?,
            actor_kind:facts.actor_kind.clone(),actor_id:facts.actor_id.clone(),
            actions:facts.actions.clone(),capability_ids:facts.capability_ids.clone(),delegation_chain:facts.delegation_chain.clone(),
            session_capabilities:facts.session_capabilities.clone(),principal_capabilities:facts.principal_capabilities.clone(),
            capability_profile_id:facts.capability_profile_id.clone(),
            statuses:facts.statuses.clone().try_into().map_err(|_|ResourceAuthorityError::InvalidInput("activation status shape"))?,
            resource_kind:facts.resource_kind.clone(),capability_id:self.scope.capability_id.clone(),
            action:self.scope.action.as_str().to_owned(),purpose:self.association.purpose.clone(),
            grant_expires_at:facts.grant_expires_at.as_ref().map(|expiry|expiry.clone().into_inner().to_rfc3339()),
            session_expires_at:facts.session_expires_at.clone().into_inner().to_rfc3339(),
        })
    }

    fn require_unexpired(&self) -> Result<(), ResourceAuthorityError> {
        CanonicalNavigationPublicationAuthority { grant_expires_at:self.facts.grant_expires_at.clone(),
            session_expires_at:self.facts.session_expires_at.clone() }.require_unexpired()
    }
}

pub(crate) struct CanonicalNavigationPublicationAuthority {
    grant_expires_at: Option<Datetime>,
    session_expires_at: Datetime,
}

impl CanonicalNavigationPublicationAuthority {
    pub(crate) fn require_unexpired(&self) -> Result<(), ResourceAuthorityError> {
        let now = Utc::now();
        if self.session_expires_at.clone().into_inner() <= now
            || self.grant_expires_at.as_ref().is_some_and(|expiry| expiry.clone().into_inner() <= now)
        {
            return Err(ResourceAuthorityError::InvalidInput("navigation publication authority expired"));
        }
        Ok(())
    }
}

#[derive(Clone, SurrealValue)]
struct RegistryPublicationBindings {
    resource: RecordId,
    original_grant: RecordId,
    kind: String,
    action: String,
    capability: String,
}

const REGISTRY_PUBLICATION_AUTHORITY: &str = canonical_grant_query!(
    "SELECT id AS grant_id, resource_id, account_id, principal_id, principal_id.actor_kind AS actor_kind, principal_id.actor_id AS actor_id, $auth.id AS session_id, access_space_id, grant_version, [policy_version, resource_id.policy_version, account_id.policy_version, principal_id.policy_version, access_space_id.policy_version, $auth.id.policy_version] AS policy_versions, [account_id.revocation_epoch, principal_id.revocation_epoch, access_space_id.revocation_epoch, $auth.id.account_revocation_epoch, $auth.id.principal_revocation_epoch, $auth.id.space_revocation_epoch] AS revocation_epochs, actions, capability_ids, delegation_chain, $auth.id.delegated_capabilities AS session_capabilities, principal_id.delegated_capabilities AS principal_capabilities, principal_id.capability_profile_id AS capability_profile_id, [status, resource_id.lifecycle_state, account_id.status, principal_id.status, access_space_id.status] AS statuses, resource_id.resource_kind AS resource_kind, expires_at AS grant_expires_at, $auth.id.expires_at AS session_expires_at",
    " AND id = $original_grant AND fn::mt109_live_session() AND resource_id != NONE AND resource_id.resource_kind = $kind AND resource_id.policy_version <= policy_version AND policy_version <= $auth.policy_version AND ($auth.principal_id.delegated_capabilities CONTAINS '*' OR $auth.principal_id.delegated_capabilities CONTAINS $capability) AND $auth.id.revoked_at = NONE AND $auth.id.expires_at > time::now()",
    "2"
);

struct ProducerSourceFacts {
    request: AuthorizationRequest,
    decision: AuthorizationDecision,
    authority: RegistryPublicationAuthority,
}

struct ProducerSourceFactsCapture {
    operation_id: Uuid,
    credentials: RoleAuthorityAuthInput,
    session: super::local_accounts::LocalSessionContext,
    registry: crate::capabilities::CapabilityRegistry,
    lifecycle: crate::swarm_orchestration::resource_scope::ResourceAccessLifecycleRegistry,
    sources: Vec<ProducerSourceFacts>,
    fence: std::sync::Arc<tokio::sync::OwnedRwLockReadGuard<()>>,
}

#[derive(Debug, Error)]
enum ProducerRunFailureOrigin {
    #[error("producer run preflight failed: {0}")]
    Preflight(#[source] crate::swarm_orchestration::model_lane::ModelLaneError),
    #[error("producer run callback failed: {0}")]
    OriginalCallback(#[source] crate::swarm_orchestration::model_lane::ModelLaneError),
    #[error("producer run controller failed: {0}")]
    Controller(#[from] SurrealStorageError),
}

#[derive(Debug, Error)]
#[error("producer run operation {original_operation_id}: {origin}")]
struct ProducerRunFailure {
    original_operation_id: Uuid,
    #[source]
    origin: ProducerRunFailureOrigin,
}

impl SurrealStorage {
    async fn capture_producer_source_facts(
        &self,
        operation_id: Uuid,
        registry: &crate::capabilities::CapabilityRegistry,
        credentials: RoleAuthorityAuthInput,
        requests: Vec<AuthorizationRequest>,
    ) -> Result<ProducerSourceFactsCapture, ResourceAuthorityError> {
        if operation_id.is_nil() || requests.is_empty() {
            return Err(ResourceAuthorityError::InvalidInput("producer source capture requires operation and requests"));
        }
        if requests.iter().any(|request| {
            request.session_token != credentials.session_token
                || request.channel_binding_hash.as_deref() != Some(credentials.channel_binding_hash.as_str())
        }) {
            return Err(ResourceAuthorityError::InvalidInput("producer source credentials differ"));
        }
        let fence = std::sync::Arc::new(self.authority_publication_read().await?);
        let session = self.authenticate_local_session(
            &credentials.session_token, &credentials.channel_binding_hash,
        ).await?;
        let mut sources = Vec::with_capacity(requests.len());
        for request in requests {
            sources.push(self.read_producer_source_facts(request, &session, registry).await?);
        }
        let capture = ProducerSourceFactsCapture {
            operation_id, credentials, session, registry: registry.clone(),
            lifecycle: self.resource_access_lifecycle(), sources, fence,
        };
        self.revalidate_producer_source_facts(&capture).await?;
        Ok(capture)
    }

    async fn read_producer_source_facts(
        &self,
        request: AuthorizationRequest,
        session: &super::local_accounts::LocalSessionContext,
        registry: &crate::capabilities::CapabilityRegistry,
    ) -> Result<ProducerSourceFacts, ResourceAuthorityError> {
        let decision = self.authorize_protected_resource(request.clone()).await?;
        let denied = || ResourceAuthorityError::Denied { decision_id: decision.decision_id.clone() };
        if session.session_id != decision.session_id
            || session.identity.account_id != decision.account_id
            || session.identity.principal_id != decision.principal_id
            || session.identity.access_space_id != decision.access_space_id
            || session.delegation_chain != decision.delegation_chain
            || !registry.profile_can(&decision.capability_profile_id, &request.capability_id).unwrap_or(false)
        {
            return Err(denied());
        }
        let scope = RecordUserScope {
            grant_id: Some(decision.grant_id.clone()),
            workspace_id: (request.resource_kind == ResourceKind::Workspace)
                .then(|| request.external_resource_id.clone()),
            session_token: request.session_token.clone(),
            channel_binding_hash: request.channel_binding_hash.clone(),
            resource_id: decision.resource_id.clone(), session_id: decision.session_id.clone(),
            capability_id: request.capability_id.clone(), action: request.action,
        };
        let bindings = RegistryPublicationBindings {
            resource: RecordId::new("protected_resources", decision.resource_id.clone()),
            original_grant: RecordId::new("resource_grants", decision.grant_id.clone()),
            kind: request.resource_kind.as_str().to_owned(),
            action: request.action.as_str().to_owned(), capability: request.capability_id.clone(),
        };
        let rows: Vec<RegistryPublicationAuthority> = self.with_record_user_scope(
            scope, self.with_data_operation(move |database| {
                Box::pin(async move { database.query_values(REGISTRY_PUBLICATION_AUTHORITY, bindings).await })
            }),
        ).await?;
        let [authority] = rows.as_slice() else { return Err(denied()); };
        if authority.account_id != RecordId::new("local_accounts", decision.account_id.clone())
            || authority.principal_id != RecordId::new("principals", decision.principal_id.clone())
            || authority.session_id != RecordId::new("authenticated_sessions", decision.session_id.clone())
            || authority.access_space_id != RecordId::new("access_spaces", decision.access_space_id.clone())
            || authority.resource_id != RecordId::new("protected_resources", decision.resource_id.clone())
            || authority.grant_id != RecordId::new("resource_grants", decision.grant_id.clone())
            || authority.resource_kind != request.resource_kind.as_str()
            || authority.capability_profile_id != decision.capability_profile_id
            || authority.delegation_chain != decision.delegation_chain
            || authority.actor_id != session.actor_id || authority.actor_kind != session.actor_kind
        {
            return Err(denied());
        }
        CanonicalNavigationPublicationAuthority {
            grant_expires_at: authority.grant_expires_at.clone(),
            session_expires_at: authority.session_expires_at.clone(),
        }.require_unexpired()?;
        Ok(ProducerSourceFacts { request, decision, authority: authority.clone() })
    }

    async fn producer_source_run_authority(
        &self,
        capture: &std::sync::Arc<ProducerSourceFactsCapture>,
        source_index: usize,
        store: &crate::swarm_orchestration::model_lane::ModelLaneStore,
        action: ResourceAction,
    ) -> crate::swarm_orchestration::model_lane::ModelLaneResult<(RecordUserScope, super::ModelLaneOperationSubject)> {
        use crate::swarm_orchestration::model_lane::ModelLaneError;
        use crate::swarm_orchestration::resource_scope::{
            AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, ExactResourceScopeAttribution,
            OwnerAccountId, WorkspaceScopeRef,
        };

        self.revalidate_producer_source_facts(capture).await?;
        let denied = || ModelLaneError::AuthorityDenied("producer run differs from captured authority".into());
        let source = capture.sources.get(source_index).ok_or_else(denied)?;
        if source.request.resource_kind != ResourceKind::Workspace
            || source.request.action != action
        {
            return Err(denied());
        }
        let decision = &source.decision;
        let parse = |value: &str| Uuid::parse_str(value).map_err(|_| denied());
        let exact = ExactResourceScopeAttribution {
            owner_account_id: OwnerAccountId::from_uuid(parse(&decision.account_id)?),
            actor_principal_id: ActorPrincipalId::from_uuid(parse(&decision.principal_id)?),
            authenticated_session_id: AuthenticatedSessionRef::from_uuid(parse(&decision.session_id)?),
            access_space_id: AccessSpaceRef::from_uuid(parse(&decision.access_space_id)?),
            workspace_id: WorkspaceScopeRef::new(source.request.external_resource_id.clone())
                .map_err(|_| denied())?,
        };
        if store.access().exact_read_scope() != Some(&exact)
            || store.access().lifecycle_authority() != Some(&capture.lifecycle)
            || store.surreal_storage().resource_access_lifecycle() != capture.lifecycle
            || store.surreal_storage().config().namespace() != self.config().namespace()
            || store.surreal_storage().config().database() != self.config().database()
        {
            return Err(denied());
        }
        let scope = RecordUserScope {
            grant_id: Some(decision.grant_id.clone()),
            workspace_id: Some(source.request.external_resource_id.clone()),
            session_token: capture.credentials.session_token.clone(),
            channel_binding_hash: Some(capture.credentials.channel_binding_hash.clone()),
            resource_id: decision.resource_id.clone(),
            session_id: decision.session_id.clone(),
            capability_id: source.request.capability_id.clone(),
            action: source.request.action,
        };
        let subject = super::ModelLaneOperationSubject {
            kind: source.request.resource_kind,
            external_id: source.request.external_resource_id.clone(),
        };
        Ok((scope, subject))
    }

    async fn record_producer_source_run(
        &self,
        capture: &std::sync::Arc<ProducerSourceFactsCapture>,
        source_index: usize,
        store: crate::swarm_orchestration::model_lane::ModelLaneStore,
        input: crate::swarm_orchestration::model_lane::NewModelLaneRun,
    ) -> Result<crate::swarm_orchestration::model_lane::ModelLaneRunRecord, ProducerRunFailure> {
        let operation_id = capture.operation_id;
        let (scope, subject) = self.producer_source_run_authority(
            capture, source_index, &store, ResourceAction::Create,
        ).await.map_err(|error| ProducerRunFailure {
            original_operation_id: operation_id,
            origin: ProducerRunFailureOrigin::Preflight(error),
        })?;
        // The original fixture owner keeps its Arc through unresolved settlement.
        // The worker clone keeps the same fence while its callback is running.
        let capture = std::sync::Arc::clone(capture);
        let worker_storage = self.clone();
        super::with_model_lane_subject(subject, self.with_record_user_scope(
            scope,
            self.with_authenticated_transaction(operation_id, move |transaction| Box::pin(async move {
                let _transaction = transaction;
                worker_storage.revalidate_producer_source_facts(&capture).await
                    .map_err(|error| ProducerRunFailureOrigin::OriginalCallback(error.into()))?;
                let run = store.record_run(input).await
                    .map_err(ProducerRunFailureOrigin::OriginalCallback)?;
                worker_storage.revalidate_producer_source_facts(&capture).await
                    .map_err(|error| ProducerRunFailureOrigin::OriginalCallback(error.into()))?;
                Ok::<_, ProducerRunFailureOrigin>((run, capture))
            })),
        )).await.map(|(run, _capture)| run).map_err(|origin| ProducerRunFailure {
            original_operation_id: operation_id,
            origin,
        })
    }

    async fn replay_producer_source_run(
        &self,
        capture: &std::sync::Arc<ProducerSourceFactsCapture>,
        source_index: usize,
        store: &crate::swarm_orchestration::model_lane::ModelLaneStore,
        run_id: &str,
    ) -> crate::swarm_orchestration::model_lane::ModelLaneResult<crate::swarm_orchestration::model_lane::ModelLaneReplay> {
        let (scope, subject) = self.producer_source_run_authority(
            capture, source_index, store, ResourceAction::Read,
        ).await?;
        let replay = super::with_model_lane_subject(subject,
            self.with_record_user_scope(scope, store.replay_run(run_id)),
        ).await?;
        self.revalidate_producer_source_facts(capture).await?;
        Ok(replay)
    }

    async fn revalidate_producer_source_facts(
        &self,
        capture: &ProducerSourceFactsCapture,
    ) -> Result<(), ResourceAuthorityError> {
        if capture.lifecycle != self.resource_access_lifecycle()
            || capture.operation_id.is_nil() || capture.sources.is_empty()
        {
            return Err(ResourceAuthorityError::InvalidInput("producer source capture owner differs"));
        }
        let _fence = capture.fence.clone();
        let current = self.authenticate_local_session(
            &capture.credentials.session_token, &capture.credentials.channel_binding_hash,
        ).await?;
        let original = &capture.session;
        if current.identity.account_id != original.identity.account_id
            || current.identity.principal_id != original.identity.principal_id
            || current.identity.access_space_id != original.identity.access_space_id
            || current.session_id != original.session_id || current.actor_id != original.actor_id
            || current.actor_kind != original.actor_kind || current.principal_kind != original.principal_kind
            || current.policy_version != original.policy_version
            || current.delegation_chain != original.delegation_chain
            || current.delegated_capabilities != original.delegated_capabilities
        {
            return Err(ResourceAuthorityError::InvalidInput("producer source session changed"));
        }
        for source in &capture.sources {
            if source.request.session_token != capture.credentials.session_token
                || source.request.channel_binding_hash.as_deref()
                    != Some(capture.credentials.channel_binding_hash.as_str())
            {
                return Err(ResourceAuthorityError::InvalidInput("producer source credentials changed"));
            }
            let fresh = self.read_producer_source_facts(source.request.clone(), &current, &capture.registry).await?;
            if fresh.authority != source.authority
                || fresh.decision.policy_version != source.decision.policy_version
            {
                return Err(ResourceAuthorityError::InvalidInput("producer source authority changed"));
            }
        }
        for source in &capture.sources {
            CanonicalNavigationPublicationAuthority {
                grant_expires_at: source.authority.grant_expires_at.clone(),
                session_expires_at: source.authority.session_expires_at.clone(),
            }.require_unexpired()?;
        }
        Ok(())
    }
}

#[cfg(test)]
struct ActivationPreparedTestLatch {
    lifecycle: crate::swarm_orchestration::resource_scope::ResourceAccessLifecycleRegistry,
    prepared: tokio::sync::watch::Sender<bool>,
    released: tokio::sync::watch::Sender<bool>,
    timeout: Duration,
}

#[cfg(test)]
static ACTIVATION_PREPARED_TEST_LATCHES: std::sync::OnceLock<
    std::sync::Mutex<std::collections::HashMap<Uuid, std::sync::Weak<ActivationPreparedTestLatch>>>,
> = std::sync::OnceLock::new();

#[cfg(test)]
impl ActivationPreparedTestLatch {
    async fn wait_signal(&self, signal: &tokio::sync::watch::Sender<bool>) -> Result<(), ResourceAuthorityError> {
        let mut receiver = signal.subscribe();
        tokio::time::timeout(self.timeout, async {
            loop {
                if *receiver.borrow_and_update() { return Ok(()); }
                receiver.changed().await.map_err(|_| ResourceAuthorityError::InvalidInput("activation test latch closed"))?;
            }
        }).await.map_err(|_| ResourceAuthorityError::InvalidInput("activation test latch timed out"))?
    }

    async fn wait_prepared(&self) -> Result<(), ResourceAuthorityError> {
        self.wait_signal(&self.prepared).await
    }

    fn release(&self) { self.released.send_replace(true); }

    async fn observe_and_wait(&self) -> Result<(), ResourceAuthorityError> {
        self.prepared.send_replace(true);
        self.wait_signal(&self.released).await
    }
}

#[cfg(test)]
impl SurrealStorage {
    fn install_activation_prepared_test_latch(
        &self,
        operation_id: Uuid,
        timeout: Duration,
    ) -> Result<std::sync::Arc<ActivationPreparedTestLatch>, ResourceAuthorityError> {
        if operation_id.is_nil() || timeout.is_zero() {
            return Err(ResourceAuthorityError::InvalidInput("activation test latch requires operation and timeout"));
        }
        let registrations = ACTIVATION_PREPARED_TEST_LATCHES.get_or_init(Default::default);
        let mut registrations = registrations.lock()
            .map_err(|_| ResourceAuthorityError::InvalidInput("activation test latch registry poisoned"))?;
        registrations.retain(|_, latch| latch.strong_count() != 0);
        if registrations.contains_key(&operation_id) {
            return Err(ResourceAuthorityError::InvalidInput("activation test latch already installed"));
        }
        let (prepared, _) = tokio::sync::watch::channel(false);
        let (released, _) = tokio::sync::watch::channel(false);
        let latch = std::sync::Arc::new(ActivationPreparedTestLatch {
            lifecycle: self.resource_access_lifecycle(), prepared, released, timeout,
        });
        registrations.insert(operation_id, std::sync::Arc::downgrade(&latch));
        Ok(latch)
    }

    fn activation_prepared_test_latch(
        &self,
        operation_id: Uuid,
    ) -> Option<std::sync::Arc<ActivationPreparedTestLatch>> {
        let registrations = ACTIVATION_PREPARED_TEST_LATCHES.get()?;
        let latch = registrations.lock().ok()?.get(&operation_id)?.upgrade()?;
        (latch.lifecycle == self.resource_access_lifecycle()).then_some(latch)
    }
}

struct OwnedJobAdmission {
    operation_id: Uuid,
    write_authority: OwnedCanonicalAuthSnapshot,
    creation: handshake_job_admission::CheckedJobCreation,
    registry: crate::capabilities::CapabilityRegistry,
    lifecycle: crate::swarm_orchestration::resource_scope::ResourceAccessLifecycleRegistry,
}

#[derive(Clone)]
pub(crate) struct JobAdmissionTransactionRetention {
    owner: std::sync::Arc<OwnedJobAdmission>,
    authorization_fence: std::sync::Arc<tokio::sync::OwnedRwLockReadGuard<()>>,
}

impl JobAdmissionTransactionRetention {
    pub(super) fn operation_id(&self) -> Uuid { self.owner.operation_id }

    pub(crate) fn validate_creation(
        &self,
        operation_id: Uuid,
        creation: &handshake_job_admission::CheckedJobCreation,
    ) -> Result<(), ResourceAuthorityError> {
        if operation_id != self.owner.operation_id
            || creation.canonical_bytes() != self.owner.creation.canonical_bytes()
            || creation.sha256_hex() != self.owner.creation.sha256_hex()
        {
            return Err(ResourceAuthorityError::InvalidInput("job admission command differs from original capture"));
        }
        Ok(())
    }

    pub(super) fn matches_readback(&self, readback: &JobAdmissionReadbackToken) -> bool {
        self.matches_retention(&readback.read.retention)
    }

    pub(super) fn matches_retention(&self, other: &Self) -> bool {
        self.operation_id() == other.operation_id()
            && std::sync::Arc::ptr_eq(&self.owner, &other.owner)
            && std::sync::Arc::ptr_eq(&self.authorization_fence, &other.authorization_fence)
    }
}

pub(crate) struct JobAdmissionReadbackToken {
    read: std::sync::Arc<JobAdmissionReadbackTransaction>,
}

impl JobAdmissionReadbackToken {
    pub(super) fn operation_id(&self) -> Uuid { self.read.operation_id() }

    pub(super) fn require_unexpired(&self) -> Result<(), SurrealStorageError> {
        self.read.current_read.auth.require_unexpired().map_err(Into::into)
    }
}

struct CurrentJobCreationReadAuthority {
    auth: OwnedCanonicalAuthSnapshot,
    publication_fence: std::sync::Arc<tokio::sync::OwnedRwLockReadGuard<()>>,
}

pub(crate) struct JobAdmissionReadbackTransaction {
    retention: JobAdmissionTransactionRetention,
    current_read: CurrentJobCreationReadAuthority,
    creation: handshake_job_admission::CheckedJobCreation,
}

pub(crate) struct JobAdmissionReadbackResult {
    creation: crate::storage::CanonicalJobCreation,
    token: JobAdmissionReadbackToken,
}

impl JobAdmissionReadbackResult {
    pub(crate) fn confirm(
        self,
        storage: &SurrealStorage,
    ) -> Result<crate::storage::CanonicalJobCreation, crate::storage::JobAdmissionError> {
        if !self.token.read.belongs_to(storage) {
            return Err(crate::storage::JobAdmissionError::Denied);
        }
        self.token.read.current_read.auth.require_unexpired()
            .map_err(job_admission_authority_error)?;
        storage.confirm_job_admission_readback(self.token)
            .map_err(|error| crate::storage::JobAdmissionError::Storage(error.into()))?;
        Ok(self.creation)
    }
}

impl JobAdmissionReadbackTransaction {
    pub(super) fn operation_id(&self) -> Uuid { self.retention.operation_id() }
    pub(super) fn retention(&self) -> &JobAdmissionTransactionRetention { &self.retention }
    pub(super) fn scope(&self) -> RecordUserScope { self.current_read.auth.record_user_scope() }

    pub(super) fn belongs_to(&self, storage: &SurrealStorage) -> bool {
        self.retention.owner.lifecycle == storage.resource_access_lifecycle()
    }

    pub(super) fn matches_readback(
        self: &std::sync::Arc<Self>,
        token: &JobAdmissionReadbackToken,
    ) -> bool {
        std::sync::Arc::ptr_eq(self, &token.read)
            && self.retention.matches_readback(token)
    }

    pub(super) async fn execute(
        &self,
        transaction: &super::SurrealTransactionContext<'_>,
    ) -> Result<crate::storage::CanonicalJobCreation, crate::storage::JobAdmissionError> {
        self.current_read.auth.revalidate_in_transaction(transaction, &self.retention.owner.registry)
            .await.map_err(job_admission_authority_error)?;
        let observation = super::ai_job_store::read_creation_in_transaction(transaction, &self.creation)
            .await.map_err(crate::storage::JobAdmissionError::Storage)?;
        self.current_read.auth.revalidate_in_transaction(transaction, &self.retention.owner.registry)
            .await.map_err(job_admission_authority_error)?;
        match observation {
            crate::storage::JobCreationObservation::Observed(creation) => {
                self.validate_observed_creation(&creation)?;
                Ok(creation)
            }
            crate::storage::JobCreationObservation::Unknown => {
                Err(crate::storage::JobAdmissionError::Unknown("job_creation_readback_unresolved"))
            }
        }
    }

    fn validate_observed_creation(
        &self,
        observed: &crate::storage::CanonicalJobCreation,
    ) -> Result<(), crate::storage::JobAdmissionError> {
        self.retention.validate_creation(self.operation_id(), &self.creation)
            .map_err(job_admission_authority_error)?;
        if observed.creation().canonical_bytes() != self.creation.canonical_bytes()
            || observed.creation().sha256_hex() != self.creation.sha256_hex()
            || observed.job().job_id != self.creation.identity().job_id()
        {
            return Err(crate::storage::JobAdmissionError::Integrity("job creation readback differs from original basis"));
        }
        match (self.creation.identity().reserved_workflow_run_id(), observed.reserved_workflow()) {
            (None, None) => {}
            (Some(expected), Some(workflow))
                if workflow.id == expected && workflow.job_id == self.creation.identity().job_id() => {}
            _ => return Err(crate::storage::JobAdmissionError::Integrity("job creation readback workflow differs")),
        }
        Ok(())
    }

    pub(super) async fn finalize(
        self: std::sync::Arc<Self>,
        storage: &SurrealStorage,
        observed: crate::storage::CanonicalJobCreation,
    ) -> Result<JobAdmissionReadbackResult, crate::storage::JobAdmissionError> {
        if self.retention.owner.lifecycle != storage.resource_access_lifecycle() {
            return Err(crate::storage::JobAdmissionError::Denied);
        }
        self.current_read.auth.revalidate_for_publication(storage, &self.retention.owner.registry)
            .await.map_err(job_admission_authority_error)?;
        self.validate_observed_creation(&observed)?;
        self.current_read.auth.require_unexpired().map_err(job_admission_authority_error)?;
        Ok(JobAdmissionReadbackResult {
            creation: observed,
            token: JobAdmissionReadbackToken { read: self },
        })
    }
}

impl SurrealStorage {
    pub(crate) async fn capture_job_admission_readback(
        &self,
        admission: &crate::storage::JobAdmission,
        request: AuthorizationRequest,
    ) -> Result<JobAdmissionReadbackTransaction, crate::storage::JobAdmissionError> {
        let initial = admission.initial_result();
        if initial.rejected.is_some() || !matches!(initial.primary, Some(Ok(Ok(_)))) {
            return Err(crate::storage::JobAdmissionError::Unknown("job_commit_not_observed"));
        }
        let retention = admission.retention();
        retention.validate_creation(admission.operation_id(), admission.command().checked())
            .map_err(job_admission_authority_error)?;
        let current_read = self.capture_current_job_creation_read_authority(request, retention)
            .await.map_err(job_admission_authority_error)?;
        Ok(JobAdmissionReadbackTransaction {
            retention: retention.clone(), current_read,
            creation: admission.command().checked().clone(),
        })
    }

    pub(crate) async fn execute_job_admission(
        &self,
        command: crate::storage::JobCreationCommand,
        cancellation: std::sync::Arc<super::ContextOperationCancellation>,
        retention: JobAdmissionTransactionRetention,
    ) -> Result<crate::storage::JobCreateCommitOutcome, crate::storage::JobAdmissionError> {
        retention.validate_creation(retention.operation_id(), command.checked())
            .map_err(job_admission_authority_error)?;
        if retention.owner.lifecycle != self.resource_access_lifecycle() {
            return Err(crate::storage::JobAdmissionError::Denied);
        }
        let scope = retention.owner.write_authority.record_user_scope();
        let captured = retention.clone();
        let storage = self.clone();
        self.with_record_user_scope(scope, self.with_owned_authenticated_job_transaction(
            retention.operation_id(), cancellation, retention,
            move |transaction| Box::pin(async move {
                storage.revalidate_job_admission_in_transaction(&transaction, &captured).await
                    .map_err(job_admission_authority_storage_error)?;
                captured.validate_creation(captured.operation_id(), command.checked())
                    .map_err(job_admission_authority_storage_error)?;
                let outcome = super::ai_job_store::create_in_transaction(
                    &transaction, &command, &captured,
                ).await?;
                storage.revalidate_job_admission_in_transaction(&transaction, &captured).await
                    .map_err(job_admission_authority_storage_error)?;
                Ok::<_, crate::storage::StorageError>(outcome)
            }),
        )).await.map_err(crate::storage::JobAdmissionError::Storage)
    }

    pub(crate) async fn capture_job_admission(
        &self,
        request: AuthorizationRequest,
        registry: &crate::capabilities::CapabilityRegistry,
        operation_id: Uuid,
        creation: handshake_job_admission::CheckedJobCreation,
    ) -> Result<crate::storage::JobAdmission, crate::storage::JobAdmissionError> {
        use crate::storage::JobAdmissionError;
        let command = crate::storage::JobCreationCommand::from_checked(creation.clone())?;
        let retention = self.capture_job_creation_write_authority(
            request, registry, operation_id, creation,
        ).await.map_err(job_admission_authority_error)?;
        let auth = &retention.owner.write_authority;
        self.with_record_user_scope(auth.record_user_scope(), async {
            let descriptor = retention.owner.creation.descriptor();
            for entity in descriptor.entity_refs.iter().chain(
                descriptor.planned_operations.iter().map(|operation| &operation.target),
            ) {
                match entity.entity_kind.as_str() {
                    "workspace" if entity.entity_id != auth.workspace_id => {
                        return Err(JobAdmissionError::Denied);
                    }
                    "document" => {
                        let document = self.get_document(&entity.entity_id).await
                            .map_err(|_| JobAdmissionError::Denied)?;
                        if document.workspace_id != auth.workspace_id {
                            return Err(JobAdmissionError::Denied);
                        }
                    }
                    _ => {}
                }
            }
            auth.revalidate_for_publication(self, registry).await
                .map_err(job_admission_authority_error)
        }).await?;
        crate::storage::JobAdmission::from_captured_authority(
            operation_id, super::ContextOperationCancellation::new(), command, retention,
        )
    }

    async fn capture_job_creation_write_authority(
        &self,
        request: AuthorizationRequest,
        registry: &crate::capabilities::CapabilityRegistry,
        operation_id: Uuid,
        creation: handshake_job_admission::CheckedJobCreation,
    ) -> Result<JobAdmissionTransactionRetention, ResourceAuthorityError> {
        if request.resource_kind != ResourceKind::Workspace
            || request.action != ResourceAction::Create || request.capability_id != "fs.write"
            || request.external_resource_id != creation.scope().workspace_id.as_str()
        {
            return Err(ResourceAuthorityError::InvalidInput("job creation requires exact workspace Create authority"));
        }
        let credentials = RoleAuthorityAuthInput::new(
            request.session_token.clone(), request.channel_binding_hash.clone()
                .ok_or(ResourceAuthorityError::InvalidInput("job creation requires channel binding"))?,
        );
        let auth = capture_owned_canonical_role_auth(self, registry, credentials, request).await?;
        if &auth.exact != creation.scope() {
            return Err(ResourceAuthorityError::InvalidInput("job creation basis differs from authenticated scope"));
        }
        let descriptor = creation.descriptor();
        let profile_kind = if descriptor.job_kind == "model_run" { "workflow_run" }
            else { descriptor.job_kind.as_str() };
        let profile = registry.profile_for_job_request(profile_kind, &descriptor.protocol_id)
            .map_err(|_| ResourceAuthorityError::InvalidInput("job creation capability profile unavailable"))?;
        if profile.id != descriptor.capability_profile_id {
            return Err(ResourceAuthorityError::InvalidInput("job creation capability profile differs"));
        }
        let fence = std::sync::Arc::new(self.authority_publication_read().await?);
        auth.revalidate_for_publication(self, registry).await?;
        Ok(JobAdmissionTransactionRetention {
            owner: std::sync::Arc::new(OwnedJobAdmission {
                operation_id, write_authority: auth, creation, registry: registry.clone(),
                lifecycle: self.resource_access_lifecycle(),
            }),
            authorization_fence: fence,
        })
    }

    pub(crate) async fn revalidate_job_admission_in_transaction(
        &self,
        transaction: &super::SurrealTransactionContext<'_>,
        retention: &JobAdmissionTransactionRetention,
    ) -> Result<(), ResourceAuthorityError> {
        if retention.owner.lifecycle != self.resource_access_lifecycle() {
            return Err(ResourceAuthorityError::InvalidInput("job admission storage lifecycle differs"));
        }
        retention.owner.write_authority.revalidate_in_transaction(
            transaction, &retention.owner.registry,
        ).await
    }

    async fn capture_current_job_creation_read_authority(
        &self,
        request: AuthorizationRequest,
        retention: &JobAdmissionTransactionRetention,
    ) -> Result<CurrentJobCreationReadAuthority, ResourceAuthorityError> {
        if retention.owner.lifecycle != self.resource_access_lifecycle()
            || request.resource_kind != ResourceKind::Workspace
            || request.action != ResourceAction::Read || request.capability_id != "fs.read"
            || request.external_resource_id != retention.owner.creation.scope().workspace_id.as_str()
        {
            return Err(ResourceAuthorityError::InvalidInput("job observation requires current workspace Read authority"));
        }
        let credentials = RoleAuthorityAuthInput::new(
            request.session_token.clone(), request.channel_binding_hash.clone()
                .ok_or(ResourceAuthorityError::InvalidInput("job observation requires channel binding"))?,
        );
        let auth = capture_owned_canonical_role_auth(
            self, &retention.owner.registry, credentials, request,
        ).await?;
        let original = retention.owner.creation.scope();
        if auth.exact.owner_account_id != original.owner_account_id
            || auth.exact.access_space_id != original.access_space_id
            || auth.exact.workspace_id != original.workspace_id
        {
            return Err(ResourceAuthorityError::InvalidInput("job observation target differs from original creation"));
        }
        let publication_fence = retention.authorization_fence.clone();
        auth.revalidate_for_publication(self, &retention.owner.registry).await?;
        Ok(CurrentJobCreationReadAuthority { auth, publication_fence })
    }
}

fn job_admission_authority_error(error: ResourceAuthorityError) -> crate::storage::JobAdmissionError {
    match error {
        ResourceAuthorityError::Storage(error) => crate::storage::JobAdmissionError::Storage(error.into()),
        _ => crate::storage::JobAdmissionError::Denied,
    }
}

fn job_admission_authority_storage_error(error: ResourceAuthorityError) -> crate::storage::StorageError {
    match error {
        ResourceAuthorityError::Storage(error) => error.into(),
        _ => crate::storage::StorageError::Guard("HSK-403-PROTECTED-RESOURCE"),
    }
}

pub(crate) struct RoleAuthorityAuthInput {
    session_token: String,
    channel_binding_hash: String,
}

pub(crate) struct WorkspaceOwnerActionPermit {
    auth: OwnedCanonicalAuthSnapshot,
    registry: std::sync::Arc<crate::capabilities::CapabilityRegistry>,
    command: crate::swarm_orchestration::model_lane::WorkspaceOwnerActionCommand,
}

pub(crate) struct RoleAuthorityConsumptionPermit {
    capture: RoleConsumptionCapture,
    preparation: Option<RoleAuthorityTransactionRetention>,
}

enum RoleConsumptionCapture {
    Consumer(std::sync::Arc<OwnedRoleConsumption>),
    Routing(std::sync::Arc<OwnedRoleTransaction>),
}

impl RoleConsumptionCapture {
    fn auth(&self) -> &OwnedCanonicalAuthSnapshot {
        match self {
            Self::Consumer(capture) => &capture.auth,
            Self::Routing(owner) => owner.auth(),
        }
    }

    fn command(&self) -> &crate::swarm_orchestration::model_lane::RoleConsumptionCommand {
        match self {
            Self::Consumer(capture) => &capture.command,
            Self::Routing(owner) => owner.consumer_command()
                .expect("routing permits are minted only after command initialization"),
        }
    }
}

struct OwnedRoleConsumption {
    auth: OwnedCanonicalAuthSnapshot,
    registry: std::sync::Arc<crate::capabilities::CapabilityRegistry>,
    command: crate::swarm_orchestration::model_lane::RoleConsumptionCommand,
}

impl RoleAuthorityConsumptionPermit {
    pub(crate) fn current_scope(&self) -> &crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution {
        &self.capture.auth().exact
    }

    pub(crate) fn matches_routing_stage(&self, execution: &str, stage: &str, message: &str) -> bool {
        matches!(self.capture.command(),
            crate::swarm_orchestration::model_lane::RoleConsumptionCommand::AcknowledgeRoutingStage {
                execution_id, stage_id, message_id, ..
            } if execution_id == execution && stage_id == stage && message_id == message)
    }

    async fn into_transaction_retention(self, storage: &SurrealStorage) -> Result<RoleAuthorityTransactionRetention, ResourceAuthorityError> {
        if let Some(retention) = self.preparation {
            let same_owner = match (&self.capture, retention.owner.as_ref()) {
                (RoleConsumptionCapture::Consumer(expected), OwnedRoleTransaction::Consumer(actual)) =>
                    std::sync::Arc::ptr_eq(expected, actual),
                (RoleConsumptionCapture::Routing(expected), OwnedRoleTransaction::RoutingCapture(_)) =>
                    std::sync::Arc::ptr_eq(expected, &retention.owner),
                _ => false,
            };
            if same_owner && retention.operation_id() == self.capture.command().operation_id() {
                return Ok(retention);
            }
            return Err(ResourceAuthorityError::InvalidInput("prepared routing custody identity changed"));
        }
        match self.capture {
            RoleConsumptionCapture::Consumer(capture) => Ok(RoleAuthorityTransactionRetention {
                owner: std::sync::Arc::new(OwnedRoleTransaction::Consumer(capture)),
                fence: std::sync::Arc::new(storage.authority_publication_read().await?),
            }),
            RoleConsumptionCapture::Routing(_) => Err(ResourceAuthorityError::InvalidInput(
                "routing capture cannot replace its original retention")),
        }
    }
}

enum OwnedRoleTransaction {
    Owner(WorkspaceOwnerActionPermit),
    Consumer(std::sync::Arc<OwnedRoleConsumption>),
    RoutingCapture(OwnedRoutingCapture),
}

struct RoutingCaptureRequest {
    execution_id: String,
    stage_id: String,
    response_message_id: String,
}

struct OwnedRoutingCapture {
    operation_id: Uuid,
    auth: OwnedCanonicalAuthSnapshot,
    registry: std::sync::Arc<crate::capabilities::CapabilityRegistry>,
    request: RoutingCaptureRequest,
    command: std::sync::OnceLock<crate::swarm_orchestration::model_lane::RoleConsumptionCommand>,
}

impl OwnedRoleTransaction {
    fn operation_id(&self) -> Uuid {
        match self {
            Self::Owner(permit) => permit.command.operation_id(),
            Self::Consumer(permit) => permit.command.operation_id(),
            Self::RoutingCapture(capture) => capture.operation_id,
        }
    }

    fn auth(&self) -> &OwnedCanonicalAuthSnapshot {
        match self {
            Self::Owner(permit) => &permit.auth,
            Self::Consumer(permit) => &permit.auth,
            Self::RoutingCapture(capture) => &capture.auth,
        }
    }

    fn consumer_command(&self) -> Option<&crate::swarm_orchestration::model_lane::RoleConsumptionCommand> {
        match self {
            Self::Consumer(capture) => Some(&capture.command),
            Self::RoutingCapture(capture) => capture.command.get(),
            Self::Owner(_) => None,
        }
    }

    fn consumer_registry(&self) -> Option<&crate::capabilities::CapabilityRegistry> {
        match self {
            Self::Consumer(capture) => Some(capture.registry.as_ref()),
            Self::RoutingCapture(capture) => Some(capture.registry.as_ref()),
            Self::Owner(_) => None,
        }
    }
}

enum OperatorConsumptionReadback {
    Review(super::model_lane::RoleEvidenceReadback),
    Routing {
        prior: super::model_lane::RoleEvidenceReadback,
        acknowledgement: super::model_lane::RoleEvidenceReadback,
        routing: crate::swarm_orchestration::model_lane::RoutingAuthorityCommitReadback,
    },
}

#[derive(Clone)]
pub(crate) struct RoleAuthorityTransactionRetention {
    owner: std::sync::Arc<OwnedRoleTransaction>,
    fence: std::sync::Arc<tokio::sync::OwnedRwLockReadGuard<()>>,
}

impl RoleAuthorityTransactionRetention {
    pub(crate) fn operation_id(&self) -> Uuid { self.owner.operation_id() }

    pub(super) fn retains_capture_on_failure(&self) -> bool {
        matches!(self.owner.as_ref(), OwnedRoleTransaction::RoutingCapture(_))
    }

    pub(crate) fn matches_retention(&self, other: &Self) -> bool {
        self.operation_id() == other.operation_id()
            && std::sync::Arc::ptr_eq(&self.owner, &other.owner)
            && std::sync::Arc::ptr_eq(&self.fence, &other.fence)
    }

    pub(crate) fn matches_cancellation(&self, cancellation: &RoleAuthorityPreparationCancellation) -> bool {
        self.operation_id() == cancellation.operation_id
            && self.matches_retention(&cancellation.retention)
    }

    pub(crate) fn matches_completion(&self, completion: &RoleAuthorityPublicationCompletion) -> bool {
        self.operation_id() == completion.operation_id
            && std::sync::Arc::ptr_eq(&self.owner, &completion.owner)
    }
}

pub(crate) struct RoleAuthorityPreparationCancellation {
    operation_id: Uuid,
    retention: RoleAuthorityTransactionRetention,
}

impl RoleAuthorityPreparationCancellation {
    pub(crate) fn operation_id(&self) -> Uuid { self.operation_id }
}

pub(crate) struct PreparedRoutingAuthorityConsumption {
    storage: SurrealStorage,
    retention: RoleAuthorityTransactionRetention,
    run: crate::swarm_orchestration::model_lane::ModelLaneRunRecord,
}

impl PreparedRoutingAuthorityConsumption {
    pub(crate) fn run(&self) -> &crate::swarm_orchestration::model_lane::ModelLaneRunRecord {
        &self.run
    }

    pub(crate) fn into_permit(self) -> RoleAuthorityConsumptionPermit {
        let capture = match self.retention.owner.as_ref() {
            OwnedRoleTransaction::Consumer(capture) => RoleConsumptionCapture::Consumer(capture.clone()),
            OwnedRoleTransaction::RoutingCapture(capture) if capture.command.get().is_some() =>
                RoleConsumptionCapture::Routing(self.retention.owner.clone()),
            _ => unreachable!("prepared routing custody requires an initialized consumer command"),
        };
        RoleAuthorityConsumptionPermit {
            capture, preparation: Some(self.retention),
        }
    }

    pub(crate) fn cancel(self) -> Result<(), SurrealStorageError> {
        self.storage.cancel_role_preparation(RoleAuthorityPreparationCancellation {
            operation_id: self.retention.operation_id(), retention: self.retention,
        })
    }
}

pub(crate) struct RoleAuthorityPublicationCompletion {
    operation_id: Uuid,
    owner: std::sync::Arc<OwnedRoleTransaction>,
}

pub(crate) struct RoutingPriorEvidenceReadback {
    receipt: handshake_role_authority::domain::RoleReceipt,
    issuance_operation_id: Uuid,
}

pub(crate) struct RoutingAuthorityCapturePayload {
    inputs: crate::swarm_orchestration::model_lane::RoutingAuthorityCaptureInputs,
    run: crate::swarm_orchestration::model_lane::ModelLaneRunRecord,
    prior: RoutingPriorEvidenceReadback,
}

pub(crate) struct RoutingAuthorityCaptureTransaction {
    storage: SurrealStorage,
    retention: RoleAuthorityTransactionRetention,
    store: crate::swarm_orchestration::model_lane::ModelLaneStore,
    provider: super::model_lane::SurrealModelLaneStore,
}

pub(crate) struct RoutingAuthorityCaptureRead {
    storage: SurrealStorage,
    retention: RoleAuthorityTransactionRetention,
    inputs: crate::swarm_orchestration::model_lane::RoutingAuthorityCaptureInputs,
    run: crate::swarm_orchestration::model_lane::ModelLaneRunRecord,
    prior: RoutingPriorEvidenceReadback,
}

pub(crate) struct RoutingAuthorityCaptureFinalization {
    storage: SurrealStorage,
    retention: RoleAuthorityTransactionRetention,
    inputs: crate::swarm_orchestration::model_lane::RoutingAuthorityCaptureInputs,
    run: crate::swarm_orchestration::model_lane::ModelLaneRunRecord,
    prior: RoutingPriorEvidenceReadback,
    command: crate::swarm_orchestration::model_lane::RoleConsumptionCommand,
}

impl RoutingAuthorityCaptureTransaction {
    pub(super) fn operation_id(&self) -> Uuid { self.retention.operation_id() }
    pub(super) fn retention(&self) -> &RoleAuthorityTransactionRetention { &self.retention }
    pub(super) fn belongs_to(&self, storage: &SurrealStorage) -> bool {
        self.storage.resource_access_lifecycle() == storage.resource_access_lifecycle()
    }
    pub(super) fn record_user_scope(&self) -> RecordUserScope { self.retention.owner.auth().record_user_scope() }

    pub(super) async fn execute<'tx>(
        &'tx self,
        transaction: &'tx super::SurrealTransactionContext<'tx>,
    ) -> crate::swarm_orchestration::model_lane::ModelLaneResult<RoutingAuthorityCapturePayload> {
        use crate::swarm_orchestration::model_lane::ModelLaneError;
        use crate::swarm_orchestration::routing::ModelLaneRoutingDispatchTarget;
        use handshake_role_authority::domain::Role;
        let OwnedRoleTransaction::RoutingCapture(capture) = self.retention.owner.as_ref() else {
            return Err(ModelLaneError::IntegrityViolation("routing read requires original capture custody".into()));
        };
        capture.auth.revalidate_in_transaction(transaction, &capture.registry).await?;
        let inputs = self.read_current_inputs_in_transaction(transaction).await?;
        let stage = inputs.stage_ack_key()?;
        let expected_role = match &inputs.execution().stages.get(&capture.request.stage_id)
            .ok_or_else(|| ModelLaneError::IntegrityViolation("captured routing stage is unavailable".into()))?
            .dispatch_target {
            ModelLaneRoutingDispatchTarget::Operator => Role::Operator,
            ModelLaneRoutingDispatchTarget::Validator => Role::Validator,
            _ => return Err(ModelLaneError::AuthorityDenied("routing stage has no role acknowledgement".into())),
        };
        let seal = RoutingAuthorityCaptureReadSeal {
            auth: &capture.auth, transaction, _fence: &self.retention.fence,
        };
        let (receipt, issuance_operation_id) = self.provider.read_routing_prior_in_transaction(
            transaction, &seal, &stage.prior_receipt_ref,
        ).await?;
        if receipt.body.role != expected_role || receipt.body.subject != inputs.subject()? {
            return Err(ModelLaneError::IntegrityViolation("canonical routing prior role or subject differs from stage".into()));
        }
        let prior = RoutingPriorEvidenceReadback { receipt, issuance_operation_id };
        let run = inputs.run().clone();
        validate_routing_capture_basis(&self.retention, &inputs, &run, &prior)?;
        capture.auth.revalidate_in_transaction(transaction, &capture.registry).await?;
        Ok(RoutingAuthorityCapturePayload { inputs, run, prior })
    }

    async fn read_current_inputs_in_transaction<'tx>(
        &'tx self,
        transaction: &'tx super::SurrealTransactionContext<'tx>,
    ) -> crate::swarm_orchestration::model_lane::ModelLaneResult<crate::swarm_orchestration::model_lane::RoutingAuthorityCaptureInputs> {
        let OwnedRoleTransaction::RoutingCapture(capture) = self.retention.owner.as_ref() else {
            return Err(crate::swarm_orchestration::model_lane::ModelLaneError::IntegrityViolation(
                "routing read requires original capture custody".into()));
        };
        self.storage.read_routing_capture_inputs_in_transaction(
            transaction, &capture.auth, &capture.registry, &self.retention.fence,
            &self.store, &self.provider, &capture.request.execution_id, &capture.request.stage_id,
            &capture.request.response_message_id,
        ).await
    }

    pub(super) fn into_observed_read(self, payload: RoutingAuthorityCapturePayload)
        -> crate::swarm_orchestration::model_lane::ModelLaneResult<RoutingAuthorityCaptureRead> {
        validate_routing_capture_basis(&self.retention, &payload.inputs, &payload.run, &payload.prior)?;
        Ok(RoutingAuthorityCaptureRead {
            storage: self.storage, retention: self.retention,
            inputs: payload.inputs, run: payload.run, prior: payload.prior,
        })
    }
}

impl RoutingAuthorityCaptureRead {
    pub(crate) fn operation_id(&self) -> Uuid { self.retention.operation_id() }
    pub(crate) fn run(&self) -> &crate::swarm_orchestration::model_lane::ModelLaneRunRecord { &self.run }
}

impl RoutingAuthorityCaptureFinalization {
    pub(super) fn retention(&self) -> &RoleAuthorityTransactionRetention { &self.retention }
    pub(super) fn belongs_to(&self, storage: &SurrealStorage) -> bool {
        self.storage.resource_access_lifecycle() == storage.resource_access_lifecycle()
    }

    pub(super) fn finalize_into_prepared(self)
        -> crate::swarm_orchestration::model_lane::ModelLaneResult<PreparedRoutingAuthorityConsumption> {
        use crate::swarm_orchestration::model_lane::{ModelLaneError, RoleConsumptionCommand, RoutingStageAcknowledgementIntent};
        validate_routing_capture_basis(&self.retention, &self.inputs, &self.run, &self.prior)?;
        let OwnedRoleTransaction::RoutingCapture(capture) = self.retention.owner.as_ref() else {
            return Err(ModelLaneError::IntegrityViolation("routing finalization owner changed".into()));
        };
        capture.auth.require_unexpired()?;
        let RoleConsumptionCommand::AcknowledgeRoutingStage {
            operation_id, selector, execution_id, stage_id, message_id, acknowledgement,
        } = &self.command else {
            return Err(ModelLaneError::IntegrityViolation("routing finalization requires acknowledgement".into()));
        };
        let prior = &self.prior.receipt.body;
        if *operation_id != capture.operation_id || acknowledgement.claimed_scope() != &capture.auth.exact
            || execution_id != &capture.request.execution_id || stage_id != &capture.request.stage_id
            || message_id != &capture.request.response_message_id
            || acknowledgement.stage() != &self.inputs.stage_ack_key()?
            || selector.evidence_ref != acknowledgement.stage().prior_receipt_ref
            || selector.expectation.receipt_scope != prior.scope || selector.expectation.role != prior.role
            || selector.expectation.run != prior.run || selector.expectation.subject != prior.subject
            || selector.expectation.purpose != prior.purpose || selector.expectation.expected != prior.expected
        {
            return Err(ModelLaneError::IntegrityViolation("routing command differs from original captured basis".into()));
        }
        let prepared = match acknowledgement {
            RoutingStageAcknowledgementIntent::Operator(intent) => {
                use handshake_role_authority::domain::{Purpose, Role};
                if prior.role != Role::Operator || prior.purpose != Purpose::PriorSelection
                    || selector.evidence_ref != intent.approval
                    || intent.stage.prior_receipt_ref != intent.approval
                    || prior.subject != intent.subject || prior.run != intent.run
                    || prior.scope.owner_account_id != intent.scope.owner_account_id
                    || prior.scope.workspace_id != intent.scope.workspace_id
                    || prior.expected.aggregate_id != intent.expected.aggregate_id
                    || prior.expected.base_ref != intent.expected.base_ref
                    || prior.expected.base_vector_ref != intent.expected.base_vector_ref
                    || prior.expected.schema_id != intent.expected.schema_id
                {
                    return Err(ModelLaneError::IntegrityViolation(
                        "Operator stage acknowledgement does not bind its actual prior approval".into()));
                }
                handshake_role_authority::canonical::prepare_operator_ack(intent)
            },
            RoutingStageAcknowledgementIntent::Validator(intent) =>
                handshake_role_authority::canonical::prepare_validator_ack(intent),
        }.map_err(|error| ModelLaneError::IntegrityViolation(error.to_string()))?;
        if prepared.run() != &prior.run || prepared.subject() != &self.inputs.subject()?
            || prepared.expected() != &self.inputs.expected_base() || prepared.retention() != &prior.retention
        {
            return Err(ModelLaneError::IntegrityViolation("routing acknowledgement canonical basis changed".into()));
        }
        capture.command.set(self.command)
            .map_err(|_| ModelLaneError::IntegrityViolation("routing capture command was already initialized".into()))?;
        Ok(PreparedRoutingAuthorityConsumption { storage: self.storage, retention: self.retention, run: self.run })
    }
}

fn validate_routing_capture_basis(
    retention: &RoleAuthorityTransactionRetention,
    inputs: &crate::swarm_orchestration::model_lane::RoutingAuthorityCaptureInputs,
    run: &crate::swarm_orchestration::model_lane::ModelLaneRunRecord,
    prior: &RoutingPriorEvidenceReadback,
) -> crate::swarm_orchestration::model_lane::ModelLaneResult<()> {
    use crate::swarm_orchestration::model_lane::ModelLaneError;
    let OwnedRoleTransaction::RoutingCapture(capture) = retention.owner.as_ref() else {
        return Err(ModelLaneError::IntegrityViolation("routing capture requires original staged owner".into()));
    };
    let stage = inputs.stage_ack_key()?;
    let body = &prior.receipt.body;
    if capture.operation_id.is_nil() || prior.issuance_operation_id.is_nil()
        || stage.execution_id != capture.request.execution_id || stage.stage_id != capture.request.stage_id
        || inputs.response().message_id != capture.request.response_message_id
        || inputs.execution().run_id != run.run_id || inputs.run().run_id != run.run_id
        || run.run_id != body.run.run_id || run.trace_id != body.run.trace_id
        || run.coordinator_session_id != body.run.coordinator_session_id || run.owner_session != body.run.owner_session
        || run.work_packet_id.as_deref() != Some(body.run.work_packet_id.as_str())
        || run.micro_task_id.as_deref() != Some(body.run.micro_task_id.as_str())
        || run.task_board_id.as_deref() != Some(body.run.task_board_id.as_str())
        || body.scope.owner_account_id != capture.auth.exact.owner_account_id
        || body.scope.workspace_id != capture.auth.exact.workspace_id
        || stage.prior_receipt_ref.as_str().strip_prefix("eventledger://role-authority/")
            != Some(prior.receipt.event.event_id.as_str())
    {
        return Err(ModelLaneError::IntegrityViolation("routing read differs from immutable capture request or prior issuance".into()));
    }
    let encoded = |value: &crate::swarm_orchestration::model_lane::ModelLaneRunRecord|
        serde_json::to_vec(value).map_err(|error| ModelLaneError::IntegrityViolation(error.to_string()));
    if encoded(inputs.run())? != encoded(run)? {
        return Err(ModelLaneError::IntegrityViolation("routing canonical run snapshot changed".into()));
    }
    Ok(())
}

impl RoleAuthorityPublicationCompletion {
    pub(crate) fn operation_id(&self) -> Uuid { self.operation_id }
}

pub(crate) struct RolePublicationGuard<'a> {
    owner: &'a OwnedRoleTransaction,
    _fence: &'a tokio::sync::OwnedRwLockReadGuard<()>,
}

impl RolePublicationGuard<'_> {
    pub(crate) fn operation_id(&self) -> Uuid { self.owner.operation_id() }

    pub(crate) fn current_scope(&self) -> &crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution {
        &self.owner.auth().exact
    }

    pub(crate) fn require_unexpired(&self) -> Result<(), ResourceAuthorityError> {
        self.owner.auth().require_unexpired()
    }
}

pub(crate) struct OwnerIssuanceSeal<'tx> {
    permit: &'tx WorkspaceOwnerActionPermit,
    transaction: &'tx super::SurrealTransactionContext<'tx>,
}

impl OwnerIssuanceSeal<'_> {
    pub(crate) fn operation_id(&self) -> Uuid { self.permit.command.operation_id() }
    pub(crate) fn current_scope(&self) -> &crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution {
        &self.permit.auth.exact
    }
    pub(crate) fn command(&self) -> &crate::swarm_orchestration::model_lane::WorkspaceOwnerActionCommand {
        &self.permit.command
    }
    pub(crate) fn matches_transaction(&self, transaction: &super::SurrealTransactionContext<'_>) -> bool {
        std::ptr::eq(self.transaction, transaction)
    }
}

pub(crate) struct RoutingAuthorityCaptureReadSeal<'tx> {
    auth: &'tx OwnedCanonicalAuthSnapshot,
    transaction: &'tx super::SurrealTransactionContext<'tx>,
    _fence: &'tx tokio::sync::OwnedRwLockReadGuard<()>,
}

impl RoutingAuthorityCaptureReadSeal<'_> {
    pub(crate) fn current_scope(&self) -> &crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution {
        &self.auth.exact
    }

    pub(crate) fn matches_transaction(&self, transaction: &super::SurrealTransactionContext<'_>) -> bool {
        std::ptr::eq(self.transaction, transaction)
    }
}

pub(crate) struct CurrentConsumerSeal<'tx> {
    owner: &'tx OwnedRoleTransaction,
    transaction: &'tx super::SurrealTransactionContext<'tx>,
}

impl CurrentConsumerSeal<'_> {
    pub(crate) fn operation_id(&self) -> Uuid { self.owner.operation_id() }
    pub(crate) fn current_scope(&self) -> &crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution {
        &self.owner.auth().exact
    }
    pub(crate) fn command(&self) -> &crate::swarm_orchestration::model_lane::RoleConsumptionCommand {
        self.owner.consumer_command().expect("consumer seals require an initialized command")
    }
    pub(crate) fn matches_transaction(&self, transaction: &super::SurrealTransactionContext<'_>) -> bool {
        std::ptr::eq(self.transaction, transaction)
    }
}

impl SurrealStorage {
    async fn execute_operator_role_consumption(
        &self,
        permit: RoleAuthorityConsumptionPermit,
        store: crate::swarm_orchestration::model_lane::ModelLaneStore,
    ) -> crate::swarm_orchestration::model_lane::ModelLaneResult<crate::swarm_orchestration::model_lane::RoleConsumptionResult> {
        use crate::swarm_orchestration::model_lane::{ModelLaneError, RoleConsumptionCommand,
            RoleConsumptionResult, RoutingStageAcknowledgementIntent};
        let is_operator_operation = match permit.capture.command() {
            RoleConsumptionCommand::ReviewInvocation { selector, .. } =>
                selector.expectation.role == handshake_role_authority::domain::Role::Operator,
            RoleConsumptionCommand::AcknowledgeRoutingStage { acknowledgement, .. } =>
                matches!(acknowledgement, RoutingStageAcknowledgementIntent::Operator(_)),
            _ => false,
        };
        if !is_operator_operation || store.access().exact_read_scope() != Some(permit.current_scope())
            || store.surreal_storage().resource_access_lifecycle() != self.resource_access_lifecycle()
        {
            return Err(ModelLaneError::AuthorityDenied("Operator consumption differs from its captured command or configured owner".into()));
        }
        let retention = permit.into_transaction_retention(self).await?;
        let operation_id = retention.operation_id();
        let scope = retention.owner.auth().record_user_scope();
        let provider = super::model_lane::SurrealModelLaneStore::initialize(self.clone()).await?;
        let worker_retention = retention.clone();
        let worker_provider = provider.clone();
        let worker_store = store.clone();
        let committed = self.with_record_user_scope(scope,
            self.with_owned_role_authenticated_transaction(
                operation_id, super::ContextOperationCancellation::new(), retention.clone(),
                move |transaction| Box::pin(async move {
                    let owner = worker_retention.owner.as_ref();
                    let registry = owner.consumer_registry().ok_or_else(||
                        ModelLaneError::IntegrityViolation("Operator consumer registry is unavailable".into()))?;
                    owner.auth().revalidate_in_transaction(&transaction, registry).await?;
                    let seal = CurrentConsumerSeal { owner, transaction: &transaction };
                    let command = owner.consumer_command().ok_or_else(||
                        ModelLaneError::IntegrityViolation("Operator consumer command is unavailable".into()))?;
                    let readback = match command {
                        RoleConsumptionCommand::ReviewInvocation { selector, .. } => {
                            let checked = worker_provider.resolve_operator_role_evidence(
                                &transaction, &seal, selector,
                            ).await?;
                            OperatorConsumptionReadback::Review(checked.into_readback())
                        },
                        RoleConsumptionCommand::AcknowledgeRoutingStage {
                            selector, acknowledgement: RoutingStageAcknowledgementIntent::Operator(intent), ..
                        } => {
                            let prior = worker_provider.resolve_operator_role_evidence(
                                &transaction, &seal, selector,
                            ).await?;
                            let acknowledgement = worker_provider.put_operator_stage_ack(
                                &transaction, &seal, &prior, intent,
                            ).await?;
                            let routing = worker_store.complete_routing_authority_in_transaction(
                                &transaction, &seal, &prior, &acknowledgement,
                            ).await?;
                            OperatorConsumptionReadback::Routing {
                                prior: prior.into_readback(), acknowledgement: acknowledgement.into_readback(), routing,
                            }
                        },
                        _ => return Err(ModelLaneError::IntegrityViolation("Operator consumer command changed after capture".into())),
                    };
                    owner.auth().revalidate_in_transaction(&transaction, registry).await?;
                    Ok::<_, ModelLaneError>(readback)
                }),
            ),
        ).await?;
        let registry = retention.owner.consumer_registry().ok_or_else(||
            ModelLaneError::IntegrityViolation("Operator publication registry is unavailable".into()))?;
        retention.owner.auth().revalidate_for_publication(self, registry).await?;
        let guard = RolePublicationGuard { owner: &retention.owner, _fence: &retention.fence };
        let result = self.with_record_user_scope(retention.owner.auth().record_user_scope(), async {
            match &committed {
                OperatorConsumptionReadback::Review(expected) => {
                    let published = provider.readback_role_evidence(&guard, expected).await?;
                    Ok::<_, ModelLaneError>(RoleConsumptionResult::ReviewInvocation(published.receipt().clone()))
                },
                OperatorConsumptionReadback::Routing { prior, acknowledgement, routing } => {
                    provider.readback_role_evidence(&guard, prior).await?;
                    provider.readback_role_evidence(&guard, acknowledgement).await?;
                    let execution = store.readback_routing_authority_commit(&guard, routing).await?;
                    Ok(RoleConsumptionResult::RoutingStageAcknowledgement(execution))
                },
            }
        }).await?;
        retention.owner.auth().revalidate_for_publication(self, registry).await?;
        guard.require_unexpired()?;
        self.complete_role_publication(RoleAuthorityPublicationCompletion {
            operation_id, owner: retention.owner.clone(),
        })?;
        Ok(result)
    }

    pub(crate) async fn finalize_operator_routing_authority_capture(
        &self,
        read: RoutingAuthorityCaptureRead,
    ) -> crate::swarm_orchestration::model_lane::ModelLaneResult<RoutingAuthorityCaptureFinalization> {
        use crate::swarm_orchestration::model_lane::{ModelLaneError, RoleConsumptionCommand,
            RoleEvidenceConsumeSelector, RoutingStageAcknowledgementIntent};
        use handshake_role_authority::domain::{ActionKey, AcknowledgeOperatorStage, Purpose, ResolutionExpectation, Role};
        validate_routing_capture_basis(&read.retention, &read.inputs, &read.run, &read.prior)?;
        let OwnedRoleTransaction::RoutingCapture(capture) = read.retention.owner.as_ref() else {
            return Err(ModelLaneError::IntegrityViolation("Operator finalization requires original capture custody".into()));
        };
        let prior = &read.prior.receipt.body;
        if self.resource_access_lifecycle() != read.storage.resource_access_lifecycle()
            || prior.role != Role::Operator || prior.purpose != Purpose::PriorSelection
            || read.inputs.response().authority != crate::swarm_orchestration::model_lane::ModelLaneAuthority::OperatorDecision
        {
            return Err(ModelLaneError::AuthorityDenied("Operator finalization requires its canonical prior approval".into()));
        }
        capture.auth.revalidate_for_publication(self, &capture.registry).await?;
        let stage = read.inputs.stage_ack_key()?;
        let selector = RoleEvidenceConsumeSelector {
            evidence_ref: stage.prior_receipt_ref.clone(),
            expectation: ResolutionExpectation {
                receipt_scope: prior.scope.clone(), role: prior.role.clone(), run: prior.run.clone(),
                subject: prior.subject.clone(), purpose: prior.purpose.clone(), expected: prior.expected.clone(),
            }, designation_ref: None, producer: None,
        };
        let operation_key = capture.operation_id.to_string();
        let acknowledgement = AcknowledgeOperatorStage {
            approval: stage.prior_receipt_ref.clone(),
            action: ActionKey {
                action_id: operation_key.clone(), request_id: read.inputs.response().message_id.clone(),
                idempotency_key: operation_key,
            },
            scope: capture.auth.exact.clone(), run: prior.run.clone(), subject: read.inputs.subject()?,
            stage, expected: read.inputs.expected_base(), retention: prior.retention.clone(),
        };
        handshake_role_authority::canonical::prepare_operator_ack(&acknowledgement)
            .map_err(|error| ModelLaneError::IntegrityViolation(error.to_string()))?;
        let command = RoleConsumptionCommand::AcknowledgeRoutingStage {
            operation_id: capture.operation_id, selector,
            execution_id: capture.request.execution_id.clone(), stage_id: capture.request.stage_id.clone(),
            message_id: capture.request.response_message_id.clone(),
            acknowledgement: RoutingStageAcknowledgementIntent::Operator(acknowledgement),
        };
        capture.auth.require_unexpired()?;
        Ok(RoutingAuthorityCaptureFinalization {
            storage: read.storage, retention: read.retention, inputs: read.inputs,
            run: read.run, prior: read.prior, command,
        })
    }

    async fn read_routing_capture_inputs_in_transaction<'tx>(
        &self,
        transaction: &'tx super::SurrealTransactionContext<'tx>,
        auth: &'tx OwnedCanonicalAuthSnapshot,
        registry: &crate::capabilities::CapabilityRegistry,
        fence: &'tx tokio::sync::OwnedRwLockReadGuard<()>,
        store: &crate::swarm_orchestration::model_lane::ModelLaneStore,
        provider: &super::model_lane::SurrealModelLaneStore,
        execution_id: &str,
        stage_id: &str,
        response_message_id: &str,
    ) -> crate::swarm_orchestration::model_lane::ModelLaneResult<crate::swarm_orchestration::model_lane::RoutingAuthorityCaptureInputs> {
        use crate::swarm_orchestration::model_lane::ModelLaneError;
        if store.access().exact_read_scope() != Some(&auth.exact)
            || store.surreal_storage().resource_access_lifecycle() != self.resource_access_lifecycle()
        {
            return Err(ModelLaneError::AuthorityDenied("routing capture store differs from authenticated owner".into()));
        }
        auth.revalidate_in_transaction(transaction, registry).await?;
        let seal = RoutingAuthorityCaptureReadSeal { auth, transaction, _fence: fence };
        let inputs = store.read_routing_authority_capture_inputs_in_transaction(
            provider, transaction, &seal, execution_id, stage_id, response_message_id,
        ).await?;
        auth.revalidate_in_transaction(transaction, registry).await?;
        Ok(inputs)
    }

    pub(crate) async fn capture_routing_authority_transaction(
        &self,
        bootstrap: &crate::AccountBootstrapState,
        credentials: RoleAuthorityAuthInput,
        store: crate::swarm_orchestration::model_lane::ModelLaneStore,
        execution_id: &str,
        stage_id: &str,
        response_message_id: &str,
    ) -> crate::swarm_orchestration::model_lane::ModelLaneResult<RoutingAuthorityCaptureTransaction> {
        use crate::swarm_orchestration::model_lane::ModelLaneError;
        let workspace_id = store.access().exact_read_scope()
            .ok_or_else(|| ModelLaneError::AuthorityDenied("routing capture requires configured current scope".into()))?
            .workspace_id.as_str().to_owned();
        let request = AuthorizationRequest {
            session_token: credentials.session_token.clone(),
            channel_binding_hash: Some(credentials.channel_binding_hash.clone()),
            capability_id: "fs.write".into(), resource_kind: ResourceKind::Workspace,
            external_resource_id: workspace_id, action: ResourceAction::Create,
        };
        self.capture_routing_authority_transaction_with_request(
            bootstrap, credentials, request, store, execution_id.to_owned(), stage_id.to_owned(),
            response_message_id.to_owned(),
        ).await
    }

    async fn capture_routing_authority_transaction_with_request(
        &self,
        bootstrap: &crate::AccountBootstrapState,
        credentials: RoleAuthorityAuthInput,
        request: AuthorizationRequest,
        store: crate::swarm_orchestration::model_lane::ModelLaneStore,
        execution_id: String,
        stage_id: String,
        response_message_id: String,
    ) -> crate::swarm_orchestration::model_lane::ModelLaneResult<RoutingAuthorityCaptureTransaction> {
        use crate::swarm_orchestration::model_lane::ModelLaneError;
        if execution_id.is_empty() || stage_id.is_empty() || response_message_id.is_empty()
            || bootstrap.surreal.resource_access_lifecycle() != self.resource_access_lifecycle()
        {
            return Err(ModelLaneError::AuthorityDenied("routing capture identity or bootstrap differs".into()));
        }
        let auth = capture_owned_canonical_role_auth(
            self, &bootstrap.capability_registry, credentials, request,
        ).await?;
        if store.access().exact_read_scope() != Some(&auth.exact)
            || store.surreal_storage().resource_access_lifecycle() != self.resource_access_lifecycle()
        {
            return Err(ModelLaneError::AuthorityDenied("routing capture differs from current configured authority".into()));
        }
        let fence = std::sync::Arc::new(self.authority_publication_read().await?);
        auth.revalidate_for_publication(self, &bootstrap.capability_registry).await?;
        let provider = super::model_lane::SurrealModelLaneStore::initialize(self.clone()).await?;
        let retention = RoleAuthorityTransactionRetention {
            owner: std::sync::Arc::new(OwnedRoleTransaction::RoutingCapture(OwnedRoutingCapture {
                operation_id: Uuid::now_v7(), auth, registry: bootstrap.capability_registry.clone(),
                request: RoutingCaptureRequest { execution_id, stage_id, response_message_id },
                command: std::sync::OnceLock::new(),
            })), fence,
        };
        Ok(RoutingAuthorityCaptureTransaction { storage: self.clone(), retention, store, provider })
    }

    async fn execute_operator_approval(
        &self,
        permit: WorkspaceOwnerActionPermit,
    ) -> Result<crate::swarm_orchestration::model_lane::WorkspaceOwnerActionResult, ResourceAuthorityError> {
        let scope = permit.auth.record_user_scope();
        let retention = RoleAuthorityTransactionRetention {
            owner: std::sync::Arc::new(OwnedRoleTransaction::Owner(permit)),
            fence: std::sync::Arc::new(self.authority_publication_read().await?),
        };
        let operation_id = retention.operation_id();
        let provider = super::model_lane::SurrealModelLaneStore::initialize(self.clone()).await?;
        let worker_storage = self.clone();
        let worker_retention = retention.clone();
        let worker_provider = provider.clone();
        let readback = self.with_record_user_scope(scope,
            self.with_owned_role_authenticated_transaction(
                operation_id, super::ContextOperationCancellation::new(), retention.clone(),
                move |transaction| Box::pin(async move {
                    let OwnedRoleTransaction::Owner(permit) = worker_retention.owner.as_ref() else {
                        return Err(ResourceAuthorityError::InvalidInput("operator transaction requires Owner issuance"));
                    };
                    worker_storage.operator_approval_in_transaction(
                        &transaction, permit, &worker_provider,
                    ).await
                }),
            ),
        ).await?;
        let (receipt, completion) = self.publish_operator_approval(&retention, &provider, &readback).await?;
        self.complete_role_publication(completion)?;
        Ok(crate::swarm_orchestration::model_lane::WorkspaceOwnerActionResult::ArtifactApproval(receipt))
    }

    async fn operator_approval_in_transaction<'tx>(
        &self,
        transaction: &'tx super::SurrealTransactionContext<'tx>,
        permit: &'tx WorkspaceOwnerActionPermit,
        provider: &super::model_lane::SurrealModelLaneStore,
    ) -> Result<super::model_lane::RoleEvidenceReadback, ResourceAuthorityError> {
        permit.auth.revalidate_in_transaction(transaction, &permit.registry).await?;
        let seal = OwnerIssuanceSeal { permit, transaction };
        let pending = provider.put_operator_approval(transaction, &seal).await?;
        permit.auth.revalidate_in_transaction(transaction, &permit.registry).await?;
        Ok(pending.into_readback())
    }

    async fn publish_operator_approval(
        &self,
        retention: &RoleAuthorityTransactionRetention,
        provider: &super::model_lane::SurrealModelLaneStore,
        readback: &super::model_lane::RoleEvidenceReadback,
    ) -> Result<(handshake_role_authority::domain::RoleReceipt, RoleAuthorityPublicationCompletion), ResourceAuthorityError> {
        let OwnedRoleTransaction::Owner(permit) = retention.owner.as_ref() else {
            return Err(ResourceAuthorityError::InvalidInput("operator publication requires Owner issuance"));
        };
        let crate::swarm_orchestration::model_lane::WorkspaceOwnerActionCommand::IssueArtifactApproval(command) = &permit.command else {
            return Err(ResourceAuthorityError::InvalidInput("operator publication command mismatch"));
        };
        permit.auth.revalidate_for_publication(self, &permit.registry).await?;
        let guard = RolePublicationGuard { owner: &retention.owner, _fence: &retention.fence };
        let published = self.with_record_user_scope(
            permit.auth.record_user_scope(), provider.readback_role_evidence(&guard, readback),
        ).await?;
        let prepared = handshake_role_authority::canonical::prepare_operator(&command.approval)
            .map_err(|_| ResourceAuthorityError::InvalidInput("invalid bound operator approval"))?;
        handshake_role_authority::resolution::compare_retry(published.receipt(), &prepared)
            .map_err(|_| ResourceAuthorityError::InvalidInput("published operator command mismatch"))?;
        if published.operation_id() != retention.operation_id() {
            return Err(ResourceAuthorityError::InvalidInput("published operator operation mismatch"));
        }
        permit.auth.revalidate_for_publication(self, &permit.registry).await?;
        guard.require_unexpired()?;
        Ok((published.receipt().clone(), RoleAuthorityPublicationCompletion {
            operation_id: retention.operation_id(), owner: retention.owner.clone(),
        }))
    }

    pub(crate) async fn capture_workspace_owner_action(
        &self,
        bootstrap: &crate::AccountBootstrapState,
        credentials: RoleAuthorityAuthInput,
        request: AuthorizationRequest,
        command: crate::swarm_orchestration::model_lane::WorkspaceOwnerActionCommand,
    ) -> Result<WorkspaceOwnerActionPermit, ResourceAuthorityError> {
        if command.operation_id().is_nil() || request.resource_kind != ResourceKind::Workspace
            || request.capability_id != "fs.write"
            || request.action != ResourceAction::Create
        {
            return Err(ResourceAuthorityError::InvalidInput("invalid workspace Owner issuance request"));
        }
        let auth = capture_owned_canonical_role_auth(
            self, &bootstrap.capability_registry, credentials, request,
        ).await?;
        let locator = self.canonical_resource_locator(ResourceKind::Workspace, &auth.workspace_id)
            .await?.ok_or_else(|| ResourceAuthorityError::Denied {
                decision_id: auth.decision.decision_id.clone(),
            })?;
        if !auth.matches_workspace_locator(&locator) {
            return Err(ResourceAuthorityError::Denied { decision_id: auth.decision.decision_id.clone() });
        }
        if command.claimed_scope() != &auth.exact {
            return Err(ResourceAuthorityError::Denied { decision_id: auth.decision.decision_id.clone() });
        }
        if let crate::swarm_orchestration::model_lane::WorkspaceOwnerActionCommand::IssueArtifactApproval(input) = &command {
            handshake_role_authority::canonical::prepare_operator(&input.approval)
                .map_err(|_| ResourceAuthorityError::InvalidInput("invalid artifact approval command"))?;
        }
        Ok(WorkspaceOwnerActionPermit {
            auth, registry: bootstrap.capability_registry.clone(), command,
        })
    }

    pub(crate) async fn capture_role_authority_consumption(
        &self,
        bootstrap: &crate::AccountBootstrapState,
        credentials: RoleAuthorityAuthInput,
        request: AuthorizationRequest,
        command: crate::swarm_orchestration::model_lane::RoleConsumptionCommand,
    ) -> Result<RoleAuthorityConsumptionPermit, ResourceAuthorityError> {
        if command.operation_id().is_nil() {
            return Err(ResourceAuthorityError::InvalidInput("nil role consumption operation"));
        }
        let auth = capture_owned_canonical_role_auth(
            self, &bootstrap.capability_registry, credentials, request,
        ).await?;
        for selector in command.evidence_selectors() {
            let issuer = &selector.expectation.receipt_scope;
            if issuer.owner_account_id != auth.exact.owner_account_id
                || issuer.workspace_id != auth.exact.workspace_id
            {
                return Err(ResourceAuthorityError::Denied { decision_id: auth.decision.decision_id.clone() });
            }
        }
        if let crate::swarm_orchestration::model_lane::RoleConsumptionCommand::RecordReviewCompletion { verdict, .. } = &command {
            if verdict.scope != auth.exact {
                return Err(ResourceAuthorityError::Denied { decision_id: auth.decision.decision_id.clone() });
            }
        }
        if let crate::swarm_orchestration::model_lane::RoleConsumptionCommand::AcknowledgeRoutingStage { acknowledgement, .. } = &command {
            if acknowledgement.claimed_scope() != &auth.exact {
                return Err(ResourceAuthorityError::Denied { decision_id: auth.decision.decision_id.clone() });
            }
        }
        Ok(RoleAuthorityConsumptionPermit {
            capture: RoleConsumptionCapture::Consumer(std::sync::Arc::new(OwnedRoleConsumption {
                auth, registry: bootstrap.capability_registry.clone(), command,
            })), preparation: None,
        })
    }
}

impl RoleAuthorityAuthInput {
    pub(crate) fn new(session_token: String, channel_binding_hash: String) -> Self {
        Self { session_token, channel_binding_hash }
    }
}

// Authentication facts alone do not establish Workspace Owner eligibility.
struct OwnedCanonicalAuthSnapshot {
    credentials: RoleAuthorityAuthInput,
    session: super::local_accounts::LocalSessionContext,
    decision: AuthorizationDecision,
    authority: RegistryPublicationAuthority,
    workspace_id: String,
    capability_id: String,
    action: ResourceAction,
    exact: crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution,
}

impl OwnedCanonicalAuthSnapshot {
    async fn revalidate_for_publication(
        &self,
        storage: &SurrealStorage,
        registry: &crate::capabilities::CapabilityRegistry,
    ) -> Result<(), ResourceAuthorityError> {
        let denied = || ResourceAuthorityError::Denied { decision_id: self.decision.decision_id.clone() };
        if !registry.profile_can(&self.authority.capability_profile_id, &self.capability_id)
            .unwrap_or(false)
        {
            return Err(denied());
        }
        let bindings = RegistryPublicationBindings {
            resource: self.authority.resource_id.clone(),
            original_grant: self.authority.grant_id.clone(),
            kind: self.authority.resource_kind.clone(),
            action: self.action.as_str().to_owned(),
            capability: self.capability_id.clone(),
        };
        let rows: Vec<RegistryPublicationAuthority> = storage.with_record_user_scope(
            self.record_user_scope(), storage.with_data_operation(move |database| {
                Box::pin(async move { database.query_values(REGISTRY_PUBLICATION_AUTHORITY, bindings).await })
            }),
        ).await?;
        if rows.as_slice() != std::slice::from_ref(&self.authority) {
            return Err(denied());
        }
        self.require_unexpired()
    }

    fn matches_workspace_locator(&self, locator: &CanonicalResourceLocator) -> bool {
        self.authority.resource_kind == "workspace"
            && locator.id == self.authority.resource_id
            && locator.external_resource_id == self.workspace_id
            && locator.owner_account_id == self.authority.account_id
            && locator.access_space_id == self.authority.access_space_id
    }

    fn record_user_scope(&self) -> RecordUserScope {
        RecordUserScope {
            grant_id: Some(self.decision.grant_id.clone()),
            workspace_id: Some(self.workspace_id.clone()),
            session_token: self.credentials.session_token.clone(),
            channel_binding_hash: Some(self.credentials.channel_binding_hash.clone()),
            resource_id: self.decision.resource_id.clone(),
            session_id: self.decision.session_id.clone(),
            capability_id: self.capability_id.clone(),
            action: self.action,
        }
    }

    async fn revalidate_in_transaction(
        &self,
        transaction: &super::SurrealTransactionContext<'_>,
        registry: &crate::capabilities::CapabilityRegistry,
    ) -> Result<(), ResourceAuthorityError> {
        let denied = || ResourceAuthorityError::Denied {
            decision_id: self.decision.decision_id.clone(),
        };
        let current = super::current_record_user_scope().ok_or_else(denied)?;
        let original = self.record_user_scope();
        if current.session_token != original.session_token
            || current.channel_binding_hash != original.channel_binding_hash
            || current.session_id != original.session_id
            || current.workspace_id != original.workspace_id
            || current.resource_id != original.resource_id
            || current.grant_id != original.grant_id
            || current.capability_id != original.capability_id
            || current.action != original.action
            || !registry.profile_can(&self.authority.capability_profile_id, &self.capability_id)
                .unwrap_or(false)
        {
            return Err(denied());
        }
        let rows: Vec<RegistryPublicationAuthority> = transaction.query_values_at(
            REGISTRY_PUBLICATION_AUTHORITY,
            RegistryPublicationBindings {
                resource: self.authority.resource_id.clone(),
                original_grant: self.authority.grant_id.clone(),
                kind: self.authority.resource_kind.clone(),
                action: self.action.as_str().to_owned(),
                capability: self.capability_id.clone(),
            },
            0,
        ).await?;
        if rows.as_slice() != std::slice::from_ref(&self.authority) {
            return Err(denied());
        }
        // The external locator is captured by the privileged lookup. Record
        // users cannot select it; the original record and grant bind this check.
        if self.authority.resource_kind != "workspace" {
            return Err(denied());
        }
        self.require_unexpired()
    }

    fn require_unexpired(&self) -> Result<(), ResourceAuthorityError> {
        let now = Utc::now();
        if self.authority.session_expires_at.clone().into_inner() <= now
            || self.authority.grant_expires_at.as_ref()
                .is_some_and(|expiry| expiry.clone().into_inner() <= now)
        {
            return Err(ResourceAuthorityError::Denied {
                decision_id: self.decision.decision_id.clone(),
            });
        }
        Ok(())
    }
}

async fn capture_owned_canonical_role_auth(
    storage: &SurrealStorage,
    registry: &crate::capabilities::CapabilityRegistry,
    credentials: RoleAuthorityAuthInput,
    request: AuthorizationRequest,
) -> Result<OwnedCanonicalAuthSnapshot, ResourceAuthorityError> {
    if request.resource_kind != ResourceKind::Workspace {
        return Err(ResourceAuthorityError::InvalidInput("role authority requires workspace resource"));
    }
    let session = storage.authenticate_local_session(
        &credentials.session_token, &credentials.channel_binding_hash,
    ).await?;
    if request.session_token != credentials.session_token
        || request.channel_binding_hash.as_deref() != Some(credentials.channel_binding_hash.as_str())
    {
        return Err(ResourceAuthorityError::InvalidInput("role request credential mismatch"));
    }
    let kind = request.resource_kind.as_str().to_owned();
    let capability = request.capability_id.clone();
    let action = request.action;
    let workspace_id = request.external_resource_id.clone();
    let decision = storage.authorize_protected_resource(request).await?;
    let denied = || ResourceAuthorityError::Denied { decision_id: decision.decision_id.clone() };
    if session.session_id != decision.session_id
        || session.identity.account_id != decision.account_id
        || session.identity.principal_id != decision.principal_id
        || session.identity.access_space_id != decision.access_space_id
        || session.delegation_chain != decision.delegation_chain
        || !registry.profile_can(&decision.capability_profile_id, &capability)
            .unwrap_or(false)
    {
        return Err(denied());
    }
    let scope = RecordUserScope {
        grant_id: Some(decision.grant_id.clone()),
        workspace_id: Some(workspace_id.clone()),
        session_token: credentials.session_token.clone(),
        channel_binding_hash: Some(credentials.channel_binding_hash.clone()),
        resource_id: decision.resource_id.clone(),
        session_id: decision.session_id.clone(),
        capability_id: capability.clone(),
        action,
    };
    let bindings = RegistryPublicationBindings {
        resource: RecordId::new("protected_resources", decision.resource_id.clone()),
        original_grant: RecordId::new("resource_grants", decision.grant_id.clone()),
        kind,
        action: action.as_str().to_owned(),
        capability: capability.clone(),
    };
    let rows: Vec<RegistryPublicationAuthority> = storage.with_record_user_scope(
        scope, storage.with_data_operation(move |database| {
            Box::pin(async move { database.query_values(REGISTRY_PUBLICATION_AUTHORITY, bindings).await })
        }),
    ).await?;
    let [authority] = rows.as_slice() else { return Err(denied()); };
    if authority.account_id != RecordId::new("local_accounts", decision.account_id.clone())
        || authority.principal_id != RecordId::new("principals", decision.principal_id.clone())
        || authority.session_id != RecordId::new("authenticated_sessions", decision.session_id.clone())
        || authority.access_space_id != RecordId::new("access_spaces", decision.access_space_id.clone())
        || authority.resource_id != RecordId::new("protected_resources", decision.resource_id.clone())
        || authority.grant_id != RecordId::new("resource_grants", decision.grant_id.clone())
        || authority.capability_profile_id != decision.capability_profile_id
        || authority.delegation_chain != decision.delegation_chain
    {
        return Err(denied());
    }
    use crate::swarm_orchestration::resource_scope::{
        AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, ExactResourceScopeAttribution,
        OwnerAccountId, WorkspaceScopeRef,
    };
    let parse = |value: &str| Uuid::parse_str(value)
        .map_err(|_| ResourceAuthorityError::InvalidInput("invalid canonical role scope UUID"));
    let exact = ExactResourceScopeAttribution {
        owner_account_id: OwnerAccountId::from_uuid(parse(&decision.account_id)?),
        actor_principal_id: ActorPrincipalId::from_uuid(parse(&decision.principal_id)?),
        authenticated_session_id: AuthenticatedSessionRef::from_uuid(parse(&decision.session_id)?),
        access_space_id: AccessSpaceRef::from_uuid(parse(&decision.access_space_id)?),
        workspace_id: WorkspaceScopeRef::new(workspace_id.clone())
            .map_err(|_| ResourceAuthorityError::InvalidInput("invalid canonical role workspace"))?,
    };
    let snapshot = OwnedCanonicalAuthSnapshot {
        credentials,
        session,
        authority: authority.clone(),
        decision,
        workspace_id,
        capability_id: capability,
        action,
        exact,
    };
    snapshot.require_unexpired()?;
    Ok(snapshot)
}

pub const AUTHORITY_ACCESS_METHOD: &str = "authenticated_session";
pub const RECONCILIATION_QUEUE_ID: &str = "mt109-protected-reconciliation";
pub const RECONCILIATION_PROFILE_ID: &str = "MT109Reconciler";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceAction {
    Create,
    Read,
    Update,
    Delete,
    Reconcile,
}

impl ResourceAction {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Read => "read",
            Self::Update => "update",
            Self::Delete => "delete",
            Self::Reconcile => "reconcile",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    Workspace,
    RichDocument,
    KnowledgeSource,
    KnowledgeCodeFile,
    LoomBlock,
    FlightRecorder,
    MemoryPack,
    MemoryProposal,
    MemoryCommitReport,
    MemoryItem,
    MemoryItemCount,
    ReconciliationQueue,
}

impl ResourceKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Workspace => "workspace",
            Self::RichDocument => "rich_document",
            Self::KnowledgeSource => "knowledge_source",
            Self::KnowledgeCodeFile => "knowledge_code_file",
            Self::LoomBlock => "loom_block",
            Self::FlightRecorder => "flight_recorder",
            Self::MemoryPack => "memory_pack",
            Self::MemoryProposal => "memory_proposal",
            Self::MemoryCommitReport => "memory_commit_report",
            Self::MemoryItem => "memory_item",
            Self::MemoryItemCount => "memory_item_count",
            Self::ReconciliationQueue => "reconciliation_queue",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LocalAccount {
    pub account_id: String,
    pub account_role: String,
    pub enabled: bool,
    pub revocation_epoch: i64,
    pub policy_version: i64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Principal {
    pub principal_id: String,
    pub account_id: String,
    pub principal_kind: String,
    pub actor_kind: String,
    pub actor_id: String,
    pub capability_profile_id: String,
    pub delegated_capabilities: Vec<String>,
    pub enabled: bool,
    pub revocation_epoch: i64,
    pub policy_version: i64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AccessSpace {
    pub access_space_id: String,
    pub account_id: String,
    pub name: String,
    pub enabled: bool,
    pub revocation_epoch: i64,
    pub policy_version: i64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AuthenticatedSession {
    pub session_id: String,
    pub account_id: String,
    pub principal_id: String,
    pub access_space_id: String,
    pub delegated_capabilities: Vec<String>,
    pub delegation_chain: Vec<String>,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub policy_version: i64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProtectedResource {
    pub resource_id: String,
    pub resource_kind: ResourceKind,
    pub external_resource_id: String,
    pub owner_account_id: String,
    pub created_by_principal_id: String,
    pub access_space_id: String,
    pub policy_version: i64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ResourceGrant {
    pub grant_id: String,
    pub account_id: String,
    pub principal_id: String,
    pub access_space_id: String,
    pub resource_id: String,
    pub actions: Vec<ResourceAction>,
    pub capability_ids: Vec<String>,
    pub delegation_chain: Vec<String>,
    pub policy_version: i64,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthorizationRequest {
    pub session_token: String,
    pub channel_binding_hash: Option<String>,
    pub capability_id: String,
    pub resource_kind: ResourceKind,
    pub external_resource_id: String,
    pub action: ResourceAction,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AuthorizationDecision {
    pub decision_id: String,
    pub account_id: String,
    pub principal_id: String,
    pub session_id: String,
    pub access_space_id: String,
    pub resource_id: String,
    pub actor_kind: String,
    pub actor_id: String,
    pub capability_profile_id: String,
    pub delegation_chain: Vec<String>,
    pub policy_version: i64,
    pub grant_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IssuedSession {
    pub token: String,
    pub account_id: String,
    pub principal_id: String,
    pub session_id: String,
    pub access_space_id: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IssuedSessionCredential {
    pub credential_id: String,
    pub token: String,
    pub account_id: String,
    pub principal_id: String,
    pub access_space_id: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProvisionedIdentity {
    pub account_id: String,
    pub principal_id: String,
    pub access_space_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProvisionedPrincipal {
    pub identity: ProvisionedIdentity,
    pub session: IssuedSession,
}

pub type ReconciliationPrincipal = ProvisionedPrincipal;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RegisteredResource {
    pub resource_id: String,
    pub resource_kind: ResourceKind,
    pub external_resource_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceGrantSpec {
    pub principal_id: String,
    pub resource_id: String,
    pub actions: Vec<ResourceAction>,
    pub capability_ids: Vec<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub delegation_chain: Vec<String>,
}

#[derive(Debug, Error)]
pub enum ResourceAuthorityError {
    #[error("resource authority storage failed: {0}")]
    Storage(#[from] SurrealStorageError),
    #[error("resource authority input is invalid: {0}")]
    InvalidInput(&'static str),
    #[error("authorization denied")]
    Denied { decision_id: String },
    #[error("resource authority entropy failed: {0}")]
    Entropy(String),
}

impl From<ResourceAuthorityError> for SurrealStorageError {
    fn from(error: ResourceAuthorityError) -> Self {
        match error {
            ResourceAuthorityError::Storage(error) => error,
            other => SurrealStorageError::Database(surrealdb::Error::internal(other.to_string())),
        }
    }
}

#[derive(SurrealValue)]
struct PrincipalLookupRow {
    account_id: RecordId,
    principal_id: RecordId,
    access_space_id: RecordId,
}

#[derive(SurrealValue)]
struct CanonicalPrincipalRow {
    account_id: RecordId,
    principal_id: RecordId,
}

#[derive(SurrealValue)]
struct CanonicalAccessSpaceRow {
    account_id: RecordId,
    access_space_id: RecordId,
}

#[derive(SurrealValue)]
struct SessionIssueRow {
    account_epoch: i64,
    principal_epoch: i64,
    space_epoch: i64,
    policy_version: i64,
    delegated_capabilities: Vec<String>,
}

#[derive(SurrealValue)]
pub(crate) struct CanonicalResourceLocator {
    pub id: RecordId,
    pub external_resource_id: String,
    pub owner_account_id: RecordId,
    pub access_space_id: RecordId,
}

#[derive(SurrealValue)]
struct AuthorizationRow {
    grant_id: RecordId,
    resource_id: RecordId,
    account_id: RecordId,
    principal_id: RecordId,
    session_id: RecordId,
    access_space_id: RecordId,
    actor_kind: String,
    actor_id: String,
    capability_profile_id: String,
    delegation_chain: Vec<String>,
    policy_version: i64,
}

#[derive(SurrealValue)]
struct SessionAuthorityRow {
    account_id: RecordId,
    principal_id: RecordId,
    session_id: RecordId,
    access_space_id: RecordId,
    delegation_chain: Vec<String>,
    policy_version: i64,
}

#[derive(SurrealValue)]
struct SessionAccessSpaceRow {
    account_id: RecordId,
}

#[derive(SurrealValue)]
struct AccessSpaceSwitchRow {
    account_id: RecordId,
    status: String,
    revocation_epoch: i64,
    policy_version: i64,
}

struct AuthorizationAttempt {
    session: Option<SessionAuthorityRow>,
    grant: Option<AuthorizationRow>,
}

#[derive(SurrealValue)]
struct ResourceLookupRow {
    id: RecordId,
    external_resource_id: String,
}

#[derive(SurrealValue)]
pub(crate) struct SigninParams {
    pub(crate) token_hash: String,
    pub(crate) channel_binding_hash: Option<String>,
}

fn resource_authority_test_base_schema() -> [&'static str; 3] {
    let schema = include_str!("schema.surql");
    [
        schema
            .split_once("-- 0001_init")
            .and_then(|(_, tail)| tail.split_once("-- 0002-0011"))
            .map(|(block, _)| block)
            .expect("compiled schema contains the 0001 base-table range"),
        schema
            .split_once("-- 0018_kernel_event_ledger")
            .and_then(|(_, tail)| tail.split_once("-- 0019_kernel_session_queue"))
            .map(|(block, _)| block)
            .expect("compiled schema contains the 0018 event-ledger range"),
        schema
            .split_once(
                "-- 0345_fems_memory_workspace_authority. The historical legacy server backend",
            )
            .and_then(|(_, tail)| tail.split_once("-- 0353_calendar_lossless_temporal_contract"))
            .map(|(block, _)| block)
            .expect("compiled schema contains the 0345-0352 FEMS authority range"),
    ]
}

#[derive(Clone, Debug)]
pub(crate) struct RecordUserScope {
    pub(crate) grant_id: Option<String>,
    pub(crate) workspace_id: Option<String>,
    pub(crate) session_token: String,
    pub(crate) channel_binding_hash: Option<String>,
    pub(crate) resource_id: String,
    pub(crate) session_id: String,
    pub(crate) capability_id: String,
    pub(crate) action: ResourceAction,
}

/// Bound values for a newly generated ingestion source or code-file row. The
/// record-user transaction and immutable schema stamps enforce creation ownership.
#[derive(SurrealValue)]
pub(crate) struct OwnedIndexResource {
    pub resource: RecordId,
    pub grant: RecordId,
    pub parent: RecordId,
    pub parent_grant: RecordId,
    pub delete_parent_grant: Option<RecordId>,
    pub kind: String,
    pub external_id: String,
    pub session: RecordId,
    pub account: RecordId,
    pub principal: RecordId,
    pub space: RecordId,
    pub policy_version: i64,
    pub delegation_chain: Vec<String>,
    pub locator_hash: String,
}

impl SurrealStorage {
    pub(crate) async fn prepare_owned_index_resource(
        &self,
        kind: ResourceKind,
        external_id: &str,
        parent_external_id: &str,
    ) -> Result<Option<OwnedIndexResource>, ResourceAuthorityError> {
        let Some(scope) = super::current_record_user_scope() else {
            return Ok(None);
        };
        let parent_kind = match kind {
            ResourceKind::KnowledgeSource => ResourceKind::Workspace,
            ResourceKind::KnowledgeCodeFile => ResourceKind::KnowledgeSource,
            _ => {
                return Err(ResourceAuthorityError::InvalidInput(
                    "invalid index resource kind",
                ))
            }
        };
        let parent = handshake_storage_support::diagnostics::observe_result(
            "index_source_parent_authorize",
            self.authorize_protected_resource(AuthorizationRequest {
                session_token: scope.session_token.clone(),
                channel_binding_hash: scope.channel_binding_hash.clone(),
                capability_id: "memory.propose".into(),
                resource_kind: parent_kind,
                external_resource_id: parent_external_id.to_owned(),
                action: ResourceAction::Create,
            }),
        )
        .await?;
        let delete_parent = handshake_storage_support::diagnostics::observe_result(
            "index_source_delete_authorize",
            self.authorize_protected_resource(AuthorizationRequest {
                session_token: scope.session_token.clone(),
                channel_binding_hash: scope.channel_binding_hash.clone(),
                capability_id: "fs.write".into(),
                resource_kind: parent_kind,
                external_resource_id: parent_external_id.to_owned(),
                action: ResourceAction::Delete,
            }),
        )
        .await
        .ok()
        .filter(|decision| {
            decision.resource_id == parent.resource_id && decision.session_id == parent.session_id
        });
        if parent.session_id != scope.session_id {
            return Err(ResourceAuthorityError::InvalidInput(
                "authentication denied",
            ));
        }
        Ok(Some(OwnedIndexResource {
            resource: RecordId::new("protected_resources", Uuid::now_v7().to_string()),
            grant: RecordId::new("resource_grants", Uuid::now_v7().to_string()),
            parent: RecordId::new("protected_resources", parent.resource_id),
            parent_grant: RecordId::new("resource_grants", parent.grant_id),
            delete_parent_grant: delete_parent
                .map(|decision| RecordId::new("resource_grants", decision.grant_id)),
            kind: kind.as_str().to_owned(),
            external_id: external_id.to_owned(),
            session: RecordId::new("authenticated_sessions", parent.session_id),
            account: RecordId::new("local_accounts", parent.account_id),
            principal: RecordId::new("principals", parent.principal_id),
            space: RecordId::new("access_spaces", parent.access_space_id),
            policy_version: parent.policy_version,
            delegation_chain: parent.delegation_chain,
            locator_hash: hex::encode(Sha256::digest(
                format!("{}:{external_id}", kind.as_str()).as_bytes(),
            )),
        }))
    }
}

impl SurrealStorage {
    pub(crate) async fn bootstrap_resource_authority_test_base_schema(
        &self,
    ) -> Result<(), ResourceAuthorityError> {
        let namespace = self.config().namespace().to_owned();
        let database = self.config().database().to_owned();
        self.with_lease(move |client| {
            Box::pin(async move {
                let authority = client.clone();
                authority.use_ns(namespace).use_db(database).await?;
                for block in resource_authority_test_base_schema() {
                    authority.query(block).await?.check()?;
                }
                Ok(())
            })
        })
        .await?;
        Ok(())
    }

    pub(crate) async fn bootstrap_resource_authority_schema(
        &self,
    ) -> Result<(), ResourceAuthorityError> {
        let namespace = self.config().namespace().to_owned();
        let database = self.config().database().to_owned();
        self.with_lease(move |client| {
            Box::pin(async move {
                let authority = client.clone();
                authority.use_ns(namespace).use_db(database.clone()).await?;
                // MT-154: the authority tables' permissions call the MT-154 authority-block
                // functions; the bounded bootstrap defines them too (only functions, no tables).
                authority
                    .query(format!(
                        "{}\n{}",
                        super::schema::resource_authority_schema_statements(),
                        super::schema::mt154_authority_function_block()
                    ))
                    .await?
                    .check()?;
                Ok(())
            })
        })
        .await?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn provision_principal(
        &self,
        account_key: &str,
        principal_key: &str,
        actor_kind: &str,
        actor_id: &str,
        capability_profile_id: &str,
        delegated_capabilities: &[String],
        space_key: &str,
        channel_binding_hash: Option<&str>,
        ttl: Duration,
    ) -> Result<ProvisionedPrincipal, ResourceAuthorityError> {
        for value in [
            account_key,
            principal_key,
            actor_kind,
            actor_id,
            capability_profile_id,
            space_key,
        ] {
            validate_nonempty(value)?;
        }
        if ttl.is_zero() {
            return Err(ResourceAuthorityError::InvalidInput(
                "session TTL must be greater than zero",
            ));
        }

        let account = RecordId::new("local_accounts", Uuid::now_v7().to_string());
        let principal = RecordId::new("principals", Uuid::now_v7().to_string());
        let space = RecordId::new("access_spaces", Uuid::now_v7().to_string());
        let namespace = self.config().namespace().to_owned();
        let database = self.config().database().to_owned();
        let account_key = account_key.to_owned();
        let principal_key = principal_key.to_owned();
        let principal_kind = actor_kind.to_owned();
        let actor_kind = match actor_kind {
            "human_account" | "break_glass_operator" => "operator",
            "local_model" | "remote_model" | "spawned_agent" | "mcp_client" => "agent",
            "service_identity" => "system",
            other => other,
        }
        .to_owned();
        let account_role = if principal_kind == "service_identity" {
            "Member"
        } else {
            "Owner"
        }
        .to_owned();
        let actor_id = actor_id.to_owned();
        let capability_profile_id = capability_profile_id.to_owned();
        let delegated_capabilities = delegated_capabilities.to_vec();
        let space_key = space_key.to_owned();
        let now = Datetime::from(Utc::now());

        let row = self
            .with_lease(move |client| {
                Box::pin(async move {
                    let authority = client.clone();
                    authority
                        .use_ns(namespace)
                        .use_db(database.clone())
                        .await?;
                    let mut provisioning_response = authority
                        .query(
                            "BEGIN TRANSACTION;\n\
                             LET $existing_account = (SELECT id FROM local_accounts WHERE account_key = $account_key LIMIT 1);\n\
                             IF array::len($existing_account) = 0 { CREATE $account SET account_key = $account_key, account_role = $account_role, status = 'enabled', revocation_epoch = 0, policy_version = 1, created_at = $now, updated_at = $now; };\n\
                             LET $resolved_account = (SELECT VALUE id FROM local_accounts WHERE account_key = $account_key LIMIT 1)[0];\n\
                             LET $existing_principal = (SELECT id, account_id FROM principals WHERE principal_key = $principal_key LIMIT 1);\n\
                             IF array::len($existing_principal) > 0 AND $existing_principal[0].account_id != $resolved_account { THROW 'HSK-AUTH-PRINCIPAL-ACCOUNT-MISMATCH'; };\n\
                             IF array::len($existing_principal) = 0 { CREATE $principal SET principal_key = $principal_key, account_id = $resolved_account, principal_kind = $principal_kind, actor_kind = $actor_kind, actor_id = $actor_id, capability_profile_id = $capability_profile_id, delegated_capabilities = $delegated_capabilities, status = 'enabled', revocation_epoch = 0, policy_version = 1, created_at = $now, updated_at = $now; };\n\
                             LET $existing_space = (SELECT id FROM access_spaces WHERE account_id = $resolved_account AND space_key = $space_key LIMIT 1);\n\
                             IF array::len($existing_space) = 0 { CREATE $space SET space_key = $space_key, account_id = $resolved_account, name = $space_key, status = 'active', revocation_epoch = 0, policy_version = 1, created_at = $now, updated_at = $now; };\n\
                             COMMIT TRANSACTION;",
                        )
                        .bind(("account", account))
                        .bind(("principal", principal))
                        .bind(("space", space))
                        .bind(("account_key", account_key.clone()))
                        .bind(("account_role", account_role))
                        .bind(("principal_key", principal_key.clone()))
                        .bind(("principal_kind", principal_kind))
                        .bind(("actor_kind", actor_kind))
                        .bind(("actor_id", actor_id))
                        .bind(("capability_profile_id", capability_profile_id))
                        .bind(("delegated_capabilities", delegated_capabilities))
                        .bind(("space_key", space_key.clone()))
                        .bind(("now", now))
                        .await?;
                    let mut errors = provisioning_response
                        .take_errors()
                        .into_iter()
                        .collect::<Vec<_>>();
                    errors.sort_by_key(|(statement_index, _)| *statement_index);
                    if !errors.is_empty() {
                        let meaningful = errors
                            .iter()
                            .position(|(_, error)| {
                                !error
                                    .to_string()
                                    .to_ascii_lowercase()
                                    .contains("query was not executed due to a failed transaction")
                            })
                            .unwrap_or(0);
                        return Err(errors.swap_remove(meaningful).1.into());
                    }

                    let mut principal_response = authority
                        .query(
                            "SELECT account_id, id AS principal_id FROM principals WHERE principal_key = $principal_key AND account_id.account_key = $account_key LIMIT 2;",
                        )
                        .bind(("principal_key", principal_key))
                        .bind(("account_key", account_key.clone()))
                        .await?
                        .check()?;
                    let principal_rows: Vec<CanonicalPrincipalRow> = principal_response.take(0)?;
                    if principal_rows.len() != 1 {
                        return Err(surrealdb::Error::internal(format!(
                            "principal provisioning returned {} canonical Principal rows",
                            principal_rows.len()
                        ))
                        .into());
                    }
                    let principal_row = principal_rows.into_iter().next().expect("length checked");

                    let mut space_response = authority
                        .query(
                            "SELECT account_id, id AS access_space_id FROM access_spaces WHERE account_id.account_key = $account_key AND space_key = $space_key LIMIT 2;",
                        )
                        .bind(("account_key", account_key))
                        .bind(("space_key", space_key))
                        .await?
                        .check()?;
                    let space_rows: Vec<CanonicalAccessSpaceRow> = space_response.take(0)?;
                    if space_rows.len() != 1 {
                        return Err(surrealdb::Error::internal(format!(
                            "principal provisioning returned {} canonical AccessSpace rows",
                            space_rows.len()
                        ))
                        .into());
                    }
                    let space_row = space_rows.into_iter().next().expect("length checked");
                    if principal_row.account_id != space_row.account_id {
                        return Err(surrealdb::Error::internal(
                            "canonical Principal and AccessSpace belong to different accounts"
                                .to_owned(),
                        )
                        .into());
                    }
                    Ok(PrincipalLookupRow {
                        account_id: principal_row.account_id,
                        principal_id: principal_row.principal_id,
                        access_space_id: space_row.access_space_id,
                    })
                })
            })
            .await?;

        let identity = ProvisionedIdentity {
            account_id: record_key(row.account_id)?,
            principal_id: record_key(row.principal_id)?,
            access_space_id: record_key(row.access_space_id)?,
        };
        let session = self
            .issue_authenticated_session(
                &identity.account_id,
                &identity.principal_id,
                &identity.access_space_id,
                channel_binding_hash,
                ttl,
            )
            .await?;
        Ok(ProvisionedPrincipal { identity, session })
    }

    #[cfg(test)]
    pub async fn provision_local_operator(
        &self,
        channel_binding_hash: Option<&str>,
    ) -> Result<ProvisionedPrincipal, ResourceAuthorityError> {
        self.provision_principal(
            "installation-owner",
            "installation-owner-human",
            "human_account",
            "local_operator",
            "Operator",
            &["*".to_owned()],
            "installation-owner-default-space",
            channel_binding_hash,
            Duration::from_secs(12 * 60 * 60),
        )
        .await
    }

    pub async fn provision_session_credential(
        &self,
        identity: &ProvisionedIdentity,
        ttl: Duration,
    ) -> Result<IssuedSessionCredential, ResourceAuthorityError> {
        validate_uuid(&identity.account_id)?;
        validate_uuid(&identity.principal_id)?;
        validate_uuid(&identity.access_space_id)?;
        if ttl.is_zero() {
            return Err(ResourceAuthorityError::InvalidInput(
                "credential TTL must be greater than zero",
            ));
        }
        let mut secret = [0_u8; 32];
        getrandom::getrandom(&mut secret)
            .map_err(|error| ResourceAuthorityError::Entropy(error.to_string()))?;
        let token = hex::encode(secret);
        let token_hash = sha256_hex(token.as_bytes());
        let credential_id = Uuid::now_v7().to_string();
        let credential = RecordId::new("session_exchange_credentials", credential_id.clone());
        let account = RecordId::new("local_accounts", identity.account_id.clone());
        let principal = RecordId::new("principals", identity.principal_id.clone());
        let space = RecordId::new("access_spaces", identity.access_space_id.clone());
        let expires_at = Utc::now()
            + chrono::Duration::from_std(ttl).map_err(|_| {
                ResourceAuthorityError::InvalidInput("credential TTL is outside chrono range")
            })?;
        let namespace = self.config().namespace().to_owned();
        let database = self.config().database().to_owned();
        self.with_lease(move |client| {
            Box::pin(async move {
                let authority = client.clone();
                authority
                    .use_ns(namespace)
                    .use_db(database)
                    .await?;
                authority
                    .query(
                        "LET $eligible = SELECT id FROM principals WHERE id = $principal AND account_id = $account AND status = 'enabled' AND $space.account_id = $account AND $space.status = 'active' LIMIT 1; IF array::len($eligible) = 0 { THROW 'HSK-AUTH-CREDENTIAL-IDENTITY'; }; CREATE $credential SET account_id = $account, principal_id = $principal, access_space_id = $space, token_hash = $token_hash, status = 'active', expires_at = $expires_at, revoked_at = NONE, created_at = time::now();",
                    )
                    .bind(("credential", credential))
                    .bind(("account", account))
                    .bind(("principal", principal))
                    .bind(("space", space))
                    .bind(("token_hash", token_hash))
                    .bind(("expires_at", Datetime::from(expires_at)))
                    .await?
                    .check()?;
                Ok(())
            })
        })
        .await?;
        Ok(IssuedSessionCredential {
            credential_id,
            token,
            account_id: identity.account_id.clone(),
            principal_id: identity.principal_id.clone(),
            access_space_id: identity.access_space_id.clone(),
            expires_at,
        })
    }

    pub async fn exchange_session_credential(
        &self,
        account_id: &str,
        principal_id: &str,
        access_space_id: &str,
        credential_token: &str,
        channel_binding_hash: &str,
        ttl: Duration,
    ) -> Result<IssuedSession, ResourceAuthorityError> {
        validate_uuid(account_id)?;
        validate_uuid(principal_id)?;
        validate_uuid(access_space_id)?;
        validate_nonempty(credential_token)?;
        validate_nonempty(channel_binding_hash)?;
        if ttl.is_zero() {
            return Err(ResourceAuthorityError::InvalidInput(
                "session TTL must be greater than zero",
            ));
        }
        let mut secret = [0_u8; 32];
        getrandom::getrandom(&mut secret)
            .map_err(|error| ResourceAuthorityError::Entropy(error.to_string()))?;
        let token = hex::encode(secret);
        let session_token_hash = sha256_hex(token.as_bytes());
        let credential_token_hash = sha256_hex(credential_token.as_bytes());
        let session_id = Uuid::now_v7().to_string();
        let now = Utc::now();
        let expires_at = now
            + chrono::Duration::from_std(ttl).map_err(|_| {
                ResourceAuthorityError::InvalidInput("session TTL is outside chrono range")
            })?;
        let namespace = self.config().namespace().to_owned();
        let database = self.config().database().to_owned();
        let account = RecordId::new("local_accounts", account_id.to_owned());
        let principal = RecordId::new("principals", principal_id.to_owned());
        let principal_key = principal_id.to_owned();
        let space = RecordId::new("access_spaces", access_space_id.to_owned());
        let session = RecordId::new("authenticated_sessions", session_id.clone());
        let channel_binding_hash = channel_binding_hash.to_owned();

        self.with_lease(move |client| {
            Box::pin(async move {
                let authority = client.clone();
                authority.use_ns(namespace).use_db(database).await?;
                authority
                    .query(
                        "BEGIN TRANSACTION; LET $eligible = SELECT VALUE id FROM session_exchange_credentials WHERE token_hash = $credential_token_hash AND status = 'active' AND revoked_at = NONE AND expires_at > time::now() AND account_id = $account AND principal_id = $principal AND access_space_id = $space AND account_id.status = 'enabled' AND principal_id.status = 'enabled' AND access_space_id.status = 'active' AND access_space_id.account_id = account_id LIMIT 1; IF array::len($eligible) = 0 { THROW 'HSK-AUTH-CREDENTIAL-DENIED'; }; UPDATE session_exchange_credentials SET status = 'consumed' WHERE id IN $eligible; CREATE $session_record SET account_id = $account, principal_id = $principal, access_space_id = $space, token_hash = $session_token_hash, channel_binding_hash = $channel_binding_hash, authentication_strength = 'credential_exchange', delegated_capabilities = $principal.delegated_capabilities, delegation_chain = [$principal_key], account_revocation_epoch = $account.revocation_epoch, principal_revocation_epoch = $principal.revocation_epoch, space_revocation_epoch = $space.revocation_epoch, policy_version = math::max([$account.policy_version, $principal.policy_version, $space.policy_version]), issued_at = $issued_at, expires_at = $expires_at, revoked_at = NONE; COMMIT TRANSACTION;",
                    )
                    .bind(("credential_token_hash", credential_token_hash))
                    .bind(("session_token_hash", session_token_hash))
                    .bind(("session_record", session))
                    .bind(("account", account))
                    .bind(("principal", principal))
                    .bind(("principal_key", principal_key))
                    .bind(("space", space))
                    .bind(("channel_binding_hash", channel_binding_hash))
                    .bind(("issued_at", Datetime::from(now)))
                    .bind(("expires_at", Datetime::from(expires_at)))
                    .await?
                    .check()?;
                Ok(())
            })
        })
        .await?;

        Ok(IssuedSession {
            token,
            account_id: account_id.to_owned(),
            principal_id: principal_id.to_owned(),
            session_id,
            access_space_id: access_space_id.to_owned(),
            expires_at,
        })
    }

    pub async fn revoke_session_credential(
        &self,
        credential_id: &str,
    ) -> Result<(), ResourceAuthorityError> {
        self.update_authority_record(
            "session_exchange_credentials",
            credential_id,
            "UPDATE $record SET status = 'revoked', revoked_at = time::now();",
        )
        .await
    }

    pub async fn issue_authenticated_session(
        &self,
        account_id: &str,
        principal_id: &str,
        access_space_id: &str,
        channel_binding_hash: Option<&str>,
        ttl: Duration,
    ) -> Result<IssuedSession, ResourceAuthorityError> {
        validate_uuid(account_id)?;
        validate_uuid(principal_id)?;
        validate_uuid(access_space_id)?;
        if ttl.is_zero() {
            return Err(ResourceAuthorityError::InvalidInput(
                "session TTL must be greater than zero",
            ));
        }
        let mut secret = [0_u8; 32];
        getrandom::getrandom(&mut secret)
            .map_err(|error| ResourceAuthorityError::Entropy(error.to_string()))?;
        let token = hex::encode(secret);
        let token_hash = sha256_hex(token.as_bytes());
        let session_id = Uuid::now_v7().to_string();
        let now = Utc::now();
        let expires_at = now
            + chrono::Duration::from_std(ttl).map_err(|_| {
                ResourceAuthorityError::InvalidInput("session TTL is outside chrono range")
            })?;
        let namespace = self.config().namespace().to_owned();
        let database = self.config().database().to_owned();
        let account = RecordId::new("local_accounts", account_id.to_owned());
        let principal = RecordId::new("principals", principal_id.to_owned());
        let space = RecordId::new("access_spaces", access_space_id.to_owned());
        let session = RecordId::new("authenticated_sessions", session_id.clone());
        let channel_binding_hash = channel_binding_hash.map(str::to_owned);
        let principal_key = principal_id.to_owned();

        self.with_lease(move |client| {
            Box::pin(async move {
                let authority = client.clone();
                authority
                    .use_ns(namespace)
                    .use_db(database.clone())
                    .await?;
                let mut lookup = authority
                        .query(
                            "SELECT account_id.revocation_epoch AS account_epoch, revocation_epoch AS principal_epoch, $space.revocation_epoch AS space_epoch, math::max([account_id.policy_version, policy_version, $space.policy_version]) AS policy_version, delegated_capabilities FROM $principal WHERE account_id = $account AND status = 'enabled' AND $space.account_id = $account AND $space.status = 'active';",
                    )
                    .bind(("account", account.clone()))
                    .bind(("principal", principal.clone()))
                    .bind(("space", space.clone()))
                    .await?
                    .check()?;
                let rows: Vec<SessionIssueRow> = lookup.take(0)?;
                let eligible = rows.into_iter().next().ok_or_else(|| {
                    surrealdb::Error::internal(
                        "session identity is not currently eligible".to_owned(),
                    )
                })?;
                authority
                    .query(
                        "CREATE $session_record SET account_id = $account, principal_id = $principal, access_space_id = $space, token_hash = $token_hash, channel_binding_hash = $channel_binding_hash, authentication_strength = 'local_opaque', delegated_capabilities = $delegated_capabilities, delegation_chain = [$principal_key], account_revocation_epoch = $account_epoch, principal_revocation_epoch = $principal_epoch, space_revocation_epoch = $space_epoch, policy_version = $policy_version, issued_at = $issued_at, expires_at = $expires_at, revoked_at = NONE;",
                    )
                    .bind(("session_record", session))
                    .bind(("account", account))
                    .bind(("principal", principal.clone()))
                    .bind(("space", space))
                    .bind(("token_hash", token_hash))
                    .bind(("channel_binding_hash", channel_binding_hash))
                    .bind(("delegated_capabilities", eligible.delegated_capabilities))
                    .bind(("principal_key", principal_key))
                    .bind(("account_epoch", eligible.account_epoch))
                    .bind(("principal_epoch", eligible.principal_epoch))
                    .bind(("space_epoch", eligible.space_epoch))
                    .bind(("policy_version", eligible.policy_version))
                    .bind(("issued_at", Datetime::from(now)))
                    .bind(("expires_at", Datetime::from(expires_at)))
                    .await?
                    .check()?;
                Ok(())
            })
        })
        .await?;

        Ok(IssuedSession {
            token,
            account_id: account_id.to_owned(),
            principal_id: principal_id.to_owned(),
            session_id,
            access_space_id: access_space_id.to_owned(),
            expires_at,
        })
    }

    pub async fn register_workspace_resource(
        &self,
        owner: &ProvisionedIdentity,
        workspace_id: &str,
    ) -> Result<RegisteredResource, ResourceAuthorityError> {
        self.register_protected_resource(
            owner,
            ResourceKind::Workspace,
            workspace_id,
            None,
            "private",
        )
        .await
    }

    pub async fn register_protected_resource(
        &self,
        owner: &ProvisionedIdentity,
        resource_kind: ResourceKind,
        external_resource_id: &str,
        parent_resource_id: Option<&str>,
        classification: &str,
    ) -> Result<RegisteredResource, ResourceAuthorityError> {
        validate_uuid(&owner.account_id)?;
        validate_uuid(&owner.principal_id)?;
        validate_uuid(&owner.access_space_id)?;
        validate_nonempty(external_resource_id)?;
        validate_nonempty(classification)?;
        if let Some(parent) = parent_resource_id {
            validate_uuid(parent)?;
        }
        let resource_id = Uuid::now_v7().to_string();
        let resource = RecordId::new("protected_resources", resource_id.clone());
        let namespace = self.config().namespace().to_owned();
        let database = self.config().database().to_owned();
        let account = RecordId::new("local_accounts", owner.account_id.clone());
        let principal = RecordId::new("principals", owner.principal_id.clone());
        let space = RecordId::new("access_spaces", owner.access_space_id.clone());
        let kind = resource_kind.as_str().to_owned();
        let external = external_resource_id.to_owned();
        let classification = classification.to_owned();
        let parent =
            parent_resource_id.map(|id| RecordId::new("protected_resources", id.to_owned()));
        let locator_hash = sha256_hex(format!("{kind}:{external}").as_bytes());
        let now = Datetime::from(Utc::now());

        let row = self
            .with_lease(move |client| {
                Box::pin(async move {
                    let authority = client.clone();
                    authority
                        .use_ns(namespace)
                        .use_db(database.clone())
                        .await?;
                    let mut response = authority
                        .query(
                            "IF array::len(SELECT id FROM protected_resources WHERE resource_kind = $kind AND external_resource_id = $external LIMIT 1) = 0 { CREATE $resource SET resource_kind = $kind, external_resource_id = $external, owner_account_id = $account, created_by_principal_id = $principal, created_in_session_id = NONE, access_space_id = $space, parent_resource_id = $parent, schema_version = 1, lifecycle_state = 'active', policy_version = 1, classification = $classification, storage_locator_hash = $locator_hash, created_at = $now, updated_at = $now; }; SELECT id, external_resource_id FROM protected_resources WHERE resource_kind = $kind AND external_resource_id = $external AND owner_account_id = $account AND access_space_id = $space LIMIT 1;",
                        )
                        .bind(("resource", resource))
                        .bind(("kind", kind))
                        .bind(("external", external))
                        .bind(("account", account))
                        .bind(("principal", principal))
                        .bind(("space", space))
                        .bind(("parent", parent))
                        .bind(("classification", classification))
                        .bind(("locator_hash", locator_hash))
                        .bind(("now", now))
                        .await?
                        .check()?;
                    let rows: Vec<ResourceLookupRow> = response.take(1)?;
                    rows.into_iter().next().ok_or_else(|| {
                        surrealdb::Error::internal(
                            "resource registry collision or ownership mismatch".to_owned(),
                        )
                        .into()
                    })
                })
            })
            .await?;

        Ok(RegisteredResource {
            resource_id: record_key(row.id)?,
            resource_kind,
            external_resource_id: row.external_resource_id,
        })
    }

    pub async fn grant_resource(
        &self,
        account_id: &str,
        access_space_id: &str,
        spec: ResourceGrantSpec,
    ) -> Result<ResourceGrant, ResourceAuthorityError> {
        validate_uuid(account_id)?;
        validate_uuid(access_space_id)?;
        validate_uuid(&spec.principal_id)?;
        validate_uuid(&spec.resource_id)?;
        if spec.actions.is_empty() || spec.capability_ids.is_empty() {
            return Err(ResourceAuthorityError::InvalidInput(
                "grant actions and capabilities must not be empty",
            ));
        }
        for capability in &spec.capability_ids {
            validate_nonempty(capability)?;
        }
        let grant_id = Uuid::now_v7().to_string();
        let namespace = self.config().namespace().to_owned();
        let database = self.config().database().to_owned();
        let grant = RecordId::new("resource_grants", grant_id.clone());
        let account = RecordId::new("local_accounts", account_id.to_owned());
        let principal = RecordId::new("principals", spec.principal_id.clone());
        let space = RecordId::new("access_spaces", access_space_id.to_owned());
        let resource = RecordId::new("protected_resources", spec.resource_id.clone());
        let actions = spec
            .actions
            .iter()
            .map(|action| action.as_str().to_owned())
            .collect::<Vec<_>>();
        let capabilities = spec.capability_ids.clone();
        let delegation_chain = if spec.delegation_chain.is_empty() {
            vec![spec.principal_id.clone()]
        } else {
            spec.delegation_chain.clone()
        };
        let stored_delegation_chain = delegation_chain.clone();
        let expires_at = spec.expires_at.map(Datetime::from);
        let now = Datetime::from(Utc::now());

        self.with_lease(move |client| {
            Box::pin(async move {
                let authority = client.clone();
                authority
                    .use_ns(namespace)
                    .use_db(database.clone())
                    .await?;
                authority
                    .query(
                        "CREATE $grant SET account_id = $account, principal_id = $principal, access_space_id = $space, resource_id = $resource, actions = $actions, capability_ids = $capabilities, delegation_chain = $delegation_chain, status = 'active', grant_version = 1, policy_version = math::max([$account.policy_version, $principal.policy_version, $space.policy_version, $resource.policy_version]), expires_at = $expires_at, revoked_at = NONE, created_at = $now, updated_at = $now;",
                    )
                    .bind(("grant", grant))
                    .bind(("account", account))
                    .bind(("principal", principal))
                    .bind(("space", space))
                    .bind(("resource", resource))
                    .bind(("actions", actions))
                    .bind(("capabilities", capabilities))
                    .bind(("delegation_chain", stored_delegation_chain))
                    .bind(("expires_at", expires_at))
                    .bind(("now", now))
                    .await?
                    .check()?;
                Ok(())
            })
        })
        .await?;

        Ok(ResourceGrant {
            grant_id,
            account_id: account_id.to_owned(),
            principal_id: spec.principal_id,
            access_space_id: access_space_id.to_owned(),
            resource_id: spec.resource_id,
            actions: spec.actions,
            capability_ids: spec.capability_ids,
            delegation_chain,
            policy_version: 1,
            expires_at: spec.expires_at,
        })
    }

    /// Canonical identity locator only; ordinary grant and guarded authority checks remain required.
    pub(crate) async fn canonical_resource_locator(
        &self, kind: ResourceKind, external: &str,
    ) -> Result<Option<CanonicalResourceLocator>, ResourceAuthorityError> {
        let namespace = self.config().namespace().to_owned();
        let database = self.config().database().to_owned();
        let kind = kind.as_str().to_owned();
        let external = external.to_owned();
        let rows = self.with_lease(move |client| Box::pin(async move {
            let authority = client.clone();
            authority.use_ns(namespace).use_db(database).await?;
            let mut response = handshake_storage_support::diagnostics::observe_result("authority_resource_query", authority.query(
                "SELECT id, external_resource_id, owner_account_id, access_space_id FROM protected_resources WHERE resource_kind = $kind AND external_resource_id = $external LIMIT 2;",
            ).bind(("kind", kind)).bind(("external", external))).await?.check()?;
            let rows: Vec<CanonicalResourceLocator> = response.take(0)?;
            Ok(rows)
        })).await?;
        match rows.len() {
            0 => Ok(None),
            1 => Ok(rows.into_iter().next()),
            _ => Err(SurrealStorageError::from(surrealdb::Error::internal("protected resource lookup was not canonical".to_owned())).into()),
        }
    }

    async fn with_activation_transaction<T, F>(
        &self, fence: ActivationPublicationFence, operation: F,
    ) -> Result<T, ResourceAuthorityError>
    where
        T: Send + 'static,
        F: for<'a> FnOnce(super::SurrealTransactionContext<'a>)
            -> super::SurrealTransactionOperation<'a, T, ResourceAuthorityError> + Send + 'static,
    {
        let storage=self.clone();
        // The owning await survives request cancellation, retaining the fence through
        // the existing controller's observed commit/cancel, including callback errors.
        tokio::spawn(async move {
            let result=storage.with_transaction(operation).await;
            drop(fence);
            result
        }).await.map_err(|_|ResourceAuthorityError::InvalidInput("activation transaction settlement unresolved"))?
    }

    pub(crate) async fn capture_activation_authority(
        &self, operation_id: Uuid, scope: RecordUserScope,
        association: crate::swarm_orchestration::resource_scope::ResourceAccessGrantAssociation,
        registry: &crate::capabilities::CapabilityRegistry,
    ) -> Result<CapturedActivationAuthority, ResourceAuthorityError> {
        if operation_id.is_nil() || scope.capability_id != "model_lane.navigation.read"
            || scope.action != ResourceAction::Read || association.capability_id != scope.capability_id
            || association.action != "read" || association.purpose != "navigation_read"
            || scope.session_id != association.exact.authenticated_session_id.to_string()
            || scope.workspace_id.as_deref() != Some(association.exact.workspace_id.as_str())
            || scope.grant_id.as_deref() != Some(association.grant_id.as_str())
            || scope.resource_id != association.resource_id
        { return Err(ResourceAuthorityError::InvalidInput("activation original authority mismatch")); }
        let bindings = NavigationActivationBindings {
            token_hash:sha256_hex(scope.session_token.as_bytes()),
            channel_hash:scope.channel_binding_hash.clone().ok_or(ResourceAuthorityError::InvalidInput("activation channel unavailable"))?,
            session:RecordId::new("authenticated_sessions",scope.session_id.clone()),
            resource:RecordId::new("protected_resources",association.resource_id.clone()),
            original_grant:Some(RecordId::new("resource_grants",association.grant_id.clone())),
            workspace:association.exact.workspace_id.as_str().to_owned(),
        };
        let registry = registry.clone();
        let fence=std::sync::Arc::new(self.authority_publication_read().await?);
        let facts = self.with_activation_transaction(fence, move |transaction| Box::pin(async move {
            navigation_activation_facts(&transaction, bindings, &registry).await
        })).await?;
        let exact = &association.exact;
        if facts.account_id != RecordId::new("local_accounts",exact.owner_account_id.to_string())
            || facts.principal_id != RecordId::new("principals",exact.actor_principal_id.to_string())
            || facts.access_space_id != RecordId::new("access_spaces",exact.access_space_id.to_string())
            || facts.grant_version != association.grant_version
            || facts.policy_versions.first().copied() != Some(association.policy_version)
        { return Err(ResourceAuthorityError::InvalidInput("activation original authority changed")); }
        Ok(CapturedActivationAuthority { operation_id,scope,association,facts })
    }

    pub(crate) async fn settle_activation_and_publish<R, F, D>(
        &self, captured: &CapturedActivationAuthority,
        registry: &crate::capabilities::CapabilityRegistry, runtime: &R, publication: F, deliver: D,
    ) -> Result<(), ActivationSettlementError>
    where R: Serialize + Sync, F: FnOnce()->Result<(),ResourceAuthorityError>+Send,
        D: FnOnce(&super::workspace_activation_outcome::CanonicalAppliedActivation)->Result<(),ResourceAuthorityError>+Send {
        self.settle_activation_and_publish_inner(captured,registry,runtime,publication,deliver)
            .await.map_err(|cause|ActivationSettlementError{cause})
    }

    async fn settle_activation_and_publish_inner<R, F, D>(
        &self, captured: &CapturedActivationAuthority,
        registry: &crate::capabilities::CapabilityRegistry, runtime: &R, publication: F, deliver: D,
    ) -> Result<(), ResourceAuthorityError>
    where R: Serialize + Sync, F: FnOnce()->Result<(),ResourceAuthorityError>+Send,
        D: FnOnce(&super::workspace_activation_outcome::CanonicalAppliedActivation)->Result<(),ResourceAuthorityError>+Send {
        use super::workspace_activation_outcome::{self as outcome, ActivationPreparation};
        let fence=std::sync::Arc::new(self.authority_publication_read().await?);
        let owner = captured.clone();
        let registry_before = registry.clone();
        let cancellation = super::ContextOperationCancellation::new();
        let prepared = self.with_owned_activation_transaction(
            captured.clone(), fence.clone(), cancellation.clone(), move |transaction| Box::pin(async move {
            let before = navigation_activation_facts(&transaction, owner.bindings()?, &registry_before).await?;
            if before != owner.facts { return Err(ResourceAuthorityError::InvalidInput("activation original authority changed")); }
            let preparation = outcome::prepare_activation(&transaction,&owner.envelope()?).await?;
            let after = navigation_activation_facts(&transaction, owner.bindings()?, &registry_before).await?;
            if after != before { return Err(ResourceAuthorityError::InvalidInput("activation authority changed before prepare commit")); }
            Ok::<_,ResourceAuthorityError>(preparation)
        })).await?;
        let newly_prepared = match prepared {
            ActivationPreparation::NewlyPrepared(owner)=>owner,
            ActivationPreparation::Pending=>return Err(ResourceAuthorityError::InvalidInput("activation settlement unresolved")),
            ActivationPreparation::Applied(applied)=> {
                return self.resource_access_lifecycle().with_authorized_association(&captured.association,|| {
                    captured.require_unexpired()?;
                    deliver(&applied)
                }).map_err(|_|ResourceAuthorityError::InvalidInput("activation delivery lifecycle denied"))?;
            },
        };
        // This asynchronous recheck finishes before entering the synchronous registry lock.
        #[cfg(test)]
        if let Some(latch) = self.activation_prepared_test_latch(captured.operation_id) {
            latch.observe_and_wait().await?;
        }
        let owner = captured.clone();
        let registry_publication = registry.clone();
        self.with_activation_transaction(fence.clone(), move |transaction|Box::pin(async move {
            let current=navigation_activation_facts(&transaction,owner.bindings()?,&registry_publication).await?;
            if current!=owner.facts { return Err(ResourceAuthorityError::InvalidInput("activation publication authority changed")); }
            Ok::<_,ResourceAuthorityError>(())
        })).await?;
        let lifecycle=self.resource_access_lifecycle();
        let witness=lifecycle.with_authorized_association(&captured.association,|| {
            captured.require_unexpired()?;
            newly_prepared.publish_after_observed_commit(runtime,publication)
        }).map_err(|_|ResourceAuthorityError::InvalidInput("activation publication lifecycle denied"))??;
        let owner=captured.clone();
        let registry_final=registry.clone();
        let applied=self.with_owned_activation_transaction(
            captured.clone(), fence.clone(), cancellation, move |transaction|Box::pin(async move {
            let before=navigation_activation_facts(&transaction,owner.bindings()?,&registry_final).await?;
            if before!=owner.facts { return Err(ResourceAuthorityError::InvalidInput("activation finalization authority changed")); }
            let applied=outcome::finalize_activation(&transaction,witness).await?;
            let after=navigation_activation_facts(&transaction,owner.bindings()?,&registry_final).await?;
            if after!=before { return Err(ResourceAuthorityError::InvalidInput("activation authority changed before final commit")); }
            Ok::<_,ResourceAuthorityError>(applied)
        })).await?;
        let expected_result=applied.result().clone();
        let expected_witness=applied.publication_witness().clone();
        let owner=captured.clone();
        let registry_read=registry.clone();
        let canonical=self.with_activation_transaction(fence.clone(), move |transaction|Box::pin(async move {
            let before=navigation_activation_facts(&transaction,owner.bindings()?,&registry_read).await?;
            if before!=owner.facts { return Err(ResourceAuthorityError::InvalidInput("activation delivery authority changed")); }
            let stored=outcome::read_activation(&transaction,owner.operation_id).await?
                .ok_or(ResourceAuthorityError::InvalidInput("activation settlement unresolved"))?;
            let confirmed=stored.applied().ok_or(ResourceAuthorityError::InvalidInput("activation settlement unresolved"))?;
            if stored.envelope()!=&owner.envelope()? || confirmed.result()!=&expected_result
                || confirmed.publication_witness()!=&expected_witness {
                return Err(ResourceAuthorityError::InvalidInput("activation settlement unresolved"));
            }
            let after=navigation_activation_facts(&transaction,owner.bindings()?,&registry_read).await?;
            if after!=before { return Err(ResourceAuthorityError::InvalidInput("activation delivery authority changed")); }
            Ok::<_,ResourceAuthorityError>(stored)
        })).await?;
        let canonical=canonical.applied().ok_or(ResourceAuthorityError::InvalidInput("activation settlement unresolved"))?;
        lifecycle.with_authorized_association(&captured.association,|| {
            captured.require_unexpired()?;
            deliver(canonical)
        }).map_err(|_|ResourceAuthorityError::InvalidInput("activation delivery lifecycle denied"))?
    }

    pub(crate) async fn read_activation_and_publish<F>(
        &self, token:&str, channel_hash:&str, operation_id:Uuid, workspace_id:&str,
        registry:&crate::capabilities::CapabilityRegistry, publish:F,
    )->Result<serde_json::Value,ActivationObservationError>
    where F:for<'a> FnOnce(AuthorizedActivationObservation<'a>)->Result<serde_json::Value,ResourceAuthorityError>+Send {
        use crate::swarm_orchestration::resource_scope::{AccessSpaceRef,ActorPrincipalId,AuthenticatedSessionRef,
            ExactResourceScopeAttribution,OwnerAccountId,ResourceAccessGrantAssociation,
            ResourceAccessLifecycleDecision,ResourceAccessLifecycleState,WorkspaceScopeRef};
        let fence=std::sync::Arc::new(self.authority_publication_read().await.map_err(|_|ActivationObservationError::Unavailable)?);
        let session=self.authenticate_local_session(token,channel_hash).await.map_err(activation_observation_authority_error)?;
        if !session.delegated_capabilities.iter().any(|capability|capability=="model_lane.navigation.read") {
            return Err(ActivationObservationError::Authentication);
        }
        let locator=self.canonical_resource_locator(ResourceKind::Workspace,workspace_id).await
            .map_err(|_|ActivationObservationError::Unavailable)?.ok_or(ActivationObservationError::NotFound)?;
        if locator.owner_account_id!=RecordId::new("local_accounts",session.identity.account_id.clone())
            || locator.access_space_id!=RecordId::new("access_spaces",session.identity.access_space_id.clone())
        { return Err(ActivationObservationError::NotFound); }
        let parse_id=|id:&str|Uuid::parse_str(id).map_err(|_|ActivationObservationError::Authentication);
        let exact=ExactResourceScopeAttribution {
            owner_account_id:OwnerAccountId::from_uuid(parse_id(&session.identity.account_id)?),
            actor_principal_id:ActorPrincipalId::from_uuid(parse_id(&session.identity.principal_id)?),
            authenticated_session_id:AuthenticatedSessionRef::from_uuid(parse_id(&session.session_id)?),
            access_space_id:AccessSpaceRef::from_uuid(parse_id(&session.identity.access_space_id)?),
            workspace_id:WorkspaceScopeRef::new(workspace_id).map_err(|_|ActivationObservationError::NotFound)?,
        };
        let lifecycle=self.resource_access_lifecycle();
        let bindings=NavigationActivationBindings {
            token_hash:sha256_hex(token.as_bytes()),channel_hash:channel_hash.to_owned(),
            session:RecordId::new("authenticated_sessions",session.session_id.clone()),resource:locator.id,
            original_grant:None,workspace:workspace_id.to_owned(),
        };
        let registry=registry.clone();let exact_for_read=exact.clone();
        let token=token.to_owned();let channel_hash=channel_hash.to_owned();
        let (captured,stored)=self.with_activation_transaction(fence.clone(), move |transaction|Box::pin(async move {
            let stored=super::workspace_activation_outcome::read_activation(&transaction,operation_id).await?;
            let mut bindings=bindings;
            if let Some(stored)=&stored {
                let envelope=stored.envelope();
                if envelope.owner_account_id!=exact_for_read.owner_account_id.to_string()
                    || envelope.actor_principal_id!=exact_for_read.actor_principal_id.to_string()
                    || envelope.authenticated_session_id!=exact_for_read.authenticated_session_id.to_string()
                    || envelope.access_space_id!=exact_for_read.access_space_id.to_string()
                    || envelope.workspace_id!=exact_for_read.workspace_id.as_str()
                { return Err(ResourceAuthorityError::InvalidInput("activation observation tuple mismatch")); }
                bindings.original_grant=Some(RecordId::new("resource_grants",envelope.grant_id.clone()));
            }
            let before=navigation_activation_facts(&transaction,bindings.clone(),&registry).await?;
            let association=ResourceAccessGrantAssociation {exact:exact_for_read,
                resource_id:record_key(before.resource_id.clone())?,grant_id:record_key(before.grant_id.clone())?,
                grant_version:before.grant_version,policy_version:*before.policy_versions.first()
                    .ok_or(ResourceAuthorityError::InvalidInput("activation policy unavailable"))?,
                capability_id:"model_lane.navigation.read".into(),action:"read".into(),purpose:"navigation_read".into()};
            let scope=RecordUserScope {grant_id:Some(association.grant_id.clone()),
                workspace_id:Some(association.exact.workspace_id.as_str().to_owned()),session_token:token,
                channel_binding_hash:Some(channel_hash),resource_id:association.resource_id.clone(),
                session_id:association.exact.authenticated_session_id.to_string(),
                capability_id:"model_lane.navigation.read".into(),action:ResourceAction::Read};
            let captured=CapturedActivationAuthority {operation_id,scope,association,facts:before.clone()};
            if stored.as_ref().is_some_and(|stored|captured.envelope().map_or(true,|envelope|stored.envelope()!=&envelope)) {
                return Err(ResourceAuthorityError::InvalidInput("activation observation original authority changed"));
            }
            let after=navigation_activation_facts(&transaction,bindings,&registry).await?;
            if after!=before { return Err(ResourceAuthorityError::InvalidInput("activation observation authority changed")); }
            Ok::<_,ResourceAuthorityError>((captured,stored))
        })).await.map_err(|error| match error {
            ResourceAuthorityError::InvalidInput("activation observation tuple mismatch")=>ActivationObservationError::NotFound,
            error=>activation_observation_authority_error(error),
        })?;
        match lifecycle.decision(&exact).map_err(|_|ActivationObservationError::Unavailable)? {
            ResourceAccessLifecycleDecision::Unknown=>return Err(ActivationObservationError::Unavailable),
            ResourceAccessLifecycleDecision::Known{state:ResourceAccessLifecycleState::Active,..}=>{},
            _=>return Err(ActivationObservationError::Authentication),
        }
        let envelope=captured.envelope().map_err(|_|ActivationObservationError::Unavailable)?;
        lifecycle.with_authorized_association(&captured.association,|| {
            captured.require_unexpired().map_err(|_|ActivationObservationError::Authentication)?;
            publish(AuthorizedActivationObservation{envelope:&envelope,exact:&captured.association.exact,outcome:stored.as_ref()})
                .map_err(|_|ActivationObservationError::Unavailable)
        }).map_err(|_|ActivationObservationError::Authentication)?
    }

    pub(crate) async fn with_navigation_publication_authority<T, E, F>(
        &self,
        scope: RecordUserScope,
        association: crate::swarm_orchestration::resource_scope::ResourceAccessGrantAssociation,
        capability_registry: &crate::capabilities::CapabilityRegistry,
        publication: F,
    ) -> Result<Result<T, E>, ResourceAuthorityError>
    where
        T: Send,
        E: Send,
        F: FnOnce(&CanonicalNavigationPublicationAuthority) -> Result<T, E> + Send,
    {
        let _gate = self.authority_publication_read().await?;
        let exact = &association.exact;
        if scope.capability_id != "model_lane.navigation.read"
            || scope.action != ResourceAction::Read
            || association.capability_id != scope.capability_id
            || association.action != "read" || association.purpose != "navigation_read"
            || scope.workspace_id.as_deref() != Some(exact.workspace_id.as_str())
            || scope.session_id != exact.authenticated_session_id.to_string()
            || scope.resource_id != association.resource_id
            || scope.grant_id.as_deref() != Some(association.grant_id.as_str())
        {
            return Err(ResourceAuthorityError::InvalidInput("navigation publication scope mismatch"));
        }
        let locator = self.canonical_resource_locator(ResourceKind::Workspace, exact.workspace_id.as_str()).await?
            .ok_or(ResourceAuthorityError::InvalidInput("navigation workspace unavailable"))?;
        if locator.id != RecordId::new("protected_resources", association.resource_id.clone())
            || locator.external_resource_id != exact.workspace_id.as_str()
            || locator.owner_account_id != RecordId::new("local_accounts", exact.owner_account_id.to_string())
            || locator.access_space_id != RecordId::new("access_spaces", exact.access_space_id.to_string())
        {
            return Err(ResourceAuthorityError::InvalidInput("navigation workspace authority mismatch"));
        }
        let bindings = RegistryPublicationBindings {
            resource: locator.id,
            original_grant: RecordId::new("resource_grants", association.grant_id.clone()),
            kind: ResourceKind::Workspace.as_str().to_owned(),
            action: "read".to_owned(), capability: "model_lane.navigation.read".to_owned(),
        };
        let rows = self.with_record_user_scope(scope, self.with_data_operation(move |database| {
            Box::pin(async move {
                database.query_values::<RegistryPublicationAuthority, _>(REGISTRY_PUBLICATION_AUTHORITY, bindings).await
            })
        })).await?;
        let [captured] = rows.as_slice() else {
            return Err(ResourceAuthorityError::InvalidInput("navigation original grant unavailable"));
        };
        if captured.account_id != RecordId::new("local_accounts", exact.owner_account_id.to_string())
            || captured.principal_id != RecordId::new("principals", exact.actor_principal_id.to_string())
            || captured.session_id != RecordId::new("authenticated_sessions", exact.authenticated_session_id.to_string())
            || captured.access_space_id != RecordId::new("access_spaces", exact.access_space_id.to_string())
            || captured.resource_id != RecordId::new("protected_resources", association.resource_id.clone())
            || captured.grant_id != RecordId::new("resource_grants", association.grant_id.clone())
            || captured.grant_version != association.grant_version
            || captured.policy_versions.first().copied() != Some(association.policy_version)
            || captured.resource_kind != "workspace"
            || !captured.statuses.iter().map(String::as_str).eq(["active", "active", "enabled", "enabled", "active"])
            || !captured.session_capabilities.iter().any(|capability| capability == "model_lane.navigation.read")
            || !captured.principal_capabilities.iter().any(|capability| capability == "model_lane.navigation.read")
            || !capability_registry.profile_can(&captured.capability_profile_id, "model_lane.navigation.read").unwrap_or(false)
        {
            return Err(ResourceAuthorityError::InvalidInput("navigation original authority changed"));
        }
        let authority = CanonicalNavigationPublicationAuthority {
            grant_expires_at: captured.grant_expires_at.clone(),
            session_expires_at: captured.session_expires_at.clone(),
        };
        Ok(publication(&authority))
    }

    /// Holds the publication result through exact authority checks and the
    /// existing authenticated transaction's observed commit.
    /// Restore only after terminal rejection/cancellation; unknown settlement keeps
    /// effects for canonical resolution. Later revocation cannot undo committed work.
    pub(crate) async fn with_registry_publication_authority<T, E, F, Fut>(
        &self, operation_id: Uuid, cancellation: std::sync::Arc<super::ContextOperationCancellation>, operation: F,
    ) -> Result<T, E>
    where
        T: Send + 'static,
        E: From<SurrealStorageError> + From<ResourceAuthorityError> + Send + 'static,
        F: FnOnce(serde_json::Value) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<T, E>> + Send + 'static,
    {
        let scope = super::current_record_user_scope()
            .ok_or_else(|| E::from(SurrealStorageError::TransactionAuthorityRequired))?;
        let workspace = scope.workspace_id.clone()
            .ok_or_else(|| E::from(SurrealStorageError::TransactionAuthorityRequired))?;
        if scope.capability_id != "fs.write"
            || !matches!(scope.action, ResourceAction::Create | ResourceAction::Update) {
            return Err(E::from(SurrealStorageError::TransactionAuthorityRequired));
        }
        let decision = self.authorize_protected_resource(AuthorizationRequest {
            session_token: scope.session_token.clone(),
            channel_binding_hash: scope.channel_binding_hash.clone(),
            capability_id: scope.capability_id.clone(), resource_kind: ResourceKind::Workspace,
            external_resource_id: workspace.clone(), action: scope.action,
        }).await.map_err(E::from)?;
        if Some(decision.grant_id.as_str()) != scope.grant_id.as_deref()
            || decision.resource_id != scope.resource_id || decision.session_id != scope.session_id
            || decision.capability_profile_id != "Operator" {
            return Err(E::from(SurrealStorageError::TransactionAuthorityRequired));
        }
        let bindings = RegistryPublicationBindings {
            resource: RecordId::new("protected_resources", decision.resource_id.clone()),
            original_grant: RecordId::new("resource_grants", decision.grant_id.clone()),
            kind: ResourceKind::Workspace.as_str().to_owned(),
            action: scope.action.as_str().to_owned(), capability: scope.capability_id,
        };
        let transaction_bindings = bindings;
        let account_authority = super::event_ledger::capture_account_authority();
        let receipt_storage = self.clone();
        let value = self.with_owned_authenticated_transaction(operation_id, cancellation, move |transaction| {
            Box::pin(async move {
                let before: Vec<RegistryPublicationAuthority> = transaction.query_values_at(
                    REGISTRY_PUBLICATION_AUTHORITY, transaction_bindings.clone(), 0).await.map_err(E::from)?;
                if before.len() != 1 {
                    return Err(E::from(SurrealStorageError::TransactionAuthorityRequired));
                }
                let captured = before.into_iter().next().ok_or_else(|| E::from(SurrealStorageError::TransactionAuthorityRequired))?;
                if captured.resource_id != transaction_bindings.resource
                    || captured.grant_id != transaction_bindings.original_grant
                    || captured.account_id != RecordId::new("local_accounts", decision.account_id)
                    || captured.principal_id != RecordId::new("principals", decision.principal_id)
                    || captured.session_id != RecordId::new("authenticated_sessions", decision.session_id)
                    || captured.access_space_id != RecordId::new("access_spaces", decision.access_space_id)
                    || captured.capability_profile_id != decision.capability_profile_id
                    || captured.delegation_chain != decision.delegation_chain {
                    return Err(E::from(SurrealStorageError::TransactionAuthorityRequired));
                }
                let envelope = serde_json::json!({
                    "owner_account_id": record_key(captured.account_id.clone()).map_err(E::from)?,
                    "actor_principal_id": record_key(captured.principal_id.clone()).map_err(E::from)?,
                    "actor_kind": captured.actor_kind,
                    "actor_id": captured.actor_id,
                    "authenticated_session_id": record_key(captured.session_id.clone()).map_err(E::from)?,
                    "access_space_id": record_key(captured.access_space_id.clone()).map_err(E::from)?,
                    "workspace_id": workspace,
                    "resource_id": record_key(captured.resource_id.clone()).map_err(E::from)?,
                    "grant_id": record_key(captured.grant_id.clone()).map_err(E::from)?,
                    "grant_version": captured.grant_version,
                    "policy_versions": captured.policy_versions,
                    "revocation_epochs": captured.revocation_epochs,
                    "delegation_chain": captured.delegation_chain,
                    "actions": captured.actions,
                    "capability_ids": captured.capability_ids,
                    "session_capabilities": captured.session_capabilities,
                    "principal_capabilities": captured.principal_capabilities,
                    "capability_profile_id": captured.capability_profile_id,
                    "grant_expires_at": captured.grant_expires_at.as_ref().map(|value| value.clone().into_inner().to_rfc3339()),
                    "session_expires_at": captured.session_expires_at.clone().into_inner().to_rfc3339(),
                    "capability_id": transaction_bindings.capability,
                    "action": transaction_bindings.action,
                });
                let value = super::event_ledger::with_captured_account_authority(
                    &receipt_storage, account_authority, operation(envelope)).await?;
                let after: Vec<RegistryPublicationAuthority> = transaction.query_values_at(
                    REGISTRY_PUBLICATION_AUTHORITY, transaction_bindings, 0).await.map_err(E::from)?;
                if after.as_slice() != std::slice::from_ref(&captured) {
                    return Err(E::from(SurrealStorageError::TransactionAuthorityRequired));
                }
                Ok(value)
            })
        }).await?;
        Ok(value)
    }

    pub async fn authorize_protected_resource(
        &self,
        request: AuthorizationRequest,
    ) -> Result<AuthorizationDecision, ResourceAuthorityError> {
        let decision_id = Uuid::now_v7().to_string();
        if request.session_token.is_empty()
            || request.capability_id.is_empty()
            || request.external_resource_id.is_empty()
        {
            handshake_storage_support::diagnostics::observe_result(
                "authority_audit",
                self.audit_authorization(&decision_id, &request, None, None, "deny"),
            )
            .await?;
            return Err(ResourceAuthorityError::Denied { decision_id });
        }

        let namespace = self.config().namespace().to_owned();
        let database = self.config().database().to_owned();
        let token_hash = sha256_hex(request.session_token.as_bytes());
        let channel_binding_hash = request.channel_binding_hash.clone();
        let capability = request.capability_id.clone();
        let kind = request.resource_kind.as_str().to_owned();
        let resource = self.canonical_resource_locator(request.resource_kind, &request.external_resource_id).await?.map(|row| row.id);
        let action = request.action.as_str().to_owned();

        let row = self
            .with_lease(move |client| {
                Box::pin(async move {
                    let authority = client.clone();
                    handshake_storage_support::diagnostics::observe_result("authority_namespace", authority
                        .use_ns(namespace.clone())
                        .use_db(database.clone()))
                        .await?;
                    if handshake_storage_support::diagnostics::observe_result("authority_signin", authority
                        .signin(RecordSignin {
                            namespace,
                            database: database.clone(),
                            access: AUTHORITY_ACCESS_METHOD.to_owned(),
                            params: SigninParams {
                                token_hash,
                                channel_binding_hash,
                            },
                        }))
                        .await
                        .is_err()
                    {
                        return Ok(AuthorizationAttempt {
                            session: None,
                            grant: None,
                        });
                    }
                    let mut session_response = handshake_storage_support::diagnostics::observe_result("authority_session_query", authority
                        .query(
                            "SELECT account_id, principal_id, id AS session_id, access_space_id, delegation_chain, policy_version FROM authenticated_sessions WHERE id = $auth.id LIMIT 1;",
                        ))
                        .await?
                        .check()?;
                    let session_rows: Vec<SessionAuthorityRow> = handshake_storage_support::diagnostics::observe_result("authority_response_decode", async { session_response.take(0) }).await?;
                    let session = session_rows.into_iter().next();
                    let mut response = handshake_storage_support::diagnostics::observe_result("authority_grant_query", authority
                        .query(
                            canonical_grant_query!("SELECT id AS grant_id, resource_id, account_id, principal_id, $auth.id AS session_id, access_space_id, principal_id.actor_kind AS actor_kind, principal_id.actor_id AS actor_id, principal_id.capability_profile_id AS capability_profile_id, $auth.delegation_chain AS delegation_chain, math::max([policy_version, resource_id.policy_version, account_id.policy_version, principal_id.policy_version, access_space_id.policy_version]) AS policy_version", "", "1"),
                        )
                        .bind(("resource", resource))
                        .bind(("kind", kind))
                        .bind(("action", action))
                        .bind(("capability", capability)))
                        .await?
                        .check()?;
                    let rows: Vec<AuthorizationRow> = handshake_storage_support::diagnostics::observe_result("authority_response_decode", async { response.take(0) }).await?;
                    Ok(AuthorizationAttempt {
                        session,
                        grant: rows.into_iter().next(),
                    })
                })
            })
            .await?;

        let Some(row) = row.grant else {
            handshake_storage_support::diagnostics::observe_result(
                "authority_audit",
                self.audit_authorization(
                    &decision_id,
                    &request,
                    None,
                    row.session.as_ref(),
                    "deny",
                ),
            )
            .await?;
            return Err(ResourceAuthorityError::Denied { decision_id });
        };
        let decision = AuthorizationDecision {
            decision_id: decision_id.clone(),
            account_id: record_key(row.account_id)?,
            principal_id: record_key(row.principal_id)?,
            session_id: record_key(row.session_id)?,
            access_space_id: record_key(row.access_space_id)?,
            resource_id: record_key(row.resource_id)?,
            actor_kind: row.actor_kind,
            actor_id: row.actor_id,
            capability_profile_id: row.capability_profile_id,
            delegation_chain: row.delegation_chain,
            policy_version: row.policy_version,
            grant_id: record_key(row.grant_id)?,
        };
        handshake_storage_support::diagnostics::observe_result(
            "authority_audit",
            self.audit_authorization(&decision_id, &request, Some(&decision), None, "allow"),
        )
        .await?;
        Ok(decision)
    }

    async fn audit_authorization(
        &self,
        decision_id: &str,
        request: &AuthorizationRequest,
        decision: Option<&AuthorizationDecision>,
        denied_session: Option<&SessionAuthorityRow>,
        result: &'static str,
    ) -> Result<(), ResourceAuthorityError> {
        let namespace = self.config().namespace().to_owned();
        let database = self.config().database().to_owned();
        let audit = RecordId::new("authorization_audit_events", Uuid::now_v7().to_string());
        let account = decision
            .map(|value| RecordId::new("local_accounts", value.account_id.clone()))
            .or_else(|| denied_session.map(|value| value.account_id.clone()));
        let principal = decision
            .map(|value| RecordId::new("principals", value.principal_id.clone()))
            .or_else(|| denied_session.map(|value| value.principal_id.clone()));
        let session = decision
            .map(|value| RecordId::new("authenticated_sessions", value.session_id.clone()))
            .or_else(|| denied_session.map(|value| value.session_id.clone()));
        let space = decision
            .map(|value| RecordId::new("access_spaces", value.access_space_id.clone()))
            .or_else(|| denied_session.map(|value| value.access_space_id.clone()));
        let resource =
            decision.map(|value| RecordId::new("protected_resources", value.resource_id.clone()));
        let delegation_chain = decision
            .map(|value| value.delegation_chain.clone())
            .or_else(|| denied_session.map(|value| value.delegation_chain.clone()))
            .unwrap_or_default();
        let policy_version = decision
            .map(|value| value.policy_version)
            .or_else(|| denied_session.map(|value| value.policy_version));
        let requested_resource_hash = sha256_hex(
            format!(
                "{}:{}",
                request.resource_kind.as_str(),
                request.external_resource_id
            )
            .as_bytes(),
        );
        let decision_id = decision_id.to_owned();
        let resource_kind = request.resource_kind.as_str().to_owned();
        let action = request.action.as_str().to_owned();
        let capability = request.capability_id.clone();

        self.with_lease(move |client| {
            Box::pin(async move {
                let authority = client.clone();
                handshake_storage_support::diagnostics::observe_result("authority_audit_namespace", authority
                    .use_ns(namespace)
                    .use_db(database.clone()))
                        .await?;
                handshake_storage_support::diagnostics::observe_result("authority_audit_query", authority
                    .query(
                        "CREATE $audit SET decision_id = $decision_id, account_id = $account, principal_id = $principal, session_id = $session_record, access_space_id = $space, delegation_chain = $delegation_chain, resource_id = $resource, requested_resource_hash = $requested_resource_hash, resource_kind = $resource_kind, action = $action, capability_id = $capability, result = $result, policy_version = $policy_version, occurred_at = time::now();",
                    )
                    .bind(("audit", audit))
                    .bind(("decision_id", decision_id))
                    .bind(("account", account))
                    .bind(("principal", principal))
                    .bind(("session_record", session))
                    .bind(("space", space))
                    .bind(("delegation_chain", delegation_chain))
                    .bind(("resource", resource))
                    .bind(("requested_resource_hash", requested_resource_hash))
                    .bind(("resource_kind", resource_kind))
                    .bind(("action", action))
                    .bind(("capability", capability))
                    .bind(("result", result))
                    .bind(("policy_version", policy_version)))
                        .await?
                    .check()?;
                Ok(())
            })
        })
        .await?;
        Ok(())
    }

    pub async fn revoke_session(&self, session_id: &str) -> Result<(), ResourceAuthorityError> {
        validate_uuid(session_id)?;
        let session_id = session_id.to_owned();
        self.with_owned_authority_mutation(move |storage| async move {
            storage.revoke_session_unfenced(&session_id).await
        }).await?
    }

    async fn revoke_session_unfenced(&self, session_id: &str) -> Result<(), ResourceAuthorityError> {
        self.update_authority_record(
            "authenticated_sessions",
            session_id,
            "UPDATE $record SET revoked_at = time::now();",
        )
        .await?;
        super::authority_mutation_submitted();
        let result = self.resource_access_lifecycle().invalidate_session(session_id)
            .map_err(|_| ResourceAuthorityError::InvalidInput("lifecycle authority unavailable"));
        if result.is_ok() { super::authority_mutation_settled(); }
        result
    }

    pub async fn revoke_grant(&self, grant_id: &str) -> Result<(), ResourceAuthorityError> {
        validate_uuid(grant_id)?;
        let grant_id = grant_id.to_owned();
        self.with_owned_authority_mutation(move |storage| async move {
            storage.revoke_grant_unfenced(&grant_id).await
        }).await?
    }

    async fn revoke_grant_unfenced(&self, grant_id: &str) -> Result<(), ResourceAuthorityError> {
        self.update_authority_record(
            "resource_grants",
            grant_id,
            "UPDATE $record SET status = 'revoked', revoked_at = time::now(), grant_version += 1, policy_version += 1, updated_at = time::now();",
        )
        .await?;
        super::authority_mutation_submitted();
        let result = self.resource_access_lifecycle().invalidate_grant(grant_id)
            .map_err(|_| ResourceAuthorityError::InvalidInput("lifecycle authority unavailable"));
        if result.is_ok() { super::authority_mutation_settled(); }
        result
    }

    pub async fn disable_account(&self, account_id: &str) -> Result<(), ResourceAuthorityError> {
        validate_uuid(account_id)?;
        let account_id = account_id.to_owned();
        self.with_owned_authority_mutation(move |storage| async move {
            storage.disable_account_unfenced(&account_id).await
        }).await?
    }

    async fn disable_account_unfenced(&self, account_id: &str) -> Result<(), ResourceAuthorityError> {
        self.update_authority_record(
            "local_accounts",
            account_id,
            "UPDATE $record SET status = 'disabled', revocation_epoch += 1, policy_version += 1, updated_at = time::now();",
        )
        .await?;
        super::authority_mutation_submitted();
        let result = self.resource_access_lifecycle().invalidate_account(account_id)
            .map_err(|_| ResourceAuthorityError::InvalidInput("lifecycle authority unavailable"));
        if result.is_ok() { super::authority_mutation_settled(); }
        result
    }

    pub async fn switch_session_access_space(
        &self,
        session_id: &str,
        new_access_space_id: &str,
    ) -> Result<(), ResourceAuthorityError> {
        validate_uuid(session_id)?;
        validate_uuid(new_access_space_id)?;
        let session_id = session_id.to_owned();
        let new_access_space_id = new_access_space_id.to_owned();
        self.with_owned_authority_mutation(move |storage| async move {
            storage.switch_session_access_space_unfenced(&session_id, &new_access_space_id).await
        }).await?
    }

    async fn switch_session_access_space_unfenced(
        &self,
        session_id: &str,
        new_access_space_id: &str,
    ) -> Result<(), ResourceAuthorityError> {
        validate_uuid(session_id)?;
        validate_uuid(new_access_space_id)?;
        let namespace = self.config().namespace().to_owned();
        let database = self.config().database().to_owned();
        let session = RecordId::new("authenticated_sessions", session_id.to_owned());
        let space = RecordId::new("access_spaces", new_access_space_id.to_owned());
        self.with_lease(move |client| {
            Box::pin(async move {
                let authority = client.clone();
                authority
                    .use_ns(namespace)
                    .use_db(database.clone())
                    .await?;
                let mut session_response = authority
                    .query("SELECT account_id FROM $session_record LIMIT 2;")
                    .bind(("session_record", session.clone()))
                    .await?
                    .check()?;
                let session_rows: Vec<SessionAccessSpaceRow> = session_response.take(0)?;
                let session_row = match session_rows.as_slice() {
                    [row] => row,
                    _ => {
                        return Err(surrealdb::Error::internal(
                            "HSK-AUTH-SPACE-SWITCH-DENIED".to_owned(),
                        )
                        .into())
                    }
                };
                let mut space_response = authority
                    .query(
                        "SELECT account_id, status, revocation_epoch, policy_version FROM $space LIMIT 2;",
                    )
                    .bind(("space", space.clone()))
                    .await?
                    .check()?;
                let space_rows: Vec<AccessSpaceSwitchRow> = space_response.take(0)?;
                let space_row = match space_rows.as_slice() {
                    [row]
                        if row.account_id == session_row.account_id && row.status == "active" =>
                    {
                        row
                    }
                    _ => {
                        return Err(surrealdb::Error::internal(
                            "HSK-AUTH-SPACE-SWITCH-DENIED".to_owned(),
                        )
                        .into())
                    }
                };
                super::authority_mutation_submitted();
                authority
                    .query(
                        "UPDATE $session_record SET access_space_id = $space, space_revocation_epoch = $space_epoch, policy_version = math::max([policy_version + 1, $space_policy_version]);",
                    )
                    .bind(("session_record", session))
                    .bind(("space", space))
                    .bind(("space_epoch", space_row.revocation_epoch))
                    .bind(("space_policy_version", space_row.policy_version))
                    .await?
                    .check().map_err(super::authority_mutation_error)?;
                Ok(())
            })
        })
        .await?;
        super::authority_mutation_submitted();
        self.resource_access_lifecycle().invalidate_session(session_id)
            .map_err(|_| ResourceAuthorityError::InvalidInput("lifecycle authority unavailable"))?;
        super::authority_mutation_settled();
        Ok(())
    }

    pub async fn list_registered_workspace_resource_ids(
        &self,
    ) -> Result<Vec<String>, ResourceAuthorityError> {
        let namespace = self.config().namespace().to_owned();
        let database = self.config().database().to_owned();
        let rows = self
            .with_lease(move |client| {
                Box::pin(async move {
                    let authority = client.clone();
                    authority
                        .use_ns(namespace)
                        .use_db(database.clone())
                        .await?;
                    let mut response = authority
                        .query(
                            "SELECT id, external_resource_id FROM protected_resources WHERE resource_kind = 'workspace' AND lifecycle_state = 'active' ORDER BY external_resource_id ASC;",
                        )
                        .await?
                        .check()?;
                    let rows: Vec<ResourceLookupRow> = response.take(0)?;
                    Ok(rows)
                })
            })
            .await?;
        Ok(rows
            .into_iter()
            .map(|row| row.external_resource_id)
            .collect())
    }

    pub async fn provision_reconciliation_principal(
        &self,
        workspace_external_ids: &[String],
        channel_binding_hash: Option<&str>,
    ) -> Result<ReconciliationPrincipal, ResourceAuthorityError> {
        let capabilities = vec![
            "fr.ingest.native_editor".to_owned(),
            "memory.commit".to_owned(),
        ];
        let provisioned = self
            .provision_principal(
                "mt109-reconciliation-service-account",
                "mt109-reconciliation-service-principal",
                "service_identity",
                "mt109_reconciler",
                RECONCILIATION_PROFILE_ID,
                &capabilities,
                "mt109-reconciliation-space",
                channel_binding_hash,
                Duration::from_secs(60 * 60),
            )
            .await?;
        let queue = self
            .register_protected_resource(
                &provisioned.identity,
                ResourceKind::ReconciliationQueue,
                RECONCILIATION_QUEUE_ID,
                None,
                "restricted",
            )
            .await?;
        self.ensure_reconciliation_queue_grant(
            &provisioned.identity,
            &queue.resource_id,
            &capabilities,
        )
        .await?;

        for workspace_id in workspace_external_ids {
            let workspace_queue = self
                .register_protected_resource(
                    &provisioned.identity,
                    ResourceKind::ReconciliationQueue,
                    &format!("{RECONCILIATION_QUEUE_ID}:{workspace_id}"),
                    Some(&queue.resource_id),
                    "restricted",
                )
                .await?;
            self.ensure_reconciliation_queue_grant(
                &provisioned.identity,
                &workspace_queue.resource_id,
                &capabilities,
            )
            .await?;
        }
        Ok(provisioned)
    }

    pub async fn reconciliation_principal_is_provisioned(
        &self,
    ) -> Result<bool, ResourceAuthorityError> {
        let namespace = self.config().namespace().to_owned();
        let database = self.config().database().to_owned();
        self.with_lease(move |client| {
            Box::pin(async move {
                let authority = client.clone();
                authority.use_ns(namespace).use_db(database).await?;
                let mut response = authority
                    .query(
                        "LET $principal = (SELECT * FROM ONLY principals WHERE principal_key = 'mt109-reconciliation-service-principal' AND principal_kind = 'service_identity' AND status = 'enabled' LIMIT 1); LET $space = (SELECT * FROM ONLY access_spaces WHERE account_id = $principal.account_id AND space_key = 'mt109-reconciliation-space' AND status = 'active' LIMIT 1); LET $root = (SELECT * FROM ONLY protected_resources WHERE resource_kind = 'reconciliation_queue' AND external_resource_id = 'mt109-protected-reconciliation' AND lifecycle_state = 'active' AND owner_account_id = $principal.account_id AND access_space_id = $space.id LIMIT 1); RETURN $principal != NONE AND $space != NONE AND $root != NONE AND array::len(SELECT VALUE id FROM resource_grants WHERE resource_id = $root.id AND account_id = $principal.account_id AND principal_id = $principal.id AND access_space_id = $space.id AND actions = ['reconcile'] AND capability_ids = ['fr.ingest.native_editor','memory.commit'] AND delegation_chain = [record::id($principal.id)] AND status = 'active' AND revoked_at = NONE AND expires_at = NONE LIMIT 1) = 1;",
                    )
                    .await?
                    .check()?;
                Ok(response.take::<Option<bool>>(3)?.unwrap_or(false))
            })
        })
        .await
        .map_err(ResourceAuthorityError::from)
    }

    async fn ensure_reconciliation_queue_grant(
        &self,
        identity: &ProvisionedIdentity,
        resource_id: &str,
        capabilities: &[String],
    ) -> Result<(), ResourceAuthorityError> {
        let namespace = self.config().namespace().to_owned();
        let database = self.config().database().to_owned();
        let account = RecordId::new("local_accounts", identity.account_id.clone());
        let principal = RecordId::new("principals", identity.principal_id.clone());
        let space = RecordId::new("access_spaces", identity.access_space_id.clone());
        let resource = RecordId::new("protected_resources", resource_id.to_owned());
        let grant = RecordId::new("resource_grants", Uuid::now_v7().to_string());
        let capabilities = capabilities.to_vec();
        let delegation_chain = vec![identity.principal_id.clone()];
        self.with_lease(move |client| {
            Box::pin(async move {
                let authority = client.clone();
                authority.use_ns(namespace).use_db(database).await?;
                authority
                    .query(
                        "LET $revoked = SELECT VALUE id FROM resource_grants WHERE account_id = $account AND principal_id = $principal AND access_space_id = $space AND resource_id = $resource AND actions = ['reconcile'] AND capability_ids = $capabilities AND delegation_chain = $delegation_chain AND revoked_at != NONE LIMIT 1; IF array::len($revoked) != 0 { THROW 'HSK-403-PROTECTED-RESOURCE'; }; LET $existing = SELECT VALUE id FROM resource_grants WHERE account_id = $account AND principal_id = $principal AND access_space_id = $space AND resource_id = $resource AND actions = ['reconcile'] AND capability_ids = $capabilities AND delegation_chain = $delegation_chain AND status = 'active' AND revoked_at = NONE AND expires_at = NONE LIMIT 1; IF array::len($existing) = 0 { CREATE $grant SET account_id = $account, principal_id = $principal, access_space_id = $space, resource_id = $resource, actions = ['reconcile'], capability_ids = $capabilities, delegation_chain = $delegation_chain, status = 'active', grant_version = 1, policy_version = math::max([$account.policy_version, $principal.policy_version, $space.policy_version, $resource.policy_version]), expires_at = NONE, revoked_at = NONE, created_at = time::now(), updated_at = time::now(); };",
                    )
                    .bind(("account", account))
                    .bind(("principal", principal))
                    .bind(("space", space))
                    .bind(("resource", resource))
                    .bind(("grant", grant))
                    .bind(("capabilities", capabilities))
                    .bind(("delegation_chain", delegation_chain))
                    .await?
                    .check()?;
                Ok(())
            })
        })
        .await
        .map_err(ResourceAuthorityError::from)
    }

    pub async fn issue_reconciliation_session(
        &self,
    ) -> Result<ReconciliationPrincipal, ResourceAuthorityError> {
        let namespace = self.config().namespace().to_owned();
        let database = self.config().database().to_owned();
        let row = self
            .with_lease(move |client| {
                Box::pin(async move {
                    let authority = client.clone();
                    authority
                        .use_ns(namespace)
                        .use_db(database)
                        .await?;
                    let mut response = authority
                        .query(
                            "SELECT account_id, id AS principal_id, (SELECT VALUE id FROM access_spaces WHERE account_id = $parent.account_id AND status = 'active' ORDER BY created_at ASC LIMIT 1)[0] AS access_space_id FROM principals WHERE principal_key = 'mt109-reconciliation-service-principal' AND principal_kind = 'service_identity' AND status = 'enabled' LIMIT 1;",
                        )
                        .await?
                        .check()?;
                    let rows: Vec<PrincipalLookupRow> = response.take(0)?;
                    rows.into_iter().next().ok_or_else(|| {
                        surrealdb::Error::internal(
                            "reconciliation Principal is not explicitly provisioned".to_owned(),
                        )
                        .into()
                    })
                })
            })
            .await?;
        let identity = ProvisionedIdentity {
            account_id: record_key(row.account_id)?,
            principal_id: record_key(row.principal_id)?,
            access_space_id: record_key(row.access_space_id)?,
        };
        let session = self
            .issue_authenticated_session(
                &identity.account_id,
                &identity.principal_id,
                &identity.access_space_id,
                None,
                Duration::from_secs(60 * 60),
            )
            .await?;
        Ok(ProvisionedPrincipal { identity, session })
    }

    async fn lookup_registered_resource(
        &self,
        kind: ResourceKind,
        external_resource_id: &str,
    ) -> Result<RegisteredResource, ResourceAuthorityError> {
        validate_nonempty(external_resource_id)?;
        let namespace = self.config().namespace().to_owned();
        let database = self.config().database().to_owned();
        let kind_name = kind.as_str().to_owned();
        let external = external_resource_id.to_owned();
        let row = self
            .with_lease(move |client| {
                Box::pin(async move {
                    let authority = client.clone();
                    authority
                        .use_ns(namespace)
                        .use_db(database.clone())
                        .await?;
                    let mut response = authority
                        .query(
                            "SELECT id, external_resource_id FROM protected_resources WHERE resource_kind = $kind AND external_resource_id = $external AND lifecycle_state = 'active' LIMIT 1;",
                        )
                        .bind(("kind", kind_name))
                        .bind(("external", external))
                        .await?
                        .check()?;
                    let rows: Vec<ResourceLookupRow> = response.take(0)?;
                    Ok(rows.into_iter().next())
                })
            })
            .await?
            .ok_or(ResourceAuthorityError::InvalidInput(
                "protected resource is not registered",
            ))?;
        Ok(RegisteredResource {
            resource_id: record_key(row.id)?,
            resource_kind: kind,
            external_resource_id: row.external_resource_id,
        })
    }

    async fn update_authority_record(
        &self,
        table: &'static str,
        id: &str,
        statement: &'static str,
    ) -> Result<(), ResourceAuthorityError> {
        validate_uuid(id)?;
        let namespace = self.config().namespace().to_owned();
        let database = self.config().database().to_owned();
        let record = RecordId::new(table, id.to_owned());
        self.with_lease(move |client| {
            Box::pin(async move {
                let authority = client.clone();
                authority.use_ns(namespace).use_db(database.clone()).await?;
                super::authority_mutation_submitted();
                authority
                    .query(statement)
                    .bind(("record", record))
                    .await?
                    .check().map_err(super::authority_mutation_error)?;
                Ok(())
            })
        })
        .await?;
        Ok(())
    }
}

fn validate_nonempty(value: &str) -> Result<(), ResourceAuthorityError> {
    if value.trim().is_empty() {
        Err(ResourceAuthorityError::InvalidInput(
            "required string must not be blank",
        ))
    } else {
        Ok(())
    }
}

fn validate_uuid(value: &str) -> Result<(), ResourceAuthorityError> {
    Uuid::parse_str(value)
        .map(|_| ())
        .map_err(|_| ResourceAuthorityError::InvalidInput("identifier must be a UUID"))
}

fn record_key(record: RecordId) -> Result<String, ResourceAuthorityError> {
    match record.key {
        RecordIdKey::String(value) => Ok(value),
        _ => Err(ResourceAuthorityError::InvalidInput(
            "authority record identifier is not a string UUID",
        )),
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

#[cfg(all(test, feature = "runtime-full", feature = "os-keychain"))]
mod producer_authority_tests;
