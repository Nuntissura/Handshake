use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    future::Future,
    io,
    path::{Path, PathBuf},
    pin::Pin,
    sync::{
        atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering},
        Arc, Mutex as StdMutex,
    },
    time::{Duration, Instant},
};

use handshake_storage_support::diagnostics::{observe, observe_result};
use sha2::Digest;
use surrealdb::{
    engine::local::{Db, RocksDb},
    opt::{auth::Record as RecordSignin, Config as EngineConfig},
    types::{SurrealValue, Value},
    Surreal,
};
use thiserror::Error;
use tokio::sync::{oneshot, watch, Mutex, OwnedRwLockReadGuard, OwnedRwLockWriteGuard, RwLock, RwLockWriteGuard};
use tokio_util::sync::CancellationToken;

use super::{DefaultStorageGuard, StorageGuard, StorageResult};

mod cloud_model_lane;
mod loom_search;
mod model_lane;
mod model_registry;
mod palmistry;
mod process_ledger;
mod swarm_outbox;
mod user_manual_knowledge;
pub(crate) mod workspace_activation_outcome;
mod workspace_settings;

mod first_owner_bootstrap_transaction;
mod ai_job_store;
mod ai_ready_store;
mod block_view_store;
mod blocks;
mod bridge_store;
mod calendar_store;
mod canvas_store;
mod database;
mod documents;
pub(crate) mod event_ledger;
mod governance_check_store;
mod kb003_store;
mod kernel_crdt_store;
mod kernel_queue_store;
pub mod keyed_lock;
mod knowledge;
pub mod local_accounts;
pub(crate) mod locus_store;
pub(crate) mod loom_canvas_store;
pub(crate) mod loom_store;
mod mcp_store;
#[cfg(any(test, feature = "surreal-test-support"))]
mod mt136_adversarial_regression_proof;
#[cfg(any(test, feature = "surreal-test-support"))]
mod mt136_database_surface_proof_a;
#[cfg(any(test, feature = "surreal-test-support"))]
mod mt136_database_surface_proof_b;
#[cfg(any(test, feature = "surreal-test-support"))]
mod mt136_database_surface_proof_c;
#[cfg(any(test, feature = "surreal-test-support"))]
mod mt136_kernel_action_submitter_proof;
#[cfg(any(test, feature = "surreal-test-support"))]
mod mt136_knowledge_surface_proof;
#[cfg(any(test, feature = "surreal-test-support"))]
mod mt136_proof_harness;
#[cfg(any(test, feature = "surreal-test-support"))]
mod mt136_registry_integrity_proof;
#[cfg(any(test, feature = "surreal-test-support"))]
mod mt136_rich_document_delete_proof;
mod preferences;
mod promotion_store;
pub mod resource_authority;
pub mod retry;
mod schema;
mod search_store;
mod session_store;
mod state_store;
pub(crate) mod structured_collab_store;
pub mod swarm_load_report;
#[cfg(feature = "surreal-test-support")]
mod test_inspector;
mod visual_debug_store;
mod wiki_store;
mod workflow_store;
mod workspaces;

#[cfg(test)]
mod resource_authority_tests;

pub(crate) use cloud_model_lane::{
    bootstrap_cloud_model_lane_schema, CloudModelLaneRecordKind, CloudModelLaneScope,
    CloudModelLaneStore, CloudModelLaneStoredRow,
};
pub use database::SurrealDatabase;
pub use loom_search::{
    bootstrap_loom_search_schema, SurrealEmbeddingRegistration, SurrealLoomIndexMutation,
    SurrealLoomSearchEvidence, SurrealLoomSearchStore,
};
#[cfg(feature = "test-utils")]
pub use model_lane::{ContextCompletionTestControl, ContextCoordinationBusy, ContextCoordinationTestHolder, ContextAdmissionTestControl, ContextAdmissionTestObservation, observe_context_coordination_busy_for_test};
pub(crate) use model_lane::{
    bootstrap_model_lane_schema, ContextCoordinationPurpose, ModelLaneRecordKind, ModelLaneScope,
    TxCheckedRoleEvidence,
    NavigationReadSelector, NavigationMessageInput, NavigationLaneInput,
    SurrealModelLaneNavigationRow, SurrealModelLaneNavigationReceipt, SurrealCrdtLeaseClaimOutcome,
    SurrealCrdtUpdateAppendOutcome, SurrealModelLaneCrdtEventWrite, SurrealModelLaneCrdtGuard,
    SurrealModelLaneCrdtLease, SurrealModelLaneCrdtLeaseHistory, SurrealModelLaneCrdtLeaseWrite,
    SurrealModelLaneCrdtProposal, SurrealModelLaneCrdtProposalRecord,
    SurrealModelLaneCrdtProposalWrite, SurrealModelLaneCrdtSnapshot, SurrealModelLaneCrdtUpdate,
    SurrealModelLaneMessageGuard, SurrealModelLaneRecord, SurrealModelLaneRoutingAttemptRow,
    SurrealModelLaneRoutingAttemptWrite, SurrealModelLaneRoutingClaim,
    SurrealModelLaneRoutingCommit, SurrealModelLaneRoutingEventWrite,
    SurrealModelLaneRoutingExecutionRow, SurrealModelLaneRoutingExecutionWrite,
    SurrealModelLaneRoutingOutboxRow, SurrealModelLaneRoutingOutboxWrite,
    SurrealModelLaneSchemaRow, SurrealModelLaneStore, SurrealModelLaneWrite,
};
pub use model_registry::{bootstrap_model_registry_schema, SurrealModelRegistryStore};
pub use palmistry::{
    bootstrap_palmistry_schema, SurrealPalmistryError, SurrealPalmistryStore,
    SurrealPalmistryVerifier,
};
pub(crate) use palmistry::{
    PalmistryWatcherAdoptionInput, PalmistryWatcherAdoptionReceipt, PalmistryWatcherAdoptionResult,
    PalmistryWatcherOsRecheck,
    PalmistryWatcherAdmissionManifest, PalmistryWatcherPreparedAdmission,
    PalmistryWatcherAdmissionIdentity, PalmistryWatcherInstalledPermit,
    WatcherAdmissionDeadline, WatcherAdmissionPurpose, prepare_watcher_admission,
};
pub use process_ledger::{
    ProcessOwnershipInspection, SurrealModelLaneStaleSessionSource, SurrealProcessLedgerStore,
};
pub use swarm_outbox::{
    bootstrap_swarm_outbox_schema, SurrealSwarmOutboxError, SurrealSwarmOutboxEvent,
    SurrealSwarmOutboxStore,
};
pub use user_manual_knowledge::{
    bootstrap_user_manual_knowledge_schema, SurrealUserManualKnowledgeStore,
    UserManualKnowledgeEntityMutation,
};
pub use workspace_settings::{
    bootstrap_workspace_settings_schema, SurrealWorkspaceSettingsError,
    SurrealWorkspaceSettingsState, SurrealWorkspaceSettingsStore, SurrealWorkspaceSettingsWrite,
};
#[cfg(feature = "surreal-test-support")]
pub use workspace_settings::{
    workspace_settings_test_event_count, workspace_settings_test_seed_legacy_unscoped_row,
};

/// Bootstrap every production schema against the one process-owned embedded
/// store. Dedicated schemas are ordered after the shared substrate and after
/// any authority they reference; consumers receive clones of this same handle.
pub async fn bootstrap_production_schemas(
    storage: &SurrealStorage,
) -> Result<SchemaBootstrapReport, SurrealStorageError> {
    static COMPOSITION_MUTEX: Mutex<()> = Mutex::const_new(());
    schema::bounded_bootstrap(schema::BOOTSTRAP_WATCHDOG, async {
        let _composition_guard = COMPOSITION_MUTEX.lock().await;
        let report = schema::bootstrap_schema(storage).await?;
        if schema::is_production_report(&report) {
            initialize_watcher_admission_verifier(storage).await;
            return Ok(report);
        }
        schema::begin_production_schema(storage).await?;
        install_production_module_schemas(storage).await?;
        schema::finish_production_schema(storage, report.outcome).await
    }).await
}

// One ordered installation body for production and the existing direct Mem proof.
async fn install_production_module_schemas(storage: &SurrealStorage) -> Result<(), SurrealStorageError> {
    model_registry::bootstrap_model_registry_schema(storage).await?;
    loom_search::bootstrap_loom_search_schema(storage).await?;
    model_lane::bootstrap_model_lane_schema(storage).await?;
    cloud_model_lane::bootstrap_cloud_model_lane_schema(storage).await?;
    process_ledger::bootstrap_process_ledger_schema(storage).await?;
    palmistry::install_palmistry_schema(storage).await?;
    initialize_watcher_admission_verifier(storage).await;
    swarm_outbox::install_swarm_outbox_schema(storage).await?;
    user_manual_knowledge::bootstrap_user_manual_knowledge_schema(storage).await?;
    workspace_settings::install_workspace_settings_schema(storage).await?;
    Ok(())
}

#[cfg(feature = "surreal-test-support")]
pub fn loom_folder_sibling_key_for_test(
    workspace_id: &str,
    parent_folder_id: Option<&str>,
    name: &str,
) -> String {
    loom_store::loom_folder_sibling_key(workspace_id, parent_folder_id, name)
}
pub use kb003_store::SurrealKb003Storage;
pub(crate) use knowledge::KnowledgeRichDocumentDeleteOutcome;
pub use schema::{
    bootstrap_atelier_schema, bootstrap_schema, SchemaBootstrapOutcome, SchemaBootstrapReport,
    DECLARATIVE_SCHEMA_CATALOG_SHA256, EXPECTED_SCHEMA_INFO_SHA256, GENERATED_SURREALQL_SHA256,
    SCHEMA_LINEAGE_SHA256, SCHEMA_REVISION, SCHEMA_VERSION,
};
#[cfg(feature = "surreal-test-support")]
pub use schema::{
    bootstrap_mt137_flight_recorder_test_schema, bootstrap_mt137_process_ledger_test_schema,
};

#[cfg(feature = "surreal-test-support")]
pub async fn run_mt136_surface_proofs() -> StorageResult<()> {
    eprintln!("MT136_PROOF_START adversarial_regressions");
    mt136_adversarial_regression_proof::run_all().await?;
    eprintln!("MT136_PROOF_PASS adversarial_regressions");
    eprintln!("MT136_PROOF_START database_surface_a");
    mt136_database_surface_proof_a::run_all().await?;
    eprintln!("MT136_PROOF_PASS database_surface_a");
    eprintln!("MT136_PROOF_START database_surface_b");
    mt136_database_surface_proof_b::run_all().await?;
    eprintln!("MT136_PROOF_PASS database_surface_b");
    eprintln!("MT136_PROOF_START database_surface_c");
    mt136_database_surface_proof_c::run_all().await?;
    eprintln!("MT136_PROOF_PASS database_surface_c");
    eprintln!("MT136_PROOF_START knowledge_surface");
    mt136_knowledge_surface_proof::run_all().await?;
    eprintln!("MT136_PROOF_PASS knowledge_surface");
    eprintln!("MT136_PROOF_START kernel_action_submitter");
    mt136_kernel_action_submitter_proof::run_all().await?;
    eprintln!("MT136_PROOF_PASS kernel_action_submitter");
    eprintln!("MT136_PROOF_START registry_integrity");
    mt136_registry_integrity_proof::run_all().await?;
    eprintln!("MT136_PROOF_PASS registry_integrity");
    eprintln!("MT136_PROOF_START rich_document_delete");
    mt136_rich_document_delete_proof::run_all().await?;
    eprintln!("MT136_PROOF_PASS rich_document_delete");
    Ok(())
}

#[cfg(feature = "surreal-test-support")]
pub async fn run_mt136_adversarial_regression_proof() -> StorageResult<()> {
    mt136_adversarial_regression_proof::run_all().await
}

#[cfg(feature = "surreal-test-support")]
pub async fn run_mt136_kernel_action_submitter_proof() -> StorageResult<()> {
    mt136_kernel_action_submitter_proof::run_all().await
}

#[cfg(feature = "surreal-test-support")]
pub async fn run_mt136_registry_integrity_proof() -> StorageResult<()> {
    mt136_registry_integrity_proof::run_all().await
}
#[cfg(feature = "surreal-test-support")]
pub use test_inspector::{
    FieldCatalog, FieldSelector, IndexCatalog, ProjectedRow, RecordIdentity, ReferenceCatalog,
    RowFilter, ScalarValue, SchemaCatalogSnapshot, SurrealTestInspector, SurrealTestInspectorError,
    SurrealTestMutator, TableCatalog, TableSelector, TestFieldMutation, TestMutationValue,
    TestRecordKey,
};

pub const HANDSHAKE_DATA_DIR_ENV: &str = "HANDSHAKE_DATA_DIR";
pub const DEFAULT_NAMESPACE: &str = "handshake";
pub const DEFAULT_DATABASE: &str = "primary";
pub const DEFAULT_STORE_DIRECTORY: &str = "handshake-surreal";
pub const DEFAULT_SHUTDOWN_WAIT: Duration = Duration::from_secs(30);
/// MT-142 AC-142-7: how long shutdown waits for in-flight leases before it
/// cancels cooperative work (retry loops, keyed-lock waits).
pub const DEFAULT_DRAIN_GRACE: Duration = Duration::from_secs(5);
/// MT-142: caller-side bound applied by [`SurrealStorage::with_data_operation`]
/// to EVERY data operation of every store (knowledge, loom, wiki, canvas, ...).
///
/// 300 s rather than the 30 s first proposed (review R2-1-10, accepted with
/// this written reason): in the 4-way parallel `knowledge_documents_api_tests`
/// batch on the HDD (concurrent fresh bootstraps, `sync=every` fsync) a single
/// save statement exceeded 30 s and the engine still held it 35 s later at
/// teardown (`mt142-knowledge_documents_api_tests-20260910T173440Z.err.log`,
/// `ShutdownStillInProgress`), while the same test passes alone in 87 s, and
/// the fresh ~4,500-statement schema bootstrap exceeds 60 s under that load
/// (`suite-run-1.out.log:1718,1731,1735`). A tight default converts disk load
/// into terminal, outcome-unknown `StatementTimeout`s. Production consequence:
/// a hung statement blocks its request for up to 5 minutes, beyond typical
/// 30-60 s HTTP client timeouts, whose retries multiply engine work; deployments
/// and fixtures that want a tighter bound lower it per store with
/// [`SurrealStorageConfig::with_statement_timeout`], and the load report
/// records the effective value.
pub const DEFAULT_STATEMENT_TIMEOUT: Duration = Duration::from_secs(300);
/// MT-142: recommended engine-level query deadline when a caller opts in via
/// [`SurrealStorageConfig::with_engine_timeouts`]. NOT applied by default
/// (review R2-1-10, accepted with this written reason): engine deadlines are
/// per engine open and cannot exclude the fresh schema bootstrap (one
/// ~4,500-statement transaction), which exceeds 60 s on a loaded HDD (MT-141
/// baseline `suite-run-1.out.log:1718,1731,1735`, "running for over 60
/// seconds"); with these values applied the executor cancelled that
/// transaction (`surrealdb-core-3.2.0/src/dbs/executor.rs:1034-1049`) and
/// bootstrap failed closed (run `mt142-LIB-20260910T111428Z`). Therefore no
/// engine-side cap exists by default; the caller-side
/// [`DEFAULT_STATEMENT_TIMEOUT`] in [`SurrealStorage::with_data_operation`]
/// bounds every data operation instead, and stores whose bootstrap is known to
/// fit can enable these values.
pub const DEFAULT_ENGINE_QUERY_TIMEOUT: Duration = Duration::from_secs(30);
/// MT-142: recommended engine-level write-transaction deadline for opt-in use
/// (see [`DEFAULT_ENGINE_QUERY_TIMEOUT`] for why it is not the default).
pub const DEFAULT_ENGINE_TRANSACTION_TIMEOUT: Duration = Duration::from_secs(60);

pub(crate) type SurrealClient = Surreal<Db>;

pub(crate) struct WorkspaceArtifactPublicationOperationContext {
    original_operation_id: uuid::Uuid,
    storage: std::sync::Weak<SurrealStorageInner>,
    namespace: String,
    database: String,
    scope: resource_authority::RecordUserScope,
    original_monotonic: Instant,
    original_utc: chrono::DateTime<chrono::Utc>,
    deadline: Instant,
    expires_at: chrono::DateTime<chrono::Utc>,
    authority_expires_at: chrono::DateTime<chrono::Utc>,
}

impl WorkspaceArtifactPublicationOperationContext {
    pub(crate) fn operation_id(&self) -> uuid::Uuid { self.original_operation_id }
    pub(crate) fn deadline(&self) -> Instant { self.deadline }
    pub(crate) fn expires_at(&self) -> &chrono::DateTime<chrono::Utc> { &self.expires_at }
}

/// RAM observation of the worker's database transaction only.
/// This does not certify filesystem/publication outcome or release custody.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AuthenticatedTransactionContextObservation {
    Uncertain,
    CommitObserved,
    CancelObserved,
    CallbackPanicObserved,
}

pub(crate) struct WorkspaceArtifactPublicationReconciliationRead {
    original: Arc<WorkspaceArtifactPublicationOperationContext>,
    deadline: Instant,
    client: Option<Arc<SurrealClient>>,
    authority_expires_at: chrono::DateTime<chrono::Utc>,
}

impl WorkspaceArtifactPublicationReconciliationRead {
    pub(crate) fn deadline(&self) -> Instant { self.deadline }
    pub(crate) fn authority_expires_at(&self) -> chrono::DateTime<chrono::Utc> { self.authority_expires_at }
    pub(crate) fn matches_original(&self, original: &Arc<WorkspaceArtifactPublicationOperationContext>) -> bool {
        Arc::ptr_eq(&self.original, original) && self.authority_expires_at == original.authority_expires_at
    }
}
#[derive(Clone, Copy)]
enum WorkspaceArtifactEngineFinalization {
    Commit,
    Rollback,
}

pub(crate) enum ArtifactPublicationCanonicalDecision {
    Commit,
    Abort,
}

pub(crate) struct ArtifactPublicationCanonicalTerminalForOwner {
    original: Arc<WorkspaceArtifactPublicationOperationContext>,
    resources: Arc<crate::storage::artifacts::ArtifactPublicationResources>,
    decision: ArtifactPublicationCanonicalDecision,
}

impl ArtifactPublicationCanonicalTerminalForOwner {
    pub(crate) fn operation_id(&self) -> uuid::Uuid { self.original.operation_id() }
    pub(crate) fn decision(&self) -> &ArtifactPublicationCanonicalDecision { &self.decision }
    pub(crate) fn matches_resources(
        &self, resources: &Arc<crate::storage::artifacts::ArtifactPublicationResources>,
    ) -> bool { Arc::ptr_eq(&self.resources, resources) }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum OriginalRunCreateFinalization {
    Commit,
    Rollback,
}

struct OriginalRunCreateBinding {
    operation_id: uuid::Uuid,
    storage: std::sync::Weak<SurrealStorageInner>,
    owner: Arc<dyn std::any::Any + Send + Sync>,
    _scope: resource_authority::RecordUserScope,
    _deadline: Instant,
}

struct OriginalRunCreateBeginWitness {
    binding: Arc<OriginalRunCreateBinding>,
}

fn retain_original_runcreate_error<E: Send + 'static>(
    retention: Option<&Arc<StdMutex<OriginalRunCreateRetention>>>, error: &mut Option<E>,
) {
    if let Some(retention) = retention {
        let mut state = retention.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.error.is_none() {
            state.error = error.take().map(|error| Box::new(error) as Box<dyn std::any::Any + Send>);
        }
    }
}

fn seal_original_runcreate_finalization(
    retention: Option<&Arc<StdMutex<OriginalRunCreateRetention>>>,
    witness: &mut Option<OriginalRunCreateBeginWitness>,
    finalization: OriginalRunCreateFinalization,
) -> Result<(), SurrealStorageError> {
    if let Some(retention) = retention {
        let witness = witness.take().ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
        let mut state = retention.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let operation_id = witness.binding.operation_id;
        if state.finalized.is_some() || state.proof.is_some()
            || !state.binding.as_ref().is_some_and(|binding| Arc::ptr_eq(binding, &witness.binding))
        {
            return Err(SurrealStorageError::TransactionRetentionMismatch { operation_id });
        }
        state.finalized = Some(finalization);
        state.proof = Some(OriginalRunCreateFinalizationProof { binding: witness.binding, finalization });
    }
    Ok(())
}

pub(crate) struct OriginalRunCreateFinalizationProof {
    binding: Arc<OriginalRunCreateBinding>,
    finalization: OriginalRunCreateFinalization,
}

impl OriginalRunCreateFinalizationProof {
    pub(crate) fn finalization(&self) -> OriginalRunCreateFinalization { self.finalization }
    pub(crate) fn matches_original(
        &self, storage: &SurrealStorage, operation_id: uuid::Uuid,
        owner: &Arc<dyn std::any::Any + Send + Sync>,
    ) -> bool {
        self.binding.operation_id == operation_id && Arc::ptr_eq(&self.binding.owner, owner)
            && self.binding.storage.upgrade().is_some_and(|inner| Arc::ptr_eq(&inner, &storage.inner))
    }
}

pub(crate) struct OriginalRunCreateOutcome<T, E> {
    result: Result<T, E>,
    proof: Option<OriginalRunCreateFinalizationProof>,
}

impl<T, E> OriginalRunCreateOutcome<T, E> {
    pub(crate) fn into_parts(self) -> (Result<T, E>, Option<OriginalRunCreateFinalizationProof>) {
        (self.result, self.proof)
    }
}

struct OriginalRunCreateRetention {
    owner: Arc<dyn std::any::Any + Send + Sync>,
    binding: Option<Arc<OriginalRunCreateBinding>>,
    proof: Option<OriginalRunCreateFinalizationProof>,
    finalized: Option<OriginalRunCreateFinalization>,
    value: Option<Box<dyn std::any::Any + Send>>,
    error: Option<Box<dyn std::any::Any + Send>>,
    finished: bool,
}
struct WatcherAdmissionCredentials {
    secret_hex: String,
    key_version: String,
    not_after: chrono::DateTime<chrono::Utc>,
}

fn load_watcher_admission_credentials_from_environment()
    -> Result<WatcherAdmissionCredentials, SurrealPalmistryError>
{
    let invalid = || SurrealPalmistryError::PermissionDenied;
    let secret_hex = env::var("HANDSHAKE_WATCHER_ADMISSION_INSTALLER_SECRET_HEX")
        .map_err(|_| invalid())?;
    if secret_hex.len() != 64 || !secret_hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(invalid().into());
    }
    let key_version = env::var("HANDSHAKE_WATCHER_ADMISSION_INSTALLER_KEY_VERSION")
        .map_err(|_| invalid())?;
    if key_version.is_empty() || !key_version.bytes().all(|byte|
        byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(invalid().into());
    }
    let not_after = chrono::DateTime::parse_from_rfc3339(
        &env::var("HANDSHAKE_WATCHER_ADMISSION_INSTALLER_NOT_AFTER")
            .map_err(|_| invalid())?)
        .map_err(|_| invalid())?;
    if not_after.offset().local_minus_utc() != 0 {
        return Err(invalid().into());
    }
    let not_after = not_after.with_timezone(&chrono::Utc);
    if not_after <= chrono::Utc::now() { return Err(invalid().into()); }
    Ok(WatcherAdmissionCredentials {
        secret_hex: secret_hex.to_ascii_lowercase(), key_version, not_after,
    })
}

impl WatcherAdmissionCredentials {
    fn verifier(&self, access: &str) -> String {
        let mut digest = sha2::Sha256::new();
        digest.update(b"handshake-watcher-admission-v1|");
        digest.update(access.as_bytes());
        digest.update(b"|");
        digest.update(self.key_version.as_bytes());
        digest.update(b"|");
        digest.update(self.secret_hex.as_bytes());
        hex::encode(digest.finalize())
    }
}



enum WatcherAdmissionBootstrapFailure {
    Lease,
    Query,
    Response,
}

#[derive(surrealdb::types::SurrealValue)]
struct WatcherAdmissionVerifierConfiguration {
    key_version: String,
    admission_verifier: String,
    installer_verifier: String,
    not_after: chrono::DateTime<chrono::Utc>,
}

const WATCHER_ADMISSION_VERIFIER_BOOTSTRAP_SQL: &str = r#"
BEGIN TRANSACTION;
LET $existing = (SELECT * FROM palmistry_watcher_admission_config WHERE key_version = $key_version);
IF array::len($existing) = 0 {
    CREATE palmistry_watcher_admission_config CONTENT {
        key_version: $key_version, admission_verifier: $admission_verifier,
        installer_verifier: $installer_verifier, not_after: $not_after, revoked_at: NONE
    };
} ELSE IF array::len($existing) != 1
    OR $existing[0].admission_verifier != $admission_verifier
    OR $existing[0].installer_verifier != $installer_verifier
    OR $existing[0].not_after != $not_after
    OR $existing[0].revoked_at != NONE {
    THROW 'watcher admission configuration conflict';
};
COMMIT TRANSACTION;
"#;

async fn initialize_watcher_admission_verifier(storage: &SurrealStorage) {
    if let Some(credentials) = &storage.inner.watcher_admission_credentials {
        let configuration = WatcherAdmissionVerifierConfiguration {
            key_version: credentials.key_version.clone(),
            admission_verifier: credentials.verifier("palmistry_watcher_admission"),
            installer_verifier: credentials.verifier("palmistry_watcher_permit_installer"),
            not_after: credentials.not_after,
        };
        let configured = bootstrap_watcher_admission_verifier(storage, configuration).await.is_ok();
        storage.inner.watcher_admission_configured.store(configured, Ordering::Release);
    }
}

async fn bootstrap_watcher_admission_verifier(
    storage: &SurrealStorage,
    configuration: WatcherAdmissionVerifierConfiguration,
) -> Result<(), SurrealStorageError> {
    *storage.inner.watcher_admission_bootstrap_failure.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(WatcherAdmissionBootstrapFailure::Lease);
    let private_storage = storage.clone();
    let result = storage.with_admin_operation(move |admin| Box::pin(async move {
        *private_storage.inner.watcher_admission_bootstrap_failure.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(WatcherAdmissionBootstrapFailure::Query);
        let response = admin.client.query(WATCHER_ADMISSION_VERIFIER_BOOTSTRAP_SQL)
            .bind(surrealdb::types::SurrealValue::into_value(configuration))
            .await.map_err(|_| SurrealStorageError::InvalidModelLaneRecord {
                reason: "watcher admission configuration unavailable",
            })?;
        *private_storage.inner.watcher_admission_bootstrap_failure.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(WatcherAdmissionBootstrapFailure::Response);
        response.check().map_err(|_| SurrealStorageError::InvalidModelLaneRecord {
            reason: "watcher admission configuration unavailable",
        })?;
        Ok(())
    })).await;
    if result.is_ok() {
        *storage.inner.watcher_admission_bootstrap_failure.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
    }
    result
}

pub(crate) struct WatcherAdmissionBudget {
    pub(crate) limit: Duration,
    pub(crate) credential_not_after: chrono::DateTime<chrono::Utc>,
}

pub(crate) enum WatcherAdmissionInspection {
    NoPriorRelevantAdmission,
    ExistingReadOnly(PalmistryWatcherAdoptionResult),
}


#[derive(surrealdb::types::SurrealValue)]
struct WatcherAdmissionSigninScope {
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(surrealdb::types::SurrealValue)]
struct WatcherAdmissionSigninParams {
    scope: WatcherAdmissionSigninScope,
    resource: surrealdb::types::RecordId,
    parent: surrealdb::types::RecordId,
    grant: surrealdb::types::RecordId,
    token_hash: String,
    channel_binding_hash: String,
    key_version: String,
    installer_secret_hex: String,
}

fn watcher_admission_signin_params(
    prepared: &PalmistryWatcherPreparedAdmission,
    original: &resource_authority::RecordUserScope,
    credentials: &WatcherAdmissionCredentials,
) -> Result<WatcherAdmissionSigninParams, SurrealStorageError> {
    let captured = prepared.authorization().authority().record_user_scope();
    if original.session_token != captured.session_token
        || original.channel_binding_hash != captured.channel_binding_hash
        || original.session_id != captured.session_id
        || original.grant_id != captured.grant_id
        || original.resource_id != captured.resource_id
        || original.workspace_id != captured.workspace_id
        || original.capability_id != captured.capability_id
    {
        return Err(watcher_admission_controller_error());
    }
    let scope = prepared.authorization().exact_scope();
    Ok(WatcherAdmissionSigninParams {
        scope: WatcherAdmissionSigninScope {
            owner_account_id: scope.owner_account_id.as_uuid().to_string(),
            actor_principal_id: scope.actor_principal_id.as_uuid().to_string(),
            authenticated_session_id: scope.authenticated_session_id.as_uuid().to_string(),
            access_space_id: scope.access_space_id.as_uuid().to_string(),
            workspace_id: scope.workspace_id.as_str().to_owned(),
        },
        resource: prepared.authorization().resource_id().clone(),
        parent: prepared.authorization().authority().workspace_parent().id.clone(),
        grant: prepared.authorization().grant_id().clone(),
        token_hash: hex::encode(sha2::Sha256::digest(original.session_token.as_bytes())),
        channel_binding_hash: original.channel_binding_hash.clone()
            .ok_or_else(watcher_admission_controller_error)?,
        key_version: credentials.key_version.clone(),
        installer_secret_hex: credentials.secret_hex.clone(),
    })
}

fn watcher_admission_controller_error() -> SurrealStorageError {
    SurrealStorageError::InvalidModelLaneRecord {
        reason: "watcher admission controller unavailable",
    }
}

struct WatcherAdmissionController {
    prepared: Arc<PalmistryWatcherPreparedAdmission>,
    state: Mutex<WatcherAdmissionControllerState>,
}

struct WatcherAdmissionControllerState {
    phase: WatcherAdmissionControllerPhase,
    stage: WatcherAdmissionStage,
    installer_client: Option<SurrealClient>,
    identity: Option<PalmistryWatcherAdmissionIdentity>,
    installed_permit: Option<PalmistryWatcherInstalledPermit>,
    callback_receipts: Option<Vec<PalmistryWatcherAdoptionReceipt>>,
    actual_outcome: Option<WatcherAdmissionActualOutcome>,
    reconciliation: Option<WatcherAdmissionReconciliation>,
}


