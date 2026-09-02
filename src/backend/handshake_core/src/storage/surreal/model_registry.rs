use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use surrealdb::types::{RecordId, SurrealValue};

use crate::{
    kernel::{
        context_bundle::{canonical_json_bytes, sha256_hex},
        KernelActor, KernelEvent, KernelEventType, NewKernelEvent,
    },
    model_runtime::{
        artifact_locator_for_sha256, ensure_runtime_binding_matches, ensure_selection_matches,
        parse_provider, parse_runtime_binding, parse_runtime_role, provider_token,
        require_active_registration, runtime_binding_token, validate_rebind_request,
        validate_role_bound_registration_set, validate_selection, validate_selection_set,
        BaseModelTag, ExplicitModelRuntimeRebind, ModelCapabilities, ModelId, ModelRegistration,
        ModelRegistryLifecycleState, ModelRegistryPersistenceError, ModelRuntimeRole,
        ModelRuntimeSelection, ModelRuntimeSelectionPurpose, OperatorId,
        PersistedActiveModelSelection, PersistedModelRegistration, RoleBoundModelRegistration,
        MODEL_REGISTRY_INPUT_TEXT_BYTE_CAP, MODEL_REGISTRY_ROW_CAP,
        MODEL_RUNTIME_ACTIVE_SELECTION_EVENT_SCHEMA_ID, MODEL_RUNTIME_ACTIVE_SELECTION_SCHEMA_ID,
        MODEL_RUNTIME_CAPABILITIES_SCHEMA_ID, MODEL_RUNTIME_REGISTRY_SCHEMA_ID,
        MODEL_RUNTIME_SELECTION_EVENT_SCHEMA_ID,
    },
    swarm_orchestration::resource_scope::ExactResourceScopeAttribution,
};

use super::{SurrealStorage, SurrealStorageError};

const SCHEMA: &str = include_str!("model_registry_schema.surql");

#[derive(Clone)]
pub struct SurrealModelRegistryStore {
    storage: SurrealStorage,
}

#[derive(Debug, SurrealValue)]
struct ExactScopeBindings {
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: RecordId,
}

#[derive(Clone, Debug, SurrealValue)]
struct KernelReceiptWrite {
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
    created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, SurrealValue)]
struct RegistrationContent {
    registry_row_id: String,
    schema_id: String,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: RecordId,
    artifact_sha256: String,
    artifact_locator: String,
    last_observed_runtime_model_id: String,
    runtime_binding: String,
    runtime_role: String,
    capabilities_schema_id: String,
    capabilities: Value,
    provider: String,
    base_model_tag: String,
    last_observed_by: String,
    embedding_space_id: Option<String>,
    embedding_dimension: Option<i64>,
    lifecycle_state: String,
    selection_revision: i64,
    selection_event_id: RecordId,
    selection_created_event_id: RecordId,
    selection_updated_event_id: RecordId,
    current_selection_fingerprint: String,
    latest_mutation_fingerprint: String,
    mutation_fingerprint: String,
    last_rebind_request_fingerprint: Option<String>,
    registered_at_utc: DateTime<Utc>,
    selection_created_at_utc: DateTime<Utc>,
    selection_updated_at_utc: DateTime<Utc>,
    last_observed_at_utc: DateTime<Utc>,
}

#[derive(Clone, Debug, SurrealValue)]
struct BootMutation {
    record: RecordId,
    content: RegistrationContent,
    event: KernelReceiptWrite,
}

#[derive(Debug, SurrealValue)]
struct BootSetBindings {
    rows: Vec<BootMutation>,
    scope: ExactScopeBindings,
    artifact_sha256: Vec<String>,
}

#[derive(Clone, Debug, SurrealValue)]
struct StoredRegistration {
    registry_row_id: String,
    schema_id: String,
    artifact_sha256: String,
    artifact_locator: String,
    last_observed_runtime_model_id: String,
    runtime_binding: String,
    runtime_role: String,
    capabilities_schema_id: String,
    capabilities: Value,
    provider: String,
    base_model_tag: String,
    last_observed_by: String,
    lifecycle_state: String,
    selection_revision: i64,
    selection_created_event_id: String,
    selection_updated_event_id: String,
    selection_created_at_utc: DateTime<Utc>,
    selection_updated_at_utc: DateTime<Utc>,
    last_observed_at_utc: DateTime<Utc>,
    mutation_fingerprint: String,
    current_selection_fingerprint: String,
    latest_mutation_fingerprint: String,
    last_rebind_request_fingerprint: Option<String>,
}

#[derive(Debug, SurrealValue)]
struct ReadRegistrationBindings {
    scope: ExactScopeBindings,
    artifact_sha256: String,
}

#[derive(Debug, SurrealValue)]
struct ReadSetBindings {
    scope: ExactScopeBindings,
    artifact_sha256: Vec<String>,
}

#[derive(Debug, SurrealValue)]
struct RebindBindings {
    scope: ExactScopeBindings,
    artifact_sha256: String,
    expected_revision: i64,
    runtime_binding: String,
    runtime_role: String,
    capabilities: Value,
    provider: String,
    current_selection_fingerprint: String,
    request_fingerprint: String,
    event: KernelReceiptWrite,
}

#[derive(Clone, Debug, SurrealValue)]
struct ActiveSelectionContent {
    selection_id: String,
    schema_id: String,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: RecordId,
    purpose: String,
    runtime_role: String,
    artifact_sha256: String,
    lifecycle_state: String,
    selection_revision: i64,
    selection_created_event_id: RecordId,
    selection_updated_event_id: RecordId,
    latest_mutation_fingerprint: String,
    last_request_fingerprint: Option<String>,
    selection_created_at_utc: DateTime<Utc>,
    selection_updated_at_utc: DateTime<Utc>,
}

#[derive(Clone, Debug, SurrealValue)]
struct ActiveDefaultMutation {
    record: RecordId,
    content: ActiveSelectionContent,
    event: KernelReceiptWrite,
}

#[derive(Debug, SurrealValue)]
struct ActiveDefaultsBindings {
    rows: Vec<ActiveDefaultMutation>,
    scope: ExactScopeBindings,
    purposes: Vec<String>,
}

#[derive(Clone, Debug, SurrealValue)]
struct StoredActiveSelection {
    purpose: String,
    runtime_role: String,
    artifact_sha256: String,
    lifecycle_state: String,
    selection_revision: i64,
    selection_created_event_id: String,
    selection_updated_event_id: String,
    selection_created_at_utc: DateTime<Utc>,
    selection_updated_at_utc: DateTime<Utc>,
    latest_mutation_fingerprint: String,
    last_request_fingerprint: Option<String>,
}

#[derive(Debug, SurrealValue)]
struct ActiveReadBindings {
    scope: ExactScopeBindings,
}

#[derive(Debug, SurrealValue)]
struct ActiveSelectBindings {
    scope: ExactScopeBindings,
    purpose: String,
    runtime_role: String,
    target_artifact_sha256: String,
    expected_revision: i64,
    request_fingerprint: String,
    event: KernelReceiptWrite,
}

#[cfg(feature = "test-utils")]
#[derive(Debug, SurrealValue)]
struct TestArtifactBindings {
    scope: ExactScopeBindings,
    artifact_sha256: String,
}

#[cfg(feature = "test-utils")]
#[derive(Debug, SurrealValue)]
struct TestLifecycleBindings {
    scope: ExactScopeBindings,
    artifact_sha256: String,
    lifecycle_state: String,
}

#[cfg(feature = "test-utils")]
#[derive(Debug, SurrealValue)]
struct TestOrphanReceiptBindings {
    event: KernelReceiptWrite,
}

const READ_REGISTRATION_QUERY: &str = r#"
BEGIN TRANSACTION;
LET $rows = (SELECT registry_row_id, schema_id, artifact_sha256, artifact_locator,
    last_observed_runtime_model_id, runtime_binding, runtime_role,
    capabilities_schema_id, capabilities, provider, base_model_tag,
    last_observed_by, lifecycle_state, selection_revision,
    record::id(selection_created_event_id) AS selection_created_event_id,
    record::id(selection_updated_event_id) AS selection_updated_event_id,
    selection_created_at_utc, selection_updated_at_utc,
    last_observed_at_utc, mutation_fingerprint, current_selection_fingerprint,
    latest_mutation_fingerprint, last_rebind_request_fingerprint
FROM model_runtime_registry
WHERE owner_account_id = $scope.owner_account_id
  AND actor_principal_id = $scope.actor_principal_id
  AND authenticated_session_id = $scope.authenticated_session_id
  AND access_space_id = $scope.access_space_id
  AND workspace_id = $scope.workspace_id
  AND artifact_sha256 = $artifact_sha256
LIMIT 2);
LET $receipts = IF array::len($rows) = 1 {
    (SELECT event_id FROM kernel_event_ledger
     WHERE event_id = $rows[0].selection_updated_event_id
       AND aggregate_type = 'model_runtime_registry'
       AND aggregate_id = $rows[0].registry_row_id
       AND payload.mutation_fingerprint = $rows[0].latest_mutation_fingerprint
       AND payload.selection_revision = $rows[0].selection_revision
       AND owner_account_id = $scope.owner_account_id
       AND actor_principal_id = $scope.actor_principal_id
       AND authenticated_session_id = $scope.authenticated_session_id
       AND access_space_id = $scope.access_space_id
       AND workspace_id = record::id($scope.workspace_id)
     LIMIT 2)
} ELSE { [] };
LET $created_receipts = IF array::len($rows) = 1 {
    (SELECT event_id FROM kernel_event_ledger
     WHERE event_id = $rows[0].selection_created_event_id
       AND event_type = 'MODEL_RUNTIME_SELECTION_RECORDED'
       AND aggregate_type = 'model_runtime_registry'
       AND aggregate_id = $rows[0].registry_row_id
       AND payload.mutation_fingerprint = $rows[0].mutation_fingerprint
       AND payload.selection_revision = 1
       AND owner_account_id = $scope.owner_account_id
       AND actor_principal_id = $scope.actor_principal_id
       AND authenticated_session_id = $scope.authenticated_session_id
       AND access_space_id = $scope.access_space_id
       AND workspace_id = record::id($scope.workspace_id)
     LIMIT 2)
} ELSE { [] };
IF array::len($rows) > 1 OR array::len($receipts) > 1
   OR array::len($created_receipts) > 1 {
    THROW 'model registry exact-scope identity or receipt is ambiguous';
} ELSE IF array::len($rows) = 1
   AND (array::len($receipts) != 1 OR array::len($created_receipts) != 1) {
    THROW 'model registry mutation exists without canonical latest receipt';
};
RETURN $rows;
COMMIT TRANSACTION;
"#;

