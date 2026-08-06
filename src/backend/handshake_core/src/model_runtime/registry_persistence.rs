//! PostgreSQL authority for durable ModelRuntime selections.
//!
//! The live [`ModelRegistry`](super::ModelRegistry) is process-local dispatch
//! state. This store durably owns the artifact-to-adapter selection. Adapter,
//! capabilities, and provider are immutable during ordinary boot; display and
//! per-boot observation fields may change without changing selection revision.

use std::{
    collections::{BTreeMap, BTreeSet},
    ops::{Deref, DerefMut},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use sqlx::{
    pool::PoolConnection,
    postgres::{PgConnection, PgPool, PgRow},
    Executor, Postgres, Row,
};
use thiserror::Error;
use tokio::{sync::OnceCell, time::Instant};
use uuid::Uuid;

use crate::{
    kernel::{
        context_bundle::{canonical_json_bytes, sha256_hex},
        KernelActor, KernelEventType, NewKernelEvent,
    },
    storage::postgres::append_kernel_event_with_executor,
    swarm_orchestration::resource_scope::{
        stored_resource_scope_from_row, ResourceAccessContext, ResourceScope, ResourceScopeQuery,
        ScopeDenied, SystemScopeAuthority, RESOURCE_SCOPE_INSERT_COLUMNS,
        RESOURCE_SCOPE_SELECT_COLUMNS,
    },
};

use super::{
    BaseModelTag, ModelCapabilities, ModelId, ModelRegistration, ModelRegistry, ModelRuntimeError,
    ModelRuntimeRole, OperatorId, ProviderKind, RuntimeBinding,
};

/// PostgreSQL table installed by migration `0348_model_runtime_registry.sql`.
pub const MODEL_RUNTIME_REGISTRY_TABLE: &str = "model_runtime_registry";
/// Schema discriminator persisted on every canonical registry row.
pub const MODEL_RUNTIME_REGISTRY_SCHEMA_ID: &str = "hsk.model_runtime_registry.row@2";
/// Schema discriminator for the canonical serialized `ModelCapabilities`.
pub const MODEL_RUNTIME_CAPABILITIES_SCHEMA_ID: &str = "hsk.model_runtime.capabilities@1";
/// Schema discriminator carried by initial-selection and explicit-rebind events.
pub const MODEL_RUNTIME_SELECTION_EVENT_SCHEMA_ID: &str = "hsk.model_runtime.selection_event@3";
pub const MODEL_RUNTIME_ACTIVE_SELECTION_SCHEMA_ID: &str = "hsk.model_runtime.active_selection@1";
pub const MODEL_RUNTIME_ACTIVE_SELECTION_EVENT_SCHEMA_ID: &str =
    "hsk.model_runtime.active_selection_event@1";
const LEGACY_MODEL_RUNTIME_SELECTION_EVENT_SCHEMA_IDS: &[&str] = &[
    "hsk.model_runtime.selection_event@1",
    "hsk.model_runtime.selection_event@2",
];

const MODEL_RUNTIME_REGISTRY_UPDATED_INDEX: &str =
    "idx_model_runtime_registry_selection_updated_at";
const MODEL_REGISTRY_ADVISORY_LOCK_TIMEOUT: Duration = Duration::from_secs(2);
const MODEL_REGISTRY_ADVISORY_LOCK_RETRY_INTERVAL: Duration = Duration::from_millis(20);
// PostgreSQL documents that lock_timeout must be lower than statement_timeout;
// otherwise statement_timeout always wins and erases the typed 55P03 lock-wait
// signal. Keep a material margin so artifact row contention is classified as
// SelectionLockTimeout rather than a generic statement timeout.
const MODEL_REGISTRY_DATABASE_LOCK_TIMEOUT: Duration = Duration::from_millis(1_500);
const MODEL_REGISTRY_DATABASE_STATEMENT_TIMEOUT: Duration = Duration::from_secs(2);
#[cfg(feature = "test-utils")]
const MODEL_REGISTRY_TEST_START_GATE_SERVER_TIMEOUT: Duration = Duration::from_secs(3);
const MODEL_REGISTRY_AUDIT_EVENT_CAP: i64 = 4_096;
const MODEL_REGISTRY_ROW_ENUMERATION_CAP: i64 = 4_096;
const MODEL_REGISTRY_ENUMERATION_AUDIT_EVENT_CAP: u64 = 4_096;
const MODEL_REGISTRY_CAPABILITIES_JSON_BYTE_CAP: i64 = 64 * 1_024;
const MODEL_REGISTRY_ROW_BYTE_CAP: i64 = 1_024 * 1_024;
const MODEL_REGISTRY_ROW_SET_BYTE_CAP: i64 = 16 * 1_024 * 1_024;
const MODEL_REGISTRY_AUDIT_PAYLOAD_BYTE_CAP: i64 = 1_024 * 1_024;
const MODEL_REGISTRY_AUDIT_EVENT_BYTE_CAP: i64 = 2 * 1_024 * 1_024;
const MODEL_REGISTRY_AUDIT_SET_BYTE_CAP: i64 = 16 * 1_024 * 1_024;
const MODEL_REGISTRY_INPUT_TEXT_BYTE_CAP: usize = 64 * 1_024;

const MODEL_REGISTRY_SELECT_COLUMNS: &str = r#"
    schema_id,
    registry_row_id,
    artifact_sha256,
    artifact_locator,
    last_observed_runtime_model_id,
    runtime_binding,
    runtime_role,
    capabilities_schema_id,
    capabilities_json,
    provider,
    base_model_tag,
    last_observed_by,
    selection_revision,
    selection_created_event_id,
    selection_updated_event_id,
    selection_created_at_utc,
    selection_updated_at_utc,
    last_observed_at_utc
"#;

/// The immutable portion of an artifact's durable ModelRuntime selection.
///
/// Paths, display labels, registering actors, timestamps, and boot-scoped
/// `ModelId` values are deliberately excluded so project relocation or display
/// rename cannot masquerade as an adapter rebind.
#[derive(Clone, Debug, PartialEq)]
pub struct ModelRuntimeSelection {
    pub artifact_sha256: [u8; 32],
    pub runtime_binding: RuntimeBinding,
    pub runtime_role: ModelRuntimeRole,
    pub declared_capabilities: ModelCapabilities,
    pub provider: ProviderKind,
}

impl From<&ModelRegistration> for ModelRuntimeSelection {
    fn from(registration: &ModelRegistration) -> Self {
        Self {
            artifact_sha256: registration.sha256,
            runtime_binding: registration.runtime_binding,
            runtime_role: ModelRuntimeRole::Completion,
            declared_capabilities: registration.declared_capabilities.clone(),
            provider: registration.provider,
        }
    }
}

/// One live registration bound to its explicit persisted runtime role.
#[derive(Clone, Debug, PartialEq)]
pub struct RoleBoundModelRegistration {
    pub registration: ModelRegistration,
    pub runtime_role: ModelRuntimeRole,
}

impl RoleBoundModelRegistration {
    pub fn completion(registration: ModelRegistration) -> Self {
        Self {
            registration,
            runtime_role: ModelRuntimeRole::Completion,
        }
    }

    pub fn embedding(registration: ModelRegistration) -> Self {
        Self {
            registration,
            runtime_role: ModelRuntimeRole::Embedding,
        }
    }

    fn selection(&self) -> ModelRuntimeSelection {
        ModelRuntimeSelection {
            artifact_sha256: self.registration.sha256,
            runtime_binding: self.registration.runtime_binding,
            runtime_role: self.runtime_role,
            declared_capabilities: self.registration.declared_capabilities.clone(),
            provider: self.registration.provider,
        }
    }
}

/// Operator/control-plane evidence required for an explicit immutable rebind.
#[derive(Clone, Debug, PartialEq)]
pub struct ExplicitModelRuntimeRebind {
    actor: KernelActor,
    reason: String,
    expected_selection_revision: u64,
}

impl ExplicitModelRuntimeRebind {
    pub fn new(
        actor: KernelActor,
        reason: impl Into<String>,
        expected_selection_revision: u64,
    ) -> Result<Self, ModelRegistryPersistenceError> {
        let request = Self {
            actor,
            reason: reason.into().trim().to_string(),
            expected_selection_revision,
        };
        validate_rebind_request(&request)?;
        Ok(request)
    }

    pub fn actor(&self) -> &KernelActor {
        &self.actor
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }

    pub fn expected_selection_revision(&self) -> u64 {
        self.expected_selection_revision
    }
}

/// Durable PostgreSQL-backed model-selection authority.
#[derive(Clone)]
pub struct ModelRegistryStore {
    pool: PgPool,
    authority: Arc<OnceCell<ModelRegistryAuthority>>,
    /// HBR-PRIV-001/002 account-bound scope for the registry authority tables.
    /// Registry rows and active-selection receipts are durable product
    /// resources: which models an operator has registered, and which one they
    /// made their default, is their data, not the node's.
    access: ResourceAccessContext,
    #[cfg(feature = "test-utils")]
    precommit_advisory_gate_for_tests: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ModelRegistryAuthority {
    schema: String,
    relation_oid: i64,
    event_ledger_oid: i64,
}

/// A committed row recovered from the durable model-selection authority.
#[derive(Clone, Debug, PartialEq)]
pub struct PersistedModelRegistration {
    pub schema_id: String,
    pub registry_row_id: Uuid,
    pub artifact_sha256: [u8; 32],
    pub artifact_locator: String,
    pub last_observed_runtime_model_id: ModelId,
    pub runtime_binding: RuntimeBinding,
    pub runtime_role: ModelRuntimeRole,
    pub capabilities_schema_id: String,
    pub declared_capabilities: ModelCapabilities,
    pub provider: ProviderKind,
    pub base_model_tag: BaseModelTag,
    pub last_observed_by: OperatorId,
    pub selection_revision: u64,
    pub selection_created_event_id: String,
    pub selection_updated_event_id: String,
    pub selection_created_at_utc: DateTime<Utc>,
    pub selection_updated_at_utc: DateTime<Utc>,
    pub last_observed_at_utc: DateTime<Utc>,
}

/// Stable purpose key for one PostgreSQL-authoritative active runtime default.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum ModelRuntimeSelectionPurpose {
    #[serde(rename = "application/default")]
    ApplicationDefault,
    #[serde(rename = "embeddings/default")]
    EmbeddingsDefault,
}

impl ModelRuntimeSelectionPurpose {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ApplicationDefault => "application/default",
            Self::EmbeddingsDefault => "embeddings/default",
        }
    }

    pub const fn runtime_role(self) -> ModelRuntimeRole {
        match self {
            Self::ApplicationDefault => ModelRuntimeRole::Completion,
            Self::EmbeddingsDefault => ModelRuntimeRole::Embedding,
        }
    }
}

/// Committed active-default authority. The artifact hash is restart-stable;
/// callers join it to the current boot catalog rather than persisting a boot UUID.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersistedActiveModelSelection {
    pub purpose: ModelRuntimeSelectionPurpose,
    pub runtime_role: ModelRuntimeRole,
    pub artifact_sha256: [u8; 32],
    pub selection_revision: u64,
    pub selection_created_event_id: String,
    pub selection_updated_event_id: String,
    pub selection_created_at_utc: DateTime<Utc>,
    pub selection_updated_at_utc: DateTime<Utc>,
}

#[derive(Debug)]
struct PersistedSelectionEvent {
    event_id: String,
    event_sequence: i64,
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
    created_at_utc: DateTime<Utc>,
}

struct RecoveredSelectionAudit {
    registry_row_id: Uuid,
    selection_revision: u64,
    selection_created_event_id: String,
    selection_updated_event_id: String,
    selection_created_at_utc: DateTime<Utc>,
    selection_updated_at_utc: DateTime<Utc>,
}

impl PersistedModelRegistration {
    /// Return only the immutable selection fields checked during boot.
    pub fn selection(&self) -> ModelRuntimeSelection {
        ModelRuntimeSelection {
            artifact_sha256: self.artifact_sha256,
            runtime_binding: self.runtime_binding,
            runtime_role: self.runtime_role,
            declared_capabilities: self.declared_capabilities.clone(),
            provider: self.provider,
        }
    }

    /// Rehydrate a live registration using a newly resolved project-local path
    /// and a fresh boot identity. The durable row never stores the host path.
    pub fn rehydrate_with_current_runtime_model_id(
        &self,
        current_runtime_model_id: ModelId,
        artifact_path: PathBuf,
    ) -> Result<ModelRegistration, ModelRegistryPersistenceError> {
        if artifact_path.as_os_str().is_empty() {
            return Err(ModelRegistryPersistenceError::CorruptRow(
                "configured artifact path is empty during registry rehydration".to_string(),
            ));
        }
        validate_artifact_locator(self.artifact_sha256, &self.artifact_locator)?;
        let registration = ModelRegistration {
            model_id: current_runtime_model_id,
            artifact_path,
            sha256: self.artifact_sha256,
            runtime_binding: self.runtime_binding,
            declared_capabilities: self.declared_capabilities.clone(),
            base_model_tag: self.base_model_tag.clone(),
            registered_at_utc: Utc::now(),
            registered_by: self.last_observed_by.clone(),
            provider: self.provider,
        };
        validate_registration(&registration)?;
        Ok(registration)
    }
}

#[derive(Debug, Error)]
pub enum ModelRegistryPersistenceError {
    #[error("model registry persistence rejected registration: {0}")]
    InvalidRegistration(String),
    #[error("model registry persistence database error: {0}")]
    Database(sqlx::Error),
    #[error("model registry persistence serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("model registry persistence audit error: {0}")]
    Audit(String),
    #[error("model registry persistence returned corrupt row: {0}")]
    CorruptRow(String),
    #[error("model registry persistence authority is unavailable: {0}")]
    AuthorityUnavailable(String),
    #[error("model registry selection conflict: {0}")]
    SelectionConflict(String),
    #[error("model registry committed observation mismatch: {0}")]
    ObservationMismatch(String),
    #[error("model registry selection revision mismatch: expected {expected}, found {actual}")]
    SelectionRevisionMismatch { expected: u64, actual: u64 },
    #[error("model registry selection is absent for artifact {0}")]
    SelectionNotFound(String),
    #[error(
        "model registry selection lock timed out after {timeout_ms} ms for artifact {artifact_sha256}"
    )]
    SelectionLockTimeout {
        artifact_sha256: String,
        timeout_ms: u64,
    },
    #[error("model registry explicit rebind rejected: {0}")]
    InvalidRebind(String),
    /// HBR-PRIV-002 default-deny. Carries the stable denial reason code only —
    /// never the withheld row's identifiers or contents.
    #[error("model registry resource scope denied: {0}")]
    ScopeDenied(#[from] ScopeDenied),
}

impl From<sqlx::Error> for ModelRegistryPersistenceError {
    fn from(error: sqlx::Error) -> Self {
        if matches!(&error, sqlx::Error::PoolTimedOut) {
            return Self::AuthorityUnavailable(format!(
                "PostgreSQL model registry transaction start exceeded the bounded {}ms deadline",
                MODEL_REGISTRY_DATABASE_STATEMENT_TIMEOUT.as_millis()
            ));
        }
        if matches!(
            &error,
            sqlx::Error::Database(database_error)
                if database_error.code().as_deref() == Some("55P03")
        ) {
            return Self::AuthorityUnavailable(format!(
                "PostgreSQL model registry authority lock exceeded the bounded {} ms deadline",
                MODEL_REGISTRY_DATABASE_LOCK_TIMEOUT.as_millis()
            ));
        }
        if matches!(
            &error,
            sqlx::Error::Database(database_error)
                if database_error.code().as_deref() == Some("57014")
        ) {
            return Self::AuthorityUnavailable(format!(
                "PostgreSQL model registry statement exceeded the bounded {} ms authority deadline",
                MODEL_REGISTRY_DATABASE_STATEMENT_TIMEOUT.as_millis()
            ));
        }
        Self::Database(error)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AuthorityTransactionMode {
    ReadWrite,
    RepeatableReadOnly,
}

/// Cancellation-safe owner for a PostgreSQL authority transaction.
///
/// SQLx 0.8.6 increments its PostgreSQL transaction depth only after the
/// server's `ReadyForQuery`. A cancelled custom `Transaction::begin` can
/// therefore return a connection whose server transaction is open while the
/// client believes it is idle. This wrapper keeps the connection non-reusable
/// until an explicit COMMIT acknowledgement. Any timeout, database error, or
/// caller cancellation physically closes the connection instead of re-pooling
/// an uncertain transaction state.
struct AuthorityTransaction {
    connection: Option<PoolConnection<Postgres>>,
    reusable_after_drop: bool,
}

impl AuthorityTransaction {
    fn new(connection: PoolConnection<Postgres>) -> Self {
        Self {
            connection: Some(connection),
            reusable_after_drop: false,
        }
    }

    async fn commit(mut self) -> Result<(), ModelRegistryPersistenceError> {
        let result = tokio::time::timeout(
            MODEL_REGISTRY_DATABASE_STATEMENT_TIMEOUT,
            (&mut *self).execute(sqlx::raw_sql("COMMIT")),
        )
        .await
        .map_err(|_| {
            ModelRegistryPersistenceError::AuthorityUnavailable(format!(
                "PostgreSQL model registry commit acknowledgement exceeded the bounded {}ms deadline; the physical connection was closed and durable state must be recovered before retry",
                MODEL_REGISTRY_DATABASE_STATEMENT_TIMEOUT.as_millis()
            ))
        })?;
        result?;

        // There must be no await/cancellation point between the acknowledged
        // COMMIT and making the physical connection reusable.
        self.reusable_after_drop = true;
        Ok(())
    }
}

impl Deref for AuthorityTransaction {
    type Target = PgConnection;

    fn deref(&self) -> &Self::Target {
        &*self
            .connection
            .as_ref()
            .expect("authority transaction connection remains owned until drop")
    }
}

impl DerefMut for AuthorityTransaction {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self
            .connection
            .as_mut()
            .expect("authority transaction connection remains owned until drop")
    }
}

impl Drop for AuthorityTransaction {
    fn drop(&mut self) {
        if !self.reusable_after_drop {
            let Some(pooled) = self.connection.take() else {
                return;
            };
            // An uncertain transaction must leave the pool immediately. Both
            // SQLx close paths flush buffered writes and can wait behind the
            // in-flight query that was just cancelled. Detach and synchronously
            // drop PgConnection so its buffered transport is discarded without
            // flushing, spawning, or re-pooling uncertain state.
            drop(pooled.detach());
        }
    }
}

impl ModelRegistryStore {
    /// Bind the store to Handshake's existing managed PostgreSQL pool.
    ///
    /// Legacy constructor: it yields
    /// [`SystemScopeAuthority::legacy_unscoped_call_site`], so reads are not
    /// account-filtered and writes are stamped with a NULL `owner_account_id`.
    /// New code and every HTTP boundary MUST use [`Self::new_for_owner`] or
    /// [`Self::new_scoped`].
    pub fn new(pool: PgPool) -> Self {
        Self::new_with_access(
            pool,
            ResourceAccessContext::system(SystemScopeAuthority::legacy_unscoped_call_site()),
        )
    }

    /// Registry store bound to one owning account for reads and writes.
    pub fn new_scoped(pool: PgPool, scope: ResourceScope) -> Self {
        Self::new_with_access(pool, ResourceAccessContext::for_account(scope))
    }

    /// Read-only registry store bound to one owning account.
    pub fn new_for_owner(pool: PgPool, query: ResourceScopeQuery) -> Self {
        Self::new_with_access(pool, ResourceAccessContext::for_reader(query))
    }

    pub fn new_with_access(pool: PgPool, access: ResourceAccessContext) -> Self {
        Self {
            pool,
            authority: Arc::new(OnceCell::new()),
            access,
            #[cfg(feature = "test-utils")]
            precommit_advisory_gate_for_tests: None,
        }
    }

    pub fn access(&self) -> &ResourceAccessContext {
        &self.access
    }

    /// Integration-only fault seam that blocks after registry/audit DML and read-back but before
    /// COMMIT. A test holds the matching session advisory lock so PostgreSQL's transaction-local
    /// lock timeout aborts the real transaction and proves post-load rollback behavior.
    #[doc(hidden)]
    #[cfg(feature = "test-utils")]
    pub fn with_precommit_advisory_gate_for_tests(mut self, advisory_key: i64) -> Self {
        self.precommit_advisory_gate_for_tests = Some(advisory_key);
        self
    }

