use super::*;

pub(crate) struct ValidatorDelegationParent {
    create: OwnedCanonicalAuthSnapshot,
    read: OwnedCanonicalAuthSnapshot,
}

// This transaction owns identity issuance only. Ordinary resource operations
// subsequently authenticate as the child record user and retain table permissions.
const ISSUE_CHILD: &str = r#"
BEGIN TRANSACTION;
LET $parent = (SELECT * FROM ONLY $parent_record);
LET $grant = (SELECT * FROM ONLY $parent_grant);
LET $read_grant = (SELECT * FROM ONLY $parent_read_grant);
LET $read_authority = {
    grant_id: $read_grant.id, resource_id: $read_grant.resource_id,
    account_id: $read_grant.account_id, principal_id: $read_grant.principal_id,
    actor_kind: $read_grant.principal_id.actor_kind, actor_id: $read_grant.principal_id.actor_id,
    session_id: $parent_record, access_space_id: $read_grant.access_space_id,
    grant_version: $read_grant.grant_version,
    policy_versions: [$read_grant.policy_version, $resource.policy_version, $account.policy_version,
        $parent_principal.policy_version, $space.policy_version, $parent.policy_version],
    revocation_epochs: [$account.revocation_epoch, $parent_principal.revocation_epoch, $space.revocation_epoch,
        $parent.account_revocation_epoch, $parent.principal_revocation_epoch, $parent.space_revocation_epoch],
    actions: $read_grant.actions, capability_ids: $read_grant.capability_ids,
    delegation_chain: $read_grant.delegation_chain, session_capabilities: $parent.delegated_capabilities,
    principal_capabilities: $parent_principal.delegated_capabilities,
    capability_profile_id: $parent_principal.capability_profile_id,
    statuses: [$read_grant.status, $resource.lifecycle_state, $account.status, $parent_principal.status, $space.status],
    resource_kind: $resource.resource_kind, grant_expires_at: $read_grant.expires_at,
    session_expires_at: $parent.expires_at
};
IF $parent = NONE OR !fn::mt109_session_live($parent_record)
    OR $account.account_role != 'Owner' OR $parent_principal.principal_kind != 'human_account'
    OR $read_authority != $expected_parent_read_authority
    OR $parent.token_hash != $parent_token_hash
    OR $parent.channel_binding_hash != $parent_channel
    OR $parent.account_id != $account OR $parent.principal_id != $parent_principal
    OR $parent.access_space_id != $space OR $parent.delegation_chain != $parent_chain
    OR $parent.policy_version != $policy OR $parent.expires_at != $parent_expiry
    OR $parent.principal_id.capability_profile_id != $profile
    OR !($parent.delegated_capabilities CONTAINS '*' OR $parent.delegated_capabilities CONTAINS 'fs.write')
    OR !($parent.principal_id.delegated_capabilities CONTAINS '*' OR $parent.principal_id.delegated_capabilities CONTAINS 'fs.write')
    OR $grant = NONE OR $grant.account_id != $account OR $grant.principal_id != $parent_principal
    OR $grant.access_space_id != $space OR $grant.resource_id != $resource
    OR $grant.delegation_chain != $parent_chain OR $grant.grant_version != $grant_version
    OR $grant.status != 'active' OR $grant.revoked_at != NONE
    OR $grant.expires_at != $grant_expiry OR ($grant.expires_at != NONE AND $grant.expires_at <= time::now())
    OR !($grant.actions CONTAINS 'create') OR !($grant.capability_ids CONTAINS 'fs.write')
    OR !($parent.delegated_capabilities CONTAINS '*' OR $parent.delegated_capabilities CONTAINS 'fs.read')
    OR !($parent.principal_id.delegated_capabilities CONTAINS '*' OR $parent.principal_id.delegated_capabilities CONTAINS 'fs.read')
    OR $read_grant = NONE OR $read_grant.account_id != $account
    OR $read_grant.principal_id != $parent_principal OR $read_grant.access_space_id != $space
    OR $read_grant.resource_id != $resource OR $read_grant.delegation_chain != $parent_chain
    OR $read_grant.grant_version != $read_grant_version
    OR $read_grant.status != 'active' OR $read_grant.revoked_at != NONE
    OR $read_grant.expires_at != $read_grant_expiry
    OR ($read_grant.expires_at != NONE AND $read_grant.expires_at <= time::now())
    OR !($read_grant.actions CONTAINS 'read') OR !($read_grant.capability_ids CONTAINS 'fs.read')
    OR $resource.resource_kind != 'workspace' OR $resource.external_resource_id != $workspace
    OR $resource.lifecycle_state != 'active' OR $resource.created_by_principal_id.status != 'enabled'
    OR $resource.owner_account_id != $account OR $resource.access_space_id != $space
    OR $resource.policy_version > $grant.policy_version OR $grant.policy_version > $policy
    OR $resource.policy_version > $read_grant.policy_version OR $read_grant.policy_version > $policy
    OR $parent.expires_at <= time::now() OR $expires_at > $parent.expires_at
    OR ($grant.expires_at != NONE AND $expires_at > $grant.expires_at)
    OR ($read_grant.expires_at != NONE AND $expires_at > $read_grant.expires_at)
{ THROW 'HSK-403-PROTECTED-RESOURCE'; };
LET $chain = array::append($parent.delegation_chain, $principal_key);
LET $ancestors = array::append($parent.ancestor_session_ids ?? [], $parent_record);
CREATE $principal SET principal_key = $principal_key, account_id = $account,
    principal_kind = 'spawned_agent', actor_kind = 'agent', actor_id = $actor_id,
    capability_profile_id = $profile, delegated_capabilities = ['fs.write', 'fs.read'],
    status = 'enabled', revocation_epoch = 0, policy_version = $policy,
    created_at = time::now(), updated_at = time::now();
CREATE $child_grant SET account_id = $account, principal_id = $principal,
    access_space_id = $space, resource_id = $resource, actions = ['create'],
    capability_ids = ['fs.write'], delegation_chain = $chain, status = 'active',
    grant_version = 1, policy_version = $grant.policy_version, expires_at = $expires_at,
    revoked_at = NONE, created_at = time::now(), updated_at = time::now();
CREATE $child_read_grant SET account_id = $account, principal_id = $principal,
    access_space_id = $space, resource_id = $resource, actions = ['read'],
    capability_ids = ['fs.read'], delegation_chain = $chain, status = 'active',
    grant_version = 1, policy_version = $read_grant.policy_version, expires_at = $expires_at,
    revoked_at = NONE, created_at = time::now(), updated_at = time::now();
CREATE $child_session SET account_id = $account, principal_id = $principal,
    access_space_id = $space, token_hash = $child_token_hash, channel_binding_hash = $child_channel,
    authentication_strength = 'local_delegated_opaque', delegated_capabilities = ['fs.write', 'fs.read'],
    delegation_chain = $chain, parent_session_id = $parent_record, ancestor_session_ids = $ancestors,
    delegation_grant_edges = [
        { child_grant_id: $child_grant, parent_grant_id: $parent_grant },
        { child_grant_id: $child_read_grant, parent_grant_id: $parent_read_grant }
    ],
    account_revocation_epoch = $account.revocation_epoch, principal_revocation_epoch = 0,
    space_revocation_epoch = $space.revocation_epoch, policy_version = $policy,
    issued_at = time::now(), expires_at = $expires_at, revoked_at = NONE;
COMMIT TRANSACTION;
"#;

impl SurrealStorage {
    pub(crate) async fn authorize_validator_delegation_parent(
        &self,
        registry: &crate::capabilities::CapabilityRegistry,
        credentials: RoleAuthorityAuthInput,
        exact: &crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution,
    ) -> Result<ValidatorDelegationParent, ResourceAuthorityError> {
        let request = AuthorizationRequest {
            session_token: credentials.session_token.clone(),
            channel_binding_hash: Some(credentials.channel_binding_hash.clone()),
            resource_kind: ResourceKind::Workspace,
            external_resource_id: exact.workspace_id.as_str().to_owned(),
            capability_id: "fs.write".to_owned(), action: ResourceAction::Create,
        };
        let parent = capture_owned_canonical_role_auth(self, registry, credentials.clone(), request).await?;
        if &parent.exact != exact {
            return Err(ResourceAuthorityError::InvalidInput("delegation parent scope mismatch"));
        }
        let read_request = AuthorizationRequest {
            session_token: credentials.session_token.clone(),
            channel_binding_hash: Some(credentials.channel_binding_hash.clone()),
            resource_kind: ResourceKind::Workspace,
            external_resource_id: exact.workspace_id.as_str().to_owned(),
            capability_id: "fs.read".to_owned(), action: ResourceAction::Read,
        };
        let mut read = capture_owned_canonical_role_auth(self, registry, credentials, read_request).await?;
        read.requires_workspace_owner = true;
        read.revalidate_for_publication(self, registry).await?;
        if &read.exact != exact || read.authority.resource_id != parent.authority.resource_id
            || read.authority.delegation_chain != parent.authority.delegation_chain {
            return Err(ResourceAuthorityError::InvalidInput("delegation parent READ scope mismatch"));
        }
        Ok(ValidatorDelegationParent { create: parent, read })
    }

    pub(crate) async fn issue_authenticated_validator_launch(
        &self,
        registry: &crate::capabilities::CapabilityRegistry,
        parent: ValidatorDelegationParent,
        instance: &crate::swarm_orchestration::coordinator::RegisteredValidatorInstance,
    ) -> Result<AuthenticatedValidatorLaunchSeal, ResourceAuthorityError> {
        let parent_session_id = instance.parent_session_id().to_owned();
        if parent_session_id.trim().is_empty() {
            return Err(ResourceAuthorityError::InvalidInput("missing registered runtime parent"));
        }
        let ValidatorDelegationParent { create: parent, read: parent_read } = parent;
        let exact = parent.exact.clone();
        parent.revalidate_for_publication(self, registry).await?;
        parent_read.revalidate_for_publication(self, registry).await?;
        let mut expires_at = parent.authority.session_expires_at.clone().into_inner();
        if let Some(expiry) = &parent.authority.grant_expires_at {
            expires_at = expires_at.min(expiry.clone().into_inner());
        }
        if let Some(expiry) = &parent_read.authority.grant_expires_at {
            expires_at = expires_at.min(expiry.clone().into_inner());
        }
        let mut secret = [0_u8; 64];
        getrandom::getrandom(&mut secret)
            .map_err(|error| ResourceAuthorityError::Entropy(error.to_string()))?;
        let child_credentials = RoleAuthorityAuthInput::new(
            hex::encode(&secret[..32]), sha256_hex(&secret[32..]),
        );
        let child_session_id = Uuid::now_v7().to_string();
        let child_principal_id = Uuid::now_v7().to_string();
        let child_session = RecordId::new("authenticated_sessions", child_session_id.clone());
        let child_principal = RecordId::new("principals", child_principal_id.clone());
        let child_grant = RecordId::new("resource_grants", Uuid::now_v7().to_string());
        let child_read_grant = RecordId::new("resource_grants", Uuid::now_v7().to_string());
        let child_token_hash = sha256_hex(child_credentials.session_token.as_bytes());
        let child_channel = child_credentials.channel_binding_hash.clone();
        let parent_token_hash = sha256_hex(parent.credentials.session_token.as_bytes());
        let parent_channel = parent.credentials.channel_binding_hash.clone();
        let authority = parent.authority.clone();
        let read_authority = parent_read.authority.clone();
        let parent_record = authority.session_id.clone();
        let profile = authority.capability_profile_id.clone();
        let policy = parent.session.policy_version;
        let workspace = parent.workspace_id.clone();
        let namespace = self.config().namespace().to_owned();
        let database = self.config().database().to_owned();
        let actor_id = instance.instance_id().to_string();
        self.with_lease(move |client| Box::pin(async move {
            let issuer = client.clone();
            issuer.use_ns(namespace).use_db(database).await?;
            issuer.query(ISSUE_CHILD)
                .bind(("parent_record", parent_record))
                .bind(("parent_grant", authority.grant_id))
                .bind(("expected_parent_read_authority", read_authority.clone()))
                .bind(("parent_read_grant", read_authority.grant_id))
                .bind(("read_grant_version", read_authority.grant_version))
                .bind(("read_grant_expiry", read_authority.grant_expires_at))
                .bind(("account", authority.account_id))
                .bind(("parent_principal", authority.principal_id))
                .bind(("space", authority.access_space_id))
                .bind(("resource", authority.resource_id))
                .bind(("parent_chain", authority.delegation_chain))
                .bind(("parent_expiry", authority.session_expires_at))
                .bind(("grant_expiry", authority.grant_expires_at))
                .bind(("grant_version", authority.grant_version))
                .bind(("parent_token_hash", parent_token_hash))
                .bind(("parent_channel", parent_channel))
                .bind(("profile", profile)).bind(("policy", policy))
                .bind(("workspace", workspace)).bind(("actor_id", actor_id))
                .bind(("principal_key", child_principal_id))
                .bind(("principal", child_principal)).bind(("child_grant", child_grant))
                .bind(("child_read_grant", child_read_grant))
                .bind(("child_session", child_session))
                .bind(("child_token_hash", child_token_hash))
                .bind(("child_channel", child_channel))
                .bind(("expires_at", Datetime::from(expires_at)))
                .await?.check()?;
            Ok(())
        })).await?;
        let request = AuthorizationRequest {
            session_token: child_credentials.session_token.clone(),
            channel_binding_hash: Some(child_credentials.channel_binding_hash.clone()),
            resource_kind: ResourceKind::Workspace,
            external_resource_id: exact.workspace_id.as_str().to_owned(),
            capability_id: "fs.write".to_owned(), action: ResourceAction::Create,
        };
        let auth = match capture_owned_canonical_role_auth(self, registry, child_credentials.clone(), request).await {
            Ok(auth) => auth,
            Err(error) => {
                self.revoke_session(&child_session_id).await?;
                return Err(error);
            }
        };
        let read_request = AuthorizationRequest {
            session_token: child_credentials.session_token.clone(),
            channel_binding_hash: Some(child_credentials.channel_binding_hash.clone()),
            resource_kind: ResourceKind::Workspace,
            external_resource_id: exact.workspace_id.as_str().to_owned(),
            capability_id: "fs.read".to_owned(), action: ResourceAction::Read,
        };
        let child_read = match capture_owned_canonical_role_auth(self, registry, child_credentials, read_request).await {
            Ok(read) => read,
            Err(error) => {
                self.revoke_session(&child_session_id).await?;
                return Err(error);
            }
        };
        if child_read.exact != auth.exact || child_read.authority.resource_id != auth.authority.resource_id {
            self.revoke_session(&child_session_id).await?;
            return Err(ResourceAuthorityError::InvalidInput("issued Validator READ scope mismatch"));
        }
        Ok(AuthenticatedValidatorLaunchSeal {
            auth, registry: registry.clone(), storage: self.clone(), instance: instance.clone(),
            parent_session_id, delegating_scope: exact, expires_at,
        })
    }
}

pub(crate) struct ValidatorAssociationIssuanceInput {
    parent: ValidatorDelegationParent,
    instance: crate::swarm_orchestration::coordinator::RegisteredValidatorInstance,
    invocation: crate::swarm_orchestration::operator_chat::ValidatorAssociationInvocation,
    coordinator: std::sync::Arc<crate::swarm_orchestration::SwarmCoordinator>,
}
impl ValidatorAssociationIssuanceInput {
    pub(crate) fn new(
        parent: ValidatorDelegationParent,
        instance: crate::swarm_orchestration::coordinator::RegisteredValidatorInstance,
        invocation: crate::swarm_orchestration::operator_chat::ValidatorAssociationInvocation,
        coordinator: std::sync::Arc<crate::swarm_orchestration::SwarmCoordinator>,
    ) -> Self { Self { parent, instance, invocation, coordinator } }

    pub(crate) fn requested_task_snapshot(&self) -> &serde_json::Value {
        self.invocation.designation().requested_task_snapshot()
    }
}
// RAM ONLY: append to resource_authority/validator_sessions.rs.
// Requires the separately frozen typed settlement integration; no static map.
use futures::FutureExt;

struct PreparedValidatorChildInventory {
    original_issued_at: chrono::DateTime<Utc>,
    child_principal_id: String,
    child_session_id: String,
    child_create_grant_id: String,
    child_read_grant_id: String,
    association_event_id: String,
    child_credentials: RoleAuthorityAuthInput,
    original_expires_at: chrono::DateTime<Utc>,
}

impl PreparedValidatorChildInventory {
    fn prepare(input: &ValidatorAssociationIssuanceInput)
        -> Result<Self, ResourceAuthorityError> {
        let parent = &input.parent.create;
        let read = &input.parent.read;
        let mut expires_at = parent.authority.session_expires_at.clone().into_inner();
        for expiry in [
            parent.authority.grant_expires_at.as_ref(),
            read.authority.grant_expires_at.as_ref(),
        ].into_iter().flatten() {
            expires_at = expires_at.min(expiry.clone().into_inner());
        }
        // Also retain the actual READ session bound, without renewing anything.
        expires_at = expires_at.min(read.authority.session_expires_at.clone().into_inner());
        let original_issued_at = Utc::now();
        if original_issued_at >= expires_at {
            return Err(ResourceAuthorityError::InvalidInput("original child expiry elapsed"));
        }
        let mut secret = [0_u8; 64];
        getrandom::getrandom(&mut secret)
            .map_err(|error| ResourceAuthorityError::Entropy(error.to_string()))?;
        Ok(Self {
            child_principal_id: Uuid::now_v7().to_string(),
            child_session_id: Uuid::now_v7().to_string(),
            child_create_grant_id: Uuid::now_v7().to_string(),
            child_read_grant_id: Uuid::now_v7().to_string(),
            association_event_id: format!("evt-model-lane-{}", Uuid::now_v7()),
            child_credentials: RoleAuthorityAuthInput::new(
                hex::encode(&secret[..32]), sha256_hex(&secret[32..]),
            ),
            original_expires_at: expires_at,
            original_issued_at,
        })
    }
}

pub(crate) enum ValidatorAssociationFault {
    Authority(ResourceAuthorityError),
    Storage(SurrealStorageError),
    Runtime(crate::swarm_orchestration::SwarmError),
    Panicked(Box<dyn std::any::Any + Send>),
    Codec(handshake_producer_association::AssociationCodecError),
    MissingCanonicalSourceCapture,
    MissingControlledIdentityWorker,
    OriginalDeadlineReached { original_deadline: std::time::Instant },
    DriverJoin(tokio::task::JoinError),
    DriverFinishedWithoutOutcome,
    ControlledIdentityRetained { phase: &'static str },
    CanonicalSourceJson(serde_json::Error),
    TaskSnapshotDecode(serde_json::Error),
    Scope(crate::swarm_orchestration::resource_scope::ScopeDenied),
    Designation(crate::swarm_orchestration::model_lane::ModelLaneError),
}

impl From<SurrealStorageError> for ValidatorAssociationFault {
    fn from(error: SurrealStorageError) -> Self { Self::Storage(error) }
}
impl From<ResourceAuthorityError> for ValidatorAssociationFault {
    fn from(error: ResourceAuthorityError) -> Self { Self::Authority(error) }
}

struct ConfirmedValidatorAssociation {
    seal: std::sync::Arc<AuthenticatedValidatorLaunchSeal>,
    child_read: std::sync::Arc<OwnedCanonicalAuthSnapshot>,
    checked: std::sync::Arc<handshake_producer_association::CheckedProducerAssociation>,
}

struct ValidatorAssociationState {
    inventory_prepared: bool,
    engine_started: bool,
    engine_observation: Option<ValidatorAssociationEngineObservation>,
    driver: Option<tokio::task::JoinHandle<()>>,
    driver_join_observed: bool,
    outcome: Option<Result<ConfirmedValidatorAssociation, ValidatorAssociationFault>>,
    publication_failures: Vec<ValidatorAssociationFault>,
    query_observation_owner: std::sync::Arc<()>,
    query_observations: Vec<Box<dyn std::any::Any + Send>>,
    initial_write: OriginalAssociationWorkerState,
    initial_read: OriginalAssociationWorkerState,
}

struct OwnedValidatorAssociation {
    storage: SurrealStorage,
    registry: crate::capabilities::CapabilityRegistry,
    input: ValidatorAssociationIssuanceInput,
    original_deadline: std::time::Instant,
    inventory: std::sync::OnceLock<PreparedValidatorChildInventory>,
    state: std::sync::Mutex<ValidatorAssociationState>,
    ready: tokio::sync::Notify,
    controlled_identity: std::sync::Mutex<ControlledValidatorIdentityState>,
    requested_task: std::sync::OnceLock<std::sync::Arc<ValidatorAssociationTaskSnapshotV1>>,
    task_mapping: std::sync::OnceLock<ValidatorAssociationTaskMapping>,
    designation: std::sync::OnceLock<std::sync::Arc<ValidatorAssociationDesignation>>,
}

pub(crate) struct ValidatorAssociationIssuanceHandle {
    retention: ValidatorAssociationTransactionRetention,
}

pub(crate) struct ValidatorAssociationRetainedFailure {
    handle: ValidatorAssociationIssuanceHandle,
    publication_failure_index: Option<usize>,
}

pub(crate) struct ValidatorAssociationReserveRejection {
    input: ValidatorAssociationIssuanceInput,
    error: SurrealStorageError,
}

impl ValidatorAssociationReserveRejection {
    pub(crate) fn into_parts(self) -> (ValidatorAssociationIssuanceInput, SurrealStorageError) {
        (self.input, self.error)
    }
}

impl OwnedValidatorAssociation {
    fn same_input(&self, incoming: &ValidatorAssociationIssuanceInput) -> bool {
        let a = &self.input;
        let b = incoming;
        a.invocation.designation() == b.invocation.designation()
            && a.requested_task_snapshot() == b.requested_task_snapshot()
            && std::sync::Arc::ptr_eq(&a.coordinator, &b.coordinator)
            && a.instance.instance_id() == b.instance.instance_id()
            && a.instance.spawn_generation() == b.instance.spawn_generation()
            && std::sync::Arc::ptr_eq(a.invocation.source(), b.invocation.source())
            && a.invocation.selection_json() == b.invocation.selection_json()
            && a.invocation.committed_launch_json() == b.invocation.committed_launch_json()
            && a.invocation.original_launch_json() == b.invocation.original_launch_json()
            && a.invocation.canonical_run_trace_json() == b.invocation.canonical_run_trace_json()
            && a.invocation.canonical_launch_plan() == b.invocation.canonical_launch_plan()
            && a.parent.create.authority == b.parent.create.authority
            && a.parent.read.authority == b.parent.read.authority
            && a.parent.create.credentials.session_token == b.parent.create.credentials.session_token
            && a.parent.create.credentials.channel_binding_hash == b.parent.create.credentials.channel_binding_hash
            && a.parent.read.credentials.session_token == b.parent.read.credentials.session_token
            && a.parent.read.credentials.channel_binding_hash == b.parent.read.credentials.channel_binding_hash
    }

