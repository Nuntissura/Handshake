use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use surrealdb::types::{RecordId, SurrealValue};
use uuid::Uuid;

use crate::kernel::{KernelActor, KernelEvent, KernelEventType, NewKernelEvent};
use crate::storage::knowledge::{KnowledgeEntity, KnowledgeEntityKind, KnowledgeEntityLifecycle};
use crate::storage::{StorageError, StorageResult};
use crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution;

use super::{SurrealStorage, SurrealStorageError};

const SCHEMA: &str = include_str!("user_manual_knowledge_schema.surql");

#[derive(Clone)]
pub struct SurrealUserManualKnowledgeStore {
    storage: SurrealStorage,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UserManualKnowledgeEntityMutation {
    pub entity: KnowledgeEntity,
    pub event_ledger_event_id: String,
    pub changed: bool,
}

#[derive(Debug, SurrealValue)]
struct ExactScopeBindings {
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: RecordId,
}

#[derive(Debug, SurrealValue)]
struct KnowledgeEntityContent {
    entity_id: String,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: RecordId,
    entity_kind: String,
    entity_key: String,
    display_name: String,
    detection_provenance: Value,
    lifecycle_state: String,
    recorded_event_id: RecordId,
    mutation_fingerprint: String,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, SurrealValue)]