const READ_REGISTRATION_SET_QUERY: &str = r#"
BEGIN TRANSACTION;
LET $rows = (SELECT registry_row_id, schema_id, artifact_sha256, artifact_locator,
    last_observed_runtime_model_id, runtime_binding, runtime_role,
    capabilities_schema_id, capabilities, provider, base_model_tag,
    last_observed_by, lifecycle_state, selection_revision,
    record::id(selection_created_event_id) AS selection_created_event_id,
    record::id(selection_updated_event_id) AS selection_updated_event_id,
    selection_created_at_utc, selection_updated_at_utc,
    last_observed_at_utc, mutation_fingerprint, current_selection_fingerprint,
    latest_mutation_fingerprint, last_rebind_request_fingerprint
FROM model_runtime_registry
WHERE owner_account_id = $scope.owner_account_id
  AND actor_principal_id = $scope.actor_principal_id
  AND authenticated_session_id = $scope.authenticated_session_id
  AND access_space_id = $scope.access_space_id
  AND workspace_id = $scope.workspace_id
  AND artifact_sha256 IN $artifact_sha256
ORDER BY artifact_sha256 ASC
LIMIT 4097);
IF array::len($rows) > 4096 {
    THROW 'model registry exact-scope enumeration exceeds bounded row cap';
};
FOR $row IN $rows {
    LET $receipts = (SELECT event_id FROM kernel_event_ledger
     WHERE event_id = $row.selection_updated_event_id
       AND aggregate_type = 'model_runtime_registry'
       AND aggregate_id = $row.registry_row_id
       AND payload.mutation_fingerprint = $row.latest_mutation_fingerprint
       AND payload.selection_revision = $row.selection_revision
       AND owner_account_id = $scope.owner_account_id
       AND actor_principal_id = $scope.actor_principal_id
       AND authenticated_session_id = $scope.authenticated_session_id
       AND access_space_id = $scope.access_space_id
       AND workspace_id = record::id($scope.workspace_id)
     LIMIT 2);
    LET $created_receipts = (SELECT event_id FROM kernel_event_ledger
     WHERE event_id = $row.selection_created_event_id
       AND event_type = 'MODEL_RUNTIME_SELECTION_RECORDED'
       AND aggregate_type = 'model_runtime_registry'
       AND aggregate_id = $row.registry_row_id
       AND payload.mutation_fingerprint = $row.mutation_fingerprint
       AND payload.selection_revision = 1
       AND owner_account_id = $scope.owner_account_id
       AND actor_principal_id = $scope.actor_principal_id
       AND authenticated_session_id = $scope.authenticated_session_id
       AND access_space_id = $scope.access_space_id
       AND workspace_id = record::id($scope.workspace_id)
     LIMIT 2);
    IF array::len($receipts) != 1 OR array::len($created_receipts) != 1 {
        THROW 'model registry set contains missing or ambiguous receipt evidence';
    };
};
RETURN $rows;
COMMIT TRANSACTION;
"#;

const PERSIST_BOOT_SET_QUERY: &str = r#"
BEGIN TRANSACTION;
FOR $item IN $rows {
    LET $existing = (SELECT id, registry_row_id, lifecycle_state,
        mutation_fingerprint, current_selection_fingerprint, latest_mutation_fingerprint,
        selection_revision,
        record::id(selection_created_event_id) AS selection_created_event_id,
        record::id(selection_updated_event_id) AS selection_updated_event_id
    FROM model_runtime_registry
    WHERE owner_account_id = $item.content.owner_account_id
      AND actor_principal_id = $item.content.actor_principal_id
      AND authenticated_session_id = $item.content.authenticated_session_id
      AND access_space_id = $item.content.access_space_id
      AND workspace_id = $item.content.workspace_id
      AND artifact_sha256 = $item.content.artifact_sha256
    LIMIT 2);
    LET $latest_receipt = IF array::len($existing) = 1 {
        (SELECT event_id FROM kernel_event_ledger
         WHERE event_id = $existing[0].selection_updated_event_id
           AND aggregate_type = 'model_runtime_registry'
           AND aggregate_id = $existing[0].registry_row_id
           AND payload.mutation_fingerprint = $existing[0].latest_mutation_fingerprint
           AND payload.selection_revision = $existing[0].selection_revision
           AND owner_account_id = $item.content.owner_account_id
           AND actor_principal_id = $item.content.actor_principal_id
           AND authenticated_session_id = $item.content.authenticated_session_id
           AND access_space_id = $item.content.access_space_id
           AND workspace_id = record::id($item.content.workspace_id)
         LIMIT 2)
    } ELSE { [] };
    LET $created_receipt = IF array::len($existing) = 1 {
        (SELECT event_id FROM kernel_event_ledger
         WHERE event_id = $existing[0].selection_created_event_id
           AND event_type = 'MODEL_RUNTIME_SELECTION_RECORDED'
           AND aggregate_type = 'model_runtime_registry'
           AND aggregate_id = $existing[0].registry_row_id
           AND payload.mutation_fingerprint = $existing[0].mutation_fingerprint
           AND payload.selection_revision = 1
           AND owner_account_id = $item.content.owner_account_id
           AND actor_principal_id = $item.content.actor_principal_id
           AND authenticated_session_id = $item.content.authenticated_session_id
           AND access_space_id = $item.content.access_space_id
           AND workspace_id = record::id($item.content.workspace_id)
         LIMIT 2)
    } ELSE { [] };
    LET $event_existing = (SELECT event_id FROM kernel_event_ledger
     WHERE idempotency_key = $item.event.idempotency_key LIMIT 2);
    IF array::len($existing) > 1 OR array::len($latest_receipt) > 1
       OR array::len($created_receipt) > 1
       OR array::len($event_existing) > 1 {
        THROW 'model registry boot-set identity or receipt is ambiguous';
    } ELSE IF array::len($existing) = 1 {
        IF $existing[0].lifecycle_state != 'active' {
            THROW 'model registry boot-set selected stale or revoked authority';
        } ELSE IF $existing[0].current_selection_fingerprint != $item.content.current_selection_fingerprint {
            THROW 'model registry boot-set immutable selection conflict';
        } ELSE IF array::len($latest_receipt) != 1
           OR array::len($created_receipt) != 1 {
            THROW 'model registry boot-set mutation exists without canonical receipt';
        };
        UPDATE $existing[0].id SET
            last_observed_runtime_model_id = $item.content.last_observed_runtime_model_id,
            base_model_tag = $item.content.base_model_tag,
            last_observed_by = $item.content.last_observed_by,
            last_observed_at_utc = $item.content.last_observed_at_utc;
    } ELSE {
        IF array::len($event_existing) != 0 {
            THROW 'model registry boot receipt exists without its mutation';
        };
        CREATE type::record('kernel_event_ledger', $item.event.event_id) CONTENT $item.event;
        CREATE $item.record CONTENT $item.content;
    };
};
RETURN SELECT registry_row_id, schema_id, artifact_sha256, artifact_locator,
    last_observed_runtime_model_id, runtime_binding, runtime_role,
    capabilities_schema_id, capabilities, provider, base_model_tag,
    last_observed_by, lifecycle_state, selection_revision,
    record::id(selection_created_event_id) AS selection_created_event_id,
    record::id(selection_updated_event_id) AS selection_updated_event_id,
    selection_created_at_utc, selection_updated_at_utc,
    last_observed_at_utc, mutation_fingerprint, current_selection_fingerprint,
    latest_mutation_fingerprint, last_rebind_request_fingerprint
FROM model_runtime_registry
WHERE owner_account_id = $scope.owner_account_id
  AND actor_principal_id = $scope.actor_principal_id
  AND authenticated_session_id = $scope.authenticated_session_id
  AND access_space_id = $scope.access_space_id
  AND workspace_id = $scope.workspace_id
  AND artifact_sha256 IN $artifact_sha256
ORDER BY artifact_sha256 ASC;
COMMIT TRANSACTION;
"#;

const REBIND_SELECTION_QUERY: &str = r#"
BEGIN TRANSACTION;
LET $rows = (SELECT id, registry_row_id, lifecycle_state, runtime_role,
    selection_revision, current_selection_fingerprint,
    latest_mutation_fingerprint, last_rebind_request_fingerprint,
    record::id(selection_updated_event_id) AS selection_updated_event_id
FROM model_runtime_registry
WHERE owner_account_id = $scope.owner_account_id
  AND actor_principal_id = $scope.actor_principal_id
  AND authenticated_session_id = $scope.authenticated_session_id
  AND access_space_id = $scope.access_space_id
  AND workspace_id = $scope.workspace_id
  AND artifact_sha256 = $artifact_sha256
LIMIT 2);
LET $latest_receipt = IF array::len($rows) = 1 {
    (SELECT event_id FROM kernel_event_ledger
     WHERE event_id = $rows[0].selection_updated_event_id
       AND aggregate_type = 'model_runtime_registry'
       AND aggregate_id = $rows[0].registry_row_id
       AND payload.mutation_fingerprint = $rows[0].latest_mutation_fingerprint
       AND payload.selection_revision = $rows[0].selection_revision
       AND owner_account_id = $scope.owner_account_id
       AND actor_principal_id = $scope.actor_principal_id
       AND authenticated_session_id = $scope.authenticated_session_id
       AND access_space_id = $scope.access_space_id
       AND workspace_id = record::id($scope.workspace_id)
     LIMIT 2)
} ELSE { [] };
LET $request_receipt = (SELECT event_id FROM kernel_event_ledger
 WHERE idempotency_key = $event.idempotency_key LIMIT 2);
IF array::len($rows) > 1 OR array::len($latest_receipt) > 1
   OR array::len($request_receipt) > 1 {
    THROW 'model registry rebind identity or receipt is ambiguous';
} ELSE IF array::len($rows) = 0 {
    THROW 'model registry rebind selection is absent';
} ELSE IF array::len($latest_receipt) != 1 {
    THROW 'model registry rebind source lacks canonical receipt';
} ELSE IF $rows[0].last_rebind_request_fingerprint = $request_fingerprint
   AND $rows[0].selection_revision = $expected_revision + 1
   AND $rows[0].current_selection_fingerprint = $current_selection_fingerprint {
    IF array::len($request_receipt) != 1
       OR $request_receipt[0].event_id != $rows[0].selection_updated_event_id {
        THROW 'model registry identical rebind retry has orphan or mismatched receipt';
    };
    RETURN SELECT registry_row_id, schema_id, artifact_sha256, artifact_locator,
        last_observed_runtime_model_id, runtime_binding, runtime_role,
        capabilities_schema_id, capabilities, provider, base_model_tag,
        last_observed_by, lifecycle_state, selection_revision,
        record::id(selection_created_event_id) AS selection_created_event_id,
        record::id(selection_updated_event_id) AS selection_updated_event_id,
        selection_created_at_utc, selection_updated_at_utc,
        last_observed_at_utc, mutation_fingerprint, current_selection_fingerprint,
        latest_mutation_fingerprint, last_rebind_request_fingerprint
    FROM $rows[0].id;
} ELSE {
    IF array::len($request_receipt) != 0 {
        THROW 'model registry rebind receipt exists without matching mutation';
    } ELSE IF $rows[0].lifecycle_state != 'active' {
        THROW 'model registry rebind denied stale or revoked selection';
    } ELSE IF $rows[0].runtime_role != $runtime_role {
        THROW 'model registry rebind cannot change runtime role';
    } ELSE IF $rows[0].selection_revision != $expected_revision {
        THROW 'model registry rebind revision mismatch';
    } ELSE IF $rows[0].current_selection_fingerprint = $current_selection_fingerprint {
        THROW 'model registry rebind target is unchanged';
    };
    CREATE type::record('kernel_event_ledger', $event.event_id) CONTENT $event;
    UPDATE $rows[0].id SET
        runtime_binding = $runtime_binding,
        capabilities = $capabilities,
        provider = $provider,
        selection_revision = $expected_revision + 1,
        selection_updated_event_id = type::record('kernel_event_ledger', $event.event_id),
        selection_updated_at_utc = $event.created_at,
        current_selection_fingerprint = $current_selection_fingerprint,
        latest_mutation_fingerprint = $request_fingerprint,
        last_rebind_request_fingerprint = $request_fingerprint;
    RETURN SELECT registry_row_id, schema_id, artifact_sha256, artifact_locator,
        last_observed_runtime_model_id, runtime_binding, runtime_role,
        capabilities_schema_id, capabilities, provider, base_model_tag,
        last_observed_by, lifecycle_state, selection_revision,
        record::id(selection_created_event_id) AS selection_created_event_id,
        record::id(selection_updated_event_id) AS selection_updated_event_id,
        selection_created_at_utc, selection_updated_at_utc,
        last_observed_at_utc, mutation_fingerprint, current_selection_fingerprint,
        latest_mutation_fingerprint, last_rebind_request_fingerprint
    FROM $rows[0].id;
};
COMMIT TRANSACTION;
"#;

