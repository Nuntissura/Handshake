use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use sha2::{Digest, Sha256};
use surrealdb::types::{RecordId, SurrealValue};
use uuid::Uuid;

use crate::process_ledger::reclaim::{
    default_dead_owner_confirmation_gap, runtime_owner_loopback_lease_is_free,
    ReclaimResourceScope, StaleSessionProcessSet,
};
use crate::process_ledger::{
    EmbeddedRuntimeInstanceDescriptor, LedgerEvent, LedgerEventKind, ProcessEngineKind,
    ProcessLedgerError, ProcessLedgerStore, ProcessRuntimeOwner, ProcessStop, ReclaimClaim,
    ReclaimKillOperation, ReclaimKillOperationCandidate, ReclaimKillOperationStatus,
    ReclaimProcessStore, ReclaimableProcess, StaleSessionSource,
    EMBEDDED_RUNTIME_INSTANCE_SCHEMA_ID, EMBEDDED_RUNTIME_LOOPBACK_UDP_PROTOCOL,
    PROCESS_LEDGER_TABLE_NAME,
};

use super::{SurrealStorage, SurrealStorageError};

const SCHEMA: &str = include_str!("process_ledger_schema.surql");

#[derive(Clone)]
pub struct SurrealProcessLedgerStore {
    storage: SurrealStorage,
}

/// Read-only, exact-scope projection of one durable ProcessLedger lifecycle.
///
/// The canonical EventLedger link is verified before this projection is
/// returned. Both open and stopped rows are inspectable so registry recovery
/// does not lose the ownership evidence for an unloaded artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessOwnershipInspection {
    pub process_uuid: Uuid,
    pub os_pid: Option<u32>,
    pub model_artifact_sha256: Option<String>,
    pub engine_kind: ProcessEngineKind,
    pub owner_role: String,
    pub owner_wp: Option<String>,
    pub sandbox_adapter_id: Option<String>,
    pub lifecycle_state: LedgerEventKind,
    pub started_at: DateTime<Utc>,
    pub stopped_at: Option<DateTime<Utc>>,
    pub exit_code: Option<i64>,
    pub stop_reason: Option<String>,
    pub runtime_owner: Option<ProcessRuntimeOwner>,
    pub resource_scope: ReclaimResourceScope,
    pub event_ledger_event_id: RecordId,
}

#[derive(Clone)]
pub struct SurrealModelLaneStaleSessionSource {
    storage: SurrealStorage,
    runtime_instance: EmbeddedRuntimeInstanceDescriptor,
    dead_owner_confirmation_gap: Duration,
    dead_owner_first_observed_free: Arc<Mutex<HashMap<ProcessRuntimeOwner, Instant>>>,
}

impl SurrealModelLaneStaleSessionSource {
    pub fn new(
        storage: SurrealStorage,
        runtime_instance: EmbeddedRuntimeInstanceDescriptor,
    ) -> Self {
        Self {
            storage,
            runtime_instance,
            dead_owner_confirmation_gap: default_dead_owner_confirmation_gap(),
            dead_owner_first_observed_free: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn with_dead_owner_confirmation_gap(mut self, gap: Duration) -> Self {
        self.dead_owner_confirmation_gap = gap;
        self
    }

    fn owner_is_confirmed_dead(&self, owner: &ProcessRuntimeOwner) -> bool {
        let observed_free = runtime_owner_loopback_lease_is_free(owner);
        let mut observations = self
            .dead_owner_first_observed_free
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if !observed_free {
            observations.remove(owner);
            return false;
        }
        if self.dead_owner_confirmation_gap.is_zero() {
            return true;
        }
        let now = Instant::now();
        match observations.get(owner) {
            Some(first_observed)
                if now.duration_since(*first_observed) >= self.dead_owner_confirmation_gap =>
            {
                true
            }
            Some(_) => false,
            None => {
                observations.insert(owner.clone(), now);
                tracing::info!(
                    target: "handshake::process_ledger::reclaim",
                    runtime_instance_id = %owner.runtime_instance_id,
                    lease_port = owner.lease_port,
                    confirmation_gap_ms = self.dead_owner_confirmation_gap.as_millis(),
                    "prior runtime-owner loopback lease observed free for the first time; restart reclaim is withheld until a second corroborating observation"
                );
                false
            }
        }
    }

    async fn restart_lifecycle_rows(
        &self,
        complete_scope: bool,
    ) -> Result<Vec<RestartLifecycleRow>, ProcessLedgerError> {
        self.storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values::<RestartLifecycleRow, _>(
                            RESTART_LIFECYCLE_ROWS,
                            RestartLifecycleBindings { complete_scope },
                        )
                        .await
                })
            })
            .await
            .map_err(surreal_store_error)
    }

    async fn stale_lifecycle_rows(
        &self,
        complete_scope: bool,
    ) -> Result<Vec<StaleLifecycleRow>, ProcessLedgerError> {
        let bindings = StaleLifecycleBindings { complete_scope };
        self.storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values::<StaleLifecycleRow, _>(STALE_LIFECYCLE_ROWS, bindings)
                        .await
                })
            })
            .await
            .map_err(surreal_store_error)
    }

    async fn model_lane_authority_rows(
        &self,
        scope: &ExactResourceScope,
    ) -> Result<Vec<ModelLaneAuthorityRow>, ProcessLedgerError> {
        let bindings = ModelLaneAuthorityBindings {
            owner_account_id: scope.owner_account_id.clone(),
            actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(),
            access_space_id: scope.access_space_id.clone(),
            workspace_id: scope.workspace_id.clone(),
        };
        self.storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values::<ModelLaneAuthorityRow, _>(
                            MODEL_LANE_AUTHORITY_ROWS_FOR_SCOPE,
                            bindings,
                        )
                        .await
                })
            })
            .await
            .map_err(surreal_store_error)
    }

    async fn restart_process_sets(
        &self,
    ) -> Result<Vec<StaleSessionProcessSet>, ProcessLedgerError> {
        let complete_rows = self.restart_lifecycle_rows(true).await?;
        let incomplete_rows = self.restart_lifecycle_rows(false).await?;
        let mut parsed_rows = Vec::with_capacity(complete_rows.len() + incomplete_rows.len());
        let mut canonical_descriptors = HashMap::<Uuid, ProcessRuntimeOwner>::new();
        let mut conflicting_instance_ids = BTreeSet::<Uuid>::new();

        for (row, complete_scope) in complete_rows
            .into_iter()
            .map(|row| (row, true))
            .chain(incomplete_rows.into_iter().map(|row| (row, false)))
        {
            let scope = restart_lifecycle_scope(&row)?;
            if complete_scope && scope.is_none() {
                return Err(ProcessLedgerError::Store(
                    "complete-scope Surreal restart row has no ResourceScope".to_owned(),
                ));
            }
            let owner = runtime_owner_from_restart_row(&row)?;
            if let Some(owner) = &owner {
                match canonical_descriptors.get(&owner.runtime_instance_id) {
                    Some(canonical) if canonical != owner => {
                        conflicting_instance_ids.insert(owner.runtime_instance_id);
                    }
                    None => {
                        canonical_descriptors.insert(owner.runtime_instance_id, owner.clone());
                    }
                    Some(_) => {}
                }
            }
            if row.parent_session_id.is_some() && row.sandbox_adapter_id.is_some() {
                parsed_rows.push((
                    row.parent_session_id.expect("candidate checked"),
                    row.process_uuid,
                    scope,
                    owner,
                    complete_scope,
                ));
            }
        }

        let mut veto_sessions = BTreeSet::<String>::new();
        let mut session_scopes = BTreeMap::<String, BTreeSet<ExactResourceScope>>::new();
        let mut session_safe =
            BTreeMap::<(ExactResourceScope, String), (bool, BTreeSet<Uuid>)>::new();
        let mut descriptor_state = HashMap::<ProcessRuntimeOwner, bool>::new();
        for (session_id, process_uuid, scope, owner, complete_scope) in parsed_rows {
            let Some(scope) = scope.filter(|_| complete_scope) else {
                veto_sessions.insert(session_id);
                continue;
            };
            session_scopes
                .entry(session_id.clone())
                .or_default()
                .insert(scope.clone());
            let safely_dead = match owner {
                Some(owner) if conflicting_instance_ids.contains(&owner.runtime_instance_id) => {
                    tracing::error!(
                        runtime_instance_id = %owner.runtime_instance_id,
                        session_id,
                        "conflicting typed runtime-owner descriptors veto Surreal restart reclaim"
                    );
                    false
                }
                Some(owner)
                    if owner.host_scope_id == self.runtime_instance.host_scope_id
                        && owner.runtime_instance_id != self.runtime_instance.instance_id
                        && owner.lease_schema_id == EMBEDDED_RUNTIME_INSTANCE_SCHEMA_ID
                        && owner.lease_protocol == EMBEDDED_RUNTIME_LOOPBACK_UDP_PROTOCOL =>
                {
                    if let Some(dead) = descriptor_state.get(&owner) {
                        *dead
                    } else {
                        let dead = self.owner_is_confirmed_dead(&owner);
                        descriptor_state.insert(owner, dead);
                        dead
                    }
                }
                _ => false,
            };
            session_safe
                .entry((scope, session_id))
                .and_modify(|(safe, process_uuids)| {
                    *safe &= safely_dead;
                    process_uuids.insert(process_uuid);
                })
                .or_insert_with(|| (safely_dead, BTreeSet::from([process_uuid])));
        }

        let mut candidates = Vec::new();
        for ((scope, session_id), (safe, process_uuids)) in session_safe {
            let single_scope = session_scopes
                .get(&session_id)
                .is_some_and(|scopes| scopes.len() == 1);
            if safe && single_scope && !veto_sessions.contains(&session_id) {
                candidates.push(StaleSessionProcessSet {
                    resource_scope: reclaim_resource_scope(&scope)?,
                    session_id,
                    authorized_process_uuids: process_uuids.into_iter().collect(),
                });
            }
        }
        Ok(candidates)
    }

    async fn restart_session_ids(&self) -> Result<Vec<String>, ProcessLedgerError> {
        Ok(self
            .restart_process_sets()
            .await?
            .into_iter()
            .map(|candidate| candidate.session_id)
            .collect())
    }

    async fn stale_process_sets(
        &self,
        ttl: Duration,
    ) -> Result<Vec<StaleSessionProcessSet>, ProcessLedgerError> {
        let incomplete_rows = self.stale_lifecycle_rows(false).await?;
        let mut veto_sessions = BTreeSet::<String>::new();
        for row in incomplete_rows {
            if let Some(session_id) = row.parent_session_id {
                veto_sessions.insert(session_id);
            }
        }

        let complete_rows = self.stale_lifecycle_rows(true).await?;
        let mut rows_by_scope = BTreeMap::<ExactResourceScope, Vec<(String, Uuid, bool)>>::new();
        for row in complete_rows {
            let Some(session_id) = row.parent_session_id.clone() else {
                continue;
            };
            let scope = stale_lifecycle_scope(&row)?.ok_or_else(|| {
                ProcessLedgerError::Store(
                    "complete-scope Surreal stale-session row has no ResourceScope".to_owned(),
                )
            })?;
            let owned_by_self = row.owner_runtime_instance_id
                == Some(self.runtime_instance.instance_id)
                && row.owner_host_scope_id.as_deref()
                    == Some(self.runtime_instance.host_scope_id.as_str());
            rows_by_scope.entry(scope).or_default().push((
                session_id,
                row.process_uuid,
                owned_by_self,
            ));
        }

        let now = Utc::now();
        let ttl = chrono::Duration::from_std(ttl).map_err(|error| {
            ProcessLedgerError::InvalidConfig(format!("invalid stale-session TTL: {error}"))
        })?;
        let mut session_scopes = BTreeMap::<String, BTreeSet<ExactResourceScope>>::new();
        let mut session_reclaimable =
            BTreeMap::<(ExactResourceScope, String), (bool, BTreeSet<Uuid>)>::new();

        for (scope, lifecycle_rows) in rows_by_scope {
            let authority_rows = self.model_lane_authority_rows(&scope).await?;
            let mut lane_records = Vec::with_capacity(authority_rows.len());
            for authority_row in authority_rows {
                if model_lane_authority_scope(&authority_row)? != scope {
                    return Err(ProcessLedgerError::Store(
                        "Surreal model-lane authority escaped its bound ResourceScope".to_owned(),
                    ));
                }
                let record =
                    serde_json::from_str::<Value>(&authority_row.record_json).map_err(|error| {
                        ProcessLedgerError::Store(format!(
                            "model lane stale-session record is invalid JSON: {error}"
                        ))
                    })?;
                lane_records.push(record);
            }

            for (session_id, process_uuid, owned_by_self) in lifecycle_rows {
                session_scopes
                    .entry(session_id.clone())
                    .or_default()
                    .insert(scope.clone());
                let mut exact_matches = 0usize;
                let mut row_reclaimable = owned_by_self;
                for record in &lane_records {
                    if let Some(reclaimable) = exact_model_lane_reclaimability(
                        record,
                        &session_id,
                        process_uuid,
                        now,
                        ttl,
                    )? {
                        exact_matches += 1;
                        row_reclaimable &= reclaimable;
                    }
                }
                row_reclaimable &= exact_matches == 1;
                session_reclaimable
                    .entry((scope.clone(), session_id))
                    .and_modify(|(all_reclaimable, process_uuids)| {
                        *all_reclaimable &= row_reclaimable;
                        process_uuids.insert(process_uuid);
                    })
                    .or_insert_with(|| (row_reclaimable, BTreeSet::from([process_uuid])));
            }
        }

        let mut candidates = Vec::new();
        for ((scope, session_id), (all_reclaimable, process_uuids)) in session_reclaimable {
            let single_scope = session_scopes
                .get(&session_id)
                .is_some_and(|scopes| scopes.len() == 1);
            if all_reclaimable && single_scope && !veto_sessions.contains(&session_id) {
                candidates.push(StaleSessionProcessSet {
                    resource_scope: reclaim_resource_scope(&scope)?,
                    session_id,
                    authorized_process_uuids: process_uuids.into_iter().collect(),
                });
            }
        }
        Ok(candidates)
    }
}

#[async_trait]
impl StaleSessionSource for SurrealModelLaneStaleSessionSource {
    fn self_runtime_instance_id(&self) -> Option<Uuid> {
        Some(self.runtime_instance.instance_id)
    }

    fn self_runtime_owner_scope(&self) -> Option<(Uuid, String)> {
        Some((
            self.runtime_instance.instance_id,
            self.runtime_instance.host_scope_id.clone(),
        ))
    }

    async fn restart_sessions(&self) -> Result<Vec<String>, ProcessLedgerError> {
        self.restart_session_ids().await
    }

    async fn restart_session_process_sets(
        &self,
    ) -> Result<Vec<StaleSessionProcessSet>, ProcessLedgerError> {
        self.restart_process_sets().await
    }

    async fn stale_sessions(&self, ttl: Duration) -> Result<Vec<String>, ProcessLedgerError> {
        Ok(self
            .stale_process_sets(ttl)
            .await?
            .into_iter()
            .map(|candidate| candidate.session_id)
            .collect())
    }

    async fn stale_session_process_sets(
        &self,
        ttl: Duration,
    ) -> Result<Vec<StaleSessionProcessSet>, ProcessLedgerError> {
        self.stale_process_sets(ttl).await
    }
}

impl SurrealProcessLedgerStore {
    pub fn new(storage: SurrealStorage) -> Self {
        Self { storage }
    }

    pub async fn open(storage: SurrealStorage) -> Result<Self, ProcessLedgerError> {
        let store = Self::new(storage);
        store.preflight().await?;
        Ok(store)
    }

    pub fn storage(&self) -> &SurrealStorage {
        &self.storage
    }

    pub async fn inspect_ownership_by_process_uuid(
        &self,
        resource_scope: &ReclaimResourceScope,
        process_uuid: Uuid,
    ) -> Result<Option<ProcessOwnershipInspection>, ProcessLedgerError> {
        validate_inspection_scope(resource_scope)?;
        let canonical_record = RecordId::new(PROCESS_LEDGER_TABLE_NAME, process_uuid.to_string());
        let canonical = self
            .storage
            .with_data_operation(|database| {
                let bindings =
                    InspectionProcessBindings::new(resource_scope, process_uuid, canonical_record);
                Box::pin(async move {
                    database
                        .query_first::<InspectionLifecycleRow, _>(
                            INSPECT_CANONICAL_PROCESS_RECORD,
                            bindings,
                        )
                        .await
                })
            })
            .await
            .map_err(surreal_store_error)?;
        let exact_rows = self
            .storage
            .with_data_operation(|database| {
                let bindings = InspectionProcessBindings::new(
                    resource_scope,
                    process_uuid,
                    RecordId::new(PROCESS_LEDGER_TABLE_NAME, process_uuid.to_string()),
                );
                Box::pin(async move {
                    database
                        .query_values::<InspectionLifecycleRow, _>(
                            INSPECT_EXACT_PROCESS_ROWS,
                            bindings,
                        )
                        .await
                })
            })
            .await
            .map_err(surreal_store_error)?;

        if exact_rows.len() > 1 {
            return Err(inspection_error("duplicate canonical process identity"));
        }
        let Some(canonical) = canonical else {
            return if exact_rows.is_empty() {
                Ok(None)
            } else {
                Err(inspection_error("noncanonical process record identity"))
            };
        };
        if canonical.process_uuid != process_uuid {
            return Err(inspection_error("canonical process identity mismatch"));
        }
        let stored_scope = inspection_scope(&canonical)?;
        if canonical.exact_scope_match != (&stored_scope == resource_scope) {
            return Err(inspection_error("scope predicate verification mismatch"));
        }
        if &stored_scope != resource_scope {
            return if exact_rows.is_empty() {
                Ok(None)
            } else {
                Err(inspection_error("duplicate cross-scope process identity"))
            };
        }
        if exact_rows.len() != 1 || exact_rows[0].id != canonical.id {
            return Err(inspection_error("duplicate canonical process identity"));
        }
        self.verify_and_project_inspection(resource_scope, canonical)
            .await
            .map(Some)
    }