struct KnowledgeReceiptWrite {
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
struct UpsertBindings {
    entity_record: RecordId,
    entity: KnowledgeEntityContent,
    event: KnowledgeReceiptWrite,
    predecessor_event_id: Option<String>,
}

#[derive(Debug, SurrealValue)]
struct IdentityBindings {
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: RecordId,
    entity_key: String,
}

#[cfg(feature = "test-utils")]
#[derive(Debug, SurrealValue)]
struct ReceiptOnlyBindings {
    entity_record: RecordId,
    event: KnowledgeReceiptWrite,
}

#[cfg(feature = "test-utils")]
#[derive(Debug, SurrealValue)]
struct MismatchReceiptBindings {
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: RecordId,
    entity_key: String,
    mismatched_display_name: String,
}

#[cfg(feature = "test-utils")]
#[derive(Debug, SurrealValue)]
struct ReceiptOnlyResult {
    event_id: String,
}

#[derive(Debug, SurrealValue)]
struct StoredKnowledgeEntity {
    entity_id: String,
    workspace_id: String,
    entity_kind: String,
    entity_key: String,
    display_name: String,
    detection_provenance: Value,
    lifecycle_state: String,
    primary_source_id: Option<String>,
    first_detected_in_run: Option<String>,
    last_detected_in_run: Option<String>,
    recorded_event_id: String,
    mutation_fingerprint: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    changed: bool,
}

const UPSERT_USER_MANUAL_ENTITY_QUERY: &str = r#"
BEGIN TRANSACTION;
LET $existing = (SELECT
    entity_id,
    record::id(workspace_id) AS workspace_id,
    entity_kind,
    entity_key,
    display_name,
    detection_provenance,
    lifecycle_state,
    record::id(primary_source_id) AS primary_source_id,
    record::id(first_detected_in_run) AS first_detected_in_run,
    record::id(last_detected_in_run) AS last_detected_in_run,
    record::id(recorded_event_id) AS recorded_event_id,
    mutation_fingerprint,
    created_at,
    updated_at
FROM knowledge_entities
WHERE owner_account_id = $entity.owner_account_id
    AND actor_principal_id = $entity.actor_principal_id
    AND authenticated_session_id = $entity.authenticated_session_id
    AND access_space_id = $entity.access_space_id
    AND workspace_id = $entity.workspace_id
    AND entity_kind = 'user_manual_page'
    AND entity_key = $entity.entity_key
LIMIT 2);
LET $linked_event = IF array::len($existing) = 1 {
    (SELECT event_id
    FROM kernel_event_ledger
    WHERE event_id = $existing[0].recorded_event_id
        AND aggregate_type = 'knowledge_entity'
        AND aggregate_id = $existing[0].entity_id
        AND event_type = 'KNOWLEDGE_USER_MANUAL_ENTRY_RECORDED'
        AND actor_kind = 'system'
        AND actor_id = 'user_manual::bundle_bridge'
        AND source_component = 'user_manual::bundle_bridge'
        AND session_run_id = $entity.authenticated_session_id
        AND payload.action = 'user_manual_page_knowledge_entity_upserted'
        AND payload.entity_id = $existing[0].entity_id
        AND payload.entity_key = $existing[0].entity_key
        AND payload.display_name = $existing[0].display_name
        AND payload.mutation_fingerprint = $existing[0].mutation_fingerprint
        AND owner_account_id = $entity.owner_account_id
        AND actor_principal_id = $entity.actor_principal_id
        AND authenticated_session_id = $entity.authenticated_session_id
        AND access_space_id = $entity.access_space_id
        AND workspace_id = record::id($entity.workspace_id)
    LIMIT 2)
} ELSE { [] };
LET $event_existing = (SELECT event_id
FROM kernel_event_ledger
WHERE idempotency_key = $event.idempotency_key
    AND owner_account_id = $event.owner_account_id
    AND actor_principal_id = $event.actor_principal_id
    AND authenticated_session_id = $event.authenticated_session_id
    AND access_space_id = $event.access_space_id
    AND workspace_id = $event.workspace_id
LIMIT 2);
IF array::len($existing) > 1 OR array::len($linked_event) > 1
    OR array::len($event_existing) > 1 {
    THROW 'UserManual knowledge entity or receipt identity is ambiguous';
} ELSE IF (array::len($existing) = 0 AND $predecessor_event_id != NONE)
    OR (array::len($existing) = 1
        AND ($predecessor_event_id = NONE
            OR $existing[0].recorded_event_id != $predecessor_event_id
            OR array::len($linked_event) != 1)) {
    THROW 'UserManual knowledge entity predecessor changed before mutation';
} ELSE IF array::len($existing) = 1
    AND $existing[0].mutation_fingerprint = $entity.mutation_fingerprint {
    IF array::len($linked_event) != 1 {
        THROW 'UserManual knowledge entity has inconsistent receipt evidence';
    };
    RETURN SELECT
        entity_id,
        record::id(workspace_id) AS workspace_id,
        entity_kind,
        entity_key,
        display_name,
        detection_provenance,
        lifecycle_state,
        record::id(primary_source_id) AS primary_source_id,
        record::id(first_detected_in_run) AS first_detected_in_run,
        record::id(last_detected_in_run) AS last_detected_in_run,
        record::id(recorded_event_id) AS recorded_event_id,
        mutation_fingerprint,
        created_at,
        updated_at,
        false AS changed
    FROM $entity_record
    WHERE owner_account_id = $entity.owner_account_id
        AND actor_principal_id = $entity.actor_principal_id
        AND authenticated_session_id = $entity.authenticated_session_id
        AND access_space_id = $entity.access_space_id
        AND workspace_id = $entity.workspace_id;
} ELSE {
    IF array::len($event_existing) != 0 {
        THROW 'UserManual knowledge receipt exists without its entity mutation';
    };
    CREATE type::record('kernel_event_ledger', $event.event_id) CONTENT {
        event_id: $event.event_id,
        event_version: $event.event_version,
        kernel_task_run_id: $event.kernel_task_run_id,
        session_run_id: $event.session_run_id,
        aggregate_type: $event.aggregate_type,
        aggregate_id: $event.aggregate_id,
        idempotency_key: $event.idempotency_key,
        event_type: $event.event_type,
        actor_kind: $event.actor_kind,
        actor_id: $event.actor_id,
        causation_id: $event.causation_id,
        correlation_id: $event.correlation_id,
        payload_hash: $event.payload_hash,
        source_component: $event.source_component,
        payload: $event.payload,
        owner_account_id: $event.owner_account_id,
        actor_principal_id: $event.actor_principal_id,
        authenticated_session_id: $event.authenticated_session_id,
        access_space_id: $event.access_space_id,
        workspace_id: $event.workspace_id
    };
    UPSERT $entity_record MERGE $entity;
    RETURN SELECT
        entity_id,
        record::id(workspace_id) AS workspace_id,
        entity_kind,
        entity_key,
        display_name,
        detection_provenance,
        lifecycle_state,
        record::id(primary_source_id) AS primary_source_id,
        record::id(first_detected_in_run) AS first_detected_in_run,
        record::id(last_detected_in_run) AS last_detected_in_run,
        record::id(recorded_event_id) AS recorded_event_id,
        mutation_fingerprint,
        created_at,
        updated_at,
        true AS changed
    FROM $entity_record
    WHERE owner_account_id = $entity.owner_account_id
        AND actor_principal_id = $entity.actor_principal_id
        AND authenticated_session_id = $entity.authenticated_session_id
        AND access_space_id = $entity.access_space_id
        AND workspace_id = $entity.workspace_id;
};
COMMIT TRANSACTION;
"#;

const GET_USER_MANUAL_ENTITY_QUERY: &str = r#"
BEGIN TRANSACTION;
LET $existing = (SELECT
    entity_id,
    record::id(workspace_id) AS workspace_id,
    entity_kind,
    entity_key,
    display_name,
    detection_provenance,
    lifecycle_state,
    record::id(primary_source_id) AS primary_source_id,
    record::id(first_detected_in_run) AS first_detected_in_run,
    record::id(last_detected_in_run) AS last_detected_in_run,
    record::id(recorded_event_id) AS recorded_event_id,
    mutation_fingerprint,
    created_at,
    updated_at,
    false AS changed
FROM knowledge_entities
WHERE owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
    AND entity_kind = 'user_manual_page'
    AND entity_key = $entity_key
LIMIT 2);
LET $linked_event = IF array::len($existing) = 1 {
    (SELECT event_id
    FROM kernel_event_ledger
    WHERE event_id = $existing[0].recorded_event_id
        AND aggregate_type = 'knowledge_entity'
        AND aggregate_id = $existing[0].entity_id
        AND event_type = 'KNOWLEDGE_USER_MANUAL_ENTRY_RECORDED'
        AND actor_kind = 'system'
        AND actor_id = 'user_manual::bundle_bridge'
        AND source_component = 'user_manual::bundle_bridge'
        AND session_run_id = $authenticated_session_id
        AND payload.action = 'user_manual_page_knowledge_entity_upserted'
        AND payload.entity_id = $existing[0].entity_id
        AND payload.entity_key = $existing[0].entity_key
        AND payload.display_name = $existing[0].display_name
        AND payload.mutation_fingerprint = $existing[0].mutation_fingerprint
        AND owner_account_id = $owner_account_id
        AND actor_principal_id = $actor_principal_id
        AND authenticated_session_id = $authenticated_session_id
        AND access_space_id = $access_space_id
        AND workspace_id = record::id($workspace_id)
    LIMIT 2)
} ELSE { [] };
IF array::len($existing) > 1 OR array::len($linked_event) > 1 {
    THROW 'UserManual knowledge entity identity is ambiguous';
} ELSE IF array::len($existing) = 0 {
    RETURN [];
} ELSE IF array::len($linked_event) != 1 {
    THROW 'UserManual knowledge entity has inconsistent receipt evidence';
} ELSE {
    RETURN $existing;
};
COMMIT TRANSACTION;
"#;

#[cfg(feature = "test-utils")]
const INSERT_ORPHAN_RECEIPT_FIXTURE_QUERY: &str = r#"
BEGIN TRANSACTION;
LET $entity = (SELECT VALUE id FROM $entity_record
WHERE owner_account_id = $event.owner_account_id
    AND actor_principal_id = $event.actor_principal_id
    AND authenticated_session_id = $event.authenticated_session_id
    AND access_space_id = $event.access_space_id
    AND workspace_id = type::record('workspaces', $event.workspace_id)
LIMIT 2);
LET $existing = (SELECT event_id FROM kernel_event_ledger
WHERE idempotency_key = $event.idempotency_key
    AND owner_account_id = $event.owner_account_id
    AND actor_principal_id = $event.actor_principal_id
    AND authenticated_session_id = $event.authenticated_session_id
    AND access_space_id = $event.access_space_id
    AND workspace_id = $event.workspace_id
LIMIT 2);
IF array::len($entity) != 0 OR array::len($existing) > 1 {
    THROW 'orphan receipt fixture requires one absent entity and unambiguous receipt identity';
} ELSE IF array::len($existing) = 1 {
    RETURN $existing;
} ELSE {
    CREATE type::record('kernel_event_ledger', $event.event_id) CONTENT {
        event_id: $event.event_id,
        event_version: $event.event_version,
        kernel_task_run_id: $event.kernel_task_run_id,
        session_run_id: $event.session_run_id,
        aggregate_type: $event.aggregate_type,
        aggregate_id: $event.aggregate_id,
        idempotency_key: $event.idempotency_key,
        event_type: $event.event_type,
        actor_kind: $event.actor_kind,
        actor_id: $event.actor_id,
        causation_id: $event.causation_id,
        correlation_id: $event.correlation_id,
        payload_hash: $event.payload_hash,
        source_component: $event.source_component,
        payload: $event.payload,
        owner_account_id: $event.owner_account_id,
        actor_principal_id: $event.actor_principal_id,
        authenticated_session_id: $event.authenticated_session_id,
        access_space_id: $event.access_space_id,
        workspace_id: $event.workspace_id
    };
    RETURN [{ event_id: $event.event_id }];
};
COMMIT TRANSACTION;
"#;

#[cfg(feature = "test-utils")]
const MISMATCH_CANONICAL_RECEIPT_FIXTURE_QUERY: &str = r#"
BEGIN TRANSACTION;
LET $entity = (SELECT recorded_event_id
FROM knowledge_entities
WHERE owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
    AND entity_kind = 'user_manual_page'
    AND entity_key = $entity_key
LIMIT 2);
LET $receipt = IF array::len($entity) = 1 {
    (SELECT id, event_id
    FROM kernel_event_ledger
    WHERE id = $entity[0].recorded_event_id
        AND owner_account_id = $owner_account_id
        AND actor_principal_id = $actor_principal_id
        AND authenticated_session_id = $authenticated_session_id
        AND access_space_id = $access_space_id
        AND workspace_id = record::id($workspace_id)
    LIMIT 2)
} ELSE { [] };
IF array::len($entity) != 1 OR array::len($receipt) != 1 {
    THROW 'canonical receipt mismatch fixture requires one exact entity and receipt';
} ELSE {
    UPDATE $receipt[0].id SET payload.display_name = $mismatched_display_name
    WHERE owner_account_id = $owner_account_id
        AND actor_principal_id = $actor_principal_id
        AND authenticated_session_id = $authenticated_session_id
        AND access_space_id = $access_space_id
        AND workspace_id = record::id($workspace_id);
    RETURN [{ event_id: $receipt[0].event_id }];
};
COMMIT TRANSACTION;
"#;

impl SurrealUserManualKnowledgeStore {
    pub fn new(storage: SurrealStorage) -> Self {
        Self { storage }
    }

