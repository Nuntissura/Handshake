//! Explicit local Owner setup and password authentication. Channel possession is never login.

use super::resource_authority::ResourceAuthorityError;
use super::resource_authority::{IssuedSessionCredential, ProvisionedIdentity};
use super::SurrealStorage;
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Algorithm, Argon2, Params, Version,
};
use sha2::{Digest, Sha256};
use std::time::Duration;
use surrealdb::types::{RecordId, RecordIdKey, SurrealValue};
use uuid::Uuid;

const PASSWORD_MEMORY_KIB: u32 = 19 * 1024;
const PASSWORD_ITERATIONS: u32 = 2;
const MAX_PASSWORD_BYTES: usize = 1024;
static PASSWORD_WORKERS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(2);

pub(crate) async fn new_password_verifier(
    password: String,
) -> Result<String, ResourceAuthorityError> {
    run_password_worker(move || password_verifier(&password)).await?
}

async fn run_password_worker<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
) -> Result<T, ResourceAuthorityError> {
    let _permit = PASSWORD_WORKERS
        .acquire()
        .await
        .map_err(|_| ResourceAuthorityError::InvalidInput("authentication unavailable"))?;
    tokio::task::spawn_blocking(move || {
        let _permit = _permit;
        work()
    })
    .await
    .map_err(|_| ResourceAuthorityError::InvalidInput("authentication worker failed"))
}

fn password_hasher() -> Argon2<'static> {
    Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(PASSWORD_MEMORY_KIB, PASSWORD_ITERATIONS, 1, None)
            .expect("fixed Argon2 parameters"),
    )
}

/// PHC stores the algorithm, version, cost, unique salt and verifier; never the password.
pub fn password_verifier(password: &str) -> Result<String, ResourceAuthorityError> {
    if password.len() < 12 || password.len() > MAX_PASSWORD_BYTES {
        return Err(ResourceAuthorityError::InvalidInput(
            "password must contain 12 to 1024 UTF-8 bytes",
        ));
    }
    let mut salt = [0_u8; 16];
    getrandom::getrandom(&mut salt).map_err(|e| ResourceAuthorityError::Entropy(e.to_string()))?;
    let salt = SaltString::encode_b64(&salt)
        .map_err(|_| ResourceAuthorityError::InvalidInput("password salt encoding failed"))?;
    password_hasher()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| ResourceAuthorityError::InvalidInput("password hashing failed"))
}

pub fn verify_password(password: &str, verifier: &str) -> bool {
    if password.len() > MAX_PASSWORD_BYTES {
        return false;
    }
    let Ok(hash) = PasswordHash::new(verifier) else {
        return false;
    };
    // Only this versioned producer's parameters are accepted: corrupted verifier rows cannot
    // request unbounded memory, and a weakened hash cannot become an authentication shortcut.
    if hash.algorithm.as_str() != "argon2id"
        || hash.version != Some(19)
        || hash.params.get_decimal("m") != Some(PASSWORD_MEMORY_KIB)
        || hash.params.get_decimal("t") != Some(PASSWORD_ITERATIONS)
        || hash.params.get_decimal("p") != Some(1)
    {
        return false;
    }
    password_hasher()
        .verify_password(password.as_bytes(), &hash)
        .is_ok()
}

#[derive(SurrealValue)]
struct LoginRow {
    account_id: RecordId,
    principal_id: RecordId,
    access_space_id: RecordId,
    password_verifier: String,
}

#[derive(SurrealValue)]
struct SessionRow {
    account_id: RecordId,
    principal_id: RecordId,
    access_space_id: RecordId,
    session_id: RecordId,
    actor_id: String,
    principal_kind: String,
    actor_kind: String,
    policy_version: i64,
    delegation_chain: Vec<String>,
    delegated_capabilities: Vec<String>,
}

#[derive(SurrealValue)]
struct CanvasPlacementIdentityRow {
    canvas_block_id: RecordId,
    placed_block_id: RecordId,
}

#[derive(SurrealValue)]
struct CanvasPlacementIdentityBindings {
    placement: RecordId,
    workspace: RecordId,
    account: RecordId,
    principal: RecordId,
    space: RecordId,
    workspace_resource: RecordId,
    workspace_grant: RecordId,
}

#[derive(Clone)]
pub struct LocalSessionContext {
    pub identity: ProvisionedIdentity,
    pub session_id: String,
    pub actor_id: String,
    pub principal_kind: String,
    pub actor_kind: String,
    pub policy_version: i64,
    pub delegation_chain: Vec<String>,
    /// MT-155: the capabilities delegated to THIS session (copied from the principal when the
    /// session was issued). Route-level capability checks read it; deny by default when absent.
    pub delegated_capabilities: Vec<String>,
}

pub(crate) fn account_event(
    event_type: crate::kernel::KernelEventType,
    identity: &ProvisionedIdentity,
    session_id: Option<&str>,
    policy_version: i64,
    delegation_chain: Vec<String>,
) -> Result<crate::kernel::NewKernelEvent, ResourceAuthorityError> {
    let operation = Uuid::now_v7().to_string();
    let action = event_type.as_str();
    crate::kernel::NewKernelEvent::builder(format!("local-account:{operation}"),
        session_id.unwrap_or(&operation), event_type.clone(),
        crate::kernel::KernelActor::Operator(identity.principal_id.clone()))
        .aggregate("local_account", &identity.account_id).source_component("local_account_authority")
        .payload(serde_json::json!({"schema_version":"hsk.local_account_event@1",
            "account_id":identity.account_id,"principal_id":identity.principal_id,
            "access_space_id":identity.access_space_id,"session_id":session_id,
            "delegation_chain":delegation_chain,"resource_refs":[{"kind":"local_account","id":identity.account_id}],
            "action":action,"result":"allow","policy_version":policy_version}))
        .build().map_err(|_| ResourceAuthorityError::InvalidInput("account audit event invalid"))
}

fn record_key(id: RecordId) -> Result<String, ResourceAuthorityError> {
    match id.key {
        RecordIdKey::String(key) => Ok(key),
        _ => Err(ResourceAuthorityError::InvalidInput(
            "invalid account identity",
        )),
    }
}

impl SurrealStorage {
    pub(crate) async fn document_for_embed(
        &self,
        embed_id: &str,
    ) -> Result<Option<String>, ResourceAuthorityError> {
        let embed_id = embed_id.to_owned();
        let namespace = self.config().namespace().to_owned();
        let database = self.config().database().to_owned();
        self.with_lease(move |client| Box::pin(async move {
            let broker = client.clone();
            broker.use_ns(namespace).use_db(database).await?;
            let mut response = broker.query("SELECT VALUE record::id(rich_document_id) FROM knowledge_document_embeds WHERE embed_id = $embed_id LIMIT 1;")
                .bind(("embed_id", embed_id)).await?.check()?;
            let mut rows: Vec<String> = response.take(0)?;
            Ok(rows.pop())
        })).await.map_err(Into::into)
    }
    /// A revoked bearer only names its own vault key for deletion, never content authority.
    pub(crate) async fn retired_session_cleanup_id(
        &self,
        token: &str,
        channel_hash: &str,
    ) -> Result<Option<String>, ResourceAuthorityError> {
        let token_hash = hex::encode(Sha256::digest(token.as_bytes()));
        let channel_hash = channel_hash.to_owned();
        let namespace = self.config().namespace().to_owned();
        let database = self.config().database().to_owned();
        self.with_lease(move |client| Box::pin(async move {
            let broker = client.clone();
            broker.use_ns(namespace).use_db(database).await?;
            let mut response = broker.query("SELECT VALUE record::id(id) FROM authenticated_sessions WHERE token_hash = $token_hash AND channel_binding_hash = $channel_hash AND revoked_at != NONE LIMIT 1;")
                .bind(("token_hash", token_hash)).bind(("channel_hash", channel_hash)).await?.check()?;
            let mut rows: Vec<String> = response.take(0)?;
            Ok(rows.pop())
        })).await.map_err(Into::into)
    }
    pub(crate) async fn logout_local_account(
        &self,
        context: &LocalSessionContext,
    ) -> Result<(), ResourceAuthorityError> {
        let context = context.clone();
        self.with_owned_authority_mutation(move |storage| async move {
            storage.logout_local_account_unfenced(&context).await
        }).await?
    }

    async fn logout_local_account_unfenced(
        &self,
        context: &LocalSessionContext,
    ) -> Result<(), ResourceAuthorityError> {
        let event = account_event(
            crate::kernel::KernelEventType::LocalAccountLogout,
            &context.identity,
            Some(&context.session_id),
            context.policy_version,
            context.delegation_chain.clone(),
        )?;
        let (_, write) = super::event_ledger::prepare_event(event)
            .map_err(|_| ResourceAuthorityError::InvalidInput("account audit event invalid"))?;
        let event = super::event_ledger::LedgerBulkInsert::from(write);
        let account_session = RecordId::new("authenticated_sessions", context.session_id.clone());
        let namespace = self.config().namespace().to_owned();
        let database = self.config().database().to_owned();
        self.with_lease(move |client| Box::pin(async move {
            let broker = client.clone();
            broker.use_ns(namespace).use_db(database).await?;
            super::authority_mutation_submitted();
            broker.query("BEGIN TRANSACTION; IF $account_session.revoked_at != NONE OR $account_session.expires_at <= time::now() { THROW 'HSK-403-PROTECTED-RESOURCE'; }; UPDATE $account_session SET revoked_at = time::now(); INSERT INTO kernel_event_ledger $event; COMMIT TRANSACTION;")
                .bind(("account_session", account_session)).bind(("event", event)).await?.check().map_err(super::authority_mutation_error)?;
            Ok(())
        })).await?;
        super::authority_mutation_submitted();
        self.resource_access_lifecycle().invalidate_session(&context.session_id)
            .map_err(|_| ResourceAuthorityError::InvalidInput("lifecycle authority unavailable"))?;
        super::authority_mutation_settled();
        Ok(())
    }
    pub(crate) async fn authorized_document_workspace(
        &self,
        resource_id: &str,
        account_id: &str,
        space_id: &str,
    ) -> Result<Option<String>, ResourceAuthorityError> {
        let resource = RecordId::new("protected_resources", resource_id.to_owned());
        let account = RecordId::new("local_accounts", account_id.to_owned());
        let space = RecordId::new("access_spaces", space_id.to_owned());
        let namespace = self.config().namespace().to_owned();
        let database = self.config().database().to_owned();
        self.with_lease(move |client| Box::pin(async move {
            let broker = client.clone();
            broker.use_ns(namespace).use_db(database).await?;
            let mut response = broker.query("SELECT VALUE parent_resource_id.external_resource_id FROM $resource WHERE resource_kind = 'rich_document' AND lifecycle_state = 'active' AND owner_account_id = $account AND access_space_id = $space AND parent_resource_id.resource_kind = 'workspace' AND parent_resource_id.owner_account_id = $account AND parent_resource_id.access_space_id = $space AND parent_resource_id.lifecycle_state = 'active' LIMIT 1;")
                .bind(("resource", resource)).bind(("account", account)).bind(("space", space)).await?.check()?;
            let mut rows: Vec<String> = response.take(0)?;
            Ok(rows.pop())
        })).await.map_err(Into::into)
    }

    /// Resolves only the immutable board/source ids needed to authorize a
    /// placement removal. The caller has already proven the workspace read
    /// grant; this admin-context lookup rechecks both Loom-resource lineages
    /// against that authenticated account, principal, space, and workspace.
    pub(crate) async fn authorized_canvas_placement_identity(
        &self,
        scope: &super::resource_authority::RecordUserScope,
        workspace_id: &str,
        placement_id: &str,
    ) -> Result<Option<(String, String)>, ResourceAuthorityError> {
        let Some(channel) = scope.channel_binding_hash.as_deref() else {
            return Ok(None);
        };
        let Some(grant_id) = scope.grant_id.as_deref() else {
            return Ok(None);
        };
        let session = self
            .authenticate_local_session(&scope.session_token, channel)
            .await?;
        if session.session_id != scope.session_id {
            return Ok(None);
        }
        let bindings = CanvasPlacementIdentityBindings {
            placement: RecordId::new("loom_canvas_placements", placement_id.to_owned()),
            workspace: RecordId::new("workspaces", workspace_id.to_owned()),
            account: RecordId::new("local_accounts", session.identity.account_id),
            principal: RecordId::new("principals", session.identity.principal_id),
            space: RecordId::new("access_spaces", session.identity.access_space_id),
            workspace_resource: RecordId::new("protected_resources", scope.resource_id.clone()),
            workspace_grant: RecordId::new("resource_grants", grant_id.to_owned()),
        };
        let namespace = self.config().namespace().to_owned();
        let database = self.config().database().to_owned();
        self.with_lease(move |client| Box::pin(async move {
            let broker = client.clone();
            broker.use_ns(namespace).use_db(database).await?;
            // MT-109 C3 (C2 follow-up b): a text card places its RichDocument's same-id Loom
            // projection, whose protected resource is the `rich_document`.
            let mut response = broker.query(
                "SELECT canvas_block_id, placed_block_id FROM $placement \
                 WHERE workspace_id = $workspace \
                   AND canvas_block_id.workspace_id = $workspace \
                   AND placed_block_id.workspace_id = $workspace \
                   AND array::len((SELECT id FROM $workspace_resource WHERE resource_kind = 'workspace' \
                     AND external_resource_id = record::id($workspace) AND lifecycle_state = 'active' \
                     AND owner_account_id = $account AND access_space_id = $space)) = 1 \
                   AND array::len((SELECT id FROM $workspace_grant WHERE resource_id = $workspace_resource \
                     AND status = 'active' AND revoked_at = NONE AND (expires_at = NONE OR expires_at > time::now()) \
                     AND account_id = $account AND principal_id = $principal AND access_space_id = $space \
                     AND actions CONTAINS 'read' AND capability_ids CONTAINS 'fs.read')) = 1 \
                   AND array::len((SELECT id FROM protected_resources WHERE resource_kind = 'loom_block' \
                     AND external_resource_id = record::id($placement.canvas_block_id) AND lifecycle_state = 'active' \
                     AND owner_account_id = $account AND created_by_principal_id.status = 'enabled' AND access_space_id = $space \
                     AND parent_resource_id.resource_kind = 'workspace' AND parent_resource_id.external_resource_id = record::id($workspace) \
                     AND parent_resource_id.lifecycle_state = 'active' AND parent_resource_id.owner_account_id = $account \
                     AND parent_resource_id.access_space_id = $space)) = 1 \
                   AND array::len((SELECT id FROM protected_resources WHERE resource_kind IN ['loom_block', 'rich_document'] \
                     AND external_resource_id = record::id($placement.placed_block_id) AND lifecycle_state = 'active' \
                     AND owner_account_id = $account AND created_by_principal_id.status = 'enabled' AND access_space_id = $space \
                     AND parent_resource_id.resource_kind = 'workspace' AND parent_resource_id.external_resource_id = record::id($workspace) \
                     AND parent_resource_id.lifecycle_state = 'active' AND parent_resource_id.owner_account_id = $account \
                     AND parent_resource_id.access_space_id = $space)) = 1 LIMIT 1;"
            ).bind(SurrealValue::into_value(bindings)).await?.check()?;
            let rows: Vec<CanvasPlacementIdentityRow> = response.take(0)?;
            Ok(rows.into_iter().next())
        })).await.map_err(Into::into).map(|row| row.and_then(|row| {
            let canvas = match row.canvas_block_id.key { RecordIdKey::String(value) => value, _ => return None };
            let placed = match row.placed_block_id.key { RecordIdKey::String(value) => value, _ => return None };
            Some((canvas, placed))
        }))
    }
    pub async fn authenticate_local_session(
        &self,
        token: &str,
        channel_hash: &str,
    ) -> Result<LocalSessionContext, ResourceAuthorityError> {
        use super::resource_authority::{SigninParams, AUTHORITY_ACCESS_METHOD};
        use surrealdb::opt::auth::Record;
        let namespace = self.config().namespace().to_owned();
        let database = self.config().database().to_owned();
        let token_hash = hex::encode(Sha256::digest(token.as_bytes()));
        let channel_binding_hash = Some(channel_hash.to_owned());
        let row = self.with_lease(move |client| Box::pin(async move {
            let ordinary = client.clone();
            ordinary.use_ns(namespace.clone()).use_db(database.clone()).await?;
            ordinary.signin(Record { namespace, database, access: AUTHORITY_ACCESS_METHOD.to_owned(),
                params: SigninParams { token_hash, channel_binding_hash } }).await?;
            let mut response = ordinary.query("SELECT account_id, principal_id, access_space_id, id AS session_id, principal_id.actor_id AS actor_id, principal_id.principal_kind AS principal_kind, principal_id.actor_kind AS actor_kind, policy_version, delegation_chain, delegated_capabilities FROM authenticated_sessions WHERE id = $auth.id LIMIT 1;").await?.check()?;
            let mut rows: Vec<SessionRow> = response.take(0)?;
            Ok(rows.pop())
        })).await?;
        let row = row.ok_or(ResourceAuthorityError::InvalidInput(
            "authentication denied",
        ))?;
        Ok(LocalSessionContext {
            identity: ProvisionedIdentity {
                account_id: record_key(row.account_id)?,
                principal_id: record_key(row.principal_id)?,
                access_space_id: record_key(row.access_space_id)?,
            },
            session_id: record_key(row.session_id)?,
            actor_id: row.actor_id,
            principal_kind: row.principal_kind,
            actor_kind: row.actor_kind,
            policy_version: row.policy_version,
            delegation_chain: row.delegation_chain,
            delegated_capabilities: row.delegated_capabilities,
        })
    }
    /// First-run status is installation state only. It never returns account names or ids.
    pub async fn local_owner_setup_required(&self) -> Result<bool, ResourceAuthorityError> {
        let namespace = self.config().namespace().to_owned();
        let database = self.config().database().to_owned();
        let needed = self.with_lease(move |client| Box::pin(async move {
            let admin = client.clone();
            admin.use_ns(namespace).use_db(database).await?;
            let mut response = admin.query("RETURN !record::exists(local_account_setup:primary) AND array::len(SELECT VALUE id FROM local_accounts WHERE account_role = 'Owner' LIMIT 1) = 0;").await?.check()?;
            Ok(response.take::<Option<bool>>(0)?.unwrap_or(false))
        })).await?;
        Ok(needed)
    }