    /// Verify table kind, schema discriminator, column type/nullability,
    /// every named constraint, and the required query index on the pinned
    /// authority schema.
    pub async fn ensure_authority_available(&self) -> Result<(), ModelRegistryPersistenceError> {
        let (tx, _) = self
            .begin_authority_transaction(AuthorityTransactionMode::RepeatableReadOnly)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn begin_authority_transaction(
        &self,
        mode: AuthorityTransactionMode,
    ) -> Result<(AuthorityTransaction, ModelRegistryAuthority), ModelRegistryPersistenceError> {
        self.begin_authority_transaction_inner(mode, None).await
    }

    async fn begin_authority_transaction_inner(
        &self,
        mode: AuthorityTransactionMode,
        #[cfg_attr(not(feature = "test-utils"), allow(unused_variables))]
        start_gate_for_tests: Option<i64>,
    ) -> Result<(AuthorityTransaction, ModelRegistryAuthority), ModelRegistryPersistenceError> {
        let database_lock_timeout =
            format!("{}ms", MODEL_REGISTRY_DATABASE_LOCK_TIMEOUT.as_millis());
        let database_statement_timeout = format!(
            "{}ms",
            MODEL_REGISTRY_DATABASE_STATEMENT_TIMEOUT.as_millis()
        );
        let begin_mode = if mode == AuthorityTransactionMode::RepeatableReadOnly {
            "BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY"
        } else {
            "BEGIN READ WRITE"
        };
        let synchronous_commit = if mode == AuthorityTransactionMode::ReadWrite {
            "; SET LOCAL synchronous_commit = 'on'"
        } else {
            ""
        };
        #[cfg(feature = "test-utils")]
        let start_gate = start_gate_for_tests
            .map(|key| {
                format!(
                    "SET LOCAL lock_timeout = '{}ms'; SET LOCAL statement_timeout = '{}ms'; SELECT pg_catalog.pg_advisory_xact_lock({key}::pg_catalog.int8);",
                    MODEL_REGISTRY_TEST_START_GATE_SERVER_TIMEOUT.as_millis(),
                    MODEL_REGISTRY_TEST_START_GATE_SERVER_TIMEOUT.as_millis()
                )
            })
            .unwrap_or_default();
        #[cfg(not(feature = "test-utils"))]
        let start_gate = String::new();

        // ROLLBACK sanitizes transaction state inherited from any other
        // cancellation-unsafe borrower of this shared pool. Production
        // transaction-local deadlines are installed before any authority lock
        // can block. The test-only gate then extends only its server lock and
        // statement deadlines to three seconds, so the two-second client
        // deadline/cancellation closes the transport first while PostgreSQL is
        // still guaranteed to unwind the abandoned statement without an
        // out-of-band PID signal.
        let begin_statement = format!(
            "ROLLBACK; {begin_mode}; SET LOCAL lock_timeout = '{database_lock_timeout}'; SET LOCAL statement_timeout = '{database_statement_timeout}'; SET LOCAL TIME ZONE 'UTC'{synchronous_commit}; {start_gate}"
        );
        let connection = tokio::time::timeout(
            MODEL_REGISTRY_DATABASE_STATEMENT_TIMEOUT,
            self.pool.acquire(),
        )
        .await
        .map_err(|_| {
            ModelRegistryPersistenceError::AuthorityUnavailable(format!(
                "PostgreSQL model registry transaction start exceeded the bounded {}ms deadline",
                MODEL_REGISTRY_DATABASE_STATEMENT_TIMEOUT.as_millis()
            ))
        })??;
        let mut tx = AuthorityTransaction::new(connection);
        tokio::time::timeout(
            MODEL_REGISTRY_DATABASE_STATEMENT_TIMEOUT,
            (&mut *tx).execute(sqlx::raw_sql(&begin_statement)),
        )
        .await
        .map_err(|_| {
            ModelRegistryPersistenceError::AuthorityUnavailable(format!(
                "PostgreSQL model registry transaction initialization exceeded the bounded {}ms deadline; the physical connection was closed without re-pooling",
                MODEL_REGISTRY_DATABASE_STATEMENT_TIMEOUT.as_millis()
            ))
        })??;
        let authority = if let Some(authority) = self.authority.get() {
            authority.clone()
        } else {
            let resolved = resolve_authority_tx(&mut tx).await?;
            if self.authority.set(resolved.clone()).is_err() {
                let concurrent = self.authority.get().ok_or_else(|| {
                    ModelRegistryPersistenceError::AuthorityUnavailable(
                        "model registry authority cache initialization raced without a winner"
                            .to_string(),
                    )
                })?;
                if concurrent != &resolved {
                    return Err(ModelRegistryPersistenceError::AuthorityUnavailable(
                        "model registry authority identity changed during concurrent initialization"
                            .to_string(),
                    ));
                }
            }
            resolved
        };
        let schema = authority.schema.clone();
        let configured_lock_timeout: String =
            sqlx::query_scalar("SELECT pg_catalog.current_setting('lock_timeout')")
                .fetch_one(&mut *tx)
                .await?;
        if configured_lock_timeout.trim().is_empty() {
            return Err(ModelRegistryPersistenceError::AuthorityUnavailable(
                "failed to bound PostgreSQL model registry lock waits".to_string(),
            ));
        }
        let configured_statement_timeout: String =
            sqlx::query_scalar("SELECT pg_catalog.current_setting('statement_timeout')")
                .fetch_one(&mut *tx)
                .await?;
        if configured_statement_timeout.trim().is_empty() {
            return Err(ModelRegistryPersistenceError::AuthorityUnavailable(
                "failed to bound PostgreSQL model registry statement execution".to_string(),
            ));
        }
        let configured_time_zone: String =
            sqlx::query_scalar("SELECT pg_catalog.current_setting('TimeZone')")
                .fetch_one(&mut *tx)
                .await?;
        if configured_time_zone != "UTC" {
            return Err(ModelRegistryPersistenceError::AuthorityUnavailable(
                format!(
                    "failed to pin PostgreSQL model registry transaction TimeZone to UTC; got {configured_time_zone}"
                ),
            ));
        }
        if mode == AuthorityTransactionMode::ReadWrite {
            let configured_synchronous_commit: String =
                sqlx::query_scalar("SELECT pg_catalog.current_setting('synchronous_commit')")
                    .fetch_one(&mut *tx)
                    .await?;
            if configured_synchronous_commit != "on" {
                return Err(ModelRegistryPersistenceError::AuthorityUnavailable(
                    format!(
                        "failed to require synchronous PostgreSQL commit for model registry mutation; got {configured_synchronous_commit}"
                    ),
                ));
            }
        }
        pin_authority_schema_tx(&mut tx, &schema).await?;
        lock_model_registry_authority_tx(&mut tx, &authority, mode).await?;
        assert_model_registry_authority_tx(&mut tx, &authority).await?;
        ensure_authority_available_tx(&mut tx, &authority).await?;
        require_model_registry_crash_durability_tx(&mut tx).await?;
        Ok((tx, authority))
    }

    /// Recover and validate a complete configured selection set from one
    /// repeatable-read snapshot. Input order is preserved.
    pub async fn recover_configured_selection_set(
        &self,
        configured: &[ModelRuntimeSelection],
    ) -> Result<Vec<Option<PersistedModelRegistration>>, ModelRegistryPersistenceError> {
        validate_selection_set(configured)?;
        let (mut tx, _) = self
            .begin_authority_transaction(AuthorityTransactionMode::RepeatableReadOnly)
            .await?;
        let mut recovered = Vec::with_capacity(configured.len());
        for selection in configured {
            let row =
                load_by_artifact_sha256_tx(&mut tx, &selection.artifact_sha256, false).await?;
            if let Some(row) = &row {
                ensure_selection_matches(row, selection)?;
            } else {
                // A projection rebuild must not erase the pre-artifact
                // immutable-selection gate. The EventLedger chain is the
                // durable authority when its projection row is absent.
                recover_missing_registry_audit_tx(&mut tx, selection, true, false).await?;
            }
            recovered.push(row);
        }
        tx.commit().await?;
        Ok(recovered)
    }

    /// Pre-artifact boot gate for immutable identity that can be known without
    /// reading model bytes. Runtime binding and provider are checked here;
    /// architecture-derived capabilities are checked against the loaded
    /// runtime and persisted exactly before READY exposure.
    pub async fn recover_configured_runtime_binding_set(
        &self,
        configured: &[ModelRuntimeSelection],
    ) -> Result<Vec<Option<PersistedModelRegistration>>, ModelRegistryPersistenceError> {
        validate_selection_set(configured)?;
        let (mut tx, _) = self
            .begin_authority_transaction(AuthorityTransactionMode::RepeatableReadOnly)
            .await?;
        let mut recovered = Vec::with_capacity(configured.len());
        for selection in configured {
            let row =
                load_by_artifact_sha256_tx(&mut tx, &selection.artifact_sha256, false).await?;
            if let Some(row) = &row {
                ensure_runtime_binding_matches(row, selection)?;
            } else {
                recover_missing_registry_audit_tx(&mut tx, selection, false, false).await?;
            }
            recovered.push(row);
        }
        tx.commit().await?;
        Ok(recovered)
    }

    /// Recover one configured selection. Batch boot should prefer
    /// [`Self::recover_configured_selection_set`].
    pub async fn recover_configured_selection(
        &self,
        configured: &ModelRuntimeSelection,
    ) -> Result<Option<PersistedModelRegistration>, ModelRegistryPersistenceError> {
        let mut recovered = self
            .recover_configured_selection_set(std::slice::from_ref(configured))
            .await?;
        Ok(recovered
            .pop()
            .expect("single recovery preserves cardinality"))
    }

    /// Atomically persist the complete successfully loaded boot set.
    ///
    /// All registrations are validated first, artifact locks are acquired in
    /// deterministic SHA order, and every incompatible existing selection is
    /// rejected before any event or row mutation. Initial-selection events and
    /// rows share the transaction. Results are captured under the same locks
    /// before commit and returned in input order, so a later rebind cannot
    /// rewrite this operation's receipt.
    pub async fn persist_boot_set_and_read_back(
        &self,
        registrations: &[ModelRegistration],
    ) -> Result<Vec<PersistedModelRegistration>, ModelRegistryPersistenceError> {
        let role_bound = registrations
            .iter()
            .cloned()
            .map(RoleBoundModelRegistration::completion)
            .collect::<Vec<_>>();
        self.persist_role_bound_boot_set_and_read_back(&role_bound)
            .await
    }

    /// Atomically persist a boot set with explicit completion/embedding roles.
    /// Production boot uses this path so a dedicated embedding registration can
    /// never be reconstructed as a completion-selection candidate.
    pub async fn persist_role_bound_boot_set_and_read_back(
        &self,
        registrations: &[RoleBoundModelRegistration],
    ) -> Result<Vec<PersistedModelRegistration>, ModelRegistryPersistenceError> {
        let selections = validate_role_bound_registration_set(registrations)?;
        let mut locked_hashes = selections
            .iter()
            .map(|selection| selection.artifact_sha256)
            .collect::<Vec<_>>();
        locked_hashes.sort_unstable();

        let (mut tx, authority) = self
            .begin_authority_transaction(AuthorityTransactionMode::ReadWrite)
            .await?;
        if registrations.is_empty() {
            tx.commit().await?;
            return Ok(Vec::new());
        }
        for artifact_sha256 in &locked_hashes {
            lock_model_registry_selection_tx(&mut tx, artifact_sha256).await?;
        }

        let mut existing_by_hash = BTreeMap::new();
        for selection in &selections {
            let existing =
                load_by_artifact_sha256_tx(&mut tx, &selection.artifact_sha256, true).await?;
            if let Some(existing) = &existing {
                ensure_selection_matches(existing, selection)?;
            }
            existing_by_hash.insert(selection.artifact_sha256, existing);
        }

        for (role_bound, selection) in registrations.iter().zip(&selections) {
            let registration = &role_bound.registration;
            if existing_by_hash
                .get(&selection.artifact_sha256)
                .and_then(Option::as_ref)
                .is_some()
            {
                let result = sqlx::query(
                    r#"
                    UPDATE ONLY model_runtime_registry
                    SET last_observed_runtime_model_id = $2,
                        base_model_tag = $3,
                        last_observed_by = $4,
                        last_observed_at_utc = pg_catalog.clock_timestamp()
                    WHERE artifact_sha256 = $1
                    "#,
                )
                .bind(selection.artifact_sha256.as_slice())
                .bind(registration.model_id.as_uuid())
                .bind(registration.base_model_tag.as_str())
                .bind(registration.registered_by.as_str())
                .execute(&mut *tx)
                .await
                .map_err(|error| map_sqlx_selection_error(error, &selection.artifact_sha256))?;
                if result.rows_affected() != 1 {
                    return Err(ModelRegistryPersistenceError::AuthorityUnavailable(
                        format!(
                            "model registry observation update affected {} rows for artifact {}",
                            result.rows_affected(),
                            hex::encode(selection.artifact_sha256)
                        ),
                    ));
                }
                continue;
            }

            let audit = match recover_missing_registry_audit_tx(&mut tx, selection, true, true)
                .await?
            {
                Some(audit) => audit,
                None => {
                    let registry_row_id = Uuid::now_v7();
                    let event =
                        build_initial_selection_event(registration, selection, registry_row_id)?;
                    let stored_event = append_kernel_event_with_executor(&mut *tx, event)
                        .await
                        .map_err(|error| ModelRegistryPersistenceError::Audit(error.to_string()))?;
                    repin_and_reassert_model_registry_authority_tx(&mut tx, &authority).await?;
                    RecoveredSelectionAudit {
                        registry_row_id,
                        selection_revision: 1,
                        selection_created_event_id: stored_event.event_id.clone(),
                        selection_updated_event_id: stored_event.event_id,
                        selection_created_at_utc: stored_event.created_at,
                        selection_updated_at_utc: stored_event.created_at,
                    }
                }
            };
            let selection_revision = i64::try_from(audit.selection_revision).map_err(|_| {
                ModelRegistryPersistenceError::CorruptRow(
                    "recovered model registry selection revision exceeds PostgreSQL BIGINT"
                        .to_string(),
                )
            })?;
            let registry_insert_sql = format!(
                r#"
                INSERT INTO model_runtime_registry (
                    schema_id,
                    registry_row_id,
                    artifact_sha256,
                    artifact_locator,
                    last_observed_runtime_model_id,
                    runtime_binding,
                    runtime_role,
                    capabilities_schema_id,
                    capabilities_json,
                    provider,
                    base_model_tag,
                    last_observed_by,
                    selection_revision,
                    selection_created_event_id,
                    selection_updated_event_id,
                    selection_created_at_utc,
                    selection_updated_at_utc,
                    last_observed_at_utc,
                    {RESOURCE_SCOPE_INSERT_COLUMNS}
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, pg_catalog.clock_timestamp(), $18, $19, $20, $21, $22)
                "#
            );
            self.access
                .insert_columns()
                .bind(
                    sqlx::query(&registry_insert_sql)
                        .bind(MODEL_RUNTIME_REGISTRY_SCHEMA_ID)
                        .bind(audit.registry_row_id)
                        .bind(selection.artifact_sha256.as_slice())
                        .bind(artifact_locator_for_sha256(selection.artifact_sha256))
                        .bind(registration.model_id.as_uuid())
                        .bind(runtime_binding_token(selection.runtime_binding))
                        .bind(selection.runtime_role.as_str())
                        .bind(MODEL_RUNTIME_CAPABILITIES_SCHEMA_ID)
                        .bind(serde_json::to_value(&selection.declared_capabilities)?)
                        .bind(provider_token(selection.provider))
                        .bind(registration.base_model_tag.as_str())
                        .bind(registration.registered_by.as_str())
                        .bind(selection_revision)
                        .bind(audit.selection_created_event_id)
                        .bind(audit.selection_updated_event_id)
                        .bind(audit.selection_created_at_utc)
                        .bind(audit.selection_updated_at_utc),
                )
                .execute(&mut *tx)
                .await
                .map_err(|error| map_sqlx_selection_error(error, &selection.artifact_sha256))?;
        }
        force_deferred_constraints_immediate_tx(&mut tx).await?;
        repin_and_reassert_model_registry_authority_tx(&mut tx, &authority).await?;
        let mut committed = Vec::with_capacity(selections.len());
        for (role_bound, selection) in registrations.iter().zip(&selections) {
            let registration = &role_bound.registration;
            let row = load_by_artifact_sha256_tx(&mut tx, &selection.artifact_sha256, false)
                .await?
                .ok_or_else(|| {
                    ModelRegistryPersistenceError::AuthorityUnavailable(format!(
                        "registry row disappeared before transaction commit for artifact {}",
                        hex::encode(selection.artifact_sha256)
                    ))
                })?;
            ensure_selection_matches(&row, selection)?;
            ensure_observation_matches(&row, registration)?;
            committed.push(row);
        }
        require_model_registry_synchronous_commit_tx(&mut tx).await?;
        #[cfg(feature = "test-utils")]
        if let Some(advisory_key) = self.precommit_advisory_gate_for_tests {
            sqlx::query("SELECT pg_catalog.pg_advisory_xact_lock($1)")
                .bind(advisory_key)
                .execute(&mut *tx)
                .await
                .map_err(|error| {
                    ModelRegistryPersistenceError::AuthorityUnavailable(format!(
                        "MT014_FORCED_PRECOMMIT_REGISTRY_FAILURE after registry/audit DML and read-back: {}",
                        ModelRegistryPersistenceError::from(error)
                    ))
                })?;
        }
        tx.commit().await?;
        Ok(committed)
    }

    /// Create missing purpose defaults and recover existing ones from one
    /// PostgreSQL transaction. Existing choices are never overwritten by boot.
    pub async fn ensure_active_defaults(
        &self,
        candidates: &[(ModelRuntimeSelectionPurpose, [u8; 32])],
    ) -> Result<Vec<PersistedActiveModelSelection>, ModelRegistryPersistenceError> {
        let mut seen = BTreeSet::new();
        for (purpose, _) in candidates {
            if !seen.insert(*purpose) {
                return Err(ModelRegistryPersistenceError::InvalidRegistration(format!(
                    "active default candidate set contains duplicate purpose {}",
                    purpose.as_str()
                )));
            }
        }
        let (mut tx, authority) = self
            .begin_authority_transaction(AuthorityTransactionMode::ReadWrite)
            .await?;
        let mut recovered = Vec::with_capacity(candidates.len());
        for (purpose, candidate_sha256) in candidates {
            let candidate = load_by_artifact_sha256_tx(&mut tx, candidate_sha256, true)
                .await?
                .ok_or_else(|| {
                    ModelRegistryPersistenceError::SelectionNotFound(hex::encode(candidate_sha256))
                })?;
            if candidate.runtime_role != purpose.runtime_role() {
                return Err(ModelRegistryPersistenceError::SelectionConflict(format!(
                    "active purpose {} requires role {}, but candidate artifact {} has role {}",
                    purpose.as_str(),
                    purpose.runtime_role().as_str(),
                    hex::encode(candidate_sha256),
                    candidate.runtime_role.as_str()
                )));
            }
            if let Some(existing) = load_active_selection_tx(&mut tx, *purpose, true).await? {
                let selected =
                    load_by_artifact_sha256_tx(&mut tx, &existing.artifact_sha256, false)
                        .await?
                        .ok_or_else(|| {
                            ModelRegistryPersistenceError::CorruptRow(format!(
                                "active purpose {} references absent artifact {}",
                                purpose.as_str(),
                                hex::encode(existing.artifact_sha256)
                            ))
                        })?;
                if selected.runtime_role != purpose.runtime_role()
                    || existing.runtime_role != purpose.runtime_role()
                {
                    return Err(ModelRegistryPersistenceError::CorruptRow(format!(
                        "active purpose {} role does not match its registry row",
                        purpose.as_str()
                    )));
                }
                recovered.push(existing);
                continue;
            }

            let event = build_active_selection_event(
                *purpose,
                None,
                candidate_sha256,
                1,
                KernelActor::System("model-runtime-registry".to_owned()),
                "initial active default selected during boot",
            )?;
            let stored_event = append_kernel_event_with_executor(&mut *tx, event)
                .await
                .map_err(|error| ModelRegistryPersistenceError::Audit(error.to_string()))?;
            repin_and_reassert_model_registry_authority_tx(&mut tx, &authority).await?;
            let active_selection_insert_sql = format!(
                r#"
                INSERT INTO model_runtime_active_selection (
                    schema_id, purpose, runtime_role, artifact_sha256,
                    selection_revision, selection_created_event_id,
                    selection_updated_event_id, selection_created_at_utc,
                    selection_updated_at_utc,
                    {RESOURCE_SCOPE_INSERT_COLUMNS}
                ) VALUES ($1, $2, $3, $4, 1, $5, $5, $6, $6, $7, $8, $9, $10, $11)
                "#
            );
            self.access
                .insert_columns()
                .bind(
                    sqlx::query(&active_selection_insert_sql)
                        .bind(MODEL_RUNTIME_ACTIVE_SELECTION_SCHEMA_ID)
                        .bind(purpose.as_str())
                        .bind(purpose.runtime_role().as_str())
                        .bind(candidate_sha256.as_slice())
                        .bind(&stored_event.event_id)
                        .bind(stored_event.created_at),
                )
                .execute(&mut *tx)
                .await?;
            recovered.push(
                load_active_selection_tx(&mut tx, *purpose, false)
                    .await?
                    .ok_or_else(|| {
                        ModelRegistryPersistenceError::AuthorityUnavailable(format!(
                            "active purpose {} disappeared before commit",
                            purpose.as_str()
                        ))
                    })?,
            );
        }
        require_model_registry_synchronous_commit_tx(&mut tx).await?;
        tx.commit().await?;
        Ok(recovered)
    }

    /// Read the active purpose defaults **this reader owns** from one repeatable
    /// PostgreSQL snapshot. Before scoping this was a full-table read, so any
    /// caller learned which model every account had made its default.
    pub async fn list_active_selections(
        &self,
    ) -> Result<Vec<PersistedActiveModelSelection>, ModelRegistryPersistenceError> {
        let (mut tx, _) = self
            .begin_authority_transaction(AuthorityTransactionMode::RepeatableReadOnly)
            .await?;
        let predicate = self.access.sql_predicate(1);
        let active_selection_sql = format!(
            r#"
            SELECT schema_id, purpose, runtime_role, artifact_sha256, selection_revision,
                   selection_created_event_id, selection_updated_event_id,
                   selection_created_at_utc, selection_updated_at_utc,
                   {RESOURCE_SCOPE_SELECT_COLUMNS}
            FROM ONLY model_runtime_active_selection
            WHERE TRUE{}
            ORDER BY purpose ASC
            LIMIT 3
            "#,
            predicate.clause()
        );
        let rows = predicate
            .bind(sqlx::query(&active_selection_sql))
            .fetch_all(&mut *tx)
            .await?;
        if rows.len() > 2 {
            return Err(ModelRegistryPersistenceError::CorruptRow(
                "active ModelRuntime selection authority contains more than two purposes"
                    .to_owned(),
            ));
        }
        // Second enforcement layer (HBR-PRIV-002): re-authorize the scope
        // columns that came back, so a future edit to the predicate above cannot
        // silently turn this back into a full-table disclosure.
        for row in &rows {
            self.access
                .authorize_row(&stored_resource_scope_from_row(row)?)?;
        }
        let decoded = rows
            .into_iter()
            .map(decode_active_selection)
            .collect::<Result<Vec<_>, _>>()?;
        for selection in &decoded {
            validate_active_selection_audit_tx(&mut tx, selection, false).await?;
        }
        tx.commit().await?;
        Ok(decoded)
    }

    /// Audited compare-and-set of one active purpose default. All validation,
    /// EventLedger append, mutation, and read-back share the transaction.
    pub async fn select_active_model(
        &self,
        purpose: ModelRuntimeSelectionPurpose,
        target_artifact_sha256: [u8; 32],
        expected_revision: u64,
        actor: KernelActor,
        reason: &str,
    ) -> Result<PersistedActiveModelSelection, ModelRegistryPersistenceError> {
        if expected_revision == 0 {
            return Err(ModelRegistryPersistenceError::InvalidRebind(
                "active selection expected revision must be at least one".to_owned(),
            ));
        }
        if !matches!(actor, KernelActor::Operator(_))
            || actor.actor_id().trim().is_empty()
            || actor.actor_id().len() > MODEL_REGISTRY_INPUT_TEXT_BYTE_CAP
            || reason.trim().is_empty()
            || reason.len() > MODEL_REGISTRY_INPUT_TEXT_BYTE_CAP
        {
            return Err(ModelRegistryPersistenceError::InvalidRebind(
                "active selection requires a bounded explicit operator actor and reason".to_owned(),
            ));
        }
        let (mut tx, authority) = self
            .begin_authority_transaction(AuthorityTransactionMode::ReadWrite)
            .await?;
        let existing = load_active_selection_tx(&mut tx, purpose, true)
            .await?
            .ok_or_else(|| {
                ModelRegistryPersistenceError::SelectionNotFound(purpose.as_str().to_owned())
            })?;
        if existing.selection_revision != expected_revision {
            return Err(ModelRegistryPersistenceError::SelectionRevisionMismatch {
                expected: expected_revision,
                actual: existing.selection_revision,
            });
        }
        if existing.artifact_sha256 == target_artifact_sha256 {
            return Err(ModelRegistryPersistenceError::InvalidRebind(
                "target active default is already selected".to_owned(),
            ));
        }
        let target = load_by_artifact_sha256_tx(&mut tx, &target_artifact_sha256, false)
            .await?
            .ok_or_else(|| {
                ModelRegistryPersistenceError::SelectionNotFound(hex::encode(
                    target_artifact_sha256,
                ))
            })?;
        if target.runtime_role != purpose.runtime_role() {
            return Err(ModelRegistryPersistenceError::SelectionConflict(format!(
                "artifact {} has role {} and cannot satisfy purpose {}",
                hex::encode(target_artifact_sha256),
                target.runtime_role.as_str(),
                purpose.as_str()
            )));
        }
        let next_revision = existing.selection_revision.checked_add(1).ok_or_else(|| {
            ModelRegistryPersistenceError::InvalidRebind(
                "active selection revision cannot be incremented".to_owned(),
            )
        })?;
        let event = build_active_selection_event(
            purpose,
            Some(&existing),
            &target_artifact_sha256,
            next_revision,
            actor,
            reason,
        )?;
        let stored_event = append_kernel_event_with_executor(&mut *tx, event)
            .await
            .map_err(|error| ModelRegistryPersistenceError::Audit(error.to_string()))?;
        repin_and_reassert_model_registry_authority_tx(&mut tx, &authority).await?;
        let result = sqlx::query(
            r#"
            UPDATE ONLY model_runtime_active_selection
            SET artifact_sha256 = $3,
                selection_revision = $4,
                selection_updated_event_id = $5,
                selection_updated_at_utc = $6
            WHERE purpose = $1 AND selection_revision = $2
            "#,
        )
        .bind(purpose.as_str())
        .bind(i64::try_from(expected_revision).map_err(|_| {
            ModelRegistryPersistenceError::InvalidRebind(
                "active selection expected revision exceeds PostgreSQL BIGINT".to_owned(),
            )
        })?)
        .bind(target_artifact_sha256.as_slice())
        .bind(i64::try_from(next_revision).map_err(|_| {
            ModelRegistryPersistenceError::InvalidRebind(
                "active selection next revision exceeds PostgreSQL BIGINT".to_owned(),
            )
        })?)
        .bind(&stored_event.event_id)
        .bind(stored_event.created_at)
        .execute(&mut *tx)
        .await?;
        if result.rows_affected() != 1 {
            return Err(ModelRegistryPersistenceError::SelectionRevisionMismatch {
                expected: expected_revision,
                actual: existing.selection_revision,
            });
        }
        let committed = load_active_selection_tx(&mut tx, purpose, false)
            .await?
            .ok_or_else(|| {
                ModelRegistryPersistenceError::AuthorityUnavailable(format!(
                    "active purpose {} disappeared before commit",
                    purpose.as_str()
                ))
            })?;
        if committed.artifact_sha256 != target_artifact_sha256
            || committed.selection_revision != next_revision
        {
            return Err(ModelRegistryPersistenceError::AuthorityUnavailable(
                format!(
                    "active purpose {} failed transactional read-back",
                    purpose.as_str()
                ),
            ));
        }
        require_model_registry_synchronous_commit_tx(&mut tx).await?;
        tx.commit().await?;
        Ok(committed)
    }

    /// Persist one successfully loaded registration. Batch boot should prefer
    /// [`Self::persist_boot_set_and_read_back`].
    pub async fn persist_and_read_back(
        &self,
        registration: &ModelRegistration,
    ) -> Result<PersistedModelRegistration, ModelRegistryPersistenceError> {
        let mut committed = self
            .persist_boot_set_and_read_back(std::slice::from_ref(registration))
            .await?;
        Ok(committed
            .pop()
            .expect("single registration persistence preserves cardinality"))
    }

    /// Apply an audited immutable-selection compare-and-swap after the runtime
    /// owner has proved the previous adapter is unloaded.
    ///
    /// This is a production-compiled kernel primitive, not an operator-facing
    /// route. The caller owns the unload proof; the store owns revision
    /// comparison, EventLedger evidence, read-back, and atomic commit.
    pub async fn rebind_selection_after_verified_unload(
        &self,
        target: &ModelRuntimeSelection,
        request: ExplicitModelRuntimeRebind,
    ) -> Result<PersistedModelRegistration, ModelRegistryPersistenceError> {
        validate_selection(target)?;
        validate_rebind_request(&request)?;
        let (mut tx, authority) = self
            .begin_authority_transaction(AuthorityTransactionMode::ReadWrite)
            .await?;
        lock_model_registry_selection_tx(&mut tx, &target.artifact_sha256).await?;
        let existing = load_by_artifact_sha256_tx(&mut tx, &target.artifact_sha256, true)
            .await?
            .ok_or_else(|| {
                ModelRegistryPersistenceError::SelectionNotFound(hex::encode(
                    target.artifact_sha256,
                ))
            })?;
        if existing.selection_revision != request.expected_selection_revision {
            return Err(ModelRegistryPersistenceError::SelectionRevisionMismatch {
                expected: request.expected_selection_revision,
                actual: existing.selection_revision,
            });
        }
        if existing.runtime_role != target.runtime_role {
            return Err(ModelRegistryPersistenceError::InvalidRebind(format!(
                "runtime role is artifact authority and cannot be rebound from {:?} to {:?}",
                existing.runtime_role, target.runtime_role
            )));
        }
        if existing.selection() == *target {
            return Err(ModelRegistryPersistenceError::InvalidRebind(
                "target immutable selection is unchanged".to_string(),
            ));
        }
        let next_revision = existing.selection_revision.checked_add(1).ok_or_else(|| {
            ModelRegistryPersistenceError::InvalidRebind(
                "selection revision cannot be incremented".to_string(),
            )
        })?;
        let event = build_rebind_event(&existing, target, &request, next_revision)?;
        let stored_event = append_kernel_event_with_executor(&mut *tx, event)
            .await
            .map_err(|error| ModelRegistryPersistenceError::Audit(error.to_string()))?;
        repin_and_reassert_model_registry_authority_tx(&mut tx, &authority).await?;
        let result = sqlx::query(
            r#"
            UPDATE ONLY model_runtime_registry
            SET runtime_binding = $3,
                capabilities_json = $4,
                provider = $5,
                selection_revision = $6,
                selection_updated_event_id = $7,
                selection_updated_at_utc = $8
            WHERE artifact_sha256 = $1 AND selection_revision = $2
            "#,
        )
        .bind(target.artifact_sha256.as_slice())
        .bind(
            i64::try_from(request.expected_selection_revision).map_err(|_| {
                ModelRegistryPersistenceError::InvalidRebind(
                    "expected selection revision exceeds PostgreSQL BIGINT".to_string(),
                )
            })?,
        )
        .bind(runtime_binding_token(target.runtime_binding))
        .bind(serde_json::to_value(&target.declared_capabilities)?)
        .bind(provider_token(target.provider))
        .bind(i64::try_from(next_revision).map_err(|_| {
            ModelRegistryPersistenceError::InvalidRebind(
                "next selection revision exceeds PostgreSQL BIGINT".to_string(),
            )
        })?)
        .bind(stored_event.event_id)
        .bind(stored_event.created_at)
        .execute(&mut *tx)
        .await
        .map_err(|error| map_sqlx_selection_error(error, &target.artifact_sha256))?;
        if result.rows_affected() != 1 {
            return Err(ModelRegistryPersistenceError::SelectionRevisionMismatch {
                expected: request.expected_selection_revision,
                actual: existing.selection_revision,
            });
        }
        force_deferred_constraints_immediate_tx(&mut tx).await?;
        repin_and_reassert_model_registry_authority_tx(&mut tx, &authority).await?;
        let committed = load_by_artifact_sha256_tx(&mut tx, &target.artifact_sha256, false)
            .await?
            .ok_or_else(|| {
                ModelRegistryPersistenceError::AuthorityUnavailable(format!(
                    "rebound registry row disappeared before transaction commit for artifact {}",
                    hex::encode(target.artifact_sha256)
                ))
            })?;
        ensure_selection_matches(&committed, target)?;
        if committed.selection_revision != next_revision {
            return Err(ModelRegistryPersistenceError::AuthorityUnavailable(
                format!(
                    "committed rebind readback revision is {}, expected {next_revision}",
                    committed.selection_revision
                ),
            ));
        }
        require_model_registry_synchronous_commit_tx(&mut tx).await?;
        tx.commit().await?;
        Ok(committed)
    }

    /// Integration-only proof seam for the raw audited compare-and-swap.
    ///
    /// Production code must not expose this primitive as an operator action:
    /// SPEC §4.6.2 requires the runtime owner to prove unload before a governed
    /// unload-and-re-register workflow can change immutable selection.
    #[cfg(feature = "test-utils")]
    pub async fn rebind_selection_for_tests(
        &self,
        target: &ModelRuntimeSelection,
        request: ExplicitModelRuntimeRebind,
    ) -> Result<PersistedModelRegistration, ModelRegistryPersistenceError> {
        self.rebind_selection_after_verified_unload(target, request)
            .await
    }

    /// Recover one durable registry row by stable artifact identity.
    pub async fn load_by_artifact_sha256(
        &self,
        artifact_sha256: &[u8; 32],
    ) -> Result<Option<PersistedModelRegistration>, ModelRegistryPersistenceError> {
        let (mut tx, _) = self
            .begin_authority_transaction(AuthorityTransactionMode::RepeatableReadOnly)
            .await?;
        let row = load_by_artifact_sha256_tx(&mut tx, artifact_sha256, false).await?;
        tx.commit().await?;
        Ok(row)
    }

    /// Enumerate the committed durable selections **this reader owns**, in
    /// stable artifact-hash order.
    ///
    /// Before scoping this enumerated the whole `model_runtime_registry` table,
    /// so `GET /model-runtime/registry` disclosed every account's registered
    /// model artifacts to any caller (HBR-PRIV-002). The owner predicate is
    /// applied to both the transfer-budget probe and the row read, so the
    /// bounded-enumeration caps are computed over the same row set the caller is
    /// actually allowed to see.
    pub async fn list_recoverable(
        &self,
    ) -> Result<Vec<PersistedModelRegistration>, ModelRegistryPersistenceError> {
        let (mut tx, _) = self
            .begin_authority_transaction(AuthorityTransactionMode::RepeatableReadOnly)
            .await?;
        let probe_predicate = self.access.sql_predicate(2);
        let transfer_shape_sql = format!(
            r#"
            WITH bounded AS (
                SELECT pg_catalog.octet_length(capabilities_json::pg_catalog.text)::pg_catalog.int8
                           AS capabilities_bytes,
                       (
                           pg_catalog.octet_length(schema_id)::pg_catalog.int8
                           + pg_catalog.octet_length(artifact_sha256)::pg_catalog.int8
                           + pg_catalog.octet_length(artifact_locator)::pg_catalog.int8
                           + pg_catalog.octet_length(runtime_binding)::pg_catalog.int8
                           + pg_catalog.octet_length(runtime_role)::pg_catalog.int8
                           + pg_catalog.octet_length(capabilities_schema_id)::pg_catalog.int8
                           + pg_catalog.octet_length(capabilities_json::pg_catalog.text)::pg_catalog.int8
                           + pg_catalog.octet_length(provider)::pg_catalog.int8
                           + pg_catalog.octet_length(base_model_tag)::pg_catalog.int8
                           + pg_catalog.octet_length(last_observed_by)::pg_catalog.int8
                           + pg_catalog.octet_length(selection_created_event_id)::pg_catalog.int8
                           + pg_catalog.octet_length(selection_updated_event_id)::pg_catalog.int8
                           + 128
                       )::pg_catalog.int8 AS row_bytes
                FROM ONLY model_runtime_registry
                WHERE TRUE{}
                ORDER BY artifact_sha256 ASC
                LIMIT $1
            )
            SELECT pg_catalog.count(*)::pg_catalog.int8 AS row_count,
                   COALESCE(pg_catalog.max(capabilities_bytes), 0)::pg_catalog.int8
                       AS max_capabilities_bytes,
                   COALESCE(pg_catalog.max(row_bytes), 0)::pg_catalog.int8
                       AS max_row_bytes,
                   COALESCE(pg_catalog.sum(row_bytes), 0)::pg_catalog.int8
                       AS total_row_bytes
            FROM bounded
            "#,
            probe_predicate.clause()
        );
        let transfer_shape = probe_predicate
            .bind(sqlx::query(&transfer_shape_sql).bind(MODEL_REGISTRY_ROW_ENUMERATION_CAP + 1))
            .fetch_one(&mut *tx)
            .await?;
        let row_count: i64 = transfer_shape.try_get("row_count")?;
        if row_count > MODEL_REGISTRY_ROW_ENUMERATION_CAP {
            return Err(ModelRegistryPersistenceError::AuthorityUnavailable(
                format!(
                    "model registry enumeration exceeds the bounded {}-row limit",
                    MODEL_REGISTRY_ROW_ENUMERATION_CAP
                ),
            ));
        }
        enforce_registry_row_transfer_budget(
            transfer_shape.try_get("max_capabilities_bytes")?,
            transfer_shape.try_get("max_row_bytes")?,
        )?;
        let total_row_bytes: i64 = transfer_shape.try_get("total_row_bytes")?;
        if total_row_bytes > MODEL_REGISTRY_ROW_SET_BYTE_CAP {
            return Err(ModelRegistryPersistenceError::AuthorityUnavailable(format!(
                "model registry enumeration is {total_row_bytes} variable bytes, exceeding the bounded {MODEL_REGISTRY_ROW_SET_BYTE_CAP}-byte transfer limit"
            )));
        }
        let row_predicate = self.access.sql_predicate(2);
        let statement = format!(
            "SELECT {MODEL_REGISTRY_SELECT_COLUMNS}, {RESOURCE_SCOPE_SELECT_COLUMNS} FROM ONLY model_runtime_registry WHERE TRUE{} ORDER BY artifact_sha256 ASC LIMIT $1",
            row_predicate.clause()
        );
        let rows = row_predicate
            .bind(sqlx::query(&statement).bind(MODEL_REGISTRY_ROW_ENUMERATION_CAP + 1))
            .fetch_all(&mut *tx)
            .await?;
        if i64::try_from(rows.len()).unwrap_or(i64::MAX) > MODEL_REGISTRY_ROW_ENUMERATION_CAP {
            return Err(ModelRegistryPersistenceError::AuthorityUnavailable(
                format!(
                    "model registry enumeration exceeds the bounded {}-row limit",
                    MODEL_REGISTRY_ROW_ENUMERATION_CAP
                ),
            ));
        }
        // Second enforcement layer (HBR-PRIV-002).
        for row in &rows {
            self.access
                .authorize_row(&stored_resource_scope_from_row(row)?)?;
        }
        let registrations = rows
            .into_iter()
            .map(decode_row)
            .collect::<Result<Vec<_>, _>>()?;
        let total_revisions = registrations
            .iter()
            .try_fold(0_u64, |total, registration| {
                total
                    .checked_add(registration.selection_revision)
                    .ok_or_else(|| {
                        ModelRegistryPersistenceError::AuthorityUnavailable(
                            "model registry enumeration audit revision total overflowed"
                                .to_string(),
                        )
                    })
            })?;
        if total_revisions > MODEL_REGISTRY_ENUMERATION_AUDIT_EVENT_CAP {
            return Err(ModelRegistryPersistenceError::AuthorityUnavailable(format!(
                "model registry enumeration requires {total_revisions} audit events, exceeding the bounded {}-event total",
                MODEL_REGISTRY_ENUMERATION_AUDIT_EVENT_CAP
            )));
        }
        let aggregate_ids = registrations
            .iter()
            .map(|registration| registration.artifact_locator.clone())
            .collect::<Vec<_>>();
        let events = load_selection_events_for_aggregates_tx(
            &mut tx,
            &aggregate_ids,
            i64::try_from(MODEL_REGISTRY_ENUMERATION_AUDIT_EVENT_CAP).unwrap_or(i64::MAX) + 1,
            false,
        )
        .await?;
        if u64::try_from(events.len()).unwrap_or(u64::MAX)
            > MODEL_REGISTRY_ENUMERATION_AUDIT_EVENT_CAP
        {
            return Err(ModelRegistryPersistenceError::AuthorityUnavailable(
                format!(
                    "model registry enumeration audit rows exceed the bounded {}-event total",
                    MODEL_REGISTRY_ENUMERATION_AUDIT_EVENT_CAP
                ),
            ));
        }
        let mut events_by_aggregate = BTreeMap::<String, Vec<PersistedSelectionEvent>>::new();
        for event in events {
            events_by_aggregate
                .entry(event.aggregate_id.clone())
                .or_default()
                .push(event);
        }
        for registration in &registrations {
            let events = events_by_aggregate
                .remove(&registration.artifact_locator)
                .unwrap_or_default();
            validate_selection_audit_chain(registration, &events)?;
        }
        tx.commit().await?;
        Ok(registrations)
    }

    /// Integration-only proof that authority transactions enforce and type-map
    /// the production statement deadline.
    #[doc(hidden)]
    #[cfg(feature = "test-utils")]
    pub async fn prove_statement_timeout_for_tests(
        &self,
    ) -> Result<(), ModelRegistryPersistenceError> {
        let (mut tx, _) = self
            .begin_authority_transaction(AuthorityTransactionMode::RepeatableReadOnly)
            .await?;
        sqlx::query("SELECT pg_catalog.pg_sleep($1)")
            .bind(MODEL_REGISTRY_DATABASE_STATEMENT_TIMEOUT.as_secs_f64() + 5.0)
            .execute(&mut *tx)
            .await?;
        Err(ModelRegistryPersistenceError::AuthorityUnavailable(
            "PostgreSQL statement timeout probe unexpectedly completed".to_string(),
        ))
    }

    /// Integration-only seam that blocks after PostgreSQL has accepted BEGIN
    /// and installed the production transaction-local deadlines. Its test-only
    /// three-second server lock and statement deadlines outlive the two-second
    /// client deadline but still guarantee bounded server unwind. Tests cancel
    /// or time out this future to prove the physical connection is never
    /// returned to the pool with an untracked transaction.
    #[doc(hidden)]
    #[cfg(feature = "test-utils")]
    pub async fn prove_cancel_safe_transaction_start_for_tests(
        &self,
        advisory_key: i64,
    ) -> Result<(), ModelRegistryPersistenceError> {
        let (tx, _) = self
            .begin_authority_transaction_inner(
                AuthorityTransactionMode::ReadWrite,
                Some(advisory_key),
            )
            .await?;
        tx.commit().await
    }

    /// Integration-only one-shot seam for proving that an EventLedger payload
    /// fetch is restricted to the fixed-size sequence keys returned by its own
    /// byte preflight, even under a READ COMMITTED insert interleaving.
    #[doc(hidden)]
    #[cfg(feature = "test-utils")]
    pub async fn prove_event_byte_preflight_sequence_pin_for_tests(
        &self,
        aggregate_id: &str,
    ) -> Result<usize, ModelRegistryPersistenceError> {
        if aggregate_id.trim().is_empty() || aggregate_id.len() > MODEL_REGISTRY_INPUT_TEXT_BYTE_CAP
        {
            return Err(ModelRegistryPersistenceError::InvalidRegistration(
                "test aggregate id is empty or exceeds the bounded persistence limit".to_string(),
            ));
        }
        let (mut tx, _) = self
            .begin_authority_transaction(AuthorityTransactionMode::ReadWrite)
            .await?;
        let events = load_selection_events_for_aggregates_tx(
            &mut tx,
            &[aggregate_id.to_string()],
            MODEL_REGISTRY_AUDIT_EVENT_CAP + 1,
            true,
        )
        .await?;
        let count = events.len();
        tx.commit().await?;
        Ok(count)
    }
}

async fn resolve_authority_tx(
    tx: &mut AuthorityTransaction,
) -> Result<ModelRegistryAuthority, ModelRegistryPersistenceError> {
    let rows = sqlx::query(
        r#"
        SELECT namespace.nspname AS schema_name,
               relation.oid::pg_catalog.int8 AS relation_oid,
               event_ledger.oid::pg_catalog.int8 AS event_ledger_oid
        FROM pg_catalog.pg_class AS relation
        JOIN pg_catalog.pg_namespace AS namespace ON namespace.oid = relation.relnamespace
        JOIN pg_catalog.pg_class AS event_ledger
          ON event_ledger.relnamespace = namespace.oid
         AND event_ledger.relname = 'kernel_event_ledger'
        WHERE relation.relname = $1
          AND namespace.nspname = ANY (pg_catalog.current_schemas(false))
        ORDER BY pg_catalog.array_position(
            pg_catalog.current_schemas(false),
            namespace.nspname
        )
        "#,
    )
    .bind(MODEL_RUNTIME_REGISTRY_TABLE)
    .fetch_all(&mut **tx)
    .await?;
    let relations = rows
        .into_iter()
        .map(|row| -> Result<ModelRegistryAuthority, sqlx::Error> {
            Ok(ModelRegistryAuthority {
                schema: row.try_get("schema_name")?,
                relation_oid: row.try_get("relation_oid")?,
                event_ledger_oid: row.try_get("event_ledger_oid")?,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()?;
    match relations.as_slice() {
        [authority] => Ok(authority.clone()),
        [] => Err(ModelRegistryPersistenceError::AuthorityUnavailable(
            "model_runtime_registry is absent from the configured PostgreSQL search path; run the current migration chain"
                .to_string(),
        )),
        authorities => Err(ModelRegistryPersistenceError::AuthorityUnavailable(format!(
            "model_runtime_registry authority is ambiguous across configured schemas: {}",
            authorities
                .iter()
                .map(|authority| authority.schema.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
}

async fn assert_model_registry_authority_tx(
    tx: &mut AuthorityTransaction,
    authority: &ModelRegistryAuthority,
) -> Result<(), ModelRegistryPersistenceError> {
    let relation = sqlx::query(
        r#"
        SELECT namespace.nspname AS schema_name,
               relation.relname AS relation_name,
               relation.relkind::pg_catalog.text AS relation_kind,
               relation.relpersistence::pg_catalog.text AS relation_persistence
        FROM pg_catalog.pg_class AS relation
        JOIN pg_catalog.pg_namespace AS namespace ON namespace.oid = relation.relnamespace
        WHERE relation.oid = $1::pg_catalog.oid
          AND NOT EXISTS (
              SELECT 1
              FROM pg_catalog.pg_inherits AS inheritance
              WHERE inheritance.inhparent = relation.oid
                 OR inheritance.inhrelid = relation.oid
          )
        "#,
    )
    .bind(authority.relation_oid)
    .fetch_optional(&mut **tx)
    .await?;
    let Some(relation) = relation else {
        return Err(ModelRegistryPersistenceError::AuthorityUnavailable(
            format!(
                "{}.model_runtime_registry changed identity",
                authority.schema
            ),
        ));
    };
    let schema_name: String = relation.try_get("schema_name")?;
    let relation_name: String = relation.try_get("relation_name")?;
    if schema_name != authority.schema || relation_name != MODEL_RUNTIME_REGISTRY_TABLE {
        return Err(ModelRegistryPersistenceError::AuthorityUnavailable(
            format!(
                "{}.model_runtime_registry changed identity",
                authority.schema
            ),
        ));
    }
    let relation_kind: String = relation.try_get("relation_kind")?;
    if relation_kind != "r" {
        return Err(ModelRegistryPersistenceError::AuthorityUnavailable(format!(
            "{}.model_runtime_registry must be an ordinary PostgreSQL table; relkind={relation_kind}",
            authority.schema
        )));
    }
    let relation_persistence: String = relation.try_get("relation_persistence")?;
    if relation_persistence != "p" {
        return Err(ModelRegistryPersistenceError::AuthorityUnavailable(format!(
            "{}.model_runtime_registry must be a permanent logged PostgreSQL table; relpersistence={relation_persistence}",
            authority.schema
        )));
    }
    assert_relation_hook_free_tx(
        tx,
        authority.relation_oid,
        &authority.schema,
        MODEL_RUNTIME_REGISTRY_TABLE,
    )
    .await?;
    assert_relation_hook_free_tx(
        tx,
        authority.event_ledger_oid,
        &authority.schema,
        "kernel_event_ledger",
    )
    .await?;
    Ok(())
}

async fn assert_relation_hook_free_tx(
    tx: &mut AuthorityTransaction,
    relation_oid: i64,
    expected_schema: &str,
    expected_name: &str,
) -> Result<(), ModelRegistryPersistenceError> {
    let posture = sqlx::query(
        r#"
        SELECT namespace.nspname AS schema_name,
               relation.relname AS relation_name,
               relation.relkind::pg_catalog.text AS relation_kind,
               relation.relpersistence::pg_catalog.text AS relation_persistence,
               relation.relrowsecurity,
               relation.relforcerowsecurity,
               EXISTS (
                   SELECT 1 FROM pg_catalog.pg_inherits AS inheritance
                   WHERE inheritance.inhparent = relation.oid
                      OR inheritance.inhrelid = relation.oid
               ) AS has_inheritance,
               EXISTS (
                   SELECT 1 FROM pg_catalog.pg_trigger AS trigger_row
                   WHERE trigger_row.tgrelid = relation.oid
                     AND NOT trigger_row.tgisinternal
                     AND trigger_row.tgenabled <> 'D'
               ) AS has_enabled_user_trigger,
               EXISTS (
                   SELECT 1 FROM pg_catalog.pg_rewrite AS rule_row
                   WHERE rule_row.ev_class = relation.oid
                     AND rule_row.rulename <> '_RETURN'
                     AND rule_row.ev_enabled <> 'D'
               ) AS has_enabled_user_rule,
               EXISTS (
                   SELECT 1 FROM pg_catalog.pg_policy AS policy_row
                   WHERE policy_row.polrelid = relation.oid
               ) AS has_policy
        FROM pg_catalog.pg_class AS relation
        JOIN pg_catalog.pg_namespace AS namespace ON namespace.oid = relation.relnamespace
        WHERE relation.oid = $1::pg_catalog.oid
        "#,
    )
    .bind(relation_oid)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| {
        ModelRegistryPersistenceError::AuthorityUnavailable(format!(
            "{expected_schema}.{expected_name} changed identity"
        ))
    })?;
    let posture_matches = posture.try_get::<String, _>("schema_name")? == expected_schema
        && posture.try_get::<String, _>("relation_name")? == expected_name
        && posture.try_get::<String, _>("relation_kind")? == "r"
        && posture.try_get::<String, _>("relation_persistence")? == "p"
        && !posture.try_get::<bool, _>("relrowsecurity")?
        && !posture.try_get::<bool, _>("relforcerowsecurity")?
        && !posture.try_get::<bool, _>("has_inheritance")?
        && !posture.try_get::<bool, _>("has_enabled_user_trigger")?
        && !posture.try_get::<bool, _>("has_enabled_user_rule")?
        && !posture.try_get::<bool, _>("has_policy")?;
    if !posture_matches {
        return Err(ModelRegistryPersistenceError::AuthorityUnavailable(format!(
            "{expected_schema}.{expected_name} must remain a permanent ordinary non-inherited authority with RLS disabled and no enabled user trigger, rule, or policy"
        )));
    }
    Ok(())
}

async fn lock_model_registry_authority_tx(
    tx: &mut AuthorityTransaction,
    authority: &ModelRegistryAuthority,
    mode: AuthorityTransactionMode,
) -> Result<(), ModelRegistryPersistenceError> {
    let qualified_table = format!(
        "{}.{}",
        quote_pg_identifier(&authority.schema),
        quote_pg_identifier(MODEL_RUNTIME_REGISTRY_TABLE)
    );
    let qualified_event_ledger = format!(
        "{}.{}",
        quote_pg_identifier(&authority.schema),
        quote_pg_identifier("kernel_event_ledger")
    );
    let statement = match mode {
        AuthorityTransactionMode::ReadWrite => format!(
            "LOCK TABLE ONLY {qualified_table} IN SHARE UPDATE EXCLUSIVE MODE; LOCK TABLE ONLY {qualified_event_ledger} IN ROW EXCLUSIVE MODE"
        ),
        AuthorityTransactionMode::RepeatableReadOnly => format!(
            "LOCK TABLE ONLY {qualified_table}, ONLY {qualified_event_ledger} IN ACCESS SHARE MODE"
        ),
    };
    (&mut **tx).execute(sqlx::raw_sql(&statement)).await?;
    Ok(())
}

fn quote_pg_identifier(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

async fn require_model_registry_crash_durability_tx(
    tx: &mut AuthorityTransaction,
) -> Result<(), ModelRegistryPersistenceError> {
    let (fsync, full_page_writes): (String, String) = sqlx::query_as(
        r#"
        SELECT pg_catalog.current_setting('fsync'),
               pg_catalog.current_setting('full_page_writes')
        "#,
    )
    .fetch_one(&mut **tx)
    .await?;
    if fsync != "on" || full_page_writes != "on" {
        return Err(ModelRegistryPersistenceError::AuthorityUnavailable(
            format!(
                "model registry requires PostgreSQL fsync=on and full_page_writes=on for crash-durable authority; got fsync={fsync}, full_page_writes={full_page_writes}"
            ),
        ));
    }
    Ok(())
}

async fn require_model_registry_synchronous_commit_tx(
    tx: &mut AuthorityTransaction,
) -> Result<(), ModelRegistryPersistenceError> {
    let synchronous_commit: String =
        sqlx::query_scalar("SELECT pg_catalog.set_config('synchronous_commit', 'on', true)")
            .fetch_one(&mut **tx)
            .await?;
    if synchronous_commit != "on" {
        return Err(ModelRegistryPersistenceError::AuthorityUnavailable(
            format!(
                "failed to require synchronous PostgreSQL commit for model registry mutation; got {synchronous_commit}"
            ),
        ));
    }
    Ok(())
}

async fn force_deferred_constraints_immediate_tx(
    tx: &mut AuthorityTransaction,
) -> Result<(), ModelRegistryPersistenceError> {
    // Constraint triggers may be DEFERRABLE INITIALLY DEFERRED and can mutate
    // rows at commit after an apparently successful readback. Fire every
    // queued event now; no registry DML occurs after the subsequent readback,
    // so commit cannot invalidate the receipt that boot is about to trust.
    sqlx::query("SET CONSTRAINTS ALL IMMEDIATE")
        .execute(&mut **tx)
        .await?;
    Ok(())
}

async fn pin_authority_schema_tx(
    tx: &mut AuthorityTransaction,
    schema: &str,
) -> Result<(), ModelRegistryPersistenceError> {
    // List pg_catalog first so same-named functions/operators in an authority
    // schema cannot alter lock or constraint semantics. List pg_temp explicitly
    // last; otherwise PostgreSQL implicitly searches it first for relations.
    let pinned: String = sqlx::query_scalar(
        r#"
        SELECT pg_catalog.set_config(
            'search_path',
            'pg_catalog, ' || pg_catalog.quote_ident($1) || ', pg_temp',
            true
        )
        "#,
    )
    .bind(schema)
    .fetch_one(&mut **tx)
    .await?;
    if pinned.trim().is_empty() {
        return Err(ModelRegistryPersistenceError::AuthorityUnavailable(
            "failed to pin the model registry authority schema".to_string(),
        ));
    }
    Ok(())
}

async fn repin_and_reassert_model_registry_authority_tx(
    tx: &mut AuthorityTransaction,
    authority: &ModelRegistryAuthority,
) -> Result<(), ModelRegistryPersistenceError> {
    // A database expression executes inside the server and can change
    // transaction-local GUCs. Re-pin before any subsequent unqualified legacy
    // EventLedger helper/readback statement, then prove the locked OIDs and
    // their complete behavior-bearing shapes still match authority.
    pin_authority_schema_tx(tx, &authority.schema).await?;
    assert_model_registry_authority_tx(tx, authority).await?;
    ensure_authority_available_tx(tx, authority).await?;
    Ok(())
}

async fn ensure_authority_available_tx(
    tx: &mut AuthorityTransaction,
    authority: &ModelRegistryAuthority,
) -> Result<(), ModelRegistryPersistenceError> {
    let schema = &authority.schema;

    let rows = sqlx::query(
        r#"
        SELECT attribute.attname AS column_name,
               CASE attribute.attname
                   WHEN 'schema_id' THEN attribute.atttypid = 'pg_catalog.text'::pg_catalog.regtype
                   WHEN 'registry_row_id' THEN attribute.atttypid = 'pg_catalog.uuid'::pg_catalog.regtype
                   WHEN 'artifact_sha256' THEN attribute.atttypid = 'pg_catalog.bytea'::pg_catalog.regtype
                   WHEN 'artifact_locator' THEN attribute.atttypid = 'pg_catalog.text'::pg_catalog.regtype
                   WHEN 'last_observed_runtime_model_id' THEN attribute.atttypid = 'pg_catalog.uuid'::pg_catalog.regtype
                   WHEN 'runtime_binding' THEN attribute.atttypid = 'pg_catalog.text'::pg_catalog.regtype
                   WHEN 'runtime_role' THEN attribute.atttypid = 'pg_catalog.text'::pg_catalog.regtype
                   WHEN 'capabilities_schema_id' THEN attribute.atttypid = 'pg_catalog.text'::pg_catalog.regtype
                   WHEN 'capabilities_json' THEN attribute.atttypid = 'pg_catalog.jsonb'::pg_catalog.regtype
                   WHEN 'provider' THEN attribute.atttypid = 'pg_catalog.text'::pg_catalog.regtype
                   WHEN 'base_model_tag' THEN attribute.atttypid = 'pg_catalog.text'::pg_catalog.regtype
                   WHEN 'last_observed_by' THEN attribute.atttypid = 'pg_catalog.text'::pg_catalog.regtype
                   WHEN 'selection_revision' THEN attribute.atttypid = 'pg_catalog.int8'::pg_catalog.regtype
                   WHEN 'selection_created_event_id' THEN attribute.atttypid = 'pg_catalog.text'::pg_catalog.regtype
                   WHEN 'selection_updated_event_id' THEN attribute.atttypid = 'pg_catalog.text'::pg_catalog.regtype
                   WHEN 'selection_created_at_utc' THEN attribute.atttypid = 'pg_catalog.timestamptz'::pg_catalog.regtype
                   WHEN 'selection_updated_at_utc' THEN attribute.atttypid = 'pg_catalog.timestamptz'::pg_catalog.regtype
                   WHEN 'last_observed_at_utc' THEN attribute.atttypid = 'pg_catalog.timestamptz'::pg_catalog.regtype
                   -- HBR-PRIV account-bound resource scope (migrations 0363/0364).
                   WHEN 'owner_account_id' THEN attribute.atttypid = 'pg_catalog.uuid'::pg_catalog.regtype
                   WHEN 'actor_principal_id' THEN attribute.atttypid = 'pg_catalog.uuid'::pg_catalog.regtype
                   WHEN 'authenticated_session_id' THEN attribute.atttypid = 'pg_catalog.uuid'::pg_catalog.regtype
                   WHEN 'access_space_id' THEN attribute.atttypid = 'pg_catalog.uuid'::pg_catalog.regtype
                   WHEN 'workspace_id' THEN attribute.atttypid = 'pg_catalog.text'::pg_catalog.regtype
                   ELSE FALSE
               END AS type_matches,
               attribute.attnotnull AS not_null,
               CAST(attribute.attgenerated AS pg_catalog.text) AS generated_kind,
               CAST(attribute.attidentity AS pg_catalog.text) AS identity_kind
        FROM pg_catalog.pg_attribute AS attribute
        WHERE attribute.attrelid = $1::pg_catalog.oid
          AND attribute.attnum > 0
          AND NOT attribute.attisdropped
        ORDER BY attribute.attnum
        "#,
    )
    .bind(authority.relation_oid)
    .fetch_all(&mut **tx)
    .await?;
    let actual_columns = rows
        .into_iter()
        .map(|row| {
            Ok((
                row.try_get::<String, _>("column_name")?,
                (
                    row.try_get::<bool, _>("type_matches")?,
                    row.try_get::<bool, _>("not_null")?,
                    row.try_get::<String, _>("generated_kind")?,
                    row.try_get::<String, _>("identity_kind")?,
                ),
            ))
        })
        .collect::<Result<BTreeMap<_, _>, sqlx::Error>>()?;
    if actual_columns.len() != required_model_registry_columns().len() {
        return Err(ModelRegistryPersistenceError::AuthorityUnavailable(
            format!(
                "{schema}.model_runtime_registry has {} visible columns, expected exactly {}",
                actual_columns.len(),
                required_model_registry_columns().len()
            ),
        ));
    }
    for expected in required_model_registry_columns() {
        match actual_columns.get(expected.name) {
            Some((true, not_null, generated_kind, identity_kind))
                if *not_null == (expected.is_nullable == "NO")
                    && generated_kind.is_empty()
                    && identity_kind.is_empty() => {}
            Some((type_matches, not_null, generated_kind, identity_kind)) => {
                return Err(ModelRegistryPersistenceError::AuthorityUnavailable(format!(
                    "{schema}.model_runtime_registry.{} has built-in-type-match/nullability/generated/identity {type_matches}/{not_null}/{generated_kind:?}/{identity_kind:?}, expected pg_catalog.{}/{} with no generated or identity behavior",
                    expected.name, expected.udt_name, expected.is_nullable
                )));
            }
            None => {
                return Err(ModelRegistryPersistenceError::AuthorityUnavailable(
                    format!(
                        "{schema}.model_runtime_registry is missing required column {}",
                        expected.name
                    ),
                ));
            }
        }
    }

    let constraint_rows = sqlx::query(
        r#"
        SELECT constraint_row.conname,
               constraint_row.contype::text AS constraint_type,
               pg_catalog.pg_get_constraintdef(constraint_row.oid, false) AS constraint_definition,
               referenced_namespace.nspname AS referenced_schema,
               referenced_relation.relname AS referenced_table,
               referenced_relation.oid::pg_catalog.int8 AS referenced_oid
        FROM pg_catalog.pg_constraint AS constraint_row
        LEFT JOIN pg_catalog.pg_class AS referenced_relation
               ON referenced_relation.oid = constraint_row.confrelid
        LEFT JOIN pg_catalog.pg_namespace AS referenced_namespace
               ON referenced_namespace.oid = referenced_relation.relnamespace
        WHERE constraint_row.conrelid = $1::pg_catalog.oid
        "#,
    )
    .bind(authority.relation_oid)
    .fetch_all(&mut **tx)
    .await?;
    let constraints = constraint_rows
        .into_iter()
        .map(|row| {
            Ok((
                row.try_get::<String, _>("conname")?,
                (
                    row.try_get::<String, _>("constraint_type")?,
                    row.try_get::<String, _>("constraint_definition")?,
                    row.try_get::<Option<String>, _>("referenced_schema")?,
                    row.try_get::<Option<String>, _>("referenced_table")?,
                    row.try_get::<Option<i64>, _>("referenced_oid")?,
                ),
            ))
        })
        .collect::<Result<BTreeMap<_, _>, sqlx::Error>>()?;
    if constraints.len() != required_model_registry_constraints().len() {
        let unexpected = constraints
            .keys()
            .filter(|name| {
                !required_model_registry_constraints()
                    .iter()
                    .any(|expected| expected.name == name.as_str())
            })
            .cloned()
            .collect::<Vec<_>>();
        return Err(ModelRegistryPersistenceError::AuthorityUnavailable(format!(
            "{schema}.model_runtime_registry has {} constraints, expected exactly {}; unexpected constraints: {unexpected:?}",
            constraints.len(),
            required_model_registry_constraints().len()
        )));
    }
    for expected in required_model_registry_constraints() {
        let (actual_type, actual_definition, referenced_schema, referenced_table, referenced_oid) =
            constraints.get(expected.name).ok_or_else(|| {
                ModelRegistryPersistenceError::AuthorityUnavailable(format!(
                    "{schema}.model_runtime_registry is missing required constraint {}",
                    expected.name
                ))
            })?;
        let expected_referenced_table =
            (expected.constraint_type == "f").then_some("kernel_event_ledger");
        let reference_matches = match expected_referenced_table {
            Some(expected_table) => {
                referenced_schema.as_deref() == Some(schema)
                    && referenced_table.as_deref() == Some(expected_table)
                    && *referenced_oid == Some(authority.event_ledger_oid)
            }
            None => {
                referenced_schema.is_none()
                    && referenced_table.is_none()
                    && referenced_oid.is_none()
            }
        };
        if actual_type != expected.constraint_type
            || !expected.accepts_definition(actual_definition)
            || !reference_matches
        {
            return Err(ModelRegistryPersistenceError::AuthorityUnavailable(format!(
                "{schema}.model_runtime_registry constraint {} has semantic definition `{actual_definition}` (type {actual_type}, reference {referenced_schema:?}.{referenced_table:?}), expected {}",
                expected.name, expected.description
            )));
        }
    }

    let index_names = sqlx::query_scalar::<_, String>(
        r#"
        SELECT index_relation.relname
        FROM pg_catalog.pg_index AS index_state
        JOIN pg_catalog.pg_class AS index_relation
          ON index_relation.oid = index_state.indexrelid
        JOIN pg_catalog.pg_namespace AS index_namespace
          ON index_namespace.oid = index_relation.relnamespace
        WHERE index_state.indrelid = $1::pg_catalog.oid
          AND index_state.indislive
        ORDER BY index_relation.relname
        "#,
    )
    .bind(authority.relation_oid)
    .fetch_all(&mut **tx)
    .await?;
    // Exact-set pin, ordered by index name. The two HBR-PRIV scope indexes are
    // part of the canonical shape now: they are the ACCESS PATH for every
    // default-deny read (`WHERE owner_account_id = $n [AND workspace_id = $m]`)
    // and for actor attribution (HBR-PRIV-005), not reporting conveniences. If
    // migration 0363/0364 were reverted or skipped they would disappear, and
    // this check must fail loudly rather than let the registry silently fall
    // back to unindexed full-table scans of other accounts' rows.
    let expected_index_names = vec![
        "idx_model_runtime_registry_actor_principal".to_string(),
        "idx_model_runtime_registry_owner_scope".to_string(),
        MODEL_RUNTIME_REGISTRY_UPDATED_INDEX.to_string(),
        "pk_model_runtime_registry".to_string(),
        "uq_model_runtime_registry_artifact_sha256".to_string(),
    ];
    if index_names != expected_index_names {
        return Err(ModelRegistryPersistenceError::AuthorityUnavailable(format!(
            "{schema}.model_runtime_registry must have exactly its canonical primary-key, artifact-identity, updated-at, and HBR-PRIV scope indexes; found {index_names:?}"
        )));
    }

    for (index_name, expected_column, expected_primary) in [
        ("pk_model_runtime_registry", "registry_row_id", true),
        (
            "uq_model_runtime_registry_artifact_sha256",
            "artifact_sha256",
            false,
        ),
    ] {
        let index_matches: bool = sqlx::query_scalar(
            r#"
            SELECT pg_catalog.count(*) = 1
            FROM pg_catalog.pg_index AS index_state
            JOIN pg_catalog.pg_class AS index_relation
              ON index_relation.oid = index_state.indexrelid
            JOIN pg_catalog.pg_namespace AS index_namespace
              ON index_namespace.oid = index_relation.relnamespace
            JOIN pg_catalog.pg_am AS access_method
              ON access_method.oid = index_relation.relam
            JOIN pg_catalog.pg_attribute AS key_attribute
              ON key_attribute.attrelid = index_state.indrelid
             AND key_attribute.attnum = index_state.indkey[0]
            JOIN pg_catalog.pg_opclass AS key_opclass
              ON key_opclass.oid = index_state.indclass[0]
             AND key_opclass.opcmethod = index_relation.relam
             AND key_opclass.opcintype = key_attribute.atttypid
             AND key_opclass.opcdefault
            WHERE index_state.indrelid = $1::pg_catalog.oid
              AND index_relation.relname = $2
              AND index_namespace.nspname = $3
              AND index_relation.relkind = 'i'
              AND access_method.amname = 'btree'
              AND index_state.indisunique
              AND index_state.indisprimary = $4
              AND index_state.indisvalid
              AND index_state.indisready
              AND index_state.indislive
              AND NOT index_state.indisexclusion
              AND index_state.indimmediate
              AND index_state.indnkeyatts = 1
              AND index_state.indnatts = 1
              AND index_state.indpred IS NULL
              AND index_state.indexprs IS NULL
              AND index_state.indoption[0] = 0
              AND index_state.indcollation[0] = key_attribute.attcollation
              AND key_attribute.attname = $5
            "#,
        )
        .bind(authority.relation_oid)
        .bind(index_name)
        .bind(schema)
        .bind(expected_primary)
        .bind(expected_column)
        .fetch_one(&mut **tx)
        .await?;
        if !index_matches {
            return Err(ModelRegistryPersistenceError::AuthorityUnavailable(format!(
                "{schema}.model_runtime_registry index {index_name} is not the exact valid default-btree one-column {expected_column} authority"
            )));
        }
    }

    let index = sqlx::query(
        r#"
        SELECT index_relation.relkind::text AS relation_kind,
               COALESCE(index_state.indisvalid, false) AS indisvalid,
               COALESCE(index_state.indisready, false) AS indisready,
               COALESCE(index_state.indislive, false) AS indislive,
               COALESCE(index_state.indisunique, false) AS indisunique,
               COALESCE(index_state.indnkeyatts::integer, 0) AS indnkeyatts,
               COALESCE(index_state.indnatts::integer, 0) AS indnatts,
               index_state.indpred IS NULL AS has_no_predicate,
               index_state.indexprs IS NULL AS has_no_expressions,
               target_namespace.nspname AS target_schema,
               target_relation.relname AS target_table,
               target_relation.oid::pg_catalog.int8 AS target_oid,
               CASE
                   WHEN index_relation.relkind = 'i'
                   THEN pg_catalog.pg_get_indexdef(index_relation.oid)
                   ELSE '<not an index>'
               END AS index_definition
        FROM pg_catalog.pg_class AS index_relation
        JOIN pg_catalog.pg_namespace AS index_namespace
          ON index_namespace.oid = index_relation.relnamespace
        LEFT JOIN pg_catalog.pg_index AS index_state
          ON index_state.indexrelid = index_relation.oid
        LEFT JOIN pg_catalog.pg_class AS target_relation
          ON target_relation.oid = index_state.indrelid
        LEFT JOIN pg_catalog.pg_namespace AS target_namespace
          ON target_namespace.oid = target_relation.relnamespace
        WHERE index_state.indrelid = $1::pg_catalog.oid
          AND index_relation.relname = $2
          AND index_namespace.nspname = $3
        "#,
    )
    .bind(authority.relation_oid)
    .bind(MODEL_RUNTIME_REGISTRY_UPDATED_INDEX)
    .bind(schema)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| {
        ModelRegistryPersistenceError::AuthorityUnavailable(format!(
            "{schema}.model_runtime_registry is missing required index {MODEL_RUNTIME_REGISTRY_UPDATED_INDEX}"
        ))
    })?;
    let index_definition: String = index.try_get("index_definition")?;
    let normalized_index_definition = normalize_index_definition(&index_definition);
    let expected_index_prefix = format!("createindex{MODEL_RUNTIME_REGISTRY_UPDATED_INDEX}on");
    let expected_index_suffix =
        "model_runtime_registryusingbtree(selection_updated_at_utcdesc,registry_row_id)";
    let index_matches = index.try_get::<String, _>("relation_kind")? == "i"
        && index.try_get::<bool, _>("indisvalid")?
        && index.try_get::<bool, _>("indisready")?
        && index.try_get::<bool, _>("indislive")?
        && !index.try_get::<bool, _>("indisunique")?
        && index.try_get::<i32, _>("indnkeyatts")? == 2
        && index.try_get::<i32, _>("indnatts")? == 2
        && index.try_get::<bool, _>("has_no_predicate")?
        && index.try_get::<bool, _>("has_no_expressions")?
        && index
            .try_get::<Option<String>, _>("target_schema")?
            .as_deref()
            == Some(schema)
        && index
            .try_get::<Option<String>, _>("target_table")?
            .as_deref()
            == Some(MODEL_RUNTIME_REGISTRY_TABLE)
        && index.try_get::<Option<i64>, _>("target_oid")? == Some(authority.relation_oid)
        && normalized_index_definition.starts_with(&expected_index_prefix)
        && normalized_index_definition.ends_with(expected_index_suffix);
    if !index_matches {
        return Err(ModelRegistryPersistenceError::AuthorityUnavailable(format!(
            "{schema}.model_runtime_registry index {MODEL_RUNTIME_REGISTRY_UPDATED_INDEX} has semantic definition `{index_definition}`, expected non-unique valid btree (selection_updated_at_utc DESC, registry_row_id)"
        )));
    }

    assert_event_ledger_authority_shape_tx(tx, authority).await?;

    Ok(())
}

async fn assert_event_ledger_authority_shape_tx(
    tx: &mut AuthorityTransaction,
    authority: &ModelRegistryAuthority,
) -> Result<(), ModelRegistryPersistenceError> {
    let rows = sqlx::query(
        r#"
        SELECT attribute.attname AS column_name,
               CASE attribute.attname
                   WHEN 'event_sequence' THEN attribute.atttypid = 'pg_catalog.int8'::pg_catalog.regtype
                   WHEN 'payload' THEN attribute.atttypid = 'pg_catalog.jsonb'::pg_catalog.regtype
                   WHEN 'created_at' THEN attribute.atttypid = 'pg_catalog.timestamp'::pg_catalog.regtype
                   ELSE attribute.atttypid = 'pg_catalog.text'::pg_catalog.regtype
               END AS type_matches,
               attribute.attnotnull AS not_null,
               attribute.attgenerated::pg_catalog.text AS generated_kind,
               attribute.attidentity::pg_catalog.text AS identity_kind,
               pg_catalog.pg_get_expr(default_row.adbin, default_row.adrelid) AS default_definition
        FROM pg_catalog.pg_attribute AS attribute
        LEFT JOIN pg_catalog.pg_attrdef AS default_row
          ON default_row.adrelid = attribute.attrelid
         AND default_row.adnum = attribute.attnum
        WHERE attribute.attrelid = $1::pg_catalog.oid
          AND attribute.attnum > 0
          AND NOT attribute.attisdropped
        ORDER BY attribute.attnum
        "#,
    )
    .bind(authority.event_ledger_oid)
    .fetch_all(&mut **tx)
    .await?;
    let expected_columns = [
        ("event_id", true),
        ("event_sequence", true),
        ("event_version", true),
        ("kernel_task_run_id", true),
        ("session_run_id", true),
        ("aggregate_type", true),
        ("aggregate_id", true),
        ("idempotency_key", true),
        ("event_type", true),
        ("actor_kind", true),
        ("actor_id", true),
        ("causation_id", false),
        ("correlation_id", false),
        ("payload_hash", true),
        ("source_component", true),
        ("payload", true),
        ("created_at", true),
    ];
    if rows.len() != expected_columns.len() {
        return Err(ModelRegistryPersistenceError::AuthorityUnavailable(
            format!(
                "{}.kernel_event_ledger has {} visible columns, expected exactly {}",
                authority.schema,
                rows.len(),
                expected_columns.len()
            ),
        ));
    }
    for (row, (expected_name, expected_not_null)) in rows.iter().zip(expected_columns) {
        let actual_name: String = row.try_get("column_name")?;
        let type_matches: bool = row.try_get("type_matches")?;
        let not_null: bool = row.try_get("not_null")?;
        let generated_kind: String = row.try_get("generated_kind")?;
        let identity_kind: String = row.try_get("identity_kind")?;
        let default_definition: Option<String> = row.try_get("default_definition")?;
        if actual_name != expected_name
            || !type_matches
            || not_null != expected_not_null
            || !generated_kind.is_empty()
            || !identity_kind.is_empty()
        {
            return Err(ModelRegistryPersistenceError::AuthorityUnavailable(format!(
                "{}.kernel_event_ledger column {actual_name:?} has unexpected position/type/nullability/generated/identity behavior; expected {expected_name}",
                authority.schema
            )));
        }
        let normalized_default = default_definition
            .as_deref()
            .map(normalize_default_expression);
        let default_matches = match expected_name {
            "event_sequence" => {
                let unqualified =
                    "nextval('kernel_event_ledger_event_sequence_seq'::regclass)".to_string();
                let qualified = format!(
                    "nextval('{}.kernel_event_ledger_event_sequence_seq'::regclass)",
                    authority.schema.to_ascii_lowercase()
                );
                normalized_default
                    .as_ref()
                    .is_some_and(|actual| actual == &unqualified || actual == &qualified)
            }
            "created_at" => normalized_default
                .as_deref()
                .is_some_and(|actual| matches!(actual, "current_timestamp" | "now()")),
            _ => normalized_default.is_none(),
        };
        if !default_matches {
            return Err(ModelRegistryPersistenceError::AuthorityUnavailable(format!(
                "{}.kernel_event_ledger.{expected_name} has unexpected default {default_definition:?}",
                authority.schema
            )));
        }
    }

    let sequence_shape = sqlx::query(
        r#"
        SELECT sequence_namespace.nspname AS sequence_schema,
               sequence_relation.relname AS sequence_name,
               sequence_relation.oid::pg_catalog.int8 AS sequence_oid,
               sequence_relation.relkind::pg_catalog.text AS sequence_kind,
               sequence_relation.relpersistence::pg_catalog.text AS sequence_persistence,
               sequence_parameters.seqtypid = 'pg_catalog.int8'::pg_catalog.regtype
                   AS sequence_type_matches,
               sequence_parameters.seqstart,
               sequence_parameters.seqincrement,
               sequence_parameters.seqmax,
               sequence_parameters.seqmin,
               sequence_parameters.seqcache,
               sequence_parameters.seqcycle
        FROM pg_catalog.pg_class AS sequence_relation
        JOIN pg_catalog.pg_namespace AS sequence_namespace
          ON sequence_namespace.oid = sequence_relation.relnamespace
        JOIN pg_catalog.pg_sequence AS sequence_parameters
          ON sequence_parameters.seqrelid = sequence_relation.oid
        JOIN pg_catalog.pg_depend AS ownership
          ON ownership.classid = 'pg_catalog.pg_class'::pg_catalog.regclass
         AND ownership.objid = sequence_relation.oid
         AND ownership.refclassid = 'pg_catalog.pg_class'::pg_catalog.regclass
         AND ownership.refobjid = $1::pg_catalog.oid
         AND ownership.refobjsubid = (
             SELECT attribute.attnum
             FROM pg_catalog.pg_attribute AS attribute
             WHERE attribute.attrelid = $1::pg_catalog.oid
               AND attribute.attname = 'event_sequence'
               AND NOT attribute.attisdropped
         )
         AND ownership.deptype = 'a'
        "#,
    )
    .bind(authority.event_ledger_oid)
    .fetch_all(&mut **tx)
    .await?;
    if sequence_shape.len() != 1
        || sequence_shape[0].try_get::<String, _>("sequence_schema")? != authority.schema
        || sequence_shape[0].try_get::<String, _>("sequence_name")?
            != "kernel_event_ledger_event_sequence_seq"
        || sequence_shape[0].try_get::<String, _>("sequence_kind")? != "S"
        || sequence_shape[0].try_get::<String, _>("sequence_persistence")? != "p"
        || !sequence_shape[0].try_get::<bool, _>("sequence_type_matches")?
        || sequence_shape[0].try_get::<i64, _>("seqstart")? != 1
        || sequence_shape[0].try_get::<i64, _>("seqincrement")? != 1
        || sequence_shape[0].try_get::<i64, _>("seqmax")? != i64::MAX
        || sequence_shape[0].try_get::<i64, _>("seqmin")? != 1
        || sequence_shape[0].try_get::<i64, _>("seqcache")? != 1
        || sequence_shape[0].try_get::<bool, _>("seqcycle")?
    {
        return Err(ModelRegistryPersistenceError::AuthorityUnavailable(
            format!(
                "{}.kernel_event_ledger event_sequence must use its exact owned permanent bigint sequence with start/increment/min/cache 1, max bigint, and no cycle",
                authority.schema
            ),
        ));
    }

    let sequence_oid: i64 = sequence_shape[0].try_get("sequence_oid")?;
    let sequence_state_visible: bool = sqlx::query_scalar(
        "SELECT pg_catalog.has_sequence_privilege($1::pg_catalog.oid, 'SELECT')",
    )
    .bind(sequence_oid)
    .fetch_one(&mut **tx)
    .await?;
    if !sequence_state_visible {
        return Err(ModelRegistryPersistenceError::AuthorityUnavailable(format!(
            "{}.kernel_event_ledger event_sequence live state is not inspectable with SELECT privilege",
            authority.schema
        )));
    }
    let sequence_can_advance: bool = sqlx::query_scalar(
        "SELECT pg_catalog.has_sequence_privilege($1::pg_catalog.oid, 'USAGE') OR pg_catalog.has_sequence_privilege($1::pg_catalog.oid, 'UPDATE')",
    )
    .bind(sequence_oid)
    .fetch_one(&mut **tx)
    .await?;
    if !sequence_can_advance {
        return Err(ModelRegistryPersistenceError::AuthorityUnavailable(format!(
            "{}.kernel_event_ledger event_sequence cannot advance because the authority role lacks USAGE or UPDATE privilege",
            authority.schema
        )));
    }
    let sequence_state_statement = format!(
        "SELECT last_value::pg_catalog.int8 AS last_value, is_called FROM {}.{}",
        quote_pg_identifier(&authority.schema),
        quote_pg_identifier("kernel_event_ledger_event_sequence_seq")
    );
    let sequence_state = sqlx::query(&sequence_state_statement)
        .fetch_one(&mut **tx)
        .await?;
    let sequence_last_value: i64 = sequence_state.try_get("last_value")?;
    let sequence_is_called: bool = sequence_state.try_get("is_called")?;
    if sequence_last_value == i64::MAX && sequence_is_called {
        return Err(ModelRegistryPersistenceError::AuthorityUnavailable(
            format!(
            "{}.kernel_event_ledger event_sequence is exhausted at bigint max with is_called=true",
            authority.schema
        ),
        ));
    }

    let constraint_rows = sqlx::query(
        r#"
        SELECT constraint_row.conname,
               constraint_row.contype::pg_catalog.text AS constraint_type,
               pg_catalog.pg_get_constraintdef(constraint_row.oid, false) AS constraint_definition
        FROM pg_catalog.pg_constraint AS constraint_row
        WHERE constraint_row.conrelid = $1::pg_catalog.oid
        ORDER BY constraint_row.conname
        "#,
    )
    .bind(authority.event_ledger_oid)
    .fetch_all(&mut **tx)
    .await?;
    if constraint_rows.len() != 1
        || constraint_rows[0].try_get::<String, _>("conname")? != "kernel_event_ledger_pkey"
        || constraint_rows[0].try_get::<String, _>("constraint_type")? != "p"
        || normalize_constraint_definition(
            &constraint_rows[0].try_get::<String, _>("constraint_definition")?,
        ) != normalize_constraint_definition("PRIMARY KEY (event_id)")
    {
        return Err(ModelRegistryPersistenceError::AuthorityUnavailable(format!(
            "{}.kernel_event_ledger must have exactly its canonical event_id primary key and no behavior-bearing extra constraints",
            authority.schema
        )));
    }

    let unsafe_index_dependency_count: i64 = sqlx::query_scalar(
        r#"
        SELECT pg_catalog.count(DISTINCT index_state.indexrelid)::pg_catalog.int8
        FROM pg_catalog.pg_index AS index_state
        JOIN pg_catalog.pg_depend AS dependency
          ON dependency.classid = 'pg_catalog.pg_class'::pg_catalog.regclass
         AND dependency.objid = index_state.indexrelid
        LEFT JOIN pg_catalog.pg_proc AS function_row
          ON dependency.refclassid = 'pg_catalog.pg_proc'::pg_catalog.regclass
         AND function_row.oid = dependency.refobjid
        LEFT JOIN pg_catalog.pg_namespace AS function_namespace
          ON function_namespace.oid = function_row.pronamespace
        LEFT JOIN pg_catalog.pg_operator AS operator_row
          ON dependency.refclassid = 'pg_catalog.pg_operator'::pg_catalog.regclass
         AND operator_row.oid = dependency.refobjid
        LEFT JOIN pg_catalog.pg_namespace AS operator_namespace
          ON operator_namespace.oid = operator_row.oprnamespace
        WHERE index_state.indrelid = $1::pg_catalog.oid
          AND (
              (function_row.oid IS NOT NULL AND function_namespace.nspname <> 'pg_catalog')
              OR (operator_row.oid IS NOT NULL AND operator_namespace.nspname <> 'pg_catalog')
          )
        "#,
    )
    .bind(authority.event_ledger_oid)
    .fetch_one(&mut **tx)
    .await?;
    if unsafe_index_dependency_count != 0 {
        return Err(ModelRegistryPersistenceError::AuthorityUnavailable(format!(
            "{}.kernel_event_ledger has an index expression or predicate that depends on non-pg_catalog executable behavior",
            authority.schema
        )));
    }

    for (index_name, expected_column) in [
        ("kernel_event_ledger_pkey", "event_id"),
        ("idx_kernel_event_ledger_sequence", "event_sequence"),
        ("idx_kernel_event_ledger_idempotency", "idempotency_key"),
    ] {
        let index_matches: bool = sqlx::query_scalar(
            r#"
            SELECT pg_catalog.count(*) = 1
            FROM pg_catalog.pg_index AS index_state
            JOIN pg_catalog.pg_class AS index_relation
              ON index_relation.oid = index_state.indexrelid
            JOIN pg_catalog.pg_am AS access_method
              ON access_method.oid = index_relation.relam
            JOIN pg_catalog.pg_attribute AS key_attribute
              ON key_attribute.attrelid = index_state.indrelid
             AND key_attribute.attnum = index_state.indkey[0]
            JOIN pg_catalog.pg_opclass AS key_opclass
              ON key_opclass.oid = index_state.indclass[0]
             AND key_opclass.opcmethod = index_relation.relam
             AND key_opclass.opcintype = key_attribute.atttypid
             AND key_opclass.opcdefault
            WHERE index_state.indrelid = $1::pg_catalog.oid
              AND index_relation.relname = $2
              AND index_relation.relkind = 'i'
              AND access_method.amname = 'btree'
              AND index_state.indisunique
              AND index_state.indisvalid
              AND index_state.indisready
              AND index_state.indnkeyatts = 1
              AND index_state.indnatts = 1
              AND index_state.indpred IS NULL
              AND index_state.indexprs IS NULL
              AND index_state.indoption[0] = 0
              AND index_state.indcollation[0] = key_attribute.attcollation
              AND key_attribute.attname = $3
            "#,
        )
        .bind(authority.event_ledger_oid)
        .bind(index_name)
        .bind(expected_column)
        .fetch_one(&mut **tx)
        .await?;
        if !index_matches {
            return Err(ModelRegistryPersistenceError::AuthorityUnavailable(format!(
                "{}.kernel_event_ledger index {index_name} is not the exact valid one-column unique {expected_column} authority",
                authority.schema
            )));
        }
    }

    let aggregate_replay_index_matches: bool = sqlx::query_scalar(
        r#"
        SELECT pg_catalog.count(*) = 1
        FROM pg_catalog.pg_index AS index_state
        JOIN pg_catalog.pg_class AS index_relation
          ON index_relation.oid = index_state.indexrelid
        JOIN pg_catalog.pg_am AS access_method
          ON access_method.oid = index_relation.relam
        JOIN pg_catalog.pg_attribute AS aggregate_type_attribute
          ON aggregate_type_attribute.attrelid = index_state.indrelid
         AND aggregate_type_attribute.attnum = index_state.indkey[0]
        JOIN pg_catalog.pg_attribute AS aggregate_id_attribute
          ON aggregate_id_attribute.attrelid = index_state.indrelid
         AND aggregate_id_attribute.attnum = index_state.indkey[1]
        JOIN pg_catalog.pg_attribute AS event_sequence_attribute
          ON event_sequence_attribute.attrelid = index_state.indrelid
         AND event_sequence_attribute.attnum = index_state.indkey[2]
        JOIN pg_catalog.pg_opclass AS aggregate_type_opclass
          ON aggregate_type_opclass.oid = index_state.indclass[0]
         AND aggregate_type_opclass.opcmethod = index_relation.relam
         AND aggregate_type_opclass.opcintype = aggregate_type_attribute.atttypid
         AND aggregate_type_opclass.opcdefault
        JOIN pg_catalog.pg_opclass AS aggregate_id_opclass
          ON aggregate_id_opclass.oid = index_state.indclass[1]
         AND aggregate_id_opclass.opcmethod = index_relation.relam
         AND aggregate_id_opclass.opcintype = aggregate_id_attribute.atttypid
         AND aggregate_id_opclass.opcdefault
        JOIN pg_catalog.pg_opclass AS event_sequence_opclass
          ON event_sequence_opclass.oid = index_state.indclass[2]
         AND event_sequence_opclass.opcmethod = index_relation.relam
         AND event_sequence_opclass.opcintype = event_sequence_attribute.atttypid
         AND event_sequence_opclass.opcdefault
        WHERE index_state.indrelid = $1::pg_catalog.oid
          AND index_relation.relname = 'idx_kernel_event_ledger_aggregate_replay'
          AND index_relation.relkind = 'i'
          AND access_method.amname = 'btree'
          AND NOT index_state.indisunique
          AND index_state.indisvalid
          AND index_state.indisready
          AND index_state.indnkeyatts = 3
          AND index_state.indnatts = 3
          AND index_state.indpred IS NULL
          AND index_state.indexprs IS NULL
          AND index_state.indoption[0] = 0
          AND index_state.indoption[1] = 0
          AND index_state.indoption[2] = 0
          AND index_state.indcollation[0] = aggregate_type_attribute.attcollation
          AND index_state.indcollation[1] = aggregate_id_attribute.attcollation
          AND index_state.indcollation[2] = event_sequence_attribute.attcollation
          AND aggregate_type_attribute.attname = 'aggregate_type'
          AND aggregate_id_attribute.attname = 'aggregate_id'
          AND event_sequence_attribute.attname = 'event_sequence'
        "#,
    )
    .bind(authority.event_ledger_oid)
    .fetch_one(&mut **tx)
    .await?;
    if !aggregate_replay_index_matches {
        return Err(ModelRegistryPersistenceError::AuthorityUnavailable(format!(
            "{}.kernel_event_ledger index idx_kernel_event_ledger_aggregate_replay is not the exact valid btree (aggregate_type, aggregate_id, event_sequence) replay authority",
            authority.schema
        )));
    }

    Ok(())
}

fn normalize_default_expression(definition: &str) -> String {
    definition
        .to_ascii_lowercase()
        .chars()
        .filter(|character| !character.is_whitespace() && *character != '"')
        .collect()
}

async fn lock_model_registry_selection_tx(
    tx: &mut AuthorityTransaction,
    artifact_sha256: &[u8; 32],
) -> Result<(), ModelRegistryPersistenceError> {
    let key = format!(
        "handshake.model_runtime_registry.selection.v1:{}",
        hex::encode(artifact_sha256)
    );
    let deadline = Instant::now() + MODEL_REGISTRY_ADVISORY_LOCK_TIMEOUT;
    loop {
        let acquired: bool = sqlx::query_scalar(
            "SELECT pg_catalog.pg_try_advisory_xact_lock(('x' || pg_catalog.substr(pg_catalog.md5($1), 1, 16))::bit(64)::bigint)",
        )
        .bind(&key)
        .fetch_one(&mut **tx)
        .await?;
        if acquired {
            return Ok(());
        }
        let now = Instant::now();
        if now >= deadline {
            return Err(ModelRegistryPersistenceError::SelectionLockTimeout {
                artifact_sha256: hex::encode(artifact_sha256),
                timeout_ms: u64::try_from(MODEL_REGISTRY_ADVISORY_LOCK_TIMEOUT.as_millis())
                    .unwrap_or(u64::MAX),
            });
        }
        tokio::time::sleep(
            MODEL_REGISTRY_ADVISORY_LOCK_RETRY_INTERVAL
                .min(deadline.saturating_duration_since(now)),
        )
        .await;
    }
}

async fn load_by_artifact_sha256_tx(
    tx: &mut AuthorityTransaction,
    artifact_sha256: &[u8; 32],
    for_update: bool,
) -> Result<Option<PersistedModelRegistration>, ModelRegistryPersistenceError> {
    let row_lock = if for_update { " FOR UPDATE" } else { "" };
    let transfer_statement = format!(
        r#"
        WITH locked AS MATERIALIZED (
            SELECT *
            FROM ONLY model_runtime_registry
            WHERE artifact_sha256 = $1
            {row_lock}
        ),
        sized AS MATERIALIZED (
            SELECT locked.*,
                   pg_catalog.octet_length(capabilities_json::pg_catalog.text)::pg_catalog.int8
                       AS capabilities_bytes,
                   (
                       pg_catalog.octet_length(schema_id)::pg_catalog.int8
                       + pg_catalog.octet_length(artifact_sha256)::pg_catalog.int8
                       + pg_catalog.octet_length(artifact_locator)::pg_catalog.int8
                       + pg_catalog.octet_length(runtime_binding)::pg_catalog.int8
                       + pg_catalog.octet_length(runtime_role)::pg_catalog.int8
                       + pg_catalog.octet_length(capabilities_schema_id)::pg_catalog.int8
                       + pg_catalog.octet_length(capabilities_json::pg_catalog.text)::pg_catalog.int8
                       + pg_catalog.octet_length(provider)::pg_catalog.int8
                       + pg_catalog.octet_length(base_model_tag)::pg_catalog.int8
                       + pg_catalog.octet_length(last_observed_by)::pg_catalog.int8
                       + pg_catalog.octet_length(selection_created_event_id)::pg_catalog.int8
                       + pg_catalog.octet_length(selection_updated_event_id)::pg_catalog.int8
                       + 128
                   )::pg_catalog.int8 AS row_bytes
            FROM locked
        ),
        bounded AS (
            SELECT sized.*,
                   capabilities_bytes <= $2 AND row_bytes <= $3 AS within_budget
            FROM sized
        )
        SELECT capabilities_bytes,
               row_bytes,
               CASE WHEN within_budget THEN schema_id END AS schema_id,
               CASE WHEN within_budget THEN registry_row_id END AS registry_row_id,
               CASE WHEN within_budget THEN artifact_sha256 END AS artifact_sha256,
               CASE WHEN within_budget THEN artifact_locator END AS artifact_locator,
               CASE WHEN within_budget THEN last_observed_runtime_model_id END
                   AS last_observed_runtime_model_id,
               CASE WHEN within_budget THEN runtime_binding END AS runtime_binding,
               CASE WHEN within_budget THEN runtime_role END AS runtime_role,
               CASE WHEN within_budget THEN capabilities_schema_id END AS capabilities_schema_id,
               CASE WHEN within_budget THEN capabilities_json END AS capabilities_json,
               CASE WHEN within_budget THEN provider END AS provider,
               CASE WHEN within_budget THEN base_model_tag END AS base_model_tag,
               CASE WHEN within_budget THEN last_observed_by END AS last_observed_by,
               CASE WHEN within_budget THEN selection_revision END AS selection_revision,
               CASE WHEN within_budget THEN selection_created_event_id END
                   AS selection_created_event_id,
               CASE WHEN within_budget THEN selection_updated_event_id END
                   AS selection_updated_event_id,
               CASE WHEN within_budget THEN selection_created_at_utc END
                   AS selection_created_at_utc,
               CASE WHEN within_budget THEN selection_updated_at_utc END
                   AS selection_updated_at_utc,
               CASE WHEN within_budget THEN last_observed_at_utc END AS last_observed_at_utc
        FROM bounded
        "#
    );
    let row = sqlx::query(&transfer_statement)
        .bind(artifact_sha256.as_slice())
        .bind(MODEL_REGISTRY_CAPABILITIES_JSON_BYTE_CAP)
        .bind(MODEL_REGISTRY_ROW_BYTE_CAP)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|error| map_sqlx_selection_error(error, artifact_sha256))?;
    if let Some(row) = &row {
        enforce_registry_row_transfer_budget(
            row.try_get("capabilities_bytes")?,
            row.try_get("row_bytes")?,
        )?;
    }
    let decoded = row.map(decode_row).transpose()?;
    if let Some(registration) = &decoded {
        validate_selection_audit_chain_tx(tx, registration, for_update).await?;
    }
    Ok(decoded)
}

fn enforce_registry_row_transfer_budget(
    capabilities_bytes: i64,
    row_bytes: i64,
) -> Result<(), ModelRegistryPersistenceError> {
    if capabilities_bytes > MODEL_REGISTRY_CAPABILITIES_JSON_BYTE_CAP {
        return Err(ModelRegistryPersistenceError::CorruptRow(format!(
            "capabilities_json is {capabilities_bytes} bytes, exceeding the bounded {MODEL_REGISTRY_CAPABILITIES_JSON_BYTE_CAP}-byte decode limit"
        )));
    }
    if row_bytes > MODEL_REGISTRY_ROW_BYTE_CAP {
        return Err(ModelRegistryPersistenceError::CorruptRow(format!(
            "model registry row is {row_bytes} variable bytes, exceeding the bounded {MODEL_REGISTRY_ROW_BYTE_CAP}-byte transfer limit"
        )));
    }
    Ok(())
}

fn map_sqlx_selection_error(
    error: sqlx::Error,
    artifact_sha256: &[u8; 32],
) -> ModelRegistryPersistenceError {
    if matches!(
        &error,
        sqlx::Error::Database(database_error)
            if database_error.code().as_deref() == Some("55P03")
    ) {
        return selection_lock_timeout_error(artifact_sha256);
    }
    ModelRegistryPersistenceError::from(error)
}

fn selection_lock_timeout_error(artifact_sha256: &[u8; 32]) -> ModelRegistryPersistenceError {
    ModelRegistryPersistenceError::SelectionLockTimeout {
        artifact_sha256: hex::encode(artifact_sha256),
        timeout_ms: u64::try_from(MODEL_REGISTRY_DATABASE_LOCK_TIMEOUT.as_millis())
            .unwrap_or(u64::MAX),
    }
}

async fn validate_selection_audit_chain_tx(
    tx: &mut AuthorityTransaction,
    registration: &PersistedModelRegistration,
    lock_rows: bool,
) -> Result<(), ModelRegistryPersistenceError> {
    let aggregate_ids = [registration.artifact_locator.clone()];
    let events = load_selection_events_for_aggregates_tx(
        tx,
        &aggregate_ids,
        MODEL_REGISTRY_AUDIT_EVENT_CAP + 1,
        lock_rows,
    )
    .await?;
    validate_selection_audit_chain(registration, &events)
}

fn validate_selection_audit_chain(
    registration: &PersistedModelRegistration,
    events: &[PersistedSelectionEvent],
) -> Result<(), ModelRegistryPersistenceError> {
    if registration.selection_revision
        > u64::try_from(MODEL_REGISTRY_AUDIT_EVENT_CAP).unwrap_or(u64::MAX)
    {
        return Err(audit_chain_corrupt(
            registration,
            format!(
                "selection revision {} exceeds the bounded {}-event audit limit",
                registration.selection_revision, MODEL_REGISTRY_AUDIT_EVENT_CAP
            ),
        ));
    }
    let latest = events
        .last()
        .ok_or_else(|| audit_chain_corrupt(registration, "selection audit chain is absent"))?;
    let latest_event_id = latest.event_id.as_str();
    let chain_count = i64::try_from(events.len()).unwrap_or(i64::MAX);
    let latest_created_at_utc = latest.created_at_utc;
    if chain_count > MODEL_REGISTRY_AUDIT_EVENT_CAP {
        return Err(audit_chain_corrupt(
            registration,
            format!(
                "selection audit chain has {chain_count} events, exceeding the bounded {}-event limit",
                MODEL_REGISTRY_AUDIT_EVENT_CAP
            ),
        ));
    }
    if u64::try_from(chain_count).ok() != Some(registration.selection_revision)
        || latest_event_id != registration.selection_updated_event_id.as_str()
    {
        return Err(audit_chain_corrupt(
            registration,
            format!(
                "projection revision/ref is not the latest EventLedger authority: row revision {} ref {}, audit count {chain_count} latest {latest_event_id}",
                registration.selection_revision, registration.selection_updated_event_id
            ),
        ));
    }
    if latest_created_at_utc != registration.selection_updated_at_utc {
        return Err(audit_chain_corrupt(
            registration,
            format!(
                "projection updated timestamp {} differs from latest EventLedger timestamp {}",
                registration.selection_updated_at_utc, latest_created_at_utc
            ),
        ));
    }
    if registration.selection_revision == 1
        && registration.selection_created_event_id != registration.selection_updated_event_id
    {
        return Err(audit_chain_corrupt(
            registration,
            "revision one must use the same created and updated EventLedger ref",
        ));
    }
    if registration.selection_revision > 1
        && registration.selection_created_event_id == registration.selection_updated_event_id
    {
        return Err(audit_chain_corrupt(
            registration,
            format!(
                "revision {} rebound selection must not reuse the initial EventLedger ref as its updated ref",
                registration.selection_revision
            ),
        ));
    }

    let mut expected_revision = registration.selection_revision;
    let mut expected_selection = registration.selection();
    let mut current_event_id = registration.selection_updated_event_id.clone();
    let mut visited = BTreeSet::new();
    let mut newer_event_sequence = None;
    let mut newer_created_at = None;
    let events_by_id = events
        .iter()
        .map(|event| (event.event_id.as_str(), event))
        .collect::<BTreeMap<_, _>>();
    if events_by_id.len() != events.len() {
        return Err(audit_chain_corrupt(
            registration,
            "selection audit query returned duplicate event ids",
        ));
    }

    loop {
        if i64::try_from(visited.len()).unwrap_or(i64::MAX) >= MODEL_REGISTRY_AUDIT_EVENT_CAP {
            return Err(audit_chain_corrupt(
                registration,
                format!(
                    "selection audit traversal exceeds the bounded {}-event limit",
                    MODEL_REGISTRY_AUDIT_EVENT_CAP
                ),
            ));
        }
        if !visited.insert(current_event_id.clone()) {
            return Err(audit_chain_corrupt(
                registration,
                format!("selection audit causation chain contains a cycle at {current_event_id}"),
            ));
        }
        let event = events_by_id
            .get(current_event_id.as_str())
            .copied()
            .ok_or_else(|| {
                audit_chain_corrupt(
                    registration,
                    format!(
                        "selection audit event {current_event_id} is absent from the artifact-scoped audit set"
                    ),
                )
            })?;
        if newer_event_sequence.is_some_and(|newer| event.event_sequence >= newer) {
            return Err(audit_chain_corrupt(
                registration,
                format!(
                    "selection audit event {} is not earlier than its causal successor",
                    event.event_id
                ),
            ));
        }
        if newer_created_at.is_some_and(|newer| event.created_at_utc > newer) {
            return Err(audit_chain_corrupt(
                registration,
                format!(
                    "selection audit event {} timestamp is later than its causal successor",
                    event.event_id
                ),
            ));
        }
        newer_event_sequence = Some(event.event_sequence);
        newer_created_at = Some(event.created_at_utc);
        validate_selection_event_common(registration, event, expected_revision)?;

        if expected_revision == 1 {
            if event.event_id != registration.selection_created_event_id {
                return Err(audit_chain_corrupt(
                    registration,
                    format!(
                        "selection audit chain ended at {}, expected created ref {}",
                        event.event_id, registration.selection_created_event_id
                    ),
                ));
            }
            if event.created_at_utc != registration.selection_created_at_utc {
                return Err(audit_chain_corrupt(
                    registration,
                    format!(
                        "projection created timestamp {} differs from initial EventLedger timestamp {}",
                        registration.selection_created_at_utc, event.created_at_utc
                    ),
                ));
            }
            let initial_selection = validate_initial_selection_event(registration, event)?;
            if initial_selection != expected_selection {
                return Err(audit_chain_corrupt(
                    registration,
                    "initial selection payload does not continue the audited selection chain",
                ));
            }
            if visited.len() != events.len() {
                return Err(audit_chain_corrupt(
                    registration,
                    "selection audit contains events outside the updated-to-created causation chain",
                ));
            }
            return Ok(());
        }

        let (previous_selection, target_selection) =
            validate_rebind_selection_event(registration, event, expected_revision)?;
        if target_selection != expected_selection {
            return Err(audit_chain_corrupt(
                registration,
                format!(
                    "revision {expected_revision} target selection does not match the newer audited selection"
                ),
            ));
        }
        let previous_event_id = event.causation_id.clone().ok_or_else(|| {
            audit_chain_corrupt(
                registration,
                format!("revision {expected_revision} rebind event has no causation ref"),
            )
        })?;
        expected_selection = previous_selection;
        expected_revision -= 1;
        current_event_id = previous_event_id;
    }
}

async fn recover_missing_registry_audit_tx(
    tx: &mut AuthorityTransaction,
    attempted: &ModelRuntimeSelection,
    require_capabilities_match: bool,
    lock_rows: bool,
) -> Result<Option<RecoveredSelectionAudit>, ModelRegistryPersistenceError> {
    let aggregate_ids = [artifact_locator_for_sha256(attempted.artifact_sha256)];
    let events = load_selection_events_for_aggregates_tx(
        tx,
        &aggregate_ids,
        MODEL_REGISTRY_AUDIT_EVENT_CAP + 1,
        lock_rows,
    )
    .await?;
    if events.is_empty() {
        return Ok(None);
    }
    if i64::try_from(events.len()).unwrap_or(i64::MAX) > MODEL_REGISTRY_AUDIT_EVENT_CAP {
        return Err(ModelRegistryPersistenceError::Audit(format!(
            "model registry audit recovery exceeds the bounded {}-event limit",
            MODEL_REGISTRY_AUDIT_EVENT_CAP
        )));
    }
    let row_identity_event = events
        .get(if events.len() > 1 { 1 } else { 0 })
        .expect("nonempty audit chain");
    let registry_row_id = row_identity_event
        .correlation_id
        .as_deref()
        .and_then(|value| Uuid::parse_str(value).ok())
        .ok_or_else(|| {
            ModelRegistryPersistenceError::Audit(
                "model registry audit event has no UUID row correlation".to_string(),
            )
        })?;
    let selection_revision = u64::try_from(events.len()).map_err(|_| {
        ModelRegistryPersistenceError::Audit(
            "model registry audit chain length exceeds u64".to_string(),
        )
    })?;
    let first = events.first().expect("nonempty audit chain");
    let initial_payload = first.payload.as_object().ok_or_else(|| {
        ModelRegistryPersistenceError::Audit(
            "initial model registry audit payload is not an object".to_string(),
        )
    })?;
    let observed_model_id = initial_payload
        .get("observed_runtime_model_id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            ModelRegistryPersistenceError::Audit(
                "initial model registry audit has no observed runtime model id".to_string(),
            )
        })?;
    let audited_model_id = Uuid::parse_str(observed_model_id)
        .map(ModelId::from)
        .map_err(|_| {
            ModelRegistryPersistenceError::Audit(
                "initial model registry audit observed runtime model id is not a UUID".to_string(),
            )
        })?;
    let audited_base_model_tag = BaseModelTag::try_new(
        initial_payload
            .get("base_model_tag")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                ModelRegistryPersistenceError::Audit(
                    "initial model registry audit has no base model tag".to_string(),
                )
            })?,
    )
    .map_err(|error| ModelRegistryPersistenceError::Audit(error.to_string()))?;
    let audited_observer = OperatorId::try_new(
        initial_payload
            .get("observed_by")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                ModelRegistryPersistenceError::Audit(
                    "initial model registry audit has no observer".to_string(),
                )
            })?,
    )
    .map_err(|error| ModelRegistryPersistenceError::Audit(error.to_string()))?;
    let selection_created_at_utc = first.created_at_utc;
    let selection_updated_at_utc = events.last().expect("nonempty audit chain").created_at_utc;
    let synthetic = PersistedModelRegistration {
        schema_id: MODEL_RUNTIME_REGISTRY_SCHEMA_ID.to_string(),
        registry_row_id,
        artifact_sha256: attempted.artifact_sha256,
        artifact_locator: artifact_locator_for_sha256(attempted.artifact_sha256),
        // Observation fields are recovered only into this transient validator
        // from the actual initial audit payload. A successful real load writes
        // the current observation into the recreated projection later.
        last_observed_runtime_model_id: audited_model_id,
        runtime_binding: attempted.runtime_binding,
        runtime_role: attempted.runtime_role,
        capabilities_schema_id: MODEL_RUNTIME_CAPABILITIES_SCHEMA_ID.to_string(),
        declared_capabilities: attempted.declared_capabilities.clone(),
        provider: attempted.provider,
        base_model_tag: audited_base_model_tag,
        last_observed_by: audited_observer,
        selection_revision,
        selection_created_event_id: events
            .first()
            .expect("nonempty audit chain")
            .event_id
            .clone(),
        selection_updated_event_id: events
            .last()
            .expect("nonempty audit chain")
            .event_id
            .clone(),
        selection_created_at_utc,
        selection_updated_at_utc,
        last_observed_at_utc: selection_created_at_utc,
    };

    validate_selection_event_common(&synthetic, first, 1)?;
    let mut audited_selection = validate_initial_selection_event(&synthetic, first)?;
    let mut previous_event_id = first.event_id.as_str();
    let mut previous_sequence = first.event_sequence;
    let mut previous_created_at = first.created_at_utc;
    for (index, event) in events.iter().enumerate().skip(1) {
        let revision = u64::try_from(index + 1).map_err(|_| {
            ModelRegistryPersistenceError::Audit(
                "model registry audit revision exceeds u64".to_string(),
            )
        })?;
        if event.event_sequence <= previous_sequence
            || event.causation_id.as_deref() != Some(previous_event_id)
            || event.created_at_utc < previous_created_at
        {
            return Err(audit_chain_corrupt(
                &synthetic,
                format!("revision {revision} audit ordering/causation is not contiguous"),
            ));
        }
        validate_selection_event_common(&synthetic, event, revision)?;
        let (previous, target) = validate_rebind_selection_event(&synthetic, event, revision)?;
        if previous != audited_selection {
            return Err(audit_chain_corrupt(
                &synthetic,
                format!("revision {revision} previous selection does not match audit history"),
            ));
        }
        audited_selection = target;
        previous_event_id = event.event_id.as_str();
        previous_sequence = event.event_sequence;
        previous_created_at = event.created_at_utc;
    }
    if audited_selection.runtime_binding != attempted.runtime_binding
        || audited_selection.provider != attempted.provider
        || (require_capabilities_match
            && audited_selection.declared_capabilities != attempted.declared_capabilities)
    {
        return Err(ModelRegistryPersistenceError::SelectionConflict(format!(
            "artifact {} has an audit-preserved immutable selection that differs from the configured adapter",
            hex::encode(attempted.artifact_sha256)
        )));
    }

    Ok(Some(RecoveredSelectionAudit {
        registry_row_id,
        selection_revision,
        selection_created_event_id: synthetic.selection_created_event_id,
        selection_updated_event_id: synthetic.selection_updated_event_id,
        selection_created_at_utc,
        selection_updated_at_utc,
    }))
}

async fn load_selection_events_for_aggregates_tx(
    tx: &mut AuthorityTransaction,
    aggregate_ids: &[String],
    limit: i64,
    lock_rows: bool,
) -> Result<Vec<PersistedSelectionEvent>, ModelRegistryPersistenceError> {
    if aggregate_ids.is_empty() {
        return Ok(Vec::new());
    }
    let lock_clause = if lock_rows { " FOR SHARE" } else { "" };
    let transfer_statement = format!(
        r#"
        SELECT event_sequence AS bounded_event_sequence,
               pg_catalog.octet_length(payload::pg_catalog.text)::pg_catalog.int8
                   AS payload_bytes,
               (
                   pg_catalog.octet_length(event_id)::pg_catalog.int8
                   + pg_catalog.octet_length(event_version)::pg_catalog.int8
                   + pg_catalog.octet_length(kernel_task_run_id)::pg_catalog.int8
                   + pg_catalog.octet_length(session_run_id)::pg_catalog.int8
                   + pg_catalog.octet_length(aggregate_type)::pg_catalog.int8
                   + pg_catalog.octet_length(aggregate_id)::pg_catalog.int8
                   + pg_catalog.octet_length(idempotency_key)::pg_catalog.int8
                   + pg_catalog.octet_length(event_type)::pg_catalog.int8
                   + pg_catalog.octet_length(actor_kind)::pg_catalog.int8
                   + pg_catalog.octet_length(actor_id)::pg_catalog.int8
                   + COALESCE(pg_catalog.octet_length(causation_id), 0)::pg_catalog.int8
                   + COALESCE(pg_catalog.octet_length(correlation_id), 0)::pg_catalog.int8
                   + pg_catalog.octet_length(payload_hash)::pg_catalog.int8
                   + pg_catalog.octet_length(source_component)::pg_catalog.int8
                   + pg_catalog.octet_length(payload::pg_catalog.text)::pg_catalog.int8
                   + 32
               )::pg_catalog.int8 AS event_bytes
        FROM ONLY kernel_event_ledger
        WHERE aggregate_type = 'model_runtime_registry'
          AND aggregate_id = ANY($1)
          AND source_component = 'model_runtime_registry'
        ORDER BY aggregate_id ASC, event_sequence ASC
        LIMIT $2{lock_clause}
        "#
    );
    let transfer_rows = sqlx::query(&transfer_statement)
        .bind(aggregate_ids)
        .bind(limit)
        .fetch_all(&mut **tx)
        .await?;
    let mut max_payload_bytes = 0_i64;
    let mut max_event_bytes = 0_i64;
    let mut total_event_bytes = 0_i64;
    let mut bounded_event_sequences = Vec::with_capacity(transfer_rows.len());
    for transfer_row in transfer_rows {
        bounded_event_sequences.push(transfer_row.try_get::<i64, _>("bounded_event_sequence")?);
        let payload_bytes: i64 = transfer_row.try_get("payload_bytes")?;
        let event_bytes: i64 = transfer_row.try_get("event_bytes")?;
        max_payload_bytes = max_payload_bytes.max(payload_bytes);
        max_event_bytes = max_event_bytes.max(event_bytes);
        total_event_bytes = total_event_bytes.checked_add(event_bytes).ok_or_else(|| {
            ModelRegistryPersistenceError::Audit(
                "model registry EventLedger transfer byte total overflowed".to_string(),
            )
        })?;
    }
    if max_payload_bytes > MODEL_REGISTRY_AUDIT_PAYLOAD_BYTE_CAP {
        return Err(ModelRegistryPersistenceError::Audit(format!(
            "model registry EventLedger payload is {max_payload_bytes} bytes, exceeding the bounded {MODEL_REGISTRY_AUDIT_PAYLOAD_BYTE_CAP}-byte decode limit"
        )));
    }
    if max_event_bytes > MODEL_REGISTRY_AUDIT_EVENT_BYTE_CAP {
        return Err(ModelRegistryPersistenceError::Audit(format!(
            "model registry EventLedger row is {max_event_bytes} variable bytes, exceeding the bounded {MODEL_REGISTRY_AUDIT_EVENT_BYTE_CAP}-byte transfer limit"
        )));
    }
    if total_event_bytes > MODEL_REGISTRY_AUDIT_SET_BYTE_CAP {
        return Err(ModelRegistryPersistenceError::Audit(format!(
            "model registry EventLedger selection set is {total_event_bytes} variable bytes, exceeding the bounded {MODEL_REGISTRY_AUDIT_SET_BYTE_CAP}-byte transfer limit"
        )));
    }
    if bounded_event_sequences.is_empty() {
        return Ok(Vec::new());
    }
    let rows = sqlx::query(
        r#"
        SELECT event_id,
               event_sequence,
               event_version,
               kernel_task_run_id,
               session_run_id,
               aggregate_type,
               aggregate_id,
               idempotency_key,
               event_type,
               actor_kind,
               actor_id,
               causation_id,
               correlation_id,
               payload_hash,
               source_component,
               payload,
               created_at AT TIME ZONE 'UTC' AS created_at_utc
        FROM ONLY kernel_event_ledger
        WHERE aggregate_type = 'model_runtime_registry'
          AND aggregate_id = ANY($1)
          AND source_component = 'model_runtime_registry'
          AND event_sequence = ANY($3)
        ORDER BY aggregate_id ASC, event_sequence ASC
        LIMIT $2
        "#,
    )
    .bind(aggregate_ids)
    .bind(limit)
    .bind(&bounded_event_sequences)
    .fetch_all(&mut **tx)
    .await?;
    if rows.len() != bounded_event_sequences.len() {
        return Err(ModelRegistryPersistenceError::Audit(
            "model registry EventLedger rows changed after bounded transfer preflight".to_string(),
        ));
    }
    rows.into_iter().map(decode_selection_event_row).collect()
}

fn decode_selection_event_row(
    row: PgRow,
) -> Result<PersistedSelectionEvent, ModelRegistryPersistenceError> {
    Ok(PersistedSelectionEvent {
        event_id: row.try_get("event_id")?,
        event_sequence: row.try_get("event_sequence")?,
        event_version: row.try_get("event_version")?,
        kernel_task_run_id: row.try_get("kernel_task_run_id")?,
        session_run_id: row.try_get("session_run_id")?,
        aggregate_type: row.try_get("aggregate_type")?,
        aggregate_id: row.try_get("aggregate_id")?,
        idempotency_key: row.try_get("idempotency_key")?,
        event_type: row.try_get("event_type")?,
        actor_kind: row.try_get("actor_kind")?,
        actor_id: row.try_get("actor_id")?,
        causation_id: row.try_get("causation_id")?,
        correlation_id: row.try_get("correlation_id")?,
        payload_hash: row.try_get("payload_hash")?,
        source_component: row.try_get("source_component")?,
        payload: row.try_get("payload")?,
        created_at_utc: row.try_get("created_at_utc")?,
    })
}

fn validate_selection_event_common(
    registration: &PersistedModelRegistration,
    event: &PersistedSelectionEvent,
    expected_revision: u64,
) -> Result<(), ModelRegistryPersistenceError> {
    let artifact = hex::encode(registration.artifact_sha256);
    let expected_idempotency =
        format!("model-runtime-selection:{artifact}:revision:{expected_revision}");
    let expected_task = format!("KTR-MODEL-RUNTIME-REGISTRY-{artifact}");
    let computed_payload_hash = sha256_hex(&canonical_json_bytes(&event.payload));
    let common_matches = event.event_sequence > 0
        && event.event_version == "kernel_event_v1"
        && event.kernel_task_run_id == expected_task
        && event.aggregate_type == "model_runtime_registry"
        && event.aggregate_id == registration.artifact_locator
        && event.idempotency_key == expected_idempotency
        && event.source_component == "model_runtime_registry"
        && event.payload_hash == computed_payload_hash;
    if !common_matches {
        return Err(audit_chain_corrupt(
            registration,
            format!(
                "EventLedger metadata/hash mismatch at revision {expected_revision} event {}",
                event.event_id
            ),
        ));
    }
    Ok(())
}

fn validate_initial_selection_event(
    registration: &PersistedModelRegistration,
    event: &PersistedSelectionEvent,
) -> Result<ModelRuntimeSelection, ModelRegistryPersistenceError> {
    let schema_id = event
        .payload
        .as_object()
        .and_then(|object| object.get("schema_id"))
        .and_then(Value::as_str)
        .ok_or_else(|| audit_chain_corrupt(registration, "initial selection schema_id missing"))?;
    let is_current_schema = schema_id == MODEL_RUNTIME_SELECTION_EVENT_SCHEMA_ID;
    let object = exact_payload_object(
        registration,
        &event.payload,
        if is_current_schema {
            &[
                "schema_id",
                "action",
                "artifact_sha256",
                "selection_revision",
                "runtime_binding",
                "runtime_role",
                "capabilities_schema_id",
                "declared_capabilities",
                "provider",
                "observed_runtime_model_id",
                "base_model_tag",
                "observed_by",
                "reason",
            ][..]
        } else {
            &[
                "schema_id",
                "action",
                "artifact_sha256",
                "selection_revision",
                "runtime_binding",
                "capabilities_schema_id",
                "declared_capabilities",
                "provider",
                "observed_runtime_model_id",
                "base_model_tag",
                "observed_by",
                "reason",
            ][..]
        },
        "initial selection",
    )?;
    if schema_id != MODEL_RUNTIME_SELECTION_EVENT_SCHEMA_ID
        && !LEGACY_MODEL_RUNTIME_SELECTION_EVENT_SCHEMA_IDS.contains(&schema_id)
    {
        return Err(audit_chain_corrupt(
            registration,
            format!("unsupported initial selection event schema {schema_id:?}"),
        ));
    }
    expect_payload_string(registration, object, "action", "initial_selection")?;
    expect_payload_string(
        registration,
        object,
        "artifact_sha256",
        &hex::encode(registration.artifact_sha256),
    )?;
    expect_payload_revision(registration, object, "selection_revision", 1)?;
    expect_payload_string(
        registration,
        object,
        "reason",
        "initial model-register selection",
    )?;

    let observed_model_id = payload_string(registration, object, "observed_runtime_model_id")?;
    let observed_uuid = Uuid::parse_str(observed_model_id).map_err(|_| {
        audit_chain_corrupt(
            registration,
            "initial selection observed_runtime_model_id is not a UUID",
        )
    })?;
    if observed_uuid.get_version_num() != 7 {
        return Err(audit_chain_corrupt(
            registration,
            "initial selection observed_runtime_model_id is not UUIDv7",
        ));
    }
    BaseModelTag::try_new(payload_string(registration, object, "base_model_tag")?)
        .map_err(|error| audit_chain_corrupt(registration, error.to_string()))?;
    OperatorId::try_new(payload_string(registration, object, "observed_by")?)
        .map_err(|error| audit_chain_corrupt(registration, error.to_string()))?;

    let expected_correlation = if schema_id == "hsk.model_runtime.selection_event@1" {
        observed_model_id.to_string()
    } else {
        registration.registry_row_id.to_string()
    };
    if event.event_type != KernelEventType::ModelRuntimeSelectionRecorded.as_str()
        || event.actor_kind != "system"
        || event.actor_id != "model-runtime-registry"
        || event.causation_id.is_some()
        || event.correlation_id.as_deref() != Some(expected_correlation.as_str())
        || event.session_run_id != format!("SR-MODEL-RUNTIME-REGISTRY-{expected_correlation}")
    {
        return Err(audit_chain_corrupt(
            registration,
            "initial selection EventLedger metadata/actor/correlation is invalid",
        ));
    }

    selection_from_payload_fields(registration, object)
}

fn validate_rebind_selection_event(
    registration: &PersistedModelRegistration,
    event: &PersistedSelectionEvent,
    expected_revision: u64,
) -> Result<(ModelRuntimeSelection, ModelRuntimeSelection), ModelRegistryPersistenceError> {
    let object = exact_payload_object(
        registration,
        &event.payload,
        &[
            "schema_id",
            "action",
            "artifact_sha256",
            "previous_selection_revision",
            "selection_revision",
            "previous_selection",
            "target_selection",
            "reason",
        ],
        "selection rebind",
    )?;
    let schema_id = payload_string(registration, object, "schema_id")?;
    if schema_id != MODEL_RUNTIME_SELECTION_EVENT_SCHEMA_ID
        && !LEGACY_MODEL_RUNTIME_SELECTION_EVENT_SCHEMA_IDS.contains(&schema_id)
    {
        return Err(audit_chain_corrupt(
            registration,
            format!("unsupported selection rebind event schema {schema_id:?}"),
        ));
    }
    expect_payload_string(registration, object, "action", "explicit_rebind")?;
    expect_payload_string(
        registration,
        object,
        "artifact_sha256",
        &hex::encode(registration.artifact_sha256),
    )?;
    expect_payload_revision(
        registration,
        object,
        "previous_selection_revision",
        expected_revision - 1,
    )?;
    expect_payload_revision(
        registration,
        object,
        "selection_revision",
        expected_revision,
    )?;
    if payload_string(registration, object, "reason")?
        .trim()
        .is_empty()
    {
        return Err(audit_chain_corrupt(
            registration,
            "selection rebind reason is empty",
        ));
    }

    let registry_row_id = registration.registry_row_id.to_string();
    if event.event_type != KernelEventType::ModelRuntimeSelectionRebound.as_str()
        || event.actor_kind != "operator"
        || event.actor_id.trim().is_empty()
        || !event
            .causation_id
            .as_deref()
            .is_some_and(|causation_id| !causation_id.is_empty())
        || event.correlation_id.as_deref() != Some(registry_row_id.as_str())
        || event.session_run_id != format!("SR-MODEL-RUNTIME-REGISTRY-{registry_row_id}")
    {
        return Err(audit_chain_corrupt(
            registration,
            format!("revision {expected_revision} rebind EventLedger metadata is invalid"),
        ));
    }

    let previous = selection_from_nested_payload(
        registration,
        object
            .get("previous_selection")
            .expect("exact key set checked"),
        "previous_selection",
    )?;
    let target = selection_from_nested_payload(
        registration,
        object
            .get("target_selection")
            .expect("exact key set checked"),
        "target_selection",
    )?;
    if previous == target {
        return Err(audit_chain_corrupt(
            registration,
            format!("revision {expected_revision} rebind does not change immutable selection"),
        ));
    }
    Ok((previous, target))
}

fn exact_payload_object<'a>(
    registration: &PersistedModelRegistration,
    payload: &'a Value,
    expected_keys: &[&str],
    context: &str,
) -> Result<&'a serde_json::Map<String, Value>, ModelRegistryPersistenceError> {
    let object = payload.as_object().ok_or_else(|| {
        audit_chain_corrupt(registration, format!("{context} payload is not an object"))
    })?;
    let actual_keys = object.keys().map(String::as_str).collect::<BTreeSet<_>>();
    let expected_keys = expected_keys.iter().copied().collect::<BTreeSet<_>>();
    if actual_keys != expected_keys {
        return Err(audit_chain_corrupt(
            registration,
            format!("{context} payload keys are not canonical"),
        ));
    }
    Ok(object)
}

fn selection_from_nested_payload(
    registration: &PersistedModelRegistration,
    value: &Value,
    context: &str,
) -> Result<ModelRuntimeSelection, ModelRegistryPersistenceError> {
    let has_runtime_role = value
        .as_object()
        .is_some_and(|object| object.contains_key("runtime_role"));
    let object = exact_payload_object(
        registration,
        value,
        if has_runtime_role {
            &[
                "runtime_binding",
                "runtime_role",
                "capabilities_schema_id",
                "declared_capabilities",
                "provider",
            ][..]
        } else {
            &[
                "runtime_binding",
                "capabilities_schema_id",
                "declared_capabilities",
                "provider",
            ][..]
        },
        context,
    )?;
    selection_from_payload_fields(registration, object)
}

fn selection_from_payload_fields(
    registration: &PersistedModelRegistration,
    object: &serde_json::Map<String, Value>,
) -> Result<ModelRuntimeSelection, ModelRegistryPersistenceError> {
    expect_payload_string(
        registration,
        object,
        "capabilities_schema_id",
        MODEL_RUNTIME_CAPABILITIES_SCHEMA_ID,
    )?;
    let runtime_binding =
        parse_runtime_binding(payload_string(registration, object, "runtime_binding")?)?;
    let runtime_role = match object.get("runtime_role").and_then(Value::as_str) {
        Some(token) => parse_runtime_role(token)?,
        None => registration.runtime_role,
    };
    let provider = parse_provider(payload_string(registration, object, "provider")?)?;
    let capabilities_value = object
        .get("declared_capabilities")
        .expect("selection payload key set checked")
        .clone();
    let declared_capabilities = serde_json::from_value::<ModelCapabilities>(
        capabilities_value.clone(),
    )
    .map_err(|error| {
        audit_chain_corrupt(
            registration,
            format!("selection capabilities cannot be decoded: {error}"),
        )
    })?;
    if serde_json::to_value(&declared_capabilities)? != capabilities_value {
        return Err(audit_chain_corrupt(
            registration,
            "selection capabilities payload is not canonical",
        ));
    }
    let selection = ModelRuntimeSelection {
        artifact_sha256: registration.artifact_sha256,
        runtime_binding,
        runtime_role,
        declared_capabilities,
        provider,
    };
    validate_selection(&selection)
        .map_err(|error| audit_chain_corrupt(registration, error.to_string()))?;
    Ok(selection)
}

fn payload_string<'a>(
    registration: &PersistedModelRegistration,
    object: &'a serde_json::Map<String, Value>,
    key: &str,
) -> Result<&'a str, ModelRegistryPersistenceError> {
    object.get(key).and_then(Value::as_str).ok_or_else(|| {
        audit_chain_corrupt(
            registration,
            format!("selection event `{key}` is not a string"),
        )
    })
}