const ACTIVE_SELECTION_READ_QUERY: &str = r#"
BEGIN TRANSACTION;
LET $rows = (SELECT purpose, runtime_role, artifact_sha256, lifecycle_state,
    selection_revision,
    record::id(selection_created_event_id) AS selection_created_event_id,
    record::id(selection_updated_event_id) AS selection_updated_event_id,
    selection_created_at_utc, selection_updated_at_utc,
    latest_mutation_fingerprint, last_request_fingerprint
FROM model_runtime_active_selection
WHERE owner_account_id = $scope.owner_account_id
  AND actor_principal_id = $scope.actor_principal_id
  AND authenticated_session_id = $scope.authenticated_session_id
  AND access_space_id = $scope.access_space_id
  AND workspace_id = $scope.workspace_id
ORDER BY purpose ASC
LIMIT 3);
IF array::len($rows) > 2 {
    THROW 'model registry active selection authority has too many purposes';
};
FOR $row IN $rows {
    LET $receipt = (SELECT event_id FROM kernel_event_ledger
     WHERE event_id = $row.selection_updated_event_id
       AND aggregate_type = 'model_runtime_active_selection'
       AND aggregate_id = $row.purpose
       AND payload.mutation_fingerprint = $row.latest_mutation_fingerprint
       AND payload.selection_revision = $row.selection_revision
       AND owner_account_id = $scope.owner_account_id
       AND actor_principal_id = $scope.actor_principal_id
       AND authenticated_session_id = $scope.authenticated_session_id
       AND access_space_id = $scope.access_space_id
       AND workspace_id = record::id($scope.workspace_id)
     LIMIT 2);
    LET $created_receipt = (SELECT event_id FROM kernel_event_ledger
     WHERE event_id = $row.selection_created_event_id
       AND event_type = 'MODEL_RUNTIME_SELECTION_RECORDED'
       AND aggregate_type = 'model_runtime_active_selection'
       AND aggregate_id = $row.purpose
       AND payload.purpose = $row.purpose
       AND payload.runtime_role = $row.runtime_role
       AND payload.selection_revision = 1
       AND owner_account_id = $scope.owner_account_id
       AND actor_principal_id = $scope.actor_principal_id
       AND authenticated_session_id = $scope.authenticated_session_id
       AND access_space_id = $scope.access_space_id
       AND workspace_id = record::id($scope.workspace_id)
     LIMIT 2);
    LET $selected = (SELECT runtime_role, lifecycle_state FROM model_runtime_registry
     WHERE owner_account_id = $scope.owner_account_id
       AND actor_principal_id = $scope.actor_principal_id
       AND authenticated_session_id = $scope.authenticated_session_id
       AND access_space_id = $scope.access_space_id
       AND workspace_id = $scope.workspace_id
       AND artifact_sha256 = $row.artifact_sha256
     LIMIT 2);
    IF array::len($receipt) != 1 OR array::len($created_receipt) != 1
       OR array::len($selected) != 1
       OR $selected[0].lifecycle_state != 'active'
       OR $selected[0].runtime_role != $row.runtime_role {
        THROW 'model registry active selection lacks canonical active role-bound authority';
    };
};
RETURN $rows;
COMMIT TRANSACTION;
"#;

const ENSURE_ACTIVE_DEFAULTS_QUERY: &str = r#"
BEGIN TRANSACTION;
FOR $item IN $rows {
    LET $candidate = (SELECT runtime_role, lifecycle_state FROM model_runtime_registry
     WHERE owner_account_id = $item.content.owner_account_id
       AND actor_principal_id = $item.content.actor_principal_id
       AND authenticated_session_id = $item.content.authenticated_session_id
       AND access_space_id = $item.content.access_space_id
       AND workspace_id = $item.content.workspace_id
       AND artifact_sha256 = $item.content.artifact_sha256
     LIMIT 2);
    LET $existing = (SELECT id, purpose, artifact_sha256, runtime_role,
        lifecycle_state, selection_revision, latest_mutation_fingerprint,
        record::id(selection_created_event_id) AS selection_created_event_id,
        record::id(selection_updated_event_id) AS selection_updated_event_id
     FROM model_runtime_active_selection
     WHERE owner_account_id = $item.content.owner_account_id
       AND actor_principal_id = $item.content.actor_principal_id
       AND authenticated_session_id = $item.content.authenticated_session_id
       AND access_space_id = $item.content.access_space_id
       AND workspace_id = $item.content.workspace_id
       AND purpose = $item.content.purpose
     LIMIT 2);
    LET $selected = IF array::len($existing) = 1 {
        (SELECT runtime_role, lifecycle_state FROM model_runtime_registry
         WHERE owner_account_id = $item.content.owner_account_id
           AND actor_principal_id = $item.content.actor_principal_id
           AND authenticated_session_id = $item.content.authenticated_session_id
           AND access_space_id = $item.content.access_space_id
           AND workspace_id = $item.content.workspace_id
           AND artifact_sha256 = $existing[0].artifact_sha256
         LIMIT 2)
    } ELSE { [] };
    LET $existing_receipt = IF array::len($existing) = 1 {
        (SELECT event_id FROM kernel_event_ledger
         WHERE event_id = $existing[0].selection_updated_event_id
           AND aggregate_type = 'model_runtime_active_selection'
           AND aggregate_id = $existing[0].purpose
           AND payload.mutation_fingerprint = $existing[0].latest_mutation_fingerprint
           AND payload.selection_revision = $existing[0].selection_revision
           AND owner_account_id = $item.content.owner_account_id
           AND actor_principal_id = $item.content.actor_principal_id
           AND authenticated_session_id = $item.content.authenticated_session_id
           AND access_space_id = $item.content.access_space_id
           AND workspace_id = record::id($item.content.workspace_id)
         LIMIT 2)
    } ELSE { [] };
    LET $created_receipt = IF array::len($existing) = 1 {
        (SELECT event_id FROM kernel_event_ledger
         WHERE event_id = $existing[0].selection_created_event_id
           AND event_type = 'MODEL_RUNTIME_SELECTION_RECORDED'
           AND aggregate_type = 'model_runtime_active_selection'
           AND aggregate_id = $existing[0].purpose
           AND payload.purpose = $existing[0].purpose
           AND payload.runtime_role = $existing[0].runtime_role
           AND payload.selection_revision = 1
           AND owner_account_id = $item.content.owner_account_id
           AND actor_principal_id = $item.content.actor_principal_id
           AND authenticated_session_id = $item.content.authenticated_session_id
           AND access_space_id = $item.content.access_space_id
           AND workspace_id = record::id($item.content.workspace_id)
         LIMIT 2)
    } ELSE { [] };
    LET $event_existing = (SELECT event_id FROM kernel_event_ledger
     WHERE idempotency_key = $item.event.idempotency_key LIMIT 2);
    IF array::len($candidate) != 1 OR $candidate[0].lifecycle_state != 'active'
       OR $candidate[0].runtime_role != $item.content.runtime_role {
        THROW 'model registry active default candidate is absent, inactive, or role-incompatible';
    } ELSE IF array::len($existing) > 1 OR array::len($existing_receipt) > 1
       OR array::len($created_receipt) > 1
       OR array::len($event_existing) > 1 {
        THROW 'model registry active default identity or receipt is ambiguous';
    } ELSE IF array::len($existing) = 1 {
        IF $existing[0].lifecycle_state != 'active'
           OR $existing[0].runtime_role != $item.content.runtime_role
           OR array::len($selected) != 1
           OR $selected[0].lifecycle_state != 'active'
           OR $selected[0].runtime_role != $item.content.runtime_role
           OR array::len($existing_receipt) != 1
           OR array::len($created_receipt) != 1 {
            THROW 'model registry active default is inactive, role-invalid, or unaudited';
        };
    } ELSE {
        IF array::len($event_existing) != 0 {
            THROW 'model registry active-default receipt exists without mutation';
        };
        CREATE type::record('kernel_event_ledger', $item.event.event_id) CONTENT $item.event;
        CREATE $item.record CONTENT $item.content;
    };
};
RETURN SELECT purpose, runtime_role, artifact_sha256, lifecycle_state,
    selection_revision,
    record::id(selection_created_event_id) AS selection_created_event_id,
    record::id(selection_updated_event_id) AS selection_updated_event_id,
    selection_created_at_utc, selection_updated_at_utc,
    latest_mutation_fingerprint, last_request_fingerprint
FROM model_runtime_active_selection
WHERE owner_account_id = $scope.owner_account_id
  AND actor_principal_id = $scope.actor_principal_id
  AND authenticated_session_id = $scope.authenticated_session_id
  AND access_space_id = $scope.access_space_id
  AND workspace_id = $scope.workspace_id
  AND purpose IN $purposes
ORDER BY purpose ASC;
COMMIT TRANSACTION;
"#;

const SELECT_ACTIVE_MODEL_QUERY: &str = r#"
BEGIN TRANSACTION;
LET $target = (SELECT runtime_role, lifecycle_state FROM model_runtime_registry
 WHERE owner_account_id = $scope.owner_account_id
   AND actor_principal_id = $scope.actor_principal_id
   AND authenticated_session_id = $scope.authenticated_session_id
   AND access_space_id = $scope.access_space_id
   AND workspace_id = $scope.workspace_id
   AND artifact_sha256 = $target_artifact_sha256
 LIMIT 2);
LET $rows = (SELECT id, purpose, runtime_role, artifact_sha256,
    lifecycle_state, selection_revision, latest_mutation_fingerprint,
    last_request_fingerprint,
    record::id(selection_updated_event_id) AS selection_updated_event_id
 FROM model_runtime_active_selection
 WHERE owner_account_id = $scope.owner_account_id
   AND actor_principal_id = $scope.actor_principal_id
   AND authenticated_session_id = $scope.authenticated_session_id
   AND access_space_id = $scope.access_space_id
   AND workspace_id = $scope.workspace_id
   AND purpose = $purpose
 LIMIT 2);
