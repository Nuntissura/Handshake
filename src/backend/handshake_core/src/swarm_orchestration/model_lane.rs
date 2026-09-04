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
    KernelActor, KernelEvent, KernelEventType, NewKernelEvent,
};
use crate::model_runtime::ProviderKind;
use crate::storage::surreal::{
    bootstrap_cloud_model_lane_schema, CloudModelLaneRecordKind, CloudModelLaneScope,
    CloudModelLaneStore, CloudModelLaneStoredRow, ModelLaneRecordKind as SurrealRecordKind,
    ModelLaneScope as SurrealModelLaneScope, SurrealCrdtLeaseClaimOutcome,
    SurrealCrdtUpdateAppendOutcome, SurrealModelLaneCrdtEventWrite, SurrealModelLaneCrdtGuard,
    SurrealModelLaneCrdtLeaseHistory, SurrealModelLaneCrdtLeaseWrite,
    SurrealModelLaneCrdtProposalRecord, SurrealModelLaneCrdtProposalWrite,
    SurrealModelLaneCrdtSnapshot, SurrealModelLaneCrdtUpdate, SurrealModelLaneMessageGuard,
    SurrealModelLaneRecord, SurrealModelLaneRoutingAttemptWrite, SurrealModelLaneRoutingClaim,
    SurrealModelLaneRoutingCommit, SurrealModelLaneRoutingEventWrite,
    SurrealModelLaneRoutingExecutionRow, SurrealModelLaneRoutingExecutionWrite,
    SurrealModelLaneRoutingOutboxRow, SurrealModelLaneRoutingOutboxWrite, SurrealModelLaneStore,
    SurrealModelLaneWrite, SurrealStorage, SurrealStorageError,
};
use crate::storage::StorageError;

use super::error::SwarmError;
use super::factory::LiveSession;
use super::ids::{ByokCloudProvider, SpawnRequest};
use super::resource_scope::{
    AccessSpaceRef, AccountBoundAuthority, ActorPrincipalId, AuthenticatedSessionRef,
    ExactResourceScopeAttribution, OwnerAccountId, ResourceAccessContext,
    ResourceAccessLifecycleRegistry, ResourceScope, ResourceScopeQuery, ScopeDenied,
    WorkspaceScopeRef,
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
    #[error("model lane embedded SurrealDB error: {0}")]
    Surreal(#[from] SurrealStorageError),
    #[error("model lane json error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type ModelLaneResult<T> = Result<T, ModelLaneError>;

#[derive(Debug, Clone)]
pub struct ModelLaneStore {
    storage: SurrealStorage,
    provider: Arc<OnceCell<SurrealModelLaneStore>>,
    cloud_authority: CloudModelLaneStore,
    /// HBR-PRIV-001/002. Every write stamps this context onto the five scope
    /// fields and every scoped read binds all five fields in SurrealQL.
    access: ResourceAccessContext,
    #[cfg(feature = "surreal-test-support")]
    terminal_commit_test_control: ModelLaneTerminalCommitTestControl,
}

/// Provider-neutral, per-store terminal-commit fault controller. The hooks are
/// feature-gated and expose no storage client or query capability.
#[cfg(feature = "surreal-test-support")]
#[derive(Clone, Debug)]
pub struct ModelLaneTerminalCommitTestControl {
    pause_once: Arc<std::sync::atomic::AtomicBool>,
    fail_once: Arc<std::sync::atomic::AtomicBool>,
    entered: Arc<tokio::sync::Notify>,
    release: Arc<tokio::sync::Notify>,
}

#[cfg(feature = "surreal-test-support")]
impl Default for ModelLaneTerminalCommitTestControl {
    fn default() -> Self {
        Self {
            pause_once: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            fail_once: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            entered: Arc::new(tokio::sync::Notify::new()),
            release: Arc::new(tokio::sync::Notify::new()),
        }
    }
}

#[cfg(feature = "surreal-test-support")]
impl ModelLaneTerminalCommitTestControl {
    pub fn pause_next(&self) {
        self.pause_once
            .store(true, std::sync::atomic::Ordering::Release);
    }

    pub fn fail_next(&self) {
        self.fail_once
            .store(true, std::sync::atomic::Ordering::Release);
    }

    pub async fn wait_until_paused(&self) {
        self.entered.notified().await;
    }

    pub fn release_paused(&self) {
        self.release.notify_one();
    }

    async fn before_commit(&self) -> ModelLaneResult<()> {
        if self
            .fail_once
            .swap(false, std::sync::atomic::Ordering::AcqRel)
        {
            return Err(ModelLaneError::IntegrityViolation(
                "injected terminal commit failure before durable mutation".into(),
            ));
        }
        if self
            .pause_once
            .swap(false, std::sync::atomic::Ordering::AcqRel)
        {
            self.entered.notify_one();
            self.release.notified().await;
        }
        Ok(())
    }
}

#[cfg(feature = "surreal-test-support")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelLaneRoutingTestCorruption {
    AttemptEventAggregateType,
    AttemptEventAggregateId,
    ExecutionEventSequence,
    AttemptEventSequence,
    OutboxEventSequence,
}

#[cfg(feature = "surreal-test-support")]
impl ModelLaneRoutingTestCorruption {
    const fn as_str(self) -> &'static str {
        match self {
            Self::AttemptEventAggregateType => "attempt_event_aggregate_type",
            Self::AttemptEventAggregateId => "attempt_event_aggregate_id",
            Self::ExecutionEventSequence => "execution_event_sequence",
            Self::AttemptEventSequence => "attempt_event_sequence",
            Self::OutboxEventSequence => "outbox_event_sequence",
        }
    }
}

#[cfg(feature = "surreal-test-support")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelLaneCrdtTestCorruption {
    RecordedReceiptAggregate,
    AppliedReceiptPayloadHash,
    ProposalDiffHash,
    UpdateContentHash,
    ProposalIncompleteAttribution,
    AppliedReceiptMixedScope,
    ProposalActorIdentity,
    ProposalSessionIdentity,
    ProposalTraceIdentity,
    ProposalDocumentIdentity,
    PromotionAcceptedCausation,
}

#[cfg(feature = "surreal-test-support")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelLaneAuthorityTestCorruption {
    ProjectionEventSequence,
    ProjectionScope,
    ReceiptPayloadHash,
    ReceiptScope,
    /// Seeds one deliberately incomplete-attribution authority row in the
    /// exact namespace/database: the row keeps owner, Principal, and
    /// workspace attribution but loses its authenticated session and
    /// AccessSpace attribution, like a row that predates five-field scope.
    IncompleteAttribution,
    /// Attempts to blank a scope field; the SCHEMAFULL authority table must
    /// refuse it, so the seam is expected to fail without mutating the row.
    BlankAttribution,
}

#[cfg(feature = "surreal-test-support")]
impl ModelLaneAuthorityTestCorruption {
    const fn as_str(self) -> &'static str {
        match self {
            Self::ProjectionEventSequence => "projection_event_sequence",
            Self::ProjectionScope => "projection_scope",
            Self::ReceiptPayloadHash => "receipt_payload_hash",
            Self::ReceiptScope => "receipt_scope",
            Self::IncompleteAttribution => "incomplete_attribution",
            Self::BlankAttribution => "blank_attribution",
        }
    }
}

#[cfg(feature = "surreal-test-support")]
impl ModelLaneCrdtTestCorruption {
    const fn as_str(self) -> &'static str {
        match self {
            Self::RecordedReceiptAggregate => "recorded_receipt_aggregate",
            Self::AppliedReceiptPayloadHash => "applied_receipt_payload_hash",
            Self::ProposalDiffHash => "proposal_diff_hash",
            Self::UpdateContentHash => "update_content_hash",
            Self::ProposalIncompleteAttribution => "proposal_incomplete_attribution",
            Self::AppliedReceiptMixedScope => "applied_receipt_mixed_scope",
            Self::ProposalActorIdentity => "proposal_actor_identity",
            Self::ProposalSessionIdentity => "proposal_session_identity",
            Self::ProposalTraceIdentity => "proposal_trace_identity",
            Self::ProposalDocumentIdentity => "proposal_document_identity",
            Self::PromotionAcceptedCausation => "promotion_accepted_causation",
        }
    }
}

#[cfg(feature = "surreal-test-support")]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ModelLaneCrdtAuthorityCounts {
    pub proposal_rows: i64,
    pub update_rows: i64,
    pub snapshot_rows: i64,
    pub lease_rows: i64,
    pub event_rows: i64,
}

struct SurrealResolvedMessageAuthority {
    crdt_binding: Option<ModelLaneCrdtAuthorityBinding>,
    guard: SurrealModelLaneMessageGuard,
}

