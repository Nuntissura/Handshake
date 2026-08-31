//! Dexterity model-lane persistence.
//!
//! Dexterity is the operator-facing name for the internal kernel that launches,
//! switches, and records local, cloud, CLI, human, subagent, and validator
//! lanes. The stable wire/schema names remain `ModelLaneRun`, `ModelLane`, and
//! `ModelLaneMessage`.

use std::sync::atomic::{AtomicI64, Ordering as AtomicOrdering};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    ops::Deref,
    sync::Arc,
};

use base64::Engine;
use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Row, Transaction};
use thiserror::Error;
use tokio::sync::OnceCell;
use uuid::Uuid;
use yrs::{
    updates::{decoder::Decode, encoder::Encode},
    Doc, ReadTxn, StateVector, Transact, Update,
};

use crate::kernel::{
    context_bundle::{canonical_json_bytes, ContextBundle},
    crdt::{
        actor_site::{derive_knowledge_site_id, KnowledgeActorIdV1},
        persistence::{
            validate_crdt_update_record, CrdtReplayMetadataV1, CrdtStorageAuthorityPosture,
            CrdtUpdateRecordV1, CRDT_UPDATE_RECORD_SCHEMA_ID,
        },
        snapshot::{
            validate_crdt_snapshot_record, CrdtSnapshotRecordV1, CRDT_SNAPSHOT_RECORD_SCHEMA_ID,
        },
        state_vector::KnowledgeStateVectorV1,
    },
    KernelActor, KernelEventType, NewKernelEvent,
};
use crate::model_runtime::ProviderKind;
use crate::storage::postgres::append_kernel_event_with_executor;
use crate::storage::surreal::{
    bootstrap_cloud_model_lane_schema, CloudModelLaneRecordKind, CloudModelLaneScope,
    CloudModelLaneStore, CloudModelLaneStoredRow, SurrealStorage, SurrealStorageError,
};
use crate::storage::{knowledge_crdt, StorageError};

use super::error::SwarmError;
use super::factory::LiveSession;
use super::ids::{ByokCloudProvider, SpawnRequest};
use super::resource_scope::{
    stored_resource_scope_from_row, AccessSpaceRef, AccountBoundAuthority, ActorPrincipalId,
    AuthenticatedSessionRef, ExactResourceScopeAttribution, OwnerAccountId, ResourceAccessContext,
    ResourceScope, ResourceScopeQuery, ScopeColumnValues, ScopeDenied, SystemScopeAuthority,
    WorkspaceScopeRef, RESOURCE_SCOPE_INSERT_COLUMNS, RESOURCE_SCOPE_SELECT_COLUMNS,
};

const SOURCE_COMPONENT: &str = "dexterity_model_lane";
const MAX_CONTEXT_BUNDLE_LOOM_REFS: usize = 64;
const MAX_CONTEXT_BUNDLE_MEMORY_PACK_REFS: usize = 16;
static CLOUD_EVENT_SEQUENCE: AtomicI64 = AtomicI64::new(0);

#[derive(Debug, Error)]
pub enum ModelLaneError {
    #[error("invalid model lane input: {0}")]
    InvalidInput(String),
    #[error("model lane authority denied: {0}")]
    AuthorityDenied(String),
    /// HBR-PRIV-002 default-deny. Carries only the stable denial reason code and
    /// no identifiers or row contents, so surfacing it can never become a
    /// metadata side channel for the resource that was withheld.
    #[error("model lane resource scope denied: {0}")]
    ScopeDenied(#[from] ScopeDenied),
    #[error("model lane idempotency conflict: {0}")]
    IdempotencyConflict(String),
    #[error("model lane ambiguous lookup: {0}")]
    AmbiguousLookup(String),
    #[error("model lane not found: {0}")]
    NotFound(String),
    #[error("model lane integrity violation: {0}")]
    IntegrityViolation(String),
    #[error("model lane storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("model lane database error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("model lane embedded SurrealDB error: {0}")]
    Surreal(#[from] SurrealStorageError),
    #[error("model lane json error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type ModelLaneResult<T> = Result<T, ModelLaneError>;

#[derive(Debug, Clone)]
pub struct ModelLaneStore {
    pool: ModelLaneRelationalStore,
    /// MT-006 cloud authority is owned exclusively by Handshake's embedded
    /// SurrealDB. The PostgreSQL pool remains only for unrelated legacy lanes.
    cloud_authority: Arc<OnceCell<CloudModelLaneStore>>,
    /// HBR-PRIV-001/002. Every write stamps this context onto the five scope
    /// columns migration 0363 added, and every enforced read filters on it — in
    /// SQL first, so a denied row never leaves PostgreSQL, and again in Rust
    /// after deserialization, because HBR-PRIV-002 says hiding a row in one
    /// layer is never sufficient.
    access: ResourceAccessContext,
}

#[derive(Clone)]
enum ModelLaneRelationalStore {
    Postgres(PgPool),
    #[cfg(feature = "test-utils")]
    DisabledForSurrealCloudProof,
}

impl std::fmt::Debug for ModelLaneRelationalStore {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Postgres(_) => formatter.write_str("Postgres(<pool>)"),
            #[cfg(feature = "test-utils")]
            Self::DisabledForSurrealCloudProof => {
                formatter.write_str("DisabledForSurrealCloudProof")
            }
        }
    }
}

impl ModelLaneRelationalStore {
    fn postgres_pool_if_available(&self) -> Option<PgPool> {
        match self {
            Self::Postgres(pool) => Some(pool.clone()),
            #[cfg(feature = "test-utils")]
            Self::DisabledForSurrealCloudProof => None,
        }
    }

    fn postgres_pool(&self) -> PgPool {
        match self {
            Self::Postgres(pool) => pool.clone(),
            #[cfg(feature = "test-utils")]
            Self::DisabledForSurrealCloudProof => {
                panic!("MT-006 Surreal-only cloud proof attempted relational persistence")
            }
        }
    }
}

impl Deref for ModelLaneRelationalStore {
    type Target = PgPool;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Postgres(pool) => pool,
            #[cfg(feature = "test-utils")]
            Self::DisabledForSurrealCloudProof => {
                panic!("MT-006 Surreal-only cloud proof attempted relational persistence")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DurableSessionCleanupReceipt {
    pub instance_id: String,
    pub lane_id: Option<String>,
    pub process_uuid: Uuid,
    pub terminal_event_id: Uuid,
    pub resource_evicted_event_id: Uuid,
    pub status: String,
    pub terminal_state: String,
    pub reason: String,
    pub exit_code: i32,
    pub last_error: Option<String>,
}

impl ModelLaneStore {
    /// Legacy constructor for call sites that predate account identity.
    ///
    /// It yields a store holding [`SystemScopeAuthority::legacy_unscoped_call_site`]:
    /// reads are NOT account-filtered and writes are stamped with a NULL
    /// `owner_account_id`. Those unattributed rows are unreadable by every
    /// account-scoped reader, so this constructor cannot be used to smuggle a
    /// row into someone's account — but it is a real read bypass and is the
    /// residual seam WP-KERNEL-006 MT-014/016 closes with PostgreSQL RLS.
    ///
    /// New code, and every HTTP boundary, MUST use [`Self::new_for_owner`] or
    /// [`Self::new_scoped`].
    pub fn new(pool: PgPool) -> Self {
        Self::new_with_access(
            pool,
            ResourceAccessContext::system(SystemScopeAuthority::legacy_unscoped_call_site()),
        )
    }

    /// Store bound to one owning account for reads and writes.
    pub fn new_scoped(pool: PgPool, scope: ResourceScope) -> Self {
        Self::new_with_access(pool, ResourceAccessContext::for_account(scope))
    }

    /// Read-only store bound to one owning account. Writes through this store
    /// are unattributed by construction, which is what a read boundary that
    /// carries an account but no actor Principal must be able to guarantee.
    pub fn new_for_owner(pool: PgPool, query: ResourceScopeQuery) -> Self {
        Self::new_with_access(pool, ResourceAccessContext::for_reader(query))
    }

    /// Store with an explicit, named cross-owner authority.
    pub fn new_system_authority(pool: PgPool, authority: SystemScopeAuthority) -> Self {
        Self::new_with_access(pool, ResourceAccessContext::system(authority))
    }

    pub fn new_with_access(pool: PgPool, access: ResourceAccessContext) -> Self {
        Self {
            pool: ModelLaneRelationalStore::Postgres(pool),
            cloud_authority: Arc::new(OnceCell::new()),
            access,
        }
    }

    #[cfg(feature = "test-utils")]
    pub async fn new_surreal_cloud_authority_only(
        access: ResourceAccessContext,
        storage: SurrealStorage,
    ) -> ModelLaneResult<Self> {
        bootstrap_cloud_model_lane_schema(&storage).await?;
        let cloud_authority = Arc::new(OnceCell::new());
        cloud_authority
            .set(CloudModelLaneStore::new(storage))
            .map_err(|_| {
                ModelLaneError::IntegrityViolation("cloud authority initialized twice".into())
            })?;
        Ok(Self {
            pool: ModelLaneRelationalStore::DisabledForSurrealCloudProof,
            cloud_authority,
            access,
        })
    }

    #[cfg(feature = "test-utils")]
    pub fn new_unscoped_cloud_authority_without_storage() -> Self {
        Self {
            pool: ModelLaneRelationalStore::DisabledForSurrealCloudProof,
            cloud_authority: Arc::new(OnceCell::new()),
            access: ResourceAccessContext::system(SystemScopeAuthority::legacy_unscoped_call_site()),
        }
    }

    async fn cloud_authority(&self) -> ModelLaneResult<&CloudModelLaneStore> {
        self.cloud_authority
            .get_or_try_init(|| async {
                let storage = SurrealStorage::open_default().await?;
                bootstrap_cloud_model_lane_schema(&storage).await?;
                Ok::<_, SurrealStorageError>(CloudModelLaneStore::new(storage))
            })
            .await
            .map_err(Into::into)
    }

    pub fn access(&self) -> &ResourceAccessContext {
        &self.access
    }

    /// The scope stamped onto rows written through this store, if any.
    /// Trusted write scope bound at store construction. Production hosts use
    /// this to derive projection attribution from the durable authority rather
    /// than accepting account identifiers from request or renderer payloads.
    pub fn write_scope(&self) -> Option<&ResourceScope> {
        self.access.write_scope()
    }

    fn scope_columns(&self) -> ScopeColumnValues<'_> {
        self.access.insert_columns()
    }

    /// Require an explicitly named cross-owner authority for an operation that
    /// is genuinely system-wide (restart recovery). An account-scoped store must
    /// not be able to reach it, or "recovery" becomes a disclosure route.
    fn require_system_authority(&self, operation: &str) -> ModelLaneResult<SystemScopeAuthority> {
        self.access.system_authority().ok_or_else(|| {
            ModelLaneError::AuthorityDenied(format!(
                "{operation} is a cross-owner system operation and requires an explicit SystemScopeAuthority"
            ))
        })
    }

    pub(crate) fn postgres_pool(&self) -> PgPool {
        self.pool.postgres_pool()
    }

    pub(crate) fn postgres_pool_if_available(&self) -> Option<PgPool> {
        self.pool.postgres_pool_if_available()
    }

    pub(crate) async fn record_session_cleanup_receipt(
        &self,
        instance_id: &str,
        lane_id: Option<&str>,
        process_uuid: Uuid,
        terminal_event_id: Uuid,
        resource_evicted_event_id: Uuid,
        status: &str,
        terminal_state: &str,
        reason: &str,
        exit_code: i32,
        last_error: Option<&str>,
    ) -> ModelLaneResult<()> {
        let record_json = serde_json::json!({
            "schema_id": "hsk.swarm_session_cleanup_receipt@1",
            "instance_id": instance_id,
            "lane_id": lane_id,
            "process_uuid": process_uuid,
            "terminal_event_id": terminal_event_id,
            "resource_evicted_event_id": resource_evicted_event_id,
            "status": status,
            "terminal_state": terminal_state,
            "reason": reason,
            "exit_code": exit_code,
            "last_error": last_error,
        });
        sqlx::query(
            r#"
            INSERT INTO swarm_session_cleanup_receipts (
                instance_id, revision, status, terminal_state, reason,
                exit_code, last_error, record_json, updated_at_unix_ms
            ) VALUES ($1, 1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (instance_id) DO UPDATE SET
                revision = swarm_session_cleanup_receipts.revision + 1,
                status = EXCLUDED.status,
                terminal_state = EXCLUDED.terminal_state,
                reason = EXCLUDED.reason,
                exit_code = EXCLUDED.exit_code,
                last_error = EXCLUDED.last_error,
                record_json = EXCLUDED.record_json,
                updated_at_unix_ms = EXCLUDED.updated_at_unix_ms
            "#,
        )
        .bind(instance_id)
        .bind(status)
        .bind(terminal_state)
        .bind(reason)
        .bind(exit_code)
        .bind(last_error)
        .bind(record_json)
        .bind(Utc::now().timestamp_millis())
        .execute(&*self.pool)
        .await?;
        Ok(())
    }

    /// Durable cleanup intents that survived a coordinator/runtime restart.
    /// This is deliberately system-authority-only: boot reconciliation is a
    /// cross-owner operation and must not become an account disclosure route.
    pub(crate) async fn pending_session_cleanup_receipts(
        &self,
    ) -> ModelLaneResult<Vec<DurableSessionCleanupReceipt>> {
        self.require_system_authority("pending swarm-session cleanup restart reconciliation")?;
        let rows = sqlx::query(
            r#"
            SELECT instance_id, status, terminal_state, reason, exit_code,
                   last_error, record_json
            FROM swarm_session_cleanup_receipts
            WHERE status <> 'completed'
            ORDER BY updated_at_unix_ms ASC, instance_id ASC
            "#,
        )
        .fetch_all(&*self.pool)
        .await?;
        rows.into_iter()
            .map(|row| {
                let record: Value = row.try_get("record_json")?;
                let required = |field: &str| {
                    record
                        .get(field)
                        .and_then(Value::as_str)
                        .filter(|value| !value.trim().is_empty())
                        .ok_or_else(|| {
                            ModelLaneError::IntegrityViolation(format!(
                                "pending cleanup receipt is missing {field}"
                            ))
                        })
                };
                Ok(DurableSessionCleanupReceipt {
                    instance_id: row.try_get("instance_id")?,
                    lane_id: record
                        .get("lane_id")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                    process_uuid: Uuid::parse_str(required("process_uuid")?).map_err(|error| {
                        ModelLaneError::IntegrityViolation(format!(
                            "pending cleanup receipt process_uuid is invalid: {error}"
                        ))
                    })?,
                    terminal_event_id: Uuid::parse_str(required("terminal_event_id")?).map_err(
                        |error| {
                            ModelLaneError::IntegrityViolation(format!(
                                "pending cleanup receipt terminal_event_id is invalid: {error}"
                            ))
                        },
                    )?,
                    resource_evicted_event_id: Uuid::parse_str(required(
                        "resource_evicted_event_id",
                    )?)
                    .map_err(|error| {
                        ModelLaneError::IntegrityViolation(format!(
                            "pending cleanup receipt resource_evicted_event_id is invalid: {error}"
                        ))
                    })?,
                    status: row.try_get("status")?,
                    terminal_state: row.try_get("terminal_state")?,
                    reason: row.try_get("reason")?,
                    exit_code: row.try_get("exit_code")?,
                    last_error: row.try_get("last_error")?,
                })
            })
            .collect()
    }

    pub(crate) async fn cleanup_process_is_durably_closed(
        &self,
        process_uuid: Uuid,
    ) -> ModelLaneResult<bool> {
        self.require_system_authority("swarm-session cleanup process closure verification")?;
        let closed: Option<bool> = sqlx::query_scalar(
            r#"
            SELECT stopped_at IS NOT NULL
               AND exit_code IS NOT NULL
               AND stop_reason IS NOT NULL
            FROM kernel_process_lifecycle
            WHERE process_uuid = $1
            "#,
        )
        .bind(process_uuid)
        .fetch_optional(&*self.pool)
        .await?;
        Ok(closed.unwrap_or(false))
    }

    pub(crate) async fn session_cleanup_completed(
        &self,
        instance_id: &str,
        terminal_state: &str,
        reason: &str,
    ) -> ModelLaneResult<bool> {
        let completed: Option<bool> = sqlx::query_scalar(
            r#"
            SELECT status = 'completed'
               AND terminal_state = $2
               AND reason = $3
            FROM swarm_session_cleanup_receipts
            WHERE instance_id = $1
            "#,
        )
        .bind(instance_id)
        .bind(terminal_state)
        .bind(reason)
        .fetch_optional(&*self.pool)
        .await?;
        Ok(completed.unwrap_or(false))
    }

    pub async fn record_successful_launch(
        &self,
        request: &SpawnRequest,
        live: &LiveSession,
    ) -> ModelLaneResult<(ModelLaneRunRecord, ModelLaneRecord)> {
        let records = build_successful_launch_records(request, live)?;
        self.record_prepared_launch(records).await
    }

    pub async fn record_prepared_launch(
        &self,
        records: (NewModelLaneRun, NewModelLane),
    ) -> ModelLaneResult<(ModelLaneRunRecord, ModelLaneRecord)> {
        validate_run(&records.0)?;
        validate_lane(&records.1)?;
        validate_prepared_launch_pair(&records.0, &records.1)?;
        let cloud_check = is_cloud_lane(&records.1)
            .then(|| cloud_launch_check_from_records(&records.0, &records.1));
        if let Some(check) = cloud_check {
            require_exact_cloud_launch_scope(&self.access)?;
            if let Err(error) = self.ensure_cloud_launch_authority_surreal(&check).await {
                return self
                    .deny_cloud_launch(
                        check,
                        &format!("final cloud launch insertion fence denied: {error}"),
                    )
                    .await;
            }
            let stored_run = self.record_cloud_run_surreal(records.0).await?;
            let stored_lane = self.record_cloud_lane_surreal(records.1).await?;
            return Ok((stored_run, stored_lane));
        }
        let mut tx = self.pool.begin().await?;
        let stored_run =
            record_or_extend_run_tx(&mut tx, records.0, &records.1, &self.access).await?;
        let stored_lane = record_lane_tx(&mut tx, records.1, self.scope_columns()).await?;
        tx.commit().await?;
        Ok((stored_run, stored_lane))
    }

    #[cfg(feature = "test-utils")]
    pub async fn test_record_prepared_launch_holding_receipt_fence(
        &self,
        records: (NewModelLaneRun, NewModelLane),
        entered: std::sync::Arc<tokio::sync::Notify>,
        release: std::sync::Arc<tokio::sync::Notify>,
    ) -> ModelLaneResult<(ModelLaneRunRecord, ModelLaneRecord)> {
        validate_run(&records.0)?;
        validate_lane(&records.1)?;
        validate_prepared_launch_pair(&records.0, &records.1)?;
        require_exact_cloud_launch_scope(&self.access)?;
        let check = cloud_launch_check_from_records(&records.0, &records.1);
        self.ensure_cloud_launch_authority_surreal(&check).await?;
        entered.notify_one();
        release.notified().await;
        // Recheck after the pause: a concurrent revocation that won during the
        // test fence must prevent the cloud lane write.
        self.ensure_cloud_launch_authority_surreal(&check).await?;
        let stored_run = self.record_cloud_run_surreal(records.0).await?;
        let stored_lane = self.record_cloud_lane_surreal(records.1).await?;
        Ok((stored_run, stored_lane))
    }

    pub async fn record_normalized_launch(
        &self,
        launch: DexterityNormalizedLaunch,
    ) -> ModelLaneResult<(ModelLaneRunRecord, ModelLaneRecord)> {
        self.record_prepared_launch(launch.to_records()?).await
    }

    pub async fn record_run(&self, input: NewModelLaneRun) -> ModelLaneResult<ModelLaneRunRecord> {
        validate_run(&input)?;
        let mut tx = self.pool.begin().await?;
        let stored = record_run_tx(&mut tx, input, self.scope_columns()).await?;
        tx.commit().await?;
        Ok(stored)
    }

    pub async fn record_lane(&self, input: NewModelLane) -> ModelLaneResult<ModelLaneRecord> {
        validate_lane(&input)?;
        let cloud_check = is_cloud_lane(&input).then(|| cloud_launch_check_from_lane(&input));
        if let Some(check) = cloud_check {
            require_exact_cloud_launch_scope(&self.access)?;
            if let Err(error) = self.ensure_cloud_launch_authority_surreal(&check).await {
                return self
                    .deny_cloud_launch(
                        check,
                        &format!("final cloud lane insertion fence denied: {error}"),
                    )
                    .await;
            }
            return self.record_cloud_lane_surreal(input).await;
        }
        let mut tx = self.pool.begin().await?;
        let stored = record_lane_tx(&mut tx, input, self.scope_columns()).await?;
        tx.commit().await?;
        Ok(stored)
    }

    pub async fn record_message(
        &self,
        input: NewModelLaneMessage,
    ) -> ModelLaneResult<ModelLaneMessageRecord> {
        validate_message(&input)?;
        let mut tx = self.pool.begin().await?;
        let stored = Self::record_message_tx(&mut tx, input, self.scope_columns()).await?;
        tx.commit().await?;
        Ok(stored)
    }

    #[cfg(feature = "test-utils")]
    pub async fn test_record_message_holding_crdt_authority_lock(
        &self,
        input: NewModelLaneMessage,
        entered: std::sync::Arc<tokio::sync::Notify>,
        release: std::sync::Arc<tokio::sync::Notify>,
    ) -> ModelLaneResult<ModelLaneMessageRecord> {
        validate_message(&input)?;
        let mut tx = self.pool.begin().await?;
        let stored = Self::record_message_tx_with_crdt_pause(
            &mut tx,
            input,
            Some((entered, release)),
            self.scope_columns(),
        )
        .await?;
        tx.commit().await?;
        Ok(stored)
    }

    /// Commit a ModelLane payload binding and its message in one PostgreSQL
    /// transaction.  The message is checked first, so a terminal lane rejects
    /// before an ArtifactStore/EventLedger binding can be left behind.  A
    /// later binding failure rolls the message transaction back as well.
    pub async fn record_message_with_payload_binding(
        &self,
        message: NewModelLaneMessage,
        binding: NewModelLaneContextBundleArtifactBinding,
    ) -> ModelLaneResult<ModelLaneMessageRecord> {
        let mut tx = self.pool.begin().await?;
        let stored_message = Self::record_message_with_payload_binding_tx(
            &mut tx,
            message,
            binding,
            self.scope_columns(),
        )
        .await?;
        tx.commit().await?;
        Ok(stored_message)
    }

    pub(crate) async fn record_message_with_payload_binding_tx(
        tx: &mut Transaction<'_, Postgres>,
        message: NewModelLaneMessage,
        binding: NewModelLaneContextBundleArtifactBinding,
        scope: ScopeColumnValues<'_>,
    ) -> ModelLaneResult<ModelLaneMessageRecord> {
        validate_message(&message)?;
        validate_context_bundle_artifact_binding(&binding)?;
        validate_message_payload_binding_pair(&message, &binding)?;
        let stored_message = Self::record_message_tx(tx, message, scope).await?;
        Self::record_context_bundle_artifact_binding_tx(tx, binding, scope).await?;
        Ok(stored_message)
    }

    pub(crate) async fn record_message_with_validation_tx(
        tx: &mut Transaction<'_, Postgres>,
        message: NewModelLaneMessage,
        scope: ScopeColumnValues<'_>,
    ) -> ModelLaneResult<ModelLaneMessageRecord> {
        validate_message(&message)?;
        Self::record_message_tx(tx, message, scope).await
    }

    pub(crate) async fn record_context_bundle_artifact_binding_with_validation_tx(
        tx: &mut Transaction<'_, Postgres>,
        binding: NewModelLaneContextBundleArtifactBinding,
        scope: ScopeColumnValues<'_>,
    ) -> ModelLaneResult<ModelLaneContextBundleArtifactBindingRecord> {
        validate_context_bundle_artifact_binding(&binding)?;
        Self::record_context_bundle_artifact_binding_tx(tx, binding, scope).await
    }

    async fn record_message_tx(
        tx: &mut Transaction<'_, Postgres>,
        input: NewModelLaneMessage,
        scope: ScopeColumnValues<'_>,
    ) -> ModelLaneResult<ModelLaneMessageRecord> {
        Self::record_message_tx_with_crdt_pause(tx, input, None, scope).await
    }

    async fn record_message_tx_with_crdt_pause(
        tx: &mut Transaction<'_, Postgres>,
        input: NewModelLaneMessage,
        crdt_pause: Option<(
            std::sync::Arc<tokio::sync::Notify>,
            std::sync::Arc<tokio::sync::Notify>,
        )>,
        scope: ScopeColumnValues<'_>,
    ) -> ModelLaneResult<ModelLaneMessageRecord> {
        lock_idempotency_key_tx(tx, &input.idempotency_key).await?;
        lock_idempotency_key_tx(tx, &format!("model-lane-message:{}", input.message_id)).await?;
        require_message_physical_keys_authorized_tx(
            tx,
            &input.message_id,
            &input.idempotency_key,
            scope,
        )
        .await?;
        if let Some(existing) =
            message_by_idempotency_key_for_write_scope_tx(tx, &input.idempotency_key, scope).await?
        {
            validate_stored_message_eventledger_authority_for_write_scope_tx(tx, scope, &existing)
                .await?;
            if existing.payload_sha256 == input.payload_sha256 {
                // Spec 4.3.9.2.5: "Duplicate retries with the same
                // idempotency_key and payload hash MUST be idempotent." The
                // idempotency_key is the caller's dedup token; message_id and
                // message_span_id identify a single delivery attempt (the
                // coordinator may assign a fresh id/span per retry), so they must
                // not defeat idempotent replay. All payload-authority and routing
                // fields (to_lane, authority, locus, crdt, payload_ref, ...) are
                // still compared and MUST match or the retry fails closed.
                let mut retry_identity = input.clone();
                retry_identity.message_id = existing.message_id.clone();
                retry_identity.message_span_id = existing.message_span_id.clone();
                ensure_idempotent_input_matches(
                    "model_lane_message",
                    &input.idempotency_key,
                    &existing.inner,
                    &retry_identity,
                )?;
                return Ok(existing);
            }
            return Err(ModelLaneError::IdempotencyConflict(format!(
                "idempotency_key {} already belongs to payload_sha256 {}",
                input.idempotency_key, existing.payload_sha256
            )));
        }
        // Lock peer lanes in one canonical order.  A pair of simultaneous
        // A->B / B->A messages must not acquire the two PostgreSQL row locks
        // in opposite order and deadlock the durable EventLedger path.
        let target_lane = match &input.to_lane {
            ModelLaneTarget::Lane(target_lane_id) if target_lane_id != &input.from_lane_id => {
                if target_lane_id < &input.from_lane_id {
                    let target_lane =
                        message_lane_by_id_for_run_tx(tx, &input.run_id, target_lane_id, scope)
                            .await?;
                    let source_lane = message_lane_by_id_tx(tx, &input.from_lane_id, scope).await?;
                    (source_lane, Some(target_lane))
                } else {
                    let source_lane = message_lane_by_id_tx(tx, &input.from_lane_id, scope).await?;
                    let target_lane =
                        message_lane_by_id_for_run_tx(tx, &input.run_id, target_lane_id, scope)
                            .await?;
                    (source_lane, Some(target_lane))
                }
            }
            ModelLaneTarget::Lane(_) => {
                let source_lane = message_lane_by_id_tx(tx, &input.from_lane_id, scope).await?;
                (source_lane.clone(), Some(source_lane))
            }
            ModelLaneTarget::Broadcast | ModelLaneTarget::Coordinator => (
                message_lane_by_id_tx(tx, &input.from_lane_id, scope).await?,
                None,
            ),
        };
        let (source_lane, target_lane) = target_lane;
        require_equal(
            "message.run_id",
            &input.run_id,
            "source_lane.run_id",
            &source_lane.run_id,
        )?;
        ensure_message_lane_is_live(&source_lane, "source")?;
        let source_run = message_run_by_id_tx(tx, &input.run_id, scope).await?;
        require_equal(
            "message.event_ledger_stream_id",
            &input.event_ledger_stream_id,
            "source_lane.event_ledger_stream_id",
            &source_lane.event_ledger_stream_id,
        )?;
        require_equal(
            "message.event_ledger_stream_id",
            &input.event_ledger_stream_id,
            "source_run.event_ledger_stream_id",
            &source_run.event_ledger_stream_id,
        )?;
        if let Some(target_lane) = target_lane.as_ref() {
            ensure_message_lane_is_live(target_lane, "target")?;
            require_equal(
                "message.event_ledger_stream_id",
                &input.event_ledger_stream_id,
                "target_lane.event_ledger_stream_id",
                &target_lane.event_ledger_stream_id,
            )?;
        }
        let cloud_source = is_cloud_lane_record(&source_lane);
        let resolved_crdt = validate_message_crdt_authority_tx(tx, &input).await?;
        let crdt_lease_authority = if let Some(resolved) = resolved_crdt.as_ref() {
            knowledge_crdt::lock_crdt_lease_authority_domain_tx(
                tx,
                &resolved.workspace_id,
                &resolved.crdt_document_id,
            )
            .await?;
            validate_crdt_lane_session_uniqueness_tx(tx, &source_lane, resolved).await?;
            Some(resolve_active_crdt_actor_lane_lease_tx(tx, &source_lane, resolved).await?)
        } else {
            None
        };
        let crdt_authority_binding = resolved_crdt
            .as_ref()
            .zip(crdt_lease_authority.as_ref())
            .map(|(resolved, lease)| {
                bind_crdt_authority_to_lane(&input, &source_lane, resolved, lease)
            })
            .transpose()?;
        if crdt_authority_binding.is_some() {
            if let Some((entered, release)) = crdt_pause {
                entered.notify_one();
                release.notified().await;
            }
        }
        match input.authority {
            ModelLaneAuthority::Promoted => {
                ensure_promoted_message_has_decision_tx(tx, &input, scope).await?;
            }
            ModelLaneAuthority::OperatorDecision | ModelLaneAuthority::ValidatorVerdict
                if cloud_source =>
            {
                return Err(ModelLaneError::InvalidInput(
                    "Cloud ModelLaneMessage authority must remain advisory or promotion_candidate until an approved PromotionGate writes promoted authority"
                        .into(),
                ));
            }
            _ => {}
        }

        let mut payload = json!({
            "schema_id": "hsk.model_lane_message@1",
            "dexterity_kernel": "Dexterity",
            "record": input,
            "crdt_authority_binding": crdt_authority_binding,
        });
        if let Some(exact_scope) = exact_resource_scope_from_columns(scope, "ModelLaneMessage")? {
            exact_scope.stamp_json_object(&mut payload).map_err(|_| {
                ModelLaneError::AuthorityDenied(
                    "ModelLaneMessage could not stamp complete resource attribution".into(),
                )
            })?;
        } else if input.authority == ModelLaneAuthority::Promoted {
            return Err(ModelLaneError::AuthorityDenied(
                "Promoted ModelLaneMessage could not stamp complete resource attribution".into(),
            ));
        }
        let event = model_lane_event(
            KernelEventType::ModelResponseRecorded,
            "model_lane_message",
            &input.message_id,
            &input.idempotency_key,
            input.work_packet_id.as_deref().unwrap_or(&input.run_id),
            &input.event_ledger_stream_id,
            payload,
        )?;

        let stored_event = append_kernel_event_with_executor(&mut **tx, event).await?;
        let sequence = stored_event.event_sequence;
        let record = ModelLaneMessageRecord {
            event_ledger_event_id: stored_event.event_id.clone(),
            event_ledger_seq: sequence,
            event_stream_version: sequence,
            transaction_seq: sequence,
            crdt_authority_binding,
            inner: input,
        };

        let insert_sql = format!(
            r#"
            INSERT INTO model_lane_messages (
                message_id, run_id, trace_id, message_span_id, from_lane_id,
                coordinator_session_id, work_packet_id, micro_task_id,
                task_board_id, owner_session, idempotency_key,
                payload_sha256, replay_order_key, authority,
                event_ledger_stream_id, event_ledger_event_id,
                event_ledger_seq, event_stream_version, transaction_seq,
                record_json,
                {RESOURCE_SCOPE_INSERT_COLUMNS}
            )
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24,$25)
            ON CONFLICT (idempotency_key) DO NOTHING
            RETURNING record_json
            "#
        );
        let inserted = scope
            .bind(
                sqlx::query(&insert_sql)
                    .bind(&record.message_id)
                    .bind(&record.run_id)
                    .bind(&record.trace_id)
                    .bind(&record.message_span_id)
                    .bind(&record.from_lane_id)
                    .bind(&record.coordinator_session_id)
                    .bind(record.work_packet_id.as_deref())
                    .bind(record.micro_task_id.as_deref())
                    .bind(record.task_board_id.as_deref())
                    .bind(&record.owner_session)
                    .bind(&record.idempotency_key)
                    .bind(&record.payload_sha256)
                    .bind(&record.replay_order_key)
                    .bind(record.authority.as_str())
                    .bind(&record.event_ledger_stream_id)
                    .bind(&record.event_ledger_event_id)
                    .bind(record.event_ledger_seq)
                    .bind(record.event_stream_version)
                    .bind(record.transaction_seq)
                    .bind(serde_json::to_value(&record)?),
            )
            .fetch_optional(&mut **tx)
            .await?;

        let stored = if let Some(row) = inserted {
            serde_json::from_value(row_to_json(row, "record_json")?)?
        } else {
            let existing =
                message_by_idempotency_key_for_write_scope_tx(tx, &record.idempotency_key, scope)
                    .await?;
            let existing = existing.ok_or_else(|| {
                ModelLaneError::AuthorityDenied("ModelLaneMessage authority unavailable".into())
            })?;
            validate_stored_message_eventledger_authority_for_write_scope_tx(tx, scope, &existing)
                .await?;
            if existing.payload_sha256 == record.payload_sha256 {
                ensure_idempotent_input_matches(
                    "model_lane_message",
                    &record.idempotency_key,
                    &existing.inner,
                    &record.inner,
                )?;
                existing
            } else {
                return Err(ModelLaneError::IdempotencyConflict(format!(
                    "idempotency_key {} already belongs to payload_sha256 {}",
                    record.idempotency_key, existing.payload_sha256
                )));
            }
        };

        Ok(stored)
    }

    pub async fn record_cloud_projection_plan(
        &self,
        input: NewModelLaneCloudProjectionPlan,
    ) -> ModelLaneResult<ModelLaneCloudProjectionPlanRecord> {
        self.record_cloud_projection_plan_surreal(input).await
    }

    pub async fn record_cloud_consent_receipt(
        &self,
        input: NewModelLaneCloudConsentReceipt,
    ) -> ModelLaneResult<ModelLaneCloudConsentReceiptRecord> {
        self.record_cloud_consent_receipt_surreal(input).await
    }

    pub async fn replay_cloud_consent_authority(
        &self,
        run_id: &str,
    ) -> ModelLaneResult<ModelLaneCloudConsentAuthorityReplay> {
        self.replay_cloud_consent_authority_surreal(run_id).await
    }

    pub async fn preflight_cloud_spawn_request(
        &self,
        request: &SpawnRequest,
    ) -> ModelLaneResult<()> {
        if request.provider != Some(ProviderKind::ByokCloud) {
            return Ok(());
        }
        let contract = request.dexterity_launch.as_ref().ok_or_else(|| {
            ModelLaneError::InvalidInput(
                "CX-MM-007 cloud launch requires Dexterity launch contract before provider call"
                    .into(),
            )
        })?;
        let provider_kind = match request.byok_cloud_provider {
            Some(ByokCloudProvider::OpenAi) => "openai",
            Some(ByokCloudProvider::Anthropic) => "anthropic",
            None => {
                let mut check = CloudLaunchAuthorityCheck::from_contract(
                    contract,
                    "unknown",
                    "",
                    runtime_session_id(request),
                )?;
                check.work_packet_id = request
                    .wp_id
                    .clone()
                    .unwrap_or_else(|| contract.run_id.clone());
                check.micro_task_id = request.mt_id.clone();
                check.owner_session = request.owner_role.clone();
                return self
                    .deny_cloud_launch(check, "missing_byok_cloud_provider")
                    .await;
            }
        };
        let requested_model_id = dexterity_candidate_model_ids(request)
            .into_iter()
            .next()
            .unwrap_or_else(|| request.instance_id.model_id.to_string());
        let mut check = CloudLaunchAuthorityCheck::from_contract(
            contract,
            provider_kind,
            &requested_model_id,
            runtime_session_id(request),
        )?;
        check.work_packet_id = request
            .wp_id
            .clone()
            .unwrap_or_else(|| contract.run_id.clone());
        check.micro_task_id = request.mt_id.clone();
        check.owner_session = request.owner_role.clone();
        self.preflight_cloud_launch(check).await
    }

    pub(crate) async fn fence_cloud_consent_revocation(
        &self,
        consent_receipt_id: &str,
        revoked_by_ref: &str,
        reason: &str,
    ) -> ModelLaneResult<Vec<ModelLaneRecord>> {
        self.fence_cloud_consent_revocation_surreal(consent_receipt_id, revoked_by_ref, reason)
            .await
    }

    pub(crate) async fn finalize_cloud_consent_revocation(
        &self,
        consent_receipt_id: &str,
        revoked_by_ref: &str,
        reason: &str,
        provider_cancelled_lane_ids: &std::collections::BTreeSet<String>,
    ) -> ModelLaneResult<Vec<ModelLaneRecord>> {
        self.finalize_cloud_consent_revocation_surreal(
            consent_receipt_id,
            revoked_by_ref,
            reason,
            provider_cancelled_lane_ids,
        )
        .await
    }

    #[cfg(feature = "test-utils")]
    pub async fn test_fence_cloud_consent_revocation(
        &self,
        consent_receipt_id: &str,
        revoked_by_ref: &str,
        reason: &str,
    ) -> ModelLaneResult<Vec<ModelLaneRecord>> {
        self.fence_cloud_consent_revocation(consent_receipt_id, revoked_by_ref, reason)
            .await
    }

    #[cfg(feature = "test-utils")]
    pub async fn test_finalize_cloud_consent_revocation(
        &self,
        consent_receipt_id: &str,
        revoked_by_ref: &str,
        reason: &str,
        provider_cancelled_lane_ids: &std::collections::BTreeSet<String>,
    ) -> ModelLaneResult<Vec<ModelLaneRecord>> {
        self.finalize_cloud_consent_revocation(
            consent_receipt_id,
            revoked_by_ref,
            reason,
            provider_cancelled_lane_ids,
        )
        .await
    }

    #[cfg(feature = "test-utils")]
    pub async fn test_commit_cloud_consent_revocation(
        &self,
        consent_receipt_id: &str,
        revoked_by_ref: &str,
        reason: &str,
    ) -> ModelLaneResult<Vec<ModelLaneRecord>> {
        self.fence_cloud_consent_revocation(consent_receipt_id, revoked_by_ref, reason)
            .await?;
        self.finalize_cloud_consent_revocation(
            consent_receipt_id,
            revoked_by_ref,
            reason,
            &std::collections::BTreeSet::new(),
        )
        .await
    }

    pub async fn record_promotion_decision(
        &self,
        input: NewModelLanePromotionDecision,
    ) -> ModelLaneResult<ModelLanePromotionDecisionRecord> {
        let exact_scope = self
            .write_scope()
            .ok_or_else(|| {
                ModelLaneError::AuthorityDenied(
                    "PromotionGate requires complete account/principal/session/AccessSpace/workspace authority"
                        .into(),
                )
            })
            .and_then(|scope| {
                ExactResourceScopeAttribution::try_from_resource_scope(scope).map_err(|_| {
                    ModelLaneError::AuthorityDenied(
                        "PromotionGate requires complete account/principal/session/AccessSpace/workspace authority"
                            .into(),
                    )
                })
            })?;
        let mut input = input;
        let routing_graph = super::routing::ModelLaneRoutingGraph::for_policy(input.routing_policy);
        routing_graph
            .validate()
            .map_err(|error| ModelLaneError::InvalidInput(error.to_string()))?;
        input.diagnostic_payload = merge_diagnostic_payload(
            input.diagnostic_payload,
            json!({
                "routing_graph": routing_graph,
                "routing_graph_schema_id": super::routing::ModelLaneRoutingGraph::SCHEMA_ID,
            }),
        );
        validate_promotion_decision(&input)?;
        let mut tx = self.pool.begin().await?;
        lock_idempotency_key_tx(&mut tx, &input.idempotency_key).await?;
        lock_idempotency_key_tx(
            &mut tx,
            &format!("model-lane-promotion-decision:{}", input.decision_id),
        )
        .await?;
        require_promotion_physical_keys_authorized_tx(
            &mut tx,
            &self.access,
            &input.decision_id,
            &input.idempotency_key,
        )
        .await?;
        let prepared =
            prepare_promotion_decision_tx(&mut tx, &self.access, &exact_scope, input).await?;

        if let Some(existing) = promotion_decision_by_idempotency_key_tx(
            &mut tx,
            &self.access,
            &prepared.idempotency_key,
        )
        .await?
        {
            if existing.canonical_decision_hash == prepared.canonical_decision_hash {
                tx.commit().await?;
                return Ok(existing);
            }
            tx.rollback().await?;
            return Err(ModelLaneError::IdempotencyConflict(format!(
                "idempotency_key {} already belongs to canonical_decision_hash {}",
                prepared.idempotency_key, existing.canonical_decision_hash
            )));
        }

        if let Some(existing) =
            promotion_decision_by_id_tx(&mut tx, &self.access, &prepared.decision_id).await?
        {
            if existing.canonical_decision_hash == prepared.canonical_decision_hash {
                tx.commit().await?;
                return Ok(existing);
            }
            tx.rollback().await?;
            return Err(ModelLaneError::IdempotencyConflict(format!(
                "decision_id {} already belongs to idempotency_key {}",
                prepared.decision_id, existing.idempotency_key
            )));
        }

        let event_type = match prepared.outcome {
            ModelLanePromotionOutcome::Approved => KernelEventType::PromotionAccepted,
            ModelLanePromotionOutcome::Denied => KernelEventType::PromotionRejected,
        };
        let mut payload = json!({
            "schema_id": "hsk.model_lane_promotion_decision@1",
            "dexterity_kernel": "Dexterity",
            "record": &prepared,
        });
        exact_scope.stamp_json_object(&mut payload).map_err(|_| {
            ModelLaneError::AuthorityDenied(
                "PromotionGate could not stamp complete resource attribution".into(),
            )
        })?;
        let event = model_lane_event(
            event_type,
            "model_lane_promotion_decision",
            &prepared.decision_id,
            &prepared.idempotency_key,
            prepared
                .work_packet_id
                .as_deref()
                .unwrap_or(&prepared.run_id),
            &prepared.event_ledger_stream_id,
            payload,
        )?;
        let stored_event = append_kernel_event_with_executor(&mut *tx, event).await?;
        let sequence = stored_event.event_sequence;
        let record = ModelLanePromotionDecisionRecord {
            event_ledger_event_id: stored_event.event_id.clone(),
            event_ledger_seq: sequence,
            event_stream_version: sequence,
            transaction_seq: sequence,
            ..prepared
        };
        let mut final_payload = json!({
            "schema_id": "hsk.model_lane_promotion_decision@1",
            "dexterity_kernel": "Dexterity",
            "record": &record,
        });
        exact_scope
            .stamp_json_object(&mut final_payload)
            .map_err(|_| {
                ModelLaneError::AuthorityDenied(
                    "PromotionGate could not stamp complete resource attribution".into(),
                )
            })?;
        stamp_kernel_event_payload_tx(&mut tx, &record.event_ledger_event_id, final_payload)
            .await?;

        let insert_sql = format!(
            r#"
            INSERT INTO model_lane_promotion_decisions (
                decision_id, run_id, trace_id, decision_span_id,
                coordinator_session_id, routing_policy, outcome, final_state,
                denial_reason, work_packet_id, micro_task_id,
                task_board_id, owner_session, idempotency_key,
                canonical_decision_hash, expected_event_ledger_aggregate_type,
                expected_event_ledger_aggregate_id, expected_event_ledger_version,
                current_event_ledger_version, schema_id, current_schema_id,
                base_snapshot_ref, current_base_snapshot_ref, state_vector,
                current_state_vector, promotion_gate_ref, promotion_receipt_ref,
                event_ledger_stream_id, event_ledger_event_id, event_ledger_seq,
                event_stream_version, transaction_seq, record_json,
                {RESOURCE_SCOPE_INSERT_COLUMNS}
            )
            VALUES (
                $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,
                $17,$18,$19,$20,$21,$22,$23,$24,$25,$26,$27,$28,$29,$30,
                $31,$32,$33,$34,$35,$36,$37,$38
            )
            ON CONFLICT (idempotency_key) DO NOTHING
            RETURNING record_json
            "#
        );
        let inserted = self
            .scope_columns()
            .bind(
                sqlx::query(&insert_sql)
                    .bind(&record.decision_id)
                    .bind(&record.run_id)
                    .bind(&record.trace_id)
                    .bind(&record.decision_span_id)
                    .bind(&record.coordinator_session_id)
                    .bind(record.routing_policy.as_str())
                    .bind(record.outcome.as_str())
                    .bind(record.final_state.as_str())
                    .bind(record.denial_reason.as_ref().map(|reason| reason.as_str()))
                    .bind(record.work_packet_id.as_deref())
                    .bind(record.micro_task_id.as_deref())
                    .bind(record.task_board_id.as_deref())
                    .bind(&record.owner_session)
                    .bind(&record.idempotency_key)
                    .bind(&record.canonical_decision_hash)
                    .bind(&record.expected_event_ledger_aggregate_type)
                    .bind(&record.expected_event_ledger_aggregate_id)
                    .bind(record.expected_event_ledger_version)
                    .bind(record.current_event_ledger_version)
                    .bind(&record.schema_id)
                    .bind(record.current_schema_id.as_deref())
                    .bind(&record.base_snapshot_ref)
                    .bind(&record.current_base_snapshot_ref)
                    .bind(&record.state_vector)
                    .bind(&record.current_state_vector)
                    .bind(&record.promotion_gate_ref)
                    .bind(record.promotion_receipt_ref.as_deref())
                    .bind(&record.event_ledger_stream_id)
                    .bind(&record.event_ledger_event_id)
                    .bind(record.event_ledger_seq)
                    .bind(record.event_stream_version)
                    .bind(record.transaction_seq)
                    .bind(serde_json::to_value(&record)?),
            )
            .fetch_optional(&mut *tx)
            .await?;

        let stored = if let Some(row) = inserted {
            serde_json::from_value(row_to_json(row, "record_json")?)?
        } else {
            let existing = promotion_decision_by_idempotency_key_tx(
                &mut tx,
                &self.access,
                &record.idempotency_key,
            )
            .await?;
            let existing = existing.ok_or_else(|| {
                ModelLaneError::NotFound(format!(
                    "idempotency_key {} after promotion decision insert conflict",
                    record.idempotency_key
                ))
            })?;
            if existing.canonical_decision_hash == record.canonical_decision_hash {
                existing
            } else {
                tx.rollback().await?;
                return Err(ModelLaneError::IdempotencyConflict(format!(
                    "idempotency_key {} already belongs to canonical_decision_hash {}",
                    record.idempotency_key, existing.canonical_decision_hash
                )));
            }
        };

        tx.commit().await?;
        Ok(stored)
    }

    pub async fn replay_promotion_decisions(
        &self,
        run_id: &str,
    ) -> ModelLaneResult<Vec<ModelLanePromotionDecisionRecord>> {
        require_token("run_id", run_id)?;
        let predicate = self.access.sql_predicate(2);
        let mut tx = self.pool.begin().await?;
        let sql = format!(
            r#"
            SELECT record_json, {RESOURCE_SCOPE_SELECT_COLUMNS}
            FROM model_lane_promotion_decisions
            WHERE run_id = $1{}
            ORDER BY event_ledger_seq ASC
            "#,
            predicate.clause()
        );
        let records = predicate
            .bind(sqlx::query(&sql).bind(run_id))
            .fetch_all(&mut *tx)
            .await?
            .into_iter()
            .map(|row| authorize_and_decode_row(&self.access, row))
            .collect::<ModelLaneResult<Vec<_>>>()?;
        for record in &records {
            validate_stored_promotion_decision_authority_tx(&mut tx, &self.access, record).await?;
        }
        tx.commit().await?;
        Ok(records)
    }

    pub async fn record_context_bundle_artifact_binding(
        &self,
        input: NewModelLaneContextBundleArtifactBinding,
    ) -> ModelLaneResult<ModelLaneContextBundleArtifactBindingRecord> {
        validate_context_bundle_artifact_binding(&input)?;
        let mut tx = self.pool.begin().await?;
        let stored =
            Self::record_context_bundle_artifact_binding_tx(&mut tx, input, self.scope_columns())
                .await?;
        tx.commit().await?;
        Ok(stored)
    }

    async fn record_context_bundle_artifact_binding_tx(
        tx: &mut Transaction<'_, Postgres>,
        input: NewModelLaneContextBundleArtifactBinding,
        scope: ScopeColumnValues<'_>,
    ) -> ModelLaneResult<ModelLaneContextBundleArtifactBindingRecord> {
        lock_idempotency_key_tx(tx, &input.idempotency_key).await?;
        context_bundle_run_by_id_for_write_scope_tx(tx, &input.run_id, scope).await?;
        let artifact_access =
            ResourceAccessContext::for_exact_reader(promotion_exact_scope_from_columns(scope)?);
        let binding_hash = context_bundle_artifact_binding_hash(&input)?;
        let prepared = ModelLaneContextBundleArtifactBindingRecord {
            inner: input,
            artifact_binding_hash: binding_hash,
            event_ledger_event_id: String::new(),
            event_ledger_seq: 0,
            event_stream_version: 0,
            transaction_seq: 0,
        };

        if let Some(existing) =
            context_bundle_artifact_binding_by_idempotency_key_for_write_scope_tx(
                tx,
                &prepared.idempotency_key,
                scope,
            )
            .await?
        {
            if existing.artifact_binding_hash == prepared.artifact_binding_hash {
                validate_stored_context_bundle_artifact_authority_tx(
                    tx,
                    &artifact_access,
                    &existing,
                )
                .await?;
                return Ok(existing);
            }
            return Err(ModelLaneError::IdempotencyConflict(format!(
                "idempotency_key {} already belongs to artifact_binding_hash {}",
                prepared.idempotency_key, existing.artifact_binding_hash
            )));
        }

        let event = model_lane_event(
            KernelEventType::ArtifactStored,
            "model_lane_context_bundle_artifact",
            &prepared.artifact_binding_id,
            &prepared.idempotency_key,
            &prepared.work_packet_id,
            &prepared.event_ledger_stream_id,
            context_bundle_artifact_binding_event_payload(&prepared, scope),
        )?;
        let stored_event = append_kernel_event_with_executor(&mut **tx, event).await?;
        let sequence = stored_event.event_sequence;
        let record = ModelLaneContextBundleArtifactBindingRecord {
            event_ledger_event_id: stored_event.event_id.clone(),
            event_ledger_seq: sequence,
            event_stream_version: sequence,
            transaction_seq: sequence,
            ..prepared
        };
        stamp_kernel_event_payload_tx(
            tx,
            &record.event_ledger_event_id,
            context_bundle_artifact_binding_event_payload(&record, scope),
        )
        .await?;

        let insert_sql = format!(
            r#"
            INSERT INTO model_lane_context_bundle_artifacts (
                artifact_binding_id, run_id, trace_id, artifact_ref,
                artifact_sha256, content_hash, artifact_kind,
                artifact_manifest_ref, artifact_payload_ref, payload_json,
                event_ledger_stream_id, work_packet_id, micro_task_id,
                task_board_id, owner_session, idempotency_key,
                artifact_binding_hash, event_ledger_event_id,
                event_ledger_seq, event_stream_version, transaction_seq,
                record_json,
                {RESOURCE_SCOPE_INSERT_COLUMNS}
            )
            VALUES (
                $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,
                $13,$14,$15,$16,$17,$18,$19,$20,$21,$22,
                $23,$24,$25,$26,$27
            )
            ON CONFLICT (idempotency_key) DO NOTHING
            RETURNING record_json
            "#
        );
        let inserted = scope
            .bind(
                sqlx::query(&insert_sql)
                    .bind(&record.artifact_binding_id)
                    .bind(&record.run_id)
                    .bind(&record.trace_id)
                    .bind(&record.artifact_ref)
                    .bind(&record.artifact_sha256)
                    .bind(&record.content_hash)
                    .bind(&record.artifact_kind)
                    .bind(&record.artifact_manifest_ref)
                    .bind(&record.artifact_payload_ref)
                    .bind(&record.payload_json)
                    .bind(&record.event_ledger_stream_id)
                    .bind(&record.work_packet_id)
                    .bind(&record.micro_task_id)
                    .bind(&record.task_board_id)
                    .bind(&record.owner_session)
                    .bind(&record.idempotency_key)
                    .bind(&record.artifact_binding_hash)
                    .bind(&record.event_ledger_event_id)
                    .bind(record.event_ledger_seq)
                    .bind(record.event_stream_version)
                    .bind(record.transaction_seq)
                    .bind(serde_json::to_value(&record)?),
            )
            .fetch_optional(&mut **tx)
            .await?;

        let stored = if let Some(row) = inserted {
            serde_json::from_value(row_to_json(row, "record_json")?)?
        } else {
            let existing = context_bundle_artifact_binding_by_idempotency_key_for_write_scope_tx(
                tx,
                &record.idempotency_key,
                scope,
            )
            .await?;
            let existing = existing.ok_or_else(|| {
                ModelLaneError::NotFound(format!(
                    "idempotency_key {} after artifact binding insert conflict",
                    record.idempotency_key
                ))
            })?;
            if existing.artifact_binding_hash == record.artifact_binding_hash {
                existing
            } else {
                return Err(ModelLaneError::IdempotencyConflict(format!(
                    "idempotency_key {} already belongs to artifact_binding_hash {}",
                    record.idempotency_key, existing.artifact_binding_hash
                )));
            }
        };

        validate_stored_context_bundle_artifact_authority_tx(tx, &artifact_access, &stored).await?;
        Ok(stored)
    }

    pub async fn record_context_bundle_handoff(
        &self,
        input: NewModelLaneContextBundleHandoff,
    ) -> ModelLaneResult<ModelLaneContextBundleHandoffRecord> {
        validate_context_bundle_handoff(&input)?;
        let mut tx = self.pool.begin().await?;
        lock_idempotency_key_tx(&mut tx, &input.idempotency_key).await?;
        let prepared = prepare_context_bundle_handoff_tx(&mut tx, &self.access, input).await?;

        if let Some(existing) = context_bundle_handoff_by_idempotency_key_tx(
            &mut tx,
            &self.access,
            &prepared.idempotency_key,
        )
        .await?
        {
            if existing.context_bundle_hash == prepared.context_bundle_hash {
                validate_stored_context_bundle_handoff_authority_tx(
                    &mut tx,
                    &self.access,
                    &existing,
                )
                .await?;
                tx.commit().await?;
                return Ok(existing);
            }
            tx.rollback().await?;
            return Err(ModelLaneError::IdempotencyConflict(format!(
                "idempotency_key {} already belongs to context_bundle_hash {}",
                prepared.idempotency_key, existing.context_bundle_hash
            )));
        }

        let event = model_lane_event(
            KernelEventType::ContextBundleRecorded,
            "model_lane_context_bundle_handoff",
            &prepared.handoff_id,
            &prepared.idempotency_key,
            &prepared.work_packet_id,
            &prepared.event_ledger_stream_id,
            context_bundle_handoff_event_payload(&prepared, self.scope_columns()),
        )?;
        let stored_event = append_kernel_event_with_executor(&mut *tx, event).await?;
        let sequence = stored_event.event_sequence;
        let record = ModelLaneContextBundleHandoffRecord {
            event_ledger_event_id: stored_event.event_id.clone(),
            event_ledger_seq: sequence,
            event_stream_version: sequence,
            transaction_seq: sequence,
            ..prepared
        };
        stamp_kernel_event_payload_tx(
            &mut tx,
            &record.event_ledger_event_id,
            context_bundle_handoff_event_payload(&record, self.scope_columns()),
        )
        .await?;

        let insert_sql = format!(
            r#"
            INSERT INTO model_lane_context_bundle_handoffs (
                handoff_id, context_bundle_id, run_id, trace_id, handoff_span_id,
                downstream_lane_id, source_lane_id, source_message_id,
                artifact_ref, artifact_sha256, content_hash, source_kind, authority_state,
                selection_state, reason_code, decision_ref, reviewer_ref,
                work_packet_id, micro_task_id, task_board_id, owner_session,
                idempotency_key, context_bundle_hash, event_ledger_stream_id,
                event_ledger_event_id, event_ledger_seq, event_stream_version,
                transaction_seq, record_json,
                {RESOURCE_SCOPE_INSERT_COLUMNS}
            )
            VALUES (
                $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,
                $17,$18,$19,$20,$21,$22,$23,$24,$25,$26,$27,$28,$29,
                $30,$31,$32,$33,$34
            )
            ON CONFLICT (idempotency_key) DO NOTHING
            RETURNING record_json
            "#
        );
        let inserted = self
            .scope_columns()
            .bind(
                sqlx::query(&insert_sql)
                    .bind(&record.handoff_id)
                    .bind(&record.context_bundle_id)
                    .bind(&record.run_id)
                    .bind(&record.trace_id)
                    .bind(&record.handoff_span_id)
                    .bind(&record.downstream_lane_id)
                    .bind(&record.source_lane_id)
                    .bind(&record.source_message_id)
                    .bind(&record.artifact_ref)
                    .bind(&record.artifact_sha256)
                    .bind(&record.content_hash)
                    .bind(record.source_kind.as_str())
                    .bind(record.authority_state.as_str())
                    .bind(record.selection_state.as_str())
                    .bind(&record.reason_code)
                    .bind(record.decision_ref.as_deref())
                    .bind(record.reviewer_ref.as_deref())
                    .bind(&record.work_packet_id)
                    .bind(&record.micro_task_id)
                    .bind(&record.task_board_id)
                    .bind(&record.owner_session)
                    .bind(&record.idempotency_key)
                    .bind(&record.context_bundle_hash)
                    .bind(&record.event_ledger_stream_id)
                    .bind(&record.event_ledger_event_id)
                    .bind(record.event_ledger_seq)
                    .bind(record.event_stream_version)
                    .bind(record.transaction_seq)
                    .bind(serde_json::to_value(&record)?),
            )
            .fetch_optional(&mut *tx)
            .await?;

        let stored = if let Some(row) = inserted {
            serde_json::from_value(row_to_json(row, "record_json")?)?
        } else {
            let existing = context_bundle_handoff_by_idempotency_key_tx(
                &mut tx,
                &self.access,
                &record.idempotency_key,
            )
            .await?;
            let existing = existing.ok_or_else(|| {
                ModelLaneError::NotFound(format!(
                    "idempotency_key {} after context bundle handoff insert conflict",
                    record.idempotency_key
                ))
            })?;
            if existing.context_bundle_hash == record.context_bundle_hash {
                existing
            } else {
                tx.rollback().await?;
                return Err(ModelLaneError::IdempotencyConflict(format!(
                    "idempotency_key {} already belongs to context_bundle_hash {}",
                    record.idempotency_key, existing.context_bundle_hash
                )));
            }
        };

        validate_stored_context_bundle_handoff_authority_tx(&mut tx, &self.access, &stored).await?;
        tx.commit().await?;
        Ok(stored)
    }

    pub async fn consume_context_bundle_for_downstream(
        &self,
        run_id: &str,
        context_bundle_id: &str,
        downstream_lane_id: &str,
    ) -> ModelLaneResult<ModelLaneDownstreamContextBundle> {
        require_token("run_id", run_id)?;
        require_token("context_bundle_id", context_bundle_id)?;
        require_token("downstream_lane_id", downstream_lane_id)?;
        require_exact_context_bundle_read_scope(&self.access)?;
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT set_config('application_name', $1, true)")
            .bind(format!("hsk:cb:consume:{run_id}"))
            .execute(&mut *tx)
            .await?;
        let run = context_bundle_run_by_id_tx(&mut tx, &self.access, run_id).await?;
        require_equal("run.run_id", &run.run_id, "run_id", run_id)?;
        let lane = context_bundle_lane_by_id_tx(&mut tx, &self.access, downstream_lane_id)
            .await
            .map_err(|err| match err {
                ModelLaneError::NotFound(message) => ModelLaneError::InvalidInput(format!(
                    "downstream_lane_id {downstream_lane_id} is not replayable: {message}"
                )),
                other => other,
            })?;
        require_equal("downstream.run_id", &lane.run_id, "run_id", run_id)?;
        let predicate = self.access.sql_predicate(4);
        let sql = format!(
            "SELECT record_json, {RESOURCE_SCOPE_SELECT_COLUMNS} \
             FROM model_lane_context_bundle_handoffs \
             WHERE run_id = $1 AND context_bundle_id = $2 AND downstream_lane_id = $3{} \
             ORDER BY event_ledger_seq ASC",
            predicate.clause()
        );
        let records = predicate
            .bind(
                sqlx::query(&sql)
                    .bind(run_id)
                    .bind(context_bundle_id)
                    .bind(downstream_lane_id),
            )
            .fetch_all(&mut *tx)
            .await?
            .into_iter()
            .map(|row| authorize_and_decode_row(&self.access, row))
            .collect::<ModelLaneResult<Vec<ModelLaneContextBundleHandoffRecord>>>()?;
        if records.is_empty() {
            tx.rollback().await?;
            return Err(ModelLaneError::InvalidInput(format!(
                "context_bundle_id {context_bundle_id} has no replayable handoffs for downstream_lane_id {downstream_lane_id}"
            )));
        }
        for record in &records {
            validate_stored_context_bundle_handoff_authority_tx(&mut tx, &self.access, record)
                .await?;
            let artifact = context_bundle_artifact_binding_by_ref_tx(
                &mut tx,
                &self.access,
                &record.run_id,
                &record.artifact_ref,
            )
            .await?
            .ok_or_else(|| {
                ModelLaneError::InvalidInput(format!(
                    "artifact_ref {} is not backed by ArtifactStore/EventLedger authority",
                    record.artifact_ref
                ))
            })?;
            require_equal(
                "replay.artifact_sha256",
                &record.artifact_sha256,
                "artifact_binding.artifact_sha256",
                &artifact.artifact_sha256,
            )?;
            require_equal(
                "replay.content_hash",
                &record.content_hash,
                "artifact_binding.content_hash",
                &artifact.content_hash,
            )?;
            if let Some(crdt_payload) = record.crdt_payload.as_ref() {
                validate_crdt_handoff_authority_tx(&mut tx, crdt_payload).await?;
            }
        }
        tx.commit().await?;
        Ok(build_downstream_context_bundle(
            run_id,
            context_bundle_id,
            downstream_lane_id,
            records,
        )?)
    }

    pub async fn replay_context_bundle_handoffs(
        &self,
        run_id: &str,
        context_bundle_id: &str,
    ) -> ModelLaneResult<Vec<ModelLaneContextBundleHandoffRecord>> {
        require_token("run_id", run_id)?;
        require_token("context_bundle_id", context_bundle_id)?;
        require_exact_context_bundle_read_scope(&self.access)?;
        let mut tx = self.pool.begin().await?;
        let predicate = self.access.sql_predicate(3);
        let sql = format!(
            "SELECT record_json, {RESOURCE_SCOPE_SELECT_COLUMNS} \
             FROM model_lane_context_bundle_handoffs \
             WHERE run_id = $1 AND context_bundle_id = $2{} \
             ORDER BY event_ledger_seq ASC",
            predicate.clause()
        );
        let records = predicate
            .bind(sqlx::query(&sql).bind(run_id).bind(context_bundle_id))
            .fetch_all(&mut *tx)
            .await?
            .into_iter()
            .map(|row| authorize_and_decode_row(&self.access, row))
            .collect::<ModelLaneResult<Vec<ModelLaneContextBundleHandoffRecord>>>()?;
        for record in &records {
            validate_stored_context_bundle_handoff_authority_tx(&mut tx, &self.access, record)
                .await?;
        }
        tx.commit().await?;
        Ok(records)
    }

    pub async fn record_lane_terminal_status(
        &self,
        lane_id: &str,
        status: ModelLaneStatus,
        reason: &str,
    ) -> ModelLaneResult<ModelLaneRecord> {
        require_token("lane_id", lane_id)?;
        require_token("terminal_reason", reason)?;
        if !matches!(
            status,
            ModelLaneStatus::Completed | ModelLaneStatus::Failed | ModelLaneStatus::Cancelled
        ) {
            return Err(ModelLaneError::InvalidInput(format!(
                "terminal lane update requires completed, failed, or cancelled status; got {}",
                status.as_str()
            )));
        }

        let requested_exact_scope = match self.write_scope() {
            Some(scope) => Some(
                ExactResourceScopeAttribution::try_from_resource_scope(scope).map_err(|_| {
                    ModelLaneError::AuthorityDenied(
                        "ModelLane terminal mutation requires exact owner, Principal, authenticated session, AccessSpace, and workspace authority"
                            .into(),
                    )
                })?,
            ),
            None if self.access.system_authority().is_some() => None,
            None => {
                return Err(ModelLaneError::AuthorityDenied(
                    "ModelLane terminal mutation requires write authority".into(),
                ))
            }
        };
        let mut tx = self.pool.begin().await?;
        lock_idempotency_key_tx(&mut tx, &format!("model-lane-lifecycle:{lane_id}")).await?;
        let (existing, stored_exact_scope) = lane_by_access_tx(&mut tx, &self.access, lane_id)
            .await?
            .ok_or_else(|| ModelLaneError::NotFound("ModelLane lifecycle authority".into()))?;
        let exact_scope = requested_exact_scope.or(stored_exact_scope);
        let terminal_idempotency_key = if existing.restart_generation == 0 {
            format!("model-lane-terminal:{lane_id}")
        } else {
            format!(
                "model-lane-terminal:{lane_id}:restart:{}",
                existing.restart_generation
            )
        };
        lock_idempotency_key_tx(&mut tx, &terminal_idempotency_key).await?;
        if matches!(
            existing.status,
            ModelLaneStatus::Completed | ModelLaneStatus::Failed | ModelLaneStatus::Cancelled
        ) {
            if existing.status == status {
                tx.commit().await?;
                return Ok(existing);
            }
            tx.rollback().await?;
            return Err(ModelLaneError::IdempotencyConflict(format!(
                "lane_id {lane_id} is already terminal as {}",
                existing.status.as_str()
            )));
        }

        let mut lane = existing.inner.clone();
        lane.status = status.clone();
        lane.recovery_state = recovery_for_status(&status);
        lane.failstate_code = match status {
            ModelLaneStatus::Completed => None,
            ModelLaneStatus::Failed => Some("failed".into()),
            ModelLaneStatus::Cancelled => Some("cancelled".into()),
            _ => unreachable!("terminal status validated above"),
        };
        if status == ModelLaneStatus::Failed && lane.startup_failure_ref.is_none() {
            lane.startup_failure_ref = Some(format!("terminal-failure://dexterity/{lane_id}"));
        }
        lane.reason_ref = Some(format!(
            "terminal-reason://dexterity/{lane_id}/{}",
            status.as_str()
        ));
        lane.recovery_hint_ref = Some("usermanual://model-lane-launch-adapters#recovery".into());
        lane.last_runtime_status_ref = Some(format!(
            "runtime-status://dexterity/{lane_id}/{}",
            status.as_str()
        ));
        lane.last_recovery_event_ref = Some(format!(
            "event-ledger://dexterity/{lane_id}/{}",
            status.as_str()
        ));
        validate_lane(&lane)?;

        let event_type = match status {
            ModelLaneStatus::Completed => KernelEventType::SessionCompleted,
            ModelLaneStatus::Failed => KernelEventType::SessionFailed,
            ModelLaneStatus::Cancelled => KernelEventType::SessionCancelled,
            _ => unreachable!("terminal status validated above"),
        };
        let mut payload = json!({
            "schema_id": "hsk.model_lane_terminal@1",
            "dexterity_kernel": "Dexterity",
            "lane_id": &lane.lane_id,
            "run_id": &lane.run_id,
            "status": status.as_str(),
            "reason": reason,
            "previous_event_ledger_event_id": &existing.event_ledger_event_id,
            "previous_event_ledger_seq": existing.event_ledger_seq,
        });
        if let Some(exact_scope) = exact_scope.as_ref() {
            exact_scope.stamp_json_object(&mut payload).map_err(|_| {
                ModelLaneError::AuthorityDenied(
                    "ModelLane terminal EventLedger payload requires exact resource attribution"
                        .into(),
                )
            })?;
        }
        let event = model_lane_event(
            event_type,
            "model_lane_terminal",
            &lane.lane_id,
            &terminal_idempotency_key,
            lane.work_packet_id.as_deref().unwrap_or(&lane.run_id),
            &lane.event_ledger_stream_id,
            payload,
        )?;
        let stored_event = append_kernel_event_with_executor(&mut *tx, event).await?;
        lane.last_recovery_event_ref = Some(stored_event.event_id.clone());
        let record = ModelLaneRecord {
            event_ledger_event_id: stored_event.event_id.clone(),
            event_ledger_seq: stored_event.event_sequence,
            inner: lane,
        };

        // The mutable `model_lanes` row is repointed below to this terminal
        // event. `validate_diagnostics_row_eventledger_authority` (invoked by
        // replay_run/diagnostics_projection) requires the row's
        // event_ledger_event_id to resolve to an EventLedger payload whose
        // `record` matches the row. Re-stamp the terminal event payload with the
        // full updated lane record so that invariant holds instead of failing
        // with "model_lane EventLedger payload missing record".
        let mut stored_terminal_payload = json!({
            "schema_id": "hsk.model_lane_terminal@1",
            "dexterity_kernel": "Dexterity",
            "lane_id": &record.lane_id,
            "run_id": &record.run_id,
            "status": status.as_str(),
            "reason": reason,
            "previous_event_ledger_event_id": &existing.event_ledger_event_id,
            "previous_event_ledger_seq": existing.event_ledger_seq,
            "record": serde_json::to_value(&record.inner)?,
        });
        if let Some(exact_scope) = exact_scope.as_ref() {
            exact_scope
                .stamp_json_object(&mut stored_terminal_payload)
                .map_err(|_| {
                    ModelLaneError::AuthorityDenied(
                    "ModelLane terminal EventLedger payload requires exact resource attribution"
                        .into(),
                )
                })?;
        }
        stamp_kernel_event_payload_tx(
            &mut tx,
            &record.event_ledger_event_id,
            stored_terminal_payload,
        )
        .await?;

        let row = sqlx::query(
            r#"
            UPDATE model_lanes
            SET status = $2,
                event_ledger_event_id = $3,
                event_ledger_seq = $4,
                record_json = $5,
                updated_at = NOW()
            WHERE lane_id = $1
            RETURNING record_json
            "#,
        )
        .bind(lane_id)
        .bind(record.status.as_str())
        .bind(&record.event_ledger_event_id)
        .bind(record.event_ledger_seq)
        .bind(serde_json::to_value(&record)?)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ModelLaneError::NotFound(format!("lane_id {lane_id}")))?;
        tx.commit().await?;
        serde_json::from_value(row_to_json(row, "record_json")?).map_err(Into::into)
    }

    /// Replay one run. This is the widest ModelLane read funnel — transcript,
    /// diagnostics, palmistry, and all eight navigation routes reach durable
    /// rows through here — so it is the primary HBR-PRIV-002 chokepoint.
    ///
    /// Two enforcement layers, both required:
    ///   1. the owner predicate is pushed into every `WHERE`, so a denied row is
    ///      never transferred out of PostgreSQL at all; and
    ///   2. the scope columns are read back and re-authorized after
    ///      deserialization, so a future query edit that drops the predicate
    ///      still fails closed instead of silently disclosing.
    pub async fn replay_run(&self, run_id: &str) -> ModelLaneResult<ModelLaneReplay> {
        require_token("run_id", run_id)?;
        let predicate = self.access.sql_predicate(2);
        let mut tx = self.pool.begin().await?;

        let run_sql = format!(
            "SELECT record_json, {RESOURCE_SCOPE_SELECT_COLUMNS} FROM model_lane_runs WHERE run_id = $1{}",
            predicate.clause()
        );
        let run_row = predicate
            .bind(sqlx::query(&run_sql).bind(run_id))
            .fetch_optional(&mut *tx)
            .await?
            // A run the reader may not see is reported as absent, not as
            // "forbidden": existence itself is metadata (HBR-PRIV-004).
            .ok_or_else(|| ModelLaneError::NotFound(format!("run_id {run_id}")))?;
        self.access
            .authorize_row(&stored_resource_scope_from_row(&run_row)?)?;
        let run: ModelLaneRunRecord = serde_json::from_value(row_to_json(run_row, "record_json")?)?;
        validate_stored_run_eventledger_authority_tx(&mut tx, &run, self.access.exact_read_scope())
            .await?;
        validate_diagnostics_row_eventledger_authority(&*self.pool, run_id)
            .await
            .map_err(|_| {
                ModelLaneError::AuthorityDenied("ModelLane navigation authority unavailable".into())
            })?;

        let lanes_sql = format!(
            "SELECT record_json, {RESOURCE_SCOPE_SELECT_COLUMNS} FROM model_lanes WHERE run_id = $1{} ORDER BY event_ledger_seq ASC",
            predicate.clause()
        );
        let lanes = self.authorize_and_decode_rows::<ModelLaneRecord>(
            predicate
                .bind(sqlx::query(&lanes_sql).bind(run_id))
                .fetch_all(&mut *tx)
                .await?,
        )?;
        for lane in &lanes {
            validate_stored_lane_eventledger_authority_tx(
                &mut tx,
                lane,
                self.access.exact_read_scope(),
            )
            .await?;
        }

        let messages_sql = format!(
            "SELECT record_json, {RESOURCE_SCOPE_SELECT_COLUMNS} FROM model_lane_messages WHERE run_id = $1{} ORDER BY event_ledger_seq ASC",
            predicate.clause()
        );
        let messages = self.authorize_and_decode_rows::<ModelLaneMessageRecord>(
            predicate
                .bind(sqlx::query(&messages_sql).bind(run_id))
                .fetch_all(&mut *tx)
                .await?,
        )?;

        for message in &messages {
            validate_stored_message_eventledger_authority_tx(
                &mut tx,
                message,
                self.access.exact_read_scope(),
            )
            .await?;
        }
        tx.commit().await?;

        Ok(ModelLaneReplay {
            run,
            lanes,
            messages,
        })
    }

    /// Second enforcement layer for a multi-row read: re-authorize every row's
    /// stored scope after the SQL predicate already filtered.
    fn authorize_and_decode_rows<T>(
        &self,
        rows: Vec<sqlx::postgres::PgRow>,
    ) -> ModelLaneResult<Vec<T>>
    where
        T: DeserializeOwned,
    {
        rows.into_iter()
            .map(|row| {
                self.access
                    .authorize_row(&stored_resource_scope_from_row(&row)?)?;
                row_to_json(row, "record_json")
                    .and_then(|value| serde_json::from_value(value).map_err(Into::into))
            })
            .collect()
    }

    /// "The newest run **this reader owns**", not "the newest run on the node".
    /// Before scoping, this handed whoever asked the globally newest run's full
    /// diagnostics projection.
    pub async fn latest_diagnostics_projection(
        &self,
    ) -> ModelLaneResult<ModelLaneDiagnosticsProjection> {
        let predicate = self.access.sql_predicate(1);
        let latest_sql = format!(
            "SELECT run_id FROM model_lane_runs WHERE TRUE{} ORDER BY event_ledger_seq DESC LIMIT 1",
            predicate.clause()
        );
        let run_id: String = predicate
            .bind_scalar(sqlx::query_scalar(&latest_sql))
            .fetch_optional(&*self.pool)
            .await?
            .ok_or_else(|| ModelLaneError::NotFound("no model lane runs recorded".into()))?;
        self.diagnostics_projection(&run_id).await
    }

    pub async fn latest_diagnostics_projection_with_model_catalog(
        &self,
        model_catalog: Option<&crate::model_runtime::ModelCatalog>,
    ) -> ModelLaneResult<ModelLaneDiagnosticsProjection> {
        let mut projection = self.latest_diagnostics_projection().await?;
        apply_diagnostics_model_catalog_labels(&mut projection, model_catalog);
        Ok(projection)
    }

    pub async fn diagnostics_projection_with_model_catalog(
        &self,
        run_id: &str,
        model_catalog: Option<&crate::model_runtime::ModelCatalog>,
    ) -> ModelLaneResult<ModelLaneDiagnosticsProjection> {
        let mut projection = self.diagnostics_projection(run_id).await?;
        apply_diagnostics_model_catalog_labels(&mut projection, model_catalog);
        Ok(projection)
    }

    pub async fn diagnostics_projection(
        &self,
        run_id: &str,
    ) -> ModelLaneResult<ModelLaneDiagnosticsProjection> {
        validate_diagnostics_row_eventledger_authority(&*self.pool, run_id).await?;
        let replay = self.replay_run(run_id).await?;
        let tier_posture = self
            .validate_diagnostic_tier_posture(run_id, "HBR-INT-009")
            .await?;
        let mt_runtime_statuses = select_records_by_column::<ModelLaneMtRuntimeStatusRecord>(
            &*self.pool,
            &self.access,
            "model_lane_mt_runtime_statuses",
            "run_id",
            run_id,
        )
        .await?;
        let leases = select_records_by_column::<ModelLaneLeaseRecord>(
            &*self.pool,
            &self.access,
            "model_lane_leases",
            "run_id",
            run_id,
        )
        .await?;
        let active_lease_count = leases
            .iter()
            .filter(|lease| lease.state == ModelLaneLeaseState::Active)
            .count();
        let reclaimable_leases = leases
            .iter()
            .filter(|lease| {
                lease.state == ModelLaneLeaseState::Active
                    && parse_utc("lease_expires_at_utc", &lease.lease_expires_at_utc)
                        .map(|expires| expires <= Utc::now())
                        .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        let reclaimable_lane_ids = reclaimable_leases
            .iter()
            .filter_map(|lease| lease.lane_id.as_ref())
            .cloned()
            .collect::<BTreeSet<_>>();
        let reclaimable_lease_ids = reclaimable_leases
            .iter()
            .map(|lease| lease.lease_id.clone())
            .collect::<Vec<_>>();
        let routing_executions =
            super::routing_execution::ModelLaneRoutingExecutionStore::new_with_access(
                self.pool.postgres_pool(),
                self.access.clone(),
            )
            .diagnostics_for_run(run_id)
            .await
            .map_err(ModelLaneError::InvalidInput)?;
        let messages_by_lane = replay.messages.iter().fold(
            BTreeMap::<String, Vec<&ModelLaneMessageRecord>>::new(),
            |mut acc, msg| {
                acc.entry(msg.from_lane_id.clone()).or_default().push(msg);
                acc
            },
        );
        let anchor_predicate = self.access.sql_predicate(2);
        let anchor_sql = format!(
            "SELECT lane_id, model_stable_anchor, {RESOURCE_SCOPE_SELECT_COLUMNS} \
             FROM model_lanes WHERE run_id = $1{}",
            anchor_predicate.clause()
        );
        let lane_anchors = anchor_predicate
            .bind(sqlx::query(&anchor_sql).bind(run_id))
            .fetch_all(&*self.pool)
            .await?
            .into_iter()
            .map(|row| {
                self.access
                    .authorize_row(&stored_resource_scope_from_row(&row)?)?;
                Ok((
                    row.try_get::<String, _>("lane_id")?,
                    row.try_get::<Option<String>, _>("model_stable_anchor")?,
                ))
            })
            .collect::<ModelLaneResult<BTreeMap<_, _>>>()?;
        let lanes = replay
            .lanes
            .iter()
            .map(|lane| {
                let lane_messages = messages_by_lane
                    .get(&lane.lane_id)
                    .cloned()
                    .unwrap_or_default();
                let payload_error_count = lane_messages
                    .iter()
                    .filter(|msg| {
                        msg.failstate_code.is_some()
                            || msg
                                .diagnostic_payload
                                .get("payload_error")
                                .and_then(Value::as_str)
                                .is_some()
                    })
                    .count();
                let last_activity_utc = lane_messages
                    .iter()
                    .map(|msg| msg.created_at_utc.clone())
                    .max()
                    .or_else(|| lane.heartbeat_at_utc.clone());
                let model_stable_anchor = lane_anchors
                    .get(&lane.lane_id)
                    .cloned()
                    .flatten();
                let model_anchor_unavailable_reason = if lane.kind == ModelLaneKind::LocalModel
                    && model_stable_anchor.is_none()
                {
                    Some(
                        "legacy ModelLane row predates persisted artifact SHA-256 anchor, or its boot UUID had no durable registry observation"
                            .to_owned(),
                    )
                } else {
                    None
                };
                ModelLaneDiagnosticsLane {
                    lane_id: lane.lane_id.clone(),
                    kind: lane.kind.as_str().to_owned(),
                    role: lane.role.clone(),
                    backend: lane.backend.clone(),
                    status: lane.status.as_str().to_owned(),
                    recovery_state: lane.recovery_state.as_str().to_owned(),
                    model_id: lane.model_id.clone(),
                    model_display_name: crate::model_runtime::UNKNOWN_MODEL_LABEL.to_owned(),
                    model_stable_anchor,
                    model_anchor_unavailable_reason,
                    session_id: lane.session_id.clone(),
                    model_session_id: lane.model_session_id.clone(),
                    adapter_id: lane.adapter_id.clone(),
                    provider_kind: lane.provider_kind.as_str().to_owned(),
                    runtime_binding: lane.runtime_binding.as_str().to_owned(),
                    launch_authority: lane.launch_authority.as_str().to_owned(),
                    capability_token_ids: lane.capability_token_ids.clone(),
                    effective_capability_snapshot_ref: lane
                        .effective_capability_snapshot_ref
                        .clone(),
                    capability_negotiation_ref: lane.capability_negotiation_ref.clone(),
                    provider_feature_profile_ref: lane.provider_feature_profile_ref.clone(),
                    requested_execution_policy_ref: lane.requested_execution_policy_ref.clone(),
                    effective_execution_policy_ref: lane.effective_execution_policy_ref.clone(),
                    projection_plan_ref: lane.projection_plan_ref.clone(),
                    consent_receipt_ref: lane.consent_receipt_ref.clone(),
                    tool_gate_decision_refs: lane.tool_gate_decision_refs.clone(),
                    trace_id: lane.trace_id.clone(),
                    lane_span_id: lane.lane_span_id.clone(),
                    event_ledger_event_id: lane.event_ledger_event_id.clone(),
                    event_ledger_seq: lane.event_ledger_seq,
                    flight_recorder_correlation_id: lane.event_ledger_event_id.clone(),
                    last_activity_utc,
                    message_count: lane_messages.len(),
                    payload_error_count,
                    orphan_state: if reclaimable_lane_ids.contains(&lane.lane_id) {
                        "reclaimable"
                    } else {
                        "none"
                    }
                    .to_owned(),
                    cancellation_ref: lane.cancellation_ref.clone(),
                    reclaim_policy_ref: lane.reclaim_policy_ref.clone(),
                    terminal_status_mapping_ref: lane.terminal_status_mapping_ref.clone(),
                    process_ownership_ref: lane.process_ownership_ref.clone(),
                    no_os_process_reason_ref: lane.no_os_process_reason_ref.clone(),
                    last_runtime_status_ref: lane.last_runtime_status_ref.clone(),
                    last_recovery_event_ref: lane.last_recovery_event_ref.clone(),
                    failstate_code: lane.failstate_code.clone(),
                    startup_failure_ref: lane.startup_failure_ref.clone(),
                    reason_ref: lane.reason_ref.clone(),
                    recovery_hint_ref: lane.recovery_hint_ref.clone(),
                    work_packet_id: lane.work_packet_id.clone(),
                    micro_task_id: lane.micro_task_id.clone(),
                    task_board_id: lane.task_board_id.clone(),
                    owner_session: lane.owner_session.clone(),
                    locus_ref: lane
                        .locus_binding
                        .as_ref()
                        .map(|binding| binding.locus_binding_ref.clone()),
                }
            })
            .collect::<Vec<_>>();
        let messages = replay
            .messages
            .iter()
            .map(|message| ModelLaneDiagnosticsMessage {
                message_id: message.message_id.clone(),
                from_lane_id: message.from_lane_id.clone(),
                to_lane: model_lane_target_label(&message.to_lane),
                routing_target_role: message
                    .routing
                    .as_ref()
                    .map(|routing| routing.target_role.clone()),
                routing_target_session: message
                    .routing
                    .as_ref()
                    .map(|routing| routing.target_session.clone()),
                routing_correlation_id: message
                    .routing
                    .as_ref()
                    .map(|routing| routing.correlation_id.clone()),
                routing_requires_ack: message
                    .routing
                    .as_ref()
                    .map(|routing| routing.requires_ack)
                    .unwrap_or(false),
                routing_ack_for: message
                    .routing
                    .as_ref()
                    .and_then(|routing| routing.ack_for.clone()),
                kind: message.kind.as_str().to_owned(),
                authority: message.authority.as_str().to_owned(),
                promotion_state: message
                    .promotion_decision_id
                    .as_ref()
                    .map(|_| "decision_recorded")
                    .unwrap_or_else(|| message.authority.as_str())
                    .to_owned(),
                payload_ref: message.payload_ref.clone(),
                payload_sha256: message.payload_sha256.clone(),
                artifact_ref: message
                    .promoted_artifact_ref
                    .clone()
                    .or_else(|| json_string(&message.diagnostic_payload, "artifact_ref")),
                promotion_decision_id: message.promotion_decision_id.clone(),
                promotion_gate_ref: message.promotion_gate_ref.clone(),
                promotion_receipt_ref: message.promotion_receipt_ref.clone(),
                validator_verdict_ref: message.validator_verdict_ref.clone(),
                operator_decision_ref: message.operator_decision_ref.clone(),
                promoted_artifact_sha256: message.promoted_artifact_sha256.clone(),
                promoted_artifact_version: message.promoted_artifact_version.clone(),
                tool_gate_decision_refs: message.tool_gate_decision_refs.clone(),
                coordinator_session_id: message.coordinator_session_id.clone(),
                work_packet_id: message.work_packet_id.clone(),
                micro_task_id: message.micro_task_id.clone(),
                task_board_id: message.task_board_id.clone(),
                owner_session: message.owner_session.clone(),
                trace_id: message.trace_id.clone(),
                message_span_id: message.message_span_id.clone(),
                parent_span_id: message.parent_span_id.clone(),
                linked_span_contexts: message.linked_span_contexts.clone(),
                event_ledger_event_id: message.event_ledger_event_id.clone(),
                event_ledger_seq: message.event_ledger_seq,
                flight_recorder_correlation_id: message.event_ledger_event_id.clone(),
                locus_ref: message
                    .locus_binding
                    .as_ref()
                    .map(|binding| binding.locus_binding_ref.clone())
                    .or_else(|| json_string(&message.diagnostic_payload, "locus_ref")),
                loom_ref: json_string(&message.diagnostic_payload, "loom_ref"),
                fems_ref: json_string(&message.diagnostic_payload, "fems_ref"),
                proposal_ref: message.proposal_ref.clone(),
                crdt_update_ref: message.crdt_update_ref.clone(),
                crdt_base_snapshot_ref: message.crdt_base_snapshot_ref.clone(),
                crdt_state_vector: message.crdt_state_vector.clone(),
                crdt_proposal_ref: message.crdt_proposal_ref.clone(),
                crdt_stale_base_ref: message.crdt_stale_base_ref.clone(),
                payload_error: message
                    .failstate_code
                    .clone()
                    .or_else(|| json_string(&message.diagnostic_payload, "payload_error")),
                reason_ref: message.reason_ref.clone(),
                recovery_hint_ref: message.recovery_hint_ref.clone(),
                created_at_utc: message.created_at_utc.clone(),
            })
            .collect::<Vec<_>>();

        Ok(ModelLaneDiagnosticsProjection {
            schema_id: MODEL_LANE_DIAGNOSTICS_PROJECTION_SCHEMA_ID.to_owned(),
            surface_contract_id: MODEL_LANE_DIAGNOSTICS_SURFACE_CONTRACT_ID.to_owned(),
            run: ModelLaneDiagnosticsRun {
                run_id: replay.run.run_id.clone(),
                trace_id: replay.run.trace_id.clone(),
                run_span_id: replay.run.run_span_id.clone(),
                coordinator_session_id: replay.run.coordinator_session_id.clone(),
                routing_policy: replay.run.routing_policy.clone(),
                artifact_namespace: replay.run.artifact_namespace.clone(),
                projection_plan_ref: replay.run.projection_plan_ref.clone(),
                consent_receipt_ref: replay.run.consent_receipt_ref.clone(),
                work_packet_id: replay.run.work_packet_id.clone(),
                micro_task_id: replay.run.micro_task_id.clone(),
                task_board_id: replay.run.task_board_id.clone(),
                owner_session: replay.run.owner_session.clone(),
                event_ledger_event_id: replay.run.event_ledger_event_id.clone(),
                event_ledger_seq: replay.run.event_ledger_seq,
                flight_recorder_correlation_id: replay.run.event_ledger_event_id.clone(),
                context_bundle_id: replay.run.context_bundle_id.clone(),
                memory_pack_ref: replay.run.memory_pack_ref.clone(),
                memory_pack_hash: replay.run.memory_pack_hash.clone(),
                locus_ref: replay
                    .run
                    .locus_binding
                    .as_ref()
                    .map(|binding| binding.locus_binding_ref.clone()),
                loom_ref: None,
                fems_ref: None,
                status: replay.run.recovery_state.as_str().to_owned(),
                recovery_hint_ref: replay.run.recovery_hint_ref.clone(),
                selected_model_id: replay.run.selected_model_id.clone(),
                candidate_model_ids: replay.run.candidate_model_ids.clone(),
                budget_summary_ref: replay.run.budget_summary_ref.clone(),
                determinism_mode: replay.run.determinism_mode.clone(),
            },
            lanes,
            messages,
            diagnostic_tiers: tier_posture
                .tiers
                .into_iter()
                .map(|tier| ModelLaneDiagnosticsTier {
                    tier: tier.tier.as_str().to_owned(),
                    state: tier.state.as_str().to_owned(),
                    reason: tier.reason.clone(),
                    evidence_ref: tier.evidence_ref.clone(),
                    follow_up_ref: tier.follow_up_ref.clone(),
                })
                .collect(),
            mt_runtime_statuses: mt_runtime_statuses
                .into_iter()
                .map(|status| ModelLaneDiagnosticsMtStatus {
                    micro_task_id: status.micro_task_id.clone(),
                    status: status.status.as_str().to_owned(),
                    proof_status_ref: status.proof_status_ref.clone(),
                    hbr_status_ref: status.hbr_status_ref.clone(),
                    event_ledger_event_id: status.event_ledger_event_id.clone(),
                    event_ledger_seq: status.event_ledger_seq,
                })
                .collect(),
            routing_executions,
            active_lease_count,
            orphan_state: if reclaimable_lease_ids.is_empty() {
                "none".to_owned()
            } else {
                "reclaimable".to_owned()
            },
            reclaimable_lease_ids,
        })
    }

    pub async fn navigation_by_run(
        &self,
        run_id: &str,
    ) -> ModelLaneResult<ModelLaneNavigationProjection> {
        self.navigation_projection_for_run("model_lane.navigation.run", "run", run_id, run_id)
            .await
    }

    #[cfg(feature = "test-utils")]
    pub async fn test_cloud_schema_state(&self) -> ModelLaneResult<(String, i64, String)> {
        let state = self
            .cloud_authority()
            .await?
            .schema_state()
            .await?
            .ok_or_else(|| {
                ModelLaneError::IntegrityViolation(
                    "MT-006 cloud authority schema state is missing".into(),
                )
            })?;
        Ok((
            state.schema_version,
            state.schema_revision,
            state.apply_state,
        ))
    }

    pub async fn navigation_by_lane(
        &self,
        lane_id: &str,
    ) -> ModelLaneResult<ModelLaneNavigationProjection> {
        require_token("lane_id", lane_id)?;
        let lane = validated_navigation_origins(
            &*self.pool,
            &self.access,
            "lane",
            "lane_id = $1",
            lane_id,
        )
        .await?
        .into_iter()
        .next()
        .and_then(|origin| match origin {
            ValidatedNavigationOrigin::Lane(record) => Some(record),
            _ => None,
        })
        .ok_or_else(|| ModelLaneError::NotFound(format!("lane_id {lane_id}")))?;
        let mut projection = self
            .navigation_projection_for_run(
                "model_lane.navigation.lane",
                "lane",
                lane_id,
                &lane.run_id,
            )
            .await?;
        projection.lanes.retain(|row| row.lane_id == lane_id);
        projection
            .messages
            .retain(|row| message_mentions_lane(row, lane_id));
        projection
            .recovery_checkpoints
            .retain(|row| row.lane_id.as_deref() == Some(lane_id));
        projection
            .recovery_events
            .retain(|row| row.lane_id.as_deref() == Some(lane_id));
        projection
            .leases
            .retain(|row| row.lane_id.as_deref() == Some(lane_id));
        projection.rebuild_navigation_evidence();
        Ok(projection)
    }

    pub async fn navigation_by_message(
        &self,
        message_id: &str,
    ) -> ModelLaneResult<ModelLaneNavigationProjection> {
        require_token("message_id", message_id)?;
        let message = validated_navigation_origins(
            &*self.pool,
            &self.access,
            "message",
            "message_id = $1",
            message_id,
        )
        .await?
        .into_iter()
        .next()
        .and_then(|origin| match origin {
            ValidatedNavigationOrigin::Message(record) => Some(record),
            _ => None,
        })
        .ok_or_else(|| ModelLaneError::NotFound(format!("message_id {message_id}")))?;
        let mut projection = self
            .navigation_projection_for_run(
                "model_lane.navigation.message",
                "message",
                message_id,
                &message.run_id,
            )
            .await?;
        projection
            .messages
            .retain(|row| row.message_id == message_id);
        projection
            .lanes
            .retain(|row| message_mentions_lane(&message, &row.lane_id));
        projection.artifacts.retain(|row| {
            row.artifact_ref == message.payload_ref
                || row.artifact_payload_ref == message.payload_ref
                || row.artifact_sha256 == message.payload_sha256
                || row.content_hash == message.payload_sha256
        });
        projection
            .context_handoffs
            .retain(|row| row.source_message_id == message_id);
        projection.rebuild_navigation_evidence();
        Ok(projection)
    }

    pub async fn navigation_by_artifact_or_context(
        &self,
        artifact_ref: Option<&str>,
        context_bundle_id: Option<&str>,
        run_id: Option<&str>,
    ) -> ModelLaneResult<ModelLaneNavigationProjection> {
        if artifact_ref.is_none() && context_bundle_id.is_none() {
            return Err(ModelLaneError::InvalidInput(
                "artifact_ref or context_bundle_id is required".into(),
            ));
        }
        if let Some(value) = artifact_ref {
            require_token("artifact_ref", value)?;
        }
        if let Some(value) = context_bundle_id {
            require_token("context_bundle_id", value)?;
        }
        if let Some(value) = run_id {
            require_token("run_id", value)?;
        }

        let artifacts = match artifact_ref {
            Some(value) => self.context_artifacts_by_ref(value).await?,
            None => Vec::new(),
        };
        let mut handoffs = match context_bundle_id {
            Some(value) => self.context_handoffs_by_context(value).await?,
            None => Vec::new(),
        };
        if let Some(value) = artifact_ref {
            handoffs.extend(self.context_handoffs_by_artifact_ref(value).await?);
        }
        dedupe_context_handoffs(&mut handoffs);
        let context_run = if let Some(value) = context_bundle_id {
            validated_navigation_origins(
                &*self.pool,
                &self.access,
                "run",
                "record_json->>'context_bundle_id' = $1",
                value,
            )
            .await?
            .into_iter()
            .next()
            .and_then(|origin| match origin {
                ValidatedNavigationOrigin::Run(record) => Some(record),
                _ => None,
            })
        } else {
            None
        };

        let derived_run_id = if let Some(value) = run_id {
            value.to_owned()
        } else {
            let mut run_ids = artifacts
                .iter()
                .map(|row| row.run_id.clone())
                .collect::<Vec<_>>();
            run_ids.extend(handoffs.iter().map(|row| row.run_id.clone()));
            if let Some(row) = context_run.as_ref() {
                run_ids.push(row.run_id.clone());
            }
            unique_run_id_for_lookup(
                "artifact_context",
                artifact_ref
                    .or(context_bundle_id)
                    .unwrap_or("artifact_context"),
                run_ids,
            )?
            .ok_or_else(|| {
                ModelLaneError::NotFound(format!(
                    "artifact_ref {:?} context_bundle_id {:?}",
                    artifact_ref, context_bundle_id
                ))
            })?
        };
        let mut projection = self
            .navigation_projection_for_run(
                "model_lane.navigation.artifact_context",
                "artifact_context",
                artifact_ref
                    .or(context_bundle_id)
                    .unwrap_or("artifact_context"),
                &derived_run_id,
            )
            .await?;
        if let Some(value) = artifact_ref {
            projection
                .artifacts
                .retain(|row| artifact_matches(row, value));
            projection.context_handoffs.retain(|row| {
                row.artifact_ref == value
                    || row.artifact_sha256 == value
                    || row.content_hash == value
            });
            let artifact_message_refs: BTreeSet<String> = projection
                .artifacts
                .iter()
                .flat_map(|artifact| {
                    [
                        artifact.artifact_ref.as_str(),
                        artifact.artifact_manifest_ref.as_str(),
                        artifact.artifact_payload_ref.as_str(),
                        artifact.artifact_sha256.as_str(),
                        artifact.content_hash.as_str(),
                    ]
                })
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .collect();
            projection.messages.retain(|row| {
                artifact_message_refs.contains(&row.payload_ref)
                    || artifact_message_refs.contains(&row.payload_sha256)
                    || row.payload_ref == value
                    || row.payload_sha256 == value
            });
        }
        if let Some(value) = context_bundle_id {
            projection
                .context_handoffs
                .retain(|row| row.context_bundle_id == value);
        }
        let artifact_matched = artifact_ref.is_none()
            || !projection.artifacts.is_empty()
            || !projection.context_handoffs.is_empty()
            || !projection.messages.is_empty();
        let context_matched = context_bundle_id.is_none()
            || context_bundle_id.is_some_and(|value| {
                projection
                    .run
                    .as_ref()
                    .is_some_and(|row| row.context_bundle_id == value)
            })
            || !projection.context_handoffs.is_empty();
        if !artifact_matched || !context_matched {
            return Err(ModelLaneError::NotFound(format!(
                "artifact_ref {:?} context_bundle_id {:?} run_id {:?}",
                artifact_ref, context_bundle_id, run_id
            )));
        }
        projection.rebuild_navigation_evidence();
        Ok(projection)
    }

    pub async fn navigation_by_trace(
        &self,
        trace_id: &str,
        span_id: Option<&str>,
    ) -> ModelLaneResult<ModelLaneNavigationProjection> {
        require_token("trace_id", trace_id)?;
        if let Some(value) = span_id {
            require_token("span_id", value)?;
        }
        let run = validated_navigation_origins(
            &*self.pool,
            &self.access,
            "run",
            "trace_id = $1",
            trace_id,
        )
        .await?
        .into_iter()
        .next();
        let run_id = if let Some(run) = run {
            run.run_id().to_owned()
        } else if let Some(lane) = validated_navigation_origins(
            &*self.pool,
            &self.access,
            "lane",
            "trace_id = $1",
            trace_id,
        )
        .await?
        .into_iter()
        .next()
        {
            lane.run_id().to_owned()
        } else if let Some(message) = validated_navigation_origins(
            &*self.pool,
            &self.access,
            "message",
            "trace_id = $1",
            trace_id,
        )
        .await?
        .into_iter()
        .next()
        {
            message.run_id().to_owned()
        } else {
            return Err(ModelLaneError::NotFound(format!("trace_id {trace_id}")));
        };
        let mut projection = self
            .navigation_projection_for_run(
                "model_lane.navigation.trace_span",
                "trace_span",
                span_id.unwrap_or(trace_id),
                &run_id,
            )
            .await?;
        projection.run = projection
            .run
            .filter(|row| row.trace_id == trace_id && span_matches(span_id, &row.run_span_id));
        projection
            .lanes
            .retain(|row| row.trace_id == trace_id && span_matches(span_id, &row.lane_span_id));
        projection.messages.retain(|row| {
            row.trace_id == trace_id
                && (span_matches(span_id, &row.message_span_id)
                    || row.parent_span_id.as_deref() == span_id
                    || row
                        .linked_span_contexts
                        .iter()
                        .any(|linked| Some(linked.as_str()) == span_id))
        });
        projection
            .context_handoffs
            .retain(|row| row.trace_id == trace_id && span_matches(span_id, &row.handoff_span_id));
        projection
            .recovery_events
            .retain(|row| row.trace_id == trace_id && span_matches(span_id, &row.span_id));
        projection.rebuild_navigation_evidence();
        Ok(projection)
    }

    pub async fn navigation_by_diagnostics(
        &self,
        run_id: &str,
        behavior_id: Option<&str>,
        tier: Option<&str>,
        mt_id: Option<&str>,
    ) -> ModelLaneResult<ModelLaneNavigationProjection> {
        require_token("run_id", run_id)?;
        let mut projection = self
            .navigation_projection_for_run(
                "model_lane.navigation.diagnostic_tier",
                "diagnostic_tier",
                behavior_id.or(tier).or(mt_id).unwrap_or(run_id),
                run_id,
            )
            .await?;
        if let Some(value) = behavior_id {
            projection
                .diagnostic_tiers
                .retain(|row| row.behavior_id == value);
        }
        if let Some(value) = tier {
            projection
                .diagnostic_tiers
                .retain(|row| row.tier.as_str() == value);
        }
        if let Some(value) = mt_id {
            projection
                .mt_runtime_statuses
                .retain(|row| row.micro_task_id == value);
        }
        projection.rebuild_navigation_evidence();
        Ok(projection)
    }

    pub async fn navigation_by_recovery(
        &self,
        run_id: &str,
    ) -> ModelLaneResult<ModelLaneNavigationProjection> {
        self.navigation_projection_for_run(
            "model_lane.navigation.recovery",
            "recovery",
            run_id,
            run_id,
        )
        .await
    }

    pub async fn navigation_by_lookup(
        &self,
        lookup: ModelLaneNavigationLookup,
    ) -> ModelLaneResult<ModelLaneNavigationProjection> {
        let (lookup_kind, lookup_ref, run_id) = self.resolve_navigation_lookup(lookup).await?;
        self.navigation_projection_for_run(
            "model_lane.navigation.lookup",
            &lookup_kind,
            &lookup_ref,
            &run_id,
        )
        .await
    }

    async fn resolve_navigation_lookup(
        &self,
        lookup: ModelLaneNavigationLookup,
    ) -> ModelLaneResult<(String, String, String)> {
        let requested = lookup.requested()?;
        let (lookup_kind, lookup_ref) = requested;
        let run_id = match lookup_kind.as_str() {
            "run_id" => lookup_ref.clone(),
            "lane_id" => validated_navigation_origins(
                &*self.pool,
                &self.access,
                "lane",
                "lane_id = $1",
                &lookup_ref,
            )
            .await?
            .into_iter()
            .next()
            .map(|row| row.run_id().to_owned())
            .ok_or_else(|| ModelLaneError::NotFound(format!("lane_id {lookup_ref}")))?,
            "message_id" => validated_navigation_origins(
                &*self.pool,
                &self.access,
                "message",
                "message_id = $1",
                &lookup_ref,
            )
            .await?
            .into_iter()
            .next()
            .map(|row| row.run_id().to_owned())
            .ok_or_else(|| ModelLaneError::NotFound(format!("message_id {lookup_ref}")))?,
            "model_session_id" => self
                .run_id_by_model_session_id(&lookup_ref)
                .await?
                .ok_or_else(|| {
                    ModelLaneError::NotFound(format!("model_session_id {lookup_ref}"))
                })?,
            "session_id" => self
                .run_id_by_session_id(&lookup_ref)
                .await?
                .ok_or_else(|| ModelLaneError::NotFound(format!("session_id {lookup_ref}")))?,
            "wp_id" | "work_packet_id" => self
                .run_id_by_work_packet_id(&lookup_ref)
                .await?
                .ok_or_else(|| ModelLaneError::NotFound(format!("wp_id {lookup_ref}")))?,
            "mt_id" | "micro_task_id" => self
                .run_id_by_micro_task_id(&lookup_ref)
                .await?
                .ok_or_else(|| ModelLaneError::NotFound(format!("mt_id {lookup_ref}")))?,
            "task_board_id" => self
                .run_id_by_task_board_id(&lookup_ref)
                .await?
                .ok_or_else(|| ModelLaneError::NotFound(format!("task_board_id {lookup_ref}")))?,
            "artifact_ref" => self
                .run_id_by_artifact_ref(&lookup_ref)
                .await?
                .ok_or_else(|| ModelLaneError::NotFound(format!("artifact_ref {lookup_ref}")))?,
            "context_bundle_id" => self
                .run_id_by_context_bundle_id(&lookup_ref)
                .await?
                .ok_or_else(|| {
                    ModelLaneError::NotFound(format!("context_bundle_id {lookup_ref}"))
                })?,
            "locus_ref" | "locus_binding_ref" => self
                .run_id_by_locus_ref(&lookup_ref)
                .await?
                .ok_or_else(|| ModelLaneError::NotFound(format!("locus_ref {lookup_ref}")))?,
            "loom_ref" => self
                .run_id_by_diagnostic_payload_ref(&lookup_ref, &["loom_ref", "loom_block_id"])
                .await?
                .ok_or_else(|| ModelLaneError::NotFound(format!("loom_ref {lookup_ref}")))?,
            "loom_block_id" => self
                .run_id_by_loom_block_id(&lookup_ref)
                .await?
                .ok_or_else(|| ModelLaneError::NotFound(format!("loom_block_id {lookup_ref}")))?,
            "fems_ref" => self
                .run_id_by_diagnostic_payload_ref(&lookup_ref, &["fems_ref", "memory_pack_ref"])
                .await?
                .ok_or_else(|| ModelLaneError::NotFound(format!("fems_ref {lookup_ref}")))?,
            "memory_pack_ref" => self
                .run_id_by_memory_pack_ref(&lookup_ref)
                .await?
                .ok_or_else(|| ModelLaneError::NotFound(format!("memory_pack_ref {lookup_ref}")))?,
            "memory_pack_hash" => self
                .run_id_by_memory_pack_hash(&lookup_ref)
                .await?
                .ok_or_else(|| {
                    ModelLaneError::NotFound(format!("memory_pack_hash {lookup_ref}"))
                })?,
            "event_ledger_event_id" => self
                .run_id_by_event_ledger_event_id(&lookup_ref)
                .await?
                .ok_or_else(|| {
                    ModelLaneError::NotFound(format!("event_ledger_event_id {lookup_ref}"))
                })?,
            "event_ledger_seq" => {
                let seq = lookup_ref.parse::<i64>().map_err(|err| {
                    ModelLaneError::InvalidInput(format!("event_ledger_seq must be i64: {err}"))
                })?;
                self.run_id_by_event_ledger_seq(seq).await?.ok_or_else(|| {
                    ModelLaneError::NotFound(format!("event_ledger_seq {lookup_ref}"))
                })?
            }
            "trace_id" => self
                .run_id_by_trace_id(&lookup_ref)
                .await?
                .ok_or_else(|| ModelLaneError::NotFound(format!("trace_id {lookup_ref}")))?,
            "span_id" => self
                .run_id_by_span_id(&lookup_ref)
                .await?
                .ok_or_else(|| ModelLaneError::NotFound(format!("span_id {lookup_ref}")))?,
            "error_code" => self
                .run_id_by_error_code(&lookup_ref)
                .await?
                .ok_or_else(|| ModelLaneError::NotFound(format!("error_code {lookup_ref}")))?,
            other => {
                return Err(ModelLaneError::InvalidInput(format!(
                    "unsupported ModelLane navigation lookup kind {other}"
                )));
            }
        };
        Ok((lookup_kind, lookup_ref, run_id))
    }

    async fn run_id_by_model_session_id(&self, value: &str) -> ModelLaneResult<Option<String>> {
        // `model_lanes` stores model_session_id only inside record_json; the
        // recovery tables carry it as physical columns.
        if let Some(row) = validated_navigation_origins(
            &*self.pool,
            &self.access,
            "lane",
            "record_json->>'model_session_id' = $1",
            value,
        )
        .await?
        .into_iter()
        .next()
        {
            return Ok(Some(row.run_id().to_owned()));
        }
        if let Some(row) = select_record_by_column::<ModelLaneRecoveryEventRecord>(
            &*self.pool,
            &self.access,
            "model_lane_recovery_events",
            "model_session_id",
            value,
        )
        .await?
        {
            return Ok(Some(row.run_id.clone()));
        }
        select_record_by_column::<ModelLaneRecoveryCheckpointRecord>(
            &*self.pool,
            &self.access,
            "model_lane_recovery_checkpoints",
            "model_session_id",
            value,
        )
        .await
        .map(|row| row.map(|row| row.run_id.clone()))
    }

    async fn run_id_by_session_id(&self, value: &str) -> ModelLaneResult<Option<String>> {
        // `model_lanes` stores session_id only inside record_json; the recovery
        // tables carry it as physical columns.
        if let Some(row) = validated_navigation_origins(
            &*self.pool,
            &self.access,
            "lane",
            "record_json->>'session_id' = $1",
            value,
        )
        .await?
        .into_iter()
        .next()
        {
            return Ok(Some(row.run_id().to_owned()));
        }
        if let Some(row) = select_record_by_column::<ModelLaneRecoveryEventRecord>(
            &*self.pool,
            &self.access,
            "model_lane_recovery_events",
            "session_id",
            value,
        )
        .await?
        {
            return Ok(Some(row.run_id.clone()));
        }
        select_record_by_column::<ModelLaneRecoveryCheckpointRecord>(
            &*self.pool,
            &self.access,
            "model_lane_recovery_checkpoints",
            "session_id",
            value,
        )
        .await
        .map(|row| row.map(|row| row.run_id.clone()))
    }

    async fn run_id_by_work_packet_id(&self, value: &str) -> ModelLaneResult<Option<String>> {
        let run_ids = validated_navigation_run_ids(
            &*self.pool,
            &self.access,
            "run",
            "work_packet_id = $1",
            value,
        )
        .await?;
        unique_run_id_for_lookup("wp_id", value, run_ids)
    }

    async fn run_id_by_micro_task_id(&self, value: &str) -> ModelLaneResult<Option<String>> {
        let mut run_ids = validated_navigation_run_ids(
            &*self.pool,
            &self.access,
            "run",
            "micro_task_id = $1",
            value,
        )
        .await?;
        run_ids.extend(
            select_run_ids_by_column(
                &*self.pool,
                &self.access,
                "model_lane_mt_runtime_statuses",
                "micro_task_id",
                value,
            )
            .await?,
        );
        unique_run_id_for_lookup("mt_id", value, run_ids)
    }

    async fn run_id_by_task_board_id(&self, value: &str) -> ModelLaneResult<Option<String>> {
        let mut run_ids = validated_navigation_run_ids(
            &*self.pool,
            &self.access,
            "run",
            "task_board_id = $1",
            value,
        )
        .await?;
        run_ids.extend(
            select_run_ids_by_column(
                &*self.pool,
                &self.access,
                "model_lane_mt_runtime_statuses",
                "task_board_id",
                value,
            )
            .await?,
        );
        unique_run_id_for_lookup("task_board_id", value, run_ids)
    }

    async fn run_id_by_artifact_ref(&self, value: &str) -> ModelLaneResult<Option<String>> {
        let mut run_ids = select_records_by_any_artifact_ref(&*self.pool, &self.access, value)
            .await?
            .into_iter()
            // MT-003 unblock (out-of-scope, pre-existing WIP commit 0adac5d8):
            // `select_records_by_any_artifact_ref` yields borrowed rows, so
            // `run_id` (String, not Copy) must be cloned out. Compiler-suggested
            // fix; behavior-preserving.
            .map(|row| row.run_id.clone())
            .collect::<Vec<_>>();
        // `payload_ref` is stored only inside record_json; `payload_sha256` is a
        // physical column on model_lane_messages.
        run_ids.extend(
            validated_navigation_run_ids(
                &*self.pool,
                &self.access,
                "message",
                "record_json->>'payload_ref' = $1",
                value,
            )
            .await?,
        );
        run_ids.extend(
            validated_navigation_run_ids(
                &*self.pool,
                &self.access,
                "message",
                "payload_sha256 = $1",
                value,
            )
            .await?,
        );
        unique_run_id_for_lookup("artifact_ref", value, run_ids)
    }

    async fn run_id_by_context_bundle_id(&self, value: &str) -> ModelLaneResult<Option<String>> {
        // `model_lane_runs` carries context_bundle_id only inside record_json; the
        // handoff table exposes it as a physical column.
        let mut run_ids = validated_navigation_run_ids(
            &*self.pool,
            &self.access,
            "run",
            "record_json->>'context_bundle_id' = $1",
            value,
        )
        .await?;
        run_ids.extend(
            select_run_ids_by_column(
                &*self.pool,
                &self.access,
                "model_lane_context_bundle_handoffs",
                "context_bundle_id",
                value,
            )
            .await?,
        );
        unique_run_id_for_lookup("context_bundle_id", value, run_ids)
    }

    async fn run_id_by_memory_pack_ref(&self, value: &str) -> ModelLaneResult<Option<String>> {
        let run_ids = validated_navigation_run_ids(
            &*self.pool,
            &self.access,
            "run",
            "record_json->>'memory_pack_ref' = $1",
            value,
        )
        .await?;
        unique_run_id_for_lookup("memory_pack_ref", value, run_ids)
    }

    async fn run_id_by_memory_pack_hash(&self, value: &str) -> ModelLaneResult<Option<String>> {
        let run_ids = validated_navigation_run_ids(
            &*self.pool,
            &self.access,
            "run",
            "record_json->>'memory_pack_hash' = $1",
            value,
        )
        .await?;
        unique_run_id_for_lookup("memory_pack_hash", value, run_ids)
    }

    async fn run_id_by_trace_id(&self, value: &str) -> ModelLaneResult<Option<String>> {
        if let Some(row) =
            validated_navigation_origins(&*self.pool, &self.access, "run", "trace_id = $1", value)
                .await?
                .into_iter()
                .next()
        {
            return Ok(Some(row.run_id().to_owned()));
        }
        if let Some(row) =
            validated_navigation_origins(&*self.pool, &self.access, "lane", "trace_id = $1", value)
                .await?
                .into_iter()
                .next()
        {
            return Ok(Some(row.run_id().to_owned()));
        }
        validated_navigation_origins(&*self.pool, &self.access, "message", "trace_id = $1", value)
            .await
            .map(|rows| rows.into_iter().next().map(|row| row.run_id().to_owned()))
    }

    async fn run_id_by_span_id(&self, value: &str) -> ModelLaneResult<Option<String>> {
        if let Some(row) = validated_navigation_origins(
            &*self.pool,
            &self.access,
            "run",
            "run_span_id = $1",
            value,
        )
        .await?
        .into_iter()
        .next()
        {
            return Ok(Some(row.run_id().to_owned()));
        }
        if let Some(row) = validated_navigation_origins(
            &*self.pool,
            &self.access,
            "lane",
            "lane_span_id = $1",
            value,
        )
        .await?
        .into_iter()
        .next()
        {
            return Ok(Some(row.run_id().to_owned()));
        }
        if let Some(row) = validated_navigation_origins(
            &*self.pool,
            &self.access,
            "message",
            "message_span_id = $1",
            value,
        )
        .await?
        .into_iter()
        .next()
        {
            return Ok(Some(row.run_id().to_owned()));
        }
        select_record_by_column::<ModelLaneRecoveryEventRecord>(
            &*self.pool,
            &self.access,
            "model_lane_recovery_events",
            "span_id",
            value,
        )
        .await
        .map(|row| row.map(|row| row.run_id.clone()))
    }

    async fn run_id_by_error_code(&self, value: &str) -> ModelLaneResult<Option<String>> {
        // `error_code` is a physical column on model_lane_recovery_events, but the
        // run/lane failstate_code lives only inside record_json.
        let mut run_ids = select_run_ids_by_column(
            &*self.pool,
            &self.access,
            "model_lane_recovery_events",
            "error_code",
            value,
        )
        .await?;
        run_ids.extend(
            validated_navigation_run_ids(
                &*self.pool,
                &self.access,
                "run",
                "record_json->>'failstate_code' = $1",
                value,
            )
            .await?,
        );
        run_ids.extend(
            validated_navigation_run_ids(
                &*self.pool,
                &self.access,
                "lane",
                "record_json->>'failstate_code' = $1",
                value,
            )
            .await?,
        );
        unique_run_id_for_lookup("error_code", value, run_ids)
    }

    async fn run_id_by_locus_ref(&self, value: &str) -> ModelLaneResult<Option<String>> {
        let mut run_ids = validated_navigation_run_ids(
            &*self.pool,
            &self.access,
            "run",
            "record_json #>> '{locus_binding,locus_binding_ref}' = $1",
            value,
        )
        .await?;
        run_ids.extend(
            validated_navigation_run_ids(
                &*self.pool,
                &self.access,
                "lane",
                "record_json #>> '{locus_binding,locus_binding_ref}' = $1",
                value,
            )
            .await?,
        );
        run_ids.extend(
            validated_navigation_run_ids(
                &*self.pool,
                &self.access,
                "message",
                "record_json #>> '{locus_binding,locus_binding_ref}' = $1 OR record_json #>> '{diagnostic_payload,locus_ref}' = $1",
                value,
            )
            .await?,
        );
        unique_run_id_for_lookup("locus_ref", value, run_ids)
    }

    async fn run_id_by_loom_block_id(&self, value: &str) -> ModelLaneResult<Option<String>> {
        let mut run_ids = validated_navigation_handoff_run_ids(
            &*self.pool,
            &self.access,
            "EXISTS (
                SELECT 1
                FROM jsonb_array_elements(COALESCE(record_json->'loom_refs', '[]'::jsonb)) AS loom_ref
                WHERE loom_ref->>'block_id' = $1
            )",
            value,
        )
        .await?;
        run_ids.extend(
            validated_navigation_run_ids(
                &*self.pool,
                &self.access,
                "message",
                "record_json #>> '{diagnostic_payload,loom_block_id}' = $1",
                value,
            )
            .await?,
        );
        run_ids.extend(
            validated_navigation_handoff_run_ids(
                &*self.pool,
                &self.access,
                "record_json #>> '{diagnostic_payload,loom_block_id}' = $1",
                value,
            )
            .await?,
        );
        unique_run_id_for_lookup("loom_block_id", value, run_ids)
    }

    async fn run_id_by_diagnostic_payload_ref(
        &self,
        value: &str,
        keys: &[&str],
    ) -> ModelLaneResult<Option<String>> {
        for key in keys {
            let condition = format!("record_json #>> '{{diagnostic_payload,{key}}}' = $1");
            let run_ids = validated_navigation_run_ids(
                &*self.pool,
                &self.access,
                "message",
                &condition,
                value,
            )
            .await?;
            if let Some(run_id) =
                unique_run_id_for_lookup("diagnostic_payload_ref", value, run_ids)?
            {
                return Ok(Some(run_id));
            }
        }
        Ok(None)
    }

    async fn run_id_by_event_ledger_event_id(
        &self,
        value: &str,
    ) -> ModelLaneResult<Option<String>> {
        let predicate = self.access.sql_predicate(2);
        let sql = format!(
            "SELECT payload, {RESOURCE_SCOPE_SELECT_COLUMNS} FROM (\
                 SELECT event_id, payload, \
                        NULLIF(payload->>'owner_account_id', '')::uuid AS owner_account_id, \
                        NULLIF(payload->>'actor_principal_id', '')::uuid AS actor_principal_id, \
                        NULLIF(payload->>'authenticated_session_id', '')::uuid AS authenticated_session_id, \
                        NULLIF(payload->>'access_space_id', '')::uuid AS access_space_id, \
                        payload->>'workspace_id' AS workspace_id \
                 FROM kernel_event_ledger\
             ) scoped WHERE event_id = $1{} LIMIT 1",
            predicate.clause()
        );
        let row = predicate
            .bind(sqlx::query(&sql).bind(value))
            .fetch_optional(&*self.pool)
            .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        self.access
            .authorize_row(&stored_resource_scope_from_row(&row)?)?;
        let payload: Value = row.try_get("payload")?;
        Ok(event_payload_run_id(&payload))
    }

    async fn run_id_by_event_ledger_seq(&self, value: i64) -> ModelLaneResult<Option<String>> {
        let predicate = self.access.sql_predicate(2);
        let sql = format!(
            "SELECT payload, {RESOURCE_SCOPE_SELECT_COLUMNS} FROM (\
                 SELECT event_id, event_sequence, payload, \
                        NULLIF(payload->>'owner_account_id', '')::uuid AS owner_account_id, \
                        NULLIF(payload->>'actor_principal_id', '')::uuid AS actor_principal_id, \
                        NULLIF(payload->>'authenticated_session_id', '')::uuid AS authenticated_session_id, \
                        NULLIF(payload->>'access_space_id', '')::uuid AS access_space_id, \
                        payload->>'workspace_id' AS workspace_id \
                 FROM kernel_event_ledger\
             ) scoped WHERE event_sequence = $1{} ORDER BY event_id ASC LIMIT 1",
            predicate.clause()
        );
        let row = predicate
            .bind(sqlx::query(&sql).bind(value))
            .fetch_optional(&*self.pool)
            .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        self.access
            .authorize_row(&stored_resource_scope_from_row(&row)?)?;
        let payload: Value = row.try_get("payload")?;
        Ok(event_payload_run_id(&payload))
    }

    async fn navigation_projection_for_run(
        &self,
        route_id: &str,
        lookup_kind: &str,
        lookup_ref: &str,
        run_id: &str,
    ) -> ModelLaneResult<ModelLaneNavigationProjection> {
        let replay = self.replay_run(run_id).await?;
        let artifacts = select_records_by_column::<ModelLaneContextBundleArtifactBindingRecord>(
            &*self.pool,
            &self.access,
            "model_lane_context_bundle_artifacts",
            "run_id",
            run_id,
        )
        .await?;
        let context_handoffs = select_records_by_column::<ModelLaneContextBundleHandoffRecord>(
            &*self.pool,
            &self.access,
            "model_lane_context_bundle_handoffs",
            "run_id",
            run_id,
        )
        .await?;
        let mut authority_tx = self.pool.begin().await?;
        for artifact in &artifacts {
            validate_stored_context_bundle_artifact_authority_tx(
                &mut authority_tx,
                &self.access,
                artifact,
            )
            .await
            .map_err(|_| {
                ModelLaneError::AuthorityDenied("ModelLane navigation authority unavailable".into())
            })?;
        }
        for handoff in &context_handoffs {
            validate_stored_context_bundle_handoff_authority_tx(
                &mut authority_tx,
                &self.access,
                handoff,
            )
            .await
            .map_err(|_| {
                ModelLaneError::AuthorityDenied("ModelLane navigation authority unavailable".into())
            })?;
        }
        authority_tx.commit().await?;
        let recovery_checkpoints = select_records_by_column::<ModelLaneRecoveryCheckpointRecord>(
            &*self.pool,
            &self.access,
            "model_lane_recovery_checkpoints",
            "run_id",
            run_id,
        )
        .await?;
        let recovery_events = select_records_by_column::<ModelLaneRecoveryEventRecord>(
            &*self.pool,
            &self.access,
            "model_lane_recovery_events",
            "run_id",
            run_id,
        )
        .await?;
        let leases = select_records_by_column::<ModelLaneLeaseRecord>(
            &*self.pool,
            &self.access,
            "model_lane_leases",
            "run_id",
            run_id,
        )
        .await?;
        let diagnostic_tiers = select_records_by_column::<ModelLaneDiagnosticTierStatusRecord>(
            &*self.pool,
            &self.access,
            "model_lane_diagnostic_tier_statuses",
            "run_id",
            run_id,
        )
        .await?;
        let mt_runtime_statuses = select_records_by_column::<ModelLaneMtRuntimeStatusRecord>(
            &*self.pool,
            &self.access,
            "model_lane_mt_runtime_statuses",
            "run_id",
            run_id,
        )
        .await?;
        let mut projection = ModelLaneNavigationProjection {
            schema_id: "hsk.model_lane_navigation@1".into(),
            surface_contract_id: "native_swarm_lane_diagnostics".into(),
            route_id: route_id.into(),
            lookup_kind: lookup_kind.into(),
            lookup_ref: lookup_ref.into(),
            input_schema_ref: "hsk.model_lane_navigation_request@1".into(),
            output_schema_ref: "hsk.model_lane_navigation@1".into(),
            manual_refs: vec![
                "usermanual://model-lane-navigation".into(),
                "usermanual://model-lane-diagnostics".into(),
                "usermanual://model-lane-recovery".into(),
                "usermanual://model-lane-validation-harness".into(),
            ],
            run: Some(replay.run),
            lanes: replay.lanes,
            messages: replay.messages,
            artifacts,
            context_handoffs,
            recovery_checkpoints,
            recovery_events,
            leases,
            diagnostic_tiers,
            mt_runtime_statuses,
            event_ledger_refs: Vec::new(),
            flight_recorder_refs: Vec::new(),
            error_codes: Vec::new(),
            recovery_routes: vec![
                "GET /swarm/model-lanes/navigation/recovery/{run_id}".into(),
                "GET /swarm/model-lanes/diagnostics/{run_id}".into(),
                "ModelLaneStore::recover_run_after_restart".into(),
                "ModelLaneStore::replay_run".into(),
            ],
        };
        projection.rebuild_navigation_evidence();
        Ok(projection)
    }

    async fn context_artifacts_by_ref(
        &self,
        value: &str,
    ) -> ModelLaneResult<Vec<ModelLaneContextBundleArtifactBindingRecord>> {
        select_records_by_any_artifact_ref(&*self.pool, &self.access, value).await
    }

    async fn context_handoffs_by_context(
        &self,
        context_bundle_id: &str,
    ) -> ModelLaneResult<Vec<ModelLaneContextBundleHandoffRecord>> {
        select_records_by_column::<ModelLaneContextBundleHandoffRecord>(
            &*self.pool,
            &self.access,
            "model_lane_context_bundle_handoffs",
            "context_bundle_id",
            context_bundle_id,
        )
        .await
    }

    async fn context_handoffs_by_artifact_ref(
        &self,
        value: &str,
    ) -> ModelLaneResult<Vec<ModelLaneContextBundleHandoffRecord>> {
        select_records_by_any_handoff_artifact_ref(&*self.pool, &self.access, value).await
    }

    pub async fn record_recovery_checkpoint(
        &self,
        input: NewModelLaneRecoveryCheckpoint,
    ) -> ModelLaneResult<ModelLaneRecoveryCheckpointRecord> {
        validate_recovery_checkpoint(&input)?;
        let mut tx = self.pool.begin().await?;
        let recovery_child_store = Self::new_with_access(
            self.pool.postgres_pool(),
            recovery_child_access_for_canonical_run_tx(&mut tx, &self.access, &input.run_id)
                .await?,
        );
        lock_idempotency_key_tx(&mut tx, &input.idempotency_key).await?;
        if let Some(existing) = recovery_checkpoint_by_idempotency_key_tx(
            &mut tx,
            &recovery_child_store.access,
            &input.idempotency_key,
        )
        .await?
        {
            ensure_idempotent_input_matches(
                "model_lane_recovery_checkpoint",
                &input.idempotency_key,
                &existing.inner,
                &input,
            )?;
            tx.commit().await?;
            return Ok(existing);
        }
        let run =
            recovery_run_by_id_tx(&mut tx, &recovery_child_store.access, &input.run_id).await?;
        require_equal(
            "model_lane_recovery_checkpoint.event_ledger_stream_id",
            &input.event_ledger_stream_id,
            "run.event_ledger_stream_id",
            &run.event_ledger_stream_id,
        )?;
        if let Some(lane_id) = input.lane_id.as_deref() {
            recovery_lane_by_id_for_run_tx(
                &mut tx,
                &recovery_child_store.access,
                &input.run_id,
                lane_id,
            )
            .await?;
        }
        // A checkpoint watermark is a statement about the exact stream state
        // immediately before the checkpoint event is appended, not merely a
        // reference to any older row in that stream. Checkpoints are rare, so
        // fence ledger inserts for this short validation+append transaction and
        // prove equality with the current stream maximum before writing.
        sqlx::query("LOCK TABLE kernel_event_ledger IN SHARE ROW EXCLUSIVE MODE")
            .execute(&mut *tx)
            .await?;
        ensure_exact_event_ledger_high_watermark_tx(
            &mut tx,
            input.last_event_ledger_seq,
            &input.event_ledger_stream_id,
        )
        .await?;
        let payload = json!({
            "schema_id": "hsk.model_lane_recovery_checkpoint@1",
            "dexterity_kernel": "Dexterity",
            "record": input,
        });
        let event = model_lane_event(
            KernelEventType::ValidationRecorded,
            "model_lane_recovery_checkpoint",
            &input.checkpoint_id,
            &input.idempotency_key,
            &input.work_packet_id,
            &input.event_ledger_stream_id,
            payload,
        )?;
        let stored_event = append_kernel_event_with_executor(&mut *tx, event).await?;
        let sequence = stored_event.event_sequence;
        let record = ModelLaneRecoveryCheckpointRecord {
            inner: input,
            event_ledger_event_id: stored_event.event_id.clone(),
            event_ledger_seq: sequence,
            event_stream_version: sequence,
            transaction_seq: sequence,
        };
        stamp_kernel_event_payload_tx(
            &mut tx,
            &record.event_ledger_event_id,
            recovery_checkpoint_event_payload(&record, recovery_child_store.scope_columns()),
        )
        .await?;
        let insert_sql = format!(
            r#"
            INSERT INTO model_lane_recovery_checkpoints (
                checkpoint_id, run_id, lane_id, session_id, model_session_id,
                lane_status, checkpoint_status, last_event_ledger_seq,
                last_message_id, open_payload_refs, lease_id,
                idempotency_scope, recovery_state, recovery_event_ref,
                event_ledger_stream_id, work_packet_id, micro_task_id,
                task_board_id, owner_session, idempotency_key, created_at_utc,
                recovery_hint_ref, diagnostic_payload, event_ledger_event_id,
                event_ledger_seq, event_stream_version, transaction_seq, record_json,
                {RESOURCE_SCOPE_INSERT_COLUMNS}
            )
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21::timestamptz,$22,$23,$24,$25,$26,$27,$28,$29,$30,$31,$32,$33)
            RETURNING record_json
            "#
        );
        let row = recovery_child_store
            .scope_columns()
            .bind(
                sqlx::query(&insert_sql)
                    .bind(&record.checkpoint_id)
                    .bind(&record.run_id)
                    .bind(record.lane_id.as_deref())
                    .bind(&record.session_id)
                    .bind(&record.model_session_id)
                    .bind(record.lane_status.as_str())
                    .bind(record.checkpoint_status.as_str())
                    .bind(record.last_event_ledger_seq)
                    .bind(record.last_message_id.as_deref())
                    .bind(serde_json::to_value(&record.open_payload_refs)?)
                    .bind(record.lease_id.as_deref())
                    .bind(&record.idempotency_scope)
                    .bind(record.recovery_state.as_str())
                    .bind(record.recovery_event_ref.as_deref())
                    .bind(&record.event_ledger_stream_id)
                    .bind(&record.work_packet_id)
                    .bind(&record.micro_task_id)
                    .bind(&record.task_board_id)
                    .bind(&record.owner_session)
                    .bind(&record.idempotency_key)
                    .bind(&record.created_at_utc)
                    .bind(record.recovery_hint_ref.as_deref())
                    .bind(&record.diagnostic_payload)
                    .bind(&record.event_ledger_event_id)
                    .bind(record.event_ledger_seq)
                    .bind(record.event_stream_version)
                    .bind(record.transaction_seq)
                    .bind(serde_json::to_value(&record)?),
            )
            .fetch_one(&mut *tx)
            .await?;
        tx.commit().await?;
        serde_json::from_value(row_to_json(row, "record_json")?).map_err(Into::into)
    }

    pub async fn record_recovery_event(
        &self,
        input: NewModelLaneRecoveryEvent,
    ) -> ModelLaneResult<ModelLaneRecoveryEventRecord> {
        validate_recovery_event(&input)?;
        let mut tx = self.pool.begin().await?;
        lock_recovery_run_tx(&mut tx, &input.run_id).await?;
        let recovery_child_store = Self::new_with_access(
            self.pool.postgres_pool(),
            recovery_child_access_for_canonical_run_tx(&mut tx, &self.access, &input.run_id)
                .await?,
        );
        let record = recovery_child_store
            .record_recovery_event_tx(&mut tx, input)
            .await?;
        tx.commit().await?;
        Ok(record)
    }

    async fn record_recovery_event_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        mut input: NewModelLaneRecoveryEvent,
    ) -> ModelLaneResult<ModelLaneRecoveryEventRecord> {
        lock_idempotency_key_tx(tx, &input.idempotency_key).await?;
        if let Some(existing) =
            recovery_event_by_idempotency_key_tx(tx, &self.access, &input.idempotency_key).await?
        {
            // replay_order_seq is allocated by this store. Normalize retries to
            // the committed allocation before applying the semantic-idempotency
            // check so a caller cannot reserve or rewrite the run tail.
            input.replay_order_seq = existing.replay_order_seq;
            ensure_idempotent_input_matches(
                "model_lane_recovery_event",
                &input.idempotency_key,
                &existing.inner,
                &input,
            )?;
            return Ok(existing);
        }
        input.replay_order_seq = next_recovery_replay_order_seq_tx(tx, &input.run_id).await?;
        let run = recovery_run_by_id_tx(tx, &self.access, &input.run_id).await?;
        require_equal(
            "model_lane_recovery_event.event_ledger_stream_id",
            &input.event_ledger_stream_id,
            "run.event_ledger_stream_id",
            &run.event_ledger_stream_id,
        )?;
        if let Some(lane_id) = input.lane_id.as_deref() {
            recovery_lane_by_id_for_run_tx(tx, &self.access, &input.run_id, lane_id).await?;
        }
        if let Some(source_event_ledger_seq) = input.source_event_ledger_seq {
            ensure_event_ledger_sequence_in_stream_tx(
                tx,
                source_event_ledger_seq,
                &input.event_ledger_stream_id,
            )
            .await?;
        }
        let payload = json!({
            "schema_id": "hsk.model_lane_recovery_event@2",
            "dexterity_kernel": "Dexterity",
            "record": input,
        });
        let event = model_lane_event(
            KernelEventType::ValidationRecorded,
            "model_lane_recovery_event",
            &input.recovery_event_id,
            &input.idempotency_key,
            &input.work_packet_id,
            &input.event_ledger_stream_id,
            payload,
        )?;
        let stored_event = append_kernel_event_with_executor(&mut **tx, event).await?;
        let sequence = stored_event.event_sequence;
        let record = ModelLaneRecoveryEventRecord {
            inner: input,
            event_ledger_event_id: stored_event.event_id.clone(),
            event_ledger_seq: sequence,
            event_stream_version: sequence,
            transaction_seq: sequence,
        };
        stamp_kernel_event_payload_tx(
            tx,
            &record.event_ledger_event_id,
            recovery_event_event_payload(&record, self.scope_columns()),
        )
        .await?;
        let insert_sql = format!(
            r#"
            INSERT INTO model_lane_recovery_events (
                recovery_event_id, run_id, lane_id, trace_id, span_id,
                parent_span_id, linked_span_contexts, session_id, model_session_id,
                event_kind, recovery_status, replay_order_seq,
                source_event_ledger_seq, payload_refs, artifact_refs,
                crdt_base_snapshot_ref, crdt_state_vector, crdt_stale_base_ref,
                lease_id, failure_kind, error_code, replay_hint,
                event_ledger_stream_id, work_packet_id, micro_task_id,
                task_board_id, owner_session, idempotency_key,
                recovery_hint_ref, diagnostic_payload, event_ledger_event_id,
                event_ledger_seq, event_stream_version, transaction_seq, record_json,
                {RESOURCE_SCOPE_INSERT_COLUMNS}
            )
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24,$25,$26,$27,$28,$29,$30,$31,$32,$33,$34,$35,$36,$37,$38,$39,$40)
            RETURNING record_json
            "#
        );
        let row = self
            .scope_columns()
            .bind(
                sqlx::query(&insert_sql)
                    .bind(&record.recovery_event_id)
                    .bind(&record.run_id)
                    .bind(record.lane_id.as_deref())
                    .bind(&record.trace_id)
                    .bind(&record.span_id)
                    .bind(record.parent_span_id.as_deref())
                    .bind(serde_json::to_value(&record.linked_span_contexts)?)
                    .bind(record.session_id.as_deref())
                    .bind(record.model_session_id.as_deref())
                    .bind(record.event_kind.as_str())
                    .bind(record.recovery_status.as_str())
                    .bind(record.replay_order_seq)
                    .bind(record.source_event_ledger_seq)
                    .bind(serde_json::to_value(&record.payload_refs)?)
                    .bind(serde_json::to_value(&record.artifact_refs)?)
                    .bind(record.crdt_base_snapshot_ref.as_deref())
                    .bind(record.crdt_state_vector.as_deref())
                    .bind(record.crdt_stale_base_ref.as_deref())
                    .bind(record.lease_id.as_deref())
                    .bind(record.failure_kind.map(|kind| kind.as_str()))
                    .bind(record.error_code.as_deref())
                    .bind(&record.replay_hint)
                    .bind(&record.event_ledger_stream_id)
                    .bind(&record.work_packet_id)
                    .bind(&record.micro_task_id)
                    .bind(&record.task_board_id)
                    .bind(&record.owner_session)
                    .bind(&record.idempotency_key)
                    .bind(record.recovery_hint_ref.as_deref())
                    .bind(&record.diagnostic_payload)
                    .bind(&record.event_ledger_event_id)
                    .bind(record.event_ledger_seq)
                    .bind(record.event_stream_version)
                    .bind(record.transaction_seq)
                    .bind(serde_json::to_value(&record)?),
            )
            .fetch_one(&mut **tx)
            .await?;
        serde_json::from_value(row_to_json(row, "record_json")?).map_err(Into::into)
    }

    pub async fn record_lane_lease(
        &self,
        input: NewModelLaneLease,
    ) -> ModelLaneResult<ModelLaneLeaseRecord> {
        validate_lane_lease(&input)?;
        let mut tx = self.pool.begin().await?;
        let recovery_child_store = Self::new_with_access(
            self.pool.postgres_pool(),
            recovery_child_access_for_canonical_run_tx(&mut tx, &self.access, &input.run_id)
                .await?,
        );
        lock_idempotency_key_tx(&mut tx, &input.idempotency_key).await?;
        if let Some(existing) = lane_lease_by_idempotency_key_tx(
            &mut tx,
            &recovery_child_store.access,
            &input.idempotency_key,
        )
        .await?
        {
            ensure_idempotent_input_matches(
                "model_lane_lease",
                &input.idempotency_key,
                &existing.inner,
                &input,
            )?;
            tx.commit().await?;
            return Ok(existing);
        }
        let run =
            recovery_run_by_id_tx(&mut tx, &recovery_child_store.access, &input.run_id).await?;
        require_equal(
            "model_lane_lease.event_ledger_stream_id",
            &input.event_ledger_stream_id,
            "run.event_ledger_stream_id",
            &run.event_ledger_stream_id,
        )?;
        if let Some(lane_id) = input.lane_id.as_deref() {
            recovery_lane_by_id_for_run_tx(
                &mut tx,
                &recovery_child_store.access,
                &input.run_id,
                lane_id,
            )
            .await?;
        }
        let payload = json!({
            "schema_id": "hsk.model_lane_lease@1",
            "dexterity_kernel": "Dexterity",
            "record": input,
        });
        let event = model_lane_event(
            KernelEventType::ValidationRecorded,
            "model_lane_lease",
            &input.lease_id,
            &input.idempotency_key,
            &input.work_packet_id,
            &input.event_ledger_stream_id,
            payload,
        )?;
        let stored_event = append_kernel_event_with_executor(&mut *tx, event).await?;
        let sequence = stored_event.event_sequence;
        let record = ModelLaneLeaseRecord {
            inner: input,
            event_ledger_event_id: stored_event.event_id.clone(),
            event_ledger_seq: sequence,
            event_stream_version: sequence,
            transaction_seq: sequence,
        };
        stamp_kernel_event_payload_tx(
            &mut tx,
            &record.event_ledger_event_id,
            lane_lease_event_payload(&record, recovery_child_store.scope_columns()),
        )
        .await?;
        let insert_sql = format!(
            r#"
            INSERT INTO model_lane_leases (
                lease_id, run_id, lane_id, scope, scope_ref, holder_actor_id,
                holder_session_id, lease_expires_at_utc, takeover_policy_ref,
                state, event_ledger_stream_id, work_packet_id, micro_task_id,
                task_board_id, owner_session, idempotency_key, recovery_hint_ref,
                diagnostic_payload, event_ledger_event_id, event_ledger_seq,
                event_stream_version, transaction_seq, record_json,
                {RESOURCE_SCOPE_INSERT_COLUMNS}
            )
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8::timestamptz,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24,$25,$26,$27,$28)
            RETURNING record_json
            "#
        );
        let row = recovery_child_store
            .scope_columns()
            .bind(
                sqlx::query(&insert_sql)
                    .bind(&record.lease_id)
                    .bind(&record.run_id)
                    .bind(record.lane_id.as_deref())
                    .bind(record.scope.as_str())
                    .bind(&record.scope_ref)
                    .bind(&record.holder_actor_id)
                    .bind(&record.holder_session_id)
                    .bind(&record.lease_expires_at_utc)
                    .bind(&record.takeover_policy_ref)
                    .bind(record.state.as_str())
                    .bind(&record.event_ledger_stream_id)
                    .bind(&record.work_packet_id)
                    .bind(&record.micro_task_id)
                    .bind(&record.task_board_id)
                    .bind(&record.owner_session)
                    .bind(&record.idempotency_key)
                    .bind(record.recovery_hint_ref.as_deref())
                    .bind(&record.diagnostic_payload)
                    .bind(&record.event_ledger_event_id)
                    .bind(record.event_ledger_seq)
                    .bind(record.event_stream_version)
                    .bind(record.transaction_seq)
                    .bind(serde_json::to_value(&record)?),
            )
            .fetch_one(&mut *tx)
            .await?;
        tx.commit().await?;
        serde_json::from_value(row_to_json(row, "record_json")?).map_err(Into::into)
    }

    pub async fn record_diagnostic_tier_status(
        &self,
        input: NewModelLaneDiagnosticTierStatus,
    ) -> ModelLaneResult<ModelLaneDiagnosticTierStatusRecord> {
        validate_diagnostic_tier_status(&input)?;
        let mut tx = self.pool.begin().await?;
        let recovery_child_store = Self::new_with_access(
            self.pool.postgres_pool(),
            recovery_child_access_for_canonical_run_tx(&mut tx, &self.access, &input.run_id)
                .await?,
        );
        lock_idempotency_key_tx(&mut tx, &input.idempotency_key).await?;
        if let Some(existing) = diagnostic_tier_by_idempotency_key_tx(
            &mut tx,
            &recovery_child_store.access,
            &input.idempotency_key,
        )
        .await?
        {
            ensure_idempotent_input_matches(
                "model_lane_diagnostic_tier",
                &input.idempotency_key,
                &existing.inner,
                &input,
            )?;
            tx.commit().await?;
            return Ok(existing);
        }
        let run =
            recovery_run_by_id_tx(&mut tx, &recovery_child_store.access, &input.run_id).await?;
        require_equal(
            "diagnostic_tier.event_ledger_stream_id",
            &input.event_ledger_stream_id,
            "run.event_ledger_stream_id",
            &run.event_ledger_stream_id,
        )?;
        let payload = json!({
            "schema_id": "hsk.model_lane_diagnostic_tier@1",
            "dexterity_kernel": "Dexterity",
            "record": input,
        });
        let event = model_lane_event(
            KernelEventType::ValidationRecorded,
            "model_lane_diagnostic_tier",
            &input.diagnostic_status_id,
            &input.idempotency_key,
            &input.work_packet_id,
            &input.event_ledger_stream_id,
            payload,
        )?;
        let stored_event = append_kernel_event_with_executor(&mut *tx, event).await?;
        let sequence = stored_event.event_sequence;
        let record = ModelLaneDiagnosticTierStatusRecord {
            inner: input,
            event_ledger_event_id: stored_event.event_id.clone(),
            event_ledger_seq: sequence,
            event_stream_version: sequence,
            transaction_seq: sequence,
        };
        stamp_kernel_event_payload_tx(
            &mut tx,
            &record.event_ledger_event_id,
            diagnostic_tier_event_payload(&record, recovery_child_store.scope_columns()),
        )
        .await?;
        let insert_sql = format!(
            r#"
            INSERT INTO model_lane_diagnostic_tier_statuses (
                diagnostic_status_id, behavior_id, run_id, tier, state, reason,
                evidence_ref, follow_up_ref, event_ledger_stream_id,
                work_packet_id, micro_task_id, task_board_id, owner_session,
                idempotency_key, diagnostic_payload, event_ledger_event_id,
                event_ledger_seq, event_stream_version, transaction_seq, record_json,
                {RESOURCE_SCOPE_INSERT_COLUMNS}
            )
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24,$25)
            RETURNING record_json
            "#
        );
        let row = recovery_child_store
            .scope_columns()
            .bind(
                sqlx::query(&insert_sql)
                    .bind(&record.diagnostic_status_id)
                    .bind(&record.behavior_id)
                    .bind(&record.run_id)
                    .bind(record.tier.as_str())
                    .bind(record.state.as_str())
                    .bind(&record.reason)
                    .bind(&record.evidence_ref)
                    .bind(record.follow_up_ref.as_deref())
                    .bind(&record.event_ledger_stream_id)
                    .bind(&record.work_packet_id)
                    .bind(&record.micro_task_id)
                    .bind(&record.task_board_id)
                    .bind(&record.owner_session)
                    .bind(&record.idempotency_key)
                    .bind(&record.diagnostic_payload)
                    .bind(&record.event_ledger_event_id)
                    .bind(record.event_ledger_seq)
                    .bind(record.event_stream_version)
                    .bind(record.transaction_seq)
                    .bind(serde_json::to_value(&record)?),
            )
            .fetch_one(&mut *tx)
            .await?;
        tx.commit().await?;
        serde_json::from_value(row_to_json(row, "record_json")?).map_err(Into::into)
    }

    pub async fn diagnostic_tier_posture(
        &self,
        run_id: &str,
        behavior_id: &str,
    ) -> ModelLaneResult<ModelLaneDiagnosticTierPosture> {
        require_token("run_id", run_id)?;
        require_token("behavior_id", behavior_id)?;
        // HBR-PRIV-004/005: diagnostic tier records carry reasons, evidence
        // refs, and diagnostic payloads for a run, so an unscoped read here is a
        // cross-account side channel even when `replay_run` above it is scoped.
        // Both enforcement layers apply, exactly as on every other model-lane
        // read: the owner predicate keeps denied rows inside PostgreSQL, and the
        // stored scope columns are re-authorized after deserialization so a
        // future edit that drops the predicate still fails closed.
        let predicate = self.access.sql_predicate(3);
        let sql = format!(
            r#"
            SELECT DISTINCT ON (tier) record_json, {RESOURCE_SCOPE_SELECT_COLUMNS}
            FROM model_lane_diagnostic_tier_statuses
            WHERE run_id = $1
              AND behavior_id = $2{}
            ORDER BY tier, event_ledger_seq DESC
            "#,
            predicate.clause()
        );
        let tiers = self.authorize_and_decode_rows::<ModelLaneDiagnosticTierStatusRecord>(
            predicate
                .bind(sqlx::query(&sql).bind(run_id).bind(behavior_id))
                .fetch_all(&*self.pool)
                .await?,
        )?;
        Ok(ModelLaneDiagnosticTierPosture {
            run_id: run_id.to_string(),
            behavior_id: behavior_id.to_string(),
            tiers,
        })
    }

    pub async fn validate_diagnostic_tier_posture(
        &self,
        run_id: &str,
        behavior_id: &str,
    ) -> ModelLaneResult<ModelLaneDiagnosticTierPosture> {
        let posture = self.diagnostic_tier_posture(run_id, behavior_id).await?;
        let have_flight = posture
            .tiers
            .iter()
            .any(|tier| tier.tier == ModelLaneDiagnosticTier::FlightRecorder);
        let have_internal = posture
            .tiers
            .iter()
            .any(|tier| tier.tier == ModelLaneDiagnosticTier::InternalDiagnostics);
        let have_palmistry = posture
            .tiers
            .iter()
            .any(|tier| tier.tier == ModelLaneDiagnosticTier::Palmistry);
        if posture
            .tiers
            .iter()
            .any(|tier| tier.state == ModelLaneDiagnosticTierState::Missing)
        {
            return Err(ModelLaneError::InvalidInput(format!(
                "HBR-INT-009 diagnostic posture for {behavior_id} contains missing tier state"
            )));
        }
        if !have_flight {
            return Err(ModelLaneError::InvalidInput(format!(
                "HBR-INT-009 diagnostic posture for {behavior_id} requires FlightRecorder/EventLedger tier"
            )));
        }
        if have_flight && (!have_internal || !have_palmistry) {
            return Err(ModelLaneError::InvalidInput(format!(
                "HBR-INT-009 diagnostic posture for {behavior_id} is FlightRecorder-only; missing internal_diagnostics or palmistry tier"
            )));
        }
        if !have_internal || !have_palmistry {
            return Err(ModelLaneError::InvalidInput(format!(
                "HBR-INT-009 diagnostic posture for {behavior_id} requires internal_diagnostics and palmistry tier records"
            )));
        }
        for tier in &posture.tiers {
            if tier.state == ModelLaneDiagnosticTierState::DeferredWithReason
                && tier.follow_up_ref.is_none()
            {
                return Err(ModelLaneError::InvalidInput(format!(
                    "HBR-INT-009 deferred tier {} for {behavior_id} requires follow_up_ref",
                    tier.tier.as_str()
                )));
            }
        }
        Ok(posture)
    }

    pub async fn record_mt_runtime_status(
        &self,
        input: NewModelLaneMtRuntimeStatus,
    ) -> ModelLaneResult<ModelLaneMtRuntimeStatusRecord> {
        validate_mt_runtime_status(&input)?;
        let mut tx = self.pool.begin().await?;
        let recovery_child_store = Self::new_with_access(
            self.pool.postgres_pool(),
            recovery_child_access_for_canonical_run_tx(&mut tx, &self.access, &input.run_id)
                .await?,
        );
        lock_idempotency_key_tx(&mut tx, &input.idempotency_key).await?;
        if let Some(existing) = mt_runtime_status_by_idempotency_key_tx(
            &mut tx,
            &recovery_child_store.access,
            &input.idempotency_key,
        )
        .await?
        {
            ensure_idempotent_input_matches(
                "model_lane_mt_runtime_status",
                &input.idempotency_key,
                &existing.inner,
                &input,
            )?;
            tx.commit().await?;
            return Ok(existing);
        }
        let run =
            recovery_run_by_id_tx(&mut tx, &recovery_child_store.access, &input.run_id).await?;
        require_equal(
            "model_lane_mt_runtime_status.event_ledger_stream_id",
            &input.event_ledger_stream_id,
            "run.event_ledger_stream_id",
            &run.event_ledger_stream_id,
        )?;
        let payload = json!({
            "schema_id": "hsk.model_lane_mt_runtime_status@1",
            "dexterity_kernel": "Dexterity",
            "record": input,
        });
        let event = model_lane_event(
            KernelEventType::ValidationRecorded,
            "model_lane_mt_runtime_status",
            &input.mt_status_id,
            &input.idempotency_key,
            &input.work_packet_id,
            &input.event_ledger_stream_id,
            payload,
        )?;
        let stored_event = append_kernel_event_with_executor(&mut *tx, event).await?;
        let sequence = stored_event.event_sequence;
        let record = ModelLaneMtRuntimeStatusRecord {
            inner: input,
            event_ledger_event_id: stored_event.event_id.clone(),
            event_ledger_seq: sequence,
            event_stream_version: sequence,
            transaction_seq: sequence,
        };
        stamp_kernel_event_payload_tx(
            &mut tx,
            &record.event_ledger_event_id,
            mt_runtime_status_event_payload(&record, recovery_child_store.scope_columns()),
        )
        .await?;
        let insert_sql = format!(
            r#"
            INSERT INTO model_lane_mt_runtime_statuses (
                mt_status_id, run_id, work_packet_id, micro_task_id,
                task_board_id, status, claimed_by_ref, blocker_ref,
                missing_resource_ref, proof_status_ref, hbr_status_ref,
                last_recovery_event_ref, last_runtime_status_ref,
                event_ledger_stream_id, owner_session, idempotency_key,
                diagnostic_payload, event_ledger_event_id, event_ledger_seq,
                event_stream_version, transaction_seq, record_json,
                {RESOURCE_SCOPE_INSERT_COLUMNS}
            )
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24,$25,$26,$27)
            RETURNING record_json
            "#
        );
        let row = recovery_child_store
            .scope_columns()
            .bind(
                sqlx::query(&insert_sql)
                    .bind(&record.mt_status_id)
                    .bind(&record.run_id)
                    .bind(&record.work_packet_id)
                    .bind(&record.micro_task_id)
                    .bind(&record.task_board_id)
                    .bind(record.status.as_str())
                    .bind(record.claimed_by_ref.as_deref())
                    .bind(record.blocker_ref.as_deref())
                    .bind(record.missing_resource_ref.as_deref())
                    .bind(record.proof_status_ref.as_deref())
                    .bind(record.hbr_status_ref.as_deref())
                    .bind(record.last_recovery_event_ref.as_deref())
                    .bind(record.last_runtime_status_ref.as_deref())
                    .bind(&record.event_ledger_stream_id)
                    .bind(&record.owner_session)
                    .bind(&record.idempotency_key)
                    .bind(&record.diagnostic_payload)
                    .bind(&record.event_ledger_event_id)
                    .bind(record.event_ledger_seq)
                    .bind(record.event_stream_version)
                    .bind(record.transaction_seq)
                    .bind(serde_json::to_value(&record)?),
            )
            .fetch_one(&mut *tx)
            .await?;
        tx.commit().await?;
        serde_json::from_value(row_to_json(row, "record_json")?).map_err(Into::into)
    }

    /// Recover every latest checkpoint whose EventLedger authority remains
    /// restartable/reclaimable. The production core boot path invokes this
    /// before exposing backend routes or the managed runtime.
    /// INTENTIONALLY CROSS-OWNER, and the only ModelLane read that is.
    ///
    /// Restart recovery has to reclaim runs abandoned by a crashed process
    /// before anybody has authenticated, so there is no account context to scope
    /// it by and scoping it would silently strand other accounts' runs. Rather
    /// than leaving that as an unmarked unscoped query, the store must be
    /// holding an explicit [`SystemScopeAuthority`]; an account-scoped store is
    /// refused here, so "recovery" can never be used as a disclosure route by a
    /// caller that does hold an account context.
    pub async fn recover_restartable_runs_at_boot(
        &self,
    ) -> ModelLaneResult<Vec<ModelLaneRecoveredRun>> {
        let authority = self.require_system_authority("recover_restartable_runs_at_boot")?;
        tracing::info!(
            target: "handshake_core::model_lane",
            system_scope_authority = authority.reason(),
            "model_lane_boot_recovery_cross_owner_scan"
        );
        let run_ids = sqlx::query_scalar::<_, String>(
            r#"
            SELECT run_id
            FROM (
                SELECT DISTINCT ON (payload->'record'->>'run_id')
                       payload->'record'->>'run_id' AS run_id,
                       payload->'record'->>'recovery_state' AS recovery_state,
                       event_sequence
                FROM kernel_event_ledger
                WHERE aggregate_type = 'model_lane_recovery_checkpoint'
                  AND COALESCE(payload->'record'->>'run_id', '') <> ''
                ORDER BY payload->'record'->>'run_id', event_sequence DESC
            ) AS latest_checkpoint
            WHERE recovery_state IN ('restartable', 'reclaimable')
            ORDER BY run_id
            "#,
        )
        .fetch_all(&*self.pool)
        .await?;
        let mut recovered = Vec::with_capacity(run_ids.len());
        for run_id in run_ids {
            recovered.push(self.recover_run_after_restart(&run_id).await?);
        }
        Ok(recovered)
    }

    pub async fn recover_run_after_restart(
        &self,
        run_id: &str,
    ) -> ModelLaneResult<ModelLaneRecoveredRun> {
        require_token("run_id", run_id)?;
        // Serialize the complete read/reconcile/write cycle and every recovery
        // event append for one run. Orphan decisions are written through this
        // same transaction, so the advisory fence and replay-tail allocation are
        // one atomic authority. Rollback/drop releases the fence on every path.
        let mut recovery_fence = self.pool.begin().await?;
        lock_recovery_run_tx(&mut recovery_fence, run_id).await?;
        let result = self
            .recover_run_after_restart_fenced(&mut recovery_fence, run_id)
            .await;
        match result {
            Ok(recovered) => {
                recovery_fence.commit().await?;
                Ok(recovered)
            }
            Err(error) => {
                let _ = recovery_fence.rollback().await;
                Err(error)
            }
        }
    }

    async fn recover_run_after_restart_fenced(
        &self,
        recovery_fence: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        run_id: &str,
    ) -> ModelLaneResult<ModelLaneRecoveredRun> {
        let recovery_child_access =
            recovery_child_access_for_canonical_run_tx(recovery_fence, &self.access, run_id)
                .await?;
        let canonical_run = canonical_run_for_recovery(&*self.pool, &self.access, run_id).await?;
        validate_diagnostics_row_eventledger_authority(&*self.pool, run_id).await?;
        let recovery_child_store =
            Self::new_with_access(self.pool.postgres_pool(), recovery_child_access);
        validate_recovery_eventledger_resource_scope(
            recovery_fence,
            run_id,
            &canonical_run.event_ledger_stream_id,
        )
        .await?;
        let checkpoint =
            latest_recovery_checkpoint(&*self.pool, run_id, &canonical_run.event_ledger_stream_id)
                .await?;
        require_equal(
            "recovery_checkpoint.event_ledger_stream_id",
            &checkpoint.event_ledger_stream_id,
            "canonical_run.event_ledger_stream_id",
            &canonical_run.event_ledger_stream_id,
        )?;
        validate_recovery_checkpoint_record(&*self.pool, &checkpoint).await?;
        // Spec 4.3.9.2.5 + MT-007 acceptance define a PER-KIND recovery boundary, not a
        // single blunt cut at the checkpoint high-watermark. Neither "replay the whole
        // stream high-watermark" nor "bound everything at the checkpoint" is correct.
        //
        // * CATCH UP (forward stream): "Replay MUST load the latest checkpoint, apply
        //   EventLedger records AFTER that sequence in order." When the run's
        //   coordinator-owned ModelLaneMessage stream genuinely advanced past the
        //   checkpoint (a NEW message was committed), post-checkpoint forward state MUST
        //   be replayed -- messages, recovery events, MT runtime status, and the payload
        //   authority for those NEW messages/events catch up to the current stream
        //   high-watermark. Absent real forward-message progress there is nothing to
        //   catch up and the bound stays at the checkpoint.
        // * RECONCILE (current ownership authority): lane leases are never forward
        //   replay input, but recovery must reconcile their latest committed state even
        //   when a lease was acquired after the checkpoint. This current-authority pass
        //   does not move the replay watermark or make adjunct writes forward progress.
        //   Cloud-consent denials remain checkpoint-bounded replay diagnostics.
        // * REJECT (repairs of already-checkpointed refs): a payload ref that was open
        //   AT the checkpoint, and the CRDT base a recovery event replays against, MUST
        //   have been satisfied at/before the checkpoint. A post-checkpoint artifact or
        //   CRDT "repair" of such a checkpointed ref fails closed, so those two checks
        //   stay bounded at the checkpoint.
        let checkpoint_bound_event_ledger_seq = checkpoint.last_event_ledger_seq;
        let forward_bound_event_ledger_seq = if has_post_checkpoint_forward_messages(
            &*self.pool,
            run_id,
            &checkpoint.event_ledger_stream_id,
            checkpoint_bound_event_ledger_seq,
        )
        .await?
        {
            recovery_stream_high_watermark(&*self.pool, &checkpoint.event_ledger_stream_id).await?
        } else {
            checkpoint_bound_event_ledger_seq
        };
        let bounded_recovery_events = recovery_events_for_run(
            &*self.pool,
            run_id,
            &checkpoint.event_ledger_stream_id,
            forward_bound_event_ledger_seq,
        )
        .await?;
        validate_recovery_event_stream(
            &*self.pool,
            run_id,
            forward_bound_event_ledger_seq,
            &bounded_recovery_events,
        )
        .await?;
        validate_recovery_payload_refs(
            &*self.pool,
            run_id,
            &checkpoint,
            checkpoint_bound_event_ledger_seq,
            forward_bound_event_ledger_seq,
            &bounded_recovery_events,
        )
        .await?;
        validate_recovery_crdt_posture(
            recovery_fence,
            run_id,
            &checkpoint,
            checkpoint_bound_event_ledger_seq,
            &bounded_recovery_events,
        )
        .await?;
        let replay = replay_run_at_recovery_bound(
            &*self.pool,
            run_id,
            &checkpoint,
            forward_bound_event_ledger_seq,
        )
        .await?;
        validate_replay_message_payload_authority(
            &*self.pool,
            run_id,
            &checkpoint,
            forward_bound_event_ledger_seq,
            &replay.messages,
        )
        .await?;
        validate_replay_message_crdt_posture(recovery_fence, &replay.messages).await?;
        // Recovery-event appends are reconciliation authority, not forward
        // message progress. Load the current tail under the run fence so an
        // ordinary append that won the fence before boot recovery participates
        // in contiguous ordering without widening the checkpoint replay bound.
        let mut recovery_events = current_recovery_events_for_run_tx(
            recovery_fence,
            run_id,
            &checkpoint.event_ledger_stream_id,
        )
        .await?;
        validate_contiguous_recovery_order(run_id, &recovery_events)?;
        let leases =
            current_lane_leases_for_run(&*self.pool, run_id, &checkpoint.event_ledger_stream_id)
                .await?;
        let now = Utc::now();
        let mut active_leases = Vec::new();
        let mut reclaimable_lease_ids = Vec::new();
        for lease in leases {
            if lease.state != ModelLaneLeaseState::Active {
                continue;
            }
            let expires = parse_utc("lease_expires_at_utc", &lease.lease_expires_at_utc)?;
            if expires > now {
                active_leases.push(lease);
            } else {
                let authoritative_lane = if let Some(lane_id) = lease.lane_id.as_deref() {
                    Some(
                        current_lane_for_recovery_tx(
                            recovery_fence,
                            run_id,
                            &checkpoint.event_ledger_stream_id,
                            lane_id,
                        )
                        .await?,
                    )
                } else {
                    None
                };
                if !recovery_events.iter().any(|event| {
                    event.event_kind == ModelLaneRecoveryEventKind::OrphanDetected
                        && event.lease_id.as_deref() == Some(lease.lease_id.as_str())
                }) {
                    let orphan_event = recovery_child_store
                        .record_orphan_recovery_event_tx(
                            recovery_fence,
                            &checkpoint,
                            &lease,
                            authoritative_lane.as_ref(),
                        )
                        .await?;
                    recovery_events.push(orphan_event);
                }
                reclaimable_lease_ids.push(lease.lease_id.clone());
            }
        }
        validate_contiguous_recovery_order(run_id, &recovery_events)?;
        let cloud_consent_denials = cloud_consent_denials_for_run(
            &*self.pool,
            run_id,
            &checkpoint.event_ledger_stream_id,
            checkpoint_bound_event_ledger_seq,
        )
        .await?;
        let mt_runtime_statuses = mt_runtime_statuses_for_run(
            &*self.pool,
            run_id,
            &checkpoint.event_ledger_stream_id,
            forward_bound_event_ledger_seq,
        )
        .await?;
        Ok(ModelLaneRecoveredRun {
            replay,
            checkpoint,
            recovery_events,
            active_leases,
            reclaimable_lease_ids,
            cloud_consent_denials,
            mt_runtime_statuses,
        })
    }

    async fn record_orphan_recovery_event_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        checkpoint: &ModelLaneRecoveryCheckpointRecord,
        lease: &ModelLaneLeaseRecord,
        authoritative_lane: Option<&ModelLaneRecord>,
    ) -> ModelLaneResult<ModelLaneRecoveryEventRecord> {
        self.record_recovery_event_tx(tx, NewModelLaneRecoveryEvent {
            recovery_event_id: format!(
                "recovery-event-orphan-{}-{}",
                checkpoint.checkpoint_id, lease.lease_id
            ),
            run_id: checkpoint.run_id.clone(),
            lane_id: lease.lane_id.clone(),
            trace_id: authoritative_lane
                .map(|lane| lane.trace_id.clone())
                .unwrap_or_else(|| format!("trace-{}", checkpoint.run_id)),
            span_id: format!("span-orphan-{}", lease.lease_id),
            parent_span_id: authoritative_lane.map(|lane| lane.lane_span_id.clone()),
            linked_span_contexts: vec![format!(
                "eventledger://{}/{}",
                checkpoint.event_ledger_stream_id, lease.event_ledger_seq
            )],
            session_id: Some(
                authoritative_lane
                    .map(|lane| lane.session_id.clone())
                    .unwrap_or_else(|| checkpoint.session_id.clone()),
            ),
            model_session_id: Some(
                authoritative_lane
                    .map(|lane| lane.model_session_id.clone())
                    .unwrap_or_else(|| checkpoint.model_session_id.clone()),
            ),
            event_kind: ModelLaneRecoveryEventKind::OrphanDetected,
            recovery_status: ModelLaneRecoveryStatus::Observed,
            // The transaction-scoped tail allocator replaces this placeholder.
            replay_order_seq: 1,
            source_event_ledger_seq: Some(lease.event_ledger_seq),
            payload_refs: Vec::new(),
            artifact_refs: vec![lease.scope_ref.clone()],
            crdt_base_snapshot_ref: None,
            crdt_state_vector: None,
            crdt_stale_base_ref: None,
            lease_id: Some(lease.lease_id.clone()),
            failure_kind: Some(ModelLaneRecoveryFailureKind::OrphanedSubagent),
            error_code: Some(ModelLaneRecoveryFailureKind::OrphanedSubagent.code().into()),
            replay_hint: "Expired active lease detected during checkpoint recovery; lane is reclaimable before relaunch".into(),
            event_ledger_stream_id: checkpoint.event_ledger_stream_id.clone(),
            work_packet_id: lease.work_packet_id.clone(),
            micro_task_id: lease.micro_task_id.clone(),
            task_board_id: lease.task_board_id.clone(),
            owner_session: lease.owner_session.clone(),
            idempotency_key: format!(
                "model-lane-orphan-recovery:{}:{}:{}",
                checkpoint.run_id, checkpoint.checkpoint_id, lease.lease_id
            ),
            recovery_hint_ref: Some("usermanual://dexterity/recovery#orphan-reclaim".into()),
            diagnostic_payload: json!({
                "flight_recorder": "EventLedger",
                "reason_code": ModelLaneRecoveryFailureKind::OrphanedSubagent.code(),
                "lease_event_ledger_seq": lease.event_ledger_seq,
                "checkpoint_id": checkpoint.checkpoint_id,
                "reclaimable": true
            }),
        })
        .await
    }

    async fn preflight_cloud_launch_records(
        &self,
        run: &NewModelLaneRun,
        lane: &NewModelLane,
    ) -> ModelLaneResult<()> {
        self.preflight_cloud_launch(cloud_launch_check_from_records(run, lane))
            .await
    }

    async fn preflight_cloud_lane_record(&self, lane: &NewModelLane) -> ModelLaneResult<()> {
        self.preflight_cloud_launch(cloud_launch_check_from_lane(lane))
            .await
    }

    async fn preflight_cloud_launch(
        &self,
        check: CloudLaunchAuthorityCheck,
    ) -> ModelLaneResult<()> {
        require_exact_cloud_launch_scope(&self.access)?;
        match self.ensure_cloud_launch_authority_surreal(&check).await {
            Ok(()) => Ok(()),
            Err(reason) => self.deny_cloud_launch(check, &reason.to_string()).await,
        }
    }

    async fn deny_cloud_launch<T>(
        &self,
        check: CloudLaunchAuthorityCheck,
        reason: &str,
    ) -> ModelLaneResult<T> {
        let exact_scope = require_exact_cloud_launch_scope(&self.access)?;
        let scope = cloud_model_lane_scope(&exact_scope);
        let failure_kind_hash = dexterity_sha256_hex(reason.as_bytes());
        let stable_basis = json!({
            "resource_scope": exact_scope,
            "run_id": &check.run_id,
            "lane_id": &check.lane_id,
        });
        let idempotency_key = format!(
            "model-lane-cloud-consent-denial:{}:{}:{}",
            check.run_id,
            check.lane_id,
            dexterity_sha256_hex(canonical_json_bytes(&stable_basis))
        );
        let mut payload = json!({
            "schema_id": "hsk.model_lane_cloud_consent_denial@1",
            "dexterity_kernel": "Dexterity",
            "reason_code": "CX-MM-007",
            "consent_status": "CX-MM-007",
            "failure_kind": reason,
            "failure_kind_hash": failure_kind_hash,
            "detail": "CX-MM-007 cloud lane launch denied before provider call",
            "run_id": &check.run_id,
            "lane_id": &check.lane_id,
            "model_session_id": &check.model_session_id,
            "provider_kind": &check.provider_kind,
            "requested_model_id": &check.requested_model_id,
            "projection_plan_ref": &check.projection_plan_ref,
            "consent_receipt_ref": &check.consent_receipt_ref,
            "provider_call_attempted": false,
            "partial_authority_state_created": false,
            "flight_recorder": "SurrealDB EventLedger",
            "user_manual_behavior_ref": &check.user_manual_behavior_ref,
            "micro_task_id": &check.micro_task_id,
            "owner_session": &check.owner_session,
        });
        exact_scope
            .stamp_json_object(&mut payload)
            .map_err(|error| {
                ModelLaneError::AuthorityDenied(format!(
                    "cloud denial audit requires exact resource-scope attribution: {error}"
                ))
            })?;
        let event_id = Uuid::now_v7().to_string();
        let event_seq = next_cloud_event_sequence();
        self.cloud_authority()
            .await?
            .put_immutable(
                CloudModelLaneRecordKind::ConsentDenial,
                &check.lane_id,
                &check.run_id,
                check.projection_plan_ref.as_deref(),
                check.consent_receipt_ref.as_deref(),
                &idempotency_key,
                serde_json::to_string(&payload)?,
                event_id,
                event_seq,
                serde_json::to_string(&payload)?,
                &scope,
            )
            .await?;
        Err(ModelLaneError::InvalidInput(format!(
            "CX-MM-007 cloud lane launch denied for run_id {} lane_id {}: {reason}",
            check.run_id, check.lane_id
        )))
    }

    async fn record_cloud_projection_plan_surreal(
        &self,
        mut input: NewModelLaneCloudProjectionPlan,
    ) -> ModelLaneResult<ModelLaneCloudProjectionPlanRecord> {
        canonicalize_cloud_consent_targets(&mut input.target_bindings);
        validate_cloud_projection_plan(&input)?;
        ensure_authority_matches_write_scope(
            "ProjectionPlan.export_delegation.source_scope",
            &input.export_delegation.source_scope,
            &self.access,
        )?;
        let exact_scope = require_exact_cloud_launch_scope(&self.access)?;
        let scope = cloud_model_lane_scope(&exact_scope);
        let target_bindings_hash =
            cloud_consent_target_bindings_hash(input.consent_scope, &input.target_bindings)?;
        let projection_plan_hash = cloud_projection_plan_hash(&input)?;
        let event_id = Uuid::now_v7().to_string();
        let event_seq = next_cloud_event_sequence();
        let record = ModelLaneCloudProjectionPlanRecord {
            inner: input,
            target_bindings_hash,
            projection_plan_hash,
            event_ledger_event_id: event_id.clone(),
            event_ledger_seq: event_seq,
            event_stream_version: event_seq,
            transaction_seq: event_seq,
        };
        let payload = cloud_projection_plan_event_payload(&record);
        let stored = self
            .cloud_authority()
            .await?
            .put_immutable(
                CloudModelLaneRecordKind::ProjectionPlan,
                &record.projection_plan_id,
                &record.run_id,
                Some(&record.projection_plan_id),
                None,
                &record.idempotency_key,
                serde_json::to_string(&record)?,
                event_id,
                event_seq,
                serde_json::to_string(&payload)?,
                &scope,
            )
            .await?;
        let stored_record: ModelLaneCloudProjectionPlanRecord =
            serde_json::from_str(&stored.record_json)?;
        validate_cloud_projection_authority_surreal(&stored_record, &stored)?;
        if stored_record.projection_plan_hash != record.projection_plan_hash {
            return Err(ModelLaneError::IdempotencyConflict(format!(
                "idempotency_key {} already belongs to projection_plan_hash {}",
                record.idempotency_key, stored_record.projection_plan_hash
            )));
        }
        Ok(stored_record)
    }

    async fn record_cloud_consent_receipt_surreal(
        &self,
        mut input: NewModelLaneCloudConsentReceipt,
    ) -> ModelLaneResult<ModelLaneCloudConsentReceiptRecord> {
        canonicalize_cloud_consent_targets(&mut input.target_bindings);
        validate_cloud_consent_receipt(&input)?;
        ensure_authority_matches_write_scope(
            "ConsentReceipt.approver",
            &input.approver,
            &self.access,
        )?;
        let exact_scope = require_exact_cloud_launch_scope(&self.access)?;
        let scope = cloud_model_lane_scope(&exact_scope);
        let projection_row = self
            .cloud_authority()
            .await?
            .get(
                CloudModelLaneRecordKind::ProjectionPlan,
                &input.projection_plan_id,
                &scope,
            )
            .await?
            .ok_or_else(|| {
                ModelLaneError::AuthorityDenied(format!(
                    "CX-MM-007 ProjectionPlan {} is not durable",
                    input.projection_plan_id
                ))
            })?;
        let projection: ModelLaneCloudProjectionPlanRecord =
            serde_json::from_str(&projection_row.record_json)?;
        validate_cloud_projection_authority_surreal(&projection, &projection_row)?;
        let target_bindings_hash =
            cloud_consent_target_bindings_hash(input.consent_scope, &input.target_bindings)?;
        let consent_receipt_hash = cloud_consent_receipt_hash(&input)?;
        let event_id = Uuid::now_v7().to_string();
        let event_seq = next_cloud_event_sequence();
        let record = ModelLaneCloudConsentReceiptRecord {
            inner: input,
            target_bindings_hash,
            consent_receipt_hash,
            event_ledger_event_id: event_id.clone(),
            event_ledger_seq: event_seq,
            event_stream_version: event_seq,
            transaction_seq: event_seq,
        };
        let payload = cloud_consent_receipt_event_payload(&record);
        let stored = self
            .cloud_authority()
            .await?
            .put_immutable(
                CloudModelLaneRecordKind::ConsentReceipt,
                &record.consent_receipt_id,
                &record.run_id,
                Some(&record.projection_plan_id),
                Some(&record.consent_receipt_id),
                &record.idempotency_key,
                serde_json::to_string(&record)?,
                event_id,
                event_seq,
                serde_json::to_string(&payload)?,
                &scope,
            )
            .await?;
        let stored_record: ModelLaneCloudConsentReceiptRecord =
            serde_json::from_str(&stored.record_json)?;
        validate_cloud_consent_authority_surreal(&stored_record, &stored)?;
        if stored_record.consent_receipt_hash != record.consent_receipt_hash {
            return Err(ModelLaneError::IdempotencyConflict(format!(
                "idempotency_key {} already belongs to consent_receipt_hash {}",
                record.idempotency_key, stored_record.consent_receipt_hash
            )));
        }
        Ok(stored_record)
    }

    async fn replay_cloud_consent_authority_surreal(
        &self,
        run_id: &str,
    ) -> ModelLaneResult<ModelLaneCloudConsentAuthorityReplay> {
        require_token("run_id", run_id)?;
        let exact_scope = require_exact_cloud_launch_scope(&self.access)?;
        let scope = cloud_model_lane_scope(&exact_scope);
        let store = self.cloud_authority().await?;
        let projection_rows = store
            .list_run(CloudModelLaneRecordKind::ProjectionPlan, run_id, &scope)
            .await?;
        let consent_rows = store
            .list_run(CloudModelLaneRecordKind::ConsentReceipt, run_id, &scope)
            .await?;
        let mut projection_plans = Vec::with_capacity(projection_rows.len());
        for row in projection_rows {
            let record = serde_json::from_str(&row.record_json)?;
            validate_cloud_projection_authority_surreal(&record, &row)?;
            projection_plans.push(record);
        }
        let mut consent_receipts = Vec::with_capacity(consent_rows.len());
        for row in consent_rows {
            let record: ModelLaneCloudConsentReceiptRecord =
                serde_json::from_str(&row.record_json)?;
            validate_cloud_consent_authority_surreal(&record, &row)?;
            let plan = projection_plans
                .iter()
                .find(|plan| plan.projection_plan_id == record.projection_plan_id)
                .ok_or_else(|| {
                    ModelLaneError::AuthorityDenied(format!(
                        "CX-MM-007 consent receipt {} references projection plan {} outside the replay snapshot",
                        record.consent_receipt_id, record.projection_plan_id
                    ))
                })?;
            validate_cloud_authority_pair(plan, &record)?;
            consent_receipts.push(record);
        }
        Ok(ModelLaneCloudConsentAuthorityReplay {
            projection_plans,
            consent_receipts,
        })
    }

    async fn ensure_cloud_launch_authority_surreal(
        &self,
        check: &CloudLaunchAuthorityCheck,
    ) -> ModelLaneResult<()> {
        require_token("cloud.run_id", &check.run_id)?;
        require_token("cloud.lane_id", &check.lane_id)?;
        require_token("cloud.model_session_id", &check.model_session_id)?;
        require_token("cloud.provider_kind", &check.provider_kind)?;
        require_token("cloud.requested_model_id", &check.requested_model_id)?;
        require_token(
            "cloud.capability_snapshot_ref",
            &check.capability_snapshot_ref,
        )?;
        require_token("cloud.provider_endpoint_ref", &check.provider_endpoint_ref)?;
        let projection_plan_id =
            require_optional_token("projection_plan_ref", check.projection_plan_ref.as_deref())?;
        let consent_receipt_id =
            require_optional_token("consent_receipt_ref", check.consent_receipt_ref.as_deref())?;
        let exact_scope = require_exact_cloud_launch_scope(&self.access)?;
        let scope = cloud_model_lane_scope(&exact_scope);
        let store = self.cloud_authority().await?;
        let projection_row = store
            .get(
                CloudModelLaneRecordKind::ProjectionPlan,
                &projection_plan_id,
                &scope,
            )
            .await?
            .ok_or_else(|| {
                ModelLaneError::InvalidInput(format!(
                    "ProjectionPlan {projection_plan_id} is not durable"
                ))
            })?;
        let consent_row = store
            .get(
                CloudModelLaneRecordKind::ConsentReceipt,
                &consent_receipt_id,
                &scope,
            )
            .await?
            .ok_or_else(|| {
                ModelLaneError::InvalidInput(format!(
                    "ConsentReceipt {consent_receipt_id} is not durable"
                ))
            })?;
        let projection: ModelLaneCloudProjectionPlanRecord =
            serde_json::from_str(&projection_row.record_json)?;
        let consent: ModelLaneCloudConsentReceiptRecord =
            serde_json::from_str(&consent_row.record_json)?;
        validate_cloud_projection_authority_surreal(&projection, &projection_row)?;
        validate_cloud_consent_authority_surreal(&consent, &consent_row)?;
        validate_cloud_authority_pair(&projection, &consent)?;
        validate_cloud_launch_pair(&self.access, &projection, &consent, check)
    }

    async fn record_cloud_lane_surreal(
        &self,
        input: NewModelLane,
    ) -> ModelLaneResult<ModelLaneRecord> {
        validate_lane(&input)?;
        let exact_scope = require_exact_cloud_launch_scope(&self.access)?;
        let scope = cloud_model_lane_scope(&exact_scope);
        let event_id = Uuid::now_v7().to_string();
        let event_seq = next_cloud_event_sequence();
        let record = ModelLaneRecord {
            event_ledger_event_id: event_id.clone(),
            event_ledger_seq: event_seq,
            inner: input,
        };
        let mut payload = json!({
            "schema_id": "hsk.model_lane@1",
            "dexterity_kernel": "Dexterity",
            "record": &record,
        });
        exact_scope.stamp_json_object(&mut payload).map_err(|_| {
            ModelLaneError::AuthorityDenied(
                "cloud ModelLane could not stamp exact resource attribution".into(),
            )
        })?;
        let consent_receipt_ref = record.consent_receipt_ref.as_deref().ok_or_else(|| {
            ModelLaneError::InvalidInput("cloud ModelLane requires consent_receipt_ref".into())
        })?;
        let stored = self
            .cloud_authority()
            .await?
            .put_immutable(
                CloudModelLaneRecordKind::CloudLane,
                &record.lane_id,
                &record.run_id,
                record.projection_plan_ref.as_deref(),
                Some(consent_receipt_ref),
                &format!("model-lane:{}:{}", record.run_id, record.lane_id),
                serde_json::to_string(&record)?,
                event_id,
                event_seq,
                serde_json::to_string(&payload)?,
                &scope,
            )
            .await?;
        let stored_record: ModelLaneRecord = serde_json::from_str(&stored.record_json)?;
        if stored_record.lane_id != record.lane_id || stored_record.run_id != record.run_id {
            return Err(ModelLaneError::IdempotencyConflict(format!(
                "lane_id {} already belongs to a different cloud lane",
                record.lane_id
            )));
        }
        Ok(stored_record)
    }

    async fn record_cloud_run_surreal(
        &self,
        input: NewModelLaneRun,
    ) -> ModelLaneResult<ModelLaneRunRecord> {
        validate_run(&input)?;
        let exact_scope = require_exact_cloud_launch_scope(&self.access)?;
        let scope = cloud_model_lane_scope(&exact_scope);
        let store = self.cloud_authority().await?;
        if let Some(existing) = store
            .get(CloudModelLaneRecordKind::CloudRun, &input.run_id, &scope)
            .await?
        {
            let existing: ModelLaneRunRecord = serde_json::from_str(&existing.record_json)?;
            if existing.inner == input {
                return Ok(existing);
            }
            return Err(ModelLaneError::IdempotencyConflict(format!(
                "run_id {} already belongs to idempotency_key {}",
                input.run_id, existing.idempotency_key
            )));
        }
        let event_id = Uuid::now_v7().to_string();
        let event_seq = next_cloud_event_sequence();
        let record = ModelLaneRunRecord {
            event_ledger_event_id: event_id.clone(),
            event_ledger_seq: event_seq,
            inner: input,
        };
        let mut payload = json!({
            "schema_id": "hsk.model_lane_run@1",
            "dexterity_kernel": "Dexterity",
            "record": &record.inner,
        });
        exact_scope.stamp_json_object(&mut payload).map_err(|_| {
            ModelLaneError::AuthorityDenied(
                "cloud ModelLaneRun could not stamp exact resource attribution".into(),
            )
        })?;
        let stored = store
            .put_immutable(
                CloudModelLaneRecordKind::CloudRun,
                &record.run_id,
                &record.run_id,
                record.projection_plan_ref.as_deref(),
                record.consent_receipt_ref.as_deref(),
                &record.idempotency_key,
                serde_json::to_string(&record)?,
                event_id,
                event_seq,
                serde_json::to_string(&payload)?,
                &scope,
            )
            .await?;
        let stored_record: ModelLaneRunRecord = serde_json::from_str(&stored.record_json)?;
        if stored_record != record {
            return Err(ModelLaneError::IdempotencyConflict(format!(
                "run_id {} already belongs to a different cloud launch",
                record.run_id
            )));
        }
        Ok(stored_record)
    }

    async fn fence_cloud_consent_revocation_surreal(
        &self,
        consent_receipt_id: &str,
        revoked_by_ref: &str,
        reason: &str,
    ) -> ModelLaneResult<Vec<ModelLaneRecord>> {
        require_token("consent_receipt_id", consent_receipt_id)?;
        require_token("revoked_by_ref", revoked_by_ref)?;
        require_token("reason", reason)?;
        let exact_scope = require_exact_lifecycle_write_scope(self)?;
        let scope = cloud_model_lane_scope(&exact_scope);
        let store = self.cloud_authority().await?;
        let receipt_row = store
            .get(
                CloudModelLaneRecordKind::ConsentReceipt,
                consent_receipt_id,
                &scope,
            )
            .await?
            .ok_or_else(|| {
                ModelLaneError::AuthorityDenied(
                    "CX-MM-007 consent receipt authority unavailable".into(),
                )
            })?;
        let existing: ModelLaneCloudConsentReceiptRecord =
            serde_json::from_str(&receipt_row.record_json)?;
        validate_cloud_consent_authority_surreal(&existing, &receipt_row)?;
        let projection_row = store
            .get(
                CloudModelLaneRecordKind::ProjectionPlan,
                &existing.projection_plan_id,
                &scope,
            )
            .await?
            .ok_or_else(|| {
                ModelLaneError::AuthorityDenied(format!(
                    "CX-MM-007 ProjectionPlan {} is missing during revocation",
                    existing.projection_plan_id
                ))
            })?;
        let projection: ModelLaneCloudProjectionPlanRecord =
            serde_json::from_str(&projection_row.record_json)?;
        validate_cloud_projection_authority_surreal(&projection, &projection_row)?;
        validate_cloud_authority_pair(&projection, &existing)?;

        let revocation_input_hash =
            cloud_consent_revocation_input_hash(consent_receipt_id, revoked_by_ref, reason);
        if existing.status == ModelLaneCloudConsentReceiptStatus::Revoked {
            if existing.revocation_input_hash.as_deref() != Some(revocation_input_hash.as_str()) {
                return Err(ModelLaneError::IdempotencyConflict(format!(
                    "consent_receipt_id {consent_receipt_id} was already revoked with a different actor or reason"
                )));
            }
            return self
                .cloud_lanes_for_receipt_surreal(&existing, &scope)
                .await;
        }

        let covered_lanes = self
            .cloud_lanes_for_receipt_surreal(&existing, &scope)
            .await?;
        let mut receipt_inner = existing.inner.clone();
        receipt_inner.status = ModelLaneCloudConsentReceiptStatus::Revoked;
        receipt_inner.approved = false;
        receipt_inner.revoked_at_utc = Some(Utc::now().to_rfc3339());
        receipt_inner.revocation_ref = Some(revoked_by_ref.to_owned());
        receipt_inner.revocation_input_hash = Some(revocation_input_hash);
        receipt_inner.diagnostic_payload = merge_diagnostic_payload(
            receipt_inner.diagnostic_payload,
            json!({
                "consent_status": "CX-MM-007",
                "revocation_reason": reason,
                "revoked_by_ref": revoked_by_ref,
                "storage_authority": "embedded_surrealdb",
            }),
        );
        let event_id = Uuid::now_v7().to_string();
        let event_seq = next_cloud_event_sequence();
        let revoked = ModelLaneCloudConsentReceiptRecord {
            consent_receipt_hash: cloud_consent_receipt_hash(&receipt_inner)?,
            target_bindings_hash: existing.target_bindings_hash.clone(),
            event_ledger_event_id: event_id.clone(),
            event_ledger_seq: event_seq,
            event_stream_version: event_seq,
            transaction_seq: event_seq,
            inner: receipt_inner,
        };
        let payload = cloud_consent_receipt_event_payload(&revoked);
        let stored = store
            .replace(
                CloudModelLaneRecordKind::ConsentReceipt,
                consent_receipt_id,
                serde_json::to_string(&revoked)?,
                event_id,
                event_seq,
                serde_json::to_string(&payload)?,
                &scope,
            )
            .await?
            .ok_or_else(|| {
                ModelLaneError::AuthorityDenied(
                    "CX-MM-007 consent receipt disappeared during revocation".into(),
                )
            })?;
        validate_cloud_consent_authority_surreal(&revoked, &stored)?;
        Ok(covered_lanes)
    }

    async fn cloud_lanes_for_receipt_surreal(
        &self,
        receipt: &ModelLaneCloudConsentReceiptRecord,
        scope: &CloudModelLaneScope,
    ) -> ModelLaneResult<Vec<ModelLaneRecord>> {
        let rows = self
            .cloud_authority()
            .await?
            .list_consent_lanes(&receipt.consent_receipt_id, scope)
            .await?;
        let mut lanes = Vec::with_capacity(rows.len());
        for row in rows {
            let lane: ModelLaneRecord = serde_json::from_str(&row.record_json)?;
            let core_matches = lane.run_id == receipt.run_id
                && lane.consent_receipt_ref.as_deref() == Some(receipt.consent_receipt_id.as_str())
                && lane.projection_plan_ref.as_deref() == Some(receipt.projection_plan_id.as_str());
            let single_lane_matches = receipt.consent_scope
                != ModelLaneCloudConsentScope::SingleLane
                || (receipt.lane_id.as_deref() == Some(lane.lane_id.as_str())
                    && receipt.model_session_id.as_deref() == Some(lane.model_session_id.as_str())
                    && receipt.provider_kind.as_deref() == Some(lane.provider_kind.as_str())
                    && receipt.requested_model_id.as_deref() == lane.model_id.as_deref());
            if !core_matches || !single_lane_matches {
                return Err(ModelLaneError::AuthorityDenied(format!(
                    "CX-MM-007 cloud lane {} differs from consent {}",
                    lane.lane_id, receipt.consent_receipt_id
                )));
            }
            lanes.push(lane);
        }
        Ok(lanes)
    }

    async fn finalize_cloud_consent_revocation_surreal(
        &self,
        consent_receipt_id: &str,
        revoked_by_ref: &str,
        reason: &str,
        provider_cancelled_lane_ids: &BTreeSet<String>,
    ) -> ModelLaneResult<Vec<ModelLaneRecord>> {
        let covered_lanes = self
            .fence_cloud_consent_revocation_surreal(consent_receipt_id, revoked_by_ref, reason)
            .await?;
        let exact_scope = require_exact_lifecycle_write_scope(self)?;
        let scope = cloud_model_lane_scope(&exact_scope);
        let store = self.cloud_authority().await?;
        let mut cancelled = Vec::with_capacity(covered_lanes.len());
        for existing in covered_lanes {
            if matches!(
                existing.status,
                ModelLaneStatus::Completed | ModelLaneStatus::Failed | ModelLaneStatus::Cancelled
            ) {
                if existing.status == ModelLaneStatus::Cancelled
                    && existing.failstate_code.as_deref() == Some("CX-MM-007")
                {
                    cancelled.push(existing);
                }
                continue;
            }
            let mut lane = existing.inner.clone();
            lane.status = ModelLaneStatus::Cancelled;
            lane.recovery_state = ModelLaneRecoveryState::Terminal;
            lane.failstate_code = Some("CX-MM-007".into());
            lane.reason_ref = Some(format!(
                "cloud-consent-revoked://dexterity/{}/{}",
                lane.run_id, lane.lane_id
            ));
            lane.recovery_hint_ref =
                Some("usermanual://model-lane-cloud-projection-consent#recovery".into());
            lane.last_runtime_status_ref = Some(format!(
                "runtime-status://dexterity/{}/cloud-consent-revoked",
                lane.lane_id
            ));
            validate_lane(&lane)?;
            let event_id = Uuid::now_v7().to_string();
            let event_seq = next_cloud_event_sequence();
            lane.last_recovery_event_ref = Some(event_id.clone());
            let record = ModelLaneRecord {
                event_ledger_event_id: event_id.clone(),
                event_ledger_seq: event_seq,
                inner: lane,
            };
            let mut payload = json!({
                "schema_id": "hsk.model_lane_terminal@1",
                "dexterity_kernel": "Dexterity",
                "lane_id": &record.lane_id,
                "run_id": &record.run_id,
                "status": "cancelled",
                "reason": reason,
                "reason_code": "CX-MM-007",
                "consent_receipt_id": consent_receipt_id,
                "provider_call_cancelled": provider_cancelled_lane_ids.contains(&record.lane_id),
                "flight_recorder": "SurrealDB EventLedger",
                "record": &record,
            });
            exact_scope.stamp_json_object(&mut payload).map_err(|_| {
                ModelLaneError::AuthorityDenied(
                    "cloud consent terminal EventLedger payload requires exact resource attribution".into(),
                )
            })?;
            let stored = store
                .replace(
                    CloudModelLaneRecordKind::CloudLane,
                    &record.lane_id,
                    serde_json::to_string(&record)?,
                    event_id,
                    event_seq,
                    serde_json::to_string(&payload)?,
                    &scope,
                )
                .await?
                .ok_or_else(|| {
                    ModelLaneError::AuthorityDenied(format!(
                        "CX-MM-007 cloud lane {} disappeared during revocation",
                        record.lane_id
                    ))
                })?;
            if stored.event_id != record.event_ledger_event_id
                || stored.event_seq != record.event_ledger_seq
            {
                return Err(ModelLaneError::IntegrityViolation(format!(
                    "cloud lane {} SurrealDB EventLedger envelope mismatch",
                    record.lane_id
                )));
            }
            cancelled.push(record);
        }
        Ok(cancelled)
    }

    pub async fn schema_registry_rows(&self) -> ModelLaneResult<Vec<ModelLaneSchemaRegistryRow>> {
        sqlx::query(
            r#"
            SELECT schema_id, schema_version, record_kind, table_name
            FROM model_lane_schema_registry
            ORDER BY schema_id
            "#,
        )
        .fetch_all(&*self.pool)
        .await?
        .into_iter()
        .map(|row| {
            Ok(ModelLaneSchemaRegistryRow {
                schema_id: row.try_get("schema_id")?,
                schema_version: row.try_get("schema_version")?,
                record_kind: row.try_get("record_kind")?,
                table_name: row.try_get("table_name")?,
            })
        })
        .collect()
    }
}

/// Keep the model-lane read futures used by Axum route handlers `Send`.
/// `yrs` values are thread-affine, so this compile-time proof catches any
/// future database suspension accidentally introduced while a Yjs value lives.
#[allow(dead_code)]
fn assert_model_lane_route_futures_are_send(store: &ModelLaneStore) {
    fn assert_send<T: Send>(_: T) {}

    assert_send(store.replay_run("send-proof"));
    assert_send(store.diagnostics_projection("send-proof"));
    assert_send(store.navigation_by_run("send-proof"));
    assert_send(store.navigation_by_lane("send-proof"));
    assert_send(store.navigation_by_message("send-proof"));
    assert_send(store.navigation_by_artifact_or_context(None, Some("send-proof"), None));
    assert_send(store.navigation_by_trace("send-proof", None));
    assert_send(store.navigation_by_diagnostics("send-proof", None, None, None));
    assert_send(store.navigation_by_recovery("send-proof"));
    assert_send(store.navigation_by_lookup(ModelLaneNavigationLookup {
        run_id: Some("send-proof".to_string()),
        ..ModelLaneNavigationLookup::default()
    }));
}

fn cloud_launch_check_from_records(
    run: &NewModelLaneRun,
    lane: &NewModelLane,
) -> CloudLaunchAuthorityCheck {
    CloudLaunchAuthorityCheck {
        run_id: run.run_id.clone(),
        lane_id: lane.lane_id.clone(),
        model_session_id: lane.model_session_id.clone(),
        provider_kind: lane.provider_kind.as_str().to_string(),
        requested_model_id: lane.model_id.clone().unwrap_or_default(),
        capability_snapshot_ref: lane
            .effective_capability_snapshot_ref
            .clone()
            .unwrap_or_default(),
        provider_endpoint_ref: lane.adapter_id.clone(),
        projection_plan_ref: lane.projection_plan_ref.clone(),
        consent_receipt_ref: lane.consent_receipt_ref.clone(),
        event_ledger_stream_id: lane.event_ledger_stream_id.clone(),
        work_packet_id: lane
            .work_packet_id
            .clone()
            .or_else(|| run.work_packet_id.clone())
            .unwrap_or_else(|| run.run_id.clone()),
        micro_task_id: lane
            .micro_task_id
            .clone()
            .or_else(|| run.micro_task_id.clone()),
        owner_session: lane.owner_session.clone(),
        user_manual_behavior_ref: "usermanual://model-lane-cloud-projection-consent#launch".into(),
    }
}

fn cloud_launch_check_from_lane(lane: &NewModelLane) -> CloudLaunchAuthorityCheck {
    CloudLaunchAuthorityCheck {
        run_id: lane.run_id.clone(),
        lane_id: lane.lane_id.clone(),
        model_session_id: lane.model_session_id.clone(),
        provider_kind: lane.provider_kind.as_str().to_string(),
        requested_model_id: lane.model_id.clone().unwrap_or_default(),
        capability_snapshot_ref: lane
            .effective_capability_snapshot_ref
            .clone()
            .unwrap_or_default(),
        provider_endpoint_ref: lane.adapter_id.clone(),
        projection_plan_ref: lane.projection_plan_ref.clone(),
        consent_receipt_ref: lane.consent_receipt_ref.clone(),
        event_ledger_stream_id: lane.event_ledger_stream_id.clone(),
        work_packet_id: lane
            .work_packet_id
            .clone()
            .unwrap_or_else(|| lane.run_id.clone()),
        micro_task_id: lane.micro_task_id.clone(),
        owner_session: lane.owner_session.clone(),
        user_manual_behavior_ref: "usermanual://model-lane-cloud-projection-consent#launch".into(),
    }
}

async fn record_or_extend_run_tx(
    tx: &mut Transaction<'_, Postgres>,
    input: NewModelLaneRun,
    lane: &NewModelLane,
    access: &ResourceAccessContext,
) -> ModelLaneResult<ModelLaneRunRecord> {
    // Transaction-local marker so pg_stat_activity names the CALL SITE holding
    // this salt-0 advisory key, not just the SQL text. `true` scopes it to the
    // transaction, so it reverts on commit/rollback and cannot leak across a
    // pooled connection. set_config is used instead of `SET LOCAL` because it
    // accepts a bind parameter; interpolating run_id into SET would be an
    // injection seam.
    sqlx::query("SELECT set_config('application_name', $1, true)")
        .bind(format!("hsk:record_or_extend_run_tx:{}", input.run_id))
        .execute(&mut **tx)
        .await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(&input.run_id)
        .execute(&mut **tx)
        .await?;
    let existing = sqlx::query(&format!(
        "SELECT record_json, {RESOURCE_SCOPE_SELECT_COLUMNS} FROM model_lane_runs WHERE run_id = $1 FOR UPDATE"
    ))
    .bind(&input.run_id)
    .fetch_optional(&mut **tx)
    .await?;
    let Some(existing) = existing else {
        return record_run_tx(tx, input, access.insert_columns()).await;
    };
    // HBR-PRIV-002: attaching a lane to an existing run is a WRITE against that
    // run's row. An account-scoped writer must not be able to extend a run it
    // does not own, or "extend" becomes a cross-account write channel.
    access.authorize_row(&stored_resource_scope_from_row(&existing)?)?;
    let existing: ModelLaneRunRecord =
        serde_json::from_value(row_to_json(existing, "record_json")?)?;
    validate_stored_run_eventledger_authority_tx(tx, &existing, access.exact_read_scope())
        .await
        .map_err(|_| {
            ModelLaneError::AuthorityDenied("ModelLaneRun authority unavailable".into())
        })?;
    let stable_match = existing.trace_id == input.trace_id
        && existing.run_span_id == input.run_span_id
        && existing.coordinator_session_id == input.coordinator_session_id
        && existing.routing_policy == input.routing_policy
        && existing.context_bundle_id == input.context_bundle_id
        && existing.event_ledger_stream_id == input.event_ledger_stream_id
        && existing.artifact_namespace == input.artifact_namespace
        && existing.work_packet_id == input.work_packet_id
        && existing.micro_task_id == input.micro_task_id
        && existing.task_board_id == input.task_board_id
        && existing.owner_session == input.owner_session
        && existing.memory_pack_ref == input.memory_pack_ref
        && existing.memory_pack_hash == input.memory_pack_hash
        && existing.determinism_mode == input.determinism_mode
        && existing.budget_summary_ref == input.budget_summary_ref;
    if !stable_match {
        return Err(ModelLaneError::IdempotencyConflict(format!(
            "run_id {} cannot be extended by a lane with different immutable run identity",
            input.run_id
        )));
    }
    for (name, existing_ref, incoming_ref) in [
        (
            "projection_plan_ref",
            existing.projection_plan_ref.as_ref(),
            input.projection_plan_ref.as_ref(),
        ),
        (
            "consent_receipt_ref",
            existing.consent_receipt_ref.as_ref(),
            input.consent_receipt_ref.as_ref(),
        ),
    ] {
        if existing_ref.is_some() && incoming_ref.is_some() && existing_ref != incoming_ref {
            return Err(ModelLaneError::IdempotencyConflict(format!(
                "run_id {} cannot change {name} while attaching lane {}",
                input.run_id, lane.lane_id
            )));
        }
    }
    if existing
        .lane_ids
        .iter()
        .any(|lane_id| lane_id == &lane.lane_id)
    {
        return Ok(existing);
    }
    let mut merged = existing.inner.clone();
    let mut lane_ids: BTreeSet<String> = merged.lane_ids.into_iter().collect();
    lane_ids.extend(input.lane_ids);
    lane_ids.insert(lane.lane_id.clone());
    merged.lane_ids = lane_ids.into_iter().collect();
    let mut candidate_model_ids: BTreeSet<String> =
        merged.candidate_model_ids.into_iter().collect();
    candidate_model_ids.extend(input.candidate_model_ids);
    if let Some(model_id) = input.selected_model_id.as_ref() {
        candidate_model_ids.insert(model_id.clone());
    }
    if let Some(model_id) = lane.model_id.as_ref() {
        candidate_model_ids.insert(model_id.clone());
    }
    merged.candidate_model_ids = candidate_model_ids.into_iter().collect();
    merged.projection_plan_ref = merged.projection_plan_ref.or(input.projection_plan_ref);
    merged.consent_receipt_ref = merged.consent_receipt_ref.or(input.consent_receipt_ref);
    let idempotency_key = format!("model-lane-run-attach:{}:{}", merged.run_id, lane.lane_id);
    let mut extension_payload = json!({
        "schema_id": "hsk.model_lane_run_extension@1",
        "dexterity_kernel": "Dexterity",
        "run_id": merged.run_id,
        "attached_lane_id": lane.lane_id,
        "record": merged,
    });
    if let Some(exact_scope) = access.exact_read_scope() {
        exact_scope
            .stamp_json_object(&mut extension_payload)
            .map_err(|_| {
                ModelLaneError::AuthorityDenied(
                    "ModelLaneRun extension could not stamp complete resource attribution".into(),
                )
            })?;
    }
    let event = model_lane_event(
        KernelEventType::SessionStarted,
        "model_lane_run",
        &merged.run_id,
        &idempotency_key,
        merged.work_packet_id.as_deref().unwrap_or(&merged.run_id),
        &merged.event_ledger_stream_id,
        extension_payload,
    )?;
    lock_idempotency_key_tx(tx, &idempotency_key).await?;
    let stored_event = append_kernel_event_with_executor(&mut **tx, event).await?;
    let record = ModelLaneRunRecord {
        inner: merged,
        event_ledger_event_id: stored_event.event_id,
        event_ledger_seq: stored_event.event_sequence,
    };
    sqlx::query(
        "UPDATE model_lane_runs SET event_ledger_event_id=$2, event_ledger_seq=$3, record_json=$4 WHERE run_id=$1",
    )
    .bind(&record.run_id)
    .bind(&record.event_ledger_event_id)
    .bind(record.event_ledger_seq)
    .bind(serde_json::to_value(&record)?)
    .execute(&mut **tx)
    .await?;
    Ok(record)
}

async fn record_run_tx(
    tx: &mut Transaction<'_, Postgres>,
    input: NewModelLaneRun,
    scope: ScopeColumnValues<'_>,
) -> ModelLaneResult<ModelLaneRunRecord> {
    let mut payload = json!({
        "schema_id": "hsk.model_lane_run@1",
        "dexterity_kernel": "Dexterity",
        "record": input,
    });
    if let Some(exact_scope) = exact_resource_scope_from_columns(scope, "ModelLaneRun")? {
        exact_scope.stamp_json_object(&mut payload).map_err(|_| {
            ModelLaneError::AuthorityDenied(
                "ModelLaneRun could not stamp complete resource attribution".into(),
            )
        })?;
    }
    let event = model_lane_event(
        KernelEventType::SessionStarted,
        "model_lane_run",
        &input.run_id,
        &input.idempotency_key,
        input.work_packet_id.as_deref().unwrap_or(&input.run_id),
        &input.event_ledger_stream_id,
        payload,
    )?;

    lock_idempotency_key_tx(tx, &input.idempotency_key).await?;
    let stored_event = append_kernel_event_with_executor(&mut **tx, event).await?;
    let record = ModelLaneRunRecord {
        event_ledger_event_id: stored_event.event_id.clone(),
        event_ledger_seq: stored_event.event_sequence,
        inner: input,
    };
    let insert_sql = format!(
        r#"
        INSERT INTO model_lane_runs (
            run_id, trace_id, run_span_id, coordinator_session_id,
            work_packet_id, micro_task_id, task_board_id, owner_session,
            idempotency_key, replay_order_key, event_ledger_stream_id,
            event_ledger_event_id, event_ledger_seq, record_json,
            {RESOURCE_SCOPE_INSERT_COLUMNS}
        )
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19)
        ON CONFLICT (run_id) DO NOTHING
        RETURNING record_json
        "#
    );
    let inserted = scope
        .bind(
            sqlx::query(&insert_sql)
                .bind(&record.run_id)
                .bind(&record.trace_id)
                .bind(&record.run_span_id)
                .bind(&record.coordinator_session_id)
                .bind(record.work_packet_id.as_deref())
                .bind(record.micro_task_id.as_deref())
                .bind(record.task_board_id.as_deref())
                .bind(&record.owner_session)
                .bind(&record.idempotency_key)
                .bind(&record.replay_order_key)
                .bind(&record.event_ledger_stream_id)
                .bind(&record.event_ledger_event_id)
                .bind(record.event_ledger_seq)
                .bind(serde_json::to_value(&record)?),
        )
        .fetch_optional(&mut **tx)
        .await?;

    if let Some(row) = inserted {
        return serde_json::from_value(row_to_json(row, "record_json")?).map_err(Into::into);
    }

    let existing = run_by_id_tx(tx, &record.run_id).await?;
    if existing == record {
        Ok(existing)
    } else {
        Err(ModelLaneError::IdempotencyConflict(format!(
            "run_id {} already belongs to idempotency_key {}",
            record.run_id, existing.idempotency_key
        )))
    }
}

async fn record_lane_tx(
    tx: &mut Transaction<'_, Postgres>,
    input: NewModelLane,
    scope: ScopeColumnValues<'_>,
) -> ModelLaneResult<ModelLaneRecord> {
    lock_idempotency_key_tx(tx, &format!("model-lane-lifecycle:{}", input.lane_id)).await?;
    let event_idempotency_key = if input.restart_generation == 0 {
        format!("model-lane:{}:{}", input.run_id, input.lane_id)
    } else {
        format!(
            "model-lane:{}:{}:restart:{}",
            input.run_id, input.lane_id, input.restart_generation
        )
    };
    let model_stable_anchor = resolve_lane_stable_anchor_tx(tx, &input).await?;
    let mut payload = json!({
        "schema_id": "hsk.model_lane@1",
        "dexterity_kernel": "Dexterity",
        "model_stable_anchor": model_stable_anchor,
        "record": input,
    });
    if let Some(exact_scope) = exact_resource_scope_from_columns(scope, "ModelLane")? {
        exact_scope.stamp_json_object(&mut payload).map_err(|_| {
            ModelLaneError::AuthorityDenied(
                "ModelLane could not stamp complete resource attribution".into(),
            )
        })?;
    }
    let event = model_lane_event(
        KernelEventType::ModelAdapterInvoked,
        "model_lane",
        &input.lane_id,
        &event_idempotency_key,
        input.work_packet_id.as_deref().unwrap_or(&input.run_id),
        &input.event_ledger_stream_id,
        payload,
    )?;

    lock_idempotency_key_tx(tx, &event_idempotency_key).await?;
    let stored_event = append_kernel_event_with_executor(&mut **tx, event).await?;
    let record = ModelLaneRecord {
        event_ledger_event_id: stored_event.event_id.clone(),
        event_ledger_seq: stored_event.event_sequence,
        inner: input,
    };

    let insert_sql = format!(
        r#"
        INSERT INTO model_lanes (
            lane_id, run_id, trace_id, lane_span_id, kind,
            runtime_binding, launch_authority, status, work_packet_id,
            micro_task_id, task_board_id, owner_session, event_ledger_stream_id,
            event_ledger_event_id, event_ledger_seq, record_json, model_stable_anchor,
            {RESOURCE_SCOPE_INSERT_COLUMNS}
        )
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22)
        ON CONFLICT (lane_id) DO NOTHING
        RETURNING record_json
        "#
    );
    let inserted = scope
        .bind(
            sqlx::query(&insert_sql)
                .bind(&record.lane_id)
                .bind(&record.run_id)
                .bind(&record.trace_id)
                .bind(&record.lane_span_id)
                .bind(record.kind.as_str())
                .bind(record.runtime_binding.as_str())
                .bind(record.launch_authority.as_str())
                .bind(record.status.as_str())
                .bind(record.work_packet_id.as_deref())
                .bind(record.micro_task_id.as_deref())
                .bind(record.task_board_id.as_deref())
                .bind(&record.owner_session)
                .bind(&record.event_ledger_stream_id)
                .bind(&record.event_ledger_event_id)
                .bind(record.event_ledger_seq)
                .bind(serde_json::to_value(&record)?)
                .bind(model_stable_anchor.as_deref()),
        )
        .fetch_optional(&mut **tx)
        .await?;

    if let Some(row) = inserted {
        return serde_json::from_value(row_to_json(row, "record_json")?).map_err(Into::into);
    }

    let existing = lane_by_id_tx(tx, &record.lane_id).await?;
    if existing == record {
        return Ok(existing);
    }

    validate_lane_restart(&existing, &record.inner)?;
    let row = sqlx::query(
        r#"
        UPDATE model_lanes
        SET status = $2,
            event_ledger_event_id = $3,
            event_ledger_seq = $4,
            record_json = $5,
            updated_at = NOW()
        WHERE lane_id = $1
        RETURNING record_json
        "#,
    )
    .bind(&record.lane_id)
    .bind(record.status.as_str())
    .bind(&record.event_ledger_event_id)
    .bind(record.event_ledger_seq)
    .bind(serde_json::to_value(&record)?)
    .fetch_one(&mut **tx)
    .await?;
    serde_json::from_value(row_to_json(row, "record_json")?).map_err(Into::into)
}

fn validate_lane_restart(
    existing: &ModelLaneRecord,
    restart: &NewModelLane,
) -> ModelLaneResult<()> {
    let expected_generation = existing.restart_generation.checked_add(1).ok_or_else(|| {
        ModelLaneError::IdempotencyConflict(format!(
            "lane_id {} restart_generation overflowed",
            existing.lane_id
        ))
    })?;
    if restart.restart_generation != expected_generation {
        return Err(ModelLaneError::IdempotencyConflict(format!(
            "lane_id {} restart_generation {} must follow durable generation {}",
            existing.lane_id, restart.restart_generation, existing.restart_generation
        )));
    }
    if !matches!(
        existing.status,
        ModelLaneStatus::Failed | ModelLaneStatus::Cancelled
    ) {
        return Err(ModelLaneError::IdempotencyConflict(format!(
            "lane_id {} generation {} cannot restart from status {}",
            existing.lane_id,
            existing.restart_generation,
            existing.status.as_str()
        )));
    }
    let stable_identity_matches = existing.lane_id == restart.lane_id
        && existing.run_id == restart.run_id
        && existing.trace_id == restart.trace_id
        && existing.lane_span_id == restart.lane_span_id
        && existing.event_ledger_stream_id == restart.event_ledger_stream_id
        && existing.kind == restart.kind
        && existing.role == restart.role
        && existing.backend == restart.backend
        && existing.model_id == restart.model_id
        && existing.adapter_id == restart.adapter_id
        && existing.runtime_binding == restart.runtime_binding
        && existing.launch_authority == restart.launch_authority
        && existing.provider_kind == restart.provider_kind
        && existing.projection_plan_ref == restart.projection_plan_ref
        && existing.consent_receipt_ref == restart.consent_receipt_ref
        && existing.work_packet_id == restart.work_packet_id
        && existing.micro_task_id == restart.micro_task_id
        && existing.task_board_id == restart.task_board_id
        && existing.owner_session == restart.owner_session;
    if !stable_identity_matches {
        return Err(ModelLaneError::IdempotencyConflict(format!(
            "lane_id {} restart changed stable lane authority",
            existing.lane_id
        )));
    }
    Ok(())
}

async fn resolve_lane_stable_anchor_tx(
    tx: &mut Transaction<'_, Postgres>,
    input: &NewModelLane,
) -> ModelLaneResult<Option<String>> {
    if input.kind != ModelLaneKind::LocalModel
        || input.runtime_binding != RuntimeBinding::Local
        || input.provider_kind != ModelLaneProviderKind::LocalRuntime
    {
        return Ok(None);
    }
    let Some(model_id) = input.model_id.as_deref() else {
        return Ok(None);
    };
    let Ok(model_uuid) = Uuid::parse_str(model_id) else {
        return Ok(None);
    };
    Ok(sqlx::query_scalar::<_, String>(
        "SELECT encode(artifact_sha256, 'hex') FROM model_runtime_registry WHERE last_observed_runtime_model_id = $1",
    )
    .bind(model_uuid)
    .fetch_optional(&mut **tx)
    .await?)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneKind {
    LocalModel,
    CloudModel,
    CliModel,
    HumanOperator,
    Subagent,
    Validator,
}

impl ModelLaneKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LocalModel => "local_model",
            Self::CloudModel => "cloud_model",
            Self::CliModel => "cli_model",
            Self::HumanOperator => "human_operator",
            Self::Subagent => "subagent",
            Self::Validator => "validator",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeBinding {
    Local,
    Cloud,
    CliBridge,
    Human,
    Subagent,
    Validator,
}

impl RuntimeBinding {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Cloud => "cloud",
            Self::CliBridge => "cli_bridge",
            Self::Human => "human",
            Self::Subagent => "subagent",
            Self::Validator => "validator",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaunchAuthority {
    ModelRuntime,
    CloudLane,
    CliBridge,
    Operator,
    SubagentManager,
    ValidatorRunner,
}

impl LaunchAuthority {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ModelRuntime => "model_runtime",
            Self::CloudLane => "cloud_lane",
            Self::CliBridge => "cli_bridge",
            Self::Operator => "operator",
            Self::SubagentManager => "subagent_manager",
            Self::ValidatorRunner => "validator_runner",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneProviderKind {
    OpenAi,
    Anthropic,
    LocalRuntime,
    OfficialCli,
    Human,
    Subagent,
    Validator,
    Other,
}

impl ModelLaneProviderKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::OpenAi => "openai",
            Self::Anthropic => "anthropic",
            Self::LocalRuntime => "local_runtime",
            Self::OfficialCli => "official_cli",
            Self::Human => "human",
            Self::Subagent => "subagent",
            Self::Validator => "validator",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DexterityLaunchAdapterKind {
    LocalModelRuntime,
    ByokCloudOpenAi,
    ByokCloudAnthropic,
    OfficialCliBridge,
    CliBridge,
    HumanOperator,
    Subagent,
    Validator,
    DirectEndpoint,
    FrontendAppSrc,
    AppSrcTauri,
    TerminalOnly,
    ExternalCompat,
}

impl DexterityLaunchAdapterKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LocalModelRuntime => "local_model_runtime",
            Self::ByokCloudOpenAi => "byok_cloud_openai",
            Self::ByokCloudAnthropic => "byok_cloud_anthropic",
            Self::OfficialCliBridge => "official_cli_bridge",
            Self::CliBridge => "cli_bridge",
            Self::HumanOperator => "human_operator",
            Self::Subagent => "subagent",
            Self::Validator => "validator",
            Self::DirectEndpoint => "direct_endpoint",
            Self::FrontendAppSrc => "frontend_app_src",
            Self::AppSrcTauri => "app_src_tauri",
            Self::TerminalOnly => "terminal_only",
            Self::ExternalCompat => "external_compat",
        }
    }

    fn is_bypass(&self) -> bool {
        matches!(
            self,
            Self::DirectEndpoint
                | Self::FrontendAppSrc
                | Self::AppSrcTauri
                | Self::TerminalOnly
                | Self::ExternalCompat
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DexterityLaunchAdapterDescriptor {
    pub adapter_kind: DexterityLaunchAdapterKind,
    pub kind: ModelLaneKind,
    pub runtime_binding: RuntimeBinding,
    pub launch_authority: LaunchAuthority,
    pub provider_kind: ModelLaneProviderKind,
    pub default_backend: String,
    pub default_adapter_id: String,
    pub required_capability_tokens: Vec<String>,
    pub supported_tool_capability_tokens: Vec<String>,
    pub provider_feature_profile_ref: String,
    pub requested_execution_policy_ref: String,
    pub effective_execution_policy_ref: String,
    pub requires_projection_plan: bool,
    pub requires_consent_receipt: bool,
    pub requires_process_ownership: bool,
    pub no_os_process_reason_ref: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DexterityLaunchAdapterRegistry {
    descriptors: BTreeMap<DexterityLaunchAdapterKind, DexterityLaunchAdapterDescriptor>,
}

impl DexterityLaunchAdapterRegistry {
    pub fn standard() -> Self {
        let descriptors = [
            descriptor(
                DexterityLaunchAdapterKind::LocalModelRuntime,
                ModelLaneKind::LocalModel,
                RuntimeBinding::Local,
                LaunchAuthority::ModelRuntime,
                ModelLaneProviderKind::LocalRuntime,
                "model_runtime",
                "model_runtime",
                ["capability://dexterity/local-generate"],
                ["tool-capability://read-context"],
                false,
                false,
                true,
                None,
            ),
            descriptor(
                DexterityLaunchAdapterKind::ByokCloudOpenAi,
                ModelLaneKind::CloudModel,
                RuntimeBinding::Cloud,
                LaunchAuthority::CloudLane,
                ModelLaneProviderKind::OpenAi,
                "cloud_lane_openai",
                "openai_byok",
                ["capability://dexterity/cloud-generate"],
                ["tool-capability://read-context"],
                true,
                true,
                true,
                None,
            ),
            descriptor(
                DexterityLaunchAdapterKind::ByokCloudAnthropic,
                ModelLaneKind::CloudModel,
                RuntimeBinding::Cloud,
                LaunchAuthority::CloudLane,
                ModelLaneProviderKind::Anthropic,
                "cloud_lane_anthropic",
                "anthropic_byok",
                ["capability://dexterity/cloud-generate"],
                ["tool-capability://read-context"],
                true,
                true,
                true,
                None,
            ),
            descriptor(
                DexterityLaunchAdapterKind::OfficialCliBridge,
                ModelLaneKind::CliModel,
                RuntimeBinding::CliBridge,
                LaunchAuthority::CliBridge,
                ModelLaneProviderKind::OfficialCli,
                "official_cli_bridge",
                "official_cli_bridge",
                ["capability://dexterity/cli-generate"],
                ["tool-capability://read-context"],
                false,
                false,
                true,
                None,
            ),
            descriptor(
                DexterityLaunchAdapterKind::CliBridge,
                ModelLaneKind::CliModel,
                RuntimeBinding::CliBridge,
                LaunchAuthority::CliBridge,
                ModelLaneProviderKind::OfficialCli,
                "cli_bridge",
                "cli_bridge",
                ["capability://dexterity/cli-bridge"],
                ["tool-capability://read-context"],
                false,
                false,
                true,
                None,
            ),
            descriptor(
                DexterityLaunchAdapterKind::HumanOperator,
                ModelLaneKind::HumanOperator,
                RuntimeBinding::Human,
                LaunchAuthority::Operator,
                ModelLaneProviderKind::Human,
                "operator_lane",
                "operator",
                ["capability://dexterity/operator-participant"],
                ["tool-capability://read-context"],
                false,
                false,
                false,
                Some("no-os-process://operator-lane".to_string()),
            ),
            descriptor(
                DexterityLaunchAdapterKind::Subagent,
                ModelLaneKind::Subagent,
                RuntimeBinding::Subagent,
                LaunchAuthority::SubagentManager,
                ModelLaneProviderKind::Subagent,
                "subagent_manager",
                "subagent_manager",
                ["capability://dexterity/subagent-participant"],
                ["tool-capability://read-context"],
                false,
                false,
                false,
                Some("no-os-process://subagent-manager-owned".to_string()),
            ),
            descriptor(
                DexterityLaunchAdapterKind::Validator,
                ModelLaneKind::Validator,
                RuntimeBinding::Validator,
                LaunchAuthority::ValidatorRunner,
                ModelLaneProviderKind::Validator,
                "validator_runner",
                "validator_runner",
                ["capability://dexterity/validator-participant"],
                ["tool-capability://read-context"],
                false,
                false,
                false,
                Some("no-os-process://validator-runner-owned".to_string()),
            ),
        ]
        .into_iter()
        .map(|entry| (entry.adapter_kind.clone(), entry))
        .collect();
        Self { descriptors }
    }

    pub fn descriptor(
        &self,
        kind: &DexterityLaunchAdapterKind,
    ) -> ModelLaneResult<&DexterityLaunchAdapterDescriptor> {
        if kind.is_bypass() {
            return Err(ModelLaneError::InvalidInput(format!(
                "Dexterity rejects {} launch bypass; launch authority must be Rust SwarmCoordinator, ModelRuntime, CloudLane, CLI bridge, operator, subagent, or validator runner",
                kind.as_str()
            )));
        }
        self.descriptors.get(kind).ok_or_else(|| {
            ModelLaneError::InvalidInput(format!(
                "Dexterity launch adapter {} is not registered",
                kind.as_str()
            ))
        })
    }

    pub fn adapter_kind_for_spawn_request(
        &self,
        request: &SpawnRequest,
    ) -> ModelLaneResult<DexterityLaunchAdapterKind> {
        match request.provider.unwrap_or(ProviderKind::Local) {
            ProviderKind::Local => Ok(DexterityLaunchAdapterKind::LocalModelRuntime),
            ProviderKind::ByokCloud => match request.byok_cloud_provider {
                Some(ByokCloudProvider::OpenAi) => Ok(DexterityLaunchAdapterKind::ByokCloudOpenAi),
                Some(ByokCloudProvider::Anthropic) => {
                    Ok(DexterityLaunchAdapterKind::ByokCloudAnthropic)
                }
                None => Err(ModelLaneError::InvalidInput(
                    "BYOK cloud Dexterity launch requires an explicit byok_cloud_provider".into(),
                )),
            },
            ProviderKind::OfficialCli => Ok(DexterityLaunchAdapterKind::OfficialCliBridge),
            ProviderKind::ExternalCompat => Err(ModelLaneError::InvalidInput(
                "Dexterity rejects external_compat launch bypass; use a registered Rust adapter"
                    .into(),
            )),
        }
    }

    pub fn preflight_spawn_request(
        &self,
        request: &SpawnRequest,
    ) -> ModelLaneResult<&DexterityLaunchAdapterDescriptor> {
        let contract = request.dexterity_launch.as_ref().ok_or_else(|| {
            ModelLaneError::InvalidInput(
                "Dexterity launch preflight requires SpawnRequest::dexterity_launch".into(),
            )
        })?;
        let adapter_kind = self.adapter_kind_for_spawn_request(request)?;
        let descriptor = self.descriptor(&adapter_kind)?;
        if adapter_kind == DexterityLaunchAdapterKind::OfficialCliBridge
            || request.requested_execution_policy_ref.is_some()
        {
            let requested_policy = request
                .requested_execution_policy_ref
                .as_deref()
                .ok_or_else(|| {
                    ModelLaneError::InvalidInput(
                        "Official-CLI Dexterity launch preflight requires requested_execution_policy_ref"
                            .into(),
                    )
                })?;
            let effective_policy = crate::sandbox::resolve_execution_policy_ref(requested_policy)
                .ok_or_else(|| {
                    ModelLaneError::InvalidInput(format!(
                        "Dexterity launch preflight rejected unknown or stale execution-policy reference {requested_policy}"
                    ))
                })?;
            if requested_policy != descriptor.requested_execution_policy_ref
                || effective_policy != descriptor.effective_execution_policy_ref
            {
                return Err(ModelLaneError::InvalidInput(format!(
                    "Dexterity execution-policy mismatch: requested {requested_policy}, resolved {effective_policy}, adapter requires {} -> {}",
                    descriptor.requested_execution_policy_ref,
                    descriptor.effective_execution_policy_ref
                )));
            }
        }
        if contract.capability_token_ids.is_empty() {
            return Err(ModelLaneError::InvalidInput(
                "Dexterity launch preflight requires capability_token_ids".into(),
            ));
        }
        contract.preflight_for_spawn_request(request, descriptor)?;
        require_token(
            "effective_capability_snapshot_ref",
            &contract.effective_capability_snapshot_ref,
        )?;
        if descriptor.requires_projection_plan {
            require_optional_token(
                "projection_plan_ref",
                contract.projection_plan_ref.as_deref(),
            )?;
        }
        if descriptor.requires_consent_receipt {
            require_optional_token(
                "consent_receipt_ref",
                contract.consent_receipt_ref.as_deref(),
            )?;
        }
        Ok(descriptor)
    }

    pub fn normalize(
        &self,
        mut request: DexterityLaunchAdapterRequest,
    ) -> ModelLaneResult<DexterityNormalizedLaunch> {
        let descriptor = self.descriptor(&request.adapter_kind)?.clone();
        for capability in &request.requested_tool_capability_tokens {
            if !descriptor
                .supported_tool_capability_tokens
                .contains(capability)
            {
                return Err(ModelLaneError::InvalidInput(format!(
                    "unsupported tool capability {capability} for Dexterity adapter {}",
                    request.adapter_kind.as_str()
                )));
            }
        }
        if descriptor.requires_projection_plan {
            require_optional_token(
                "projection_plan_ref",
                request.projection_plan_ref.as_deref(),
            )?;
        }
        if descriptor.requires_consent_receipt {
            require_optional_token(
                "consent_receipt_ref",
                request.consent_receipt_ref.as_deref(),
            )?;
        }
        let status = request.status.unwrap_or(ModelLaneStatus::Ready);
        request.heartbeat_at_utc = request
            .heartbeat_at_utc
            .or_else(|| Some(chrono::Utc::now().to_rfc3339()));
        request.cancellation_ref = request
            .cancellation_ref
            .or_else(|| Some(format!("cancel-token://{}", request.lane_id)));
        request.reclaim_policy_ref = request.reclaim_policy_ref.or_else(|| {
            Some(format!(
                "reclaim-policy://dexterity/{}",
                request.adapter_kind.as_str()
            ))
        });
        request.terminal_status_mapping_ref = request.terminal_status_mapping_ref.or_else(|| {
            Some(format!(
                "terminal-status://session-broker/{}",
                descriptor.runtime_binding.as_str()
            ))
        });
        request.capability_negotiation_ref = request.capability_negotiation_ref.or_else(|| {
            Some(format!(
                "capability-negotiation://dexterity/{}",
                request.lane_id
            ))
        });
        request.effective_capability_snapshot_ref =
            request.effective_capability_snapshot_ref.or_else(|| {
                Some(format!(
                    "capability-snapshot://dexterity/{}",
                    request.lane_id
                ))
            });
        if descriptor.requires_process_ownership {
            require_optional_token(
                "process_ownership_ref",
                request.process_ownership_ref.as_deref(),
            )?;
        } else {
            request.no_os_process_reason_ref =
                Some(descriptor.no_os_process_reason_ref.clone().ok_or_else(|| {
                    ModelLaneError::InvalidInput(format!(
                        "adapter {} requires no_os_process_reason_ref",
                        request.adapter_kind.as_str()
                    ))
                })?);
            request.process_ownership_ref = None;
        }
        let mut capability_token_ids = descriptor.required_capability_tokens.clone();
        capability_token_ids.extend(request.extra_capability_token_ids.iter().cloned());
        capability_token_ids.sort();
        capability_token_ids.dedup();
        if capability_token_ids.is_empty() {
            return Err(ModelLaneError::InvalidInput(
                "Dexterity launch requires at least one negotiated capability".into(),
            ));
        }
        let selected_model_id = request
            .selected_model_id
            .clone()
            .or_else(|| request.model_id.clone());
        let mut candidate_model_ids = request.candidate_model_ids.clone();
        if candidate_model_ids.is_empty() {
            if let Some(model_id) = selected_model_id.clone() {
                candidate_model_ids.push(model_id);
            } else {
                candidate_model_ids.push(format!(
                    "lane://{}:{}",
                    request.adapter_kind.as_str(),
                    request.lane_id
                ));
            }
        }
        Ok(DexterityNormalizedLaunch {
            adapter_kind: request.adapter_kind,
            run_id: request.run_id,
            lane_id: request.lane_id,
            trace_id: request.trace_id,
            run_span_id: request.run_span_id,
            lane_span_id: request.lane_span_id,
            coordinator_session_id: request.coordinator_session_id,
            routing_policy: request.routing_policy,
            context_bundle_id: request.context_bundle_id,
            event_ledger_stream_id: request.event_ledger_stream_id,
            artifact_namespace: request.artifact_namespace,
            work_packet_id: request.work_packet_id,
            micro_task_id: request.micro_task_id,
            task_board_id: request.task_board_id,
            owner_session: request.owner_session,
            locus_binding_ref: request.locus_binding_ref,
            role: request.role,
            backend: request.backend.unwrap_or(descriptor.default_backend),
            adapter_id: request.adapter_id.unwrap_or(descriptor.default_adapter_id),
            model_id: request.model_id,
            session_id: request.session_id,
            model_session_id: request.model_session_id,
            capability_token_ids,
            effective_capability_snapshot_ref: request.effective_capability_snapshot_ref,
            capability_negotiation_ref: request.capability_negotiation_ref,
            provider_feature_profile_ref: request
                .provider_feature_profile_ref
                .unwrap_or(descriptor.provider_feature_profile_ref),
            requested_execution_policy_ref: request
                .requested_execution_policy_ref
                .unwrap_or(descriptor.requested_execution_policy_ref),
            effective_execution_policy_ref: request
                .effective_execution_policy_ref
                .unwrap_or(descriptor.effective_execution_policy_ref),
            projection_plan_ref: request.projection_plan_ref,
            consent_receipt_ref: request.consent_receipt_ref,
            tool_gate_decision_refs: request.tool_gate_decision_refs,
            status,
            heartbeat_at_utc: request.heartbeat_at_utc,
            lease_expires_at_utc: request.lease_expires_at_utc,
            reclaim_after_utc: request.reclaim_after_utc,
            restart_generation: request.restart_generation,
            cancellation_ref: request.cancellation_ref,
            reclaim_policy_ref: request.reclaim_policy_ref,
            terminal_status_mapping_ref: request.terminal_status_mapping_ref,
            process_ownership_ref: request.process_ownership_ref,
            no_os_process_reason_ref: request.no_os_process_reason_ref,
            backpressure_ref: request.backpressure_ref,
            loop_counter_ref: request.loop_counter_ref,
            last_runtime_status_ref: request.last_runtime_status_ref,
            last_recovery_event_ref: request.last_recovery_event_ref,
            startup_failure_code: request.startup_failure_code,
            startup_failure_ref: request.startup_failure_ref,
            reason_ref: request.reason_ref,
            run_recovery_hint_ref: request.run_recovery_hint_ref,
            lane_recovery_hint_ref: request.lane_recovery_hint_ref,
            memory_pack_ref: request.memory_pack_ref,
            memory_pack_hash: request.memory_pack_hash,
            determinism_mode: request.determinism_mode,
            budget_summary_ref: request.budget_summary_ref,
            selected_model_id,
            candidate_model_ids,
            procedural_review_status: request.procedural_review_status,
            truncation_warning_ref: request.truncation_warning_ref,
            rejection_reason_refs: request.rejection_reason_refs,
        })
    }
}

fn descriptor(
    adapter_kind: DexterityLaunchAdapterKind,
    kind: ModelLaneKind,
    runtime_binding: RuntimeBinding,
    launch_authority: LaunchAuthority,
    provider_kind: ModelLaneProviderKind,
    default_backend: &str,
    default_adapter_id: &str,
    required_capability_tokens: impl IntoIterator<Item = &'static str>,
    supported_tool_capability_tokens: impl IntoIterator<Item = &'static str>,
    requires_projection_plan: bool,
    requires_consent_receipt: bool,
    requires_process_ownership: bool,
    no_os_process_reason_ref: Option<String>,
) -> DexterityLaunchAdapterDescriptor {
    DexterityLaunchAdapterDescriptor {
        provider_feature_profile_ref: format!(
            "provider-feature-profile://{}",
            provider_kind.as_str()
        ),
        requested_execution_policy_ref: format!(
            "execution-policy://requested/{}",
            runtime_binding.as_str()
        ),
        effective_execution_policy_ref: format!(
            "execution-policy://effective/{}",
            launch_authority.as_str()
        ),
        adapter_kind,
        kind,
        runtime_binding,
        launch_authority,
        provider_kind,
        default_backend: default_backend.to_string(),
        default_adapter_id: default_adapter_id.to_string(),
        required_capability_tokens: required_capability_tokens
            .into_iter()
            .map(str::to_string)
            .collect(),
        supported_tool_capability_tokens: supported_tool_capability_tokens
            .into_iter()
            .map(str::to_string)
            .collect(),
        requires_projection_plan,
        requires_consent_receipt,
        requires_process_ownership,
        no_os_process_reason_ref,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DexterityLaunchAdapterRequest {
    pub adapter_kind: DexterityLaunchAdapterKind,
    pub run_id: String,
    pub lane_id: String,
    pub trace_id: String,
    pub run_span_id: String,
    pub lane_span_id: String,
    pub coordinator_session_id: String,
    pub routing_policy: String,
    pub context_bundle_id: String,
    pub event_ledger_stream_id: String,
    pub artifact_namespace: String,
    pub work_packet_id: Option<String>,
    pub micro_task_id: Option<String>,
    pub task_board_id: Option<String>,
    pub owner_session: String,
    pub locus_binding_ref: String,
    pub role: String,
    pub backend: Option<String>,
    pub adapter_id: Option<String>,
    pub model_id: Option<String>,
    pub session_id: String,
    pub model_session_id: String,
    pub extra_capability_token_ids: Vec<String>,
    pub requested_tool_capability_tokens: Vec<String>,
    pub effective_capability_snapshot_ref: Option<String>,
    pub capability_negotiation_ref: Option<String>,
    pub provider_feature_profile_ref: Option<String>,
    pub requested_execution_policy_ref: Option<String>,
    pub effective_execution_policy_ref: Option<String>,
    pub projection_plan_ref: Option<String>,
    pub consent_receipt_ref: Option<String>,
    pub tool_gate_decision_refs: Vec<String>,
    pub status: Option<ModelLaneStatus>,
    pub heartbeat_at_utc: Option<String>,
    pub lease_expires_at_utc: Option<String>,
    pub reclaim_after_utc: Option<String>,
    pub restart_generation: i64,
    pub cancellation_ref: Option<String>,
    pub reclaim_policy_ref: Option<String>,
    pub terminal_status_mapping_ref: Option<String>,
    pub process_ownership_ref: Option<String>,
    pub no_os_process_reason_ref: Option<String>,
    pub backpressure_ref: Option<String>,
    pub loop_counter_ref: Option<String>,
    pub last_runtime_status_ref: Option<String>,
    pub last_recovery_event_ref: Option<String>,
    pub startup_failure_code: Option<String>,
    pub startup_failure_ref: Option<String>,
    pub reason_ref: Option<String>,
    pub run_recovery_hint_ref: Option<String>,
    pub lane_recovery_hint_ref: Option<String>,
    pub memory_pack_ref: String,
    pub memory_pack_hash: String,
    pub determinism_mode: String,
    pub budget_summary_ref: String,
    pub selected_model_id: Option<String>,
    pub candidate_model_ids: Vec<String>,
    pub procedural_review_status: String,
    pub truncation_warning_ref: Option<String>,
    pub rejection_reason_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DexterityNormalizedLaunch {
    pub adapter_kind: DexterityLaunchAdapterKind,
    pub run_id: String,
    pub lane_id: String,
    pub trace_id: String,
    pub run_span_id: String,
    pub lane_span_id: String,
    pub coordinator_session_id: String,
    pub routing_policy: String,
    pub context_bundle_id: String,
    pub event_ledger_stream_id: String,
    pub artifact_namespace: String,
    pub work_packet_id: Option<String>,
    pub micro_task_id: Option<String>,
    pub task_board_id: Option<String>,
    pub owner_session: String,
    pub locus_binding_ref: String,
    pub role: String,
    pub backend: String,
    pub adapter_id: String,
    pub model_id: Option<String>,
    pub session_id: String,
    pub model_session_id: String,
    pub capability_token_ids: Vec<String>,
    pub effective_capability_snapshot_ref: Option<String>,
    pub capability_negotiation_ref: Option<String>,
    pub provider_feature_profile_ref: String,
    pub requested_execution_policy_ref: String,
    pub effective_execution_policy_ref: String,
    pub projection_plan_ref: Option<String>,
    pub consent_receipt_ref: Option<String>,
    pub tool_gate_decision_refs: Vec<String>,
    pub status: ModelLaneStatus,
    pub heartbeat_at_utc: Option<String>,
    pub lease_expires_at_utc: Option<String>,
    pub reclaim_after_utc: Option<String>,
    pub restart_generation: i64,
    pub cancellation_ref: Option<String>,
    pub reclaim_policy_ref: Option<String>,
    pub terminal_status_mapping_ref: Option<String>,
    pub process_ownership_ref: Option<String>,
    pub no_os_process_reason_ref: Option<String>,
    pub backpressure_ref: Option<String>,
    pub loop_counter_ref: Option<String>,
    pub last_runtime_status_ref: Option<String>,
    pub last_recovery_event_ref: Option<String>,
    pub startup_failure_code: Option<String>,
    pub startup_failure_ref: Option<String>,
    pub reason_ref: Option<String>,
    pub run_recovery_hint_ref: Option<String>,
    pub lane_recovery_hint_ref: Option<String>,
    pub memory_pack_ref: String,
    pub memory_pack_hash: String,
    pub determinism_mode: String,
    pub budget_summary_ref: String,
    pub selected_model_id: Option<String>,
    pub candidate_model_ids: Vec<String>,
    pub procedural_review_status: String,
    pub truncation_warning_ref: Option<String>,
    pub rejection_reason_refs: Vec<String>,
}

impl DexterityNormalizedLaunch {
    pub fn to_records(self) -> ModelLaneResult<(NewModelLaneRun, NewModelLane)> {
        let descriptor = DexterityLaunchAdapterRegistry::standard()
            .descriptor(&self.adapter_kind)?
            .clone();
        let locus = self.locus()?;
        let run = NewModelLaneRun {
            run_id: self.run_id.clone(),
            trace_id: self.trace_id.clone(),
            run_span_id: self.run_span_id.clone(),
            coordinator_session_id: self.coordinator_session_id.clone(),
            routing_policy: self.routing_policy.clone(),
            context_bundle_id: self.context_bundle_id.clone(),
            lane_ids: vec![self.lane_id.clone()],
            event_ledger_stream_id: self.event_ledger_stream_id.clone(),
            artifact_namespace: self.artifact_namespace.clone(),
            projection_plan_ref: self.projection_plan_ref.clone(),
            consent_receipt_ref: self.consent_receipt_ref.clone(),
            work_packet_id: self.work_packet_id.clone(),
            micro_task_id: self.micro_task_id.clone(),
            task_board_id: self.task_board_id.clone(),
            owner_session: self.owner_session.clone(),
            idempotency_key: format!("dexterity-normalized-launch-run:{}", self.run_id),
            replay_order_key: format!("{}:00000000:run", self.run_id),
            replay_after_event_ledger_seq: None,
            recovery_state: recovery_for_status(&self.status),
            failstate_code: self.startup_failure_code.clone(),
            reason_ref: self.reason_ref.clone(),
            recovery_hint_ref: self.run_recovery_hint_ref.clone(),
            locus_binding: Some(locus.clone()),
            memory_pack_ref: self.memory_pack_ref.clone(),
            memory_pack_hash: self.memory_pack_hash.clone(),
            determinism_mode: self.determinism_mode.clone(),
            budget_summary_ref: self.budget_summary_ref.clone(),
            selected_model_id: self.selected_model_id.clone(),
            candidate_model_ids: self.candidate_model_ids.clone(),
            procedural_review_status: self.procedural_review_status.clone(),
            truncation_warning_ref: self.truncation_warning_ref.clone(),
            rejection_reason_refs: self.rejection_reason_refs.clone(),
        };
        let lane = NewModelLane {
            lane_id: self.lane_id.clone(),
            run_id: self.run_id,
            trace_id: self.trace_id,
            lane_span_id: self.lane_span_id,
            event_ledger_stream_id: self.event_ledger_stream_id,
            kind: descriptor.kind,
            role: self.role,
            backend: self.backend,
            model_id: self.model_id,
            session_id: self.session_id,
            model_session_id: self.model_session_id,
            adapter_id: self.adapter_id,
            runtime_binding: descriptor.runtime_binding,
            launch_authority: descriptor.launch_authority,
            provider_kind: descriptor.provider_kind,
            capability_token_ids: self.capability_token_ids,
            effective_capability_snapshot_ref: self.effective_capability_snapshot_ref,
            capability_negotiation_ref: self.capability_negotiation_ref,
            provider_feature_profile_ref: Some(self.provider_feature_profile_ref),
            requested_execution_policy_ref: Some(self.requested_execution_policy_ref),
            effective_execution_policy_ref: Some(self.effective_execution_policy_ref),
            projection_plan_ref: self.projection_plan_ref,
            consent_receipt_ref: self.consent_receipt_ref,
            tool_gate_decision_refs: self.tool_gate_decision_refs,
            status: self.status.clone(),
            recovery_state: recovery_for_status(&self.status),
            heartbeat_at_utc: self.heartbeat_at_utc,
            lease_expires_at_utc: self.lease_expires_at_utc,
            reclaim_after_utc: self.reclaim_after_utc,
            restart_generation: self.restart_generation,
            cancellation_ref: self.cancellation_ref,
            reclaim_policy_ref: self.reclaim_policy_ref,
            terminal_status_mapping_ref: self.terminal_status_mapping_ref,
            process_ownership_ref: self.process_ownership_ref,
            no_os_process_reason_ref: self.no_os_process_reason_ref,
            backpressure_ref: self.backpressure_ref,
            loop_counter_ref: self.loop_counter_ref,
            last_runtime_status_ref: self.last_runtime_status_ref,
            last_recovery_event_ref: self.last_recovery_event_ref,
            failstate_code: self.startup_failure_code,
            startup_failure_ref: self.startup_failure_ref,
            reason_ref: self.reason_ref,
            recovery_hint_ref: self.lane_recovery_hint_ref,
            work_packet_id: self.work_packet_id,
            micro_task_id: self.micro_task_id,
            task_board_id: self.task_board_id,
            owner_session: self.owner_session,
            locus_binding: Some(locus),
        };
        Ok((run, lane))
    }

    fn locus(&self) -> ModelLaneResult<ModelLaneLocusBinding> {
        Ok(ModelLaneLocusBinding {
            work_packet_id: require_optional_token(
                "work_packet_id",
                self.work_packet_id.as_deref(),
            )?,
            micro_task_id: require_optional_token("micro_task_id", self.micro_task_id.as_deref())?,
            task_board_id: self.task_board_id.clone(),
            coordinator_session_id: self.coordinator_session_id.clone(),
            session_id: self.session_id.clone(),
            model_session_id: self.model_session_id.clone(),
            owner_session: self.owner_session.clone(),
            locus_binding_ref: self.locus_binding_ref.clone(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneStatus {
    Planned,
    Ready,
    Running,
    Waiting,
    Completed,
    Failed,
    Cancelled,
    Reclaimable,
}

impl ModelLaneStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Ready => "ready",
            Self::Running => "running",
            Self::Waiting => "waiting",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Reclaimable => "reclaimable",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneRecoveryState {
    Restartable,
    Reclaimable,
    Terminal,
    Blocked,
}

impl ModelLaneRecoveryState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Restartable => "restartable",
            Self::Reclaimable => "reclaimable",
            Self::Terminal => "terminal",
            Self::Blocked => "blocked",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneRecoveryStatus {
    Observed,
    Checkpointed,
    Recovered,
    Failed,
}

impl ModelLaneRecoveryStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Observed => "observed",
            Self::Checkpointed => "checkpointed",
            Self::Recovered => "recovered",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneRecoveryEventKind {
    RunCreated,
    RunCompleted,
    RunFailed,
    LanePlanned,
    LaneStarted,
    LaneStatusChanged,
    LaneCompleted,
    LaneFailed,
    LaneCancelled,
    OrphanDetected,
    MessageRecorded,
    PayloadRefRecorded,
    PayloadRefMissing,
    RecoveryRequested,
    ReplayReconstructed,
    RecoveryFailed,
    CheckpointRestored,
    CrdtUpdateObserved,
    PayloadRefObserved,
    LeaseObserved,
    CloudConsentDenied,
    MtStatusRestored,
}

impl ModelLaneRecoveryEventKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RunCreated => "run_created",
            Self::RunCompleted => "run_completed",
            Self::RunFailed => "run_failed",
            Self::LanePlanned => "lane_planned",
            Self::LaneStarted => "lane_started",
            Self::LaneStatusChanged => "lane_status_changed",
            Self::LaneCompleted => "lane_completed",
            Self::LaneFailed => "lane_failed",
            Self::LaneCancelled => "lane_cancelled",
            Self::OrphanDetected => "orphan_detected",
            Self::MessageRecorded => "message_recorded",
            Self::PayloadRefRecorded => "payload_ref_recorded",
            Self::PayloadRefMissing => "payload_ref_missing",
            Self::RecoveryRequested => "recovery_requested",
            Self::ReplayReconstructed => "replay_reconstructed",
            Self::RecoveryFailed => "recovery_failed",
            Self::CheckpointRestored => "checkpoint_restored",
            Self::CrdtUpdateObserved => "crdt_update_observed",
            Self::PayloadRefObserved => "payload_ref_observed",
            Self::LeaseObserved => "lease_observed",
            Self::CloudConsentDenied => "cloud_consent_denied",
            Self::MtStatusRestored => "mt_status_restored",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneRecoveryFailureKind {
    EventLedgerSequenceGap,
    MissingPayloadAuthority,
    StaleCrdtBase,
    CorruptCheckpoint,
    MissingCheckpoint,
    MissingEventLedgerRow,
    OrphanedSubagent,
    CancelledProcess,
    CrashedProcess,
    NeverStartedLane,
}

impl ModelLaneRecoveryFailureKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::EventLedgerSequenceGap => "event_ledger_sequence_gap",
            Self::MissingPayloadAuthority => "missing_payload_authority",
            Self::StaleCrdtBase => "stale_crdt_base",
            Self::CorruptCheckpoint => "corrupt_checkpoint",
            Self::MissingCheckpoint => "missing_checkpoint",
            Self::MissingEventLedgerRow => "missing_event_ledger_row",
            Self::OrphanedSubagent => "orphaned_subagent",
            Self::CancelledProcess => "cancelled_process",
            Self::CrashedProcess => "crashed_process",
            Self::NeverStartedLane => "never_started_lane",
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::EventLedgerSequenceGap => "CX-MM-003",
            Self::MissingPayloadAuthority => "CX-MM-006",
            Self::StaleCrdtBase => "CX-MM-008",
            Self::CorruptCheckpoint => "CX-MM-009",
            Self::MissingCheckpoint => "CX-MM-010",
            Self::MissingEventLedgerRow => "CX-MM-011",
            Self::OrphanedSubagent => "CX-MM-009",
            Self::CancelledProcess => "CX-MM-012",
            Self::CrashedProcess => "CX-MM-013",
            Self::NeverStartedLane => "CX-MM-014",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneLeaseScope {
    Run,
    Lane,
}

impl ModelLaneLeaseScope {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Run => "run",
            Self::Lane => "lane",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneLeaseState {
    Active,
    Released,
    Reclaimed,
    Cancelled,
}

impl ModelLaneLeaseState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Released => "released",
            Self::Reclaimed => "reclaimed",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneDiagnosticTier {
    FlightRecorder,
    InternalDiagnostics,
    Palmistry,
}

impl ModelLaneDiagnosticTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::FlightRecorder => "flight_recorder",
            Self::InternalDiagnostics => "internal_diagnostics",
            Self::Palmistry => "palmistry",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneDiagnosticTierState {
    Wired,
    NotApplicableWithReason,
    DeferredWithReason,
    Missing,
}

impl ModelLaneDiagnosticTierState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Wired => "wired",
            Self::NotApplicableWithReason => "not_applicable_with_reason",
            Self::DeferredWithReason => "deferred_with_reason",
            Self::Missing => "missing",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneMtRuntimeStatus {
    Pending,
    Claimed,
    Blocked,
    ProofRunning,
    ReadyForValidation,
    Completed,
}

impl ModelLaneMtRuntimeStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Claimed => "claimed",
            Self::Blocked => "blocked",
            Self::ProofRunning => "proof_running",
            Self::ReadyForValidation => "ready_for_validation",
            Self::Completed => "completed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneMessageKind {
    Proposal,
    Critique,
    ToolRequest,
    ToolResult,
    Status,
    PromotionRequest,
    Recovery,
}

impl ModelLaneMessageKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Proposal => "proposal",
            Self::Critique => "critique",
            Self::ToolRequest => "tool_request",
            Self::ToolResult => "tool_result",
            Self::Status => "status",
            Self::PromotionRequest => "promotion_request",
            Self::Recovery => "recovery",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneAuthority {
    Advisory,
    PromotionCandidate,
    Promoted,
    OperatorDecision,
    ValidatorVerdict,
}

impl ModelLaneAuthority {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Advisory => "advisory",
            Self::PromotionCandidate => "promotion_candidate",
            Self::Promoted => "promoted",
            Self::OperatorDecision => "operator_decision",
            Self::ValidatorVerdict => "validator_verdict",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneRoutingPolicy {
    LocalFirst,
    CloudReview,
    CloudPlanLocalExecute,
    ParallelDebate,
    ValidatorLane,
    OperatorLane,
}

impl ModelLaneRoutingPolicy {
    pub fn all() -> &'static [Self] {
        &[
            Self::LocalFirst,
            Self::CloudReview,
            Self::CloudPlanLocalExecute,
            Self::ParallelDebate,
            Self::ValidatorLane,
            Self::OperatorLane,
        ]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LocalFirst => "local_first",
            Self::CloudReview => "cloud_review",
            Self::CloudPlanLocalExecute => "cloud_plan_local_execute",
            Self::ParallelDebate => "parallel_debate",
            Self::ValidatorLane => "validator_lane",
            Self::OperatorLane => "operator_lane",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLanePromotionState {
    Advisory,
    PromotionRequested,
    PendingPolicy,
    PendingApproval,
    Approved,
    Denied,
    Expired,
    Executing,
    Executed,
    Skipped,
    Unsupported,
}

impl ModelLanePromotionState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Advisory => "advisory",
            Self::PromotionRequested => "promotion_requested",
            Self::PendingPolicy => "pending_policy",
            Self::PendingApproval => "pending_approval",
            Self::Approved => "approved",
            Self::Denied => "denied",
            Self::Expired => "expired",
            Self::Executing => "executing",
            Self::Executed => "executed",
            Self::Skipped => "skipped",
            Self::Unsupported => "unsupported",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLanePromotionOutcome {
    Approved,
    Denied,
}

impl ModelLanePromotionOutcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Approved => "approved",
            Self::Denied => "denied",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLanePromotionDenialReason {
    StaleBase,
    StaleStateVector,
    SchemaMismatch,
    AggregateVersionMismatch,
    InputRefMismatch,
    DirectAuthorityMutation,
    MissingPromotionAuthority,
    MissingPromotedArtifactBinding,
}

impl ModelLanePromotionDenialReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::StaleBase => "stale_base",
            Self::StaleStateVector => "stale_state_vector",
            Self::SchemaMismatch => "schema_mismatch",
            Self::AggregateVersionMismatch => "aggregate_version_mismatch",
            Self::InputRefMismatch => "input_ref_mismatch",
            Self::DirectAuthorityMutation => "direct_authority_mutation",
            Self::MissingPromotionAuthority => "missing_promotion_authority",
            Self::MissingPromotedArtifactBinding => "missing_promoted_artifact_binding",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneHandoffSelectionState {
    Selected,
    Rejected,
    Unresolved,
    Superseded,
}

impl ModelLaneHandoffSelectionState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Selected => "selected",
            Self::Rejected => "rejected",
            Self::Unresolved => "unresolved",
            Self::Superseded => "superseded",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneHandoffSourceKind {
    Proposal,
    Critique,
    ToolRequest,
    ToolResult,
    Status,
    PromotionRequest,
    Recovery,
}

impl ModelLaneHandoffSourceKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Proposal => "proposal",
            Self::Critique => "critique",
            Self::ToolRequest => "tool_request",
            Self::ToolResult => "tool_result",
            Self::Status => "status",
            Self::PromotionRequest => "promotion_request",
            Self::Recovery => "recovery",
        }
    }

    fn from_message_kind(kind: &ModelLaneMessageKind) -> Self {
        match kind {
            ModelLaneMessageKind::Proposal => Self::Proposal,
            ModelLaneMessageKind::Critique => Self::Critique,
            ModelLaneMessageKind::ToolRequest => Self::ToolRequest,
            ModelLaneMessageKind::ToolResult => Self::ToolResult,
            ModelLaneMessageKind::Status => Self::Status,
            ModelLaneMessageKind::PromotionRequest => Self::PromotionRequest,
            ModelLaneMessageKind::Recovery => Self::Recovery,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "target_kind", content = "target_id", rename_all = "snake_case")]
pub enum ModelLaneTarget {
    Lane(String),
    Broadcast,
    Coordinator,
}

fn model_lane_target_label(target: &ModelLaneTarget) -> String {
    match target {
        ModelLaneTarget::Lane(lane_id) => format!("lane:{lane_id}"),
        ModelLaneTarget::Broadcast => "broadcast".to_owned(),
        ModelLaneTarget::Coordinator => "coordinator".to_owned(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelLaneLocusBinding {
    pub work_packet_id: String,
    pub micro_task_id: String,
    pub task_board_id: Option<String>,
    pub coordinator_session_id: String,
    pub session_id: String,
    pub model_session_id: String,
    pub owner_session: String,
    pub locus_binding_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelLaneRoutingMetadata {
    pub target_role: String,
    pub target_session: String,
    pub correlation_id: String,
    pub requires_ack: bool,
    pub ack_for: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DexterityLaunchContract {
    pub run_id: String,
    pub lane_id: String,
    #[serde(default)]
    pub restart_generation: i64,
    pub trace_id: String,
    pub run_span_id: String,
    pub lane_span_id: String,
    pub routing_policy: String,
    pub context_bundle_id: String,
    pub event_ledger_stream_id: String,
    pub artifact_namespace: String,
    pub task_board_id: String,
    pub locus_binding_ref: String,
    pub role: String,
    pub backend: String,
    pub adapter_id: String,
    pub capability_token_ids: Vec<String>,
    pub effective_capability_snapshot_ref: String,
    pub projection_plan_ref: Option<String>,
    pub consent_receipt_ref: Option<String>,
    pub tool_gate_decision_refs: Vec<String>,
    pub memory_pack_ref: String,
    pub memory_pack_hash: String,
    pub determinism_mode: String,
    pub budget_summary_ref: String,
    pub candidate_model_ids: Vec<String>,
    pub procedural_review_status: String,
    pub truncation_warning_ref: Option<String>,
    pub rejection_reason_refs: Vec<String>,
    pub run_recovery_hint_ref: Option<String>,
    pub lane_recovery_hint_ref: Option<String>,
}

impl DexterityLaunchContract {
    pub fn attach_to_spawn_request(
        mut request: SpawnRequest,
        work_packet_id: impl Into<String>,
        micro_task_id: impl Into<String>,
    ) -> ModelLaneResult<SpawnRequest> {
        request = request.with_wp(work_packet_id).with_mt(micro_task_id);
        let contract = Self::from_spawn_request(&request)?;
        Ok(request.with_dexterity_launch(contract))
    }

    pub fn from_spawn_request(request: &SpawnRequest) -> ModelLaneResult<Self> {
        required_request_field("wp_id", request.wp_id.as_deref())?;
        required_request_field("mt_id", request.mt_id.as_deref())?;
        let adapter_kind = dexterity_adapter_kind_for_spawn(request)?;
        let registry = DexterityLaunchAdapterRegistry::standard();
        let descriptor = registry.descriptor(&adapter_kind)?;
        let run_uuid = Uuid::now_v7();
        let lane_uuid = Uuid::now_v7();
        let run_id = format!("dexterity-run-{run_uuid}");
        let lane_id = format!(
            "dexterity-lane-{}-{lane_uuid}",
            descriptor.adapter_kind.as_str()
        );
        let trace_id = format!("trace-dexterity-{run_uuid}");
        let task_board_id = request
            .swarm_id
            .as_deref()
            .map(|swarm| format!("task-board://swarm-runtime/{swarm}"))
            .unwrap_or_else(|| "task-board://swarm-runtime/unassigned".to_string());
        let candidate_model_ids = dexterity_candidate_model_ids(request);
        let projection_plan_ref = descriptor
            .requires_projection_plan
            .then(|| format!("projection-plan://dexterity/{lane_id}"));
        let consent_receipt_ref = descriptor.requires_consent_receipt.then(|| {
            format!(
                "consent://dexterity/{}/{}",
                descriptor.provider_kind.as_str(),
                lane_id
            )
        });
        let memory_pack_ref = format!("memory-pack://dexterity/{run_id}");
        let memory_pack_hash = dexterity_sha256_hex(format!(
            "{}:{}:{}:{}",
            request.instance_id,
            request.parent_session_id,
            descriptor.adapter_kind.as_str(),
            request
                .model_artifact_sha256
                .as_deref()
                .or(request.cloud_model_name.as_deref())
                .unwrap_or("no-model-material")
        ));
        Ok(Self {
            run_id: run_id.clone(),
            lane_id: lane_id.clone(),
            restart_generation: 0,
            trace_id,
            run_span_id: format!("span-{run_id}-run"),
            lane_span_id: format!("span-{lane_id}-lane"),
            routing_policy: format!("dexterity_{}", descriptor.runtime_binding.as_str()),
            context_bundle_id: format!("context-bundle://dexterity/{}", request.parent_session_id),
            event_ledger_stream_id: format!("event-ledger://dexterity/{run_id}"),
            artifact_namespace: format!("artifact://dexterity/{run_id}"),
            task_board_id,
            locus_binding_ref: format!(
                "locus://dexterity/{}/{}/{}",
                request.wp_id.as_deref().unwrap_or("unknown-wp"),
                request.mt_id.as_deref().unwrap_or("unknown-mt"),
                lane_id
            ),
            role: request.owner_role.clone(),
            backend: descriptor.default_backend.clone(),
            adapter_id: descriptor.default_adapter_id.clone(),
            capability_token_ids: descriptor.required_capability_tokens.clone(),
            effective_capability_snapshot_ref: format!("capability-snapshot://dexterity/{lane_id}"),
            projection_plan_ref,
            consent_receipt_ref,
            tool_gate_decision_refs: vec![format!("toolgate://dexterity/{lane_id}/read-context")],
            memory_pack_ref,
            memory_pack_hash,
            determinism_mode: "deterministic_replay".into(),
            budget_summary_ref: format!("budget://dexterity/{run_id}"),
            candidate_model_ids,
            procedural_review_status: "runtime_preflight".into(),
            truncation_warning_ref: None,
            rejection_reason_refs: vec!["rejection://dexterity/no-bypass-authority".into()],
            run_recovery_hint_ref: Some("usermanual://model-lane-launch-adapters#recovery".into()),
            lane_recovery_hint_ref: Some("usermanual://model-lane-launch-adapters#recovery".into()),
        })
    }

    fn preflight_for_spawn_request(
        &self,
        request: &SpawnRequest,
        descriptor: &DexterityLaunchAdapterDescriptor,
    ) -> ModelLaneResult<()> {
        required_request_field("wp_id", request.wp_id.as_deref())?;
        required_request_field("mt_id", request.mt_id.as_deref())?;
        require_token("parent_session_id", &request.parent_session_id)?;
        require_token("owner_role", &request.owner_role)?;
        require_token("run_id", &self.run_id)?;
        require_token("lane_id", &self.lane_id)?;
        require_token("trace_id", &self.trace_id)?;
        require_token("run_span_id", &self.run_span_id)?;
        require_token("lane_span_id", &self.lane_span_id)?;
        require_token("routing_policy", &self.routing_policy)?;
        require_token("context_bundle_id", &self.context_bundle_id)?;
        require_token("event_ledger_stream_id", &self.event_ledger_stream_id)?;
        require_token("artifact_namespace", &self.artifact_namespace)?;
        require_token("task_board_id", &self.task_board_id)?;
        require_token("locus_binding_ref", &self.locus_binding_ref)?;
        require_token("role", &self.role)?;
        require_token("backend", &self.backend)?;
        require_token("adapter_id", &self.adapter_id)?;
        require_token(
            "effective_capability_snapshot_ref",
            &self.effective_capability_snapshot_ref,
        )?;
        require_token("memory_pack_ref", &self.memory_pack_ref)?;
        validate_sha256("memory_pack_hash", &self.memory_pack_hash)?;
        require_token("determinism_mode", &self.determinism_mode)?;
        require_token("budget_summary_ref", &self.budget_summary_ref)?;
        require_token("procedural_review_status", &self.procedural_review_status)?;
        if self.restart_generation < 0 {
            return Err(ModelLaneError::InvalidInput(
                "Dexterity launch preflight requires non-negative restart_generation".into(),
            ));
        }
        if self.capability_token_ids.is_empty() {
            return Err(ModelLaneError::InvalidInput(
                "Dexterity launch preflight requires capability_token_ids".into(),
            ));
        }
        if self.tool_gate_decision_refs.is_empty() {
            return Err(ModelLaneError::InvalidInput(
                "Dexterity launch preflight requires tool_gate_decision_refs".into(),
            ));
        }
        if self.candidate_model_ids.is_empty() {
            return Err(ModelLaneError::InvalidInput(
                "Dexterity launch preflight requires candidate_model_ids".into(),
            ));
        }
        for capability in &self.capability_token_ids {
            require_token("capability_token_ids[]", capability)?;
        }
        for decision_ref in &self.tool_gate_decision_refs {
            require_token("tool_gate_decision_refs[]", decision_ref)?;
        }
        if descriptor.requires_projection_plan {
            require_optional_token("projection_plan_ref", self.projection_plan_ref.as_deref())?;
        }
        if descriptor.requires_consent_receipt {
            require_optional_token("consent_receipt_ref", self.consent_receipt_ref.as_deref())?;
        }
        Ok(())
    }

    fn to_run(
        &self,
        request: &SpawnRequest,
        live: &LiveSession,
    ) -> ModelLaneResult<NewModelLaneRun> {
        let work_packet_id = required_request_field("wp_id", request.wp_id.as_deref())?;
        let micro_task_id = required_request_field("mt_id", request.mt_id.as_deref())?;
        let locus = self.locus(request, live)?;
        Ok(NewModelLaneRun {
            run_id: self.run_id.clone(),
            trace_id: self.trace_id.clone(),
            run_span_id: self.run_span_id.clone(),
            coordinator_session_id: request.parent_session_id.clone(),
            routing_policy: self.routing_policy.clone(),
            context_bundle_id: self.context_bundle_id.clone(),
            lane_ids: vec![self.lane_id.clone()],
            event_ledger_stream_id: self.event_ledger_stream_id.clone(),
            artifact_namespace: self.artifact_namespace.clone(),
            projection_plan_ref: self.projection_plan_ref.clone(),
            consent_receipt_ref: self.consent_receipt_ref.clone(),
            work_packet_id: Some(work_packet_id),
            micro_task_id: Some(micro_task_id),
            task_board_id: Some(self.task_board_id.clone()),
            owner_session: request.owner_role.clone(),
            idempotency_key: format!("dexterity-launch-run:{}:{}", self.run_id, self.lane_id),
            replay_order_key: format!("{}:00000000:run", self.run_id),
            replay_after_event_ledger_seq: None,
            recovery_state: ModelLaneRecoveryState::Restartable,
            failstate_code: None,
            reason_ref: None,
            recovery_hint_ref: self.run_recovery_hint_ref.clone(),
            locus_binding: Some(locus),
            memory_pack_ref: self.memory_pack_ref.clone(),
            memory_pack_hash: self.memory_pack_hash.clone(),
            determinism_mode: self.determinism_mode.clone(),
            budget_summary_ref: self.budget_summary_ref.clone(),
            selected_model_id: Some(self.persisted_model_id(request, live)),
            candidate_model_ids: self.candidate_model_ids.clone(),
            procedural_review_status: self.procedural_review_status.clone(),
            truncation_warning_ref: self.truncation_warning_ref.clone(),
            rejection_reason_refs: self.rejection_reason_refs.clone(),
        })
    }

    fn to_failed_run(
        &self,
        request: &SpawnRequest,
        failure_code: &str,
        reason_ref: &str,
    ) -> ModelLaneResult<NewModelLaneRun> {
        let work_packet_id = required_request_field("wp_id", request.wp_id.as_deref())?;
        let micro_task_id = required_request_field("mt_id", request.mt_id.as_deref())?;
        let model_session_id = failed_model_session_id(request);
        let locus = self.failed_locus(request, &model_session_id)?;
        let candidate_model_ids = self.candidate_model_ids(request);
        Ok(NewModelLaneRun {
            run_id: self.run_id.clone(),
            trace_id: self.trace_id.clone(),
            run_span_id: self.run_span_id.clone(),
            coordinator_session_id: request.parent_session_id.clone(),
            routing_policy: self.routing_policy.clone(),
            context_bundle_id: self.context_bundle_id.clone(),
            lane_ids: vec![self.lane_id.clone()],
            event_ledger_stream_id: self.event_ledger_stream_id.clone(),
            artifact_namespace: self.artifact_namespace.clone(),
            projection_plan_ref: self.projection_plan_ref.clone(),
            consent_receipt_ref: self.consent_receipt_ref.clone(),
            work_packet_id: Some(work_packet_id),
            micro_task_id: Some(micro_task_id),
            task_board_id: Some(self.task_board_id.clone()),
            owner_session: request.owner_role.clone(),
            idempotency_key: format!(
                "dexterity-launch-failed-run:{}:{}",
                self.run_id, self.lane_id
            ),
            replay_order_key: format!("{}:00000000:failed-run", self.run_id),
            replay_after_event_ledger_seq: None,
            recovery_state: ModelLaneRecoveryState::Reclaimable,
            failstate_code: Some(failure_code.to_string()),
            reason_ref: Some(reason_ref.to_string()),
            recovery_hint_ref: self.run_recovery_hint_ref.clone(),
            locus_binding: Some(locus),
            memory_pack_ref: self.memory_pack_ref.clone(),
            memory_pack_hash: self.memory_pack_hash.clone(),
            determinism_mode: self.determinism_mode.clone(),
            budget_summary_ref: self.budget_summary_ref.clone(),
            selected_model_id: Some(request.instance_id.model_id.to_string()),
            candidate_model_ids,
            procedural_review_status: self.procedural_review_status.clone(),
            truncation_warning_ref: self.truncation_warning_ref.clone(),
            rejection_reason_refs: self.rejection_reason_refs.clone(),
        })
    }

    fn to_lane(&self, request: &SpawnRequest, live: &LiveSession) -> ModelLaneResult<NewModelLane> {
        let work_packet_id = required_request_field("wp_id", request.wp_id.as_deref())?;
        let micro_task_id = required_request_field("mt_id", request.mt_id.as_deref())?;
        let mapped = map_spawn_provider(request)?;
        let heartbeat = chrono::Utc::now();
        let process_ownership_ref =
            format!("process-ledger://{}", live.process_record_id.as_uuid());
        let provider_feature_profile_ref = format!(
            "provider-feature-profile://{}",
            mapped.provider_kind.as_str()
        );
        let requested_execution_policy_ref = format!(
            "execution-policy://requested/{}",
            mapped.runtime_binding.as_str()
        );
        let effective_execution_policy_ref = format!(
            "execution-policy://effective/{}",
            mapped.launch_authority.as_str()
        );
        let terminal_status_mapping_ref = format!(
            "terminal-status://session-broker/{}",
            mapped.runtime_binding.as_str()
        );
        Ok(NewModelLane {
            lane_id: self.lane_id.clone(),
            run_id: self.run_id.clone(),
            trace_id: self.trace_id.clone(),
            lane_span_id: self.lane_span_id.clone(),
            event_ledger_stream_id: self.event_ledger_stream_id.clone(),
            kind: mapped.kind,
            role: self.role.clone(),
            backend: self.backend.clone(),
            model_id: Some(self.persisted_model_id(request, live)),
            session_id: runtime_session_id(request),
            model_session_id: dexterity_spawn_model_session_id(request),
            adapter_id: self.adapter_id.clone(),
            runtime_binding: mapped.runtime_binding,
            launch_authority: mapped.launch_authority,
            provider_kind: mapped.provider_kind,
            capability_token_ids: self.capability_token_ids.clone(),
            effective_capability_snapshot_ref: Some(self.effective_capability_snapshot_ref.clone()),
            capability_negotiation_ref: Some(format!(
                "capability-negotiation://{}",
                self.effective_capability_snapshot_ref
            )),
            provider_feature_profile_ref: Some(provider_feature_profile_ref),
            requested_execution_policy_ref: Some(requested_execution_policy_ref),
            effective_execution_policy_ref: Some(effective_execution_policy_ref),
            projection_plan_ref: self.projection_plan_ref.clone(),
            consent_receipt_ref: self.consent_receipt_ref.clone(),
            tool_gate_decision_refs: self.tool_gate_decision_refs.clone(),
            status: ModelLaneStatus::Ready,
            recovery_state: ModelLaneRecoveryState::Restartable,
            heartbeat_at_utc: Some(heartbeat.to_rfc3339()),
            lease_expires_at_utc: Some((heartbeat + chrono::Duration::minutes(5)).to_rfc3339()),
            reclaim_after_utc: Some((heartbeat + chrono::Duration::minutes(6)).to_rfc3339()),
            restart_generation: self.restart_generation,
            cancellation_ref: Some(format!("cancel-token://{}", self.lane_id)),
            reclaim_policy_ref: Some("reclaim-policy://swarm-coordinator-lease".into()),
            terminal_status_mapping_ref: Some(terminal_status_mapping_ref),
            process_ownership_ref: Some(process_ownership_ref.clone()),
            no_os_process_reason_ref: None,
            backpressure_ref: None,
            loop_counter_ref: Some(format!("budget://{}", self.budget_summary_ref)),
            last_runtime_status_ref: Some(process_ownership_ref),
            last_recovery_event_ref: None,
            failstate_code: None,
            startup_failure_ref: None,
            reason_ref: None,
            recovery_hint_ref: self.lane_recovery_hint_ref.clone(),
            work_packet_id: Some(work_packet_id),
            micro_task_id: Some(micro_task_id),
            task_board_id: Some(self.task_board_id.clone()),
            owner_session: request.owner_role.clone(),
            locus_binding: Some(self.locus(request, live)?),
        })
    }

    fn to_failed_lane(
        &self,
        request: &SpawnRequest,
        failure_code: &str,
        startup_failure_ref: &str,
        reason_ref: &str,
    ) -> ModelLaneResult<NewModelLane> {
        let work_packet_id = required_request_field("wp_id", request.wp_id.as_deref())?;
        let micro_task_id = required_request_field("mt_id", request.mt_id.as_deref())?;
        let mapped = map_spawn_provider(request)?;
        let heartbeat = chrono::Utc::now();
        let model_session_id = failed_model_session_id(request);
        let runtime_binding = mapped.runtime_binding.clone();
        let launch_authority = mapped.launch_authority.clone();
        let provider_kind = mapped.provider_kind.clone();
        let terminal_status_mapping_ref = format!(
            "terminal-status://session-broker/{}",
            runtime_binding.as_str()
        );
        let provider_feature_profile_ref =
            format!("provider-feature-profile://{}", provider_kind.as_str());
        let requested_execution_policy_ref =
            format!("execution-policy://requested/{}", runtime_binding.as_str());
        let effective_execution_policy_ref =
            format!("execution-policy://effective/{}", launch_authority.as_str());
        Ok(NewModelLane {
            lane_id: self.lane_id.clone(),
            run_id: self.run_id.clone(),
            trace_id: self.trace_id.clone(),
            lane_span_id: self.lane_span_id.clone(),
            event_ledger_stream_id: self.event_ledger_stream_id.clone(),
            kind: mapped.kind,
            role: self.role.clone(),
            backend: self.backend.clone(),
            model_id: Some(request.instance_id.model_id.to_string()),
            session_id: runtime_session_id(request),
            model_session_id: model_session_id.clone(),
            adapter_id: self.adapter_id.clone(),
            runtime_binding,
            launch_authority,
            provider_kind,
            capability_token_ids: self.capability_token_ids.clone(),
            effective_capability_snapshot_ref: Some(self.effective_capability_snapshot_ref.clone()),
            capability_negotiation_ref: Some(format!(
                "capability-negotiation://{}",
                self.effective_capability_snapshot_ref
            )),
            provider_feature_profile_ref: Some(provider_feature_profile_ref),
            requested_execution_policy_ref: Some(requested_execution_policy_ref),
            effective_execution_policy_ref: Some(effective_execution_policy_ref),
            projection_plan_ref: self.projection_plan_ref.clone(),
            consent_receipt_ref: self.consent_receipt_ref.clone(),
            tool_gate_decision_refs: self.tool_gate_decision_refs.clone(),
            status: ModelLaneStatus::Failed,
            recovery_state: ModelLaneRecoveryState::Reclaimable,
            heartbeat_at_utc: Some(heartbeat.to_rfc3339()),
            lease_expires_at_utc: Some((heartbeat + chrono::Duration::minutes(5)).to_rfc3339()),
            reclaim_after_utc: Some((heartbeat + chrono::Duration::minutes(6)).to_rfc3339()),
            restart_generation: self.restart_generation,
            cancellation_ref: Some(format!("cancel-token://{}", self.lane_id)),
            reclaim_policy_ref: Some("reclaim-policy://failed-startup".into()),
            terminal_status_mapping_ref: Some(terminal_status_mapping_ref),
            process_ownership_ref: None,
            no_os_process_reason_ref: Some(format!(
                "no-os-process://factory-create-failed/{}",
                self.lane_id
            )),
            backpressure_ref: None,
            loop_counter_ref: Some(format!("budget://{}", self.budget_summary_ref)),
            last_runtime_status_ref: Some(startup_failure_ref.to_string()),
            last_recovery_event_ref: None,
            failstate_code: Some(failure_code.to_string()),
            startup_failure_ref: Some(startup_failure_ref.to_string()),
            reason_ref: Some(reason_ref.to_string()),
            recovery_hint_ref: self.lane_recovery_hint_ref.clone(),
            work_packet_id: Some(work_packet_id),
            micro_task_id: Some(micro_task_id),
            task_board_id: Some(self.task_board_id.clone()),
            owner_session: request.owner_role.clone(),
            locus_binding: Some(self.failed_locus(request, &model_session_id)?),
        })
    }

    fn locus(
        &self,
        request: &SpawnRequest,
        _live: &LiveSession,
    ) -> ModelLaneResult<ModelLaneLocusBinding> {
        Ok(ModelLaneLocusBinding {
            work_packet_id: required_request_field("wp_id", request.wp_id.as_deref())?,
            micro_task_id: required_request_field("mt_id", request.mt_id.as_deref())?,
            task_board_id: Some(self.task_board_id.clone()),
            coordinator_session_id: request.parent_session_id.clone(),
            session_id: runtime_session_id(request),
            model_session_id: dexterity_spawn_model_session_id(request),
            owner_session: request.owner_role.clone(),
            locus_binding_ref: self.locus_binding_ref.clone(),
        })
    }

    fn failed_locus(
        &self,
        request: &SpawnRequest,
        model_session_id: &str,
    ) -> ModelLaneResult<ModelLaneLocusBinding> {
        Ok(ModelLaneLocusBinding {
            work_packet_id: required_request_field("wp_id", request.wp_id.as_deref())?,
            micro_task_id: required_request_field("mt_id", request.mt_id.as_deref())?,
            task_board_id: Some(self.task_board_id.clone()),
            coordinator_session_id: request.parent_session_id.clone(),
            session_id: runtime_session_id(request),
            model_session_id: model_session_id.to_string(),
            owner_session: request.owner_role.clone(),
            locus_binding_ref: self.locus_binding_ref.clone(),
        })
    }

    fn candidate_model_ids(&self, request: &SpawnRequest) -> Vec<String> {
        if self.candidate_model_ids.is_empty() {
            vec![request.instance_id.model_id.to_string()]
        } else {
            self.candidate_model_ids.clone()
        }
    }

    fn persisted_model_id(&self, request: &SpawnRequest, live: &LiveSession) -> String {
        if request.provider == Some(ProviderKind::ByokCloud) {
            return self
                .candidate_model_ids(request)
                .into_iter()
                .next()
                .unwrap_or_else(|| live.model_id.to_string());
        }
        live.model_id.to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewModelLaneRun {
    pub run_id: String,
    pub trace_id: String,
    pub run_span_id: String,
    pub coordinator_session_id: String,
    pub routing_policy: String,
    pub context_bundle_id: String,
    pub lane_ids: Vec<String>,
    pub event_ledger_stream_id: String,
    pub artifact_namespace: String,
    pub projection_plan_ref: Option<String>,
    pub consent_receipt_ref: Option<String>,
    pub work_packet_id: Option<String>,
    pub micro_task_id: Option<String>,
    pub task_board_id: Option<String>,
    pub owner_session: String,
    pub idempotency_key: String,
    pub replay_order_key: String,
    pub replay_after_event_ledger_seq: Option<i64>,
    pub recovery_state: ModelLaneRecoveryState,
    pub failstate_code: Option<String>,
    pub reason_ref: Option<String>,
    pub recovery_hint_ref: Option<String>,
    pub locus_binding: Option<ModelLaneLocusBinding>,
    pub memory_pack_ref: String,
    pub memory_pack_hash: String,
    pub determinism_mode: String,
    pub budget_summary_ref: String,
    pub selected_model_id: Option<String>,
    pub candidate_model_ids: Vec<String>,
    pub procedural_review_status: String,
    pub truncation_warning_ref: Option<String>,
    pub rejection_reason_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelLaneRunRecord {
    #[serde(flatten)]
    pub inner: NewModelLaneRun,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
}

impl Deref for ModelLaneRunRecord {
    type Target = NewModelLaneRun;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewModelLane {
    pub lane_id: String,
    pub run_id: String,
    pub trace_id: String,
    pub lane_span_id: String,
    pub event_ledger_stream_id: String,
    pub kind: ModelLaneKind,
    pub role: String,
    pub backend: String,
    pub model_id: Option<String>,
    pub session_id: String,
    pub model_session_id: String,
    pub adapter_id: String,
    pub runtime_binding: RuntimeBinding,
    pub launch_authority: LaunchAuthority,
    pub provider_kind: ModelLaneProviderKind,
    pub capability_token_ids: Vec<String>,
    pub effective_capability_snapshot_ref: Option<String>,
    pub capability_negotiation_ref: Option<String>,
    pub provider_feature_profile_ref: Option<String>,
    pub requested_execution_policy_ref: Option<String>,
    pub effective_execution_policy_ref: Option<String>,
    pub projection_plan_ref: Option<String>,
    pub consent_receipt_ref: Option<String>,
    pub tool_gate_decision_refs: Vec<String>,
    pub status: ModelLaneStatus,
    pub recovery_state: ModelLaneRecoveryState,
    pub heartbeat_at_utc: Option<String>,
    pub lease_expires_at_utc: Option<String>,
    pub reclaim_after_utc: Option<String>,
    pub restart_generation: i64,
    pub cancellation_ref: Option<String>,
    pub reclaim_policy_ref: Option<String>,
    pub terminal_status_mapping_ref: Option<String>,
    pub process_ownership_ref: Option<String>,
    pub no_os_process_reason_ref: Option<String>,
    pub backpressure_ref: Option<String>,
    pub loop_counter_ref: Option<String>,
    pub last_runtime_status_ref: Option<String>,
    pub last_recovery_event_ref: Option<String>,
    pub failstate_code: Option<String>,
    pub startup_failure_ref: Option<String>,
    pub reason_ref: Option<String>,
    pub recovery_hint_ref: Option<String>,
    pub work_packet_id: Option<String>,
    pub micro_task_id: Option<String>,
    pub task_board_id: Option<String>,
    pub owner_session: String,
    pub locus_binding: Option<ModelLaneLocusBinding>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelLaneRecord {
    #[serde(flatten)]
    pub inner: NewModelLane,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
}

impl Deref for ModelLaneRecord {
    type Target = NewModelLane;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewModelLaneMessage {
    pub message_id: String,
    pub run_id: String,
    pub trace_id: String,
    pub message_span_id: String,
    pub parent_span_id: Option<String>,
    pub linked_span_contexts: Vec<String>,
    pub from_lane_id: String,
    pub to_lane: ModelLaneTarget,
    #[serde(default)]
    pub routing: Option<ModelLaneRoutingMetadata>,
    pub kind: ModelLaneMessageKind,
    pub payload_ref: String,
    pub payload_sha256: String,
    pub event_ledger_stream_id: String,
    pub summary: String,
    pub authority: ModelLaneAuthority,
    #[serde(default)]
    pub promotion_decision_id: Option<String>,
    pub promotion_gate_ref: Option<String>,
    pub promotion_receipt_ref: Option<String>,
    pub validator_verdict_ref: Option<String>,
    pub operator_decision_ref: Option<String>,
    pub promoted_artifact_ref: Option<String>,
    pub promoted_artifact_sha256: Option<String>,
    pub promoted_artifact_version: Option<String>,
    pub tool_gate_decision_refs: Vec<String>,
    pub coordinator_session_id: String,
    pub work_packet_id: Option<String>,
    pub micro_task_id: Option<String>,
    pub task_board_id: Option<String>,
    pub owner_session: String,
    pub locus_binding: Option<ModelLaneLocusBinding>,
    pub idempotency_key: String,
    pub replay_order_key: String,
    pub replay_after_event_ledger_seq: Option<i64>,
    pub proposal_ref: Option<String>,
    pub crdt_update_ref: Option<String>,
    pub crdt_base_snapshot_ref: Option<String>,
    pub crdt_state_vector: Option<String>,
    pub crdt_proposal_ref: Option<String>,
    pub crdt_stale_base_ref: Option<String>,
    pub failstate_code: Option<String>,
    pub reason_ref: Option<String>,
    pub recovery_hint_ref: Option<String>,
    pub created_at_utc: String,
    pub diagnostic_payload: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelLaneMessageRecord {
    #[serde(flatten)]
    pub inner: NewModelLaneMessage,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crdt_authority_binding: Option<ModelLaneCrdtAuthorityBinding>,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
    pub event_stream_version: i64,
    pub transaction_seq: i64,
}

impl Deref for ModelLaneMessageRecord {
    type Target = NewModelLaneMessage;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// Server-derived, durable ownership and replay binding for a CRDT-bearing
/// ModelLane message. The binding is persisted in both the message projection
/// and its EventLedger payload; callers cannot supply or override it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelLaneCrdtAuthorityBinding {
    pub run_id: String,
    pub lane_id: String,
    pub lane_session_id: String,
    pub model_session_id: String,
    pub lane_trace_id: String,
    pub crdt_session_id: String,
    pub crdt_trace_id: String,
    pub workspace_id: String,
    pub document_id: String,
    pub crdt_document_id: String,
    pub actor_id: String,
    pub actor_kind: String,
    pub lease_id: String,
    pub lease_correlation_id: String,
    pub lease_scope_kind: String,
    pub lease_scope_id: String,
    pub lease_claimed_at_utc: DateTime<Utc>,
    pub lease_expires_at_utc: DateTime<Utc>,
    pub lease_admitted_at_utc: DateTime<Utc>,
    pub crdt_site_id: String,
    pub update_id: String,
    pub update_seq: i64,
    pub update_bytes_ref: String,
    pub base_snapshot_ref: String,
    pub state_vector: String,
    /// Canonical Yjs v1 state-vector bytes derived from the locked snapshot
    /// and update bytes. This is distinct from `state_vector`, which is the
    /// kernel's site-indexed receipt clock.
    #[serde(default)]
    pub yjs_state_vector_b64: String,
    pub materialized_projection_hash: String,
    pub update_event_ledger_event_id: String,
    pub crdt_proposal_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewModelLaneRecoveryCheckpoint {
    pub checkpoint_id: String,
    pub run_id: String,
    pub lane_id: Option<String>,
    pub session_id: String,
    pub model_session_id: String,
    pub lane_status: ModelLaneStatus,
    pub checkpoint_status: ModelLaneRecoveryStatus,
    pub last_event_ledger_seq: i64,
    pub last_message_id: Option<String>,
    pub open_payload_refs: Vec<String>,
    pub lease_id: Option<String>,
    pub idempotency_scope: String,
    pub recovery_state: ModelLaneRecoveryState,
    pub recovery_event_ref: Option<String>,
    pub event_ledger_stream_id: String,
    pub work_packet_id: String,
    pub micro_task_id: String,
    pub task_board_id: String,
    pub owner_session: String,
    pub idempotency_key: String,
    pub created_at_utc: String,
    pub recovery_hint_ref: Option<String>,
    pub diagnostic_payload: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelLaneRecoveryCheckpointRecord {
    #[serde(flatten)]
    pub inner: NewModelLaneRecoveryCheckpoint,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
    pub event_stream_version: i64,
    pub transaction_seq: i64,
}

impl Deref for ModelLaneRecoveryCheckpointRecord {
    type Target = NewModelLaneRecoveryCheckpoint;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewModelLaneRecoveryEvent {
    pub recovery_event_id: String,
    pub run_id: String,
    pub lane_id: Option<String>,
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub linked_span_contexts: Vec<String>,
    pub session_id: Option<String>,
    pub model_session_id: Option<String>,
    pub event_kind: ModelLaneRecoveryEventKind,
    pub recovery_status: ModelLaneRecoveryStatus,
    pub replay_order_seq: i64,
    pub source_event_ledger_seq: Option<i64>,
    pub payload_refs: Vec<String>,
    pub artifact_refs: Vec<String>,
    pub crdt_base_snapshot_ref: Option<String>,
    pub crdt_state_vector: Option<String>,
    pub crdt_stale_base_ref: Option<String>,
    pub lease_id: Option<String>,
    pub failure_kind: Option<ModelLaneRecoveryFailureKind>,
    pub error_code: Option<String>,
    pub replay_hint: String,
    pub event_ledger_stream_id: String,
    pub work_packet_id: String,
    pub micro_task_id: String,
    pub task_board_id: String,
    pub owner_session: String,
    pub idempotency_key: String,
    pub recovery_hint_ref: Option<String>,
    pub diagnostic_payload: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelLaneRecoveryEventRecord {
    #[serde(flatten)]
    pub inner: NewModelLaneRecoveryEvent,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
    pub event_stream_version: i64,
    pub transaction_seq: i64,
}

impl Deref for ModelLaneRecoveryEventRecord {
    type Target = NewModelLaneRecoveryEvent;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewModelLaneLease {
    pub lease_id: String,
    pub run_id: String,
    pub lane_id: Option<String>,
    pub scope: ModelLaneLeaseScope,
    pub scope_ref: String,
    pub holder_actor_id: String,
    pub holder_session_id: String,
    pub lease_expires_at_utc: String,
    pub takeover_policy_ref: String,
    pub state: ModelLaneLeaseState,
    pub event_ledger_stream_id: String,
    pub work_packet_id: String,
    pub micro_task_id: String,
    pub task_board_id: String,
    pub owner_session: String,
    pub idempotency_key: String,
    pub recovery_hint_ref: Option<String>,
    pub diagnostic_payload: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelLaneLeaseRecord {
    #[serde(flatten)]
    pub inner: NewModelLaneLease,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
    pub event_stream_version: i64,
    pub transaction_seq: i64,
}

impl Deref for ModelLaneLeaseRecord {
    type Target = NewModelLaneLease;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewModelLaneDiagnosticTierStatus {
    pub diagnostic_status_id: String,
    pub behavior_id: String,
    pub run_id: String,
    pub tier: ModelLaneDiagnosticTier,
    pub state: ModelLaneDiagnosticTierState,
    pub reason: String,
    pub evidence_ref: String,
    pub follow_up_ref: Option<String>,
    pub event_ledger_stream_id: String,
    pub work_packet_id: String,
    pub micro_task_id: String,
    pub task_board_id: String,
    pub owner_session: String,
    pub idempotency_key: String,
    pub diagnostic_payload: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelLaneDiagnosticTierStatusRecord {
    #[serde(flatten)]
    pub inner: NewModelLaneDiagnosticTierStatus,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
    pub event_stream_version: i64,
    pub transaction_seq: i64,
}

impl Deref for ModelLaneDiagnosticTierStatusRecord {
    type Target = NewModelLaneDiagnosticTierStatus;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelLaneDiagnosticTierPosture {
    pub run_id: String,
    pub behavior_id: String,
    pub tiers: Vec<ModelLaneDiagnosticTierStatusRecord>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewModelLaneMtRuntimeStatus {
    pub mt_status_id: String,
    pub run_id: String,
    pub work_packet_id: String,
    pub micro_task_id: String,
    pub task_board_id: String,
    pub status: ModelLaneMtRuntimeStatus,
    pub claimed_by_ref: Option<String>,
    pub blocker_ref: Option<String>,
    pub missing_resource_ref: Option<String>,
    pub proof_status_ref: Option<String>,
    pub hbr_status_ref: Option<String>,
    pub last_recovery_event_ref: Option<String>,
    pub last_runtime_status_ref: Option<String>,
    pub event_ledger_stream_id: String,
    pub owner_session: String,
    pub idempotency_key: String,
    pub diagnostic_payload: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelLaneMtRuntimeStatusRecord {
    #[serde(flatten)]
    pub inner: NewModelLaneMtRuntimeStatus,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
    pub event_stream_version: i64,
    pub transaction_seq: i64,
}

impl Deref for ModelLaneMtRuntimeStatusRecord {
    type Target = NewModelLaneMtRuntimeStatus;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelLaneCloudConsentDenialRecord {
    pub event_id: String,
    pub event_ledger_seq: i64,
    pub run_id: String,
    pub lane_id: String,
    pub reason_code: String,
    pub failure_kind: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelLaneRecoveredRun {
    pub replay: ModelLaneReplay,
    pub checkpoint: ModelLaneRecoveryCheckpointRecord,
    pub recovery_events: Vec<ModelLaneRecoveryEventRecord>,
    pub active_leases: Vec<ModelLaneLeaseRecord>,
    pub reclaimable_lease_ids: Vec<String>,
    pub cloud_consent_denials: Vec<ModelLaneCloudConsentDenialRecord>,
    pub mt_runtime_statuses: Vec<ModelLaneMtRuntimeStatusRecord>,
}

pub const MODEL_LANE_DIAGNOSTICS_PROJECTION_SCHEMA_ID: &str =
    "hsk.model_lane_diagnostics_projection@3";
pub const MODEL_LANE_DIAGNOSTICS_SURFACE_CONTRACT_ID: &str = "native_swarm_lane_diagnostics";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelLaneDiagnosticsProjection {
    pub schema_id: String,
    pub surface_contract_id: String,
    pub run: ModelLaneDiagnosticsRun,
    pub lanes: Vec<ModelLaneDiagnosticsLane>,
    pub messages: Vec<ModelLaneDiagnosticsMessage>,
    pub diagnostic_tiers: Vec<ModelLaneDiagnosticsTier>,
    pub mt_runtime_statuses: Vec<ModelLaneDiagnosticsMtStatus>,
    pub routing_executions: Vec<super::routing_execution::ModelLaneRoutingExecutionDiagnostics>,
    pub active_lease_count: usize,
    pub reclaimable_lease_ids: Vec<String>,
    pub orphan_state: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelLaneDiagnosticsRun {
    pub run_id: String,
    pub trace_id: String,
    pub run_span_id: String,
    pub coordinator_session_id: String,
    pub routing_policy: String,
    pub artifact_namespace: String,
    pub projection_plan_ref: Option<String>,
    pub consent_receipt_ref: Option<String>,
    pub work_packet_id: Option<String>,
    pub micro_task_id: Option<String>,
    pub task_board_id: Option<String>,
    pub owner_session: String,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
    pub flight_recorder_correlation_id: String,
    pub context_bundle_id: String,
    pub memory_pack_ref: String,
    pub memory_pack_hash: String,
    pub locus_ref: Option<String>,
    pub loom_ref: Option<String>,
    pub fems_ref: Option<String>,
    pub status: String,
    pub recovery_hint_ref: Option<String>,
    pub selected_model_id: Option<String>,
    pub candidate_model_ids: Vec<String>,
    pub budget_summary_ref: String,
    pub determinism_mode: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelLaneDiagnosticsLane {
    pub lane_id: String,
    pub kind: String,
    pub role: String,
    pub backend: String,
    pub status: String,
    pub recovery_state: String,
    pub model_id: Option<String>,
    pub model_display_name: String,
    pub model_stable_anchor: Option<String>,
    pub model_anchor_unavailable_reason: Option<String>,
    pub session_id: String,
    pub model_session_id: String,
    pub adapter_id: String,
    pub provider_kind: String,
    pub runtime_binding: String,
    pub launch_authority: String,
    pub capability_token_ids: Vec<String>,
    pub effective_capability_snapshot_ref: Option<String>,
    pub capability_negotiation_ref: Option<String>,
    pub provider_feature_profile_ref: Option<String>,
    pub requested_execution_policy_ref: Option<String>,
    pub effective_execution_policy_ref: Option<String>,
    pub projection_plan_ref: Option<String>,
    pub consent_receipt_ref: Option<String>,
    pub tool_gate_decision_refs: Vec<String>,
    pub trace_id: String,
    pub lane_span_id: String,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
    pub flight_recorder_correlation_id: String,
    pub last_activity_utc: Option<String>,
    pub message_count: usize,
    pub payload_error_count: usize,
    pub orphan_state: String,
    pub cancellation_ref: Option<String>,
    pub reclaim_policy_ref: Option<String>,
    pub terminal_status_mapping_ref: Option<String>,
    pub process_ownership_ref: Option<String>,
    pub no_os_process_reason_ref: Option<String>,
    pub last_runtime_status_ref: Option<String>,
    pub last_recovery_event_ref: Option<String>,
    pub failstate_code: Option<String>,
    pub startup_failure_ref: Option<String>,
    pub reason_ref: Option<String>,
    pub recovery_hint_ref: Option<String>,
    pub work_packet_id: Option<String>,
    pub micro_task_id: Option<String>,
    pub task_board_id: Option<String>,
    pub owner_session: String,
    pub locus_ref: Option<String>,
}

fn apply_diagnostics_model_catalog_labels(
    projection: &mut ModelLaneDiagnosticsProjection,
    model_catalog: Option<&crate::model_runtime::ModelCatalog>,
) {
    for lane in &mut projection.lanes {
        let (label, reason) = diagnostics_model_identity_label(
            &lane.kind,
            &lane.runtime_binding,
            &lane.provider_kind,
            lane.model_id.as_deref(),
            lane.model_stable_anchor.as_deref(),
            model_catalog,
        );
        lane.model_display_name = label;
        if reason.is_some() {
            lane.model_anchor_unavailable_reason = reason;
        }
    }
}

pub fn diagnostics_model_identity_label(
    kind: &str,
    runtime_binding: &str,
    provider_kind: &str,
    model_id: Option<&str>,
    stable_anchor: Option<&str>,
    model_catalog: Option<&crate::model_runtime::ModelCatalog>,
) -> (String, Option<String>) {
    let is_local_runtime = kind == ModelLaneKind::LocalModel.as_str()
        && runtime_binding == RuntimeBinding::Local.as_str()
        && provider_kind == ModelLaneProviderKind::LocalRuntime.as_str();
    if !is_local_runtime {
        return (
            model_id
                .map(|id| format!("{provider_kind} / {id}"))
                .unwrap_or_else(|| format!("{provider_kind} lane")),
            None,
        );
    }
    let Some(anchor) = stable_anchor else {
        return (
            crate::model_runtime::UNKNOWN_MODEL_LABEL.to_owned(),
            Some("legacy local lane has no persisted artifact SHA-256 anchor".to_owned()),
        );
    };
    let Some(catalog) = model_catalog else {
        return (
            crate::model_runtime::UNKNOWN_MODEL_LABEL.to_owned(),
            Some(format!(
                "live model catalog unavailable for stable anchor {anchor}"
            )),
        );
    };
    match catalog.entry_for_stable_anchor(anchor) {
        Some(entry) => (entry.display_name, None),
        None => (
            crate::model_runtime::UNKNOWN_MODEL_LABEL.to_owned(),
            Some(format!(
                "stable anchor {anchor} is not loaded in the current model catalog"
            )),
        ),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelLaneDiagnosticsMessage {
    pub message_id: String,
    pub from_lane_id: String,
    pub to_lane: String,
    pub routing_target_role: Option<String>,
    pub routing_target_session: Option<String>,
    pub routing_correlation_id: Option<String>,
    pub routing_requires_ack: bool,
    pub routing_ack_for: Option<String>,
    pub kind: String,
    pub authority: String,
    pub promotion_state: String,
    pub payload_ref: String,
    pub payload_sha256: String,
    pub artifact_ref: Option<String>,
    pub promotion_decision_id: Option<String>,
    pub promotion_gate_ref: Option<String>,
    pub promotion_receipt_ref: Option<String>,
    pub validator_verdict_ref: Option<String>,
    pub operator_decision_ref: Option<String>,
    pub promoted_artifact_sha256: Option<String>,
    pub promoted_artifact_version: Option<String>,
    pub tool_gate_decision_refs: Vec<String>,
    pub coordinator_session_id: String,
    pub work_packet_id: Option<String>,
    pub micro_task_id: Option<String>,
    pub task_board_id: Option<String>,
    pub owner_session: String,
    pub trace_id: String,
    pub message_span_id: String,
    pub parent_span_id: Option<String>,
    pub linked_span_contexts: Vec<String>,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
    pub flight_recorder_correlation_id: String,
    pub locus_ref: Option<String>,
    pub loom_ref: Option<String>,
    pub fems_ref: Option<String>,
    pub proposal_ref: Option<String>,
    pub crdt_update_ref: Option<String>,
    pub crdt_base_snapshot_ref: Option<String>,
    pub crdt_state_vector: Option<String>,
    pub crdt_proposal_ref: Option<String>,
    pub crdt_stale_base_ref: Option<String>,
    pub payload_error: Option<String>,
    pub reason_ref: Option<String>,
    pub recovery_hint_ref: Option<String>,
    pub created_at_utc: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelLaneDiagnosticsTier {
    pub tier: String,
    pub state: String,
    pub reason: String,
    pub evidence_ref: String,
    pub follow_up_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelLaneDiagnosticsMtStatus {
    pub micro_task_id: String,
    pub status: String,
    pub proof_status_ref: Option<String>,
    pub hbr_status_ref: Option<String>,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelLaneNavigationProjection {
    pub schema_id: String,
    pub surface_contract_id: String,
    pub route_id: String,
    pub lookup_kind: String,
    pub lookup_ref: String,
    pub input_schema_ref: String,
    pub output_schema_ref: String,
    pub manual_refs: Vec<String>,
    pub run: Option<ModelLaneRunRecord>,
    pub lanes: Vec<ModelLaneRecord>,
    pub messages: Vec<ModelLaneMessageRecord>,
    pub artifacts: Vec<ModelLaneContextBundleArtifactBindingRecord>,
    pub context_handoffs: Vec<ModelLaneContextBundleHandoffRecord>,
    pub recovery_checkpoints: Vec<ModelLaneRecoveryCheckpointRecord>,
    pub recovery_events: Vec<ModelLaneRecoveryEventRecord>,
    pub leases: Vec<ModelLaneLeaseRecord>,
    pub diagnostic_tiers: Vec<ModelLaneDiagnosticTierStatusRecord>,
    pub mt_runtime_statuses: Vec<ModelLaneMtRuntimeStatusRecord>,
    pub event_ledger_refs: Vec<String>,
    pub flight_recorder_refs: Vec<String>,
    pub error_codes: Vec<String>,
    pub recovery_routes: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelLaneNavigationLookup {
    pub lookup_kind: Option<String>,
    pub lookup_ref: Option<String>,
    pub run_id: Option<String>,
    pub lane_id: Option<String>,
    pub message_id: Option<String>,
    pub model_session_id: Option<String>,
    pub session_id: Option<String>,
    pub wp_id: Option<String>,
    pub work_packet_id: Option<String>,
    pub mt_id: Option<String>,
    pub micro_task_id: Option<String>,
    pub task_board_id: Option<String>,
    pub artifact_ref: Option<String>,
    pub context_bundle_id: Option<String>,
    pub locus_ref: Option<String>,
    pub locus_binding_ref: Option<String>,
    pub loom_ref: Option<String>,
    pub loom_block_id: Option<String>,
    pub fems_ref: Option<String>,
    pub memory_pack_ref: Option<String>,
    pub memory_pack_hash: Option<String>,
    pub event_ledger_event_id: Option<String>,
    pub event_ledger_seq: Option<String>,
    pub trace_id: Option<String>,
    pub span_id: Option<String>,
    pub error_code: Option<String>,
}

impl ModelLaneNavigationLookup {
    fn requested(&self) -> ModelLaneResult<(String, String)> {
        let mut requested = Vec::new();
        if let (Some(kind), Some(value)) = (
            nonempty_lookup_value(self.lookup_kind.as_deref()),
            nonempty_lookup_value(self.lookup_ref.as_deref()),
        ) {
            requested.push((kind, value));
        }
        for (kind, value) in [
            ("run_id", self.run_id.as_deref()),
            ("lane_id", self.lane_id.as_deref()),
            ("message_id", self.message_id.as_deref()),
            ("model_session_id", self.model_session_id.as_deref()),
            ("session_id", self.session_id.as_deref()),
            ("wp_id", self.wp_id.as_deref()),
            ("work_packet_id", self.work_packet_id.as_deref()),
            ("mt_id", self.mt_id.as_deref()),
            ("micro_task_id", self.micro_task_id.as_deref()),
            ("task_board_id", self.task_board_id.as_deref()),
            ("artifact_ref", self.artifact_ref.as_deref()),
            ("context_bundle_id", self.context_bundle_id.as_deref()),
            ("locus_ref", self.locus_ref.as_deref()),
            ("locus_binding_ref", self.locus_binding_ref.as_deref()),
            ("loom_ref", self.loom_ref.as_deref()),
            ("loom_block_id", self.loom_block_id.as_deref()),
            ("fems_ref", self.fems_ref.as_deref()),
            ("memory_pack_ref", self.memory_pack_ref.as_deref()),
            ("memory_pack_hash", self.memory_pack_hash.as_deref()),
            (
                "event_ledger_event_id",
                self.event_ledger_event_id.as_deref(),
            ),
            ("event_ledger_seq", self.event_ledger_seq.as_deref()),
            ("trace_id", self.trace_id.as_deref()),
            ("span_id", self.span_id.as_deref()),
            ("error_code", self.error_code.as_deref()),
        ] {
            if let Some(value) = nonempty_lookup_value(value) {
                requested.push((kind.to_string(), value));
            }
        }
        match requested.len() {
            1 => Ok(requested.remove(0)),
            0 => Err(ModelLaneError::InvalidInput(
                "ModelLane navigation lookup requires exactly one selector".into(),
            )),
            _ => Err(ModelLaneError::InvalidInput(
                "ModelLane navigation lookup accepts exactly one selector".into(),
            )),
        }
    }
}

impl ModelLaneNavigationProjection {
    fn rebuild_navigation_evidence(&mut self) {
        let mut event_ledger_refs = BTreeSet::new();
        let mut flight_recorder_refs = BTreeSet::new();
        let mut error_codes = BTreeSet::new();

        if let Some(run) = &self.run {
            push_event_ref(&mut event_ledger_refs, &run.event_ledger_event_id);
            push_event_seq_ref(&mut event_ledger_refs, run.event_ledger_seq);
            push_optional_string(&mut flight_recorder_refs, run.recovery_hint_ref.as_deref());
            push_optional_string(&mut flight_recorder_refs, Some(&run.memory_pack_ref));
            push_optional_string(&mut error_codes, run.failstate_code.as_deref());
        }
        for lane in &self.lanes {
            push_event_ref(&mut event_ledger_refs, &lane.event_ledger_event_id);
            push_event_seq_ref(&mut event_ledger_refs, lane.event_ledger_seq);
            push_optional_string(
                &mut flight_recorder_refs,
                lane.process_ownership_ref.as_deref(),
            );
            push_optional_string(
                &mut flight_recorder_refs,
                lane.last_runtime_status_ref.as_deref(),
            );
            push_optional_string(
                &mut flight_recorder_refs,
                lane.last_recovery_event_ref.as_deref(),
            );
            push_optional_string(&mut flight_recorder_refs, lane.recovery_hint_ref.as_deref());
            push_optional_string(&mut error_codes, lane.failstate_code.as_deref());
        }
        for message in &self.messages {
            push_event_ref(&mut event_ledger_refs, &message.event_ledger_event_id);
            push_event_seq_ref(&mut event_ledger_refs, message.event_ledger_seq);
            push_optional_string(&mut flight_recorder_refs, Some(&message.payload_ref));
            push_optional_string(
                &mut flight_recorder_refs,
                message.recovery_hint_ref.as_deref(),
            );
            push_optional_string(&mut flight_recorder_refs, message.proposal_ref.as_deref());
            push_optional_string(
                &mut flight_recorder_refs,
                message.crdt_update_ref.as_deref(),
            );
            push_optional_string(&mut error_codes, message.failstate_code.as_deref());
            push_optional_json_string(
                &mut flight_recorder_refs,
                &message.diagnostic_payload,
                "flight_recorder",
            );
            push_optional_json_string(
                &mut flight_recorder_refs,
                &message.diagnostic_payload,
                "internal_diagnostics",
            );
            push_optional_json_string(
                &mut flight_recorder_refs,
                &message.diagnostic_payload,
                "palmistry",
            );
            push_optional_json_string(
                &mut flight_recorder_refs,
                &message.diagnostic_payload,
                "locus_ref",
            );
            push_optional_json_string(
                &mut flight_recorder_refs,
                &message.diagnostic_payload,
                "loom_ref",
            );
            push_optional_json_string(
                &mut flight_recorder_refs,
                &message.diagnostic_payload,
                "fems_ref",
            );
        }
        for artifact in &self.artifacts {
            push_event_ref(&mut event_ledger_refs, &artifact.event_ledger_event_id);
            push_event_seq_ref(&mut event_ledger_refs, artifact.event_ledger_seq);
            push_optional_string(&mut flight_recorder_refs, Some(&artifact.artifact_ref));
            push_optional_string(
                &mut flight_recorder_refs,
                Some(&artifact.artifact_manifest_ref),
            );
            push_optional_string(
                &mut flight_recorder_refs,
                Some(&artifact.artifact_payload_ref),
            );
        }
        for handoff in &self.context_handoffs {
            push_event_ref(&mut event_ledger_refs, &handoff.event_ledger_event_id);
            push_event_seq_ref(&mut event_ledger_refs, handoff.event_ledger_seq);
            push_optional_string(&mut flight_recorder_refs, Some(&handoff.context_bundle_id));
            push_optional_string(&mut flight_recorder_refs, Some(&handoff.artifact_ref));
            push_optional_json_string(
                &mut flight_recorder_refs,
                &handoff.diagnostic_payload,
                "flight_recorder",
            );
            push_optional_json_string(
                &mut flight_recorder_refs,
                &handoff.diagnostic_payload,
                "internal_diagnostics",
            );
            push_optional_json_string(
                &mut flight_recorder_refs,
                &handoff.diagnostic_payload,
                "palmistry",
            );
            push_optional_string(&mut error_codes, Some(&handoff.reason_code));
        }
        for checkpoint in &self.recovery_checkpoints {
            push_event_ref(&mut event_ledger_refs, &checkpoint.event_ledger_event_id);
            push_event_seq_ref(&mut event_ledger_refs, checkpoint.event_ledger_seq);
            push_optional_string(
                &mut flight_recorder_refs,
                checkpoint.recovery_hint_ref.as_deref(),
            );
            push_optional_string(
                &mut flight_recorder_refs,
                checkpoint.recovery_event_ref.as_deref(),
            );
        }
        for event in &self.recovery_events {
            push_event_ref(&mut event_ledger_refs, &event.event_ledger_event_id);
            push_event_seq_ref(&mut event_ledger_refs, event.event_ledger_seq);
            push_optional_string(
                &mut flight_recorder_refs,
                event.recovery_hint_ref.as_deref(),
            );
            push_optional_string(&mut error_codes, event.error_code.as_deref());
            push_optional_string(
                &mut error_codes,
                event
                    .failure_kind
                    .as_ref()
                    .map(ModelLaneRecoveryFailureKind::code),
            );
        }
        for lease in &self.leases {
            push_event_ref(&mut event_ledger_refs, &lease.event_ledger_event_id);
            push_event_seq_ref(&mut event_ledger_refs, lease.event_ledger_seq);
            push_optional_string(
                &mut flight_recorder_refs,
                lease.recovery_hint_ref.as_deref(),
            );
        }
        for tier in &self.diagnostic_tiers {
            push_event_ref(&mut event_ledger_refs, &tier.event_ledger_event_id);
            push_event_seq_ref(&mut event_ledger_refs, tier.event_ledger_seq);
            push_optional_string(&mut flight_recorder_refs, Some(&tier.evidence_ref));
            push_optional_string(&mut flight_recorder_refs, tier.follow_up_ref.as_deref());
        }
        for status in &self.mt_runtime_statuses {
            push_event_ref(&mut event_ledger_refs, &status.event_ledger_event_id);
            push_event_seq_ref(&mut event_ledger_refs, status.event_ledger_seq);
            push_optional_string(
                &mut flight_recorder_refs,
                status.proof_status_ref.as_deref(),
            );
            push_optional_string(&mut flight_recorder_refs, status.hbr_status_ref.as_deref());
            push_optional_string(
                &mut flight_recorder_refs,
                status.last_recovery_event_ref.as_deref(),
            );
            push_optional_string(
                &mut flight_recorder_refs,
                status.last_runtime_status_ref.as_deref(),
            );
        }

        self.event_ledger_refs = event_ledger_refs.into_iter().collect();
        self.flight_recorder_refs = flight_recorder_refs.into_iter().collect();
        self.error_codes = error_codes.into_iter().collect();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneCloudProjectionPlanStatus {
    Active,
    Superseded,
}

impl ModelLaneCloudProjectionPlanStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Superseded => "superseded",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneCloudConsentReceiptStatus {
    Approved,
    Revoked,
}

impl ModelLaneCloudConsentReceiptStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Approved => "approved",
            Self::Revoked => "revoked",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneCloudConsentScope {
    SingleLane,
    SingleRun,
}

impl ModelLaneCloudConsentScope {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SingleLane => "single_lane",
            Self::SingleRun => "single_run",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneCloudRetentionPolicy {
    NoTrainingEphemeral,
    ProviderDefault,
}

impl ModelLaneCloudRetentionPolicy {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NoTrainingEphemeral => "no_training_ephemeral",
            Self::ProviderDefault => "provider_default",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneCloudExportPosture {
    RedactedContextOnly,
    NoExport,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelLaneCloudConsentTargetBinding {
    pub lane_id: String,
    pub model_session_id: String,
    pub provider_kind: String,
    pub requested_model_id: String,
    pub capability_snapshot_ref: String,
    pub provider_endpoint_ref: String,
}

impl ModelLaneCloudExportPosture {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RedactedContextOnly => "redacted_context_only",
            Self::NoExport => "no_export",
        }
    }
}

/// HBR-PRIV-007 remote/SaaS delegation record carried by every ProjectionPlan.
///
/// A cloud projection is a delegation of the operator's local data to a third
/// party. HBR-PRIV-007 requires that delegation to carry (a) an audience-bound
/// scope, (b) the local visibility it was derived from, and (c) the
/// authorization receipt that permits it. Without (b) there is nothing to
/// compare a remote export against, so "the export did not widen local
/// visibility" is unprovable rather than true.
///
/// `audience_refs` is validated as a SUBSET of the plan's `fan_out_targets`, so
/// the audience can never name a destination the plan did not already disclose.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloudExportDelegation {
    /// The exact third-party endpoints this projection may reach. Must be a
    /// subset of the plan's `fan_out_targets`.
    pub audience_refs: Vec<String>,
    /// The LOCAL account-bound visibility this export is derived from. A remote
    /// export may not exceed it, and a reader from another account may not use
    /// it. `Unattributed` means the export was produced without any
    /// authenticated account context and is therefore unusable as authority.
    pub source_scope: AccountBoundAuthority,
    /// The `consent_receipt_id` that authorizes this delegation, when the plan
    /// and receipt are minted as a 1:1 pair. Optional because a plan is durable
    /// evidence in its own right and is recorded BEFORE its receipt; when it is
    /// present it is enforced to match in [`validate_cloud_authority_pair`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authorization_receipt_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewModelLaneCloudProjectionPlan {
    pub projection_plan_id: String,
    pub run_id: String,
    pub trace_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lane_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_model_id: Option<String>,
    pub scope_hash: String,
    pub source_artifact_refs: Vec<String>,
    pub payload_artifact_ref: String,
    pub payload_sha256: String,
    pub redaction_policy_ref: String,
    pub redaction_summary: String,
    pub retention_policy: ModelLaneCloudRetentionPolicy,
    pub export_posture: ModelLaneCloudExportPosture,
    pub provider_profile_ref: String,
    pub fan_out_targets: Vec<String>,
    /// HBR-PRIV-007. See [`CloudExportDelegation`].
    pub export_delegation: CloudExportDelegation,
    pub consent_scope: ModelLaneCloudConsentScope,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub target_bindings: Vec<ModelLaneCloudConsentTargetBinding>,
    pub status: ModelLaneCloudProjectionPlanStatus,
    pub event_ledger_stream_id: String,
    pub work_packet_id: String,
    pub micro_task_id: String,
    pub task_board_id: String,
    pub owner_session: String,
    pub idempotency_key: String,
    pub created_at_utc: String,
    pub user_manual_behavior_ref: String,
    pub diagnostic_payload: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelLaneCloudProjectionPlanRecord {
    #[serde(flatten)]
    pub inner: NewModelLaneCloudProjectionPlan,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_bindings_hash: Option<String>,
    pub projection_plan_hash: String,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
    pub event_stream_version: i64,
    pub transaction_seq: i64,
}

impl Deref for ModelLaneCloudProjectionPlanRecord {
    type Target = NewModelLaneCloudProjectionPlan;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewModelLaneCloudConsentReceipt {
    pub consent_receipt_id: String,
    pub projection_plan_id: String,
    pub projection_plan_hash: String,
    pub run_id: String,
    pub trace_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lane_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_model_id: Option<String>,
    pub scope_hash: String,
    pub consent_scope: ModelLaneCloudConsentScope,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub target_bindings: Vec<ModelLaneCloudConsentTargetBinding>,
    pub retention_policy: ModelLaneCloudRetentionPolicy,
    pub export_posture: ModelLaneCloudExportPosture,
    pub fan_out_targets: Vec<String>,
    pub approved: bool,
    /// HBR-PRIV-005/007: the ONLY authorization surface on this receipt.
    ///
    /// Every gate that decides "may this receipt authorize a cloud export"
    /// consults this typed value and nothing else. It cannot be a formatted
    /// string, because a string is what let the operator-chat path mint
    /// `operator://<governance_role_label>/cloud-selection` and call it an
    /// operator approval.
    pub approver: AccountBoundAuthority,
    /// PROVENANCE ONLY — **not** authorization.
    ///
    /// # What happened to the legacy self-minted value, and why
    ///
    /// The operator-chat path used to write
    /// `format!("operator://{}/cloud-selection", owner_session)` here, where
    /// `owner_session` is a governance ROLE LABEL. Two options were available:
    /// reject every legacy-shaped value at write time, or retain the field for
    /// provenance and refuse to treat it as authorization.
    ///
    /// Both were taken, in the narrowest defensible split:
    ///
    /// * **Retained for provenance.** Real deployed receipts and the existing
    ///   proof corpus carry human-meaningful refs (`operator://mt006/approval`,
    ///   ticket ids, UI action refs). Deleting the column would destroy real
    ///   lineage and would rewrite history to pretend the self-issued receipts
    ///   never existed. It is kept, and it is kept honest by being demoted: no
    ///   gate reads it.
    /// * **Rejected at write time, but only for the self-issuance shape.**
    ///   [`reject_self_minted_approver`] refuses a value whose identity
    ///   component IS this row's own `owner_session` — i.e. exactly
    ///   `operator://{owner_session}/...`. That is the shape that carries no
    ///   information, because the subject and the issuer are the same label. A
    ///   blanket ban on `operator://` would have been theatre: it would reject
    ///   honest refs while a caller could still self-issue under any other
    ///   scheme, and the real fix (the typed `approver`) is what closes that.
    ///
    /// Nothing here is silently trusted: the typed `approver` is required, and
    /// an `Unattributed` approver cannot satisfy any account-scoped gate.
    pub approved_by_ref: String,
    pub approved_at_utc: String,
    pub valid_from_utc: String,
    pub valid_until_utc: String,
    pub revoked_at_utc: Option<String>,
    pub revocation_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revocation_input_hash: Option<String>,
    pub status: ModelLaneCloudConsentReceiptStatus,
    pub event_ledger_stream_id: String,
    pub work_packet_id: String,
    pub micro_task_id: String,
    pub task_board_id: String,
    pub owner_session: String,
    pub idempotency_key: String,
    pub created_at_utc: String,
    pub user_manual_behavior_ref: String,
    pub diagnostic_payload: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelLaneCloudConsentReceiptRecord {
    #[serde(flatten)]
    pub inner: NewModelLaneCloudConsentReceipt,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_bindings_hash: Option<String>,
    pub consent_receipt_hash: String,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
    pub event_stream_version: i64,
    pub transaction_seq: i64,
}

impl Deref for ModelLaneCloudConsentReceiptRecord {
    type Target = NewModelLaneCloudConsentReceipt;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelLaneCloudConsentAuthorityReplay {
    pub projection_plans: Vec<ModelLaneCloudProjectionPlanRecord>,
    pub consent_receipts: Vec<ModelLaneCloudConsentReceiptRecord>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelLaneCrdtHandoffMetadata {
    pub schema_id: String,
    pub document_id: String,
    pub workspace_id: String,
    pub actor_id: String,
    pub actor_kind: String,
    pub lane_id: String,
    pub crdt_site_id: String,
    pub update_seq: i64,
    pub update_bytes_ref: String,
    pub update_sha256: String,
    pub state_vector: String,
    pub base_snapshot_ref: String,
    pub materialized_projection_hash: String,
    pub replay_metadata: Value,
    pub promotion_gate_ref: String,
    pub promotion_receipt_ref: Option<String>,
    pub validation_runner_ref: String,
    pub authority_effect: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelLaneLoomHandoffRef {
    pub workspace_id: String,
    pub block_id: String,
    pub source_block_id: Option<String>,
    pub target_block_id: Option<String>,
    pub artifact_ref: Option<String>,
    pub content_hash: String,
    pub version: String,
    pub event_ledger_evidence_ref: String,
    pub flight_recorder_evidence_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelLaneMemoryPackHandoffRef {
    pub memory_pack_ref: String,
    pub memory_pack_hash: String,
    pub scope_tag: String,
    pub review_status: String,
    pub cloud_safe: bool,
    pub classification: String,
    pub projection_ref: Option<String>,
    pub evidence_ref: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewModelLaneContextBundleArtifactBinding {
    pub artifact_binding_id: String,
    pub run_id: String,
    pub trace_id: String,
    pub artifact_ref: String,
    pub artifact_sha256: String,
    pub content_hash: String,
    pub artifact_kind: String,
    pub artifact_manifest_ref: String,
    pub artifact_payload_ref: String,
    pub payload_json: Value,
    pub event_ledger_stream_id: String,
    pub work_packet_id: String,
    pub micro_task_id: String,
    pub task_board_id: String,
    pub owner_session: String,
    pub idempotency_key: String,
    pub created_at_utc: String,
    pub diagnostic_payload: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelLaneContextBundleArtifactBindingRecord {
    #[serde(flatten)]
    pub inner: NewModelLaneContextBundleArtifactBinding,
    pub artifact_binding_hash: String,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
    pub event_stream_version: i64,
    pub transaction_seq: i64,
}

impl Deref for ModelLaneContextBundleArtifactBindingRecord {
    type Target = NewModelLaneContextBundleArtifactBinding;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewModelLaneContextBundleHandoff {
    pub handoff_id: String,
    pub context_bundle_id: String,
    pub run_id: String,
    pub trace_id: String,
    pub handoff_span_id: String,
    pub parent_span_id: Option<String>,
    pub linked_span_contexts: Vec<String>,
    pub downstream_lane_id: String,
    pub source_lane_id: String,
    pub source_message_id: String,
    pub artifact_ref: String,
    pub artifact_sha256: String,
    pub content_hash: String,
    pub source_kind: ModelLaneHandoffSourceKind,
    pub authority_state: ModelLaneAuthority,
    pub selection_state: ModelLaneHandoffSelectionState,
    pub reason_code: String,
    pub decision_ref: Option<String>,
    pub reviewer_ref: Option<String>,
    pub replay_hint: String,
    pub crdt_payload: Option<ModelLaneCrdtHandoffMetadata>,
    pub loom_refs: Vec<ModelLaneLoomHandoffRef>,
    pub memory_pack_refs: Vec<ModelLaneMemoryPackHandoffRef>,
    pub event_ledger_stream_id: String,
    pub work_packet_id: String,
    pub micro_task_id: String,
    pub task_board_id: String,
    pub owner_session: String,
    pub idempotency_key: String,
    pub replay_order_key: String,
    pub created_at_utc: String,
    pub diagnostic_payload: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelLaneContextBundleHandoffRecord {
    #[serde(flatten)]
    pub inner: NewModelLaneContextBundleHandoff,
    pub context_bundle_hash: String,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
    pub event_stream_version: i64,
    pub transaction_seq: i64,
}

impl Deref for ModelLaneContextBundleHandoffRecord {
    type Target = NewModelLaneContextBundleHandoff;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelLaneDownstreamContextBundle {
    pub run_id: String,
    pub context_bundle_id: String,
    pub downstream_lane_id: String,
    pub context_hash: String,
    pub allowed_context: Value,
    pub records: Vec<ModelLaneContextBundleHandoffRecord>,
}

impl ModelLaneDownstreamContextBundle {
    pub fn to_kernel_context_bundle(&self) -> crate::kernel::KernelResult<ContextBundle> {
        ContextBundle::new(
            self.run_id.clone(),
            self.downstream_lane_id.clone(),
            self.allowed_context.clone(),
        )
    }
}

pub fn model_lane_context_bundle_id_for_handoff(
    input: &NewModelLaneContextBundleHandoff,
) -> ModelLaneResult<String> {
    let hash = dexterity_sha256_hex(serde_json::to_vec(&context_bundle_identity_hash_basis(
        input,
    ))?);
    Ok(format!("CTX-{}", &hash[..16]))
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewModelLanePromotionDecision {
    pub decision_id: String,
    pub run_id: String,
    pub trace_id: String,
    pub decision_span_id: String,
    pub parent_span_id: Option<String>,
    pub linked_span_contexts: Vec<String>,
    pub coordinator_session_id: String,
    pub routing_policy: ModelLaneRoutingPolicy,
    #[serde(default)]
    pub routing_launch_plan: Vec<super::routing::ModelLaneRoutingStageLaunchPlan>,
    pub input_refs: Vec<String>,
    pub selected_input_refs: Vec<String>,
    pub rejected_input_refs: Vec<String>,
    pub validator_authority_ref: Option<String>,
    pub operator_authority_ref: Option<String>,
    pub expected_event_ledger_aggregate_type: String,
    pub expected_event_ledger_aggregate_id: String,
    pub expected_event_ledger_version: i64,
    pub base_snapshot_ref: String,
    pub current_base_snapshot_ref: String,
    pub state_vector: String,
    pub current_state_vector: String,
    pub schema_id: String,
    pub deterministic_tie_break_rule: String,
    pub promotion_gate_ref: String,
    pub promotion_receipt_ref: Option<String>,
    #[serde(default)]
    pub promoted_artifact_ref: Option<String>,
    #[serde(default)]
    pub promoted_artifact_sha256: Option<String>,
    #[serde(default)]
    pub promoted_artifact_version: Option<String>,
    pub direct_authority_mutation_attempt_ref: Option<String>,
    pub event_ledger_stream_id: String,
    pub work_packet_id: Option<String>,
    pub micro_task_id: Option<String>,
    pub task_board_id: Option<String>,
    pub owner_session: String,
    pub idempotency_key: String,
    pub replay_order_key: String,
    pub recovery_hint_ref: Option<String>,
    pub created_at_utc: String,
    pub diagnostic_payload: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelLanePromotionDecisionRecord {
    #[serde(flatten)]
    pub inner: NewModelLanePromotionDecision,
    pub outcome: ModelLanePromotionOutcome,
    pub final_state: ModelLanePromotionState,
    pub denial_reason: Option<ModelLanePromotionDenialReason>,
    pub state_history: Vec<ModelLanePromotionState>,
    pub canonical_input_refs: Vec<String>,
    pub canonical_hash_basis: Value,
    pub canonical_decision_hash: String,
    pub current_event_ledger_version: Option<i64>,
    pub current_schema_id: Option<String>,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
    pub event_stream_version: i64,
    pub transaction_seq: i64,
}

impl Deref for ModelLanePromotionDecisionRecord {
    type Target = NewModelLanePromotionDecision;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelLaneReplay {
    pub run: ModelLaneRunRecord,
    pub lanes: Vec<ModelLaneRecord>,
    pub messages: Vec<ModelLaneMessageRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelLaneSchemaRegistryRow {
    pub schema_id: String,
    pub schema_version: i32,
    pub record_kind: String,
    pub table_name: String,
}

pub fn build_successful_launch_records(
    request: &SpawnRequest,
    live: &LiveSession,
) -> ModelLaneResult<(NewModelLaneRun, NewModelLane)> {
    DexterityLaunchAdapterRegistry::standard().preflight_spawn_request(request)?;
    let contract = request.dexterity_launch.as_ref().ok_or_else(|| {
        ModelLaneError::InvalidInput(
            "Dexterity launch recording requires SpawnRequest::dexterity_launch".into(),
        )
    })?;
    Ok((
        contract.to_run(request, live)?,
        contract.to_lane(request, live)?,
    ))
}

pub fn build_failed_launch_records(
    request: &SpawnRequest,
    err: &SwarmError,
) -> ModelLaneResult<(NewModelLaneRun, NewModelLane)> {
    DexterityLaunchAdapterRegistry::standard().preflight_spawn_request(request)?;
    let contract = request.dexterity_launch.as_ref().ok_or_else(|| {
        ModelLaneError::InvalidInput(
            "Dexterity failed launch recording requires SpawnRequest::dexterity_launch".into(),
        )
    })?;
    let failure_code = err.class().as_str();
    let reason_ref = format!(
        "reason://dexterity/{}/{}/{}",
        contract.run_id, contract.lane_id, failure_code
    );
    let startup_failure_ref = format!(
        "startup-failure://dexterity/{}/{}/{}",
        contract.run_id, contract.lane_id, failure_code
    );
    Ok((
        contract.to_failed_run(request, failure_code, &reason_ref)?,
        contract.to_failed_lane(request, failure_code, &startup_failure_ref, &reason_ref)?,
    ))
}

struct MappedSpawnProvider {
    kind: ModelLaneKind,
    runtime_binding: RuntimeBinding,
    launch_authority: LaunchAuthority,
    provider_kind: ModelLaneProviderKind,
}

fn map_spawn_provider(request: &SpawnRequest) -> ModelLaneResult<MappedSpawnProvider> {
    match request.provider.unwrap_or(ProviderKind::Local) {
        ProviderKind::Local => Ok(MappedSpawnProvider {
            kind: ModelLaneKind::LocalModel,
            runtime_binding: RuntimeBinding::Local,
            launch_authority: LaunchAuthority::ModelRuntime,
            provider_kind: ModelLaneProviderKind::LocalRuntime,
        }),
        ProviderKind::ByokCloud => {
            let provider_kind = match request.byok_cloud_provider {
                Some(ByokCloudProvider::Anthropic) => ModelLaneProviderKind::Anthropic,
                Some(ByokCloudProvider::OpenAi) => ModelLaneProviderKind::OpenAi,
                None => {
                    return Err(ModelLaneError::InvalidInput(
                        "BYOK cloud Dexterity launch requires byok_cloud_provider".into(),
                    ));
                }
            };
            Ok(MappedSpawnProvider {
                kind: ModelLaneKind::CloudModel,
                runtime_binding: RuntimeBinding::Cloud,
                launch_authority: LaunchAuthority::CloudLane,
                provider_kind,
            })
        }
        ProviderKind::OfficialCli => Ok(MappedSpawnProvider {
            kind: ModelLaneKind::CliModel,
            runtime_binding: RuntimeBinding::CliBridge,
            launch_authority: LaunchAuthority::CliBridge,
            provider_kind: ModelLaneProviderKind::OfficialCli,
        }),
        ProviderKind::ExternalCompat => Err(ModelLaneError::InvalidInput(
            "Dexterity model-lane schema does not support external_compat provider".into(),
        )),
    }
}

fn dexterity_adapter_kind_for_spawn(
    request: &SpawnRequest,
) -> ModelLaneResult<DexterityLaunchAdapterKind> {
    match request.provider.unwrap_or(ProviderKind::Local) {
        ProviderKind::Local => Ok(DexterityLaunchAdapterKind::LocalModelRuntime),
        ProviderKind::ByokCloud => match request.byok_cloud_provider {
            Some(ByokCloudProvider::Anthropic) => {
                Ok(DexterityLaunchAdapterKind::ByokCloudAnthropic)
            }
            Some(ByokCloudProvider::OpenAi) => Ok(DexterityLaunchAdapterKind::ByokCloudOpenAi),
            None => Err(ModelLaneError::InvalidInput(
                "BYOK cloud Dexterity launch requires byok_cloud_provider".into(),
            )),
        },
        ProviderKind::OfficialCli => Ok(DexterityLaunchAdapterKind::OfficialCliBridge),
        ProviderKind::ExternalCompat => Err(ModelLaneError::InvalidInput(
            "Dexterity model-lane schema does not support external_compat provider".into(),
        )),
    }
}

fn dexterity_candidate_model_ids(request: &SpawnRequest) -> Vec<String> {
    if let Some(model_name) = request.cloud_model_name.as_deref() {
        return vec![format!(
            "model://dexterity/{}/{}",
            dexterity_provider_kind_label(request.provider.unwrap_or(ProviderKind::Local)),
            model_name
        )];
    }
    vec![request.instance_id.model_id.to_string()]
}

fn dexterity_provider_kind_label(provider: ProviderKind) -> &'static str {
    match provider {
        ProviderKind::Local => "local",
        ProviderKind::ByokCloud => "byok_cloud",
        ProviderKind::OfficialCli => "official_cli",
        ProviderKind::ExternalCompat => "external_compat",
    }
}

fn dexterity_sha256_hex(input: impl AsRef<[u8]>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_ref());
    format!("{:x}", hasher.finalize())
}

pub fn dexterity_spawn_model_session_id(request: &SpawnRequest) -> String {
    format!("swarm-session:{}", request.instance_id)
}

async fn validate_model_lane_authority_tx(
    tx: &mut Transaction<'_, Postgres>,
    lane: &ModelLaneRecord,
) -> ModelLaneResult<()> {
    let row = sqlx::query(
        r#"
        SELECT aggregate_type, aggregate_id, event_sequence, session_run_id, payload
        FROM kernel_event_ledger
        WHERE event_id = $1
        "#,
    )
    .bind(&lane.event_ledger_event_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| {
        ModelLaneError::AuthorityDenied(format!(
            "CX-MM-007 lane {} references missing EventLedger event {}",
            lane.lane_id, lane.event_ledger_event_id
        ))
    })?;
    let aggregate_type: String = row.try_get("aggregate_type")?;
    let aggregate_id: String = row.try_get("aggregate_id")?;
    let event_sequence: i64 = row.try_get("event_sequence")?;
    let session_run_id: String = row.try_get("session_run_id")?;
    let payload: Value = row.try_get("payload")?;
    let ledger_lane = payload.get("record").ok_or_else(|| {
        ModelLaneError::AuthorityDenied(format!(
            "CX-MM-007 lane {} EventLedger event has no canonical record",
            lane.lane_id
        ))
    })?;
    if !matches!(
        aggregate_type.as_str(),
        "model_lane" | "model_lane_terminal"
    ) || aggregate_id != lane.lane_id
        || event_sequence != lane.event_ledger_seq
        || session_run_id != lane.event_ledger_stream_id
        || ledger_lane != &serde_json::to_value(&lane.inner)?
    {
        return Err(ModelLaneError::AuthorityDenied(format!(
            "CX-MM-007 lane {} mutable row differs from EventLedger authority",
            lane.lane_id
        )));
    }
    Ok(())
}

fn runtime_session_id(request: &SpawnRequest) -> String {
    dexterity_spawn_model_session_id(request)
}

fn failed_model_session_id(request: &SpawnRequest) -> String {
    format!("failed-model-session:{}", request.instance_id)
}

fn required_request_field(field: &str, value: Option<&str>) -> ModelLaneResult<String> {
    let value = value.ok_or_else(|| {
        ModelLaneError::InvalidInput(format!("Dexterity launch requires SpawnRequest::{field}"))
    })?;
    require_token(field, value)?;
    Ok(value.to_string())
}

fn model_lane_event(
    event_type: KernelEventType,
    aggregate_type: &str,
    aggregate_id: &str,
    idempotency_key: &str,
    kernel_task_run_id: &str,
    session_run_id: &str,
    payload: Value,
) -> ModelLaneResult<NewKernelEvent> {
    NewKernelEvent::builder(
        kernel_task_run_id,
        session_run_id,
        event_type,
        KernelActor::ModelAdapter("Dexterity".into()),
    )
    .aggregate(aggregate_type, aggregate_id)
    .idempotency_key(idempotency_key)
    .correlation_id(format!("dexterity:{kernel_task_run_id}:{session_run_id}"))
    .source_component(SOURCE_COMPONENT)
    .payload(payload)
    .build()
    .map_err(|err| ModelLaneError::InvalidInput(err.to_string()))
}

#[derive(Debug, Clone)]
struct CloudLaunchAuthorityCheck {
    run_id: String,
    lane_id: String,
    model_session_id: String,
    provider_kind: String,
    requested_model_id: String,
    capability_snapshot_ref: String,
    provider_endpoint_ref: String,
    projection_plan_ref: Option<String>,
    consent_receipt_ref: Option<String>,
    event_ledger_stream_id: String,
    work_packet_id: String,
    micro_task_id: Option<String>,
    owner_session: String,
    user_manual_behavior_ref: String,
}

impl CloudLaunchAuthorityCheck {
    fn from_contract(
        contract: &DexterityLaunchContract,
        provider_kind: &str,
        requested_model_id: &str,
        model_session_id: String,
    ) -> ModelLaneResult<Self> {
        require_token("run_id", &contract.run_id)?;
        require_token("lane_id", &contract.lane_id)?;
        require_token("event_ledger_stream_id", &contract.event_ledger_stream_id)?;
        Ok(Self {
            run_id: contract.run_id.clone(),
            lane_id: contract.lane_id.clone(),
            model_session_id,
            provider_kind: provider_kind.to_string(),
            requested_model_id: requested_model_id.to_string(),
            capability_snapshot_ref: contract.effective_capability_snapshot_ref.clone(),
            provider_endpoint_ref: contract.adapter_id.clone(),
            projection_plan_ref: contract.projection_plan_ref.clone(),
            consent_receipt_ref: contract.consent_receipt_ref.clone(),
            event_ledger_stream_id: contract.event_ledger_stream_id.clone(),
            work_packet_id: contract.run_id.clone(),
            micro_task_id: None,
            owner_session: String::new(),
            user_manual_behavior_ref: "usermanual://model-lane-cloud-projection-consent#launch"
                .into(),
        })
    }
}

fn validate_cloud_authority_pair(
    projection: &ModelLaneCloudProjectionPlanRecord,
    consent: &ModelLaneCloudConsentReceiptRecord,
) -> ModelLaneResult<()> {
    let coherent = consent.projection_plan_id == projection.projection_plan_id
        && consent.projection_plan_hash == projection.projection_plan_hash
        && consent.run_id == projection.run_id
        && consent.trace_id == projection.trace_id
        && consent.lane_id == projection.lane_id
        && consent.model_session_id == projection.model_session_id
        && consent.provider_kind == projection.provider_kind
        && consent.requested_model_id == projection.requested_model_id
        && consent.scope_hash == projection.scope_hash
        && consent.consent_scope == projection.consent_scope
        && consent.target_bindings == projection.target_bindings
        && consent.target_bindings_hash == projection.target_bindings_hash
        && consent.retention_policy == projection.retention_policy
        && consent.export_posture == projection.export_posture
        && consent.fan_out_targets == projection.fan_out_targets
        && consent.event_ledger_stream_id == projection.event_ledger_stream_id
        && consent.work_packet_id == projection.work_packet_id
        && consent.micro_task_id == projection.micro_task_id
        && consent.task_board_id == projection.task_board_id
        && consent.owner_session == projection.owner_session;
    if !coherent {
        return Err(ModelLaneError::AuthorityDenied(format!(
            "CX-MM-007 ConsentReceipt {} is not fully coherent with ProjectionPlan {}",
            consent.consent_receipt_id, projection.projection_plan_id
        )));
    }

    // HBR-PRIV-007: the export's declared local source scope and the account that
    // approved it must be the SAME account. Otherwise account A could approve an
    // export of account B's data, which is precisely the delegation-without-
    // authorization case the pillar names. Checked separately from the coherence
    // chain above so the denial says which invariant broke.
    if !projection
        .export_delegation
        .source_scope
        .same_owner_as(&consent.approver)
    {
        return Err(ModelLaneError::AuthorityDenied(format!(
            "CX-MM-007 ConsentReceipt {} approver account does not own the ProjectionPlan {} export source scope",
            consent.consent_receipt_id, projection.projection_plan_id
        )));
    }

    // When the plan names the receipt that authorizes it, that binding is
    // enforced rather than decorative: a plan may not be paired with a receipt it
    // did not name.
    if let Some(authorized_by) = projection
        .export_delegation
        .authorization_receipt_ref
        .as_deref()
    {
        if authorized_by != consent.consent_receipt_id {
            return Err(ModelLaneError::AuthorityDenied(format!(
                "CX-MM-007 ProjectionPlan {} is authorized by {authorized_by}, not by ConsentReceipt {}",
                projection.projection_plan_id, consent.consent_receipt_id
            )));
        }
    }
    Ok(())
}

fn next_cloud_event_sequence() -> i64 {
    let observed = Utc::now().timestamp_micros().max(1);
    let mut current = CLOUD_EVENT_SEQUENCE.load(AtomicOrdering::Relaxed);
    loop {
        let next = observed.max(current.saturating_add(1));
        match CLOUD_EVENT_SEQUENCE.compare_exchange_weak(
            current,
            next,
            AtomicOrdering::SeqCst,
            AtomicOrdering::Relaxed,
        ) {
            Ok(_) => return next,
            Err(actual) => current = actual,
        }
    }
}

fn cloud_model_lane_scope(exact: &ExactResourceScopeAttribution) -> CloudModelLaneScope {
    CloudModelLaneScope {
        owner_account_id: exact.owner_account_id.to_string(),
        actor_principal_id: exact.actor_principal_id.to_string(),
        authenticated_session_id: exact.authenticated_session_id.to_string(),
        access_space_id: exact.access_space_id.to_string(),
        workspace_id: exact.workspace_id.to_string(),
    }
}

fn validate_cloud_projection_authority_surreal(
    record: &ModelLaneCloudProjectionPlanRecord,
    stored: &CloudModelLaneStoredRow,
) -> ModelLaneResult<()> {
    validate_cloud_projection_plan(&record.inner)?;
    let expected_targets =
        cloud_consent_target_bindings_hash(record.consent_scope, &record.target_bindings)?;
    if record.target_bindings_hash != expected_targets {
        return Err(ModelLaneError::IntegrityViolation(format!(
            "ProjectionPlan {} target_bindings_hash mismatch",
            record.projection_plan_id
        )));
    }
    let expected_hash = cloud_projection_plan_hash(&record.inner)?;
    require_equal(
        "ProjectionPlan.projection_plan_hash",
        &record.projection_plan_hash,
        "canonical ProjectionPlan hash",
        &expected_hash,
    )?;
    validate_surreal_event_envelope(
        &record.event_ledger_event_id,
        record.event_ledger_seq,
        &cloud_projection_plan_event_payload(record),
        stored,
        "ProjectionPlan",
        &record.projection_plan_id,
    )
}

fn validate_cloud_consent_authority_surreal(
    record: &ModelLaneCloudConsentReceiptRecord,
    stored: &CloudModelLaneStoredRow,
) -> ModelLaneResult<()> {
    validate_cloud_consent_receipt(&record.inner)?;
    let expected_targets =
        cloud_consent_target_bindings_hash(record.consent_scope, &record.target_bindings)?;
    if record.target_bindings_hash != expected_targets {
        return Err(ModelLaneError::IntegrityViolation(format!(
            "ConsentReceipt {} target_bindings_hash mismatch",
            record.consent_receipt_id
        )));
    }
    let expected_hash = cloud_consent_receipt_hash(&record.inner)?;
    require_equal(
        "ConsentReceipt.consent_receipt_hash",
        &record.consent_receipt_hash,
        "canonical ConsentReceipt hash",
        &expected_hash,
    )?;
    validate_surreal_event_envelope(
        &record.event_ledger_event_id,
        record.event_ledger_seq,
        &cloud_consent_receipt_event_payload(record),
        stored,
        "ConsentReceipt",
        &record.consent_receipt_id,
    )
}

fn validate_surreal_event_envelope(
    event_id: &str,
    event_seq: i64,
    expected_payload: &Value,
    stored: &CloudModelLaneStoredRow,
    label: &str,
    aggregate_id: &str,
) -> ModelLaneResult<()> {
    if event_id != stored.event_id || event_seq != stored.event_seq || event_seq <= 0 {
        return Err(ModelLaneError::IntegrityViolation(format!(
            "CX-MM-007 {label} {aggregate_id} SurrealDB EventLedger envelope mismatch"
        )));
    }
    let observed_payload: Value = serde_json::from_str(&stored.event_payload_json)?;
    if observed_payload != *expected_payload {
        return Err(ModelLaneError::IntegrityViolation(format!(
            "CX-MM-007 {label} {aggregate_id} mutable/SurrealDB EventLedger authority mismatch"
        )));
    }
    Ok(())
}

fn validate_cloud_launch_pair(
    access: &ResourceAccessContext,
    projection: &ModelLaneCloudProjectionPlanRecord,
    consent: &ModelLaneCloudConsentReceiptRecord,
    check: &CloudLaunchAuthorityCheck,
) -> ModelLaneResult<()> {
    if projection.status != ModelLaneCloudProjectionPlanStatus::Active {
        return Err(ModelLaneError::InvalidInput(
            "ProjectionPlan is not active".into(),
        ));
    }
    require_equal(
        "ProjectionPlan.run_id",
        &projection.run_id,
        "lane.run_id",
        &check.run_id,
    )?;
    ensure_cloud_authority_target("ProjectionPlan", &projection.inner, check)?;
    if consent.revoked_at_utc.is_some()
        || consent.revocation_ref.is_some()
        || consent.status == ModelLaneCloudConsentReceiptStatus::Revoked
    {
        return Err(ModelLaneError::InvalidInput(
            "ConsentReceipt is revoked".into(),
        ));
    }
    if consent.status != ModelLaneCloudConsentReceiptStatus::Approved || !consent.approved {
        return Err(ModelLaneError::InvalidInput(
            "ConsentReceipt is not approved".into(),
        ));
    }
    require_equal(
        "ConsentReceipt.run_id",
        &consent.run_id,
        "lane.run_id",
        &check.run_id,
    )?;
    ensure_cloud_consent_receipt_target("ConsentReceipt", &consent.inner, check)?;
    if let Some(query) = access.read_query() {
        consent.approver.authorizes(query).map_err(|denied| {
            ModelLaneError::AuthorityDenied(format!(
                "CX-MM-007 ConsentReceipt {} carries no approval usable by this account: {}",
                consent.consent_receipt_id,
                denied.reason_code()
            ))
        })?;
    }
    let now = Utc::now();
    let valid_from = parse_utc("ConsentReceipt.valid_from_utc", &consent.valid_from_utc)?;
    let valid_until = parse_utc("ConsentReceipt.valid_until_utc", &consent.valid_until_utc)?;
    if now < valid_from || now > valid_until {
        return Err(ModelLaneError::InvalidInput(
            "ConsentReceipt validity window is not current".into(),
        ));
    }
    Ok(())
}

fn ensure_cloud_authority_target(
    label: &str,
    authority: &NewModelLaneCloudProjectionPlan,
    check: &CloudLaunchAuthorityCheck,
) -> ModelLaneResult<()> {
    ensure_cloud_target_binding(
        label,
        authority.consent_scope,
        authority.lane_id.as_deref(),
        authority.model_session_id.as_deref(),
        authority.provider_kind.as_deref(),
        authority.requested_model_id.as_deref(),
        &authority.target_bindings,
        check,
    )
}

fn ensure_cloud_consent_receipt_target(
    label: &str,
    authority: &NewModelLaneCloudConsentReceipt,
    check: &CloudLaunchAuthorityCheck,
) -> ModelLaneResult<()> {
    ensure_cloud_target_binding(
        label,
        authority.consent_scope,
        authority.lane_id.as_deref(),
        authority.model_session_id.as_deref(),
        authority.provider_kind.as_deref(),
        authority.requested_model_id.as_deref(),
        &authority.target_bindings,
        check,
    )
}

fn ensure_cloud_target_binding(
    label: &str,
    scope: ModelLaneCloudConsentScope,
    lane_id: Option<&str>,
    model_session_id: Option<&str>,
    provider_kind: Option<&str>,
    requested_model_id: Option<&str>,
    target_bindings: &[ModelLaneCloudConsentTargetBinding],
    check: &CloudLaunchAuthorityCheck,
) -> ModelLaneResult<()> {
    if scope == ModelLaneCloudConsentScope::SingleRun {
        return Ok(());
    }
    if scope == ModelLaneCloudConsentScope::SingleLane {
        require_equal(
            &format!("{label}.lane_id"),
            lane_id.unwrap_or_default(),
            "lane.lane_id",
            &check.lane_id,
        )?;
        require_equal(
            &format!("{label}.model_session_id"),
            model_session_id.unwrap_or_default(),
            "lane.model_session_id",
            &check.model_session_id,
        )?;
        require_equal(
            &format!("{label}.provider_kind"),
            provider_kind.unwrap_or_default(),
            "lane.provider_kind",
            &check.provider_kind,
        )?;
        return require_equal(
            &format!("{label}.requested_model_id"),
            requested_model_id.unwrap_or_default(),
            "lane.model_id",
            &check.requested_model_id,
        );
    }

    let _ = target_bindings;
    Ok(())
}

async fn recovery_checkpoint_by_idempotency_key_tx(
    tx: &mut Transaction<'_, Postgres>,
    access: &ResourceAccessContext,
    idempotency_key: &str,
) -> ModelLaneResult<Option<ModelLaneRecoveryCheckpointRecord>> {
    recovery_record_by_idempotency_key_tx(
        tx,
        access,
        "model_lane_recovery_checkpoints",
        idempotency_key,
    )
    .await
}

async fn recovery_event_by_idempotency_key_tx(
    tx: &mut Transaction<'_, Postgres>,
    access: &ResourceAccessContext,
    idempotency_key: &str,
) -> ModelLaneResult<Option<ModelLaneRecoveryEventRecord>> {
    recovery_record_by_idempotency_key_tx(tx, access, "model_lane_recovery_events", idempotency_key)
        .await
}

async fn lane_lease_by_idempotency_key_tx(
    tx: &mut Transaction<'_, Postgres>,
    access: &ResourceAccessContext,
    idempotency_key: &str,
) -> ModelLaneResult<Option<ModelLaneLeaseRecord>> {
    recovery_record_by_idempotency_key_tx(tx, access, "model_lane_leases", idempotency_key).await
}

async fn diagnostic_tier_by_idempotency_key_tx(
    tx: &mut Transaction<'_, Postgres>,
    access: &ResourceAccessContext,
    idempotency_key: &str,
) -> ModelLaneResult<Option<ModelLaneDiagnosticTierStatusRecord>> {
    recovery_record_by_idempotency_key_tx(
        tx,
        access,
        "model_lane_diagnostic_tier_statuses",
        idempotency_key,
    )
    .await
}

async fn mt_runtime_status_by_idempotency_key_tx(
    tx: &mut Transaction<'_, Postgres>,
    access: &ResourceAccessContext,
    idempotency_key: &str,
) -> ModelLaneResult<Option<ModelLaneMtRuntimeStatusRecord>> {
    recovery_record_by_idempotency_key_tx(
        tx,
        access,
        "model_lane_mt_runtime_statuses",
        idempotency_key,
    )
    .await
}

/// Recovery records are children of one canonical ModelLaneRun. Account writes
/// therefore require the same complete five-dimensional scope on parent and
/// child; only an explicit system store may use the legacy NULL-scope path.
fn require_exact_recovery_account_scope(access: &ResourceAccessContext) -> ModelLaneResult<()> {
    if !access.is_system() && access.exact_read_scope().is_none() {
        return Err(ModelLaneError::AuthorityDenied(
            "recovery writes require exact owner, Principal, authenticated session, AccessSpace, and workspace authority"
                .into(),
        ));
    }
    Ok(())
}

/// Cloud runtime rows are externally delegated execution authority, so even an
/// explicitly named system store may not create them without immutable
/// account, Principal, authenticated-session, AccessSpace, and workspace
/// attribution. Legacy unscoped stores remain available for migration reads
/// and non-cloud compatibility paths only.
fn require_exact_cloud_launch_scope(
    access: &ResourceAccessContext,
) -> ModelLaneResult<ExactResourceScopeAttribution> {
    let write_scope = access.write_scope().ok_or_else(|| {
        ModelLaneError::AuthorityDenied(
            "cloud launch requires exact writable owner, Principal, authenticated session, AccessSpace, and workspace authority"
                .into(),
        )
    })?;
    ExactResourceScopeAttribution::try_from_resource_scope(write_scope).map_err(|error| {
        ModelLaneError::AuthorityDenied(format!(
            "cloud launch requires exact writable owner, Principal, authenticated session, AccessSpace, and workspace authority: {error}"
        ))
    })
}

async fn recovery_record_by_idempotency_key_tx<T>(
    tx: &mut Transaction<'_, Postgres>,
    access: &ResourceAccessContext,
    table: &'static str,
    idempotency_key: &str,
) -> ModelLaneResult<Option<T>>
where
    T: DeserializeOwned,
{
    require_exact_recovery_account_scope(access)?;
    let predicate = access.sql_predicate(2);
    let sql = format!(
        "SELECT record_json, {RESOURCE_SCOPE_SELECT_COLUMNS} FROM {table} \
         WHERE idempotency_key = $1{} LIMIT 1",
        predicate.clause()
    );
    if let Some(row) = predicate
        .bind(sqlx::query(&sql).bind(idempotency_key))
        .fetch_optional(&mut **tx)
        .await?
    {
        return authorize_and_decode_row(access, row).map(Some);
    }

    // A globally unique idempotency key owned by another scope is reported as
    // absent. Otherwise the later INSERT would expose its existence through a
    // uniqueness error even though the scoped replay correctly returned none.
    if !access.is_system()
        && sqlx::query_scalar::<_, bool>(&format!(
            "SELECT EXISTS (SELECT 1 FROM {table} WHERE idempotency_key = $1)"
        ))
        .bind(idempotency_key)
        .fetch_one(&mut **tx)
        .await?
    {
        return Err(ModelLaneError::NotFound(
            "model lane resource is not available".into(),
        ));
    }
    Ok(None)
}

async fn recovery_run_by_id_tx(
    tx: &mut Transaction<'_, Postgres>,
    access: &ResourceAccessContext,
    run_id: &str,
) -> ModelLaneResult<ModelLaneRunRecord> {
    require_exact_recovery_account_scope(access)?;
    let predicate = access.sql_predicate(2);
    let sql = format!(
        "SELECT record_json, {RESOURCE_SCOPE_SELECT_COLUMNS} FROM model_lane_runs \
         WHERE run_id = $1{} LIMIT 1 FOR UPDATE",
        predicate.clause()
    );
    predicate
        .bind(sqlx::query(&sql).bind(run_id))
        .fetch_optional(&mut **tx)
        .await?
        .map(|row| authorize_and_decode_row(access, row))
        .transpose()?
        .ok_or_else(|| ModelLaneError::NotFound("model lane resource is not available".into()))
}

async fn recovery_lane_by_id_for_run_tx(
    tx: &mut Transaction<'_, Postgres>,
    access: &ResourceAccessContext,
    run_id: &str,
    lane_id: &str,
) -> ModelLaneResult<ModelLaneRecord> {
    require_exact_recovery_account_scope(access)?;
    let predicate = access.sql_predicate(3);
    let sql = format!(
        "SELECT record_json, {RESOURCE_SCOPE_SELECT_COLUMNS} FROM model_lanes \
         WHERE run_id = $1 AND lane_id = $2{} LIMIT 1 FOR UPDATE",
        predicate.clause()
    );
    predicate
        .bind(sqlx::query(&sql).bind(run_id).bind(lane_id))
        .fetch_optional(&mut **tx)
        .await?
        .map(|row| authorize_and_decode_row(access, row))
        .transpose()?
        .ok_or_else(|| ModelLaneError::NotFound("model lane resource is not available".into()))
}

async fn canonical_run_for_recovery(
    pool: &PgPool,
    access: &ResourceAccessContext,
    run_id: &str,
) -> ModelLaneResult<ModelLaneRunRecord> {
    let run = select_record_by_column::<ModelLaneRunRecord>(
        pool,
        access,
        "model_lane_runs",
        "run_id",
        run_id,
    )
    .await?
    .ok_or_else(|| ModelLaneError::NotFound(format!("run_id {run_id}")))?;
    let ledger_session_run_id: String = sqlx::query_scalar(
        r#"
        SELECT session_run_id
        FROM kernel_event_ledger
        WHERE event_id = $1
          AND aggregate_type = 'model_lane_run'
          AND aggregate_id = $2
        "#,
    )
    .bind(&run.event_ledger_event_id)
    .bind(run_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| {
        ModelLaneError::InvalidInput(format!(
            "model_lane_run {run_id} has no canonical EventLedger session_run_id"
        ))
    })?;
    require_equal(
        "model_lane_run.session_run_id",
        &ledger_session_run_id,
        "record.event_ledger_stream_id",
        &run.event_ledger_stream_id,
    )?;
    Ok(run)
}

/// Boot recovery is intentionally system-authority and cross-owner, but every
/// child it replays must remain pinned to the canonical run's stored scope.
/// Comparing in PostgreSQL keeps malformed/missing attribution fail-closed and
/// supports the explicit legacy system path where all five values are NULL.
async fn validate_recovery_eventledger_resource_scope(
    tx: &mut Transaction<'_, Postgres>,
    run_id: &str,
    event_ledger_stream_id: &str,
) -> ModelLaneResult<()> {
    let mismatches: i64 = sqlx::query_scalar(
        r#"
        WITH canonical_scope AS (
            SELECT jsonb_build_object(
                'owner_account_id', owner_account_id,
                'actor_principal_id', actor_principal_id,
                'authenticated_session_id', authenticated_session_id,
                'access_space_id', access_space_id,
                'workspace_id', workspace_id
            ) AS value
            FROM model_lane_runs
            WHERE run_id = $1
        )
        SELECT COUNT(*)
        FROM kernel_event_ledger AS event
        CROSS JOIN canonical_scope
        WHERE event.session_run_id = $2
          AND event.aggregate_type IN (
              'model_lane_recovery_checkpoint',
              'model_lane_recovery_event',
              'model_lane_lease',
              'model_lane_diagnostic_tier',
              'model_lane_mt_runtime_status'
          )
          AND event.payload->'record'->>'run_id' = $1
          AND event.payload->'resource_scope' IS DISTINCT FROM canonical_scope.value
        "#,
    )
    .bind(run_id)
    .bind(event_ledger_stream_id)
    .fetch_one(&mut **tx)
    .await?;
    if mismatches != 0 {
        return Err(ModelLaneError::IntegrityViolation(
            "recovery EventLedger resource scope does not match the canonical run".into(),
        ));
    }
    Ok(())
}

/// System boot recovery may discover runs across owners, but every child it
/// appends remains a derivative of the canonical run. Derive that write
/// authority from the run's physical scope columns under the recovery fence;
/// never stamp the system scanner's NULL scope onto an account-owned run.
async fn recovery_child_access_for_canonical_run_tx(
    tx: &mut Transaction<'_, Postgres>,
    recovery_access: &ResourceAccessContext,
    run_id: &str,
) -> ModelLaneResult<ResourceAccessContext> {
    let row = sqlx::query(&format!(
        "SELECT {RESOURCE_SCOPE_SELECT_COLUMNS} FROM model_lane_runs \
         WHERE run_id = $1 LIMIT 1 FOR UPDATE"
    ))
    .bind(run_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| ModelLaneError::NotFound("model lane resource is not available".into()))?;
    let stored = stored_resource_scope_from_row(&row)?;
    recovery_access
        .authorize_row(&stored)
        .map_err(|_| ModelLaneError::NotFound("model lane resource is not available".into()))?;
    match (
        stored.owner_account_id,
        stored.actor_principal_id,
        stored.authenticated_session,
        stored.access_space,
        stored.workspace,
    ) {
        (Some(owner), Some(actor), Some(session), Some(access_space), Some(workspace)) => {
            Ok(ResourceAccessContext::for_account(
                ResourceScope::new(owner, actor)
                    .with_session(session)
                    .with_access_space(access_space)
                    .with_workspace(workspace),
            ))
        }
        (None, None, None, None, None) => recovery_access
            .system_authority()
            .map(ResourceAccessContext::system)
            .ok_or_else(|| {
                ModelLaneError::AuthorityDenied(
                    "legacy unattributed recovery requires explicit system authority".into(),
                )
            }),
        _ => Err(ModelLaneError::IntegrityViolation(
            "canonical recovery run has incomplete resource scope".into(),
        )),
    }
}

async fn latest_recovery_checkpoint(
    pool: &PgPool,
    run_id: &str,
    canonical_event_ledger_stream_id: &str,
) -> ModelLaneResult<ModelLaneRecoveryCheckpointRecord> {
    sqlx::query(
        r#"
        SELECT aggregate_id, session_run_id, payload
        FROM kernel_event_ledger
        WHERE aggregate_type = 'model_lane_recovery_checkpoint'
          AND payload->'record'->>'run_id' = $1
          AND session_run_id = $2
          AND payload->'record'->>'event_ledger_stream_id' = $2
        ORDER BY event_sequence DESC
        LIMIT 1
        "#,
    )
    .bind(run_id)
    .bind(canonical_event_ledger_stream_id)
    .fetch_optional(pool)
    .await?
    .map(|row| {
        let aggregate_id: String = row.try_get("aggregate_id")?;
        let session_run_id: String = row.try_get("session_run_id")?;
        let payload: Value = row.try_get("payload")?;
        let record: ModelLaneRecoveryCheckpointRecord =
            event_payload_record(&payload, "model_lane_recovery_checkpoint", &aggregate_id)?;
        require_equal(
            "checkpoint.session_run_id",
            &session_run_id,
            "checkpoint.record.event_ledger_stream_id",
            &record.event_ledger_stream_id,
        )?;
        // MT-003 unblock (out-of-scope, pre-existing WIP commit 0adac5d8): the
        // closure's error type is ambiguous (ModelLaneError has From<sqlx::Error>
        // + From<StorageError> + From<serde_json::Error>), so pin it to the
        // function's own ModelLaneResult error type. Compiler-suggested fix.
        Ok::<_, ModelLaneError>(record)
    })
    .transpose()?
    .ok_or_else(|| {
        let failure = ModelLaneRecoveryFailureKind::MissingCheckpoint;
        ModelLaneError::InvalidInput(format!(
            "{} {} no recovery checkpoint exists for run_id {run_id}",
            failure.code(),
            failure.as_str()
        ))
    })
}

/// Current committed high-watermark (max global EventLedger `event_sequence`) for a
/// ModelLaneRun stream. Used as the forward catch-up bound when the run advanced past
/// its last checkpoint (spec 4.3.9.2.5: "apply EventLedger records after that sequence
/// in order").
async fn recovery_stream_high_watermark(
    pool: &PgPool,
    event_ledger_stream_id: &str,
) -> ModelLaneResult<i64> {
    let high_watermark: i64 = sqlx::query_scalar(
        r#"
        SELECT COALESCE(MAX(event_sequence), 0)
        FROM kernel_event_ledger
        WHERE session_run_id = $1
        "#,
    )
    .bind(event_ledger_stream_id)
    .fetch_one(pool)
    .await?;
    Ok(high_watermark)
}

/// True when the coordinator-owned ModelLaneMessage stream genuinely advanced past the
/// checkpoint (a NEW `model_lane_message` was committed after
/// `checkpoint_bound_event_ledger_seq`). Only real forward-message progress triggers
/// catch-up. Current-state adjunct writes recorded after a checkpoint with no new
/// message (post-checkpoint leases, MT status, cloud denials) are NOT forward progress.
/// Leases are reconciled separately from current ownership authority; they never widen
/// this replay bound. This distinguishes legitimate message catch-up from adjunct state.
async fn has_post_checkpoint_forward_messages(
    pool: &PgPool,
    run_id: &str,
    event_ledger_stream_id: &str,
    checkpoint_bound_event_ledger_seq: i64,
) -> ModelLaneResult<bool> {
    let exists: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM kernel_event_ledger
            WHERE aggregate_type = 'model_lane_message'
              AND session_run_id = $2
              AND payload->'record'->>'run_id' = $1
              AND event_sequence > $3
        )
        "#,
    )
    .bind(run_id)
    .bind(event_ledger_stream_id)
    .bind(checkpoint_bound_event_ledger_seq)
    .fetch_one(pool)
    .await?;
    Ok(exists)
}

async fn recovery_events_for_run(
    pool: &PgPool,
    run_id: &str,
    event_ledger_stream_id: &str,
    recovery_bound_event_ledger_seq: i64,
) -> ModelLaneResult<Vec<ModelLaneRecoveryEventRecord>> {
    sqlx::query(
        r#"
        SELECT aggregate_id, payload
        FROM kernel_event_ledger
        WHERE aggregate_type = 'model_lane_recovery_event'
          AND session_run_id = $2
          AND payload->'record'->>'run_id' = $1
          AND event_sequence <= $3
        ORDER BY (payload->'record'->>'replay_order_seq')::bigint ASC
        "#,
    )
    .bind(run_id)
    .bind(event_ledger_stream_id)
    .bind(recovery_bound_event_ledger_seq)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| {
        let aggregate_id: String = row.try_get("aggregate_id")?;
        let payload: Value = row.try_get("payload")?;
        event_payload_record(&payload, "model_lane_recovery_event", &aggregate_id)
    })
    .collect()
}

async fn lock_recovery_run_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    run_id: &str,
) -> ModelLaneResult<()> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 5569896166133588818))")
        .bind(run_id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

async fn next_recovery_replay_order_seq_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    run_id: &str,
) -> ModelLaneResult<i64> {
    let next: i64 = sqlx::query_scalar(
        r#"
        SELECT COALESCE(MAX(replay_order_seq), 0) + 1
        FROM model_lane_recovery_events
        WHERE run_id = $1
        "#,
    )
    .bind(run_id)
    .fetch_one(&mut **tx)
    .await?;
    Ok(next)
}

async fn current_recovery_events_for_run_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    run_id: &str,
    event_ledger_stream_id: &str,
) -> ModelLaneResult<Vec<ModelLaneRecoveryEventRecord>> {
    sqlx::query(
        r#"
        SELECT aggregate_id, payload
        FROM kernel_event_ledger
        WHERE aggregate_type = 'model_lane_recovery_event'
          AND session_run_id = $2
          AND payload->'record'->>'run_id' = $1
        ORDER BY (payload->'record'->>'replay_order_seq')::bigint ASC
        "#,
    )
    .bind(run_id)
    .bind(event_ledger_stream_id)
    .fetch_all(&mut **tx)
    .await?
    .into_iter()
    .map(|row| {
        let aggregate_id: String = row.try_get("aggregate_id")?;
        let payload: Value = row.try_get("payload")?;
        event_payload_record(&payload, "model_lane_recovery_event", &aggregate_id)
    })
    .collect()
}

/// Resolve the latest committed lane authority independently of checkpoint replay.
/// This is used only to attribute current lease reconciliation and never changes the
/// replay watermark or injects a post-checkpoint lane into `ModelLaneReplay`.
async fn current_lane_for_recovery_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    run_id: &str,
    event_ledger_stream_id: &str,
    lane_id: &str,
) -> ModelLaneResult<ModelLaneRecord> {
    let row = sqlx::query(
        r#"
        SELECT event_id, event_sequence, aggregate_id, payload
        FROM kernel_event_ledger
        WHERE aggregate_type = 'model_lane'
          AND aggregate_id = $3
          AND session_run_id = $2
          AND payload->'record'->>'run_id' = $1
        ORDER BY event_sequence DESC
        LIMIT 1
        "#,
    )
    .bind(run_id)
    .bind(event_ledger_stream_id)
    .bind(lane_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| {
        ModelLaneError::InvalidInput(format!(
            "current lease authority references lane {lane_id} absent from kernel_event_ledger for run {run_id}"
        ))
    })?;
    let event_id: String = row.try_get("event_id")?;
    let event_ledger_seq: i64 = row.try_get("event_sequence")?;
    let aggregate_id: String = row.try_get("aggregate_id")?;
    let payload: Value = row.try_get("payload")?;
    let inner: NewModelLane = event_payload_record(&payload, "model_lane", &aggregate_id)?;
    Ok(ModelLaneRecord {
        inner,
        event_ledger_event_id: event_id,
        event_ledger_seq,
    })
}

/// Resolve the latest canonical lane covered by a consent receipt even when
/// the mutable `model_lanes` projection was lost. Revocation uses this to
/// cancel from EventLedger authority and then rebuild the terminal projection.
async fn current_lane_for_cloud_consent_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    run_id: &str,
    lane_id: &str,
    consent_receipt_id: &str,
    exact_scope: &ExactResourceScopeAttribution,
) -> ModelLaneResult<Option<ModelLaneRecord>> {
    let row = sqlx::query(
        r#"
        SELECT event_id, event_sequence, payload
        FROM kernel_event_ledger
        WHERE aggregate_id = $1
          AND aggregate_type IN ('model_lane', 'model_lane_terminal')
          AND payload->'record'->>'run_id' = $2
          AND payload->'record'->>'consent_receipt_ref' = $3
          AND payload->>'owner_account_id' = $4
          AND payload->>'actor_principal_id' = $5
          AND payload->>'authenticated_session_id' = $6
          AND payload->>'access_space_id' = $7
          AND payload->>'workspace_id' = $8
        ORDER BY event_sequence DESC
        LIMIT 1
        "#,
    )
    .bind(lane_id)
    .bind(run_id)
    .bind(consent_receipt_id)
    .bind(exact_scope.owner_account_id.to_string())
    .bind(exact_scope.actor_principal_id.to_string())
    .bind(exact_scope.authenticated_session_id.to_string())
    .bind(exact_scope.access_space_id.to_string())
    .bind(exact_scope.workspace_id.as_str())
    .fetch_optional(&mut **tx)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let event_id: String = row.try_get("event_id")?;
    let event_ledger_seq: i64 = row.try_get("event_sequence")?;
    let payload: Value = row.try_get("payload")?;
    let inner: NewModelLane = event_payload_record(&payload, "model_lane", lane_id)?;
    Ok(Some(ModelLaneRecord {
        inner,
        event_ledger_event_id: event_id,
        event_ledger_seq,
    }))
}

/// Read the latest committed EventLedger authority for every lease in the run.
///
/// This query is intentionally NOT checkpoint-bounded. A lease acquired after the
/// latest checkpoint represents current process ownership that restart recovery must
/// surface or reclaim. Keeping this separate from `recovery_events_for_run` and
/// `replay_run_at_recovery_bound` preserves deterministic checkpoint replay while
/// preventing live or expired post-checkpoint work from becoming invisible.
async fn current_lane_leases_for_run(
    pool: &PgPool,
    run_id: &str,
    event_ledger_stream_id: &str,
) -> ModelLaneResult<Vec<ModelLaneLeaseRecord>> {
    sqlx::query(
        r#"
        SELECT aggregate_id, payload
        FROM (
            SELECT DISTINCT ON (payload->'record'->>'lease_id')
                   aggregate_id, payload, event_sequence
            FROM kernel_event_ledger
            WHERE aggregate_type = 'model_lane_lease'
              AND session_run_id = $2
              AND payload->'record'->>'run_id' = $1
            ORDER BY payload->'record'->>'lease_id', event_sequence DESC
        ) AS current_lease_authority
        ORDER BY event_sequence ASC
        "#,
    )
    .bind(run_id)
    .bind(event_ledger_stream_id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| {
        let aggregate_id: String = row.try_get("aggregate_id")?;
        let payload: Value = row.try_get("payload")?;
        event_payload_record(&payload, "model_lane_lease", &aggregate_id)
    })
    .collect()
}

async fn mt_runtime_statuses_for_run(
    pool: &PgPool,
    run_id: &str,
    event_ledger_stream_id: &str,
    recovery_bound_event_ledger_seq: i64,
) -> ModelLaneResult<Vec<ModelLaneMtRuntimeStatusRecord>> {
    sqlx::query(
        r#"
        SELECT aggregate_id, payload
        FROM kernel_event_ledger
        WHERE aggregate_type = 'model_lane_mt_runtime_status'
          AND session_run_id = $2
          AND payload->'record'->>'run_id' = $1
          AND event_sequence <= $3
        ORDER BY event_sequence ASC
        "#,
    )
    .bind(run_id)
    .bind(event_ledger_stream_id)
    .bind(recovery_bound_event_ledger_seq)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| {
        let aggregate_id: String = row.try_get("aggregate_id")?;
        let payload: Value = row.try_get("payload")?;
        event_payload_record(&payload, "model_lane_mt_runtime_status", &aggregate_id)
    })
    .collect()
}

async fn cloud_consent_denials_for_run(
    pool: &PgPool,
    run_id: &str,
    event_ledger_stream_id: &str,
    recovery_bound_event_ledger_seq: i64,
) -> ModelLaneResult<Vec<ModelLaneCloudConsentDenialRecord>> {
    sqlx::query(
        r#"
        SELECT event_id, event_sequence, aggregate_id, payload
        FROM kernel_event_ledger
        WHERE aggregate_type = 'model_lane_cloud_consent_denial'
          AND session_run_id = $2
          AND payload->>'run_id' = $1
          AND event_sequence <= $3
        ORDER BY event_sequence ASC
        "#,
    )
    .bind(run_id)
    .bind(event_ledger_stream_id)
    .bind(recovery_bound_event_ledger_seq)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| {
        let payload: Value = row.try_get("payload")?;
        require_json_string(
            &payload,
            "schema_id",
            "hsk.model_lane_cloud_consent_denial@1",
        )?;
        require_json_string(&payload, "reason_code", "CX-MM-007")?;
        if payload.get("provider_call_attempted").and_then(Value::as_bool) != Some(false) {
            let failure = ModelLaneRecoveryFailureKind::MissingEventLedgerRow;
            return Err(ModelLaneError::InvalidInput(format!(
                "{} {} cloud consent denial for run_id {run_id} must prove provider_call_attempted=false",
                failure.code(),
                failure.as_str()
            )));
        }
        let lane_id = required_json_text(&payload, "lane_id")?;
        let aggregate_id: String = row.try_get("aggregate_id")?;
        // Fail closed when a cloud-consent-denial ledger row's aggregate_id was
        // tampered off its own lane_id. Phrased as an "aggregate_id mismatch" for
        // parity with event_payload_record's aggregate-id integrity diagnosis.
        if aggregate_id != lane_id {
            let failure = ModelLaneRecoveryFailureKind::MissingEventLedgerRow;
            return Err(ModelLaneError::InvalidInput(format!(
                "{} {} model_lane_cloud_consent_denial aggregate_id mismatch: ledger aggregate_id {aggregate_id}, payload lane_id {lane_id}",
                failure.code(),
                failure.as_str()
            )));
        }
        let failure_kind = required_json_text(&payload, "failure_kind")?;
        Ok(ModelLaneCloudConsentDenialRecord {
            event_id: row.try_get("event_id")?,
            event_ledger_seq: row.try_get("event_sequence")?,
            run_id: run_id.to_string(),
            lane_id,
            reason_code: "CX-MM-007".into(),
            failure_kind,
        })
    })
    .collect()
}

async fn replay_run_at_recovery_bound(
    pool: &PgPool,
    run_id: &str,
    checkpoint: &ModelLaneRecoveryCheckpointRecord,
    recovery_bound_event_ledger_seq: i64,
) -> ModelLaneResult<ModelLaneReplay> {
    let run_row = sqlx::query(
        r#"
        SELECT event_id, event_sequence, payload
        FROM kernel_event_ledger
        WHERE aggregate_type = 'model_lane_run'
          AND aggregate_id = $1
          AND session_run_id = $2
          AND event_sequence <= $3
        ORDER BY event_sequence DESC
        LIMIT 1
        "#,
    )
    .bind(run_id)
    .bind(&checkpoint.event_ledger_stream_id)
    .bind(recovery_bound_event_ledger_seq)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| ModelLaneError::NotFound(format!("run_id {run_id} before checkpoint")))?;
    let run_event_id: String = run_row.try_get("event_id")?;
    let run_event_seq: i64 = run_row.try_get("event_sequence")?;
    let run_payload: Value = run_row.try_get("payload")?;
    let run_record = ModelLaneRunRecord {
        inner: event_payload_record(&run_payload, "model_lane_run", run_id)?,
        event_ledger_event_id: run_event_id,
        event_ledger_seq: run_event_seq,
    };

    let lanes = sqlx::query(
        r#"
        SELECT event_id, event_sequence, aggregate_id, payload
        FROM kernel_event_ledger
        WHERE aggregate_type = 'model_lane'
          AND session_run_id = $1
          AND payload->'record'->>'run_id' = $2
          AND event_sequence <= $3
        ORDER BY event_sequence ASC
        "#,
    )
    .bind(&checkpoint.event_ledger_stream_id)
    .bind(run_id)
    .bind(recovery_bound_event_ledger_seq)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| {
        let event_id: String = row.try_get("event_id")?;
        let event_seq: i64 = row.try_get("event_sequence")?;
        let aggregate_id: String = row.try_get("aggregate_id")?;
        let payload: Value = row.try_get("payload")?;
        let lane: NewModelLane = event_payload_record(&payload, "model_lane", &aggregate_id)?;
        Ok(ModelLaneRecord {
            inner: lane,
            event_ledger_event_id: event_id,
            event_ledger_seq: event_seq,
        })
    })
    .collect::<ModelLaneResult<Vec<ModelLaneRecord>>>()?;

    let messages = sqlx::query(
        r#"
        SELECT event_id, event_sequence, aggregate_id, payload
        FROM kernel_event_ledger
        WHERE aggregate_type = 'model_lane_message'
          AND session_run_id = $1
          AND payload->'record'->>'run_id' = $2
          AND event_sequence <= $3
        ORDER BY event_sequence ASC
        "#,
    )
    .bind(&checkpoint.event_ledger_stream_id)
    .bind(run_id)
    .bind(recovery_bound_event_ledger_seq)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| {
        let event_id: String = row.try_get("event_id")?;
        let event_seq: i64 = row.try_get("event_sequence")?;
        let aggregate_id: String = row.try_get("aggregate_id")?;
        let payload: Value = row.try_get("payload")?;
        let message: NewModelLaneMessage =
            event_payload_record(&payload, "model_lane_message", &aggregate_id)?;
        let crdt_authority_binding = payload
            .get("crdt_authority_binding")
            .filter(|value| !value.is_null())
            .cloned()
            .map(serde_json::from_value)
            .transpose()?;
        Ok(ModelLaneMessageRecord {
            inner: message,
            event_ledger_event_id: event_id,
            event_ledger_seq: event_seq,
            event_stream_version: event_seq,
            transaction_seq: event_seq,
            crdt_authority_binding,
        })
    })
    .collect::<ModelLaneResult<Vec<ModelLaneMessageRecord>>>()?;

    if let Some(lane_id) = checkpoint.lane_id.as_deref() {
        let lane = lanes
            .iter()
            .find(|lane| lane.lane_id == lane_id)
            .ok_or_else(|| {
                ModelLaneError::InvalidInput(format!(
                    "CX-MM-009 checkpoint {} references missing lane {lane_id}",
                    checkpoint.checkpoint_id
                ))
            })?;
        require_equal(
            "checkpoint.session_id",
            &checkpoint.session_id,
            "lane.session_id",
            &lane.session_id,
        )?;
        require_equal(
            "checkpoint.model_session_id",
            &checkpoint.model_session_id,
            "lane.model_session_id",
            &lane.model_session_id,
        )?;
        require_equal(
            "checkpoint.lane_status",
            checkpoint.lane_status.as_str(),
            "lane.status",
            lane.status.as_str(),
        )?;
    }
    if let Some(last_message_id) = checkpoint.last_message_id.as_deref() {
        if !messages
            .iter()
            .any(|message| message.message_id == last_message_id)
        {
            let failure = ModelLaneRecoveryFailureKind::MissingPayloadAuthority;
            return Err(ModelLaneError::InvalidInput(format!(
                "{} {} checkpoint {} last_message_id {last_message_id} is not replayable before checkpoint",
                failure.code(),
                failure.as_str(),
                checkpoint.checkpoint_id
            )));
        }
    }

    Ok(ModelLaneReplay {
        run: run_record,
        lanes,
        messages,
    })
}

fn event_payload_record<T>(
    payload: &Value,
    aggregate_type: &str,
    aggregate_id: &str,
) -> ModelLaneResult<T>
where
    T: for<'de> Deserialize<'de>,
{
    let record = payload.get("record").ok_or_else(|| {
        ModelLaneError::InvalidInput(format!(
            "{aggregate_type} EventLedger payload missing record"
        ))
    })?;
    if !aggregate_id.is_empty() {
        let payload_id = match aggregate_type {
            "model_lane_run" => record.get("run_id"),
            "model_lane" => record.get("lane_id"),
            "model_lane_message" => record.get("message_id"),
            "model_lane_cloud_projection_plan" => record.get("projection_plan_id"),
            "model_lane_cloud_consent_receipt" => record.get("consent_receipt_id"),
            "model_lane_promotion_decision" => record.get("decision_id"),
            "model_lane_context_bundle_artifact" => record.get("artifact_binding_id"),
            "model_lane_context_bundle_handoff" => record.get("handoff_id"),
            "model_lane_recovery_checkpoint" => record.get("checkpoint_id"),
            "model_lane_recovery_event" => record.get("recovery_event_id"),
            "model_lane_lease" => record.get("lease_id"),
            "model_lane_diagnostic_tier" => record.get("diagnostic_status_id"),
            "model_lane_mt_runtime_status" => record.get("mt_status_id"),
            _ => None,
        }
        .and_then(Value::as_str)
        .unwrap_or_default();
        if payload_id != aggregate_id {
            return Err(ModelLaneError::InvalidInput(format!(
                "{aggregate_type} EventLedger payload aggregate_id mismatch: payload record id {payload_id}, ledger aggregate_id {aggregate_id}"
            )));
        }
    }
    serde_json::from_value(record.clone()).map_err(Into::into)
}

async fn validate_diagnostics_row_eventledger_authority(
    pool: &PgPool,
    run_id: &str,
) -> ModelLaneResult<()> {
    validate_diagnostics_row_eventledger_authority_for::<ModelLaneRunRecord, NewModelLaneRun>(
        pool,
        run_id,
        "model_lane_run",
        "model_lane_runs",
        "run_id",
        "run_id",
    )
    .await?;
    validate_diagnostics_row_eventledger_authority_for::<ModelLaneRecord, NewModelLane>(
        pool,
        run_id,
        "model_lane",
        "model_lanes",
        "lane_id",
        "run_id",
    )
    .await?;
    validate_model_lane_stable_anchor_authority(pool, run_id).await?;
    validate_diagnostics_row_eventledger_authority_for::<
        ModelLaneMessageRecord,
        NewModelLaneMessage,
    >(
        pool,
        run_id,
        "model_lane_message",
        "model_lane_messages",
        "message_id",
        "run_id",
    )
    .await?;
    // Artifact projections were the one durable ModelLane surface no replay,
    // diagnostics, or recovery read ever proved against the EventLedger, so a
    // row could be edited in place and every reader accepted it. The stage
    // output payload lives here, which makes it the highest-value row to tamper
    // and the only one that was unguarded.
    validate_diagnostics_row_eventledger_authority_for::<
        ModelLaneContextBundleArtifactBindingRecord,
        NewModelLaneContextBundleArtifactBinding,
    >(
        pool,
        run_id,
        "model_lane_context_bundle_artifact",
        "model_lane_context_bundle_artifacts",
        "artifact_binding_id",
        "run_id",
    )
    .await?;
    validate_diagnostics_row_eventledger_authority_for::<ModelLaneLeaseRecord, NewModelLaneLease>(
        pool,
        run_id,
        "model_lane_lease",
        "model_lane_leases",
        "lease_id",
        "run_id",
    )
    .await?;
    validate_diagnostics_row_eventledger_authority_for::<
        ModelLaneDiagnosticTierStatusRecord,
        NewModelLaneDiagnosticTierStatus,
    >(
        pool,
        run_id,
        "model_lane_diagnostic_tier",
        "model_lane_diagnostic_tier_statuses",
        "diagnostic_status_id",
        "run_id",
    )
    .await?;
    validate_diagnostics_row_eventledger_authority_for::<
        ModelLaneMtRuntimeStatusRecord,
        NewModelLaneMtRuntimeStatus,
    >(
        pool,
        run_id,
        "model_lane_mt_runtime_status",
        "model_lane_mt_runtime_statuses",
        "mt_status_id",
        "run_id",
    )
    .await
}

/// Prove the mutable ModelLane projection column against the immutable anchor
/// captured by the initial lane EventLedger event. The current lane event
/// reference can advance to terminal/status events, so this lookup deliberately
/// resolves the original `hsk.model_lane@1` event by aggregate identity instead
/// of trusting the row's latest event pointer.
async fn validate_model_lane_stable_anchor_authority(
    pool: &PgPool,
    run_id: &str,
) -> ModelLaneResult<()> {
    let rows = sqlx::query(
        r#"
        SELECT lanes.lane_id,
               lanes.model_stable_anchor AS row_model_stable_anchor,
               initial.event_id AS initial_event_id,
               initial.payload ->> 'model_stable_anchor' AS ledger_model_stable_anchor
        FROM model_lanes lanes
        LEFT JOIN LATERAL (
            SELECT ledger.event_id, ledger.payload
            FROM kernel_event_ledger ledger
            WHERE ledger.aggregate_type = 'model_lane'
              AND ledger.aggregate_id = lanes.lane_id
              AND ledger.payload ->> 'schema_id' = 'hsk.model_lane@1'
            ORDER BY ledger.event_sequence ASC
            LIMIT 1
        ) initial ON TRUE
        WHERE lanes.run_id = $1
        ORDER BY lanes.event_ledger_seq ASC
        "#,
    )
    .bind(run_id)
    .fetch_all(pool)
    .await?;

    for row in rows {
        let lane_id: String = row.try_get("lane_id")?;
        let row_anchor: Option<String> = row.try_get("row_model_stable_anchor")?;
        let initial_event_id: Option<String> = row.try_get("initial_event_id")?;
        let ledger_anchor: Option<String> = row.try_get("ledger_model_stable_anchor")?;
        if initial_event_id.is_none() {
            return Err(ModelLaneError::InvalidInput(format!(
                "model_lane {lane_id} diagnostics projection row drift: initial hsk.model_lane@1 EventLedger event missing"
            )));
        }
        if row_anchor != ledger_anchor {
            return Err(ModelLaneError::InvalidInput(format!(
                "model_lane {lane_id} diagnostics projection row drift: model_stable_anchor does not match initial EventLedger payload"
            )));
        }
    }
    Ok(())
}

async fn validate_diagnostics_row_eventledger_authority_for<R, I>(
    pool: &PgPool,
    run_id: &str,
    aggregate_type: &'static str,
    table_name: &'static str,
    id_field: &'static str,
    run_field: &'static str,
) -> ModelLaneResult<()>
where
    R: for<'de> Deserialize<'de> + Deref<Target = I>,
    I: for<'de> Deserialize<'de> + PartialEq,
{
    let row_sequence_metadata = match table_name {
        "model_lane_messages"
        | "model_lane_leases"
        | "model_lane_diagnostic_tier_statuses"
        | "model_lane_mt_runtime_statuses"
        | "model_lane_context_bundle_artifacts" => {
            "rows.event_stream_version AS row_event_stream_version,
               rows.transaction_seq AS row_transaction_seq,"
        }
        _ => {
            "NULL::BIGINT AS row_event_stream_version,
               NULL::BIGINT AS row_transaction_seq,"
        }
    };
    let sql = format!(
        r#"
        SELECT rows.{id_field} AS row_id,
               rows.record_json AS record_json,
               rows.event_ledger_event_id AS row_event_ledger_event_id,
               rows.event_ledger_seq AS row_event_ledger_seq,
               {row_sequence_metadata}
               ledger.aggregate_id AS aggregate_id,
               ledger.event_id AS ledger_event_id,
               ledger.event_sequence AS ledger_event_sequence,
               ledger.payload AS payload
        FROM {table_name} rows
        LEFT JOIN kernel_event_ledger ledger
          ON ledger.event_id = rows.event_ledger_event_id
        WHERE rows.{run_field} = $1
        ORDER BY rows.event_ledger_seq ASC
        "#
    );
    for row in sqlx::query(&sql).bind(run_id).fetch_all(pool).await? {
        let sql_row_id: String = row.try_get("row_id")?;
        let record_json: Value = row.try_get("record_json")?;
        let row_event_ledger_event_id: String = row.try_get("row_event_ledger_event_id")?;
        let row_event_ledger_seq: i64 = row.try_get("row_event_ledger_seq")?;
        let row_event_stream_version: Option<i64> = row.try_get("row_event_stream_version")?;
        let row_transaction_seq: Option<i64> = row.try_get("row_transaction_seq")?;
        let aggregate_id: Option<String> = row.try_get("aggregate_id")?;
        let ledger_event_id: Option<String> = row.try_get("ledger_event_id")?;
        let ledger_event_sequence: Option<i64> = row.try_get("ledger_event_sequence")?;
        let payload: Option<Value> = row.try_get("payload")?;
        let (Some(aggregate_id), Some(ledger_event_id), Some(ledger_event_sequence), Some(payload)) = (
            aggregate_id,
            ledger_event_id,
            ledger_event_sequence,
            payload,
        ) else {
            return Err(ModelLaneError::InvalidInput(format!(
                "{aggregate_type} {sql_row_id} diagnostics projection row drift: row EventLedger columns do not resolve to kernel_event_ledger"
            )));
        };
        let ledger_record: I = event_payload_record(&payload, aggregate_type, &aggregate_id)?;
        let row_id = payload
            .get("record")
            .and_then(|record| record.get(id_field))
            .and_then(Value::as_str)
            .unwrap_or(aggregate_id.as_str());
        // Validate row IDENTITY against the ledger before deserializing/comparing the
        // mutable record body. A mutable row whose primary-key id was aliased onto
        // another valid ledger event is an identity tamper and MUST surface as the typed
        // "SQL row <id> does not match kernel_event_ledger" drift diagnosis -- not as a
        // raw deserialization error on the aliased (foreign-shaped) body. Identity is
        // logically prior to body equality: comparing bodies is meaningless once the row
        // points at the wrong ledger event. Per spec 4.3.9.2.5 recovery diagnostics MUST
        // be structured, not inferred from prose (a raw serde "missing field" is not).
        if sql_row_id != row_id || sql_row_id != aggregate_id {
            return Err(ModelLaneError::InvalidInput(format!(
                "{aggregate_type} {sql_row_id} diagnostics projection row drift: SQL row {id_field} does not match kernel_event_ledger aggregate/payload id {row_id}"
            )));
        }
        validate_record_json_eventledger_metadata(
            aggregate_type,
            row_id,
            &record_json,
            &row_event_ledger_event_id,
            row_event_ledger_seq,
            row_event_stream_version,
            row_transaction_seq,
            &ledger_event_id,
            ledger_event_sequence,
        )?;
        let row_record: R = serde_json::from_value(record_json.clone())?;
        if row_record.deref() != &ledger_record {
            return Err(ModelLaneError::InvalidInput(format!(
                "{aggregate_type} {row_id} diagnostics projection row drift: mutable row does not match its EventLedger payload in kernel_event_ledger"
            )));
        }
        if aggregate_type == "model_lane_message" {
            let row_binding = record_json
                .get("crdt_authority_binding")
                .cloned()
                .unwrap_or(Value::Null);
            let ledger_binding = payload
                .get("crdt_authority_binding")
                .cloned()
                .unwrap_or(Value::Null);
            if row_binding != ledger_binding {
                return Err(ModelLaneError::InvalidInput(format!(
                    "{aggregate_type} {row_id} diagnostics projection row drift: mutable crdt_authority_binding does not match kernel_event_ledger payload"
                )));
            }
        }
    }
    Ok(())
}

fn validate_record_json_eventledger_metadata(
    aggregate_type: &str,
    row_id: &str,
    record_json: &Value,
    row_event_ledger_event_id: &str,
    row_event_ledger_seq: i64,
    row_event_stream_version: Option<i64>,
    row_transaction_seq: Option<i64>,
    ledger_event_id: &str,
    ledger_event_sequence: i64,
) -> ModelLaneResult<()> {
    if row_event_ledger_event_id != ledger_event_id || row_event_ledger_seq != ledger_event_sequence
    {
        return Err(ModelLaneError::InvalidInput(format!(
            "{aggregate_type} {row_id} diagnostics projection row drift: row EventLedger columns do not match kernel_event_ledger"
        )));
    }
    if let Some(actual) = row_event_stream_version {
        if actual != ledger_event_sequence {
            return Err(ModelLaneError::InvalidInput(format!(
                "{aggregate_type} {row_id} diagnostics projection row drift: row event_stream_version does not match kernel_event_ledger"
            )));
        }
    }
    if let Some(actual) = row_transaction_seq {
        if actual != ledger_event_sequence {
            return Err(ModelLaneError::InvalidInput(format!(
                "{aggregate_type} {row_id} diagnostics projection row drift: row transaction_seq does not match kernel_event_ledger"
            )));
        }
    }
    let Some(record_event_id) = record_json
        .get("event_ledger_event_id")
        .and_then(Value::as_str)
    else {
        return Err(ModelLaneError::InvalidInput(format!(
            "{aggregate_type} {row_id} diagnostics projection row drift: record_json missing event_ledger_event_id"
        )));
    };
    if record_event_id != ledger_event_id {
        return Err(ModelLaneError::InvalidInput(format!(
            "{aggregate_type} {row_id} diagnostics projection row drift: record_json event_ledger_event_id does not match kernel_event_ledger"
        )));
    }
    validate_record_json_i64_metadata(
        aggregate_type,
        row_id,
        record_json,
        "event_ledger_seq",
        ledger_event_sequence,
    )?;
    validate_optional_record_json_i64_metadata(
        aggregate_type,
        row_id,
        record_json,
        "event_stream_version",
        ledger_event_sequence,
    )?;
    validate_optional_record_json_i64_metadata(
        aggregate_type,
        row_id,
        record_json,
        "transaction_seq",
        ledger_event_sequence,
    )
}

fn validate_record_json_i64_metadata(
    aggregate_type: &str,
    row_id: &str,
    record_json: &Value,
    field: &str,
    expected: i64,
) -> ModelLaneResult<()> {
    match record_json.get(field).and_then(Value::as_i64) {
        Some(actual) if actual == expected => Ok(()),
        Some(_) => Err(ModelLaneError::InvalidInput(format!(
            "{aggregate_type} {row_id} diagnostics projection row drift: record_json {field} does not match kernel_event_ledger"
        ))),
        None => Err(ModelLaneError::InvalidInput(format!(
            "{aggregate_type} {row_id} diagnostics projection row drift: record_json missing {field}"
        ))),
    }
}

fn validate_optional_record_json_i64_metadata(
    aggregate_type: &str,
    row_id: &str,
    record_json: &Value,
    field: &str,
    expected: i64,
) -> ModelLaneResult<()> {
    match record_json.get(field).and_then(Value::as_i64) {
        Some(actual) if actual == expected => Ok(()),
        Some(_) => Err(ModelLaneError::InvalidInput(format!(
            "{aggregate_type} {row_id} diagnostics projection row drift: record_json {field} does not match kernel_event_ledger"
        ))),
        None => Ok(()),
    }
}

/// Re-authorize one row's stored scope and decode it. Every generic navigation
/// helper funnels through here so the second (post-deserialization) enforcement
/// layer cannot be forgotten at an individual call site.
fn authorize_and_decode_row<T>(
    access: &ResourceAccessContext,
    row: sqlx::postgres::PgRow,
) -> ModelLaneResult<T>
where
    T: DeserializeOwned,
{
    access.authorize_row(&stored_resource_scope_from_row(&row)?)?;
    row_to_json(row, "record_json").and_then(|v| serde_json::from_value(v).map_err(Into::into))
}

fn require_exact_context_bundle_write_scope(scope: ScopeColumnValues<'_>) -> ModelLaneResult<()> {
    if scope.owner_account_id.is_none()
        || scope.actor_principal_id.is_none()
        || scope.authenticated_session_id.is_none()
        || scope.access_space_id.is_none()
        || scope.workspace_id.is_none()
    {
        return Err(ModelLaneError::AuthorityDenied(
            "ContextBundle writes require exact owner, Principal, authenticated session, AccessSpace, and workspace authority"
                .into(),
        ));
    }
    Ok(())
}

fn require_exact_context_bundle_read_scope(
    access: &ResourceAccessContext,
) -> ModelLaneResult<&ExactResourceScopeAttribution> {
    access.exact_read_scope().ok_or_else(|| {
        ModelLaneError::AuthorityDenied(
            "ContextBundle reads require exact owner, Principal, authenticated session, AccessSpace, and workspace authority"
                .into(),
        )
    })
}

fn context_bundle_authorize_and_decode_write_scope<T>(
    scope: ScopeColumnValues<'_>,
    row: sqlx::postgres::PgRow,
) -> ModelLaneResult<T>
where
    T: DeserializeOwned,
{
    let stored = stored_resource_scope_from_row(&row)?;
    if stored.owner_account_id.map(|value| value.as_uuid()) != scope.owner_account_id
        || stored.actor_principal_id.map(|value| value.as_uuid()) != scope.actor_principal_id
        || stored.authenticated_session.map(|value| value.as_uuid())
            != scope.authenticated_session_id
        || stored.access_space.map(|value| value.as_uuid()) != scope.access_space_id
        || stored.workspace.as_ref().map(|value| value.as_str()) != scope.workspace_id
    {
        return Err(ModelLaneError::ScopeDenied(
            ScopeDenied::ExactAttributionMismatch,
        ));
    }
    row_to_json(row, "record_json")
        .and_then(|value| serde_json::from_value(value).map_err(Into::into))
}

async fn context_bundle_record_by_key_for_write_scope_tx<T>(
    tx: &mut Transaction<'_, Postgres>,
    table: &str,
    key_column: &str,
    key: &str,
    scope: ScopeColumnValues<'_>,
    for_update: bool,
) -> ModelLaneResult<Option<T>>
where
    T: DeserializeOwned,
{
    require_exact_context_bundle_write_scope(scope)?;
    let lock = if for_update { " FOR UPDATE" } else { "" };
    let sql = format!(
        "SELECT record_json, {RESOURCE_SCOPE_SELECT_COLUMNS} FROM {table} \
         WHERE {key_column} = $1 \
           AND owner_account_id IS NOT DISTINCT FROM $2::uuid \
           AND actor_principal_id IS NOT DISTINCT FROM $3::uuid \
           AND authenticated_session_id IS NOT DISTINCT FROM $4::uuid \
           AND access_space_id IS NOT DISTINCT FROM $5::uuid \
           AND workspace_id IS NOT DISTINCT FROM $6{lock}"
    );
    let row = sqlx::query(&sql)
        .bind(key)
        .bind(scope.owner_account_id)
        .bind(scope.actor_principal_id)
        .bind(scope.authenticated_session_id)
        .bind(scope.access_space_id)
        .bind(scope.workspace_id)
        .fetch_optional(&mut **tx)
        .await?;
    row.map(|row| context_bundle_authorize_and_decode_write_scope(scope, row))
        .transpose()
}

async fn select_record_by_column<T>(
    pool: &PgPool,
    access: &ResourceAccessContext,
    table_name: &'static str,
    column_name: &'static str,
    value: &str,
) -> ModelLaneResult<Option<T>>
where
    T: DeserializeOwned,
{
    let predicate = access.sql_predicate(2);
    let sql = format!(
        "SELECT record_json, {RESOURCE_SCOPE_SELECT_COLUMNS} FROM {table_name} WHERE {column_name} = $1{} ORDER BY event_ledger_seq ASC LIMIT 1",
        predicate.clause()
    );
    predicate
        .bind(sqlx::query(&sql).bind(value))
        .fetch_optional(pool)
        .await?
        .map(|row| authorize_and_decode_row(access, row))
        .transpose()
}

/// Look up a single record by a field that lives inside the `record_json` JSONB
/// payload rather than as a physical column. Several ModelLane navigation
/// identifiers (`context_bundle_id`, `model_session_id`, `session_id`,
/// `memory_pack_ref`, `failstate_code`, ...) are stored only in `record_json`;
/// querying them as physical columns raises a fail-closed "column does not
/// exist" database error that surfaces to callers as a 500. Resolving through
/// the JSONB text accessor keeps a valid query from ever 500-ing.
async fn select_record_by_json_field<T>(
    pool: &PgPool,
    access: &ResourceAccessContext,
    table_name: &'static str,
    json_field: &'static str,
    value: &str,
) -> ModelLaneResult<Option<T>>
where
    T: DeserializeOwned,
{
    let predicate = access.sql_predicate(2);
    let sql = format!(
        "SELECT record_json, {RESOURCE_SCOPE_SELECT_COLUMNS} FROM {table_name} WHERE record_json->>'{json_field}' = $1{} ORDER BY event_ledger_seq ASC LIMIT 1",
        predicate.clause()
    );
    predicate
        .bind(sqlx::query(&sql).bind(value))
        .fetch_optional(pool)
        .await?
        .map(|row| authorize_and_decode_row(access, row))
        .transpose()
}

/// `run_id` companion to [`select_record_by_json_field`] for aggregate lookups
/// that resolve a set of run ids by a `record_json`-only field. `run_id` remains
/// a physical column on every ModelLane table, so only the WHERE predicate moves
/// into the JSONB payload.
async fn select_run_ids_by_json_field(
    pool: &PgPool,
    access: &ResourceAccessContext,
    table_name: &'static str,
    json_field: &'static str,
    value: &str,
) -> ModelLaneResult<Vec<String>> {
    let predicate = access.sql_predicate(2);
    // These helpers project only `run_id`, so there is no row to re-authorize
    // afterwards. The SQL predicate is therefore the enforcement point, and the
    // run id it yields is re-checked by `replay_run` before any row is
    // disclosed — that is the second layer for this path.
    let sql = format!(
        "SELECT DISTINCT run_id FROM {table_name} WHERE record_json->>'{json_field}' = $1{} ORDER BY run_id ASC",
        predicate.clause()
    );
    predicate
        .bind_scalar(sqlx::query_scalar::<_, String>(&sql).bind(value))
        .fetch_all(pool)
        .await
        .map_err(Into::into)
}

async fn select_run_ids_by_column(
    pool: &PgPool,
    access: &ResourceAccessContext,
    table_name: &'static str,
    column_name: &'static str,
    value: &str,
) -> ModelLaneResult<Vec<String>> {
    let predicate = access.sql_predicate(2);
    let sql = format!(
        "SELECT DISTINCT run_id FROM {table_name} WHERE {column_name} = $1{} ORDER BY run_id ASC",
        predicate.clause()
    );
    predicate
        .bind_scalar(sqlx::query_scalar::<_, String>(&sql).bind(value))
        .fetch_all(pool)
        .await
        .map_err(Into::into)
}

#[derive(Debug)]
enum ValidatedNavigationOrigin {
    Run(ModelLaneRunRecord),
    Lane(ModelLaneRecord),
    Message(ModelLaneMessageRecord),
}

impl ValidatedNavigationOrigin {
    fn run_id(&self) -> &str {
        match self {
            Self::Run(record) => &record.run_id,
            Self::Lane(record) => &record.run_id,
            Self::Message(record) => &record.run_id,
        }
    }
}

/// Resolve mutable navigation origins under one locking transaction. The
/// projection row and its EventLedger authority are reconciled before any
/// caller is allowed to consume `record_json.run_id`.
async fn validated_navigation_origins(
    pool: &PgPool,
    access: &ResourceAccessContext,
    origin: &str,
    condition: &str,
    value: &str,
) -> ModelLaneResult<Vec<ValidatedNavigationOrigin>> {
    let table = match origin {
        "run" => "model_lane_runs",
        "lane" => "model_lanes",
        "message" => "model_lane_messages",
        _ => {
            return Err(ModelLaneError::InvalidInput(
                "unsupported navigation origin".into(),
            ));
        }
    };
    let mut tx = pool.begin().await?;
    let predicate = access.sql_predicate(2);
    let sql = format!(
        "SELECT record_json, {RESOURCE_SCOPE_SELECT_COLUMNS} FROM {table} \
         WHERE ({condition}){} ORDER BY record_json->>'run_id' ASC FOR SHARE",
        predicate.clause()
    );
    let rows = predicate
        .bind(sqlx::query(&sql).bind(value))
        .fetch_all(&mut *tx)
        .await?;
    let mut records = Vec::with_capacity(rows.len());
    for row in rows {
        match origin {
            "run" => {
                let record: ModelLaneRunRecord = authorize_and_decode_row(access, row)?;
                validate_stored_run_eventledger_authority_tx(
                    &mut tx,
                    &record,
                    access.exact_read_scope(),
                )
                .await
                .map_err(|_| {
                    ModelLaneError::AuthorityDenied(
                        "ModelLane navigation authority unavailable".into(),
                    )
                })?;
                records.push(ValidatedNavigationOrigin::Run(record));
            }
            "lane" => {
                let record: ModelLaneRecord = authorize_and_decode_row(access, row)?;
                validate_stored_lane_eventledger_authority_tx(
                    &mut tx,
                    &record,
                    access.exact_read_scope(),
                )
                .await
                .map_err(|_| {
                    ModelLaneError::AuthorityDenied(
                        "ModelLane navigation authority unavailable".into(),
                    )
                })?;
                records.push(ValidatedNavigationOrigin::Lane(record));
            }
            "message" => {
                let record: ModelLaneMessageRecord = authorize_and_decode_row(access, row)?;
                validate_stored_message_eventledger_authority_tx(
                    &mut tx,
                    &record,
                    access.exact_read_scope(),
                )
                .await
                .map_err(|_| {
                    ModelLaneError::AuthorityDenied(
                        "ModelLane navigation authority unavailable".into(),
                    )
                })?;
                records.push(ValidatedNavigationOrigin::Message(record));
            }
            _ => unreachable!("origin validated above"),
        }
    }
    tx.commit().await?;
    Ok(records)
}

async fn validated_navigation_run_ids(
    pool: &PgPool,
    access: &ResourceAccessContext,
    origin: &str,
    condition: &str,
    value: &str,
) -> ModelLaneResult<Vec<String>> {
    Ok(
        validated_navigation_origins(pool, access, origin, condition, value)
            .await?
            .into_iter()
            .map(|record| record.run_id().to_owned())
            .collect(),
    )
}

async fn validated_navigation_handoff_run_ids(
    pool: &PgPool,
    access: &ResourceAccessContext,
    condition: &str,
    value: &str,
) -> ModelLaneResult<Vec<String>> {
    let mut tx = pool.begin().await?;
    let predicate = access.sql_predicate(2);
    let sql = format!(
        "SELECT record_json, {RESOURCE_SCOPE_SELECT_COLUMNS} \
         FROM model_lane_context_bundle_handoffs WHERE ({condition}){} \
         ORDER BY run_id ASC FOR SHARE",
        predicate.clause()
    );
    let rows = predicate
        .bind(sqlx::query(&sql).bind(value))
        .fetch_all(&mut *tx)
        .await?;
    let mut run_ids = Vec::with_capacity(rows.len());
    for row in rows {
        let record: ModelLaneContextBundleHandoffRecord = authorize_and_decode_row(access, row)?;
        validate_stored_context_bundle_handoff_authority_tx(&mut tx, access, &record)
            .await
            .map_err(|_| {
                ModelLaneError::AuthorityDenied("ModelLane navigation authority unavailable".into())
            })?;
        run_ids.push(record.run_id.clone());
    }
    tx.commit().await?;
    Ok(run_ids)
}

fn unique_run_id_for_lookup(
    lookup_kind: &str,
    lookup_ref: &str,
    run_ids: Vec<String>,
) -> ModelLaneResult<Option<String>> {
    let unique = run_ids.into_iter().collect::<BTreeSet<_>>();
    match unique.len() {
        0 => Ok(None),
        1 => Ok(unique.into_iter().next()),
        _ => {
            let candidates = unique.into_iter().collect::<Vec<_>>();
            Err(ModelLaneError::AmbiguousLookup(format!(
                "{lookup_kind} {lookup_ref} resolves to multiple runs: {}",
                candidates.join(", ")
            )))
        }
    }
}

async fn select_records_by_column<T>(
    pool: &PgPool,
    access: &ResourceAccessContext,
    table_name: &'static str,
    column_name: &'static str,
    value: &str,
) -> ModelLaneResult<Vec<T>>
where
    T: DeserializeOwned,
{
    let predicate = access.sql_predicate(2);
    let sql = format!(
        "SELECT record_json, {RESOURCE_SCOPE_SELECT_COLUMNS} FROM {table_name} WHERE {column_name} = $1{} ORDER BY event_ledger_seq ASC",
        predicate.clause()
    );
    predicate
        .bind(sqlx::query(&sql).bind(value))
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|row| authorize_and_decode_row(access, row))
        .collect()
}

async fn select_records_by_any_artifact_ref(
    pool: &PgPool,
    access: &ResourceAccessContext,
    value: &str,
) -> ModelLaneResult<Vec<ModelLaneContextBundleArtifactBindingRecord>> {
    let predicate = access.sql_predicate(2);
    let sql = format!(
        r#"
        SELECT record_json, {RESOURCE_SCOPE_SELECT_COLUMNS}
        FROM model_lane_context_bundle_artifacts
        WHERE (artifact_ref = $1
           OR artifact_payload_ref = $1
           OR artifact_manifest_ref = $1
           OR artifact_binding_id = $1
           OR artifact_sha256 = $1
           OR content_hash = $1){}
        ORDER BY event_ledger_seq ASC
        "#,
        predicate.clause()
    );
    predicate
        .bind(sqlx::query(&sql).bind(value))
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|row| authorize_and_decode_row(access, row))
        .collect()
}

async fn select_records_by_any_handoff_artifact_ref(
    pool: &PgPool,
    access: &ResourceAccessContext,
    value: &str,
) -> ModelLaneResult<Vec<ModelLaneContextBundleHandoffRecord>> {
    let predicate = access.sql_predicate(2);
    let sql = format!(
        r#"
        SELECT record_json, {RESOURCE_SCOPE_SELECT_COLUMNS}
        FROM model_lane_context_bundle_handoffs
        WHERE (artifact_ref = $1
           OR artifact_sha256 = $1
           OR content_hash = $1){}
        ORDER BY event_ledger_seq ASC
        "#,
        predicate.clause()
    );
    predicate
        .bind(sqlx::query(&sql).bind(value))
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|row| authorize_and_decode_row(access, row))
        .collect()
}

fn dedupe_context_handoffs(rows: &mut Vec<ModelLaneContextBundleHandoffRecord>) {
    let mut seen = BTreeSet::new();
    rows.retain(|row| seen.insert(row.handoff_id.clone()));
}

fn artifact_matches(row: &ModelLaneContextBundleArtifactBindingRecord, value: &str) -> bool {
    row.artifact_ref == value
        || row.artifact_binding_id == value
        || row.artifact_manifest_ref == value
        || row.artifact_payload_ref == value
        || row.artifact_sha256 == value
        || row.content_hash == value
}

fn message_mentions_lane(row: &ModelLaneMessageRecord, lane_id: &str) -> bool {
    row.from_lane_id == lane_id
        || matches!(&row.to_lane, ModelLaneTarget::Lane(target_lane_id) if target_lane_id == lane_id)
}

fn span_matches(span_id: Option<&str>, actual: &str) -> bool {
    span_id.map_or(true, |expected| expected == actual)
}

fn push_event_ref(refs: &mut BTreeSet<String>, event_id: &str) {
    if !event_id.is_empty() {
        refs.insert(format!("eventledger://kernel/{event_id}"));
    }
}

fn push_event_seq_ref(refs: &mut BTreeSet<String>, event_seq: i64) {
    if event_seq > 0 {
        refs.insert(format!("eventledger://kernel/seq/{event_seq}"));
    }
}

fn push_optional_string(refs: &mut BTreeSet<String>, value: Option<&str>) {
    if let Some(value) = value.filter(|value| !value.trim().is_empty()) {
        refs.insert(value.to_owned());
    }
}

fn push_optional_json_string(refs: &mut BTreeSet<String>, payload: &Value, key: &str) {
    push_optional_string(refs, payload.get(key).and_then(Value::as_str));
}

fn nonempty_lookup_value(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

async fn query_optional_run_id(
    pool: &PgPool,
    sql: &str,
    value: &str,
) -> ModelLaneResult<Option<String>> {
    sqlx::query_scalar::<_, String>(sql)
        .bind(value)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
}

fn event_payload_run_id(payload: &Value) -> Option<String> {
    payload
        .get("record")
        .and_then(|record| record.get("run_id"))
        .and_then(Value::as_str)
        .or_else(|| payload.get("run_id").and_then(Value::as_str))
        .map(str::to_owned)
}

async fn validate_recovery_event_stream(
    pool: &PgPool,
    run_id: &str,
    recovery_bound_event_ledger_seq: i64,
    events: &[ModelLaneRecoveryEventRecord],
) -> ModelLaneResult<()> {
    let mut expected = 1_i64;
    for event in events {
        if event.replay_order_seq != expected {
            let failure = ModelLaneRecoveryFailureKind::EventLedgerSequenceGap;
            return Err(ModelLaneError::InvalidInput(format!(
                "{} {} recovery replay gap for run_id {run_id}: expected replay_order_seq {expected}, got {}",
                failure.code(),
                failure.as_str(),
                event.replay_order_seq
            )));
        }
        expected += 1;
        if event.event_ledger_seq > recovery_bound_event_ledger_seq {
            let failure = ModelLaneRecoveryFailureKind::EventLedgerSequenceGap;
            return Err(ModelLaneError::InvalidInput(format!(
                "{} {} recovery_event_id {} is after recovery high-watermark {}",
                failure.code(),
                failure.as_str(),
                event.recovery_event_id,
                recovery_bound_event_ledger_seq
            )));
        }
        let row = sqlx::query(
            r#"
            SELECT event_sequence, session_run_id, payload
            FROM kernel_event_ledger
            WHERE event_id = $1
              AND aggregate_type = 'model_lane_recovery_event'
              AND aggregate_id = $2
            "#,
        )
        .bind(&event.event_ledger_event_id)
        .bind(&event.recovery_event_id)
        .fetch_optional(pool)
        .await?;
        let Some(row) = row else {
            let failure = ModelLaneRecoveryFailureKind::MissingEventLedgerRow;
            return Err(ModelLaneError::InvalidInput(format!(
                "{} {} recovery_event_id {} is not backed by matching kernel_event_ledger row",
                failure.code(),
                failure.as_str(),
                event.recovery_event_id
            )));
        };
        let ledger_seq: i64 = row.try_get("event_sequence")?;
        let session_run_id: String = row.try_get("session_run_id")?;
        if ledger_seq != event.event_ledger_seq || session_run_id != event.event_ledger_stream_id {
            let failure = ModelLaneRecoveryFailureKind::MissingEventLedgerRow;
            return Err(ModelLaneError::InvalidInput(format!(
                "{} {} recovery_event_id {} is not backed by matching kernel_event_ledger row",
                failure.code(),
                failure.as_str(),
                event.recovery_event_id
            )));
        }
        let ledger_payload: Value = row.try_get("payload")?;
        let ledger_record: ModelLaneRecoveryEventRecord = event_payload_record(
            &ledger_payload,
            "model_lane_recovery_event",
            &event.recovery_event_id,
        )?;
        if &ledger_record != event {
            let failure = ModelLaneRecoveryFailureKind::MissingEventLedgerRow;
            return Err(ModelLaneError::InvalidInput(format!(
                "{} {} recovery_event_id {} mutable row differs from EventLedger payload",
                failure.code(),
                failure.as_str(),
                event.recovery_event_id
            )));
        }
        if let Some(source_event_ledger_seq) = event.source_event_ledger_seq {
            let source_stream: Option<String> = sqlx::query_scalar(
                "SELECT session_run_id FROM kernel_event_ledger WHERE event_sequence = $1",
            )
            .bind(source_event_ledger_seq)
            .fetch_optional(pool)
            .await?;
            if source_stream.as_deref() != Some(event.event_ledger_stream_id.as_str())
                || source_event_ledger_seq > event.event_ledger_seq
            {
                let failure = ModelLaneRecoveryFailureKind::MissingEventLedgerRow;
                return Err(ModelLaneError::InvalidInput(format!(
                    "{} {} source_event_ledger_seq {source_event_ledger_seq} for recovery_event_id {} is missing, cross-stream, or after the recovery event",
                    failure.code(),
                    failure.as_str(),
                    event.recovery_event_id
                )));
            }
        }
    }
    Ok(())
}

fn validate_contiguous_recovery_order(
    run_id: &str,
    events: &[ModelLaneRecoveryEventRecord],
) -> ModelLaneResult<()> {
    for (index, event) in events.iter().enumerate() {
        let expected = index as i64 + 1;
        if event.replay_order_seq != expected {
            let failure = ModelLaneRecoveryFailureKind::EventLedgerSequenceGap;
            return Err(ModelLaneError::InvalidInput(format!(
                "{} {} fenced recovery ordering gap for run_id {run_id}: expected replay_order_seq {expected}, got {}",
                failure.code(),
                failure.as_str(),
                event.replay_order_seq
            )));
        }
    }
    Ok(())
}

async fn validate_recovery_payload_refs(
    pool: &PgPool,
    run_id: &str,
    checkpoint: &ModelLaneRecoveryCheckpointRecord,
    checkpoint_bound_event_ledger_seq: i64,
    forward_bound_event_ledger_seq: i64,
    events: &[ModelLaneRecoveryEventRecord],
) -> ModelLaneResult<()> {
    // Payload refs that were OPEN at the checkpoint MUST have been satisfied by
    // ArtifactStore/EventLedger authority at/before the checkpoint. A post-checkpoint
    // artifact "repair" of such an already-checkpointed ref fails closed, so these
    // stay bounded at the checkpoint.
    let checkpoint_refs: BTreeSet<String> = checkpoint.open_payload_refs.iter().cloned().collect();
    validate_payload_authority_refs(
        pool,
        run_id,
        checkpoint,
        checkpoint_bound_event_ledger_seq,
        checkpoint_refs,
    )
    .await?;
    // Caught-up (post-checkpoint) recovery events reference NEW forward-stream payloads;
    // their authority is validated at the forward catch-up bound so genuine post-checkpoint
    // progress replays while checkpointed-ref repairs above still fail closed.
    let mut forward_refs = BTreeSet::new();
    for event in events {
        forward_refs.extend(event.payload_refs.iter().cloned());
    }
    validate_payload_authority_refs(
        pool,
        run_id,
        checkpoint,
        forward_bound_event_ledger_seq,
        forward_refs,
    )
    .await
}

async fn validate_replay_message_payload_authority(
    pool: &PgPool,
    run_id: &str,
    checkpoint: &ModelLaneRecoveryCheckpointRecord,
    recovery_bound_event_ledger_seq: i64,
    messages: &[ModelLaneMessageRecord],
) -> ModelLaneResult<()> {
    let mut refs = BTreeSet::new();
    let mut expected_hashes = BTreeMap::new();
    for message in messages {
        refs.insert(message.payload_ref.clone());
        if let Some(existing_hash) =
            expected_hashes.insert(message.payload_ref.clone(), message.payload_sha256.clone())
        {
            require_equal(
                "message.payload_sha256",
                &message.payload_sha256,
                "existing.payload_sha256",
                &existing_hash,
            )?;
        }
    }
    validate_payload_authority_refs(
        pool,
        run_id,
        checkpoint,
        recovery_bound_event_ledger_seq,
        refs,
    )
    .await?;
    validate_payload_authority_hashes(
        pool,
        run_id,
        checkpoint,
        recovery_bound_event_ledger_seq,
        expected_hashes,
    )
    .await
}

async fn validate_payload_authority_refs(
    pool: &PgPool,
    run_id: &str,
    checkpoint: &ModelLaneRecoveryCheckpointRecord,
    recovery_bound_event_ledger_seq: i64,
    refs: BTreeSet<String>,
) -> ModelLaneResult<()> {
    for payload_ref in refs {
        require_token("recovery.payload_ref", &payload_ref)?;
        let row = sqlx::query(
            r#"
            SELECT artifacts.record_json AS artifact_record_json,
                   ledger.aggregate_id AS ledger_aggregate_id,
                   ledger.payload AS ledger_payload
            FROM model_lane_context_bundle_artifacts artifacts
            JOIN kernel_event_ledger ledger
              ON ledger.event_id = artifacts.event_ledger_event_id
             AND ledger.event_sequence = artifacts.event_ledger_seq
             AND ledger.aggregate_type = 'model_lane_context_bundle_artifact'
            WHERE artifacts.run_id = $1
              AND (artifacts.artifact_ref = $2 OR artifacts.artifact_payload_ref = $2)
              AND artifacts.event_ledger_stream_id = $3
              AND artifacts.event_ledger_seq <= $4
              AND ledger.session_run_id = $3
            ORDER BY artifacts.event_ledger_seq DESC
            LIMIT 1
            "#,
        )
        .bind(run_id)
        .bind(&payload_ref)
        .bind(&checkpoint.event_ledger_stream_id)
        .bind(recovery_bound_event_ledger_seq)
        .fetch_optional(pool)
        .await?;
        let Some(row) = row else {
            let failure = ModelLaneRecoveryFailureKind::MissingPayloadAuthority;
            return Err(ModelLaneError::InvalidInput(format!(
                "{} {} payload_ref {payload_ref} is not backed by recovery-bounded ArtifactStore/EventLedger authority",
                failure.code(),
                failure.as_str()
            )));
        };
        let artifact_record_json: Value = row.try_get("artifact_record_json")?;
        let artifact_record: ModelLaneContextBundleArtifactBindingRecord =
            serde_json::from_value(artifact_record_json)?;
        let ledger_aggregate_id: String = row.try_get("ledger_aggregate_id")?;
        let ledger_payload: Value = row.try_get("ledger_payload")?;
        let ledger_record: ModelLaneContextBundleArtifactBindingRecord = event_payload_record(
            &ledger_payload,
            "model_lane_context_bundle_artifact",
            &ledger_aggregate_id,
        )?;
        require_equal(
            "artifact.sql_row_id",
            &artifact_record.artifact_binding_id,
            "ledger.aggregate_id",
            &ledger_aggregate_id,
        )?;
        if ledger_record != artifact_record {
            let failure = ModelLaneRecoveryFailureKind::MissingPayloadAuthority;
            return Err(ModelLaneError::InvalidInput(format!(
                "{} {} payload_ref {payload_ref} artifact row differs from EventLedger payload",
                failure.code(),
                failure.as_str()
            )));
        }
    }
    Ok(())
}

async fn validate_payload_authority_hashes(
    pool: &PgPool,
    run_id: &str,
    checkpoint: &ModelLaneRecoveryCheckpointRecord,
    recovery_bound_event_ledger_seq: i64,
    expected_hashes: BTreeMap<String, String>,
) -> ModelLaneResult<()> {
    for (payload_ref, payload_sha256) in expected_hashes {
        require_token("recovery.payload_ref", &payload_ref)?;
        validate_sha256("message.payload_sha256", &payload_sha256)?;
        let row = sqlx::query(
            r#"
            SELECT artifacts.record_json AS artifact_record_json
            FROM model_lane_context_bundle_artifacts artifacts
            WHERE artifacts.run_id = $1
              AND (artifacts.artifact_ref = $2 OR artifacts.artifact_payload_ref = $2)
              AND artifacts.event_ledger_stream_id = $3
              AND artifacts.event_ledger_seq <= $4
            ORDER BY artifacts.event_ledger_seq DESC
            LIMIT 1
            "#,
        )
        .bind(run_id)
        .bind(&payload_ref)
        .bind(&checkpoint.event_ledger_stream_id)
        .bind(recovery_bound_event_ledger_seq)
        .fetch_optional(pool)
        .await?;
        let Some(row) = row else {
            let failure = ModelLaneRecoveryFailureKind::MissingPayloadAuthority;
            return Err(ModelLaneError::InvalidInput(format!(
                "{} {} payload_ref {payload_ref} is not backed by recovery-bounded ArtifactStore authority",
                failure.code(),
                failure.as_str()
            )));
        };
        let artifact_record_json: Value = row.try_get("artifact_record_json")?;
        let artifact_record: ModelLaneContextBundleArtifactBindingRecord =
            serde_json::from_value(artifact_record_json)?;
        if artifact_record.content_hash != payload_sha256
            || artifact_record.artifact_sha256 != payload_sha256
        {
            let failure = ModelLaneRecoveryFailureKind::MissingPayloadAuthority;
            return Err(ModelLaneError::InvalidInput(format!(
                "{} {} payload_ref {payload_ref} hash mismatch: message payload_sha256 {} does not match ArtifactStore content_hash {} and artifact_sha256 {}",
                failure.code(),
                failure.as_str(),
                payload_sha256,
                artifact_record.content_hash,
                artifact_record.artifact_sha256
            )));
        }
    }
    Ok(())
}

fn model_lane_message_has_crdt_authority(message: &NewModelLaneMessage) -> bool {
    message.crdt_update_ref.is_some()
        || message.crdt_base_snapshot_ref.is_some()
        || message.crdt_state_vector.is_some()
        || message.crdt_proposal_ref.is_some()
        || message.crdt_stale_base_ref.is_some()
}

/// Validate one stored CRDT-bearing message from all three immutable roots:
/// the current projection row, its exact MODEL_RESPONSE_RECORDED EventLedger
/// payload, and the historical lease authority captured at admission.
async fn validate_stored_crdt_message_authority_tx(
    tx: &mut Transaction<'_, Postgres>,
    message: &ModelLaneMessageRecord,
) -> ModelLaneResult<Option<ResolvedModelLaneCrdtAuthority>> {
    if !model_lane_message_has_crdt_authority(&message.inner) {
        if message.crdt_authority_binding.is_some() {
            return Err(crdt_authority_denied(format!(
                "non-CRDT message {} carries a CRDT authority binding",
                message.message_id
            )));
        }
        return Ok(None);
    }

    let resolved = validate_message_crdt_authority_tx(tx, &message.inner)
        .await?
        .ok_or_else(|| {
            crdt_authority_denied(format!(
                "stored CRDT-bearing message {} resolved no authority",
                message.message_id
            ))
        })?;
    knowledge_crdt::lock_crdt_lease_authority_domain_tx(
        tx,
        &resolved.workspace_id,
        &resolved.crdt_document_id,
    )
    .await?;
    let lane = lane_by_id_for_run_tx(tx, &message.run_id, &message.from_lane_id).await?;
    validate_model_lane_authority_tx(tx, &lane).await?;
    validate_crdt_lane_session_uniqueness_tx(tx, &lane, &resolved).await?;
    let stored_binding = message.crdt_authority_binding.as_ref().ok_or_else(|| {
        crdt_authority_denied(format!(
            "stored CRDT-bearing message {} has no persisted lease authority binding",
            message.message_id
        ))
    })?;
    let lease =
        validate_historical_crdt_actor_lane_lease_tx(tx, &lane, &resolved, stored_binding).await?;
    let recomputed_binding = bind_crdt_authority_to_lane(&message.inner, &lane, &resolved, &lease)?;
    if stored_binding != &recomputed_binding {
        return Err(crdt_authority_denied(format!(
            "stored message {} crdt_authority_binding does not match recomputed PostgreSQL authority",
            message.message_id
        )));
    }

    let row = sqlx::query(
        r#"
        SELECT event_type, aggregate_type, aggregate_id, event_sequence,
               session_run_id, payload
        FROM kernel_event_ledger
        WHERE event_id = $1
        "#,
    )
    .bind(&message.event_ledger_event_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| {
        crdt_authority_denied(format!(
            "stored CRDT message {} references missing EventLedger event {}",
            message.message_id, message.event_ledger_event_id
        ))
    })?;
    let event_type: String = row.try_get("event_type")?;
    let aggregate_type: String = row.try_get("aggregate_type")?;
    let aggregate_id: String = row.try_get("aggregate_id")?;
    let event_sequence: i64 = row.try_get("event_sequence")?;
    let session_run_id: String = row.try_get("session_run_id")?;
    let payload: Value = row.try_get("payload")?;
    let ledger_message: NewModelLaneMessage = payload
        .get("record")
        .cloned()
        .ok_or_else(|| {
            crdt_authority_denied(format!(
                "MODEL_RESPONSE_RECORDED EventLedger payload for {} has no record",
                message.message_id
            ))
        })
        .and_then(|record| serde_json::from_value(record).map_err(ModelLaneError::from))?;
    let ledger_binding: ModelLaneCrdtAuthorityBinding = payload
        .get("crdt_authority_binding")
        .cloned()
        .ok_or_else(|| {
            crdt_authority_denied(format!(
                "MODEL_RESPONSE_RECORDED EventLedger payload for {} has no crdt_authority_binding",
                message.message_id
            ))
        })
        .and_then(|binding| serde_json::from_value(binding).map_err(ModelLaneError::from))?;
    if event_type != "MODEL_RESPONSE_RECORDED"
        || aggregate_type != "model_lane_message"
        || aggregate_id != message.message_id
        || event_sequence != message.event_ledger_seq
        || session_run_id != message.event_ledger_stream_id
        || ledger_message != message.inner
        || &ledger_binding != stored_binding
    {
        return Err(crdt_authority_denied(format!(
            "stored CRDT message {} projection does not equal its MODEL_RESPONSE_RECORDED EventLedger authority",
            message.message_id
        )));
    }
    Ok(Some(resolved))
}

/// Reconcile every durable message projection against the immutable
/// MODEL_RESPONSE_RECORDED event. CRDT messages retain their additional lease
/// validation, while non-CRDT and promoted messages can no longer bypass the
/// EventLedger authority check merely because they carry no CRDT references.
async fn validate_stored_message_eventledger_authority_tx(
    tx: &mut Transaction<'_, Postgres>,
    message: &ModelLaneMessageRecord,
    expected_exact_scope: Option<&ExactResourceScopeAttribution>,
) -> ModelLaneResult<Option<ResolvedModelLaneCrdtAuthority>> {
    let crdt_authority = validate_stored_crdt_message_authority_tx(tx, message).await?;

    let row = sqlx::query(
        r#"
        SELECT event_type, aggregate_type, aggregate_id, event_sequence,
               session_run_id, payload, payload_hash
        FROM kernel_event_ledger
        WHERE event_id = $1
        "#,
    )
    .bind(&message.event_ledger_event_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| {
        ModelLaneError::AuthorityDenied(
            "ModelLaneMessage projection does not equal its EventLedger authority".into(),
        )
    })?;
    let event_type: String = row.try_get("event_type")?;
    let aggregate_type: String = row.try_get("aggregate_type")?;
    let aggregate_id: String = row.try_get("aggregate_id")?;
    let event_sequence: i64 = row.try_get("event_sequence")?;
    let session_run_id: String = row.try_get("session_run_id")?;
    let payload: Value = row.try_get("payload")?;
    let payload_hash: String = row.try_get("payload_hash")?;

    let mut expected_payload = json!({
        "schema_id": "hsk.model_lane_message@1",
        "dexterity_kernel": "Dexterity",
        "record": &message.inner,
        "crdt_authority_binding": &message.crdt_authority_binding,
    });
    if let Some(exact_scope) = expected_exact_scope {
        exact_scope
            .stamp_json_object(&mut expected_payload)
            .map_err(|_| {
                ModelLaneError::AuthorityDenied(
                    "ModelLaneMessage EventLedger authority requires complete resource attribution"
                        .into(),
                )
            })?;
    } else if message.authority == ModelLaneAuthority::Promoted {
        return Err(ModelLaneError::AuthorityDenied(
            "Promoted ModelLaneMessage EventLedger authority requires complete resource attribution"
                .into(),
        ));
    }
    let expected_payload_hash = dexterity_sha256_hex(canonical_json_bytes(&payload));
    if event_type != "MODEL_RESPONSE_RECORDED"
        || aggregate_type != "model_lane_message"
        || aggregate_id != message.message_id
        || event_sequence != message.event_ledger_seq
        || event_sequence != message.event_stream_version
        || event_sequence != message.transaction_seq
        || session_run_id != message.event_ledger_stream_id
        || payload != expected_payload
        || payload_hash != expected_payload_hash
    {
        return Err(ModelLaneError::AuthorityDenied(format!(
            "ModelLaneMessage {} projection does not equal its EventLedger authority",
            message.message_id
        )));
    }
    Ok(crdt_authority)
}

async fn validate_stored_message_eventledger_authority_for_write_scope_tx(
    tx: &mut Transaction<'_, Postgres>,
    scope: ScopeColumnValues<'_>,
    message: &ModelLaneMessageRecord,
) -> ModelLaneResult<()> {
    let exact_scope = exact_resource_scope_from_columns(scope, "ModelLaneMessage")?;
    if message.authority == ModelLaneAuthority::Promoted && exact_scope.is_none() {
        return Err(ModelLaneError::AuthorityDenied(
            "Promoted ModelLaneMessage EventLedger authority requires complete resource attribution"
                .into(),
        ));
    }
    validate_stored_message_eventledger_authority_tx(tx, message, exact_scope.as_ref())
        .await
        .map(|_| ())
}

async fn validate_stored_run_eventledger_authority_tx(
    tx: &mut Transaction<'_, Postgres>,
    run: &ModelLaneRunRecord,
    expected_exact_scope: Option<&ExactResourceScopeAttribution>,
) -> ModelLaneResult<()> {
    let row = sqlx::query(
        "SELECT event_type, aggregate_type, aggregate_id, event_sequence, \
         session_run_id, payload, payload_hash FROM kernel_event_ledger WHERE event_id = $1",
    )
    .bind(&run.event_ledger_event_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| {
        ModelLaneError::AuthorityDenied(
            "ModelLaneRun projection does not equal its EventLedger authority".into(),
        )
    })?;
    let payload: Value = row.try_get("payload")?;
    let schema_id = payload.get("schema_id").and_then(Value::as_str);
    let mut expected_payload = match schema_id {
        Some("hsk.model_lane_run@1") => json!({
            "schema_id": "hsk.model_lane_run@1",
            "dexterity_kernel": "Dexterity",
            "record": &run.inner,
        }),
        Some("hsk.model_lane_run_extension@1") => {
            let claimed_attached_lane_id = payload
                .get("attached_lane_id")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    ModelLaneError::AuthorityDenied(
                        "ModelLaneRun extension projection does not equal its EventLedger authority"
                            .into(),
                    )
                })?;
            let previous = sqlx::query(
                "SELECT event_type, aggregate_type, aggregate_id, event_sequence, \
                        session_run_id, payload, payload_hash \
                 FROM kernel_event_ledger \
                 WHERE aggregate_type = 'model_lane_run' AND aggregate_id = $1 \
                   AND event_sequence < $2 ORDER BY event_sequence DESC LIMIT 1",
            )
            .bind(&run.run_id)
            .bind(run.event_ledger_seq)
            .fetch_optional(&mut **tx)
            .await?
            .ok_or_else(|| {
                ModelLaneError::AuthorityDenied(
                    "ModelLaneRun extension has no preceding EventLedger authority".into(),
                )
            })?;
            let previous_payload: Value = previous.try_get("payload")?;
            if previous.try_get::<String, _>("payload_hash")?
                != dexterity_sha256_hex(canonical_json_bytes(&previous_payload))
            {
                return Err(ModelLaneError::AuthorityDenied(
                    "ModelLaneRun preceding EventLedger authority hash mismatch".into(),
                ));
            }
            let previous_inner: NewModelLaneRun = serde_json::from_value(
                previous_payload.get("record").cloned().ok_or_else(|| {
                    ModelLaneError::AuthorityDenied(
                        "ModelLaneRun preceding EventLedger authority has no record".into(),
                    )
                })?,
            )?;
            let mut expected_previous_payload =
                match previous_payload.get("schema_id").and_then(Value::as_str) {
                    Some("hsk.model_lane_run@1") => json!({
                        "schema_id": "hsk.model_lane_run@1",
                        "dexterity_kernel": "Dexterity",
                        "record": &previous_inner,
                    }),
                    Some("hsk.model_lane_run_extension@1") => {
                        let previous_attached_lane_id = previous_payload
                            .get("attached_lane_id")
                            .and_then(Value::as_str)
                            .ok_or_else(|| {
                                ModelLaneError::AuthorityDenied(
                                "ModelLaneRun preceding extension authority has no attached lane"
                                    .into(),
                            )
                            })?;
                        json!({
                            "schema_id": "hsk.model_lane_run_extension@1",
                            "dexterity_kernel": "Dexterity",
                            "run_id": run.run_id,
                            "attached_lane_id": previous_attached_lane_id,
                            "record": &previous_inner,
                        })
                    }
                    _ => {
                        return Err(ModelLaneError::AuthorityDenied(
                        "ModelLaneRun preceding EventLedger authority has an unsupported schema"
                            .into(),
                    ));
                    }
                };
            if let Some(exact_scope) = expected_exact_scope {
                exact_scope
                    .stamp_json_object(&mut expected_previous_payload)
                    .map_err(|_| {
                        ModelLaneError::AuthorityDenied(
                            "ModelLaneRun preceding EventLedger authority requires complete resource attribution"
                                .into(),
                        )
                    })?;
            }
            if previous.try_get::<String, _>("event_type")? != "SESSION_STARTED"
                || previous.try_get::<String, _>("aggregate_type")? != "model_lane_run"
                || previous.try_get::<String, _>("aggregate_id")? != run.run_id
                || previous.try_get::<String, _>("session_run_id")? != run.event_ledger_stream_id
                || previous_payload != expected_previous_payload
            {
                return Err(ModelLaneError::AuthorityDenied(
                    "ModelLaneRun preceding EventLedger authority is not canonical".into(),
                ));
            }
            let previous_lane_ids: BTreeSet<&str> =
                previous_inner.lane_ids.iter().map(String::as_str).collect();
            let current_lane_ids: BTreeSet<&str> =
                run.lane_ids.iter().map(String::as_str).collect();
            let added_lanes = current_lane_ids
                .difference(&previous_lane_ids)
                .copied()
                .collect::<Vec<_>>();
            if added_lanes.as_slice() != [claimed_attached_lane_id]
                || !previous_lane_ids.is_subset(&current_lane_ids)
            {
                return Err(ModelLaneError::AuthorityDenied(
                    "ModelLaneRun extension attached lane does not equal the independently reconstructed delta"
                        .into(),
                ));
            }
            let previous_models: BTreeSet<&str> = previous_inner
                .candidate_model_ids
                .iter()
                .map(String::as_str)
                .collect();
            let current_models: BTreeSet<&str> =
                run.candidate_model_ids.iter().map(String::as_str).collect();
            let mut reconstructed_previous = run.inner.clone();
            reconstructed_previous.lane_ids = previous_inner.lane_ids.clone();
            reconstructed_previous.candidate_model_ids = previous_inner.candidate_model_ids.clone();
            reconstructed_previous.projection_plan_ref = previous_inner.projection_plan_ref.clone();
            reconstructed_previous.consent_receipt_ref = previous_inner.consent_receipt_ref.clone();
            if reconstructed_previous != previous_inner
                || !previous_models.is_subset(&current_models)
                || (previous_inner.projection_plan_ref.is_some()
                    && previous_inner.projection_plan_ref != run.projection_plan_ref)
                || (previous_inner.consent_receipt_ref.is_some()
                    && previous_inner.consent_receipt_ref != run.consent_receipt_ref)
            {
                return Err(ModelLaneError::AuthorityDenied(
                    "ModelLaneRun extension changed fields outside the permitted merge delta"
                        .into(),
                ));
            }
            json!({
                "schema_id": "hsk.model_lane_run_extension@1",
                "dexterity_kernel": "Dexterity",
                "run_id": run.run_id,
                "attached_lane_id": claimed_attached_lane_id,
                "record": &run.inner,
            })
        }
        _ => {
            return Err(ModelLaneError::AuthorityDenied(
                "ModelLaneRun projection does not equal its EventLedger authority".into(),
            ));
        }
    };
    if let Some(exact_scope) = expected_exact_scope {
        exact_scope
            .stamp_json_object(&mut expected_payload)
            .map_err(|_| {
                ModelLaneError::AuthorityDenied(
                    "ModelLaneRun EventLedger authority requires complete resource attribution"
                        .into(),
                )
            })?;
    }
    let valid = row.try_get::<String, _>("event_type")? == "SESSION_STARTED"
        && row.try_get::<String, _>("aggregate_type")? == "model_lane_run"
        && row.try_get::<String, _>("aggregate_id")? == run.run_id
        && row.try_get::<i64, _>("event_sequence")? == run.event_ledger_seq
        && row.try_get::<String, _>("session_run_id")? == run.event_ledger_stream_id
        && payload == expected_payload
        && row.try_get::<String, _>("payload_hash")?
            == dexterity_sha256_hex(canonical_json_bytes(&payload));
    if !valid {
        return Err(ModelLaneError::AuthorityDenied(format!(
            "ModelLaneRun {} projection does not equal its EventLedger authority",
            run.run_id
        )));
    }
    Ok(())
}

async fn validate_stored_lane_eventledger_authority_tx(
    tx: &mut Transaction<'_, Postgres>,
    lane: &ModelLaneRecord,
    expected_exact_scope: Option<&ExactResourceScopeAttribution>,
) -> ModelLaneResult<()> {
    let row = sqlx::query(
        "SELECT event.event_type, event.aggregate_type, event.aggregate_id, \
         event.event_sequence, event.session_run_id, event.payload, event.payload_hash, \
         lane.model_stable_anchor \
         FROM kernel_event_ledger event \
         JOIN model_lanes lane ON lane.lane_id = $2 \
         WHERE event.event_id = $1",
    )
    .bind(&lane.event_ledger_event_id)
    .bind(&lane.lane_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| {
        ModelLaneError::AuthorityDenied(
            "ModelLane projection does not equal its EventLedger authority".into(),
        )
    })?;
    let event_type: String = row.try_get("event_type")?;
    let aggregate_type: String = row.try_get("aggregate_type")?;
    let payload: Value = row.try_get("payload")?;
    let payload_hash: String = row.try_get("payload_hash")?;
    let model_stable_anchor: Option<String> = row.try_get("model_stable_anchor")?;
    let mut expected_payload = if aggregate_type == "model_lane" {
        if event_type != "MODEL_ADAPTER_INVOKED" {
            return Err(ModelLaneError::AuthorityDenied(
                "ModelLane projection does not equal its EventLedger authority".into(),
            ));
        }
        json!({
            "schema_id": "hsk.model_lane@1",
            "dexterity_kernel": "Dexterity",
            "model_stable_anchor": model_stable_anchor,
            "record": &lane.inner,
        })
    } else if aggregate_type == "model_lane_terminal" {
        let expected_event_type =
            match &lane.status {
                ModelLaneStatus::Completed => "SESSION_COMPLETED",
                ModelLaneStatus::Failed => "SESSION_FAILED",
                ModelLaneStatus::Cancelled => "SESSION_CANCELLED",
                _ => return Err(ModelLaneError::AuthorityDenied(
                    "non-terminal ModelLane projection points at terminal EventLedger authority"
                        .into(),
                )),
            };
        if event_type != expected_event_type {
            return Err(ModelLaneError::AuthorityDenied(
                "ModelLane terminal projection has the wrong EventLedger event type".into(),
            ));
        }
        let reason = payload
            .get("reason")
            .and_then(Value::as_str)
            .filter(|reason| !reason.trim().is_empty())
            .ok_or_else(|| {
                ModelLaneError::AuthorityDenied(
                    "ModelLane terminal EventLedger authority has no reason".into(),
                )
            })?;
        let previous_event_id = payload
            .get("previous_event_ledger_event_id")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                ModelLaneError::AuthorityDenied(
                    "ModelLane terminal EventLedger authority has no predecessor".into(),
                )
            })?;
        let previous_event_seq = payload
            .get("previous_event_ledger_seq")
            .and_then(Value::as_i64)
            .filter(|seq| *seq > 0)
            .ok_or_else(|| {
                ModelLaneError::AuthorityDenied(
                    "ModelLane terminal EventLedger authority has no predecessor sequence".into(),
                )
            })?;
        validate_terminal_lane_predecessor_tx(
            tx,
            lane,
            previous_event_id,
            previous_event_seq,
            model_stable_anchor.as_deref(),
            expected_exact_scope,
        )
        .await?;

        if lane.failstate_code.as_deref() == Some("CX-MM-007") {
            let provider_call_cancelled = payload
                .get("provider_call_cancelled")
                .and_then(Value::as_bool)
                .ok_or_else(|| {
                    ModelLaneError::AuthorityDenied(
                        "CX-MM-007 terminal authority has no provider cancellation outcome".into(),
                    )
                })?;
            let provider_cancel_outcome = if provider_call_cancelled {
                "cancelled_by_coordinator"
            } else {
                "not_live_at_revocation"
            };
            json!({
                "schema_id": "hsk.model_lane_terminal@1",
                "dexterity_kernel": "Dexterity",
                "lane_id": &lane.lane_id,
                "run_id": &lane.run_id,
                "status": "cancelled",
                "reason": reason,
                "reason_code": "CX-MM-007",
                "consent_status": "CX-MM-007",
                "consent_receipt_id": lane.consent_receipt_ref.as_deref().ok_or_else(|| ModelLaneError::AuthorityDenied("CX-MM-007 terminal authority has no consent receipt".into()))?,
                "projection_plan_id": lane.projection_plan_ref.as_deref().ok_or_else(|| ModelLaneError::AuthorityDenied("CX-MM-007 terminal authority has no projection plan".into()))?,
                "provider_call_cancelled": provider_call_cancelled,
                "provider_cancel_outcome": provider_cancel_outcome,
                "flight_recorder": "EventLedger",
                "previous_event_ledger_event_id": previous_event_id,
                "previous_event_ledger_seq": previous_event_seq,
                "record": &lane.inner,
            })
        } else {
            json!({
                "schema_id": "hsk.model_lane_terminal@1",
                "dexterity_kernel": "Dexterity",
                "lane_id": &lane.lane_id,
                "run_id": &lane.run_id,
                "status": lane.status.as_str(),
                "reason": reason,
                "previous_event_ledger_event_id": previous_event_id,
                "previous_event_ledger_seq": previous_event_seq,
                "record": &lane.inner,
            })
        }
    } else {
        return Err(ModelLaneError::AuthorityDenied(
            "ModelLane projection has unsupported EventLedger aggregate authority".into(),
        ));
    };
    if let Some(exact_scope) = expected_exact_scope {
        exact_scope
            .stamp_json_object(&mut expected_payload)
            .map_err(|_| {
                ModelLaneError::AuthorityDenied(
                    "ModelLane EventLedger authority requires complete resource attribution".into(),
                )
            })?;
    }
    let valid = row.try_get::<String, _>("aggregate_id")? == lane.lane_id
        && row.try_get::<i64, _>("event_sequence")? == lane.event_ledger_seq
        && row.try_get::<String, _>("session_run_id")? == lane.event_ledger_stream_id
        && payload == expected_payload
        && payload_hash == dexterity_sha256_hex(canonical_json_bytes(&payload));
    if !valid {
        return Err(ModelLaneError::AuthorityDenied(format!(
            "ModelLane {} projection does not equal its EventLedger authority",
            lane.lane_id
        )));
    }
    Ok(())
}

async fn validate_terminal_lane_predecessor_tx(
    tx: &mut Transaction<'_, Postgres>,
    terminal_lane: &ModelLaneRecord,
    previous_event_id: &str,
    previous_event_seq: i64,
    model_stable_anchor: Option<&str>,
    expected_exact_scope: Option<&ExactResourceScopeAttribution>,
) -> ModelLaneResult<()> {
    let row = sqlx::query(
        "SELECT event_type, aggregate_type, aggregate_id, event_sequence, session_run_id, \
                payload, payload_hash FROM kernel_event_ledger WHERE event_id = $1",
    )
    .bind(previous_event_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| {
        ModelLaneError::AuthorityDenied(
            "ModelLane terminal EventLedger predecessor is missing".into(),
        )
    })?;
    let payload: Value = row.try_get("payload")?;
    let previous_lane: NewModelLane =
        event_payload_record(&payload, "model_lane", &terminal_lane.lane_id)?;
    let mut expected_payload = json!({
        "schema_id": "hsk.model_lane@1",
        "dexterity_kernel": "Dexterity",
        "model_stable_anchor": model_stable_anchor,
        "record": &previous_lane,
    });
    if let Some(exact_scope) = expected_exact_scope {
        exact_scope
            .stamp_json_object(&mut expected_payload)
            .map_err(|_| {
                ModelLaneError::AuthorityDenied(
                    "ModelLane predecessor authority requires complete resource attribution".into(),
                )
            })?;
    }
    let valid = row.try_get::<String, _>("event_type")? == "MODEL_ADAPTER_INVOKED"
        && row.try_get::<String, _>("aggregate_type")? == "model_lane"
        && row.try_get::<String, _>("aggregate_id")? == terminal_lane.lane_id
        && row.try_get::<i64, _>("event_sequence")? == previous_event_seq
        && row.try_get::<String, _>("session_run_id")? == terminal_lane.event_ledger_stream_id
        && previous_lane.lane_id == terminal_lane.lane_id
        && previous_lane.run_id == terminal_lane.run_id
        && previous_lane.event_ledger_stream_id == terminal_lane.event_ledger_stream_id
        && !matches!(
            previous_lane.status,
            ModelLaneStatus::Completed | ModelLaneStatus::Failed | ModelLaneStatus::Cancelled
        )
        && payload == expected_payload
        && row.try_get::<String, _>("payload_hash")?
            == dexterity_sha256_hex(canonical_json_bytes(&payload));
    if !valid {
        return Err(ModelLaneError::AuthorityDenied(
            "ModelLane terminal EventLedger predecessor is not canonical".into(),
        ));
    }
    Ok(())
}

async fn validate_replay_message_crdt_posture(
    tx: &mut Transaction<'_, Postgres>,
    messages: &[ModelLaneMessageRecord],
) -> ModelLaneResult<()> {
    for message in messages {
        let has_crdt_ref = model_lane_message_has_crdt_authority(&message.inner);
        if !has_crdt_ref {
            continue;
        }
        if message.crdt_stale_base_ref.is_some()
            || message.crdt_base_snapshot_ref.is_none()
            || message.crdt_state_vector.is_none()
        {
            let failure = ModelLaneRecoveryFailureKind::StaleCrdtBase;
            return Err(ModelLaneError::InvalidInput(format!(
                "{} {} replayed message_id {} cannot be recovered against a stale or missing CRDT base",
                failure.code(),
                failure.as_str(),
                message.message_id
            )));
        }
        validate_stored_crdt_message_authority_tx(tx, message).await?;
    }
    Ok(())
}

async fn validate_recovery_crdt_posture(
    tx: &mut Transaction<'_, Postgres>,
    run_id: &str,
    checkpoint: &ModelLaneRecoveryCheckpointRecord,
    recovery_bound_event_ledger_seq: i64,
    events: &[ModelLaneRecoveryEventRecord],
) -> ModelLaneResult<()> {
    for event in events {
        if event.crdt_stale_base_ref.is_some()
            || (event.event_kind == ModelLaneRecoveryEventKind::CrdtUpdateObserved
                && (event.crdt_base_snapshot_ref.is_none() || event.crdt_state_vector.is_none()))
        {
            let failure = ModelLaneRecoveryFailureKind::StaleCrdtBase;
            return Err(ModelLaneError::InvalidInput(format!(
                "{} {} recovery_event_id {} cannot be replayed against a stale or missing CRDT base",
                failure.code(),
                failure.as_str(),
                event.recovery_event_id
            )));
        }
        if event.event_kind == ModelLaneRecoveryEventKind::CrdtUpdateObserved {
            let row = sqlx::query(
                r#"
                SELECT messages.record_json AS message_record_json,
                       ledger.payload AS ledger_payload
                FROM model_lane_messages messages
                JOIN kernel_event_ledger ledger
                  ON ledger.event_id = messages.event_ledger_event_id
                 AND ledger.event_sequence = messages.event_ledger_seq
                 AND ledger.aggregate_type = 'model_lane_message'
                WHERE messages.run_id = $1
                  AND ($2::text IS NULL OR messages.from_lane_id = $2)
                  AND messages.record_json->>'crdt_base_snapshot_ref' = $3
                  AND messages.record_json->>'crdt_state_vector' = $4
                  AND messages.record_json->>'crdt_stale_base_ref' IS NULL
                  AND messages.event_ledger_stream_id = $5
                  AND messages.event_ledger_seq <= $6
                  AND ledger.session_run_id = $5
                ORDER BY messages.event_ledger_seq DESC
                LIMIT 1
                "#,
            )
            .bind(run_id)
            .bind(event.lane_id.as_deref())
            .bind(event.crdt_base_snapshot_ref.as_deref())
            .bind(event.crdt_state_vector.as_deref())
            .bind(&checkpoint.event_ledger_stream_id)
            .bind(recovery_bound_event_ledger_seq)
            .fetch_optional(&mut **tx)
            .await?;
            let Some(row) = row else {
                let failure = ModelLaneRecoveryFailureKind::StaleCrdtBase;
                return Err(ModelLaneError::InvalidInput(format!(
                    "{} {} recovery_event_id {} does not match any recovery-bounded non-stale ModelLaneMessage CRDT base/state vector",
                    failure.code(),
                    failure.as_str(),
                    event.recovery_event_id
                )));
            };
            let message_record_json: Value = row.try_get("message_record_json")?;
            let message_record: ModelLaneMessageRecord =
                serde_json::from_value(message_record_json)?;
            let _: Value = row.try_get("ledger_payload")?;
            validate_stored_crdt_message_authority_tx(tx, &message_record).await?;
        }
    }
    Ok(())
}

async fn validate_recovery_checkpoint_record(
    pool: &PgPool,
    checkpoint: &ModelLaneRecoveryCheckpointRecord,
) -> ModelLaneResult<()> {
    if checkpoint.last_event_ledger_seq <= 0 {
        let failure = ModelLaneRecoveryFailureKind::CorruptCheckpoint;
        return Err(ModelLaneError::InvalidInput(format!(
            "{} {} checkpoint {} has non-positive last_event_ledger_seq",
            failure.code(),
            failure.as_str(),
            checkpoint.checkpoint_id
        )));
    }
    if checkpoint.last_event_ledger_seq > checkpoint.event_ledger_seq {
        let failure = ModelLaneRecoveryFailureKind::CorruptCheckpoint;
        return Err(ModelLaneError::InvalidInput(format!(
            "{} {} checkpoint {} high-watermark is after its checkpoint event",
            failure.code(),
            failure.as_str(),
            checkpoint.checkpoint_id
        )));
    }
    let stream: Option<String> = sqlx::query_scalar(
        "SELECT session_run_id FROM kernel_event_ledger WHERE event_sequence = $1",
    )
    .bind(checkpoint.last_event_ledger_seq)
    .fetch_optional(pool)
    .await?;
    if stream.as_deref() != Some(checkpoint.event_ledger_stream_id.as_str()) {
        let failure = ModelLaneRecoveryFailureKind::MissingEventLedgerRow;
        return Err(ModelLaneError::InvalidInput(format!(
            "{} {} checkpoint {} high-watermark {} is missing or cross-stream",
            failure.code(),
            failure.as_str(),
            checkpoint.checkpoint_id,
            checkpoint.last_event_ledger_seq
        )));
    }
    let row = sqlx::query(
        r#"
        SELECT event_sequence, session_run_id, payload
        FROM kernel_event_ledger
        WHERE event_id = $1
          AND aggregate_type = 'model_lane_recovery_checkpoint'
          AND aggregate_id = $2
        "#,
    )
    .bind(&checkpoint.event_ledger_event_id)
    .bind(&checkpoint.checkpoint_id)
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else {
        let failure = ModelLaneRecoveryFailureKind::MissingEventLedgerRow;
        return Err(ModelLaneError::InvalidInput(format!(
            "{} {} checkpoint {} is not backed by matching kernel_event_ledger row",
            failure.code(),
            failure.as_str(),
            checkpoint.checkpoint_id
        )));
    };
    let ledger_seq: i64 = row.try_get("event_sequence")?;
    let session_run_id: String = row.try_get("session_run_id")?;
    if ledger_seq != checkpoint.event_ledger_seq
        || session_run_id != checkpoint.event_ledger_stream_id
    {
        let failure = ModelLaneRecoveryFailureKind::MissingEventLedgerRow;
        return Err(ModelLaneError::InvalidInput(format!(
            "{} {} checkpoint {} is not backed by matching kernel_event_ledger row",
            failure.code(),
            failure.as_str(),
            checkpoint.checkpoint_id
        )));
    }
    let ledger_payload: Value = row.try_get("payload")?;
    let ledger_record: ModelLaneRecoveryCheckpointRecord = event_payload_record(
        &ledger_payload,
        "model_lane_recovery_checkpoint",
        &checkpoint.checkpoint_id,
    )?;
    if &ledger_record != checkpoint {
        let failure = ModelLaneRecoveryFailureKind::CorruptCheckpoint;
        return Err(ModelLaneError::InvalidInput(format!(
            "{} {} checkpoint {} mutable row differs from EventLedger payload",
            failure.code(),
            failure.as_str(),
            checkpoint.checkpoint_id
        )));
    }
    Ok(())
}

async fn run_by_id_tx(
    tx: &mut Transaction<'_, Postgres>,
    run_id: &str,
) -> ModelLaneResult<ModelLaneRunRecord> {
    sqlx::query("SELECT record_json FROM model_lane_runs WHERE run_id = $1")
        .bind(run_id)
        .fetch_optional(&mut **tx)
        .await?
        .map(|row| {
            row_to_json(row, "record_json")
                .and_then(|v| serde_json::from_value(v).map_err(Into::into))
        })
        .transpose()?
        .ok_or_else(|| ModelLaneError::NotFound(format!("run_id {run_id}")))
}

async fn message_run_by_id_tx(
    tx: &mut Transaction<'_, Postgres>,
    run_id: &str,
    scope: ScopeColumnValues<'_>,
) -> ModelLaneResult<ModelLaneRunRecord> {
    let run = message_record_by_key_for_write_scope_tx(
        tx,
        "model_lane_runs",
        "run_id",
        run_id,
        scope,
        true,
    )
    .await?
    .ok_or_else(|| {
        ModelLaneError::AuthorityDenied("ModelLaneMessage authority unavailable".into())
    })?;
    let exact_scope = exact_resource_scope_from_columns(scope, "ModelLaneRun")?;
    validate_stored_run_eventledger_authority_tx(tx, &run, exact_scope.as_ref())
        .await
        .map_err(|_| {
            ModelLaneError::AuthorityDenied("ModelLaneMessage authority unavailable".into())
        })?;
    Ok(run)
}

async fn context_bundle_run_by_id_tx(
    tx: &mut Transaction<'_, Postgres>,
    access: &ResourceAccessContext,
    run_id: &str,
) -> ModelLaneResult<ModelLaneRunRecord> {
    let predicate = access.sql_predicate(2);
    let sql = format!(
        "SELECT record_json, {RESOURCE_SCOPE_SELECT_COLUMNS} FROM model_lane_runs \
         WHERE run_id = $1{} FOR UPDATE",
        predicate.clause()
    );
    let record = predicate
        .bind(sqlx::query(&sql).bind(run_id))
        .fetch_optional(&mut **tx)
        .await?
        .map(|row| authorize_and_decode_row(access, row))
        .transpose()?
        .ok_or_else(|| ModelLaneError::NotFound("ContextBundle run authority".into()))?;
    validate_stored_run_eventledger_authority_tx(tx, &record, access.exact_read_scope())
        .await
        .map_err(|_| ModelLaneError::AuthorityDenied("ContextBundle run authority".into()))?;
    Ok(record)
}

async fn context_bundle_run_by_id_for_write_scope_tx(
    tx: &mut Transaction<'_, Postgres>,
    run_id: &str,
    scope: ScopeColumnValues<'_>,
) -> ModelLaneResult<ModelLaneRunRecord> {
    let run = context_bundle_record_by_key_for_write_scope_tx(
        tx,
        "model_lane_runs",
        "run_id",
        run_id,
        scope,
        true,
    )
    .await?
    .ok_or_else(|| ModelLaneError::NotFound("ContextBundle run authority".into()))?;
    let exact_scope = promotion_exact_scope_from_columns(scope)?;
    validate_stored_run_eventledger_authority_tx(tx, &run, Some(&exact_scope))
        .await
        .map_err(|_| ModelLaneError::AuthorityDenied("ContextBundle run authority".into()))?;
    Ok(run)
}

async fn lock_idempotency_key_tx(
    tx: &mut Transaction<'_, Postgres>,
    idempotency_key: &str,
) -> ModelLaneResult<()> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1))")
        .bind(idempotency_key)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

fn ensure_idempotent_input_matches<T>(
    entity: &str,
    idempotency_key: &str,
    existing: &T,
    input: &T,
) -> ModelLaneResult<()>
where
    T: Serialize + PartialEq,
{
    if existing == input {
        return Ok(());
    }
    let existing_hash =
        dexterity_sha256_hex(canonical_json_bytes(&serde_json::to_value(existing)?));
    let input_hash = dexterity_sha256_hex(canonical_json_bytes(&serde_json::to_value(input)?));
    Err(ModelLaneError::IdempotencyConflict(format!(
        "{entity} idempotency_key {idempotency_key} already belongs to semantic_hash {existing_hash}, retry supplied {input_hash}"
    )))
}

async fn ensure_event_ledger_sequence_in_stream_tx(
    tx: &mut Transaction<'_, Postgres>,
    event_ledger_seq: i64,
    event_ledger_stream_id: &str,
) -> ModelLaneResult<()> {
    let row = sqlx::query(
        r#"
        SELECT session_run_id
        FROM kernel_event_ledger
        WHERE event_sequence = $1
        "#,
    )
    .bind(event_ledger_seq)
    .fetch_optional(&mut **tx)
    .await?;
    let Some(row) = row else {
        let failure = ModelLaneRecoveryFailureKind::MissingEventLedgerRow;
        return Err(ModelLaneError::InvalidInput(format!(
            "{} {} event_ledger_seq {event_ledger_seq} does not exist",
            failure.code(),
            failure.as_str()
        )));
    };
    let session_run_id: String = row.try_get("session_run_id")?;
    require_equal(
        "event_ledger.session_run_id",
        &session_run_id,
        "record.event_ledger_stream_id",
        event_ledger_stream_id,
    )
}

async fn ensure_exact_event_ledger_high_watermark_tx(
    tx: &mut Transaction<'_, Postgres>,
    event_ledger_seq: i64,
    event_ledger_stream_id: &str,
) -> ModelLaneResult<()> {
    let exact_high_watermark: i64 = sqlx::query_scalar(
        r#"
        SELECT COALESCE(MAX(event_sequence), 0)
        FROM kernel_event_ledger
        WHERE session_run_id = $1
        "#,
    )
    .bind(event_ledger_stream_id)
    .fetch_one(&mut **tx)
    .await?;
    if event_ledger_seq != exact_high_watermark {
        let failure = ModelLaneRecoveryFailureKind::CorruptCheckpoint;
        return Err(ModelLaneError::InvalidInput(format!(
            "{} {} last_event_ledger_seq {event_ledger_seq} is not the exact pre-write stream high-watermark {exact_high_watermark} for {event_ledger_stream_id}",
            failure.code(),
            failure.as_str()
        )));
    }
    ensure_event_ledger_sequence_in_stream_tx(tx, event_ledger_seq, event_ledger_stream_id).await
}

async fn lane_by_id_tx(
    tx: &mut Transaction<'_, Postgres>,
    lane_id: &str,
) -> ModelLaneResult<ModelLaneRecord> {
    sqlx::query("SELECT record_json FROM model_lanes WHERE lane_id = $1 FOR UPDATE")
        .bind(lane_id)
        .fetch_optional(&mut **tx)
        .await?
        .map(|row| {
            row_to_json(row, "record_json")
                .and_then(|v| serde_json::from_value(v).map_err(Into::into))
        })
        .transpose()?
        .ok_or_else(|| ModelLaneError::NotFound(format!("lane_id {lane_id}")))
}

fn require_exact_lifecycle_write_scope(
    store: &ModelLaneStore,
) -> ModelLaneResult<ExactResourceScopeAttribution> {
    store
        .write_scope()
        .ok_or_else(|| {
            ModelLaneError::AuthorityDenied(
                "ModelLane lifecycle mutation requires exact owner, Principal, authenticated session, AccessSpace, and workspace write authority"
                    .into(),
            )
        })
        .and_then(|scope| {
            ExactResourceScopeAttribution::try_from_resource_scope(scope).map_err(|_| {
                ModelLaneError::AuthorityDenied(
                    "ModelLane lifecycle mutation requires exact owner, Principal, authenticated session, AccessSpace, and workspace write authority"
                        .into(),
                )
            })
        })
}

async fn lane_by_access_tx(
    tx: &mut Transaction<'_, Postgres>,
    access: &ResourceAccessContext,
    lane_id: &str,
) -> ModelLaneResult<Option<(ModelLaneRecord, Option<ExactResourceScopeAttribution>)>> {
    let predicate = access.sql_predicate(2);
    let sql = format!(
        "SELECT record_json, {RESOURCE_SCOPE_SELECT_COLUMNS} FROM model_lanes \
         WHERE lane_id = $1{} FOR UPDATE",
        predicate.clause()
    );
    let row = predicate
        .bind(sqlx::query(&sql).bind(lane_id))
        .fetch_optional(&mut **tx)
        .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let stored_scope = stored_resource_scope_from_row(&row)?;
    access.authorize_row(&stored_scope)?;
    let stored_exact_scope = exact_resource_scope_from_stored(&stored_scope, "ModelLane")?;
    let record: ModelLaneRecord = serde_json::from_value(row_to_json(row, "record_json")?)?;
    validate_stored_lane_eventledger_authority_tx(
        tx,
        &record,
        access.exact_read_scope().or(stored_exact_scope.as_ref()),
    )
    .await?;
    Ok(Some((record, stored_exact_scope)))
}

async fn message_lane_by_id_tx(
    tx: &mut Transaction<'_, Postgres>,
    lane_id: &str,
    scope: ScopeColumnValues<'_>,
) -> ModelLaneResult<ModelLaneRecord> {
    let lane = message_record_by_key_for_write_scope_tx(
        tx,
        "model_lanes",
        "lane_id",
        lane_id,
        scope,
        true,
    )
    .await?
    .ok_or_else(|| {
        ModelLaneError::AuthorityDenied("ModelLaneMessage authority unavailable".into())
    })?;
    let exact_scope = exact_resource_scope_from_columns(scope, "ModelLane")?;
    validate_stored_lane_eventledger_authority_tx(tx, &lane, exact_scope.as_ref())
        .await
        .map_err(|_| {
            ModelLaneError::AuthorityDenied("ModelLaneMessage authority unavailable".into())
        })?;
    Ok(lane)
}

async fn message_lane_by_id_for_run_tx(
    tx: &mut Transaction<'_, Postgres>,
    run_id: &str,
    lane_id: &str,
    scope: ScopeColumnValues<'_>,
) -> ModelLaneResult<ModelLaneRecord> {
    let lane = message_lane_by_id_tx(tx, lane_id, scope).await?;
    require_equal("lane.run_id", &lane.run_id, "record.run_id", run_id)?;
    Ok(lane)
}

async fn context_bundle_lane_by_id_tx(
    tx: &mut Transaction<'_, Postgres>,
    access: &ResourceAccessContext,
    lane_id: &str,
) -> ModelLaneResult<ModelLaneRecord> {
    let predicate = access.sql_predicate(2);
    let sql = format!(
        "SELECT record_json, {RESOURCE_SCOPE_SELECT_COLUMNS} FROM model_lanes \
         WHERE lane_id = $1{} FOR UPDATE",
        predicate.clause()
    );
    let record = predicate
        .bind(sqlx::query(&sql).bind(lane_id))
        .fetch_optional(&mut **tx)
        .await?
        .map(|row| authorize_and_decode_row(access, row))
        .transpose()?
        .ok_or_else(|| ModelLaneError::NotFound("ContextBundle lane authority".into()))?;
    validate_stored_lane_eventledger_authority_tx(tx, &record, access.exact_read_scope())
        .await
        .map_err(|_| ModelLaneError::AuthorityDenied("ContextBundle lane authority".into()))?;
    Ok(record)
}

async fn lane_by_id_for_run_tx(
    tx: &mut Transaction<'_, Postgres>,
    run_id: &str,
    lane_id: &str,
) -> ModelLaneResult<ModelLaneRecord> {
    let lane = lane_by_id_tx(tx, lane_id).await?;
    require_equal("lane.run_id", &lane.run_id, "record.run_id", run_id)?;
    Ok(lane)
}

/// A terminal lane is a durable lifecycle boundary, not merely a projection
/// hint.  Once its terminal EventLedger row is committed, no new
/// `ModelLaneMessage` may be appended from or to that lane.  Idempotent
/// retries are resolved before this check in `record_message`, so a retry of a
/// pre-terminal message remains safe and does not reopen the stream.
fn ensure_message_lane_is_live(lane: &ModelLaneRecord, direction: &str) -> ModelLaneResult<()> {
    if matches!(
        lane.status,
        ModelLaneStatus::Completed | ModelLaneStatus::Failed | ModelLaneStatus::Cancelled
    ) {
        return Err(ModelLaneError::InvalidInput(format!(
            "cannot append ModelLaneMessage for terminal {direction} lane {} ({})",
            lane.lane_id,
            lane.status.as_str()
        )));
    }
    Ok(())
}

fn validate_message_payload_binding_pair(
    message: &NewModelLaneMessage,
    binding: &NewModelLaneContextBundleArtifactBinding,
) -> ModelLaneResult<()> {
    require_equal(
        "binding.run_id",
        &binding.run_id,
        "message.run_id",
        &message.run_id,
    )?;
    require_equal(
        "binding.trace_id",
        &binding.trace_id,
        "message.trace_id",
        &message.trace_id,
    )?;
    require_equal(
        "binding.artifact_ref",
        &binding.artifact_ref,
        "message.payload_ref",
        &message.payload_ref,
    )?;
    require_equal(
        "binding.artifact_payload_ref",
        &binding.artifact_payload_ref,
        "message.payload_ref",
        &message.payload_ref,
    )?;
    require_equal(
        "binding.artifact_sha256",
        &binding.artifact_sha256,
        "message.payload_sha256",
        &message.payload_sha256,
    )?;
    require_equal(
        "binding.content_hash",
        &binding.content_hash,
        "message.payload_sha256",
        &message.payload_sha256,
    )?;
    require_equal(
        "binding.event_ledger_stream_id",
        &binding.event_ledger_stream_id,
        "message.event_ledger_stream_id",
        &message.event_ledger_stream_id,
    )?;
    require_equal(
        "binding.owner_session",
        &binding.owner_session,
        "message.owner_session",
        &message.owner_session,
    )?;

    let message_wp = message.work_packet_id.as_deref().ok_or_else(|| {
        ModelLaneError::InvalidInput(
            "message.work_packet_id is required for an atomic payload binding".into(),
        )
    })?;
    let message_mt = message.micro_task_id.as_deref().ok_or_else(|| {
        ModelLaneError::InvalidInput(
            "message.micro_task_id is required for an atomic payload binding".into(),
        )
    })?;
    let message_board = message.task_board_id.as_deref().ok_or_else(|| {
        ModelLaneError::InvalidInput(
            "message.task_board_id is required for an atomic payload binding".into(),
        )
    })?;
    require_equal(
        "binding.work_packet_id",
        &binding.work_packet_id,
        "message.work_packet_id",
        message_wp,
    )?;
    require_equal(
        "binding.micro_task_id",
        &binding.micro_task_id,
        "message.micro_task_id",
        message_mt,
    )?;
    require_equal(
        "binding.task_board_id",
        &binding.task_board_id,
        "message.task_board_id",
        message_board,
    )?;
    Ok(())
}

fn stored_scope_matches_write_columns(
    stored: &super::resource_scope::StoredResourceScope,
    scope: ScopeColumnValues<'_>,
) -> bool {
    stored.owner_account_id.map(|value| value.as_uuid()) == scope.owner_account_id
        && stored.actor_principal_id.map(|value| value.as_uuid()) == scope.actor_principal_id
        && stored.authenticated_session.map(|value| value.as_uuid())
            == scope.authenticated_session_id
        && stored.access_space.map(|value| value.as_uuid()) == scope.access_space_id
        && stored.workspace.as_ref().map(|value| value.as_str()) == scope.workspace_id
}

async fn message_record_by_key_for_write_scope_tx<T>(
    tx: &mut Transaction<'_, Postgres>,
    table: &str,
    key_column: &str,
    key: &str,
    scope: ScopeColumnValues<'_>,
    for_update: bool,
) -> ModelLaneResult<Option<T>>
where
    T: DeserializeOwned,
{
    let lock = if for_update { " FOR UPDATE" } else { "" };
    let sql = format!(
        "SELECT record_json, {RESOURCE_SCOPE_SELECT_COLUMNS} FROM {table} \
         WHERE {key_column} = $1 \
           AND owner_account_id IS NOT DISTINCT FROM $2::uuid \
           AND actor_principal_id IS NOT DISTINCT FROM $3::uuid \
           AND authenticated_session_id IS NOT DISTINCT FROM $4::uuid \
           AND access_space_id IS NOT DISTINCT FROM $5::uuid \
           AND workspace_id IS NOT DISTINCT FROM $6{lock}"
    );
    let row = sqlx::query(&sql)
        .bind(key)
        .bind(scope.owner_account_id)
        .bind(scope.actor_principal_id)
        .bind(scope.authenticated_session_id)
        .bind(scope.access_space_id)
        .bind(scope.workspace_id)
        .fetch_optional(&mut **tx)
        .await?;
    row.map(|row| {
        let stored = stored_resource_scope_from_row(&row)?;
        if !stored_scope_matches_write_columns(&stored, scope) {
            return Err(ModelLaneError::AuthorityDenied(
                "ModelLaneMessage authority unavailable".into(),
            ));
        }
        row_to_json(row, "record_json")
            .and_then(|value| serde_json::from_value(value).map_err(Into::into))
    })
    .transpose()
}

async fn require_message_physical_keys_authorized_tx(
    tx: &mut Transaction<'_, Postgres>,
    message_id: &str,
    idempotency_key: &str,
    scope: ScopeColumnValues<'_>,
) -> ModelLaneResult<()> {
    let rows = sqlx::query(&format!(
        "SELECT {RESOURCE_SCOPE_SELECT_COLUMNS} FROM model_lane_messages \
         WHERE message_id = $1 OR idempotency_key = $2 FOR UPDATE"
    ))
    .bind(message_id)
    .bind(idempotency_key)
    .fetch_all(&mut **tx)
    .await?;
    if rows.iter().any(|row| {
        stored_resource_scope_from_row(row)
            .map(|stored| !stored_scope_matches_write_columns(&stored, scope))
            .unwrap_or(true)
    }) {
        return Err(ModelLaneError::AuthorityDenied(
            "ModelLaneMessage authority unavailable".into(),
        ));
    }
    Ok(())
}

async fn message_by_idempotency_key_for_write_scope_tx(
    tx: &mut Transaction<'_, Postgres>,
    idempotency_key: &str,
    scope: ScopeColumnValues<'_>,
) -> ModelLaneResult<Option<ModelLaneMessageRecord>> {
    message_record_by_key_for_write_scope_tx(
        tx,
        "model_lane_messages",
        "idempotency_key",
        idempotency_key,
        scope,
        false,
    )
    .await
}

async fn context_bundle_artifact_binding_by_idempotency_key_for_write_scope_tx(
    tx: &mut Transaction<'_, Postgres>,
    idempotency_key: &str,
    scope: ScopeColumnValues<'_>,
) -> ModelLaneResult<Option<ModelLaneContextBundleArtifactBindingRecord>> {
    context_bundle_record_by_key_for_write_scope_tx(
        tx,
        "model_lane_context_bundle_artifacts",
        "idempotency_key",
        idempotency_key,
        scope,
        false,
    )
    .await
}

async fn context_bundle_artifact_binding_by_ref_tx(
    tx: &mut Transaction<'_, Postgres>,
    access: &ResourceAccessContext,
    run_id: &str,
    artifact_ref: &str,
) -> ModelLaneResult<Option<ModelLaneContextBundleArtifactBindingRecord>> {
    require_exact_context_bundle_read_scope(access)?;
    let predicate = access.sql_predicate(3);
    let sql = format!(
        "SELECT record_json, {RESOURCE_SCOPE_SELECT_COLUMNS} \
         FROM model_lane_context_bundle_artifacts \
         WHERE run_id = $1 AND artifact_ref = $2{} FOR UPDATE",
        predicate.clause()
    );
    let record = predicate
        .bind(sqlx::query(&sql).bind(run_id).bind(artifact_ref))
        .fetch_optional(&mut **tx)
        .await?
        .map(|row| authorize_and_decode_row(access, row))
        .transpose()?;
    if let Some(record) = record.as_ref() {
        validate_stored_context_bundle_artifact_authority_tx(tx, access, record).await?;
    }
    Ok(record)
}

async fn promotion_decision_by_idempotency_key_tx(
    tx: &mut Transaction<'_, Postgres>,
    access: &ResourceAccessContext,
    idempotency_key: &str,
) -> ModelLaneResult<Option<ModelLanePromotionDecisionRecord>> {
    let predicate = access.sql_predicate(2);
    let sql = format!(
        "SELECT record_json, {RESOURCE_SCOPE_SELECT_COLUMNS} FROM model_lane_promotion_decisions WHERE idempotency_key = $1{} LIMIT 1",
        predicate.clause()
    );
    let record = predicate
        .bind(sqlx::query(&sql).bind(idempotency_key))
        .fetch_optional(&mut **tx)
        .await?
        .map(|row| authorize_and_decode_row(access, row))
        .transpose()?;
    if let Some(record) = record.as_ref() {
        validate_stored_promotion_decision_authority_tx(tx, access, record).await?;
    }
    Ok(record)
}

async fn require_promotion_physical_keys_authorized_tx(
    tx: &mut Transaction<'_, Postgres>,
    access: &ResourceAccessContext,
    decision_id: &str,
    idempotency_key: &str,
) -> ModelLaneResult<()> {
    let rows = sqlx::query(&format!(
        "SELECT {RESOURCE_SCOPE_SELECT_COLUMNS} FROM model_lane_promotion_decisions \
         WHERE decision_id = $1 OR idempotency_key = $2 FOR UPDATE"
    ))
    .bind(decision_id)
    .bind(idempotency_key)
    .fetch_all(&mut **tx)
    .await?;
    if rows.iter().any(|row| {
        let Ok(stored) = stored_resource_scope_from_row(row) else {
            return true;
        };
        access.authorize_row(&stored).is_err()
    }) {
        return Err(ModelLaneError::AuthorityDenied(
            "PromotionGate decision authority unavailable".into(),
        ));
    }
    Ok(())
}

async fn message_by_id_tx(
    tx: &mut Transaction<'_, Postgres>,
    message_id: &str,
) -> ModelLaneResult<Option<ModelLaneMessageRecord>> {
    sqlx::query("SELECT record_json FROM model_lane_messages WHERE message_id = $1 FOR UPDATE")
        .bind(message_id)
        .fetch_optional(&mut **tx)
        .await?
        .map(|row| {
            row_to_json(row, "record_json")
                .and_then(|v| serde_json::from_value(v).map_err(Into::into))
        })
        .transpose()
}

async fn context_bundle_message_by_id_tx(
    tx: &mut Transaction<'_, Postgres>,
    access: &ResourceAccessContext,
    message_id: &str,
) -> ModelLaneResult<Option<ModelLaneMessageRecord>> {
    let predicate = access.sql_predicate(2);
    let sql = format!(
        "SELECT record_json, {RESOURCE_SCOPE_SELECT_COLUMNS} FROM model_lane_messages \
         WHERE message_id = $1{} FOR UPDATE",
        predicate.clause()
    );
    let record = predicate
        .bind(sqlx::query(&sql).bind(message_id))
        .fetch_optional(&mut **tx)
        .await?
        .map(|row| authorize_and_decode_row(access, row))
        .transpose()?;
    if let Some(record) = record.as_ref() {
        validate_stored_message_eventledger_authority_tx(tx, record, access.exact_read_scope())
            .await
            .map_err(|_| {
                ModelLaneError::AuthorityDenied("ContextBundle message authority".into())
            })?;
    }
    Ok(record)
}

async fn promotion_decision_by_id_tx(
    tx: &mut Transaction<'_, Postgres>,
    access: &ResourceAccessContext,
    decision_id: &str,
) -> ModelLaneResult<Option<ModelLanePromotionDecisionRecord>> {
    let predicate = access.sql_predicate(2);
    let sql = format!(
        "SELECT record_json, {RESOURCE_SCOPE_SELECT_COLUMNS} FROM model_lane_promotion_decisions WHERE decision_id = $1{} LIMIT 1",
        predicate.clause()
    );
    let record = predicate
        .bind(sqlx::query(&sql).bind(decision_id))
        .fetch_optional(&mut **tx)
        .await?
        .map(|row| authorize_and_decode_row(access, row))
        .transpose()?;
    if let Some(record) = record.as_ref() {
        validate_stored_promotion_decision_authority_tx(tx, access, record).await?;
    }
    Ok(record)
}

async fn promotion_decision_by_id_for_write_scope_tx(
    tx: &mut Transaction<'_, Postgres>,
    decision_id: &str,
    scope: ScopeColumnValues<'_>,
) -> ModelLaneResult<Option<ModelLanePromotionDecisionRecord>> {
    if scope.owner_account_id.is_none()
        || scope.actor_principal_id.is_none()
        || scope.authenticated_session_id.is_none()
        || scope.access_space_id.is_none()
        || scope.workspace_id.is_none()
    {
        return Ok(None);
    }
    let row = sqlx::query(&format!(
        "SELECT record_json, {RESOURCE_SCOPE_SELECT_COLUMNS} FROM model_lane_promotion_decisions \
         WHERE decision_id = $1 \
           AND owner_account_id IS NOT DISTINCT FROM $2::uuid \
           AND actor_principal_id IS NOT DISTINCT FROM $3::uuid \
           AND authenticated_session_id IS NOT DISTINCT FROM $4::uuid \
           AND access_space_id IS NOT DISTINCT FROM $5::uuid \
           AND workspace_id IS NOT DISTINCT FROM $6 LIMIT 1"
    ))
    .bind(decision_id)
    .bind(scope.owner_account_id)
    .bind(scope.actor_principal_id)
    .bind(scope.authenticated_session_id)
    .bind(scope.access_space_id)
    .bind(scope.workspace_id)
    .fetch_optional(&mut **tx)
    .await?;
    let record = row
        .map(|row| {
            let stored = stored_resource_scope_from_row(&row)?;
            if stored.owner_account_id.map(|value| value.as_uuid()) != scope.owner_account_id
                || stored.actor_principal_id.map(|value| value.as_uuid())
                    != scope.actor_principal_id
                || stored.authenticated_session.map(|value| value.as_uuid())
                    != scope.authenticated_session_id
                || stored.access_space.map(|value| value.as_uuid()) != scope.access_space_id
                || stored.workspace.as_ref().map(|value| value.as_str()) != scope.workspace_id
            {
                return Err(ModelLaneError::ScopeDenied(
                    ScopeDenied::ExactAttributionMismatch,
                ));
            }
            row_to_json(row, "record_json")
                .and_then(|value| serde_json::from_value(value).map_err(Into::into))
        })
        .transpose()?;
    if let Some(record) = record.as_ref() {
        let access =
            ResourceAccessContext::for_exact_reader(promotion_exact_scope_from_columns(scope)?);
        validate_stored_promotion_decision_authority_tx(tx, &access, record).await?;
    }
    Ok(record)
}

async fn context_bundle_handoff_by_idempotency_key_tx(
    tx: &mut Transaction<'_, Postgres>,
    access: &ResourceAccessContext,
    idempotency_key: &str,
) -> ModelLaneResult<Option<ModelLaneContextBundleHandoffRecord>> {
    let predicate = access.sql_predicate(2);
    let sql = format!(
        "SELECT record_json, {RESOURCE_SCOPE_SELECT_COLUMNS} \
         FROM model_lane_context_bundle_handoffs WHERE idempotency_key = $1{} LIMIT 1",
        predicate.clause()
    );
    predicate
        .bind(sqlx::query(&sql).bind(idempotency_key))
        .fetch_optional(&mut **tx)
        .await?
        .map(|row| authorize_and_decode_row(access, row))
        .transpose()
}

async fn stamp_kernel_event_payload_tx(
    tx: &mut Transaction<'_, Postgres>,
    event_id: &str,
    payload: Value,
) -> ModelLaneResult<()> {
    let payload_hash = dexterity_sha256_hex(canonical_json_bytes(&payload));
    sqlx::query(
        r#"
        UPDATE kernel_event_ledger
        SET payload = $2,
            payload_hash = $3
        WHERE event_id = $1
        "#,
    )
    .bind(event_id)
    .bind(payload)
    .bind(payload_hash)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn ensure_promoted_message_has_decision_tx(
    tx: &mut Transaction<'_, Postgres>,
    input: &NewModelLaneMessage,
    scope: ScopeColumnValues<'_>,
) -> ModelLaneResult<()> {
    let promotion_decision_id =
        require_optional_token("promotion_decision_id", input.promotion_decision_id.as_deref())
            .map_err(|_| {
                ModelLaneError::InvalidInput(
                    "Promoted ModelLaneMessage requires approved PromotionGate resolution: promotion_decision_id is required"
                        .into(),
                )
            })?;
    let promotion_gate_ref =
        require_optional_token("promotion_gate_ref", input.promotion_gate_ref.as_deref())
            .map_err(|_| {
                ModelLaneError::InvalidInput(
                    "Promoted ModelLaneMessage requires approved PromotionGate resolution: promotion_gate_ref is required"
                        .into(),
                )
            })?;
    let promotion_receipt_ref =
        require_optional_token("promotion_receipt_ref", input.promotion_receipt_ref.as_deref())
            .map_err(|_| {
                ModelLaneError::InvalidInput(
                    "Promoted ModelLaneMessage requires approved PromotionGate resolution: promotion_receipt_ref is required"
                        .into(),
                )
            })?;
    let promoted_artifact_ref =
        require_optional_token("promoted_artifact_ref", input.promoted_artifact_ref.as_deref())
            .map_err(|_| {
                ModelLaneError::InvalidInput(
                    "Promoted ModelLaneMessage requires approved PromotionGate resolution: promoted_artifact_ref is required"
                        .into(),
                )
            })?;
    let promoted_artifact_sha256 = require_optional_token(
        "promoted_artifact_sha256",
        input.promoted_artifact_sha256.as_deref(),
    )
    .map_err(|_| {
        ModelLaneError::InvalidInput(
            "Promoted ModelLaneMessage requires approved PromotionGate resolution: promoted_artifact_sha256 is required"
                .into(),
        )
    })?;
    let promoted_artifact_version = require_optional_token(
        "promoted_artifact_version",
        input.promoted_artifact_version.as_deref(),
    )
    .map_err(|_| {
        ModelLaneError::InvalidInput(
            "Promoted ModelLaneMessage requires approved PromotionGate resolution: promoted_artifact_version is required"
                .into(),
        )
    })?;
    let decision = promotion_decision_by_id_for_write_scope_tx(tx, &promotion_decision_id, scope)
        .await?
        .ok_or_else(|| {
            ModelLaneError::InvalidInput(format!(
                "Promoted ModelLaneMessage requires approved PromotionGate resolution for promotion_decision_id {promotion_decision_id}"
            ))
        })?;
    let decision_matches = decision.run_id == input.run_id
        && decision.outcome == ModelLanePromotionOutcome::Approved
        && decision.final_state == ModelLanePromotionState::Executed
        && decision.denial_reason.is_none()
        && decision.promotion_gate_ref == promotion_gate_ref
        && decision.promotion_receipt_ref.as_deref() == Some(promotion_receipt_ref.as_str())
        && decision.promoted_artifact_ref.as_deref() == Some(promoted_artifact_ref.as_str())
        && decision.promoted_artifact_sha256.as_deref() == Some(promoted_artifact_sha256.as_str())
        && decision.promoted_artifact_version.as_deref()
            == Some(promoted_artifact_version.as_str());
    if !decision_matches {
        return Err(ModelLaneError::InvalidInput(format!(
            "Promoted ModelLaneMessage requires exact approved PromotionGate resolution and artifact binding for promotion_decision_id {promotion_decision_id}"
        )));
    }
    let artifact_access =
        ResourceAccessContext::for_exact_reader(promotion_exact_scope_from_columns(scope)?);
    let artifact_binding = context_bundle_artifact_binding_by_ref_tx(
        tx,
        &artifact_access,
        &input.run_id,
        &promoted_artifact_ref,
    )
    .await?
    .ok_or_else(|| {
        ModelLaneError::InvalidInput(format!(
            "Promoted ModelLaneMessage requires ArtifactStore/EventLedger authority for promotion_decision_id {promotion_decision_id}"
        ))
    })?;
    if artifact_binding.artifact_sha256 != promoted_artifact_sha256
        || artifact_binding.content_hash != promoted_artifact_sha256
        || artifact_binding
            .payload_json
            .get("artifact_version")
            .and_then(Value::as_str)
            != Some(promoted_artifact_version.as_str())
    {
        return Err(ModelLaneError::InvalidInput(format!(
            "Promoted ModelLaneMessage requires exact ArtifactStore/EventLedger artifact binding for promotion_decision_id {promotion_decision_id}"
        )));
    }
    Ok(())
}

async fn prepare_context_bundle_handoff_tx(
    tx: &mut Transaction<'_, Postgres>,
    access: &ResourceAccessContext,
    input: NewModelLaneContextBundleHandoff,
) -> ModelLaneResult<ModelLaneContextBundleHandoffRecord> {
    context_bundle_run_by_id_tx(tx, access, &input.run_id).await?;
    let downstream_lane =
        context_bundle_lane_by_id_tx(tx, access, &input.downstream_lane_id).await?;
    require_equal(
        "handoff.run_id",
        &input.run_id,
        "downstream.run_id",
        &downstream_lane.run_id,
    )?;
    let source = context_bundle_message_by_id_tx(tx, access, &input.source_message_id)
        .await?
        .ok_or_else(|| {
            ModelLaneError::InvalidInput(format!(
                "source_message_id {} is not replayable",
                input.source_message_id
            ))
        })?;
    validate_stored_message_eventledger_authority_tx(tx, &source, access.exact_read_scope())
        .await?;
    let source_lane = context_bundle_lane_by_id_tx(tx, access, &source.from_lane_id).await?;
    require_equal(
        "handoff.run_id",
        &input.run_id,
        "source_lane.run_id",
        &source_lane.run_id,
    )?;
    require_equal(
        "handoff.run_id",
        &input.run_id,
        "source.run_id",
        &source.run_id,
    )?;
    require_equal(
        "handoff.source_lane_id",
        &input.source_lane_id,
        "source.from_lane_id",
        &source.from_lane_id,
    )?;
    require_equal(
        "handoff.artifact_ref",
        &input.artifact_ref,
        "source.payload_ref",
        &source.payload_ref,
    )?;
    require_equal(
        "handoff.artifact_sha256",
        &input.artifact_sha256,
        "source.payload_sha256",
        &source.payload_sha256,
    )?;
    require_equal(
        "handoff.content_hash",
        &input.content_hash,
        "source.payload_sha256",
        &source.payload_sha256,
    )?;
    let artifact_binding =
        context_bundle_artifact_binding_by_ref_tx(tx, access, &input.run_id, &input.artifact_ref)
            .await?
            .ok_or_else(|| {
                ModelLaneError::InvalidInput(format!(
                    "artifact_ref {} is not backed by ArtifactStore/EventLedger authority",
                    input.artifact_ref
                ))
            })?;
    require_equal(
        "handoff.artifact_sha256",
        &input.artifact_sha256,
        "artifact_binding.artifact_sha256",
        &artifact_binding.artifact_sha256,
    )?;
    require_equal(
        "handoff.content_hash",
        &input.content_hash,
        "artifact_binding.content_hash",
        &artifact_binding.content_hash,
    )?;
    require_equal(
        "handoff.source_kind",
        input.source_kind.as_str(),
        "source.kind",
        ModelLaneHandoffSourceKind::from_message_kind(&source.kind).as_str(),
    )?;
    require_equal(
        "handoff.authority_state",
        input.authority_state.as_str(),
        "source.authority",
        source.authority.as_str(),
    )?;
    let source_has_crdt = source.crdt_proposal_ref.is_some() || source.crdt_update_ref.is_some();
    if !source_has_crdt && input.crdt_payload.is_some() {
        return Err(crdt_authority_denied(
            "non-CRDT ContextBundle source message cannot acquire CRDT authority in handoff metadata",
        ));
    }
    if source_has_crdt {
        let crdt = input.crdt_payload.as_ref().ok_or_else(|| {
            ModelLaneError::InvalidInput(
                "CRDT ModelLaneMessage handoff requires crdt_payload metadata".into(),
            )
        })?;
        let source_authority = validate_message_crdt_authority_tx(tx, &source.inner)
            .await?
            .ok_or_else(|| {
                crdt_authority_denied(
                    "CRDT ContextBundle source message has no canonical update authority",
                )
            })?;
        let handoff_authority = validate_crdt_handoff_authority_tx(tx, crdt).await?;
        if source_authority.update_bytes_ref != handoff_authority.update_bytes_ref
            || source_authority.snapshot_bytes_ref != handoff_authority.snapshot_bytes_ref
            || source_authority.state_vector_after != handoff_authority.state_vector_after
            || source_authority.materialized_projection_hash
                != handoff_authority.materialized_projection_hash
        {
            return Err(crdt_authority_denied(
                "ContextBundle CRDT payload does not resolve to the source message authority",
            ));
        }
        require_equal(
            "crdt_payload.lane_id",
            &crdt.lane_id,
            "handoff.source_lane_id",
            &input.source_lane_id,
        )?;
        let source_binding = source.crdt_authority_binding.as_ref().ok_or_else(|| {
            crdt_authority_denied(
                "CRDT ContextBundle source message is missing its durable lane authority binding",
            )
        })?;
        for (field, actual, expected) in [
            (
                "source binding run_id",
                source_binding.run_id.as_str(),
                input.run_id.as_str(),
            ),
            (
                "source binding lane_id",
                source_binding.lane_id.as_str(),
                input.source_lane_id.as_str(),
            ),
            (
                "source binding update_id",
                source_binding.update_id.as_str(),
                handoff_authority.update_id.as_str(),
            ),
            (
                "source binding CRDT document",
                source_binding.crdt_document_id.as_str(),
                handoff_authority.crdt_document_id.as_str(),
            ),
            (
                "source binding workspace",
                source_binding.workspace_id.as_str(),
                handoff_authority.workspace_id.as_str(),
            ),
            (
                "source binding document",
                source_binding.document_id.as_str(),
                handoff_authority.document_id.as_str(),
            ),
            (
                "source binding actor_id",
                source_binding.actor_id.as_str(),
                handoff_authority.actor_id.as_str(),
            ),
            (
                "source binding actor_kind",
                source_binding.actor_kind.as_str(),
                handoff_authority.actor_kind.as_str(),
            ),
            (
                "source binding CRDT site",
                source_binding.crdt_site_id.as_str(),
                handoff_authority.site_id.as_str(),
            ),
            (
                "source binding CRDT session",
                source_binding.crdt_session_id.as_str(),
                handoff_authority.session_id.as_str(),
            ),
            (
                "source binding CRDT trace",
                source_binding.crdt_trace_id.as_str(),
                handoff_authority.trace_id.as_str(),
            ),
            (
                "source binding update ref",
                source_binding.update_bytes_ref.as_str(),
                handoff_authority.update_bytes_ref.as_str(),
            ),
            (
                "source binding base snapshot ref",
                source_binding.base_snapshot_ref.as_str(),
                handoff_authority.snapshot_bytes_ref.as_str(),
            ),
            (
                "source binding state vector",
                source_binding.state_vector.as_str(),
                handoff_authority.state_vector_after.as_str(),
            ),
            (
                "source binding update EventLedger id",
                source_binding.update_event_ledger_event_id.as_str(),
                handoff_authority.event_ledger_event_id.as_str(),
            ),
            (
                "source binding projection hash",
                source_binding.materialized_projection_hash.as_str(),
                handoff_authority.materialized_projection_hash.as_str(),
            ),
        ] {
            require_equal(field, actual, "resolved CRDT handoff authority", expected)?;
        }
        if source_binding.update_seq != handoff_authority.update_seq {
            return Err(crdt_authority_denied(
                "source binding update_seq does not match resolved CRDT handoff authority",
            ));
        }
        let expected_promotion_gate_ref = format!(
            "promotion-gate://model-lane-message/{}",
            input.source_message_id
        );
        require_equal(
            "crdt_payload.promotion_gate_ref",
            &crdt.promotion_gate_ref,
            "source message promotion gate",
            &expected_promotion_gate_ref,
        )?;
        if let Some(source_state_vector) = source.crdt_state_vector.as_deref() {
            require_equal(
                "crdt_payload.state_vector",
                &crdt.state_vector,
                "source.crdt_state_vector",
                source_state_vector,
            )?;
        }
        if let Some(source_base_snapshot_ref) = source.crdt_base_snapshot_ref.as_deref() {
            require_equal(
                "crdt_payload.base_snapshot_ref",
                &crdt.base_snapshot_ref,
                "source.crdt_base_snapshot_ref",
                source_base_snapshot_ref,
            )?;
        }
        if let Some(source_update_ref) = source.crdt_update_ref.as_deref() {
            require_equal(
                "crdt_payload.update_bytes_ref",
                &crdt.update_bytes_ref,
                "source.crdt_update_ref",
                source_update_ref,
            )?;
        }
    }
    let cloud_downstream = downstream_lane.runtime_binding == RuntimeBinding::Cloud
        || matches!(
            downstream_lane.provider_kind,
            ModelLaneProviderKind::OpenAi | ModelLaneProviderKind::Anthropic
        );
    if cloud_downstream && input.memory_pack_refs.is_empty() {
        return Err(ModelLaneError::InvalidInput(
            "cloud downstream handoff requires explicit cloud_safe MemoryPack refs".into(),
        ));
    }
    if cloud_downstream
        && input
            .memory_pack_refs
            .iter()
            .any(|memory_pack| !memory_pack.cloud_safe)
    {
        return Err(ModelLaneError::InvalidInput(
            "cloud downstream handoff requires every MemoryPack ref to be cloud_safe".into(),
        ));
    }
    if cloud_downstream
        && input
            .memory_pack_refs
            .iter()
            .any(|memory_pack| memory_pack.classification == "local_only_context")
    {
        return Err(ModelLaneError::InvalidInput(
            "cloud downstream handoff cannot use local_only_context MemoryPack refs".into(),
        ));
    }
    let context_bundle_hash = context_bundle_handoff_hash(&input)?;
    Ok(ModelLaneContextBundleHandoffRecord {
        inner: input,
        context_bundle_hash,
        event_ledger_event_id: String::new(),
        event_ledger_seq: 0,
        event_stream_version: 0,
        transaction_seq: 0,
    })
}

/// Rebuild and compare a stored ContextBundle handoff against its source
/// message/lease authority and exact CONTEXT_BUNDLE_RECORDED ledger event.
async fn validate_stored_context_bundle_handoff_authority_tx(
    tx: &mut Transaction<'_, Postgres>,
    access: &ResourceAccessContext,
    record: &ModelLaneContextBundleHandoffRecord,
) -> ModelLaneResult<()> {
    let prepared = prepare_context_bundle_handoff_tx(tx, access, record.inner.clone()).await?;
    if prepared.context_bundle_hash != record.context_bundle_hash
        || context_bundle_handoff_hash(&record.inner)? != record.context_bundle_hash
    {
        return Err(crdt_authority_denied(format!(
            "ContextBundle handoff {} context_bundle_hash does not match its canonical payload",
            record.handoff_id
        )));
    }

    let row = sqlx::query(
        r#"
        SELECT event_type, aggregate_type, aggregate_id, event_sequence,
               session_run_id, payload
        FROM kernel_event_ledger
        WHERE event_id = $1
        "#,
    )
    .bind(&record.event_ledger_event_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| {
        crdt_authority_denied(format!(
            "ContextBundle handoff {} references missing EventLedger event {}",
            record.handoff_id, record.event_ledger_event_id
        ))
    })?;
    let event_type: String = row.try_get("event_type")?;
    let aggregate_type: String = row.try_get("aggregate_type")?;
    let aggregate_id: String = row.try_get("aggregate_id")?;
    let event_sequence: i64 = row.try_get("event_sequence")?;
    let session_run_id: String = row.try_get("session_run_id")?;
    let payload: Value = row.try_get("payload")?;
    let ledger_scope = exact_context_bundle_ledger_scope(&payload, &record.handoff_id)?;
    let expected_scope = require_exact_context_bundle_read_scope(access)?;
    let ledger_record: ModelLaneContextBundleHandoffRecord = payload
        .get("record")
        .cloned()
        .ok_or_else(|| {
            crdt_authority_denied(format!(
                "CONTEXT_BUNDLE_RECORDED EventLedger payload for {} has no record",
                record.handoff_id
            ))
        })
        .and_then(|value| serde_json::from_value(value).map_err(ModelLaneError::from))?;
    if event_type != "CONTEXT_BUNDLE_RECORDED"
        || aggregate_type != "model_lane_context_bundle_handoff"
        || aggregate_id != record.handoff_id
        || event_sequence != record.event_ledger_seq
        || session_run_id != record.event_ledger_stream_id
        || &ledger_scope != expected_scope
        || &ledger_record != record
    {
        return Err(crdt_authority_denied(format!(
            "ContextBundle handoff {} projection does not equal its CONTEXT_BUNDLE_RECORDED EventLedger resource_scope authority",
            record.handoff_id
        )));
    }
    Ok(())
}

async fn validate_stored_context_bundle_artifact_authority_tx(
    tx: &mut Transaction<'_, Postgres>,
    access: &ResourceAccessContext,
    record: &ModelLaneContextBundleArtifactBindingRecord,
) -> ModelLaneResult<()> {
    if context_bundle_artifact_binding_hash(&record.inner)? != record.artifact_binding_hash {
        return Err(crdt_authority_denied(format!(
            "ContextBundle artifact {} artifact_binding_hash does not match its canonical payload",
            record.artifact_binding_id
        )));
    }
    let row = sqlx::query(
        r#"
        SELECT event_type, aggregate_type, aggregate_id, event_sequence,
               session_run_id, payload
        FROM kernel_event_ledger
        WHERE event_id = $1
        "#,
    )
    .bind(&record.event_ledger_event_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| {
        crdt_authority_denied(format!(
            "ContextBundle artifact {} references missing EventLedger event {}",
            record.artifact_binding_id, record.event_ledger_event_id
        ))
    })?;
    let event_type: String = row.try_get("event_type")?;
    let aggregate_type: String = row.try_get("aggregate_type")?;
    let aggregate_id: String = row.try_get("aggregate_id")?;
    let event_sequence: i64 = row.try_get("event_sequence")?;
    let session_run_id: String = row.try_get("session_run_id")?;
    let payload: Value = row.try_get("payload")?;
    let ledger_scope = exact_context_bundle_ledger_scope(&payload, &record.artifact_binding_id)?;
    let expected_scope = require_exact_context_bundle_read_scope(access)?;
    let ledger_record: ModelLaneContextBundleArtifactBindingRecord = payload
        .get("record")
        .cloned()
        .ok_or_else(|| {
            crdt_authority_denied(format!(
                "ARTIFACT_STORED EventLedger payload for {} has no record",
                record.artifact_binding_id
            ))
        })
        .and_then(|value| serde_json::from_value(value).map_err(ModelLaneError::from))?;
    if event_type != "ARTIFACT_STORED"
        || aggregate_type != "model_lane_context_bundle_artifact"
        || aggregate_id != record.artifact_binding_id
        || event_sequence != record.event_ledger_seq
        || session_run_id != record.event_ledger_stream_id
        || &ledger_scope != expected_scope
        || &ledger_record != record
    {
        return Err(crdt_authority_denied(format!(
            "ContextBundle artifact {} projection does not equal its ARTIFACT_STORED EventLedger resource_scope authority",
            record.artifact_binding_id
        )));
    }
    Ok(())
}

fn exact_context_bundle_ledger_scope(
    payload: &Value,
    resource_id: &str,
) -> ModelLaneResult<ExactResourceScopeAttribution> {
    let scope = payload.get("resource_scope").ok_or_else(|| {
        crdt_authority_denied(format!(
            "ContextBundle EventLedger payload for {resource_id} has no resource_scope"
        ))
    })?;
    let object = scope.as_object().ok_or_else(|| {
        crdt_authority_denied(format!(
            "ContextBundle EventLedger payload for {resource_id} has malformed resource_scope"
        ))
    })?;
    const EXACT_FIELDS: [&str; 5] = [
        "owner_account_id",
        "actor_principal_id",
        "authenticated_session_id",
        "access_space_id",
        "workspace_id",
    ];
    if object.len() != EXACT_FIELDS.len()
        || EXACT_FIELDS
            .iter()
            .any(|field| !object.contains_key(*field))
    {
        return Err(crdt_authority_denied(format!(
            "ContextBundle EventLedger payload for {resource_id} has malformed resource_scope"
        )));
    }
    serde_json::from_value(scope.clone()).map_err(|_| {
        crdt_authority_denied(format!(
            "ContextBundle EventLedger payload for {resource_id} has malformed resource_scope"
        ))
    })
}

#[derive(Debug, Clone)]
struct PromotionInputResolution {
    denial_reason: Option<ModelLanePromotionDenialReason>,
    current_base_snapshot_ref: Option<String>,
    current_state_vector: Option<String>,
    selected_message_ids: Vec<String>,
}

async fn promotion_run_by_id_tx(
    tx: &mut Transaction<'_, Postgres>,
    access: &ResourceAccessContext,
    run_id: &str,
) -> ModelLaneResult<ModelLaneRunRecord> {
    let predicate = access.sql_predicate(2);
    let sql = format!(
        "SELECT record_json, {RESOURCE_SCOPE_SELECT_COLUMNS} FROM model_lane_runs \
         WHERE run_id = $1{} FOR UPDATE",
        predicate.clause()
    );
    let run = predicate
        .bind(sqlx::query(&sql).bind(run_id))
        .fetch_optional(&mut **tx)
        .await?
        .map(|row| authorize_and_decode_row(access, row))
        .transpose()?
        .ok_or_else(|| ModelLaneError::NotFound("PromotionGate run authority".into()))?;
    validate_stored_run_eventledger_authority_tx(tx, &run, access.exact_read_scope())
        .await
        .map_err(|_| ModelLaneError::AuthorityDenied("PromotionGate run authority".into()))?;
    Ok(run)
}

async fn promotion_message_by_id_tx(
    tx: &mut Transaction<'_, Postgres>,
    access: &ResourceAccessContext,
    message_id: &str,
) -> ModelLaneResult<Option<ModelLaneMessageRecord>> {
    let predicate = access.sql_predicate(2);
    let sql = format!(
        "SELECT record_json, {RESOURCE_SCOPE_SELECT_COLUMNS} FROM model_lane_messages \
         WHERE message_id = $1{} FOR UPDATE",
        predicate.clause()
    );
    let record = predicate
        .bind(sqlx::query(&sql).bind(message_id))
        .fetch_optional(&mut **tx)
        .await?
        .map(|row| authorize_and_decode_row(access, row))
        .transpose()?;
    if let Some(record) = record.as_ref() {
        validate_stored_message_eventledger_authority_tx(tx, record, access.exact_read_scope())
            .await?;
    }
    Ok(record)
}

async fn prepare_promotion_decision_tx(
    tx: &mut Transaction<'_, Postgres>,
    access: &ResourceAccessContext,
    exact_scope: &ExactResourceScopeAttribution,
    mut input: NewModelLanePromotionDecision,
) -> ModelLaneResult<ModelLanePromotionDecisionRecord> {
    promotion_run_by_id_tx(tx, access, &input.run_id).await?;
    let canonical_input_refs = canonicalize_refs("input_refs", &input.input_refs)?;
    let selected_input_refs = canonicalize_refs("selected_input_refs", &input.selected_input_refs)?;
    let rejected_input_refs = canonicalize_refs("rejected_input_refs", &input.rejected_input_refs)?;
    if selected_input_refs.is_empty() {
        return Err(ModelLaneError::InvalidInput(
            "selected_input_refs must contain at least one advisory input".into(),
        ));
    }
    require_refs_subset(
        "selected_input_refs",
        &selected_input_refs,
        &canonical_input_refs,
    )?;
    require_refs_subset(
        "rejected_input_refs",
        &rejected_input_refs,
        &canonical_input_refs,
    )?;
    require_refs_disjoint(
        "selected_input_refs",
        &selected_input_refs,
        "rejected_input_refs",
        &rejected_input_refs,
    )?;
    let resolution = resolve_promotion_input_refs_tx(
        tx,
        access,
        &input.run_id,
        &canonical_input_refs,
        &selected_input_refs,
    )
    .await?;
    input.input_refs = canonical_input_refs.clone();
    input.selected_input_refs = selected_input_refs;
    input.rejected_input_refs = rejected_input_refs;
    if let Some(current_base_snapshot_ref) = resolution.current_base_snapshot_ref.clone() {
        input.current_base_snapshot_ref = current_base_snapshot_ref;
    } else {
        input.current_base_snapshot_ref = "not-applicable".into();
    }
    if let Some(current_state_vector) = resolution.current_state_vector.clone() {
        input.current_state_vector = current_state_vector;
    } else {
        input.current_state_vector = "not-applicable".into();
    }

    let current_event_ledger_version = latest_event_ledger_version_tx(
        tx,
        &input.expected_event_ledger_aggregate_type,
        &input.expected_event_ledger_aggregate_id,
    )
    .await?;
    let current_schema_id =
        current_schema_id_for_aggregate_tx(tx, &input.expected_event_ledger_aggregate_type).await?;
    let expected_aggregate_matches_selected = input.expected_event_ledger_aggregate_type
        == "model_lane_message"
        && resolution
            .selected_message_ids
            .iter()
            .any(|id| id == &input.expected_event_ledger_aggregate_id);
    let denial_reason = if let Some(reason) = resolution.denial_reason {
        Some(reason)
    } else if resolution.current_base_snapshot_ref.is_none()
        && (input.base_snapshot_ref != "not-applicable" || input.state_vector != "not-applicable")
    {
        Some(ModelLanePromotionDenialReason::InputRefMismatch)
    } else if !expected_aggregate_matches_selected {
        Some(ModelLanePromotionDenialReason::AggregateVersionMismatch)
    } else if current_event_ledger_version != Some(input.expected_event_ledger_version) {
        Some(ModelLanePromotionDenialReason::AggregateVersionMismatch)
    } else if current_schema_id.as_deref() != Some(input.schema_id.as_str()) {
        Some(ModelLanePromotionDenialReason::SchemaMismatch)
    } else if input.base_snapshot_ref != input.current_base_snapshot_ref {
        Some(ModelLanePromotionDenialReason::StaleBase)
    } else if input.state_vector != input.current_state_vector {
        Some(ModelLanePromotionDenialReason::StaleStateVector)
    } else if input.direct_authority_mutation_attempt_ref.is_some() {
        Some(ModelLanePromotionDenialReason::DirectAuthorityMutation)
    } else if input.validator_authority_ref.is_none() && input.operator_authority_ref.is_none() {
        Some(ModelLanePromotionDenialReason::MissingPromotionAuthority)
    } else if missing_promoted_artifact_binding(&input)
        || !promotion_artifact_binding_matches_tx(tx, access, &input).await?
    {
        Some(ModelLanePromotionDenialReason::MissingPromotedArtifactBinding)
    } else {
        None
    };
    let outcome = if denial_reason.is_some() {
        ModelLanePromotionOutcome::Denied
    } else {
        ModelLanePromotionOutcome::Approved
    };
    let state_history = promotion_state_history(outcome);
    let final_state = *state_history
        .last()
        .ok_or_else(|| ModelLaneError::InvalidInput("empty promotion state history".into()))?;
    let canonical_hash_basis = promotion_canonical_hash_basis(
        &input,
        outcome,
        final_state,
        denial_reason,
        current_event_ledger_version,
        current_schema_id.as_deref(),
        exact_scope,
    );
    let canonical_decision_hash = dexterity_sha256_hex(serde_json::to_vec(&canonical_hash_basis)?);

    Ok(ModelLanePromotionDecisionRecord {
        inner: input,
        outcome,
        final_state,
        denial_reason,
        state_history,
        canonical_input_refs,
        canonical_hash_basis,
        canonical_decision_hash,
        current_event_ledger_version,
        current_schema_id,
        event_ledger_event_id: String::new(),
        event_ledger_seq: 0,
        event_stream_version: 0,
        transaction_seq: 0,
    })
}

async fn promotion_artifact_binding_matches_tx(
    tx: &mut Transaction<'_, Postgres>,
    access: &ResourceAccessContext,
    input: &NewModelLanePromotionDecision,
) -> ModelLaneResult<bool> {
    let (Some(artifact_ref), Some(artifact_sha256), Some(artifact_version)) = (
        input.promoted_artifact_ref.as_deref(),
        input.promoted_artifact_sha256.as_deref(),
        input.promoted_artifact_version.as_deref(),
    ) else {
        return Ok(false);
    };
    let Some(binding) =
        context_bundle_artifact_binding_by_ref_tx(tx, access, &input.run_id, artifact_ref).await?
    else {
        return Ok(false);
    };
    Ok(binding.artifact_sha256 == artifact_sha256
        && binding.content_hash == artifact_sha256
        && binding
            .payload_json
            .get("artifact_version")
            .and_then(Value::as_str)
            == Some(artifact_version))
}

fn promotion_exact_scope_from_columns(
    scope: ScopeColumnValues<'_>,
) -> ModelLaneResult<ExactResourceScopeAttribution> {
    let denied = || {
        ModelLaneError::AuthorityDenied(
            "PromotionGate requires complete account/principal/session/AccessSpace/workspace authority"
                .into(),
        )
    };
    Ok(ExactResourceScopeAttribution {
        owner_account_id: OwnerAccountId::from_uuid(scope.owner_account_id.ok_or_else(denied)?),
        actor_principal_id: ActorPrincipalId::from_uuid(
            scope.actor_principal_id.ok_or_else(denied)?,
        ),
        authenticated_session_id: AuthenticatedSessionRef::from_uuid(
            scope.authenticated_session_id.ok_or_else(denied)?,
        ),
        access_space_id: AccessSpaceRef::from_uuid(scope.access_space_id.ok_or_else(denied)?),
        workspace_id: WorkspaceScopeRef::new(scope.workspace_id.ok_or_else(denied)?)
            .map_err(|_| denied())?,
    })
}

fn exact_resource_scope_from_columns(
    scope: ScopeColumnValues<'_>,
    resource: &str,
) -> ModelLaneResult<Option<ExactResourceScopeAttribution>> {
    let dimensions = [
        scope.owner_account_id.is_some(),
        scope.actor_principal_id.is_some(),
        scope.authenticated_session_id.is_some(),
        scope.access_space_id.is_some(),
        scope.workspace_id.is_some(),
    ];
    if dimensions.iter().all(|present| !present) {
        return Ok(None);
    }
    if !dimensions.iter().all(|present| *present) {
        return Err(ModelLaneError::AuthorityDenied(format!(
            "{resource} requires either complete account/principal/session/AccessSpace/workspace authority or fully unattributed system scope"
        )));
    }
    promotion_exact_scope_from_columns(scope).map(Some)
}

fn exact_resource_scope_from_stored(
    stored: &super::resource_scope::StoredResourceScope,
    resource: &str,
) -> ModelLaneResult<Option<ExactResourceScopeAttribution>> {
    let dimensions = [
        stored.owner_account_id.is_some(),
        stored.actor_principal_id.is_some(),
        stored.authenticated_session.is_some(),
        stored.access_space.is_some(),
        stored.workspace.is_some(),
    ];
    if dimensions.iter().all(|present| !present) {
        return Ok(None);
    }
    if !dimensions.iter().all(|present| *present) {
        return Err(ModelLaneError::AuthorityDenied(format!(
            "{resource} requires either complete account/principal/session/AccessSpace/workspace authority or fully unattributed system scope"
        )));
    }
    Ok(Some(ExactResourceScopeAttribution {
        owner_account_id: stored.owner_account_id.expect("complete scope checked"),
        actor_principal_id: stored.actor_principal_id.expect("complete scope checked"),
        authenticated_session_id: stored
            .authenticated_session
            .expect("complete scope checked"),
        access_space_id: stored.access_space.expect("complete scope checked"),
        workspace_id: stored.workspace.clone().expect("complete scope checked"),
    }))
}

async fn validate_stored_promotion_decision_authority_tx(
    tx: &mut Transaction<'_, Postgres>,
    access: &ResourceAccessContext,
    record: &ModelLanePromotionDecisionRecord,
) -> ModelLaneResult<()> {
    let denied = || {
        ModelLaneError::AuthorityDenied(
            "PromotionGate decision projection does not equal its canonical/EventLedger authority"
                .into(),
        )
    };
    let exact_scope = access.exact_read_scope().ok_or_else(denied)?;
    let expected_basis = promotion_canonical_hash_basis(
        &record.inner,
        record.outcome,
        record.final_state,
        record.denial_reason,
        record.current_event_ledger_version,
        record.current_schema_id.as_deref(),
        exact_scope,
    );
    let expected_hash =
        dexterity_sha256_hex(serde_json::to_vec(&expected_basis).map_err(ModelLaneError::from)?);
    let expected_input_refs =
        canonicalize_refs("input_refs", &record.input_refs).map_err(|_| denied())?;
    if record.canonical_hash_basis != expected_basis
        || record.canonical_decision_hash != expected_hash
        || record.canonical_input_refs != expected_input_refs
        || record.state_history != promotion_state_history(record.outcome)
        || record.state_history.last().copied() != Some(record.final_state)
    {
        return Err(denied());
    }

    let row = sqlx::query(
        r#"
        SELECT event_type, aggregate_type, aggregate_id, event_sequence,
               session_run_id, payload, payload_hash
        FROM kernel_event_ledger
        WHERE event_id = $1
        "#,
    )
    .bind(&record.event_ledger_event_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(denied)?;
    let event_type: String = row.try_get("event_type")?;
    let aggregate_type: String = row.try_get("aggregate_type")?;
    let aggregate_id: String = row.try_get("aggregate_id")?;
    let event_sequence: i64 = row.try_get("event_sequence")?;
    let session_run_id: String = row.try_get("session_run_id")?;
    let payload: Value = row.try_get("payload")?;
    let payload_hash: String = row.try_get("payload_hash")?;
    let mut expected_payload = json!({
        "schema_id": "hsk.model_lane_promotion_decision@1",
        "dexterity_kernel": "Dexterity",
        "record": record,
    });
    exact_scope
        .stamp_json_object(&mut expected_payload)
        .map_err(|_| denied())?;
    let expected_event_type = match record.outcome {
        ModelLanePromotionOutcome::Approved => "PROMOTION_ACCEPTED",
        ModelLanePromotionOutcome::Denied => "PROMOTION_REJECTED",
    };
    if event_type != expected_event_type
        || aggregate_type != "model_lane_promotion_decision"
        || aggregate_id != record.decision_id
        || event_sequence != record.event_ledger_seq
        || event_sequence != record.event_stream_version
        || event_sequence != record.transaction_seq
        || session_run_id != record.event_ledger_stream_id
        || payload != expected_payload
        || payload_hash != dexterity_sha256_hex(canonical_json_bytes(&payload))
    {
        return Err(denied());
    }
    Ok(())
}

async fn latest_event_ledger_version_tx(
    tx: &mut Transaction<'_, Postgres>,
    aggregate_type: &str,
    aggregate_id: &str,
) -> ModelLaneResult<Option<i64>> {
    sqlx::query_scalar(
        r#"
        SELECT event_sequence
        FROM kernel_event_ledger
        WHERE aggregate_type = $1 AND aggregate_id = $2
        ORDER BY event_sequence DESC
        LIMIT 1
        "#,
    )
    .bind(aggregate_type)
    .bind(aggregate_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(ModelLaneError::from)
}

async fn current_schema_id_for_aggregate_tx(
    tx: &mut Transaction<'_, Postgres>,
    aggregate_type: &str,
) -> ModelLaneResult<Option<String>> {
    let table_name = match aggregate_type {
        "model_lane_run" => "model_lane_runs",
        "model_lane" => "model_lanes",
        "model_lane_message" => "model_lane_messages",
        "model_lane_promotion_decision" => "model_lane_promotion_decisions",
        "model_lane_context_bundle_artifact" => "model_lane_context_bundle_artifacts",
        "model_lane_context_bundle_handoff" => "model_lane_context_bundle_handoffs",
        "model_lane_recovery_checkpoint" => "model_lane_recovery_checkpoints",
        "model_lane_recovery_event" => "model_lane_recovery_events",
        "model_lane_lease" => "model_lane_leases",
        "model_lane_diagnostic_tier" => "model_lane_diagnostic_tier_statuses",
        "model_lane_mt_runtime_status" => "model_lane_mt_runtime_statuses",
        _ => return Ok(None),
    };
    sqlx::query_scalar(
        r#"
        SELECT schema_id
        FROM model_lane_schema_registry
        WHERE table_name = $1
        ORDER BY schema_version DESC
        LIMIT 1
        "#,
    )
    .bind(table_name)
    .fetch_optional(&mut **tx)
    .await
    .map_err(ModelLaneError::from)
}

fn canonicalize_refs(field: &str, refs: &[String]) -> ModelLaneResult<Vec<String>> {
    let mut out = BTreeSet::new();
    for reference in refs {
        require_token(field, reference)?;
        out.insert(reference.clone());
    }
    Ok(out.into_iter().collect())
}

fn require_refs_subset(field: &str, refs: &[String], input_refs: &[String]) -> ModelLaneResult<()> {
    for reference in refs {
        if !input_refs.iter().any(|candidate| candidate == reference) {
            return Err(ModelLaneError::InvalidInput(format!(
                "{field} contains {reference}, which is not present in input_refs"
            )));
        }
    }
    Ok(())
}

fn require_refs_disjoint(
    left_field: &str,
    left: &[String],
    right_field: &str,
    right: &[String],
) -> ModelLaneResult<()> {
    for reference in left {
        if right.iter().any(|candidate| candidate == reference) {
            return Err(ModelLaneError::InvalidInput(format!(
                "{left_field} and {right_field} both contain {reference}"
            )));
        }
    }
    Ok(())
}

fn validate_recovery_checkpoint(input: &NewModelLaneRecoveryCheckpoint) -> ModelLaneResult<()> {
    require_token("checkpoint_id", &input.checkpoint_id)?;
    require_token("run_id", &input.run_id)?;
    if let Some(lane_id) = input.lane_id.as_deref() {
        require_token("lane_id", lane_id)?;
    }
    require_token("session_id", &input.session_id)?;
    require_token("model_session_id", &input.model_session_id)?;
    if input.last_event_ledger_seq <= 0 {
        return Err(ModelLaneError::InvalidInput(
            "recovery checkpoint last_event_ledger_seq must be positive".into(),
        ));
    }
    if let Some(last_message_id) = input.last_message_id.as_deref() {
        require_token("last_message_id", last_message_id)?;
    }
    for payload_ref in &input.open_payload_refs {
        require_token("open_payload_refs[]", payload_ref)?;
    }
    if let Some(lease_id) = input.lease_id.as_deref() {
        require_token("lease_id", lease_id)?;
    }
    require_token("idempotency_scope", &input.idempotency_scope)?;
    if let Some(recovery_event_ref) = input.recovery_event_ref.as_deref() {
        require_token("recovery_event_ref", recovery_event_ref)?;
    }
    require_token("event_ledger_stream_id", &input.event_ledger_stream_id)?;
    require_token("work_packet_id", &input.work_packet_id)?;
    require_token("micro_task_id", &input.micro_task_id)?;
    require_token("task_board_id", &input.task_board_id)?;
    require_token("owner_session", &input.owner_session)?;
    require_token("idempotency_key", &input.idempotency_key)?;
    parse_utc("created_at_utc", &input.created_at_utc)?;
    if let Some(recovery_hint_ref) = input.recovery_hint_ref.as_deref() {
        require_token("recovery_hint_ref", recovery_hint_ref)?;
    }
    ensure_object_payload("diagnostic_payload", &input.diagnostic_payload)
}

fn validate_recovery_event(input: &NewModelLaneRecoveryEvent) -> ModelLaneResult<()> {
    require_token("recovery_event_id", &input.recovery_event_id)?;
    require_token("run_id", &input.run_id)?;
    if let Some(lane_id) = input.lane_id.as_deref() {
        require_token("lane_id", lane_id)?;
    }
    require_token("trace_id", &input.trace_id)?;
    require_token("span_id", &input.span_id)?;
    if let Some(parent_span_id) = input.parent_span_id.as_deref() {
        require_token("parent_span_id", parent_span_id)?;
    }
    for linked in &input.linked_span_contexts {
        require_token("linked_span_contexts[]", linked)?;
    }
    if let Some(session_id) = input.session_id.as_deref() {
        require_token("session_id", session_id)?;
    }
    if let Some(model_session_id) = input.model_session_id.as_deref() {
        require_token("model_session_id", model_session_id)?;
    }
    if input.replay_order_seq <= 0 {
        return Err(ModelLaneError::InvalidInput(
            "recovery event replay_order_seq must be positive".into(),
        ));
    }
    if input.source_event_ledger_seq.is_some_and(|seq| seq <= 0) {
        return Err(ModelLaneError::InvalidInput(
            "recovery event source_event_ledger_seq must be positive when present".into(),
        ));
    }
    for payload_ref in &input.payload_refs {
        require_token("payload_refs[]", payload_ref)?;
    }
    for artifact_ref in &input.artifact_refs {
        require_token("artifact_refs[]", artifact_ref)?;
    }
    if let Some(crdt_base_snapshot_ref) = input.crdt_base_snapshot_ref.as_deref() {
        require_token("crdt_base_snapshot_ref", crdt_base_snapshot_ref)?;
    }
    if let Some(crdt_state_vector) = input.crdt_state_vector.as_deref() {
        require_token("crdt_state_vector", crdt_state_vector)?;
    }
    if let Some(crdt_stale_base_ref) = input.crdt_stale_base_ref.as_deref() {
        require_token("crdt_stale_base_ref", crdt_stale_base_ref)?;
    }
    if let Some(lease_id) = input.lease_id.as_deref() {
        require_token("lease_id", lease_id)?;
    }
    if let Some(error_code) = input.error_code.as_deref() {
        require_token("error_code", error_code)?;
    }
    require_token("replay_hint", &input.replay_hint)?;
    require_token("event_ledger_stream_id", &input.event_ledger_stream_id)?;
    require_token("work_packet_id", &input.work_packet_id)?;
    require_token("micro_task_id", &input.micro_task_id)?;
    require_token("task_board_id", &input.task_board_id)?;
    require_token("owner_session", &input.owner_session)?;
    require_token("idempotency_key", &input.idempotency_key)?;
    if let Some(recovery_hint_ref) = input.recovery_hint_ref.as_deref() {
        require_token("recovery_hint_ref", recovery_hint_ref)?;
    }
    ensure_object_payload("diagnostic_payload", &input.diagnostic_payload)
}

fn validate_lane_lease(input: &NewModelLaneLease) -> ModelLaneResult<()> {
    require_token("lease_id", &input.lease_id)?;
    require_token("run_id", &input.run_id)?;
    if let Some(lane_id) = input.lane_id.as_deref() {
        require_token("lane_id", lane_id)?;
    } else if input.scope == ModelLaneLeaseScope::Lane {
        return Err(ModelLaneError::InvalidInput(
            "lane-scoped lease requires lane_id".into(),
        ));
    }
    require_token("scope_ref", &input.scope_ref)?;
    require_token("holder_actor_id", &input.holder_actor_id)?;
    require_token("holder_session_id", &input.holder_session_id)?;
    parse_utc("lease_expires_at_utc", &input.lease_expires_at_utc)?;
    require_token("takeover_policy_ref", &input.takeover_policy_ref)?;
    require_token("event_ledger_stream_id", &input.event_ledger_stream_id)?;
    require_token("work_packet_id", &input.work_packet_id)?;
    require_token("micro_task_id", &input.micro_task_id)?;
    require_token("task_board_id", &input.task_board_id)?;
    require_token("owner_session", &input.owner_session)?;
    require_token("idempotency_key", &input.idempotency_key)?;
    if let Some(recovery_hint_ref) = input.recovery_hint_ref.as_deref() {
        require_token("recovery_hint_ref", recovery_hint_ref)?;
    }
    ensure_object_payload("diagnostic_payload", &input.diagnostic_payload)
}

fn validate_diagnostic_tier_status(
    input: &NewModelLaneDiagnosticTierStatus,
) -> ModelLaneResult<()> {
    require_token("diagnostic_status_id", &input.diagnostic_status_id)?;
    require_token("behavior_id", &input.behavior_id)?;
    require_token("run_id", &input.run_id)?;
    require_token("reason", &input.reason)?;
    require_token("evidence_ref", &input.evidence_ref)?;
    if input.tier == ModelLaneDiagnosticTier::FlightRecorder
        && input.evidence_ref.starts_with("flight-recorder://")
    {
        return Err(ModelLaneError::InvalidInput(
            "FlightRecorder tier must point at kernel_event_ledger/EventLedger evidence, not a detached flight-recorder-only ref".into(),
        ));
    }
    if let Some(follow_up_ref) = input.follow_up_ref.as_deref() {
        require_token("follow_up_ref", follow_up_ref)?;
    }
    if input.state == ModelLaneDiagnosticTierState::Missing {
        return Err(ModelLaneError::InvalidInput(
            "HBR-INT-009 diagnostic tier status cannot be missing".into(),
        ));
    }
    if input.state == ModelLaneDiagnosticTierState::DeferredWithReason
        && input.follow_up_ref.is_none()
    {
        return Err(ModelLaneError::InvalidInput(
            "deferred diagnostic tier requires follow_up_ref".into(),
        ));
    }
    require_token("event_ledger_stream_id", &input.event_ledger_stream_id)?;
    require_token("work_packet_id", &input.work_packet_id)?;
    require_token("micro_task_id", &input.micro_task_id)?;
    require_token("task_board_id", &input.task_board_id)?;
    require_token("owner_session", &input.owner_session)?;
    require_token("idempotency_key", &input.idempotency_key)?;
    ensure_object_payload("diagnostic_payload", &input.diagnostic_payload)
}

fn validate_mt_runtime_status(input: &NewModelLaneMtRuntimeStatus) -> ModelLaneResult<()> {
    require_token("mt_status_id", &input.mt_status_id)?;
    require_token("run_id", &input.run_id)?;
    require_token("work_packet_id", &input.work_packet_id)?;
    require_token("micro_task_id", &input.micro_task_id)?;
    require_token("task_board_id", &input.task_board_id)?;
    if let Some(claimed_by_ref) = input.claimed_by_ref.as_deref() {
        require_token("claimed_by_ref", claimed_by_ref)?;
    }
    if let Some(blocker_ref) = input.blocker_ref.as_deref() {
        require_token("blocker_ref", blocker_ref)?;
    }
    if let Some(missing_resource_ref) = input.missing_resource_ref.as_deref() {
        require_token("missing_resource_ref", missing_resource_ref)?;
    }
    if let Some(proof_status_ref) = input.proof_status_ref.as_deref() {
        require_token("proof_status_ref", proof_status_ref)?;
    }
    if let Some(hbr_status_ref) = input.hbr_status_ref.as_deref() {
        require_token("hbr_status_ref", hbr_status_ref)?;
    }
    if let Some(last_recovery_event_ref) = input.last_recovery_event_ref.as_deref() {
        require_token("last_recovery_event_ref", last_recovery_event_ref)?;
    }
    if let Some(last_runtime_status_ref) = input.last_runtime_status_ref.as_deref() {
        require_token("last_runtime_status_ref", last_runtime_status_ref)?;
    }
    require_token("event_ledger_stream_id", &input.event_ledger_stream_id)?;
    require_token("owner_session", &input.owner_session)?;
    require_token("idempotency_key", &input.idempotency_key)?;
    ensure_object_payload("diagnostic_payload", &input.diagnostic_payload)
}

fn validate_cloud_projection_plan(input: &NewModelLaneCloudProjectionPlan) -> ModelLaneResult<()> {
    require_token("projection_plan_id", &input.projection_plan_id)?;
    require_token("run_id", &input.run_id)?;
    require_token("trace_id", &input.trace_id)?;
    validate_cloud_consent_scope_bindings(
        input.consent_scope,
        input.lane_id.as_deref(),
        input.model_session_id.as_deref(),
        input.provider_kind.as_deref(),
        input.requested_model_id.as_deref(),
        &input.target_bindings,
    )?;
    validate_sha256("scope_hash", &input.scope_hash)?;
    validate_sha256("payload_sha256", &input.payload_sha256)?;
    if input.source_artifact_refs.is_empty() {
        return Err(ModelLaneError::InvalidInput(
            "ProjectionPlan requires source_artifact_refs".into(),
        ));
    }
    for reference in &input.source_artifact_refs {
        require_token("source_artifact_refs[]", reference)?;
        reject_hidden_provider_ref("source_artifact_refs[]", reference)?;
    }
    require_token("payload_artifact_ref", &input.payload_artifact_ref)?;
    reject_hidden_provider_ref("payload_artifact_ref", &input.payload_artifact_ref)?;
    require_token("redaction_policy_ref", &input.redaction_policy_ref)?;
    require_token("redaction_summary", &input.redaction_summary)?;
    require_token("provider_profile_ref", &input.provider_profile_ref)?;
    if input.fan_out_targets.is_empty() {
        return Err(ModelLaneError::InvalidInput(
            "ProjectionPlan requires fan_out_targets".into(),
        ));
    }
    for target in &input.fan_out_targets {
        require_token("fan_out_targets[]", target)?;
    }
    validate_cloud_export_delegation(&input.export_delegation, &input.fan_out_targets)?;
    require_token("event_ledger_stream_id", &input.event_ledger_stream_id)?;
    require_token("work_packet_id", &input.work_packet_id)?;
    require_token("micro_task_id", &input.micro_task_id)?;
    require_token("task_board_id", &input.task_board_id)?;
    require_token("owner_session", &input.owner_session)?;
    require_token("idempotency_key", &input.idempotency_key)?;
    parse_utc("created_at_utc", &input.created_at_utc)?;
    require_token("user_manual_behavior_ref", &input.user_manual_behavior_ref)?;
    ensure_object_payload("diagnostic_payload", &input.diagnostic_payload)
}

/// HBR-PRIV-007 non-widening gate for the remote/SaaS delegation record.
///
/// The audience is checked as a subset of the plan's disclosed `fan_out_targets`
/// rather than as free text. Subset, not equality, because a plan may legitimately
/// disclose more possible destinations than one projection actually delegates to —
/// but it may never delegate to a destination it never disclosed.
fn validate_cloud_export_delegation(
    delegation: &CloudExportDelegation,
    fan_out_targets: &[String],
) -> ModelLaneResult<()> {
    if delegation.audience_refs.is_empty() {
        return Err(ModelLaneError::InvalidInput(
            "export_delegation requires audience_refs".into(),
        ));
    }
    // Duplicates are deliberately NOT rejected. A broadcast plan's fan-out list
    // legitimately repeats one provider endpoint when several lanes target it,
    // and the audience is derived from that list — requiring the audience to be
    // stricter than the list it must be a subset of would be an invented
    // constraint. A repeated destination cannot widen visibility; naming an
    // undisclosed destination can, and that is what is checked below.
    for audience in &delegation.audience_refs {
        require_token("export_delegation.audience_refs[]", audience)?;
        reject_hidden_provider_ref("export_delegation.audience_refs[]", audience)?;
        if !fan_out_targets.iter().any(|target| target == audience) {
            return Err(ModelLaneError::InvalidInput(format!(
                "export_delegation.audience_refs must not widen beyond fan_out_targets: {audience} is not a disclosed fan-out target"
            )));
        }
    }
    validate_account_bound_authority("export_delegation.source_scope", &delegation.source_scope)?;
    if let Some(receipt_ref) = delegation.authorization_receipt_ref.as_deref() {
        require_token("export_delegation.authorization_receipt_ref", receipt_ref)?;
    }
    Ok(())
}

/// Reject an identity that is structurally incapable of naming anybody.
///
/// A nil UUID is the "I had to put something here" value; accepting it would
/// reintroduce the exact failure mode this pillar exists to stop, one layer
/// deeper. An `Unattributed` authority must carry a stable reason so every
/// unattributed row is enumerable by an auditor.
fn validate_account_bound_authority(
    field: &str,
    authority: &AccountBoundAuthority,
) -> ModelLaneResult<()> {
    match authority {
        AccountBoundAuthority::Account {
            owner_account_id,
            actor_principal_id,
            ..
        } => {
            if owner_account_id.as_uuid().is_nil() {
                return Err(ModelLaneError::InvalidInput(format!(
                    "{field}.owner_account_id must not be the nil UUID"
                )));
            }
            if actor_principal_id.as_uuid().is_nil() {
                return Err(ModelLaneError::InvalidInput(format!(
                    "{field}.actor_principal_id must not be the nil UUID"
                )));
            }
            Ok(())
        }
        AccountBoundAuthority::Unattributed { reason } => {
            require_token(&format!("{field}.reason"), reason)
        }
    }
}

/// Refuse an `approved_by_ref` whose identity component is the row's own
/// governance role label — the self-issuance shape
/// `operator://{owner_session}/...` that the operator-chat cloud path used to
/// mint.
///
/// This is deliberately narrow. It rejects the shape that carries zero
/// information (issuer == subject) without pretending that string shape is where
/// authorization lives; the typed `approver` is the actual gate. Scoping it this
/// way also means an honest reference that merely happens to use the `operator://`
/// scheme is untouched, so no real lineage is destroyed to satisfy a lint.
/// A durable authorization record must name the account the store is actually
/// writing as.
///
/// Without this, `approver` would be one more client-asserted value with a nicer
/// type: a caller could stamp any account id it liked into it and the row would
/// look account-bound. The account comes from the store's
/// [`ResourceAccessContext`], which is derived from the request seam
/// (`X-Handshake-Owner-Account` today, an authenticated session after
/// WP-KERNEL-006) — never from the payload.
///
/// A legacy/system store has no write scope, so on that path only an
/// `Unattributed` authority is accepted: an unscoped call site cannot mint an
/// account-bound approval at all.
fn ensure_authority_matches_write_scope(
    field: &str,
    authority: &AccountBoundAuthority,
    access: &ResourceAccessContext,
) -> ModelLaneResult<()> {
    let permitted = AccountBoundAuthority::from_access(access);
    if authority.owner_account_id() != permitted.owner_account_id() {
        return Err(ModelLaneError::AuthorityDenied(format!(
            "CX-MM-007 {field} names an owning account this store is not authorized to write as"
        )));
    }
    Ok(())
}

fn reject_self_minted_approver(approved_by_ref: &str, owner_session: &str) -> ModelLaneResult<()> {
    let Some((_scheme, rest)) = approved_by_ref.split_once("://") else {
        return Ok(());
    };
    let authority = rest.split('/').next().unwrap_or_default();
    if !authority.is_empty() && authority == owner_session.trim() {
        return Err(ModelLaneError::InvalidInput(format!(
            "approved_by_ref {approved_by_ref} is self-issued: its identity component is this row's own owner_session governance role label, which authorizes nothing. Record a typed approver instead."
        )));
    }
    Ok(())
}

fn validate_cloud_consent_receipt(input: &NewModelLaneCloudConsentReceipt) -> ModelLaneResult<()> {
    require_token("consent_receipt_id", &input.consent_receipt_id)?;
    require_token("projection_plan_id", &input.projection_plan_id)?;
    validate_sha256("projection_plan_hash", &input.projection_plan_hash)?;
    require_token("run_id", &input.run_id)?;
    require_token("trace_id", &input.trace_id)?;
    validate_cloud_consent_scope_bindings(
        input.consent_scope,
        input.lane_id.as_deref(),
        input.model_session_id.as_deref(),
        input.provider_kind.as_deref(),
        input.requested_model_id.as_deref(),
        &input.target_bindings,
    )?;
    validate_sha256("scope_hash", &input.scope_hash)?;
    if input.fan_out_targets.is_empty() {
        return Err(ModelLaneError::InvalidInput(
            "ConsentReceipt requires fan_out_targets".into(),
        ));
    }
    for target in &input.fan_out_targets {
        require_token("fan_out_targets[]", target)?;
    }
    validate_account_bound_authority("approver", &input.approver)?;
    require_token("approved_by_ref", &input.approved_by_ref)?;
    reject_self_minted_approver(&input.approved_by_ref, &input.owner_session)?;
    parse_utc("approved_at_utc", &input.approved_at_utc)?;
    let valid_from = parse_utc("valid_from_utc", &input.valid_from_utc)?;
    let valid_until = parse_utc("valid_until_utc", &input.valid_until_utc)?;
    if valid_until <= valid_from {
        return Err(ModelLaneError::InvalidInput(
            "valid_until_utc must be after valid_from_utc".into(),
        ));
    }
    if let Some(revoked_at_utc) = input.revoked_at_utc.as_deref() {
        parse_utc("revoked_at_utc", revoked_at_utc)?;
    }
    if input.status == ModelLaneCloudConsentReceiptStatus::Revoked {
        require_optional_token("revocation_ref", input.revocation_ref.as_deref())?;
        let hash = input.revocation_input_hash.as_deref().ok_or_else(|| {
            ModelLaneError::InvalidInput(
                "revoked ConsentReceipt requires revocation_input_hash".into(),
            )
        })?;
        validate_sha256("revocation_input_hash", hash)?;
    } else if input.revocation_input_hash.is_some() {
        return Err(ModelLaneError::InvalidInput(
            "approved ConsentReceipt must not carry revocation_input_hash".into(),
        ));
    }
    require_token("event_ledger_stream_id", &input.event_ledger_stream_id)?;
    require_token("work_packet_id", &input.work_packet_id)?;
    require_token("micro_task_id", &input.micro_task_id)?;
    require_token("task_board_id", &input.task_board_id)?;
    require_token("owner_session", &input.owner_session)?;
    require_token("idempotency_key", &input.idempotency_key)?;
    parse_utc("created_at_utc", &input.created_at_utc)?;
    require_token("user_manual_behavior_ref", &input.user_manual_behavior_ref)?;
    ensure_object_payload("diagnostic_payload", &input.diagnostic_payload)
}

fn validate_cloud_provider_kind(provider_kind: &str) -> ModelLaneResult<()> {
    require_token("provider_kind", provider_kind)?;
    match provider_kind {
        "openai" | "anthropic" => Ok(()),
        other => Err(ModelLaneError::InvalidInput(format!(
            "cloud provider_kind {other} is not supported by Dexterity cloud consent"
        ))),
    }
}

fn canonicalize_cloud_consent_targets(targets: &mut Vec<ModelLaneCloudConsentTargetBinding>) {
    targets.sort_by(|left, right| {
        (
            &left.lane_id,
            &left.model_session_id,
            &left.provider_kind,
            &left.requested_model_id,
            &left.capability_snapshot_ref,
            &left.provider_endpoint_ref,
        )
            .cmp(&(
                &right.lane_id,
                &right.model_session_id,
                &right.provider_kind,
                &right.requested_model_id,
                &right.capability_snapshot_ref,
                &right.provider_endpoint_ref,
            ))
    });
}

fn validate_cloud_consent_scope_bindings(
    scope: ModelLaneCloudConsentScope,
    lane_id: Option<&str>,
    model_session_id: Option<&str>,
    provider_kind: Option<&str>,
    requested_model_id: Option<&str>,
    target_bindings: &[ModelLaneCloudConsentTargetBinding],
) -> ModelLaneResult<()> {
    match scope {
        ModelLaneCloudConsentScope::SingleLane => {
            require_optional_token("lane_id", lane_id)?;
            require_optional_token("model_session_id", model_session_id)?;
            let provider_kind = require_optional_token("provider_kind", provider_kind)?;
            validate_cloud_provider_kind(&provider_kind)?;
            require_optional_token("requested_model_id", requested_model_id)?;
            if !target_bindings.is_empty() {
                return Err(ModelLaneError::InvalidInput(
                    "single_lane cloud consent must not carry broadcast target_bindings".into(),
                ));
            }
        }
        ModelLaneCloudConsentScope::SingleRun => {
            if lane_id.is_some()
                || model_session_id.is_some()
                || provider_kind.is_some()
                || requested_model_id.is_some()
            {
                return Err(ModelLaneError::InvalidInput(
                    "single_run cloud consent must not carry lane-bound identity".into(),
                ));
            }
            if !target_bindings.is_empty() {
                return Err(ModelLaneError::InvalidInput(
                    "single_run cloud consent must not carry lane-bound target_bindings".into(),
                ));
            }
        }
    }

    let mut canonical = target_bindings.to_vec();
    canonicalize_cloud_consent_targets(&mut canonical);
    if canonical != target_bindings {
        return Err(ModelLaneError::InvalidInput(
            "cloud consent target_bindings must be in canonical order".into(),
        ));
    }
    let mut lane_ids = std::collections::BTreeSet::new();
    let mut model_session_ids = std::collections::BTreeSet::new();
    for target in target_bindings {
        require_token("target_bindings[].lane_id", &target.lane_id)?;
        require_token(
            "target_bindings[].model_session_id",
            &target.model_session_id,
        )?;
        validate_cloud_provider_kind(&target.provider_kind)?;
        require_token(
            "target_bindings[].requested_model_id",
            &target.requested_model_id,
        )?;
        require_token(
            "target_bindings[].capability_snapshot_ref",
            &target.capability_snapshot_ref,
        )?;
        require_token(
            "target_bindings[].provider_endpoint_ref",
            &target.provider_endpoint_ref,
        )?;
        if !lane_ids.insert(target.lane_id.as_str())
            || !model_session_ids.insert(target.model_session_id.as_str())
        {
            return Err(ModelLaneError::InvalidInput(
                "cloud consent target_bindings require unique lane_id and model_session_id".into(),
            ));
        }
    }
    Ok(())
}

fn cloud_consent_target_bindings_hash(
    _scope: ModelLaneCloudConsentScope,
    _target_bindings: &[ModelLaneCloudConsentTargetBinding],
) -> ModelLaneResult<Option<String>> {
    Ok(None)
}

fn cloud_projection_plan_hash(input: &NewModelLaneCloudProjectionPlan) -> ModelLaneResult<String> {
    Ok(dexterity_sha256_hex(canonical_json_bytes(
        &serde_json::to_value(input)?,
    )))
}

fn cloud_consent_receipt_hash(input: &NewModelLaneCloudConsentReceipt) -> ModelLaneResult<String> {
    Ok(dexterity_sha256_hex(canonical_json_bytes(
        &serde_json::to_value(input)?,
    )))
}

fn cloud_consent_revocation_input_hash(
    consent_receipt_id: &str,
    revoked_by_ref: &str,
    reason: &str,
) -> String {
    dexterity_sha256_hex(canonical_json_bytes(&json!({
        "consent_receipt_id": consent_receipt_id,
        "revoked_by_ref": revoked_by_ref,
        "reason": reason,
    })))
}

fn cloud_projection_plan_event_payload(record: &ModelLaneCloudProjectionPlanRecord) -> Value {
    json!({
        "schema_id": "hsk.model_lane_cloud_projection_plan@2",
        "dexterity_kernel": "Dexterity",
        "flight_recorder": "EventLedger",
        "user_manual_behavior_ref": &record.user_manual_behavior_ref,
        "record": record,
    })
}

fn cloud_consent_receipt_event_payload(record: &ModelLaneCloudConsentReceiptRecord) -> Value {
    let mut payload = json!({
        "schema_id": "hsk.model_lane_cloud_consent_receipt@2",
        "dexterity_kernel": "Dexterity",
        "flight_recorder": "EventLedger",
        "user_manual_behavior_ref": &record.user_manual_behavior_ref,
        "record": record,
    });
    if record.status == ModelLaneCloudConsentReceiptStatus::Revoked {
        if let Some(object) = payload.as_object_mut() {
            object.insert("reason_code".into(), json!("CX-MM-007"));
            object.insert("consent_status".into(), json!("CX-MM-007"));
            object.insert(
                "revocation_ref".into(),
                json!(record.revocation_ref.as_deref()),
            );
        }
    }
    payload
}

fn recovery_checkpoint_event_payload(
    record: &ModelLaneRecoveryCheckpointRecord,
    scope: ScopeColumnValues<'_>,
) -> Value {
    json!({
        "schema_id": "hsk.model_lane_recovery_checkpoint@1",
        "dexterity_kernel": "Dexterity",
        "flight_recorder": "EventLedger",
        "resource_scope": context_bundle_resource_scope_payload(scope),
        "record": record,
    })
}

fn recovery_event_event_payload(
    record: &ModelLaneRecoveryEventRecord,
    scope: ScopeColumnValues<'_>,
) -> Value {
    json!({
        "schema_id": "hsk.model_lane_recovery_event@2",
        "dexterity_kernel": "Dexterity",
        "flight_recorder": "EventLedger",
        "resource_scope": context_bundle_resource_scope_payload(scope),
        "record": record,
    })
}

fn lane_lease_event_payload(record: &ModelLaneLeaseRecord, scope: ScopeColumnValues<'_>) -> Value {
    json!({
        "schema_id": "hsk.model_lane_lease@1",
        "dexterity_kernel": "Dexterity",
        "flight_recorder": "EventLedger",
        "resource_scope": context_bundle_resource_scope_payload(scope),
        "record": record,
    })
}

fn diagnostic_tier_event_payload(
    record: &ModelLaneDiagnosticTierStatusRecord,
    scope: ScopeColumnValues<'_>,
) -> Value {
    json!({
        "schema_id": "hsk.model_lane_diagnostic_tier@1",
        "dexterity_kernel": "Dexterity",
        "flight_recorder": "EventLedger",
        "resource_scope": context_bundle_resource_scope_payload(scope),
        "record": record,
    })
}

fn mt_runtime_status_event_payload(
    record: &ModelLaneMtRuntimeStatusRecord,
    scope: ScopeColumnValues<'_>,
) -> Value {
    json!({
        "schema_id": "hsk.model_lane_mt_runtime_status@1",
        "dexterity_kernel": "Dexterity",
        "flight_recorder": "EventLedger",
        "resource_scope": context_bundle_resource_scope_payload(scope),
        "record": record,
    })
}

fn parse_utc(field: &str, value: &str) -> ModelLaneResult<DateTime<Utc>> {
    require_token(field, value)?;
    DateTime::parse_from_rfc3339(value)
        .map(|date| date.with_timezone(&Utc))
        .map_err(|err| ModelLaneError::InvalidInput(format!("{field} must be RFC3339 UTC: {err}")))
}

fn ensure_object_payload(field: &str, payload: &Value) -> ModelLaneResult<()> {
    if payload.is_object() {
        Ok(())
    } else {
        Err(ModelLaneError::InvalidInput(format!(
            "{field} must be a JSON object"
        )))
    }
}

fn required_json_text(payload: &Value, field: &str) -> ModelLaneResult<String> {
    let value = payload
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or_default();
    require_token(field, value)?;
    Ok(value.to_string())
}

fn require_json_string(payload: &Value, field: &str, expected: &str) -> ModelLaneResult<()> {
    let actual = required_json_text(payload, field)?;
    require_equal(field, &actual, "expected", expected)
}

fn json_string(payload: &Value, field: &str) -> Option<String> {
    payload
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
}

fn merge_diagnostic_payload(mut base: Value, overlay: Value) -> Value {
    match (&mut base, overlay) {
        (Value::Object(base_map), Value::Object(overlay_map)) => {
            for (key, value) in overlay_map {
                base_map.insert(key, value);
            }
            base
        }
        (_, overlay) => overlay,
    }
}

fn is_cloud_lane(input: &NewModelLane) -> bool {
    input.runtime_binding == RuntimeBinding::Cloud
        || matches!(
            input.provider_kind,
            ModelLaneProviderKind::OpenAi | ModelLaneProviderKind::Anthropic
        )
}

fn is_cloud_lane_record(record: &ModelLaneRecord) -> bool {
    is_cloud_lane(&record.inner)
}

fn reject_hidden_provider_ref(field: &str, reference: &str) -> ModelLaneResult<()> {
    let normalized = reference.trim().to_ascii_lowercase();
    if normalized.starts_with("provider-session://") || normalized.starts_with("provider-memory://")
    {
        return Err(ModelLaneError::InvalidInput(format!(
            "{field} cannot use hidden provider/session memory"
        )));
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct ResolvedModelLaneCrdtAuthority {
    workspace_id: String,
    document_id: String,
    crdt_document_id: String,
    update_id: String,
    update_seq: i64,
    update_sha256: String,
    update_bytes_ref: String,
    actor_id: String,
    actor_kind: String,
    session_id: String,
    trace_id: String,
    state_vector_after: String,
    yjs_state_vector_b64: String,
    replay_metadata: Value,
    snapshot_bytes_ref: String,
    site_id: String,
    materialized_projection_hash: String,
    event_ledger_event_id: String,
}

#[derive(Debug, Clone)]
struct ResolvedModelLaneCrdtLeaseAuthority {
    lease_id: String,
    correlation_id: String,
    scope_kind: String,
    scope_id: String,
    claimed_at_utc: DateTime<Utc>,
    expires_at_utc: DateTime<Utc>,
    admitted_at_utc: DateTime<Utc>,
}

fn crdt_authority_denied(detail: impl Into<String>) -> ModelLaneError {
    ModelLaneError::AuthorityDenied(format!(
        "CX-MM-006 ModelLane CRDT authority resolution failed: {}",
        detail.into()
    ))
}

fn expected_crdt_actor_kind_for_lane(kind: &ModelLaneKind) -> &'static str {
    match kind {
        ModelLaneKind::LocalModel | ModelLaneKind::CliModel | ModelLaneKind::Subagent => {
            "local_model"
        }
        ModelLaneKind::CloudModel => "cloud_model",
        ModelLaneKind::HumanOperator => "operator",
        ModelLaneKind::Validator => "validator",
    }
}

async fn validate_crdt_lane_session_uniqueness_tx(
    tx: &mut Transaction<'_, Postgres>,
    lane: &ModelLaneRecord,
    resolved: &ResolvedModelLaneCrdtAuthority,
) -> ModelLaneResult<()> {
    let matching_lanes = sqlx::query(
        r#"
        SELECT lane_id, run_id
        FROM model_lanes
        WHERE record_json->>'session_id' = $1
           OR record_json->>'model_session_id' = $1
        ORDER BY run_id, lane_id
        FOR SHARE
        "#,
    )
    .bind(&resolved.session_id)
    .fetch_all(&mut **tx)
    .await?;
    if matching_lanes.len() != 1 {
        return Err(crdt_authority_denied(format!(
            "crdt session {} is not uniquely owned by one ModelLane",
            resolved.session_id
        )));
    }
    let owner_lane_id: String = matching_lanes[0].try_get("lane_id")?;
    let owner_run_id: String = matching_lanes[0].try_get("run_id")?;
    if owner_lane_id != lane.lane_id || owner_run_id != lane.run_id {
        return Err(crdt_authority_denied(format!(
            "crdt session {} belongs to run {} lane {}, not source run {} lane {}",
            resolved.session_id, owner_run_id, owner_lane_id, lane.run_id, lane.lane_id
        )));
    }
    Ok(())
}

fn crdt_lease_scope_covers_resolved_authority(
    scope_kind: &str,
    scope_id: &str,
    resolved: &ResolvedModelLaneCrdtAuthority,
) -> bool {
    match scope_kind {
        "workspace" => scope_id == resolved.workspace_id,
        // Knowledge rich-document write authority uses the CRDT document ID
        // as its typed document lease scope (see guard_lease_for_write).
        "document" => scope_id == resolved.crdt_document_id,
        _ => false,
    }
}

async fn resolve_active_crdt_actor_lane_lease_tx(
    tx: &mut Transaction<'_, Postgres>,
    lane: &ModelLaneRecord,
    resolved: &ResolvedModelLaneCrdtAuthority,
) -> ModelLaneResult<ResolvedModelLaneCrdtLeaseAuthority> {
    let admitted_at_utc: DateTime<Utc> = sqlx::query_scalar("SELECT statement_timestamp()")
        .fetch_one(&mut **tx)
        .await?;
    let leases = sqlx::query(
        r#"
        SELECT lease_id, correlation_id, scope_kind, scope_id,
               claimed_at_utc, expires_at_utc
        FROM knowledge_crdt_agent_lane_leases
        WHERE lane_id = $1
          AND actor_id = $2
          AND actor_kind = $3
          AND session_id = $4
          AND correlation_id = $5
          AND claimed_at_utc <= $6
          AND expires_at_utc > $6
          AND released_at_utc IS NULL
          AND (
                (scope_kind = 'workspace' AND scope_id = $7)
             OR (scope_kind = 'document' AND scope_id = $8)
          )
        ORDER BY claimed_at_utc, lease_id
        FOR SHARE
        "#,
    )
    .bind(&lane.lane_id)
    .bind(&resolved.actor_id)
    .bind(&resolved.actor_kind)
    .bind(&resolved.session_id)
    .bind(&resolved.trace_id)
    .bind(admitted_at_utc.clone())
    .bind(&resolved.workspace_id)
    .bind(&resolved.crdt_document_id)
    .fetch_all(&mut **tx)
    .await?;
    if leases.is_empty() {
        return Err(crdt_authority_denied(format!(
            "crdt actor {} session {} has no persisted knowledge-agent lease binding that is exact and active for source lane {}, trace {}, and CRDT document {}",
            resolved.actor_id,
            resolved.session_id,
            lane.lane_id,
            resolved.trace_id,
            resolved.crdt_document_id
        )));
    }
    if leases.len() != 1 {
        let lease_ids = leases
            .iter()
            .map(|row| row.try_get::<String, _>("lease_id"))
            .collect::<Result<Vec<_>, _>>()?;
        return Err(crdt_authority_denied(format!(
            "crdt actor {} session {} has ambiguous active knowledge-agent lease bindings {:?} to source lane {} for trace {} and CRDT document {}",
            resolved.actor_id,
            resolved.session_id,
            lease_ids,
            lane.lane_id,
            resolved.trace_id,
            resolved.crdt_document_id
        )));
    }
    let lease = &leases[0];
    Ok(ResolvedModelLaneCrdtLeaseAuthority {
        lease_id: lease.try_get("lease_id")?,
        correlation_id: lease.try_get("correlation_id")?,
        scope_kind: lease.try_get("scope_kind")?,
        scope_id: lease.try_get("scope_id")?,
        claimed_at_utc: lease.try_get("claimed_at_utc")?,
        expires_at_utc: lease.try_get("expires_at_utc")?,
        admitted_at_utc,
    })
}

async fn validate_historical_crdt_actor_lane_lease_tx(
    tx: &mut Transaction<'_, Postgres>,
    lane: &ModelLaneRecord,
    resolved: &ResolvedModelLaneCrdtAuthority,
    binding: &ModelLaneCrdtAuthorityBinding,
) -> ModelLaneResult<ResolvedModelLaneCrdtLeaseAuthority> {
    let lease = sqlx::query(
        r#"
        SELECT lane_id, actor_id, actor_kind, session_id, correlation_id,
               scope_kind, scope_id, claimed_at_utc, expires_at_utc,
               released_at_utc
        FROM knowledge_crdt_agent_lane_leases
        WHERE lease_id = $1
        FOR SHARE
        "#,
    )
    .bind(&binding.lease_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| {
        crdt_authority_denied(format!(
            "replayed CRDT lease {} no longer resolves to persisted authority",
            binding.lease_id
        ))
    })?;

    let lease_lane_id: String = lease.try_get("lane_id")?;
    let lease_actor_id: String = lease.try_get("actor_id")?;
    let lease_actor_kind: String = lease.try_get("actor_kind")?;
    let lease_session_id: String = lease.try_get("session_id")?;
    let lease_correlation_id: String = lease.try_get("correlation_id")?;
    let lease_scope_kind: String = lease.try_get("scope_kind")?;
    let lease_scope_id: String = lease.try_get("scope_id")?;
    let lease_claimed_at_utc: DateTime<Utc> = lease.try_get("claimed_at_utc")?;
    let current_lease_expires_at_utc: DateTime<Utc> = lease.try_get("expires_at_utc")?;
    let released_at_utc: Option<DateTime<Utc>> = lease.try_get("released_at_utc")?;

    let identity_matches = lease_lane_id == lane.lane_id
        && lease_actor_id == resolved.actor_id
        && lease_actor_kind == resolved.actor_kind
        && lease_session_id == resolved.session_id
        && lease_correlation_id == resolved.trace_id
        && lease_correlation_id == binding.lease_correlation_id
        && lease_scope_kind == binding.lease_scope_kind
        && lease_scope_id == binding.lease_scope_id
        && lease_claimed_at_utc == binding.lease_claimed_at_utc
        && current_lease_expires_at_utc >= binding.lease_expires_at_utc
        && crdt_lease_scope_covers_resolved_authority(&lease_scope_kind, &lease_scope_id, resolved);
    let historically_active = binding.lease_admitted_at_utc >= binding.lease_claimed_at_utc
        && binding.lease_admitted_at_utc < binding.lease_expires_at_utc
        && released_at_utc
            .map(|released| released > binding.lease_admitted_at_utc)
            .unwrap_or(true);
    if !identity_matches || !historically_active {
        return Err(crdt_authority_denied(format!(
            "replayed CRDT lease {} does not prove exact lane, actor, session, trace, scope, and active-at-admission authority",
            binding.lease_id
        )));
    }

    Ok(ResolvedModelLaneCrdtLeaseAuthority {
        lease_id: binding.lease_id.clone(),
        correlation_id: binding.lease_correlation_id.clone(),
        scope_kind: binding.lease_scope_kind.clone(),
        scope_id: binding.lease_scope_id.clone(),
        claimed_at_utc: binding.lease_claimed_at_utc.clone(),
        expires_at_utc: binding.lease_expires_at_utc.clone(),
        admitted_at_utc: binding.lease_admitted_at_utc.clone(),
    })
}

fn bind_crdt_authority_to_lane(
    message: &NewModelLaneMessage,
    lane: &ModelLaneRecord,
    resolved: &ResolvedModelLaneCrdtAuthority,
    lease: &ResolvedModelLaneCrdtLeaseAuthority,
) -> ModelLaneResult<ModelLaneCrdtAuthorityBinding> {
    let expected_actor_kind = expected_crdt_actor_kind_for_lane(&lane.kind);
    if resolved.actor_kind != expected_actor_kind {
        return Err(crdt_authority_denied(format!(
            "crdt actor_kind {} cannot be attributed to {} lane {}",
            resolved.actor_kind,
            lane.kind.as_str(),
            lane.lane_id
        )));
    }
    if resolved.session_id != lane.session_id && resolved.session_id != lane.model_session_id {
        return Err(crdt_authority_denied(format!(
            "crdt session {} is not owned by source lane {}",
            resolved.session_id, lane.lane_id
        )));
    }
    if !message
        .linked_span_contexts
        .iter()
        .any(|link| link == &resolved.trace_id)
    {
        return Err(crdt_authority_denied(format!(
            "message {} does not link the CRDT trace {}",
            message.message_id, resolved.trace_id
        )));
    }

    Ok(ModelLaneCrdtAuthorityBinding {
        run_id: message.run_id.clone(),
        lane_id: lane.lane_id.clone(),
        lane_session_id: lane.session_id.clone(),
        model_session_id: lane.model_session_id.clone(),
        lane_trace_id: lane.trace_id.clone(),
        crdt_session_id: resolved.session_id.clone(),
        crdt_trace_id: resolved.trace_id.clone(),
        workspace_id: resolved.workspace_id.clone(),
        document_id: resolved.document_id.clone(),
        crdt_document_id: resolved.crdt_document_id.clone(),
        actor_id: resolved.actor_id.clone(),
        actor_kind: resolved.actor_kind.clone(),
        lease_id: lease.lease_id.clone(),
        lease_correlation_id: lease.correlation_id.clone(),
        lease_scope_kind: lease.scope_kind.clone(),
        lease_scope_id: lease.scope_id.clone(),
        lease_claimed_at_utc: lease.claimed_at_utc.clone(),
        lease_expires_at_utc: lease.expires_at_utc.clone(),
        lease_admitted_at_utc: lease.admitted_at_utc.clone(),
        crdt_site_id: resolved.site_id.clone(),
        update_id: resolved.update_id.clone(),
        update_seq: resolved.update_seq,
        update_bytes_ref: resolved.update_bytes_ref.clone(),
        base_snapshot_ref: resolved.snapshot_bytes_ref.clone(),
        state_vector: resolved.state_vector_after.clone(),
        yjs_state_vector_b64: resolved.yjs_state_vector_b64.clone(),
        materialized_projection_hash: resolved.materialized_projection_hash.clone(),
        update_event_ledger_event_id: resolved.event_ledger_event_id.clone(),
        crdt_proposal_ref: message.crdt_proposal_ref.clone(),
    })
}

fn required_event_payload_string(
    payload: &Value,
    field: &str,
    authority_ref: &str,
) -> ModelLaneResult<String> {
    payload
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| {
            crdt_authority_denied(format!(
                "EventLedger payload for {authority_ref} is missing {field}"
            ))
        })
}

/// Reconcile the CRDT-taxonomy actor recorded on a CRDT authority row
/// (`kernel_crdt_updates` / `kernel_crdt_snapshots`) with the kernel-taxonomy
/// actor recorded on its EventLedger event.
///
/// CRDT authority rows persist `actor.kind().as_str()` (`operator`,
/// `local_model`, `cloud_model`, `validator`, `system`), while every EventLedger
/// event persists `event.actor.actor_kind()`, and `KnowledgeActorIdV1::to_kernel_actor`
/// projects `LocalModel`/`CloudModel` -> `model_adapter` and `Validator` ->
/// `validation_runner`. Comparing the two raw strings denies every model- or
/// validator-authored CRDT update even though the same actor authored both rows.
///
/// This verifies, fail-closed, that (1) the row's CRDT `actor_kind` is exactly
/// the kind encoded by its canonical `actor_id`, and (2) the EventLedger
/// `actor_kind` is exactly the kernel projection of that actor. The caller still
/// cross-checks `actor_id` verbatim between row and event, so actor identity is
/// fully preserved; only the redundant taxonomy label is compared in the correct
/// space.
fn reconcile_crdt_and_ledger_actor_kind(
    crdt_actor_id: &str,
    crdt_actor_kind: &str,
    ledger_actor_kind: &str,
    reference: &str,
) -> ModelLaneResult<()> {
    let actor = KnowledgeActorIdV1::parse(crdt_actor_id).map_err(|error| {
        crdt_authority_denied(format!(
            "{reference} actor_id {crdt_actor_id} is invalid: {error}"
        ))
    })?;
    if actor.kind().as_str() != crdt_actor_kind {
        return Err(crdt_authority_denied(format!(
            "{reference} actor_kind {crdt_actor_kind} does not match canonical actor_id {crdt_actor_id}"
        )));
    }
    let expected_ledger_actor_kind = actor.to_kernel_actor().actor_kind();
    if ledger_actor_kind != expected_ledger_actor_kind {
        return Err(crdt_authority_denied(format!(
            "{reference} EventLedger actor_kind {ledger_actor_kind} does not match kernel projection {expected_ledger_actor_kind} of CRDT actor {crdt_actor_id}"
        )));
    }
    Ok(())
}

async fn resolve_model_lane_crdt_authority_tx(
    tx: &mut Transaction<'_, Postgres>,
    update_bytes_ref: &str,
    base_snapshot_ref: &str,
    state_vector: &str,
) -> ModelLaneResult<ResolvedModelLaneCrdtAuthority> {
    let update_row = sqlx::query(
        r#"
        SELECT updates.schema_id,
               updates.workspace_id,
               updates.document_id,
               updates.crdt_document_id,
               updates.update_id,
               updates.update_seq,
               updates.update_sha256,
               updates.update_bytes_ref,
               updates.update_bytes,
               updates.actor_id,
               updates.actor_kind,
               updates.session_id,
               updates.trace_id,
               updates.state_vector_before,
               updates.state_vector_after,
               updates.replay_metadata_json,
               updates.event_ledger_stream_id,
               updates.event_ledger_event_id,
               updates.storage_authority,
               ledger.session_run_id AS ledger_session_run_id,
               ledger.event_type AS ledger_event_type,
               ledger.aggregate_type AS ledger_aggregate_type,
               ledger.aggregate_id AS ledger_aggregate_id,
               ledger.actor_kind AS ledger_actor_kind,
               ledger.actor_id AS ledger_actor_id,
               ledger.correlation_id AS ledger_correlation_id,
               ledger.payload_hash AS ledger_payload_hash,
               ledger.payload AS ledger_payload
        FROM kernel_crdt_updates updates
        JOIN kernel_event_ledger ledger
          ON ledger.event_id = updates.event_ledger_event_id
        WHERE updates.update_bytes_ref = $1
        FOR SHARE OF updates, ledger
        "#,
    )
    .bind(update_bytes_ref)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| {
        crdt_authority_denied(format!(
            "crdt_update_ref {update_bytes_ref} does not resolve to kernel_crdt_updates"
        ))
    })?;

    let update_schema_id: String = update_row.try_get("schema_id")?;
    let update_storage_authority: String = update_row.try_get("storage_authority")?;
    if update_schema_id != CRDT_UPDATE_RECORD_SCHEMA_ID
        || update_storage_authority != "postgres_event_ledger"
        || !update_bytes_ref.starts_with("postgres://")
    {
        return Err(crdt_authority_denied(format!(
            "crdt_update_ref {update_bytes_ref} has non-canonical schema/storage authority"
        )));
    }
    let update_bytes: Vec<u8> = update_row.try_get("update_bytes")?;
    let update_sha256: String = update_row.try_get("update_sha256")?;
    let computed_update_sha256 = dexterity_sha256_hex(&update_bytes);
    if computed_update_sha256 != update_sha256 {
        return Err(crdt_authority_denied(format!(
            "crdt_update_ref {update_bytes_ref} stored bytes hash {computed_update_sha256} does not match persisted update_sha256 {update_sha256}"
        )));
    }
    Update::decode_v1(&update_bytes).map_err(|error| {
        crdt_authority_denied(format!(
            "crdt_update_ref {update_bytes_ref} does not decode as a Yjs v1 update: {error}"
        ))
    })?;

    let workspace_id: String = update_row.try_get("workspace_id")?;
    let document_id: String = update_row.try_get("document_id")?;
    let crdt_document_id: String = update_row.try_get("crdt_document_id")?;
    let update_id: String = update_row.try_get("update_id")?;
    let update_seq: i64 = update_row.try_get("update_seq")?;
    let actor_id: String = update_row.try_get("actor_id")?;
    let actor_kind: String = update_row.try_get("actor_kind")?;
    let session_id: String = update_row.try_get("session_id")?;
    let trace_id: String = update_row.try_get("trace_id")?;
    let state_vector_before: String = update_row.try_get("state_vector_before")?;
    let state_vector_after: String = update_row.try_get("state_vector_after")?;
    let replay_metadata: Value = update_row.try_get("replay_metadata_json")?;
    let typed_replay_metadata: CrdtReplayMetadataV1 =
        serde_json::from_value(replay_metadata.clone()).map_err(|error| {
            crdt_authority_denied(format!(
                "crdt_update_ref {update_bytes_ref} has invalid replay metadata: {error}"
            ))
        })?;
    if typed_replay_metadata.encoding != "yjs-update-v1"
        || typed_replay_metadata.schema_version != "kernel-crdt-update-v1"
    {
        return Err(crdt_authority_denied(format!(
            "crdt_update_ref {update_bytes_ref} replay metadata is not canonical Yjs v1"
        )));
    }
    let target_record = CrdtUpdateRecordV1 {
        schema_id: update_schema_id.clone(),
        workspace_id: workspace_id.clone(),
        document_id: document_id.clone(),
        crdt_document_id: crdt_document_id.clone(),
        update_id: update_id.clone(),
        update_seq: u64::try_from(update_seq).map_err(|_| {
            crdt_authority_denied(format!(
                "crdt_update_ref {update_bytes_ref} has invalid update_seq {update_seq}"
            ))
        })?,
        update_sha256: update_sha256.clone(),
        update_bytes_ref: update_bytes_ref.to_string(),
        actor_id: actor_id.clone(),
        actor_kind: actor_kind.clone(),
        session_id: session_id.clone(),
        trace_id: trace_id.clone(),
        state_vector_before: state_vector_before.clone(),
        state_vector_after: state_vector_after.clone(),
        replay_metadata: typed_replay_metadata,
        event_ledger_stream_id: update_row.try_get("event_ledger_stream_id")?,
        event_ledger_event_id: update_row.try_get("event_ledger_event_id")?,
        storage_authority: CrdtStorageAuthorityPosture::PostgresEventLedger,
    };
    if let Err(errors) = validate_crdt_update_record(&target_record) {
        return Err(crdt_authority_denied(format!(
            "crdt_update_ref {update_bytes_ref} fails canonical update validation: {errors:?}"
        )));
    }
    if state_vector != state_vector_after {
        return Err(crdt_authority_denied(format!(
            "crdt_state_vector {state_vector} does not match persisted state_vector_after {state_vector_after} for {update_bytes_ref}"
        )));
    }

    let event_ledger_stream_id: String = update_row.try_get("event_ledger_stream_id")?;
    let event_ledger_event_id: String = update_row.try_get("event_ledger_event_id")?;
    let ledger_session_run_id: String = update_row.try_get("ledger_session_run_id")?;
    let ledger_event_type: String = update_row.try_get("ledger_event_type")?;
    let ledger_aggregate_type: String = update_row.try_get("ledger_aggregate_type")?;
    let ledger_aggregate_id: String = update_row.try_get("ledger_aggregate_id")?;
    let ledger_actor_kind: String = update_row.try_get("ledger_actor_kind")?;
    let ledger_actor_id: String = update_row.try_get("ledger_actor_id")?;
    let ledger_correlation_id: Option<String> = update_row.try_get("ledger_correlation_id")?;
    let ledger_payload_hash: String = update_row.try_get("ledger_payload_hash")?;
    let ledger_payload: Value = update_row.try_get("ledger_payload")?;
    let computed_payload_hash = dexterity_sha256_hex(&canonical_json_bytes(&ledger_payload));
    let expected_crdt_stream_id = format!("knowledge-crdt:{crdt_document_id}");
    reconcile_crdt_and_ledger_actor_kind(
        &actor_id,
        &actor_kind,
        &ledger_actor_kind,
        &format!("crdt_update_ref {update_bytes_ref}"),
    )?;
    if event_ledger_stream_id != expected_crdt_stream_id
        || session_id != ledger_session_run_id
        || ledger_event_type != "KNOWLEDGE_CRDT_UPDATE_RECORDED"
        || ledger_aggregate_type != "knowledge_crdt_document"
        || ledger_aggregate_id != crdt_document_id
        || ledger_actor_id != actor_id
        || ledger_correlation_id.as_deref() != Some(trace_id.as_str())
        || ledger_payload_hash != computed_payload_hash
    {
        return Err(crdt_authority_denied(format!(
            "crdt_update_ref {update_bytes_ref} disagrees with EventLedger event {event_ledger_event_id} identity or payload hash"
        )));
    }
    for (field, expected) in [
        ("update_id", update_id.as_str()),
        ("actor_id", actor_id.as_str()),
        ("update_sha256", update_sha256.as_str()),
        ("state_vector_before", state_vector_before.as_str()),
        ("state_vector_after", state_vector_after.as_str()),
    ] {
        let actual = required_event_payload_string(&ledger_payload, field, update_bytes_ref)?;
        if actual != expected {
            return Err(crdt_authority_denied(format!(
                "crdt_update_ref {update_bytes_ref} EventLedger payload {field}={actual} does not match persisted value {expected}"
            )));
        }
    }
    let ledger_update_seq = ledger_payload
        .get("update_seq")
        .and_then(Value::as_i64)
        .ok_or_else(|| {
            crdt_authority_denied(format!(
                "crdt_update_ref {update_bytes_ref} EventLedger payload is missing update_seq"
            ))
        })?;
    if ledger_update_seq != update_seq {
        return Err(crdt_authority_denied(format!(
            "crdt_update_ref {update_bytes_ref} EventLedger update_seq {ledger_update_seq} does not match persisted update_seq {update_seq}"
        )));
    }
    let site_id = required_event_payload_string(&ledger_payload, "site_id", update_bytes_ref)?;

    let snapshot_row = sqlx::query(
        r#"
        SELECT snapshots.schema_id,
               snapshots.snapshot_id,
               snapshots.workspace_id,
               snapshots.document_id,
               snapshots.crdt_document_id,
               snapshots.covered_update_seq,
               snapshots.state_vector,
               snapshots.snapshot_sha256,
               snapshots.snapshot_bytes_ref,
               snapshots.snapshot_bytes,
               snapshots.actor_id,
               snapshots.actor_kind,
               snapshots.event_ledger_stream_id,
               snapshots.event_ledger_event_id,
               snapshots.promotion_evidence_update_ids,
               snapshots.storage_authority,
               ledger.event_type AS ledger_event_type,
               ledger.aggregate_type AS ledger_aggregate_type,
               ledger.aggregate_id AS ledger_aggregate_id,
               ledger.actor_kind AS ledger_actor_kind,
               ledger.actor_id AS ledger_actor_id,
               ledger.payload_hash AS ledger_payload_hash,
               ledger.payload AS ledger_payload
        FROM kernel_crdt_snapshots snapshots
        JOIN kernel_event_ledger ledger
          ON ledger.event_id = snapshots.event_ledger_event_id
        WHERE snapshots.snapshot_bytes_ref = $1
        FOR SHARE OF snapshots, ledger
        "#,
    )
    .bind(base_snapshot_ref)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| {
        crdt_authority_denied(format!(
            "crdt_base_snapshot_ref {base_snapshot_ref} does not resolve to kernel_crdt_snapshots"
        ))
    })?;
    let snapshot_schema_id: String = snapshot_row.try_get("schema_id")?;
    let snapshot_id: String = snapshot_row.try_get("snapshot_id")?;
    let snapshot_storage_authority: String = snapshot_row.try_get("storage_authority")?;
    let snapshot_workspace_id: String = snapshot_row.try_get("workspace_id")?;
    let snapshot_document_id: String = snapshot_row.try_get("document_id")?;
    let snapshot_crdt_document_id: String = snapshot_row.try_get("crdt_document_id")?;
    let covered_update_seq: i64 = snapshot_row.try_get("covered_update_seq")?;
    let snapshot_state_vector: String = snapshot_row.try_get("state_vector")?;
    let snapshot_sha256: String = snapshot_row.try_get("snapshot_sha256")?;
    let snapshot_bytes_ref: String = snapshot_row.try_get("snapshot_bytes_ref")?;
    let snapshot_bytes: Vec<u8> = snapshot_row.try_get("snapshot_bytes")?;
    if workspace_id != snapshot_workspace_id
        || document_id != snapshot_document_id
        || crdt_document_id != snapshot_crdt_document_id
        || covered_update_seq >= update_seq
    {
        return Err(crdt_authority_denied(format!(
            "crdt_base_snapshot_ref {base_snapshot_ref} does not belong to the update entity or is not causally before update_seq {update_seq}"
        )));
    }
    if snapshot_schema_id != CRDT_SNAPSHOT_RECORD_SCHEMA_ID
        || snapshot_storage_authority != "postgres_event_ledger"
        || !snapshot_bytes_ref.starts_with("postgres://")
    {
        return Err(crdt_authority_denied(format!(
            "crdt_base_snapshot_ref {base_snapshot_ref} has non-canonical schema/storage authority"
        )));
    }
    let computed_snapshot_sha256 = dexterity_sha256_hex(&snapshot_bytes);
    if computed_snapshot_sha256 != snapshot_sha256 {
        return Err(crdt_authority_denied(format!(
            "crdt_base_snapshot_ref {base_snapshot_ref} stored bytes hash {computed_snapshot_sha256} does not match persisted snapshot_sha256 {snapshot_sha256}"
        )));
    }
    Update::decode_v1(&snapshot_bytes).map_err(|error| {
        crdt_authority_denied(format!(
            "crdt_base_snapshot_ref {base_snapshot_ref} does not decode as a Yjs v1 update: {error}"
        ))
    })?;

    let snapshot_actor_id: String = snapshot_row.try_get("actor_id")?;
    let snapshot_actor_kind: String = snapshot_row.try_get("actor_kind")?;
    let snapshot_event_stream_id: String = snapshot_row.try_get("event_ledger_stream_id")?;
    let snapshot_event_id: String = snapshot_row.try_get("event_ledger_event_id")?;
    let snapshot_promotion_evidence: Vec<String> =
        serde_json::from_value(snapshot_row.try_get::<Value, _>("promotion_evidence_update_ids")?)
            .map_err(|error| {
                crdt_authority_denied(format!(
            "crdt_base_snapshot_ref {base_snapshot_ref} has invalid promotion evidence: {error}"
        ))
            })?;
    let snapshot_record = CrdtSnapshotRecordV1 {
        schema_id: snapshot_schema_id.clone(),
        snapshot_id,
        workspace_id: snapshot_workspace_id.clone(),
        document_id: snapshot_document_id.clone(),
        crdt_document_id: snapshot_crdt_document_id.clone(),
        covered_update_seq: u64::try_from(covered_update_seq).map_err(|_| {
            crdt_authority_denied(format!(
                "crdt_base_snapshot_ref {base_snapshot_ref} has invalid covered_update_seq {covered_update_seq}"
            ))
        })?,
        state_vector: snapshot_state_vector.clone(),
        snapshot_sha256: snapshot_sha256.clone(),
        snapshot_bytes_ref: snapshot_bytes_ref.clone(),
        actor_id: snapshot_actor_id.clone(),
        actor_kind: snapshot_actor_kind.clone(),
        event_ledger_stream_id: snapshot_event_stream_id.clone(),
        event_ledger_event_id: snapshot_event_id.clone(),
        promotion_evidence_update_ids: snapshot_promotion_evidence,
        storage_authority: CrdtStorageAuthorityPosture::PostgresEventLedger,
    };
    if let Err(errors) = validate_crdt_snapshot_record(&snapshot_record) {
        return Err(crdt_authority_denied(format!(
            "crdt_base_snapshot_ref {base_snapshot_ref} fails canonical snapshot validation: {errors:?}"
        )));
    }
    let snapshot_aggregate_type: String = snapshot_row.try_get("ledger_aggregate_type")?;
    let snapshot_event_type: String = snapshot_row.try_get("ledger_event_type")?;
    let snapshot_aggregate_id: String = snapshot_row.try_get("ledger_aggregate_id")?;
    let snapshot_ledger_actor_kind: String = snapshot_row.try_get("ledger_actor_kind")?;
    let snapshot_ledger_actor_id: String = snapshot_row.try_get("ledger_actor_id")?;
    let snapshot_ledger_payload_hash: String = snapshot_row.try_get("ledger_payload_hash")?;
    let snapshot_ledger_payload: Value = snapshot_row.try_get("ledger_payload")?;
    let computed_snapshot_payload_hash =
        dexterity_sha256_hex(&canonical_json_bytes(&snapshot_ledger_payload));
    reconcile_crdt_and_ledger_actor_kind(
        &snapshot_actor_id,
        &snapshot_actor_kind,
        &snapshot_ledger_actor_kind,
        &format!("crdt_base_snapshot_ref {base_snapshot_ref}"),
    )?;
    if snapshot_event_stream_id != expected_crdt_stream_id
        || snapshot_event_type != "KNOWLEDGE_CRDT_SNAPSHOT_RECORDED"
        || snapshot_aggregate_type != "knowledge_crdt_document"
        || snapshot_aggregate_id != crdt_document_id
        || snapshot_actor_id != snapshot_ledger_actor_id
        || snapshot_ledger_payload_hash != computed_snapshot_payload_hash
    {
        return Err(crdt_authority_denied(format!(
            "crdt_base_snapshot_ref {base_snapshot_ref} disagrees with EventLedger event {snapshot_event_id} identity or payload hash"
        )));
    }
    for (field, expected) in [
        ("document_id", document_id.as_str()),
        ("state_vector", snapshot_state_vector.as_str()),
    ] {
        let actual =
            required_event_payload_string(&snapshot_ledger_payload, field, base_snapshot_ref)?;
        if actual != expected {
            return Err(crdt_authority_denied(format!(
                "crdt_base_snapshot_ref {base_snapshot_ref} EventLedger payload {field}={actual} does not match persisted value {expected}"
            )));
        }
    }
    let ledger_covered_update_seq = snapshot_ledger_payload
        .get("covered_update_seq")
        .and_then(Value::as_i64)
        .ok_or_else(|| {
            crdt_authority_denied(format!(
                "crdt_base_snapshot_ref {base_snapshot_ref} EventLedger payload is missing covered_update_seq"
            ))
        })?;
    if ledger_covered_update_seq != covered_update_seq {
        return Err(crdt_authority_denied(format!(
            "crdt_base_snapshot_ref {base_snapshot_ref} EventLedger covered_update_seq {ledger_covered_update_seq} does not match persisted covered_update_seq {covered_update_seq}"
        )));
    }

    let chain_rows = sqlx::query(
        r#"
        SELECT updates.schema_id,
               updates.workspace_id,
               updates.document_id,
               updates.crdt_document_id,
               updates.update_id,
               updates.update_seq,
               updates.update_sha256,
               updates.update_bytes_ref,
               updates.update_bytes,
               updates.actor_id,
               updates.actor_kind,
               updates.session_id,
               updates.trace_id,
               updates.state_vector_before,
               updates.state_vector_after,
               updates.replay_metadata_json,
               updates.event_ledger_stream_id,
               updates.event_ledger_event_id,
               updates.storage_authority,
               ledger.session_run_id AS ledger_session_run_id,
               ledger.event_type AS ledger_event_type,
               ledger.aggregate_type AS ledger_aggregate_type,
               ledger.aggregate_id AS ledger_aggregate_id,
               ledger.actor_kind AS ledger_actor_kind,
               ledger.actor_id AS ledger_actor_id,
               ledger.correlation_id AS ledger_correlation_id,
               ledger.payload_hash AS ledger_payload_hash,
               ledger.payload AS ledger_payload
        FROM kernel_crdt_updates updates
        JOIN kernel_event_ledger ledger
          ON ledger.event_id = updates.event_ledger_event_id
        WHERE updates.workspace_id = $1
          AND updates.document_id = $2
          AND updates.crdt_document_id = $3
          AND updates.update_seq > $4
          AND updates.update_seq <= $5
        ORDER BY updates.update_seq ASC
        FOR SHARE OF updates, ledger
        "#,
    )
    .bind(&workspace_id)
    .bind(&document_id)
    .bind(&crdt_document_id)
    .bind(covered_update_seq)
    .bind(update_seq)
    .fetch_all(&mut **tx)
    .await?;

    let replay_count = update_seq.checked_sub(covered_update_seq).ok_or_else(|| {
        crdt_authority_denied(format!(
            "invalid replay bounds snapshot={covered_update_seq} update={update_seq}"
        ))
    })?;
    let expected_chain_len = usize::try_from(replay_count).map_err(|_| {
        crdt_authority_denied(format!(
            "invalid replay bounds snapshot={covered_update_seq} update={update_seq}"
        ))
    })?;
    if chain_rows.len() != expected_chain_len {
        return Err(crdt_authority_denied(format!(
            "CRDT replay chain is not contiguous from snapshot seq {covered_update_seq} through update seq {update_seq}"
        )));
    }

    let mut derived_vector =
        KnowledgeStateVectorV1::parse(&snapshot_state_vector).map_err(|error| {
            crdt_authority_denied(format!(
                "crdt_base_snapshot_ref {base_snapshot_ref} has invalid state vector: {error}"
            ))
        })?;

    let mut seen_update_ids: BTreeSet<String> = sqlx::query_scalar(
        r#"
        SELECT update_id
        FROM kernel_crdt_updates
        WHERE workspace_id = $1
          AND document_id = $2
          AND crdt_document_id = $3
          AND update_seq <= $4
        ORDER BY update_seq ASC
        FOR SHARE
        "#,
    )
    .bind(&workspace_id)
    .bind(&document_id)
    .bind(&crdt_document_id)
    .bind(covered_update_seq)
    .fetch_all(&mut **tx)
    .await?
    .into_iter()
    .collect();

    // `yrs::Doc` and its transactions are intentionally thread-affine. Keep
    // every database suspension above this boundary so ModelLaneStore futures
    // remain Send and can be used directly by Axum handlers.
    let materialized = Doc::new();
    let decoded_snapshot = Update::decode_v1(&snapshot_bytes).map_err(|error| {
        crdt_authority_denied(format!(
            "cannot decode locked base snapshot {base_snapshot_ref}: {error}"
        ))
    })?;
    let decoded_snapshot_vector = decoded_snapshot.state_vector();
    materialized
        .transact_mut()
        .apply_update(decoded_snapshot)
        .map_err(|error| {
            crdt_authority_denied(format!(
                "cannot materialize locked base snapshot {base_snapshot_ref}: {error}"
            ))
        })?;
    let materialized_snapshot_vector = materialized.transact().state_vector();
    if materialized_snapshot_vector != decoded_snapshot_vector {
        return Err(crdt_authority_denied(format!(
            "crdt_base_snapshot_ref {base_snapshot_ref} decoded Yjs state vector does not match the materialized snapshot bytes"
        )));
    }

    for (offset, row) in chain_rows.into_iter().enumerate() {
        let chain_ref: String = row.try_get("update_bytes_ref")?;
        let chain_seq: i64 = row.try_get("update_seq")?;
        let expected_seq = covered_update_seq
            .checked_add(
                i64::try_from(offset)
                    .map_err(|_| crdt_authority_denied("CRDT replay chain offset exceeds i64"))?,
            )
            .and_then(|sequence| sequence.checked_add(1))
            .ok_or_else(|| crdt_authority_denied("CRDT replay sequence overflows i64"))?;
        if chain_seq != expected_seq {
            return Err(crdt_authority_denied(format!(
                "CRDT replay sequence gap: expected {expected_seq}, found {chain_seq}"
            )));
        }
        let chain_schema_id: String = row.try_get("schema_id")?;
        let chain_storage_authority: String = row.try_get("storage_authority")?;
        let chain_workspace_id: String = row.try_get("workspace_id")?;
        let chain_document_id: String = row.try_get("document_id")?;
        let chain_crdt_document_id: String = row.try_get("crdt_document_id")?;
        let chain_update_id: String = row.try_get("update_id")?;
        let chain_update_sha256: String = row.try_get("update_sha256")?;
        let chain_bytes: Vec<u8> = row.try_get("update_bytes")?;
        let chain_actor_id: String = row.try_get("actor_id")?;
        let chain_actor_kind: String = row.try_get("actor_kind")?;
        let chain_session_id: String = row.try_get("session_id")?;
        let chain_trace_id: String = row.try_get("trace_id")?;
        let chain_before: String = row.try_get("state_vector_before")?;
        let chain_after: String = row.try_get("state_vector_after")?;
        let chain_replay_json: Value = row.try_get("replay_metadata_json")?;
        let chain_replay: CrdtReplayMetadataV1 = serde_json::from_value(chain_replay_json)
            .map_err(|error| {
                crdt_authority_denied(format!(
                    "crdt_update_ref {chain_ref} has invalid replay metadata: {error}"
                ))
            })?;
        let chain_stream_id: String = row.try_get("event_ledger_stream_id")?;
        let chain_event_id: String = row.try_get("event_ledger_event_id")?;
        let chain_record = CrdtUpdateRecordV1 {
            schema_id: chain_schema_id,
            workspace_id: chain_workspace_id.clone(),
            document_id: chain_document_id.clone(),
            crdt_document_id: chain_crdt_document_id.clone(),
            update_id: chain_update_id.clone(),
            update_seq: u64::try_from(chain_seq).map_err(|_| {
                crdt_authority_denied(format!(
                    "crdt_update_ref {chain_ref} has invalid update_seq {chain_seq}"
                ))
            })?,
            update_sha256: chain_update_sha256.clone(),
            update_bytes_ref: chain_ref.clone(),
            actor_id: chain_actor_id.clone(),
            actor_kind: chain_actor_kind.clone(),
            session_id: chain_session_id.clone(),
            trace_id: chain_trace_id.clone(),
            state_vector_before: chain_before.clone(),
            state_vector_after: chain_after.clone(),
            replay_metadata: chain_replay.clone(),
            event_ledger_stream_id: chain_stream_id.clone(),
            event_ledger_event_id: chain_event_id.clone(),
            storage_authority: CrdtStorageAuthorityPosture::PostgresEventLedger,
        };
        if let Err(errors) = validate_crdt_update_record(&chain_record) {
            return Err(crdt_authority_denied(format!(
                "crdt_update_ref {chain_ref} fails canonical update validation: {errors:?}"
            )));
        }
        if chain_storage_authority != "postgres_event_ledger"
            || chain_record.schema_id != CRDT_UPDATE_RECORD_SCHEMA_ID
            || chain_replay.encoding != "yjs-update-v1"
            || chain_replay.schema_version != "kernel-crdt-update-v1"
            || chain_workspace_id != workspace_id
            || chain_document_id != document_id
            || chain_crdt_document_id != crdt_document_id
            || chain_stream_id != expected_crdt_stream_id
            || !chain_ref.starts_with("postgres://")
            || dexterity_sha256_hex(&chain_bytes) != chain_update_sha256
        {
            return Err(crdt_authority_denied(format!(
                "crdt_update_ref {chain_ref} has invalid schema, storage, entity, encoding, or bytes hash"
            )));
        }
        if chain_replay
            .dependency_update_ids
            .iter()
            .any(|dependency| !seen_update_ids.contains(dependency))
        {
            return Err(crdt_authority_denied(format!(
                "crdt_update_ref {chain_ref} has an unresolved causal dependency"
            )));
        }

        let ledger_session: String = row.try_get("ledger_session_run_id")?;
        let ledger_event_type: String = row.try_get("ledger_event_type")?;
        let ledger_aggregate_type: String = row.try_get("ledger_aggregate_type")?;
        let ledger_aggregate_id: String = row.try_get("ledger_aggregate_id")?;
        let ledger_actor_kind: String = row.try_get("ledger_actor_kind")?;
        let ledger_actor_id: String = row.try_get("ledger_actor_id")?;
        let ledger_correlation_id: Option<String> = row.try_get("ledger_correlation_id")?;
        let ledger_payload_hash: String = row.try_get("ledger_payload_hash")?;
        let ledger_payload: Value = row.try_get("ledger_payload")?;
        reconcile_crdt_and_ledger_actor_kind(
            &chain_actor_id,
            &chain_actor_kind,
            &ledger_actor_kind,
            &format!("crdt_update_ref {chain_ref}"),
        )?;
        if ledger_session != chain_session_id
            || ledger_event_type != "KNOWLEDGE_CRDT_UPDATE_RECORDED"
            || ledger_aggregate_type != "knowledge_crdt_document"
            || ledger_aggregate_id != crdt_document_id
            || ledger_actor_id != chain_actor_id
            || ledger_correlation_id.as_deref() != Some(chain_trace_id.as_str())
            || ledger_payload_hash != dexterity_sha256_hex(&canonical_json_bytes(&ledger_payload))
        {
            return Err(crdt_authority_denied(format!(
                "crdt_update_ref {chain_ref} disagrees with EventLedger event {chain_event_id}"
            )));
        }
        for (field, expected) in [
            ("update_id", chain_update_id.as_str()),
            ("actor_id", chain_actor_id.as_str()),
            ("update_sha256", chain_update_sha256.as_str()),
            ("state_vector_before", chain_before.as_str()),
            ("state_vector_after", chain_after.as_str()),
        ] {
            let actual = required_event_payload_string(&ledger_payload, field, &chain_ref)?;
            if actual != expected {
                return Err(crdt_authority_denied(format!(
                    "crdt_update_ref {chain_ref} EventLedger {field} mismatch"
                )));
            }
        }
        if ledger_payload.get("update_seq").and_then(Value::as_i64) != Some(chain_seq) {
            return Err(crdt_authority_denied(format!(
                "crdt_update_ref {chain_ref} EventLedger update_seq mismatch"
            )));
        }
        let chain_site_id = required_event_payload_string(&ledger_payload, "site_id", &chain_ref)?;
        let actor = KnowledgeActorIdV1::parse(&chain_actor_id).map_err(|error| {
            crdt_authority_denied(format!(
                "crdt_update_ref {chain_ref} actor_id is invalid: {error}"
            ))
        })?;
        let derived_site = derive_knowledge_site_id(&workspace_id, &crdt_document_id, &actor);
        if derived_site.site_id != chain_site_id || derived_vector.encode() != chain_before {
            return Err(crdt_authority_denied(format!(
                "crdt_update_ref {chain_ref} has wrong site attribution or stale state_vector_before"
            )));
        }
        derived_vector.increment(&chain_site_id);
        if derived_vector.encode() != chain_after {
            return Err(crdt_authority_denied(format!(
                "crdt_update_ref {chain_ref} state_vector_after is not server-derived"
            )));
        }
        let decoded_update = Update::decode_v1(&chain_bytes).map_err(|error| {
            crdt_authority_denied(format!(
                "crdt_update_ref {chain_ref} does not decode as Yjs v1: {error}"
            ))
        })?;
        let yjs_before = materialized.transact().state_vector();
        let decoded_update_vector = decoded_update.state_vector();
        if !decoded_update.extends(&yjs_before) {
            return Err(crdt_authority_denied(format!(
                "crdt_update_ref {chain_ref} does not advance the Yjs state vector derived from persisted bytes"
            )));
        }
        materialized
            .transact_mut()
            .apply_update(decoded_update)
            .map_err(|error| {
                crdt_authority_denied(format!(
                    "crdt_update_ref {chain_ref} cannot be materialized: {error}"
                ))
            })?;
        let yjs_after = materialized.transact().state_vector();
        if yjs_after.partial_cmp(&yjs_before) != Some(Ordering::Greater)
            || !matches!(
                yjs_after.partial_cmp(&decoded_update_vector),
                Some(Ordering::Equal | Ordering::Greater)
            )
        {
            return Err(crdt_authority_denied(format!(
                "crdt_update_ref {chain_ref} materialized Yjs state vector does not contain the decoded update clocks"
            )));
        }
        seen_update_ids.insert(chain_update_id);
    }

    if derived_vector.encode() != state_vector_after {
        return Err(crdt_authority_denied(format!(
            "crdt_update_ref {update_bytes_ref} final state vector is not derived from the locked replay chain"
        )));
    }
    let materialized_projection = materialized
        .transact()
        .encode_state_as_update_v1(&StateVector::default());
    let materialized_projection_hash = dexterity_sha256_hex(&materialized_projection);
    let yjs_state_vector_b64 = base64::engine::general_purpose::STANDARD
        .encode(materialized.transact().state_vector().encode_v1());

    Ok(ResolvedModelLaneCrdtAuthority {
        workspace_id,
        document_id,
        crdt_document_id,
        update_id,
        update_seq,
        update_sha256,
        update_bytes_ref: update_bytes_ref.to_string(),
        actor_id,
        actor_kind,
        session_id,
        trace_id,
        state_vector_after,
        yjs_state_vector_b64,
        replay_metadata,
        snapshot_bytes_ref,
        site_id,
        materialized_projection_hash,
        event_ledger_event_id,
    })
}

async fn validate_message_crdt_authority_tx(
    tx: &mut Transaction<'_, Postgres>,
    message: &NewModelLaneMessage,
) -> ModelLaneResult<Option<ResolvedModelLaneCrdtAuthority>> {
    let has_any_crdt_metadata = message.crdt_update_ref.is_some()
        || message.crdt_base_snapshot_ref.is_some()
        || message.crdt_state_vector.is_some()
        || message.crdt_proposal_ref.is_some()
        || message.crdt_stale_base_ref.is_some();
    let Some(update_bytes_ref) = message.crdt_update_ref.as_deref() else {
        if has_any_crdt_metadata {
            return Err(crdt_authority_denied(
                "partial CRDT metadata cannot be admitted without crdt_update_ref",
            ));
        }
        return Ok(None);
    };
    let base_snapshot_ref = message.crdt_base_snapshot_ref.as_deref().ok_or_else(|| {
        crdt_authority_denied(format!(
            "crdt_update_ref {update_bytes_ref} requires crdt_base_snapshot_ref"
        ))
    })?;
    let state_vector = message.crdt_state_vector.as_deref().ok_or_else(|| {
        crdt_authority_denied(format!(
            "crdt_update_ref {update_bytes_ref} requires crdt_state_vector"
        ))
    })?;
    if message.crdt_stale_base_ref.is_some() {
        return Err(crdt_authority_denied(format!(
            "crdt_update_ref {update_bytes_ref} cannot be admitted with crdt_stale_base_ref"
        )));
    }
    if message.kind == ModelLaneMessageKind::Proposal && message.crdt_proposal_ref.is_none() {
        return Err(crdt_authority_denied(format!(
            "Proposal message {} carrying crdt_update_ref requires a persisted crdt_proposal_ref",
            message.message_id
        )));
    }
    let resolved =
        resolve_model_lane_crdt_authority_tx(tx, update_bytes_ref, base_snapshot_ref, state_vector)
            .await?;

    if let Some(proposal_ref) = message.crdt_proposal_ref.as_deref() {
        let proposal_id = proposal_ref
            .strip_prefix("crdt-proposal://")
            .filter(|proposal_id| !proposal_id.is_empty() && !proposal_id.contains('/'))
            .ok_or_else(|| {
                crdt_authority_denied(format!(
                    "crdt_proposal_ref {proposal_ref} must use crdt-proposal://<proposal_id>"
                ))
            })?;
        let proposal = sqlx::query(
            r#"
            SELECT workspace_id, document_id, crdt_document_id,
                   actor_id, actor_kind, session_id, correlation_id,
                   review_state, diff_sha256, applied_update_id, applied_update_sha256
            FROM knowledge_crdt_ai_edit_proposals
            WHERE proposal_id = $1
            FOR SHARE
            "#,
        )
        .bind(proposal_id)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or_else(|| {
            crdt_authority_denied(format!(
                "crdt_proposal_ref {proposal_ref} does not resolve to a persisted AI edit proposal"
            ))
        })?;
        let proposal_workspace_id: String = proposal.try_get("workspace_id")?;
        let proposal_document_id: String = proposal.try_get("document_id")?;
        let proposal_crdt_document_id: String = proposal.try_get("crdt_document_id")?;
        let proposal_actor_id: String = proposal.try_get("actor_id")?;
        let proposal_actor_kind: String = proposal.try_get("actor_kind")?;
        let proposal_session_id: String = proposal.try_get("session_id")?;
        let proposal_correlation_id: String = proposal.try_get("correlation_id")?;
        let review_state: String = proposal.try_get("review_state")?;
        let proposal_diff_sha256: String = proposal.try_get("diff_sha256")?;
        let applied_update_id: Option<String> = proposal.try_get("applied_update_id")?;
        let applied_update_sha256: Option<String> = proposal.try_get("applied_update_sha256")?;
        // WP-1 MT-018: the proposal is bound to the referenced
        // `kernel_crdt_updates` row by IDENTITY. Combined with the
        // workspace/document/crdt_document equality above, `applied_update_id ==
        // resolved.update_id` pins the full four-column PRIMARY KEY of
        // `kernel_crdt_updates` (migration 0020), so the applied binding names
        // exactly one real persisted update. That row's own byte integrity
        // (`sha256(update_bytes) == update_sha256` plus `Update::decode_v1`) is
        // already proven by `resolve_model_lane_crdt_authority_tx` and is not
        // re-derived here.
        if proposal_workspace_id != resolved.workspace_id
            || proposal_document_id != resolved.document_id
            || proposal_crdt_document_id != resolved.crdt_document_id
            || proposal_actor_id != resolved.actor_id
            || proposal_actor_kind != resolved.actor_kind
            || proposal_session_id != resolved.session_id
            || proposal_correlation_id != resolved.trace_id
            || !matches!(review_state.as_str(), "approved" | "promoted")
            || applied_update_id.as_deref() != Some(resolved.update_id.as_str())
        {
            return Err(crdt_authority_denied(format!(
                "crdt_proposal_ref {proposal_ref} is not an approved applied proposal for update {}",
                resolved.update_id
            )));
        }
        // WP-1 MT-018: `applied_update_sha256` is the APPROVED-DIFF hash
        // (`sha256(serde_json::to_vec(applied_diff))`, migration 0192 CHECK
        // `applied_update_sha256 = diff_sha256`), NOT the Yjs-v1 binary hash
        // carried by `kernel_crdt_updates.update_sha256`. Requiring the two to
        // be equal was unsatisfiable and made every Proposal-kind CRDT message
        // un-admittable. The surviving invariant is INTERNAL CONSISTENCY of the
        // proposal row: its applied binding must still cite its own approved
        // diff. A distinct reason string keeps identity failure and
        // internal-consistency failure diagnosable apart.
        if applied_update_sha256.as_deref() != Some(proposal_diff_sha256.as_str()) {
            return Err(crdt_authority_denied(format!(
                "crdt_proposal_ref {proposal_ref} applied_update_sha256 does not match its own approved diff_sha256"
            )));
        }
    }

    Ok(Some(resolved))
}

async fn validate_crdt_handoff_authority_tx(
    tx: &mut Transaction<'_, Postgres>,
    crdt: &ModelLaneCrdtHandoffMetadata,
) -> ModelLaneResult<ResolvedModelLaneCrdtAuthority> {
    let resolved = resolve_model_lane_crdt_authority_tx(
        tx,
        &crdt.update_bytes_ref,
        &crdt.base_snapshot_ref,
        &crdt.state_vector,
    )
    .await?;
    for (field, actual, expected) in [
        (
            "crdt_payload.workspace_id",
            crdt.workspace_id.as_str(),
            resolved.workspace_id.as_str(),
        ),
        (
            "crdt_payload.document_id",
            crdt.document_id.as_str(),
            resolved.document_id.as_str(),
        ),
        (
            "crdt_payload.actor_id",
            crdt.actor_id.as_str(),
            resolved.actor_id.as_str(),
        ),
        (
            "crdt_payload.actor_kind",
            crdt.actor_kind.as_str(),
            resolved.actor_kind.as_str(),
        ),
        (
            "crdt_payload.crdt_site_id",
            crdt.crdt_site_id.as_str(),
            resolved.site_id.as_str(),
        ),
        (
            "crdt_payload.update_sha256",
            crdt.update_sha256.as_str(),
            resolved.update_sha256.as_str(),
        ),
        (
            "crdt_payload.materialized_projection_hash",
            crdt.materialized_projection_hash.as_str(),
            resolved.materialized_projection_hash.as_str(),
        ),
    ] {
        if actual != expected {
            return Err(crdt_authority_denied(format!(
                "{field}={actual} does not match persisted CRDT authority {expected}"
            )));
        }
    }
    if crdt.update_seq != resolved.update_seq {
        return Err(crdt_authority_denied(format!(
            "crdt_payload.update_seq={} does not match persisted update_seq {}",
            crdt.update_seq, resolved.update_seq
        )));
    }
    let declared_format = crdt
        .replay_metadata
        .get("format")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if declared_format != "yjs_update_v1"
        || resolved
            .replay_metadata
            .get("encoding")
            .and_then(Value::as_str)
            != Some("yjs-update-v1")
    {
        return Err(crdt_authority_denied(
            "crdt_payload replay metadata disagrees with persisted Yjs v1 encoding",
        ));
    }
    for (field, actual, expected) in [
        (
            "crdt_payload.replay_metadata.replay_order_key",
            crdt.replay_metadata
                .get("replay_order_key")
                .and_then(Value::as_str),
            resolved
                .replay_metadata
                .get("replay_order_key")
                .and_then(Value::as_str),
        ),
        (
            "crdt_payload.replay_metadata.schema_version",
            crdt.replay_metadata
                .get("schema_version")
                .and_then(Value::as_str),
            resolved
                .replay_metadata
                .get("schema_version")
                .and_then(Value::as_str),
        ),
    ] {
        if actual.is_none() || actual != expected {
            return Err(crdt_authority_denied(format!(
                "{field} does not match persisted replay authority"
            )));
        }
    }
    let declared_dependencies = crdt
        .replay_metadata
        .get("dependency_update_ids")
        .and_then(Value::as_array);
    let persisted_dependencies = resolved
        .replay_metadata
        .get("dependency_update_ids")
        .and_then(Value::as_array);
    if declared_dependencies.is_none() || declared_dependencies != persisted_dependencies {
        return Err(crdt_authority_denied(
            "crdt_payload.replay_metadata.dependency_update_ids does not match persisted replay authority",
        ));
    }
    let expected_validation_runner_ref =
        format!("eventledger://{}", resolved.event_ledger_event_id);
    if crdt.validation_runner_ref != expected_validation_runner_ref {
        return Err(crdt_authority_denied(format!(
            "crdt_payload.validation_runner_ref={} does not resolve to persisted validation evidence {}",
            crdt.validation_runner_ref, expected_validation_runner_ref
        )));
    }
    Ok(resolved)
}

async fn resolve_promotion_input_refs_tx(
    tx: &mut Transaction<'_, Postgres>,
    access: &ResourceAccessContext,
    run_id: &str,
    input_refs: &[String],
    selected_input_refs: &[String],
) -> ModelLaneResult<PromotionInputResolution> {
    let mut records_by_ref = BTreeMap::new();
    let mut denial_reason = None;
    for reference in input_refs {
        let message_id = message_id_from_ref("input_refs[]", reference)?;
        let resolved_message = match promotion_message_by_id_tx(tx, access, &message_id).await {
            Ok(record) => record,
            Err(ModelLaneError::AuthorityDenied(_)) | Err(ModelLaneError::InvalidInput(_)) => {
                denial_reason =
                    denial_reason.or(Some(ModelLanePromotionDenialReason::InputRefMismatch));
                continue;
            }
            Err(error) => return Err(error),
        };
        match resolved_message {
            Some(record) if record.run_id == run_id => {
                if !matches!(
                    record.authority,
                    ModelLaneAuthority::Advisory | ModelLaneAuthority::PromotionCandidate
                ) {
                    denial_reason =
                        denial_reason.or(Some(ModelLanePromotionDenialReason::InputRefMismatch));
                }
                records_by_ref.insert(reference.clone(), record);
            }
            Some(_) | None => {
                denial_reason =
                    denial_reason.or(Some(ModelLanePromotionDenialReason::InputRefMismatch));
            }
        }
    }

    let mut selected_records = Vec::new();
    for reference in selected_input_refs {
        if let Some(record) = records_by_ref.get(reference) {
            selected_records.push(record.clone());
        } else {
            denial_reason =
                denial_reason.or(Some(ModelLanePromotionDenialReason::InputRefMismatch));
        }
    }
    if selected_records.is_empty() {
        denial_reason = denial_reason.or(Some(ModelLanePromotionDenialReason::InputRefMismatch));
    }

    let mut current_base_snapshot_ref: Option<String> = None;
    let mut current_state_vector: Option<String> = None;
    for record in &selected_records {
        let resolved = match validate_stored_message_eventledger_authority_tx(
            tx,
            record,
            access.exact_read_scope(),
        )
        .await
        {
            Ok(resolved) => resolved,
            Err(ModelLaneError::AuthorityDenied(_)) | Err(ModelLaneError::InvalidInput(_)) => {
                denial_reason =
                    denial_reason.or(Some(ModelLanePromotionDenialReason::InputRefMismatch));
                continue;
            }
            Err(error) => return Err(error),
        };
        let Some(resolved) = resolved else {
            // Nonshared advisory output is promotable as an artifact/decision,
            // but it must not manufacture CRDT snapshot or vector lineage.
            continue;
        };
        let base_snapshot_ref = resolved.snapshot_bytes_ref;
        let state_vector = resolved.state_vector_after;
        if current_base_snapshot_ref
            .as_deref()
            .is_some_and(|current| current != base_snapshot_ref)
            || current_state_vector
                .as_deref()
                .is_some_and(|current| current != state_vector)
        {
            denial_reason =
                denial_reason.or(Some(ModelLanePromotionDenialReason::InputRefMismatch));
        }
        current_base_snapshot_ref.get_or_insert(base_snapshot_ref);
        current_state_vector.get_or_insert(state_vector);
    }

    selected_records.sort_by(|left, right| {
        left.event_ledger_seq
            .cmp(&right.event_ledger_seq)
            .then_with(|| left.message_id.cmp(&right.message_id))
    });
    Ok(PromotionInputResolution {
        denial_reason,
        current_base_snapshot_ref,
        current_state_vector,
        selected_message_ids: selected_records
            .into_iter()
            .map(|record| record.message_id.clone())
            .collect(),
    })
}

fn message_id_from_ref(field: &str, reference: &str) -> ModelLaneResult<String> {
    require_token(field, reference)?;
    let message_id = reference
        .strip_prefix("model-lane-message://")
        .ok_or_else(|| {
            ModelLaneError::InvalidInput(format!(
                "{field} must use model-lane-message://<message_id>"
            ))
        })?;
    require_token(field, message_id)?;
    Ok(message_id.to_string())
}

fn missing_promoted_artifact_binding(input: &NewModelLanePromotionDecision) -> bool {
    input.promoted_artifact_ref.is_none()
        || input.promoted_artifact_sha256.is_none()
        || input.promoted_artifact_version.is_none()
}

fn promotion_state_history(outcome: ModelLanePromotionOutcome) -> Vec<ModelLanePromotionState> {
    match outcome {
        ModelLanePromotionOutcome::Approved => vec![
            ModelLanePromotionState::Advisory,
            ModelLanePromotionState::PromotionRequested,
            ModelLanePromotionState::PendingPolicy,
            ModelLanePromotionState::PendingApproval,
            ModelLanePromotionState::Approved,
            ModelLanePromotionState::Executing,
            ModelLanePromotionState::Executed,
        ],
        ModelLanePromotionOutcome::Denied => vec![
            ModelLanePromotionState::Advisory,
            ModelLanePromotionState::PromotionRequested,
            ModelLanePromotionState::PendingPolicy,
            ModelLanePromotionState::Denied,
        ],
    }
}

fn promotion_canonical_hash_basis(
    input: &NewModelLanePromotionDecision,
    outcome: ModelLanePromotionOutcome,
    final_state: ModelLanePromotionState,
    denial_reason: Option<ModelLanePromotionDenialReason>,
    current_event_ledger_version: Option<i64>,
    current_schema_id: Option<&str>,
    exact_scope: &ExactResourceScopeAttribution,
) -> Value {
    json!({
        "schema_id": "hsk.model_lane_promotion_decision@1",
        "resource_scope": exact_scope,
        "run_id": &input.run_id,
        "trace_id": &input.trace_id,
        "coordinator_session_id": &input.coordinator_session_id,
        "routing_policy": input.routing_policy.as_str(),
        "input_refs": &input.input_refs,
        "selected_input_refs": &input.selected_input_refs,
        "rejected_input_refs": &input.rejected_input_refs,
        "validator_authority_ref": &input.validator_authority_ref,
        "operator_authority_ref": &input.operator_authority_ref,
        "expected_event_ledger": {
            "aggregate_type": &input.expected_event_ledger_aggregate_type,
            "aggregate_id": &input.expected_event_ledger_aggregate_id,
            "version": input.expected_event_ledger_version,
            "current_version": current_event_ledger_version,
        },
        "crdt": {
            "base_snapshot_ref": &input.base_snapshot_ref,
            "current_base_snapshot_ref": &input.current_base_snapshot_ref,
            "state_vector": &input.state_vector,
            "current_state_vector": &input.current_state_vector,
        },
        "schema_guard": {
            "expected_schema_id": &input.schema_id,
            "current_schema_id": current_schema_id,
        },
        "deterministic_tie_break_rule": &input.deterministic_tie_break_rule,
        "promotion_gate_ref": &input.promotion_gate_ref,
        "promotion_receipt_ref": &input.promotion_receipt_ref,
        "promoted_artifact": {
            "ref": &input.promoted_artifact_ref,
            "sha256": &input.promoted_artifact_sha256,
            "version": &input.promoted_artifact_version,
        },
        "direct_authority_mutation_attempt_ref": &input.direct_authority_mutation_attempt_ref,
        "outcome": outcome.as_str(),
        "final_state": final_state.as_str(),
        "denial_reason": denial_reason.map(|reason| reason.as_str()),
    })
}

fn validate_run(input: &NewModelLaneRun) -> ModelLaneResult<()> {
    require_token("run_id", &input.run_id)?;
    require_token("trace_id", &input.trace_id)?;
    require_token("run_span_id", &input.run_span_id)?;
    require_token("coordinator_session_id", &input.coordinator_session_id)?;
    require_token("context_bundle_id", &input.context_bundle_id)?;
    require_token("event_ledger_stream_id", &input.event_ledger_stream_id)?;
    require_token("artifact_namespace", &input.artifact_namespace)?;
    require_token("owner_session", &input.owner_session)?;
    require_token("idempotency_key", &input.idempotency_key)?;
    require_token("replay_order_key", &input.replay_order_key)?;
    let work_packet_id = require_optional_token("work_packet_id", input.work_packet_id.as_deref())?;
    let micro_task_id = require_optional_token("micro_task_id", input.micro_task_id.as_deref())?;
    let task_board_id = require_optional_token("task_board_id", input.task_board_id.as_deref())?;
    require_token("memory_pack_ref", &input.memory_pack_ref)?;
    validate_sha256("memory_pack_hash", &input.memory_pack_hash)?;
    require_token("determinism_mode", &input.determinism_mode)?;
    require_token("budget_summary_ref", &input.budget_summary_ref)?;
    require_token("procedural_review_status", &input.procedural_review_status)?;
    if input.candidate_model_ids.is_empty() {
        return Err(ModelLaneError::InvalidInput(
            "candidate_model_ids must contain at least one model id".into(),
        ));
    }
    if input.lane_ids.is_empty() {
        return Err(ModelLaneError::InvalidInput(
            "lane_ids must contain at least one lane".into(),
        ));
    }
    let locus = validate_locus(input.locus_binding.as_ref(), "run")?;
    validate_locus_common(
        locus,
        &work_packet_id,
        &micro_task_id,
        Some(&task_board_id),
        &input.coordinator_session_id,
        &input.owner_session,
    )?;
    Ok(())
}

fn validate_lane(input: &NewModelLane) -> ModelLaneResult<()> {
    require_token("lane_id", &input.lane_id)?;
    require_token("run_id", &input.run_id)?;
    require_token("trace_id", &input.trace_id)?;
    require_token("lane_span_id", &input.lane_span_id)?;
    require_token("event_ledger_stream_id", &input.event_ledger_stream_id)?;
    require_token("role", &input.role)?;
    require_token("backend", &input.backend)?;
    require_token("session_id", &input.session_id)?;
    require_token("model_session_id", &input.model_session_id)?;
    require_token("adapter_id", &input.adapter_id)?;
    require_token("owner_session", &input.owner_session)?;
    if input.restart_generation < 0 {
        return Err(ModelLaneError::InvalidInput(
            "restart_generation must be non-negative".into(),
        ));
    }
    let work_packet_id = require_optional_token("work_packet_id", input.work_packet_id.as_deref())?;
    let micro_task_id = require_optional_token("micro_task_id", input.micro_task_id.as_deref())?;
    let task_board_id = require_optional_token("task_board_id", input.task_board_id.as_deref())?;
    validate_lane_runtime_contract(input)?;
    let locus = validate_locus(input.locus_binding.as_ref(), "lane")?;
    validate_locus_common(
        locus,
        &work_packet_id,
        &micro_task_id,
        Some(&task_board_id),
        &locus.coordinator_session_id,
        &input.owner_session,
    )?;
    require_equal(
        "locus.session_id",
        &locus.session_id,
        "lane.session_id",
        &input.session_id,
    )?;
    require_equal(
        "locus.model_session_id",
        &locus.model_session_id,
        "lane.model_session_id",
        &input.model_session_id,
    )?;
    Ok(())
}

fn validate_prepared_launch_pair(
    run: &NewModelLaneRun,
    lane: &NewModelLane,
) -> ModelLaneResult<()> {
    require_equal("lane.run_id", &lane.run_id, "run.run_id", &run.run_id)?;
    if !run.lane_ids.iter().any(|id| id == &lane.lane_id) {
        return Err(ModelLaneError::InvalidInput(format!(
            "run.lane_ids must include lane.lane_id {}",
            lane.lane_id
        )));
    }
    require_equal(
        "lane.trace_id",
        &lane.trace_id,
        "run.trace_id",
        &run.trace_id,
    )?;
    require_equal(
        "lane.event_ledger_stream_id",
        &lane.event_ledger_stream_id,
        "run.event_ledger_stream_id",
        &run.event_ledger_stream_id,
    )?;
    require_equal(
        "lane.owner_session",
        &lane.owner_session,
        "run.owner_session",
        &run.owner_session,
    )?;
    require_equal(
        "lane.work_packet_id",
        lane.work_packet_id.as_deref().unwrap_or(""),
        "run.work_packet_id",
        run.work_packet_id.as_deref().unwrap_or(""),
    )?;
    require_equal(
        "lane.micro_task_id",
        lane.micro_task_id.as_deref().unwrap_or(""),
        "run.micro_task_id",
        run.micro_task_id.as_deref().unwrap_or(""),
    )?;
    require_equal(
        "lane.task_board_id",
        lane.task_board_id.as_deref().unwrap_or(""),
        "run.task_board_id",
        run.task_board_id.as_deref().unwrap_or(""),
    )?;
    Ok(())
}

fn validate_message(input: &NewModelLaneMessage) -> ModelLaneResult<()> {
    require_token("message_id", &input.message_id)?;
    require_token("run_id", &input.run_id)?;
    require_token("trace_id", &input.trace_id)?;
    require_token("message_span_id", &input.message_span_id)?;
    require_token("from_lane_id", &input.from_lane_id)?;
    require_token("payload_ref", &input.payload_ref)?;
    reject_hidden_provider_ref("payload_ref", &input.payload_ref)?;
    validate_sha256("payload_sha256", &input.payload_sha256)?;
    require_token("event_ledger_stream_id", &input.event_ledger_stream_id)?;
    require_token("coordinator_session_id", &input.coordinator_session_id)?;
    require_token("owner_session", &input.owner_session)?;
    require_token("idempotency_key", &input.idempotency_key)?;
    require_token("replay_order_key", &input.replay_order_key)?;
    require_token("created_at_utc", &input.created_at_utc)?;
    let work_packet_id = require_optional_token("work_packet_id", input.work_packet_id.as_deref())?;
    let micro_task_id = require_optional_token("micro_task_id", input.micro_task_id.as_deref())?;
    let task_board_id = require_optional_token("task_board_id", input.task_board_id.as_deref())?;
    validate_message_trace(input)?;
    validate_message_routing(input)?;
    for (field, value) in [
        ("proposal_ref", input.proposal_ref.as_deref()),
        ("crdt_update_ref", input.crdt_update_ref.as_deref()),
        (
            "crdt_base_snapshot_ref",
            input.crdt_base_snapshot_ref.as_deref(),
        ),
        ("crdt_proposal_ref", input.crdt_proposal_ref.as_deref()),
        ("crdt_stale_base_ref", input.crdt_stale_base_ref.as_deref()),
        (
            "promoted_artifact_ref",
            input.promoted_artifact_ref.as_deref(),
        ),
    ] {
        if let Some(reference) = value {
            reject_hidden_provider_ref(field, reference)?;
        }
    }
    validate_message_authority(input)?;
    let locus = validate_locus(input.locus_binding.as_ref(), "message")?;
    validate_locus_common(
        locus,
        &work_packet_id,
        &micro_task_id,
        Some(&task_board_id),
        &input.coordinator_session_id,
        &input.owner_session,
    )?;
    Ok(())
}

fn validate_promotion_decision(input: &NewModelLanePromotionDecision) -> ModelLaneResult<()> {
    require_token("decision_id", &input.decision_id)?;
    require_token("run_id", &input.run_id)?;
    require_token("trace_id", &input.trace_id)?;
    require_token("decision_span_id", &input.decision_span_id)?;
    require_token("coordinator_session_id", &input.coordinator_session_id)?;
    require_token(
        "expected_event_ledger_aggregate_type",
        &input.expected_event_ledger_aggregate_type,
    )?;
    require_token(
        "expected_event_ledger_aggregate_id",
        &input.expected_event_ledger_aggregate_id,
    )?;
    if input.expected_event_ledger_version <= 0 {
        return Err(ModelLaneError::InvalidInput(
            "expected_event_ledger_version must be positive".into(),
        ));
    }
    require_token("base_snapshot_ref", &input.base_snapshot_ref)?;
    require_token(
        "current_base_snapshot_ref",
        &input.current_base_snapshot_ref,
    )?;
    require_token("state_vector", &input.state_vector)?;
    require_token("current_state_vector", &input.current_state_vector)?;
    require_token("schema_id", &input.schema_id)?;
    require_token(
        "deterministic_tie_break_rule",
        &input.deterministic_tie_break_rule,
    )?;
    require_token("promotion_gate_ref", &input.promotion_gate_ref)?;
    require_optional_token(
        "promotion_receipt_ref",
        input.promotion_receipt_ref.as_deref(),
    )?;
    require_token("event_ledger_stream_id", &input.event_ledger_stream_id)?;
    require_token("owner_session", &input.owner_session)?;
    require_token("idempotency_key", &input.idempotency_key)?;
    require_token("replay_order_key", &input.replay_order_key)?;
    require_token("created_at_utc", &input.created_at_utc)?;
    require_optional_token("work_packet_id", input.work_packet_id.as_deref())?;
    require_optional_token("micro_task_id", input.micro_task_id.as_deref())?;
    require_optional_token("task_board_id", input.task_board_id.as_deref())?;
    require_optional_token("recovery_hint_ref", input.recovery_hint_ref.as_deref())?;
    if let Some(validator_ref) = input.validator_authority_ref.as_deref() {
        require_token("validator_authority_ref", validator_ref)?;
    }
    if let Some(operator_ref) = input.operator_authority_ref.as_deref() {
        require_token("operator_authority_ref", operator_ref)?;
    }
    let routing_authority = super::routing::ModelLaneRoutingAuthority {
        cloud_consent_receipt_ref: input
            .diagnostic_payload
            .get("cloud_consent_receipt_ref")
            .and_then(Value::as_str)
            .map(str::to_string),
        validator_authority_ref: input.validator_authority_ref.clone(),
        operator_authority_ref: input.operator_authority_ref.clone(),
    };
    super::routing::ModelLaneRoutingGraph::for_policy(input.routing_policy)
        .require_authority_contract(&routing_authority)
        .map_err(|error| ModelLaneError::InvalidInput(error.to_string()))?;
    let parent_span_id = require_optional_token("parent_span_id", input.parent_span_id.as_deref())?;
    if parent_span_id == input.decision_span_id {
        return Err(ModelLaneError::InvalidInput(
            "parent_span_id must not equal decision_span_id".into(),
        ));
    }
    if input.linked_span_contexts.is_empty() {
        return Err(ModelLaneError::InvalidInput(
            "linked_span_contexts must include at least one span".into(),
        ));
    }
    for linked in &input.linked_span_contexts {
        require_token("linked_span_contexts[]", linked)?;
        if linked == &input.decision_span_id {
            return Err(ModelLaneError::InvalidInput(
                "linked_span_contexts must not include decision_span_id".into(),
            ));
        }
    }
    if input.input_refs.is_empty() {
        return Err(ModelLaneError::InvalidInput(
            "input_refs must contain at least one advisory input".into(),
        ));
    }
    if input.selected_input_refs.is_empty() {
        return Err(ModelLaneError::InvalidInput(
            "selected_input_refs must contain at least one advisory input".into(),
        ));
    }
    for reference in &input.input_refs {
        require_token("input_refs[]", reference)?;
    }
    for reference in &input.selected_input_refs {
        require_token("selected_input_refs[]", reference)?;
    }
    for reference in &input.rejected_input_refs {
        require_token("rejected_input_refs[]", reference)?;
    }
    if let Some(attempt_ref) = input.direct_authority_mutation_attempt_ref.as_deref() {
        require_token("direct_authority_mutation_attempt_ref", attempt_ref)?;
    }
    if let Some(artifact_ref) = input.promoted_artifact_ref.as_deref() {
        require_token("promoted_artifact_ref", artifact_ref)?;
        reject_hidden_provider_ref("promoted_artifact_ref", artifact_ref)?;
    }
    if let Some(artifact_sha256) = input.promoted_artifact_sha256.as_deref() {
        validate_sha256("promoted_artifact_sha256", artifact_sha256)?;
    }
    if let Some(artifact_version) = input.promoted_artifact_version.as_deref() {
        require_token("promoted_artifact_version", artifact_version)?;
    }
    Ok(())
}

fn validate_context_bundle_artifact_binding(
    input: &NewModelLaneContextBundleArtifactBinding,
) -> ModelLaneResult<()> {
    require_token("artifact_binding_id", &input.artifact_binding_id)?;
    require_token("run_id", &input.run_id)?;
    require_token("trace_id", &input.trace_id)?;
    require_token("artifact_ref", &input.artifact_ref)?;
    validate_sha256("artifact_sha256", &input.artifact_sha256)?;
    validate_sha256("content_hash", &input.content_hash)?;
    require_equal(
        "artifact_sha256",
        &input.artifact_sha256,
        "content_hash",
        &input.content_hash,
    )?;
    require_token("artifact_kind", &input.artifact_kind)?;
    require_token("artifact_manifest_ref", &input.artifact_manifest_ref)?;
    require_token("artifact_payload_ref", &input.artifact_payload_ref)?;
    require_equal(
        "artifact_ref",
        &input.artifact_ref,
        "artifact_payload_ref",
        &input.artifact_payload_ref,
    )?;
    let payload_hash = dexterity_sha256_hex(canonical_json_bytes(&input.payload_json));
    require_equal(
        "payload_json sha256",
        &payload_hash,
        "content_hash",
        &input.content_hash,
    )?;
    require_token("event_ledger_stream_id", &input.event_ledger_stream_id)?;
    require_token("work_packet_id", &input.work_packet_id)?;
    require_token("micro_task_id", &input.micro_task_id)?;
    require_token("task_board_id", &input.task_board_id)?;
    require_token("owner_session", &input.owner_session)?;
    require_token("idempotency_key", &input.idempotency_key)?;
    require_token("created_at_utc", &input.created_at_utc)?;
    if !input.diagnostic_payload.is_object() {
        return Err(ModelLaneError::InvalidInput(
            "diagnostic_payload must be a JSON object".into(),
        ));
    }
    Ok(())
}

fn validate_context_bundle_handoff(
    input: &NewModelLaneContextBundleHandoff,
) -> ModelLaneResult<()> {
    require_token("handoff_id", &input.handoff_id)?;
    require_token("context_bundle_id", &input.context_bundle_id)?;
    require_token("run_id", &input.run_id)?;
    require_token("trace_id", &input.trace_id)?;
    require_token("handoff_span_id", &input.handoff_span_id)?;
    require_token("downstream_lane_id", &input.downstream_lane_id)?;
    require_token("source_lane_id", &input.source_lane_id)?;
    require_token("source_message_id", &input.source_message_id)?;
    require_token("artifact_ref", &input.artifact_ref)?;
    validate_sha256("artifact_sha256", &input.artifact_sha256)?;
    validate_sha256("content_hash", &input.content_hash)?;
    require_token("reason_code", &input.reason_code)?;
    if let Some(decision_ref) = input.decision_ref.as_deref() {
        require_token("decision_ref", decision_ref)?;
    }
    if let Some(reviewer_ref) = input.reviewer_ref.as_deref() {
        require_token("reviewer_ref", reviewer_ref)?;
    }
    require_token("replay_hint", &input.replay_hint)?;
    require_token("event_ledger_stream_id", &input.event_ledger_stream_id)?;
    require_token("work_packet_id", &input.work_packet_id)?;
    require_token("micro_task_id", &input.micro_task_id)?;
    require_token("task_board_id", &input.task_board_id)?;
    require_token("owner_session", &input.owner_session)?;
    require_token("idempotency_key", &input.idempotency_key)?;
    require_token("replay_order_key", &input.replay_order_key)?;
    require_token("created_at_utc", &input.created_at_utc)?;
    let expected_context_bundle_id = model_lane_context_bundle_id_for_handoff(input)?;
    require_equal(
        "context_bundle_id",
        &input.context_bundle_id,
        "derived context bundle id",
        &expected_context_bundle_id,
    )?;
    let parent_span_id = require_optional_token("parent_span_id", input.parent_span_id.as_deref())?;
    if parent_span_id == input.handoff_span_id {
        return Err(ModelLaneError::InvalidInput(
            "parent_span_id must not equal handoff_span_id".into(),
        ));
    }
    if input.linked_span_contexts.is_empty() {
        return Err(ModelLaneError::InvalidInput(
            "linked_span_contexts must include at least one span".into(),
        ));
    }
    for linked in &input.linked_span_contexts {
        require_token("linked_span_contexts[]", linked)?;
        if linked == &input.handoff_span_id {
            return Err(ModelLaneError::InvalidInput(
                "linked_span_contexts must not include handoff_span_id".into(),
            ));
        }
    }
    if matches!(
        input.selection_state,
        ModelLaneHandoffSelectionState::Selected
            | ModelLaneHandoffSelectionState::Rejected
            | ModelLaneHandoffSelectionState::Superseded
    ) {
        require_optional_token("decision_ref", input.decision_ref.as_deref())?;
        require_optional_token("reviewer_ref", input.reviewer_ref.as_deref())?;
    }
    if !input.diagnostic_payload.is_object() {
        return Err(ModelLaneError::InvalidInput(
            "diagnostic_payload must be a JSON object".into(),
        ));
    }
    if let Some(crdt) = input.crdt_payload.as_ref() {
        validate_crdt_handoff_metadata(crdt)?;
    }
    if input.loom_refs.len() > MAX_CONTEXT_BUNDLE_LOOM_REFS {
        return Err(ModelLaneError::InvalidInput(format!(
            "loom_refs exceeds bounded limit {MAX_CONTEXT_BUNDLE_LOOM_REFS}"
        )));
    }
    for loom_ref in &input.loom_refs {
        validate_loom_handoff_ref(loom_ref)?;
    }
    if input.memory_pack_refs.len() > MAX_CONTEXT_BUNDLE_MEMORY_PACK_REFS {
        return Err(ModelLaneError::InvalidInput(format!(
            "memory_pack_refs exceeds bounded FEMS limit {MAX_CONTEXT_BUNDLE_MEMORY_PACK_REFS}"
        )));
    }
    for memory_pack_ref in &input.memory_pack_refs {
        validate_memory_pack_handoff_ref(memory_pack_ref)?;
    }
    Ok(())
}

fn validate_crdt_handoff_metadata(crdt: &ModelLaneCrdtHandoffMetadata) -> ModelLaneResult<()> {
    require_token("crdt_payload.schema_id", &crdt.schema_id)?;
    if crdt.schema_id != "hsk.model_lane_crdt_payload@1" {
        return Err(ModelLaneError::InvalidInput(
            "crdt_payload.schema_id must be hsk.model_lane_crdt_payload@1".into(),
        ));
    }
    require_token("crdt_payload.document_id", &crdt.document_id)?;
    require_token("crdt_payload.workspace_id", &crdt.workspace_id)?;
    require_token("crdt_payload.actor_id", &crdt.actor_id)?;
    require_token("crdt_payload.actor_kind", &crdt.actor_kind)?;
    require_token("crdt_payload.lane_id", &crdt.lane_id)?;
    require_token("crdt_payload.crdt_site_id", &crdt.crdt_site_id)?;
    if crdt.update_seq <= 0 {
        return Err(ModelLaneError::InvalidInput(
            "crdt_payload.update_seq must be positive".into(),
        ));
    }
    require_token("crdt_payload.update_bytes_ref", &crdt.update_bytes_ref)?;
    validate_sha256("crdt_payload.update_sha256", &crdt.update_sha256)?;
    require_token("crdt_payload.state_vector", &crdt.state_vector)?;
    require_token("crdt_payload.base_snapshot_ref", &crdt.base_snapshot_ref)?;
    validate_sha256(
        "crdt_payload.materialized_projection_hash",
        &crdt.materialized_projection_hash,
    )?;
    if !crdt.replay_metadata.is_object() {
        return Err(ModelLaneError::InvalidInput(
            "crdt_payload.replay_metadata must be a JSON object".into(),
        ));
    }
    let format = crdt
        .replay_metadata
        .get("format")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let yjs_compatible = crdt
        .replay_metadata
        .get("yjs_compatible")
        .and_then(Value::as_bool)
        == Some(true);
    if !yjs_compatible || format != "yjs_update_v1" {
        return Err(ModelLaneError::InvalidInput(
            "crdt_payload.replay_metadata must declare Yjs-compatible format yjs_update_v1".into(),
        ));
    }
    require_token("crdt_payload.promotion_gate_ref", &crdt.promotion_gate_ref)?;
    if let Some(promotion_receipt_ref) = crdt.promotion_receipt_ref.as_deref() {
        require_token("crdt_payload.promotion_receipt_ref", promotion_receipt_ref)?;
    }
    require_token(
        "crdt_payload.validation_runner_ref",
        &crdt.validation_runner_ref,
    )?;
    require_token("crdt_payload.authority_effect", &crdt.authority_effect)?;
    if crdt.authority_effect != "advisory_only" {
        return Err(ModelLaneError::InvalidInput(
            "crdt_payload.authority_effect must be advisory_only before promotion".into(),
        ));
    }
    if crdt.promotion_receipt_ref.is_some() {
        return Err(ModelLaneError::InvalidInput(
            "crdt_payload.promotion_receipt_ref must remain null while authority_effect is advisory_only"
                .into(),
        ));
    }
    Ok(())
}

fn validate_loom_handoff_ref(loom_ref: &ModelLaneLoomHandoffRef) -> ModelLaneResult<()> {
    require_token("loom_ref.workspace_id", &loom_ref.workspace_id)?;
    require_token("loom_ref.block_id", &loom_ref.block_id)?;
    if let Some(source_block_id) = loom_ref.source_block_id.as_deref() {
        require_token("loom_ref.source_block_id", source_block_id)?;
    }
    if let Some(target_block_id) = loom_ref.target_block_id.as_deref() {
        require_token("loom_ref.target_block_id", target_block_id)?;
    }
    if let Some(artifact_ref) = loom_ref.artifact_ref.as_deref() {
        require_token("loom_ref.artifact_ref", artifact_ref)?;
    }
    validate_sha256("loom_ref.content_hash", &loom_ref.content_hash)?;
    require_token("loom_ref.version", &loom_ref.version)?;
    require_token(
        "loom_ref.event_ledger_evidence_ref",
        &loom_ref.event_ledger_evidence_ref,
    )?;
    if !loom_ref
        .event_ledger_evidence_ref
        .starts_with("eventledger://")
    {
        return Err(ModelLaneError::InvalidInput(
            "loom_ref.event_ledger_evidence_ref must use eventledger://".into(),
        ));
    }
    require_token(
        "loom_ref.flight_recorder_evidence_ref",
        &loom_ref.flight_recorder_evidence_ref,
    )?;
    if !loom_ref
        .flight_recorder_evidence_ref
        .starts_with("flight-recorder://")
    {
        return Err(ModelLaneError::InvalidInput(
            "loom_ref.flight_recorder_evidence_ref must use flight-recorder://".into(),
        ));
    }
    Ok(())
}

fn validate_memory_pack_handoff_ref(
    memory_pack: &ModelLaneMemoryPackHandoffRef,
) -> ModelLaneResult<()> {
    require_token("memory_pack_ref", &memory_pack.memory_pack_ref)?;
    if is_hidden_memory_pack_ref(&memory_pack.memory_pack_ref) {
        return Err(ModelLaneError::InvalidInput(
            "MemoryPack handoff cannot use hidden provider/session memory as authority".into(),
        ));
    }
    validate_sha256("memory_pack_hash", &memory_pack.memory_pack_hash)?;
    require_token("memory_pack.scope_tag", &memory_pack.scope_tag)?;
    require_token("memory_pack.review_status", &memory_pack.review_status)?;
    if !matches!(
        memory_pack.review_status.as_str(),
        "reviewed" | "operator_reviewed" | "validator_reviewed"
    ) {
        return Err(ModelLaneError::InvalidInput(
            "MemoryPack handoff requires review_status reviewed, operator_reviewed, or validator_reviewed".into(),
        ));
    }
    require_token("memory_pack.classification", &memory_pack.classification)?;
    if !matches!(
        memory_pack.classification.as_str(),
        "cloud_safe_context" | "local_only_context" | "operator_reviewed_context"
    ) {
        return Err(ModelLaneError::InvalidInput(
            "MemoryPack handoff classification must be cloud_safe_context, local_only_context, or operator_reviewed_context".into(),
        ));
    }
    if let Some(projection_ref) = memory_pack.projection_ref.as_deref() {
        require_token("memory_pack.projection_ref", projection_ref)?;
        if is_hidden_memory_pack_ref(projection_ref) {
            return Err(ModelLaneError::InvalidInput(
                "MemoryPack handoff projection_ref cannot use hidden provider/session memory as authority".into(),
            ));
        }
    }
    require_token("memory_pack.evidence_ref", &memory_pack.evidence_ref)?;
    if !memory_pack.evidence_ref.starts_with("eventledger://")
        && !memory_pack.evidence_ref.starts_with("flight-recorder://")
    {
        return Err(ModelLaneError::InvalidInput(
            "MemoryPack handoff evidence_ref must use eventledger:// or flight-recorder://".into(),
        ));
    }
    Ok(())
}

fn is_hidden_memory_pack_ref(reference: &str) -> bool {
    let normalized = reference.trim().to_ascii_lowercase();
    [
        "hidden://",
        "provider-session://",
        "provider_memory://",
        "session-memory://",
        "chat-history://",
    ]
    .iter()
    .any(|prefix| normalized.starts_with(prefix))
}

fn context_bundle_artifact_binding_hash(
    input: &NewModelLaneContextBundleArtifactBinding,
) -> ModelLaneResult<String> {
    Ok(dexterity_sha256_hex(canonical_json_bytes(
        &context_bundle_artifact_binding_hash_basis(input),
    )))
}

fn context_bundle_artifact_binding_hash_basis(
    input: &NewModelLaneContextBundleArtifactBinding,
) -> Value {
    json!({
        "schema_id": "hsk.model_lane_context_bundle_artifact@1",
        "dexterity_kernel": "Dexterity",
        "run_id": &input.run_id,
        "trace_id": &input.trace_id,
        "artifact_ref": &input.artifact_ref,
        "artifact_sha256": &input.artifact_sha256,
        "content_hash": &input.content_hash,
        "artifact_kind": &input.artifact_kind,
        "artifact_manifest_ref": &input.artifact_manifest_ref,
        "artifact_payload_ref": &input.artifact_payload_ref,
        "payload_json": &input.payload_json,
        "event_ledger_stream_id": &input.event_ledger_stream_id,
        "work_packet_id": &input.work_packet_id,
        "micro_task_id": &input.micro_task_id,
        "task_board_id": &input.task_board_id,
        "owner_session": &input.owner_session,
        "diagnostic_payload": &input.diagnostic_payload,
    })
}

fn context_bundle_artifact_binding_event_payload(
    record: &ModelLaneContextBundleArtifactBindingRecord,
    scope: ScopeColumnValues<'_>,
) -> Value {
    json!({
        "schema_id": "hsk.model_lane_context_bundle_artifact@1",
        "dexterity_kernel": "Dexterity",
        "resource_scope": context_bundle_resource_scope_payload(scope),
        "record": record,
    })
}

fn context_bundle_handoff_event_payload(
    record: &ModelLaneContextBundleHandoffRecord,
    scope: ScopeColumnValues<'_>,
) -> Value {
    json!({
        "schema_id": "hsk.model_lane_context_bundle_handoff@1",
        "dexterity_kernel": "Dexterity",
        "resource_scope": context_bundle_resource_scope_payload(scope),
        "record": record,
    })
}

fn context_bundle_resource_scope_payload(scope: ScopeColumnValues<'_>) -> Value {
    json!({
        "owner_account_id": scope.owner_account_id,
        "actor_principal_id": scope.actor_principal_id,
        "authenticated_session_id": scope.authenticated_session_id,
        "access_space_id": scope.access_space_id,
        "workspace_id": scope.workspace_id,
    })
}

fn build_downstream_context_bundle(
    run_id: &str,
    context_bundle_id: &str,
    downstream_lane_id: &str,
    records: Vec<ModelLaneContextBundleHandoffRecord>,
) -> ModelLaneResult<ModelLaneDownstreamContextBundle> {
    let selected: Vec<_> = records
        .iter()
        .filter(|record| record.selection_state == ModelLaneHandoffSelectionState::Selected)
        .cloned()
        .collect();
    let rejected: Vec<_> = records
        .iter()
        .filter(|record| record.selection_state == ModelLaneHandoffSelectionState::Rejected)
        .cloned()
        .collect();
    let unresolved: Vec<_> = records
        .iter()
        .filter(|record| record.selection_state == ModelLaneHandoffSelectionState::Unresolved)
        .cloned()
        .collect();
    let superseded: Vec<_> = records
        .iter()
        .filter(|record| record.selection_state == ModelLaneHandoffSelectionState::Superseded)
        .cloned()
        .collect();
    let allowed_context = json!({
        "schema_id": "hsk.model_lane_downstream_context_bundle@1",
        "dexterity_kernel": "Dexterity",
        "run_id": run_id,
        "context_bundle_id": context_bundle_id,
        "downstream_lane_id": downstream_lane_id,
        "handoffs": &records,
        "selected": selected,
        "rejected": rejected,
        "unresolved": unresolved,
        "superseded": superseded,
    });
    let context_hash = dexterity_sha256_hex(canonical_json_bytes(&allowed_context));
    Ok(ModelLaneDownstreamContextBundle {
        run_id: run_id.to_string(),
        context_bundle_id: context_bundle_id.to_string(),
        downstream_lane_id: downstream_lane_id.to_string(),
        context_hash,
        allowed_context,
        records,
    })
}

fn context_bundle_identity_hash_basis(input: &NewModelLaneContextBundleHandoff) -> Value {
    json!({
        "schema_id": "hsk.model_lane_context_bundle_identity@1",
        "run_id": &input.run_id,
        "trace_id": &input.trace_id,
        "downstream_lane_id": &input.downstream_lane_id,
        "work_packet_id": &input.work_packet_id,
        "micro_task_id": &input.micro_task_id,
        "task_board_id": &input.task_board_id,
        "owner_session": &input.owner_session,
        "event_ledger_stream_id": &input.event_ledger_stream_id,
    })
}

fn context_bundle_handoff_hash(
    input: &NewModelLaneContextBundleHandoff,
) -> ModelLaneResult<String> {
    let basis = context_bundle_handoff_hash_basis(input);
    Ok(dexterity_sha256_hex(serde_json::to_vec(&basis)?))
}

fn context_bundle_handoff_hash_basis(input: &NewModelLaneContextBundleHandoff) -> Value {
    json!({
        "schema_id": "hsk.model_lane_context_bundle_handoff@1",
        "dexterity_kernel": "Dexterity",
        "context_bundle_id": &input.context_bundle_id,
        "run_id": &input.run_id,
        "trace_id": &input.trace_id,
        "handoff_span_id": &input.handoff_span_id,
        "parent_span_id": &input.parent_span_id,
        "linked_span_contexts": &input.linked_span_contexts,
        "downstream_lane_id": &input.downstream_lane_id,
        "source_lane_id": &input.source_lane_id,
        "source_message_id": &input.source_message_id,
        "artifact_ref": &input.artifact_ref,
        "artifact_sha256": &input.artifact_sha256,
        "content_hash": &input.content_hash,
        "source_kind": input.source_kind.as_str(),
        "authority_state": input.authority_state.as_str(),
        "selection_state": input.selection_state.as_str(),
        "reason_code": &input.reason_code,
        "decision_ref": &input.decision_ref,
        "reviewer_ref": &input.reviewer_ref,
        "replay_hint": &input.replay_hint,
        "crdt_payload": &input.crdt_payload,
        "loom_refs": &input.loom_refs,
        "memory_pack_refs": &input.memory_pack_refs,
        "event_ledger_stream_id": &input.event_ledger_stream_id,
        "work_packet_id": &input.work_packet_id,
        "micro_task_id": &input.micro_task_id,
        "task_board_id": &input.task_board_id,
        "owner_session": &input.owner_session,
        "replay_order_key": &input.replay_order_key,
        "diagnostic_payload": &input.diagnostic_payload,
    })
}

fn validate_locus<'a>(
    locus: Option<&'a ModelLaneLocusBinding>,
    owner_kind: &str,
) -> ModelLaneResult<&'a ModelLaneLocusBinding> {
    let locus = locus.ok_or_else(|| {
        ModelLaneError::InvalidInput(format!("{owner_kind} requires locus_binding_ref"))
    })?;
    require_token("locus.work_packet_id", &locus.work_packet_id)?;
    require_token("locus.micro_task_id", &locus.micro_task_id)?;
    require_optional_token("locus.task_board_id", locus.task_board_id.as_deref())?;
    require_token(
        "locus.coordinator_session_id",
        &locus.coordinator_session_id,
    )?;
    require_token("locus.session_id", &locus.session_id)?;
    require_token("locus.model_session_id", &locus.model_session_id)?;
    require_token("locus.owner_session", &locus.owner_session)?;
    require_token("locus_binding_ref", &locus.locus_binding_ref)?;
    Ok(locus)
}

fn validate_locus_common(
    locus: &ModelLaneLocusBinding,
    work_packet_id: &str,
    micro_task_id: &str,
    task_board_id: Option<&str>,
    coordinator_session_id: &str,
    owner_session: &str,
) -> ModelLaneResult<()> {
    require_equal(
        "locus.work_packet_id",
        &locus.work_packet_id,
        "record.work_packet_id",
        work_packet_id,
    )?;
    require_equal(
        "locus.micro_task_id",
        &locus.micro_task_id,
        "record.micro_task_id",
        micro_task_id,
    )?;
    if let Some(task_board_id) = task_board_id {
        require_equal(
            "locus.task_board_id",
            locus.task_board_id.as_deref().unwrap_or(""),
            "record.task_board_id",
            task_board_id,
        )?;
    }
    require_equal(
        "locus.coordinator_session_id",
        &locus.coordinator_session_id,
        "record.coordinator_session_id",
        coordinator_session_id,
    )?;
    require_equal(
        "locus.owner_session",
        &locus.owner_session,
        "record.owner_session",
        owner_session,
    )
}

fn validate_lane_runtime_contract(input: &NewModelLane) -> ModelLaneResult<()> {
    if input.provider_kind == ModelLaneProviderKind::Other {
        return Err(ModelLaneError::InvalidInput(
            "provider_kind other is not supported by Dexterity".into(),
        ));
    }
    if input.capability_token_ids.is_empty() {
        return Err(ModelLaneError::InvalidInput(
            "capability_token_ids must include at least one capability token".into(),
        ));
    }
    require_optional_token(
        "effective_capability_snapshot_ref",
        input.effective_capability_snapshot_ref.as_deref(),
    )?;
    require_optional_token(
        "capability_negotiation_ref",
        input.capability_negotiation_ref.as_deref(),
    )?;
    require_optional_token(
        "provider_feature_profile_ref",
        input.provider_feature_profile_ref.as_deref(),
    )?;
    require_optional_token(
        "requested_execution_policy_ref",
        input.requested_execution_policy_ref.as_deref(),
    )?;
    require_optional_token(
        "effective_execution_policy_ref",
        input.effective_execution_policy_ref.as_deref(),
    )?;
    require_optional_token("cancellation_ref", input.cancellation_ref.as_deref())?;
    require_optional_token("reclaim_policy_ref", input.reclaim_policy_ref.as_deref())?;
    require_optional_token(
        "terminal_status_mapping_ref",
        input.terminal_status_mapping_ref.as_deref(),
    )?;
    if input.tool_gate_decision_refs.is_empty() {
        return Err(ModelLaneError::InvalidInput(
            "tool_gate_decision_refs must include at least one ToolGate decision".into(),
        ));
    }
    for decision_ref in &input.tool_gate_decision_refs {
        require_token("tool_gate_decision_refs[]", decision_ref)?;
    }
    let expected = match input.runtime_binding {
        RuntimeBinding::Local => (
            ModelLaneKind::LocalModel,
            LaunchAuthority::ModelRuntime,
            vec![ModelLaneProviderKind::LocalRuntime],
        ),
        RuntimeBinding::Cloud => (
            ModelLaneKind::CloudModel,
            LaunchAuthority::CloudLane,
            vec![
                ModelLaneProviderKind::OpenAi,
                ModelLaneProviderKind::Anthropic,
            ],
        ),
        RuntimeBinding::CliBridge => (
            ModelLaneKind::CliModel,
            LaunchAuthority::CliBridge,
            vec![ModelLaneProviderKind::OfficialCli],
        ),
        RuntimeBinding::Human => (
            ModelLaneKind::HumanOperator,
            LaunchAuthority::Operator,
            vec![ModelLaneProviderKind::Human],
        ),
        RuntimeBinding::Subagent => (
            ModelLaneKind::Subagent,
            LaunchAuthority::SubagentManager,
            vec![ModelLaneProviderKind::Subagent],
        ),
        RuntimeBinding::Validator => (
            ModelLaneKind::Validator,
            LaunchAuthority::ValidatorRunner,
            vec![ModelLaneProviderKind::Validator],
        ),
    };
    if input.kind != expected.0 || input.launch_authority != expected.1 {
        return Err(ModelLaneError::InvalidInput(format!(
            "runtime_binding {:?} does not match kind {:?} and launch_authority {:?}",
            input.runtime_binding, input.kind, input.launch_authority
        )));
    }
    if !expected.2.contains(&input.provider_kind) {
        return Err(ModelLaneError::InvalidInput(format!(
            "provider_kind {:?} is not supported for runtime_binding {:?}",
            input.provider_kind, input.runtime_binding
        )));
    }
    match input.runtime_binding {
        RuntimeBinding::Local | RuntimeBinding::Cloud | RuntimeBinding::CliBridge => {
            if input.process_ownership_ref.is_some() {
                require_optional_token(
                    "process_ownership_ref",
                    input.process_ownership_ref.as_deref(),
                )?;
                if input.no_os_process_reason_ref.is_some() {
                    return Err(ModelLaneError::InvalidInput(
                        "process-backed lanes must not use no_os_process_reason_ref when process_ownership_ref exists".into(),
                    ));
                }
            } else if input.status == ModelLaneStatus::Failed && input.startup_failure_ref.is_some()
            {
                require_optional_token(
                    "no_os_process_reason_ref",
                    input.no_os_process_reason_ref.as_deref(),
                )?;
            } else {
                return Err(ModelLaneError::InvalidInput(
                    "process-backed lanes require process_ownership_ref unless startup failed before OS ownership".into(),
                ));
            }
        }
        RuntimeBinding::Human | RuntimeBinding::Subagent | RuntimeBinding::Validator => {
            require_optional_token(
                "no_os_process_reason_ref",
                input.no_os_process_reason_ref.as_deref(),
            )?;
            if input.process_ownership_ref.is_some() {
                return Err(ModelLaneError::InvalidInput(
                    "no-OS-process lanes must not use process_ownership_ref".into(),
                ));
            }
        }
    }
    if input.runtime_binding == RuntimeBinding::Cloud {
        require_optional_token("projection_plan_ref", input.projection_plan_ref.as_deref())?;
        require_optional_token("consent_receipt_ref", input.consent_receipt_ref.as_deref())?;
    }
    if matches!(
        input.status,
        ModelLaneStatus::Failed | ModelLaneStatus::Cancelled | ModelLaneStatus::Reclaimable
    ) {
        require_optional_token("failstate_code", input.failstate_code.as_deref())?;
        require_optional_token("reason_ref", input.reason_ref.as_deref())?;
    }
    if input.status == ModelLaneStatus::Failed {
        require_optional_token("startup_failure_ref", input.startup_failure_ref.as_deref())?;
    }
    Ok(())
}

fn recovery_for_status(status: &ModelLaneStatus) -> ModelLaneRecoveryState {
    match status {
        ModelLaneStatus::Planned
        | ModelLaneStatus::Ready
        | ModelLaneStatus::Running
        | ModelLaneStatus::Waiting => ModelLaneRecoveryState::Restartable,
        ModelLaneStatus::Failed | ModelLaneStatus::Reclaimable => {
            ModelLaneRecoveryState::Reclaimable
        }
        ModelLaneStatus::Cancelled | ModelLaneStatus::Completed => ModelLaneRecoveryState::Terminal,
    }
}

fn validate_message_trace(input: &NewModelLaneMessage) -> ModelLaneResult<()> {
    let parent_span_id = require_optional_token("parent_span_id", input.parent_span_id.as_deref())?;
    if parent_span_id == input.message_span_id {
        return Err(ModelLaneError::InvalidInput(
            "parent_span_id must not equal message_span_id".into(),
        ));
    }
    if input.linked_span_contexts.is_empty() {
        return Err(ModelLaneError::InvalidInput(
            "linked_span_contexts must include at least one span".into(),
        ));
    }
    for linked in &input.linked_span_contexts {
        require_token("linked_span_contexts[]", linked)?;
        if linked == &input.message_span_id {
            return Err(ModelLaneError::InvalidInput(
                "linked_span_contexts must not include message_span_id".into(),
            ));
        }
    }
    Ok(())
}

fn validate_message_routing(input: &NewModelLaneMessage) -> ModelLaneResult<()> {
    if let ModelLaneTarget::Lane(lane_id) = &input.to_lane {
        require_token("to_lane.lane_id", lane_id)?;
    }
    let routing = input
        .routing
        .as_ref()
        .ok_or_else(|| ModelLaneError::InvalidInput("routing metadata is required".into()))?;
    require_token("routing.target_role", &routing.target_role)?;
    require_token("routing.target_session", &routing.target_session)?;
    require_token("routing.correlation_id", &routing.correlation_id)?;
    if let Some(ack_for) = routing.ack_for.as_deref() {
        require_token("routing.ack_for", ack_for)?;
    }
    Ok(())
}

fn validate_message_authority(input: &NewModelLaneMessage) -> ModelLaneResult<()> {
    // Cheap pre-transaction CRDT posture validation. The AUTHORITATIVE
    // completeness + resolution gate is `validate_message_crdt_authority_tx`
    // (durable, fail-closed): it requires base_snapshot + state_vector when an
    // update ref is present, denies any partial CRDT metadata that lacks an
    // update ref, denies update_ref + stale_base together, and — for Proposal
    // kind — requires a persisted crdt_proposal_ref. This sync layer must NOT
    // shadow those specific denials with a generic "field is required" error.
    //
    // Per MT-002 acceptance the CRDT fields are carried "as applicable", NOT all
    // five unconditionally. The prior code required proposal_ref + all four
    // crdt_* fields whenever ANY was set, which (a) made a Proposal's kind-aware
    // proposal-ref rule dead code and (b) made every CRDT-bearing message
    // unsatisfiable, since at the time no proposal row could be minted whose
    // applied_update_sha256 equalled a Yjs-update hash. WP-1 MT-018 removed that
    // second blocker at its source: `applied_update_sha256` is the approved-DIFF
    // hash and is cross-checked against the proposal's own `diff_sha256`, while
    // Yjs update identity is carried by `applied_update_id`, so the Proposal-kind
    // CRDT path is now genuinely admissible. proposal_ref is required
    // by AUTHORITY STATE (PromotionCandidate/Promoted) below, never by CRDT.
    //
    // Fail-closed is preserved: every field that IS present must be a valid
    // non-empty token, and when a concrete update ref is present its base
    // snapshot + state vector are required here too (matching the durable gate).
    if let Some(proposal_ref) = input.proposal_ref.as_deref() {
        require_token("proposal_ref", proposal_ref)?;
    }
    if let Some(update_ref) = input.crdt_update_ref.as_deref() {
        require_token("crdt_update_ref", update_ref)?;
        let base_snapshot = input.crdt_base_snapshot_ref.as_deref().ok_or_else(|| {
            ModelLaneError::InvalidInput("crdt_base_snapshot_ref is required".into())
        })?;
        require_token("crdt_base_snapshot_ref", base_snapshot)?;
        let state_vector = input
            .crdt_state_vector
            .as_deref()
            .ok_or_else(|| ModelLaneError::InvalidInput("crdt_state_vector is required".into()))?;
        require_token("crdt_state_vector", state_vector)?;
    } else {
        if let Some(base_snapshot) = input.crdt_base_snapshot_ref.as_deref() {
            require_token("crdt_base_snapshot_ref", base_snapshot)?;
        }
        if let Some(state_vector) = input.crdt_state_vector.as_deref() {
            require_token("crdt_state_vector", state_vector)?;
        }
    }
    if let Some(proposal_ref) = input.crdt_proposal_ref.as_deref() {
        require_token("crdt_proposal_ref", proposal_ref)?;
    }
    if let Some(stale_base) = input.crdt_stale_base_ref.as_deref() {
        require_token("crdt_stale_base_ref", stale_base)?;
    }
    if matches!(
        input.kind,
        ModelLaneMessageKind::ToolRequest | ModelLaneMessageKind::ToolResult
    ) && input.tool_gate_decision_refs.is_empty()
    {
        return Err(ModelLaneError::InvalidInput(
            "tool messages require tool_gate_decision_refs".into(),
        ));
    }
    match input.authority {
        ModelLaneAuthority::Advisory => Ok(()),
        ModelLaneAuthority::PromotionCandidate => {
            require_optional_token("proposal_ref", input.proposal_ref.as_deref())?;
            require_optional_token("promotion_gate_ref", input.promotion_gate_ref.as_deref())?;
            Ok(())
        }
        ModelLaneAuthority::Promoted => {
            require_optional_token(
                "promotion_decision_id",
                input.promotion_decision_id.as_deref(),
            )
            .map_err(|_| {
                ModelLaneError::InvalidInput(
                    "Promoted ModelLaneMessage requires approved PromotionGate resolution: promotion_decision_id is required"
                        .into(),
                )
            })?;
            require_optional_token("promotion_gate_ref", input.promotion_gate_ref.as_deref())
                .map_err(|_| {
                    ModelLaneError::InvalidInput(
                        "Promoted ModelLaneMessage requires approved PromotionGate resolution: promotion_gate_ref is required"
                            .into(),
                    )
                })?;
            require_optional_token(
                "promotion_receipt_ref",
                input.promotion_receipt_ref.as_deref(),
            )
            .map_err(|_| {
                ModelLaneError::InvalidInput(
                    "Promoted ModelLaneMessage requires approved PromotionGate resolution: promotion_receipt_ref is required"
                        .into(),
                )
            })?;
            require_optional_token(
                "promoted_artifact_ref",
                input.promoted_artifact_ref.as_deref(),
            )
            .map_err(|_| {
                ModelLaneError::InvalidInput(
                    "Promoted ModelLaneMessage requires approved PromotionGate resolution: promoted_artifact_ref is required"
                        .into(),
                )
            })?;
            validate_sha256(
                "promoted_artifact_sha256",
                require_optional_token(
                    "promoted_artifact_sha256",
                    input.promoted_artifact_sha256.as_deref(),
                )?
                .as_str(),
            )?;
            require_optional_token(
                "promoted_artifact_version",
                input.promoted_artifact_version.as_deref(),
            )
            .map_err(|_| {
                ModelLaneError::InvalidInput(
                    "Promoted ModelLaneMessage requires approved PromotionGate resolution: promoted_artifact_version is required"
                        .into(),
                )
            })?;
            Ok(())
        }
        ModelLaneAuthority::OperatorDecision => {
            require_optional_token(
                "operator_decision_ref",
                input.operator_decision_ref.as_deref(),
            )?;
            Ok(())
        }
        ModelLaneAuthority::ValidatorVerdict => {
            require_optional_token(
                "validator_verdict_ref",
                input.validator_verdict_ref.as_deref(),
            )?;
            Ok(())
        }
    }
}

fn require_token(field: &str, value: &str) -> ModelLaneResult<()> {
    if value.trim().is_empty() {
        return Err(ModelLaneError::InvalidInput(format!("{field} is required")));
    }
    if value.len() > 512 {
        return Err(ModelLaneError::InvalidInput(format!(
            "{field} exceeds 512 bytes"
        )));
    }
    Ok(())
}

fn require_optional_token(field: &str, value: Option<&str>) -> ModelLaneResult<String> {
    let value =
        value.ok_or_else(|| ModelLaneError::InvalidInput(format!("{field} is required")))?;
    require_token(field, value)?;
    Ok(value.to_string())
}

fn require_equal(
    left_field: &str,
    left: &str,
    right_field: &str,
    right: &str,
) -> ModelLaneResult<()> {
    if left == right {
        return Ok(());
    }
    Err(ModelLaneError::InvalidInput(format!(
        "{left_field} must match {right_field}"
    )))
}

fn validate_sha256(field: &str, value: &str) -> ModelLaneResult<()> {
    if value.len() == 64
        && value
            .chars()
            .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase())
    {
        return Ok(());
    }
    Err(ModelLaneError::InvalidInput(format!(
        "{field} must be lowercase sha256 hex"
    )))
}

fn row_to_json(row: sqlx::postgres::PgRow, column: &str) -> ModelLaneResult<Value> {
    row.try_get::<Value, _>(column)
        .map_err(ModelLaneError::from)
}