LET $latest_receipt = IF array::len($rows) = 1 {
    (SELECT event_id FROM kernel_event_ledger
     WHERE event_id = $rows[0].selection_updated_event_id
       AND aggregate_type = 'model_runtime_active_selection'
       AND aggregate_id = $rows[0].purpose
       AND payload.mutation_fingerprint = $rows[0].latest_mutation_fingerprint
       AND payload.selection_revision = $rows[0].selection_revision
       AND owner_account_id = $scope.owner_account_id
       AND actor_principal_id = $scope.actor_principal_id
       AND authenticated_session_id = $scope.authenticated_session_id
       AND access_space_id = $scope.access_space_id
       AND workspace_id = record::id($scope.workspace_id)
     LIMIT 2)
} ELSE { [] };
LET $request_receipt = (SELECT event_id FROM kernel_event_ledger
 WHERE idempotency_key = $event.idempotency_key LIMIT 2);
IF array::len($target) != 1 OR $target[0].lifecycle_state != 'active'
   OR $target[0].runtime_role != $runtime_role {
    THROW 'model registry active selection target is absent, inactive, or role-incompatible';
} ELSE IF array::len($rows) != 1 OR array::len($latest_receipt) > 1
   OR array::len($request_receipt) > 1 {
    THROW 'model registry active selection identity or receipt is invalid';
} ELSE IF array::len($latest_receipt) != 1 {
    THROW 'model registry active selection source lacks canonical receipt';
} ELSE IF $rows[0].last_request_fingerprint = $request_fingerprint
   AND $rows[0].selection_revision = $expected_revision + 1
   AND $rows[0].artifact_sha256 = $target_artifact_sha256 {
    IF array::len($request_receipt) != 1
       OR $request_receipt[0].event_id != $rows[0].selection_updated_event_id {
        THROW 'model registry identical active-selection retry has mismatched receipt';
    };
    RETURN SELECT purpose, runtime_role, artifact_sha256, lifecycle_state,
        selection_revision,
        record::id(selection_created_event_id) AS selection_created_event_id,
        record::id(selection_updated_event_id) AS selection_updated_event_id,
        selection_created_at_utc, selection_updated_at_utc,
        latest_mutation_fingerprint, last_request_fingerprint
    FROM $rows[0].id;
} ELSE {
    IF array::len($request_receipt) != 0 {
        THROW 'model registry active-selection receipt exists without mutation';
    } ELSE IF $rows[0].lifecycle_state != 'active' {
        THROW 'model registry active selection is stale or revoked';
    } ELSE IF $rows[0].selection_revision != $expected_revision {
        THROW 'model registry active selection revision mismatch';
    } ELSE IF $rows[0].artifact_sha256 = $target_artifact_sha256 {
        THROW 'model registry active selection target is unchanged';
    };
    CREATE type::record('kernel_event_ledger', $event.event_id) CONTENT $event;
    UPDATE $rows[0].id SET
        artifact_sha256 = $target_artifact_sha256,
        selection_revision = $expected_revision + 1,
        selection_updated_event_id = type::record('kernel_event_ledger', $event.event_id),
        selection_updated_at_utc = $event.created_at,
        latest_mutation_fingerprint = $request_fingerprint,
        last_request_fingerprint = $request_fingerprint;
    RETURN SELECT purpose, runtime_role, artifact_sha256, lifecycle_state,
        selection_revision,
        record::id(selection_created_event_id) AS selection_created_event_id,
        record::id(selection_updated_event_id) AS selection_updated_event_id,
        selection_created_at_utc, selection_updated_at_utc,
        latest_mutation_fingerprint, last_request_fingerprint
    FROM $rows[0].id;
};
COMMIT TRANSACTION;
"#;

const LIST_REGISTRATIONS_QUERY: &str = r#"
BEGIN TRANSACTION;
LET $rows = (SELECT registry_row_id, schema_id, artifact_sha256, artifact_locator,
    last_observed_runtime_model_id, runtime_binding, runtime_role,
    capabilities_schema_id, capabilities, provider, base_model_tag,
    last_observed_by, lifecycle_state, selection_revision,
    record::id(selection_created_event_id) AS selection_created_event_id,
    record::id(selection_updated_event_id) AS selection_updated_event_id,
    selection_created_at_utc, selection_updated_at_utc,
    last_observed_at_utc, mutation_fingerprint, current_selection_fingerprint,
    latest_mutation_fingerprint, last_rebind_request_fingerprint
FROM model_runtime_registry
WHERE owner_account_id = $owner_account_id
  AND actor_principal_id = $actor_principal_id
  AND authenticated_session_id = $authenticated_session_id
  AND access_space_id = $access_space_id
  AND workspace_id = $workspace_id
ORDER BY artifact_sha256 ASC
LIMIT 4097);
IF array::len($rows) > 4096 {
    THROW 'model registry exact-scope enumeration exceeds bounded row cap';
};
FOR $row IN $rows {
    LET $receipt = (SELECT event_id FROM kernel_event_ledger
     WHERE event_id = $row.selection_updated_event_id
       AND aggregate_type = 'model_runtime_registry'
       AND aggregate_id = $row.registry_row_id
       AND payload.mutation_fingerprint = $row.latest_mutation_fingerprint
       AND payload.selection_revision = $row.selection_revision
       AND owner_account_id = $owner_account_id
       AND actor_principal_id = $actor_principal_id
       AND authenticated_session_id = $authenticated_session_id
       AND access_space_id = $access_space_id
       AND workspace_id = record::id($workspace_id)
     LIMIT 2);
    LET $created_receipt = (SELECT event_id FROM kernel_event_ledger
     WHERE event_id = $row.selection_created_event_id
       AND event_type = 'MODEL_RUNTIME_SELECTION_RECORDED'
       AND aggregate_type = 'model_runtime_registry'
       AND aggregate_id = $row.registry_row_id
       AND payload.mutation_fingerprint = $row.mutation_fingerprint
       AND payload.selection_revision = 1
       AND owner_account_id = $owner_account_id
       AND actor_principal_id = $actor_principal_id
       AND authenticated_session_id = $authenticated_session_id
       AND access_space_id = $access_space_id
       AND workspace_id = record::id($workspace_id)
     LIMIT 2);
    IF array::len($receipt) != 1 OR array::len($created_receipt) != 1 {
        THROW 'model registry enumeration found missing or ambiguous receipt';
    };
};
RETURN $rows;
COMMIT TRANSACTION;
"#;

impl SurrealModelRegistryStore {
    pub fn new(storage: SurrealStorage) -> Self {
        Self { storage }
    }

    pub fn storage(&self) -> &SurrealStorage {
        &self.storage
    }

    pub async fn ensure_authority_available(
        &self,
        scope: &ExactResourceScopeAttribution,
    ) -> Result<(), ModelRegistryPersistenceError> {
        self.list_recoverable(scope).await.map(|_| ())
    }

    pub async fn recover_configured_selection_set(
        &self,
        scope: &ExactResourceScopeAttribution,
        configured: &[ModelRuntimeSelection],
    ) -> Result<Vec<Option<PersistedModelRegistration>>, ModelRegistryPersistenceError> {
        validate_selection_set(configured)?;
        let hashes = configured
            .iter()
            .map(|selection| hex::encode(selection.artifact_sha256))
            .collect::<Vec<_>>();
        let rows = self.read_registration_set(scope, hashes).await?;
        let by_hash = rows
            .into_iter()
            .map(|row| (row.artifact_sha256, row))
            .collect::<BTreeMap<_, _>>();
        configured
            .iter()
            .map(|selection| {
                let row = by_hash.get(&selection.artifact_sha256).cloned();
                if let Some(row) = &row {
                    ensure_selection_matches(row, selection)?;
                }
                Ok(row)
            })
            .collect()
    }

    pub async fn recover_configured_runtime_binding_set(
        &self,
        scope: &ExactResourceScopeAttribution,
        configured: &[ModelRuntimeSelection],
    ) -> Result<Vec<Option<PersistedModelRegistration>>, ModelRegistryPersistenceError> {
        validate_selection_set(configured)?;
        let hashes = configured
            .iter()
            .map(|selection| hex::encode(selection.artifact_sha256))
            .collect::<Vec<_>>();
        let rows = self.read_registration_set(scope, hashes).await?;
        let by_hash = rows
            .into_iter()
            .map(|row| (row.artifact_sha256, row))
            .collect::<BTreeMap<_, _>>();
        configured
            .iter()
            .map(|selection| {
                let row = by_hash.get(&selection.artifact_sha256).cloned();
                if let Some(row) = &row {
                    ensure_runtime_binding_matches(row, selection)?;
                }
                Ok(row)
            })
            .collect()
    }

    pub async fn recover_configured_selection(
        &self,
        scope: &ExactResourceScopeAttribution,
        configured: &ModelRuntimeSelection,
    ) -> Result<Option<PersistedModelRegistration>, ModelRegistryPersistenceError> {
        let mut rows = self
            .recover_configured_selection_set(scope, std::slice::from_ref(configured))
            .await?;
        Ok(rows.pop().expect("single selection preserves cardinality"))
    }

    pub async fn persist_boot_set_and_read_back(
        &self,
        scope: &ExactResourceScopeAttribution,
        registrations: &[ModelRegistration],
    ) -> Result<Vec<PersistedModelRegistration>, ModelRegistryPersistenceError> {
        let role_bound = registrations
            .iter()
            .cloned()
            .map(RoleBoundModelRegistration::completion)
            .collect::<Vec<_>>();
        self.persist_role_bound_boot_set_and_read_back(scope, &role_bound)
            .await
    }

