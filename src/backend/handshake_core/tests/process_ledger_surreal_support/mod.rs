#[path = "../surreal_test_store_support/mod.rs"]
mod surreal_test_store_support;

use std::sync::Arc;

use chrono::{DateTime, Utc};
use handshake_core::{
    process_ledger::{
        ProcessLedgerError, ProcessLedgerStore, ReclaimResourceScope, SurrealProcessLedgerStore,
    },
    storage::surreal::{bootstrap_schema, SurrealStorage},
    swarm_orchestration::resource_scope::{
        AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, OwnerAccountId, ResourceScope,
        WorkspaceScopeRef,
    },
};
use serde_json::{json, Value};
use surreal_test_store_support::EmbeddedSurrealTestScope;
use surrealdb::types::{RecordId, SurrealValue};
use uuid::Uuid;

pub struct ProcessLedgerSurrealHarness {
    allocator: EmbeddedSurrealTestScope,
    storage: SurrealStorage,
    store: Arc<SurrealProcessLedgerStore>,
    scope: ReclaimResourceScope,
}

impl ProcessLedgerSurrealHarness {
    pub async fn open() -> Self {
        Self::open_with_scope(exact_scope()).await
    }

    pub async fn open_with_scope(scope: ReclaimResourceScope) -> Self {
        let mut allocator = EmbeddedSurrealTestScope::create()
            .await
            .expect("allocate isolated ProcessLedger Surreal scope");
        let storage = allocator
            .activate_storage()
            .await
            .expect("activate injected ProcessLedger SurrealStorage");
        bootstrap_schema(&storage)
            .await
            .expect("bootstrap shared Surreal schema");
        let store = Arc::new(
            SurrealProcessLedgerStore::open(storage.clone())
                .await
                .expect("open ProcessLedger on injected SurrealStorage"),
        );
        Self {
            allocator,
            storage,
            store,
            scope,
        }
    }

    pub fn storage(&self) -> SurrealStorage {
        self.storage.clone()
    }

    pub fn store(&self) -> Arc<SurrealProcessLedgerStore> {
        self.store.clone()
    }

    pub fn resource_scope(&self) -> &ReclaimResourceScope {
        &self.scope
    }

    pub fn model_resource_scope(&self) -> ResourceScope {
        ResourceScope::new(
            OwnerAccountId::from_uuid(self.scope.account_uuid),
            ActorPrincipalId::from_uuid(self.scope.actor_uuid),
        )
        .with_session(AuthenticatedSessionRef::from_uuid(self.scope.session_uuid))
        .with_access_space(AccessSpaceRef::from_uuid(self.scope.access_space_uuid))
        .with_workspace(
            WorkspaceScopeRef::new(self.scope.workspace_id.clone())
                .expect("nonblank ProcessLedger test workspace"),
        )
    }

    pub fn metadata(&self) -> Value {
        scope_metadata(&self.scope)
    }

    pub async fn write_batch(
        &self,
        events: Vec<handshake_core::process_ledger::LedgerEvent>,
    ) -> Result<(), ProcessLedgerError> {
        self.store.write_batch(events).await
    }

    pub async fn lifecycle(&self, process_uuid: Uuid) -> Option<LifecycleProbe> {
        let bindings = ExactRecordBindings::new(
            &self.scope,
            RecordId::new("kernel_process_lifecycle", process_uuid.to_string()),
        );
        self.storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_first::<LifecycleProbe, _>(READ_EXACT_LIFECYCLE, bindings)
                        .await
                })
            })
            .await
            .expect("read exact-scope lifecycle")
    }

    pub async fn lifecycle_count(&self) -> i64 {
        self.exact_count(COUNT_EXACT_LIFECYCLES).await
    }

    pub async fn lifecycles(&self) -> Vec<LifecycleProbe> {
        let bindings = ExactScopeBindings::new(&self.scope);
        self.storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values::<LifecycleProbe, _>(READ_EXACT_LIFECYCLES, bindings)
                        .await
                })
            })
            .await
            .expect("read exact-scope lifecycles")
    }

    pub async fn open_lifecycle_count(&self) -> i64 {
        self.exact_count(COUNT_EXACT_OPEN_LIFECYCLES).await
    }

    pub async fn process_event_count(&self) -> i64 {
        self.exact_count(COUNT_EXACT_PROCESS_EVENTS).await
    }

    pub async fn lifecycles_for_artifact(&self, artifact_sha256: &str) -> Vec<LifecycleProbe> {
        let bindings = ExactArtifactBindings::new(&self.scope, artifact_sha256);
        self.storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values::<LifecycleProbe, _>(READ_EXACT_ARTIFACT_LIFECYCLES, bindings)
                        .await
                })
            })
            .await
            .expect("read exact-scope artifact lifecycles")
    }

    async fn exact_count(&self, statement: &'static str) -> i64 {
        let bindings = ExactScopeBindings::new(&self.scope);
        self.storage
            .with_data_operation(|database| {
                Box::pin(async move { database.query_first::<i64, _>(statement, bindings).await })
            })
            .await
            .expect("count exact-scope ProcessLedger rows")
            .expect("exact-scope count query returns one value")
    }

    pub async fn close(mut self) {
        drop(self.store);
        drop(self.storage);
        self.allocator
            .shutdown_storage_for_reopen()
            .await
            .expect("shutdown injected ProcessLedger SurrealStorage");
        let diagnostics = self
            .allocator
            .cleanup()
            .await
            .expect("clean isolated ProcessLedger Surreal scope");
        assert!(diagnostics.database_absent);
        assert!(diagnostics.namespace_absent_after_reopen);
        assert!(diagnostics.error.is_none());
    }
}

