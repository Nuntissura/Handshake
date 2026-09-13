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

pub const AUTHORITY_DATABASE: &str = "resource_authority";
pub const AUTHORITY_ACCESS_METHOD: &str = "authenticated_session";
pub const RECONCILIATION_QUEUE_ID: &str = "mt109-protected-reconciliation";
pub const RECONCILIATION_PROFILE_ID: &str = "MT109Reconciler";

const AUTHORITY_SCHEMA: &str = r#"
DEFINE TABLE IF NOT EXISTS local_accounts TYPE NORMAL SCHEMAFULL
  PERMISSIONS FOR select WHERE id = $auth.account_id, FOR create, update, delete NONE;
DEFINE FIELD IF NOT EXISTS account_key ON local_accounts TYPE string ASSERT string::len($value) > 0 PERMISSIONS NONE;
DEFINE FIELD IF NOT EXISTS account_role ON local_accounts TYPE string ASSERT $value IN ['Owner', 'Admin', 'Member'];
DEFINE FIELD IF NOT EXISTS status ON local_accounts TYPE string ASSERT $value IN ['enabled', 'disabled'];
DEFINE FIELD IF NOT EXISTS revocation_epoch ON local_accounts TYPE int ASSERT $value >= 0;
DEFINE FIELD IF NOT EXISTS policy_version ON local_accounts TYPE int ASSERT $value >= 1;
DEFINE FIELD IF NOT EXISTS created_at ON local_accounts TYPE datetime;
DEFINE FIELD IF NOT EXISTS updated_at ON local_accounts TYPE datetime;
DEFINE INDEX IF NOT EXISTS local_accounts_account_key_unique ON local_accounts FIELDS account_key UNIQUE;

DEFINE TABLE IF NOT EXISTS principals TYPE NORMAL SCHEMAFULL
  PERMISSIONS FOR select WHERE id = $auth.principal_id, FOR create, update, delete NONE;
DEFINE FIELD IF NOT EXISTS principal_key ON principals TYPE string ASSERT string::len($value) > 0 PERMISSIONS NONE;
DEFINE FIELD IF NOT EXISTS account_id ON principals TYPE record<local_accounts>;
DEFINE FIELD IF NOT EXISTS principal_kind ON principals TYPE string ASSERT $value IN ['human_account', 'local_model', 'remote_model', 'spawned_agent', 'mcp_client', 'service_identity', 'visitor_pass', 'break_glass_operator'];
DEFINE FIELD IF NOT EXISTS actor_kind ON principals TYPE string ASSERT string::len($value) > 0;
DEFINE FIELD IF NOT EXISTS actor_id ON principals TYPE string ASSERT string::len($value) > 0;
DEFINE FIELD IF NOT EXISTS capability_profile_id ON principals TYPE string ASSERT string::len($value) > 0;
DEFINE FIELD IF NOT EXISTS delegated_capabilities ON principals TYPE array<string>;
DEFINE FIELD IF NOT EXISTS status ON principals TYPE string ASSERT $value IN ['enabled', 'disabled'];
DEFINE FIELD IF NOT EXISTS revocation_epoch ON principals TYPE int ASSERT $value >= 0;
DEFINE FIELD IF NOT EXISTS policy_version ON principals TYPE int ASSERT $value >= 1;
DEFINE FIELD IF NOT EXISTS created_at ON principals TYPE datetime;
DEFINE FIELD IF NOT EXISTS updated_at ON principals TYPE datetime;
DEFINE INDEX IF NOT EXISTS principals_principal_key_unique ON principals FIELDS principal_key UNIQUE;
DEFINE INDEX IF NOT EXISTS principals_account_idx ON principals FIELDS account_id;

DEFINE TABLE IF NOT EXISTS access_spaces TYPE NORMAL SCHEMAFULL
  PERMISSIONS FOR select WHERE account_id = $auth.account_id AND id = $auth.access_space_id, FOR create, update, delete NONE;