    pub async fn persist_role_bound_boot_set_and_read_back(
        &self,
        scope: &ExactResourceScopeAttribution,
        registrations: &[RoleBoundModelRegistration],
    ) -> Result<Vec<PersistedModelRegistration>, ModelRegistryPersistenceError> {
        let selections = validate_role_bound_registration_set(registrations)?;
        if registrations.is_empty() {
            return Ok(Vec::new());
        }
        let mut rows = Vec::with_capacity(registrations.len());
        let mut hashes = Vec::with_capacity(registrations.len());
        for (role_bound, selection) in registrations.iter().zip(&selections) {
            let selection_fingerprint = selection_fingerprint(scope, selection)?;
            let registry_row_id = registry_row_id(scope, selection)?;
            let event = initial_registration_event(
                scope,
                &registry_row_id,
                role_bound,
                &selection_fingerprint,
            )?;
            let receipt = receipt_write(event, scope);
            let event_record = RecordId::new("kernel_event_ledger", receipt.event_id.clone());
            let (embedding_space_id, embedding_dimension) = embedding_identity(selection)?;
            let now = receipt.created_at.to_owned();
            let artifact_sha256 = hex::encode(selection.artifact_sha256);
            hashes.push(artifact_sha256.clone());
            rows.push(BootMutation {
                record: RecordId::new("model_runtime_registry", registry_row_id.clone()),
                content: RegistrationContent {
                    registry_row_id,
                    schema_id: MODEL_RUNTIME_REGISTRY_SCHEMA_ID.to_owned(),
                    owner_account_id: scope.owner_account_id.to_string(),
                    actor_principal_id: scope.actor_principal_id.to_string(),
                    authenticated_session_id: scope.authenticated_session_id.to_string(),
                    access_space_id: scope.access_space_id.to_string(),
                    workspace_id: workspace_record(scope),
                    artifact_sha256,
                    artifact_locator: artifact_locator_for_sha256(selection.artifact_sha256),
                    last_observed_runtime_model_id: role_bound.registration.model_id.to_string(),
                    runtime_binding: runtime_binding_token(selection.runtime_binding).to_owned(),
                    runtime_role: selection.runtime_role.as_str().to_owned(),
                    capabilities_schema_id: MODEL_RUNTIME_CAPABILITIES_SCHEMA_ID.to_owned(),
                    capabilities: serde_json::to_value(&selection.declared_capabilities)?,
                    provider: provider_token(selection.provider).to_owned(),
                    base_model_tag: role_bound.registration.base_model_tag.as_str().to_owned(),
                    last_observed_by: role_bound.registration.registered_by.as_str().to_owned(),
                    embedding_space_id,
                    embedding_dimension,
                    lifecycle_state: ModelRegistryLifecycleState::Active.as_str().to_owned(),
                    selection_revision: 1,
                    selection_event_id: event_record.clone(),
                    selection_created_event_id: event_record.clone(),
                    selection_updated_event_id: event_record,
                    current_selection_fingerprint: selection_fingerprint.clone(),
                    latest_mutation_fingerprint: selection_fingerprint.clone(),
                    mutation_fingerprint: selection_fingerprint,
                    last_rebind_request_fingerprint: None,
                    registered_at_utc: role_bound.registration.registered_at_utc.to_owned(),
                    selection_created_at_utc: now.to_owned(),
                    selection_updated_at_utc: now,
                    last_observed_at_utc: Utc::now(),
                },
                event: receipt,
            });
        }
        let stored = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values_at::<StoredRegistration, _>(
                            PERSIST_BOOT_SET_QUERY,
                            BootSetBindings {
                                rows,
                                scope: exact_scope_bindings(scope),
                                artifact_sha256: hashes,
                            },
                            2,
                        )
                        .await
                })
            })
            .await
            .map_err(storage_error)?;
        if stored.len() != registrations.len() {
            return Err(ModelRegistryPersistenceError::AuthorityUnavailable(
                format!(
                    "boot-set readback returned {} rows for {} registrations",
                    stored.len(),
                    registrations.len()
                ),
            ));
        }
        let decoded = stored
            .into_iter()
            .map(|row| decode_registration(row, scope))
            .collect::<Result<Vec<_>, _>>()?;
        let by_hash = decoded
            .into_iter()
            .map(|row| (row.artifact_sha256, row))
            .collect::<BTreeMap<_, _>>();
        registrations
            .iter()
            .zip(&selections)
            .map(|(registration, selection)| {
                let row = by_hash
                    .get(&selection.artifact_sha256)
                    .cloned()
                    .ok_or_else(|| {
                        ModelRegistryPersistenceError::AuthorityUnavailable(format!(
                            "boot-set readback omitted artifact {}",
                            hex::encode(selection.artifact_sha256)
                        ))
                    })?;
                ensure_selection_matches(&row, selection)?;
                if row.last_observed_runtime_model_id != registration.registration.model_id
                    || row.base_model_tag != registration.registration.base_model_tag
                    || row.last_observed_by != registration.registration.registered_by
                {
                    return Err(ModelRegistryPersistenceError::ObservationMismatch(format!(
                        "artifact {} observation readback differs from the current boot",
                        hex::encode(selection.artifact_sha256)
                    )));
                }
                Ok(row)
            })
            .collect()
    }

    pub async fn ensure_active_defaults(
        &self,
        scope: &ExactResourceScopeAttribution,
        candidates: &[(ModelRuntimeSelectionPurpose, [u8; 32])],
    ) -> Result<Vec<PersistedActiveModelSelection>, ModelRegistryPersistenceError> {
        let mut seen = BTreeMap::new();
        for (purpose, artifact_sha256) in candidates {
            if seen.insert(*purpose, *artifact_sha256).is_some() {
                return Err(ModelRegistryPersistenceError::InvalidRegistration(format!(
                    "active default candidate set contains duplicate purpose {}",
                    purpose.as_str()
                )));
            }
        }
        if candidates.is_empty() {
            return Ok(Vec::new());
        }
        let mut rows = Vec::with_capacity(candidates.len());
        let mut purposes = Vec::with_capacity(candidates.len());
        for (purpose, artifact_sha256) in candidates {
            let mutation_fingerprint =
                active_default_fingerprint(scope, *purpose, artifact_sha256)?;
            let selection_id = active_selection_id(scope, *purpose);
            let event = active_selection_event(
                scope,
                *purpose,
                None,
                artifact_sha256,
                1,
                &mutation_fingerprint,
                KernelActor::System("model-runtime-registry".to_owned()),
                "initial active default selected during boot",
            )?;
            let receipt = receipt_write(event, scope);
            let event_record = RecordId::new("kernel_event_ledger", receipt.event_id.clone());
            let now = receipt.created_at.to_owned();
            purposes.push(purpose.as_str().to_owned());
            rows.push(ActiveDefaultMutation {
                record: RecordId::new("model_runtime_active_selection", selection_id.clone()),
                content: ActiveSelectionContent {
                    selection_id,
                    schema_id: MODEL_RUNTIME_ACTIVE_SELECTION_SCHEMA_ID.to_owned(),
                    owner_account_id: scope.owner_account_id.to_string(),
                    actor_principal_id: scope.actor_principal_id.to_string(),
                    authenticated_session_id: scope.authenticated_session_id.to_string(),
                    access_space_id: scope.access_space_id.to_string(),
                    workspace_id: workspace_record(scope),
                    purpose: purpose.as_str().to_owned(),
                    runtime_role: purpose.runtime_role().as_str().to_owned(),
                    artifact_sha256: hex::encode(artifact_sha256),
                    lifecycle_state: ModelRegistryLifecycleState::Active.as_str().to_owned(),
                    selection_revision: 1,
                    selection_created_event_id: event_record.clone(),
                    selection_updated_event_id: event_record,
                    latest_mutation_fingerprint: mutation_fingerprint,
                    last_request_fingerprint: None,
                    selection_created_at_utc: now.to_owned(),
                    selection_updated_at_utc: now,
                },
                event: receipt,
            });
        }
        let stored = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values_at::<StoredActiveSelection, _>(
                            ENSURE_ACTIVE_DEFAULTS_QUERY,
                            ActiveDefaultsBindings {
                                rows,
                                scope: exact_scope_bindings(scope),
                                purposes,
                            },
                            2,
                        )
                        .await
                })
            })
            .await
            .map_err(storage_error)?;
        if stored.len() != candidates.len() {
            return Err(ModelRegistryPersistenceError::AuthorityUnavailable(
                format!(
                    "active-default readback returned {} rows for {} purposes",
                    stored.len(),
                    candidates.len()
                ),
            ));
        }
        stored.into_iter().map(decode_active_selection).collect()
    }

    pub async fn list_active_selections(
        &self,
        scope: &ExactResourceScopeAttribution,
    ) -> Result<Vec<PersistedActiveModelSelection>, ModelRegistryPersistenceError> {
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values_at::<StoredActiveSelection, _>(
                            ACTIVE_SELECTION_READ_QUERY,
                            ActiveReadBindings {
                                scope: exact_scope_bindings(scope),
                            },
                            4,
                        )
                        .await
                })
            })
            .await
            .map_err(storage_error)?;
        rows.into_iter().map(decode_active_selection).collect()
    }

    pub async fn select_active_model(
        &self,
        scope: &ExactResourceScopeAttribution,
        purpose: ModelRuntimeSelectionPurpose,
        target_artifact_sha256: [u8; 32],
        expected_revision: u64,
        actor: KernelActor,
        reason: &str,
    ) -> Result<PersistedActiveModelSelection, ModelRegistryPersistenceError> {
        validate_operator_mutation(&actor, reason, expected_revision)?;
        let current = self
            .list_active_selections(scope)
            .await?
            .into_iter()
            .find(|selection| selection.purpose == purpose)
            .ok_or_else(|| {
                ModelRegistryPersistenceError::SelectionNotFound(purpose.as_str().to_owned())
            })?;
        let request_fingerprint = active_selection_request_fingerprint(
            scope,
            purpose,
            &target_artifact_sha256,
            expected_revision,
            &actor,
            reason,
        )?;
        if current.selection_revision != expected_revision
            && !(current.selection_revision == expected_revision.saturating_add(1)
                && current.artifact_sha256 == target_artifact_sha256)
        {
            return Err(ModelRegistryPersistenceError::SelectionRevisionMismatch {
                expected: expected_revision,
                actual: current.selection_revision,
            });
        }
        let event = active_selection_event(
            scope,
            purpose,
            Some(&current),
            &target_artifact_sha256,
            expected_revision.checked_add(1).ok_or_else(|| {
                ModelRegistryPersistenceError::InvalidRebind(
                    "active selection revision cannot be incremented".to_owned(),
                )
            })?,
            &request_fingerprint,
            actor,
            reason,
        )?;
        let expected_revision_i64 = i64::try_from(expected_revision).map_err(|_| {
            ModelRegistryPersistenceError::InvalidRebind(
                "active selection revision exceeds i64".to_owned(),
            )
        })?;
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values_at::<StoredActiveSelection, _>(
                            SELECT_ACTIVE_MODEL_QUERY,
                            ActiveSelectBindings {
                                scope: exact_scope_bindings(scope),
                                purpose: purpose.as_str().to_owned(),
                                runtime_role: purpose.runtime_role().as_str().to_owned(),
                                target_artifact_sha256: hex::encode(target_artifact_sha256),
                                expected_revision: expected_revision_i64,
                                request_fingerprint,
                                event: receipt_write(event, scope),
                            },
                            5,
                        )
                        .await
                })
            })
            .await
            .map_err(storage_error)?;
        decode_active_selection(one(rows, "active selection mutation")?)
    }

    pub async fn persist_and_read_back(
        &self,
        scope: &ExactResourceScopeAttribution,
        registration: &ModelRegistration,
    ) -> Result<PersistedModelRegistration, ModelRegistryPersistenceError> {
        let mut rows = self
            .persist_boot_set_and_read_back(scope, std::slice::from_ref(registration))
            .await?;
        Ok(rows
            .pop()
            .expect("single registration preserves cardinality"))
    }

    pub async fn rebind_selection_after_verified_unload(
        &self,
        scope: &ExactResourceScopeAttribution,
        target: &ModelRuntimeSelection,
        request: ExplicitModelRuntimeRebind,
    ) -> Result<PersistedModelRegistration, ModelRegistryPersistenceError> {
        validate_selection(target)?;
        validate_rebind_request(&request)?;
        let existing = self
            .load_by_artifact_sha256(scope, &target.artifact_sha256)
            .await?
            .ok_or_else(|| {
                ModelRegistryPersistenceError::SelectionNotFound(hex::encode(
                    target.artifact_sha256,
                ))
            })?;
        require_active_registration(&existing)?;
        if existing.runtime_role != target.runtime_role {
            return Err(ModelRegistryPersistenceError::InvalidRebind(
                "runtime role is artifact authority and cannot be rebound".to_owned(),
            ));
        }
        let request_fingerprint = rebind_request_fingerprint(scope, target, &request)?;
        let retry = existing.selection_revision
            == request.expected_selection_revision().saturating_add(1)
            && existing.selection() == *target
            && existing.last_rebind_request_fingerprint.as_deref()
                == Some(request_fingerprint.as_str());
        if existing.selection_revision != request.expected_selection_revision() && !retry {
            return Err(ModelRegistryPersistenceError::SelectionRevisionMismatch {
                expected: request.expected_selection_revision(),
                actual: existing.selection_revision,
            });
        }
        if existing.selection() == *target && !retry {
            return Err(ModelRegistryPersistenceError::InvalidRebind(
                "target immutable selection is unchanged".to_owned(),
            ));
        }
        let next_revision = request
            .expected_selection_revision()
            .checked_add(1)
            .ok_or_else(|| {
                ModelRegistryPersistenceError::InvalidRebind(
                    "selection revision cannot be incremented".to_owned(),
                )
            })?;
        let event = rebind_event(
            scope,
            &existing,
            target,
            &request,
            next_revision,
            &request_fingerprint,
        )?;
        let expected_revision =
            i64::try_from(request.expected_selection_revision()).map_err(|_| {
                ModelRegistryPersistenceError::InvalidRebind(
                    "expected selection revision exceeds i64".to_owned(),
                )
            })?;
        let capabilities = serde_json::to_value(&target.declared_capabilities)?;
        let current_selection_fingerprint = selection_fingerprint(scope, target)?;
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values_at::<StoredRegistration, _>(
                            REBIND_SELECTION_QUERY,
                            RebindBindings {
                                scope: exact_scope_bindings(scope),
                                artifact_sha256: hex::encode(target.artifact_sha256),
                                expected_revision,
                                runtime_binding: runtime_binding_token(target.runtime_binding)
                                    .to_owned(),
                                runtime_role: target.runtime_role.as_str().to_owned(),
                                capabilities,
                                provider: provider_token(target.provider).to_owned(),
                                current_selection_fingerprint,
                                request_fingerprint,
                                event: receipt_write(event, scope),
                            },
                            4,
                        )
                        .await
                })
            })
            .await
            .map_err(storage_error)?;
        let committed = decode_registration(one(rows, "selection rebind")?, scope)?;
        ensure_selection_matches(&committed, target)?;
        if committed.selection_revision != next_revision {
            return Err(ModelRegistryPersistenceError::AuthorityUnavailable(
                "selection rebind returned an unstable revision".to_owned(),
            ));
        }
        Ok(committed)
    }

    #[cfg(feature = "test-utils")]
    pub async fn rebind_selection_for_tests(
        &self,
        scope: &ExactResourceScopeAttribution,
        target: &ModelRuntimeSelection,
        request: ExplicitModelRuntimeRebind,
    ) -> Result<PersistedModelRegistration, ModelRegistryPersistenceError> {
        self.rebind_selection_after_verified_unload(scope, target, request)
            .await
    }

    /// Creates the exact workspace predecessor required by registry records.
    /// This bounded seam exists only for integration tests; production callers
    /// must resolve an existing workspace through normal composition.
    #[cfg(feature = "test-utils")]
    pub async fn ensure_workspace_for_tests(
        &self,
        scope: &ExactResourceScopeAttribution,
    ) -> Result<(), ModelRegistryPersistenceError> {
        #[derive(Debug, SurrealValue)]
        struct WorkspaceFixture {
            name: String,
            updated_at: DateTime<Utc>,
        }

        self.storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .upsert_one::<surrealdb::types::Value, _>(
                            "workspaces",
                            scope.workspace_id.as_str(),
                            WorkspaceFixture {
                                name: "model registry embedded test workspace".to_owned(),
                                updated_at: Utc::now(),
                            },
                        )
                        .await
                })
            })
            .await
            .map_err(storage_error)?;
        Ok(())
    }

    /// Injects the canonical initial receipt without its registry mutation.
    /// The next identical boot write must fail closed as an orphan receipt.
    #[cfg(feature = "test-utils")]
    pub async fn inject_orphan_initial_receipt_for_tests(
        &self,
        scope: &ExactResourceScopeAttribution,
        registration: &RoleBoundModelRegistration,
    ) -> Result<String, ModelRegistryPersistenceError> {
        let selections = validate_role_bound_registration_set(std::slice::from_ref(registration))?;
        let selection = selections
            .into_iter()
            .next()
            .expect("single validated registration preserves cardinality");
        let mutation_fingerprint = selection_fingerprint(scope, &selection)?;
        let row_id = registry_row_id(scope, &selection)?;
        let event = receipt_write(
            initial_registration_event(scope, &row_id, registration, &mutation_fingerprint)?,
            scope,
        );
        let event_id = event.event_id.clone();
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values::<surrealdb::types::Value, _>(
                            "CREATE type::record('kernel_event_ledger', $event.event_id) CONTENT $event;",
                            TestOrphanReceiptBindings { event },
                        )
                        .await
                })
            })
            .await
            .map_err(storage_error)?;
        if rows.len() != 1 {
            return Err(ModelRegistryPersistenceError::AuthorityUnavailable(
                "orphan-receipt test seam did not create exactly one receipt".to_owned(),
            ));
        }
        Ok(event_id)
    }

    /// Changes only lifecycle state for stale/revoked denial proofs.
    #[cfg(feature = "test-utils")]
    pub async fn set_registration_lifecycle_for_tests(
        &self,
        scope: &ExactResourceScopeAttribution,
        artifact_sha256: &[u8; 32],
        lifecycle_state: ModelRegistryLifecycleState,
    ) -> Result<(), ModelRegistryPersistenceError> {
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values::<surrealdb::types::Value, _>(
                            "UPDATE model_runtime_registry SET lifecycle_state = $lifecycle_state \
                             WHERE owner_account_id = $scope.owner_account_id \
                               AND actor_principal_id = $scope.actor_principal_id \
                               AND authenticated_session_id = $scope.authenticated_session_id \
                               AND access_space_id = $scope.access_space_id \
                               AND workspace_id = $scope.workspace_id \
                               AND artifact_sha256 = $artifact_sha256 RETURN AFTER;",
                            TestLifecycleBindings {
                                scope: exact_scope_bindings(scope),
                                artifact_sha256: hex::encode(artifact_sha256),
                                lifecycle_state: lifecycle_state.as_str().to_owned(),
                            },
                        )
                        .await
                })
            })
            .await
            .map_err(storage_error)?;
        require_single_test_mutation(rows, "lifecycle")
    }

    /// Corrupts the latest canonical receipt while preserving its referenced
    /// record, proving readers validate receipt semantics rather than presence.
    #[cfg(feature = "test-utils")]
    pub async fn corrupt_latest_receipt_for_tests(
        &self,
        scope: &ExactResourceScopeAttribution,
        artifact_sha256: &[u8; 32],
    ) -> Result<(), ModelRegistryPersistenceError> {
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values_at::<surrealdb::types::Value, _>(
                            "BEGIN TRANSACTION; \
                             LET $rows = (SELECT record::id(selection_updated_event_id) AS event_id \
                               FROM model_runtime_registry \
                               WHERE owner_account_id = $scope.owner_account_id \
                                 AND actor_principal_id = $scope.actor_principal_id \
                                 AND authenticated_session_id = $scope.authenticated_session_id \
                                 AND access_space_id = $scope.access_space_id \
                                 AND workspace_id = $scope.workspace_id \
                                 AND artifact_sha256 = $artifact_sha256 LIMIT 2); \
                             IF array::len($rows) != 1 { THROW 'receipt tamper target is not unique'; }; \
                             UPDATE type::record('kernel_event_ledger', $rows[0].event_id) \
                               SET aggregate_id = 'tampered-model-registry-aggregate' RETURN AFTER; \
                             COMMIT TRANSACTION;",
                            TestArtifactBindings {
                                scope: exact_scope_bindings(scope),
                                artifact_sha256: hex::encode(artifact_sha256),
                            },
                            3,
                        )
                        .await
                })
            })
            .await
            .map_err(storage_error)?;
        require_single_test_mutation(rows, "latest receipt")
    }

    /// Advances the projection revision without changing its receipt. A read
    /// must reject the resulting mutation-without-receipt inconsistency.
    #[cfg(feature = "test-utils")]
    pub async fn advance_projection_without_receipt_for_tests(
        &self,
        scope: &ExactResourceScopeAttribution,
        artifact_sha256: &[u8; 32],
    ) -> Result<(), ModelRegistryPersistenceError> {
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values::<surrealdb::types::Value, _>(
                            "UPDATE model_runtime_registry SET selection_revision += 1 \
                             WHERE owner_account_id = $scope.owner_account_id \
                               AND actor_principal_id = $scope.actor_principal_id \
                               AND authenticated_session_id = $scope.authenticated_session_id \
                               AND access_space_id = $scope.access_space_id \
                               AND workspace_id = $scope.workspace_id \
                               AND artifact_sha256 = $artifact_sha256 RETURN AFTER;",
                            TestArtifactBindings {
                                scope: exact_scope_bindings(scope),
                                artifact_sha256: hex::encode(artifact_sha256),
                            },
                        )
                        .await
                })
            })
            .await
            .map_err(storage_error)?;
        require_single_test_mutation(rows, "projection revision")
    }

    pub async fn load_by_artifact_sha256(
        &self,
        scope: &ExactResourceScopeAttribution,
        artifact_sha256: &[u8; 32],
    ) -> Result<Option<PersistedModelRegistration>, ModelRegistryPersistenceError> {
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values_at::<StoredRegistration, _>(
                            READ_REGISTRATION_QUERY,
                            ReadRegistrationBindings {
                                scope: exact_scope_bindings(scope),
                                artifact_sha256: hex::encode(artifact_sha256),
                            },
                            5,
                        )
                        .await
                })
            })
            .await
            .map_err(storage_error)?;
        match rows.len() {
            0 => Ok(None),
            1 => decode_registration(rows.into_iter().next().expect("length checked"), scope)
                .map(Some),
            _ => Err(ModelRegistryPersistenceError::CorruptRow(
                "exact-scope artifact identity returned more than one row".to_owned(),
            )),
        }
    }

    pub async fn list_recoverable(
        &self,
        scope: &ExactResourceScopeAttribution,
    ) -> Result<Vec<PersistedModelRegistration>, ModelRegistryPersistenceError> {
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values_at::<StoredRegistration, _>(
                            LIST_REGISTRATIONS_QUERY,
                            exact_scope_bindings(scope),
                            4,
                        )
                        .await
                })
            })
            .await
            .map_err(storage_error)?;
        if rows.len() > MODEL_REGISTRY_ROW_CAP {
            return Err(ModelRegistryPersistenceError::AuthorityUnavailable(
                "model registry enumeration exceeded bounded row cap".to_owned(),
            ));
        }
        rows.into_iter()
            .map(|row| decode_registration(row, scope))
            .collect()
    }

    async fn read_registration_set(
        &self,
        scope: &ExactResourceScopeAttribution,
        artifact_sha256: Vec<String>,
    ) -> Result<Vec<PersistedModelRegistration>, ModelRegistryPersistenceError> {
        if artifact_sha256.is_empty() {
            return Ok(Vec::new());
        }
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values_at::<StoredRegistration, _>(
                            READ_REGISTRATION_SET_QUERY,
                            ReadSetBindings {
                                scope: exact_scope_bindings(scope),
                                artifact_sha256,
                            },
                            4,
                        )
                        .await
                })
            })
            .await
            .map_err(storage_error)?;
        rows.into_iter()
            .map(|row| decode_registration(row, scope))
            .collect()
    }
}

