use std::sync::Arc;

use serde_json::Value;
use sha2::{Digest, Sha256};
use surrealdb::types::SurrealValue;
use surrealdb::types::ToSql;
use thiserror::Error;
use tokio::sync::OnceCell;
use uuid::Uuid;

use super::{SurrealStorage, SurrealStorageError};
use crate::{
    kernel::NewKernelEvent, swarm_orchestration::resource_scope::ExactResourceScopeAttribution,
};

const SCHEMA: &str = include_str!("palmistry_schema.surql");
pub(super) const SCHEMA_STATE: &str = "\
DEFINE TABLE IF NOT EXISTS palmistry_schema_state SCHEMAFULL;\
DEFINE FIELD IF NOT EXISTS schema_version ON palmistry_schema_state TYPE string;\
DEFINE FIELD IF NOT EXISTS schema_revision ON palmistry_schema_state TYPE int;\
DEFINE FIELD IF NOT EXISTS apply_state ON palmistry_schema_state TYPE string;";
const SCHEMA_VERSION: &str = "mt003-palmistry-authority-v1";
const SCHEMA_REVISION: i64 = 1;

#[derive(Debug, Error)]
pub enum SurrealPalmistryError {
    #[error("embedded Palmistry storage failed: {0}")]
    Storage(#[from] SurrealStorageError),
    #[error("Palmistry durable authority input is invalid")]
    InvalidInput,
    #[error("Palmistry durable verifier identity conflicts with an existing row")]
    IdentityConflict,
    #[error("Palmistry durable authority is ambiguous or corrupt")]
    CorruptAuthority,
    #[error("Palmistry source retirement requires durable STOP")]
    StopRequired,
    #[error("authenticated Palmistry receipt permission denied; no canonical receipt was produced")]
    PermissionDenied,
    #[error("native watcher recheck failed: {0}")]
    NativeRecheck(String),
    #[error(transparent)]
    Authority(#[from] super::resource_authority::ResourceAuthorityError),
    #[error("watcher adoption operation {operation_id} callback failed: {error}")]
    OriginalCallback { operation_id: Uuid, #[source] error: Box<SurrealPalmistryError> },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurrealPalmistryVerifier {
    pub session_id: Uuid,
    pub launch_nonce: Uuid,
    pub parent_pid: i64,
    pub watcher_pid: i64,
    pub watcher_creation_time_100ns: i64,
    pub process_uuid: Uuid,
    pub executable_sha256: String,
    pub verifying_key_hex: String,
}


pub type PalmistryWatcherOsRecheck = Arc<dyn Fn() -> Result<(), String> + Send + Sync + 'static>;

#[derive(Clone, Debug)]
pub struct PalmistryWatcherAdoptionInput {
    pub verifier: SurrealPalmistryVerifier,
    pub original_start: crate::process_ledger::ProcessStart,
    pub parent_creation_time_100ns: i64,
    pub custodian_registration_id: String,
    pub signed_native_evidence: Value,
}


#[derive(Clone, Copy, PartialEq, Eq, SurrealValue)]
pub(crate) enum WatcherAdmissionPurpose {
    Adopt,
    Register,
}

impl WatcherAdmissionPurpose {
    pub(crate) fn as_str(self) -> &'static str {
        match self { Self::Adopt => "adopt", Self::Register => "register" }
    }
}

pub(crate) struct WatcherAdmissionDeadline {
    monotonic: std::time::Instant,
    expires_at: chrono::DateTime<chrono::Utc>,
}

impl WatcherAdmissionDeadline {
    pub(crate) fn capture_for_original_owner(
        authorization: &super::resource_authority::PalmistryWatcherAdmissionAuthorization,
        original_monotonic: std::time::Instant,
        original_utc: chrono::DateTime<chrono::Utc>,
        budget: super::WatcherAdmissionBudget,
    ) -> Result<Self, SurrealPalmistryError> {
        let limit = budget.limit.min(std::time::Duration::from_secs(300));
        if limit.is_zero() { return Err(SurrealPalmistryError::InvalidInput); }
        let delta = chrono::Duration::from_std(limit).map_err(|_| SurrealPalmistryError::InvalidInput)?;
        let requested = original_utc.checked_add_signed(delta).ok_or(SurrealPalmistryError::InvalidInput)?;
        let expires_at = requested.min(budget.credential_not_after).min(*authorization.expires_at());
        let remaining = expires_at.signed_duration_since(original_utc).to_std().map_err(|_| SurrealPalmistryError::InvalidInput)?;
        if remaining.is_zero() { return Err(SurrealPalmistryError::InvalidInput); }
        let monotonic = original_monotonic.checked_add(remaining).ok_or(SurrealPalmistryError::InvalidInput)?;
        authorization.validate_binding(authorization.authority(), &expires_at)?;
        if monotonic <= std::time::Instant::now() { return Err(SurrealPalmistryError::InvalidInput); }
        Ok(Self { monotonic, expires_at })
    }

    pub(crate) fn monotonic(&self) -> std::time::Instant { self.monotonic }
    pub(crate) fn expires_at(&self) -> &chrono::DateTime<chrono::Utc> { &self.expires_at }
}

pub(crate) struct PalmistryWatcherAdmissionManifest {
    purpose: WatcherAdmissionPurpose,
    operation_id: Uuid,
    body: Value,
    hash: String,
}

impl PalmistryWatcherAdmissionManifest {
    pub(crate) fn purpose(&self) -> WatcherAdmissionPurpose { self.purpose }
    pub(crate) fn operation_id(&self) -> Uuid { self.operation_id }
    pub(crate) fn body(&self) -> &Value { &self.body }
    pub(crate) fn hash(&self) -> &str { &self.hash }
}

pub(crate) struct PalmistryWatcherPreparedAdmission {
    authorization: Arc<super::resource_authority::PalmistryWatcherAdmissionAuthorization>,
    input: PalmistryWatcherAdoptionInput,
    witness: crate::api::palmistry::NativeWatcherAdoptionWitness,
    deadline: WatcherAdmissionDeadline,
    manifest: PalmistryWatcherAdmissionManifest,
}

impl PalmistryWatcherPreparedAdmission {
    pub(crate) fn authorization(&self) -> &Arc<super::resource_authority::PalmistryWatcherAdmissionAuthorization> { &self.authorization }
    pub(crate) fn deadline(&self) -> &WatcherAdmissionDeadline { &self.deadline }
    pub(crate) fn manifest(&self) -> &PalmistryWatcherAdmissionManifest { &self.manifest }
    pub(crate) fn operation_id(&self) -> Uuid { self.manifest.operation_id }
    pub(crate) fn validate_binding(&self) -> Result<(), SurrealPalmistryError> {
        self.authorization.validate_binding(self.authorization.authority(), self.deadline.expires_at())?;
        if self.deadline.monotonic() <= std::time::Instant::now() { return Err(SurrealPalmistryError::InvalidInput); }
        self.witness.validate_admission_binding(self.authorization.authority(), &self.input, self.deadline.monotonic())
            .map_err(|_| SurrealPalmistryError::InvalidInput)
    }
}

pub(crate) fn prepare_watcher_admission(
    authorization: Arc<super::resource_authority::PalmistryWatcherAdmissionAuthorization>,
    input: PalmistryWatcherAdoptionInput,
    witness: crate::api::palmistry::NativeWatcherAdoptionWitness,
    deadline: WatcherAdmissionDeadline,
) -> Result<Arc<PalmistryWatcherPreparedAdmission>, SurrealPalmistryError> {
    authorization.validate_binding(authorization.authority(), deadline.expires_at())?;
    validate_watcher_native_evidence(&input)?;
    witness.validate_admission_binding(authorization.authority(), &input, deadline.monotonic())
        .map_err(|_| SurrealPalmistryError::InvalidInput)?;
    let authority = authorization.authority();
    let verifier = &input.verifier;
    if authority.watcher_id() != verifier.process_uuid || authority.operation_id().is_nil() {
        return Err(SurrealPalmistryError::InvalidInput);
    }
    let bindings = SurrealPalmistryStore::watcher_adoption_bindings_for_scope(authority.exact_scope(), authority, &input)?;
    let body = serde_json::json!({
        "schema_id": "handshake.palmistry.watcher-admission-manifest", "schema_version": 1,
        "purpose": "adopt", "operation_id": authority.operation_id(),
        "input_hash": bindings.input_hash, "event_payload": bindings.event_payload,
        "adoption_record_id": bindings.adoption_record_id, "ledger_record_id": bindings.ledger_record_id,
        "start_record_id": bindings.start_record_id, "original_start_hash": bindings.original_start_hash,
        "parent_resource_id": authority.workspace_parent().id.to_sql(),
        "owner_account_id": authority.exact_scope().owner_account_id.to_string(),
        "actor_principal_id": authority.exact_scope().actor_principal_id.to_string(),
        "authenticated_session_id": authority.exact_scope().authenticated_session_id.to_string(),
        "access_space_id": authority.exact_scope().access_space_id.to_string(),
        "workspace_id": authority.exact_scope().workspace_id.as_str(),
        "resource_id": authorization.resource_id().to_sql(), "grant_id": authorization.grant_id().to_sql(),
        "grant_version": authorization.grant_version(), "policy_versions": authorization.policy_versions(),
        "revocation_epochs": authorization.revocation_epochs(), "delegation_chain": authorization.delegation_chain(),
        "expires_at": deadline.expires_at().to_rfc3339(),
        "verifier": { "session_id": verifier.session_id, "launch_nonce": verifier.launch_nonce,
            "parent_pid": verifier.parent_pid, "watcher_pid": verifier.watcher_pid,
            "watcher_creation_time_100ns": verifier.watcher_creation_time_100ns,
            "process_uuid": verifier.process_uuid, "executable_sha256": verifier.executable_sha256,
            "verifying_key_hex": verifier.verifying_key_hex },
        "original_start": serde_json::to_value(&input.original_start).map_err(|_| SurrealPalmistryError::InvalidInput)?,
        "parent_creation_time_100ns": input.parent_creation_time_100ns,
        "custodian_registration_id": input.custodian_registration_id,
        "signed_native_evidence": input.signed_native_evidence,
    });
    let hash = sha256_hex(&serde_json::to_vec(&body).map_err(|_| SurrealPalmistryError::InvalidInput)?);
    let prepared = Arc::new(PalmistryWatcherPreparedAdmission {
        manifest: PalmistryWatcherAdmissionManifest { purpose: WatcherAdmissionPurpose::Adopt,
            operation_id: authority.operation_id(), body, hash },
        authorization, input, witness, deadline,
    });
    prepared.validate_binding()?;
    Ok(prepared)
}


#[derive(SurrealValue)]
pub(crate) struct PalmistryWatcherAdmissionIdentity {
    access: String,
    auth: surrealdb::types::RecordId,
    token_access: String,
    token_id: String,
    jti: String,
}

impl PalmistryWatcherAdmissionIdentity {
    pub(crate) fn validate(&self, prepared: &PalmistryWatcherPreparedAdmission) -> Result<(), SurrealPalmistryError> {
        prepared.validate_binding()?;
        let expected = surrealdb::types::RecordId::new("authenticated_sessions", prepared.authorization.exact_scope().authenticated_session_id.to_string());
        if self.access != "palmistry_watcher_admission" || self.token_access != self.access
            || self.auth != expected || self.token_id != expected.to_sql()
            || Uuid::parse_str(&self.jti).is_err() {
            return Err(SurrealPalmistryError::PermissionDenied);
        }
        Ok(())
    }
}

pub(crate) const WATCHER_ADMISSION_IDENTITY_SQL: &str =
    "RETURN { access: $access, auth: $auth, token_access: $token.AC, token_id: $token.ID, jti: $token.jti };";

#[derive(Clone, SurrealValue)]
pub(crate) struct PalmistryWatcherInstalledPermit {
    permit_id: String,
    operation_id: Uuid,
    purpose: String,
    manifest_hash: String,
    a_access: String,
    a_auth: surrealdb::types::RecordId,
    a_token_id: String,
    a_jti: String,
    key_version: String,
    expires_at: chrono::DateTime<chrono::Utc>,
    state: String,
}

impl PalmistryWatcherInstalledPermit {
    pub(crate) fn validate(&self, prepared: &PalmistryWatcherPreparedAdmission) -> Result<(), SurrealPalmistryError> {
        prepared.validate_binding()?;
        let expected = surrealdb::types::RecordId::new("authenticated_sessions", prepared.authorization.exact_scope().authenticated_session_id.to_string());
        if self.permit_id != format!("palmistry-{}-{}", prepared.manifest.purpose.as_str(), prepared.operation_id())
            || self.operation_id != prepared.operation_id() || self.purpose != prepared.manifest.purpose.as_str()
            || self.manifest_hash != prepared.manifest.hash || self.a_access != "palmistry_watcher_admission"
            || self.a_auth != expected || self.a_token_id != expected.to_sql()
            || Uuid::parse_str(&self.a_jti).is_err() || self.key_version.trim().is_empty()
            || self.expires_at != *prepared.deadline.expires_at() || self.state != "issued" {
            return Err(SurrealPalmistryError::PermissionDenied);
        }
        Ok(())
    }
}


#[derive(SurrealValue)]
struct CanonicalWatcherLookupBindings {
    registration: Option<String>, process_uuid: Uuid, session_id: Uuid, launch_nonce: Uuid,
    parent_pid: i64, watcher_pid: i64, parent_birth: i64, watcher_birth: i64,
    executable_sha256: String, verifying_key_hex: String, resource: String, parent: surrealdb::types::RecordId,
}

#[derive(SurrealValue)]
struct CanonicalWatcherOriginalStart {
    registration_id: String,
    original_start: Value,
}


#[derive(SurrealValue)]
pub(crate) struct WatcherPermitInstallBindings {
    permit_id: String,
    permit: WatcherPermitContent,
}

#[derive(SurrealValue)]
struct WatcherPermitContent {
    operation_id: Uuid, purpose: String, state: String,
    a_access: String, a_auth: surrealdb::types::RecordId, a_token_id: String, a_jti: String,
    key_version: String, expires_at: chrono::DateTime<chrono::Utc>,
    manifest_hash: String, manifest: Value,
    owner_account_id: String, actor_principal_id: String, authenticated_session_id: String,
    access_space_id: String, workspace_id: String,
    authority_resource_id: surrealdb::types::RecordId,
    authority_parent_resource_id: surrealdb::types::RecordId,
    authority_grant_id: surrealdb::types::RecordId,
    authority_session_id: surrealdb::types::RecordId,
    authority_capability_id: String, authority_action: String, grant_version: i64,
}

impl PalmistryWatcherPreparedAdmission {
    pub(crate) fn permit_install_bindings(
        &self, identity: &PalmistryWatcherAdmissionIdentity, key_version: &str,
    ) -> Result<WatcherPermitInstallBindings, SurrealPalmistryError> {
        identity.validate(self)?;
        if key_version.is_empty() || !key_version.bytes().all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte)) {
            return Err(SurrealPalmistryError::InvalidInput);
        }
        let exact = self.authorization.exact_scope();
        Ok(WatcherPermitInstallBindings {
            permit_id: format!("palmistry-{}-{}", self.manifest.purpose.as_str(), self.operation_id()),
            permit: WatcherPermitContent {
                operation_id: self.operation_id(), purpose: self.manifest.purpose.as_str().into(), state: "issued".into(),
                a_access: identity.access.clone(), a_auth: identity.auth.clone(),
                a_token_id: identity.token_id.clone(), a_jti: identity.jti.clone(), key_version: key_version.into(),
                expires_at: *self.deadline.expires_at(), manifest_hash: self.manifest.hash.clone(), manifest: self.manifest.body.clone(),
                owner_account_id: exact.owner_account_id.to_string(), actor_principal_id: exact.actor_principal_id.to_string(),
                authenticated_session_id: exact.authenticated_session_id.to_string(), access_space_id: exact.access_space_id.to_string(),
                workspace_id: exact.workspace_id.as_str().to_owned(),
                authority_resource_id: self.authorization.resource_id().clone(),
                authority_parent_resource_id: self.authorization.authority().workspace_parent().id.clone(),
                authority_grant_id: self.authorization.grant_id().clone(), authority_session_id: identity.auth.clone(),
                authority_capability_id: "diagnostics.watcher.adopt_existing".into(), authority_action: "reconcile".into(),
                grant_version: self.authorization.grant_version(),
            },
        })
    }
}