    pub async fn open(storage: SurrealStorage) -> StorageResult<Self> {
        bootstrap_user_manual_knowledge_schema(&storage).await?;
        Ok(Self::new(storage))
    }

    pub fn storage(&self) -> &SurrealStorage {
        &self.storage
    }

    pub async fn upsert_user_manual_page_entity(
        &self,
        scope: &ExactResourceScopeAttribution,
        entity_key: &str,
        display_name: &str,
        detection_provenance: Value,
    ) -> StorageResult<UserManualKnowledgeEntityMutation> {
        let predecessor_event_id = self
            .get_user_manual_page_entity_by_identity(scope, entity_key)
            .await?
            .map(|stored| stored.event_ledger_event_id);
        self.upsert_user_manual_page_entity_with_expected_predecessor(
            scope,
            entity_key,
            display_name,
            detection_provenance,
            predecessor_event_id.as_deref(),
        )
        .await
    }

    /// Commits a derived manual entity only when its canonical receipt still matches the
    /// caller-observed predecessor. This is the production compare-and-swap boundary used by
    /// concurrent bundle builders; the check occurs in the same transaction as the entity and
    /// canonical EventLedger receipt writes.
    pub async fn upsert_user_manual_page_entity_with_expected_predecessor(
        &self,
        scope: &ExactResourceScopeAttribution,
        entity_key: &str,
        display_name: &str,
        detection_provenance: Value,
        expected_predecessor_event_id: Option<&str>,
    ) -> StorageResult<UserManualKnowledgeEntityMutation> {
        validate_manual_entity_input(scope, entity_key, display_name, &detection_provenance)?;
        if expected_predecessor_event_id
            .is_some_and(|event_id| event_id.trim().is_empty() || event_id.trim() != event_id)
        {
            return Err(StorageError::Validation(
                "UserManual knowledge predecessor receipt must be non-empty without surrounding whitespace",
            ));
        }
        let fingerprint =
            mutation_fingerprint(scope, entity_key, display_name, &detection_provenance)?;
        let entity_id = deterministic_entity_id(scope, entity_key);
        let predecessor_event_id = expected_predecessor_event_id.map(str::to_owned);
        let event = receipt_write(
            knowledge_receipt_event(
                scope,
                &entity_id,
                entity_key,
                display_name,
                &detection_provenance,
                &fingerprint,
                predecessor_event_id.as_deref(),
            )?,
            scope,
        );
        let event_id = event.event_id.clone();
        let exact = exact_scope_bindings(scope);
        let content = KnowledgeEntityContent {
            entity_id: entity_id.clone(),
            owner_account_id: exact.owner_account_id,
            actor_principal_id: exact.actor_principal_id,
            authenticated_session_id: exact.authenticated_session_id,
            access_space_id: exact.access_space_id,
            workspace_id: exact.workspace_id,
            entity_kind: KnowledgeEntityKind::UserManualPage.as_str().to_owned(),
            entity_key: entity_key.to_owned(),
            display_name: display_name.to_owned(),
            detection_provenance,
            lifecycle_state: KnowledgeEntityLifecycle::Active.as_str().to_owned(),
            recorded_event_id: RecordId::new("kernel_event_ledger", event_id),
            mutation_fingerprint: fingerprint,
            updated_at: Utc::now(),
        };
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values_at::<StoredKnowledgeEntity, _>(
                            UPSERT_USER_MANUAL_ENTITY_QUERY,
                            UpsertBindings {
                                entity_record: RecordId::new("knowledge_entities", entity_id),
                                entity: content,
                                event,
                                predecessor_event_id,
                            },
                            4,
                        )
                        .await
                })
            })
            .await?;
        one_mutation(rows)
    }

    /// Resolves one unambiguous derived-entity citation against its canonical receipt.
    /// Candidate cardinality is validated before any durable read, and the exact-scope lookup
    /// revalidates the entity-to-receipt link before returning content.
    pub async fn resolve_user_manual_page_entity_citation(
        &self,
        scope: &ExactResourceScopeAttribution,
        entity_keys: &[&str],
        receipt_event_ids: &[&str],
    ) -> StorageResult<UserManualKnowledgeEntityMutation> {
        if entity_keys.is_empty() || receipt_event_ids.is_empty() {
            return Err(StorageError::Validation(
                "UserManual knowledge citation requires one entity and one receipt",
            ));
        }
        if entity_keys.len() != 1 {
            return Err(StorageError::Conflict(
                "UserManual knowledge entity citation is ambiguous",
            ));
        }
        if receipt_event_ids.len() != 1 {
            return Err(StorageError::Conflict(
                "UserManual knowledge receipt citation is ambiguous",
            ));
        }
        let expected_receipt = receipt_event_ids[0];
        if expected_receipt.trim().is_empty() || expected_receipt.trim() != expected_receipt {
            return Err(StorageError::Validation(
                "UserManual knowledge citation receipt must be non-empty without surrounding whitespace",
            ));
        }
        let resolved = self
            .get_user_manual_page_entity_by_identity(scope, entity_keys[0])
            .await?
            .ok_or(StorageError::NotFound(
                "UserManual knowledge citation did not resolve",
            ))?;
        if resolved.event_ledger_event_id != expected_receipt {
            return Err(StorageError::Conflict(
                "UserManual knowledge citation receipt does not match canonical evidence",
            ));
        }
        Ok(resolved)
    }

    pub async fn get_user_manual_page_entity_by_identity(
        &self,
        scope: &ExactResourceScopeAttribution,
        entity_key: &str,
    ) -> StorageResult<Option<UserManualKnowledgeEntityMutation>> {
        if entity_key.trim().is_empty() || entity_key.trim() != entity_key {
            return Err(StorageError::Validation(
                "UserManual knowledge entity key must be non-empty without surrounding whitespace",
            ));
        }
        let exact = exact_scope_bindings(scope);
        let entity_key = entity_key.to_owned();
        let mut rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values_at::<StoredKnowledgeEntity, _>(
                            GET_USER_MANUAL_ENTITY_QUERY,
                            IdentityBindings {
                                owner_account_id: exact.owner_account_id,
                                actor_principal_id: exact.actor_principal_id,
                                authenticated_session_id: exact.authenticated_session_id,
                                access_space_id: exact.access_space_id,
                                workspace_id: exact.workspace_id,
                                entity_key,
                            },
                            3,
                        )
                        .await
                })
            })
            .await?;
        if rows.len() > 1 {
            return Err(StorageError::Conflict(
                "UserManual knowledge entity identity is ambiguous",
            ));
        }
        rows.pop().map(stored_mutation).transpose()
    }

    #[cfg(feature = "test-utils")]
    pub async fn ensure_workspace_fixture(&self, workspace_id: &str) -> StorageResult<()> {
        if workspace_id.trim().is_empty() || workspace_id.trim() != workspace_id {
            return Err(StorageError::Validation(
                "knowledge fixture workspace id must be non-empty without surrounding whitespace",
            ));
        }
        #[derive(Debug, SurrealValue)]
        struct WorkspaceFixture {
            name: String,
            updated_at: DateTime<Utc>,
        }
        let workspace_id = workspace_id.to_owned();
        self.storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .upsert_one::<Value, _>(
                            "workspaces",
                            &workspace_id,
                            WorkspaceFixture {
                                name: "UserManual knowledge bridge fixture".to_owned(),
                                updated_at: Utc::now(),
                            },
                        )
                        .await
                })
            })
            .await?;
        Ok(())
    }

    #[cfg(feature = "test-utils")]
    pub async fn insert_orphan_receipt_fixture(
        &self,
        scope: &ExactResourceScopeAttribution,
        entity_key: &str,
        display_name: &str,
        detection_provenance: Value,
    ) -> StorageResult<String> {
        validate_manual_entity_input(scope, entity_key, display_name, &detection_provenance)?;
        let fingerprint =
            mutation_fingerprint(scope, entity_key, display_name, &detection_provenance)?;
        let entity_id = deterministic_entity_id(scope, entity_key);
        let event = receipt_write(
            knowledge_receipt_event(
                scope,
                &entity_id,
                entity_key,
                display_name,
                &detection_provenance,
                &fingerprint,
                None,
            )?,
            scope,
        );
        let expected_event_id = event.event_id.clone();
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values_at::<ReceiptOnlyResult, _>(
                            INSERT_ORPHAN_RECEIPT_FIXTURE_QUERY,
                            ReceiptOnlyBindings {
                                entity_record: RecordId::new("knowledge_entities", entity_id),
                                event,
                            },
                            3,
                        )
                        .await
                })
            })
            .await?;
        if rows.len() != 1 || rows[0].event_id != expected_event_id {
            return Err(StorageError::Validation(
                "orphan UserManual knowledge receipt fixture returned unstable evidence",
            ));
        }
        Ok(expected_event_id)
    }

    #[cfg(feature = "test-utils")]
    pub async fn mismatch_canonical_receipt_fixture(
        &self,
        scope: &ExactResourceScopeAttribution,
        entity_key: &str,
        mismatched_display_name: &str,
    ) -> StorageResult<String> {
        if entity_key.trim().is_empty() || entity_key.trim() != entity_key {
            return Err(StorageError::Validation(
                "UserManual knowledge entity key must be non-empty without surrounding whitespace",
            ));
        }
        if mismatched_display_name.trim().is_empty() {
            return Err(StorageError::Validation(
                "mismatched UserManual receipt display name is required",
            ));
        }
        let exact = exact_scope_bindings(scope);
        let bindings = MismatchReceiptBindings {
            owner_account_id: exact.owner_account_id,
            actor_principal_id: exact.actor_principal_id,
            authenticated_session_id: exact.authenticated_session_id,
            access_space_id: exact.access_space_id,
            workspace_id: exact.workspace_id,
            entity_key: entity_key.to_owned(),
            mismatched_display_name: mismatched_display_name.to_owned(),
        };
        let mut rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values_at::<ReceiptOnlyResult, _>(
                            MISMATCH_CANONICAL_RECEIPT_FIXTURE_QUERY,
                            bindings,
                            3,
                        )
                        .await
                })
            })
            .await?;
        if rows.len() != 1 {
            return Err(StorageError::Validation(
                "canonical UserManual receipt mismatch fixture returned unstable evidence",
            ));
        }
        Ok(rows.pop().expect("one row checked").event_id)
    }
}

