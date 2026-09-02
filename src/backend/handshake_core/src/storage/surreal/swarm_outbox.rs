use std::sync::Arc;

use sha2::{Digest, Sha256};
use surrealdb::types::SurrealValue;
use thiserror::Error;
use tokio::sync::OnceCell;

use super::{SurrealStorage, SurrealStorageError};
use crate::swarm_orchestration::resource_scope::{
    ExactResourceScopeAttribution, ResourceScope,
};

const SCHEMA: &str = include_str!("swarm_outbox_schema.surql");
const SCHEMA_STATE: &str = "\
DEFINE TABLE IF NOT EXISTS swarm_outbox_schema_state SCHEMAFULL;\
DEFINE FIELD IF NOT EXISTS schema_version ON swarm_outbox_schema_state TYPE string;\
DEFINE FIELD IF NOT EXISTS schema_revision ON swarm_outbox_schema_state TYPE int;\
DEFINE FIELD IF NOT EXISTS apply_state ON swarm_outbox_schema_state TYPE string;";
const SCHEMA_VERSION: &str = "mt003-swarm-terminal-outbox-v1";
const SCHEMA_REVISION: i64 = 1;
const PERSIST_EVENT: &str = r#"
BEGIN TRANSACTION;
LET $existing = (
    SELECT event_json_sha256 FROM type::record('swarm_terminal_event_outbox', $record_id)
    WHERE event_id = $event_id
      AND owner_account_id = $owner_account_id
      AND actor_principal_id = $actor_principal_id
      AND authenticated_session_id = $authenticated_session_id
      AND access_space_id = $access_space_id
      AND workspace_id = $workspace_id
    LIMIT 2
);
LET $prior = (
    SELECT payload_hash FROM type::record('kernel_event_ledger', $ledger_record_id)
    WHERE event_id = $ledger_record_id
      AND idempotency_key = $idempotency_key
      AND source_component = 'swarm_terminal_outbox'
      AND owner_account_id = $owner_account_id
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
} ELSE IF array::len($existing) = 1 {
    RETURN { outcome: IF $existing[0].event_json_sha256 = $event_json_sha256 { 'already_queued' } ELSE { 'conflict' } };
} ELSE IF array::len($prior) = 1 {
    RETURN { outcome: IF $prior[0].payload_hash = $event_json_sha256 { 'already_committed' } ELSE { 'conflict' } };
} ELSE IF array::len($queued) >= $capacity {
    RETURN { outcome: 'capacity' };
} ELSE {
    LET $ledger = CREATE type::record('kernel_event_ledger', $ledger_record_id) CONTENT {
        event_id: $ledger_record_id,
        event_version: 'kernel_event_v1',
        kernel_task_run_id: $workspace_id,
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
    };
    RETURN { outcome: 'stored' };
};
COMMIT TRANSACTION;
"#;

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

#[derive(Debug, SurrealValue)]
struct PersistBindings {
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
}

#[derive(Debug, SurrealValue)]
struct PendingRow {
    event_id: String,
    event_json: String,
}

pub async fn bootstrap_swarm_outbox_schema(
    storage: &SurrealStorage,
) -> Result<(), SurrealStorageError> {
    super::bootstrap_schema(storage).await?;
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
    pub fn new_exact(
        storage: SurrealStorage,
        scope: ExactResourceScopeAttribution,
    ) -> Self {
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

    async fn ensure_initialized(&self) -> Result<(), SurrealSwarmOutboxError> {
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
    ) -> Result<(), SurrealSwarmOutboxError> {
        self.ensure_initialized().await?;
        if event_id.trim().is_empty()
            || event_json.trim().is_empty()
            || serde_json::from_str::<serde_json::Value>(&event_json).is_err()
            || capacity == 0
        {
            return Err(SurrealSwarmOutboxError::InvalidEvent);
        }
        let event_json_sha256 = sha256_hex(event_json.as_bytes());
        let identity = self.identity_hash(event_id);
        let bindings = PersistBindings {
            record_id: format!("swarm-outbox-{identity}"),
            ledger_record_id: format!("evt-swarm-terminal-{identity}"),
            event_id: event_id.to_owned(),
            event_json,
            event_json_sha256,
            idempotency_key: format!("swarm-terminal:{identity}"),
            capacity: i64::try_from(capacity).unwrap_or(i64::MAX),
            owner_account_id: self.scope.owner_account_id.as_uuid().to_string(),
            actor_principal_id: self.scope.actor_principal_id.as_uuid().to_string(),
            authenticated_session_id: self
                .scope
                .authenticated_session_id
                .as_uuid()
                .to_string(),
            access_space_id: self.scope.access_space_id.as_uuid().to_string(),
            workspace_id: self.scope.workspace_id.as_str().to_owned(),
        };
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values_at::<PersistResult, _>(PERSIST_EVENT, bindings, 4)
                        .await
                })
            })
            .await?;
        let outcome = rows
            .into_iter()
            .next()
            .ok_or(SurrealSwarmOutboxError::CorruptAuthority)?
            .outcome;
        match outcome.as_str() {
            "stored" | "already_queued" | "already_committed" => Ok(()),
            "conflict" => Err(SurrealSwarmOutboxError::IdempotencyConflict),
            "capacity" => Err(SurrealSwarmOutboxError::CapacityExceeded { capacity }),
            _ => Err(SurrealSwarmOutboxError::CorruptAuthority),
        }
    }

    pub async fn next_pending(
        &self,
    ) -> Result<Option<SurrealSwarmOutboxEvent>, SurrealSwarmOutboxError> {
        self.ensure_initialized().await?;
        let bindings = self.scope_bindings();
        let rows = self.storage.with_data_operation(|database| Box::pin(async move {
            database.query_values::<PendingRow, _>(
                "SELECT event_id, event_json FROM swarm_terminal_event_outbox WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id AND storage_authority = 'embedded_surrealdb' AND event_ledger_event_id.owner_account_id = $owner_account_id AND event_ledger_event_id.actor_principal_id = $actor_principal_id AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id AND event_ledger_event_id.access_space_id = $access_space_id AND event_ledger_event_id.workspace_id = $workspace_id AND event_ledger_event_id.source_component = 'swarm_terminal_outbox' AND event_ledger_event_id.aggregate_type = 'swarm_terminal_event' ORDER BY created_at ASC, event_id ASC LIMIT 2;",
                bindings,
            ).await
        })).await?;
        if rows.len() > 1 {
            return Err(SurrealSwarmOutboxError::CorruptAuthority);
        }
        Ok(rows.into_iter().next().map(|row| SurrealSwarmOutboxEvent {
            event_id: row.event_id,
            event_json: row.event_json,
        }))
    }

    pub async fn mark_delivered(
        &self,
        event_id: &str,
    ) -> Result<(), SurrealSwarmOutboxError> {
        self.ensure_initialized().await?;
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
        self.ensure_initialized().await?;
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
            authenticated_session_id: self
                .scope
                .authenticated_session_id
                .as_uuid()
                .to_string(),
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

    fn record_ids(
        store: &SurrealSwarmOutboxStore,
        event_id: &str,
    ) -> (RecordId, RecordId) {
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

        assert_eq!(
            local.next_pending().await.expect("read local outbox"),
            None,
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