    fn require_original_deadline(&self) -> Result<(), ValidatorAssociationFault> {
        if std::time::Instant::now() >= self.original_deadline {
            return Err(ValidatorAssociationFault::OriginalDeadlineReached {
                original_deadline: self.original_deadline,
            });
        }
        Ok(())
    }

    async fn require_current_sources(&self) -> Result<(), ValidatorAssociationFault> {
        self.require_original_deadline()?;
        // This exact mapper revalidation checks BOTH original Create and Read
        // source snapshots and the same original task Arc/order/full tuple.
        self.revalidate_original_task_mapping().await?;
        let inventory = self.inventory.get().ok_or(
            ValidatorAssociationFault::MissingCanonicalSourceCapture,
        )?;
        if Utc::now() >= inventory.original_expires_at {
            return Err(ResourceAuthorityError::InvalidInput(
                "original child expiry elapsed",
            ).into());
        }
        self.require_current_runtime_source()
    }

    async fn run_original_attempt(owner: std::sync::Arc<Self>, retention: ValidatorAssociationTransactionRetention)
        -> Result<ConfirmedValidatorAssociation, ValidatorAssociationFault> {
        owner.require_original_deadline()?;
        let inventory = PreparedValidatorChildInventory::prepare(&owner.input)?;
        if owner.inventory.set(inventory).is_err() {
            return Err(ResourceAuthorityError::InvalidInput("child inventory already prepared").into());
        }
        owner.state.lock().unwrap_or_else(|p| p.into_inner()).inventory_prepared = true;
        owner.prepare_original_task_mapping().await?;
        owner.require_current_sources().await?;
        let canonical = Self::issue_original_controlled_child(std::sync::Arc::clone(&owner)).await?;
        let (seal, child_read) = owner.authorize_original_controlled_child().await?;
        if !std::sync::Arc::ptr_eq(&canonical, &owner.committed_canonical_source()?) {
            return Err(ResourceAuthorityError::InvalidInput("association controlled canonical owner changed").into());
        }
        let provider = super::super::model_lane::SurrealModelLaneStore::initialize(owner.storage.clone()).await?;
        owner.require_current_sources().await?;
        let write_owner = std::sync::Arc::clone(&owner);
        let write_provider = provider.clone();
        let write_wait = owner.storage.with_record_user_scope(seal.auth.record_user_scope(),
            owner.storage.with_owned_validator_association_authenticated_transaction(
                retention.operation_id(), super::super::ContextOperationCancellation::new(), retention.clone(),
                move |transaction| Box::pin(async move {
                    SurrealStorage::with_operation_deadline(write_owner.original_deadline, async {
                    write_owner.require_original_deadline()?;
                    write_owner.require_current_runtime_source()?;
                    let issuer = write_owner.capture_create_read_seal(&transaction).await?;
                    let readback = std::sync::Arc::new(write_provider.put_validator_association(&transaction, &issuer).await?);
                    write_owner.state.lock().unwrap_or_else(|p| p.into_inner()).initial_write.write_readback = Some(std::sync::Arc::clone(&readback));
                    issuer.revalidate().await?;
                    write_owner.require_original_deadline()?;
                    write_owner.require_current_runtime_source()?;
                    Ok::<_, ValidatorAssociationFault>(readback)
                    }).await
                }),
            )
        ).await;
        owner.state.lock().unwrap_or_else(|p| p.into_inner()).initial_write.engine_wait = Some(Box::new(write_wait));
        let write_readback = {
            let state = owner.state.lock().unwrap_or_else(|p| p.into_inner());
            if state.engine_observation != Some(ValidatorAssociationEngineObservation::CommitObserved)
                || !matches!(state.initial_write.callback.as_ref(), Some(Ok(_))) {
                return Err(SurrealStorageError::TransactionSettlementPending { operation_id: retention.operation_id() }.into());
            }
            state.initial_write.write_readback.as_ref().map(std::sync::Arc::clone)
                .ok_or(SurrealStorageError::TransactionSettlementPending { operation_id: retention.operation_id() })?
        };
        owner.require_current_sources().await?;
        let read_owner = std::sync::Arc::clone(&owner);
        let read_provider = provider.clone();
        let expected = std::sync::Arc::clone(&write_readback);
        let read_wait = owner.storage.with_record_user_scope(child_read.record_user_scope(),
            owner.storage.with_owned_validator_association_readback_transaction(
                retention.operation_id(), super::super::ContextOperationCancellation::new(), retention.clone(),
                move |transaction| Box::pin(async move {
                    SurrealStorage::with_operation_deadline(read_owner.original_deadline, async {
                    read_owner.require_original_deadline()?;
                    read_owner.require_current_runtime_source()?;
                    let guard = read_owner.capture_readback_guard(&transaction).await?;
                    let checked = std::sync::Arc::new(read_provider.readback_validator_association(&guard, &expected).await?);
                    read_owner.state.lock().unwrap_or_else(|p| p.into_inner()).initial_read.checked = Some(std::sync::Arc::clone(&checked));
                    guard.revalidate().await?;
                    read_owner.require_original_deadline()?;
                    read_owner.require_current_runtime_source()?;
                    Ok::<_, ValidatorAssociationFault>(checked)
                    }).await
                }),
            )
        ).await;
        owner.state.lock().unwrap_or_else(|p| p.into_inner()).initial_read.engine_wait = Some(Box::new(read_wait));
        owner.require_current_sources().await?;
        seal.auth.revalidate_for_publication(&owner.storage, &owner.registry).await?;
        child_read.revalidate_for_publication(&owner.storage, &owner.registry).await?;
        owner.require_current_runtime_source()?;
        let checked = {
            let state = owner.state.lock().unwrap_or_else(|p| p.into_inner());
            if state.initial_read.observation != Some(ValidatorAssociationEngineObservation::CommitObserved)
                || !matches!(state.initial_read.callback.as_ref(), Some(Ok(_))) {
                return Err(SurrealStorageError::TransactionSettlementPending { operation_id: retention.operation_id() }.into());
            }
            state.initial_read.checked.as_ref().map(std::sync::Arc::clone)
                .ok_or(SurrealStorageError::TransactionSettlementPending { operation_id: retention.operation_id() })?
        };
        Ok(ConfirmedValidatorAssociation { seal, child_read, checked })
    }

    fn start_original_driver(owner: &std::sync::Arc<Self>,
        retention: ValidatorAssociationTransactionRetention) {
        let mut state = owner.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.driver.is_some() || state.outcome.is_some() { return; }
        let runtime = match tokio::runtime::Handle::try_current() {
            Ok(runtime) => runtime,
            Err(_) => {
                state.outcome = Some(Err(ResourceAuthorityError::InvalidInput(
                    "original association runtime unavailable",
                ).into()));
                owner.ready.notify_waiters();
                return;
            }
        };
        let worker_owner = std::sync::Arc::clone(owner);
        let launched = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            runtime.spawn(async move {
                let result = std::panic::AssertUnwindSafe(SurrealStorage::with_operation_deadline(
                    worker_owner.original_deadline,
                    Self::run_original_attempt(std::sync::Arc::clone(&worker_owner), retention),
                )).catch_unwind().await;
                let outcome = match result {
                    Ok(outcome) => outcome,
                    Err(payload) => Err(ValidatorAssociationFault::Panicked(payload)),
                };
                let mut state = worker_owner.state.lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                // No caller takes/replaces the result and no second driver runs.
                state.outcome = Some(outcome);
                drop(state);
                worker_owner.ready.notify_waiters();
            })
        }));
        match launched {
            Ok(driver) => state.driver = Some(driver),
            Err(payload) => {
                state.outcome = Some(Err(ValidatorAssociationFault::Panicked(payload)));
                owner.ready.notify_waiters();
            }
        }
    }
}

impl ValidatorAssociationIssuanceHandle {
    fn poll_original_outcome(&self, context: &mut std::task::Context<'_>)
        -> std::task::Poll<Result<(
            std::sync::Arc<AuthenticatedValidatorLaunchSeal>,
            std::sync::Arc<OwnedCanonicalAuthSnapshot>,
        ), ValidatorAssociationRetainedFailure>> {
        self.retention.poll_initial_worker_joins(context);
            let mut state = self.retention.frame.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            let mut notify_terminal = false;
            if state.outcome.is_none() && !state.driver_join_observed {
                if let Some(driver) = state.driver.as_mut() {
                    match std::future::Future::poll(std::pin::Pin::new(driver), context) {
                        std::task::Poll::Ready(joined) => {
                            state.driver_join_observed = true;
                            // Normal driver completion publishes before finishing.
                            // A cancelled task may never reach that publication.
                            if state.outcome.is_none() {
                                state.outcome = Some(Err(match joined {
                                    Err(error) => ValidatorAssociationFault::DriverJoin(error),
                                    Ok(()) => ValidatorAssociationFault::DriverFinishedWithoutOutcome,
                                }));
                                notify_terminal = true;
                            }
                        }
                        std::task::Poll::Pending => {}
                    }
                }
            }
            let observed = match state.outcome.as_ref() {
                Some(Ok(confirmed)) => std::task::Poll::Ready(Ok((
                    std::sync::Arc::clone(&confirmed.seal),
                    std::sync::Arc::clone(&confirmed.child_read),
                ))),
                Some(Err(_)) => std::task::Poll::Ready(Err(ValidatorAssociationRetainedFailure {
                    handle: Self { retention: self.retention.clone() },
                    publication_failure_index: None,
                })),
                None => std::task::Poll::Pending,
            };
            drop(state);
            if notify_terminal { self.retention.frame.ready.notify_waiters(); }
            observed
    }

    fn retain_publication_failure(&self, fault: ValidatorAssociationFault)
        -> ValidatorAssociationRetainedFailure {
        let mut state = self.retention.frame.state.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let index = state.publication_failures.len();
        state.publication_failures.push(fault);
        ValidatorAssociationRetainedFailure {
            handle: Self { retention: self.retention.clone() },
            publication_failure_index: Some(index),
        }
    }

    pub(crate) async fn commit_and_readback(&mut self, storage: &SurrealStorage)
        -> Result<std::sync::Arc<AuthenticatedValidatorLaunchSeal>,
            ValidatorAssociationRetainedFailure> {
        let owner = &self.retention.frame;
        if owner.storage.resource_access_lifecycle() != storage.resource_access_lifecycle() {
            return Err(self.retain_publication_failure(ResourceAuthorityError::InvalidInput(
                "original association storage owner differs",
            ).into()));
        }
        // Poll the SAME retained task ONCE before deadline admission. This
        // observes an already cancelled JoinHandle even after deadline expiry.
        let existing = std::future::poll_fn(|context| std::task::Poll::Ready(
            self.poll_original_outcome(context),
        )).await;
        let observed = match existing {
            std::task::Poll::Ready(observed) => Some(observed?),
            std::task::Poll::Pending => None,
        };
        if observed.is_none() {
            if let Err(error) = owner.require_original_deadline() {
                return Err(self.retain_publication_failure(error));
            }
            OwnedValidatorAssociation::start_original_driver(owner, self.retention.clone());
        }
        // Each same-original waiter registers its OWN fan-out notification
        // BEFORE checking the shared outcome. JoinHandle has only one waker.
        let notified = owner.ready.notified();
        tokio::pin!(notified);
        notified.as_mut().enable();
        let wait_original = std::future::poll_fn(|context| {
            loop {
                match self.poll_original_outcome(context) {
                    std::task::Poll::Ready(observed) => return std::task::Poll::Ready(observed),
                    std::task::Poll::Pending => {}
                }
                match std::future::Future::poll(notified.as_mut(), context) {
                    std::task::Poll::Pending => return std::task::Poll::Pending,
                    std::task::Poll::Ready(()) => {
                        notified.set(owner.ready.notified());
                        notified.as_mut().enable();
                    }
                }
            }
        });
        let (seal, child_read) = if let Some(observed) = observed {
            observed
        } else {
            match tokio::time::timeout_at(
                tokio::time::Instant::from_std(owner.original_deadline), wait_original,
            ).await {
                Ok(observed) => observed?,
                Err(_) => return Err(self.retain_publication_failure(
                    ValidatorAssociationFault::OriginalDeadlineReached {
                        original_deadline: owner.original_deadline,
                    },
                )),
            }
        };
        // Clamp EVERY revalidation query to the SAME actual original deadline.
        // Timeout of the borrowed wait never aborts/drops the original driver.
        let current = SurrealStorage::with_operation_deadline(owner.original_deadline, async {
            owner.require_original_deadline()?;
            seal.revalidate(storage).await?;
            owner.require_original_deadline()?;
            child_read.revalidate_for_publication(storage, &owner.registry).await?;
            owner.require_current_sources().await?;
            Ok::<_, ValidatorAssociationFault>(())
        }).await;
        if let Err(error) = current {
            return Err(self.retain_publication_failure(error));
        }
        Ok(seal)
    }
}

impl ValidatorAssociationRetainedFailure {
    pub(crate) fn with_fault<R>(&self,
        read: impl FnOnce(&ValidatorAssociationFault) -> R) -> Option<R> {
        let state = self.handle.retention.frame.state.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(index) = self.publication_failure_index {
            return state.publication_failures.get(index).map(read);
        }
        match state.outcome.as_ref() { Some(Err(error)) => Some(read(error)), _ => None }
    }
    pub(crate) fn into_handle(self) -> ValidatorAssociationIssuanceHandle { self.handle }
}

impl SurrealStorage {
    pub(crate) fn reserve_validator_association_issuance(
        &self,
        registry: &crate::capabilities::CapabilityRegistry,
        input: ValidatorAssociationIssuanceInput,
    ) -> Result<ValidatorAssociationIssuanceHandle, ValidatorAssociationReserveRejection> {
        let operation_id = input.invocation.operation_id();
        let reject = |input, error| ValidatorAssociationReserveRejection { input, error };
        if operation_id.is_nil() {
            return Err(reject(input, SurrealStorageError::TransactionRetentionMismatch { operation_id }));
        }
        let mut settlements = self.inner.transaction_settlements.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(existing) = settlements.get(&operation_id) {
            return match existing {
                Some(super::super::AuthenticatedTransactionRetention::ValidatorAssociation(retention))
                    if retention.frame.same_input(&input) =>
                    Ok(ValidatorAssociationIssuanceHandle { retention: retention.clone() }),
                _ => Err(reject(input,
                    SurrealStorageError::TransactionSettlementPending { operation_id })),
            };
        }
        // Only a NEW original frame consumes fresh admission budget.
        // Same-input redelivery above reuses its original deadline/result.
        let budget = self.effective_statement_timeout();
        if budget.is_zero() {
            return Err(reject(input, SurrealStorageError::StatementBudgetExhausted { budget_ms: 0 }));
        }
        let frame = std::sync::Arc::new(OwnedValidatorAssociation {
            storage: self.clone(), registry: registry.clone(), input,
            original_deadline: std::time::Instant::now() + budget,
            inventory: std::sync::OnceLock::new(),
            state: std::sync::Mutex::new(ValidatorAssociationState {
                inventory_prepared: false, engine_started: false,
                engine_observation: None, driver: None, driver_join_observed: false, outcome: None,
                publication_failures: Vec::new(),
                query_observation_owner: std::sync::Arc::new(()),
                query_observations: Vec::new(),
                initial_write: Default::default(),
                initial_read: Default::default(),
            }),
            ready: tokio::sync::Notify::new(),
            controlled_identity: std::sync::Mutex::new(Default::default()),
            requested_task: std::sync::OnceLock::new(),
            task_mapping: std::sync::OnceLock::new(),
            designation: std::sync::OnceLock::new(),
        });
        let retention = ValidatorAssociationTransactionRetention { frame };
        settlements.insert(operation_id, Some(
            super::super::AuthenticatedTransactionRetention::ValidatorAssociation(retention.clone()),
        ));
        Ok(ValidatorAssociationIssuanceHandle { retention })
    }
}




#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ValidatorAssociationTaskSnapshotV1 {
    format_version: u32,
    task_id: String,
    claimed_revision: u64,
    operations: Box<[ValidatorAssociationRequestedOperationV1]>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ValidatorAssociationTaskToolV1 {
    DelegateWorkspaceCreate,
    DelegateWorkspaceRead,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ValidatorAssociationRequestedOperationV1 {
    tool: ValidatorAssociationTaskToolV1,
    resource_kind: ResourceKind,
    external_resource_id: String,
    action: ResourceAction,
    capability_id: String,
}

impl ValidatorAssociationTaskSnapshotV1 {
    pub(crate) fn format_version(&self) -> u32 { self.format_version }
    pub(crate) fn task_id(&self) -> &str { &self.task_id }
    pub(crate) fn claimed_revision(&self) -> u64 { self.claimed_revision }
    pub(crate) fn operations(&self) -> &[ValidatorAssociationRequestedOperationV1] {
        &self.operations
    }
}

impl ValidatorAssociationRequestedOperationV1 {
    pub(crate) fn tool(&self) -> ValidatorAssociationTaskToolV1 { self.tool }
    pub(crate) fn resource_kind(&self) -> ResourceKind { self.resource_kind }
    pub(crate) fn external_resource_id(&self) -> &str { &self.external_resource_id }
    pub(crate) fn action(&self) -> ResourceAction { self.action }
    pub(crate) fn capability_id(&self) -> &str { &self.capability_id }
}

pub(crate) struct ValidatorAssociationTaskMapping {
    requested: std::sync::Arc<ValidatorAssociationTaskSnapshotV1>,
    exact: crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution,
    create_authority: RegistryPublicationAuthority,
    read_authority: RegistryPublicationAuthority,
    ordered_tools: Box<[ValidatorAssociationTaskToolV1]>,
}

impl ValidatorAssociationTaskMapping {
    pub(crate) fn requested(&self) -> &ValidatorAssociationTaskSnapshotV1 {
        &self.requested
    }

    pub(crate) fn matches_requested(
        &self,
        requested: &std::sync::Arc<ValidatorAssociationTaskSnapshotV1>,
    ) -> bool {
        std::sync::Arc::ptr_eq(&self.requested, requested)
            && self.requested.as_ref() == requested.as_ref()
    }

    pub(crate) fn ordered_tools(&self) -> &[ValidatorAssociationTaskToolV1] {
        &self.ordered_tools
    }
}

struct ValidatorAssociationMappedSource<'a> {
    requested: &'a ValidatorAssociationRequestedOperationV1,
    canonical: &'a OwnedCanonicalAuthSnapshot,
}

fn association_task_denied() -> ResourceAuthorityError {
    ResourceAuthorityError::InvalidInput(
        "requested association task differs from server policy",
    )
}

impl ValidatorDelegationParent {
    fn association_task_source(
        &self,
        tool: ValidatorAssociationTaskToolV1,
    ) -> (&OwnedCanonicalAuthSnapshot, ResourceAction, &'static str) {
        match tool {
            ValidatorAssociationTaskToolV1::DelegateWorkspaceCreate =>
                (&self.create, ResourceAction::Create, "fs.write"),
            ValidatorAssociationTaskToolV1::DelegateWorkspaceRead =>
                (&self.read, ResourceAction::Read, "fs.read"),
        }
    }