pub async fn bootstrap_user_manual_knowledge_schema(
    storage: &SurrealStorage,
) -> Result<(), SurrealStorageError> {
    storage
        .with_admin_operation(|database| Box::pin(async move { database.query(SCHEMA).await }))
        .await?;
    Ok(())
}

fn exact_scope_bindings(scope: &ExactResourceScopeAttribution) -> ExactScopeBindings {
    ExactScopeBindings {
        owner_account_id: scope.owner_account_id.to_string(),
        actor_principal_id: scope.actor_principal_id.to_string(),
        authenticated_session_id: scope.authenticated_session_id.to_string(),
        access_space_id: scope.access_space_id.to_string(),
        workspace_id: RecordId::new("workspaces", scope.workspace_id.as_str().to_owned()),
    }
}

fn validate_manual_entity_input(
    scope: &ExactResourceScopeAttribution,
    entity_key: &str,
    display_name: &str,
    detection_provenance: &Value,
) -> StorageResult<()> {
    if entity_key.trim().is_empty() || entity_key.trim() != entity_key {
        return Err(StorageError::Validation(
            "UserManual knowledge entity key must be non-empty without surrounding whitespace",
        ));
    }
    if display_name.trim().is_empty() {
        return Err(StorageError::Validation(
            "UserManual knowledge entity display name is required",
        ));
    }
    if !detection_provenance.is_object() {
        return Err(StorageError::Validation(
            "UserManual knowledge detection provenance must be an object",
        ));
    }
    if scope.workspace_id.as_str().trim().is_empty() {
        return Err(StorageError::Validation(
            "UserManual knowledge entity requires an exact workspace",
        ));
    }
    Ok(())
}