pub(crate) const WATCHER_PERMIT_INSTALL_SQL: &str = r#"CREATE type::record('palmistry_watcher_operation_permit', $permit_id) CONTENT $permit;
RETURN SELECT record::id(id) AS permit_id, operation_id, purpose, manifest_hash, a_access, a_auth, a_token_id, a_jti, key_version, expires_at, state FROM type::record('palmistry_watcher_operation_permit', $permit_id) WHERE $this = object::extend($permit, { id: type::record('palmistry_watcher_operation_permit', $permit_id) }) LIMIT 2;"#;
pub(crate) const WATCHER_PERMIT_READ_SQL: &str = r#"SELECT record::id(id) AS permit_id, operation_id, purpose, manifest_hash, a_access, a_auth, a_token_id, a_jti, key_version, expires_at, state FROM type::record('palmistry_watcher_operation_permit', $permit_id) WHERE $this = object::extend($permit, { id: type::record('palmistry_watcher_operation_permit', $permit_id) }) LIMIT 2;"#;

#[derive(Clone, Debug, PartialEq, Eq, SurrealValue)]
pub struct PalmistryWatcherAdoptionReceipt {
    pub operation_id: Uuid,
    pub process_uuid: Uuid,
    pub diagnostic_session_id: Uuid,
    pub original_started_at: String,
    pub custodian_registration_id: String,
    pub verifier_record_id: String,
    pub start_record_id: String,
    pub event_ledger_event_id: String,
    pub input_hash: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PalmistryWatcherAdoptionResult {
    Applied(PalmistryWatcherAdoptionReceipt),
    AlreadyApplied(PalmistryWatcherAdoptionReceipt),
    Uncertain {
        operation_id: Uuid,
        canonical_receipt: Option<PalmistryWatcherAdoptionReceipt>,
        diagnostic: String,
    },
}

#[derive(Clone)]
pub struct SurrealPalmistryStore {
    storage: SurrealStorage,
    scope: ExactResourceScopeAttribution,
    initialized: Arc<OnceCell<()>>,
}

#[derive(Debug, SurrealValue)]
struct SchemaState {
    schema_version: String,
    schema_revision: i64,
    apply_state: String,
}

#[derive(Debug, SurrealValue)]
struct SchemaStateBindings {
    schema_version: String,
    schema_revision: i64,
}

#[derive(Debug, SurrealValue)]
struct StoredVerifier {
    session_id: Uuid,
    launch_nonce: Uuid,
    parent_pid: i64,
    watcher_pid: i64,
    watcher_creation_time_100ns: i64,
    process_uuid: Uuid,
    executable_sha256: String,
    verifying_key_hex: String,
}

impl From<StoredVerifier> for SurrealPalmistryVerifier {
    fn from(row: StoredVerifier) -> Self {
        Self {
            session_id: row.session_id,
            launch_nonce: row.launch_nonce,
            parent_pid: row.parent_pid,
            watcher_pid: row.watcher_pid,
            watcher_creation_time_100ns: row.watcher_creation_time_100ns,
            process_uuid: row.process_uuid,
            executable_sha256: row.executable_sha256,
            verifying_key_hex: row.verifying_key_hex,
        }
    }
}

#[derive(Debug, SurrealValue)]
struct VerifierBindings {
    record_id: String,
    ledger_record_id: String,
    idempotency_key: String,
    session_id: Uuid,
    launch_nonce: Uuid,
    parent_pid: i64,
    watcher_pid: i64,
    watcher_creation_time_100ns: i64,
    process_uuid: Uuid,
    executable_sha256: String,
    verifying_key_hex: String,
    payload_hash: String,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(Debug, SurrealValue)]
struct SessionBindings {
    session_id: Uuid,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(Debug, SurrealValue)]
struct ProcessBindings {
    record_id: String,
    ledger_record_id: String,
    idempotency_key: String,
    payload_hash: String,
    session_id: Uuid,
    launch_nonce: Uuid,
    process_uuid: Uuid,
    watcher_pid: i64,
    watcher_creation_time_100ns: i64,
    executable_sha256: String,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(Debug, SurrealValue)]
struct ScopedProcessBindings {
    process_uuid: Uuid,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(Debug, SurrealValue)]
struct KernelEventBindings {
    ledger_record_id: String,
    event_id: String,
    event_version: String,
    kernel_task_run_id: String,
    session_run_id: String,
    aggregate_type: String,
    aggregate_id: String,
    idempotency_key: String,
    event_type: String,
    actor_kind: String,
    actor_id: String,
    causation_id: Option<String>,
    correlation_id: Option<String>,
    payload_hash: String,
    source_component: String,
    payload: Value,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
    authority_resource_id: String,
    authority_grant_id: String,
    authority_session_id: String,
    authority_capability_id: String,
    authority_action: String,
}

#[derive(Debug, SurrealValue)]
struct MutationResult {
    outcome: String,
}

#[derive(Debug, SurrealValue)]
struct EventIdResult {
    event_id: String,
}

// FINAL catalog reuse requires the owning durable receipt; this never installs DDL.
pub(super) async fn verify_schema_receipt(
    database: &super::SurrealAdminContext<'_>,
) -> Result<(), SurrealStorageError> {
    let mut response = database.query("SELECT * FROM ONLY palmistry_schema_state:primary;").await?;
    let state: Option<SchemaState> = response.take(0)?;
    if !state.as_ref().is_some_and(|state| {
        state.schema_version == SCHEMA_VERSION
            && state.schema_revision == SCHEMA_REVISION
            && state.apply_state == "complete"
    }) {
        return Err(SurrealStorageError::InvalidModelLaneRecord {
            reason: "Palmistry schema receipt missing or version/revision/state mismatch",
        });
    }
    Ok(())
}

pub async fn bootstrap_palmistry_schema(
    storage: &SurrealStorage,
) -> Result<(), SurrealStorageError> {
    super::bootstrap_schema(storage).await?;
    install_palmistry_schema(storage).await
}

// Only the production composer calls this after its verified shared prerequisite.
pub(super) async fn install_palmistry_schema(storage: &SurrealStorage) -> Result<(), SurrealStorageError> {
    storage
        .with_admin_operation(|database| {
            Box::pin(async move {
                database.query(SCHEMA_STATE).await?;
                let mut response = database
                    .query("SELECT * FROM ONLY palmistry_schema_state:primary;")
                    .await?;
                let state: Option<SchemaState> = response.take(0)?;
                if state.as_ref().is_some_and(|state| {
                    state.schema_version != SCHEMA_VERSION
                        || state.schema_revision != SCHEMA_REVISION
                        || state.apply_state != "complete"
                }) {
                    return Err(SurrealStorageError::InvalidModelLaneRecord {
                        reason: "Palmistry schema state version/revision mismatch",
                    });
                }
                database.query(SCHEMA).await?;
                if state.is_none() {
                    database
                        .query_bound(
                            "UPSERT palmistry_schema_state:primary CONTENT { schema_version: $schema_version, schema_revision: $schema_revision, apply_state: 'complete' };",
                            SchemaStateBindings {
                                schema_version: SCHEMA_VERSION.to_owned(),
                                schema_revision: SCHEMA_REVISION,
                            },
                        )
                        .await?;
                }
                Ok(())
            })
        })
        .await
}

impl SurrealPalmistryStore {
    pub fn new_exact(storage: SurrealStorage, scope: ExactResourceScopeAttribution) -> Self {
        Self {
            storage,
            scope,
            initialized: Arc::new(OnceCell::new()),
        }
    }