    /// A fixed record id, created in the same transaction as all identity rows, serializes
    /// competing first-run requests even across processes. No pre-existing resource is claimed.
    pub async fn setup_local_owner(
        &self, account_name: &str, verifier: String, capabilities: Vec<String>,
    ) -> Result<ProvisionedIdentity, ResourceAuthorityError> {
        let name = account_name.trim();
        if name.is_empty() || name.len() > 128 || capabilities.is_empty()
            || capabilities.iter().any(|value| value.contains('*'))
        { return Err(ResourceAuthorityError::InvalidInput("invalid local account setup")); }
        let operation_id = Uuid::now_v7();
        let identity = ProvisionedIdentity {
            account_id: Uuid::now_v7().to_string(), principal_id: Uuid::now_v7().to_string(),
            access_space_id: Uuid::now_v7().to_string(),
        };
        let event = account_event(crate::kernel::KernelEventType::LocalAccountSetup,
            &identity, None, 1, vec![identity.principal_id.clone()])?;
        let (_, event) = super::event_ledger::prepare_event(event)
            .map_err(|_| ResourceAuthorityError::InvalidInput("account audit event invalid"))?;
        let receipt = FirstOwnerSetupReceipt {
            setup_operation_id: operation_id.to_string(),
            account: RecordId::new("local_accounts", identity.account_id.clone()),
            operator_principal: RecordId::new("principals", identity.principal_id.clone()),
            space: RecordId::new("access_spaces", identity.access_space_id.clone()),
            purpose: "prospective_first_owner_administration".to_owned(), version: 1,
            setup_receipt: event.record.clone(), setup_policy_version: 1, created_at: event.created_at.clone(),
        };
        let deadline = std::time::Instant::now().checked_add(self.config().statement_timeout())
            .ok_or(ResourceAuthorityError::InvalidInput("setup deadline outside range"))?;
        let original = std::sync::Arc::new(FirstOwnerSetupOriginal {
            storage: self.clone(), operation_id, identity, receipt: receipt.clone(), deadline,
            expected: FirstOwnerSetupExpected { name: name.to_owned(), verifier: verifier.clone(),
                capabilities: capabilities.clone(), event: event.clone() },
            write: std::sync::Mutex::new(Some(FirstOwnerSetupWrite {
                origin: receipt, name: name.to_owned(), verifier, capabilities,
                event: super::event_ledger::LedgerBulkInsert::from(event),
            })),
        });
        let owner = std::sync::Arc::new(FirstOwnerSetupOperation {
            original: original.clone(), state: tokio::sync::Mutex::new(FirstOwnerSetupWorkerState::Starting),
        });
        {
            let admission = self.inner.member_login_admission.lock()
                .map_err(|_| ResourceAuthorityError::InvalidInput("setup admission poisoned"))?;
            if !*admission { return Err(super::SurrealStorageError::Closed.into()); }
            self.reserve_transaction_settlement(operation_id, Some(
                super::AuthenticatedTransactionRetention::FirstOwnerSetup(owner.clone())))?;
            let mut state = owner.state.try_lock()
                .map_err(|_| ResourceAuthorityError::InvalidInput("unpublished setup worker is locked"))?;
            *state = FirstOwnerSetupWorkerState::Running(tokio::spawn(async move {
                original.storage.clone().run_prospective_first_owner_setup(original).await
            }));
        }
        match owner.wait().await {
            FirstOwnerSetupObservation::Complete(result) => match result.as_ref() {
                Ok(identity) => Ok(identity.clone()),
                Err(_) => Err(super::SurrealStorageError::TransactionSettlementPending { operation_id }.into()),
            },
            FirstOwnerSetupObservation::Join(_) =>
                Err(super::SurrealStorageError::TransactionSettlementPending { operation_id }.into()),
        }
    }

    /// Password verification is independent of native channel possession and
    /// does not mint a credential. Controlled setup recovery uses this exact
    /// check before completing a previously interrupted setup operation.
    pub async fn verify_local_owner_password(
        &self,
        account_name: &str,
        password: String,
    ) -> Result<ProvisionedIdentity, ResourceAuthorityError> {
        if password.len() < 12 || password.len() > MAX_PASSWORD_BYTES {
            return Err(ResourceAuthorityError::InvalidInput(
                "authentication denied",
            ));
        }
        let name = account_name.trim().to_owned();
        let namespace = self.config().namespace().to_owned();
        let database = self.config().database().to_owned();
        let row = self.with_lease(move |client| Box::pin(async move {
            let admin = client.clone();
            admin.use_ns(namespace).use_db(database).await?;
            let mut response = admin.query("SELECT account_id, principal_id, access_space_id, account_id.password_verifier AS password_verifier FROM local_account_setup:primary WHERE account_id.account_key = $name AND account_id.status = 'enabled' AND principal_id.status = 'enabled' AND access_space_id.status = 'active' AND account_id.password_verifier != NONE;")
                .bind(("name", name)).await?.check()?;
            let mut rows: Vec<LoginRow> = response.take(0)?;
            Ok(if rows.len() == 1 { rows.pop() } else { None })
        })).await?;
        // Verify a real-cost dummy on absent identities to avoid a cheap account-existence oracle.
        let verifier = row.as_ref().map(|row| row.password_verifier.clone());
        let valid = run_password_worker(move || match verifier {
            Some(verifier) => verify_password(&password, &verifier),
            None => {
                let _ = password_verifier("unavailable-local-account");
                false
            }
        })
        .await?;
        if !valid {
            return Err(ResourceAuthorityError::Denied {
                decision_id: Uuid::now_v7().to_string(),
            });
        }
        let row = row.ok_or(ResourceAuthorityError::InvalidInput(
            "authentication denied",
        ))?;
        Ok(ProvisionedIdentity {
            account_id: record_key(row.account_id)?,
            principal_id: record_key(row.principal_id)?,
            access_space_id: record_key(row.access_space_id)?,
        })
    }