impl WatcherAdmissionController {
    fn new(prepared: Arc<PalmistryWatcherPreparedAdmission>)
        -> Result<Arc<Self>, SurrealPalmistryError>
    {
        prepared.validate_binding()?;
        let operation_id = prepared.operation_id();
        if operation_id != prepared.authorization().operation_id()
            || operation_id != prepared.manifest().operation_id()
            || prepared.deadline().monotonic() <= Instant::now()
            || prepared.deadline().expires_at() > prepared.authorization().expires_at()
        {
            return Err(SurrealPalmistryError::PermissionDenied);
        }
        Ok(Arc::new(Self {
            prepared,
            state: Mutex::new(WatcherAdmissionControllerState {
                phase: WatcherAdmissionControllerPhase::Prepared,
                stage: WatcherAdmissionStage::Prepared,
                installer_client: None,
                identity: None,
                installed_permit: None,
                callback_receipts: None,
                actual_outcome: None,
                reconciliation: None,
            }),
        }))
    }

    async fn observe_settlement(&self, observed: ActualSettlement) {
        let mut state = self.state.lock().await;
        let outcome = match observed {
            ActualSettlement::CommitObserved => match state.callback_receipts.take() {
                Some(receipts) => WatcherAdmissionActualOutcome::Committed(receipts),
                None => WatcherAdmissionActualOutcome::Uncertain,
            },
            ActualSettlement::CancelNoEffectObserved => WatcherAdmissionActualOutcome::CancelNoEffectObserved,
            ActualSettlement::CallbackPanicRollbackObserved => WatcherAdmissionActualOutcome::CallbackPanicRollbackObserved,
            _ => WatcherAdmissionActualOutcome::Uncertain,
        };
        state.phase = if matches!(&outcome, WatcherAdmissionActualOutcome::Uncertain) {
            WatcherAdmissionControllerPhase::Uncertain
        } else { WatcherAdmissionControllerPhase::Settled };
        state.actual_outcome = Some(outcome);
    }

    async fn begin_issuance(&self) -> Result<(), SurrealPalmistryError> {
        self.prepared.validate_binding()?;
        let mut state = self.state.lock().await;
        if !matches!(state.phase, WatcherAdmissionControllerPhase::Prepared)
            || self.prepared.deadline().monotonic() <= Instant::now()
        {
            return Err(SurrealPalmistryError::PermissionDenied);
        }
        state.phase = WatcherAdmissionControllerPhase::Issuing;
        Ok(())
    }
}

enum WatcherAdmissionStage {
    Prepared,
    OrdinaryAuthentication,
    AdmissionAuthentication,
    ProtectedIdentity,
    InstallerAuthentication,
    PermitInstall,
    PermitRead,
    TransactionBegin,
    FixedTransaction,
    ReconciliationRead,
}

enum WatcherAdmissionReconciliation {
    PermitConfirmed,
    Uncertain,
}

enum WatcherAdmissionActualOutcome {
    Committed(Vec<PalmistryWatcherAdoptionReceipt>),
    CancelNoEffectObserved,
    CallbackPanicRollbackObserved,
    Uncertain,
}

enum WatcherAdmissionControllerPhase {
    Prepared,
    Issuing,
    Installed,
    Running,
    Settled,
    Uncertain,
}


const LIFECYCLE_OPEN: u8 = 0;
const LIFECYCLE_CLOSING: u8 = 1;
const LIFECYCLE_CLOSED: u8 = 2;

#[derive(Debug, Error)]
pub enum SurrealStorageError {
    #[error("invalid model-lane authority record: {reason}")]
    InvalidModelLaneRecord { reason: &'static str },
    #[error("embedded cloud model-lane record has an invalid shape: {reason}")]
    InvalidCloudModelLaneRecord { reason: &'static str },

    #[error("failed to prepare embedded database path {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("embedded database error: {0}")]
    Database(#[from] surrealdb::Error),
    #[error("embedded transaction commit failed after its callback completed: {0}")]
    TransactionCommit(surrealdb::Error),
    #[error("embedded transaction cancellation failed: {0}")]
    TransactionCancel(surrealdb::Error),
    #[error("protected transaction requires an authenticated record-user envelope")]
    TransactionAuthorityRequired,
    #[error("resource authority infrastructure is unavailable")]
    ResourceAuthorityUnavailable,
    #[error("protected transaction {operation_id} has an unresolved settlement; retry is forbidden")]
    TransactionSettlementPending { operation_id: uuid::Uuid },

    #[error("album membership operation {operation:?} has an unresolved outcome: {source}")]
    AlbumMutationUnknown {
        operation: AlbumMutationOperationReference,
        source: Box<SurrealStorageError>,
    },
    #[error("protected transaction {operation_id} retention does not match its publication completion")]
    TransactionRetentionMismatch { operation_id: uuid::Uuid },
    #[error("embedded transaction caller was cancelled")]
    TransactionCancelled,
    #[error("embedded transaction callback panicked")]
    TransactionCallbackPanicked,
    #[error("embedded transaction worker failed: {0}")]
    TransactionWorker(String),
    #[error("embedded database is closed")]
    Closed,
    #[error("no platform-local application data directory is available")]
    MissingApplicationDataDirectory,
    #[error("embedded database data directory must not be empty")]
    EmptyDataDirectory,
    #[error(
        "embedded database path {path} is incompatible with the SurrealDB 3.2 endpoint parser: {reason}"
    )]
    IncompatibleEndpointPath { path: PathBuf, reason: &'static str },
    #[error(
        "embedded database context mismatch: expected namespace/database {expected_namespace}/{expected_database}, observed {actual_namespace}/{actual_database}"
    )]
    ContextMismatch {
        expected_namespace: String,
        expected_database: String,
        actual_namespace: String,
        actual_database: String,
    },
    #[error("shutdown cannot be called from inside an embedded database operation")]
    ReentrantShutdown,
    #[error("embedded database shutdown failed: {0}")]
    Shutdown(Arc<str>),
    #[error(
        "embedded database shutdown is still draining operations after {waited_ms} ms; closure continues in the background"
    )]
    ShutdownStillInProgress { waited_ms: u128 },
    #[error("embedded database shutdown wait timeout must be greater than zero")]
    InvalidShutdownWaitTimeout,
    #[error("embedded storage timeout `{setting}` must be greater than zero")]
    InvalidTimeout { setting: &'static str },
    /// The last SDK handle was dropped but the engine had not released the
    /// store's `LOCK` within the shutdown budget (MT-142 AC-142-7). The store
    /// stays closed; a reopen of the same path fails until the engine finishes
    /// the abandoned statement.
    #[error(
        "embedded engine did not release its store lock within {waited_ms} ms after the last handle was dropped; the store stays closed without a release proof"
    )]
    EngineReleaseUnproven { waited_ms: u128 },
    /// The caller stopped waiting for one statement attempt (MT-142). Terminal
    /// by contract: the engine may still apply the statement, so it is never
    /// retried.
    #[error(
        "embedded statement attempt exceeded the {waited_ms} ms statement timeout; the engine may still apply it"
    )]
    StatementTimeout { waited_ms: u128 },
    /// The enclosing operation's budget was already spent, so the statement was
    /// NEVER dispatched. Distinct from [`Self::StatementTimeout`], which leaves
    /// the outcome unknown: here the engine never saw the statement, so no
    /// effect can have been applied.
    #[error(
        "embedded statement was not dispatched: the {budget_ms} ms operation budget was already spent"
    )]
    StatementBudgetExhausted { budget_ms: u128 },
    /// MT-154 hardening (IV 2026-09-23, dumps hang-run15/18/21/22): the schema bootstrap did not
    /// finish within its watchdog. Observed cause: the commit coordinator blocked in the OS file
    /// flush (`WinWritableFile::Sync` -> `NtFlushBuffersFile`) with no competing I/O. The store
    /// may be partially bootstrapped; a later open resumes or fails closed.
    #[error(
        "schema bootstrap stalled after {waited_ms} ms (possible OS flush stall); the store may be partially bootstrapped"
    )]
    BootstrapStalled { waited_ms: u128 },
    #[error("embedded workspace record has an invalid shape: {reason}")]
    InvalidWorkspaceRecord { reason: &'static str },
    #[error("embedded document record has an invalid shape: {reason}")]
    InvalidDocumentRecord { reason: &'static str },
    #[error("embedded block record has an invalid shape: {reason}")]
    InvalidBlockRecord { reason: &'static str },
    #[error("embedded preference record has an invalid shape: {reason}")]
    InvalidPreferenceRecord { reason: &'static str },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurrealStorageConfig {
    path: PathBuf,
    namespace: String,
    database: String,
    shutdown_wait: Duration,
    drain_grace: Duration,
    statement_timeout: Duration,
    engine_query_timeout: Option<Duration>,
    engine_transaction_timeout: Option<Duration>,
    /// TEST-ONLY (IV 2026-09-23): open RocksDB with `?sync=never`. Always false outside test
    /// support; see [`Self::with_test_sync_from_env`].
    test_datastore_sync_never: bool,
    /// TEST-ONLY: keep the engine default (`Every`) even when [`HANDSHAKE_TEST_SURREAL_SYNC_ENV`]
    /// is `never`; for crash/durability proofs. See [`Self::with_test_datastore_sync_durable`].
    test_datastore_sync_durable: bool,
}

/// TEST-ONLY switch read by the test-support store openers: `never` disables the per-commit WAL
/// fsync for that store. Unset (the default) keeps the engine default (`Every`), so durability,
/// crash and restart proofs keep real fsync.
#[cfg(any(test, feature = "surreal-test-support"))]
pub const HANDSHAKE_TEST_SURREAL_SYNC_ENV: &str = "HANDSHAKE_TEST_SURREAL_SYNC";

#[cfg(feature = "surreal-test-support")]
fn valid_test_scope_name(name: &str, prefix: &str) -> bool {
    name.strip_prefix(prefix).is_some_and(|suffix| suffix.len() == 32
        && suffix.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)))
}

impl SurrealStorageConfig {
    pub fn from_env() -> Result<Self, SurrealStorageError> {
        let data_dir = match env::var_os(HANDSHAKE_DATA_DIR_ENV) {
            Some(path) if !path.is_empty() => PathBuf::from(path),
            Some(_) => return Err(SurrealStorageError::EmptyDataDirectory),
            None => dirs::data_local_dir()
                .map(|path| path.join("handshake"))
                .ok_or(SurrealStorageError::MissingApplicationDataDirectory)?,
        };
        Self::for_data_dir(data_dir)
    }

    pub fn for_data_dir(data_dir: impl AsRef<Path>) -> Result<Self, SurrealStorageError> {
        let data_dir = data_dir.as_ref();
        if data_dir.as_os_str().is_empty() {
            return Err(SurrealStorageError::EmptyDataDirectory);
        }
        Self::for_store_path(data_dir.join(DEFAULT_STORE_DIRECTORY))
    }

    #[cfg(test)]
    pub fn with_path(path: impl Into<PathBuf>) -> Result<Self, SurrealStorageError> {
        Self::for_store_path(path.into())
    }

    fn for_store_path(path: impl Into<PathBuf>) -> Result<Self, SurrealStorageError> {
        let path = path.into();
        if path.as_os_str().is_empty() {
            return Err(SurrealStorageError::EmptyDataDirectory);
        }
        Ok(Self {
            path: absolute_path(path)?,
            namespace: DEFAULT_NAMESPACE.to_owned(),
            database: DEFAULT_DATABASE.to_owned(),
            shutdown_wait: DEFAULT_SHUTDOWN_WAIT,
            drain_grace: DEFAULT_DRAIN_GRACE,
            statement_timeout: DEFAULT_STATEMENT_TIMEOUT,
            // Off by default; see DEFAULT_ENGINE_QUERY_TIMEOUT for the evidence.
            engine_query_timeout: None,
            engine_transaction_timeout: None,
            test_datastore_sync_never: false,
            test_datastore_sync_durable: false,
        })
    }

    #[cfg(feature = "surreal-test-support")]
    pub fn for_test_scope(
        store_path: impl Into<PathBuf>, namespace: &str, database: &str, shutdown_wait: Duration,
    ) -> Result<Self, SurrealStorageError> {
        if !valid_test_scope_name(namespace, "hs_test_ns_") || !valid_test_scope_name(database, "hs_test_db_") {
            return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "test catalog scope must use strict generated identifiers" });
        }
        let mut config = Self::for_store_path(store_path)?;
        config.namespace = namespace.to_owned();
        config.database = database.to_owned();
        config.with_shutdown_wait_timeout(shutdown_wait)
    }

    /// TEST-ONLY: the embedded engine never reads `SURREAL_DATASTORE_SYNC` (surrealdb-3.2.0
    /// `engine/local/native.rs:131` builds from an empty `ConfigMap`); its only sync input is the
    /// endpoint query string (`surrealdb-core-3.2.0 kvs/ds.rs:582-590`, `?sync=never` ->
    /// `datastore_sync`). With [`HANDSHAKE_TEST_SURREAL_SYNC_ENV`]`=never` the store opens with that
    /// query string; otherwise nothing changes.
    #[cfg(any(test, feature = "surreal-test-support"))]
    pub fn with_test_sync_from_env(mut self) -> Self {
        if env::var(HANDSHAKE_TEST_SURREAL_SYNC_ENV).as_deref() == Ok("never") {
            self.test_datastore_sync_never = true;
        }
        self
    }

    /// TEST-ONLY: force `?sync=never` for this store (unit proofs of the switch).
    #[cfg(any(test, feature = "surreal-test-support"))]
    pub fn with_test_datastore_sync_never(mut self) -> Self {
        self.test_datastore_sync_never = true;
        self
    }

    /// TEST-ONLY: pin this store to the engine default sync (`Every`) regardless of
    /// [`HANDSHAKE_TEST_SURREAL_SYNC_ENV`]. Crash/durability proofs use it on every open and reopen.
    #[cfg(any(test, feature = "surreal-test-support"))]
    pub fn with_test_datastore_sync_durable(mut self) -> Self {
        self.test_datastore_sync_never = false;
        self.test_datastore_sync_durable = true;
        self
    }

    /// Whether this store opens with `?sync=never`. Test-support builds apply
    /// [`HANDSHAKE_TEST_SURREAL_SYNC_ENV`]`=never` to EVERY opener (first open and reopen alike, so
    /// the endpoint form never differs between them), unless the config is pinned durable. Without
    /// test support only the explicit flag counts, and nothing sets it.
    pub fn test_datastore_sync_never(&self) -> bool {
        #[cfg(any(test, feature = "surreal-test-support"))]
        if !self.test_datastore_sync_durable
            && env::var(HANDSHAKE_TEST_SURREAL_SYNC_ENV).as_deref() == Ok("never")
        {
            return true;
        }
        self.test_datastore_sync_never
    }

    /// Sets how long each caller waits for the shared background close.
    ///
    /// The close continues safely if a caller times out. Zero is rejected
    /// because it would make every shutdown call observationally fail.
    pub fn with_shutdown_wait_timeout(
        mut self,
        timeout: Duration,
    ) -> Result<Self, SurrealStorageError> {
        if timeout.is_zero() {
            return Err(SurrealStorageError::InvalidShutdownWaitTimeout);
        }
        self.shutdown_wait = timeout;
        Ok(self)
    }

    /// Sets how long shutdown waits for in-flight leases before cancelling
    /// cooperative work (MT-142 AC-142-7). Zero cancels immediately.
    pub fn with_drain_grace(mut self, grace: Duration) -> Self {
        self.drain_grace = grace;
        self
    }

    /// Sets the caller-side bound on one statement attempt (MT-142). A timed
    /// out attempt is terminal and never retried. Zero is rejected.
    pub fn with_statement_timeout(
        mut self,
        timeout: Duration,
    ) -> Result<Self, SurrealStorageError> {
        if timeout.is_zero() {
            return Err(SurrealStorageError::InvalidTimeout {
                setting: "statement_timeout",
            });
        }
        self.statement_timeout = timeout;
        Ok(self)
    }

    /// Sets the engine-level query and write-transaction deadlines handed to
    /// the embedded datastore at open (`None` disables one; both are `None`
    /// by default, see [`DEFAULT_ENGINE_QUERY_TIMEOUT`]). They apply to every
    /// statement including schema bootstrap, so enable them only on stores
    /// whose bootstrap is known to fit. Zero is rejected.
    pub fn with_engine_timeouts(
        mut self,
        query_timeout: Option<Duration>,
        transaction_timeout: Option<Duration>,
    ) -> Result<Self, SurrealStorageError> {
        if query_timeout.is_some_and(|timeout| timeout.is_zero()) {
            return Err(SurrealStorageError::InvalidTimeout {
                setting: "engine_query_timeout",
            });
        }
        if transaction_timeout.is_some_and(|timeout| timeout.is_zero()) {
            return Err(SurrealStorageError::InvalidTimeout {
                setting: "engine_transaction_timeout",
            });
        }
        self.engine_query_timeout = query_timeout;
        self.engine_transaction_timeout = transaction_timeout;
        Ok(self)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn shutdown_wait(&self) -> Duration {
        self.shutdown_wait
    }

    pub fn drain_grace(&self) -> Duration {
        self.drain_grace
    }

    pub fn statement_timeout(&self) -> Duration {
        self.statement_timeout
    }

    pub fn engine_query_timeout(&self) -> Option<Duration> {
        self.engine_query_timeout
    }

    pub fn engine_transaction_timeout(&self) -> Option<Duration> {
        self.engine_transaction_timeout
    }

    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    pub fn database(&self) -> &str {
        &self.database
    }
}

#[derive(Clone)]
pub struct SurrealStorage {
    inner: Arc<SurrealStorageInner>,
}

impl std::fmt::Debug for SurrealStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SurrealStorage")
            .field("config", &self.inner.config)
            .finish_non_exhaustive()
    }
}

pub type SurrealOperation<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, SurrealStorageError>> + Send + 'a>>;
pub(crate) type SurrealStorageOperation<'a, T> =
    Pin<Box<dyn Future<Output = StorageResult<T>> + Send + 'a>>;
pub(crate) type SurrealTransactionOperation<'a, T, E> =
    Pin<Box<dyn Future<Output = Result<T, E>> + Send + 'a>>;

tokio::task_local! {
    pub(crate) static RESOURCE_ACCESS_AUTHORIZATION: crate::swarm_orchestration::resource_scope::ResourceAccessGrantAssociation;
    static INSIDE_SURREAL_OPERATION: ();
    static RECORD_USER_SCOPE: resource_authority::RecordUserScope;
    static PROTECTED_TRANSACTION: ProtectedTransaction;
    static MODEL_LANE_OPERATION_PURPOSE: model_lane::ContextCoordinationPurpose;
    static MODEL_LANE_OPERATION_SUBJECT: ModelLaneOperationSubject;
    static MODEL_LANE_FENCES: Vec<model_lane::ContextFenceProof>;
    static MODEL_LANE_CURRENT_GUARDS: Arc<StdMutex<Vec<model_lane::ContextCoordinationGuard>>>;
    /// MT-142: deadline for one logical (possibly retried) storage operation.
    /// Set by [`SurrealStorage::with_operation_deadline`]; every data operation
    /// underneath clamps its own bound to the remaining budget so a retry loop
    /// cannot multiply `statement_timeout` by its attempt count.
    static OPERATION_DEADLINE: Instant;
}

pub(crate) fn current_record_user_scope() -> Option<resource_authority::RecordUserScope> {
    RECORD_USER_SCOPE.try_with(Clone::clone).ok()
}

pub(crate) struct ContextOperationCancellation {
    cancelled: AtomicBool,
    notification: tokio::sync::Notify,
}
impl ContextOperationCancellation {
    pub(crate) fn new() -> Arc<Self> {
        Arc::new(Self { cancelled: AtomicBool::new(false), notification: tokio::sync::Notify::new() })
    }
    pub(crate) fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
        self.notification.notify_waiters();
    }
    pub(crate) fn is_cancelled(&self) -> bool { self.cancelled.load(Ordering::Acquire) }
    async fn cancelled(&self) {
        loop {
            let notified = self.notification.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if self.is_cancelled() { return; }
            notified.await;
        }
    }
}

/// A sealed, lease-bound view for ordinary typed data operations.
///
/// This facade deliberately has no raw SurrealQL, `Clone`, `Deref`, or
/// raw-client accessor. Namespace/database switching and schema administration
/// are available only through the crate-private admin lease.
/// Its methods await database work to completion, so no SDK handle can escape
/// the lifecycle lease held by [`SurrealStorage::with_data_operation`].
pub struct SurrealDataContext<'a> {
    client: &'a SurrealClient,
    transaction: Option<&'a surrealdb::method::Transaction<Db>>,
}

#[derive(Clone)]
struct ProtectedTransaction {
    transaction: Arc<surrealdb::method::Transaction<Db>>,
    client: SurrealClient,
    storage: SurrealStorage,
    authority: resource_authority::RecordUserScope,
}


#[derive(Clone)]
enum TransactionInvocation {
    WatcherAdmission {
        scope: resource_authority::RecordUserScope,
        deadline: Instant,
        operation_id: uuid::Uuid,
        admission: Arc<WatcherAdmissionController>,
    },
    Existing,
    Authenticated {
        scope: resource_authority::RecordUserScope,
        deadline: Instant,
        operation_id: uuid::Uuid,
    },
}

// Reserved authority bindings come from the captured original operation, never
// from caller-supplied query values. Database predicates still validate the live
// canonical grant/session/resource; these bindings alone confer no permission.
#[derive(Clone)]
pub(crate) struct ModelLaneOperationSubject {
    pub(crate) kind: resource_authority::ResourceKind,
    pub(crate) external_id: String,
}

pub(crate) async fn with_model_lane_subject<T>(
    subject: ModelLaneOperationSubject,
    operation: impl Future<Output = T>,
) -> T {
    MODEL_LANE_OPERATION_SUBJECT.scope(subject, operation).await
}

pub(crate) fn current_model_lane_subject() -> Option<ModelLaneOperationSubject> {
    MODEL_LANE_OPERATION_SUBJECT.try_with(Clone::clone).ok()
}

pub(crate) async fn with_model_lane_operation<T>(
    purpose: model_lane::ContextCoordinationPurpose,
    operation: impl Future<Output = T>,
) -> T {
    MODEL_LANE_OPERATION_PURPOSE.scope(purpose, operation).await
}

// Detached callers preserve absence as well as the actual original authority.
#[derive(Clone)]
pub(crate) struct CapturedModelLaneOperation {
    account: Option<event_ledger::CapturedAccountAuthority>,
    subject: Option<ModelLaneOperationSubject>,
    purpose: Option<model_lane::ContextCoordinationPurpose>,
}

pub(crate) fn capture_model_lane_operation() -> CapturedModelLaneOperation {
    CapturedModelLaneOperation {
        account: event_ledger::capture_account_authority(),
        subject: current_model_lane_subject(),
        purpose: MODEL_LANE_OPERATION_PURPOSE.try_with(Clone::clone).ok(),
    }
}

pub(crate) async fn with_captured_model_lane_operation<T>(
    storage: &SurrealStorage,
    captured: CapturedModelLaneOperation,
    operation: impl Future<Output = T>,
) -> T {
    let CapturedModelLaneOperation { account, subject, purpose } = captured;
    let work = async move {
        let work = async move {
            if let Some(purpose) = purpose {
                with_model_lane_operation(purpose, operation).await
            } else { operation.await }
        };
        if let Some(subject) = subject {
            with_model_lane_subject(subject, work).await
        } else { work.await }
    };
    event_ledger::with_captured_account_authority(storage, account, work).await
}

fn original_operation_bindings(mut bindings: surrealdb::types::Value) -> surrealdb::types::Value {
    if let Some(scope) = current_record_user_scope() {
        if matches!(bindings, surrealdb::types::Value::None) {
            bindings = surrealdb::types::Value::Object(surrealdb::types::Object::new());
        }
        if let surrealdb::types::Value::Object(object) = &mut bindings {
            object.insert("wp012_original_resource", scope.resource_id);
            object.insert("wp012_original_grant", scope.grant_id);
            object.insert("wp012_original_session", scope.session_id);
            object.insert("wp012_original_workspace", scope.workspace_id);
            object.insert("wp012_original_capability", scope.capability_id);
            object.insert("wp012_original_action", scope.action.as_str().to_owned());
            object.insert("wp012_original_purpose", MODEL_LANE_OPERATION_PURPOSE.try_with(|purpose| purpose.as_str().to_owned()).ok());
            let subject = MODEL_LANE_OPERATION_SUBJECT.try_with(Clone::clone).ok();
            object.insert("wp012_original_subject_kind", subject.as_ref().map(|subject| subject.kind.as_str().to_owned()));
            object.insert("wp012_original_subject_id", subject.map(|subject| subject.external_id));
            let proofs = MODEL_LANE_CURRENT_GUARDS.try_with(|guards| guards.lock().expect("coordination guard state poisoned").iter().map(model_lane::ContextCoordinationGuard::proof).collect::<Vec<_>>()).unwrap_or_else(|_| MODEL_LANE_FENCES.try_with(Clone::clone).unwrap_or_default());
            object.insert("wp012_fences", proofs.into_iter().map(model_lane::ContextFenceProof::into_binding).collect::<Vec<_>>());
        }
    }
    resource_authority::append_protected_role_bindings(&mut bindings);
    bindings
}

/// A borrowed query facade owned by the storage transaction controller.
pub(crate) struct SurrealTransactionContext<'a> {
    transaction: &'a surrealdb::method::Transaction<Db>,
}

impl SurrealTransactionContext<'_> {
    pub(crate) async fn query_first<R, B>(
        &self,
        statement: &'static str,
        bindings: B,
    ) -> Result<Option<R>, SurrealStorageError>
    where
        R: surrealdb::types::SurrealValue,
        B: surrealdb::types::SurrealValue + Send,
    {
        decode_first_value(self.query_values_at(statement, bindings, 0).await?)
    }

    pub(crate) async fn query_values_at<R, B>(
        &self,
        statement: &'static str,
        bindings: B,
        statement_index: usize,
    ) -> Result<Vec<R>, SurrealStorageError>
    where
        R: surrealdb::types::SurrealValue,
        B: surrealdb::types::SurrealValue + Send,
    {
        let bindings = original_operation_bindings(surrealdb::types::SurrealValue::into_value(bindings));
        let query = self.transaction.query(statement);
        let response = if matches!(bindings, surrealdb::types::Value::None) {
            query.await?
        } else {
            query.bind(bindings).await?
        };
        decode_query_values(response, statement_index)
    }
}

#[cfg(test)]
static_assertions::assert_not_impl_any!(SurrealTransactionContext<'static>:
    Clone,
    std::ops::Deref,
    AsRef<SurrealClient>,
    std::borrow::Borrow<SurrealClient>
);

// Preserve SDK extraction for every consumed result/error. Unconsumed slots are drained
// only when the response is dropped; captured correlation survives its parent scope.
pub(crate) enum ObservedQueryResults {
    Plain(surrealdb::IndexedResults),
    Timed(TimedQueryResults),
}
pub(crate) struct TimedQueryResults {
    response: surrealdb::method::WithStats<surrealdb::IndexedResults>,
    remaining: std::collections::BTreeSet<usize>,
    statement_count: usize,
    context: handshake_storage_support::diagnostics::StatementTimingContext,
}
impl TimedQueryResults {
    fn observed(&mut self, index: usize, duration: Option<std::time::Duration>) {
        if self.remaining.remove(&index) {
            self.context.emit(index, self.statement_count, duration);
        }
    }
}
impl Drop for TimedQueryResults {
    fn drop(&mut self) {
        for index in std::mem::take(&mut self.remaining) {
            let duration = self
                .response
                .take::<surrealdb::types::Value>(index)
                .and_then(|(stats, _)| stats.execution_time);
            self.context.emit(index, self.statement_count, duration);
        }
    }
}
impl From<surrealdb::IndexedResults> for ObservedQueryResults {
    fn from(value: surrealdb::IndexedResults) -> Self {
        Self::Plain(value)
    }
}
impl ObservedQueryResults {
    pub(crate) fn take_errors(&mut self) -> std::collections::HashMap<usize, surrealdb::Error> {
        match self {
            Self::Plain(response) => response.take_errors(),
            Self::Timed(response) => response
                .response
                .take_errors()
                .into_iter()
                .map(|(index, (stats, error))| {
                    response.observed(index, stats.execution_time);
                    (index, error)
                })
                .collect(),
        }
    }
    pub(crate) fn take<R: surrealdb::types::SurrealValue>(
        &mut self,
        index: usize,
    ) -> Result<Vec<R>, surrealdb::Error> {
        match self {
            Self::Plain(response) => response.take(index),
            Self::Timed(response) => match response.response.take::<Vec<R>>(index) {
                Some((stats, result)) => {
                    response.observed(index, stats.execution_time);
                    result
                }
                None => {
                    response.observed(index, None);
                    // WithStats::take returns before extraction if no stats/slot exists.
                    response.response.0.take(index)
                }
            },
        }
    }
}

pub(crate) async fn query_with_document_stats(
    query: surrealdb::method::Query<'_, surrealdb::engine::local::Db>,
) -> Result<ObservedQueryResults, surrealdb::Error> {
    use handshake_storage_support::diagnostics::StatementTimingContext;
    let Some(context) = StatementTimingContext::capture() else {
        return query.await.map(ObservedQueryResults::Plain);
    };
    let response = query.with_stats().await?;
    // These document boundaries issue ordinary SurrealQL statements, never LIVE/KILL.
    let statement_count = response.num_statements();
    Ok(ObservedQueryResults::Timed(TimedQueryResults {
        response,
        remaining: (0..statement_count).collect(),
        statement_count,
        context,
    }))
}

fn check_query_response(
    response: &mut ObservedQueryResults,
) -> Result<(), SurrealStorageError> {
    let mut errors = response.take_errors().into_iter().collect::<Vec<_>>();
    errors.sort_by_key(|(statement_index, _)| *statement_index);
    if !errors.is_empty() {
        let meaningful = errors
            .iter()
            .position(|(_, error)| {
                !error
                    .to_string()
                    .to_ascii_lowercase()
                    .contains("query was not executed due to a failed transaction")
            })
            .unwrap_or(0);
        return Err(errors.swap_remove(meaningful).1.into());
    }
    Ok(())
}

