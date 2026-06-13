//! WP-KERNEL-009 MT-209..216 ParallelSwarmStateRecovery backend foundations.
//!
//! This module is intentionally backend-only. It gives local/cloud model lanes
//! typed identity, claim leases over shared worktrees/workspaces, role-mailbox
//! handoff receipts, deterministic backend navigation commands, restartable
//! compaction checkpoints, recovery receipts, and a serial lease queue for
//! parallel index writers. PostgreSQL tables from migration 0311 are authority;
//! EventLedger rows provide the receipt trail.

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{postgres::PgRow, PgPool, Postgres, Row, Transaction};
use thiserror::Error;
use uuid::Uuid;

use crate::kernel::{KernelActor, KernelEvent, KernelEventType, NewKernelEvent};
use crate::storage::{Database, StorageError};

#[derive(Debug, Error)]
pub enum StateRecoveryError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("kernel event error: {0}")]
    Kernel(String),
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("sqlx error: {0}")]
    Sqlx(#[from] sqlx::Error),
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

#[derive(Clone)]
pub struct ParallelSwarmStateRecoveryStore {
    pool: PgPool,
}

impl ParallelSwarmStateRecoveryStore {
    pub fn new(pool: PgPool, _events: Arc<dyn Database>) -> Self {
        Self { pool }
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
        let lane_json = serde_json::to_value(&persistent_lane.attribution)?;
        let mut tx = self.pool.begin().await?;
        let inserted = sqlx::query(
            r#"
            INSERT INTO knowledge_agent_worktree_claims (
                claim_id, workspace_id, wp_id, mt_id, scope_kind, scope_id,
                lane_id, actor_id, lane_kind, attribution_jsonb, session_id,
                status, reason, expires_at_utc
            )
            VALUES (
                $1, $2, $3, $4, $5, $6,
                $7, $8, $9, $10, $11,
                'active', $12, NOW() + ($13::BIGINT * INTERVAL '1 second')
            )
            "#,
        )
        .bind(&claim_id)
        .bind(&request.workspace_id)
        .bind(&request.wp_id)
        .bind(&request.mt_id)
        .bind(request.scope.kind_str())
        .bind(request.scope.scope_id())
        .bind(&request.lane.lane_id)
        .bind(&request.lane.actor_id)
        .bind(request.lane.lane_kind.as_str())
        .bind(lane_json)
        .bind(&request.session_id)
        .bind(&request.reason)
        .bind(request.ttl_seconds)
        .execute(&mut *tx)
        .await;

        match inserted {
            Ok(_) => {
                let event_id = self
                    .append_event_tx(
                        &mut tx,
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
                    )
                    .await?;
                sqlx::query(
                    r#"
                    UPDATE knowledge_agent_worktree_claims
                       SET event_ledger_event_id = $2
                     WHERE claim_id = $1
                    "#,
                )
                .bind(&claim_id)
                .bind(&event_id)
                .execute(&mut *tx)
                .await?;
                tx.commit().await?;
                Ok(WorkClaimOutcome {
                    status: ClaimStatus::Active,
                    claim_id,
                    active_holder: None,
                    event_ledger_event_id: Some(event_id),
                })
            }
            Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
                let _ = tx.rollback().await;
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
            Err(err) => Err(err.into()),
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
        let rows = sqlx::query(
            r#"
            SELECT * FROM knowledge_agent_worktree_claims
            WHERE workspace_id = $1
              AND status = 'active'
              AND released_at_utc IS NULL
              AND expires_at_utc > NOW()
            ORDER BY claimed_at_utc ASC, claim_id ASC
            "#,
        )
        .bind(workspace_id)
        .fetch_all(&self.pool)
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

        let claim_rows = sqlx::query(
            r#"
            SELECT * FROM knowledge_agent_worktree_claims
            WHERE workspace_id = $1
            ORDER BY claimed_at_utc DESC, claim_id DESC
            LIMIT $2
            "#,
        )
        .bind(&request.workspace_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        let handoff_rows = sqlx::query(
            r#"
            SELECT h.*
            FROM knowledge_agent_role_mailbox_handoffs h
            INNER JOIN knowledge_agent_worktree_claims c
                    ON c.claim_id = h.claim_id
            WHERE c.workspace_id = $1
            ORDER BY h.created_at_utc DESC, h.handoff_id DESC
            LIMIT $2
            "#,
        )
        .bind(&request.workspace_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        let checkpoint_rows = sqlx::query(
            r#"
            SELECT * FROM knowledge_agent_state_recovery_checkpoints
            WHERE workspace_id = $1
            ORDER BY created_at_utc DESC, checkpoint_id DESC
            LIMIT $2
            "#,
        )
        .bind(&request.workspace_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        let recovery_rows = sqlx::query(
            r#"
            SELECT r.*
            FROM knowledge_agent_recovery_receipts r
            INNER JOIN knowledge_agent_state_recovery_checkpoints c
                    ON c.checkpoint_id = r.checkpoint_id
            WHERE c.workspace_id = $1
            ORDER BY r.recovered_at_utc DESC, r.receipt_id DESC
            LIMIT $2
            "#,
        )
        .bind(&request.workspace_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        let lease_rows = sqlx::query(
            r#"
            SELECT * FROM knowledge_parallel_indexing_lease_queue
            WHERE workspace_id = $1
            ORDER BY enqueued_at_utc DESC, lease_id DESC
            LIMIT $2
            "#,
        )
        .bind(&request.workspace_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        let quiet_rows = sqlx::query(
            r#"
            SELECT * FROM knowledge_agent_quiet_background_work
            WHERE workspace_id = $1
            ORDER BY created_at_utc DESC, receipt_id DESC
            LIMIT $2
            "#,
        )
        .bind(&request.workspace_id)
        .bind(limit)
        .fetch_all(&self.pool)
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

        let claim_rows = sqlx::query(
            r#"
            SELECT * FROM knowledge_agent_worktree_claims
            WHERE workspace_id = $1
              AND ($2::TEXT IS NULL OR wp_id = $2)
              AND ($3::TEXT IS NULL OR mt_id = $3)
            ORDER BY claimed_at_utc DESC, claim_id DESC
            LIMIT $4
            "#,
        )
        .bind(&request.workspace_id)
        .bind(request.wp_id.as_deref())
        .bind(request.mt_id.as_deref())
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        let claims = claim_rows
            .into_iter()
            .map(work_claim_from_row)
            .collect::<StateRecoveryResult<Vec<_>>>()?;

        let handoff_rows = sqlx::query(
            r#"
            SELECT h.*
            FROM knowledge_agent_role_mailbox_handoffs h
            INNER JOIN knowledge_agent_worktree_claims c
                    ON c.claim_id = h.claim_id
            WHERE c.workspace_id = $1
              AND ($2::TEXT IS NULL OR h.wp_id = $2)
              AND ($3::TEXT IS NULL OR h.mt_id = $3)
            ORDER BY h.created_at_utc DESC, h.handoff_id DESC
            LIMIT $4
            "#,
        )
        .bind(&request.workspace_id)
        .bind(request.wp_id.as_deref())
        .bind(request.mt_id.as_deref())
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        let mailbox_handoffs = handoff_rows
            .into_iter()
            .map(mailbox_handoff_from_row)
            .collect::<StateRecoveryResult<Vec<_>>>()?;

        let checkpoint_rows = sqlx::query(
            r#"
            SELECT * FROM knowledge_agent_state_recovery_checkpoints
            WHERE workspace_id = $1
              AND ($2::TEXT IS NULL OR wp_id = $2)
              AND ($3::TEXT IS NULL OR mt_id = $3)
            ORDER BY created_at_utc DESC, checkpoint_id DESC
            LIMIT $4
            "#,
        )
        .bind(&request.workspace_id)
        .bind(request.wp_id.as_deref())
        .bind(request.mt_id.as_deref())
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        let checkpoints = checkpoint_rows
            .into_iter()
            .map(checkpoint_from_row)
            .collect::<StateRecoveryResult<Vec<_>>>()?;

        let recovery_rows = sqlx::query(
            r#"
            SELECT r.*
            FROM knowledge_agent_recovery_receipts r
            INNER JOIN knowledge_agent_state_recovery_checkpoints c
                    ON c.checkpoint_id = r.checkpoint_id
            WHERE c.workspace_id = $1
              AND ($2::TEXT IS NULL OR c.wp_id = $2)
              AND ($3::TEXT IS NULL OR c.mt_id = $3)
            ORDER BY r.recovered_at_utc DESC, r.receipt_id DESC
            LIMIT $4
            "#,
        )
        .bind(&request.workspace_id)
        .bind(request.wp_id.as_deref())
        .bind(request.mt_id.as_deref())
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        let recovery_receipts = recovery_rows
            .into_iter()
            .map(recovery_receipt_from_row)
            .collect::<StateRecoveryResult<Vec<_>>>()?;

        let lease_rows = sqlx::query(
            r#"
            SELECT * FROM knowledge_parallel_indexing_lease_queue
            WHERE workspace_id = $1
              AND ($2::TEXT IS NULL OR wp_id = $2)
              AND ($3::TEXT IS NULL OR mt_id = $3)
            ORDER BY enqueued_at_utc DESC, lease_id DESC
            LIMIT $4
            "#,
        )
        .bind(&request.workspace_id)
        .bind(request.wp_id.as_deref())
        .bind(request.mt_id.as_deref())
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        let indexing_leases = lease_rows
            .into_iter()
            .map(index_lease_from_row)
            .collect::<StateRecoveryResult<Vec<_>>>()?;

        let quiet_rows = sqlx::query(
            r#"
            SELECT * FROM knowledge_agent_quiet_background_work
            WHERE workspace_id = $1
              AND ($2::TEXT IS NULL OR wp_id = $2)
              AND ($3::TEXT IS NULL OR mt_id = $3)
            ORDER BY created_at_utc DESC, receipt_id DESC
            LIMIT $4
            "#,
        )
        .bind(&request.workspace_id)
        .bind(request.wp_id.as_deref())
        .bind(request.mt_id.as_deref())
        .bind(limit)
        .fetch_all(&self.pool)
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
        let claim_summary = sqlx::query(
            r#"
            SELECT COUNT(*) AS claims,
                   COUNT(*) FILTER (WHERE status = 'active') AS active_claims,
                   COUNT(*) FILTER (
                       WHERE status = 'active' AND expires_at_utc <= NOW()
                   ) AS stale_active_claims
            FROM knowledge_agent_worktree_claims
            WHERE workspace_id = $1
              AND ($2::TEXT IS NULL OR wp_id = $2)
              AND ($3::TEXT IS NULL OR mt_id = $3)
            "#,
        )
        .bind(workspace_id)
        .bind(wp_id)
        .bind(mt_id)
        .fetch_one(&self.pool)
        .await?;
        let handoff_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM knowledge_agent_role_mailbox_handoffs h
            INNER JOIN knowledge_agent_worktree_claims c
                    ON c.claim_id = h.claim_id
            WHERE c.workspace_id = $1
              AND ($2::TEXT IS NULL OR h.wp_id = $2)
              AND ($3::TEXT IS NULL OR h.mt_id = $3)
            "#,
        )
        .bind(workspace_id)
        .bind(wp_id)
        .bind(mt_id)
        .fetch_one(&self.pool)
        .await?;
        let checkpoint_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM knowledge_agent_state_recovery_checkpoints
            WHERE workspace_id = $1
              AND ($2::TEXT IS NULL OR wp_id = $2)
              AND ($3::TEXT IS NULL OR mt_id = $3)
            "#,
        )
        .bind(workspace_id)
        .bind(wp_id)
        .bind(mt_id)
        .fetch_one(&self.pool)
        .await?;
        let recovery_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM knowledge_agent_recovery_receipts r
            INNER JOIN knowledge_agent_state_recovery_checkpoints c
                    ON c.checkpoint_id = r.checkpoint_id
            WHERE c.workspace_id = $1
              AND ($2::TEXT IS NULL OR c.wp_id = $2)
              AND ($3::TEXT IS NULL OR c.mt_id = $3)
            "#,
        )
        .bind(workspace_id)
        .bind(wp_id)
        .bind(mt_id)
        .fetch_one(&self.pool)
        .await?;
        let lease_summary = sqlx::query(
            r#"
            SELECT COUNT(*) AS indexing_leases,
                   COUNT(*) FILTER (WHERE status = 'acquired') AS acquired_indexing_leases
            FROM knowledge_parallel_indexing_lease_queue
            WHERE workspace_id = $1
              AND ($2::TEXT IS NULL OR wp_id = $2)
              AND ($3::TEXT IS NULL OR mt_id = $3)
            "#,
        )
        .bind(workspace_id)
        .bind(wp_id)
        .bind(mt_id)
        .fetch_one(&self.pool)
        .await?;
        let quiet_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM knowledge_agent_quiet_background_work
            WHERE workspace_id = $1
              AND ($2::TEXT IS NULL OR wp_id = $2)
              AND ($3::TEXT IS NULL OR mt_id = $3)
            "#,
        )
        .bind(workspace_id)
        .bind(wp_id)
        .bind(mt_id)
        .fetch_one(&self.pool)
        .await?;
        let event_count: i64 = sqlx::query_scalar(
            r#"
            WITH source_events(event_id) AS (
                SELECT event_ledger_event_id
                FROM knowledge_agent_worktree_claims
                WHERE workspace_id = $1
                  AND ($2::TEXT IS NULL OR wp_id = $2)
                  AND ($3::TEXT IS NULL OR mt_id = $3)
                  AND event_ledger_event_id IS NOT NULL
                UNION
                SELECT release_event_ledger_event_id
                FROM knowledge_agent_worktree_claims
                WHERE workspace_id = $1
                  AND ($2::TEXT IS NULL OR wp_id = $2)
                  AND ($3::TEXT IS NULL OR mt_id = $3)
                  AND release_event_ledger_event_id IS NOT NULL
                UNION
                SELECT reclaim_event_ledger_event_id
                FROM knowledge_agent_worktree_claims
                WHERE workspace_id = $1
                  AND ($2::TEXT IS NULL OR wp_id = $2)
                  AND ($3::TEXT IS NULL OR mt_id = $3)
                  AND reclaim_event_ledger_event_id IS NOT NULL
                UNION
                SELECT h.event_ledger_event_id
                FROM knowledge_agent_role_mailbox_handoffs h
                INNER JOIN knowledge_agent_worktree_claims c
                        ON c.claim_id = h.claim_id
                WHERE c.workspace_id = $1
                  AND ($2::TEXT IS NULL OR h.wp_id = $2)
                  AND ($3::TEXT IS NULL OR h.mt_id = $3)
                UNION
                SELECT event_ledger_event_id
                FROM knowledge_agent_state_recovery_checkpoints
                WHERE workspace_id = $1
                  AND ($2::TEXT IS NULL OR wp_id = $2)
                  AND ($3::TEXT IS NULL OR mt_id = $3)
                UNION
                SELECT r.event_ledger_event_id
                FROM knowledge_agent_recovery_receipts r
                INNER JOIN knowledge_agent_state_recovery_checkpoints c
                        ON c.checkpoint_id = r.checkpoint_id
                WHERE c.workspace_id = $1
                  AND ($2::TEXT IS NULL OR c.wp_id = $2)
                  AND ($3::TEXT IS NULL OR c.mt_id = $3)
                UNION
                SELECT event_ledger_event_id
                FROM knowledge_parallel_indexing_lease_queue
                WHERE workspace_id = $1
                  AND ($2::TEXT IS NULL OR wp_id = $2)
                  AND ($3::TEXT IS NULL OR mt_id = $3)
                  AND event_ledger_event_id IS NOT NULL
                UNION
                SELECT event_ledger_event_id
                FROM knowledge_agent_quiet_background_work
                WHERE workspace_id = $1
                  AND ($2::TEXT IS NULL OR wp_id = $2)
                  AND ($3::TEXT IS NULL OR mt_id = $3)
            )
            SELECT COUNT(DISTINCT e.event_id)
            FROM source_events s
            INNER JOIN kernel_event_ledger e
                    ON e.event_id = s.event_id
                   AND e.source_component = $4
            "#,
        )
        .bind(workspace_id)
        .bind(wp_id)
        .bind(mt_id)
        .bind(PARALLEL_SWARM_SOURCE_COMPONENT)
        .fetch_one(&self.pool)
        .await?;
        let claim_status_rows = sqlx::query(
            r#"
            SELECT status, COUNT(*) AS row_count
            FROM knowledge_agent_worktree_claims
            WHERE workspace_id = $1
              AND ($2::TEXT IS NULL OR wp_id = $2)
              AND ($3::TEXT IS NULL OR mt_id = $3)
            GROUP BY status
            "#,
        )
        .bind(workspace_id)
        .bind(wp_id)
        .bind(mt_id)
        .fetch_all(&self.pool)
        .await?;
        let handoff_status_rows = sqlx::query(
            r#"
            SELECT h.status, COUNT(*) AS row_count
            FROM knowledge_agent_role_mailbox_handoffs h
            INNER JOIN knowledge_agent_worktree_claims c
                    ON c.claim_id = h.claim_id
            WHERE c.workspace_id = $1
              AND ($2::TEXT IS NULL OR h.wp_id = $2)
              AND ($3::TEXT IS NULL OR h.mt_id = $3)
            GROUP BY h.status
            "#,
        )
        .bind(workspace_id)
        .bind(wp_id)
        .bind(mt_id)
        .fetch_all(&self.pool)
        .await?;
        let lease_status_rows = sqlx::query(
            r#"
            SELECT status, COUNT(*) AS row_count
            FROM knowledge_parallel_indexing_lease_queue
            WHERE workspace_id = $1
              AND ($2::TEXT IS NULL OR wp_id = $2)
              AND ($3::TEXT IS NULL OR mt_id = $3)
            GROUP BY status
            "#,
        )
        .bind(workspace_id)
        .bind(wp_id)
        .bind(mt_id)
        .fetch_all(&self.pool)
        .await?;
        let quiet_kind_rows = sqlx::query(
            r#"
            SELECT work_kind, COUNT(*) AS row_count
            FROM knowledge_agent_quiet_background_work
            WHERE workspace_id = $1
              AND ($2::TEXT IS NULL OR wp_id = $2)
              AND ($3::TEXT IS NULL OR mt_id = $3)
            GROUP BY work_kind
            "#,
        )
        .bind(workspace_id)
        .bind(wp_id)
        .bind(mt_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(SwarmDashboardAuthorityTotals {
            claims: claim_summary.try_get("claims")?,
            active_claims: claim_summary.try_get("active_claims")?,
            stale_active_claims: claim_summary.try_get("stale_active_claims")?,
            mailbox_handoffs: handoff_count,
            recovery_checkpoints: checkpoint_count,
            recovery_receipts: recovery_count,
            indexing_leases: lease_summary.try_get("indexing_leases")?,
            acquired_indexing_leases: lease_summary.try_get("acquired_indexing_leases")?,
            quiet_background_work: quiet_count,
            events: event_count,
            claims_by_status: dashboard_group_count_map(claim_status_rows, "status")?,
            handoffs_by_status: dashboard_group_count_map(handoff_status_rows, "status")?,
            leases_by_status: dashboard_group_count_map(lease_status_rows, "status")?,
            quiet_work_by_kind: dashboard_group_count_map(quiet_kind_rows, "work_kind")?,
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
        let rows = sqlx::query(
            r#"
            SELECT event_id,
                   source_component,
                   aggregate_type,
                   aggregate_id,
                   created_at AT TIME ZONE 'UTC' AS created_at_utc
            FROM kernel_event_ledger
            WHERE source_component = $1
              AND event_id = ANY($2)
            ORDER BY created_at DESC, event_id DESC
            "#,
        )
        .bind(PARALLEL_SWARM_SOURCE_COMPONENT)
        .bind(&event_ids)
        .fetch_all(&self.pool)
        .await?;
        let mut found = BTreeSet::new();
        let mut counts = BTreeMap::<String, i64>::new();
        let mut max_created = None;
        let mut events = Vec::new();
        for row in rows {
            let event_id: String = row.try_get("event_id")?;
            let source_component: String = row.try_get("source_component")?;
            let aggregate_type: String = row.try_get("aggregate_type")?;
            let aggregate_id: String = row.try_get("aggregate_id")?;
            let created_at: DateTime<Utc> = row.try_get("created_at_utc")?;
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
        let lane_json = serde_json::to_value(&persistent_lane.attribution)?;
        let policy_json = serde_json::to_value(&request.policy)?;
        let mut tx = self.pool.begin().await?;
        let event_id = self
            .append_event_tx(
                &mut tx,
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
            )
            .await?;
        let row = sqlx::query(
            r#"
            INSERT INTO knowledge_agent_quiet_background_work (
                receipt_id, workspace_id, wp_id, mt_id, work_kind, subject_id,
                lane_id, actor_id, lane_kind, attribution_jsonb, session_id,
                quiet_policy_jsonb, evidence_ref, event_ledger_event_id
            )
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)
            RETURNING *
            "#,
        )
        .bind(&receipt_id)
        .bind(&request.workspace_id)
        .bind(&request.wp_id)
        .bind(&request.mt_id)
        .bind(request.work_kind.as_str())
        .bind(&request.subject_id)
        .bind(&persistent_lane.lane_id)
        .bind(&persistent_lane.actor_id)
        .bind(persistent_lane.lane_kind.as_str())
        .bind(lane_json)
        .bind(&request.session_id)
        .bind(policy_json)
        .bind(&request.evidence_ref)
        .bind(&event_id)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        quiet_background_work_from_row(row)
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
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query(
            r#"
            SELECT * FROM knowledge_agent_worktree_claims
             WHERE claim_id = $1
               AND actor_id = $2
               AND status = 'active'
               AND released_at_utc IS NULL
             FOR UPDATE
            "#,
        )
        .bind(claim_id)
        .bind(&lane.actor_id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(row) = row else {
            tx.rollback().await?;
            return Ok(false);
        };
        let claim = work_claim_from_row(row)?;
        let persistent_lane = lane.scrubbed_for_persistence();
        let event_id = self
            .append_event_tx(
                &mut tx,
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
            )
            .await?;
        sqlx::query(
            r#"
            UPDATE knowledge_agent_worktree_claims
               SET status = 'released',
                   released_at_utc = NOW(),
                   reason = $3,
                   release_event_ledger_event_id = $4
             WHERE claim_id = $1
               AND actor_id = $2
               AND status = 'active'
               AND released_at_utc IS NULL
            "#,
        )
        .bind(claim_id)
        .bind(&lane.actor_id)
        .bind(reason)
        .bind(&event_id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(true)
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
        let mut tx = self.pool.begin().await?;
        let event_id = self
            .append_event_tx(
                &mut tx,
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
            )
            .await?;
        let lane_json = serde_json::to_value(&from_lane.attribution)?;
        let row = sqlx::query(
            r#"
            INSERT INTO knowledge_agent_role_mailbox_handoffs (
                handoff_id, wp_id, mt_id, claim_id, from_lane_id, from_actor_id,
                from_lane_kind, from_attribution_jsonb, to_role,
                mailbox_thread_id, mailbox_message_id, status, summary,
                body_sha256, event_ledger_event_id
            )
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)
            RETURNING *
            "#,
        )
        .bind(&handoff_id)
        .bind(&request.wp_id)
        .bind(&request.mt_id)
        .bind(&request.claim_id)
        .bind(&from_lane.lane_id)
        .bind(&from_lane.actor_id)
        .bind(from_lane.lane_kind.as_str())
        .bind(lane_json)
        .bind(&request.to_role)
        .bind(&request.mailbox_thread_id)
        .bind(&request.mailbox_message_id)
        .bind(request.status.as_str())
        .bind(&request.summary)
        .bind(&request.body_sha256)
        .bind(&event_id)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        mailbox_handoff_from_row(row)
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
        let mut tx = self.pool.begin().await?;
        let fallback_basis_event_id = self
            .append_event_tx(
                &mut tx,
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
            )
            .await?;
        tx.commit().await?;

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
        let lane_json = serde_json::to_value(&from_lane.attribution)?;
        let mut tx = self.pool.begin().await?;
        let claim = self
            .active_cloud_assistance_claim_tx(&mut tx, &request)
            .await?
            .ok_or_else(|| {
                StateRecoveryError::InvalidInput(
                    "cloud assistance requires an active cloud-owned workspace claim".to_string(),
                )
            })?;
        self.ensure_cloud_fallback_basis_event_tx(&mut tx, &request)
            .await?;

        let handoff_event_id = self
            .append_event_tx(
                &mut tx,
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
            )
            .await?;
        let handoff_row = sqlx::query(
            r#"
            INSERT INTO knowledge_agent_role_mailbox_handoffs (
                handoff_id, wp_id, mt_id, claim_id, from_lane_id, from_actor_id,
                from_lane_kind, from_attribution_jsonb, to_role,
                mailbox_thread_id, mailbox_message_id, status, summary,
                body_sha256, event_ledger_event_id
            )
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)
            RETURNING *
            "#,
        )
        .bind(&handoff_id)
        .bind(&request.wp_id)
        .bind(&request.mt_id)
        .bind(&request.claim_id)
        .bind(&from_lane.lane_id)
        .bind(&from_lane.actor_id)
        .bind(from_lane.lane_kind.as_str())
        .bind(lane_json)
        .bind(&request.to_role)
        .bind(&request.mailbox_thread_id)
        .bind(&request.mailbox_message_id)
        .bind(SwarmReceiptStatus::Progress.as_str())
        .bind(&request.summary)
        .bind(&request.body_sha256)
        .bind(&handoff_event_id)
        .fetch_one(&mut *tx)
        .await?;

        let cloud_assistance_event_id = self
            .append_event_tx(
                &mut tx,
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
            )
            .await?;
        sqlx::query(
            r#"
            INSERT INTO knowledge_agent_cloud_assistance_receipts (
                receipt_id, workspace_id, wp_id, mt_id, claim_id,
                handoff_id, handoff_event_ledger_event_id, cloud_assistance_event_id,
                fallback_basis_event_id, parent_session_id, prompt_sha256,
                lane_id, actor_id, lane_kind,
                provider, model_label, attribution_jsonb, session_id,
                fallback_reason, output_kind, output_sha256, body_sha256,
                output_text, output_body_jsonb, target_ref,
                review_state, non_authoritative, requires_promotion,
                authority_mutation_allowed, promotion_event_id
            )
            VALUES (
                $1,$2,$3,$4,$5,
                $6,$7,$8,
                $9,$10,$11,
                $12,$13,$14,
                $15,$16,$17,$18,
                $19,$20,$21,$22,
                $23,$24,$25,
                'pending_review', TRUE, TRUE, FALSE, NULL
            )
            "#,
        )
        .bind(&receipt_id)
        .bind(&request.workspace_id)
        .bind(&request.wp_id)
        .bind(&request.mt_id)
        .bind(&request.claim_id)
        .bind(&handoff_id)
        .bind(&handoff_event_id)
        .bind(&cloud_assistance_event_id)
        .bind(&request.fallback_basis_event_id)
        .bind(&request.parent_session_id)
        .bind(&request.prompt_sha256)
        .bind(&from_lane.lane_id)
        .bind(&from_lane.actor_id)
        .bind(from_lane.lane_kind.as_str())
        .bind(model_provider_kind_as_str(
            from_lane.attribution.provider.ok_or_else(|| {
                StateRecoveryError::InvalidInput(
                    "cloud assistance provider must be present".to_string(),
                )
            })?,
        ))
        .bind(&from_lane.attribution.model_label)
        .bind(serde_json::to_value(&from_lane.attribution)?)
        .bind(&request.session_id)
        .bind(request.fallback_reason.as_str())
        .bind(request.output_kind.as_str())
        .bind(&request.output_sha256)
        .bind(&request.body_sha256)
        .bind(&request.output_text)
        .bind(&request.output_body_jsonb)
        .bind(&request.target_ref)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        let handoff = mailbox_handoff_from_row(handoff_row)?;
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

    async fn active_cloud_assistance_claim_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        request: &CloudAssistanceRequest,
    ) -> StateRecoveryResult<Option<WorkClaimRecord>> {
        let row = sqlx::query(
            r#"
            SELECT *
            FROM knowledge_agent_worktree_claims
            WHERE claim_id = $1
              AND workspace_id = $2
              AND wp_id = $3
              AND mt_id = $4
              AND scope_kind = 'workspace'
              AND scope_id = $2
              AND lane_id = $5
              AND actor_id = $6
              AND lane_kind = 'cloud'
              AND status = 'active'
              AND released_at_utc IS NULL
              AND expires_at_utc > NOW()
            FOR UPDATE
            "#,
        )
        .bind(&request.claim_id)
        .bind(&request.workspace_id)
        .bind(&request.wp_id)
        .bind(&request.mt_id)
        .bind(&request.from_lane.lane_id)
        .bind(&request.from_lane.actor_id)
        .fetch_optional(&mut **tx)
        .await?;
        row.map(work_claim_from_row).transpose()
    }

    async fn ensure_cloud_fallback_basis_event_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        request: &CloudAssistanceRequest,
    ) -> StateRecoveryResult<()> {
        let found: Option<i64> = sqlx::query_scalar(
            r#"
            SELECT 1::BIGINT
            FROM kernel_event_ledger
            WHERE event_id = $1
              AND aggregate_type = 'parallel_swarm_cloud_fallback_basis'
              AND source_component = $2
              AND payload ->> 'schema_id' = $3
              AND payload ->> 'workspace_id' = $4
              AND payload ->> 'wp_id' = $5
              AND payload ->> 'mt_id' = $6
              AND payload ->> 'claim_id' = $7
              AND payload ->> 'parent_session_id' = $8
              AND payload ->> 'prompt_sha256' = $9
              AND payload ->> 'fallback_reason' = $10
              AND COALESCE(payload -> 'lane' ->> 'lane_kind', '') <> 'cloud'
              AND COALESCE(payload -> 'lane' ->> 'lane_kind', '') IN ('local', 'system')
            "#,
        )
        .bind(&request.fallback_basis_event_id)
        .bind(PARALLEL_SWARM_SOURCE_COMPONENT)
        .bind(PARALLEL_SWARM_CLOUD_FALLBACK_BASIS_SCHEMA_ID)
        .bind(&request.workspace_id)
        .bind(&request.wp_id)
        .bind(&request.mt_id)
        .bind(&request.claim_id)
        .bind(&request.parent_session_id)
        .bind(&request.prompt_sha256)
        .bind(request.fallback_reason.as_str())
        .fetch_optional(&mut **tx)
        .await?;
        if found.is_some() {
            Ok(())
        } else {
            Err(StateRecoveryError::InvalidInput(
                "cloud assistance requires a matching fallback-basis EventLedger proof".to_string(),
            ))
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
        let mut tx = self.pool.begin().await?;
        let event_id = self
            .append_event_tx(
                &mut tx,
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
            )
            .await?;
        let lane_json = serde_json::to_value(&lane.attribution)?;
        let row = sqlx::query(
            r#"
            INSERT INTO knowledge_agent_state_recovery_checkpoints (
                checkpoint_id, lane_id, actor_id, lane_kind, attribution_jsonb,
                session_id, workspace_id, wp_id, mt_id, claim_id,
                mailbox_handoff_id, navigation_command_id, resume_pointer_jsonb,
                touched_files_jsonb, tests_jsonb, hbr_rows_jsonb,
                next_step_context, payload_jsonb, payload_sha256,
                compaction_reason, git_head, event_ledger_event_id
            )
            VALUES (
                $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,
                $14,$15,$16,$17,$18,$19,$20,$21,$22
            )
            RETURNING *
            "#,
        )
        .bind(&checkpoint_id)
        .bind(&lane.lane_id)
        .bind(&lane.actor_id)
        .bind(lane.lane_kind.as_str())
        .bind(lane_json)
        .bind(&request.session_id)
        .bind(&request.workspace_id)
        .bind(&request.wp_id)
        .bind(&request.mt_id)
        .bind(&request.claim_id)
        .bind(&request.mailbox_handoff_id)
        .bind(&request.navigation_command_id)
        .bind(serde_json::to_value(&request.resume_pointer)?)
        .bind(json!(request.touched_files))
        .bind(json!(request.tests))
        .bind(json!(request.hbr_rows))
        .bind(&request.next_step_context)
        .bind(&request.payload)
        .bind(&payload_sha256)
        .bind(&request.compaction_reason)
        .bind(&request.git_head)
        .bind(&event_id)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        checkpoint_from_row(row)
    }

    pub async fn recover_from_checkpoint(
        &self,
        checkpoint_id: &str,
        new_lane: AgentLaneIdentity,
        new_session_id: &str,
    ) -> StateRecoveryResult<RecoveredCheckpoint> {
        require_capability(&new_lane, AgentCapability::RecordCheckpoint)?;
        let row = sqlx::query(
            "SELECT * FROM knowledge_agent_state_recovery_checkpoints WHERE checkpoint_id = $1",
        )
        .bind(checkpoint_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| StateRecoveryError::CheckpointNotFound(checkpoint_id.to_string()))?;
        let checkpoint = checkpoint_from_row(row)?;
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
        let mut tx = self.pool.begin().await?;
        let event_id = self
            .append_event_tx(
                &mut tx,
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
            )
            .await?;
        let lane_json = serde_json::to_value(&new_lane.attribution)?;
        let row = sqlx::query(
            r#"
            INSERT INTO knowledge_agent_recovery_receipts (
                receipt_id, checkpoint_id, prior_session_id, new_session_id,
                new_lane_id, new_actor_id, new_lane_kind, new_attribution_jsonb,
                resume_pointer_jsonb, event_ledger_event_id
            )
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
            RETURNING *
            "#,
        )
        .bind(&receipt_id)
        .bind(&checkpoint.checkpoint_id)
        .bind(&checkpoint.session_id)
        .bind(new_session_id)
        .bind(&new_lane.lane_id)
        .bind(&new_lane.actor_id)
        .bind(new_lane.lane_kind.as_str())
        .bind(lane_json)
        .bind(serde_json::to_value(&checkpoint.resume_pointer)?)
        .bind(&event_id)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        let receipt = recovery_receipt_from_row(row)?;
        Ok(RecoveredCheckpoint {
            resume_pointer: checkpoint.resume_pointer.clone(),
            checkpoint,
            receipt,
        })
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
        let status = if active.is_some() {
            IndexLeaseStatus::Queued
        } else {
            IndexLeaseStatus::Acquired
        };
        let blocked_by = active.map(|record| record.lease_id);
        match self
            .insert_indexing_lease_outcome(&request, status, blocked_by)
            .await
        {
            Ok(record) => Ok(record),
            Err(StateRecoveryError::Sqlx(sqlx::Error::Database(db_err)))
                if db_err.is_unique_violation() && status == IndexLeaseStatus::Acquired =>
            {
                let active = self.active_index_writer_for_scope(&request.scope).await?;
                let (retry_status, retry_blocked_by) = if let Some(active) = active {
                    (IndexLeaseStatus::Queued, Some(active.lease_id))
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
        match self
            .insert_indexing_lease_outcome(&request, IndexLeaseStatus::Acquired, None)
            .await
        {
            Ok(record) => Ok(Some(record)),
            Err(StateRecoveryError::Sqlx(sqlx::Error::Database(db_err)))
                if db_err.is_unique_violation() =>
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
        let lane_json = serde_json::to_value(&persistent_lane.attribution)?;
        let mut tx = self.pool.begin().await?;
        let inserted = sqlx::query(
            r#"
            INSERT INTO knowledge_parallel_indexing_lease_queue (
                lease_id, workspace_id, wp_id, mt_id, scope_kind, scope_id,
                lane_id, actor_id, lane_kind, attribution_jsonb, session_id,
                index_run_id, priority, ttl_seconds, quiet_policy_jsonb, status,
                blocked_by_lease_id, acquired_at_utc, expires_at_utc
            )
            VALUES (
                $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,
                CASE WHEN $16 = 'acquired' THEN NOW() ELSE NULL END,
                CASE WHEN $16 = 'acquired' THEN NOW() + ($14::BIGINT * INTERVAL '1 second') ELSE NULL END
            )
            "#,
        )
        .bind(&lease_id)
        .bind(&request.workspace_id)
        .bind(&request.wp_id)
        .bind(&request.mt_id)
        .bind(request.scope.kind_str())
        .bind(request.scope.scope_id())
        .bind(&persistent_lane.lane_id)
        .bind(&persistent_lane.actor_id)
        .bind(persistent_lane.lane_kind.as_str())
        .bind(lane_json)
        .bind(&request.session_id)
        .bind(&request.index_run_id)
        .bind(request.priority)
        .bind(request.ttl_seconds)
        .bind(serde_json::to_value(&request.quiet_policy)?)
        .bind(status.as_str())
        .bind(blocked_by.as_deref())
        .execute(&mut *tx)
        .await;
        if let Err(error) = inserted {
            let _ = tx.rollback().await;
            return Err(error.into());
        }

        let event_id = self
            .append_event_tx(
                &mut tx,
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
            )
            .await?;
        let row = sqlx::query(
            r#"
            UPDATE knowledge_parallel_indexing_lease_queue
               SET event_ledger_event_id = $2
             WHERE lease_id = $1
            RETURNING *
            "#,
        )
        .bind(&lease_id)
        .bind(&event_id)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        index_lease_from_row(row)
    }

    pub async fn active_index_writer_for_scope(
        &self,
        scope: &ClaimScope,
    ) -> StateRecoveryResult<Option<IndexingLeaseRecord>> {
        let row = sqlx::query(
            r#"
            SELECT * FROM knowledge_parallel_indexing_lease_queue
            WHERE scope_kind = $1
              AND scope_id = $2
              AND status = 'acquired'
              AND expires_at_utc > NOW()
            ORDER BY acquired_at_utc ASC
            LIMIT 1
            "#,
        )
        .bind(scope.kind_str())
        .bind(scope.scope_id())
        .fetch_optional(&self.pool)
        .await?;
        row.map(index_lease_from_row).transpose()
    }

    pub async fn complete_indexing_lease(
        &self,
        lease_id: &str,
        lane: &AgentLaneIdentity,
    ) -> StateRecoveryResult<bool> {
        let result = sqlx::query(
            r#"
            UPDATE knowledge_parallel_indexing_lease_queue
               SET status = 'completed', completed_at_utc = NOW()
             WHERE lease_id = $1
               AND actor_id = $2
               AND status = 'acquired'
            "#,
        )
        .bind(lease_id)
        .bind(&lane.actor_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn acquire_next_indexing_lease(
        &self,
        scope: &ClaimScope,
    ) -> StateRecoveryResult<Option<IndexingLeaseRecord>> {
        if self.active_index_writer_for_scope(scope).await?.is_some() {
            return Ok(None);
        }
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query(
            r#"
            UPDATE knowledge_parallel_indexing_lease_queue
               SET status = 'acquired',
                   blocked_by_lease_id = NULL,
                   acquired_at_utc = NOW(),
                   expires_at_utc = NOW() + (ttl_seconds::BIGINT * INTERVAL '1 second')
             WHERE lease_id = (
                 SELECT lease_id
                   FROM knowledge_parallel_indexing_lease_queue
                  WHERE scope_kind = $1
                    AND scope_id = $2
                    AND status = 'queued'
                  ORDER BY priority DESC, enqueued_at_utc ASC, lease_id ASC
                  LIMIT 1
             )
            RETURNING *
            "#,
        )
        .bind(scope.kind_str())
        .bind(scope.scope_id())
        .fetch_optional(&mut *tx)
        .await?;
        let Some(row) = row else {
            tx.commit().await?;
            return Ok(None);
        };
        let promoted = index_lease_from_row(row)?;
        let event_id = self
            .append_event_tx(
                &mut tx,
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
        let row = sqlx::query(
            r#"
            UPDATE knowledge_parallel_indexing_lease_queue
               SET event_ledger_event_id = $2
             WHERE lease_id = $1
            RETURNING *
            "#,
        )
        .bind(&promoted.lease_id)
        .bind(&event_id)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        index_lease_from_row(row).map(Some)
    }

    pub async fn reclaim_orphaned_indexing_leases(
        &self,
    ) -> StateRecoveryResult<Vec<IndexingLeaseRecord>> {
        let mut tx = self.pool.begin().await?;
        let rows = sqlx::query(
            r#"
            SELECT * FROM knowledge_parallel_indexing_lease_queue
             WHERE status = 'acquired'
               AND expires_at_utc <= NOW()
             ORDER BY acquired_at_utc ASC, lease_id ASC
             FOR UPDATE
            "#,
        )
        .fetch_all(&mut *tx)
        .await?;
        let mut reclaimed = Vec::with_capacity(rows.len());
        for row in rows {
            let lease = index_lease_from_row(row)?;
            let event_id = self
                .append_event_tx(
                    &mut tx,
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
            let row = sqlx::query(
                r#"
                UPDATE knowledge_parallel_indexing_lease_queue
                   SET status = 'reclaimed',
                       completed_at_utc = NOW(),
                       event_ledger_event_id = $2
                 WHERE lease_id = $1
                   AND status = 'acquired'
                RETURNING *
                "#,
            )
            .bind(&lease.lease_id)
            .bind(&event_id)
            .fetch_one(&mut *tx)
            .await?;
            reclaimed.push(index_lease_from_row(row)?);
        }
        tx.commit().await?;
        Ok(reclaimed)
    }

    async fn active_claim_for_scope(
        &self,
        scope: &ClaimScope,
    ) -> StateRecoveryResult<Option<WorkClaimRecord>> {
        let row = sqlx::query(
            r#"
            SELECT * FROM knowledge_agent_worktree_claims
            WHERE scope_kind = $1
              AND scope_id = $2
              AND status = 'active'
              AND released_at_utc IS NULL
              AND expires_at_utc > NOW()
            ORDER BY claimed_at_utc ASC
            LIMIT 1
            "#,
        )
        .bind(scope.kind_str())
        .bind(scope.scope_id())
        .fetch_optional(&self.pool)
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
        let mut tx = self.pool.begin().await?;
        let rows = sqlx::query(
            r#"
            SELECT * FROM knowledge_agent_worktree_claims
             WHERE status = 'active'
               AND released_at_utc IS NULL
               AND expires_at_utc <= NOW()
             ORDER BY claimed_at_utc ASC, claim_id ASC
             FOR UPDATE
            "#,
        )
        .fetch_all(&mut *tx)
        .await?;
        let mut reclaimed = Vec::with_capacity(rows.len());
        for row in rows {
            let claim = work_claim_from_row(row)?;
            let event_id = self
                .append_event_tx(
                    &mut tx,
                    KernelEventType::SessionCancelled,
                    "parallel_swarm_claim_reclaim",
                    &claim.claim_id,
                    &reclaimer,
                    session_id,
                    json!({
                        "schema_id": "hsk.parallel_swarm.claim_reclaim@1",
                        "claim_id": &claim.claim_id,
                        "workspace_id": &claim.workspace_id,
                        "wp_id": &claim.wp_id,
                        "mt_id": &claim.mt_id,
                        "scope": &claim.scope,
                        "prior_lane": &claim.lane,
                        "reclaimed_by_lane": &reclaimer,
                        "reason": reason,
                    }),
                )
                .await?;
            let row = sqlx::query(
                r#"
                UPDATE knowledge_agent_worktree_claims
                   SET status = 'reclaimed',
                       released_at_utc = NOW(),
                       reason = $2,
                       reclaim_event_ledger_event_id = $3
                 WHERE claim_id = $1
                   AND status = 'active'
                RETURNING *
                "#,
            )
            .bind(&claim.claim_id)
            .bind(reason)
            .bind(&event_id)
            .fetch_one(&mut *tx)
            .await?;
            reclaimed.push(work_claim_from_row(row)?);
        }
        tx.commit().await?;
        Ok(reclaimed)
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

    async fn append_event_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
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
        let payload = String::from_utf8(crate::kernel::context_bundle::canonical_json_bytes(
            &event.payload,
        ))
        .expect("canonical JSON is valid UTF-8");
        sqlx::query_scalar(
            r#"
            INSERT INTO kernel_event_ledger (
                event_id,
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
                created_at
            )
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15::jsonb,$16)
            RETURNING event_id
            "#,
        )
        .bind(&kernel_event.event_id)
        .bind(&event.event_version)
        .bind(&event.kernel_task_run_id)
        .bind(&event.session_run_id)
        .bind(&event.aggregate_type)
        .bind(&event.aggregate_id)
        .bind(&event.idempotency_key)
        .bind(event.event_type.as_str())
        .bind(event.actor.actor_kind())
        .bind(event.actor.actor_id())
        .bind(event.causation_id.as_deref())
        .bind(event.correlation_id.as_deref())
        .bind(&event.payload_hash)
        .bind(&event.source_component)
        .bind(payload)
        .bind(kernel_event.created_at)
        .fetch_one(&mut **tx)
        .await
        .map_err(StateRecoveryError::from)
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
            if source_ref.row_source_ref != format!("postgres://{}/{}", expected_table, row_id) {
                errors.push(format!("{row_kind} {row_id} has mismatched row source ref"));
            }
            if source_ref.row_source_ref.trim().is_empty()
                || !source_ref.row_source_ref.starts_with("postgres://")
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
        row_source_ref: format!("postgres://{table_name}/{row_id}"),
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

fn dashboard_group_count_map(
    rows: Vec<PgRow>,
    key_column: &str,
) -> StateRecoveryResult<BTreeMap<String, i64>> {
    rows.into_iter()
        .map(|row| {
            let key: String = row.try_get(key_column)?;
            let count: i64 = row.try_get("row_count")?;
            Ok((key, count))
        })
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

fn work_claim_from_row(row: PgRow) -> StateRecoveryResult<WorkClaimRecord> {
    let scope = scope_from_parts(
        row.try_get("scope_kind")?,
        row.try_get::<String, _>("scope_id")?,
    )?;
    let lane = lane_from_parts(
        row.try_get("lane_id")?,
        row.try_get("actor_id")?,
        row.try_get("lane_kind")?,
        row.try_get("attribution_jsonb")?,
    )?;
    Ok(WorkClaimRecord {
        claim_id: row.try_get("claim_id")?,
        workspace_id: row.try_get("workspace_id")?,
        wp_id: row.try_get("wp_id")?,
        mt_id: row.try_get("mt_id")?,
        scope,
        lane,
        session_id: row.try_get("session_id")?,
        status: ClaimStatus::parse(row.try_get::<String, _>("status")?.as_str())?,
        reason: row.try_get("reason")?,
        claimed_at_utc: row.try_get("claimed_at_utc")?,
        expires_at_utc: row.try_get("expires_at_utc")?,
        released_at_utc: row.try_get("released_at_utc")?,
        event_ledger_event_id: row.try_get("event_ledger_event_id")?,
        release_event_ledger_event_id: row.try_get("release_event_ledger_event_id")?,
        reclaim_event_ledger_event_id: row.try_get("reclaim_event_ledger_event_id")?,
    })
}

fn mailbox_handoff_from_row(row: PgRow) -> StateRecoveryResult<RoleMailboxHandoffRecord> {
    let lane = lane_from_parts(
        row.try_get("from_lane_id")?,
        row.try_get("from_actor_id")?,
        row.try_get("from_lane_kind")?,
        row.try_get("from_attribution_jsonb")?,
    )?;
    Ok(RoleMailboxHandoffRecord {
        handoff_id: row.try_get("handoff_id")?,
        wp_id: row.try_get("wp_id")?,
        mt_id: row.try_get("mt_id")?,
        claim_id: row.try_get("claim_id")?,
        from_lane: lane,
        to_role: row.try_get("to_role")?,
        mailbox_thread_id: row.try_get("mailbox_thread_id")?,
        mailbox_message_id: row.try_get("mailbox_message_id")?,
        status: SwarmReceiptStatus::parse(row.try_get::<String, _>("status")?.as_str())?,
        summary: row.try_get("summary")?,
        body_sha256: row.try_get("body_sha256")?,
        event_ledger_event_id: row.try_get("event_ledger_event_id")?,
        created_at_utc: row.try_get("created_at_utc")?,
    })
}

fn checkpoint_from_row(row: PgRow) -> StateRecoveryResult<RecoveryCheckpointRecord> {
    let lane = lane_from_parts(
        row.try_get("lane_id")?,
        row.try_get("actor_id")?,
        row.try_get("lane_kind")?,
        row.try_get("attribution_jsonb")?,
    )?;
    let resume_pointer: RecoveryResumePointer =
        serde_json::from_value(row.try_get("resume_pointer_jsonb")?)?;
    Ok(RecoveryCheckpointRecord {
        checkpoint_id: row.try_get("checkpoint_id")?,
        lane,
        session_id: row.try_get("session_id")?,
        workspace_id: row.try_get("workspace_id")?,
        wp_id: row.try_get("wp_id")?,
        mt_id: row.try_get("mt_id")?,
        claim_id: row.try_get("claim_id")?,
        mailbox_handoff_id: row.try_get("mailbox_handoff_id")?,
        navigation_command_id: row.try_get("navigation_command_id")?,
        resume_pointer,
        touched_files: serde_json::from_value(row.try_get("touched_files_jsonb")?)?,
        tests: serde_json::from_value(row.try_get("tests_jsonb")?)?,
        hbr_rows: serde_json::from_value(row.try_get("hbr_rows_jsonb")?)?,
        next_step_context: row.try_get("next_step_context")?,
        payload: row.try_get("payload_jsonb")?,
        payload_sha256: row.try_get("payload_sha256")?,
        compaction_reason: row.try_get("compaction_reason")?,
        git_head: row.try_get("git_head")?,
        event_ledger_event_id: row.try_get("event_ledger_event_id")?,
        created_at_utc: row.try_get("created_at_utc")?,
    })
}

fn recovery_receipt_from_row(row: PgRow) -> StateRecoveryResult<RecoveryReceiptRecord> {
    let new_lane = lane_from_parts(
        row.try_get("new_lane_id")?,
        row.try_get("new_actor_id")?,
        row.try_get("new_lane_kind")?,
        row.try_get("new_attribution_jsonb")?,
    )?;
    Ok(RecoveryReceiptRecord {
        receipt_id: row.try_get("receipt_id")?,
        checkpoint_id: row.try_get("checkpoint_id")?,
        prior_session_id: row.try_get("prior_session_id")?,
        new_session_id: row.try_get("new_session_id")?,
        new_lane,
        resume_pointer: serde_json::from_value(row.try_get("resume_pointer_jsonb")?)?,
        event_ledger_event_id: row.try_get("event_ledger_event_id")?,
        recovered_at_utc: row.try_get("recovered_at_utc")?,
    })
}

fn quiet_background_work_from_row(row: PgRow) -> StateRecoveryResult<QuietBackgroundWorkRecord> {
    let lane = lane_from_parts(
        row.try_get("lane_id")?,
        row.try_get("actor_id")?,
        row.try_get("lane_kind")?,
        row.try_get("attribution_jsonb")?,
    )?;
    let policy: QuietBackgroundPolicy = serde_json::from_value(row.try_get("quiet_policy_jsonb")?)?;
    Ok(QuietBackgroundWorkRecord {
        receipt_id: row.try_get("receipt_id")?,
        workspace_id: row.try_get("workspace_id")?,
        wp_id: row.try_get("wp_id")?,
        mt_id: row.try_get("mt_id")?,
        work_kind: QuietBackgroundWorkKind::parse(row.try_get::<String, _>("work_kind")?.as_str())?,
        subject_id: row.try_get("subject_id")?,
        lane,
        session_id: row.try_get("session_id")?,
        policy,
        evidence_ref: row.try_get("evidence_ref")?,
        event_ledger_event_id: row.try_get("event_ledger_event_id")?,
        created_at_utc: row.try_get("created_at_utc")?,
    })
}

fn index_lease_from_row(row: PgRow) -> StateRecoveryResult<IndexingLeaseRecord> {
    let scope = scope_from_parts(
        row.try_get("scope_kind")?,
        row.try_get::<String, _>("scope_id")?,
    )?;
    let lane = lane_from_parts(
        row.try_get("lane_id")?,
        row.try_get("actor_id")?,
        row.try_get("lane_kind")?,
        row.try_get("attribution_jsonb")?,
    )?;
    Ok(IndexingLeaseRecord {
        lease_id: row.try_get("lease_id")?,
        workspace_id: row.try_get("workspace_id")?,
        wp_id: row.try_get("wp_id")?,
        mt_id: row.try_get("mt_id")?,
        scope,
        lane,
        session_id: row.try_get("session_id")?,
        index_run_id: row.try_get("index_run_id")?,
        priority: row.try_get("priority")?,
        ttl_seconds: row.try_get("ttl_seconds")?,
        status: IndexLeaseStatus::parse(row.try_get::<String, _>("status")?.as_str())?,
        blocked_by_lease_id: row.try_get("blocked_by_lease_id")?,
        quiet_policy: serde_json::from_value(row.try_get("quiet_policy_jsonb")?)?,
        event_ledger_event_id: row.try_get("event_ledger_event_id")?,
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

fn ensure_sha256(value: &str) -> StateRecoveryResult<()> {
    if value.len() == 64
        && value
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, 'a'..='f'))
    {
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