    fn validate_association_task_operations(
        &self,
        exact: &crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution,
        requested: &ValidatorAssociationTaskSnapshotV1,
    ) -> Result<Box<[ValidatorAssociationTaskToolV1]>, ResourceAuthorityError> {
        if requested.format_version != 1
            || requested.task_id.trim().is_empty()
            || requested.operations.len() != 2
            || &self.create.exact != exact
            || &self.read.exact != exact
        {
            return Err(association_task_denied());
        }
        let mut seen = 0_u8;
        let mut ordered = Vec::with_capacity(2);
        for operation in requested.operations.iter() {
            let bit = match operation.tool {
                ValidatorAssociationTaskToolV1::DelegateWorkspaceCreate => 1_u8,
                ValidatorAssociationTaskToolV1::DelegateWorkspaceRead => 2_u8,
            };
            let (source, action, capability) = self.association_task_source(operation.tool);
            if seen & bit != 0
                || operation.resource_kind != ResourceKind::Workspace
                || operation.external_resource_id != exact.workspace_id.as_str()
                || operation.action != action
                || operation.capability_id != capability
                || source.workspace_id != exact.workspace_id.as_str()
                || source.authority.resource_kind != ResourceKind::Workspace.as_str()
                || source.action != action
                || source.capability_id != capability
            {
                return Err(association_task_denied());
            }
            seen |= bit;
            ordered.push(operation.tool);
        }
        if seen != 3 {
            return Err(association_task_denied());
        }
        Ok(ordered.into_boxed_slice())
    }

    pub(crate) async fn map_requested_association_task(
        &self,
        storage: &SurrealStorage,
        registry: &crate::capabilities::CapabilityRegistry,
        exact: &crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution,
        requested: std::sync::Arc<ValidatorAssociationTaskSnapshotV1>,
    ) -> Result<ValidatorAssociationTaskMapping, ResourceAuthorityError> {
        let ordered_tools = self.validate_association_task_operations(exact, &requested)?;
        self.create.revalidate_for_publication(storage, registry).await?;
        self.read.revalidate_for_publication(storage, registry).await?;
        Ok(ValidatorAssociationTaskMapping {
            requested,
            exact: exact.clone(),
            create_authority: self.create.authority.clone(),
            read_authority: self.read.authority.clone(),
            ordered_tools,
        })
    }

    pub(crate) async fn revalidate_association_task_mapping(
        &self,
        storage: &SurrealStorage,
        registry: &crate::capabilities::CapabilityRegistry,
        exact: &crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution,
        requested: &std::sync::Arc<ValidatorAssociationTaskSnapshotV1>,
        mapping: &ValidatorAssociationTaskMapping,
    ) -> Result<(), ResourceAuthorityError> {
        if !mapping.matches_requested(requested)
            || &mapping.exact != exact
            || mapping.create_authority != self.create.authority
            || mapping.read_authority != self.read.authority
            || self.validate_association_task_operations(exact, requested)?
                != mapping.ordered_tools
        {
            return Err(association_task_denied());
        }
        self.create.revalidate_for_publication(storage, registry).await?;
        self.read.revalidate_for_publication(storage, registry).await?;
        Ok(())
    }

    fn association_task_sources_in_requested_order<'a>(
        &'a self,
        mapping: &'a ValidatorAssociationTaskMapping,
    ) -> Result<Vec<ValidatorAssociationMappedSource<'a>>, ResourceAuthorityError> {
        if mapping.create_authority != self.create.authority
            || mapping.read_authority != self.read.authority
            || self.validate_association_task_operations(&mapping.exact, &mapping.requested)?
                != mapping.ordered_tools
        {
            return Err(association_task_denied());
        }
        Ok(mapping.requested.operations.iter().map(|requested| {
            let (canonical, _, _) = self.association_task_source(requested.tool);
            ValidatorAssociationMappedSource { requested, canonical }
        }).collect())
    }
}


// Shared RAM ABI; definitions live in resource_authority/validator_sessions.rs.
// Main resource_authority.rs needs only the crate-private re-export later.
#[derive(Clone)]
pub(crate) struct ValidatorAssociationTransactionRetention {
    frame: std::sync::Arc<OwnedValidatorAssociation>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ValidatorAssociationEngineObservation {
    Unobserved,
    CommitObserved,
    CancelNoEffectObserved,
    CallbackPanicRollbackObserved,
}

impl ValidatorAssociationTransactionRetention {
    pub(crate) fn operation_id(&self) -> Uuid {
        self.frame.input.invocation.operation_id()
    }

    pub(crate) fn original_deadline(&self) -> std::time::Instant {
        self.frame.original_deadline
    }

    pub(crate) fn matches_retention(&self, other: &Self) -> bool {
        self.operation_id() == other.operation_id()
            && std::sync::Arc::ptr_eq(&self.frame, &other.frame)
    }

    // Called under the EXISTING transaction_settlements write lock, only when
    // the resident ValidatorAssociation variant equals incoming retention.
    // This is the ONE start transition; no second invocation/engine task.
    pub(crate) fn enter_engine_once(&self) -> Result<(), SurrealStorageError> {
        let mut state = self.frame.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if !state.inventory_prepared || state.engine_started {
            return Err(SurrealStorageError::TransactionSettlementPending {
                operation_id: self.operation_id(),
            });
        }
        state.engine_started = true;
        Ok(())
    }

    // Record ONLY the actually observed transaction-worker settlement.
    // Never remove this retained variant on callback failure, caller drop,
    // engine Unknown, rollback or success before owned canonical readback.
    pub(crate) fn observe_engine(
        &self,
        observed: ValidatorAssociationEngineObservation,
    ) -> Result<(), SurrealStorageError> {
        let mut state = self.frame.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if !state.engine_started {
            return Err(SurrealStorageError::TransactionRetentionMismatch {
                operation_id: self.operation_id(),
            });
        }
        if let Some(previous) = state.engine_observation {
            if previous != ValidatorAssociationEngineObservation::Unobserved && previous != observed {
                return Err(SurrealStorageError::TransactionRetentionMismatch {
                    operation_id: self.operation_id(),
                });
            }
        }
        state.engine_observation = Some(observed);
        Ok(())
    }
}



const CONTROLLED_VALIDATOR_GATE: &str = r#"LET $parent = (SELECT * FROM ONLY $parent_record);
LET $grant = (SELECT * FROM ONLY $parent_grant);
LET $read_grant = (SELECT * FROM ONLY $parent_read_grant);
LET $read_authority = {
    grant_id: $read_grant.id, resource_id: $read_grant.resource_id,
    account_id: $read_grant.account_id, principal_id: $read_grant.principal_id,
    actor_kind: $read_grant.principal_id.actor_kind, actor_id: $read_grant.principal_id.actor_id,
    session_id: $parent_record, access_space_id: $read_grant.access_space_id,
    grant_version: $read_grant.grant_version,
    policy_versions: [$read_grant.policy_version, $resource.policy_version, $account.policy_version,
        $parent_principal.policy_version, $space.policy_version, $parent.policy_version],
    revocation_epochs: [$account.revocation_epoch, $parent_principal.revocation_epoch, $space.revocation_epoch,
        $parent.account_revocation_epoch, $parent.principal_revocation_epoch, $parent.space_revocation_epoch],
    actions: $read_grant.actions, capability_ids: $read_grant.capability_ids,
    delegation_chain: $read_grant.delegation_chain, session_capabilities: $parent.delegated_capabilities,
    principal_capabilities: $parent_principal.delegated_capabilities,
    capability_profile_id: $parent_principal.capability_profile_id,
    statuses: [$read_grant.status, $resource.lifecycle_state, $account.status, $parent_principal.status, $space.status],
    resource_kind: $resource.resource_kind, grant_expires_at: $read_grant.expires_at,
    session_expires_at: $parent.expires_at
};
LET $create_authority = {
    grant_id: $grant.id, resource_id: $grant.resource_id,
    account_id: $grant.account_id, principal_id: $grant.principal_id,
    actor_kind: $grant.principal_id.actor_kind, actor_id: $grant.principal_id.actor_id,
    session_id: $parent_record, access_space_id: $grant.access_space_id,
    grant_version: $grant.grant_version,
    policy_versions: [$grant.policy_version, $resource.policy_version, $account.policy_version,
        $parent_principal.policy_version, $space.policy_version, $parent.policy_version],
    revocation_epochs: [$account.revocation_epoch, $parent_principal.revocation_epoch, $space.revocation_epoch,
        $parent.account_revocation_epoch, $parent.principal_revocation_epoch, $parent.space_revocation_epoch],
    actions: $grant.actions, capability_ids: $grant.capability_ids,
    delegation_chain: $grant.delegation_chain, session_capabilities: $parent.delegated_capabilities,
    principal_capabilities: $parent_principal.delegated_capabilities,
    capability_profile_id: $parent_principal.capability_profile_id,
    statuses: [$grant.status, $resource.lifecycle_state, $account.status, $parent_principal.status, $space.status],
    resource_kind: $resource.resource_kind, grant_expires_at: $grant.expires_at,
    session_expires_at: $parent.expires_at
};
IF $parent = NONE
    OR $account.account_role != 'Owner' OR $parent_principal.principal_kind != 'human_account'
    OR $read_authority != $expected_parent_read_authority
    OR $create_authority != $expected_parent_create_authority
    OR $parent.token_hash != $parent_token_hash
    OR $parent.channel_binding_hash != $parent_channel
    OR $parent.account_id != $account OR $parent.principal_id != $parent_principal
    OR $parent.access_space_id != $space OR $parent.delegation_chain != $parent_chain
    OR $parent.policy_version != $policy OR $parent.expires_at != $parent_expiry
    OR $parent.principal_id.capability_profile_id != $profile
    OR !($parent.delegated_capabilities CONTAINS '*' OR $parent.delegated_capabilities CONTAINS 'fs.write')
    OR !($parent.principal_id.delegated_capabilities CONTAINS '*' OR $parent.principal_id.delegated_capabilities CONTAINS 'fs.write')
    OR $grant = NONE OR $grant.account_id != $account OR $grant.principal_id != $parent_principal
    OR $grant.access_space_id != $space OR $grant.resource_id != $resource
    OR $grant.delegation_chain != $parent_chain OR $grant.grant_version != $grant_version
    OR $grant.status != 'active' OR $grant.revoked_at != NONE
    OR $grant.expires_at != $grant_expiry OR ($grant.expires_at != NONE AND $grant.expires_at <= time::now())
    OR !($grant.actions CONTAINS 'create') OR !($grant.capability_ids CONTAINS 'fs.write')
    OR !($parent.delegated_capabilities CONTAINS '*' OR $parent.delegated_capabilities CONTAINS 'fs.read')
    OR !($parent.principal_id.delegated_capabilities CONTAINS '*' OR $parent.principal_id.delegated_capabilities CONTAINS 'fs.read')
    OR $read_grant = NONE OR $read_grant.account_id != $account
    OR $read_grant.principal_id != $parent_principal OR $read_grant.access_space_id != $space
    OR $read_grant.resource_id != $resource OR $read_grant.delegation_chain != $parent_chain
    OR $read_grant.grant_version != $read_grant_version
    OR $read_grant.status != 'active' OR $read_grant.revoked_at != NONE
    OR $read_grant.expires_at != $read_grant_expiry
    OR ($read_grant.expires_at != NONE AND $read_grant.expires_at <= time::now())
    OR !($read_grant.actions CONTAINS 'read') OR !($read_grant.capability_ids CONTAINS 'fs.read')
    OR $resource.resource_kind != 'workspace' OR $resource.external_resource_id != $workspace
    OR $resource.lifecycle_state != 'active' OR $resource.created_by_principal_id.status != 'enabled'
    OR $resource.owner_account_id != $account OR $resource.access_space_id != $space
    OR $resource.policy_version > $grant.policy_version OR $grant.policy_version > $policy
    OR $resource.policy_version > $read_grant.policy_version OR $read_grant.policy_version > $policy
    OR $parent.expires_at <= time::now() OR $expires_at > $parent.expires_at
    OR ($grant.expires_at != NONE AND $expires_at > $grant.expires_at)
    OR ($read_grant.expires_at != NONE AND $expires_at > $read_grant.expires_at)
{ THROW 'HSK-403-PROTECTED-RESOURCE'; };

IF !fn::mt109_session_lineage_live($parent_record) { THROW 'HSK-403-PROTECTED-RESOURCE'; };
"#;

const CONTROLLED_VALIDATOR_CAPTURE: &str = r#"SELECT VALUE {
    account: (SELECT * OMIT password_verifier FROM ONLY $account),
    space: (SELECT * FROM ONLY $space),
    resource: (SELECT * FROM ONLY $resource),
    ordered_chain: array::map(array::append($parent_record.ancestor_session_ids ?? [], $parent_record), |$node| {
        session: (SELECT * FROM ONLY $node),
        principal: (SELECT * FROM ONLY $node.principal_id),
        grant_edges: array::map($node.delegation_grant_edges ?? [], |$edge| {
            edge: $edge,
            child_grant: (SELECT * FROM ONLY $edge.child_grant_id),
            parent_grant: (SELECT * FROM ONLY $edge.parent_grant_id),
            child_resource: (SELECT * FROM ONLY $edge.child_grant_id.resource_id),
            parent_resource: (SELECT * FROM ONLY $edge.parent_grant_id.resource_id)
        })
    }),
    principal_ids: array::map(array::append($parent_record.ancestor_session_ids ?? [], $parent_record), |$node| $node.principal_id),
    session_ids: array::append($parent_record.ancestor_session_ids ?? [], $parent_record),
    create_grant: (SELECT * FROM ONLY $parent_grant),
    read_grant: (SELECT * FROM ONLY $parent_read_grant)
} FROM $parent_record;"#;

const CONTROLLED_VALIDATOR_CREATE: &str = r#"LET $chain = array::append($parent.delegation_chain, $principal_key);
LET $ancestors = array::append($parent.ancestor_session_ids ?? [], $parent_record);
CREATE $principal SET principal_key = $principal_key, account_id = $account,
    principal_kind = 'spawned_agent', actor_kind = 'agent', actor_id = $actor_id,
    capability_profile_id = $profile, delegated_capabilities = ['fs.write', 'fs.read'],
    status = 'enabled', revocation_epoch = 0, policy_version = $policy,
    created_at = $issued_at, updated_at = $issued_at;
CREATE $child_grant SET account_id = $account, principal_id = $principal,
    access_space_id = $space, resource_id = $resource, actions = ['create'],
    capability_ids = ['fs.write'], delegation_chain = $chain, status = 'active',
    grant_version = 1, policy_version = $grant.policy_version, expires_at = $expires_at,
    revoked_at = NONE, created_at = $issued_at, updated_at = $issued_at;
CREATE $child_read_grant SET account_id = $account, principal_id = $principal,
    access_space_id = $space, resource_id = $resource, actions = ['read'],
    capability_ids = ['fs.read'], delegation_chain = $chain, status = 'active',
    grant_version = 1, policy_version = $read_grant.policy_version, expires_at = $expires_at,
    revoked_at = NONE, created_at = $issued_at, updated_at = $issued_at;
CREATE $child_session SET account_id = $account, principal_id = $principal,
    access_space_id = $space, token_hash = $child_token_hash, channel_binding_hash = $child_channel,
    authentication_strength = 'local_delegated_opaque', delegated_capabilities = ['fs.write', 'fs.read'],
    delegation_chain = $chain, parent_session_id = $parent_record, ancestor_session_ids = $ancestors,
    delegation_grant_edges = [
        { child_grant_id: $child_grant, parent_grant_id: $parent_grant },
        { child_grant_id: $child_read_grant, parent_grant_id: $parent_read_grant }
    ],
    account_revocation_epoch = $account.revocation_epoch, principal_revocation_epoch = 0,
    space_revocation_epoch = $space.revocation_epoch, policy_version = $policy,
    issued_at = $issued_at, expires_at = $expires_at, revoked_at = NONE;
"#;

const CONTROLLED_VALIDATOR_CHILD_CAPTURE: &str = r#"LET $child_create_authority = {
    grant_id: $child_grant.id, resource_id: $child_grant.resource_id,
    account_id: $child_grant.account_id, principal_id: $child_grant.principal_id,
    actor_kind: $child_grant.principal_id.actor_kind, actor_id: $child_grant.principal_id.actor_id,
    session_id: $child_session, access_space_id: $child_grant.access_space_id,
    grant_version: $child_grant.grant_version,
    policy_versions: [$child_grant.policy_version, $resource.policy_version, $account.policy_version,
        $principal.policy_version, $space.policy_version, $child_session.policy_version],
    revocation_epochs: [$account.revocation_epoch, $principal.revocation_epoch, $space.revocation_epoch,
        $child_session.account_revocation_epoch, $child_session.principal_revocation_epoch, $child_session.space_revocation_epoch],
    actions: $child_grant.actions, capability_ids: $child_grant.capability_ids,
    delegation_chain: $child_grant.delegation_chain, session_capabilities: $child_session.delegated_capabilities,
    principal_capabilities: $principal.delegated_capabilities,
    capability_profile_id: $principal.capability_profile_id,
    statuses: [$child_grant.status, $resource.lifecycle_state, $account.status, $principal.status, $space.status],
    resource_kind: $resource.resource_kind, grant_expires_at: $child_grant.expires_at,
    session_expires_at: $child_session.expires_at
};
LET $child_read_authority = {
    grant_id: $child_read_grant.id, resource_id: $child_read_grant.resource_id,
    account_id: $child_read_grant.account_id, principal_id: $child_read_grant.principal_id,
    actor_kind: $child_read_grant.principal_id.actor_kind, actor_id: $child_read_grant.principal_id.actor_id,
    session_id: $child_session, access_space_id: $child_read_grant.access_space_id,
    grant_version: $child_read_grant.grant_version,
    policy_versions: [$child_read_grant.policy_version, $resource.policy_version, $account.policy_version,
        $principal.policy_version, $space.policy_version, $child_session.policy_version],
    revocation_epochs: [$account.revocation_epoch, $principal.revocation_epoch, $space.revocation_epoch,
        $child_session.account_revocation_epoch, $child_session.principal_revocation_epoch, $child_session.space_revocation_epoch],
    actions: $child_read_grant.actions, capability_ids: $child_read_grant.capability_ids,
    delegation_chain: $child_read_grant.delegation_chain, session_capabilities: $child_session.delegated_capabilities,
    principal_capabilities: $principal.delegated_capabilities,
    capability_profile_id: $principal.capability_profile_id,
    statuses: [$child_read_grant.status, $resource.lifecycle_state, $account.status, $principal.status, $space.status],
    resource_kind: $resource.resource_kind, grant_expires_at: $child_read_grant.expires_at,
    session_expires_at: $child_session.expires_at
};
IF !fn::mt109_session_lineage_live($child_session) { THROW 'HSK-403-PROTECTED-RESOURCE'; };
SELECT VALUE {
    principal: (SELECT * FROM ONLY $principal),
    session: (SELECT * FROM ONLY $child_session),
    create_grant: (SELECT * FROM ONLY $child_grant),
    read_grant: (SELECT * FROM ONLY $child_read_grant),
    create_authority: $child_create_authority,
    read_authority: $child_read_authority,
    child_link: {
        session: (SELECT * FROM ONLY $child_session),
        principal: (SELECT * FROM ONLY $principal),
        grant_edges: array::map($child_session.delegation_grant_edges, |$edge| {
            edge: $edge,
            child_grant: (SELECT * FROM ONLY $edge.child_grant_id),
            parent_grant: (SELECT * FROM ONLY $edge.parent_grant_id),
            child_resource: (SELECT * FROM ONLY $edge.child_grant_id.resource_id),
            parent_resource: (SELECT * FROM ONLY $edge.parent_grant_id.resource_id)
        })
    }
} FROM $child_session;"#;

type ControlledValidatorClient = surrealdb::Surreal<surrealdb::engine::local::Db>;