fn decode_query_values<R: surrealdb::types::SurrealValue>(
    response: impl Into<ObservedQueryResults>,
    index: usize,
) -> Result<Vec<R>, SurrealStorageError> {
    let mut response = response.into();
    check_query_response(&mut response)?;
    Ok(response.take(index)?)
}

fn decode_first_value<R: surrealdb::types::SurrealValue>(
    values: Vec<surrealdb::types::Value>,
) -> Result<Option<R>, SurrealStorageError> {
    if matches!(values.as_slice(), [surrealdb::types::Value::None]) {
        return Ok(None);
    }
    // Preserve validation of every returned row, even though this API returns
    // only the first. Only the explicit singleton NONE denotes no result.
    let rows = values
        .into_iter()
        .map(|value| {
            R::from_value(value).map_err(|error| surrealdb::Error::internal(error.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows.into_iter().next())
}

#[cfg(test)]
mod first_value_tests {
    use super::decode_first_value;
    use surrealdb::types::Value;

    #[test]
    fn empty_and_singleton_none_are_absent() {
        assert_eq!(decode_first_value::<bool>(vec![]).unwrap(), None);
        assert_eq!(decode_first_value::<bool>(vec![Value::None]).unwrap(), None);
    }

    #[test]
    fn valid_rows_return_first_without_skipping_later_validation() {
        assert_eq!(
            decode_first_value::<bool>(vec![Value::Bool(true), Value::Bool(false)]).unwrap(),
            Some(true)
        );
        assert!(decode_first_value::<bool>(vec![Value::Bool(true), Value::None]).is_err());
        assert!(decode_first_value::<bool>(vec![
            Value::Bool(true),
            Value::String("malformed".into())
        ])
        .is_err());
    }

    #[test]
    fn null_is_not_silently_absent() {
        assert!(decode_first_value::<bool>(vec![Value::Null]).is_err());
    }
}

impl SurrealDataContext<'_> {
    pub(crate) async fn create_one<R, D>(
        &self,
        table: &str,
        id: &str,
        content: D,
    ) -> Result<Option<R>, SurrealStorageError>
    where
        R: surrealdb::types::SurrealValue,
        D: surrealdb::types::SurrealValue,
    {
        Ok(match self.transaction { Some(transaction) => transaction.create((table, id)).content(content).await?, None => self.client.create((table, id)).content(content).await? })
    }

    pub(crate) async fn upsert_one<R, D>(
        &self,
        table: &str,
        id: &str,
        content: D,
    ) -> Result<Option<R>, SurrealStorageError>
    where
        R: surrealdb::types::SurrealValue,
        D: surrealdb::types::SurrealValue,
    {
        Ok(match self.transaction { Some(transaction) => transaction.upsert((table, id)).content(content).await?, None => self.client.upsert((table, id)).content(content).await? })
    }

    pub async fn select_one<R>(
        &self,
        table: &str,
        id: &str,
    ) -> Result<Option<R>, SurrealStorageError>
    where
        R: surrealdb::types::SurrealValue,
    {
        Ok(match self.transaction { Some(transaction) => transaction.select((table, id)).await?, None => self.client.select((table, id)).await? })
    }

    /// Every row of `table`.
    pub(crate) async fn select_all<R>(&self, table: &str) -> Result<Vec<R>, SurrealStorageError>
    where
        R: surrealdb::types::SurrealValue,
    {
        Ok(match self.transaction { Some(transaction) => transaction.select(table).await?, None => self.client.select(table).await? })
    }

    /// Deletes one record, returning it when it existed.
    pub(crate) async fn delete_one<R>(
        &self,
        table: &str,
        id: &str,
    ) -> Result<Option<R>, SurrealStorageError>
    where
        R: surrealdb::types::SurrealValue,
    {
        Ok(match self.transaction { Some(transaction) => transaction.delete((table, id)).await?, None => self.client.delete((table, id)).await? })
    }

    /// Runs a parameterized SurrealQL statement and returns the rows of the
    /// FIRST result set.
    ///
    /// Caller values travel as BINDINGS and are never concatenated into the
    /// statement text, so this widens what the facade can express without
    /// giving callers a way to build an injectable query or to reach the SDK
    /// handle. The statement stays a `&'static str` for the same reason: a
    /// runtime-assembled query string cannot be passed in.
    ///
    /// Multiple statements separated by `;` execute in one round trip, so a
    /// caller that needs several statements to be atomic writes them as
    /// `BEGIN TRANSACTION; ...; COMMIT TRANSACTION;` here rather than issuing
    /// them separately.
    pub async fn query_values<R, B>(
        &self,
        statement: &'static str,
        bindings: B,
    ) -> Result<Vec<R>, SurrealStorageError>
    where
        R: surrealdb::types::SurrealValue,
        B: surrealdb::types::SurrealValue + Send,
    {
        self.query_values_at(statement, bindings, 0).await
    }

    /// Runs a parameterized multi-statement query and decodes one explicit
    /// result set. Statement indexes count every semicolon-terminated
    /// statement, including transaction delimiters. This keeps transaction
    /// users behind the lease-bound facade instead of exposing the SDK client.
    pub(crate) async fn query_values_at<R, B>(
        &self,
        statement: &'static str,
        bindings: B,
        index: usize,
    ) -> Result<Vec<R>, SurrealStorageError>
    where
        R: surrealdb::types::SurrealValue,
        B: surrealdb::types::SurrealValue + Send,
    {
        let bindings = original_operation_bindings(surrealdb::types::SurrealValue::into_value(bindings));
        let query = match self.transaction { Some(transaction) => transaction.query(statement), None => self.client.query(statement) };
        let response = if matches!(bindings, surrealdb::types::Value::None) {
            observe_result("storage_query", query_with_document_stats(query)).await?
        } else {
            observe_result(
                "storage_query",
                query_with_document_stats(query.bind(bindings)),
            )
            .await?
        };
        observe_result("storage_response_decode", async {
            decode_query_values(response, index)
        })
        .await
    }

    /// Runs a bound effect-only query, checking every statement before discarding
    /// its observed response inside the current data-operation lease.
    pub(crate) async fn query_bound<B>(
        &self,
        statement: &'static str,
        bindings: B,
    ) -> Result<(), SurrealStorageError>
    where
        B: surrealdb::types::SurrealValue + Send,
    {
        let bindings = original_operation_bindings(surrealdb::types::SurrealValue::into_value(bindings));
        let query = match self.transaction { Some(transaction) => transaction.query(statement), None => self.client.query(statement) };
        let response = if matches!(bindings, surrealdb::types::Value::None) {
            observe_result("storage_query", query_with_document_stats(query)).await?
        } else {
            observe_result(
                "storage_query",
                query_with_document_stats(query.bind(bindings)),
            )
            .await?
        };
        observe_result("storage_response_decode", async {
            let mut response = response;
            let result = check_query_response(&mut response);
            drop(response);
            result
        })
        .await
    }

    /// Runs one bound multi-statement query and decodes five result sets from
    /// that same response. This preserves one explicit transaction snapshot
    /// for same-shaped source queries; calling [`Self::query_values_at`] five
    /// times would rerun the transaction and mix snapshots.
    pub(crate) async fn query_five_values_at<R, B>(
        &self,
        statement: &'static str,
        bindings: B,
        indexes: [usize; 5],
    ) -> Result<[Vec<R>; 5], SurrealStorageError>
    where
        R: surrealdb::types::SurrealValue,
        B: surrealdb::types::SurrealValue + Send,
    {
        let bindings = original_operation_bindings(surrealdb::types::SurrealValue::into_value(bindings));
        let query = match self.transaction { Some(transaction) => transaction.query(statement), None => self.client.query(statement) };
        let mut response = if matches!(bindings, surrealdb::types::Value::None) {
            query.await?
        } else {
            query.bind(bindings).await?
        };
        let mut errors = response.take_errors().into_iter().collect::<Vec<_>>();
        errors.sort_by_key(|(statement_index, _)| *statement_index);
        if !errors.is_empty() {
            let meaningful = errors
                .iter()
                .position(|(_, error)| {
                    !error
                        .to_string()
                        .to_ascii_lowercase()
                        .contains("query was not executed due to a failed transaction")
                })
                .unwrap_or(0);
            return Err(errors.swap_remove(meaningful).1.into());
        }
        let first: Vec<R> = response.take(indexes[0])?;
        let second: Vec<R> = response.take(indexes[1])?;
        let third: Vec<R> = response.take(indexes[2])?;
        let fourth: Vec<R> = response.take(indexes[3])?;
        let fifth: Vec<R> = response.take(indexes[4])?;
        Ok([first, second, third, fourth, fifth])
    }

    /// [`Self::query_values`] returning only the first row.
    pub(crate) async fn query_first<R, B>(
        &self,
        statement: &'static str,
        bindings: B,
    ) -> Result<Option<R>, SurrealStorageError>
    where
        R: surrealdb::types::SurrealValue,
        B: surrealdb::types::SurrealValue + Send,
    {
        decode_first_value(self.query_values(statement, bindings).await?)
    }

    /// Runs a parameterized statement for its effect and reports how many rows
    /// the first result set returned.
    ///
    /// Statements whose affected-row count matters must end in `RETURN AFTER`
    /// (or `RETURN BEFORE`), because SurrealDB reports affected rows by
    /// returning them. A conditional update that matched nothing therefore
    /// yields `0`, which is the signal callers use to detect a lost race.
    pub(crate) async fn execute_returning<B>(
        &self,
        statement: &'static str,
        bindings: B,
    ) -> Result<usize, SurrealStorageError>
    where
        B: surrealdb::types::SurrealValue + Send,
    {
        let rows = self
            .query_values::<surrealdb::types::Value, B>(statement, bindings)
            .await?;
        Ok(rows.len())
    }

    /// Creates a record only when its id is free, returning `Ok(None)` when it
    /// is already taken.
    ///
    /// This is the fail-closed counterpart of [`Self::upsert_one`] and gives
    /// idempotency keys, mailbox lease acquisition and duplicate-outcome
    /// detection the insert-if-absent primitive they need without any
    /// unique-violation error-code handling. The existence test and the create
    /// run inside ONE statement, so two callers racing for the same id cannot
    /// both observe it as free.
    pub(crate) async fn create_if_absent<R, D>(
        &self,
        table: &'static str,
        id: &str,
        content: D,
    ) -> Result<Option<R>, SurrealStorageError>
    where
        R: surrealdb::types::SurrealValue,
        D: surrealdb::types::SurrealValue + Send,
    {
        // The derive expands to unqualified `SurrealValue` references, so the
        // trait has to be in scope here rather than named by full path. The
        // content is converted to a `Value` before binding so this struct stays
        // non-generic: a generic field would need its own `SurrealValue` bound
        // threaded through the derive.
        use surrealdb::types::{SurrealValue, Value};

        #[derive(SurrealValue)]
        struct CreateIfAbsentBindings {
            tb: String,
            id: String,
            content: Value,
        }

        let rows = self
            .query_values_at::<R, _>(
                "LET $record = type::record($tb, $id); \
                 IF (SELECT VALUE id FROM $record)[0] = NONE \
                 { RETURN CREATE $record CONTENT $content; } \
                 ELSE { RETURN []; };",
                CreateIfAbsentBindings {
                    tb: table.to_owned(),
                    id: id.to_owned(),
                    content: content.into_value(),
                },
                1,
            )
            .await?;
        Ok(rows.into_iter().next())
    }
}

struct SurrealAdminContext<'a> {
    client: &'a SurrealClient,
    config: SurrealStorageConfig,
}

impl SurrealAdminContext<'_> {
    async fn query(
        &self,
        statement: impl Into<String> + Send,
    ) -> Result<surrealdb::IndexedResults, SurrealStorageError> {
        #[cfg(not(test))]
        {
            Ok(self.client.query(statement.into()).await?.check()?)
        }
        #[cfg(test)]
        {
            let mut response = self.client.query(statement.into()).await?;
            let mut errors = response.take_errors().into_iter().collect::<Vec<_>>();
            errors.sort_by_key(|(index, _)| *index);
            let first_error = errors.first().map(|(_, error)| error.clone());
            let masked = errors.iter().filter(|(_, error)| matches!(
                error.query_details(), Some(surrealdb::types::QueryError::NotExecuted | surrealdb::types::QueryError::Cancelled)
            )).count();
            if !errors.is_empty() {
                eprintln!("SURREAL_ADMIN_STATEMENT_ERROR_COUNTS total={} masked={masked}", errors.len());
                for (index, error) in errors.iter().filter(|(_, error)| !matches!(
                    error.query_details(), Some(surrealdb::types::QueryError::NotExecuted | surrealdb::types::QueryError::Cancelled)
                )) {
                    eprintln!("SURREAL_ADMIN_STATEMENT_ERROR index={index} error={error:?}");
                }
                if masked == errors.len() {
                    if let Some((index, error)) = errors.first() {
                        eprintln!("SURREAL_ADMIN_FIRST_MASKED_ERROR index={index} error={error:?}");
                    }
                    if let Some((index, error)) = errors.last() {
                        eprintln!("SURREAL_ADMIN_LAST_MASKED_ERROR index={index} error={error:?}");
                    }
                }
            }
            // take_errors removes failures; preserve check's original first error.
            if let Some(error) = first_error { return Err(error.into()); }
            Ok(response.check()?)
        }
    }

    async fn query_bound<B>(
        &self,
        statement: impl Into<String> + Send,
        bindings: B,
    ) -> Result<surrealdb::IndexedResults, SurrealStorageError>
    where
        B: surrealdb::types::SurrealValue + Send,
    {
        #[cfg(not(test))]
        {
            Ok(self
                .client
                .query(statement.into())
                .bind(surrealdb::types::SurrealValue::into_value(bindings))
                .await?
                .check()?)
        }
        #[cfg(test)]
        {
            let mut response = self.client
                .query(statement.into())
                .bind(surrealdb::types::SurrealValue::into_value(bindings))
                .await?;
            let mut errors = response.take_errors().into_iter().collect::<Vec<_>>();
            errors.sort_by_key(|(index, _)| *index);
            let first_error = errors.first().map(|(_, error)| error.clone());
            let masked = errors.iter().filter(|(_, error)| matches!(
                error.query_details(), Some(surrealdb::types::QueryError::NotExecuted | surrealdb::types::QueryError::Cancelled)
            )).count();
            if !errors.is_empty() {
                eprintln!("SURREAL_ADMIN_STATEMENT_ERROR_COUNTS total={} masked={masked}", errors.len());
                for (index, error) in errors.iter().filter(|(_, error)| !matches!(
                    error.query_details(), Some(surrealdb::types::QueryError::NotExecuted | surrealdb::types::QueryError::Cancelled)
                )) {
                    eprintln!("SURREAL_ADMIN_STATEMENT_ERROR index={index} error={error:?}");
                }
                if masked == errors.len() {
                    if let Some((index, error)) = errors.first() {
                        eprintln!("SURREAL_ADMIN_FIRST_MASKED_ERROR index={index} error={error:?}");
                    }
                    if let Some((index, error)) = errors.last() {
                        eprintln!("SURREAL_ADMIN_LAST_MASKED_ERROR index={index} error={error:?}");
                    }
                }
            }
            // take_errors removes failures; preserve check's original first error.
            if let Some(error) = first_error { return Err(error.into()); }
            Ok(response.check()?)
        }
    }
}

type SharedShutdownResult = Result<(), Arc<str>>;

enum ShutdownCoordinatorState {
    Open,
    Closing {
        receiver: watch::Receiver<Option<SharedShutdownResult>>,
    },
    Closed,
    Failed {
        error: Arc<str>,
    },
}

enum ShutdownAttemptError {
    /// The sole client is still owned by the storage and another shutdown call
    /// may safely retry the flush barrier.
    Retryable(SurrealStorageError),
    /// MT-142 AC-142-7: the drain did not complete inside `shutdown_wait`
    /// after the cancellation token fired. The sole client is still owned by
    /// the storage, admission stays stopped, cooperative work stays cancelled,
    /// and a later shutdown call retries the close.
    DrainTimedOut(SurrealStorageError),
    /// The sole client was taken and dropped. Operations must remain closed even
    /// though the platform release proof could not be completed.
    Terminal(SurrealStorageError),
}

/// Outcome of one embedded-store shutdown (MT-142 AC-142-7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShutdownReport {
    /// Every in-flight lease drained within `drain_grace`.
    pub drained: bool,
    /// The grace expired, so the cooperative cancellation token was fired
    /// before the remaining engine-bound leases were awaited.
    pub cancelled: bool,
    /// Wall time from the close attempt start to engine release.
    pub elapsed: Duration,
}

tokio::task_local! {
    static AUTHORITY_MUTATION_SUBMITTED: Arc<AtomicU8>;
}

pub(crate) fn authority_mutation_submitted() {
    let _ = AUTHORITY_MUTATION_SUBMITTED.try_with(|submitted| submitted.store(1, Ordering::Release));
}

pub(crate) fn authority_mutation_settled() {
    let _ = AUTHORITY_MUTATION_SUBMITTED.try_with(|submitted| submitted.store(2, Ordering::Release));
}

pub(crate) fn authority_mutation_error(error: surrealdb::Error) -> surrealdb::Error {
    // These engine responses establish rejection of an atomic statement/transaction.
    // A masked NotExecuted, timeout, or transport error does not establish settlement.
    if error.is_thrown() || error.not_allowed_details().is_some()
        || matches!(error.query_details(), Some(surrealdb::types::QueryError::TransactionConflict))
    {
        authority_mutation_settled();
    }
    error
}

struct ActivationTransactionRetention {
    _authority: resource_authority::CapturedActivationAuthority,
    _fence: Arc<OwnedRwLockReadGuard<()>>,
}

enum AuthenticatedTransactionRetention {
    AdministratorPasswordLogin(Arc<resource_authority::AdministratorPasswordLoginOperation>),
    AdministratorSessionExchange(Arc<resource_authority::AdministratorSessionExchangeOperation>),
    FirstOwnerSetup(Arc<local_accounts::FirstOwnerSetupOperation>),
    FirstOwnerConfirmation(Arc<local_accounts::FirstOwnerAdministrationConfirmationOperation>),
    FirstOwnerWorkspace(Arc<workspaces::FirstOwnerWorkspaceCreationOperation>),
    FirstOwnerWorkspaceRecovery(Arc<workspaces::FirstOwnerWorkspaceCreationWitness>),
    MemberPasswordLogin(Arc<resource_authority::MemberPasswordLoginOperation>),
    MemberSessionExchange(Arc<resource_authority::MemberSessionExchangeOperation>),
    OriginalRunCreate(Arc<StdMutex<OriginalRunCreateRetention>>),
    WorkspaceArtifactPublication {
        original: Arc<WorkspaceArtifactPublicationOperationContext>,
        owner: Arc<dyn std::any::Any + Send + Sync>,
        resources: Arc<crate::storage::artifacts::ArtifactPublicationResources>,
        completed_result: Option<Box<dyn std::any::Any + Send>>,
        completed_error: Option<Box<dyn std::any::Any + Send>>,
        worker_finished: bool,
        callback_succeeded: bool,
        canonical_terminal: Option<Arc<ArtifactPublicationCanonicalTerminalForOwner>>,
        owned_closure: Option<Arc<crate::storage::artifacts::ArtifactPublicationOwnedClosure>>,
        owned_finalization: Option<WorkspaceArtifactEngineFinalization>,
        reconciliation_read: Option<Arc<WorkspaceArtifactPublicationReconciliationRead>>,
        canonical_read_client: Option<Arc<SurrealClient>>,
        settlement: ActualSettlement,
    },
    AlbumMembership(AlbumMembershipTransactionRetention),
    Activation(ActivationTransactionRetention),
    JobAdmission {
        retention: resource_authority::JobAdmissionTransactionRetention,
        phase: JobAdmissionTransactionPhase,
        readback: Option<Arc<resource_authority::JobAdmissionReadbackTransaction>>,
    },
    ValidatorAssociation(resource_authority::ValidatorAssociationTransactionRetention),
    Role {
        retention: resource_authority::RoleAuthorityTransactionRetention,
        phase: RoleTransactionPhase,
    },
}

enum RoleTransactionPhase {
    CaptureReadPending,
    CaptureReadObserved,
    PreEnginePrepared,
    EnginePending,
    CommitObservedPublicationPending,
}

enum JobAdmissionTransactionPhase {
    EnginePending,
    CommitObservedReadbackPending,
    ReadbackReadPending,
    ReadbackObserved,
}

#[derive(Clone, Copy)]
enum ActualSettlement {
    WorkspaceArtifactCanonicalReadRollbackObserved,
    Unobserved,
    CommitObserved,
    CancelNoEffectObserved,
    CallbackPanicRollbackObserved,
    CaptureReadRollbackObserved,
    JobReadbackRollbackObserved,
}

#[derive(Clone)]
enum TransactionSettlementMode {
    WorkspaceArtifactCanonicalRead(Arc<SurrealClient>),
    Mutation,
    RoutingCapture(resource_authority::RoleAuthorityTransactionRetention),
    JobReadback(Arc<resource_authority::JobAdmissionReadbackTransaction>),
}

struct SurrealStorageInner {
    watcher_admission_credentials: Option<Arc<WatcherAdmissionCredentials>>,
    watcher_admission_configured: AtomicBool,
    watcher_admission_bootstrap_failure: StdMutex<Option<WatcherAdmissionBootstrapFailure>>,
    watcher_admissions: StdMutex<BTreeMap<uuid::Uuid, Arc<WatcherAdmissionController>>>,
    resource_access_lifecycle: crate::swarm_orchestration::resource_scope::ResourceAccessLifecycleRegistry,
    config: SurrealStorageConfig,
    client: RwLock<Option<SurrealClient>>,
    guard: Arc<dyn StorageGuard>,
    lifecycle: AtomicU8,
    shutdown: Mutex<ShutdownCoordinatorState>,
    /// Cooperative cancellation for retry loops and keyed-lock waits; child
    /// tokens are handed out by [`SurrealStorage::cancellation_token`].
    cancel: StdMutex<CancellationToken>,
    shutdown_report: StdMutex<Option<ShutdownReport>>,
    authority_publication_gate: Arc<RwLock<()>>,
    pending_authority_mutations: StdMutex<BTreeMap<uuid::Uuid, OwnedRwLockWriteGuard<()>>>,
    authority_pending_changed: watch::Sender<Option<uuid::Uuid>>,
    transaction_settlements: StdMutex<BTreeMap<uuid::Uuid, Option<AuthenticatedTransactionRetention>>>,
    member_login_admission: StdMutex<bool>,
    pending_context_guards: StdMutex<BTreeMap<uuid::Uuid, Vec<model_lane::ContextCoordinationGuard>>>,
    pending_context_acquisitions: StdMutex<BTreeMap<uuid::Uuid, model_lane::PendingContextAcquisition>>,
    /// Leases held right now and the high-water mark since the last reset:
    /// the engine-window concurrency gauge (review R2-1-3) that a hidden
    /// global mutex cannot inflate, unlike a call-boundary counter.
    leases_in_flight: AtomicUsize,
    lease_high_water: AtomicUsize,
}

/// Decrements the in-flight lease count on every exit path, including drops of
/// a cancelled or timed-out operation future.
struct LeaseCount<'a>(&'a AtomicUsize);

impl Drop for LeaseCount<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