    async fn ensure_initialized(&self) -> Result<(), SurrealPalmistryError> {
        self.initialized
            .get_or_try_init(|| async { bootstrap_palmistry_schema(&self.storage).await })
            .await?;
        Ok(())
    }

    pub async fn register(
        &self,
        verifier: SurrealPalmistryVerifier,
    ) -> Result<(), SurrealPalmistryError> {
        self.ensure_initialized().await?;
        if verifier.session_id.is_nil()
            || verifier.launch_nonce.is_nil()
            || verifier.process_uuid.is_nil()
            || verifier.parent_pid <= 0
            || verifier.watcher_pid <= 0
            || verifier.watcher_creation_time_100ns < 0
            || verifier.executable_sha256.len() != 64
            || verifier.verifying_key_hex.len() != 64
        {
            return Err(SurrealPalmistryError::InvalidInput);
        }
        let identity = format!(
            "{}:{}:{}",
            verifier.session_id, verifier.launch_nonce, verifier.process_uuid
        );
        let identity_hash = self.identity_hash(&identity);
        let payload = serde_json::json!({
            "session_id": verifier.session_id,
            "launch_nonce": verifier.launch_nonce,
            "parent_pid": verifier.parent_pid,
            "watcher_pid": verifier.watcher_pid,
            "watcher_creation_time_100ns": verifier.watcher_creation_time_100ns,
            "process_uuid": verifier.process_uuid,
            "executable_sha256": verifier.executable_sha256,
            "verifying_key_hex": verifier.verifying_key_hex,
        });
        let payload_hash = sha256_hex(
            serde_json::to_string(&payload)
                .map_err(|_| SurrealPalmistryError::InvalidInput)?
                .as_bytes(),
        );
        let scope = self.session_bindings(verifier.session_id);
        let bindings = VerifierBindings {
            record_id: format!("palmistry-verifier-{identity_hash}"),
            ledger_record_id: format!("evt-palmistry-verifier-{identity_hash}"),
            idempotency_key: format!("palmistry-verifier:{identity_hash}"),
            session_id: verifier.session_id,
            launch_nonce: verifier.launch_nonce,
            parent_pid: verifier.parent_pid,
            watcher_pid: verifier.watcher_pid,
            watcher_creation_time_100ns: verifier.watcher_creation_time_100ns,
            process_uuid: verifier.process_uuid,
            executable_sha256: verifier.executable_sha256,
            verifying_key_hex: verifier.verifying_key_hex,
            payload_hash,
            owner_account_id: scope.owner_account_id,
            actor_principal_id: scope.actor_principal_id,
            authenticated_session_id: scope.authenticated_session_id,
            access_space_id: scope.access_space_id,
            workspace_id: scope.workspace_id,
        };
        let rows = self.storage.with_data_operation(|database| Box::pin(async move {
            database.query_values_at::<MutationResult, _>(
                "BEGIN TRANSACTION;\
                 LET $active = (SELECT session_id, launch_nonce, process_uuid, executable_sha256, verifying_key_hex FROM palmistry_durable_verifier WHERE session_id = $session_id AND retired_at = NONE AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2);\
                 LET $prior = (SELECT payload_hash FROM type::record('kernel_event_ledger', $ledger_record_id) WHERE idempotency_key = $idempotency_key AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2);\
                 IF array::len($active) > 1 OR array::len($prior) > 1 { RETURN { outcome: 'corrupt' }; } ELSE IF array::len($active) = 1 { RETURN { outcome: IF $active[0].launch_nonce = $launch_nonce AND $active[0].process_uuid = $process_uuid AND $active[0].executable_sha256 = $executable_sha256 AND $active[0].verifying_key_hex = $verifying_key_hex { 'already_applied' } ELSE { 'conflict' } }; } ELSE IF array::len($prior) = 1 { RETURN { outcome: IF $prior[0].payload_hash = $payload_hash { 'already_applied' } ELSE { 'conflict' } }; } ELSE {\
                   LET $ledger = CREATE type::record('kernel_event_ledger', $ledger_record_id) CONTENT { event_id: $ledger_record_id, event_version: 'kernel_event_v1', kernel_task_run_id: $workspace_id, session_run_id: <string>$session_id, aggregate_type: 'palmistry_watcher_verifier', aggregate_id: $record_id, idempotency_key: $idempotency_key, event_type: 'PALMISTRY_WATCHER_VERIFIER_RECORDED', actor_kind: 'principal', actor_id: $actor_principal_id, causation_id: NONE, correlation_id: NONE, payload_hash: $payload_hash, source_component: 'palmistry', payload: { session_id: $session_id, launch_nonce: $launch_nonce, process_uuid: $process_uuid, watcher_pid: $watcher_pid, executable_sha256: $executable_sha256 }, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id, created_at: time::now() };\
                   LET $stored = CREATE type::record('palmistry_durable_verifier', $record_id) CONTENT { session_id: $session_id, launch_nonce: $launch_nonce, parent_pid: $parent_pid, watcher_pid: $watcher_pid, watcher_creation_time_100ns: $watcher_creation_time_100ns, process_uuid: $process_uuid, executable_sha256: $executable_sha256, verifying_key_hex: $verifying_key_hex, retired_at: NONE, event_ledger_event_id: type::record('kernel_event_ledger', $ledger_record_id), owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id, storage_authority: 'embedded_surrealdb', created_at: time::now() };\
                   RETURN { outcome: 'stored' };\
                 };\
                 COMMIT TRANSACTION;",
                bindings,
                3,
            ).await
        })).await?;
        match rows.into_iter().next().map(|row| row.outcome).as_deref() {
            Some("stored" | "already_applied") => Ok(()),
            Some("conflict") => Err(SurrealPalmistryError::IdentityConflict),
            _ => Err(SurrealPalmistryError::CorruptAuthority),
        }
    }