    /// Password verification is independent of the native process/channel identity.
    pub async fn login_local_owner(
        &self,
        account_name: &str,
        password: String,
    ) -> Result<IssuedSessionCredential, ResourceAuthorityError> {
        let identity = self
            .verify_local_owner_password(account_name, password)
            .await?;
        self.provision_session_credential(&identity, Duration::from_secs(60))
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::KernelEventType;

    #[test]
    fn local_account_event_types_round_trip_and_unknown_types_remain_rejected() {
        for event_type in [
            KernelEventType::LocalAccountSetup,
            KernelEventType::LocalAccountLogin,
            KernelEventType::LocalAccountLoginDenied,
            KernelEventType::LocalAccountLogout,
        ] {
            assert_eq!(
                KernelEventType::try_from(event_type.as_str()).expect("known local account event"),
                event_type
            );
        }
        assert!(KernelEventType::try_from("LOCAL_ACCOUNT_UNKNOWN").is_err());
    }

    #[tokio::test]
    async fn cancelled_password_request_retains_worker_permit_until_blocking_exit() {
        let available = PASSWORD_WORKERS.available_permits();
        assert_eq!(
            available, 2,
            "focused worker proof requires an idle password pool"
        );
        let (entered, started) = tokio::sync::oneshot::channel();
        let (release, blocked) = std::sync::mpsc::channel();
        let (finished, exited) = tokio::sync::oneshot::channel();
        let request = tokio::spawn(run_password_worker(move || {
            entered.send(()).unwrap();
            blocked.recv().unwrap();
            let _ = finished.send(());
        }));
        started.await.unwrap();
        request.abort();
        assert!(request.await.unwrap_err().is_cancelled());
        assert_eq!(PASSWORD_WORKERS.available_permits(), available - 1);
        release.send(()).unwrap();
        exited.await.unwrap();
        tokio::time::timeout(Duration::from_secs(2), async {
            while PASSWORD_WORKERS.available_permits() != available {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("blocking exit releases its worker permit");
    }
    #[test]
    fn local_password_verifier_is_salted_versioned_and_fail_closed() {
        let first = password_verifier("correct horse battery staple").unwrap();
        let second = password_verifier("correct horse battery staple").unwrap();
        assert_ne!(first, second);
        assert!(first.starts_with("$argon2id$v=19$m=19456,t=2,p=1$"));
        assert!(verify_password("correct horse battery staple", &first));
        assert!(!verify_password("wrong password", &first));
        assert!(!verify_password(
            "correct horse battery staple",
            &first.replace("m=19456", "m=8")
        ));
        assert!(!verify_password(
            "correct horse battery staple",
            "not-a-verifier"
        ));
    }

    #[tokio::test]
    async fn local_owner_setup_login_replay_and_revocation_embedded() {
        let temp = tempfile::tempdir().expect("isolated runtime directory");
        let storage = SurrealStorage::open(
            super::super::SurrealStorageConfig::for_data_dir(temp.path()).unwrap(),
        )
        .await
        .expect("embedded storage");
        super::super::schema::bootstrap_schema(&storage)
            .await
            .expect("canonical schema");
        assert!(storage.local_owner_setup_required().await.unwrap());
        let verifier = password_verifier("correct horse battery staple").unwrap();
        let capabilities = vec!["fs.read".to_owned(), "fs.write".to_owned()];
        let (first, second) = tokio::join!(
            storage.setup_local_owner("Owner", verifier.clone(), capabilities.clone()),
            storage.setup_local_owner("Owner", verifier, capabilities)
        );
        assert_ne!(first.is_ok(), second.is_ok(), "one atomic first-run winner");
        let owner = first.or(second).unwrap();
        assert!(!storage.local_owner_setup_required().await.unwrap());
        assert!(storage
            .login_local_owner("Owner", "wrong password".into())
            .await
            .is_err());
        assert!(storage
            .login_local_owner("Missing", "correct horse battery staple".into())
            .await
            .is_err());
        let credential = storage
            .login_local_owner("Owner", "correct horse battery staple".into())
            .await
            .unwrap();
        let channel = "a".repeat(64);
        assert!(storage
            .exchange_session_credential(
                &Uuid::now_v7().to_string(),
                &credential.principal_id,
                &credential.access_space_id,
                &credential.token,
                &channel,
                Duration::from_secs(60)
            )
            .await
            .is_err());
        let session = storage
            .exchange_session_credential(
                &credential.account_id,
                &credential.principal_id,
                &credential.access_space_id,
                &credential.token,
                &channel,
                Duration::from_secs(60),
            )
            .await
            .unwrap();
        assert!(storage
            .exchange_session_credential(
                &credential.account_id,
                &credential.principal_id,
                &credential.access_space_id,
                &credential.token,
                &channel,
                Duration::from_secs(60)
            )
            .await
            .is_err());
        assert!(storage
            .authenticate_local_session(&session.token, &"b".repeat(64))
            .await
            .is_err());
        let context = storage
            .authenticate_local_session(&session.token, &channel)
            .await
            .unwrap();
        assert_eq!(context.identity.account_id, owner.account_id);
        assert_eq!(context.actor_id, owner.principal_id);
        storage.logout_local_account(&context).await.unwrap();
        assert!(storage
            .authenticate_local_session(&session.token, &channel)
            .await
            .is_err());
        assert!(storage.logout_local_account(&context).await.is_err());
        let counts = storage.with_lease(|client| Box::pin(async move {
            let mut response = client.query("RETURN {owners: array::len(SELECT VALUE id FROM local_accounts WHERE account_role = 'Owner'), setups: array::len(SELECT VALUE id FROM kernel_event_ledger WHERE event_type = 'LOCAL_ACCOUNT_SETUP'), logouts: array::len(SELECT VALUE id FROM kernel_event_ledger WHERE event_type = 'LOCAL_ACCOUNT_LOGOUT')};").await?.check()?;
            let value: Option<serde_json::Value> = response.take(0)?;
            Ok(value.unwrap())
        })).await.unwrap();
        assert_eq!(
            counts,
            serde_json::json!({"owners":1,"setups":1,"logouts":1})
        );
        storage.shutdown().await.unwrap();
    }
}

// RAM ONLY local_accounts.rs additions. Credential authentication boundary ONLY.
// Existing Owner verifier/setup/login and session exchange remain unchanged.
// No Clone/Debug/Serialize/public fields on the returned operation-bound observation.
#[derive(Clone, PartialEq, Eq, SurrealValue)]
struct CurrentActorPasswordRow {
    account_id: RecordId,
    principal_id: RecordId,
    access_space_id: RecordId,
    session_id: RecordId,
    password_verifier: String,
    credential_revision: i64,
    account_epoch: i64,
    principal_epoch: i64,
    space_epoch: i64,
    session_account_epoch: i64,
    session_principal_epoch: i64,
    session_space_epoch: i64,
    account_policy: i64,
    principal_policy: i64,
    space_policy: i64,
    session_policy: i64,
    actor_id: String,
    actor_kind: String,
    principal_kind: String,
    principal_profile: String,
    principal_capabilities: Vec<String>,
    session_capabilities: Vec<String>,
    delegation_chain: Vec<String>,
    session_expires_at: surrealdb::types::Datetime,
    channel_binding_hash: Option<String>,
}
#[derive(SurrealValue)]
struct CurrentActorPasswordBindings {
    account: RecordId,
    principal: RecordId,
    space: RecordId,
    session: RecordId,
    token_hash: String,
    channel_hash: String,
}
pub(super) struct CurrentActorPasswordReauthentication {
    storage: SurrealStorage,
    operation_id: Uuid,
    enrollment_operation_id: Uuid,
    token_hash: String,
    requested_channel_hash: String,
    observed: CurrentActorPasswordRow,
}
fn current_actor_password_denied() -> ResourceAuthorityError {
    ResourceAuthorityError::Denied { decision_id: Uuid::now_v7().to_string() }
}
impl CurrentActorPasswordRow {
    fn require_authenticated_actor(&self, session: &LocalSessionContext)
        -> Result<(), ResourceAuthorityError>
    {
        if self.account_id != RecordId::new("local_accounts", session.identity.account_id.clone())
            || self.principal_id != RecordId::new("principals", session.identity.principal_id.clone())
            || self.access_space_id != RecordId::new("access_spaces", session.identity.access_space_id.clone())
            || self.session_id != RecordId::new("authenticated_sessions", session.session_id.clone())
            || self.actor_id != session.actor_id || self.actor_kind != session.actor_kind
            || self.principal_kind != session.principal_kind || self.session_policy != session.policy_version
            || self.delegation_chain != session.delegation_chain || self.session_capabilities != session.delegated_capabilities
        { return Err(current_actor_password_denied()); }
        Ok(())
    }
}
impl SurrealStorage {
    // Server credential lookup follows the existing Owner password-verification
    // boundary, but derives its sole target from the ACTUAL authenticated actor.
    // It performs no ordinary resource effect and never exports the verifier.
    async fn read_current_actor_password(
        &self, session: &LocalSessionContext, token_hash: String, channel_hash: String,
    ) -> Result<Option<CurrentActorPasswordRow>, ResourceAuthorityError> {
        let bindings = CurrentActorPasswordBindings {
            account: RecordId::new("local_accounts", session.identity.account_id.clone()),
            principal: RecordId::new("principals", session.identity.principal_id.clone()),
            space: RecordId::new("access_spaces", session.identity.access_space_id.clone()),
            session: RecordId::new("authenticated_sessions", session.session_id.clone()),
            token_hash, channel_hash,
        };
        let namespace = self.config().namespace().to_owned();
        let database = self.config().database().to_owned();
        self.with_lease(move |client| Box::pin(async move {
            let credential_authority = client.clone();
            credential_authority.use_ns(namespace).use_db(database).await?;
            let mut response = credential_authority.query(
                "SELECT account_id, principal_id, access_space_id, id AS session_id, account_id.password_verifier AS password_verifier, account_id.credential_revision AS credential_revision,
                 account_id.revocation_epoch AS account_epoch, principal_id.revocation_epoch AS principal_epoch, access_space_id.revocation_epoch AS space_epoch,
                 account_revocation_epoch AS session_account_epoch, principal_revocation_epoch AS session_principal_epoch, space_revocation_epoch AS session_space_epoch,
                 account_id.policy_version AS account_policy, principal_id.policy_version AS principal_policy, access_space_id.policy_version AS space_policy, policy_version AS session_policy,
                 principal_id.actor_id AS actor_id, principal_id.actor_kind AS actor_kind, principal_id.principal_kind AS principal_kind,
                 principal_id.capability_profile_id AS principal_profile, principal_id.delegated_capabilities AS principal_capabilities,
                 delegated_capabilities AS session_capabilities, delegation_chain, expires_at AS session_expires_at, channel_binding_hash
                 FROM authenticated_sessions WHERE id = $session AND account_id = $account AND principal_id = $principal AND access_space_id = $space
                 AND principal_id.account_id = $account AND access_space_id.account_id = $account
                 AND token_hash = $token_hash AND (channel_binding_hash = NONE OR channel_binding_hash = $channel_hash)
                 AND account_id.password_verifier != NONE AND account_id.credential_revision > 0 AND fn::mt109_session_lineage_live(id) LIMIT 2;"
            ).bind(bindings.into_value()).await?.check()?;
            let mut rows: Vec<CurrentActorPasswordRow> = response.take(0)?;
            Ok(if rows.len() == 1 { rows.pop() } else { None })
        })).await.map_err(Into::into)
    }

    pub(super) async fn reauthenticate_current_actor_password(
        &self, operation_id: Uuid, enrollment_operation_id: Uuid,
        session_token: &str, channel_hash: &str, password: String,
    ) -> Result<CurrentActorPasswordReauthentication, ResourceAuthorityError> {
        if operation_id.is_nil() || enrollment_operation_id.is_nil()
            || password.len() < 12 || password.len() > MAX_PASSWORD_BYTES
        { return Err(current_actor_password_denied()); }
        let before_session = self.authenticate_local_session(session_token, channel_hash)
            .await.map_err(|_| current_actor_password_denied())?;
        let token_hash = hex::encode(Sha256::digest(session_token.as_bytes()));
        let before = self.read_current_actor_password(
            &before_session, token_hash.clone(), channel_hash.to_owned(),
        ).await?;
        if let Some(row) = &before { row.require_authenticated_actor(&before_session)?; }
        let verifier = before.as_ref().map(|row| row.password_verifier.clone());
        let valid = run_password_worker(move || match verifier {
            Some(verifier) => verify_password(&password, &verifier),
            None => {
                let _ = password_verifier("unavailable-local-account");
                false
            }
        }).await?;
        if !valid { return Err(current_actor_password_denied()); }
        // Reauthenticate the SAME supplied token/channel and reread the SAME
        // canonical identity/credential after the blocking worker has completed.
        let after_session = self.authenticate_local_session(session_token, channel_hash)
            .await.map_err(|_| current_actor_password_denied())?;
        let after = self.read_current_actor_password(
            &after_session, token_hash.clone(), channel_hash.to_owned(),
        ).await?;
        let before = before.ok_or_else(current_actor_password_denied)?;
        let after = after.ok_or_else(current_actor_password_denied)?;
        after.require_authenticated_actor(&after_session)?;
        if before != after { return Err(current_actor_password_denied()); }
        Ok(CurrentActorPasswordReauthentication {
            storage: self.clone(), operation_id, enrollment_operation_id, token_hash,
            requested_channel_hash: channel_hash.to_owned(), observed: after,
        })
    }

    // Canonical server-boundary recheck, not a same-borrowed-transaction witness.
    // It does not replace that transaction's real actor/source/credential fences.
    pub(super) async fn recheck_current_actor_password(
        &self, observed: &CurrentActorPasswordReauthentication,
        session_token: &str, channel_hash: &str,
    ) -> Result<(), ResourceAuthorityError> {
        if !std::sync::Arc::ptr_eq(&self.inner, &observed.storage.inner)
            || observed.token_hash != hex::encode(Sha256::digest(session_token.as_bytes()))
            || observed.requested_channel_hash != channel_hash
        { return Err(current_actor_password_denied()); }
        let session = self.authenticate_local_session(session_token, channel_hash)
            .await.map_err(|_| current_actor_password_denied())?;
        let current = self.read_current_actor_password(
            &session, observed.token_hash.clone(), channel_hash.to_owned(),
        ).await?.ok_or_else(current_actor_password_denied)?;
        current.require_authenticated_actor(&session)?;
        if current != observed.observed { return Err(current_actor_password_denied()); }
        Ok(())
    }
}
impl CurrentActorPasswordReauthentication {
    // This checks an observation against the current genuine capture facts; it
    // grants no administrator role/capability and cannot mint a new observation.
    pub(super) fn require_binding(
        &self, storage: &SurrealStorage, operation_id: Uuid, enrollment_operation_id: Uuid,
        session: &LocalSessionContext, session_token: &str, channel_hash: &str,
        policy_versions: &[i64], revocation_epochs: &[i64],
    ) -> Result<(), ResourceAuthorityError> {
        self.observed.require_authenticated_actor(session)?;
        let expected_policy = [
            self.observed.account_policy, self.observed.principal_policy,
            self.observed.space_policy, self.observed.session_policy,
        ];
        let expected_epochs = [
            self.observed.account_epoch, self.observed.principal_epoch, self.observed.space_epoch,
            self.observed.session_account_epoch, self.observed.session_principal_epoch, self.observed.session_space_epoch,
        ];
        if !std::sync::Arc::ptr_eq(&storage.inner, &self.storage.inner)
            || self.operation_id != operation_id || self.enrollment_operation_id != enrollment_operation_id
            || self.token_hash != hex::encode(Sha256::digest(session_token.as_bytes()))
            || self.requested_channel_hash != channel_hash
            || policy_versions.get(2..6) != Some(expected_policy.as_slice())
            || revocation_epochs != expected_epochs.as_slice()
            || self.observed.session_expires_at.clone().into_inner() <= chrono::Utc::now()
        { return Err(current_actor_password_denied()); }
        Ok(())
    }
}

// Metadata-only fence, requiring actual new credential_revision guard adoption.
// Uses the existing legitimate account authorization_touch_nonce mutation; PHC
// remains unreadable under ordinary record-user permissions and is never bound.
#[derive(SurrealValue)]
struct CurrentActorCredentialFenceBindings {
    account: RecordId,
    principal: RecordId,
    space: RecordId,
    session: RecordId,
    credential_revision: i64,
    account_epoch: i64,
    account_policy: i64,
    session_policy: i64,
}
#[derive(SurrealValue)]
struct CurrentActorCredentialFenceRow {
    id: RecordId,
    credential_revision: i64,
}
impl CurrentActorPasswordReauthentication {
    pub(super) async fn fence_in_transaction(
        &self, storage: &SurrealStorage,
        transaction: &super::SurrealTransactionContext<'_>,
        scope: &super::resource_authority::RecordUserScope,
        operation_id: Uuid, enrollment_operation_id: Uuid,
    ) -> Result<(), ResourceAuthorityError> {
        if !std::sync::Arc::ptr_eq(&storage.inner, &self.storage.inner)
            || self.operation_id != operation_id || self.enrollment_operation_id != enrollment_operation_id
            || self.observed.session_id != RecordId::new("authenticated_sessions", scope.session_id.clone())
            || self.token_hash != hex::encode(Sha256::digest(scope.session_token.as_bytes()))
            || scope.channel_binding_hash.as_deref() != Some(self.requested_channel_hash.as_str())
            || self.observed.credential_revision <= 0
        { return Err(current_actor_password_denied()); }
        // Existing RAM provider attestation compares actual protected SDK tx,
        // storage identity and all current/protected/expected scope fields.
        transaction.require_authenticated_scope(storage, scope)?;
        let rows: Vec<CurrentActorCredentialFenceRow> = transaction.query_values_at(
            "UPDATE $account SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1
             WHERE id = $auth.account_id AND $auth.id = $session
             AND $auth.principal_id = $principal AND $auth.access_space_id = $space
             AND $principal.account_id = id AND $space.account_id = id
             AND fn::mt109_live_session() AND $auth.id.policy_version = $session_policy
             AND credential_revision = $credential_revision AND revocation_epoch = $account_epoch AND policy_version = $account_policy
             RETURN id, credential_revision;",
            CurrentActorCredentialFenceBindings {
                account: self.observed.account_id.clone(), principal: self.observed.principal_id.clone(),
                space: self.observed.access_space_id.clone(), session: self.observed.session_id.clone(),
                credential_revision: self.observed.credential_revision, account_epoch: self.observed.account_epoch,
                account_policy: self.observed.account_policy, session_policy: self.observed.session_policy,
            }, 0,
        ).await?;
        let [row] = rows.as_slice() else { return Err(current_actor_password_denied()); };
        if row.id != self.observed.account_id || row.credential_revision != self.observed.credential_revision {
            return Err(current_actor_password_denied());
        }
        Ok(())
    }
}
// Install pre-effect AND precommit on the same actual mutation context. A changed
// revision rejects even if the PHC changed back to its previous value. The nonce
// UPDATE places that account record in the mutation transaction's write set;
// actual same-record concurrent-write conflict/finalization requires pinned-engine
// proof and is not inferred from successful static review.
// No freshness TTL, generic epoch guarantee, PHC read permission, role shortcut,
// session mint or retained Unknown settlement is introduced.

// RAM ONLY local_accounts.rs addendum to exact reviewed 2a954 helper.
// These methods expose credential revision METADATA only. The revision itself
// never attests password knowledge or grants a capability/table mutation.
impl CurrentActorPasswordReauthentication {
    pub(super) fn bound_credential_revision(
        &self, storage: &SurrealStorage, operation_id: Uuid, enrollment_operation_id: Uuid,
        session: &LocalSessionContext, session_token: &str, channel_hash: &str,
        policy_versions: &[i64], revocation_epochs: &[i64],
    ) -> Result<i64, ResourceAuthorityError> {
        self.require_binding(
            storage, operation_id, enrollment_operation_id, session, session_token,
            channel_hash, policy_versions, revocation_epochs,
        )?;
        if self.observed.credential_revision <= 0 {
            return Err(current_actor_password_denied());
        }
        Ok(self.observed.credential_revision)
    }

    pub(super) async fn fenced_credential_revision_in_transaction(
        &self, storage: &SurrealStorage, transaction: &super::SurrealTransactionContext<'_>,
        scope: &super::resource_authority::RecordUserScope,
        operation_id: Uuid, enrollment_operation_id: Uuid,
    ) -> Result<i64, ResourceAuthorityError> {
        self.fence_in_transaction(storage, transaction, scope, operation_id, enrollment_operation_id).await?;
        // The actual existing fence required a singleton canonical row with this
        // exact revision after the guarded nonce write in this exact SDK transaction.
        Ok(self.observed.credential_revision)
    }
}

// local_accounts.rs addition replacing the reviewed target-password reader fragment.
// Existing verify_local_owner_password and login_local_owner remain unchanged.
#[derive(Clone, PartialEq, Eq, SurrealValue)]
struct PasswordLoginAccountRow {
    account_id: RecordId,
    account_name: String,
    account_role: String,
    status: String,
    password_verifier: String,
    credential_revision: i64,
    revocation_epoch: i64,
    policy_version: i64,
}

pub(super) struct PasswordVerifiedLocalAccount {
    row: PasswordLoginAccountRow,
    storage: SurrealStorage,
}

// Metadata for the private issuer, not a transferable password proof.
#[derive(SurrealValue)]
pub(super) struct MemberPasswordCredentialSnapshot {
    pub(super) account: RecordId,
    pub(super) name: String,
    pub(super) verifier: String,
    pub(super) credential_revision: i64,
    pub(super) revocation_epoch: i64,
    pub(super) policy_version: i64,
}

impl PasswordVerifiedLocalAccount {
    pub(super) fn credential_snapshot(
        &self,
        storage: &SurrealStorage,
    ) -> Result<MemberPasswordCredentialSnapshot, ResourceAuthorityError> {
        if !std::sync::Arc::ptr_eq(&self.storage.inner, &storage.inner)
            || self.row.credential_revision <= 0
        {
            return Err(ResourceAuthorityError::InvalidInput("authentication denied"));
        }
        Ok(MemberPasswordCredentialSnapshot {
            account: self.row.account_id.clone(),
            name: self.row.account_name.clone(),
            verifier: self.row.password_verifier.clone(),
            credential_revision: self.row.credential_revision,
            revocation_epoch: self.row.revocation_epoch,
            policy_version: self.row.policy_version,
        })
    }
}

#[derive(SurrealValue)]
struct PasswordLoginRoleRow {
    account_role: String,
}

impl SurrealStorage {
    async fn read_member_password_account(
        &self,
        name: String,
    ) -> Result<Option<PasswordLoginAccountRow>, ResourceAuthorityError> {
        let namespace = self.config().namespace().to_owned();
        let database = self.config().database().to_owned();
        let row = self.with_lease(move |client| Box::pin(async move {
            let authority = client.clone();
            authority.use_ns(namespace).use_db(database).await?;
            let mut response = authority.query(
                "SELECT id AS account_id, account_key AS account_name, account_role, status, password_verifier, credential_revision, revocation_epoch, policy_version FROM local_accounts WHERE account_key = $name AND account_role = 'Member' AND status = 'enabled' AND password_verifier != NONE AND credential_revision > 0 LIMIT 2;"
            ).bind(("name", name)).await?.check()?;
            let mut rows: Vec<PasswordLoginAccountRow> = response.take(0)?;
            Ok(if rows.len() == 1 { rows.pop() } else { None })
        })).await?;
        Ok(row)
    }

    async fn verify_target_local_account_password(
        &self,
        account_name: &str,
        password: String,
    ) -> Result<PasswordVerifiedLocalAccount, ResourceAuthorityError> {
        if password.len() < 12 || password.len() > MAX_PASSWORD_BYTES {
            return Err(ResourceAuthorityError::InvalidInput("authentication denied"));
        }
        let name = account_name.trim().to_owned();
        let before = self.read_member_password_account(name.clone()).await?;
        let verifier = before.as_ref().map(|row| row.password_verifier.clone());
        let valid = run_password_worker(move || match verifier {
            Some(verifier) => verify_password(&password, &verifier),
            None => {
                let _ = password_verifier("unavailable-local-account");
                false
            }
        }).await?;
        let after = self.read_member_password_account(name).await?;
        if !valid || before.as_ref() != after.as_ref() {
            return Err(ResourceAuthorityError::Denied {
                decision_id: Uuid::now_v7().to_string(),
            });
        }
        let row = after.ok_or(ResourceAuthorityError::InvalidInput("authentication denied"))?;
        Ok(PasswordVerifiedLocalAccount { row, storage: self.clone() })
    }

    pub async fn login_local_account(
        &self,
        account_name: &str,
        password: String,
    ) -> Result<IssuedSessionCredential, ResourceAuthorityError> {
        let name = account_name.trim().to_owned();
        let namespace = self.config().namespace().to_owned();
        let database = self.config().database().to_owned();
        let role = self.with_lease(move |client| Box::pin(async move {
            let authority = client.clone();
            authority.use_ns(namespace).use_db(database).await?;
            let mut response = authority.query(
                "SELECT account_role FROM local_accounts WHERE account_key = $name LIMIT 2;"
            ).bind(("name", name)).await?.check()?;
            let mut rows: Vec<PasswordLoginRoleRow> = response.take(0)?;
            Ok(if rows.len() == 1 { rows.pop() } else { None })
        })).await?;
        if role.as_ref().map(|row| row.account_role.as_str()) == Some("Owner") {
            return self.login_local_owner(account_name, password).await;
        }
        let verified = self.verify_target_local_account_password(account_name, password).await?;
        self.provision_verified_member_session_credential(verified, Duration::from_secs(60)).await
    }
}

// Exact existing api/authority.rs login_owner call-site substitution only:
// .login_local_owner(&input.account_name, input.password)
// becomes:
// .login_local_account(&input.account_name, input.password)
// The parent supplies the private issuer in resource_authority.rs with this ABI:
// pub(super) async fn provision_verified_member_session_credential(
//     &self,
//     checked: super::local_accounts::PasswordVerifiedLocalAccount,
//     ttl: Duration,
// ) -> Result<IssuedSessionCredential, ResourceAuthorityError>
// The retained local_accounts.rs consumed-mapping resolver can still access checked.row.

#[derive(SurrealValue)]
struct RedeemedMemberLoginRow {
    account_id: RecordId,
    principal_id: RecordId,
    access_space_id: RecordId,
    operation_id: String,
    descriptor_hex: String,
    descriptor_sha256: String,
    account_name: String,
}
#[derive(SurrealValue)]
struct RedeemedMemberLoginBindings {
    account: RecordId,
    verifier: String,
    credential_revision: i64,
    revocation_epoch: i64,
    policy_version: i64,
}
impl SurrealStorage {
    pub(super) async fn resolve_redeemed_member_login_in_transaction(
        &self, tx: &super::SurrealTransactionContext<'_>, checked: &PasswordVerifiedLocalAccount,
    ) -> Result<ProvisionedIdentity, ResourceAuthorityError> {
        // Required private provider: this transaction must be authorized to read
        // private enrollment mapping and credential fields. No root/login bypass.
        let rows: Vec<RedeemedMemberLoginRow> = tx.query_values_at(
            "SELECT account_id, principal_id, access_space_id, operation_id, descriptor_hex, descriptor_sha256, account_name FROM member_enrollments
             WHERE account_id = $account AND status = 'consumed' AND purpose = 'identity_enrollment'
             AND password_verifier != NONE AND redemption_attempt_id != NONE AND consumed_at != NONE
             AND account_id.status = 'enabled' AND account_id.account_role = 'Member' AND account_id.account_key = account_name
             AND account_id.password_verifier = $verifier AND account_id.credential_revision = $credential_revision AND account_id.revocation_epoch = $revocation_epoch AND account_id.policy_version = $policy_version
             AND principal_id.account_id = account_id AND principal_id.status = 'enabled' AND principal_id.principal_kind = 'human_account'
             AND access_space_id.account_id = account_id AND access_space_id.status = 'active' LIMIT 2;",
            RedeemedMemberLoginBindings {
                account: checked.row.account_id.clone(), verifier: checked.row.password_verifier.clone(),
                credential_revision: checked.row.credential_revision,
                revocation_epoch: checked.row.revocation_epoch, policy_version: checked.row.policy_version,
            }, 0,
        ).await?;
        let [row] = rows.as_slice() else { return Err(ResourceAuthorityError::InvalidInput("authentication denied")); };
        let bytes = hex::decode(&row.descriptor_hex)
            .map_err(|_| ResourceAuthorityError::InvalidInput("authentication denied"))?;
        let descriptor = handshake_member_enrollment::command_plan::decode_enrollment(&bytes)
            .map_err(|_| ResourceAuthorityError::InvalidInput("authentication denied"))?;
        let staged = descriptor.staged();
        if hex::encode(sha2::Sha256::digest(&bytes)) != row.descriptor_sha256
            || descriptor.operation_id().to_string() != row.operation_id
            || descriptor.account_name() != row.account_name
            || row.account_id != RecordId::new("local_accounts", staged.account_id())
            || row.principal_id != RecordId::new("principals", staged.principal_id())
            || row.access_space_id != RecordId::new("access_spaces", staged.access_space_id())
        { return Err(ResourceAuthorityError::InvalidInput("authentication denied")); }
        Ok(ProvisionedIdentity {
            account_id: record_key(row.account_id.clone())?, principal_id: record_key(row.principal_id.clone())?,
            access_space_id: record_key(row.access_space_id.clone())?,
        })
    }
}
// The existing login/session ABI is not replaced by this resolver. Final credential
// issue must consume this exact observation with current lifecycle/credential fences
// in the owned sealed transaction. No session or grant is returned by redemption.


#[derive(Clone, Copy, Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalAccountLoginTarget {
    Default,
    MemberEnrollmentAdministrator,
}

pub(super) struct AdministratorPasswordVerifiedAccount {
    row: PasswordLoginAccountRow,
    storage: SurrealStorage,
}
#[derive(SurrealValue)]
pub(super) struct AdministratorPasswordCredentialSnapshot {
    pub(super) account: RecordId,
    pub(super) name: String,
    pub(super) verifier: String,
    pub(super) credential_revision: i64,
    pub(super) revocation_epoch: i64,
    pub(super) policy_version: i64,
}
impl AdministratorPasswordVerifiedAccount {
    pub(super) fn credential_snapshot(
        &self, storage: &SurrealStorage,
    ) -> Result<AdministratorPasswordCredentialSnapshot, ResourceAuthorityError> {
        if !std::sync::Arc::ptr_eq(&self.storage.inner, &storage.inner)
            || self.row.account_role != "Owner" || self.row.status != "enabled"
            || self.row.credential_revision <= 0 {
            return Err(ResourceAuthorityError::InvalidInput("authentication denied"));
        }
        Ok(AdministratorPasswordCredentialSnapshot {
            account: self.row.account_id.clone(), name: self.row.account_name.clone(),
            verifier: self.row.password_verifier.clone(),
            credential_revision: self.row.credential_revision,
            revocation_epoch: self.row.revocation_epoch, policy_version: self.row.policy_version,
        })
    }
}
impl SurrealStorage {
    async fn read_administrator_password_account(
        &self, name: String,
    ) -> Result<Option<PasswordLoginAccountRow>, ResourceAuthorityError> {
        let namespace = self.config().namespace().to_owned();
        let database = self.config().database().to_owned();
        self.with_lease(move |client| Box::pin(async move {
            let authority = client.clone();
            authority.use_ns(namespace).use_db(database).await?;
            let mut response = authority.query(
                "SELECT id AS account_id, account_key AS account_name, account_role, status, password_verifier, credential_revision, revocation_epoch, policy_version FROM local_accounts WHERE account_key = $name AND account_role = 'Owner' AND status = 'enabled' AND password_verifier != NONE AND credential_revision > 0 LIMIT 2;"
            ).bind(("name", name)).await?.check()?;
            let mut rows: Vec<PasswordLoginAccountRow> = response.take(0)?;
            Ok(if rows.len() == 1 { rows.pop() } else { None })
        })).await.map_err(Into::into)
    }

    async fn verify_administrator_account_password(
        &self, account_name: &str, password: String,
    ) -> Result<AdministratorPasswordVerifiedAccount, ResourceAuthorityError> {
        if password.len() < 12 || password.len() > MAX_PASSWORD_BYTES {
            return Err(ResourceAuthorityError::InvalidInput("authentication denied"));
        }
        let name = account_name.trim().to_owned();
        let before = self.read_administrator_password_account(name.clone()).await?;
        let verifier = before.as_ref().map(|row| row.password_verifier.clone());
        let password = zeroize::Zeroizing::new(password);
        let valid = run_password_worker(move || match verifier {
            Some(verifier) => verify_password(&password, &verifier),
            None => { let _ = password_verifier("unavailable-local-account"); false }
        }).await?;
        let after = self.read_administrator_password_account(name).await?;
        if !valid || before.as_ref() != after.as_ref() {
            return Err(ResourceAuthorityError::Denied { decision_id: Uuid::now_v7().to_string() });
        }
        let row = after.ok_or(ResourceAuthorityError::InvalidInput("authentication denied"))?;
        Ok(AdministratorPasswordVerifiedAccount { row, storage: self.clone() })
    }

    pub async fn login_local_account_target(
        &self, account_name: &str, password: String, target: LocalAccountLoginTarget,
    ) -> Result<IssuedSessionCredential, ResourceAuthorityError> {
        match target {
            LocalAccountLoginTarget::Default => self.login_local_account(account_name, password).await,
            LocalAccountLoginTarget::MemberEnrollmentAdministrator => {
                let checked = self.verify_administrator_account_password(account_name, password).await?;
                self.provision_verified_administrator_session_credential(checked, Duration::from_secs(60)).await
            }
        }
    }
}

#[derive(SurrealValue)]
struct AdministratorConfirmationPasswordBinding {
    account: RecordId, principal: RecordId, space: RecordId, session: RecordId,
    operation: String, token_hash: String, channel: String, verifier: String,
    credential_revision: i64, account_epoch: i64, account_policy: i64,
    principal_epoch: i64, principal_policy: i64, space_epoch: i64, space_policy: i64,
    session_policy: i64, capabilities: Vec<String>, chain: Vec<String>,
    principal_capabilities: Vec<String>, principal_kind: String, actor_kind: String, actor_id: String, session_expires_at: surrealdb::types::Datetime,
}
impl AdministratorPasswordVerifiedAccount {
    pub(super) async fn from_confirmation_reauthentication(
        storage: &SurrealStorage, proof: &CurrentActorPasswordReauthentication,
    ) -> Result<Self, ResourceAuthorityError> {
        if !std::sync::Arc::ptr_eq(&storage.inner, &proof.storage.inner)
            || proof.observed.credential_revision <= 0
            || proof.observed.principal_profile != "Operator"
            || proof.observed.session_expires_at.clone().into_inner() <= chrono::Utc::now() {
            return Err(ResourceAuthorityError::InvalidInput("authentication denied"));
        }
        let bindings = AdministratorConfirmationPasswordBinding {
            account: proof.observed.account_id.clone(), principal: proof.observed.principal_id.clone(),
            space: proof.observed.access_space_id.clone(), session: proof.observed.session_id.clone(),
            operation: proof.operation_id.to_string(), token_hash: proof.token_hash.clone(),
            channel: proof.requested_channel_hash.clone(), verifier: proof.observed.password_verifier.clone(),
            credential_revision: proof.observed.credential_revision,
            account_epoch: proof.observed.account_epoch, account_policy: proof.observed.account_policy,
            principal_epoch: proof.observed.principal_epoch, principal_policy: proof.observed.principal_policy,
            space_epoch: proof.observed.space_epoch, space_policy: proof.observed.space_policy,
            session_policy: proof.observed.session_policy, capabilities: proof.observed.session_capabilities.clone(),
            chain: proof.observed.delegation_chain.clone(), principal_capabilities: proof.observed.principal_capabilities.clone(),
            principal_kind: proof.observed.principal_kind.clone(), actor_kind: proof.observed.actor_kind.clone(),
            actor_id: proof.observed.actor_id.clone(), session_expires_at: proof.observed.session_expires_at.clone(),
        };
        let namespace = storage.config().namespace().to_owned();
        let database = storage.config().database().to_owned();
        let row = storage.with_lease(move |client| Box::pin(async move {
            let authority = client.clone();
            authority.use_ns(namespace).use_db(database).await?;
            let mut response = authority.query(
              "SELECT id AS account_id, account_key AS account_name, account_role, status, password_verifier, credential_revision, revocation_epoch, policy_version FROM local_accounts
               WHERE id = $account AND account_role = 'Owner' AND status = 'enabled'
               AND password_verifier = $verifier AND credential_revision = $credential_revision
               AND revocation_epoch = $account_epoch AND policy_version = $account_policy
               AND first_owner_administration_origins:primary.account = id
               AND first_owner_administration_origins:primary.operator_principal = $principal
               AND first_owner_administration_origins:primary.space = $space
               AND first_owner_administration_origins:primary.purpose = 'prospective_first_owner_administration'
               AND first_owner_administration_origins:primary.version = 1
               AND first_owner_administration_origins:primary.confirmation_state = 'confirmed'
               AND first_owner_administration_origins:primary.confirmation_operation_id = $operation
               AND first_owner_administration_origins:primary.confirmation_actor_session = $session
               AND first_owner_administration_origins:primary.confirmation_channel_hash = $channel
               AND $session.account_id = id AND $session.principal_id = $principal AND $session.access_space_id = $space
               AND $session.token_hash = $token_hash AND $session.channel_binding_hash = $channel
               AND $session.revoked_at = NONE AND $session.expires_at > time::now() AND $session.expires_at = $session_expires_at
               AND $session.policy_version = $session_policy AND $session.delegated_capabilities = $capabilities
               AND $session.delegation_chain = $chain
               AND $session.account_revocation_epoch = $account_epoch
               AND $session.principal_revocation_epoch = $principal_epoch AND $session.space_revocation_epoch = $space_epoch
               AND $principal.account_id = id AND $principal.status = 'enabled'
               AND $principal.capability_profile_id = 'Operator' AND $principal.policy_version = $principal_policy
               AND $principal.delegated_capabilities = $principal_capabilities AND $principal.principal_kind = $principal_kind
               AND $principal.actor_kind = $actor_kind AND $principal.actor_id = $actor_id
               AND $principal.revocation_epoch = $principal_epoch
               AND $space.account_id = id AND $space.status = 'active'
               AND $space.policy_version = $space_policy AND $space.revocation_epoch = $space_epoch
               AND fn::mt109_session_lineage_live($session) LIMIT 2;"
            ).bind(bindings).await?.check()?;
            let mut rows: Vec<PasswordLoginAccountRow> = response.take(0)?;
            Ok(if rows.len() == 1 { rows.pop() } else { None })
        })).await?;
        let row = row.ok_or(ResourceAuthorityError::InvalidInput("authentication denied"))?;
        Ok(Self { row, storage: storage.clone() })
    }
}

// External local_accounts.rs candidate. Replaces only setup_local_owner; adds its retained owner.
#[derive(Clone, PartialEq, Eq, SurrealValue)]
struct FirstOwnerSetupReceipt {
    setup_operation_id: String,
    account: RecordId,
    operator_principal: RecordId,
    space: RecordId,
    purpose: String,
    version: i64,
    setup_receipt: RecordId,
    setup_policy_version: i64,
    created_at: surrealdb::types::Datetime,
}
#[derive(Clone, SurrealValue)]
struct FirstOwnerSetupExpected {
    name: String,
    verifier: String,
    capabilities: Vec<String>,
    event: super::event_ledger::LedgerWrite,
}
#[derive(SurrealValue)]
struct FirstOwnerSetupWrite {
    origin: FirstOwnerSetupReceipt,
    name: String,
    verifier: String,
    capabilities: Vec<String>,
    event: super::event_ledger::LedgerBulkInsert,
}
struct FirstOwnerSetupOriginal {
    storage: SurrealStorage,
    operation_id: Uuid,
    identity: ProvisionedIdentity,
    receipt: FirstOwnerSetupReceipt,
    expected: FirstOwnerSetupExpected,
    write: std::sync::Mutex<Option<FirstOwnerSetupWrite>>,
    deadline: std::time::Instant,
}
pub(super) enum FirstOwnerSetupFailure {
    Operation(ResourceAuthorityError),
    Cancel { operation: ResourceAuthorityError, cancellation: Result<super::SurrealClient, surrealdb::Error> },
    Commit(surrealdb::Error),
    CanonicalRead(super::SurrealStorageError),
    CanonicalMismatch,
}
#[derive(Clone)]
pub(super) enum FirstOwnerSetupObservation {
    Complete(std::sync::Arc<Result<ProvisionedIdentity, FirstOwnerSetupFailure>>),
    Join(std::sync::Arc<tokio::task::JoinError>),
}
enum FirstOwnerSetupWorkerState {
    Starting,
    Running(tokio::task::JoinHandle<Result<ProvisionedIdentity, FirstOwnerSetupFailure>>),
    Observed(FirstOwnerSetupObservation),
}
pub(super) struct FirstOwnerSetupOperation {
    original: std::sync::Arc<FirstOwnerSetupOriginal>,
    state: tokio::sync::Mutex<FirstOwnerSetupWorkerState>,
}
impl FirstOwnerSetupOperation {
    pub(super) async fn wait(&self) -> FirstOwnerSetupObservation {
        let mut state = self.state.lock().await;
        if let FirstOwnerSetupWorkerState::Running(handle) = &mut *state {
            let result = match handle.await {
                Ok(result) => FirstOwnerSetupObservation::Complete(std::sync::Arc::new(result)),
                Err(error) => FirstOwnerSetupObservation::Join(std::sync::Arc::new(error)),
            };
            *state = FirstOwnerSetupWorkerState::Observed(result);
        }
        match &*state {
            FirstOwnerSetupWorkerState::Observed(observation) => observation.clone(),
            _ => unreachable!("a setup owner is published with its worker before observation"),
        }
    }
    pub(super) async fn drain(&self) -> Result<(), ResourceAuthorityError> {
        match self.wait().await {
            FirstOwnerSetupObservation::Complete(result) if result.is_ok() => Ok(()),
            _ => Err(super::SurrealStorageError::TransactionSettlementPending {
                operation_id: self.original.operation_id,
            }.into()),
        }
    }
}
const FIRST_OWNER_SETUP_CREATE: &str = r#"
IF record::exists(local_account_setup:primary)
 OR record::exists(first_owner_administration_origins:primary)
 OR array::len(SELECT VALUE id FROM local_accounts WHERE account_role = 'Owner' LIMIT 1) != 0
 { THROW 'HSK-AUTH-SETUP-CLOSED'; };
CREATE local_account_setup:primary SET account_id = $origin.account,
 principal_id = $origin.operator_principal, access_space_id = $origin.space, created_at = $origin.created_at;
CREATE $origin.account SET account_key = $name, account_role = 'Owner', status = 'enabled',
 password_verifier = $verifier, credential_revision = 1, revocation_epoch = 0, policy_version = 1,
 first_owner_administration_origin = first_owner_administration_origins:primary,
 created_at = $origin.created_at, updated_at = $origin.created_at;
CREATE $origin.operator_principal SET principal_key = record::id($origin.operator_principal),
 account_id = $origin.account, principal_kind = 'human_account', actor_kind = 'operator',
 actor_id = record::id($origin.operator_principal), capability_profile_id = 'Operator',
 delegated_capabilities = $capabilities, status = 'enabled', revocation_epoch = 0,
 policy_version = 1, created_at = $origin.created_at, updated_at = $origin.created_at;
CREATE $origin.space SET space_key = record::id($origin.operator_principal), account_id = $origin.account,
 name = 'Private', status = 'active', revocation_epoch = 0, policy_version = 1,
 created_at = $origin.created_at, updated_at = $origin.created_at;
INSERT INTO kernel_event_ledger $event;
CREATE first_owner_administration_origins:primary SET setup_operation_id = $origin.setup_operation_id,
 account = $origin.account, operator_principal = $origin.operator_principal, space = $origin.space,
 purpose = $origin.purpose, version = $origin.version, setup_receipt = $origin.setup_receipt,
 setup_policy_version = $origin.setup_policy_version, first_workspace_state = 'unclaimed',
 first_workspace_receipt_nonce = 0, confirmation_state = 'unconfirmed', created_at = $origin.created_at;
"#;
const FIRST_OWNER_SETUP_READ: &str = r#"
SELECT setup_operation_id, account, operator_principal, space, purpose, version, setup_receipt,
 setup_policy_version, created_at FROM first_owner_administration_origins:primary
 WHERE setup_operation_id = $origin.setup_operation_id AND account = $origin.account
 AND operator_principal = $origin.operator_principal AND space = $origin.space
 AND purpose = 'prospective_first_owner_administration' AND version = 1
 AND setup_receipt = $origin.setup_receipt AND setup_policy_version = 1
 AND record::exists(setup_receipt) AND setup_receipt.event_type = 'LOCAL_ACCOUNT_SETUP'
 AND local_account_setup:primary.account_id = account
 AND local_account_setup:primary.principal_id = operator_principal
 AND local_account_setup:primary.access_space_id = space
 AND created_at = $origin.created_at
 AND local_account_setup:primary.created_at = $origin.created_at
 AND account.account_key = $expected.name AND account.password_verifier = $expected.verifier
 AND account.first_owner_administration_origin = first_owner_administration_origins:primary
 AND account.credential_revision = 1 AND account.revocation_epoch = 0 AND account.policy_version = 1
 AND account.created_at = $origin.created_at AND account.updated_at = $origin.created_at
 AND account.status = 'enabled' AND account.account_role = 'Owner'
 AND operator_principal.account_id = account AND operator_principal.status = 'enabled'
 AND operator_principal.capability_profile_id = 'Operator'
 AND operator_principal.principal_key = record::id($origin.operator_principal)
 AND operator_principal.principal_kind = 'human_account'
 AND operator_principal.actor_kind = 'operator' AND operator_principal.actor_id = record::id($origin.operator_principal)
 AND operator_principal.delegated_capabilities = $expected.capabilities
 AND operator_principal.revocation_epoch = 0 AND operator_principal.policy_version = 1
 AND operator_principal.created_at = $origin.created_at AND operator_principal.updated_at = $origin.created_at
 AND space.account_id = account AND space.status = 'active'
 AND space.space_key = record::id($origin.operator_principal) AND space.name = 'Private'
 AND space.revocation_epoch = 0 AND space.policy_version = 1
 AND space.created_at = $origin.created_at AND space.updated_at = $origin.created_at;
SELECT id AS record, event_id, event_version, kernel_task_run_id, session_run_id,
 aggregate_type, aggregate_id, idempotency_key, event_type, actor_kind, actor_id,
 causation_id, correlation_id, payload_hash, source_component, payload, wsids,
 authority_resource_id, authority_session_id, authority_capability_id, authority_action, created_at
 FROM $origin.setup_receipt;
"#;
#[derive(SurrealValue)]
struct FirstOwnerSetupRead { origin: FirstOwnerSetupReceipt, expected: FirstOwnerSetupExpected }
impl SurrealStorage {

    async fn run_prospective_first_owner_setup(
        &self, original: std::sync::Arc<FirstOwnerSetupOriginal>,
    ) -> Result<ProvisionedIdentity, FirstOwnerSetupFailure> {
        let write = original.write.lock().map_err(|_|
            FirstOwnerSetupFailure::Operation(ResourceAuthorityError::InvalidInput("setup original poisoned")))?
            .take().ok_or_else(|| FirstOwnerSetupFailure::Operation(
                ResourceAuthorityError::InvalidInput("setup original already dispatched")))?;
        let namespace = self.config().namespace().to_owned();
        let database = self.config().database().to_owned();
        let retained = original.clone();
        let result = self.with_lease(move |client| Box::pin(async move {
            let authority = client.clone();
            authority.use_ns(namespace).use_db(database).await?;
            if std::time::Instant::now() >= retained.deadline {
                return Ok(Err(FirstOwnerSetupFailure::Operation(
                    ResourceAuthorityError::InvalidInput("setup original deadline elapsed"))));
            }
            let transaction = authority.begin().await?;
            let body: Result<(), ResourceAuthorityError> = async {
                transaction.query(FIRST_OWNER_SETUP_CREATE).bind(write).await
                    .map_err(super::SurrealStorageError::from)?.check()
                    .map_err(super::SurrealStorageError::from)?;
                if std::time::Instant::now() >= retained.deadline {
                    return Err(ResourceAuthorityError::InvalidInput("setup original deadline elapsed"));
                }
                Ok(())
            }.await;
            if let Err(operation) = body {
                return Ok(Err(FirstOwnerSetupFailure::Cancel {
                    operation, cancellation: transaction.cancel().await,
                }));
            }
            let canonical = match transaction.commit().await {
                Ok(client) => client,
                Err(error) => return Ok(Err(FirstOwnerSetupFailure::Commit(error))),
            };
            let read: Result<(Vec<FirstOwnerSetupReceipt>, Vec<super::event_ledger::LedgerWrite>), super::SurrealStorageError> = async {
                let mut response = canonical.query(FIRST_OWNER_SETUP_READ)
                    .bind(FirstOwnerSetupRead { origin: retained.receipt.clone(), expected: retained.expected.clone() }).await?.check()?;
                Ok((response.take(0)?, response.take(1)?))
            }.await;
            match read {
                Ok((rows, events)) if rows.as_slice() == [retained.receipt.clone()].as_slice()
                    && events.len() == 1
                    && events[0].clone().into_value() == retained.expected.event.clone().into_value() =>
                    Ok(Ok(retained.identity.clone())),
                Ok(_) => Ok(Err(FirstOwnerSetupFailure::CanonicalMismatch)),
                Err(error) => Ok(Err(FirstOwnerSetupFailure::CanonicalRead(error))),
            }
        })).await;
        match result { Ok(result) => result, Err(error) => Err(FirstOwnerSetupFailure::CanonicalRead(error)) }
    }
    pub(crate) async fn drain_first_owner_setup_operations(&self) -> Result<(), ResourceAuthorityError> {
        self.close_member_login_admission()?;
        let owners: Vec<_> = {
            let retained = self.inner.transaction_settlements.lock().unwrap_or_else(|poison| poison.into_inner());
            retained.values().filter_map(|entry| match entry {
                Some(super::AuthenticatedTransactionRetention::FirstOwnerSetup(owner)) => Some(owner.clone()),
                _ => None,
            }).collect()
        };
        let mut first_error = None;
        for owner in owners {
            if let Err(error) = owner.drain().await { if first_error.is_none() { first_error = Some(error); } }
        }
        first_error.map_or(Ok(()), Err)
    }
}

// External local_accounts.rs producer component. CurrentActorPasswordRow additionally derives Clone.
// Uses Host789254 typed input/registration bridge and the actual Workspace-owned witness.
struct FirstOwnerConfirmationOriginal {
    operation_id: Uuid,
    administration_resource: RecordId,
    administrator: ProvisionedIdentity,
    grants: [RecordId; 3],
    input: super::resource_authority::AccountAdministrationRegistrationInput,
    deadline: std::time::Instant,
    workspace_lookup_errors: std::sync::Mutex<Vec<std::sync::Arc<ResourceAuthorityError>>>,
}
struct FirstOwnerConfirmationAuthorization<'tx> {
    storage: SurrealStorage,
    original: std::sync::Arc<FirstOwnerConfirmationOriginal>,
    session: LocalSessionContext,
    password: CurrentActorPasswordReauthentication,
    workspace: std::sync::Arc<super::workspaces::FirstOwnerWorkspaceCreationWitness>,
    origin: surrealdb::types::Object,
    transaction: &'tx super::SurrealTransactionContext<'tx>,
    birth: super::first_owner_bootstrap_transaction::SameStorageBootstrapTransaction<'tx>,
}
pub(super) struct ConfirmedFirstOwnerAdministrationRegistration<'tx> {
    proof: std::sync::Arc<FirstOwnerConfirmationAuthorization<'tx>>,
}
#[derive(SurrealValue)]
struct FirstOwnerBootstrapOriginRead {
    account: RecordId,
    principal: RecordId,
    space: RecordId,
    session: RecordId,
    channel: String,
    setup_operation_id: String,
    workspace_operation_id: String,
    parent: RecordId,
    external: String,
    policy: i64,
}
const FIRST_OWNER_CONFIRMATION_ORIGIN: &str = r#"
SELECT VALUE $this FROM first_owner_administration_origins:primary
 WHERE purpose = 'prospective_first_owner_administration' AND version = 1
 AND setup_operation_id = $setup_operation_id AND setup_receipt != NONE
 AND setup_receipt.event_type = 'LOCAL_ACCOUNT_SETUP'
 AND account = $account AND operator_principal = $principal AND space = $space
 AND setup_policy_version = 1
 AND local_account_setup:primary.account_id = account
 AND local_account_setup:primary.principal_id = operator_principal
 AND local_account_setup:primary.access_space_id = space
 AND first_workspace_state = 'created' AND first_workspace_operation_id = $workspace_operation_id
 AND first_workspace_resource = $parent AND first_workspace = type::record('workspaces', $external)
 AND first_workspace_session = $session AND first_workspace_channel_hash = $channel
 AND first_workspace_policy_version = $policy AND first_workspace_receipt_nonce > 0
 AND confirmation_state = 'unconfirmed' AND confirmation_operation_id = NONE;
"#;
#[derive(SurrealValue)]
struct FirstOwnerBootstrapFence {
    before: CurrentActorPasswordRow,
    origin: surrealdb::types::Object,
}
const FIRST_OWNER_BOOTSTRAP_FENCE: &str = r#"
LET $a = UPDATE $before.account_id SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1
 WHERE status = 'enabled' AND password_verifier = $before.password_verifier
 AND credential_revision = $before.credential_revision AND revocation_epoch = $before.account_epoch
 AND policy_version = $before.account_policy RETURN VALUE id;
IF array::len($a) != 1 { THROW 'HSK-FIRST-OWNER-CONFIRMATION-DENIED'; };
LET $p = UPDATE $before.principal_id SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1
 WHERE account_id = $before.account_id AND status = 'enabled'
 AND principal_kind = $before.principal_kind AND actor_kind = $before.actor_kind AND actor_id = $before.actor_id
 AND capability_profile_id = $before.principal_profile AND delegated_capabilities = $before.principal_capabilities
 AND revocation_epoch = $before.principal_epoch AND policy_version = $before.principal_policy RETURN VALUE id;
IF array::len($p) != 1 { THROW 'HSK-FIRST-OWNER-CONFIRMATION-DENIED'; };
LET $s = UPDATE $before.access_space_id SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1
 WHERE account_id = $before.account_id AND status = 'active'
 AND revocation_epoch = $before.space_epoch AND policy_version = $before.space_policy RETURN VALUE id;
IF array::len($s) != 1 { THROW 'HSK-FIRST-OWNER-CONFIRMATION-DENIED'; };
LET $t = UPDATE $before.session_id SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1
 WHERE account_id = $before.account_id AND principal_id = $before.principal_id
 AND access_space_id = $before.access_space_id AND revoked_at = NONE AND expires_at = $before.session_expires_at
 AND expires_at > time::now() AND policy_version = $before.session_policy
 AND account_revocation_epoch = $before.session_account_epoch
 AND principal_revocation_epoch = $before.session_principal_epoch
 AND space_revocation_epoch = $before.session_space_epoch
 AND delegated_capabilities = $before.session_capabilities AND delegation_chain = $before.delegation_chain
 AND channel_binding_hash = $before.channel_binding_hash AND fn::mt109_session_lineage_live(id) RETURN VALUE id;
IF array::len($t) != 1 { THROW 'HSK-FIRST-OWNER-CONFIRMATION-DENIED'; };
LET $o = UPDATE first_owner_administration_origins:primary
 SET first_workspace_receipt_nonce = first_workspace_receipt_nonce
 WHERE $this = $origin AND confirmation_state = 'unconfirmed' RETURN VALUE id;
IF array::len($o) != 1 { THROW 'HSK-FIRST-OWNER-CONFIRMATION-DENIED'; };
RETURN true;
"#;
const FIRST_OWNER_CURRENT_PASSWORD_READ: &str = "SELECT account_id, principal_id, access_space_id, id AS session_id, account_id.password_verifier AS password_verifier, account_id.credential_revision AS credential_revision,\n                 account_id.revocation_epoch AS account_epoch, principal_id.revocation_epoch AS principal_epoch, access_space_id.revocation_epoch AS space_epoch,\n                 account_revocation_epoch AS session_account_epoch, principal_revocation_epoch AS session_principal_epoch, space_revocation_epoch AS session_space_epoch,\n                 account_id.policy_version AS account_policy, principal_id.policy_version AS principal_policy, access_space_id.policy_version AS space_policy, policy_version AS session_policy,\n                 principal_id.actor_id AS actor_id, principal_id.actor_kind AS actor_kind, principal_id.principal_kind AS principal_kind,\n                 principal_id.capability_profile_id AS principal_profile, principal_id.delegated_capabilities AS principal_capabilities,\n                 delegated_capabilities AS session_capabilities, delegation_chain, expires_at AS session_expires_at, channel_binding_hash\n                 FROM authenticated_sessions WHERE id = $session AND account_id = $account AND principal_id = $principal AND access_space_id = $space\n                 AND principal_id.account_id = $account AND access_space_id.account_id = $account\n                 AND token_hash = $token_hash AND (channel_binding_hash = NONE OR channel_binding_hash = $channel_hash)\n                 AND account_id.password_verifier != NONE AND account_id.credential_revision > 0 AND fn::mt109_session_lineage_live(id) LIMIT 2;";
impl FirstOwnerConfirmationAuthorization<'_> {
    fn live(&self) -> Result<(), ResourceAuthorityError> {
        if std::time::Instant::now() >= self.original.deadline
            || self.password.operation_id != self.original.operation_id
            || self.password.enrollment_operation_id != self.original.operation_id
            || !std::sync::Arc::ptr_eq(&self.storage.inner, &self.password.storage.inner)
            || self.password.observed.principal_profile != "Operator"
            || self.password.observed.principal_kind != "human_account"
            || self.password.observed.actor_kind != "operator"
            || self.session.identity.account_id != self.original.administrator.account_id
            || self.session.identity.access_space_id != self.original.administrator.access_space_id
            || self.session.identity.principal_id == self.original.administrator.principal_id
        { return Err(current_actor_password_denied()); }
        if let super::resource_authority::AdministrationLifetimeChoice::ExpiresAt(expiry) = self.original.input.lifetime {
            let expiry = chrono::DateTime::<chrono::Utc>::from_timestamp(expiry.unix_seconds(), expiry.nanoseconds())
                .ok_or_else(current_actor_password_denied)?;
            if expiry <= chrono::Utc::now() { return Err(current_actor_password_denied()); }
        }
        self.password.observed.require_authenticated_actor(&self.session)?;
        Ok(())
    }
    fn origin_query(&self) -> FirstOwnerBootstrapOriginRead {
        FirstOwnerBootstrapOriginRead {
            account: self.password.observed.account_id.clone(), principal: self.password.observed.principal_id.clone(),
            space: self.password.observed.access_space_id.clone(), session: self.password.observed.session_id.clone(),
            channel: self.password.requested_channel_hash.clone(),
            setup_operation_id: self.workspace.setup_operation_id().to_string(),
            workspace_operation_id: self.workspace.operation_id().to_string(),
            parent: self.workspace.parent_workspace().id.clone(),
            external: self.workspace.parent_workspace_external_id().to_owned(), policy: self.session.policy_version,
        }
    }
    async fn require_current_in_transaction(&self, transaction: &super::SurrealTransactionContext<'_>)
        -> Result<(), ResourceAuthorityError>
    {
        self.live()?;
        self.birth.require_storage_context(&self.storage, transaction)?;
        if !std::ptr::eq(transaction.transaction, self.transaction.transaction) {
            return Err(current_actor_password_denied());
        }
        self.workspace.require_current_in_transaction(transaction, &self.storage, &self.session,
            &self.password.requested_channel_hash).await?;
        let current: Vec<CurrentActorPasswordRow> = transaction.query_values_at(
            FIRST_OWNER_CURRENT_PASSWORD_READ, CurrentActorPasswordBindings {
                account: self.password.observed.account_id.clone(), principal: self.password.observed.principal_id.clone(),
                space: self.password.observed.access_space_id.clone(), session: self.password.observed.session_id.clone(),
                token_hash: self.password.token_hash.clone(), channel_hash: self.password.requested_channel_hash.clone(),
            }, 0,
        ).await?;
        if current.as_slice() != [self.password.observed.clone()].as_slice() {
            return Err(current_actor_password_denied());
        }
        let origins: Vec<surrealdb::types::Object> = transaction.query_values_at(
            FIRST_OWNER_CONFIRMATION_ORIGIN, self.origin_query(), 0,
        ).await?;
        if origins.as_slice() != [self.origin.clone()].as_slice() { return Err(current_actor_password_denied()); }
        // A physical write fence serializes credential, identity and origin changes with this bootstrap TX.
        let mut response = transaction.transaction.query(FIRST_OWNER_BOOTSTRAP_FENCE)
            .bind(FirstOwnerBootstrapFence { before: self.password.observed.clone(), origin: self.origin.clone() })
            .await.map_err(super::SurrealStorageError::from)?.check().map_err(super::SurrealStorageError::from)?;
        let value: surrealdb::types::Value = response.take(10).map_err(super::SurrealStorageError::from)?;
        if !matches!(value, surrealdb::types::Value::Bool(true)) { return Err(current_actor_password_denied()); }
        self.live()?;
        Ok(())
    }
}
impl ConfirmedFirstOwnerAdministrationRegistration<'_> {
    pub(super) fn owner(&self) -> &ProvisionedIdentity { &self.proof.session.identity }
    pub(super) fn session_id(&self) -> &str { &self.proof.session.session_id }
    pub(super) fn parent_workspace(&self) -> &super::resource_authority::CanonicalResourceLocator {
        self.proof.workspace.parent_workspace()
    }
    pub(super) fn parent_workspace_external_id(&self) -> &str { self.proof.workspace.parent_workspace_external_id() }
    pub(super) fn administration_resource(&self) -> &RecordId { &self.proof.original.administration_resource }
    pub(super) fn require_input(&self, input: &super::resource_authority::AccountAdministrationRegistrationInput)
        -> Result<(), ResourceAuthorityError>
    {
        self.proof.live()?;
        if input != &self.proof.original.input { return Err(current_actor_password_denied()); }
        Ok(())
    }
    pub(super) async fn require_current_in_transaction(&self, tx: &super::SurrealTransactionContext<'_>)
        -> Result<(), ResourceAuthorityError> { self.proof.require_current_in_transaction(tx).await }
}
impl SurrealStorage {
    // Only the retained prospective confirmation worker in this module may call this producer.
    async fn capture_first_owner_registration_in_transaction<'tx>(
        &self, transaction: &'tx super::SurrealTransactionContext<'tx>,
        birth: super::first_owner_bootstrap_transaction::SameStorageBootstrapTransaction<'tx>,
        original: std::sync::Arc<FirstOwnerConfirmationOriginal>,
        session_token: &str, channel_hash: &str, password: String,
        first_workspace_operation_id: Uuid,
    ) -> Result<std::sync::Arc<FirstOwnerConfirmationAuthorization<'tx>>, ResourceAuthorityError> {
        birth.require_storage_context(self, transaction)?;
        if std::time::Instant::now() >= original.deadline { return Err(current_actor_password_denied()); }
        let reauthentication = self.reauthenticate_current_actor_password(original.operation_id,
            original.operation_id, session_token, channel_hash, password).await?;
        let session = self.authenticate_local_session(session_token, channel_hash).await?;
        let workspace = match self.first_owner_workspace_creation_witness(first_workspace_operation_id).await {
            Ok(workspace) => workspace,
            Err(error) => {
                original.workspace_lookup_errors.lock().map_err(|_| current_actor_password_denied())?
                    .push(std::sync::Arc::new(error));
                // The Workspace provider itself rejects recovery whenever a live retained
                // original exists, including failed/Unknown originals. No error-string routing.
                self.recover_first_owner_workspace_creation_witness(transaction, &session,
                    channel_hash, first_workspace_operation_id).await?
            }
        };
        reauthentication.observed.require_authenticated_actor(&session)?;
        if original.input.parent_workspace_external_id != workspace.parent_workspace_external_id()
            || original.input.resource_policy_version != session.policy_version
            || original.input.classification != "account_private"
            || original.input.external_resource_id != record_key(original.administration_resource.clone())?
        { return Err(current_actor_password_denied()); }
        let query = FirstOwnerBootstrapOriginRead {
            account: reauthentication.observed.account_id.clone(), principal: reauthentication.observed.principal_id.clone(),
            space: reauthentication.observed.access_space_id.clone(), session: reauthentication.observed.session_id.clone(),
            channel: channel_hash.to_owned(), setup_operation_id: workspace.setup_operation_id().to_string(),
            workspace_operation_id: workspace.operation_id().to_string(), parent: workspace.parent_workspace().id.clone(),
            external: workspace.parent_workspace_external_id().to_owned(), policy: session.policy_version,
        };
        let mut origins: Vec<surrealdb::types::Object> = transaction.query_values_at(FIRST_OWNER_CONFIRMATION_ORIGIN, query, 0).await?;
        if origins.len() != 1 { return Err(current_actor_password_denied()); }
        let origin = origins.pop().ok_or_else(current_actor_password_denied)?;
        let proof = std::sync::Arc::new(FirstOwnerConfirmationAuthorization {
            storage: self.clone(), original, session, password: reauthentication, workspace, origin,
            transaction, birth,
        });
        proof.require_current_in_transaction(transaction).await?;
        Ok(proof)
    }
}