type ControlledValidatorBeginFuture = <surrealdb::method::Begin<
    surrealdb::engine::local::Db> as std::future::IntoFuture>::IntoFuture;

type ControlledValidatorTerminalFuture = <surrealdb::method::Commit<
    surrealdb::engine::local::Db> as std::future::IntoFuture>::IntoFuture;

#[derive(Clone, Copy)]
enum ControlledValidatorTerminalKind { Commit, Rollback }

#[derive(Clone, Copy, PartialEq, Eq)]
enum ControlledValidatorSettlement { CommitObserved, RollbackObserved, Unknown }

#[derive(Default)]
enum ControlledValidatorEngine {
    #[default]
    New,
    Beginning(ControlledValidatorBeginFuture),
    BeginFailed(surrealdb::Error),
    Active(ControlledValidatorTransaction),
    Transitioning,
    Terminal {
        kind: ControlledValidatorTerminalKind,
        future: ControlledValidatorTerminalFuture,
    },
    Finished {
        kind: ControlledValidatorTerminalKind,
        result: Result<ControlledValidatorClient, surrealdb::Error>,
    },
}

type ControlledValidatorTransaction =
    surrealdb::method::Transaction<surrealdb::engine::local::Db>;

#[derive(Clone, PartialEq, SurrealValue)]
struct CanonicalValidatorSourceRows {
    account: surrealdb::types::Value,
    space: surrealdb::types::Value,
    resource: surrealdb::types::Value,
    ordered_chain: Vec<surrealdb::types::Value>,
    principal_ids: Vec<RecordId>,
    session_ids: Vec<RecordId>,
    create_grant: surrealdb::types::Value,
    read_grant: surrealdb::types::Value,
}

#[derive(Clone, SurrealValue)]
struct CanonicalValidatorChildRows {
    principal: surrealdb::types::Value,
    session: surrealdb::types::Value,
    create_grant: surrealdb::types::Value,
    read_grant: surrealdb::types::Value,
    create_authority: RegistryPublicationAuthority,
    read_authority: RegistryPublicationAuthority,
    child_link: surrealdb::types::Value,
}

#[derive(Default)]
struct ControlledValidatorIdentityState {
    entered: bool,
    engine: std::sync::Arc<tokio::sync::Mutex<ControlledValidatorEngine>>,
    source: Option<std::sync::Arc<CanonicalValidatorSourceRows>>,
    created: Option<std::sync::Arc<CanonicalValidatorChildRows>>,
    // Keep EVERY returned query error, not just the first meaningful one.
    query_errors: Vec<(&'static str, Vec<(usize, surrealdb::Error)>)>,
    query_decode_errors: Vec<(&'static str, Box<dyn std::error::Error + Send + Sync>)>,
    raw_read_results: Vec<(&'static str, Vec<surrealdb::types::Value>)>,
    settlement: Option<ControlledValidatorSettlement>,
    attempt_failure: Option<ValidatorAssociationFault>,
    rejected_child_create: Option<OwnedCanonicalAuthSnapshot>,
    child_seal: Option<std::sync::Arc<AuthenticatedValidatorLaunchSeal>>,
    child_read: Option<std::sync::Arc<OwnedCanonicalAuthSnapshot>>,
    // Raw source/child documents remain above even if constructing JSON fails.
    canonical: Option<std::sync::Arc<CanonicalValidatorAssociationSource>>,
}

pub(crate) struct CanonicalValidatorAssociationSource {
    body: handshake_producer_association::AssociationBodyV1,
    expected: handshake_producer_association::ExpectedAssociationInventoryV1,
}

impl CanonicalValidatorAssociationSource {
    pub(crate) fn body(&self) -> &handshake_producer_association::AssociationBodyV1 {
        &self.body
    }
    pub(crate) fn expected_inventory(&self)
        -> &handshake_producer_association::ExpectedAssociationInventoryV1 { &self.expected }
}

#[derive(Clone, SurrealValue)]
struct ControlledValidatorBindings {
    parent_record: RecordId,
    parent_grant: RecordId,
    parent_read_grant: RecordId,
    expected_parent_create_authority: RegistryPublicationAuthority,
    expected_parent_read_authority: RegistryPublicationAuthority,
    account: RecordId,
    parent_principal: RecordId,
    space: RecordId,
    resource: RecordId,
    parent_chain: Vec<String>,
    parent_expiry: Datetime,
    grant_expiry: Option<Datetime>,
    grant_version: i64,
    read_grant_expiry: Option<Datetime>,
    read_grant_version: i64,
    parent_token_hash: String,
    parent_channel: String,
    profile: String,
    policy: i64,
    workspace: String,
    actor_id: String,
    principal_key: String,
    principal: RecordId,
    child_grant: RecordId,
    child_read_grant: RecordId,
    child_session: RecordId,
    child_token_hash: String,
    child_channel: String,
    expires_at: Datetime,
    issued_at: Datetime,
}

impl OwnedValidatorAssociation {
    fn controlled_identity_bindings(&self)
        -> Result<ControlledValidatorBindings, ValidatorAssociationFault> {
        self.require_original_deadline()?;
        self.require_current_runtime_source()?;
        self.require_original_task_binding()?;
        let create = &self.input.parent.create;
        let read = &self.input.parent.read;
        let scope = &self.input.invocation.designation().scope;
        let a = &create.authority;
        if &create.exact != scope || &read.exact != scope
            || create.action != ResourceAction::Create || create.capability_id != "fs.write"
            || read.action != ResourceAction::Read || read.capability_id != "fs.read"
            || a.session_id != read.authority.session_id
            || a.account_id != read.authority.account_id
            || a.principal_id != read.authority.principal_id
            || a.access_space_id != read.authority.access_space_id
            || a.resource_id != read.authority.resource_id
            || a.delegation_chain != read.authority.delegation_chain
            || create.credentials.session_token != read.credentials.session_token
            || create.credentials.channel_binding_hash != read.credentials.channel_binding_hash
            || record_key(a.account_id.clone())? != scope.owner_account_id.to_string()
            || record_key(a.principal_id.clone())? != scope.actor_principal_id.to_string()
            || record_key(a.session_id.clone())? != scope.authenticated_session_id.to_string()
            || record_key(a.access_space_id.clone())? != scope.access_space_id.to_string()
            || create.workspace_id != scope.workspace_id.as_str()
            || !self.registry.profile_can(&a.capability_profile_id, "fs.write").unwrap_or(false)
            || !self.registry.profile_can(&read.authority.capability_profile_id, "fs.read").unwrap_or(false)
        {
            return Err(ResourceAuthorityError::InvalidInput(
                "controlled Validator original Create/Read identity differs",
            ).into());
        }
        create.require_unexpired()?;
        read.require_unexpired()?;
        let ids = self.inventory.get().ok_or(
            ValidatorAssociationFault::MissingCanonicalSourceCapture,
        )?;
        if Utc::now() >= ids.original_expires_at {
            return Err(ResourceAuthorityError::InvalidInput("original child expiry elapsed").into());
        }
        Ok(ControlledValidatorBindings {
            parent_record: a.session_id.clone(),
            parent_grant: a.grant_id.clone(),
            parent_read_grant: read.authority.grant_id.clone(),
            expected_parent_create_authority: a.clone(),
            expected_parent_read_authority: read.authority.clone(),
            account: a.account_id.clone(), parent_principal: a.principal_id.clone(),
            space: a.access_space_id.clone(), resource: a.resource_id.clone(),
            parent_chain: a.delegation_chain.clone(), parent_expiry: a.session_expires_at.clone(),
            grant_expiry: a.grant_expires_at.clone(), grant_version: a.grant_version,
            read_grant_expiry: read.authority.grant_expires_at.clone(),
            read_grant_version: read.authority.grant_version,
            parent_token_hash: sha256_hex(create.credentials.session_token.as_bytes()),
            parent_channel: create.credentials.channel_binding_hash.clone(),
            profile: a.capability_profile_id.clone(), policy: create.session.policy_version,
            workspace: create.workspace_id.clone(),
            actor_id: self.input.instance.instance_id().to_string(),
            principal_key: ids.child_principal_id.clone(),
            principal: RecordId::new("principals", ids.child_principal_id.clone()),
            child_grant: RecordId::new("resource_grants", ids.child_create_grant_id.clone()),
            child_read_grant: RecordId::new("resource_grants", ids.child_read_grant_id.clone()),
            child_session: RecordId::new("authenticated_sessions", ids.child_session_id.clone()),
            child_token_hash: sha256_hex(ids.child_credentials.session_token.as_bytes()),
            child_channel: ids.child_credentials.channel_binding_hash.clone(),
            expires_at: Datetime::from(ids.original_expires_at),
            issued_at: Datetime::from(ids.original_issued_at),
        })
    }

    fn retain_controlled_query_errors(
        &self, phase: &'static str, response: &mut surrealdb::IndexedResults,
    ) -> Result<(), ValidatorAssociationFault> {
        let mut errors = response.take_errors().into_iter().collect::<Vec<_>>();
        errors.sort_by_key(|(index, _)| *index);
        if errors.is_empty() { return Ok(()); }
        self.controlled_identity.lock().unwrap_or_else(|p| p.into_inner())
            .query_errors.push((phase, errors));
        Err(ValidatorAssociationFault::ControlledIdentityRetained { phase })
    }

    async fn controlled_gate(
        &self, transaction: &ControlledValidatorTransaction,
        bindings: &ControlledValidatorBindings,
    ) -> Result<(), ValidatorAssociationFault> {
        self.require_original_deadline()?;
        self.require_current_runtime_source()?;
        let query = transaction.query(CONTROLLED_VALIDATOR_GATE).bind(bindings.clone());
        let mut response = tokio::time::timeout_at(
            tokio::time::Instant::from_std(self.original_deadline), async { query.await },
        ).await.map_err(|_| ValidatorAssociationFault::OriginalDeadlineReached {
            original_deadline: self.original_deadline,
        })?.map_err(SurrealStorageError::from)?;
        self.retain_controlled_query_errors("source_authorization", &mut response)?;
        self.require_original_deadline()?;
        self.require_current_runtime_source()
    }

    async fn controlled_source_rows(
        &self, transaction: &ControlledValidatorTransaction,
        bindings: &ControlledValidatorBindings,
    ) -> Result<CanonicalValidatorSourceRows, ValidatorAssociationFault> {
        // Only called AFTER controlled_gate has verified the original actor,
        // Owner/workspace, credentials, BOTH grants, profile and full lineage.
        self.require_original_deadline()?;
        let query = transaction.query(CONTROLLED_VALIDATOR_CAPTURE).bind(bindings.clone());
        let mut response = tokio::time::timeout_at(
            tokio::time::Instant::from_std(self.original_deadline), async { query.await },
        ).await.map_err(|_| ValidatorAssociationFault::OriginalDeadlineReached {
            original_deadline: self.original_deadline,
        })?.map_err(SurrealStorageError::from)?;
        self.retain_controlled_query_errors("source_capture", &mut response)?;
        let raw = response.take::<Vec<surrealdb::types::Value>>(0)
            .map_err(SurrealStorageError::from)?;
        self.controlled_identity.lock().unwrap_or_else(|p| p.into_inner())
            .raw_read_results.push(("source_capture", raw.clone()));
        if raw.len() != 1 {
            return Err(ResourceAuthorityError::InvalidInput(
                "controlled Validator source cardinality differs",
            ).into());
        }
        let decoded = CanonicalValidatorSourceRows::from_value(raw[0].clone());
        match decoded {
            Ok(row) => Ok(row),
            Err(error) => {
                self.controlled_identity.lock().unwrap_or_else(|p| p.into_inner())
                    .query_decode_errors.push(("source_capture", Box::new(error)));
                Err(ValidatorAssociationFault::ControlledIdentityRetained {
                    phase: "source_capture_decode",
                })
            }
        }
    }

    // Private controlled Account producer, not a general administrative facade.
    // It accepts no caller SQL, role, unchecked scope or fabricated inventory.
    // The original retained driver calls it ONCE; lease is physical transport.
    async fn issue_original_controlled_child(owner: std::sync::Arc<Self>)
        -> Result<std::sync::Arc<CanonicalValidatorAssociationSource>, ValidatorAssociationFault> {
        let bindings = owner.controlled_identity_bindings()?;
        {
            let mut state = owner.controlled_identity.lock().unwrap_or_else(|p| p.into_inner());
            if state.entered {
                return Err(ValidatorAssociationFault::ControlledIdentityRetained {
                    phase: "already_entered",
                });
            }
            state.entered = true;
        }
        let storage = owner.storage.clone();
        let namespace = storage.config().namespace().to_owned();
        let database = storage.config().database().to_owned();
        // No retry wrapper. The outer original driver remains sole owner.
        storage.with_lease(move |client| Box::pin(async move {
            let account = client.clone();
            account.use_ns(namespace).use_db(database).await?;
            use std::future::IntoFuture;
            let engine_cell = {
                let state = owner.controlled_identity.lock().unwrap_or_else(|p| p.into_inner());
                std::sync::Arc::clone(&state.engine)
            };
            let mut engine = engine_cell.lock().await;
            if !matches!(&*engine, ControlledValidatorEngine::New) {
                return Ok(Err(ValidatorAssociationFault::ControlledIdentityRetained {
                    phase: "engine_already_owned",
                }));
            }
            *engine = ControlledValidatorEngine::Beginning(account.begin().into_future());
            let begin = match &mut *engine {
                ControlledValidatorEngine::Beginning(future) => future.as_mut().await,
                _ => unreachable!("same original begin phase"),
            };
            match begin {
                Ok(transaction) => *engine = ControlledValidatorEngine::Active(transaction),
                Err(error) => {
                    *engine = ControlledValidatorEngine::BeginFailed(error);
                    return Ok(Err(ValidatorAssociationFault::ControlledIdentityRetained {
                        phase: "begin_failed",
                    }));
                }
            }
            let transaction = match &*engine {
                ControlledValidatorEngine::Active(transaction) => transaction,
                _ => unreachable!("same original active phase"),
            };
            let attempt: Result<(), ValidatorAssociationFault> = async {
                owner.controlled_gate(&transaction, &bindings).await?;
                let source = std::sync::Arc::new(
                    owner.controlled_source_rows(&transaction, &bindings).await?,
                );
                owner.controlled_identity.lock().unwrap_or_else(|p| p.into_inner())
                    .source = Some(std::sync::Arc::clone(&source));
                // Check the exact original sources again inside this SAME tx
                // immediately before CREATE, including complete captured rows.
                owner.controlled_gate(&transaction, &bindings).await?;
                let reread = owner.controlled_source_rows(&transaction, &bindings).await?;
                if *source != reread {
                    return Err(ResourceAuthorityError::InvalidInput(
                        "controlled Validator full source changed before creation",
                    ).into());
                }
                owner.require_current_runtime_source()?;
                owner.require_original_deadline()?;
                // LET bindings are local to ONE request. The complete current
                // gate and CREATE execute in that SAME actual request.
                let create_request = format!("{}\n{}",
                    CONTROLLED_VALIDATOR_GATE, CONTROLLED_VALIDATOR_CREATE,
                );
                let mut effect = transaction.query(create_request)
                    .bind(bindings.clone()).await.map_err(SurrealStorageError::from)?;
                owner.retain_controlled_query_errors("child_creation", &mut effect)?;
                // Reading the actual newly created documents is not a guessed
                // expected-CAS reconstruction and is retained before COMMIT.
                let query = transaction.query(CONTROLLED_VALIDATOR_CHILD_CAPTURE)
                    .bind(bindings.clone());
                let mut response = tokio::time::timeout_at(
                    tokio::time::Instant::from_std(owner.original_deadline), async { query.await },
                ).await.map_err(|_| ValidatorAssociationFault::OriginalDeadlineReached {
                    original_deadline: owner.original_deadline,
                })?.map_err(SurrealStorageError::from)?;
                owner.retain_controlled_query_errors("child_capture", &mut response)?;
                // Two LETs and the exact created-child lineage IF precede SELECT.
                let raw = response.take::<Vec<surrealdb::types::Value>>(3)
                    .map_err(SurrealStorageError::from)?;
                owner.controlled_identity.lock().unwrap_or_else(|p| p.into_inner())
                    .raw_read_results.push(("child_capture", raw.clone()));
                if raw.len() != 1 {
                    return Err(ResourceAuthorityError::InvalidInput(
                        "controlled Validator child cardinality differs",
                    ).into());
                }
                let decoded = CanonicalValidatorChildRows::from_value(raw[0].clone());
                let created = match decoded {
                    Ok(row) => std::sync::Arc::new(row),
                    Err(error) => {
                        owner.controlled_identity.lock().unwrap_or_else(|p| p.into_inner())
                            .query_decode_errors.push(("child_capture", Box::new(error)));
                        return Err(ValidatorAssociationFault::ControlledIdentityRetained {
                            phase: "child_capture_decode",
                        });
                    }
                };
                owner.controlled_identity.lock().unwrap_or_else(|p| p.into_inner())
                    .created = Some(std::sync::Arc::clone(&created));
                owner.controlled_gate(&transaction, &bindings).await?;
                owner.require_current_runtime_source()?;
                owner.require_original_deadline()?;
                // Construct the exact typed source before commit, preserving the
                // native documents as well if JSON/identity validation rejects.
                let canonical = std::sync::Arc::new(owner.canonical_association_source(
                    &source, &created,
                )?);
                owner.controlled_identity.lock().unwrap_or_else(|p| p.into_inner())
                    .canonical = Some(canonical);
                Ok(())
            }.await;
            let (kind, original_failed) = match attempt {
                Ok(()) => (ControlledValidatorTerminalKind::Commit, false),
                Err(error) => {
                    // Preserve the complete original fault BEFORE awaiting
                    // rollback, including cancellation of this driver.
                    owner.controlled_identity.lock().unwrap_or_else(|p| p.into_inner())
                        .attempt_failure = Some(error);
                    (ControlledValidatorTerminalKind::Rollback, true)
                }
            };
            // Consume the sole real handle into its OWNED terminal future
            // synchronously. Install the future in SAME frame BEFORE polling.
            let transaction = match std::mem::replace(
                &mut *engine, ControlledValidatorEngine::Transitioning,
            ) {
                ControlledValidatorEngine::Active(transaction) => transaction,
                phase => {
                    *engine = phase;
                    return Ok(Err(ValidatorAssociationFault::ControlledIdentityRetained {
                        phase: "active_terminal_transition_mismatch",
                    }));
                }
            };
            let future: ControlledValidatorTerminalFuture = match kind {
                ControlledValidatorTerminalKind::Commit => transaction.commit().into_future(),
                ControlledValidatorTerminalKind::Rollback => transaction.cancel().into_future(),
            };
            *engine = ControlledValidatorEngine::Terminal { kind, future };
            // A cancelled driver releases its mutex guard but leaves this
            // exact future in the frame. Never race/drop physical settlement.
            let result = match &mut *engine {
                ControlledValidatorEngine::Terminal { future, .. } => future.as_mut().await,
                _ => unreachable!("same original terminal phase"),
            };
            let observed = result.is_ok();
            // SDK3.2 returns the actual Surreal client. Retain that client or
            // the whole raw SDK error, without inventing a unit result ABI.
            *engine = ControlledValidatorEngine::Finished { kind, result };
            let settlement = if !observed {
                ControlledValidatorSettlement::Unknown
            } else {
                match kind {
                    ControlledValidatorTerminalKind::Commit =>
                        ControlledValidatorSettlement::CommitObserved,
                    ControlledValidatorTerminalKind::Rollback =>
                        ControlledValidatorSettlement::RollbackObserved,
                }
            };
            owner.controlled_identity.lock().unwrap_or_else(|p| p.into_inner())
                .settlement = Some(settlement);
            if original_failed {
                // Raw original fault and actual rollback result both remain.
                return Ok(Err(ValidatorAssociationFault::ControlledIdentityRetained {
                    phase: "original_attempt_failed",
                }));
            }
            if settlement != ControlledValidatorSettlement::CommitObserved {
                return Ok(Err(ValidatorAssociationFault::ControlledIdentityRetained {
                    phase: "commit_unknown",
                }));
            }
            if let Err(error) = owner.require_current_runtime_source() {
                return Ok(Err(error));
            }
            let canonical = owner.controlled_identity.lock().unwrap_or_else(|p| p.into_inner())
                .canonical.as_ref().map(std::sync::Arc::clone);
            Ok(canonical.ok_or(ValidatorAssociationFault::ControlledIdentityRetained {
                phase: "committed_source_missing",
            }))
        })).await.map_err(ValidatorAssociationFault::Storage)?
    }
}

impl OwnedValidatorAssociation {
    fn canonical_association_source(
        &self, source: &CanonicalValidatorSourceRows, child: &CanonicalValidatorChildRows,
    ) -> Result<CanonicalValidatorAssociationSource, ValidatorAssociationFault> {
        use handshake_producer_association as codec;
        let ids = self.inventory.get().ok_or(
            ValidatorAssociationFault::MissingCanonicalSourceCapture,
        )?;
        let original = &self.input.invocation;
        let request = original.designation();
        let exact = &request.scope;
        fn encode<T: serde::Serialize>(value: &T)
            -> Result<serde_json::Value, ValidatorAssociationFault> {
            serde_json::to_value(value).map_err(ValidatorAssociationFault::CanonicalSourceJson)
        }
        let issuer = codec::Scope5V1 {
            owner_account_id: exact.owner_account_id.to_string(),
            actor_principal_id: exact.actor_principal_id.to_string(),
            authenticated_session_id: exact.authenticated_session_id.to_string(),
            access_space_id: exact.access_space_id.to_string(),
            workspace_id: exact.workspace_id.as_str().to_owned(),
        };
        let child_scope = codec::Scope5V1 {
            owner_account_id: issuer.owner_account_id.clone(),
            actor_principal_id: ids.child_principal_id.clone(),
            authenticated_session_id: ids.child_session_id.clone(),
            access_space_id: issuer.access_space_id.clone(),
            workspace_id: issuer.workspace_id.clone(),
        };
        if record_key(child.create_authority.principal_id.clone())? != ids.child_principal_id
            || record_key(child.read_authority.principal_id.clone())? != ids.child_principal_id
            || record_key(child.create_authority.session_id.clone())? != ids.child_session_id
            || record_key(child.read_authority.session_id.clone())? != ids.child_session_id
            || record_key(child.create_authority.grant_id.clone())? != ids.child_create_grant_id
            || record_key(child.read_authority.grant_id.clone())? != ids.child_read_grant_id
            || child.create_authority.resource_id != self.input.parent.create.authority.resource_id
            || child.read_authority.resource_id != self.input.parent.read.authority.resource_id
            || child.create_authority.actions != ["create"]
            || child.create_authority.capability_ids != ["fs.write"]
            || child.read_authority.actions != ["read"]
            || child.read_authority.capability_ids != ["fs.read"]
            || child.create_authority.session_expires_at.clone().into_inner() != ids.original_expires_at
            || child.read_authority.session_expires_at.clone().into_inner() != ids.original_expires_at
            || child.create_authority.grant_expires_at.as_ref().map(|d| d.clone().into_inner())
                != Some(ids.original_expires_at)
            || child.read_authority.grant_expires_at.as_ref().map(|d| d.clone().into_inner())
                != Some(ids.original_expires_at)
        {
            return Err(ResourceAuthorityError::InvalidInput(
                "actual controlled child differs from original prepared inventory",
            ).into());
        }
        let source_chain = source.ordered_chain.iter().cloned()
            .map(surrealdb::types::Value::into_json_value).collect::<Vec<_>>();
        if source_chain.is_empty()
            || source_chain.iter().any(|link| !link.is_object())
        {
            return Err(ResourceAuthorityError::InvalidInput(
                "canonical ordered source links missing",
            ).into());
        }
        let mut child_chain = source_chain.clone();
        child_chain.push(child.child_link.clone().into_json_value());
        let (requested_task_snapshot, server_task_mapping) =
            self.canonical_task_audit(source, child)?;
        let mapping = self.task_mapping.get().ok_or(
            ResourceAuthorityError::InvalidInput("original server mapping not prepared"),
        )?;
        let mapped_sources = self.input.parent.association_task_sources_in_requested_order(mapping)?;
        let full_intent = serde_json::json!({
            "requested_task_snapshot": requested_task_snapshot,
            "server_task_mapping": server_task_mapping,
            "designation": encode(request)?,
            "causation_semantics": original.causation_semantics(),
            "selection": original.selection_json(),
            "committed_launch": original.committed_launch_json(),
            "pre_spawn_launch": original.original_launch_json(),
            "canonical_run_trace": original.canonical_run_trace_json(),
            "canonical_launch_plan": encode(&original.canonical_launch_plan())?,
        });
        let actual_instance = codec::InstanceGenerationV1 {
            instance_id: self.input.instance.instance_id().to_string(),
            generation: self.input.instance.spawn_generation(),
        };
        let native_source_documents = serde_json::json!({
            "account": source.account.clone().into_json_value(),
            "space": source.space.clone().into_json_value(),
            "resource": source.resource.clone().into_json_value(),
            "ordered_chain": source_chain,
            "create_grant": source.create_grant.clone().into_json_value(),
            "read_grant": source.read_grant.clone().into_json_value(),
        });
        let native_child_documents = serde_json::json!({
            "principal": child.principal.clone().into_json_value(),
            "session": child.session.clone().into_json_value(),
            "create_grant": child.create_grant.clone().into_json_value(),
            "read_grant": child.read_grant.clone().into_json_value(),
            "ordered_chain": child_chain,
        });
        let resource = codec::ResourceV1 {
            resource_kind: "workspace".to_owned(),
            resource_id: record_key(self.input.parent.create.authority.resource_id.clone())?,
        };
        let mut pairs = Vec::with_capacity(2);
        let mut cleanup_grants = Vec::with_capacity(2);
        for (operation_ordinal, mapped) in mapped_sources.iter().enumerate() {
            let (parent_auth, child_auth, parent_row, child_row, action, capability) =
                match mapped.requested.tool() {
                    ValidatorAssociationTaskToolV1::DelegateWorkspaceCreate => (
                        &self.input.parent.create.authority, &child.create_authority,
                        &source.create_grant, &child.create_grant, "create", "fs.write",
                    ),
                    ValidatorAssociationTaskToolV1::DelegateWorkspaceRead => (
                        &self.input.parent.read.authority, &child.read_authority,
                        &source.read_grant, &child.read_grant, "read", "fs.read",
                    ),
                };
            if mapped.canonical.authority != *parent_auth
                || mapped.requested.action().as_str() != action
                || mapped.requested.capability_id() != capability
            {
                return Err(ResourceAuthorityError::InvalidInput(
                    "ordered mapped source/child pair differs",
                ).into());
            }
            let requested_operation = serde_json::to_value(mapped.requested)
                .map_err(ValidatorAssociationFault::CanonicalSourceJson)?;
            let mapped_tool = serde_json::to_value(mapped.requested.tool())
                .map_err(ValidatorAssociationFault::CanonicalSourceJson)?;
            let source_grant_id = record_key(parent_auth.grant_id.clone())?;
            let child_grant_id = record_key(child_auth.grant_id.clone())?;
            let provenance = |row: &surrealdb::types::Value,
                              authority: &RegistryPublicationAuthority, chain: &[serde_json::Value]| {
                serde_json::json!({
                    "operation_ordinal": operation_ordinal,
                    "requested_operation": requested_operation,
                    "server_tool": mapped_tool,
                    "requested_task_snapshot": requested_task_snapshot,
                    "server_task_mapping": server_task_mapping,
                    "grant": row.clone().into_json_value(),
                    "ordered_delegation_chain": chain,
                    "account": source.account.clone().into_json_value(),
                    "space": source.space.clone().into_json_value(),
                    "resource": source.resource.clone().into_json_value(),
                    "policy_versions": authority.policy_versions,
                    "revocation_epochs": authority.revocation_epochs,
                })
            };
            let audit = |grant_id: String, auth: &RegistryPublicationAuthority, full| {
                codec::GrantAuditV1 {
                    grant_id, grant_version: auth.grant_version,
                    policy_versions: auth.policy_versions.clone(),
                    revocation_epochs: auth.revocation_epochs.clone(),
                    expires_at: auth.grant_expires_at.as_ref()
                        .map(|d| d.clone().into_inner().to_rfc3339_opts(
                            chrono::SecondsFormat::Nanos, true,
                        )),
                    full_grant_provenance: full,
                }
            };
            let pair = codec::SourceChildPairV1 {
                source_grant: audit(source_grant_id.clone(), parent_auth,
                    provenance(parent_row, parent_auth, &source_chain)),
                child_grant: audit(child_grant_id.clone(), child_auth,
                    provenance(child_row, child_auth, &child_chain)),
                source_resource: resource.clone(), child_resource: resource.clone(),
                source_action_capability: codec::ActionCapabilityV1 {
                    action: action.to_owned(), capability_id: capability.to_owned(),
                },
                child_action_capability: codec::ActionCapabilityV1 {
                    action: action.to_owned(), capability_id: capability.to_owned(),
                },
                source_delegation_chain: source_chain.clone(),
                child_delegation_chain: child_chain.clone(),
            };
            // This is the ACTUAL full post-CREATE row, not a projection of the
            // ID/version. A cleanup consumer must compare canonical rows and
            // the original binding atomically before any mutation.
            let expected_cas = serde_json::json!({
                "original_operation_id": original.operation_id(),
                "original_issuer": issuer,
                "child_scope": child_scope,
                "actual_instance": actual_instance,
                "full_intent": full_intent,
                "source_pair": pair,
                "source_documents": native_source_documents,
                "child_documents": native_child_documents,
                "actual_child_grant": child_row.clone().into_json_value(),
                "original_association_event_id": ids.association_event_id,
                "original_issued_at": ids.original_issued_at.to_rfc3339_opts(
                    chrono::SecondsFormat::Nanos, true,
                ),
            });
            cleanup_grants.push(codec::ChildGrantCleanupV1 {
                source_parent_grant_id: source_grant_id,
                source_parent_grant_version: parent_auth.grant_version,
                child_grant_id, child_grant_version: child_auth.grant_version,
                expected_cas,
            });
            pairs.push(pair);
        }
        let cleanup = codec::ChildCleanupInventoryV1 {
            child_principal_id: ids.child_principal_id.clone(),
            child_session_id: ids.child_session_id.clone(),
            child_grants: cleanup_grants,
            full_canonical_cleanup_inventory: serde_json::json!({
                "requested_task_snapshot": requested_task_snapshot,
                "server_task_mapping": server_task_mapping,
                "original_operation_id": original.operation_id(),
                "original_issuer": issuer,
                "child_scope": child_scope,
                "actual_instance": actual_instance,
                "full_intent": full_intent,
                "source_documents": native_source_documents,
                "child_documents": native_child_documents,
                "source_child_pairs": pairs,
                "original_association_event_id": ids.association_event_id,
            }),
        };
        fn unique_record_keys(rows: &[RecordId]) -> Result<Vec<String>, ResourceAuthorityError> {
            let mut seen = std::collections::BTreeSet::new();
            let mut ordered = vec![];
            for record in rows {
                let key = record_key(record.clone())?;
                if seen.insert(key.clone()) { ordered.push(key); }
            }
            Ok(ordered)
        }
        let original_principal_ids = unique_record_keys(&source.principal_ids)?;
        let original_session_ids = unique_record_keys(&source.session_ids)?;
        let mut original_grants = vec![];
        for auth in [&self.input.parent.create.authority, &self.input.parent.read.authority] {
            let grant_id = record_key(auth.grant_id.clone())?;
            if !original_grants.iter().any(|g: &codec::GrantIdentityV1| g.grant_id == grant_id) {
                original_grants.push(codec::GrantIdentityV1 {
                    grant_id, grant_version: auth.grant_version,
                });
            }
        }
        let expected = codec::ExpectedAssociationInventoryV1 {
            original_principal_ids, original_session_ids, original_grants,
            created: cleanup.clone(),
        };
        let locus = original.canonical_run_trace_json()
            .get("run").and_then(|run| run.get("locus_binding"))
            .filter(|locus| locus.is_object()).cloned()
            .ok_or(ResourceAuthorityError::InvalidInput(
                "original canonical run has no complete Locus",
            ))?;
        let body = codec::AssociationBodyV1 {
            format_version: 1,
            original_operation_id: original.operation_id(),
            request_id: original.request_id().to_owned(),
            idempotency_key: original.idempotency_key().to_owned(),
            causation_ref: original.causation_ref().to_owned(),
            original_issuer: issuer, child_scope,
            actual_instance,
            server_normalized_selection: original.selection_json().clone(),
            run: codec::RunTraceV1 {
                run_id: request.run.run_id.clone(), trace_id: request.run.trace_id.clone(),
                work_packet_id: Some(request.run.work_packet_id.clone()),
                micro_task_id: Some(request.run.micro_task_id.clone()),
                locus, full_run_trace: original.canonical_run_trace_json().clone(),
            },
            launch: original.committed_launch_json().clone(),
            source_chain: pairs, cleanup,
        };
        Ok(CanonicalValidatorAssociationSource { body, expected })
    }
}

impl OwnedValidatorAssociation {
    fn committed_canonical_source(&self)
        -> Result<std::sync::Arc<CanonicalValidatorAssociationSource>, ValidatorAssociationFault> {
        let state = self.controlled_identity.lock().unwrap_or_else(|p| p.into_inner());
        if !matches!(state.settlement, Some(ControlledValidatorSettlement::CommitObserved)) {
            return Err(ValidatorAssociationFault::ControlledIdentityRetained {
                phase: "identity_not_committed",
            });
        }
        state.canonical.as_ref().map(std::sync::Arc::clone)
            .ok_or(ValidatorAssociationFault::ControlledIdentityRetained {
                phase: "identity_source_missing",
            })
    }

    async fn authorize_original_controlled_child(
        &self,
    ) -> Result<(std::sync::Arc<AuthenticatedValidatorLaunchSeal>,
                 std::sync::Arc<OwnedCanonicalAuthSnapshot>), ValidatorAssociationFault> {
        let _canonical = self.committed_canonical_source()?;
        self.require_current_sources().await?;
        let ids = self.inventory.get().ok_or(
            ValidatorAssociationFault::MissingCanonicalSourceCapture,
        )?;
        let exact = self.input.invocation.designation().scope.clone();
        let create_request = AuthorizationRequest {
            session_token: ids.child_credentials.session_token.clone(),
            channel_binding_hash: Some(ids.child_credentials.channel_binding_hash.clone()),
            resource_kind: ResourceKind::Workspace,
            external_resource_id: exact.workspace_id.as_str().to_owned(),
            capability_id: "fs.write".to_owned(), action: ResourceAction::Create,
        };
        let auth = capture_owned_canonical_role_auth(
            &self.storage, &self.registry, ids.child_credentials.clone(), create_request,
        ).await?;
        // Child scope differs from issuer on principal/session, intentionally.
        if auth.exact.owner_account_id != exact.owner_account_id
            || auth.exact.access_space_id != exact.access_space_id
            || auth.exact.workspace_id != exact.workspace_id
            || auth.exact.actor_principal_id.to_string() != ids.child_principal_id
            || auth.exact.authenticated_session_id.to_string() != ids.child_session_id
            || record_key(auth.authority.grant_id.clone())? != ids.child_create_grant_id
        {
            self.controlled_identity.lock().unwrap_or_else(|p| p.into_inner())
                .rejected_child_create = Some(auth);
            return Err(ResourceAuthorityError::InvalidInput(
                "controlled child current Create capture differs",
            ).into());
        }
        let seal = std::sync::Arc::new(AuthenticatedValidatorLaunchSeal {
            auth, registry: self.registry.clone(), storage: self.storage.clone(),
            instance: self.input.instance.clone(),
            parent_session_id: self.input.instance.parent_session_id().to_owned(),
            delegating_scope: exact.clone(), expires_at: ids.original_expires_at,
        });
        self.controlled_identity.lock().unwrap_or_else(|p| p.into_inner())
            .child_seal = Some(std::sync::Arc::clone(&seal));
        self.require_original_deadline()?;
        let read_request = AuthorizationRequest {
            session_token: ids.child_credentials.session_token.clone(),
            channel_binding_hash: Some(ids.child_credentials.channel_binding_hash.clone()),
            resource_kind: ResourceKind::Workspace,
            external_resource_id: exact.workspace_id.as_str().to_owned(),
            capability_id: "fs.read".to_owned(), action: ResourceAction::Read,
        };
        let child_read = std::sync::Arc::new(capture_owned_canonical_role_auth(
            &self.storage, &self.registry, ids.child_credentials.clone(), read_request,
        ).await?);
        self.controlled_identity.lock().unwrap_or_else(|p| p.into_inner())
            .child_read = Some(std::sync::Arc::clone(&child_read));
        if child_read.exact != seal.auth.exact
            || record_key(child_read.authority.grant_id.clone())? != ids.child_read_grant_id
        {
            return Err(ResourceAuthorityError::InvalidInput(
                "controlled child current Read capture differs",
            ).into());
        }
        // No revoke_session on failure. Original child IDs/rows, both captures
        // and all outcomes stay in the same retained original operation.
        self.require_current_sources().await?;
        seal.auth.revalidate_for_publication(&self.storage, &self.registry).await?;
        child_read.revalidate_for_publication(&self.storage, &self.registry).await?;
        self.require_current_runtime_source()?;
        Ok((seal, child_read))
    }
}

pub(crate) struct CurrentValidatorAssociationSeal<'tx> {
    owner: &'tx OwnedValidatorAssociation,
    transaction: &'tx super::super::SurrealTransactionContext<'tx>,
    original_stream: &'tx str,
    canonical: std::sync::Arc<CanonicalValidatorAssociationSource>,
}

pub(crate) struct ValidatorAssociationReadbackGuard<'tx> {
    owner: &'tx OwnedValidatorAssociation,
    transaction: &'tx super::super::SurrealTransactionContext<'tx>,
    original_stream: &'tx str,
    canonical: std::sync::Arc<CanonicalValidatorAssociationSource>,
}

impl OwnedValidatorAssociation {
    fn retained_child_sources(&self)
        -> Result<(std::sync::Arc<AuthenticatedValidatorLaunchSeal>,
                   std::sync::Arc<OwnedCanonicalAuthSnapshot>), ValidatorAssociationFault> {
        let state = self.controlled_identity.lock().unwrap_or_else(|p| p.into_inner());
        if !matches!(state.settlement, Some(ControlledValidatorSettlement::CommitObserved)) {
            return Err(ValidatorAssociationFault::ControlledIdentityRetained {
                phase: "identity_not_committed",
            });
        }
        let create = state.child_seal.as_ref().map(std::sync::Arc::clone);
        let read = state.child_read.as_ref().map(std::sync::Arc::clone);
        match (create, read) {
            (Some(create), Some(read)) => Ok((create, read)),
            _ => Err(ValidatorAssociationFault::ControlledIdentityRetained {
                phase: "child_authority_capture_missing",
            }),
        }
    }

