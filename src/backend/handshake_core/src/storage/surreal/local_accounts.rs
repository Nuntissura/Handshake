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
    policy_version: i64,
    delegation_chain: Vec<String>,
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

pub struct LocalSessionContext {
    pub identity: ProvisionedIdentity,
    pub session_id: String,
    pub actor_id: String,
    pub policy_version: i64,
    pub delegation_chain: Vec<String>,
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
            broker.query("BEGIN TRANSACTION; IF $account_session.revoked_at != NONE OR $account_session.expires_at <= time::now() { THROW 'HSK-403-PROTECTED-RESOURCE'; }; UPDATE $account_session SET revoked_at = time::now(); INSERT INTO kernel_event_ledger $event; COMMIT TRANSACTION;")
                .bind(("account_session", account_session)).bind(("event", event)).await?.check()?;
            Ok(())
        })).await?;
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
                   AND array::len((SELECT id FROM protected_resources WHERE resource_kind = 'loom_block' \
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
            let mut response = ordinary.query("SELECT account_id, principal_id, access_space_id, id AS session_id, principal_id.actor_id AS actor_id, policy_version, delegation_chain FROM authenticated_sessions WHERE id = $auth.id LIMIT 1;").await?.check()?;
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
            policy_version: row.policy_version,
            delegation_chain: row.delegation_chain,
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
        &self,
        account_name: &str,
        verifier: String,
        capabilities: Vec<String>,
    ) -> Result<ProvisionedIdentity, ResourceAuthorityError> {
        let name = account_name.trim();
        if name.is_empty()
            || name.len() > 128
            || capabilities.is_empty()
            || capabilities.iter().any(|value| value.contains('*'))
        {
            return Err(ResourceAuthorityError::InvalidInput(
                "invalid local account setup",
            ));
        }
        let account_name = name.to_owned();
        let identity = ProvisionedIdentity {
            account_id: Uuid::now_v7().to_string(),
            principal_id: Uuid::now_v7().to_string(),
            access_space_id: Uuid::now_v7().to_string(),
        };
        let account = RecordId::new("local_accounts", identity.account_id.clone());
        let principal = RecordId::new("principals", identity.principal_id.clone());
        let space = RecordId::new("access_spaces", identity.access_space_id.clone());
        let actor_id = identity.principal_id.clone();
        let event = account_event(
            crate::kernel::KernelEventType::LocalAccountSetup,
            &identity,
            None,
            1,
            vec![identity.principal_id.clone()],
        )?;
        let (_, write) = super::event_ledger::prepare_event(event)
            .map_err(|_| ResourceAuthorityError::InvalidInput("account audit event invalid"))?;
        let event = super::event_ledger::LedgerBulkInsert::from(write);
        let namespace = self.config().namespace().to_owned();
        let database = self.config().database().to_owned();
        self.with_lease(move |client| Box::pin(async move {
            let admin = client.clone();
            admin.use_ns(namespace).use_db(database).await?;
            admin.query("BEGIN TRANSACTION;
                IF record::exists(local_account_setup:primary) OR array::len(SELECT VALUE id FROM local_accounts WHERE account_role = 'Owner' LIMIT 1) != 0 { THROW 'HSK-AUTH-SETUP-CLOSED'; };
                CREATE local_account_setup:primary SET account_id = $account, principal_id = $principal, access_space_id = $space, created_at = time::now();
                CREATE $account SET account_key = $name, account_role = 'Owner', status = 'enabled', password_verifier = $verifier, revocation_epoch = 0, policy_version = 1, created_at = time::now(), updated_at = time::now();
                CREATE $principal SET principal_key = $actor, account_id = $account, principal_kind = 'human_account', actor_kind = 'operator', actor_id = $actor, capability_profile_id = 'Operator', delegated_capabilities = $capabilities, status = 'enabled', revocation_epoch = 0, policy_version = 1, created_at = time::now(), updated_at = time::now();
                CREATE $space SET space_key = $actor, account_id = $account, name = 'Private', status = 'active', revocation_epoch = 0, policy_version = 1, created_at = time::now(), updated_at = time::now();
                INSERT INTO kernel_event_ledger $account_event;
                COMMIT TRANSACTION;")
                .bind(("account", account)).bind(("principal", principal)).bind(("space", space))
                .bind(("name", account_name)).bind(("verifier", verifier))
                .bind(("actor", actor_id)).bind(("capabilities", capabilities)).bind(("account_event", event)).await?.check()?;
            Ok(())
        })).await?;
        Ok(identity)
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