impl SurrealStorageInner {
    fn enter_lease(&self) -> LeaseCount<'_> {
        let current = self.leases_in_flight.fetch_add(1, Ordering::AcqRel) + 1;
        self.lease_high_water.fetch_max(current, Ordering::AcqRel);
        LeaseCount(&self.leases_in_flight)
    }
    fn cancel_operations(&self) {
        self.cancel
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .cancel();
    }

    /// Installs a fresh token after a retryable shutdown failure reopened
    /// admission, so later operations are not born cancelled.
    fn reset_cancellation(&self) {
        *self
            .cancel
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = CancellationToken::new();
    }

    fn child_cancellation_token(&self) -> CancellationToken {
        self.cancel
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .child_token()
    }

    fn record_shutdown_report(&self, report: ShutdownReport) {
        *self
            .shutdown_report
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(report);
    }

    fn shutdown_report(&self) -> Option<ShutdownReport> {
        *self
            .shutdown_report
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl SurrealStorage {
    fn capture_workspace_artifact_publication_operation_context(
        &self,
        operation_id: uuid::Uuid,
        scope: resource_authority::RecordUserScope,
        authorization_expires_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<Arc<WorkspaceArtifactPublicationOperationContext>, SurrealStorageError> {
        let original_monotonic = Instant::now();
        let original_utc = chrono::Utc::now();
        let budget = self.effective_statement_timeout();
        let invalid = || SurrealStorageError::InvalidModelLaneRecord {
            reason: "workspace artifact publication context unavailable",
        };
        let remaining = authorization_expires_at.signed_duration_since(original_utc)
            .to_std().map_err(|_| invalid())?;
        let limit = budget.min(remaining);
        if limit.is_zero() { return Err(invalid()); }
        let deadline = original_monotonic.checked_add(limit).ok_or_else(invalid)?;
        let delta = chrono::Duration::from_std(limit).map_err(|_| invalid())?;
        let expires_at = original_utc.checked_add_signed(delta).ok_or_else(invalid)?;
        Ok(Arc::new(WorkspaceArtifactPublicationOperationContext {
            original_operation_id: operation_id,
            storage: Arc::downgrade(&self.inner),
            namespace: self.config().namespace().to_owned(),
            database: self.config().database().to_owned(),
            scope, original_monotonic, original_utc,
            deadline, expires_at, authority_expires_at: authorization_expires_at,
        }))
    }

    pub(crate) fn workspace_artifact_publication_operation_budget(&self) -> Duration {
        self.effective_statement_timeout()
    }

    pub(crate) async fn inspect_watcher_admission_read(
        &self,
        read: &Arc<resource_authority::PalmistryWatcherAdmissionReadAuthorization>,
        operation_id: uuid::Uuid,
        purpose: WatcherAdmissionPurpose,
    ) -> Result<WatcherAdmissionInspection, SurrealPalmistryError> {
        let authority = read.authority();
        let store = SurrealPalmistryStore::new_exact(self.clone(), read.exact_scope().clone());
        let inspection = self.with_current_watcher_admission_read(read,
            store.inspect_canonical_watcher_admission_read(authority, operation_id, purpose),
        ).await?;
        self.retain_watcher_admission_uncertainty(authority, operation_id, purpose, inspection)
    }

    pub(crate) async fn inspect_watcher_admission(
        &self,
        authority: &Arc<resource_authority::PalmistryWatcherAdoptionAuthority>,
        operation_id: uuid::Uuid,
        purpose: WatcherAdmissionPurpose,
    ) -> Result<WatcherAdmissionInspection, SurrealPalmistryError> {
        let read = self.authorize_retained_watcher_admission_read(authority).await?;
        self.inspect_watcher_admission_read(&read, operation_id, purpose).await
    }

    fn retain_watcher_admission_uncertainty(
        &self,
        authority: &Arc<resource_authority::PalmistryWatcherAdoptionAuthority>,
        operation_id: uuid::Uuid,
        purpose: WatcherAdmissionPurpose,
        inspection: WatcherAdmissionInspection,
    ) -> Result<WatcherAdmissionInspection, SurrealPalmistryError> {
        if operation_id.is_nil() || operation_id != authority.operation_id() {
            return Err(SurrealPalmistryError::PermissionDenied);
        }
        let registry = self.inner.watcher_admissions.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(controller) = registry.get(&operation_id) else { return Ok(inspection); };
        let retained = controller.prepared.authorization().authority();
        if retained.exact_scope() != authority.exact_scope()
            || retained.watcher_id() != authority.watcher_id()
            || controller.prepared.manifest().purpose() != purpose
        {
            return Err(SurrealPalmistryError::PermissionDenied);
        }
        Ok(match inspection {
            WatcherAdmissionInspection::NoPriorRelevantAdmission =>
                WatcherAdmissionInspection::ExistingReadOnly(PalmistryWatcherAdoptionResult::Uncertain {
                    operation_id, canonical_receipt: None,
                    diagnostic: "same-operation admission controller exists; canonical receipt unavailable".into(),
                }),
            existing => existing,
        })
    }
    pub(crate) async fn execute_watcher_admission(
        &self, prepared: Arc<PalmistryWatcherPreparedAdmission>,
    ) -> Result<Vec<PalmistryWatcherAdoptionReceipt>, SurrealPalmistryError> {
        prepared.validate_binding()?;
        if !self.inner.watcher_admission_configured.load(Ordering::Acquire)
            || self.inner.watcher_admission_credentials.as_ref()
                .is_none_or(|credentials| credentials.not_after < *prepared.deadline().expires_at())
        { return Err(SurrealPalmistryError::PermissionDenied); }
        prepared.authorization().revalidate(self).await
            .map_err(|_| SurrealPalmistryError::PermissionDenied)?;
        let (admission, is_new) = self.watcher_admission_controller(prepared.clone())?;
        if !is_new {
            return self.reconcile_existing_watcher_admission(admission).await;
        }
        admission.begin_issuance().await?;
        self.watcher_admission_transaction(admission).await
    }

    async fn reconcile_existing_watcher_admission(
        &self, admission: Arc<WatcherAdmissionController>,
    ) -> Result<Vec<PalmistryWatcherAdoptionReceipt>, SurrealPalmistryError> {
        let (installer, bindings) = {
            let mut state = admission.state.lock().await;
            state.reconciliation = Some(WatcherAdmissionReconciliation::Uncertain);
            state.stage = WatcherAdmissionStage::ReconciliationRead;
            let credentials = self.inner.watcher_admission_credentials.as_ref()
                .ok_or(SurrealPalmistryError::CorruptAuthority)?;
            let installer = state.installer_client.as_ref()
                .ok_or(SurrealPalmistryError::CorruptAuthority)?.clone();
            let identity = state.identity.as_ref().ok_or(SurrealPalmistryError::CorruptAuthority)?;
            let bindings = admission.prepared.permit_install_bindings(identity, &credentials.key_version)?;
            (installer, bindings)
        };
        let response = installer.query(palmistry::WATCHER_PERMIT_READ_SQL)
            .bind(surrealdb::types::SurrealValue::into_value(bindings)).await
            .map_err(|_| SurrealPalmistryError::CorruptAuthority)?;
        let permits = decode_query_values::<PalmistryWatcherInstalledPermit>(response, 0)
            .map_err(|_| SurrealPalmistryError::CorruptAuthority)?;
        if permits.len() != 1 { return Err(SurrealPalmistryError::CorruptAuthority); }
        permits[0].validate(&admission.prepared)?;
        admission.state.lock().await.reconciliation = Some(WatcherAdmissionReconciliation::PermitConfirmed);
        // A worker commit does not certify publication. The owning leaf reconciles
        // this untagged outcome without entering its publication callback again.
        Err(SurrealPalmistryError::CorruptAuthority)
    }

    async fn watcher_admission_transaction(
        &self, admission: Arc<WatcherAdmissionController>,
    ) -> Result<Vec<PalmistryWatcherAdoptionReceipt>, SurrealPalmistryError> {
        let prepared = admission.prepared.clone();
        let operation_id = prepared.operation_id();
        let invocation = TransactionInvocation::WatcherAdmission {
            scope: prepared.authorization().authority().record_user_scope(),
            deadline: prepared.deadline().monotonic(), operation_id,
            admission: admission.clone(),
        };
        let storage = self.clone();
        self.transaction_worker(invocation, None, None, TransactionSettlementMode::Mutation,
            move |tx| Box::pin(async move {
                let permit = {
                    let mut state = admission.state.lock().await;
                    if !matches!(state.phase, WatcherAdmissionControllerPhase::Installed) {
                        return Err(SurrealPalmistryError::CorruptAuthority);
                    }
                    let permit = state.installed_permit.clone()
                        .ok_or(SurrealPalmistryError::CorruptAuthority)?;
                    state.phase = WatcherAdmissionControllerPhase::Running;
                    state.stage = WatcherAdmissionStage::FixedTransaction;
                    permit
                };
                match palmistry::execute_prepared_watcher_admission_transaction(
                    &storage, tx, prepared, permit).await
                {
                    Ok(receipts) => {
                        admission.state.lock().await.callback_receipts = Some(receipts.clone());
                        Ok(receipts)
                    }
                    Err(error) => Err(SurrealPalmistryError::OriginalCallback {
                        operation_id, error: Box::new(error),
                    }),
                }
            })).await
    }

    async fn install_watcher_admission(
        &self, original: &SurrealClient, installer: SurrealClient,
        scope: &resource_authority::RecordUserScope,
        admission: &Arc<WatcherAdmissionController>,
    ) -> Result<(), SurrealStorageError> {
        let prepared = &admission.prepared;
        prepared.validate_binding().map_err(|_| watcher_admission_controller_error())?;
        let credentials = self.inner.watcher_admission_credentials.as_ref()
            .ok_or_else(watcher_admission_controller_error)?;
        if !self.inner.watcher_admission_configured.load(Ordering::Acquire)
            || credentials.not_after < *prepared.deadline().expires_at()
            || Instant::now() >= prepared.deadline().monotonic()
        { return Err(watcher_admission_controller_error()); }
        {
            let mut state = admission.state.lock().await;
            if !matches!(state.phase, WatcherAdmissionControllerPhase::Issuing) {
                return Err(watcher_admission_controller_error());
            }
            state.installer_client = Some(installer.clone());
        }
        admission.state.lock().await.stage = WatcherAdmissionStage::AdmissionAuthentication;
        original.signin(RecordSignin {
            namespace: self.config().namespace().to_owned(),
            database: self.config().database().to_owned(),
            access: "palmistry_watcher_admission".to_owned(),
            params: watcher_admission_signin_params(prepared, scope, credentials)?,
        }).await.map_err(|_| watcher_admission_controller_error())?;
        admission.state.lock().await.stage = WatcherAdmissionStage::ProtectedIdentity;
        let response = original.query(palmistry::WATCHER_ADMISSION_IDENTITY_SQL)
            .await.map_err(|_| watcher_admission_controller_error())?;
        let identities = decode_query_values::<PalmistryWatcherAdmissionIdentity>(response, 0)
            .map_err(|_| watcher_admission_controller_error())?;
        if identities.len() != 1 { return Err(watcher_admission_controller_error()); }
        let identity = identities.into_iter().next().ok_or_else(watcher_admission_controller_error)?;
        identity.validate(prepared).map_err(|_| watcher_admission_controller_error())?;
        let install_bindings = prepared.permit_install_bindings(&identity, &credentials.key_version)
            .map_err(|_| watcher_admission_controller_error())?;
        let read_bindings = prepared.permit_install_bindings(&identity, &credentials.key_version)
            .map_err(|_| watcher_admission_controller_error())?;
        admission.state.lock().await.identity = Some(identity);
        admission.state.lock().await.stage = WatcherAdmissionStage::InstallerAuthentication;
        installer.signin(RecordSignin {
            namespace: self.config().namespace().to_owned(),
            database: self.config().database().to_owned(),
            access: "palmistry_watcher_permit_installer".to_owned(),
            params: watcher_admission_signin_params(prepared, scope, credentials)?,
        }).await.map_err(|_| watcher_admission_controller_error())?;
        admission.state.lock().await.stage = WatcherAdmissionStage::PermitInstall;
        let response = installer.query(palmistry::WATCHER_PERMIT_INSTALL_SQL)
            .bind(surrealdb::types::SurrealValue::into_value(install_bindings))
            .await.map_err(|_| watcher_admission_controller_error())?;
        let installed = decode_query_values::<PalmistryWatcherInstalledPermit>(response, 1)
            .map_err(|_| watcher_admission_controller_error())?;
        if installed.len() != 1 { return Err(watcher_admission_controller_error()); }
        installed[0].validate(prepared).map_err(|_| watcher_admission_controller_error())?;
        admission.state.lock().await.stage = WatcherAdmissionStage::PermitRead;
        let response = installer.query(palmistry::WATCHER_PERMIT_READ_SQL)
            .bind(surrealdb::types::SurrealValue::into_value(read_bindings))
            .await.map_err(|_| watcher_admission_controller_error())?;
        let permits = decode_query_values::<PalmistryWatcherInstalledPermit>(response, 0)
            .map_err(|_| watcher_admission_controller_error())?;
        if permits.len() != 1 { return Err(watcher_admission_controller_error()); }
        let permit = permits.into_iter().next().ok_or_else(watcher_admission_controller_error)?;
        permit.validate(prepared).map_err(|_| watcher_admission_controller_error())?;
        if Instant::now() >= prepared.deadline().monotonic() {
            return Err(watcher_admission_controller_error());
        }
        let mut state = admission.state.lock().await;
        if !matches!(state.phase, WatcherAdmissionControllerPhase::Issuing) {
            return Err(watcher_admission_controller_error());
        }
        state.installed_permit = Some(permit);
        state.phase = WatcherAdmissionControllerPhase::Installed;
        Ok(())
    }

    fn watcher_admission_controller(&self, prepared: Arc<PalmistryWatcherPreparedAdmission>)
        -> Result<(Arc<WatcherAdmissionController>, bool), SurrealPalmistryError>
    {
        prepared.validate_binding()?;
        let mut registry = self.inner.watcher_admissions.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match registry.entry(prepared.operation_id()) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                let controller = WatcherAdmissionController::new(prepared)?;
                entry.insert(controller.clone());
                Ok((controller, true))
            }
            std::collections::btree_map::Entry::Occupied(entry) => {
                let controller = entry.get();
                if controller.prepared.manifest().hash() != prepared.manifest().hash()
                    || controller.prepared.deadline().monotonic() != prepared.deadline().monotonic()
                    || controller.prepared.deadline().expires_at() != prepared.deadline().expires_at()
                    || !Arc::ptr_eq(controller.prepared.authorization(), prepared.authorization())
                {
                    return Err(SurrealPalmistryError::PermissionDenied);
                }
                Ok((controller.clone(), false))
            }
        }
    }

    pub(crate) fn watcher_admission_operation_budget(&self)
        -> Result<WatcherAdmissionBudget, SurrealPalmistryError>
    {
        let unavailable = || SurrealPalmistryError::PermissionDenied;
        let credentials = self.inner.watcher_admission_credentials.as_ref()
            .ok_or_else(unavailable)?;
        let limit = self.effective_statement_timeout().min(Duration::from_secs(300));
        if !self.inner.watcher_admission_configured.load(Ordering::Acquire)
            || limit.is_zero() || credentials.not_after <= chrono::Utc::now() {
            return Err(unavailable().into());
        }
        Ok(WatcherAdmissionBudget {
            limit, credential_not_after: credentials.not_after,
        })
    }

    pub(crate) async fn with_record_user_scope<T>(
        &self,
        scope: resource_authority::RecordUserScope,
        operation: impl Future<Output = T>,
    ) -> T {
        RECORD_USER_SCOPE.scope(scope, operation).await
    }

    /// Returns the feature-gated, read-only test inspection facade.
    ///
    /// The facade exposes only catalog-validated selectors and bound-value
    /// observations; it never exposes the underlying SDK client or raw query
    /// execution.
    #[cfg(feature = "surreal-test-support")]
    pub fn test_inspector(&self) -> SurrealTestInspector {
        SurrealTestInspector::new(self.clone())
    }

    /// Closed, catalog-validated mutation support for storage-boundary tests.
    #[cfg(feature = "surreal-test-support")]
    pub fn test_mutator(&self) -> SurrealTestMutator {
        SurrealTestMutator::new(self.clone())
    }

    pub async fn open(config: SurrealStorageConfig) -> Result<Self, SurrealStorageError> {
        Self::open_with_guard(config, Arc::new(DefaultStorageGuard)).await
    }

    pub async fn open_with_guard(
        config: SurrealStorageConfig, guard: Arc<dyn StorageGuard>,
    ) -> Result<Self, SurrealStorageError> {
        Self::open_with_context(config, guard, true).await
    }

    #[cfg(feature = "surreal-test-support")]
    async fn open_test_root_catalog(config: SurrealStorageConfig) -> Result<Self, SurrealStorageError> {
        if !valid_test_scope_name(config.namespace(), "hs_test_ns_") || !valid_test_scope_name(config.database(), "hs_test_db_") {
            return Err(SurrealStorageError::InvalidModelLaneRecord { reason: "test catalog scope must use strict generated identifiers" });
        }
        Self::open_with_context(config, Arc::new(DefaultStorageGuard), false).await
    }

    async fn open_with_context(
        mut config: SurrealStorageConfig, guard: Arc<dyn StorageGuard>, select_context: bool,
    ) -> Result<Self, SurrealStorageError> {
        validate_supported_storage_path(config.path())?;
        std::fs::create_dir_all(config.path()).map_err(|source| SurrealStorageError::Io {
            path: config.path().to_path_buf(),
            source,
        })?;
        config.path =
            dunce::canonicalize(config.path()).map_err(|source| SurrealStorageError::Io {
                path: config.path().to_path_buf(),
                source,
            })?;
        validate_supported_storage_path(config.path())?;

        // Engine-level deadlines (MT-142 AC-142-7). The pinned SDK forwards
        // `Config::query_timeout`/`transaction_timeout` to the datastore builder
        // (`surrealdb-3.2.0/src/engine/local/native.rs:131-133`, fields at
        // `surrealdb-3.2.0/src/opt/config.rs:16-17,53-61`); the executor wraps
        // every write transaction in `tokio::time::timeout` and cancels it on
        // expiry (`surrealdb-core-3.2.0/src/dbs/executor.rs:1034-1049`), and the
        // query timeout becomes the context deadline of every query
        // (`surrealdb-core-3.2.0/src/kvs/ds.rs:3951-3953`). The `(PathBuf, Config)`
        // endpoint form is `surrealdb-3.2.0/src/opt/endpoint/rocksdb.rs:27-36`.
        let engine_config = EngineConfig::new()
            .query_timeout(config.engine_query_timeout())
            .transaction_timeout(config.engine_transaction_timeout());
        let client = if config.test_datastore_sync_never() {
            // TEST-ONLY: the SDK keeps the query string (surrealdb-3.2.0 `opt/endpoint/mod.rs`
            // path_to_string) and the datastore maps `sync=never` to `datastore_sync`.
            let endpoint = format!("{}?sync=never", config.path().display());
            Surreal::new::<RocksDb>((endpoint, engine_config)).await?
        } else {
            Surreal::new::<RocksDb>((config.path().to_path_buf(), engine_config)).await?
        };
        if select_context {
        client
            .use_ns(config.namespace())
            .use_db(config.database())
            .await?;

        let mut context = client
            .query("RETURN session::ns(); RETURN session::db();")
            .await?
            .check()?;
        let actual_namespace: Option<String> = context.take(0)?;
        let actual_database: Option<String> = context.take(1)?;
        let actual_namespace = actual_namespace.unwrap_or_default();
        let actual_database = actual_database.unwrap_or_default();
        if actual_namespace != config.namespace() || actual_database != config.database() {
            return Err(SurrealStorageError::ContextMismatch {
                expected_namespace: config.namespace().to_owned(),
                expected_database: config.database().to_owned(),
                actual_namespace,
                actual_database,
            });
        }

        }

        Ok(Self::from_open_client(config, guard, client))
    }

    fn from_open_client(config: SurrealStorageConfig, guard: Arc<dyn StorageGuard>, client: SurrealClient) -> Self {
        Self {
            inner: Arc::new(SurrealStorageInner {
                watcher_admission_credentials: load_watcher_admission_credentials_from_environment().ok().map(Arc::new),
                watcher_admissions: StdMutex::new(BTreeMap::new()),
                watcher_admission_configured: AtomicBool::new(false),
                watcher_admission_bootstrap_failure: StdMutex::new(None),
                resource_access_lifecycle: crate::swarm_orchestration::resource_scope::ResourceAccessLifecycleRegistry::new(),
                config,
                client: RwLock::new(Some(client)),
                guard,
                lifecycle: AtomicU8::new(LIFECYCLE_OPEN),
                shutdown: Mutex::new(ShutdownCoordinatorState::Open),
                cancel: StdMutex::new(CancellationToken::new()),
                shutdown_report: StdMutex::new(None),
                authority_publication_gate: Arc::new(RwLock::new(())),
                pending_authority_mutations: StdMutex::new(BTreeMap::new()),
                authority_pending_changed: watch::channel(None).0,
                transaction_settlements: StdMutex::new(BTreeMap::new()),
                member_login_admission: StdMutex::new(true),
                pending_context_guards: StdMutex::new(BTreeMap::new()),
                pending_context_acquisitions: StdMutex::new(BTreeMap::new()),
                leases_in_flight: AtomicUsize::new(0),
                lease_high_water: AtomicUsize::new(0),
            }),
        }
    }

    #[cfg(test)]
    fn for_schema_measurement(client: SurrealClient) -> Result<Self, SurrealStorageError> {
        let config = SurrealStorageConfig::with_path(std::env::temp_dir().join("handshake-schema-measurement"))?;
        Ok(Self::from_open_client(config, Arc::new(DefaultStorageGuard), client))
    }

    fn pending_authority_mutation(&self) -> Option<uuid::Uuid> {
        self.inner.pending_authority_mutations.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()).keys().next().copied()
    }

    pub(crate) async fn authority_publication_read(&self) -> Result<OwnedRwLockReadGuard<()>, SurrealStorageError> {
        let mut changed = self.inner.authority_pending_changed.subscribe();
        if let Some(operation_id) = self.pending_authority_mutation() {
            return Err(SurrealStorageError::TransactionSettlementPending { operation_id });
        }
        let guard = tokio::select! {
            biased;
            _ = changed.changed() => {
                let operation_id = self.pending_authority_mutation()
                    .ok_or(SurrealStorageError::ResourceAuthorityUnavailable)?;
                return Err(SurrealStorageError::TransactionSettlementPending { operation_id });
            }
            _ = tokio::time::sleep(self.config().statement_timeout()) => {
                return Err(SurrealStorageError::StatementTimeout { waited_ms: self.config().statement_timeout().as_millis() });
            }
            guard = self.inner.authority_publication_gate.clone().read_owned() => guard,
        };
        if let Some(operation_id) = self.pending_authority_mutation() {
            return Err(SurrealStorageError::TransactionSettlementPending { operation_id });
        }
        Ok(guard)
    }

    pub(crate) async fn with_owned_authority_mutation<T, E, F, Fut>(
        &self, operation: F,
    ) -> Result<Result<T, E>, SurrealStorageError>
    where
        T: Send + 'static,
        E: Send + 'static,
        F: FnOnce(SurrealStorage) -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, E>> + Send + 'static,
    {
        use futures::FutureExt;
        let storage = self.clone();
        let captured = capture_model_lane_operation();
        let resource_authorization = RESOURCE_ACCESS_AUTHORIZATION.try_with(Clone::clone).ok();
        // The worker owns the gate independently of the requesting HTTP future.
        tokio::spawn(async move {
            let operation_id = uuid::Uuid::now_v7();
            let mut changed = storage.inner.authority_pending_changed.subscribe();
            if let Some(operation_id) = storage.pending_authority_mutation() {
                return Err(SurrealStorageError::TransactionSettlementPending { operation_id });
            }
            let guard = tokio::select! {
                biased;
                _ = changed.changed() => {
                    let operation_id = storage.pending_authority_mutation()
                        .ok_or(SurrealStorageError::ResourceAuthorityUnavailable)?;
                    return Err(SurrealStorageError::TransactionSettlementPending { operation_id });
                }
                _ = tokio::time::sleep(storage.config().statement_timeout()) => {
                    return Err(SurrealStorageError::StatementTimeout { waited_ms: storage.config().statement_timeout().as_millis() });
                }
                guard = storage.inner.authority_publication_gate.clone().write_owned() => guard,
            };
            if let Some(operation_id) = storage.pending_authority_mutation() {
                return Err(SurrealStorageError::TransactionSettlementPending { operation_id });
            }
            let submitted = Arc::new(AtomicU8::new(0));
            let work = with_captured_model_lane_operation(&storage, captured,
                AUTHORITY_MUTATION_SUBMITTED.scope(submitted.clone(), operation(storage.clone())));
            let result = std::panic::AssertUnwindSafe(async {
                if let Some(authority) = resource_authorization {
                    RESOURCE_ACCESS_AUTHORIZATION.scope(authority, work).await
                } else {
                    work.await
                }
            }).catch_unwind().await;
            let succeeded = matches!(&result, Ok(Ok(_)));
            if !succeeded && submitted.load(Ordering::Acquire) == 1 {
                // An engine-bound failure is not proof of rollback. Retain this exact
                // writer fence until independent canonical settlement reconciliation.
                storage.inner.pending_authority_mutations.lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner()).insert(operation_id, guard);
                storage.inner.authority_pending_changed.send_replace(Some(operation_id));
                tracing::error!(%operation_id, "canonical authority mutation requires settlement reconciliation");
            } else {
                drop(guard);
            }
            match result {
                Ok(result) => Ok(result),
                Err(_) => Err(SurrealStorageError::TransactionCallbackPanicked),
            }
        }).await.map_err(|error| SurrealStorageError::TransactionWorker(error.to_string()))?
    }

    pub async fn open_default() -> Result<Self, SurrealStorageError> {
        Self::open(SurrealStorageConfig::from_env()?).await
    }

    /// One initially empty authentication lifecycle shared by all storage clones.
    pub fn resource_access_lifecycle(&self) -> crate::swarm_orchestration::resource_scope::ResourceAccessLifecycleRegistry {
        self.inner.resource_access_lifecycle.clone()
    }

    pub fn config(&self) -> &SurrealStorageConfig {
        &self.inner.config
    }

    /// Runs an operation while holding a shared lifecycle lease.
    ///
    /// The callback receives only a sealed facade. Holding the read lease across
    /// its returned future lets shutdown stop new work and drain every in-flight
    /// operation before dropping the final owned SDK handle.
    ///
    /// MT-142: every data operation is bounded by the configured
    /// `statement_timeout` (review R2-1-4), clamped to the remaining budget of
    /// the enclosing [`Self::with_operation_deadline`] scope when there is one.
    /// On expiry the operation future is dropped (lease released) and the typed
    /// [`SurrealStorageError::StatementTimeout`] is returned; the engine may
    /// still apply the statement, so the outcome is unknown and never retried.
    /// Paths that remain unbounded by design: `with_admin_operation` (schema
    /// bootstrap, test inspector/mutator) and the existing `with_transaction`
    /// Atelier mirror path. Protected transactions use the authenticated entrypoint.
    pub async fn with_data_operation<T, F>(&self, operation: F) -> Result<T, SurrealStorageError>
    where
        T: Send,
        F: for<'a> FnOnce(SurrealDataContext<'a>) -> SurrealOperation<'a, T> + Send,
    {
        let timeout = self.effective_statement_timeout();
        if timeout.is_zero() {
            // Never dispatch with a zero budget: a `timeout(0, ..)` would report
            // `StatementTimeout { waited_ms: 0 }`, which claims the engine may
            // have applied a statement it never received.
            return Err(SurrealStorageError::StatementBudgetExhausted {
                budget_ms: self.inner.config.statement_timeout.as_millis(),
            });
        }
        if let Ok(protected) = PROTECTED_TRANSACTION.try_with(Clone::clone) {
            let scope = current_record_user_scope().ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
            if !Arc::ptr_eq(&self.inner, &protected.storage.inner)
                || scope.resource_id != protected.authority.resource_id
                || scope.grant_id != protected.authority.grant_id
                || scope.session_id != protected.authority.session_id
                || scope.workspace_id != protected.authority.workspace_id
                || scope.capability_id != protected.authority.capability_id
                || scope.action != protected.authority.action
                || scope.channel_binding_hash != protected.authority.channel_binding_hash
                || scope.session_token != protected.authority.session_token
            {
                return Err(SurrealStorageError::TransactionAuthorityRequired);
            }
            // The outer controller owns cancellation, settlement, and the lifecycle
            // lease. Dropping a nested timeout must not drop that transaction.
            return operation(SurrealDataContext {
                client: &protected.client, transaction: Some(&protected.transaction),
            }).await;
        }
        let record_user_scope = current_record_user_scope();
        let namespace = self.config().namespace().to_owned();
        let database = self.config().database().to_owned();
        let leased_operation = async move {
            if self.inner.lifecycle.load(Ordering::Acquire) != LIFECYCLE_OPEN {
                return Err(SurrealStorageError::Closed);
            }
            let guard = observe("storage_lease_wait", 0, self.inner.client.read(), |_| false).await;
            if self.inner.lifecycle.load(Ordering::Acquire) != LIFECYCLE_OPEN {
                return Err(SurrealStorageError::Closed);
            }
            let client = guard.as_ref().ok_or(SurrealStorageError::Closed)?;
            let _lease_count = self.inner.enter_lease();
            INSIDE_SURREAL_OPERATION
                .scope((), async move {
                    if let Some(scope) = record_user_scope {
                        let scoped = client.clone();
                        observe_result(
                            "storage_namespace",
                            scoped.use_ns(namespace).use_db(database),
                        )
                        .await?;
                        observe_result(
                            "storage_signin",
                            scoped.signin(RecordSignin {
                                namespace: self.config().namespace().to_owned(),
                                database: self.config().database().to_owned(),
                                access: resource_authority::AUTHORITY_ACCESS_METHOD.to_owned(),
                                params: resource_authority::SigninParams {
                                    token_hash: hex::encode(sha2::Sha256::digest(
                                        scope.session_token.as_bytes(),
                                    )),
                                    channel_binding_hash: scope.channel_binding_hash,
                                },
                            }),
                        )
                        .await?;
                        observe_result(
                            "storage_operation",
                            operation(SurrealDataContext { client: &scoped, transaction: None }),
                        )
                        .await
                    } else {
                        observe_result(
                            "storage_operation",
                            operation(SurrealDataContext { client, transaction: None }),
                        )
                        .await
                    }
                })
                .await
        };
        match tokio::time::timeout(timeout, leased_operation).await {
            Ok(result) => result,
            Err(_elapsed) => Err(SurrealStorageError::StatementTimeout {
                waited_ms: timeout.as_millis(),
            }),
        }
    }

    /// Keeps the lifecycle lease until an explicit transaction is committed or
    /// cancelled, including when its caller drops the awaiting future.
    pub(crate) async fn with_transaction<T, E, F>(&self, operation: F) -> Result<T, E>
    where
        T: Send + 'static,
        E: From<SurrealStorageError> + Send + 'static,
        F: for<'a> FnOnce(SurrealTransactionContext<'a>) -> SurrealTransactionOperation<'a, T, E>
            + Send + 'static,
    {
        self.transaction_worker(TransactionInvocation::Existing, None, None,
            TransactionSettlementMode::Mutation, operation).await
    }

    pub(crate) async fn with_owned_activation_transaction<T, E, F>(
        &self,
        owner: resource_authority::CapturedActivationAuthority,
        fence: Arc<OwnedRwLockReadGuard<()>>,
        cancellation: Arc<ContextOperationCancellation>,
        operation: F,
    ) -> Result<T, E>
    where
        T: Send + 'static,
        E: From<SurrealStorageError> + Send + 'static,
        F: for<'a> FnOnce(SurrealTransactionContext<'a>) -> SurrealTransactionOperation<'a, T, E>
            + Send + 'static,
    {
        let operation_id = owner.operation_id();
        self.reserve_transaction_settlement(operation_id, Some(AuthenticatedTransactionRetention::Activation(ActivationTransactionRetention {
            _authority: owner,
            _fence: fence,
        }))).map_err(E::from)?;
        self.transaction_worker(
            TransactionInvocation::Existing,
            Some(operation_id),
            Some(cancellation),
            TransactionSettlementMode::Mutation,
            operation,
        ).await
    }


    pub(crate) async fn with_album_membership_transaction<T, E, F>(
        &self,
        operation: AlbumMutationOperationReference,
        commit_expectation: AlbumMutationCommitExpectation,
        callback: F,
    ) -> Result<T, E>
    where
        T: Send + 'static,
        E: From<SurrealStorageError> + Send + 'static,
        F: for<'a> FnOnce(SurrealTransactionContext<'a>) -> SurrealTransactionOperation<'a, T, E>
            + Send + 'static,
    {
        let authority = current_record_user_scope()
            .ok_or_else(|| E::from(SurrealStorageError::TransactionAuthorityRequired))?;
        if !commit_expectation.matches_operation_and_authority(&operation, &authority) {
            return Err(E::from(SurrealStorageError::TransactionAuthorityRequired));
        }
        let operation_id = operation.operation_id;
        let retention = AlbumMembershipTransactionRetention {
            operation,
            authority,
            commit_expectation: Arc::new(commit_expectation),
            settlement: AlbumMutationSettlement::Pending,
            #[cfg(feature = "test-utils")]
            proof: ALBUM_MUTATION_PROOF_CONTROL.try_with(Clone::clone).ok(),
        };
        self.authenticated_transaction(
            operation_id,
            None,
            Some(AuthenticatedTransactionRetention::AlbumMembership(retention)),
            callback,
        ).await
    }

    pub fn observe_album_membership_transaction(
        &self,
        operation: &AlbumMutationOperationReference,
    ) -> Result<AlbumMutationObservation, SurrealStorageError> {
        let authority = current_record_user_scope()
            .ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
        let settlements = self.inner.transaction_settlements.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match settlements.get(&operation.operation_id) {
            Some(Some(AuthenticatedTransactionRetention::AlbumMembership(retained)))
                if retained.operation == *operation && retained.matches_authority(&authority) => {
                Ok(AlbumMutationObservation {
                    operation: retained.operation.clone(),
                    settlement: retained.settlement,
                })
            }
            _ => Err(SurrealStorageError::TransactionAuthorityRequired),
        }
    }


    /// Reconciles only a positive commit witness for the retained original operation.
    /// Missing, denied, or mismatched canonical state never establishes rollback.
    pub async fn reconcile_album_membership_transaction(
        &self,
        operation: &AlbumMutationOperationReference,
    ) -> Result<AlbumMutationObservation, SurrealStorageError> {
        let authority = current_record_user_scope().ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
        if PROTECTED_TRANSACTION.try_with(|_| ()).is_ok() {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
        let expected = {
            let settlements = self.inner.transaction_settlements.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            let Some(Some(AuthenticatedTransactionRetention::AlbumMembership(retained))) =
                settlements.get(&operation.operation_id) else { return Err(SurrealStorageError::TransactionAuthorityRequired); };
            if retained.operation != *operation || !retained.matches_authority(&authority) {
                return Err(SurrealStorageError::TransactionAuthorityRequired);
            }
            if retained.settlement != AlbumMutationSettlement::Unobserved {
                return Ok(AlbumMutationObservation { operation: retained.operation.clone(), settlement: retained.settlement });
            }
            retained.commit_expectation.clone()
        };
        let bindings = expected.read_bindings();
        let rows: Vec<AlbumMutationCommitReadback> = self.with_data_operation(move |context| Box::pin(async move {
            context.query_values(ALBUM_MUTATION_COMMIT_READBACK, bindings).await
        })).await?;
        let matched = matches!(rows.as_slice(), [row] if expected.matches_readback(row));
        let authority = current_record_user_scope().ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
        let mut settlements = self.inner.transaction_settlements.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(Some(AuthenticatedTransactionRetention::AlbumMembership(retained))) =
            settlements.get_mut(&operation.operation_id) else { return Err(SurrealStorageError::TransactionAuthorityRequired); };
        if retained.operation != *operation || !retained.matches_authority(&authority)
            || !Arc::ptr_eq(&retained.commit_expectation, &expected) {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
        if matched && retained.settlement == AlbumMutationSettlement::Unobserved {
            retained.settlement = AlbumMutationSettlement::CommitObserved;
            #[cfg(feature = "test-utils")]
            if let Some(proof) = &retained.proof { proof.observe(retained.settlement); }
        }
        Ok(AlbumMutationObservation { operation: retained.operation.clone(), settlement: retained.settlement })
    }


    pub fn acknowledge_album_membership_transaction(
        &self,
        operation: &AlbumMutationOperationReference,
    ) -> Result<bool, SurrealStorageError> {
        let authority = current_record_user_scope()
            .ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
        let mut settlements = self.inner.transaction_settlements.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let terminal = match settlements.get(&operation.operation_id) {
            Some(Some(AuthenticatedTransactionRetention::AlbumMembership(retained)))
                if retained.operation == *operation && retained.matches_authority(&authority) => {
                !matches!(retained.settlement,
                    AlbumMutationSettlement::Pending | AlbumMutationSettlement::Unobserved)
            }
            _ => return Err(SurrealStorageError::TransactionAuthorityRequired),
        };
        if terminal {
            settlements.remove(&operation.operation_id);
        }
        Ok(terminal)
    }

    pub(crate) fn authenticated_transaction_context_observation(
        &self,
        original: &Arc<WorkspaceArtifactPublicationOperationContext>,
    ) -> AuthenticatedTransactionContextObservation {
        use AuthenticatedTransactionContextObservation as Observation;
        let Some(bound_storage) = original.storage.upgrade() else {
            return Observation::Uncertain;
        };
        if !Arc::ptr_eq(&bound_storage, &self.inner)
            || original.namespace != self.config().namespace()
            || original.database != self.config().database()
        {
            return Observation::Uncertain;
        }
        let settlements = self.inner.transaction_settlements.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match settlements.get(&original.operation_id()) {
            Some(Some(AuthenticatedTransactionRetention::WorkspaceArtifactPublication {
                original: retained, settlement, ..
            })) if Arc::ptr_eq(retained, original) => match settlement {
                ActualSettlement::CommitObserved => Observation::CommitObserved,
                ActualSettlement::CancelNoEffectObserved => Observation::CancelObserved,
                ActualSettlement::CallbackPanicRollbackObserved => Observation::CallbackPanicObserved,
                _ => Observation::Uncertain,
            },
            _ => Observation::Uncertain,
        }
    }

    fn retain_workspace_artifact_error<E: Send + 'static>(
        &self, operation_id: Option<uuid::Uuid>, error: &mut Option<E>,
    ) {
        let Some(operation_id) = operation_id else { return; };
        let mut settlements = self.inner.transaction_settlements.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(Some(AuthenticatedTransactionRetention::WorkspaceArtifactPublication {
            completed_error, ..
        })) = settlements.get_mut(&operation_id) {
            if completed_error.is_none() { *completed_error = error.take().map(|error| Box::new(error) as Box<dyn std::any::Any + Send>); }
        }
    }
    fn record_workspace_artifact_owned_finalization(
        &self, operation_id: Option<uuid::Uuid>, finalization: WorkspaceArtifactEngineFinalization,
    ) {
        let Some(operation_id) = operation_id else { return; };
        let mut settlements = self.inner.transaction_settlements.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(Some(AuthenticatedTransactionRetention::WorkspaceArtifactPublication {
            owned_finalization, ..
        })) = settlements.get_mut(&operation_id) {
            if owned_finalization.is_none() { *owned_finalization = Some(finalization); }
        }
    }

    async fn recheck_workspace_artifact_publication_owner(
        &self, original: &Arc<WorkspaceArtifactPublicationOperationContext>,
        authority: Arc<resource_authority::WorkspaceArtifactPublicationAuthorization>,
        full_association: bool,
        read: Arc<WorkspaceArtifactPublicationReconciliationRead>,
    ) -> Result<(), SurrealStorageError> {
        let operation_id = original.operation_id();
        let storage = self.clone();
        self.transaction_worker(
            TransactionInvocation::Authenticated {
                scope: original.scope.clone(), deadline: read.deadline(), operation_id,
            }, None, None, TransactionSettlementMode::WorkspaceArtifactCanonicalRead(read.client.clone().ok_or(SurrealStorageError::TransactionSettlementPending { operation_id })?),
            move |transaction| Box::pin(async move {
                if full_association {
                    storage.recheck_workspace_artifact_publication_in_transaction(&authority, &transaction, &read).await
                } else {
                    storage.recheck_workspace_artifact_publication_authority_in_transaction(&authority, &transaction, &read).await
                }
            }),
        ).await.map_err(|_: resource_authority::ResourceAuthorityError|
            SurrealStorageError::TransactionSettlementPending { operation_id })
    }

    async fn recheck_closed_workspace_artifact_publication_owner(
        &self, original: &Arc<WorkspaceArtifactPublicationOperationContext>,
        authority: Arc<resource_authority::WorkspaceArtifactPublicationAuthorization>,
        terminal: Arc<ArtifactPublicationCanonicalTerminalForOwner>,
        closure: Arc<crate::storage::artifacts::ArtifactPublicationOwnedClosure>,
        read: Arc<WorkspaceArtifactPublicationReconciliationRead>,
    ) -> Result<(), SurrealStorageError> {
        let operation_id = original.operation_id();
        let storage = self.clone();
        self.transaction_worker(
            TransactionInvocation::Authenticated {
                scope: original.scope.clone(), deadline: read.deadline(), operation_id,
            }, None, None, TransactionSettlementMode::WorkspaceArtifactCanonicalRead(read.client.clone().ok_or(SurrealStorageError::TransactionSettlementPending { operation_id })?),
            move |transaction| Box::pin(async move {
                storage.recheck_workspace_artifact_publication_after_owned_closure_in_transaction(
                    &authority, &transaction, &terminal, &closure, &read,
                ).await
            }),
        ).await.map_err(|_: resource_authority::ResourceAuthorityError|
            SurrealStorageError::TransactionSettlementPending { operation_id })
    }
    pub(crate) async fn reconcile_workspace_artifact_publication<T, E>(
        &self,
        original: Arc<WorkspaceArtifactPublicationOperationContext>,
        retention: Arc<dyn std::any::Any + Send + Sync>,
        resources: Arc<crate::storage::artifacts::ArtifactPublicationResources>,
    ) -> Result<T, E>
    where T: Send + 'static, E: From<SurrealStorageError> + Send + 'static {
        let operation_id = original.operation_id();
        let pending = || SurrealStorageError::TransactionSettlementPending { operation_id };
        let (finalization, cached_terminal, cached_closure, read) = {
            let mut settlements = self.inner.transaction_settlements.lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let Some(Some(AuthenticatedTransactionRetention::WorkspaceArtifactPublication {
                original: retained, owner, resources: held, worker_finished: true,
                owned_finalization: Some(finalization), callback_succeeded,
                canonical_terminal, owned_closure, reconciliation_read, canonical_read_client, ..
            })) = settlements.get_mut(&operation_id) else { return Err(E::from(pending())); };
            if !Arc::ptr_eq(retained, &original) || !Arc::ptr_eq(owner, &retention)
                || !Arc::ptr_eq(held, &resources) || reconciliation_read.is_some()
                || (matches!(finalization, WorkspaceArtifactEngineFinalization::Commit) && !*callback_succeeded) {
                return Err(E::from(pending()));
            }
            let remaining = original.authority_expires_at.signed_duration_since(chrono::Utc::now())
                .to_std().unwrap_or(Duration::ZERO).min(Duration::from_millis(5000));
            let now = Instant::now();
            let deadline = now.checked_add(remaining).unwrap_or(now);
            let read = Arc::new(WorkspaceArtifactPublicationReconciliationRead {
                original: original.clone(), deadline, client: canonical_read_client.clone(), authority_expires_at: original.authority_expires_at,
            });
            *reconciliation_read = Some(read.clone());
            if remaining.is_zero() || read.client.is_none() { return Err(E::from(pending())); }
            (*finalization, canonical_terminal.clone(), owned_closure.clone(), read)
        };
        let authority = retention.downcast::<resource_authority::WorkspaceArtifactPublicationAuthorization>()
            .map_err(|_| E::from(pending()))?;
        if let (Some(terminal), Some(closure)) = (cached_terminal, cached_closure) {
            if !Arc::ptr_eq(&terminal.original, &original) || !terminal.matches_resources(&resources)
                || !closure.matches_terminal(&terminal) { return Err(E::from(pending())); }
            if matches!(terminal.decision(), ArtifactPublicationCanonicalDecision::Commit) {
                self.recheck_closed_workspace_artifact_publication_owner(
                    &original, authority, terminal.clone(), closure.clone(), read.clone(),
                ).await.map_err(E::from)?;
            } else {
                self.recheck_workspace_artifact_publication_owner(&original, authority, false, read.clone()).await.map_err(E::from)?;
            }
            return if matches!(terminal.decision(), ArtifactPublicationCanonicalDecision::Commit) {
                self.finish_workspace_artifact_publication(&original, &terminal, &closure, &read).map_err(E::from)
            } else {
                Err(self.finish_aborted_workspace_artifact_publication::<E>(&original, &terminal, &closure, &read).map_err(E::from)?)
            };
        }
        self.recheck_workspace_artifact_publication_owner(&original, authority.clone(),
            matches!(finalization, WorkspaceArtifactEngineFinalization::Commit), read.clone()).await.map_err(E::from)?;
        let terminal = {
            let mut settlements = self.inner.transaction_settlements.lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let Some(Some(AuthenticatedTransactionRetention::WorkspaceArtifactPublication {
                original: retained, resources: held, canonical_terminal, ..
            })) = settlements.get_mut(&operation_id) else { return Err(E::from(pending())); };
            if !Arc::ptr_eq(retained, &original) || !Arc::ptr_eq(held, &resources) {
                return Err(E::from(pending()));
            }
            canonical_terminal.get_or_insert_with(|| Arc::new(ArtifactPublicationCanonicalTerminalForOwner {
                original: original.clone(), resources: resources.clone(),
                decision: match finalization {
                    WorkspaceArtifactEngineFinalization::Commit => ArtifactPublicationCanonicalDecision::Commit,
                    WorkspaceArtifactEngineFinalization::Rollback => ArtifactPublicationCanonicalDecision::Abort,
                },
            })).clone()
        };
        let closure = {
            let settlements = self.inner.transaction_settlements.lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            match settlements.get(&operation_id) {
                Some(Some(AuthenticatedTransactionRetention::WorkspaceArtifactPublication { owned_closure, .. })) => owned_closure.clone(),
                _ => return Err(E::from(pending())),
            }
        };
        let closure = match closure {
            Some(closure) => closure,
            None => {
                {
                    let settlements = self.inner.transaction_settlements.lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    if read.deadline() <= Instant::now() || read.authority_expires_at() <= chrono::Utc::now()
                        || !matches!(settlements.get(&operation_id),
                            Some(Some(AuthenticatedTransactionRetention::WorkspaceArtifactPublication {
                                original: held, reconciliation_read: Some(held_read), ..
                            })) if Arc::ptr_eq(held, &original) && Arc::ptr_eq(held_read, &read)) {
                        return Err(E::from(pending()));
                    }
                }
                let closure = Arc::new(resources.close_after_canonical_terminal(&terminal).map_err(|_| E::from(pending()))?);
                let mut settlements = self.inner.transaction_settlements.lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                let Some(Some(AuthenticatedTransactionRetention::WorkspaceArtifactPublication {
                    original: retained, resources: held, owned_closure, ..
                })) = settlements.get_mut(&operation_id) else { return Err(E::from(pending())); };
                if !Arc::ptr_eq(retained, &original) || !Arc::ptr_eq(held, &resources) { return Err(E::from(pending())); }
                owned_closure.get_or_insert_with(|| closure.clone()).clone()
            }
        };
        self.recheck_workspace_artifact_publication_owner(&original, authority, false, read.clone()).await.map_err(E::from)?;
        if matches!(terminal.decision(), ArtifactPublicationCanonicalDecision::Commit) {
            self.finish_workspace_artifact_publication(&original, &terminal, &closure, &read).map_err(E::from)
        } else {
            Err(self.finish_aborted_workspace_artifact_publication::<E>(&original, &terminal, &closure, &read).map_err(E::from)?)
        }
    }

    fn finish_aborted_workspace_artifact_publication<E: Send + 'static>(
        &self, original: &Arc<WorkspaceArtifactPublicationOperationContext>,
        terminal: &ArtifactPublicationCanonicalTerminalForOwner,
        closure: &crate::storage::artifacts::ArtifactPublicationOwnedClosure,
        read: &Arc<WorkspaceArtifactPublicationReconciliationRead>,
    ) -> Result<E, SurrealStorageError> {
        let operation_id = original.operation_id();
        let pending = || SurrealStorageError::TransactionSettlementPending { operation_id };
        if read.deadline() <= Instant::now() || read.authority_expires_at() <= chrono::Utc::now()
            || !read.matches_original(original) { return Err(pending()); }
        if !Arc::ptr_eq(original, &terminal.original) || !closure.matches_terminal(terminal)
            || !matches!(terminal.decision(), ArtifactPublicationCanonicalDecision::Abort) {
            return Err(pending());
        }
        let mut settlements = self.inner.transaction_settlements.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(Some(AuthenticatedTransactionRetention::WorkspaceArtifactPublication {
            original: held, resources, completed_error, worker_finished: true, reconciliation_read: Some(held_read), ..
        })) = settlements.get_mut(&operation_id) else { return Err(pending()); };
        if !Arc::ptr_eq(held_read, read) || !Arc::ptr_eq(held, original) || !terminal.matches_resources(resources)
            || !completed_error.as_ref().is_some_and(|value| value.is::<E>()) { return Err(pending()); }
        if read.deadline() <= Instant::now() || read.authority_expires_at() <= chrono::Utc::now() {
            return Err(pending());
        }
        let error = completed_error.take().ok_or_else(pending)?;
        let error = match error.downcast::<E>() {
            Ok(error) => error,
            Err(error) => { *completed_error = Some(error); return Err(pending()); }
        };
        settlements.remove(&operation_id);
        Ok(*error)
    }
    fn finish_workspace_artifact_publication<T: Send + 'static>(
        &self,
        original: &Arc<WorkspaceArtifactPublicationOperationContext>,
        terminal: &ArtifactPublicationCanonicalTerminalForOwner,
        closure: &crate::storage::artifacts::ArtifactPublicationOwnedClosure,
        read: &Arc<WorkspaceArtifactPublicationReconciliationRead>,
    ) -> Result<T, SurrealStorageError> {
        let operation_id = original.operation_id();
        let pending = || SurrealStorageError::TransactionSettlementPending { operation_id };
        if read.deadline() <= Instant::now() || read.authority_expires_at() <= chrono::Utc::now()
            || !read.matches_original(original) { return Err(pending()); }
        let storage = original.storage.upgrade().ok_or_else(pending)?;
        if !Arc::ptr_eq(&storage, &self.inner)
            || !Arc::ptr_eq(original, &terminal.original)
            || original.namespace != self.config().namespace()
            || original.database != self.config().database()
            || !matches!(terminal.decision(), ArtifactPublicationCanonicalDecision::Commit)
            || !closure.matches_terminal(terminal)
        {
            return Err(pending());
        }
        let mut settlements = self.inner.transaction_settlements.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(Some(AuthenticatedTransactionRetention::WorkspaceArtifactPublication {
            original: retained, resources, completed_result, worker_finished, reconciliation_read: Some(held_read), ..
        })) = settlements.get_mut(&operation_id) else { return Err(pending()); };
        if !Arc::ptr_eq(held_read, read) || !*worker_finished || !Arc::ptr_eq(retained, original)
            || !terminal.matches_resources(resources)
            || !completed_result.as_ref().is_some_and(|value| value.is::<T>())
        {
            return Err(pending());
        }
        if read.deadline() <= Instant::now() || read.authority_expires_at() <= chrono::Utc::now() {
            return Err(pending());
        }
        let value = completed_result.take().ok_or_else(pending)?;
        let value = match value.downcast::<T>() {
            Ok(value) => value,
            Err(value) => {
                *completed_result = Some(value);
                return Err(pending());
            }
        };
        settlements.remove(&operation_id);
        Ok(*value)
    }
    pub(crate) async fn with_authenticated_transaction_context<T, E, F>(
        &self,
        original: Arc<WorkspaceArtifactPublicationOperationContext>,
        retention: Arc<dyn std::any::Any + Send + Sync>,
        resources: Arc<crate::storage::artifacts::ArtifactPublicationResources>,
        operation: F,
    ) -> Result<T, E>
    where
        T: Send + 'static,
        E: From<SurrealStorageError> + Send + 'static,
        F: for<'a> FnOnce(SurrealTransactionContext<'a>) -> SurrealTransactionOperation<'a, T, E>
            + Send + 'static,
    {
        let invalid = || E::from(SurrealStorageError::InvalidModelLaneRecord {
            reason: "workspace artifact publication context unavailable",
        });
        let bound_storage = original.storage.upgrade().ok_or_else(invalid)?;
        if !Arc::ptr_eq(&bound_storage, &self.inner)
            || original.namespace != self.config().namespace()
            || original.database != self.config().database()
            || original.deadline <= Instant::now()
            || original.expires_at <= chrono::Utc::now()
        {
            return Err(invalid());
        }
        let operation_id = original.operation_id();
        let scope = original.scope.clone();
        let deadline = original.deadline;
        self.reserve_transaction_settlement(operation_id, Some(
            AuthenticatedTransactionRetention::WorkspaceArtifactPublication {
                original: original.clone(), owner: retention.clone(), resources: resources.clone(), completed_result: None, completed_error: None, worker_finished: false, callback_succeeded: false, canonical_terminal: None, owned_closure: None, owned_finalization: None, reconciliation_read: None, canonical_read_client: None, settlement: ActualSettlement::Unobserved,
            },
        )).map_err(E::from)?;
        let _result = self.transaction_worker(TransactionInvocation::Authenticated {
            scope, deadline, operation_id,
        }, Some(operation_id), None, TransactionSettlementMode::Mutation, operation).await;
        if let Err(error) = _result {
            let retain_error = {
                let settlements = self.inner.transaction_settlements.lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                !matches!(settlements.get(&operation_id),
                    Some(Some(AuthenticatedTransactionRetention::WorkspaceArtifactPublication {
                        worker_finished: true, callback_succeeded: true,
                        owned_finalization: Some(WorkspaceArtifactEngineFinalization::Commit), ..
                    })))
            };
            if retain_error {
                let mut error = Some(error);
                self.retain_workspace_artifact_error(Some(operation_id), &mut error);
            }
        }
        match self.reconcile_workspace_artifact_publication(original, retention, resources).await {
            Ok(value) => Ok(value),
            Err(error) => Err(error),
        }
    }

    pub(crate) async fn with_original_runcreate_transaction<T, E, F>(
        &self, operation_id: uuid::Uuid, retention: Arc<dyn std::any::Any + Send + Sync>, operation: F,
    ) -> OriginalRunCreateOutcome<T, E>
    where T: Send + 'static, E: From<SurrealStorageError> + Send + 'static,
        F: for<'a> FnOnce(SurrealTransactionContext<'a>) -> SurrealTransactionOperation<'a, T, E> + Send + 'static,
    {
        let retained = Arc::new(StdMutex::new(OriginalRunCreateRetention {
            owner: retention, binding: None, proof: None, finalized: None,
            value: None, error: None, finished: false,
        }));
        let result = self.authenticated_transaction(operation_id, None,
            Some(AuthenticatedTransactionRetention::OriginalRunCreate(retained.clone())), operation).await;
        let mut state = retained.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.finished && state.finalized == Some(OriginalRunCreateFinalization::Commit)
            && state.value.as_ref().is_some_and(|value| value.is::<T>()) {
            if let Some(value) = state.value.take() {
                match value.downcast::<T>() {
                    Ok(value) => return OriginalRunCreateOutcome { result: Ok(*value), proof: state.proof.take() },
                    Err(value) => state.value = Some(value),
                }
            }
        }
        if state.error.as_ref().is_some_and(|error| error.is::<E>()) {
            if let Some(error) = state.error.take() {
                match error.downcast::<E>() {
                    Ok(error) => {
                        let proof = if state.finished && state.finalized == Some(OriginalRunCreateFinalization::Rollback) {
                            state.proof.take()
                        } else { None };
                        return OriginalRunCreateOutcome { result: Err(*error), proof };
                    }
                    Err(error) => state.error = Some(error),
                }
            }
        }
        OriginalRunCreateOutcome { result, proof: None }
    }

    pub(crate) fn acknowledge_original_runcreate_finalization(
        &self, proof: OriginalRunCreateFinalizationProof,
        owner: &Arc<dyn std::any::Any + Send + Sync>,
    ) -> Result<(), (SurrealStorageError, OriginalRunCreateFinalizationProof)> {
        let operation_id = proof.binding.operation_id;
        if !proof.matches_original(self, operation_id, owner) {
            return Err((SurrealStorageError::TransactionRetentionMismatch { operation_id }, proof));
        }
        let mut settlements = self.inner.transaction_settlements.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(Some(AuthenticatedTransactionRetention::OriginalRunCreate(retained))) = settlements.get(&operation_id)
            else { return Err((SurrealStorageError::TransactionRetentionMismatch { operation_id }, proof)); };
        let state = retained.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if !state.finished || state.finalized != Some(proof.finalization)
            || !state.binding.as_ref().is_some_and(|binding| Arc::ptr_eq(binding, &proof.binding)) {
            return Err((SurrealStorageError::TransactionRetentionMismatch { operation_id }, proof));
        }
        drop(state);
        settlements.remove(&operation_id);
        Ok(())
    }
    pub(crate) async fn with_authenticated_transaction<T, E, F>(
        &self, operation_id: uuid::Uuid, operation: F,
    ) -> Result<T, E>
    where
        T: Send + 'static,
        E: From<SurrealStorageError> + Send + 'static,
        F: for<'a> FnOnce(SurrealTransactionContext<'a>) -> SurrealTransactionOperation<'a, T, E>
            + Send + 'static,
    {
        self.authenticated_transaction(operation_id, None, None, operation).await
    }

    pub(crate) async fn with_owned_authenticated_transaction<T, E, F>(
        &self, operation_id: uuid::Uuid, cancellation: Arc<ContextOperationCancellation>, operation: F,
    ) -> Result<T, E>
    where T: Send + 'static, E: From<SurrealStorageError> + Send + 'static,
        F: for<'a> FnOnce(SurrealTransactionContext<'a>) -> SurrealTransactionOperation<'a, T, E> + Send + 'static,
    {
        self.authenticated_transaction(operation_id, Some(cancellation), None, operation).await
    }

    pub(crate) async fn with_owned_authenticated_job_transaction<T, E, F>(
        &self,
        operation_id: uuid::Uuid,
        cancellation: Arc<ContextOperationCancellation>,
        retention: resource_authority::JobAdmissionTransactionRetention,
        operation: F,
    ) -> Result<T, E>
    where
        T: Send + 'static,
        E: From<SurrealStorageError> + Send + 'static,
        F: for<'a> FnOnce(SurrealTransactionContext<'a>) -> SurrealTransactionOperation<'a, T, E>
            + Send + 'static,
    {
        if retention.operation_id() != operation_id {
            return Err(E::from(SurrealStorageError::TransactionRetentionMismatch { operation_id }));
        }
        self.authenticated_transaction(operation_id, Some(cancellation), Some(
            AuthenticatedTransactionRetention::JobAdmission {
                retention,
                phase: JobAdmissionTransactionPhase::EnginePending,
                readback: None,
            },
        ), operation).await
    }

    pub(crate) async fn read_owned_job_admission(
        &self,
        read: resource_authority::JobAdmissionReadbackTransaction,
    ) -> Result<resource_authority::JobAdmissionReadbackResult, crate::storage::JobAdmissionError> {
        let operation_id = read.operation_id();
        if !read.belongs_to(self) {
            return Err(SurrealStorageError::TransactionRetentionMismatch { operation_id }.into());
        }
        let budget = self.effective_statement_timeout();
        let read = Arc::new(read);
        {
            let mut settlements = self.inner.transaction_settlements.lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            match settlements.get_mut(&operation_id) {
                Some(Some(AuthenticatedTransactionRetention::JobAdmission { retention, phase, readback }))
                    if retention.matches_retention(read.retention()) => {
                    if !matches!(phase, JobAdmissionTransactionPhase::CommitObservedReadbackPending)
                        || readback.is_some()
                    {
                        return Err(SurrealStorageError::TransactionSettlementPending { operation_id }.into());
                    }
                    *readback = Some(read.clone());
                    *phase = JobAdmissionTransactionPhase::ReadbackReadPending;
                }
                _ => return Err(SurrealStorageError::TransactionRetentionMismatch { operation_id }.into()),
            }
        }
        if budget.is_zero() {
            return Err(SurrealStorageError::StatementBudgetExhausted { budget_ms: 0 }.into());
        }
        let worker_read = read.clone();
        let observed = self.transaction_worker(
            TransactionInvocation::Authenticated {
                scope: read.scope(), deadline: Instant::now() + budget, operation_id,
            },
            Some(operation_id), None, TransactionSettlementMode::JobReadback(read.clone()),
            move |transaction| Box::pin(async move { worker_read.execute(&transaction).await }),
        ).await?;
        read.finalize(self, observed).await
    }

    pub(crate) fn confirm_job_admission_readback(
        &self,
        completion: resource_authority::JobAdmissionReadbackToken,
    ) -> Result<(), SurrealStorageError> {
        let operation_id = completion.operation_id();
        let mut settlements = self.inner.transaction_settlements.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match settlements.get(&operation_id) {
            Some(Some(AuthenticatedTransactionRetention::JobAdmission { retention, phase, readback: Some(read) }))
                if retention.matches_readback(&completion) && read.matches_readback(&completion) => {
                if !matches!(phase, JobAdmissionTransactionPhase::ReadbackObserved) {
                    return Err(SurrealStorageError::TransactionSettlementPending { operation_id });
                }
                if !read.belongs_to(self) {
                    return Err(SurrealStorageError::TransactionRetentionMismatch { operation_id });
                }
            }
            _ => return Err(SurrealStorageError::TransactionRetentionMismatch { operation_id }),
        }
        completion.require_unexpired()?;
        settlements.remove(&operation_id);
        Ok(())
    }

    pub(crate) async fn with_owned_validator_association_authenticated_transaction<T, E, F>(
        &self,
        operation_id: uuid::Uuid,
        cancellation: Arc<ContextOperationCancellation>,
        retention: resource_authority::ValidatorAssociationTransactionRetention,
        operation: F,
    ) -> Result<T, E>
    where
        T: Send + 'static,
        E: From<SurrealStorageError> + Send + 'static,
        F: for<'a> FnOnce(SurrealTransactionContext<'a>) -> SurrealTransactionOperation<'a, T, E>
            + Send + 'static,
    {
        if retention.operation_id() != operation_id {
            return Err(E::from(SurrealStorageError::TransactionRetentionMismatch { operation_id }));
        }
        let scope = current_record_user_scope()
            .ok_or_else(|| E::from(SurrealStorageError::TransactionAuthorityRequired))?;
        let deadline = retention.original_deadline();
        if Instant::now() >= deadline {
            return Err(E::from(SurrealStorageError::StatementBudgetExhausted { budget_ms: 0 }));
        }
        {
            let settlements = self.inner.transaction_settlements.lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            match settlements.get(&operation_id) {
                Some(Some(AuthenticatedTransactionRetention::ValidatorAssociation(original)))
                    if original.matches_retention(&retention) => {
                    original.enter_engine_once().map_err(E::from)?;
                }
                _ => return Err(E::from(SurrealStorageError::TransactionRetentionMismatch { operation_id })),
            }
        }
        self.transaction_worker(
            TransactionInvocation::Authenticated { scope, deadline, operation_id },
            Some(operation_id),
            Some(cancellation),
            TransactionSettlementMode::Mutation,
            operation,
        ).await
    }

    pub(crate) async fn with_owned_role_authenticated_transaction<T, E, F>(
        &self,
        operation_id: uuid::Uuid,
        cancellation: Arc<ContextOperationCancellation>,
        retention: resource_authority::RoleAuthorityTransactionRetention,
        operation: F,
    ) -> Result<T, E>
    where
        T: Send + 'static,
        E: From<SurrealStorageError> + Send + 'static,
        F: for<'a> FnOnce(SurrealTransactionContext<'a>) -> SurrealTransactionOperation<'a, T, E>
            + Send + 'static,
    {
        if retention.operation_id() != operation_id {
            return Err(E::from(SurrealStorageError::TransactionRetentionMismatch { operation_id }));
        }
        self.authenticated_transaction(operation_id, Some(cancellation), Some(
            AuthenticatedTransactionRetention::Role {
                retention,
                phase: RoleTransactionPhase::EnginePending,
            },
        ), operation).await
    }

    pub(crate) fn reserve_role_preparation(
        &self,
        retention: resource_authority::RoleAuthorityTransactionRetention,
    ) -> Result<(), SurrealStorageError> {
        self.reserve_transaction_settlement(retention.operation_id(), Some(
            AuthenticatedTransactionRetention::Role {
                retention,
                phase: RoleTransactionPhase::PreEnginePrepared,
            },
        ))
    }

    pub(crate) async fn read_owned_routing_authority_capture(
        &self,
        capture: resource_authority::RoutingAuthorityCaptureTransaction,
    ) -> crate::swarm_orchestration::model_lane::ModelLaneResult<resource_authority::RoutingAuthorityCaptureRead> {
        let operation_id = capture.operation_id();
        if !capture.belongs_to(self) {
            return Err(SurrealStorageError::TransactionRetentionMismatch { operation_id }.into());
        }
        let retention = capture.retention().clone();
        self.reserve_transaction_settlement(operation_id, Some(
            AuthenticatedTransactionRetention::Role {
                retention: retention.clone(),
                phase: RoleTransactionPhase::CaptureReadPending,
            },
        ))?;
        let scope = capture.record_user_scope();
        let budget = self.effective_statement_timeout();
        if budget.is_zero() {
            return Err(SurrealStorageError::StatementBudgetExhausted { budget_ms: 0 }.into());
        }
        let (capture, payload) = self.transaction_worker(
            TransactionInvocation::Authenticated {
                scope, deadline: Instant::now() + budget, operation_id,
            },
            Some(operation_id), Some(ContextOperationCancellation::new()),
            TransactionSettlementMode::RoutingCapture(retention),
            move |transaction| Box::pin(async move {
                let payload = capture.execute(&transaction).await?;
                Ok::<_, crate::swarm_orchestration::model_lane::ModelLaneError>((capture, payload))
            }),
        ).await?;
        capture.into_observed_read(payload)
    }

    pub(crate) fn prepare_owned_routing_authority_capture(
        &self,
        finalization: resource_authority::RoutingAuthorityCaptureFinalization,
    ) -> crate::swarm_orchestration::model_lane::ModelLaneResult<resource_authority::PreparedRoutingAuthorityConsumption> {
        let expected = finalization.retention().clone();
        let operation_id = expected.operation_id();
        if !finalization.belongs_to(self) {
            return Err(SurrealStorageError::TransactionRetentionMismatch { operation_id }.into());
        }
        let mut settlements = self.inner.transaction_settlements.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match settlements.get_mut(&operation_id) {
            Some(Some(AuthenticatedTransactionRetention::Role { retention, phase }))
                if matches!(phase, RoleTransactionPhase::CaptureReadObserved)
                    && retention.matches_retention(&expected) => {
                let prepared = finalization.finalize_into_prepared()?;
                *phase = RoleTransactionPhase::PreEnginePrepared;
                Ok(prepared)
            }
            _ => Err(SurrealStorageError::TransactionRetentionMismatch { operation_id }.into()),
        }
    }

    pub(crate) fn cancel_role_preparation(
        &self,
        cancellation: resource_authority::RoleAuthorityPreparationCancellation,
    ) -> Result<(), SurrealStorageError> {
        let operation_id = cancellation.operation_id();
        let mut settlements = self.inner.transaction_settlements.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match settlements.get(&operation_id) {
            Some(Some(AuthenticatedTransactionRetention::Role {
                retention, phase: RoleTransactionPhase::PreEnginePrepared,
            })) if retention.matches_cancellation(&cancellation) => {}
            _ => return Err(SurrealStorageError::TransactionRetentionMismatch { operation_id }),
        }
        settlements.remove(&operation_id);
        Ok(())
    }

    pub(crate) fn complete_role_publication(
        &self,
        completion: resource_authority::RoleAuthorityPublicationCompletion,
    ) -> Result<(), SurrealStorageError> {
        let operation_id = completion.operation_id();
        let mut settlements = self.inner.transaction_settlements.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match settlements.get(&operation_id) {
            Some(Some(AuthenticatedTransactionRetention::Role { retention, phase }))
                if retention.matches_completion(&completion) => {
                if !matches!(phase, RoleTransactionPhase::CommitObservedPublicationPending) {
                    return Err(SurrealStorageError::TransactionSettlementPending { operation_id });
                }
            }
            _ => return Err(SurrealStorageError::TransactionRetentionMismatch { operation_id }),
        }
        settlements.remove(&operation_id);
        Ok(())
    }

    async fn authenticated_transaction<T, E, F>(
        &self,
        operation_id: uuid::Uuid,
        cancellation: Option<Arc<ContextOperationCancellation>>,
        retention: Option<AuthenticatedTransactionRetention>,
        operation: F,
    ) -> Result<T, E>
    where T: Send + 'static, E: From<SurrealStorageError> + Send + 'static,
        F: for<'a> FnOnce(SurrealTransactionContext<'a>) -> SurrealTransactionOperation<'a, T, E> + Send + 'static,
    {
        let scope = current_record_user_scope()
            .ok_or_else(|| E::from(SurrealStorageError::TransactionAuthorityRequired))?;
        let budget = self.effective_statement_timeout();
        if budget.is_zero() {
            return Err(E::from(SurrealStorageError::StatementBudgetExhausted { budget_ms: 0 }));
        }
        self.reserve_transaction_settlement(operation_id, retention).map_err(E::from)?;
        self.transaction_worker(TransactionInvocation::Authenticated {
            scope, deadline: Instant::now() + budget, operation_id,
        }, Some(operation_id), cancellation, TransactionSettlementMode::Mutation, operation).await
    }

    pub(crate) fn context_reconciliation_pending(&self, operation_id: uuid::Uuid) -> bool {
        self.inner.pending_context_guards.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).contains_key(&operation_id)
            || self.inner.pending_context_acquisitions.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).contains_key(&operation_id)
    }

    pub(crate) fn retain_context_acquisition(&self, operation_id: uuid::Uuid, acquisition: model_lane::PendingContextAcquisition) {
        self.inner.pending_context_acquisitions.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).insert(operation_id, acquisition);
        tracing::error!(%operation_id, "owned ContextBundle acquisition requires canonical reconciliation");
    }

    pub(crate) fn retain_context_reconciliation(
        &self, operation_id: uuid::Uuid, guards: Vec<model_lane::ContextCoordinationGuard>,
    ) {
        self.inner.pending_context_guards.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(operation_id, guards);
        tracing::error!(%operation_id, "owned ContextBundle coordination requires canonical reconciliation");
    }

    fn reserve_transaction_settlement(
        &self,
        operation_id: uuid::Uuid,
        retention: Option<AuthenticatedTransactionRetention>,
    ) -> Result<(), SurrealStorageError> {
        let mut settlements = self.inner.transaction_settlements.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match settlements.entry(operation_id) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(retention);
                Ok(())
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                match (entry.get_mut(), retention) {
                    (Some(AuthenticatedTransactionRetention::Role { retention: prepared, phase }),
                        Some(AuthenticatedTransactionRetention::Role {
                            retention: incoming, phase: RoleTransactionPhase::EnginePending,
                        })) if matches!(phase, RoleTransactionPhase::PreEnginePrepared)
                            && prepared.operation_id() == operation_id
                            && prepared.matches_retention(&incoming) => {
                        *phase = RoleTransactionPhase::EnginePending;
                        Ok(())
                    }
                    _ => Err(SurrealStorageError::TransactionSettlementPending { operation_id }),
                }
            }
        }
    }

    pub(crate) fn transaction_settlement_pending(&self, operation_id: uuid::Uuid) -> bool {
        self.inner.transaction_settlements.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()).contains_key(&operation_id)
    }

    async fn transaction_worker<T, E, F>(
        &self, invocation: TransactionInvocation, settlement_id: Option<uuid::Uuid>,
        owned_cancellation: Option<Arc<ContextOperationCancellation>>,
        settlement_mode: TransactionSettlementMode, operation: F,
    ) -> Result<T, E>
    where
        T: Send + 'static,
        E: From<SurrealStorageError> + Send + 'static,
        F: for<'a> FnOnce(SurrealTransactionContext<'a>) -> SurrealTransactionOperation<'a, T, E>
            + Send + 'static,
    {
        use futures::FutureExt;

        if let TransactionSettlementMode::RoutingCapture(expected) = &settlement_mode {
            let operation_id = expected.operation_id();
            if !matches!(&invocation,
                TransactionInvocation::Authenticated { operation_id: invoked, .. }
                    if *invoked == operation_id)
                || settlement_id != Some(operation_id)
            {
                return Err(E::from(SurrealStorageError::TransactionRetentionMismatch { operation_id }));
            }
            let settlements = self.inner.transaction_settlements.lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if !matches!(settlements.get(&operation_id),
                Some(Some(AuthenticatedTransactionRetention::Role { retention, phase }))
                    if matches!(phase, RoleTransactionPhase::CaptureReadPending)
                        && retention.matches_retention(expected))
            {
                return Err(E::from(SurrealStorageError::TransactionRetentionMismatch { operation_id }));
            }
        }

        if let TransactionSettlementMode::JobReadback(expected) = &settlement_mode {
            let operation_id = expected.operation_id();
            if !expected.belongs_to(self) || !matches!(&invocation,
                TransactionInvocation::Authenticated { operation_id: invoked, .. }
                    if *invoked == operation_id)
                || settlement_id != Some(operation_id)
            {
                return Err(E::from(SurrealStorageError::TransactionRetentionMismatch { operation_id }));
            }
            let settlements = self.inner.transaction_settlements.lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if !matches!(settlements.get(&operation_id),
                Some(Some(AuthenticatedTransactionRetention::JobAdmission { retention, phase, readback: Some(read) }))
                    if matches!(phase, JobAdmissionTransactionPhase::ReadbackReadPending)
                        && retention.matches_retention(expected.retention())
                        && Arc::ptr_eq(read, expected))
            {
                return Err(E::from(SurrealStorageError::TransactionRetentionMismatch { operation_id }));
            }
        }

        let association_retention = if let Some(operation_id) = settlement_id {
            let settlements = self.inner.transaction_settlements.lock().unwrap_or_else(|p| p.into_inner());
            match settlements.get(&operation_id) {
                Some(Some(AuthenticatedTransactionRetention::ValidatorAssociation(retention))) =>
                    Some((retention.clone(), retention.worker_phase())),
                _ => None,
            }
        } else { None };
        let association_worker_retention = association_retention.clone();
        let runcreate_retention = if let Some(operation_id) = settlement_id {
            let settlements = self.inner.transaction_settlements.lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            match settlements.get(&operation_id) {
                Some(Some(AuthenticatedTransactionRetention::OriginalRunCreate(retention))) => Some(retention.clone()),
                _ => None,
            }
        } else { None };
        if let Some(retention) = &runcreate_retention {
            let TransactionInvocation::Authenticated { scope, deadline, operation_id } = &invocation else {
                return Err(E::from(SurrealStorageError::TransactionAuthorityRequired));
            };
            if settlement_id != Some(*operation_id) || !matches!(&settlement_mode, TransactionSettlementMode::Mutation) {
                return Err(E::from(SurrealStorageError::TransactionRetentionMismatch { operation_id: *operation_id }));
            }
            let mut state = retention.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            if state.binding.is_some() {
                return Err(E::from(SurrealStorageError::TransactionRetentionMismatch { operation_id: *operation_id }));
            }
            let owner = state.owner.clone();
            state.binding = Some(Arc::new(OriginalRunCreateBinding {
                operation_id: *operation_id, storage: Arc::downgrade(&self.inner),
                owner, _scope: scope.clone(), _deadline: *deadline,
            }));
        }
        let read_window = if matches!(&settlement_mode, TransactionSettlementMode::WorkspaceArtifactCanonicalRead(_)) {
            match &invocation {
                TransactionInvocation::Authenticated { deadline, operation_id, .. } => Some((*deadline, *operation_id)),
                _ => return Err(E::from(SurrealStorageError::TransactionAuthorityRequired)),
            }
        } else { None };
        let storage = self.clone();
        let worker_invocation = invocation.clone();
        let worker_settlement_mode = settlement_mode.clone();
        let worker_admission = match &invocation {
            TransactionInvocation::WatcherAdmission { admission, .. } => Some(admission.clone()),
            _ => None,
        };
        let operation_purpose = MODEL_LANE_OPERATION_PURPOSE.try_with(|purpose| *purpose).ok();
        let operation_subject = MODEL_LANE_OPERATION_SUBJECT.try_with(Clone::clone).ok();
        let operation_fences = MODEL_LANE_FENCES.try_with(Clone::clone).ok();
        let operation_guards = MODEL_LANE_CURRENT_GUARDS.try_with(Arc::clone).ok();
        let (cancel_sender, mut cancelled) = oneshot::channel::<()>();
        let owned_settlement = owned_cancellation.is_some();

        #[cfg(feature = "test-utils")]
        let album_proof = settlement_id.and_then(|operation_id| {
            let settlements = self.inner.transaction_settlements.lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            match settlements.get(&operation_id) {
                Some(Some(AuthenticatedTransactionRetention::AlbumMembership(retained))) =>
                    retained.proof.clone().map(|proof| (retained.operation.clone(), proof)),
                _ => None,
            }
        });
        let authority_mutation = AUTHORITY_MUTATION_SUBMITTED.try_with(Clone::clone).ok();
        let worker_operation = async move {
            let transaction_storage = storage.clone();
            let transaction_runcreate_retention = runcreate_retention.clone();
            let transaction_association_retention = association_worker_retention.clone();
            let result = storage
                .with_lease(move |client| {
                    Box::pin(async move {
                        if read_window.is_some_and(|(deadline, _)| Instant::now() >= deadline) {
                            return Err(SurrealStorageError::TransactionCancelled);
                        }
                        // Begin must complete even if the caller disappears before the
                        // SDK has returned the transaction identifier needed to cancel.
                        let scoped = match &worker_settlement_mode {
                            TransactionSettlementMode::WorkspaceArtifactCanonicalRead(snapshot) => snapshot.as_ref().clone(),
                            _ => (*client).clone(),
                        };
                        if !matches!(&worker_settlement_mode, TransactionSettlementMode::WorkspaceArtifactCanonicalRead(_)) {
                        if let TransactionInvocation::Authenticated { scope, .. }
                            | TransactionInvocation::WatcherAdmission { scope, .. } = &worker_invocation {
                            if let TransactionInvocation::WatcherAdmission { admission, .. } = &worker_invocation {
                                admission.state.lock().await.stage = WatcherAdmissionStage::OrdinaryAuthentication;
                            }
                            observe_result("transaction_namespace",
                                scoped.use_ns(transaction_storage.config().namespace()).use_db(transaction_storage.config().database())).await?;
                        if read_window.is_some_and(|(deadline, _)| Instant::now() >= deadline) {
                            return Err(SurrealStorageError::TransactionCancelled);
                        }
                            observe_result("transaction_signin", scoped.signin(RecordSignin {
                                namespace: transaction_storage.config().namespace().to_owned(),
                                database: transaction_storage.config().database().to_owned(),
                                access: resource_authority::AUTHORITY_ACCESS_METHOD.to_owned(),
                                params: resource_authority::SigninParams {
                                    token_hash: hex::encode(sha2::Sha256::digest(scope.session_token.as_bytes())),
                                    channel_binding_hash: scope.channel_binding_hash.clone(),
                                },
                            })).await?;
                        }
                        if read_window.is_some_and(|(deadline, _)| Instant::now() >= deadline) {
                            return Err(SurrealStorageError::TransactionCancelled);
                        }
                        }
                        let callback_client = scoped.clone();
                        if let Some(operation_id) = settlement_id {
                            let mut settlements = transaction_storage.inner.transaction_settlements.lock()
                                .unwrap_or_else(|poisoned| poisoned.into_inner());
                            if let Some(Some(AuthenticatedTransactionRetention::WorkspaceArtifactPublication {
                                canonical_read_client, ..
                            })) = settlements.get_mut(&operation_id) {
                                if canonical_read_client.is_some() {
                                    return Err(SurrealStorageError::TransactionRetentionMismatch { operation_id });
                                }
                                *canonical_read_client = Some(Arc::new(callback_client.clone()));
                            }
                        }
                        if let TransactionInvocation::WatcherAdmission { scope, admission, .. } = &worker_invocation {
                            let installer = scoped.clone();
                            transaction_storage.install_watcher_admission(&scoped, installer, scope, admission).await?;
                            admission.state.lock().await.stage = WatcherAdmissionStage::TransactionBegin;
                        }
                        if read_window.is_some_and(|(deadline, _)| Instant::now() >= deadline) {
                            return Err(SurrealStorageError::TransactionCancelled);
                        }
                        let transaction = Arc::new(observe_result("transaction_begin", scoped.begin()).await?);
                        // The witness follows this SDK transaction through its borrowed
                        // callback and sole first finalizer, without another transaction Arc.
                        let mut runcreate_begin = transaction_runcreate_retention.as_ref().map(|retention| {
                            let state = retention.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                            OriginalRunCreateBeginWitness { binding: state.binding.as_ref().expect("bound original RunCreate").clone() }
                        });
                        let callback_deadline = match &worker_invocation {
                            TransactionInvocation::Existing => None,
                            TransactionInvocation::Authenticated { deadline, .. }
                            | TransactionInvocation::WatcherAdmission { deadline, .. } => Some(*deadline),
                        };
                        let outcome = {
                            let context = SurrealTransactionContext {
                                transaction: &transaction,
                            };
                            let callback_transaction = transaction.clone();
                            let callback_storage = transaction_storage.clone();
                            let callback =
                                std::panic::AssertUnwindSafe(
                                    async move {
                                        let callback = async move {
                                            match &worker_invocation {
                                                TransactionInvocation::Existing => operation(context).await,
                                                TransactionInvocation::Authenticated { scope, deadline, .. }
                                                | TransactionInvocation::WatcherAdmission { scope, deadline, .. } => {
                                                    let protected = ProtectedTransaction {
                                                        transaction: callback_transaction, client: callback_client,
                                                        storage: callback_storage, authority: scope.clone(),
                                                    };
                                                    let callback = PROTECTED_TRANSACTION.scope(protected,
                                                        RECORD_USER_SCOPE.scope(scope.clone(), OPERATION_DEADLINE.scope(*deadline,
                                                            async move { operation(context).await })));
                                                    let scoped_callback = async move {
                                                        if let Some(subject) = operation_subject {
                                                            MODEL_LANE_OPERATION_SUBJECT.scope(subject, callback).await
                                                        } else {
                                                            callback.await
                                                        }
                                                    };
                                                    if let Some(purpose) = operation_purpose {
                                                        MODEL_LANE_OPERATION_PURPOSE.scope(purpose, scoped_callback).await
                                                    } else {
                                                        scoped_callback.await
                                                    }
                                                }
                                            }
                                        };
                                        let guarded_callback = async move {
                                            if let Some(guards) = operation_guards {
                                                MODEL_LANE_CURRENT_GUARDS.scope(guards, callback).await
                                            } else {
                                                callback.await
                                            }
                                        };
                                        if let Some(fences) = operation_fences {
                                            MODEL_LANE_FENCES.scope(fences, guarded_callback).await
                                        } else {
                                            guarded_callback.await
                                        }
                                    },
                                )
                                .catch_unwind();
                            tokio::pin!(callback);
                            if let Some(deadline) = callback_deadline {
                                tokio::select! {
                                    biased;
                                    _ = &mut cancelled => None,
                                    _ = async {
                                        if let Some(cancellation) = &owned_cancellation { cancellation.cancelled().await; }
                                        else { std::future::pending::<()>().await; }
                                    } => None,
                                    _ = tokio::time::sleep_until(deadline.into()) => None,
                                    result = &mut callback => Some(result),
                                }
                            } else {
                                tokio::select! {
                                    biased;
                                    _ = &mut cancelled => None,
                                    result = &mut callback => Some(result),
                                }
                            }
                        };
                        // All callback-owned facades have now dropped. Keep the worker
                        // and its lease until any internal outstanding query borrow settles.
                        let mut owned_transaction = transaction;
                        let transaction = loop {
                            match Arc::try_unwrap(owned_transaction) {
                                Ok(transaction) => break transaction,
                                Err(transaction) => {
                                    owned_transaction = transaction;
                                    tokio::task::yield_now().await;
                                }
                            }
                        };
                        #[cfg(feature = "test-utils")]
                        if let (Some((operation, proof)), Some(deadline)) = (&album_proof, callback_deadline) {
                            let statement_timeout = transaction_storage.config().statement_timeout();
                            let pending_deadline = deadline + statement_timeout;
                            let release_deadline = pending_deadline.checked_add(statement_timeout)
                                .unwrap_or(pending_deadline);
                            proof.pause(operation.clone(),
                                release_deadline.saturating_duration_since(Instant::now())).await;
                        }
                        let budget_expired = callback_deadline.is_some_and(|deadline| Instant::now() >= deadline);
                        match outcome {
                            Some(Ok(Ok(value))) => {
                                let mut value = Some(value);
                                if let Some((retention, phase)) = &transaction_association_retention {
                                    retention.retain_worker_callback_value(*phase, &mut value);
                                }
                                if let Some(retention) = &transaction_runcreate_retention {
                                    let mut state = retention.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                                    if state.value.is_some() {
                                        return Err(SurrealStorageError::TransactionRetentionMismatch {
                                            operation_id: settlement_id.expect("original RunCreate operation"),
                                        });
                                    }
                                    state.value = value.take().map(|value| Box::new(value) as Box<dyn std::any::Any + Send>);
                                }
                                if let Some(operation_id) = settlement_id {
                                    let mut settlements = transaction_storage.inner.transaction_settlements.lock()
                                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                                    if let Some(Some(AuthenticatedTransactionRetention::WorkspaceArtifactPublication {
                                        completed_result, callback_succeeded, ..
                                    })) = settlements.get_mut(&operation_id) {
                                        if completed_result.is_some() {
                                            return Err(SurrealStorageError::TransactionRetentionMismatch { operation_id });
                                        }
                                        *callback_succeeded = true;
                                        *completed_result = value.take().map(|value| Box::new(value) as Box<dyn std::any::Any + Send>);
                                    }
                                }
                                if !matches!(
                                    cancelled.try_recv(),
                                    Err(oneshot::error::TryRecvError::Empty)
                                ) || budget_expired || owned_cancellation.as_ref().is_some_and(|state| state.is_cancelled()) {
                                    let mut retained_error = Some(E::from(SurrealStorageError::TransactionCancelled));
                                    transaction_storage.retain_workspace_artifact_error(settlement_id, &mut retained_error);
                                    retain_original_runcreate_error(transaction_runcreate_retention.as_ref(), &mut retained_error);
                                    authority_mutation_submitted();
                                    transaction
                                        .cancel()
                                        .await
                                        .map_err(SurrealStorageError::TransactionCancel)?;
                                    transaction_storage.record_workspace_artifact_owned_finalization(settlement_id, WorkspaceArtifactEngineFinalization::Rollback);
                                    seal_original_runcreate_finalization(transaction_runcreate_retention.as_ref(), &mut runcreate_begin, OriginalRunCreateFinalization::Rollback)?;
                                    authority_mutation_settled();
                                    return Ok((Err(retained_error.unwrap_or_else(|| E::from(SurrealStorageError::TransactionSettlementPending { operation_id: settlement_id.expect("retained publication operation") }))), ActualSettlement::CancelNoEffectObserved));
                                }
                                if let TransactionSettlementMode::WorkspaceArtifactCanonicalRead(_) = &worker_settlement_mode {
                                    transaction.cancel().await.map_err(SurrealStorageError::TransactionCancel)?;
                                    return Ok((value.ok_or_else(|| E::from(SurrealStorageError::TransactionAuthorityRequired)),
                                        ActualSettlement::WorkspaceArtifactCanonicalReadRollbackObserved));
                                }
                                if let TransactionSettlementMode::JobReadback(_) = &worker_settlement_mode {
                                    transaction.cancel().await
                                        .map_err(SurrealStorageError::TransactionCancel)?;
                                    return Ok((value.ok_or_else(|| E::from(SurrealStorageError::TransactionSettlementPending { operation_id: settlement_id.expect("retained publication operation") })), ActualSettlement::JobReadbackRollbackObserved));
                                }
                                if let TransactionSettlementMode::RoutingCapture(_) = &worker_settlement_mode {
                                    transaction.cancel().await
                                        .map_err(SurrealStorageError::TransactionCancel)?;
                                    return Ok((value.ok_or_else(|| E::from(SurrealStorageError::TransactionSettlementPending { operation_id: settlement_id.expect("retained publication operation") })), ActualSettlement::CaptureReadRollbackObserved));
                                }
                                // Once commit begins, await its outcome; an attached
                                // external mirror cannot be undone or replayed safely.
                                authority_mutation_submitted();
                                transaction
                                    .commit()
                                    .await
                                    .map_err(SurrealStorageError::TransactionCommit)?;
                                transaction_storage.record_workspace_artifact_owned_finalization(settlement_id, WorkspaceArtifactEngineFinalization::Commit);
                                seal_original_runcreate_finalization(transaction_runcreate_retention.as_ref(), &mut runcreate_begin, OriginalRunCreateFinalization::Commit)?;
                                Ok((value.ok_or_else(|| E::from(SurrealStorageError::TransactionSettlementPending { operation_id: settlement_id.expect("retained publication operation") })), ActualSettlement::CommitObserved))
                            }
                            Some(Ok(Err(error))) => {
                                let mut error = Some(error);
                                if let Some((retention, phase)) = &transaction_association_retention {
                                    retention.retain_worker_callback_error(*phase, &mut error);
                                }
                                transaction_storage.retain_workspace_artifact_error(settlement_id, &mut error);
                                retain_original_runcreate_error(transaction_runcreate_retention.as_ref(), &mut error);
                                authority_mutation_submitted();
                                transaction
                                    .cancel()
                                    .await
                                    .map_err(SurrealStorageError::TransactionCancel)?;
                                    transaction_storage.record_workspace_artifact_owned_finalization(settlement_id, WorkspaceArtifactEngineFinalization::Rollback);
                                    seal_original_runcreate_finalization(transaction_runcreate_retention.as_ref(), &mut runcreate_begin, OriginalRunCreateFinalization::Rollback)?;
                                authority_mutation_settled();
                                Ok((Err(error.unwrap_or_else(|| E::from(SurrealStorageError::TransactionSettlementPending { operation_id: settlement_id.expect("retained publication operation") }))), ActualSettlement::CancelNoEffectObserved))
                            }
                            Some(Err(panic_payload)) => {
                                if let Some((retention, phase)) = &transaction_association_retention {
                                    retention.retain_worker_callback_panic(*phase, panic_payload);
                                }
                                let mut error = Some(E::from(SurrealStorageError::TransactionCallbackPanicked));
                                transaction_storage.retain_workspace_artifact_error(settlement_id, &mut error);
                                retain_original_runcreate_error(transaction_runcreate_retention.as_ref(), &mut error);
                                authority_mutation_submitted();
                                transaction
                                    .cancel()
                                    .await
                                    .map_err(SurrealStorageError::TransactionCancel)?;
                                    transaction_storage.record_workspace_artifact_owned_finalization(settlement_id, WorkspaceArtifactEngineFinalization::Rollback);
                                    seal_original_runcreate_finalization(transaction_runcreate_retention.as_ref(), &mut runcreate_begin, OriginalRunCreateFinalization::Rollback)?;
                                authority_mutation_settled();
                                Ok((Err(error.unwrap_or_else(|| E::from(SurrealStorageError::TransactionSettlementPending { operation_id: settlement_id.expect("retained publication operation") }))), ActualSettlement::CallbackPanicRollbackObserved))
                            }
                            None => {
                                let mut retained_error = Some(E::from(if budget_expired {
                                    SurrealStorageError::StatementTimeout {
                                        waited_ms: transaction_storage.config().statement_timeout().as_millis(),
                                    }
                                } else { SurrealStorageError::TransactionCancelled }));
                                transaction_storage.retain_workspace_artifact_error(settlement_id, &mut retained_error);
                                retain_original_runcreate_error(transaction_runcreate_retention.as_ref(), &mut retained_error);
                                authority_mutation_submitted();
                                transaction
                                    .cancel()
                                    .await
                                    .map_err(SurrealStorageError::TransactionCancel)?;
                                    transaction_storage.record_workspace_artifact_owned_finalization(settlement_id, WorkspaceArtifactEngineFinalization::Rollback);
                                    seal_original_runcreate_finalization(transaction_runcreate_retention.as_ref(), &mut runcreate_begin, OriginalRunCreateFinalization::Rollback)?;
                                authority_mutation_settled();
                                Ok((Err(retained_error.unwrap_or_else(|| E::from(SurrealStorageError::TransactionSettlementPending { operation_id: settlement_id.expect("retained publication operation") }))), ActualSettlement::CancelNoEffectObserved))
                            }
                        }
                    })
                })
                .await;
            if let Some(admission) = &worker_admission {
                let observed = result.as_ref().map_or(ActualSettlement::Unobserved, |(_, observed)| *observed);
                admission.observe_settlement(observed).await;
            }
            if let Err(error) = &result {
                tracing::error!(error = %error, "embedded transaction controller failed");
            }
            if let Some(operation_id) = settlement_id {
                let observed = result.as_ref().map_or(ActualSettlement::Unobserved, |(_, observed)| *observed);
                let mut settlements = storage.inner.transaction_settlements.lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                if let TransactionSettlementMode::JobReadback(expected) = &settlement_mode {
                    if matches!(observed, ActualSettlement::JobReadbackRollbackObserved) {
                        match settlements.get_mut(&operation_id) {
                            Some(Some(AuthenticatedTransactionRetention::JobAdmission { retention, phase, readback: Some(read) }))
                                if matches!(phase, JobAdmissionTransactionPhase::ReadbackReadPending)
                                    && retention.matches_retention(expected.retention())
                                    && Arc::ptr_eq(read, expected) => {
                                *phase = JobAdmissionTransactionPhase::ReadbackObserved;
                            }
                            _ => return Err(E::from(SurrealStorageError::TransactionRetentionMismatch { operation_id })),
                        }
                    }
                    return result.map_err(E::from)?.0;
                }
                if let TransactionSettlementMode::RoutingCapture(expected) = &settlement_mode {
                    if matches!(observed, ActualSettlement::CaptureReadRollbackObserved) {
                        match settlements.get_mut(&operation_id) {
                            Some(Some(AuthenticatedTransactionRetention::Role { retention, phase }))
                                if matches!(phase, RoleTransactionPhase::CaptureReadPending)
                                    && expected.operation_id() == operation_id
                                    && retention.matches_retention(expected) => {
                                *phase = RoleTransactionPhase::CaptureReadObserved;
                            }
                            _ => return Err(E::from(SurrealStorageError::TransactionRetentionMismatch { operation_id })),
                        }
                    }
                    return result.map_err(E::from)?.0;
                }
                let remove = match settlements.get_mut(&operation_id) {
                    Some(Some(AuthenticatedTransactionRetention::WorkspaceArtifactPublication {
                        settlement, worker_finished, ..
                    })) => {
                        *settlement = observed;
                        *worker_finished = true;
                        false
                    }

                    Some(Some(AuthenticatedTransactionRetention::AlbumMembership(retained))) => {
                        retained.settlement = observed.into();
                        #[cfg(feature = "test-utils")]
                        if let Some(proof) = &retained.proof {
                            proof.observe(retained.settlement);
                        }
                        false
                    }
                    Some(Some(AuthenticatedTransactionRetention::JobAdmission { phase, .. })) => match observed {
                        ActualSettlement::CommitObserved => {
                            *phase = JobAdmissionTransactionPhase::CommitObservedReadbackPending;
                            false
                        }
                        ActualSettlement::CancelNoEffectObserved =>
                            matches!(phase, JobAdmissionTransactionPhase::EnginePending),
                        ActualSettlement::Unobserved
                        | ActualSettlement::CallbackPanicRollbackObserved
                        | ActualSettlement::JobReadbackRollbackObserved
                        | ActualSettlement::CaptureReadRollbackObserved
                        | ActualSettlement::WorkspaceArtifactCanonicalReadRollbackObserved => false,
                    },
                    Some(Some(AuthenticatedTransactionRetention::Role { retention, phase })) => match observed {
                        ActualSettlement::CommitObserved => {
                            *phase = RoleTransactionPhase::CommitObservedPublicationPending;
                            false
                        }
                        ActualSettlement::CancelNoEffectObserved
                        | ActualSettlement::CallbackPanicRollbackObserved => !retention.retains_capture_on_failure(),
                        ActualSettlement::Unobserved => false,
                        ActualSettlement::CaptureReadRollbackObserved
                        | ActualSettlement::JobReadbackRollbackObserved
                        | ActualSettlement::WorkspaceArtifactCanonicalReadRollbackObserved => false,
                    },
                    Some(Some(AuthenticatedTransactionRetention::ValidatorAssociation(retention))) => {
                        use resource_authority::ValidatorAssociationEngineObservation as Observation;
                        let observation = match observed {
                            ActualSettlement::Unobserved => Observation::Unobserved,
                            ActualSettlement::CommitObserved => Observation::CommitObserved,
                            ActualSettlement::CancelNoEffectObserved => Observation::CancelNoEffectObserved,
                            ActualSettlement::CallbackPanicRollbackObserved => Observation::CallbackPanicRollbackObserved,
                            ActualSettlement::CaptureReadRollbackObserved
                            | ActualSettlement::JobReadbackRollbackObserved
                            | ActualSettlement::WorkspaceArtifactCanonicalReadRollbackObserved =>
                                return Err(E::from(SurrealStorageError::TransactionRetentionMismatch { operation_id })),
                        };
                        let Some((expected, phase)) = &association_worker_retention else {
                            return Err(E::from(SurrealStorageError::TransactionRetentionMismatch { operation_id }));
                        };
                        if !retention.matches_retention(expected) {
                            return Err(E::from(SurrealStorageError::TransactionRetentionMismatch { operation_id }));
                        }
                        retention.observe_worker_phase(*phase, observation).map_err(E::from)?;
                        false
                    }
                    Some(Some(AuthenticatedTransactionRetention::OriginalRunCreate(_))) => false,
                    // Preserve the existing Activation and unretained settlement behavior.
                    Some(Some(AuthenticatedTransactionRetention::FirstOwnerWorkspace(_)))
                    | Some(Some(AuthenticatedTransactionRetention::FirstOwnerWorkspaceRecovery(_)))
                    | Some(Some(AuthenticatedTransactionRetention::FirstOwnerSetup(_)))
                    | Some(Some(AuthenticatedTransactionRetention::FirstOwnerConfirmation(_)))
                    | Some(Some(AuthenticatedTransactionRetention::AdministratorPasswordLogin(_)))
                    | Some(Some(AuthenticatedTransactionRetention::AdministratorSessionExchange(_))) => false,
                    Some(_) => result.is_ok(),
                    None => false,
                };
                if remove {
                    settlements.remove(&operation_id);
                    tracing::info!(%operation_id, "authenticated transaction settlement observed");
                } else {
                    tracing::error!(%operation_id, "authenticated transaction retains custody pending canonical resolution or publication");
                }
            }
            let final_result = result.map_err(E::from).and_then(|(value, _)| value);
            if let Some((retention, phase)) = &association_worker_retention {
                retention.retain_worker_final(*phase, final_result);
                return Err(E::from(SurrealStorageError::TransactionSettlementPending {
                    operation_id: retention.operation_id(),
                }));
            }
            if let Some(retention) = &runcreate_retention {
                let mut state = retention.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                state.finished = true;
                if let Err(error) = final_result {
                    if state.error.is_none()
                        && !(state.finalized == Some(OriginalRunCreateFinalization::Commit) && state.value.is_some())
                    {
                        state.error = Some(Box::new(error));
                    }
                }
                return Err(E::from(SurrealStorageError::TransactionSettlementPending {
                    operation_id: settlement_id.expect("original RunCreate operation"),
                }));
            }
            if let Some(operation_id) = settlement_id {
                let mut settlements = storage.inner.transaction_settlements.lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                if let Some(Some(AuthenticatedTransactionRetention::WorkspaceArtifactPublication {
                    completed_error, owned_finalization, callback_succeeded, ..
                })) = settlements.get_mut(&operation_id) {
                    if !(matches!(owned_finalization, Some(WorkspaceArtifactEngineFinalization::Commit)) && *callback_succeeded) {
                        if let Err(error) = final_result {
                            if completed_error.is_none() { *completed_error = Some(Box::new(error)); }
                        }
                    }
                    return Err(E::from(SurrealStorageError::TransactionSettlementPending { operation_id }));
                }
            }
            final_result
        };
        let mut worker = tokio::spawn(async move {
            let execution = async move {
                if let Some(marker) = authority_mutation {
                    AUTHORITY_MUTATION_SUBMITTED.scope(marker, worker_operation).await
                } else {
                    worker_operation.await
                }
            };
            if let Some((deadline, _)) = read_window {
                OPERATION_DEADLINE.scope(deadline, execution).await
            } else {
                execution.await
            }
        });
        if let Some((retention, phase)) = &association_retention {
            if *phase != resource_authority::ValidatorAssociationWorkerPhase::Designation {
                let retained_task = retention.retain_initial_worker_task(*phase, worker).map_err(E::from)?;
                let mut owned_task = retained_task.lock().await;
                // SAME task mutex excludes the external original-handle observer.
                if retention.initial_worker_join_observed(*phase) {
                    drop(cancel_sender);
                    return Err(E::from(SurrealStorageError::TransactionSettlementPending { operation_id: retention.operation_id() }));
                }
                let observed = (&mut *owned_task).await;
                retention.mark_initial_worker_join_observed(*phase);
                let result = match observed {
                    Ok(result) => result,
                    Err(error) => {
                        retention.retain_worker_join(*phase, error);
                        Err(E::from(SurrealStorageError::TransactionSettlementPending { operation_id: retention.operation_id() }))
                    }
                };
                drop(cancel_sender);
                return result;
            }
        }
        if let Some((deadline, operation_id)) = read_window {
            match tokio::time::timeout_at(deadline.into(), &mut worker).await {
                Ok(result) => {
                    drop(cancel_sender);
                    result.map_err(|error| E::from(SurrealStorageError::TransactionWorker(error.to_string())))?
                }
                Err(_) => {
                    drop(cancel_sender);
                    Err(E::from(SurrealStorageError::TransactionSettlementPending { operation_id }))
                }
            }
        } else if owned_settlement {
            // The completion supervisor retains the worker, transaction lease and
            // final guards until the actual engine outcome is observed.
            let result = match worker.await {
                Ok(result) => result,
                Err(error) => {
                    if let Some((retention, phase)) = &association_retention {
                        retention.retain_worker_join(*phase, error);
                        return Err(E::from(SurrealStorageError::TransactionSettlementPending {
                            operation_id: retention.operation_id(),
                        }));
                    }
                    return Err(E::from(SurrealStorageError::TransactionWorker(error.to_string())));
                }
            };
            drop(cancel_sender);
            result
        } else if let TransactionInvocation::Authenticated { deadline, operation_id, .. }
            | TransactionInvocation::WatcherAdmission { deadline, operation_id, .. } = invocation {
            match tokio::time::timeout_at((deadline + self.config().statement_timeout()).into(), &mut worker).await {
                Ok(result) => {
                    drop(cancel_sender);
                    result.map_err(|error| E::from(SurrealStorageError::TransactionWorker(error.to_string())))?
                }
                Err(_) => {
                    // Dropping only the caller-side cancellation sender requests cancellation.
                    // The worker retains its lease and observes begin/cancel/commit settlement.
                    drop(cancel_sender);
                    Err(E::from(SurrealStorageError::TransactionSettlementPending { operation_id }))
                }
            }
        } else {
            let result = worker.await
                .map_err(|error| E::from(SurrealStorageError::TransactionWorker(error.to_string())))?;
            drop(cancel_sender);
            result
        }
    }

    /// Runs a domain storage operation under the same lifecycle lease while
    /// preserving its typed [`crate::storage::StorageError`] as the inner
    /// result. The outer result remains reserved for lifecycle failures.
    pub(crate) async fn with_storage_operation<T, F>(
        &self,
        operation: F,
    ) -> Result<StorageResult<T>, SurrealStorageError>
    where
        T: Send,
        F: for<'a> FnOnce(SurrealDataContext<'a>) -> SurrealStorageOperation<'a, T>
            + Send
            + 'static,
    {
        self.with_data_operation(move |database| {
            Box::pin(async move { Ok(operation(database).await) })
        })
        .await
    }

    #[cfg(any(test, feature = "test-utils"))]
    pub(crate) async fn test_admin_query(
        &self,
        statement: String,
    ) -> Result<surrealdb::IndexedResults, SurrealStorageError> {
        self.with_admin_operation(move |admin| {
            Box::pin(async move { admin.query(statement).await })
        })
        .await
    }

    #[cfg(test)]
    pub(crate) async fn test_admin_query_bound<B>(
        &self,
        statement: String,
        bindings: B,
    ) -> Result<surrealdb::IndexedResults, SurrealStorageError>
    where
        B: surrealdb::types::SurrealValue + Send + 'static,
    {
        self.with_admin_operation(move |admin| {
            Box::pin(async move { admin.query_bound(statement, bindings).await })
        })
        .await
    }

    async fn with_admin_operation<T, F>(&self, operation: F) -> Result<T, SurrealStorageError>
    where
        T: Send,
        F: for<'a> FnOnce(SurrealAdminContext<'a>) -> SurrealOperation<'a, T>,
    {
        let config = self.config().clone();
        self.with_lease(move |client| operation(SurrealAdminContext { client, config }))
            .await
    }

    async fn with_lease<T, F>(&self, operation: F) -> Result<T, SurrealStorageError>
    where
        T: Send,
        F: for<'a> FnOnce(&'a SurrealClient) -> SurrealOperation<'a, T>,
    {
        if self.inner.lifecycle.load(Ordering::Acquire) != LIFECYCLE_OPEN {
            return Err(SurrealStorageError::Closed);
        }
        let guard = observe("authority_lease_wait", 0, self.inner.client.read(), |_| {
            false
        })
        .await;
        if self.inner.lifecycle.load(Ordering::Acquire) != LIFECYCLE_OPEN {
            return Err(SurrealStorageError::Closed);
        }
        let client = guard.as_ref().ok_or(SurrealStorageError::Closed)?;
        let _lease_count = self.inner.enter_lease();
        INSIDE_SURREAL_OPERATION
            .scope((), observe_result("authority_operation", operation(client)))
            .await
    }

    /// Runs one logical storage operation - including every retry attempt it
    /// makes - under a single wall-clock budget (MT-142 AC-142-5).
    ///
    /// Without this the per-attempt `statement_timeout` multiplies by the retry
    /// policy's `maximum_attempts` (8 x 300 s = 40 minutes), because
    /// `retry`'s `maximum_elapsed` deliberately bounds only the sleeps between
    /// attempts: an in-flight attempt is never raced against a deadline, so an
    /// acknowledged commit can never be dropped (see `retry.rs`). Scoping the
    /// deadline here keeps that property while bounding the whole operation:
    /// every `with_data_operation` underneath clamps its bound to what is left,
    /// so the last attempt cannot overshoot.
    pub async fn with_operation_deadline<T, F>(deadline: Instant, operation: F) -> T
    where
        F: Future<Output = T>,
    {
        OPERATION_DEADLINE.scope(deadline, operation).await
    }

    /// The configured statement timeout, clamped to the remaining budget of an
    /// enclosing [`Self::with_operation_deadline`] scope.
    fn effective_statement_timeout(&self) -> Duration {
        let configured = self.inner.config.statement_timeout;
        OPERATION_DEADLINE
            .try_with(|deadline| deadline.saturating_duration_since(Instant::now()))
            .map_or(configured, |remaining| configured.min(remaining))
    }

    /// Leases held right now (operations inside the engine window).
    pub fn leases_in_flight(&self) -> usize {
        self.inner.leases_in_flight.load(Ordering::Acquire)
    }

    /// Highest number of simultaneously held leases since the last
    /// [`Self::reset_lease_high_water`]: the engine-window concurrency measure
    /// for `maximum_concurrent_operations` (review R2-1-3). A store serialized
    /// by a hidden global mutex cannot exceed 1 here.
    pub fn lease_high_water(&self) -> usize {
        self.inner.lease_high_water.load(Ordering::Acquire)
    }

    /// Restarts the high-water mark from the current in-flight count.
    pub fn reset_lease_high_water(&self) {
        self.inner
            .lease_high_water
            .store(self.leases_in_flight(), Ordering::Release);
    }

    pub async fn is_closed(&self) -> bool {
        self.inner.lifecycle.load(Ordering::Acquire) == LIFECYCLE_CLOSED
    }

    pub fn is_accepting_operations(&self) -> bool {
        self.inner.lifecycle.load(Ordering::Acquire) == LIFECYCLE_OPEN
    }

    /// Child token that fires once shutdown's drain grace expires (MT-142
    /// AC-142-7). Retry loops and keyed-lock waits observe it; work already
    /// blocked inside the engine is not interruptible and is drained instead.
    pub fn cancellation_token(&self) -> CancellationToken {
        self.inner.child_cancellation_token()
    }

    /// Report of the most recent shutdown attempt, if one ran.
    pub fn last_shutdown_report(&self) -> Option<ShutdownReport> {
        self.inner.shutdown_report()
    }

    /// [`Self::shutdown`] plus the typed report of the completed close.
    pub async fn shutdown_with_report(&self) -> Result<ShutdownReport, SurrealStorageError> {
        self.shutdown().await?;
        self.last_shutdown_report().ok_or_else(|| {
            SurrealStorageError::Shutdown(Arc::from(
                "shutdown completed without recording a report",
            ))
        })
    }

    /// Flushes prior work through a query barrier and closes every wrapper clone.
    ///
    /// SurrealDB 3.2 exposes no explicit embedded-engine close method. This type
    /// therefore never hands out a cloned handle and drops its sole handle after
    /// draining operations. On Windows, completion is published only after an
    /// exclusive open of RocksDB's `LOCK` file proves the prior engine handle is
    /// gone. The locked SDK exposes no equivalent portable completion signal, so
    /// non-Windows builds do not claim that stronger release proof.
    /// Repeated and concurrent shutdown calls are intentionally idempotent.
    pub async fn shutdown(&self) -> Result<(), SurrealStorageError> {
        if INSIDE_SURREAL_OPERATION.try_with(|_| ()).is_ok() {
            return Err(SurrealStorageError::ReentrantShutdown);
        }

        let mut receiver = {
            let mut coordinator = self.inner.shutdown.lock().await;
            match &*coordinator {
                ShutdownCoordinatorState::Closed => return Ok(()),
                ShutdownCoordinatorState::Failed { error } => {
                    return Err(SurrealStorageError::Shutdown(Arc::clone(error)));
                }
                ShutdownCoordinatorState::Closing { receiver } => receiver.clone(),
                ShutdownCoordinatorState::Open => {
                    let (sender, receiver) = watch::channel(None);
                    *coordinator = ShutdownCoordinatorState::Closing {
                        receiver: receiver.clone(),
                    };
                    self.inner
                        .lifecycle
                        .store(LIFECYCLE_CLOSING, Ordering::Release);
                    let inner = Arc::clone(&self.inner);
                    tokio::spawn(async move {
                        SurrealStorage::run_shutdown(inner, sender).await;
                    });
                    receiver
                }
            }
        };

        let wait = async {
            loop {
                if let Some(result) = receiver.borrow().clone() {
                    return result.map_err(SurrealStorageError::Shutdown);
                }
                receiver.changed().await.map_err(|_| {
                    SurrealStorageError::Shutdown(Arc::from(
                        "shutdown coordinator exited without publishing a result",
                    ))
                })?;
            }
        };
        match tokio::time::timeout(self.inner.config.shutdown_wait, wait).await {
            Ok(result) => result,
            Err(_) => Err(SurrealStorageError::ShutdownStillInProgress {
                waited_ms: self.inner.config.shutdown_wait.as_millis(),
            }),
        }
    }

    async fn run_shutdown(
        inner: Arc<SurrealStorageInner>,
        sender: watch::Sender<Option<SharedShutdownResult>>,
    ) {
        let attempt = SurrealStorage::perform_shutdown(&inner).await;
        let terminal_failure = matches!(&attempt, Err(ShutdownAttemptError::Terminal(_)));
        let drain_timed_out = matches!(&attempt, Err(ShutdownAttemptError::DrainTimedOut(_)));
        let result = attempt.map_err(|failure| {
            let error = match failure {
                ShutdownAttemptError::Retryable(error)
                | ShutdownAttemptError::DrainTimedOut(error)
                | ShutdownAttemptError::Terminal(error) => error,
            };
            Arc::<str>::from(error.to_string())
        });

        let mut coordinator = inner.shutdown.lock().await;
        match &result {
            Ok(()) => {
                inner.lifecycle.store(LIFECYCLE_CLOSED, Ordering::Release);
                *coordinator = ShutdownCoordinatorState::Closed;
            }
            Err(_) if drain_timed_out => {
                // Admission stays stopped (lifecycle CLOSING) and cooperative
                // work stays cancelled; the coordinator reopens so a later
                // shutdown call retries the bounded close.
                *coordinator = ShutdownCoordinatorState::Open;
            }
            Err(error) if terminal_failure => {
                // The client has already been taken and dropped. Preserve that
                // truth for operations and every subsequent shutdown caller;
                // reopening this wrapper would claim ownership of a client that
                // no longer exists.
                inner.lifecycle.store(LIFECYCLE_CLOSED, Ordering::Release);
                *coordinator = ShutdownCoordinatorState::Failed {
                    error: Arc::clone(error),
                };
            }
            Err(_) => {
                // The flush barrier failed before the sole client was taken, so
                // the wrapper still owns a usable client and shutdown may retry.
                inner.reset_cancellation();
                inner.lifecycle.store(LIFECYCLE_OPEN, Ordering::Release);
                *coordinator = ShutdownCoordinatorState::Open;
            }
        }
        let _ = sender.send(Some(result));
    }

    /// MT-142 AC-142-7 close sequence, bounded end to end by `shutdown_wait`:
    /// admission already stopped (`CLOSING`); wait up to `drain_grace` for
    /// every lease; on expiry fire the cooperative cancellation token so retry
    /// loops and keyed-lock waits return `Closed`, then wait for the remaining
    /// engine-bound leases (every data operation is itself bounded by
    /// `statement_timeout`) only until the `shutdown_wait` budget is spent
    /// (`DrainTimedOut`, typed `ShutdownStillInProgress`); flush, drop the sole
    /// handle, and prove the engine released the store within the remaining
    /// budget (`EngineReleaseUnproven` otherwise).
    async fn perform_shutdown(inner: &SurrealStorageInner) -> Result<(), ShutdownAttemptError> {
        let started = Instant::now();
        let budget = inner.config.shutdown_wait;
        let mut report = ShutdownReport {
            drained: true,
            cancelled: false,
            elapsed: Duration::ZERO,
        };
        let guard = match tokio::time::timeout(inner.config.drain_grace, inner.client.write()).await
        {
            Ok(guard) => guard,
            Err(_) => {
                report.drained = false;
                report.cancelled = true;
                inner.cancel_operations();
                tracing::warn!(
                    target: "handshake_core::storage::surreal",
                    drain_grace_ms = u64::try_from(inner.config.drain_grace.as_millis())
                        .unwrap_or(u64::MAX),
                    "embedded store drain grace expired; cooperative work cancelled, waiting for engine-bound leases"
                );
                let remaining = budget.saturating_sub(started.elapsed());
                match tokio::time::timeout(remaining, inner.client.write()).await {
                    Ok(guard) => guard,
                    Err(_) => {
                        report.elapsed = started.elapsed();
                        inner.record_shutdown_report(report);
                        return Err(ShutdownAttemptError::DrainTimedOut(
                            SurrealStorageError::ShutdownStillInProgress {
                                waited_ms: started.elapsed().as_millis(),
                            },
                        ));
                    }
                }
            }
        };
        let release_budget = budget.saturating_sub(started.elapsed());
        let outcome = SurrealStorage::close_client(inner, guard, release_budget).await;
        report.elapsed = started.elapsed();
        inner.record_shutdown_report(report);
        outcome
    }

    async fn close_client(
        inner: &SurrealStorageInner,
        mut guard: RwLockWriteGuard<'_, Option<SurrealClient>>,
        release_budget: Duration,
    ) -> Result<(), ShutdownAttemptError> {
        let Some(client) = guard.as_ref() else {
            return Ok(());
        };
        client
            .query("RETURN true;")
            .await
            .map_err(SurrealStorageError::from)
            .and_then(|response| response.check().map_err(SurrealStorageError::from))
            .map_err(ShutdownAttemptError::Retryable)?;
        let Some(client) = guard.take() else {
            return Ok(());
        };
        drop(guard);
        drop(client);
        wait_for_engine_release(&inner.config.path, release_budget)
            .await
            .map_err(ShutdownAttemptError::Terminal)?;
        Ok(())
    }
}