fn deterministic_entity_id(scope: &ExactResourceScopeAttribution, entity_key: &str) -> String {
    format!(
        "KEN-{}",
        sha256_hex(&format!(
            "{}\0{}\0{}\0{}\0{}\0{}\0{}",
            scope.owner_account_id,
            scope.actor_principal_id,
            scope.authenticated_session_id,
            scope.access_space_id,
            scope.workspace_id.as_str(),
            KnowledgeEntityKind::UserManualPage.as_str(),
            entity_key
        ))
    )
}

fn mutation_fingerprint(
    scope: &ExactResourceScopeAttribution,
    entity_key: &str,
    display_name: &str,
    detection_provenance: &Value,
) -> StorageResult<String> {
    let payload = json!({
        "owner_account_id": scope.owner_account_id.to_string(),
        "actor_principal_id": scope.actor_principal_id.to_string(),
        "authenticated_session_id": scope.authenticated_session_id.to_string(),
        "access_space_id": scope.access_space_id.to_string(),
        "workspace_id": scope.workspace_id.as_str(),
        "entity_kind": KnowledgeEntityKind::UserManualPage.as_str(),
        "entity_key": entity_key,
        "display_name": display_name,
        "detection_provenance": detection_provenance,
    });
    Ok(sha256_hex(&serde_json::to_string(&payload)?))
}

