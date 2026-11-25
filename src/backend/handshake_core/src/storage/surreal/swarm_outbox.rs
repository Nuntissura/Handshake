use std::sync::Arc;

use sha2::{Digest, Sha256};
use surrealdb::types::{RecordId, SurrealValue};
use thiserror::Error;
use tokio::sync::OnceCell;

use super::{SurrealStorage, SurrealStorageError};
use crate::swarm_orchestration::resource_scope::{ExactResourceScopeAttribution, ResourceScope};

const SCHEMA: &str = include_str!("swarm_outbox_schema.surql");
pub(super) const SCHEMA_STATE: &str = "\
DEFINE TABLE IF NOT EXISTS swarm_outbox_schema_state SCHEMAFULL;\
DEFINE FIELD IF NOT EXISTS schema_version ON swarm_outbox_schema_state TYPE string;\
DEFINE FIELD IF NOT EXISTS schema_revision ON swarm_outbox_schema_state TYPE int;\
DEFINE FIELD IF NOT EXISTS apply_state ON swarm_outbox_schema_state TYPE string;";
const SCHEMA_VERSION: &str = "mt003-swarm-terminal-outbox-v1";
const SCHEMA_REVISION: i64 = 1;
const PERSIST_EVENT: &str = r#"
BEGIN TRANSACTION;
LET $existing = (
    SELECT id, event_id, event_json, event_json_sha256, event_ledger_event_id, storage_authority
    FROM type::record('swarm_terminal_event_outbox', $record_id)
    WHERE owner_account_id = $owner_account_id
      AND actor_principal_id = $actor_principal_id
      AND authenticated_session_id = $authenticated_session_id
      AND access_space_id = $access_space_id
      AND workspace_id = $workspace_id
    LIMIT 2
);
LET $prior = (
    SELECT id, event_id, event_version, idempotency_key, source_component,
        event_type, aggregate_type, aggregate_id, payload_hash, payload
    FROM type::record('kernel_event_ledger', $ledger_record_id)
    WHERE owner_account_id = $owner_account_id
      AND actor_principal_id = $actor_principal_id
      AND authenticated_session_id = $authenticated_session_id
      AND access_space_id = $access_space_id
      AND workspace_id = $workspace_id
    LIMIT 2
);
LET $queued = (
    SELECT VALUE id FROM swarm_terminal_event_outbox
    WHERE owner_account_id = $owner_account_id
      AND actor_principal_id = $actor_principal_id
      AND authenticated_session_id = $authenticated_session_id
      AND access_space_id = $access_space_id
      AND workspace_id = $workspace_id
);
IF array::len($existing) > 1 OR array::len($prior) > 1 {
    RETURN { outcome: 'corrupt' };
} ELSE IF array::len($existing) = 1 OR array::len($prior) = 1 {
    IF array::len($prior) != 1 { RETURN { outcome: 'corrupt' }; };
    LET $receipt = $prior[0];
    IF !type::is_string($receipt.payload.event_json)
        OR !type::is_string($receipt.payload_hash)
    { RETURN { outcome: 'corrupt' }; };
    IF $receipt.id != type::record('kernel_event_ledger', $ledger_record_id)
        OR $receipt.event_id != $ledger_record_id
        OR $receipt.event_version != 'kernel_event_v1'
        OR $receipt.idempotency_key != $idempotency_key
        OR $receipt.source_component != 'swarm_terminal_outbox'
        OR $receipt.event_type != 'SWARM_TERMINAL_EVENT_ENQUEUED'
        OR $receipt.aggregate_type != 'swarm_terminal_event'
        OR $receipt.aggregate_id != $event_id
        OR $receipt.payload != { event_id: $event_id, event_json: $receipt.payload.event_json }
        OR $receipt.payload_hash != crypto::sha256($receipt.payload.event_json)
    { RETURN { outcome: 'corrupt' }; };
    IF array::len($existing) = 1 {
        IF !type::is_string($existing[0].event_json)
            OR !type::is_string($existing[0].event_json_sha256)
        { RETURN { outcome: 'corrupt' }; };
        IF $existing[0].id != type::record('swarm_terminal_event_outbox', $record_id)
            OR $existing[0].event_id != $event_id
            OR $existing[0].storage_authority != 'embedded_surrealdb'
            OR $existing[0].event_ledger_event_id != $receipt.id
            OR $existing[0].event_json != $receipt.payload.event_json
            OR $existing[0].event_json_sha256 != crypto::sha256($existing[0].event_json)
            OR $existing[0].event_json_sha256 != $receipt.payload_hash
        { RETURN { outcome: 'corrupt' }; };
        IF $stable_operation {
            RETURN { outcome: 'already_queued', original_event_json: $receipt.payload.event_json };
        };
        RETURN { outcome: IF $existing[0].event_json_sha256 = $event_json_sha256
            AND $existing[0].event_json = $event_json { 'already_queued' } ELSE { 'conflict' } };
    };
    IF !$unfiltered_owner { RETURN { outcome: 'corrupt' }; };
    LET $canonical_occupancy = (
        SELECT VALUE id FROM type::record('swarm_terminal_event_outbox', $record_id)
    );
    LET $other_event_occupancy = (
        SELECT VALUE id FROM swarm_terminal_event_outbox
        WHERE owner_account_id = $owner_account_id
            AND actor_principal_id = $actor_principal_id
            AND authenticated_session_id = $authenticated_session_id
            AND access_space_id = $access_space_id
            AND workspace_id = $workspace_id
            AND event_id = $event_id
            AND id != type::record('swarm_terminal_event_outbox', $record_id)
        LIMIT 1
    );
    IF array::len($canonical_occupancy) != 0 OR array::len($other_event_occupancy) != 0 {
        RETURN { outcome: 'corrupt' };
    };
    IF $stable_operation {
        RETURN { outcome: 'already_committed', original_event_json: $receipt.payload.event_json };
    };
    RETURN { outcome: IF $receipt.payload_hash = $event_json_sha256
        AND $receipt.payload.event_json = $event_json { 'already_committed' } ELSE { 'conflict' } };
} ELSE IF array::len($queued) >= $capacity {
    RETURN { outcome: 'capacity' };
} ELSE {
    LET $ledger = CREATE type::record('kernel_event_ledger', $ledger_record_id) CONTENT {
        event_id: $ledger_record_id,
        event_version: 'kernel_event_v1',
        kernel_task_run_id: $workspace_id,
        wsids: [$workspace_id],
        session_run_id: $authenticated_session_id,
        aggregate_type: 'swarm_terminal_event',
        aggregate_id: $event_id,
        idempotency_key: $idempotency_key,
        event_type: 'SWARM_TERMINAL_EVENT_ENQUEUED',
        actor_kind: 'principal',
        actor_id: $actor_principal_id,
        causation_id: NONE,
        correlation_id: NONE,
        payload_hash: $event_json_sha256,
        source_component: 'swarm_terminal_outbox',
        payload: { event_id: $event_id, event_json: $event_json },
        owner_account_id: $owner_account_id,
        actor_principal_id: $actor_principal_id,
        authenticated_session_id: $authenticated_session_id,
        access_space_id: $access_space_id,
        workspace_id: $workspace_id,
        created_at: time::now()
    } RETURN VALUE id;
    IF array::len($ledger) != 1 OR $ledger[0] != type::record('kernel_event_ledger', $ledger_record_id) {
        THROW 'swarm outbox receipt creation was not confirmed';
    };
    LET $outbox = CREATE type::record('swarm_terminal_event_outbox', $record_id) CONTENT {
        event_id: $event_id,
        event_json: $event_json,
        event_json_sha256: $event_json_sha256,
        event_ledger_event_id: type::record('kernel_event_ledger', $ledger_record_id),
        owner_account_id: $owner_account_id,
        actor_principal_id: $actor_principal_id,
        authenticated_session_id: $authenticated_session_id,
        access_space_id: $access_space_id,
        workspace_id: $workspace_id,
        attempts: 0,
        last_error: NONE,
        created_at: time::now(),
        last_attempt_at: NONE,
        storage_authority: 'embedded_surrealdb'
    } RETURN VALUE id;
    IF array::len($outbox) != 1 OR $outbox[0] != type::record('swarm_terminal_event_outbox', $record_id) {
        THROW 'swarm outbox row creation was not confirmed';
    };
    RETURN { outcome: 'stored' };
};
COMMIT TRANSACTION;
"#;