fn expect_payload_string(
    registration: &PersistedModelRegistration,
    object: &serde_json::Map<String, Value>,
    key: &str,
    expected: &str,
) -> Result<(), ModelRegistryPersistenceError> {
    let actual = payload_string(registration, object, key)?;
    if actual != expected {
        return Err(audit_chain_corrupt(
            registration,
            format!("selection event `{key}` is `{actual}`, expected `{expected}`"),
        ));
    }
    Ok(())
}

fn expect_payload_revision(
    registration: &PersistedModelRegistration,
    object: &serde_json::Map<String, Value>,
    key: &str,
    expected: u64,
) -> Result<(), ModelRegistryPersistenceError> {
    let actual = object.get(key).and_then(Value::as_u64).ok_or_else(|| {
        audit_chain_corrupt(
            registration,
            format!("selection event `{key}` is not an unsigned integer"),
        )
    })?;
    if actual != expected {
        return Err(audit_chain_corrupt(
            registration,
            format!("selection event `{key}` is {actual}, expected {expected}"),
        ));
    }
    Ok(())
}

fn audit_chain_corrupt(
    registration: &PersistedModelRegistration,
    detail: impl Into<String>,
) -> ModelRegistryPersistenceError {
    ModelRegistryPersistenceError::CorruptRow(format!(
        "artifact {} selection audit chain is invalid: {}",
        hex::encode(registration.artifact_sha256),
        detail.into()
    ))
}