fn knowledge_receipt_event(
    scope: &ExactResourceScopeAttribution,
    entity_id: &str,
    entity_key: &str,
    display_name: &str,
    detection_provenance: &Value,
    mutation_fingerprint: &str,
    predecessor_event_id: Option<&str>,
) -> StorageResult<NewKernelEvent> {
    let transition_fingerprint = sha256_hex(&format!(
        "{}\0{}",
        predecessor_event_id.unwrap_or("absent"),
        mutation_fingerprint
    ));
    NewKernelEvent::builder(
        format!("UMK-{transition_fingerprint}"),
        scope.authenticated_session_id.to_string(),
        KernelEventType::KnowledgeUserManualEntryRecorded,
        KernelActor::System("user_manual::bundle_bridge".to_owned()),
    )
    .aggregate("knowledge_entity", entity_id)
    .idempotency_key(format!("UMK-MUT-{transition_fingerprint}"))
    .source_component("user_manual::bundle_bridge")
    .payload(json!({
        "action": "user_manual_page_knowledge_entity_upserted",
        "entity_id": entity_id,
        "entity_kind": KnowledgeEntityKind::UserManualPage.as_str(),
        "entity_key": entity_key,
        "display_name": display_name,
        "detection_provenance": detection_provenance,
        "predecessor_event_id": predecessor_event_id,
        "mutation_fingerprint": mutation_fingerprint,
    }))
    .build()
    .map_err(|_| StorageError::Validation("UserManual knowledge receipt event is invalid"))
}