    pub async fn inspect_latest_ownership_by_artifact(
        &self,
        resource_scope: &ReclaimResourceScope,
        model_artifact_sha256: &str,
    ) -> Result<Option<ProcessOwnershipInspection>, ProcessLedgerError> {
        validate_inspection_scope(resource_scope)?;
        let model_artifact_sha256 = model_artifact_sha256.trim();
        if model_artifact_sha256.is_empty() {
            return Err(ProcessLedgerError::InvalidConfig(
                "ProcessLedger inspection artifact hash is missing".to_owned(),
            ));
        }
        let rows = self
            .storage
            .with_data_operation(|database| {
                let bindings =
                    InspectionArtifactBindings::new(resource_scope, model_artifact_sha256);
                Box::pin(async move {
                    database
                        .query_values::<InspectionLifecycleRow, _>(
                            INSPECT_EXACT_ARTIFACT_ROWS,
                            bindings,
                        )
                        .await
                })
            })
            .await
            .map_err(surreal_store_error)?;
        if rows.is_empty() {
            return Ok(None);
        }

        let mut identities = HashSet::with_capacity(rows.len());
        for row in &rows {
            if !row.exact_scope_match || inspection_scope(row)? != *resource_scope {
                return Err(inspection_error("artifact ownership escaped exact scope"));
            }
            if !identities.insert(row.process_uuid) {
                return Err(inspection_error("duplicate artifact process identity"));
            }
            if row.id != RecordId::new(PROCESS_LEDGER_TABLE_NAME, row.process_uuid.to_string()) {
                return Err(inspection_error("noncanonical artifact process identity"));
            }
        }
        if rows
            .get(1)
            .is_some_and(|next| next.started_at == rows[0].started_at)
        {
            return Err(inspection_error("ambiguous latest artifact ownership"));
        }
        self.verify_and_project_inspection(resource_scope, rows.into_iter().next().unwrap())
            .await
            .map(Some)
    }

    async fn verify_and_project_inspection(
        &self,
        resource_scope: &ReclaimResourceScope,
        row: InspectionLifecycleRow,
    ) -> Result<ProcessOwnershipInspection, ProcessLedgerError> {
        let event_record = row
            .event_ledger_event_id
            .clone()
            .ok_or_else(|| inspection_error("missing canonical EventLedger receipt"))?;
        let expected_kind = if row.stopped_at.is_some() {
            LedgerEventKind::Stop
        } else {
            LedgerEventKind::Start
        };
        let expected_event_id = format!(
            "process-lifecycle-{}-{}",
            row.process_uuid,
            expected_kind.as_str().to_ascii_lowercase()
        );
        let expected_record = RecordId::new("kernel_event_ledger", expected_event_id.clone());
        if event_record != expected_record {
            return Err(inspection_error("canonical EventLedger linkage mismatch"));
        }
        let receipt = self
            .storage
            .with_data_operation(|database| {
                let bindings = InspectionReceiptBindings::new(resource_scope, event_record.clone());
                Box::pin(async move {
                    database
                        .query_first::<InspectionEventRow, _>(INSPECT_EXACT_RECEIPT, bindings)
                        .await
                })
            })
            .await
            .map_err(surreal_store_error)?
            .ok_or_else(|| inspection_error("missing or foreign canonical EventLedger receipt"))?;
        verify_inspection_receipt(&row, &receipt, expected_kind, &expected_event_id)?;
        inspection_from_row(row, event_record)
    }

    #[cfg(any(test, feature = "test-utils"))]
    pub async fn test_set_inspection_event_link(
        &self,
        resource_scope: &ReclaimResourceScope,
        process_uuid: Uuid,
        event_ledger_event_id: Option<RecordId>,
    ) -> Result<(), ProcessLedgerError> {
        validate_inspection_scope(resource_scope)?;
        let affected = self
            .storage
            .with_data_operation(|database| {
                let bindings = InspectionLinkTamperBindings::new(
                    resource_scope,
                    process_uuid,
                    event_ledger_event_id,
                );
                Box::pin(async move {
                    database
                        .execute_returning(TEST_SET_INSPECTION_EVENT_LINK, bindings)
                        .await
                })
            })
            .await
            .map_err(surreal_store_error)?;
        if affected != 1 {
            return Err(inspection_error(
                "test event-link tamper escaped exact scope",
            ));
        }
        Ok(())
    }

    #[cfg(any(test, feature = "test-utils"))]
    pub async fn test_clear_inspection_scope_field(
        &self,
        resource_scope: &ReclaimResourceScope,
        process_uuid: Uuid,
        field: &str,
    ) -> Result<(), ProcessLedgerError> {
        validate_inspection_scope(resource_scope)?;
        let statement = match field {
            "owner_account_id" => TEST_CLEAR_OWNER_ACCOUNT_ID,
            "actor_principal_id" => TEST_CLEAR_ACTOR_PRINCIPAL_ID,
            "authenticated_session_id" => TEST_CLEAR_AUTHENTICATED_SESSION_ID,
            "access_space_id" => TEST_CLEAR_ACCESS_SPACE_ID,
            "workspace_id" => TEST_CLEAR_WORKSPACE_ID,
            _ => {
                return Err(ProcessLedgerError::InvalidConfig(
                    "unsupported ProcessLedger inspection scope field".to_owned(),
                ))
            }
        };
        let affected = self
            .storage
            .with_data_operation(|database| {
                let bindings = InspectionScopeTamperBindings::new(resource_scope, process_uuid);
                Box::pin(async move { database.execute_returning(statement, bindings).await })
            })
            .await
            .map_err(surreal_store_error)?;
        if affected != 1 {
            return Err(inspection_error("test scope tamper escaped exact scope"));
        }
        Ok(())
    }

    #[cfg(any(test, feature = "test-utils"))]
    pub async fn test_set_inspection_identity_field(
        &self,
        resource_scope: &ReclaimResourceScope,
        process_uuid: Uuid,
        field: &str,
        value: Option<&str>,
    ) -> Result<(), ProcessLedgerError> {
        validate_inspection_scope(resource_scope)?;
        let statement = match field {
            "engine_kind" => TEST_SET_INSPECTION_ENGINE_KIND,
            "owner_role" => TEST_SET_INSPECTION_OWNER_ROLE,
            "owner_wp" => TEST_SET_INSPECTION_OWNER_WP,
            "sandbox_adapter_id" => TEST_SET_INSPECTION_SANDBOX_ADAPTER_ID,
            _ => {
                return Err(ProcessLedgerError::InvalidConfig(
                    "unsupported ProcessLedger inspection identity field".to_owned(),
                ))
            }
        };
        let affected = self
            .storage
            .with_data_operation(|database| {
                let bindings = InspectionIdentityTamperBindings::new(
                    resource_scope,
                    process_uuid,
                    value.map(str::to_owned),
                );
                Box::pin(async move { database.execute_returning(statement, bindings).await })
            })
            .await
            .map_err(surreal_store_error)?;
        if affected != 1 {
            return Err(inspection_error("test identity tamper escaped exact scope"));
        }
        Ok(())
    }

    #[cfg(any(test, feature = "test-utils"))]
    pub async fn test_replace_inspection_runtime_owner(
        &self,
        resource_scope: &ReclaimResourceScope,
        process_uuid: Uuid,
        runtime_owner: &ProcessRuntimeOwner,
    ) -> Result<(), ProcessLedgerError> {
        validate_inspection_scope(resource_scope)?;
        let affected = self
            .storage
            .with_data_operation(|database| {
                let bindings = InspectionRuntimeOwnerTamperBindings::new(
                    resource_scope,
                    process_uuid,
                    runtime_owner,
                );
                Box::pin(async move {
                    database
                        .execute_returning(TEST_REPLACE_INSPECTION_RUNTIME_OWNER, bindings)
                        .await
                })
            })
            .await
            .map_err(surreal_store_error)?;
        if affected != 1 {
            return Err(inspection_error(
                "test runtime-owner tamper escaped exact scope",
            ));
        }
        Ok(())
    }

    #[cfg(any(test, feature = "test-utils"))]
    pub async fn test_clear_inspection_runtime_owner_host_scope(
        &self,
        resource_scope: &ReclaimResourceScope,
        process_uuid: Uuid,
    ) -> Result<(), ProcessLedgerError> {
        validate_inspection_scope(resource_scope)?;
        let affected = self
            .storage
            .with_data_operation(|database| {
                let bindings = InspectionScopeTamperBindings::new(resource_scope, process_uuid);
                Box::pin(async move {
                    database
                        .execute_returning(TEST_CLEAR_INSPECTION_RUNTIME_OWNER_HOST_SCOPE, bindings)
                        .await
                })
            })
            .await
            .map_err(surreal_store_error)?;
        if affected != 1 {
            return Err(inspection_error(
                "test partial runtime-owner tamper escaped exact scope",
            ));
        }
        Ok(())
    }

    #[cfg(any(test, feature = "test-utils"))]
    pub async fn test_delete_inspection_lifecycle_projection(
        &self,
        resource_scope: &ReclaimResourceScope,
        process_uuid: Uuid,
    ) -> Result<(), ProcessLedgerError> {
        validate_inspection_scope(resource_scope)?;
        let affected = self
            .storage
            .with_data_operation(|database| {
                let bindings = InspectionScopeTamperBindings::new(resource_scope, process_uuid);
                Box::pin(async move {
                    database
                        .execute_returning(TEST_DELETE_INSPECTION_LIFECYCLE, bindings)
                        .await
                })
            })
            .await
            .map_err(surreal_store_error)?;
        if affected != 1 {
            return Err(inspection_error(
                "test lifecycle deletion escaped exact scope",
            ));
        }
        Ok(())
    }

    #[cfg(any(test, feature = "test-utils"))]
    pub async fn test_delete_inspection_receipt(
        &self,
        resource_scope: &ReclaimResourceScope,
        process_uuid: Uuid,
        kind: LedgerEventKind,
    ) -> Result<(), ProcessLedgerError> {
        validate_inspection_scope(resource_scope)?;
        let bindings = InspectionReceiptDeleteBindings::new(
            resource_scope,
            RecordId::new(
                "kernel_event_ledger",
                format!(
                    "process-lifecycle-{process_uuid}-{}",
                    kind.as_str().to_ascii_lowercase()
                ),
            ),
        );
        let affected = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .execute_returning(TEST_DELETE_INSPECTION_RECEIPT, bindings)
                        .await
                })
            })
            .await
            .map_err(surreal_store_error)?;
        if affected != 1 {
            return Err(inspection_error(
                "test receipt deletion escaped exact scope",
            ));
        }
        Ok(())
    }

    #[cfg(any(test, feature = "test-utils"))]
    pub async fn test_move_inspection_receipt_to_scope(
        &self,
        resource_scope: &ReclaimResourceScope,
        process_uuid: Uuid,
        kind: LedgerEventKind,
        foreign_scope: &ReclaimResourceScope,
    ) -> Result<(), ProcessLedgerError> {
        validate_inspection_scope(resource_scope)?;
        validate_inspection_scope(foreign_scope)?;
        let bindings = InspectionReceiptScopeTamperBindings::new(
            resource_scope,
            foreign_scope,
            RecordId::new(
                "kernel_event_ledger",
                format!(
                    "process-lifecycle-{process_uuid}-{}",
                    kind.as_str().to_ascii_lowercase()
                ),
            ),
        );
        let affected = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .execute_returning(TEST_MOVE_INSPECTION_RECEIPT_SCOPE, bindings)
                        .await
                })
            })
            .await
            .map_err(surreal_store_error)?;
        if affected != 1 {
            return Err(inspection_error(
                "test receipt scope tamper escaped exact scope",
            ));
        }
        Ok(())
    }

    #[cfg(any(test, feature = "test-utils"))]
    pub async fn test_duplicate_inspection_identity(
        &self,
        resource_scope: &ReclaimResourceScope,
        process_uuid: Uuid,
    ) -> Result<(), ProcessLedgerError> {
        validate_inspection_scope(resource_scope)?;
        let bindings = InspectionDuplicateBindings::new(resource_scope, process_uuid);
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values_at::<surrealdb::types::Value, _>(
                            TEST_DUPLICATE_INSPECTION_IDENTITY,
                            bindings,
                            2,
                        )
                        .await
                })
            })
            .await
            .map_err(surreal_store_error)?;
        if rows.len() != 1 {
            return Err(inspection_error(
                "test duplicate identity setup escaped exact scope",
            ));
        }
        Ok(())
    }

    async fn claim_rows(
        &self,
        statement: &'static str,
        bindings: ClaimBindings,
    ) -> Result<Vec<ReclaimableProcess>, ProcessLedgerError> {
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values::<ReclaimRow, _>(statement, bindings)
                        .await
                })
            })
            .await
            .map_err(surreal_store_error)?;
        rows.into_iter().map(ReclaimableProcess::try_from).collect()
    }

    fn claim_bindings(
        resource_scope: &ReclaimResourceScope,
        session_id: &str,
        process_uuid: Uuid,
        owner_runtime_instance_id: Uuid,
        owner_host_scope_id: &str,
        excluded_owner_runtime_instance_id: Uuid,
        mut authorized_process_uuids: Vec<Uuid>,
    ) -> ClaimBindings {
        authorized_process_uuids.sort_unstable();
        let now_unix_ms = Utc::now().timestamp_millis();
        ClaimBindings {
            owner_account_id: resource_scope.account_uuid.to_string(),
            actor_principal_id: resource_scope.actor_uuid.to_string(),
            authenticated_session_id: resource_scope.session_uuid.to_string(),
            access_space_id: resource_scope.access_space_uuid.to_string(),
            workspace_id: resource_scope.workspace_id.clone(),
            session_id: session_id.to_owned(),
            process_uuid,
            owner_runtime_instance_id,
            owner_host_scope_id: owner_host_scope_id.to_owned(),
            excluded_owner_runtime_instance_id,
            authorized_process_uuids,
            claimant_uuid: Uuid::now_v7(),
            kill_operation_uuid: Uuid::now_v7(),
            now_unix_ms,
            lease_expires_at_unix_ms: now_unix_ms + 30_000,
        }
    }

    fn fence_bindings(
        process_uuid: Uuid,
        claim: &ReclaimClaim,
        metadata: Value,
        resolution_status: &str,
    ) -> Result<ClaimFenceBindings, ProcessLedgerError> {
        let generation = i64::try_from(claim.generation).map_err(|_| {
            ProcessLedgerError::Store("reclaim claim generation exceeds Surreal int".to_owned())
        })?;
        let now_unix_ms = Utc::now().timestamp_millis();
        Ok(ClaimFenceBindings {
            owner_account_id: claim.resource_scope.account_uuid.to_string(),
            actor_principal_id: claim.resource_scope.actor_uuid.to_string(),
            authenticated_session_id: claim.resource_scope.session_uuid.to_string(),
            access_space_id: claim.resource_scope.access_space_uuid.to_string(),
            workspace_id: claim.resource_scope.workspace_id.clone(),
            process_uuid,
            claimant_uuid: claim.claimant_uuid,
            kill_operation_uuid: claim.kill_operation_uuid,
            generation,
            now_unix_ms,
            lease_expires_at_unix_ms: now_unix_ms + 30_000,
            metadata,
            resolution_status: resolution_status.to_owned(),
        })
    }

    async fn fenced_update(
        &self,
        statement: &'static str,
        bindings: ClaimFenceBindings,
        failure: String,
    ) -> Result<ReclaimRow, ProcessLedgerError> {
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values::<ReclaimRow, _>(statement, bindings)
                        .await
                })
            })
            .await
            .map_err(surreal_store_error)?;
        if rows.len() != 1 {
            return Err(ProcessLedgerError::Store(failure));
        }
        Ok(rows.into_iter().next().expect("one row checked"))
    }

    async fn recovery_rows(
        &self,
        statement: &'static str,
        resource_scope: &ReclaimResourceScope,
        session_id: &str,
        excluded_owner_runtime_instance_id: Uuid,
        owner_runtime_instance_id: Uuid,
        owner_host_scope_id: &str,
        mut authorized_process_uuids: Vec<Uuid>,
        limit: usize,
    ) -> Result<Vec<ReclaimKillOperationCandidate>, ProcessLedgerError> {
        if limit == 0 || limit > 64 {
            return Err(ProcessLedgerError::InvalidConfig(
                "in-progress reclaim recovery limit must be 1..=64".to_owned(),
            ));
        }
        authorized_process_uuids.sort_unstable();
        let bindings = RecoveryBindings {
            owner_account_id: resource_scope.account_uuid.to_string(),
            actor_principal_id: resource_scope.actor_uuid.to_string(),
            authenticated_session_id: resource_scope.session_uuid.to_string(),
            access_space_id: resource_scope.access_space_uuid.to_string(),
            workspace_id: resource_scope.workspace_id.clone(),
            session_id: session_id.to_owned(),
            excluded_owner_runtime_instance_id,
            owner_runtime_instance_id,
            owner_host_scope_id: owner_host_scope_id.to_owned(),
            authorized_process_uuids,
            limit: i64::try_from(limit).expect("recovery limit is at most 64"),
        };
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values::<RecoveryRow, _>(statement, bindings)
                        .await
                })
            })
            .await
            .map_err(surreal_store_error)?;
        Ok(rows
            .into_iter()
            .map(|row| {
                let row_scope = ReclaimResourceScope::try_from_stored(
                    &row.owner_account_id,
                    &row.actor_principal_id,
                    &row.authenticated_session_id,
                    &row.workspace_id,
                    &row.access_space_id,
                );
                if !matches!(row_scope, Ok(ref scope) if scope == resource_scope) {
                    return ReclaimKillOperationCandidate::Malformed {
                        process_identity: row.process_uuid.to_string(),
                        kill_operation_identity: row
                            .reclaim_kill_operation_uuid
                            .map(|value| value.to_string()),
                        error: "in-progress Surreal reclaim row escaped its exact ResourceScope"
                            .to_owned(),
                    };
                }
                match row.reclaim_kill_operation_uuid {
                    Some(kill_operation_uuid) => ReclaimKillOperationCandidate::Operation {
                        operation: ReclaimKillOperation {
                            resource_scope: resource_scope.clone(),
                            process_uuid: row.process_uuid,
                            kill_operation_uuid,
                        },
                    },
                    None => ReclaimKillOperationCandidate::Malformed {
                        process_identity: row.process_uuid.to_string(),
                        kill_operation_identity: None,
                        error: format!(
                            "in-progress process {} is missing kill_operation_uuid",
                            row.process_uuid
                        ),
                    },
                }
            })
            .collect())
    }
}

