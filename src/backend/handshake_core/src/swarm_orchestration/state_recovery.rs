//! WP-KERNEL-009 MT-209..216 ParallelSwarmStateRecovery backend foundations.
//!
//! This module is intentionally backend-only. It gives local/cloud model lanes
//! typed identity, claim leases over shared worktrees/workspaces, role-mailbox
//! handoff receipts, deterministic backend navigation commands, restartable
//! compaction checkpoints, recovery receipts, and a serial lease queue for
//! parallel index writers. Embedded SurrealDB tables declared in
//! `storage/surreal/schema.surql` (`knowledge_agent_*`,
//! `knowledge_parallel_indexing_lease_queue`) are authority; EventLedger rows
//! provide the receipt trail.
//!
//! # Porting notes (PostgreSQL -> embedded SurrealDB)
//!
//! * The row-plus-event transactions where every value is known up front
//!   (claims, handoffs, checkpoints, recovery receipts, quiet-work receipts,
//!   cloud-assistance receipts, lease inserts) are single
//!   `BEGIN TRANSACTION; ...; COMMIT TRANSACTION;` statements, so the
//!   receipt row and its EventLedger receipt still land or fail together.
//!   The rows now carry their EventLedger link at CREATE time, replacing the
//!   PostgreSQL insert-then-UPDATE-event-id shape.
//! * The `SELECT ... FOR UPDATE` reclaim/release/promotion loops are guarded
//!   single-statement UPDATEs: the guard re-states the condition the row lock
//!   used to protect, so a lost race matches zero rows instead of trampling.
//!   Where the EventLedger receipt for a state transition needs data only the
//!   transition produces (lease promotion, orphan reclaim), the transition
//!   commits first and the receipt lands in a follow-up write. DISCLOSED
//!   NARROWING: a crash between the two leaves the transitioned row without
//!   its receipt link; the row state itself remains correct and
//!   TTL-recoverable.
//! * PostgreSQL partial unique indexes are stored-discriminator UNIQUE
//!   indexes in the schema (`active_scope_key`, `acquired_scope_key`); a
//!   racing writer surfaces as an index violation exactly as before, detected
//!   by index name in the store error text rather than SQLSTATE 23505.

use std::collections::{BTreeMap, BTreeSet};
#[cfg(feature = "test-utils")]
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use surrealdb::types::{RecordId, RecordIdKey, SurrealValue};
use thiserror::Error;
use uuid::Uuid;

use crate::kernel::{
    sandbox::{EnvRedactionV1, Redactor},
    KernelActor, KernelEvent, KernelEventType, NewKernelEvent,
};
use crate::storage::surreal::{SurrealStorage, SurrealStorageError};
use crate::storage::StorageError;

#[derive(Debug, Error)]
pub enum StateRecoveryError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("kernel event error: {0}")]
    Kernel(String),
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("embedded store error: {0}")]
    Surreal(#[from] SurrealStorageError),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("checkpoint not found: {0}")]
    CheckpointNotFound(String),
    #[error(
        "checkpoint payload hash mismatch for {checkpoint_id}: expected {expected}, found {found}"
    )]
    PayloadHashMismatch {
        checkpoint_id: String,
        expected: String,
        found: String,
    },
}

pub type StateRecoveryResult<T> = Result<T, StateRecoveryError>;

#[cfg(feature = "test-utils")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StateRecoveryTestFailpoint {
    ClaimAfterEventBeforeAuthority,
    ReleaseAfterAuthorityBeforeEvent,
    QuietAfterEventBeforeAuthority,
    RecoveryAfterEventBeforeAuthority,
    ReclaimAfterAuthorityBeforeEvent,
}