DEFINE FIELD IF NOT EXISTS space_key ON access_spaces TYPE string ASSERT string::len($value) > 0 PERMISSIONS NONE;
DEFINE FIELD IF NOT EXISTS account_id ON access_spaces TYPE record<local_accounts>;
DEFINE FIELD IF NOT EXISTS name ON access_spaces TYPE string ASSERT string::len($value) > 0;
DEFINE FIELD IF NOT EXISTS status ON access_spaces TYPE string ASSERT $value IN ['active', 'disabled'];
DEFINE FIELD IF NOT EXISTS revocation_epoch ON access_spaces TYPE int ASSERT $value >= 0;
DEFINE FIELD IF NOT EXISTS policy_version ON access_spaces TYPE int ASSERT $value >= 1;
DEFINE FIELD IF NOT EXISTS created_at ON access_spaces TYPE datetime;
DEFINE FIELD IF NOT EXISTS updated_at ON access_spaces TYPE datetime;
DEFINE INDEX IF NOT EXISTS access_spaces_key_unique ON access_spaces FIELDS space_key UNIQUE;
DEFINE INDEX IF NOT EXISTS access_spaces_account_idx ON access_spaces FIELDS account_id;

DEFINE TABLE IF NOT EXISTS authenticated_sessions TYPE NORMAL SCHEMAFULL
  PERMISSIONS FOR select WHERE id = $auth.id, FOR create, update, delete NONE;
DEFINE FIELD IF NOT EXISTS account_id ON authenticated_sessions TYPE record<local_accounts>;
DEFINE FIELD IF NOT EXISTS principal_id ON authenticated_sessions TYPE record<principals>;
DEFINE FIELD IF NOT EXISTS access_space_id ON authenticated_sessions TYPE record<access_spaces>;
DEFINE FIELD IF NOT EXISTS token_hash ON authenticated_sessions TYPE string ASSERT string::len($value) = 64 PERMISSIONS NONE;
DEFINE FIELD IF NOT EXISTS channel_binding_hash ON authenticated_sessions TYPE option<string> PERMISSIONS NONE;
DEFINE FIELD IF NOT EXISTS authentication_strength ON authenticated_sessions TYPE string;
DEFINE FIELD IF NOT EXISTS delegated_capabilities ON authenticated_sessions TYPE array<string>;
DEFINE FIELD IF NOT EXISTS delegation_chain ON authenticated_sessions TYPE array<string>;
DEFINE FIELD IF NOT EXISTS account_revocation_epoch ON authenticated_sessions TYPE int;
DEFINE FIELD IF NOT EXISTS principal_revocation_epoch ON authenticated_sessions TYPE int;
DEFINE FIELD IF NOT EXISTS space_revocation_epoch ON authenticated_sessions TYPE int;
DEFINE FIELD IF NOT EXISTS policy_version ON authenticated_sessions TYPE int ASSERT $value >= 1;
DEFINE FIELD IF NOT EXISTS issued_at ON authenticated_sessions TYPE datetime;
DEFINE FIELD IF NOT EXISTS expires_at ON authenticated_sessions TYPE datetime;
DEFINE FIELD IF NOT EXISTS revoked_at ON authenticated_sessions TYPE option<datetime>;
DEFINE INDEX IF NOT EXISTS authenticated_sessions_token_hash_unique ON authenticated_sessions FIELDS token_hash UNIQUE;
DEFINE INDEX IF NOT EXISTS authenticated_sessions_principal_idx ON authenticated_sessions FIELDS principal_id;

DEFINE TABLE IF NOT EXISTS protected_resources TYPE NORMAL SCHEMAFULL
  PERMISSIONS FOR select WHERE
    lifecycle_state = 'active'
    AND owner_account_id = $auth.account_id
    AND access_space_id = $auth.access_space_id
    AND owner_account_id.status = 'enabled'
    AND created_by_principal_id.status = 'enabled'
    AND access_space_id.status = 'active'
    AND array::len((SELECT id FROM resource_grants WHERE resource_id = $parent.id AND status = 'active' AND revoked_at = NONE AND (expires_at = NONE OR expires_at > time::now()) AND account_id = $auth.account_id AND principal_id = $auth.principal_id AND access_space_id = $auth.access_space_id)) > 0,
  FOR create, update, delete NONE;