pub fn exact_scope() -> ReclaimResourceScope {
    ReclaimResourceScope {
        account_uuid: Uuid::now_v7(),
        actor_uuid: Uuid::now_v7(),
        session_uuid: Uuid::now_v7(),
        workspace_id: format!("workspace-{}", Uuid::now_v7()),
        access_space_uuid: Uuid::now_v7(),
    }
}

pub fn scope_metadata(scope: &ReclaimResourceScope) -> Value {
    json!({
        "owner_account_id": scope.account_uuid.to_string(),
        "actor_principal_id": scope.actor_uuid.to_string(),
        "authenticated_session_id": scope.session_uuid.to_string(),
        "access_space_id": scope.access_space_uuid.to_string(),
        "workspace_id": scope.workspace_id.clone(),
    })
}

#[derive(Clone, Debug, SurrealValue)]
pub struct LifecycleProbe {
    pub process_uuid: Uuid,
    pub os_pid: Option<i64>,
    pub engine_kind: String,
    pub started_at: DateTime<Utc>,
    pub stopped_at: Option<DateTime<Utc>>,
    pub exit_code: Option<i64>,
    pub stop_reason: Option<String>,
    pub model_artifact_sha256: Option<String>,
    pub owner_role: String,
    pub owner_runtime_instance_id: Option<Uuid>,
    pub owner_host_scope_id: Option<String>,
    pub owner_lease_schema_id: Option<String>,
    pub owner_lease_protocol: Option<String>,
    pub owner_lease_address: Option<String>,
    pub owner_lease_port: Option<i64>,
    pub event_ledger_event_id: Option<RecordId>,
    pub metadata: Value,
}

#[derive(Debug, SurrealValue)]
struct ExactScopeBindings {
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

impl ExactScopeBindings {
    fn new(scope: &ReclaimResourceScope) -> Self {
        Self {
            owner_account_id: scope.account_uuid.to_string(),
            actor_principal_id: scope.actor_uuid.to_string(),
            authenticated_session_id: scope.session_uuid.to_string(),
            access_space_id: scope.access_space_uuid.to_string(),
            workspace_id: scope.workspace_id.clone(),
        }
    }
}

#[derive(Debug, SurrealValue)]
struct ExactRecordBindings {
    record: RecordId,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

impl ExactRecordBindings {
    fn new(scope: &ReclaimResourceScope, record: RecordId) -> Self {
        let scope = ExactScopeBindings::new(scope);
        Self {
            record,
            owner_account_id: scope.owner_account_id,
            actor_principal_id: scope.actor_principal_id,
            authenticated_session_id: scope.authenticated_session_id,
            access_space_id: scope.access_space_id,
            workspace_id: scope.workspace_id,
        }
    }
}

#[derive(Debug, SurrealValue)]
struct ExactArtifactBindings {
    artifact_sha256: String,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

impl ExactArtifactBindings {
    fn new(scope: &ReclaimResourceScope, artifact_sha256: &str) -> Self {
        let scope = ExactScopeBindings::new(scope);
        Self {
            artifact_sha256: artifact_sha256.to_string(),
            owner_account_id: scope.owner_account_id,
            actor_principal_id: scope.actor_principal_id,
            authenticated_session_id: scope.authenticated_session_id,
            access_space_id: scope.access_space_id,
            workspace_id: scope.workspace_id,
        }
    }
}

const READ_EXACT_LIFECYCLE: &str = r#"
SELECT process_uuid, os_pid, engine_kind, started_at, stopped_at, exit_code,
    stop_reason, model_artifact_sha256, owner_role, owner_runtime_instance_id,
    owner_host_scope_id, owner_lease_schema_id, owner_lease_protocol,
    owner_lease_address, owner_lease_port, event_ledger_event_id, metadata
FROM ONLY $record
WHERE owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id;
"#;

const READ_EXACT_ARTIFACT_LIFECYCLES: &str = r#"
SELECT process_uuid, os_pid, engine_kind, started_at, stopped_at, exit_code,
    stop_reason, model_artifact_sha256, owner_role, owner_runtime_instance_id,
    owner_host_scope_id, owner_lease_schema_id, owner_lease_protocol,
    owner_lease_address, owner_lease_port, event_ledger_event_id, metadata
FROM kernel_process_lifecycle
WHERE model_artifact_sha256 = $artifact_sha256
    AND owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id;
"#;

const READ_EXACT_LIFECYCLES: &str = r#"
SELECT process_uuid, os_pid, engine_kind, started_at, stopped_at, exit_code,
    stop_reason, model_artifact_sha256, owner_role, owner_runtime_instance_id,
    owner_host_scope_id, owner_lease_schema_id, owner_lease_protocol,
    owner_lease_address, owner_lease_port, event_ledger_event_id, metadata
FROM kernel_process_lifecycle
WHERE owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id;
"#;

const COUNT_EXACT_LIFECYCLES: &str = r#"
RETURN array::len(SELECT VALUE id FROM kernel_process_lifecycle
WHERE owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id);
"#;

const COUNT_EXACT_OPEN_LIFECYCLES: &str = r#"
RETURN array::len(SELECT VALUE id FROM kernel_process_lifecycle
WHERE stopped_at = NONE
    AND owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id);
"#;

const COUNT_EXACT_PROCESS_EVENTS: &str = r#"
RETURN array::len(SELECT VALUE id FROM kernel_event_ledger
WHERE source_component = 'process_ledger'
    AND payload.metadata_jsonb.owner_account_id = $owner_account_id
    AND payload.metadata_jsonb.actor_principal_id = $actor_principal_id
    AND payload.metadata_jsonb.authenticated_session_id = $authenticated_session_id
    AND payload.metadata_jsonb.access_space_id = $access_space_id
    AND payload.metadata_jsonb.workspace_id = $workspace_id);
"#;