pub(crate) async fn bootstrap_process_ledger_schema(
    storage: &SurrealStorage,
) -> Result<(), SurrealStorageError> {
    storage
        .with_admin_operation(|database| {
            Box::pin(async move { database.query(SCHEMA).await.map(|_| ()) })
        })
        .await
}

fn surreal_store_error(error: SurrealStorageError) -> ProcessLedgerError {
    ProcessLedgerError::Store(error.to_string())
}

fn exact_resource_scope_from_parts(
    values: [Option<&str>; 5],
) -> Result<Option<ExactResourceScope>, ProcessLedgerError> {
    if values.iter().all(Option::is_none) {
        return Ok(None);
    }
    if values.iter().any(Option::is_none) {
        return Err(ProcessLedgerError::Store(
            "partial ResourceScope attribution in Surreal stale-session row".to_owned(),
        ));
    }
    let values = values.map(Option::unwrap);
    if values.iter().any(|value| value.trim().is_empty()) {
        return Err(ProcessLedgerError::Store(
            "empty ResourceScope attribution in Surreal stale-session row".to_owned(),
        ));
    }
    for value in values.iter().take(4) {
        Uuid::parse_str(value).map_err(|_| {
            ProcessLedgerError::Store(
                "invalid ResourceScope UUID in Surreal stale-session row".to_owned(),
            )
        })?;
    }
    Ok(Some(ExactResourceScope {
        owner_account_id: values[0].to_owned(),
        actor_principal_id: values[1].to_owned(),
        authenticated_session_id: values[2].to_owned(),
        access_space_id: values[3].to_owned(),
        workspace_id: values[4].to_owned(),
    }))
}

fn stale_lifecycle_scope(
    row: &StaleLifecycleRow,
) -> Result<Option<ExactResourceScope>, ProcessLedgerError> {
    exact_resource_scope_from_parts([
        row.owner_account_id.as_deref(),
        row.actor_principal_id.as_deref(),
        row.authenticated_session_id.as_deref(),
        row.access_space_id.as_deref(),
        row.workspace_id.as_deref(),
    ])
}

fn restart_lifecycle_scope(
    row: &RestartLifecycleRow,
) -> Result<Option<ExactResourceScope>, ProcessLedgerError> {
    exact_resource_scope_from_parts([
        row.owner_account_id.as_deref(),
        row.actor_principal_id.as_deref(),
        row.authenticated_session_id.as_deref(),
        row.access_space_id.as_deref(),
        row.workspace_id.as_deref(),
    ])
}

fn runtime_owner_from_restart_row(
    row: &RestartLifecycleRow,
) -> Result<Option<ProcessRuntimeOwner>, ProcessLedgerError> {
    match (
        row.owner_runtime_instance_id,
        row.owner_host_scope_id.as_ref(),
        row.owner_lease_schema_id.as_ref(),
        row.owner_lease_protocol.as_ref(),
        row.owner_lease_address.as_ref(),
        row.owner_lease_port,
    ) {
        (None, None, None, None, None, None) => Ok(None),
        (
            Some(runtime_instance_id),
            Some(host_scope_id),
            Some(lease_schema_id),
            Some(lease_protocol),
            Some(lease_address),
            Some(lease_port),
        ) => {
            let lease_port = u16::try_from(lease_port).map_err(|_| {
                ProcessLedgerError::Store(
                    "owner_lease_port in Surreal restart row is outside 1..=65535".to_owned(),
                )
            })?;
            if lease_port == 0 {
                return Err(ProcessLedgerError::Store(
                    "owner_lease_port in Surreal restart row must not be zero".to_owned(),
                ));
            }
            Ok(Some(ProcessRuntimeOwner {
                runtime_instance_id,
                host_scope_id: host_scope_id.clone(),
                lease_schema_id: lease_schema_id.clone(),
                lease_protocol: lease_protocol.clone(),
                lease_address: lease_address.clone(),
                lease_port,
            }))
        }
        _ => Err(ProcessLedgerError::Store(
            "partial typed runtime-owner identity in Surreal restart row".to_owned(),
        )),
    }
}

fn model_lane_authority_scope(
    row: &ModelLaneAuthorityRow,
) -> Result<ExactResourceScope, ProcessLedgerError> {
    exact_resource_scope_from_parts([
        Some(row.owner_account_id.as_str()),
        Some(row.actor_principal_id.as_str()),
        Some(row.authenticated_session_id.as_str()),
        Some(row.access_space_id.as_str()),
        Some(row.workspace_id.as_str()),
    ])?
    .ok_or_else(|| {
        ProcessLedgerError::Store("model-lane authority row has no exact ResourceScope".to_owned())
    })
}

fn reclaim_resource_scope(
    scope: &ExactResourceScope,
) -> Result<ReclaimResourceScope, ProcessLedgerError> {
    ReclaimResourceScope::try_from_stored(
        &scope.owner_account_id,
        &scope.actor_principal_id,
        &scope.authenticated_session_id,
        &scope.workspace_id,
        &scope.access_space_id,
    )
}

fn parse_optional_model_lane_time(
    record: &Value,
    field: &str,
) -> Result<Option<DateTime<Utc>>, ProcessLedgerError> {
    record
        .get(field)
        .and_then(Value::as_str)
        .map(|value| {
            DateTime::parse_from_rfc3339(value)
                .map(|time| time.with_timezone(&Utc))
                .map_err(|error| {
                    ProcessLedgerError::Store(format!(
                        "model lane {field} is invalid RFC3339: {error}"
                    ))
                })
        })
        .transpose()
}

fn exact_model_lane_reclaimability(
    record: &Value,
    session_id: &str,
    process_uuid: Uuid,
    now: DateTime<Utc>,
    ttl: chrono::Duration,
) -> Result<Option<bool>, ProcessLedgerError> {
    let expected_ownership_ref = format!("process-ledger://{process_uuid}");
    if record.get("coordinator_session_id").and_then(Value::as_str) != Some(session_id)
        || record.get("process_ownership_ref").and_then(Value::as_str)
            != Some(expected_ownership_ref.as_str())
    {
        return Ok(None);
    }
    let status = record.get("status").and_then(Value::as_str).unwrap_or("");
    let terminal = matches!(status, "completed" | "failed" | "cancelled" | "reclaimable");
    let reclaim_due = parse_optional_model_lane_time(record, "reclaim_after_utc")?
        .is_some_and(|deadline| deadline <= now);
    let heartbeat_stale = parse_optional_model_lane_time(record, "heartbeat_at_utc")?
        .is_some_and(|heartbeat| heartbeat < now - ttl);
    Ok(Some(terminal || reclaim_due || heartbeat_stale))
}

#[derive(Debug, Clone, SurrealValue)]
struct LifecycleRow {
    process_uuid: Uuid,
    os_pid: Option<i64>,
    parent_session_id: Option<String>,
    parent_process_id: Option<Uuid>,
    sandbox_adapter_id: Option<String>,
    sandbox_internal_id: Option<String>,
    engine_kind: String,
    started_at: DateTime<Utc>,
    stopped_at: Option<DateTime<Utc>>,
    exit_code: Option<i64>,
    stop_reason: Option<String>,
    model_artifact_sha256: Option<String>,
    work_profile_id: Option<String>,
    owner_role: String,
    owner_wp: Option<String>,
    role_id: Option<String>,
    wp_id: Option<String>,
    mt_id: Option<String>,
    owner_runtime_instance_id: Option<Uuid>,
    owner_host_scope_id: Option<String>,
    owner_lease_schema_id: Option<String>,
    owner_lease_protocol: Option<String>,
    owner_lease_address: Option<String>,
    owner_lease_port: Option<i64>,
    owner_account_id: Option<String>,
    actor_principal_id: Option<String>,
    authenticated_session_id: Option<String>,
    access_space_id: Option<String>,
    workspace_id: Option<String>,
    sandbox_capabilities_snapshot: Value,
    metadata: Value,
}

#[derive(Debug, Clone, SurrealValue)]
struct InspectionLifecycleRow {
    id: RecordId,
    exact_scope_match: bool,
    process_uuid: Uuid,
    os_pid: Option<i64>,
    model_artifact_sha256: Option<String>,
    engine_kind: String,
    owner_role: String,
    owner_wp: Option<String>,
    sandbox_adapter_id: Option<String>,
    started_at: DateTime<Utc>,
    stopped_at: Option<DateTime<Utc>>,
    exit_code: Option<i64>,
    stop_reason: Option<String>,
    owner_runtime_instance_id: Option<Uuid>,
    owner_host_scope_id: Option<String>,
    owner_lease_schema_id: Option<String>,
    owner_lease_protocol: Option<String>,
    owner_lease_address: Option<String>,
    owner_lease_port: Option<i64>,
    owner_account_id: Option<String>,
    actor_principal_id: Option<String>,
    authenticated_session_id: Option<String>,
    access_space_id: Option<String>,
    workspace_id: Option<String>,
    event_ledger_event_id: Option<RecordId>,
}

#[derive(Debug, SurrealValue)]
struct InspectionEventRow {
    id: RecordId,
    event_id: String,
    event_version: String,
    aggregate_type: String,
    aggregate_id: String,
    idempotency_key: String,
    event_type: String,
    payload_hash: String,
    source_component: String,
    payload: Value,
    owner_account_id: Option<String>,
    actor_principal_id: Option<String>,
    authenticated_session_id: Option<String>,
    access_space_id: Option<String>,
    workspace_id: Option<String>,
}