pub(crate) enum FirstOwnerAdministrationConfirmationLifetime {
    NoExpiry,
    ExpiresAt { expires_at: chrono::DateTime<chrono::Utc> },
}
pub(crate) struct FirstOwnerAdministrationConfirmationRequest {
    pub(crate) operation_id: Uuid,
    pub(crate) first_workspace_operation_id: Uuid,
    pub(crate) parent_workspace_external_id: String,
    pub(crate) lifetime: FirstOwnerAdministrationConfirmationLifetime,
    pub(crate) actor: LocalSessionContext,
    pub(crate) session_token: zeroize::Zeroizing<String>,
    pub(crate) channel_hash: String,
    pub(crate) password: zeroize::Zeroizing<String>,
}
pub(super) struct FirstOwnerConfirmationCredentialIntent {
    record: RecordId,
    token: zeroize::Zeroizing<String>,
    token_hash: String,
    requested_expires_at: chrono::DateTime<chrono::Utc>,
}
impl FirstOwnerConfirmationCredentialIntent {
    pub(super) fn record(&self) -> &RecordId { &self.record }
    pub(super) fn token(&self) -> &str { self.token.as_str() }
    pub(super) fn token_hash(&self) -> &str { &self.token_hash }
    pub(super) fn requested_expires_at(&self) -> chrono::DateTime<chrono::Utc> { self.requested_expires_at }
}
struct FirstOwnerConfirmationFrame {
    request: FirstOwnerAdministrationConfirmationRequest,
    original: Option<std::sync::Arc<FirstOwnerConfirmationOriginal>>,
    credential: Option<FirstOwnerConfirmationCredentialIntent>,
    event: Option<crate::kernel::NewKernelEvent>,
    retention_id: Uuid,
    deadline: std::time::Instant,
    storage: SurrealStorage,
    captured_origin: std::sync::OnceLock<surrealdb::types::Object>,
    expected_origin: std::sync::OnceLock<surrealdb::types::Object>,
    event_result: std::sync::OnceLock<std::sync::Arc<Result<super::event_ledger::LedgerWrite, crate::storage::StorageError>>>,
    expected: std::sync::OnceLock<FirstOwnerConfirmationCanonical>,
    canonical: std::sync::OnceLock<std::sync::Arc<Result<Vec<FirstOwnerConfirmationCanonical>, super::SurrealStorageError>>>,
    actor_canonical: std::sync::OnceLock<std::sync::Arc<Result<Vec<CurrentActorPasswordRow>, super::SurrealStorageError>>>,
    credential_canonical: std::sync::OnceLock<std::sync::Arc<Result<surrealdb::types::Value, super::SurrealStorageError>>>,
    replay: std::sync::OnceLock<FirstOwnerAdministrationConfirmationObservation>,
    replay_read: std::sync::OnceLock<std::sync::Arc<Result<FirstOwnerConfirmationReplayRead, super::SurrealStorageError>>>,
    bootstrap: std::sync::Mutex<Option<std::sync::Arc<FirstOwnerConfirmationBootstrapResult>>>,
    retired: std::sync::Mutex<Option<FirstOwnerConfirmationRetiredTerminal>>,
    pending_observation: std::sync::OnceLock<std::sync::Arc<Result<IssuedSessionCredential,FirstOwnerAdministrationConfirmationFailure>>>,
    body_entered: std::sync::atomic::AtomicBool,
    read_only: std::sync::atomic::AtomicBool,
}
pub(crate) enum FirstOwnerAdministrationConfirmationFailure {
    Operation(ResourceAuthorityError),
    Entropy(getrandom::Error),
    Audit(std::sync::Arc<Result<super::event_ledger::LedgerWrite, crate::storage::StorageError>>),
    Replay,
    Bootstrap,
}
#[derive(Clone)]
pub(crate) enum FirstOwnerAdministrationConfirmationObservation {
    Complete(std::sync::Arc<Result<IssuedSessionCredential, FirstOwnerAdministrationConfirmationFailure>>),
    Join(std::sync::Arc<tokio::task::JoinError>),
}
enum FirstOwnerConfirmationWorkerState {
    Starting,
    Running(tokio::task::JoinHandle<Result<IssuedSessionCredential, FirstOwnerAdministrationConfirmationFailure>>),
    Observed(FirstOwnerAdministrationConfirmationObservation),
}
pub(crate) struct FirstOwnerAdministrationConfirmationOperation {
    frame: std::sync::Arc<FirstOwnerConfirmationFrame>,
    verified: std::sync::atomic::AtomicBool,
    cancellation: std::sync::Arc<super::ContextOperationCancellation>,
    state: tokio::sync::Mutex<FirstOwnerConfirmationWorkerState>,
}
impl FirstOwnerAdministrationConfirmationOperation {
    pub(crate) async fn wait(&self) -> FirstOwnerAdministrationConfirmationObservation {
        match tokio::time::timeout_at(self.frame.deadline.into(),self.wait_worker()).await {
            Ok(observed)=>observed,
            Err(_elapsed)=>FirstOwnerAdministrationConfirmationObservation::Complete(
                self.frame.pending_observation.get_or_init(|| std::sync::Arc::new(Err(
                    FirstOwnerAdministrationConfirmationFailure::Operation(
                        super::SurrealStorageError::TransactionSettlementPending { operation_id:self.operation_id() }.into())))).clone()),
        }
    }
    async fn wait_worker(&self) -> FirstOwnerAdministrationConfirmationObservation {
        let mut state = self.state.lock().await;
        if let FirstOwnerConfirmationWorkerState::Running(handle) = &mut *state {
            let observed = match handle.await {
                Ok(result) => FirstOwnerAdministrationConfirmationObservation::Complete(std::sync::Arc::new(result)),
                Err(error) => FirstOwnerAdministrationConfirmationObservation::Join(std::sync::Arc::new(error)),
            };
            *state = FirstOwnerConfirmationWorkerState::Observed(observed);
        }
        match &*state {
            FirstOwnerConfirmationWorkerState::Observed(observed) => observed.clone(),
            _ => unreachable!("confirmation worker or denial is published before observation"),
        }
    }
    fn original(&self) -> Result<&std::sync::Arc<FirstOwnerConfirmationOriginal>, ResourceAuthorityError> {
        self.frame.original.as_ref().ok_or_else(current_actor_password_denied)
    }
    fn live(&self) -> Result<(), ResourceAuthorityError> {
        if std::time::Instant::now() >= self.original()?.deadline { return Err(current_actor_password_denied()); }
        Ok(())
    }
    fn same_request(&self, other: &Self) -> bool {
        let a = &self.frame.request; let b = &other.frame.request;
        a.operation_id == b.operation_id && a.first_workspace_operation_id == b.first_workspace_operation_id
            && a.parent_workspace_external_id == b.parent_workspace_external_id
            && a.actor.identity.account_id == b.actor.identity.account_id
            && a.actor.identity.principal_id == b.actor.identity.principal_id
            && a.actor.identity.access_space_id == b.actor.identity.access_space_id
            && a.actor.session_id == b.actor.session_id && a.actor.actor_kind == b.actor.actor_kind
            && a.actor.actor_id == b.actor.actor_id && a.actor.principal_kind == b.actor.principal_kind
            && a.actor.policy_version == b.actor.policy_version && a.actor.delegation_chain == b.actor.delegation_chain
            && a.actor.delegated_capabilities == b.actor.delegated_capabilities
            && a.channel_hash == b.channel_hash && a.session_token.as_str() == b.session_token.as_str()
            && self.frame.original.as_ref().zip(other.frame.original.as_ref()).is_some_and(|(x,y)| x.input.lifetime == y.input.lifetime)
    }
}
#[derive(Clone, PartialEq, Eq, SurrealValue)]
pub(crate) struct FirstOwnerConfirmationCanonical {
    origin: surrealdb::types::Object,
    principal: surrealdb::types::Object,
    administration: surrealdb::types::Object,
    parent: surrealdb::types::Object,
    enroll: surrealdb::types::Object,
    revoke: surrealdb::types::Object,
    assign: surrealdb::types::Object,
    event: surrealdb::types::Object,
    principal_grants: Vec<RecordId>,
}
#[derive(SurrealValue)]
struct FirstOwnerConfirmationMutation {
    before: surrealdb::types::Object, after: surrealdb::types::Object,
    administrator: RecordId, account: RecordId, space: RecordId,
    administration: RecordId, parent: RecordId, grants: Vec<RecordId>,
    policy: i64, expires_at: Option<surrealdb::types::Datetime>,
    created_at: surrealdb::types::Datetime,
    event: super::event_ledger::LedgerBulkInsert,
}
#[derive(Clone, SurrealValue)]
struct FirstOwnerConfirmationRead {
    administrator: RecordId, administration: RecordId, parent: RecordId,
    grants: Vec<RecordId>, event: RecordId,
}
const FIRST_OWNER_CONFIRMATION_READ: &str = r#"
RETURN {
 origin:(SELECT * FROM ONLY first_owner_administration_origins:primary),
 principal:(SELECT * OMIT authorization_touch_nonce FROM ONLY $administrator),
 administration:(SELECT * OMIT authorization_touch_nonce FROM ONLY $administration),
 parent:(SELECT * OMIT authorization_touch_nonce FROM ONLY $parent),
 enroll:(SELECT * OMIT authorization_touch_nonce FROM ONLY $grants[0]),
 revoke:(SELECT * OMIT authorization_touch_nonce FROM ONLY $grants[1]),
 assign:(SELECT * OMIT authorization_touch_nonce FROM ONLY $grants[2]),
 event:(SELECT * FROM ONLY $event),
 principal_grants:(SELECT VALUE id FROM resource_grants WHERE principal_id = $administrator ORDER BY id ASC LIMIT 4)
};
"#;
const FIRST_OWNER_CONFIRMATION_MUTATE: &str = r#"
IF record::exists($administrator) OR record::exists($grants[0]) OR record::exists($grants[1]) OR record::exists($grants[2])
 { THROW 'HSK-FIRST-OWNER-CONFIRMATION-COLLISION'; };