fn receipt_write(
    event: NewKernelEvent,
    scope: &ExactResourceScopeAttribution,
) -> KnowledgeReceiptWrite {
    let event = KernelEvent::from_new(event);
    KnowledgeReceiptWrite {
        event_id: event.event_id,
        event_version: event.event_version,
        kernel_task_run_id: event.kernel_task_run_id,
        session_run_id: event.session_run_id,
        aggregate_type: event.aggregate_type,
        aggregate_id: event.aggregate_id,
        idempotency_key: event.idempotency_key,
        event_type: event.event_type.to_string(),
        actor_kind: event.actor.actor_kind().to_owned(),
        actor_id: event.actor.actor_id().to_owned(),
        causation_id: event.causation_id,
        correlation_id: event.correlation_id,
        payload_hash: event.payload_hash,
        source_component: event.source_component,
        payload: event.payload,
        owner_account_id: scope.owner_account_id.to_string(),
        actor_principal_id: scope.actor_principal_id.to_string(),
        authenticated_session_id: scope.authenticated_session_id.to_string(),
        access_space_id: scope.access_space_id.to_string(),
        workspace_id: scope.workspace_id.as_str().to_owned(),
    }
}

fn one_mutation(
    rows: Vec<StoredKnowledgeEntity>,
) -> StorageResult<UserManualKnowledgeEntityMutation> {
    if rows.len() != 1 {
        return Err(StorageError::Validation(
            "UserManual knowledge transaction returned an invalid result",
        ));
    }
    stored_mutation(rows.into_iter().next().expect("one row checked"))
}

fn stored_mutation(row: StoredKnowledgeEntity) -> StorageResult<UserManualKnowledgeEntityMutation> {
    if row.mutation_fingerprint.len() != 64
        || !row
            .mutation_fingerprint
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(StorageError::Validation(
            "UserManual knowledge entity has an invalid mutation fingerprint",
        ));
    }
    Ok(UserManualKnowledgeEntityMutation {
        entity: KnowledgeEntity {
            entity_id: row.entity_id,
            workspace_id: row.workspace_id,
            entity_kind: row.entity_kind.parse()?,
            entity_key: row.entity_key,
            display_name: row.display_name,
            detection_provenance: row.detection_provenance,
            lifecycle_state: row.lifecycle_state.parse()?,
            primary_source_id: row.primary_source_id,
            first_detected_in_run: row.first_detected_in_run,
            last_detected_in_run: row.last_detected_in_run,
            created_at: row.created_at,
            updated_at: row.updated_at,
        },
        event_ledger_event_id: row.recorded_event_id,
        changed: row.changed,
    })
}

fn sha256_hex(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}