DEFINE FIELD IF NOT EXISTS resource_kind ON protected_resources TYPE string ASSERT string::len($value) > 0;
DEFINE FIELD IF NOT EXISTS external_resource_id ON protected_resources TYPE string ASSERT string::len($value) > 0 PERMISSIONS NONE;
DEFINE FIELD IF NOT EXISTS owner_account_id ON protected_resources TYPE record<local_accounts>;
DEFINE FIELD IF NOT EXISTS created_by_principal_id ON protected_resources TYPE record<principals>;
DEFINE FIELD IF NOT EXISTS created_in_session_id ON protected_resources TYPE option<record<authenticated_sessions>>;
DEFINE FIELD IF NOT EXISTS access_space_id ON protected_resources TYPE record<access_spaces>;
DEFINE FIELD IF NOT EXISTS parent_resource_id ON protected_resources TYPE option<record<protected_resources>>;
DEFINE FIELD IF NOT EXISTS schema_version ON protected_resources TYPE int ASSERT $value >= 1;
DEFINE FIELD IF NOT EXISTS lifecycle_state ON protected_resources TYPE string ASSERT $value IN ['active', 'quarantined', 'deleted'];
DEFINE FIELD IF NOT EXISTS policy_version ON protected_resources TYPE int ASSERT $value >= 1;
DEFINE FIELD IF NOT EXISTS classification ON protected_resources TYPE string ASSERT string::len($value) > 0;
DEFINE FIELD IF NOT EXISTS storage_locator_hash ON protected_resources TYPE string ASSERT string::len($value) = 64 PERMISSIONS NONE;
DEFINE FIELD IF NOT EXISTS created_at ON protected_resources TYPE datetime;
DEFINE FIELD IF NOT EXISTS updated_at ON protected_resources TYPE datetime;
DEFINE INDEX IF NOT EXISTS protected_resources_external_unique ON protected_resources FIELDS resource_kind, external_resource_id UNIQUE;
DEFINE INDEX IF NOT EXISTS protected_resources_owner_idx ON protected_resources FIELDS owner_account_id, access_space_id;

DEFINE TABLE IF NOT EXISTS resource_grants TYPE NORMAL SCHEMAFULL
  PERMISSIONS FOR select WHERE
    status = 'active'
    AND account_id = $auth.account_id
    AND principal_id = $auth.principal_id
    AND access_space_id = $auth.access_space_id,
  FOR create, update, delete NONE;
DEFINE FIELD IF NOT EXISTS account_id ON resource_grants TYPE record<local_accounts>;
DEFINE FIELD IF NOT EXISTS principal_id ON resource_grants TYPE record<principals>;
DEFINE FIELD IF NOT EXISTS access_space_id ON resource_grants TYPE record<access_spaces>;
DEFINE FIELD IF NOT EXISTS resource_id ON resource_grants TYPE record<protected_resources>;
DEFINE FIELD IF NOT EXISTS actions ON resource_grants TYPE array<string>;
DEFINE FIELD IF NOT EXISTS capability_ids ON resource_grants TYPE array<string>;
DEFINE FIELD IF NOT EXISTS delegation_chain ON resource_grants TYPE array<string>;
DEFINE FIELD IF NOT EXISTS status ON resource_grants TYPE string ASSERT $value IN ['active', 'revoked'];
DEFINE FIELD IF NOT EXISTS grant_version ON resource_grants TYPE int ASSERT $value >= 1;
DEFINE FIELD IF NOT EXISTS policy_version ON resource_grants TYPE int ASSERT $value >= 1;
DEFINE FIELD IF NOT EXISTS expires_at ON resource_grants TYPE option<datetime>;
DEFINE FIELD IF NOT EXISTS revoked_at ON resource_grants TYPE option<datetime>;
DEFINE FIELD IF NOT EXISTS created_at ON resource_grants TYPE datetime;
DEFINE FIELD IF NOT EXISTS updated_at ON resource_grants TYPE datetime;
DEFINE INDEX IF NOT EXISTS resource_grants_exact_idx ON resource_grants FIELDS account_id, principal_id, access_space_id, resource_id;

DEFINE TABLE IF NOT EXISTS authorization_audit_events TYPE NORMAL SCHEMAFULL
  PERMISSIONS FOR select WHERE account_id = $auth.account_id AND principal_id = $auth.principal_id, FOR create, update, delete NONE;