CREATE $administrator SET principal_key = record::id($administrator), account_id = $account,
 principal_kind = 'human_account', actor_kind = 'operator', actor_id = record::id($administrator),
 capability_profile_id = 'MemberEnrollmentAdministrator',
 delegated_capabilities = ['account.identity.enroll','account.identity.enrollment.revoke','account.identity.member_profile.assign'],
 status = 'enabled', revocation_epoch = 0, policy_version = $policy, created_at = $created_at, updated_at = $created_at;
CREATE $grants[0] SET account_id = $account, principal_id = $administrator, access_space_id = $space,
 resource_id = $administration, actions = ['create'], capability_ids = ['account.identity.enroll'],
 delegation_chain = [record::id($administrator)], status = 'active', grant_version = 1, policy_version = $policy,
 expires_at = $expires_at, revoked_at = NONE, created_at = $created_at, updated_at = $created_at;
CREATE $grants[1] SET account_id = $account, principal_id = $administrator, access_space_id = $space,
 resource_id = $administration, actions = ['update'], capability_ids = ['account.identity.enrollment.revoke'],
 delegation_chain = [record::id($administrator)], status = 'active', grant_version = 1, policy_version = $policy,
 expires_at = $expires_at, revoked_at = NONE, created_at = $created_at, updated_at = $created_at;