fn validate_selection_set(
    selections: &[ModelRuntimeSelection],
) -> Result<(), ModelRegistryPersistenceError> {
    if selections.len() > usize::try_from(MODEL_REGISTRY_ROW_ENUMERATION_CAP).unwrap_or(usize::MAX)
    {
        return Err(ModelRegistryPersistenceError::InvalidRegistration(format!(
            "configured selection set contains {} rows, exceeding the bounded {}-row limit",
            selections.len(),
            MODEL_REGISTRY_ROW_ENUMERATION_CAP
        )));
    }
    let mut artifact_hashes = BTreeSet::new();
    for selection in selections {
        validate_selection(selection)?;
        if !artifact_hashes.insert(selection.artifact_sha256) {
            return Err(ModelRegistryPersistenceError::InvalidRegistration(format!(
                "configured selection set contains duplicate artifact SHA-256 {}",
                hex::encode(selection.artifact_sha256)
            )));
        }
    }
    Ok(())
}

fn validate_registration_set(
    registrations: &[ModelRegistration],
) -> Result<Vec<ModelRuntimeSelection>, ModelRegistryPersistenceError> {
    if registrations.len()
        > usize::try_from(MODEL_REGISTRY_ROW_ENUMERATION_CAP).unwrap_or(usize::MAX)
    {
        return Err(ModelRegistryPersistenceError::InvalidRegistration(format!(
            "boot registration set contains {} rows, exceeding the bounded {}-row limit",
            registrations.len(),
            MODEL_REGISTRY_ROW_ENUMERATION_CAP
        )));
    }
    let mut selections = Vec::with_capacity(registrations.len());
    let mut artifact_hashes = BTreeSet::new();
    for registration in registrations {
        validate_registration(registration)?;
        if !artifact_hashes.insert(registration.sha256) {
            return Err(ModelRegistryPersistenceError::InvalidRegistration(format!(
                "boot registration set contains duplicate artifact SHA-256 {}",
                hex::encode(registration.sha256)
            )));
        }
        selections.push(ModelRuntimeSelection::from(registration));
    }
    Ok(selections)
}