#[cfg(windows)]
async fn wait_for_engine_release(
    store_path: &Path,
    budget: Duration,
) -> Result<(), SurrealStorageError> {
    use std::{fs::OpenOptions, os::windows::fs::OpenOptionsExt};

    const ERROR_SHARING_VIOLATION: i32 = 32;
    const ERROR_LOCK_VIOLATION: i32 = 33;
    const INITIAL_BACKOFF: Duration = Duration::from_millis(5);
    const MAX_BACKOFF: Duration = Duration::from_millis(250);

    let lock_path = store_path.join("LOCK");
    let mut backoff = INITIAL_BACKOFF;
    let started = Instant::now();

    loop {
        match OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .share_mode(0)
            .open(&lock_path)
        {
            Ok(probe) => {
                // RocksDB opens this file with the same zero-sharing contract.
                // Acquiring it proves the prior WinFileLock CloseHandle ran.
                drop(probe);
                return Ok(());
            }
            Err(error)
                if matches!(
                    error.raw_os_error(),
                    Some(ERROR_SHARING_VIOLATION) | Some(ERROR_LOCK_VIOLATION)
                ) => {}
            Err(error) => {
                tracing::warn!(
                    lock_path = %lock_path.display(),
                    error = %error,
                    "unexpected error while proving RocksDB lock release; shutdown is terminally closed without a release proof"
                );
                return Err(SurrealStorageError::Io {
                    path: lock_path,
                    source: error,
                });
            }
        }

        let waited = started.elapsed();
        if waited >= budget {
            return Err(SurrealStorageError::EngineReleaseUnproven {
                waited_ms: waited.as_millis(),
            });
        }
        tokio::time::sleep(backoff.min(budget.saturating_sub(waited))).await;
        backoff = backoff.saturating_mul(2).min(MAX_BACKOFF);
    }
}