#[derive(Debug, SurrealValue)]
struct InspectionProcessBindings {
    record: RecordId,
    process_uuid: Uuid,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

impl InspectionProcessBindings {
    fn new(scope: &ReclaimResourceScope, process_uuid: Uuid, record: RecordId) -> Self {
        Self {
            record,
            process_uuid,
            owner_account_id: scope.account_uuid.to_string(),
            actor_principal_id: scope.actor_uuid.to_string(),
            authenticated_session_id: scope.session_uuid.to_string(),
            access_space_id: scope.access_space_uuid.to_string(),
            workspace_id: scope.workspace_id.clone(),
        }
    }
}

#[derive(Debug, SurrealValue)]
struct InspectionArtifactBindings {
    model_artifact_sha256: String,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

impl InspectionArtifactBindings {
    fn new(scope: &ReclaimResourceScope, model_artifact_sha256: &str) -> Self {
        Self {
            model_artifact_sha256: model_artifact_sha256.to_owned(),
            owner_account_id: scope.account_uuid.to_string(),
            actor_principal_id: scope.actor_uuid.to_string(),
            authenticated_session_id: scope.session_uuid.to_string(),
            access_space_id: scope.access_space_uuid.to_string(),
            workspace_id: scope.workspace_id.clone(),
        }
    }
}

#[derive(Debug, SurrealValue)]
struct InspectionReceiptBindings {
    event_record: RecordId,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

impl InspectionReceiptBindings {
    fn new(scope: &ReclaimResourceScope, event_record: RecordId) -> Self {
        Self {
            event_record,
            owner_account_id: scope.account_uuid.to_string(),
            actor_principal_id: scope.actor_uuid.to_string(),
            authenticated_session_id: scope.session_uuid.to_string(),
            access_space_id: scope.access_space_uuid.to_string(),
            workspace_id: scope.workspace_id.clone(),
        }
    }
}

#[cfg(any(test, feature = "test-utils"))]
#[derive(Debug, SurrealValue)]
struct InspectionLinkTamperBindings {
    process_uuid: Uuid,
    event_ledger_event_id: Option<RecordId>,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[cfg(any(test, feature = "test-utils"))]
impl InspectionLinkTamperBindings {
    fn new(
        scope: &ReclaimResourceScope,
        process_uuid: Uuid,
        event_ledger_event_id: Option<RecordId>,
    ) -> Self {
        Self {
            process_uuid,
            event_ledger_event_id,
            owner_account_id: scope.account_uuid.to_string(),
            actor_principal_id: scope.actor_uuid.to_string(),
            authenticated_session_id: scope.session_uuid.to_string(),
            access_space_id: scope.access_space_uuid.to_string(),
            workspace_id: scope.workspace_id.clone(),
        }
    }
}

#[cfg(any(test, feature = "test-utils"))]
#[derive(Debug, SurrealValue)]
struct InspectionScopeTamperBindings {
    process_uuid: Uuid,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[cfg(any(test, feature = "test-utils"))]
#[derive(Debug, SurrealValue)]
struct InspectionIdentityTamperBindings {
    process_uuid: Uuid,
    value: Option<String>,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[cfg(any(test, feature = "test-utils"))]
impl InspectionIdentityTamperBindings {
    fn new(scope: &ReclaimResourceScope, process_uuid: Uuid, value: Option<String>) -> Self {
        Self {
            process_uuid,
            value,
            owner_account_id: scope.account_uuid.to_string(),
            actor_principal_id: scope.actor_uuid.to_string(),
            authenticated_session_id: scope.session_uuid.to_string(),
            access_space_id: scope.access_space_uuid.to_string(),
            workspace_id: scope.workspace_id.clone(),
        }
    }
}

#[cfg(any(test, feature = "test-utils"))]
#[derive(Debug, SurrealValue)]
struct InspectionRuntimeOwnerTamperBindings {
    process_uuid: Uuid,
    replacement_runtime_instance_id: Uuid,
    replacement_host_scope_id: String,
    replacement_lease_schema_id: String,
    replacement_lease_protocol: String,
    replacement_lease_address: String,
    replacement_lease_port: i64,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[cfg(any(test, feature = "test-utils"))]
impl InspectionRuntimeOwnerTamperBindings {
    fn new(
        scope: &ReclaimResourceScope,
        process_uuid: Uuid,
        runtime_owner: &ProcessRuntimeOwner,
    ) -> Self {
        Self {
            process_uuid,
            replacement_runtime_instance_id: runtime_owner.runtime_instance_id,
            replacement_host_scope_id: runtime_owner.host_scope_id.clone(),
            replacement_lease_schema_id: runtime_owner.lease_schema_id.clone(),
            replacement_lease_protocol: runtime_owner.lease_protocol.clone(),
            replacement_lease_address: runtime_owner.lease_address.clone(),
            replacement_lease_port: i64::from(runtime_owner.lease_port),
            owner_account_id: scope.account_uuid.to_string(),
            actor_principal_id: scope.actor_uuid.to_string(),
            authenticated_session_id: scope.session_uuid.to_string(),
            access_space_id: scope.access_space_uuid.to_string(),
            workspace_id: scope.workspace_id.clone(),
        }
    }
}

#[cfg(any(test, feature = "test-utils"))]
impl InspectionScopeTamperBindings {
    fn new(scope: &ReclaimResourceScope, process_uuid: Uuid) -> Self {
        Self {
            process_uuid,
            owner_account_id: scope.account_uuid.to_string(),
            actor_principal_id: scope.actor_uuid.to_string(),
            authenticated_session_id: scope.session_uuid.to_string(),
            access_space_id: scope.access_space_uuid.to_string(),
            workspace_id: scope.workspace_id.clone(),
        }
    }
}

#[cfg(any(test, feature = "test-utils"))]
#[derive(Debug, SurrealValue)]
struct InspectionReceiptScopeTamperBindings {
    event_record: RecordId,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
    foreign_owner_account_id: String,
    foreign_actor_principal_id: String,
    foreign_authenticated_session_id: String,
    foreign_access_space_id: String,
    foreign_workspace_id: String,
}

#[cfg(any(test, feature = "test-utils"))]
impl InspectionReceiptScopeTamperBindings {
    fn new(
        scope: &ReclaimResourceScope,
        foreign_scope: &ReclaimResourceScope,
        event_record: RecordId,
    ) -> Self {
        Self {
            event_record,
            owner_account_id: scope.account_uuid.to_string(),
            actor_principal_id: scope.actor_uuid.to_string(),
            authenticated_session_id: scope.session_uuid.to_string(),
            access_space_id: scope.access_space_uuid.to_string(),
            workspace_id: scope.workspace_id.clone(),
            foreign_owner_account_id: foreign_scope.account_uuid.to_string(),
            foreign_actor_principal_id: foreign_scope.actor_uuid.to_string(),
            foreign_authenticated_session_id: foreign_scope.session_uuid.to_string(),
            foreign_access_space_id: foreign_scope.access_space_uuid.to_string(),
            foreign_workspace_id: foreign_scope.workspace_id.clone(),
        }
    }
}

#[cfg(any(test, feature = "test-utils"))]
#[derive(Debug, SurrealValue)]
struct InspectionReceiptDeleteBindings {
    event_record: RecordId,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[cfg(any(test, feature = "test-utils"))]
impl InspectionReceiptDeleteBindings {
    fn new(scope: &ReclaimResourceScope, event_record: RecordId) -> Self {
        Self {
            event_record,
            owner_account_id: scope.account_uuid.to_string(),
            actor_principal_id: scope.actor_uuid.to_string(),
            authenticated_session_id: scope.session_uuid.to_string(),
            access_space_id: scope.access_space_uuid.to_string(),
            workspace_id: scope.workspace_id.clone(),
        }
    }
}

#[cfg(any(test, feature = "test-utils"))]
#[derive(Debug, SurrealValue)]
struct InspectionDuplicateBindings {
    source_record: RecordId,
    duplicate_record: RecordId,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[cfg(any(test, feature = "test-utils"))]
impl InspectionDuplicateBindings {
    fn new(scope: &ReclaimResourceScope, process_uuid: Uuid) -> Self {
        Self {
            source_record: RecordId::new(PROCESS_LEDGER_TABLE_NAME, process_uuid.to_string()),
            duplicate_record: RecordId::new(
                PROCESS_LEDGER_TABLE_NAME,
                format!("inspection-duplicate-{process_uuid}"),
            ),
            owner_account_id: scope.account_uuid.to_string(),
            actor_principal_id: scope.actor_uuid.to_string(),
            authenticated_session_id: scope.session_uuid.to_string(),
            access_space_id: scope.access_space_uuid.to_string(),
            workspace_id: scope.workspace_id.clone(),
        }
    }
}

#[derive(Debug, Clone, SurrealValue)]
struct BatchApplyItem {
    record: RecordId,
    incoming: LifecycleRow,
    is_start: bool,
    event_record: RecordId,
    event_id: String,
    event_type: String,
    idempotency_key: String,
    task_run_id: String,
    session_run_id: String,
    actor_id: String,
    payload_hash: String,
    event_payload: Value,
    identity_conflict_marker: String,
    verification_mismatch_marker: String,
}

#[derive(Debug, SurrealValue)]
struct ExpectedLifecycleRow {
    record: RecordId,
    incoming: LifecycleRow,
    event_record: RecordId,
    verification_mismatch_marker: String,
}

#[derive(Debug, SurrealValue)]
struct BatchApplyBindings {
    items: Vec<BatchApplyItem>,
    expected_final_rows: Vec<ExpectedLifecycleRow>,
}

#[derive(Debug, SurrealValue)]
struct ReclaimRow {
    process_uuid: Uuid,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
    os_pid: Option<i64>,
    parent_session_id: Option<String>,
    parent_process_id: Option<Uuid>,
    sandbox_adapter_id: Option<String>,
    sandbox_internal_id: Option<String>,
    engine_kind: String,
    started_at: DateTime<Utc>,
    model_artifact_sha256: Option<String>,
    work_profile_id: Option<String>,
    owner_role: String,
    owner_wp: Option<String>,
    role_id: Option<String>,
    wp_id: Option<String>,
    mt_id: Option<String>,
    owner_runtime_instance_id: Option<Uuid>,
    owner_host_scope_id: Option<String>,
    owner_lease_schema_id: Option<String>,
    owner_lease_protocol: Option<String>,
    owner_lease_address: Option<String>,
    owner_lease_port: Option<i64>,
    sandbox_capabilities_snapshot: Value,
    metadata: Value,
    stop_reason: Option<String>,
    reclaim_claimant_uuid: Option<Uuid>,
    reclaim_kill_operation_uuid: Option<Uuid>,
    reclaim_generation: Option<i64>,
    reclaim_claimed_at_unix_ms: Option<i64>,
    reclaim_lease_expires_at_unix_ms: Option<i64>,
}

#[derive(Debug, SurrealValue)]
struct ClaimBindings {
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
    session_id: String,
    process_uuid: Uuid,
    owner_runtime_instance_id: Uuid,
    owner_host_scope_id: String,
    excluded_owner_runtime_instance_id: Uuid,
    authorized_process_uuids: Vec<Uuid>,
    claimant_uuid: Uuid,
    kill_operation_uuid: Uuid,
    now_unix_ms: i64,
    lease_expires_at_unix_ms: i64,
}

#[derive(Debug, SurrealValue)]
struct ClaimFenceBindings {
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
    process_uuid: Uuid,
    claimant_uuid: Uuid,
    kill_operation_uuid: Uuid,
    generation: i64,
    now_unix_ms: i64,
    lease_expires_at_unix_ms: i64,
    metadata: Value,
    resolution_status: String,
}

#[derive(Debug, SurrealValue)]
struct RecoveryBindings {
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
    session_id: String,
    excluded_owner_runtime_instance_id: Uuid,
    owner_runtime_instance_id: Uuid,
    owner_host_scope_id: String,
    authorized_process_uuids: Vec<Uuid>,
    limit: i64,
}

#[derive(Debug, SurrealValue)]
struct RecoveryRow {
    process_uuid: Uuid,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
    reclaim_kill_operation_uuid: Option<Uuid>,
}

#[derive(Debug, SurrealValue)]
struct StaleLifecycleRow {
    process_uuid: Uuid,
    parent_session_id: Option<String>,
    owner_runtime_instance_id: Option<Uuid>,
    owner_host_scope_id: Option<String>,
    owner_account_id: Option<String>,
    actor_principal_id: Option<String>,
    authenticated_session_id: Option<String>,
    access_space_id: Option<String>,
    workspace_id: Option<String>,
}

#[derive(Debug, SurrealValue)]
struct ModelLaneAuthorityRow {
    record_json: String,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(Debug, SurrealValue)]
struct RestartLifecycleRow {
    process_uuid: Uuid,
    parent_session_id: Option<String>,
    sandbox_adapter_id: Option<String>,
    owner_runtime_instance_id: Option<Uuid>,
    owner_host_scope_id: Option<String>,
    owner_lease_schema_id: Option<String>,
    owner_lease_protocol: Option<String>,
    owner_lease_address: Option<String>,
    owner_lease_port: Option<i64>,
    owner_account_id: Option<String>,
    actor_principal_id: Option<String>,
    authenticated_session_id: Option<String>,
    access_space_id: Option<String>,
    workspace_id: Option<String>,
}

#[derive(Debug, SurrealValue)]
struct StaleLifecycleBindings {
    complete_scope: bool,
}

#[derive(Debug, SurrealValue)]
struct RestartLifecycleBindings {
    complete_scope: bool,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct ExactResourceScope {
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(Debug, SurrealValue)]
struct ModelLaneAuthorityBindings {
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

const INSPECT_CANONICAL_PROCESS_RECORD: &str = r#"
SELECT id, process_uuid, os_pid, model_artifact_sha256, started_at, stopped_at,
    engine_kind, owner_role, owner_wp, sandbox_adapter_id,
    exit_code, stop_reason, owner_runtime_instance_id, owner_host_scope_id,
    owner_lease_schema_id, owner_lease_protocol, owner_lease_address,
    owner_lease_port, owner_account_id, actor_principal_id,
    authenticated_session_id, access_space_id, workspace_id,
    event_ledger_event_id,
    (owner_account_id = $owner_account_id
        AND actor_principal_id = $actor_principal_id
        AND authenticated_session_id = $authenticated_session_id
        AND access_space_id = $access_space_id
        AND workspace_id = $workspace_id) AS exact_scope_match
FROM ONLY $record
WHERE process_uuid = $process_uuid;
"#;

const INSPECT_EXACT_PROCESS_ROWS: &str = r#"
SELECT id, process_uuid, os_pid, model_artifact_sha256, started_at, stopped_at,
    engine_kind, owner_role, owner_wp, sandbox_adapter_id,
    exit_code, stop_reason, owner_runtime_instance_id, owner_host_scope_id,
    owner_lease_schema_id, owner_lease_protocol, owner_lease_address,
    owner_lease_port, owner_account_id, actor_principal_id,
    authenticated_session_id, access_space_id, workspace_id,
    event_ledger_event_id, true AS exact_scope_match
FROM kernel_process_lifecycle
WHERE process_uuid = $process_uuid
    AND owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
ORDER BY started_at DESC, id ASC;
"#;

const INSPECT_EXACT_ARTIFACT_ROWS: &str = r#"
SELECT id, process_uuid, os_pid, model_artifact_sha256, started_at, stopped_at,
    engine_kind, owner_role, owner_wp, sandbox_adapter_id,
    exit_code, stop_reason, owner_runtime_instance_id, owner_host_scope_id,
    owner_lease_schema_id, owner_lease_protocol, owner_lease_address,
    owner_lease_port, owner_account_id, actor_principal_id,
    authenticated_session_id, access_space_id, workspace_id,
    event_ledger_event_id, true AS exact_scope_match
FROM kernel_process_lifecycle
WHERE model_artifact_sha256 = $model_artifact_sha256
    AND owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
ORDER BY started_at DESC, process_uuid DESC, id ASC;
"#;

const INSPECT_EXACT_RECEIPT: &str = r#"
SELECT id, event_id, event_version, aggregate_type, aggregate_id,
    idempotency_key, event_type, payload_hash, source_component, payload,
    owner_account_id, actor_principal_id, authenticated_session_id,
    access_space_id, workspace_id
FROM ONLY $event_record
WHERE owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id;
"#;

#[cfg(any(test, feature = "test-utils"))]
const TEST_SET_INSPECTION_EVENT_LINK: &str = r#"
UPDATE kernel_process_lifecycle SET event_ledger_event_id = $event_ledger_event_id
WHERE process_uuid = $process_uuid
    AND owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
RETURN AFTER;
"#;

#[cfg(any(test, feature = "test-utils"))]
const TEST_SET_INSPECTION_ENGINE_KIND: &str = r#"
UPDATE kernel_process_lifecycle SET engine_kind = $value
WHERE process_uuid = $process_uuid
    AND owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
RETURN AFTER;
"#;

#[cfg(any(test, feature = "test-utils"))]
const TEST_SET_INSPECTION_OWNER_ROLE: &str = r#"
UPDATE kernel_process_lifecycle SET owner_role = $value
WHERE process_uuid = $process_uuid
    AND owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
RETURN AFTER;
"#;

#[cfg(any(test, feature = "test-utils"))]
const TEST_SET_INSPECTION_OWNER_WP: &str = r#"
UPDATE kernel_process_lifecycle SET owner_wp = $value
WHERE process_uuid = $process_uuid
    AND owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
RETURN AFTER;
"#;

#[cfg(any(test, feature = "test-utils"))]
const TEST_SET_INSPECTION_SANDBOX_ADAPTER_ID: &str = r#"
UPDATE kernel_process_lifecycle SET sandbox_adapter_id = $value
WHERE process_uuid = $process_uuid
    AND owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
RETURN AFTER;
"#;

#[cfg(any(test, feature = "test-utils"))]
const TEST_REPLACE_INSPECTION_RUNTIME_OWNER: &str = r#"
UPDATE kernel_process_lifecycle SET
    owner_runtime_instance_id = $replacement_runtime_instance_id,
    owner_host_scope_id = $replacement_host_scope_id,
    owner_lease_schema_id = $replacement_lease_schema_id,
    owner_lease_protocol = $replacement_lease_protocol,
    owner_lease_address = $replacement_lease_address,
    owner_lease_port = $replacement_lease_port
WHERE process_uuid = $process_uuid
    AND owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
RETURN AFTER;
"#;

#[cfg(any(test, feature = "test-utils"))]
const TEST_CLEAR_INSPECTION_RUNTIME_OWNER_HOST_SCOPE: &str = r#"
UPDATE kernel_process_lifecycle SET owner_host_scope_id = NONE
WHERE process_uuid = $process_uuid
    AND owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
RETURN AFTER;
"#;

#[cfg(any(test, feature = "test-utils"))]
const TEST_DELETE_INSPECTION_LIFECYCLE: &str = r#"
DELETE kernel_process_lifecycle
WHERE process_uuid = $process_uuid
    AND owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
RETURN BEFORE;
"#;

#[cfg(any(test, feature = "test-utils"))]
const TEST_DELETE_INSPECTION_RECEIPT: &str = r#"
DELETE $event_record
WHERE owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
RETURN BEFORE;
"#;

#[cfg(any(test, feature = "test-utils"))]
const TEST_MOVE_INSPECTION_RECEIPT_SCOPE: &str = r#"
UPDATE $event_record SET
    owner_account_id = $foreign_owner_account_id,
    actor_principal_id = $foreign_actor_principal_id,
    authenticated_session_id = $foreign_authenticated_session_id,
    access_space_id = $foreign_access_space_id,
    workspace_id = $foreign_workspace_id,
    payload.metadata_jsonb.owner_account_id = $foreign_owner_account_id,
    payload.metadata_jsonb.actor_principal_id = $foreign_actor_principal_id,
    payload.metadata_jsonb.authenticated_session_id = $foreign_authenticated_session_id,
    payload.metadata_jsonb.access_space_id = $foreign_access_space_id,
    payload.metadata_jsonb.workspace_id = $foreign_workspace_id
WHERE owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
RETURN AFTER;
"#;

#[cfg(any(test, feature = "test-utils"))]
const TEST_DUPLICATE_INSPECTION_IDENTITY: &str = r#"
REMOVE INDEX pk_kernel_process_lifecycle ON TABLE kernel_process_lifecycle;
LET $source = SELECT * OMIT id FROM ONLY $source_record
    WHERE owner_account_id = $owner_account_id
        AND actor_principal_id = $actor_principal_id
        AND authenticated_session_id = $authenticated_session_id
        AND access_space_id = $access_space_id
        AND workspace_id = $workspace_id;
CREATE $duplicate_record CONTENT $source RETURN AFTER;
"#;

#[cfg(any(test, feature = "test-utils"))]
const TEST_CLEAR_OWNER_ACCOUNT_ID: &str = r#"
UPDATE kernel_process_lifecycle SET owner_account_id = NONE
WHERE process_uuid = $process_uuid
    AND owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
RETURN AFTER;
"#;

#[cfg(any(test, feature = "test-utils"))]
const TEST_CLEAR_ACTOR_PRINCIPAL_ID: &str = r#"
UPDATE kernel_process_lifecycle SET actor_principal_id = NONE
WHERE process_uuid = $process_uuid
    AND owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
RETURN AFTER;
"#;

#[cfg(any(test, feature = "test-utils"))]
const TEST_CLEAR_AUTHENTICATED_SESSION_ID: &str = r#"
UPDATE kernel_process_lifecycle SET authenticated_session_id = NONE
WHERE process_uuid = $process_uuid
    AND owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
RETURN AFTER;
"#;

#[cfg(any(test, feature = "test-utils"))]
const TEST_CLEAR_ACCESS_SPACE_ID: &str = r#"
UPDATE kernel_process_lifecycle SET access_space_id = NONE
WHERE process_uuid = $process_uuid
    AND owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
RETURN AFTER;
"#;

#[cfg(any(test, feature = "test-utils"))]
const TEST_CLEAR_WORKSPACE_ID: &str = r#"
UPDATE kernel_process_lifecycle SET workspace_id = NONE
WHERE process_uuid = $process_uuid
    AND owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
RETURN AFTER;
"#;

const APPLY_EVENT_BATCH: &str = r#"
BEGIN TRANSACTION;
FOR $item IN $items {
    LET $existing = SELECT * FROM ONLY $item.record
        WHERE owner_account_id = $item.incoming.owner_account_id
            AND actor_principal_id = $item.incoming.actor_principal_id
            AND authenticated_session_id = $item.incoming.authenticated_session_id
            AND access_space_id = $item.incoming.access_space_id
            AND workspace_id = $item.incoming.workspace_id;
    LET $same_identity = $existing != NONE
        AND $existing.process_uuid = $item.incoming.process_uuid
        AND $existing.os_pid = $item.incoming.os_pid
        AND $existing.parent_session_id = $item.incoming.parent_session_id
        AND $existing.parent_process_id = $item.incoming.parent_process_id
        AND $existing.sandbox_adapter_id = $item.incoming.sandbox_adapter_id
        AND $existing.sandbox_internal_id = $item.incoming.sandbox_internal_id
        AND $existing.engine_kind = $item.incoming.engine_kind
        AND $existing.started_at = $item.incoming.started_at
        AND $existing.model_artifact_sha256 = $item.incoming.model_artifact_sha256
        AND $existing.work_profile_id = $item.incoming.work_profile_id
        AND $existing.owner_role = $item.incoming.owner_role
        AND $existing.owner_wp = $item.incoming.owner_wp
        AND $existing.role_id = $item.incoming.role_id
        AND $existing.wp_id = $item.incoming.wp_id
        AND $existing.mt_id = $item.incoming.mt_id
        AND $existing.owner_runtime_instance_id = $item.incoming.owner_runtime_instance_id
        AND $existing.owner_host_scope_id = $item.incoming.owner_host_scope_id
        AND $existing.owner_lease_schema_id = $item.incoming.owner_lease_schema_id
        AND $existing.owner_lease_protocol = $item.incoming.owner_lease_protocol
        AND $existing.owner_lease_address = $item.incoming.owner_lease_address
        AND $existing.owner_lease_port = $item.incoming.owner_lease_port
        AND $existing.sandbox_capabilities_snapshot = $item.incoming.sandbox_capabilities_snapshot;
    IF $item.is_start {
        IF $existing != NONE {
            IF $same_identity = false OR $existing.stopped_at != NONE {
                THROW $item.identity_conflict_marker;
            };
        } ELSE {
            LET $prior_event = SELECT * FROM ONLY $item.event_record
                WHERE owner_account_id = $item.incoming.owner_account_id
                    AND actor_principal_id = $item.incoming.actor_principal_id
                    AND authenticated_session_id = $item.incoming.authenticated_session_id
                    AND access_space_id = $item.incoming.access_space_id
                    AND workspace_id = $item.incoming.workspace_id
                    AND payload.metadata_jsonb.owner_account_id = $item.incoming.owner_account_id
                    AND payload.metadata_jsonb.actor_principal_id = $item.incoming.actor_principal_id
                    AND payload.metadata_jsonb.authenticated_session_id = $item.incoming.authenticated_session_id
                    AND payload.metadata_jsonb.access_space_id = $item.incoming.access_space_id
                    AND payload.metadata_jsonb.workspace_id = $item.incoming.workspace_id;
            IF $prior_event != NONE { THROW $item.verification_mismatch_marker; };
            LET $event = CREATE $item.event_record CONTENT {
                event_id: $item.event_id,
                event_version: 'hsk.process_ownership@1',
                kernel_task_run_id: $item.task_run_id,
                session_run_id: $item.session_run_id,
                aggregate_type: 'process_ownership',
                aggregate_id: <string>$item.incoming.process_uuid,
                idempotency_key: $item.idempotency_key,
                event_type: $item.event_type,
                actor_kind: 'kernel_role',
                actor_id: $item.actor_id,
                causation_id: NONE,
                correlation_id: NONE,
                payload_hash: $item.payload_hash,
                source_component: 'process_ledger',
                payload: $item.event_payload,
                owner_account_id: $item.incoming.owner_account_id,
                actor_principal_id: $item.incoming.actor_principal_id,
                authenticated_session_id: $item.incoming.authenticated_session_id,
                access_space_id: $item.incoming.access_space_id,
                workspace_id: $item.incoming.workspace_id,
                created_at: time::now()
            };
            CREATE $item.record CONTENT $item.incoming;
            UPDATE $item.record SET event_ledger_event_id = $item.event_record
                WHERE owner_account_id = $item.incoming.owner_account_id
                    AND actor_principal_id = $item.incoming.actor_principal_id
                    AND authenticated_session_id = $item.incoming.authenticated_session_id
                    AND access_space_id = $item.incoming.access_space_id
                    AND workspace_id = $item.incoming.workspace_id;
        };
    } ELSE {
        IF $existing = NONE OR $same_identity = false {
            THROW $item.identity_conflict_marker;
        };
        IF $existing.stopped_at != NONE {
            IF $existing.stopped_at != $item.incoming.stopped_at
                OR $existing.exit_code != $item.incoming.exit_code
                OR $existing.stop_reason != $item.incoming.stop_reason
            {
                THROW $item.identity_conflict_marker;
            };
        } ELSE {
            LET $prior_event = SELECT * FROM ONLY $item.event_record
                WHERE owner_account_id = $item.incoming.owner_account_id
                    AND actor_principal_id = $item.incoming.actor_principal_id
                    AND authenticated_session_id = $item.incoming.authenticated_session_id
                    AND access_space_id = $item.incoming.access_space_id
                    AND workspace_id = $item.incoming.workspace_id
                    AND payload.metadata_jsonb.owner_account_id = $item.incoming.owner_account_id
                    AND payload.metadata_jsonb.actor_principal_id = $item.incoming.actor_principal_id
                    AND payload.metadata_jsonb.authenticated_session_id = $item.incoming.authenticated_session_id
                    AND payload.metadata_jsonb.access_space_id = $item.incoming.access_space_id
                    AND payload.metadata_jsonb.workspace_id = $item.incoming.workspace_id;
            IF $prior_event != NONE { THROW $item.verification_mismatch_marker; };
            LET $event = CREATE $item.event_record CONTENT {
                event_id: $item.event_id,
                event_version: 'hsk.process_ownership@1',
                kernel_task_run_id: $item.task_run_id,
                session_run_id: $item.session_run_id,
                aggregate_type: 'process_ownership',
                aggregate_id: <string>$item.incoming.process_uuid,
                idempotency_key: $item.idempotency_key,
                event_type: $item.event_type,
                actor_kind: 'kernel_role',
                actor_id: $item.actor_id,
                causation_id: NONE,
                correlation_id: NONE,
                payload_hash: $item.payload_hash,
                source_component: 'process_ledger',
                payload: $item.event_payload,
                owner_account_id: $item.incoming.owner_account_id,
                actor_principal_id: $item.incoming.actor_principal_id,
                authenticated_session_id: $item.incoming.authenticated_session_id,
                access_space_id: $item.incoming.access_space_id,
                workspace_id: $item.incoming.workspace_id,
                created_at: time::now()
            };
            UPDATE $item.record SET stopped_at = $item.incoming.stopped_at,
                exit_code = $item.incoming.exit_code,
                stop_reason = $item.incoming.stop_reason,
                metadata = $item.incoming.metadata,
                event_ledger_event_id = $item.event_record
            WHERE owner_account_id = $item.incoming.owner_account_id
                AND actor_principal_id = $item.incoming.actor_principal_id
                AND authenticated_session_id = $item.incoming.authenticated_session_id
                AND access_space_id = $item.incoming.access_space_id
                AND workspace_id = $item.incoming.workspace_id;
        };
    };

    LET $final_event = SELECT * FROM ONLY $item.event_record
        WHERE owner_account_id = $item.incoming.owner_account_id
            AND actor_principal_id = $item.incoming.actor_principal_id
            AND authenticated_session_id = $item.incoming.authenticated_session_id
            AND access_space_id = $item.incoming.access_space_id
            AND workspace_id = $item.incoming.workspace_id
            AND payload.metadata_jsonb.owner_account_id = $item.incoming.owner_account_id
            AND payload.metadata_jsonb.actor_principal_id = $item.incoming.actor_principal_id
            AND payload.metadata_jsonb.authenticated_session_id = $item.incoming.authenticated_session_id
            AND payload.metadata_jsonb.access_space_id = $item.incoming.access_space_id
            AND payload.metadata_jsonb.workspace_id = $item.incoming.workspace_id;
    IF $final_event = NONE
        OR $final_event.id != $item.event_record
        OR $final_event.event_id != $item.event_id
        OR $final_event.event_version != 'hsk.process_ownership@1'
        OR $final_event.kernel_task_run_id != $item.task_run_id
        OR $final_event.session_run_id != $item.session_run_id
        OR $final_event.aggregate_type != 'process_ownership'
        OR $final_event.aggregate_id != <string>$item.incoming.process_uuid
        OR $final_event.idempotency_key != $item.idempotency_key
        OR $final_event.event_type != $item.event_type
        OR $final_event.actor_kind != 'kernel_role'
        OR $final_event.actor_id != $item.actor_id
        OR $final_event.causation_id != NONE
        OR $final_event.correlation_id != NONE
        OR $final_event.payload_hash != $item.payload_hash
        OR $final_event.source_component != 'process_ledger'
        OR $final_event.payload != $item.event_payload
        OR $final_event.owner_account_id != $item.incoming.owner_account_id
        OR $final_event.actor_principal_id != $item.incoming.actor_principal_id
        OR $final_event.authenticated_session_id != $item.incoming.authenticated_session_id
        OR $final_event.access_space_id != $item.incoming.access_space_id
        OR $final_event.workspace_id != $item.incoming.workspace_id
    {
        THROW $item.verification_mismatch_marker;
    };
};

FOR $expected IN $expected_final_rows {
    LET $final = SELECT * FROM ONLY $expected.record
        WHERE owner_account_id = $expected.incoming.owner_account_id
            AND actor_principal_id = $expected.incoming.actor_principal_id
            AND authenticated_session_id = $expected.incoming.authenticated_session_id
            AND access_space_id = $expected.incoming.access_space_id
            AND workspace_id = $expected.incoming.workspace_id;
    IF $final = NONE
        OR $final.process_uuid != $expected.incoming.process_uuid
        OR $final.os_pid != $expected.incoming.os_pid
        OR $final.parent_session_id != $expected.incoming.parent_session_id
        OR $final.parent_process_id != $expected.incoming.parent_process_id
        OR $final.sandbox_adapter_id != $expected.incoming.sandbox_adapter_id
        OR $final.sandbox_internal_id != $expected.incoming.sandbox_internal_id
        OR $final.engine_kind != $expected.incoming.engine_kind
        OR $final.started_at != $expected.incoming.started_at
        OR $final.stopped_at != $expected.incoming.stopped_at
        OR $final.exit_code != $expected.incoming.exit_code
        OR $final.stop_reason != $expected.incoming.stop_reason
        OR $final.model_artifact_sha256 != $expected.incoming.model_artifact_sha256
        OR $final.work_profile_id != $expected.incoming.work_profile_id
        OR $final.owner_role != $expected.incoming.owner_role
        OR $final.owner_wp != $expected.incoming.owner_wp
        OR $final.role_id != $expected.incoming.role_id
        OR $final.wp_id != $expected.incoming.wp_id
        OR $final.mt_id != $expected.incoming.mt_id
        OR $final.owner_runtime_instance_id != $expected.incoming.owner_runtime_instance_id
        OR $final.owner_host_scope_id != $expected.incoming.owner_host_scope_id
        OR $final.owner_lease_schema_id != $expected.incoming.owner_lease_schema_id
        OR $final.owner_lease_protocol != $expected.incoming.owner_lease_protocol
        OR $final.owner_lease_address != $expected.incoming.owner_lease_address
        OR $final.owner_lease_port != $expected.incoming.owner_lease_port
        OR $final.owner_account_id != $expected.incoming.owner_account_id
        OR $final.actor_principal_id != $expected.incoming.actor_principal_id
        OR $final.authenticated_session_id != $expected.incoming.authenticated_session_id
        OR $final.access_space_id != $expected.incoming.access_space_id
        OR $final.workspace_id != $expected.incoming.workspace_id
        OR $final.sandbox_capabilities_snapshot != $expected.incoming.sandbox_capabilities_snapshot
        OR $final.metadata != $expected.incoming.metadata
        OR $final.event_ledger_event_id != $expected.event_record
    {
        THROW $expected.verification_mismatch_marker;
    };
};
RETURN array::len($items);
COMMIT TRANSACTION;
"#;

const CLAIM_SESSION: &str = r#"
UPDATE kernel_process_lifecycle SET
    reclaim_claimant_uuid = $claimant_uuid,
    reclaim_kill_operation_uuid = IF reclaim_kill_operation_uuid = NONE { $kill_operation_uuid } ELSE { reclaim_kill_operation_uuid },
    reclaim_generation = IF reclaim_generation = NONE { 1 } ELSE { reclaim_generation + 1 },
    reclaim_claimed_at_unix_ms = $now_unix_ms,
    reclaim_lease_expires_at_unix_ms = $lease_expires_at_unix_ms,
    reclaim_state = IF stop_reason = 'kill_succeeded_pending_stop' { 'kill_succeeded_pending_stop' } ELSE { 'claimed' },
    stop_reason = IF stop_reason = 'kill_succeeded_pending_stop' { stop_reason } ELSE { 'reclaim_claimed' }
WHERE stopped_at = NONE
    AND sandbox_adapter_id != NONE
    AND parent_session_id = $session_id
    AND owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
    AND (stop_reason = NONE OR stop_reason = 'kill_succeeded_pending_stop'
        OR (stop_reason IN ['reclaim_claimed', 'reclaim_kill_in_progress']
            AND reclaim_lease_expires_at_unix_ms < $now_unix_ms))
RETURN AFTER;
"#;

const CLAIM_SESSION_PROCESS: &str = r#"
UPDATE kernel_process_lifecycle SET
    reclaim_claimant_uuid = $claimant_uuid,
    reclaim_kill_operation_uuid = IF reclaim_kill_operation_uuid = NONE { $kill_operation_uuid } ELSE { reclaim_kill_operation_uuid },
    reclaim_generation = IF reclaim_generation = NONE { 1 } ELSE { reclaim_generation + 1 },
    reclaim_claimed_at_unix_ms = $now_unix_ms,
    reclaim_lease_expires_at_unix_ms = $lease_expires_at_unix_ms,
    reclaim_state = IF stop_reason = 'kill_succeeded_pending_stop' { 'kill_succeeded_pending_stop' } ELSE { 'claimed' },
    stop_reason = IF stop_reason = 'kill_succeeded_pending_stop' { stop_reason } ELSE { 'reclaim_claimed' }
WHERE stopped_at = NONE AND sandbox_adapter_id != NONE
    AND parent_session_id = $session_id AND process_uuid = $process_uuid
    AND owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
    AND (stop_reason = NONE OR stop_reason = 'kill_succeeded_pending_stop'
        OR (stop_reason IN ['reclaim_claimed', 'reclaim_kill_in_progress']
            AND reclaim_lease_expires_at_unix_ms < $now_unix_ms))
RETURN AFTER;
"#;

const CLAIM_OWNED_PROCESS: &str = r#"
UPDATE kernel_process_lifecycle SET
    reclaim_claimant_uuid = $claimant_uuid,
    reclaim_kill_operation_uuid = IF reclaim_kill_operation_uuid = NONE { $kill_operation_uuid } ELSE { reclaim_kill_operation_uuid },
    reclaim_generation = IF reclaim_generation = NONE { 1 } ELSE { reclaim_generation + 1 },
    reclaim_claimed_at_unix_ms = $now_unix_ms,
    reclaim_lease_expires_at_unix_ms = $lease_expires_at_unix_ms,
    reclaim_state = IF stop_reason = 'kill_succeeded_pending_stop' { 'kill_succeeded_pending_stop' } ELSE { 'claimed' },
    stop_reason = IF stop_reason = 'kill_succeeded_pending_stop' { stop_reason } ELSE { 'reclaim_claimed' }
WHERE stopped_at = NONE AND sandbox_adapter_id != NONE
    AND process_uuid = $process_uuid
    AND owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
    AND owner_runtime_instance_id = $owner_runtime_instance_id
    AND (stop_reason = NONE OR stop_reason = 'kill_succeeded_pending_stop'
        OR (stop_reason IN ['reclaim_claimed', 'reclaim_kill_in_progress']
            AND reclaim_lease_expires_at_unix_ms < $now_unix_ms))
RETURN AFTER;
"#;

const CLAIM_FOREIGN_SESSION: &str = r#"
UPDATE kernel_process_lifecycle SET
    reclaim_claimant_uuid = $claimant_uuid,
    reclaim_kill_operation_uuid = IF reclaim_kill_operation_uuid = NONE { $kill_operation_uuid } ELSE { reclaim_kill_operation_uuid },
    reclaim_generation = IF reclaim_generation = NONE { 1 } ELSE { reclaim_generation + 1 },
    reclaim_claimed_at_unix_ms = $now_unix_ms,
    reclaim_lease_expires_at_unix_ms = $lease_expires_at_unix_ms,
    reclaim_state = IF stop_reason = 'kill_succeeded_pending_stop' { 'kill_succeeded_pending_stop' } ELSE { 'claimed' },
    stop_reason = IF stop_reason = 'kill_succeeded_pending_stop' { stop_reason } ELSE { 'reclaim_claimed' }
WHERE stopped_at = NONE AND sandbox_adapter_id != NONE
    AND parent_session_id = $session_id
    AND owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
    AND (owner_runtime_instance_id = NONE
        OR owner_runtime_instance_id != $excluded_owner_runtime_instance_id)
    AND process_uuid IN $authorized_process_uuids
    AND $authorized_process_uuids = (
        SELECT VALUE process_uuid FROM kernel_process_lifecycle
        WHERE stopped_at = NONE AND sandbox_adapter_id != NONE
            AND parent_session_id = $session_id
            AND owner_account_id = $owner_account_id
            AND actor_principal_id = $actor_principal_id
            AND authenticated_session_id = $authenticated_session_id
            AND access_space_id = $access_space_id
            AND workspace_id = $workspace_id
            AND (owner_runtime_instance_id = NONE
                OR owner_runtime_instance_id != $excluded_owner_runtime_instance_id)
        ORDER BY process_uuid
    )
    AND (stop_reason = NONE OR stop_reason = 'kill_succeeded_pending_stop'
        OR (stop_reason IN ['reclaim_claimed', 'reclaim_kill_in_progress']
            AND reclaim_lease_expires_at_unix_ms < $now_unix_ms))
RETURN AFTER;
"#;

const CLAIM_STALE_OWNED_SESSION: &str = r#"
UPDATE kernel_process_lifecycle SET
    reclaim_claimant_uuid = $claimant_uuid,
    reclaim_kill_operation_uuid = IF reclaim_kill_operation_uuid = NONE { $kill_operation_uuid } ELSE { reclaim_kill_operation_uuid },
    reclaim_generation = IF reclaim_generation = NONE { 1 } ELSE { reclaim_generation + 1 },
    reclaim_claimed_at_unix_ms = $now_unix_ms,
    reclaim_lease_expires_at_unix_ms = $lease_expires_at_unix_ms,
    reclaim_state = IF stop_reason = 'kill_succeeded_pending_stop' { 'kill_succeeded_pending_stop' } ELSE { 'claimed' },
    stop_reason = IF stop_reason = 'kill_succeeded_pending_stop' { stop_reason } ELSE { 'reclaim_claimed' }
WHERE stopped_at = NONE AND sandbox_adapter_id != NONE
    AND parent_session_id = $session_id
    AND owner_runtime_instance_id = $owner_runtime_instance_id
    AND owner_host_scope_id = $owner_host_scope_id
    AND owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
    AND process_uuid IN $authorized_process_uuids
    AND $authorized_process_uuids = (
        SELECT VALUE process_uuid FROM kernel_process_lifecycle
        WHERE stopped_at = NONE AND sandbox_adapter_id != NONE
            AND parent_session_id = $session_id
            AND owner_runtime_instance_id = $owner_runtime_instance_id
            AND owner_host_scope_id = $owner_host_scope_id
            AND owner_account_id = $owner_account_id
            AND actor_principal_id = $actor_principal_id
            AND authenticated_session_id = $authenticated_session_id
            AND access_space_id = $access_space_id
            AND workspace_id = $workspace_id
        ORDER BY process_uuid
    )
    AND (stop_reason = NONE OR stop_reason = 'kill_succeeded_pending_stop'
        OR (stop_reason IN ['reclaim_claimed', 'reclaim_kill_in_progress']
            AND reclaim_lease_expires_at_unix_ms < $now_unix_ms))
RETURN AFTER;
"#;

const RENEW_CLAIM: &str = r#"
UPDATE kernel_process_lifecycle SET
    reclaim_claimed_at_unix_ms = $now_unix_ms,
    reclaim_lease_expires_at_unix_ms = $lease_expires_at_unix_ms
WHERE process_uuid = $process_uuid AND stopped_at = NONE
    AND owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
    AND stop_reason IN ['reclaim_claimed', 'reclaim_kill_in_progress', 'kill_succeeded_pending_stop']
    AND reclaim_claimant_uuid = $claimant_uuid
    AND reclaim_kill_operation_uuid = $kill_operation_uuid
    AND reclaim_generation = $generation
RETURN AFTER;
"#;

const MARK_KILL_STARTED: &str = r#"
UPDATE kernel_process_lifecycle SET
    stop_reason = 'reclaim_kill_in_progress',
    reclaim_state = 'kill_in_progress',
    metadata.reclaim_last_kill_operation = {
        kill_operation_uuid: <string>$kill_operation_uuid,
        status: 'in_progress',
        recorded_at_unix_ms: $now_unix_ms
    }
WHERE process_uuid = $process_uuid AND stopped_at = NONE
    AND owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
    AND stop_reason = 'reclaim_claimed'
    AND reclaim_claimant_uuid = $claimant_uuid
    AND reclaim_kill_operation_uuid = $kill_operation_uuid
    AND reclaim_generation = $generation
RETURN AFTER;
"#;

const MARK_KILL_SUCCEEDED: &str = r#"
UPDATE kernel_process_lifecycle SET
    stop_reason = 'kill_succeeded_pending_stop',
    reclaim_state = 'kill_succeeded_pending_stop',
    metadata = $metadata
WHERE process_uuid = $process_uuid AND stopped_at = NONE
    AND owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
    AND stop_reason IN ['reclaim_claimed', 'reclaim_kill_in_progress']
    AND reclaim_claimant_uuid = $claimant_uuid
    AND reclaim_kill_operation_uuid = $kill_operation_uuid
    AND reclaim_generation = $generation
RETURN AFTER;
"#;

const RELEASE_CLAIM: &str = r#"
UPDATE kernel_process_lifecycle SET
    metadata.reclaim_last_kill_operation = {
        kill_operation_uuid: <string>$kill_operation_uuid,
        status: IF stop_reason = 'reclaim_kill_in_progress' { 'failed' } ELSE { 'not_started' },
        recorded_at_unix_ms: $now_unix_ms
    },
    metadata.reclaim_claim = NONE,
    stop_reason = NONE,
    reclaim_state = NONE,
    reclaim_claimant_uuid = NONE,
    reclaim_kill_operation_uuid = NONE,
    reclaim_claimed_at_unix_ms = NONE,
    reclaim_lease_expires_at_unix_ms = NONE
WHERE process_uuid = $process_uuid AND stopped_at = NONE
    AND owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
    AND stop_reason IN ['reclaim_claimed', 'reclaim_kill_in_progress']
    AND reclaim_claimant_uuid = $claimant_uuid
    AND reclaim_kill_operation_uuid = $kill_operation_uuid
    AND reclaim_generation = $generation
RETURN AFTER;
"#;

const RESOLVE_KILL_SUCCEEDED: &str = r#"
UPDATE kernel_process_lifecycle SET
    stop_reason = 'kill_succeeded_pending_stop',
    reclaim_state = 'kill_succeeded_pending_stop',
    metadata.reclaim_last_kill_operation.status = 'succeeded',
    metadata.reclaim_last_kill_operation.recorded_at_unix_ms = $now_unix_ms
WHERE process_uuid = $process_uuid AND stopped_at = NONE
    AND owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
    AND stop_reason = 'reclaim_kill_in_progress'
    AND reclaim_kill_operation_uuid = $kill_operation_uuid
RETURN AFTER;
"#;

const RESOLVE_KILL_RELEASED: &str = r#"
UPDATE kernel_process_lifecycle SET
    metadata.reclaim_last_kill_operation.status = $resolution_status,
    metadata.reclaim_last_kill_operation.recorded_at_unix_ms = $now_unix_ms,
    metadata.reclaim_claim = NONE,
    stop_reason = NONE,
    reclaim_state = NONE,
    reclaim_claimant_uuid = NONE,
    reclaim_kill_operation_uuid = NONE,
    reclaim_claimed_at_unix_ms = NONE,
    reclaim_lease_expires_at_unix_ms = NONE
WHERE process_uuid = $process_uuid AND stopped_at = NONE
    AND owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
    AND stop_reason = 'reclaim_kill_in_progress'
    AND reclaim_kill_operation_uuid = $kill_operation_uuid
RETURN AFTER;
"#;

const IN_PROGRESS_FOR_SESSION: &str = r#"
SELECT process_uuid, owner_account_id, actor_principal_id,
    authenticated_session_id, access_space_id, workspace_id,
    reclaim_kill_operation_uuid FROM kernel_process_lifecycle
WHERE parent_session_id = $session_id AND stopped_at = NONE
    AND owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
    AND (owner_runtime_instance_id = NONE
        OR owner_runtime_instance_id != $excluded_owner_runtime_instance_id)
    AND process_uuid IN $authorized_process_uuids
    AND $authorized_process_uuids = (
        SELECT VALUE process_uuid FROM kernel_process_lifecycle
        WHERE parent_session_id = $session_id AND stopped_at = NONE
            AND sandbox_adapter_id != NONE
            AND owner_account_id = $owner_account_id
            AND actor_principal_id = $actor_principal_id
            AND authenticated_session_id = $authenticated_session_id
            AND access_space_id = $access_space_id
            AND workspace_id = $workspace_id
            AND (owner_runtime_instance_id = NONE
                OR owner_runtime_instance_id != $excluded_owner_runtime_instance_id)
        ORDER BY process_uuid
    )
    AND stop_reason = 'reclaim_kill_in_progress'
ORDER BY started_at, process_uuid LIMIT $limit;
"#;

const IN_PROGRESS_FOR_STALE_OWNER: &str = r#"
SELECT process_uuid, owner_account_id, actor_principal_id,
    authenticated_session_id, access_space_id, workspace_id,
    reclaim_kill_operation_uuid FROM kernel_process_lifecycle
WHERE parent_session_id = $session_id AND stopped_at = NONE
    AND sandbox_adapter_id != NONE
    AND owner_runtime_instance_id = $owner_runtime_instance_id
    AND owner_host_scope_id = $owner_host_scope_id
    AND owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
    AND process_uuid IN $authorized_process_uuids
    AND $authorized_process_uuids = (
        SELECT VALUE process_uuid FROM kernel_process_lifecycle
        WHERE parent_session_id = $session_id AND stopped_at = NONE
            AND sandbox_adapter_id != NONE
            AND owner_runtime_instance_id = $owner_runtime_instance_id
            AND owner_host_scope_id = $owner_host_scope_id
            AND owner_account_id = $owner_account_id
            AND actor_principal_id = $actor_principal_id
            AND authenticated_session_id = $authenticated_session_id
            AND access_space_id = $access_space_id
            AND workspace_id = $workspace_id
        ORDER BY process_uuid
    )
    AND stop_reason = 'reclaim_kill_in_progress'
ORDER BY started_at, process_uuid LIMIT $limit;
"#;

const RESTART_LIFECYCLE_ROWS: &str = r#"
SELECT process_uuid, parent_session_id, sandbox_adapter_id, owner_runtime_instance_id,
    owner_host_scope_id, owner_lease_schema_id, owner_lease_protocol,
    owner_lease_address, owner_lease_port, owner_account_id,
    actor_principal_id, authenticated_session_id, access_space_id, workspace_id
FROM kernel_process_lifecycle
WHERE stopped_at = NONE
    AND ((parent_session_id != NONE AND sandbox_adapter_id != NONE)
        OR owner_runtime_instance_id != NONE)
    AND (($complete_scope = true
            AND owner_account_id != NONE
            AND actor_principal_id != NONE
            AND authenticated_session_id != NONE
            AND access_space_id != NONE
            AND workspace_id != NONE)
        OR ($complete_scope = false
            AND (owner_account_id = NONE
                OR actor_principal_id = NONE
                OR authenticated_session_id = NONE
                OR access_space_id = NONE
                OR workspace_id = NONE)))
ORDER BY parent_session_id, owner_runtime_instance_id;
"#;

const STALE_LIFECYCLE_ROWS: &str = r#"
SELECT process_uuid, parent_session_id, owner_runtime_instance_id,
    owner_host_scope_id, owner_account_id, actor_principal_id,
    authenticated_session_id, access_space_id, workspace_id
FROM kernel_process_lifecycle
WHERE stopped_at = NONE AND sandbox_adapter_id != NONE
    AND parent_session_id != NONE
    AND (($complete_scope = true
            AND owner_account_id != NONE
            AND actor_principal_id != NONE
            AND authenticated_session_id != NONE
            AND access_space_id != NONE
            AND workspace_id != NONE)
        OR ($complete_scope = false
            AND (owner_account_id = NONE
                OR actor_principal_id = NONE
                OR authenticated_session_id = NONE
                OR access_space_id = NONE
                OR workspace_id = NONE)))
ORDER BY parent_session_id, process_uuid;
"#;

const MODEL_LANE_AUTHORITY_ROWS_FOR_SCOPE: &str = r#"
SELECT record_json, owner_account_id, actor_principal_id,
    authenticated_session_id, access_space_id, workspace_id
FROM model_lane_authority
WHERE record_kind = 'lane'
    AND owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
ORDER BY event_seq DESC;
"#;

#[async_trait]
impl ProcessLedgerStore for SurrealProcessLedgerStore {
    async fn preflight(&self) -> Result<(), ProcessLedgerError> {
        bootstrap_process_ledger_schema(&self.storage)
            .await
            .map_err(|error| ProcessLedgerError::Store(error.to_string()))
    }