#[cfg(feature = "test-utils")]
#[derive(Default)]
struct StateRecoveryTestControl {
    armed: Mutex<BTreeSet<StateRecoveryTestFailpoint>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentLaneKind {
    Local,
    Cloud,
    Operator,
    Validator,
    IntegrationValidator,
    Indexer,
    Editor,
    System,
}

impl AgentLaneKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Cloud => "cloud",
            Self::Operator => "operator",
            Self::Validator => "validator",
            Self::IntegrationValidator => "integration_validator",
            Self::Indexer => "indexer",
            Self::Editor => "editor",
            Self::System => "system",
        }
    }

    fn parse(value: &str) -> StateRecoveryResult<Self> {
        match value {
            "local" => Ok(Self::Local),
            "cloud" => Ok(Self::Cloud),
            "operator" => Ok(Self::Operator),
            "validator" => Ok(Self::Validator),
            "integration_validator" => Ok(Self::IntegrationValidator),
            "indexer" => Ok(Self::Indexer),
            "editor" => Ok(Self::Editor),
            "system" => Ok(Self::System),
            other => Err(StateRecoveryError::InvalidInput(format!(
                "unknown lane kind: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentCapability {
    ClaimWorktree,
    ClaimWorkspace,
    EditRichDocument,
    InspectEvidence,
    MutateGraph,
    RunQuietBackgroundWork,
    WriteLocalIndex,
    WriteMailbox,
    NavigateBackend,
    RecordCheckpoint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelProviderKind {
    OpenAi,
    Anthropic,
    LocalRuntime,
    OfficialCli,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttributionMode {
    Local,
    Cloud,
    Operator,
    System,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocalCloudAttribution {
    pub mode: AttributionMode,
    pub provider: Option<ModelProviderKind>,
    pub runtime: Option<String>,
    pub model_label: String,
    pub credential_ref: Option<String>,
    #[serde(default)]
    pub provider_metadata: Value,
}

impl LocalCloudAttribution {
    pub fn local(runtime: impl Into<String>, model_label: impl Into<String>) -> Self {
        Self {
            mode: AttributionMode::Local,
            provider: Some(ModelProviderKind::LocalRuntime),
            runtime: Some(runtime.into()),
            model_label: model_label.into(),
            credential_ref: None,
            provider_metadata: json!({}),
        }
    }

    pub fn cloud(
        provider: ModelProviderKind,
        model_label: impl Into<String>,
        credential_ref: impl Into<String>,
        provider_metadata: Value,
    ) -> Self {
        Self {
            mode: AttributionMode::Cloud,
            provider: Some(provider),
            runtime: None,
            model_label: model_label.into(),
            credential_ref: Some(credential_ref.into()),
            provider_metadata: scrub_secret_metadata(provider_metadata),
        }
    }

    fn scrubbed_for_persistence(&self) -> Self {
        let mut scrubbed = self.clone();
        scrubbed.provider_metadata = scrub_secret_metadata(scrubbed.provider_metadata);
        scrubbed
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentLaneIdentity {
    pub lane_id: String,
    pub actor_id: String,
    pub lane_kind: AgentLaneKind,
    pub attribution: LocalCloudAttribution,
}

impl AgentLaneIdentity {
    pub fn new(
        lane_id: impl Into<String>,
        actor_id: impl Into<String>,
        lane_kind: AgentLaneKind,
        attribution: LocalCloudAttribution,
    ) -> StateRecoveryResult<Self> {
        let lane_id = lane_id.into();
        let actor_id = actor_id.into();
        ensure_safe_token("lane_id", &lane_id)?;
        ensure_safe_token("actor_id", &actor_id)?;
        Ok(Self {
            lane_id,
            actor_id,
            lane_kind,
            attribution,
        })
    }

    pub fn capabilities(&self) -> Vec<AgentCapability> {
        use AgentCapability::*;
        match self.lane_kind {
            AgentLaneKind::Local => vec![
                ClaimWorktree,
                ClaimWorkspace,
                EditRichDocument,
                MutateGraph,
                RunQuietBackgroundWork,
                WriteLocalIndex,
                WriteMailbox,
                NavigateBackend,
                RecordCheckpoint,
            ],
            AgentLaneKind::Cloud => {
                vec![
                    ClaimWorkspace,
                    RunQuietBackgroundWork,
                    WriteMailbox,
                    NavigateBackend,
                    RecordCheckpoint,
                ]
            }
            AgentLaneKind::Operator => vec![
                ClaimWorktree,
                ClaimWorkspace,
                EditRichDocument,
                MutateGraph,
                RunQuietBackgroundWork,
                WriteMailbox,
                NavigateBackend,
                RecordCheckpoint,
            ],
            AgentLaneKind::Validator | AgentLaneKind::IntegrationValidator => {
                vec![InspectEvidence, NavigateBackend]
            }
            AgentLaneKind::Indexer => {
                vec![
                    ClaimWorkspace,
                    RunQuietBackgroundWork,
                    WriteLocalIndex,
                    NavigateBackend,
                    RecordCheckpoint,
                ]
            }
            AgentLaneKind::Editor => vec![
                ClaimWorkspace,
                EditRichDocument,
                MutateGraph,
                RunQuietBackgroundWork,
                NavigateBackend,
                RecordCheckpoint,
            ],
            AgentLaneKind::System => vec![
                ClaimWorktree,
                ClaimWorkspace,
                EditRichDocument,
                MutateGraph,
                RunQuietBackgroundWork,
                WriteLocalIndex,
                WriteMailbox,
                NavigateBackend,
                RecordCheckpoint,
            ],
        }
    }

    fn scrubbed_for_persistence(&self) -> Self {
        Self {
            lane_id: self.lane_id.clone(),
            actor_id: self.actor_id.clone(),
            lane_kind: self.lane_kind,
            attribution: self.attribution.scrubbed_for_persistence(),
        }
    }

    fn to_kernel_actor(&self) -> KernelActor {
        match self.lane_kind {
            AgentLaneKind::Operator => KernelActor::Operator(self.actor_id.clone()),
            AgentLaneKind::Validator | AgentLaneKind::IntegrationValidator => {
                KernelActor::ValidationRunner(self.actor_id.clone())
            }
            AgentLaneKind::System => KernelActor::System(self.actor_id.clone()),
            AgentLaneKind::Cloud
            | AgentLaneKind::Local
            | AgentLaneKind::Indexer
            | AgentLaneKind::Editor => KernelActor::ModelAdapter(self.actor_id.clone()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ClaimScope {
    Worktree {
        worktree_id: String,
    },
    Workspace {
        workspace_id: String,
    },
    RichDocument {
        workspace_id: String,
        document_id: String,
    },
    GraphMutation {
        workspace_id: String,
        graph_id: String,
    },
    IndexRun {
        workspace_id: String,
        source_root_id: String,
    },
}

impl ClaimScope {
    fn kind_str(&self) -> &'static str {
        match self {
            Self::Worktree { .. } => "worktree",
            Self::Workspace { .. } => "workspace",
            Self::RichDocument { .. } => "rich_document",
            Self::GraphMutation { .. } => "graph_mutation",
            Self::IndexRun { .. } => "index_run",
        }
    }

    fn scope_id(&self) -> String {
        match self {
            Self::Worktree { worktree_id } => worktree_id.clone(),
            Self::Workspace { workspace_id } => workspace_id.clone(),
            Self::RichDocument {
                workspace_id,
                document_id,
            } => format!("{workspace_id}/{document_id}"),
            Self::GraphMutation {
                workspace_id,
                graph_id,
            } => format!("{workspace_id}/{graph_id}"),
            Self::IndexRun {
                workspace_id,
                source_root_id,
            } => format!("{workspace_id}/{source_root_id}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimStatus {
    Active,
    Held,
    Released,
    Reclaimed,
}

impl ClaimStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Held => "held",
            Self::Released => "released",
            Self::Reclaimed => "reclaimed",
        }
    }

    fn parse(value: &str) -> StateRecoveryResult<Self> {
        match value {
            "active" => Ok(Self::Active),
            "held" => Ok(Self::Held),
            "released" => Ok(Self::Released),
            "reclaimed" => Ok(Self::Reclaimed),
            other => Err(StateRecoveryError::InvalidInput(format!(
                "unknown claim status: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone)]
pub struct WorkClaimRequest {
    pub workspace_id: String,
    pub wp_id: String,
    pub mt_id: Option<String>,
    pub scope: ClaimScope,
    pub lane: AgentLaneIdentity,
    pub session_id: String,
    pub ttl_seconds: i64,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkClaimRecord {
    pub claim_id: String,
    pub workspace_id: String,
    pub wp_id: String,
    pub mt_id: Option<String>,
    pub scope: ClaimScope,
    pub lane: AgentLaneIdentity,
    pub session_id: String,
    pub status: ClaimStatus,
    pub reason: String,
    pub claimed_at_utc: DateTime<Utc>,
    pub expires_at_utc: DateTime<Utc>,
    pub released_at_utc: Option<DateTime<Utc>>,
    pub event_ledger_event_id: Option<String>,
    pub release_event_ledger_event_id: Option<String>,
    pub reclaim_event_ledger_event_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkClaimOutcome {
    pub status: ClaimStatus,
    pub claim_id: String,
    pub active_holder: Option<AgentLaneIdentity>,
    pub event_ledger_event_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SwarmEvidenceInspectionRequest {
    pub lane: AgentLaneIdentity,
    pub workspace_id: String,
    pub limit: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SwarmEvidenceInspectionSnapshot {
    pub workspace_id: String,
    pub claims: Vec<WorkClaimRecord>,
    pub mailbox_handoffs: Vec<RoleMailboxHandoffRecord>,
    pub checkpoints: Vec<RecoveryCheckpointRecord>,
    pub recovery_receipts: Vec<RecoveryReceiptRecord>,
    pub indexing_leases: Vec<IndexingLeaseRecord>,
    pub quiet_background_work: Vec<QuietBackgroundWorkRecord>,
}

pub const PARALLEL_SWARM_DASHBOARD_SCHEMA_ID: &str = "hsk.parallel_swarm.dashboard_projection@1";

const PARALLEL_SWARM_SOURCE_COMPONENT: &str = "parallel_swarm_state_recovery";

const PARALLEL_SWARM_DASHBOARD_SOURCE_TABLES: &[&str] = &[
    "knowledge_agent_worktree_claims",
    "knowledge_agent_role_mailbox_handoffs",
    "knowledge_agent_state_recovery_checkpoints",
    "knowledge_agent_recovery_receipts",
    "knowledge_parallel_indexing_lease_queue",
    "knowledge_agent_quiet_background_work",
    "kernel_event_ledger",
];

const PARALLEL_SWARM_DASHBOARD_EVENT_AGGREGATES: &[&str] = &[
    "parallel_swarm_claim",
    "parallel_swarm_claim_reclaim",
    "parallel_swarm_handoff",
    "parallel_swarm_checkpoint",
    "parallel_swarm_recovery",
    "parallel_indexing_lease",
    "parallel_swarm_quiet_background_work",
];

#[derive(Debug, Clone)]
pub struct SwarmDashboardProjectionRequest {
    pub lane: AgentLaneIdentity,
    pub workspace_id: String,
    pub wp_id: Option<String>,
    pub mt_id: Option<String>,
    pub limit: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParallelSwarmDashboardProjectionV1 {
    pub schema_id: String,
    pub workspace_id: String,
    pub generated_at_utc: DateTime<Utc>,
    pub filters: SwarmDashboardProjectionFilters,
    pub projection_contract: SwarmDashboardProjectionContractV1,
    pub source_watermark: SwarmDashboardSourceWatermarkV1,
    pub totals: SwarmDashboardTotalsV1,
    pub lanes: Vec<SwarmDashboardLaneRowV1>,
    pub claims: Vec<SwarmDashboardClaimRowV1>,
    pub mailbox_handoffs: Vec<SwarmDashboardHandoffRowV1>,
    pub recovery_checkpoints: Vec<SwarmDashboardCheckpointRowV1>,
    pub recovery_receipts: Vec<SwarmDashboardRecoveryReceiptRowV1>,
    pub indexing_leases: Vec<SwarmDashboardIndexingLeaseRowV1>,
    pub quiet_background_work: Vec<SwarmDashboardQuietWorkRowV1>,
    pub warnings: Vec<SwarmDashboardWarningV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwarmDashboardProjectionFilters {
    pub workspace_id: String,
    pub wp_id: Option<String>,
    pub mt_id: Option<String>,
    pub limit: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwarmDashboardProjectionContractV1 {
    pub projection_only: bool,
    pub authority_mutation_allowed: bool,
    pub ui_state_authoritative: bool,
    pub source_component: String,
    pub source_tables: Vec<String>,
    pub source_event_aggregates: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwarmDashboardSourceWatermarkV1 {
    pub source_component: String,
    pub event_count: i64,
    pub max_event_created_at_utc: Option<DateTime<Utc>>,
    pub events: Vec<SwarmDashboardEventRefV1>,
    pub aggregate_counts: Vec<SwarmDashboardAggregateCountV1>,
    pub missing_event_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwarmDashboardEventRefV1 {
    pub event_id: String,
    pub source_component: String,
    pub aggregate_type: String,
    pub aggregate_id: String,
    pub created_at_utc: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwarmDashboardAggregateCountV1 {
    pub aggregate_type: String,
    pub count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwarmDashboardTotalsV1 {
    pub claims: i64,
    pub active_claims: i64,
    pub stale_active_claims: i64,
    pub mailbox_handoffs: i64,
    pub recovery_checkpoints: i64,
    pub recovery_receipts: i64,
    pub indexing_leases: i64,
    pub acquired_indexing_leases: i64,
    pub quiet_background_work: i64,
    pub events: i64,
    pub warnings: i64,
    pub claims_by_status: BTreeMap<String, i64>,
    pub handoffs_by_status: BTreeMap<String, i64>,
    pub leases_by_status: BTreeMap<String, i64>,
    pub quiet_work_by_kind: BTreeMap<String, i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwarmDashboardLaneRowV1 {
    pub lane_id: String,
    pub actor_id: String,
    pub lane_kind: String,
    pub attribution_mode: String,
    pub total_rows: i64,
    pub active_claims: i64,
    pub handoffs: i64,
    pub checkpoints: i64,
    pub recovery_receipts: i64,
    pub indexing_leases: i64,
    pub quiet_background_work: i64,
    pub source_event_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwarmDashboardSourceRefV1 {
    pub table_name: String,
    pub row_id: String,
    pub row_source_ref: String,
    pub event_ledger_event_id: Option<String>,
    pub event_source_ref: Option<String>,
    pub event_aggregate_type: Option<String>,
    pub event_aggregate_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SwarmDashboardClaimRowV1 {
    pub claim_id: String,
    pub wp_id: String,
    pub mt_id: Option<String>,
    pub scope_kind: String,
    pub scope_id: String,
    pub lane_id: String,
    pub actor_id: String,
    pub lane_kind: String,
    pub status: String,
    pub reason: String,
    pub claimed_at_utc: DateTime<Utc>,
    pub expires_at_utc: DateTime<Utc>,
    pub released_at_utc: Option<DateTime<Utc>>,
    pub stale: bool,
    pub source_refs: Vec<SwarmDashboardSourceRefV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SwarmDashboardHandoffRowV1 {
    pub handoff_id: String,
    pub wp_id: String,
    pub mt_id: String,
    pub claim_id: Option<String>,
    pub from_lane_id: String,
    pub from_actor_id: String,
    pub from_lane_kind: String,
    pub to_role: String,
    pub mailbox_thread_id: String,
    pub mailbox_message_id: String,
    pub status: String,
    pub summary: String,
    pub created_at_utc: DateTime<Utc>,
    pub source_refs: Vec<SwarmDashboardSourceRefV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SwarmDashboardCheckpointRowV1 {
    pub checkpoint_id: String,
    pub wp_id: String,
    pub mt_id: String,
    pub session_id: String,
    pub lane_id: String,
    pub actor_id: String,
    pub lane_kind: String,
    pub claim_id: Option<String>,
    pub mailbox_handoff_id: Option<String>,
    pub navigation_command_id: Option<String>,
    pub resume_pointer: RecoveryResumePointer,
    pub payload_sha256: String,
    pub compaction_reason: String,
    pub git_head: String,
    pub created_at_utc: DateTime<Utc>,
    pub source_refs: Vec<SwarmDashboardSourceRefV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SwarmDashboardRecoveryReceiptRowV1 {
    pub receipt_id: String,
    pub checkpoint_id: String,
    pub prior_session_id: String,
    pub new_session_id: String,
    pub new_lane_id: String,
    pub new_actor_id: String,
    pub new_lane_kind: String,
    pub resume_pointer: RecoveryResumePointer,
    pub recovered_at_utc: DateTime<Utc>,
    pub source_refs: Vec<SwarmDashboardSourceRefV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SwarmDashboardIndexingLeaseRowV1 {
    pub lease_id: String,
    pub wp_id: String,
    pub mt_id: String,
    pub scope_kind: String,
    pub scope_id: String,
    pub lane_id: String,
    pub actor_id: String,
    pub lane_kind: String,
    pub session_id: String,
    pub index_run_id: String,
    pub status: String,
    pub blocked_by_lease_id: Option<String>,
    pub quiet_policy_ok: bool,
    pub source_refs: Vec<SwarmDashboardSourceRefV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SwarmDashboardQuietWorkRowV1 {
    pub receipt_id: String,
    pub wp_id: String,
    pub mt_id: String,
    pub work_kind: String,
    pub subject_id: String,
    pub lane_id: String,
    pub actor_id: String,
    pub lane_kind: String,
    pub session_id: String,
    pub evidence_ref: String,
    pub quiet_policy_ok: bool,
    pub created_at_utc: DateTime<Utc>,
    pub source_refs: Vec<SwarmDashboardSourceRefV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwarmDashboardWarningV1 {
    pub code: String,
    pub detail: String,
}

pub const PARALLEL_SWARM_CLOUD_ASSISTANCE_SCHEMA_ID: &str = "hsk.parallel_swarm.cloud_assistance@1";
pub const PARALLEL_SWARM_CLOUD_FALLBACK_BASIS_SCHEMA_ID: &str =
    "hsk.parallel_swarm.cloud_fallback_basis@1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CloudFallbackReason {
    LocalFailed,
    LocalLowConfidence,
    LocalOverloaded,
    LocalSuppressed,
    HardReasoning,
    ForceCloud,
    NoLocalModel,
}

impl CloudFallbackReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::LocalFailed => "local_failed",
            Self::LocalLowConfidence => "local_low_confidence",
            Self::LocalOverloaded => "local_overloaded",
            Self::LocalSuppressed => "local_suppressed",
            Self::HardReasoning => "hard_reasoning",
            Self::ForceCloud => "force_cloud",
            Self::NoLocalModel => "no_local_model",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CloudAssistanceOutputKind {
    Analysis,
    PatchSuggestion,
    ValidationSummary,
    HandoffSummary,
}

impl CloudAssistanceOutputKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Analysis => "analysis",
            Self::PatchSuggestion => "patch_suggestion",
            Self::ValidationSummary => "validation_summary",
            Self::HandoffSummary => "handoff_summary",
        }
    }
}

#[derive(Debug, Clone)]
pub struct CloudAssistanceRequest {
    pub from_lane: AgentLaneIdentity,
    pub workspace_id: String,
    pub wp_id: String,
    pub mt_id: String,
    pub claim_id: String,
    pub session_id: String,
    pub to_role: String,
    pub mailbox_thread_id: String,
    pub mailbox_message_id: String,
    pub fallback_basis_event_id: String,
    pub parent_session_id: String,
    pub prompt_sha256: String,
    pub fallback_reason: CloudFallbackReason,
    pub output_kind: CloudAssistanceOutputKind,
    pub output_sha256: String,
    pub body_sha256: String,
    pub output_text: String,
    pub output_body_jsonb: serde_json::Value,
    pub summary: String,
    pub target_ref: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CloudAssistanceReceiptV1 {
    pub schema_id: String,
    pub receipt_id: String,
    pub workspace_id: String,
    pub wp_id: String,
    pub mt_id: String,
    pub claim_id: String,
    pub handoff_id: String,
    pub handoff_event_ledger_event_id: String,
    pub cloud_assistance_event_id: String,
    pub fallback_basis_event_id: String,
    pub parent_session_id: String,
    pub prompt_sha256: String,
    pub lane_id: String,
    pub actor_id: String,
    pub provider: Option<ModelProviderKind>,
    pub model_label: String,
    pub fallback_reason: CloudFallbackReason,
    pub output_kind: CloudAssistanceOutputKind,
    pub output_sha256: String,
    pub body_sha256: String,
    pub output_text: String,
    pub target_ref: String,
    pub review_state: String,
    pub non_authoritative: bool,
    pub requires_promotion: bool,
    pub authority_mutation_allowed: bool,
    pub promotion_event_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CloudFallbackBasisRequest {
    pub lane: AgentLaneIdentity,
    pub workspace_id: String,
    pub wp_id: String,
    pub mt_id: String,
    pub claim_id: String,
    pub parent_session_id: String,
    pub prompt_sha256: String,
    pub session_id: String,
    pub fallback_reason: CloudFallbackReason,
    pub local_attempt_ref: String,
    pub evidence_sha256: String,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CloudFallbackBasisReceiptV1 {
    pub schema_id: String,
    pub basis_id: String,
    pub fallback_basis_event_id: String,
    pub workspace_id: String,
    pub wp_id: String,
    pub mt_id: String,
    pub claim_id: String,
    pub parent_session_id: String,
    pub prompt_sha256: String,
    pub lane_id: String,
    pub actor_id: String,
    pub fallback_reason: CloudFallbackReason,
    pub local_attempt_ref: String,
    pub evidence_sha256: String,
}

#[derive(Debug, Clone, Default)]
struct SwarmDashboardAuthorityTotals {
    claims: i64,
    active_claims: i64,
    stale_active_claims: i64,
    mailbox_handoffs: i64,
    recovery_checkpoints: i64,
    recovery_receipts: i64,
    indexing_leases: i64,
    acquired_indexing_leases: i64,
    quiet_background_work: i64,
    events: i64,
    claims_by_status: BTreeMap<String, i64>,
    handoffs_by_status: BTreeMap<String, i64>,
    leases_by_status: BTreeMap<String, i64>,
    quiet_work_by_kind: BTreeMap<String, i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuietBackgroundWorkKind {
    Indexing,
    BackendNavigation,
    VisualCapture,
    TestRun,
}

impl QuietBackgroundWorkKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Indexing => "indexing",
            Self::BackendNavigation => "backend_navigation",
            Self::VisualCapture => "visual_capture",
            Self::TestRun => "test_run",
        }
    }

    fn parse(value: &str) -> StateRecoveryResult<Self> {
        match value {
            "indexing" => Ok(Self::Indexing),
            "backend_navigation" => Ok(Self::BackendNavigation),
            "visual_capture" => Ok(Self::VisualCapture),
            "test_run" => Ok(Self::TestRun),
            other => Err(StateRecoveryError::InvalidInput(format!(
                "unknown quiet background work kind: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuietBackgroundPolicy {
    pub work_kind: QuietBackgroundWorkKind,
    pub no_foreground_window: bool,
    pub no_focus_steal: bool,
    pub no_os_shell_window: bool,
    pub bounded: bool,
    pub observable: bool,
}

impl QuietBackgroundPolicy {
    pub fn quiet_for(work_kind: QuietBackgroundWorkKind) -> Self {
        Self {
            work_kind,
            no_foreground_window: true,
            no_focus_steal: true,
            no_os_shell_window: true,
            bounded: true,
            observable: true,
        }
    }

    pub fn all_quiet(&self) -> bool {
        self.no_foreground_window
            && self.no_focus_steal
            && self.no_os_shell_window
            && self.bounded
            && self.observable
    }
}

#[derive(Debug, Clone)]
pub struct QuietBackgroundWorkRequest {
    pub lane: AgentLaneIdentity,
    pub workspace_id: String,
    pub wp_id: String,
    pub mt_id: String,
    pub work_kind: QuietBackgroundWorkKind,
    pub subject_id: String,
    pub session_id: String,
    pub policy: QuietBackgroundPolicy,
    pub evidence_ref: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuietBackgroundWorkRecord {
    pub receipt_id: String,
    pub workspace_id: String,
    pub wp_id: String,
    pub mt_id: String,
    pub work_kind: QuietBackgroundWorkKind,
    pub subject_id: String,
    pub lane: AgentLaneIdentity,
    pub session_id: String,
    pub policy: QuietBackgroundPolicy,
    pub evidence_ref: String,
    pub event_ledger_event_id: String,
    pub created_at_utc: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SwarmReceiptStatus {
    Started,
    Progress,
    Blocked,
    Pass,
    Fail,
}

impl SwarmReceiptStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Started => "started",
            Self::Progress => "progress",
            Self::Blocked => "blocked",
            Self::Pass => "pass",
            Self::Fail => "fail",
        }
    }

    fn parse(value: &str) -> StateRecoveryResult<Self> {
        match value {
            "started" => Ok(Self::Started),
            "progress" => Ok(Self::Progress),
            "blocked" => Ok(Self::Blocked),
            "pass" => Ok(Self::Pass),
            "fail" => Ok(Self::Fail),
            other => Err(StateRecoveryError::InvalidInput(format!(
                "unknown swarm receipt status: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RoleMailboxHandoffRequest {
    pub from_lane: AgentLaneIdentity,
    pub to_role: String,
    pub wp_id: String,
    pub mt_id: String,
    pub claim_id: Option<String>,
    pub mailbox_thread_id: String,
    pub mailbox_message_id: String,
    pub status: SwarmReceiptStatus,
    pub summary: String,
    pub body_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoleMailboxHandoffRecord {
    pub handoff_id: String,
    pub wp_id: String,
    pub mt_id: String,
    pub claim_id: Option<String>,
    pub from_lane: AgentLaneIdentity,
    pub to_role: String,
    pub mailbox_thread_id: String,
    pub mailbox_message_id: String,
    pub status: SwarmReceiptStatus,
    pub summary: String,
    pub body_sha256: String,
    pub event_ledger_event_id: String,
    pub created_at_utc: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackendNavigationCommand {
    Sources,
    Symbols,
    Docs,
    Graph,
    RetrievalTraces,
    UserManualPages,
    RepairQueue,
    ValidationState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NavigationCommandSpec {
    pub command: BackendNavigationCommand,
    pub command_id: &'static str,
    pub route: &'static str,
    pub required_params: &'static [&'static str],
}

impl NavigationCommandSpec {
    pub fn quiet_policy(&self) -> QuietBackgroundPolicy {
        QuietBackgroundPolicy::quiet_for(QuietBackgroundWorkKind::BackendNavigation)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ResolvedNavigationCommand {
    pub command: BackendNavigationCommand,
    pub command_id: &'static str,
    pub route: &'static str,
    pub params: Value,
    pub deterministic_cache_key: String,
    pub quiet_policy: QuietBackgroundPolicy,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct QuietResolvedNavigationCommand {
    pub resolved: ResolvedNavigationCommand,
    pub quiet_receipt: QuietBackgroundWorkRecord,
}

#[derive(Debug, Default, Clone)]
pub struct NavigationCommandSet;

impl NavigationCommandSet {
    pub fn commands(&self) -> &'static [NavigationCommandSpec] {
        NAV_COMMANDS
    }

    pub fn resolve(
        &self,
        command: BackendNavigationCommand,
        params: Value,
    ) -> StateRecoveryResult<ResolvedNavigationCommand> {
        let spec = NAV_COMMANDS
            .iter()
            .find(|spec| spec.command == command)
            .ok_or_else(|| StateRecoveryError::InvalidInput("unknown command".to_string()))?;
        let map = params.as_object().ok_or_else(|| {
            StateRecoveryError::InvalidInput("navigation params must be an object".to_string())
        })?;
        for required in spec.required_params {
            if !map.contains_key(*required) {
                return Err(StateRecoveryError::InvalidInput(format!(
                    "navigation command {} requires param {}",
                    spec.command_id, required
                )));
            }
        }
        let canonical = canonical_json(&params);
        let key_hash = sha256_hex(format!("{}:{canonical}", spec.command_id).as_bytes());
        Ok(ResolvedNavigationCommand {
            command,
            command_id: spec.command_id,
            route: spec.route,
            params,
            deterministic_cache_key: format!("NAV-{key_hash}"),
            quiet_policy: spec.quiet_policy(),
        })
    }
}

const NAV_COMMANDS: &[NavigationCommandSpec] = &[
    NavigationCommandSpec {
        command: BackendNavigationCommand::Sources,
        command_id: "sources",
        route: "/knowledge/sources",
        required_params: &["workspace_id"],
    },
    NavigationCommandSpec {
        command: BackendNavigationCommand::Symbols,
        command_id: "symbols",
        route: "/knowledge/code/symbols",
        required_params: &["workspace_id"],
    },
    NavigationCommandSpec {
        command: BackendNavigationCommand::Docs,
        command_id: "docs",
        route: "/knowledge/documents",
        required_params: &["workspace_id"],
    },
    NavigationCommandSpec {
        command: BackendNavigationCommand::Graph,
        command_id: "graph",
        route: "/knowledge/graph",
        required_params: &["workspace_id"],
    },
    NavigationCommandSpec {
        command: BackendNavigationCommand::RetrievalTraces,
        command_id: "retrieval_traces",
        route: "/knowledge/retrieval/traces",
        required_params: &["workspace_id"],
    },
    NavigationCommandSpec {
        command: BackendNavigationCommand::UserManualPages,
        command_id: "user_manual_pages",
        route: "/user_manual/pages",
        required_params: &["workspace_id"],
    },
    NavigationCommandSpec {
        command: BackendNavigationCommand::RepairQueue,
        command_id: "repair_queue",
        route: "/knowledge/repair_queue",
        required_params: &["workspace_id"],
    },
    NavigationCommandSpec {
        command: BackendNavigationCommand::ValidationState,
        command_id: "validation_state",
        route: "/kernel/validation/state",
        required_params: &["workspace_id"],
    },
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "pointer", rename_all = "snake_case")]
pub enum RecoveryResumePointer {
    MicroTask {
        mt_id: String,
    },
    Claim {
        claim_id: String,
    },
    Navigation {
        command_id: String,
    },
    IndexRunPosition {
        index_run_id: String,
        position: String,
    },
}

#[derive(Debug, Clone)]
pub struct RecoveryCheckpointRequest {
    pub lane: AgentLaneIdentity,
    pub session_id: String,
    pub workspace_id: String,
    pub wp_id: String,
    pub mt_id: String,
    pub claim_id: Option<String>,
    pub mailbox_handoff_id: Option<String>,
    pub navigation_command_id: Option<String>,
    pub resume_pointer: RecoveryResumePointer,
    pub touched_files: Vec<String>,
    pub tests: Vec<String>,
    pub hbr_rows: Vec<String>,
    pub next_step_context: String,
    pub payload: Value,
    pub compaction_reason: String,
    pub git_head: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecoveryCheckpointRecord {
    pub checkpoint_id: String,
    pub lane: AgentLaneIdentity,
    pub session_id: String,
    pub workspace_id: String,
    pub wp_id: String,
    pub mt_id: String,
    pub claim_id: Option<String>,
    pub mailbox_handoff_id: Option<String>,
    pub navigation_command_id: Option<String>,
    pub resume_pointer: RecoveryResumePointer,
    pub touched_files: Vec<String>,
    pub tests: Vec<String>,
    pub hbr_rows: Vec<String>,
    pub next_step_context: String,
    pub payload: Value,
    pub payload_sha256: String,
    pub compaction_reason: String,
    pub git_head: String,
    pub event_ledger_event_id: String,
    pub created_at_utc: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecoveryReceiptRecord {
    pub receipt_id: String,
    pub checkpoint_id: String,
    pub prior_session_id: String,
    pub new_session_id: String,
    pub new_lane: AgentLaneIdentity,
    pub resume_pointer: RecoveryResumePointer,
    pub event_ledger_event_id: String,
    pub recovered_at_utc: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecoveredCheckpoint {
    pub checkpoint: RecoveryCheckpointRecord,
    pub receipt: RecoveryReceiptRecord,
    pub resume_pointer: RecoveryResumePointer,
}

pub const PARALLEL_SWARM_HANDOFF_COMPRESSION_SCHEMA_ID: &str =
    "hsk.parallel_swarm.handoff_compression@1";

#[derive(Debug, Clone)]
pub struct HandoffCompressionRequest {
    pub requested_by_lane: AgentLaneIdentity,
    pub checkpoint_id: String,
    pub max_chars: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandoffCompressionSourceRefV1 {
    pub source_kind: String,
    pub source_id: String,
    pub event_ledger_event_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HandoffCompressionTemplateV1 {
    pub schema_id: String,
    pub checkpoint_id: String,
    pub workspace_id: String,
    pub wp_id: String,
    pub mt_id: String,
    pub source_session_id: String,
    pub source_lane_id: String,
    pub source_actor_id: String,
    pub source_lane_kind: String,
    pub resume_pointer: RecoveryResumePointer,
    pub git_head: String,
    pub payload_sha256: String,
    pub body_sha256: String,
    pub body: String,
    pub omitted_inputs: Vec<String>,
    pub source_refs: Vec<HandoffCompressionSourceRefV1>,
    pub warnings: Vec<String>,
    pub generated_at_utc: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IndexLeaseStatus {
    Queued,
    Acquired,
    Completed,
    Cancelled,
    Reclaimed,
}

impl IndexLeaseStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Acquired => "acquired",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
            Self::Reclaimed => "reclaimed",
        }
    }

    fn parse(value: &str) -> StateRecoveryResult<Self> {
        match value {
            "queued" => Ok(Self::Queued),
            "acquired" => Ok(Self::Acquired),
            "completed" => Ok(Self::Completed),
            "cancelled" => Ok(Self::Cancelled),
            "reclaimed" => Ok(Self::Reclaimed),
            other => Err(StateRecoveryError::InvalidInput(format!(
                "unknown index lease status: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone)]
pub struct IndexingLeaseRequest {
    pub workspace_id: String,
    pub wp_id: String,
    pub mt_id: String,
    pub scope: ClaimScope,
    pub lane: AgentLaneIdentity,
    pub session_id: String,
    pub index_run_id: String,
    pub priority: i32,
    pub ttl_seconds: i64,
    pub quiet_policy: QuietBackgroundPolicy,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexingLeaseRecord {
    pub lease_id: String,
    pub workspace_id: String,
    pub wp_id: String,
    pub mt_id: String,
    pub scope: ClaimScope,
    pub lane: AgentLaneIdentity,
    pub session_id: String,
    pub index_run_id: String,
    pub priority: i32,
    pub ttl_seconds: i64,
    pub status: IndexLeaseStatus,
    pub blocked_by_lease_id: Option<String>,
    pub quiet_policy: QuietBackgroundPolicy,
    pub event_ledger_event_id: String,
}

const CLAIMS_TABLE: &str = "knowledge_agent_worktree_claims";
const HANDOFFS_TABLE: &str = "knowledge_agent_role_mailbox_handoffs";
const CHECKPOINTS_TABLE: &str = "knowledge_agent_state_recovery_checkpoints";
const RECEIPTS_TABLE: &str = "knowledge_agent_recovery_receipts";
const LEASES_TABLE: &str = "knowledge_parallel_indexing_lease_queue";
const QUIET_TABLE: &str = "knowledge_agent_quiet_background_work";
const CLOUD_RECEIPTS_TABLE: &str = "knowledge_agent_cloud_assistance_receipts";
const EVENT_LEDGER_TABLE: &str = "kernel_event_ledger";

/// One `kernel_event_ledger` receipt row; mirrors the `SCHEMAFULL` table and
/// the write shape established by `flight_recorder::fr_emitter`.
#[derive(SurrealValue)]
struct EventLedgerWriteRow {
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
    created_at: DateTime<Utc>,
}

/// One `knowledge_agent_worktree_claims` record. EventLedger links are record
/// links; the schema's computed `active_scope_key` is deliberately absent
/// (reads tolerate it because unmodelled fields are ignored).
#[derive(SurrealValue)]
struct ClaimRow {
    claim_id: String,
    workspace_id: String,
    wp_id: String,
    mt_id: Option<String>,
    scope_kind: String,
    scope_id: String,
    lane_id: String,
    actor_id: String,
    lane_kind: String,
    attribution_jsonb: Value,
    session_id: String,
    status: String,
    reason: String,
    claimed_at_utc: DateTime<Utc>,
    expires_at_utc: DateTime<Utc>,
    released_at_utc: Option<DateTime<Utc>>,
    event_ledger_event_id: Option<RecordId>,
    release_event_ledger_event_id: Option<RecordId>,
    reclaim_event_ledger_event_id: Option<RecordId>,
}

/// One `knowledge_agent_role_mailbox_handoffs` record.
#[derive(SurrealValue)]
struct HandoffRow {
    handoff_id: String,
    wp_id: String,
    mt_id: String,
    claim_id: Option<RecordId>,
    from_lane_id: String,
    from_actor_id: String,
    from_lane_kind: String,
    from_attribution_jsonb: Value,
    to_role: String,
    mailbox_thread_id: String,
    mailbox_message_id: String,
    status: String,
    summary: String,
    body_sha256: String,
    event_ledger_event_id: RecordId,
    created_at_utc: DateTime<Utc>,
}

/// One `knowledge_agent_state_recovery_checkpoints` record.
#[derive(SurrealValue)]
struct CheckpointRow {
    checkpoint_id: String,
    lane_id: String,
    actor_id: String,
    lane_kind: String,
    attribution_jsonb: Value,
    session_id: String,
    workspace_id: String,
    wp_id: String,
    mt_id: String,
    claim_id: Option<RecordId>,
    mailbox_handoff_id: Option<RecordId>,
    navigation_command_id: Option<String>,
    resume_pointer_jsonb: Value,
    touched_files_jsonb: Vec<String>,
    tests_jsonb: Vec<String>,
    hbr_rows_jsonb: Vec<String>,
    next_step_context: String,
    payload_jsonb: Value,
    payload_sha256: String,
    compaction_reason: String,
    git_head: String,
    event_ledger_event_id: RecordId,
    created_at_utc: DateTime<Utc>,
}

/// One `knowledge_agent_recovery_receipts` record.
#[derive(SurrealValue)]
struct RecoveryReceiptRow {
    receipt_id: String,
    checkpoint_id: RecordId,
    prior_session_id: String,
    new_session_id: String,
    new_lane_id: String,
    new_actor_id: String,
    new_lane_kind: String,
    new_attribution_jsonb: Value,
    resume_pointer_jsonb: Value,
    event_ledger_event_id: RecordId,
    recovered_at_utc: DateTime<Utc>,
}

/// One `knowledge_parallel_indexing_lease_queue` record; the computed
/// `acquired_scope_key` is deliberately absent.
#[derive(Clone, SurrealValue)]
struct LeaseRow {
    lease_id: String,
    workspace_id: String,
    wp_id: String,
    mt_id: String,
    scope_kind: String,
    scope_id: String,
    lane_id: String,
    actor_id: String,
    lane_kind: String,
    attribution_jsonb: Value,
    session_id: String,
    index_run_id: String,
    priority: i64,
    ttl_seconds: i64,
    status: String,
    blocked_by_lease_id: Option<String>,
    enqueued_at_utc: DateTime<Utc>,
    acquired_at_utc: Option<DateTime<Utc>>,
    expires_at_utc: Option<DateTime<Utc>>,
    completed_at_utc: Option<DateTime<Utc>>,
    event_ledger_event_id: Option<RecordId>,
    quiet_policy_jsonb: Value,
}

/// One `knowledge_agent_quiet_background_work` record.
#[derive(SurrealValue)]
struct QuietWorkRow {
    receipt_id: String,
    workspace_id: String,
    wp_id: String,
    mt_id: String,
    work_kind: String,
    subject_id: String,
    lane_id: String,
    actor_id: String,
    lane_kind: String,
    attribution_jsonb: Value,
    session_id: String,
    quiet_policy_jsonb: Value,
    evidence_ref: String,
    event_ledger_event_id: RecordId,
    created_at_utc: DateTime<Utc>,
}

/// Bindings for the canonical "receipt row plus its EventLedger receipt in
/// one transaction" write.
#[derive(SurrealValue)]
struct CreateRowWithEventBindings {
    event_record: RecordId,
    event_content: surrealdb::types::Value,
    record: RecordId,
    content: surrealdb::types::Value,
}

#[derive(SurrealValue)]
struct WorkspaceBinding {
    workspace_id: String,
}

#[derive(SurrealValue)]
struct WorkspaceLimitBindings {
    workspace_id: String,
    limit: i64,
}

#[derive(SurrealValue)]
struct DashboardFilterBindings {
    workspace_id: String,
    wp_id: Option<String>,
    mt_id: Option<String>,
    limit: i64,
}

#[derive(SurrealValue)]
struct TotalsFilterBindings {
    workspace_id: String,
    wp_id: Option<String>,
    mt_id: Option<String>,
}

#[derive(SurrealValue)]
struct EventLookupBindings {
    source_component: String,
    event_ids: Vec<String>,
}

#[derive(SurrealValue)]
struct GroupCountRow {
    group_key: String,
    row_count: i64,
}

#[derive(SurrealValue)]
struct CheckpointIdBinding {
    checkpoint_id: String,
}

#[derive(SurrealValue)]
struct ScopeBinding {
    scope_kind: String,
    scope_id: String,
}

#[derive(SurrealValue)]
struct RecordActorBinding {
    record: RecordId,
    actor_id: String,
    completed_at_utc: DateTime<Utc>,
}

#[derive(SurrealValue)]
struct AcquireLeaseBinding {
    record: RecordId,
    scope_kind: String,
    scope_id: String,
    acquired_at_utc: DateTime<Utc>,
    expires_at_utc: DateTime<Utc>,
}

#[derive(SurrealValue)]
struct RecordEventBinding {
    record: RecordId,
    event_record: RecordId,
}

#[derive(SurrealValue)]
struct CreateEventBinding {
    event_record: RecordId,
    event_content: surrealdb::types::Value,
}

#[derive(SurrealValue)]
struct ClaimActorBindings {
    claim_id: String,
    actor_id: String,
}

#[derive(SurrealValue)]
struct ReleaseClaimBindings {
    claim: RecordId,
    actor_id: String,
    reason: String,
    event_record: RecordId,
    event_content: surrealdb::types::Value,
}

#[derive(SurrealValue)]
struct EmptyBindings {}

#[derive(SurrealValue)]
struct ExpiredRecordBinding {
    record: RecordId,
    completed_at_utc: DateTime<Utc>,
}

#[derive(SurrealValue)]
struct ReclaimClaimBinding {
    record: RecordId,
    released_at_utc: DateTime<Utc>,
    reason: String,
    event_record: RecordId,
    event_content: surrealdb::types::Value,
}

#[cfg(feature = "test-utils")]
#[derive(SurrealValue)]
struct CorruptCheckpointPayloadBinding {
    record: RecordId,
    payload: Value,
}

fn scope_binding(scope: &ClaimScope) -> ScopeBinding {
    ScopeBinding {
        scope_kind: scope.kind_str().to_string(),
        scope_id: scope.scope_id(),
    }
}

const CREATE_ROW_WITH_EVENT_QUERY: &str = "BEGIN TRANSACTION; \
     CREATE $event_record CONTENT $event_content; \
     CREATE $record CONTENT $content; \
     COMMIT TRANSACTION;";

#[cfg(feature = "test-utils")]
const CREATE_ROW_WITH_EVENT_FAIL_AFTER_EVENT_QUERY: &str = "BEGIN TRANSACTION; \
     CREATE $event_record CONTENT $event_content; \
     THROW 'HSK-TEST-FAIL-AFTER-EVENT'; \
     CREATE $record CONTENT $content; \
     COMMIT TRANSACTION;";

const RELEASE_CLAIM_QUERY: &str = "BEGIN TRANSACTION; \
     IF count((SELECT VALUE id FROM $claim \
         WHERE actor_id = $actor_id AND status = 'active' \
           AND released_at_utc = NONE)) > 0 { \
         CREATE $event_record CONTENT $event_content; \
         UPDATE $claim SET status = 'released', \
             released_at_utc = time::now(), reason = $reason, \
             release_event_ledger_event_id = $event_record; \
         RETURN true; \
     } ELSE { RETURN NONE; }; \
     COMMIT TRANSACTION;";

#[cfg(feature = "test-utils")]
const RELEASE_CLAIM_FAIL_BEFORE_EVENT_QUERY: &str = "BEGIN TRANSACTION; \
     IF count((SELECT VALUE id FROM $claim \
         WHERE actor_id = $actor_id AND status = 'active' \
           AND released_at_utc = NONE)) > 0 { \
         UPDATE $claim SET status = 'released', released_at_utc = time::now(), \
             reason = $reason; \
         THROW 'HSK-TEST-FAIL-RELEASE-EVENT'; \
         CREATE $event_record CONTENT $event_content; \
         RETURN true; \
     } ELSE { RETURN NONE; }; \
     COMMIT TRANSACTION;";

const RECLAIM_CLAIM_QUERY: &str = "BEGIN TRANSACTION; \
     LET $changed = (UPDATE $record SET status = 'reclaimed', \
         released_at_utc = $released_at_utc, reason = $reason \
         WHERE status = 'active' AND released_at_utc = NONE \
           AND expires_at_utc <= time::now() RETURN AFTER)[0]; \
     IF $changed != NONE { \
         CREATE $event_record CONTENT $event_content; \
         UPDATE $record SET reclaim_event_ledger_event_id = $event_record; \
         RETURN (SELECT * FROM $record)[0]; \
     } ELSE { RETURN NONE; }; \
     COMMIT TRANSACTION;";

#[cfg(feature = "test-utils")]
const RECLAIM_CLAIM_FAIL_BEFORE_EVENT_QUERY: &str = "BEGIN TRANSACTION; \
     LET $changed = (UPDATE $record SET status = 'reclaimed', \
         released_at_utc = $released_at_utc, reason = $reason \
         WHERE status = 'active' AND released_at_utc = NONE \
           AND expires_at_utc <= time::now() RETURN AFTER)[0]; \
     IF $changed != NONE { \
         THROW 'HSK-TEST-FAIL-RECLAIM-EVENT'; \
         CREATE $event_record CONTENT $event_content; \
         RETURN $changed; \
     } ELSE { RETURN NONE; }; \
     COMMIT TRANSACTION;";

fn event_record(event_id: &str) -> RecordId {
    RecordId::new(EVENT_LEDGER_TABLE, event_id.to_string())
}

/// The stored-discriminator UNIQUE indexes replace PostgreSQL's partial
/// unique indexes, so a lost race surfaces as an index violation. The locked
/// SDK exposes the violated index only in the error text; the index names are
/// schema-owned constants, so matching on them is stable.
fn is_unique_violation(error: &SurrealStorageError, index_name: &str) -> bool {
    matches!(error, SurrealStorageError::Database(_)) && error.to_string().contains(index_name)
}

/// EventLedger record keys are strings (`KE-...`); a non-string key is a
/// corrupt link rather than a case to tolerate.
fn record_key_string(record: &RecordId) -> StateRecoveryResult<String> {
    match &record.key {
        RecordIdKey::String(value) => Ok(value.clone()),
        other => Err(StateRecoveryError::InvalidInput(format!(
            "record id key is not a string: {other:?}"
        ))),
    }
}

fn optional_record_key(record: Option<&RecordId>) -> StateRecoveryResult<Option<String>> {
    record.map(record_key_string).transpose()
}

fn event_ledger_write_row(
    event: &NewKernelEvent,
    kernel_event: &KernelEvent,
) -> EventLedgerWriteRow {
    EventLedgerWriteRow {
        event_id: kernel_event.event_id.clone(),
        event_version: event.event_version.clone(),
        kernel_task_run_id: event.kernel_task_run_id.clone(),
        session_run_id: event.session_run_id.clone(),
        aggregate_type: event.aggregate_type.clone(),
        aggregate_id: event.aggregate_id.clone(),
        idempotency_key: event.idempotency_key.clone(),
        event_type: event.event_type.as_str().to_string(),
        actor_kind: event.actor.actor_kind().to_string(),
        actor_id: event.actor.actor_id().to_string(),
        causation_id: event.causation_id.clone(),
        correlation_id: event.correlation_id.clone(),
        payload_hash: event.payload_hash.clone(),
        source_component: event.source_component.clone(),
        payload: event.payload.clone(),
        created_at: kernel_event.created_at,
    }
}

#[derive(Clone)]
pub struct ParallelSwarmStateRecoveryStore {
    storage: SurrealStorage,
    #[cfg(feature = "test-utils")]
    test_control: Arc<StateRecoveryTestControl>,
}

impl ParallelSwarmStateRecoveryStore {
    pub fn new(storage: SurrealStorage) -> Self {
        Self {
            storage,
            #[cfg(feature = "test-utils")]
            test_control: Arc::new(StateRecoveryTestControl::default()),
        }
    }

    pub fn storage(&self) -> &SurrealStorage {
        &self.storage
    }

    #[cfg(feature = "test-utils")]
    pub fn arm_test_failpoint(&self, failpoint: StateRecoveryTestFailpoint) {
        self.test_control
            .armed
            .lock()
            .expect("state-recovery test failpoint mutex poisoned")
            .insert(failpoint);
    }

    #[cfg(feature = "test-utils")]
    fn take_test_failpoint(&self, failpoint: StateRecoveryTestFailpoint) -> bool {
        self.test_control
            .armed
            .lock()
            .expect("state-recovery test failpoint mutex poisoned")
            .remove(&failpoint)
    }

    async fn query<R, B>(
        &self,
        statement: &'static str,
        bindings: B,
    ) -> Result<Vec<R>, SurrealStorageError>
    where
        R: SurrealValue + Send + 'static,
        B: SurrealValue + Send + 'static,
    {
        self.storage
            .with_data_operation(move |database| {
                Box::pin(async move { database.query_values(statement, bindings).await })
            })
            .await
    }

    async fn query_first<R, B>(
        &self,
        statement: &'static str,
        bindings: B,
    ) -> Result<Option<R>, SurrealStorageError>
    where
        R: SurrealValue + Send + 'static,
        B: SurrealValue + Send + 'static,
    {
        Ok(self.query(statement, bindings).await?.into_iter().next())
    }

    /// Builds the receipt event and the row-plus-event transaction bindings
    /// for a fresh receipt row.
    fn event_and_bindings(
        event: NewKernelEvent,
        table: &'static str,
        row_id: String,
        content: surrealdb::types::Value,
    ) -> (String, CreateRowWithEventBindings) {
        let kernel_event = KernelEvent::from_new(event.clone());
        let event_id = kernel_event.event_id.clone();
        let bindings = CreateRowWithEventBindings {
            event_record: event_record(&event_id),
            event_content: event_ledger_write_row(&event, &kernel_event).into_value(),
            record: RecordId::new(table, row_id),
            content,
        };
        (event_id, bindings)
    }

    pub async fn claim_work_surface(
        &self,
        request: WorkClaimRequest,
    ) -> StateRecoveryResult<WorkClaimOutcome> {
        validate_ttl(request.ttl_seconds)?;
        validate_claim_scope(&request.workspace_id, &request.scope)?;
        require_capability(&request.lane, required_claim_capability(&request.scope))?;
        let reclaimer = system_reclaimer_lane()?;
        self.reclaim_expired_work_claims(
            &reclaimer,
            "system-expired-claim-reclaim",
            "opportunistic expired claim sweep",
        )
        .await?;
        if let Some(holder) = self.active_claim_for_scope(&request.scope).await? {
            return Ok(WorkClaimOutcome {
                status: ClaimStatus::Held,
                claim_id: holder.claim_id,
                active_holder: Some(holder.lane),
                event_ledger_event_id: holder.event_ledger_event_id,
            });
        }

        let claim_id = format!("PSR-CLAIM-{}", Uuid::now_v7());
        let persistent_lane = request.lane.scrubbed_for_persistence();
        let now = Utc::now();
        let event = Self::build_event(
            KernelEventType::SessionClaimed,
            "parallel_swarm_claim",
            &claim_id,
            &persistent_lane,
            &request.session_id,
            json!({
                "schema_id": "hsk.parallel_swarm.claim@1",
                "claim_id": claim_id,
                "workspace_id": request.workspace_id,
                "wp_id": request.wp_id,
                "mt_id": request.mt_id,
                "scope": request.scope,
                "lane": persistent_lane,
                "reason": request.reason,
            }),
        )?;
        let kernel_event = KernelEvent::from_new(event.clone());
        let event_id = kernel_event.event_id.clone();
        let claim_row = ClaimRow {
            claim_id: claim_id.clone(),
            workspace_id: request.workspace_id.clone(),
            wp_id: request.wp_id.clone(),
            mt_id: request.mt_id.clone(),
            scope_kind: request.scope.kind_str().to_string(),
            scope_id: request.scope.scope_id(),
            lane_id: persistent_lane.lane_id.clone(),
            actor_id: persistent_lane.actor_id.clone(),
            lane_kind: persistent_lane.lane_kind.as_str().to_string(),
            attribution_jsonb: serde_json::to_value(&persistent_lane.attribution)?,
            session_id: request.session_id.clone(),
            status: ClaimStatus::Active.as_str().to_string(),
            reason: request.reason.clone(),
            claimed_at_utc: now,
            expires_at_utc: now + ChronoDuration::seconds(request.ttl_seconds),
            released_at_utc: None,
            event_ledger_event_id: Some(event_record(&event_id)),
            release_event_ledger_event_id: None,
            reclaim_event_ledger_event_id: None,
        };
        let bindings = CreateRowWithEventBindings {
            event_record: event_record(&event_id),
            event_content: event_ledger_write_row(&event, &kernel_event).into_value(),
            record: RecordId::new(CLAIMS_TABLE, claim_id.clone()),
            content: claim_row.into_value(),
        };
        let statement = CREATE_ROW_WITH_EVENT_QUERY;
        #[cfg(feature = "test-utils")]
        let statement = if self
            .take_test_failpoint(StateRecoveryTestFailpoint::ClaimAfterEventBeforeAuthority)
        {
            CREATE_ROW_WITH_EVENT_FAIL_AFTER_EVENT_QUERY
        } else {
            statement
        };
        match self
            .query::<surrealdb::types::Value, _>(statement, bindings)
            .await
        {
            Ok(_) => Ok(WorkClaimOutcome {
                status: ClaimStatus::Active,
                claim_id,
                active_holder: None,
                event_ledger_event_id: Some(event_id),
            }),
            Err(error) if is_unique_violation(&error, "ux_agent_worktree_claims_active_scope") => {
                let holder = self.active_claim_for_scope(&request.scope).await?;
                Ok(WorkClaimOutcome {
                    status: ClaimStatus::Held,
                    claim_id: holder
                        .as_ref()
                        .map(|h| h.claim_id.clone())
                        .unwrap_or(claim_id),
                    active_holder: holder.map(|h| h.lane),
                    event_ledger_event_id: None,
                })
            }
            Err(error) => Err(error.into()),
        }
    }

    pub async fn list_active_claims(
        &self,
        workspace_id: &str,
    ) -> StateRecoveryResult<Vec<WorkClaimRecord>> {
        let reclaimer = system_reclaimer_lane()?;
        self.reclaim_expired_work_claims(
            &reclaimer,
            "system-expired-claim-reclaim",
            "opportunistic expired claim sweep",
        )
        .await?;
        let rows: Vec<ClaimRow> = self
            .query(
                "SELECT * FROM knowledge_agent_worktree_claims \
                 WHERE workspace_id = $workspace_id AND status = 'active' \
                   AND released_at_utc = NONE AND expires_at_utc > time::now() \
                 ORDER BY claimed_at_utc ASC, claim_id ASC;",
                WorkspaceBinding {
                    workspace_id: workspace_id.to_string(),
                },
            )
            .await?;
        rows.into_iter().map(work_claim_from_row).collect()
    }

    pub async fn inspect_swarm_evidence(
        &self,
        request: SwarmEvidenceInspectionRequest,
    ) -> StateRecoveryResult<SwarmEvidenceInspectionSnapshot> {
        require_capability(&request.lane, AgentCapability::InspectEvidence)?;
        ensure_safe_token("workspace_id", &request.workspace_id)?;
        let limit = bounded_inspection_limit(request.limit)?;
        let bindings = || WorkspaceLimitBindings {
            workspace_id: request.workspace_id.clone(),
            limit,
        };

        let claim_rows: Vec<ClaimRow> = self
            .query(
                "SELECT * FROM knowledge_agent_worktree_claims \
                 WHERE workspace_id = $workspace_id \
                 ORDER BY claimed_at_utc DESC, claim_id DESC LIMIT $limit;",
                bindings(),
            )
            .await?;
        // The previous SQL joins walked handoff -> claim and receipt ->
        // checkpoint; the record links express the same constraint as field
        // traversals (a NONE link never equals the workspace, matching the
        // INNER JOIN exclusion).
        let handoff_rows: Vec<HandoffRow> = self
            .query(
                "SELECT * FROM knowledge_agent_role_mailbox_handoffs \
                 WHERE claim_id.workspace_id = $workspace_id \
                 ORDER BY created_at_utc DESC, handoff_id DESC LIMIT $limit;",
                bindings(),
            )
            .await?;
        let checkpoint_rows: Vec<CheckpointRow> = self
            .query(
                "SELECT * FROM knowledge_agent_state_recovery_checkpoints \
                 WHERE workspace_id = $workspace_id \
                 ORDER BY created_at_utc DESC, checkpoint_id DESC LIMIT $limit;",
                bindings(),
            )
            .await?;
        let recovery_rows: Vec<RecoveryReceiptRow> = self
            .query(
                "SELECT * FROM knowledge_agent_recovery_receipts \
                 WHERE checkpoint_id.workspace_id = $workspace_id \
                 ORDER BY recovered_at_utc DESC, receipt_id DESC LIMIT $limit;",
                bindings(),
            )
            .await?;
        let lease_rows: Vec<LeaseRow> = self
            .query(
                "SELECT * FROM knowledge_parallel_indexing_lease_queue \
                 WHERE workspace_id = $workspace_id \
                 ORDER BY enqueued_at_utc DESC, lease_id DESC LIMIT $limit;",
                bindings(),
            )
            .await?;
        let quiet_rows: Vec<QuietWorkRow> = self
            .query(
                "SELECT * FROM knowledge_agent_quiet_background_work \
                 WHERE workspace_id = $workspace_id \
                 ORDER BY created_at_utc DESC, receipt_id DESC LIMIT $limit;",
                bindings(),
            )
            .await?;

        Ok(SwarmEvidenceInspectionSnapshot {
            workspace_id: request.workspace_id,
            claims: claim_rows
                .into_iter()
                .map(work_claim_from_row)
                .collect::<StateRecoveryResult<Vec<_>>>()?,
            mailbox_handoffs: handoff_rows
                .into_iter()
                .map(mailbox_handoff_from_row)
                .collect::<StateRecoveryResult<Vec<_>>>()?,
            checkpoints: checkpoint_rows
                .into_iter()
                .map(checkpoint_from_row)
                .collect::<StateRecoveryResult<Vec<_>>>()?,
            recovery_receipts: recovery_rows
                .into_iter()
                .map(recovery_receipt_from_row)
                .collect::<StateRecoveryResult<Vec<_>>>()?,
            indexing_leases: lease_rows
                .into_iter()
                .map(index_lease_from_row)
                .collect::<StateRecoveryResult<Vec<_>>>()?,
            quiet_background_work: quiet_rows
                .into_iter()
                .map(quiet_background_work_from_row)
                .collect::<StateRecoveryResult<Vec<_>>>()?,
        })
    }

    pub async fn project_swarm_dashboard(
        &self,
        request: SwarmDashboardProjectionRequest,
    ) -> StateRecoveryResult<ParallelSwarmDashboardProjectionV1> {
        require_capability(&request.lane, AgentCapability::InspectEvidence)?;
        ensure_safe_token("workspace_id", &request.workspace_id)?;
        if let Some(wp_id) = request.wp_id.as_deref() {
            ensure_safe_token("wp_id", wp_id)?;
        }
        if let Some(mt_id) = request.mt_id.as_deref() {
            ensure_safe_token("mt_id", mt_id)?;
        }
        let limit = bounded_inspection_limit(request.limit)?;
        let generated_at_utc = Utc::now();

        let bindings = || DashboardFilterBindings {
            workspace_id: request.workspace_id.clone(),
            wp_id: request.wp_id.clone(),
            mt_id: request.mt_id.clone(),
            limit,
        };

        let claim_rows: Vec<ClaimRow> = self
            .query(
                "SELECT * FROM knowledge_agent_worktree_claims \
                 WHERE workspace_id = $workspace_id \
                   AND ($wp_id = NONE OR wp_id = $wp_id) \
                   AND ($mt_id = NONE OR mt_id = $mt_id) \
                 ORDER BY claimed_at_utc DESC, claim_id DESC LIMIT $limit;",
                bindings(),
            )
            .await?;
        let claims = claim_rows
            .into_iter()
            .map(work_claim_from_row)
            .collect::<StateRecoveryResult<Vec<_>>>()?;

        let handoff_rows: Vec<HandoffRow> = self
            .query(
                "SELECT * FROM knowledge_agent_role_mailbox_handoffs \
                 WHERE claim_id.workspace_id = $workspace_id \
                   AND ($wp_id = NONE OR wp_id = $wp_id) \
                   AND ($mt_id = NONE OR mt_id = $mt_id) \
                 ORDER BY created_at_utc DESC, handoff_id DESC LIMIT $limit;",
                bindings(),
            )
            .await?;
        let mailbox_handoffs = handoff_rows
            .into_iter()
            .map(mailbox_handoff_from_row)
            .collect::<StateRecoveryResult<Vec<_>>>()?;

        let checkpoint_rows: Vec<CheckpointRow> = self
            .query(
                "SELECT * FROM knowledge_agent_state_recovery_checkpoints \
                 WHERE workspace_id = $workspace_id \
                   AND ($wp_id = NONE OR wp_id = $wp_id) \
                   AND ($mt_id = NONE OR mt_id = $mt_id) \
                 ORDER BY created_at_utc DESC, checkpoint_id DESC LIMIT $limit;",
                bindings(),
            )
            .await?;
        let checkpoints = checkpoint_rows
            .into_iter()
            .map(checkpoint_from_row)
            .collect::<StateRecoveryResult<Vec<_>>>()?;

        let recovery_rows: Vec<RecoveryReceiptRow> = self
            .query(
                "SELECT * FROM knowledge_agent_recovery_receipts \
                 WHERE checkpoint_id.workspace_id = $workspace_id \
                   AND ($wp_id = NONE OR checkpoint_id.wp_id = $wp_id) \
                   AND ($mt_id = NONE OR checkpoint_id.mt_id = $mt_id) \
                 ORDER BY recovered_at_utc DESC, receipt_id DESC LIMIT $limit;",
                bindings(),
            )
            .await?;
        let recovery_receipts = recovery_rows
            .into_iter()
            .map(recovery_receipt_from_row)
            .collect::<StateRecoveryResult<Vec<_>>>()?;

        let lease_rows: Vec<LeaseRow> = self
            .query(
                "SELECT * FROM knowledge_parallel_indexing_lease_queue \
                 WHERE workspace_id = $workspace_id \
                   AND ($wp_id = NONE OR wp_id = $wp_id) \
                   AND ($mt_id = NONE OR mt_id = $mt_id) \
                 ORDER BY enqueued_at_utc DESC, lease_id DESC LIMIT $limit;",
                bindings(),
            )
            .await?;
        let indexing_leases = lease_rows
            .into_iter()
            .map(index_lease_from_row)
            .collect::<StateRecoveryResult<Vec<_>>>()?;

        let quiet_rows: Vec<QuietWorkRow> = self
            .query(
                "SELECT * FROM knowledge_agent_quiet_background_work \
                 WHERE workspace_id = $workspace_id \
                   AND ($wp_id = NONE OR wp_id = $wp_id) \
                   AND ($mt_id = NONE OR mt_id = $mt_id) \
                 ORDER BY created_at_utc DESC, receipt_id DESC LIMIT $limit;",
                bindings(),
            )
            .await?;
        let quiet_background_work = quiet_rows
            .into_iter()
            .map(quiet_background_work_from_row)
            .collect::<StateRecoveryResult<Vec<_>>>()?;

        let authority_totals = self
            .dashboard_authority_totals(
                &request.workspace_id,
                request.wp_id.as_deref(),
                request.mt_id.as_deref(),
            )
            .await?;

        let mut warnings = vec![SwarmDashboardWarningV1 {
            code: "handoffs_without_workspace_source_ref_excluded".to_string(),
            detail: "mailbox handoff receipts without a claim-backed workspace source are excluded by contract and are never counted from workspace dashboards".to_string(),
        }];

        let mut source_event_ids = BTreeSet::new();
        collect_projection_event_ids(
            &claims,
            &mailbox_handoffs,
            &checkpoints,
            &recovery_receipts,
            &indexing_leases,
            &quiet_background_work,
            &mut source_event_ids,
        );
        let source_watermark = self
            .dashboard_event_watermark(source_event_ids.iter().cloned().collect())
            .await?;
        for missing in &source_watermark.missing_event_refs {
            warnings.push(SwarmDashboardWarningV1 {
                code: "missing_event_ledger_ref".to_string(),
                detail: format!("projection source referenced missing EventLedger row {missing}"),
            });
        }

        let claim_rows = claims
            .iter()
            .map(|claim| dashboard_claim_row(claim, generated_at_utc))
            .collect::<Vec<_>>();
        let handoff_rows = mailbox_handoffs
            .iter()
            .map(dashboard_handoff_row)
            .collect::<Vec<_>>();
        let checkpoint_rows = checkpoints
            .iter()
            .map(dashboard_checkpoint_row)
            .collect::<Vec<_>>();
        let recovery_rows = recovery_receipts
            .iter()
            .map(dashboard_recovery_receipt_row)
            .collect::<Vec<_>>();
        let lease_rows = indexing_leases
            .iter()
            .map(dashboard_indexing_lease_row)
            .collect::<Vec<_>>();
        let quiet_rows = quiet_background_work
            .iter()
            .map(dashboard_quiet_work_row)
            .collect::<Vec<_>>();
        add_truncation_warning(
            &mut warnings,
            "claims",
            claim_rows.len(),
            authority_totals.claims,
        );
        add_truncation_warning(
            &mut warnings,
            "mailbox_handoffs",
            handoff_rows.len(),
            authority_totals.mailbox_handoffs,
        );
        add_truncation_warning(
            &mut warnings,
            "recovery_checkpoints",
            checkpoint_rows.len(),
            authority_totals.recovery_checkpoints,
        );
        add_truncation_warning(
            &mut warnings,
            "recovery_receipts",
            recovery_rows.len(),
            authority_totals.recovery_receipts,
        );
        add_truncation_warning(
            &mut warnings,
            "indexing_leases",
            lease_rows.len(),
            authority_totals.indexing_leases,
        );
        add_truncation_warning(
            &mut warnings,
            "quiet_background_work",
            quiet_rows.len(),
            authority_totals.quiet_background_work,
        );

        let lanes = dashboard_lane_rows(
            &claims,
            &mailbox_handoffs,
            &checkpoints,
            &recovery_receipts,
            &indexing_leases,
            &quiet_background_work,
        );
        let mut totals = dashboard_totals(authority_totals);
        totals.warnings = warnings.len() as i64;

        Ok(ParallelSwarmDashboardProjectionV1 {
            schema_id: PARALLEL_SWARM_DASHBOARD_SCHEMA_ID.to_string(),
            workspace_id: request.workspace_id.clone(),
            generated_at_utc,
            filters: SwarmDashboardProjectionFilters {
                workspace_id: request.workspace_id,
                wp_id: request.wp_id,
                mt_id: request.mt_id,
                limit,
            },
            projection_contract: swarm_dashboard_projection_contract(),
            source_watermark,
            totals,
            lanes,
            claims: claim_rows,
            mailbox_handoffs: handoff_rows,
            recovery_checkpoints: checkpoint_rows,
            recovery_receipts: recovery_rows,
            indexing_leases: lease_rows,
            quiet_background_work: quiet_rows,
            warnings,
        })
    }

    async fn dashboard_authority_totals(
        &self,
        workspace_id: &str,
        wp_id: Option<&str>,
        mt_id: Option<&str>,
    ) -> StateRecoveryResult<SwarmDashboardAuthorityTotals> {
        #[derive(SurrealValue)]
        struct AuthorityCounts {
            claims: i64,
            active_claims: i64,
            stale_active_claims: i64,
            mailbox_handoffs: i64,
            recovery_checkpoints: i64,
            recovery_receipts: i64,
            indexing_leases: i64,
            acquired_indexing_leases: i64,
            quiet_background_work: i64,
        }

        let bindings = || TotalsFilterBindings {
            workspace_id: workspace_id.to_string(),
            wp_id: wp_id.map(ToOwned::to_owned),
            mt_id: mt_id.map(ToOwned::to_owned),
        };

        // One RETURN statement evaluates every scalar total in a single
        // statement (one transaction), replacing PostgreSQL's per-table
        // COUNT/FILTER queries with one snapshot.
        let counts: AuthorityCounts = self
            .query_first(
                "RETURN { \
                 claims: array::len((SELECT VALUE id FROM knowledge_agent_worktree_claims \
                     WHERE workspace_id = $workspace_id \
                       AND ($wp_id = NONE OR wp_id = $wp_id) \
                       AND ($mt_id = NONE OR mt_id = $mt_id))), \
                 active_claims: array::len((SELECT VALUE id FROM knowledge_agent_worktree_claims \
                     WHERE workspace_id = $workspace_id AND status = 'active' \
                       AND ($wp_id = NONE OR wp_id = $wp_id) \
                       AND ($mt_id = NONE OR mt_id = $mt_id))), \
                 stale_active_claims: array::len((SELECT VALUE id FROM knowledge_agent_worktree_claims \
                     WHERE workspace_id = $workspace_id AND status = 'active' \
                       AND expires_at_utc <= time::now() \
                       AND ($wp_id = NONE OR wp_id = $wp_id) \
                       AND ($mt_id = NONE OR mt_id = $mt_id))), \
                 mailbox_handoffs: array::len((SELECT VALUE id FROM knowledge_agent_role_mailbox_handoffs \
                     WHERE claim_id.workspace_id = $workspace_id \
                       AND ($wp_id = NONE OR wp_id = $wp_id) \
                       AND ($mt_id = NONE OR mt_id = $mt_id))), \
                 recovery_checkpoints: array::len((SELECT VALUE id FROM knowledge_agent_state_recovery_checkpoints \
                     WHERE workspace_id = $workspace_id \
                       AND ($wp_id = NONE OR wp_id = $wp_id) \
                       AND ($mt_id = NONE OR mt_id = $mt_id))), \
                 recovery_receipts: array::len((SELECT VALUE id FROM knowledge_agent_recovery_receipts \
                     WHERE checkpoint_id.workspace_id = $workspace_id \
                       AND ($wp_id = NONE OR checkpoint_id.wp_id = $wp_id) \
                       AND ($mt_id = NONE OR checkpoint_id.mt_id = $mt_id))), \
                 indexing_leases: array::len((SELECT VALUE id FROM knowledge_parallel_indexing_lease_queue \
                     WHERE workspace_id = $workspace_id \
                       AND ($wp_id = NONE OR wp_id = $wp_id) \
                       AND ($mt_id = NONE OR mt_id = $mt_id))), \
                 acquired_indexing_leases: array::len((SELECT VALUE id FROM knowledge_parallel_indexing_lease_queue \
                     WHERE workspace_id = $workspace_id AND status = 'acquired' \
                       AND ($wp_id = NONE OR wp_id = $wp_id) \
                       AND ($mt_id = NONE OR mt_id = $mt_id))), \
                 quiet_background_work: array::len((SELECT VALUE id FROM knowledge_agent_quiet_background_work \
                     WHERE workspace_id = $workspace_id \
                       AND ($wp_id = NONE OR wp_id = $wp_id) \
                       AND ($mt_id = NONE OR mt_id = $mt_id))) \
                 };",
                bindings(),
            )
            .await?
            .ok_or_else(|| {
                StateRecoveryError::InvalidInput(
                    "dashboard authority totals returned no snapshot".to_string(),
                )
            })?;

        // The distinct-EventLedger count: gather the receipt links from every
        // source table (the PostgreSQL CTE's UNION legs), then count the
        // matching ledger rows. `record::id(...)` unwraps the record link to
        // the ledger's string event id; the UNION's DISTINCT is reproduced by
        // the BTreeSet.
        let mut source_event_ids = BTreeSet::new();
        let id_batches: [Vec<String>; 8] = [
            self.query(
                "SELECT VALUE record::id(event_ledger_event_id) \
                 FROM knowledge_agent_worktree_claims \
                 WHERE workspace_id = $workspace_id AND event_ledger_event_id != NONE \
                   AND ($wp_id = NONE OR wp_id = $wp_id) \
                   AND ($mt_id = NONE OR mt_id = $mt_id);",
                bindings(),
            )
            .await?,
            self.query(
                "SELECT VALUE record::id(release_event_ledger_event_id) \
                 FROM knowledge_agent_worktree_claims \
                 WHERE workspace_id = $workspace_id AND release_event_ledger_event_id != NONE \
                   AND ($wp_id = NONE OR wp_id = $wp_id) \
                   AND ($mt_id = NONE OR mt_id = $mt_id);",
                bindings(),
            )
            .await?,
            self.query(
                "SELECT VALUE record::id(reclaim_event_ledger_event_id) \
                 FROM knowledge_agent_worktree_claims \
                 WHERE workspace_id = $workspace_id AND reclaim_event_ledger_event_id != NONE \
                   AND ($wp_id = NONE OR wp_id = $wp_id) \
                   AND ($mt_id = NONE OR mt_id = $mt_id);",
                bindings(),
            )
            .await?,
            self.query(
                "SELECT VALUE record::id(event_ledger_event_id) \
                 FROM knowledge_agent_role_mailbox_handoffs \
                 WHERE claim_id.workspace_id = $workspace_id \
                   AND ($wp_id = NONE OR wp_id = $wp_id) \
                   AND ($mt_id = NONE OR mt_id = $mt_id);",
                bindings(),
            )
            .await?,
            self.query(
                "SELECT VALUE record::id(event_ledger_event_id) \
                 FROM knowledge_agent_state_recovery_checkpoints \
                 WHERE workspace_id = $workspace_id \
                   AND ($wp_id = NONE OR wp_id = $wp_id) \
                   AND ($mt_id = NONE OR mt_id = $mt_id);",
                bindings(),
            )
            .await?,
            self.query(
                "SELECT VALUE record::id(event_ledger_event_id) \
                 FROM knowledge_agent_recovery_receipts \
                 WHERE checkpoint_id.workspace_id = $workspace_id \
                   AND ($wp_id = NONE OR checkpoint_id.wp_id = $wp_id) \
                   AND ($mt_id = NONE OR checkpoint_id.mt_id = $mt_id);",
                bindings(),
            )
            .await?,
            self.query(
                "SELECT VALUE record::id(event_ledger_event_id) \
                 FROM knowledge_parallel_indexing_lease_queue \
                 WHERE workspace_id = $workspace_id AND event_ledger_event_id != NONE \
                   AND ($wp_id = NONE OR wp_id = $wp_id) \
                   AND ($mt_id = NONE OR mt_id = $mt_id);",
                bindings(),
            )
            .await?,
            self.query(
                "SELECT VALUE record::id(event_ledger_event_id) \
                 FROM knowledge_agent_quiet_background_work \
                 WHERE workspace_id = $workspace_id \
                   AND ($wp_id = NONE OR wp_id = $wp_id) \
                   AND ($mt_id = NONE OR mt_id = $mt_id);",
                bindings(),
            )
            .await?,
        ];
        for batch in id_batches {
            source_event_ids.extend(batch);
        }
        let events = if source_event_ids.is_empty() {
            0
        } else {
            let matched: Vec<String> = self
                .query(
                    "SELECT VALUE event_id FROM kernel_event_ledger \
                     WHERE source_component = $source_component AND event_id IN $event_ids;",
                    EventLookupBindings {
                        source_component: PARALLEL_SWARM_SOURCE_COMPONENT.to_string(),
                        event_ids: source_event_ids.into_iter().collect(),
                    },
                )
                .await?;
            matched.into_iter().collect::<BTreeSet<String>>().len() as i64
        };

        let claim_status_rows: Vec<GroupCountRow> = self
            .query(
                "SELECT status AS group_key, count() AS row_count \
                 FROM knowledge_agent_worktree_claims \
                 WHERE workspace_id = $workspace_id \
                   AND ($wp_id = NONE OR wp_id = $wp_id) \
                   AND ($mt_id = NONE OR mt_id = $mt_id) \
                 GROUP BY group_key;",
                bindings(),
            )
            .await?;
        let handoff_status_rows: Vec<GroupCountRow> = self
            .query(
                "SELECT status AS group_key, count() AS row_count \
                 FROM knowledge_agent_role_mailbox_handoffs \
                 WHERE claim_id.workspace_id = $workspace_id \
                   AND ($wp_id = NONE OR wp_id = $wp_id) \
                   AND ($mt_id = NONE OR mt_id = $mt_id) \
                 GROUP BY group_key;",
                bindings(),
            )
            .await?;
        let lease_status_rows: Vec<GroupCountRow> = self
            .query(
                "SELECT status AS group_key, count() AS row_count \
                 FROM knowledge_parallel_indexing_lease_queue \
                 WHERE workspace_id = $workspace_id \
                   AND ($wp_id = NONE OR wp_id = $wp_id) \
                   AND ($mt_id = NONE OR mt_id = $mt_id) \
                 GROUP BY group_key;",
                bindings(),
            )
            .await?;
        let quiet_kind_rows: Vec<GroupCountRow> = self
            .query(
                "SELECT work_kind AS group_key, count() AS row_count \
                 FROM knowledge_agent_quiet_background_work \
                 WHERE workspace_id = $workspace_id \
                   AND ($wp_id = NONE OR wp_id = $wp_id) \
                   AND ($mt_id = NONE OR mt_id = $mt_id) \
                 GROUP BY group_key;",
                bindings(),
            )
            .await?;

        Ok(SwarmDashboardAuthorityTotals {
            claims: counts.claims,
            active_claims: counts.active_claims,
            stale_active_claims: counts.stale_active_claims,
            mailbox_handoffs: counts.mailbox_handoffs,
            recovery_checkpoints: counts.recovery_checkpoints,
            recovery_receipts: counts.recovery_receipts,
            indexing_leases: counts.indexing_leases,
            acquired_indexing_leases: counts.acquired_indexing_leases,
            quiet_background_work: counts.quiet_background_work,
            events,
            claims_by_status: dashboard_group_count_map(claim_status_rows),
            handoffs_by_status: dashboard_group_count_map(handoff_status_rows),
            leases_by_status: dashboard_group_count_map(lease_status_rows),
            quiet_work_by_kind: dashboard_group_count_map(quiet_kind_rows),
        })
    }

    async fn dashboard_event_watermark(
        &self,
        event_ids: Vec<String>,
    ) -> StateRecoveryResult<SwarmDashboardSourceWatermarkV1> {
        if event_ids.is_empty() {
            return Ok(SwarmDashboardSourceWatermarkV1 {
                source_component: PARALLEL_SWARM_SOURCE_COMPONENT.to_string(),
                event_count: 0,
                max_event_created_at_utc: None,
                events: Vec::new(),
                aggregate_counts: Vec::new(),
                missing_event_refs: Vec::new(),
            });
        }
        #[derive(SurrealValue)]
        struct WatermarkRow {
            event_id: String,
            source_component: String,
            aggregate_type: String,
            aggregate_id: String,
            created_at: DateTime<Utc>,
        }

        let rows: Vec<WatermarkRow> = self
            .query(
                "SELECT event_id, source_component, aggregate_type, aggregate_id, created_at \
                 FROM kernel_event_ledger \
                 WHERE source_component = $source_component AND event_id IN $event_ids \
                 ORDER BY created_at DESC, event_id DESC;",
                EventLookupBindings {
                    source_component: PARALLEL_SWARM_SOURCE_COMPONENT.to_string(),
                    event_ids: event_ids.clone(),
                },
            )
            .await?;
        let mut found = BTreeSet::new();
        let mut counts = BTreeMap::<String, i64>::new();
        let mut max_created = None;
        let mut events = Vec::new();
        for row in rows {
            let event_id = row.event_id;
            let source_component = row.source_component;
            let aggregate_type = row.aggregate_type;
            let aggregate_id = row.aggregate_id;
            let created_at = row.created_at;
            found.insert(event_id.clone());
            *counts.entry(aggregate_type.clone()).or_insert(0) += 1;
            if max_created.map_or(true, |current| created_at > current) {
                max_created = Some(created_at);
            }
            events.push(SwarmDashboardEventRefV1 {
                event_id,
                source_component,
                aggregate_type,
                aggregate_id,
                created_at_utc: created_at,
            });
        }
        let missing_event_refs = event_ids
            .into_iter()
            .filter(|event_id| !found.contains(event_id))
            .collect::<Vec<_>>();
        let aggregate_counts = counts
            .into_iter()
            .map(|(aggregate_type, count)| SwarmDashboardAggregateCountV1 {
                aggregate_type,
                count,
            })
            .collect::<Vec<_>>();
        Ok(SwarmDashboardSourceWatermarkV1 {
            source_component: PARALLEL_SWARM_SOURCE_COMPONENT.to_string(),
            event_count: found.len() as i64,
            max_event_created_at_utc: max_created,
            events,
            aggregate_counts,
            missing_event_refs,
        })
    }

    pub async fn record_quiet_background_work(
        &self,
        request: QuietBackgroundWorkRequest,
    ) -> StateRecoveryResult<QuietBackgroundWorkRecord> {
        require_capability(&request.lane, AgentCapability::RunQuietBackgroundWork)?;
        ensure_safe_token("workspace_id", &request.workspace_id)?;
        ensure_safe_token("wp_id", &request.wp_id)?;
        ensure_safe_token("mt_id", &request.mt_id)?;
        ensure_safe_token("subject_id", &request.subject_id)?;
        ensure_safe_token("session_id", &request.session_id)?;
        validate_quiet_background_policy(request.work_kind, &request.policy)?;
        ensure_bounded_text("evidence_ref", &request.evidence_ref, 512)?;

        let receipt_id = format!("PSR-QUIET-{}", Uuid::now_v7());
        let persistent_lane = request.lane.scrubbed_for_persistence();
        let created_at_utc = Utc::now();
        let event = Self::build_event(
            KernelEventType::KnowledgeQuietBackgroundWorkRecorded,
            "parallel_swarm_quiet_background_work",
            &receipt_id,
            &persistent_lane,
            &request.session_id,
            json!({
                "schema_id": "hsk.parallel_swarm.quiet_background_work@1",
                "receipt_id": &receipt_id,
                "workspace_id": &request.workspace_id,
                "wp_id": &request.wp_id,
                "mt_id": &request.mt_id,
                "work_kind": request.work_kind,
                "subject_id": &request.subject_id,
                "quiet_policy": &request.policy,
                "evidence_ref": &request.evidence_ref,
            }),
        )?;
        let kernel_event = KernelEvent::from_new(event.clone());
        let event_id = kernel_event.event_id.clone();
        let row = QuietWorkRow {
            receipt_id: receipt_id.clone(),
            workspace_id: request.workspace_id.clone(),
            wp_id: request.wp_id.clone(),
            mt_id: request.mt_id.clone(),
            work_kind: request.work_kind.as_str().to_string(),
            subject_id: request.subject_id.clone(),
            lane_id: persistent_lane.lane_id.clone(),
            actor_id: persistent_lane.actor_id.clone(),
            lane_kind: persistent_lane.lane_kind.as_str().to_string(),
            attribution_jsonb: serde_json::to_value(&persistent_lane.attribution)?,
            session_id: request.session_id.clone(),
            quiet_policy_jsonb: serde_json::to_value(&request.policy)?,
            evidence_ref: request.evidence_ref.clone(),
            event_ledger_event_id: event_record(&event_id),
            created_at_utc,
        };
        let bindings = CreateRowWithEventBindings {
            event_record: event_record(&event_id),
            event_content: event_ledger_write_row(&event, &kernel_event).into_value(),
            record: RecordId::new(QUIET_TABLE, receipt_id.clone()),
            content: row.into_value(),
        };
        let statement = CREATE_ROW_WITH_EVENT_QUERY;
        #[cfg(feature = "test-utils")]
        let statement = if self
            .take_test_failpoint(StateRecoveryTestFailpoint::QuietAfterEventBeforeAuthority)
        {
            CREATE_ROW_WITH_EVENT_FAIL_AFTER_EVENT_QUERY
        } else {
            statement
        };
        let _: Vec<surrealdb::types::Value> = self.query(statement, bindings).await?;
        Ok(QuietBackgroundWorkRecord {
            receipt_id,
            workspace_id: request.workspace_id,
            wp_id: request.wp_id,
            mt_id: request.mt_id,
            work_kind: request.work_kind,
            subject_id: request.subject_id,
            lane: persistent_lane,
            session_id: request.session_id,
            policy: request.policy,
            evidence_ref: request.evidence_ref,
            event_ledger_event_id: event_id,
            created_at_utc,
        })
    }

    pub async fn resolve_backend_navigation_quiet(
        &self,
        lane: AgentLaneIdentity,
        session_id: String,
        wp_id: String,
        mt_id: String,
        command: BackendNavigationCommand,
        params: Value,
    ) -> StateRecoveryResult<QuietResolvedNavigationCommand> {
        let resolved = NavigationCommandSet::default().resolve(command, params)?;
        let workspace_id = resolved
            .params
            .get("workspace_id")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                StateRecoveryError::InvalidInput(
                    "quiet backend navigation requires workspace_id".to_string(),
                )
            })?
            .to_string();
        let evidence_ref = format!(
            "backend-nav://{}#{}",
            resolved.command_id, resolved.deterministic_cache_key
        );
        let quiet_receipt = self
            .record_quiet_background_work(QuietBackgroundWorkRequest {
                lane,
                workspace_id,
                wp_id,
                mt_id,
                work_kind: QuietBackgroundWorkKind::BackendNavigation,
                subject_id: resolved.deterministic_cache_key.clone(),
                session_id,
                policy: resolved.quiet_policy.clone(),
                evidence_ref,
            })
            .await?;
        Ok(QuietResolvedNavigationCommand {
            resolved,
            quiet_receipt,
        })
    }

    pub async fn release_claim(
        &self,
        claim_id: &str,
        lane: &AgentLaneIdentity,
        reason: &str,
    ) -> StateRecoveryResult<bool> {
        // The event payload is built from a pre-read of the owned active
        // claim; the guarded transaction below re-checks the same ownership
        // predicate, so a lost race between the read and the write releases
        // nothing (the FOR UPDATE lock's guarantee, carried by the guard).
        let rows: Vec<ClaimRow> = self
            .query(
                "SELECT * FROM knowledge_agent_worktree_claims \
                 WHERE claim_id = $claim_id AND actor_id = $actor_id \
                   AND status = 'active' AND released_at_utc = NONE;",
                ClaimActorBindings {
                    claim_id: claim_id.to_string(),
                    actor_id: lane.actor_id.clone(),
                },
            )
            .await?;
        let Some(claim) = rows
            .into_iter()
            .next()
            .map(work_claim_from_row)
            .transpose()?
        else {
            return Ok(false);
        };
        let persistent_lane = lane.scrubbed_for_persistence();
        let event = Self::build_event(
            KernelEventType::SessionCompleted,
            "parallel_swarm_claim",
            claim_id,
            &persistent_lane,
            &format!("release-{claim_id}"),
            json!({
                "schema_id": "hsk.parallel_swarm.claim_release@1",
                "claim_id": claim_id,
                "workspace_id": claim.workspace_id,
                "wp_id": claim.wp_id,
                "mt_id": claim.mt_id,
                "scope": claim.scope,
                "lane": persistent_lane,
                "status": ClaimStatus::Released,
                "reason": reason,
            }),
        )?;
        let kernel_event = KernelEvent::from_new(event.clone());
        let event_id = kernel_event.event_id.clone();
        let statement = RELEASE_CLAIM_QUERY;
        #[cfg(feature = "test-utils")]
        let statement = if self
            .take_test_failpoint(StateRecoveryTestFailpoint::ReleaseAfterAuthorityBeforeEvent)
        {
            RELEASE_CLAIM_FAIL_BEFORE_EVENT_QUERY
        } else {
            statement
        };
        let released: Option<bool> = self
            .query_first(
                statement,
                ReleaseClaimBindings {
                    claim: RecordId::new(CLAIMS_TABLE, claim_id.to_string()),
                    actor_id: lane.actor_id.clone(),
                    reason: reason.to_string(),
                    event_record: event_record(&event_id),
                    event_content: event_ledger_write_row(&event, &kernel_event).into_value(),
                },
            )
            .await?;
        Ok(released.unwrap_or(false))
    }

    pub async fn record_role_mailbox_handoff(
        &self,
        request: RoleMailboxHandoffRequest,
    ) -> StateRecoveryResult<RoleMailboxHandoffRecord> {
        require_capability(&request.from_lane, AgentCapability::WriteMailbox)?;
        ensure_safe_token("to_role", &request.to_role)?;
        ensure_sha256(&request.body_sha256)?;
        let handoff_id = format!("PSR-HANDOFF-{}", Uuid::now_v7());
        let from_lane = request.from_lane.scrubbed_for_persistence();
        let created_at_utc = Utc::now();
        let event = Self::build_event(
            KernelEventType::HbrHandoffGate,
            "parallel_swarm_handoff",
            &handoff_id,
            &from_lane,
            &format!("handoff-{handoff_id}"),
            json!({
                "schema_id": "hsk.parallel_swarm.mailbox_handoff@1",
                "handoff_id": handoff_id,
                "wp_id": request.wp_id,
                "mt_id": request.mt_id,
                "claim_id": request.claim_id,
                "to_role": request.to_role,
                "mailbox_thread_id": request.mailbox_thread_id,
                "mailbox_message_id": request.mailbox_message_id,
                "status": request.status,
                "summary": request.summary,
                "body_sha256": request.body_sha256,
            }),
        )?;
        let kernel_event = KernelEvent::from_new(event.clone());
        let event_id = kernel_event.event_id.clone();
        let row = HandoffRow {
            handoff_id: handoff_id.clone(),
            wp_id: request.wp_id.clone(),
            mt_id: request.mt_id.clone(),
            claim_id: request
                .claim_id
                .as_ref()
                .map(|claim_id| RecordId::new(CLAIMS_TABLE, claim_id.clone())),
            from_lane_id: from_lane.lane_id.clone(),
            from_actor_id: from_lane.actor_id.clone(),
            from_lane_kind: from_lane.lane_kind.as_str().to_string(),
            from_attribution_jsonb: serde_json::to_value(&from_lane.attribution)?,
            to_role: request.to_role.clone(),
            mailbox_thread_id: request.mailbox_thread_id.clone(),
            mailbox_message_id: request.mailbox_message_id.clone(),
            status: request.status.as_str().to_string(),
            summary: request.summary.clone(),
            body_sha256: request.body_sha256.clone(),
            event_ledger_event_id: event_record(&event_id),
            created_at_utc,
        };
        let bindings = CreateRowWithEventBindings {
            event_record: event_record(&event_id),
            event_content: event_ledger_write_row(&event, &kernel_event).into_value(),
            record: RecordId::new(HANDOFFS_TABLE, handoff_id.clone()),
            content: row.into_value(),
        };
        let _: Vec<surrealdb::types::Value> =
            self.query(CREATE_ROW_WITH_EVENT_QUERY, bindings).await?;
        Ok(RoleMailboxHandoffRecord {
            handoff_id,
            wp_id: request.wp_id,
            mt_id: request.mt_id,
            claim_id: request.claim_id,
            from_lane,
            to_role: request.to_role,
            mailbox_thread_id: request.mailbox_thread_id,
            mailbox_message_id: request.mailbox_message_id,
            status: request.status,
            summary: request.summary,
            body_sha256: request.body_sha256,
            event_ledger_event_id: event_id,
            created_at_utc,
        })
    }

    pub async fn record_cloud_fallback_basis(
        &self,
        request: CloudFallbackBasisRequest,
    ) -> StateRecoveryResult<CloudFallbackBasisReceiptV1> {
        require_capability(&request.lane, AgentCapability::NavigateBackend)?;
        if request.lane.lane_kind == AgentLaneKind::Cloud {
            return Err(StateRecoveryError::InvalidInput(
                "cloud fallback basis must be recorded by a non-cloud lane".to_string(),
            ));
        }
        if !matches!(
            request.lane.lane_kind,
            AgentLaneKind::Local | AgentLaneKind::System
        ) {
            return Err(StateRecoveryError::InvalidInput(
                "cloud fallback basis must be recorded by a local/system lane".to_string(),
            ));
        }
        ensure_safe_token("workspace_id", &request.workspace_id)?;
        ensure_safe_token("wp_id", &request.wp_id)?;
        ensure_safe_token("mt_id", &request.mt_id)?;
        ensure_safe_token("claim_id", &request.claim_id)?;
        ensure_safe_token("parent_session_id", &request.parent_session_id)?;
        ensure_safe_token("session_id", &request.session_id)?;
        ensure_sha256(&request.prompt_sha256)?;
        ensure_sha256(&request.evidence_sha256)?;
        ensure_bounded_text("local_attempt_ref", &request.local_attempt_ref, 512)?;
        ensure_bounded_text("summary", &request.summary, 512)?;

        let basis_id = format!("PSR-FALLBACK-{}", Uuid::now_v7());
        let lane = request.lane.scrubbed_for_persistence();
        let event = Self::build_event(
            KernelEventType::HbrHandoffGate,
            "parallel_swarm_cloud_fallback_basis",
            &basis_id,
            &lane,
            &request.session_id,
            json!({
                "schema_id": PARALLEL_SWARM_CLOUD_FALLBACK_BASIS_SCHEMA_ID,
                "basis_id": &basis_id,
                "workspace_id": &request.workspace_id,
                "wp_id": &request.wp_id,
                "mt_id": &request.mt_id,
                "claim_id": &request.claim_id,
                "parent_session_id": &request.parent_session_id,
                "prompt_sha256": &request.prompt_sha256,
                "lane": &lane,
                "fallback_reason": request.fallback_reason,
                "local_attempt_ref": &request.local_attempt_ref,
                "evidence_sha256": &request.evidence_sha256,
                "summary": &request.summary,
            }),
        )?;
        let kernel_event = KernelEvent::from_new(event.clone());
        let fallback_basis_event_id = kernel_event.event_id.clone();
        let _: Vec<surrealdb::types::Value> = self
            .query(
                "CREATE $event_record CONTENT $event_content RETURN AFTER;",
                CreateEventBinding {
                    event_record: event_record(&fallback_basis_event_id),
                    event_content: event_ledger_write_row(&event, &kernel_event).into_value(),
                },
            )
            .await?;

        Ok(CloudFallbackBasisReceiptV1 {
            schema_id: PARALLEL_SWARM_CLOUD_FALLBACK_BASIS_SCHEMA_ID.to_string(),
            basis_id,
            fallback_basis_event_id,
            workspace_id: request.workspace_id,
            wp_id: request.wp_id,
            mt_id: request.mt_id,
            claim_id: request.claim_id,
            parent_session_id: request.parent_session_id,
            prompt_sha256: request.prompt_sha256,
            lane_id: lane.lane_id,
            actor_id: lane.actor_id,
            fallback_reason: request.fallback_reason,
            local_attempt_ref: request.local_attempt_ref,
            evidence_sha256: request.evidence_sha256,
        })
    }

    pub async fn record_cloud_assistance_output(
        &self,
        request: CloudAssistanceRequest,
    ) -> StateRecoveryResult<CloudAssistanceReceiptV1> {
        require_capability(&request.from_lane, AgentCapability::WriteMailbox)?;
        ensure_cloud_assistance_lane(&request.from_lane)?;
        ensure_safe_token("workspace_id", &request.workspace_id)?;
        ensure_safe_token("wp_id", &request.wp_id)?;
        ensure_safe_token("mt_id", &request.mt_id)?;
        ensure_safe_token("claim_id", &request.claim_id)?;
        ensure_safe_token("session_id", &request.session_id)?;
        ensure_safe_token("to_role", &request.to_role)?;
        ensure_safe_token("mailbox_thread_id", &request.mailbox_thread_id)?;
        ensure_safe_token("mailbox_message_id", &request.mailbox_message_id)?;
        ensure_event_id("fallback_basis_event_id", &request.fallback_basis_event_id)?;
        ensure_safe_token("parent_session_id", &request.parent_session_id)?;
        ensure_sha256(&request.prompt_sha256)?;
        ensure_sha256(&request.output_sha256)?;
        ensure_sha256(&request.body_sha256)?;
        ensure_bounded_text("output_text", &request.output_text, 65_536)?;
        ensure_bounded_text("summary", &request.summary, 512)?;
        ensure_bounded_text("target_ref", &request.target_ref, 512)?;

        let receipt_id = format!("PSR-CLOUD-{}", Uuid::now_v7());
        let handoff_id = format!("PSR-HANDOFF-{}", Uuid::now_v7());
        let from_lane = request.from_lane.scrubbed_for_persistence();
        // Pre-reads produce the typed errors; the transaction below re-checks
        // both predicates in-store and THROWs on a lost race, so the receipt
        // can never land against a claim or basis proof that disappeared.
        let claim = self
            .active_cloud_assistance_claim(&request)
            .await?
            .ok_or_else(|| {
                StateRecoveryError::InvalidInput(
                    "cloud assistance requires an active cloud-owned workspace claim".to_string(),
                )
            })?;
        self.ensure_cloud_fallback_basis_event(&request).await?;

        let created_at_utc = Utc::now();
        let handoff_event = Self::build_event(
            KernelEventType::HbrHandoffGate,
            "parallel_swarm_handoff",
            &handoff_id,
            &from_lane,
            &format!("handoff-{handoff_id}"),
            json!({
                "schema_id": "hsk.parallel_swarm.mailbox_handoff@1",
                "handoff_id": &handoff_id,
                "wp_id": &request.wp_id,
                "mt_id": &request.mt_id,
                "claim_id": &request.claim_id,
                "to_role": &request.to_role,
                "mailbox_thread_id": &request.mailbox_thread_id,
                "mailbox_message_id": &request.mailbox_message_id,
                "status": SwarmReceiptStatus::Progress,
                "summary": &request.summary,
                "body_sha256": &request.body_sha256,
                "cloud_assistance_receipt_id": &receipt_id,
            }),
        )?;
        let handoff_kernel_event = KernelEvent::from_new(handoff_event.clone());
        let handoff_event_id = handoff_kernel_event.event_id.clone();
        let handoff_row = HandoffRow {
            handoff_id: handoff_id.clone(),
            wp_id: request.wp_id.clone(),
            mt_id: request.mt_id.clone(),
            claim_id: Some(RecordId::new(CLAIMS_TABLE, request.claim_id.clone())),
            from_lane_id: from_lane.lane_id.clone(),
            from_actor_id: from_lane.actor_id.clone(),
            from_lane_kind: from_lane.lane_kind.as_str().to_string(),
            from_attribution_jsonb: serde_json::to_value(&from_lane.attribution)?,
            to_role: request.to_role.clone(),
            mailbox_thread_id: request.mailbox_thread_id.clone(),
            mailbox_message_id: request.mailbox_message_id.clone(),
            status: SwarmReceiptStatus::Progress.as_str().to_string(),
            summary: request.summary.clone(),
            body_sha256: request.body_sha256.clone(),
            event_ledger_event_id: event_record(&handoff_event_id),
            created_at_utc,
        };

        let assistance_event = Self::build_event(
            KernelEventType::HbrHandoffGate,
            "parallel_swarm_cloud_assistance",
            &receipt_id,
            &from_lane,
            &request.session_id,
            json!({
                "schema_id": PARALLEL_SWARM_CLOUD_ASSISTANCE_SCHEMA_ID,
                "receipt_id": &receipt_id,
                "workspace_id": &request.workspace_id,
                "wp_id": &request.wp_id,
                "mt_id": &request.mt_id,
                "claim_id": &request.claim_id,
                "handoff_id": &handoff_id,
                "handoff_event_ledger_event_id": &handoff_event_id,
                "fallback_basis_event_id": &request.fallback_basis_event_id,
                "parent_session_id": &request.parent_session_id,
                "prompt_sha256": &request.prompt_sha256,
                "session_id": &request.session_id,
                "lane": &from_lane,
                "fallback_reason": request.fallback_reason,
                "output_kind": request.output_kind,
                "output_sha256": &request.output_sha256,
                "body_sha256": &request.body_sha256,
                "output_text": &request.output_text,
                "output_body": &request.output_body_jsonb,
                "target_ref": &request.target_ref,
                "review_state": "pending_review",
                "non_authoritative": true,
                "requires_promotion": true,
                "authority_mutation_allowed": false,
                "promotion_event_id": Option::<String>::None,
            }),
        )?;
        let assistance_kernel_event = KernelEvent::from_new(assistance_event.clone());
        let cloud_assistance_event_id = assistance_kernel_event.event_id.clone();

        #[derive(SurrealValue)]
        struct CloudReceiptRow {
            receipt_id: String,
            workspace_id: String,
            wp_id: String,
            mt_id: String,
            claim_id: RecordId,
            handoff_id: RecordId,
            handoff_event_ledger_event_id: RecordId,
            cloud_assistance_event_id: RecordId,
            fallback_basis_event_id: RecordId,
            parent_session_id: String,
            prompt_sha256: String,
            lane_id: String,
            actor_id: String,
            lane_kind: String,
            provider: String,
            model_label: String,
            attribution_jsonb: Value,
            session_id: String,
            fallback_reason: String,
            output_kind: String,
            output_sha256: String,
            body_sha256: String,
            output_text: String,
            output_body_jsonb: Value,
            target_ref: String,
            review_state: String,
            non_authoritative: bool,
            requires_promotion: bool,
            authority_mutation_allowed: bool,
            promotion_event_id: Option<String>,
        }

        let receipt_row = CloudReceiptRow {
            receipt_id: receipt_id.clone(),
            workspace_id: request.workspace_id.clone(),
            wp_id: request.wp_id.clone(),
            mt_id: request.mt_id.clone(),
            claim_id: RecordId::new(CLAIMS_TABLE, request.claim_id.clone()),
            handoff_id: RecordId::new(HANDOFFS_TABLE, handoff_id.clone()),
            handoff_event_ledger_event_id: event_record(&handoff_event_id),
            cloud_assistance_event_id: event_record(&cloud_assistance_event_id),
            fallback_basis_event_id: event_record(&request.fallback_basis_event_id),
            parent_session_id: request.parent_session_id.clone(),
            prompt_sha256: request.prompt_sha256.clone(),
            lane_id: from_lane.lane_id.clone(),
            actor_id: from_lane.actor_id.clone(),
            lane_kind: from_lane.lane_kind.as_str().to_string(),
            provider: model_provider_kind_as_str(from_lane.attribution.provider.ok_or_else(
                || {
                    StateRecoveryError::InvalidInput(
                        "cloud assistance provider must be present".to_string(),
                    )
                },
            )?)
            .to_string(),
            model_label: from_lane.attribution.model_label.clone(),
            attribution_jsonb: serde_json::to_value(&from_lane.attribution)?,
            session_id: request.session_id.clone(),
            fallback_reason: request.fallback_reason.as_str().to_string(),
            output_kind: request.output_kind.as_str().to_string(),
            output_sha256: request.output_sha256.clone(),
            body_sha256: request.body_sha256.clone(),
            output_text: request.output_text.clone(),
            output_body_jsonb: request.output_body_jsonb.clone(),
            target_ref: request.target_ref.clone(),
            review_state: "pending_review".to_string(),
            non_authoritative: true,
            requires_promotion: true,
            authority_mutation_allowed: false,
            promotion_event_id: None,
        };

        #[derive(SurrealValue)]
        struct CloudAssistanceTxBindings {
            claim: RecordId,
            workspace_id: String,
            wp_id: String,
            mt_id: String,
            lane_id: String,
            actor_id: String,
            basis_event: RecordId,
            source_component: String,
            basis_schema_id: String,
            claim_id_text: String,
            parent_session_id: String,
            prompt_sha256: String,
            fallback_reason: String,
            handoff_event_record: RecordId,
            handoff_event_content: surrealdb::types::Value,
            handoff_record: RecordId,
            handoff_content: surrealdb::types::Value,
            assistance_event_record: RecordId,
            assistance_event_content: surrealdb::types::Value,
            receipt_record: RecordId,
            receipt_content: surrealdb::types::Value,
        }

        let bindings = CloudAssistanceTxBindings {
            claim: RecordId::new(CLAIMS_TABLE, request.claim_id.clone()),
            workspace_id: request.workspace_id.clone(),
            wp_id: request.wp_id.clone(),
            mt_id: request.mt_id.clone(),
            lane_id: from_lane.lane_id.clone(),
            actor_id: from_lane.actor_id.clone(),
            basis_event: event_record(&request.fallback_basis_event_id),
            source_component: PARALLEL_SWARM_SOURCE_COMPONENT.to_string(),
            basis_schema_id: PARALLEL_SWARM_CLOUD_FALLBACK_BASIS_SCHEMA_ID.to_string(),
            claim_id_text: request.claim_id.clone(),
            parent_session_id: request.parent_session_id.clone(),
            prompt_sha256: request.prompt_sha256.clone(),
            fallback_reason: request.fallback_reason.as_str().to_string(),
            handoff_event_record: event_record(&handoff_event_id),
            handoff_event_content: event_ledger_write_row(&handoff_event, &handoff_kernel_event)
                .into_value(),
            handoff_record: RecordId::new(HANDOFFS_TABLE, handoff_id.clone()),
            handoff_content: handoff_row.into_value(),
            assistance_event_record: event_record(&cloud_assistance_event_id),
            assistance_event_content: event_ledger_write_row(
                &assistance_event,
                &assistance_kernel_event,
            )
            .into_value(),
            receipt_record: RecordId::new(CLOUD_RECEIPTS_TABLE, receipt_id.clone()),
            receipt_content: receipt_row.into_value(),
        };
        let tx_result: Result<Vec<surrealdb::types::Value>, SurrealStorageError> = self
            .query(
                "BEGIN TRANSACTION; \
                 IF count((SELECT VALUE id FROM $claim \
                     WHERE workspace_id = $workspace_id AND wp_id = $wp_id \
                       AND mt_id = $mt_id AND scope_kind = 'workspace' \
                       AND scope_id = $workspace_id AND lane_id = $lane_id \
                       AND actor_id = $actor_id AND lane_kind = 'cloud' \
                       AND status = 'active' AND released_at_utc = NONE \
                       AND expires_at_utc > time::now())) = 0 { \
                     THROW 'PSR_CLOUD_CLAIM_GONE'; \
                 }; \
                 IF count((SELECT VALUE id FROM $basis_event \
                     WHERE aggregate_type = 'parallel_swarm_cloud_fallback_basis' \
                       AND source_component = $source_component \
                       AND payload.schema_id = $basis_schema_id \
                       AND payload.workspace_id = $workspace_id \
                       AND payload.wp_id = $wp_id AND payload.mt_id = $mt_id \
                       AND payload.claim_id = $claim_id_text \
                       AND payload.parent_session_id = $parent_session_id \
                       AND payload.prompt_sha256 = $prompt_sha256 \
                       AND payload.fallback_reason = $fallback_reason \
                       AND payload.lane.lane_kind IN ['local', 'system'])) = 0 { \
                     THROW 'PSR_CLOUD_BASIS_GONE'; \
                 }; \
                 CREATE $handoff_event_record CONTENT $handoff_event_content; \
                 CREATE $handoff_record CONTENT $handoff_content; \
                 CREATE $assistance_event_record CONTENT $assistance_event_content; \
                 CREATE $receipt_record CONTENT $receipt_content; \
                 COMMIT TRANSACTION;",
                bindings,
            )
            .await;
        if let Err(error) = tx_result {
            let message = error.to_string();
            if message.contains("PSR_CLOUD_CLAIM_GONE") {
                return Err(StateRecoveryError::InvalidInput(
                    "cloud assistance requires an active cloud-owned workspace claim".to_string(),
                ));
            }
            if message.contains("PSR_CLOUD_BASIS_GONE") {
                return Err(StateRecoveryError::InvalidInput(
                    "cloud assistance requires a matching fallback-basis EventLedger proof"
                        .to_string(),
                ));
            }
            return Err(error.into());
        }
        let handoff = RoleMailboxHandoffRecord {
            handoff_id,
            wp_id: request.wp_id.clone(),
            mt_id: request.mt_id.clone(),
            claim_id: Some(request.claim_id.clone()),
            from_lane: from_lane.clone(),
            to_role: request.to_role.clone(),
            mailbox_thread_id: request.mailbox_thread_id.clone(),
            mailbox_message_id: request.mailbox_message_id.clone(),
            status: SwarmReceiptStatus::Progress,
            summary: request.summary.clone(),
            body_sha256: request.body_sha256.clone(),
            event_ledger_event_id: handoff_event_id,
            created_at_utc,
        };
        let receipt = CloudAssistanceReceiptV1 {
            schema_id: PARALLEL_SWARM_CLOUD_ASSISTANCE_SCHEMA_ID.to_string(),
            receipt_id,
            workspace_id: claim.workspace_id,
            wp_id: handoff.wp_id,
            mt_id: handoff.mt_id,
            claim_id: handoff.claim_id.clone().unwrap_or_default(),
            handoff_id: handoff.handoff_id,
            handoff_event_ledger_event_id: handoff.event_ledger_event_id,
            cloud_assistance_event_id,
            fallback_basis_event_id: request.fallback_basis_event_id,
            parent_session_id: request.parent_session_id,
            prompt_sha256: request.prompt_sha256,
            lane_id: handoff.from_lane.lane_id,
            actor_id: handoff.from_lane.actor_id,
            provider: request.from_lane.attribution.provider,
            model_label: request.from_lane.attribution.model_label,
            fallback_reason: request.fallback_reason,
            output_kind: request.output_kind,
            output_sha256: request.output_sha256,
            body_sha256: request.body_sha256,
            output_text: request.output_text,
            target_ref: request.target_ref,
            review_state: "pending_review".to_string(),
            non_authoritative: true,
            requires_promotion: true,
            authority_mutation_allowed: false,
            promotion_event_id: None,
        };
        validate_cloud_assistance_receipt(&receipt).map_err(|errors| {
            StateRecoveryError::InvalidInput(format!(
                "cloud assistance receipt failed validation: {errors:?}"
            ))
        })?;
        Ok(receipt)
    }

    async fn active_cloud_assistance_claim(
        &self,
        request: &CloudAssistanceRequest,
    ) -> StateRecoveryResult<Option<WorkClaimRecord>> {
        #[derive(SurrealValue)]
        struct Bindings {
            claim_id: String,
            workspace_id: String,
            wp_id: String,
            mt_id: String,
            lane_id: String,
            actor_id: String,
        }

        let rows: Vec<ClaimRow> = self
            .query(
                "SELECT * FROM knowledge_agent_worktree_claims \
                 WHERE claim_id = $claim_id AND workspace_id = $workspace_id \
                   AND wp_id = $wp_id AND mt_id = $mt_id \
                   AND scope_kind = 'workspace' AND scope_id = $workspace_id \
                   AND lane_id = $lane_id AND actor_id = $actor_id \
                   AND lane_kind = 'cloud' AND status = 'active' \
                   AND released_at_utc = NONE AND expires_at_utc > time::now();",
                Bindings {
                    claim_id: request.claim_id.clone(),
                    workspace_id: request.workspace_id.clone(),
                    wp_id: request.wp_id.clone(),
                    mt_id: request.mt_id.clone(),
                    lane_id: request.from_lane.lane_id.clone(),
                    actor_id: request.from_lane.actor_id.clone(),
                },
            )
            .await?;
        rows.into_iter().next().map(work_claim_from_row).transpose()
    }

    async fn ensure_cloud_fallback_basis_event(
        &self,
        request: &CloudAssistanceRequest,
    ) -> StateRecoveryResult<()> {
        #[derive(SurrealValue)]
        struct Bindings {
            basis_event: RecordId,
            source_component: String,
            basis_schema_id: String,
            workspace_id: String,
            wp_id: String,
            mt_id: String,
            claim_id_text: String,
            parent_session_id: String,
            prompt_sha256: String,
            fallback_reason: String,
        }

        let found: Vec<String> = self
            .query(
                "SELECT VALUE event_id FROM $basis_event \
                 WHERE aggregate_type = 'parallel_swarm_cloud_fallback_basis' \
                   AND source_component = $source_component \
                   AND payload.schema_id = $basis_schema_id \
                   AND payload.workspace_id = $workspace_id \
                   AND payload.wp_id = $wp_id AND payload.mt_id = $mt_id \
                   AND payload.claim_id = $claim_id_text \
                   AND payload.parent_session_id = $parent_session_id \
                   AND payload.prompt_sha256 = $prompt_sha256 \
                   AND payload.fallback_reason = $fallback_reason \
                   AND payload.lane.lane_kind IN ['local', 'system'];",
                Bindings {
                    basis_event: event_record(&request.fallback_basis_event_id),
                    source_component: PARALLEL_SWARM_SOURCE_COMPONENT.to_string(),
                    basis_schema_id: PARALLEL_SWARM_CLOUD_FALLBACK_BASIS_SCHEMA_ID.to_string(),
                    workspace_id: request.workspace_id.clone(),
                    wp_id: request.wp_id.clone(),
                    mt_id: request.mt_id.clone(),
                    claim_id_text: request.claim_id.clone(),
                    parent_session_id: request.parent_session_id.clone(),
                    prompt_sha256: request.prompt_sha256.clone(),
                    fallback_reason: request.fallback_reason.as_str().to_string(),
                },
            )
            .await?;
        if found.is_empty() {
            Err(StateRecoveryError::InvalidInput(
                "cloud assistance requires a matching fallback-basis EventLedger proof".to_string(),
            ))
        } else {
            Ok(())
        }
    }

    pub async fn record_checkpoint(
        &self,
        request: RecoveryCheckpointRequest,
    ) -> StateRecoveryResult<RecoveryCheckpointRecord> {
        require_capability(&request.lane, AgentCapability::RecordCheckpoint)?;
        let checkpoint_id = format!("PSR-CHKPT-{}", Uuid::now_v7());
        let payload_bytes = serde_json::to_vec(&request.payload)?;
        let payload_sha256 = sha256_hex(&payload_bytes);
        let resume_pointer = serde_json::to_value(&request.resume_pointer)?;
        let lane = request.lane.scrubbed_for_persistence();
        let created_at_utc = Utc::now();
        let event = Self::build_event(
            KernelEventType::KnowledgeCrdtCheckpointRecorded,
            "parallel_swarm_checkpoint",
            &checkpoint_id,
            &lane,
            &request.session_id,
            json!({
                "schema_id": "hsk.parallel_swarm.checkpoint@1",
                "checkpoint_id": checkpoint_id,
                "workspace_id": request.workspace_id,
                "wp_id": request.wp_id,
                "mt_id": request.mt_id,
                "claim_id": request.claim_id,
                "mailbox_handoff_id": request.mailbox_handoff_id,
                "navigation_command_id": request.navigation_command_id,
                "resume_pointer": resume_pointer,
                "payload_sha256": payload_sha256,
                "compaction_reason": request.compaction_reason,
                "git_head": request.git_head,
            }),
        )?;
        let kernel_event = KernelEvent::from_new(event.clone());
        let event_id = kernel_event.event_id.clone();
        let row = CheckpointRow {
            checkpoint_id: checkpoint_id.clone(),
            lane_id: lane.lane_id.clone(),
            actor_id: lane.actor_id.clone(),
            lane_kind: lane.lane_kind.as_str().to_string(),
            attribution_jsonb: serde_json::to_value(&lane.attribution)?,
            session_id: request.session_id.clone(),
            workspace_id: request.workspace_id.clone(),
            wp_id: request.wp_id.clone(),
            mt_id: request.mt_id.clone(),
            claim_id: request
                .claim_id
                .as_ref()
                .map(|claim_id| RecordId::new(CLAIMS_TABLE, claim_id.clone())),
            mailbox_handoff_id: request
                .mailbox_handoff_id
                .as_ref()
                .map(|handoff_id| RecordId::new(HANDOFFS_TABLE, handoff_id.clone())),
            navigation_command_id: request.navigation_command_id.clone(),
            resume_pointer_jsonb: serde_json::to_value(&request.resume_pointer)?,
            touched_files_jsonb: request.touched_files.clone(),
            tests_jsonb: request.tests.clone(),
            hbr_rows_jsonb: request.hbr_rows.clone(),
            next_step_context: request.next_step_context.clone(),
            payload_jsonb: request.payload.clone(),
            payload_sha256: payload_sha256.clone(),
            compaction_reason: request.compaction_reason.clone(),
            git_head: request.git_head.clone(),
            event_ledger_event_id: event_record(&event_id),
            created_at_utc,
        };
        let bindings = CreateRowWithEventBindings {
            event_record: event_record(&event_id),
            event_content: event_ledger_write_row(&event, &kernel_event).into_value(),
            record: RecordId::new(CHECKPOINTS_TABLE, checkpoint_id.clone()),
            content: row.into_value(),
        };
        let _: Vec<surrealdb::types::Value> =
            self.query(CREATE_ROW_WITH_EVENT_QUERY, bindings).await?;
        Ok(RecoveryCheckpointRecord {
            checkpoint_id,
            lane,
            session_id: request.session_id,
            workspace_id: request.workspace_id,
            wp_id: request.wp_id,
            mt_id: request.mt_id,
            claim_id: request.claim_id,
            mailbox_handoff_id: request.mailbox_handoff_id,
            navigation_command_id: request.navigation_command_id,
            resume_pointer: request.resume_pointer,
            touched_files: request.touched_files,
            tests: request.tests,
            hbr_rows: request.hbr_rows,
            next_step_context: request.next_step_context,
            payload: request.payload,
            payload_sha256,
            compaction_reason: request.compaction_reason,
            git_head: request.git_head,
            event_ledger_event_id: event_id,
            created_at_utc,
        })
    }

    pub async fn recover_from_checkpoint(
        &self,
        checkpoint_id: &str,
        new_lane: AgentLaneIdentity,
        new_session_id: &str,
    ) -> StateRecoveryResult<RecoveredCheckpoint> {
        require_capability(&new_lane, AgentCapability::RecordCheckpoint)?;
        let rows: Vec<CheckpointRow> = self
            .query(
                "SELECT * FROM knowledge_agent_state_recovery_checkpoints \
                 WHERE checkpoint_id = $checkpoint_id;",
                CheckpointIdBinding {
                    checkpoint_id: checkpoint_id.to_string(),
                },
            )
            .await?;
        let checkpoint = rows
            .into_iter()
            .next()
            .map(checkpoint_from_row)
            .transpose()?
            .ok_or_else(|| StateRecoveryError::CheckpointNotFound(checkpoint_id.to_string()))?;
        let found = sha256_hex(&serde_json::to_vec(&checkpoint.payload)?);
        if found != checkpoint.payload_sha256 {
            return Err(StateRecoveryError::PayloadHashMismatch {
                checkpoint_id: checkpoint.checkpoint_id.clone(),
                expected: checkpoint.payload_sha256.clone(),
                found,
            });
        }
        let receipt_id = format!("PSR-RECOVERY-{}", Uuid::now_v7());
        let new_lane = new_lane.scrubbed_for_persistence();
        let recovered_at_utc = Utc::now();
        let event = Self::build_event(
            KernelEventType::KnowledgeCrdtRecoveryReceiptRecorded,
            "parallel_swarm_recovery",
            &receipt_id,
            &new_lane,
            new_session_id,
            json!({
                "schema_id": "hsk.parallel_swarm.recovery_receipt@1",
                "receipt_id": receipt_id,
                "checkpoint_id": checkpoint.checkpoint_id,
                "prior_session_id": checkpoint.session_id,
                "new_session_id": new_session_id,
                "resume_pointer": checkpoint.resume_pointer,
            }),
        )?;
        let kernel_event = KernelEvent::from_new(event.clone());
        let event_id = kernel_event.event_id.clone();
        let row = RecoveryReceiptRow {
            receipt_id: receipt_id.clone(),
            checkpoint_id: RecordId::new(CHECKPOINTS_TABLE, checkpoint.checkpoint_id.clone()),
            prior_session_id: checkpoint.session_id.clone(),
            new_session_id: new_session_id.to_string(),
            new_lane_id: new_lane.lane_id.clone(),
            new_actor_id: new_lane.actor_id.clone(),
            new_lane_kind: new_lane.lane_kind.as_str().to_string(),
            new_attribution_jsonb: serde_json::to_value(&new_lane.attribution)?,
            resume_pointer_jsonb: serde_json::to_value(&checkpoint.resume_pointer)?,
            event_ledger_event_id: event_record(&event_id),
            recovered_at_utc,
        };
        let bindings = CreateRowWithEventBindings {
            event_record: event_record(&event_id),
            event_content: event_ledger_write_row(&event, &kernel_event).into_value(),
            record: RecordId::new(RECEIPTS_TABLE, receipt_id.clone()),
            content: row.into_value(),
        };
        let statement = CREATE_ROW_WITH_EVENT_QUERY;
        #[cfg(feature = "test-utils")]
        let statement = if self
            .take_test_failpoint(StateRecoveryTestFailpoint::RecoveryAfterEventBeforeAuthority)
        {
            CREATE_ROW_WITH_EVENT_FAIL_AFTER_EVENT_QUERY
        } else {
            statement
        };
        let _: Vec<surrealdb::types::Value> = self.query(statement, bindings).await?;
        let receipt = RecoveryReceiptRecord {
            receipt_id,
            checkpoint_id: checkpoint.checkpoint_id.clone(),
            prior_session_id: checkpoint.session_id.clone(),
            new_session_id: new_session_id.to_string(),
            new_lane,
            resume_pointer: checkpoint.resume_pointer.clone(),
            event_ledger_event_id: event_id,
            recovered_at_utc,
        };
        Ok(RecoveredCheckpoint {
            resume_pointer: checkpoint.resume_pointer.clone(),
            checkpoint,
            receipt,
        })
    }

    pub async fn build_handoff_compression_template(
        &self,
        request: HandoffCompressionRequest,
    ) -> StateRecoveryResult<HandoffCompressionTemplateV1> {
        require_capability(&request.requested_by_lane, AgentCapability::NavigateBackend)?;
        ensure_safe_token("checkpoint_id", &request.checkpoint_id)?;
        let max_chars = bounded_handoff_body_chars(request.max_chars)?;
        let row: CheckpointRow = self
            .query_first(
                "SELECT * FROM knowledge_agent_state_recovery_checkpoints \
                 WHERE checkpoint_id = $checkpoint_id;",
                CheckpointIdBinding {
                    checkpoint_id: request.checkpoint_id.clone(),
                },
            )
            .await?
            .ok_or_else(|| StateRecoveryError::CheckpointNotFound(request.checkpoint_id.clone()))?;
        let checkpoint = checkpoint_from_row(row)?;
        let found = sha256_hex(&serde_json::to_vec(&checkpoint.payload)?);
        if found != checkpoint.payload_sha256 {
            return Err(StateRecoveryError::PayloadHashMismatch {
                checkpoint_id: checkpoint.checkpoint_id.clone(),
                expected: checkpoint.payload_sha256.clone(),
                found,
            });
        }
        ensure_handoff_checkpoint_metadata_safe(&checkpoint)?;

        let mut warnings = Vec::new();
        let body = compressed_handoff_body(&checkpoint, max_chars as usize, &mut warnings)?;
        let body_sha256 = sha256_hex(body.as_bytes());
        let template = HandoffCompressionTemplateV1 {
            schema_id: PARALLEL_SWARM_HANDOFF_COMPRESSION_SCHEMA_ID.to_string(),
            checkpoint_id: checkpoint.checkpoint_id.clone(),
            workspace_id: checkpoint.workspace_id.clone(),
            wp_id: checkpoint.wp_id.clone(),
            mt_id: checkpoint.mt_id.clone(),
            source_session_id: checkpoint.session_id.clone(),
            source_lane_id: checkpoint.lane.lane_id.clone(),
            source_actor_id: checkpoint.lane.actor_id.clone(),
            source_lane_kind: checkpoint.lane.lane_kind.as_str().to_string(),
            resume_pointer: checkpoint.resume_pointer.clone(),
            git_head: checkpoint.git_head.clone(),
            payload_sha256: checkpoint.payload_sha256.clone(),
            body_sha256,
            body,
            omitted_inputs: handoff_omitted_inputs(),
            source_refs: handoff_source_refs(&checkpoint),
            warnings,
            generated_at_utc: Utc::now(),
        };
        validate_handoff_compression_template(&template).map_err(|errors| {
            StateRecoveryError::InvalidInput(format!(
                "handoff compression template failed validation: {errors:?}"
            ))
        })?;
        Ok(template)
    }

    pub async fn enqueue_indexing_lease(
        &self,
        request: IndexingLeaseRequest,
    ) -> StateRecoveryResult<IndexingLeaseRecord> {
        validate_ttl(request.ttl_seconds)?;
        validate_quiet_background_policy(QuietBackgroundWorkKind::Indexing, &request.quiet_policy)?;
        require_capability(&request.lane, AgentCapability::WriteLocalIndex)?;
        self.reclaim_orphaned_indexing_leases().await?;
        let active = self.active_index_writer_for_scope(&request.scope).await?;
        let queued_ahead = if active.is_none() {
            self.queued_index_writer_for_scope(&request.scope).await?
        } else {
            None
        };
        let (status, blocked_by) = if let Some(record) = active {
            (IndexLeaseStatus::Queued, Some(record.lease_id))
        } else if let Some(record) = queued_ahead {
            (IndexLeaseStatus::Queued, Some(record.lease_id))
        } else {
            (IndexLeaseStatus::Acquired, None)
        };
        match self
            .insert_indexing_lease_outcome(&request, status, blocked_by)
            .await
        {
            Ok(record) => Ok(record),
            Err(StateRecoveryError::Surreal(error))
                if is_unique_violation(&error, "ux_parallel_indexing_lease_queue_active_scope")
                    && status == IndexLeaseStatus::Acquired =>
            {
                let active = self.active_index_writer_for_scope(&request.scope).await?;
                let queued_ahead = if active.is_none() {
                    self.queued_index_writer_for_scope(&request.scope).await?
                } else {
                    None
                };
                let (retry_status, retry_blocked_by) = if let Some(active) = active {
                    (IndexLeaseStatus::Queued, Some(active.lease_id))
                } else if let Some(queued_ahead) = queued_ahead {
                    (IndexLeaseStatus::Queued, Some(queued_ahead.lease_id))
                } else {
                    (IndexLeaseStatus::Acquired, None)
                };
                self.insert_indexing_lease_outcome(&request, retry_status, retry_blocked_by)
                    .await
            }
            Err(error) => Err(error),
        }
    }

    pub async fn try_acquire_indexing_lease(
        &self,
        request: IndexingLeaseRequest,
    ) -> StateRecoveryResult<Option<IndexingLeaseRecord>> {
        validate_ttl(request.ttl_seconds)?;
        validate_quiet_background_policy(QuietBackgroundWorkKind::Indexing, &request.quiet_policy)?;
        require_capability(&request.lane, AgentCapability::WriteLocalIndex)?;
        self.reclaim_orphaned_indexing_leases().await?;
        if self
            .active_index_writer_for_scope(&request.scope)
            .await?
            .is_some()
        {
            return Ok(None);
        }
        if self
            .queued_index_writer_for_scope(&request.scope)
            .await?
            .is_some()
        {
            return Ok(None);
        }
        match self
            .insert_indexing_lease_outcome(&request, IndexLeaseStatus::Acquired, None)
            .await
        {
            Ok(record) => Ok(Some(record)),
            Err(StateRecoveryError::Surreal(error))
                if is_unique_violation(&error, "ux_parallel_indexing_lease_queue_active_scope") =>
            {
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }

    async fn insert_indexing_lease_outcome(
        &self,
        request: &IndexingLeaseRequest,
        status: IndexLeaseStatus,
        blocked_by: Option<String>,
    ) -> StateRecoveryResult<IndexingLeaseRecord> {
        let lease_id = format!("PSR-IDXLEASE-{}", Uuid::now_v7());
        let persistent_lane = request.lane.scrubbed_for_persistence();
        let now = Utc::now();
        let (acquired_at_utc, expires_at_utc) = if status == IndexLeaseStatus::Acquired {
            (
                Some(now),
                Some(now + ChronoDuration::seconds(request.ttl_seconds)),
            )
        } else {
            (None, None)
        };
        let event = Self::build_event(
            match status {
                IndexLeaseStatus::Acquired => KernelEventType::KnowledgeIndexRunStarted,
                IndexLeaseStatus::Queued => KernelEventType::SessionQueued,
                _ => KernelEventType::KnowledgeIndexRunStarted,
            },
            "parallel_indexing_lease",
            &lease_id,
            &persistent_lane,
            &request.session_id,
            json!({
                "schema_id": "hsk.parallel_swarm.indexing_lease@1",
                "lease_id": lease_id,
                "workspace_id": &request.workspace_id,
                "wp_id": &request.wp_id,
                "mt_id": &request.mt_id,
                "scope": &request.scope,
                "index_run_id": &request.index_run_id,
                "status": status,
                "blocked_by_lease_id": blocked_by.as_deref(),
                "quiet_policy": &request.quiet_policy,
            }),
        )?;
        let kernel_event = KernelEvent::from_new(event.clone());
        let event_id = kernel_event.event_id.clone();
        let row = LeaseRow {
            lease_id: lease_id.clone(),
            workspace_id: request.workspace_id.clone(),
            wp_id: request.wp_id.clone(),
            mt_id: request.mt_id.clone(),
            scope_kind: request.scope.kind_str().to_string(),
            scope_id: request.scope.scope_id(),
            lane_id: persistent_lane.lane_id.clone(),
            actor_id: persistent_lane.actor_id.clone(),
            lane_kind: persistent_lane.lane_kind.as_str().to_string(),
            attribution_jsonb: serde_json::to_value(&persistent_lane.attribution)?,
            session_id: request.session_id.clone(),
            index_run_id: request.index_run_id.clone(),
            priority: i64::from(request.priority),
            ttl_seconds: request.ttl_seconds,
            status: status.as_str().to_string(),
            blocked_by_lease_id: blocked_by,
            enqueued_at_utc: now,
            acquired_at_utc,
            expires_at_utc,
            completed_at_utc: None,
            event_ledger_event_id: Some(event_record(&event_id)),
            quiet_policy_jsonb: serde_json::to_value(&request.quiet_policy)?,
        };
        let bindings = CreateRowWithEventBindings {
            event_record: event_record(&event_id),
            event_content: event_ledger_write_row(&event, &kernel_event).into_value(),
            record: RecordId::new(LEASES_TABLE, lease_id),
            content: row.clone().into_value(),
        };
        let _: Vec<surrealdb::types::Value> =
            self.query(CREATE_ROW_WITH_EVENT_QUERY, bindings).await?;
        index_lease_from_row(row)
    }

    pub async fn active_index_writer_for_scope(
        &self,
        scope: &ClaimScope,
    ) -> StateRecoveryResult<Option<IndexingLeaseRecord>> {
        let row: Option<LeaseRow> = self
            .query_first(
                "SELECT * FROM knowledge_parallel_indexing_lease_queue \
                 WHERE scope_kind = $scope_kind AND scope_id = $scope_id \
                   AND status = 'acquired' AND expires_at_utc > time::now() \
                 ORDER BY acquired_at_utc ASC LIMIT 1;",
                scope_binding(scope),
            )
            .await?;
        row.map(index_lease_from_row).transpose()
    }

    async fn queued_index_writer_for_scope(
        &self,
        scope: &ClaimScope,
    ) -> StateRecoveryResult<Option<IndexingLeaseRecord>> {
        let row: Option<LeaseRow> = self
            .query_first(
                "SELECT * FROM knowledge_parallel_indexing_lease_queue \
                 WHERE scope_kind = $scope_kind AND scope_id = $scope_id \
                   AND status = 'queued' \
                 ORDER BY priority DESC, enqueued_at_utc ASC, lease_id ASC LIMIT 1;",
                scope_binding(scope),
            )
            .await?;
        row.map(index_lease_from_row).transpose()
    }

    pub async fn complete_indexing_lease(
        &self,
        lease_id: &str,
        lane: &AgentLaneIdentity,
    ) -> StateRecoveryResult<bool> {
        let rows: Vec<LeaseRow> = self
            .query(
                "UPDATE $record SET status = 'completed', \
                 completed_at_utc = $completed_at_utc \
                 WHERE actor_id = $actor_id AND status = 'acquired' RETURN AFTER;",
                RecordActorBinding {
                    record: RecordId::new(LEASES_TABLE, lease_id.to_string()),
                    actor_id: lane.actor_id.clone(),
                    completed_at_utc: Utc::now(),
                },
            )
            .await?;
        Ok(!rows.is_empty())
    }

    pub async fn acquire_next_indexing_lease(
        &self,
        scope: &ClaimScope,
    ) -> StateRecoveryResult<Option<IndexingLeaseRecord>> {
        if self.active_index_writer_for_scope(scope).await?.is_some() {
            return Ok(None);
        }
        let Some(candidate) = self.queued_index_writer_for_scope(scope).await? else {
            return Ok(None);
        };
        let acquired_at_utc = Utc::now();
        let rows: Vec<LeaseRow> = self
            .query(
                "UPDATE $record SET status = 'acquired', blocked_by_lease_id = NONE, \
                 acquired_at_utc = $acquired_at_utc, expires_at_utc = $expires_at_utc \
                 WHERE status = 'queued' AND array::len((SELECT VALUE id \
                   FROM knowledge_parallel_indexing_lease_queue \
                   WHERE scope_kind = $scope_kind AND scope_id = $scope_id \
                     AND status = 'acquired' AND expires_at_utc > time::now())) = 0 \
                 RETURN AFTER;",
                AcquireLeaseBinding {
                    record: RecordId::new(LEASES_TABLE, candidate.lease_id.clone()),
                    scope_kind: scope.kind_str().to_string(),
                    scope_id: scope.scope_id(),
                    acquired_at_utc,
                    expires_at_utc: acquired_at_utc
                        + ChronoDuration::seconds(candidate.ttl_seconds),
                },
            )
            .await?;
        let Some(row) = rows.into_iter().next() else {
            return Ok(None);
        };
        let mut promoted = index_lease_from_row(row)?;
        let event_id = self
            .append_event(
                KernelEventType::KnowledgeIndexRunStarted,
                "parallel_indexing_lease",
                &promoted.lease_id,
                &promoted.lane,
                &promoted.session_id,
                json!({
                    "schema_id": "hsk.parallel_swarm.indexing_lease@1",
                    "lease_id": &promoted.lease_id,
                    "workspace_id": &promoted.workspace_id,
                    "wp_id": &promoted.wp_id,
                    "mt_id": &promoted.mt_id,
                    "scope": &promoted.scope,
                    "index_run_id": &promoted.index_run_id,
                    "status": IndexLeaseStatus::Acquired,
                    "blocked_by_lease_id": Option::<String>::None,
                    "quiet_policy": &promoted.quiet_policy,
                }),
            )
            .await?;
        let _: Vec<LeaseRow> = self
            .query(
                "UPDATE $record SET event_ledger_event_id = $event_record \
                 WHERE status = 'acquired' RETURN AFTER;",
                RecordEventBinding {
                    record: RecordId::new(LEASES_TABLE, promoted.lease_id.clone()),
                    event_record: event_record(&event_id),
                },
            )
            .await?;
        promoted.event_ledger_event_id = event_id;
        Ok(Some(promoted))
    }

    pub async fn reclaim_orphaned_indexing_leases(
        &self,
    ) -> StateRecoveryResult<Vec<IndexingLeaseRecord>> {
        let rows: Vec<LeaseRow> = self
            .query(
                "SELECT * FROM knowledge_parallel_indexing_lease_queue \
                 WHERE status = 'acquired' AND expires_at_utc <= time::now() \
                 ORDER BY acquired_at_utc ASC, lease_id ASC;",
                EmptyBindings {},
            )
            .await?;
        let mut reclaimed = Vec::with_capacity(rows.len());
        for row in rows {
            let candidate = index_lease_from_row(row)?;
            let changed: Vec<LeaseRow> = self
                .query(
                    "UPDATE $record SET status = 'reclaimed', \
                     completed_at_utc = $completed_at_utc \
                     WHERE status = 'acquired' AND expires_at_utc <= time::now() \
                     RETURN AFTER;",
                    ExpiredRecordBinding {
                        record: RecordId::new(LEASES_TABLE, candidate.lease_id.clone()),
                        completed_at_utc: Utc::now(),
                    },
                )
                .await?;
            let Some(row) = changed.into_iter().next() else {
                continue;
            };
            let mut lease = index_lease_from_row(row)?;
            let event_id = self
                .append_event(
                    KernelEventType::KnowledgeIndexRunCancelled,
                    "parallel_indexing_lease",
                    &lease.lease_id,
                    &lease.lane,
                    &lease.session_id,
                    json!({
                        "schema_id": "hsk.parallel_swarm.indexing_lease@1",
                        "lease_id": &lease.lease_id,
                        "workspace_id": &lease.workspace_id,
                        "wp_id": &lease.wp_id,
                        "mt_id": &lease.mt_id,
                        "scope": &lease.scope,
                        "index_run_id": &lease.index_run_id,
                        "status": IndexLeaseStatus::Reclaimed,
                        "blocked_by_lease_id": lease.blocked_by_lease_id.as_deref(),
                        "quiet_policy": &lease.quiet_policy,
                    }),
                )
                .await?;
            let _: Vec<LeaseRow> = self
                .query(
                    "UPDATE $record SET event_ledger_event_id = $event_record \
                     WHERE status = 'reclaimed' RETURN AFTER;",
                    RecordEventBinding {
                        record: RecordId::new(LEASES_TABLE, lease.lease_id.clone()),
                        event_record: event_record(&event_id),
                    },
                )
                .await?;
            lease.event_ledger_event_id = event_id;
            reclaimed.push(lease);
        }
        Ok(reclaimed)
    }

    async fn active_claim_for_scope(
        &self,
        scope: &ClaimScope,
    ) -> StateRecoveryResult<Option<WorkClaimRecord>> {
        let row: Option<ClaimRow> = self
            .query_first(
                "SELECT * FROM knowledge_agent_worktree_claims \
                 WHERE scope_kind = $scope_kind AND scope_id = $scope_id \
                   AND status = 'active' AND released_at_utc = NONE \
                   AND expires_at_utc > time::now() \
                 ORDER BY claimed_at_utc ASC LIMIT 1;",
                scope_binding(scope),
            )
            .await?;
        row.map(work_claim_from_row).transpose()
    }

    pub async fn reclaim_expired_work_claims(
        &self,
        lane: &AgentLaneIdentity,
        session_id: &str,
        reason: &str,
    ) -> StateRecoveryResult<Vec<WorkClaimRecord>> {
        require_capability(lane, AgentCapability::ClaimWorktree)?;
        let reclaimer = lane.scrubbed_for_persistence();
        let rows: Vec<ClaimRow> = self
            .query(
                "SELECT * FROM knowledge_agent_worktree_claims \
                 WHERE status = 'active' AND released_at_utc = NONE \
                   AND expires_at_utc <= time::now() \
                 ORDER BY claimed_at_utc ASC, claim_id ASC;",
                EmptyBindings {},
            )
            .await?;
        let mut reclaimed = Vec::with_capacity(rows.len());
        for row in rows {
            let candidate = work_claim_from_row(row)?;
            let event = Self::build_event(
                KernelEventType::SessionCancelled,
                "parallel_swarm_claim_reclaim",
                &candidate.claim_id,
                &reclaimer,
                session_id,
                json!({
                    "schema_id": "hsk.parallel_swarm.claim_reclaim@1",
                    "claim_id": &candidate.claim_id,
                    "workspace_id": &candidate.workspace_id,
                    "wp_id": &candidate.wp_id,
                    "mt_id": &candidate.mt_id,
                    "scope": &candidate.scope,
                    "prior_lane": &candidate.lane,
                    "reclaimed_by_lane": &reclaimer,
                    "reason": reason,
                }),
            )?;
            let kernel_event = KernelEvent::from_new(event.clone());
            let event_id = kernel_event.event_id.clone();
            let statement = RECLAIM_CLAIM_QUERY;
            #[cfg(feature = "test-utils")]
            let statement = if self
                .take_test_failpoint(StateRecoveryTestFailpoint::ReclaimAfterAuthorityBeforeEvent)
            {
                RECLAIM_CLAIM_FAIL_BEFORE_EVENT_QUERY
            } else {
                statement
            };
            let changed: Option<ClaimRow> = self
                .query_first(
                    statement,
                    ReclaimClaimBinding {
                        record: RecordId::new(CLAIMS_TABLE, candidate.claim_id.clone()),
                        released_at_utc: Utc::now(),
                        reason: reason.to_string(),
                        event_record: event_record(&event_id),
                        event_content: event_ledger_write_row(&event, &kernel_event).into_value(),
                    },
                )
                .await?;
            if let Some(row) = changed {
                reclaimed.push(work_claim_from_row(row)?);
            }
        }
        Ok(reclaimed)
    }

    #[cfg(feature = "test-utils")]
    pub async fn corrupt_checkpoint_payload_for_test(
        &self,
        checkpoint_id: &str,
        payload: Value,
    ) -> StateRecoveryResult<()> {
        let changed: Vec<CheckpointRow> = self
            .query(
                "UPDATE $record SET payload_jsonb = $payload RETURN AFTER;",
                CorruptCheckpointPayloadBinding {
                    record: RecordId::new(CHECKPOINTS_TABLE, checkpoint_id.to_string()),
                    payload,
                },
            )
            .await?;
        if changed.len() != 1 {
            return Err(StateRecoveryError::CheckpointNotFound(
                checkpoint_id.to_string(),
            ));
        }
        Ok(())
    }

    fn build_event(
        event_type: KernelEventType,
        aggregate_type: &str,
        aggregate_id: &str,
        lane: &AgentLaneIdentity,
        session_id: &str,
        payload: Value,
    ) -> StateRecoveryResult<NewKernelEvent> {
        NewKernelEvent::builder(
            format!("KTR-PSR-{aggregate_id}"),
            session_id.to_string(),
            event_type,
            lane.to_kernel_actor(),
        )
        .aggregate(aggregate_type.to_string(), aggregate_id.to_string())
        .idempotency_key(format!(
            "psr:{aggregate_type}:{aggregate_id}:{}",
            Uuid::now_v7()
        ))
        .correlation_id(aggregate_id.to_string())
        .source_component("parallel_swarm_state_recovery")
        .payload(payload)
        .build()
        .map_err(|error| StateRecoveryError::Kernel(error.to_string()))
    }

    async fn append_event(
        &self,
        event_type: KernelEventType,
        aggregate_type: &str,
        aggregate_id: &str,
        lane: &AgentLaneIdentity,
        session_id: &str,
        payload: Value,
    ) -> StateRecoveryResult<String> {
        let event = Self::build_event(
            event_type,
            aggregate_type,
            aggregate_id,
            lane,
            session_id,
            payload,
        )?;
        let kernel_event = KernelEvent::from_new(event.clone());
        let event_id = kernel_event.event_id.clone();
        let created: Vec<EventLedgerWriteRow> = self
            .query(
                "CREATE $event_record CONTENT $event_content RETURN AFTER;",
                CreateEventBinding {
                    event_record: event_record(&event_id),
                    event_content: event_ledger_write_row(&event, &kernel_event).into_value(),
                },
            )
            .await?;
        if created.len() != 1 {
            return Err(StateRecoveryError::InvalidInput(format!(
                "EventLedger append for {event_id} returned {} rows",
                created.len()
            )));
        }
        Ok(event_id)
    }
}

pub fn validate_swarm_dashboard_projection(
    projection: &ParallelSwarmDashboardProjectionV1,
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    if projection.schema_id != PARALLEL_SWARM_DASHBOARD_SCHEMA_ID {
        errors.push("schema_id must be hsk.parallel_swarm.dashboard_projection@1".to_string());
    }
    let contract = &projection.projection_contract;
    if !contract.projection_only {
        errors.push("projection contract must set projection_only=true".to_string());
    }
    if contract.authority_mutation_allowed {
        errors.push("projection contract must set authority_mutation_allowed=false".to_string());
    }
    if contract.ui_state_authoritative {
        errors.push("projection contract must set ui_state_authoritative=false".to_string());
    }
    if contract.source_component != PARALLEL_SWARM_SOURCE_COMPONENT {
        errors
            .push("projection source_component must be parallel_swarm_state_recovery".to_string());
    }
    for expected in PARALLEL_SWARM_DASHBOARD_SOURCE_TABLES {
        if !contract.source_tables.iter().any(|table| table == expected) {
            errors.push(format!(
                "projection contract missing source table {expected}"
            ));
        }
    }
    for expected in PARALLEL_SWARM_DASHBOARD_EVENT_AGGREGATES {
        if !contract
            .source_event_aggregates
            .iter()
            .any(|aggregate| aggregate == expected)
        {
            errors.push(format!(
                "projection contract missing source event aggregate {expected}"
            ));
        }
    }
    if projection.totals.warnings != projection.warnings.len() as i64 {
        errors.push("totals.warnings must match warnings length".to_string());
    }
    if projection.source_watermark.event_count != projection.source_watermark.events.len() as i64 {
        errors.push(
            "source_watermark.event_count must match source_watermark.events length".to_string(),
        );
    }
    if !projection.source_watermark.missing_event_refs.is_empty() {
        errors.push(
            "source_watermark.missing_event_refs must be empty for a valid projection".to_string(),
        );
    }
    let mut watermark_events = BTreeMap::<String, &SwarmDashboardEventRefV1>::new();
    let mut aggregate_counts = BTreeMap::<String, i64>::new();
    for event in &projection.source_watermark.events {
        if event.source_component != PARALLEL_SWARM_SOURCE_COMPONENT {
            errors.push(format!(
                "watermark event {} has invalid source_component",
                event.event_id
            ));
        }
        if watermark_events
            .insert(event.event_id.clone(), event)
            .is_some()
        {
            errors.push(format!(
                "source_watermark.events contains duplicate event {}",
                event.event_id
            ));
        }
        *aggregate_counts
            .entry(event.aggregate_type.clone())
            .or_insert(0) += 1;
    }
    let declared_aggregate_counts = projection
        .source_watermark
        .aggregate_counts
        .iter()
        .map(|row| (row.aggregate_type.clone(), row.count))
        .collect::<BTreeMap<_, _>>();
    if aggregate_counts != declared_aggregate_counts {
        errors.push(
            "source_watermark.aggregate_counts must match source_watermark.events".to_string(),
        );
    }

    validate_source_refs(
        &mut errors,
        "claim",
        "knowledge_agent_worktree_claims",
        &["parallel_swarm_claim", "parallel_swarm_claim_reclaim"],
        &watermark_events,
        projection
            .claims
            .iter()
            .map(|row| (row.claim_id.as_str(), row.source_refs.as_slice())),
    );
    validate_source_refs(
        &mut errors,
        "mailbox_handoff",
        "knowledge_agent_role_mailbox_handoffs",
        &["parallel_swarm_handoff"],
        &watermark_events,
        projection
            .mailbox_handoffs
            .iter()
            .map(|row| (row.handoff_id.as_str(), row.source_refs.as_slice())),
    );
    validate_source_refs(
        &mut errors,
        "checkpoint",
        "knowledge_agent_state_recovery_checkpoints",
        &["parallel_swarm_checkpoint"],
        &watermark_events,
        projection
            .recovery_checkpoints
            .iter()
            .map(|row| (row.checkpoint_id.as_str(), row.source_refs.as_slice())),
    );
    validate_source_refs(
        &mut errors,
        "recovery_receipt",
        "knowledge_agent_recovery_receipts",
        &["parallel_swarm_recovery"],
        &watermark_events,
        projection
            .recovery_receipts
            .iter()
            .map(|row| (row.receipt_id.as_str(), row.source_refs.as_slice())),
    );
    validate_source_refs(
        &mut errors,
        "indexing_lease",
        "knowledge_parallel_indexing_lease_queue",
        &["parallel_indexing_lease"],
        &watermark_events,
        projection
            .indexing_leases
            .iter()
            .map(|row| (row.lease_id.as_str(), row.source_refs.as_slice())),
    );
    validate_source_refs(
        &mut errors,
        "quiet_background_work",
        "knowledge_agent_quiet_background_work",
        &["parallel_swarm_quiet_background_work"],
        &watermark_events,
        projection
            .quiet_background_work
            .iter()
            .map(|row| (row.receipt_id.as_str(), row.source_refs.as_slice())),
    );

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn validate_cloud_assistance_receipt(
    receipt: &CloudAssistanceReceiptV1,
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    if receipt.schema_id != PARALLEL_SWARM_CLOUD_ASSISTANCE_SCHEMA_ID {
        errors.push("schema_id must be hsk.parallel_swarm.cloud_assistance@1".to_string());
    }
    if receipt.receipt_id.trim().is_empty() || !receipt.receipt_id.starts_with("PSR-CLOUD-") {
        errors.push("receipt_id must be a PSR-CLOUD receipt id".to_string());
    }
    if receipt.handoff_id.trim().is_empty() || !receipt.handoff_id.starts_with("PSR-HANDOFF-") {
        errors.push("handoff_id must be a PSR-HANDOFF id".to_string());
    }
    if !receipt.handoff_event_ledger_event_id.starts_with("KE-") {
        errors.push("handoff_event_ledger_event_id must be an EventLedger id".to_string());
    }
    if !receipt.cloud_assistance_event_id.starts_with("KE-") {
        errors.push("cloud_assistance_event_id must be an EventLedger id".to_string());
    }
    if !receipt.fallback_basis_event_id.starts_with("KE-") {
        errors.push("fallback_basis_event_id must be an EventLedger id".to_string());
    }
    if receipt.parent_session_id.trim().is_empty() {
        errors.push("parent_session_id must be present for cloud assistance".to_string());
    }
    if receipt.prompt_sha256.len() != 64
        || !receipt
            .prompt_sha256
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, 'a'..='f'))
    {
        errors.push("prompt_sha256 must be lowercase sha256 hex".to_string());
    }
    if receipt.provider == Some(ModelProviderKind::LocalRuntime) {
        errors.push("cloud assistance provider must not be local_runtime".to_string());
    }
    if receipt.model_label.trim().is_empty() {
        errors.push("model_label must be present for cloud assistance".to_string());
    }
    if receipt.output_sha256.len() != 64
        || !receipt
            .output_sha256
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, 'a'..='f'))
    {
        errors.push("output_sha256 must be lowercase sha256 hex".to_string());
    }
    if receipt.body_sha256.len() != 64
        || !receipt
            .body_sha256
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, 'a'..='f'))
    {
        errors.push("body_sha256 must be lowercase sha256 hex".to_string());
    }
    if receipt.output_text.trim().is_empty() {
        errors.push("output_text must be present for review".to_string());
    }
    if receipt.review_state != "pending_review" {
        errors.push("cloud assistance review_state must be pending_review".to_string());
    }
    if !receipt.non_authoritative {
        errors.push("cloud assistance receipt must be non_authoritative=true".to_string());
    }
    if !receipt.requires_promotion {
        errors.push("cloud assistance receipt must require promotion".to_string());
    }
    if receipt.authority_mutation_allowed {
        errors.push("cloud assistance must not allow authority mutation".to_string());
    }
    if receipt.promotion_event_id.is_some() {
        errors.push("cloud assistance receipt must not carry a promotion_event_id".to_string());
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn validate_handoff_compression_template(
    template: &HandoffCompressionTemplateV1,
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    if template.schema_id != PARALLEL_SWARM_HANDOFF_COMPRESSION_SCHEMA_ID {
        errors.push("schema_id must be hsk.parallel_swarm.handoff_compression@1".to_string());
    }
    if !template.checkpoint_id.starts_with("PSR-CHKPT-") {
        errors.push("checkpoint_id must be a PSR-CHKPT id".to_string());
    }
    for (field, value) in [
        ("workspace_id", template.workspace_id.as_str()),
        ("wp_id", template.wp_id.as_str()),
        ("mt_id", template.mt_id.as_str()),
        ("source_session_id", template.source_session_id.as_str()),
        ("source_lane_id", template.source_lane_id.as_str()),
        ("source_actor_id", template.source_actor_id.as_str()),
        ("source_lane_kind", template.source_lane_kind.as_str()),
        ("git_head", template.git_head.as_str()),
    ] {
        if value.trim().is_empty() {
            errors.push(format!("{field} must be present"));
        }
    }
    if !is_lower_sha256(&template.payload_sha256) {
        errors.push("payload_sha256 must be lowercase sha256 hex".to_string());
    }
    if !is_lower_sha256(&template.body_sha256) {
        errors.push("body_sha256 must be lowercase sha256 hex".to_string());
    }
    if template.body_sha256 != sha256_hex(template.body.as_bytes()) {
        errors.push("body_sha256 must match body bytes".to_string());
    }
    if template.body.trim().is_empty() || template.body.len() > 20_000 {
        errors.push("body must be non-empty and bounded to 20000 bytes".to_string());
    }
    if !template.body.contains(&template.checkpoint_id)
        || !template.body.contains(&template.mt_id)
        || !template.body.contains("payload_sha256")
    {
        errors
            .push("body must include checkpoint, MT, and payload hash restart anchors".to_string());
    }
    for required in [
        "raw_checkpoint_payload",
        "provider_chat_transcript",
        "full_conversation_history",
    ] {
        if !template
            .omitted_inputs
            .iter()
            .any(|input| input == required)
        {
            errors.push(format!("omitted_inputs must declare {required}"));
        }
    }
    if !template.source_refs.iter().any(|source| {
        source.source_kind == "checkpoint"
            && source.source_id == template.checkpoint_id
            && source
                .event_ledger_event_id
                .as_deref()
                .is_some_and(|event_id| event_id.starts_with("KE-"))
    }) {
        errors.push("source_refs must include checkpoint table/EventLedger authority".to_string());
    }
    if contains_obvious_secret_token(&template.body) {
        errors.push("body must not contain obvious raw secret tokens".to_string());
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn validate_source_refs<'a>(
    errors: &mut Vec<String>,
    row_kind: &str,
    expected_table: &str,
    allowed_event_aggregate_types: &[&str],
    watermark_events: &BTreeMap<String, &SwarmDashboardEventRefV1>,
    rows: impl Iterator<Item = (&'a str, &'a [SwarmDashboardSourceRefV1])>,
) {
    for (row_id, refs) in rows {
        if refs.is_empty() {
            errors.push(format!("{row_kind} {row_id} has no source_refs"));
            continue;
        }
        if !refs
            .iter()
            .any(|source_ref| source_ref.event_ledger_event_id.is_some())
        {
            errors.push(format!("{row_kind} {row_id} has no EventLedger source ref"));
        }
        for source_ref in refs {
            if source_ref.table_name.trim().is_empty() {
                errors.push(format!("{row_kind} {row_id} has empty source table"));
            }
            if source_ref.table_name != expected_table {
                errors.push(format!(
                    "{row_kind} {row_id} source table must be {expected_table}"
                ));
            }
            if source_ref.row_id != row_id {
                errors.push(format!("{row_kind} {row_id} has mismatched source row_id"));
            }
            if source_ref.row_source_ref != format!("surreal://{}/{}", expected_table, row_id) {
                errors.push(format!("{row_kind} {row_id} has mismatched row source ref"));
            }
            if source_ref.row_source_ref.trim().is_empty()
                || !source_ref.row_source_ref.starts_with("surreal://")
            {
                errors.push(format!("{row_kind} {row_id} has invalid row source ref"));
            }
            match (
                source_ref.event_ledger_event_id.as_deref(),
                source_ref.event_source_ref.as_deref(),
            ) {
                (Some(event_id), Some(event_ref))
                    if event_ref == format!("event-ledger://{event_id}") => {}
                (Some(_), _) => {
                    errors.push(format!("{row_kind} {row_id} has invalid event source ref"))
                }
                (None, Some(_)) => {
                    errors.push(format!("{row_kind} {row_id} has dangling event source ref"))
                }
                (None, None) => {}
            }
            match (
                source_ref.event_ledger_event_id.as_deref(),
                source_ref.event_aggregate_type.as_deref(),
                source_ref.event_aggregate_id.as_deref(),
            ) {
                (Some(event_id), Some(aggregate_type), Some(aggregate_id)) => {
                    if !allowed_event_aggregate_types.contains(&aggregate_type) {
                        errors.push(format!(
                            "{row_kind} {row_id} has invalid event aggregate_type"
                        ));
                    }
                    if aggregate_id != row_id {
                        errors.push(format!(
                            "{row_kind} {row_id} has mismatched event aggregate_id"
                        ));
                    }
                    match watermark_events.get(event_id) {
                        Some(event)
                            if event.aggregate_type == aggregate_type
                                && event.aggregate_id == aggregate_id
                                && event.source_component == PARALLEL_SWARM_SOURCE_COMPONENT => {}
                        Some(_) => errors.push(format!(
                            "{row_kind} {row_id} EventLedger watermark aggregate mismatch"
                        )),
                        None => errors.push(format!(
                            "{row_kind} {row_id} EventLedger ref missing from watermark"
                        )),
                    }
                }
                (Some(_), _, _) => errors.push(format!(
                    "{row_kind} {row_id} missing EventLedger aggregate identity"
                )),
                (None, Some(_), _) | (None, _, Some(_)) => errors.push(format!(
                    "{row_kind} {row_id} has aggregate identity without EventLedger ref"
                )),
                (None, None, None) => {}
            }
        }
    }
}

fn swarm_dashboard_projection_contract() -> SwarmDashboardProjectionContractV1 {
    SwarmDashboardProjectionContractV1 {
        projection_only: true,
        authority_mutation_allowed: false,
        ui_state_authoritative: false,
        source_component: PARALLEL_SWARM_SOURCE_COMPONENT.to_string(),
        source_tables: PARALLEL_SWARM_DASHBOARD_SOURCE_TABLES
            .iter()
            .map(|table| (*table).to_string())
            .collect(),
        source_event_aggregates: PARALLEL_SWARM_DASHBOARD_EVENT_AGGREGATES
            .iter()
            .map(|aggregate| (*aggregate).to_string())
            .collect(),
    }
}

fn collect_projection_event_ids(
    claims: &[WorkClaimRecord],
    handoffs: &[RoleMailboxHandoffRecord],
    checkpoints: &[RecoveryCheckpointRecord],
    recovery_receipts: &[RecoveryReceiptRecord],
    indexing_leases: &[IndexingLeaseRecord],
    quiet_work: &[QuietBackgroundWorkRecord],
    out: &mut BTreeSet<String>,
) {
    for claim in claims {
        extend_event_id(out, claim.event_ledger_event_id.as_deref());
        extend_event_id(out, claim.release_event_ledger_event_id.as_deref());
        extend_event_id(out, claim.reclaim_event_ledger_event_id.as_deref());
    }
    for handoff in handoffs {
        extend_event_id(out, Some(&handoff.event_ledger_event_id));
    }
    for checkpoint in checkpoints {
        extend_event_id(out, Some(&checkpoint.event_ledger_event_id));
    }
    for receipt in recovery_receipts {
        extend_event_id(out, Some(&receipt.event_ledger_event_id));
    }
    for lease in indexing_leases {
        extend_event_id(out, Some(&lease.event_ledger_event_id));
    }
    for quiet in quiet_work {
        extend_event_id(out, Some(&quiet.event_ledger_event_id));
    }
}

fn extend_event_id(out: &mut BTreeSet<String>, event_id: Option<&str>) {
    if let Some(event_id) = event_id.filter(|value| !value.trim().is_empty()) {
        out.insert(event_id.to_string());
    }
}

fn dashboard_source_ref(
    table_name: &str,
    row_id: &str,
    event_ledger_event_id: Option<&str>,
    event_aggregate_type: Option<&str>,
    event_aggregate_id: Option<&str>,
) -> SwarmDashboardSourceRefV1 {
    SwarmDashboardSourceRefV1 {
        table_name: table_name.to_string(),
        row_id: row_id.to_string(),
        row_source_ref: format!("surreal://{table_name}/{row_id}"),
        event_ledger_event_id: event_ledger_event_id.map(ToOwned::to_owned),
        event_source_ref: event_ledger_event_id
            .map(|event_id| format!("event-ledger://{event_id}")),
        event_aggregate_type: event_aggregate_type.map(ToOwned::to_owned),
        event_aggregate_id: event_aggregate_id.map(ToOwned::to_owned),
    }
}

fn dashboard_claim_row(
    claim: &WorkClaimRecord,
    generated_at_utc: DateTime<Utc>,
) -> SwarmDashboardClaimRowV1 {
    let mut source_refs = vec![dashboard_source_ref(
        "knowledge_agent_worktree_claims",
        &claim.claim_id,
        claim.event_ledger_event_id.as_deref(),
        claim
            .event_ledger_event_id
            .as_deref()
            .map(|_| "parallel_swarm_claim"),
        claim
            .event_ledger_event_id
            .as_deref()
            .map(|_| claim.claim_id.as_str()),
    )];
    if let Some(event_id) = claim.release_event_ledger_event_id.as_deref() {
        source_refs.push(dashboard_source_ref(
            "knowledge_agent_worktree_claims",
            &claim.claim_id,
            Some(event_id),
            Some("parallel_swarm_claim"),
            Some(&claim.claim_id),
        ));
    }
    if let Some(event_id) = claim.reclaim_event_ledger_event_id.as_deref() {
        source_refs.push(dashboard_source_ref(
            "knowledge_agent_worktree_claims",
            &claim.claim_id,
            Some(event_id),
            Some("parallel_swarm_claim_reclaim"),
            Some(&claim.claim_id),
        ));
    }
    SwarmDashboardClaimRowV1 {
        claim_id: claim.claim_id.clone(),
        wp_id: claim.wp_id.clone(),
        mt_id: claim.mt_id.clone(),
        scope_kind: claim.scope.kind_str().to_string(),
        scope_id: claim.scope.scope_id(),
        lane_id: claim.lane.lane_id.clone(),
        actor_id: claim.lane.actor_id.clone(),
        lane_kind: claim.lane.lane_kind.as_str().to_string(),
        status: claim.status.as_str().to_string(),
        reason: claim.reason.clone(),
        claimed_at_utc: claim.claimed_at_utc,
        expires_at_utc: claim.expires_at_utc,
        released_at_utc: claim.released_at_utc,
        stale: claim.status == ClaimStatus::Active && claim.expires_at_utc <= generated_at_utc,
        source_refs,
    }
}

fn dashboard_handoff_row(handoff: &RoleMailboxHandoffRecord) -> SwarmDashboardHandoffRowV1 {
    SwarmDashboardHandoffRowV1 {
        handoff_id: handoff.handoff_id.clone(),
        wp_id: handoff.wp_id.clone(),
        mt_id: handoff.mt_id.clone(),
        claim_id: handoff.claim_id.clone(),
        from_lane_id: handoff.from_lane.lane_id.clone(),
        from_actor_id: handoff.from_lane.actor_id.clone(),
        from_lane_kind: handoff.from_lane.lane_kind.as_str().to_string(),
        to_role: handoff.to_role.clone(),
        mailbox_thread_id: handoff.mailbox_thread_id.clone(),
        mailbox_message_id: handoff.mailbox_message_id.clone(),
        status: handoff.status.as_str().to_string(),
        summary: handoff.summary.clone(),
        created_at_utc: handoff.created_at_utc,
        source_refs: vec![dashboard_source_ref(
            "knowledge_agent_role_mailbox_handoffs",
            &handoff.handoff_id,
            Some(&handoff.event_ledger_event_id),
            Some("parallel_swarm_handoff"),
            Some(&handoff.handoff_id),
        )],
    }
}

fn dashboard_checkpoint_row(
    checkpoint: &RecoveryCheckpointRecord,
) -> SwarmDashboardCheckpointRowV1 {
    SwarmDashboardCheckpointRowV1 {
        checkpoint_id: checkpoint.checkpoint_id.clone(),
        wp_id: checkpoint.wp_id.clone(),
        mt_id: checkpoint.mt_id.clone(),
        session_id: checkpoint.session_id.clone(),
        lane_id: checkpoint.lane.lane_id.clone(),
        actor_id: checkpoint.lane.actor_id.clone(),
        lane_kind: checkpoint.lane.lane_kind.as_str().to_string(),
        claim_id: checkpoint.claim_id.clone(),
        mailbox_handoff_id: checkpoint.mailbox_handoff_id.clone(),
        navigation_command_id: checkpoint.navigation_command_id.clone(),
        resume_pointer: checkpoint.resume_pointer.clone(),
        payload_sha256: checkpoint.payload_sha256.clone(),
        compaction_reason: checkpoint.compaction_reason.clone(),
        git_head: checkpoint.git_head.clone(),
        created_at_utc: checkpoint.created_at_utc,
        source_refs: vec![dashboard_source_ref(
            "knowledge_agent_state_recovery_checkpoints",
            &checkpoint.checkpoint_id,
            Some(&checkpoint.event_ledger_event_id),
            Some("parallel_swarm_checkpoint"),
            Some(&checkpoint.checkpoint_id),
        )],
    }
}

fn dashboard_recovery_receipt_row(
    receipt: &RecoveryReceiptRecord,
) -> SwarmDashboardRecoveryReceiptRowV1 {
    SwarmDashboardRecoveryReceiptRowV1 {
        receipt_id: receipt.receipt_id.clone(),
        checkpoint_id: receipt.checkpoint_id.clone(),
        prior_session_id: receipt.prior_session_id.clone(),
        new_session_id: receipt.new_session_id.clone(),
        new_lane_id: receipt.new_lane.lane_id.clone(),
        new_actor_id: receipt.new_lane.actor_id.clone(),
        new_lane_kind: receipt.new_lane.lane_kind.as_str().to_string(),
        resume_pointer: receipt.resume_pointer.clone(),
        recovered_at_utc: receipt.recovered_at_utc,
        source_refs: vec![dashboard_source_ref(
            "knowledge_agent_recovery_receipts",
            &receipt.receipt_id,
            Some(&receipt.event_ledger_event_id),
            Some("parallel_swarm_recovery"),
            Some(&receipt.receipt_id),
        )],
    }
}

fn dashboard_indexing_lease_row(lease: &IndexingLeaseRecord) -> SwarmDashboardIndexingLeaseRowV1 {
    SwarmDashboardIndexingLeaseRowV1 {
        lease_id: lease.lease_id.clone(),
        wp_id: lease.wp_id.clone(),
        mt_id: lease.mt_id.clone(),
        scope_kind: lease.scope.kind_str().to_string(),
        scope_id: lease.scope.scope_id(),
        lane_id: lease.lane.lane_id.clone(),
        actor_id: lease.lane.actor_id.clone(),
        lane_kind: lease.lane.lane_kind.as_str().to_string(),
        session_id: lease.session_id.clone(),
        index_run_id: lease.index_run_id.clone(),
        status: lease.status.as_str().to_string(),
        blocked_by_lease_id: lease.blocked_by_lease_id.clone(),
        quiet_policy_ok: validate_quiet_background_policy(
            QuietBackgroundWorkKind::Indexing,
            &lease.quiet_policy,
        )
        .is_ok(),
        source_refs: vec![dashboard_source_ref(
            "knowledge_parallel_indexing_lease_queue",
            &lease.lease_id,
            Some(&lease.event_ledger_event_id),
            Some("parallel_indexing_lease"),
            Some(&lease.lease_id),
        )],
    }
}

fn dashboard_quiet_work_row(quiet: &QuietBackgroundWorkRecord) -> SwarmDashboardQuietWorkRowV1 {
    SwarmDashboardQuietWorkRowV1 {
        receipt_id: quiet.receipt_id.clone(),
        wp_id: quiet.wp_id.clone(),
        mt_id: quiet.mt_id.clone(),
        work_kind: quiet.work_kind.as_str().to_string(),
        subject_id: quiet.subject_id.clone(),
        lane_id: quiet.lane.lane_id.clone(),
        actor_id: quiet.lane.actor_id.clone(),
        lane_kind: quiet.lane.lane_kind.as_str().to_string(),
        session_id: quiet.session_id.clone(),
        evidence_ref: quiet.evidence_ref.clone(),
        quiet_policy_ok: validate_quiet_background_policy(quiet.work_kind, &quiet.policy).is_ok(),
        created_at_utc: quiet.created_at_utc,
        source_refs: vec![dashboard_source_ref(
            "knowledge_agent_quiet_background_work",
            &quiet.receipt_id,
            Some(&quiet.event_ledger_event_id),
            Some("parallel_swarm_quiet_background_work"),
            Some(&quiet.receipt_id),
        )],
    }
}

#[derive(Default)]
struct DashboardLaneAccumulator {
    actor_id: String,
    lane_kind: String,
    attribution_mode: String,
    total_rows: i64,
    active_claims: i64,
    handoffs: i64,
    checkpoints: i64,
    recovery_receipts: i64,
    indexing_leases: i64,
    quiet_background_work: i64,
    source_event_ids: BTreeSet<String>,
}

fn dashboard_lane_rows(
    claims: &[WorkClaimRecord],
    handoffs: &[RoleMailboxHandoffRecord],
    checkpoints: &[RecoveryCheckpointRecord],
    recovery_receipts: &[RecoveryReceiptRecord],
    indexing_leases: &[IndexingLeaseRecord],
    quiet_work: &[QuietBackgroundWorkRecord],
) -> Vec<SwarmDashboardLaneRowV1> {
    let mut lanes = BTreeMap::<String, DashboardLaneAccumulator>::new();
    for claim in claims {
        let lane = lane_accumulator(&mut lanes, &claim.lane);
        lane.total_rows += 1;
        if claim.status == ClaimStatus::Active {
            lane.active_claims += 1;
        }
        extend_event_id(
            &mut lane.source_event_ids,
            claim.event_ledger_event_id.as_deref(),
        );
        extend_event_id(
            &mut lane.source_event_ids,
            claim.release_event_ledger_event_id.as_deref(),
        );
        extend_event_id(
            &mut lane.source_event_ids,
            claim.reclaim_event_ledger_event_id.as_deref(),
        );
    }
    for handoff in handoffs {
        let lane = lane_accumulator(&mut lanes, &handoff.from_lane);
        lane.total_rows += 1;
        lane.handoffs += 1;
        extend_event_id(
            &mut lane.source_event_ids,
            Some(&handoff.event_ledger_event_id),
        );
    }
    for checkpoint in checkpoints {
        let lane = lane_accumulator(&mut lanes, &checkpoint.lane);
        lane.total_rows += 1;
        lane.checkpoints += 1;
        extend_event_id(
            &mut lane.source_event_ids,
            Some(&checkpoint.event_ledger_event_id),
        );
    }
    for receipt in recovery_receipts {
        let lane = lane_accumulator(&mut lanes, &receipt.new_lane);
        lane.total_rows += 1;
        lane.recovery_receipts += 1;
        extend_event_id(
            &mut lane.source_event_ids,
            Some(&receipt.event_ledger_event_id),
        );
    }
    for lease in indexing_leases {
        let lane = lane_accumulator(&mut lanes, &lease.lane);
        lane.total_rows += 1;
        lane.indexing_leases += 1;
        extend_event_id(
            &mut lane.source_event_ids,
            Some(&lease.event_ledger_event_id),
        );
    }
    for quiet in quiet_work {
        let lane = lane_accumulator(&mut lanes, &quiet.lane);
        lane.total_rows += 1;
        lane.quiet_background_work += 1;
        extend_event_id(
            &mut lane.source_event_ids,
            Some(&quiet.event_ledger_event_id),
        );
    }

    lanes
        .into_iter()
        .map(|(lane_id, lane)| SwarmDashboardLaneRowV1 {
            lane_id,
            actor_id: lane.actor_id,
            lane_kind: lane.lane_kind,
            attribution_mode: lane.attribution_mode,
            total_rows: lane.total_rows,
            active_claims: lane.active_claims,
            handoffs: lane.handoffs,
            checkpoints: lane.checkpoints,
            recovery_receipts: lane.recovery_receipts,
            indexing_leases: lane.indexing_leases,
            quiet_background_work: lane.quiet_background_work,
            source_event_ids: lane.source_event_ids.into_iter().collect(),
        })
        .collect()
}

fn lane_accumulator<'a>(
    lanes: &'a mut BTreeMap<String, DashboardLaneAccumulator>,
    lane: &AgentLaneIdentity,
) -> &'a mut DashboardLaneAccumulator {
    lanes
        .entry(lane.lane_id.clone())
        .or_insert_with(|| DashboardLaneAccumulator {
            actor_id: lane.actor_id.clone(),
            lane_kind: lane.lane_kind.as_str().to_string(),
            attribution_mode: attribution_mode_as_str(lane.attribution.mode).to_string(),
            ..DashboardLaneAccumulator::default()
        })
}

fn dashboard_totals(authority: SwarmDashboardAuthorityTotals) -> SwarmDashboardTotalsV1 {
    SwarmDashboardTotalsV1 {
        claims: authority.claims,
        active_claims: authority.active_claims,
        stale_active_claims: authority.stale_active_claims,
        mailbox_handoffs: authority.mailbox_handoffs,
        recovery_checkpoints: authority.recovery_checkpoints,
        recovery_receipts: authority.recovery_receipts,
        indexing_leases: authority.indexing_leases,
        acquired_indexing_leases: authority.acquired_indexing_leases,
        quiet_background_work: authority.quiet_background_work,
        events: authority.events,
        warnings: 0,
        claims_by_status: authority.claims_by_status,
        handoffs_by_status: authority.handoffs_by_status,
        leases_by_status: authority.leases_by_status,
        quiet_work_by_kind: authority.quiet_work_by_kind,
    }
}

fn dashboard_group_count_map(rows: Vec<GroupCountRow>) -> BTreeMap<String, i64> {
    rows.into_iter()
        .map(|row| (row.group_key, row.row_count))
        .collect()
}

fn add_truncation_warning(
    warnings: &mut Vec<SwarmDashboardWarningV1>,
    section: &str,
    returned: usize,
    total: i64,
) {
    if total > returned as i64 {
        warnings.push(SwarmDashboardWarningV1 {
            code: "dashboard_section_truncated".to_string(),
            detail: format!(
                "{section} returned {returned} of {total} durable source row(s); increase limit or use narrower filters to inspect the full set"
            ),
        });
    }
}

fn attribution_mode_as_str(mode: AttributionMode) -> &'static str {
    match mode {
        AttributionMode::Local => "local",
        AttributionMode::Cloud => "cloud",
        AttributionMode::Operator => "operator",
        AttributionMode::System => "system",
    }
}

fn work_claim_from_row(row: ClaimRow) -> StateRecoveryResult<WorkClaimRecord> {
    let event_ledger_event_id = optional_record_key(row.event_ledger_event_id.as_ref())?;
    let release_event_ledger_event_id =
        optional_record_key(row.release_event_ledger_event_id.as_ref())?;
    let reclaim_event_ledger_event_id =
        optional_record_key(row.reclaim_event_ledger_event_id.as_ref())?;
    let scope = scope_from_parts(row.scope_kind, row.scope_id)?;
    let lane = lane_from_parts(
        row.lane_id,
        row.actor_id,
        row.lane_kind,
        row.attribution_jsonb,
    )?;
    Ok(WorkClaimRecord {
        claim_id: row.claim_id,
        workspace_id: row.workspace_id,
        wp_id: row.wp_id,
        mt_id: row.mt_id,
        scope,
        lane,
        session_id: row.session_id,
        status: ClaimStatus::parse(&row.status)?,
        reason: row.reason,
        claimed_at_utc: row.claimed_at_utc,
        expires_at_utc: row.expires_at_utc,
        released_at_utc: row.released_at_utc,
        event_ledger_event_id,
        release_event_ledger_event_id,
        reclaim_event_ledger_event_id,
    })
}

fn mailbox_handoff_from_row(row: HandoffRow) -> StateRecoveryResult<RoleMailboxHandoffRecord> {
    let claim_id = optional_record_key(row.claim_id.as_ref())?;
    let event_ledger_event_id = record_key_string(&row.event_ledger_event_id)?;
    let lane = lane_from_parts(
        row.from_lane_id,
        row.from_actor_id,
        row.from_lane_kind,
        row.from_attribution_jsonb,
    )?;
    Ok(RoleMailboxHandoffRecord {
        handoff_id: row.handoff_id,
        wp_id: row.wp_id,
        mt_id: row.mt_id,
        claim_id,
        from_lane: lane,
        to_role: row.to_role,
        mailbox_thread_id: row.mailbox_thread_id,
        mailbox_message_id: row.mailbox_message_id,
        status: SwarmReceiptStatus::parse(&row.status)?,
        summary: row.summary,
        body_sha256: row.body_sha256,
        event_ledger_event_id,
        created_at_utc: row.created_at_utc,
    })
}

fn checkpoint_from_row(row: CheckpointRow) -> StateRecoveryResult<RecoveryCheckpointRecord> {
    let claim_id = optional_record_key(row.claim_id.as_ref())?;
    let mailbox_handoff_id = optional_record_key(row.mailbox_handoff_id.as_ref())?;
    let event_ledger_event_id = record_key_string(&row.event_ledger_event_id)?;
    let lane = lane_from_parts(
        row.lane_id,
        row.actor_id,
        row.lane_kind,
        row.attribution_jsonb,
    )?;
    let resume_pointer: RecoveryResumePointer = serde_json::from_value(row.resume_pointer_jsonb)?;
    Ok(RecoveryCheckpointRecord {
        checkpoint_id: row.checkpoint_id,
        lane,
        session_id: row.session_id,
        workspace_id: row.workspace_id,
        wp_id: row.wp_id,
        mt_id: row.mt_id,
        claim_id,
        mailbox_handoff_id,
        navigation_command_id: row.navigation_command_id,
        resume_pointer,
        touched_files: row.touched_files_jsonb,
        tests: row.tests_jsonb,
        hbr_rows: row.hbr_rows_jsonb,
        next_step_context: row.next_step_context,
        payload: row.payload_jsonb,
        payload_sha256: row.payload_sha256,
        compaction_reason: row.compaction_reason,
        git_head: row.git_head,
        event_ledger_event_id,
        created_at_utc: row.created_at_utc,
    })
}

fn recovery_receipt_from_row(
    row: RecoveryReceiptRow,
) -> StateRecoveryResult<RecoveryReceiptRecord> {
    let checkpoint_id = record_key_string(&row.checkpoint_id)?;
    let event_ledger_event_id = record_key_string(&row.event_ledger_event_id)?;
    let new_lane = lane_from_parts(
        row.new_lane_id,
        row.new_actor_id,
        row.new_lane_kind,
        row.new_attribution_jsonb,
    )?;
    Ok(RecoveryReceiptRecord {
        receipt_id: row.receipt_id,
        checkpoint_id,
        prior_session_id: row.prior_session_id,
        new_session_id: row.new_session_id,
        new_lane,
        resume_pointer: serde_json::from_value(row.resume_pointer_jsonb)?,
        event_ledger_event_id,
        recovered_at_utc: row.recovered_at_utc,
    })
}

fn quiet_background_work_from_row(
    row: QuietWorkRow,
) -> StateRecoveryResult<QuietBackgroundWorkRecord> {
    let event_ledger_event_id = record_key_string(&row.event_ledger_event_id)?;
    let lane = lane_from_parts(
        row.lane_id,
        row.actor_id,
        row.lane_kind,
        row.attribution_jsonb,
    )?;
    let policy: QuietBackgroundPolicy = serde_json::from_value(row.quiet_policy_jsonb)?;
    Ok(QuietBackgroundWorkRecord {
        receipt_id: row.receipt_id,
        workspace_id: row.workspace_id,
        wp_id: row.wp_id,
        mt_id: row.mt_id,
        work_kind: QuietBackgroundWorkKind::parse(&row.work_kind)?,
        subject_id: row.subject_id,
        lane,
        session_id: row.session_id,
        policy,
        evidence_ref: row.evidence_ref,
        event_ledger_event_id,
        created_at_utc: row.created_at_utc,
    })
}

fn index_lease_from_row(row: LeaseRow) -> StateRecoveryResult<IndexingLeaseRecord> {
    let event_ledger_event_id = row
        .event_ledger_event_id
        .as_ref()
        .ok_or_else(|| {
            StateRecoveryError::InvalidInput(format!(
                "indexing lease {} is missing its EventLedger receipt",
                row.lease_id
            ))
        })
        .and_then(record_key_string)?;
    let priority = i32::try_from(row.priority).map_err(|_| {
        StateRecoveryError::InvalidInput(format!(
            "indexing lease {} has out-of-range priority {}",
            row.lease_id, row.priority
        ))
    })?;
    let scope = scope_from_parts(row.scope_kind, row.scope_id)?;
    let lane = lane_from_parts(
        row.lane_id,
        row.actor_id,
        row.lane_kind,
        row.attribution_jsonb,
    )?;
    Ok(IndexingLeaseRecord {
        lease_id: row.lease_id,
        workspace_id: row.workspace_id,
        wp_id: row.wp_id,
        mt_id: row.mt_id,
        scope,
        lane,
        session_id: row.session_id,
        index_run_id: row.index_run_id,
        priority,
        ttl_seconds: row.ttl_seconds,
        status: IndexLeaseStatus::parse(&row.status)?,
        blocked_by_lease_id: row.blocked_by_lease_id,
        quiet_policy: serde_json::from_value(row.quiet_policy_jsonb)?,
        event_ledger_event_id,
    })
}

fn lane_from_parts(
    lane_id: String,
    actor_id: String,
    lane_kind: String,
    attribution: Value,
) -> StateRecoveryResult<AgentLaneIdentity> {
    AgentLaneIdentity::new(
        lane_id,
        actor_id,
        AgentLaneKind::parse(&lane_kind)?,
        serde_json::from_value::<LocalCloudAttribution>(attribution)?.scrubbed_for_persistence(),
    )
}

fn scope_from_parts(kind: String, scope_id: String) -> StateRecoveryResult<ClaimScope> {
    match kind.as_str() {
        "worktree" => Ok(ClaimScope::Worktree {
            worktree_id: scope_id,
        }),
        "workspace" => Ok(ClaimScope::Workspace {
            workspace_id: scope_id,
        }),
        "rich_document" => {
            let (workspace_id, document_id) = split_scoped_claim_id("rich_document", &scope_id)?;
            Ok(ClaimScope::RichDocument {
                workspace_id,
                document_id,
            })
        }
        "graph_mutation" => {
            let (workspace_id, graph_id) = split_scoped_claim_id("graph_mutation", &scope_id)?;
            Ok(ClaimScope::GraphMutation {
                workspace_id,
                graph_id,
            })
        }
        "index_run" => {
            let (workspace_id, source_root_id) = scope_id.split_once('/').ok_or_else(|| {
                StateRecoveryError::InvalidInput("index_run scope missing slash".to_string())
            })?;
            Ok(ClaimScope::IndexRun {
                workspace_id: workspace_id.to_string(),
                source_root_id: source_root_id.to_string(),
            })
        }
        other => Err(StateRecoveryError::InvalidInput(format!(
            "unknown claim scope kind: {other}"
        ))),
    }
}

fn split_scoped_claim_id(kind: &str, scope_id: &str) -> StateRecoveryResult<(String, String)> {
    let (workspace_id, child_id) = scope_id
        .split_once('/')
        .ok_or_else(|| StateRecoveryError::InvalidInput(format!("{kind} scope missing slash")))?;
    if workspace_id.is_empty() || child_id.is_empty() {
        return Err(StateRecoveryError::InvalidInput(format!(
            "{kind} scope has empty segment"
        )));
    }
    Ok((workspace_id.to_string(), child_id.to_string()))
}

fn validate_claim_scope(request_workspace_id: &str, scope: &ClaimScope) -> StateRecoveryResult<()> {
    match scope {
        ClaimScope::RichDocument {
            workspace_id,
            document_id,
        } => {
            ensure_composite_scope_segment("rich_document.workspace_id", workspace_id)?;
            ensure_composite_scope_segment("rich_document.document_id", document_id)?;
            ensure_scope_workspace_matches("rich_document", request_workspace_id, workspace_id)
        }
        ClaimScope::GraphMutation {
            workspace_id,
            graph_id,
        } => {
            ensure_composite_scope_segment("graph_mutation.workspace_id", workspace_id)?;
            ensure_composite_scope_segment("graph_mutation.graph_id", graph_id)?;
            ensure_scope_workspace_matches("graph_mutation", request_workspace_id, workspace_id)
        }
        ClaimScope::Worktree { .. }
        | ClaimScope::Workspace { .. }
        | ClaimScope::IndexRun { .. } => Ok(()),
    }
}

fn ensure_composite_scope_segment(field: &str, value: &str) -> StateRecoveryResult<()> {
    if value.contains('/') {
        return Err(StateRecoveryError::InvalidInput(format!(
            "{field} must not contain '/'"
        )));
    }
    ensure_safe_token(field, value)
}

fn ensure_scope_workspace_matches(
    kind: &str,
    request_workspace_id: &str,
    scope_workspace_id: &str,
) -> StateRecoveryResult<()> {
    if request_workspace_id == scope_workspace_id {
        Ok(())
    } else {
        Err(StateRecoveryError::InvalidInput(format!(
            "{kind} scope workspace_id must match request workspace_id"
        )))
    }
}

fn validate_ttl(ttl_seconds: i64) -> StateRecoveryResult<()> {
    if ttl_seconds <= 0 {
        return Err(StateRecoveryError::InvalidInput(
            "ttl_seconds must be positive".to_string(),
        ));
    }
    Ok(())
}

fn bounded_inspection_limit(limit: i64) -> StateRecoveryResult<i64> {
    if !(1..=500).contains(&limit) {
        return Err(StateRecoveryError::InvalidInput(
            "inspection limit must be between 1 and 500".to_string(),
        ));
    }
    Ok(limit)
}

fn bounded_handoff_body_chars(max_chars: i64) -> StateRecoveryResult<i64> {
    if !(512..=20_000).contains(&max_chars) {
        return Err(StateRecoveryError::InvalidInput(
            "handoff compression max_chars must be between 512 and 20000".to_string(),
        ));
    }
    Ok(max_chars)
}

fn compressed_handoff_body(
    checkpoint: &RecoveryCheckpointRecord,
    max_chars: usize,
    warnings: &mut Vec<String>,
) -> StateRecoveryResult<String> {
    let redactor = Redactor::from_policy(&EnvRedactionV1::default());
    let resume_pointer = serde_json::to_string(&checkpoint.resume_pointer)?;
    let mut body = String::new();

    let mandatory_lines = [
        format!("schema_id={PARALLEL_SWARM_HANDOFF_COMPRESSION_SCHEMA_ID}"),
        format!("checkpoint_id={}", checkpoint.checkpoint_id),
        format!("workspace_id={}", checkpoint.workspace_id),
        format!("wp_id={}", checkpoint.wp_id),
        format!("mt_id={}", checkpoint.mt_id),
        format!("source_session_id={}", checkpoint.session_id),
        format!(
            "source_lane={} actor={} kind={}",
            checkpoint.lane.lane_id,
            checkpoint.lane.actor_id,
            checkpoint.lane.lane_kind.as_str()
        ),
        format!("resume_pointer={resume_pointer}"),
        format!("git_head={}", checkpoint.git_head),
        format!("payload_sha256={}", checkpoint.payload_sha256),
        format!("checkpoint_event_id={}", checkpoint.event_ledger_event_id),
        "authority=embedded SurrealDB checkpoint record plus EventLedger receipt; this compressed handoff is a projection only".to_string(),
        "resume_action=recover_from_checkpoint(checkpoint_id) and continue from resume_pointer".to_string(),
        format!(
            "omitted_inputs={}",
            handoff_omitted_inputs().join(",")
        ),
    ];

    for line in mandatory_lines {
        push_required_handoff_line(&mut body, &line, max_chars)?;
    }

    push_handoff_section(
        &mut body,
        "touched_files",
        &checkpoint.touched_files,
        max_chars,
        warnings,
        &redactor,
    );
    push_handoff_section(
        &mut body,
        "tests",
        &checkpoint.tests,
        max_chars,
        warnings,
        &redactor,
    );
    push_handoff_section(
        &mut body,
        "hbr_rows",
        &checkpoint.hbr_rows,
        max_chars,
        warnings,
        &redactor,
    );

    if contains_raw_handoff_input_marker(&checkpoint.next_step_context) {
        warnings.push("next_step_context_omitted_raw_input_marker".to_string());
    } else {
        let context = redactor.redact_text(&checkpoint.next_step_context);
        push_optional_handoff_line(
            &mut body,
            &format!("next_step_context={context}"),
            max_chars,
            warnings,
            "next_step_context_truncated",
        );
    }

    Ok(body)
}

fn ensure_handoff_checkpoint_metadata_safe(
    checkpoint: &RecoveryCheckpointRecord,
) -> StateRecoveryResult<()> {
    let redactor = Redactor::from_policy(&EnvRedactionV1::default());
    for (field, value) in [
        ("workspace_id", checkpoint.workspace_id.as_str()),
        ("wp_id", checkpoint.wp_id.as_str()),
        ("mt_id", checkpoint.mt_id.as_str()),
        ("source_session_id", checkpoint.session_id.as_str()),
        ("source_lane_id", checkpoint.lane.lane_id.as_str()),
        ("source_actor_id", checkpoint.lane.actor_id.as_str()),
        ("git_head", checkpoint.git_head.as_str()),
    ] {
        if redactor.redact_text(value) != value || contains_raw_handoff_input_marker(value) {
            return Err(StateRecoveryError::InvalidInput(format!(
                "mandatory checkpoint metadata {field} must not contain secret-looking values or raw handoff input markers"
            )));
        }
    }
    Ok(())
}

fn push_required_handoff_line(
    body: &mut String,
    line: &str,
    max_chars: usize,
) -> StateRecoveryResult<()> {
    let needed = line.len() + 1;
    if body.len() + needed > max_chars {
        return Err(StateRecoveryError::InvalidInput(
            "handoff compression max_chars is too small for mandatory restart anchors".to_string(),
        ));
    }
    body.push_str(line);
    body.push('\n');
    Ok(())
}

fn push_optional_handoff_line(
    body: &mut String,
    line: &str,
    max_chars: usize,
    warnings: &mut Vec<String>,
    warning: &str,
) {
    if body.len() + line.len() + 1 <= max_chars {
        body.push_str(line);
        body.push('\n');
    } else if !warnings.iter().any(|item| item == warning) {
        warnings.push(warning.to_string());
    }
}

fn push_handoff_section(
    body: &mut String,
    label: &str,
    values: &[String],
    max_chars: usize,
    warnings: &mut Vec<String>,
    redactor: &Redactor,
) {
    if values.is_empty() {
        push_optional_handoff_line(
            body,
            &format!("{label}=NONE"),
            max_chars,
            warnings,
            &format!("{label}_truncated"),
        );
        return;
    }
    for (index, value) in values.iter().enumerate() {
        let redacted_value = redactor.redact_text(value);
        push_optional_handoff_line(
            body,
            &format!("{label}[{index}]={redacted_value}"),
            max_chars,
            warnings,
            &format!("{label}_truncated"),
        );
    }
}

fn contains_raw_handoff_input_marker(value: &str) -> bool {
    let lowered = value.to_ascii_lowercase();
    [
        "raw_checkpoint_payload",
        "raw checkpoint payload",
        "provider_chat_transcript",
        "provider chat transcript",
        "provider chat",
        "full_conversation_history",
        "full conversation history",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn handoff_omitted_inputs() -> Vec<String> {
    vec![
        "raw_checkpoint_payload".to_string(),
        "provider_chat_transcript".to_string(),
        "full_conversation_history".to_string(),
    ]
}

fn handoff_source_refs(
    checkpoint: &RecoveryCheckpointRecord,
) -> Vec<HandoffCompressionSourceRefV1> {
    let mut refs = vec![HandoffCompressionSourceRefV1 {
        source_kind: "checkpoint".to_string(),
        source_id: checkpoint.checkpoint_id.clone(),
        event_ledger_event_id: Some(checkpoint.event_ledger_event_id.clone()),
    }];
    if let Some(claim_id) = checkpoint.claim_id.as_ref() {
        refs.push(HandoffCompressionSourceRefV1 {
            source_kind: "claim".to_string(),
            source_id: claim_id.clone(),
            event_ledger_event_id: None,
        });
    }
    if let Some(handoff_id) = checkpoint.mailbox_handoff_id.as_ref() {
        refs.push(HandoffCompressionSourceRefV1 {
            source_kind: "mailbox_handoff".to_string(),
            source_id: handoff_id.clone(),
            event_ledger_event_id: None,
        });
    }
    refs
}

fn validate_quiet_background_policy(
    expected_kind: QuietBackgroundWorkKind,
    policy: &QuietBackgroundPolicy,
) -> StateRecoveryResult<()> {
    if policy.work_kind != expected_kind {
        return Err(StateRecoveryError::InvalidInput(format!(
            "quiet policy work_kind must be {}",
            expected_kind.as_str()
        )));
    }
    if !policy.no_foreground_window {
        return Err(StateRecoveryError::InvalidInput(
            "quiet policy requires no_foreground_window".to_string(),
        ));
    }
    if !policy.no_focus_steal {
        return Err(StateRecoveryError::InvalidInput(
            "quiet policy requires no_focus_steal".to_string(),
        ));
    }
    if !policy.no_os_shell_window {
        return Err(StateRecoveryError::InvalidInput(
            "quiet policy requires no_os_shell_window".to_string(),
        ));
    }
    if !policy.bounded {
        return Err(StateRecoveryError::InvalidInput(
            "quiet policy requires bounded".to_string(),
        ));
    }
    if !policy.observable {
        return Err(StateRecoveryError::InvalidInput(
            "quiet policy requires observable".to_string(),
        ));
    }
    Ok(())
}

fn ensure_bounded_text(field: &str, value: &str, max_len: usize) -> StateRecoveryResult<()> {
    if value.trim().is_empty() || value.len() > max_len {
        return Err(StateRecoveryError::InvalidInput(format!(
            "{field} must be non-empty and at most {max_len} bytes"
        )));
    }
    Ok(())
}

fn require_capability(
    lane: &AgentLaneIdentity,
    capability: AgentCapability,
) -> StateRecoveryResult<()> {
    if lane.capabilities().contains(&capability) {
        Ok(())
    } else {
        Err(StateRecoveryError::InvalidInput(format!(
            "lane {} ({}) requires capability {:?}",
            lane.lane_id,
            lane.lane_kind.as_str(),
            capability
        )))
    }
}

fn ensure_cloud_assistance_lane(lane: &AgentLaneIdentity) -> StateRecoveryResult<()> {
    if lane.lane_kind != AgentLaneKind::Cloud || lane.attribution.mode != AttributionMode::Cloud {
        return Err(StateRecoveryError::InvalidInput(
            "cloud assistance requires a cloud lane with cloud attribution".to_string(),
        ));
    }
    if lane.attribution.provider == Some(ModelProviderKind::LocalRuntime) {
        return Err(StateRecoveryError::InvalidInput(
            "cloud assistance provider must not be local_runtime".to_string(),
        ));
    }
    if lane.attribution.model_label.trim().is_empty() {
        return Err(StateRecoveryError::InvalidInput(
            "cloud assistance requires a model label".to_string(),
        ));
    }
    Ok(())
}

fn required_claim_capability(scope: &ClaimScope) -> AgentCapability {
    match scope {
        ClaimScope::Worktree { .. } => AgentCapability::ClaimWorktree,
        ClaimScope::Workspace { .. } => AgentCapability::ClaimWorkspace,
        ClaimScope::RichDocument { .. } => AgentCapability::EditRichDocument,
        ClaimScope::GraphMutation { .. } => AgentCapability::MutateGraph,
        ClaimScope::IndexRun { .. } => AgentCapability::WriteLocalIndex,
    }
}

fn system_reclaimer_lane() -> StateRecoveryResult<AgentLaneIdentity> {
    AgentLaneIdentity::new(
        "lane-system-state-recovery",
        "system-state-recovery",
        AgentLaneKind::System,
        LocalCloudAttribution {
            mode: AttributionMode::System,
            provider: None,
            runtime: Some("parallel_swarm_state_recovery".to_string()),
            model_label: "system".to_string(),
            credential_ref: None,
            provider_metadata: json!({}),
        },
    )
}

fn ensure_safe_token(field: &str, value: &str) -> StateRecoveryResult<()> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.len() > 256
        || !trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | ':' | '.' | '/' | '#'))
    {
        return Err(StateRecoveryError::InvalidInput(format!(
            "{field} must be a bounded safe token"
        )));
    }
    Ok(())
}

fn ensure_event_id(field: &str, value: &str) -> StateRecoveryResult<()> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.len() > 160
        || !trimmed.starts_with("KE-")
        || !trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
    {
        return Err(StateRecoveryError::InvalidInput(format!(
            "{field} must be a safe EventLedger id"
        )));
    }
    Ok(())
}

fn is_lower_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, 'a'..='f'))
}

fn ensure_sha256(value: &str) -> StateRecoveryResult<()> {
    if is_lower_sha256(value) {
        Ok(())
    } else {
        Err(StateRecoveryError::InvalidInput(
            "body_sha256 must be lowercase sha256 hex".to_string(),
        ))
    }
}

fn model_provider_kind_as_str(provider: ModelProviderKind) -> &'static str {
    match provider {
        ModelProviderKind::OpenAi => "open_ai",
        ModelProviderKind::Anthropic => "anthropic",
        ModelProviderKind::LocalRuntime => "local_runtime",
        ModelProviderKind::OfficialCli => "official_cli",
        ModelProviderKind::Other => "other",
    }
}

fn scrub_secret_metadata(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (key, value) in map {
                let key_l = key.to_ascii_lowercase();
                if key_l.contains("secret")
                    || key_l.contains("token")
                    || key_l.contains("password")
                    || key_l.contains("api_key")
                    || key_l == "key"
                {
                    out.insert(key, Value::String("[REDACTED]".to_string()));
                } else {
                    out.insert(key, scrub_secret_metadata(value));
                }
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(items.into_iter().map(scrub_secret_metadata).collect()),
        other => other,
    }
}

fn contains_obvious_secret_token(value: &str) -> bool {
    value.lines().any(|line| {
        let dynamic_value = handoff_dynamic_line_value(line);
        if dynamic_value.is_some_and(contains_high_entropy_secret_token) {
            return true;
        }
        let lowered = dynamic_value.unwrap_or(line).to_ascii_lowercase();
        lowered.contains("sk-")
            || lowered.contains("-----begin ")
            || contains_aws_access_key(&lowered)
            || contains_url_credential(&lowered)
            || contains_unredacted_assignment(&lowered, "bearer ")
            || contains_unredacted_assignment(&lowered, "password=")
            || contains_unredacted_assignment(&lowered, "password:")
            || contains_unredacted_assignment(&lowered, "secret_token=")
            || contains_unredacted_assignment(&lowered, "api_key=")
            || contains_unredacted_assignment(&lowered, "api-key=")
    })
}

fn handoff_dynamic_line_value(line: &str) -> Option<&str> {
    if line.starts_with("touched_files[")
        || line.starts_with("tests[")
        || line.starts_with("hbr_rows[")
        || line.starts_with("next_step_context=")
    {
        line.split_once('=').map(|(_, value)| value)
    } else {
        None
    }
}

fn contains_unredacted_assignment(lowered: &str, needle: &str) -> bool {
    let mut offset = 0;
    while let Some(relative) = lowered[offset..].find(needle) {
        let value_start = offset + relative + needle.len();
        let value = lowered[value_start..].trim_start();
        if !value.starts_with("[redacted:") {
            return true;
        }
        offset = value_start;
    }
    false
}

fn contains_aws_access_key(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.windows(20).any(|window| {
        matches!(&window[..4], b"akia" | b"asia")
            && window[4..]
                .iter()
                .all(|byte| byte.is_ascii_digit() || byte.is_ascii_lowercase())
    })
}

fn contains_url_credential(value: &str) -> bool {
    value.split_whitespace().any(|part| {
        part.find("://")
            .and_then(|scheme| {
                let after_scheme = &part[scheme + 3..];
                after_scheme.find('@').map(|at| {
                    let credential = &after_scheme[..at];
                    !credential.contains("[redacted:url_cred]") && credential.contains(':')
                })
            })
            .unwrap_or(false)
    })
}

fn contains_high_entropy_secret_token(value: &str) -> bool {
    let mut buf = String::new();
    for c in value.chars() {
        if c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '+' | '/' | '=') {
            buf.push(c);
        } else if flush_high_entropy_probe(&mut buf) {
            return true;
        }
    }
    flush_high_entropy_probe(&mut buf)
}

fn flush_high_entropy_probe(buf: &mut String) -> bool {
    let found = buf.len() >= 32
        && buf.chars().any(|c| c.is_ascii_lowercase())
        && buf.chars().any(|c| c.is_ascii_uppercase())
        && buf.chars().any(|c| c.is_ascii_digit());
    buf.clear();
    found
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

fn canonical_json(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(v) => v.to_string(),
        Value::Number(v) => v.to_string(),
        Value::String(v) => serde_json::to_string(v).expect("string serializes"),
        Value::Array(items) => {
            let values: Vec<String> = items.iter().map(canonical_json).collect();
            format!("[{}]", values.join(","))
        }
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let fields: Vec<String> = keys
                .into_iter()
                .map(|key| {
                    format!(
                        "{}:{}",
                        serde_json::to_string(key).expect("key serializes"),
                        canonical_json(&map[key])
                    )
                })
                .collect();
            format!("{{{}}}", fields.join(","))
        }
    }
}