const READ_PERSISTED_ENQUEUE: &str = r#"
SELECT payload.event_json AS original_event_json, payload_hash AS original_event_json_sha256,
    array::len((
        SELECT VALUE id FROM type::record('swarm_terminal_event_outbox', $record_id)
    )) AS canonical_count,
    array::len((
        SELECT VALUE id FROM swarm_terminal_event_outbox
        WHERE owner_account_id = $owner_account_id
            AND actor_principal_id = $actor_principal_id
            AND authenticated_session_id = $authenticated_session_id
            AND access_space_id = $access_space_id
            AND workspace_id = $workspace_id
            AND event_id = $event_id
            AND id != type::record('swarm_terminal_event_outbox', $record_id)
        LIMIT 1
    )) AS other_event_count,
    (SELECT id, event_id, event_json, event_json_sha256, event_ledger_event_id
        FROM type::record('swarm_terminal_event_outbox', $record_id)
        WHERE owner_account_id = $owner_account_id
            AND actor_principal_id = $actor_principal_id
            AND authenticated_session_id = $authenticated_session_id
            AND access_space_id = $access_space_id
            AND workspace_id = $workspace_id
            AND event_id = $event_id
            AND storage_authority = 'embedded_surrealdb'
            AND event_ledger_event_id = type::record('kernel_event_ledger', $ledger_record_id)
    ) AS queued
FROM type::record('kernel_event_ledger', $ledger_record_id)
WHERE id = type::record('kernel_event_ledger', $ledger_record_id)
    AND event_id = $ledger_record_id
    AND event_version = 'kernel_event_v1'
    AND owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
    AND source_component = 'swarm_terminal_outbox'
    AND event_type = 'SWARM_TERMINAL_EVENT_ENQUEUED'
    AND aggregate_type = 'swarm_terminal_event'
    AND aggregate_id = $event_id
    AND idempotency_key = $idempotency_key
    AND type::is_string(payload.event_json)
    AND type::is_string(payload_hash)
    AND payload = { event_id: $event_id, event_json: payload.event_json }
    AND payload_hash = crypto::sha256(payload.event_json);
"#;