    async fn write_batch(&self, events: Vec<LedgerEvent>) -> Result<(), ProcessLedgerError> {
        if events.is_empty() {
            return Ok(());
        }

        let mut items = Vec::with_capacity(events.len());
        let mut expected_final_rows: Vec<ExpectedLifecycleRow> = Vec::new();
        for (index, event) in events.iter().enumerate() {
            let kind = event.kind();
            let process_uuid = event.process_uuid();
            let incoming = LifecycleRow::try_from(event)?;
            let mut payload = event.sampled_payload();
            payload
                .as_object_mut()
                .ok_or_else(|| {
                    ProcessLedgerError::Event(
                        "ProcessLedger EventLedger payload must be an object".to_owned(),
                    )
                })?
                .insert(
                    "runtime_owner".to_owned(),
                    process_runtime_owner_payload(event)?,
                );
            let payload_bytes = serde_json::to_vec(&payload)
                .map_err(|error| ProcessLedgerError::Event(error.to_string()))?;
            let record = RecordId::new(PROCESS_LEDGER_TABLE_NAME, process_uuid.to_string());
            let event_id = format!(
                "process-lifecycle-{process_uuid}-{}",
                kind.as_str().to_lowercase()
            );
            let event_record = RecordId::new("kernel_event_ledger", event_id.clone());
            let verification_mismatch_marker = format!(
                "PROCESS_LEDGER_VERIFICATION_MISMATCH:{index}:{process_uuid}:{}",
                kind.as_str()
            );
            let item = BatchApplyItem {
                record: record.clone(),
                incoming: incoming.clone(),
                is_start: kind == LedgerEventKind::Start,
                event_record: event_record.clone(),
                event_id,
                event_type: format!("PROCESS_{}", kind.as_str()),
                idempotency_key: format!("process-lifecycle:{process_uuid}:{}", kind.as_str()),
                task_run_id: event
                    .parent_session_id()
                    .unwrap_or("process-ledger")
                    .to_owned(),
                session_run_id: event
                    .parent_session_id()
                    .unwrap_or("process-ledger")
                    .to_owned(),
                actor_id: event.owner_role().to_owned(),
                payload_hash: format!("{:x}", Sha256::digest(payload_bytes)),
                event_payload: payload,
                identity_conflict_marker: format!(
                    "PROCESS_LEDGER_IDENTITY_CONFLICT:{index}:{process_uuid}:{}",
                    kind.as_str()
                ),
                verification_mismatch_marker: verification_mismatch_marker.clone(),
            };

            if let Some(expected) = expected_final_rows
                .iter_mut()
                .find(|expected| expected.incoming.process_uuid == process_uuid)
            {
                expected.incoming = incoming;
                expected.event_record = event_record;
                expected.verification_mismatch_marker = verification_mismatch_marker;
            } else {
                expected_final_rows.push(ExpectedLifecycleRow {
                    record,
                    incoming,
                    event_record,
                    verification_mismatch_marker,
                });
            }
            items.push(item);
        }

        let expected_count = i64::try_from(items.len()).map_err(|_| {
            ProcessLedgerError::Store("Surreal ProcessLedger batch is too large".to_owned())
        })?;
        let bindings = BatchApplyBindings {
            items,
            expected_final_rows,
        };
        let result = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values_at::<i64, _>(APPLY_EVENT_BATCH, bindings, 3)
                        .await
                })
            })
            .await;
        let counts = match result {
            Ok(counts) => counts,
            Err(error) => {
                let message = error.to_string();
                for (index, event) in events.iter().enumerate() {
                    let marker = format!(
                        "PROCESS_LEDGER_IDENTITY_CONFLICT:{index}:{}:{}",
                        event.process_uuid(),
                        event.kind().as_str()
                    );
                    if message.contains(&marker) {
                        return Err(match event {
                            LedgerEvent::Start(start) => {
                                ProcessLedgerError::StartIdentityConflict {
                                    process_uuid: start.process_uuid,
                                    conflicting_start: Box::new(start.clone()),
                                }
                            }
                            LedgerEvent::Stop(stop) => ProcessLedgerError::StopIdentityConflict {
                                process_uuid: stop.process_uuid,
                                conflicting_stop: Box::new(stop.clone()),
                            },
                        });
                    }
                }
                return Err(ProcessLedgerError::Store(message));
            }
        };
        if counts.as_slice() != [expected_count] {
            return Err(ProcessLedgerError::Store(format!(
                "invalid Surreal ProcessLedger batch acknowledgement: {counts:?}"
            )));
        }
        Ok(())
    }
}

#[async_trait]
impl ReclaimProcessStore for SurrealProcessLedgerStore {
    async fn active_processes_for_session(
        &self,
        resource_scope: &ReclaimResourceScope,
        session_id: &str,
    ) -> Result<Vec<ReclaimableProcess>, ProcessLedgerError> {
        self.claim_rows(
            CLAIM_SESSION,
            Self::claim_bindings(
                resource_scope,
                session_id,
                Uuid::nil(),
                Uuid::nil(),
                "",
                Uuid::nil(),
                Vec::new(),
            ),
        )
        .await
    }