pub async fn bootstrap_model_registry_schema(
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
        workspace_id: workspace_record(scope),
    }
}

fn workspace_record(scope: &ExactResourceScopeAttribution) -> RecordId {
    RecordId::new("workspaces", scope.workspace_id.as_str().to_owned())
}

fn scope_json(scope: &ExactResourceScopeAttribution) -> Value {
    json!({
        "owner_account_id": scope.owner_account_id.to_string(),
        "actor_principal_id": scope.actor_principal_id.to_string(),
        "authenticated_session_id": scope.authenticated_session_id.to_string(),
        "access_space_id": scope.access_space_id.to_string(),
        "workspace_id": scope.workspace_id.as_str(),
    })
}

fn fingerprint(value: &Value) -> String {
    sha256_hex(&canonical_json_bytes(value))
}

fn selection_json(selection: &ModelRuntimeSelection) -> Result<Value, serde_json::Error> {
    Ok(json!({
        "artifact_sha256": hex::encode(selection.artifact_sha256),
        "runtime_binding": runtime_binding_token(selection.runtime_binding),
        "runtime_role": selection.runtime_role.as_str(),
        "capabilities_schema_id": MODEL_RUNTIME_CAPABILITIES_SCHEMA_ID,
        "declared_capabilities": serde_json::to_value(&selection.declared_capabilities)?,
        "provider": provider_token(selection.provider),
    }))
}