#[cfg(not(windows))]
async fn wait_for_engine_release(
    _store_path: &Path,
    _budget: Duration,
) -> Result<(), SurrealStorageError> {
    // The locked SDK provides no router join or datastore-close acknowledgement.
    // A yield preserves the prior non-Windows behavior without claiming proof
    // equivalent to the Windows zero-sharing LOCK acquisition above.
    tokio::task::yield_now().await;
    Ok(())
}

fn absolute_path(path: PathBuf) -> Result<PathBuf, SurrealStorageError> {
    if path.is_absolute() {
        return Ok(path);
    }

    env::current_dir()
        .map(|current| current.join(&path))
        .map_err(|source| SurrealStorageError::Io { path, source })
}

fn validate_supported_storage_path(path: &Path) -> Result<(), SurrealStorageError> {
    let endpoint_path =
        path.to_str()
            .ok_or_else(|| SurrealStorageError::IncompatibleEndpointPath {
                path: path.to_path_buf(),
                reason: "the storage path is not valid Unicode",
            })?;

    #[cfg(windows)]
    if let Some(std::path::Component::Prefix(prefix)) = path.components().next() {
        match prefix.kind() {
            std::path::Prefix::UNC(_, _) | std::path::Prefix::VerbatimUNC(_, _) => {
                return Err(SurrealStorageError::IncompatibleEndpointPath {
                    path: path.to_path_buf(),
                    reason: "Windows network-share storage roots are not supported by the locked SurrealDB 3.2 and RocksDB integration",
                });
            }
            std::path::Prefix::DeviceNS(_) | std::path::Prefix::Verbatim(_) => {
                return Err(SurrealStorageError::IncompatibleEndpointPath {
                    path: path.to_path_buf(),
                    reason: "Windows device namespace storage roots are not supported",
                });
            }
            std::path::Prefix::VerbatimDisk(_) => {
                return Err(SurrealStorageError::IncompatibleEndpointPath {
                    path: path.to_path_buf(),
                    reason: "the path retains an SDK-incompatible Windows verbatim drive prefix",
                });
            }
            std::path::Prefix::Disk(_) => {}
        }
    }

    // SurrealDB 3.2 treats the first `?` in every local endpoint as the start
    // of query parameters. Validate before filesystem creation and again after
    // canonicalization so this delimiter can never redirect the storage root.
    if endpoint_path.contains('?') {
        return Err(SurrealStorageError::IncompatibleEndpointPath {
            path: path.to_path_buf(),
            reason: "the path contains the SurrealDB 3.2 endpoint query delimiter `?`",
        });
    }

    Ok(())
}