    async fn capture_create_read_seal<'tx>(
        &'tx self,
        transaction: &'tx super::super::SurrealTransactionContext<'tx>,
    ) -> Result<CurrentValidatorAssociationSeal<'tx>, ValidatorAssociationFault> {
        SurrealStorage::with_operation_deadline(self.original_deadline, async {
            self.require_current_runtime_source()?;
            let canonical = self.committed_canonical_source()?;
            let (create, read) = self.retained_child_sources()?;
            transaction.require_authenticated_scope(&self.storage, &create.auth.record_user_scope())?;
            create.auth.revalidate_in_transaction(transaction, &self.registry).await?;
            self.require_original_deadline()?;
            revalidate_secondary_validator_source(
                &read, &create.auth, transaction, &self.registry,
            ).await?;
            // The same child session's actual lineage predicate guards the
            // source ancestor/grant chain in this child record-user transaction.
            create.auth.revalidate_in_transaction(transaction, &self.registry).await?;
            self.require_current_runtime_source()?;
            transaction.require_authenticated_scope(&self.storage, &create.auth.record_user_scope())?;
            Ok(CurrentValidatorAssociationSeal {
                owner: self, transaction, original_stream: self.original_stream()?, canonical,
            })
        }).await
    }

    async fn capture_readback_guard<'tx>(
        &'tx self,
        transaction: &'tx super::super::SurrealTransactionContext<'tx>,
    ) -> Result<ValidatorAssociationReadbackGuard<'tx>, ValidatorAssociationFault> {
        SurrealStorage::with_operation_deadline(self.original_deadline, async {
            self.require_current_runtime_source()?;
            let canonical = self.committed_canonical_source()?;
            let (create, read) = self.retained_child_sources()?;
            transaction.require_authenticated_scope(&self.storage, &read.record_user_scope())?;
            read.revalidate_in_transaction(transaction, &self.registry).await?;
            self.require_original_deadline()?;
            revalidate_secondary_validator_source(
                &create.auth, &read, transaction, &self.registry,
            ).await?;
            read.revalidate_in_transaction(transaction, &self.registry).await?;
            self.require_current_runtime_source()?;
            transaction.require_authenticated_scope(&self.storage, &read.record_user_scope())?;
            Ok(ValidatorAssociationReadbackGuard {
                owner: self, transaction, original_stream: self.original_stream()?, canonical,
            })
        }).await
    }
}