fn selection_fingerprint(
    scope: &ExactResourceScopeAttribution,
    selection: &ModelRuntimeSelection,
) -> Result<String, ModelRegistryPersistenceError> {
    Ok(fingerprint(&json!({
        "scope": scope_json(scope),
        "selection": selection_json(selection)?,
    })))
}

fn embedding_identity(
    selection: &ModelRuntimeSelection,
) -> Result<(Option<String>, Option<i64>), ModelRegistryPersistenceError> {
    if selection.runtime_role != ModelRuntimeRole::Embedding {
        return Ok((None, None));
    }
    let dimension = selection
        .declared_capabilities
        .embedding_dimension
        .expect("embedding role validation requires a dimension");
    let artifact_sha256 = hex::encode(selection.artifact_sha256);
    let digest = format!(
        "{:x}",
        Sha256::digest(format!("{artifact_sha256}\0{dimension}").as_bytes())
    );
    let dimension_i64 = i64::try_from(dimension).map_err(|_| {
        ModelRegistryPersistenceError::InvalidRegistration(
            "embedding dimension exceeds embedded database integer range".to_owned(),
        )
    })?;
    Ok((Some(format!("EMS-{digest}")), Some(dimension_i64)))
}

fn registry_row_id(
    scope: &ExactResourceScopeAttribution,
    selection: &ModelRuntimeSelection,
) -> Result<String, ModelRegistryPersistenceError> {
    let identity = embedding_identity(selection)?
        .0
        .unwrap_or_else(|| format!("completion:{}", hex::encode(selection.artifact_sha256)));
    let digest = format!(
        "{:x}",
        Sha256::digest(
            format!(
                "{}\0{}\0{}\0{}\0{}\0{}",
                scope.owner_account_id,
                scope.actor_principal_id,
                scope.authenticated_session_id,
                scope.access_space_id,
                scope.workspace_id.as_str(),
                identity
            )
            .as_bytes()
        )
    );
    Ok(format!("MRR-{digest}"))
}

fn active_selection_id(
    scope: &ExactResourceScopeAttribution,
    purpose: ModelRuntimeSelectionPurpose,
) -> String {
    let digest = format!(
        "{:x}",
        Sha256::digest(
            format!(
                "{}\0{}\0{}\0{}\0{}\0{}",
                scope.owner_account_id,
                scope.actor_principal_id,
                scope.authenticated_session_id,
                scope.access_space_id,
                scope.workspace_id.as_str(),
                purpose.as_str()
            )
            .as_bytes()
        )
    );
    format!("MRAS-{digest}")
}

fn active_default_fingerprint(
    scope: &ExactResourceScopeAttribution,
    purpose: ModelRuntimeSelectionPurpose,
    artifact_sha256: &[u8; 32],
) -> Result<String, ModelRegistryPersistenceError> {
    Ok(fingerprint(&json!({
        "schema_id": MODEL_RUNTIME_ACTIVE_SELECTION_EVENT_SCHEMA_ID,
        "scope": scope_json(scope),
        "purpose": purpose.as_str(),
        "runtime_role": purpose.runtime_role().as_str(),
        "target_artifact_sha256": hex::encode(artifact_sha256),
        "selection_revision": 1,
    })))
}

fn rebind_request_fingerprint(
    scope: &ExactResourceScopeAttribution,
    target: &ModelRuntimeSelection,
    request: &ExplicitModelRuntimeRebind,
) -> Result<String, ModelRegistryPersistenceError> {
    Ok(fingerprint(&json!({
        "scope": scope_json(scope),
        "target": selection_json(target)?,
        "expected_selection_revision": request.expected_selection_revision(),
        "actor_kind": request.actor().actor_kind(),
        "actor_id": request.actor().actor_id(),
        "reason": request.reason(),
    })))
}

fn active_selection_request_fingerprint(
    scope: &ExactResourceScopeAttribution,
    purpose: ModelRuntimeSelectionPurpose,
    target_artifact_sha256: &[u8; 32],
    expected_revision: u64,
    actor: &KernelActor,
    reason: &str,
) -> Result<String, ModelRegistryPersistenceError> {
    Ok(fingerprint(&json!({
        "scope": scope_json(scope),
        "purpose": purpose.as_str(),
        "target_artifact_sha256": hex::encode(target_artifact_sha256),
        "expected_selection_revision": expected_revision,
        "actor_kind": actor.actor_kind(),
        "actor_id": actor.actor_id(),
        "reason": reason,
    })))
}

fn initial_registration_event(
    scope: &ExactResourceScopeAttribution,
    registry_row_id: &str,
    registration: &RoleBoundModelRegistration,
    mutation_fingerprint: &str,
) -> Result<NewKernelEvent, ModelRegistryPersistenceError> {
    let selection = registration.selection();
    NewKernelEvent::builder(
        format!("MODELREG-{mutation_fingerprint}"),
        scope.authenticated_session_id.to_string(),
        KernelEventType::ModelRuntimeSelectionRecorded,
        KernelActor::System("model-runtime-registry".to_owned()),
    )
    .aggregate("model_runtime_registry", registry_row_id)
    .idempotency_key(format!(
        "model-runtime-registry:initial:{mutation_fingerprint}"
    ))
    .source_component("model_runtime_registry")
    .payload(json!({
        "schema_id": MODEL_RUNTIME_SELECTION_EVENT_SCHEMA_ID,
        "action": "model_runtime_selection_recorded",
        "registry_row_id": registry_row_id,
        "artifact_sha256": hex::encode(selection.artifact_sha256),
        "selection": selection_json(&selection)?,
        "base_model_tag": registration.registration.base_model_tag.as_str(),
        "last_observed_by": registration.registration.registered_by.as_str(),
        "selection_revision": 1,
        "mutation_fingerprint": mutation_fingerprint,
    }))
    .build()
    .map_err(|error| ModelRegistryPersistenceError::Audit(error.to_string()))
}

fn rebind_event(
    scope: &ExactResourceScopeAttribution,
    existing: &PersistedModelRegistration,
    target: &ModelRuntimeSelection,
    request: &ExplicitModelRuntimeRebind,
    next_revision: u64,
    mutation_fingerprint: &str,
) -> Result<NewKernelEvent, ModelRegistryPersistenceError> {
    NewKernelEvent::builder(
        format!("MODELREG-REBIND-{mutation_fingerprint}"),
        scope.authenticated_session_id.to_string(),
        KernelEventType::ModelRuntimeSelectionRebound,
        request.actor().clone(),
    )
    .aggregate("model_runtime_registry", &existing.registry_row_id)
    .idempotency_key(format!(
        "model-runtime-registry:rebind:{mutation_fingerprint}"
    ))
    .source_component("model_runtime_registry")
    .causation_id(existing.selection_updated_event_id.clone())
    .correlation_id(existing.selection_created_event_id.clone())
    .payload(json!({
        "schema_id": MODEL_RUNTIME_SELECTION_EVENT_SCHEMA_ID,
        "action": "model_runtime_selection_rebound_after_verified_unload",
        "artifact_sha256": hex::encode(target.artifact_sha256),
        "runtime_role": target.runtime_role.as_str(),
        "previous_selection": selection_json(&existing.selection())?,
        "target_selection": selection_json(target)?,
        "previous_selection_revision": request.expected_selection_revision(),
        "selection_revision": next_revision,
        "reason": request.reason(),
        "mutation_fingerprint": mutation_fingerprint,
    }))
    .build()
    .map_err(|error| ModelRegistryPersistenceError::Audit(error.to_string()))
}

#[allow(clippy::too_many_arguments)]
fn active_selection_event(
    scope: &ExactResourceScopeAttribution,
    purpose: ModelRuntimeSelectionPurpose,
    previous: Option<&PersistedActiveModelSelection>,
    target_artifact_sha256: &[u8; 32],
    next_revision: u64,
    mutation_fingerprint: &str,
    actor: KernelActor,
    reason: &str,
) -> Result<NewKernelEvent, ModelRegistryPersistenceError> {
    let event_type = if previous.is_some() {
        KernelEventType::ModelRuntimeSelectionRebound
    } else {
        KernelEventType::ModelRuntimeSelectionRecorded
    };
    let mut builder = NewKernelEvent::builder(
        format!("MODELREG-ACTIVE-{mutation_fingerprint}"),
        scope.authenticated_session_id.to_string(),
        event_type,
        actor,
    )
    .aggregate("model_runtime_active_selection", purpose.as_str())
    .idempotency_key(format!("model-runtime-active:{mutation_fingerprint}"))
    .source_component("model_runtime_registry")
    .payload(json!({
        "schema_id": MODEL_RUNTIME_ACTIVE_SELECTION_EVENT_SCHEMA_ID,
        "action": if previous.is_some() { "active_default_changed" } else { "active_default_initialized" },
        "purpose": purpose.as_str(),
        "runtime_role": purpose.runtime_role().as_str(),
        "previous_artifact_sha256": previous.map(|row| hex::encode(row.artifact_sha256)),
        "target_artifact_sha256": hex::encode(target_artifact_sha256),
        "previous_selection_revision": previous.map(|row| row.selection_revision),
        "selection_revision": next_revision,
        "reason": reason,
        "mutation_fingerprint": mutation_fingerprint,
    }));
    if let Some(previous) = previous {
        builder = builder
            .causation_id(previous.selection_updated_event_id.clone())
            .correlation_id(previous.selection_created_event_id.clone());
    }
    builder
        .build()
        .map_err(|error| ModelRegistryPersistenceError::Audit(error.to_string()))
}

