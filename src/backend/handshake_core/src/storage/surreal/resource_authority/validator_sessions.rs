use super::*;

pub(crate) struct ValidatorDelegationParent(OwnedCanonicalAuthSnapshot);

// This transaction owns identity issuance only. Ordinary resource operations
// subsequently authenticate as the child record user and retain table permissions.
const ISSUE_CHILD: &str = r#"
BEGIN TRANSACTION;
LET $parent = (SELECT * FROM ONLY $parent_record);
LET $grant = (SELECT * FROM ONLY $parent_grant);
IF $parent = NONE OR !fn::mt109_session_live($parent_record)
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
    OR $resource.resource_kind != 'workspace' OR $resource.external_resource_id != $workspace
    OR $resource.lifecycle_state != 'active' OR $resource.created_by_principal_id.status != 'enabled'
    OR $resource.owner_account_id != $account OR $resource.access_space_id != $space
    OR $resource.policy_version > $grant.policy_version OR $grant.policy_version > $policy
    OR $parent.expires_at <= time::now() OR $expires_at > $parent.expires_at
    OR ($grant.expires_at != NONE AND $expires_at > $grant.expires_at)
{ THROW 'HSK-403-PROTECTED-RESOURCE'; };
LET $chain = array::append($parent.delegation_chain, $principal_key);
LET $ancestors = array::append($parent.ancestor_session_ids ?? [], $parent_record);
CREATE $principal SET principal_key = $principal_key, account_id = $account,
    principal_kind = 'spawned_agent', actor_kind = 'agent', actor_id = $actor_id,
    capability_profile_id = $profile, delegated_capabilities = ['fs.write'],
    status = 'enabled', revocation_epoch = 0, policy_version = $policy,
    created_at = time::now(), updated_at = time::now();
CREATE $child_grant SET account_id = $account, principal_id = $principal,
    access_space_id = $space, resource_id = $resource, actions = ['create'],
    capability_ids = ['fs.write'], delegation_chain = $chain, status = 'active',
    grant_version = 1, policy_version = $grant.policy_version, expires_at = $expires_at,
    revoked_at = NONE, created_at = time::now(), updated_at = time::now();
CREATE $child_session SET account_id = $account, principal_id = $principal,
    access_space_id = $space, token_hash = $child_token_hash, channel_binding_hash = $child_channel,
    authentication_strength = 'local_delegated_opaque', delegated_capabilities = ['fs.write'],
    delegation_chain = $chain, parent_session_id = $parent_record, ancestor_session_ids = $ancestors,
    delegation_grant_edges = [{ child_grant_id: $child_grant, parent_grant_id: $parent_grant }],
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
        let parent = capture_owned_canonical_role_auth(self, registry, credentials, request).await?;
        if &parent.exact != exact {
            return Err(ResourceAuthorityError::InvalidInput("delegation parent scope mismatch"));
        }
        Ok(ValidatorDelegationParent(parent))
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
        let parent = parent.0;
        let exact = parent.exact.clone();
        parent.revalidate_for_publication(self, registry).await?;
        let mut expires_at = parent.authority.session_expires_at.clone().into_inner();
        if let Some(expiry) = &parent.authority.grant_expires_at {
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
        let child_token_hash = sha256_hex(child_credentials.session_token.as_bytes());
        let child_channel = child_credentials.channel_binding_hash.clone();
        let parent_token_hash = sha256_hex(parent.credentials.session_token.as_bytes());
        let parent_channel = parent.credentials.channel_binding_hash.clone();
        let authority = parent.authority.clone();
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
        let auth = match capture_owned_canonical_role_auth(self, registry, child_credentials, request).await {
            Ok(auth) => auth,
            Err(error) => {
                self.revoke_session(&child_session_id).await?;
                return Err(error);
            }
        };
        Ok(AuthenticatedValidatorLaunchSeal {
            auth, registry: registry.clone(), storage: self.clone(), instance: instance.clone(),
            parent_session_id, delegating_scope: exact, expires_at,
        })
    }
}