const MODEL_LANE_NAVIGATION_RECORD_KINDS: &[SurrealRecordKind] = &[
    SurrealRecordKind::Run,
    SurrealRecordKind::Lane,
    SurrealRecordKind::Message,
    SurrealRecordKind::PromotionDecision,
    SurrealRecordKind::ContextArtifact,
    SurrealRecordKind::ContextHandoff,
    SurrealRecordKind::RecoveryCheckpoint,
    SurrealRecordKind::RecoveryEvent,
    SurrealRecordKind::Lease,
    SurrealRecordKind::DiagnosticTier,
    SurrealRecordKind::MtRuntimeStatus,
    SurrealRecordKind::SessionCleanupReceipt,
    SurrealRecordKind::SelectionAudit,
    SurrealRecordKind::RoutingExecution,
    SurrealRecordKind::CloudProjectionPlan,
    SurrealRecordKind::CloudConsentReceipt,
    SurrealRecordKind::CloudConsentDenial,
];

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewModelLaneSelectionAudit {
    pub audit_id: String,
    pub run_id: String,
    pub selected_model_id: String,
    pub actor_ref: String,
    pub reason: String,
    pub selection_context: Value,
    pub event_ledger_stream_id: String,
    pub work_packet_id: String,
    pub micro_task_id: String,
    pub task_board_id: String,
    pub owner_session: String,
    pub idempotency_key: String,
    pub created_at_utc: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelLaneSelectionAuditRecord {
    #[serde(flatten)]
    pub inner: NewModelLaneSelectionAudit,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
    pub event_stream_version: i64,
    pub transaction_seq: i64,
}

impl Deref for ModelLaneSelectionAuditRecord {
    type Target = NewModelLaneSelectionAudit;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[cfg(feature = "surreal-test-support")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelLaneScopedAuthorityReceipt {
    pub record_kind: String,
    pub aggregate_id: String,
    pub run_id: String,
    pub event_id: String,
    pub event_ledger_seq: i64,
    pub event_type: String,
    pub payload_hash: String,
    pub owner_account_id: String,
    pub actor_principal_id: String,
    pub authenticated_session_id: String,
    pub access_space_id: String,
    pub workspace_id: String,
}

#[cfg(feature = "surreal-test-support")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelLaneCleanupReceiptInspection {
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
    pub fn new(storage: SurrealStorage, access: ResourceAccessContext) -> Self {
        Self {
            provider: Arc::new(OnceCell::new()),
            cloud_authority: CloudModelLaneStore::new(storage.clone()),
            storage,
            access,
            #[cfg(feature = "surreal-test-support")]
            terminal_commit_test_control: ModelLaneTerminalCommitTestControl::default(),
        }
    }

    #[cfg(feature = "surreal-test-support")]
    pub fn test_terminal_commit_control(&self) -> ModelLaneTerminalCommitTestControl {
        self.terminal_commit_test_control.clone()
    }

    /// Binds a facade to the shared embedded store after provisioning the
    /// cloud-lane authority schema on it, so cloud policy proofs run against
    /// exactly the namespace/database the caller injected.
    pub async fn new_surreal_cloud_authority_only(
        access: ResourceAccessContext,
        storage: SurrealStorage,
    ) -> ModelLaneResult<Self> {
        bootstrap_cloud_model_lane_schema(&storage).await?;
        Ok(Self::new(storage, access))
    }

    pub fn new_scoped(storage: SurrealStorage, scope: ResourceScope) -> Self {
        Self::new(storage, ResourceAccessContext::for_account(scope))
    }

    /// Production account construction. The supplied registry must be the
    /// shared authentication/session authority for the composition root, and
    /// the exact tuple must already be registered active. Unlike
    /// [`Self::new_scoped`], this path can authorize work; the legacy constructor
    /// intentionally remains fail-closed until its caller is migrated.
    pub fn new_scoped_with_lifecycle(
        storage: SurrealStorage,
        scope: ResourceScope,
        lifecycle: ResourceAccessLifecycleRegistry,
    ) -> Self {
        Self::new(
            storage,
            ResourceAccessContext::for_account_with_lifecycle(scope, lifecycle),
        )
    }

    #[cfg(feature = "surreal-test-support")]
    pub fn new_test_stale_access(storage: SurrealStorage, scope: ResourceScope) -> Self {
        let lifecycle = ResourceAccessLifecycleRegistry::new();
        let exact = ExactResourceScopeAttribution::try_from_resource_scope(&scope)
            .expect("stale-access proof requires an exact resource scope");
        lifecycle
            .register_active(exact.clone())
            .expect("register stale-access proof context");
        lifecycle
            .mark_stale(&exact)
            .expect("mark stale-access proof context stale");
        Self::new_scoped_with_lifecycle(storage, scope, lifecycle)
    }

    #[cfg(feature = "surreal-test-support")]
    pub fn new_test_revoked_access(storage: SurrealStorage, scope: ResourceScope) -> Self {
        let lifecycle = ResourceAccessLifecycleRegistry::new();
        let exact = ExactResourceScopeAttribution::try_from_resource_scope(&scope)
            .expect("revoked-access proof requires an exact resource scope");
        lifecycle
            .register_active(exact.clone())
            .expect("register revoked-access proof context");
        lifecycle
            .revoke(&exact)
            .expect("revoke revoked-access proof context");
        Self::new_scoped_with_lifecycle(storage, scope, lifecycle)
    }

    async fn provider(&self) -> ModelLaneResult<&SurrealModelLaneStore> {
        self.provider
            .get_or_try_init(|| SurrealModelLaneStore::initialize(self.storage.clone()))
            .await
            .map_err(Into::into)
    }

    async fn cloud_authority(&self) -> ModelLaneResult<&CloudModelLaneStore> {
        bootstrap_cloud_model_lane_schema(&self.storage).await?;
        Ok(&self.cloud_authority)
    }

    pub fn access(&self) -> &ResourceAccessContext {
        &self.access
    }

    pub(crate) fn surreal_storage(&self) -> &SurrealStorage {
        &self.storage
    }

    /// The scope stamped onto rows written through this store, if any.
    /// Trusted write scope bound at store construction. Production hosts use
    /// this to derive projection attribution from the durable authority rather
    /// than accepting account identifiers from request or renderer payloads.
    pub fn write_scope(&self) -> Option<&ResourceScope> {
        self.access.write_scope()
    }

    pub async fn routing_execution_snapshot(
        &self,
        execution_id: &str,
    ) -> ModelLaneResult<Option<super::routing_execution::ModelLaneRoutingExecutionState>> {
        let scope = self.surreal_read_scope("routing execution snapshot")?;
        let provider = self.provider().await?;
        let Some(row) = provider
            .routing_execution_snapshot(execution_id, &scope)
            .await?
        else {
            return Ok(None);
        };
        let attempts = provider
            .routing_attempts_for_execution(execution_id, &scope)
            .await?;
        let outbox = provider
            .routing_outbox_for_execution(execution_id, &scope)
            .await?;
        routing_execution_from_surreal(row, attempts, &outbox).map(Some)
    }

    pub async fn routing_execution_diagnostics_for_run(
        &self,
        run_id: &str,
    ) -> ModelLaneResult<
        Vec<super::routing_execution::ModelLaneRoutingExecutionDiagnostics>,
    > {
        let scope = self.surreal_read_scope("routing execution diagnostics")?;
        let provider = self.provider().await?;
        let rows = provider.routing_executions_for_run(run_id, &scope).await?;
        if rows.len() > 4096 {
            return Err(ModelLaneError::IntegrityViolation(
                "routing diagnostics exceeded the bounded execution row cap".into(),
            ));
        }
        let mut diagnostics = Vec::with_capacity(rows.len());
        for row in rows {
            let execution_id = row.execution_id.clone();
            let attempts = provider
                .routing_attempts_for_execution(&execution_id, &scope)
                .await?;
            let outbox = provider
                .routing_outbox_for_execution(&execution_id, &scope)
                .await?;
            let execution = routing_execution_from_surreal(row, attempts, &outbox)?;
            if execution.run_id != run_id {
                return Err(ModelLaneError::IntegrityViolation(
                    "routing diagnostics escaped the requested run scope".into(),
                ));
            }
            diagnostics.push(routing_diagnostics_from_surreal(execution, outbox)?);
        }
        Ok(diagnostics)
    }

    /// Feature-gated, exact-scope corruption seam for routing authorization
    /// counterfactuals. It exposes no client or arbitrary query capability.
    #[cfg(feature = "surreal-test-support")]
    pub async fn test_corrupt_routing_authority(
        &self,
        execution_id: &str,
        stage_id: &str,
        attempt: u32,
        corruption: ModelLaneRoutingTestCorruption,
    ) -> ModelLaneResult<()> {
        let scope = self.surreal_write_scope("routing authorization corruption proof")?;
        let provider = self.provider().await?;
        provider
            .test_corrupt_routing_authority(
                execution_id,
                stage_id,
                i64::from(attempt),
                corruption.as_str(),
                &scope,
            )
            .await?;
        Ok(())
    }

    /// Feature-gated exact-scope projection for non-mutation assertions. It
    /// exposes counts only, never the Surreal client or arbitrary query text.
    #[cfg(feature = "surreal-test-support")]
    pub async fn test_crdt_authority_counts(
        &self,
    ) -> ModelLaneResult<ModelLaneCrdtAuthorityCounts> {
        let scope = self.surreal_read_scope("inspect CRDT authority counts")?;
        let counts = self
            .provider()
            .await?
            .test_crdt_authority_counts(&scope)
            .await?;
        Ok(ModelLaneCrdtAuthorityCounts {
            proposal_rows: counts.proposal_rows,
            update_rows: counts.update_rows,
            snapshot_rows: counts.snapshot_rows,
            lease_rows: counts.lease_rows,
            event_rows: counts.event_rows,
        })
    }

    /// Feature-gated enumerated corruption seam for fail-closed CRDT proofs.
    /// Positive posture must be created through the production APIs above.
    #[cfg(feature = "surreal-test-support")]
    pub async fn test_corrupt_crdt_proposal_authority(
        &self,
        proposal_id: &str,
        update_id: &str,
        corruption: ModelLaneCrdtTestCorruption,
    ) -> ModelLaneResult<()> {
        let scope = self.surreal_write_scope("corrupt CRDT authority for denial proof")?;
        self.provider()
            .await?
            .test_corrupt_crdt_proposal_authority(
                proposal_id,
                update_id,
                corruption.as_str(),
                &scope,
            )
            .await?;
        Ok(())
    }

    /// `events` contains exactly one execution, attempt, and outbox event plus
    /// any correlated scheduling/context-handoff events. Every extra event is
    /// durably linked to this revision in the same Surreal transaction; callers
    /// must never append those EventLedger receipts separately.
    #[allow(clippy::too_many_arguments)]
    pub async fn commit_routing_execution_atomic(
        &self,
        expected_revision: u64,
        expected_claim: Option<&super::routing_execution::ModelLaneRoutingStageClaim>,
        next_execution: super::routing_execution::ModelLaneRoutingExecutionState,
        changed_attempt: super::routing_execution::ModelLaneRoutingStageState,
        outbox_status: &str,
        events: Vec<NewKernelEvent>,
    ) -> ModelLaneResult<super::routing_execution::ModelLaneRoutingExecutionState> {
        let (execution, message, binding) = self
            .commit_routing_atomic(
                expected_revision,
                expected_claim,
                next_execution,
                changed_attempt,
                outbox_status,
                None,
                None,
                events,
            )
            .await?;
        if message.is_some() || binding.is_some() {
            return Err(ModelLaneError::IntegrityViolation(
                "plain routing commit returned unexpected message authority".into(),
            ));
        }
        Ok(execution)
    }

    /// Uses the same atomic required-plus-extra event contract as
    /// [`Self::commit_routing_execution_atomic`].
    #[allow(clippy::too_many_arguments)]
    pub async fn commit_routing_authority_request_atomic(
        &self,
        expected_revision: u64,
        claim: &super::routing_execution::ModelLaneRoutingStageClaim,
        next_execution: super::routing_execution::ModelLaneRoutingExecutionState,
        changed_attempt: super::routing_execution::ModelLaneRoutingStageState,
        outbox_status: &str,
        message: NewModelLaneMessage,
        events: Vec<NewKernelEvent>,
    ) -> ModelLaneResult<(
        super::routing_execution::ModelLaneRoutingExecutionState,
        ModelLaneMessageRecord,
    )> {
        let (execution, message, binding) = self
            .commit_routing_atomic(
                expected_revision,
                Some(claim),
                next_execution,
                changed_attempt,
                outbox_status,
                Some(message),
                None,
                events,
            )
            .await?;
        if binding.is_some() {
            return Err(ModelLaneError::IntegrityViolation(
                "routing authority request returned unexpected artifact binding".into(),
            ));
        }
        Ok((
            execution,
            message.ok_or_else(|| {
                ModelLaneError::IntegrityViolation(
                    "routing authority request returned no durable message".into(),
                )
            })?,
        ))
    }

    /// Uses the same atomic required-plus-extra event contract as
    /// [`Self::commit_routing_execution_atomic`].
    #[allow(clippy::too_many_arguments)]
    pub async fn commit_routing_generated_output_atomic(
        &self,
        expected_revision: u64,
        claim: &super::routing_execution::ModelLaneRoutingStageClaim,
        next_execution: super::routing_execution::ModelLaneRoutingExecutionState,
        changed_attempt: super::routing_execution::ModelLaneRoutingStageState,
        outbox_status: &str,
        message: Option<NewModelLaneMessage>,
        binding: NewModelLaneContextBundleArtifactBinding,
        events: Vec<NewKernelEvent>,
    ) -> ModelLaneResult<(
        super::routing_execution::ModelLaneRoutingExecutionState,
        Option<ModelLaneMessageRecord>,
        ModelLaneContextBundleArtifactBindingRecord,
    )> {
        let (execution, message, binding) = self
            .commit_routing_atomic(
                expected_revision,
                Some(claim),
                next_execution,
                changed_attempt,
                outbox_status,
                message,
                Some(binding),
                events,
            )
            .await?;
        Ok((
            execution,
            message,
            binding.ok_or_else(|| {
                ModelLaneError::IntegrityViolation(
                    "routing generated output returned no durable artifact binding".into(),
                )
            })?,
        ))
    }

    #[allow(clippy::too_many_arguments)]
    async fn commit_routing_atomic(
        &self,
        expected_revision: u64,
        expected_claim: Option<&super::routing_execution::ModelLaneRoutingStageClaim>,
        next_execution: super::routing_execution::ModelLaneRoutingExecutionState,
        changed_attempt: super::routing_execution::ModelLaneRoutingStageState,
        outbox_status: &str,
        message: Option<NewModelLaneMessage>,
        binding: Option<NewModelLaneContextBundleArtifactBinding>,
        events: Vec<NewKernelEvent>,
    ) -> ModelLaneResult<(
        super::routing_execution::ModelLaneRoutingExecutionState,
        Option<ModelLaneMessageRecord>,
        Option<ModelLaneContextBundleArtifactBindingRecord>,
    )> {
        let expected_revision = i64::try_from(expected_revision).map_err(|_| {
            ModelLaneError::InvalidInput("routing revision exceeds durable integer range".into())
        })?;
        let scope = self.surreal_write_scope("atomic routing execution commit")?;
        validate_routing_commit_projection(
            expected_revision,
            &next_execution,
            &changed_attempt,
            expected_claim,
        )?;
        if let Some(message) = message.as_ref() {
            validate_message(message)?;
        }
        if let Some(binding) = binding.as_ref() {
            validate_context_bundle_artifact_binding(binding)?;
        }
        if let (Some(message), Some(binding)) = (message.as_ref(), binding.as_ref()) {
            validate_message_payload_binding_pair(message, binding)?;
        }
        let provider = self.provider().await?;
        let existing_outbox = provider
            .routing_outbox_for_execution(&next_execution.execution_id, &scope)
            .await?;
        let command_id = format!(
            "routing-command:{}:{}:{}",
            next_execution.execution_id, changed_attempt.stage_id, changed_attempt.attempt
        );
        let created_at_unix_ms = match existing_outbox
            .iter()
            .find(|row| row.command_id == command_id)
        {
            Some(row) => u64::try_from(row.created_at_unix_ms).map_err(|_| {
                ModelLaneError::IntegrityViolation(
                    "routing outbox has negative durable creation time".into(),
                )
            })?,
            None => changed_attempt.updated_at_unix_ms,
        };
        let (message_write, message_guard) = if let Some(message) = message.as_ref() {
            let authority = self
                .validate_surreal_message_authority(message, &scope)
                .await?;
            (
                Some(surreal_message_write(
                    message,
                    authority.crdt_binding,
                    &scope,
                )?),
                Some(authority.guard),
            )
        } else {
            (None, None)
        };
        let binding_write = binding
            .as_ref()
            .map(|binding| surreal_binding_write(binding, &scope))
            .transpose()?;
        let attempt_id = format!(
            "{}:{}:{}",
            next_execution.execution_id, changed_attempt.stage_id, changed_attempt.attempt
        );
        let commit = SurrealModelLaneRoutingCommit {
            expected_revision,
            expected_claim: expected_claim.map(routing_claim_to_surreal),
            execution: SurrealModelLaneRoutingExecutionWrite {
                execution_id: next_execution.execution_id.clone(),
                run_id: next_execution.run_id.clone(),
                revision: i64::try_from(next_execution.revision).map_err(|_| {
                    ModelLaneError::InvalidInput(
                        "routing next revision exceeds durable integer range".into(),
                    )
                })?,
                context_hash: routing_execution_context_hash(&next_execution)?,
                record_json: routing_execution_projection_json(&next_execution)?,
            },
            attempt: SurrealModelLaneRoutingAttemptWrite {
                attempt_id,
                execution_id: next_execution.execution_id.clone(),
                run_id: next_execution.run_id.clone(),
                stage_id: changed_attempt.stage_id.clone(),
                attempt: i64::from(changed_attempt.attempt),
                state: routing_stage_state_name(changed_attempt.state)?,
                lease_owner: changed_attempt.lease_owner.clone(),
                fencing_token: changed_attempt.fencing_token.clone(),
                lease_expires_at_unix_ms: changed_attempt
                    .lease_expires_at_unix_ms
                    .map(|value| i64::try_from(value))
                    .transpose()
                    .map_err(|_| {
                        ModelLaneError::InvalidInput(
                            "routing lease expiry exceeds durable integer range".into(),
                        )
                    })?,
                record_json: routing_attempt_projection_json(&changed_attempt)?,
            },
            outbox: SurrealModelLaneRoutingOutboxWrite {
                command_id,
                execution_id: next_execution.execution_id.clone(),
                run_id: next_execution.run_id.clone(),
                stage_id: changed_attempt.stage_id.clone(),
                attempt: i64::from(changed_attempt.attempt),
                status: outbox_status.to_owned(),
                lease_owner: changed_attempt.lease_owner.clone(),
                fencing_token: changed_attempt.fencing_token.clone(),
                lease_expires_at_unix_ms: changed_attempt
                    .lease_expires_at_unix_ms
                    .map(i64::try_from)
                    .transpose()
                    .map_err(|_| {
                        ModelLaneError::InvalidInput(
                            "routing lease expiry exceeds durable integer range".into(),
                        )
                    })?,
                created_at_unix_ms: i64::try_from(created_at_unix_ms).map_err(|_| {
                    ModelLaneError::InvalidInput(
                        "routing outbox creation time exceeds durable integer range".into(),
                    )
                })?,
                updated_at_unix_ms: i64::try_from(changed_attempt.updated_at_unix_ms).map_err(
                    |_| {
                        ModelLaneError::InvalidInput(
                            "routing outbox update time exceeds durable integer range".into(),
                        )
                    },
                )?,
            },
            events: routing_events_to_surreal(events)?,
            message: message_write,
            binding: binding_write,
            message_guard,
        };
        provider.commit_routing_atomic(commit, &scope).await?;
        let execution = self
            .routing_execution_snapshot(&next_execution.execution_id)
            .await?
            .ok_or_else(|| {
                ModelLaneError::IntegrityViolation(
                    "atomic routing commit readback omitted execution authority".into(),
                )
            })?;
        let stored_message = if let Some(message) = message.as_ref() {
            let stored = provider
                .get(SurrealRecordKind::Message, &message.message_id, &scope)
                .await?
                .ok_or_else(|| {
                    ModelLaneError::IntegrityViolation(
                        "atomic routing commit readback omitted message authority".into(),
                    )
                })?;
            provider
                .validate_event_link(SurrealRecordKind::Message, &stored, &scope)
                .await?;
            let stored = surreal_message_record(stored)?;
            self.validate_surreal_stored_message_authority(&stored, &scope)
                .await?;
            Some(stored)
        } else {
            None
        };
        let stored_binding = if let Some(binding) = binding.as_ref() {
            let stored = provider
                .get(
                    SurrealRecordKind::ContextArtifact,
                    &binding.artifact_binding_id,
                    &scope,
                )
                .await?
                .ok_or_else(|| {
                    ModelLaneError::IntegrityViolation(
                        "atomic routing commit readback omitted artifact binding authority".into(),
                    )
                })?;
            provider
                .validate_event_link(SurrealRecordKind::ContextArtifact, &stored, &scope)
                .await?;
            Some(surreal_context_bundle_artifact_record(stored)?)
        } else {
            None
        };
        Ok((execution, stored_message, stored_binding))
    }

    fn surreal_write_scope(&self, operation: &str) -> ModelLaneResult<SurrealModelLaneScope> {
        // Shape and lifecycle first (see surreal_read_scope), then write authority. All three
        // remain mandatory and all three fail closed; ordering only decides which cause is
        // reported, and a malformed five-field scope must not be reported as a write-authority
        // or authentication problem.
        let scope = self.surreal_read_scope(operation)?;
        if self.write_scope().is_none() {
            return Err(ModelLaneError::AuthorityDenied(format!(
                "{operation} requires scoped write authority"
            )));
        }
        Ok(scope)
    }

    fn surreal_read_scope(&self, operation: &str) -> ModelLaneResult<SurrealModelLaneScope> {
        // Shape before lifecycle. A scope missing any of the five fields can never be registered
        // active, so reporting it as an unknown authenticated context describes the wrong defect
        // and sends the caller looking at authentication instead of the malformed scope. Both
        // checks remain mandatory and both fail closed; only the reported cause changes.
        let exact = self.access.exact_read_scope().ok_or_else(|| {
            ModelLaneError::AuthorityDenied(format!(
                "{operation} requires exact owner, Principal, authenticated session, AccessSpace, and workspace authority"
            ))
        })?;
        self.access.require_lifecycle_active()?;
        Ok(SurrealModelLaneScope {
            owner_account_id: exact.owner_account_id.as_uuid().to_string(),
            actor_principal_id: exact.actor_principal_id.as_uuid().to_string(),
            authenticated_session_id: exact.authenticated_session_id.as_uuid().to_string(),
            access_space_id: exact.access_space_id.as_uuid().to_string(),
            workspace_id: exact.workspace_id.as_str().to_owned(),
        })
    }

    pub async fn claim_crdt_lease(
        &self,
        input: NewModelLaneCrdtLease,
    ) -> ModelLaneResult<ModelLaneCrdtLeaseClaimOutcome> {
        let scope = self.surreal_write_scope("claim CRDT document lease")?;
        validate_new_crdt_lease(&input)?;
        let actor = validated_crdt_actor(&input.actor_id, &input.actor_kind)?;
        let lane = self
            .provider()
            .await?
            .get(SurrealRecordKind::Lane, &input.lane_id, &scope)
            .await?
            .ok_or_else(|| crdt_authority_denied("CRDT lease source lane is unavailable"))?;
        let lane = surreal_lane_record(lane)?;
        if lane.run_id != input.kernel_task_run_id || lane.session_id != input.session_id {
            return Err(crdt_authority_denied(
                "CRDT lease source lane does not own the requested run and session",
            ));
        }
        let event = new_surreal_crdt_event(
            &scope,
            &input.kernel_task_run_id,
            &input.session_id,
            "knowledge_crdt_lease",
            &input.lease_id,
            &input.idempotency_key,
            KernelEventType::KnowledgeCrdtLeaseClaimed,
            actor,
            Some(input.correlation_id.clone()),
            json!({
                "lease_id": input.lease_id,
                "lane_id": input.lane_id,
                "document_id": input.document_id,
                "crdt_document_id": input.crdt_document_id,
                "actor_id": input.actor_id,
                "session_id": input.session_id,
                "scope_kind": "document",
                "scope_id": input.crdt_document_id,
                "ttl_seconds": input.ttl_seconds,
            }),
        )?;
        let row = SurrealModelLaneCrdtLeaseWrite {
            lease_id: input.lease_id,
            owner_account_id: scope.owner_account_id.clone(),
            actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(),
            access_space_id: scope.access_space_id.clone(),
            workspace_id: scope.workspace_id.clone(),
            document_id: Some(input.document_id),
            crdt_document_id: Some(input.crdt_document_id.clone()),
            lane_id: input.lane_id,
            actor_id: input.actor_id,
            actor_kind: input.actor_kind,
            session_id: input.session_id,
            correlation_id: input.correlation_id,
            scope_kind: "document".to_owned(),
            scope_id: input.crdt_document_id,
            takeover_of: None,
        };
        let outcome = self
            .provider()
            .await?
            .claim_crdt_lease_atomic(row, input.ttl_seconds, event, &scope)
            .await?;
        Ok(match outcome {
            SurrealCrdtLeaseClaimOutcome::Claimed(row) => {
                ModelLaneCrdtLeaseClaimOutcome::Claimed(crdt_lease_record(row))
            }
            SurrealCrdtLeaseClaimOutcome::AlreadyClaimed(row) => {
                ModelLaneCrdtLeaseClaimOutcome::AlreadyClaimed(crdt_lease_record(row))
            }
            SurrealCrdtLeaseClaimOutcome::ScopeHeld(row) => {
                ModelLaneCrdtLeaseClaimOutcome::ScopeHeld(crdt_lease_record(row))
            }
        })
    }

    pub async fn append_crdt_update(
        &self,
        input: NewModelLaneCrdtUpdate,
    ) -> ModelLaneResult<ModelLaneCrdtUpdateAppendOutcome> {
        let scope = self.surreal_write_scope("append CRDT Yjs update")?;
        validate_new_crdt_update(&input, &scope)?;
        let actor = validated_crdt_actor(&input.actor_id, &input.actor_kind)?;
        let update_sha256 = dexterity_sha256_hex(&input.update_bytes);
        let event_payload = json!({
            "document_id": input.document_id,
            "crdt_document_id": input.crdt_document_id,
            "update_id": input.update_id,
            "update_seq": input.update_seq,
            "actor_id": input.actor_id,
            "update_sha256": update_sha256,
            "state_vector_before": input.state_vector_before,
            "state_vector_after": input.state_vector_after,
            "site_id": input.site_id,
        });
        let event = new_surreal_crdt_event(
            &scope,
            &input.kernel_task_run_id,
            &input.session_id,
            "knowledge_crdt_document",
            &input.crdt_document_id,
            &input.idempotency_key,
            KernelEventType::KnowledgeCrdtUpdateRecorded,
            actor,
            Some(input.trace_id.clone()),
            event_payload,
        )?;
        let update_bytes_ref = scoped_crdt_ref("kernel_crdt_updates", &input.update_id, &scope);
        let row = SurrealModelLaneCrdtUpdate {
            schema_id: input.schema_id,
            owner_account_id: scope.owner_account_id.clone(),
            actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(),
            access_space_id: scope.access_space_id.clone(),
            workspace_id: scope.workspace_id.clone(),
            document_id: input.document_id,
            crdt_document_id: input.crdt_document_id.clone(),
            update_id: input.update_id.clone(),
            update_seq: input.update_seq,
            update_sha256: update_sha256.clone(),
            update_bytes_ref,
            update_bytes_b64: base64::engine::general_purpose::STANDARD.encode(&input.update_bytes),
            actor_id: input.actor_id.clone(),
            actor_kind: input.actor_kind,
            session_id: input.session_id,
            trace_id: input.trace_id,
            state_vector_before: input.state_vector_before.clone(),
            state_vector_after: input.state_vector_after.clone(),
            replay_order_key: input.replay_order_key,
            dependency_update_ids: input.dependency_update_ids,
            replay_encoding: "yjs-update-v1".to_owned(),
            replay_schema_version: "kernel-crdt-update-v1".to_owned(),
            event_ledger_stream_id: format!("knowledge-crdt:{}", input.crdt_document_id),
            event_ledger_event_id: event.event_id.clone(),
            storage_authority: "embedded_surrealdb".to_owned(),
            ledger_session_run_id: event.session_run_id.clone(),
            ledger_event_type: event.event_type.clone(),
            ledger_aggregate_type: event.aggregate_type.clone(),
            ledger_aggregate_id: event.aggregate_id.clone(),
            ledger_actor_kind: event.actor_kind.clone(),
            ledger_actor_id: event.actor_id.clone(),
            ledger_correlation_id: event.correlation_id.clone(),
            ledger_payload_hash: event.payload_hash.clone(),
            ledger_update_id: input.update_id,
            ledger_update_seq: input.update_seq,
            ledger_actor_payload_id: input.actor_id,
            ledger_update_sha256: update_sha256,
            ledger_state_vector_before: input.state_vector_before,
            ledger_state_vector_after: input.state_vector_after,
            ledger_site_id: input.site_id,
        };
        let expected_head_update_seq = input.update_seq - 1;
        let expected_head_state_vector = row.state_vector_before.clone();
        let outcome = self
            .provider()
            .await?
            .append_crdt_update_atomic(
                expected_head_update_seq,
                &expected_head_state_vector,
                row,
                event,
                &scope,
            )
            .await?;
        Ok(crdt_update_append_outcome(outcome))
    }

    pub async fn append_crdt_snapshot(
        &self,
        input: NewModelLaneCrdtSnapshot,
    ) -> ModelLaneResult<ModelLaneCrdtSnapshotRecord> {
        let scope = self.surreal_write_scope("append CRDT Yjs snapshot")?;
        validate_new_crdt_snapshot(&input)?;
        let actor = validated_crdt_actor(&input.actor_id, &input.actor_kind)?;
        let snapshot_sha256 = dexterity_sha256_hex(&input.snapshot_bytes);
        let event = new_surreal_crdt_event(
            &scope,
            &input.kernel_task_run_id,
            &input.session_id,
            "knowledge_crdt_document",
            &input.crdt_document_id,
            &input.idempotency_key,
            KernelEventType::KnowledgeCrdtSnapshotRecorded,
            actor,
            None,
            json!({
                "snapshot_id": &input.snapshot_id,
                "document_id": &input.document_id,
                "crdt_document_id": &input.crdt_document_id,
                "covered_update_seq": input.covered_update_seq,
                "state_vector": &input.state_vector,
                "snapshot_sha256": &snapshot_sha256,
            }),
        )?;
        let row = SurrealModelLaneCrdtSnapshot {
            schema_id: input.schema_id,
            snapshot_id: input.snapshot_id.clone(),
            owner_account_id: scope.owner_account_id.clone(),
            actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(),
            access_space_id: scope.access_space_id.clone(),
            workspace_id: scope.workspace_id.clone(),
            document_id: input.document_id.clone(),
            crdt_document_id: input.crdt_document_id.clone(),
            covered_update_seq: input.covered_update_seq,
            state_vector: input.state_vector.clone(),
            snapshot_sha256: snapshot_sha256.clone(),
            snapshot_bytes_ref: scoped_crdt_ref(
                "kernel_crdt_snapshots",
                &input.snapshot_id,
                &scope,
            ),
            snapshot_bytes_b64: base64::engine::general_purpose::STANDARD
                .encode(input.snapshot_bytes),
            actor_id: input.actor_id.clone(),
            actor_kind: input.actor_kind,
            event_ledger_stream_id: format!("knowledge-crdt:{}", input.crdt_document_id),
            event_ledger_event_id: event.event_id.clone(),
            promotion_evidence_update_ids: input.promotion_evidence_update_ids,
            storage_authority: "embedded_surrealdb".to_owned(),
            ledger_event_type: event.event_type.clone(),
            ledger_aggregate_type: event.aggregate_type.clone(),
            ledger_aggregate_id: event.aggregate_id.clone(),
            ledger_actor_kind: event.actor_kind.clone(),
            ledger_actor_id: event.actor_id.clone(),
            ledger_payload_hash: event.payload_hash.clone(),
            ledger_document_id: input.document_id,
            ledger_state_vector: input.state_vector,
            ledger_covered_update_seq: input.covered_update_seq,
        };
        let stored = self
            .provider()
            .await?
            .append_crdt_snapshot_atomic(row, event, &scope)
            .await?;
        Ok(crdt_snapshot_record(stored))
    }

    pub async fn record_crdt_proposal(
        &self,
        input: NewModelLaneCrdtProposal,
    ) -> ModelLaneResult<ModelLaneCrdtProposalRecord> {
        let scope = self.surreal_write_scope("record CRDT AI edit proposal")?;
        validate_new_crdt_proposal(&input)?;
        let actor = validated_crdt_actor(&input.actor_id, &input.actor_kind)?;
        let diff_sha256 = dexterity_sha256_hex(&canonical_json_bytes(&input.proposed_diff));
        let event = new_surreal_crdt_event(
            &scope,
            &input.kernel_task_run_id,
            &input.session_id,
            "knowledge_crdt_ai_edit_proposal",
            &input.proposal_id,
            &input.idempotency_key,
            KernelEventType::AiEditProposalRecorded,
            actor,
            Some(input.correlation_id.clone()),
            json!({
                "proposal_id": &input.proposal_id,
                "document_id": &input.document_id,
                "crdt_document_id": &input.crdt_document_id,
                "base_update_seq": input.base_update_seq,
                "base_state_vector": &input.base_state_vector,
                "diff_sha256": &diff_sha256,
                "actor_id": &input.actor_id,
                "lease_id": &input.lease_id,
            }),
        )?;
        let row = SurrealModelLaneCrdtProposalWrite {
            proposal_id: input.proposal_id,
            owner_account_id: scope.owner_account_id.clone(),
            actor_principal_id: scope.actor_principal_id.clone(),
            authenticated_session_id: scope.authenticated_session_id.clone(),
            access_space_id: scope.access_space_id.clone(),
            workspace_id: scope.workspace_id.clone(),
            document_id: input.document_id,
            crdt_document_id: input.crdt_document_id,
            base_update_seq: input.base_update_seq,
            base_state_vector: input.base_state_vector,
            proposed_diff: input.proposed_diff,
            diff_sha256,
            source_span_citations: input.source_span_citations,
            actor_id: input.actor_id,
            actor_kind: input.actor_kind,
            session_id: input.session_id,
            correlation_id: input.correlation_id,
            lease_id: input.lease_id,
        };
        let stored = self
            .provider()
            .await?
            .record_crdt_proposal_atomic(row, event, &scope)
            .await?;
        Ok(crdt_proposal_record(stored))
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn decide_crdt_proposal(
        &self,
        proposal_id: &str,
        decision: ModelLaneCrdtProposalDecision,
        decided_by: &str,
        decision_reason: Option<String>,
        kernel_task_run_id: &str,
        session_run_id: &str,
        idempotency_key: &str,
    ) -> ModelLaneResult<Option<ModelLaneCrdtProposalRecord>> {
        let scope = self.surreal_write_scope("decide CRDT AI edit proposal")?;
        for (field, value) in [
            ("proposal_id", proposal_id),
            ("decided_by", decided_by),
            ("kernel_task_run_id", kernel_task_run_id),
            ("session_run_id", session_run_id),
            ("idempotency_key", idempotency_key),
        ] {
            require_token(field, value)?;
        }
        let Some(current) = self
            .provider()
            .await?
            .crdt_proposal_record(proposal_id, &scope)
            .await?
        else {
            return Ok(None);
        };
        if decision == ModelLaneCrdtProposalDecision::Promoted
            && (!matches!(current.review_state.as_str(), "approved" | "promoted")
                || current.applied_update_id.is_none()
                || current.applied_update_sha256.as_deref() != Some(current.diff_sha256.as_str()))
        {
            return Err(crdt_authority_denied(
                "CRDT proposal promotion requires an approved applied proposal",
            ));
        }
        let (event, promotion_accepted_event) =
            if decision == ModelLaneCrdtProposalDecision::Promoted {
                let requested_idempotency_key = format!("{idempotency_key}:requested");
                let accepted_idempotency_key = format!("{idempotency_key}:accepted");
                let requested = new_surreal_crdt_promotion_event(
                    &scope,
                    kernel_task_run_id,
                    session_run_id,
                    proposal_id,
                    &requested_idempotency_key,
                    KernelEventType::PromotionRequested,
                    decided_by,
                    &current.correlation_id,
                    None,
                    json!({
                        "proposal_id": proposal_id,
                        "diff_sha256": &current.diff_sha256,
                        "base_update_seq": current.base_update_seq,
                        "base_state_vector": &current.base_state_vector,
                        "decided_by": decided_by,
                        "promotion_reason": &decision_reason,
                    }),
                )?;
                let accepted = new_surreal_crdt_promotion_event(
                    &scope,
                    kernel_task_run_id,
                    session_run_id,
                    proposal_id,
                    &accepted_idempotency_key,
                    KernelEventType::PromotionAccepted,
                    decided_by,
                    &current.correlation_id,
                    Some(requested.event_id.clone()),
                    json!({
                        "proposal_id": proposal_id,
                        "review_state": decision.as_str(),
                        "decided_by": decided_by,
                        "promotion_reason": &decision_reason,
                        "source_span_citations": &current.source_span_citations,
                        "diff_sha256": &current.diff_sha256,
                        "applied_update_id": &current.applied_update_id,
                        "applied_update_sha256": &current.applied_update_sha256,
                    }),
                )?;
                (requested, Some(accepted))
            } else {
                (
                    new_surreal_crdt_event(
                        &scope,
                        kernel_task_run_id,
                        session_run_id,
                        "knowledge_crdt_ai_edit_proposal",
                        proposal_id,
                        idempotency_key,
                        KernelEventType::AiEditProposalDecided,
                        KernelActor::Operator(decided_by.to_owned()),
                        Some(current.correlation_id.clone()),
                        json!({
                            "proposal_id": proposal_id,
                            "review_state": decision.as_str(),
                            "decided_by": decided_by,
                            "decision_reason": decision_reason,
                            "diff_sha256": &current.diff_sha256,
                            "applied_update_id": &current.applied_update_id,
                            "applied_update_sha256": &current.applied_update_sha256,
                        }),
                    )?,
                    None,
                )
            };
        let stored = self
            .provider()
            .await?
            .decide_crdt_proposal_atomic(
                proposal_id,
                decision.as_str(),
                decided_by,
                decision_reason,
                event,
                promotion_accepted_event,
                &scope,
            )
            .await?;
        Ok(stored.map(crdt_proposal_record))
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn bind_crdt_proposal_update(
        &self,
        proposal_id: &str,
        applied_update_id: &str,
        kernel_task_run_id: &str,
        idempotency_key: &str,
    ) -> ModelLaneResult<Option<ModelLaneCrdtProposalRecord>> {
        let scope = self.surreal_write_scope("bind CRDT proposal to Yjs update")?;
        for (field, value) in [
            ("proposal_id", proposal_id),
            ("applied_update_id", applied_update_id),
            ("kernel_task_run_id", kernel_task_run_id),
            ("idempotency_key", idempotency_key),
        ] {
            require_token(field, value)?;
        }
        let provider = self.provider().await?;
        let Some(proposal) = provider.crdt_proposal_record(proposal_id, &scope).await? else {
            return Ok(None);
        };
        let Some(update) = provider
            .crdt_update_by_id(applied_update_id, &scope)
            .await?
        else {
            return Err(crdt_authority_denied(
                "applied CRDT update authority is unavailable",
            ));
        };
        validate_surreal_crdt_update_row(&update, &scope)?;
        if proposal.document_id != update.document_id
            || proposal.crdt_document_id != update.crdt_document_id
            || proposal.actor_id != update.actor_id
            || proposal.actor_kind != update.actor_kind
            || proposal.session_id != update.session_id
            || proposal.correlation_id != update.trace_id
            || update.update_seq != proposal.base_update_seq + 1
            || update.state_vector_before != proposal.base_state_vector
        {
            return Err(crdt_authority_denied(
                "proposal and applied Yjs update authority do not share terminal identity",
            ));
        }
        let actor = validated_crdt_actor(&proposal.actor_id, &proposal.actor_kind)?;
        let event = new_surreal_crdt_event(
            &scope,
            kernel_task_run_id,
            &proposal.session_id,
            "knowledge_crdt_ai_edit_proposal",
            proposal_id,
            idempotency_key,
            KernelEventType::AiEditProposalDecided,
            actor,
            Some(proposal.correlation_id.clone()),
            json!({
                "proposal_id": proposal_id,
                "applied_update_id": applied_update_id,
                "applied_update_sha256": proposal.diff_sha256,
                "approved_diff_sha256": proposal.diff_sha256,
                "yjs_update_sha256": update.update_sha256,
            }),
        )?;
        let stored = provider
            .bind_crdt_proposal_update_atomic(
                proposal_id,
                applied_update_id,
                &proposal.diff_sha256,
                &proposal.actor_id,
                event,
                &scope,
            )
            .await?;
        Ok(stored.map(crdt_proposal_record))
    }

    pub async fn crdt_proposal(
        &self,
        proposal_id: &str,
    ) -> ModelLaneResult<Option<ModelLaneCrdtProposalRecord>> {
        require_token("proposal_id", proposal_id)?;
        let scope = self.surreal_read_scope("read CRDT proposal")?;
        self.provider()
            .await?
            .crdt_proposal_record(proposal_id, &scope)
            .await
            .map(|row| row.map(crdt_proposal_record))
            .map_err(Into::into)
    }

    /// Persist the model-selection projection and its canonical EventLedger
    /// receipt in one scoped Surreal transaction.
    pub async fn record_selection_audit_atomic(
        &self,
        input: NewModelLaneSelectionAudit,
    ) -> ModelLaneResult<ModelLaneSelectionAuditRecord> {
        for (field, value) in [
            ("audit_id", input.audit_id.as_str()),
            ("run_id", input.run_id.as_str()),
            ("selected_model_id", input.selected_model_id.as_str()),
            ("actor_ref", input.actor_ref.as_str()),
            ("reason", input.reason.as_str()),
            (
                "event_ledger_stream_id",
                input.event_ledger_stream_id.as_str(),
            ),
            ("work_packet_id", input.work_packet_id.as_str()),
            ("micro_task_id", input.micro_task_id.as_str()),
            ("task_board_id", input.task_board_id.as_str()),
            ("owner_session", input.owner_session.as_str()),
            ("idempotency_key", input.idempotency_key.as_str()),
            ("created_at_utc", input.created_at_utc.as_str()),
        ] {
            require_token(field, value)?;
        }
        parse_utc("created_at_utc", &input.created_at_utc)?;
        if !input.selection_context.is_object() {
            return Err(ModelLaneError::InvalidInput(
                "selection_context must be a JSON object".into(),
            ));
        }
        let scope = self.surreal_write_scope("record model selection audit")?;
        let provider = self.provider().await?;
        let run_row = provider
            .get(SurrealRecordKind::Run, &input.run_id, &scope)
            .await?
            .ok_or_else(|| ModelLaneError::NotFound(format!("run_id {}", input.run_id)))?;
        provider
            .validate_event_link(SurrealRecordKind::Run, &run_row, &scope)
            .await?;
        let run = surreal_run_record(run_row)?;
        require_equal(
            "selection_audit.event_ledger_stream_id",
            &input.event_ledger_stream_id,
            "run.event_ledger_stream_id",
            &run.event_ledger_stream_id,
        )?;
        for (field, requested, canonical) in [
            (
                "work_packet_id",
                input.work_packet_id.as_str(),
                run.work_packet_id.as_deref(),
            ),
            (
                "micro_task_id",
                input.micro_task_id.as_str(),
                run.micro_task_id.as_deref(),
            ),
            (
                "task_board_id",
                input.task_board_id.as_str(),
                run.task_board_id.as_deref(),
            ),
            (
                "owner_session",
                input.owner_session.as_str(),
                Some(run.owner_session.as_str()),
            ),
        ] {
            let canonical = canonical.ok_or_else(|| {
                ModelLaneError::AuthorityDenied(
                    "selection audit requires complete canonical run Locus authority".into(),
                )
            })?;
            require_equal(
                &format!("selection_audit.{field}"),
                requested,
                &format!("run.{field}"),
                canonical,
            )?;
        }
        let prepared = ModelLaneSelectionAuditRecord {
            inner: input,
            event_ledger_event_id: String::new(),
            event_ledger_seq: 0,
            event_stream_version: 0,
            transaction_seq: 0,
        };
        self.access.require_lifecycle_active()?;
        let stored = provider
            .put_immutable(
                SurrealRecordKind::SelectionAudit,
                &prepared.audit_id,
                &prepared.run_id,
                &prepared.idempotency_key,
                serde_json::to_string(&prepared)?,
                vec![
                    format!("selected_model_id={}", prepared.selected_model_id),
                    format!("actor_ref={}", prepared.actor_ref),
                    format!("micro_task_id={}", prepared.micro_task_id),
                ],
                scoped_surreal_event_payload(
                    "hsk.model_lane_selection_audit@1",
                    &prepared,
                    &scope,
                )?,
                &scope,
            )
            .await?;
        provider
            .validate_event_link(SurrealRecordKind::SelectionAudit, &stored, &scope)
            .await?;
        surreal_selection_audit_record(stored)
    }

    pub async fn selection_audits_for_run(
        &self,
        run_id: &str,
    ) -> ModelLaneResult<Vec<ModelLaneSelectionAuditRecord>> {
        require_token("run_id", run_id)?;
        let scope = self.surreal_read_scope("read model selection audits")?;
        let provider = self.provider().await?;
        let rows = provider
            .list_run(SurrealRecordKind::SelectionAudit, run_id, &scope)
            .await?;
        let mut records = Vec::with_capacity(rows.len());
        for row in rows {
            provider
                .validate_event_link(SurrealRecordKind::SelectionAudit, &row, &scope)
                .await?;
            records.push(surreal_selection_audit_record(row)?);
        }
        Ok(records)
    }

    #[cfg(feature = "surreal-test-support")]
    pub async fn test_cleanup_receipts_bounded(
        &self,
        max_rows: usize,
    ) -> ModelLaneResult<Vec<ModelLaneCleanupReceiptInspection>> {
        let scope = self.surreal_read_scope("inspect bounded cleanup receipts")?;
        let provider = self.provider().await?;
        let rows = provider
            .test_list_kind_bounded(SurrealRecordKind::SessionCleanupReceipt, max_rows, &scope)
            .await?;
        let mut receipts = Vec::with_capacity(rows.len());
        for row in rows {
            provider
                .validate_event_link(SurrealRecordKind::SessionCleanupReceipt, &row, &scope)
                .await?;
            let receipt = cleanup_receipt_from_surreal(&row)?;
            receipts.push(ModelLaneCleanupReceiptInspection {
                instance_id: receipt.instance_id,
                lane_id: receipt.lane_id,
                process_uuid: receipt.process_uuid,
                terminal_event_id: receipt.terminal_event_id,
                resource_evicted_event_id: receipt.resource_evicted_event_id,
                status: receipt.status,
                terminal_state: receipt.terminal_state,
                reason: receipt.reason,
                exit_code: receipt.exit_code,
                last_error: receipt.last_error,
            });
        }
        Ok(receipts)
    }

    #[cfg(feature = "surreal-test-support")]
    pub async fn test_scoped_authority_receipts(
        &self,
        run_id: &str,
        max_rows: usize,
    ) -> ModelLaneResult<Vec<ModelLaneScopedAuthorityReceipt>> {
        let scope = self.surreal_read_scope("inspect scoped canonical authority receipts")?;
        let rows = self
            .provider()
            .await?
            .test_scoped_authority_receipts(run_id, max_rows, &scope)
            .await?;
        Ok(rows
            .into_iter()
            .map(|row| ModelLaneScopedAuthorityReceipt {
                record_kind: row.record_kind,
                aggregate_id: row.aggregate_id,
                run_id: row.run_id,
                event_id: row.event_id,
                event_ledger_seq: row.event_ledger_seq,
                event_type: row.event_type,
                payload_hash: row.payload_hash,
                owner_account_id: row.owner_account_id,
                actor_principal_id: row.actor_principal_id,
                authenticated_session_id: row.authenticated_session_id,
                access_space_id: row.access_space_id,
                workspace_id: row.workspace_id,
            })
            .collect())
    }

    #[cfg(feature = "surreal-test-support")]
    pub async fn test_corrupt_scoped_authority(
        &self,
        record_kind: &str,
        aggregate_id: &str,
        corruption: ModelLaneAuthorityTestCorruption,
    ) -> ModelLaneResult<()> {
        let kind = surreal_record_kind(record_kind).ok_or_else(|| {
            ModelLaneError::InvalidInput(format!(
                "unsupported ModelLane authority record kind {record_kind}"
            ))
        })?;
        let scope = self.surreal_write_scope("corrupt scoped authority for denial proof")?;
        self.provider()
            .await?
            .test_corrupt_scoped_authority(kind, aggregate_id, corruption.as_str(), &scope)
            .await?;
        Ok(())
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
        let scope = self.surreal_write_scope("record swarm-session cleanup receipt")?;
        let provider = self.provider().await?;
        let run_id = if let Some(lane_id) = lane_id {
            let lane = provider
                .get(SurrealRecordKind::Lane, lane_id, &scope)
                .await?
                .ok_or_else(|| ModelLaneError::NotFound(format!("lane_id {lane_id}")))?;
            provider
                .validate_event_link(SurrealRecordKind::Lane, &lane, &scope)
                .await?;
            lane.run_id
        } else {
            process_uuid.to_string()
        };
        let record = serde_json::json!({
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
        let record_json = serde_json::to_string(&record)?;
        let search_terms = vec![
            format!("instance_id={instance_id}"),
            format!("process_uuid={process_uuid}"),
            format!("status={status}"),
        ];
        let stored = if let Some(existing) = provider
            .get(SurrealRecordKind::SessionCleanupReceipt, instance_id, &scope)
            .await?
        {
            provider
                .validate_event_link(
                    SurrealRecordKind::SessionCleanupReceipt,
                    &existing,
                    &scope,
                )
                .await?;
            let prior = cleanup_receipt_from_surreal(&existing)?;
            if prior.instance_id != instance_id
                || prior.lane_id.as_deref() != lane_id
                || prior.process_uuid != process_uuid
                || prior.terminal_event_id != terminal_event_id
                || prior.resource_evicted_event_id != resource_evicted_event_id
            {
                return Err(ModelLaneError::IdempotencyConflict(format!(
                    "cleanup receipt {instance_id} immutable authority changed"
                )));
            }
            provider
                .replace(
                    SurrealRecordKind::SessionCleanupReceipt,
                    instance_id,
                    record_json,
                    search_terms,
                    serde_json::to_string(&record)?,
                    &scope,
                )
                .await?
                .ok_or_else(|| {
                    ModelLaneError::IntegrityViolation(format!(
                        "cleanup receipt {instance_id} disappeared during replacement"
                    ))
                })?
        } else {
            provider
                .put_immutable(
                    SurrealRecordKind::SessionCleanupReceipt,
                    instance_id,
                    &run_id,
                    &format!("session-cleanup:{instance_id}"),
                    record_json,
                    search_terms,
                    serde_json::to_string(&record)?,
                    &scope,
                )
                .await?
        };
        provider
            .validate_event_link(
                SurrealRecordKind::SessionCleanupReceipt,
                &stored,
                &scope,
            )
            .await?;
        Ok(())
    }

    /// Durable cleanup intents that survived a coordinator/runtime restart.
    /// Boot reconciliation is restricted to the exact product resource scope;
    /// unattributed and foreign-scope receipts are deliberately invisible.
    pub(crate) async fn pending_session_cleanup_receipts(
        &self,
    ) -> ModelLaneResult<Vec<DurableSessionCleanupReceipt>> {
        let scope = self.surreal_read_scope("read pending swarm-session cleanup receipts")?;
        let provider = self.provider().await?;
        let rows = provider
            .list_kind(SurrealRecordKind::SessionCleanupReceipt, &scope)
            .await?;
        let mut receipts = Vec::new();
        for row in rows {
            provider
                .validate_event_link(
                    SurrealRecordKind::SessionCleanupReceipt,
                    &row,
                    &scope,
                )
                .await?;
            let receipt = cleanup_receipt_from_surreal(&row)?;
            if receipt.status != "completed" {
                receipts.push(receipt);
            }
        }
        Ok(receipts)
    }

    pub(crate) async fn cleanup_process_is_durably_closed(
        &self,
        process_uuid: Uuid,
    ) -> ModelLaneResult<bool> {
        let scope = self.surreal_read_scope("verify swarm-session cleanup process closure")?;
        self.provider()
            .await?
            .process_is_durably_closed(process_uuid, &scope)
            .await
            .map_err(Into::into)
    }

    pub(crate) async fn session_cleanup_completed(
        &self,
        instance_id: &str,
        terminal_state: &str,
        reason: &str,
    ) -> ModelLaneResult<bool> {
        let scope = self.surreal_read_scope("verify swarm-session cleanup completion")?;
        let provider = self.provider().await?;
        let Some(row) = provider
            .get(
                SurrealRecordKind::SessionCleanupReceipt,
                instance_id,
                &scope,
            )
            .await?
        else {
            return Ok(false);
        };
        provider
            .validate_event_link(SurrealRecordKind::SessionCleanupReceipt, &row, &scope)
            .await?;
        let receipt = cleanup_receipt_from_surreal(&row)?;
        Ok(receipt.status == "completed"
            && receipt.terminal_state == terminal_state
            && receipt.reason == reason)
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
        }
        let stored_run = self
            .record_or_extend_run_surreal(records.0, &records.1)
            .await?;
        let stored_lane = self.record_lane(records.1).await?;
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
        let scope = self.surreal_write_scope("ModelLaneRun write")?;
        let event_payload_json = scoped_surreal_event_payload(
            "hsk.model_lane_run@1",
            &input,
            &scope,
        )?;
        let stored = self
            .provider()
            .await?
            .put_immutable(
                SurrealRecordKind::Run,
                &input.run_id,
                &input.run_id,
                &input.idempotency_key,
                serde_json::to_string(&input)?,
                run_search_terms(&input),
                event_payload_json,
                &scope,
            )
            .await?;
        surreal_run_record(stored)
    }

    async fn record_or_extend_run_surreal(
        &self,
        input: NewModelLaneRun,
        lane: &NewModelLane,
    ) -> ModelLaneResult<ModelLaneRunRecord> {
        let scope = self.surreal_write_scope("ModelLaneRun lane attachment")?;
        let provider = self.provider().await?;
        let Some(existing) = provider
            .get(SurrealRecordKind::Run, &input.run_id, &scope)
            .await?
        else {
            return self.record_run(input).await;
        };
        let existing = surreal_run_record(existing)?;
        let Some(merged) = merge_run_for_lane(&existing, input, lane)? else {
            return Ok(existing);
        };
        let event_payload_json = scoped_surreal_event_payload(
            "hsk.model_lane_run_extension@1",
            &merged,
            &scope,
        )?;
        let stored = provider
            .replace(
                SurrealRecordKind::Run,
                &merged.run_id,
                serde_json::to_string(&merged)?,
                run_search_terms(&merged),
                event_payload_json,
                &scope,
            )
            .await?
            .ok_or_else(|| {
                ModelLaneError::AuthorityDenied(
                    "ModelLaneRun authority disappeared during lane attachment".into(),
                )
            })?;
        surreal_run_record(stored)
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
        }
        let scope = self.surreal_write_scope("ModelLane write")?;
        let provider = self.provider().await?;
        if let Some(existing) = provider
            .get(SurrealRecordKind::Lane, &input.lane_id, &scope)
            .await?
        {
            let existing = surreal_lane_record(existing)?;
            if existing.inner == input {
                return Ok(existing);
            }
            validate_lane_restart(&existing, &input)?;
            let event_payload_json = scoped_surreal_event_payload(
                "hsk.model_lane@1",
                &input,
                &scope,
            )?;
            let stored = provider
                .replace(
                    SurrealRecordKind::Lane,
                    &input.lane_id,
                    serde_json::to_string(&input)?,
                    lane_search_terms(&input),
                    event_payload_json,
                    &scope,
                )
                .await?
                .ok_or_else(|| {
                    ModelLaneError::AuthorityDenied(
                        "ModelLane restart authority disappeared during replace".into(),
                    )
                })?;
            return surreal_lane_record(stored);
        }
        let event_idempotency_key = lane_event_idempotency_key(&input);
        let event_payload_json = scoped_surreal_event_payload(
            "hsk.model_lane@1",
            &input,
            &scope,
        )?;
        let stored = provider
            .put_immutable(
                SurrealRecordKind::Lane,
                &input.lane_id,
                &input.run_id,
                &event_idempotency_key,
                serde_json::to_string(&input)?,
                lane_search_terms(&input),
                event_payload_json,
                &scope,
            )
            .await?;
        surreal_lane_record(stored)
    }

    pub async fn record_message(
        &self,
        input: NewModelLaneMessage,
    ) -> ModelLaneResult<ModelLaneMessageRecord> {
        validate_message(&input)?;
        let scope = self.surreal_write_scope("ModelLaneMessage write")?;
        let provider = self.provider().await?;
        let idempotency_term = format!("idempotency_key={}", input.idempotency_key);
        let existing = provider
            .find_by_term(SurrealRecordKind::Message, &idempotency_term, &scope)
            .await?;
        if existing.len() > 1 {
            return Err(ModelLaneError::IntegrityViolation(format!(
                "ModelLaneMessage idempotency_key {} resolves to multiple scoped records",
                input.idempotency_key
            )));
        }
        if let Some(existing) = existing.into_iter().next() {
            let existing =
                validate_surreal_message_retry(surreal_message_record(existing)?, &input)?;
            self.validate_surreal_stored_message_authority(&existing, &scope)
                .await?;
            return Ok(existing);
        }
        let authority = self
            .validate_surreal_message_authority(&input, &scope)
            .await?;
        let message_write =
            surreal_message_write(&input, authority.crdt_binding, &scope)?;
        let (stored, payload_binding) = provider
            .put_message_immutable_guarded(message_write, None, authority.guard, &scope)
            .await?;
        if payload_binding.is_some() {
            return Err(ModelLaneError::IntegrityViolation(
                "single ModelLaneMessage write returned an unexpected payload binding".into(),
            ));
        }
        let stored = surreal_message_record(stored)?;
        self.validate_surreal_stored_message_authority(&stored, &scope)
            .await?;
        Ok(stored)
    }

    async fn validate_surreal_message_authority(
        &self,
        input: &NewModelLaneMessage,
        scope: &SurrealModelLaneScope,
    ) -> ModelLaneResult<SurrealResolvedMessageAuthority> {
        let provider = self.provider().await?;
        let source_lane_row = provider
            .get(SurrealRecordKind::Lane, &input.from_lane_id, scope)
            .await?
            .ok_or_else(|| {
                ModelLaneError::AuthorityDenied("ModelLaneMessage source lane unavailable".into())
            })?;
        let source_lane_record_json = source_lane_row.record_json.clone();
        let source_lane = surreal_lane_record(source_lane_row)?;
        require_equal(
            "message.run_id",
            &input.run_id,
            "source_lane.run_id",
            &source_lane.run_id,
        )?;
        ensure_message_lane_is_live(&source_lane, "source")?;
        let source_run = provider
            .get(SurrealRecordKind::Run, &input.run_id, scope)
            .await?
            .ok_or_else(|| {
                ModelLaneError::AuthorityDenied("ModelLaneMessage source run unavailable".into())
            })?;
        let source_run = surreal_run_record(source_run)?;
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
        if let ModelLaneTarget::Lane(target_lane_id) = &input.to_lane {
            let target_lane = provider
                .get(SurrealRecordKind::Lane, target_lane_id, scope)
                .await?
                .ok_or_else(|| {
                    ModelLaneError::AuthorityDenied(
                        "ModelLaneMessage target lane unavailable".into(),
                    )
                })?;
            let target_lane = surreal_lane_record(target_lane)?;
            require_equal(
                "message.run_id",
                &input.run_id,
                "target_lane.run_id",
                &target_lane.run_id,
            )?;
            ensure_message_lane_is_live(&target_lane, "target")?;
            require_equal(
                "message.event_ledger_stream_id",
                &input.event_ledger_stream_id,
                "target_lane.event_ledger_stream_id",
                &target_lane.event_ledger_stream_id,
            )?;
        }
        if is_cloud_lane_record(&source_lane)
            && matches!(
                input.authority,
                ModelLaneAuthority::OperatorDecision | ModelLaneAuthority::ValidatorVerdict
            )
        {
            return Err(ModelLaneError::InvalidInput(
                "Cloud ModelLaneMessage authority must remain advisory or promotion_candidate until an approved PromotionGate writes promoted authority"
                    .into(),
            ));
        }
        let promotion = if input.authority == ModelLaneAuthority::Promoted {
            Some(
                self.resolve_surreal_promoted_message_guard(provider, input, scope)
                    .await?,
            )
        } else {
            None
        };
        let crdt_binding = self
            .resolve_surreal_crdt_message_authority(provider, input, &source_lane, scope)
            .await?;
        let crdt = crdt_binding
            .as_ref()
            .map(|(_, guard)| guard.clone());
        Ok(SurrealResolvedMessageAuthority {
            crdt_binding: crdt_binding.map(|(binding, _)| binding),
            guard: SurrealModelLaneMessageGuard {
                source_lane_id: source_lane.lane_id.clone(),
                source_lane_record_json,
                source_session_id: source_lane.session_id.clone(),
                source_model_session_id: source_lane.model_session_id.clone(),
                promotion_decision_id: promotion.as_ref().map(|(id, _)| id.clone()),
                promotion_record_json: promotion.map(|(_, record_json)| record_json),
                crdt,
            },
        })
    }

    async fn resolve_surreal_promoted_message_guard(
        &self,
        provider: &SurrealModelLaneStore,
        input: &NewModelLaneMessage,
        scope: &SurrealModelLaneScope,
    ) -> ModelLaneResult<(String, String)> {
        let required = |field: &str, value: Option<&str>| {
            require_optional_token(field, value).map_err(|_| {
                ModelLaneError::InvalidInput(format!(
                    "Promoted ModelLaneMessage requires approved PromotionGate resolution: {field} is required"
                ))
            })
        };
        let promotion_decision_id =
            required("promotion_decision_id", input.promotion_decision_id.as_deref())?;
        let promotion_gate_ref =
            required("promotion_gate_ref", input.promotion_gate_ref.as_deref())?;
        let promotion_receipt_ref = required(
            "promotion_receipt_ref",
            input.promotion_receipt_ref.as_deref(),
        )?;
        let promoted_artifact_ref = required(
            "promoted_artifact_ref",
            input.promoted_artifact_ref.as_deref(),
        )?;
        let promoted_artifact_sha256 = required(
            "promoted_artifact_sha256",
            input.promoted_artifact_sha256.as_deref(),
        )?;
        let promoted_artifact_version = required(
            "promoted_artifact_version",
            input.promoted_artifact_version.as_deref(),
        )?;
        let stored = provider
            .get(
                SurrealRecordKind::PromotionDecision,
                &promotion_decision_id,
                scope,
            )
            .await?
            .ok_or_else(|| {
                ModelLaneError::InvalidInput(format!(
                    "Promoted ModelLaneMessage requires approved PromotionGate resolution for promotion_decision_id {promotion_decision_id}"
                ))
            })?;
        let record_json = stored.record_json.clone();
        let decision = surreal_promotion_record(stored)?;
        let exact_scope = self.access.exact_read_scope().ok_or_else(|| {
            ModelLaneError::AuthorityDenied(
                "Promoted ModelLaneMessage requires complete five-field resource scope".into(),
            )
        })?;
        validate_surreal_promotion_record_authority(&decision, exact_scope)?;
        if decision.run_id != input.run_id
            || decision.outcome != ModelLanePromotionOutcome::Approved
            || decision.final_state != ModelLanePromotionState::Executed
            || decision.denial_reason.is_some()
            || decision.promotion_gate_ref != promotion_gate_ref
            || decision.promotion_receipt_ref.as_deref()
                != Some(promotion_receipt_ref.as_str())
            || decision.promoted_artifact_ref.as_deref()
                != Some(promoted_artifact_ref.as_str())
            || decision.promoted_artifact_sha256.as_deref()
                != Some(promoted_artifact_sha256.as_str())
            || decision.promoted_artifact_version.as_deref()
                != Some(promoted_artifact_version.as_str())
        {
            return Err(ModelLaneError::InvalidInput(format!(
                "Promoted ModelLaneMessage requires exact approved PromotionGate resolution and artifact binding for promotion_decision_id {promotion_decision_id}"
            )));
        }
        let bindings = provider
            .find_by_term(
                SurrealRecordKind::ContextArtifact,
                &format!("artifact_ref={promoted_artifact_ref}"),
                scope,
            )
            .await?;
        if bindings.len() != 1 || bindings[0].run_id != input.run_id {
            return Err(ModelLaneError::InvalidInput(format!(
                "Promoted ModelLaneMessage requires scoped artifact authority for promotion_decision_id {promotion_decision_id}"
            )));
        }
        let binding: NewModelLaneContextBundleArtifactBinding =
            serde_json::from_str(&bindings[0].record_json)?;
        validate_context_bundle_artifact_binding(&binding)?;
        if binding.artifact_sha256 != promoted_artifact_sha256
            || binding.content_hash != promoted_artifact_sha256
            || binding
                .payload_json
                .get("artifact_version")
                .and_then(Value::as_str)
                != Some(promoted_artifact_version.as_str())
        {
            return Err(ModelLaneError::InvalidInput(format!(
                "Promoted ModelLaneMessage requires exact scoped artifact binding for promotion_decision_id {promotion_decision_id}"
            )));
        }
        Ok((promotion_decision_id, record_json))
    }

    async fn resolve_surreal_crdt_message_authority(
        &self,
        provider: &SurrealModelLaneStore,
        message: &NewModelLaneMessage,
        lane: &ModelLaneRecord,
        scope: &SurrealModelLaneScope,
    ) -> ModelLaneResult<
        Option<(ModelLaneCrdtAuthorityBinding, SurrealModelLaneCrdtGuard)>,
    > {
        let has_any = model_lane_message_has_crdt_authority(message);
        let Some(update_ref) = message.crdt_update_ref.as_deref() else {
            if has_any {
                return Err(crdt_authority_denied(
                    "partial CRDT metadata cannot be admitted without crdt_update_ref",
                ));
            }
            return Ok(None);
        };
        let snapshot_ref = message.crdt_base_snapshot_ref.as_deref().ok_or_else(|| {
            crdt_authority_denied(format!(
                "crdt_update_ref {update_ref} requires crdt_base_snapshot_ref"
            ))
        })?;
        let state_vector = message.crdt_state_vector.as_deref().ok_or_else(|| {
            crdt_authority_denied(format!(
                "crdt_update_ref {update_ref} requires crdt_state_vector"
            ))
        })?;
        if message.crdt_stale_base_ref.is_some() {
            return Err(crdt_authority_denied(format!(
                "crdt_update_ref {update_ref} cannot be admitted with crdt_stale_base_ref"
            )));
        }
        if message.kind == ModelLaneMessageKind::Proposal
            && message.crdt_proposal_ref.is_none()
        {
            return Err(crdt_authority_denied(format!(
                "Proposal message {} carrying crdt_update_ref requires a persisted crdt_proposal_ref",
                message.message_id
            )));
        }

        let update = provider
            .crdt_update_by_ref(update_ref, scope)
            .await?
            .ok_or_else(|| {
                crdt_authority_denied(format!(
                    "crdt_update_ref {update_ref} does not resolve to scoped embedded SurrealDB authority"
                ))
            })?;
        validate_surreal_crdt_update_row(&update, scope)?;
        if update.state_vector_after != state_vector {
            return Err(crdt_authority_denied(format!(
                "crdt_state_vector {state_vector} does not match persisted state_vector_after {}",
                update.state_vector_after
            )));
        }
        let snapshot = provider
            .crdt_snapshot_by_ref(snapshot_ref, scope)
            .await?
            .ok_or_else(|| {
                crdt_authority_denied(format!(
                    "crdt_base_snapshot_ref {snapshot_ref} does not resolve to scoped embedded SurrealDB authority"
                ))
            })?;
        let snapshot_bytes = validate_surreal_crdt_snapshot_row(&snapshot, scope)?;
        if snapshot.workspace_id != update.workspace_id
            || snapshot.document_id != update.document_id
            || snapshot.crdt_document_id != update.crdt_document_id
            || snapshot.covered_update_seq >= update.update_seq
        {
            return Err(crdt_authority_denied(format!(
                "crdt_base_snapshot_ref {snapshot_ref} is not causally before update {update_ref}"
            )));
        }
        let chain = provider
            .crdt_update_chain(
                &update.document_id,
                &update.crdt_document_id,
                snapshot.covered_update_seq,
                update.update_seq,
                scope,
            )
            .await?;
        let replay_count = update
            .update_seq
            .checked_sub(snapshot.covered_update_seq)
            .and_then(|value| usize::try_from(value).ok())
            .ok_or_else(|| crdt_authority_denied("invalid CRDT replay bounds"))?;
        if chain.len() != replay_count {
            return Err(crdt_authority_denied(format!(
                "CRDT replay chain is not contiguous from snapshot seq {} through update seq {}",
                snapshot.covered_update_seq, update.update_seq
            )));
        }
        let mut seen_update_ids: BTreeSet<String> = provider
            .crdt_seen_update_ids(
                &update.document_id,
                &update.crdt_document_id,
                snapshot.covered_update_seq,
                scope,
            )
            .await?
            .into_iter()
            .collect();
        let mut derived_vector =
            KnowledgeStateVectorV1::parse(&snapshot.state_vector).map_err(|error| {
                crdt_authority_denied(format!(
                    "crdt_base_snapshot_ref {snapshot_ref} has invalid state vector: {error}"
                ))
            })?;
        let materialized = Doc::new();
        let decoded_snapshot = Update::decode_v1(&snapshot_bytes).map_err(|error| {
            crdt_authority_denied(format!(
                "cannot decode base snapshot {snapshot_ref}: {error}"
            ))
        })?;
        materialized
            .transact_mut()
            .apply_update(decoded_snapshot)
            .map_err(|error| {
                crdt_authority_denied(format!(
                    "cannot materialize base snapshot {snapshot_ref}: {error}"
                ))
            })?;
        for (offset, row) in chain.iter().enumerate() {
            let expected_seq = snapshot
                .covered_update_seq
                .checked_add(i64::try_from(offset).map_err(|_| {
                    crdt_authority_denied("CRDT replay chain offset exceeds i64")
                })?)
                .and_then(|value| value.checked_add(1))
                .ok_or_else(|| crdt_authority_denied("CRDT replay sequence overflows i64"))?;
            if row.update_seq != expected_seq
                || row.workspace_id != update.workspace_id
                || row.document_id != update.document_id
                || row.crdt_document_id != update.crdt_document_id
                || row
                    .dependency_update_ids
                    .iter()
                    .any(|dependency| !seen_update_ids.contains(dependency))
            {
                return Err(crdt_authority_denied(format!(
                    "crdt_update_ref {} has a sequence, entity, or causal dependency mismatch",
                    row.update_bytes_ref
                )));
            }
            let bytes = validate_surreal_crdt_update_row(row, scope)?;
            let actor = KnowledgeActorIdV1::parse(&row.actor_id).map_err(|error| {
                crdt_authority_denied(format!(
                    "crdt_update_ref {} actor_id is invalid: {error}",
                    row.update_bytes_ref
                ))
            })?;
            let site = derive_knowledge_site_id(&row.workspace_id, &row.crdt_document_id, &actor);
            if derived_vector.encode() != row.state_vector_before
                || site.site_id != row.ledger_site_id
            {
                return Err(crdt_authority_denied(format!(
                    "crdt_update_ref {} has stale state or wrong site attribution",
                    row.update_bytes_ref
                )));
            }
            derived_vector.increment(&site.site_id);
            if derived_vector.encode() != row.state_vector_after {
                return Err(crdt_authority_denied(format!(
                    "crdt_update_ref {} state_vector_after is not server-derived",
                    row.update_bytes_ref
                )));
            }
            let decoded = Update::decode_v1(&bytes).map_err(|error| {
                crdt_authority_denied(format!(
                    "crdt_update_ref {} does not decode as Yjs v1: {error}",
                    row.update_bytes_ref
                ))
            })?;
            let before = materialized.transact().state_vector();
            if !decoded.extends(&before) {
                return Err(crdt_authority_denied(format!(
                    "crdt_update_ref {} does not advance persisted Yjs state",
                    row.update_bytes_ref
                )));
            }
            materialized
                .transact_mut()
                .apply_update(decoded)
                .map_err(|error| {
                    crdt_authority_denied(format!(
                        "crdt_update_ref {} cannot be materialized: {error}",
                        row.update_bytes_ref
                    ))
                })?;
            seen_update_ids.insert(row.update_id.clone());
        }
        if derived_vector.encode() != update.state_vector_after
            || chain.last().map(|row| &row.update_id) != Some(&update.update_id)
        {
            return Err(crdt_authority_denied(format!(
                "crdt_update_ref {update_ref} is not the terminal authority of its replay chain"
            )));
        }
        let projection = materialized
            .transact()
            .encode_state_as_update_v1(&StateVector::default());
        let materialized_projection_hash = dexterity_sha256_hex(&projection);
        let yjs_state_vector_b64 = base64::engine::general_purpose::STANDARD
            .encode(materialized.transact().state_vector().encode_v1());
        let resolved = ResolvedModelLaneCrdtAuthority {
            workspace_id: update.workspace_id.clone(),
            document_id: update.document_id.clone(),
            crdt_document_id: update.crdt_document_id.clone(),
            update_id: update.update_id.clone(),
            update_seq: update.update_seq,
            update_sha256: update.update_sha256.clone(),
            update_bytes_ref: update.update_bytes_ref.clone(),
            actor_id: update.actor_id.clone(),
            actor_kind: update.actor_kind.clone(),
            session_id: update.session_id.clone(),
            trace_id: update.trace_id.clone(),
            state_vector_after: update.state_vector_after.clone(),
            yjs_state_vector_b64,
            replay_metadata: json!({
                "replay_order_key": update.replay_order_key,
                "dependency_update_ids": update.dependency_update_ids,
                "encoding": update.replay_encoding,
                "schema_version": update.replay_schema_version,
            }),
            snapshot_bytes_ref: snapshot.snapshot_bytes_ref.clone(),
            site_id: update.ledger_site_id.clone(),
            materialized_projection_hash,
            event_ledger_event_id: update.event_ledger_event_id.clone(),
        };
        self.finish_surreal_crdt_message_authority(
            provider, message, lane, scope, resolved, update, snapshot,
        )
        .await
        .map(Some)
    }

    async fn finish_surreal_crdt_message_authority(
        &self,
        provider: &SurrealModelLaneStore,
        message: &NewModelLaneMessage,
        lane: &ModelLaneRecord,
        scope: &SurrealModelLaneScope,
        resolved: ResolvedModelLaneCrdtAuthority,
        update: SurrealModelLaneCrdtUpdate,
        snapshot: SurrealModelLaneCrdtSnapshot,
    ) -> ModelLaneResult<(ModelLaneCrdtAuthorityBinding, SurrealModelLaneCrdtGuard)> {
        let matching_lanes = provider
            .list_kind(SurrealRecordKind::Lane, scope)
            .await?
            .into_iter()
            .map(surreal_lane_record)
            .collect::<ModelLaneResult<Vec<_>>>()?
            .into_iter()
            .filter(|candidate| {
                candidate.session_id == resolved.session_id
                    || candidate.model_session_id == resolved.session_id
            })
            .collect::<Vec<_>>();
        if matching_lanes.len() != 1
            || matching_lanes[0].lane_id != lane.lane_id
            || matching_lanes[0].run_id != lane.run_id
        {
            return Err(crdt_authority_denied(format!(
                "crdt session {} is not uniquely owned by source lane {}",
                resolved.session_id, lane.lane_id
            )));
        }

        let proposal = if let Some(proposal_ref) = message.crdt_proposal_ref.as_deref() {
            let proposal_id = proposal_ref
                .strip_prefix("crdt-proposal://")
                .filter(|value| !value.is_empty() && !value.contains('/'))
                .ok_or_else(|| {
                    crdt_authority_denied(format!(
                        "crdt_proposal_ref {proposal_ref} must use crdt-proposal://<proposal_id>"
                    ))
                })?;
            let proposal = provider
                .crdt_proposal(proposal_id, scope)
                .await?
                .ok_or_else(|| {
                    crdt_authority_denied(format!(
                        "crdt_proposal_ref {proposal_ref} does not resolve to scoped embedded SurrealDB authority"
                    ))
                })?;
            if proposal.workspace_id != resolved.workspace_id
                || proposal.owner_account_id != scope.owner_account_id
                || proposal.actor_principal_id != scope.actor_principal_id
                || proposal.authenticated_session_id != scope.authenticated_session_id
                || proposal.access_space_id != scope.access_space_id
                || proposal.document_id != resolved.document_id
                || proposal.crdt_document_id != resolved.crdt_document_id
                || proposal.actor_id != resolved.actor_id
                || proposal.actor_kind != resolved.actor_kind
                || proposal.session_id != resolved.session_id
                || proposal.correlation_id != resolved.trace_id
                || !matches!(proposal.review_state.as_str(), "approved" | "promoted")
                || proposal.applied_update_id.as_deref() != Some(resolved.update_id.as_str())
                || proposal.applied_update_sha256.as_deref()
                    != Some(proposal.diff_sha256.as_str())
            {
                return Err(crdt_authority_denied(format!(
                    "crdt_proposal_ref {proposal_ref} is not an approved applied proposal for update {}",
                    resolved.update_id
                )));
            }
            Some(proposal)
        } else {
            None
        };

        let leases = provider
            .active_crdt_leases(
                &lane.lane_id,
                &resolved.actor_id,
                &resolved.actor_kind,
                &resolved.session_id,
                &resolved.trace_id,
                &resolved.document_id,
                &resolved.crdt_document_id,
                scope,
            )
            .await?;
        if leases.len() != 1 {
            return Err(crdt_authority_denied(format!(
                "crdt actor {} session {} requires exactly one active scoped lease for source lane {}",
                resolved.actor_id, resolved.session_id, lane.lane_id
            )));
        }
        let lease = &leases[0];
        if lease.owner_account_id != scope.owner_account_id
            || lease.actor_principal_id != scope.actor_principal_id
            || lease.authenticated_session_id != scope.authenticated_session_id
            || lease.access_space_id != scope.access_space_id
            || lease.workspace_id != scope.workspace_id
            || (lease.scope_kind == "document"
                && (lease.document_id.as_deref() != Some(resolved.document_id.as_str())
                    || lease.crdt_document_id.as_deref()
                        != Some(resolved.crdt_document_id.as_str())))
            || !crdt_lease_scope_covers_resolved_authority(
                &lease.scope_kind,
                &lease.scope_id,
                &resolved,
            )
        {
            return Err(crdt_authority_denied(format!(
                "CRDT lease {} does not cover workspace/document authority",
                lease.lease_id
            )));
        }
        let resolved_lease = ResolvedModelLaneCrdtLeaseAuthority {
            lease_id: lease.lease_id.clone(),
            correlation_id: lease.correlation_id.clone(),
            scope_kind: lease.scope_kind.clone(),
            scope_id: lease.scope_id.clone(),
            claimed_at_utc: lease.claimed_at_utc.clone(),
            expires_at_utc: lease.expires_at_utc.clone(),
            admitted_at_utc: lease.admitted_at_utc.clone(),
        };
        let binding = bind_crdt_authority_to_lane(message, lane, &resolved, &resolved_lease)?;
        let guard = SurrealModelLaneCrdtGuard {
            update_ref: update.update_bytes_ref,
            update_id: update.update_id,
            update_sha256: update.update_sha256,
            state_vector: update.state_vector_after,
            snapshot_ref: snapshot.snapshot_bytes_ref,
            snapshot_id: snapshot.snapshot_id,
            snapshot_sha256: snapshot.snapshot_sha256,
            document_id: resolved.document_id,
            crdt_document_id: resolved.crdt_document_id,
            actor_id: resolved.actor_id,
            actor_kind: resolved.actor_kind,
            session_id: resolved.session_id,
            trace_id: resolved.trace_id,
            lease_id: lease.lease_id.clone(),
            lease_scope_kind: lease.scope_kind.clone(),
            lease_scope_id: lease.scope_id.clone(),
            lease_claimed_at_utc: lease.claimed_at_utc.clone(),
            lease_expires_at_utc: lease.expires_at_utc.clone(),
            lease_admitted_at_utc: lease.admitted_at_utc.clone(),
            proposal_id: proposal.as_ref().map(|value| value.proposal_id.clone()),
            proposal_diff_sha256: proposal.map(|value| value.diff_sha256),
        };
        Ok((binding, guard))
    }

    async fn validate_surreal_stored_crdt_binding(
        &self,
        message: &ModelLaneMessageRecord,
        scope: &SurrealModelLaneScope,
    ) -> ModelLaneResult<()> {
        let has_crdt = model_lane_message_has_crdt_authority(&message.inner);
        let Some(binding) = message.crdt_authority_binding.as_ref() else {
            if has_crdt {
                return Err(crdt_authority_denied(format!(
                    "stored message {} has CRDT metadata without durable authority binding",
                    message.message_id
                )));
            }
            return Ok(());
        };
        if !has_crdt || binding.workspace_id != scope.workspace_id {
            return Err(crdt_authority_denied(format!(
                "stored message {} has a CRDT binding outside its exact scoped metadata",
                message.message_id
            )));
        }
        let provider = self.provider().await?;
        let lane = provider
            .get(SurrealRecordKind::Lane, &message.from_lane_id, scope)
            .await?
            .ok_or_else(|| crdt_authority_denied("stored CRDT source lane is unavailable"))
            .and_then(surreal_lane_record)?;
        let update = provider
            .crdt_update_by_ref(&binding.update_bytes_ref, scope)
            .await?
            .ok_or_else(|| crdt_authority_denied("stored CRDT update authority is unavailable"))?;
        validate_surreal_crdt_update_row(&update, scope)?;
        let snapshot = provider
            .crdt_snapshot_by_ref(&binding.base_snapshot_ref, scope)
            .await?
            .ok_or_else(|| {
                crdt_authority_denied("stored CRDT snapshot authority is unavailable")
            })?;
        validate_surreal_crdt_snapshot_row(&snapshot, scope)?;
        let expected_actor_kind = expected_crdt_actor_kind_for_lane(&lane.kind);
        if binding.run_id != message.run_id
            || binding.lane_id != lane.lane_id
            || binding.lane_session_id != lane.session_id
            || binding.model_session_id != lane.model_session_id
            || binding.lane_trace_id != lane.trace_id
            || binding.actor_kind != expected_actor_kind
            || binding.workspace_id != update.workspace_id
            || binding.document_id != update.document_id
            || binding.crdt_document_id != update.crdt_document_id
            || binding.actor_id != update.actor_id
            || binding.actor_kind != update.actor_kind
            || binding.crdt_session_id != update.session_id
            || binding.crdt_trace_id != update.trace_id
            || binding.update_id != update.update_id
            || binding.update_seq != update.update_seq
            || binding.update_bytes_ref != update.update_bytes_ref
            || binding.state_vector != update.state_vector_after
            || binding.crdt_site_id != update.ledger_site_id
            || binding.update_event_ledger_event_id != update.event_ledger_event_id
            || binding.base_snapshot_ref != snapshot.snapshot_bytes_ref
            || snapshot.workspace_id != update.workspace_id
            || snapshot.document_id != update.document_id
            || snapshot.crdt_document_id != update.crdt_document_id
        {
            return Err(crdt_authority_denied(format!(
                "stored message {} CRDT binding does not equal embedded SurrealDB authority",
                message.message_id
            )));
        }
        let lease = provider
            .crdt_lease_history(
                &binding.lease_id,
                &binding.document_id,
                &binding.crdt_document_id,
                scope,
            )
            .await?
            .ok_or_else(|| crdt_authority_denied("stored CRDT lease authority is unavailable"))?;
        self.validate_surreal_historical_crdt_lease(binding, &lease, &update)?;
        if let Some(proposal_ref) = binding.crdt_proposal_ref.as_deref() {
            let proposal_id = proposal_ref
                .strip_prefix("crdt-proposal://")
                .ok_or_else(|| crdt_authority_denied("stored CRDT proposal ref is malformed"))?;
            let proposal = provider
                .crdt_proposal(proposal_id, scope)
                .await?
                .ok_or_else(|| crdt_authority_denied("stored CRDT proposal is unavailable"))?;
            if proposal.owner_account_id != scope.owner_account_id
                || proposal.actor_principal_id != scope.actor_principal_id
                || proposal.authenticated_session_id != scope.authenticated_session_id
                || proposal.access_space_id != scope.access_space_id
                || proposal.workspace_id != update.workspace_id
                || proposal.document_id != update.document_id
                || proposal.crdt_document_id != update.crdt_document_id
                || proposal.actor_id != update.actor_id
                || proposal.actor_kind != update.actor_kind
                || proposal.session_id != update.session_id
                || proposal.correlation_id != update.trace_id
                || proposal.applied_update_id.as_deref() != Some(update.update_id.as_str())
                || proposal.applied_update_sha256.as_deref()
                    != Some(proposal.diff_sha256.as_str())
                || !matches!(proposal.review_state.as_str(), "approved" | "promoted")
            {
                return Err(crdt_authority_denied(
                    "stored CRDT proposal no longer proves approved update identity",
                ));
            }
        }
        Ok(())
    }

    async fn validate_surreal_stored_message_authority(
        &self,
        message: &ModelLaneMessageRecord,
        scope: &SurrealModelLaneScope,
    ) -> ModelLaneResult<()> {
        if message.authority == ModelLaneAuthority::Promoted {
            self.resolve_surreal_promoted_message_guard(
                self.provider().await?,
                &message.inner,
                scope,
            )
            .await?;
        }
        self.validate_surreal_stored_crdt_binding(message, scope)
            .await
    }

    fn validate_surreal_historical_crdt_lease(
        &self,
        binding: &ModelLaneCrdtAuthorityBinding,
        lease: &SurrealModelLaneCrdtLeaseHistory,
        update: &SurrealModelLaneCrdtUpdate,
    ) -> ModelLaneResult<()> {
        let scope_covers = match lease.scope_kind.as_str() {
            "workspace" => lease.scope_id == update.workspace_id,
            "document" => lease.scope_id == update.crdt_document_id,
            _ => false,
        };
        let historically_active = binding.lease_admitted_at_utc >= lease.claimed_at_utc
            && binding.lease_admitted_at_utc < binding.lease_expires_at_utc
            && lease.expires_at_utc >= binding.lease_expires_at_utc
            && lease
                .released_at_utc
                .as_ref()
                .map_or(true, |released| released > &binding.lease_admitted_at_utc)
            && lease
                .expired_at_utc
                .as_ref()
                .map_or(true, |expired| expired > &binding.lease_admitted_at_utc);
        if lease.lease_id != binding.lease_id
            || lease.owner_account_id != update.owner_account_id
            || lease.actor_principal_id != update.actor_principal_id
            || lease.authenticated_session_id != update.authenticated_session_id
            || lease.access_space_id != update.access_space_id
            || lease.workspace_id != update.workspace_id
            || (lease.scope_kind == "document"
                && (lease.document_id.as_deref() != Some(update.document_id.as_str())
                    || lease.crdt_document_id.as_deref()
                        != Some(update.crdt_document_id.as_str())))
            || lease.lane_id != binding.lane_id
            || lease.actor_id != binding.actor_id
            || lease.actor_kind != binding.actor_kind
            || lease.session_id != binding.crdt_session_id
            || lease.correlation_id != binding.lease_correlation_id
            || lease.scope_kind != binding.lease_scope_kind
            || lease.scope_id != binding.lease_scope_id
            || lease.claimed_at_utc != binding.lease_claimed_at_utc
            || !scope_covers
            || !historically_active
        {
            return Err(crdt_authority_denied(format!(
                "stored CRDT lease {} does not prove exact historical admission authority",
                binding.lease_id
            )));
        }
        Ok(())
    }

    /// Commit a ModelLane payload binding and its message in one embedded
    /// SurrealDB transaction. Both immutable authorities are returned on retry;
    /// a partially present pair fails closed.
    pub async fn record_message_with_payload_binding(
        &self,
        message: NewModelLaneMessage,
        binding: NewModelLaneContextBundleArtifactBinding,
    ) -> ModelLaneResult<ModelLaneMessageRecord> {
        validate_message(&message)?;
        validate_context_bundle_artifact_binding(&binding)?;
        validate_message_payload_binding_pair(&message, &binding)?;
        let scope = self.surreal_write_scope("ModelLaneMessage payload binding")?;
        let provider = self.provider().await?;
        let message_term = format!("idempotency_key={}", message.idempotency_key);
        let binding_term = format!("idempotency_key={}", binding.idempotency_key);
        let existing_messages = provider
            .find_by_term(SurrealRecordKind::Message, &message_term, &scope)
            .await?;
        let existing_bindings = provider
            .find_by_term(SurrealRecordKind::ContextArtifact, &binding_term, &scope)
            .await?;
        match (existing_messages.len(), existing_bindings.len()) {
            (1, 1) => {
                let stored_message = surreal_message_record(existing_messages[0].clone())?;
                let stored_message = validate_surreal_message_retry(stored_message, &message)?;
                self.validate_surreal_stored_message_authority(&stored_message, &scope)
                    .await?;
                validate_surreal_binding_retry(&existing_bindings[0], &binding)?;
                return Ok(stored_message);
            }
            (0, 0) => {}
            _ => {
                return Err(ModelLaneError::IntegrityViolation(
                    "ModelLaneMessage payload-binding immutable pair is partial or ambiguous"
                        .into(),
                ));
            }
        }
        let authority = self
            .validate_surreal_message_authority(&message, &scope)
            .await?;
        let message_write =
            surreal_message_write(&message, authority.crdt_binding, &scope)?;
        let binding_write = surreal_binding_write(&binding, &scope)?;
        let (stored_message, stored_binding) = provider
            .put_message_immutable_guarded(
                message_write,
                Some(binding_write),
                authority.guard,
                &scope,
            )
            .await?;
        let stored_binding = stored_binding.ok_or_else(|| {
            ModelLaneError::IntegrityViolation(
                "ModelLaneMessage payload-binding transaction returned no binding".into(),
            )
        })?;
        validate_surreal_binding_retry(&stored_binding, &binding)?;
        let stored_message = surreal_message_record(stored_message)?;
        self.validate_surreal_stored_message_authority(&stored_message, &scope)
            .await?;
        Ok(stored_message)
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

    async fn prepare_promotion_decision_surreal(
        &self,
        provider: &SurrealModelLaneStore,
        scope: &SurrealModelLaneScope,
        exact_scope: &ExactResourceScopeAttribution,
        mut input: NewModelLanePromotionDecision,
    ) -> ModelLaneResult<ModelLanePromotionDecisionRecord> {
        provider
            .get(SurrealRecordKind::Run, &input.run_id, scope)
            .await?
            .ok_or_else(|| ModelLaneError::NotFound("PromotionGate run authority".into()))
            .and_then(surreal_run_record)?;
        let canonical_input_refs = canonicalize_refs("input_refs", &input.input_refs)?;
        let selected_input_refs =
            canonicalize_refs("selected_input_refs", &input.selected_input_refs)?;
        let rejected_input_refs =
            canonicalize_refs("rejected_input_refs", &input.rejected_input_refs)?;
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

        let mut records_by_ref = BTreeMap::new();
        let mut denial_reason = None;
        for reference in &canonical_input_refs {
            let message_id = message_id_from_ref("input_refs[]", reference)?;
            let record = provider
                .get(SurrealRecordKind::Message, &message_id, scope)
                .await?
                .map(surreal_message_record)
                .transpose()?;
            match record {
                Some(record)
                    if record.run_id == input.run_id
                        && matches!(
                            record.authority,
                            ModelLaneAuthority::Advisory
                                | ModelLaneAuthority::PromotionCandidate
                        ) =>
                {
                    self.validate_surreal_stored_crdt_binding(&record, scope)
                        .await?;
                    records_by_ref.insert(reference.clone(), record);
                }
                Some(_) | None => {
                    denial_reason = denial_reason
                        .or(Some(ModelLanePromotionDenialReason::InputRefMismatch));
                }
            }
        }
        let mut selected_records = Vec::new();
        for reference in &selected_input_refs {
            if let Some(record) = records_by_ref.get(reference) {
                selected_records.push(record.clone());
            } else {
                denial_reason =
                    denial_reason.or(Some(ModelLanePromotionDenialReason::InputRefMismatch));
            }
        }
        if selected_records.is_empty() {
            denial_reason =
                denial_reason.or(Some(ModelLanePromotionDenialReason::InputRefMismatch));
        }
        let mut current_base_snapshot_ref = None;
        let mut current_state_vector = None;
        for record in &selected_records {
            let Some(binding) = record.crdt_authority_binding.as_ref() else {
                continue;
            };
            if current_base_snapshot_ref
                .as_deref()
                .is_some_and(|value| value != binding.base_snapshot_ref)
                || current_state_vector
                    .as_deref()
                    .is_some_and(|value| value != binding.state_vector)
            {
                denial_reason =
                    denial_reason.or(Some(ModelLanePromotionDenialReason::InputRefMismatch));
            }
            current_base_snapshot_ref.get_or_insert(binding.base_snapshot_ref.clone());
            current_state_vector.get_or_insert(binding.state_vector.clone());
        }
        selected_records.sort_by(|left, right| {
            left.event_ledger_seq
                .cmp(&right.event_ledger_seq)
                .then_with(|| left.message_id.cmp(&right.message_id))
        });

        input.input_refs = canonical_input_refs.clone();
        input.selected_input_refs = selected_input_refs;
        input.rejected_input_refs = rejected_input_refs;
        input.current_base_snapshot_ref = current_base_snapshot_ref
            .clone()
            .unwrap_or_else(|| "not-applicable".into());
        input.current_state_vector = current_state_vector
            .clone()
            .unwrap_or_else(|| "not-applicable".into());
        let expected_record = selected_records.iter().find(|record| {
            record.message_id == input.expected_event_ledger_aggregate_id
        });
        let current_event_ledger_version = expected_record
            .map(|record| record.event_stream_version);
        let current_schema_id = (input.expected_event_ledger_aggregate_type
            == "model_lane_message")
            .then(|| "hsk.model_lane_message@1".to_string());
        let expected_aggregate_matches_selected = input.expected_event_ledger_aggregate_type
            == "model_lane_message"
            && expected_record.is_some();
        let artifact_matches = self
            .promotion_artifact_binding_matches_surreal(provider, scope, &input)
            .await?;
        let denial_reason = if let Some(reason) = denial_reason {
            Some(reason)
        } else if current_base_snapshot_ref.is_none()
            && (input.base_snapshot_ref != "not-applicable"
                || input.state_vector != "not-applicable")
        {
            Some(ModelLanePromotionDenialReason::InputRefMismatch)
        } else if !expected_aggregate_matches_selected
            || current_event_ledger_version != Some(input.expected_event_ledger_version)
        {
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
        } else if missing_promoted_artifact_binding(&input) || !artifact_matches {
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
        let canonical_decision_hash =
            dexterity_sha256_hex(serde_json::to_vec(&canonical_hash_basis)?);
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

    async fn promotion_artifact_binding_matches_surreal(
        &self,
        provider: &SurrealModelLaneStore,
        scope: &SurrealModelLaneScope,
        input: &NewModelLanePromotionDecision,
    ) -> ModelLaneResult<bool> {
        let (Some(artifact_ref), Some(artifact_sha256), Some(artifact_version)) = (
            input.promoted_artifact_ref.as_deref(),
            input.promoted_artifact_sha256.as_deref(),
            input.promoted_artifact_version.as_deref(),
        ) else {
            return Ok(false);
        };
        let rows = provider
            .find_by_term(
                SurrealRecordKind::ContextArtifact,
                &format!("artifact_ref={artifact_ref}"),
                scope,
            )
            .await?;
        if rows.len() != 1 || rows[0].run_id != input.run_id {
            return Ok(false);
        }
        let binding: NewModelLaneContextBundleArtifactBinding =
            serde_json::from_str(&rows[0].record_json)?;
        validate_context_bundle_artifact_binding(&binding)?;
        Ok(binding.artifact_sha256 == artifact_sha256
            && binding.content_hash == artifact_sha256
            && binding
                .payload_json
                .get("artifact_version")
                .and_then(Value::as_str)
                == Some(artifact_version))
    }

    pub async fn record_promotion_decision(
        &self,
        input: NewModelLanePromotionDecision,
    ) -> ModelLaneResult<ModelLanePromotionDecisionRecord> {
        let scope = self.surreal_write_scope("PromotionGate decision write")?;
        let exact_scope = self.access.exact_read_scope().ok_or_else(|| {
            ModelLaneError::AuthorityDenied(
                "PromotionGate requires complete account/principal/session/AccessSpace/workspace authority"
                    .into(),
            )
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
        let provider = self.provider().await?;
        let prepared = self
            .prepare_promotion_decision_surreal(provider, &scope, exact_scope, input)
            .await?;
        let idempotency_term = format!("idempotency_key={}", prepared.idempotency_key);
        let existing = provider
            .find_by_term(
                SurrealRecordKind::PromotionDecision,
                &idempotency_term,
                &scope,
            )
            .await?;
        if existing.len() > 1 {
            return Err(ModelLaneError::IntegrityViolation(format!(
                "PromotionGate idempotency_key {} resolves to multiple scoped records",
                prepared.idempotency_key
            )));
        }
        if let Some(existing) = existing.into_iter().next() {
            let existing = surreal_promotion_record(existing)?;
            validate_surreal_promotion_record_authority(&existing, exact_scope)?;
            if existing.canonical_decision_hash == prepared.canonical_decision_hash {
                return Ok(existing);
            }
            return Err(ModelLaneError::IdempotencyConflict(format!(
                "idempotency_key {} already belongs to canonical_decision_hash {}",
                prepared.idempotency_key, existing.canonical_decision_hash
            )));
        }
        if let Some(existing) = provider
            .get(
                SurrealRecordKind::PromotionDecision,
                &prepared.decision_id,
                &scope,
            )
            .await?
        {
            let existing = surreal_promotion_record(existing)?;
            validate_surreal_promotion_record_authority(&existing, exact_scope)?;
            if existing.canonical_decision_hash == prepared.canonical_decision_hash {
                return Ok(existing);
            }
            return Err(ModelLaneError::IdempotencyConflict(format!(
                "decision_id {} already belongs to idempotency_key {}",
                prepared.decision_id, existing.idempotency_key
            )));
        }
        let event_payload_json = scoped_surreal_event_payload(
            "hsk.model_lane_promotion_decision@1",
            &prepared,
            &scope,
        )?;
        let stored = provider
            .put_immutable(
                SurrealRecordKind::PromotionDecision,
                &prepared.decision_id,
                &prepared.run_id,
                &prepared.idempotency_key,
                serde_json::to_string(&prepared)?,
                promotion_search_terms(&prepared),
                event_payload_json,
                &scope,
            )
            .await?;
        let record = surreal_promotion_record(stored)?;
        validate_surreal_promotion_record_authority(&record, exact_scope)?;
        Ok(record)
    }

    pub async fn replay_promotion_decisions(
        &self,
        run_id: &str,
    ) -> ModelLaneResult<Vec<ModelLanePromotionDecisionRecord>> {
        require_token("run_id", run_id)?;
        let scope = self.surreal_read_scope("PromotionGate replay")?;
        let exact_scope = self.access.exact_read_scope().ok_or_else(|| {
            ModelLaneError::AuthorityDenied(
                "PromotionGate replay requires complete five-field resource scope".into(),
            )
        })?;
        let records = self
            .provider()
            .await?
            .list_run(SurrealRecordKind::PromotionDecision, run_id, &scope)
            .await?
            .into_iter()
            .map(surreal_promotion_record)
            .collect::<ModelLaneResult<Vec<_>>>()?;
        for record in &records {
            validate_surreal_promotion_record_authority(record, exact_scope)?;
        }
        Ok(records)
    }

    pub async fn record_context_bundle_artifact_binding(
        &self,
        input: NewModelLaneContextBundleArtifactBinding,
    ) -> ModelLaneResult<ModelLaneContextBundleArtifactBindingRecord> {
        validate_context_bundle_artifact_binding(&input)?;
        let scope = self.surreal_write_scope("record ContextBundle artifact binding")?;
        let provider = self.provider().await?;
        let run_row = provider
            .get(SurrealRecordKind::Run, &input.run_id, &scope)
            .await?
            .ok_or_else(|| ModelLaneError::NotFound(format!("run_id {}", input.run_id)))?;
        provider
            .validate_event_link(SurrealRecordKind::Run, &run_row, &scope)
            .await?;
        surreal_run_record(run_row)?;
        let artifact_binding_hash = context_bundle_artifact_binding_hash(&input)?;
        let prepared = ModelLaneContextBundleArtifactBindingRecord {
            inner: input,
            artifact_binding_hash,
            event_ledger_event_id: String::new(),
            event_ledger_seq: 0,
            event_stream_version: 0,
            transaction_seq: 0,
        };
        let existing = provider
            .find_by_term(
                SurrealRecordKind::ContextArtifact,
                &format!("idempotency_key={}", prepared.idempotency_key),
                &scope,
            )
            .await?;
        if existing.len() > 1 {
            return Err(ModelLaneError::AmbiguousLookup(format!(
                "ContextBundle artifact idempotency_key {}",
                prepared.idempotency_key
            )));
        }
        if let Some(existing) = existing.into_iter().next() {
            provider
                .validate_event_link(SurrealRecordKind::ContextArtifact, &existing, &scope)
                .await?;
            let existing = surreal_context_bundle_artifact_record(existing)?;
            if existing.artifact_binding_hash == prepared.artifact_binding_hash {
                return Ok(existing);
            }
            return Err(ModelLaneError::IdempotencyConflict(format!(
                "idempotency_key {} already belongs to artifact_binding_hash {}",
                prepared.idempotency_key, existing.artifact_binding_hash
            )));
        }
        let stored = provider
            .put_immutable(
                SurrealRecordKind::ContextArtifact,
                &prepared.artifact_binding_id,
                &prepared.run_id,
                &prepared.idempotency_key,
                serde_json::to_string(&prepared)?,
                context_bundle_artifact_search_terms(&prepared),
                scoped_surreal_event_payload(
                    "hsk.model_lane_context_bundle_artifact@1",
                    &prepared,
                    &scope,
                )?,
                &scope,
            )
            .await?;
        provider
            .validate_event_link(SurrealRecordKind::ContextArtifact, &stored, &scope)
            .await?;
        let stored = surreal_context_bundle_artifact_record(stored)?;
        if stored.artifact_binding_hash != prepared.artifact_binding_hash {
            return Err(ModelLaneError::IdempotencyConflict(format!(
                "artifact_binding_id {} already belongs to another immutable binding",
                prepared.artifact_binding_id
            )));
        }
        Ok(stored)
    }


    pub async fn record_context_bundle_handoff(
        &self,
        input: NewModelLaneContextBundleHandoff,
    ) -> ModelLaneResult<ModelLaneContextBundleHandoffRecord> {
        validate_context_bundle_handoff(&input)?;
        let scope = self.surreal_write_scope("record ContextBundle handoff")?;
        let provider = self.provider().await?;
        let prepared = self
            .prepare_context_bundle_handoff_surreal(provider, &scope, input)
            .await?;
        let existing = provider
            .find_by_term(
                SurrealRecordKind::ContextHandoff,
                &format!("idempotency_key={}", prepared.idempotency_key),
                &scope,
            )
            .await?;
        if existing.len() > 1 {
            return Err(ModelLaneError::AmbiguousLookup(format!(
                "ContextBundle handoff idempotency_key {}",
                prepared.idempotency_key
            )));
        }
        if let Some(existing) = existing.into_iter().next() {
            provider
                .validate_event_link(SurrealRecordKind::ContextHandoff, &existing, &scope)
                .await?;
            let existing = surreal_context_bundle_handoff_record(existing)?;
            if existing.context_bundle_hash == prepared.context_bundle_hash {
                return Ok(existing);
            }
            return Err(ModelLaneError::IdempotencyConflict(format!(
                "idempotency_key {} already belongs to context_bundle_hash {}",
                prepared.idempotency_key, existing.context_bundle_hash
            )));
        }
        let stored = provider
            .put_immutable(
                SurrealRecordKind::ContextHandoff,
                &prepared.handoff_id,
                &prepared.run_id,
                &prepared.idempotency_key,
                serde_json::to_string(&prepared)?,
                context_bundle_handoff_search_terms(&prepared),
                scoped_surreal_event_payload(
                    "hsk.model_lane_context_bundle_handoff@1",
                    &prepared,
                    &scope,
                )?,
                &scope,
            )
            .await?;
        provider
            .validate_event_link(SurrealRecordKind::ContextHandoff, &stored, &scope)
            .await?;
        let stored = surreal_context_bundle_handoff_record(stored)?;
        if stored.context_bundle_hash != prepared.context_bundle_hash {
            return Err(ModelLaneError::IdempotencyConflict(format!(
                "handoff_id {} already belongs to another immutable handoff",
                prepared.handoff_id
            )));
        }
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
        let scope = self.surreal_read_scope("consume ContextBundle for downstream")?;
        let provider = self.provider().await?;
        let run_row = provider
            .get(SurrealRecordKind::Run, run_id, &scope)
            .await?
            .ok_or_else(|| ModelLaneError::NotFound(format!("run_id {run_id}")))?;
        provider
            .validate_event_link(SurrealRecordKind::Run, &run_row, &scope)
            .await?;
        let run = surreal_run_record(run_row)?;
        require_equal("run.run_id", &run.run_id, "run_id", run_id)?;
        let lane_row = provider
            .get(SurrealRecordKind::Lane, downstream_lane_id, &scope)
            .await?
            .ok_or_else(|| ModelLaneError::NotFound(format!("lane_id {downstream_lane_id}")))
            .map_err(|err| match err {
                ModelLaneError::NotFound(message) => ModelLaneError::InvalidInput(format!(
                    "downstream_lane_id {downstream_lane_id} is not replayable: {message}"
                )),
                other => other,
            })?;
        provider
            .validate_event_link(SurrealRecordKind::Lane, &lane_row, &scope)
            .await?;
        let lane = surreal_lane_record(lane_row)?;
        require_equal("downstream.run_id", &lane.run_id, "run_id", run_id)?;
        let mut records = Vec::new();
        for stored in provider
            .list_run(SurrealRecordKind::ContextHandoff, run_id, &scope)
            .await?
        {
            provider
                .validate_event_link(SurrealRecordKind::ContextHandoff, &stored, &scope)
                .await?;
            let record = surreal_context_bundle_handoff_record(stored)?;
            if record.context_bundle_id == context_bundle_id
                && record.downstream_lane_id == downstream_lane_id
            {
                records.push(record);
            }
        }
        if records.is_empty() {
            return Err(ModelLaneError::InvalidInput(format!(
                "context_bundle_id {context_bundle_id} has no replayable handoffs for downstream_lane_id {downstream_lane_id}"
            )));
        }
        for record in &records {
            let validated = self
                .prepare_context_bundle_handoff_surreal(
                    provider,
                    &scope,
                    record.inner.clone(),
                )
                .await?;
            if validated.context_bundle_hash != record.context_bundle_hash {
                return Err(ModelLaneError::IntegrityViolation(format!(
                    "ContextBundle handoff {} hash does not match durable authority",
                    record.handoff_id
                )));
            }
        }
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
        let scope = self.surreal_read_scope("replay ContextBundle handoffs")?;
        let provider = self.provider().await?;
        let mut records = Vec::new();
        for stored in provider
            .list_run(SurrealRecordKind::ContextHandoff, run_id, &scope)
            .await?
        {
            provider
                .validate_event_link(SurrealRecordKind::ContextHandoff, &stored, &scope)
                .await?;
            let record = surreal_context_bundle_handoff_record(stored)?;
            if record.context_bundle_id == context_bundle_id {
                records.push(record);
            }
        }
        for record in &records {
            let validated = self
                .prepare_context_bundle_handoff_surreal(
                    provider,
                    &scope,
                    record.inner.clone(),
                )
                .await?;
            if validated.context_bundle_hash != record.context_bundle_hash {
                return Err(ModelLaneError::IntegrityViolation(format!(
                    "ContextBundle handoff {} hash does not match durable authority",
                    record.handoff_id
                )));
            }
        }
        Ok(records)
    }

    async fn prepare_context_bundle_handoff_surreal(
        &self,
        provider: &SurrealModelLaneStore,
        scope: &SurrealModelLaneScope,
        input: NewModelLaneContextBundleHandoff,
    ) -> ModelLaneResult<ModelLaneContextBundleHandoffRecord> {
        let run_row = provider
            .get(SurrealRecordKind::Run, &input.run_id, scope)
            .await?
            .ok_or_else(|| ModelLaneError::NotFound(format!("run_id {}", input.run_id)))?;
        provider
            .validate_event_link(SurrealRecordKind::Run, &run_row, scope)
            .await?;
        surreal_run_record(run_row)?;

        let downstream_row = provider
            .get(SurrealRecordKind::Lane, &input.downstream_lane_id, scope)
            .await?
            .ok_or_else(|| {
                ModelLaneError::NotFound(format!("lane_id {}", input.downstream_lane_id))
            })?;
        provider
            .validate_event_link(SurrealRecordKind::Lane, &downstream_row, scope)
            .await?;
        let downstream_lane = surreal_lane_record(downstream_row)?;
        require_equal(
            "handoff.run_id",
            &input.run_id,
            "downstream.run_id",
            &downstream_lane.run_id,
        )?;

        let source_row = provider
            .get(SurrealRecordKind::Message, &input.source_message_id, scope)
            .await?
            .ok_or_else(|| {
                ModelLaneError::InvalidInput(format!(
                    "source_message_id {} is not replayable",
                    input.source_message_id
                ))
            })?;
        provider
            .validate_event_link(SurrealRecordKind::Message, &source_row, scope)
            .await?;
        let source = surreal_message_record(source_row)?;
        self.validate_surreal_stored_message_authority(&source, scope)
            .await?;

        let source_lane_row = provider
            .get(SurrealRecordKind::Lane, &source.from_lane_id, scope)
            .await?
            .ok_or_else(|| ModelLaneError::NotFound(format!("lane_id {}", source.from_lane_id)))?;
        provider
            .validate_event_link(SurrealRecordKind::Lane, &source_lane_row, scope)
            .await?;
        let source_lane = surreal_lane_record(source_lane_row)?;
        for (left_name, left, right_name, right) in [
            (
                "handoff.run_id",
                input.run_id.as_str(),
                "source_lane.run_id",
                source_lane.run_id.as_str(),
            ),
            (
                "handoff.run_id",
                input.run_id.as_str(),
                "source.run_id",
                source.run_id.as_str(),
            ),
            (
                "handoff.source_lane_id",
                input.source_lane_id.as_str(),
                "source.from_lane_id",
                source.from_lane_id.as_str(),
            ),
            (
                "handoff.artifact_ref",
                input.artifact_ref.as_str(),
                "source.payload_ref",
                source.payload_ref.as_str(),
            ),
            (
                "handoff.artifact_sha256",
                input.artifact_sha256.as_str(),
                "source.payload_sha256",
                source.payload_sha256.as_str(),
            ),
            (
                "handoff.content_hash",
                input.content_hash.as_str(),
                "source.payload_sha256",
                source.payload_sha256.as_str(),
            ),
        ] {
            require_equal(left_name, left, right_name, right)?;
        }

        let artifact = self
            .context_bundle_artifact_by_ref_surreal(
                provider,
                scope,
                &input.run_id,
                &input.artifact_ref,
            )
            .await?
            .ok_or_else(|| {
                ModelLaneError::InvalidInput(format!(
                    "artifact_ref {} is not backed by exact-scope ArtifactStore/EventLedger authority",
                    input.artifact_ref
                ))
            })?;
        if context_bundle_artifact_binding_hash(&artifact.inner)? != artifact.artifact_binding_hash {
            return Err(ModelLaneError::IntegrityViolation(format!(
                "ContextBundle artifact {} hash does not match durable authority",
                artifact.artifact_binding_id
            )));
        }
        require_equal(
            "handoff.artifact_sha256",
            &input.artifact_sha256,
            "artifact_binding.artifact_sha256",
            &artifact.artifact_sha256,
        )?;
        require_equal(
            "handoff.content_hash",
            &input.content_hash,
            "artifact_binding.content_hash",
            &artifact.content_hash,
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

        self.validate_context_bundle_crdt_metadata_surreal(provider, scope, &input, &source)
            .await?;
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

    async fn validate_context_bundle_crdt_metadata_surreal(
        &self,
        provider: &SurrealModelLaneStore,
        scope: &SurrealModelLaneScope,
        input: &NewModelLaneContextBundleHandoff,
        source: &ModelLaneMessageRecord,
    ) -> ModelLaneResult<()> {
        let source_has_crdt =
            source.crdt_proposal_ref.is_some() || source.crdt_update_ref.is_some();
        if !source_has_crdt {
            if input.crdt_payload.is_some() {
                return Err(crdt_authority_denied(
                    "non-CRDT ContextBundle source message cannot acquire CRDT authority in handoff metadata",
                ));
            }
            return Ok(());
        }
        let crdt = input.crdt_payload.as_ref().ok_or_else(|| {
            ModelLaneError::InvalidInput(
                "CRDT ModelLaneMessage handoff requires crdt_payload metadata".into(),
            )
        })?;
        let binding = source.crdt_authority_binding.as_ref().ok_or_else(|| {
            crdt_authority_denied(
                "CRDT ContextBundle source message is missing its durable lane authority binding",
            )
        })?;
        let update = provider
            .crdt_update_by_ref(&crdt.update_bytes_ref, scope)
            .await?
            .ok_or_else(|| {
                crdt_authority_denied(format!(
                    "CRDT update ref {} is not exact-scope durable authority",
                    crdt.update_bytes_ref
                ))
            })?;
        validate_surreal_crdt_update_row(&update, scope)?;
        for (field, actual, expected) in [
            (
                "crdt_payload.workspace_id",
                crdt.workspace_id.as_str(),
                binding.workspace_id.as_str(),
            ),
            (
                "crdt_payload.document_id",
                crdt.document_id.as_str(),
                binding.document_id.as_str(),
            ),
            (
                "crdt_payload.actor_id",
                crdt.actor_id.as_str(),
                binding.actor_id.as_str(),
            ),
            (
                "crdt_payload.actor_kind",
                crdt.actor_kind.as_str(),
                binding.actor_kind.as_str(),
            ),
            (
                "crdt_payload.lane_id",
                crdt.lane_id.as_str(),
                binding.lane_id.as_str(),
            ),
            (
                "crdt_payload.crdt_site_id",
                crdt.crdt_site_id.as_str(),
                binding.crdt_site_id.as_str(),
            ),
            (
                "crdt_payload.update_bytes_ref",
                crdt.update_bytes_ref.as_str(),
                binding.update_bytes_ref.as_str(),
            ),
            (
                "crdt_payload.base_snapshot_ref",
                crdt.base_snapshot_ref.as_str(),
                binding.base_snapshot_ref.as_str(),
            ),
            (
                "crdt_payload.state_vector",
                crdt.state_vector.as_str(),
                binding.state_vector.as_str(),
            ),
            (
                "crdt_payload.materialized_projection_hash",
                crdt.materialized_projection_hash.as_str(),
                binding.materialized_projection_hash.as_str(),
            ),
            (
                "crdt_payload.update_sha256",
                crdt.update_sha256.as_str(),
                update.update_sha256.as_str(),
            ),
        ] {
            require_equal(field, actual, "persisted CRDT authority", expected)?;
        }
        if crdt.update_seq != binding.update_seq || crdt.update_seq != update.update_seq {
            return Err(crdt_authority_denied(format!(
                "crdt_payload.update_seq={} does not match persisted update_seq",
                crdt.update_seq
            )));
        }
        let expected_promotion_gate_ref =
            format!("promotion-gate://model-lane-message/{}", input.source_message_id);
        require_equal(
            "crdt_payload.promotion_gate_ref",
            &crdt.promotion_gate_ref,
            "source message promotion gate",
            &expected_promotion_gate_ref,
        )?;
        let expected_validation_runner_ref = format!("eventledger://{}", update.event_ledger_event_id);
        require_equal(
            "crdt_payload.validation_runner_ref",
            &crdt.validation_runner_ref,
            "persisted CRDT EventLedger evidence",
            &expected_validation_runner_ref,
        )?;
        let replay = &crdt.replay_metadata;
        if replay.get("format").and_then(Value::as_str) != Some("yjs_update_v1")
            || update.replay_encoding != "yjs-update-v1"
            || replay.get("replay_order_key").and_then(Value::as_str)
                != Some(update.replay_order_key.as_str())
            || replay.get("schema_version").and_then(Value::as_str)
                != Some(update.replay_schema_version.as_str())
        {
            return Err(crdt_authority_denied(
                "crdt_payload replay metadata disagrees with persisted Yjs authority",
            ));
        }
        let persisted_dependencies = update
            .dependency_update_ids
            .iter()
            .map(|value| Value::String(value.clone()))
            .collect::<Vec<_>>();
        if replay
            .get("dependency_update_ids")
            .and_then(Value::as_array)
            != Some(&persisted_dependencies)
        {
            return Err(crdt_authority_denied(
                "crdt_payload dependency_update_ids disagree with persisted Yjs authority",
            ));
        }
        Ok(())
    }

    async fn context_bundle_artifact_by_ref_surreal(
        &self,
        provider: &SurrealModelLaneStore,
        scope: &SurrealModelLaneScope,
        run_id: &str,
        artifact_ref: &str,
    ) -> ModelLaneResult<Option<ModelLaneContextBundleArtifactBindingRecord>> {
        let mut matches = Vec::new();
        for stored in provider
            .find_by_term(
                SurrealRecordKind::ContextArtifact,
                &format!("artifact_ref={artifact_ref}"),
                scope,
            )
            .await?
        {
            if stored.run_id != run_id {
                continue;
            }
            provider
                .validate_event_link(SurrealRecordKind::ContextArtifact, &stored, scope)
                .await?;
            let record = surreal_context_bundle_artifact_record(stored)?;
            if record.artifact_ref == artifact_ref {
                matches.push(record);
            }
        }
        if matches.len() > 1 {
            return Err(ModelLaneError::AmbiguousLookup(format!(
                "ContextBundle artifact_ref {artifact_ref} in run {run_id}"
            )));
        }
        Ok(matches.pop())
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

        let scope = self.surreal_write_scope("record ModelLane terminal status")?;
        let provider = self.provider().await?;
        let existing_row = provider
            .get(SurrealRecordKind::Lane, lane_id, &scope)
            .await?
            .ok_or_else(|| ModelLaneError::NotFound("ModelLane lifecycle authority".into()))?;
        provider
            .validate_event_link(SurrealRecordKind::Lane, &existing_row, &scope)
            .await?;
        let existing_event_stream_version = existing_row.event_stream_version;
        let existing = surreal_lane_record(existing_row)?;
        if matches!(
            existing.status,
            ModelLaneStatus::Completed | ModelLaneStatus::Failed | ModelLaneStatus::Cancelled
        ) {
            if existing.status == status {
                return Ok(existing);
            }
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
        let event_id = format!("evt-model-lane-{}", Uuid::now_v7());
        lane.last_recovery_event_ref = Some(event_id.clone());
        validate_lane(&lane)?;
        let payload = json!({
            "schema_id": "hsk.model_lane_terminal@1",
            "dexterity_kernel": "Dexterity",
            "owner_account_id": &scope.owner_account_id,
            "actor_principal_id": &scope.actor_principal_id,
            "authenticated_session_id": &scope.authenticated_session_id,
            "access_space_id": &scope.access_space_id,
            "workspace_id": &scope.workspace_id,
            "lane_id": &lane.lane_id,
            "run_id": &lane.run_id,
            "status": status.as_str(),
            "reason": reason,
            "previous_event_ledger_event_id": &existing.event_ledger_event_id,
            "previous_event_ledger_seq": existing.event_ledger_seq,
            "record": &lane,
        });
        #[cfg(feature = "surreal-test-support")]
        self.terminal_commit_test_control.before_commit().await?;
        self.access.require_lifecycle_active()?;
        let stored = provider
            .replace_if_version_with_event_id(
                SurrealRecordKind::Lane,
                lane_id,
                existing_event_stream_version,
                serde_json::to_string(&lane)?,
                lane_search_terms(&lane),
                serde_json::to_string(&payload)?,
                event_id,
                &scope,
            )
            .await?
            .ok_or_else(|| {
                ModelLaneError::IdempotencyConflict(format!(
                    "lane_id {lane_id} changed while terminal status was committing"
                ))
            })?;
        provider
            .validate_event_link(SurrealRecordKind::Lane, &stored, &scope)
            .await?;
        surreal_lane_record(stored)
    }

    /// Replay one run. This is the widest ModelLane read funnel — transcript,
    /// diagnostics, palmistry, and all eight navigation routes reach durable
    /// rows through here — so it is the primary HBR-PRIV-002 chokepoint.
    ///
    /// Exact five-field predicates are pushed into every provider query. Each
    /// returned row is additionally bound to its exact-scope EventLedger
    /// envelope before it is exposed to transcript or diagnostic callers.
    pub async fn replay_run(&self, run_id: &str) -> ModelLaneResult<ModelLaneReplay> {
        require_token("run_id", run_id)?;
        let scope = self.surreal_read_scope("ModelLane replay")?;
        let provider = self.provider().await?;
        let run_row = provider
            .get(SurrealRecordKind::Run, run_id, &scope)
            .await?
            .ok_or_else(|| ModelLaneError::NotFound(format!("run_id {run_id}")))?;
        provider
            .validate_event_link(SurrealRecordKind::Run, &run_row, &scope)
            .await?;
        let run = surreal_run_record(run_row)?;
        let mut lanes = Vec::new();
        for stored in provider
            .list_run(SurrealRecordKind::Lane, run_id, &scope)
            .await?
        {
            provider
                .validate_event_link(SurrealRecordKind::Lane, &stored, &scope)
                .await?;
            lanes.push(surreal_lane_record(stored)?);
        }
        let mut messages = Vec::new();
        for stored in provider
            .list_run(SurrealRecordKind::Message, run_id, &scope)
            .await?
        {
            provider
                .validate_event_link(SurrealRecordKind::Message, &stored, &scope)
                .await?;
            messages.push(surreal_message_record(stored)?);
        }
        for message in &messages {
            self.validate_surreal_stored_message_authority(message, &scope)
                .await?;
        }
        Ok(ModelLaneReplay {
            run,
            lanes,
            messages,
        })
    }

    /// Second enforcement layer for a multi-row read: re-authorize every row's
    /// stored scope after the SQL predicate already filtered.

    /// "The newest run **this reader owns**", not "the newest run on the node".
    /// Before scoping, this handed whoever asked the globally newest run's full
    /// diagnostics projection.
    pub async fn latest_diagnostics_projection(
        &self,
    ) -> ModelLaneResult<ModelLaneDiagnosticsProjection> {
        let scope = self.surreal_read_scope("read latest ModelLane diagnostics")?;
        let provider = self.provider().await?;
        let latest = provider
            .list_kind(SurrealRecordKind::Run, &scope)
            .await?
            .into_iter()
            .max_by_key(|row| row.event_seq)
            .ok_or_else(|| ModelLaneError::NotFound("no model lane runs recorded".into()))?;
        provider
            .validate_event_link(SurrealRecordKind::Run, &latest, &scope)
            .await?;
        self.diagnostics_projection(&latest.run_id).await
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
        let replay = self.replay_run(run_id).await?;
        let scope = self.surreal_read_scope("read ModelLane diagnostics")?;
        let provider = self.provider().await?;
        let tier_posture = self
            .validate_diagnostic_tier_posture(run_id, "HBR-INT-009")
            .await?;
        let mut mt_runtime_statuses = Vec::new();
        for stored in provider
            .list_run(SurrealRecordKind::MtRuntimeStatus, run_id, &scope)
            .await?
        {
            provider
                .validate_event_link(SurrealRecordKind::MtRuntimeStatus, &stored, &scope)
                .await?;
            mt_runtime_statuses.push(surreal_mt_runtime_status_record(stored)?);
        }
        let mut leases = Vec::new();
        for stored in provider
            .list_run(SurrealRecordKind::Lease, run_id, &scope)
            .await?
        {
            provider
                .validate_event_link(SurrealRecordKind::Lease, &stored, &scope)
                .await?;
            leases.push(surreal_lease_record(stored)?);
        }
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
        let mut routing_executions = Vec::new();
        for stored in provider
            .list_run(SurrealRecordKind::RoutingExecution, run_id, &scope)
            .await?
        {
            provider
                .validate_event_link(SurrealRecordKind::RoutingExecution, &stored, &scope)
                .await?;
            routing_executions.push(serde_json::from_str::<
                super::routing_execution::ModelLaneRoutingExecutionDiagnostics,
            >(&stored.record_json)?);
        }
        let messages_by_lane = replay.messages.iter().fold(
            BTreeMap::<String, Vec<&ModelLaneMessageRecord>>::new(),
            |mut acc, msg| {
                acc.entry(msg.from_lane_id.clone()).or_default().push(msg);
                acc
            },
        );
        let mut lane_anchors = BTreeMap::new();
        for stored in provider
            .list_run(SurrealRecordKind::Lane, run_id, &scope)
            .await?
        {
            let value: Value = serde_json::from_str(&stored.record_json)?;
            let anchor = value
                .get("model_stable_anchor")
                .or_else(|| value.get("inner").and_then(|inner| inner.get("model_stable_anchor")))
                .and_then(Value::as_str)
                .map(str::to_owned);
            lane_anchors.insert(stored.aggregate_id, anchor);
        }
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
        let scope = self.surreal_read_scope("navigate ModelLane by lane")?;
        let provider = self.provider().await?;
        let row = provider
            .get(SurrealRecordKind::Lane, lane_id, &scope)
            .await?
            .ok_or_else(|| ModelLaneError::NotFound(format!("lane_id {lane_id}")))?;
        provider
            .validate_event_link(SurrealRecordKind::Lane, &row, &scope)
            .await?;
        let lane = surreal_lane_record(row)?;
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
        let scope = self.surreal_read_scope("navigate ModelLane by message")?;
        let provider = self.provider().await?;
        let row = provider
            .get(SurrealRecordKind::Message, message_id, &scope)
            .await?
            .ok_or_else(|| ModelLaneError::NotFound(format!("message_id {message_id}")))?;
        provider
            .validate_event_link(SurrealRecordKind::Message, &row, &scope)
            .await?;
        let message = surreal_message_record(row)?;
        self.validate_surreal_stored_crdt_binding(&message, &scope)
            .await?;
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
            self.run_id_by_context_bundle_id(value).await?
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
            if let Some(run_id) = context_run.as_ref() {
                run_ids.push(run_id.clone());
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
        let run_id = self
            .run_id_by_trace_id(trace_id)
            .await?
            .ok_or_else(|| ModelLaneError::NotFound(format!("trace_id {trace_id}")))?;
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
            "lane_id" => self
                .run_id_by_scoped_aggregate(SurrealRecordKind::Lane, &lookup_ref)
                .await?
                .ok_or_else(|| ModelLaneError::NotFound(format!("lane_id {lookup_ref}")))?,
            "message_id" => self
                .run_id_by_scoped_aggregate(SurrealRecordKind::Message, &lookup_ref)
                .await?
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

    async fn run_id_by_scoped_aggregate(
        &self,
        kind: SurrealRecordKind,
        aggregate_id: &str,
    ) -> ModelLaneResult<Option<String>> {
        let scope = self.surreal_read_scope("resolve ModelLane navigation aggregate")?;
        let provider = self.provider().await?;
        let Some(row) = provider.get(kind, aggregate_id, &scope).await? else {
            return Ok(None);
        };
        provider.validate_event_link(kind, &row, &scope).await?;
        Ok(Some(row.run_id))
    }

    async fn run_id_by_scoped_fields(
        &self,
        value: &str,
        fields: &[&str],
    ) -> ModelLaneResult<Option<String>> {
        let scope = self.surreal_read_scope("resolve ModelLane navigation field")?;
        let provider = self.provider().await?;
        let mut run_ids = BTreeSet::new();
        for kind in MODEL_LANE_NAVIGATION_RECORD_KINDS {
            for row in provider.list_kind(*kind, &scope).await? {
                provider.validate_event_link(*kind, &row, &scope).await?;
                let record: Value = serde_json::from_str(&row.record_json)?;
                if json_named_value_matches(&record, fields, value) {
                    run_ids.insert(row.run_id);
                }
            }
        }
        if run_ids.len() > 1 {
            return Err(ModelLaneError::AmbiguousLookup(format!(
                "navigation value {value} resolves to multiple exact-scope runs"
            )));
        }
        Ok(run_ids.into_iter().next())
    }

    async fn run_id_by_model_session_id(&self, value: &str) -> ModelLaneResult<Option<String>> {
        self.run_id_by_scoped_fields(value, &["model_session_id"])
            .await
    }

    async fn run_id_by_session_id(&self, value: &str) -> ModelLaneResult<Option<String>> {
        self.run_id_by_scoped_fields(value, &["session_id", "coordinator_session_id"])
            .await
    }

    async fn run_id_by_work_packet_id(&self, value: &str) -> ModelLaneResult<Option<String>> {
        self.run_id_by_scoped_fields(value, &["work_packet_id", "wp_id"])
            .await
    }

    async fn run_id_by_micro_task_id(&self, value: &str) -> ModelLaneResult<Option<String>> {
        self.run_id_by_scoped_fields(value, &["micro_task_id", "mt_id"])
            .await
    }

    async fn run_id_by_task_board_id(&self, value: &str) -> ModelLaneResult<Option<String>> {
        self.run_id_by_scoped_fields(value, &["task_board_id"])
            .await
    }

    async fn run_id_by_artifact_ref(&self, value: &str) -> ModelLaneResult<Option<String>> {
        self.run_id_by_scoped_fields(
            value,
            &[
                "artifact_ref",
                "artifact_refs",
                "payload_ref",
                "payload_refs",
                "open_payload_refs",
            ],
        )
        .await
    }

    async fn run_id_by_context_bundle_id(&self, value: &str) -> ModelLaneResult<Option<String>> {
        self.run_id_by_scoped_fields(value, &["context_bundle_id"])
            .await
    }

    async fn run_id_by_memory_pack_ref(&self, value: &str) -> ModelLaneResult<Option<String>> {
        self.run_id_by_scoped_fields(value, &["memory_pack_ref", "fems_ref"])
            .await
    }

    async fn run_id_by_memory_pack_hash(&self, value: &str) -> ModelLaneResult<Option<String>> {
        self.run_id_by_scoped_fields(value, &["memory_pack_hash"])
            .await
    }

    async fn run_id_by_trace_id(&self, value: &str) -> ModelLaneResult<Option<String>> {
        self.run_id_by_scoped_fields(value, &["trace_id"])
            .await
    }

    async fn run_id_by_span_id(&self, value: &str) -> ModelLaneResult<Option<String>> {
        self.run_id_by_scoped_fields(
            value,
            &[
                "span_id",
                "run_span_id",
                "lane_span_id",
                "message_span_id",
                "handoff_span_id",
                "parent_span_id",
                "linked_span_contexts",
            ],
        )
        .await
    }

    async fn run_id_by_error_code(&self, value: &str) -> ModelLaneResult<Option<String>> {
        self.run_id_by_scoped_fields(value, &["error_code", "reason_code"])
            .await
    }

    async fn run_id_by_locus_ref(&self, value: &str) -> ModelLaneResult<Option<String>> {
        self.run_id_by_scoped_fields(value, &["locus_ref", "locus_binding_ref"])
            .await
    }

    async fn run_id_by_loom_block_id(&self, value: &str) -> ModelLaneResult<Option<String>> {
        self.run_id_by_scoped_fields(value, &["loom_block_id", "loom_ref"])
            .await
    }

    async fn run_id_by_diagnostic_payload_ref(
        &self,
        value: &str,
        keys: &[&str],
    ) -> ModelLaneResult<Option<String>> {
        self.run_id_by_scoped_fields(value, keys).await
    }

    async fn run_id_by_event_ledger_event_id(
        &self,
        value: &str,
    ) -> ModelLaneResult<Option<String>> {
        let scope = self.surreal_read_scope("resolve ModelLane EventLedger event id")?;
        let provider = self.provider().await?;
        let mut run_ids = BTreeSet::new();
        for kind in MODEL_LANE_NAVIGATION_RECORD_KINDS {
            for row in provider.list_kind(*kind, &scope).await? {
                if row.event_id == value {
                    provider.validate_event_link(*kind, &row, &scope).await?;
                    run_ids.insert(row.run_id);
                }
            }
        }
        one_navigation_run_id(value, run_ids)
    }

    async fn run_id_by_event_ledger_seq(&self, value: i64) -> ModelLaneResult<Option<String>> {
        let scope = self.surreal_read_scope("resolve ModelLane EventLedger sequence")?;
        let provider = self.provider().await?;
        let mut run_ids = BTreeSet::new();
        for kind in MODEL_LANE_NAVIGATION_RECORD_KINDS {
            for row in provider.list_kind(*kind, &scope).await? {
                if row.event_seq == value {
                    provider.validate_event_link(*kind, &row, &scope).await?;
                    run_ids.insert(row.run_id);
                }
            }
        }
        one_navigation_run_id(&value.to_string(), run_ids)
    }

    async fn validated_surreal_rows_for_run(
        &self,
        kind: SurrealRecordKind,
        run_id: &str,
    ) -> ModelLaneResult<Vec<SurrealModelLaneRecord>> {
        let scope = self.surreal_read_scope("read ModelLane navigation rows")?;
        let provider = self.provider().await?;
        let rows = provider.list_run(kind, run_id, &scope).await?;
        for row in &rows {
            provider.validate_event_link(kind, row, &scope).await?;
        }
        Ok(rows)
    }

    async fn navigation_projection_for_run(
        &self,
        route_id: &str,
        lookup_kind: &str,
        lookup_ref: &str,
        run_id: &str,
    ) -> ModelLaneResult<ModelLaneNavigationProjection> {
        let replay = self.replay_run(run_id).await?;
        let artifacts = self
            .validated_surreal_rows_for_run(SurrealRecordKind::ContextArtifact, run_id)
            .await?
            .into_iter()
            .map(surreal_context_bundle_artifact_record)
            .collect::<ModelLaneResult<Vec<_>>>()?;
        let context_handoffs = self
            .validated_surreal_rows_for_run(SurrealRecordKind::ContextHandoff, run_id)
            .await?
            .into_iter()
            .map(surreal_context_bundle_handoff_record)
            .collect::<ModelLaneResult<Vec<_>>>()?;
        let recovery_checkpoints = self
            .validated_surreal_rows_for_run(SurrealRecordKind::RecoveryCheckpoint, run_id)
            .await?
            .into_iter()
            .map(surreal_recovery_checkpoint_record)
            .collect::<ModelLaneResult<Vec<_>>>()?;
        let recovery_events = self
            .validated_surreal_rows_for_run(SurrealRecordKind::RecoveryEvent, run_id)
            .await?
            .into_iter()
            .map(surreal_recovery_event_record)
            .collect::<ModelLaneResult<Vec<_>>>()?;
        let leases = self
            .validated_surreal_rows_for_run(SurrealRecordKind::Lease, run_id)
            .await?
            .into_iter()
            .map(surreal_lease_record)
            .collect::<ModelLaneResult<Vec<_>>>()?;
        let diagnostic_tiers = self
            .validated_surreal_rows_for_run(SurrealRecordKind::DiagnosticTier, run_id)
            .await?
            .into_iter()
            .map(surreal_diagnostic_tier_record)
            .collect::<ModelLaneResult<Vec<_>>>()?;
        let mt_runtime_statuses = self
            .validated_surreal_rows_for_run(SurrealRecordKind::MtRuntimeStatus, run_id)
            .await?
            .into_iter()
            .map(surreal_mt_runtime_status_record)
            .collect::<ModelLaneResult<Vec<_>>>()?;
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
        let scope = self.surreal_read_scope("read ModelLane context artifacts by ref")?;
        let provider = self.provider().await?;
        let rows = provider
            .find_by_term(
                SurrealRecordKind::ContextArtifact,
                &format!("artifact_ref={value}"),
                &scope,
            )
            .await?;
        let mut records = Vec::with_capacity(rows.len());
        for row in rows {
            provider
                .validate_event_link(SurrealRecordKind::ContextArtifact, &row, &scope)
                .await?;
            records.push(surreal_context_bundle_artifact_record(row)?);
        }
        Ok(records)
    }

    async fn context_handoffs_by_context(
        &self,
        context_bundle_id: &str,
    ) -> ModelLaneResult<Vec<ModelLaneContextBundleHandoffRecord>> {
        let scope = self.surreal_read_scope("read ModelLane handoffs by ContextBundle")?;
        let provider = self.provider().await?;
        let rows = provider
            .find_by_term(
                SurrealRecordKind::ContextHandoff,
                &format!("context_bundle_id={context_bundle_id}"),
                &scope,
            )
            .await?;
        let mut records = Vec::with_capacity(rows.len());
        for row in rows {
            provider
                .validate_event_link(SurrealRecordKind::ContextHandoff, &row, &scope)
                .await?;
            records.push(surreal_context_bundle_handoff_record(row)?);
        }
        Ok(records)
    }

    async fn context_handoffs_by_artifact_ref(
        &self,
        value: &str,
    ) -> ModelLaneResult<Vec<ModelLaneContextBundleHandoffRecord>> {
        let scope = self.surreal_read_scope("read ModelLane handoffs by artifact ref")?;
        let provider = self.provider().await?;
        let rows = provider
            .find_by_term(
                SurrealRecordKind::ContextHandoff,
                &format!("artifact_ref={value}"),
                &scope,
            )
            .await?;
        let mut records = Vec::with_capacity(rows.len());
        for row in rows {
            provider
                .validate_event_link(SurrealRecordKind::ContextHandoff, &row, &scope)
                .await?;
            records.push(surreal_context_bundle_handoff_record(row)?);
        }
        Ok(records)
    }

    pub async fn record_recovery_checkpoint(
        &self,
        input: NewModelLaneRecoveryCheckpoint,
    ) -> ModelLaneResult<ModelLaneRecoveryCheckpointRecord> {
        validate_recovery_checkpoint(&input)?;
        let scope = self.surreal_write_scope("record ModelLane recovery checkpoint")?;
        let provider = self.provider().await?;
        let existing = provider
            .find_by_term(
                SurrealRecordKind::RecoveryCheckpoint,
                &format!("idempotency_key={}", input.idempotency_key),
                &scope,
            )
            .await?;
        if existing.len() > 1 {
            return Err(ModelLaneError::AmbiguousLookup(format!(
                "recovery checkpoint idempotency_key {}",
                input.idempotency_key
            )));
        }
        if let Some(existing) = existing.into_iter().next() {
            provider
                .validate_event_link(SurrealRecordKind::RecoveryCheckpoint, &existing, &scope)
                .await?;
            let existing = surreal_recovery_checkpoint_record(existing)?;
            ensure_idempotent_input_matches(
                "model_lane_recovery_checkpoint",
                &input.idempotency_key,
                &existing.inner,
                &input,
            )?;
            return Ok(existing);
        }
        let run_row = provider
            .get(SurrealRecordKind::Run, &input.run_id, &scope)
            .await?
            .ok_or_else(|| ModelLaneError::NotFound(format!("run_id {}", input.run_id)))?;
        provider
            .validate_event_link(SurrealRecordKind::Run, &run_row, &scope)
            .await?;
        let run = surreal_run_record(run_row)?;
        require_equal(
            "model_lane_recovery_checkpoint.event_ledger_stream_id",
            &input.event_ledger_stream_id,
            "run.event_ledger_stream_id",
            &run.event_ledger_stream_id,
        )?;
        if let Some(lane_id) = input.lane_id.as_deref() {
            let lane_row = provider
                .get(SurrealRecordKind::Lane, lane_id, &scope)
                .await?
                .ok_or_else(|| ModelLaneError::NotFound(format!("lane_id {lane_id}")))?;
            provider
                .validate_event_link(SurrealRecordKind::Lane, &lane_row, &scope)
                .await?;
            let lane = surreal_lane_record(lane_row)?;
            require_equal(
                "recovery checkpoint run_id",
                &input.run_id,
                "lane.run_id",
                &lane.run_id,
            )?;
        }
        let prepared = ModelLaneRecoveryCheckpointRecord {
            inner: input,
            event_ledger_event_id: String::new(),
            event_ledger_seq: 0,
            event_stream_version: 0,
            transaction_seq: 0,
        };
        let stored = provider
            .put_recovery_checkpoint_atomic(
                SurrealModelLaneWrite {
                    kind: SurrealRecordKind::RecoveryCheckpoint,
                    aggregate_id: prepared.checkpoint_id.clone(),
                    run_id: prepared.run_id.clone(),
                    idempotency_key: prepared.idempotency_key.clone(),
                    record_json: serde_json::to_string(&prepared)?,
                    search_terms: recovery_checkpoint_search_terms(&prepared),
                    event_payload_json: scoped_surreal_event_payload(
                        "hsk.model_lane_recovery_checkpoint@1",
                        &prepared,
                        &scope,
                    )?,
                },
                prepared.lane_id.as_deref(),
                prepared.last_event_ledger_seq,
                &scope,
            )
            .await?;
        provider
            .validate_event_link(SurrealRecordKind::RecoveryCheckpoint, &stored, &scope)
            .await?;
        surreal_recovery_checkpoint_record(stored)
    }

    pub async fn record_recovery_event(
        &self,
        mut input: NewModelLaneRecoveryEvent,
    ) -> ModelLaneResult<ModelLaneRecoveryEventRecord> {
        validate_recovery_event(&input)?;
        let scope = self.surreal_write_scope("record ModelLane recovery event")?;
        let provider = self.provider().await?;
        let existing = provider
            .find_by_term(
                SurrealRecordKind::RecoveryEvent,
                &format!("idempotency_key={}", input.idempotency_key),
                &scope,
            )
            .await?;
        if existing.len() > 1 {
            return Err(ModelLaneError::AmbiguousLookup(format!(
                "recovery event idempotency_key {}",
                input.idempotency_key
            )));
        }
        if let Some(existing) = existing.into_iter().next() {
            provider
                .validate_event_link(SurrealRecordKind::RecoveryEvent, &existing, &scope)
                .await?;
            let existing = surreal_recovery_event_record(existing)?;
            input.replay_order_seq = existing.replay_order_seq;
            ensure_idempotent_input_matches(
                "model_lane_recovery_event",
                &input.idempotency_key,
                &existing.inner,
                &input,
            )?;
            return Ok(existing);
        }
        let run_row = provider
            .get(SurrealRecordKind::Run, &input.run_id, &scope)
            .await?
            .ok_or_else(|| ModelLaneError::NotFound(format!("run_id {}", input.run_id)))?;
        provider
            .validate_event_link(SurrealRecordKind::Run, &run_row, &scope)
            .await?;
        let run = surreal_run_record(run_row)?;
        require_equal(
            "model_lane_recovery_event.event_ledger_stream_id",
            &input.event_ledger_stream_id,
            "run.event_ledger_stream_id",
            &run.event_ledger_stream_id,
        )?;
        if let Some(lane_id) = input.lane_id.as_deref() {
            let lane_row = provider
                .get(SurrealRecordKind::Lane, lane_id, &scope)
                .await?
                .ok_or_else(|| ModelLaneError::NotFound(format!("lane_id {lane_id}")))?;
            provider
                .validate_event_link(SurrealRecordKind::Lane, &lane_row, &scope)
                .await?;
            let lane = surreal_lane_record(lane_row)?;
            require_equal(
                "recovery event run_id",
                &input.run_id,
                "lane.run_id",
                &lane.run_id,
            )?;
        }
        for _ in 0..8 {
            input.replay_order_seq = provider.next_recovery_order_seq(&input.run_id, &scope).await?;
            let prepared = ModelLaneRecoveryEventRecord {
                inner: input.clone(),
                event_ledger_event_id: String::new(),
                event_ledger_seq: 0,
                event_stream_version: 0,
                transaction_seq: 0,
            };
            let stored = provider
                .put_recovery_event_atomic(
                    SurrealModelLaneWrite {
                        kind: SurrealRecordKind::RecoveryEvent,
                        aggregate_id: prepared.recovery_event_id.clone(),
                        run_id: prepared.run_id.clone(),
                        idempotency_key: prepared.idempotency_key.clone(),
                        record_json: serde_json::to_string(&prepared)?,
                        search_terms: recovery_event_search_terms(&prepared),
                        event_payload_json: scoped_surreal_event_payload(
                            "hsk.model_lane_recovery_event@2",
                            &prepared,
                            &scope,
                        )?,
                    },
                    prepared.lane_id.as_deref(),
                    prepared.source_event_ledger_seq,
                    prepared.replay_order_seq,
                    &scope,
                )
                .await?;
            if let Some(stored) = stored {
                provider
                    .validate_event_link(SurrealRecordKind::RecoveryEvent, &stored, &scope)
                    .await?;
                return surreal_recovery_event_record(stored);
            }
        }
        Err(ModelLaneError::IntegrityViolation(format!(
            "recovery event {} could not acquire the next scoped replay order",
            input.recovery_event_id
        )))
    }


    pub async fn record_lane_lease(
        &self,
        input: NewModelLaneLease,
    ) -> ModelLaneResult<ModelLaneLeaseRecord> {
        validate_lane_lease(&input)?;
        let scope = self.surreal_write_scope("record ModelLane lease")?;
        let provider = self.provider().await?;
        let run_row = provider
            .get(SurrealRecordKind::Run, &input.run_id, &scope)
            .await?
            .ok_or_else(|| ModelLaneError::NotFound(format!("run_id {}", input.run_id)))?;
        provider
            .validate_event_link(SurrealRecordKind::Run, &run_row, &scope)
            .await?;
        let run = surreal_run_record(run_row)?;
        require_equal(
            "model_lane_lease.event_ledger_stream_id",
            &input.event_ledger_stream_id,
            "run.event_ledger_stream_id",
            &run.event_ledger_stream_id,
        )?;
        if let Some(lane_id) = input.lane_id.as_deref() {
            let lane_row = provider
                .get(SurrealRecordKind::Lane, lane_id, &scope)
                .await?
                .ok_or_else(|| ModelLaneError::NotFound(format!("lane_id {lane_id}")))?;
            provider
                .validate_event_link(SurrealRecordKind::Lane, &lane_row, &scope)
                .await?;
            let lane = surreal_lane_record(lane_row)?;
            require_equal("lease.run_id", &input.run_id, "lane.run_id", &lane.run_id)?;
        }
        let existing = provider
            .find_by_term(
                SurrealRecordKind::Lease,
                &format!("idempotency_key={}", input.idempotency_key),
                &scope,
            )
            .await?;
        if existing.len() > 1 {
            return Err(ModelLaneError::AmbiguousLookup(format!(
                "lease idempotency_key {}",
                input.idempotency_key
            )));
        }
        if let Some(existing) = existing.into_iter().next() {
            provider
                .validate_event_link(SurrealRecordKind::Lease, &existing, &scope)
                .await?;
            let existing = surreal_lease_record(existing)?;
            ensure_idempotent_input_matches(
                "model_lane_lease",
                &input.idempotency_key,
                &existing.inner,
                &input,
            )?;
            return Ok(existing);
        }
        let prepared = ModelLaneLeaseRecord {
            inner: input,
            event_ledger_event_id: String::new(),
            event_ledger_seq: 0,
            event_stream_version: 0,
            transaction_seq: 0,
        };
        let stored = provider
            .put_immutable(
                SurrealRecordKind::Lease,
                &prepared.lease_id,
                &prepared.run_id,
                &prepared.idempotency_key,
                serde_json::to_string(&prepared)?,
                lease_search_terms(&prepared),
                scoped_surreal_event_payload(
                    "hsk.model_lane_lease@1",
                    &prepared,
                    &scope,
                )?,
                &scope,
            )
            .await?;
        provider
            .validate_event_link(SurrealRecordKind::Lease, &stored, &scope)
            .await?;
        surreal_lease_record(stored)
    }

    pub async fn record_diagnostic_tier_status(
        &self,
        input: NewModelLaneDiagnosticTierStatus,
    ) -> ModelLaneResult<ModelLaneDiagnosticTierStatusRecord> {
        validate_diagnostic_tier_status(&input)?;
        let scope = self.surreal_write_scope("record ModelLane diagnostic tier")?;
        let provider = self.provider().await?;
        let run_row = provider
            .get(SurrealRecordKind::Run, &input.run_id, &scope)
            .await?
            .ok_or_else(|| ModelLaneError::NotFound(format!("run_id {}", input.run_id)))?;
        provider
            .validate_event_link(SurrealRecordKind::Run, &run_row, &scope)
            .await?;
        let run = surreal_run_record(run_row)?;
        require_equal(
            "diagnostic_tier.event_ledger_stream_id",
            &input.event_ledger_stream_id,
            "run.event_ledger_stream_id",
            &run.event_ledger_stream_id,
        )?;
        let existing = provider
            .find_by_term(
                SurrealRecordKind::DiagnosticTier,
                &format!("idempotency_key={}", input.idempotency_key),
                &scope,
            )
            .await?;
        if existing.len() > 1 {
            return Err(ModelLaneError::AmbiguousLookup(format!(
                "diagnostic tier idempotency_key {}",
                input.idempotency_key
            )));
        }
        if let Some(existing) = existing.into_iter().next() {
            provider
                .validate_event_link(SurrealRecordKind::DiagnosticTier, &existing, &scope)
                .await?;
            let existing = surreal_diagnostic_tier_record(existing)?;
            ensure_idempotent_input_matches(
                "model_lane_diagnostic_tier",
                &input.idempotency_key,
                &existing.inner,
                &input,
            )?;
            return Ok(existing);
        }
        let prepared = ModelLaneDiagnosticTierStatusRecord {
            inner: input,
            event_ledger_event_id: String::new(),
            event_ledger_seq: 0,
            event_stream_version: 0,
            transaction_seq: 0,
        };
        let stored = provider
            .put_immutable(
                SurrealRecordKind::DiagnosticTier,
                &prepared.diagnostic_status_id,
                &prepared.run_id,
                &prepared.idempotency_key,
                serde_json::to_string(&prepared)?,
                diagnostic_tier_search_terms(&prepared),
                scoped_surreal_event_payload(
                    "hsk.model_lane_diagnostic_tier@1",
                    &prepared,
                    &scope,
                )?,
                &scope,
            )
            .await?;
        provider
            .validate_event_link(SurrealRecordKind::DiagnosticTier, &stored, &scope)
            .await?;
        surreal_diagnostic_tier_record(stored)
    }

    pub async fn diagnostic_tier_posture(
        &self,
        run_id: &str,
        behavior_id: &str,
    ) -> ModelLaneResult<ModelLaneDiagnosticTierPosture> {
        require_token("run_id", run_id)?;
        require_token("behavior_id", behavior_id)?;
        let scope = self.surreal_read_scope("read ModelLane diagnostic tier posture")?;
        let provider = self.provider().await?;
        let mut latest_by_tier = BTreeMap::new();
        for stored in provider
            .list_run(SurrealRecordKind::DiagnosticTier, run_id, &scope)
            .await?
        {
            provider
                .validate_event_link(SurrealRecordKind::DiagnosticTier, &stored, &scope)
                .await?;
            let record = surreal_diagnostic_tier_record(stored)?;
            if record.behavior_id == behavior_id {
                latest_by_tier.insert(record.tier.as_str().to_owned(), record);
            }
        }
        Ok(ModelLaneDiagnosticTierPosture {
            run_id: run_id.to_string(),
            behavior_id: behavior_id.to_string(),
            tiers: latest_by_tier.into_values().collect(),
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
        let scope = self.surreal_write_scope("record ModelLane MT runtime status")?;
        let provider = self.provider().await?;
        let run_row = provider
            .get(SurrealRecordKind::Run, &input.run_id, &scope)
            .await?
            .ok_or_else(|| ModelLaneError::NotFound(format!("run_id {}", input.run_id)))?;
        provider
            .validate_event_link(SurrealRecordKind::Run, &run_row, &scope)
            .await?;
        let run = surreal_run_record(run_row)?;
        require_equal(
            "model_lane_mt_runtime_status.event_ledger_stream_id",
            &input.event_ledger_stream_id,
            "run.event_ledger_stream_id",
            &run.event_ledger_stream_id,
        )?;
        let existing = provider
            .find_by_term(
                SurrealRecordKind::MtRuntimeStatus,
                &format!("idempotency_key={}", input.idempotency_key),
                &scope,
            )
            .await?;
        if existing.len() > 1 {
            return Err(ModelLaneError::AmbiguousLookup(format!(
                "MT runtime status idempotency_key {}",
                input.idempotency_key
            )));
        }
        if let Some(existing) = existing.into_iter().next() {
            provider
                .validate_event_link(SurrealRecordKind::MtRuntimeStatus, &existing, &scope)
                .await?;
            let existing = surreal_mt_runtime_status_record(existing)?;
            ensure_idempotent_input_matches(
                "model_lane_mt_runtime_status",
                &input.idempotency_key,
                &existing.inner,
                &input,
            )?;
            return Ok(existing);
        }
        let prepared = ModelLaneMtRuntimeStatusRecord {
            inner: input,
            event_ledger_event_id: String::new(),
            event_ledger_seq: 0,
            event_stream_version: 0,
            transaction_seq: 0,
        };
        let stored = provider
            .put_immutable(
                SurrealRecordKind::MtRuntimeStatus,
                &prepared.mt_status_id,
                &prepared.run_id,
                &prepared.idempotency_key,
                serde_json::to_string(&prepared)?,
                mt_runtime_status_search_terms(&prepared),
                scoped_surreal_event_payload(
                    "hsk.model_lane_mt_runtime_status@1",
                    &prepared,
                    &scope,
                )?,
                &scope,
            )
            .await?;
        provider
            .validate_event_link(SurrealRecordKind::MtRuntimeStatus, &stored, &scope)
            .await?;
        surreal_mt_runtime_status_record(stored)
    }

    /// Recover latest restartable/reclaimable checkpoints inside the store's
    /// exact five-field resource scope. Process-level cross-scope discovery is
    /// owned by the capability-gated ProcessLedger stale-session source.
    pub async fn recover_restartable_runs_at_boot(
        &self,
    ) -> ModelLaneResult<Vec<ModelLaneRecoveredRun>> {
        let scope = self.surreal_read_scope("recover restartable ModelLane runs at boot")?;
        let provider = self.provider().await?;
        let rows = provider
            .list_kind(SurrealRecordKind::RecoveryCheckpoint, &scope)
            .await?;
        let mut latest = BTreeMap::<String, ModelLaneRecoveryCheckpointRecord>::new();
        for row in rows {
            provider
                .validate_event_link(SurrealRecordKind::RecoveryCheckpoint, &row, &scope)
                .await?;
            let checkpoint = surreal_recovery_checkpoint_record(row)?;
            let replace = latest
                .get(&checkpoint.run_id)
                .map_or(true, |current| {
                    current.event_ledger_seq < checkpoint.event_ledger_seq
                });
            if replace {
                latest.insert(checkpoint.run_id.clone(), checkpoint);
            }
        }
        let run_ids: Vec<String> = latest
            .into_values()
            .filter(|checkpoint| {
                checkpoint.recovery_state == ModelLaneRecoveryState::Restartable
                    || checkpoint.recovery_state == ModelLaneRecoveryState::Reclaimable
            })
            .map(|checkpoint| checkpoint.run_id.clone())
            .collect();
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
        self.recover_run_after_restart_fenced(run_id, Utc::now())
            .await
    }

    #[cfg(feature = "surreal-test-support")]
    pub async fn test_recover_run_after_restart_at(
        &self,
        run_id: &str,
        now: DateTime<Utc>,
    ) -> ModelLaneResult<ModelLaneRecoveredRun> {
        require_token("run_id", run_id)?;
        self.recover_run_after_restart_fenced(run_id, now).await
    }

    async fn recover_run_after_restart_fenced(
        &self,
        run_id: &str,
        now: DateTime<Utc>,
    ) -> ModelLaneResult<ModelLaneRecoveredRun> {
        let scope = self.surreal_read_scope("recover ModelLane run after restart")?;
        let provider = self.provider().await?;
        let run_row = provider
            .get(SurrealRecordKind::Run, run_id, &scope)
            .await?
            .ok_or_else(|| ModelLaneError::NotFound(format!("run_id {run_id}")))?;
        provider
            .validate_event_link(SurrealRecordKind::Run, &run_row, &scope)
            .await?;
        let canonical_run = surreal_run_record(run_row)?;
        let checkpoint_rows = provider
            .list_run(SurrealRecordKind::RecoveryCheckpoint, run_id, &scope)
            .await?;
        let mut checkpoints = Vec::with_capacity(checkpoint_rows.len());
        for row in checkpoint_rows {
            provider
                .validate_event_link(SurrealRecordKind::RecoveryCheckpoint, &row, &scope)
                .await?;
            checkpoints.push(surreal_recovery_checkpoint_record(row)?);
        }
        let checkpoint = checkpoints
            .into_iter()
            .filter(|checkpoint| {
                checkpoint.event_ledger_stream_id == canonical_run.event_ledger_stream_id
            })
            .max_by_key(|checkpoint| checkpoint.event_ledger_seq)
            .ok_or_else(|| {
                ModelLaneError::NotFound(format!("recovery checkpoint for run_id {run_id}"))
            })?;
        require_equal(
            "recovery_checkpoint.event_ledger_stream_id",
            &checkpoint.event_ledger_stream_id,
            "canonical_run.event_ledger_stream_id",
            &canonical_run.event_ledger_stream_id,
        )?;
        let checkpoint_bound_event_ledger_seq = checkpoint.last_event_ledger_seq;
        if !provider
            .run_contains_event_sequence(run_id, checkpoint_bound_event_ledger_seq, &scope)
            .await?
        {
            return Err(ModelLaneError::IntegrityViolation(format!(
                "checkpoint {} references an event outside its exact scoped run stream",
                checkpoint.checkpoint_id
            )));
        }
        let replay = self.replay_run(run_id).await?;
        for message in &replay.messages {
            self.validate_surreal_stored_crdt_binding(message, &scope)
                .await?;
        }
        let high_watermark = provider
            .run_event_high_watermark(run_id, &scope)
            .await?
            .ok_or_else(|| {
                ModelLaneError::IntegrityViolation(format!(
                    "run {run_id} has no exact-scope EventLedger high watermark"
                ))
            })?;
        let forward_bound_event_ledger_seq = if replay
            .messages
            .iter()
            .any(|message| message.event_ledger_seq > checkpoint_bound_event_ledger_seq)
        {
            high_watermark
        } else {
            checkpoint_bound_event_ledger_seq
        };
        let recovery_rows = provider
            .list_run(SurrealRecordKind::RecoveryEvent, run_id, &scope)
            .await?;
        let mut recovery_events = Vec::with_capacity(recovery_rows.len());
        for row in recovery_rows {
            provider
                .validate_event_link(SurrealRecordKind::RecoveryEvent, &row, &scope)
                .await?;
            let event = surreal_recovery_event_record(row)?;
            require_equal(
                "recovery_event.event_ledger_stream_id",
                &event.event_ledger_stream_id,
                "checkpoint.event_ledger_stream_id",
                &checkpoint.event_ledger_stream_id,
            )?;
            if let Some(source_seq) = event.source_event_ledger_seq {
                if !provider
                    .run_contains_event_sequence(run_id, source_seq, &scope)
                    .await?
                {
                    return Err(ModelLaneError::IntegrityViolation(format!(
                        "recovery event {} references an event outside its exact scoped run stream",
                        event.recovery_event_id
                    )));
                }
            }
            if event.event_ledger_seq <= forward_bound_event_ledger_seq
                || event.event_kind == ModelLaneRecoveryEventKind::OrphanDetected
            {
                recovery_events.push(event);
            }
        }
        validate_contiguous_recovery_order(run_id, &recovery_events)?;
        validate_recovery_payload_refs_surreal(
            provider,
            &scope,
            &checkpoint,
            &replay.messages,
        )
        .await?;
        validate_recovery_crdt_posture_surreal(
            provider,
            &scope,
            &recovery_events,
            &replay.messages,
        )
        .await?;
        let lease_rows = provider
            .list_run(SurrealRecordKind::Lease, run_id, &scope)
            .await?;
        let mut leases = Vec::with_capacity(lease_rows.len());
        for row in lease_rows {
            provider
                .validate_event_link(SurrealRecordKind::Lease, &row, &scope)
                .await?;
            leases.push(surreal_lease_record(row)?);
        }
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
                    let row = provider
                        .get(SurrealRecordKind::Lane, lane_id, &scope)
                        .await?
                        .ok_or_else(|| ModelLaneError::NotFound(format!("lane_id {lane_id}")))?;
                    provider
                        .validate_event_link(SurrealRecordKind::Lane, &row, &scope)
                        .await?;
                    let lane = surreal_lane_record(row)?;
                    require_equal("recovery lease run_id", run_id, "lane.run_id", &lane.run_id)?;
                    Some(lane)
                } else {
                    None
                };
                if !recovery_events.iter().any(|event| {
                    event.event_kind == ModelLaneRecoveryEventKind::OrphanDetected
                        && event.lease_id.as_deref() == Some(lease.lease_id.as_str())
                }) {
                    let orphan_event = self
                        .record_orphan_recovery_event(
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
        let denial_rows = provider
            .list_run(SurrealRecordKind::CloudConsentDenial, run_id, &scope)
            .await?;
        let mut cloud_consent_denials = Vec::new();
        for row in denial_rows {
            provider
                .validate_event_link(SurrealRecordKind::CloudConsentDenial, &row, &scope)
                .await?;
            if row.event_seq <= checkpoint_bound_event_ledger_seq {
                cloud_consent_denials.push(serde_json::from_str(&row.record_json)?);
            }
        }
        let mt_rows = provider
            .list_run(SurrealRecordKind::MtRuntimeStatus, run_id, &scope)
            .await?;
        let mut mt_runtime_statuses = Vec::new();
        for row in mt_rows {
            provider
                .validate_event_link(SurrealRecordKind::MtRuntimeStatus, &row, &scope)
                .await?;
            if row.event_seq <= forward_bound_event_ledger_seq {
                mt_runtime_statuses.push(surreal_mt_runtime_status_record(row)?);
            }
        }
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

    async fn record_orphan_recovery_event(
        &self,
        checkpoint: &ModelLaneRecoveryCheckpointRecord,
        lease: &ModelLaneLeaseRecord,
        authoritative_lane: Option<&ModelLaneRecord>,
    ) -> ModelLaneResult<ModelLaneRecoveryEventRecord> {
        self.record_recovery_event(NewModelLaneRecoveryEvent {
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
        let scope = self.surreal_read_scope("read ModelLane schema registry")?;
        Ok(self
            .provider()
            .await?
            .schema_registry(&scope)
            .await?
            .into_iter()
            .map(|row| ModelLaneSchemaRegistryRow {
                schema_id: row.schema_id,
                schema_version: i32::try_from(row.schema_version).unwrap_or(i32::MAX),
                record_kind: row.record_kind,
                table_name: row.table_name,
            })
            .collect())
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

fn merge_run_for_lane(
    existing: &ModelLaneRunRecord,
    input: NewModelLaneRun,
    lane: &NewModelLane,
) -> ModelLaneResult<Option<NewModelLaneRun>> {
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
    if existing.lane_ids.iter().any(|lane_id| lane_id == &lane.lane_id) {
        return Ok(None);
    }
    let mut merged = existing.inner.clone();
    let mut lane_ids: BTreeSet<String> = merged.lane_ids.into_iter().collect();
    lane_ids.extend(input.lane_ids);
    lane_ids.insert(lane.lane_id.clone());
    merged.lane_ids = lane_ids.into_iter().collect();
    let mut candidate_model_ids: BTreeSet<String> =
        merged.candidate_model_ids.into_iter().collect();
    candidate_model_ids.extend(input.candidate_model_ids);
    if let Some(model_id) = input.selected_model_id {
        candidate_model_ids.insert(model_id);
    }
    if let Some(model_id) = lane.model_id.as_ref() {
        candidate_model_ids.insert(model_id.clone());
    }
    merged.candidate_model_ids = candidate_model_ids.into_iter().collect();
    merged.projection_plan_ref = merged.projection_plan_ref.or(input.projection_plan_ref);
    merged.consent_receipt_ref = merged.consent_receipt_ref.or(input.consent_receipt_ref);
    Ok(Some(merged))
}

fn scoped_surreal_event_payload<T: Serialize>(
    schema_id: &str,
    record: &T,
    scope: &SurrealModelLaneScope,
) -> ModelLaneResult<String> {
    Ok(serde_json::to_string(&json!({
        "schema_id": schema_id,
        "dexterity_kernel": "Dexterity",
        "owner_account_id": scope.owner_account_id,
        "actor_principal_id": scope.actor_principal_id,
        "authenticated_session_id": scope.authenticated_session_id,
        "access_space_id": scope.access_space_id,
        "workspace_id": scope.workspace_id,
        "record": record,
    }))?)
}

fn surreal_run_record(stored: SurrealModelLaneRecord) -> ModelLaneResult<ModelLaneRunRecord> {
    Ok(ModelLaneRunRecord {
        inner: serde_json::from_str(&stored.record_json)?,
        event_ledger_event_id: stored.event_id,
        event_ledger_seq: stored.event_seq,
    })
}

fn surreal_lane_record(stored: SurrealModelLaneRecord) -> ModelLaneResult<ModelLaneRecord> {
    Ok(ModelLaneRecord {
        inner: serde_json::from_str(&stored.record_json)?,
        event_ledger_event_id: stored.event_id,
        event_ledger_seq: stored.event_seq,
    })
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct SurrealStoredModelLaneMessage {
    inner: NewModelLaneMessage,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    crdt_authority_binding: Option<ModelLaneCrdtAuthorityBinding>,
}

fn validate_routing_commit_projection(
    expected_revision: i64,
    next_execution: &super::routing_execution::ModelLaneRoutingExecutionState,
    changed_attempt: &super::routing_execution::ModelLaneRoutingStageState,
    expected_claim: Option<&super::routing_execution::ModelLaneRoutingStageClaim>,
) -> ModelLaneResult<()> {
    let next_revision = i64::try_from(next_execution.revision).map_err(|_| {
        ModelLaneError::InvalidInput("routing next revision exceeds durable range".into())
    })?;
    let projected = next_execution
        .stages
        .get(&changed_attempt.stage_id)
        .ok_or_else(|| {
            ModelLaneError::InvalidInput(
                "routing changed attempt is absent from next execution".into(),
            )
        })?;
    if next_revision != expected_revision + 1
        || projected != changed_attempt
        || changed_attempt.expected_run_id != next_execution.run_id
        || changed_attempt.attempt == 0
    {
        return Err(ModelLaneError::InvalidInput(
            "routing commit does not bind revision, run, and changed attempt".into(),
        ));
    }
    if let Some(claim) = expected_claim {
        if claim.execution_id != next_execution.execution_id
            || claim.stage_id != changed_attempt.stage_id
            || claim.attempt != changed_attempt.attempt
            || claim.lease_owner.trim().is_empty()
            || claim.fencing_token.trim().is_empty()
        {
            return Err(ModelLaneError::AuthorityDenied(
                "routing expected claim does not bind the changed attempt".into(),
            ));
        }
    }
    Ok(())
}

fn routing_claim_to_surreal(
    claim: &super::routing_execution::ModelLaneRoutingStageClaim,
) -> SurrealModelLaneRoutingClaim {
    SurrealModelLaneRoutingClaim {
        stage_id: claim.stage_id.clone(),
        attempt: i64::from(claim.attempt),
        lease_owner: claim.lease_owner.clone(),
        fencing_token: claim.fencing_token.clone(),
        observed_at_unix_ms: Utc::now().timestamp_millis().max(1),
    }
}

fn routing_events_to_surreal(
    events: Vec<NewKernelEvent>,
) -> ModelLaneResult<Vec<SurrealModelLaneRoutingEventWrite>> {
    events
        .into_iter()
        .map(|event| {
            event.validate().map_err(|error| {
                ModelLaneError::InvalidInput(format!("invalid routing event: {error}"))
            })?;
            let canonical_payload_hash = super::routing_execution::canonical_sha256(&event.payload)
                .map_err(|error| {
                    ModelLaneError::InvalidInput(format!(
                        "routing event payload cannot be canonicalized: {error}"
                    ))
                })?;
            if event.payload_hash != canonical_payload_hash {
                return Err(ModelLaneError::InvalidInput(
                    "routing event payload_hash does not match its canonical payload".into(),
                ));
            }
            let event = KernelEvent::from_new(event);
            Ok(SurrealModelLaneRoutingEventWrite {
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
                created_at: event.created_at,
            })
        })
        .collect()
}

fn routing_execution_context_hash(
    execution: &super::routing_execution::ModelLaneRoutingExecutionState,
) -> ModelLaneResult<String> {
    let immutable_context = serde_json::json!({
        "schema_id": &execution.schema_id,
        "execution_id": &execution.execution_id,
        "run_id": &execution.run_id,
        "selecting_decision_id": &execution.selecting_decision_id,
        "selecting_decision_event_id": &execution.selecting_decision_event_id,
        "selecting_decision_event_seq": execution.selecting_decision_event_seq,
        "trace_id": &execution.trace_id,
        "run_span_id": &execution.run_span_id,
        "coordinator_session_id": &execution.coordinator_session_id,
        "locus_ref": &execution.locus_ref,
        "work_packet_id": &execution.work_packet_id,
        "micro_task_id": &execution.micro_task_id,
        "task_board_id": &execution.task_board_id,
        "owner_session": &execution.owner_session,
        "canonical_graph": &execution.canonical_graph,
        "canonical_graph_sha256": &execution.canonical_graph_sha256,
        "canonical_launch_plan": &execution.canonical_launch_plan,
        "canonical_launch_plan_sha256": &execution.canonical_launch_plan_sha256,
        "authority": &execution.authority,
        "initial_input_ref": &execution.initial_input_ref,
        "initial_input_sha256": &execution.initial_input_sha256,
    });
    super::routing_execution::canonical_sha256(&immutable_context).map_err(|error| {
        ModelLaneError::InvalidInput(format!(
            "routing immutable context cannot be canonicalized: {error}"
        ))
    })
}

fn routing_execution_projection_json(
    execution: &super::routing_execution::ModelLaneRoutingExecutionState,
) -> ModelLaneResult<Value> {
    let mut value = serde_json::to_value(execution)?;
    let object = value.as_object_mut().ok_or_else(|| {
        ModelLaneError::InvalidInput("routing execution projection must be an object".into())
    })?;
    object.insert("event_ledger_event_id".into(), Value::String(String::new()));
    object.insert("event_ledger_seq".into(), Value::from(0));
    if let Some(stages) = object.get_mut("stages").and_then(Value::as_object_mut) {
        for stage in stages.values_mut() {
            if let Some(stage) = stage.as_object_mut() {
                stage.insert("event_ledger_event_id".into(), Value::String(String::new()));
                stage.insert("event_ledger_seq".into(), Value::from(0));
            }
        }
    }
    Ok(value)
}

fn routing_attempt_projection_json(
    attempt: &super::routing_execution::ModelLaneRoutingStageState,
) -> ModelLaneResult<Value> {
    let mut value = serde_json::to_value(attempt)?;
    let object = value.as_object_mut().ok_or_else(|| {
        ModelLaneError::InvalidInput("routing attempt projection must be an object".into())
    })?;
    object.insert("event_ledger_event_id".into(), Value::String(String::new()));
    object.insert("event_ledger_seq".into(), Value::from(0));
    Ok(value)
}

fn routing_stage_state_name(
    state: super::routing_execution::ModelLaneRoutingStageStateKind,
) -> ModelLaneResult<String> {
    serde_json::to_value(state)?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| {
            ModelLaneError::IntegrityViolation(
                "routing stage state did not serialize as a stable token".into(),
            )
        })
}

fn routing_execution_from_surreal(
    row: SurrealModelLaneRoutingExecutionRow,
    attempts: Vec<crate::storage::surreal::SurrealModelLaneRoutingAttemptRow>,
    outbox: &[SurrealModelLaneRoutingOutboxRow],
) -> ModelLaneResult<super::routing_execution::ModelLaneRoutingExecutionState> {
    if attempts.len() > 4096 || outbox.len() > 4096 {
        return Err(ModelLaneError::IntegrityViolation(
            "routing execution exceeded the bounded attempt/outbox row cap".into(),
        ));
    }
    let mut execution: super::routing_execution::ModelLaneRoutingExecutionState =
        serde_json::from_value(row.record_json.clone())?;
    let durable_revision = u64::try_from(row.revision).map_err(|_| {
        ModelLaneError::IntegrityViolation("routing execution has a negative revision".into())
    })?;
    let durable_context_hash = routing_execution_context_hash(&execution)?;
    if execution.execution_id != row.execution_id
        || execution.run_id != row.run_id
        || execution.revision != durable_revision
        || row.context_hash != durable_context_hash
        || row.event_ledger_event_id.trim().is_empty()
        || row.event_ledger_seq <= 0
    {
        return Err(ModelLaneError::IntegrityViolation(
            "routing execution projection does not bind its exact durable row".into(),
        ));
    }
    let mut attempts_by_key = BTreeMap::new();
    for attempt in attempts {
        let key = (attempt.stage_id.clone(), attempt.attempt);
        if attempts_by_key.insert(key, attempt).is_some() {
            return Err(ModelLaneError::IntegrityViolation(
                "routing attempt identity is ambiguous in exact scope".into(),
            ));
        }
    }
    let mut outbox_by_key = BTreeMap::new();
    for row in outbox {
        let key = (row.stage_id.clone(), row.attempt);
        if outbox_by_key.insert(key, row).is_some() {
            return Err(ModelLaneError::IntegrityViolation(
                "routing outbox identity is ambiguous in exact scope".into(),
            ));
        }
    }
    for stage in execution.stages.values_mut() {
        let key = (stage.stage_id.clone(), i64::from(stage.attempt));
        let attempt_row = attempts_by_key.get(&key).ok_or_else(|| {
            ModelLaneError::IntegrityViolation(
                "routing execution is missing its current attempt authority".into(),
            )
        })?;
        let mut durable_stage: super::routing_execution::ModelLaneRoutingStageState =
            serde_json::from_value(attempt_row.record_json.clone())?;
        if attempt_row.execution_id != execution.execution_id
            || attempt_row.run_id != execution.run_id
            || attempt_row.attempt_id
                != format!(
                    "{}:{}:{}",
                    execution.execution_id, stage.stage_id, stage.attempt
                )
            || attempt_row.state != routing_stage_state_name(durable_stage.state)?
            || routing_attempt_projection_json(stage)? != attempt_row.record_json
            || attempt_row.event_ledger_event_id.trim().is_empty()
            || attempt_row.event_ledger_seq <= 0
        {
            return Err(ModelLaneError::IntegrityViolation(
                "routing attempt projection/EventLedger identity mismatch".into(),
            ));
        }
        durable_stage.event_ledger_event_id = attempt_row.event_ledger_event_id.clone();
        durable_stage.event_ledger_seq = attempt_row.event_ledger_seq;
        let outbox_row = outbox_by_key.get(&key).ok_or_else(|| {
            ModelLaneError::IntegrityViolation(
                "routing execution is missing its current outbox authority".into(),
            )
        })?;
        if outbox_row.command_id
            != format!(
                "routing-command:{}:{}:{}",
                execution.execution_id, durable_stage.stage_id, durable_stage.attempt
            )
            || outbox_row.execution_id != execution.execution_id
            || outbox_row.run_id != execution.run_id
            || outbox_row.status != routing_expected_outbox_status(durable_stage.state)
            || outbox_row.lease_owner != durable_stage.lease_owner
            || outbox_row.fencing_token != durable_stage.fencing_token
            || outbox_row.lease_expires_at_unix_ms
                != durable_stage
                    .lease_expires_at_unix_ms
                    .map(i64::try_from)
                    .transpose()
                    .map_err(|_| {
                        ModelLaneError::IntegrityViolation(
                            "routing lease expiry exceeds durable integer range".into(),
                        )
                    })?
            || outbox_row.event_ledger_event_id.trim().is_empty()
            || outbox_row.event_ledger_seq <= 0
        {
            return Err(ModelLaneError::IntegrityViolation(
                "routing outbox projection/EventLedger identity mismatch".into(),
            ));
        }
        *stage = durable_stage;
    }
    execution.event_ledger_event_id = row.event_ledger_event_id;
    execution.event_ledger_seq = row.event_ledger_seq;
    Ok(execution)
}

fn routing_expected_outbox_status(
    state: super::routing_execution::ModelLaneRoutingStageStateKind,
) -> &'static str {
    use super::routing_execution::ModelLaneRoutingStageStateKind as State;
    match state {
        State::Scheduled => "pending",
        State::Claimed | State::InFlight | State::AwaitingAuthority => "claimed",
        State::Cancelled => "cancelled",
        State::Compensated => "compensated",
        State::Succeeded | State::Failed | State::Joined => "acked",
    }
}

fn routing_diagnostics_from_surreal(
    execution: super::routing_execution::ModelLaneRoutingExecutionState,
    outbox: Vec<SurrealModelLaneRoutingOutboxRow>,
) -> ModelLaneResult<super::routing_execution::ModelLaneRoutingExecutionDiagnostics> {
    let graph_stages = execution
        .canonical_graph
        .get("stages")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            ModelLaneError::IntegrityViolation(
                "routing diagnostics canonical graph has no stages".into(),
            )
        })?;
    let mut dependencies = BTreeMap::new();
    for graph_stage in graph_stages {
        let stage_id = graph_stage
            .get("stage_id")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                ModelLaneError::IntegrityViolation(
                    "routing diagnostics graph stage has no identity".into(),
                )
            })?;
        let depends_on = graph_stage
            .get("depends_on")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                ModelLaneError::IntegrityViolation(
                    "routing diagnostics graph stage has no dependency set".into(),
                )
            })?
            .iter()
            .map(|value| {
                value.as_str().map(str::to_owned).ok_or_else(|| {
                    ModelLaneError::IntegrityViolation(
                        "routing diagnostics dependency is not a stable stage id".into(),
                    )
                })
            })
            .collect::<ModelLaneResult<Vec<_>>>()?;
        if dependencies.insert(stage_id.to_owned(), depends_on).is_some() {
            return Err(ModelLaneError::IntegrityViolation(
                "routing diagnostics graph contains duplicate stage identity".into(),
            ));
        }
    }
    let mut outbox_by_key = BTreeMap::new();
    for row in outbox {
        let key = (row.stage_id.clone(), row.attempt);
        if outbox_by_key.insert(key, row).is_some() {
            return Err(ModelLaneError::IntegrityViolation(
                "routing diagnostics outbox identity is ambiguous".into(),
            ));
        }
    }
    let observed_at_unix_ms = Utc::now().timestamp_millis().max(0) as u64;
    let mut stages = Vec::with_capacity(execution.stages.len());
    for stage in execution.stages.values() {
        let key = (stage.stage_id.clone(), i64::from(stage.attempt));
        let row = outbox_by_key.get(&key).ok_or_else(|| {
            ModelLaneError::IntegrityViolation(
                "routing diagnostics current stage has no outbox authority".into(),
            )
        })?;
        stages.push(super::routing_execution::ModelLaneRoutingStageDiagnostics {
            execution_id: execution.execution_id.clone(),
            stage_id: stage.stage_id.clone(),
            state: routing_stage_state_name(stage.state)?,
            attempt: stage.attempt,
            dispatch_target: routing_stable_token(&stage.dispatch_target)?,
            dependency_stage_ids: dependencies
                .get(&stage.stage_id)
                .cloned()
                .ok_or_else(|| {
                    ModelLaneError::IntegrityViolation(
                        "routing diagnostics current stage is absent from canonical graph".into(),
                    )
                })?,
            expected_run_id: stage.expected_run_id.clone(),
            expected_lane_id: stage.expected_lane_id.clone(),
            expected_model_id: stage.expected_model_id.clone(),
            expected_provider: stage
                .expected_provider
                .map(|provider| format!("{provider:?}").to_ascii_lowercase()),
            instance_id: stage.instance_id.clone(),
            lane_id: stage.lane_id.clone(),
            input_refs: stage.input_refs.clone(),
            output_ref: stage.output_ref.clone(),
            output_message_ref: stage.output_message_ref.clone(),
            authority_request_message_ref: stage.authority_request_message_ref.clone(),
            output_sha256: stage.output_sha256.clone(),
            authority_ref: stage.authority_ref.clone(),
            lease_owner: stage.lease_owner.clone(),
            fencing_token: stage.fencing_token.clone(),
            lease_expires_at_unix_ms: stage.lease_expires_at_unix_ms,
            lease_expired: stage
                .lease_expires_at_unix_ms
                .is_some_and(|expires| expires <= observed_at_unix_ms),
            detail: stage.detail.clone(),
            event_ledger_event_id: stage.event_ledger_event_id.clone(),
            event_ledger_seq: stage.event_ledger_seq,
            updated_at_unix_ms: stage.updated_at_unix_ms,
            outbox: super::routing_execution::ModelLaneRoutingOutboxDiagnostics {
                command_id: row.command_id.clone(),
                status: row.status.clone(),
                fencing_token: row.fencing_token.clone(),
                lease_owner: row.lease_owner.clone(),
                lease_expires_at_unix_ms: row
                    .lease_expires_at_unix_ms
                    .map(u64::try_from)
                    .transpose()
                    .map_err(|_| {
                        ModelLaneError::IntegrityViolation(
                            "routing diagnostics outbox has negative lease expiry".into(),
                        )
                    })?,
                event_ledger_event_id: row.event_ledger_event_id.clone(),
                event_ledger_seq: row.event_ledger_seq,
                created_at_unix_ms: u64::try_from(row.created_at_unix_ms).map_err(|_| {
                    ModelLaneError::IntegrityViolation(
                        "routing diagnostics outbox has negative creation time".into(),
                    )
                })?,
                updated_at_unix_ms: u64::try_from(row.updated_at_unix_ms).map_err(|_| {
                    ModelLaneError::IntegrityViolation(
                        "routing diagnostics outbox has negative update time".into(),
                    )
                })?,
            },
        });
    }
    Ok(
        super::routing_execution::ModelLaneRoutingExecutionDiagnostics {
            execution_id: execution.execution_id,
            run_id: execution.run_id,
            selecting_decision_id: execution.selecting_decision_id,
            selecting_decision_event_id: execution.selecting_decision_event_id,
            selecting_decision_event_seq: execution.selecting_decision_event_seq,
            trace_id: execution.trace_id,
            run_span_id: execution.run_span_id,
            coordinator_session_id: execution.coordinator_session_id,
            locus_ref: execution.locus_ref,
            work_packet_id: execution.work_packet_id,
            micro_task_id: execution.micro_task_id,
            task_board_id: execution.task_board_id,
            owner_session: execution.owner_session,
            canonical_graph_sha256: execution.canonical_graph_sha256,
            canonical_launch_plan_sha256: execution.canonical_launch_plan_sha256,
            cloud_consent_receipt_ref: execution.authority.cloud_consent_receipt_ref,
            validator_authority_ref: execution.authority.validator_authority_ref,
            operator_authority_ref: execution.authority.operator_authority_ref,
            initial_input_ref: execution.initial_input_ref,
            initial_input_sha256: execution.initial_input_sha256,
            status: routing_stable_token(&execution.status)?,
            failure_reason: execution.failure_reason,
            cancel_reason: execution.cancel_reason,
            revision: execution.revision,
            stages,
            event_ledger_event_id: execution.event_ledger_event_id,
            event_ledger_seq: execution.event_ledger_seq,
        },
    )
}

fn routing_stable_token<T: Serialize>(value: &T) -> ModelLaneResult<String> {
    serde_json::to_value(value)?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| {
            ModelLaneError::IntegrityViolation(
                "routing enum did not serialize as a stable token".into(),
            )
        })
}

fn surreal_message_record(
    stored: SurrealModelLaneRecord,
) -> ModelLaneResult<ModelLaneMessageRecord> {
    let value: Value = serde_json::from_str(&stored.record_json)?;
    let persisted = if value.get("inner").is_some() {
        serde_json::from_value::<SurrealStoredModelLaneMessage>(value)?
    } else {
        SurrealStoredModelLaneMessage {
            inner: serde_json::from_value(value)?,
            crdt_authority_binding: None,
        }
    };
    Ok(ModelLaneMessageRecord {
        inner: persisted.inner,
        crdt_authority_binding: persisted.crdt_authority_binding,
        event_ledger_event_id: stored.event_id,
        event_ledger_seq: stored.event_seq,
        event_stream_version: stored.event_stream_version,
        transaction_seq: stored.transaction_seq,
    })
}

fn surreal_context_bundle_artifact_record(
    stored: SurrealModelLaneRecord,
) -> ModelLaneResult<ModelLaneContextBundleArtifactBindingRecord> {
    let mut record: ModelLaneContextBundleArtifactBindingRecord =
        serde_json::from_str(&stored.record_json)?;
    record.event_ledger_event_id = stored.event_id;
    record.event_ledger_seq = stored.event_seq;
    record.event_stream_version = stored.event_stream_version;
    record.transaction_seq = stored.transaction_seq;
    Ok(record)
}

fn surreal_context_bundle_handoff_record(
    stored: SurrealModelLaneRecord,
) -> ModelLaneResult<ModelLaneContextBundleHandoffRecord> {
    let mut record: ModelLaneContextBundleHandoffRecord =
        serde_json::from_str(&stored.record_json)?;
    record.event_ledger_event_id = stored.event_id;
    record.event_ledger_seq = stored.event_seq;
    record.event_stream_version = stored.event_stream_version;
    record.transaction_seq = stored.transaction_seq;
    Ok(record)
}

fn surreal_recovery_checkpoint_record(
    stored: SurrealModelLaneRecord,
) -> ModelLaneResult<ModelLaneRecoveryCheckpointRecord> {
    let mut record: ModelLaneRecoveryCheckpointRecord =
        serde_json::from_str(&stored.record_json)?;
    record.event_ledger_event_id = stored.event_id;
    record.event_ledger_seq = stored.event_seq;
    record.event_stream_version = stored.event_stream_version;
    record.transaction_seq = stored.transaction_seq;
    Ok(record)
}

fn surreal_recovery_event_record(
    stored: SurrealModelLaneRecord,
) -> ModelLaneResult<ModelLaneRecoveryEventRecord> {
    let mut record: ModelLaneRecoveryEventRecord = serde_json::from_str(&stored.record_json)?;
    record.event_ledger_event_id = stored.event_id;
    record.event_ledger_seq = stored.event_seq;
    record.event_stream_version = stored.event_stream_version;
    record.transaction_seq = stored.transaction_seq;
    Ok(record)
}

fn surreal_lease_record(stored: SurrealModelLaneRecord) -> ModelLaneResult<ModelLaneLeaseRecord> {
    let mut record: ModelLaneLeaseRecord = serde_json::from_str(&stored.record_json)?;
    record.event_ledger_event_id = stored.event_id;
    record.event_ledger_seq = stored.event_seq;
    record.event_stream_version = stored.event_stream_version;
    record.transaction_seq = stored.transaction_seq;
    Ok(record)
}

fn surreal_diagnostic_tier_record(
    stored: SurrealModelLaneRecord,
) -> ModelLaneResult<ModelLaneDiagnosticTierStatusRecord> {
    let mut record: ModelLaneDiagnosticTierStatusRecord =
        serde_json::from_str(&stored.record_json)?;
    record.event_ledger_event_id = stored.event_id;
    record.event_ledger_seq = stored.event_seq;
    record.event_stream_version = stored.event_stream_version;
    record.transaction_seq = stored.transaction_seq;
    Ok(record)
}

fn surreal_mt_runtime_status_record(
    stored: SurrealModelLaneRecord,
) -> ModelLaneResult<ModelLaneMtRuntimeStatusRecord> {
    let mut record: ModelLaneMtRuntimeStatusRecord = serde_json::from_str(&stored.record_json)?;
    record.event_ledger_event_id = stored.event_id;
    record.event_ledger_seq = stored.event_seq;
    record.event_stream_version = stored.event_stream_version;
    record.transaction_seq = stored.transaction_seq;
    Ok(record)
}

fn surreal_selection_audit_record(
    stored: SurrealModelLaneRecord,
) -> ModelLaneResult<ModelLaneSelectionAuditRecord> {
    let mut record: ModelLaneSelectionAuditRecord = serde_json::from_str(&stored.record_json)?;
    record.event_ledger_event_id = stored.event_id;
    record.event_ledger_seq = stored.event_seq;
    record.event_stream_version = stored.event_stream_version;
    record.transaction_seq = stored.transaction_seq;
    Ok(record)
}

#[cfg(feature = "surreal-test-support")]
fn surreal_record_kind(value: &str) -> Option<SurrealRecordKind> {
    Some(match value {
        "run" => SurrealRecordKind::Run,
        "lane" => SurrealRecordKind::Lane,
        "message" => SurrealRecordKind::Message,
        "promotion_decision" => SurrealRecordKind::PromotionDecision,
        "context_artifact" => SurrealRecordKind::ContextArtifact,
        "context_handoff" => SurrealRecordKind::ContextHandoff,
        "recovery_checkpoint" => SurrealRecordKind::RecoveryCheckpoint,
        "recovery_event" => SurrealRecordKind::RecoveryEvent,
        "lease" => SurrealRecordKind::Lease,
        "diagnostic_tier" => SurrealRecordKind::DiagnosticTier,
        "mt_runtime_status" => SurrealRecordKind::MtRuntimeStatus,
        "session_cleanup_receipt" => SurrealRecordKind::SessionCleanupReceipt,
        "selection_audit" => SurrealRecordKind::SelectionAudit,
        _ => return None,
    })
}

fn cleanup_receipt_from_surreal(
    stored: &SurrealModelLaneRecord,
) -> ModelLaneResult<DurableSessionCleanupReceipt> {
    let value: Value = serde_json::from_str(&stored.record_json)?;
    if value.get("schema_id").and_then(Value::as_str)
        != Some("hsk.swarm_session_cleanup_receipt@1")
    {
        return Err(ModelLaneError::IntegrityViolation(
            "cleanup receipt has an unsupported or missing schema_id".into(),
        ));
    }
    let receipt: DurableSessionCleanupReceipt = serde_json::from_value(value)?;
    if receipt.instance_id != stored.aggregate_id {
        return Err(ModelLaneError::IntegrityViolation(
            "cleanup receipt aggregate identity mismatch".into(),
        ));
    }
    Ok(receipt)
}

fn json_named_value_matches(value: &Value, fields: &[&str], expected: &str) -> bool {
    match value {
        Value::Object(object) => object.iter().any(|(name, value)| {
            (fields.iter().any(|field| *field == name) && json_value_contains(value, expected))
                || json_named_value_matches(value, fields, expected)
        }),
        Value::Array(values) => values
            .iter()
            .any(|value| json_named_value_matches(value, fields, expected)),
        _ => false,
    }
}

fn json_value_contains(value: &Value, expected: &str) -> bool {
    match value {
        Value::String(value) => value == expected,
        Value::Array(values) => values
            .iter()
            .any(|value| json_value_contains(value, expected)),
        Value::Object(values) => values
            .values()
            .any(|value| json_value_contains(value, expected)),
        _ => false,
    }
}

fn one_navigation_run_id(
    value: &str,
    run_ids: BTreeSet<String>,
) -> ModelLaneResult<Option<String>> {
    if run_ids.len() > 1 {
        return Err(ModelLaneError::AmbiguousLookup(format!(
            "navigation value {value} resolves to multiple exact-scope runs"
        )));
    }
    Ok(run_ids.into_iter().next())
}

async fn validate_recovery_payload_refs_surreal(
    provider: &SurrealModelLaneStore,
    scope: &SurrealModelLaneScope,
    checkpoint: &ModelLaneRecoveryCheckpointRecord,
    messages: &[ModelLaneMessageRecord],
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
    for payload_ref in &checkpoint.open_payload_refs {
        let message_match = messages.iter().any(|message| {
            message.payload_ref == payload_ref.as_str()
                && message.event_ledger_seq <= checkpoint.last_event_ledger_seq
        });
        let rows = provider
            .find_by_term(
                SurrealRecordKind::ContextArtifact,
                &format!("artifact_ref={payload_ref}"),
                scope,
            )
            .await?;
        let mut artifact_match = false;
        for row in rows {
            provider
                .validate_event_link(SurrealRecordKind::ContextArtifact, &row, scope)
                .await?;
            artifact_match |= row.run_id == checkpoint.run_id
                && row.event_seq <= checkpoint.last_event_ledger_seq;
        }
        if !message_match && !artifact_match {
            return Err(ModelLaneError::IntegrityViolation(format!(
                "checkpoint {} open payload ref {} has no recovery-bounded exact-scope authority",
                checkpoint.checkpoint_id, payload_ref
            )));
        }
    }
    Ok(())
}

async fn validate_recovery_crdt_posture_surreal(
    provider: &SurrealModelLaneStore,
    scope: &SurrealModelLaneScope,
    events: &[ModelLaneRecoveryEventRecord],
    messages: &[ModelLaneMessageRecord],
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
        if event.event_kind != ModelLaneRecoveryEventKind::CrdtUpdateObserved {
            continue;
        }
        let base_ref = event.crdt_base_snapshot_ref.as_deref().unwrap_or_default();
        let state_vector = event.crdt_state_vector.as_deref().unwrap_or_default();
        let snapshot = provider
            .crdt_snapshot_by_ref(base_ref, scope)
            .await?
            .ok_or_else(|| {
                ModelLaneError::IntegrityViolation(format!(
                    "recovery event {} CRDT base is outside its exact resource scope",
                    event.recovery_event_id
                ))
            })?;
        validate_surreal_crdt_snapshot_row(&snapshot, scope)?;
        if snapshot.state_vector != state_vector
            || !messages.iter().any(|message| {
                message.crdt_base_snapshot_ref.as_deref() == Some(base_ref)
                    && message.crdt_state_vector.as_deref() == Some(state_vector)
                    && message.crdt_stale_base_ref.is_none()
                    && event
                        .lane_id
                        .as_deref()
                        .map_or(true, |lane_id| message.from_lane_id == lane_id)
            })
        {
            let failure = ModelLaneRecoveryFailureKind::StaleCrdtBase;
            return Err(ModelLaneError::InvalidInput(format!(
                "{} {} recovery_event_id {} has no exact-scope message CRDT base/state authority",
                failure.code(),
                failure.as_str(),
                event.recovery_event_id
            )));
        }
    }
    Ok(())
}

fn recovery_checkpoint_search_terms(record: &ModelLaneRecoveryCheckpointRecord) -> Vec<String> {
    let mut terms = vec![
        format!("idempotency_key={}", record.idempotency_key),
        format!("session_id={}", record.session_id),
        format!("model_session_id={}", record.model_session_id),
        format!("event_ledger_stream_id={}", record.event_ledger_stream_id),
        format!("checkpoint_status={}", record.checkpoint_status.as_str()),
    ];
    if let Some(lane_id) = record.lane_id.as_deref() {
        terms.push(format!("lane_id={lane_id}"));
    }
    terms
}

fn recovery_event_search_terms(record: &ModelLaneRecoveryEventRecord) -> Vec<String> {
    let mut terms = vec![
        format!("idempotency_key={}", record.idempotency_key),
        format!("event_ledger_stream_id={}", record.event_ledger_stream_id),
        format!("event_kind={}", record.event_kind.as_str()),
        format!("recovery_status={}", record.recovery_status.as_str()),
        format!("replay_order_seq={}", record.replay_order_seq),
    ];
    if let Some(lane_id) = record.lane_id.as_deref() {
        terms.push(format!("lane_id={lane_id}"));
    }
    if let Some(session_id) = record.session_id.as_deref() {
        terms.push(format!("session_id={session_id}"));
    }
    if let Some(model_session_id) = record.model_session_id.as_deref() {
        terms.push(format!("model_session_id={model_session_id}"));
    }
    terms
}

fn diagnostic_tier_search_terms(record: &ModelLaneDiagnosticTierStatusRecord) -> Vec<String> {
    vec![
        format!("idempotency_key={}", record.idempotency_key),
        format!("behavior_id={}", record.behavior_id),
        format!("tier={}", record.tier.as_str()),
        format!("micro_task_id={}", record.micro_task_id),
    ]
}

fn mt_runtime_status_search_terms(record: &ModelLaneMtRuntimeStatusRecord) -> Vec<String> {
    vec![
        format!("idempotency_key={}", record.idempotency_key),
        format!("work_packet_id={}", record.work_packet_id),
        format!("micro_task_id={}", record.micro_task_id),
        format!("task_board_id={}", record.task_board_id),
        format!("status={}", record.status.as_str()),
    ]
}

fn lease_search_terms(record: &ModelLaneLeaseRecord) -> Vec<String> {
    let mut terms = vec![
        format!("idempotency_key={}", record.idempotency_key),
        format!("scope_ref={}", record.scope_ref),
        format!("holder_actor_id={}", record.holder_actor_id),
        format!("holder_session_id={}", record.holder_session_id),
        format!("state={}", record.state.as_str()),
    ];
    if let Some(lane_id) = record.lane_id.as_deref() {
        terms.push(format!("lane_id={lane_id}"));
    }
    terms
}

fn context_bundle_artifact_search_terms(
    record: &ModelLaneContextBundleArtifactBindingRecord,
) -> Vec<String> {
    vec![
        format!("idempotency_key={}", record.idempotency_key),
        format!("artifact_ref={}", record.artifact_ref),
        format!("trace_id={}", record.trace_id),
    ]
}

fn context_bundle_handoff_search_terms(
    record: &ModelLaneContextBundleHandoffRecord,
) -> Vec<String> {
    vec![
        format!("idempotency_key={}", record.idempotency_key),
        format!("context_bundle_id={}", record.context_bundle_id),
        format!("downstream_lane_id={}", record.downstream_lane_id),
        format!("source_message_id={}", record.source_message_id),
        format!("artifact_ref={}", record.artifact_ref),
        format!("trace_id={}", record.trace_id),
    ]
}

fn validate_surreal_message_retry(
    existing: ModelLaneMessageRecord,
    input: &NewModelLaneMessage,
) -> ModelLaneResult<ModelLaneMessageRecord> {
    if existing.payload_sha256 != input.payload_sha256 {
        return Err(ModelLaneError::IdempotencyConflict(format!(
            "idempotency_key {} already belongs to payload_sha256 {}",
            input.idempotency_key, existing.payload_sha256
        )));
    }
    // Contract (MT-002): a duplicate retry carrying the same idempotency_key AND the same
    // payload hash MUST be idempotent; only a different payload hash may fail closed. The
    // payload hash is checked above, so what remains is to normalize the fields that are
    // per-attempt identity rather than meaning. message_id and its derived span, correlation
    // and artifact-ref values differ on every retry by construction, so comparing them would
    // reject exactly the replay the contract requires to succeed. Meaning-bearing fields
    // (run_id, lanes, kind, authority, summary, payload_sha256) stay under the equality guard
    // and still fail closed.
    let mut retry_identity = input.clone();
    retry_identity.message_id = existing.message_id.clone();
    retry_identity.message_span_id = existing.message_span_id.clone();
    retry_identity.payload_ref = existing.inner.payload_ref.clone();
    retry_identity.replay_order_key = existing.inner.replay_order_key.clone();
    if let (Some(retry_routing), Some(existing_routing)) = (
        retry_identity.routing.as_mut(),
        existing.inner.routing.as_ref(),
    ) {
        retry_routing.correlation_id = existing_routing.correlation_id.clone();
    }
    ensure_idempotent_input_matches(
        "model_lane_message",
        &input.idempotency_key,
        &existing.inner,
        &retry_identity,
    )?;
    Ok(existing)
}

fn surreal_promotion_record(
    stored: SurrealModelLaneRecord,
) -> ModelLaneResult<ModelLanePromotionDecisionRecord> {
    let mut record: ModelLanePromotionDecisionRecord =
        serde_json::from_str(&stored.record_json)?;
    record.event_ledger_event_id = stored.event_id;
    record.event_ledger_seq = stored.event_seq;
    record.event_stream_version = stored.event_stream_version;
    record.transaction_seq = stored.transaction_seq;
    Ok(record)
}

fn validate_surreal_promotion_record_authority(
    record: &ModelLanePromotionDecisionRecord,
    exact_scope: &ExactResourceScopeAttribution,
) -> ModelLaneResult<()> {
    let expected_basis = promotion_canonical_hash_basis(
        &record.inner,
        record.outcome,
        record.final_state,
        record.denial_reason,
        record.current_event_ledger_version,
        record.current_schema_id.as_deref(),
        exact_scope,
    );
    let expected_hash = dexterity_sha256_hex(serde_json::to_vec(&expected_basis)?);
    let expected_input_refs = canonicalize_refs("input_refs", &record.input_refs)?;
    if record.canonical_hash_basis != expected_basis
        || record.canonical_decision_hash != expected_hash
        || record.canonical_input_refs != expected_input_refs
        || record.state_history != promotion_state_history(record.outcome)
        || record.state_history.last().copied() != Some(record.final_state)
        || record.event_ledger_seq <= 0
        || record.event_stream_version <= 0
        || record.transaction_seq <= 0
    {
        return Err(ModelLaneError::AuthorityDenied(
            "PromotionGate decision projection does not equal its canonical scoped Surreal authority"
                .into(),
        ));
    }
    Ok(())
}

fn promotion_search_terms(input: &ModelLanePromotionDecisionRecord) -> Vec<String> {
    vec![
        format!("decision_id={}", input.decision_id),
        format!("run_id={}", input.run_id),
        format!("trace_id={}", input.trace_id),
        format!("idempotency_key={}", input.idempotency_key),
        format!("canonical_decision_hash={}", input.canonical_decision_hash),
    ]
}

fn run_search_terms(input: &NewModelLaneRun) -> Vec<String> {
    let mut terms = vec![
        format!("run_id={}", input.run_id),
        format!("trace_id={}", input.trace_id),
        format!("coordinator_session_id={}", input.coordinator_session_id),
        format!("idempotency_key={}", input.idempotency_key),
    ];
    terms.extend(input.lane_ids.iter().map(|value| format!("lane_id={value}")));
    terms
}

fn lane_search_terms(input: &NewModelLane) -> Vec<String> {
    vec![
        format!("lane_id={}", input.lane_id),
        format!("run_id={}", input.run_id),
        format!("trace_id={}", input.trace_id),
        format!("session_id={}", input.session_id),
        format!("model_session_id={}", input.model_session_id),
    ]
}

fn message_search_terms(input: &NewModelLaneMessage) -> Vec<String> {
    let mut terms = vec![
        format!("message_id={}", input.message_id),
        format!("run_id={}", input.run_id),
        format!("trace_id={}", input.trace_id),
        format!("from_lane_id={}", input.from_lane_id),
        format!("idempotency_key={}", input.idempotency_key),
        format!("payload_ref={}", input.payload_ref),
    ];
    if let ModelLaneTarget::Lane(target_lane_id) = &input.to_lane {
        terms.push(format!("to_lane_id={target_lane_id}"));
    }
    terms
}

fn surreal_message_write(
    input: &NewModelLaneMessage,
    crdt_authority_binding: Option<ModelLaneCrdtAuthorityBinding>,
    scope: &SurrealModelLaneScope,
) -> ModelLaneResult<SurrealModelLaneWrite> {
    let persisted = SurrealStoredModelLaneMessage {
        inner: input.clone(),
        crdt_authority_binding,
    };
    Ok(SurrealModelLaneWrite {
        kind: SurrealRecordKind::Message,
        aggregate_id: input.message_id.clone(),
        run_id: input.run_id.clone(),
        idempotency_key: input.idempotency_key.clone(),
        record_json: serde_json::to_string(&persisted)?,
        search_terms: message_search_terms(input),
        event_payload_json: scoped_surreal_event_payload(
            "hsk.model_lane_message@1",
            &persisted,
            scope,
        )?,
    })
}

fn surreal_binding_write(
    input: &NewModelLaneContextBundleArtifactBinding,
    scope: &SurrealModelLaneScope,
) -> ModelLaneResult<SurrealModelLaneWrite> {
    let prepared = ModelLaneContextBundleArtifactBindingRecord {
        inner: input.clone(),
        artifact_binding_hash: context_bundle_artifact_binding_hash(input)?,
        event_ledger_event_id: String::new(),
        event_ledger_seq: 0,
        event_stream_version: 0,
        transaction_seq: 0,
    };
    Ok(SurrealModelLaneWrite {
        kind: SurrealRecordKind::ContextArtifact,
        aggregate_id: input.artifact_binding_id.clone(),
        run_id: input.run_id.clone(),
        idempotency_key: input.idempotency_key.clone(),
        record_json: serde_json::to_string(&prepared)?,
        search_terms: vec![
            format!("artifact_binding_id={}", input.artifact_binding_id),
            format!("run_id={}", input.run_id),
            format!("trace_id={}", input.trace_id),
            format!("artifact_ref={}", input.artifact_ref),
            format!("idempotency_key={}", input.idempotency_key),
        ],
        event_payload_json: scoped_surreal_event_payload(
            "hsk.model_lane_context_bundle_artifact@1",
            &prepared,
            scope,
        )?,
    })
}

fn validate_surreal_binding_retry(
    stored: &SurrealModelLaneRecord,
    input: &NewModelLaneContextBundleArtifactBinding,
) -> ModelLaneResult<()> {
    let existing: NewModelLaneContextBundleArtifactBinding =
        serde_json::from_str(&stored.record_json)?;
    let expected_hash = context_bundle_artifact_binding_hash(input)?;
    if context_bundle_artifact_binding_hash(&existing)? != expected_hash {
        return Err(ModelLaneError::IdempotencyConflict(format!(
            "idempotency_key {} already belongs to a different artifact binding",
            input.idempotency_key
        )));
    }
    Ok(())
}

fn lane_event_idempotency_key(input: &NewModelLane) -> String {
    if input.restart_generation == 0 {
        format!("model-lane:{}:{}", input.run_id, input.lane_id)
    } else {
        format!(
            "model-lane:{}:{}:restart:{}",
            input.run_id, input.lane_id, input.restart_generation
        )
    }
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewModelLaneCrdtLease {
    pub lease_id: String,
    pub lane_id: String,
    pub document_id: String,
    pub crdt_document_id: String,
    pub actor_id: String,
    pub actor_kind: String,
    pub session_id: String,
    pub correlation_id: String,
    pub ttl_seconds: i64,
    pub kernel_task_run_id: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelLaneCrdtLeaseRecord {
    pub lease_id: String,
    pub lane_id: String,
    pub document_id: Option<String>,
    pub crdt_document_id: Option<String>,
    pub actor_id: String,
    pub actor_kind: String,
    pub session_id: String,
    pub correlation_id: String,
    pub scope_kind: String,
    pub scope_id: String,
    pub claimed_at_utc: DateTime<Utc>,
    pub expires_at_utc: DateTime<Utc>,
    pub renewal_count: i64,
    pub released_at_utc: Option<DateTime<Utc>>,
    pub recorded_event_id: String,
    pub last_transition_event_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelLaneCrdtLeaseClaimOutcome {
    Claimed(ModelLaneCrdtLeaseRecord),
    AlreadyClaimed(ModelLaneCrdtLeaseRecord),
    ScopeHeld(ModelLaneCrdtLeaseRecord),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewModelLaneCrdtUpdate {
    pub schema_id: String,
    pub document_id: String,
    pub crdt_document_id: String,
    pub update_id: String,
    pub update_seq: i64,
    pub update_bytes: Vec<u8>,
    pub actor_id: String,
    pub actor_kind: String,
    pub session_id: String,
    pub trace_id: String,
    pub state_vector_before: String,
    pub state_vector_after: String,
    pub replay_order_key: String,
    pub dependency_update_ids: Vec<String>,
    pub site_id: String,
    pub kernel_task_run_id: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelLaneCrdtUpdateRecord {
    pub update_id: String,
    pub update_seq: i64,
    pub update_sha256: String,
    pub update_bytes_ref: String,
    pub state_vector_before: String,
    pub state_vector_after: String,
    pub replay_order_key: String,
    pub dependency_update_ids: Vec<String>,
    pub replay_schema_version: String,
    pub event_ledger_event_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelLaneCrdtUpdateAppendOutcome {
    Stored(ModelLaneCrdtUpdateRecord),
    AlreadyStored(ModelLaneCrdtUpdateRecord),
    ContentMismatch { update_id: String },
    StaleHead {
        head_update_seq: i64,
        head_state_vector: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewModelLaneCrdtSnapshot {
    pub schema_id: String,
    pub snapshot_id: String,
    pub document_id: String,
    pub crdt_document_id: String,
    pub covered_update_seq: i64,
    pub state_vector: String,
    pub snapshot_bytes: Vec<u8>,
    pub actor_id: String,
    pub actor_kind: String,
    pub promotion_evidence_update_ids: Vec<String>,
    pub session_id: String,
    pub kernel_task_run_id: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelLaneCrdtSnapshotRecord {
    pub snapshot_id: String,
    pub covered_update_seq: i64,
    pub state_vector: String,
    pub snapshot_sha256: String,
    pub snapshot_bytes_ref: String,
    pub event_ledger_event_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewModelLaneCrdtProposal {
    pub proposal_id: String,
    pub document_id: String,
    pub crdt_document_id: String,
    pub base_update_seq: i64,
    pub base_state_vector: String,
    pub proposed_diff: Value,
    pub source_span_citations: Vec<String>,
    pub actor_id: String,
    pub actor_kind: String,
    pub session_id: String,
    pub correlation_id: String,
    pub lease_id: String,
    pub kernel_task_run_id: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLaneCrdtProposalDecision {
    Approved,
    Rejected,
    Promoted,
}

impl ModelLaneCrdtProposalDecision {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Promoted => "promoted",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelLaneCrdtProposalRecord {
    pub proposal_id: String,
    pub document_id: String,
    pub crdt_document_id: String,
    pub base_update_seq: i64,
    pub base_state_vector: String,
    pub proposed_diff: Value,
    pub diff_sha256: String,
    pub actor_id: String,
    pub actor_kind: String,
    pub session_id: String,
    pub correlation_id: String,
    pub lease_id: Option<String>,
    pub review_state: String,
    pub decided_by: Option<String>,
    pub applied_update_id: Option<String>,
    /// Approved-diff integrity hash. The cited Yjs row owns its independent
    /// `update_sha256`; these two hash domains are intentionally never equated.
    pub applied_update_sha256: Option<String>,
    pub applied_event_id: Option<String>,
    pub promotion_requested_event_id: Option<String>,
    pub promotion_accepted_event_id: Option<String>,
    pub recorded_event_id: String,
    pub last_transition_event_id: String,
}

fn validated_crdt_actor(actor_id: &str, actor_kind: &str) -> ModelLaneResult<KernelActor> {
    let actor = KnowledgeActorIdV1::parse(actor_id).map_err(|error| {
        ModelLaneError::InvalidInput(format!("CRDT actor_id is invalid: {error}"))
    })?;
    if actor.kind().as_str() != actor_kind {
        return Err(ModelLaneError::InvalidInput(
            "CRDT actor_kind does not match canonical actor_id".into(),
        ));
    }
    Ok(actor.to_kernel_actor())
}

#[allow(clippy::too_many_arguments)]
fn new_surreal_crdt_event(
    scope: &SurrealModelLaneScope,
    kernel_task_run_id: &str,
    session_run_id: &str,
    aggregate_type: &str,
    aggregate_id: &str,
    idempotency_key: &str,
    event_type: KernelEventType,
    actor: KernelActor,
    correlation_id: Option<String>,
    payload: Value,
) -> ModelLaneResult<SurrealModelLaneCrdtEventWrite> {
    let event_id = scoped_crdt_event_id(scope, idempotency_key, event_type.as_str());
    let mut builder = NewKernelEvent::builder(
        kernel_task_run_id,
        session_run_id,
        event_type,
        actor,
    )
    .aggregate(aggregate_type, aggregate_id)
    .idempotency_key(idempotency_key)
    .source_component("model_lane_crdt")
    .payload(payload);
    if let Some(correlation_id) = correlation_id {
        builder = builder.correlation_id(correlation_id);
    }
    let event = builder
        .build()
        .map_err(|error| ModelLaneError::InvalidInput(error.to_string()))?;
    Ok(SurrealModelLaneCrdtEventWrite {
        event_id,
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
    })
}

#[allow(clippy::too_many_arguments)]
fn new_surreal_crdt_promotion_event(
    scope: &SurrealModelLaneScope,
    kernel_task_run_id: &str,
    session_run_id: &str,
    proposal_id: &str,
    idempotency_key: &str,
    event_type: KernelEventType,
    actor_id: &str,
    correlation_id: &str,
    causation_id: Option<String>,
    payload: Value,
) -> ModelLaneResult<SurrealModelLaneCrdtEventWrite> {
    let event_id = scoped_crdt_event_id(scope, idempotency_key, event_type.as_str());
    let mut builder = NewKernelEvent::builder(
        kernel_task_run_id,
        session_run_id,
        event_type,
        KernelActor::Operator(actor_id.to_owned()),
    )
    .aggregate("knowledge_ai_edit_promotion", proposal_id)
    .idempotency_key(idempotency_key)
    .source_component("knowledge_crdt_ai_edit_proposal")
    .correlation_id(correlation_id)
    .payload(payload);
    if let Some(causation_id) = causation_id {
        builder = builder.causation_id(causation_id);
    }
    let event = builder
        .build()
        .map_err(|error| ModelLaneError::InvalidInput(error.to_string()))?;
    Ok(SurrealModelLaneCrdtEventWrite {
        event_id,
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
    })
}

fn scoped_crdt_event_id(
    scope: &SurrealModelLaneScope,
    idempotency_key: &str,
    event_type: &str,
) -> String {
    let digest = dexterity_sha256_hex(
        format!(
            "crdt-event\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
            scope.owner_account_id,
            scope.actor_principal_id,
            scope.authenticated_session_id,
            scope.access_space_id,
            scope.workspace_id,
            event_type,
            idempotency_key,
        )
        .as_bytes(),
    );
    format!("crdt_evt_{digest}")
}

fn scoped_crdt_ref(kind: &str, identity: &str, scope: &SurrealModelLaneScope) -> String {
    let digest = dexterity_sha256_hex(
        format!(
            "crdt-ref\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
            scope.owner_account_id,
            scope.actor_principal_id,
            scope.authenticated_session_id,
            scope.access_space_id,
            scope.workspace_id,
            kind,
            identity,
        )
        .as_bytes(),
    );
    format!("surreal://{kind}/{digest}")
}

fn crdt_lease_record(row: SurrealModelLaneCrdtLeaseHistory) -> ModelLaneCrdtLeaseRecord {
    ModelLaneCrdtLeaseRecord {
        lease_id: row.lease_id,
        lane_id: row.lane_id,
        document_id: row.document_id,
        crdt_document_id: row.crdt_document_id,
        actor_id: row.actor_id,
        actor_kind: row.actor_kind,
        session_id: row.session_id,
        correlation_id: row.correlation_id,
        scope_kind: row.scope_kind,
        scope_id: row.scope_id,
        claimed_at_utc: row.claimed_at_utc,
        expires_at_utc: row.expires_at_utc,
        renewal_count: row.renewal_count,
        released_at_utc: row.released_at_utc,
        recorded_event_id: row.recorded_event_id,
        last_transition_event_id: row.last_transition_event_id,
    }
}

fn crdt_update_record(row: SurrealModelLaneCrdtUpdate) -> ModelLaneCrdtUpdateRecord {
    ModelLaneCrdtUpdateRecord {
        update_id: row.update_id,
        update_seq: row.update_seq,
        update_sha256: row.update_sha256,
        update_bytes_ref: row.update_bytes_ref,
        state_vector_before: row.state_vector_before,
        state_vector_after: row.state_vector_after,
        replay_order_key: row.replay_order_key,
        dependency_update_ids: row.dependency_update_ids,
        replay_schema_version: row.replay_schema_version,
        event_ledger_event_id: row.event_ledger_event_id,
    }
}

fn crdt_update_append_outcome(
    outcome: SurrealCrdtUpdateAppendOutcome,
) -> ModelLaneCrdtUpdateAppendOutcome {
    match outcome {
        SurrealCrdtUpdateAppendOutcome::Stored(row) => {
            ModelLaneCrdtUpdateAppendOutcome::Stored(crdt_update_record(row))
        }
        SurrealCrdtUpdateAppendOutcome::AlreadyStored(row) => {
            ModelLaneCrdtUpdateAppendOutcome::AlreadyStored(crdt_update_record(row))
        }
        SurrealCrdtUpdateAppendOutcome::ContentMismatch { update_id } => {
            ModelLaneCrdtUpdateAppendOutcome::ContentMismatch { update_id }
        }
        SurrealCrdtUpdateAppendOutcome::StaleHead {
            head_update_seq,
            head_state_vector,
        } => ModelLaneCrdtUpdateAppendOutcome::StaleHead {
            head_update_seq,
            head_state_vector,
        },
    }
}

fn crdt_snapshot_record(row: SurrealModelLaneCrdtSnapshot) -> ModelLaneCrdtSnapshotRecord {
    ModelLaneCrdtSnapshotRecord {
        snapshot_id: row.snapshot_id,
        covered_update_seq: row.covered_update_seq,
        state_vector: row.state_vector,
        snapshot_sha256: row.snapshot_sha256,
        snapshot_bytes_ref: row.snapshot_bytes_ref,
        event_ledger_event_id: row.event_ledger_event_id,
    }
}

fn crdt_proposal_record(row: SurrealModelLaneCrdtProposalRecord) -> ModelLaneCrdtProposalRecord {
    ModelLaneCrdtProposalRecord {
        proposal_id: row.proposal_id,
        document_id: row.document_id,
        crdt_document_id: row.crdt_document_id,
        base_update_seq: row.base_update_seq,
        base_state_vector: row.base_state_vector,
        proposed_diff: row.proposed_diff,
        diff_sha256: row.diff_sha256,
        actor_id: row.actor_id,
        actor_kind: row.actor_kind,
        session_id: row.session_id,
        correlation_id: row.correlation_id,
        lease_id: row.lease_id,
        review_state: row.review_state,
        decided_by: row.decided_by,
        applied_update_id: row.applied_update_id,
        applied_update_sha256: row.applied_update_sha256,
        applied_event_id: row.applied_event_id,
        promotion_requested_event_id: row.promotion_requested_event_id,
        promotion_accepted_event_id: row.promotion_accepted_event_id,
        recorded_event_id: row.recorded_event_id,
        last_transition_event_id: row.last_transition_event_id,
    }
}

fn validate_new_crdt_lease(input: &NewModelLaneCrdtLease) -> ModelLaneResult<()> {
    for (field, value) in [
        ("lease_id", input.lease_id.as_str()),
        ("lane_id", input.lane_id.as_str()),
        ("document_id", input.document_id.as_str()),
        ("crdt_document_id", input.crdt_document_id.as_str()),
        ("actor_id", input.actor_id.as_str()),
        ("actor_kind", input.actor_kind.as_str()),
        ("session_id", input.session_id.as_str()),
        ("correlation_id", input.correlation_id.as_str()),
        ("kernel_task_run_id", input.kernel_task_run_id.as_str()),
        ("idempotency_key", input.idempotency_key.as_str()),
    ] {
        require_token(field, value)?;
    }
    if input.ttl_seconds <= 0 {
        return Err(ModelLaneError::InvalidInput(
            "CRDT lease ttl_seconds must be positive".into(),
        ));
    }
    Ok(())
}

fn validate_new_crdt_update(
    input: &NewModelLaneCrdtUpdate,
    scope: &SurrealModelLaneScope,
) -> ModelLaneResult<()> {
    for (field, value) in [
        ("schema_id", input.schema_id.as_str()),
        ("document_id", input.document_id.as_str()),
        ("crdt_document_id", input.crdt_document_id.as_str()),
        ("update_id", input.update_id.as_str()),
        ("actor_id", input.actor_id.as_str()),
        ("actor_kind", input.actor_kind.as_str()),
        ("session_id", input.session_id.as_str()),
        ("trace_id", input.trace_id.as_str()),
        ("state_vector_before", input.state_vector_before.as_str()),
        ("state_vector_after", input.state_vector_after.as_str()),
        ("replay_order_key", input.replay_order_key.as_str()),
        ("site_id", input.site_id.as_str()),
        ("kernel_task_run_id", input.kernel_task_run_id.as_str()),
        ("idempotency_key", input.idempotency_key.as_str()),
    ] {
        require_token(field, value)?;
    }
    if input.schema_id != CRDT_UPDATE_RECORD_SCHEMA_ID
        || input.update_seq <= 0
        || !input.state_vector_before.starts_with("hsk-sv1:")
        || !input.state_vector_after.starts_with("hsk-sv1:")
        || input.state_vector_before == input.state_vector_after
    {
        return Err(ModelLaneError::InvalidInput(
            "CRDT update schema, sequence, or state-vector transition is invalid".into(),
        ));
    }
    Update::decode_v1(&input.update_bytes).map_err(|error| {
        ModelLaneError::InvalidInput(format!("CRDT update is not Yjs v1: {error}"))
    })?;
    let actor = KnowledgeActorIdV1::parse(&input.actor_id).map_err(|error| {
        ModelLaneError::InvalidInput(format!("CRDT actor_id is invalid: {error}"))
    })?;
    let site = derive_knowledge_site_id(&scope.workspace_id, &input.crdt_document_id, &actor);
    if site.site_id != input.site_id {
        return Err(ModelLaneError::InvalidInput(
            "CRDT update site_id is not server-derived".into(),
        ));
    }
    Ok(())
}

fn validate_new_crdt_snapshot(input: &NewModelLaneCrdtSnapshot) -> ModelLaneResult<()> {
    for (field, value) in [
        ("schema_id", input.schema_id.as_str()),
        ("snapshot_id", input.snapshot_id.as_str()),
        ("document_id", input.document_id.as_str()),
        ("crdt_document_id", input.crdt_document_id.as_str()),
        ("state_vector", input.state_vector.as_str()),
        ("actor_id", input.actor_id.as_str()),
        ("actor_kind", input.actor_kind.as_str()),
        ("session_id", input.session_id.as_str()),
        ("kernel_task_run_id", input.kernel_task_run_id.as_str()),
        ("idempotency_key", input.idempotency_key.as_str()),
    ] {
        require_token(field, value)?;
    }
    if input.schema_id != CRDT_SNAPSHOT_RECORD_SCHEMA_ID
        || input.covered_update_seq < 0
        || !input.state_vector.starts_with("hsk-sv1:")
    {
        return Err(ModelLaneError::InvalidInput(
            "CRDT snapshot schema, sequence, or state vector is invalid".into(),
        ));
    }
    Update::decode_v1(&input.snapshot_bytes).map_err(|error| {
        ModelLaneError::InvalidInput(format!("CRDT snapshot is not Yjs v1: {error}"))
    })?;
    Ok(())
}

fn validate_new_crdt_proposal(input: &NewModelLaneCrdtProposal) -> ModelLaneResult<()> {
    for (field, value) in [
        ("proposal_id", input.proposal_id.as_str()),
        ("document_id", input.document_id.as_str()),
        ("crdt_document_id", input.crdt_document_id.as_str()),
        ("base_state_vector", input.base_state_vector.as_str()),
        ("actor_id", input.actor_id.as_str()),
        ("actor_kind", input.actor_kind.as_str()),
        ("session_id", input.session_id.as_str()),
        ("correlation_id", input.correlation_id.as_str()),
        ("lease_id", input.lease_id.as_str()),
        ("kernel_task_run_id", input.kernel_task_run_id.as_str()),
        ("idempotency_key", input.idempotency_key.as_str()),
    ] {
        require_token(field, value)?;
    }
    if input.base_update_seq < 0
        || !input.base_state_vector.starts_with("hsk-sv1:")
        || !input.proposed_diff.is_object()
        || input.source_span_citations.is_empty()
        || input
            .source_span_citations
            .iter()
            .any(|citation| citation.trim().is_empty())
    {
        return Err(ModelLaneError::InvalidInput(
            "CRDT proposal content or source citations are invalid".into(),
        ));
    }
    Ok(())
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






/// Recovery records are children of one canonical ModelLaneRun. Account writes
/// therefore require the same complete five-dimensional scope on parent and
/// child; only an explicit system store may use the legacy NULL-scope path.
fn require_exact_recovery_account_scope(access: &ResourceAccessContext) -> ModelLaneResult<()> {
    if !access.is_system() {
        access.require_lifecycle_active()?;
    }
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
    access.require_lifecycle_active()?;
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





/// Boot recovery is intentionally system-authority and cross-owner, but every
/// child it replays must remain pinned to the canonical run's stored scope.
/// Comparing exact stored attribution keeps malformed/missing authority fail-closed and
/// supports the explicit legacy system path where all five values are NULL.

/// System boot recovery may discover runs across owners, but every child it
/// appends remains a derivative of the canonical run. Derive that write
/// authority from the run's physical scope columns under the recovery fence;
/// never stamp the system scanner's NULL scope onto an account-owned run.


/// Current committed high-watermark (max global EventLedger `event_sequence`) for a
/// ModelLaneRun stream. Used as the forward catch-up bound when the run advanced past
/// its last checkpoint (spec 4.3.9.2.5: "apply EventLedger records after that sequence
/// in order").

/// True when the coordinator-owned ModelLaneMessage stream genuinely advanced past the
/// checkpoint (a NEW `model_lane_message` was committed after
/// `checkpoint_bound_event_ledger_seq`). Only real forward-message progress triggers
/// catch-up. Current-state adjunct writes recorded after a checkpoint with no new
/// message (post-checkpoint leases, MT status, cloud denials) are NOT forward progress.
/// Leases are reconciled separately from current ownership authority; they never widen
/// this replay bound. This distinguishes legitimate message catch-up from adjunct state.





/// Resolve the latest committed lane authority independently of checkpoint replay.
/// This is used only to attribute current lease reconciliation and never changes the
/// replay watermark or injects a post-checkpoint lane into `ModelLaneReplay`.

/// Resolve the latest canonical lane covered by a consent receipt even when
/// the mutable `model_lanes` projection was lost. Revocation uses this to
/// cancel from EventLedger authority and then rebuild the terminal projection.

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


/// Prove the mutable ModelLane projection column against the immutable anchor
/// captured by the initial lane EventLedger event. The current lane event
/// reference can advance to terminal/status events, so this lookup deliberately
/// resolves the original `hsk.model_lane@1` event by aggregate identity instead
/// of trusting the row's latest event pointer.


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




/// Look up a single record by a field that lives inside the `record_json` JSONB
/// payload rather than as a physical column. Several ModelLane navigation
/// identifiers (`context_bundle_id`, `model_session_id`, `session_id`,
/// `memory_pack_ref`, `failstate_code`, ...) are stored only in `record_json`;
/// querying them as physical columns raises a fail-closed "column does not
/// exist" database error that surfaces to callers as a 500. Resolving through
/// the JSONB text accessor keeps a valid query from ever 500-ing.

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


fn event_payload_run_id(payload: &Value) -> Option<String> {
    payload
        .get("record")
        .and_then(|record| record.get("run_id"))
        .and_then(Value::as_str)
        .or_else(|| payload.get("run_id").and_then(Value::as_str))
        .map(str::to_owned)
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

/// Reconcile every durable message projection against the immutable
/// MODEL_RESPONSE_RECORDED event. CRDT messages retain their additional lease
/// validation, while non-CRDT and promoted messages can no longer bypass the
/// EventLedger authority check merely because they carry no CRDT references.













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




fn require_exact_lifecycle_write_scope(
    store: &ModelLaneStore,
) -> ModelLaneResult<ExactResourceScopeAttribution> {
    store.access.require_lifecycle_active()?;
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











/// Rebuild and compare a stored ContextBundle handoff against its source
/// message/lease authority and exact CONTEXT_BUNDLE_RECORDED ledger event.


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

fn decode_surreal_crdt_bytes(
    authority_ref: &str,
    encoded: &str,
    expected_sha256: &str,
) -> ModelLaneResult<Vec<u8>> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|error| {
            crdt_authority_denied(format!(
                "{authority_ref} has invalid embedded SurrealDB base64 bytes: {error}"
            ))
        })?;
    if dexterity_sha256_hex(&bytes) != expected_sha256 {
        return Err(crdt_authority_denied(format!(
            "{authority_ref} stored bytes do not match its persisted sha256"
        )));
    }
    Update::decode_v1(&bytes).map_err(|error| {
        crdt_authority_denied(format!(
            "{authority_ref} does not decode as a Yjs v1 update: {error}"
        ))
    })?;
    Ok(bytes)
}

fn validate_surreal_crdt_update_row(
    row: &SurrealModelLaneCrdtUpdate,
    scope: &SurrealModelLaneScope,
) -> ModelLaneResult<Vec<u8>> {
    if row.schema_id != CRDT_UPDATE_RECORD_SCHEMA_ID
        || row.owner_account_id != scope.owner_account_id
        || row.actor_principal_id != scope.actor_principal_id
        || row.authenticated_session_id != scope.authenticated_session_id
        || row.access_space_id != scope.access_space_id
        || row.workspace_id != scope.workspace_id
        || row.storage_authority != "embedded_surrealdb"
        || !row.update_bytes_ref.starts_with("surreal://")
        || row.update_seq <= 0
        || row.replay_encoding != "yjs-update-v1"
        || row.replay_schema_version != "kernel-crdt-update-v1"
        || row.event_ledger_stream_id != format!("knowledge-crdt:{}", row.crdt_document_id)
        || row.ledger_session_run_id != row.session_id
        || row.ledger_event_type != "KNOWLEDGE_CRDT_UPDATE_RECORDED"
        || row.ledger_aggregate_type != "knowledge_crdt_document"
        || row.ledger_aggregate_id != row.crdt_document_id
        || row.ledger_actor_id != row.actor_id
        || row.ledger_correlation_id.as_deref() != Some(row.trace_id.as_str())
        || row.ledger_update_id != row.update_id
        || row.ledger_update_seq != row.update_seq
        || row.ledger_actor_payload_id != row.actor_id
        || row.ledger_update_sha256 != row.update_sha256
        || row.ledger_state_vector_before != row.state_vector_before
        || row.ledger_state_vector_after != row.state_vector_after
        || row.ledger_payload_hash.len() != 64
    {
        return Err(crdt_authority_denied(format!(
            "crdt_update_ref {} disagrees with embedded SurrealDB/EventLedger identity",
            row.update_bytes_ref
        )));
    }
    reconcile_crdt_and_ledger_actor_kind(
        &row.actor_id,
        &row.actor_kind,
        &row.ledger_actor_kind,
        &format!("crdt_update_ref {}", row.update_bytes_ref),
    )?;
    let actor = KnowledgeActorIdV1::parse(&row.actor_id).map_err(|error| {
        crdt_authority_denied(format!(
            "crdt_update_ref {} actor_id is invalid: {error}",
            row.update_bytes_ref
        ))
    })?;
    let site = derive_knowledge_site_id(&row.workspace_id, &row.crdt_document_id, &actor);
    if site.site_id != row.ledger_site_id || row.replay_order_key.trim().is_empty() {
        return Err(crdt_authority_denied(format!(
            "crdt_update_ref {} has invalid site or replay identity",
            row.update_bytes_ref
        )));
    }
    decode_surreal_crdt_bytes(
        &row.update_bytes_ref,
        &row.update_bytes_b64,
        &row.update_sha256,
    )
}

fn validate_surreal_crdt_snapshot_row(
    row: &SurrealModelLaneCrdtSnapshot,
    scope: &SurrealModelLaneScope,
) -> ModelLaneResult<Vec<u8>> {
    if row.schema_id != CRDT_SNAPSHOT_RECORD_SCHEMA_ID
        || row.owner_account_id != scope.owner_account_id
        || row.actor_principal_id != scope.actor_principal_id
        || row.authenticated_session_id != scope.authenticated_session_id
        || row.access_space_id != scope.access_space_id
        || row.workspace_id != scope.workspace_id
        || row.storage_authority != "embedded_surrealdb"
        || !row.snapshot_bytes_ref.starts_with("surreal://")
        || row.covered_update_seq < 0
        || row.event_ledger_stream_id != format!("knowledge-crdt:{}", row.crdt_document_id)
        || row.ledger_event_type != "KNOWLEDGE_CRDT_SNAPSHOT_RECORDED"
        || row.ledger_aggregate_type != "knowledge_crdt_document"
        || row.ledger_aggregate_id != row.crdt_document_id
        || row.ledger_actor_id != row.actor_id
        || row.ledger_document_id != row.document_id
        || row.ledger_state_vector != row.state_vector
        || row.ledger_covered_update_seq != row.covered_update_seq
        || row.ledger_payload_hash.len() != 64
    {
        return Err(crdt_authority_denied(format!(
            "crdt_base_snapshot_ref {} disagrees with embedded SurrealDB/EventLedger identity",
            row.snapshot_bytes_ref
        )));
    }
    reconcile_crdt_and_ledger_actor_kind(
        &row.actor_id,
        &row.actor_kind,
        &row.ledger_actor_kind,
        &format!("crdt_base_snapshot_ref {}", row.snapshot_bytes_ref),
    )?;
    decode_surreal_crdt_bytes(
        &row.snapshot_bytes_ref,
        &row.snapshot_bytes_b64,
        &row.snapshot_sha256,
    )
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
    // Cheap pre-write CRDT posture validation. The authoritative durable
    // completeness + resolution gate is the guarded Surreal message mutation
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