#[cfg(all(test, windows))]
mod windows_path_tests {
    use std::{ffi::OsString, os::windows::ffi::OsStringExt};

    use super::*;

    #[test]
    fn sdk_endpoint_path_accepts_ordinary_absolute_drive_path() {
        assert!(validate_supported_storage_path(Path::new(r"C:\handshake\data")).is_ok());
    }

    #[test]
    fn sdk_endpoint_path_rejects_unc_device_and_verbatim_prefixes() {
        assert!(matches!(
            validate_supported_storage_path(Path::new(r"\\server\share\handshake\data")),
            Err(SurrealStorageError::IncompatibleEndpointPath { .. })
        ));
        assert!(matches!(
            validate_supported_storage_path(Path::new(r"\\?\UNC\server\share\handshake\data")),
            Err(SurrealStorageError::IncompatibleEndpointPath { .. })
        ));
        assert!(matches!(
            validate_supported_storage_path(Path::new(r"\\?\C:\handshake\data")),
            Err(SurrealStorageError::IncompatibleEndpointPath { .. })
        ));
        assert!(matches!(
            validate_supported_storage_path(Path::new(r"\\.\C:\handshake\data")),
            Err(SurrealStorageError::IncompatibleEndpointPath { .. })
        ));
    }

    #[tokio::test]
    async fn open_rejects_query_delimiter_before_creating_directory() {
        let temp = tempfile::tempdir().expect("create temporary root");
        let path = temp.path().join("unsafe?query");
        let config = SurrealStorageConfig::for_store_path(&path).expect("configure absolute path");
        assert!(matches!(
            SurrealStorage::open(config).await,
            Err(SurrealStorageError::IncompatibleEndpointPath { .. })
        ));
        assert!(!path.exists(), "rejected path must not be created");
    }

    #[tokio::test]
    async fn open_rejects_non_unicode_path_before_creating_directory() {
        let temp = tempfile::tempdir().expect("create temporary root");
        let mut path = temp.path().to_path_buf();
        path.push(OsString::from_wide(&[b'x' as u16, 0xD800]));
        let config = SurrealStorageConfig::for_store_path(&path).expect("configure absolute path");
        assert!(matches!(
            SurrealStorage::open(config).await,
            Err(SurrealStorageError::IncompatibleEndpointPath { .. })
        ));
        assert!(!path.exists(), "rejected path must not be created");
    }

    #[tokio::test]
    async fn engine_release_waits_for_the_windows_exclusive_lock_to_be_released() {
        use std::{fs::OpenOptions, os::windows::fs::OpenOptionsExt};

        let temp = tempfile::tempdir().expect("create temporary root");
        let lock_path = temp.path().join("LOCK");
        let held_lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .share_mode(0)
            .open(&lock_path)
            .expect("hold exclusive RocksDB-style lock");

        let store_path = temp.path().to_path_buf();
        let mut waiter = tokio::spawn(async move {
            wait_for_engine_release(&store_path, Duration::from_secs(30)).await
        });
        assert!(
            tokio::time::timeout(Duration::from_millis(50), &mut waiter)
                .await
                .is_err(),
            "release proof must remain pending while the exclusive handle is held"
        );

        drop(held_lock);
        tokio::time::timeout(Duration::from_secs(2), waiter)
            .await
            .expect("release proof should finish after CloseHandle")
            .expect("release proof task should not panic")
            .expect("exclusive release probe should succeed");
    }

    /// MT-142 AC-142-7 / review R2-1-4(ii): the LOCK probe is bounded, so a
    /// never-released engine handle yields a typed outcome instead of an
    /// unbounded loop.
    #[tokio::test]
    async fn engine_release_probe_gives_up_within_its_budget() {
        use std::{fs::OpenOptions, os::windows::fs::OpenOptionsExt};

        let temp = tempfile::tempdir().expect("create temporary root");
        let lock_path = temp.path().join("LOCK");
        let _held_lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .share_mode(0)
            .open(&lock_path)
            .expect("hold exclusive RocksDB-style lock");

        let started = Instant::now();
        let outcome = wait_for_engine_release(temp.path(), Duration::from_millis(120)).await;
        assert!(
            matches!(
                outcome,
                Err(SurrealStorageError::EngineReleaseUnproven { .. })
            ),
            "held lock must yield the typed unproven outcome, got {outcome:?}"
        );
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "probe must return inside its budget, took {:?}",
            started.elapsed()
        );
    }

    #[tokio::test]
    async fn unexpected_release_probe_error_terminally_closes_the_wrapper() {
        let temp = tempfile::tempdir().expect("create temporary root");
        let actual_store_path = temp.path().join("actual-store");
        let config =
            SurrealStorageConfig::for_store_path(&actual_store_path).expect("configure store");
        let mut storage = SurrealStorage::open(config).await.expect("open storage");

        // Redirect only the post-drop proof path to a missing parent. This
        // deterministically produces a non-sharing Windows error after the
        // actual engine client has already been taken and dropped.
        let missing_store_path = temp.path().join("missing-parent").join("store");
        Arc::get_mut(&mut storage.inner)
            .expect("test owns the only inner reference")
            .config
            .path = missing_store_path;

        let first_error = tokio::time::timeout(Duration::from_secs(2), storage.shutdown())
            .await
            .expect("unexpected probe error must not retry forever")
            .expect_err("shutdown must report the missing release proof")
            .to_string();
        assert!(
            first_error.contains("failed to prepare embedded database path"),
            "unexpected terminal shutdown error: {first_error}"
        );
        assert!(storage.is_closed().await);
        assert!(!storage.is_accepting_operations());
        assert!(storage.inner.client.read().await.is_none());

        let operation_result: Result<(), SurrealStorageError> = storage
            .with_data_operation(|_| Box::pin(async { Ok(()) }))
            .await;
        assert!(matches!(operation_result, Err(SurrealStorageError::Closed)));

        let repeated_error = tokio::time::timeout(Duration::from_millis(100), storage.shutdown())
            .await
            .expect("terminal shutdown result must be immediately reusable")
            .expect_err("terminal shutdown failure must remain observable")
            .to_string();
        assert_eq!(repeated_error, first_error);
    }
}