    async fn active_process_for_session(
        &self,
        resource_scope: &ReclaimResourceScope,
        session_id: &str,
        process_uuid: Uuid,
    ) -> Result<Option<ReclaimableProcess>, ProcessLedgerError> {
        Ok(self
            .claim_rows(
                CLAIM_SESSION_PROCESS,
                Self::claim_bindings(
                    resource_scope,
                    session_id,
                    process_uuid,
                    Uuid::nil(),
                    "",
                    Uuid::nil(),
                    Vec::new(),
                ),
            )
            .await?
            .into_iter()
            .next())
    }

    async fn active_owned_process(
        &self,
        resource_scope: &ReclaimResourceScope,
        process_uuid: Uuid,
        owner_runtime_instance_id: Uuid,
    ) -> Result<Option<ReclaimableProcess>, ProcessLedgerError> {
        Ok(self
            .claim_rows(
                CLAIM_OWNED_PROCESS,
                Self::claim_bindings(
                    resource_scope,
                    "",
                    process_uuid,
                    owner_runtime_instance_id,
                    "",
                    Uuid::nil(),
                    Vec::new(),
                ),
            )
            .await?
            .into_iter()
            .next())
    }

    async fn active_foreign_owner_processes_for_session(
        &self,
        resource_scope: &ReclaimResourceScope,
        session_id: &str,
        excluded_owner_runtime_instance_id: Uuid,
        authorized_process_uuids: &[Uuid],
    ) -> Result<Vec<ReclaimableProcess>, ProcessLedgerError> {
        self.claim_rows(
            CLAIM_FOREIGN_SESSION,
            Self::claim_bindings(
                resource_scope,
                session_id,
                Uuid::nil(),
                Uuid::nil(),
                "",
                excluded_owner_runtime_instance_id,
                authorized_process_uuids.to_vec(),
            ),
        )
        .await
    }

    async fn active_stale_owned_processes_for_session(
        &self,
        resource_scope: &ReclaimResourceScope,
        session_id: &str,
        owner_runtime_instance_id: Uuid,
        owner_host_scope_id: &str,
        authorized_process_uuids: &[Uuid],
    ) -> Result<Vec<ReclaimableProcess>, ProcessLedgerError> {
        self.claim_rows(
            CLAIM_STALE_OWNED_SESSION,
            Self::claim_bindings(
                resource_scope,
                session_id,
                Uuid::nil(),
                owner_runtime_instance_id,
                owner_host_scope_id,
                Uuid::nil(),
                authorized_process_uuids.to_vec(),
            ),
        )
        .await
    }

    async fn renew_reclaim_claim(
        &self,
        process_uuid: Uuid,
        claim: &ReclaimClaim,
    ) -> Result<ReclaimClaim, ProcessLedgerError> {
        let row = self
            .fenced_update(
                RENEW_CLAIM,
                Self::fence_bindings(process_uuid, claim, Value::Null, "")?,
                format!("reclaim claim ownership lost while renewing process {process_uuid}"),
            )
            .await?;
        Ok(ReclaimClaim {
            resource_scope: claim.resource_scope.clone(),
            claimant_uuid: claim.claimant_uuid,
            kill_operation_uuid: claim.kill_operation_uuid,
            generation: claim.generation,
            claimed_at_unix_ms: row.reclaim_claimed_at_unix_ms.ok_or_else(|| {
                ProcessLedgerError::Store(
                    "renewed Surreal reclaim claim has no claim time".to_owned(),
                )
            })?,
            lease_expires_at_unix_ms: row.reclaim_lease_expires_at_unix_ms.ok_or_else(|| {
                ProcessLedgerError::Store(
                    "renewed Surreal reclaim claim has no lease expiry".to_owned(),
                )
            })?,
        })
    }

    async fn mark_reclaim_kill_started(
        &self,
        process_uuid: Uuid,
        claim: &ReclaimClaim,
    ) -> Result<(), ProcessLedgerError> {
        self.fenced_update(
            MARK_KILL_STARTED,
            Self::fence_bindings(process_uuid, claim, Value::Null, "")?,
            format!("reclaim kill-start fence lost ownership for process_uuid {process_uuid}"),
        )
        .await?;
        Ok(())
    }