const CONFIRM_EXACT_FINALIZATION: &str = r#"
SELECT VALUE true FROM type::record('kernel_event_ledger', $ledger_record_id)
WHERE id = type::record('kernel_event_ledger', $ledger_record_id)
    AND event_id = $ledger_record_id
    AND event_version = 'kernel_event_v1'
    AND owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
    AND source_component = 'swarm_terminal_outbox'
    AND event_type = 'SWARM_TERMINAL_EVENT_ENQUEUED'
    AND aggregate_type = 'swarm_terminal_event'
    AND aggregate_id = $event_id
    AND idempotency_key = $idempotency_key
    AND payload_hash = $event_json_sha256
    AND payload = { event_id: $event_id, event_json: $event_json }
    AND array::len((
        SELECT VALUE id FROM type::record('swarm_terminal_event_outbox', $record_id)
    )) = 0
    AND array::len((
        SELECT VALUE id FROM swarm_terminal_event_outbox
        WHERE owner_account_id = $owner_account_id
            AND actor_principal_id = $actor_principal_id
            AND authenticated_session_id = $authenticated_session_id
            AND access_space_id = $access_space_id
            AND workspace_id = $workspace_id
            AND event_id = $event_id
        LIMIT 1
    )) = 0;
"#;

macro_rules! exact_outbox_cas {
    ($operation:literal) => {
        concat!(
            $operation,
            r#"
                WHERE event_id = $event_id
                  AND owner_account_id = $owner_account_id
                  AND actor_principal_id = $actor_principal_id
                  AND authenticated_session_id = $authenticated_session_id
                  AND access_space_id = $access_space_id
                  AND workspace_id = $workspace_id
                  AND storage_authority = 'embedded_surrealdb'
                  AND event_json = $event_json
                  AND event_json_sha256 = $event_json_sha256
                  AND event_ledger_event_id = type::record('kernel_event_ledger', $ledger_record_id)
                  AND event_ledger_event_id.event_id = $ledger_record_id
                  AND event_ledger_event_id.event_version = 'kernel_event_v1'
                  AND event_ledger_event_id.owner_account_id = $owner_account_id
                  AND event_ledger_event_id.actor_principal_id = $actor_principal_id
                  AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id
                  AND event_ledger_event_id.access_space_id = $access_space_id
                  AND event_ledger_event_id.workspace_id = $workspace_id
                  AND event_ledger_event_id.source_component = 'swarm_terminal_outbox'
                  AND event_ledger_event_id.event_type = 'SWARM_TERMINAL_EVENT_ENQUEUED'
                  AND event_ledger_event_id.aggregate_type = 'swarm_terminal_event'
                  AND event_ledger_event_id.aggregate_id = $event_id
                  AND event_ledger_event_id.idempotency_key = $idempotency_key
                  AND event_ledger_event_id.payload_hash = $event_json_sha256
                  AND event_ledger_event_id.payload = { event_id: $event_id, event_json: $event_json }
                RETURN VALUE id;
            "#
        )
    };
}