    pub async fn active_for_session(
        &self,
        session_id: Uuid,
    ) -> Result<Vec<SurrealPalmistryVerifier>, SurrealPalmistryError> {
        self.ensure_initialized().await?;
        let bindings = self.session_bindings(session_id);
        let rows = self.storage.with_data_operation(|database| Box::pin(async move {
            database.query_values::<StoredVerifier, _>(
                "SELECT session_id, launch_nonce, parent_pid, watcher_pid, watcher_creation_time_100ns, process_uuid, executable_sha256, verifying_key_hex FROM palmistry_durable_verifier WHERE session_id = $session_id AND retired_at = NONE AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND storage_authority = 'embedded_surrealdb' AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id AND event_ledger_event_id.source_component = 'palmistry' AND event_ledger_event_id.aggregate_type = 'palmistry_watcher_verifier' ORDER BY created_at ASC LIMIT 2;",
                bindings,
            ).await
        })).await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn active_process_matches(
        &self,
        verifier: &SurrealPalmistryVerifier,
    ) -> Result<bool, SurrealPalmistryError> {
        self.ensure_initialized().await?;
        let bindings = self.process_bindings(verifier, String::new());
        let rows = self.storage.with_data_operation(|database| Box::pin(async move {
            database.query_values::<Uuid, _>(
                "SELECT VALUE process_uuid FROM kernel_process_lifecycle WHERE process_uuid = $process_uuid AND parent_session_id = <string>$session_id AND os_pid = $watcher_pid AND sandbox_adapter_id = 'palmistry-watcher' AND metadata.os_creation_time_100ns = $watcher_creation_time_100ns AND metadata.executable_sha256 = $executable_sha256 AND metadata.owner_account_id = $owner_account_id AND metadata.actor_principal_id = $actor_principal_id AND metadata.authenticated_session_id = $authenticated_session_id AND metadata.access_space_id = $access_space_id AND metadata.workspace_id = $workspace_id AND stopped_at = NONE LIMIT 2;",
                bindings,
            ).await
        })).await?;
        if rows.len() > 1 {
            return Err(SurrealPalmistryError::CorruptAuthority);
        }
        Ok(rows.len() == 1)
    }

    pub async fn stop_is_durable(&self, process_uuid: Uuid) -> Result<bool, SurrealPalmistryError> {
        self.ensure_initialized().await?;
        let bindings = self.scoped_process_bindings(process_uuid);
        let rows = self.storage.with_data_operation(|database| Box::pin(async move {
            database.query_values::<Uuid, _>(
                "SELECT VALUE process_uuid FROM kernel_process_lifecycle WHERE process_uuid = $process_uuid AND metadata.owner_account_id = $owner_account_id AND metadata.actor_principal_id = $actor_principal_id AND metadata.authenticated_session_id = $authenticated_session_id AND metadata.access_space_id = $access_space_id AND metadata.workspace_id = $workspace_id AND stopped_at != NONE LIMIT 2;",
                bindings,
            ).await
        })).await?;
        if rows.len() > 1 {
            return Err(SurrealPalmistryError::CorruptAuthority);
        }
        Ok(rows.len() == 1)
    }

    pub async fn retire_exact(
        &self,
        session_id: Uuid,
        launch_nonce: Uuid,
        process_uuid: Uuid,
    ) -> Result<bool, SurrealPalmistryError> {
        self.ensure_initialized().await?;
        let identity = format!("{session_id}:{launch_nonce}:{process_uuid}");
        let identity_hash = self.identity_hash(&identity);
        let scope = self.session_bindings(session_id);
        let bindings = ProcessBindings {
            record_id: format!("palmistry-verifier-{identity_hash}"),
            ledger_record_id: format!("evt-palmistry-verifier-retired-{identity_hash}"),
            idempotency_key: format!("palmistry-verifier-retired:{identity_hash}"),
            payload_hash: sha256_hex(format!("{identity}:retired").as_bytes()),
            session_id,
            launch_nonce,
            process_uuid,
            watcher_pid: 0,
            watcher_creation_time_100ns: 0,
            executable_sha256: String::new(),
            owner_account_id: scope.owner_account_id,
            actor_principal_id: scope.actor_principal_id,
            authenticated_session_id: scope.authenticated_session_id,
            access_space_id: scope.access_space_id,
            workspace_id: scope.workspace_id,
        };
        let rows = self.storage.with_data_operation(|database| Box::pin(async move {
            database.query_values_at::<MutationResult, _>(
                "BEGIN TRANSACTION;\
                 LET $current = (SELECT VALUE id FROM type::record('palmistry_durable_verifier', $record_id) WHERE session_id = $session_id AND launch_nonce = $launch_nonce AND process_uuid = $process_uuid AND retired_at = NONE AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND storage_authority = 'embedded_surrealdb' AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id LIMIT 2);\
                 LET $stopped = (SELECT VALUE process_uuid FROM kernel_process_lifecycle WHERE process_uuid = $process_uuid AND metadata.owner_account_id = $owner_account_id AND metadata.actor_principal_id = $actor_principal_id AND metadata.authenticated_session_id = $authenticated_session_id AND metadata.access_space_id = $access_space_id AND metadata.workspace_id = $workspace_id AND stopped_at != NONE LIMIT 2);\
                 LET $prior = (SELECT VALUE id FROM type::record('kernel_event_ledger', $ledger_record_id) WHERE idempotency_key = $idempotency_key AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2);\
                 IF array::len($current) > 1 OR array::len($stopped) > 1 OR array::len($prior) > 1 { RETURN { outcome: 'corrupt' }; } ELSE IF array::len($prior) = 1 { RETURN { outcome: 'already_applied' }; } ELSE IF array::len($stopped) != 1 { RETURN { outcome: 'stop_required' }; } ELSE IF array::len($current) != 1 { RETURN { outcome: 'missing' }; } ELSE {\
                   LET $ledger = CREATE type::record('kernel_event_ledger', $ledger_record_id) CONTENT { event_id: $ledger_record_id, event_version: 'kernel_event_v1', kernel_task_run_id: $workspace_id, session_run_id: <string>$session_id, aggregate_type: 'palmistry_watcher_verifier', aggregate_id: $record_id, idempotency_key: $idempotency_key, event_type: 'PALMISTRY_WATCHER_VERIFIER_RETIRED', actor_kind: 'principal', actor_id: $actor_principal_id, causation_id: NONE, correlation_id: NONE, payload_hash: $payload_hash, source_component: 'palmistry', payload: { session_id: $session_id, launch_nonce: $launch_nonce, process_uuid: $process_uuid }, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id, created_at: time::now() };\
                   LET $retired = UPDATE type::record('palmistry_durable_verifier', $record_id) SET retired_at = time::now(), event_ledger_event_id = type::record('kernel_event_ledger', $ledger_record_id);\
                   RETURN { outcome: 'retired' };\
                 };\
                 COMMIT TRANSACTION;",
                bindings,
                4,
            ).await
        })).await?;
        match rows.into_iter().next().map(|row| row.outcome).as_deref() {
            Some("retired" | "already_applied") => Ok(true),
            Some("missing") => Ok(false),
            Some("stop_required") => Err(SurrealPalmistryError::StopRequired),
            _ => Err(SurrealPalmistryError::CorruptAuthority),
        }
    }

    pub(crate) async fn append_kernel_event(
        &self,
        event: NewKernelEvent,
        authority: super::resource_authority::RecordUserScope,
        operation_id: Uuid,
    ) -> Result<String, SurrealPalmistryError> {
        self.ensure_initialized().await?;
        if operation_id.is_nil()
            || authority.workspace_id.as_deref() != Some(self.scope.workspace_id.as_str())
            || authority.session_id != self.scope.authenticated_session_id.to_string()
            || authority.capability_id != "fr.ingest.native_editor"
            || authority.action != super::resource_authority::ResourceAction::Create
            || authority.grant_id.as_ref().is_none_or(String::is_empty)
        {
            return Err(SurrealPalmistryError::PermissionDenied);
        }
        event
            .validate()
            .map_err(|_| SurrealPalmistryError::InvalidInput)?;
        let identity_hash = self.identity_hash(&event.idempotency_key);
        let ledger_record_id = format!("evt-palmistry-{identity_hash}");
        let scope = self.session_bindings(Uuid::nil());
        let bindings = KernelEventBindings {
            ledger_record_id: ledger_record_id.clone(),
            event_id: ledger_record_id,
            event_version: event.event_version,
            kernel_task_run_id: event.kernel_task_run_id,
            session_run_id: event.session_run_id,
            aggregate_type: event.aggregate_type,
            aggregate_id: event.aggregate_id,
            idempotency_key: event.idempotency_key,
            event_type: event.event_type.as_str().to_owned(),
            actor_kind: event.actor.actor_kind().to_owned(),
            actor_id: event.actor.actor_id().to_owned(),
            causation_id: event.causation_id,
            correlation_id: event.correlation_id,
            payload_hash: event.payload_hash,
            source_component: event.source_component,
            payload: event.payload,
            owner_account_id: scope.owner_account_id,
            actor_principal_id: scope.actor_principal_id,
            authenticated_session_id: scope.authenticated_session_id,
            access_space_id: scope.access_space_id,
            workspace_id: scope.workspace_id,
            authority_resource_id: authority.resource_id.clone(),
            authority_grant_id: authority.grant_id.clone().ok_or(SurrealPalmistryError::PermissionDenied)?,
            authority_session_id: authority.session_id.clone(),
            authority_capability_id: authority.capability_id.clone(),
            authority_action: authority.action.as_str().to_owned(),
        };
        let rows = self.storage.with_record_user_scope(authority,
            self.storage.with_authenticated_transaction::<Vec<EventIdResult>, SurrealPalmistryError, _>(operation_id, move |transaction| Box::pin(async move {
            transaction.query_values_at::<EventIdResult, _>(
                "LET $resource = type::record('protected_resources', $authority_resource_id);\
                 IF !fn::mt109_ledger_access($resource, type::record('authenticated_sessions', $authority_session_id), $authority_capability_id, $authority_action) { THROW 'HSK-403-PROTECTED-RESOURCE'; };\
                 LET $grant = (SELECT * FROM type::record('resource_grants', $authority_grant_id) WHERE account_id = $auth.account_id AND principal_id = $auth.principal_id AND access_space_id = $auth.access_space_id AND resource_id = $resource AND status = 'active' AND revoked_at = NONE AND (expires_at = NONE OR expires_at > time::now()) AND actions CONTAINS $authority_action AND capability_ids CONTAINS $authority_capability_id AND delegation_chain = $auth.delegation_chain);\
                 IF array::len($grant) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; };\
                 LET $prior = (SELECT event_id, payload_hash, aggregate_type, aggregate_id, event_type, source_component FROM type::record('kernel_event_ledger', $ledger_record_id) WHERE idempotency_key = $idempotency_key AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2);\
                 IF array::len($prior) > 1 { THROW 'Palmistry EventLedger authority is ambiguous'; } ELSE IF array::len($prior) = 1 { IF $prior[0].payload_hash != $payload_hash OR $prior[0].aggregate_type != $aggregate_type OR $prior[0].aggregate_id != $aggregate_id OR $prior[0].event_type != $event_type OR $prior[0].source_component != $source_component { THROW 'Palmistry EventLedger idempotency conflict'; }; RETURN $prior; } ELSE { RETURN CREATE type::record('kernel_event_ledger', $ledger_record_id) CONTENT { event_id: $event_id, event_version: $event_version, kernel_task_run_id: $kernel_task_run_id, session_run_id: $session_run_id, aggregate_type: $aggregate_type, aggregate_id: $aggregate_id, idempotency_key: $idempotency_key, event_type: $event_type, actor_kind: $actor_kind, actor_id: $actor_id, causation_id: $causation_id, correlation_id: $correlation_id, payload_hash: $payload_hash, source_component: $source_component, payload: $payload, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id, wsids: [$workspace_id], authority_resource_id: $resource, authority_grant_id: type::record('resource_grants', $authority_grant_id), authority_session_id: type::record('authenticated_sessions', $authority_session_id), authority_capability_id: $authority_capability_id, authority_action: $authority_action, created_at: time::now() }; };",
                bindings,
                5,
            ).await.map_err(SurrealPalmistryError::from)
        }))).await?;
        let [row] = rows.as_slice() else {
            return Err(SurrealPalmistryError::PermissionDenied);
        };
        Ok(row.event_id.clone())
    }

    fn session_bindings(&self, session_id: Uuid) -> SessionBindings {
        SessionBindings {
            session_id,
            owner_account_id: self.scope.owner_account_id.as_uuid().to_string(),
            actor_principal_id: self.scope.actor_principal_id.as_uuid().to_string(),
            authenticated_session_id: self.scope.authenticated_session_id.as_uuid().to_string(),
            access_space_id: self.scope.access_space_id.as_uuid().to_string(),
            workspace_id: self.scope.workspace_id.as_str().to_owned(),
        }
    }

    fn scoped_process_bindings(&self, process_uuid: Uuid) -> ScopedProcessBindings {
        let scope = self.session_bindings(Uuid::nil());
        ScopedProcessBindings {
            process_uuid,
            owner_account_id: scope.owner_account_id,
            actor_principal_id: scope.actor_principal_id,
            authenticated_session_id: scope.authenticated_session_id,
            access_space_id: scope.access_space_id,
            workspace_id: scope.workspace_id,
        }
    }

    fn process_bindings(
        &self,
        verifier: &SurrealPalmistryVerifier,
        ledger_record_id: String,
    ) -> ProcessBindings {
        let scope = self.session_bindings(verifier.session_id);
        let identity = format!(
            "{}:{}:{}",
            verifier.session_id, verifier.launch_nonce, verifier.process_uuid
        );
        ProcessBindings {
            record_id: format!("palmistry-verifier-{}", self.identity_hash(&identity)),
            ledger_record_id,
            idempotency_key: format!(
                "palmistry-verifier-retired:{}",
                self.identity_hash(&identity)
            ),
            payload_hash: sha256_hex(format!("{identity}:retired").as_bytes()),
            session_id: verifier.session_id,
            launch_nonce: verifier.launch_nonce,
            process_uuid: verifier.process_uuid,
            watcher_pid: verifier.watcher_pid,
            watcher_creation_time_100ns: verifier.watcher_creation_time_100ns,
            executable_sha256: verifier.executable_sha256.clone(),
            owner_account_id: scope.owner_account_id,
            actor_principal_id: scope.actor_principal_id,
            authenticated_session_id: scope.authenticated_session_id,
            access_space_id: scope.access_space_id,
            workspace_id: scope.workspace_id,
        }
    }

    fn identity_hash(&self, identity: &str) -> String {
        let scope = self.session_bindings(Uuid::nil());
        sha256_hex(
            format!(
                "{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}",
                scope.owner_account_id,
                scope.actor_principal_id,
                scope.authenticated_session_id,
                scope.access_space_id,
                scope.workspace_id,
                identity
            )
            .as_bytes(),
        )
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

#[derive(Debug, Error)]
enum WatcherAdoptionTransactionFailure {
    #[error("watcher adoption callback failed: {0}")]
    OriginalCallback(SurrealPalmistryError),
    #[error(transparent)]
    Controller(#[from] SurrealStorageError),
}

#[derive(Clone, Debug, SurrealValue)]
struct WatcherAdoptionBindings {
    permit_id: Option<String>,
    manifest_hash: Option<String>,
    operation_id: Uuid,
    adoption_record_id: String,
    ledger_record_id: String,
    start_record_id: String,
    custodian_registration_id: String,
    process_uuid: Uuid,
    session_id: Uuid,
    launch_nonce: Uuid,
    parent_pid: i64,
    watcher_pid: i64,
    parent_creation_time_100ns: i64,
    watcher_creation_time_100ns: i64,
    executable_sha256: String,
    verifying_key_hex: String,
    start_parent_process_id: Option<Uuid>,
    start_sandbox_internal_id: Option<String>,
    start_engine_kind: String,
    start_model_artifact_sha256: Option<String>,
    start_work_profile_id: Option<String>,
    start_owner_role: String,
    start_owner_wp: Option<String>,
    start_role_id: Option<String>,
    start_wp_id: Option<String>,
    start_mt_id: Option<String>,
    start_capabilities: Value,
    start_metadata: Value,
    event_payload: Value,
    original_started_at: String,
    original_start_hash: String,
    original_start: Value,
    signed_native_evidence: Value,
    input_hash: String,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
    authority_resource_id: String,
    authority_parent_resource_id: surrealdb::types::RecordId,
    authority_grant_id: String,
}

#[derive(Debug, SurrealValue)]
struct WatcherAdoptionRow {
    already_applied: bool,
    receipt: PalmistryWatcherAdoptionReceipt,
}

impl SurrealPalmistryStore {
    pub(crate) async fn inspect_canonical_watcher_admission_read(
        &self,
        authority: &Arc<super::resource_authority::PalmistryWatcherAdoptionAuthority>,
        operation_id: Uuid,
        purpose: WatcherAdmissionPurpose,
    ) -> Result<super::WatcherAdmissionInspection, SurrealPalmistryError> {
        if authority.operation_id() != operation_id || operation_id.is_nil()
            || authority.exact_scope() != &self.scope
        {
            return Err(SurrealPalmistryError::PermissionDenied);
        }
        #[derive(SurrealValue)]
        struct SummaryBindings { operation_id: Uuid, watcher: Uuid, purpose: String }
        #[derive(SurrealValue)]
        struct Summary {
            permits: Vec<Value>,
            custodians: i64,
            adoptions: i64,
            verifiers: i64,
            events: i64,
        }
        let bindings = SummaryBindings { operation_id, watcher: authority.watcher_id(), purpose: purpose.as_str().into() };
        let rows = self.storage.with_data_operation(|database| Box::pin(async move {
            database.query_values::<Summary, _>(r#"RETURN [{
                permits: (SELECT purpose, manifest, manifest_hash, state FROM palmistry_watcher_operation_permit WHERE operation_id = $operation_id OR manifest.verifier.process_uuid = <string>$watcher LIMIT 2),
                custodians: array::len(SELECT VALUE id FROM palmistry_watcher_custodian WHERE registration_operation_id = $operation_id OR ($purpose = 'register' AND process_uuid = $watcher) LIMIT 2),
                adoptions: array::len(SELECT VALUE id FROM palmistry_watcher_adoption WHERE operation_id = $operation_id OR process_uuid = $watcher LIMIT 2),
                verifiers: array::len(SELECT VALUE id FROM palmistry_durable_verifier WHERE process_uuid = $watcher LIMIT 2),
                events: array::len(SELECT VALUE id FROM kernel_event_ledger WHERE id = type::record('kernel_event_ledger', 'evt-palmistry-adopt-' + <string>$operation_id) OR (source_component = 'palmistry' AND (payload.operation_id = <string>$operation_id OR payload.process_uuid = <string>$watcher)) LIMIT 2)
            }];"#, bindings).await
        })).await?;
        let mut rows = rows.into_iter();
        let summary = rows.next().ok_or(SurrealPalmistryError::CorruptAuthority)?;
        if rows.next().is_some() { return Err(SurrealPalmistryError::CorruptAuthority); }
        if summary.permits.is_empty() && summary.custodians == 0 && summary.adoptions == 0 && summary.verifiers == 0 && summary.events == 0 {
            return Ok(super::WatcherAdmissionInspection::NoPriorRelevantAdmission);
        }
        let uncertain = |diagnostic: &str| super::WatcherAdmissionInspection::ExistingReadOnly(
            PalmistryWatcherAdoptionResult::Uncertain {
                operation_id, canonical_receipt: None, diagnostic: diagnostic.into(),
            },
        );
        let [permit] = summary.permits.as_slice() else {
            return Ok(uncertain("canonical admission manifest is missing or ambiguous"));
        };
        let manifest = &permit["manifest"];
        if purpose != WatcherAdmissionPurpose::Adopt || permit["purpose"].as_str() != Some(purpose.as_str())
            || permit["state"].as_str() != Some("finalized")
            || manifest["operation_id"].as_str() != Some(operation_id.to_string().as_str())
            || manifest["purpose"].as_str() != Some(purpose.as_str())
            || permit["manifest_hash"].as_str() != Some(sha256_hex(
                &serde_json::to_vec(manifest).map_err(|_| SurrealPalmistryError::CorruptAuthority)?,
            ).as_str())
            || summary.adoptions != 1 || summary.verifiers != 1 || summary.events != 1
        {
            return Ok(uncertain("canonical admission graph is incomplete or conflicts with this operation"));
        }
        #[derive(serde::Deserialize)]
        struct RetainedVerifier {
            session_id: Uuid, launch_nonce: Uuid, parent_pid: i64, watcher_pid: i64,
            watcher_creation_time_100ns: i64, process_uuid: Uuid,
            executable_sha256: String, verifying_key_hex: String,
        }
        let input = (|| -> Result<PalmistryWatcherAdoptionInput, SurrealPalmistryError> {
            let verifier: RetainedVerifier = serde_json::from_value(manifest["verifier"].clone())
                .map_err(|_| SurrealPalmistryError::CorruptAuthority)?;
            Ok(PalmistryWatcherAdoptionInput {
                verifier: SurrealPalmistryVerifier {
                    session_id: verifier.session_id, launch_nonce: verifier.launch_nonce,
                    parent_pid: verifier.parent_pid, watcher_pid: verifier.watcher_pid,
                    watcher_creation_time_100ns: verifier.watcher_creation_time_100ns,
                    process_uuid: verifier.process_uuid, executable_sha256: verifier.executable_sha256,
                    verifying_key_hex: verifier.verifying_key_hex,
                },
                original_start: serde_json::from_value(manifest["original_start"].clone())
                    .map_err(|_| SurrealPalmistryError::CorruptAuthority)?,
                parent_creation_time_100ns: manifest["parent_creation_time_100ns"].as_i64()
                    .ok_or(SurrealPalmistryError::CorruptAuthority)?,
                custodian_registration_id: manifest["custodian_registration_id"].as_str()
                    .ok_or(SurrealPalmistryError::CorruptAuthority)?.to_owned(),
                signed_native_evidence: manifest["signed_native_evidence"].clone(),
            })
        })();
        let canonical = match input {
            Ok(input) => match self.watcher_adoption_bindings(authority, &input) {
                Ok(bindings) => self.read_watcher_adoption(bindings).await,
                Err(error) => Err(error),
            },
            Err(error) => Err(error),
        };
        Ok(super::WatcherAdmissionInspection::ExistingReadOnly(match canonical {
            Ok(row) => PalmistryWatcherAdoptionResult::Uncertain {
                operation_id, canonical_receipt: Some(row.receipt),
                diagnostic: "canonical admission graph found; original controller result is still required".into(),
            },
            Err(error) => PalmistryWatcherAdoptionResult::Uncertain {
                operation_id, canonical_receipt: None, diagnostic: error.to_string(),
            },
        }))
    }
    pub(crate) async fn resolve_canonical_watcher_adoption_input(
        &self,
        authority: &Arc<super::resource_authority::PalmistryWatcherAdoptionAuthority>,
        custodian_registration_id: Option<&str>,
        verifier: SurrealPalmistryVerifier,
        parent_creation_time_100ns: i64,
        signed_native_evidence: Value,
    ) -> Result<Option<PalmistryWatcherAdoptionInput>, SurrealPalmistryError> {
        if custodian_registration_id.is_some_and(|value| value.trim().is_empty()) || authority.exact_scope() != &self.scope
            || authority.watcher_id() != verifier.process_uuid {
            return Err(SurrealPalmistryError::InvalidInput);
        }
        let registration = custodian_registration_id.map(str::to_owned);
        let process_uuid = verifier.process_uuid;
        let session_id = verifier.session_id;
        let launch_nonce = verifier.launch_nonce;
        let parent_pid = verifier.parent_pid;
        let watcher_pid = verifier.watcher_pid;
        let watcher_birth = verifier.watcher_creation_time_100ns;
        let executable_sha256 = verifier.executable_sha256.clone();
        let verifying_key_hex = verifier.verifying_key_hex.clone();
        let resource = authority.record_user_scope().resource_id;
        let parent = authority.workspace_parent().id.clone();
        let read = async { self.storage.with_data_operation(|database| Box::pin(async move {
            database.query_values::<CanonicalWatcherOriginalStart, _>(
                "SELECT record::id(id) AS registration_id, original_start FROM palmistry_watcher_custodian WHERE ($registration = NONE OR id = type::record('palmistry_watcher_custodian', $registration)) AND process_uuid = $process_uuid AND diagnostic_session_id = $session_id AND launch_nonce = $launch_nonce AND parent_pid = $parent_pid AND watcher_pid = $watcher_pid AND parent_creation_time_100ns = $parent_birth AND watcher_creation_time_100ns = $watcher_birth AND executable_sha256 = $executable_sha256 AND verifying_key_hex = $verifying_key_hex AND authority_resource_id = type::record('protected_resources', $resource) AND authority_parent_resource_id = $parent LIMIT 2;",
                CanonicalWatcherLookupBindings { registration, process_uuid, session_id, launch_nonce, parent_pid,
                    watcher_pid, parent_birth: parent_creation_time_100ns, watcher_birth,
                    executable_sha256, verifying_key_hex, resource, parent },
            ).await
        })).await.map_err(SurrealPalmistryError::from) };
        self.storage.with_palmistry_watcher_adoption_readback(authority, read, |rows| {
            let mut rows = rows.into_iter();
            let Some(row) = rows.next() else { return Ok(None); };
            if rows.next().is_some() { return Err(SurrealPalmistryError::CorruptAuthority); }
            let original_start = serde_json::from_value(row.original_start).map_err(|_| SurrealPalmistryError::CorruptAuthority)?;
            let input = PalmistryWatcherAdoptionInput { verifier, original_start,
                parent_creation_time_100ns, custodian_registration_id: row.registration_id,
                signed_native_evidence };
            self.watcher_adoption_bindings(authority, &input)?;
            Ok(Some(input))
        }).await
    }

    pub(crate) async fn adopt_existing_watcher<P>(
        &self,
        authority: &Arc<super::resource_authority::PalmistryWatcherAdoptionAuthority>,
        prepared: Arc<PalmistryWatcherPreparedAdmission>,
        recheck: PalmistryWatcherOsRecheck,
        publish: P,
    ) -> Result<PalmistryWatcherAdoptionResult, SurrealPalmistryError>
    where
        P: FnOnce(&SurrealPalmistryVerifier) -> Result<(), String> + Send,
    {
        if !Arc::ptr_eq(authority, prepared.authorization.authority()) || authority.exact_scope() != &self.scope {
            return Err(SurrealPalmistryError::PermissionDenied);
        }
        prepared.validate_binding()?;
        let bindings = self.watcher_adoption_bindings(authority, &prepared.input)?;
        self.storage.revalidate_palmistry_watcher_adoption_authority(authority).await?;
        recheck().map_err(|_| SurrealPalmistryError::PermissionDenied)?;
        let operation_id = authority.operation_id();
        let rows = match self.storage.execute_watcher_admission(Arc::clone(&prepared)).await {
            Ok(rows) => rows,
            Err(error @ SurrealPalmistryError::OriginalCallback { .. }) => return Err(error),
            Err(_) => return Ok(PalmistryWatcherAdoptionResult::Uncertain {
                operation_id, canonical_receipt: None, diagnostic: "watcher admission controller outcome uncertain".into(),
            }),
        };
        let [receipt] = rows.as_slice() else {
            return Ok(PalmistryWatcherAdoptionResult::Uncertain {
                operation_id, canonical_receipt: None, diagnostic: "watcher admission canonical receipt unavailable".into(),
            });
        };
        let receipt = receipt.clone();
        let publication = self.storage.with_palmistry_watcher_adoption_readback(authority,
            self.read_watcher_adoption(bindings), |canonical| {
                if canonical.receipt != receipt { return Err(SurrealPalmistryError::CorruptAuthority); }
                prepared.validate_binding()?;
                recheck().map_err(|_| SurrealPalmistryError::PermissionDenied)?;
                publish(&prepared.input.verifier).map_err(|_| SurrealPalmistryError::PermissionDenied)
            },
        ).await;
        if publication.is_err() {
            return Ok(PalmistryWatcherAdoptionResult::Uncertain {
                operation_id, canonical_receipt: Some(receipt), diagnostic: "watcher admission protected publication uncertain".into(),
            });
        }
        Ok(PalmistryWatcherAdoptionResult::Applied(receipt))
    }

    pub async fn inspect_uncertain_watcher_adoption(
        &self,
        authority: &Arc<super::resource_authority::PalmistryWatcherAdoptionAuthority>,
        input: &PalmistryWatcherAdoptionInput,
        recheck: PalmistryWatcherOsRecheck,
    ) -> Result<PalmistryWatcherAdoptionResult, SurrealPalmistryError> {
        let bindings = self.watcher_adoption_bindings(authority, input)?;
        let operation_id = authority.operation_id();
        recheck().map_err(SurrealPalmistryError::NativeRecheck)?;
        let canonical = self.storage.with_palmistry_watcher_adoption_readback(
            authority, self.read_watcher_adoption(bindings), |row| {
                recheck().map_err(SurrealPalmistryError::NativeRecheck)?;
                Ok(row.receipt)
            },
        ).await;
        Ok(match canonical {
            Ok(receipt) => PalmistryWatcherAdoptionResult::Uncertain {
                operation_id, canonical_receipt: Some(receipt),
                diagnostic: "same-operation receipt found; original controller result is still required".into(),
            },
            Err(error) => PalmistryWatcherAdoptionResult::Uncertain {
                operation_id, canonical_receipt: None, diagnostic: error.to_string(),
            },
        })
    }

    async fn read_watcher_adoption(
        &self,
        bindings: WatcherAdoptionBindings,
    ) -> Result<WatcherAdoptionRow, SurrealPalmistryError> {
        let rows = self.storage.with_data_operation(|database| Box::pin(async move {
            database.query_values::<WatcherAdoptionRow, _>(WATCHER_ADOPTION_READ_SQL, bindings).await
        })).await?;
        let mut rows = rows.into_iter();
        let row = rows.next().ok_or(SurrealPalmistryError::CorruptAuthority)?;
        if rows.next().is_some() { return Err(SurrealPalmistryError::CorruptAuthority); }
        Ok(row)
    }

    fn watcher_adoption_bindings(
        &self,
        authority: &super::resource_authority::PalmistryWatcherAdoptionAuthority,
        input: &PalmistryWatcherAdoptionInput,
    ) -> Result<WatcherAdoptionBindings, SurrealPalmistryError> {
        Self::watcher_adoption_bindings_for_scope(&self.scope, authority, input)
    }

    fn watcher_adoption_bindings_for_scope(
        scope: &ExactResourceScopeAttribution,
        authority: &super::resource_authority::PalmistryWatcherAdoptionAuthority,
        input: &PalmistryWatcherAdoptionInput,
    ) -> Result<WatcherAdoptionBindings, SurrealPalmistryError> {
        validate_watcher_native_evidence(input)?;
        let verifier = &input.verifier;
        let start = &input.original_start;
        let ticks = i128::from(verifier.watcher_creation_time_100ns) - 116_444_736_000_000_000_i128;
        if authority.exact_scope() != scope || authority.operation_id().is_nil()
            || authority.watcher_id() != verifier.process_uuid
            || verifier.process_uuid.is_nil() || verifier.session_id.is_nil() || verifier.launch_nonce.is_nil()
            || verifier.parent_pid <= 0 || verifier.watcher_pid <= 0
            || verifier.watcher_creation_time_100ns < 0 || input.parent_creation_time_100ns < 0
            || start.process_uuid != verifier.process_uuid
            || start.os_pid.map(i64::from) != Some(verifier.watcher_pid)
            || start.parent_session_id.as_deref() != Some(verifier.session_id.to_string().as_str())
            || start.sandbox_adapter_id.as_deref() != Some("palmistry-watcher")
            || i128::from(start.started_at.timestamp()) != ticks.div_euclid(10_000_000)
            || i128::from(start.started_at.timestamp_subsec_nanos()) != ticks.rem_euclid(10_000_000) * 100
            || start.owner_role.trim().is_empty()
            || start.runtime_owner.is_none()
            || !start.sandbox_capabilities_snapshot.is_object() || !start.metadata_jsonb.is_object()
            || start.work_profile_id.as_deref().is_none_or(|value| value.trim().is_empty())
            || ["palmistry_original_start_hash", "palmistry_original_start", "palmistry_adoption_authority"].iter().any(|key| start.metadata_jsonb.get(*key).is_some())
            || verifier.executable_sha256.len() != 64 || verifier.verifying_key_hex.len() != 64
            || !verifier.executable_sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
            || !verifier.verifying_key_hex.bytes().all(|byte| byte.is_ascii_hexdigit())
            || input.custodian_registration_id.trim().is_empty()
            || !input.signed_native_evidence.is_object()
            || input.signed_native_evidence.get("schema_version").and_then(Value::as_u64) != Some(1)
            || start.metadata_jsonb.get("os_creation_time_100ns").and_then(Value::as_i64) != Some(verifier.watcher_creation_time_100ns)
            || start.metadata_jsonb.get("executable_sha256").and_then(Value::as_str) != Some(verifier.executable_sha256.as_str())
        { return Err(SurrealPalmistryError::InvalidInput); }
        let original_start = serde_json::to_value(start).map_err(|_| SurrealPalmistryError::InvalidInput)?;
        let original_start_hash = sha256_hex(serde_json::to_vec(&original_start).map_err(|_| SurrealPalmistryError::InvalidInput)?.as_slice());
        let start_engine_kind = serde_json::to_value(start.engine_kind.clone()).map_err(|_| SurrealPalmistryError::InvalidInput)?.as_str().ok_or(SurrealPalmistryError::InvalidInput)?.to_owned();
        let operation_id = authority.operation_id();
        let identity = serde_json::json!({
            "operation_id": operation_id, "process_uuid": verifier.process_uuid,
            "session_id": verifier.session_id, "launch_nonce": verifier.launch_nonce,
            "parent_pid": verifier.parent_pid, "watcher_pid": verifier.watcher_pid,
            "parent_creation_time_100ns": input.parent_creation_time_100ns,
            "watcher_creation_time_100ns": verifier.watcher_creation_time_100ns,
            "executable_sha256": verifier.executable_sha256, "verifying_key_hex": verifier.verifying_key_hex,
            "custodian_registration_id": input.custodian_registration_id,
            "original_start": original_start, "signed_native_evidence": input.signed_native_evidence,
        });
        let input_hash = sha256_hex(&serde_json::to_vec(&identity).map_err(|_| SurrealPalmistryError::InvalidInput)?);
        let signed = authority.record_user_scope();
        Ok(WatcherAdoptionBindings {
            permit_id: None, manifest_hash: None,
            operation_id, adoption_record_id: format!("palmistry-adopt-{operation_id}"),
            ledger_record_id: format!("evt-palmistry-adopt-{operation_id}"),
            start_record_id: verifier.process_uuid.to_string(),
            custodian_registration_id: input.custodian_registration_id.clone(),
            process_uuid: verifier.process_uuid, session_id: verifier.session_id, launch_nonce: verifier.launch_nonce,
            parent_pid: verifier.parent_pid, watcher_pid: verifier.watcher_pid,
            parent_creation_time_100ns: input.parent_creation_time_100ns,
            watcher_creation_time_100ns: verifier.watcher_creation_time_100ns,
            executable_sha256: verifier.executable_sha256.clone(), verifying_key_hex: verifier.verifying_key_hex.clone(),
            start_parent_process_id: start.parent_process_id,
            start_sandbox_internal_id: start.sandbox_internal_id.clone(), start_engine_kind,
            start_model_artifact_sha256: start.model_artifact_sha256.clone(),
            start_work_profile_id: start.work_profile_id.clone(), start_owner_role: start.owner_role.clone(),
            start_owner_wp: start.owner_wp.clone(), start_role_id: start.role_id.clone(),
            start_wp_id: start.wp_id.clone(), start_mt_id: start.mt_id.clone(),
            start_capabilities: start.sandbox_capabilities_snapshot.clone(),
            start_metadata: start.metadata_jsonb.clone(), event_payload: identity,
            original_started_at: start.started_at.to_rfc3339(), original_start_hash, original_start,
            signed_native_evidence: input.signed_native_evidence.clone(), input_hash,
            owner_account_id: scope.owner_account_id.to_string(),
            actor_principal_id: scope.actor_principal_id.to_string(),
            authenticated_session_id: scope.authenticated_session_id.to_string(),
            access_space_id: scope.access_space_id.to_string(), workspace_id: scope.workspace_id.as_str().to_owned(),
            authority_resource_id: signed.resource_id,
            authority_parent_resource_id: authority.workspace_parent().id.clone(),
            authority_grant_id: signed.grant_id.ok_or(SurrealPalmistryError::PermissionDenied)?,
        })
    }
}


pub(crate) async fn execute_prepared_watcher_admission_transaction(
    storage: &SurrealStorage,
    transaction: super::SurrealTransactionContext<'_>,
    prepared: Arc<PalmistryWatcherPreparedAdmission>,
    permit: PalmistryWatcherInstalledPermit,
) -> Result<Vec<PalmistryWatcherAdoptionReceipt>, SurrealPalmistryError> {
    prepared.validate_binding()?;
    permit.validate(&prepared)?;
    let authority = prepared.authorization.authority();
    authority.revalidate_in_transaction(storage, &transaction).await?;
    let mut bindings = SurrealPalmistryStore::watcher_adoption_bindings_for_scope(
        prepared.authorization.exact_scope(), authority, &prepared.input,
    )?;
    bindings.permit_id = Some(permit.permit_id);
    bindings.manifest_hash = Some(prepared.manifest.hash.clone());
    let rows = transaction.query_values_at::<WatcherAdoptionRow, _>(WATCHER_ADOPTION_SQL, bindings, 0).await?;
    authority.revalidate_in_transaction(storage, &transaction).await?;
    prepared.validate_binding()?;
    if rows.len() != 1 { return Err(SurrealPalmistryError::CorruptAuthority); }
    Ok(rows.into_iter().map(|row| row.receipt).collect())
}

const WATCHER_ADOPTION_SQL: &str = r#"RETURN {
    LET $permits = SELECT * FROM type::record('palmistry_watcher_operation_permit', $permit_id) WHERE state = 'issued' AND purpose = 'adopt' AND operation_id = $operation_id AND manifest_hash = $manifest_hash AND fn::wp012_palmistry_admission_actual_identity($this) LIMIT 2;
    IF array::len($permits) != 1 { THROW 'HSK-403-WATCHER-PERMIT'; };
    LET $reserved = UPDATE type::record('palmistry_watcher_operation_permit', $permit_id) SET state = 'consuming' WHERE state = 'issued' AND operation_id = $operation_id AND manifest_hash = $manifest_hash RETURN AFTER;
    IF array::len($reserved) != 1 { THROW 'HSK-403-WATCHER-PERMIT'; };
    LET $graph = {
LET $resource = type::record('protected_resources', $authority_resource_id);
LET $grant = type::record('resource_grants', $authority_grant_id);
IF !fn::wp012_palmistry_watcher_grant($resource, $authority_parent_resource_id, $grant, $auth.id) { THROW 'HSK-403-PROTECTED-RESOURCE'; };
LET $custodians = SELECT * FROM type::record('palmistry_watcher_custodian', $custodian_registration_id) WHERE process_uuid = $process_uuid AND diagnostic_session_id = $session_id AND launch_nonce = $launch_nonce AND parent_pid = $parent_pid AND watcher_pid = $watcher_pid AND parent_creation_time_100ns = $parent_creation_time_100ns AND watcher_creation_time_100ns = $watcher_creation_time_100ns AND executable_sha256 = $executable_sha256 AND verifying_key_hex = $verifying_key_hex AND original_started_at = type::datetime($original_started_at) AND original_start_hash = $original_start_hash AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND authority_resource_id = $resource AND authority_parent_resource_id = $authority_parent_resource_id LIMIT 2;
IF array::len($custodians) != 1 { THROW 'Palmistry original authenticated custodian is missing or differs'; };
LET $prior = SELECT true AS already_applied, { operation_id: operation_id, process_uuid: process_uuid, diagnostic_session_id: diagnostic_session_id, original_started_at: <string>original_started_at, custodian_registration_id: record::id(original_custodian), verifier_record_id: record::id(verifier), start_record_id: start_record_id, event_ledger_event_id: record::id(event_ledger_event_id), input_hash: input_hash } AS receipt FROM type::record('palmistry_watcher_adoption', $adoption_record_id) WHERE operation_id = $operation_id LIMIT 2;
IF array::len($prior) > 1 { THROW 'Palmistry adoption receipt is ambiguous'; } ELSE IF array::len($prior) = 1 {
    IF $prior[0].receipt.input_hash != $input_hash { THROW 'Palmistry same-operation input conflict'; };
    RETURN $prior;
} ELSE {
    LET $custodian = $custodians[0];
    LET $authorization = { owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id, authority_resource_id: $resource, authority_parent_resource_id: $authority_parent_resource_id, authority_grant_id: $grant, authority_session_id: $auth.id, authority_capability_id: 'diagnostics.watcher.adopt_existing', authority_action: 'reconcile' };
    LET $existing_verifier = SELECT * FROM type::record('palmistry_durable_verifier', $custodian.verifier_record_id) LIMIT 2;
    IF array::len($existing_verifier) > 1 { THROW 'Palmistry verifier is ambiguous'; };
    IF array::len($existing_verifier) = 1 AND ($existing_verifier[0].process_uuid != $process_uuid OR $existing_verifier[0].session_id != $session_id OR $existing_verifier[0].launch_nonce != $launch_nonce OR $existing_verifier[0].parent_pid != $parent_pid OR $existing_verifier[0].watcher_pid != $watcher_pid OR $existing_verifier[0].watcher_creation_time_100ns != $watcher_creation_time_100ns OR $existing_verifier[0].executable_sha256 != $executable_sha256 OR $existing_verifier[0].verifying_key_hex != $verifying_key_hex OR $existing_verifier[0].retired_at != NONE) { THROW 'Palmistry verifier identity conflict'; };
    LET $starts = SELECT * FROM kernel_process_lifecycle WHERE process_uuid = $process_uuid LIMIT 2;
    IF array::len($starts) > 1 { THROW 'Palmistry START is ambiguous'; };
    IF array::len($starts) = 1 AND ($starts[0].started_at != type::datetime($original_started_at) OR $starts[0].stopped_at != NONE OR !fn::wp012_palmistry_watcher_start_matches($starts[0], $custodian.original_start)) { THROW 'Palmistry original START differs'; };
    CREATE type::record('kernel_event_ledger', $ledger_record_id) CONTENT object::extend($authorization, {event_id: $ledger_record_id, event_version: '1', kernel_task_run_id: <string>$operation_id, session_run_id: <string>$session_id, aggregate_type: 'palmistry_watcher_adoption', aggregate_id: <string>$process_uuid, idempotency_key: 'palmistry-adopt:' + <string>$operation_id, event_type: 'PALMISTRY_WATCHER_ADOPTED', actor_kind: $auth.principal_id.actor_kind, actor_id: $auth.principal_id.actor_id, payload_hash: $input_hash, source_component: 'palmistry', payload: $event_payload, wsids: [$workspace_id], created_at: time::now()});
    IF array::len($existing_verifier) = 0 {
        CREATE type::record('palmistry_durable_verifier', $custodian.verifier_record_id) CONTENT {session_id: $session_id, launch_nonce: $launch_nonce, parent_pid: $parent_pid, watcher_pid: $watcher_pid, watcher_creation_time_100ns: $watcher_creation_time_100ns, process_uuid: $process_uuid, executable_sha256: $executable_sha256, verifying_key_hex: $verifying_key_hex, retired_at: NONE, event_ledger_event_id: type::record('kernel_event_ledger', $ledger_record_id), owner_account_id: $custodian.owner_account_id, actor_principal_id: $custodian.actor_principal_id, authenticated_session_id: $custodian.authenticated_session_id, access_space_id: $custodian.access_space_id, workspace_id: $custodian.workspace_id, storage_authority: 'embedded_surrealdb'};
    };
    IF array::len($starts) = 0 {
        CREATE type::record('kernel_process_lifecycle', $start_record_id) CONTENT { process_uuid: $process_uuid, os_pid: $watcher_pid, parent_session_id: <string>$session_id, parent_process_id: $start_parent_process_id, sandbox_adapter_id: 'palmistry-watcher', sandbox_internal_id: $start_sandbox_internal_id, engine_kind: $start_engine_kind, started_at: type::datetime($original_started_at), model_artifact_sha256: $start_model_artifact_sha256, work_profile_id: $start_work_profile_id, owner_role: $start_owner_role, owner_wp: $start_owner_wp, role_id: $start_role_id, wp_id: $start_wp_id, mt_id: $start_mt_id, sandbox_capabilities_snapshot: $start_capabilities, metadata: object::extend($start_metadata, {palmistry_original_start_hash: $original_start_hash, palmistry_original_start: $original_start, palmistry_adoption_authority: $authorization}) };
    };
    LET $start_id = IF array::len($starts) = 1 { record::id($starts[0].id) } ELSE { $start_record_id };
    CREATE type::record('palmistry_watcher_adoption', $adoption_record_id) CONTENT object::extend($authorization, {operation_id: $operation_id, process_uuid: $process_uuid, diagnostic_session_id: $session_id, launch_nonce: $launch_nonce, original_started_at: type::datetime($original_started_at), original_custodian: $custodian.id, verifier: type::record('palmistry_durable_verifier', $custodian.verifier_record_id), start_record_id: $start_id, event_ledger_event_id: type::record('kernel_event_ledger', $ledger_record_id), input_hash: $input_hash});
    RETURN [{already_applied: false, receipt: {operation_id: $operation_id, process_uuid: $process_uuid, diagnostic_session_id: $session_id, original_started_at: <string>type::datetime($original_started_at), custodian_registration_id: $custodian_registration_id, verifier_record_id: $custodian.verifier_record_id, start_record_id: $start_id, event_ledger_event_id: $ledger_record_id, input_hash: $input_hash}}];
};
    };
    LET $canonical = (SELECT true AS already_applied, { operation_id: operation_id, process_uuid: process_uuid, diagnostic_session_id: diagnostic_session_id, original_started_at: <string>original_started_at, custodian_registration_id: record::id(original_custodian), verifier_record_id: record::id(verifier), start_record_id: start_record_id, event_ledger_event_id: record::id(event_ledger_event_id), input_hash: input_hash } AS receipt FROM palmistry_watcher_adoption WHERE operation_id = $operation_id AND input_hash = $input_hash AND process_uuid = $process_uuid AND diagnostic_session_id = $session_id AND launch_nonce = $launch_nonce AND original_started_at = type::datetime($original_started_at) AND original_custodian = type::record('palmistry_watcher_custodian', $custodian_registration_id) AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND authority_resource_id = type::record('protected_resources', $authority_resource_id) AND authority_parent_resource_id = $authority_parent_resource_id AND authenticated_session_id = <string>record::id(authority_session_id) AND owner_account_id = <string>record::id(authority_session_id.account_id) AND actor_principal_id = <string>record::id(authority_session_id.principal_id) AND access_space_id = <string>record::id(authority_session_id.access_space_id) AND original_custodian.original_start_hash = $original_start_hash AND verifier.process_uuid = $process_uuid AND verifier.session_id = $session_id AND verifier.launch_nonce = $launch_nonce AND verifier.parent_pid = $parent_pid AND verifier.watcher_pid = $watcher_pid AND verifier.watcher_creation_time_100ns = $watcher_creation_time_100ns AND verifier.executable_sha256 = $executable_sha256 AND verifier.verifying_key_hex = $verifying_key_hex AND verifier.retired_at = NONE AND event_ledger_event_id.event_id = record::id(event_ledger_event_id) AND event_ledger_event_id.payload_hash = $input_hash AND event_ledger_event_id.payload = $event_payload AND event_ledger_event_id.source_component = 'palmistry' AND event_ledger_event_id.event_type = 'PALMISTRY_WATCHER_ADOPTED' AND event_ledger_event_id.payload.operation_id = <string>$operation_id AND start_record_id = (SELECT VALUE record::id(id) FROM kernel_process_lifecycle WHERE process_uuid = $process_uuid AND started_at = type::datetime($original_started_at) AND stopped_at = NONE AND fn::wp012_palmistry_watcher_start_matches($this, type::record('palmistry_watcher_custodian', $custodian_registration_id).original_start) LIMIT 2)[0] AND array::len(SELECT VALUE id FROM kernel_process_lifecycle WHERE process_uuid = $process_uuid AND started_at = type::datetime($original_started_at) AND stopped_at = NONE AND fn::wp012_palmistry_watcher_start_matches($this, type::record('palmistry_watcher_custodian', $custodian_registration_id).original_start) LIMIT 2) = 1 LIMIT 2);
    IF array::len($graph) != 1 OR array::len($canonical) != 1 OR $graph[0].receipt != $canonical[0].receipt { THROW 'HSK-403-WATCHER-GRAPH'; };
    LET $finalized = UPDATE type::record('palmistry_watcher_operation_permit', $permit_id) SET state = 'finalized' WHERE state = 'consuming' AND operation_id = $operation_id AND manifest_hash = $manifest_hash AND fn::wp012_palmistry_admission_actual_identity($this) RETURN AFTER;
    IF array::len($finalized) != 1 { THROW 'HSK-403-WATCHER-PERMIT'; };
    RETURN $graph;
};"#;const WATCHER_ADOPTION_READ_SQL: &str = r#"SELECT true AS already_applied, { operation_id: operation_id, process_uuid: process_uuid, diagnostic_session_id: diagnostic_session_id, original_started_at: <string>original_started_at, custodian_registration_id: record::id(original_custodian), verifier_record_id: record::id(verifier), start_record_id: start_record_id, event_ledger_event_id: record::id(event_ledger_event_id), input_hash: input_hash } AS receipt FROM palmistry_watcher_adoption WHERE operation_id = $operation_id AND input_hash = $input_hash AND process_uuid = $process_uuid AND diagnostic_session_id = $session_id AND launch_nonce = $launch_nonce AND original_started_at = type::datetime($original_started_at) AND original_custodian = type::record('palmistry_watcher_custodian', $custodian_registration_id) AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND authority_resource_id = type::record('protected_resources', $authority_resource_id) AND authority_parent_resource_id = $authority_parent_resource_id AND authenticated_session_id = <string>record::id(authority_session_id) AND owner_account_id = <string>record::id(authority_session_id.account_id) AND actor_principal_id = <string>record::id(authority_session_id.principal_id) AND access_space_id = <string>record::id(authority_session_id.access_space_id) AND original_custodian.original_start_hash = $original_start_hash AND verifier.process_uuid = $process_uuid AND verifier.session_id = $session_id AND verifier.launch_nonce = $launch_nonce AND verifier.parent_pid = $parent_pid AND verifier.watcher_pid = $watcher_pid AND verifier.watcher_creation_time_100ns = $watcher_creation_time_100ns AND verifier.executable_sha256 = $executable_sha256 AND verifier.verifying_key_hex = $verifying_key_hex AND verifier.retired_at = NONE AND event_ledger_event_id.event_id = record::id(event_ledger_event_id) AND event_ledger_event_id.payload_hash = $input_hash AND event_ledger_event_id.payload = $event_payload AND event_ledger_event_id.source_component = 'palmistry' AND event_ledger_event_id.event_type = 'PALMISTRY_WATCHER_ADOPTED' AND event_ledger_event_id.payload.operation_id = <string>$operation_id AND start_record_id = (SELECT VALUE record::id(id) FROM kernel_process_lifecycle WHERE process_uuid = $process_uuid AND started_at = type::datetime($original_started_at) AND stopped_at = NONE AND fn::wp012_palmistry_watcher_start_matches($this, type::record('palmistry_watcher_custodian', $custodian_registration_id).original_start) LIMIT 2)[0] AND array::len(SELECT VALUE id FROM kernel_process_lifecycle WHERE process_uuid = $process_uuid AND started_at = type::datetime($original_started_at) AND stopped_at = NONE AND fn::wp012_palmistry_watcher_start_matches($this, type::record('palmistry_watcher_custodian', $custodian_registration_id).original_start) LIMIT 2) = 1 AND array::len(SELECT VALUE id FROM palmistry_watcher_operation_permit WHERE operation_id = $operation_id AND purpose = 'adopt' AND state = 'finalized' AND manifest.input_hash = $input_hash AND manifest.event_payload = $event_payload AND manifest.original_start = $original_start AND authority_resource_id = type::record('protected_resources', $authority_resource_id) AND authority_parent_resource_id = $authority_parent_resource_id LIMIT 2) = 1 LIMIT 2;"#;

fn validate_watcher_native_evidence(input: &PalmistryWatcherAdoptionInput) -> Result<(), SurrealPalmistryError> {
    let evidence = &input.signed_native_evidence;
    let invalid = || SurrealPalmistryError::InvalidInput;
    if evidence.get("schema_id").and_then(Value::as_str) != Some("handshake.palmistry.watcher-adoption-native-evidence")
        || evidence.get("schema_version").and_then(Value::as_u64) != Some(1) {
        return Err(invalid());
    }
    let decode = |field: &str| -> Result<Vec<u8>, SurrealPalmistryError> {
        hex::decode(evidence.get(field).and_then(Value::as_str).ok_or_else(invalid)?).map_err(|_| invalid())
    };
    if evidence.get("reply_json_bytes_hex").and_then(Value::as_str).is_none_or(|value| value.len() > 32 * 1024) { return Err(invalid()); }
    let challenge = decode("challenge_wire_hex")?;
    let reply = decode("reply_json_bytes_hex")?;
    if challenge.last() != Some(&b'\n') || reply.is_empty() || reply.len() > 16 * 1024 || reply.last() == Some(&b'\n') {
        return Err(invalid());
    }
    for field in ["fresh_challenge_hex", "expected_verifier_hex", "transport_public_key_hex"] {
        if decode(field)?.len() != 32 { return Err(invalid()); }
    }
    if decode("expected_verifier_hex")? != hex::decode(&input.verifier.verifying_key_hex).map_err(|_| invalid())? {
        return Err(invalid());
    }
    let readiness = evidence.get("original_readiness").ok_or_else(invalid)?;
    for field in ["challenge", "signature"] {
        if readiness.get(field).is_none_or(Value::is_null) { return Err(invalid()); }
    }
    let observed = evidence.get("observed_os").ok_or_else(invalid)?;
    for (field, expected) in [
        ("watcher_creation_time_100ns", input.verifier.watcher_creation_time_100ns),
        ("parent_creation_time_100ns", input.parent_creation_time_100ns),
    ] {
        if observed.get(field).and_then(Value::as_str).and_then(|value| value.parse::<i64>().ok()) != Some(expected) {
            return Err(invalid());
        }
    }
    Ok(())
}