impl CurrentValidatorAssociationSeal<'_> {
    pub(crate) fn body(&self) -> &handshake_producer_association::AssociationBodyV1 {
        self.canonical.body()
    }
    pub(crate) fn expected_inventory(&self)
        -> &handshake_producer_association::ExpectedAssociationInventoryV1 {
        self.canonical.expected_inventory()
    }
    pub(crate) fn matches_readback(&self, guard: &ValidatorAssociationReadbackGuard<'_>) -> bool {
        std::ptr::eq(self.owner, guard.owner)
            && self.operation_id() == guard.operation_id()
            && std::sync::Arc::ptr_eq(&self.canonical, &guard.canonical)
    }
}

impl ValidatorAssociationReadbackGuard<'_> {
    pub(crate) fn body(&self) -> &handshake_producer_association::AssociationBodyV1 {
        self.canonical.body()
    }
    pub(crate) fn expected_inventory(&self)
        -> &handshake_producer_association::ExpectedAssociationInventoryV1 {
        self.canonical.expected_inventory()
    }
}

impl OwnedValidatorAssociation {
    async fn prepare_original_task_mapping(&self)
        -> Result<(), ValidatorAssociationFault> {
        self.require_original_deadline()?;
        self.require_current_runtime_source()?;
        // This is private one-shot original-driver preparation, not a new
        // borrowed public owner/future or a retry of a cancelled original task.
        if self.requested_task.get().is_some() || self.task_mapping.get().is_some() {
            return Err(ResourceAuthorityError::InvalidInput(
                "original requested task mapping already prepared",
            ).into());
        }
        let requested: ValidatorAssociationTaskSnapshotV1 = serde_json::from_value(
            self.input.requested_task_snapshot().clone(),
        ).map_err(ValidatorAssociationFault::TaskSnapshotDecode)?;
        let requested = std::sync::Arc::new(requested);
        self.requested_task.set(std::sync::Arc::clone(&requested)).map_err(|_| {
            ResourceAuthorityError::InvalidInput("original task already retained")
        })?;
        // Host187's actual server mapper validates complete ordered caller
        // tuples, both concrete server operations, exact workspace and actual
        // parent Create/Read snapshots. Claimed revision stays UNTRUSTED.
        let mapping = self.input.parent.map_requested_association_task(
            &self.storage, &self.registry,
            &self.input.invocation.designation().scope, requested,
        ).await?;
        self.task_mapping.set(mapping).map_err(|_| {
            ResourceAuthorityError::InvalidInput("original server mapping already retained")
        })?;
        self.revalidate_original_task_mapping().await
    }

    async fn revalidate_original_task_mapping(&self)
        -> Result<(), ValidatorAssociationFault> {
        self.require_original_deadline()?;
        let requested = self.requested_task.get().ok_or(
            ResourceAuthorityError::InvalidInput("original task snapshot not prepared"),
        )?;
        let mapping = self.task_mapping.get().ok_or(
            ResourceAuthorityError::InvalidInput("original server mapping not prepared"),
        )?;
        self.input.parent.revalidate_association_task_mapping(
            &self.storage, &self.registry,
            &self.input.invocation.designation().scope, requested, mapping,
        ).await?;
        self.require_original_deadline()?;
        self.require_current_runtime_source()
    }
}

impl OwnedValidatorAssociation {
    fn canonical_task_audit(
        &self, source: &CanonicalValidatorSourceRows, child: &CanonicalValidatorChildRows,
    ) -> Result<(serde_json::Value, serde_json::Value), ValidatorAssociationFault> {
        let requested = self.requested_task.get().ok_or(
            ResourceAuthorityError::InvalidInput("original task snapshot not prepared"),
        )?;
        let mapping = self.task_mapping.get().ok_or(
            ResourceAuthorityError::InvalidInput("original server mapping not prepared"),
        )?;
        if !mapping.matches_requested(requested) {
            return Err(ResourceAuthorityError::InvalidInput(
                "original task mapping identity differs",
            ).into());
        }
        let ordered = self.input.parent.association_task_sources_in_requested_order(mapping)?;
        let mut operations = Vec::with_capacity(ordered.len());
        for (ordinal, mapped) in ordered.iter().enumerate() {
            // The explicit SERVER tool tag selects its genuine parent/child
            // pair. Never zip posted order with a fixed Create/Read vector.
            let (parent, child_auth, parent_row, child_row) = match mapped.requested.tool() {
                ValidatorAssociationTaskToolV1::DelegateWorkspaceCreate => (
                    &self.input.parent.create, &child.create_authority,
                    &source.create_grant, &child.create_grant,
                ),
                ValidatorAssociationTaskToolV1::DelegateWorkspaceRead => (
                    &self.input.parent.read, &child.read_authority,
                    &source.read_grant, &child.read_grant,
                ),
            };
            if !std::ptr::eq(mapped.canonical, parent) {
                return Err(ResourceAuthorityError::InvalidInput(
                    "server mapped source owner differs",
                ).into());
            }
            let requested_operation = serde_json::to_value(mapped.requested)
                .map_err(ValidatorAssociationFault::CanonicalSourceJson)?;
            let tool = serde_json::to_value(mapped.requested.tool())
                .map_err(ValidatorAssociationFault::CanonicalSourceJson)?;
            operations.push(serde_json::json!({
                "ordinal": ordinal,
                "requested_operation": requested_operation,
                "server_tool": tool,
                "server_resource_kind": parent.authority.resource_kind,
                "server_resource_id": record_key(parent.authority.resource_id.clone())?,
                "server_external_resource_id": parent.workspace_id,
                "server_action": parent.action.as_str(),
                "server_capability_id": parent.capability_id,
                "source_grant_id": record_key(parent.authority.grant_id.clone())?,
                "source_grant_version": parent.authority.grant_version,
                "source_policy_versions": parent.authority.policy_versions,
                "source_revocation_epochs": parent.authority.revocation_epochs,
                "source_profile_id": parent.authority.capability_profile_id,
                "source_session_capabilities": parent.authority.session_capabilities,
                "source_principal_capabilities": parent.authority.principal_capabilities,
                "canonical_parent_grant": parent_row.clone().into_json_value(),
                "actual_child_grant_id": record_key(child_auth.grant_id.clone())?,
                "actual_child_grant_version": child_auth.grant_version,
                "canonical_child_grant": child_row.clone().into_json_value(),
            }));
        }
        let task = serde_json::to_value(requested.as_ref())
            .map_err(ValidatorAssociationFault::CanonicalSourceJson)?;
        let server = serde_json::json!({
            "format_version": 1,
            "server_handler": "OperatorChatLaunchService::register_authenticated_validator_producer_for_designation",
            "original_operation_id": self.input.invocation.operation_id(),
            "original_designation": serde_json::to_value(self.input.invocation.designation())
                .map_err(ValidatorAssociationFault::CanonicalSourceJson)?,
            "requested_task_snapshot": task,
            "ordered_operations": operations,
        });
        Ok((task, server))
    }
}

impl ValidatorAssociationIssuanceHandle {
    pub(crate) fn matches_cached_producer(
        &self, producer: &crate::swarm_orchestration::coordinator::ValidatorProducerKey,
    ) -> bool {
        let state = self.retention.frame.state.lock().unwrap_or_else(|p| p.into_inner());
        matches!(state.outcome.as_ref(), Some(Ok(confirmed))
            if std::ptr::eq(confirmed.seal.as_ref(), producer.authorization_seal()))
    }
    pub(crate) fn retain_runtime_failure(
        &self, error: crate::swarm_orchestration::SwarmError,
    ) -> ValidatorAssociationRetainedFailure {
        self.retain_publication_failure(ValidatorAssociationFault::Runtime(error))
    }
    pub(crate) fn retain_registration_failure(
        &self, error: crate::swarm_orchestration::coordinator::ValidatorProducerRegistrationFailure,
    ) -> ValidatorAssociationRetainedFailure {
        use crate::swarm_orchestration::coordinator::ValidatorProducerRegistrationFailure;
        let fault = match error {
            ValidatorProducerRegistrationFailure::Runtime(error) =>
                ValidatorAssociationFault::Runtime(error),
            ValidatorProducerRegistrationFailure::Authority(error) =>
                ValidatorAssociationFault::Authority(error),
        };
        self.retain_publication_failure(fault)
    }
    pub(crate) fn retain_scope_failure(
        &self, error: crate::swarm_orchestration::resource_scope::ScopeDenied,
    ) -> ValidatorAssociationRetainedFailure {
        self.retain_publication_failure(ValidatorAssociationFault::Scope(error))
    }
    pub(crate) fn retain_designation_failure(
        &self, error: crate::swarm_orchestration::model_lane::ModelLaneError,
    ) -> ValidatorAssociationRetainedFailure {
        self.retain_publication_failure(ValidatorAssociationFault::Designation(error))
    }
    pub(crate) fn reject_existing_producer(&self) -> ValidatorAssociationRetainedFailure {
        self.retain_publication_failure(ResourceAuthorityError::InvalidInput(
            "existing producer is not this original association result",
        ).into())
    }
}

impl ValidatorAssociationIssuanceHandle {
    pub(crate) fn retain_public_authority_failure(
        &self, error: ResourceAuthorityError,
    ) -> ValidatorAssociationRetainedFailure {
        self.retain_publication_failure(ValidatorAssociationFault::Authority(error))
    }
}

impl OwnedValidatorAssociation {
    fn require_original_task_binding(&self) -> Result<(), ValidatorAssociationFault> {
        let requested = self.requested_task.get().ok_or(
            ResourceAuthorityError::InvalidInput("original task snapshot not prepared"),
        )?;
        let mapping = self.task_mapping.get().ok_or(
            ResourceAuthorityError::InvalidInput("original server mapping not prepared"),
        )?;
        if !mapping.matches_requested(requested) {
            return Err(ResourceAuthorityError::InvalidInput(
                "original task binding differs",
            ).into());
        }
        self.input.parent.association_task_sources_in_requested_order(mapping)?;
        Ok(())
    }
}

struct ValidatorAssociationQueryObservation<R> {
    precheck: Option<Result<(), ValidatorAssociationFault>>,
    query: Option<Result<Vec<R>, SurrealStorageError>>,
    postcheck: Option<Result<(), ValidatorAssociationFault>>,
}

pub(crate) struct RetainedValidatorAssociationQuery<R> {
    operation_id: Uuid,
    ordinal: usize,
    owner_token: std::sync::Arc<()>,
    observation: std::sync::Arc<std::sync::Mutex<ValidatorAssociationQueryObservation<R>>>,
}

impl<R> RetainedValidatorAssociationQuery<R> {
    pub(crate) fn operation_id(&self) -> Uuid { self.operation_id }
    pub(crate) fn ordinal(&self) -> usize { self.ordinal }

    pub(crate) fn with_observation<T>(
        &self,
        inspect: impl FnOnce(
            Option<&Result<(), ValidatorAssociationFault>>,
            Option<&Result<Vec<R>, SurrealStorageError>>,
            Option<&Result<(), ValidatorAssociationFault>>,
        ) -> T,
    ) -> T {
        let observation = self.observation.lock().unwrap_or_else(|p| p.into_inner());
        inspect(
            observation.precheck.as_ref(),
            observation.query.as_ref(),
            observation.postcheck.as_ref(),
        )
    }
}

impl OwnedValidatorAssociation {
    fn matches_query_observation<R: 'static>(
        &self, retained: &RetainedValidatorAssociationQuery<R>,
    ) -> bool {
        let state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        retained.operation_id == self.input.invocation.operation_id()
            && std::sync::Arc::ptr_eq(&retained.owner_token, &state.query_observation_owner)
            && state.query_observations.get(retained.ordinal)
                .and_then(|entry| entry.downcast_ref::<std::sync::Arc<
                    std::sync::Mutex<ValidatorAssociationQueryObservation<R>>>>())
                .is_some_and(|entry| std::sync::Arc::ptr_eq(entry, &retained.observation))
    }

    fn retain_query_observation<R: Send + 'static>(
        &self,
    ) -> RetainedValidatorAssociationQuery<R> {
        let observation = std::sync::Arc::new(std::sync::Mutex::new(
            ValidatorAssociationQueryObservation {
                precheck: None, query: None, postcheck: None,
            },
        ));
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        let ordinal = state.query_observations.len();
        // Independent SAME-frame custody is installed synchronously, BEFORE
        // even precheck awaits. Dropping a returned handle cannot delete it.
        state.query_observations.push(Box::new(std::sync::Arc::clone(&observation)));
        RetainedValidatorAssociationQuery {
            operation_id: self.input.invocation.operation_id(), ordinal,
            owner_token: std::sync::Arc::clone(&state.query_observation_owner), observation,
        }
    }
}

impl CurrentValidatorAssociationSeal<'_> {
    pub(crate) fn association_event_id(&self) -> &str {
        self.owner.inventory.get().expect("guard retains original inventory")
            .association_event_id.as_str()
    }
    pub(crate) fn matches_observation<R: 'static>(
        &self, retained: &RetainedValidatorAssociationQuery<R>,
    ) -> bool { self.owner.matches_query_observation(retained) }
    pub(crate) fn original_deadline(&self) -> std::time::Instant {
        self.owner.original_deadline
    }
    pub(crate) async fn revalidate(&self) -> Result<(), ValidatorAssociationFault> {
        let current = self.owner.capture_create_read_seal(self.transaction).await?;
        if !std::sync::Arc::ptr_eq(&self.canonical, &current.canonical) {
            return Err(ResourceAuthorityError::InvalidInput(
                "association canonical source changed",
            ).into());
        }
        Ok(())
    }

    pub(crate) async fn query_values_at<R, B>(
        &self, statement: &'static str, bindings: B, statement_index: usize,
    ) -> RetainedValidatorAssociationQuery<R>
    where
        R: surrealdb::types::SurrealValue + Send + 'static,
        B: surrealdb::types::SurrealValue + Send,
    {
        let retained = self.owner.retain_query_observation();
        let precheck = async {
            self.revalidate().await?;
            self.owner.protected_association_query_binding(
                &self.canonical, self.transaction, false,
            )
        }.await;
        let binding = match precheck {
            Ok(binding) => {
                retained.observation.lock().unwrap_or_else(|p| p.into_inner())
                    .precheck = Some(Ok(()));
                binding
            }
            Err(fault) => {
                retained.observation.lock().unwrap_or_else(|p| p.into_inner())
                    .precheck = Some(Err(fault));
                return retained;
            }
        };
        // The private task-local spans only the actual protected query. The
        // append hook runs AFTER caller bindings and overwrites forged keys.
        let result = super::PROTECTED_VALIDATOR_ASSOCIATION_BINDING.scope(
            binding,
            self.transaction.query_values_at(statement, bindings, statement_index),
        ).await;
        retained.observation.lock().unwrap_or_else(|p| p.into_inner())
            .query = Some(result);
        let current = self.revalidate().await;
        retained.observation.lock().unwrap_or_else(|p| p.into_inner())
            .postcheck = Some(current);
        retained
    }
}

impl ValidatorAssociationReadbackGuard<'_> {
    pub(crate) fn association_event_id(&self) -> &str {
        self.owner.inventory.get().expect("guard retains original inventory")
            .association_event_id.as_str()
    }
    pub(crate) fn matches_observation<R: 'static>(
        &self, retained: &RetainedValidatorAssociationQuery<R>,
    ) -> bool { self.owner.matches_query_observation(retained) }
    pub(crate) fn original_deadline(&self) -> std::time::Instant {
        self.owner.original_deadline
    }
    pub(crate) fn transaction(&self)
        -> &super::super::SurrealTransactionContext<'_> { self.transaction }
    pub(crate) async fn revalidate(&self) -> Result<(), ValidatorAssociationFault> {
        let current = self.owner.capture_readback_guard(self.transaction).await?;
        if !std::sync::Arc::ptr_eq(&self.canonical, &current.canonical) {
            return Err(ResourceAuthorityError::InvalidInput(
                "association readback canonical source changed",
            ).into());
        }
        Ok(())
    }

    pub(crate) async fn query_values_at<R, B>(
        &self, statement: &'static str, bindings: B, statement_index: usize,
    ) -> RetainedValidatorAssociationQuery<R>
    where
        R: surrealdb::types::SurrealValue + Send + 'static,
        B: surrealdb::types::SurrealValue + Send,
    {
        let retained = self.owner.retain_query_observation();
        let precheck = async {
            self.revalidate().await?;
            self.owner.protected_association_query_binding(
                &self.canonical, self.transaction, true,
            )
        }.await;
        let binding = match precheck {
            Ok(binding) => {
                retained.observation.lock().unwrap_or_else(|p| p.into_inner())
                    .precheck = Some(Ok(()));
                binding
            }
            Err(fault) => {
                retained.observation.lock().unwrap_or_else(|p| p.into_inner())
                    .precheck = Some(Err(fault));
                return retained;
            }
        };
        // The private task-local spans only the actual protected query. The
        // append hook runs AFTER caller bindings and overwrites forged keys.
        let result = super::PROTECTED_VALIDATOR_ASSOCIATION_BINDING.scope(
            binding,
            self.transaction.query_values_at(statement, bindings, statement_index),
        ).await;
        retained.observation.lock().unwrap_or_else(|p| p.into_inner())
            .query = Some(result);
        let current = self.revalidate().await;
        retained.observation.lock().unwrap_or_else(|p| p.into_inner())
            .postcheck = Some(current);
        retained
    }
}

impl OwnedValidatorAssociation {
    fn protected_association_query_binding(
        &self,
        canonical: &std::sync::Arc<CanonicalValidatorAssociationSource>,
        transaction: &super::super::SurrealTransactionContext<'_>,
        read_primary: bool,
    ) -> Result<super::ProtectedValidatorAssociationBinding, ValidatorAssociationFault> {
        self.require_original_deadline()?;
        self.require_current_runtime_source()?;
        let current = self.committed_canonical_source()?;
        if !std::sync::Arc::ptr_eq(canonical, &current) {
            return Err(ResourceAuthorityError::InvalidInput("association source owner changed").into());
        }
        let (create, read) = self.retained_child_sources()?;
        let primary = if read_primary { &*read } else { &create.auth };
        transaction.require_authenticated_scope(&self.storage, &primary.record_user_scope())?;
        let original = &self.input.invocation.designation().scope;
        let body = canonical.body();
        let matches_scope = |actual: &handshake_producer_association::Scope5V1,
            exact: &crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution| {
            actual.owner_account_id == exact.owner_account_id.to_string()
                && actual.actor_principal_id == exact.actor_principal_id.to_string()
                && actual.authenticated_session_id == exact.authenticated_session_id.to_string()
                && actual.access_space_id == exact.access_space_id.to_string()
                && actual.workspace_id == exact.workspace_id.as_str()
        };
        if body.original_operation_id != self.input.invocation.operation_id()
            || !matches_scope(&body.original_issuer, original)
            || !matches_scope(&body.child_scope, &create.auth.exact)
            || create.auth.exact != read.exact
            || create.auth.action != ResourceAction::Create || create.auth.capability_id != "fs.write"
            || read.action != ResourceAction::Read || read.capability_id != "fs.read"
        {
            return Err(ResourceAuthorityError::InvalidInput("association child binding differs").into());
        }
        let inventory = self.inventory.get().ok_or(ValidatorAssociationFault::MissingCanonicalSourceCapture)?;
        if inventory.association_event_id.is_empty() {
            return Err(ResourceAuthorityError::InvalidInput("association event inventory missing").into());
        }
        let stream = self.original_stream()?;
        let (record_id, record_json, outer_json) =
            super::super::model_lane::validator_association_native_binding_data(body, original, stream)?;
        self.require_original_deadline()?;
        self.require_current_runtime_source()?;
        transaction.require_authenticated_scope(&self.storage, &primary.record_user_scope())?;
        Ok(super::ProtectedValidatorAssociationBinding {
            operation_id: body.original_operation_id.to_string(),
            record_id, event_id: inventory.association_event_id.clone(),
            original_stream: stream.to_owned(), record_json, outer_json,
            original_scope: super::ProtectedRoleScopeBinding::from(original),
            child_scope: super::ProtectedRoleScopeBinding::from(&create.auth.exact),
            expected_child_create: create.auth.authority.clone(),
            expected_child_read: read.authority.clone(),
        })
    }
}

struct ValidatorAssociationDesignation {
    request: crate::swarm_orchestration::model_lane::WorkspaceValidatorRunnerDesignationRequest,
    producer: std::sync::Arc<crate::swarm_orchestration::coordinator::ValidatorProducerKey>,
    event_id: String,
    role: std::sync::OnceLock<ValidatorAssociationRoleCapture>,
    state: std::sync::Mutex<ValidatorAssociationDesignationState>,
    future: tokio::sync::Mutex<Option<std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>>>,
}