DEFINE FIELD IF NOT EXISTS decision_id ON authorization_audit_events TYPE string ASSERT string::len($value) > 0;
DEFINE FIELD IF NOT EXISTS account_id ON authorization_audit_events TYPE option<record<local_accounts>>;
DEFINE FIELD IF NOT EXISTS principal_id ON authorization_audit_events TYPE option<record<principals>>;
DEFINE FIELD IF NOT EXISTS session_id ON authorization_audit_events TYPE option<record<authenticated_sessions>>;
DEFINE FIELD IF NOT EXISTS access_space_id ON authorization_audit_events TYPE option<record<access_spaces>>;
DEFINE FIELD IF NOT EXISTS delegation_chain ON authorization_audit_events TYPE array<string>;
DEFINE FIELD IF NOT EXISTS resource_id ON authorization_audit_events TYPE option<record<protected_resources>>;
DEFINE FIELD IF NOT EXISTS requested_resource_hash ON authorization_audit_events TYPE string ASSERT string::len($value) = 64;
DEFINE FIELD IF NOT EXISTS resource_kind ON authorization_audit_events TYPE string;
DEFINE FIELD IF NOT EXISTS action ON authorization_audit_events TYPE string;
DEFINE FIELD IF NOT EXISTS capability_id ON authorization_audit_events TYPE string;
DEFINE FIELD IF NOT EXISTS result ON authorization_audit_events TYPE string ASSERT $value IN ['allow', 'deny', 'error'];
DEFINE FIELD IF NOT EXISTS policy_version ON authorization_audit_events TYPE option<int>;
DEFINE FIELD IF NOT EXISTS occurred_at ON authorization_audit_events TYPE datetime;
DEFINE INDEX IF NOT EXISTS authorization_audit_decision_unique ON authorization_audit_events FIELDS decision_id UNIQUE;

DEFINE ACCESS IF NOT EXISTS authenticated_session ON DATABASE TYPE RECORD
  SIGNIN (
    SELECT * FROM authenticated_sessions
    WHERE token_hash = $token_hash
      AND revoked_at = NONE
      AND expires_at > time::now()
      AND (channel_binding_hash = NONE OR channel_binding_hash = $channel_binding_hash)
      AND account_id.status = 'enabled'
      AND principal_id.status = 'enabled'
      AND access_space_id.status = 'active'
      AND account_id.revocation_epoch = account_revocation_epoch
      AND principal_id.revocation_epoch = principal_revocation_epoch
      AND access_space_id.revocation_epoch = space_revocation_epoch
      AND account_id.policy_version <= policy_version
      AND principal_id.policy_version <= policy_version
      AND access_space_id.policy_version <= policy_version
    LIMIT 1
  )
  DURATION FOR TOKEN 5m, FOR SESSION 12h;
"#;

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
    FlightRecorder,
    MemoryPack,
    MemoryProposal,
    MemoryCommitReport,
    MemoryItemCount,
    ReconciliationQueue,
}