#[derive(Debug, Error)]
pub enum SurrealSwarmOutboxError {
    #[error("embedded swarm outbox storage failed: {0}")]
    Storage(#[from] SurrealStorageError),
    #[error("swarm outbox requires an exact five-field ResourceScope")]
    IncompleteScope,
    #[error("swarm outbox event identity or payload is invalid")]
    InvalidEvent,
    #[error("swarm outbox event id was reused for different content")]
    IdempotencyConflict,
    #[error("swarm outbox reached bounded capacity {capacity}")]
    CapacityExceeded { capacity: usize },
    #[error("swarm outbox durable authority is ambiguous or corrupt")]
    CorruptAuthority,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurrealSwarmOutboxPersistOutcome {
    Stored,
    AlreadyQueued,
    AlreadyCommitted,
}

#[derive(Clone)]
pub struct SurrealSwarmOutboxStore {
    storage: SurrealStorage,
    scope: ExactResourceScopeAttribution,
    initialized: Arc<OnceCell<()>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurrealSwarmOutboxEvent {
    pub event_id: String,
    pub event_json: String,
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

#[derive(Debug, Clone, SurrealValue)]
struct PersistBindings {
    stable_operation: bool,
    unfiltered_owner: bool,
    record_id: String,
    ledger_record_id: String,
    event_id: String,
    event_json: String,
    event_json_sha256: String,
    idempotency_key: String,
    capacity: i64,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(Debug, Clone, SurrealValue)]
struct ExactEventBindings {
    record_id: String,
    ledger_record_id: String,
    event_id: String,
    event_json: String,
    event_json_sha256: String,
    idempotency_key: String,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
    last_error: String,
}

#[derive(Debug, SurrealValue)]
struct EventBindings {
    record_id: String,
    event_id: String,
    last_error: String,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(Debug, SurrealValue)]
struct ScopeBindings {
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(Debug, SurrealValue)]
struct PersistResult {
    outcome: String,
    original_event_json: Option<String>,
}

#[derive(Debug, SurrealValue)]
struct PersistReadback {
    original_event_json: String,
    original_event_json_sha256: String,
    canonical_count: i64,
    other_event_count: i64,
    queued: Vec<PersistQueuedReadback>,
}

#[derive(Debug, SurrealValue)]
struct PersistQueuedReadback {
    id: RecordId,
    event_id: String,
    event_json: String,
    event_json_sha256: String,
    event_ledger_event_id: RecordId,
}

#[derive(Debug, SurrealValue)]
struct PendingRow {
    id: RecordId,
    event_id: String,
    event_json: String,
    event_json_sha256: Option<String>,
    receipt: Option<PendingReceipt>,
}

#[derive(Debug, SurrealValue)]
struct PendingReceipt {
    receipt_record: Option<RecordId>,
    receipt_event_id: Option<String>,
    receipt_event_version: Option<String>,
    receipt_idempotency_key: Option<String>,
    receipt_aggregate_id: Option<String>,
    receipt_event_type: Option<String>,
    receipt_payload_hash: Option<String>,
    receipt_payload: Option<serde_json::Value>,
}

// FINAL catalog reuse requires the owning durable receipt; this never installs DDL.
pub(super) async fn verify_schema_receipt(
    database: &super::SurrealAdminContext<'_>,
) -> Result<(), SurrealStorageError> {
    let mut response = database.query("SELECT * FROM ONLY swarm_outbox_schema_state:primary;").await?;
    let state: Option<SchemaState> = response.take(0)?;
    if !state.as_ref().is_some_and(|state| {
        state.schema_version == SCHEMA_VERSION
            && state.schema_revision == SCHEMA_REVISION
            && state.apply_state == "complete"
    }) {
        return Err(SurrealStorageError::InvalidModelLaneRecord {
            reason: "swarm outbox schema receipt missing or version/revision/state mismatch",
        });
    }
    Ok(())
}

pub async fn bootstrap_swarm_outbox_schema(
    storage: &SurrealStorage,
) -> Result<(), SurrealStorageError> {
    super::bootstrap_production_schemas(storage).await?;
    Ok(())
}

// Production composition calls this only after its verified shared prerequisite.
pub(super) async fn install_swarm_outbox_schema(storage: &SurrealStorage) -> Result<(), SurrealStorageError> {
    storage
        .with_admin_operation(|database| {
            Box::pin(async move {
                database.query(SCHEMA_STATE).await?;
                let mut response = database
                    .query("SELECT * FROM ONLY swarm_outbox_schema_state:primary;")
                    .await?;
                let state: Option<SchemaState> = response.take(0)?;
                if state.as_ref().is_some_and(|state| {
                    state.schema_version != SCHEMA_VERSION
                        || state.schema_revision != SCHEMA_REVISION
                        || state.apply_state != "complete"
                }) {
                    return Err(SurrealStorageError::InvalidModelLaneRecord {
                        reason: "swarm outbox schema state version/revision mismatch",
                    });
                }
                database.query(SCHEMA).await?;
                if state.is_none() {
                    database
                        .query_bound(
                            "UPSERT swarm_outbox_schema_state:primary CONTENT { schema_version: $schema_version, schema_revision: $schema_revision, apply_state: 'complete' };",
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

impl SurrealSwarmOutboxStore {
    pub fn new_exact(storage: SurrealStorage, scope: ExactResourceScopeAttribution) -> Self {
        Self {
            storage,
            scope,
            initialized: Arc::new(OnceCell::new()),
        }
    }

    pub fn new(
        storage: SurrealStorage,
        resource_scope: ResourceScope,
    ) -> Result<Self, SurrealSwarmOutboxError> {
        let scope = ExactResourceScopeAttribution::try_from_resource_scope(&resource_scope)
            .map_err(|_| SurrealSwarmOutboxError::IncompleteScope)?;
        Ok(Self::new_exact(storage, scope))
    }

    pub(crate) fn storage_and_scope(&self) -> (&SurrealStorage, &ExactResourceScopeAttribution) {
        (&self.storage, &self.scope)
    }

    /// Prepare this store before a worker begins accepting timed events.
    pub async fn prepare(&self) -> Result<(), SurrealSwarmOutboxError> {
        self.initialized
            .get_or_try_init(|| async { bootstrap_swarm_outbox_schema(&self.storage).await })
            .await?;
        Ok(())
    }

    pub async fn persist(
        &self,
        event_id: &str,
        event_json: String,
        capacity: usize,
    ) -> Result<SurrealSwarmOutboxPersistOutcome, SurrealSwarmOutboxError> {
        self.persist_inner(event_id, event_json, capacity, None).await
    }

    pub(crate) async fn persist_stable_terminal(
        &self,
        operation: &crate::swarm_orchestration::events::StableTerminalEvent,
        capacity: usize,
    ) -> Result<SurrealSwarmOutboxPersistOutcome, SurrealSwarmOutboxError> {
        let event = operation.event();
        let event_json = serde_json::to_string(event)
            .map_err(|_| SurrealSwarmOutboxError::InvalidEvent)?;
        self.persist_inner(&event.event_id.to_string(), event_json, capacity, Some(operation)).await
    }

    async fn persist_inner(
        &self,
        event_id: &str,
        event_json: String,
        capacity: usize,
        stable_operation: Option<&crate::swarm_orchestration::events::StableTerminalEvent>,
    ) -> Result<SurrealSwarmOutboxPersistOutcome, SurrealSwarmOutboxError> {
        self.prepare().await?;
        if event_id.trim().is_empty()
            || event_json.trim().is_empty()
            || serde_json::from_str::<serde_json::Value>(&event_json).is_err()
            || capacity == 0
        {
            return Err(SurrealSwarmOutboxError::InvalidEvent);
        }
        let event_json_sha256 = sha256_hex(event_json.as_bytes());
        let identity = self.identity_hash(event_id);
        let unfiltered_owner = super::current_record_user_scope().is_none();
        let bindings = PersistBindings {
            stable_operation: stable_operation.is_some(),
            unfiltered_owner,
            record_id: format!("swarm-outbox-{identity}"),
            ledger_record_id: format!("evt-swarm-terminal-{identity}"),
            event_id: event_id.to_owned(),
            event_json,
            event_json_sha256,
            idempotency_key: format!("swarm-terminal:{identity}"),
            capacity: i64::try_from(capacity).unwrap_or(i64::MAX),
            owner_account_id: self.scope.owner_account_id.as_uuid().to_string(),
            actor_principal_id: self.scope.actor_principal_id.as_uuid().to_string(),
            authenticated_session_id: self.scope.authenticated_session_id.as_uuid().to_string(),
            access_space_id: self.scope.access_space_id.as_uuid().to_string(),
            workspace_id: self.scope.workspace_id.as_str().to_owned(),
        };
        let readback = bindings.clone();
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values_at::<PersistResult, _>(PERSIST_EVENT, bindings, 4)
                        .await
                })
            })
            .await;
        let rows = match rows {
            Ok(rows) => rows,
            Err(error) => {
                let ambiguous = matches!(&error, SurrealStorageError::StatementTimeout { .. });
                let original = SurrealSwarmOutboxError::Storage(error);
                if ambiguous && unfiltered_owner && super::current_record_user_scope().is_none() {
                    if let Some(outcome) = self.read_persisted_enqueue(readback, stable_operation).await {
                        return Ok(outcome);
                    }
                }
                return Err(original);
            }
        };
        if rows.len() != 1 { return Err(SurrealSwarmOutboxError::CorruptAuthority); }
        let result = rows.into_iter().next().ok_or(SurrealSwarmOutboxError::CorruptAuthority)?;
        if let Some(operation) = stable_operation {
            if matches!(result.outcome.as_str(), "already_queued" | "already_committed") {
                let original = result.original_event_json.as_deref()
                    .ok_or(SurrealSwarmOutboxError::CorruptAuthority)?;
                if !operation.matches_original(original)
                    .map_err(|_| SurrealSwarmOutboxError::CorruptAuthority)?
                {
                    return Err(SurrealSwarmOutboxError::IdempotencyConflict);
                }
            }
        }
        match result.outcome.as_str() {
            "stored" => Ok(SurrealSwarmOutboxPersistOutcome::Stored),
            "already_queued" => Ok(SurrealSwarmOutboxPersistOutcome::AlreadyQueued),
            "already_committed" => Ok(SurrealSwarmOutboxPersistOutcome::AlreadyCommitted),
            "conflict" => Err(SurrealSwarmOutboxError::IdempotencyConflict),
            "capacity" => Err(SurrealSwarmOutboxError::CapacityExceeded { capacity }),
            _ => Err(SurrealSwarmOutboxError::CorruptAuthority),
        }
    }

    async fn read_persisted_enqueue(
        &self,
        bindings: PersistBindings,
        stable_operation: Option<&crate::swarm_orchestration::events::StableTerminalEvent>,
    ) -> Option<SurrealSwarmOutboxPersistOutcome> {
        let original = bindings.clone();
        let rows = self.storage.with_data_operation(|database| Box::pin(async move {
            database.query_values::<PersistReadback, _>(READ_PERSISTED_ENQUEUE, bindings).await
        })).await.ok()?;
        if rows.len() != 1 { return None; }
        let row = rows.into_iter().next()?;
        if let Some(operation) = stable_operation {
            if !operation.matches_original(&row.original_event_json).ok()? { return None; }
        } else if row.original_event_json != original.event_json
            || row.original_event_json_sha256 != original.event_json_sha256
        {
            return None;
        }
        match (row.canonical_count, row.other_event_count, row.queued.as_slice()) {
            (0, 0, []) => Some(SurrealSwarmOutboxPersistOutcome::AlreadyCommitted),
            (1, 0, [queued])
                if queued.id == RecordId::new("swarm_terminal_event_outbox", original.record_id.clone())
                    && queued.event_id == original.event_id
                    && queued.event_ledger_event_id == RecordId::new("kernel_event_ledger", original.ledger_record_id.clone())
                    && queued.event_json == row.original_event_json
                    && queued.event_json_sha256 == row.original_event_json_sha256 =>
                Some(SurrealSwarmOutboxPersistOutcome::AlreadyQueued),
            _ => None,
        }
    }

    pub(crate) async fn has_scoped_rows(&self) -> Result<bool, SurrealSwarmOutboxError> {
        self.prepare().await?;
        let bindings = self.scope_bindings();
        let rows = self.storage.with_data_operation(|database| Box::pin(async move {
            database.query_values::<bool, _>(
                "SELECT VALUE true FROM swarm_terminal_event_outbox WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 1;",
                bindings,
            ).await
        })).await?;
        Ok(!rows.is_empty())
    }

    pub async fn next_pending(
        &self,
    ) -> Result<Option<SurrealSwarmOutboxEvent>, SurrealSwarmOutboxError> {
        self.prepare().await?;
        let bindings = self.scope_bindings();
        let rows = self.storage.with_data_operation(|database| Box::pin(async move {
            database.query_values::<PendingRow, _>(
                r#"SELECT id, event_id, event_json, event_json_sha256,
                    IF storage_authority = 'embedded_surrealdb'
                        AND event_ledger_event_id.owner_account_id = $owner_account_id
                        AND event_ledger_event_id.actor_principal_id = $actor_principal_id
                        AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id
                        AND event_ledger_event_id.access_space_id = $access_space_id
                        AND event_ledger_event_id.workspace_id = $workspace_id
                        AND event_ledger_event_id.source_component = 'swarm_terminal_outbox'
                        AND event_ledger_event_id.aggregate_type = 'swarm_terminal_event'
                    { {
                        receipt_record: event_ledger_event_id,
                        receipt_event_id: event_ledger_event_id.event_id,
                        receipt_event_version: event_ledger_event_id.event_version,
                        receipt_idempotency_key: event_ledger_event_id.idempotency_key,
                        receipt_aggregate_id: event_ledger_event_id.aggregate_id,
                        receipt_event_type: event_ledger_event_id.event_type,
                        receipt_payload_hash: event_ledger_event_id.payload_hash,
                        receipt_payload: event_ledger_event_id.payload
                    } } ELSE { NONE } AS receipt
                FROM (
                    SELECT id, event_id, event_json, event_json_sha256, created_at,
                        storage_authority, event_ledger_event_id
                    FROM swarm_terminal_event_outbox
                    WHERE owner_account_id = $owner_account_id
                        AND actor_principal_id = $actor_principal_id
                        AND authenticated_session_id = $authenticated_session_id
                        AND access_space_id = $access_space_id
                        AND workspace_id = $workspace_id
                    ORDER BY created_at ASC, event_id ASC LIMIT 1
                );"#,
                bindings,
            ).await
        })).await?;
        let Some(row) = rows.into_iter().next() else { return Ok(None); };
        let receipt = row.receipt.as_ref().ok_or(SurrealSwarmOutboxError::CorruptAuthority)?;
        let identity = self.identity_hash(&row.event_id);
        let ledger_id = format!("evt-swarm-terminal-{identity}");
        let idempotency_key = format!("swarm-terminal:{identity}");
        let payload_hash = sha256_hex(row.event_json.as_bytes());
        let expected_receipt_payload = serde_json::json!({
            "event_id": &row.event_id, "event_json": &row.event_json,
        });
        if row.id != RecordId::new("swarm_terminal_event_outbox", format!("swarm-outbox-{identity}"))
            || receipt.receipt_record.as_ref() != Some(&RecordId::new("kernel_event_ledger", ledger_id.clone()))
            || receipt.receipt_event_id.as_deref() != Some(ledger_id.as_str())
            || receipt.receipt_event_version.as_deref() != Some("kernel_event_v1")
            || receipt.receipt_idempotency_key.as_deref() != Some(idempotency_key.as_str())
            || receipt.receipt_aggregate_id.as_deref() != Some(row.event_id.as_str())
            || receipt.receipt_event_type.as_deref() != Some("SWARM_TERMINAL_EVENT_ENQUEUED")
            || row.event_json_sha256.as_deref() != Some(payload_hash.as_str())
            || receipt.receipt_payload_hash.as_deref() != Some(payload_hash.as_str())
            || receipt.receipt_payload.as_ref() != Some(&expected_receipt_payload)
        {
            return Err(SurrealSwarmOutboxError::CorruptAuthority);
        }
        Ok(Some(SurrealSwarmOutboxEvent {
            event_id: row.event_id,
            event_json: row.event_json,
        }))
    }

    fn exact_event_bindings(
        &self,
        event: &SurrealSwarmOutboxEvent,
        last_error: String,
    ) -> Result<(RecordId, ExactEventBindings), SurrealSwarmOutboxError> {
        if event.event_id.trim().is_empty() || event.event_json.trim().is_empty() {
            return Err(SurrealSwarmOutboxError::InvalidEvent);
        }
        let identity = self.identity_hash(&event.event_id);
        let scope = self.scope_bindings();
        let expected_id = RecordId::new(
            "swarm_terminal_event_outbox", format!("swarm-outbox-{identity}"),
        );
        let bindings = ExactEventBindings {
            record_id: format!("swarm-outbox-{identity}"),
            ledger_record_id: format!("evt-swarm-terminal-{identity}"),
            event_id: event.event_id.clone(),
            event_json: event.event_json.clone(),
            event_json_sha256: sha256_hex(event.event_json.as_bytes()),
            idempotency_key: format!("swarm-terminal:{identity}"),
            last_error,
            owner_account_id: scope.owner_account_id,
            actor_principal_id: scope.actor_principal_id,
            authenticated_session_id: scope.authenticated_session_id,
            access_space_id: scope.access_space_id,
            workspace_id: scope.workspace_id,
        };
        Ok((expected_id, bindings))
    }

    pub(crate) async fn finalize_exact(
        &self,
        event: &SurrealSwarmOutboxEvent,
    ) -> Result<(), SurrealSwarmOutboxError> {
        self.prepare().await?;
        let (expected_id, bindings) = self.exact_event_bindings(event, String::new())?;
        let unfiltered_owner = super::current_record_user_scope().is_none();
        let readback = bindings.clone();
        let removed = self.storage.with_data_operation(|database| Box::pin(async move {
            database.query_values::<surrealdb::types::Value, _>(
                exact_outbox_cas!("DELETE type::record('swarm_terminal_event_outbox', $record_id)"),
                bindings,
            ).await
        })).await;
        let original_error = match removed {
            Ok(mut rows) if rows.len() == 1 => {
                let removed = RecordId::from_value(rows.remove(0))
                    .map_err(|_| SurrealSwarmOutboxError::CorruptAuthority)?;
                return if removed == expected_id { Ok(()) }
                    else { Err(SurrealSwarmOutboxError::CorruptAuthority) };
            }
            Ok(rows) if !rows.is_empty() => return Err(SurrealSwarmOutboxError::CorruptAuthority),
            Ok(_) => SurrealSwarmOutboxError::CorruptAuthority,
            Err(error) => {
                let ambiguous = matches!(&error, SurrealStorageError::StatementTimeout { .. });
                let original = SurrealSwarmOutboxError::Storage(error);
                if !ambiguous { return Err(original); }
                original
            }
        };
        if !unfiltered_owner || super::current_record_user_scope().is_some() {
            return Err(original_error);
        }
        let verdict = self.storage.with_data_operation(|database| Box::pin(async move {
            database.query_values::<bool, _>(CONFIRM_EXACT_FINALIZATION, readback).await
        })).await;
        match verdict {
            Ok(rows) if rows.as_slice() == [true] => Ok(()),
            _ => Err(original_error),
        }
    }

    pub(crate) async fn record_failure_exact(
        &self,
        event: &SurrealSwarmOutboxEvent,
        error: &str,
    ) -> Result<(), SurrealSwarmOutboxError> {
        self.prepare().await?;
        let (expected_id, bindings) = self.exact_event_bindings(
            event, error.chars().take(4096).collect(),
        )?;
        let updated = self.storage.with_data_operation(|database| Box::pin(async move {
            database.query_values::<RecordId, _>(
                exact_outbox_cas!("UPDATE type::record('swarm_terminal_event_outbox', $record_id) SET attempts += 1, last_error = $last_error, last_attempt_at = time::now()"),
                bindings,
            ).await
        })).await?;
        if updated.len() != 1 || updated[0] != expected_id {
            return Err(SurrealSwarmOutboxError::CorruptAuthority);
        }
        Ok(())
    }

    pub async fn mark_delivered(&self, event_id: &str) -> Result<(), SurrealSwarmOutboxError> {
        self.prepare().await?;
        let bindings = self.event_bindings(event_id, String::new())?;
        self.storage.with_data_operation(|database| Box::pin(async move {
            database.query_bound(
                "DELETE type::record('swarm_terminal_event_outbox', $record_id) WHERE event_id = $event_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND storage_authority = 'embedded_surrealdb' AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id AND event_ledger_event_id.source_component = 'swarm_terminal_outbox' AND event_ledger_event_id.aggregate_type = 'swarm_terminal_event';",
                bindings,
            ).await?;
            Ok(())
        })).await?;
        Ok(())
    }

    pub async fn record_failure(
        &self,
        event_id: &str,
        error: &str,
    ) -> Result<(), SurrealSwarmOutboxError> {
        self.prepare().await?;
        let last_error: String = error.chars().take(4096).collect();
        let bindings = self.event_bindings(event_id, last_error)?;
        self.storage.with_data_operation(|database| Box::pin(async move {
            database.query_bound(
                "UPDATE type::record('swarm_terminal_event_outbox', $record_id) SET attempts += 1, last_error = $last_error, last_attempt_at = time::now() WHERE event_id = $event_id AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND storage_authority = 'embedded_surrealdb' AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id AND event_ledger_event_id.source_component = 'swarm_terminal_outbox' AND event_ledger_event_id.aggregate_type = 'swarm_terminal_event';",
                bindings,
            ).await?;
            Ok(())
        })).await?;
        Ok(())
    }

    fn scope_bindings(&self) -> ScopeBindings {
        ScopeBindings {
            owner_account_id: self.scope.owner_account_id.as_uuid().to_string(),
            actor_principal_id: self.scope.actor_principal_id.as_uuid().to_string(),
            authenticated_session_id: self.scope.authenticated_session_id.as_uuid().to_string(),
            access_space_id: self.scope.access_space_id.as_uuid().to_string(),
            workspace_id: self.scope.workspace_id.as_str().to_owned(),
        }
    }

    fn event_bindings(
        &self,
        event_id: &str,
        last_error: String,
    ) -> Result<EventBindings, SurrealSwarmOutboxError> {
        if event_id.trim().is_empty() {
            return Err(SurrealSwarmOutboxError::InvalidEvent);
        }
        let scope = self.scope_bindings();
        Ok(EventBindings {
            record_id: format!("swarm-outbox-{}", self.identity_hash(event_id)),
            event_id: event_id.to_owned(),
            last_error,
            owner_account_id: scope.owner_account_id,
            actor_principal_id: scope.actor_principal_id,
            authenticated_session_id: scope.authenticated_session_id,
            access_space_id: scope.access_space_id,
            workspace_id: scope.workspace_id,
        })
    }

    fn identity_hash(&self, event_id: &str) -> String {
        let scope = self.scope_bindings();
        sha256_hex(
            format!(
                "{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}",
                scope.owner_account_id,
                scope.actor_principal_id,
                scope.authenticated_session_id,
                scope.access_space_id,
                scope.workspace_id,
                event_id
            )
            .as_bytes(),
        )
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::surreal::{bootstrap_schema, SurrealStorageConfig};
    use crate::swarm_orchestration::resource_scope::{
        AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, OwnerAccountId,
        WorkspaceScopeRef,
    };
    use surrealdb::types::RecordId;

    #[derive(Debug, SurrealValue)]
    struct RecordBinding {
        record: RecordId,
    }

    #[derive(Debug, SurrealValue)]
    struct LinkBinding {
        outbox: RecordId,
        ledger: RecordId,
    }

    #[derive(Debug, SurrealValue)]
    struct RowPairBinding {
        first: RecordId,
        second: RecordId,
    }

    async fn open_test_store() -> (tempfile::TempDir, SurrealStorage) {
        let directory = tempfile::tempdir().expect("create swarm outbox test directory");
        let storage = SurrealStorage::open(
            SurrealStorageConfig::with_path(directory.path().join("store"))
                .expect("configure swarm outbox test store"),
        )
        .await
        .expect("open swarm outbox test store");
        bootstrap_schema(&storage)
            .await
            .expect("bootstrap canonical schema");
        bootstrap_swarm_outbox_schema(&storage)
            .await
            .expect("bootstrap swarm outbox schema");
        (directory, storage)
    }

    fn exact_scope(workspace: &str) -> ExactResourceScopeAttribution {
        ExactResourceScopeAttribution {
            owner_account_id: OwnerAccountId::mint(),
            actor_principal_id: ActorPrincipalId::mint(),
            authenticated_session_id: AuthenticatedSessionRef::mint(),
            access_space_id: AccessSpaceRef::mint(),
            workspace_id: WorkspaceScopeRef::new(workspace).expect("valid workspace scope"),
        }
    }

    fn record_ids(store: &SurrealSwarmOutboxStore, event_id: &str) -> (RecordId, RecordId) {
        let identity = store.identity_hash(event_id);
        (
            RecordId::new(
                "swarm_terminal_event_outbox",
                format!("swarm-outbox-{identity}"),
            ),
            RecordId::new(
                "kernel_event_ledger",
                format!("evt-swarm-terminal-{identity}"),
            ),
        )
    }

    #[tokio::test]
    async fn canonical_receipt_delete_is_rejected_while_outbox_row_exists() {
        let (_directory, storage) = open_test_store().await;
        let store = SurrealSwarmOutboxStore::new_exact(
            storage.clone(),
            exact_scope("workspace-outbox-delete-reject"),
        );
        let event_id = "terminal-event-delete-reject";
        store
            .persist(event_id, r#"{"kind":"terminal"}"#.to_owned(), 8)
            .await
            .expect("persist canonical outbox and receipt");
        let (_outbox_record, ledger_record) = record_ids(&store, event_id);

        let delete_result = storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    database
                        .query_bound(
                            "DELETE $record;",
                            RecordBinding {
                                record: ledger_record,
                            },
                        )
                        .await?;
                    Ok(())
                })
            })
            .await;
        assert!(
            delete_result.is_err(),
            "canonical receipt deletion must be rejected while the outbox reference exists"
        );
        assert_eq!(
            store.next_pending().await.expect("read pending outbox"),
            Some(SurrealSwarmOutboxEvent {
                event_id: event_id.to_owned(),
                event_json: r#"{"kind":"terminal"}"#.to_owned(),
            })
        );
        storage.shutdown().await.expect("shutdown test storage");
    }

    #[tokio::test]
    async fn missing_or_foreign_event_ledger_receipt_is_never_returned_or_deleted() {
        let (_directory, storage) = open_test_store().await;
        let local = SurrealSwarmOutboxStore::new_exact(
            storage.clone(),
            exact_scope("workspace-outbox-local"),
        );
        let foreign = SurrealSwarmOutboxStore::new_exact(
            storage.clone(),
            exact_scope("workspace-outbox-foreign"),
        );
        let missing_event_id = "terminal-event-missing-receipt";
        let foreign_event_id = "terminal-event-foreign-receipt";
        let foreign_receipt_source_id = "terminal-event-foreign-source";
        for (event_id, payload) in [
            (missing_event_id, r#"{"kind":"missing"}"#),
            (foreign_event_id, r#"{"kind":"foreign"}"#),
        ] {
            local
                .persist(event_id, payload.to_owned(), 8)
                .await
                .expect("persist local outbox row");
        }
        foreign
            .persist(
                foreign_receipt_source_id,
                r#"{"kind":"foreign-source"}"#.to_owned(),
                8,
            )
            .await
            .expect("persist foreign receipt source");

        let (missing_outbox, _) = record_ids(&local, missing_event_id);
        let (foreign_outbox, _) = record_ids(&local, foreign_event_id);
        let (_, foreign_ledger) = record_ids(&foreign, foreign_receipt_source_id);
        storage
            .with_admin_operation(|database| {
                let missing_outbox = missing_outbox.clone();
                let foreign_outbox = foreign_outbox.clone();
                Box::pin(async move {
                    database
                        .query(
                            "DEFINE FIELD OVERWRITE event_ledger_event_id ON TABLE swarm_terminal_event_outbox TYPE record<kernel_event_ledger>;",
                        )
                        .await?;
                    database
                        .query_bound(
                            "UPDATE $outbox SET event_ledger_event_id = $ledger;",
                            LinkBinding {
                                outbox: missing_outbox,
                                ledger: RecordId::new(
                                    "kernel_event_ledger",
                                    "deliberately-missing-receipt",
                                ),
                            },
                        )
                        .await?;
                    database
                        .query_bound(
                            "UPDATE $outbox SET event_ledger_event_id = $ledger;",
                            LinkBinding {
                                outbox: foreign_outbox,
                                ledger: foreign_ledger,
                            },
                        )
                        .await?;
                    Ok(())
                })
            })
            .await
            .expect("seed missing and foreign receipt links through test-only admin");

        assert!(
            matches!(local.next_pending().await, Err(SurrealSwarmOutboxError::CorruptAuthority)),
            "neither unattributed durable row may be returned"
        );
        local
            .mark_delivered(missing_event_id)
            .await
            .expect("missing receipt is a non-mutating delivery miss");
        local
            .mark_delivered(foreign_event_id)
            .await
            .expect("foreign receipt is a non-mutating delivery miss");

        let remaining = storage
            .with_admin_operation(|database| {
                Box::pin(async move {
                    let mut response = database
                        .query_bound(
                            "SELECT VALUE id FROM $first; SELECT VALUE id FROM $second;",
                            RowPairBinding {
                                first: missing_outbox,
                                second: foreign_outbox,
                            },
                        )
                        .await?;
                    let first: Vec<RecordId> = response.take(0)?;
                    let second: Vec<RecordId> = response.take(1)?;
                    Ok((first.len(), second.len()))
                })
            })
            .await
            .expect("inspect denied rows through test-only admin");
        assert_eq!(remaining.0, 1, "missing-receipt row was deleted");
        assert_eq!(remaining.1, 1, "foreign-receipt row was deleted");
        storage.shutdown().await.expect("shutdown test storage");
    }
}