#[derive(Clone, PartialEq, SurrealValue)]
pub(crate) struct AlbumMutationLedgerExpectation {
    pub(crate) ledger_id: surrealdb::types::RecordId,
    pub(crate) event_id: String,
    pub(crate) event_version: String,
    pub(crate) kernel_task_run_id: String,
    pub(crate) session_run_id: String,
    pub(crate) aggregate_type: String,
    pub(crate) aggregate_id: String,
    pub(crate) idempotency_key: String,
    pub(crate) event_type: String,
    pub(crate) actor_kind: String,
    pub(crate) actor_id: String,
    pub(crate) causation_id: Option<String>,
    pub(crate) correlation_id: Option<String>,
    pub(crate) payload_hash: String,
    pub(crate) source_component: String,
    pub(crate) payload: serde_json::Value,
    pub(crate) wsids: Vec<String>,
    pub(crate) authority_resource_id: Option<surrealdb::types::RecordId>,
    pub(crate) authority_session_id: Option<surrealdb::types::RecordId>,
    pub(crate) authority_capability_id: Option<String>,
    pub(crate) authority_action: Option<String>,
    pub(crate) created_at: surrealdb::types::Datetime,
}

#[derive(Clone, PartialEq, SurrealValue)]
pub(crate) struct AlbumMutationAtelierExpectation {
    pub(crate) id: surrealdb::types::RecordId,
    pub(crate) event_id: surrealdb::types::Uuid,
    pub(crate) event_family: String,
    pub(crate) aggregate_type: String,
    pub(crate) aggregate_id: String,
    pub(crate) kernel_event_id: String,
    pub(crate) payload: serde_json::Value,
}

#[derive(Clone, PartialEq)]
pub(crate) struct AlbumMutationReorderItem {
    pub(crate) member_id: surrealdb::types::RecordId,
    pub(crate) asset_id: surrealdb::types::RecordId,
    pub(crate) sort_order: i64,
}

#[derive(Clone, PartialEq)]
pub(crate) enum AlbumMembershipCommitIntent {
    Unlink {
        collection_id: surrealdb::types::RecordId,
        member_id: surrealdb::types::RecordId,
        asset_id: surrealdb::types::RecordId,
        receipt_id: surrealdb::types::RecordId,
        actor: String,
    },
    LinkRefs {
        collection_id: surrealdb::types::RecordId,
        member_id: surrealdb::types::RecordId,
        asset_id: surrealdb::types::RecordId,
        source_path_ref: Option<Option<String>>,
        source_url_ref: Option<Option<String>>,
        actor: String,
    },
    Reorder {
        collection_id: surrealdb::types::RecordId,
        items: Vec<AlbumMutationReorderItem>,
        actor: String,
    },
}

#[derive(Clone, PartialEq)]
pub(crate) struct AlbumMutationCommitExpectation {
    pub(crate) ledger: AlbumMutationLedgerExpectation,
    pub(crate) atelier: AlbumMutationAtelierExpectation,
    pub(crate) membership: AlbumMembershipCommitIntent,
}

#[derive(SurrealValue)]
struct AlbumMutationCommitReadBindings {
    ledger_id: surrealdb::types::RecordId,
    atelier_id: surrealdb::types::RecordId,
    collection_id: surrealdb::types::RecordId,
    member_ids: Vec<surrealdb::types::RecordId>,
    full_membership: bool,
    receipt_id: Option<surrealdb::types::RecordId>,
    asset_id: Option<surrealdb::types::RecordId>,
}

#[derive(SurrealValue)]
struct AlbumMutationLedgerReadback {
    #[surreal(flatten)]
    expected: AlbumMutationLedgerExpectation,
    event_sequence: i64,
}

#[derive(SurrealValue)]
struct AlbumMutationAtelierReadback {
    #[surreal(flatten)]
    expected: AlbumMutationAtelierExpectation,
    kernel_event_sequence: i64,
}

#[derive(SurrealValue)]
struct AlbumMutationMemberReadback {
    id: surrealdb::types::RecordId,
    collection_id: surrealdb::types::RecordId,
    asset_id: surrealdb::types::RecordId,
    sort_order: i64,
    source_path_ref: Option<String>,
    source_url_ref: Option<String>,
    updated_by: String,
}

#[derive(SurrealValue)]
struct AlbumMutationUnlinkReadback {
    id: surrealdb::types::RecordId,
    collection_id: surrealdb::types::RecordId,
    asset_id: surrealdb::types::RecordId,
    unlinked_by: String,
}

#[derive(SurrealValue)]
struct AlbumMutationCommitReadback {
    ledger: Vec<AlbumMutationLedgerReadback>,
    atelier: Vec<AlbumMutationAtelierReadback>,
    members: Vec<AlbumMutationMemberReadback>,
    receipts: Vec<AlbumMutationUnlinkReadback>,
    assets: Vec<surrealdb::types::RecordId>,
}

const ALBUM_MUTATION_COMMIT_READBACK: &str = "RETURN {
    ledger: (SELECT id AS ledger_id, event_id, event_version, kernel_task_run_id, session_run_id,
        aggregate_type, aggregate_id, idempotency_key, event_type, actor_kind, actor_id,
        causation_id, correlation_id, payload_hash, source_component, payload, wsids,
        authority_resource_id, authority_session_id, authority_capability_id, authority_action,
        created_at, event_sequence FROM $ledger_id),
    atelier: (SELECT id, event_id, event_family, aggregate_type, aggregate_id, kernel_event_id,
        payload, kernel_event_sequence FROM $atelier_id),
    members: (SELECT id, collection_id, asset_id, sort_order, source_path_ref, source_url_ref,
        updated_by FROM atelier_collection_item
        WHERE collection_id = $collection_id AND ($full_membership OR id IN $member_ids)),
    receipts: (SELECT id, collection_id, asset_id, unlinked_by
        FROM atelier_collection_item_unlink_receipt WHERE id = $receipt_id),
    assets: (SELECT VALUE id FROM atelier_media_asset WHERE id = $asset_id)
};";

impl AlbumMutationCommitExpectation {
    fn matches_operation_and_authority(
        &self,
        operation: &AlbumMutationOperationReference,
        authority: &resource_authority::RecordUserScope,
    ) -> bool {
        use surrealdb::types::RecordId;
        let Ok(collection_id) = uuid::Uuid::parse_str(&self.ledger.aggregate_id) else { return false; };
        self.ledger.aggregate_type == "atelier_collection"
            && self.read_bindings().collection_id == RecordId::new("atelier_collection", surrealdb::types::Uuid::from(collection_id))
            && self.ledger.event_id == operation.event_id
            && self.ledger.idempotency_key == operation.idempotency_key
            && self.ledger.ledger_id == RecordId::new("kernel_event_ledger", operation.event_id.clone())
            && self.atelier.kernel_event_id == operation.event_id
            && self.atelier.aggregate_type == self.ledger.aggregate_type
            && self.atelier.aggregate_id == self.ledger.aggregate_id
            && self.ledger.authority_resource_id == Some(RecordId::new("protected_resources", authority.resource_id.clone()))
            && self.ledger.authority_session_id == Some(RecordId::new("authenticated_sessions", authority.session_id.clone()))
            && self.ledger.authority_capability_id.as_deref() == Some(authority.capability_id.as_str())
            && self.ledger.authority_action.as_deref() == Some(authority.action.as_str())
    }

    fn read_bindings(&self) -> AlbumMutationCommitReadBindings {
        let (collection_id, member_ids, full_membership, receipt_id, asset_id) = match &self.membership {
            AlbumMembershipCommitIntent::Unlink { collection_id, member_id, receipt_id, asset_id, .. } =>
                (collection_id.clone(), vec![member_id.clone()], false, Some(receipt_id.clone()), Some(asset_id.clone())),
            AlbumMembershipCommitIntent::LinkRefs { collection_id, member_id, .. } =>
                (collection_id.clone(), vec![member_id.clone()], false, None, None),
            AlbumMembershipCommitIntent::Reorder { collection_id, items, .. } =>
                (collection_id.clone(), items.iter().map(|item| item.member_id.clone()).collect(), true, None, None),
        };
        AlbumMutationCommitReadBindings {
            ledger_id: self.ledger.ledger_id.clone(), atelier_id: self.atelier.id.clone(),
            collection_id, member_ids, full_membership, receipt_id, asset_id,
        }
    }

    fn matches_readback(&self, read: &AlbumMutationCommitReadback) -> bool {
        let ([ledger], [atelier]) = (read.ledger.as_slice(), read.atelier.as_slice()) else { return false; };
        if ledger.expected != self.ledger || atelier.expected != self.atelier
            || ledger.event_sequence != atelier.kernel_event_sequence {
            return false;
        }
        match &self.membership {
            AlbumMembershipCommitIntent::Unlink { collection_id, asset_id, receipt_id, actor, .. } => {
                let [receipt] = read.receipts.as_slice() else { return false; };
                read.members.is_empty() && matches!(read.assets.as_slice(), [asset] if asset == asset_id)
                    && &receipt.id == receipt_id && &receipt.collection_id == collection_id
                    && &receipt.asset_id == asset_id && &receipt.unlinked_by == actor
            }
            AlbumMembershipCommitIntent::LinkRefs { collection_id, member_id, asset_id, source_path_ref, source_url_ref, actor } => {
                let [member] = read.members.as_slice() else { return false; };
                &member.id == member_id && &member.collection_id == collection_id && &member.asset_id == asset_id
                    && &member.updated_by == actor
                    && source_path_ref.as_ref().is_none_or(|expected| expected == &member.source_path_ref)
                    && source_url_ref.as_ref().is_none_or(|expected| expected == &member.source_url_ref)
            }
            AlbumMembershipCommitIntent::Reorder { collection_id, items, actor } =>
                read.members.len() == items.len()
                    && items.iter().enumerate().all(|(index, item)| {
                        !items[..index].iter().any(|prior| prior.member_id == item.member_id)
                            && read.members.iter().any(|member| member.id == item.member_id
                                && &member.collection_id == collection_id && member.asset_id == item.asset_id
                                && member.sort_order == item.sort_order && &member.updated_by == actor)
                    }),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AlbumMutationOperationReference {
    pub operation_id: uuid::Uuid,
    pub event_id: String,
    pub idempotency_key: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum AlbumMutationSettlement {
    Pending,
    Unobserved,
    CommitObserved,
    CancelNoEffectObserved,
    CallbackPanicRollbackObserved,
    CaptureReadRollbackObserved,
    JobReadbackRollbackObserved,
}

impl From<ActualSettlement> for AlbumMutationSettlement {
    fn from(settlement: ActualSettlement) -> Self {
        match settlement {
            ActualSettlement::Unobserved
            | ActualSettlement::WorkspaceArtifactCanonicalReadRollbackObserved => Self::Unobserved,
            ActualSettlement::CommitObserved => Self::CommitObserved,
            ActualSettlement::CancelNoEffectObserved => Self::CancelNoEffectObserved,
            ActualSettlement::CallbackPanicRollbackObserved => Self::CallbackPanicRollbackObserved,
            ActualSettlement::CaptureReadRollbackObserved => Self::CaptureReadRollbackObserved,
            ActualSettlement::JobReadbackRollbackObserved => Self::JobReadbackRollbackObserved,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct AlbumMutationObservation {
    pub operation: AlbumMutationOperationReference,
    pub settlement: AlbumMutationSettlement,
}

#[derive(Clone)]
struct AlbumMembershipTransactionRetention {
    operation: AlbumMutationOperationReference,
    authority: resource_authority::RecordUserScope,
    commit_expectation: Arc<AlbumMutationCommitExpectation>,
    settlement: AlbumMutationSettlement,
    #[cfg(feature = "test-utils")]
    proof: Option<Arc<AlbumMutationProofControl>>,
}

impl AlbumMembershipTransactionRetention {
    fn matches_authority(&self, authority: &resource_authority::RecordUserScope) -> bool {
        self.authority.resource_id == authority.resource_id
            && self.authority.grant_id == authority.grant_id
            && self.authority.session_id == authority.session_id
            && self.authority.workspace_id == authority.workspace_id
            && self.authority.capability_id == authority.capability_id
            && self.authority.action == authority.action
            && self.authority.channel_binding_hash == authority.channel_binding_hash
            && self.authority.session_token == authority.session_token
    }
}

#[cfg(feature = "test-utils")]
tokio::task_local! {
    static ALBUM_MUTATION_PROOF_CONTROL: Arc<AlbumMutationProofControl>;
}

#[cfg(feature = "test-utils")]
pub(crate) fn take_album_mutation_ledger_collision() -> Option<String> {
    ALBUM_MUTATION_PROOF_CONTROL.try_with(|proof| {
        proof.ledger_collision.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).take()
    }).ok().flatten()
}

#[cfg(feature = "test-utils")]
pub(crate) fn album_mutation_proof_active() -> bool {
    ALBUM_MUTATION_PROOF_CONTROL.try_with(|_| ()).is_ok()
}

#[cfg(feature = "test-utils")]
pub async fn with_album_mutation_proof_control<T>(
    control: Arc<AlbumMutationProofControl>,
    operation: impl std::future::Future<Output = T>,
) -> T {
    ALBUM_MUTATION_PROOF_CONTROL.scope(control, operation).await
}

#[cfg(feature = "test-utils")]
pub struct AlbumMutationProofControl {
    ledger_collision: StdMutex<Option<String>>,
    operation: StdMutex<Option<AlbumMutationOperationReference>>,
    settlement: StdMutex<Option<AlbumMutationSettlement>>,
    entered: tokio::sync::Notify,
    settled: tokio::sync::Notify,
    release: tokio::sync::Semaphore,
}

#[cfg(feature = "test-utils")]
impl AlbumMutationProofControl {
    pub fn new() -> Self {
        Self {
            ledger_collision: StdMutex::new(None),
            operation: StdMutex::new(None),
            settlement: StdMutex::new(None),
            entered: tokio::sync::Notify::new(),
            settled: tokio::sync::Notify::new(),
            release: tokio::sync::Semaphore::new(0),
        }
    }

    pub fn collide_next_ledger_create_with(&self, existing_event_id: String) {
        *self.ledger_collision.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(existing_event_id);
    }

    async fn pause(&self, operation: AlbumMutationOperationReference, maximum_wait: Duration) {
        *self.operation.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(operation);
        self.entered.notify_one();
        if let Ok(Ok(permit)) = tokio::time::timeout(maximum_wait, self.release.acquire()).await {
            permit.forget();
        }
    }

    fn observe(&self, settlement: AlbumMutationSettlement) {
        *self.settlement.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(settlement);
        self.settled.notify_one();
    }

    pub async fn wait_for_entry(&self) -> AlbumMutationOperationReference {
        loop {
            let notified = self.entered.notified();
            if let Some(operation) = self.operation.lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()).clone() {
                return operation;
            }
            notified.await;
        }
    }

    pub fn release(&self) {
        self.release.add_permits(1);
    }

    pub async fn wait_for_settlement(&self) -> AlbumMutationSettlement {
        loop {
            let notified = self.settled.notified();
            if let Some(settlement) = *self.settlement.lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) {
                return settlement;
            }
            notified.await;
        }
    }
}

impl SurrealStorage {
    pub(crate) async fn with_owned_validator_designation_authenticated_transaction<T, E, F>(
        &self,
        operation_id: uuid::Uuid,
        cancellation: Arc<ContextOperationCancellation>,
        association: resource_authority::ValidatorAssociationTransactionRetention,
        role: resource_authority::RoleAuthorityTransactionRetention,
        operation: F,
    ) -> Result<T, E>
    where
        T: Send + 'static, E: From<SurrealStorageError> + Send + 'static,
        F: for<'a> FnOnce(SurrealTransactionContext<'a>) -> SurrealTransactionOperation<'a, T, E> + Send + 'static,
    {
        if association.operation_id() != operation_id || role.operation_id() != operation_id {
            return Err(E::from(SurrealStorageError::TransactionRetentionMismatch { operation_id }));
        }
        let scope = current_record_user_scope()
            .ok_or_else(|| E::from(SurrealStorageError::TransactionAuthorityRequired))?;
        let deadline = association.original_deadline();
        if Instant::now() >= deadline {
            return Err(E::from(SurrealStorageError::StatementBudgetExhausted { budget_ms: 0 }));
        }
        {
            let settlements = self.inner.transaction_settlements.lock().unwrap_or_else(|p| p.into_inner());
            match settlements.get(&operation_id) {
                Some(Some(AuthenticatedTransactionRetention::ValidatorAssociation(original)))
                    if original.matches_retention(&association) => original.enter_designation_engine_once(&role).map_err(E::from)?,
                _ => return Err(E::from(SurrealStorageError::TransactionRetentionMismatch { operation_id })),
            }
        }
        self.transaction_worker(
            TransactionInvocation::Authenticated { scope, deadline, operation_id },
            Some(operation_id), Some(cancellation), TransactionSettlementMode::Mutation, operation,
        ).await
    }
    pub(crate) fn confirm_original_validator_designation(
        &self,
        association: &resource_authority::ValidatorAssociationTransactionRetention,
        role: &resource_authority::RoleAuthorityTransactionRetention,
    ) -> Result<(), SurrealStorageError> {
        let operation_id = association.operation_id();
        let settlements = self.inner.transaction_settlements.lock().unwrap_or_else(|p| p.into_inner());
        match settlements.get(&operation_id) {
            Some(Some(AuthenticatedTransactionRetention::ValidatorAssociation(original)))
                if original.matches_retention(association) => original.confirm_designation_publication(role),
            _ => Err(SurrealStorageError::TransactionRetentionMismatch { operation_id }),
        }
        // No removal/replacement. Association queries/results/role remain owned.
    }
}

impl SurrealStorage {
    pub(crate) async fn with_owned_validator_association_readback_transaction<T, E, F>(
        &self, operation_id: uuid::Uuid, cancellation: Arc<ContextOperationCancellation>,
        retention: resource_authority::ValidatorAssociationTransactionRetention, operation: F,
    ) -> Result<T, E>
    where T: Send + 'static, E: From<SurrealStorageError> + Send + 'static,
        F: for<'a> FnOnce(SurrealTransactionContext<'a>) -> SurrealTransactionOperation<'a, T, E> + Send + 'static,
    {
        if retention.operation_id() != operation_id { return Err(E::from(SurrealStorageError::TransactionRetentionMismatch { operation_id })); }
        let scope = current_record_user_scope().ok_or_else(|| E::from(SurrealStorageError::TransactionAuthorityRequired))?;
        let deadline = retention.original_deadline();
        if Instant::now() >= deadline { return Err(E::from(SurrealStorageError::StatementBudgetExhausted { budget_ms: 0 })); }
        {
            let settlements = self.inner.transaction_settlements.lock().unwrap_or_else(|p| p.into_inner());
            match settlements.get(&operation_id) {
                Some(Some(AuthenticatedTransactionRetention::ValidatorAssociation(original))) if original.matches_retention(&retention) => original.enter_initial_read_once().map_err(E::from)?,
                _ => return Err(E::from(SurrealStorageError::TransactionRetentionMismatch { operation_id })),
            }
        }
        // This read-only callback uses ONLY the actual child Read guard/static
        // Core SELECT. Its empty transaction's real commit closes readback;
        // it never overwrites the earlier association-write CommitObserved.
        self.transaction_worker(TransactionInvocation::Authenticated { scope, deadline, operation_id },
            Some(operation_id), Some(cancellation), TransactionSettlementMode::Mutation, operation).await
    }
}

impl SurrealTransactionContext<'_> {
pub(crate) fn require_authenticated_scope(
    &self,
    storage: &SurrealStorage,
    expected: &resource_authority::RecordUserScope,
) -> Result<(), SurrealStorageError> {
    let same_scope = |left: &resource_authority::RecordUserScope,
                      right: &resource_authority::RecordUserScope| {
        left.grant_id == right.grant_id && left.workspace_id == right.workspace_id
            && left.session_token == right.session_token
            && left.channel_binding_hash == right.channel_binding_hash
            && left.resource_id == right.resource_id && left.session_id == right.session_id
            && left.capability_id == right.capability_id && left.action == right.action
    };
    let current = current_record_user_scope()
        .ok_or(SurrealStorageError::TransactionAuthorityRequired)?;
    PROTECTED_TRANSACTION.try_with(|protected| {
        if !std::ptr::eq(self.transaction, protected.transaction.as_ref())
            || !Arc::ptr_eq(&storage.inner, &protected.storage.inner)
            || !same_scope(&protected.authority, expected)
            || !same_scope(&current, expected)
        {
            return Err(SurrealStorageError::TransactionAuthorityRequired);
        }
        Ok(())
    }).unwrap_or_else(|_| Err(SurrealStorageError::TransactionAuthorityRequired))
}
}

impl SurrealStorage {
    // Both private login issuers hold this SAME gate through actual strong-owner
    // insertion and worker-handle publication. Closing therefore precedes a
    // stable snapshot of BOTH operation kinds. Closing never reopens admission.
    fn close_member_login_admission(&self) -> Result<(), SurrealStorageError> {
        let mut admission = self.inner.member_login_admission.lock()
            .map_err(|_| SurrealStorageError::Closed)?;
        *admission = false;
        Ok(())
    }
}

// New bounded startup-only provisioning methods in storage/surreal.rs.
// Runtime redemption never calls these methods or uses an administrator client.
pub(crate) const MEMBER_RECIPIENT_KEY_ID: &str = "member-enrollment-recipient-v1";
const MEMBER_RECIPIENT_CONFIG_ID: &str = "primary";
pub(crate) enum MemberRecipientCredentialProvisionState {
    Absent,
    Reserved,
    WriteStarted { verifier_hex: String },
    Ready { verifier_hex: String },
}
#[derive(surrealdb::types::SurrealValue)]
struct MemberRecipientProvisionRow {
    id: surrealdb::types::RecordId,
    key_id: String,
    state: String,
    verifier_hex: Option<String>,
}
fn member_recipient_provision_error() -> SurrealStorageError {
    SurrealStorageError::InvalidModelLaneRecord { reason: "recipient credential provisioning unavailable" }
}
fn member_recipient_verifier_shape(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
fn member_recipient_provision_decode(
    rows: Vec<MemberRecipientProvisionRow>,
) -> Result<MemberRecipientCredentialProvisionState, SurrealStorageError> {
    if rows.is_empty() { return Ok(MemberRecipientCredentialProvisionState::Absent); }
    let [row] = rows.as_slice() else { return Err(member_recipient_provision_error()); };
    if row.id != surrealdb::types::RecordId::new("member_enrollment_recipient_auth_config", MEMBER_RECIPIENT_CONFIG_ID)
        || row.key_id != MEMBER_RECIPIENT_KEY_ID { return Err(member_recipient_provision_error()); }
    match (row.state.as_str(), row.verifier_hex.as_deref()) {
        ("reserved", None) => Ok(MemberRecipientCredentialProvisionState::Reserved),
        ("write_started", Some(value)) if member_recipient_verifier_shape(value) =>
            Ok(MemberRecipientCredentialProvisionState::WriteStarted { verifier_hex: value.to_owned() }),
        ("ready", Some(value)) if member_recipient_verifier_shape(value) =>
            Ok(MemberRecipientCredentialProvisionState::Ready { verifier_hex: value.to_owned() }),
        _ => Err(member_recipient_provision_error()),
    }
}
impl SurrealStorage {
    pub(crate) async fn member_recipient_provision_observe(
        &self,
    ) -> Result<MemberRecipientCredentialProvisionState, SurrealStorageError> {
        self.with_admin_operation(|admin| Box::pin(async move {
            let response = admin.client.query(
                "SELECT * FROM member_enrollment_recipient_auth_config:primary;"
            ).await?;
            member_recipient_provision_decode(decode_query_values(response, 0)?)
        })).await
    }
    pub(crate) async fn member_recipient_provision_reserve(
        &self,
    ) -> Result<MemberRecipientCredentialProvisionState, SurrealStorageError> {
        // External lifecycle owner holds its real cross-process purpose lock.
        // A conflicting CREATE fails; its caller may reobserve, never overwrite.
        self.with_admin_operation(|admin| Box::pin(async move {
            let response = admin.client.query(
                "IF record::exists(member_enrollment_recipient_auth_config:primary) {
                    SELECT * FROM member_enrollment_recipient_auth_config:primary;
                 } ELSE {
                    CREATE member_enrollment_recipient_auth_config:primary
                    SET key_id = 'member-enrollment-recipient-v1', state = 'reserved', verifier_hex = NONE
                    RETURN AFTER;
                 };"
            ).await?;
            member_recipient_provision_decode(decode_query_values(response, 0)?)
        })).await
    }
    pub(crate) async fn member_recipient_provision_mark_write_started(
        &self, expected_reserved: &MemberRecipientCredentialProvisionState, verifier_hex: &str,
    ) -> Result<MemberRecipientCredentialProvisionState, SurrealStorageError> {
        if !matches!(expected_reserved, MemberRecipientCredentialProvisionState::Reserved)
            || !member_recipient_verifier_shape(verifier_hex) { return Err(member_recipient_provision_error()); }
        let verifier_hex = verifier_hex.to_owned();
        self.with_admin_operation(move |admin| Box::pin(async move {
            let response = admin.client.query(
                "UPDATE member_enrollment_recipient_auth_config:primary
                 SET state = 'write_started', verifier_hex = $verifier
                 WHERE key_id = 'member-enrollment-recipient-v1' AND state = 'reserved' AND verifier_hex = NONE
                 RETURN AFTER;"
            ).bind(("verifier", verifier_hex.clone())).await?;
            let state = member_recipient_provision_decode(decode_query_values(response, 0)?)?;
            match &state {
                MemberRecipientCredentialProvisionState::WriteStarted { verifier_hex: observed }
                    if observed == &verifier_hex => Ok(state),
                _ => Err(member_recipient_provision_error()),
            }
        })).await
    }
    pub(crate) async fn member_recipient_provision_bind_verified(
        &self, verifier_hex: &str,
    ) -> Result<MemberRecipientCredentialProvisionState, SurrealStorageError> {
        if !member_recipient_verifier_shape(verifier_hex) { return Err(member_recipient_provision_error()); }
        let verifier_hex = verifier_hex.to_owned();
        self.with_admin_operation(move |admin| Box::pin(async move {
            let response = admin.client.query(
                "UPDATE member_enrollment_recipient_auth_config:primary
                 SET state = 'ready'
                 WHERE key_id = 'member-enrollment-recipient-v1'
                 AND state IN ['write_started', 'ready'] AND verifier_hex = $verifier
                 RETURN AFTER;"
            ).bind(("verifier", verifier_hex.clone())).await?;
            let state = member_recipient_provision_decode(decode_query_values(response, 0)?)?;
            match &state {
                MemberRecipientCredentialProvisionState::Ready { verifier_hex: observed }
                    if observed == &verifier_hex => Ok(state),
                _ => Err(member_recipient_provision_error()),
            }
        })).await
    }
}
// Exact companion schema. Only configured storage bootstrap uses admin access.
const MEMBER_RECIPIENT_PROVISION_SCHEMA: &str = r#"
DEFINE TABLE OVERWRITE member_enrollment_recipient_auth_config TYPE NORMAL SCHEMAFULL PERMISSIONS NONE;
DEFINE FIELD OVERWRITE key_id ON TABLE member_enrollment_recipient_auth_config TYPE string
    ASSERT $value = 'member-enrollment-recipient-v1';
DEFINE FIELD OVERWRITE state ON TABLE member_enrollment_recipient_auth_config TYPE string
    ASSERT $value IN ['reserved', 'write_started', 'ready'];
DEFINE FIELD OVERWRITE verifier_hex ON TABLE member_enrollment_recipient_auth_config TYPE option<string>
    ASSERT ($value = NONE AND $this.state = 'reserved')
        OR ($this.state IN ['write_started', 'ready'] AND string::matches($value, /^[0-9a-f]{64}$/));
DEFINE EVENT OVERWRITE member_recipient_provision_guard ON TABLE member_enrollment_recipient_auth_config
WHEN $event IN ['CREATE', 'UPDATE', 'DELETE']
THEN {
    IF $auth != NONE OR $event = 'DELETE' { THROW 'HSK-RECIPIENT-PROVISION-DENIED'; };
    IF $event = 'CREATE' {
        IF $after.id != member_enrollment_recipient_auth_config:primary
            OR $after.key_id != 'member-enrollment-recipient-v1'
            OR $after.state != 'reserved' OR $after.verifier_hex != NONE {
            THROW 'HSK-RECIPIENT-PROVISION-DENIED';
        };
    } ELSE {
        IF $before.id != $after.id OR $before.key_id != $after.key_id
            OR !(($before.state = 'reserved' AND $before.verifier_hex = NONE
                    AND $after.state = 'write_started' AND $after.verifier_hex != NONE)
                OR ($before.state IN ['write_started', 'ready'] AND $after.state = 'ready'
                    AND $before.verifier_hex = $after.verifier_hex)) {
            THROW 'HSK-RECIPIENT-PROVISION-DENIED';
        };
    };
};
"#;

// New bounded startup schema installer in storage/surreal.rs.
// Exact companion constant is in the reviewed 7957/b5ac provision code.
impl SurrealStorage {
    pub(crate) async fn install_member_recipient_provisioning_schema(
        &self,
    ) -> Result<(), SurrealStorageError> {
        self.with_admin_operation(|admin| Box::pin(async move {
            admin.client.query(MEMBER_RECIPIENT_PROVISION_SCHEMA).await?.check()?;
            Ok(())
        })).await
    }
}
// This method installs only the fixed credential-provisioning declarations.
// Full recipient/table/nonce schema remains the schema owner's pinned migration,
// never an opportunistic per-command admin schema change.