fn validate_role_bound_registration_set(
    registrations: &[RoleBoundModelRegistration],
) -> Result<Vec<ModelRuntimeSelection>, ModelRegistryPersistenceError> {
    if registrations.len()
        > usize::try_from(MODEL_REGISTRY_ROW_ENUMERATION_CAP).unwrap_or(usize::MAX)
    {
        return Err(ModelRegistryPersistenceError::InvalidRegistration(format!(
            "boot registration set contains {} rows, exceeding the bounded {}-row limit",
            registrations.len(),
            MODEL_REGISTRY_ROW_ENUMERATION_CAP
        )));
    }
    let mut selections = Vec::with_capacity(registrations.len());
    let mut artifact_hashes = BTreeSet::new();
    for role_bound in registrations {
        validate_registration(&role_bound.registration)?;
        if !artifact_hashes.insert(role_bound.registration.sha256) {
            return Err(ModelRegistryPersistenceError::InvalidRegistration(format!(
                "boot registration set contains duplicate artifact SHA-256 {}",
                hex::encode(role_bound.registration.sha256)
            )));
        }
        selections.push(role_bound.selection());
    }
    Ok(selections)
}

fn validate_selection(
    selection: &ModelRuntimeSelection,
) -> Result<(), ModelRegistryPersistenceError> {
    validate_registration(&ModelRegistration {
        model_id: ModelId::new_v7(),
        artifact_path: PathBuf::from("model-runtime-selection-validation"),
        sha256: selection.artifact_sha256,
        runtime_binding: selection.runtime_binding,
        declared_capabilities: selection.declared_capabilities.clone(),
        base_model_tag: BaseModelTag::new("selection-validation"),
        registered_at_utc: Utc::now(),
        registered_by: OperatorId::new("model-runtime-registry"),
        provider: selection.provider,
    })
}