fn receipt_write(
    event: NewKernelEvent,
    scope: &ExactResourceScopeAttribution,
) -> KernelReceiptWrite {
    let event = KernelEvent::from_new(event);
    KernelReceiptWrite {
        event_id: event.event_id,
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
        owner_account_id: scope.owner_account_id.to_string(),
        actor_principal_id: scope.actor_principal_id.to_string(),
        authenticated_session_id: scope.authenticated_session_id.to_string(),
        access_space_id: scope.access_space_id.to_string(),
        workspace_id: scope.workspace_id.as_str().to_owned(),
        created_at: event.created_at,
    }
}

fn decode_registration(
    row: StoredRegistration,
    scope: &ExactResourceScopeAttribution,
) -> Result<PersistedModelRegistration, ModelRegistryPersistenceError> {
    if row.schema_id != MODEL_RUNTIME_REGISTRY_SCHEMA_ID {
        return Err(ModelRegistryPersistenceError::CorruptRow(format!(
            "registry schema is `{}`, expected `{MODEL_RUNTIME_REGISTRY_SCHEMA_ID}`",
            row.schema_id
        )));
    }
    if row.capabilities_schema_id != MODEL_RUNTIME_CAPABILITIES_SCHEMA_ID {
        return Err(ModelRegistryPersistenceError::CorruptRow(
            "registry capabilities schema is not canonical".to_owned(),
        ));
    }
    let artifact_bytes = hex::decode(&row.artifact_sha256).map_err(|_| {
        ModelRegistryPersistenceError::CorruptRow(
            "registry artifact SHA-256 is not lowercase hexadecimal".to_owned(),
        )
    })?;
    let artifact_sha256: [u8; 32] = artifact_bytes.try_into().map_err(|bytes: Vec<u8>| {
        ModelRegistryPersistenceError::CorruptRow(format!(
            "registry artifact SHA-256 is {} bytes",
            bytes.len()
        ))
    })?;
    let lifecycle_state = parse_lifecycle(&row.lifecycle_state)?;
    let selection_revision = u64::try_from(row.selection_revision).map_err(|_| {
        ModelRegistryPersistenceError::CorruptRow(
            "registry selection revision is not positive".to_owned(),
        )
    })?;
    if selection_revision == 0
        || row.selection_created_event_id.trim().is_empty()
        || row.selection_updated_event_id.trim().is_empty()
        || row.mutation_fingerprint.len() != 64
        || row.latest_mutation_fingerprint.len() != 64
        || row.current_selection_fingerprint.len() != 64
    {
        return Err(ModelRegistryPersistenceError::CorruptRow(
            "registry revision, fingerprint, or receipt identity is invalid".to_owned(),
        ));
    }
    let persisted = PersistedModelRegistration {
        schema_id: row.schema_id,
        registry_row_id: row.registry_row_id,
        artifact_sha256,
        artifact_locator: row.artifact_locator,
        last_observed_runtime_model_id: ModelId::from(parse_uuid(
            &row.last_observed_runtime_model_id,
            "last observed model id",
        )?),
        runtime_binding: parse_runtime_binding(&row.runtime_binding)?,
        runtime_role: parse_runtime_role(&row.runtime_role)?,
        capabilities_schema_id: row.capabilities_schema_id,
        declared_capabilities: serde_json::from_value::<ModelCapabilities>(row.capabilities)?,
        provider: parse_provider(&row.provider)?,
        base_model_tag: BaseModelTag::try_new(row.base_model_tag)
            .map_err(|error| ModelRegistryPersistenceError::CorruptRow(error.to_string()))?,
        last_observed_by: OperatorId::try_new(row.last_observed_by)
            .map_err(|error| ModelRegistryPersistenceError::CorruptRow(error.to_string()))?,
        lifecycle_state,
        selection_revision,
        current_selection_fingerprint: row.current_selection_fingerprint.clone(),
        latest_mutation_fingerprint: row.latest_mutation_fingerprint.clone(),
        last_rebind_request_fingerprint: row.last_rebind_request_fingerprint.clone(),
        selection_created_event_id: row.selection_created_event_id,
        selection_updated_event_id: row.selection_updated_event_id,
        selection_created_at_utc: row.selection_created_at_utc,
        selection_updated_at_utc: row.selection_updated_at_utc,
        last_observed_at_utc: row.last_observed_at_utc,
    };
    crate::model_runtime::validate_artifact_locator(
        persisted.artifact_sha256,
        &persisted.artifact_locator,
    )?;
    let expected_fingerprint = selection_fingerprint(scope, &persisted.selection())?;
    if expected_fingerprint != row.current_selection_fingerprint {
        return Err(ModelRegistryPersistenceError::CorruptRow(
            "registry current selection fingerprint does not match exact scope and selection"
                .to_owned(),
        ));
    }
    require_active_registration(&persisted)?;
    Ok(persisted)
}

fn decode_active_selection(
    row: StoredActiveSelection,
) -> Result<PersistedActiveModelSelection, ModelRegistryPersistenceError> {
    let purpose = match row.purpose.as_str() {
        "application/default" => ModelRuntimeSelectionPurpose::ApplicationDefault,
        "embeddings/default" => ModelRuntimeSelectionPurpose::EmbeddingsDefault,
        other => {
            return Err(ModelRegistryPersistenceError::CorruptRow(format!(
                "unknown active selection purpose `{other}`"
            )))
        }
    };
    let runtime_role = parse_runtime_role(&row.runtime_role)?;
    if runtime_role != purpose.runtime_role() {
        return Err(ModelRegistryPersistenceError::CorruptRow(format!(
            "active purpose {} carries role {}",
            purpose.as_str(),
            runtime_role.as_str()
        )));
    }
    let artifact_bytes = hex::decode(&row.artifact_sha256).map_err(|_| {
        ModelRegistryPersistenceError::CorruptRow(
            "active selection artifact SHA-256 is not hexadecimal".to_owned(),
        )
    })?;
    let artifact_sha256: [u8; 32] = artifact_bytes.try_into().map_err(|bytes: Vec<u8>| {
        ModelRegistryPersistenceError::CorruptRow(format!(
            "active selection artifact SHA-256 is {} bytes",
            bytes.len()
        ))
    })?;
    let selection_revision = u64::try_from(row.selection_revision).map_err(|_| {
        ModelRegistryPersistenceError::CorruptRow(
            "active selection revision is not positive".to_owned(),
        )
    })?;
    if selection_revision == 0
        || row.selection_created_event_id.trim().is_empty()
        || row.selection_updated_event_id.trim().is_empty()
        || row.latest_mutation_fingerprint.len() != 64
        || row
            .last_request_fingerprint
            .as_ref()
            .is_some_and(|fingerprint| fingerprint.len() != 64)
    {
        return Err(ModelRegistryPersistenceError::CorruptRow(
            "active selection revision, fingerprint, or receipt identity is invalid".to_owned(),
        ));
    }
    let lifecycle_state = parse_lifecycle(&row.lifecycle_state)?;
    if lifecycle_state != ModelRegistryLifecycleState::Active {
        return Err(ModelRegistryPersistenceError::SelectionInactive {
            artifact_sha256: row.artifact_sha256,
            state: lifecycle_state.as_str().to_owned(),
        });
    }
    Ok(PersistedActiveModelSelection {
        purpose,
        runtime_role,
        artifact_sha256,
        lifecycle_state,
        selection_revision,
        latest_mutation_fingerprint: row.latest_mutation_fingerprint,
        last_request_fingerprint: row.last_request_fingerprint,
        selection_created_event_id: row.selection_created_event_id,
        selection_updated_event_id: row.selection_updated_event_id,
        selection_created_at_utc: row.selection_created_at_utc,
        selection_updated_at_utc: row.selection_updated_at_utc,
    })
}

fn parse_lifecycle(
    value: &str,
) -> Result<ModelRegistryLifecycleState, ModelRegistryPersistenceError> {
    match value {
        "active" => Ok(ModelRegistryLifecycleState::Active),
        "stale" => Ok(ModelRegistryLifecycleState::Stale),
        "revoked" => Ok(ModelRegistryLifecycleState::Revoked),
        other => Err(ModelRegistryPersistenceError::CorruptRow(format!(
            "unknown model registry lifecycle `{other}`"
        ))),
    }
}

fn parse_uuid(value: &str, field: &str) -> Result<uuid::Uuid, ModelRegistryPersistenceError> {
    uuid::Uuid::parse_str(value).map_err(|_| {
        ModelRegistryPersistenceError::CorruptRow(format!("model registry {field} is not a UUID"))
    })
}

fn validate_operator_mutation(
    actor: &KernelActor,
    reason: &str,
    expected_revision: u64,
) -> Result<(), ModelRegistryPersistenceError> {
    if !matches!(actor, KernelActor::Operator(_))
        || actor.actor_id().trim().is_empty()
        || reason.trim().is_empty()
        || actor.actor_id().len() > MODEL_REGISTRY_INPUT_TEXT_BYTE_CAP
        || reason.len() > MODEL_REGISTRY_INPUT_TEXT_BYTE_CAP
        || expected_revision == 0
    {
        return Err(ModelRegistryPersistenceError::InvalidRebind(
            "active selection requires an operator actor, reason, and nonzero revision".to_owned(),
        ));
    }
    Ok(())
}

fn one<T>(rows: Vec<T>, context: &str) -> Result<T, ModelRegistryPersistenceError> {
    if rows.len() != 1 {
        return Err(ModelRegistryPersistenceError::AuthorityUnavailable(
            format!("{context} returned {} rows, expected one", rows.len()),
        ));
    }
    Ok(rows.into_iter().next().expect("length checked"))
}

#[cfg(feature = "test-utils")]
fn require_single_test_mutation(
    rows: Vec<surrealdb::types::Value>,
    mutation: &str,
) -> Result<(), ModelRegistryPersistenceError> {
    if rows.len() == 1 {
        Ok(())
    } else {
        Err(ModelRegistryPersistenceError::AuthorityUnavailable(
            format!("{mutation} test seam affected {} rows", rows.len()),
        ))
    }
}

fn storage_error(error: SurrealStorageError) -> ModelRegistryPersistenceError {
    ModelRegistryPersistenceError::Storage(error.to_string())
}
