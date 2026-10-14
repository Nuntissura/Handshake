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
