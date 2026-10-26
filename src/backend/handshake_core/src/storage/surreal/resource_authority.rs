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
use super::local_accounts::ConfirmedFirstOwnerAdministrationRegistration;

mod artifact_publications;
mod validator_sessions;
pub(crate) use validator_sessions::{
    ValidatorDelegationParent,
    ValidatorAssociationIssuanceInput,
    ValidatorAssociationFault,
    ValidatorAssociationTransactionRetention,
    ValidatorAssociationEngineObservation,
    ValidatorAssociationWorkerPhase,
    ValidatorAssociationIssuanceHandle,
    ValidatorAssociationRetainedFailure,
    ValidatorAssociationReserveRejection,
    CurrentValidatorAssociationSeal,
    ValidatorAssociationReadbackGuard,
    RetainedValidatorAssociationQuery,
    RetainedValidatorDesignationQuery,
};
pub(crate) use artifact_publications::RegisteredArtifactWitness;

// One owning canonical grant predicate shared by public authorization and the
// private publication fence; the latter uses a canonical located resource ID.
// Resource table permission already enforces its creator; ordinary queries never
// re-read the hidden external locator through mt109_has_grant.
macro_rules! canonical_grant_query {
    ($projection:literal, $extra:literal, $limit:literal) => {
        concat!($projection, " FROM (SELECT * FROM resource_grants WHERE account_id = $auth.account_id AND principal_id = $auth.principal_id AND access_space_id = $auth.access_space_id AND resource_id = $resource) WHERE status = 'active' AND revoked_at = NONE AND (expires_at = NONE OR expires_at > time::now()) AND resource_id.lifecycle_state = 'active' AND resource_id.owner_account_id = $auth.account_id AND resource_id.access_space_id = $auth.access_space_id AND actions CONTAINS $action AND capability_ids CONTAINS $capability AND ($auth.delegated_capabilities CONTAINS '*' OR $auth.delegated_capabilities CONTAINS $capability) AND delegation_chain = $auth.delegation_chain AND ($kind != 'artifact' OR (resource_id.resource_kind = 'artifact' AND resource_id.parent_resource_id != NONE AND resource_id.parent_resource_id.resource_kind = 'workspace' AND resource_id.parent_resource_id.lifecycle_state = 'active' AND resource_id.parent_resource_id.owner_account_id = $auth.account_id AND resource_id.parent_resource_id.access_space_id = $auth.access_space_id AND resource_id.parent_resource_id.policy_version <= policy_version AND resource_id.policy_version <= policy_version AND policy_version <= $auth.policy_version)) AND ($kind != 'reconciliation_queue' OR (principal_id.principal_kind = 'service_identity' AND principal_id.capability_profile_id = 'MT109Reconciler'))", $extra, " LIMIT ", $limit, ";")
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
    administration_lifetime: Option<AdministrationLifetimeRow>,
}

#[derive(SurrealValue)]
struct OwnerEligibilityBindings {
    session_record: RecordId,
}

const OWNER_ELIGIBILITY: &str = "SELECT VALUE id FROM authenticated_sessions WHERE id = $session_record AND id = $auth.id AND fn::mt120_workspace_create(id) LIMIT 1;";

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
    "SELECT id AS grant_id, resource_id, account_id, principal_id, principal_id.actor_kind AS actor_kind, principal_id.actor_id AS actor_id, $auth.id AS session_id, access_space_id, grant_version, [policy_version, resource_id.policy_version, account_id.policy_version, principal_id.policy_version, access_space_id.policy_version, $auth.id.policy_version] AS policy_versions, [account_id.revocation_epoch, principal_id.revocation_epoch, access_space_id.revocation_epoch, $auth.id.account_revocation_epoch, $auth.id.principal_revocation_epoch, $auth.id.space_revocation_epoch] AS revocation_epochs, actions, capability_ids, delegation_chain, $auth.id.delegated_capabilities AS session_capabilities, principal_id.delegated_capabilities AS principal_capabilities, principal_id.capability_profile_id AS capability_profile_id, [status, resource_id.lifecycle_state, account_id.status, principal_id.status, access_space_id.status] AS statuses, resource_id.resource_kind AS resource_kind, expires_at AS grant_expires_at, $auth.id.expires_at AS session_expires_at, resource_id.account_administration_lifetime AS administration_lifetime",
    " AND id = $original_grant AND resource_id != NONE AND resource_id.resource_kind = $kind AND resource_id.policy_version <= policy_version AND policy_version <= $auth.policy_version AND ($auth.principal_id.delegated_capabilities CONTAINS '*' OR $auth.principal_id.delegated_capabilities CONTAINS $capability) AND $auth.id.revoked_at = NONE AND $auth.id.expires_at > time::now()",
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

#[derive(Clone, SurrealValue)]
struct RoutingOriginalBinding {
    operation_id: String,
    run_id: String,
    execution_id: String,
    selecting_decision_id: Option<String>,
    input_ref: Option<String>,
    action: String,
    capability_id: String,
    original: RegistryPublicationAuthority,
    source_create: Option<RegistryPublicationAuthority>,
    source_read: Option<RegistryPublicationAuthority>,
    source_update: Option<RegistryPublicationAuthority>,
}

#[derive(Clone)]
pub(crate) struct RoutingOriginalSources {
    capture: std::sync::Arc<ProducerSourceFactsCapture>,
    storage: SurrealStorage,
    registry: std::sync::Arc<crate::capabilities::CapabilityRegistry>,
    original: RecordUserScope,
    run_id: String,
    execution_id: String,
    selecting_decision_id: Option<String>,
    input_ref: Option<String>,
    create_index: Option<usize>,
    additional_create: std::sync::Arc<tokio::sync::OnceCell<std::sync::Arc<ProducerSourceFactsCapture>>>,
}

tokio::task_local! {
    static ROUTING_ORIGINAL_SOURCES: Option<std::sync::Arc<RoutingOriginalSources>>;
}

pub(crate) fn current_routing_original_sources() -> Option<std::sync::Arc<RoutingOriginalSources>> {
    ROUTING_ORIGINAL_SOURCES.try_with(Clone::clone).ok().flatten()
}

pub(crate) async fn with_routing_original_sources<T>(
    sources: std::sync::Arc<RoutingOriginalSources>, operation: impl std::future::Future<Output = T>,
) -> T {
    with_captured_routing_original_sources(Some(sources), operation).await
}

pub(crate) async fn with_captured_routing_original_sources<T>(
    sources: Option<std::sync::Arc<RoutingOriginalSources>>, operation: impl std::future::Future<Output = T>,
) -> T {
    ROUTING_ORIGINAL_SOURCES.scope(sources, operation).await
}

async fn with_role_routing_context<T>(
    sources: Option<std::sync::Arc<RoutingOriginalSources>>, operation: impl std::future::Future<Output = T>,
) -> T {
    if let Some(sources) = sources {
        super::with_configured_capability_registry(sources.registry(),
            with_routing_original_sources(sources,
                super::with_model_lane_operation(super::ContextCoordinationPurpose::RoutingTransition, operation))).await
    } else { operation.await }
}

impl RoutingOriginalSources {
    pub(crate) fn operation_id(&self) -> Uuid { self.capture.operation_id }
    pub(crate) fn run_id(&self) -> &str { &self.run_id }
    pub(crate) fn execution_id(&self) -> &str { &self.execution_id }
    pub(crate) fn selecting_decision_id(&self) -> Option<&str> { self.selecting_decision_id.as_deref() }
    pub(crate) fn action(&self) -> ResourceAction { self.original.action }
    pub(crate) fn registry(&self) -> std::sync::Arc<crate::capabilities::CapabilityRegistry> {
        self.registry.clone()
    }
    pub(crate) async fn revalidate(&self) -> Result<(), ResourceAuthorityError> {
        let storage = &self.storage;
        storage.revalidate_producer_source_facts(&self.capture).await?;
        if self.capture.sources.iter().any(|source| !self.registry.profile_can(
            &source.decision.capability_profile_id, &source.request.capability_id,
        ).unwrap_or(false)) {
            return Err(ResourceAuthorityError::InvalidInput("routing configured profile changed"));
        }
        if let Some(create) = self.additional_create.get() {
            storage.revalidate_producer_source_facts(create).await?;
            if create.sources.iter().any(|source| !self.registry.profile_can(
                &source.decision.capability_profile_id, &source.request.capability_id,
            ).unwrap_or(false)) { return Err(ResourceAuthorityError::InvalidInput("routing Create profile changed")); }
        }
        let source = &self.capture.sources[0];
        if source.decision.grant_id.as_str() != self.original.grant_id.as_deref().unwrap_or("")
            || source.decision.resource_id != self.original.resource_id
            || source.decision.session_id != self.original.session_id
            || self.capture.lifecycle != storage.resource_access_lifecycle()
        { return Err(ResourceAuthorityError::InvalidInput("routing original authority changed")); }
        Ok(())
    }
    pub(crate) fn with_selected_run(&self, run_id: String) -> Result<std::sync::Arc<Self>, ResourceAuthorityError> {
        if run_id.trim().is_empty() || (!self.run_id.is_empty() && self.run_id != run_id) {
            return Err(ResourceAuthorityError::InvalidInput("routing selected run changed"));
        }
        let mut selected = self.clone(); selected.run_id = run_id;
        Ok(std::sync::Arc::new(selected))
    }
    pub(crate) fn with_execution_origin(
        &self, run_id: String, decision_id: String, input_ref: Option<String>,
    ) -> Result<std::sync::Arc<Self>, ResourceAuthorityError> {
        if decision_id.trim().is_empty()
            || self.selecting_decision_id.as_ref().is_some_and(|prior| prior != &decision_id)
            || self.input_ref.as_ref().is_some_and(|prior| input_ref.as_ref() != Some(prior)) {
            return Err(ResourceAuthorityError::InvalidInput("routing canonical origin changed"));
        }
        let mut selected = self.with_selected_run(run_id)?.as_ref().clone();
        selected.selecting_decision_id = Some(decision_id); selected.input_ref = input_ref;
        Ok(std::sync::Arc::new(selected))
    }
    pub(crate) async fn authorize_source_creation(&self) -> Result<(), ResourceAuthorityError> {
        self.revalidate().await?;
        if self.create_index.is_some() { return Ok(()); }
        self.additional_create.get_or_try_init(|| async {
            let mut request = self.capture.sources[0].request.clone();
            request.action = ResourceAction::Create;
            let source = self.storage.read_producer_source_facts(request, &self.capture.session, &self.registry).await?;
            let original = &self.capture.sources[0].decision;
            if source.decision.account_id != original.account_id || source.decision.principal_id != original.principal_id
                || source.decision.session_id != original.session_id || source.decision.access_space_id != original.access_space_id
                || source.decision.resource_id != original.resource_id {
                return Err(ResourceAuthorityError::InvalidInput("routing Create source differs from original tuple"));
            }
            let capture = std::sync::Arc::new(ProducerSourceFactsCapture {
                operation_id: self.operation_id(), credentials: self.capture.credentials.clone(),
                session: self.capture.session.clone(), registry: self.registry.as_ref().clone(),
                lifecycle: self.capture.lifecycle.clone(), sources: vec![source], fence: self.capture.fence.clone(),
            });
            self.storage.revalidate_producer_source_facts(&capture).await?;
            Ok::<_, ResourceAuthorityError>(capture)
        }).await?;
        Ok(())
    }
    pub(crate) fn for_owned_operation(&self) -> std::sync::Arc<Self> {
        let mut sources = self.clone();
        sources.capture = std::sync::Arc::new(ProducerSourceFactsCapture {
            operation_id: Uuid::now_v7(), credentials: self.capture.credentials.clone(), session: self.capture.session.clone(),
            registry: self.capture.registry.clone(), lifecycle: self.capture.lifecycle.clone(), fence: self.capture.fence.clone(),
            sources: self.capture.sources.iter().map(|source| ProducerSourceFacts {
                request: source.request.clone(), decision: source.decision.clone(), authority: source.authority.clone(),
            }).collect(),
        });
        std::sync::Arc::new(sources)
    }

    pub(crate) async fn transition_sources(&self) -> Result<std::sync::Arc<Self>, ResourceAuthorityError> {
        self.revalidate().await?;
        if self.original.action != ResourceAction::Create || self.execution_id.is_empty() {
            return Err(ResourceAuthorityError::InvalidInput("combined lifecycle original Create or execution absent"));
        }
        let update = self.capture.sources.iter().find(|source| source.request.action == ResourceAction::Update
            && source.request.capability_id == "fs.write")
            .ok_or(ResourceAuthorityError::InvalidInput("separate lifecycle Update authority absent"))?;
        let mut selected = self.clone();
        let mut facts = vec![ProducerSourceFacts {
            request: update.request.clone(), decision: update.decision.clone(), authority: update.authority.clone(),
        }];
        facts.extend(self.capture.sources.iter().filter(|source| source.request.action != ResourceAction::Update)
            .map(|source| ProducerSourceFacts { request: source.request.clone(), decision: source.decision.clone(), authority: source.authority.clone() }));
        selected.capture = std::sync::Arc::new(ProducerSourceFactsCapture {
            operation_id: Uuid::now_v7(), credentials: self.capture.credentials.clone(), session: self.capture.session.clone(),
            registry: self.capture.registry.clone(), lifecycle: self.capture.lifecycle.clone(), fence: self.capture.fence.clone(), sources: facts,
        });
        selected.original = RecordUserScope {
            grant_id: Some(update.decision.grant_id.clone()), workspace_id: Some(update.request.external_resource_id.clone()),
            session_token: update.request.session_token.clone(), channel_binding_hash: update.request.channel_binding_hash.clone(),
            resource_id: update.decision.resource_id.clone(), session_id: update.decision.session_id.clone(),
            capability_id: update.request.capability_id.clone(), action: update.request.action,
        };
        selected.create_index = selected.capture.sources.iter().position(|source| source.request.action == ResourceAction::Create);
        let selected = std::sync::Arc::new(selected); selected.revalidate().await?; Ok(selected)
    }

    pub(crate) fn original_scope(&self) -> RecordUserScope { self.original.clone() }

    pub(crate) fn create_scope(&self) -> Result<RecordUserScope, ResourceAuthorityError> {
        let source = self.create_index.and_then(|index| self.capture.sources.get(index))
            .or_else(|| self.additional_create.get().and_then(|capture| capture.sources.first()))
            .ok_or(ResourceAuthorityError::InvalidInput("routing source creation authority absent"))?;
        if source.request.action != ResourceAction::Create || source.request.capability_id != "fs.write" {
            return Err(ResourceAuthorityError::InvalidInput("routing source creation authority differs"));
        }
        Ok(RecordUserScope {
            grant_id: Some(source.decision.grant_id.clone()), workspace_id: Some(source.request.external_resource_id.clone()),
            session_token: source.request.session_token.clone(), channel_binding_hash: source.request.channel_binding_hash.clone(),
            resource_id: source.decision.resource_id.clone(), session_id: source.decision.session_id.clone(),
            capability_id: source.request.capability_id.clone(), action: source.request.action,
        })
    }
    fn binding(&self) -> RoutingOriginalBinding {
        RoutingOriginalBinding {
            operation_id: self.operation_id().to_string(), run_id: self.run_id.clone(), execution_id: self.execution_id.clone(),
            selecting_decision_id: self.selecting_decision_id.clone(), input_ref: self.input_ref.clone(),
            action: self.original.action.as_str().to_owned(), capability_id: self.original.capability_id.clone(),
            original: self.capture.sources[0].authority.clone(),
            source_create: self.create_index.map(|index| self.capture.sources[index].authority.clone())
                .or_else(|| self.additional_create.get().and_then(|capture| capture.sources.first()).map(|source| source.authority.clone())),
            source_read: self.capture.sources.iter().find(|source| source.request.action == ResourceAction::Read
                && source.request.capability_id == "fs.read").map(|source| source.authority.clone()),
            source_update: self.capture.sources.iter().find(|source| source.request.action == ResourceAction::Update
                && source.request.capability_id == "fs.write").map(|source| source.authority.clone()),
        }
    }
}


const PALMISTRY_WATCHER_ADOPT_CAPABILITY: &str = "diagnostics.watcher.adopt_existing";

/// Current adoption authority only. Original watcher custody is verified by its registration owner.
pub struct PalmistryWatcherAdoptionAuthority {
    capture: ProducerSourceFactsCapture,
    watcher_id: Uuid,
    parent: CanonicalResourceLocator,
    exact: crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution,
    namespace: String,
    database: String,
}

#[derive(SurrealValue)]
struct NavigationArtifactResourceBindings {
    artifact: RecordId,
    workspace: RecordId,
}

#[derive(SurrealValue)]
struct NavigationArtifactResourceRow {
    id: RecordId,
    resource_kind: String,
    external_resource_id: String,
    owner_account_id: RecordId,
    access_space_id: RecordId,
    parent_resource_id: Option<RecordId>,
    lifecycle_state: String,
    policy_version: i64,
    classification: String,
}

pub(crate) struct NavigationArtifactReadPermit<'tx> {
    transaction: &'tx super::SurrealTransactionContext<'tx>,
    original_bindings: super::model_lane::NavigationReadBindings,
    exact_scope: crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution,
    selection: serde_json::Value,
    source: serde_json::Value,
    snapshot: serde_json::Value,
    managed_root: std::path::PathBuf,
}

pub(crate) struct NavigationArtifactRegistration {
    manifest: crate::storage::artifacts::ArtifactManifest,
    snapshot: serde_json::Value,
    managed_root: std::path::PathBuf,
}

impl NavigationArtifactRegistration {
    pub(crate) fn manifest(&self) -> &crate::storage::artifacts::ArtifactManifest { &self.manifest }
    pub(crate) fn snapshot(&self) -> &serde_json::Value { &self.snapshot }
    pub(crate) fn managed_root(&self) -> &std::path::Path { &self.managed_root }
    pub(crate) fn same_registration(&self, other: &Self) -> bool {
        self.snapshot == other.snapshot && self.managed_root == other.managed_root
    }
}

fn navigation_artifact_scope(
    exact: &crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution,
) -> serde_json::Value {
    serde_json::json!({
        "owner_account_id": exact.owner_account_id.as_uuid().to_string(),
        "actor_principal_id": exact.actor_principal_id.as_uuid().to_string(),
        "authenticated_session_id": exact.authenticated_session_id.as_uuid().to_string(),
        "access_space_id": exact.access_space_id.as_uuid().to_string(),
        "workspace_id": exact.workspace_id.as_str(),
    })
}

fn navigation_artifact_root() -> Result<std::path::PathBuf, ResourceAuthorityError> {
    let root = crate::storage::artifacts::resolve_workspace_root()
        .map_err(|_| ResourceAuthorityError::InvalidInput("artifact managed root unavailable"))?
        .canonicalize().map_err(|_| ResourceAuthorityError::InvalidInput("artifact managed root unavailable"))?;
    if !root.is_absolute() {
        return Err(ResourceAuthorityError::InvalidInput("artifact managed root is not absolute"));
    }
    Ok(root)
}

fn navigation_artifact_source_member(
    metadata: &serde_json::Value, source: &serde_json::Value,
) -> Result<bool, ResourceAuthorityError> {
    if metadata.is_null() { return Ok(false); }
    if let Some(rows) = metadata.as_array() {
        let mut member = false;
        for row in rows { member |= navigation_artifact_source_member(row, source)?; }
        return Ok(member);
    }
    let Some(membership) = metadata.get("source_membership") else {
        return Err(ResourceAuthorityError::InvalidInput("artifact source membership metadata malformed"));
    };
    let membership = membership.as_array()
        .ok_or(ResourceAuthorityError::InvalidInput("artifact source membership is not an array"))?;
    Ok(membership.iter().any(|candidate| candidate == source))
}

async fn navigation_artifact_snapshot(
    transaction: &super::SurrealTransactionContext<'_>,
    original_bindings: &super::model_lane::NavigationReadBindings,
    exact: &crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution,
    selection: &serde_json::Value, source: &serde_json::Value,
) -> Result<Option<(crate::storage::artifacts::ArtifactManifest, serde_json::Value)>, ResourceAuthorityError> {
    use artifact_publications::{ArtifactPublicationLocator, ArtifactPublicationRow};
    if source.get("kind").and_then(serde_json::Value::as_str) != Some("artifact") { return Ok(None); }
    let Some(source_id) = source.get("id").and_then(serde_json::Value::as_str) else { return Ok(None); };
    let Some(source_hash) = source.get("hash").and_then(serde_json::Value::as_str) else { return Ok(None); };
    let original_selection = original_bindings.artifact_selection_value()
        .map_err(|_| ResourceAuthorityError::InvalidInput("artifact original selection serialization failed"))?;
    if !original_bindings.artifact_scope_matches(exact) || original_selection != *selection {
        return Err(ResourceAuthorityError::InvalidInput("artifact original navigation binding differs"));
    }
    let scope = navigation_artifact_scope(exact);
    let membership: Vec<serde_json::Value> = transaction.query_values_at(
        "RETURN [fn::wp012_model_lane_navigation_memory_metadata({ owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id }, $navigation_selection)];",
        original_bindings.clone(), 0,
    ).await?;
    let [memory] = membership.as_slice() else {
        return Err(ResourceAuthorityError::InvalidInput("artifact source membership result malformed"));
    };
    if !navigation_artifact_source_member(memory, source)? { return Ok(None); }
    let metadata: Vec<ArtifactPublicationLocator> = transaction.query_values_at(
        "RETURN fn::wp012_model_lane_navigation_memory_artifact_metadata({ owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id }, $navigation_selection);",
        original_bindings.clone(), 0,
    ).await?;
    let matches: Vec<_> = metadata.iter().filter(|row| row.artifact_id == source_id).collect();
    let expected = match matches.as_slice() {
        [] => return Ok(None),
        [row] => *row,
        _ => return Err(ResourceAuthorityError::InvalidInput("artifact publication metadata is not canonical")),
    };
    if expected.registration_version != 1 || expected.workspace_external_id != exact.workspace_id.as_str() {
        return Err(ResourceAuthorityError::InvalidInput("artifact publication metadata scope differs"));
    }
    let publications: Vec<ArtifactPublicationRow> = transaction.query_values_at(
        "RETURN fn::wp012_model_lane_navigation_memory_artifact_publications({ owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id }, $navigation_selection);",
        original_bindings.clone(), 0,
    ).await?;
    let matches: Vec<_> = publications.iter().filter(|row| row.artifact_id == source_id).collect();
    let gated = match matches.as_slice() {
        [] => return Ok(None),
        [row] => *row,
        _ => return Err(ResourceAuthorityError::InvalidInput("artifact publication read is not canonical")),
    };
    let manifest = gated.validated_manifest()?;
    if manifest.artifact_id.to_string() != source_id || manifest.content_hash != source_hash {
        return Err(ResourceAuthorityError::InvalidInput("artifact source manifest identity differs"));
    }
    let resources: Vec<NavigationArtifactResourceRow> = transaction.query_values_at(
        "SELECT id, resource_kind, external_resource_id, owner_account_id, access_space_id, parent_resource_id, lifecycle_state, policy_version, classification FROM protected_resources WHERE id IN [$artifact, $workspace] LIMIT 3;",
        NavigationArtifactResourceBindings {
            artifact: RecordId::new("protected_resources", expected.artifact_resource_id.clone()),
            workspace: RecordId::new("protected_resources", expected.workspace_resource_id.clone()),
        }, 0,
    ).await?;
    if resources.is_empty() { return Ok(None); }
    let artifact_rows: Vec<_> = resources.iter().filter(|row| row.resource_kind == "artifact").collect();
    let workspace_rows: Vec<_> = resources.iter().filter(|row| row.resource_kind == "workspace").collect();
    let (artifact, workspace) = match (artifact_rows.as_slice(), workspace_rows.as_slice()) {
        ([artifact], [workspace]) if resources.len() == 2 => (*artifact, *workspace),
        _ => return Ok(None),
    };
    let account = RecordId::new("local_accounts", exact.owner_account_id.as_uuid().to_string());
    let space = RecordId::new("access_spaces", exact.access_space_id.as_uuid().to_string());
    if record_key(artifact.id.clone())? != expected.artifact_resource_id
        || record_key(workspace.id.clone())? != expected.workspace_resource_id
        || artifact.external_resource_id != source_id || workspace.external_resource_id != exact.workspace_id.as_str()
        || artifact.parent_resource_id.as_ref() != Some(&workspace.id)
        || artifact.owner_account_id != account || workspace.owner_account_id != account
        || artifact.access_space_id != space || workspace.access_space_id != space
        || artifact.lifecycle_state != "active" || workspace.lifecycle_state != "active"
        || artifact.policy_version < 1 || workspace.policy_version < 1
    {
        return Err(ResourceAuthorityError::InvalidInput("artifact canonical resource parent differs"));
    }
    let classification = serde_json::to_value(manifest.classification)
        .map_err(|_| ResourceAuthorityError::InvalidInput("artifact classification serialization failed"))?;
    if classification.as_str() != Some(artifact.classification.as_str()) {
        return Err(ResourceAuthorityError::InvalidInput("artifact canonical classification differs"));
    }
    let locator = |row: &NavigationArtifactResourceRow| CanonicalResourceLocator {
        id: row.id.clone(), external_resource_id: row.external_resource_id.clone(),
        owner_account_id: row.owner_account_id.clone(), access_space_id: row.access_space_id.clone(),
        parent_resource_id: row.parent_resource_id.clone(), lifecycle_state: row.lifecycle_state.clone(), policy_version: row.policy_version,
    };
    let canonical = artifact_publications::read_artifact_publication(
        transaction, &locator(artifact), &locator(workspace), expected,
    ).await?;
    let Some(canonical) = canonical else { return Ok(None); };
    if &canonical != gated {
        return Err(ResourceAuthorityError::InvalidInput("artifact gated publication changed"));
    }
    let raw_manifest: serde_json::Value = serde_json::from_str(&canonical.manifest_json)
        .map_err(|_| ResourceAuthorityError::InvalidInput("artifact manifest JSON malformed"))?;
    let full_manifest = serde_json::to_value(&manifest)
        .map_err(|_| ResourceAuthorityError::InvalidInput("artifact manifest serialization failed"))?;
    if raw_manifest != full_manifest {
        return Err(ResourceAuthorityError::InvalidInput("artifact full manifest representation differs"));
    }
    let publication = serde_json::to_value(&canonical)
        .map_err(|_| ResourceAuthorityError::InvalidInput("artifact publication serialization failed"))?;
    let snapshot = serde_json::json!({
        "scope": scope, "selection": selection, "source": source, "publication": publication,
        "artifact": {"resource_id":expected.artifact_resource_id,"external_resource_id":artifact.external_resource_id,
            "owner_account_id":exact.owner_account_id.as_uuid().to_string(),"access_space_id":exact.access_space_id.as_uuid().to_string(),
            "parent_resource_id":expected.workspace_resource_id,"lifecycle_state":artifact.lifecycle_state,
            "policy_version":artifact.policy_version,"classification":artifact.classification},
        "workspace": {"resource_id":expected.workspace_resource_id,"external_resource_id":workspace.external_resource_id,
            "owner_account_id":exact.owner_account_id.as_uuid().to_string(),"access_space_id":exact.access_space_id.as_uuid().to_string(),
            "lifecycle_state":workspace.lifecycle_state,"policy_version":workspace.policy_version,
            "classification":workspace.classification},
    });
    Ok(Some((manifest, snapshot)))
}

pub(crate) async fn authorize_navigation_artifact_read<'tx>(
    transaction: &'tx super::SurrealTransactionContext<'tx>,
    original_bindings: &super::model_lane::NavigationReadBindings,
    exact_scope: &crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution,
    selection: &serde_json::Value, source: &crate::ace::FemsSourceRef,
) -> Result<Option<NavigationArtifactReadPermit<'tx>>, ResourceAuthorityError> {
    let source = serde_json::to_value(source)
        .map_err(|_| ResourceAuthorityError::InvalidInput("artifact source serialization failed"))?;
    let Some((_manifest, snapshot)) = navigation_artifact_snapshot(transaction, original_bindings, exact_scope, selection, &source).await? else {
        return Ok(None);
    };
    let managed_root = navigation_artifact_root()?;
    Ok(Some(NavigationArtifactReadPermit {
        transaction, original_bindings: original_bindings.clone(), exact_scope: exact_scope.clone(), selection: selection.clone(), source, snapshot, managed_root,
    }))
}

pub(crate) async fn read_navigation_artifact(
    transaction: &super::SurrealTransactionContext<'_>, permit: &NavigationArtifactReadPermit<'_>,
) -> Result<Option<NavigationArtifactRegistration>, ResourceAuthorityError> {
    if !std::ptr::eq(transaction, permit.transaction) {
        return Err(ResourceAuthorityError::InvalidInput("artifact read transaction differs"));
    }
    let Some((manifest, snapshot)) = navigation_artifact_snapshot(
        transaction, &permit.original_bindings, &permit.exact_scope, &permit.selection, &permit.source,
    ).await? else { return Ok(None); };
    if snapshot != permit.snapshot || navigation_artifact_root()? != permit.managed_root {
        return Err(ResourceAuthorityError::InvalidInput("artifact read registration changed"));
    }
    let persisted = crate::storage::artifacts::read_persisted_file_artifact(
        &permit.managed_root, manifest.layer.clone(), manifest.artifact_id,
    ).map_err(|_| ResourceAuthorityError::InvalidInput("artifact managed payload validation failed"))?;
    let expected = serde_json::to_value(&manifest)
        .map_err(|_| ResourceAuthorityError::InvalidInput("artifact manifest serialization failed"))?;
    let actual = serde_json::to_value(persisted.manifest())
        .map_err(|_| ResourceAuthorityError::InvalidInput("artifact manifest serialization failed"))?;
    if actual != expected {
        return Err(ResourceAuthorityError::InvalidInput("artifact managed manifest differs"));
    }
    let Some((_manifest, after)) = navigation_artifact_snapshot(
        transaction, &permit.original_bindings, &permit.exact_scope, &permit.selection, &permit.source,
    ).await? else { return Ok(None); };
    if after != snapshot || navigation_artifact_root()? != permit.managed_root {
        return Err(ResourceAuthorityError::InvalidInput("artifact read registration changed"));
    }
    Ok(Some(NavigationArtifactRegistration { manifest, snapshot, managed_root: permit.managed_root.clone() }))
}

#[derive(SurrealValue)]
struct ArtifactResourceRegistrationBindings {
    resource: RecordId,
    external: String,
    account: RecordId,
    principal: RecordId,
    session: RecordId,
    space: RecordId,
    parent: RecordId,
    classification: String,
    locator_hash: String,
    now: Datetime,
}

#[derive(Clone, Eq, PartialEq, SurrealValue)]
struct ArtifactRegisteredResourceRow {
    id: RecordId,
    external_resource_id: String,
    owner_account_id: RecordId,
    access_space_id: RecordId,
    parent_resource_id: Option<RecordId>,
    lifecycle_state: String,
    policy_version: i64,
    created_by_principal_id: RecordId,
    created_in_session_id: RecordId,
    classification: String,
    schema_version: i64,
    storage_locator_hash: String,
}

enum ArtifactPublicationFilesystemCheck<'a> {
    Live,
    Closed {
        terminal: &'a super::ArtifactPublicationCanonicalTerminalForOwner,
        closure: &'a crate::storage::artifacts::ArtifactPublicationOwnedClosure,
    },
}

#[derive(Clone, Eq, PartialEq)]
struct ExpectedWorkspaceArtifactPublication {
    resource: ArtifactRegisteredResourceRow,
    publication: artifact_publications::ArtifactPublicationRow,
}

#[derive(SurrealValue)]
struct ArtifactResourceRecheckBindings {
    resource: RecordId,
}

pub(crate) struct WorkspaceArtifactPublicationAuthorization {
    resources: std::sync::Arc<crate::storage::artifacts::ArtifactPublicationResources>,
    expected_publications: std::sync::Mutex<Vec<ExpectedWorkspaceArtifactPublication>>,
    operation_context: std::sync::Arc<super::WorkspaceArtifactPublicationOperationContext>,
    managed_root: std::path::PathBuf,
    capture: ProducerSourceFactsCapture,
    workspace: CanonicalResourceLocator,
    exact: crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution,
    namespace: String,
    database: String,
}

pub(crate) struct WorkspaceArtifactPublicationPermit<'tx> {
    authorization: &'tx WorkspaceArtifactPublicationAuthorization,
    transaction: &'tx super::SurrealTransactionContext<'tx>,
}

impl WorkspaceArtifactPublicationAuthorization {
    pub(crate) fn publication_resources(&self) -> &std::sync::Arc<crate::storage::artifacts::ArtifactPublicationResources> {
        &self.resources
    }

    pub(crate) fn managed_root(&self) -> &std::path::Path { &self.managed_root }

    fn require_current_managed_root(&self) -> Result<(), ResourceAuthorityError> {
        let current = crate::storage::artifacts::resolve_workspace_root()
            .map_err(|_| ResourceAuthorityError::InvalidInput("artifact managed root unavailable"))?
            .canonicalize().map_err(|_| ResourceAuthorityError::InvalidInput("artifact managed root unavailable"))?;
        if current != self.managed_root {
            return Err(ResourceAuthorityError::InvalidInput("artifact managed root changed"));
        }
        Ok(())
    }

    pub(crate) fn operation_id(&self) -> Uuid { self.capture.operation_id }
    pub(crate) fn exact_scope(&self) -> &crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution { &self.exact }
    pub(crate) fn workspace_parent(&self) -> &CanonicalResourceLocator { &self.workspace }

    fn record_user_scope(&self) -> RecordUserScope {
        let source = &self.capture.sources[0];
        RecordUserScope {
            grant_id: Some(source.decision.grant_id.clone()), workspace_id: Some(self.workspace.external_resource_id.clone()),
            session_token: self.capture.credentials.session_token.clone(),
            channel_binding_hash: Some(self.capture.credentials.channel_binding_hash.clone()),
            resource_id: source.decision.resource_id.clone(), session_id: source.decision.session_id.clone(),
            capability_id: source.request.capability_id.clone(), action: source.request.action,
        }
    }
}

impl<'tx> WorkspaceArtifactPublicationPermit<'tx> {
    pub(crate) fn persist_file_artifact_owned(
        &self, manifest: &crate::storage::artifacts::ArtifactManifest, payload: &[u8],
    ) -> Result<crate::storage::artifacts::PersistedFileArtifact, crate::storage::artifacts::ArtifactError> {
        self.authorization.resources.persist_file_artifact_owned(manifest, payload)
    }

    pub(crate) async fn register_persisted_artifact(
        &self, persisted: &crate::storage::artifacts::PersistedFileArtifact,
    ) -> Result<RegisteredArtifactWitness, ResourceAuthorityError> {
        let manifest = persisted.manifest();
        self.authorization.require_current_managed_root()?;
        let disk = self.authorization.resources.read_persisted_file_artifact_owned(
            manifest.layer.clone(), manifest.artifact_id,
        ).map_err(|_| ResourceAuthorityError::InvalidInput("artifact managed payload validation failed"))?;
        let expected_manifest = serde_json::to_value(manifest)
            .map_err(|_| ResourceAuthorityError::InvalidInput("artifact manifest serialization failed"))?;
        let actual_manifest = serde_json::to_value(disk.manifest())
            .map_err(|_| ResourceAuthorityError::InvalidInput("artifact manifest serialization failed"))?;
        if actual_manifest != expected_manifest {
            return Err(ResourceAuthorityError::InvalidInput("artifact managed manifest differs"));
        }
        self.authorization.require_current_managed_root()?;
        if manifest.artifact_id.is_nil() {
            return Err(ResourceAuthorityError::InvalidInput("publication artifact UUID is required"));
        }
        let external = manifest.artifact_id.to_string();
        let classification = serde_json::to_value(manifest.classification)
            .map_err(|_| ResourceAuthorityError::InvalidInput("artifact classification serialization failed"))?
            .as_str().ok_or(ResourceAuthorityError::InvalidInput("artifact classification is not a string"))?.to_owned();
        let facts = &self.authorization.capture.sources[0].authority;
        let locator_hash = sha256_hex(format!("artifact:{external}").as_bytes());
        let rows: Vec<ArtifactRegisteredResourceRow> = self.transaction.query_values_at(
            "IF array::len(SELECT id FROM protected_resources WHERE resource_kind = 'artifact' AND external_resource_id = $external LIMIT 2) = 0 { CREATE $resource SET resource_kind = 'artifact', external_resource_id = $external, owner_account_id = $account, created_by_principal_id = $principal, created_in_session_id = $session, access_space_id = $space, parent_resource_id = $parent, schema_version = 1, lifecycle_state = 'active', policy_version = 1, classification = $classification, storage_locator_hash = $locator_hash, created_at = $now, updated_at = $now; }; SELECT id, external_resource_id, owner_account_id, access_space_id, parent_resource_id, lifecycle_state, policy_version, created_by_principal_id, created_in_session_id, classification, schema_version, storage_locator_hash FROM protected_resources WHERE resource_kind = 'artifact' AND external_resource_id = $external LIMIT 2;",
            ArtifactResourceRegistrationBindings {
                resource: RecordId::new("protected_resources", Uuid::now_v7().to_string()), external: external.clone(),
                account: facts.account_id.clone(), principal: facts.principal_id.clone(), session: facts.session_id.clone(),
                space: facts.access_space_id.clone(), parent: self.authorization.workspace.id.clone(),
                classification: classification.clone(), locator_hash: locator_hash.clone(), now: Datetime::from(Utc::now()),
            }, 1,
        ).await?;
        let [row] = rows.as_slice() else {
            return Err(ResourceAuthorityError::InvalidInput("artifact resource is not canonical"));
        };
        if row.external_resource_id != external || row.owner_account_id != facts.account_id
            || row.access_space_id != facts.access_space_id || row.parent_resource_id.as_ref() != Some(&self.authorization.workspace.id)
            || row.lifecycle_state != "active" || row.policy_version != 1
            || row.created_by_principal_id != facts.principal_id || row.created_in_session_id != facts.session_id
            || row.classification != classification || row.schema_version != 1 || row.storage_locator_hash != locator_hash
        {
            return Err(ResourceAuthorityError::InvalidInput("artifact resource metadata differs"));
        }
        let canonical = CanonicalResourceLocator {
            id: row.id.clone(), external_resource_id: row.external_resource_id.clone(),
            owner_account_id: row.owner_account_id.clone(), access_space_id: row.access_space_id.clone(),
            parent_resource_id: row.parent_resource_id.clone(), lifecycle_state: row.lifecycle_state.clone(), policy_version: row.policy_version,
        };
        let witness = artifact_publications::register_artifact_publication(self, &canonical, persisted).await?;
        let expected = artifact_publications::ArtifactPublicationLocator {
            artifact_id: external,
            artifact_resource_id: record_key(canonical.id.clone())?,
            workspace_external_id: self.authorization.workspace.external_resource_id.clone(),
            workspace_resource_id: record_key(self.authorization.workspace.id.clone())?,
            manifest_sha256: sha256_hex(serde_json::to_string(manifest)
                .map_err(|_| ResourceAuthorityError::InvalidInput("artifact manifest serialization failed"))?.as_bytes()),
            producer_operation_id: self.operation_id().to_string(), registration_version: 1,
        };
        let publication = artifact_publications::read_artifact_publication(
            self.transaction, &canonical, &self.authorization.workspace, &expected,
        ).await?.ok_or(ResourceAuthorityError::InvalidInput("artifact registered publication unavailable"))?;
        let validated = publication.validated_manifest()?;
        if serde_json::to_value(&validated)
            .map_err(|_| ResourceAuthorityError::InvalidInput("artifact manifest serialization failed"))? != expected_manifest
        {
            return Err(ResourceAuthorityError::InvalidInput("artifact registered manifest differs"));
        }
        let facts = ExpectedWorkspaceArtifactPublication { resource: row.clone(), publication };
        let mut expected = self.authorization.expected_publications.lock()
            .map_err(|_| ResourceAuthorityError::InvalidInput("artifact expected publications unavailable"))?;
        if let Some(prior) = expected.iter().find(|prior| prior.publication.artifact_id == facts.publication.artifact_id) {
            if prior != &facts {
                return Err(ResourceAuthorityError::InvalidInput("artifact expected publication conflicts"));
            }
        } else {
            expected.push(facts);
        }
        Ok(witness)
    }


    pub(crate) fn managed_root(&self) -> &std::path::Path { self.authorization.managed_root() }
    pub(crate) fn operation_id(&self) -> Uuid { self.authorization.operation_id() }
    pub(crate) fn exact_scope(&self) -> &crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution { self.authorization.exact_scope() }
    pub(crate) fn workspace_parent(&self) -> &CanonicalResourceLocator { self.authorization.workspace_parent() }
    pub(crate) fn transaction(&self) -> &super::SurrealTransactionContext<'tx> { self.transaction }
    pub(crate) fn matches_transaction(&self, transaction: &super::SurrealTransactionContext<'_>) -> bool {
        std::ptr::eq(self.transaction, transaction)
    }
}

pub(crate) struct PalmistryWatcherAdmissionReadAuthorization {
    retained: std::sync::Arc<PalmistryWatcherAdoptionAuthority>,
    incoming: RoleAuthorityAuthInput,
    registry: crate::capabilities::CapabilityRegistry,
    source: ProducerSourceFacts,
    parent: CanonicalResourceLocator,
}

impl PalmistryWatcherAdmissionReadAuthorization {
    pub(crate) fn authority(&self) -> &std::sync::Arc<PalmistryWatcherAdoptionAuthority> { &self.retained }
    pub(crate) fn operation_id(&self) -> Uuid { self.retained.operation_id() }
    pub(crate) fn exact_scope(&self) -> &crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution {
        self.retained.exact_scope()
    }
}

pub(crate) struct PalmistryWatcherAdmissionAuthorization {
    authority: std::sync::Arc<PalmistryWatcherAdoptionAuthority>,
    expires_at: chrono::DateTime<Utc>,
}

impl PalmistryWatcherAdmissionAuthorization {
    pub(crate) fn authority(&self) -> &std::sync::Arc<PalmistryWatcherAdoptionAuthority> { &self.authority }
    pub(crate) fn operation_id(&self) -> Uuid { self.authority.operation_id() }
    pub(crate) fn exact_scope(&self) -> &crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution {
        self.authority.exact_scope()
    }
    pub(crate) fn resource_id(&self) -> &RecordId { &self.authority.capture.sources[0].authority.resource_id }
    pub(crate) fn grant_id(&self) -> &RecordId { &self.authority.capture.sources[0].authority.grant_id }
    pub(crate) fn grant_version(&self) -> i64 { self.authority.capture.sources[0].authority.grant_version }
    pub(crate) fn policy_versions(&self) -> &[i64] { &self.authority.capture.sources[0].authority.policy_versions }
    pub(crate) fn revocation_epochs(&self) -> &[i64] { &self.authority.capture.sources[0].authority.revocation_epochs }
    pub(crate) fn delegation_chain(&self) -> &[String] { &self.authority.capture.sources[0].authority.delegation_chain }
    pub(crate) fn expires_at(&self) -> &chrono::DateTime<Utc> { &self.expires_at }

    pub(crate) fn validate_binding(
        &self, authority: &std::sync::Arc<PalmistryWatcherAdoptionAuthority>, expires_at: &chrono::DateTime<Utc>,
    ) -> Result<(), ResourceAuthorityError> {
        if !std::sync::Arc::ptr_eq(&self.authority, authority)
            || *expires_at > self.expires_at || *expires_at <= Utc::now()
        {
            return Err(ResourceAuthorityError::InvalidInput("watcher admission authorization binding differs"));
        }
        Ok(())
    }

    pub(crate) async fn revalidate(&self, storage: &SurrealStorage) -> Result<(), ResourceAuthorityError> {
        storage.revalidate_palmistry_watcher_adoption_authority(&self.authority).await?;
        self.validate_binding(&self.authority, &self.expires_at)
    }
}

#[derive(SurrealValue)]
struct PalmistryWatcherParentBindings {
    resource: RecordId,
    parent: RecordId,
    account: RecordId,
    space: RecordId,
    policy: i64,
}

impl PalmistryWatcherAdoptionAuthority {
    pub(crate) fn operation_id(&self) -> Uuid { self.capture.operation_id }
    pub(crate) fn watcher_id(&self) -> Uuid { self.watcher_id }
    pub(crate) fn exact_scope(&self) -> &crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution {
        &self.exact
    }
    pub(crate) fn workspace_parent(&self) -> &CanonicalResourceLocator { &self.parent }

    pub(crate) fn record_user_scope(&self) -> RecordUserScope {
        let source = &self.capture.sources[0];
        RecordUserScope {
            grant_id: Some(source.decision.grant_id.clone()),
            workspace_id: Some(self.parent.external_resource_id.clone()),
            session_token: self.capture.credentials.session_token.clone(),
            channel_binding_hash: Some(self.capture.credentials.channel_binding_hash.clone()),
            resource_id: source.decision.resource_id.clone(),
            session_id: source.decision.session_id.clone(),
            capability_id: source.request.capability_id.clone(),
            action: source.request.action,
        }
    }

    fn require_current_binding(&self, storage: &SurrealStorage) -> Result<(), ResourceAuthorityError> {
        let source = &self.capture.sources[0];
        let denied = || ResourceAuthorityError::Denied { decision_id: source.decision.decision_id.clone() };
        let grant_policy = source.authority.policy_versions.first().ok_or_else(denied)?;
        if self.capture.lifecycle != storage.resource_access_lifecycle()
            || self.namespace != storage.config().namespace() || self.database != storage.config().database()
            || self.capture.operation_id.is_nil() || self.watcher_id.is_nil()
            || source.request.resource_kind != ResourceKind::PalmistryWatcher
            || source.request.external_resource_id != self.watcher_id.to_string()
            || source.request.action != ResourceAction::Reconcile
            || source.request.capability_id != PALMISTRY_WATCHER_ADOPT_CAPABILITY
            || source.authority.capability_profile_id != "Operator"
            || !self.capture.registry.profile_can("Operator", PALMISTRY_WATCHER_ADOPT_CAPABILITY).unwrap_or(false)
            || !source.authority.capability_ids.iter().any(|capability| capability == PALMISTRY_WATCHER_ADOPT_CAPABILITY)
            || self.parent.owner_account_id != source.authority.account_id
            || self.parent.access_space_id != source.authority.access_space_id
            || self.parent.lifecycle_state != "active" || self.parent.policy_version > *grant_policy
        {
            return Err(denied());
        }
        Ok(())
    }

    pub(crate) async fn revalidate_in_transaction(
        &self, storage: &SurrealStorage, transaction: &super::SurrealTransactionContext<'_>,
    ) -> Result<(), ResourceAuthorityError> {
        self.require_current_binding(storage)?;
        let _fence = self.capture.fence.clone();
        let source = &self.capture.sources[0];
        let denied = || ResourceAuthorityError::Denied { decision_id: source.decision.decision_id.clone() };
        let current = super::current_record_user_scope().ok_or_else(denied)?;
        let original = self.record_user_scope();
        if current.session_token != original.session_token || current.channel_binding_hash != original.channel_binding_hash
            || current.session_id != original.session_id || current.workspace_id != original.workspace_id
            || current.resource_id != original.resource_id || current.grant_id != original.grant_id
            || current.capability_id != original.capability_id || current.action != original.action
        {
            return Err(denied());
        }
        let rows: Vec<RegistryPublicationAuthority> = transaction.query_values_at(
            REGISTRY_PUBLICATION_AUTHORITY,
            RegistryPublicationBindings {
                resource: source.authority.resource_id.clone(), original_grant: source.authority.grant_id.clone(),
                kind: source.authority.resource_kind.clone(), action: source.request.action.as_str().to_owned(),
                capability: source.request.capability_id.clone(),
            }, 0,
        ).await?;
        if rows.as_slice() != std::slice::from_ref(&source.authority) { return Err(denied()); }
        let parents: Vec<RecordId> = transaction.query_values_at(
            "SELECT VALUE parent_resource_id FROM protected_resources WHERE id = $resource AND resource_kind = 'palmistry_watcher' AND lifecycle_state = 'active' AND parent_resource_id = $parent AND parent_resource_id.resource_kind = 'workspace' AND parent_resource_id.lifecycle_state = 'active' AND parent_resource_id.owner_account_id = $account AND parent_resource_id.access_space_id = $space AND parent_resource_id.policy_version = $policy LIMIT 2;",
            PalmistryWatcherParentBindings {
                resource: source.authority.resource_id.clone(), parent: self.parent.id.clone(),
                account: source.authority.account_id.clone(), space: source.authority.access_space_id.clone(),
                policy: self.parent.policy_version,
            }, 0,
        ).await?;
        if parents.as_slice() != std::slice::from_ref(&self.parent.id) { return Err(denied()); }
        CanonicalNavigationPublicationAuthority {
            grant_expires_at: source.authority.grant_expires_at.clone(),
            session_expires_at: source.authority.session_expires_at.clone(),
        }.require_unexpired()
    }
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

struct ProducerRunCompletion {
    operation_id: Uuid,
    result: Result<crate::swarm_orchestration::model_lane::ModelLaneRunRecord, ProducerRunFailure>,
    finalization: Option<super::OriginalRunCreateFinalizationProof>,
    owner: std::sync::Arc<dyn std::any::Any + Send + Sync>,
}

impl SurrealStorage {
    pub(crate) async fn capture_workspace_artifact_publication_authorization(
        &self, operation_id: Uuid, registry: &crate::capabilities::CapabilityRegistry,
        credentials: RoleAuthorityAuthInput, workspace_id: &str,
    ) -> Result<std::sync::Arc<WorkspaceArtifactPublicationAuthorization>, ResourceAuthorityError> {
        use crate::swarm_orchestration::resource_scope::{AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef,
            ExactResourceScopeAttribution, OwnerAccountId, WorkspaceScopeRef};
        let request = AuthorizationRequest {
            session_token: credentials.session_token.clone(), channel_binding_hash: Some(credentials.channel_binding_hash.clone()),
            capability_id: "fs.write".to_owned(), resource_kind: ResourceKind::Workspace,
            external_resource_id: workspace_id.to_owned(), action: ResourceAction::Create,
        };
        let capture = self.capture_producer_source_facts(operation_id, registry, credentials, vec![request]).await?;
        let workspace = self.canonical_resource_locator(ResourceKind::Workspace, workspace_id).await?
            .ok_or(ResourceAuthorityError::InvalidInput("publication Workspace unavailable"))?;
        let source = &capture.sources[0];
        if workspace.id != source.authority.resource_id || workspace.owner_account_id != source.authority.account_id
            || workspace.access_space_id != source.authority.access_space_id || workspace.lifecycle_state != "active"
        {
            return Err(ResourceAuthorityError::InvalidInput("publication Workspace authority differs"));
        }
        let parse = |value: &str| Uuid::parse_str(value)
            .map_err(|_| ResourceAuthorityError::InvalidInput("invalid publication authority UUID"));
        let exact = ExactResourceScopeAttribution {
            owner_account_id: OwnerAccountId::from_uuid(parse(&source.decision.account_id)?),
            actor_principal_id: ActorPrincipalId::from_uuid(parse(&source.decision.principal_id)?),
            authenticated_session_id: AuthenticatedSessionRef::from_uuid(parse(&source.decision.session_id)?),
            access_space_id: AccessSpaceRef::from_uuid(parse(&source.decision.access_space_id)?),
            workspace_id: WorkspaceScopeRef::new(workspace_id.to_owned())
                .map_err(|_| ResourceAuthorityError::InvalidInput("invalid publication Workspace"))?,
        };
        let mut authorization_expires_at = source.authority.session_expires_at.clone().into_inner();
        if let Some(grant_expiry) = &source.authority.grant_expires_at {
            authorization_expires_at = authorization_expires_at.min(grant_expiry.clone().into_inner());
        }
        let operation_context = self.capture_workspace_artifact_publication_operation_context(
            operation_id, RecordUserScope {
                grant_id: Some(source.decision.grant_id.clone()), workspace_id: Some(workspace.external_resource_id.clone()),
                session_token: capture.credentials.session_token.clone(),
                channel_binding_hash: Some(capture.credentials.channel_binding_hash.clone()),
                resource_id: source.decision.resource_id.clone(), session_id: source.decision.session_id.clone(),
                capability_id: source.request.capability_id.clone(), action: source.request.action,
            }, authorization_expires_at,
        )?;
        let managed_root = crate::storage::artifacts::resolve_workspace_root()
            .map_err(|_| ResourceAuthorityError::InvalidInput("artifact managed root unavailable"))?
            .canonicalize().map_err(|_| ResourceAuthorityError::InvalidInput("artifact managed root unavailable"))?;
        if !managed_root.is_absolute() {
            return Err(ResourceAuthorityError::InvalidInput("artifact managed root is not absolute"));
        }
        let resources = std::sync::Arc::new(crate::storage::artifacts::ArtifactPublicationResources::new(
            operation_id, managed_root.clone(),
        ));
        let authority = std::sync::Arc::new(WorkspaceArtifactPublicationAuthorization {
            capture, workspace, exact, managed_root, operation_context, resources,
            expected_publications: std::sync::Mutex::new(Vec::new()), namespace: self.config().namespace().to_owned(), database: self.config().database().to_owned(),
        });
        self.revalidate_workspace_artifact_publication_authorization(&authority).await?;
        Ok(authority)
    }

    pub(crate) async fn revalidate_workspace_artifact_publication_authorization(
        &self, authority: &WorkspaceArtifactPublicationAuthorization,
    ) -> Result<(), ResourceAuthorityError> {
        authority.require_current_managed_root()?;
        if authority.namespace != self.config().namespace() || authority.database != self.config().database() {
            return Err(ResourceAuthorityError::InvalidInput("publication backend differs"));
        }
        self.revalidate_producer_source_facts(&authority.capture).await?;
        let source = &authority.capture.sources[0];
        if source.request.resource_kind != ResourceKind::Workspace || source.request.action != ResourceAction::Create
            || source.request.capability_id != "fs.write"
            || self.canonical_resource_locator(ResourceKind::Workspace, &authority.workspace.external_resource_id).await?.as_ref()
                != Some(&authority.workspace)
        {
            return Err(ResourceAuthorityError::InvalidInput("publication Workspace changed"));
        }
        Ok(())
    }

    async fn revalidate_workspace_artifact_publication_in_transaction(
        &self, authority: &WorkspaceArtifactPublicationAuthorization, transaction: &super::SurrealTransactionContext<'_>,
    ) -> Result<(), ResourceAuthorityError> {
        authority.require_current_managed_root()?;
        if authority.capture.lifecycle != self.resource_access_lifecycle()
            || authority.namespace != self.config().namespace() || authority.database != self.config().database()
        {
            return Err(ResourceAuthorityError::InvalidInput("publication transaction backend differs"));
        }
        let current = super::current_record_user_scope()
            .ok_or(ResourceAuthorityError::InvalidInput("publication signed scope unavailable"))?;
        let original = authority.record_user_scope();
        if current.session_token != original.session_token || current.channel_binding_hash != original.channel_binding_hash
            || current.session_id != original.session_id || current.workspace_id != original.workspace_id
            || current.resource_id != original.resource_id || current.grant_id != original.grant_id
            || current.capability_id != original.capability_id || current.action != original.action
        {
            return Err(ResourceAuthorityError::InvalidInput("publication transaction scope differs"));
        }
        let source = &authority.capture.sources[0];
        let rows: Vec<RegistryPublicationAuthority> = transaction.query_values_at(
            REGISTRY_PUBLICATION_AUTHORITY, RegistryPublicationBindings {
                resource: source.authority.resource_id.clone(), original_grant: source.authority.grant_id.clone(),
                kind: ResourceKind::Workspace.as_str().to_owned(), action: ResourceAction::Create.as_str().to_owned(),
                capability: "fs.write".to_owned(),
            }, 0,
        ).await?;
        if rows.as_slice() != std::slice::from_ref(&source.authority) {
            return Err(ResourceAuthorityError::InvalidInput("publication transaction authority changed"));
        }
        CanonicalNavigationPublicationAuthority {
            grant_expires_at: source.authority.grant_expires_at.clone(), session_expires_at: source.authority.session_expires_at.clone(),
        }.require_unexpired()
    }

    fn require_workspace_artifact_publication_reconciliation_read(
        &self, authority: &WorkspaceArtifactPublicationAuthorization,
        read: &super::WorkspaceArtifactPublicationReconciliationRead,
    ) -> Result<(), ResourceAuthorityError> {
        let facts = &authority.capture.sources[0].authority;
        let mut authority_expires_at = facts.session_expires_at.clone().into_inner();
        if let Some(grant_expiry) = &facts.grant_expires_at {
            authority_expires_at = authority_expires_at.min(grant_expiry.clone().into_inner());
        }
        if !read.matches_original(&authority.operation_context)
            || read.authority_expires_at() != authority_expires_at
            || std::time::Instant::now() >= read.deadline()
        {
            return Err(ResourceAuthorityError::InvalidInput("artifact reconciliation read binding expired or differs"));
        }
        Ok(())
    }

    async fn with_workspace_artifact_publication_reconciliation_read<T, F>(
        &self, authority: &std::sync::Arc<WorkspaceArtifactPublicationAuthorization>,
        transaction: &super::SurrealTransactionContext<'_>,
        read: &super::WorkspaceArtifactPublicationReconciliationRead, operation: F,
    ) -> Result<T, ResourceAuthorityError>
    where
        F: std::future::Future<Output = Result<T, ResourceAuthorityError>>,
    {
        self.require_workspace_artifact_publication_reconciliation_read(authority, read)?;
        tokio::time::timeout_at(tokio::time::Instant::from_std(read.deadline()), async {
            self.revalidate_workspace_artifact_publication_in_transaction(authority, transaction).await?;
            let result = operation.await?;
            self.revalidate_workspace_artifact_publication_in_transaction(authority, transaction).await?;
            self.require_workspace_artifact_publication_reconciliation_read(authority, read)?;
            Ok(result)
        }).await.map_err(|_| ResourceAuthorityError::InvalidInput("artifact reconciliation read deadline elapsed"))?
    }

    pub(crate) async fn recheck_workspace_artifact_publication_authority_in_transaction(
        &self, authority: &std::sync::Arc<WorkspaceArtifactPublicationAuthorization>,
        transaction: &super::SurrealTransactionContext<'_>,
        read: &super::WorkspaceArtifactPublicationReconciliationRead,
    ) -> Result<(), ResourceAuthorityError> {
        self.with_workspace_artifact_publication_reconciliation_read(
            authority, transaction, read, std::future::ready(Ok(())),
        ).await
    }

    pub(crate) async fn recheck_workspace_artifact_publication_in_transaction(
        &self, authority: &std::sync::Arc<WorkspaceArtifactPublicationAuthorization>,
        transaction: &super::SurrealTransactionContext<'_>,
        read: &super::WorkspaceArtifactPublicationReconciliationRead,
    ) -> Result<(), ResourceAuthorityError> {
        self.with_workspace_artifact_publication_reconciliation_read(
            authority, transaction, read, self.recheck_workspace_artifact_publication_state_in_transaction(
                authority, transaction, ArtifactPublicationFilesystemCheck::Live,
            ),
        ).await
    }

    pub(crate) async fn recheck_workspace_artifact_publication_after_owned_closure_in_transaction(
        &self, authority: &std::sync::Arc<WorkspaceArtifactPublicationAuthorization>,
        transaction: &super::SurrealTransactionContext<'_>,
        terminal: &super::ArtifactPublicationCanonicalTerminalForOwner,
        closure: &crate::storage::artifacts::ArtifactPublicationOwnedClosure,
        read: &super::WorkspaceArtifactPublicationReconciliationRead,
    ) -> Result<(), ResourceAuthorityError> {
        self.with_workspace_artifact_publication_reconciliation_read(
            authority, transaction, read, self.recheck_workspace_artifact_publication_state_in_transaction(
                authority, transaction, ArtifactPublicationFilesystemCheck::Closed { terminal, closure },
            ),
        ).await
    }

    async fn recheck_workspace_artifact_publication_state_in_transaction(
        &self, authority: &std::sync::Arc<WorkspaceArtifactPublicationAuthorization>,
        transaction: &super::SurrealTransactionContext<'_>, filesystem: ArtifactPublicationFilesystemCheck<'_>,
    ) -> Result<(), ResourceAuthorityError> {
        if let ArtifactPublicationFilesystemCheck::Closed { terminal, closure } = &filesystem {
            if !terminal.matches_resources(authority.publication_resources())
                || terminal.operation_id() != authority.operation_id()
                || !matches!(terminal.decision(), &super::ArtifactPublicationCanonicalDecision::Commit)
                || !closure.matches_terminal(terminal)
            {
                return Err(ResourceAuthorityError::InvalidInput("artifact owned closure binding differs"));
            }
        }
        let expected = authority.expected_publications.lock()
            .map_err(|_| ResourceAuthorityError::InvalidInput("artifact expected publications unavailable"))?.clone();
        if expected.is_empty() {
            return Err(ResourceAuthorityError::InvalidInput("artifact expected publications incomplete"));
        }
        for expected in expected {
            let resources: Vec<ArtifactRegisteredResourceRow> = transaction.query_values_at(
                "SELECT id, external_resource_id, owner_account_id, access_space_id, parent_resource_id, lifecycle_state, policy_version, created_by_principal_id, created_in_session_id, classification, schema_version, storage_locator_hash FROM protected_resources WHERE id = $resource AND resource_kind = 'artifact' LIMIT 2;",
                ArtifactResourceRecheckBindings { resource: expected.resource.id.clone() }, 0,
            ).await?;
            if resources.as_slice() != std::slice::from_ref(&expected.resource) {
                return Err(ResourceAuthorityError::InvalidInput("artifact canonical resource changed"));
            }
            let canonical = CanonicalResourceLocator {
                id: expected.resource.id.clone(), external_resource_id: expected.resource.external_resource_id.clone(),
                owner_account_id: expected.resource.owner_account_id.clone(), access_space_id: expected.resource.access_space_id.clone(),
                parent_resource_id: expected.resource.parent_resource_id.clone(), lifecycle_state: expected.resource.lifecycle_state.clone(),
                policy_version: expected.resource.policy_version,
            };
            let locator = artifact_publications::ArtifactPublicationLocator {
                artifact_id: expected.publication.artifact_id.clone(), artifact_resource_id: expected.publication.artifact_resource_id.clone(),
                workspace_external_id: expected.publication.workspace_external_id.clone(), workspace_resource_id: expected.publication.workspace_resource_id.clone(),
                manifest_sha256: expected.publication.manifest_sha256.clone(), producer_operation_id: expected.publication.producer_operation_id.clone(),
                registration_version: expected.publication.registration_version,
            };
            let current = artifact_publications::read_artifact_publication(
                transaction, &canonical, &authority.workspace, &locator,
            ).await?.ok_or(ResourceAuthorityError::InvalidInput("artifact canonical publication unavailable"))?;
            if current != expected.publication {
                return Err(ResourceAuthorityError::InvalidInput("artifact canonical publication changed"));
            }
            let manifest = current.validated_manifest()?;
            if matches!(&filesystem, ArtifactPublicationFilesystemCheck::Live) {
                let disk = authority.resources.read_persisted_file_artifact_owned(manifest.layer.clone(), manifest.artifact_id)
                    .map_err(|_| ResourceAuthorityError::InvalidInput("artifact managed payload validation failed"))?;
                let expected_manifest = serde_json::to_value(&manifest)
                    .map_err(|_| ResourceAuthorityError::InvalidInput("artifact manifest serialization failed"))?;
                let actual_manifest = serde_json::to_value(disk.manifest())
                    .map_err(|_| ResourceAuthorityError::InvalidInput("artifact manifest serialization failed"))?;
                if actual_manifest != expected_manifest {
                    return Err(ResourceAuthorityError::InvalidInput("artifact managed manifest differs"));
                }
            }
        }
        Ok(())
    }

    pub(crate) fn workspace_artifact_publication_observation(
        &self, authority: &WorkspaceArtifactPublicationAuthorization,
    ) -> super::AuthenticatedTransactionContextObservation {
        self.authenticated_transaction_context_observation(&authority.operation_context)
    }

    pub(crate) async fn with_workspace_artifact_publication<T, E, F>(
        &self, authority: &std::sync::Arc<WorkspaceArtifactPublicationAuthorization>, operation: F,
    ) -> Result<T, E>
    where
        T: Send + 'static,
        E: From<SurrealStorageError> + From<ResourceAuthorityError> + Send + 'static,
        F: for<'tx> FnOnce(WorkspaceArtifactPublicationPermit<'tx>) -> super::SurrealTransactionOperation<'tx, T, E> + Send + 'static,
    {
        self.revalidate_workspace_artifact_publication_authorization(authority).await?;
        let worker_storage = self.clone();
        let worker_authority = authority.clone();
        let retention: std::sync::Arc<dyn std::any::Any + Send + Sync> = authority.clone();
        self.with_authenticated_transaction_context(
            authority.operation_context.clone(), retention, std::sync::Arc::clone(authority.publication_resources()),
            move |transaction| Box::pin(async move {
                worker_storage.revalidate_workspace_artifact_publication_in_transaction(&worker_authority, &transaction).await?;
                let result = operation(WorkspaceArtifactPublicationPermit { authorization: &worker_authority, transaction: &transaction }).await?;
                worker_storage.revalidate_workspace_artifact_publication_in_transaction(&worker_authority, &transaction).await?;
                Ok::<_, E>((result, worker_authority))
            }),
        ).await.map(|(result, _authority)| result)
    }

    async fn current_watcher_admission_read_facts(
        &self, retained: &std::sync::Arc<PalmistryWatcherAdoptionAuthority>,
        registry: &crate::capabilities::CapabilityRegistry, incoming: &RoleAuthorityAuthInput,
    ) -> Result<(ProducerSourceFacts, CanonicalResourceLocator), ResourceAuthorityError> {
        retained.require_current_binding(self)?;
        let original = &retained.capture.sources[0].decision;
        let denied = || ResourceAuthorityError::Denied { decision_id: original.decision_id.clone() };
        if incoming.channel_binding_hash != retained.capture.credentials.channel_binding_hash {
            return Err(denied());
        }
        let session = self.authenticate_local_session(&incoming.session_token, &incoming.channel_binding_hash).await?;
        if session.session_id != original.session_id
            || session.identity.account_id != original.account_id
            || session.identity.principal_id != original.principal_id
            || session.identity.access_space_id != original.access_space_id
            || session.actor_id != retained.capture.session.actor_id
            || session.actor_kind != retained.capture.session.actor_kind
            || session.principal_kind != retained.capture.session.principal_kind
        {
            return Err(denied());
        }
        let request = AuthorizationRequest {
            session_token: incoming.session_token.clone(),
            channel_binding_hash: Some(incoming.channel_binding_hash.clone()),
            capability_id: PALMISTRY_WATCHER_ADOPT_CAPABILITY.to_owned(),
            resource_kind: ResourceKind::PalmistryWatcher,
            external_resource_id: retained.watcher_id.to_string(),
            action: ResourceAction::Read,
        };
        let source = self.read_producer_source_facts(request, &session, registry).await?;
        let parent = self.canonical_watcher_workspace_parent(retained.watcher_id).await?.ok_or_else(denied)?;
        if source.authority.capability_profile_id != "Operator"
            || !registry.profile_can("Operator", PALMISTRY_WATCHER_ADOPT_CAPABILITY).unwrap_or(false)
            || !source.authority.capability_ids.iter().any(|id| id == PALMISTRY_WATCHER_ADOPT_CAPABILITY)
            || source.decision.session_id != original.session_id
            || source.decision.account_id != original.account_id
            || source.decision.principal_id != original.principal_id
            || source.decision.access_space_id != original.access_space_id
            || source.decision.resource_id != original.resource_id
            || parent.id != retained.parent.id
            || parent.external_resource_id != retained.exact.workspace_id.as_str()
            || parent.owner_account_id != source.authority.account_id
            || parent.access_space_id != source.authority.access_space_id
            || parent.lifecycle_state != "active"
            || source.authority.policy_versions.first().is_none_or(|policy| parent.policy_version > *policy)
        {
            return Err(denied());
        }
        Ok((source, parent))
    }

    pub(crate) async fn authorize_current_watcher_admission_read(
        &self, retained: &std::sync::Arc<PalmistryWatcherAdoptionAuthority>,
        registry: &crate::capabilities::CapabilityRegistry, incoming: RoleAuthorityAuthInput,
    ) -> Result<std::sync::Arc<PalmistryWatcherAdmissionReadAuthorization>, ResourceAuthorityError> {
        let (source, parent) = self.current_watcher_admission_read_facts(retained, registry, &incoming).await?;
        Ok(std::sync::Arc::new(PalmistryWatcherAdmissionReadAuthorization {
            retained: retained.clone(), incoming, registry: registry.clone(), source, parent,
        }))
    }

    pub(crate) async fn authorize_retained_watcher_admission_read(
        &self, retained: &std::sync::Arc<PalmistryWatcherAdoptionAuthority>,
    ) -> Result<std::sync::Arc<PalmistryWatcherAdmissionReadAuthorization>, ResourceAuthorityError> {
        self.authorize_current_watcher_admission_read(
            retained, &retained.capture.registry, retained.capture.credentials.clone(),
        ).await
    }

    pub(crate) async fn revalidate_watcher_admission_read_authorization(
        &self, read: &PalmistryWatcherAdmissionReadAuthorization,
    ) -> Result<(), ResourceAuthorityError> {
        let (source, parent) = self.current_watcher_admission_read_facts(&read.retained, &read.registry, &read.incoming).await?;
        if source.authority != read.source.authority || parent != read.parent {
            return Err(ResourceAuthorityError::InvalidInput("watcher admission current read authority changed"));
        }
        Ok(())
    }

    pub(crate) async fn with_current_watcher_admission_read<T, E, F>(
        &self, read: &std::sync::Arc<PalmistryWatcherAdmissionReadAuthorization>, operation: F,
    ) -> Result<T, E>
    where
        E: From<ResourceAuthorityError>,
        F: std::future::Future<Output = Result<T, E>>,
    {
        self.revalidate_watcher_admission_read_authorization(read).await?;
        let scope = RecordUserScope {
            grant_id: Some(read.source.decision.grant_id.clone()),
            workspace_id: Some(read.parent.external_resource_id.clone()),
            session_token: read.incoming.session_token.clone(),
            channel_binding_hash: Some(read.incoming.channel_binding_hash.clone()),
            resource_id: read.source.decision.resource_id.clone(),
            session_id: read.source.decision.session_id.clone(),
            capability_id: read.source.request.capability_id.clone(), action: read.source.request.action,
        };
        let result = self.with_record_user_scope(scope, operation).await?;
        self.revalidate_watcher_admission_read_authorization(read).await?;
        Ok(result)
    }

    pub(crate) async fn seal_palmistry_watcher_admission_authorization(
        &self, authority: &std::sync::Arc<PalmistryWatcherAdoptionAuthority>,
        requested_expires_at: chrono::DateTime<Utc>,
    ) -> Result<std::sync::Arc<PalmistryWatcherAdmissionAuthorization>, ResourceAuthorityError> {
        self.revalidate_palmistry_watcher_adoption_authority(authority).await?;
        let facts = &authority.capture.sources[0].authority;
        let mut expires_at = requested_expires_at.min(facts.session_expires_at.clone().into_inner());
        if let Some(grant_expiry) = &facts.grant_expires_at {
            expires_at = expires_at.min(grant_expiry.clone().into_inner());
        }
        let authorization = std::sync::Arc::new(PalmistryWatcherAdmissionAuthorization {
            authority: authority.clone(), expires_at,
        });
        authorization.validate_binding(authority, &authorization.expires_at)?;
        Ok(authorization)
    }

    pub(crate) async fn capture_palmistry_watcher_adoption_authority(
        &self, operation_id: Uuid, registry: &crate::capabilities::CapabilityRegistry,
        credentials: RoleAuthorityAuthInput, watcher_id: Uuid, canonical_workspace_parent: CanonicalResourceLocator,
    ) -> Result<std::sync::Arc<PalmistryWatcherAdoptionAuthority>, ResourceAuthorityError> {
        use crate::swarm_orchestration::resource_scope::{AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef,
            ExactResourceScopeAttribution, OwnerAccountId, WorkspaceScopeRef};
        if watcher_id.is_nil() { return Err(ResourceAuthorityError::InvalidInput("watcher UUID is required")); }
        let request = AuthorizationRequest {
            session_token: credentials.session_token.clone(), channel_binding_hash: Some(credentials.channel_binding_hash.clone()),
            capability_id: PALMISTRY_WATCHER_ADOPT_CAPABILITY.to_owned(), resource_kind: ResourceKind::PalmistryWatcher,
            external_resource_id: watcher_id.to_string(), action: ResourceAction::Reconcile,
        };
        let capture = self.capture_producer_source_facts(operation_id, registry, credentials, vec![request]).await?;
        let parent = self.canonical_watcher_workspace_parent(watcher_id).await?
            .ok_or(ResourceAuthorityError::InvalidInput("canonical watcher Workspace parent unavailable"))?;
        if parent != canonical_workspace_parent {
            return Err(ResourceAuthorityError::InvalidInput("watcher Workspace parent differs"));
        }
        let decision = &capture.sources[0].decision;
        let parse = |value: &str| Uuid::parse_str(value)
            .map_err(|_| ResourceAuthorityError::InvalidInput("invalid canonical watcher authority UUID"));
        let exact = ExactResourceScopeAttribution {
            owner_account_id: OwnerAccountId::from_uuid(parse(&decision.account_id)?),
            actor_principal_id: ActorPrincipalId::from_uuid(parse(&decision.principal_id)?),
            authenticated_session_id: AuthenticatedSessionRef::from_uuid(parse(&decision.session_id)?),
            access_space_id: AccessSpaceRef::from_uuid(parse(&decision.access_space_id)?),
            workspace_id: WorkspaceScopeRef::new(parent.external_resource_id.clone())
                .map_err(|_| ResourceAuthorityError::InvalidInput("invalid watcher Workspace parent"))?,
        };
        let authority = std::sync::Arc::new(PalmistryWatcherAdoptionAuthority {
            capture, watcher_id, parent, exact,
            namespace: self.config().namespace().to_owned(), database: self.config().database().to_owned(),
        });
        self.revalidate_palmistry_watcher_adoption_authority(&authority).await?;
        Ok(authority)
    }

    pub(crate) async fn with_palmistry_watcher_adoption_readback<T, U, E, F, P>(
        &self, authority: &std::sync::Arc<PalmistryWatcherAdoptionAuthority>, read: F, publish: P,
    ) -> Result<U, E>
    where
        E: From<ResourceAuthorityError>,
        F: std::future::Future<Output = Result<T, E>>,
        P: FnOnce(T) -> Result<U, E>,
    {
        authority.require_current_binding(self)?;
        self.revalidate_palmistry_watcher_adoption_authority(authority).await?;
        let observed = self.with_record_user_scope(authority.record_user_scope(), read).await?;
        self.revalidate_palmistry_watcher_adoption_authority(authority).await?;
        let source = &authority.capture.sources[0];
        CanonicalNavigationPublicationAuthority {
            grant_expires_at: source.authority.grant_expires_at.clone(),
            session_expires_at: source.authority.session_expires_at.clone(),
        }.require_unexpired()?;
        publish(observed)
    }

    pub(crate) async fn revalidate_palmistry_watcher_adoption_authority(
        &self, authority: &PalmistryWatcherAdoptionAuthority,
    ) -> Result<(), ResourceAuthorityError> {
        authority.require_current_binding(self)?;
        self.revalidate_producer_source_facts(&authority.capture).await?;
        let parent = self.canonical_watcher_workspace_parent(authority.watcher_id).await?
            .ok_or(ResourceAuthorityError::InvalidInput("watcher Workspace parent unavailable"))?;
        if parent != authority.parent {
            return Err(ResourceAuthorityError::InvalidInput("watcher Workspace parent changed"));
        }
        Ok(())
    }

    pub(crate) async fn capture_routing_original_sources(
        &self, store: &crate::swarm_orchestration::model_lane::ModelLaneStore,
        action: ResourceAction, capability: &str, run_id: String, execution_id: String,
        selecting_decision_id: Option<String>, input_ref: Option<String>, source_create: bool,
    ) -> Result<std::sync::Arc<RoutingOriginalSources>, ResourceAuthorityError> {
        let original = super::current_record_user_scope()
            .ok_or(ResourceAuthorityError::InvalidInput("routing original authority absent"))?;
        let subject = super::current_model_lane_subject()
            .ok_or(ResourceAuthorityError::InvalidInput("routing original subject absent"))?;
        let registry = super::configured_capability_registry()
            .ok_or(ResourceAuthorityError::InvalidInput("routing configured registry absent"))?;
        if !matches!((action, capability), (ResourceAction::Read, "fs.read" | "diagnostics.read")
                | (ResourceAction::Create | ResourceAction::Update, "fs.write"))
            || original.action != action || original.capability_id != capability || original.grant_id.is_none()
            || subject.kind != ResourceKind::Workspace
            || original.workspace_id.as_deref() != Some(subject.external_id.as_str())
            || (run_id.trim().is_empty() && execution_id.trim().is_empty())
            || (action == ResourceAction::Create && (run_id.trim().is_empty() || execution_id.trim().is_empty()))
        { return Err(ResourceAuthorityError::InvalidInput("routing original operation differs")); }
        let channel = original.channel_binding_hash.clone()
            .filter(|value| !value.is_empty()).ok_or(ResourceAuthorityError::InvalidInput("routing channel absent"))?;
        let credentials = RoleAuthorityAuthInput::new(original.session_token.clone(), channel);
        let request = AuthorizationRequest {
            session_token: original.session_token.clone(), channel_binding_hash: original.channel_binding_hash.clone(),
            capability_id: capability.to_owned(), resource_kind: ResourceKind::Workspace,
            external_resource_id: subject.external_id, action,
        };
        let mut requests = vec![request.clone()];
        let create_index = if action == ResourceAction::Create { Some(0) }
            else if source_create { let mut create = request; create.action = ResourceAction::Create;
                requests.push(create); Some(1) } else { None };
        let capture = std::sync::Arc::new(self.capture_producer_source_facts(
            Uuid::now_v7(), &registry, credentials, requests,
        ).await?);
        if capture.sources.iter().any(|source| source.authority.resource_id != capture.sources[0].authority.resource_id) {
            return Err(ResourceAuthorityError::InvalidInput("captured workspace operations differ in canonical resource"));
        }
        self.producer_source_run_authority(&capture, 0, store, action).await
            .map_err(|_| ResourceAuthorityError::InvalidInput("routing captured scope differs"))?;
        let sources = std::sync::Arc::new(RoutingOriginalSources {
            capture, storage: self.clone(), registry, original, run_id, execution_id, selecting_decision_id, input_ref, create_index, additional_create: Default::default(),
        });
        sources.revalidate().await?;
        Ok(sources)
    }

    /// Capture two independent live native requests for an already committed
    /// operation. This creates no role permit, evidence, receipt or new grant.
    pub(crate) async fn capture_committed_promotion_retry_sources(
        &self, bootstrap: &crate::AccountBootstrapState,
        store: &crate::swarm_orchestration::model_lane::ModelLaneStore,
        request: AuthorizationRequest, operation_id: Uuid, run_id: String,
    ) -> Result<std::sync::Arc<RoutingOriginalSources>, ResourceAuthorityError> {
        if self.resource_access_lifecycle() != bootstrap.surreal.resource_access_lifecycle()
            || request.action != ResourceAction::Create || request.capability_id != "fs.write"
            || request.resource_kind != ResourceKind::Workspace || operation_id.is_nil() || run_id.trim().is_empty() {
            return Err(ResourceAuthorityError::InvalidInput("committed promotion retry requires original native Workspace Create"));
        }
        let channel = request.channel_binding_hash.clone().filter(|value| !value.is_empty())
            .ok_or(ResourceAuthorityError::InvalidInput("committed promotion retry requires authenticated channel"))?;
        let credentials = RoleAuthorityAuthInput::new(request.session_token.clone(), channel);
        let mut read = request.clone(); read.action = ResourceAction::Read; read.capability_id = "fs.read".into();
        let capture = std::sync::Arc::new(self.capture_producer_source_facts(operation_id,
            &bootstrap.capability_registry, credentials, vec![read, request]).await?);
        if capture.sources.len() != 2 || capture.sources[0].authority.resource_id != capture.sources[1].authority.resource_id {
            return Err(ResourceAuthorityError::InvalidInput("committed promotion retry Creator and Read resources differ"));
        }
        self.producer_source_run_authority(&capture, 0, store, ResourceAction::Read).await
            .map_err(|_| ResourceAuthorityError::InvalidInput("committed promotion retry exact scope differs"))?;
        self.producer_source_run_authority(&capture, 1, store, ResourceAction::Create).await
            .map_err(|_| ResourceAuthorityError::InvalidInput("committed promotion retry Creator scope differs"))?;
        let read = &capture.sources[0];
        let original = RecordUserScope {
            grant_id: Some(read.decision.grant_id.clone()), workspace_id: Some(read.request.external_resource_id.clone()),
            session_token: read.request.session_token.clone(), channel_binding_hash: read.request.channel_binding_hash.clone(),
            resource_id: read.decision.resource_id.clone(), session_id: read.decision.session_id.clone(),
            capability_id: "fs.read".into(), action: ResourceAction::Read,
        };
        let sources = std::sync::Arc::new(RoutingOriginalSources {
            capture, storage: self.clone(), registry: bootstrap.capability_registry.clone(), original,
            run_id, execution_id: String::new(), selecting_decision_id: None, input_ref: None,
            create_index: Some(1), additional_create: Default::default(),
        });
        sources.revalidate().await?;
        Ok(sources)
    }

    pub(crate) async fn capture_run_attachment_sources(
        &self, store: &crate::swarm_orchestration::model_lane::ModelLaneStore, run_id: String,
    ) -> Result<std::sync::Arc<RoutingOriginalSources>, ResourceAuthorityError> {
        self.capture_create_update_sources(store, run_id, String::new(), None, None).await
    }

    pub(crate) async fn capture_routing_lifecycle_sources(
        &self, store: &crate::swarm_orchestration::model_lane::ModelLaneStore,
        run_id: String, execution_id: String, decision_id: String, input_ref: String,
    ) -> Result<std::sync::Arc<RoutingOriginalSources>, ResourceAuthorityError> {
        if execution_id.trim().is_empty() || decision_id.trim().is_empty() || input_ref.trim().is_empty() {
            return Err(ResourceAuthorityError::InvalidInput("routing lifecycle canonical selectors absent"));
        }
        self.capture_create_update_sources(store, run_id, execution_id, Some(decision_id), Some(input_ref)).await
    }

    async fn capture_create_update_sources(
        &self, store: &crate::swarm_orchestration::model_lane::ModelLaneStore,
        run_id: String, execution_id: String, selecting_decision_id: Option<String>, input_ref: Option<String>,
    ) -> Result<std::sync::Arc<RoutingOriginalSources>, ResourceAuthorityError> {
        let original = super::current_record_user_scope()
            .ok_or(ResourceAuthorityError::InvalidInput("run attachment original Create absent"))?;
        let subject = super::current_model_lane_subject()
            .ok_or(ResourceAuthorityError::InvalidInput("run attachment workspace absent"))?;
        let registry = super::configured_capability_registry()
            .ok_or(ResourceAuthorityError::InvalidInput("run attachment configured registry absent"))?;
        if original.action != ResourceAction::Create || original.capability_id != "fs.write"
            || original.grant_id.is_none() || subject.kind != ResourceKind::Workspace || run_id.trim().is_empty()
            || original.workspace_id.as_deref() != Some(subject.external_id.as_str()) {
            return Err(ResourceAuthorityError::InvalidInput("run attachment original operation differs"));
        }
        let channel = original.channel_binding_hash.clone().filter(|value| !value.is_empty())
            .ok_or(ResourceAuthorityError::InvalidInput("run attachment channel absent"))?;
        let request = AuthorizationRequest {
            session_token: original.session_token.clone(), channel_binding_hash: Some(channel.clone()),
            capability_id: "fs.write".to_owned(), resource_kind: ResourceKind::Workspace,
            external_resource_id: subject.external_id, action: ResourceAction::Create,
        };
        let mut update = request.clone(); update.action = ResourceAction::Update;
        let capture = std::sync::Arc::new(self.capture_producer_source_facts(Uuid::now_v7(), &registry,
            RoleAuthorityAuthInput::new(original.session_token.clone(), channel), vec![request, update]).await?);
        if capture.sources.iter().any(|source| source.authority.resource_id != capture.sources[0].authority.resource_id) {
            return Err(ResourceAuthorityError::InvalidInput("captured workspace operations differ in canonical resource"));
        }
        self.producer_source_run_authority(&capture, 0, store, ResourceAction::Create).await
            .map_err(|_| ResourceAuthorityError::InvalidInput("run attachment exact scope differs"))?;
        let sources = std::sync::Arc::new(RoutingOriginalSources {
            capture, storage: self.clone(), registry, original, run_id, execution_id,
            selecting_decision_id, input_ref, create_index: Some(0), additional_create: Default::default(),
        });
        sources.revalidate().await?;
        Ok(sources)
    }

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
    ) -> ProducerRunCompletion {
        let operation_id = capture.operation_id;
        let owner: std::sync::Arc<dyn std::any::Any + Send + Sync> = capture.clone();
        let (scope, subject) = match self.producer_source_run_authority(
            capture, source_index, &store, ResourceAction::Create,
        ).await {
            Ok(authority) => authority,
            Err(error) => return ProducerRunCompletion {
                operation_id, result: Err(ProducerRunFailure {
                    original_operation_id: operation_id, origin: ProducerRunFailureOrigin::Preflight(error),
                }), finalization: None, owner,
            },
        };
        // The original fixture owner keeps its Arc through unresolved settlement.
        // The worker clone keeps the same fence while its callback is running.
        let capture = std::sync::Arc::clone(capture);
        let worker_storage = self.clone();
        let outcome = super::with_model_lane_subject(subject, self.with_record_user_scope(
            scope,
            self.with_original_runcreate_transaction(operation_id, owner.clone(), move |transaction| Box::pin(async move {
                let _transaction = transaction;
                worker_storage.revalidate_producer_source_facts(&capture).await
                    .map_err(|error| ProducerRunFailureOrigin::OriginalCallback(error.into()))?;
                let run = store.record_run(input).await
                    .map_err(ProducerRunFailureOrigin::OriginalCallback)?;
                worker_storage.revalidate_producer_source_facts(&capture).await
                    .map_err(|error| ProducerRunFailureOrigin::OriginalCallback(error.into()))?;
                Ok::<_, ProducerRunFailureOrigin>((run, capture))
            })),
        )).await;
        let (result, finalization) = outcome.into_parts();
        ProducerRunCompletion {
            operation_id, owner, finalization,
            result: result.map(|(run, _capture)| run).map_err(|origin| ProducerRunFailure {
                original_operation_id: operation_id, origin,
            }),
        }
    }

    fn acknowledge_producer_run_completion(
        &self, capture: &std::sync::Arc<ProducerSourceFactsCapture>, completion: &mut ProducerRunCompletion,
    ) -> Result<bool, SurrealStorageError> {
        let Some(owner) = completion.owner.as_ref().downcast_ref::<ProducerSourceFactsCapture>() else {
            return Ok(false);
        };
        if completion.operation_id != capture.operation_id || !std::ptr::eq(owner, capture.as_ref())
            || completion.result.as_ref().err().is_some_and(|failure| failure.original_operation_id != capture.operation_id)
        {
            return Ok(false);
        }
        let expected = if completion.result.is_ok() {
            super::OriginalRunCreateFinalization::Commit
        } else {
            super::OriginalRunCreateFinalization::Rollback
        };
        let Some(proof) = completion.finalization.as_ref() else { return Ok(false); };
        if proof.finalization() != expected || !proof.matches_original(self, capture.operation_id, &completion.owner) {
            return Ok(false);
        }
        let Some(proof) = completion.finalization.take() else { return Ok(false); };
        match self.acknowledge_original_runcreate_finalization(proof, &completion.owner) {
            Ok(()) => Ok(true),
            Err((error, proof)) => {
                completion.finalization = Some(proof);
                Err(error)
            }
        }
    }

    async fn replay_producer_source_run(
        &self,
        capture: &std::sync::Arc<ProducerSourceFactsCapture>,
        source_index: usize,
        store: &crate::swarm_orchestration::model_lane::ModelLaneStore,
        expected: &crate::swarm_orchestration::model_lane::NewModelLaneRun,
    ) -> crate::swarm_orchestration::model_lane::ModelLaneResult<crate::swarm_orchestration::model_lane::ModelLaneRunCanonicalSet> {
        let (scope, subject) = self.producer_source_run_authority(
            capture, source_index, store, ResourceAction::Read,
        ).await?;
        let canonical = super::with_model_lane_subject(subject,
            self.with_record_user_scope(scope, store.original_run_create_canonical_set(expected)),
        ).await?;
        self.revalidate_producer_source_facts(capture).await?;
        Ok(canonical)
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

async fn with_sealed_role_creation<T, F>(
    record_kind: &str, record_id: &str, event_id: &str, record_sha256: &str, operation_id: Uuid,
    issuer_scope: &crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution, expected_child_read: Option<RegistryPublicationAuthority>, expected_owner_read: Option<RegistryPublicationAuthority>, retained: &std::sync::Mutex<Vec<ProtectedRoleCreationBinding>>, future: F,
) -> Result<T, SurrealStorageError>
where F: std::future::Future<Output = Result<T, SurrealStorageError>> {
    if record_id.is_empty() || event_id.is_empty() || record_sha256.len() != 64 || !record_sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(SurrealStorageError::TransactionAuthorityRequired);
    }
    let binding = ProtectedRoleCreationBinding {
        record_kind: record_kind.to_owned(), record_id: record_id.to_owned(), event_id: event_id.to_owned(),
        record_sha256: record_sha256.to_owned(), operation_id: operation_id.to_string(), issuer_scope: issuer_scope.into(), expected_child_read, expected_owner_read,
    };
    let result = PROTECTED_ROLE_CREATION_BINDING.scope(binding.clone(), future).await?;
    let mut records = retained.lock().map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?;
    if records.iter().any(|prior| prior.record_kind == binding.record_kind && prior.record_id == binding.record_id && prior != &binding) {
        return Err(SurrealStorageError::TransactionAuthorityRequired);
    }
    if !records.contains(&binding) { records.push(binding); }
    Ok(result)
}

#[derive(Clone, PartialEq, Eq, SurrealValue)]
struct ProtectedRoleScopeBinding {
    owner_account_id: String, actor_principal_id: String, authenticated_session_id: String,
    access_space_id: String, workspace_id: String,
}
impl From<&crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution> for ProtectedRoleScopeBinding {
    fn from(scope: &crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution) -> Self {
        Self { owner_account_id: scope.owner_account_id.to_string(), actor_principal_id: scope.actor_principal_id.to_string(),
            authenticated_session_id: scope.authenticated_session_id.to_string(), access_space_id: scope.access_space_id.to_string(),
            workspace_id: scope.workspace_id.as_str().to_owned() }
    }
}
#[derive(Clone, SurrealValue)]
struct ProtectedRoleCaptureBinding {
    record_kind: String,
    event_id: String,
    operation_id: String,
    consumer_operation_id: String,
    issuer_scope: ProtectedRoleScopeBinding,
    exact_consumer_scope: ProtectedRoleScopeBinding,
    expected_owner_read: RegistryPublicationAuthority,
    expected_child_read: RegistryPublicationAuthority,
    operator_ref: String,
    designation_ref: String,
    selection_json: String,
}

#[derive(Clone, SurrealValue)]
struct ProtectedRoleConsumptionBinding { binding_event_id: String, evidence_event_id: String, consumer_operation_id: String, expected_child_read: Option<RegistryPublicationAuthority>, expected_owner_read: Option<RegistryPublicationAuthority> }
#[derive(Clone, PartialEq, Eq, SurrealValue)]
struct ProtectedRoleCreationBinding {
    record_kind: String, record_id: String, event_id: String, record_sha256: String,
    operation_id: String, issuer_scope: ProtectedRoleScopeBinding, expected_child_read: Option<RegistryPublicationAuthority>, expected_owner_read: Option<RegistryPublicationAuthority>,
}
#[derive(Clone, SurrealValue)]
struct RoutingPriorReadBinding {
    operation_id: String, execution_id: String, stage_id: String, run_id: String, event_id: String,
}

#[derive(Clone, SurrealValue)]
struct RoutingAckBinding {
    operation_id: String, acknowledgement_event_id: String, prior_event_id: String,
    execution_id: String, stage_id: String, message_id: String, attempt: i64,
    lease_owner: String, fencing_token: String, lease_expires_at_unix_ms: i64,
    observed_revision: i64, observed_execution_event_id: String, observed_attempt_event_id: String,
}

#[derive(Clone, SurrealValue)]
struct FinalPromotionBinding {
    operation_id: String, decision_id: String, run_id: String,
    input_json: String, selectors_json: String,
    protected_binding_event_id: String, initial_consumer_operation_id: String,
    initial_validator_event_id: String, initial_validator_receipt_json: String,
    creator: RegistryPublicationAuthority, owner_read: RegistryPublicationAuthority,
    child_read: RegistryPublicationAuthority,
    prepared_record_json: Option<String>, operator_receipt_json: Option<String>,
    validator_receipt_json: Option<String>,
    current_consumption_event_id: Option<String>, current_consumption_body_json: Option<String>,
}

tokio::task_local! {
    static FINAL_PROMOTION_BINDING: FinalPromotionBinding;
    static ROUTING_ACK_BINDING: RoutingAckBinding;
    static ROUTING_PRIOR_READ_BINDING: RoutingPriorReadBinding;
    static PROMOTION_READ_CUSTODY: std::sync::Arc<InitialValidatorIssuance>;
    static PROTECTED_ROLE_CONSUMPTION_BINDING: ProtectedRoleConsumptionBinding;
    static PROTECTED_ROLE_CREATION_BINDING: ProtectedRoleCreationBinding;
    static PROTECTED_ROLE_CAPTURE_BINDING: ProtectedRoleCaptureBinding;
}
#[cfg(feature = "test-utils")]
tokio::task_local! { static TEST_PROMOTION_WRONG_CURRENT_OPERATION: bool; }
#[cfg(feature = "test-utils")]
pub(crate) async fn with_wrong_current_promotion_operation<T>(operation: impl std::future::Future<Output = T>) -> T {
    TEST_PROMOTION_WRONG_CURRENT_OPERATION.scope(true, operation).await
}

pub(crate) async fn with_promotion_read_custody<T>(
    custody: std::sync::Arc<InitialValidatorIssuance>, operation: impl std::future::Future<Output = T>,
) -> T { PROMOTION_READ_CUSTODY.scope(custody, operation).await }

pub(super) fn append_protected_role_bindings(bindings: &mut surrealdb::types::Value) {
    if !matches!(bindings, surrealdb::types::Value::Object(_)) {
        *bindings = surrealdb::types::Value::Object(surrealdb::types::Object::new());
    }
    if let surrealdb::types::Value::Object(object) = bindings {
        object.insert("wp012_routing_original", current_routing_original_sources().map(|sources| sources.binding()));
        object.insert("wp012_promotion_original", FINAL_PROMOTION_BINDING.try_with(Clone::clone).ok());
        object.insert("wp012_routing_ack", ROUTING_ACK_BINDING.try_with(Clone::clone).ok());
        object.insert("wp012_routing_prior", ROUTING_PRIOR_READ_BINDING.try_with(Clone::clone).ok());
        object.insert("wp012_role_evidence_consume", PROTECTED_ROLE_CONSUMPTION_BINDING.try_with(Clone::clone).ok());
        object.insert("wp012_role_evidence_create", PROTECTED_ROLE_CREATION_BINDING.try_with(Clone::clone).ok());
        object.insert("wp012_role_evidence_capture", PROTECTED_ROLE_CAPTURE_BINDING.try_with(Clone::clone).ok());
    }
    append_validator_association_binding(bindings);
}

#[derive(Clone)]
pub(crate) struct RoleAuthorityAuthInput {
    session_token: String,
    channel_binding_hash: String,
}

pub(crate) struct WorkspaceOwnerActionPermit {
    auth: OwnedCanonicalAuthSnapshot,
    role_evidence_read: Option<OwnedCanonicalAuthSnapshot>,
    registry: std::sync::Arc<crate::capabilities::CapabilityRegistry>,
    command: crate::swarm_orchestration::model_lane::WorkspaceOwnerActionCommand,
}

pub(crate) struct RoleAuthorityConsumptionPermit {
    capture: RoleConsumptionCapture,
    preparation: Option<RoleAuthorityTransactionRetention>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct ProtectedRoleReadAuthority {
    pub(crate) grant_id: String,
    pub(crate) grant_version: i64,
    pub(crate) resource_id: String,
    pub(crate) session_id: String,
    pub(crate) capability_id: String,
    pub(crate) action: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct ProtectedRoleEvidenceTarget {
    pub(crate) record_kind: String,
    pub(crate) record_id: String,
    pub(crate) event_id: String,
    pub(crate) record_sha256: String,
    pub(crate) issuer_scope: crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct ProtectedRoleEvidenceConsumptionBody {
    pub(crate) schema_id: String,
    pub(crate) operation_id: Uuid,
    pub(crate) consumer_operation_id: Uuid,
    pub(crate) issuer_scope: crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution,
    pub(crate) exact_consumer_scope: crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution,
    pub(crate) owner_read: ProtectedRoleReadAuthority,
    pub(crate) child_read: ProtectedRoleReadAuthority,
    pub(crate) designation_ref: handshake_role_authority::domain::DesignationRef,
    pub(crate) producer: crate::swarm_orchestration::model_lane::ValidatorRunnerRegistrationBinding,
    pub(crate) run: handshake_role_authority::domain::RunBinding,
    pub(crate) subject: handshake_role_authority::domain::ArtifactSubject,
    pub(crate) expected: handshake_role_authority::domain::ExpectedBase,
    pub(crate) purpose: handshake_role_authority::domain::Purpose,
    pub(crate) targets: Vec<ProtectedRoleEvidenceTarget>,
}
#[derive(Clone, Debug, Serialize)]
pub(crate) struct CurrentPromotionConsumptionBody {
    pub(crate) schema_id: String,
    pub(crate) operation_id: Uuid,
    pub(crate) exact_consumer_scope: crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution,
    pub(crate) decision_id: String,
    pub(crate) input_json: String,
    pub(crate) selectors_json: String,
    pub(crate) creator: ProtectedRoleReadAuthority,
    pub(crate) owner_read: ProtectedRoleReadAuthority,
    pub(crate) child_read: ProtectedRoleReadAuthority,
    pub(crate) owner_read_scope: crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution,
    pub(crate) child_read_scope: crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution,
    pub(crate) lineage_binding_event_id: String,
    pub(crate) initial_consumer_operation_id: Uuid,
    pub(crate) producer: crate::swarm_orchestration::model_lane::ValidatorRunnerRegistrationBinding,
    pub(crate) designation_ref: handshake_role_authority::domain::DesignationRef,
    pub(crate) run: handshake_role_authority::domain::RunBinding,
    pub(crate) subject: handshake_role_authority::domain::ArtifactSubject,
    pub(crate) targets: Vec<ProtectedRoleEvidenceTarget>,
}

struct CurrentPromotionConsumption {
    body: CurrentPromotionConsumptionBody,
    event_id: String,
}

pub(crate) struct InitialValidatorIssuance {
    storage: SurrealStorage,
    issued_receipt: std::sync::OnceLock<handshake_role_authority::domain::RoleReceipt>,
    protected_publication_operation_id: Uuid,
    protected_consumption: std::sync::OnceLock<PublishedProtectedRoleConsumption>,
    owner_read: OwnedCanonicalAuthSnapshot,
    child_read: OwnedCanonicalAuthSnapshot,
    coordinator: std::sync::Arc<crate::swarm_orchestration::SwarmCoordinator>,
    producer: std::sync::Arc<crate::swarm_orchestration::coordinator::ValidatorProducerKey>,
    observed: crate::swarm_orchestration::coordinator::RegisteredValidatorReviewOutput,
    operator: crate::swarm_orchestration::model_lane::RoleEvidenceConsumeSelector,
    verdict: handshake_role_authority::domain::RecordValidatorVerdict,
    operation_id: Uuid,
}

impl std::fmt::Debug for InitialValidatorIssuance {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("InitialValidatorIssuance")
    }
}

impl InitialValidatorIssuance {
    pub(crate) fn operator_prior(&self) -> &crate::swarm_orchestration::model_lane::RoleEvidenceConsumeSelector { &self.operator }
    pub(crate) fn verdict(&self) -> &handshake_role_authority::domain::RecordValidatorVerdict { &self.verdict }
    pub(crate) fn output(&self) -> &str { self.observed.output() }
    pub(crate) fn producer_binding(&self) -> crate::swarm_orchestration::model_lane::ValidatorRunnerRegistrationBinding {
        crate::swarm_orchestration::model_lane::ValidatorRunnerRegistrationBinding {
            producer_registration_ref: self.producer.registration_ref().to_owned(),
            producer_instance_ref: self.producer.instance_id().to_string(),
            producer_spawn_generation: self.producer.spawn_generation(),
            producer_registered_principal: self.producer.registered_principal().clone(),
        }
    }
    pub(crate) fn delegating_scope(&self) -> &crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution {
        self.producer.delegating_scope()
    }
    async fn revalidate(&self, storage: &SurrealStorage) -> Result<(), ResourceAuthorityError> {
        self.producer.revalidate(storage).await?;
        let registry = &self.producer.authorization_seal().registry;
        self.owner_read.revalidate_for_publication(storage, registry).await?;
        self.child_read.revalidate_for_publication(storage, registry).await?;
        self.require_current_registry()
    }
    fn require_current_registry(&self) -> Result<(), ResourceAuthorityError> {
        self.coordinator.with_current_validator_producer(&self.producer, Some(self.observed.terminal()), || Ok(()))
            .map_err(|_| ResourceAuthorityError::InvalidInput("initial Validator producer or terminal is stale"))?;
        Ok(())
    }
}

impl SurrealStorage {
    pub(crate) async fn read_owner_model_lane_replay(
        &self, bootstrap: &crate::AccountBootstrapState, credentials: RoleAuthorityAuthInput,
        store: &crate::swarm_orchestration::model_lane::ModelLaneStore, run_id: &str,
    ) -> crate::swarm_orchestration::model_lane::ModelLaneResult<crate::swarm_orchestration::model_lane::ModelLaneReplay> {
        let exact = store.access().exact_read_scope().ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
        let mut request = credentials.workspace_creation_request(exact.workspace_id.as_str());
        request.action = ResourceAction::Read; request.capability_id = "fs.read".to_owned();
        let read = std::sync::Arc::new(capture_owned_canonical_role_auth(
            self, &bootstrap.capability_registry, credentials, request).await?);
        if &read.exact != exact { return Err(SurrealStorageError::TransactionAuthorityRequired.into()); }
        let _fence = self.authority_publication_read().await?;
        read.revalidate_for_publication(self, &bootstrap.capability_registry).await?;
        let worker_read = read.clone(); let worker_registry = bootstrap.capability_registry.clone();
        let worker_store = store.clone(); let selected = run_id.to_owned();
        let result = self.with_record_user_scope(read.record_user_scope(),
            super::with_model_lane_subject(super::ModelLaneOperationSubject {
                kind: ResourceKind::Workspace, external_id: exact.workspace_id.to_string(),
            }, super::with_configured_capability_registry(bootstrap.capability_registry.clone(),
                super::with_model_lane_operation(super::ContextCoordinationPurpose::SourceRead,
                    self.with_authenticated_transaction(Uuid::now_v7(), move |transaction| Box::pin(async move {
                        worker_read.revalidate_in_transaction(&transaction, &worker_registry).await?;
                        let result = worker_store.replay_run(&selected).await?;
                        worker_read.revalidate_in_transaction(&transaction, &worker_registry).await?;
                        Ok::<_, crate::swarm_orchestration::model_lane::ModelLaneError>(result)
                    })))))).await?;
        read.revalidate_for_publication(self, &bootstrap.capability_registry).await?;
        Ok(result)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn capture_initial_validator_issuance(
        &self, bootstrap: &crate::AccountBootstrapState,
        owner_credentials: RoleAuthorityAuthInput,
        coordinator: std::sync::Arc<crate::swarm_orchestration::SwarmCoordinator>,
        producer: std::sync::Arc<crate::swarm_orchestration::coordinator::ValidatorProducerKey>,
        observed: crate::swarm_orchestration::coordinator::RegisteredValidatorReviewOutput,
        operator: crate::swarm_orchestration::model_lane::RoleEvidenceConsumeSelector,
        verdict: handshake_role_authority::domain::RecordValidatorVerdict, operation_id: Uuid,
    ) -> Result<std::sync::Arc<InitialValidatorIssuance>, ResourceAuthorityError> {
        use handshake_role_authority::domain::{Role, Purpose};
        let terminal = observed.terminal();
        let artifact_bytes = serde_json::to_vec(&serde_json::Value::String(observed.output().to_owned()))
            .map_err(|_| ResourceAuthorityError::InvalidInput("Validator output cannot be encoded as canonical artifact text"))?;
        if operation_id.is_nil() || bootstrap.surreal.resource_access_lifecycle() != self.resource_access_lifecycle()
            || operator.expectation.role != Role::Operator || operator.expectation.purpose != Purpose::PriorSelection
            || operator.designation_ref.is_some() || operator.producer.is_some()
            || operator.expectation.run != verdict.run || operator.expectation.subject != verdict.subject
            || operator.expectation.expected != verdict.expected || verdict.purpose != Purpose::PriorSelection
            || operator.expectation.receipt_scope.owner_account_id != verdict.scope.owner_account_id
            || operator.expectation.receipt_scope.workspace_id != verdict.scope.workspace_id
            || &verdict.scope != producer.exact_scope()
            || &verdict.execution.runner_principal != producer.registered_principal()
            || terminal.run_id() != verdict.run.run_id || terminal.session_id() != verdict.run.coordinator_session_id
            || terminal.generated_tokens() == 0 || observed.output().is_empty()
            || verdict.execution.invocation_ref != terminal.trace_id().to_string()
            || verdict.execution.verdict_artifact.sha256.to_hex() != sha256_hex(&artifact_bytes)
        { return Err(ResourceAuthorityError::InvalidInput("initial Validator execution binding differs")); }
        handshake_role_authority::canonical::prepare_validator_verdict_declared(&verdict)
            .map_err(|_| ResourceAuthorityError::InvalidInput("initial Validator intent is invalid"))?;
        let read_request = |credentials: &RoleAuthorityAuthInput| AuthorizationRequest {
            session_token: credentials.session_token.clone(), channel_binding_hash: Some(credentials.channel_binding_hash.clone()),
            resource_kind: ResourceKind::Workspace, external_resource_id: verdict.scope.workspace_id.as_str().to_owned(),
            capability_id: "fs.read".to_owned(), action: ResourceAction::Read,
        };
        let mut owner_read = capture_owned_canonical_role_auth(self, &bootstrap.capability_registry,
            owner_credentials.clone(), read_request(&owner_credentials)).await?;
        owner_read.requires_workspace_owner = true;
        owner_read.revalidate_for_publication(self, &bootstrap.capability_registry).await?;
        let child_credentials = producer.authorization_seal().auth.credentials.clone();
        let child_read = capture_owned_canonical_role_auth(self, &bootstrap.capability_registry,
            child_credentials.clone(), read_request(&child_credentials)).await?;
        if &owner_read.exact != producer.delegating_scope() || &child_read.exact != producer.exact_scope()
            || owner_read.authority.resource_id != child_read.authority.resource_id {
            return Err(ResourceAuthorityError::InvalidInput("protected role READ authority differs from registered Owner or child"));
        }
        let issuance = std::sync::Arc::new(InitialValidatorIssuance {
            storage: self.clone(),
            issued_receipt: std::sync::OnceLock::new(),
            protected_publication_operation_id: Uuid::now_v7(),
            protected_consumption: std::sync::OnceLock::new(),
            owner_read, child_read, coordinator, producer, observed, operator, verdict, operation_id,
        });
        issuance.revalidate(self).await?;
        Ok(issuance)
    }

    async fn publish_protected_role_evidence_consumption(
        &self, bootstrap: &crate::AccountBootstrapState, issuance: std::sync::Arc<InitialValidatorIssuance>,
    ) -> Result<(), ResourceAuthorityError> {
        if issuance.protected_consumption.get().is_some() {
            issuance.revalidate(self).await?;
            return Ok(());
        }
        let fence = std::sync::Arc::new(self.authority_publication_read().await?);
        issuance.revalidate(self).await?;
        let retention = RoleAuthorityTransactionRetention {
            owner: std::sync::Arc::new(OwnedRoleTransaction::ProtectedRoleConsumption(OwnedProtectedRoleConsumption {
                operation_id: issuance.protected_publication_operation_id, issuance: issuance.clone(), registry: bootstrap.capability_registry.as_ref().clone(),
                body: std::sync::OnceLock::new(),
            })), fence,
        };
        let provider = super::model_lane::SurrealModelLaneStore::initialize(self.clone()).await?;
        let worker_provider = provider.clone();
        let worker_storage = self.clone();
        let worker_retention = retention.clone();
        let scope = issuance.owner_read.record_user_scope();
        let committed = self.with_record_user_scope(scope,
            self.with_owned_role_authenticated_transaction(retention.operation_id(), super::ContextOperationCancellation::new(), retention.clone(),
                move |transaction| Box::pin(async move {
                    let OwnedRoleTransaction::ProtectedRoleConsumption(capture) = worker_retention.owner.as_ref() else {
                        return Err(ResourceAuthorityError::InvalidInput("protected role publication custody differs"));
                    };
                    capture.issuance.revalidate(&worker_storage).await?;
                    capture.issuance.owner_read.revalidate_in_transaction(&transaction, &capture.registry).await?;
                    let target_seal = ProtectedRoleEvidenceTargetReadSeal { capture, transaction: &transaction };
                    let targets = worker_provider.capture_protected_role_evidence_targets(&transaction, &target_seal).await?;
                    let read = |auth: &OwnedCanonicalAuthSnapshot| ProtectedRoleReadAuthority {
                        grant_id: auth.decision.grant_id.clone(), grant_version: auth.authority.grant_version,
                        resource_id: auth.decision.resource_id.clone(), session_id: auth.decision.session_id.clone(),
                        capability_id: auth.capability_id.clone(), action: auth.action.as_str().to_owned(),
                    };
                    let issuance = &capture.issuance;
                    capture.body.set(ProtectedRoleEvidenceConsumptionBody {
                        schema_id: "handshake.role-evidence-consumption@1".to_owned(),
                        operation_id: capture.operation_id, consumer_operation_id: issuance.operation_id,
                        issuer_scope: issuance.owner_read.exact.clone(), exact_consumer_scope: issuance.child_read.exact.clone(),
                        owner_read: read(&issuance.owner_read), child_read: read(&issuance.child_read),
                        designation_ref: issuance.verdict.designation_ref.clone(), producer: issuance.producer_binding(),
                        run: issuance.verdict.run.clone(), subject: issuance.verdict.subject.clone(), expected: issuance.verdict.expected.clone(),
                        purpose: issuance.verdict.purpose.clone(), targets,
                    }).map_err(|_| ResourceAuthorityError::InvalidInput("protected role body was already captured"))?;
                    let seal = ProtectedRoleEvidenceCreationSeal { capture, transaction: &transaction };
                    let committed = worker_provider.put_protected_role_evidence_consumption(&transaction, &seal).await?;
                    capture.issuance.owner_read.revalidate_in_transaction(&transaction, &capture.registry).await?;
                    capture.issuance.require_current_registry()?;
                    Ok::<_, ResourceAuthorityError>(committed)
                }),
            ),
        ).await?;
        issuance.revalidate(self).await?;
        let guard = RolePublicationGuard { owner: &retention.owner, _fence: &retention.fence };
        let published = self.with_record_user_scope(issuance.owner_read.record_user_scope(),
            provider.readback_protected_role_evidence_consumption(&guard, &committed)).await?;
        let OwnedRoleTransaction::ProtectedRoleConsumption(capture) = retention.owner.as_ref() else { unreachable!() };
        let body = capture.body.get().ok_or(ResourceAuthorityError::InvalidInput("protected body absent after commit"))?.clone();
        issuance.revalidate(self).await?;
        self.complete_role_publication(RoleAuthorityPublicationCompletion { operation_id: retention.operation_id(), owner: retention.owner.clone() })?;
        issuance.protected_consumption.set(PublishedProtectedRoleConsumption {
            body, event_id: published.binding_event_id().to_owned(),
        }).map_err(|_| ResourceAuthorityError::InvalidInput("protected role publication already completed"))?;
        Ok(())
    }

    pub(crate) async fn consume_initial_validator_issuance(
        &self, bootstrap: &crate::AccountBootstrapState, issuance: std::sync::Arc<InitialValidatorIssuance>,
        store: crate::swarm_orchestration::model_lane::ModelLaneStore,
    ) -> crate::swarm_orchestration::model_lane::ModelLaneResult<handshake_role_authority::domain::RoleReceipt> {
        use crate::swarm_orchestration::model_lane::{RoleConsumptionCommand, RoleConsumptionResult, ModelLaneError};
        issuance.revalidate(self).await?;
        self.publish_protected_role_evidence_consumption(bootstrap, issuance.clone()).await?;
        let credentials = issuance.producer.authorization_seal().auth.credentials.clone();
        let request = credentials.workspace_creation_request(issuance.verdict.scope.workspace_id.as_str());
        let command = RoleConsumptionCommand::IssueInitialValidatorVerdict {
            operation_id: issuance.operation_id, operator: issuance.operator.clone(), issuance: issuance.clone(),
        };
        let permit = self.capture_role_authority_consumption(bootstrap, credentials, request, command).await?;
        match self.execute_role_consumption(permit, store).await? {
            RoleConsumptionResult::ReviewCompletion(receipt) => {
                if issuance.issued_receipt.get().is_some_and(|prior| prior != &receipt) {
                    return Err(ModelLaneError::IntegrityViolation("initial Validator issuance receipt changed".into()));
                }
                let _ = issuance.issued_receipt.set(receipt.clone());
                Ok(receipt)
            },
            _ => Err(ModelLaneError::IntegrityViolation("initial Validator issuance returned another result".into())),
        }
    }
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
    promotion_read: Option<std::sync::Arc<InitialValidatorIssuance>>,
    promotion_consumption: std::sync::OnceLock<CurrentPromotionConsumption>,
    #[cfg(feature = "test-utils")]
    test_wrong_current_operation: bool,
    auth: OwnedCanonicalAuthSnapshot,
    registry: std::sync::Arc<crate::capabilities::CapabilityRegistry>,
    command: crate::swarm_orchestration::model_lane::RoleConsumptionCommand,
}

impl RoleAuthorityConsumptionPermit {
    pub(crate) fn operation_id(&self) -> Uuid { self.capture.command().operation_id() }
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
    ProtectedRoleConsumption(OwnedProtectedRoleConsumption),
}

struct RoutingCaptureRequest {
    execution_id: String,
    stage_id: String,
    response_message_id: String,
}

struct OwnedRoutingCapture {
    operation_id: Uuid,
    auth: OwnedCanonicalAuthSnapshot,
    source_create: OwnedCanonicalAuthSnapshot,
    owner_read: OwnedCanonicalAuthSnapshot,
    sources: std::sync::Arc<RoutingOriginalSources>,
    storage: SurrealStorage,
    registry: std::sync::Arc<crate::capabilities::CapabilityRegistry>,
    request: RoutingCaptureRequest,
    execution_fence: std::sync::OnceLock<crate::swarm_orchestration::routing_execution::ModelLaneRoutingExecutionState>,
    command: std::sync::OnceLock<crate::swarm_orchestration::model_lane::RoleConsumptionCommand>,
}

async fn with_routing_prior_read<T, F>(
    capture: &OwnedRoutingCapture, run_id: &str, stage_id: &str, event_id: &str, operation: F,
) -> Result<T, SurrealStorageError>
where F: std::future::Future<Output = Result<T, SurrealStorageError>> {
    let sources = capture.sources.with_selected_run(run_id.to_owned())
        .map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?;
    capture.owner_read.revalidate_for_publication(&capture.storage, &capture.registry).await
        .map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?;
    let binding = RoutingPriorReadBinding {
        operation_id: capture.operation_id.to_string(), execution_id: capture.request.execution_id.clone(),
        stage_id: stage_id.to_owned(), run_id: run_id.to_owned(), event_id: event_id.to_owned(),
    };
    let result = capture.storage.with_record_user_scope(capture.owner_read.record_user_scope(),
        with_routing_original_sources(sources,
            super::with_model_lane_operation(super::ContextCoordinationPurpose::SourceRead,
                ROUTING_PRIOR_READ_BINDING.scope(binding, operation)))).await?;
    capture.owner_read.revalidate_for_publication(&capture.storage, &capture.registry).await
        .map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?;
    Ok(result)
}

impl OwnedRoleTransaction {
    async fn revalidate_in_transaction(
        &self, transaction: &super::SurrealTransactionContext<'_>, registry: &crate::capabilities::CapabilityRegistry,
    ) -> Result<(), ResourceAuthorityError> {
        self.auth().revalidate_in_transaction(transaction, registry).await?;
        if let Self::Consumer(capture) = self {
            if let Some(read) = &capture.promotion_read {
                read.require_current_registry()?;
                read.owner_read.require_unexpired()?;
                read.child_read.require_unexpired()?;
                if !registry.profile_can(&read.child_read.authority.capability_profile_id, "fs.read").unwrap_or(false) {
                    return Err(ResourceAuthorityError::InvalidInput("protected promotion Child Read profile changed"));
                }
                read.storage.with_record_user_scope(read.owner_read.record_user_scope(),
                    read.owner_read.revalidate_in_transaction(transaction, registry)).await?;
            }
        }
        if let Self::RoutingCapture(capture) = self {
            capture.storage.with_record_user_scope(capture.source_create.record_user_scope(),
                capture.source_create.revalidate_in_transaction(transaction, registry)).await?;
            capture.storage.with_record_user_scope(capture.owner_read.record_user_scope(),
                capture.owner_read.revalidate_in_transaction(transaction, registry)).await?;
        }
        Ok(())
    }
    async fn revalidate_for_publication(
        &self, storage: &SurrealStorage, registry: &crate::capabilities::CapabilityRegistry,
    ) -> Result<(), ResourceAuthorityError> {
        self.auth().revalidate_for_publication(storage, registry).await?;
        if let Self::Consumer(capture) = self {
            if let Some(read) = &capture.promotion_read {
                read.revalidate(storage).await?;
                read.owner_read.revalidate_for_publication(storage, registry).await?;
                read.child_read.revalidate_for_publication(storage, registry).await?;
            }
        }
        if let Self::RoutingCapture(capture) = self {
            capture.source_create.revalidate_for_publication(storage, registry).await?;
            capture.owner_read.revalidate_for_publication(storage, registry).await?;
        }
        Ok(())
    }
    fn promotion_read(&self) -> Option<&std::sync::Arc<InitialValidatorIssuance>> {
        let Self::Consumer(capture) = self else { return None; };
        capture.promotion_read.as_ref()
    }
    fn current_promotion_consumption(&self) -> Option<&CurrentPromotionConsumption> {
        let Self::Consumer(capture) = self else { return None; };
        capture.promotion_consumption.get()
    }
    fn promotion_binding(&self, prepared: Option<String>, operator: Option<String>, validator: Option<String>)
        -> Result<FinalPromotionBinding, SurrealStorageError> {
        let crate::swarm_orchestration::model_lane::RoleConsumptionCommand::ResolvePromotion { selectors, input, .. } =
            self.consumer_command().ok_or(SurrealStorageError::TransactionAuthorityRequired)? else {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        };
        let custody = self.promotion_read().ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
        let published = custody.protected_consumption.get().ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
        let receipt = custody.issued_receipt.get().ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
        Ok(FinalPromotionBinding {
            operation_id: self.operation_id().to_string(), decision_id: input.decision_id.clone(), run_id: input.run_id.clone(),
            input_json: serde_json::to_string(input).map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?,
            selectors_json: serde_json::to_string(&serde_json::json!({
                "operator": { "evidence_ref": selectors.operator.evidence_ref, "expectation": selectors.operator.expectation,
                    "designation_ref": selectors.operator.designation_ref, "producer": selectors.operator.producer },
                "validator": { "evidence_ref": selectors.validator.evidence_ref, "expectation": selectors.validator.expectation,
                    "designation_ref": selectors.validator.designation_ref, "producer": selectors.validator.producer },
            })).map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?,
            protected_binding_event_id: published.event_id.clone(), initial_consumer_operation_id: custody.operation_id.to_string(),
            initial_validator_event_id: receipt.event.event_id.clone(), initial_validator_receipt_json: serde_json::to_string(receipt).map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?,
            creator: self.auth().authority.clone(), owner_read: custody.owner_read.authority.clone(), child_read: custody.child_read.authority.clone(),
            prepared_record_json: prepared, operator_receipt_json: operator, validator_receipt_json: validator,
            current_consumption_event_id: self.current_promotion_consumption().map(|current| current.event_id.clone()),
            current_consumption_body_json: self.current_promotion_consumption().map(|current|
                serde_json::to_string(&current.body).map_err(|_| SurrealStorageError::TransactionAuthorityRequired)).transpose()?,
        })
    }
    fn routing_prior_capture(&self, selector: &crate::swarm_orchestration::model_lane::RoleEvidenceConsumeSelector)
        -> Option<&OwnedRoutingCapture> {
        let Self::RoutingCapture(capture) = self else { return None; };
        let Some(crate::swarm_orchestration::model_lane::RoleConsumptionCommand::AcknowledgeRoutingStage {
            selector: expected, .. }) = capture.command.get() else { return None; };
        if expected.evidence_ref != selector.evidence_ref || expected.expectation != selector.expectation
            || selector.expectation.receipt_scope != capture.owner_read.exact { return None; }
        Some(capture)
    }

    fn protected_owner_read_authority(&self) -> Option<RegistryPublicationAuthority> {
        match self {
            Self::ProtectedRoleConsumption(capture) => Some(capture.issuance.owner_read.authority.clone()),
            _ => match self.consumer_command() {
                Some(crate::swarm_orchestration::model_lane::RoleConsumptionCommand::IssueInitialValidatorVerdict { issuance, .. }) =>
                    Some(issuance.owner_read.authority.clone()),
                _ => None,
            },
        }
    }
    fn protected_child_read_authority(&self) -> Option<RegistryPublicationAuthority> {
        match self {
            Self::ProtectedRoleConsumption(capture) => Some(capture.issuance.child_read.authority.clone()),
            _ => match self.consumer_command() {
                Some(crate::swarm_orchestration::model_lane::RoleConsumptionCommand::IssueInitialValidatorVerdict { issuance, .. }) =>
                    Some(issuance.child_read.authority.clone()),
                _ => None,
            },
        }
    }
    fn operation_id(&self) -> Uuid {
        match self {
            Self::Owner(permit) => permit.command.operation_id(),
            Self::Consumer(permit) => permit.command.operation_id(),
            Self::RoutingCapture(capture) => capture.operation_id,
            Self::ProtectedRoleConsumption(capture) => capture.operation_id,
        }
    }

    fn auth(&self) -> &OwnedCanonicalAuthSnapshot {
        match self {
            Self::Owner(permit) => &permit.auth,
            Self::Consumer(permit) => &permit.auth,
            Self::RoutingCapture(capture) => &capture.auth,
            Self::ProtectedRoleConsumption(capture) => &capture.issuance.owner_read,
        }
    }

    fn consumer_command(&self) -> Option<&crate::swarm_orchestration::model_lane::RoleConsumptionCommand> {
        match self {
            Self::Consumer(capture) => Some(&capture.command),
            Self::RoutingCapture(capture) => capture.command.get(),
            Self::Owner(_) | Self::ProtectedRoleConsumption(_) => None,
        }
    }

    fn consumer_registry(&self) -> Option<&crate::capabilities::CapabilityRegistry> {
        match self {
            Self::Consumer(capture) => Some(capture.registry.as_ref()),
            Self::RoutingCapture(capture) => Some(capture.registry.as_ref()),
            Self::ProtectedRoleConsumption(capture) => Some(&capture.registry),
            Self::Owner(_) => None,
        }
    }
}

struct PublishedProtectedRoleConsumption {
    body: ProtectedRoleEvidenceConsumptionBody,
    event_id: String,
}

struct OwnedProtectedRoleConsumption {
    operation_id: Uuid,
    issuance: std::sync::Arc<InitialValidatorIssuance>,
    registry: crate::capabilities::CapabilityRegistry,
    body: std::sync::OnceLock<ProtectedRoleEvidenceConsumptionBody>,
}

pub(crate) struct ProtectedRoleEvidenceTargetReadSeal<'tx> {
    capture: &'tx OwnedProtectedRoleConsumption,
    transaction: &'tx super::SurrealTransactionContext<'tx>,
}
impl ProtectedRoleEvidenceTargetReadSeal<'_> {
    pub(crate) async fn with_protected_role_evidence_capture<T, F>(
        &self, record_kind: &str, event_id: &str, future: F,
    ) -> Result<T, SurrealStorageError>
    where F: std::future::Future<Output = Result<T, SurrealStorageError>> {
        let issuance = &self.capture.issuance;
        let owner = &issuance.owner_read;
        let selected_event = match record_kind {
            "role_evidence" => issuance.operator.evidence_ref.as_str().strip_prefix("eventledger://role-authority/"),
            "validator_designation" => issuance.verdict.designation_ref.as_str().strip_prefix("eventledger://validator-designation/"),
            _ => None,
        };
        let current = super::current_record_user_scope().ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
        let expected = owner.record_user_scope();
        if selected_event != Some(event_id) || event_id.is_empty()
            || !owner.requires_workspace_owner || owner.action != ResourceAction::Read || owner.capability_id != "fs.read"
            || owner.exact != issuance.operator.expectation.receipt_scope
            || current.session_token != expected.session_token || current.channel_binding_hash != expected.channel_binding_hash
            || current.session_id != expected.session_id || current.resource_id != expected.resource_id
            || current.grant_id != expected.grant_id || current.workspace_id != expected.workspace_id
            || current.action != expected.action || current.capability_id != expected.capability_id {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
        owner.require_unexpired().map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?;
        issuance.require_current_registry().map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?;
        let selection_json = serde_json::to_string(&serde_json::json!({
            "run": issuance.verdict.run,
            "subject": issuance.verdict.subject,
            "expected": issuance.verdict.expected,
            "purpose": issuance.verdict.purpose,
            "producer": issuance.producer_binding(),
        })).map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?;
        PROTECTED_ROLE_CAPTURE_BINDING.scope(ProtectedRoleCaptureBinding {
            record_kind: record_kind.to_owned(), event_id: event_id.to_owned(),
            operation_id: self.capture.operation_id.to_string(), consumer_operation_id: issuance.operation_id.to_string(),
            issuer_scope: (&owner.exact).into(), exact_consumer_scope: (&issuance.child_read.exact).into(),
            expected_owner_read: owner.authority.clone(), expected_child_read: issuance.child_read.authority.clone(),
            operator_ref: issuance.operator.evidence_ref.to_string(), designation_ref: issuance.verdict.designation_ref.to_string(),
            selection_json,
        }, future).await
    }
    pub(crate) fn current_scope(&self) -> &crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution { &self.capture.issuance.owner_read.exact }
    pub(crate) fn operation_id(&self) -> Uuid { self.capture.operation_id }
    pub(crate) fn issuance(&self) -> &InitialValidatorIssuance { &self.capture.issuance }
    pub(crate) fn matches_transaction(&self, transaction: &super::SurrealTransactionContext<'_>) -> bool { std::ptr::eq(self.transaction, transaction) }
}

pub(crate) struct ProtectedRoleEvidenceCreationSeal<'tx> {
    capture: &'tx OwnedProtectedRoleConsumption,
    transaction: &'tx super::SurrealTransactionContext<'tx>,
}
impl ProtectedRoleEvidenceCreationSeal<'_> {
    pub(crate) async fn with_protected_role_evidence_creation<T, F>(
        &self, record_id: &str, event_id: &str, record_sha256: &str, future: F,
    ) -> Result<T, SurrealStorageError>
    where F: std::future::Future<Output = Result<T, SurrealStorageError>> {
        if record_id.is_empty() || event_id != format!("evt-{record_id}") || record_sha256.len() != 64 {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
        with_sealed_role_creation("role_evidence_consumption", record_id, event_id, record_sha256,
            self.operation_id(), self.current_scope(), Some(self.capture.issuance.child_read.authority.clone()), Some(self.capture.issuance.owner_read.authority.clone()), &self.capture.issuance.owner_read.created_role_rows, future).await
    }
    pub(crate) fn body(&self) -> &ProtectedRoleEvidenceConsumptionBody { self.capture.body.get().expect("creation seal follows canonical target capture") }
    pub(crate) fn current_scope(&self) -> &crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution { &self.capture.issuance.owner_read.exact }
    pub(crate) fn operation_id(&self) -> Uuid { self.capture.operation_id }
    pub(crate) fn matches_transaction(&self, transaction: &super::SurrealTransactionContext<'_>) -> bool { std::ptr::eq(self.transaction, transaction) }
}

enum OperatorConsumptionReadback {
    Review(super::model_lane::RoleEvidenceReadback),
    Completion(super::model_lane::RoleEvidenceReadback),
    Promotion {
        operator: super::model_lane::RoleEvidenceReadback,
        validator: super::model_lane::RoleEvidenceReadback,
        decision: crate::swarm_orchestration::model_lane::ModelLanePromotionDecisionRecord,
    },
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
    launch_plan: Vec<crate::swarm_orchestration::routing::ModelLaneRoutingStageLaunchPlan>,
}

impl PreparedRoutingAuthorityConsumption {
    pub(crate) fn launch_plan(&self) -> &[crate::swarm_orchestration::routing::ModelLaneRoutingStageLaunchPlan] {
        &self.launch_plan
    }

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
    pub(crate) fn continuation_sources(&self) -> std::sync::Arc<RoutingOriginalSources> {
        let OwnedRoleTransaction::RoutingCapture(capture) = self.retention.owner.as_ref() else {
            unreachable!("routing capture retains its original owner");
        };
        let mut sources = capture.sources.for_owned_operation().as_ref().clone();
        let retained = sources.capture.as_ref();
        let facts = retained.sources.iter().filter(|source| source.request.action != ResourceAction::Read)
            .map(|source| ProducerSourceFacts { request: source.request.clone(), decision: source.decision.clone(), authority: source.authority.clone() }).collect::<Vec<_>>();
        sources.capture = std::sync::Arc::new(ProducerSourceFactsCapture {
            operation_id: retained.operation_id, credentials: retained.credentials.clone(), session: retained.session.clone(),
            registry: retained.registry.clone(), lifecycle: retained.lifecycle.clone(), fence: retained.fence.clone(), sources: facts,
        });
        sources.create_index = sources.capture.sources.iter().position(|source| source.request.action == ResourceAction::Create);
        std::sync::Arc::new(sources)
    }

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
        self.retention.owner.revalidate_in_transaction(transaction, &capture.registry).await?;
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
            stage_id: &capture.request.stage_id,
            auth: &capture.auth, transaction, _fence: &self.retention.fence,
        };
        let selected = capture.sources.with_execution_origin(inputs.execution().run_id.clone(),
            inputs.execution().selecting_decision_id.clone(), inputs.execution().initial_input_ref.clone())?;
        let prior_event_id = stage.prior_receipt_ref.as_str().strip_prefix("eventledger://role-authority/")
            .ok_or_else(|| ModelLaneError::AuthorityDenied("canonical prior reference malformed".into()))?;
        let (receipt, issuance_operation_id) = with_routing_original_sources(selected,
            with_routing_prior_read(capture, &inputs.execution().run_id, &capture.request.stage_id, prior_event_id,
                self.provider.read_routing_prior_in_transaction(transaction, &seal, &stage.prior_receipt_ref))).await?;
        if receipt.body.scope != capture.owner_read.exact {
            return Err(ModelLaneError::AuthorityDenied(
                "routing prior issuer requires separate protected Read/delegation authority".into()));
        }
        if receipt.body.role != expected_role || receipt.body.subject != inputs.subject()? {
            return Err(ModelLaneError::IntegrityViolation("canonical routing prior role or subject differs from stage".into()));
        }
        let prior = RoutingPriorEvidenceReadback { receipt, issuance_operation_id };
        let run = inputs.run().clone();
        validate_routing_capture_basis(&self.retention, &inputs, &run, &prior)?;
        self.retention.owner.revalidate_in_transaction(transaction, &capture.registry).await?;
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
        super::with_configured_capability_registry(capture.registry.clone(),
            with_routing_original_sources(capture.sources.clone(),
                super::with_model_lane_operation(super::ContextCoordinationPurpose::RoutingTransition,
                    self.storage.read_routing_capture_inputs_in_transaction(
                        transaction, &capture.auth, &capture.registry, &self.retention.fence,
                        &self.store, &self.provider, &capture.request.execution_id, &capture.request.stage_id,
                        &capture.request.response_message_id,
                    )))).await
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
            || prepared.expected() != &prior.expected || prepared.retention() != &prior.retention
        {
            return Err(ModelLaneError::IntegrityViolation("routing acknowledgement canonical basis changed".into()));
        }
        capture.execution_fence.set(self.inputs.execution().clone())
            .map_err(|_| ModelLaneError::IntegrityViolation("routing execution fence was already captured".into()))?;
        capture.command.set(self.command)
            .map_err(|_| ModelLaneError::IntegrityViolation("routing capture command was already initialized".into()))?;
        Ok(PreparedRoutingAuthorityConsumption { storage: self.storage, retention: self.retention, run: self.run,
            launch_plan: self.inputs.execution().canonical_launch_plan.clone() })
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
    pub(crate) fn current_promotion_consumption(&self) -> Result<(&CurrentPromotionConsumptionBody, &str), SurrealStorageError> {
        let current = self.owner.current_promotion_consumption().ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
        Ok((&current.body, current.event_id.as_str()))
    }
    pub(crate) fn is_final_promotion(&self) -> bool { self.owner.promotion_read().is_some() }
    pub(crate) async fn with_promotion_read<T, F>(
        &self, expected: &crate::swarm_orchestration::model_lane::ModelLanePromotionDecisionRecord, operation: F,
    ) -> crate::swarm_orchestration::model_lane::ModelLaneResult<T>
    where F: std::future::Future<Output = crate::swarm_orchestration::model_lane::ModelLaneResult<T>> {
        let custody = self.owner.promotion_read().ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
        custody.revalidate(&custody.storage).await?;
        let binding = self.owner.promotion_binding(Some(serde_json::to_string(expected)
            .map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?), None, None)?;
        let result = custody.storage.with_record_user_scope(custody.owner_read.record_user_scope(),
            super::with_model_lane_operation(super::ContextCoordinationPurpose::FinalPromotionRead,
                FINAL_PROMOTION_BINDING.scope(binding, operation))).await?;
        custody.revalidate(&custody.storage).await?;
        Ok(result)
    }
    pub(crate) fn is_routing_prior(&self, selector: &crate::swarm_orchestration::model_lane::RoleEvidenceConsumeSelector) -> bool {
        self.owner.routing_prior_capture(selector).is_some()
    }
    pub(crate) async fn with_routing_prior<T, F>(
        &self, selector: &crate::swarm_orchestration::model_lane::RoleEvidenceConsumeSelector, operation: F,
    ) -> Result<T, SurrealStorageError>
    where F: std::future::Future<Output = Result<T, SurrealStorageError>> {
        let capture = self.owner.routing_prior_capture(selector).ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
        let event_id = selector.evidence_ref.as_str().strip_prefix("eventledger://role-authority/")
            .ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
        with_routing_prior_read(capture, &selector.expectation.run.run_id, &capture.request.stage_id, event_id, operation).await
    }

    pub(crate) fn has_protected_role_evidence_target(&self, event_id: &str) -> bool {
        let Some(crate::swarm_orchestration::model_lane::RoleConsumptionCommand::IssueInitialValidatorVerdict { issuance, .. }) = self.owner.consumer_command() else { return false; };
        issuance.protected_consumption.get().is_some_and(|published|
            self.require_unexpired().is_ok()
                && published.body.consumer_operation_id == self.operation_id()
                && &published.body.exact_consumer_scope == self.current_scope()
                && published.body.targets.iter().any(|target| target.event_id == event_id))
    }
    pub(crate) async fn with_protected_role_authority_read<T, F>(
        &self, record_kind: &str, record_id: &str, event_id: &str, record_sha256: &str, future: F,
    ) -> Result<T, SurrealStorageError>
    where F: std::future::Future<Output = Result<T, SurrealStorageError>> {
        self.require_unexpired().map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?;
        let binding = ProtectedRoleCreationBinding {
            record_kind: record_kind.to_owned(), record_id: record_id.to_owned(), event_id: event_id.to_owned(),
            record_sha256: record_sha256.to_owned(), operation_id: self.operation_id().to_string(), issuer_scope: self.current_scope().into(),
            expected_child_read: self.owner.protected_child_read_authority(), expected_owner_read: self.owner.protected_owner_read_authority(),
        };
        if !self.owner.auth().created_role_rows.lock().map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?.contains(&binding) {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
        let work = PROTECTED_ROLE_CREATION_BINDING.scope(binding, future);
        if let OwnedRoleTransaction::RoutingCapture(capture) = self.owner {
            capture.source_create.revalidate_for_publication(&capture.storage, &capture.registry).await
                .map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?;
            let result = capture.storage.with_record_user_scope(capture.source_create.record_user_scope(), work).await?;
            capture.source_create.revalidate_for_publication(&capture.storage, &capture.registry).await
                .map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?;
            Ok(result)
        } else { work.await }
    }
    pub(crate) async fn with_protected_role_evidence_consumption<T, F>(
        &self, evidence_event_id: &str, future: F,
    ) -> Result<T, SurrealStorageError>
    where F: std::future::Future<Output = Result<T, SurrealStorageError>> {
        self.require_unexpired().map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?;
        let Some(crate::swarm_orchestration::model_lane::RoleConsumptionCommand::IssueInitialValidatorVerdict { issuance, .. }) = self.owner.consumer_command() else {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        };
        let published = issuance.protected_consumption.get().ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
        if published.body.consumer_operation_id != self.operation_id() || &published.body.exact_consumer_scope != self.current_scope()
            || !published.body.targets.iter().any(|target| target.event_id == evidence_event_id) {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
        PROTECTED_ROLE_CONSUMPTION_BINDING.scope(ProtectedRoleConsumptionBinding {
            binding_event_id: published.event_id.clone(), evidence_event_id: evidence_event_id.to_owned(), consumer_operation_id: self.operation_id().to_string(), expected_child_read: self.owner.protected_child_read_authority(), expected_owner_read: self.owner.protected_owner_read_authority(),
        }, future).await
    }
    pub(crate) fn operation_id(&self) -> Uuid { self.owner.operation_id() }

    pub(crate) fn current_scope(&self) -> &crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution {
        &self.owner.auth().exact
    }

    pub(crate) fn require_unexpired(&self) -> Result<(), ResourceAuthorityError> {
        self.owner.auth().require_unexpired()?;
        if let OwnedRoleTransaction::RoutingCapture(capture) = self.owner {
            capture.source_create.require_unexpired()?;
            capture.owner_read.require_unexpired()?;
        }
        Ok(())
    }
}

pub(crate) struct OwnerIssuanceSeal<'tx> {
    permit: &'tx WorkspaceOwnerActionPermit,
    transaction: &'tx super::SurrealTransactionContext<'tx>,
}

impl OwnerIssuanceSeal<'_> {
    pub(crate) async fn with_protected_role_authority_creation<T, F>(
        &self, record_kind: &str, record_id: &str, event_id: &str, record_sha256: &str, future: F,
    ) -> Result<T, SurrealStorageError>
    where F: std::future::Future<Output = Result<T, SurrealStorageError>> {
        let permitted = match self.command() {
            crate::swarm_orchestration::model_lane::WorkspaceOwnerActionCommand::IssueArtifactApproval(_) => record_kind == "role_evidence",
            crate::swarm_orchestration::model_lane::WorkspaceOwnerActionCommand::IssueRunnerDesignation(_) => record_kind == "validator_designation",
        };
        if !permitted { return Err(SurrealStorageError::TransactionAuthorityRequired); }
        with_sealed_role_creation(record_kind, record_id, event_id, record_sha256, self.operation_id(), self.current_scope(), None, None, &self.permit.auth.created_role_rows, future).await
    }
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
    stage_id: &'tx str,
    auth: &'tx OwnedCanonicalAuthSnapshot,
    transaction: &'tx super::SurrealTransactionContext<'tx>,
    _fence: &'tx tokio::sync::OwnedRwLockReadGuard<()>,
}

impl RoutingAuthorityCaptureReadSeal<'_> {
    pub(crate) fn stage_id(&self) -> &str { self.stage_id }
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
    pub(crate) fn current_promotion_consumption_body(
        &self, operator: &super::model_lane::TxCheckedRoleEvidence<'_>, validator: &super::model_lane::TxCheckedRoleEvidence<'_>,
    ) -> Result<CurrentPromotionConsumptionBody, SurrealStorageError> {
        let crate::swarm_orchestration::model_lane::RoleConsumptionCommand::ResolvePromotion { selectors, input, .. } = self.command() else {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        };
        operator.require_matches(self.transaction, self.operation_id(), &selectors.operator)
            .map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?;
        validator.require_matches(self.transaction, self.operation_id(), &selectors.validator)
            .map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?;
        operator.require_promotion_input(input).map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?;
        validator.require_promotion_input(input).map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?;
        let custody = self.owner.promotion_read().ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
        let lineage = custody.protected_consumption.get().ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
        let binding = self.owner.promotion_binding(None, None, None)?;
        let authority = |auth: &OwnedCanonicalAuthSnapshot| ProtectedRoleReadAuthority {
            grant_id: auth.decision.grant_id.clone(), grant_version: auth.authority.grant_version,
            resource_id: auth.decision.resource_id.clone(), session_id: auth.decision.session_id.clone(),
            capability_id: auth.capability_id.clone(), action: auth.action.as_str().to_owned(),
        };
        Ok(CurrentPromotionConsumptionBody {
            schema_id: "handshake.role-promotion-consumption@1".into(), operation_id: self.operation_id(),
            exact_consumer_scope: self.current_scope().clone(), decision_id: input.decision_id.clone(),
            input_json: binding.input_json, selectors_json: binding.selectors_json,
            creator: authority(self.owner.auth()), owner_read: authority(&custody.owner_read), child_read: authority(&custody.child_read),
            owner_read_scope: custody.owner_read.exact.clone(), child_read_scope: custody.child_read.exact.clone(),
            lineage_binding_event_id: lineage.event_id.clone(), initial_consumer_operation_id: custody.operation_id,
            producer: custody.producer_binding(), designation_ref: custody.verdict.designation_ref.clone(),
            run: operator.receipt().body.run.clone(), subject: operator.receipt().body.subject.clone(),
            targets: vec![operator.protected_consumption_target()?, validator.protected_consumption_target()?],
        })
    }
    pub(crate) async fn with_current_promotion_consumption_creation<T, F>(
        &self, body: &CurrentPromotionConsumptionBody, event_id: &str,
        operator: &super::model_lane::TxCheckedRoleEvidence<'_>, validator: &super::model_lane::TxCheckedRoleEvidence<'_>, operation: F,
    ) -> Result<T, SurrealStorageError>
    where F: std::future::Future<Output = Result<T, SurrealStorageError>> {
        let actual = self.current_promotion_consumption_body(operator, validator)?;
        let actual_json = serde_json::to_string(&actual).map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?;
        if serde_json::to_string(body).map_err(|_| SurrealStorageError::TransactionAuthorityRequired)? != actual_json {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
        let mut binding = self.owner.promotion_binding(None,
            Some(serde_json::to_string(operator.receipt()).map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?),
            Some(serde_json::to_string(validator.receipt()).map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?))?;
        binding.current_consumption_event_id = Some(event_id.to_owned());
        binding.current_consumption_body_json = Some(actual_json);
        let custody = self.owner.promotion_read().ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
        let registry = self.owner.consumer_registry().ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
        custody.storage.with_record_user_scope(self.owner.auth().record_user_scope(),
            super::with_model_lane_operation(super::ContextCoordinationPurpose::FinalPromotionRecord,
                FINAL_PROMOTION_BINDING.scope(binding, async {
                    self.owner.auth().revalidate_in_transaction(self.transaction, registry).await
                        .map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?;
                    let result = operation.await?;
                    self.owner.auth().revalidate_in_transaction(self.transaction, registry).await
                        .map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?;
                    Ok(result)
                }))).await
    }
    pub(crate) fn observe_current_promotion_consumption(&self, body: CurrentPromotionConsumptionBody, event_id: String)
        -> Result<(), SurrealStorageError> {
        let OwnedRoleTransaction::Consumer(capture) = self.owner else { return Err(SurrealStorageError::TransactionAuthorityRequired); };
        if let Some(current) = capture.promotion_consumption.get() {
            if current.event_id != event_id || serde_json::to_string(&current.body).ok() != serde_json::to_string(&body).ok() {
                return Err(SurrealStorageError::TransactionAuthorityRequired);
            }
            return Ok(());
        }
        capture.promotion_consumption.set(CurrentPromotionConsumption { body, event_id })
            .map_err(|_| SurrealStorageError::TransactionAuthorityRequired)
    }
    pub(crate) async fn with_promotion_read<T, F>(&self, operation: F)
        -> crate::swarm_orchestration::model_lane::ModelLaneResult<T>
    where F: std::future::Future<Output = crate::swarm_orchestration::model_lane::ModelLaneResult<T>> {
        let custody = self.owner.promotion_read().ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
        custody.require_current_registry()?;
        custody.owner_read.require_unexpired()?;
        custody.child_read.require_unexpired()?;
        let binding = self.owner.promotion_binding(None, None, None)?;
        custody.storage.with_record_user_scope(custody.owner_read.record_user_scope(),
            super::with_model_lane_operation(super::ContextCoordinationPurpose::FinalPromotionRead,
                FINAL_PROMOTION_BINDING.scope(binding, async {
                    custody.owner_read.revalidate_in_transaction(self.transaction,
                        self.owner.consumer_registry().ok_or(SurrealStorageError::TransactionAuthorityRequired)?).await?;
                    let value = operation.await?;
                    custody.owner_read.revalidate_in_transaction(self.transaction,
                        self.owner.consumer_registry().ok_or(SurrealStorageError::TransactionAuthorityRequired)?).await?;
                    custody.require_current_registry()?;
                    custody.owner_read.require_unexpired()?;
                    custody.child_read.require_unexpired()?;
                    Ok(value)
                }))).await
    }
    pub(crate) fn current_promotion_consumption_reference(&self) -> Result<(&str, Uuid), SurrealStorageError> {
        let current = self.owner.current_promotion_consumption().ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
        if current.body.operation_id != self.operation_id() { return Err(SurrealStorageError::TransactionAuthorityRequired); }
        Ok((current.event_id.as_str(), current.body.operation_id))
    }
    pub(crate) async fn with_promotion_creation<T, F>(
        &self, prepared: &crate::swarm_orchestration::model_lane::ModelLanePromotionDecisionRecord,
        operator: &super::model_lane::TxCheckedRoleEvidence<'_>, validator: &super::model_lane::TxCheckedRoleEvidence<'_>, operation: F,
    ) -> crate::swarm_orchestration::model_lane::ModelLaneResult<T>
    where F: std::future::Future<Output = crate::swarm_orchestration::model_lane::ModelLaneResult<T>> {
        let crate::swarm_orchestration::model_lane::RoleConsumptionCommand::ResolvePromotion { selectors, .. } = self.command() else {
            return Err(SurrealStorageError::TransactionAuthorityRequired.into());
        };
        operator.require_matches(self.transaction, self.operation_id(), &selectors.operator)?;
        validator.require_matches(self.transaction, self.operation_id(), &selectors.validator)?;
        self.owner.current_promotion_consumption().ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
        #[allow(unused_mut)]
        let mut binding = self.owner.promotion_binding(
            Some(serde_json::to_string(prepared).map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?),
            Some(serde_json::to_string(operator.receipt()).map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?),
            Some(serde_json::to_string(validator.receipt()).map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?))?;
        #[cfg(feature = "test-utils")]
        let _test_authority = if matches!(self.owner, OwnedRoleTransaction::Consumer(capture) if capture.test_wrong_current_operation) {
            let custody = self.owner.promotion_read().ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
            let original = self.owner.auth();
            let registry = self.owner.consumer_registry().ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
            let fresh = custody.storage.with_record_user_scope(original.record_user_scope(), capture_owned_canonical_role_auth(
                &custody.storage, registry, original.credentials.clone(),
                original.credentials.workspace_creation_request(&original.workspace_id))).await?;
            if fresh.exact != original.exact || fresh.authority != original.authority {
                return Err(SurrealStorageError::TransactionAuthorityRequired.into());
            }
            let other = Uuid::now_v7();
            if other == self.operation_id() { return Err(SurrealStorageError::TransactionAuthorityRequired.into()); }
            binding.operation_id = other.to_string();
            Some(fresh)
        } else { None };
        self.owner.promotion_read().ok_or(SurrealStorageError::TransactionAuthorityRequired)?.storage.with_record_user_scope(self.owner.auth().record_user_scope(),
            super::with_model_lane_operation(super::ContextCoordinationPurpose::FinalPromotionRecord,
                FINAL_PROMOTION_BINDING.scope(binding, async {
                    let registry = self.owner.consumer_registry().ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
                    self.owner.auth().revalidate_in_transaction(self.transaction, registry).await?;
                    let value = operation.await?;
                    self.owner.auth().revalidate_in_transaction(self.transaction, registry).await?;
                    Ok(value)
                }))).await
    }
    pub(crate) fn is_routing_prior(&self, selector: &crate::swarm_orchestration::model_lane::RoleEvidenceConsumeSelector) -> bool {
        self.owner.routing_prior_capture(selector).is_some()
    }
    pub(crate) async fn with_routing_prior<T, F>(
        &self, selector: &crate::swarm_orchestration::model_lane::RoleEvidenceConsumeSelector, operation: F,
    ) -> Result<T, SurrealStorageError>
    where F: std::future::Future<Output = Result<T, SurrealStorageError>> {
        let capture = self.owner.routing_prior_capture(selector).ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
        let event_id = selector.evidence_ref.as_str().strip_prefix("eventledger://role-authority/")
            .ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
        with_routing_prior_read(capture, &selector.expectation.run.run_id, &capture.request.stage_id, event_id, operation).await
    }

    pub(crate) fn require_routing_execution_fence(
        &self, execution: &crate::swarm_orchestration::routing_execution::ModelLaneRoutingExecutionState,
        stage_id: &str, message_id: &str,
    ) -> crate::swarm_orchestration::model_lane::ModelLaneResult<()> {
        let OwnedRoleTransaction::RoutingCapture(capture) = self.owner else {
            return Err(SurrealStorageError::TransactionAuthorityRequired.into());
        };
        let original = capture.execution_fence.get().ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
        if original != execution || execution.execution_id != capture.request.execution_id
            || stage_id != capture.request.stage_id || message_id != capture.request.response_message_id {
            return Err(crate::swarm_orchestration::model_lane::ModelLaneError::AuthorityDenied(
                "routing acknowledgement captured execution fence is stale or differs".into()));
        }
        Ok(())
    }

    pub(crate) async fn with_routing_ack<T, F>(
        &self, prior: &super::model_lane::TxCheckedRoleEvidence<'_>,
        acknowledgement: &super::model_lane::TxCheckedRoleEvidence<'_>,
        claim: &crate::swarm_orchestration::routing_execution::ModelLaneRoutingStageClaim,
        revision: u64, message_id: &str, observed_execution_event_id: &str,
        observed_attempt_event_id: &str, operation: F,
    ) -> crate::swarm_orchestration::model_lane::ModelLaneResult<T>
    where F: std::future::Future<Output = crate::swarm_orchestration::model_lane::ModelLaneResult<T>> {
        use crate::swarm_orchestration::model_lane::{ModelLaneError, RoleConsumptionCommand};
        let RoleConsumptionCommand::AcknowledgeRoutingStage { selector, execution_id, stage_id,
            message_id: selected_message, acknowledgement: intent, .. } = self.command() else {
            return Err(ModelLaneError::AuthorityDenied("routing ACK command absent".into()));
        };
        if execution_id != &claim.execution_id || stage_id != &claim.stage_id || selected_message != message_id {
            return Err(ModelLaneError::AuthorityDenied("routing ACK selected claim differs".into()));
        }
        let OwnedRoleTransaction::RoutingCapture(capture) = self.owner else {
            return Err(SurrealStorageError::TransactionAuthorityRequired.into());
        };
        let original = capture.execution_fence.get().ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
        let attempt = original.stages.get(stage_id).ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
        if original.revision != revision || original.event_ledger_event_id != observed_execution_event_id
            || attempt.event_ledger_event_id != observed_attempt_event_id || attempt.attempt != claim.attempt
            || attempt.lease_owner.as_deref() != Some(claim.lease_owner.as_str())
            || attempt.fencing_token.as_deref() != Some(claim.fencing_token.as_str())
            || attempt.lease_expires_at_unix_ms != Some(claim.lease_expires_at_unix_ms) {
            return Err(ModelLaneError::AuthorityDenied("routing acknowledgement original claim or receipt differs".into()));
        }
        prior.require_matches(self.transaction, self.operation_id(), selector)?;
        acknowledgement.require_stage_ack(self.transaction, self.operation_id(), selector, intent)?;
        let integer = |value| i64::try_from(value).map_err(|_| ModelLaneError::InvalidInput("routing ACK durable integer overflow".into()));
        ROUTING_ACK_BINDING.scope(RoutingAckBinding {
            operation_id: self.operation_id().to_string(), acknowledgement_event_id: acknowledgement.receipt().event.event_id.clone(),
            prior_event_id: prior.receipt().event.event_id.clone(), execution_id: execution_id.clone(), stage_id: stage_id.clone(),
            message_id: message_id.to_owned(), attempt: i64::from(claim.attempt), lease_owner: claim.lease_owner.clone(),
            fencing_token: claim.fencing_token.clone(), lease_expires_at_unix_ms: integer(claim.lease_expires_at_unix_ms)?,
            observed_revision: integer(revision)?, observed_execution_event_id: observed_execution_event_id.to_owned(),
            observed_attempt_event_id: observed_attempt_event_id.to_owned(),
        }, operation).await
    }

    pub(crate) async fn with_protected_role_authority_creation<T, F>(
        &self, record_kind: &str, record_id: &str, event_id: &str, record_sha256: &str, future: F,
    ) -> Result<T, SurrealStorageError>
    where F: std::future::Future<Output = Result<T, SurrealStorageError>> {
        use crate::swarm_orchestration::model_lane::RoleConsumptionCommand;
        if record_kind != "role_evidence" || !matches!(self.command(), RoleConsumptionCommand::IssueInitialValidatorVerdict { .. }
            | RoleConsumptionCommand::RecordReviewCompletion { .. } | RoleConsumptionCommand::AcknowledgeRoutingStage { .. }) {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
        let work = with_sealed_role_creation(record_kind, record_id, event_id, record_sha256,
            self.operation_id(), self.current_scope(), self.owner.protected_child_read_authority(),
            self.owner.protected_owner_read_authority(), &self.owner.auth().created_role_rows, future);
        if let OwnedRoleTransaction::RoutingCapture(capture) = self.owner {
            capture.storage.with_record_user_scope(capture.source_create.record_user_scope(), async {
                capture.source_create.revalidate_in_transaction(self.transaction, &capture.registry).await
                    .map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?;
                let result = work.await?;
                capture.source_create.revalidate_in_transaction(self.transaction, &capture.registry).await
                    .map_err(|_| SurrealStorageError::TransactionAuthorityRequired)?;
                Ok(result)
            }).await
        } else { work.await }
    }
    pub(crate) fn protected_consumption_body(&self) -> Option<&ProtectedRoleEvidenceConsumptionBody> {
        match self.command() {
            crate::swarm_orchestration::model_lane::RoleConsumptionCommand::IssueInitialValidatorVerdict { issuance, .. } =>
                issuance.protected_consumption.get().map(|published| &published.body),
            _ => None,
        }
    }
    pub(crate) fn protected_consumption_event_id(&self) -> Option<&str> {
        match self.command() {
            crate::swarm_orchestration::model_lane::RoleConsumptionCommand::IssueInitialValidatorVerdict { issuance, .. } =>
                issuance.protected_consumption.get().map(|published| published.event_id.as_str()),
            _ => None,
        }
    }
    pub(crate) async fn with_protected_role_evidence_consumption<T, F>(
        &self, evidence_event_id: &str, future: F,
    ) -> Result<T, SurrealStorageError>
    where F: std::future::Future<Output = Result<T, SurrealStorageError>> {
        let body = self.protected_consumption_body().ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
        if body.consumer_operation_id != self.operation_id() || &body.exact_consumer_scope != self.current_scope()
            || !body.targets.iter().any(|target| target.event_id == evidence_event_id) {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
        let binding_event_id = self.protected_consumption_event_id().ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
        PROTECTED_ROLE_CONSUMPTION_BINDING.scope(ProtectedRoleConsumptionBinding {
            binding_event_id: binding_event_id.to_owned(), evidence_event_id: evidence_event_id.to_owned(), consumer_operation_id: self.operation_id().to_string(), expected_child_read: self.owner.protected_child_read_authority(), expected_owner_read: self.owner.protected_owner_read_authority(),
        }, future).await
    }
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
    pub(crate) async fn execute_role_consumption(
        &self,
        permit: RoleAuthorityConsumptionPermit,
        store: crate::swarm_orchestration::model_lane::ModelLaneStore,
    ) -> crate::swarm_orchestration::model_lane::ModelLaneResult<crate::swarm_orchestration::model_lane::RoleConsumptionResult> {
        use crate::swarm_orchestration::model_lane::{ModelLaneError, RoleConsumptionCommand,
            RoleConsumptionResult, RoutingStageAcknowledgementIntent};
        let store_scope = match permit.capture.command() {
            RoleConsumptionCommand::IssueInitialValidatorVerdict { issuance, .. } => issuance.delegating_scope(),
            _ => permit.current_scope(),
        };
        if store.access().exact_read_scope() != Some(store_scope)
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
        let routing_sources = match retention.owner.as_ref() {
            OwnedRoleTransaction::RoutingCapture(capture) => Some(capture.sources.clone()), _ => None,
        };
        let committed = with_role_routing_context(routing_sources.clone(),
                self.with_record_user_scope(scope,
            self.with_owned_role_authenticated_transaction(
                operation_id, super::ContextOperationCancellation::new(), retention.clone(),
                move |transaction| Box::pin(async move {
                    let owner = worker_retention.owner.as_ref();
                    let registry = owner.consumer_registry().ok_or_else(||
                        ModelLaneError::IntegrityViolation("Operator consumer registry is unavailable".into()))?;
                    owner.revalidate_in_transaction(&transaction, registry).await?;
                    let seal = CurrentConsumerSeal { owner, transaction: &transaction };
                    let command = owner.consumer_command().ok_or_else(||
                        ModelLaneError::IntegrityViolation("Operator consumer command is unavailable".into()))?;
                    let readback = match command {
                        RoleConsumptionCommand::IssueInitialValidatorVerdict { operator, issuance, .. } => {
                            issuance.require_current_registry()?;
                            issuance.child_read.revalidate_read_in_child_transaction(&transaction, registry).await?;
                            let prior = worker_provider.resolve_operator_role_evidence(&transaction, &seal, operator).await?;
                            let completed = worker_provider.put_initial_validator_verdict(&transaction, &seal, &prior, issuance).await?;
                            issuance.child_read.revalidate_read_in_child_transaction(&transaction, registry).await?;
                            issuance.require_current_registry()?;
                            OperatorConsumptionReadback::Completion(completed.into_readback())
                        },
                        RoleConsumptionCommand::ReviewInvocation { selector, .. } => {
                            let checked = worker_provider.resolve_role_evidence(
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
                        RoleConsumptionCommand::AcknowledgeRoutingStage {
                            selector, acknowledgement: RoutingStageAcknowledgementIntent::Validator(intent), ..
                        } => {
                            let prior = worker_provider.resolve_role_evidence(&transaction, &seal, selector).await?;
                            let acknowledgement = worker_provider.put_validator_stage_ack(
                                &transaction, &seal, &prior, intent,
                            ).await?;
                            let routing = worker_store.complete_routing_authority_in_transaction(
                                &transaction, &seal, &prior, &acknowledgement,
                            ).await?;
                            OperatorConsumptionReadback::Routing {
                                prior: prior.into_readback(), acknowledgement: acknowledgement.into_readback(), routing,
                            }
                        },
                        RoleConsumptionCommand::RecordReviewCompletion { selector, verdict, .. } => {
                            let prior = worker_provider.resolve_role_evidence(&transaction, &seal, selector).await?;
                            let completed = worker_provider.put_validator_verdict(&transaction, &seal, &prior, verdict).await?;
                            OperatorConsumptionReadback::Completion(completed.into_readback())
                        },
                        RoleConsumptionCommand::ResolvePromotion { selectors, input, .. } => {
                            seal.with_promotion_read(async {
                            let operator = worker_provider.resolve_role_evidence(&transaction, &seal, &selectors.operator).await?;
                            let validator = worker_provider.resolve_role_evidence(&transaction, &seal, &selectors.validator).await?;
                            let decision = worker_store.record_promotion_decision_in_transaction(
                                &transaction, &seal, &operator, &validator, input.clone(),
                            ).await?;
                            Ok(OperatorConsumptionReadback::Promotion {
                                operator: operator.into_readback(), validator: validator.into_readback(), decision,
                            })
                            }).await?
                        },
                    };
                    owner.revalidate_in_transaction(&transaction, registry).await?;
                    Ok::<_, ModelLaneError>(readback)
                }),
            ),
        )).await?;
        let registry = retention.owner.consumer_registry().ok_or_else(||
            ModelLaneError::IntegrityViolation("Operator publication registry is unavailable".into()))?;
        retention.owner.revalidate_for_publication(self, registry).await?;
        if let Some(crate::swarm_orchestration::model_lane::RoleConsumptionCommand::IssueInitialValidatorVerdict { issuance, .. }) = retention.owner.consumer_command() {
            issuance.revalidate(self).await?;
        }
        let guard = RolePublicationGuard { owner: &retention.owner, _fence: &retention.fence };
        let result = with_role_routing_context(routing_sources,
            self.with_record_user_scope(retention.owner.auth().record_user_scope(), async {
            match &committed {
                OperatorConsumptionReadback::Review(expected) => {
                    let published = provider.readback_role_evidence(&guard, expected).await?;
                    Ok::<_, ModelLaneError>(RoleConsumptionResult::ReviewInvocation(published.receipt().clone()))
                },
                OperatorConsumptionReadback::Completion(expected) => {
                    let published = provider.readback_role_evidence(&guard, expected).await?;
                    Ok(RoleConsumptionResult::ReviewCompletion(published.receipt().clone()))
                },
                OperatorConsumptionReadback::Promotion { operator, validator, decision } => {
                    guard.with_promotion_read(decision, async {
                    provider.readback_current_promotion_consumption(&guard).await?;
                    provider.readback_role_evidence(&guard, operator).await?;
                    provider.readback_role_evidence(&guard, validator).await?;
                    let decision = store.readback_promotion_decision(&guard, operation_id, decision).await?;
                    Ok(RoleConsumptionResult::Promotion(decision))
                    }).await
                },
                OperatorConsumptionReadback::Routing { prior, acknowledgement, routing } => {
                    provider.readback_role_evidence(&guard, prior).await?;
                    provider.readback_role_evidence(&guard, acknowledgement).await?;
                    let execution = store.readback_routing_authority_commit(&guard, routing).await?;
                    Ok(RoleConsumptionResult::RoutingStageAcknowledgement(execution))
                },
            }
        })).await?;
        retention.owner.revalidate_for_publication(self, registry).await?;
        if let Some(crate::swarm_orchestration::model_lane::RoleConsumptionCommand::IssueInitialValidatorVerdict { issuance, .. }) = retention.owner.consumer_command() {
            issuance.revalidate(self).await?;
        }
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
        read.retention.owner.revalidate_for_publication(self, &capture.registry).await?;
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
            stage, expected: prior.expected.clone(), retention: prior.retention.clone(),
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
        let seal = RoutingAuthorityCaptureReadSeal { stage_id, auth, transaction, _fence: fence };
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
            external_resource_id: workspace_id, action: ResourceAction::Update,
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
        if request.action != ResourceAction::Update || request.capability_id != "fs.write" {
            return Err(ModelLaneError::AuthorityDenied("routing ACK requires original Update authority".into()));
        }
        let mut create_request = request.clone();
        create_request.action = ResourceAction::Create;
        let mut read_request = request.clone();
        read_request.action = ResourceAction::Read;
        read_request.capability_id = "fs.read".to_owned();
        let owner_read = capture_owned_canonical_role_auth(
            self, &bootstrap.capability_registry, credentials.clone(), read_request,
        ).await?;
        let auth = capture_owned_canonical_role_auth(
            self, &bootstrap.capability_registry, credentials.clone(), request,
        ).await?;
        let source_create = capture_owned_canonical_role_auth(
            self, &bootstrap.capability_registry, credentials, create_request,
        ).await?;
        if source_create.exact != auth.exact || source_create.authority.resource_id != auth.authority.resource_id
            || owner_read.exact != auth.exact || owner_read.authority.resource_id != auth.authority.resource_id {
            return Err(ModelLaneError::AuthorityDenied("routing ACK Create authority differs from original subject".into()));
        }
        if store.access().exact_read_scope() != Some(&auth.exact)
            || store.surreal_storage().resource_access_lifecycle() != self.resource_access_lifecycle()
        {
            return Err(ModelLaneError::AuthorityDenied("routing capture differs from current configured authority".into()));
        }
        let fence = std::sync::Arc::new(self.authority_publication_read().await?);
        auth.revalidate_for_publication(self, &bootstrap.capability_registry).await?;
        source_create.revalidate_for_publication(self, &bootstrap.capability_registry).await?;
        owner_read.revalidate_for_publication(self, &bootstrap.capability_registry).await?;
        let provider = super::model_lane::SurrealModelLaneStore::initialize(self.clone()).await?;
        let operation_id = Uuid::now_v7();
        let source_fact = |snapshot: &OwnedCanonicalAuthSnapshot| ProducerSourceFacts {
            request: AuthorizationRequest {
                session_token: snapshot.credentials.session_token.clone(),
                channel_binding_hash: Some(snapshot.credentials.channel_binding_hash.clone()),
                capability_id: snapshot.capability_id.clone(), resource_kind: ResourceKind::Workspace,
                external_resource_id: snapshot.workspace_id.clone(), action: snapshot.action,
            },
            decision: snapshot.decision.clone(), authority: snapshot.authority.clone(),
        };
        let sources = std::sync::Arc::new(RoutingOriginalSources {
            capture: std::sync::Arc::new(ProducerSourceFactsCapture {
                operation_id, credentials: auth.credentials.clone(), session: auth.session.clone(),
                registry: bootstrap.capability_registry.as_ref().clone(), lifecycle: self.resource_access_lifecycle(),
                sources: vec![source_fact(&auth), source_fact(&source_create), source_fact(&owner_read)], fence: fence.clone(),
            }),
            storage: self.clone(), registry: bootstrap.capability_registry.clone(), original: auth.record_user_scope(),
            run_id: String::new(), execution_id: execution_id.clone(), selecting_decision_id: None, input_ref: None,
            create_index: Some(1), additional_create: std::sync::Arc::new(tokio::sync::OnceCell::new()),
        });
        let retention = RoleAuthorityTransactionRetention {
            owner: std::sync::Arc::new(OwnedRoleTransaction::RoutingCapture(OwnedRoutingCapture {
                operation_id, auth, source_create, owner_read, sources, storage: self.clone(), registry: bootstrap.capability_registry.clone(),
                request: RoutingCaptureRequest { execution_id, stage_id, response_message_id },
                execution_fence: std::sync::OnceLock::new(),
                command: std::sync::OnceLock::new(),
            })), fence,
        };
        Ok(RoutingAuthorityCaptureTransaction { storage: self.clone(), retention, store, provider })
    }

    pub(crate) async fn execute_workspace_owner_action(
        &self,
        permit: WorkspaceOwnerActionPermit,
    ) -> Result<crate::swarm_orchestration::model_lane::WorkspaceOwnerActionResult, ResourceAuthorityError> {
        if matches!(&permit.command, crate::swarm_orchestration::model_lane::WorkspaceOwnerActionCommand::IssueArtifactApproval(_)) {
            return self.execute_operator_approval(permit).await;
        }
        let scope = permit.auth.record_user_scope();
        let retention = RoleAuthorityTransactionRetention {
            owner: std::sync::Arc::new(OwnedRoleTransaction::Owner(permit)),
            fence: std::sync::Arc::new(self.authority_publication_read().await?),
        };
        let operation_id = retention.operation_id();
        let provider = super::model_lane::SurrealModelLaneStore::initialize(self.clone()).await?;
        let worker_retention = retention.clone();
        let worker_provider = provider.clone();
        let readback = self.with_record_user_scope(scope,
            self.with_owned_role_authenticated_transaction(
                operation_id, super::ContextOperationCancellation::new(), retention.clone(),
                move |transaction| Box::pin(async move {
                    let OwnedRoleTransaction::Owner(permit) = worker_retention.owner.as_ref() else {
                        return Err(ResourceAuthorityError::InvalidInput("designation requires Owner issuance"));
                    };
                    permit.auth.revalidate_in_transaction(&transaction, &permit.registry).await?;
                    let seal = OwnerIssuanceSeal { permit, transaction: &transaction };
                    let readback = worker_provider.put_runner_designation(&transaction, &seal).await?;
                    permit.auth.revalidate_in_transaction(&transaction, &permit.registry).await?;
                    Ok::<_, ResourceAuthorityError>(readback)
                }),
            ),
        ).await?;
        let OwnedRoleTransaction::Owner(permit) = retention.owner.as_ref() else {
            return Err(ResourceAuthorityError::InvalidInput("designation publication requires Owner issuance"));
        };
        permit.auth.revalidate_for_publication(self, &permit.registry).await?;
        let guard = RolePublicationGuard { owner: &retention.owner, _fence: &retention.fence };
        let result = self.with_record_user_scope(
            permit.auth.record_user_scope(), provider.readback_runner_designation(&guard, &readback),
        ).await?;
        permit.auth.revalidate_for_publication(self, &permit.registry).await?;
        guard.require_unexpired()?;
        self.complete_role_publication(RoleAuthorityPublicationCompletion {
            operation_id, owner: retention.owner.clone(),
        })?;
        Ok(result)
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
        let mut auth = capture_owned_canonical_role_auth(
            self, &bootstrap.capability_registry, credentials, request,
        ).await?;
        auth.requires_workspace_owner = true;
        auth.revalidate_for_publication(self, &bootstrap.capability_registry).await?;
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
        let role_evidence_read = if matches!(&command,
            crate::swarm_orchestration::model_lane::WorkspaceOwnerActionCommand::IssueRunnerDesignation(_))
        {
            let read_request = AuthorizationRequest {
                session_token: auth.credentials.session_token.clone(),
                channel_binding_hash: Some(auth.credentials.channel_binding_hash.clone()),
                resource_kind: ResourceKind::Workspace,
                external_resource_id: auth.workspace_id.clone(),
                capability_id: "fs.read".to_owned(), action: ResourceAction::Read,
            };
            let mut read = capture_owned_canonical_role_auth(
                self, &bootstrap.capability_registry, auth.credentials.clone(), read_request,
            ).await?;
            if read.exact != auth.exact || read.authority.resource_id != auth.authority.resource_id {
                return Err(ResourceAuthorityError::InvalidInput("Owner role Read authority differs from issuance scope"));
            }
            read.requires_workspace_owner = true;
            read.revalidate_for_publication(self, &bootstrap.capability_registry).await?;
            Some(read)
        } else { None };
        Ok(WorkspaceOwnerActionPermit {
            auth, role_evidence_read, registry: bootstrap.capability_registry.clone(), command,
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
        let promotion_read = if let crate::swarm_orchestration::model_lane::RoleConsumptionCommand::ResolvePromotion { selectors, .. } = &command {
            let custody = PROMOTION_READ_CUSTODY.try_with(Clone::clone).ok()
                .ok_or(ResourceAuthorityError::InvalidInput("final promotion requires genuine protected Owner and Validator Read custody"))?;
            custody.revalidate(self).await?;
            let issued = custody.issued_receipt.get()
                .ok_or(ResourceAuthorityError::InvalidInput("final promotion requires observed initial Validator publication"))?;
            let expected_validator = format!("eventledger://role-authority/{}", issued.event.event_id);
            if command.operation_id() == custody.operation_id || command.operation_id() == custody.protected_publication_operation_id
                || auth.action != ResourceAction::Create || auth.capability_id != "fs.write"
                || auth.exact != custody.owner_read.exact
                || auth.authority.resource_id != custody.owner_read.authority.resource_id
                || auth.authority.resource_id != custody.child_read.authority.resource_id
                || selectors.operator.evidence_ref != custody.operator.evidence_ref
                || selectors.operator.expectation != custody.operator.expectation
                || selectors.validator.evidence_ref.as_str() != expected_validator
                || selectors.validator.expectation.receipt_scope != issued.body.scope
                || selectors.validator.expectation.run != issued.body.run
                || selectors.validator.expectation.subject != issued.body.subject
                || selectors.validator.expectation.expected != issued.body.expected
                || selectors.validator.expectation.purpose != issued.body.purpose
                || selectors.validator.designation_ref != issued.body.designation
                || selectors.validator.producer.as_ref() != Some(&custody.producer_binding())
                || !bootstrap.capability_registry.profile_can(&custody.owner_read.authority.capability_profile_id, "fs.read").unwrap_or(false)
                || !bootstrap.capability_registry.profile_can(&custody.child_read.authority.capability_profile_id, "fs.read").unwrap_or(false) {
                return Err(ResourceAuthorityError::InvalidInput("protected final promotion read custody differs from canonical evidence or current creator"));
            }
            Some(custody)
        } else { None };
        if let crate::swarm_orchestration::model_lane::RoleConsumptionCommand::IssueInitialValidatorVerdict { operation_id, operator, issuance } = &command {
            if *operation_id != issuance.operation_id || operator.expectation != issuance.operator.expectation
                || operator.evidence_ref != issuance.operator.evidence_ref
                || operator.designation_ref.is_some() || operator.producer.is_some()
                || auth.exact != issuance.verdict.scope {
                return Err(ResourceAuthorityError::InvalidInput("initial Validator sealed consumer differs"));
            }
            issuance.revalidate(self).await?;
        }
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
                auth, promotion_read, promotion_consumption: std::sync::OnceLock::new(),
                #[cfg(feature = "test-utils")]
                test_wrong_current_operation: TEST_PROMOTION_WRONG_CURRENT_OPERATION.try_with(|value| *value).unwrap_or(false),
                registry: bootstrap.capability_registry.clone(), command,
            })), preparation: None,
        })
    }
}

impl RoleAuthorityAuthInput {
    pub(crate) fn new(session_token: String, channel_binding_hash: String) -> Self {
        Self { session_token, channel_binding_hash }
    }

    pub(crate) fn workspace_creation_request(&self, workspace_id: &str) -> AuthorizationRequest {
        AuthorizationRequest {
            session_token: self.session_token.clone(), channel_binding_hash: Some(self.channel_binding_hash.clone()),
            capability_id: "fs.write".into(), resource_kind: ResourceKind::Workspace,
            external_resource_id: workspace_id.to_owned(), action: ResourceAction::Create,
        }
    }
}

// Authentication facts alone do not establish Workspace Owner eligibility.
pub(crate) struct AuthenticatedValidatorLaunchSeal {
    auth: OwnedCanonicalAuthSnapshot,
    registry: crate::capabilities::CapabilityRegistry,
    storage: SurrealStorage,
    instance: crate::swarm_orchestration::coordinator::RegisteredValidatorInstance,
    parent_session_id: String,
    delegating_scope: crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution,
    expires_at: chrono::DateTime<Utc>,
}

impl AuthenticatedValidatorLaunchSeal {
    pub(crate) fn account_id(&self) -> &str { &self.auth.session.identity.account_id }
    pub(crate) fn principal_id(&self) -> &crate::swarm_orchestration::resource_scope::ActorPrincipalId {
        &self.auth.exact.actor_principal_id
    }
    pub(crate) fn authenticated_session_id(&self) -> &str { &self.auth.session.session_id }
    pub(crate) fn exact_scope(&self) -> &crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution { &self.auth.exact }
    pub(crate) fn expires_at(&self) -> &chrono::DateTime<Utc> { &self.expires_at }
    pub(crate) fn instance_id(&self) -> crate::swarm_orchestration::ModelInstanceId { self.instance.instance_id().clone() }
    pub(crate) fn registered_instance(&self) -> &crate::swarm_orchestration::coordinator::RegisteredValidatorInstance { &self.instance }
    pub(crate) fn parent_session_id(&self) -> &str { &self.parent_session_id }
    pub(crate) fn delegating_scope(&self) -> &crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution { &self.delegating_scope }
    pub(crate) async fn revoke(&self) -> Result<(), ResourceAuthorityError> {
        self.storage.revoke_session(self.authenticated_session_id()).await
    }
    pub(crate) async fn revalidate(&self, storage: &SurrealStorage) -> Result<(), ResourceAuthorityError> {
        if self.storage.resource_access_lifecycle() != storage.resource_access_lifecycle() {
            return Err(ResourceAuthorityError::InvalidInput("validator launch storage owner differs"));
        }
        self.auth.revalidate_for_publication(storage, &self.registry).await
    }
}

struct OwnedCanonicalAuthSnapshot {
    credentials: RoleAuthorityAuthInput,
    session: super::local_accounts::LocalSessionContext,
    decision: AuthorizationDecision,
    authority: RegistryPublicationAuthority,
    workspace_id: String,
    capability_id: String,
    action: ResourceAction,
    exact: crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution,
    requires_workspace_owner: bool,
    created_role_rows: std::sync::Mutex<Vec<ProtectedRoleCreationBinding>>,
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
        if self.requires_workspace_owner {
            let bindings = OwnerEligibilityBindings { session_record: self.authority.session_id.clone() };
            let eligible: Vec<RecordId> = storage.with_record_user_scope(
                self.record_user_scope(), storage.with_data_operation(move |database| {
                    Box::pin(async move { database.query_values(OWNER_ELIGIBILITY, bindings).await })
                }),
            ).await?;
            if eligible.as_slice() != std::slice::from_ref(&self.authority.session_id) { return Err(denied()); }
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

    async fn revalidate_read_in_child_transaction(
        &self, transaction: &super::SurrealTransactionContext<'_>, registry: &crate::capabilities::CapabilityRegistry,
    ) -> Result<(), ResourceAuthorityError> {
        let denied = || ResourceAuthorityError::Denied { decision_id: self.decision.decision_id.clone() };
        let current = super::current_record_user_scope().ok_or_else(denied)?;
        if self.action != ResourceAction::Read || self.capability_id != "fs.read"
            || current.session_token != self.credentials.session_token
            || current.channel_binding_hash.as_deref() != Some(self.credentials.channel_binding_hash.as_str())
            || current.session_id != self.decision.session_id || current.resource_id != self.decision.resource_id
            || current.workspace_id.as_deref() != Some(self.workspace_id.as_str())
            || !registry.profile_can(&self.authority.capability_profile_id, "fs.read").unwrap_or(false) {
            return Err(denied());
        }
        let rows: Vec<RegistryPublicationAuthority> = transaction.query_values_at(REGISTRY_PUBLICATION_AUTHORITY,
            RegistryPublicationBindings { resource: self.authority.resource_id.clone(), original_grant: self.authority.grant_id.clone(),
                kind: self.authority.resource_kind.clone(), action: "read".to_owned(), capability: "fs.read".to_owned() }, 0).await?;
        if rows.as_slice() != std::slice::from_ref(&self.authority) { return Err(denied()); }
        self.require_unexpired()
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
        if self.requires_workspace_owner {
            let eligible: Vec<RecordId> = transaction.query_values_at(
                OWNER_ELIGIBILITY, OwnerEligibilityBindings { session_record: self.authority.session_id.clone() }, 0,
            ).await?;
            if eligible.as_slice() != std::slice::from_ref(&self.authority.session_id) { return Err(denied()); }
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
        requires_workspace_owner: false,
        created_role_rows: std::sync::Mutex::new(Vec::new()),
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
    AccountAdministration,
    RichDocument,
    KnowledgeSource,
    KnowledgeCodeFile,
    LoomBlock,
    FlightRecorder,
    PalmistryWatcher,
    Artifact,
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
            Self::AccountAdministration => "account_administration",
            Self::RichDocument => "rich_document",
            Self::KnowledgeSource => "knowledge_source",
            Self::KnowledgeCodeFile => "knowledge_code_file",
            Self::LoomBlock => "loom_block",
            Self::FlightRecorder => "flight_recorder",
            Self::PalmistryWatcher => "palmistry_watcher",
            Self::Artifact => "artifact",
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

#[derive(Eq, PartialEq, SurrealValue)]
pub(crate) struct CanonicalResourceLocator {
    pub id: RecordId,
    pub external_resource_id: String,
    pub owner_account_id: RecordId,
    pub access_space_id: RecordId,
    pub parent_resource_id: Option<RecordId>,
    pub lifecycle_state: String,
    pub policy_version: i64,
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
                        "LET $eligible = SELECT id FROM principals WHERE id = $principal AND account_id = $account AND status = 'enabled' AND capability_profile_id != 'MemberEnrollmentAdministrator' AND $space.account_id = $account AND $space.status = 'active' LIMIT 1; IF array::len($eligible) = 0 { THROW 'HSK-AUTH-CREDENTIAL-IDENTITY'; }; CREATE $credential SET account_id = $account, principal_id = $principal, access_space_id = $space, token_hash = $token_hash, status = 'active', expires_at = $expires_at, revoked_at = NONE, created_at = time::now();",
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
                            "SELECT account_id.revocation_epoch AS account_epoch, revocation_epoch AS principal_epoch, $space.revocation_epoch AS space_epoch, math::max([account_id.policy_version, policy_version, $space.policy_version]) AS policy_version, delegated_capabilities FROM $principal WHERE account_id = $account AND status = 'enabled' AND capability_profile_id != 'MemberEnrollmentAdministrator' AND $space.account_id = $account AND $space.status = 'active';",
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
        if resource_kind == ResourceKind::AccountAdministration {
            return Err(ResourceAuthorityError::InvalidInput("explicit authorized administration lifetime required"));
        }
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
                "SELECT id, external_resource_id, owner_account_id, access_space_id, parent_resource_id, lifecycle_state, policy_version FROM protected_resources WHERE resource_kind = $kind AND external_resource_id = $external LIMIT 2;",
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

    /// Identity metadata only; adoption must recheck the parent and grant in its guarded transaction.
    pub(crate) async fn canonical_watcher_workspace_parent(
        &self, watcher_id: Uuid,
    ) -> Result<Option<CanonicalResourceLocator>, ResourceAuthorityError> {
        let Some(watcher) = self.canonical_resource_locator(
            ResourceKind::PalmistryWatcher, &watcher_id.to_string(),
        ).await? else { return Ok(None); };
        if watcher.lifecycle_state != "active" { return Ok(None); }
        let Some(parent_id) = watcher.parent_resource_id else { return Ok(None); };
        let namespace = self.config().namespace().to_owned();
        let database = self.config().database().to_owned();
        let account = watcher.owner_account_id;
        let space = watcher.access_space_id;
        let expected_parent = parent_id.clone();
        let rows = self.with_lease(move |client| Box::pin(async move {
            let authority = client.clone();
            authority.use_ns(namespace).use_db(database).await?;
            let mut response = handshake_storage_support::diagnostics::observe_result("authority_resource_query", authority.query(
                "SELECT id, external_resource_id, owner_account_id, access_space_id, parent_resource_id, lifecycle_state, policy_version FROM protected_resources WHERE id = $parent AND resource_kind = 'workspace' AND lifecycle_state = 'active' AND owner_account_id = $account AND access_space_id = $space LIMIT 2;",
            ).bind(("parent", parent_id)).bind(("account", account)).bind(("space", space))).await?.check()?;
            let rows: Vec<CanonicalResourceLocator> = response.take(0)?;
            Ok(rows)
        })).await?;
        let parent = match rows.len() {
            0 => return Ok(None),
            1 => rows.into_iter().next().ok_or(ResourceAuthorityError::InvalidInput("watcher parent unavailable"))?,
            _ => return Err(ResourceAuthorityError::InvalidInput("watcher parent is not canonical")),
        };
        let Some(canonical) = self.canonical_resource_locator(
            ResourceKind::Workspace, &parent.external_resource_id,
        ).await? else { return Ok(None); };
        if canonical.id != expected_parent || canonical.id != parent.id
            || canonical.owner_account_id != parent.owner_account_id
            || canonical.access_space_id != parent.access_space_id
            || canonical.lifecycle_state != "active"
            || canonical.policy_version != parent.policy_version
        {
            return Err(ResourceAuthorityError::InvalidInput("watcher workspace parent changed"));
        }
        Ok(Some(canonical))
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
            || ((request.resource_kind == ResourceKind::Artifact || request.capability_id == "artifact.read")
                && (request.resource_kind != ResourceKind::Artifact
                    || request.action != ResourceAction::Read || request.capability_id != "artifact.read"))
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

// NEW Source027 private Account-to-schema binding consumer.
// Defined in resource_authority.rs, alongside existing protected role bindings.
// Native validator_sessions child module alone constructs it after genuine
// same-frame child Create+Read capture and full canonical envelope attestation.
// The existing append phase overwrites this key after every ordinary binding.
#[derive(Clone, SurrealValue)]
struct ProtectedValidatorAssociationBinding {
    operation_id: String,
    record_id: String,
    event_id: String,
    original_stream: String,
    record_json: String,
    outer_json: String,
    original_scope: ProtectedRoleScopeBinding,
    child_scope: ProtectedRoleScopeBinding,
    expected_child_create: RegistryPublicationAuthority,
    expected_child_read: RegistryPublicationAuthority,
}

tokio::task_local! {
    static PROTECTED_VALIDATOR_ASSOCIATION_BINDING: ProtectedValidatorAssociationBinding;
}

fn append_validator_association_binding(bindings: &mut surrealdb::types::Value) {
    if let surrealdb::types::Value::Object(object) = bindings {
        object.insert(
            "wp012_validator_association",
            PROTECTED_VALIDATOR_ASSOCIATION_BINDING.try_with(Clone::clone).ok(),
        );
    }
}



use handshake_member_enrollment::command_plan as enrollment;
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
use crate::model_runtime::cloud::member_enrollment::{AccountError, AccountFuture, AccountOriginalController, AccountResult, AccountRevision, OriginalIssueIntent, OriginalIssueRow, OriginalIssueState, OriginalSubmission};


// RAM DRAFT ONLY. Prospective addition to resource_authority.rs.
// Not applied, adopted, compiled, or a provider/pin readiness claim.

#[derive(Clone)]
struct CapturedAdministrationAuthority {
    credentials: RoleAuthorityAuthInput,
    session: super::local_accounts::LocalSessionContext,
    decision: AuthorizationDecision,
    facts: RegistryPublicationAuthority,
    exact: enrollment::IdentityScope,
    original_scope: RecordUserScope,
}
struct CapturedMemberEnrollment {
    plan: enrollment::EnrollmentPlan,
    enroll: CapturedAdministrationAuthority,
    assign: CapturedAdministrationAuthority,
    lifetime: CapturedAdministrationLifetime,
}
struct CapturedMemberRevocation {
    plan: enrollment::RevocationPlan,
    current_revoker: CapturedAdministrationAuthority,
}
fn enrollment_time(value: &Datetime) -> Result<enrollment::Timestamp, ResourceAuthorityError> {
    let value = value.clone().into_inner();
    enrollment::Timestamp::new(value.timestamp(), value.timestamp_subsec_nanos())
        .map_err(|_| ResourceAuthorityError::InvalidInput("invalid enrollment timestamp"))
}
fn enrollment_source(facts: &RegistryPublicationAuthority)
    -> Result<enrollment::SourceRef, ResourceAuthorityError>
{
    enrollment::SourceRef::new(
        record_key(facts.resource_id.clone())?, record_key(facts.grant_id.clone())?,
        facts.grant_version, facts.policy_versions.clone(), facts.revocation_epochs.clone(),
        facts.grant_expires_at.as_ref().map(enrollment_time).transpose()?,
    ).map_err(|_| ResourceAuthorityError::InvalidInput("invalid enrollment source"))
}
impl SurrealStorage {
    async fn capture_current_administration(
        &self, registry: &crate::capabilities::CapabilityRegistry,
        credentials: RoleAuthorityAuthInput, kind: enrollment::CommandKind,
        administrator_resource: &str, workspace: &str,
    ) -> Result<CapturedAdministrationAuthority, ResourceAuthorityError> {
        // REQUIRED OWNER DEPENDENCY: prospective ResourceKind::AccountAdministration,
        // canonical registered resource, parent-workspace link, and capability/profile gates.
        let action = match kind.action() {
            enrollment::AdminAction::Create => ResourceAction::Create,
            enrollment::AdminAction::Update => ResourceAction::Update,
        };
        let session = self.authenticate_local_session(
            &credentials.session_token, &credentials.channel_binding_hash,
        ).await?;
        let decision = self.authorize_protected_resource(AuthorizationRequest {
            session_token: credentials.session_token.clone(),
            channel_binding_hash: Some(credentials.channel_binding_hash.clone()),
            capability_id: kind.capability_id().to_owned(),
            resource_kind: ResourceKind::AccountAdministration,
            external_resource_id: administrator_resource.to_owned(), action,
        }).await?;
        let denied = || ResourceAuthorityError::Denied { decision_id: decision.decision_id.clone() };
        // Each identity axis is checked independently; never substitute Account for Principal/Space.
        if session.identity.account_id != decision.account_id
            || session.identity.principal_id != decision.principal_id
            || session.session_id != decision.session_id
            || session.identity.access_space_id != decision.access_space_id
            || session.delegation_chain != decision.delegation_chain
            || !registry.profile_can(&decision.capability_profile_id, kind.capability_id()).unwrap_or(false)
        { return Err(denied()); }
        let administration = self.canonical_resource_locator(
            ResourceKind::AccountAdministration, administrator_resource,
        ).await?.ok_or_else(denied)?;
        let parent = self.canonical_resource_locator(
            ResourceKind::Workspace, workspace,
        ).await?.ok_or_else(denied)?;
        if administration.id != RecordId::new("protected_resources", decision.resource_id.clone())
            || administration.owner_account_id != RecordId::new("local_accounts", decision.account_id.clone())
            || administration.access_space_id != RecordId::new("access_spaces", decision.access_space_id.clone())
            || administration.lifecycle_state != "active"
            || administration.parent_resource_id.as_ref() != Some(&parent.id)
            || parent.owner_account_id != administration.owner_account_id
            || parent.access_space_id != administration.access_space_id
            || parent.lifecycle_state != "active"
        { return Err(denied()); }
        let original_scope = RecordUserScope {
            grant_id: Some(decision.grant_id.clone()), workspace_id: Some(workspace.to_owned()),
            session_token: credentials.session_token.clone(),
            channel_binding_hash: Some(credentials.channel_binding_hash.clone()),
            resource_id: decision.resource_id.clone(), session_id: decision.session_id.clone(),
            capability_id: kind.capability_id().to_owned(), action,
        };
        let bindings = RegistryPublicationBindings {
            resource: administration.id, original_grant: RecordId::new("resource_grants", decision.grant_id.clone()),
            kind: ResourceKind::AccountAdministration.as_str().to_owned(),
            action: action.as_str().to_owned(), capability: kind.capability_id().to_owned(),
        };
        let rows: Vec<RegistryPublicationAuthority> = self.with_record_user_scope(
            original_scope.clone(), self.with_data_operation(move |database| {
                Box::pin(async move { database.query_values(REGISTRY_PUBLICATION_AUTHORITY, bindings).await })
            }),
        ).await?;
        let [facts] = rows.as_slice() else { return Err(denied()); };
        if record_key(facts.account_id.clone())? != decision.account_id
            || record_key(facts.principal_id.clone())? != decision.principal_id
            || record_key(facts.session_id.clone())? != decision.session_id
            || record_key(facts.access_space_id.clone())? != decision.access_space_id
            || record_key(facts.resource_id.clone())? != decision.resource_id
            || record_key(facts.grant_id.clone())? != decision.grant_id
            || facts.capability_profile_id != decision.capability_profile_id
            || facts.delegation_chain != decision.delegation_chain
            || facts.policy_versions.first().is_none_or(|policy| parent.policy_version > *policy)
            || facts.session_expires_at.clone().into_inner() <= Utc::now()
            || facts.grant_expires_at.as_ref().is_some_and(|value| value.clone().into_inner() <= Utc::now())
        { return Err(denied()); }
        let exact = enrollment::IdentityScope::new(
            decision.account_id.clone(), decision.principal_id.clone(), decision.session_id.clone(),
            decision.access_space_id.clone(), workspace.to_owned(),
        ).map_err(|_| ResourceAuthorityError::InvalidInput("invalid administration identity"))?;
        Ok(CapturedAdministrationAuthority { credentials, session, decision, facts: facts.clone(), exact, original_scope })
    }

    async fn capture_member_enrollment_first(
        &self, registry: &crate::capabilities::CapabilityRegistry,
        credentials: RoleAuthorityAuthInput, plan: enrollment::EnrollmentPlan,
        lifetime: CapturedAdministrationLifetime,
    ) -> Result<CapturedMemberEnrollment, ResourceAuthorityError> {
        use enrollment::CommandKind;
        let [create, assign] = plan.intents();
        if create.kind() != CommandKind::Enroll || assign.kind() != CommandKind::AssignFixedMemberProfile {
            return Err(ResourceAuthorityError::InvalidInput("enrollment plan actions differ"));
        }
        let enroll = self.capture_current_administration(registry, credentials.clone(),
            create.kind(), create.administrator_resource(), create.authorizer().workspace_id()).await?;
        let assignment = self.capture_current_administration(registry, credentials,
            assign.kind(), assign.administrator_resource(), assign.authorizer().workspace_id()).await?;
        for (intent, captured) in [(create, &enroll), (assign, &assignment)] {
            if intent.authorizer() != &captured.exact || intent.source() != &enrollment_source(&captured.facts)? {
                return Err(ResourceAuthorityError::InvalidInput("initial enrollment source differs"));
            }
        }
        if enroll.exact != assignment.exact
            || enrollment_time(&enroll.facts.session_expires_at)? != plan.descriptor().session_expires_at()
            || enrollment_time(&assignment.facts.session_expires_at)? != plan.descriptor().session_expires_at()
        { return Err(ResourceAuthorityError::InvalidInput("enrollment session binding differs")); }
        lifetime.require_matches_source(&enroll.facts)?;
        lifetime.require_matches_source(&assignment.facts)?;
        lifetime.require_initial_descriptor(plan.descriptor())?;
        handshake_member_enrollment::invitation::validate_expiry(plan.descriptor())
            .map_err(|_| ResourceAuthorityError::InvalidInput("enrollment deadline differs"))?;
        Ok(CapturedMemberEnrollment { plan, enroll, assign: assignment, lifetime })
    }

    async fn capture_member_revocation_first(
        &self, registry: &crate::capabilities::CapabilityRegistry,
        credentials: RoleAuthorityAuthInput, plan: enrollment::RevocationPlan,
    ) -> Result<CapturedMemberRevocation, ResourceAuthorityError> {
        let intent = plan.intent();
        if intent.kind() != enrollment::CommandKind::RevokeEnrollment {
            return Err(ResourceAuthorityError::InvalidInput("revocation plan action differs"));
        }
        let current = self.capture_current_administration(registry, credentials,
            intent.kind(), intent.administrator_resource(), intent.authorizer().workspace_id()).await?;
        if current.exact != *plan.request().current_revoker()
            || enrollment_source(&current.facts)? != *plan.request().source()
            || plan.request().request_id() == plan.request().enrollment_operation_id()
        { return Err(ResourceAuthorityError::InvalidInput("current revoker binding differs")); }
        // No comparison with original_enroller: an independently authorized revoker is allowed.
        Ok(CapturedMemberRevocation { plan, current_revoker: current })
    }
}
// Transaction/publication adapter remains pending the exact provider contracts:
// re-read EACH captured grant/five/chain under its original action in the borrowed transaction;
// preserve immutable original intent on retry and use separate fresh current gates;
// never call the *_first methods to reconstruct/reissue an existing intent.

struct CurrentEnrollmentReplayGates {
    enroll: CapturedAdministrationAuthority,
    assign: CapturedAdministrationAuthority,
    lifetime: CapturedAdministrationLifetime,
}
impl SurrealStorage {
    async fn capture_member_enrollment_replay(
        &self, registry: &crate::capabilities::CapabilityRegistry,
        credentials: RoleAuthorityAuthInput, original: &enrollment::EnrollmentDescriptor,
        lifetime: CapturedAdministrationLifetime,
    ) -> Result<CurrentEnrollmentReplayGates, ResourceAuthorityError> {
        let issuer = original.original_enroller();
        let enroll = self.capture_current_administration(registry, credentials.clone(),
            enrollment::CommandKind::Enroll, original.administrator_resource(), issuer.workspace_id()).await?;
        let assign = self.capture_current_administration(registry, credentials,
            enrollment::CommandKind::AssignFixedMemberProfile,
            original.administrator_resource(), issuer.workspace_id()).await?;
        for gate in [&enroll, &assign] {
            if gate.exact.owner_account_id() != issuer.owner_account_id()
                || gate.exact.actor_principal_id() != issuer.actor_principal_id()
                || gate.exact.access_space_id() != issuer.access_space_id()
                || gate.exact.workspace_id() != issuer.workspace_id()
            { return Err(ResourceAuthorityError::InvalidInput("enrollment replay actor differs")); }
        }
        if enroll.exact != assign.exact {
            return Err(ResourceAuthorityError::InvalidInput("enrollment replay current session differs"));
        }
        lifetime.require_matches_source(&enroll.facts)?;
        lifetime.require_matches_source(&assign.facts)?;
        lifetime.require_preserved_expiry(original)?;
        // Deliberately retain the original descriptor, sources, session, issue/expiry and verifier.
        // These gates authorize redelivery only. They cannot create a replacement descriptor/secret.
        Ok(CurrentEnrollmentReplayGates { enroll, assign, lifetime })
    }
    async fn capture_member_revocation_replay(
        &self, registry: &crate::capabilities::CapabilityRegistry,
        credentials: RoleAuthorityAuthInput, original_request: &enrollment::RevocationRequest,
    ) -> Result<CapturedAdministrationAuthority, ResourceAuthorityError> {
        let actor = original_request.current_revoker();
        let gate = self.capture_current_administration(registry, credentials,
            enrollment::CommandKind::RevokeEnrollment,
            original_request.administrator_resource(), actor.workspace_id()).await?;
        if gate.exact.owner_account_id() != actor.owner_account_id()
            || gate.exact.actor_principal_id() != actor.actor_principal_id()
            || gate.exact.access_space_id() != actor.access_space_id()
            || gate.exact.workspace_id() != actor.workspace_id()
        { return Err(ResourceAuthorityError::InvalidInput("revocation replay actor differs")); }
        // No equality with the original enroller or replacement of the committed request identity.
        Ok(gate)
    }
}

// RAM ONLY: required input from the AccountAdministration registration/schema owner.
// No storage-field name, row, policy default, or provider implementation is asserted here.
#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum AdministrationLifetimeChoice {
    NoExpiry,
    ExpiresAt(enrollment::Timestamp),
}
#[derive(Clone, Eq, PartialEq)]
struct CapturedAdministrationLifetime {
    administration_resource_id: RecordId,
    policy_version: i64,
    choice: AdministrationLifetimeChoice,
}
impl CapturedAdministrationLifetime {
    fn source030_bound(&self) -> Option<enrollment::Timestamp> {
        match self.choice {
            AdministrationLifetimeChoice::NoExpiry => None,
            AdministrationLifetimeChoice::ExpiresAt(value) => Some(value),
        }
    }
    fn require_matches_source(&self, facts: &RegistryPublicationAuthority)
        -> Result<(), ResourceAuthorityError>
    {
        // The inspected projection orders policy versions: grant, resource, Account,
        // Principal, AccessSpace, session. A missing resource-policy version is an error.
        if self.administration_resource_id != facts.resource_id
            || facts.policy_versions.get(1) != Some(&self.policy_version)
        {
            return Err(ResourceAuthorityError::InvalidInput("administration policy source differs"));
        }
        Ok(())
    }
    fn require_initial_descriptor(&self, original: &enrollment::EnrollmentDescriptor)
        -> Result<(), ResourceAuthorityError>
    {
        if original.policy_expires_at() != self.source030_bound() {
            return Err(ResourceAuthorityError::InvalidInput("enrollment policy bound differs"));
        }
        Ok(())
    }
    fn require_preserved_expiry(&self, original: &enrollment::EnrollmentDescriptor)
        -> Result<(), ResourceAuthorityError>
    {
        if let AdministrationLifetimeChoice::ExpiresAt(current) = self.choice {
            if original.expires_at() > current {
                return Err(ResourceAuthorityError::InvalidInput("current policy no longer permits original expiry"));
            }
        }
        Ok(())
    }
}
// Required provider-owned service, not an implemented or existing API:
// read_registered_administration_lifetime(
//     tx: &SurrealTransactionContext<'_>,
//     current_record_scope: &RecordUserScope,
//     administration_resource_id: &RecordId,
//     expected_policy_version: i64,
// ) -> Result<CapturedAdministrationLifetime, ResourceAuthorityError>
//
// The provider constructs this value ONLY from the protected canonical registered
// AccountAdministration resource's closed lifetime choice and actual resource identity/version.
// Absent/malformed/unsupported/ambiguous storage values return an error, never NoExpiry.
// The identical service runs during protected capture and again before commit;
// equality includes resource identity, policy version, choice and exact timestamp.
// It must use the owner-selected closed metadata path; no general resource_policies table.
//


// These comparison guards consume provider-returned current observations from the SAME
// authorized borrowed transaction. They neither acquire a transaction nor mint observation authority.
fn require_precommit_administration_pair(
    enroll: &CapturedAdministrationAuthority, assign: &CapturedAdministrationAuthority,
    captured_lifetime: &CapturedAdministrationLifetime,
    current_enroll: &RegistryPublicationAuthority, current_assign: &RegistryPublicationAuthority,
    current_lifetime: &CapturedAdministrationLifetime,
) -> Result<(), ResourceAuthorityError> {
    if current_enroll != &enroll.facts || current_assign != &assign.facts
        || current_lifetime != captured_lifetime
    {
        return Err(ResourceAuthorityError::InvalidInput("administration precommit observation changed"));
    }
    current_lifetime.require_matches_source(current_enroll)?;
    current_lifetime.require_matches_source(current_assign)?;
    let now = Utc::now();
    for facts in [current_enroll, current_assign] {
        if facts.session_expires_at.clone().into_inner() <= now
            || facts.grant_expires_at.as_ref().is_some_and(|expiry| expiry.clone().into_inner() <= now)
        {
            return Err(ResourceAuthorityError::InvalidInput("administration precommit authority expired"));
        }
    }
    Ok(())
}
impl CapturedMemberEnrollment {
    fn require_precommit(
        &self, current_enroll: &RegistryPublicationAuthority,
        current_assign: &RegistryPublicationAuthority,
        current_lifetime: &CapturedAdministrationLifetime,
    ) -> Result<(), ResourceAuthorityError> {
        require_precommit_administration_pair(
            &self.enroll, &self.assign, &self.lifetime,
            current_enroll, current_assign, current_lifetime,
        )?;
        current_lifetime.require_initial_descriptor(self.plan.descriptor())?;
        handshake_member_enrollment::invitation::validate_expiry(self.plan.descriptor())
            .map_err(|_| ResourceAuthorityError::InvalidInput("enrollment precommit deadline differs"))?;
        let now = enrollment_time(&Datetime::from(Utc::now()))?;
        if self.plan.descriptor().expires_at() <= now {
            return Err(ResourceAuthorityError::InvalidInput("enrollment invitation expired before first commit"));
        }
        Ok(())
    }
}
impl CurrentEnrollmentReplayGates {
    fn require_precommit(
        &self, original: &enrollment::EnrollmentDescriptor,
        current_enroll: &RegistryPublicationAuthority,
        current_assign: &RegistryPublicationAuthority,
        current_lifetime: &CapturedAdministrationLifetime,
    ) -> Result<(), ResourceAuthorityError> {
        require_precommit_administration_pair(
            &self.enroll, &self.assign, &self.lifetime,
            current_enroll, current_assign, current_lifetime,
        )?;
        current_lifetime.require_preserved_expiry(original)
    }
}


// STEP2 RAM-ONLY prospective additions to resource_authority.rs.
// ResourceKind addition: AccountAdministration.
// ResourceKind::as_str addition: Self::AccountAdministration => "account_administration".
// Existing register_protected_resource entrypoint retains its signature and legacy body.
// Add this guard BEFORE that body's existing UUID allocation/with_lease:
// if resource_kind == ResourceKind::AccountAdministration {
//     return Err(ResourceAuthorityError::InvalidInput("explicit authorized administration lifetime required"));
// }
// Thus the old API never creates an Admin resource with absent/default lifetime.

#[derive(Clone, Debug, Eq, PartialEq)]
enum AdministrationLifetimeRow {
    NoExpiry,
    ExpiresAt(Datetime),
}
impl AdministrationLifetimeRow {
    fn from_explicit(choice: AdministrationLifetimeChoice) -> Result<Self, ResourceAuthorityError> {
        match choice {
            AdministrationLifetimeChoice::NoExpiry => Ok(Self::NoExpiry),
            AdministrationLifetimeChoice::ExpiresAt(timestamp) => Ok(Self::ExpiresAt(Datetime::from(
                chrono::DateTime::<Utc>::from_timestamp(timestamp.unix_seconds(), timestamp.nanoseconds())
                    .ok_or(ResourceAuthorityError::InvalidInput("administration policy timestamp outside host range"))?
            ))),
        }
    }
    fn closed_choice(&self) -> Result<AdministrationLifetimeChoice, ResourceAuthorityError> {
        match self {
            Self::NoExpiry => Ok(AdministrationLifetimeChoice::NoExpiry),
            Self::ExpiresAt(expiry) => Ok(AdministrationLifetimeChoice::ExpiresAt(enrollment_time(expiry)?)),
        }
    }
}
impl SurrealValue for AdministrationLifetimeRow {
    fn kind_of() -> surrealdb::types::Kind {
        <surrealdb::types::Object as SurrealValue>::kind_of()
    }
    fn is_value(value: &surrealdb::types::Value) -> bool {
        Self::from_value(value.clone()).is_ok()
    }
    fn into_value(self) -> surrealdb::types::Value {
        let mut object = surrealdb::types::Object::new();
        match self {
            Self::NoExpiry => {
                object.insert("mode", "no_expiry".to_owned());
            }
            Self::ExpiresAt(expiry) => {
                object.insert("mode", "expires_at".to_owned());
                object.insert("expires_at", expiry);
            }
        }
        object.into_value()
    }
    fn from_value(value: surrealdb::types::Value) -> Result<Self, surrealdb::types::Error> {
        use surrealdb::types::Value;
        if let Value::Object(object) = value {
            match (object.get("mode"), object.get("expires_at"), object.len()) {
                (Some(Value::String(mode)), None, 1) if mode == "no_expiry" => {
                    return Ok(Self::NoExpiry);
                }
                (Some(Value::String(mode)), Some(Value::Datetime(expiry)), 2) if mode == "expires_at" => {
                    return Ok(Self::ExpiresAt(expiry.clone()));
                }
                _ => {}
            }
        }
        Err(surrealdb::types::Error::validation(
            "administration lifetime must contain exactly mode=no_expiry or mode=expires_at with a datetime expires_at".to_owned(),
            None,
        ))
    }
}

// New field on RegistryPublicationAuthority, not any Source030/policy structure:
// administration_lifetime: Option<AdministrationLifetimeRow>,
// Append ONLY this expression to the existing REGISTRY_PUBLICATION_AUTHORITY projection:
// resource_id.account_administration_lifetime AS administration_lifetime
// Keep the canonical_grant_query predicate, complete vector order and both expiry columns intact.
// Legacy resource kinds have None metadata. None on the NEW Admin kind is an error.

#[derive(Clone, Eq, PartialEq)]
pub(super) struct AccountAdministrationRegistrationInput {
    pub(super) external_resource_id: String,
    pub(super) parent_workspace_external_id: String,
    pub(super) classification: String,
    pub(super) resource_policy_version: i64,
    pub(super) lifetime: AdministrationLifetimeChoice,
}
#[derive(SurrealValue)]
struct AdministrationRegistrationBindings {
    resource: RecordId,
    account: RecordId,
    principal: RecordId,
    session: RecordId,
    space: RecordId,
    parent: RecordId,
    external: String,
    classification: String,
    policy_version: i64,
    lifetime: AdministrationLifetimeRow,
}
#[derive(SurrealValue)]
struct AdministrationRegistrationRow {
    id: RecordId,
    resource_kind: String,
    created_by_principal_id: RecordId,
    created_in_session_id: RecordId,
    external_resource_id: String,
    owner_account_id: RecordId,
    access_space_id: RecordId,
    parent_resource_id: RecordId,
    policy_version: i64,
    lifecycle_state: String,
    classification: String,
    account_administration_lifetime: AdministrationLifetimeRow,
}

pub(super) struct CapturedAccountAdministrationRegistrationAuthority<'tx> {
    confirmation: ConfirmedFirstOwnerAdministrationRegistration<'tx>,
    input: AccountAdministrationRegistrationInput,
}

impl<'tx> CapturedAccountAdministrationRegistrationAuthority<'tx> {
    pub(super) fn from_confirmation(
        confirmation: ConfirmedFirstOwnerAdministrationRegistration<'tx>,
        input: AccountAdministrationRegistrationInput,
    ) -> Result<Self, ResourceAuthorityError> {
        confirmation.require_input(&input)?;
        Ok(Self { confirmation, input })
    }

    fn owner(&self) -> &ProvisionedIdentity { self.confirmation.owner() }
    fn session_id(&self) -> &str { self.confirmation.session_id() }
    fn parent_workspace(&self) -> &CanonicalResourceLocator {
        self.confirmation.parent_workspace()
    }
    fn parent_workspace_external_id(&self) -> &str {
        self.confirmation.parent_workspace_external_id()
    }

    fn require_input(
        &self,
        input: &AccountAdministrationRegistrationInput,
    ) -> Result<(), ResourceAuthorityError> {
        if input != &self.input {
            return Err(ResourceAuthorityError::InvalidInput(
                "administration registration original input differs",
            ));
        }
        self.confirmation.require_input(input)
    }

    fn require_original_resource(&self, resource: &RecordId) -> Result<(), ResourceAuthorityError> {
        let key = record_key(resource.clone())?;
        validate_uuid(&key)?;
        if resource != &RecordId::new("protected_resources", key)
            || resource != self.confirmation.administration_resource()
        {
            return Err(ResourceAuthorityError::InvalidInput(
                "administration registration original resource differs",
            ));
        }
        Ok(())
    }

    async fn require_current_in_transaction(
        &self,
        transaction: &super::SurrealTransactionContext<'_>,
    ) -> Result<(), ResourceAuthorityError> {
        self.confirmation.require_input(&self.input)?;
        self.confirmation.require_current_in_transaction(transaction).await
    }
}

impl SurrealStorage {
    pub(super) async fn register_account_administration_in_transaction(
        &self,
        transaction: &super::SurrealTransactionContext<'_>,
        authority: &CapturedAccountAdministrationRegistrationAuthority<'_>,
        original_resource: &RecordId,
        input: AccountAdministrationRegistrationInput,
    ) -> Result<RegisteredResource, ResourceAuthorityError> {
        authority.require_current_in_transaction(transaction).await?;
        authority.require_original_resource(original_resource)?;
        authority.require_input(&input)?;
        let owner = authority.owner();
        for id in [&owner.account_id, &owner.principal_id, &owner.access_space_id] { validate_uuid(id)?; }
        validate_uuid(authority.session_id())?;
        validate_nonempty(&input.external_resource_id)?;
        validate_nonempty(&input.classification)?;
        if input.resource_policy_version < 1
            || input.parent_workspace_external_id != authority.parent_workspace_external_id()
        { return Err(ResourceAuthorityError::InvalidInput("administration registration binding differs")); }
        let parent = authority.parent_workspace();
        let account = RecordId::new("local_accounts", owner.account_id.clone());
        let space = RecordId::new("access_spaces", owner.access_space_id.clone());
        if parent.owner_account_id != account || parent.access_space_id != space
            || parent.lifecycle_state != "active"
        { return Err(ResourceAuthorityError::InvalidInput("administration parent scope differs")); }
        let lifetime = AdministrationLifetimeRow::from_explicit(input.lifetime)?;
        let expected_external = input.external_resource_id.clone();
        let expected_classification = input.classification.clone();
        let expected_policy = input.resource_policy_version;
        let expected_parent = parent.id.clone();
        let expected_lifetime = lifetime.clone();
        let rows: Vec<AdministrationRegistrationRow> = transaction.query_values_at(
            "LET $existing = SELECT VALUE id FROM protected_resources WHERE resource_kind = 'account_administration' AND external_resource_id = $external LIMIT 2; \
             IF array::len($existing) > 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; }; \
             IF array::len($existing) = 0 { \
                 CREATE $resource SET resource_kind = 'account_administration', external_resource_id = $external, \
                 owner_account_id = $account, created_by_principal_id = $principal, created_in_session_id = $session, \
                 access_space_id = $space, parent_resource_id = $parent, schema_version = 1, lifecycle_state = 'active', \
                 policy_version = $policy_version, classification = $classification, \
                 account_administration_lifetime = $lifetime, \
                 storage_locator_hash = crypto::sha256('account_administration:' + $external), \
                 created_at = time::now(), updated_at = time::now(); \
             }; \
             SELECT id, resource_kind, created_by_principal_id, created_in_session_id, external_resource_id, owner_account_id, access_space_id, parent_resource_id, \
                 policy_version, lifecycle_state, classification, account_administration_lifetime \
                 FROM protected_resources WHERE resource_kind = 'account_administration' AND external_resource_id = $external LIMIT 2;",
            AdministrationRegistrationBindings {
                resource: original_resource.clone(),
                account: account.clone(), principal: RecordId::new("principals", owner.principal_id.clone()),
                session: RecordId::new("authenticated_sessions", authority.session_id().to_owned()),
                space: space.clone(), parent: expected_parent.clone(), external: input.external_resource_id,
                classification: input.classification, policy_version: expected_policy, lifetime,
            }, 3,
        ).await?;
        let [row] = rows.as_slice() else {
            return Err(ResourceAuthorityError::InvalidInput("administration registration missing or ambiguous"));
        };
        if row.id != *original_resource || row.resource_kind != ResourceKind::AccountAdministration.as_str()
            || row.created_by_principal_id != RecordId::new("principals", owner.principal_id.clone())
            || row.created_in_session_id != RecordId::new("authenticated_sessions", authority.session_id().to_owned())
            || row.external_resource_id != expected_external || row.owner_account_id != account
            || row.access_space_id != space || row.parent_resource_id != expected_parent
            || row.policy_version != expected_policy || row.lifecycle_state != "active"
            || row.classification != expected_classification
            || row.account_administration_lifetime != expected_lifetime
        { return Err(ResourceAuthorityError::InvalidInput("immutable administration registration differs")); }
        row.account_administration_lifetime.closed_choice()?;
        authority.require_current_in_transaction(transaction).await?;
        Ok(RegisteredResource {
            resource_id: record_key(row.id.clone())?, resource_kind: ResourceKind::AccountAdministration,
            external_resource_id: row.external_resource_id.clone(),
        })
    }

}

#[derive(Clone, PartialEq, Eq, SurrealValue)]
struct MemberEnrollmentRow {
    id: RecordId,
    operation_id: String,
    descriptor_hex: String,
    descriptor_sha256: String,
    account_id: RecordId,
    principal_id: RecordId,
    access_space_id: RecordId,
    account_name: String,
    token_digest: String,
    purpose: String,
    issued_at: Datetime,
    expires_at: Datetime,
    status: String,
    password_verifier: Option<String>,
    redemption_attempt_id: Option<String>,
    consumed_at: Option<Datetime>,
}
#[derive(Clone, PartialEq, Eq, SurrealValue)]
struct MemberTargetRow {
    account_id: RecordId,
    account_key: String,
    account_role: String,
    account_status: String,
    password_verifier: Option<String>,
    principal_id: RecordId,
    principal_account: RecordId,
    principal_kind: String,
    actor_kind: String,
    actor_id: String,
    profile: String,
    capabilities: Vec<String>,
    principal_status: String,
    access_space_id: RecordId,
    space_account: RecordId,
    space_status: String,
}
#[derive(Clone, SurrealValue)]
struct MemberEnrollmentBindings {
    enrollment: RecordId,
    operation: String,
    descriptor_hex: String,
    descriptor_sha256: String,
    account: RecordId,
    principal: RecordId,
    space: RecordId,
    name: String,
    actor: String,
    profile: String,
    capabilities: Vec<String>,
    token_digest: String,
    issued_at: Datetime,
    expires_at: Datetime,
}
impl MemberEnrollmentBindings {
    fn original(plan: &enrollment::EnrollmentPlan) -> Result<Self, ResourceAuthorityError> {
        let descriptor = plan.descriptor();
        for id in [descriptor.staged().account_id(), descriptor.staged().principal_id(),
            descriptor.staged().access_space_id()] { validate_uuid(id)?; }
        let name = descriptor.account_name();
        if name != name.trim() || name.is_empty() || name.len() > 128 {
            return Err(ResourceAuthorityError::InvalidInput("invalid original enrollment name"));
        }
        handshake_member_enrollment::invitation::validate_expiry(descriptor)
            .map_err(|_| ResourceAuthorityError::InvalidInput("invalid original enrollment deadline"))?;
        let encoded = enrollment::encode_enrollment(descriptor)
            .map_err(|_| ResourceAuthorityError::InvalidInput("invalid original enrollment descriptor"))?;
        let datetime = |value: enrollment::Timestamp| {
            chrono::DateTime::<Utc>::from_timestamp(value.unix_seconds(), value.nanoseconds())
                .map(Datetime::from)
                .ok_or(ResourceAuthorityError::InvalidInput("enrollment timestamp outside host range"))
        };
        let operation = descriptor.operation_id().to_string();
        Ok(Self {
            enrollment: RecordId::new("member_enrollments", operation.clone()), operation,
            descriptor_hex: hex::encode(&encoded), descriptor_sha256: sha256_hex(&encoded),
            account: RecordId::new("local_accounts", descriptor.staged().account_id()),
            principal: RecordId::new("principals", descriptor.staged().principal_id()),
            space: RecordId::new("access_spaces", descriptor.staged().access_space_id()),
            name: name.to_owned(), actor: descriptor.staged().principal_id().to_owned(),
            profile: descriptor.profile_id().to_owned(),
            capabilities: descriptor.capabilities().iter().map(|value| (*value).to_owned()).collect(),
            token_digest: hex::encode(descriptor.token_digest()),
            issued_at: datetime(descriptor.issued_at())?, expires_at: datetime(descriptor.expires_at())?,
        })
    }
    fn require_original(&self, row: &MemberEnrollmentRow) -> Result<(), ResourceAuthorityError> {
        if row.id != self.enrollment || row.operation_id != self.operation
            || row.descriptor_hex != self.descriptor_hex || row.descriptor_sha256 != self.descriptor_sha256
            || row.account_id != self.account || row.principal_id != self.principal
            || row.access_space_id != self.space || row.account_name != self.name
            || row.token_digest != self.token_digest || row.purpose != "identity_enrollment"
            || row.issued_at != self.issued_at || row.expires_at != self.expires_at
        { return Err(ResourceAuthorityError::InvalidInput("original enrollment differs")); }
        Ok(())
    }
    fn require_target(&self, row: &MemberTargetRow, enabled: bool, verifier: Option<&str>)
        -> Result<(), ResourceAuthorityError>
    {
        if row.account_id != self.account || row.account_key != self.name || row.account_role != "Member"
            || row.principal_id != self.principal || row.principal_account != self.account
            || row.access_space_id != self.space || row.space_account != self.account
            || row.principal_kind != "human_account" || row.actor_kind != "operator"
            || row.actor_id != self.actor || row.profile != self.profile || row.capabilities != self.capabilities
            || row.account_status != if enabled { "enabled" } else { "disabled" }
            || row.principal_status != if enabled { "enabled" } else { "disabled" }
            || row.space_status != if enabled { "active" } else { "disabled" }
            || row.password_verifier.as_deref() != verifier
        { return Err(ResourceAuthorityError::InvalidInput("enrollment target readback differs")); }
        Ok(())
    }
}


enum MemberRedemptionObservation { ConsumedNow, SameAttemptAlreadyConsumed }

enum MemberIssueObservation { Created, ExistingOriginal(MemberEnrollmentRow) }
const MEMBER_TARGET_READBACK: &str = "SELECT id AS account_id, account_key, account_role, status AS account_status, password_verifier, $principal.id AS principal_id, $principal.account_id AS principal_account, $principal.principal_kind AS principal_kind, $principal.actor_kind AS actor_kind, $principal.actor_id AS actor_id, $principal.capability_profile_id AS profile, $principal.delegated_capabilities AS capabilities, $principal.status AS principal_status, $space.id AS access_space_id, $space.account_id AS space_account, $space.status AS space_status FROM $account;";

#[derive(SurrealValue)]
struct MemberEnrollmentOrigins {
    account: RecordId,
    principal: RecordId,
    space: RecordId,
}



// RAM ONLY: resource_authority.rs, alongside the genuine captures/closed lifetime code.
// Issue-only permit. No public fields, Clone, caller boolean, descriptor constructor,
// new SDK transaction, alternate scope, root client, or recipient-auth assertion.
#[derive(Clone, PartialEq, Eq, SurrealValue)]
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
struct MemberAdministrationParent {
    administration: RecordId,
    owner: RecordId,
    space: RecordId,
    resource_policy: i64,
    parent: RecordId,
    parent_external: String,
    parent_kind: String,
    parent_state: String,
    parent_owner: RecordId,
    parent_space: RecordId,
    parent_policy: i64,
}
#[derive(SurrealValue)]
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
struct MemberAdministrationParentBindings {
    resource: RecordId,
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
struct MemberAdministrationObservation {
    enroll: RegistryPublicationAuthority,
    assign: RegistryPublicationAuthority,
    lifetime: CapturedAdministrationLifetime,
    parent: MemberAdministrationParent,
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
struct MemberEnrollmentTransactionPermit<'borrow, 'engine> {
    storage: &'borrow SurrealStorage,
    transaction: &'borrow super::SurrealTransactionContext<'engine>,
    captured: &'borrow CapturedMemberEnrollment,
    registry: &'borrow crate::capabilities::CapabilityRegistry,
    reauthentication: &'borrow super::local_accounts::CurrentActorPasswordReauthentication,
    parent: MemberAdministrationParent,
    issuer_credential_revision: i64,
}

#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl<'borrow, 'engine> MemberEnrollmentTransactionPermit<'borrow, 'engine> {
    // Private constructor consumes genuine capture REFERENCES and a real borrowed
    // controller context; it cannot mint authority from an EnrollmentDescriptor.
    async fn capture(
        storage: &'borrow SurrealStorage,
        transaction: &'borrow super::SurrealTransactionContext<'engine>,
        captured: &'borrow CapturedMemberEnrollment,
        registry: &'borrow crate::capabilities::CapabilityRegistry,
        reauthentication: &'borrow super::local_accounts::CurrentActorPasswordReauthentication,
    ) -> Result<Self, ResourceAuthorityError> {
        transaction.require_authenticated_scope(storage, &captured.enroll.original_scope)?;
        Self::require_capture_pair(captured, registry)?;
        let observed = Self::observe(storage, transaction, captured).await?;
        require_precommit_administration_pair(
            &captured.enroll, &captured.assign, &captured.lifetime,
            &observed.enroll, &observed.assign, &observed.lifetime,
        )?;
        observed.lifetime.require_initial_descriptor(captured.plan.descriptor())?;
        handshake_member_enrollment::invitation::validate_expiry(captured.plan.descriptor())
            .map_err(|_| ResourceAuthorityError::InvalidInput("invalid original enrollment deadline"))?;
        let operation = captured.plan.descriptor().operation_id();
        for gate in [&captured.enroll, &captured.assign] {
            reauthentication.require_binding(
                storage, operation, operation, &gate.session,
                &gate.credentials.session_token, &gate.credentials.channel_binding_hash,
                &gate.facts.policy_versions, &gate.facts.revocation_epochs,
            )?;
        }
        let issuer_credential_revision = reauthentication.fenced_credential_revision_in_transaction(
            storage, transaction, &captured.enroll.original_scope, operation, operation,
        ).await?;
        Ok(Self { storage, transaction, captured, registry, reauthentication,
            parent: observed.parent, issuer_credential_revision })
    }

    fn require_same_transaction(
        &self, transaction: &super::SurrealTransactionContext<'_>,
    ) -> Result<(), ResourceAuthorityError> {
        if !std::ptr::eq(self.transaction, transaction) {
            return Err(ResourceAuthorityError::InvalidInput("enrollment transaction differs"));
        }
        transaction.require_authenticated_scope(self.storage, &self.captured.enroll.original_scope)?;
        Ok(())
    }

    fn require_capture_pair(
        captured: &CapturedMemberEnrollment,
        registry: &crate::capabilities::CapabilityRegistry,
    ) -> Result<(), ResourceAuthorityError> {
        let [enroll, assign] = captured.plan.intents();
        for (intent, gate, expected) in [
            (enroll, &captured.enroll, enrollment::CommandKind::Enroll),
            (assign, &captured.assign, enrollment::CommandKind::AssignFixedMemberProfile),
        ] {
            let action = match expected.action() {
                enrollment::AdminAction::Create => ResourceAction::Create,
                enrollment::AdminAction::Update => ResourceAction::Update,
            };
            let scope = &gate.original_scope;
            let facts = &gate.facts;
            if intent.kind() != expected || intent.authorizer() != &gate.exact
                || intent.source() != &enrollment_source(facts)?
                || intent.operation_id() != captured.plan.descriptor().operation_id()
                || intent.enrollment_operation_id() != captured.plan.descriptor().operation_id()
                || intent.staged() != captured.plan.descriptor().staged()
                || intent.administrator_resource() != captured.plan.descriptor().administrator_resource()
                || scope.action != action || scope.capability_id != expected.capability_id()
                || scope.grant_id.as_deref() != Some(gate.decision.grant_id.as_str())
                || scope.resource_id != gate.decision.resource_id || scope.session_id != gate.decision.session_id
                || scope.workspace_id.as_deref() != Some(gate.exact.workspace_id())
                || scope.session_token != gate.credentials.session_token
                || scope.channel_binding_hash.as_deref() != Some(gate.credentials.channel_binding_hash.as_str())
                || facts.resource_kind != ResourceKind::AccountAdministration.as_str()
                || facts.resource_id != RecordId::new("protected_resources", gate.decision.resource_id.clone())
                || facts.grant_id != RecordId::new("resource_grants", gate.decision.grant_id.clone())
                || facts.account_id != RecordId::new("local_accounts", gate.exact.owner_account_id())
                || facts.principal_id != RecordId::new("principals", gate.exact.actor_principal_id())
                || facts.session_id != RecordId::new("authenticated_sessions", gate.exact.authenticated_session_id())
                || facts.access_space_id != RecordId::new("access_spaces", gate.exact.access_space_id())
                || gate.session.identity.account_id != gate.exact.owner_account_id()
                || gate.session.identity.principal_id != gate.exact.actor_principal_id()
                || gate.session.identity.access_space_id != gate.exact.access_space_id()
                || gate.session.session_id != gate.exact.authenticated_session_id()
                || facts.delegation_chain != gate.session.delegation_chain
                || facts.capability_profile_id != gate.decision.capability_profile_id
                || !registry.profile_can(&facts.capability_profile_id, expected.capability_id()).unwrap_or(false)
            { return Err(ResourceAuthorityError::InvalidInput("enrollment captured action differs")); }
            captured.lifetime.require_matches_source(facts)?;
        }
        let left = &captured.enroll.original_scope;
        let right = &captured.assign.original_scope;
        if captured.enroll.exact != captured.assign.exact
            || left.session_token != right.session_token || left.channel_binding_hash != right.channel_binding_hash
            || left.session_id != right.session_id || left.resource_id != right.resource_id
            || left.workspace_id != right.workspace_id
            || captured.enroll.facts.delegation_chain != captured.assign.facts.delegation_chain
        { return Err(ResourceAuthorityError::InvalidInput("enrollment action identities differ")); }
        // Grant IDs may coincide only when that same real grant independently
        // satisfies BOTH actual action/capability queries; never infer one from the other.
        Ok(())
    }

    async fn observe(
        storage: &SurrealStorage, transaction: &super::SurrealTransactionContext<'_>,
        captured: &CapturedMemberEnrollment,
    ) -> Result<MemberAdministrationObservation, ResourceAuthorityError> {
        transaction.require_authenticated_scope(storage, &captured.enroll.original_scope)?;
        let mut current = Vec::with_capacity(2);
        for (gate, kind) in [
            (&captured.enroll, enrollment::CommandKind::Enroll),
            (&captured.assign, enrollment::CommandKind::AssignFixedMemberProfile),
        ] {
            let action = match kind.action() {
                enrollment::AdminAction::Create => ResourceAction::Create,
                enrollment::AdminAction::Update => ResourceAction::Update,
            };
            // Explicit original grant+action+capability, unchanged actual record user.
            // No nested task-local scope or claim that changing bindings admits a write.
            let rows: Vec<RegistryPublicationAuthority> = transaction.query_values_at(
                REGISTRY_PUBLICATION_AUTHORITY,
                RegistryPublicationBindings {
                    resource: gate.facts.resource_id.clone(), original_grant: gate.facts.grant_id.clone(),
                    kind: ResourceKind::AccountAdministration.as_str().to_owned(),
                    action: action.as_str().to_owned(), capability: kind.capability_id().to_owned(),
                }, 0,
            ).await?;
            let [row] = rows.as_slice() else { return Err(ResourceAuthorityError::InvalidInput("enrollment current action denied")); };
            if row != &gate.facts {
                return Err(ResourceAuthorityError::InvalidInput("enrollment current action changed"));
            }
            current.push(row.clone());
        }
        let assign = current.pop().ok_or(ResourceAuthorityError::InvalidInput("assignment observation missing"))?;
        let enroll = current.pop().ok_or(ResourceAuthorityError::InvalidInput("enrollment observation missing"))?;
        let lifetime = |facts: &RegistryPublicationAuthority| -> Result<CapturedAdministrationLifetime, ResourceAuthorityError> {
            let choice = facts.administration_lifetime.as_ref()
                .ok_or(ResourceAuthorityError::InvalidInput("registered administration lifetime missing"))?.closed_choice()?;
            let policy_version = *facts.policy_versions.get(1)
                .ok_or(ResourceAuthorityError::InvalidInput("administration resource policy missing"))?;
            Ok(CapturedAdministrationLifetime { administration_resource_id: facts.resource_id.clone(), policy_version, choice })
        };
        let current_lifetime = lifetime(&enroll)?;
        if current_lifetime != lifetime(&assign)? || current_lifetime != captured.lifetime {
            return Err(ResourceAuthorityError::InvalidInput("administration lifetime changed"));
        }
        let parents: Vec<MemberAdministrationParent> = transaction.query_values_at(
            "SELECT id AS administration, owner_account_id AS owner, access_space_id AS space, policy_version AS resource_policy, parent_resource_id AS parent, parent_resource_id.external_resource_id AS parent_external, parent_resource_id.resource_kind AS parent_kind, parent_resource_id.lifecycle_state AS parent_state, parent_resource_id.owner_account_id AS parent_owner, parent_resource_id.access_space_id AS parent_space, parent_resource_id.policy_version AS parent_policy FROM protected_resources WHERE id = $resource AND resource_kind = 'account_administration' AND lifecycle_state = 'active' LIMIT 2;",
            MemberAdministrationParentBindings { resource: enroll.resource_id.clone() }, 0,
        ).await?;
        let [parent] = parents.as_slice() else { return Err(ResourceAuthorityError::InvalidInput("administration current parent missing")); };
        if parent.administration != enroll.resource_id || parent.owner != enroll.account_id || parent.space != enroll.access_space_id
            || parent.resource_policy != current_lifetime.policy_version || parent.parent_kind != "workspace"
            || parent.parent_state != "active" || parent.parent_external != captured.enroll.exact.workspace_id()
            || parent.parent_owner != parent.owner || parent.parent_space != parent.space
            || enroll.policy_versions.first().is_none_or(|policy| parent.parent_policy > *policy)
            || assign.policy_versions.first().is_none_or(|policy| parent.parent_policy > *policy)
        { return Err(ResourceAuthorityError::InvalidInput("administration current parent differs")); }
        Ok(MemberAdministrationObservation { enroll, assign, lifetime: current_lifetime, parent: parent.clone() })
    }

    async fn recheck(
        &self, transaction: &super::SurrealTransactionContext<'_>,
    ) -> Result<MemberAdministrationObservation, ResourceAuthorityError> {
        self.require_same_transaction(transaction)?;
        Self::require_capture_pair(self.captured, self.registry)?;
        let observed = Self::observe(self.storage, transaction, self.captured).await?;
        if observed.parent != self.parent {
            return Err(ResourceAuthorityError::InvalidInput("administration parent changed"));
        }
        require_precommit_administration_pair(
            &self.captured.enroll, &self.captured.assign, &self.captured.lifetime,
            &observed.enroll, &observed.assign, &observed.lifetime,
        )?;
        observed.lifetime.require_initial_descriptor(self.captured.plan.descriptor())?;
        let operation = self.captured.plan.descriptor().operation_id();
        for (gate, facts) in [(&self.captured.enroll, &observed.enroll), (&self.captured.assign, &observed.assign)] {
            self.reauthentication.require_binding(
                self.storage, operation, operation, &gate.session,
                &gate.credentials.session_token, &gate.credentials.channel_binding_hash,
                &facts.policy_versions, &facts.revocation_epochs,
            )?;
        }
        let issuer_credential_revision = self.reauthentication.fenced_credential_revision_in_transaction(
            self.storage, transaction, &self.captured.enroll.original_scope, operation, operation,
        ).await?;
        if issuer_credential_revision != self.issuer_credential_revision {
            return Err(ResourceAuthorityError::InvalidInput("enrollment credential revision changed"));
        }
        Ok(observed)
    }
    async fn require_original_lookup(
        &self, transaction: &super::SurrealTransactionContext<'_>, descriptor: &enrollment::EnrollmentDescriptor,
    ) -> Result<(), ResourceAuthorityError> {
        if descriptor != self.captured.plan.descriptor() {
            return Err(ResourceAuthorityError::InvalidInput("enrollment lookup original differs"));
        }
        self.recheck(transaction).await.map(|_| ())
    }
    async fn require_first_issue(
        &self, transaction: &super::SurrealTransactionContext<'_>, captured: &CapturedMemberEnrollment,
    ) -> Result<(), ResourceAuthorityError> {
        if !std::ptr::eq(captured, self.captured) {
            return Err(ResourceAuthorityError::InvalidInput("enrollment capture custody differs"));
        }
        let observed = self.recheck(transaction).await?;
        self.captured.require_precommit(&observed.enroll, &observed.assign, &observed.lifetime)
    }
}

// Actual borrowed callback controller. It does not open/commit a transaction or
// claim Unknown settlement. The existing owner must invoke it inside its actual
// authenticated worker and retain the original outcome/captures across Unknown.
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl SurrealStorage {
    async fn stage_member_enrollment_callback(
        &self, transaction: &super::SurrealTransactionContext<'_>,
        captured: &CapturedMemberEnrollment, registry: &crate::capabilities::CapabilityRegistry,
        reauthentication: &super::local_accounts::CurrentActorPasswordReauthentication,
    ) -> Result<MemberIssueObservation, ResourceAuthorityError> {
        let permit = MemberEnrollmentTransactionPermit::capture(self, transaction, captured, registry, reauthentication).await?;
        let observed = self.stage_member_enrollment_in_transaction(transaction, &permit, captured).await?;
        // Precommit boundary for THIS exact transaction and original captured pair.
        // The owning outer controller must abort on error and finalize without another effect.
        match &observed {
            MemberIssueObservation::Created => permit.require_first_issue(transaction, captured).await?,
            MemberIssueObservation::ExistingOriginal(_) => permit.require_original_lookup(
                transaction, captured.plan.descriptor(),
            ).await?,
        }
        Ok(observed)
    }
}


// RAM ONLY resource_authority.rs additions. NEW immutable original-row projection.
// Values come from the genuine same-transaction permit/captures, not caller booleans.
#[derive(Clone, PartialEq, Eq, SurrealValue)]
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
struct MemberEnrollmentIssueProjection {
    issuer_account: RecordId,
    issuer_principal: RecordId,
    issuer_session: RecordId,
    issuer_space: RecordId,
    issuer_credential_revision: i64,
    administration_resource: RecordId,
    workspace_resource: RecordId,
    workspace_external: String,
    workspace_policy: i64,
    enroll_source: RegistryPublicationAuthority,
    assignment_source: RegistryPublicationAuthority,
    lifetime: AdministrationLifetimeRow,
    policy_expires_at: Option<Datetime>,
}
#[derive(SurrealValue)]
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
struct MemberEnrollmentIssueBindings {
    original: MemberEnrollmentBindings,
    issue: MemberEnrollmentIssueProjection,
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl MemberEnrollmentTransactionPermit<'_, '_> {
    fn issue_projection(&self) -> Result<MemberEnrollmentIssueProjection, ResourceAuthorityError> {
        self.require_same_transaction(self.transaction)?;
        let captured = self.captured;
        let operation = captured.plan.descriptor().operation_id();
        for gate in [&captured.enroll, &captured.assign] {
            let revision = self.reauthentication.bound_credential_revision(
                self.storage, operation, operation, &gate.session,
                &gate.credentials.session_token, &gate.credentials.channel_binding_hash,
                &gate.facts.policy_versions, &gate.facts.revocation_epochs,
            )?;
            if revision != self.issuer_credential_revision {
                return Err(ResourceAuthorityError::InvalidInput("enrollment projection credential revision differs"));
            }
        }
        let policy_expires_at = captured.plan.descriptor().policy_expires_at().map(|value| {
            chrono::DateTime::<Utc>::from_timestamp(value.unix_seconds(), value.nanoseconds())
                .map(Datetime::from).ok_or(ResourceAuthorityError::InvalidInput("invalid original policy bound"))
        }).transpose()?;
        Ok(MemberEnrollmentIssueProjection {
            issuer_account: captured.enroll.facts.account_id.clone(),
            issuer_principal: captured.enroll.facts.principal_id.clone(),
            issuer_session: captured.enroll.facts.session_id.clone(),
            issuer_space: captured.enroll.facts.access_space_id.clone(),
            issuer_credential_revision: self.issuer_credential_revision,
            administration_resource: captured.enroll.facts.resource_id.clone(),
            workspace_resource: self.parent.parent.clone(),
            workspace_external: self.parent.parent_external.clone(), workspace_policy: self.parent.parent_policy,
            enroll_source: captured.enroll.facts.clone(), assignment_source: captured.assign.facts.clone(),
            lifetime: AdministrationLifetimeRow::from_explicit(captured.lifetime.choice)?, policy_expires_at,
        })
    }
}


// RAM ONLY Account implementation in existing resource_authority.rs.
// Exact native ced000 ABI types are in scope; no guessed import/module path.
// NEW Account request codec; no descriptor/digest/secret placeholder before entropy.
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
const ORIGINAL_ISSUE_REQUEST_DOMAIN: &[u8] = b"hsk.member_enrollment.original_issue_request@1\0";

#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
fn original_account_fault(message: &'static str) -> AccountError {
    Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, message))
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
fn issue_request_string(out: &mut Vec<u8>, value: &str) -> Result<(), AccountError> {
    if value.is_empty() || value.len() > enrollment::MAX_STRING_BYTES {
        return Err(original_account_fault("invalid original request string"));
    }
    let length = u32::try_from(value.len()).map_err(|error| Box::new(error) as AccountError)?;
    out.extend_from_slice(&length.to_be_bytes()); out.extend_from_slice(value.as_bytes()); Ok(())
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
fn issue_request_time(out: &mut Vec<u8>, value: enrollment::Timestamp) {
    out.extend_from_slice(&value.unix_seconds().to_be_bytes()); out.extend_from_slice(&value.nanoseconds().to_be_bytes());
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
fn issue_request_optional_time(out: &mut Vec<u8>, value: Option<enrollment::Timestamp>) {
    match value { Some(value) => { out.push(1); issue_request_time(out, value); }, None => out.push(0) }
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
fn issue_request_source(out: &mut Vec<u8>, source: &enrollment::SourceRef) -> Result<(), AccountError> {
    issue_request_string(out, source.resource_id())?; issue_request_string(out, source.grant_id())?;
    out.extend_from_slice(&source.grant_version().to_be_bytes());
    for vector in [source.policy_versions(), source.revocation_epochs()] {
        if vector.len() > enrollment::MAX_VECTOR_ITEMS { return Err(original_account_fault("original source vector limit")); }
        out.extend_from_slice(&u32::try_from(vector.len()).map_err(|error| Box::new(error) as AccountError)?.to_be_bytes());
        for value in vector { out.extend_from_slice(&value.to_be_bytes()); }
    }
    issue_request_optional_time(out, source.grant_expires_at()); Ok(())
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
fn encode_original_issue_request(value: &OriginalIssueIntent) -> Result<Vec<u8>, AccountError> {
    if value.operation_id.is_nil() { return Err(original_account_fault("nil original operation")); }
    let mut out = ORIGINAL_ISSUE_REQUEST_DOMAIN.to_vec();
    out.extend_from_slice(value.operation_id.as_bytes());
    for text in [
        value.original_enroller.owner_account_id(), value.original_enroller.actor_principal_id(),
        value.original_enroller.authenticated_session_id(), value.original_enroller.access_space_id(),
        value.original_enroller.workspace_id(), value.staged.account_id(), value.staged.principal_id(),
        value.staged.access_space_id(), value.account_name.as_str(), value.administrator_resource.as_str(),
    ] { issue_request_string(&mut out, text)?; }
    issue_request_source(&mut out, &value.enroll_source)?; issue_request_source(&mut out, &value.assignment_source)?;
    for time in [value.issued_at, value.expires_at, value.session_expires_at] { issue_request_time(&mut out, time); }
    issue_request_optional_time(&mut out, value.policy_expires_at);
    for text in [&value.purpose, &value.vault_namespace, &value.vault_reference] { issue_request_string(&mut out, text)?; }
    if out.len() > enrollment::MAX_ENCODED_BYTES { return Err(original_account_fault("original request limit")); }
    Ok(out)
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
struct OriginalIssueRequestReader<'a> { bytes: &'a [u8], at: usize }
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl OriginalIssueRequestReader<'_> {
    fn bytes(&mut self, count: usize) -> Result<&[u8], AccountError> {
        let end = self.at.checked_add(count).ok_or_else(|| original_account_fault("original request length overflow"))?;
        let value = self.bytes.get(self.at..end).ok_or_else(|| original_account_fault("truncated original request"))?;
        self.at = end; Ok(value)
    }
    fn fixed<const N: usize>(&mut self) -> Result<[u8; N], AccountError> {
        self.bytes(N)?.try_into().map_err(|error| Box::new(error) as AccountError)
    }
    fn string(&mut self) -> Result<String, AccountError> {
        let count = u32::from_be_bytes(self.fixed()?) as usize;
        if count == 0 || count > enrollment::MAX_STRING_BYTES { return Err(original_account_fault("invalid original string length")); }
        String::from_utf8(self.bytes(count)?.to_vec()).map_err(|error| Box::new(error) as AccountError)
    }
    fn time(&mut self) -> Result<enrollment::Timestamp, AccountError> {
        enrollment::Timestamp::new(i64::from_be_bytes(self.fixed()?), u32::from_be_bytes(self.fixed()?))
            .map_err(|error| Box::new(error) as AccountError)
    }
    fn optional_time(&mut self) -> Result<Option<enrollment::Timestamp>, AccountError> {
        match self.fixed::<1>()?[0] {
            0 => Ok(None), 1 => self.time().map(Some), _ => Err(original_account_fault("invalid original optional time")),
        }
    }
    fn vector(&mut self) -> Result<Vec<i64>, AccountError> {
        let count = u32::from_be_bytes(self.fixed()?) as usize;
        if count > enrollment::MAX_VECTOR_ITEMS { return Err(original_account_fault("original vector limit")); }
        (0..count).map(|_| self.fixed().map(i64::from_be_bytes)).collect()
    }
    fn source(&mut self) -> Result<enrollment::SourceRef, AccountError> {
        enrollment::SourceRef::new(self.string()?, self.string()?, i64::from_be_bytes(self.fixed()?),
            self.vector()?, self.vector()?, self.optional_time()?).map_err(|error| Box::new(error) as AccountError)
    }
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
fn decode_original_issue_request(bytes: &[u8]) -> Result<OriginalIssueIntent, AccountError> {
    if bytes.len() > enrollment::MAX_ENCODED_BYTES { return Err(original_account_fault("original request limit")); }
    let mut reader = OriginalIssueRequestReader { bytes, at: 0 };
    if reader.bytes(ORIGINAL_ISSUE_REQUEST_DOMAIN.len())? != ORIGINAL_ISSUE_REQUEST_DOMAIN {
        return Err(original_account_fault("original request domain differs"));
    }
    let operation_id = Uuid::from_bytes(reader.fixed()?);
    let original_enroller = enrollment::IdentityScope::new(reader.string()?, reader.string()?, reader.string()?,
        reader.string()?, reader.string()?).map_err(|error| Box::new(error) as AccountError)?;
    let staged = enrollment::StagedIdentity::new(reader.string()?, reader.string()?, reader.string()?)
        .map_err(|error| Box::new(error) as AccountError)?;
    let value = OriginalIssueIntent {
        operation_id, original_enroller, staged, account_name: reader.string()?, administrator_resource: reader.string()?,
        enroll_source: reader.source()?, assignment_source: reader.source()?,
        issued_at: reader.time()?, expires_at: reader.time()?, session_expires_at: reader.time()?,
        policy_expires_at: reader.optional_time()?, purpose: reader.string()?, vault_namespace: reader.string()?,
        vault_reference: reader.string()?,
    };
    if reader.at != bytes.len() || encode_original_issue_request(&value)? != bytes {
        return Err(original_account_fault("noncanonical original request"));
    }
    Ok(value)
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
fn original_account_revision(value: i64) -> Result<AccountRevision, AccountError> {
    if value <= 0 { return Err(original_account_fault("invalid original row revision")); }
    Ok(AccountRevision(value.to_be_bytes().to_vec()))
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
fn original_revision_number(value: &AccountRevision) -> Result<i64, AccountError> {
    let bytes: [u8; 8] = value.0.as_slice().try_into().map_err(|error| Box::new(error) as AccountError)?;
    let number = i64::from_be_bytes(bytes);
    if number <= 0 { return Err(original_account_fault("invalid supplied original revision")); }
    Ok(number)
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
fn original_descriptor_matches(intent: &OriginalIssueIntent, value: &enrollment::EnrollmentDescriptor) -> bool {
    value.operation_id() == intent.operation_id && value.original_enroller() == &intent.original_enroller
        && value.staged() == &intent.staged && value.account_name() == intent.account_name
        && value.administrator_resource() == intent.administrator_resource
        && value.enroll_source() == &intent.enroll_source && value.assignment_source() == &intent.assignment_source
        && value.issued_at() == intent.issued_at && value.expires_at() == intent.expires_at
        && value.session_expires_at() == intent.session_expires_at && value.policy_expires_at() == intent.policy_expires_at
        && value.invitation_purpose() == intent.purpose
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
fn require_original_issue_deadline(intent: &OriginalIssueIntent) -> Result<(), AccountError> {
    // Pre-digest input cannot call the descriptor API with a fabricated digest.
    // This implements the same inspected Source030 bound over actual immutable inputs.
    let mut limit = intent.issued_at.checked_add_seconds(60).map_err(|error| Box::new(error) as AccountError)?
        .min(intent.session_expires_at);
    for source in [&intent.enroll_source, &intent.assignment_source] {
        if let Some(expiry) = source.grant_expires_at() { limit = limit.min(expiry); }
    }
    if let Some(expiry) = intent.policy_expires_at { limit = limit.min(expiry); }
    let now = enrollment_time(&Datetime::from(Utc::now()))?;
    if intent.issued_at > now || intent.expires_at <= now || intent.expires_at <= intent.issued_at || intent.expires_at > limit {
        return Err(original_account_fault("original issue deadline denied"));
    }
    Ok(())
}

// Genuine pre-digest authority: captures actual two actions, not an invented plan.
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
struct OriginalAccountAuthority {
    storage: SurrealStorage,
    registry: std::sync::Arc<crate::capabilities::CapabilityRegistry>,
    intent: OriginalIssueIntent,
    original_submission: OriginalSubmission,
    enroll: CapturedAdministrationAuthority,
    assign: CapturedAdministrationAuthority,
    lifetime: CapturedAdministrationLifetime,
    parent: MemberAdministrationParent,
    issue: MemberEnrollmentIssueProjection,
    reauthentication: super::local_accounts::CurrentActorPasswordReauthentication,
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
const ORIGINAL_ACCOUNT_PARENT_SQL: &str = "SELECT id AS administration, owner_account_id AS owner, access_space_id AS space, policy_version AS resource_policy, parent_resource_id AS parent, parent_resource_id.external_resource_id AS parent_external, parent_resource_id.resource_kind AS parent_kind, parent_resource_id.lifecycle_state AS parent_state, parent_resource_id.owner_account_id AS parent_owner, parent_resource_id.access_space_id AS parent_space, parent_resource_id.policy_version AS parent_policy FROM protected_resources WHERE id = $resource AND resource_kind = 'account_administration' AND lifecycle_state = 'active' LIMIT 2;";

#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl OriginalAccountAuthority {
    fn require_original_submission_live(&self) -> Result<(), AccountError> {
        if self.original_submission.intent != self.intent
            || tokio::time::Instant::now() >= self.original_submission.original_deadline {
            return Err(original_account_fault("original submission deadline denied"));
        }
        Ok(())
    }
    fn check_original(&self) -> Result<(), AccountError> {
        self.require_original_submission_live()?;
        let intent = &self.intent;
        require_original_issue_deadline(intent)?;
        if intent.purpose != "identity_enrollment" || intent.account_name != intent.account_name.trim()
            || intent.account_name.len() > 128
            || intent.vault_reference != format!("invitation/identity_enrollment/{}", intent.operation_id.hyphenated())
            || self.enroll.exact != intent.original_enroller || self.assign.exact != intent.original_enroller
            || enrollment_source(&self.enroll.facts)? != intent.enroll_source
            || enrollment_source(&self.assign.facts)? != intent.assignment_source
            || enrollment_time(&self.enroll.facts.session_expires_at)? != intent.session_expires_at
            || enrollment_time(&self.assign.facts.session_expires_at)? != intent.session_expires_at
            || self.lifetime.source030_bound() != intent.policy_expires_at
        { return Err(original_account_fault("original authority or immutable input differs")); }
        let left = &self.enroll.original_scope;
        let right = &self.assign.original_scope;
        if left.action != ResourceAction::Create || left.capability_id != enrollment::CommandKind::Enroll.capability_id()
            || right.action != ResourceAction::Update || right.capability_id != enrollment::CommandKind::AssignFixedMemberProfile.capability_id()
            || left.session_token != right.session_token || left.channel_binding_hash != right.channel_binding_hash
            || left.session_id != right.session_id || left.resource_id != right.resource_id || left.workspace_id != right.workspace_id
        { return Err(original_account_fault("original action scope pair differs")); }
        for (gate, kind) in [
            (&self.enroll, enrollment::CommandKind::Enroll),
            (&self.assign, enrollment::CommandKind::AssignFixedMemberProfile),
        ] {
            self.lifetime.require_matches_source(&gate.facts)?;
            if !self.registry.profile_can(&gate.facts.capability_profile_id, kind.capability_id()).unwrap_or(false) {
                return Err(original_account_fault("current original profile denied"));
            }
            let revision = self.reauthentication.bound_credential_revision(
                &self.storage, intent.operation_id, intent.operation_id, &gate.session,
                &gate.credentials.session_token, &gate.credentials.channel_binding_hash,
                &gate.facts.policy_versions, &gate.facts.revocation_epochs,
            )?;
            if revision != self.issue.issuer_credential_revision {
                return Err(original_account_fault("original projection credential revision differs"));
            }
        }
        Ok(())
    }
    fn check_parent(&self, parent: &MemberAdministrationParent) -> Result<(), AccountError> {
        if parent != &self.parent || parent.administration != self.enroll.facts.resource_id
            || parent.owner != self.enroll.facts.account_id || parent.space != self.enroll.facts.access_space_id
            || parent.parent_owner != parent.owner || parent.parent_space != parent.space
            || parent.parent_kind != "workspace" || parent.parent_state != "active"
            || parent.parent_external != self.intent.original_enroller.workspace_id()
            || parent.resource_policy != self.lifetime.policy_version
            || self.enroll.facts.policy_versions.first().is_none_or(|value| parent.parent_policy > *value)
            || self.assign.facts.policy_versions.first().is_none_or(|value| parent.parent_policy > *value)
        { return Err(original_account_fault("original current parent changed")); }
        Ok(())
    }
    async fn fence(&self, tx: &super::SurrealTransactionContext<'_>) -> Result<(), AccountError> {
        self.check_original()?;
        tx.require_authenticated_scope(&self.storage, &self.enroll.original_scope)?;
        let mut current = Vec::with_capacity(2);
        for (gate, kind) in [
            (&self.enroll, enrollment::CommandKind::Enroll),
            (&self.assign, enrollment::CommandKind::AssignFixedMemberProfile),
        ] {
            let action = match kind.action() {
                enrollment::AdminAction::Create => ResourceAction::Create,
                enrollment::AdminAction::Update => ResourceAction::Update,
            };
            self.require_original_submission_live()?;
            let rows: Vec<RegistryPublicationAuthority> = tx.query_values_at(
                REGISTRY_PUBLICATION_AUTHORITY,
                RegistryPublicationBindings {
                    resource: gate.facts.resource_id.clone(), original_grant: gate.facts.grant_id.clone(),
                    kind: ResourceKind::AccountAdministration.as_str().to_owned(),
                    action: action.as_str().to_owned(),
                    capability: kind.capability_id().to_owned(),
                }, 0,
            ).await?;
            let [facts] = rows.as_slice() else { return Err(original_account_fault("original current grant unavailable")); };
            if facts != &gate.facts { return Err(original_account_fault("original current grant changed")); }
            current.push(facts.clone());
        }
        let assign = current.pop().ok_or_else(|| original_account_fault("assignment current facts missing"))?;
        let enroll = current.pop().ok_or_else(|| original_account_fault("enrollment current facts missing"))?;
        let current_lifetime = CapturedAdministrationLifetime {
            administration_resource_id: enroll.resource_id.clone(),
            policy_version: *enroll.policy_versions.get(1).ok_or_else(|| original_account_fault("current resource policy missing"))?,
            choice: enroll.administration_lifetime.as_ref()
                .ok_or_else(|| original_account_fault("current registered lifetime missing"))?.closed_choice()?,
        };
        for facts in [&enroll, &assign] {
            current_lifetime.require_matches_source(facts)?;
            let choice = facts.administration_lifetime.as_ref()
                .ok_or_else(|| original_account_fault("original registered lifetime missing"))?.closed_choice()?;
            if choice != current_lifetime.choice { return Err(original_account_fault("original registered lifetime sources differ")); }
        }
        require_precommit_administration_pair(&self.enroll, &self.assign, &self.lifetime, &enroll, &assign, &current_lifetime)?;
        self.require_original_submission_live()?;
        let parents: Vec<MemberAdministrationParent> = tx.query_values_at(
            ORIGINAL_ACCOUNT_PARENT_SQL, MemberAdministrationParentBindings { resource: enroll.resource_id.clone() }, 0,
        ).await?;
        let [parent] = parents.as_slice() else { return Err(original_account_fault("original current parent unavailable")); };
        self.check_parent(parent)?;
        self.require_original_submission_live()?;
        let revision = self.reauthentication.fenced_credential_revision_in_transaction(
            &self.storage, tx, &self.enroll.original_scope, self.intent.operation_id, self.intent.operation_id,
        ).await?;
        if revision != self.issue.issuer_credential_revision {
            return Err(original_account_fault("original fenced credential revision differs"));
        }
        Ok(())
    }
    async fn current_read_authority(&self) -> Result<(), AccountError> {
        self.require_original_submission_live()?;
        SurrealStorage::with_operation_deadline(
            self.original_submission.original_deadline.into_std(),
            async {
                self.current_read_authority_scoped().await?;
                self.require_original_submission_live()
            },
        ).await
    }
    async fn current_read_authority_scoped(&self) -> Result<(), AccountError> {
        self.check_original()?;
        self.storage.recheck_current_actor_password(
            &self.reauthentication, &self.enroll.credentials.session_token, &self.enroll.credentials.channel_binding_hash,
        ).await?;
        // This read is independent of the completed mutation. It is not its
        // atomic credential fence; fence() already supplies that same-tx write.
        for (gate, kind) in [
            (&self.enroll, enrollment::CommandKind::Enroll),
            (&self.assign, enrollment::CommandKind::AssignFixedMemberProfile),
        ] {
            let binding = RegistryPublicationBindings {
                resource: gate.facts.resource_id.clone(), original_grant: gate.facts.grant_id.clone(),
                kind: ResourceKind::AccountAdministration.as_str().to_owned(),
                action: match kind.action() { enrollment::AdminAction::Create => "create", enrollment::AdminAction::Update => "update" }.to_owned(),
                capability: kind.capability_id().to_owned(),
            };
            self.require_original_submission_live()?;
            let rows: Vec<RegistryPublicationAuthority> = self.storage.with_record_user_scope(
                self.enroll.original_scope.clone(), self.storage.with_data_operation(move |db| Box::pin(async move {
                    db.query_values_at(REGISTRY_PUBLICATION_AUTHORITY, binding, 0).await
                })),
            ).await?;
            if rows.as_slice() != std::slice::from_ref(&gate.facts) { return Err(original_account_fault("original read grant changed")); }
        }
        let binding = MemberAdministrationParentBindings { resource: self.enroll.facts.resource_id.clone() };
        self.require_original_submission_live()?;
        let parents: Vec<MemberAdministrationParent> = self.storage.with_record_user_scope(
            self.enroll.original_scope.clone(), self.storage.with_data_operation(move |db| Box::pin(async move {
                db.query_values_at(ORIGINAL_ACCOUNT_PARENT_SQL, binding, 0).await
            })),
        ).await?;
        let [parent] = parents.as_slice() else { return Err(original_account_fault("original read parent unavailable")); };
        self.check_parent(parent)
    }
}

#[derive(SurrealValue)]
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
struct OriginalVaultRecord {
    id: RecordId,
    operation_id: String,
    original_request_hex: String,
    original_request_sha256: String,
    vault_namespace: String,
    vault_reference: String,
    vault_issue_revision: i64,
    vault_issue_state: String,
    vault_claim_id: Option<String>,
    descriptor_hex: Option<String>,
    descriptor_sha256: Option<String>,
    token_digest: Option<String>,
    account_id: RecordId,
    principal_id: RecordId,
    access_space_id: RecordId,
    account_name: String,
    purpose: String,
    issued_at: Datetime,
    expires_at: Datetime,
    status: String,
    issue: surrealdb::types::Object,
}
#[derive(Clone, SurrealValue)]
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
struct OriginalVaultLookup { enrollment: RecordId }
#[derive(SurrealValue)]
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
struct OriginalVaultReserve {
    enrollment: RecordId,
    operation: String,
    request_hex: String,
    request_sha256: String,
    namespace: String,
    reference: String,
    account: RecordId,
    principal: RecordId,
    space: RecordId,
    name: String,
    issued_at: Datetime,
    expires_at: Datetime,
    issue: MemberEnrollmentIssueProjection,
}
#[derive(SurrealValue)]
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
struct OriginalVaultCas {
    enrollment: RecordId,
    request_hex: String,
    request_sha256: String,
    namespace: String,
    reference: String,
    expected_revision: i64,
    expected_state: String,
    expected_claim: Option<String>,
    expected_descriptor: Option<String>,
    expected_descriptor_sha256: Option<String>,
    expected_digest: Option<String>,
    next_state: String,
    next_claim: Option<String>,
    next_descriptor: Option<String>,
    next_descriptor_sha256: Option<String>,
    next_digest: Option<String>,
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
const ORIGINAL_VAULT_READ: &str = "SELECT * FROM $enrollment;";
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
const ORIGINAL_VAULT_RESERVE: &str = "CREATE $enrollment SET operation_id = $operation, original_request_hex = $request_hex, original_request_sha256 = $request_sha256, vault_namespace = $namespace, vault_reference = $reference, vault_issue_revision = 1, vault_issue_state = 'reserved', account_id = $account, principal_id = $principal, access_space_id = $space, account_name = $name, issued_at = $issued_at, expires_at = $expires_at, purpose = 'identity_enrollment', status = 'preparing', issue = $issue RETURN AFTER;";
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
const ORIGINAL_VAULT_CAS: &str = "UPDATE $enrollment SET vault_issue_revision = vault_issue_revision + 1, vault_issue_state = $next_state, vault_claim_id = $next_claim, descriptor_hex = $next_descriptor, descriptor_sha256 = $next_descriptor_sha256, token_digest = $next_digest
    WHERE original_request_hex = $request_hex AND original_request_sha256 = $request_sha256 AND vault_namespace = $namespace AND vault_reference = $reference
    AND vault_issue_revision = $expected_revision AND vault_issue_state = $expected_state AND vault_claim_id = $expected_claim
    AND descriptor_hex = $expected_descriptor AND descriptor_sha256 = $expected_descriptor_sha256 AND token_digest = $expected_digest
    AND purpose = 'identity_enrollment' AND status = 'preparing' AND issued_at <= time::now() AND expires_at > time::now() RETURN AFTER;";

#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl OriginalAccountAuthority {
    fn lookup(&self) -> OriginalVaultLookup {
        OriginalVaultLookup { enrollment: RecordId::new("member_enrollments", self.intent.operation_id.to_string()) }
    }
    fn reserve_bindings(&self) -> Result<OriginalVaultReserve, AccountError> {
        let encoded = encode_original_issue_request(&self.intent)?;
        let datetime = |value: enrollment::Timestamp| -> Result<Datetime, AccountError> {
            chrono::DateTime::<Utc>::from_timestamp(value.unix_seconds(), value.nanoseconds())
                .map(Datetime::from).ok_or_else(|| original_account_fault("original timestamp outside host range"))
        };
        Ok(OriginalVaultReserve {
            enrollment: self.lookup().enrollment, operation: self.intent.operation_id.to_string(),
            request_hex: hex::encode(&encoded), request_sha256: sha256_hex(&encoded),
            namespace: self.intent.vault_namespace.clone(), reference: self.intent.vault_reference.clone(),
            account: RecordId::new("local_accounts", self.intent.staged.account_id()),
            principal: RecordId::new("principals", self.intent.staged.principal_id()),
            space: RecordId::new("access_spaces", self.intent.staged.access_space_id()),
            name: self.intent.account_name.clone(), issued_at: datetime(self.intent.issued_at)?,
            expires_at: datetime(self.intent.expires_at)?, issue: self.issue.clone(),
        })
    }
    fn decode_row(&self, row: &OriginalVaultRecord) -> Result<OriginalIssueRow, AccountError> {
        let encoded = hex::decode(&row.original_request_hex)?;
        let intent = decode_original_issue_request(&encoded)?;
        if intent != self.intent || row.original_request_sha256 != sha256_hex(&encoded)
            || row.id != self.lookup().enrollment || row.operation_id != intent.operation_id.to_string()
            || row.vault_namespace != intent.vault_namespace || row.vault_reference != intent.vault_reference
            || row.account_id != RecordId::new("local_accounts", intent.staged.account_id())
            || row.principal_id != RecordId::new("principals", intent.staged.principal_id())
            || row.access_space_id != RecordId::new("access_spaces", intent.staged.access_space_id())
            || row.account_name != intent.account_name
            || row.purpose != intent.purpose || row.purpose != "identity_enrollment"
            || enrollment_time(&row.issued_at)? != intent.issued_at || enrollment_time(&row.expires_at)? != intent.expires_at
        { return Err(original_account_fault("canonical original row or immutable request differs")); }
        let surrealdb::types::Value::Object(expected_issue) = self.issue.clone().into_value() else {
            return Err(original_account_fault("invalid original source projection"));
        };
        if row.issue != expected_issue { return Err(original_account_fault("canonical original source projection differs")); }
        let state = match row.vault_issue_state.as_str() {
            "reserved" if row.vault_claim_id.is_none() && row.descriptor_hex.is_none()
                && row.descriptor_sha256.is_none() && row.token_digest.is_none() && row.status == "preparing" => OriginalIssueState::Reserved,
            "preparing" if row.descriptor_hex.is_none() && row.descriptor_sha256.is_none()
                && row.token_digest.is_none() && row.status == "preparing" => {
                let claim = Uuid::parse_str(row.vault_claim_id.as_deref().ok_or_else(|| original_account_fault("canonical preparation claim missing"))?)?;
                if claim.is_nil() || claim == intent.operation_id { return Err(original_account_fault("canonical preparation claim invalid")); }
                OriginalIssueState::Preparing { claim_id: claim }
            }
            "write_started" | "bound" => {
                let claim = Uuid::parse_str(row.vault_claim_id.as_deref().ok_or_else(|| original_account_fault("canonical original claim missing"))?)?;
                if claim.is_nil() || claim == intent.operation_id { return Err(original_account_fault("canonical original claim invalid")); }
                let bytes = hex::decode(row.descriptor_hex.as_deref().ok_or_else(|| original_account_fault("canonical descriptor missing"))?)?;
                let descriptor = enrollment::decode_enrollment(&bytes)?;
                if enrollment::encode_enrollment(&descriptor)? != bytes || !original_descriptor_matches(&intent, &descriptor)
                    || row.descriptor_sha256.as_deref() != Some(sha256_hex(&bytes).as_str())
                    || row.token_digest.as_deref() != Some(hex::encode(descriptor.token_digest()).as_str())
                { return Err(original_account_fault("canonical bound descriptor differs")); }
                handshake_member_enrollment::invitation::validate_expiry(&descriptor)
                    .map_err(OriginalAccountDeadlineError::boxed)?;
                if row.vault_issue_state == "write_started" && row.status == "preparing" {
                    OriginalIssueState::WriteStarted(descriptor)
                } else if row.vault_issue_state == "bound" && (row.status == "preparing" || row.status == "active") {
                    OriginalIssueState::Bound(descriptor)
                } else { return Err(original_account_fault("canonical original invitation status denied")); }
            }
            _ => return Err(original_account_fault("canonical original state missing or unsupported")),
        };
        Ok(OriginalIssueRow { intent, revision: original_account_revision(row.vault_issue_revision)?, state })
    }
    async fn read_in_transaction(&self, tx: &super::SurrealTransactionContext<'_>)
        -> Result<Option<OriginalVaultRecord>, AccountError>
    {
        let mut rows: Vec<OriginalVaultRecord> = tx.query_values_at(ORIGINAL_VAULT_READ, self.lookup(), 0).await?;
        if rows.len() > 1 { return Err(original_account_fault("ambiguous canonical original")); }
        if let Some(row) = rows.first() { self.decode_row(row)?; }
        Ok(rows.pop())
    }
    async fn canonical_read(&self) -> AccountResult {
        self.current_read_authority().await?;
        let binding = self.lookup();
        let mut rows: Vec<OriginalVaultRecord> = self.storage.with_record_user_scope(
            self.enroll.original_scope.clone(), self.storage.with_data_operation(move |db| Box::pin(async move {
                db.query_values_at(ORIGINAL_VAULT_READ, binding, 0).await
            })),
        ).await?;
        if rows.len() > 1 { return Err(original_account_fault("ambiguous canonical original readback")); }
        rows.pop().map(|row| {
            if row.vault_issue_state == "bound" && row.status != "active" {
                return Err(original_account_fault("canonical Bound original has no active staged targets"));
            }
            self.decode_row(&row)
        }).transpose()
    }
}

#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
enum OriginalAccountStep {
    Reserve(OriginalIssueIntent),
    Claim(OriginalIssueRow, Uuid),
    WriteStarted(OriginalIssueRow, enrollment::EnrollmentDescriptor),
    Bind(OriginalIssueRow),
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
struct OriginalAccountCallGuard {
    uncertain: std::sync::Arc<std::sync::atomic::AtomicBool>,
    busy: std::sync::Arc<std::sync::atomic::AtomicBool>,
    complete: bool,
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl Drop for OriginalAccountCallGuard {
    fn drop(&mut self) {
        if !self.complete { self.uncertain.store(true, std::sync::atomic::Ordering::SeqCst); }
        self.busy.store(false, std::sync::atomic::Ordering::SeqCst);
    }
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
struct AccountOriginalRowController {
    authority: std::sync::Arc<OriginalAccountAuthority>,
    uncertain: std::sync::Arc<std::sync::atomic::AtomicBool>,
    busy: std::sync::Arc<std::sync::atomic::AtomicBool>,
    owned_claim: tokio::sync::Mutex<Option<OriginalIssueRow>>,
    cancellation: std::sync::Arc<super::ContextOperationCancellation>,
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl AccountOriginalRowController {
    fn begin(&self) -> Result<OriginalAccountCallGuard, AccountError> {
        use std::sync::atomic::Ordering::SeqCst;
        if self.uncertain.load(SeqCst) || self.busy.compare_exchange(false, true, SeqCst, SeqCst).is_err() {
            return Err(original_account_fault("original account controller pending"));
        }
        if self.uncertain.load(SeqCst) {
            self.busy.store(false, SeqCst);
            return Err(original_account_fault("original account controller became pending"));
        }
        Ok(OriginalAccountCallGuard { uncertain: self.uncertain.clone(), busy: self.busy.clone(), complete: false })
    }
    async fn transition(&self, step: OriginalAccountStep) -> AccountResult {
        self.authority.require_original_submission_live()?;
        let mut guard = self.begin()?;
        // Each durable phase must finish its own real transaction before the
        // external writer can advance. Never run this inside another callback.
        if super::PROTECTED_TRANSACTION.try_with(|_| ()).is_ok() {
            return Err(original_account_fault("nested original account phase denied"));
        }
        if let OriginalAccountStep::WriteStarted(expected, _) = &step {
            // Denial-only process custody: recovery of Preparing cannot invent a
            // candidate. The actual DB claim/revision/current authority remain mandatory.
            if self.owned_claim.lock().await.as_ref() != Some(expected) {
                return Err(original_account_fault("preparation has no retained positive claim"));
            }
        }
        let claimed_here = match &step { OriginalAccountStep::Claim(_, claim) => Some(*claim), _ => None };
        let binding_original = matches!(&step, OriginalAccountStep::Bind(_));
        let authority = self.authority.clone();
        let worker = authority.clone();
        let storage = authority.storage.clone();
        let scope = authority.enroll.original_scope.clone();
        let operation = authority.intent.operation_id;
        let expected_after: Option<OriginalIssueRow> = SurrealStorage::with_operation_deadline(
            authority.original_submission.original_deadline.into_std(),
            storage.with_record_user_scope(
            scope, storage.with_owned_authenticated_transaction::<Option<OriginalIssueRow>, AccountError, _>(
                operation, self.cancellation.clone(), move |tx| Box::pin(async move {
                    worker.fence(&tx).await?;
                    worker.require_original_submission_live()?;
                    let before = worker.read_in_transaction(&tx).await?;
                    let after = match step {
                        OriginalAccountStep::Reserve(intent) => {
                            if intent != worker.intent { return Err(original_account_fault("reserve full immutable request differs")); }
                            if let Some(row) = before {
                                // Existing original is observed without resetting any phase.
                                Some(worker.decode_row(&row)?)
                            } else {
                                // Explicit first reservation uses CREATE, never UPSERT.
                                // Missing observe() never calls this or implies rollback.
                                worker.require_original_submission_live()?;
                                let rows: Vec<OriginalVaultRecord> = tx.query_values_at(
                                    ORIGINAL_VAULT_RESERVE, worker.reserve_bindings()?, 0,
                                ).await?;
                                let [row] = rows.as_slice() else { return Err(original_account_fault("reservation canonical callback row unavailable")); };
                                let decoded = worker.decode_row(row)?;
                                if decoded.state != OriginalIssueState::Reserved || original_revision_number(&decoded.revision)? != 1 {
                                    return Err(original_account_fault("reservation canonical state differs"));
                                }
                                Some(decoded)
                            }
                        }
                        action => {
                            let Some(before) = before else { return Err(original_account_fault("canonical original missing during transition")); };
                            let actual = worker.decode_row(&before)?;
                            let (expected, next_state, next_claim, descriptor) = match action {
                                OriginalAccountStep::Claim(expected, claim) => {
                                    if claim.is_nil() || claim == worker.intent.operation_id || expected.state != OriginalIssueState::Reserved {
                                        return Err(original_account_fault("invalid original preparation claim"));
                                    }
                                    (expected, "preparing", Some(claim.to_string()), None)
                                }
                                OriginalAccountStep::WriteStarted(expected, descriptor) => {
                                    let OriginalIssueState::Preparing { claim_id } = &expected.state else {
                                        return Err(original_account_fault("write requires same preparation claim"));
                                    };
                                    if before.vault_claim_id.as_deref() != Some(claim_id.to_string().as_str())
                                        || !original_descriptor_matches(&worker.intent, &descriptor) {
                                        return Err(original_account_fault("write descriptor or claim differs"));
                                    }
                                    let claim = before.vault_claim_id.clone();
                                    (expected, "write_started", claim, Some(descriptor))
                                }
                                OriginalAccountStep::Bind(expected) => {
                                    let OriginalIssueState::WriteStarted(descriptor) = &expected.state else {
                                        return Err(original_account_fault("bind requires original WriteStarted"));
                                    };
                                    let descriptor = descriptor.clone();
                                    (expected, "bound", before.vault_claim_id.clone(), Some(descriptor))
                                }
                                OriginalAccountStep::Reserve(_) => unreachable!(),
                            };
                            if expected != actual || expected.intent != worker.intent {
                                return Err(original_account_fault("CAS expected full original or revision differs"));
                            }
                            let revision = original_revision_number(&expected.revision)?;
                            let next_revision = revision.checked_add(1).ok_or_else(|| original_account_fault("original revision exhausted"))?;
                            let (next_descriptor, next_descriptor_sha256, next_digest) = match &descriptor {
                                Some(value) => {
                                    if !original_descriptor_matches(&worker.intent, value) {
                                        return Err(original_account_fault("CAS bound descriptor changes original"));
                                    }
                                    handshake_member_enrollment::invitation::validate_expiry(value)
                                        .map_err(OriginalAccountDeadlineError::boxed)?;
                                    let bytes = enrollment::encode_enrollment(value)?;
                                    (Some(hex::encode(&bytes)), Some(sha256_hex(&bytes)), Some(hex::encode(value.token_digest())))
                                }
                                None => (None, None, None),
                            };
                            let binds = OriginalVaultCas {
                                enrollment: worker.lookup().enrollment, request_hex: before.original_request_hex,
                                request_sha256: before.original_request_sha256, namespace: before.vault_namespace,
                                reference: before.vault_reference, expected_revision: revision,
                                expected_state: before.vault_issue_state, expected_claim: before.vault_claim_id,
                                expected_descriptor: before.descriptor_hex, expected_descriptor_sha256: before.descriptor_sha256,
                                expected_digest: before.token_digest, next_state: next_state.to_owned(),
                                next_claim: next_claim.clone(), next_descriptor, next_descriptor_sha256, next_digest,
                            };
                            worker.require_original_submission_live()?;
                            let rows: Vec<OriginalVaultRecord> = tx.query_values_at(ORIGINAL_VAULT_CAS, binds, 0).await?;
                            let [row] = rows.as_slice() else { return Err(original_account_fault("canonical original CAS did not return exactly one row")); };
                            let decoded = worker.decode_row(row)?;
                            let expected_state = match next_state {
                                "preparing" => OriginalIssueState::Preparing {
                                    claim_id: Uuid::parse_str(next_claim.as_deref().ok_or_else(|| original_account_fault("next claim missing"))?)?,
                                },
                                "write_started" => OriginalIssueState::WriteStarted(descriptor.ok_or_else(|| original_account_fault("next descriptor missing"))?),
                                "bound" => OriginalIssueState::Bound(descriptor.ok_or_else(|| original_account_fault("bound descriptor missing"))?),
                                _ => return Err(original_account_fault("unsupported next original state")),
                            };
                            if decoded.intent != worker.intent || original_revision_number(&decoded.revision)? != next_revision || decoded.state != expected_state {
                                return Err(original_account_fault("canonical CAS callback state differs"));
                            }
                            Some(decoded)
                        }
                    };
                    if binding_original {
                        let Some(bound) = &after else { return Err(original_account_fault("bound original missing before staging")); };
                        let OriginalIssueState::Bound(descriptor) = &bound.state else {
                            return Err(original_account_fault("original bind did not produce Bound"));
                        };
                        let captured = CapturedMemberEnrollment {
                            plan: enrollment::prepare_enrollment_plan(descriptor),
                            enroll: worker.enroll.clone(), assign: worker.assign.clone(),
                            lifetime: worker.lifetime.clone(),
                        };
                        let staged = worker.storage.stage_member_enrollment_callback(
                            &tx, &captured, &worker.registry, &worker.reauthentication,
                        ).await?;
                        if !matches!(staged, MemberIssueObservation::Created) {
                            return Err(original_account_fault("bound original staging did not create its exact targets"));
                        }
                    }
                    worker.fence(&tx).await?;
                    Ok(after)
                }),
            ),
            ),
        ).await?;
        // The existing worker awaited commit. Its ACK alone does not release a
        // preparation or write: a new canonical query independently reconciles it.
        let Some(expected_after) = expected_after else { return Ok(None); };
        authority.require_original_submission_live()?;
        let canonical = SurrealStorage::with_operation_deadline(
            authority.original_submission.original_deadline.into_std(), authority.canonical_read(),
        ).await?;
        authority.require_original_submission_live()?;
        let Some(canonical) = canonical else { return Ok(None); };
        if canonical != expected_after { return Err(original_account_fault("postcommit canonical original differs")); }
        if let Some(claim) = claimed_here {
            if canonical.state != (OriginalIssueState::Preparing { claim_id: claim }) {
                return Err(original_account_fault("positive claim readback differs"));
            }
            *self.owned_claim.lock().await = Some(canonical.clone());
        }
        guard.complete = true;
        Ok(Some(canonical))
    }
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl AccountOriginalController for AccountOriginalRowController {
    fn reserve_original(&self, intent: OriginalIssueIntent) -> AccountFuture<'_> {
        Box::pin(self.transition(OriginalAccountStep::Reserve(intent)))
    }
    fn observe(&self, operation: Uuid) -> AccountFuture<'_> {
        Box::pin(async move {
            self.authority.require_original_submission_live()?;
            let mut guard = self.begin()?;
            if operation != self.authority.intent.operation_id {
                return Err(original_account_fault("observe original operation differs"));
            }
            let observed = SurrealStorage::with_operation_deadline(
                self.authority.original_submission.original_deadline.into_std(), self.authority.canonical_read(),
            ).await?;
            self.authority.require_original_submission_live()?;
            if observed.is_some() { guard.complete = true; }
            Ok(observed)
        })
    }
    fn claim_preparation(&self, expected: OriginalIssueRow, claim_id: Uuid) -> AccountFuture<'_> {
        Box::pin(self.transition(OriginalAccountStep::Claim(expected, claim_id)))
    }
    fn mark_write_started(&self, expected: OriginalIssueRow, descriptor: enrollment::EnrollmentDescriptor) -> AccountFuture<'_> {
        Box::pin(self.transition(OriginalAccountStep::WriteStarted(expected, descriptor)))
    }
    fn bind_original(&self, expected: OriginalIssueRow) -> AccountFuture<'_> {
        Box::pin(self.transition(OriginalAccountStep::Bind(expected)))
    }
}
#[derive(Debug)]
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
struct OriginalAccountDeadlineError(handshake_member_enrollment::invitation::DeadlineError);
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl std::fmt::Display for OriginalAccountDeadlineError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(formatter, "{:?}", self.0) }
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl std::error::Error for OriginalAccountDeadlineError {}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl OriginalAccountDeadlineError {
    fn boxed(error: handshake_member_enrollment::invitation::DeadlineError) -> AccountError { Box::new(Self(error)) }
}

#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl SurrealStorage {
    // Private mounting factory. The actual OsKeychainSecretsVault namespace is
    // read through its inspected getter; this performs no keychain I/O.
    async fn capture_original_account_controller(
        &self,
        registry: std::sync::Arc<crate::capabilities::CapabilityRegistry>,
        credentials: RoleAuthorityAuthInput,
        original_submission: &OriginalSubmission,
        reauthentication: super::local_accounts::CurrentActorPasswordReauthentication,
        vault: &crate::model_runtime::cloud::secrets_vault::OsKeychainSecretsVault,
    ) -> Result<AccountOriginalRowController, AccountError> {
        // Native reviewed 3dbb OriginalSubmission has these exact public fields.
        // Retain its complete immutable value before any async authority capture;
        // a Ticket cannot exist yet and no new deadline is constructed.
        let original_submission = original_submission.clone();
        let intent = original_submission.intent.clone();
        if tokio::time::Instant::now() >= original_submission.original_deadline {
            return Err(original_account_fault("original submission expired before capture"));
        }
        let exact_deadline = original_submission.original_deadline.into_std();
        SurrealStorage::with_operation_deadline(exact_deadline, async move {
        encode_original_issue_request(&intent)?;
        require_original_issue_deadline(&intent)?;
        if intent.vault_namespace != vault.service_namespace()
            || intent.vault_reference != format!("invitation/identity_enrollment/{}", intent.operation_id.hyphenated())
            || intent.purpose != "identity_enrollment"
        { return Err(original_account_fault("configured original vault identity differs")); }
        if tokio::time::Instant::now() >= original_submission.original_deadline {
            return Err(original_account_fault("original submission expired before enrolling capture"));
        }
        let enroll = self.capture_current_administration(
            &registry, credentials.clone(), enrollment::CommandKind::Enroll,
            &intent.administrator_resource, intent.original_enroller.workspace_id(),
        ).await?;
        if tokio::time::Instant::now() >= original_submission.original_deadline {
            return Err(original_account_fault("original submission expired before assignment capture"));
        }
        let assign = self.capture_current_administration(
            &registry, credentials, enrollment::CommandKind::AssignFixedMemberProfile,
            &intent.administrator_resource, intent.original_enroller.workspace_id(),
        ).await?;
        for id in [intent.staged.account_id(), intent.staged.principal_id(), intent.staged.access_space_id()] {
            validate_uuid(id)?;
        }
        let lifetime = CapturedAdministrationLifetime {
            administration_resource_id: enroll.facts.resource_id.clone(),
            policy_version: *enroll.facts.policy_versions.get(1).ok_or_else(|| original_account_fault("original resource policy absent"))?,
            choice: enroll.facts.administration_lifetime.as_ref()
                .ok_or_else(|| original_account_fault("original registered lifetime absent"))?.closed_choice()?,
        };
        let binding = MemberAdministrationParentBindings { resource: enroll.facts.resource_id.clone() };
        if tokio::time::Instant::now() >= original_submission.original_deadline {
            return Err(original_account_fault("original submission expired before parent read"));
        }
        let parents: Vec<MemberAdministrationParent> = self.with_record_user_scope(
            enroll.original_scope.clone(), self.with_data_operation(move |db| Box::pin(async move {
                db.query_values_at(ORIGINAL_ACCOUNT_PARENT_SQL, binding, 0).await
            })),
        ).await?;
        let [parent] = parents.as_slice() else { return Err(original_account_fault("original registered parent absent")); };
        let policy_expires_at = intent.policy_expires_at.map(|time| {
            chrono::DateTime::<Utc>::from_timestamp(time.unix_seconds(), time.nanoseconds())
                .map(Datetime::from).ok_or_else(|| original_account_fault("original policy time outside range"))
        }).transpose()?;
        if tokio::time::Instant::now() >= original_submission.original_deadline {
            return Err(original_account_fault("original submission expired before credential recheck"));
        }
        self.recheck_current_actor_password(
            &reauthentication, &enroll.credentials.session_token, &enroll.credentials.channel_binding_hash,
        ).await?;
        let issuer_credential_revision = reauthentication.bound_credential_revision(
            self, intent.operation_id, intent.operation_id, &enroll.session,
            &enroll.credentials.session_token, &enroll.credentials.channel_binding_hash,
            &enroll.facts.policy_versions, &enroll.facts.revocation_epochs,
        )?;
        let assignment_credential_revision = reauthentication.bound_credential_revision(
            self, intent.operation_id, intent.operation_id, &assign.session,
            &assign.credentials.session_token, &assign.credentials.channel_binding_hash,
            &assign.facts.policy_versions, &assign.facts.revocation_epochs,
        )?;
        if assignment_credential_revision != issuer_credential_revision {
            return Err(original_account_fault("original action credential revisions differ"));
        }
        let issue = MemberEnrollmentIssueProjection {
            issuer_account: enroll.facts.account_id.clone(), issuer_principal: enroll.facts.principal_id.clone(),
            issuer_session: enroll.facts.session_id.clone(), issuer_space: enroll.facts.access_space_id.clone(),
            issuer_credential_revision,
            administration_resource: enroll.facts.resource_id.clone(), workspace_resource: parent.parent.clone(),
            workspace_external: parent.parent_external.clone(), workspace_policy: parent.parent_policy,
            enroll_source: enroll.facts.clone(), assignment_source: assign.facts.clone(),
            lifetime: AdministrationLifetimeRow::from_explicit(lifetime.choice)?, policy_expires_at,
        };
        let authority = OriginalAccountAuthority {
            storage: self.clone(), registry, intent, original_submission, enroll, assign, lifetime, parent: parent.clone(), issue, reauthentication,
        };
        authority.check_original()?; authority.check_parent(&authority.parent)?;
        authority.current_read_authority().await?;
        Ok(AccountOriginalRowController {
            authority: std::sync::Arc::new(authority),
            uncertain: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            busy: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            owned_claim: tokio::sync::Mutex::new(None),
            cancellation: super::ContextOperationCancellation::new(),
        })
        }).await
    }
}


// resource_authority.rs, alongside the closed per-original controller factory.
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl SurrealStorage {
    pub(crate) async fn capture_original_member_invitation_controller(
        &self,
        registry: std::sync::Arc<crate::capabilities::CapabilityRegistry>,
        credentials: RoleAuthorityAuthInput,
        original: &crate::model_runtime::cloud::member_enrollment::OriginalSubmission,
        password: String,
        vault: &crate::model_runtime::cloud::secrets_vault::OsKeychainSecretsVault,
    ) -> Result<std::sync::Arc<dyn crate::model_runtime::cloud::member_enrollment::AccountOriginalController>,
        crate::model_runtime::cloud::member_enrollment::AccountError>
    {
        let original = original.clone();
        let operation_id = original.intent.operation_id;
        if tokio::time::Instant::now() >= original.original_deadline {
            return Err(original_account_fault("original submission expired before password verification"));
        }
        SurrealStorage::with_operation_deadline(original.original_deadline.into_std(), async move {
            // The actual issuer/session is derived from these live credentials,
            // not a supplied Owner marker or a revision metadata value.
            let proof = self.reauthenticate_current_actor_password(
                operation_id, operation_id, &credentials.session_token,
                &credentials.channel_binding_hash, password,
            ).await?;
            if tokio::time::Instant::now() >= original.original_deadline {
                return Err(original_account_fault("original submission expired after password verification"));
            }
            let controller = self.capture_original_account_controller(
                registry, credentials, &original, proof, vault,
            ).await?;
            Ok(std::sync::Arc::new(controller) as std::sync::Arc<dyn
                crate::model_runtime::cloud::member_enrollment::AccountOriginalController>)
        }).await
    }
}

// New internal per-original Account issue boundary in resource_authority.rs.
// Caller must be the existing authenticated host producer; no new endpoint.
// Consume the actual lifecycle APIs added by Runtime188 in member_enrollment.
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
pub(crate) enum MemberInvitationIssueBoundaryError {
    Authentication {
        original: crate::model_runtime::cloud::member_enrollment::OriginalSubmission,
        error: ResourceAuthorityError,
    },
    Observation {
        error: crate::model_runtime::cloud::member_enrollment::InvitationAdmissionError<std::convert::Infallible>,
    },
    Capture {
        original: crate::model_runtime::cloud::member_enrollment::OriginalSubmission,
        error: crate::model_runtime::cloud::member_enrollment::AccountError,
    },
    Configuration {
        original: crate::model_runtime::cloud::member_enrollment::OriginalSubmission,
        error: crate::model_runtime::cloud::member_enrollment::AccountError,
    },
    Admission(crate::model_runtime::cloud::member_enrollment::InvitationAdmissionError<std::io::Error>),
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl SurrealStorage {
    pub(crate) fn member_invitation_vault_namespace(
        &self,
    ) -> Result<String, crate::model_runtime::cloud::member_enrollment::AccountError> {
        // Same inspected installation identity convention as existing account
        // session vault, but a separately named invitation namespace.
        let identity = serde_json::to_vec(&(
            self.config().path().to_string_lossy(), self.config().namespace(), self.config().database(),
        ))?;
        Ok(format!("handshake-member-enrollment-invitations-{}", hex::encode(Sha256::digest(identity))))
    }

    pub(crate) async fn admit_original_member_invitation(
        &self,
        lifecycle: &crate::model_runtime::cloud::member_enrollment::InvitationLifecycle,
        registry: std::sync::Arc<crate::capabilities::CapabilityRegistry>,
        credentials: RoleAuthorityAuthInput,
        original: crate::model_runtime::cloud::member_enrollment::OriginalSubmission,
        password: String,
    ) -> Result<crate::model_runtime::cloud::member_enrollment::Ticket, MemberInvitationIssueBoundaryError> {
        use crate::model_runtime::cloud::member_enrollment::OriginalInvitationWriter;
        use crate::model_runtime::cloud::secrets_vault::OsKeychainSecretsVault;
        // History observation is authenticated but never creates a worker or
        // returns a plaintext secret, fresh capability, session or delivery proof.
        let session = self.authenticate_local_session(
            &credentials.session_token, &credentials.channel_binding_hash,
        ).await.map_err(|error| MemberInvitationIssueBoundaryError::Authentication {
            original: original.clone(), error,
        })?;
        let actor = &original.intent.original_enroller;
        if session.session_id != actor.authenticated_session_id()
            || session.identity.account_id != actor.owner_account_id()
            || session.identity.principal_id != actor.actor_principal_id()
            || session.identity.access_space_id != actor.access_space_id() {
            return Err(MemberInvitationIssueBoundaryError::Authentication {
                original, error: ResourceAuthorityError::InvalidInput("original issue actor differs"),
            });
        }
        if let Some(ticket) = lifecycle.reobserve_original(original.clone())
            .map_err(|error| MemberInvitationIssueBoundaryError::Observation { error })? {
            // Settlement custody only. Any subsequent secret redelivery must
            // use its separate fresh replay gates; this receipt grants none.
            return Ok(ticket);
        }
        let namespace = self.member_invitation_vault_namespace().map_err(|error| {
            MemberInvitationIssueBoundaryError::Configuration { original: original.clone(), error }
        })?;
        if original.intent.vault_namespace != namespace
            || original.intent.vault_reference != format!(
                "invitation/identity_enrollment/{}", original.intent.operation_id.hyphenated(),
            )
        {
            return Err(MemberInvitationIssueBoundaryError::Configuration {
                original, error: original_account_fault("configured invitation vault differs"),
            });
        }
        let root = self.config().path().parent().ok_or_else(|| MemberInvitationIssueBoundaryError::Configuration {
            original: original.clone(), error: original_account_fault("configured invitation root unavailable"),
        })?.to_path_buf();
        let vault = std::sync::Arc::new(OsKeychainSecretsVault::new(namespace));
        let controller = self.capture_original_member_invitation_controller(
            registry, credentials, &original, password, vault.as_ref(),
        ).await.map_err(|error| MemberInvitationIssueBoundaryError::Capture {
            original: original.clone(), error,
        })?;
        // Gate's synchronous closure creates a writer only for a NEW accepted
        // original. It stores the real strong owner before reserve() can effect.
        // Re-checking the same full input in the lifecycle closes capture races.
        lifecycle.admit_original(original, move || {
            OriginalInvitationWriter::new(&root, vault, controller)
        }).map_err(MemberInvitationIssueBoundaryError::Admission)
    }
}
// Actual current caller locations inspected: api/authority.rs has authenticated
// session-credential helpers and /authority setup/login/session routes, but NO
// original invitation command/endpoint. Primary owns outward native/API wiring.
// The real caller must construct ONE full OriginalSubmission from server-current
// captures and the namespace getter, attach the once-owned InvitationLifecycle,
// and retain the exact original request/absolute deadline through this method.
// This does not invent that absent route, a global dispatcher, default grant,
// fresh operation, extra profile, new session or proof of successful issuance.


// resource_authority.rs candidate. Requires the exact reviewed but unadopted
// administration captures, enrollment_time/source, lifetime and c03d helpers.
// Route owner retains ONE Arc<MemberInvitationIssueSeed> by operation UUID before
// polling this builder. Repeated HTTP input must match that seed; never call new
// for redelivery, failed capture, timeout or an unknown original-row observation.
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
#[derive(PartialEq, Eq)]
pub(crate) struct MemberInvitationIssueSelectors {
    pub(crate) operation_id: uuid::Uuid,
    pub(crate) account_name: String,
    pub(crate) workspace_id: String,
    pub(crate) administrator_resource: String,
}

#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
pub(crate) struct MemberInvitationIssueSeed {
    storage: SurrealStorage,
    selectors: MemberInvitationIssueSelectors,
    credentials: RoleAuthorityAuthInput,
    staged: enrollment::StagedIdentity,
    issued_at: enrollment::Timestamp,
    original_clock: tokio::time::Instant,
    maximum_capture_deadline: tokio::time::Instant,
    captured: tokio::sync::OnceCell<MemberInvitationIssueSeedCapture>,
}

#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
struct MemberInvitationIssueSeedCapture {
    submission: crate::model_runtime::cloud::member_enrollment::OriginalSubmission,
    registry: std::sync::Arc<crate::capabilities::CapabilityRegistry>,
    _enroll: CapturedAdministrationAuthority,
    _assign: CapturedAdministrationAuthority,
    _lifetime: CapturedAdministrationLifetime,
}

// The error retains the real original seed, not reconstructed selectors or IDs.
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
pub(crate) struct MemberInvitationIssueSeedError {
    pub(crate) seed: std::sync::Arc<MemberInvitationIssueSeed>,
    pub(crate) error: crate::model_runtime::cloud::member_enrollment::AccountError,
}

#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
pub(crate) struct CapturedMemberInvitationSubmission {
    seed: std::sync::Arc<MemberInvitationIssueSeed>,
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl CapturedMemberInvitationSubmission {
    pub(crate) fn original(
        &self,
    ) -> &crate::model_runtime::cloud::member_enrollment::OriginalSubmission {
        &self.seed.captured.get().expect("successful capture retains original").submission
    }
    pub(crate) fn seed(&self) -> &std::sync::Arc<MemberInvitationIssueSeed> { &self.seed }
}

#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl MemberInvitationIssueSeed {
    // Synchronous: every staged ID and both time samples are minted once before
    // any async capture. No invitation secret, descriptor or digest is invented.
    pub(crate) fn new(
        storage: &SurrealStorage,
        selectors: MemberInvitationIssueSelectors,
        credentials: RoleAuthorityAuthInput,
    ) -> Result<std::sync::Arc<Self>, crate::model_runtime::cloud::member_enrollment::AccountError> {
        if selectors.operation_id.is_nil()
            || selectors.account_name.is_empty()
            || selectors.account_name != selectors.account_name.trim()
            || selectors.account_name.len() > 128
            || selectors.workspace_id.trim().is_empty()
            || selectors.administrator_resource.trim().is_empty()
            || selectors.workspace_id.len() > enrollment::MAX_STRING_BYTES
            || selectors.administrator_resource.len() > enrollment::MAX_STRING_BYTES
            || credentials.session_token.is_empty()
            || credentials.channel_binding_hash.is_empty()
        {
            return Err(original_account_fault("invalid original issue selectors"));
        }
        // Monotonic first is conservative relative to the paired UTC sample.
        let original_clock = tokio::time::Instant::now();
        let issued_at = enrollment_time(&Datetime::from(Utc::now()))?;
        let maximum_capture_deadline = original_clock.checked_add(std::time::Duration::from_secs(60))
            .ok_or_else(|| original_account_fault("original issue clock overflow"))?;
        let staged = enrollment::StagedIdentity::new(
            uuid::Uuid::now_v7().to_string(),
            uuid::Uuid::now_v7().to_string(),
            uuid::Uuid::now_v7().to_string(),
        )?;
        Ok(std::sync::Arc::new(Self {
            storage: storage.clone(), selectors, credentials, staged, issued_at,
            original_clock, maximum_capture_deadline, captured: tokio::sync::OnceCell::new(),
        }))
    }

    pub(crate) fn operation_id(&self) -> uuid::Uuid { self.selectors.operation_id }

    pub(crate) fn matches_redelivery(
        &self,
        storage: &SurrealStorage,
        selectors: &MemberInvitationIssueSelectors,
        credentials: &RoleAuthorityAuthInput,
    ) -> bool {
        std::sync::Arc::ptr_eq(&self.storage.inner, &storage.inner)
            && &self.selectors == selectors
            && self.credentials.session_token == credentials.session_token
            && self.credentials.channel_binding_hash == credentials.channel_binding_hash
    }
}

#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl SurrealStorage {
    pub(crate) async fn capture_member_invitation_submission(
        &self,
        seed: std::sync::Arc<MemberInvitationIssueSeed>,
        registry: std::sync::Arc<crate::capabilities::CapabilityRegistry>,
    ) -> Result<CapturedMemberInvitationSubmission, MemberInvitationIssueSeedError> {
        let captured = async {
            if !std::sync::Arc::ptr_eq(&self.inner, &seed.storage.inner) {
                return Err(original_account_fault("original issue storage differs"));
            }
            let capture = seed.captured.get_or_try_init(|| async {
                if tokio::time::Instant::now() >= seed.maximum_capture_deadline {
                    return Err(original_account_fault("original issue capture expired"));
                }
                SurrealStorage::with_operation_deadline(seed.maximum_capture_deadline.into_std(), async {
                    let enroll = self.capture_current_administration(
                        &registry, seed.credentials.clone(), enrollment::CommandKind::Enroll,
                        &seed.selectors.administrator_resource, &seed.selectors.workspace_id,
                    ).await?;
                    if tokio::time::Instant::now() >= seed.maximum_capture_deadline {
                        return Err(original_account_fault("original issue assignment capture expired"));
                    }
                    let assign = self.capture_current_administration(
                        &registry, seed.credentials.clone(), enrollment::CommandKind::AssignFixedMemberProfile,
                        &seed.selectors.administrator_resource, &seed.selectors.workspace_id,
                    ).await?;
                    if enroll.exact != assign.exact
                        || enroll.exact.workspace_id() != seed.selectors.workspace_id
                        || enroll.original_scope.action != ResourceAction::Create
                        || assign.original_scope.action != ResourceAction::Update
                        || enroll.original_scope.capability_id != enrollment::CommandKind::Enroll.capability_id()
                        || assign.original_scope.capability_id != enrollment::CommandKind::AssignFixedMemberProfile.capability_id()
                        || enroll.original_scope.session_token != assign.original_scope.session_token
                        || enroll.original_scope.channel_binding_hash != assign.original_scope.channel_binding_hash
                        || enroll.original_scope.session_id != assign.original_scope.session_id
                        || enroll.original_scope.resource_id != assign.original_scope.resource_id
                        || enroll.original_scope.workspace_id != assign.original_scope.workspace_id
                    {
                        return Err(original_account_fault("original issue captures differ"));
                    }
                    let lifetime = CapturedAdministrationLifetime {
                        administration_resource_id: enroll.facts.resource_id.clone(),
                        policy_version: *enroll.facts.policy_versions.get(1)
                            .ok_or_else(|| original_account_fault("original resource policy missing"))?,
                        choice: enroll.facts.administration_lifetime.as_ref()
                            .ok_or_else(|| original_account_fault("original lifetime missing"))?.closed_choice()?,
                    };
                    for gate in [&enroll, &assign] {
                        lifetime.require_matches_source(&gate.facts)?;
                        let choice = gate.facts.administration_lifetime.as_ref()
                            .ok_or_else(|| original_account_fault("original lifetime missing"))?.closed_choice()?;
                        if choice != lifetime.choice {
                            return Err(original_account_fault("original lifetime choices differ"));
                        }
                    }
                    let enroll_source = enrollment_source(&enroll.facts)?;
                    let assignment_source = enrollment_source(&assign.facts)?;
                    let session_expires_at = enrollment_time(&enroll.facts.session_expires_at)?;
                    if enrollment_time(&assign.facts.session_expires_at)? != session_expires_at {
                        return Err(original_account_fault("original session expiry differs"));
                    }
                    let policy_expires_at = lifetime.source030_bound();
                    // Exactly Source030 maximum_deadline's pre-digest bound.
                    // Calling descriptor validation here would require a fake digest.
                    let mut expires_at = seed.issued_at.checked_add_seconds(60)?.min(session_expires_at);
                    for source in [&enroll_source, &assignment_source] {
                        if let Some(expiry) = source.grant_expires_at() { expires_at = expires_at.min(expiry); }
                    }
                    if let Some(expiry) = policy_expires_at { expires_at = expires_at.min(expiry); }
                    if expires_at <= seed.issued_at {
                        return Err(original_account_fault("original issue has no validity"));
                    }
                    let issued_utc = chrono::DateTime::<Utc>::from_timestamp(
                        seed.issued_at.unix_seconds(), seed.issued_at.nanoseconds(),
                    ).ok_or_else(|| original_account_fault("original issue time outside range"))?;
                    let expires_utc = chrono::DateTime::<Utc>::from_timestamp(
                        expires_at.unix_seconds(), expires_at.nanoseconds(),
                    ).ok_or_else(|| original_account_fault("original expiry outside range"))?;
                    let duration = (expires_utc - issued_utc).to_std()
                        .map_err(|_| original_account_fault("original issue duration denied"))?;
                    let original_deadline = seed.original_clock.checked_add(duration)
                        .ok_or_else(|| original_account_fault("original deadline overflow"))?;
                    if tokio::time::Instant::now() >= original_deadline {
                        return Err(original_account_fault("original issue expired during capture"));
                    }
                    let intent = crate::model_runtime::cloud::member_enrollment::OriginalIssueIntent {
                        operation_id: seed.selectors.operation_id,
                        original_enroller: enroll.exact.clone(), staged: seed.staged.clone(),
                        account_name: seed.selectors.account_name.clone(),
                        administrator_resource: seed.selectors.administrator_resource.clone(),
                        enroll_source, assignment_source, issued_at: seed.issued_at, expires_at,
                        session_expires_at, policy_expires_at,
                        purpose: "identity_enrollment".to_owned(),
                        vault_namespace: self.member_invitation_vault_namespace()?,
                        vault_reference: format!("invitation/identity_enrollment/{}", seed.selectors.operation_id.hyphenated()),
                    };
                    require_original_issue_deadline(&intent)?;
                    encode_original_issue_request(&intent)?;
                    Ok(MemberInvitationIssueSeedCapture {
                        submission: crate::model_runtime::cloud::member_enrollment::OriginalSubmission {
                            intent, original_deadline,
                        },
                        registry: registry.clone(), _enroll: enroll, _assign: assign, _lifetime: lifetime,
                    })
                }).await
            }).await?;
            if !std::sync::Arc::ptr_eq(&registry, &capture.registry) {
                return Err(original_account_fault("original issue registry differs"));
            }
            Ok::<(), crate::model_runtime::cloud::member_enrollment::AccountError>(())
        }.await;
        match captured {
            Ok(()) => Ok(CapturedMemberInvitationSubmission { seed }),
            Err(error) => Err(MemberInvitationIssueSeedError { seed, error }),
        }
    }
}
// Primary/Core173 own the actual route's atomic operation-keyed ArcSeed retention,
// full redelivery equality and retained password/response frame. No route/store is
// supplied here. Without that owner, new() cannot safely be called on HTTP retry.
// c03d admit_original_member_invitation receives original().clone() unchanged;
// this capture performs no DB writes, reserve, entropy-secret or vault operation.


// New private Source030 recipient controller. Intended in resource_authority.rs.
// Actual selected crypto home: crate::model_runtime::cloud::member_enrollment.
// This does not reuse a normal authenticated_session or an issuer password.
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
use crate::model_runtime::cloud::member_enrollment as recipient_crypto;
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
use crate::model_runtime::cloud::secrets_vault::MemberEnrollmentRecipientCredential;
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
use std::sync::Arc as RecipientArc;
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
use std::time::Instant as RecipientInstant;
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
use surrealdb::types::{Object as RecipientObject, Value as RecipientValue};

#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
struct MemberRecipientPrepared {
    storage: SurrealStorage,
    credential: RecipientArc<MemberEnrollmentRecipientCredential>,
    descriptor: enrollment::EnrollmentDescriptor,
    original: MemberEnrollmentBindings,
    canonical_original: RecipientObject,
    account_before: RecipientObject,
    principal_before: RecipientObject,
    space_before: RecipientObject,
    prior_revision: i64,
    attempt_id: Uuid,
    password_verifier: String,
    proof: recipient_crypto::OwnedRecipientSecretObservation,
    deadline: RecipientInstant,
    canonical_client: std::sync::OnceLock<super::SurrealClient>,
}
#[derive(SurrealValue)]
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
struct MemberRecipientSignin {
    original: RecordId,
    key_id: String,
    secret_hex: String,
}
#[derive(Clone, PartialEq, Eq, SurrealValue)]
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
struct MemberRecipientIdentity {
    access: String,
    original: RecordId,
    jwt_access: String,
    jwt_original: String,
    jti: String,
}
#[derive(Clone, SurrealValue)]
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
struct MemberRecipientOriginalBinding { original: RecordId }
#[derive(Clone, SurrealValue)]
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
struct MemberRecipientPermitBinding {
    permit: RecordId,
    attempt: String,
    key_id: String,
    a_auth: RecordId,
    a_jti: String,
    original: RecipientObject,
    account_before: RecipientObject,
    principal_before: RecipientObject,
    space_before: RecipientObject,
    prior_revision: i64,
    phc: String,
}
#[derive(SurrealValue)]
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
struct MemberRecipientInitialSnapshot {
    original: RecipientObject,
    account: RecipientObject,
    principal: RecipientObject,
    space: RecipientObject,
    staged: bool,
}
#[derive(SurrealValue)]
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
struct MemberRecipientFinalSnapshot {
    original: RecipientObject,
    account: RecipientObject,
    principal: RecipientObject,
    space: RecipientObject,
    permit: RecipientObject,
    complete: bool,
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
const MEMBER_RECIPIENT_IDENTITY: &str =
    "RETURN { access: $access, original: $auth.id, jwt_access: $token.AC, jwt_original: $token.ID, jti: $token.jti };";
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
const MEMBER_RECIPIENT_INITIAL: &str =
    "RETURN { original: (SELECT * FROM ONLY $original),
        account: (SELECT * FROM ONLY $original.account_id),
        principal: (SELECT * FROM ONLY $original.principal_id),
        space: (SELECT * FROM ONLY $original.access_space_id),
        staged: fn::wp012_member_recipient_staged(fn::wp012_member_recipient_auth_row()) };";
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
const MEMBER_RECIPIENT_FINAL: &str =
    "LET $p = (SELECT * FROM ONLY $permit);
     RETURN { original: (SELECT * FROM ONLY $a_auth),
        account: (SELECT * FROM ONLY $a_auth.account_id),
        principal: (SELECT * FROM ONLY $a_auth.principal_id),
        space: (SELECT * FROM ONLY $a_auth.access_space_id),
        permit: $p, complete: fn::wp012_member_recipient_complete($p) };";
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
fn member_recipient_denied() -> ResourceAuthorityError {
    ResourceAuthorityError::InvalidInput("invitation denied")
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
fn member_recipient_decode<T: SurrealValue>(object: RecipientObject) -> Result<T, ResourceAuthorityError> {
    T::from_value(RecipientValue::Object(object)).map_err(|_| member_recipient_denied())
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
fn member_recipient_single<T>(rows: Vec<T>) -> Result<T, ResourceAuthorityError> {
    let mut rows = rows;
    if rows.len() != 1 { return Err(member_recipient_denied()); }
    rows.pop().ok_or_else(member_recipient_denied)
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
fn member_recipient_revision(object: &RecipientObject) -> Result<i64, ResourceAuthorityError> {
    let value = object.get("credential_revision").ok_or_else(member_recipient_denied)?.clone();
    i64::from_value(value).map_err(|_| member_recipient_denied())
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
fn member_recipient_deadline(deadline: RecipientInstant) -> Result<(), ResourceAuthorityError> {
    if RecipientInstant::now() >= deadline { return Err(member_recipient_denied()); }
    Ok(())
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
fn member_recipient_require_identity(
    identity: &MemberRecipientIdentity, original: &RecordId, access: &str,
) -> Result<(), ResourceAuthorityError> {
    if identity.access != access || identity.jwt_access != access
        || &identity.original != original || identity.jwt_original != surrealdb::types::ToSql::to_sql(original)
        || identity.jti.trim().is_empty() { return Err(member_recipient_denied()); }
    Ok(())
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl MemberRecipientPrepared {
    fn permit_binding(&self, identity: &MemberRecipientIdentity) -> Result<MemberRecipientPermitBinding, ResourceAuthorityError> {
        member_recipient_require_identity(identity, &self.original.enrollment, "member_enrollment_recipient")?;
        self.proof.require_same_descriptor(&self.descriptor).map_err(|_| member_recipient_denied())?;
        if self.proof.operation_id() != self.descriptor.operation_id() { return Err(member_recipient_denied()); }
        Ok(MemberRecipientPermitBinding {
            permit: RecordId::new("member_enrollment_redemption_permits", self.attempt_id.to_string()),
            attempt: self.attempt_id.to_string(), key_id: self.credential.key_id().to_owned(),
            a_auth: self.original.enrollment.clone(), a_jti: identity.jti.clone(),
            original: self.canonical_original.clone(), account_before: self.account_before.clone(),
            principal_before: self.principal_before.clone(), space_before: self.space_before.clone(),
            prior_revision: self.prior_revision, phc: self.password_verifier.clone(),
        })
    }
    fn require_bound_row(&self, raw: &RecipientObject) -> Result<MemberEnrollmentRow, ResourceAuthorityError> {
        let row: MemberVaultStageRow = member_recipient_decode(raw.clone())?;
        row.require_bound_original(&self.descriptor, &self.original)
    }
    fn require_final(
        &self, snapshot: MemberRecipientFinalSnapshot, binding: &MemberRecipientPermitBinding,
    ) -> Result<(), ResourceAuthorityError> {
        let row = self.require_bound_row(&snapshot.original)?;
        if !snapshot.complete || row.status != "consumed"
            || row.redemption_attempt_id.as_deref() != Some(binding.attempt.as_str())
            || row.password_verifier.as_deref() != Some(self.password_verifier.as_str())
            || row.consumed_at.is_none() { return Err(member_recipient_denied()); }
        let mut expected_original = self.canonical_original.clone();
        for field in ["status", "password_verifier", "redemption_attempt_id", "consumed_at"] {
            let value = snapshot.original.get(field).ok_or_else(member_recipient_denied)?.clone();
            expected_original.insert(field, value);
        }
        if expected_original != snapshot.original { return Err(member_recipient_denied()); }
        for (before, after, status, account) in [
            (&self.account_before, &snapshot.account, "enabled", true),
            (&self.principal_before, &snapshot.principal, "enabled", false),
            (&self.space_before, &snapshot.space, "active", false),
        ] {
            let mut expected = before.clone();
            expected.insert("status", status);
            let old_nonce = before.get("authorization_touch_nonce").cloned().unwrap_or(0_i64.into_value());
            let old_nonce = i64::from_value(old_nonce).map_err(|_| member_recipient_denied())?;
            expected.insert("authorization_touch_nonce", old_nonce.checked_add(1).ok_or_else(member_recipient_denied)?);
            let updated = after.get("updated_at").ok_or_else(member_recipient_denied)?.clone();
            if Datetime::from_value(updated.clone()).is_err() { return Err(member_recipient_denied()); }
            expected.insert("updated_at", updated);
            if account {
                expected.insert("password_verifier", self.password_verifier.clone());
                expected.insert("credential_revision", self.prior_revision.checked_add(1).ok_or_else(member_recipient_denied)?);
            }
            if &expected != after { return Err(member_recipient_denied()); }
        }
        let mut expected_permit = match binding.clone().into_value() {
            RecipientValue::Object(value) => value,
            _ => return Err(member_recipient_denied()),
        };
        expected_permit.remove("permit");
        expected_permit.insert("id", binding.permit.clone());
        expected_permit.insert("state", "consumed");
        if expected_permit != snapshot.permit { return Err(member_recipient_denied()); }
        Ok(())
    }
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl SurrealStorage {
    async fn member_recipient_signin(
        &self, client: &super::SurrealClient, credential: &MemberEnrollmentRecipientCredential,
        original: &RecordId, access: &'static str,
    ) -> Result<MemberRecipientIdentity, ResourceAuthorityError> {
        if !matches!(access, "member_enrollment_recipient_inspection" | "member_enrollment_recipient_installer" | "member_enrollment_recipient")
            || credential.key_id() != "member-enrollment-recipient-v1" { return Err(member_recipient_denied()); }
        let params = credential.with_signin_secret(|secret| MemberRecipientSignin {
            original: original.clone(), key_id: credential.key_id().to_owned(), secret_hex: secret.to_owned(),
        });
        client.signin(RecordSignin {
            namespace: self.config().namespace().to_owned(), database: self.config().database().to_owned(),
            access: access.to_owned(), params,
        }).await.map_err(SurrealStorageError::from)?;
        let response = client.query(MEMBER_RECIPIENT_IDENTITY).await.map_err(SurrealStorageError::from)?;
        let identity = member_recipient_single(super::decode_query_values(response, 0)?)?;
        member_recipient_require_identity(&identity, original, access)?;
        Ok(identity)
    }

    async fn prepare_member_recipient_redemption(
        &self, credential: RecipientArc<MemberEnrollmentRecipientCredential>,
        attempt_id: Uuid, descriptor: enrollment::EnrollmentDescriptor,
        presented: zeroize::Zeroizing<String>, password: String, deadline: RecipientInstant,
    ) -> Result<RecipientArc<MemberRecipientPrepared>, ResourceAuthorityError> {
        member_recipient_deadline(deadline)?;
        if attempt_id.is_nil() || attempt_id == descriptor.operation_id() { return Err(member_recipient_denied()); }
        let plan = enrollment::prepare_enrollment_plan(&descriptor);
        let original = MemberEnrollmentBindings::original(&plan)?;
        let storage = self.clone();
        let read_credential = credential.clone();
        let read_original = original.enrollment.clone();
        let read = self.with_lease(move |client| Box::pin(async move {
            let scoped = (*client).clone();
            let result: Result<MemberRecipientInitialSnapshot, ResourceAuthorityError> = async {
                member_recipient_deadline(deadline)?;
                storage.member_recipient_signin(&scoped, &read_credential, &read_original, "member_enrollment_recipient_inspection").await?;
                member_recipient_deadline(deadline)?;
                let response = scoped.query(MEMBER_RECIPIENT_INITIAL)
                    .bind(MemberRecipientOriginalBinding { original: read_original }).await.map_err(SurrealStorageError::from)?;
                member_recipient_single(super::decode_query_values(response, 0)?)
            }.await;
            Ok(result)
        })).await??;
        member_recipient_deadline(deadline)?;
        let bound_row: MemberVaultStageRow = member_recipient_decode(read.original.clone())?;
        let canonical = bound_row.require_bound_original(&descriptor, &original)?;
        if !read.staged || canonical.status != "active" || canonical.password_verifier.is_some()
            || canonical.redemption_attempt_id.is_some() || canonical.consumed_at.is_some() {
            return Err(member_recipient_denied());
        }
        let prior_revision = member_recipient_revision(&read.account)?;
        if prior_revision != 0 { return Err(member_recipient_denied()); }
        let crypto_row = recipient_crypto::OriginalInvitationRow {
            operation_id: descriptor.operation_id(), descriptor_hex: &canonical.descriptor_hex,
            descriptor_sha256: &canonical.descriptor_sha256,
            account_id: descriptor.staged().account_id(), principal_id: descriptor.staged().principal_id(),
            access_space_id: descriptor.staged().access_space_id(), account_name: &canonical.account_name,
            token_digest_hex: &canonical.token_digest, purpose: &canonical.purpose,
            issued_at: enrollment_time(&canonical.issued_at)?, expires_at: enrollment_time(&canonical.expires_at)?,
            status: &canonical.status,
        };
        let binding = recipient_crypto::bind_original(&descriptor, &crypto_row, &bound_row.vault_reference)
            .map_err(|_| member_recipient_denied())?;
        let proof = recipient_crypto::verify_presented(&binding, presented.as_str(), "identity_enrollment", descriptor.operation_id())
            .map_err(|_| member_recipient_denied())?.into_owned();
        drop(presented);
        let password_verifier = super::local_accounts::new_password_verifier(password).await?;
        member_recipient_deadline(deadline)?;
        // No mutating SQL/vault/session/grant action occurred before this full frame.
        Ok(RecipientArc::new(MemberRecipientPrepared {
            storage: self.clone(), credential, descriptor, original, canonical_original: read.original,
            account_before: read.account, principal_before: read.principal, space_before: read.space,
            prior_revision, attempt_id, password_verifier, proof, deadline, canonical_client: std::sync::OnceLock::new(),
        }))
    }
}

// Private physical scope. Neither a row/digest nor a caller value can construct it.
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
struct MemberRecipientPhysical {
    storage: SurrealStorage,
    transaction: RecipientArc<surrealdb::method::Transaction<super::Db>>,
    prepared: RecipientArc<MemberRecipientPrepared>,
    identity: MemberRecipientIdentity,
}
tokio::task_local! { static MEMBER_RECIPIENT_PHYSICAL: MemberRecipientPhysical; }

#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
struct MemberEnrollmentRedemptionPermit<'borrow, 'engine> {
    storage: &'borrow SurrealStorage,
    transaction: &'borrow super::SurrealTransactionContext<'engine>,
    prepared: &'borrow RecipientArc<MemberRecipientPrepared>,
    identity: MemberRecipientIdentity,
    binding: MemberRecipientPermitBinding,
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl<'borrow, 'engine> MemberEnrollmentRedemptionPermit<'borrow, 'engine> {
    async fn capture(
        storage: &'borrow SurrealStorage, tx: &'borrow super::SurrealTransactionContext<'engine>,
        prepared: &'borrow RecipientArc<MemberRecipientPrepared>,
    ) -> Result<Self, ResourceAuthorityError> {
        prepared.proof.require_same_descriptor(&prepared.descriptor).map_err(|_| member_recipient_denied())?;
        let identity = MEMBER_RECIPIENT_PHYSICAL.try_with(|physical| {
            if !std::sync::Arc::ptr_eq(&physical.storage.inner, &storage.inner)
                || !std::sync::Arc::ptr_eq(&prepared.storage.inner, &storage.inner)
                || !RecipientArc::ptr_eq(&physical.prepared, prepared)
                || !std::ptr::eq(tx.transaction, physical.transaction.as_ref()) { return Err(member_recipient_denied()); }
            member_recipient_require_identity(&physical.identity, &prepared.original.enrollment, "member_enrollment_recipient")?;
            Ok(physical.identity.clone())
        }).map_err(|_| member_recipient_denied())??;
        let binding = prepared.permit_binding(&identity)?;
        let permit = Self { storage, transaction: tx, prepared, identity, binding };
        permit.require_same_physical(tx).await?;
        Ok(permit)
    }
    async fn require_same_physical(&self, tx: &super::SurrealTransactionContext<'_>) -> Result<(), ResourceAuthorityError> {
        if !std::ptr::eq(tx, self.transaction) { return Err(member_recipient_denied()); }
        MEMBER_RECIPIENT_PHYSICAL.try_with(|physical| {
            if !RecipientArc::ptr_eq(&physical.prepared, self.prepared)
                || !std::sync::Arc::ptr_eq(&physical.storage.inner, &self.storage.inner)
                || !std::ptr::eq(tx.transaction, physical.transaction.as_ref())
                || physical.identity != self.identity { return Err(member_recipient_denied()); }
            Ok(())
        }).map_err(|_| member_recipient_denied())??;
        let identity = member_recipient_single(tx.query_values_at::<MemberRecipientIdentity, _>(
            MEMBER_RECIPIENT_IDENTITY, (), 0,
        ).await?)?;
        if identity != self.identity { return Err(member_recipient_denied()); }
        Ok(())
    }
    async fn lookup(
        &self, tx: &super::SurrealTransactionContext<'_>,
    ) -> Result<MemberEnrollmentRow, ResourceAuthorityError> {
        self.require_same_physical(tx).await?;
        let raw = member_recipient_single(tx.query_values_at::<RecipientObject, _>(
            "SELECT * FROM $enrollment;", self.prepared.original.clone(), 0,
        ).await?)?;
        self.prepared.require_bound_row(&raw)
    }
    async fn first_redemption(
        &self, tx: &super::SurrealTransactionContext<'_>,
    ) -> Result<(), ResourceAuthorityError> {
        self.require_same_physical(tx).await?;
        member_recipient_deadline(self.prepared.deadline)?;
        let snapshot = member_recipient_single(tx.query_values_at::<MemberRecipientInitialSnapshot, _>(
            MEMBER_RECIPIENT_INITIAL, MemberRecipientOriginalBinding { original: self.prepared.original.enrollment.clone() }, 0,
        ).await?)?;
        if !snapshot.staged || snapshot.original != self.prepared.canonical_original
            || snapshot.account != self.prepared.account_before || snapshot.principal != self.prepared.principal_before
            || snapshot.space != self.prepared.space_before { return Err(member_recipient_denied()); }
        Ok(())
    }
}

#[derive(SurrealValue)]
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
struct MemberRecipientInstallBinding { permit: RecordId, content: RecipientObject }
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl SurrealStorage {
    async fn install_member_recipient_permit(
        &self, installer: &super::SurrealClient, prepared: &MemberRecipientPrepared,
        binding: &MemberRecipientPermitBinding,
    ) -> Result<(), ResourceAuthorityError> {
        member_recipient_deadline(prepared.deadline)?;
        prepared.proof.require_same_descriptor(&prepared.descriptor).map_err(|_| member_recipient_denied())?;
        if binding.attempt != prepared.attempt_id.to_string() || binding.original != prepared.canonical_original
            || binding.phc != prepared.password_verifier || binding.prior_revision != prepared.prior_revision {
            return Err(member_recipient_denied());
        }
        self.member_recipient_signin(installer, &prepared.credential,
            &prepared.original.enrollment, "member_enrollment_recipient_installer").await?;
        member_recipient_deadline(prepared.deadline)?;
        let RecipientValue::Object(mut content) = binding.clone().into_value() else {
            return Err(member_recipient_denied());
        };
        content.remove("permit");
        content.insert("state", "prepared");
        let response = installer.query("CREATE $permit CONTENT $content RETURN AFTER;")
            .bind(MemberRecipientInstallBinding { permit: binding.permit.clone(), content: content.clone() })
            .await.map_err(SurrealStorageError::from)?;
        let actual: RecipientObject = member_recipient_single(super::decode_query_values(response, 0)?)?;
        content.insert("id", binding.permit.clone());
        if actual != content { return Err(member_recipient_denied()); }
        // Permit is durable one-attempt admission, not a new invitation/session/grant.
        // A failure/unknown here never authorizes another CREATE or another attempt.
        Ok(())
    }

    async fn redeem_member_recipient_in_transaction(
        &self, tx: &super::SurrealTransactionContext<'_>,
        permit: &MemberEnrollmentRedemptionPermit<'_, '_>,
    ) -> Result<MemberRedemptionObservation, ResourceAuthorityError> {
        let p = permit.prepared;
        let row = permit.lookup(tx).await?;
        if row.status == "consumed" {
            if row.redemption_attempt_id.as_deref() != Some(permit.binding.attempt.as_str())
                || row.password_verifier.as_deref() != Some(p.password_verifier.as_str())
                || row.consumed_at.is_none() { return Err(member_recipient_denied()); }
            let final_row = member_recipient_single(tx.query_values_at::<MemberRecipientFinalSnapshot, _>(
                MEMBER_RECIPIENT_FINAL, permit.binding.clone(), 1,
            ).await?)?;
            p.require_final(final_row, &permit.binding)?;
            return Ok(MemberRedemptionObservation::SameAttemptAlreadyConsumed);
        }
        permit.first_redemption(tx).await?;
        member_recipient_deadline(p.deadline)?;
        let claimed = member_recipient_single(tx.query_values_at::<MemberEnrollmentRow, _>(
            "UPDATE $a_auth SET status = 'consumed', password_verifier = $phc,
                redemption_attempt_id = $attempt, consumed_at = time::now()
             WHERE status = 'active' AND password_verifier = NONE
                AND redemption_attempt_id = NONE AND consumed_at = NONE
                AND vault_issue_state = 'bound' AND vault_issue_revision = 4
                AND operation_id = $original.operation_id AND purpose = 'identity_enrollment'
                AND issued_at <= time::now() AND expires_at > time::now()
             RETURN AFTER;",
            permit.binding.clone(), 0,
        ).await?)?;
        p.original.require_original(&claimed)?;
        if claimed.status != "consumed" || claimed.password_verifier.as_deref() != Some(p.password_verifier.as_str())
            || claimed.redemption_attempt_id.as_deref() != Some(permit.binding.attempt.as_str())
            || claimed.consumed_at.is_none() { return Err(member_recipient_denied()); }
        permit.require_same_physical(tx).await?;
        member_recipient_deadline(p.deadline)?;
        let accounts: Vec<RecipientObject> = tx.query_values_at(
            "UPDATE $original.account_id SET status = 'enabled', password_verifier = $phc,
                credential_revision = credential_revision + 1,
                authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1, updated_at = time::now()
             WHERE status = 'disabled' AND password_verifier = NONE AND credential_revision = $prior_revision
                AND member_enrollment_origin = $a_auth RETURN AFTER;",
            permit.binding.clone(), 0,
        ).await?;
        member_recipient_single(accounts)?;
        permit.require_same_physical(tx).await?;
        member_recipient_deadline(p.deadline)?;
        let principals: Vec<RecipientObject> = tx.query_values_at(
            "UPDATE $original.principal_id SET status = 'enabled',
                authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1, updated_at = time::now()
             WHERE status = 'disabled' AND account_id = $original.account_id
                AND member_enrollment_origin = $a_auth RETURN AFTER;",
            permit.binding.clone(), 0,
        ).await?;
        member_recipient_single(principals)?;
        permit.require_same_physical(tx).await?;
        member_recipient_deadline(p.deadline)?;
        let spaces: Vec<RecipientObject> = tx.query_values_at(
            "UPDATE $original.access_space_id SET status = 'active',
                authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1, updated_at = time::now()
             WHERE status = 'disabled' AND account_id = $original.account_id
                AND member_enrollment_origin = $a_auth RETURN AFTER;",
            permit.binding.clone(), 0,
        ).await?;
        member_recipient_single(spaces)?;
        permit.require_same_physical(tx).await?;
        member_recipient_deadline(p.deadline)?;
        let consumed_permits: Vec<RecipientObject> = tx.query_values_at(
            "UPDATE $permit SET state = 'consumed' WHERE state = 'prepared' AND attempt = $attempt
                AND a_auth = $a_auth AND a_jti = $a_jti RETURN AFTER;",
            permit.binding.clone(), 0,
        ).await?;
        member_recipient_single(consumed_permits)?;
        let final_row = member_recipient_single(tx.query_values_at::<MemberRecipientFinalSnapshot, _>(
            MEMBER_RECIPIENT_FINAL, permit.binding.clone(), 1,
        ).await?)?;
        p.require_final(final_row, &permit.binding)?;
        permit.require_same_physical(tx).await?;
        member_recipient_deadline(p.deadline)?;
        Ok(MemberRedemptionObservation::ConsumedNow)
    }
}

// Errors are retained whole, never mapped to a successful or retryable result.
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
pub(crate) enum MemberRecipientFailure {
    Authority(ResourceAuthorityError),
    Storage(SurrealStorageError),
    Rollback { original: ResourceAuthorityError, cancel: Option<surrealdb::Error> },
    RetainedTransaction(RecipientArc<surrealdb::method::Transaction<super::Db>>),
    Join(tokio::task::JoinError),
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl std::fmt::Debug for MemberRecipientFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MemberRecipientFailure(retained)")
    }
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl std::fmt::Display for MemberRecipientFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("invitation denied") }
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl std::error::Error for MemberRecipientFailure {}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl From<ResourceAuthorityError> for MemberRecipientFailure {
    fn from(value: ResourceAuthorityError) -> Self { Self::Authority(value) }
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl From<SurrealStorageError> for MemberRecipientFailure {
    fn from(value: SurrealStorageError) -> Self { Self::Storage(value) }
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
pub(crate) enum MemberRecipientOutcome { ConsumedNow, SameAttemptAlreadyConsumed }
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl SurrealStorage {
    async fn run_member_recipient_redemption(
        &self, prepared: RecipientArc<MemberRecipientPrepared>,
    ) -> Result<MemberRecipientOutcome, MemberRecipientFailure> {
        if !std::sync::Arc::ptr_eq(&self.inner, &prepared.storage.inner) {
            return Err(member_recipient_denied().into());
        }
        let storage = self.clone();
        let result = self.with_lease(move |client| Box::pin(async move {
            let result: Result<MemberRecipientOutcome, MemberRecipientFailure> = async {
                member_recipient_deadline(prepared.deadline)?;
                let recipient = (*client).clone();
                let installer = (*client).clone();
                let identity = storage.member_recipient_signin(&recipient, &prepared.credential,
                    &prepared.original.enrollment, "member_enrollment_recipient").await?;
                let binding = prepared.permit_binding(&identity)?;
                prepared.canonical_client.set(recipient.clone())
                    .map_err(|_| member_recipient_denied())?;
                storage.install_member_recipient_permit(&installer, &prepared, &binding).await?;
                member_recipient_deadline(prepared.deadline)?;
                let transaction = RecipientArc::new(recipient.begin().await.map_err(SurrealStorageError::from)?);
                let physical = MemberRecipientPhysical {
                    storage: storage.clone(), transaction: transaction.clone(),
                    prepared: prepared.clone(), identity,
                };
                let callback: Result<MemberRedemptionObservation, ResourceAuthorityError> =
                    MEMBER_RECIPIENT_PHYSICAL.scope(physical, async {
                        let tx = super::SurrealTransactionContext { transaction: transaction.as_ref() };
                        let permit = MemberEnrollmentRedemptionPermit::capture(&storage, &tx, &prepared).await?;
                        storage.redeem_member_recipient_in_transaction(&tx, &permit).await
                    }).await;
                // The task-local owns the only auxiliary Arc and has now dropped.
                let transaction = RecipientArc::try_unwrap(transaction)
                    .map_err(MemberRecipientFailure::RetainedTransaction)?;
                let observed = match callback {
                    Ok(observed) => observed,
                    Err(original) => {
                        let cancel = transaction.cancel().await.err();
                        return Err(MemberRecipientFailure::Rollback { original, cancel });
                    }
                };
                if let Err(original) = member_recipient_deadline(prepared.deadline) {
                    let cancel = transaction.cancel().await.err();
                    return Err(MemberRecipientFailure::Rollback { original, cancel });
                }
                // No outer timeout/abort/drop race around submitted engine finalization.
                let committed_client = transaction.commit().await.map_err(SurrealStorageError::from)?;
                let response = committed_client.query(MEMBER_RECIPIENT_FINAL)
                    .bind(binding.clone()).await.map_err(SurrealStorageError::from)?;
                let final_row = member_recipient_single(super::decode_query_values(response, 1)?)?;
                prepared.require_final(final_row, &binding)?;
                Ok(match observed {
                    MemberRedemptionObservation::ConsumedNow => MemberRecipientOutcome::ConsumedNow,
                    MemberRedemptionObservation::SameAttemptAlreadyConsumed => MemberRecipientOutcome::SameAttemptAlreadyConsumed,
                })
            }.await;
            Ok(result)
        })).await?;
        result
    }

    async fn reobserve_member_recipient_redemption(
        &self, controller: &MemberRecipientRedemptionController,
    ) -> Result<MemberRecipientOutcome, MemberRecipientFailure> {
        let prepared = controller.prepared.clone();
        if !std::sync::Arc::ptr_eq(&self.inner, &prepared.storage.inner) {
            return Err(member_recipient_denied().into());
        }
        let client = prepared.canonical_client.get().ok_or_else(member_recipient_denied)?.clone();
        let storage = self.clone();
        // Explicit side-effect-free reconciliation on the retained original JWT.
        // No fresh key/signin/attempt/PHC, no pre-expiry outcome inferred from time.
        let result = self.with_lease(move |_| Box::pin(async move {
            let result: Result<MemberRecipientOutcome, MemberRecipientFailure> = async {
                let response = client.query(MEMBER_RECIPIENT_IDENTITY).await.map_err(SurrealStorageError::from)?;
                let identity = member_recipient_single(super::decode_query_values(response, 0)?)?;
                let binding = prepared.permit_binding(&identity)?;
                let response = client.query(MEMBER_RECIPIENT_FINAL).bind(binding.clone())
                    .await.map_err(SurrealStorageError::from)?;
                let final_row = member_recipient_single(super::decode_query_values(response, 1)?)?;
                prepared.require_final(final_row, &binding)?;
                let _same_storage = storage;
                Ok(MemberRecipientOutcome::SameAttemptAlreadyConsumed)
            }.await;
            Ok(result)
        })).await?;
        result
    }
}

// Storage-bound custody is published before the worker starts its first effect.
// No public cancel/abort/forget API. Retain pending/unknown/raw finalization errors.
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
pub(crate) type MemberRecipientResult = Result<MemberRecipientOutcome, MemberRecipientFailure>;
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
struct MemberRecipientControllerState {
    handle: Option<tokio::task::JoinHandle<MemberRecipientResult>>,
    outcome: Option<RecipientArc<MemberRecipientResult>>,
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
pub(crate) struct MemberRecipientRedemptionController {
    prepared: RecipientArc<MemberRecipientPrepared>,
    state: tokio::sync::Mutex<MemberRecipientControllerState>,
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
struct MemberRecipientRejectedInput {
    prepared: RecipientArc<MemberRecipientPrepared>,
    reason: ResourceAuthorityError,
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
type MemberRecipientInventory = std::collections::HashMap<Uuid, RecipientArc<MemberRecipientRedemptionController>>;
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
enum MemberRecipientRegistry {
    Accepting(MemberRecipientInventory),
    Closing(MemberRecipientInventory),
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
pub(crate) struct MemberRecipientRedemptionRuntime {
    storage: SurrealStorage,
    credential: RecipientArc<MemberEnrollmentRecipientCredential>,
    operations: std::sync::Mutex<MemberRecipientRegistry>,
    operation_requests: std::sync::Mutex<MemberRecipientOperationRequests>,
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl MemberRecipientRedemptionController {
    fn start(
        prepared: RecipientArc<MemberRecipientPrepared>, registry: &std::sync::Mutex<MemberRecipientRegistry>,
    ) -> Result<RecipientArc<Self>, MemberRecipientRejectedInput> {
        let key = prepared.attempt_id;
        let mut state = registry.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let registry = match &mut *state {
            MemberRecipientRegistry::Accepting(registry) => registry,
            MemberRecipientRegistry::Closing(_) => return Err(MemberRecipientRejectedInput {
                prepared, reason: member_recipient_denied(),
            }),
        };
        if let Some(original) = registry.get(&key) {
            return if RecipientArc::ptr_eq(&original.prepared, &prepared) { Ok(original.clone()) }
                else { Err(MemberRecipientRejectedInput { prepared, reason: member_recipient_denied() }) };
        }
        let controller = RecipientArc::new(Self {
            prepared: prepared.clone(),
            state: tokio::sync::Mutex::new(MemberRecipientControllerState { handle: None, outcome: None }),
        });
        // This new, unpublished mutex cannot have another owner yet.
        let Ok(mut state) = controller.state.try_lock() else {
            return Err(MemberRecipientRejectedInput { prepared, reason: member_recipient_denied() });
        };
        registry.insert(key, controller.clone());
        let storage = prepared.storage.clone();
        state.handle = Some(tokio::spawn(async move {
            SurrealStorage::with_operation_deadline(prepared.deadline,
                storage.run_member_recipient_redemption(prepared.clone())).await
        }));
        drop(state);
        Ok(controller)
    }
    pub(crate) async fn wait(&self) -> RecipientArc<MemberRecipientResult> {
        let mut state = self.state.lock().await;
        if let Some(outcome) = &state.outcome { return outcome.clone(); }
        // Await by mutable borrow: cancelled waiters leave the actual JoinHandle
        // in this retained controller. Other waiters serialize without lost wakers.
        let outcome = match state.handle.as_mut() {
            Some(handle) => match handle.await {
                Ok(outcome) => outcome,
                Err(error) => Err(MemberRecipientFailure::Join(error)),
            },
            None => Err(MemberRecipientFailure::Authority(member_recipient_denied())),
        };
        state.handle.take();
        let outcome = RecipientArc::new(outcome);
        state.outcome = Some(outcome.clone());
        outcome
    }
}
// Registry release, startup shutdown/store-retirement joining and actual API
// mounting are explicit owner integrations, not granted by this candidate.
// No sessions/grants are minted. Only normal password login consumes the final
// canonical redeemed Member mapping; this worker returns operation state only.

#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
pub(crate) struct MemberRecipientRedemptionRequest {
    pub(crate) attempt_id: Uuid,
    pub(crate) descriptor: enrollment::EnrollmentDescriptor,
    pub(crate) presented: zeroize::Zeroizing<String>,
    pub(crate) password: String,
    pub(crate) original_deadline: std::time::Instant,
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl MemberRecipientRedemptionRuntime {
    pub(crate) fn new(
        storage: SurrealStorage, credential: RecipientArc<MemberEnrollmentRecipientCredential>,
    ) -> Result<Self, ResourceAuthorityError> {
        if credential.key_id() != "member-enrollment-recipient-v1" { return Err(member_recipient_denied()); }
        Ok(Self { storage, credential, operations: std::sync::Mutex::new(MemberRecipientRegistry::Accepting(Default::default())), operation_requests: std::sync::Mutex::new(MemberRecipientOperationRequests { accepting: true, requests: Vec::new(), verified_winners: Default::default(), rejected: Vec::new() }) })
    }
    pub(crate) async fn begin(
        &self, request: MemberRecipientRedemptionRequest,
    ) -> Result<RecipientArc<MemberRecipientRedemptionController>, ResourceAuthorityError> {
        if matches!(&*self.operations.lock().unwrap_or_else(|poisoned| poisoned.into_inner()),
            MemberRecipientRegistry::Closing(_)) { return Err(member_recipient_denied()); }
        let prepared = self.storage.prepare_member_recipient_redemption(
            self.credential.clone(), request.attempt_id, request.descriptor,
            request.presented, request.password, request.original_deadline,
        ).await?;
        MemberRecipientRedemptionController::start(prepared, &self.operations).map_err(|rejected| {
            let MemberRecipientRejectedInput { prepared: _retained_input, reason } = rejected;
            reason
        })
    }
    pub(crate) async fn observe_original(
        &self, controller: &RecipientArc<MemberRecipientRedemptionController>,
    ) -> Result<MemberRecipientOutcome, MemberRecipientFailure> {
        let owned = {
            let state = self.operations.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            let inventory = match &*state {
                MemberRecipientRegistry::Accepting(inventory) | MemberRecipientRegistry::Closing(inventory) => inventory,
            };
            inventory.get(&controller.prepared.attempt_id).is_some_and(|original| RecipientArc::ptr_eq(original, controller))
        };
        if !owned { return Err(member_recipient_denied().into()); }
        self.storage.reobserve_member_recipient_redemption(controller).await
    }
    pub(crate) fn begin_shutdown(&self) -> Vec<RecipientArc<MemberRecipientRedemptionController>> {
        self.close_operation_requests();
        let mut state = self.operations.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let previous = std::mem::replace(&mut *state, MemberRecipientRegistry::Closing(Default::default()));
        let inventory = match previous {
            MemberRecipientRegistry::Accepting(inventory) | MemberRecipientRegistry::Closing(inventory) => inventory,
        };
        let snapshot = inventory.values().cloned().collect();
        *state = MemberRecipientRegistry::Closing(inventory);
        snapshot
    }
    pub(crate) fn retained_controllers(&self) -> Vec<RecipientArc<MemberRecipientRedemptionController>> {
        let state = self.operations.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        match &*state {
            MemberRecipientRegistry::Accepting(inventory) | MemberRecipientRegistry::Closing(inventory) =>
                inventory.values().cloned().collect(),
        }
    }
}
// Startup owns ONE Arc<MemberRecipientRedemptionRuntime> through shutdown.
// Runtime188 must drain/inspect these retained controllers BEFORE retiring storage
// or the vault credential. No global registry, per-request credential reload,
// implicit reset, abort or forgotten-unknown path is supplied.
// Normal password login remains the existing login/session API after canonical
// consumed mapping. This API returns redemption operation state only.

// Operation-ID recipient entry. All caller data is retained before inspection.
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
struct MemberRecipientOperationRequests {
    accepting: bool,
    requests: Vec<RecipientArc<MemberRecipientOperationRequest>>,
    verified_winners: std::collections::HashMap<Uuid, RecipientArc<MemberRecipientOperationRequest>>,
    rejected: Vec<RecipientArc<MemberRecipientOperationRejected>>,
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl MemberRecipientOperationRequests {
    fn retain_rejected(
        &mut self, rejected: MemberRecipientOperationRejected,
    ) -> RecipientArc<MemberRecipientOperationRejected> {
        let rejected = RecipientArc::new(rejected);
        self.rejected.push(rejected.clone());
        rejected
    }
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
pub(crate) struct MemberRecipientOperationRejected {
    pub(crate) operation_id: Uuid,
    pub(crate) presented: zeroize::Zeroizing<String>,
    pub(crate) password: zeroize::Zeroizing<String>,
    pub(crate) reason: ResourceAuthorityError,
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
enum MemberRecipientOperationFailureKind {
    Preparation(ResourceAuthorityError),
    Admission(MemberRecipientRejectedInput),
    Join(tokio::task::JoinError),
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
pub(crate) struct MemberRecipientOperationFailure {
    _retained: MemberRecipientOperationFailureKind,
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
pub(crate) type MemberRecipientOperationResult = Result<
    RecipientArc<MemberRecipientRedemptionController>, MemberRecipientOperationFailure,
>;
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
struct MemberRecipientOperationState {
    handle: Option<tokio::task::JoinHandle<MemberRecipientOperationResult>>,
    outcome: Option<RecipientArc<MemberRecipientOperationResult>>,
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
pub(crate) struct MemberRecipientOperationRequest {
    operation_id: Uuid,
    attempt_id: Uuid,
    presented: zeroize::Zeroizing<String>,
    password: zeroize::Zeroizing<String>,
    original_clock: RecipientInstant,
    sampled_utc: chrono::DateTime<Utc>,
    maximum_deadline: RecipientInstant,
    resolved_deadline: std::sync::OnceLock<RecipientInstant>,
    state: tokio::sync::Mutex<MemberRecipientOperationState>,
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl MemberRecipientOperationRequest {
    pub(crate) fn operation_id(&self) -> Uuid { self.operation_id }
    pub(crate) fn attempt_id(&self) -> Uuid { self.attempt_id }
    pub(crate) async fn wait(&self) -> RecipientArc<MemberRecipientOperationResult> {
        let mut state = self.state.lock().await;
        if let Some(outcome) = &state.outcome { return outcome.clone(); }
        let outcome = match state.handle.as_mut() {
            Some(handle) => match handle.await {
                Ok(result) => result,
                Err(error) => Err(MemberRecipientOperationFailure {
                    _retained: MemberRecipientOperationFailureKind::Join(error),
                }),
            },
            None => Err(MemberRecipientOperationFailure {
                _retained: MemberRecipientOperationFailureKind::Preparation(member_recipient_denied()),
            }),
        };
        state.handle.take();
        let outcome = RecipientArc::new(outcome);
        state.outcome = Some(outcome.clone());
        outcome
    }
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl MemberRecipientRedemptionRuntime {
    // Synchronous retention precedes first worker poll/read/effect. Repeated
    // original operation+full raw input returns this same request, never new IDs.
    // Distinct unverified inputs remain distinct retained submissions, not winners.
    pub(crate) fn begin_operation(
        self: &RecipientArc<Self>, operation_id: Uuid,
        presented: zeroize::Zeroizing<String>, password: String,
    ) -> Result<RecipientArc<MemberRecipientOperationRequest>, RecipientArc<MemberRecipientOperationRejected>> {
        let password = zeroize::Zeroizing::new(password);
        let mut registry = self.operation_requests.lock().unwrap_or_else(|p| p.into_inner());
        if !registry.accepting || operation_id.is_nil() {
            return Err(registry.retain_rejected(MemberRecipientOperationRejected {
                operation_id, presented, password, reason: member_recipient_denied(),
            }));
        }
        // Unverified submissions reserve no per-operation winner. Reuse only a
        // full identical input; a different input gets its own retained proof
        // attempt even if an earlier input failed secret verification.
        if let Some(request) = registry.requests.iter().find(|request| {
            request.operation_id == operation_id
                && request.presented.as_str() == presented.as_str()
                && request.password.as_str() == password.as_str()
        }) {
            return Ok(request.clone());
        }
        let original_clock = RecipientInstant::now();
        let sampled_utc = Utc::now();
        let Some(maximum_deadline) = original_clock.checked_add(std::time::Duration::from_secs(60)) else {
            return Err(registry.retain_rejected(MemberRecipientOperationRejected {
                operation_id, presented, password, reason: member_recipient_denied(),
            }));
        };
        let attempt_id = Uuid::now_v7();
        if attempt_id == operation_id {
            return Err(registry.retain_rejected(MemberRecipientOperationRejected {
                operation_id, presented, password, reason: member_recipient_denied(),
            }));
        }
        let request = RecipientArc::new(MemberRecipientOperationRequest {
            operation_id, attempt_id, presented, password, original_clock, sampled_utc,
            maximum_deadline, resolved_deadline: std::sync::OnceLock::new(),
            state: tokio::sync::Mutex::new(MemberRecipientOperationState { handle: None, outcome: None }),
        });
        // New request is unpublished, so this mutable reference needs no await.
        let mut state = request.state.try_lock().expect("unpublished recipient request state");
        registry.requests.push(request.clone());
        let runtime = self.clone();
        let owned_request = request.clone();
        state.handle = Some(tokio::spawn(async move {
            runtime.prepare_operation_request(owned_request).await
        }));
        drop(state);
        Ok(request)
    }

    async fn prepare_operation_request(
        &self, request: RecipientArc<MemberRecipientOperationRequest>,
    ) -> MemberRecipientOperationResult {
        let prepared = async {
            member_recipient_deadline(request.maximum_deadline)?;
            let original = RecordId::new("member_enrollments", request.operation_id.to_string());
            let storage = self.storage.clone();
            let credential = self.credential.clone();
            let read_original = original.clone();
            let maximum_deadline = request.maximum_deadline;
            let read = SurrealStorage::with_operation_deadline(maximum_deadline,
                self.storage.with_lease(move |client| Box::pin(async move {
                    let scoped = (*client).clone();
                    let result: Result<MemberRecipientInitialSnapshot, ResourceAuthorityError> = async {
                        member_recipient_deadline(maximum_deadline)?;
                        storage.member_recipient_signin(&scoped, &credential, &read_original,
                            "member_enrollment_recipient_inspection").await?;
                        member_recipient_deadline(maximum_deadline)?;
                        let response = scoped.query(MEMBER_RECIPIENT_INITIAL)
                            .bind(MemberRecipientOriginalBinding { original: read_original })
                            .await.map_err(SurrealStorageError::from)?;
                        member_recipient_single(super::decode_query_values(response, 0)?)
                    }.await;
                    Ok(result)
                }))
            ).await??;
            member_recipient_deadline(request.maximum_deadline)?;
            let row: MemberVaultStageRow = member_recipient_decode(read.original)?;
            let encoded = row.descriptor_hex.as_ref().ok_or_else(member_recipient_denied)?;
            let bytes = hex::decode(encoded).map_err(|_| member_recipient_denied())?;
            if row.descriptor_sha256.as_deref() != Some(sha256_hex(&bytes).as_str()) {
                return Err(member_recipient_denied());
            }
            let descriptor = enrollment::decode_enrollment(&bytes).map_err(|_| member_recipient_denied())?;
            if descriptor.operation_id() != request.operation_id || !read.staged {
                return Err(member_recipient_denied());
            }
            let plan = enrollment::prepare_enrollment_plan(&descriptor);
            let binding = MemberEnrollmentBindings::original(&plan)?;
            if binding.enrollment != original { return Err(member_recipient_denied()); }
            row.require_bound_original(&descriptor, &binding)?;
            let expires = chrono::DateTime::<Utc>::from_timestamp(
                descriptor.expires_at().unix_seconds(), descriptor.expires_at().nanoseconds(),
            ).ok_or_else(member_recipient_denied)?;
            let remaining = (expires - request.sampled_utc).to_std().map_err(|_| member_recipient_denied())?;
            let deadline = request.original_clock.checked_add(remaining)
                .ok_or_else(member_recipient_denied)?.min(request.maximum_deadline);
            request.resolved_deadline.set(deadline).map_err(|_| member_recipient_denied())?;
            member_recipient_deadline(deadline)?;
            // The existing genuine prepare re-reads the same Bound original,
            // binds its full descriptor and performs real presented-secret proof
            // then bounded chosen-password KDF. No issuer password is involved.
            SurrealStorage::with_operation_deadline(deadline,
                self.storage.prepare_member_recipient_redemption(
                    self.credential.clone(), request.attempt_id, descriptor,
                    zeroize::Zeroizing::new(request.presented.to_string()),
                    request.password.to_string(), deadline,
                )
            ).await
        }.await.map_err(|error| MemberRecipientOperationFailure {
            _retained: MemberRecipientOperationFailureKind::Preparation(error),
        })?;
        // Only the actual prepared frame contains successful canonical secret
        // verification. Claim the per-operation winner under the SAME admission
        // gate before controller start; no unverified input can populate it.
        let mut registry = self.operation_requests.lock().unwrap_or_else(|p| p.into_inner());
        if !registry.accepting {
            return Err(MemberRecipientOperationFailure {
                _retained: MemberRecipientOperationFailureKind::Admission(MemberRecipientRejectedInput {
                    prepared, reason: member_recipient_denied(),
                }),
            });
        }
        if let Some(winner) = registry.verified_winners.get(&request.operation_id) {
            if !RecipientArc::ptr_eq(winner, &request) {
                return Err(MemberRecipientOperationFailure {
                    _retained: MemberRecipientOperationFailureKind::Admission(MemberRecipientRejectedInput {
                        prepared, reason: member_recipient_denied(),
                    }),
                });
            }
        } else {
            registry.verified_winners.insert(request.operation_id, request.clone());
        }
        // Keep the request gate locked through controller admission. Shutdown
        // takes this gate first, so no late admission crosses its close point.
        // A failed/unknown/effectful winner is retained and never reset here.
        let result = MemberRecipientRedemptionController::start(prepared, &self.operations)
            .map_err(|rejected| MemberRecipientOperationFailure {
                _retained: MemberRecipientOperationFailureKind::Admission(rejected),
            });
        drop(registry);
        result
    }

    fn close_operation_requests(&self) {
        self.operation_requests.lock().unwrap_or_else(|p| p.into_inner()).accepting = false;
    }
    pub(crate) fn retained_operation_rejections(&self) -> Vec<RecipientArc<MemberRecipientOperationRejected>> {
        self.operation_requests.lock().unwrap_or_else(|p| p.into_inner()).rejected.clone()
    }
    pub(crate) fn retained_operation_requests(&self) -> Vec<RecipientArc<MemberRecipientOperationRequest>> {
        self.operation_requests.lock().unwrap_or_else(|p| p.into_inner()).requests.clone()
    }
}
// Runtime188 shutdown must wait retained_operation_requests before draining the
// existing retained_controllers and retiring storage/credential. begin_shutdown
// closes request admission too; an in-flight prepare rejected by closing worker
// admission keeps its actual prepared frame in its retained request error.
// Core173 calls ArcRuntime.begin_operation(operation_id, Zeroizing(secret), password),
// retains the returned request and waits it. Retry uses the same runtime/op/input;
// an error never authorizes remint/restart/forget or a different attempt.


// External CODE candidate for resource_authority.rs. No adoption/runtime verdict.
// Depends on the same exact administration capture/lifetime helpers as fe2b3233.
// Only the existing owned worker may call apply_in_transaction; this type does
// not start, detach, retry, abort, or replace a transaction.
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
pub(crate) struct MemberRevocationPrepared {
    storage: SurrealStorage,
    registry: std::sync::Arc<crate::capabilities::CapabilityRegistry>,
    request: enrollment::RevocationRequest,
    descriptor: enrollment::EnrollmentDescriptor,
    gate: CapturedAdministrationAuthority,
    parent: MemberAdministrationParent,
    lifetime: CapturedAdministrationLifetime,
    password: super::local_accounts::CurrentActorPasswordReauthentication,
    credential_revision: i64,
    deadline: tokio::time::Instant,
}
#[derive(Clone, SurrealValue)]
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
struct MemberRevocationLookup { original: RecordId }
#[derive(Clone, PartialEq, Eq, SurrealValue)]
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
struct MemberRevocationSnapshot {
    original: surrealdb::types::Object,
    descriptor_hex: String,
    descriptor_sha256: String,
    token_digest: String,
    account: RecordId,
    principal: RecordId,
    space: RecordId,
    account_status: String,
    principal_status: String,
    space_status: String,
    account_origin: RecordId,
    principal_origin: RecordId,
    space_origin: RecordId,
}
#[derive(SurrealValue)]
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
struct MemberRevocationWrite {
    original: RecordId,
    before: surrealdb::types::Object,
    request_id: String,
    request_hex: String,
    source: RegistryPublicationAuthority,
    revision: i64,
    event: super::event_ledger::LedgerWrite,
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
const MEMBER_REVOCATION_READ: &str = "SELECT $this AS original, descriptor_hex, descriptor_sha256, token_digest,
 account_id AS account, principal_id AS principal, access_space_id AS space,
 account_id.status AS account_status, principal_id.status AS principal_status, access_space_id.status AS space_status,
 account_id.member_enrollment_origin AS account_origin, principal_id.member_enrollment_origin AS principal_origin,
 access_space_id.member_enrollment_origin AS space_origin FROM $original;";
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
const MEMBER_REVOCATION_APPLY: &str = r#"
LET $updated = UPDATE $original SET status = 'revoked', revocation_request_id = $request_id,
 revocation_request_hex = $request_hex, revocation_source = $source,
 revoker_credential_revision = $revision, revoked_at = time::now()
 WHERE $this = $before AND status = 'active' AND vault_issue_state = 'bound' AND vault_issue_revision = 4
 AND purpose = 'identity_enrollment' AND issued_at <= time::now() AND expires_at > time::now()
 AND redemption_attempt_id = NONE AND consumed_at = NONE
 AND revocation_request_id = NONE AND revocation_request_hex = NONE AND revoked_at = NONE
 RETURN AFTER;
IF array::len($updated) != 1 { THROW 'HSK-INVITATION-DENIED'; };
LET $receipt = CREATE $event.record CONTENT {
 event_id: $event.event_id, event_version: $event.event_version,
 kernel_task_run_id: $event.kernel_task_run_id, session_run_id: $event.session_run_id,
 aggregate_type: $event.aggregate_type, aggregate_id: $event.aggregate_id,
 idempotency_key: $event.idempotency_key, event_type: $event.event_type,
 actor_kind: $event.actor_kind, actor_id: $event.actor_id,
 causation_id: $event.causation_id, correlation_id: $event.correlation_id,
 payload_hash: $event.payload_hash, source_component: $event.source_component, payload: $event.payload,
 wsids: $event.wsids, authority_resource_id: $event.authority_resource_id,
 authority_session_id: $event.authority_session_id, authority_capability_id: $event.authority_capability_id,
 authority_action: $event.authority_action, created_at: $event.created_at
} RETURN AFTER;
IF array::len($receipt) != 1 { THROW 'HSK-INVITATION-DENIED'; };
SELECT id AS record, event_id, event_version, kernel_task_run_id, session_run_id, aggregate_type, aggregate_id, idempotency_key, event_type, actor_kind, actor_id, causation_id, correlation_id, payload_hash, source_component, payload, wsids, authority_resource_id, authority_session_id, authority_capability_id, authority_action, created_at FROM $event.record;
"#;
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
fn member_revocation_denied() -> ResourceAuthorityError {
    ResourceAuthorityError::InvalidInput("member revocation denied")
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl MemberRevocationPrepared {
    fn live(&self) -> Result<(), ResourceAuthorityError> {
        if tokio::time::Instant::now() >= self.deadline { return Err(member_revocation_denied()); }
        self.lifetime.require_preserved_expiry(&self.descriptor)?;
        if self.lifetime.source030_bound().is_some_and(|bound|
            enrollment_time(&Datetime::from(Utc::now())).map_or(true, |now| bound <= now)) {
            return Err(member_revocation_denied());
        }
        Ok(())
    }
    pub(crate) fn request(&self) -> &enrollment::RevocationRequest { &self.request }
    pub(crate) fn original_deadline(&self) -> tokio::time::Instant { self.deadline }
    pub(super) fn original_scope(&self) -> &RecordUserScope { &self.gate.original_scope }
    fn lookup(&self) -> MemberRevocationLookup {
        MemberRevocationLookup { original: RecordId::new("member_enrollments", self.request.enrollment_operation_id().to_string()) }
    }
    fn require_parent(&self, parent: &MemberAdministrationParent) -> Result<(), ResourceAuthorityError> {
        let facts = &self.gate.facts;
        if parent != &self.parent || parent.administration != facts.resource_id
            || parent.owner != facts.account_id || parent.space != facts.access_space_id
            || parent.parent_owner != parent.owner || parent.parent_space != parent.space
            || parent.parent_kind != "workspace" || parent.parent_state != "active"
            || parent.parent_external != self.request.current_revoker().workspace_id()
            || parent.resource_policy != self.lifetime.policy_version
            || facts.policy_versions.first().is_none_or(|policy| parent.parent_policy > *policy)
        { return Err(member_revocation_denied()); }
        Ok(())
    }
    fn require_snapshot(&self, row: &MemberRevocationSnapshot) -> Result<(), ResourceAuthorityError> {
        let bytes = enrollment::encode_enrollment(&self.descriptor).map_err(|_| member_revocation_denied())?;
        let digest = Sha256::digest(&bytes);
        let expected_digest: [u8; 32] = digest.into();
        enrollment::validate_revocation_link(&self.request, &self.descriptor, &expected_digest)
            .map_err(|_| member_revocation_denied())?;
        if row.descriptor_hex != hex::encode(&bytes) || row.descriptor_sha256 != hex::encode(expected_digest)
            || row.token_digest != hex::encode(self.descriptor.token_digest())
            || row.account != RecordId::new("local_accounts", self.descriptor.staged().account_id())
            || row.principal != RecordId::new("principals", self.descriptor.staged().principal_id())
            || row.space != RecordId::new("access_spaces", self.descriptor.staged().access_space_id())
            || row.account_origin != self.lookup().original || row.principal_origin != self.lookup().original
            || row.space_origin != self.lookup().original
            || row.account_status != "disabled" || row.principal_status != "disabled" || row.space_status != "disabled"
        { return Err(member_revocation_denied()); }
        let status = row.original.get("status");
        if status != Some(&surrealdb::types::Value::String("active".into())) {
            return Err(member_revocation_denied());
        }
        let now = enrollment_time(&Datetime::from(Utc::now()))?;
        handshake_member_enrollment::invitation::evaluate_revocation(
            &handshake_member_enrollment::invitation::InvitationSnapshot {
                descriptor: &self.descriptor, observed_descriptor_bytes: Some(&bytes),
                status: handshake_member_enrollment::invitation::InvitationStatus::Active,
                account_status: handshake_member_enrollment::invitation::IdentityStatus::Disabled,
                principal_status: handshake_member_enrollment::invitation::IdentityStatus::Disabled,
                space_status: handshake_member_enrollment::invitation::SpaceStatus::Disabled,
            }, &self.request, &expected_digest, now,
        ).map_err(|_| member_revocation_denied())?;
        Ok(())
    }
    async fn fence(&self, tx: &super::SurrealTransactionContext<'_>) -> Result<(), ResourceAuthorityError> {
        self.live()?;
        tx.require_authenticated_scope(&self.storage, &self.gate.original_scope)?;
        let rows: Vec<RegistryPublicationAuthority> = tx.query_values_at(
            REGISTRY_PUBLICATION_AUTHORITY, RegistryPublicationBindings {
                resource: self.gate.facts.resource_id.clone(), original_grant: self.gate.facts.grant_id.clone(),
                kind: ResourceKind::AccountAdministration.as_str().to_owned(),
                action: ResourceAction::Update.as_str().to_owned(),
                capability: enrollment::CommandKind::RevokeEnrollment.capability_id().to_owned(),
            }, 0,
        ).await?;
        let [facts] = rows.as_slice() else { return Err(member_revocation_denied()); };
        if facts != &self.gate.facts || self.gate.original_scope.action != ResourceAction::Update
            || self.gate.original_scope.capability_id != enrollment::CommandKind::RevokeEnrollment.capability_id()
            || !self.registry.profile_can(&facts.capability_profile_id,
                enrollment::CommandKind::RevokeEnrollment.capability_id()).unwrap_or(false)
        { return Err(member_revocation_denied()); }
        self.lifetime.require_matches_source(facts)?;
        let parents: Vec<MemberAdministrationParent> = tx.query_values_at(
            ORIGINAL_ACCOUNT_PARENT_SQL, MemberAdministrationParentBindings { resource: facts.resource_id.clone() }, 0,
        ).await?;
        let [parent] = parents.as_slice() else { return Err(member_revocation_denied()); };
        self.require_parent(parent)?;
        let revision = self.password.fenced_credential_revision_in_transaction(
            &self.storage, tx, &self.gate.original_scope, self.request.request_id(), self.request.enrollment_operation_id(),
        ).await?;
        if revision != self.credential_revision { return Err(member_revocation_denied()); }
        self.live()
    }
    // Transaction owner MUST retain this complete value/result and await actual
    // commit before publishing success. A lost observer never calls this again.
    async fn apply_in_transaction(
        &self, tx: &super::SurrealTransactionContext<'_>,
    ) -> Result<MemberRevocationCommitCandidate, ResourceAuthorityError> {
        self.fence(tx).await?;
        let rows: Vec<MemberRevocationSnapshot> = tx.query_values_at(MEMBER_REVOCATION_READ, self.lookup(), 0).await?;
        let [before] = rows.as_slice() else { return Err(member_revocation_denied()); };
        self.require_snapshot(before)?;
        let request_hex = hex::encode(enrollment::encode_revocation(&self.request).map_err(|_| member_revocation_denied())?);
        let facts = &self.gate.facts;
        let source_value = serde_json::json!({
            "resource_id": self.request.source().resource_id(),
            "grant_id": self.request.source().grant_id(),
            "grant_version": facts.grant_version,
            "owner_account_id": self.request.current_revoker().owner_account_id(),
            "actor_principal_id": self.request.current_revoker().actor_principal_id(),
            "authenticated_session_id": self.request.current_revoker().authenticated_session_id(),
            "access_space_id": self.request.current_revoker().access_space_id(),
            "workspace_id": self.request.current_revoker().workspace_id(),
            "actor_kind": facts.actor_kind, "actor_id": facts.actor_id,
            "actions": facts.actions, "capability_ids": facts.capability_ids,
            "policy_versions": facts.policy_versions, "revocation_epochs": facts.revocation_epochs,
            "delegation_chain": facts.delegation_chain,
            "session_capabilities": facts.session_capabilities, "principal_capabilities": facts.principal_capabilities,
            "capability_profile_id": facts.capability_profile_id,
            "statuses": facts.statuses, "resource_kind": facts.resource_kind,
        });
        let payload = serde_json::json!({
            "request_id": self.request.request_id().to_string(),
            "enrollment_operation_id": self.request.enrollment_operation_id().to_string(),
            "revocation_request_hex": request_hex,
            "descriptor_sha256": before.descriptor_sha256,
            "account_id": self.descriptor.staged().account_id(),
            "principal_id": self.descriptor.staged().principal_id(),
            "access_space_id": self.descriptor.staged().access_space_id(),
            "action": "update", "capability_id": enrollment::CommandKind::RevokeEnrollment.capability_id(),
            "purpose": enrollment::CommandKind::RevokeEnrollment.purpose(),
            "profile_id": "MemberEnrollment", "source": source_value,
        });
        let actor = match self.gate.facts.actor_kind.as_str() {
            "operator" => crate::kernel::KernelActor::Operator(self.gate.facts.actor_id.clone()),
            _ => return Err(member_revocation_denied()),
        };
        let event = crate::kernel::NewKernelEvent::builder(
            self.request.request_id().to_string(), self.gate.session.session_id.clone(),
            crate::kernel::KernelEventType::MemberEnrollmentRevoked, actor,
        ).aggregate("member_enrollment", self.request.enrollment_operation_id().to_string())
            .idempotency_key(format!("member-enrollment-revoked:{}", self.request.request_id()))
            .correlation_id(self.request.enrollment_operation_id().to_string())
            .source_component("member_enrollment").payload(payload).build()
            .map_err(|_| member_revocation_denied())?;
        let (candidate, mut write) = super::event_ledger::prepare_event(event).map_err(|_| member_revocation_denied())?;
        write.wsids = vec![self.request.current_revoker().workspace_id().to_owned()];
        write.authority_resource_id = Some(self.gate.facts.resource_id.clone());
        write.authority_session_id = Some(self.gate.facts.session_id.clone());
        write.authority_capability_id = Some(enrollment::CommandKind::RevokeEnrollment.capability_id().to_owned());
        write.authority_action = Some(ResourceAction::Update.as_str().to_owned());
        self.live()?;
        // Exactly the supplied physical transaction. No EventLedger::append()
        // call opens a second database operation around this mutation.
        let receipts: Vec<super::event_ledger::LedgerWrite> = tx.query_values_at(
            MEMBER_REVOCATION_APPLY, MemberRevocationWrite {
                original: self.lookup().original, before: before.original.clone(),
                request_id: self.request.request_id().to_string(), request_hex,
                source: self.gate.facts.clone(), revision: self.credential_revision, event: write.clone(),
            }, 4,
        ).await?;
        let [receipt] = receipts.as_slice() else { return Err(member_revocation_denied()); };
        if receipt.event_id != write.event_id || receipt.payload_hash != write.payload_hash
            || receipt.idempotency_key != write.idempotency_key || receipt.payload != write.payload
        { return Err(member_revocation_denied()); }
        self.fence(tx).await?;
        let binding = MemberRevocationFinalBinding { original: self.lookup().original, event: write.record.clone() };
        let final_rows: Vec<MemberRevocationFinal> = tx.query_values_at(MEMBER_REVOCATION_FINAL, binding, 0).await?;
        let [observed] = final_rows.as_slice() else { return Err(member_revocation_denied()); };
        let mut expected_original = before.original.clone();
        for key in ["status", "revocation_request_id", "revocation_request_hex", "revocation_source",
            "revoker_credential_revision", "revoked_at"] {
            expected_original.insert(key.to_owned(), observed.original.get(key).cloned().ok_or_else(member_revocation_denied)?);
        }
        if observed.original != expected_original
            || observed.receipt.clone().into_value() != write.clone().into_value()
            || observed.sequence <= 0
        { return Err(member_revocation_denied()); }
        Ok(MemberRevocationCommitCandidate { event: candidate, expected: observed.clone() })
    }
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl SurrealStorage {
    // New bounded private constructor. Input is not an authority witness.
    pub(crate) async fn prepare_member_revocation(
        &self, registry: std::sync::Arc<crate::capabilities::CapabilityRegistry>,
        credentials: RoleAuthorityAuthInput, request: enrollment::RevocationRequest,
        descriptor: enrollment::EnrollmentDescriptor, password: String,
        original_deadline: tokio::time::Instant,
    ) -> Result<MemberRevocationPrepared, ResourceAuthorityError> {
        if request.request_id().is_nil() || request.request_id() == request.enrollment_operation_id()
            || tokio::time::Instant::now() >= original_deadline
        { return Err(member_revocation_denied()); }
        SurrealStorage::with_operation_deadline(original_deadline.into_std(), async move {
            let proof = self.reauthenticate_current_actor_password(
                request.request_id(), request.enrollment_operation_id(),
                &credentials.session_token, &credentials.channel_binding_hash, password,
            ).await?;
            let gate = self.capture_current_administration(
                &registry, credentials, enrollment::CommandKind::RevokeEnrollment,
                request.administrator_resource(), request.current_revoker().workspace_id(),
            ).await?;
            if gate.exact != *request.current_revoker() || enrollment_source(&gate.facts)? != *request.source()
                || gate.original_scope.action != ResourceAction::Update
                || gate.original_scope.capability_id != enrollment::CommandKind::RevokeEnrollment.capability_id()
            { return Err(member_revocation_denied()); }
            let lifetime = CapturedAdministrationLifetime {
                administration_resource_id: gate.facts.resource_id.clone(),
                policy_version: *gate.facts.policy_versions.get(1).ok_or_else(member_revocation_denied)?,
                choice: gate.facts.administration_lifetime.as_ref().ok_or_else(member_revocation_denied)?.closed_choice()?,
            };
            lifetime.require_matches_source(&gate.facts)?;
            let bindings = MemberAdministrationParentBindings { resource: gate.facts.resource_id.clone() };
            let parents: Vec<MemberAdministrationParent> = self.with_record_user_scope(
                gate.original_scope.clone(), self.with_data_operation(move |db| Box::pin(async move {
                    db.query_values_at(ORIGINAL_ACCOUNT_PARENT_SQL, bindings, 0).await
                })),
            ).await?;
            let [parent] = parents.as_slice() else { return Err(member_revocation_denied()); };
            self.recheck_current_actor_password(&proof, &gate.credentials.session_token, &gate.credentials.channel_binding_hash).await?;
            let revision = proof.bound_credential_revision(
                self, request.request_id(), request.enrollment_operation_id(), &gate.session,
                &gate.credentials.session_token, &gate.credentials.channel_binding_hash,
                &gate.facts.policy_versions, &gate.facts.revocation_epochs,
            )?;
            let result = MemberRevocationPrepared {
                storage: self.clone(), registry, request, descriptor, gate, parent: parent.clone(),
                lifetime, password: proof, credential_revision: revision, deadline: original_deadline,
            };
            result.live()?; result.require_parent(parent)?;
            Ok(result)
        }).await
    }
}
// Current-revoker request builder and actual router/runtime mount remain external integrations.

#[derive(Clone, SurrealValue)]
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
struct MemberRevocationFinalBinding { original: RecordId, event: RecordId }
#[derive(Clone, SurrealValue)]
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
struct MemberRevocationFinal {
    original: surrealdb::types::Object,
    receipt: super::event_ledger::LedgerWrite,
    sequence: i64,
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
const MEMBER_REVOCATION_FINAL: &str = "SELECT $this AS original,
    (SELECT id AS record, event_id, event_version, kernel_task_run_id, session_run_id, aggregate_type, aggregate_id, idempotency_key, event_type, actor_kind, actor_id, causation_id, correlation_id, payload_hash, source_component, payload, wsids, authority_resource_id, authority_session_id, authority_capability_id, authority_action, created_at FROM ONLY $event) AS receipt, $event.event_sequence AS sequence FROM $original;";
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
struct MemberRevocationCommitCandidate {
    event: crate::kernel::KernelEvent,
    expected: MemberRevocationFinal,
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
pub(crate) struct MemberRevocationOutcome {
    event: crate::kernel::KernelEvent,
    sequence: i64,
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl MemberRevocationOutcome {
    pub(crate) fn event(&self) -> &crate::kernel::KernelEvent { &self.event }
    pub(crate) fn event_sequence(&self) -> i64 { self.sequence }
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
pub(crate) enum MemberRevocationFailure {
    Authority(ResourceAuthorityError),
    Storage(SurrealStorageError),
    Join(tokio::task::JoinError),
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl From<SurrealStorageError> for MemberRevocationFailure {
    fn from(error: SurrealStorageError) -> Self { Self::Storage(error) }
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl From<ResourceAuthorityError> for MemberRevocationFailure {
    fn from(error: ResourceAuthorityError) -> Self { Self::Authority(error) }
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
pub(crate) type MemberRevocationResult = Result<MemberRevocationOutcome, MemberRevocationFailure>;
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
struct MemberRevocationControllerState {
    worker: Option<tokio::task::JoinHandle<MemberRevocationResult>>,
    outcome: Option<std::sync::Arc<MemberRevocationResult>>,
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
pub(crate) struct MemberRevocationController {
    prepared: std::sync::Arc<MemberRevocationPrepared>,
    expected: std::sync::OnceLock<MemberRevocationCommitCandidate>,
    cancellation: std::sync::Arc<super::ContextOperationCancellation>,
    state: tokio::sync::Mutex<MemberRevocationControllerState>,
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
pub(crate) enum MemberRevocationAdmissionError {
    Closing { prepared: MemberRevocationPrepared },
    RegistryPoisoned { prepared: MemberRevocationPrepared },
    DifferentOriginal { prepared: MemberRevocationPrepared },
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
type MemberRevocationInventory = std::collections::HashMap<Uuid, std::sync::Arc<MemberRevocationController>>;
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
struct MemberRevocationAdmission {
    closing: bool,
    inventory: MemberRevocationInventory,
    seeds: Vec<std::sync::Arc<MemberRevocationSeedController>>,
    verified_seeds: std::collections::HashMap<Uuid, std::sync::Arc<MemberRevocationSeedController>>,
    rejected_seeds: Vec<std::sync::Arc<MemberRevocationSeedAdmissionError>>,
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
pub(crate) struct MemberRevocationRuntime {
    storage: SurrealStorage,
    admission: std::sync::Mutex<MemberRevocationAdmission>,
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl MemberRevocationRuntime {
    // Once per actual storage owner; host must retain through store retirement.
    pub(crate) fn new(storage: SurrealStorage) -> Self {
        Self { storage, admission: std::sync::Mutex::new(MemberRevocationAdmission {
            closing: false, inventory: Default::default(), seeds: Vec::new(), verified_seeds: Default::default(), rejected_seeds: Vec::new(),
        }) }
    }
    // All private authority work precedes this call. Close and insert use the
    // same gate. A duplicate never launches another worker or consumes new proof.
    pub(crate) fn admit(
        &self, prepared: MemberRevocationPrepared,
    ) -> Result<std::sync::Arc<MemberRevocationController>, MemberRevocationAdmissionError> {
        if !std::sync::Arc::ptr_eq(&self.storage.inner, &prepared.storage.inner) {
            return Err(MemberRevocationAdmissionError::DifferentOriginal { prepared });
        }
        let mut gate = match self.admission.lock() {
            Ok(gate) => gate,
            Err(_) => return Err(MemberRevocationAdmissionError::RegistryPoisoned { prepared }),
        };
        // Existing custody is accessible only by observe_original, never by
        // replacing its password/source/deadline/prepared object on this path.
        if gate.inventory.contains_key(&prepared.request.request_id()) {
            return Err(MemberRevocationAdmissionError::DifferentOriginal { prepared });
        }
        if gate.closing { return Err(MemberRevocationAdmissionError::Closing { prepared }); }
        let prepared = std::sync::Arc::new(prepared);
        let controller = std::sync::Arc::new(MemberRevocationController {
            prepared: prepared.clone(), expected: std::sync::OnceLock::new(),
            cancellation: super::ContextOperationCancellation::new(),
            state: tokio::sync::Mutex::new(MemberRevocationControllerState { worker: None, outcome: None }),
        });
        let Ok(mut state) = controller.state.try_lock() else {
            unreachable!("new unpublished controller lock");
        };
        gate.inventory.insert(prepared.request.request_id(), controller.clone());
        let owned = controller.clone();
        state.worker = Some(tokio::spawn(async move { owned.run_once().await }));
        drop(state);
        Ok(controller)
    }
    // Internal custody lookup is not HTTP authorization or delivery permission.
    // The outward handler must freshly authorize Update for this canonical target.
    pub(crate) fn retained_original(
        &self, request: &enrollment::RevocationRequest, deadline: tokio::time::Instant,
    ) -> Option<std::sync::Arc<MemberRevocationController>> {
        let gate = self.admission.lock().ok()?;
        let controller = gate.inventory.get(&request.request_id())?;
        (controller.prepared.request == *request && controller.prepared.deadline == deadline)
            .then(|| controller.clone())
    }
    pub(crate) fn begin_shutdown(&self) -> Result<MemberRevocationShutdownInventory, ResourceAuthorityError> {
        let mut gate = self.admission.lock().map_err(|_| member_revocation_denied())?;
        gate.closing = true;
        Ok(MemberRevocationShutdownInventory { seeds: gate.seeds.clone(), effects: gate.inventory.values().cloned().collect() })
    }
    pub(crate) fn retained_controllers(&self) -> Result<Vec<std::sync::Arc<MemberRevocationController>>, ResourceAuthorityError> {
        let gate = self.admission.lock().map_err(|_| member_revocation_denied())?;
        Ok(gate.inventory.values().cloned().collect())
    }
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl MemberRevocationController {
    pub(crate) fn original(&self) -> &enrollment::RevocationRequest { &self.prepared.request }
    pub(crate) fn original_deadline(&self) -> tokio::time::Instant { self.prepared.deadline }
    async fn run_once(self: std::sync::Arc<Self>) -> MemberRevocationResult {
        let storage = self.prepared.storage.clone();
        let scope = self.prepared.gate.original_scope.clone();
        let operation = self.prepared.request.request_id();
        let worker = self.clone();
        // The actual owned storage API selects the finalization-await path via
        // this retained cancellation owner. The ordinary authenticated API
        // instead permits a timeout return before its inner worker settles.
        // Preserve the WHOLE typed storage result and this outer JoinError;
        // no new retention enum, timeout, abort or retry wraps finalization.
        SurrealStorage::with_operation_deadline(
            self.prepared.deadline.into_std(), storage.with_record_user_scope(
                scope.clone(), storage.with_owned_authenticated_transaction::<
                    (), MemberRevocationFailure, _>(
                    operation, self.cancellation.clone(), move |tx| Box::pin(async move {
                        let candidate = worker.prepared.apply_in_transaction(&tx).await?;
                        worker.expected.set(candidate).map_err(|_| member_revocation_denied())?;
                        Ok(())
                    }),
                ),
            ),
        ).await?;
        // Callback retained the complete expected raw values BEFORE commit.
        // This return awaited actual commit; a commit error leaves expected
        // values + raw error in this controller without repeating the CAS.
        self.canonical_reobserve().await
    }
    // This query has no mutation, new sign-in, grant, request, PHC or deadline.
    // It reuses the same storage and original record-user scope. It does not
    // rerun an active/revocation CAS after Unknown or a cancelled HTTP waiter.
    pub(crate) async fn canonical_reobserve(&self) -> MemberRevocationResult {
        let expected = self.expected.get().ok_or_else(member_revocation_denied)?;
        let bindings = MemberRevocationFinalBinding {
            original: self.prepared.lookup().original, event: expected.expected.receipt.record.clone(),
        };
        let rows: Vec<MemberRevocationFinal> = self.prepared.storage.with_record_user_scope(
            self.prepared.gate.original_scope.clone(),
            self.prepared.storage.with_data_operation(move |db| Box::pin(async move {
                db.query_values_at(MEMBER_REVOCATION_FINAL, bindings, 0).await
            })),
        ).await?;
        let [row] = rows.as_slice() else { return Err(member_revocation_denied().into()); };
        if row.original != expected.expected.original
            || row.receipt.clone().into_value() != expected.expected.receipt.clone().into_value()
            || row.sequence != expected.expected.sequence || row.sequence <= 0
        { return Err(member_revocation_denied().into()); }
        Ok(MemberRevocationOutcome { event: expected.event.clone(), sequence: row.sequence })
    }
    pub(crate) async fn wait(&self) -> std::sync::Arc<MemberRevocationResult> {
        let mut state = self.state.lock().await;
        if let Some(outcome) = &state.outcome { return outcome.clone(); }
        let outcome = match state.worker.as_mut() {
            Some(worker) => match worker.await {
                Ok(outcome) => outcome,
                Err(error) => Err(MemberRevocationFailure::Join(error)),
            },
            None => Err(member_revocation_denied().into()),
        };
        state.worker.take();
        let outcome = std::sync::Arc::new(outcome);
        state.outcome = Some(outcome.clone());
        outcome
    }
}
// Host owns one Arc<MemberRevocationRuntime>, same mandatory router Extension,
// and begin_shutdown + wait of ALL retained controllers BEFORE storage closes.
// Every Err/JoinError/unknown keeps the runtime+storage+prepared+outcome owners.
// No release/forget/retry API is supplied. Admission errors return whole input.
// Actual route, current request construction and Runtime188 drain mount are
// required integrations, not established by this CODE candidate.

#[derive(SurrealValue)]
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
struct MemberRevocationReplayBinding { original: RecordId, event_key: String }
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
const MEMBER_REVOCATION_REPLAY: &str = "LET $events = SELECT id AS record, event_id, event_version, kernel_task_run_id, session_run_id, aggregate_type, aggregate_id, idempotency_key, event_type, actor_kind, actor_id, causation_id, correlation_id, payload_hash, source_component, payload, wsids, authority_resource_id, authority_session_id, authority_capability_id, authority_action, created_at, event_sequence FROM kernel_event_ledger
    WHERE idempotency_key = $event_key LIMIT 2;
    IF array::len($events) != 1 { THROW 'HSK-INVITATION-DENIED'; };
    SELECT $this AS original, $events[0] AS receipt, $events[0].event_sequence AS sequence FROM $original;";
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
pub(crate) struct MemberRevocationObserved {
    request_id: Uuid,
    enrollment_operation_id: Uuid,
    event_id: String,
    sequence: i64,
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl MemberRevocationObserved {
    pub(crate) fn request_id(&self) -> Uuid { self.request_id }
    pub(crate) fn enrollment_operation_id(&self) -> Uuid { self.enrollment_operation_id }
    pub(crate) fn event_id(&self) -> &str { &self.event_id }
    pub(crate) fn event_sequence(&self) -> i64 { self.sequence }
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl SurrealStorage {
    // A restart/retry reads only the exact original request and actual event.
    // Fresh current Update admission is independent of the old event's session.
    // There is no KDF/nonce/mutation/new attempt on this metadata-only path.
    pub(crate) async fn observe_member_revocation(
        &self, registry: &crate::capabilities::CapabilityRegistry,
        credentials: RoleAuthorityAuthInput, request: &enrollment::RevocationRequest,
        descriptor: &enrollment::EnrollmentDescriptor,
    ) -> Result<MemberRevocationObserved, ResourceAuthorityError> {
        let gate = self.capture_member_revocation_replay(registry, credentials, request).await?;
        let descriptor_bytes = enrollment::encode_enrollment(descriptor).map_err(|_| member_revocation_denied())?;
        let descriptor_digest: [u8;32] = Sha256::digest(&descriptor_bytes).into();
        enrollment::validate_revocation_link(request, descriptor, &descriptor_digest).map_err(|_| member_revocation_denied())?;
        let event_key = format!("member-enrollment-revoked:{}", request.request_id());
        let binding = MemberRevocationReplayBinding {
            original: RecordId::new("member_enrollments", request.enrollment_operation_id().to_string()),
            event_key: event_key.clone(),
        };
        let rows: Vec<MemberRevocationFinal> = self.with_record_user_scope(
            gate.original_scope, self.with_data_operation(move |db| Box::pin(async move {
                db.query_values_at(MEMBER_REVOCATION_REPLAY, binding, 2).await
            })),
        ).await?;
        let [row] = rows.as_slice() else { return Err(member_revocation_denied()); };
        let equals = |field: &str, value: String| {
            row.original.get(field) == Some(&surrealdb::types::Value::String(value))
        };
        let request_hex = hex::encode(enrollment::encode_revocation(request).map_err(|_| member_revocation_denied())?);
        if !equals("status", "revoked".to_owned())
            || !equals("operation_id", request.enrollment_operation_id().to_string())
            || !equals("revocation_request_id", request.request_id().to_string())
            || !equals("revocation_request_hex", request_hex)
            || !equals("descriptor_hex", hex::encode(&descriptor_bytes))
            || !equals("descriptor_sha256", hex::encode(descriptor_digest))
            || !equals("token_digest", hex::encode(descriptor.token_digest()))
            || row.receipt.idempotency_key != event_key
            || row.receipt.event_type != "MEMBER_ENROLLMENT_REVOKED"
            || row.receipt.aggregate_type != "member_enrollment"
            || row.receipt.aggregate_id != request.enrollment_operation_id().to_string()
            || row.receipt.kernel_task_run_id != request.request_id().to_string()
            || row.sequence <= 0
        { return Err(member_revocation_denied()); }
        Ok(MemberRevocationObserved { request_id: request.request_id(),
            enrollment_operation_id: request.enrollment_operation_id(),
            event_id: row.receipt.event_id.clone(), sequence: row.sequence })
    }
}

// Server input contains selectors and credentials only. No request/descriptor,
// current actor, SourceRef, source time, PHC or authority proof is deserialized.
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
pub(crate) struct MemberRevocationServerInput {
    pub(crate) request_selector: Uuid,
    pub(crate) enrollment_operation_id: Uuid,
    pub(crate) workspace: String,
    pub(crate) administrator_resource: String,
    pub(crate) credentials: RoleAuthorityAuthInput,
    pub(crate) password: zeroize::Zeroizing<String>,
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
struct MemberRevocationServerSeed {
    input: MemberRevocationServerInput,
    registry: std::sync::Arc<crate::capabilities::CapabilityRegistry>,
    requested_at: enrollment::Timestamp,
    deadline: tokio::time::Instant,
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl MemberRevocationServerSeed {
    fn matches(&self, input: &MemberRevocationServerInput,
        registry: &std::sync::Arc<crate::capabilities::CapabilityRegistry>) -> bool {
        self.input.request_selector == input.request_selector
            && self.input.enrollment_operation_id == input.enrollment_operation_id
            && self.input.workspace == input.workspace && self.input.administrator_resource == input.administrator_resource
            && self.input.credentials.session_token == input.credentials.session_token
            && self.input.credentials.channel_binding_hash == input.credentials.channel_binding_hash
            && self.input.password.as_str() == input.password.as_str()
            && std::sync::Arc::ptr_eq(&self.registry, registry)
    }
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
pub(crate) enum MemberRevocationSeedAdmissionError {
    Invalid { input: MemberRevocationServerInput },
    Closing { input: MemberRevocationServerInput },
    RegistryPoisoned { input: MemberRevocationServerInput },
    DifferentOriginal { input: MemberRevocationServerInput },
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
pub(crate) enum MemberRevocationSeedFailure {
    Preparation(ResourceAuthorityError),
    Admission(MemberRevocationAdmissionError),
    Join(tokio::task::JoinError),
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
pub(crate) type MemberRevocationSeedResult =
    Result<std::sync::Arc<MemberRevocationController>, MemberRevocationSeedFailure>;
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
struct MemberRevocationSeedState {
    worker: Option<tokio::task::JoinHandle<MemberRevocationSeedResult>>,
    outcome: Option<std::sync::Arc<MemberRevocationSeedResult>>,
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
pub(crate) struct MemberRevocationSeedController {
    seed: std::sync::Arc<MemberRevocationServerSeed>,
    original: std::sync::OnceLock<(enrollment::RevocationRequest, enrollment::EnrollmentDescriptor)>,
    state: tokio::sync::Mutex<MemberRevocationSeedState>,
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
pub(crate) struct MemberRevocationShutdownInventory {
    pub(crate) seeds: Vec<std::sync::Arc<MemberRevocationSeedController>>,
    pub(crate) effects: Vec<std::sync::Arc<MemberRevocationController>>,
}
#[derive(SurrealValue)]
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
struct MemberRevocationServerOriginal {
    operation_id: String,
    status: String,
    descriptor_hex: String,
    descriptor_sha256: String,
    token_digest: String,
    revocation_request_hex: Option<String>,
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
const MEMBER_REVOCATION_SERVER_ORIGINAL: &str =
    "SELECT operation_id, status, descriptor_hex, descriptor_sha256, token_digest, revocation_request_hex FROM $original;";
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl MemberRevocationRuntime {
    // This synchronous admission retains full input and samples clocks once
    // before any async operation. Source030 only requires a distinct nonnil
    // request UUID, so the stable untrusted selector is preserved as its ID.
    // It is not permission or a server-derived actor/source claim.
    pub(crate) fn begin(
        self: &std::sync::Arc<Self>,
        registry: std::sync::Arc<crate::capabilities::CapabilityRegistry>,
        input: MemberRevocationServerInput,
    ) -> Result<std::sync::Arc<MemberRevocationSeedController>, std::sync::Arc<MemberRevocationSeedAdmissionError>> {
        let mut gate = match self.admission.lock() {
            Ok(gate) => gate,
            Err(poisoned) => {
                let mut gate = poisoned.into_inner();
                return Err(gate.retain_seed_rejection(MemberRevocationSeedAdmissionError::RegistryPoisoned { input }));
            },
        };
        // Retain unverified submissions separately. A failed credential or
        // password cannot reserve the canonical request selector.
        if let Some(retained) = gate.seeds.iter().find(|retained| retained.seed.matches(&input, &registry)) {
            return Ok(retained.clone());
        }
        if gate.closing { return Err(gate.retain_seed_rejection(MemberRevocationSeedAdmissionError::Closing { input })); }
        if input.request_selector.is_nil() || input.enrollment_operation_id.is_nil()
            || input.request_selector == input.enrollment_operation_id
            || input.workspace.is_empty() || input.administrator_resource.is_empty()
        { return Err(gate.retain_seed_rejection(MemberRevocationSeedAdmissionError::Invalid { input })); }
        let monotonic_now = tokio::time::Instant::now();
        let utc_now = Datetime::from(Utc::now());
        let deadline = match super::OPERATION_DEADLINE.try_with(|deadline| tokio::time::Instant::from_std(*deadline)) {
            Ok(existing) => existing,
            Err(_) => match monotonic_now.checked_add(self.storage.config().statement_timeout()) {
                Some(deadline) => deadline,
                None => return Err(gate.retain_seed_rejection(MemberRevocationSeedAdmissionError::Invalid { input })),
            },
        };
        if deadline <= monotonic_now { return Err(gate.retain_seed_rejection(MemberRevocationSeedAdmissionError::Invalid { input })); }
        let requested_at = match enrollment_time(&utc_now) {
            Ok(value) => value,
            Err(_) => return Err(gate.retain_seed_rejection(MemberRevocationSeedAdmissionError::Invalid { input })),
        };
        let controller = std::sync::Arc::new(MemberRevocationSeedController {
            seed: std::sync::Arc::new(MemberRevocationServerSeed { input, registry, requested_at, deadline }),
            original: std::sync::OnceLock::new(),
            state: tokio::sync::Mutex::new(MemberRevocationSeedState { worker: None, outcome: None }),
        });
        let Ok(mut state) = controller.state.try_lock() else { unreachable!("new seed lock"); };
        gate.seeds.push(controller.clone());
        let owner = self.clone();
        let retained = controller.clone();
        state.worker = Some(tokio::spawn(async move {
            SurrealStorage::with_operation_deadline(retained.seed.deadline.into_std(),
                owner.prepare_server_seed(retained)).await
        }));
        drop(state);
        Ok(controller)
    }
    async fn prepare_server_seed(
        &self, retained: std::sync::Arc<MemberRevocationSeedController>,
    ) -> MemberRevocationSeedResult {
        let seed = &retained.seed;
        let input = &seed.input;
        let result: Result<MemberRevocationPrepared, ResourceAuthorityError> = async {
            if tokio::time::Instant::now() >= seed.deadline { return Err(member_revocation_denied()); }
            // First actual Update authorization admits the private original read.
            // The supplied selectors are checked AGAINST the canonical descriptor.
            let gate = self.storage.capture_current_administration(
                &seed.registry, input.credentials.clone(), enrollment::CommandKind::RevokeEnrollment,
                &input.administrator_resource, &input.workspace,
            ).await?;
            let original = self.storage.read_member_revocation_server_original(&gate, input.enrollment_operation_id).await?;
            let descriptor = self.storage.decode_member_revocation_server_original(
                &original, input.enrollment_operation_id, &input.workspace, &input.administrator_resource,
            )?;
            let request = enrollment::RevocationRequest::new(
                input.request_selector, input.enrollment_operation_id, descriptor.staged().clone(),
                descriptor.administrator_resource().to_owned(),
                Sha256::digest(enrollment::encode_enrollment(&descriptor).map_err(|_| member_revocation_denied())?).into(),
                gate.exact, enrollment_source(&gate.facts)?, seed.requested_at,
            ).map_err(|_| member_revocation_denied())?;
            retained.original.set((request.clone(), descriptor.clone())).map_err(|_| member_revocation_denied())?;
            // The one KDF is inside this existing private constructor. No second
            // password verification, second sampled clock or replacement attempt.
            self.storage.prepare_member_revocation(
                seed.registry.clone(), input.credentials.clone(), request, descriptor,
                input.password.as_str().to_owned(), seed.deadline,
            ).await
        }.await;
        let prepared = result.map_err(MemberRevocationSeedFailure::Preparation)?;
        // Only this fully prepared frame carries successful current Update and
        // password proof. Failed untrusted submissions never claim a selector.
        {
            let mut gate = match self.admission.lock() {
                Ok(gate) => gate,
                Err(_) => return Err(MemberRevocationSeedFailure::Admission(
                    MemberRevocationAdmissionError::RegistryPoisoned { prepared }
                )),
            };
            if gate.closing {
                return Err(MemberRevocationSeedFailure::Admission(
                    MemberRevocationAdmissionError::Closing { prepared }
                ));
            }
            if let Some(winner) = gate.verified_seeds.get(&input.request_selector) {
                if !std::sync::Arc::ptr_eq(winner, &retained) {
                    return Err(MemberRevocationSeedFailure::Admission(
                        MemberRevocationAdmissionError::DifferentOriginal { prepared }
                    ));
                }
            } else {
                gate.verified_seeds.insert(input.request_selector, retained.clone());
            }
        }
        // The effect admission independently rechecks the SAME closing gate.
        // If shutdown crossed this interval, the full prepared rejection is
        // retained by this seed; the verified winner is never reset or retried.
        self.admit(prepared).map_err(MemberRevocationSeedFailure::Admission)
    }
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl SurrealStorage {
    async fn read_member_revocation_server_original(
        &self, gate: &CapturedAdministrationAuthority, operation: Uuid,
    ) -> Result<MemberRevocationServerOriginal, ResourceAuthorityError> {
        let binding = MemberRevocationLookup { original: RecordId::new("member_enrollments", operation.to_string()) };
        let rows: Vec<MemberRevocationServerOriginal> = self.with_record_user_scope(
            gate.original_scope.clone(), self.with_data_operation(move |db| Box::pin(async move {
                db.query_values_at(MEMBER_REVOCATION_SERVER_ORIGINAL, binding, 0).await
            })),
        ).await?;
        let mut rows = rows;
        if rows.len() != 1 { return Err(member_revocation_denied()); }
        Ok(rows.remove(0))
    }
    fn decode_member_revocation_server_original(
        &self, original: &MemberRevocationServerOriginal, operation: Uuid,
        workspace: &str, administrator_resource: &str,
    ) -> Result<enrollment::EnrollmentDescriptor, ResourceAuthorityError> {
        let bytes = hex::decode(&original.descriptor_hex).map_err(|_| member_revocation_denied())?;
        let descriptor = enrollment::decode_enrollment(&bytes).map_err(|_| member_revocation_denied())?;
        if enrollment::encode_enrollment(&descriptor).map_err(|_| member_revocation_denied())? != bytes
            || descriptor.operation_id() != operation || original.operation_id != operation.to_string()
            || original.descriptor_sha256 != hex::encode(Sha256::digest(&bytes))
            || original.token_digest != hex::encode(descriptor.token_digest())
            || descriptor.administrator_resource() != administrator_resource
            || descriptor.original_enroller().workspace_id() != workspace
        { return Err(member_revocation_denied()); }
        Ok(descriptor)
    }
    // HTTP retry/observation supplies only selectors. Canonical request and
    // descriptor are decoded here; the caller cannot supply private proof data.
    pub(crate) async fn observe_server_member_revocation(
        &self, registry: &crate::capabilities::CapabilityRegistry,
        credentials: RoleAuthorityAuthInput, request_selector: Uuid, operation: Uuid,
        workspace: &str, administrator_resource: &str,
    ) -> Result<MemberRevocationObserved, ResourceAuthorityError> {
        let gate = self.capture_current_administration(
            registry, credentials.clone(), enrollment::CommandKind::RevokeEnrollment,
            administrator_resource, workspace,
        ).await?;
        let original = self.read_member_revocation_server_original(&gate, operation).await?;
        let descriptor = self.decode_member_revocation_server_original(&original, operation, workspace, administrator_resource)?;
        let request_bytes = hex::decode(original.revocation_request_hex.as_deref().ok_or_else(member_revocation_denied)?)
            .map_err(|_| member_revocation_denied())?;
        let request = enrollment::decode_revocation(&request_bytes).map_err(|_| member_revocation_denied())?;
        if request.request_id() != request_selector || request.enrollment_operation_id() != operation
            || enrollment::encode_revocation(&request).map_err(|_| member_revocation_denied())? != request_bytes
        { return Err(member_revocation_denied()); }
        self.observe_member_revocation(registry, credentials, &request, &descriptor).await
    }
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl MemberRevocationSeedController {
    pub(crate) fn request_selector(&self) -> Uuid { self.seed.input.request_selector }
    pub(crate) fn original_deadline(&self) -> tokio::time::Instant { self.seed.deadline }
    pub(crate) async fn wait(&self) -> std::sync::Arc<MemberRevocationSeedResult> {
        let mut state = self.state.lock().await;
        if let Some(outcome) = &state.outcome { return outcome.clone(); }
        let outcome = match state.worker.as_mut() {
            Some(worker) => match worker.await {
                Ok(outcome) => outcome,
                Err(error) => Err(MemberRevocationSeedFailure::Join(error)),
            },
            None => Err(MemberRevocationSeedFailure::Preparation(member_revocation_denied())),
        };
        state.worker.take();
        let outcome = std::sync::Arc::new(outcome);
        state.outcome = Some(outcome.clone());
        outcome
    }
}
// Close -> join/retain every seed -> join/retain frozen effects -> storeclose.
// No preparation failure loses the seed/password/credentials/raw error. There
// is no process-global registry, caller-proof DTO or fresh budget on redelivery.

#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
pub(crate) struct MemberRevocationMetadata {
    operation_id: Uuid,
    status: String,
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl MemberRevocationMetadata {
    pub(crate) fn operation_id(&self) -> Uuid { self.operation_id }
    pub(crate) fn status(&self) -> &str { &self.status }
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl SurrealStorage {
    // Required before publishing pending/cached seed state to an HTTP caller.
    // This current metadata read grants no mutation or secret-delivery authority.
    pub(crate) async fn current_member_revocation_metadata(
        &self, registry: &crate::capabilities::CapabilityRegistry,
        credentials: RoleAuthorityAuthInput, operation: Uuid,
        workspace: &str, administrator_resource: &str,
    ) -> Result<MemberRevocationMetadata, ResourceAuthorityError> {
        let gate = self.capture_current_administration(
            registry, credentials, enrollment::CommandKind::RevokeEnrollment,
            administrator_resource, workspace,
        ).await?;
        let original = self.read_member_revocation_server_original(&gate, operation).await?;
        self.decode_member_revocation_server_original(&original, operation, workspace, administrator_resource)?;
        if !matches!(original.status.as_str(), "active" | "revoked" | "consumed") {
            return Err(member_revocation_denied());
        }
        Ok(MemberRevocationMetadata { operation_id: operation, status: original.status })
    }
}


#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl MemberRevocationAdmission {
    fn retain_seed_rejection(&mut self, error: MemberRevocationSeedAdmissionError)
        -> std::sync::Arc<MemberRevocationSeedAdmissionError>
    {
        let retained = std::sync::Arc::new(error);
        self.rejected_seeds.push(retained.clone());
        retained
    }
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl MemberRevocationRuntime {
    pub(crate) fn retained_seed_rejections(&self)
        -> Vec<std::sync::Arc<MemberRevocationSeedAdmissionError>>
    {
        self.admission.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
            .rejected_seeds.clone()
    }
}
// Current metadata and terminal observation return WHOLE Results. The existing
// Core-owned HTTP runtime must retain those request frames and raw Results
// before projecting a constant public response; no second metadata registry is
// introduced here. This runtime retains every synchronous seed rejection Arc,
// including rejected raw credentials/password/selectors, before returning it.


// CODE candidate for storage/surreal/resource_authority.rs. No source adoption.
// Reuses exact fe2b controller codec/row types, 4ecf capture/parent/lifetime,
// c03d namespace helper and member_enrollment crypto-owned 4fc7.
// GET is metadata only. Each secret publication performs fresh current gates.
// The route owns this frame and its original absolute request deadline.
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
mod member_invitation_issuer_delivery {
    use super::*;
    use crate::model_runtime::cloud::member_enrollment as invitation;
    use crate::model_runtime::cloud::secrets_vault::OsKeychainSecretsVault;
    use std::sync::{Arc, OnceLock};

    pub(crate) struct MemberInvitationMetadata {
        pub(crate) operation_id: uuid::Uuid,
        pub(crate) account_id: String,
        pub(crate) principal_id: String,
        pub(crate) access_space_id: String,
        pub(crate) account_name: String,
        pub(crate) status: String,
        pub(crate) vault_state: String,
        pub(crate) descriptor: Option<enrollment::EnrollmentDescriptor>,
    }

    #[derive(Clone, PartialEq, Eq, SurrealValue)]
    struct IssuerOriginal {
        original: surrealdb::types::Object,
        id: RecordId,
        operation_id: String,
        original_request_hex: String,
        original_request_sha256: String,
        vault_namespace: String,
        vault_reference: String,
        vault_issue_revision: i64,
        vault_issue_state: String,
        descriptor_hex: Option<String>,
        descriptor_sha256: Option<String>,
        token_digest: Option<String>,
        account_id: RecordId,
        principal_id: RecordId,
        access_space_id: RecordId,
        account_name: String,
        purpose: String,
        issued_at: Datetime,
        expires_at: Datetime,
        status: String,
        account_status: Option<String>,
        principal_status: Option<String>,
        space_status: Option<String>,
        account_origin: Option<RecordId>,
        principal_origin: Option<RecordId>,
        space_origin: Option<RecordId>,
    }
    #[derive(Clone, SurrealValue)]
    struct Lookup { original: RecordId }
    const READ: &str = "SELECT $this AS original, id, operation_id, original_request_hex,
        original_request_sha256, vault_namespace, vault_reference, vault_issue_revision,
        vault_issue_state, descriptor_hex, descriptor_sha256, token_digest, account_id,
        principal_id, access_space_id, account_name, purpose, issued_at, expires_at, status,
        account_id.status AS account_status, principal_id.status AS principal_status,
        access_space_id.status AS space_status, account_id.member_enrollment_origin AS account_origin,
        principal_id.member_enrollment_origin AS principal_origin, access_space_id.member_enrollment_origin AS space_origin
        FROM $original;";

    struct CurrentIssuer {
        enroll: CapturedAdministrationAuthority,
        assign: CapturedAdministrationAuthority,
        lifetime: CapturedAdministrationLifetime,
        parent: MemberAdministrationParent,
    }
    fn denied() -> invitation::AccountError {
        original_account_fault("current invitation issuer denied")
    }
    fn live(deadline: tokio::time::Instant) -> Result<(), invitation::AccountError> {
        if tokio::time::Instant::now() >= deadline { return Err(denied()); }
        Ok(())
    }

    impl IssuerOriginal {
        fn decode(
            &self, operation: uuid::Uuid, namespace: &str,
        ) -> Result<(invitation::OriginalIssueIntent, Option<enrollment::EnrollmentDescriptor>), invitation::AccountError> {
            let bytes = hex::decode(&self.original_request_hex)?;
            let intent = decode_original_issue_request(&bytes)?;
            if operation.is_nil() || intent.operation_id != operation
                || self.id != RecordId::new("member_enrollments", operation.to_string())
                || self.operation_id != operation.to_string()
                || self.original_request_sha256 != sha256_hex(&bytes)
                || self.vault_namespace != namespace || intent.vault_namespace != namespace
                || self.vault_reference != intent.vault_reference
                || intent.vault_reference != format!("invitation/identity_enrollment/{}", operation.hyphenated())
                || self.account_id != RecordId::new("local_accounts", intent.staged.account_id())
                || self.principal_id != RecordId::new("principals", intent.staged.principal_id())
                || self.access_space_id != RecordId::new("access_spaces", intent.staged.access_space_id())
                || self.account_name != intent.account_name || self.purpose != intent.purpose
                || self.purpose != "identity_enrollment"
                || enrollment_time(&self.issued_at)? != intent.issued_at
                || enrollment_time(&self.expires_at)? != intent.expires_at
            { return Err(denied()); }
            let descriptor = match (&self.descriptor_hex, &self.descriptor_sha256, &self.token_digest) {
                (None, None, None) if self.status == "preparing"
                    && ((self.vault_issue_state == "reserved" && self.vault_issue_revision == 1)
                    || (self.vault_issue_state == "preparing" && self.vault_issue_revision == 2)) => None,
                (Some(encoded), Some(hash), Some(digest)) => {
                    let encoded = hex::decode(encoded)?;
                    let descriptor = enrollment::decode_enrollment(&encoded)?;
                    if enrollment::encode_enrollment(&descriptor)? != encoded
                        || !original_descriptor_matches(&intent, &descriptor)
                        || *hash != sha256_hex(&encoded)
                        || *digest != hex::encode(descriptor.token_digest())
                        || !((self.vault_issue_state == "write_started" && self.vault_issue_revision == 3
                                && self.status == "preparing")
                            || (self.vault_issue_state == "bound" && self.vault_issue_revision == 4
                                && ["preparing", "active", "consumed", "revoked"].contains(&self.status.as_str())))
                    { return Err(denied()); }
                    Some(descriptor)
                }
                _ => return Err(denied()),
            };
            Ok((intent, descriptor))
        }
        fn metadata(&self, descriptor: Option<enrollment::EnrollmentDescriptor>) -> Result<MemberInvitationMetadata, invitation::AccountError> {
            Ok(MemberInvitationMetadata {
                operation_id: uuid::Uuid::parse_str(&self.operation_id)?,
                account_id: record_key(self.account_id.clone())?,
                principal_id: record_key(self.principal_id.clone())?,
                access_space_id: record_key(self.access_space_id.clone())?,
                account_name: self.account_name.clone(), status: self.status.clone(),
                vault_state: self.vault_issue_state.clone(), descriptor,
            })
        }
    }

    impl SurrealStorage {
        // Authenticated RECORD query, never an admin query or invented scope.
        // The first read obtains only navigation data internally. Nothing is
        // published until the actual current dual-action capture succeeds.
        async fn issuer_original_read(
            &self, operation: uuid::Uuid, credentials: &RoleAuthorityAuthInput,
            deadline: tokio::time::Instant,
        ) -> Result<IssuerOriginal, invitation::AccountError> {
            use surrealdb::opt::auth::Record;
            live(deadline)?;
            let namespace = self.config().namespace().to_owned();
            let database = self.config().database().to_owned();
            let token_hash = hex::encode(Sha256::digest(credentials.session_token.as_bytes()));
            let channel_binding_hash = Some(credentials.channel_binding_hash.clone());
            let lookup = Lookup { original: RecordId::new("member_enrollments", operation.to_string()) };
            let rows: Vec<IssuerOriginal> = tokio::time::timeout_at(deadline,
                self.with_lease(move |client| Box::pin(async move {
                    let ordinary = client.clone();
                    ordinary.use_ns(namespace.clone()).use_db(database.clone()).await?;
                    ordinary.signin(Record {
                        namespace, database, access: AUTHORITY_ACCESS_METHOD.to_owned(),
                        params: SigninParams { token_hash, channel_binding_hash },
                    }).await?;
                    let mut response = ordinary.query(READ).bind(lookup.into_value()).await?.check()?;
                    response.take(0).map_err(Into::into)
                })),
            ).await.map_err(|error| Box::new(error) as invitation::AccountError)??;
            live(deadline)?;
            let mut rows = rows;
            if rows.len() != 1 { return Err(denied()); }
            rows.pop().ok_or_else(denied)
        }

        async fn issuer_current_capture(
            &self, registry: &crate::capabilities::CapabilityRegistry,
            credentials: &RoleAuthorityAuthInput, intent: &invitation::OriginalIssueIntent,
            descriptor: Option<&enrollment::EnrollmentDescriptor>, deadline: tokio::time::Instant,
        ) -> Result<CurrentIssuer, invitation::AccountError> {
            live(deadline)?;
            let enroll = self.capture_current_administration(registry, credentials.clone(),
                enrollment::CommandKind::Enroll, &intent.administrator_resource,
                intent.original_enroller.workspace_id()).await?;
            live(deadline)?;
            let assign = self.capture_current_administration(registry, credentials.clone(),
                enrollment::CommandKind::AssignFixedMemberProfile, &intent.administrator_resource,
                intent.original_enroller.workspace_id()).await?;
            let original_actor = &intent.original_enroller;
            for gate in [&enroll, &assign] {
                if gate.exact.owner_account_id() != original_actor.owner_account_id()
                    || gate.exact.actor_principal_id() != original_actor.actor_principal_id()
                    || gate.exact.access_space_id() != original_actor.access_space_id()
                    || gate.exact.workspace_id() != original_actor.workspace_id()
                { return Err(denied()); }
            }
            if enroll.exact != assign.exact { return Err(denied()); }
            let lifetime = CapturedAdministrationLifetime {
                administration_resource_id: enroll.facts.resource_id.clone(),
                policy_version: *enroll.facts.policy_versions.get(1).ok_or_else(denied)?,
                choice: enroll.facts.administration_lifetime.as_ref().ok_or_else(denied)?.closed_choice()?,
            };
            for gate in [&enroll, &assign] {
                lifetime.require_matches_source(&gate.facts)?;
                if gate.facts.administration_lifetime.as_ref().ok_or_else(denied)?.closed_choice()? != lifetime.choice {
                    return Err(denied());
                }
            }
            if let Some(descriptor) = descriptor { lifetime.require_preserved_expiry(descriptor)?; }
            else if lifetime.source030_bound().is_some_and(|expiry| intent.expires_at > expiry) { return Err(denied()); }
            live(deadline)?;
            let binding = MemberAdministrationParentBindings { resource: enroll.facts.resource_id.clone() };
            let parents: Vec<MemberAdministrationParent> = self.with_record_user_scope(
                enroll.original_scope.clone(), self.with_data_operation(move |db| Box::pin(async move {
                    db.query_values_at(ORIGINAL_ACCOUNT_PARENT_SQL, binding, 0).await
                })),
            ).await?;
            let [parent] = parents.as_slice() else { return Err(denied()); };
            if parent.administration != enroll.facts.resource_id || parent.owner != enroll.facts.account_id
                || parent.space != enroll.facts.access_space_id || parent.parent_owner != parent.owner
                || parent.parent_space != parent.space || parent.parent_kind != "workspace"
                || parent.parent_state != "active" || parent.parent_external != original_actor.workspace_id()
                || parent.resource_policy != lifetime.policy_version
                || enroll.facts.policy_versions.first().is_none_or(|value| parent.parent_policy > *value)
                || assign.facts.policy_versions.first().is_none_or(|value| parent.parent_policy > *value)
            { return Err(denied()); }
            live(deadline)?;
            Ok(CurrentIssuer { enroll, assign, lifetime, parent: parent.clone() })
        }

        async fn issuer_current_original(
            &self, operation: uuid::Uuid, registry: &crate::capabilities::CapabilityRegistry,
            credentials: &RoleAuthorityAuthInput, deadline: tokio::time::Instant,
        ) -> Result<(IssuerOriginal, Option<enrollment::EnrollmentDescriptor>), invitation::AccountError> {
            live(deadline)?;
            SurrealStorage::with_operation_deadline(deadline.into_std(), async {
                let row = self.issuer_original_read(operation, credentials, deadline).await?;
                let (intent, descriptor) = row.decode(operation, &self.member_invitation_vault_namespace()?)?;
                let first = self.issuer_current_capture(registry, credentials, &intent, descriptor.as_ref(), deadline).await?;
                let reread = self.issuer_original_read(operation, credentials, deadline).await?;
                if reread != row { return Err(denied()); }
                let second = self.issuer_current_capture(registry, credentials, &intent, descriptor.as_ref(), deadline).await?;
                require_precommit_administration_pair(&first.enroll, &first.assign, &first.lifetime,
                    &second.enroll.facts, &second.assign.facts, &second.lifetime)?;
                if second.parent != first.parent || second.enroll.exact != first.enroll.exact
                    || second.assign.exact != first.assign.exact { return Err(denied()); }
                live(deadline)?;
                Ok((row, descriptor))
            }).await
        }

        pub(crate) async fn current_member_invitation_metadata(
            &self, operation: uuid::Uuid, registry: Arc<crate::capabilities::CapabilityRegistry>,
            credentials: RoleAuthorityAuthInput, deadline: tokio::time::Instant,
        ) -> Result<MemberInvitationMetadata, invitation::AccountError> {
            let (row, descriptor) = self.issuer_current_original(operation, &registry, &credentials, deadline).await?;
            row.metadata(descriptor)
        }
    }

    // Private retained result. It has no public plaintext getter and cannot be
    // manufactured from a Ticket, Bound receipt, password counter or role flag.
    struct Recovered {
        row: IssuerOriginal,
        descriptor: enrollment::EnrollmentDescriptor,
        token: zeroize::Zeroizing<String>,
    }
    pub(crate) struct MemberInvitationDeliveryFrame {
        storage: SurrealStorage,
        registry: Arc<crate::capabilities::CapabilityRegistry>,
        credentials: RoleAuthorityAuthInput,
        operation: uuid::Uuid,
        deadline: tokio::time::Instant,
        result: OnceLock<Result<Recovered, invitation::AccountError>>,
        current_errors: std::sync::Mutex<Vec<Arc<invitation::AccountError>>>,
        worker: tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>,
        joined: OnceLock<Result<(), Arc<tokio::task::JoinError>>>,
    }
    pub(crate) struct MemberInvitationDelivery {
        token: zeroize::Zeroizing<String>,
        descriptor: enrollment::EnrollmentDescriptor,
    }
    impl MemberInvitationDelivery {
        pub(crate) fn recipient_token(&self) -> &str { self.token.as_str() }
        pub(crate) fn descriptor(&self) -> &enrollment::EnrollmentDescriptor { &self.descriptor }
    }
    impl MemberInvitationDeliveryFrame {
        fn retain_current_error(&self, error: invitation::AccountError) -> DeliveryDenied<'_> {
            let error = Arc::new(error);
            // Recover the collection after poisoning without losing the raw
            // incoming error or any earlier retained outcome.
            let mut errors = self.current_errors.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            errors.push(error.clone());
            DeliveryDenied::Current(error)
        }
        // Call once at the actual route's retained request boundary. This creates
        // no operation ID, secret, descriptor, grant or replacement deadline.
        pub(crate) fn start(
            storage: &SurrealStorage, registry: Arc<crate::capabilities::CapabilityRegistry>,
            credentials: RoleAuthorityAuthInput, operation: uuid::Uuid, deadline: tokio::time::Instant,
        ) -> Result<Arc<Self>, invitation::AccountError> {
            live(deadline)?;
            if operation.is_nil() { return Err(denied()); }
            let frame = Arc::new(Self {
                storage: storage.clone(), registry, credentials, operation, deadline,
                result: OnceLock::new(), current_errors: std::sync::Mutex::new(Vec::new()),
                worker: tokio::sync::Mutex::new(None), joined: OnceLock::new(),
            });
            let owned = frame.clone();
            let worker = tokio::spawn(async move {
                let outcome = owned.recover().await;
                // A closed original result is never replaced or retried.
                let _ = owned.result.set(outcome);
            });
            // Before start returns only this function and recover hold frame;
            // recover never touches the handle slot. Publication is synchronous.
            *frame.worker.try_lock().expect("unpublished invitation worker slot") = Some(worker);
            Ok(frame)
        }
        pub(crate) fn matches_request(
            &self, storage: &SurrealStorage, credentials: &RoleAuthorityAuthInput,
            operation: uuid::Uuid, deadline: tokio::time::Instant,
        ) -> bool {
            Arc::ptr_eq(&self.storage.inner, &storage.inner) && self.operation == operation
                && self.deadline == deadline && self.credentials.session_token == credentials.session_token
                && self.credentials.channel_binding_hash == credentials.channel_binding_hash
        }
        async fn recover(&self) -> Result<Recovered, invitation::AccountError> {
            let (row, descriptor) = self.storage.issuer_current_original(
                self.operation, &self.registry, &self.credentials, self.deadline,
            ).await?;
            let descriptor = descriptor.ok_or_else(denied)?;
            if row.vault_issue_state != "bound" || row.vault_issue_revision != 4 || row.status != "active"
                || row.account_status.as_deref() != Some("disabled") || row.principal_status.as_deref() != Some("disabled") || row.space_status.as_deref() != Some("disabled")
                || row.account_origin.as_ref() != Some(&row.id) || row.principal_origin.as_ref() != Some(&row.id) || row.space_origin.as_ref() != Some(&row.id) {
                return Err(denied());
            }
            let crypto_row = invitation::OriginalInvitationRow {
                operation_id: self.operation,
                descriptor_hex: row.descriptor_hex.as_deref().ok_or_else(denied)?,
                descriptor_sha256: row.descriptor_sha256.as_deref().ok_or_else(denied)?,
                account_id: descriptor.staged().account_id(),
                principal_id: descriptor.staged().principal_id(),
                access_space_id: descriptor.staged().access_space_id(),
                account_name: &row.account_name,
                token_digest_hex: row.token_digest.as_deref().ok_or_else(denied)?,
                purpose: &row.purpose, issued_at: enrollment_time(&row.issued_at)?,
                expires_at: enrollment_time(&row.expires_at)?, status: &row.status,
            };
            let binding = invitation::bind_original(&descriptor, &crypto_row, &row.vault_reference)?;
            let vault = OsKeychainSecretsVault::new(row.vault_namespace.clone());
            live(self.deadline)?;
            let recovered = invitation::recover_existing(&vault, &binding)
                .map_err(|error| Box::new(RecoveryFault(error)) as invitation::AccountError)?;
            // Whole secret remains zeroizing and privately retained. No get
            // failure can invoke put, delete, reserve, entropy or a new original.
            let token = zeroize::Zeroizing::new(recovered.recipient_token().to_owned());
            drop(recovered);
            drop(binding);
            live(self.deadline)?;
            Ok(Recovered { row, descriptor, token })
        }
        // Route shutdown drains THIS same handle. Cancellation while awaiting
        // releases only the mutex guard; the borrowed handle stays in its slot.
        // There is no timeout, abort, replacement task or result flattening here.
        pub(crate) async fn drain_worker(&self) -> &Result<(), Arc<tokio::task::JoinError>> {
            let mut slot = self.worker.lock().await;
            if self.joined.get().is_none() {
                let handle = slot.as_mut().expect("retained invitation worker handle");
                let outcome = handle.await.map_err(Arc::new);
                let _ = self.joined.set(outcome);
                // The actual raw join outcome is retained BEFORE removal.
                drop(slot.take());
            }
            self.joined.get().expect("joined invitation worker outcome")
        }
        pub(crate) async fn deliver(&self) -> Result<MemberInvitationDelivery, DeliveryDenied<'_>> {
            tokio::select! {
                outcome = self.drain_worker() => {
                    if let Err(error) = outcome { return Err(DeliveryDenied::Worker(error.as_ref())); }
                },
                _ = tokio::time::sleep_until(self.deadline) => return Err(self.retain_current_error(denied())),
            }
            let result = self.result.get().ok_or_else(|| self.retain_current_error(denied()))?;
            let recovered = result.as_ref().map_err(DeliveryDenied::Retained)?;
            let (current, descriptor) = self.storage.issuer_current_original(
                self.operation, &self.registry, &self.credentials, self.deadline,
            ).await.map_err(|error| self.retain_current_error(error))?;
            if current != recovered.row || descriptor.as_ref() != Some(&recovered.descriptor) {
                return Err(self.retain_current_error(denied()));
            }
            let now = enrollment_time(&Datetime::from(Utc::now()))
                .map_err(|error| self.retain_current_error(Box::new(error)))?;
            if now < recovered.descriptor.issued_at() || now >= recovered.descriptor.expires_at() {
                return Err(self.retain_current_error(denied()));
            }
            live(self.deadline).map_err(|error| self.retain_current_error(error))?;
            Ok(MemberInvitationDelivery {
                token: zeroize::Zeroizing::new(recovered.token.as_str().to_owned()),
                descriptor: recovered.descriptor.clone(),
            })
        }
    }
    pub(crate) enum DeliveryDenied<'a> {
        Retained(&'a invitation::AccountError),
        Current(Arc<invitation::AccountError>),
        Worker(&'a tokio::task::JoinError),
    }
    impl DeliveryDenied<'_> {
        // Borrow the real retained error for internal handling; an HTTP
        // constant projection never consumes or replaces its frame-owned copy.
        pub(crate) fn error(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
            match self {
                Self::Retained(error) => error.as_ref(),
                Self::Current(error) => error.as_ref().as_ref(),
                Self::Worker(error) => *error,
            }
        }
    }
    #[derive(Debug)]
    struct RecoveryFault(invitation::SecretRecoveryError);
    impl std::fmt::Display for RecoveryFault {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("invitation recovery denied") }
    }
    impl std::error::Error for RecoveryFault {}
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
pub(crate) use member_invitation_issuer_delivery::{
    MemberInvitationMetadata, MemberInvitationDeliveryFrame, MemberInvitationDelivery, DeliveryDenied,
};


// External candidate for resource_authority.rs. No source adoption or runtime proof.
// Uses the existing configured normal-login bootstrap authority, not recipient RLS.
#[derive(Clone, PartialEq, Eq, SurrealValue)]
struct MemberPasswordLoginBinding {
    enrollment: RecordId,
    operation_id: String,
    descriptor_hex: String,
    descriptor_sha256: String,
    redemption_attempt_id: String,
    consumed_at: Datetime,
    account: RecordId,
    principal: RecordId,
    space: RecordId,
    name: String,
    credential_revision: i64,
    account_epoch: i64,
    principal_epoch: i64,
    space_epoch: i64,
    account_policy: i64,
    principal_policy: i64,
    space_policy: i64,
    profile: String,
    capabilities: Vec<String>,
}
#[derive(SurrealValue)]
struct MemberPasswordLoginQuery {
    snapshot: super::local_accounts::MemberPasswordCredentialSnapshot,
    principal: RecordId,
    space: RecordId,
}
const MEMBER_PASSWORD_LOGIN_BINDING: &str = "
SELECT id AS enrollment, operation_id, descriptor_hex, descriptor_sha256,
 redemption_attempt_id, consumed_at, account_id AS account, principal_id AS principal,
 access_space_id AS space, account_name AS name,
 account_id.credential_revision AS credential_revision,
 account_id.revocation_epoch AS account_epoch, principal_id.revocation_epoch AS principal_epoch,
 access_space_id.revocation_epoch AS space_epoch,
 account_id.policy_version AS account_policy, principal_id.policy_version AS principal_policy,
 access_space_id.policy_version AS space_policy, principal_id.capability_profile_id AS profile,
 principal_id.delegated_capabilities AS capabilities
FROM member_enrollments
WHERE account_id = $snapshot.account AND principal_id = $principal AND access_space_id = $space
 AND account_name = $snapshot.name AND purpose = 'identity_enrollment' AND status = 'consumed'
 AND password_verifier != NONE AND redemption_attempt_id != NONE AND consumed_at != NONE
 AND account_id.account_key = account_name AND account_id.account_role = 'Member'
 AND account_id.status = 'enabled' AND account_id.password_verifier = $snapshot.verifier
 AND account_id.credential_revision = $snapshot.credential_revision
 AND account_id.revocation_epoch = $snapshot.revocation_epoch AND account_id.policy_version = $snapshot.policy_version
 AND principal_id.account_id = account_id AND principal_id.principal_kind = 'human_account'
 AND principal_id.status = 'enabled' AND principal_id.capability_profile_id = 'MemberEnrollment'
 AND principal_id.delegated_capabilities = ['model_lane.navigation.read', 'fs.write', 'diagnostics.read']
 AND access_space_id.account_id = account_id AND access_space_id.status = 'active' LIMIT 2;
";
#[derive(SurrealValue)]
struct MemberPasswordLoginWrite {
    binding: MemberPasswordLoginBinding,
    verifier: String,
    credential: RecordId,
    token_hash: String,
    expires_at: Datetime,
}
const MEMBER_PASSWORD_LOGIN_WRITE: &str = "
LET $a = UPDATE $binding.account SET auth_transition_nonce += 1
 WHERE account_key = $binding.name AND account_role = 'Member' AND status = 'enabled'
 AND password_verifier = $verifier AND credential_revision = $binding.credential_revision
 AND revocation_epoch = $binding.account_epoch AND policy_version = $binding.account_policy RETURN AFTER;
IF array::len($a) != 1 { THROW 'HSK-MEMBER-LOGIN-DENIED'; };
LET $p = UPDATE $binding.principal SET auth_transition_nonce += 1
 WHERE account_id = $binding.account AND principal_kind = 'human_account' AND status = 'enabled'
 AND revocation_epoch = $binding.principal_epoch AND policy_version = $binding.principal_policy
 AND capability_profile_id = $binding.profile AND delegated_capabilities = $binding.capabilities RETURN AFTER;
IF array::len($p) != 1 { THROW 'HSK-MEMBER-LOGIN-DENIED'; };
LET $s = UPDATE $binding.space SET auth_transition_nonce += 1
 WHERE account_id = $binding.account AND status = 'active'
 AND revocation_epoch = $binding.space_epoch AND policy_version = $binding.space_policy RETURN AFTER;
IF array::len($s) != 1 { THROW 'HSK-MEMBER-LOGIN-DENIED'; };
LET $m = SELECT VALUE id FROM member_enrollments WHERE id = $binding.enrollment
 AND operation_id = $binding.operation_id AND descriptor_hex = $binding.descriptor_hex
 AND descriptor_sha256 = $binding.descriptor_sha256 AND account_id = $binding.account
 AND principal_id = $binding.principal AND access_space_id = $binding.space
 AND account_name = $binding.name AND purpose = 'identity_enrollment' AND status = 'consumed'
 AND redemption_attempt_id = $binding.redemption_attempt_id AND consumed_at = $binding.consumed_at
 AND password_verifier != NONE LIMIT 2;
IF array::len($m) != 1 OR record::exists($credential) OR $expires_at <= time::now()
 { THROW 'HSK-MEMBER-LOGIN-DENIED'; };
CREATE $credential SET account_id = $binding.account, principal_id = $binding.principal,
 access_space_id = $binding.space, token_hash = $token_hash, status = 'active',
 expires_at = $expires_at, revoked_at = NONE, created_at = time::now(), member_login_binding = $binding;
";
const MEMBER_PASSWORD_LOGIN_READBACK: &str = "
RETURN record::exists($credential)
 AND $credential.account_id = $binding.account AND $credential.principal_id = $binding.principal
 AND $credential.access_space_id = $binding.space AND $credential.token_hash = $token_hash
 AND $credential.status = 'active' AND $credential.revoked_at = NONE
 AND $credential.expires_at = $expires_at AND $credential.expires_at > time::now()
 AND $credential.member_login_binding = $binding;
";

struct MemberPasswordLoginAttempt {
    storage: SurrealStorage,
    checked: super::local_accounts::PasswordVerifiedLocalAccount,
    credential: RecordId,
    token: zeroize::Zeroizing<String>,
    token_hash: String,
    expires_at: DateTime<Utc>,
    binding: std::sync::OnceLock<MemberPasswordLoginBinding>,
}
enum MemberPasswordLoginFailure {
    Operation(ResourceAuthorityError),
    Cancel {
        operation: ResourceAuthorityError,
        cancellation: Result<super::SurrealClient, surrealdb::Error>,
    },
    Commit(surrealdb::Error),
    CanonicalRead(SurrealStorageError),
    CanonicalMismatch,
}
enum MemberPasswordLoginObservation {
    Complete(std::sync::Arc<Result<IssuedSessionCredential, MemberPasswordLoginFailure>>),
    Join(std::sync::Arc<tokio::task::JoinError>),
}
enum MemberPasswordLoginWorkerState {
    Starting,
    Running(tokio::task::JoinHandle<Result<IssuedSessionCredential, MemberPasswordLoginFailure>>),
    Observed(MemberPasswordLoginObservation),
}
pub(super) struct MemberPasswordLoginOperation {
    original: std::sync::Arc<MemberPasswordLoginAttempt>,
    state: tokio::sync::Mutex<MemberPasswordLoginWorkerState>,
}
impl MemberPasswordLoginOperation {
    async fn observe(&self) -> Result<IssuedSessionCredential, ResourceAuthorityError> {
        let mut state = self.state.lock().await;
        if let MemberPasswordLoginWorkerState::Running(worker) = &mut *state {
            // Borrow the retained handle. Cancelling a waiter cannot detach the worker.
            let observed = match worker.await {
                Ok(result) => MemberPasswordLoginObservation::Complete(std::sync::Arc::new(result)),
                Err(error) => MemberPasswordLoginObservation::Join(std::sync::Arc::new(error)),
            };
            *state = MemberPasswordLoginWorkerState::Observed(observed);
        }
        match &*state {
            MemberPasswordLoginWorkerState::Observed(MemberPasswordLoginObservation::Complete(result)) => {
                match result.as_ref() {
                    Ok(credential) => Ok(credential.clone()),
                    Err(_) => Err(SurrealStorageError::TransactionSettlementPending {
                        operation_id: Uuid::parse_str(&record_key(self.original.credential.clone())?)
                            .map_err(|_| ResourceAuthorityError::InvalidInput("invalid credential operation"))?,
                    }.into()),
                }
            }
            _ => Err(SurrealStorageError::TransactionSettlementPending {
                operation_id: Uuid::parse_str(&record_key(self.original.credential.clone())?)
                    .map_err(|_| ResourceAuthorityError::InvalidInput("invalid credential operation"))?,
            }.into()),
        }
    }
}
impl SurrealStorage {
    pub(super) async fn provision_verified_member_session_credential(
        &self,
        checked: super::local_accounts::PasswordVerifiedLocalAccount,
        ttl: Duration,
    ) -> Result<IssuedSessionCredential, ResourceAuthorityError> {
        // Existing normal login policy only. The caller cannot extend a password proof.
        if ttl != Duration::from_secs(60) {
            return Err(ResourceAuthorityError::InvalidInput("invalid login credential lifetime"));
        }
        checked.credential_snapshot(self)?;
        let mut entropy = zeroize::Zeroizing::new([0u8; 32]);
        getrandom::getrandom(&mut entropy[..])
            .map_err(|error| ResourceAuthorityError::Entropy(error.to_string()))?;
        let token = zeroize::Zeroizing::new(hex::encode(entropy.as_ref()));
        let token_hash = sha256_hex(token.as_bytes());
        let operation_id = Uuid::now_v7();
        let original = std::sync::Arc::new(MemberPasswordLoginAttempt {
            storage: self.clone(), checked,
            credential: RecordId::new("session_exchange_credentials", operation_id.to_string()),
            token, token_hash, expires_at: Utc::now() + chrono::Duration::seconds(60),
            binding: std::sync::OnceLock::new(),
        });
        let operation = std::sync::Arc::new(MemberPasswordLoginOperation {
            original: original.clone(),
            state: tokio::sync::Mutex::new(MemberPasswordLoginWorkerState::Starting),
        });
        // Add MemberPasswordLogin(Arc<resource_authority::MemberPasswordLoginOperation>)
        // to the existing parent AuthenticatedTransactionRetention enum.
        {
            let admission = self.inner.member_login_admission.lock()
                .map_err(|_| ResourceAuthorityError::InvalidInput("login admission poisoned"))?;
            if !*admission { return Err(SurrealStorageError::Closed.into()); }
            self.reserve_transaction_settlement(operation_id, Some(
                super::AuthenticatedTransactionRetention::MemberPasswordLogin(operation.clone())
            ))?;
            // No await between publishing the strong owner and publishing its worker.
            let mut state = operation.state.try_lock().map_err(|_| {
                ResourceAuthorityError::InvalidInput("unpublished login worker is locked")
            })?;
            let worker = tokio::spawn(async move { original.storage.clone().run_member_password_login(original).await });
            *state = MemberPasswordLoginWorkerState::Running(worker);
            drop(state);
            drop(admission);
        }
        let result = operation.observe().await;
        if result.is_ok() {
            let mut retained = self.inner.transaction_settlements.lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if matches!(retained.get(&operation_id),
                Some(Some(super::AuthenticatedTransactionRetention::MemberPasswordLogin(current)))
                if std::sync::Arc::ptr_eq(current, &operation))
            { retained.remove(&operation_id); }
        }
        result
    }

    async fn run_member_password_login(
        &self, original: std::sync::Arc<MemberPasswordLoginAttempt>,
    ) -> Result<IssuedSessionCredential, MemberPasswordLoginFailure> {
        let storage = self.clone();
        let namespace = self.config().namespace().to_owned();
        let database = self.config().database().to_owned();
        let retained = original.clone();
        let result = self.with_lease(move |client| Box::pin(async move {
            let authority = client.clone();
            authority.use_ns(namespace).use_db(database).await?;
            let transaction = authority.begin().await?;
            let body: Result<MemberPasswordLoginBinding, ResourceAuthorityError> = async {
                let tx = super::SurrealTransactionContext { transaction: &transaction };
                let identity = storage.resolve_redeemed_member_login_in_transaction(&tx, &retained.checked).await?;
                let snapshot = retained.checked.credential_snapshot(&storage)?;
                let mut response = transaction.query(MEMBER_PASSWORD_LOGIN_BINDING)
                    .bind(MemberPasswordLoginQuery {
                        snapshot, principal: RecordId::new("principals", identity.principal_id),
                        space: RecordId::new("access_spaces", identity.access_space_id),
                    }).await.map_err(SurrealStorageError::from)?.check().map_err(SurrealStorageError::from)?;
                let mut rows: Vec<MemberPasswordLoginBinding> = response.take(0).map_err(SurrealStorageError::from)?;
                if rows.len() != 1 {
                    return Err(ResourceAuthorityError::InvalidInput("authentication denied"));
                }
                let binding = rows.pop().ok_or(ResourceAuthorityError::InvalidInput("authentication denied"))?;
                if binding.credential_revision <= 0
                    || binding.account != RecordId::new("local_accounts", identity.account_id)
                { return Err(ResourceAuthorityError::InvalidInput("authentication denied")); }
                retained.binding.set(binding.clone())
                    .map_err(|_| ResourceAuthorityError::InvalidInput("login binding already captured"))?;
                let snapshot = retained.checked.credential_snapshot(&storage)?;
                transaction.query(MEMBER_PASSWORD_LOGIN_WRITE).bind(MemberPasswordLoginWrite {
                    binding: binding.clone(), verifier: snapshot.verifier,
                    credential: retained.credential.clone(), token_hash: retained.token_hash.clone(),
                    expires_at: Datetime::from(retained.expires_at),
                }).await.map_err(SurrealStorageError::from)?.check().map_err(SurrealStorageError::from)?;
                Ok(binding)
            }.await;
            let binding = match body {
                Ok(binding) => binding,
                Err(operation) => {
                    let cancellation = transaction.cancel().await;
                    return Ok(Err(MemberPasswordLoginFailure::Cancel { operation, cancellation }));
                }
            };
            let canonical = match transaction.commit().await {
                Ok(client) => client,
                Err(error) => return Ok(Err(MemberPasswordLoginFailure::Commit(error))),
            };
            let snapshot = match retained.checked.credential_snapshot(&storage) {
                Ok(snapshot) => snapshot,
                Err(error) => return Ok(Err(MemberPasswordLoginFailure::Operation(error))),
            };
            let readback: Result<bool, SurrealStorageError> = async {
                let mut response = canonical.query(MEMBER_PASSWORD_LOGIN_READBACK)
                    .bind(MemberPasswordLoginWrite {
                        binding: binding.clone(), verifier: snapshot.verifier,
                        credential: retained.credential.clone(), token_hash: retained.token_hash.clone(),
                        expires_at: Datetime::from(retained.expires_at),
                    }).await?.check()?;
                let observed: surrealdb::types::Value = response.take(0)?;
                Ok(matches!(observed, surrealdb::types::Value::Bool(true)))
            }.await;
            match readback {
                Ok(true) => Ok(Ok(IssuedSessionCredential {
                    credential_id: record_key(retained.credential.clone())
                        .map_err(|_| SurrealStorageError::TransactionRetentionMismatch {
                            operation_id: Uuid::nil(),
                        })?,
                    token: retained.token.to_string(),
                    account_id: record_key(binding.account).map_err(|_| SurrealStorageError::TransactionRetentionMismatch { operation_id: Uuid::nil() })?,
                    principal_id: record_key(binding.principal).map_err(|_| SurrealStorageError::TransactionRetentionMismatch { operation_id: Uuid::nil() })?,
                    access_space_id: record_key(binding.space).map_err(|_| SurrealStorageError::TransactionRetentionMismatch { operation_id: Uuid::nil() })?,
                    expires_at: retained.expires_at,
                })),
                Ok(false) => Ok(Err(MemberPasswordLoginFailure::CanonicalMismatch)),
                Err(error) => Ok(Err(MemberPasswordLoginFailure::CanonicalRead(error))),
            }
        })).await;
        match result {
            Ok(result) => result,
            Err(error) => Err(MemberPasswordLoginFailure::CanonicalRead(error)),
        }
    }
}
// Parent enum insertion is a typed retention owner only. Do not send this variant
// through transaction_worker: this private normal-login worker owns its SDK transaction.
// Storage shutdown must await retained MemberPasswordLoginOperation::observe before close.
// Schema: DEFINE FIELD OVERWRITE member_login_binding ON session_exchange_credentials
// FLEXIBLE TYPE option<object> PERMISSIONS NONE;
// No password verifier or raw password is persisted in member_login_binding.

impl SurrealStorage {
    pub(crate) async fn drain_member_password_login_operations(&self)
        -> Result<(), ResourceAuthorityError>
    {
        self.close_member_login_admission()?;
        let owners: Vec<_> = {
            let retained = self.inner.transaction_settlements.lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            retained.values().filter_map(|entry| match entry {
                Some(super::AuthenticatedTransactionRetention::MemberPasswordLogin(owner))
                    => Some(owner.clone()),
                _ => None,
            }).collect()
        };
        let administrator_owners: Vec<_> = {
            let retained = self.inner.transaction_settlements.lock().unwrap_or_else(|p| p.into_inner());
            retained.values().filter_map(|entry| match entry {
                Some(super::AuthenticatedTransactionRetention::AdministratorPasswordLogin(owner)) => Some(owner.clone()),
                _ => None,
            }).collect()
        };
        let (setup, workspace, confirmation) = tokio::join!(
            self.drain_first_owner_setup_operations(), self.drain_first_owner_workspace_operations(),
            self.drain_first_owner_administration_confirmation_operations(),
        );
        let mut first_error = setup.err().or(workspace.err()).or(confirmation.err());
        for owner in administrator_owners {
            if let Err(error) = owner.observe().await {
                if first_error.is_none() { first_error = Some(error); }
            }
        }
        for owner in owners {
            if let Err(error) = owner.observe().await {
                if first_error.is_none() { first_error = Some(error); }
            }
        }
        match first_error { Some(error) => Err(error), None => Ok(()) }
    }
}


// Additional resource_authority.rs candidate. Pair with private credential worker
// and member-password-login-schema. Existing Owner/Admin exchange is retained below.
#[derive(SurrealValue)]
struct MemberExchangeEligible {
    credential: RecordId,
    binding: MemberPasswordLoginBinding,
}
#[derive(Clone, SurrealValue)]
struct MemberExchangeQuery {
    account: RecordId,
    principal: RecordId,
    space: RecordId,
    credential_token_hash: String,
}
#[derive(SurrealValue)]
struct MemberExchangeWrite {
    input: MemberExchangeQuery,
    credential: RecordId,
    binding: MemberPasswordLoginBinding,
    session: RecordId,
    session_token_hash: String,
    channel: String,
    issued_at: Datetime,
    expires_at: Datetime,
    principal_key: String,
}
const MEMBER_EXCHANGE_ELIGIBLE: &str = "
SELECT id AS credential, member_login_binding AS binding FROM session_exchange_credentials
 WHERE account_id = $account AND principal_id = $principal AND access_space_id = $space
 AND token_hash = $credential_token_hash AND status = 'active' AND revoked_at = NONE
 AND expires_at > time::now() AND member_login_binding != NONE AND administrator_login_binding = NONE
 AND member_login_binding.account = account_id AND member_login_binding.principal = principal_id
 AND member_login_binding.space = access_space_id
 AND fn::wp012_member_login_binding_current(member_login_binding) LIMIT 2;
";
const MEMBER_EXCHANGE_WRITE: &str = "
IF !fn::wp012_member_login_binding_current($binding) { THROW 'HSK-MEMBER-EXCHANGE-DENIED'; };
LET $a = UPDATE $binding.account SET auth_transition_nonce += 1
 WHERE status = 'enabled' AND account_role = 'Member' AND account_key = $binding.name
 AND credential_revision = $binding.credential_revision AND password_verifier != NONE
 AND revocation_epoch = $binding.account_epoch AND policy_version = $binding.account_policy RETURN AFTER;
IF array::len($a) != 1 { THROW 'HSK-MEMBER-EXCHANGE-DENIED'; };
LET $p = UPDATE $binding.principal SET auth_transition_nonce += 1
 WHERE account_id = $binding.account AND status = 'enabled' AND principal_kind = 'human_account'
 AND revocation_epoch = $binding.principal_epoch AND policy_version = $binding.principal_policy
 AND capability_profile_id = $binding.profile AND delegated_capabilities = $binding.capabilities RETURN AFTER;
IF array::len($p) != 1 { THROW 'HSK-MEMBER-EXCHANGE-DENIED'; };
LET $s = UPDATE $binding.space SET auth_transition_nonce += 1
 WHERE account_id = $binding.account AND status = 'active'
 AND revocation_epoch = $binding.space_epoch AND policy_version = $binding.space_policy RETURN AFTER;
IF array::len($s) != 1 { THROW 'HSK-MEMBER-EXCHANGE-DENIED'; };
LET $used = UPDATE $credential SET status = 'consumed'
 WHERE account_id = $input.account AND principal_id = $input.principal AND access_space_id = $input.space
 AND token_hash = $input.credential_token_hash AND status = 'active' AND revoked_at = NONE
 AND expires_at > time::now() AND member_login_binding = $binding RETURN AFTER;
IF array::len($used) != 1 OR record::exists($session) OR $expires_at <= time::now()
 { THROW 'HSK-MEMBER-EXCHANGE-DENIED'; };
CREATE $session SET account_id = $binding.account, principal_id = $binding.principal,
 access_space_id = $binding.space, token_hash = $session_token_hash, channel_binding_hash = $channel,
 authentication_strength = 'credential_exchange', delegated_capabilities = $binding.capabilities,
 delegation_chain = [$principal_key], account_revocation_epoch = $binding.account_epoch,
 principal_revocation_epoch = $binding.principal_epoch, space_revocation_epoch = $binding.space_epoch,
 policy_version = math::max([$binding.account_policy, $binding.principal_policy, $binding.space_policy]),
 issued_at = $issued_at, expires_at = $expires_at, revoked_at = NONE;
";
const MEMBER_EXCHANGE_CANONICAL: &str = "
RETURN record::exists($session) AND $credential.status = 'consumed'
 AND $credential.member_login_binding = $binding AND $credential.token_hash = $input.credential_token_hash
 AND $session.account_id = $binding.account AND $session.principal_id = $binding.principal
 AND $session.access_space_id = $binding.space AND $session.token_hash = $session_token_hash
 AND $session.channel_binding_hash = $channel AND $session.authentication_strength = 'credential_exchange'
 AND $session.delegated_capabilities = $binding.capabilities AND $session.delegation_chain = [$principal_key]
 AND $session.account_revocation_epoch = $binding.account_epoch
 AND $session.principal_revocation_epoch = $binding.principal_epoch
 AND $session.space_revocation_epoch = $binding.space_epoch
 AND $session.policy_version = math::max([$binding.account_policy, $binding.principal_policy, $binding.space_policy])
 AND $session.issued_at = $issued_at AND $session.expires_at = $expires_at
 AND $session.expires_at > time::now() AND $session.revoked_at = NONE
 AND fn::wp012_member_login_binding_current($binding);
";
fn require_canonical_member_login_binding(
    binding: &MemberPasswordLoginBinding,
) -> Result<(), ResourceAuthorityError> {
    let bytes = hex::decode(&binding.descriptor_hex)
        .map_err(|_| ResourceAuthorityError::InvalidInput("authentication denied"))?;
    let descriptor = handshake_member_enrollment::command_plan::decode_enrollment(&bytes)
        .map_err(|_| ResourceAuthorityError::InvalidInput("authentication denied"))?;
    let staged = descriptor.staged();
    if sha256_hex(&bytes) != binding.descriptor_sha256
        || descriptor.operation_id().to_string() != binding.operation_id
        || descriptor.account_name() != binding.name
        || binding.enrollment != RecordId::new("member_enrollments", binding.operation_id.clone())
        || binding.account != RecordId::new("local_accounts", staged.account_id())
        || binding.principal != RecordId::new("principals", staged.principal_id())
        || binding.space != RecordId::new("access_spaces", staged.access_space_id())
        || binding.credential_revision <= 0
    { return Err(ResourceAuthorityError::InvalidInput("authentication denied")); }
    Ok(())
}
struct MemberSessionExchangeAttempt {
    storage: SurrealStorage,
    input: MemberExchangeQuery,
    session_id: Uuid,
    token: zeroize::Zeroizing<String>,
    token_hash: String,
    credential_token: zeroize::Zeroizing<String>,
    channel: String,
    issued_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    selected: std::sync::OnceLock<(RecordId, MemberPasswordLoginBinding)>,
}
enum MemberSessionExchangeFailure {
    Operation(ResourceAuthorityError),
    Cancel { operation: ResourceAuthorityError, cancellation: Result<super::SurrealClient, surrealdb::Error> },
    Commit(surrealdb::Error),
    Storage(SurrealStorageError),
    CanonicalMismatch,
}
enum MemberSessionExchangeObserved {
    Complete(std::sync::Arc<Result<IssuedSession, MemberSessionExchangeFailure>>),
    Join(std::sync::Arc<tokio::task::JoinError>),
}
enum MemberSessionExchangeState {
    Starting,
    Running(tokio::task::JoinHandle<Result<IssuedSession, MemberSessionExchangeFailure>>),
    Observed(MemberSessionExchangeObserved),
}
pub(super) struct MemberSessionExchangeOperation {
    original: std::sync::Arc<MemberSessionExchangeAttempt>,
    state: tokio::sync::Mutex<MemberSessionExchangeState>,
}
impl MemberSessionExchangeOperation {
    async fn observe(&self) -> Result<IssuedSession, ResourceAuthorityError> {
        let mut state = self.state.lock().await;
        if let MemberSessionExchangeState::Running(worker) = &mut *state {
            let observed = match worker.await {
                Ok(result) => MemberSessionExchangeObserved::Complete(std::sync::Arc::new(result)),
                Err(error) => MemberSessionExchangeObserved::Join(std::sync::Arc::new(error)),
            };
            *state = MemberSessionExchangeState::Observed(observed);
        }
        match &*state {
            MemberSessionExchangeState::Observed(MemberSessionExchangeObserved::Complete(result)) => {
                match result.as_ref() {
                    Ok(session) => Ok(session.clone()),
                    Err(_) => Err(SurrealStorageError::TransactionSettlementPending {
                        operation_id: self.original.session_id,
                    }.into()),
                }
            }
            _ => Err(SurrealStorageError::TransactionSettlementPending {
                operation_id: self.original.session_id,
            }.into()),
        }
    }
}
impl MemberSessionExchangeAttempt {
    fn write(&self, credential: RecordId, binding: MemberPasswordLoginBinding)
        -> Result<MemberExchangeWrite, ResourceAuthorityError>
    {
        Ok(MemberExchangeWrite {
            input: self.input.clone(), credential, binding,
            session: RecordId::new("authenticated_sessions", self.session_id.to_string()),
            session_token_hash: self.token_hash.clone(), channel: self.channel.clone(),
            issued_at: Datetime::from(self.issued_at), expires_at: Datetime::from(self.expires_at),
            principal_key: record_key(self.input.principal.clone())?,
        })
    }
}
impl SurrealStorage {
    pub async fn exchange_session_credential(
        &self, account_id: &str, principal_id: &str, access_space_id: &str,
        credential_token: &str, channel_binding_hash: &str, ttl: Duration,
    ) -> Result<IssuedSession, ResourceAuthorityError> {
        validate_uuid(account_id)?;
        validate_uuid(principal_id)?;
        validate_uuid(access_space_id)?;
        validate_nonempty(credential_token)?;
        validate_nonempty(channel_binding_hash)?;
        if ttl.is_zero() {
            return Err(ResourceAuthorityError::InvalidInput("session TTL must be greater than zero"));
        }
        if self.administrator_credential_route(account_id, principal_id, access_space_id, credential_token).await? {
            return self.exchange_administrator_session_credential(account_id, principal_id, access_space_id, credential_token, channel_binding_hash, ttl).await;
        }
        let account = RecordId::new("local_accounts", account_id.to_owned());
        let lookup = account.clone();
        let namespace = self.config().namespace().to_owned();
        let database = self.config().database().to_owned();
        let role = self.with_lease(move |client| Box::pin(async move {
            let authority = client.clone();
            authority.use_ns(namespace).use_db(database).await?;
            let mut response = authority.query("SELECT VALUE account_role FROM $account LIMIT 2;")
                .bind(("account", lookup)).await?.check()?;
            let mut roles: Vec<String> = response.take(0)?;
            Ok(if roles.len() == 1 { roles.pop() } else { None })
        })).await?;
        if matches!(role.as_deref(), Some("Owner" | "Admin")) {
            return self.exchange_non_member_session_credential(
                account_id, principal_id, access_space_id, credential_token, channel_binding_hash, ttl,
            ).await;
        }
        if role.as_deref() != Some("Member") {
            return Err(ResourceAuthorityError::InvalidInput("authentication denied"));
        }
        let mut secret = zeroize::Zeroizing::new([0u8; 32]);
        getrandom::getrandom(&mut secret[..])
            .map_err(|error| ResourceAuthorityError::Entropy(error.to_string()))?;
        let token = zeroize::Zeroizing::new(hex::encode(secret.as_ref()));
        let token_hash = sha256_hex(token.as_bytes());
        let session_id = Uuid::now_v7();
        let issued_at = Utc::now();
        let expires_at = issued_at + chrono::Duration::from_std(ttl)
            .map_err(|_| ResourceAuthorityError::InvalidInput("session TTL is outside chrono range"))?;
        let original = std::sync::Arc::new(MemberSessionExchangeAttempt {
            storage: self.clone(),
            input: MemberExchangeQuery {
                account, principal: RecordId::new("principals", principal_id.to_owned()),
                space: RecordId::new("access_spaces", access_space_id.to_owned()),
                credential_token_hash: sha256_hex(credential_token.as_bytes()),
            },
            session_id, token, token_hash,
            credential_token: zeroize::Zeroizing::new(credential_token.to_owned()),
            channel: channel_binding_hash.to_owned(), issued_at, expires_at,
            selected: std::sync::OnceLock::new(),
        });
        let owner = std::sync::Arc::new(MemberSessionExchangeOperation {
            original: original.clone(), state: tokio::sync::Mutex::new(MemberSessionExchangeState::Starting),
        });
        {
            let mut state = owner.state.try_lock()
                .map_err(|_| ResourceAuthorityError::InvalidInput("unpublished exchange state locked"))?;
            let admission = self.inner.member_login_admission.lock()
                .map_err(|_| ResourceAuthorityError::InvalidInput("login admission poisoned"))?;
            if !*admission { return Err(SurrealStorageError::Closed.into()); }
            self.reserve_transaction_settlement(session_id, Some(
                super::AuthenticatedTransactionRetention::MemberSessionExchange(owner.clone())
            ))?;
            *state = MemberSessionExchangeState::Running(tokio::spawn(async move {
                original.storage.clone().run_member_session_exchange(original).await
            }));
            drop(state);
            drop(admission);
        }
        let result = owner.observe().await;
        if result.is_ok() {
            let mut retained = self.inner.transaction_settlements.lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if matches!(retained.get(&session_id),
                Some(Some(super::AuthenticatedTransactionRetention::MemberSessionExchange(current)))
                if std::sync::Arc::ptr_eq(current, &owner))
            { retained.remove(&session_id); }
        }
        result
    }

    async fn run_member_session_exchange(
        &self, original: std::sync::Arc<MemberSessionExchangeAttempt>,
    ) -> Result<IssuedSession, MemberSessionExchangeFailure> {
        let namespace = self.config().namespace().to_owned();
        let database = self.config().database().to_owned();
        let result = self.with_lease(move |client| Box::pin(async move {
            let authority = client.clone();
            authority.use_ns(namespace).use_db(database).await?;
            let transaction = authority.begin().await?;
            let body: Result<(RecordId, MemberPasswordLoginBinding), ResourceAuthorityError> = async {
                let mut response = transaction.query(MEMBER_EXCHANGE_ELIGIBLE)
                    .bind(original.input.clone()).await.map_err(SurrealStorageError::from)?.check().map_err(SurrealStorageError::from)?;
                let mut rows: Vec<MemberExchangeEligible> = response.take(0).map_err(SurrealStorageError::from)?;
                if rows.len() != 1 { return Err(ResourceAuthorityError::InvalidInput("authentication denied")); }
                let selected = rows.pop().ok_or(ResourceAuthorityError::InvalidInput("authentication denied"))?;
                require_canonical_member_login_binding(&selected.binding)?;
                original.selected.set((selected.credential.clone(), selected.binding.clone()))
                    .map_err(|_| ResourceAuthorityError::InvalidInput("exchange selection already captured"))?;
                let write = original.write(selected.credential.clone(), selected.binding.clone())?;
                transaction.query(MEMBER_EXCHANGE_WRITE).bind(write).await.map_err(SurrealStorageError::from)?.check().map_err(SurrealStorageError::from)?;
                Ok((selected.credential, selected.binding))
            }.await;
            let (credential, binding) = match body {
                Ok(selected) => selected,
                Err(operation) => {
                    let cancellation = transaction.cancel().await;
                    return Ok(Err(MemberSessionExchangeFailure::Cancel { operation, cancellation }));
                }
            };
            let canonical = match transaction.commit().await {
                Ok(client) => client,
                Err(error) => return Ok(Err(MemberSessionExchangeFailure::Commit(error))),
            };
            let write = match original.write(credential, binding.clone()) {
                Ok(write) => write,
                Err(error) => return Ok(Err(MemberSessionExchangeFailure::Operation(error))),
            };
            let read: Result<bool, SurrealStorageError> = async {
                let mut response = canonical.query(MEMBER_EXCHANGE_CANONICAL).bind(write).await?.check()?;
                let observed: surrealdb::types::Value = response.take(0)?;
                Ok(matches!(observed, surrealdb::types::Value::Bool(true)))
            }.await;
            match read {
                Ok(true) => {
                    let session = (|| -> Result<IssuedSession, ResourceAuthorityError> { Ok(IssuedSession {
                        token: original.token.to_string(), session_id: original.session_id.to_string(),
                        account_id: record_key(binding.account)?,
                        principal_id: record_key(binding.principal)?,
                        access_space_id: record_key(binding.space)?,
                        expires_at: original.expires_at,
                    }) })();
                    Ok(session.map_err(MemberSessionExchangeFailure::Operation))
                }
                Ok(false) => Ok(Err(MemberSessionExchangeFailure::CanonicalMismatch)),
                Err(error) => Ok(Err(MemberSessionExchangeFailure::Storage(error))),
            }
        })).await;
        result.map_err(MemberSessionExchangeFailure::Storage)?
    }

    pub(crate) async fn drain_member_session_exchange_operations(&self)
        -> Result<(), ResourceAuthorityError>
    {
        self.close_member_login_admission()?;
        let owners: Vec<_> = {
            let retained = self.inner.transaction_settlements.lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            retained.values().filter_map(|entry| match entry {
                Some(super::AuthenticatedTransactionRetention::MemberSessionExchange(owner))
                    => Some(owner.clone()),
                _ => None,
            }).collect()
        };
        let administrator_owners: Vec<_> = {
            let retained = self.inner.transaction_settlements.lock().unwrap_or_else(|p| p.into_inner());
            retained.values().filter_map(|entry| match entry {
                Some(super::AuthenticatedTransactionRetention::AdministratorSessionExchange(owner)) => Some(owner.clone()),
                _ => None,
            }).collect()
        };
        let mut first_error = None;
        for owner in administrator_owners {
            if let Err(error) = owner.observe().await {
                if first_error.is_none() { first_error = Some(error); }
            }
        }
        for owner in owners {
            if let Err(error) = owner.observe().await {
                if first_error.is_none() { first_error = Some(error); }
            }
        }
        match first_error { Some(error) => Err(error), None => Ok(()) }
    }
}
// Parent AuthenticatedTransactionRetention additionally receives:
// MemberSessionExchange(Arc<resource_authority::MemberSessionExchangeOperation>).
// Host shutdown drains BOTH Member login inventories before storage close and
// retains the store and full startup context on an unresolved drain.
// Legacy non-Member method below is the actual old body with ONLY a private name
// and current non-Member role condition added. Existing Owner/Admin policy is preserved.

impl SurrealStorage {
    async fn exchange_non_member_session_credential(
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
                        "BEGIN TRANSACTION; LET $eligible = SELECT VALUE id FROM session_exchange_credentials WHERE token_hash = $credential_token_hash AND status = 'active' AND revoked_at = NONE AND expires_at > time::now() AND account_id = $account AND principal_id = $principal AND access_space_id = $space AND account_id.status = 'enabled' AND account_id.account_role IN ['Owner', 'Admin'] AND member_login_binding = NONE AND administrator_login_binding = NONE AND principal_id.capability_profile_id != 'MemberEnrollmentAdministrator' AND principal_id.status = 'enabled' AND access_space_id.status = 'active' AND access_space_id.account_id = account_id LIMIT 1; IF array::len($eligible) = 0 { THROW 'HSK-AUTH-CREDENTIAL-DENIED'; }; UPDATE session_exchange_credentials SET status = 'consumed' WHERE id IN $eligible; CREATE $session_record SET account_id = $account, principal_id = $principal, access_space_id = $space, token_hash = $session_token_hash, channel_binding_hash = $channel_binding_hash, authentication_strength = 'credential_exchange', delegated_capabilities = $principal.delegated_capabilities, delegation_chain = [$principal_key], account_revocation_epoch = $account.revocation_epoch, principal_revocation_epoch = $principal.revocation_epoch, space_revocation_epoch = $space.revocation_epoch, policy_version = math::max([$account.policy_version, $principal.policy_version, $space.policy_version]), issued_at = $issued_at, expires_at = $expires_at, revoked_at = NONE; COMMIT TRANSACTION;",
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

}

// A credential carrying the private Member binding cannot enter the legacy
// exchange path after a role change. Its original revision/identity binding
// remains mandatory; ordinary legacy Owner/Admin credentials have NONE here.


// RAM ONLY resource_authority.rs: replacement for ONLY stage_member_enrollment_in_transaction.
// Requires exact reviewed original-row helpers, mandatory e585 permit, a713 codec,
// native ced000 OriginalIssueIntent and Account-owned7cbc coordinated projection.
// No alternate original-row CREATE. A missing/unbound original fails closed.
#[derive(SurrealValue)]
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
struct MemberVaultStageRow {
    id: RecordId,
    operation_id: String,
    original_request_hex: String,
    original_request_sha256: String,
    vault_namespace: String,
    vault_reference: String,
    vault_issue_revision: i64,
    vault_issue_state: String,
    vault_claim_id: Option<String>,
    descriptor_hex: Option<String>,
    descriptor_sha256: Option<String>,
    token_digest: Option<String>,
    account_id: RecordId,
    principal_id: RecordId,
    access_space_id: RecordId,
    account_name: String,
    purpose: String,
    issued_at: Datetime,
    expires_at: Datetime,
    status: String,
    password_verifier: Option<String>,
    redemption_attempt_id: Option<String>,
    consumed_at: Option<Datetime>,
    issue: surrealdb::types::Object,
}
#[derive(SurrealValue)]
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
struct MemberBoundStageBindings {
    original: MemberEnrollmentBindings,
    request_hex: String,
    request_sha256: String,
    namespace: String,
    reference: String,
    revision: i64,
    claim: String,
    issue: MemberEnrollmentIssueProjection,
}
#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl MemberVaultStageRow {
    fn require_bound_original(
        &self,
        descriptor: &enrollment::EnrollmentDescriptor,
        expected: &MemberEnrollmentBindings,
    ) -> Result<MemberEnrollmentRow, ResourceAuthorityError> {
        let denied = || ResourceAuthorityError::InvalidInput("original bound enrollment unavailable");
        if self.vault_issue_state != "bound" || self.vault_issue_revision != 4
            || !matches!(self.status.as_str(), "preparing" | "active" | "consumed" | "revoked")
            || self.purpose != "identity_enrollment"
        { return Err(denied()); }
        let claim = Uuid::parse_str(self.vault_claim_id.as_deref().ok_or_else(denied)?)
            .map_err(|_| denied())?;
        if claim.is_nil() || claim == descriptor.operation_id()
            || self.vault_claim_id.as_deref() != Some(claim.to_string().as_str())
            || self.vault_namespace.is_empty()
            || self.vault_reference != format!(
                "invitation/identity_enrollment/{}", descriptor.operation_id().hyphenated(),
            )
        { return Err(denied()); }
        let request_bytes = hex::decode(&self.original_request_hex).map_err(|_| denied())?;
        let request = decode_original_issue_request(&request_bytes).map_err(|_| denied())?;
        let actual = OriginalIssueIntent {
            operation_id: descriptor.operation_id(),
            original_enroller: descriptor.original_enroller().clone(),
            staged: descriptor.staged().clone(),
            account_name: descriptor.account_name().to_owned(),
            administrator_resource: descriptor.administrator_resource().to_owned(),
            enroll_source: descriptor.enroll_source().clone(),
            assignment_source: descriptor.assignment_source().clone(),
            issued_at: descriptor.issued_at(), expires_at: descriptor.expires_at(),
            session_expires_at: descriptor.session_expires_at(),
            policy_expires_at: descriptor.policy_expires_at(),
            purpose: descriptor.invitation_purpose().to_owned(),
            vault_namespace: self.vault_namespace.clone(),
            vault_reference: self.vault_reference.clone(),
        };
        if request != actual
            || encode_original_issue_request(&request).map_err(|_| denied())? != request_bytes
            || self.original_request_hex != hex::encode(&request_bytes)
            || self.original_request_sha256 != sha256_hex(&request_bytes)
        { return Err(denied()); }
        let bytes = enrollment::encode_enrollment(descriptor).map_err(|_| denied())?;
        if self.descriptor_hex.as_deref() != Some(hex::encode(&bytes).as_str())
            || self.descriptor_sha256.as_deref() != Some(sha256_hex(&bytes).as_str())
            || self.token_digest.as_deref() != Some(hex::encode(descriptor.token_digest()).as_str())
        { return Err(denied()); }
        let row = MemberEnrollmentRow {
            id: self.id.clone(), operation_id: self.operation_id.clone(),
            descriptor_hex: self.descriptor_hex.clone().ok_or_else(denied)?,
            descriptor_sha256: self.descriptor_sha256.clone().ok_or_else(denied)?,
            account_id: self.account_id.clone(), principal_id: self.principal_id.clone(),
            access_space_id: self.access_space_id.clone(), account_name: self.account_name.clone(),
            token_digest: self.token_digest.clone().ok_or_else(denied)?, purpose: self.purpose.clone(),
            issued_at: self.issued_at.clone(), expires_at: self.expires_at.clone(),
            status: self.status.clone(), password_verifier: self.password_verifier.clone(),
            redemption_attempt_id: self.redemption_attempt_id.clone(), consumed_at: self.consumed_at.clone(),
        };
        expected.require_original(&row)?;
        Ok(row)
    }
}

#[cfg(all(feature = "runtime-full", feature = "os-keychain"))]
impl SurrealStorage {
    async fn stage_member_enrollment_in_transaction(
        &self, tx: &super::SurrealTransactionContext<'_>,
        permit: &MemberEnrollmentTransactionPermit<'_, '_>, captured: &CapturedMemberEnrollment,
    ) -> Result<MemberIssueObservation, ResourceAuthorityError> {
        let original = MemberEnrollmentBindings::original(&captured.plan)?;
        permit.require_original_lookup(tx, captured.plan.descriptor()).await?;
        let rows: Vec<MemberVaultStageRow> = tx.query_values_at(
            "SELECT * FROM $enrollment;", original.clone(), 0,
        ).await?;
        let [before] = rows.as_slice() else {
            return Err(ResourceAuthorityError::InvalidInput("original enrollment unobserved"));
        };
        let observed = before.require_bound_original(captured.plan.descriptor(), &original)?;
        if observed.status != "preparing" {
            // Read-only original reconciliation, never another create/activation.
            // Existing active/consumed/revoked semantics remain the old caller's;
            // this branch grants no recipient verification or fresh delivery.
            return Ok(MemberIssueObservation::ExistingOriginal(observed));
        }
        if observed.password_verifier.is_some() || observed.redemption_attempt_id.is_some()
            || observed.consumed_at.is_some()
        {
            return Err(ResourceAuthorityError::InvalidInput("original preparing row already redeemed"));
        }
        // This exact private permit rechecks actual dual actions, source facts,
        // parent/lifetime and current KDF-bound credential nonce in THIS tx.
        permit.require_first_issue(tx, captured).await?;
        let issue = permit.issue_projection()?;
        let surrealdb::types::Value::Object(expected_issue) = issue.clone().into_value() else {
            return Err(ResourceAuthorityError::InvalidInput("invalid issue source projection"));
        };
        if before.issue != expected_issue {
            return Err(ResourceAuthorityError::InvalidInput("bound original issue projection changed"));
        }
        let claim = before.vault_claim_id.clone()
            .ok_or(ResourceAuthorityError::InvalidInput("bound original claim missing"))?;
        let binding = MemberBoundStageBindings {
            original: original.clone(),
            request_hex: before.original_request_hex.clone(),
            request_sha256: before.original_request_sha256.clone(),
            namespace: before.vault_namespace.clone(), reference: before.vault_reference.clone(),
            revision: before.vault_issue_revision, claim, issue: issue.clone(),
        };
        let rows: Vec<MemberEnrollmentRow> = tx.query_values_at(
            "IF record::exists($original.account) OR record::exists($original.principal) OR record::exists($original.space) OR array::len(SELECT id FROM local_accounts WHERE account_key = $original.name LIMIT 1) != 0 { THROW 'HSK-ENROLLMENT-COLLISION'; };
             CREATE $original.account SET account_key = $original.name, account_role = 'Member', status = 'disabled', credential_revision = 0, member_enrollment_origin = $original.enrollment, revocation_epoch = 0, policy_version = 1, created_at = time::now(), updated_at = time::now();
             CREATE $original.principal SET principal_key = $original.actor, account_id = $original.account, principal_kind = 'human_account', actor_kind = 'operator', actor_id = $original.actor, capability_profile_id = $original.profile, delegated_capabilities = $original.capabilities, status = 'disabled', member_enrollment_origin = $original.enrollment, revocation_epoch = 0, policy_version = 1, created_at = time::now(), updated_at = time::now();
             CREATE $original.space SET space_key = $original.actor, account_id = $original.account, name = 'Private', status = 'disabled', member_enrollment_origin = $original.enrollment, revocation_epoch = 0, policy_version = 1, created_at = time::now(), updated_at = time::now();
             LET $activated = UPDATE $original.enrollment SET status = 'active'
                WHERE status = 'preparing' AND vault_issue_state = 'bound' AND vault_issue_revision = $revision AND vault_claim_id = $claim
                AND original_request_hex = $request_hex AND original_request_sha256 = $request_sha256
                AND vault_namespace = $namespace AND vault_reference = $reference AND issue = $issue
                AND operation_id = $original.operation AND descriptor_hex = $original.descriptor_hex
                AND descriptor_sha256 = $original.descriptor_sha256 AND token_digest = $original.token_digest
                AND account_id = $original.account AND principal_id = $original.principal AND access_space_id = $original.space
                AND account_name = $original.name AND purpose = 'identity_enrollment'
                AND issued_at = $original.issued_at AND expires_at = $original.expires_at
                AND issued_at <= time::now() AND expires_at > time::now()
                AND password_verifier = NONE AND redemption_attempt_id = NONE AND consumed_at = NONE RETURN AFTER;
             IF array::len($activated) != 1 { THROW 'HSK-ENROLLMENT-ACTIVATE'; };
             SELECT * FROM $original.enrollment;",
            binding, 6,
        ).await?;
        let [row] = rows.as_slice() else {
            return Err(ResourceAuthorityError::InvalidInput("missing activated enrollment readback"));
        };
        original.require_original(row)?;
        if row.status != "active" || row.password_verifier.is_some()
            || row.redemption_attempt_id.is_some() || row.consumed_at.is_some()
        {
            return Err(ResourceAuthorityError::InvalidInput("invalid activated enrollment"));
        }
        let targets: Vec<MemberTargetRow> = tx.query_values_at(
            MEMBER_TARGET_READBACK, original.clone(), 0,
        ).await?;
        let [target] = targets.as_slice() else {
            return Err(ResourceAuthorityError::InvalidInput("missing original staged targets"));
        };
        original.require_target(target, false, None)?;
        let origins: Vec<MemberEnrollmentOrigins> = tx.query_values_at(
            "RETURN { account: $account.member_enrollment_origin, principal: $principal.member_enrollment_origin, space: $space.member_enrollment_origin };",
            original.clone(), 0,
        ).await?;
        let [origins] = origins.as_slice() else {
            return Err(ResourceAuthorityError::InvalidInput("missing original staged origins"));
        };
        if origins.account != original.enrollment || origins.principal != original.enrollment
            || origins.space != original.enrollment
        {
            return Err(ResourceAuthorityError::InvalidInput("staged origins differ"));
        }
        let final_rows: Vec<MemberVaultStageRow> = tx.query_values_at(
            "SELECT * FROM $enrollment;", original.clone(), 0,
        ).await?;
        let [after] = final_rows.as_slice() else {
            return Err(ResourceAuthorityError::InvalidInput("missing final original phase"));
        };
        let final_original = after.require_bound_original(captured.plan.descriptor(), &original)?;
        if final_original.status != "active" || after.issue != expected_issue
            || after.original_request_hex != before.original_request_hex
            || after.original_request_sha256 != before.original_request_sha256
            || after.vault_namespace != before.vault_namespace
            || after.vault_reference != before.vault_reference
            || after.vault_claim_id != before.vault_claim_id
            || after.vault_issue_revision != before.vault_issue_revision
        {
            return Err(ResourceAuthorityError::InvalidInput("original phase changed during staging"));
        }
        // Canonical readback is still inside the SAME borrowed worker, not a
        // commit receipt. Outer actual controller owns commit/Unknown custody.
        permit.require_first_issue(tx, captured).await?;
        Ok(MemberIssueObservation::Created)
    }
}
// Account owns its projection producer extension: issuer_credential_revision
// MUST come from7cbc genuine bound_credential_revision in BOTH factories, and
// every mutation must retain exact private KDF observation/physical nonce proof.
// This body implements no controller, no password enable, no session/grant mint.
// Old redemption/login helpers are unchanged and their unmounted private provider
// and nonce/permission blockers are not waived by successful original staging.
// A missing/Reserved/Preparing/WriteStarted/corrupt row never falls back to CREATE.


#[derive(Clone, PartialEq, Eq, SurrealValue)]
struct AdministratorPasswordLoginBinding {
    origin: surrealdb::types::Object,
    administration: surrealdb::types::Object,
    parent: surrealdb::types::Object,
    enroll: surrealdb::types::Object,
    revoke: surrealdb::types::Object,
    assign: surrealdb::types::Object,
    account: RecordId, principal: RecordId, space: RecordId, name: String,
    credential_revision: i64,
    account_epoch: i64, principal_epoch: i64, space_epoch: i64,
    account_policy: i64, principal_policy: i64, space_policy: i64,
    profile: String, capabilities: Vec<String>,
}
fn administrator_binding_expiry(
    binding: &AdministratorPasswordLoginBinding, requested: DateTime<Utc>,
) -> Result<DateTime<Utc>, ResourceAuthorityError> {
    let denied = || ResourceAuthorityError::InvalidInput("authentication denied");
    let mut expires = requested;
    for grant in [&binding.enroll, &binding.revoke, &binding.assign] {
        if let Some(value) = grant.get("expires_at") {
            if *value != surrealdb::types::Value::None {
                let timestamp = Datetime::from_value(value.clone()).map_err(|_| denied())?.into_inner();
                expires = expires.min(timestamp);
            }
        }
    }
    let lifetime = binding.origin.get("administration_lifetime").ok_or_else(denied)?;
    match AdministrationLifetimeRow::from_value(lifetime.clone()).map_err(|_| denied())? {
        AdministrationLifetimeRow::NoExpiry => {},
        AdministrationLifetimeRow::ExpiresAt(timestamp) => expires = expires.min(timestamp.into_inner()),
    }
    if expires <= Utc::now() { return Err(denied()); }
    Ok(expires)
}

#[derive(SurrealValue)]
struct AdministratorPasswordLoginQuery { snapshot: super::local_accounts::AdministratorPasswordCredentialSnapshot }
const ADMINISTRATOR_PASSWORD_LOGIN_BINDING: &str = "RETURN fn::wp012_administrator_login_snapshot($snapshot);";
#[derive(SurrealValue)]
struct AdministratorPasswordLoginWrite { binding: AdministratorPasswordLoginBinding, verifier: String, credential: RecordId, token_hash: String, expires_at: Datetime }
const ADMINISTRATOR_PASSWORD_LOGIN_WRITE: &str = "IF !fn::wp012_administrator_login_binding_current($binding) { THROW 'HSK-ADMINISTRATOR-LOGIN-DENIED'; };\nLET $a = UPDATE $binding.account SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 WHERE status = 'enabled' AND account_role = 'Owner' AND account_key = $binding.name AND credential_revision = $binding.credential_revision AND password_verifier != NONE AND revocation_epoch = $binding.account_epoch AND policy_version = $binding.account_policy RETURN VALUE id;\nLET $p = UPDATE $binding.principal SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 WHERE account_id = $binding.account AND principal_kind = 'human_account' AND status = 'enabled' AND revocation_epoch = $binding.principal_epoch AND policy_version = $binding.principal_policy AND capability_profile_id = $binding.profile AND delegated_capabilities = $binding.capabilities RETURN VALUE id;\nLET $s = UPDATE $binding.space SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 WHERE account_id = $binding.account AND status = 'active' AND revocation_epoch = $binding.space_epoch AND policy_version = $binding.space_policy RETURN VALUE id;\nLET $r = UPDATE $binding.administration.id SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id;\nLET $w = UPDATE $binding.parent.id SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id;\nLET $e = UPDATE $binding.enroll.id SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id;\nLET $v = UPDATE $binding.revoke.id SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id;\nLET $g = UPDATE $binding.assign.id SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id;\nIF array::len($a) != 1 OR array::len($p) != 1 OR array::len($s) != 1 OR array::len($r) != 1 OR array::len($w) != 1 OR array::len($e) != 1 OR array::len($v) != 1 OR array::len($g) != 1 OR !fn::wp012_administrator_login_binding_current($binding) { THROW 'HSK-ADMINISTRATOR-LOGIN-DENIED'; };\nIF $binding.account.password_verifier != $verifier OR record::exists($credential) OR $expires_at <= time::now() { THROW 'HSK-ADMINISTRATOR-LOGIN-DENIED'; };\nCREATE $credential SET account_id = $binding.account, principal_id = $binding.principal, access_space_id = $binding.space, token_hash = $token_hash, status = 'active', expires_at = $expires_at, revoked_at = NONE, created_at = time::now(), administrator_login_binding = $binding;\n";
const ADMINISTRATOR_PASSWORD_LOGIN_READBACK: &str = "RETURN record::exists($credential) AND $credential.account_id = $binding.account AND $credential.principal_id = $binding.principal AND $credential.access_space_id = $binding.space AND $credential.token_hash = $token_hash AND $credential.status = 'active' AND $credential.revoked_at = NONE AND $credential.expires_at = $expires_at AND $credential.expires_at > time::now() AND $credential.administrator_login_binding = $binding AND $credential.member_login_binding = NONE AND fn::wp012_administrator_login_binding_current($binding);";

struct AdministratorPasswordLoginAttempt {
    storage: SurrealStorage,
    checked: super::local_accounts::AdministratorPasswordVerifiedAccount,
    credential: RecordId,
    token: zeroize::Zeroizing<String>,
    token_hash: String,
    expires_at: DateTime<Utc>,
    binding: std::sync::OnceLock<AdministratorPasswordLoginBinding>,
}
enum AdministratorPasswordLoginFailure {
    Operation(ResourceAuthorityError),
    Cancel {
        operation: ResourceAuthorityError,
        cancellation: Result<super::SurrealClient, surrealdb::Error>,
    },
    Commit(surrealdb::Error),
    CanonicalRead(SurrealStorageError),
    CanonicalMismatch,
}
enum AdministratorPasswordLoginObservation {
    Complete(std::sync::Arc<Result<IssuedSessionCredential, AdministratorPasswordLoginFailure>>),
    Join(std::sync::Arc<tokio::task::JoinError>),
}
enum AdministratorPasswordLoginWorkerState {
    Starting,
    Running(tokio::task::JoinHandle<Result<IssuedSessionCredential, AdministratorPasswordLoginFailure>>),
    Observed(AdministratorPasswordLoginObservation),
}
pub(super) struct AdministratorPasswordLoginOperation {
    original: std::sync::Arc<AdministratorPasswordLoginAttempt>,
    state: tokio::sync::Mutex<AdministratorPasswordLoginWorkerState>,
}
impl AdministratorPasswordLoginOperation {
    async fn observe(&self) -> Result<IssuedSessionCredential, ResourceAuthorityError> {
        let mut state = self.state.lock().await;
        if let AdministratorPasswordLoginWorkerState::Running(worker) = &mut *state {
            // Borrow the retained handle. Cancelling a waiter cannot detach the worker.
            let observed = match worker.await {
                Ok(result) => AdministratorPasswordLoginObservation::Complete(std::sync::Arc::new(result)),
                Err(error) => AdministratorPasswordLoginObservation::Join(std::sync::Arc::new(error)),
            };
            *state = AdministratorPasswordLoginWorkerState::Observed(observed);
        }
        match &*state {
            AdministratorPasswordLoginWorkerState::Observed(AdministratorPasswordLoginObservation::Complete(result)) => {
                match result.as_ref() {
                    Ok(credential) => Ok(credential.clone()),
                    Err(_) => Err(SurrealStorageError::TransactionSettlementPending {
                        operation_id: Uuid::parse_str(&record_key(self.original.credential.clone())?)
                            .map_err(|_| ResourceAuthorityError::InvalidInput("invalid credential operation"))?,
                    }.into()),
                }
            }
            _ => Err(SurrealStorageError::TransactionSettlementPending {
                operation_id: Uuid::parse_str(&record_key(self.original.credential.clone())?)
                    .map_err(|_| ResourceAuthorityError::InvalidInput("invalid credential operation"))?,
            }.into()),
        }
    }
}
impl SurrealStorage {
    pub(super) async fn provision_verified_administrator_session_credential(
        &self,
        checked: super::local_accounts::AdministratorPasswordVerifiedAccount,
        ttl: Duration,
    ) -> Result<IssuedSessionCredential, ResourceAuthorityError> {
        // Existing normal login policy only. The caller cannot extend a password proof.
        if ttl != Duration::from_secs(60) {
            return Err(ResourceAuthorityError::InvalidInput("invalid login credential lifetime"));
        }
        checked.credential_snapshot(self)?;
        let mut entropy = zeroize::Zeroizing::new([0u8; 32]);
        getrandom::getrandom(&mut entropy[..])
            .map_err(|error| ResourceAuthorityError::Entropy(error.to_string()))?;
        let token = zeroize::Zeroizing::new(hex::encode(entropy.as_ref()));
        let token_hash = sha256_hex(token.as_bytes());
        let operation_id = Uuid::now_v7();
        let original = std::sync::Arc::new(AdministratorPasswordLoginAttempt {
            storage: self.clone(), checked,
            credential: RecordId::new("session_exchange_credentials", operation_id.to_string()),
            token, token_hash, expires_at: Utc::now() + chrono::Duration::seconds(60),
            binding: std::sync::OnceLock::new(),
        });
        let operation = std::sync::Arc::new(AdministratorPasswordLoginOperation {
            original: original.clone(),
            state: tokio::sync::Mutex::new(AdministratorPasswordLoginWorkerState::Starting),
        });
        // Add AdministratorPasswordLogin(Arc<resource_authority::AdministratorPasswordLoginOperation>)
        // to the existing parent AuthenticatedTransactionRetention enum.
        {
            let admission = self.inner.member_login_admission.lock()
                .map_err(|_| ResourceAuthorityError::InvalidInput("login admission poisoned"))?;
            if !*admission { return Err(SurrealStorageError::Closed.into()); }
            self.reserve_transaction_settlement(operation_id, Some(
                super::AuthenticatedTransactionRetention::AdministratorPasswordLogin(operation.clone())
            ))?;
            // No await between publishing the strong owner and publishing its worker.
            let mut state = operation.state.try_lock().map_err(|_| {
                ResourceAuthorityError::InvalidInput("unpublished login worker is locked")
            })?;
            let worker = tokio::spawn(async move { original.storage.clone().run_administrator_password_login(original).await });
            *state = AdministratorPasswordLoginWorkerState::Running(worker);
            drop(state);
            drop(admission);
        }
        let result = operation.observe().await;
        if result.is_ok() {
            let mut retained = self.inner.transaction_settlements.lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if matches!(retained.get(&operation_id),
                Some(Some(super::AuthenticatedTransactionRetention::AdministratorPasswordLogin(current)))
                if std::sync::Arc::ptr_eq(current, &operation))
            { retained.remove(&operation_id); }
        }
        result
    }

    async fn run_administrator_password_login(
        &self, original: std::sync::Arc<AdministratorPasswordLoginAttempt>,
    ) -> Result<IssuedSessionCredential, AdministratorPasswordLoginFailure> {
        let storage = self.clone();
        let namespace = self.config().namespace().to_owned();
        let database = self.config().database().to_owned();
        let retained = original.clone();
        let result = self.with_lease(move |client| Box::pin(async move {
            let authority = client.clone();
            authority.use_ns(namespace).use_db(database).await?;
            let transaction = authority.begin().await?;
            let body: Result<(AdministratorPasswordLoginBinding, DateTime<Utc>), ResourceAuthorityError> = async {
                let snapshot = retained.checked.credential_snapshot(&storage)?;
                let mut response = transaction.query(ADMINISTRATOR_PASSWORD_LOGIN_BINDING)
                    .bind(AdministratorPasswordLoginQuery {
                        snapshot,
                    }).await.map_err(SurrealStorageError::from)?.check().map_err(SurrealStorageError::from)?;
                let mut rows: Vec<AdministratorPasswordLoginBinding> = response.take(0).map_err(SurrealStorageError::from)?;
                if rows.len() != 1 {
                    return Err(ResourceAuthorityError::InvalidInput("authentication denied"));
                }
                let binding = rows.pop().ok_or(ResourceAuthorityError::InvalidInput("authentication denied"))?;
                if binding.credential_revision <= 0
                    || binding.account != retained.checked.credential_snapshot(&storage)?.account
                { return Err(ResourceAuthorityError::InvalidInput("authentication denied")); }
                retained.binding.set(binding.clone())
                    .map_err(|_| ResourceAuthorityError::InvalidInput("login binding already captured"))?;
                let effective_expiry = administrator_binding_expiry(&binding, retained.expires_at)?;
                let snapshot = retained.checked.credential_snapshot(&storage)?;
                transaction.query(ADMINISTRATOR_PASSWORD_LOGIN_WRITE).bind(AdministratorPasswordLoginWrite {
                    binding: binding.clone(), verifier: snapshot.verifier,
                    credential: retained.credential.clone(), token_hash: retained.token_hash.clone(),
                    expires_at: Datetime::from(effective_expiry),
                }).await.map_err(SurrealStorageError::from)?.check().map_err(SurrealStorageError::from)?;
                Ok((binding, effective_expiry))
            }.await;
            let (binding, effective_expiry) = match body {
                Ok(selected) => selected,
                Err(operation) => {
                    let cancellation = transaction.cancel().await;
                    return Ok(Err(AdministratorPasswordLoginFailure::Cancel { operation, cancellation }));
                }
            };
            let canonical = match transaction.commit().await {
                Ok(client) => client,
                Err(error) => return Ok(Err(AdministratorPasswordLoginFailure::Commit(error))),
            };
            let snapshot = match retained.checked.credential_snapshot(&storage) {
                Ok(snapshot) => snapshot,
                Err(error) => return Ok(Err(AdministratorPasswordLoginFailure::Operation(error))),
            };
            let readback: Result<bool, SurrealStorageError> = async {
                let mut response = canonical.query(ADMINISTRATOR_PASSWORD_LOGIN_READBACK)
                    .bind(AdministratorPasswordLoginWrite {
                        binding: binding.clone(), verifier: snapshot.verifier,
                        credential: retained.credential.clone(), token_hash: retained.token_hash.clone(),
                        expires_at: Datetime::from(effective_expiry),
                    }).await.map_err(SurrealStorageError::from)?.check().map_err(SurrealStorageError::from)?;
                let observed: surrealdb::types::Value = response.take(0)?;
                Ok(matches!(observed, surrealdb::types::Value::Bool(true)))
            }.await;
            match readback {
                Ok(true) => Ok(Ok(IssuedSessionCredential {
                    credential_id: record_key(retained.credential.clone())
                        .map_err(|_| SurrealStorageError::TransactionRetentionMismatch {
                            operation_id: Uuid::nil(),
                        })?,
                    token: retained.token.to_string(),
                    account_id: record_key(binding.account).map_err(|_| SurrealStorageError::TransactionRetentionMismatch { operation_id: Uuid::nil() })?,
                    principal_id: record_key(binding.principal).map_err(|_| SurrealStorageError::TransactionRetentionMismatch { operation_id: Uuid::nil() })?,
                    access_space_id: record_key(binding.space).map_err(|_| SurrealStorageError::TransactionRetentionMismatch { operation_id: Uuid::nil() })?,
                    expires_at: effective_expiry,
                })),
                Ok(false) => Ok(Err(AdministratorPasswordLoginFailure::CanonicalMismatch)),
                Err(error) => Ok(Err(AdministratorPasswordLoginFailure::CanonicalRead(error))),
            }
        })).await;
        match result {
            Ok(result) => result,
            Err(error) => Err(AdministratorPasswordLoginFailure::CanonicalRead(error)),
        }
    }
}
// Parent enum insertion is a typed retention owner only. Do not send this variant
// through transaction_worker: this private normal-login worker owns its SDK transaction.
// Storage shutdown must await retained AdministratorPasswordLoginOperation::observe before close.
// Schema: DEFINE FIELD OVERWRITE member_login_binding ON session_exchange_credentials
// FLEXIBLE TYPE option<object> PERMISSIONS NONE;
// No password verifier or raw password is persisted in member_login_binding.


// Additional resource_authority.rs candidate. Pair with private credential worker
// and member-password-login-schema. Existing Owner/Admin exchange is retained below.
#[derive(SurrealValue)]
struct AdministratorExchangeEligible {
    credential: RecordId,
    binding: AdministratorPasswordLoginBinding,
}
#[derive(Clone, SurrealValue)]
struct AdministratorExchangeQuery {
    account: RecordId,
    principal: RecordId,
    space: RecordId,
    credential_token_hash: String,
}
#[derive(SurrealValue)]
struct AdministratorExchangeWrite {
    input: AdministratorExchangeQuery,
    credential: RecordId,
    binding: AdministratorPasswordLoginBinding,
    session: RecordId,
    session_token_hash: String,
    channel: String,
    issued_at: Datetime,
    expires_at: Datetime,
    principal_key: String,
}
const ADMINISTRATOR_EXCHANGE_ELIGIBLE: &str = "SELECT id AS credential, administrator_login_binding AS binding FROM session_exchange_credentials WHERE account_id = $account AND principal_id = $principal AND access_space_id = $space AND token_hash = $credential_token_hash AND status = 'active' AND revoked_at = NONE AND expires_at > time::now() AND member_login_binding = NONE AND administrator_login_binding != NONE AND administrator_login_binding.account = account_id AND administrator_login_binding.principal = principal_id AND administrator_login_binding.space = access_space_id AND fn::wp012_administrator_login_binding_current(administrator_login_binding) LIMIT 2;";
const ADMINISTRATOR_EXCHANGE_WRITE: &str = "IF !fn::wp012_administrator_login_binding_current($binding) { THROW 'HSK-ADMINISTRATOR-LOGIN-DENIED'; };\nLET $a = UPDATE $binding.account SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 WHERE status = 'enabled' AND account_role = 'Owner' AND account_key = $binding.name AND credential_revision = $binding.credential_revision AND password_verifier != NONE AND revocation_epoch = $binding.account_epoch AND policy_version = $binding.account_policy RETURN VALUE id;\nLET $p = UPDATE $binding.principal SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 WHERE account_id = $binding.account AND principal_kind = 'human_account' AND status = 'enabled' AND revocation_epoch = $binding.principal_epoch AND policy_version = $binding.principal_policy AND capability_profile_id = $binding.profile AND delegated_capabilities = $binding.capabilities RETURN VALUE id;\nLET $s = UPDATE $binding.space SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 WHERE account_id = $binding.account AND status = 'active' AND revocation_epoch = $binding.space_epoch AND policy_version = $binding.space_policy RETURN VALUE id;\nLET $r = UPDATE $binding.administration.id SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id;\nLET $w = UPDATE $binding.parent.id SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id;\nLET $e = UPDATE $binding.enroll.id SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id;\nLET $v = UPDATE $binding.revoke.id SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id;\nLET $g = UPDATE $binding.assign.id SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id;\nIF array::len($a) != 1 OR array::len($p) != 1 OR array::len($s) != 1 OR array::len($r) != 1 OR array::len($w) != 1 OR array::len($e) != 1 OR array::len($v) != 1 OR array::len($g) != 1 OR !fn::wp012_administrator_login_binding_current($binding) { THROW 'HSK-ADMINISTRATOR-LOGIN-DENIED'; };\nLET $used = UPDATE $credential SET status = 'consumed' WHERE account_id = $input.account AND principal_id = $input.principal AND access_space_id = $input.space AND token_hash = $input.credential_token_hash AND status = 'active' AND revoked_at = NONE AND expires_at > time::now() AND member_login_binding = NONE AND administrator_login_binding = $binding RETURN AFTER;\nIF array::len($used) != 1 OR record::exists($session) OR $expires_at <= time::now() { THROW 'HSK-ADMINISTRATOR-EXCHANGE-DENIED'; };\nCREATE $session SET account_id = $binding.account, principal_id = $binding.principal, access_space_id = $binding.space, token_hash = $session_token_hash, channel_binding_hash = $channel, authentication_strength = 'credential_exchange', delegated_capabilities = $binding.capabilities, delegation_chain = [$principal_key], account_revocation_epoch = $binding.account_epoch, principal_revocation_epoch = $binding.principal_epoch, space_revocation_epoch = $binding.space_epoch, policy_version = math::max([$binding.account_policy,$binding.principal_policy,$binding.space_policy]), administrator_login_binding = $binding, issued_at = $issued_at, expires_at = $expires_at, revoked_at = NONE;\n";
const ADMINISTRATOR_EXCHANGE_CANONICAL: &str = "RETURN record::exists($session) AND $credential.status = 'consumed' AND $credential.administrator_login_binding = $binding AND $credential.member_login_binding = NONE AND $credential.token_hash = $input.credential_token_hash AND $session.account_id = $binding.account AND $session.principal_id = $binding.principal AND $session.access_space_id = $binding.space AND $session.token_hash = $session_token_hash AND $session.channel_binding_hash = $channel AND $session.authentication_strength = 'credential_exchange' AND $session.delegated_capabilities = $binding.capabilities AND $session.delegation_chain = [$principal_key] AND $session.account_revocation_epoch = $binding.account_epoch AND $session.principal_revocation_epoch = $binding.principal_epoch AND $session.space_revocation_epoch = $binding.space_epoch AND $session.policy_version = math::max([$binding.account_policy,$binding.principal_policy,$binding.space_policy]) AND $session.issued_at = $issued_at AND $session.expires_at = $expires_at AND $session.expires_at > time::now() AND $session.revoked_at = NONE AND $session.administrator_login_binding = $binding AND fn::wp012_administrator_login_binding_current($binding);";
fn require_canonical_administrator_login_binding(binding: &AdministratorPasswordLoginBinding) -> Result<(), ResourceAuthorityError> {
 if binding.credential_revision <= 0 || binding.profile != "MemberEnrollmentAdministrator"
  || !binding.capabilities.iter().map(String::as_str).eq(["account.identity.enroll", "account.identity.enrollment.revoke", "account.identity.member_profile.assign"]) {
  return Err(ResourceAuthorityError::InvalidInput("authentication denied"));
 }
 Ok(())
}
struct AdministratorSessionExchangeAttempt {
    storage: SurrealStorage,
    input: AdministratorExchangeQuery,
    session_id: Uuid,
    token: zeroize::Zeroizing<String>,
    token_hash: String,
    credential_token: zeroize::Zeroizing<String>,
    channel: String,
    issued_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    selected: std::sync::OnceLock<(RecordId, AdministratorPasswordLoginBinding)>,
}
enum AdministratorSessionExchangeFailure {
    Operation(ResourceAuthorityError),
    Cancel { operation: ResourceAuthorityError, cancellation: Result<super::SurrealClient, surrealdb::Error> },
    Commit(surrealdb::Error),
    Storage(SurrealStorageError),
    CanonicalMismatch,
}
enum AdministratorSessionExchangeObserved {
    Complete(std::sync::Arc<Result<IssuedSession, AdministratorSessionExchangeFailure>>),
    Join(std::sync::Arc<tokio::task::JoinError>),
}
enum AdministratorSessionExchangeState {
    Starting,
    Running(tokio::task::JoinHandle<Result<IssuedSession, AdministratorSessionExchangeFailure>>),
    Observed(AdministratorSessionExchangeObserved),
}
pub(super) struct AdministratorSessionExchangeOperation {
    original: std::sync::Arc<AdministratorSessionExchangeAttempt>,
    state: tokio::sync::Mutex<AdministratorSessionExchangeState>,
}
impl AdministratorSessionExchangeOperation {
    async fn observe(&self) -> Result<IssuedSession, ResourceAuthorityError> {
        let mut state = self.state.lock().await;
        if let AdministratorSessionExchangeState::Running(worker) = &mut *state {
            let observed = match worker.await {
                Ok(result) => AdministratorSessionExchangeObserved::Complete(std::sync::Arc::new(result)),
                Err(error) => AdministratorSessionExchangeObserved::Join(std::sync::Arc::new(error)),
            };
            *state = AdministratorSessionExchangeState::Observed(observed);
        }
        match &*state {
            AdministratorSessionExchangeState::Observed(AdministratorSessionExchangeObserved::Complete(result)) => {
                match result.as_ref() {
                    Ok(session) => Ok(session.clone()),
                    Err(_) => Err(SurrealStorageError::TransactionSettlementPending {
                        operation_id: self.original.session_id,
                    }.into()),
                }
            }
            _ => Err(SurrealStorageError::TransactionSettlementPending {
                operation_id: self.original.session_id,
            }.into()),
        }
    }
}
impl AdministratorSessionExchangeAttempt {
    fn write(&self, credential: RecordId, binding: AdministratorPasswordLoginBinding)
        -> Result<AdministratorExchangeWrite, ResourceAuthorityError>
    {
        let expires_at = administrator_binding_expiry(&binding, self.expires_at)?;
        Ok(AdministratorExchangeWrite {
            input: self.input.clone(), credential, binding,
            session: RecordId::new("authenticated_sessions", self.session_id.to_string()),
            session_token_hash: self.token_hash.clone(), channel: self.channel.clone(),
            issued_at: Datetime::from(self.issued_at), expires_at: Datetime::from(expires_at),
            principal_key: record_key(self.input.principal.clone())?,
        })
    }
}
impl SurrealStorage {
    async fn exchange_administrator_session_credential(
        &self, account_id: &str, principal_id: &str, access_space_id: &str,
        credential_token: &str, channel_binding_hash: &str, ttl: Duration,
    ) -> Result<IssuedSession, ResourceAuthorityError> {
        validate_uuid(account_id)?;
        validate_uuid(principal_id)?;
        validate_uuid(access_space_id)?;
        validate_nonempty(credential_token)?;
        validate_nonempty(channel_binding_hash)?;
        if ttl.is_zero() {
            return Err(ResourceAuthorityError::InvalidInput("session TTL must be greater than zero"));
        }
        let account = RecordId::new("local_accounts", account_id.to_owned());
        let mut secret = zeroize::Zeroizing::new([0u8; 32]);
        getrandom::getrandom(&mut secret[..])
            .map_err(|error| ResourceAuthorityError::Entropy(error.to_string()))?;
        let token = zeroize::Zeroizing::new(hex::encode(secret.as_ref()));
        let token_hash = sha256_hex(token.as_bytes());
        let session_id = Uuid::now_v7();
        let issued_at = Utc::now();
        let expires_at = issued_at + chrono::Duration::from_std(ttl)
            .map_err(|_| ResourceAuthorityError::InvalidInput("session TTL is outside chrono range"))?;
        let original = std::sync::Arc::new(AdministratorSessionExchangeAttempt {
            storage: self.clone(),
            input: AdministratorExchangeQuery {
                account, principal: RecordId::new("principals", principal_id.to_owned()),
                space: RecordId::new("access_spaces", access_space_id.to_owned()),
                credential_token_hash: sha256_hex(credential_token.as_bytes()),
            },
            session_id, token, token_hash,
            credential_token: zeroize::Zeroizing::new(credential_token.to_owned()),
            channel: channel_binding_hash.to_owned(), issued_at, expires_at,
            selected: std::sync::OnceLock::new(),
        });
        let owner = std::sync::Arc::new(AdministratorSessionExchangeOperation {
            original: original.clone(), state: tokio::sync::Mutex::new(AdministratorSessionExchangeState::Starting),
        });
        {
            let mut state = owner.state.try_lock()
                .map_err(|_| ResourceAuthorityError::InvalidInput("unpublished exchange state locked"))?;
            let admission = self.inner.member_login_admission.lock()
                .map_err(|_| ResourceAuthorityError::InvalidInput("login admission poisoned"))?;
            if !*admission { return Err(SurrealStorageError::Closed.into()); }
            self.reserve_transaction_settlement(session_id, Some(
                super::AuthenticatedTransactionRetention::AdministratorSessionExchange(owner.clone())
            ))?;
            *state = AdministratorSessionExchangeState::Running(tokio::spawn(async move {
                original.storage.clone().run_administrator_session_exchange(original).await
            }));
            drop(state);
            drop(admission);
        }
        let result = owner.observe().await;
        if result.is_ok() {
            let mut retained = self.inner.transaction_settlements.lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if matches!(retained.get(&session_id),
                Some(Some(super::AuthenticatedTransactionRetention::AdministratorSessionExchange(current)))
                if std::sync::Arc::ptr_eq(current, &owner))
            { retained.remove(&session_id); }
        }
        result
    }

    async fn run_administrator_session_exchange(
        &self, original: std::sync::Arc<AdministratorSessionExchangeAttempt>,
    ) -> Result<IssuedSession, AdministratorSessionExchangeFailure> {
        let namespace = self.config().namespace().to_owned();
        let database = self.config().database().to_owned();
        let result = self.with_lease(move |client| Box::pin(async move {
            let authority = client.clone();
            authority.use_ns(namespace).use_db(database).await?;
            let transaction = authority.begin().await?;
            let body: Result<(RecordId, AdministratorPasswordLoginBinding), ResourceAuthorityError> = async {
                let mut response = transaction.query(ADMINISTRATOR_EXCHANGE_ELIGIBLE)
                    .bind(original.input.clone()).await.map_err(SurrealStorageError::from)?.check().map_err(SurrealStorageError::from)?;
                let mut rows: Vec<AdministratorExchangeEligible> = response.take(0).map_err(SurrealStorageError::from)?;
                if rows.len() != 1 { return Err(ResourceAuthorityError::InvalidInput("authentication denied")); }
                let selected = rows.pop().ok_or(ResourceAuthorityError::InvalidInput("authentication denied"))?;
                require_canonical_administrator_login_binding(&selected.binding)?;
                original.selected.set((selected.credential.clone(), selected.binding.clone()))
                    .map_err(|_| ResourceAuthorityError::InvalidInput("exchange selection already captured"))?;
                let write = original.write(selected.credential.clone(), selected.binding.clone())?;
                transaction.query(ADMINISTRATOR_EXCHANGE_WRITE).bind(write).await.map_err(SurrealStorageError::from)?.check().map_err(SurrealStorageError::from)?;
                Ok((selected.credential, selected.binding))
            }.await;
            let (credential, binding) = match body {
                Ok(selected) => selected,
                Err(operation) => {
                    let cancellation = transaction.cancel().await;
                    return Ok(Err(AdministratorSessionExchangeFailure::Cancel { operation, cancellation }));
                }
            };
            let canonical = match transaction.commit().await {
                Ok(client) => client,
                Err(error) => return Ok(Err(AdministratorSessionExchangeFailure::Commit(error))),
            };
            let write = match original.write(credential, binding.clone()) {
                Ok(write) => write,
                Err(error) => return Ok(Err(AdministratorSessionExchangeFailure::Operation(error))),
            };
            let read: Result<bool, SurrealStorageError> = async {
                let mut response = canonical.query(ADMINISTRATOR_EXCHANGE_CANONICAL).bind(write).await.map_err(SurrealStorageError::from)?.check().map_err(SurrealStorageError::from)?;
                let observed: surrealdb::types::Value = response.take(0)?;
                Ok(matches!(observed, surrealdb::types::Value::Bool(true)))
            }.await;
            match read {
                Ok(true) => {
                    let session = (|| -> Result<IssuedSession, ResourceAuthorityError> {
                        let expires_at = administrator_binding_expiry(&binding, original.expires_at)?;
                        Ok(IssuedSession {
                        token: original.token.to_string(), session_id: original.session_id.to_string(),
                        account_id: record_key(binding.account)?,
                        principal_id: record_key(binding.principal)?,
                        access_space_id: record_key(binding.space)?,
                        expires_at,
                    }) })();
                    Ok(session.map_err(AdministratorSessionExchangeFailure::Operation))
                }
                Ok(false) => Ok(Err(AdministratorSessionExchangeFailure::CanonicalMismatch)),
                Err(error) => Ok(Err(AdministratorSessionExchangeFailure::Storage(error))),
            }
        })).await;
        result.map_err(AdministratorSessionExchangeFailure::Storage)?
    }

}

impl SurrealStorage {
    async fn administrator_credential_route(
        &self, account_id: &str, principal_id: &str, space_id: &str, credential_token: &str,
    ) -> Result<bool, ResourceAuthorityError> {
        let namespace = self.config().namespace().to_owned();
        let database = self.config().database().to_owned();
        let query = AdministratorExchangeQuery {
            account: RecordId::new("local_accounts", account_id.to_owned()),
            principal: RecordId::new("principals", principal_id.to_owned()),
            space: RecordId::new("access_spaces", space_id.to_owned()),
            credential_token_hash: sha256_hex(credential_token.as_bytes()),
        };
        self.with_lease(move |client| Box::pin(async move {
            let authority = client.clone();
            authority.use_ns(namespace).use_db(database).await?;
            let mut response = authority.query(
                "LET $marked = SELECT VALUE id FROM session_exchange_credentials WHERE account_id = $account AND principal_id = $principal AND access_space_id = $space AND token_hash = $credential_token_hash AND administrator_login_binding != NONE LIMIT 2; RETURN array::len($marked) != 0 OR $principal.capability_profile_id = 'MemberEnrollmentAdministrator';"
            ).bind(query).await?.check()?;
            let observed: surrealdb::types::Value = response.take(1)?;
            Ok(matches!(observed, surrealdb::types::Value::Bool(true)))
        })).await.map_err(Into::into)
    }
}

pub(super) struct FirstOwnerConfirmationPreparedCredential {
    binding: AdministratorPasswordLoginBinding,
    credential: RecordId,
    token_hash: String,
    expires_at: DateTime<Utc>,
}
impl SurrealStorage {
    pub(super) async fn issue_first_owner_confirmation_credential_in_transaction(
        &self, tx: &super::SurrealTransactionContext<'_>,
        authorization: &super::local_accounts::FirstOwnerConfirmationCredentialAuthorization<'_, '_>,
        intent: &super::local_accounts::FirstOwnerConfirmationCredentialIntent,
    ) -> Result<FirstOwnerConfirmationPreparedCredential, ResourceAuthorityError> {
        let snapshot = authorization.credential_snapshot(self, tx).await?;
        let mut response = tx.transaction.query(ADMINISTRATOR_PASSWORD_LOGIN_BINDING)
            .bind(AdministratorPasswordLoginQuery { snapshot }).await.map_err(SurrealStorageError::from)?
            .check().map_err(SurrealStorageError::from)?;
        let mut rows: Vec<AdministratorPasswordLoginBinding> = response.take(0).map_err(SurrealStorageError::from)?;
        if rows.len() != 1 { return Err(ResourceAuthorityError::InvalidInput("confirmation credential denied")); }
        let binding = rows.pop().ok_or(ResourceAuthorityError::InvalidInput("confirmation credential denied"))?;
        authorization.require_binding(&binding.origin, &binding.account, &binding.principal, &binding.space)?;
        let expires_at = administrator_binding_expiry(&binding, intent.requested_expires_at())?;
        let snapshot = authorization.credential_snapshot(self, tx).await?;
        tx.transaction.query(ADMINISTRATOR_PASSWORD_LOGIN_WRITE).bind(AdministratorPasswordLoginWrite {
            binding: binding.clone(), verifier: snapshot.verifier, credential: intent.record().clone(),
            token_hash: intent.token_hash().to_owned(), expires_at: Datetime::from(expires_at),
        }).await.map_err(SurrealStorageError::from)?.check().map_err(SurrealStorageError::from)?;
        authorization.credential_snapshot(self, tx).await?;
        Ok(FirstOwnerConfirmationPreparedCredential {
            binding, credential: intent.record().clone(), token_hash: intent.token_hash().to_owned(), expires_at,
        })
    }
}
impl FirstOwnerConfirmationPreparedCredential {
    pub(super) async fn canonical_read(&self, client: &super::SurrealClient)
        -> Result<surrealdb::types::Value, SurrealStorageError> {
        let mut response = client.query(ADMINISTRATOR_PASSWORD_LOGIN_READBACK).bind(AdministratorPasswordLoginWrite {
            binding: self.binding.clone(), verifier: String::new(), credential: self.credential.clone(),
            token_hash: self.token_hash.clone(), expires_at: Datetime::from(self.expires_at),
        }).await?.check()?;
        Ok(response.take(0)?)
    }
    pub(super) fn outward(
        &self, intent: &super::local_accounts::FirstOwnerConfirmationCredentialIntent,
    ) -> Result<IssuedSessionCredential, ResourceAuthorityError> {
        if &self.credential != intent.record() || self.token_hash != intent.token_hash()
            || self.expires_at <= Utc::now() {
            return Err(ResourceAuthorityError::InvalidInput("confirmation credential denied"));
        }
        Ok(IssuedSessionCredential {
            credential_id: record_key(self.credential.clone())?, token: intent.token().to_owned(),
            account_id: record_key(self.binding.account.clone())?,
            principal_id: record_key(self.binding.principal.clone())?,
            access_space_id: record_key(self.binding.space.clone())?, expires_at: self.expires_at,
        })
    }
}