fn validate_registration(
    registration: &ModelRegistration,
) -> Result<(), ModelRegistryPersistenceError> {
    for (name, value) in [
        ("base_model_tag", registration.base_model_tag.as_str()),
        ("registered_by", registration.registered_by.as_str()),
    ] {
        if value.len() > MODEL_REGISTRY_INPUT_TEXT_BYTE_CAP {
            return Err(ModelRegistryPersistenceError::InvalidRegistration(format!(
                "{name} is {} bytes, exceeding the bounded {MODEL_REGISTRY_INPUT_TEXT_BYTE_CAP}-byte persistence limit",
                value.len()
            )));
        }
    }
    let mut registry = ModelRegistry::default();
    registry
        .register(registration.clone())
        .map_err(model_runtime_validation_error)
}

fn validate_rebind_request(
    request: &ExplicitModelRuntimeRebind,
) -> Result<(), ModelRegistryPersistenceError> {
    if !matches!(request.actor, KernelActor::Operator(_)) {
        return Err(ModelRegistryPersistenceError::InvalidRebind(
            "raw selection CAS proof requires an explicit operator actor".to_string(),
        ));
    }
    if request.actor.actor_id().trim().is_empty() {
        return Err(ModelRegistryPersistenceError::InvalidRebind(
            "actor id must not be empty".to_string(),
        ));
    }
    if request.actor.actor_id().len() > MODEL_REGISTRY_INPUT_TEXT_BYTE_CAP
        || request.reason.len() > MODEL_REGISTRY_INPUT_TEXT_BYTE_CAP
    {
        return Err(ModelRegistryPersistenceError::InvalidRebind(format!(
            "actor id and reason must each fit the bounded {MODEL_REGISTRY_INPUT_TEXT_BYTE_CAP}-byte persistence limit"
        )));
    }
    if request.reason.trim().is_empty() {
        return Err(ModelRegistryPersistenceError::InvalidRebind(
            "reason must not be empty".to_string(),
        ));
    }
    if request.expected_selection_revision == 0 {
        return Err(ModelRegistryPersistenceError::InvalidRebind(
            "expected selection revision must be at least one".to_string(),
        ));
    }
    Ok(())
}