    async fn mark_reclaim_kill_succeeded(
        &self,
        stop: &ProcessStop,
        claim: &ReclaimClaim,
    ) -> Result<(), ProcessLedgerError> {
        self.fenced_update(
            MARK_KILL_SUCCEEDED,
            Self::fence_bindings(
                stop.process_uuid,
                claim,
                stop.metadata_jsonb.clone(),
                "succeeded",
            )?,
            format!(
                "reclaim claim ownership lost before pending STOP for process {}",
                stop.process_uuid
            ),
        )
        .await?;
        Ok(())
    }

    async fn release_reclaim_claim(
        &self,
        process_uuid: Uuid,
        claim: &ReclaimClaim,
    ) -> Result<(), ProcessLedgerError> {
        self.fenced_update(
            RELEASE_CLAIM,
            Self::fence_bindings(process_uuid, claim, Value::Null, "")?,
            format!("failed to release open reclaim claim for process {process_uuid}"),
        )
        .await?;
        Ok(())
    }

    async fn resolve_reclaim_kill_operation(
        &self,
        resource_scope: &ReclaimResourceScope,
        process_uuid: Uuid,
        kill_operation_uuid: Uuid,
        status: ReclaimKillOperationStatus,
    ) -> Result<(), ProcessLedgerError> {
        let resolution_status = match status {
            ReclaimKillOperationStatus::Succeeded => "succeeded",
            ReclaimKillOperationStatus::Failed => "failed",
            ReclaimKillOperationStatus::NotStarted => "not_started",
            ReclaimKillOperationStatus::InProgress | ReclaimKillOperationStatus::Unknown => {
                return Err(ProcessLedgerError::InvalidConfig(
                    "non-terminal kill-operation evidence must remain open and cannot mutate recovery state".to_owned(),
                ));
            }
        };
        let placeholder = ReclaimClaim {
            resource_scope: resource_scope.clone(),
            claimant_uuid: Uuid::nil(),
            kill_operation_uuid,
            generation: 0,
            claimed_at_unix_ms: 0,
            lease_expires_at_unix_ms: 0,
        };
        let statement = if status == ReclaimKillOperationStatus::Succeeded {
            RESOLVE_KILL_SUCCEEDED
        } else {
            RESOLVE_KILL_RELEASED
        };
        self.fenced_update(
            statement,
            Self::fence_bindings(process_uuid, &placeholder, Value::Null, resolution_status)?,
            format!(
                "reclaim kill-operation resolution did not match process_uuid {process_uuid} operation {kill_operation_uuid}"
            ),
        ).await?;
        Ok(())
    }

    async fn in_progress_kill_operations_for_session(
        &self,
        resource_scope: &ReclaimResourceScope,
        session_id: &str,
        excluded_owner_runtime_instance_id: Uuid,
        authorized_process_uuids: &[Uuid],
        limit: usize,
    ) -> Result<Vec<ReclaimKillOperationCandidate>, ProcessLedgerError> {
        self.recovery_rows(
            IN_PROGRESS_FOR_SESSION,
            resource_scope,
            session_id,
            excluded_owner_runtime_instance_id,
            Uuid::nil(),
            "",
            authorized_process_uuids.to_vec(),
            limit,
        )
        .await
    }

    async fn in_progress_kill_operations_for_stale_owner(
        &self,
        resource_scope: &ReclaimResourceScope,
        session_id: &str,
        owner_runtime_instance_id: Uuid,
        owner_host_scope_id: &str,
        authorized_process_uuids: &[Uuid],
        limit: usize,
    ) -> Result<Vec<ReclaimKillOperationCandidate>, ProcessLedgerError> {
        self.recovery_rows(
            IN_PROGRESS_FOR_STALE_OWNER,
            resource_scope,
            session_id,
            Uuid::nil(),
            owner_runtime_instance_id,
            owner_host_scope_id,
            authorized_process_uuids.to_vec(),
            limit,
        )
        .await
    }
}

impl TryFrom<&LedgerEvent> for LifecycleRow {
    type Error = ProcessLedgerError;

    fn try_from(event: &LedgerEvent) -> Result<Self, Self::Error> {
        let (runtime, stopped_at, exit_code, stop_reason, metadata) = match event {
            LedgerEvent::Start(start) => (
                &start.runtime_owner,
                None,
                None,
                None,
                &start.metadata_jsonb,
            ),
            LedgerEvent::Stop(stop) => (
                &stop.runtime_owner,
                Some(stop.stopped_at),
                stop.exit_code.map(i64::from),
                stop.stop_reason.clone(),
                &stop.metadata_jsonb,
            ),
        };
        let scope = exact_scope(metadata)?;
        let owner = runtime.as_ref();
        if let Some(owner) = owner {
            validate_runtime_owner_for_write(owner)?;
        }
        Ok(Self {
            process_uuid: event.process_uuid(),
            os_pid: event.os_pid().map(i64::from),
            parent_session_id: event.parent_session_id().map(str::to_owned),
            parent_process_id: event.parent_process_id(),
            sandbox_adapter_id: event.sandbox_adapter_id().map(str::to_owned),
            sandbox_internal_id: event.sandbox_internal_id().map(str::to_owned),
            engine_kind: event.engine_kind().as_str().to_owned(),
            started_at: event.started_at(),
            stopped_at,
            exit_code,
            stop_reason,
            model_artifact_sha256: event.model_artifact_sha256().map(str::to_owned),
            work_profile_id: event.work_profile_id().map(str::to_owned),
            owner_role: event.owner_role().to_owned(),
            owner_wp: event.owner_wp().map(str::to_owned),
            role_id: event.role_id().map(str::to_owned),
            wp_id: event.wp_id().map(str::to_owned),
            mt_id: event.mt_id().map(str::to_owned),
            owner_runtime_instance_id: owner.map(|value| value.runtime_instance_id),
            owner_host_scope_id: owner.map(|value| value.host_scope_id.clone()),
            owner_lease_schema_id: owner.map(|value| value.lease_schema_id.clone()),
            owner_lease_protocol: owner.map(|value| value.lease_protocol.clone()),
            owner_lease_address: owner.map(|value| value.lease_address.clone()),
            owner_lease_port: owner.map(|value| i64::from(value.lease_port)),
            owner_account_id: Some(scope[0].clone()),
            actor_principal_id: Some(scope[1].clone()),
            authenticated_session_id: Some(scope[2].clone()),
            access_space_id: Some(scope[3].clone()),
            workspace_id: Some(scope[4].clone()),
            sandbox_capabilities_snapshot: event.sandbox_capabilities_snapshot().clone(),
            metadata: metadata.clone(),
        })
    }
}

fn process_runtime_owner_payload(event: &LedgerEvent) -> Result<Value, ProcessLedgerError> {
    let runtime_owner = match event {
        LedgerEvent::Start(start) => start.runtime_owner.as_ref(),
        LedgerEvent::Stop(stop) => stop.runtime_owner.as_ref(),
    };
    serde_json::to_value(runtime_owner)
        .map_err(|error| ProcessLedgerError::Event(error.to_string()))
}

fn validate_runtime_owner_for_write(
    runtime_owner: &ProcessRuntimeOwner,
) -> Result<(), ProcessLedgerError> {
    for (name, value) in [
        ("host_scope_id", runtime_owner.host_scope_id.as_str()),
        ("lease_schema_id", runtime_owner.lease_schema_id.as_str()),
        ("lease_protocol", runtime_owner.lease_protocol.as_str()),
        ("lease_address", runtime_owner.lease_address.as_str()),
    ] {
        if value.is_empty() || value.trim() != value || value.chars().any(char::is_control) {
            return Err(ProcessLedgerError::Store(format!(
                "runtime-owner {name} is malformed"
            )));
        }
    }
    if runtime_owner.lease_port == 0 {
        return Err(ProcessLedgerError::Store(
            "runtime-owner lease_port must not be zero".to_owned(),
        ));
    }
    Ok(())
}

fn exact_scope(metadata: &Value) -> Result<[String; 5], ProcessLedgerError> {
    let names = [
        "owner_account_id",
        "actor_principal_id",
        "authenticated_session_id",
        "access_space_id",
        "workspace_id",
    ];
    let values = names.map(|name| {
        metadata
            .get(name)
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .map(str::to_owned)
            .ok_or_else(|| {
                ProcessLedgerError::Store(
                    "all five non-empty ResourceScope fields are required for ProcessLedger writes"
                        .to_owned(),
                )
            })
    });
    let [owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id] =
        values;
    let values = [
        owner_account_id?,
        actor_principal_id?,
        authenticated_session_id?,
        access_space_id?,
        workspace_id?,
    ];
    for value in values.iter().take(4) {
        Uuid::parse_str(value)
            .map_err(|_| ProcessLedgerError::Store("ResourceScope UUID is invalid".to_owned()))?;
    }
    Ok(values)
}

fn validate_inspection_scope(scope: &ReclaimResourceScope) -> Result<(), ProcessLedgerError> {
    let parsed = ReclaimResourceScope::try_from_stored(
        &scope.account_uuid.to_string(),
        &scope.actor_uuid.to_string(),
        &scope.session_uuid.to_string(),
        &scope.workspace_id,
        &scope.access_space_uuid.to_string(),
    )?;
    if parsed != *scope {
        return Err(inspection_error(
            "input ResourceScope normalization mismatch",
        ));
    }
    Ok(())
}

fn inspection_error(detail: &str) -> ProcessLedgerError {
    ProcessLedgerError::Store(format!(
        "ProcessLedger ownership inspection failed closed: {detail}"
    ))
}

fn inspection_scope(
    row: &InspectionLifecycleRow,
) -> Result<ReclaimResourceScope, ProcessLedgerError> {
    let (
        Some(owner_account_id),
        Some(actor_principal_id),
        Some(authenticated_session_id),
        Some(access_space_id),
        Some(workspace_id),
    ) = (
        row.owner_account_id.as_deref(),
        row.actor_principal_id.as_deref(),
        row.authenticated_session_id.as_deref(),
        row.access_space_id.as_deref(),
        row.workspace_id.as_deref(),
    )
    else {
        return Err(inspection_error("stored ResourceScope is incomplete"));
    };
    ReclaimResourceScope::try_from_stored(
        owner_account_id,
        actor_principal_id,
        authenticated_session_id,
        workspace_id,
        access_space_id,
    )
    .map_err(|_| inspection_error("stored ResourceScope is malformed"))
}

fn inspection_runtime_owner(
    row: &InspectionLifecycleRow,
) -> Result<Option<ProcessRuntimeOwner>, ProcessLedgerError> {
    match (
        row.owner_runtime_instance_id,
        row.owner_host_scope_id.as_ref(),
        row.owner_lease_schema_id.as_ref(),
        row.owner_lease_protocol.as_ref(),
        row.owner_lease_address.as_ref(),
        row.owner_lease_port,
    ) {
        (None, None, None, None, None, None) => Ok(None),
        (
            Some(runtime_instance_id),
            Some(host_scope_id),
            Some(lease_schema_id),
            Some(lease_protocol),
            Some(lease_address),
            Some(lease_port),
        ) => {
            let lease_port = u16::try_from(lease_port)
                .ok()
                .filter(|port| *port != 0)
                .ok_or_else(|| inspection_error("runtime-owner lease port is malformed"))?;
            let host_scope_id =
                checked_inspection_identity("runtime-owner host scope", host_scope_id)?.to_owned();
            let lease_schema_id =
                checked_inspection_identity("runtime-owner lease schema", lease_schema_id)?
                    .to_owned();
            let lease_protocol =
                checked_inspection_identity("runtime-owner lease protocol", lease_protocol)?
                    .to_owned();
            let lease_address =
                checked_inspection_identity("runtime-owner lease address", lease_address)?
                    .to_owned();
            Ok(Some(ProcessRuntimeOwner {
                runtime_instance_id,
                host_scope_id,
                lease_schema_id,
                lease_protocol,
                lease_address,
                lease_port,
            }))
        }
        _ => Err(inspection_error("runtime-owner identity is incomplete")),
    }
}

fn inspection_lifecycle_identity(
    row: &InspectionLifecycleRow,
) -> Result<(ProcessEngineKind, String, Option<String>, Option<String>), ProcessLedgerError> {
    let engine_kind = ProcessEngineKind::try_from(row.engine_kind.as_str())
        .map_err(|_| inspection_error("stored engine kind is malformed"))?;
    let owner_role = checked_inspection_identity("owner role", &row.owner_role)?.to_owned();
    let owner_wp = row
        .owner_wp
        .as_deref()
        .map(|value| checked_inspection_identity("owner WP", value).map(str::to_owned))
        .transpose()?;
    let sandbox_adapter_id = row
        .sandbox_adapter_id
        .as_deref()
        .map(|value| checked_inspection_identity("sandbox adapter id", value).map(str::to_owned))
        .transpose()?;
    Ok((engine_kind, owner_role, owner_wp, sandbox_adapter_id))
}

fn checked_inspection_identity<'a>(
    name: &str,
    value: &'a str,
) -> Result<&'a str, ProcessLedgerError> {
    if value.is_empty() || value.trim() != value || value.chars().any(char::is_control) {
        return Err(inspection_error(&format!("stored {name} is malformed")));
    }
    Ok(value)
}

fn verify_inspection_receipt(
    row: &InspectionLifecycleRow,
    receipt: &InspectionEventRow,
    kind: LedgerEventKind,
    expected_event_id: &str,
) -> Result<(), ProcessLedgerError> {
    let expected_record = RecordId::new("kernel_event_ledger", expected_event_id.to_owned());
    let expected_event_type = format!("PROCESS_{}", kind.as_str());
    let expected_idempotency_key =
        format!("process-lifecycle:{}:{}", row.process_uuid, kind.as_str());
    let payload = receipt
        .payload
        .as_object()
        .ok_or_else(|| inspection_error("EventLedger payload is malformed"))?;
    let payload_hash = serde_json::to_vec(&receipt.payload)
        .map(|bytes| format!("{:x}", Sha256::digest(bytes)))
        .map_err(|_| inspection_error("EventLedger payload cannot be hashed"))?;
    if receipt.id != expected_record
        || receipt.event_id != expected_event_id
        || receipt.event_version != "hsk.process_ownership@1"
        || receipt.aggregate_type != "process_ownership"
        || receipt.aggregate_id != row.process_uuid.to_string()
        || receipt.idempotency_key != expected_idempotency_key
        || receipt.event_type != expected_event_type
        || receipt.payload_hash != payload_hash
        || receipt.source_component != "process_ledger"
    {
        return Err(inspection_error("EventLedger receipt envelope mismatch"));
    }

    let scope = inspection_scope(row)?;
    let expected_owner_account_id = scope.account_uuid.to_string();
    let expected_actor_principal_id = scope.actor_uuid.to_string();
    let expected_authenticated_session_id = scope.session_uuid.to_string();
    let expected_access_space_id = scope.access_space_uuid.to_string();
    if receipt.owner_account_id.as_deref() != Some(expected_owner_account_id.as_str())
        || receipt.actor_principal_id.as_deref() != Some(expected_actor_principal_id.as_str())
        || receipt.authenticated_session_id.as_deref()
            != Some(expected_authenticated_session_id.as_str())
        || receipt.access_space_id.as_deref() != Some(expected_access_space_id.as_str())
        || receipt.workspace_id.as_deref() != Some(scope.workspace_id.as_str())
    {
        return Err(inspection_error(
            "EventLedger top-level ResourceScope mismatch",
        ));
    }

    let expected_process_uuid = Value::String(row.process_uuid.to_string());
    let expected_kind = Value::String(kind.as_str().to_owned());
    let expected_os_pid = serde_json::to_value(
        row.os_pid
            .map(|value| u32::try_from(value))
            .transpose()
            .map_err(|_| inspection_error("stored os_pid is outside u32 range"))?,
    )
    .map_err(|_| inspection_error("stored os_pid cannot be projected"))?;
    let expected_artifact = serde_json::to_value(&row.model_artifact_sha256)
        .map_err(|_| inspection_error("stored artifact identity cannot be projected"))?;
    let (engine_kind, owner_role, owner_wp, sandbox_adapter_id) =
        inspection_lifecycle_identity(row)?;
    let expected_engine_kind = Value::String(engine_kind.as_str().to_owned());
    let expected_owner_role = Value::String(owner_role);
    let expected_owner_wp = serde_json::to_value(owner_wp)
        .map_err(|_| inspection_error("stored owner WP cannot be projected"))?;
    let expected_sandbox_adapter_id = serde_json::to_value(sandbox_adapter_id)
        .map_err(|_| inspection_error("stored sandbox adapter id cannot be projected"))?;
    let expected_runtime_owner = serde_json::to_value(inspection_runtime_owner(row)?)
        .map_err(|_| inspection_error("stored runtime owner cannot be projected"))?;
    let expected_started_at = serde_json::to_value(row.started_at)
        .map_err(|_| inspection_error("stored START time cannot be projected"))?;
    if payload.get("event_kind") != Some(&expected_kind)
        || payload.get("process_uuid") != Some(&expected_process_uuid)
        || payload.get("os_pid") != Some(&expected_os_pid)
        || payload.get("model_artifact_sha256") != Some(&expected_artifact)
        || payload.get("engine_kind") != Some(&expected_engine_kind)
        || payload.get("owner_role") != Some(&expected_owner_role)
        || payload.get("owner_wp") != Some(&expected_owner_wp)
        || payload.get("sandbox_adapter_id") != Some(&expected_sandbox_adapter_id)
        || payload.get("runtime_owner") != Some(&expected_runtime_owner)
        || payload.get("started_at") != Some(&expected_started_at)
    {
        return Err(inspection_error("EventLedger lifecycle identity mismatch"));
    }

    if kind == LedgerEventKind::Stop {
        let expected_stopped_at = serde_json::to_value(row.stopped_at)
            .map_err(|_| inspection_error("stored STOP time cannot be projected"))?;
        let expected_exit_code = serde_json::to_value(row.exit_code)
            .map_err(|_| inspection_error("stored exit code cannot be projected"))?;
        let expected_stop_reason = serde_json::to_value(&row.stop_reason)
            .map_err(|_| inspection_error("stored stop reason cannot be projected"))?;
        if payload.get("stopped_at") != Some(&expected_stopped_at)
            || payload.get("exit_code") != Some(&expected_exit_code)
            || payload.get("stop_reason") != Some(&expected_stop_reason)
        {
            return Err(inspection_error("EventLedger STOP projection mismatch"));
        }
    }

    let metadata = payload
        .get("metadata_jsonb")
        .and_then(Value::as_object)
        .ok_or_else(|| inspection_error("EventLedger ResourceScope is missing"))?;
    let expected_scope = [
        ("owner_account_id", scope.account_uuid.to_string()),
        ("actor_principal_id", scope.actor_uuid.to_string()),
        ("authenticated_session_id", scope.session_uuid.to_string()),
        ("access_space_id", scope.access_space_uuid.to_string()),
        ("workspace_id", scope.workspace_id),
    ];
    if expected_scope
        .iter()
        .any(|(name, value)| metadata.get(*name).and_then(Value::as_str) != Some(value.as_str()))
    {
        return Err(inspection_error("EventLedger ResourceScope mismatch"));
    }
    Ok(())
}

fn inspection_from_row(
    row: InspectionLifecycleRow,
    event_ledger_event_id: RecordId,
) -> Result<ProcessOwnershipInspection, ProcessLedgerError> {
    let os_pid = row
        .os_pid
        .map(u32::try_from)
        .transpose()
        .map_err(|_| inspection_error("stored os_pid is outside u32 range"))?;
    let runtime_owner = inspection_runtime_owner(&row)?;
    let resource_scope = inspection_scope(&row)?;
    let (engine_kind, owner_role, owner_wp, sandbox_adapter_id) =
        inspection_lifecycle_identity(&row)?;
    let lifecycle_state = if row.stopped_at.is_some() {
        LedgerEventKind::Stop
    } else {
        LedgerEventKind::Start
    };
    Ok(ProcessOwnershipInspection {
        process_uuid: row.process_uuid,
        os_pid,
        model_artifact_sha256: row.model_artifact_sha256,
        engine_kind,
        owner_role,
        owner_wp,
        sandbox_adapter_id,
        lifecycle_state,
        started_at: row.started_at,
        stopped_at: row.stopped_at,
        exit_code: row.exit_code,
        stop_reason: row.stop_reason,
        runtime_owner,
        resource_scope,
        event_ledger_event_id,
    })
}

fn runtime_owner_from_reclaim_row(
    row: &ReclaimRow,
) -> Result<Option<ProcessRuntimeOwner>, ProcessLedgerError> {
    match (
        row.owner_runtime_instance_id,
        row.owner_host_scope_id.as_ref(),
        row.owner_lease_schema_id.as_ref(),
        row.owner_lease_protocol.as_ref(),
        row.owner_lease_address.as_ref(),
        row.owner_lease_port,
    ) {
        (None, None, None, None, None, None) => Ok(None),
        (
            Some(runtime_instance_id),
            Some(host_scope_id),
            Some(lease_schema_id),
            Some(lease_protocol),
            Some(lease_address),
            Some(lease_port),
        ) => {
            let lease_port = u16::try_from(lease_port).map_err(|_| {
                ProcessLedgerError::Store("owner_lease_port is outside 1..=65535".to_owned())
            })?;
            if lease_port == 0 {
                return Err(ProcessLedgerError::Store(
                    "owner_lease_port must not be zero".to_owned(),
                ));
            }
            Ok(Some(ProcessRuntimeOwner {
                runtime_instance_id,
                host_scope_id: host_scope_id.clone(),
                lease_schema_id: lease_schema_id.clone(),
                lease_protocol: lease_protocol.clone(),
                lease_address: lease_address.clone(),
                lease_port,
            }))
        }
        _ => Err(ProcessLedgerError::Store(
            "partial typed runtime-owner identity in Surreal reclaim row".to_owned(),
        )),
    }
}

impl TryFrom<ReclaimRow> for ReclaimableProcess {
    type Error = ProcessLedgerError;