CREATE $grants[2] SET account_id = $account, principal_id = $administrator, access_space_id = $space,
 resource_id = $administration, actions = ['update'], capability_ids = ['account.identity.member_profile.assign'],
 delegation_chain = [record::id($administrator)], status = 'active', grant_version = 1, policy_version = $policy,
 expires_at = $expires_at, revoked_at = NONE, created_at = $created_at, updated_at = $created_at;
LET $changed = UPDATE first_owner_administration_origins:primary MERGE $after
 WHERE $this = $before AND confirmation_state = 'unconfirmed' AND confirmation_operation_id = NONE RETURN VALUE id;
IF array::len($changed) != 1 { THROW 'HSK-FIRST-OWNER-CONFIRMATION-CAS'; };
INSERT INTO kernel_event_ledger $event;
"#;

impl SurrealStorage {
    pub(crate) fn begin_first_owner_administration_confirmation(
        &self, request: FirstOwnerAdministrationConfirmationRequest,
    ) -> std::sync::Arc<FirstOwnerAdministrationConfirmationOperation> {
        let now = std::time::Instant::now();
        let deadline_choice = now.checked_add(self.config().statement_timeout());
        let deadline = deadline_choice.unwrap_or(now);
        let prepared = (|| -> Result<_, FirstOwnerAdministrationConfirmationFailure> {
            let deny = || FirstOwnerAdministrationConfirmationFailure::Operation(current_actor_password_denied());
            if deadline_choice.is_none() || request.operation_id.is_nil() || request.first_workspace_operation_id.is_nil()
                || request.parent_workspace_external_id.trim().is_empty() || request.channel_hash.is_empty()
                || request.session_token.is_empty() || request.password.len() < 12 || request.password.len() > MAX_PASSWORD_BYTES
                || request.actor.policy_version < 1 { return Err(deny()); }
            for id in [&request.actor.identity.account_id, &request.actor.identity.principal_id,
                &request.actor.identity.access_space_id, &request.actor.session_id] {
                Uuid::parse_str(id).map_err(|_| deny())?;
            }
            let lifetime = match &request.lifetime {
                FirstOwnerAdministrationConfirmationLifetime::NoExpiry => super::resource_authority::AdministrationLifetimeChoice::NoExpiry,
                FirstOwnerAdministrationConfirmationLifetime::ExpiresAt { expires_at } => {
                    if *expires_at <= chrono::Utc::now() { return Err(deny()); }
                    super::resource_authority::AdministrationLifetimeChoice::ExpiresAt(
                        handshake_member_enrollment::command_plan::Timestamp::new(expires_at.timestamp(), expires_at.timestamp_subsec_nanos())
                            .map_err(|_| deny())?)
                }
            };
            let administrator = ProvisionedIdentity {
                account_id: request.actor.identity.account_id.clone(), principal_id: Uuid::now_v7().to_string(),
                access_space_id: request.actor.identity.access_space_id.clone(),
            };
            let resource_key = Uuid::now_v7().to_string();
            let original = std::sync::Arc::new(FirstOwnerConfirmationOriginal {
                operation_id: request.operation_id, administration_resource: RecordId::new("protected_resources", resource_key.clone()),
                administrator, grants: std::array::from_fn(|_| RecordId::new("resource_grants", Uuid::now_v7().to_string())),
                input: super::resource_authority::AccountAdministrationRegistrationInput {
                    external_resource_id: resource_key, parent_workspace_external_id: request.parent_workspace_external_id.clone(),
                    classification: "account_private".to_owned(), resource_policy_version: request.actor.policy_version, lifetime,
                }, deadline, workspace_lookup_errors: std::sync::Mutex::new(Vec::new()),
            });
            let mut entropy = zeroize::Zeroizing::new([0u8; 32]);
            getrandom::getrandom(&mut entropy[..]).map_err(FirstOwnerAdministrationConfirmationFailure::Entropy)?;
            let token = zeroize::Zeroizing::new(hex::encode(entropy.as_ref()));
            let requested_expires_at = chrono::Utc::now().checked_add_signed(chrono::Duration::seconds(60)).ok_or_else(deny)?;
            let credential = FirstOwnerConfirmationCredentialIntent {
                record: RecordId::new("session_exchange_credentials", Uuid::now_v7().to_string()),
                token_hash: hex::encode(sha2::Sha256::digest(token.as_bytes())), token, requested_expires_at,
            };
            let lifetime_json = match &request.lifetime {
                FirstOwnerAdministrationConfirmationLifetime::NoExpiry => serde_json::json!({"mode":"no_expiry"}),
                FirstOwnerAdministrationConfirmationLifetime::ExpiresAt { expires_at } => serde_json::json!({"mode":"expires_at","expires_at":expires_at}),
            };
            let event = crate::kernel::NewKernelEvent::builder(
                format!("first-owner-confirmation:{}", request.operation_id), request.actor.session_id.clone(),
                crate::kernel::KernelEventType::LocalAccountSetup,
                crate::kernel::KernelActor::Operator(request.actor.identity.principal_id.clone()),
            ).aggregate("local_account", &request.actor.identity.account_id)
             .idempotency_key(format!("first-owner-confirmation:{}", request.operation_id))
             .source_component("local_account_authority")
             .payload(serde_json::json!({
                "schema_version":"hsk.local_account_event@1","phase":"confirmation",
                "operation_id":request.operation_id,"first_workspace_operation_id":request.first_workspace_operation_id,
                "account_id":request.actor.identity.account_id,"principal_id":request.actor.identity.principal_id,
                "access_space_id":request.actor.identity.access_space_id,"session_id":request.actor.session_id,
                "delegation_chain":request.actor.delegation_chain,"action":"LOCAL_ACCOUNT_SETUP","result":"allow",
                "policy_version":request.actor.policy_version,"source_origin":"first_owner_administration_origins:primary",
                "actor_scope":{"account_id":request.actor.identity.account_id,"principal_id":request.actor.identity.principal_id,
                    "session_id":request.actor.session_id,"access_space_id":request.actor.identity.access_space_id,
                    "workspace_id":request.parent_workspace_external_id},
                "administrator_scope":{"account_id":original.administrator.account_id,"principal_id":original.administrator.principal_id,
                    "session_id":serde_json::Value::Null,"access_space_id":original.administrator.access_space_id,
                    "workspace_id":request.parent_workspace_external_id},
                "administration_resource_id":record_key(original.administration_resource.clone()).map_err(|_| deny())?,
                "profile":"MemberEnrollmentAdministrator","lifetime":lifetime_json,
                "grants":[
                    {"id":record_key(original.grants[0].clone()).map_err(|_| deny())?,"version":1,"actions":["create"],"capabilities":["account.identity.enroll"]},
                    {"id":record_key(original.grants[1].clone()).map_err(|_| deny())?,"version":1,"actions":["update"],"capabilities":["account.identity.enrollment.revoke"]},
                    {"id":record_key(original.grants[2].clone()).map_err(|_| deny())?,"version":1,"actions":["update"],"capabilities":["account.identity.member_profile.assign"]}
                ],"resource_refs":[{"kind":"local_account","id":request.actor.identity.account_id},
                    {"kind":"account_administration","id":record_key(original.administration_resource.clone()).map_err(|_| deny())?}]
             })).build().map_err(|_| deny())?;
            Ok((original, credential, event))
        })();
        let (original, credential, event, rejection) = match prepared {
            Ok((original, credential, event)) => (Some(original), Some(credential), Some(event), None),
            Err(error) => (None, None, None, Some(error)),
        };
        let (admission, admission_poisoned) = match self.inner.member_login_admission.lock() {
            Ok(guard)=>(guard,false),Err(poison)=>(poison.into_inner(),true),
        };
        let (mut registry, registry_poisoned) = match self.inner.transaction_settlements.lock() {
            Ok(guard)=>(guard,false),Err(poison)=>(poison.into_inner(),true),
        };
        let retention_id = loop { let id=Uuid::now_v7(); if !registry.contains_key(&id) { break id; } };
        let frame = std::sync::Arc::new(FirstOwnerConfirmationFrame {
            request, original, credential, event, retention_id, deadline, storage: self.clone(),
            captured_origin: std::sync::OnceLock::new(), expected_origin: std::sync::OnceLock::new(),
            event_result: std::sync::OnceLock::new(), expected: std::sync::OnceLock::new(), canonical: std::sync::OnceLock::new(),
            actor_canonical: std::sync::OnceLock::new(), credential_canonical: std::sync::OnceLock::new(), replay: std::sync::OnceLock::new(),
            replay_read: std::sync::OnceLock::new(), bootstrap: std::sync::Mutex::new(None), retired: std::sync::Mutex::new(None),
            pending_observation: std::sync::OnceLock::new(),
            body_entered: std::sync::atomic::AtomicBool::new(false), read_only: std::sync::atomic::AtomicBool::new(false),
        });
        let rejection = rejection.or_else(|| {
            (admission_poisoned || registry_poisoned || !*admission).then(||
                FirstOwnerAdministrationConfirmationFailure::Operation(super::SurrealStorageError::Closed.into()))
        });
        let admitted = rejection.is_none();
        let initial_rejection = rejection.unwrap_or_else(||
            FirstOwnerAdministrationConfirmationFailure::Operation(current_actor_password_denied()));
        let owner = std::sync::Arc::new(FirstOwnerAdministrationConfirmationOperation {
            frame, verified: std::sync::atomic::AtomicBool::new(false), cancellation: super::ContextOperationCancellation::new(),
            state: tokio::sync::Mutex::new(FirstOwnerConfirmationWorkerState::Observed(
                FirstOwnerAdministrationConfirmationObservation::Complete(std::sync::Arc::new(Err(initial_rejection))))),
        });
        if admitted {
            if let Ok(mut state) = owner.state.try_lock() {
                registry.insert(owner.frame.retention_id, Some(super::AuthenticatedTransactionRetention::FirstOwnerConfirmation(owner.clone())));
                let worker = owner.clone();
                *state = FirstOwnerConfirmationWorkerState::Running(tokio::spawn(async move { worker.run().await }));
            }
        }
        drop(registry); drop(admission);
        owner
    }
}
impl FirstOwnerAdministrationConfirmationOperation {
    pub(super) fn retention_key(&self) -> Uuid { self.frame.retention_id }
    pub(super) fn operation_id(&self) -> Uuid { self.frame.request.operation_id }
    pub(super) fn original_deadline(&self) -> std::time::Instant { self.frame.deadline }
    pub(super) fn cancellation(&self) -> std::sync::Arc<super::ContextOperationCancellation> { self.cancellation.clone() }
}