fn model_runtime_validation_error(error: ModelRuntimeError) -> ModelRegistryPersistenceError {
    ModelRegistryPersistenceError::InvalidRegistration(error.to_string())
}

fn ensure_selection_matches(
    persisted: &PersistedModelRegistration,
    attempted: &ModelRuntimeSelection,
) -> Result<(), ModelRegistryPersistenceError> {
    if persisted.selection() == *attempted {
        return Ok(());
    }
    Err(ModelRegistryPersistenceError::SelectionConflict(format!(
        "artifact {} is already revision {} with adapter `{}` and a different immutable selection; restore the persisted selection or complete the governed unload-then-re-register workflow",
        hex::encode(attempted.artifact_sha256),
        persisted.selection_revision,
        persisted.runtime_binding.adapter_id(),
    )))
}

fn ensure_runtime_binding_matches(
    persisted: &PersistedModelRegistration,
    attempted: &ModelRuntimeSelection,
) -> Result<(), ModelRegistryPersistenceError> {
    if persisted.artifact_sha256 == attempted.artifact_sha256
        && persisted.runtime_binding == attempted.runtime_binding
        && persisted.provider == attempted.provider
    {
        return Ok(());
    }
    Err(ModelRegistryPersistenceError::SelectionConflict(format!(
        "artifact {} is already revision {} with adapter `{}`/provider `{}`; configured boot cannot change immutable runtime identity before artifact access",
        hex::encode(attempted.artifact_sha256),
        persisted.selection_revision,
        persisted.runtime_binding.adapter_id(),
        provider_token(persisted.provider),
    )))
}

fn ensure_observation_matches(
    persisted: &PersistedModelRegistration,
    attempted: &ModelRegistration,
) -> Result<(), ModelRegistryPersistenceError> {
    if persisted.last_observed_runtime_model_id == attempted.model_id
        && persisted.base_model_tag == attempted.base_model_tag
        && persisted.last_observed_by == attempted.registered_by
    {
        return Ok(());
    }
    Err(ModelRegistryPersistenceError::ObservationMismatch(format!(
        "artifact {} committed model_id/base_model_tag/observer as {}/{}/{}, expected {}/{}/{}",
        hex::encode(attempted.sha256),
        persisted.last_observed_runtime_model_id,
        persisted.base_model_tag.as_str(),
        persisted.last_observed_by.as_str(),
        attempted.model_id,
        attempted.base_model_tag.as_str(),
        attempted.registered_by.as_str(),
    )))
}

async fn load_active_selection_tx(
    tx: &mut AuthorityTransaction,
    purpose: ModelRuntimeSelectionPurpose,
    for_update: bool,
) -> Result<Option<PersistedActiveModelSelection>, ModelRegistryPersistenceError> {
    let row_lock = if for_update { " FOR UPDATE" } else { "" };
    let statement = format!(
        r#"
        SELECT schema_id, purpose, runtime_role, artifact_sha256, selection_revision,
               selection_created_event_id, selection_updated_event_id,
               selection_created_at_utc, selection_updated_at_utc
        FROM ONLY model_runtime_active_selection
        WHERE purpose = $1
        {row_lock}
        "#,
    );
    let decoded = sqlx::query(&statement)
        .bind(purpose.as_str())
        .fetch_optional(&mut **tx)
        .await?
        .map(decode_active_selection)
        .transpose()?;
    if let Some(selection) = &decoded {
        validate_active_selection_audit_tx(tx, selection, for_update).await?;
    }
    Ok(decoded)
}

fn decode_active_selection(
    row: PgRow,
) -> Result<PersistedActiveModelSelection, ModelRegistryPersistenceError> {
    let schema_id: String = row.try_get("schema_id")?;
    if schema_id != MODEL_RUNTIME_ACTIVE_SELECTION_SCHEMA_ID {
        return Err(ModelRegistryPersistenceError::CorruptRow(format!(
            "active ModelRuntime schema_id is `{schema_id}`, expected `{MODEL_RUNTIME_ACTIVE_SELECTION_SCHEMA_ID}`"
        )));
    }
    let purpose_token: String = row.try_get("purpose")?;
    let purpose = match purpose_token.as_str() {
        "application/default" => ModelRuntimeSelectionPurpose::ApplicationDefault,
        "embeddings/default" => ModelRuntimeSelectionPurpose::EmbeddingsDefault,
        _ => {
            return Err(ModelRegistryPersistenceError::CorruptRow(format!(
                "unknown active ModelRuntime purpose `{purpose_token}`"
            )))
        }
    };
    let runtime_role = parse_runtime_role(&row.try_get::<String, _>("runtime_role")?)?;
    if runtime_role != purpose.runtime_role() {
        return Err(ModelRegistryPersistenceError::CorruptRow(format!(
            "active ModelRuntime purpose {} carries role {}",
            purpose.as_str(),
            runtime_role.as_str()
        )));
    }
    let artifact_bytes: Vec<u8> = row.try_get("artifact_sha256")?;
    let artifact_sha256 = artifact_bytes.try_into().map_err(|bytes: Vec<u8>| {
        ModelRegistryPersistenceError::CorruptRow(format!(
            "active ModelRuntime artifact SHA-256 must be 32 bytes, got {}",
            bytes.len()
        ))
    })?;
    let revision: i64 = row.try_get("selection_revision")?;
    let selection_revision = u64::try_from(revision).map_err(|_| {
        ModelRegistryPersistenceError::CorruptRow(format!(
            "active ModelRuntime selection revision is invalid: {revision}"
        ))
    })?;
    if selection_revision == 0 {
        return Err(ModelRegistryPersistenceError::CorruptRow(
            "active ModelRuntime selection revision must be at least one".to_owned(),
        ));
    }
    Ok(PersistedActiveModelSelection {
        purpose,
        runtime_role,
        artifact_sha256,
        selection_revision,
        selection_created_event_id: row.try_get("selection_created_event_id")?,
        selection_updated_event_id: row.try_get("selection_updated_event_id")?,
        selection_created_at_utc: row.try_get("selection_created_at_utc")?,
        selection_updated_at_utc: row.try_get("selection_updated_at_utc")?,
    })
}