    fn try_from(mut row: ReclaimRow) -> Result<Self, Self::Error> {
        let resource_scope = ReclaimResourceScope::try_from_stored(
            &row.owner_account_id,
            &row.actor_principal_id,
            &row.authenticated_session_id,
            &row.workspace_id,
            &row.access_space_id,
        )?;
        let generation = row.reclaim_generation.ok_or_else(|| {
            ProcessLedgerError::Store("claimed Surreal lifecycle row has no generation".to_owned())
        })?;
        let reclaim_claim = ReclaimClaim {
            resource_scope: resource_scope.clone(),
            claimant_uuid: row.reclaim_claimant_uuid.ok_or_else(|| {
                ProcessLedgerError::Store(
                    "claimed Surreal lifecycle row has no claimant".to_owned(),
                )
            })?,
            kill_operation_uuid: row.reclaim_kill_operation_uuid.ok_or_else(|| {
                ProcessLedgerError::Store(
                    "claimed Surreal lifecycle row has no kill operation".to_owned(),
                )
            })?,
            generation: u64::try_from(generation).map_err(|_| {
                ProcessLedgerError::Store("Surreal reclaim generation is negative".to_owned())
            })?,
            claimed_at_unix_ms: row.reclaim_claimed_at_unix_ms.ok_or_else(|| {
                ProcessLedgerError::Store(
                    "claimed Surreal lifecycle row has no claim time".to_owned(),
                )
            })?,
            lease_expires_at_unix_ms: row.reclaim_lease_expires_at_unix_ms.ok_or_else(|| {
                ProcessLedgerError::Store(
                    "claimed Surreal lifecycle row has no lease expiry".to_owned(),
                )
            })?,
        };
        let claim_json = serde_json::to_value(&reclaim_claim)
            .map_err(|error| ProcessLedgerError::Store(error.to_string()))?;
        row.metadata
            .as_object_mut()
            .ok_or_else(|| {
                ProcessLedgerError::Store(
                    "Surreal reclaim metadata is not a JSON object".to_owned(),
                )
            })?
            .insert("reclaim_claim".to_owned(), claim_json);
        let runtime_owner = runtime_owner_from_reclaim_row(&row)?;
        Ok(Self {
            resource_scope,
            process_uuid: row.process_uuid,
            os_pid: row.os_pid.map(u32::try_from).transpose().map_err(|_| {
                ProcessLedgerError::Store("Surreal reclaim os_pid is outside u32".to_owned())
            })?,
            parent_session_id: row.parent_session_id,
            parent_process_id: row.parent_process_id,
            sandbox_adapter_id: row.sandbox_adapter_id,
            sandbox_internal_id: row.sandbox_internal_id,
            engine_kind: ProcessEngineKind::try_from(row.engine_kind.as_str())
                .map_err(ProcessLedgerError::Store)?,
            started_at: row.started_at,
            model_artifact_sha256: row.model_artifact_sha256,
            work_profile_id: row.work_profile_id,
            owner_role: row.owner_role,
            owner_wp: row.owner_wp,
            role_id: row.role_id,
            wp_id: row.wp_id,
            mt_id: row.mt_id,
            runtime_owner,
            sandbox_capabilities_snapshot: row.sandbox_capabilities_snapshot,
            metadata_jsonb: row.metadata,
            reclaim_claim,
            kill_succeeded_pending_stop: row.stop_reason.as_deref()
                == Some("kill_succeeded_pending_stop"),
        })
    }
}

trait LedgerEventFields {
    fn os_pid(&self) -> Option<u32>;
    fn parent_process_id(&self) -> Option<Uuid>;
    fn sandbox_adapter_id(&self) -> Option<&str>;
    fn sandbox_internal_id(&self) -> Option<&str>;
    fn engine_kind(&self) -> crate::process_ledger::ProcessEngineKind;
    fn started_at(&self) -> DateTime<Utc>;
    fn model_artifact_sha256(&self) -> Option<&str>;
    fn work_profile_id(&self) -> Option<&str>;
    fn owner_role(&self) -> &str;
    fn owner_wp(&self) -> Option<&str>;
    fn role_id(&self) -> Option<&str>;
    fn wp_id(&self) -> Option<&str>;
    fn mt_id(&self) -> Option<&str>;
    fn sandbox_capabilities_snapshot(&self) -> &Value;
}

macro_rules! event_field {
    ($self:ident, $field:ident) => {
        match $self {
            LedgerEvent::Start(v) => &v.$field,
            LedgerEvent::Stop(v) => &v.$field,
        }
    };
}
impl LedgerEventFields for LedgerEvent {
    fn os_pid(&self) -> Option<u32> {
        *event_field!(self, os_pid)
    }
    fn parent_process_id(&self) -> Option<Uuid> {
        *event_field!(self, parent_process_id)
    }
    fn sandbox_adapter_id(&self) -> Option<&str> {
        event_field!(self, sandbox_adapter_id).as_deref()
    }
    fn sandbox_internal_id(&self) -> Option<&str> {
        event_field!(self, sandbox_internal_id).as_deref()
    }
    fn engine_kind(&self) -> crate::process_ledger::ProcessEngineKind {
        *event_field!(self, engine_kind)
    }
    fn started_at(&self) -> DateTime<Utc> {
        *event_field!(self, started_at)
    }
    fn model_artifact_sha256(&self) -> Option<&str> {
        event_field!(self, model_artifact_sha256).as_deref()
    }
    fn work_profile_id(&self) -> Option<&str> {
        event_field!(self, work_profile_id).as_deref()
    }
    fn owner_role(&self) -> &str {
        event_field!(self, owner_role)
    }
    fn owner_wp(&self) -> Option<&str> {
        event_field!(self, owner_wp).as_deref()
    }
    fn role_id(&self) -> Option<&str> {
        event_field!(self, role_id).as_deref()
    }
    fn wp_id(&self) -> Option<&str> {
        event_field!(self, wp_id).as_deref()
    }
    fn mt_id(&self) -> Option<&str> {
        event_field!(self, mt_id).as_deref()
    }
    fn sandbox_capabilities_snapshot(&self) -> &Value {
        event_field!(self, sandbox_capabilities_snapshot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process_ledger::ProcessStart;
    use crate::storage::surreal::{bootstrap_schema, SurrealStorageConfig};

    #[derive(Debug, SurrealValue)]
    struct RecordBinding {
        record: RecordId,
    }

    #[derive(Debug, SurrealValue)]
    struct TamperBindings {
        record: RecordId,
        owner_account_id: String,
        actor_principal_id: String,
        authenticated_session_id: String,
        access_space_id: String,
        workspace_id: String,
    }

    async fn open_test_store() -> (tempfile::TempDir, SurrealStorage, SurrealProcessLedgerStore) {
        let directory = tempfile::tempdir().expect("create ProcessLedger test directory");
        let storage = SurrealStorage::open(
            SurrealStorageConfig::with_path(directory.path().join("store"))
                .expect("configure ProcessLedger test store"),
        )
        .await
        .expect("open ProcessLedger test store");
        bootstrap_schema(&storage)
            .await
            .expect("bootstrap shared Surreal schema");
        let store = SurrealProcessLedgerStore::open(storage.clone())
            .await
            .expect("bootstrap ProcessLedger provider schema");
        (directory, storage, store)
    }

    fn exact_scope_metadata() -> Value {
        serde_json::json!({
            "owner_account_id": Uuid::now_v7().to_string(),
            "actor_principal_id": Uuid::now_v7().to_string(),
            "authenticated_session_id": Uuid::now_v7().to_string(),
            "access_space_id": Uuid::now_v7().to_string(),
            "workspace_id": format!("workspace-{}", Uuid::now_v7()),
        })
    }

    fn scoped_start(process_uuid: Uuid) -> ProcessStart {
        ProcessStart::new(
            ProcessEngineKind::MechanicalJob,
            "process-ledger-test",
            Some("WP-1".to_owned()),
        )
        .with_process_uuid(process_uuid)
        .with_os_pid(41001)
        .with_metadata_jsonb(exact_scope_metadata())
    }

    async fn record_exists(storage: &SurrealStorage, record: RecordId) -> bool {
        storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_first::<bool, _>(
                            "RETURN record::exists($record);",
                            RecordBinding { record },
                        )
                        .await
                })
            })
            .await
            .expect("read record existence")
            .expect("record existence query returns one value")
    }

    #[test]
    fn lifecycle_conversion_rejects_missing_all_five_scope_fields() {
        let event = LedgerEvent::Start(ProcessStart::new(
            ProcessEngineKind::MechanicalJob,
            "process-ledger-test",
            Some("WP-1".to_owned()),
        ));
        let error = LifecycleRow::try_from(&event)
            .expect_err("an unattributed ProcessLedger write must fail before storage access");
        assert!(error.to_string().contains(
            "all five non-empty ResourceScope fields are required for ProcessLedger writes"
        ));
    }

    #[test]
    fn reclaim_scope_rejects_missing_and_one_field_invalid() {
        let valid = Uuid::now_v7().to_string();
        assert!(ReclaimResourceScope::try_from_stored("", "", "", "", "").is_err());
        assert!(ReclaimResourceScope::try_from_stored(
            &valid,
            "not-a-uuid",
            &valid,
            "workspace-a",
            &valid,
        )
        .is_err());
    }

    #[test]
    fn every_surreal_reclaim_statement_binds_all_five_scope_fields() {
        const PREDICATES: [&str; 5] = [
            "owner_account_id = $owner_account_id",
            "actor_principal_id = $actor_principal_id",
            "authenticated_session_id = $authenticated_session_id",
            "access_space_id = $access_space_id",
            "workspace_id = $workspace_id",
        ];
        let statements = [
            CLAIM_SESSION,
            CLAIM_SESSION_PROCESS,
            CLAIM_OWNED_PROCESS,
            CLAIM_FOREIGN_SESSION,
            CLAIM_STALE_OWNED_SESSION,
            RENEW_CLAIM,
            MARK_KILL_STARTED,
            MARK_KILL_SUCCEEDED,
            RELEASE_CLAIM,
            RESOLVE_KILL_SUCCEEDED,
            RESOLVE_KILL_RELEASED,
            IN_PROGRESS_FOR_SESSION,
            IN_PROGRESS_FOR_STALE_OWNER,
        ];
        for statement in statements {
            for predicate in PREDICATES {
                assert!(
                    statement.contains(predicate),
                    "missing exact ResourceScope predicate `{predicate}`"
                );
            }
        }
        for statement in [
            CLAIM_FOREIGN_SESSION,
            CLAIM_STALE_OWNED_SESSION,
            IN_PROGRESS_FOR_SESSION,
            IN_PROGRESS_FOR_STALE_OWNER,
        ] {
            for predicate in PREDICATES {
                assert_eq!(
                    statement.matches(predicate).count(),
                    2,
                    "outer row and authorized-set subquery must both bind `{predicate}`"
                );
            }
        }
    }

    #[tokio::test]
    async fn second_event_identity_conflict_rolls_back_entire_surreal_batch() {
        let (_directory, storage, store) = open_test_store().await;
        let process_uuid = Uuid::now_v7();
        let start = scoped_start(process_uuid);
        let mut conflicting_start = start.clone();
        conflicting_start.os_pid = Some(41002);

        let error = store
            .write_batch(vec![
                LedgerEvent::Start(start),
                LedgerEvent::Start(conflicting_start),
            ])
            .await
            .expect_err("second identity conflict must reject the batch");
        assert!(matches!(
            error,
            ProcessLedgerError::StartIdentityConflict {
                process_uuid: rejected_uuid,
                ..
            } if rejected_uuid == process_uuid
        ));

        assert!(
            !record_exists(
                &storage,
                RecordId::new(PROCESS_LEDGER_TABLE_NAME, process_uuid.to_string())
            )
            .await,
            "a conflicting second event must roll back the first lifecycle row"
        );
        assert!(
            !record_exists(
                &storage,
                RecordId::new(
                    "kernel_event_ledger",
                    format!("process-lifecycle-{process_uuid}-start")
                )
            )
            .await,
            "a conflicting second event must roll back the first EventLedger row"
        );
        storage.shutdown().await.expect("shutdown test store");
    }

    #[tokio::test]
    async fn final_lifecycle_mismatch_fails_closed_before_commit() {
        let (_directory, storage, store) = open_test_store().await;
        let process_uuid = Uuid::now_v7();
        let start = scoped_start(process_uuid);
        store
            .write_batch(vec![LedgerEvent::Start(start.clone())])
            .await
            .expect("seed authoritative lifecycle");

        let scope = exact_scope(&start.metadata_jsonb).expect("test metadata has exact scope");
        let tampered_rows = storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .execute_returning(
                            "UPDATE $record SET metadata.verification_tamper = true WHERE owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id AND authenticated_session_id = $authenticated_session_id AND access_space_id = $access_space_id AND workspace_id = $workspace_id RETURN AFTER;",
                            TamperBindings {
                                record: RecordId::new(
                                    PROCESS_LEDGER_TABLE_NAME,
                                    process_uuid.to_string(),
                                ),
                                owner_account_id: scope[0].clone(),
                                actor_principal_id: scope[1].clone(),
                                authenticated_session_id: scope[2].clone(),
                                access_space_id: scope[3].clone(),
                                workspace_id: scope[4].clone(),
                            },
                        )
                        .await
                })
            })
            .await
            .expect("tamper lifecycle fixture");
        assert_eq!(
            tampered_rows, 1,
            "fixture tamper must affect one scoped row"
        );

        let error = store
            .write_batch(vec![LedgerEvent::Start(start)])
            .await
            .expect_err("final lifecycle mismatch must fail closed");
        assert!(
            error
                .to_string()
                .contains("PROCESS_LEDGER_VERIFICATION_MISMATCH:0"),
            "verification failure marker must survive transaction rollback: {error}"
        );
        storage.shutdown().await.expect("shutdown test store");
    }
}