pub(super) struct FirstOwnerConfirmationCredentialAuthorization<'a, 'engine> {
    storage: SurrealStorage,
    transaction: &'a super::SurrealTransactionContext<'engine>,
    password: CurrentActorPasswordReauthentication,
    checked: AdministratorPasswordVerifiedAccount,
    original: std::sync::Arc<FirstOwnerConfirmationOriginal>,
    origin: surrealdb::types::Object,
}
impl FirstOwnerConfirmationCredentialAuthorization<'_, '_> {
    pub(super) fn require_binding(
        &self, origin: &surrealdb::types::Object, account: &RecordId, principal: &RecordId, space: &RecordId,
    ) -> Result<(), ResourceAuthorityError> {
        if origin != &self.origin || *account != RecordId::new("local_accounts", self.original.administrator.account_id.clone())
            || *principal != RecordId::new("principals", self.original.administrator.principal_id.clone())
            || *space != RecordId::new("access_spaces", self.original.administrator.access_space_id.clone()) {
            return Err(current_actor_password_denied());
        }
        Ok(())
    }
    pub(super) async fn credential_snapshot(
        &self, storage: &SurrealStorage, tx: &super::SurrealTransactionContext<'_>,
    ) -> Result<AdministratorPasswordCredentialSnapshot, ResourceAuthorityError> {
        if !std::sync::Arc::ptr_eq(&storage.inner, &self.storage.inner)
            || !std::ptr::eq(tx.transaction, self.transaction.transaction)
            || self.password.operation_id != self.original.operation_id
            || self.password.enrollment_operation_id != self.original.operation_id
            || std::time::Instant::now() >= self.original.deadline { return Err(current_actor_password_denied()); }
        confirmation_require_actor_current(tx, &self.password).await?;
        let origins: Vec<surrealdb::types::Object> = tx.query_values_at(
            "SELECT VALUE $this FROM first_owner_administration_origins:primary;", (), 0,
        ).await?;
        if origins.as_slice() != std::slice::from_ref(&self.origin) { return Err(current_actor_password_denied()); }
        self.checked.credential_snapshot(storage)
    }
}
async fn confirmation_require_actor_current(
    tx: &super::SurrealTransactionContext<'_>, password: &CurrentActorPasswordReauthentication,
) -> Result<(), ResourceAuthorityError> {
    let current: Vec<CurrentActorPasswordRow> = tx.query_values_at(
        FIRST_OWNER_CURRENT_PASSWORD_READ, CurrentActorPasswordBindings {
            account: password.observed.account_id.clone(), principal: password.observed.principal_id.clone(),
            space: password.observed.access_space_id.clone(), session: password.observed.session_id.clone(),
            token_hash: password.token_hash.clone(), channel_hash: password.requested_channel_hash.clone(),
        }, 0,
    ).await?;
    if current.as_slice() != std::slice::from_ref(&password.observed) { return Err(current_actor_password_denied()); }
    Ok(())
}
fn confirmation_actor_matches(a: &LocalSessionContext, b: &LocalSessionContext) -> bool {
    a.identity.account_id == b.identity.account_id && a.identity.principal_id == b.identity.principal_id
        && a.identity.access_space_id == b.identity.access_space_id && a.session_id == b.session_id
        && a.actor_id == b.actor_id && a.actor_kind == b.actor_kind && a.principal_kind == b.principal_kind
        && a.policy_version == b.policy_version && a.delegation_chain == b.delegation_chain
        && a.delegated_capabilities == b.delegated_capabilities
}
fn confirmation_fixed_capabilities() -> [&'static str; 3] {
    ["account.identity.enroll", "account.identity.enrollment.revoke", "account.identity.member_profile.assign"]
}
fn confirmation_field<T: SurrealValue>(object: &surrealdb::types::Object, key: &str, value: T) -> bool {
    let value = value.into_value();
    object.get(key) == Some(&value) || (matches!(value,surrealdb::types::Value::None) && object.get(key).is_none())
}
fn confirmation_lifetime_value(
    choice: super::resource_authority::AdministrationLifetimeChoice,
) -> Result<(surrealdb::types::Object, Option<surrealdb::types::Datetime>), ResourceAuthorityError> {
    let mut object = surrealdb::types::Object::new();
    match choice {
        super::resource_authority::AdministrationLifetimeChoice::NoExpiry => {
            object.insert("mode", "no_expiry"); Ok((object, None))
        },
        super::resource_authority::AdministrationLifetimeChoice::ExpiresAt(timestamp) => {
            let datetime = chrono::DateTime::<chrono::Utc>::from_timestamp(timestamp.unix_seconds(), timestamp.nanoseconds())
                .ok_or_else(current_actor_password_denied)?;
            if datetime <= chrono::Utc::now() { return Err(current_actor_password_denied()); }
            let datetime = surrealdb::types::Datetime::from(datetime);
            object.insert("mode", "expires_at"); object.insert("expires_at", datetime.clone());
            Ok((object, Some(datetime)))
        }
    }
}
impl FirstOwnerConfirmationCanonical {
    fn require_expected(
        &self, owner: &FirstOwnerAdministrationConfirmationOperation,
        expected_origin: &surrealdb::types::Object, event: &super::event_ledger::LedgerWrite,
    ) -> Result<(), ResourceAuthorityError> {
        let original = owner.original()?;
        let account = RecordId::new("local_accounts", original.administrator.account_id.clone());
        let principal = RecordId::new("principals", original.administrator.principal_id.clone());
        let space = RecordId::new("access_spaces", original.administrator.access_space_id.clone());
        let capabilities: Vec<String> = confirmation_fixed_capabilities().iter().map(|x| (*x).to_owned()).collect();
        let (lifetime, expiry) = confirmation_lifetime_value(original.input.lifetime)?;
        if &self.origin != expected_origin
            || !confirmation_field(&self.principal, "id", principal.clone())
            || !confirmation_field(&self.principal, "account_id", account.clone())
            || !confirmation_field(&self.principal, "principal_key", original.administrator.principal_id.clone())
            || !confirmation_field(&self.principal, "principal_kind", "human_account".to_owned())
            || !confirmation_field(&self.principal, "actor_kind", "operator".to_owned())
            || !confirmation_field(&self.principal, "actor_id", original.administrator.principal_id.clone())
            || !confirmation_field(&self.principal, "capability_profile_id", "MemberEnrollmentAdministrator".to_owned())
            || !confirmation_field(&self.principal, "delegated_capabilities", capabilities)
            || !confirmation_field(&self.principal, "status", "enabled".to_owned())
            || !confirmation_field(&self.principal, "revocation_epoch", 0i64)
            || !confirmation_field(&self.principal, "policy_version", original.input.resource_policy_version)
            || !confirmation_field(&self.administration, "id", original.administration_resource.clone())
            || !confirmation_field(&self.administration, "resource_kind", "account_administration".to_owned())
            || !confirmation_field(&self.administration, "owner_account_id", account.clone())
            || !confirmation_field(&self.administration, "access_space_id", space.clone())
            || !confirmation_field(&self.administration, "lifecycle_state", "active".to_owned())
            || !confirmation_field(&self.administration, "account_administration_lifetime", lifetime)
            || !confirmation_field(&self.parent, "owner_account_id", account.clone())
            || !confirmation_field(&self.parent, "access_space_id", space.clone())
            || !confirmation_field(&self.parent, "resource_kind", "workspace".to_owned())
            || !confirmation_field(&self.parent, "external_resource_id", original.input.parent_workspace_external_id.clone())
            || !confirmation_field(&self.parent, "lifecycle_state", "active".to_owned()) {
            return Err(current_actor_password_denied());
        }
        let mut actual_grants = self.principal_grants.clone();
        let mut expected_grants = original.grants.to_vec();
        actual_grants.sort_by_key(|id| surrealdb::types::ToSql::to_sql(id));
        expected_grants.sort_by_key(|id| surrealdb::types::ToSql::to_sql(id));
        if actual_grants != expected_grants { return Err(current_actor_password_denied()); }
        for (index, grant) in [&self.enroll, &self.revoke, &self.assign].iter().enumerate() {
            let action = if index == 0 { "create" } else { "update" };
            if !confirmation_field(grant, "id", original.grants[index].clone())
                || !confirmation_field(grant, "account_id", account.clone())
                || !confirmation_field(grant, "principal_id", principal.clone())
                || !confirmation_field(grant, "access_space_id", space.clone())
                || !confirmation_field(grant, "resource_id", original.administration_resource.clone())
                || !confirmation_field(grant, "actions", vec![action.to_owned()])
                || !confirmation_field(grant, "capability_ids", vec![confirmation_fixed_capabilities()[index].to_owned()])
                || !confirmation_field(grant, "delegation_chain", vec![original.administrator.principal_id.clone()])
                || !confirmation_field(grant, "status", "active".to_owned())
                || !confirmation_field(grant, "grant_version", 1i64)
                || !confirmation_field(grant, "policy_version", original.input.resource_policy_version)
                || !confirmation_field(grant, "expires_at", expiry.clone())
                || !confirmation_field(grant, "revoked_at", Option::<surrealdb::types::Datetime>::None) {
                return Err(current_actor_password_denied());
            }
        }
        let surrealdb::types::Value::Object(mut expected_event) = event.clone().into_value() else { return Err(current_actor_password_denied()); };
        expected_event.remove("record"); expected_event.insert("id", event.record.clone());
        let mut actual_event = self.event.clone();
        actual_event.remove("event_sequence");
        for key in ["causation_id","correlation_id","authority_resource_id","authority_session_id","authority_capability_id","authority_action"] {
            if matches!(expected_event.get(key),Some(surrealdb::types::Value::None)) && actual_event.get(key).is_none() { expected_event.remove(key); }
        }
        if actual_event != expected_event { return Err(current_actor_password_denied()); }
        Ok(())
    }
}

type FirstOwnerConfirmationBootstrapResult = Result<
    super::first_owner_bootstrap_transaction::FirstOwnerBootstrapOutcome<FirstOwnerConfirmationPrepared, IssuedSessionCredential, ResourceAuthorityError>,
    super::SurrealStorageError>;
