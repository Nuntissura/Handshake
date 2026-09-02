use std::sync::Arc;

use serde_json::Value;
use sha2::{Digest, Sha256};
use surrealdb::types::SurrealValue;
use thiserror::Error;
use tokio::sync::OnceCell;
use uuid::Uuid;

use super::{SurrealStorage, SurrealStorageError};
use crate::{
    kernel::NewKernelEvent,
    swarm_orchestration::resource_scope::ExactResourceScopeAttribution,
};

const SCHEMA: &str = include_str!("palmistry_schema.surql");
const SCHEMA_STATE: &str = "\
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
}

#[derive(Debug, SurrealValue)]
struct MutationResult {
    outcome: String,
}

#[derive(Debug, SurrealValue)]
struct EventIdResult {
    event_id: String,
}

pub async fn bootstrap_palmistry_schema(
    storage: &SurrealStorage,
) -> Result<(), SurrealStorageError> {
    super::bootstrap_schema(storage).await?;
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

    pub async fn stop_is_durable(
        &self,
        process_uuid: Uuid,
    ) -> Result<bool, SurrealPalmistryError> {
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

    pub async fn append_kernel_event(
        &self,
        event: NewKernelEvent,
    ) -> Result<String, SurrealPalmistryError> {
        self.ensure_initialized().await?;
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
        };
        let rows = self.storage.with_data_operation(|database| Box::pin(async move {
            database.query_values_at::<EventIdResult, _>(
                "BEGIN TRANSACTION;\
                 LET $prior = (SELECT event_id, payload_hash, aggregate_type, aggregate_id, event_type, source_component FROM type::record('kernel_event_ledger', $ledger_record_id) WHERE idempotency_key = $idempotency_key AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2);\
                 IF array::len($prior) > 1 { THROW 'Palmistry EventLedger authority is ambiguous'; } ELSE IF array::len($prior) = 1 { IF $prior[0].payload_hash != $payload_hash OR $prior[0].aggregate_type != $aggregate_type OR $prior[0].aggregate_id != $aggregate_id OR $prior[0].event_type != $event_type OR $prior[0].source_component != $source_component { THROW 'Palmistry EventLedger idempotency conflict'; }; RETURN $prior; } ELSE { RETURN CREATE type::record('kernel_event_ledger', $ledger_record_id) CONTENT { event_id: $event_id, event_version: $event_version, kernel_task_run_id: $kernel_task_run_id, session_run_id: $session_run_id, aggregate_type: $aggregate_type, aggregate_id: $aggregate_id, idempotency_key: $idempotency_key, event_type: $event_type, actor_kind: $actor_kind, actor_id: $actor_id, causation_id: $causation_id, correlation_id: $correlation_id, payload_hash: $payload_hash, source_component: $source_component, payload: $payload, owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, workspace_id: $workspace_id, created_at: time::now() }; };\
                 COMMIT TRANSACTION;",
                bindings,
                2,
            ).await
        })).await?;
        let [row] = rows.as_slice() else {
            return Err(SurrealPalmistryError::CorruptAuthority);
        };
        Ok(row.event_id.clone())
    }

    fn session_bindings(&self, session_id: Uuid) -> SessionBindings {
        SessionBindings {
            session_id,
            owner_account_id: self.scope.owner_account_id.as_uuid().to_string(),
            actor_principal_id: self.scope.actor_principal_id.as_uuid().to_string(),
            authenticated_session_id: self
                .scope
                .authenticated_session_id
                .as_uuid()
                .to_string(),
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
            idempotency_key: format!("palmistry-verifier-retired:{}", self.identity_hash(&identity)),
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