impl ResourceKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Workspace => "workspace",
            Self::FlightRecorder => "flight_recorder",
            Self::MemoryPack => "memory_pack",
            Self::MemoryProposal => "memory_proposal",
            Self::MemoryCommitReport => "memory_commit_report",
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
struct SessionIssueRow {
    account_epoch: i64,
    principal_epoch: i64,
    space_epoch: i64,
    policy_version: i64,
    delegated_capabilities: Vec<String>,
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
struct SigninParams {
    token_hash: String,
    channel_binding_hash: Option<String>,
}

impl SurrealStorage {
    pub async fn bootstrap_resource_authority_schema(&self) -> Result<(), ResourceAuthorityError> {
        let namespace = self.config().namespace().to_owned();
        self.with_lease(move |client| {
            Box::pin(async move {
                let authority = client.clone();
                authority
                    .use_ns(namespace)
                    .use_db(AUTHORITY_DATABASE)
                    .await?;
                authority.query(AUTHORITY_SCHEMA).await?.check()?;
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
        self.bootstrap_resource_authority_schema().await?;

        let account = RecordId::new("local_accounts", Uuid::now_v7().to_string());
        let principal = RecordId::new("principals", Uuid::now_v7().to_string());
        let space = RecordId::new("access_spaces", Uuid::now_v7().to_string());
        let namespace = self.config().namespace().to_owned();
        let account_key = account_key.to_owned();
        let principal_key = principal_key.to_owned();
        let actor_kind = actor_kind.to_owned();
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
                        .use_db(AUTHORITY_DATABASE)
                        .await?;
                    let mut response = authority
                        .query(
                            "BEGIN TRANSACTION;\n\
                             LET $existing_principal = (SELECT * FROM principals WHERE principal_key = $principal_key LIMIT 1);\n\
                             LET $existing_space = (SELECT * FROM access_spaces WHERE space_key = $space_key LIMIT 1);\n\
                             IF array::len($existing_principal) = 0 {\n\
                               CREATE $account SET account_key = $account_key, account_role = 'Member', status = 'enabled', revocation_epoch = 0, policy_version = 1, created_at = $now, updated_at = $now;\n\
                               CREATE $principal SET principal_key = $principal_key, account_id = $account, principal_kind = $principal_kind, actor_kind = $actor_kind, actor_id = $actor_id, capability_profile_id = $capability_profile_id, delegated_capabilities = $delegated_capabilities, status = 'enabled', revocation_epoch = 0, policy_version = 1, created_at = $now, updated_at = $now;\n\
                               CREATE $space SET space_key = $space_key, account_id = $account, name = $space_key, status = 'active', revocation_epoch = 0, policy_version = 1, created_at = $now, updated_at = $now;\n\
                             };\n\
                             COMMIT TRANSACTION;\n\
                             SELECT account_id, id AS principal_id, (SELECT VALUE id FROM access_spaces WHERE account_id = $parent.account_id AND space_key = $space_key LIMIT 1)[0] AS access_space_id FROM principals WHERE principal_key = $principal_key LIMIT 1;",
                        )
                        .bind(("account", account))
                        .bind(("principal", principal))
                        .bind(("space", space))
                        .bind(("account_key", account_key))
                        .bind(("principal_key", principal_key))
                        .bind(("principal_kind", actor_kind.clone()))
                        .bind(("actor_kind", actor_kind))
                        .bind(("actor_id", actor_id))
                        .bind(("capability_profile_id", capability_profile_id))
                        .bind(("delegated_capabilities", delegated_capabilities))
                        .bind(("space_key", space_key))
                        .bind(("now", now))
                        .await?
                        .check()?;
                    let rows: Vec<PrincipalLookupRow> = response.take(5)?;
                    rows.into_iter().next().ok_or_else(|| {
                        surrealdb::Error::internal(
                            "principal provisioning returned no canonical identity",
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
                channel_binding_hash,
                ttl,
            )
            .await?;
        Ok(ProvisionedPrincipal { identity, session })
    }

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
        let account = RecordId::new("local_accounts", account_id.to_owned());
        let principal = RecordId::new("principals", principal_id.to_owned());
        let space = RecordId::new("access_spaces", access_space_id.to_owned());
        let session = RecordId::new("authenticated_sessions", session_id.clone());
        let channel_binding_hash = channel_binding_hash.map(str::to_owned);

        self.with_lease(move |client| {
            Box::pin(async move {
                let authority = client.clone();
                authority
                    .use_ns(namespace)
                    .use_db(AUTHORITY_DATABASE)
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
                    surrealdb::Error::internal("session identity is not currently eligible")
                })?;
                authority
                    .query(
                        "CREATE $session SET account_id = $account, principal_id = $principal, access_space_id = $space, token_hash = $token_hash, channel_binding_hash = $channel_binding_hash, authentication_strength = 'local_opaque', delegated_capabilities = $delegated_capabilities, delegation_chain = [$principal_key], account_revocation_epoch = $account_epoch, principal_revocation_epoch = $principal_epoch, space_revocation_epoch = $space_epoch, policy_version = $policy_version, issued_at = $issued_at, expires_at = $expires_at, revoked_at = NONE;",
                    )
                    .bind(("session", session))
                    .bind(("account", account))
                    .bind(("principal", principal.clone()))
                    .bind(("space", space))
                    .bind(("token_hash", token_hash))
                    .bind(("channel_binding_hash", channel_binding_hash))
                    .bind(("delegated_capabilities", eligible.delegated_capabilities))
                    .bind(("principal_key", principal.to_string()))
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
                        .use_db(AUTHORITY_DATABASE)
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
                            "resource registry collision or ownership mismatch",
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
            vec![RecordId::new("principals", spec.principal_id.clone()).to_string()]
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
                    .use_db(AUTHORITY_DATABASE)
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

    pub async fn authorize_protected_resource(
        &self,
        request: AuthorizationRequest,
    ) -> Result<AuthorizationDecision, ResourceAuthorityError> {
        let decision_id = Uuid::now_v7().to_string();
        if request.session_token.is_empty()
            || request.capability_id.is_empty()
            || request.external_resource_id.is_empty()
        {
            self.audit_authorization(&decision_id, &request, None, None, "deny")
                .await?;
            return Err(ResourceAuthorityError::Denied { decision_id });
        }

        let namespace = self.config().namespace().to_owned();
        let token_hash = sha256_hex(request.session_token.as_bytes());
        let channel_binding_hash = request.channel_binding_hash.clone();
        let capability = request.capability_id.clone();
        let kind = request.resource_kind.as_str().to_owned();
        let external = request.external_resource_id.clone();
        let action = request.action.as_str().to_owned();

        let row = self
            .with_lease(move |client| {
                Box::pin(async move {
                    let authority = client.clone();
                    authority
                        .use_ns(namespace.clone())
                        .use_db(AUTHORITY_DATABASE)
                        .await?;
                    if authority
                        .signin(RecordSignin {
                            namespace,
                            database: AUTHORITY_DATABASE.to_owned(),
                            access: AUTHORITY_ACCESS_METHOD.to_owned(),
                            params: SigninParams {
                                token_hash,
                                channel_binding_hash,
                            },
                        })
                        .await
                        .is_err()
                    {
                        return Ok(AuthorizationAttempt {
                            session: None,
                            grant: None,
                        });
                    }
                    let mut session_response = authority
                        .query(
                            "SELECT account_id, principal_id, id AS session_id, access_space_id, delegation_chain, policy_version FROM authenticated_sessions WHERE id = $auth.id LIMIT 1;",
                        )
                        .await?
                        .check()?;
                    let session_rows: Vec<SessionAuthorityRow> = session_response.take(0)?;
                    let session = session_rows.into_iter().next();
                    let mut response = authority
                        .query(
                            "SELECT id AS grant_id, resource_id, account_id, principal_id, $auth.id AS session_id, access_space_id, principal_id.actor_kind AS actor_kind, principal_id.actor_id AS actor_id, principal_id.capability_profile_id AS capability_profile_id, $auth.delegation_chain AS delegation_chain, math::max([policy_version, resource_id.policy_version, account_id.policy_version, principal_id.policy_version, access_space_id.policy_version]) AS policy_version FROM resource_grants WHERE status = 'active' AND revoked_at = NONE AND (expires_at = NONE OR expires_at > time::now()) AND account_id = $auth.account_id AND principal_id = $auth.principal_id AND access_space_id = $auth.access_space_id AND resource_id.resource_kind = $kind AND resource_id.external_resource_id = $external AND resource_id.lifecycle_state = 'active' AND resource_id.owner_account_id = $auth.account_id AND resource_id.access_space_id = $auth.access_space_id AND array::contains(actions, $action) AND array::contains(capability_ids, $capability) AND (array::contains($auth.delegated_capabilities, '*') OR array::contains($auth.delegated_capabilities, $capability)) AND delegation_chain = $auth.delegation_chain LIMIT 1;",
                        )
                        .bind(("kind", kind))
                        .bind(("external", external))
                        .bind(("action", action))
                        .bind(("capability", capability))
                        .await?
                        .check()?;
                    let rows: Vec<AuthorizationRow> = response.take(0)?;
                    Ok(AuthorizationAttempt {
                        session,
                        grant: rows.into_iter().next(),
                    })
                })
            })
            .await?;

        let Some(row) = row.grant else {
            self.audit_authorization(&decision_id, &request, None, row.session.as_ref(), "deny")
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
        self.audit_authorization(&decision_id, &request, Some(&decision), None, "allow")
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
                authority
                    .use_ns(namespace)
                    .use_db(AUTHORITY_DATABASE)
                    .await?;
                authority
                    .query(
                        "CREATE $audit SET decision_id = $decision_id, account_id = $account, principal_id = $principal, session_id = $session, access_space_id = $space, delegation_chain = $delegation_chain, resource_id = $resource, requested_resource_hash = $requested_resource_hash, resource_kind = $resource_kind, action = $action, capability_id = $capability, result = $result, policy_version = $policy_version, occurred_at = time::now();",
                    )
                    .bind(("audit", audit))
                    .bind(("decision_id", decision_id))
                    .bind(("account", account))
                    .bind(("principal", principal))
                    .bind(("session", session))
                    .bind(("space", space))
                    .bind(("delegation_chain", delegation_chain))
                    .bind(("resource", resource))
                    .bind(("requested_resource_hash", requested_resource_hash))
                    .bind(("resource_kind", resource_kind))
                    .bind(("action", action))
                    .bind(("capability", capability))
                    .bind(("result", result))
                    .bind(("policy_version", policy_version))
                    .await?
                    .check()?;
                Ok(())
            })
        })
        .await?;
        Ok(())
    }

    pub async fn revoke_session(&self, session_id: &str) -> Result<(), ResourceAuthorityError> {
        self.update_authority_record(
            "authenticated_sessions",
            session_id,
            "UPDATE $record SET revoked_at = time::now();",
        )
        .await
    }

    pub async fn revoke_grant(&self, grant_id: &str) -> Result<(), ResourceAuthorityError> {
        self.update_authority_record(
            "resource_grants",
            grant_id,
            "UPDATE $record SET status = 'revoked', revoked_at = time::now(), grant_version += 1, policy_version += 1, updated_at = time::now();",
        )
        .await
    }

    pub async fn disable_account(&self, account_id: &str) -> Result<(), ResourceAuthorityError> {
        self.update_authority_record(
            "local_accounts",
            account_id,
            "UPDATE $record SET status = 'disabled', revocation_epoch += 1, policy_version += 1, updated_at = time::now();",
        )
        .await
    }

    pub async fn switch_session_access_space(
        &self,
        session_id: &str,
        new_access_space_id: &str,
    ) -> Result<(), ResourceAuthorityError> {
        validate_uuid(session_id)?;
        validate_uuid(new_access_space_id)?;
        let namespace = self.config().namespace().to_owned();
        let session = RecordId::new("authenticated_sessions", session_id.to_owned());
        let space = RecordId::new("access_spaces", new_access_space_id.to_owned());
        self.with_lease(move |client| {
            Box::pin(async move {
                let authority = client.clone();
                authority
                    .use_ns(namespace)
                    .use_db(AUTHORITY_DATABASE)
                    .await?;
                authority
                    .query(
                        "UPDATE $session SET access_space_id = $space, space_revocation_epoch = $space.revocation_epoch, policy_version = math::max([policy_version + 1, $space.policy_version]);",
                    )
                    .bind(("session", session))
                    .bind(("space", space))
                    .await?
                    .check()?;
                Ok(())
            })
        })
        .await?;
        Ok(())
    }

    pub async fn list_registered_workspace_resource_ids(
        &self,
    ) -> Result<Vec<String>, ResourceAuthorityError> {
        let namespace = self.config().namespace().to_owned();
        let rows = self
            .with_lease(move |client| {
                Box::pin(async move {
                    let authority = client.clone();
                    authority
                        .use_ns(namespace)
                        .use_db(AUTHORITY_DATABASE)
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
        self.grant_resource(
            &provisioned.identity.account_id,
            &provisioned.identity.access_space_id,
            ResourceGrantSpec {
                principal_id: provisioned.identity.principal_id.clone(),
                resource_id: queue.resource_id,
                actions: vec![ResourceAction::Reconcile],
                capability_ids: capabilities.clone(),
                expires_at: Some(provisioned.session.expires_at),
                delegation_chain: Vec::new(),
            },
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
            self.grant_resource(
                &provisioned.identity.account_id,
                &provisioned.identity.access_space_id,
                ResourceGrantSpec {
                    principal_id: provisioned.identity.principal_id.clone(),
                    resource_id: workspace_queue.resource_id,
                    actions: vec![ResourceAction::Reconcile],
                    capability_ids: capabilities.clone(),
                    expires_at: Some(provisioned.session.expires_at),
                    delegation_chain: Vec::new(),
                },
            )
            .await?;
        }
        Ok(provisioned)
    }

    async fn lookup_registered_resource(
        &self,
        kind: ResourceKind,
        external_resource_id: &str,
    ) -> Result<RegisteredResource, ResourceAuthorityError> {
        validate_nonempty(external_resource_id)?;
        let namespace = self.config().namespace().to_owned();
        let kind_name = kind.as_str().to_owned();
        let external = external_resource_id.to_owned();
        let row = self
            .with_lease(move |client| {
                Box::pin(async move {
                    let authority = client.clone();
                    authority
                        .use_ns(namespace)
                        .use_db(AUTHORITY_DATABASE)
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
        let record = RecordId::new(table, id.to_owned());
        self.with_lease(move |client| {
            Box::pin(async move {
                let authority = client.clone();
                authority
                    .use_ns(namespace)
                    .use_db(AUTHORITY_DATABASE)
                    .await?;
                authority
                    .query(statement)
                    .bind(("record", record))
                    .await?
                    .check()?;
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