enum FirstOwnerConfirmationSdkSuccess {
    Observed,
}
enum FirstOwnerConfirmationRetiredTerminal {
    BodyRejected {
        operation: ResourceAuthorityError,
        cancellation: FirstOwnerConfirmationSdkSuccess,
    },
    CancelledBeforeCommit {
        operation: FirstOwnerConfirmationPrepared,
        cancellation: FirstOwnerConfirmationSdkSuccess,
    },
    Committed {
        operation: std::sync::Arc<FirstOwnerConfirmationPrepared>,
        commit: FirstOwnerConfirmationSdkSuccess,
        canonical: Result<IssuedSessionCredential,ResourceAuthorityError>,
    },
    RejectedBeforeBody(super::SurrealStorageError),
}
struct FirstOwnerConfirmationPrepared {
    password: CurrentActorPasswordReauthentication,
    canonical: FirstOwnerConfirmationCanonical,
    read: FirstOwnerConfirmationRead,
    credential: super::resource_authority::FirstOwnerConfirmationPreparedCredential,
}
struct FirstOwnerConfirmationReplayRead {
    actor: Vec<CurrentActorPasswordRow>,
    canonical: Vec<FirstOwnerConfirmationCanonical>,
    credential: surrealdb::types::Value,
}
impl FirstOwnerAdministrationConfirmationOperation {
    fn winner(&self) -> Result<Option<std::sync::Arc<Self>>, ResourceAuthorityError> {
        let registry = self.frame.storage.inner.transaction_settlements.lock().map_err(|_| current_actor_password_denied())?;
        Ok(registry.values().filter_map(|entry| match entry {
            Some(super::AuthenticatedTransactionRetention::FirstOwnerConfirmation(owner))
                if owner.operation_id() == self.operation_id() && owner.verified.load(std::sync::atomic::Ordering::Acquire)
                    && owner.retention_key() != self.retention_key() => Some(owner.clone()),
            _ => None,
        }).next())
    }
    fn promote(self: &std::sync::Arc<Self>) -> Result<(), ResourceAuthorityError> {
        let _admission = self.frame.storage.inner.member_login_admission.lock().map_err(|_| current_actor_password_denied())?;
        let registry = self.frame.storage.inner.transaction_settlements.lock().map_err(|_| current_actor_password_denied())?;
        if !matches!(registry.get(&self.retention_key()),
            Some(Some(super::AuthenticatedTransactionRetention::FirstOwnerConfirmation(owner))) if std::sync::Arc::ptr_eq(owner,self)) {
            return Err(current_actor_password_denied());
        }
        if registry.values().any(|entry| matches!(entry,
            Some(super::AuthenticatedTransactionRetention::FirstOwnerConfirmation(owner))
                if owner.operation_id()==self.operation_id() && owner.retention_key()!=self.retention_key()
                    && owner.verified.load(std::sync::atomic::Ordering::Acquire))) {
            return Err(current_actor_password_denied());
        }
        self.verified.store(true,std::sync::atomic::Ordering::Release);
        Ok(())
    }
    async fn run(self: std::sync::Arc<Self>) -> Result<IssuedSessionCredential, FirstOwnerAdministrationConfirmationFailure> {
        let deadline=self.frame.deadline;
        SurrealStorage::with_operation_deadline(deadline,self.run_scoped()).await
    }
    async fn run_scoped(self: std::sync::Arc<Self>) -> Result<IssuedSessionCredential, FirstOwnerAdministrationConfirmationFailure> {
        if let Some(winner) = self.winner().map_err(FirstOwnerAdministrationConfirmationFailure::Operation)? {
            return self.reobserve(winner).await;
        }
        let event = self.frame.event.as_ref().ok_or_else(|| FirstOwnerAdministrationConfirmationFailure::Operation(current_actor_password_denied()))?;
        let prepared_event = std::sync::Arc::new(super::event_ledger::prepare_event(event.clone()).map(|(_,mut write)| {
            write.wsids = vec![self.frame.request.parent_workspace_external_id.clone()];
            write.authority_resource_id = self.frame.original.as_ref().map(|x| x.administration_resource.clone());
            write.authority_session_id = Some(RecordId::new("authenticated_sessions",self.frame.request.actor.session_id.clone()));
            write
        }));
        let _ = self.frame.event_result.set(prepared_event.clone());
        if prepared_event.is_err() { return Err(FirstOwnerAdministrationConfirmationFailure::Audit(prepared_event)); }
        let body_owner = self.clone();
        let canonical_owner = self.clone();
        let result = self.frame.storage.with_first_owner_administration_bootstrap_transaction(
            self.clone(),
            move |tx,birth| Box::pin(async move { body_owner.body(tx,birth).await }),
            move |client,prepared| Box::pin(async move { canonical_owner.read_committed(client,prepared).await }),
        ).await;
        let result = std::sync::Arc::new(result);
        *self.frame.bootstrap.lock().map_err(|_|FirstOwnerAdministrationConfirmationFailure::Operation(current_actor_password_denied()))? = Some(result.clone());
        if !self.verified.load(std::sync::atomic::Ordering::Acquire)
            && matches!(result.as_ref(),Ok(super::first_owner_bootstrap_transaction::FirstOwnerBootstrapOutcome::BodyRejected { cancellation:Ok(_),.. })) {
            if let Some(winner)=self.winner().map_err(FirstOwnerAdministrationConfirmationFailure::Operation)? { return self.reobserve(winner).await; }
        }
        match result.as_ref() {
            Ok(super::first_owner_bootstrap_transaction::FirstOwnerBootstrapOutcome::Committed { canonical:Ok(credential),.. }) => Ok(credential.clone()),
            _ => Err(FirstOwnerAdministrationConfirmationFailure::Bootstrap),
        }
    }
    async fn body<'tx>(
        self: std::sync::Arc<Self>, tx: &'tx super::SurrealTransactionContext<'tx>,
        birth: super::first_owner_bootstrap_transaction::SameStorageBootstrapTransaction<'tx>,
    ) -> Result<FirstOwnerConfirmationPrepared,ResourceAuthorityError> {
        self.frame.body_entered.store(true,std::sync::atomic::Ordering::Release);
        self.live()?;
        let original = self.original()?.clone();
        let proof = self.frame.storage.capture_first_owner_registration_in_transaction(tx,birth,original.clone(),
            self.frame.request.session_token.as_str(),&self.frame.request.channel_hash,
            self.frame.request.password.to_string(),self.frame.request.first_workspace_operation_id).await?;
        if !confirmation_actor_matches(&proof.session,&self.frame.request.actor)
            || proof.password.observed.principal_capabilities.iter().chain(proof.password.observed.session_capabilities.iter())
                .any(|cap| cap.contains('*') || confirmation_fixed_capabilities().contains(&cap.as_str())) {
            return Err(current_actor_password_denied());
        }
        let _ = self.frame.captured_origin.set(proof.origin.clone());
        self.promote()?;
        let authority = super::resource_authority::CapturedAccountAdministrationRegistrationAuthority::from_confirmation(
            ConfirmedFirstOwnerAdministrationRegistration { proof:proof.clone() },original.input.clone())?;
        self.frame.storage.register_account_administration_in_transaction(tx,&authority,
            &original.administration_resource,original.input.clone()).await?;
        proof.require_current_in_transaction(tx).await?;
        let parent = proof.workspace.parent_workspace().id.clone();
        let before = proof.origin.clone();
        let (lifetime,expiry) = confirmation_lifetime_value(original.input.lifetime)?;
        let event = self.frame.event_result.get().and_then(|r| r.as_ref().as_ref().ok()).ok_or_else(current_actor_password_denied)?.clone();
        let mut after = before.clone();
        after.insert("confirmation_state","confirmed");
        after.insert("confirmation_operation_id",original.operation_id.to_string());
        after.insert("confirmation_actor_session",RecordId::new("authenticated_sessions",proof.session.session_id.clone()));
        after.insert("confirmation_channel_hash",self.frame.request.channel_hash.clone());
        after.insert("confirmation_policy_version",proof.session.policy_version);
        after.insert("administrator_principal",RecordId::new("principals",original.administrator.principal_id.clone()));
        after.insert("administration_resource",original.administration_resource.clone());
        after.insert("enroll_grant",original.grants[0].clone()); after.insert("revoke_grant",original.grants[1].clone());
        after.insert("assign_grant",original.grants[2].clone()); after.insert("administration_lifetime",lifetime);
        after.insert("confirmation_receipt",event.record.clone());
        let _ = self.frame.expected_origin.set(after.clone());
        tx.transaction.query(FIRST_OWNER_CONFIRMATION_MUTATE).bind(FirstOwnerConfirmationMutation {
            before,after:after.clone(),administrator:RecordId::new("principals",original.administrator.principal_id.clone()),
            account:RecordId::new("local_accounts",original.administrator.account_id.clone()),
            space:RecordId::new("access_spaces",original.administrator.access_space_id.clone()),
            administration:original.administration_resource.clone(),parent:parent.clone(),grants:original.grants.to_vec(),
            policy:original.input.resource_policy_version,expires_at:expiry,created_at:event.created_at.clone(),
            event:super::event_ledger::LedgerBulkInsert::from(event.clone()),
        }).await.map_err(super::SurrealStorageError::from)?.check().map_err(super::SurrealStorageError::from)?;
        drop(authority);
        let FirstOwnerConfirmationAuthorization { password, birth, transaction, workspace, .. } =
            std::sync::Arc::try_unwrap(proof).map_err(|_| current_actor_password_denied())?;
        drop(birth); let _ = transaction; drop(workspace);
        confirmation_require_actor_current(tx,&password).await?;
        #[derive(SurrealValue)] struct AccountBinding { account:RecordId }
        let mut rows:Vec<PasswordLoginAccountRow> = tx.query_values_at(
            "SELECT id AS account_id, account_key AS account_name, account_role, status, password_verifier, credential_revision, revocation_epoch, policy_version FROM local_accounts WHERE id=$account LIMIT 2;",
            AccountBinding { account:password.observed.account_id.clone() },0).await?;
        if rows.len()!=1 { return Err(current_actor_password_denied()); }
        let row = rows.pop().ok_or_else(current_actor_password_denied)?;
        if row.password_verifier!=password.observed.password_verifier || row.credential_revision!=password.observed.credential_revision
            || row.revocation_epoch!=password.observed.account_epoch || row.policy_version!=password.observed.account_policy {
            return Err(current_actor_password_denied());
        }
        let authorization = FirstOwnerConfirmationCredentialAuthorization {
            storage:self.frame.storage.clone(),transaction:tx,password,
            checked:AdministratorPasswordVerifiedAccount { row,storage:self.frame.storage.clone() },
            original:original.clone(),origin:after.clone(),
        };
        let intent = self.frame.credential.as_ref().ok_or_else(current_actor_password_denied)?;
        let credential = self.frame.storage.issue_first_owner_confirmation_credential_in_transaction(tx,&authorization,intent).await?;
        authorization.credential_snapshot(&self.frame.storage,tx).await?;
        let read = FirstOwnerConfirmationRead {
            administrator:RecordId::new("principals",original.administrator.principal_id.clone()),
            administration:original.administration_resource.clone(),parent,grants:original.grants.to_vec(),event:event.record.clone(),
        };
        let mut canonical_rows:Vec<FirstOwnerConfirmationCanonical> = tx.query_values_at(FIRST_OWNER_CONFIRMATION_READ,read.clone(),0).await?;
        if canonical_rows.len()!=1 { return Err(current_actor_password_denied()); }
        let canonical=canonical_rows.pop().ok_or_else(current_actor_password_denied)?;
        canonical.require_expected(&self,&after,&event)?;
        let _ = self.frame.expected.set(canonical.clone());
        self.live()?;
        let FirstOwnerConfirmationCredentialAuthorization { password,.. } = authorization;
        Ok(FirstOwnerConfirmationPrepared { password,canonical,read,credential })
    }
    async fn read_committed(
        self:std::sync::Arc<Self>,client:&super::SurrealClient,prepared:std::sync::Arc<FirstOwnerConfirmationPrepared>,
    ) -> Result<IssuedSessionCredential,ResourceAuthorityError> {
        let canonical = std::sync::Arc::new(async {
            let mut response=client.query(FIRST_OWNER_CONFIRMATION_READ).bind(prepared.read.clone()).await?.check()?;
            let value:Vec<FirstOwnerConfirmationCanonical>=response.take(0)?;
            Ok::<_,super::SurrealStorageError>(value)
        }.await);
        let _=self.frame.canonical.set(canonical.clone());
        let current=canonical.as_ref().as_ref().map_err(|_|current_actor_password_denied())?;
        if current.as_slice()!=std::slice::from_ref(&prepared.canonical) { return Err(current_actor_password_denied()); }
        let credential=std::sync::Arc::new(prepared.credential.canonical_read(client).await);
        let _=self.frame.credential_canonical.set(credential.clone());
        if !matches!(credential.as_ref(),Ok(surrealdb::types::Value::Bool(true))) { return Err(current_actor_password_denied()); }
        let actor=std::sync::Arc::new(async {
            let mut response=client.query(FIRST_OWNER_CURRENT_PASSWORD_READ).bind(CurrentActorPasswordBindings {
                account:prepared.password.observed.account_id.clone(),principal:prepared.password.observed.principal_id.clone(),
                space:prepared.password.observed.access_space_id.clone(),session:prepared.password.observed.session_id.clone(),
                token_hash:prepared.password.token_hash.clone(),channel_hash:prepared.password.requested_channel_hash.clone(),
            }).await?.check()?;
            let rows:Vec<CurrentActorPasswordRow>=response.take(0)?;
            Ok::<_,super::SurrealStorageError>(rows)
        }.await);
        let _=self.frame.actor_canonical.set(actor.clone());
        if !matches!(actor.as_ref(),Ok(rows) if rows.as_slice()==std::slice::from_ref(&prepared.password.observed)) { return Err(current_actor_password_denied()); }
        self.live()?;
        prepared.credential.outward(self.frame.credential.as_ref().ok_or_else(current_actor_password_denied)?)
    }
}

impl FirstOwnerAdministrationConfirmationOperation {
    async fn reobserve(self:std::sync::Arc<Self>,winner:std::sync::Arc<Self>)
        -> Result<IssuedSessionCredential,FirstOwnerAdministrationConfirmationFailure> {
        let deadline=self.frame.deadline.min(winner.original_deadline());
        SurrealStorage::with_operation_deadline(deadline,self.reobserve_scoped(winner)).await
    }
    async fn reobserve_scoped(self:std::sync::Arc<Self>,winner:std::sync::Arc<Self>)
        -> Result<IssuedSessionCredential,FirstOwnerAdministrationConfirmationFailure> {
        self.frame.read_only.store(true,std::sync::atomic::Ordering::Release);
        let deny=||FirstOwnerAdministrationConfirmationFailure::Operation(current_actor_password_denied());
        if !self.same_request(&winner) { return Err(deny()); }
        self.live().map_err(FirstOwnerAdministrationConfirmationFailure::Operation)?;
        winner.live().map_err(FirstOwnerAdministrationConfirmationFailure::Operation)?;
        let proof=self.frame.storage.reauthenticate_current_actor_password(self.operation_id(),self.operation_id(),
            self.frame.request.session_token.as_str(),&self.frame.request.channel_hash,self.frame.request.password.to_string())
            .await.map_err(FirstOwnerAdministrationConfirmationFailure::Operation)?;
        let session=self.frame.storage.authenticate_local_session(self.frame.request.session_token.as_str(),&self.frame.request.channel_hash)
            .await.map_err(FirstOwnerAdministrationConfirmationFailure::Operation)?;
        proof.observed.require_authenticated_actor(&session).map_err(FirstOwnerAdministrationConfirmationFailure::Operation)?;
        if !confirmation_actor_matches(&session,&self.frame.request.actor) { return Err(deny()); }
        let observation=match tokio::time::timeout_at(self.frame.deadline.min(winner.original_deadline()).into(),winner.wait_worker()).await {
            Ok(observed)=>observed,
            Err(_elapsed)=>return Err(FirstOwnerAdministrationConfirmationFailure::Operation(
                super::SurrealStorageError::TransactionSettlementPending { operation_id:winner.operation_id() }.into())),
        };
        let _=self.frame.replay.set(observation.clone());
        let credential=match observation {
            FirstOwnerAdministrationConfirmationObservation::Complete(result)=>result.as_ref().as_ref().map_err(|_|deny())?.clone(),
            FirstOwnerAdministrationConfirmationObservation::Join(_)=>return Err(FirstOwnerAdministrationConfirmationFailure::Replay),
        };
        let prepared={
            let raw=winner.frame.bootstrap.lock().map_err(|_|deny())?;
            match raw.as_ref().map(|r|r.as_ref()) {
                Some(Ok(super::first_owner_bootstrap_transaction::FirstOwnerBootstrapOutcome::Committed { operation,canonical:Ok(_),.. }))=>operation.clone(),
                _=>return Err(deny()),
            }
        };
        if proof.observed!=prepared.password.observed || proof.token_hash!=prepared.password.token_hash
            || proof.requested_channel_hash!=prepared.password.requested_channel_hash { return Err(deny()); }
        let expected=prepared.clone();
        let account=proof.observed.account_id.clone(); let principal=proof.observed.principal_id.clone();
        let space=proof.observed.access_space_id.clone(); let session_id=proof.observed.session_id.clone();
        let token_hash=proof.token_hash.clone(); let channel_hash=proof.requested_channel_hash.clone();
        let namespace=self.frame.storage.config().namespace().to_owned();
        let database=self.frame.storage.config().database().to_owned();
        let snapshot=std::sync::Arc::new(self.frame.storage.with_lease(move |client|Box::pin(async move {
            let authority=client.clone(); authority.use_ns(namespace).use_db(database).await?;
            let mut current=authority.query(FIRST_OWNER_CURRENT_PASSWORD_READ).bind(CurrentActorPasswordBindings {
                account,principal,space,session:session_id,token_hash,channel_hash,
            }).await?.check()?;
            let actor:Vec<CurrentActorPasswordRow>=current.take(0)?;
            let mut response=authority.query(FIRST_OWNER_CONFIRMATION_READ).bind(expected.read.clone()).await?.check()?;
            let canonical:Vec<FirstOwnerConfirmationCanonical>=response.take(0)?;
            let credential=expected.credential.canonical_read(&authority).await?;
            Ok(FirstOwnerConfirmationReplayRead { actor,canonical,credential })
        })).await);
        let _=self.frame.replay_read.set(snapshot.clone());
        let current=snapshot.as_ref().as_ref().map_err(|_|deny())?;
        if current.actor.as_slice()!=std::slice::from_ref(&proof.observed)
            || current.canonical.as_slice()!=std::slice::from_ref(&prepared.canonical) || !matches!(&current.credential,surrealdb::types::Value::Bool(true)) {
            return Err(deny());
        }
        self.live().map_err(FirstOwnerAdministrationConfirmationFailure::Operation)?;
        winner.live().map_err(FirstOwnerAdministrationConfirmationFailure::Operation)?;
        if credential.expires_at<=chrono::Utc::now() { return Err(deny()); }
        Ok(credential)
    }
    async fn drain(&self)->Result<(),ResourceAuthorityError> {
        let observed=match tokio::time::timeout_at(self.frame.deadline.into(),self.wait_worker()).await {
            Ok(observed)=>observed,
            Err(_elapsed)=>return Err(super::SurrealStorageError::TransactionSettlementPending { operation_id:self.operation_id() }.into()),
        };
        if matches!(observed,FirstOwnerAdministrationConfirmationObservation::Join(_)) {
            return Err(super::SurrealStorageError::TransactionSettlementPending { operation_id:self.operation_id() }.into());
        }
        let terminal={
            let raw=self.frame.bootstrap.lock().map_err(|_|current_actor_password_denied())?;
            match raw.as_ref().map(|x|x.as_ref()) {
                Some(Ok(super::first_owner_bootstrap_transaction::FirstOwnerBootstrapOutcome::BodyRejected { cancellation:Ok(_),.. }))
                |Some(Ok(super::first_owner_bootstrap_transaction::FirstOwnerBootstrapOutcome::CancelledBeforeCommit { cancellation:Ok(_),.. }))
                |Some(Ok(super::first_owner_bootstrap_transaction::FirstOwnerBootstrapOutcome::Committed { canonical:Ok(_),.. }))=>true,
                Some(Err(_))=>!self.frame.body_entered.load(std::sync::atomic::Ordering::Acquire),
                None=>self.frame.retired.lock().map_err(|_|current_actor_password_denied())?.is_some()
                    || self.frame.read_only.load(std::sync::atomic::Ordering::Acquire)
                    || !self.frame.body_entered.load(std::sync::atomic::Ordering::Acquire),
                _=>false,
            }
        };
        if !terminal { return Err(super::SurrealStorageError::TransactionSettlementPending { operation_id:self.operation_id() }.into()); }
        {
            let mut raw=self.frame.bootstrap.lock().map_err(|_|current_actor_password_denied())?;
            let mut retired=self.frame.retired.lock().map_err(|_|current_actor_password_denied())?;
            if let Some(held)=raw.take() {
                if retired.is_some() {
                    *raw=Some(held);
                    return Err(super::SurrealStorageError::TransactionSettlementPending { operation_id:self.operation_id() }.into());
                }
                let outcome=match std::sync::Arc::try_unwrap(held) {
                    Ok(outcome)=>outcome,
                    Err(shared)=>{
                        *raw=Some(shared);
                        return Err(super::SurrealStorageError::TransactionSettlementPending { operation_id:self.operation_id() }.into());
                    }
                };
                match outcome {
                    Ok(super::first_owner_bootstrap_transaction::FirstOwnerBootstrapOutcome::BodyRejected { operation,cancellation:Ok(client) })=>{
                        drop(client);
                        *retired=Some(FirstOwnerConfirmationRetiredTerminal::BodyRejected {
                            operation,cancellation:FirstOwnerConfirmationSdkSuccess::Observed,
                        });
                    },
                    Ok(super::first_owner_bootstrap_transaction::FirstOwnerBootstrapOutcome::CancelledBeforeCommit { operation,cancellation:Ok(client) })=>{
                        drop(client);
                        *retired=Some(FirstOwnerConfirmationRetiredTerminal::CancelledBeforeCommit {
                            operation,cancellation:FirstOwnerConfirmationSdkSuccess::Observed,
                        });
                    },
                    Ok(super::first_owner_bootstrap_transaction::FirstOwnerBootstrapOutcome::Committed { operation,commit,canonical }) if canonical.is_ok()=>{
                        drop(commit);
                        *retired=Some(FirstOwnerConfirmationRetiredTerminal::Committed {
                            operation,commit:FirstOwnerConfirmationSdkSuccess::Observed,canonical,
                        });
                    },
                    Err(error) if !self.frame.body_entered.load(std::sync::atomic::Ordering::Acquire)=>{
                        *retired=Some(FirstOwnerConfirmationRetiredTerminal::RejectedBeforeBody(error));
                    },
                    unresolved=>{
                        *raw=Some(std::sync::Arc::new(unresolved));
                        return Err(super::SurrealStorageError::TransactionSettlementPending { operation_id:self.operation_id() }.into());
                    },
                }
            }
        }
        let mut registry=self.frame.storage.inner.transaction_settlements.lock().map_err(|_|current_actor_password_denied())?;
        if matches!(registry.get(&self.retention_key()),Some(Some(super::AuthenticatedTransactionRetention::FirstOwnerConfirmation(owner)))
            if std::ptr::eq(owner.as_ref(),self)) { registry.remove(&self.retention_key()); }
        Ok(())
    }
}
impl SurrealStorage {
    pub(crate) async fn drain_first_owner_administration_confirmation_operations(&self)->Result<(),ResourceAuthorityError> {
        self.close_member_login_admission()?;
        let owners:Vec<_>={
            let registry=self.inner.transaction_settlements.lock().map_err(|_|current_actor_password_denied())?;
            registry.values().filter_map(|entry|match entry {
                Some(super::AuthenticatedTransactionRetention::FirstOwnerConfirmation(owner))=>Some(owner.clone()),_=>None,
            }).collect()
        };
        let mut first=None;
        for owner in owners { if let Err(error)=owner.drain().await { if first.is_none() { first=Some(error); } } }
        first.map_or(Ok(()),Err)
    }
}