struct ValidatorAssociationRoleCapture {
    owner: std::sync::Arc<super::OwnedRoleTransaction>,
    fence: std::sync::Mutex<Option<std::sync::Arc<tokio::sync::OwnedRwLockReadGuard<()>>>>,
}

impl ValidatorAssociationRoleCapture {
    fn from_retention(role: &super::RoleAuthorityTransactionRetention) -> Self {
        Self { owner: std::sync::Arc::clone(&role.owner),
            fence: std::sync::Mutex::new(Some(std::sync::Arc::clone(&role.fence))) }
    }
    fn matches_retention(&self, role: &super::RoleAuthorityTransactionRetention) -> bool {
        std::sync::Arc::ptr_eq(&self.owner, &role.owner)
            && self.fence.lock().unwrap_or_else(|p| p.into_inner()).as_ref()
                .is_some_and(|fence| std::sync::Arc::ptr_eq(fence, &role.fence))
    }
    fn release_publication_fence(&self) {
        self.fence.lock().unwrap_or_else(|p| p.into_inner()).take();
    }
}

struct ValidatorAssociationDesignationState {
    engine_started: bool,
    engine_observation: Option<ValidatorAssociationEngineObservation>,
    callback_result: Option<Box<dyn std::any::Any + Send>>,
    callback_original: Option<Result<Box<dyn std::any::Any + Send>, Box<dyn std::any::Any + Send>>>,
    callback_panic: Option<Box<dyn std::any::Any + Send>>,
    engine_wait: Option<Result<std::sync::Arc<super::super::model_lane::RunnerDesignationReadback>, ResourceAuthorityError>>,
    publication_read: Option<Result<crate::swarm_orchestration::model_lane::WorkspaceOwnerActionResult, SurrealStorageError>>,
    worker_join: Option<tokio::task::JoinError>,
    publication_confirmed: bool,
    query_observations: Vec<Box<dyn std::any::Any + Send>>,
    readback: Option<std::sync::Arc<super::super::model_lane::RunnerDesignationReadback>>,
    outcome: Option<Result<crate::swarm_orchestration::model_lane::WorkspaceOwnerActionResult,
        ValidatorAssociationFault>>,
}

impl ValidatorAssociationTransactionRetention {
    // C2 settlement arm calls this instead of observe_engine. Initial engine
    // state is untouched after the separately fenced designation engine starts.
    pub(crate) fn observe_current_engine(&self, observed: ValidatorAssociationEngineObservation)
        -> Result<(), SurrealStorageError> {
        if let Some(stage) = self.frame.designation.get() {
            let mut state = stage.state.lock().unwrap_or_else(|p| p.into_inner());
            if state.engine_started {
                if state.engine_observation.is_some_and(|previous|
                    previous != ValidatorAssociationEngineObservation::Unobserved && previous != observed) {
                    return Err(SurrealStorageError::TransactionRetentionMismatch { operation_id: self.operation_id() });
                }
                state.engine_observation = Some(observed);
                return Ok(());
            }
        }
        self.observe_engine(observed)
    }
    pub(crate) fn enter_designation_engine_once(
        &self, role: &super::RoleAuthorityTransactionRetention,
    ) -> Result<(), SurrealStorageError> {
        let stage = self.frame.designation.get().ok_or(
            SurrealStorageError::TransactionRetentionMismatch { operation_id: self.operation_id() })?;
        if role.operation_id() != self.operation_id()
            || !stage.role.get().is_some_and(|original| original.matches_retention(role)) {
            return Err(SurrealStorageError::TransactionRetentionMismatch { operation_id: self.operation_id() });
        }
        // Called inside the SAME transaction_settlements lock. The native
        // caller revalidates current runtime and parent Create+Read first.
        let association = self.frame.state.lock().unwrap_or_else(|p| p.into_inner());
        if !matches!(association.outcome.as_ref(), Some(Ok(confirmed))
            if std::ptr::eq(confirmed.seal.as_ref(), stage.producer.authorization_seal())) {
            return Err(SurrealStorageError::TransactionSettlementPending { operation_id: self.operation_id() });
        }
        let mut state = stage.state.lock().unwrap_or_else(|p| p.into_inner());
        if state.engine_started || state.outcome.is_some() {
            return Err(SurrealStorageError::TransactionSettlementPending { operation_id: self.operation_id() });
        }
        state.engine_started = true;
        Ok(())
    }
    pub(crate) fn designation_engine_started(&self) -> bool {
        self.frame.designation.get().is_some_and(|stage|
            stage.state.lock().unwrap_or_else(|p| p.into_inner()).engine_started)
    }
    pub(crate) fn retain_designation_worker_result<T: Send + 'static, E: Send + 'static>(
        &self, result: Result<T, E>,
    ) {
        if let Some(stage) = self.frame.designation.get() {
            let mut state = stage.state.lock().unwrap_or_else(|p| p.into_inner());
            // Exactly one engine callback. Never take or replace its raw result.
            if state.callback_result.is_none() { state.callback_result = Some(Box::new(result)); }
        }
    }
    fn designation_readback(&self)
        -> Result<std::sync::Arc<super::super::model_lane::RunnerDesignationReadback>, ValidatorAssociationFault> {
        let stage = self.frame.designation.get().ok_or(ValidatorAssociationFault::MissingCanonicalSourceCapture)?;
        let mut state = stage.state.lock().unwrap_or_else(|p| p.into_inner());
        if state.engine_observation != Some(ValidatorAssociationEngineObservation::CommitObserved) {
            return Err(SurrealStorageError::TransactionSettlementPending { operation_id: self.operation_id() }.into());
        }

        if !matches!(state.callback_original.as_ref(), Some(Ok(_))) {
            return Err(SurrealStorageError::TransactionSettlementPending { operation_id: self.operation_id() }.into());
        }
        let readback = state.readback.as_ref().map(std::sync::Arc::clone)
            .ok_or(SurrealStorageError::TransactionSettlementPending { operation_id: self.operation_id() })?;
        Ok(readback)
    }
}

impl OwnedValidatorAssociation {
    fn require_designation_input(
        &self,
        request: &crate::swarm_orchestration::model_lane::WorkspaceValidatorRunnerDesignationRequest,
        producer: &crate::swarm_orchestration::coordinator::ValidatorProducerKey,
    ) -> Result<(), ValidatorAssociationFault> {
        let original = self.input.invocation.designation();
        if request.operation_id != original.operation_id || request.action != original.action
            || request.scope != original.scope || request.run != original.run
            || request.subject != original.subject || request.expected != original.expected
            || request.retention != original.retention
            || request.runner.producer_instance_ref != original.runner.instance_ref
            || request.runner.producer_spawn_generation != original.runner.spawn_generation
            || request.runner.producer_registration_ref != producer.registration_ref()
            || &request.runner.producer_registered_principal != producer.registered_principal()
        { return Err(ResourceAuthorityError::InvalidInput("designation differs from original association intent").into()); }
        let state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        if !matches!(state.outcome.as_ref(), Some(Ok(confirmed))
            if std::ptr::eq(confirmed.seal.as_ref(), producer.authorization_seal())) {
            return Err(ResourceAuthorityError::InvalidInput("designation association not canonically confirmed").into());
        }
        Ok(())
    }
    async fn run_designation(
        owner: std::sync::Arc<Self>,
        stage: std::sync::Arc<ValidatorAssociationDesignation>,
        retention: ValidatorAssociationTransactionRetention,
    ) -> Result<crate::swarm_orchestration::model_lane::WorkspaceOwnerActionResult, ValidatorAssociationFault> {
        use crate::swarm_orchestration::model_lane::WorkspaceOwnerActionCommand;
        owner.require_original_deadline()?;
        owner.require_current_sources().await?;
        owner.require_designation_input(&stage.request, &stage.producer)?;
        let original = &owner.input.parent.create;
        let mut auth = capture_owned_canonical_role_auth(
            &owner.storage, &owner.registry, original.credentials.clone(),
            original.credentials.workspace_creation_request(stage.request.scope.workspace_id.as_str()),
        ).await?;
        auth.requires_workspace_owner = true;
        auth.revalidate_for_publication(&owner.storage, &owner.registry).await?;
        let read_request = AuthorizationRequest {
            session_token: original.credentials.session_token.clone(),
            channel_binding_hash: Some(original.credentials.channel_binding_hash.clone()),
            resource_kind: ResourceKind::Workspace,
            external_resource_id: stage.request.scope.workspace_id.as_str().to_owned(),
            capability_id: "fs.read".to_owned(), action: ResourceAction::Read,
        };
        let mut read = capture_owned_canonical_role_auth(
            &owner.storage, &owner.registry, original.credentials.clone(), read_request,
        ).await?;
        read.requires_workspace_owner = true;
        read.revalidate_for_publication(&owner.storage, &owner.registry).await?;
        if auth.exact != stage.request.scope || read.exact != auth.exact
            || auth.authority != owner.input.parent.create.authority
            || read.authority != owner.input.parent.read.authority {
            return Err(ResourceAuthorityError::InvalidInput("designation parent capture changed").into());
        }
        let scope = auth.record_user_scope();
        let role = super::RoleAuthorityTransactionRetention {
            owner: std::sync::Arc::new(super::OwnedRoleTransaction::Owner(super::WorkspaceOwnerActionPermit {
                auth, role_evidence_read: Some(read), registry: std::sync::Arc::new(owner.registry.clone()),
                command: WorkspaceOwnerActionCommand::IssueRunnerDesignation(stage.request.clone()),
            })),
            fence: std::sync::Arc::new(owner.storage.authority_publication_read().await?),
        };
        stage.role.set(ValidatorAssociationRoleCapture::from_retention(&role)).map_err(|_| ResourceAuthorityError::InvalidInput("designation original role already installed"))?;
        let provider = super::super::model_lane::SurrealModelLaneStore::initialize(owner.storage.clone()).await?;
        owner.require_current_sources().await?;
        owner.require_designation_input(&stage.request, &stage.producer)?;
        let worker_role = role.clone();
        let worker_provider = provider.clone();
        let event_id = stage.event_id.clone();
        let worker_stage = std::sync::Arc::clone(&stage);
        let worker_association = retention.clone();
        let worker_result = owner.storage.with_record_user_scope(scope,
            owner.storage.with_owned_validator_designation_authenticated_transaction(
                retention.operation_id(), super::super::ContextOperationCancellation::new(),
                retention.clone(), role.clone(), move |transaction| Box::pin(async move {
                    let super::OwnedRoleTransaction::Owner(permit) = worker_role.owner.as_ref() else {
                        return Err(ResourceAuthorityError::InvalidInput("designation requires retained Owner permit"));
                    };
                    permit.auth.revalidate_in_transaction(&transaction, &permit.registry).await?;
                    let read = permit.role_evidence_read.as_ref().ok_or(
                        ResourceAuthorityError::InvalidInput("designation retained Read capture missing"))?;
                    read.revalidate_in_transaction(&transaction, &permit.registry).await?;
                    permit.auth.revalidate_in_transaction(&transaction, &permit.registry).await?;
                    let seal = super::OwnerIssuanceSeal { permit, transaction: &transaction };
                    let readback = worker_provider.put_runner_designation_with_original_event(
                        &transaction, &seal, &event_id, &worker_association,
                    ).await?;
                    let readback = std::sync::Arc::new(readback);
                    worker_stage.state.lock().unwrap_or_else(|p| p.into_inner())
                        .readback = Some(std::sync::Arc::clone(&readback));
                    // Return is cached by the SAME worker before any result exits.
                    // A failed postcheck is also retained as the raw callback error.
                    permit.auth.revalidate_in_transaction(&transaction, &permit.registry).await?;
                    read.revalidate_in_transaction(&transaction, &permit.registry).await?;
                    permit.auth.revalidate_in_transaction(&transaction, &permit.registry).await?;
                    Ok::<_, ResourceAuthorityError>(readback)
                }),
            )
        ).await;
        // Cache this caller-side typed engine wait result too, independently of
        // the callback/result cache maintained by the engine worker.
        stage.retain_engine_wait(worker_result);
        let readback = retention.designation_readback()?;
        let super::OwnedRoleTransaction::Owner(permit) = role.owner.as_ref() else {
            return Err(ResourceAuthorityError::InvalidInput("designation publication owner differs").into());
        };
        owner.require_current_sources().await?;
        permit.auth.revalidate_for_publication(&owner.storage, &permit.registry).await?;
        let guard = super::RolePublicationGuard { owner: &role.owner, _fence: &role.fence };
        let result = owner.storage.with_record_user_scope(
            permit.auth.record_user_scope(), provider.readback_runner_designation(&guard, &readback),
        ).await;
        // Store raw readback BEFORE the next current-authority await.
        stage.retain_publication_read(result);
        owner.require_current_sources().await?;
        permit.auth.revalidate_for_publication(&owner.storage, &permit.registry).await?;
        guard.require_unexpired()?;
        owner.require_designation_input(&stage.request, &stage.producer)?;
        owner.require_original_deadline()?;
        let result = stage.canonical_publication_result()?;
        owner.storage.confirm_original_validator_designation(&retention, &role)?;
        Ok(result)
    }
}

impl ValidatorAssociationDesignation {
    fn retain_engine_wait(&self, result: Result<std::sync::Arc<super::super::model_lane::RunnerDesignationReadback>, ResourceAuthorityError>) {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        if state.engine_wait.is_none() { state.engine_wait = Some(result); }
    }
    fn retain_publication_read(&self, result: Result<crate::swarm_orchestration::model_lane::WorkspaceOwnerActionResult, SurrealStorageError>) {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        if state.publication_read.is_none() { state.publication_read = Some(result); }
    }
    fn canonical_publication_result(&self)
        -> Result<crate::swarm_orchestration::model_lane::WorkspaceOwnerActionResult, ValidatorAssociationFault> {
        let state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        match state.publication_read.as_ref() {
            Some(Ok(result @ crate::swarm_orchestration::model_lane::WorkspaceOwnerActionResult::RunnerDesignation { request, .. }))
                if request == &self.request => Ok(result.clone()),
            _ => Err(SurrealStorageError::TransactionSettlementPending { operation_id: self.request.operation_id }.into()),
        }
    }
}

impl ValidatorAssociationTransactionRetention {
    pub(crate) fn retain_designation_worker_join(&self, error: tokio::task::JoinError) {
        if let Some(stage) = self.frame.designation.get() {
            let mut state = stage.state.lock().unwrap_or_else(|p| p.into_inner());
            if state.worker_join.is_none() { state.worker_join = Some(error); }
        }
    }
}

impl ValidatorAssociationIssuanceHandle {
    pub(crate) async fn complete_original_designation(
        &mut self,
        request: crate::swarm_orchestration::model_lane::WorkspaceValidatorRunnerDesignationRequest,
        producer: std::sync::Arc<crate::swarm_orchestration::coordinator::ValidatorProducerKey>,
    ) -> Result<crate::swarm_orchestration::model_lane::WorkspaceOwnerActionResult, ValidatorAssociationRetainedFailure> {
        let owner = std::sync::Arc::clone(&self.retention.frame);
        owner.require_designation_input(&request, &producer)
            .map_err(|fault| self.retain_publication_failure(fault))?;
        // The winning OnceLock initializer performs no effect. It retains the
        // genuine new role-event ID once, distinct from association event/op.
        let stage = std::sync::Arc::clone(owner.designation.get_or_init(|| {
            std::sync::Arc::new(ValidatorAssociationDesignation {
                request: request.clone(), producer: std::sync::Arc::clone(&producer),
                event_id: format!("evt-model-lane-{}", Uuid::now_v7()),
                role: std::sync::OnceLock::new(),
                state: std::sync::Mutex::new(ValidatorAssociationDesignationState {
                    engine_started: false, engine_observation: None, callback_result: None,
                    callback_original: None, callback_panic: None,
                    engine_wait: None, publication_read: None, worker_join: None, publication_confirmed: false, query_observations: Vec::new(),
                    readback: None, outcome: None,
                }),
                future: tokio::sync::Mutex::new(None),
            })
        }));
        if stage.request != request || !std::sync::Arc::ptr_eq(&stage.producer, &producer) {
            return Err(self.retain_publication_failure(ResourceAuthorityError::InvalidInput(
                "designation original stage identity differs").into()));
        }
        let existing = {
            let state = stage.state.lock().unwrap_or_else(|p| p.into_inner());
            state.outcome.as_ref().map(|result| result.as_ref().map(Clone::clone).map_err(|_| ()))
        };
        if let Some(result) = existing {
            SurrealStorage::with_operation_deadline(owner.original_deadline, owner.require_current_sources())
                .await.map_err(|fault| self.retain_publication_failure(fault))?;
            return result.map_err(|()| self.retain_publication_failure(
                SurrealStorageError::TransactionSettlementPending { operation_id: request.operation_id }.into()));
        }
        owner.require_original_deadline().map_err(|fault| self.retain_publication_failure(fault))?;
        let retention = self.retention.clone();
        let run = async {
            let mut slot = stage.future.lock().await;
            let complete = stage.state.lock().unwrap_or_else(|p| p.into_inner()).outcome.is_some();
            if complete { return; }
            if slot.is_none() {
                let driver_owner = std::sync::Arc::clone(&owner);
                let driver_stage = std::sync::Arc::clone(&stage);
                *slot = Some(Box::pin(async move {
                    let attempt = std::panic::AssertUnwindSafe(
                        SurrealStorage::with_operation_deadline(driver_owner.original_deadline,
                            OwnedValidatorAssociation::run_designation(driver_owner, std::sync::Arc::clone(&driver_stage), retention)),
                    ).catch_unwind().await;
                    let result = match attempt {
                        Ok(result) => result,
                        Err(payload) => Err(ValidatorAssociationFault::Panicked(payload)),
                    };
                    driver_stage.state.lock().unwrap_or_else(|p| p.into_inner()).outcome = Some(result);
                }));
            }
            // The pinned future stays INSIDE the original frame. Cancellation
            // drops only this borrow/guard; next same-input caller polls it.
            slot.as_mut().expect("original designation future installed").as_mut().await;
            // outcome owns the complete result before completed future drops.
            *slot = None;
        };
        if tokio::time::timeout_at(owner.original_deadline.into(), run).await.is_err() {
            return Err(self.retain_publication_failure(ValidatorAssociationFault::OriginalDeadlineReached {
                original_deadline: owner.original_deadline,
            }));
        }
        SurrealStorage::with_operation_deadline(owner.original_deadline, owner.require_current_sources())
                .await.map_err(|fault| self.retain_publication_failure(fault))?;
        let result = {
            let state = stage.state.lock().unwrap_or_else(|p| p.into_inner());
            state.outcome.as_ref().and_then(|result| result.as_ref().ok()).cloned()
        };
        result.ok_or_else(|| self.retain_publication_failure(
            SurrealStorageError::TransactionSettlementPending { operation_id: request.operation_id }.into()))
    }
}

impl ValidatorAssociationTransactionRetention {
    pub(crate) fn confirm_designation_publication(&self, role: &super::RoleAuthorityTransactionRetention)
        -> Result<(), SurrealStorageError> {
        let stage = self.frame.designation.get().ok_or(
            SurrealStorageError::TransactionRetentionMismatch { operation_id: self.operation_id() })?;
        if !stage.role.get().is_some_and(|original| original.matches_retention(role)) {
            return Err(SurrealStorageError::TransactionRetentionMismatch { operation_id: self.operation_id() });
        }
        let mut state = stage.state.lock().unwrap_or_else(|p| p.into_inner());
        if state.engine_observation != Some(ValidatorAssociationEngineObservation::CommitObserved)
            || !matches!(state.publication_read.as_ref(), Some(Ok(
                crate::swarm_orchestration::model_lane::WorkspaceOwnerActionResult::RunnerDesignation { request, .. }
            )) if request == &stage.request) {
            return Err(SurrealStorageError::TransactionSettlementPending { operation_id: self.operation_id() });
        }
        state.publication_confirmed = true;
        stage.role.get().expect("matching original role capture").release_publication_fence();
        Ok(())
    }
}