async fn validate_active_selection_audit_tx(
    tx: &mut AuthorityTransaction,
    selection: &PersistedActiveModelSelection,
    lock_row: bool,
) -> Result<(), ModelRegistryPersistenceError> {
    if selection.selection_revision == 1
        && (selection.selection_created_event_id != selection.selection_updated_event_id
            || selection.selection_created_at_utc != selection.selection_updated_at_utc)
    {
        return Err(ModelRegistryPersistenceError::CorruptRow(format!(
            "initial active purpose {} does not bind created and updated audit identity",
            selection.purpose.as_str()
        )));
    }
    let lock_clause = if lock_row { " FOR SHARE" } else { "" };
    let statement = format!(
        r#"
        SELECT event_type, aggregate_type, aggregate_id, source_component,
               causation_id, payload,
               created_at AT TIME ZONE 'UTC' AS created_at_utc
        FROM ONLY kernel_event_ledger
        WHERE event_id = $1
        {lock_clause}
        "#,
    );
    let row = sqlx::query(&statement)
        .bind(&selection.selection_updated_event_id)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or_else(|| {
            ModelRegistryPersistenceError::CorruptRow(format!(
                "active purpose {} updated audit event is absent",
                selection.purpose.as_str()
            ))
        })?;
    let event_type: String = row.try_get("event_type")?;
    let expected_event_type = if selection.selection_revision == 1 {
        KernelEventType::ModelRuntimeSelectionRecorded.as_str()
    } else {
        KernelEventType::ModelRuntimeSelectionRebound.as_str()
    };
    let aggregate_type: String = row.try_get("aggregate_type")?;
    let aggregate_id: String = row.try_get("aggregate_id")?;
    let source_component: String = row.try_get("source_component")?;
    let payload: Value = row.try_get("payload")?;
    let created_at_utc: DateTime<Utc> = row.try_get("created_at_utc")?;
    let payload_revision = payload.get("selection_revision").and_then(Value::as_u64);
    let payload_target = payload
        .get("target_artifact_sha256")
        .and_then(Value::as_str);
    let payload_schema = payload.get("schema_id").and_then(Value::as_str);
    let payload_purpose = payload.get("purpose").and_then(Value::as_str);
    let expected_target = hex::encode(selection.artifact_sha256);
    if event_type != expected_event_type
        || aggregate_type != "model_runtime_active_selection"
        || aggregate_id != selection.purpose.as_str()
        || source_component != "model_runtime_registry"
        || created_at_utc != selection.selection_updated_at_utc
        || payload_schema != Some(MODEL_RUNTIME_ACTIVE_SELECTION_EVENT_SCHEMA_ID)
        || payload_purpose != Some(selection.purpose.as_str())
        || payload_revision != Some(selection.selection_revision)
        || payload_target != Some(expected_target.as_str())
    {
        return Err(ModelRegistryPersistenceError::CorruptRow(format!(
            "active purpose {} projection does not match its latest EventLedger audit",
            selection.purpose.as_str()
        )));
    }
    if selection.selection_revision > 1 {
        let causation_id: Option<String> = row.try_get("causation_id")?;
        if causation_id.as_deref().is_none_or(str::is_empty) {
            return Err(ModelRegistryPersistenceError::CorruptRow(format!(
                "active purpose {} updated audit lacks causation",
                selection.purpose.as_str()
            )));
        }
    }
    Ok(())
}

fn build_active_selection_event(
    purpose: ModelRuntimeSelectionPurpose,
    previous: Option<&PersistedActiveModelSelection>,
    target_artifact_sha256: &[u8; 32],
    next_revision: u64,
    actor: KernelActor,
    reason: &str,
) -> Result<NewKernelEvent, ModelRegistryPersistenceError> {
    let purpose_token = purpose.as_str();
    let target = hex::encode(target_artifact_sha256);
    let event_type = if previous.is_some() {
        KernelEventType::ModelRuntimeSelectionRebound
    } else {
        KernelEventType::ModelRuntimeSelectionRecorded
    };
    let mut builder = NewKernelEvent::builder(
        format!("KTR-MODEL-RUNTIME-ACTIVE-{}", purpose_token.replace('/', "-")),
        format!("SR-MODEL-RUNTIME-ACTIVE-{}", Uuid::now_v7()),
        event_type,
        actor,
    )
    .aggregate("model_runtime_active_selection", purpose_token)
    .idempotency_key(format!(
        "model-runtime-active-selection:{purpose_token}:revision:{next_revision}"
    ))
    .source_component("model_runtime_registry")
    .payload(json!({
        "schema_id": MODEL_RUNTIME_ACTIVE_SELECTION_EVENT_SCHEMA_ID,
        "action": if previous.is_some() { "active_default_changed" } else { "active_default_initialized" },
        "purpose": purpose_token,
        "runtime_role": purpose.runtime_role().as_str(),
        "previous_artifact_sha256": previous.map(|row| hex::encode(row.artifact_sha256)),
        "target_artifact_sha256": target,
        "previous_selection_revision": previous.map(|row| row.selection_revision),
        "selection_revision": next_revision,
        "reason": reason,
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

fn build_initial_selection_event(
    registration: &ModelRegistration,
    selection: &ModelRuntimeSelection,
    registry_row_id: Uuid,
) -> Result<NewKernelEvent, ModelRegistryPersistenceError> {
    let artifact = hex::encode(selection.artifact_sha256);
    NewKernelEvent::builder(
        format!("KTR-MODEL-RUNTIME-REGISTRY-{artifact}"),
        format!("SR-MODEL-RUNTIME-REGISTRY-{registry_row_id}"),
        KernelEventType::ModelRuntimeSelectionRecorded,
        KernelActor::System("model-runtime-registry".to_string()),
    )
    .aggregate(
        "model_runtime_registry",
        artifact_locator_for_sha256(selection.artifact_sha256),
    )
    .idempotency_key(format!("model-runtime-selection:{artifact}:revision:1"))
    .correlation_id(registry_row_id.to_string())
    .source_component("model_runtime_registry")
    .payload(json!({
        "schema_id": MODEL_RUNTIME_SELECTION_EVENT_SCHEMA_ID,
        "action": "initial_selection",
        "artifact_sha256": artifact,
        "selection_revision": 1,
        "runtime_binding": runtime_binding_token(selection.runtime_binding),
        "runtime_role": selection.runtime_role.as_str(),
        "capabilities_schema_id": MODEL_RUNTIME_CAPABILITIES_SCHEMA_ID,
        "declared_capabilities": selection.declared_capabilities,
        "provider": provider_token(selection.provider),
        "observed_runtime_model_id": registration.model_id.to_string(),
        "base_model_tag": registration.base_model_tag.as_str(),
        "observed_by": registration.registered_by.as_str(),
        "reason": "initial model-register selection"
    }))
    .build()
    .map_err(|error| ModelRegistryPersistenceError::Audit(error.to_string()))
}

fn build_rebind_event(
    existing: &PersistedModelRegistration,
    target: &ModelRuntimeSelection,
    request: &ExplicitModelRuntimeRebind,
    next_revision: u64,
) -> Result<NewKernelEvent, ModelRegistryPersistenceError> {
    let artifact = hex::encode(target.artifact_sha256);
    NewKernelEvent::builder(
        format!("KTR-MODEL-RUNTIME-REGISTRY-{artifact}"),
        format!("SR-MODEL-RUNTIME-REGISTRY-{}", existing.registry_row_id),
        KernelEventType::ModelRuntimeSelectionRebound,
        request.actor.clone(),
    )
    .aggregate(
        "model_runtime_registry",
        artifact_locator_for_sha256(target.artifact_sha256),
    )
    .idempotency_key(format!(
        "model-runtime-selection:{artifact}:revision:{next_revision}"
    ))
    .causation_id(existing.selection_updated_event_id.clone())
    .correlation_id(existing.registry_row_id.to_string())
    .source_component("model_runtime_registry")
    .payload(json!({
        "schema_id": MODEL_RUNTIME_SELECTION_EVENT_SCHEMA_ID,
        "action": "explicit_rebind",
        "artifact_sha256": artifact,
        "previous_selection_revision": existing.selection_revision,
        "selection_revision": next_revision,
        "previous_selection": selection_event_payload(&existing.selection()),
        "target_selection": selection_event_payload(target),
        "reason": request.reason
    }))
    .build()
    .map_err(|error| ModelRegistryPersistenceError::Audit(error.to_string()))
}

fn selection_event_payload(selection: &ModelRuntimeSelection) -> Value {
    json!({
        "runtime_binding": runtime_binding_token(selection.runtime_binding),
        "runtime_role": selection.runtime_role.as_str(),
        "capabilities_schema_id": MODEL_RUNTIME_CAPABILITIES_SCHEMA_ID,
        "declared_capabilities": selection.declared_capabilities,
        "provider": provider_token(selection.provider)
    })
}

fn decode_row(row: PgRow) -> Result<PersistedModelRegistration, ModelRegistryPersistenceError> {
    let artifact_sha256: Vec<u8> = row.try_get("artifact_sha256")?;
    let sha256: [u8; 32] = artifact_sha256.try_into().map_err(|bytes: Vec<u8>| {
        ModelRegistryPersistenceError::CorruptRow(format!(
            "artifact_sha256 must be 32 bytes, got {}",
            bytes.len()
        ))
    })?;
    let schema_id: String = row.try_get("schema_id")?;
    if schema_id != MODEL_RUNTIME_REGISTRY_SCHEMA_ID {
        return Err(ModelRegistryPersistenceError::CorruptRow(format!(
            "schema_id is `{schema_id}`, expected `{MODEL_RUNTIME_REGISTRY_SCHEMA_ID}`"
        )));
    }
    let runtime_binding = parse_runtime_binding(&row.try_get::<String, _>("runtime_binding")?)?;
    let runtime_role = parse_runtime_role(&row.try_get::<String, _>("runtime_role")?)?;
    let provider = parse_provider(&row.try_get::<String, _>("provider")?)?;
    let capabilities_schema_id: String = row.try_get("capabilities_schema_id")?;
    if capabilities_schema_id != MODEL_RUNTIME_CAPABILITIES_SCHEMA_ID {
        return Err(ModelRegistryPersistenceError::CorruptRow(format!(
            "capabilities_schema_id is `{capabilities_schema_id}`, expected `{MODEL_RUNTIME_CAPABILITIES_SCHEMA_ID}`"
        )));
    }
    let capabilities: Value = row.try_get("capabilities_json")?;
    let declared_capabilities = serde_json::from_value::<ModelCapabilities>(capabilities.clone())?;
    let canonical_capabilities = serde_json::to_value(&declared_capabilities)?;
    if capabilities != canonical_capabilities {
        return Err(ModelRegistryPersistenceError::CorruptRow(format!(
            "capabilities_json is not canonical for schema `{MODEL_RUNTIME_CAPABILITIES_SCHEMA_ID}`"
        )));
    }
    let base_model_tag = BaseModelTag::try_new(row.try_get::<String, _>("base_model_tag")?)
        .map_err(model_runtime_validation_error)?;
    let last_observed_by = OperatorId::try_new(row.try_get::<String, _>("last_observed_by")?)
        .map_err(model_runtime_validation_error)?;
    let selection_revision_i64: i64 = row.try_get("selection_revision")?;
    let selection_revision = u64::try_from(selection_revision_i64).map_err(|_| {
        ModelRegistryPersistenceError::CorruptRow(format!(
            "selection_revision must be positive, got {selection_revision_i64}"
        ))
    })?;
    if selection_revision == 0 {
        return Err(ModelRegistryPersistenceError::CorruptRow(
            "selection_revision must be at least one".to_string(),
        ));
    }
    let observed_uuid: Uuid = row.try_get("last_observed_runtime_model_id")?;
    if observed_uuid.get_version_num() != 7 {
        return Err(ModelRegistryPersistenceError::CorruptRow(format!(
            "last_observed_runtime_model_id must be UUIDv7, got {observed_uuid}"
        )));
    }
    let selection_created_event_id: String = row.try_get("selection_created_event_id")?;
    let selection_updated_event_id: String = row.try_get("selection_updated_event_id")?;
    if selection_created_event_id.trim().is_empty() || selection_updated_event_id.trim().is_empty()
    {
        return Err(ModelRegistryPersistenceError::CorruptRow(
            "selection audit event ids must not be empty".to_string(),
        ));
    }
    let persisted = PersistedModelRegistration {
        schema_id,
        registry_row_id: row.try_get("registry_row_id")?,
        artifact_sha256: sha256,
        artifact_locator: row.try_get("artifact_locator")?,
        last_observed_runtime_model_id: ModelId::from(observed_uuid),
        runtime_binding,
        runtime_role,
        capabilities_schema_id,
        declared_capabilities,
        provider,
        base_model_tag,
        last_observed_by,
        selection_revision,
        selection_created_event_id,
        selection_updated_event_id,
        selection_created_at_utc: row.try_get("selection_created_at_utc")?,
        selection_updated_at_utc: row.try_get("selection_updated_at_utc")?,
        last_observed_at_utc: row.try_get("last_observed_at_utc")?,
    };
    validate_artifact_locator(persisted.artifact_sha256, &persisted.artifact_locator)?;
    if persisted.selection_updated_at_utc < persisted.selection_created_at_utc {
        return Err(ModelRegistryPersistenceError::CorruptRow(
            "selection_updated_at_utc precedes selection_created_at_utc".to_string(),
        ));
    }
    if persisted.last_observed_at_utc < persisted.selection_created_at_utc {
        return Err(ModelRegistryPersistenceError::CorruptRow(
            "last_observed_at_utc precedes selection_created_at_utc".to_string(),
        ));
    }
    validate_selection(&persisted.selection()).map_err(|error| {
        ModelRegistryPersistenceError::CorruptRow(format!(
            "persisted immutable selection is invalid: {error}"
        ))
    })?;
    Ok(persisted)
}

fn validate_artifact_locator(
    artifact_sha256: [u8; 32],
    artifact_locator: &str,
) -> Result<(), ModelRegistryPersistenceError> {
    let expected = artifact_locator_for_sha256(artifact_sha256);
    if artifact_locator != expected {
        return Err(ModelRegistryPersistenceError::CorruptRow(format!(
            "artifact locator `{artifact_locator}` does not bind to persisted SHA-256 {}",
            hex::encode(artifact_sha256)
        )));
    }
    Ok(())
}

struct RequiredColumn {
    name: &'static str,
    udt_name: &'static str,
    is_nullable: &'static str,
}

fn required_model_registry_columns() -> &'static [RequiredColumn] {
    &[
        RequiredColumn {
            name: "schema_id",
            udt_name: "text",
            is_nullable: "NO",
        },
        RequiredColumn {
            name: "registry_row_id",
            udt_name: "uuid",
            is_nullable: "NO",
        },
        RequiredColumn {
            name: "artifact_sha256",
            udt_name: "bytea",
            is_nullable: "NO",
        },
        RequiredColumn {
            name: "artifact_locator",
            udt_name: "text",
            is_nullable: "NO",
        },
        RequiredColumn {
            name: "last_observed_runtime_model_id",
            udt_name: "uuid",
            is_nullable: "NO",
        },
        RequiredColumn {
            name: "runtime_binding",
            udt_name: "text",
            is_nullable: "NO",
        },
        RequiredColumn {
            name: "runtime_role",
            udt_name: "text",
            is_nullable: "NO",
        },
        RequiredColumn {
            name: "capabilities_schema_id",
            udt_name: "text",
            is_nullable: "NO",
        },
        RequiredColumn {
            name: "capabilities_json",
            udt_name: "jsonb",
            is_nullable: "NO",
        },
        RequiredColumn {
            name: "provider",
            udt_name: "text",
            is_nullable: "NO",
        },
        RequiredColumn {
            name: "base_model_tag",
            udt_name: "text",
            is_nullable: "NO",
        },
        RequiredColumn {
            name: "last_observed_by",
            udt_name: "text",
            is_nullable: "NO",
        },
        RequiredColumn {
            name: "selection_revision",
            udt_name: "int8",
            is_nullable: "NO",
        },
        RequiredColumn {
            name: "selection_created_event_id",
            udt_name: "text",
            is_nullable: "NO",
        },
        RequiredColumn {
            name: "selection_updated_event_id",
            udt_name: "text",
            is_nullable: "NO",
        },
        RequiredColumn {
            name: "selection_created_at_utc",
            udt_name: "timestamptz",
            is_nullable: "NO",
        },
        RequiredColumn {
            name: "selection_updated_at_utc",
            udt_name: "timestamptz",
            is_nullable: "NO",
        },
        RequiredColumn {
            name: "last_observed_at_utc",
            udt_name: "timestamptz",
            is_nullable: "NO",
        },
        // HBR-PRIV account-bound resource scope, added by migrations
        // 0363/0364. These are pinned here for the same reason every other
        // column is: this check is an exact-shape authority pin, so a scope
        // column that silently disappeared (or was never applied, as happened
        // when 0363's `public.`-qualified guard skipped non-public schemas)
        // must fail the authority check loudly rather than degrade the registry
        // to an unscoped full-table read.
        //
        // NULLable until WP-KERNEL-006 MT-015 tightens them: there is no
        // LocalAccount authority to backfill existing rows from yet. WP-1 fails
        // closed in application code instead of trusting the column.
        RequiredColumn {
            name: "owner_account_id",
            udt_name: "uuid",
            is_nullable: "YES",
        },
        RequiredColumn {
            name: "actor_principal_id",
            udt_name: "uuid",
            is_nullable: "YES",
        },
        RequiredColumn {
            name: "authenticated_session_id",
            udt_name: "uuid",
            is_nullable: "YES",
        },
        RequiredColumn {
            name: "access_space_id",
            udt_name: "uuid",
            is_nullable: "YES",
        },
        RequiredColumn {
            name: "workspace_id",
            udt_name: "text",
            is_nullable: "YES",
        },
    ]
}

struct RequiredConstraint {
    name: &'static str,
    constraint_type: &'static str,
    accepted_definitions: &'static [&'static str],
    description: &'static str,
}

impl RequiredConstraint {
    fn accepts_definition(&self, actual: &str) -> bool {
        let actual = normalize_constraint_definition(actual);
        self.accepted_definitions
            .iter()
            .any(|expected| actual == normalize_constraint_definition(expected))
    }
}

fn required_model_registry_constraints() -> &'static [RequiredConstraint] {
    &[
        RequiredConstraint {
            name: "pk_model_runtime_registry",
            constraint_type: "p",
            accepted_definitions: &["PRIMARY KEY (registry_row_id)"],
            description: "primary key (registry_row_id)",
        },
        RequiredConstraint {
            name: "uq_model_runtime_registry_artifact_sha256",
            constraint_type: "u",
            accepted_definitions: &["UNIQUE (artifact_sha256)"],
            description: "unique (artifact_sha256)",
        },
        RequiredConstraint {
            name: "chk_model_runtime_registry_schema_id",
            constraint_type: "c",
            accepted_definitions: &[
                "CHECK ((schema_id = 'hsk.model_runtime_registry.row@2'::text))",
            ],
            description: "schema_id equals hsk.model_runtime_registry.row@2",
        },
        RequiredConstraint {
            name: "chk_model_runtime_registry_artifact_sha256",
            constraint_type: "c",
            accepted_definitions: &["CHECK ((octet_length(artifact_sha256) = 32))"],
            description: "artifact SHA-256 is 32 bytes",
        },
        RequiredConstraint {
            name: "chk_model_runtime_registry_artifact_locator",
            constraint_type: "c",
            accepted_definitions: &[
                "CHECK ((artifact_locator = ('sha256:'::text || encode(artifact_sha256, 'hex'::text))))",
            ],
            description: "portable locator exactly equals sha256:<artifact_sha256 hex>",
        },
        RequiredConstraint {
            name: "chk_model_runtime_registry_runtime_binding",
            constraint_type: "c",
            accepted_definitions: &[
                "CHECK ((runtime_binding = ANY (ARRAY['llama_cpp'::text, 'candle'::text])))",
            ],
            description: "runtime binding is llama_cpp or candle",
        },
        RequiredConstraint {
            name: "chk_model_runtime_registry_runtime_role",
            constraint_type: "c",
            accepted_definitions: &[
                "CHECK ((runtime_role = ANY (ARRAY['completion'::text, 'embedding'::text])))",
            ],
            description: "runtime role is completion or embedding",
        },
        RequiredConstraint {
            name: "chk_model_runtime_registry_capabilities_schema_id",
            constraint_type: "c",
            accepted_definitions: &[
                "CHECK ((capabilities_schema_id = 'hsk.model_runtime.capabilities@1'::text))",
            ],
            description: "capabilities_schema_id equals hsk.model_runtime.capabilities@1",
        },
        RequiredConstraint {
            name: "chk_model_runtime_registry_capabilities",
            constraint_type: "c",
            accepted_definitions: &["CHECK ((jsonb_typeof(capabilities_json) = 'object'::text))"],
            description: "capabilities JSON is an object",
        },
        RequiredConstraint {
            name: "chk_model_runtime_registry_provider",
            constraint_type: "c",
            accepted_definitions: &["CHECK ((provider = 'local'::text))"],
            description: "provider is local",
        },
        RequiredConstraint {
            name: "chk_model_runtime_registry_base_model_tag",
            constraint_type: "c",
            accepted_definitions: &["CHECK ((length(btrim(base_model_tag)) > 0))"],
            description: "non-empty base_model_tag",
        },
        RequiredConstraint {
            name: "chk_model_runtime_registry_last_observed_by",
            constraint_type: "c",
            accepted_definitions: &["CHECK ((length(btrim(last_observed_by)) > 0))"],
            description: "non-empty last_observed_by",
        },
        RequiredConstraint {
            name: "chk_model_runtime_registry_selection_revision",
            constraint_type: "c",
            accepted_definitions: &["CHECK ((selection_revision >= 1))"],
            description: "selection_revision at least one",
        },
        RequiredConstraint {
            name: "fk_model_runtime_registry_selection_created_event",
            constraint_type: "f",
            accepted_definitions: &[
                "FOREIGN KEY (selection_created_event_id) REFERENCES kernel_event_ledger(event_id)",
            ],
            description: "created-event EventLedger foreign key",
        },
        RequiredConstraint {
            name: "fk_model_runtime_registry_selection_updated_event",
            constraint_type: "f",
            accepted_definitions: &[
                "FOREIGN KEY (selection_updated_event_id) REFERENCES kernel_event_ledger(event_id)",
            ],
            description: "updated-event EventLedger foreign key",
        },
    ]
}

fn normalize_constraint_definition(definition: &str) -> String {
    definition
        .to_ascii_lowercase()
        .replace("::text", "")
        .chars()
        .filter(|character| !character.is_whitespace() && *character != '"')
        .collect()
}

fn normalize_index_definition(definition: &str) -> String {
    definition
        .to_ascii_lowercase()
        .chars()
        .filter(|character| !character.is_whitespace() && *character != '"')
        .collect()
}

fn artifact_locator_for_sha256(sha256: [u8; 32]) -> String {
    format!("sha256:{}", hex::encode(sha256))
}

fn runtime_binding_token(binding: RuntimeBinding) -> &'static str {
    match binding {
        RuntimeBinding::LlamaCpp => "llama_cpp",
        RuntimeBinding::Candle => "candle",
    }
}

fn parse_runtime_binding(token: &str) -> Result<RuntimeBinding, ModelRegistryPersistenceError> {
    match token {
        "llama_cpp" => Ok(RuntimeBinding::LlamaCpp),
        "candle" => Ok(RuntimeBinding::Candle),
        _ => Err(ModelRegistryPersistenceError::CorruptRow(format!(
            "unknown runtime_binding `{token}`"
        ))),
    }
}

fn parse_runtime_role(token: &str) -> Result<ModelRuntimeRole, ModelRegistryPersistenceError> {
    match token {
        "completion" => Ok(ModelRuntimeRole::Completion),
        "embedding" => Ok(ModelRuntimeRole::Embedding),
        _ => Err(ModelRegistryPersistenceError::CorruptRow(format!(
            "unknown runtime_role `{token}`"
        ))),
    }
}

fn provider_token(provider: ProviderKind) -> &'static str {
    match provider {
        ProviderKind::Local => "local",
        ProviderKind::ExternalCompat => "external_compat",
        ProviderKind::ByokCloud => "byok_cloud",
        ProviderKind::OfficialCli => "official_cli",
    }
}

fn parse_provider(token: &str) -> Result<ProviderKind, ModelRegistryPersistenceError> {
    match token {
        "local" => Ok(ProviderKind::Local),
        "external_compat" => Ok(ProviderKind::ExternalCompat),
        "byok_cloud" => Ok(ProviderKind::ByokCloud),
        "official_cli" => Ok(ProviderKind::OfficialCli),
        _ => Err(ModelRegistryPersistenceError::CorruptRow(format!(
            "unknown provider `{token}`"
        ))),
    }
}