impl ValidatorAssociationTransactionRetention {
    pub(crate) fn retain_designation_callback_value<T: Send + 'static>(&self, value: &mut Option<T>) {
        if let Some(stage) = self.frame.designation.get() {
            let mut state = stage.state.lock().unwrap_or_else(|p| p.into_inner());
            if state.engine_started && state.callback_original.is_none() {
                if let Some(value) = value.take() { state.callback_original = Some(Ok(Box::new(value))); }
            }
        }
    }
    pub(crate) fn retain_designation_callback_error<E: Send + 'static>(&self, error: &mut Option<E>) {
        if let Some(stage) = self.frame.designation.get() {
            let mut state = stage.state.lock().unwrap_or_else(|p| p.into_inner());
            if state.engine_started && state.callback_original.is_none() {
                if let Some(error) = error.take() { state.callback_original = Some(Err(Box::new(error))); }
            }
        }
    }
    pub(crate) fn retain_designation_callback_panic(&self, payload: Box<dyn std::any::Any + Send>) {
        if let Some(stage) = self.frame.designation.get() {
            let mut state = stage.state.lock().unwrap_or_else(|p| p.into_inner());
            if state.callback_panic.is_none() { state.callback_panic = Some(payload); }
        }
    }
}

impl ValidatorAssociationIssuanceHandle {
    pub(crate) fn with_original_designation_observation<T>(
        &self,
        inspect: impl FnOnce(
            Option<ValidatorAssociationEngineObservation>,
            Option<&Result<Box<dyn std::any::Any + Send>, Box<dyn std::any::Any + Send>>>,
            Option<&(dyn std::any::Any + Send)>,
            Option<&(dyn std::any::Any + Send)>,
            Option<&tokio::task::JoinError>,
            Option<&Result<crate::swarm_orchestration::model_lane::WorkspaceOwnerActionResult, SurrealStorageError>>,
            Option<&ValidatorAssociationFault>,
            bool,
        ) -> T,
    ) -> Option<T> {
        let stage = self.retention.frame.designation.get()?;
        let state = stage.state.lock().unwrap_or_else(|p| p.into_inner());
        Some(inspect(
            state.engine_observation,
            state.callback_original.as_ref(),
            state.callback_panic.as_deref(),
            state.callback_result.as_deref(),
            state.worker_join.as_ref(),
            state.publication_read.as_ref(),
            state.outcome.as_ref().and_then(|result| result.as_ref().err()),
            state.publication_confirmed,
        ))
    }
}

struct ValidatorDesignationQueryObservation<R> {
    query: Option<Result<Vec<R>, SurrealStorageError>>,
    link: Option<Result<(), SurrealStorageError>>,
    rejected_queries: Vec<Result<Vec<R>, SurrealStorageError>>,
    rejected_links: Vec<Result<(), SurrealStorageError>>,
}

pub(crate) struct RetainedValidatorDesignationQuery<R> {
    observation: std::sync::Arc<std::sync::Mutex<ValidatorDesignationQueryObservation<R>>>,
}

impl<R> RetainedValidatorDesignationQuery<R> {
    pub(crate) fn with_observation<T>(&self, inspect: impl FnOnce(
        Option<&Result<Vec<R>, SurrealStorageError>>, Option<&Result<(), SurrealStorageError>>,
    ) -> T) -> T {
        let state = self.observation.lock().unwrap_or_else(|p| p.into_inner());
        inspect(state.query.as_ref(), state.link.as_ref())
    }
    pub(crate) fn retain_query(&self, result: Result<Vec<R>, SurrealStorageError>) {
        let mut state = self.observation.lock().unwrap_or_else(|p| p.into_inner());
        if state.query.is_none() { state.query = Some(result); }
        else { state.rejected_queries.push(result); }
    }
    pub(crate) fn retain_link(&self, result: Result<(), SurrealStorageError>) {
        let mut state = self.observation.lock().unwrap_or_else(|p| p.into_inner());
        if state.link.is_none() { state.link = Some(result); }
        else { state.rejected_links.push(result); }
    }
}

impl ValidatorAssociationTransactionRetention {
    pub(crate) fn matches_designation_issuer(&self, issuer: &super::OwnerIssuanceSeal<'_>, event_id: &str) -> bool {
        self.frame.designation.get().is_some_and(|stage| {
            stage.event_id == event_id && issuer.operation_id() == self.operation_id()
                && stage.role.get().is_some_and(|role| matches!(role.owner.as_ref(),
                    super::OwnedRoleTransaction::Owner(permit) if std::ptr::eq(permit, issuer.permit)))
        })
    }
    pub(crate) fn retain_designation_query<R: Send + 'static>(&self)
        -> Result<RetainedValidatorDesignationQuery<R>, SurrealStorageError> {
        let stage = self.frame.designation.get().ok_or(
            SurrealStorageError::TransactionRetentionMismatch { operation_id: self.operation_id() })?;
        let observation = std::sync::Arc::new(std::sync::Mutex::new(
            ValidatorDesignationQueryObservation { query: None, link: None, rejected_queries: Vec::new(), rejected_links: Vec::new() }));
        stage.state.lock().unwrap_or_else(|p| p.into_inner()).query_observations
            .push(Box::new(std::sync::Arc::clone(&observation)));
        Ok(RetainedValidatorDesignationQuery { observation })
    }
    pub(crate) fn with_designation_query<R: 'static, T>(&self, ordinal: usize,
        inspect: impl FnOnce(Option<&Result<Vec<R>, SurrealStorageError>>, Option<&Result<(), SurrealStorageError>>) -> T,
    ) -> Option<T> {
        let stage = self.frame.designation.get()?;
        let state = stage.state.lock().unwrap_or_else(|p| p.into_inner());
        let observation = state.query_observations.get(ordinal)?.downcast_ref::<
            std::sync::Arc<std::sync::Mutex<ValidatorDesignationQueryObservation<R>>>>()?;
        let observed = observation.lock().unwrap_or_else(|p| p.into_inner());
        Some(inspect(observed.query.as_ref(), observed.link.as_ref()))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ValidatorAssociationWorkerPhase { InitialWrite, InitialRead, Designation }

#[derive(Default)]
struct OriginalAssociationWorkerState {
    started: bool,
    observation: Option<ValidatorAssociationEngineObservation>,
    callback: Option<Result<Box<dyn std::any::Any + Send>, Box<dyn std::any::Any + Send>>>,
    callback_panic: Option<Box<dyn std::any::Any + Send>>,
    final_result: Option<Box<dyn std::any::Any + Send>>,
    engine_wait: Option<Box<dyn std::any::Any + Send>>,
    join: Option<tokio::task::JoinError>,
    task: Option<Box<dyn std::any::Any + Send>>,
    task_join_observed: bool,
    rejected_tasks: Vec<Box<dyn std::any::Any + Send>>,
    write_readback: Option<std::sync::Arc<super::super::model_lane::ValidatorAssociationCommitReadback>>,
    checked: Option<std::sync::Arc<handshake_producer_association::CheckedProducerAssociation>>,
}

impl ValidatorAssociationState {
    fn initial_worker(&self, phase: ValidatorAssociationWorkerPhase) -> Option<&OriginalAssociationWorkerState> {
        match phase { ValidatorAssociationWorkerPhase::InitialWrite => Some(&self.initial_write),
            ValidatorAssociationWorkerPhase::InitialRead => Some(&self.initial_read),
            ValidatorAssociationWorkerPhase::Designation => None }
    }
    fn initial_worker_mut(&mut self, phase: ValidatorAssociationWorkerPhase) -> Option<&mut OriginalAssociationWorkerState> {
        match phase { ValidatorAssociationWorkerPhase::InitialWrite => Some(&mut self.initial_write),
            ValidatorAssociationWorkerPhase::InitialRead => Some(&mut self.initial_read),
            ValidatorAssociationWorkerPhase::Designation => None }
    }
}

impl ValidatorAssociationTransactionRetention {
    pub(crate) fn worker_phase(&self) -> ValidatorAssociationWorkerPhase {
        if self.designation_engine_started() { return ValidatorAssociationWorkerPhase::Designation; }
        if self.frame.state.lock().unwrap_or_else(|p| p.into_inner()).initial_read.started {
            ValidatorAssociationWorkerPhase::InitialRead
        } else { ValidatorAssociationWorkerPhase::InitialWrite }
    }
    pub(crate) fn enter_initial_read_once(&self) -> Result<(), SurrealStorageError> {
        let mut state = self.frame.state.lock().unwrap_or_else(|p| p.into_inner());
        if state.engine_observation != Some(ValidatorAssociationEngineObservation::CommitObserved)
            || !matches!(state.initial_write.callback.as_ref(), Some(Ok(_)))
            || state.initial_write.write_readback.is_none() || state.initial_read.started
            || state.outcome.is_some() {
            return Err(SurrealStorageError::TransactionSettlementPending { operation_id: self.operation_id() });
        }
        state.initial_read.started = true;
        Ok(())
    }
    pub(crate) fn observe_worker_phase(&self, phase: ValidatorAssociationWorkerPhase,
        observed: ValidatorAssociationEngineObservation) -> Result<(), SurrealStorageError> {
        match phase {
            ValidatorAssociationWorkerPhase::Designation => self.observe_current_engine(observed),
            ValidatorAssociationWorkerPhase::InitialWrite => self.observe_engine(observed),
            ValidatorAssociationWorkerPhase::InitialRead => {
                let mut state = self.frame.state.lock().unwrap_or_else(|p| p.into_inner());
                if !state.initial_read.started || state.initial_read.observation.is_some_and(|old|
                    old != ValidatorAssociationEngineObservation::Unobserved && old != observed) {
                    return Err(SurrealStorageError::TransactionRetentionMismatch { operation_id: self.operation_id() });
                }
                state.initial_read.observation = Some(observed);
                Ok(())
            }
        }
    }
    pub(crate) fn retain_worker_callback_value<T: Send + 'static>(&self,
        phase: ValidatorAssociationWorkerPhase, value: &mut Option<T>) {
        if phase == ValidatorAssociationWorkerPhase::Designation { self.retain_designation_callback_value(value); return; }
        let mut state = self.frame.state.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(target) = state.initial_worker_mut(phase) {
            if target.callback.is_none() { if let Some(value) = value.take() { target.callback = Some(Ok(Box::new(value))); } }
        }
    }
    pub(crate) fn retain_worker_callback_error<E: Send + 'static>(&self,
        phase: ValidatorAssociationWorkerPhase, error: &mut Option<E>) {
        if phase == ValidatorAssociationWorkerPhase::Designation { self.retain_designation_callback_error(error); return; }
        let mut state = self.frame.state.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(target) = state.initial_worker_mut(phase) {
            if target.callback.is_none() { if let Some(error) = error.take() { target.callback = Some(Err(Box::new(error))); } }
        }
    }
    pub(crate) fn retain_worker_callback_panic(&self, phase: ValidatorAssociationWorkerPhase, payload: Box<dyn std::any::Any + Send>) {
        if phase == ValidatorAssociationWorkerPhase::Designation { self.retain_designation_callback_panic(payload); return; }
        let mut state = self.frame.state.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(target) = state.initial_worker_mut(phase) { if target.callback_panic.is_none() { target.callback_panic = Some(payload); } }
    }
    pub(crate) fn retain_worker_final<T: Send + 'static, E: Send + 'static>(&self,
        phase: ValidatorAssociationWorkerPhase, result: Result<T, E>) {
        if phase == ValidatorAssociationWorkerPhase::Designation { self.retain_designation_worker_result(result); return; }
        let mut state = self.frame.state.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(target) = state.initial_worker_mut(phase) { if target.final_result.is_none() { target.final_result = Some(Box::new(result)); } }
    }
    pub(crate) fn retain_worker_join(&self, phase: ValidatorAssociationWorkerPhase, error: tokio::task::JoinError) {
        if phase == ValidatorAssociationWorkerPhase::Designation { self.retain_designation_worker_join(error); return; }
        let mut state = self.frame.state.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(target) = state.initial_worker_mut(phase) { if target.join.is_none() { target.join = Some(error); } }
    }
}

impl ValidatorAssociationTransactionRetention {
    pub(crate) fn retain_initial_worker_task<T: Send + 'static, E: Send + 'static>(
        &self, phase: ValidatorAssociationWorkerPhase, task: tokio::task::JoinHandle<Result<T, E>>,
    ) -> Result<std::sync::Arc<tokio::sync::Mutex<tokio::task::JoinHandle<Result<T, E>>>>, SurrealStorageError> {
        let mut state = self.frame.state.lock().unwrap_or_else(|p| p.into_inner());
        let target = state.initial_worker_mut(phase).ok_or(
            SurrealStorageError::TransactionRetentionMismatch { operation_id: self.operation_id() })?;
        let task = std::sync::Arc::new(tokio::sync::Mutex::new(task));
        if target.task.is_some() {
            // Preserve an unexpected incoming owned task before rejection.
            target.rejected_tasks.push(Box::new(task));
            return Err(SurrealStorageError::TransactionRetentionMismatch { operation_id: self.operation_id() });
        }
        target.task = Some(Box::new(std::sync::Arc::clone(&task)));
        Ok(task)
    }
    pub(crate) fn mark_initial_worker_join_observed(&self, phase: ValidatorAssociationWorkerPhase) {
        let mut state = self.frame.state.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(target) = state.initial_worker_mut(phase) { target.task_join_observed = true; }
    }
    fn poll_initial_worker_join<T: Send + 'static, E: Send + 'static>(
        &self, phase: ValidatorAssociationWorkerPhase, context: &mut std::task::Context<'_>,
    ) {
        let task = {
            let state = self.frame.state.lock().unwrap_or_else(|p| p.into_inner());
            let Some(target) = state.initial_worker(phase) else { return; };
            if target.task_join_observed { return; }
            target.task.as_ref().and_then(|task| task.downcast_ref::<std::sync::Arc<
                tokio::sync::Mutex<tokio::task::JoinHandle<Result<T, E>>>>>()).map(std::sync::Arc::clone)
        };
        let Some(task) = task else { return; };
        let Ok(mut task) = task.try_lock() else { return; };
        if self.initial_worker_join_observed(phase) { return; }
        match std::future::Future::poll(std::pin::Pin::new(&mut *task), context) {
            std::task::Poll::Pending => {},
            std::task::Poll::Ready(result) => {
                self.mark_initial_worker_join_observed(phase);
                match result {
                    Ok(result) => {
                        // The worker already cached its actual final_result.
                        // Keep the actual returned projection independently too.
                        let mut state = self.frame.state.lock().unwrap_or_else(|p| p.into_inner());
                        if let Some(target) = state.initial_worker_mut(phase) { target.engine_wait = Some(Box::new(result)); }
                    },
                    Err(error) => self.retain_worker_join(phase, error),
                }
            }
        }
    }
    fn poll_initial_worker_joins(&self, context: &mut std::task::Context<'_>) {
        self.poll_initial_worker_join::<std::sync::Arc<super::super::model_lane::ValidatorAssociationCommitReadback>, ValidatorAssociationFault>(
            ValidatorAssociationWorkerPhase::InitialWrite, context);
        self.poll_initial_worker_join::<std::sync::Arc<handshake_producer_association::CheckedProducerAssociation>, ValidatorAssociationFault>(
            ValidatorAssociationWorkerPhase::InitialRead, context);
    }
}

impl ValidatorAssociationIssuanceHandle {
    pub(crate) fn with_original_initial_worker<T>(&self, phase: ValidatorAssociationWorkerPhase,
        inspect: impl FnOnce(Option<ValidatorAssociationEngineObservation>,
            Option<&Result<Box<dyn std::any::Any + Send>, Box<dyn std::any::Any + Send>>>,
            Option<&(dyn std::any::Any + Send)>, Option<&(dyn std::any::Any + Send)>,
            Option<&tokio::task::JoinError>, bool) -> T,
    ) -> Option<T> {
        let state = self.retention.frame.state.lock().unwrap_or_else(|p| p.into_inner());
        let target = state.initial_worker(phase)?;
        let observation = if phase == ValidatorAssociationWorkerPhase::InitialWrite {
            state.engine_observation
        } else { target.observation };
        Some(inspect(observation, target.callback.as_ref(), target.callback_panic.as_deref(),
            target.final_result.as_deref(), target.join.as_ref(), target.task_join_observed))
    }
}

impl ValidatorAssociationTransactionRetention {
    pub(crate) fn initial_worker_join_observed(&self, phase: ValidatorAssociationWorkerPhase) -> bool {
        self.frame.state.lock().unwrap_or_else(|p| p.into_inner()).initial_worker(phase)
            .is_some_and(|target| target.task_join_observed)
    }
}

impl std::fmt::Debug for ValidatorAssociationReserveRejection {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ValidatorAssociationReserveRejection { .. }")
    }
}
impl std::fmt::Debug for ValidatorAssociationRetainedFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ValidatorAssociationRetainedFailure { .. }")
    }
}

async fn revalidate_secondary_validator_source(
    secondary: &OwnedCanonicalAuthSnapshot,
    primary: &OwnedCanonicalAuthSnapshot,
    transaction: &super::super::SurrealTransactionContext<'_>,
    registry: &crate::capabilities::CapabilityRegistry,
) -> Result<(), ResourceAuthorityError> {
    let denied = || ResourceAuthorityError::Denied {
        decision_id: secondary.decision.decision_id.clone(),
    };
    // The private caller has already run primary.revalidate_in_transaction.
    // Never switch the transaction identity, grant, purpose or action.
    if secondary.exact != primary.exact
        || secondary.authority.resource_id != primary.authority.resource_id
        || secondary.credentials.session_token != primary.credentials.session_token
        || secondary.credentials.channel_binding_hash != primary.credentials.channel_binding_hash
        || secondary.authority.delegation_chain != primary.authority.delegation_chain
        || secondary.authority.resource_kind != "workspace"
        || !registry.profile_can(
            &secondary.authority.capability_profile_id, &secondary.capability_id,
        ).unwrap_or(false)
    {
        return Err(denied());
    }
    let rows: Vec<RegistryPublicationAuthority> = transaction.query_values_at(
        REGISTRY_PUBLICATION_AUTHORITY,
        RegistryPublicationBindings {
            resource: secondary.authority.resource_id.clone(),
            original_grant: secondary.authority.grant_id.clone(),
            kind: secondary.authority.resource_kind.clone(),
            action: secondary.action.as_str().to_owned(),
            capability: secondary.capability_id.clone(),
        }, 0,
    ).await?;
    if rows.as_slice() != std::slice::from_ref(&secondary.authority) {
        return Err(denied());
    }
    if secondary.requires_workspace_owner {
        let eligible: Vec<RecordId> = transaction.query_values_at(
            OWNER_ELIGIBILITY,
            OwnerEligibilityBindings { session_record: secondary.authority.session_id.clone() },
            0,
        ).await?;
        if eligible.as_slice() != std::slice::from_ref(&secondary.authority.session_id) {
            return Err(denied());
        }
    }
    secondary.require_unexpired()
}

impl OwnedValidatorAssociation {
    fn require_current_runtime_source(&self) -> Result<(), ValidatorAssociationFault> {
        self.require_original_deadline()?;
        let actual = self.input.coordinator.original_operator_chat_launch(
            &self.input.instance, &self.input.invocation.designation().scope,
        ).map_err(ValidatorAssociationFault::Runtime)?;
        if !std::sync::Arc::ptr_eq(&actual, self.input.invocation.source()) {
            return Err(ResourceAuthorityError::InvalidInput(
                "original runtime incarnation differs",
            ).into());
        }
        Ok(())
    }
    fn original_stream(&self) -> Result<&str, ValidatorAssociationFault> {
        self.input.invocation.source().launch_contract()
            .map(|contract| contract.event_ledger_stream_id.as_str())
            .ok_or(ValidatorAssociationFault::MissingCanonicalSourceCapture)
    }
}

impl CurrentValidatorAssociationSeal<'_> {
    pub(crate) fn operation_id(&self) -> Uuid { self.owner.input.invocation.operation_id() }
    pub(crate) fn original_scope(&self)
        -> &crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution {
        &self.owner.input.invocation.designation().scope
    }
    pub(crate) fn original_stream(&self) -> &str { self.original_stream }
    pub(crate) fn invocation(&self)
        -> &crate::swarm_orchestration::operator_chat::ValidatorAssociationInvocation {
        &self.owner.input.invocation
    }
    pub(crate) fn matches_transaction(&self,
        transaction: &super::super::SurrealTransactionContext<'_>) -> bool {
        std::ptr::eq(self.transaction, transaction)
    }
    
}

impl ValidatorAssociationReadbackGuard<'_> {
    pub(crate) fn operation_id(&self) -> Uuid { self.owner.input.invocation.operation_id() }
    pub(crate) fn original_scope(&self)
        -> &crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution {
        &self.owner.input.invocation.designation().scope
    }
    pub(crate) fn original_stream(&self) -> &str { self.original_stream }
    pub(crate) fn matches_transaction(&self,
        transaction: &super::super::SurrealTransactionContext<'_>) -> bool {
        std::ptr::eq(self.transaction, transaction)
    }
}
