//! WP-KERNEL-009 MT-209..216 ParallelSwarmStateRecovery backend foundations.
//!
//! This module is intentionally backend-only. It gives local/cloud model lanes
//! typed identity, claim leases over shared worktrees/workspaces, role-mailbox
//! handoff receipts, deterministic backend navigation commands, restartable
//! compaction checkpoints, recovery receipts, and a serial lease queue for
//! parallel index writers. One embedded SurrealDB authority stores every row;
//! canonical EventLedger rows provide the receipt trail.

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
#[cfg(feature = "surreal-test-support")]
use std::sync::atomic::{AtomicBool, Ordering};
use surrealdb::types::SurrealValue;
use thiserror::Error;
use tokio::sync::OnceCell;
use uuid::Uuid;

use crate::kernel::{
    sandbox::{EnvRedactionV1, Redactor},
    KernelActor, KernelEvent, KernelEventType, NewKernelEvent,
};
use crate::storage::surreal::{SurrealStorage, SurrealStorageError};
use crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution;

#[derive(Debug, Error)]
pub enum StateRecoveryError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("kernel event error: {0}")]
    Kernel(String),
    #[error("embedded SurrealDB error: {0}")]
    Surreal(#[from] SurrealStorageError),
    #[error("resource access lifecycle denied: {0}")]
    AccessLifecycle(String),
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
    "parallel_swarm_state_recovery_authority",
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

const STATE_RECOVERY_SCHEMA: &str = include_str!("../storage/surreal/state_recovery_schema.surql");
const STATE_RECOVERY_AUTHORITY_TABLE: &str = "parallel_swarm_state_recovery_authority";
const STATE_RECOVERY_LIFECYCLE_TABLE: &str = "authenticated_resource_context_lifecycle";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StateRecoveryAccessLifecycle {
    Active {
        generation: i64,
        revocation_epoch: i64,
    },
    Stale {
        generation: i64,
        revocation_epoch: i64,
    },
    Revoked {
        generation: i64,
        revocation_epoch: i64,
    },
}

#[async_trait]
pub trait StateRecoveryAccessLifecycleResolver: Send + Sync {
    async fn resolve(
        &self,
        scope: &ExactResourceScopeAttribution,
    ) -> StateRecoveryResult<StateRecoveryAccessLifecycle>;
}

#[derive(Clone)]
pub struct SurrealStateRecoveryAccessLifecycleResolver {
    storage: SurrealStorage,
}

impl SurrealStateRecoveryAccessLifecycleResolver {
    pub fn new(storage: SurrealStorage) -> Self {
        Self { storage }
    }
}

#[derive(Debug, SurrealValue)]
struct EmptyStateRecoveryBindings {}

#[derive(Debug, Clone, SurrealValue)]
struct ExactScopeBindings {
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(Debug, SurrealValue)]
struct LifecycleRow {
    lifecycle_state: String,
    generation: i64,
    revocation_epoch: i64,
}

#[derive(Debug, SurrealValue)]
struct LifecycleWriteBindings {
    record_id: String,
    lifecycle_state: String,
    generation: i64,
    revocation_epoch: i64,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(Debug, SurrealValue)]
struct AuthorityLookupBindings {
    record_kind: String,
    aggregate_id: String,
    workspace_id_filter: String,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(Debug, SurrealValue)]
struct AuthorityRow {
    record_json: String,
    event_id: String,
    event_payload_hash: String,
}

#[derive(Debug, SurrealValue)]
struct EventLookupBindings {
    event_ids: Vec<String>,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(Debug, SurrealValue)]
struct EventWatermarkRow {
    event_id: String,
    source_component: String,
    aggregate_type: String,
    aggregate_id: String,
    created_at: surrealdb::types::Datetime,
}

#[derive(Debug, SurrealValue)]
struct AuthorityWriteBindings {
    record_id: String,
    record_kind: String,
    aggregate_id: String,
    workspace_id_filter: String,
    wp_id: String,
    mt_id: String,
    status: String,
    scope_kind: String,
    scope_id: String,
    record_json: String,
    expected_event_id: String,
    create_only: bool,
    event_id: String,
    event_version: String,
    kernel_task_run_id: String,
    session_run_id: String,
    event_aggregate_type: String,
    event_aggregate_id: String,
    idempotency_key: String,
    event_type: String,
    actor_kind: String,
    actor_id: String,
    causation_id: Option<String>,
    correlation_id: Option<String>,
    event_payload_hash: String,
    source_component: String,
    event_payload: Value,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
    fail_after_event: bool,
}

#[derive(Clone)]
pub struct ParallelSwarmStateRecoveryStore {
    storage: SurrealStorage,
    scope: ExactResourceScopeAttribution,
    lifecycle: Arc<dyn StateRecoveryAccessLifecycleResolver>,
    schema_ready: Arc<OnceCell<()>>,
    #[cfg(feature = "surreal-test-support")]
    fail_after_event: Arc<AtomicBool>,
}

impl std::fmt::Debug for ParallelSwarmStateRecoveryStore {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ParallelSwarmStateRecoveryStore")
            .field("config", self.storage.config())
            .field("scope", &self.scope)
            .finish_non_exhaustive()
    }
}

async fn bootstrap_state_recovery_schema(storage: &SurrealStorage) -> StateRecoveryResult<()> {
    storage
        .with_data_operation(|database| {
            Box::pin(async move {
                let _ = database
                    .query_values::<surrealdb::types::Value, _>(
                        STATE_RECOVERY_SCHEMA,
                        EmptyStateRecoveryBindings {},
                    )
                    .await?;
                Ok(())
            })
        })
        .await?;
    Ok(())
}

fn exact_scope_bindings(scope: &ExactResourceScopeAttribution) -> ExactScopeBindings {
    ExactScopeBindings {
        owner_account_id: scope.owner_account_id.as_uuid().to_string(),
        actor_principal_id: scope.actor_principal_id.as_uuid().to_string(),
        authenticated_session_id: scope.authenticated_session_id.as_uuid().to_string(),
        access_space_id: scope.access_space_id.as_uuid().to_string(),
        workspace_id: scope.workspace_id.as_str().to_string(),
    }
}

fn lifecycle_record_id(scope: &ExactResourceScopeAttribution) -> String {
    let scope = exact_scope_bindings(scope);
    sha256_hex(
        format!(
            "hsk-state-recovery-lifecycle-v1\0{}\0{}\0{}\0{}\0{}",
            scope.owner_account_id,
            scope.actor_principal_id,
            scope.authenticated_session_id,
            scope.access_space_id,
            scope.workspace_id
        )
        .as_bytes(),
    )
}

#[async_trait]
impl StateRecoveryAccessLifecycleResolver for SurrealStateRecoveryAccessLifecycleResolver {
    async fn resolve(
        &self,
        scope: &ExactResourceScopeAttribution,
    ) -> StateRecoveryResult<StateRecoveryAccessLifecycle> {
        bootstrap_state_recovery_schema(&self.storage).await?;
        let bindings = exact_scope_bindings(scope);
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values::<LifecycleRow, _>(
                            "SELECT lifecycle_state, generation, revocation_epoch \
                             FROM authenticated_resource_context_lifecycle \
                             WHERE owner_account_id = $owner_account_id \
                               AND actor_principal_id = $actor_principal_id \
                               AND authenticated_session_id = $authenticated_session_id \
                               AND access_space_id = $access_space_id \
                               AND workspace_id = $workspace_id LIMIT 2;",
                            bindings,
                        )
                        .await
                })
            })
            .await?;
        let row = match rows.as_slice() {
            [row] => row,
            [] => {
                return Err(StateRecoveryError::AccessLifecycle(
                    "canonical context lifecycle is absent".to_string(),
                ))
            }
            _ => {
                return Err(StateRecoveryError::AccessLifecycle(
                    "canonical context lifecycle is ambiguous".to_string(),
                ))
            }
        };
        match row.lifecycle_state.as_str() {
            "active" => Ok(StateRecoveryAccessLifecycle::Active {
                generation: row.generation,
                revocation_epoch: row.revocation_epoch,
            }),
            "stale" => Ok(StateRecoveryAccessLifecycle::Stale {
                generation: row.generation,
                revocation_epoch: row.revocation_epoch,
            }),
            "revoked" => Ok(StateRecoveryAccessLifecycle::Revoked {
                generation: row.generation,
                revocation_epoch: row.revocation_epoch,
            }),
            _ => Err(StateRecoveryError::AccessLifecycle(
                "canonical context lifecycle has an invalid state".to_string(),
            )),
        }
    }
}

#[cfg(feature = "surreal-test-support")]
#[derive(Clone)]
pub struct StateRecoveryAccessLifecycleTestAuthority {
    storage: SurrealStorage,
}

#[cfg(feature = "surreal-test-support")]
impl StateRecoveryAccessLifecycleTestAuthority {
    pub fn new(storage: SurrealStorage) -> Self {
        Self { storage }
    }

    pub async fn activate(
        &self,
        scope: &ExactResourceScopeAttribution,
        generation: i64,
        revocation_epoch: i64,
    ) -> StateRecoveryResult<()> {
        self.set(scope, "active", generation, revocation_epoch)
            .await
    }

    pub async fn mark_stale(
        &self,
        scope: &ExactResourceScopeAttribution,
        generation: i64,
        revocation_epoch: i64,
    ) -> StateRecoveryResult<()> {
        self.set(scope, "stale", generation, revocation_epoch).await
    }

    pub async fn revoke(
        &self,
        scope: &ExactResourceScopeAttribution,
        generation: i64,
        revocation_epoch: i64,
    ) -> StateRecoveryResult<()> {
        self.set(scope, "revoked", generation, revocation_epoch)
            .await
    }

    async fn set(
        &self,
        scope: &ExactResourceScopeAttribution,
        lifecycle_state: &str,
        generation: i64,
        revocation_epoch: i64,
    ) -> StateRecoveryResult<()> {
        if generation <= 0 || revocation_epoch < 0 {
            return Err(StateRecoveryError::InvalidInput(
                "lifecycle generation and revocation epoch are invalid".to_string(),
            ));
        }
        bootstrap_state_recovery_schema(&self.storage).await?;
        let exact = exact_scope_bindings(scope);
        let bindings = LifecycleWriteBindings {
            record_id: lifecycle_record_id(scope),
            lifecycle_state: lifecycle_state.to_string(),
            generation,
            revocation_epoch,
            owner_account_id: exact.owner_account_id,
            actor_principal_id: exact.actor_principal_id,
            authenticated_session_id: exact.authenticated_session_id,
            access_space_id: exact.access_space_id,
            workspace_id: exact.workspace_id,
        };
        self.storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    let _ = database
                        .query_values_at::<surrealdb::types::Value, _>(
                            "BEGIN TRANSACTION; \
                             LET $record = type::record('authenticated_resource_context_lifecycle', $record_id); \
                             LET $existing = (SELECT id FROM $record \
                               WHERE owner_account_id = $owner_account_id \
                                 AND actor_principal_id = $actor_principal_id \
                                 AND authenticated_session_id = $authenticated_session_id \
                                 AND access_space_id = $access_space_id \
                                 AND workspace_id = $workspace_id); \
                             IF array::len($existing) = 0 { \
                               RETURN CREATE $record CONTENT { owner_account_id: $owner_account_id, \
                                 actor_principal_id: $actor_principal_id, \
                                 authenticated_session_id: $authenticated_session_id, \
                                 access_space_id: $access_space_id, workspace_id: $workspace_id, \
                                 lifecycle_state: $lifecycle_state, generation: $generation, \
                                 revocation_epoch: $revocation_epoch }; \
                             } ELSE { \
                               RETURN UPDATE $record SET lifecycle_state = $lifecycle_state, \
                                 generation = $generation, revocation_epoch = $revocation_epoch \
                                 WHERE owner_account_id = $owner_account_id \
                                   AND actor_principal_id = $actor_principal_id \
                                   AND authenticated_session_id = $authenticated_session_id \
                                   AND access_space_id = $access_space_id \
                                   AND workspace_id = $workspace_id; \
                             }; \
                             COMMIT TRANSACTION;",
                            bindings,
                            3,
                        )
                        .await?;
                    Ok(())
                })
            })
            .await?;
        Ok(())
    }
}

impl ParallelSwarmStateRecoveryStore {
    pub fn new(
        storage: SurrealStorage,
        scope: ExactResourceScopeAttribution,
        lifecycle: Arc<dyn StateRecoveryAccessLifecycleResolver>,
    ) -> Self {
        Self {
            storage,
            scope,
            lifecycle,
            schema_ready: Arc::new(OnceCell::new()),
            #[cfg(feature = "surreal-test-support")]
            fail_after_event: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn with_surreal_lifecycle(
        storage: SurrealStorage,
        scope: ExactResourceScopeAttribution,
    ) -> Self {
        let lifecycle = Arc::new(SurrealStateRecoveryAccessLifecycleResolver::new(
            storage.clone(),
        ));
        Self::new(storage, scope, lifecycle)
    }

    pub fn storage(&self) -> &SurrealStorage {
        &self.storage
    }

    pub fn exact_scope(&self) -> &ExactResourceScopeAttribution {
        &self.scope
    }

    #[cfg(feature = "surreal-test-support")]
    pub fn fail_next_commit_after_event_for_test(&self) {
        self.fail_after_event.store(true, Ordering::SeqCst);
    }

    async fn ensure_schema(&self) -> StateRecoveryResult<()> {
        self.schema_ready
            .get_or_try_init(|| async { bootstrap_state_recovery_schema(&self.storage).await })
            .await?;
        Ok(())
    }

    async fn ensure_active_access(&self) -> StateRecoveryResult<(i64, i64)> {
        self.ensure_schema().await?;
        match self.lifecycle.resolve(&self.scope).await? {
            StateRecoveryAccessLifecycle::Active {
                generation,
                revocation_epoch,
            } if generation > 0 && revocation_epoch >= 0 => Ok((generation, revocation_epoch)),
            StateRecoveryAccessLifecycle::Active { .. } => {
                Err(StateRecoveryError::AccessLifecycle(
                    "canonical context lifecycle has invalid counters".to_string(),
                ))
            }
            StateRecoveryAccessLifecycle::Stale { .. } => Err(StateRecoveryError::AccessLifecycle(
                "canonical context lifecycle is stale".to_string(),
            )),
            StateRecoveryAccessLifecycle::Revoked { .. } => {
                Err(StateRecoveryError::AccessLifecycle(
                    "canonical context lifecycle is revoked".to_string(),
                ))
            }
        }
    }

    fn ensure_workspace(&self, workspace_id: &str) -> StateRecoveryResult<()> {
        ensure_safe_token("workspace_id", workspace_id)?;
        if workspace_id != self.scope.workspace_id.as_str() {
            return Err(StateRecoveryError::AccessLifecycle(
                "workspace does not match the exact server-owned scope".to_string(),
            ));
        }
        Ok(())
    }

    fn lookup_bindings(&self, kind: &str, aggregate_id: &str) -> AuthorityLookupBindings {
        let exact = exact_scope_bindings(&self.scope);
        AuthorityLookupBindings {
            record_kind: kind.to_string(),
            aggregate_id: aggregate_id.to_string(),
            workspace_id_filter: self.scope.workspace_id.as_str().to_string(),
            owner_account_id: exact.owner_account_id,
            actor_principal_id: exact.actor_principal_id,
            authenticated_session_id: exact.authenticated_session_id,
            access_space_id: exact.access_space_id,
            workspace_id: exact.workspace_id,
        }
    }

    async fn load_rows<T>(&self, kind: &str) -> StateRecoveryResult<Vec<T>>
    where
        T: for<'de> Deserialize<'de>,
    {
        self.ensure_active_access().await?;
        let bindings = self.lookup_bindings(kind, "");
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values::<AuthorityRow, _>(
                            "SELECT record_json, record::id(event_ledger_event_id) AS event_id, event_payload_hash \
                             FROM parallel_swarm_state_recovery_authority \
                             WHERE record_kind = $record_kind \
                               AND workspace_id = $workspace_id_filter \
                               AND owner_account_id = $owner_account_id \
                               AND actor_principal_id = $actor_principal_id \
                               AND authenticated_session_id = $authenticated_session_id \
                               AND access_space_id = $access_space_id \
                               AND workspace_id = $workspace_id \
                               AND event_ledger_event_id.owner_account_id = $owner_account_id \
                               AND event_ledger_event_id.actor_principal_id = $actor_principal_id \
                               AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id \
                               AND event_ledger_event_id.access_space_id = $access_space_id \
                               AND event_ledger_event_id.workspace_id = $workspace_id \
                               AND event_ledger_event_id.event_id = record::id(event_ledger_event_id) \
                               AND event_ledger_event_id.payload_hash = event_payload_hash \
                               AND event_ledger_event_id.source_component = 'parallel_swarm_state_recovery' \
                               AND array::len((SELECT VALUE id FROM authenticated_resource_context_lifecycle \
                                 WHERE lifecycle_state = 'active' \
                                   AND owner_account_id = $owner_account_id \
                                   AND actor_principal_id = $actor_principal_id \
                                   AND authenticated_session_id = $authenticated_session_id \
                                   AND access_space_id = $access_space_id \
                                   AND workspace_id = $workspace_id)) = 1 \
                             ORDER BY updated_at DESC, aggregate_id DESC;",
                            bindings,
                        )
                        .await
                })
            })
            .await?;
        rows.into_iter()
            .map(|row| {
                let value: T = serde_json::from_str(&row.record_json)?;
                Ok(value)
            })
            .collect()
    }

    async fn load_one<T>(&self, kind: &str, aggregate_id: &str) -> StateRecoveryResult<Option<T>>
    where
        T: for<'de> Deserialize<'de>,
    {
        self.ensure_active_access().await?;
        let bindings = self.lookup_bindings(kind, aggregate_id);
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values::<AuthorityRow, _>(
                            "SELECT record_json, record::id(event_ledger_event_id) AS event_id, event_payload_hash \
                             FROM parallel_swarm_state_recovery_authority \
                             WHERE record_kind = $record_kind AND aggregate_id = $aggregate_id \
                               AND workspace_id = $workspace_id_filter \
                               AND owner_account_id = $owner_account_id \
                               AND actor_principal_id = $actor_principal_id \
                               AND authenticated_session_id = $authenticated_session_id \
                               AND access_space_id = $access_space_id \
                               AND workspace_id = $workspace_id \
                               AND event_ledger_event_id.owner_account_id = $owner_account_id \
                               AND event_ledger_event_id.actor_principal_id = $actor_principal_id \
                               AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id \
                               AND event_ledger_event_id.access_space_id = $access_space_id \
                               AND event_ledger_event_id.workspace_id = $workspace_id \
                               AND event_ledger_event_id.event_id = record::id(event_ledger_event_id) \
                               AND event_ledger_event_id.payload_hash = event_payload_hash \
                               AND event_ledger_event_id.source_component = 'parallel_swarm_state_recovery' \
                               AND array::len((SELECT VALUE id FROM authenticated_resource_context_lifecycle \
                                 WHERE lifecycle_state = 'active' \
                                   AND owner_account_id = $owner_account_id \
                                   AND actor_principal_id = $actor_principal_id \
                                   AND authenticated_session_id = $authenticated_session_id \
                                   AND access_space_id = $access_space_id \
                                   AND workspace_id = $workspace_id)) = 1 LIMIT 2;",
                            bindings,
                        )
                        .await
                })
            })
            .await?;
        match rows.as_slice() {
            [] => Ok(None),
            [row] => Ok(Some(serde_json::from_str(&row.record_json)?)),
            _ => Err(StateRecoveryError::AccessLifecycle(
                "exact-scope authority lookup was ambiguous".to_string(),
            )),
        }
    }

    fn build_event(
        event_type: KernelEventType,
        aggregate_type: &str,
        aggregate_id: &str,
        lane: &AgentLaneIdentity,
        session_id: &str,
        payload: Value,
    ) -> StateRecoveryResult<(NewKernelEvent, KernelEvent)> {
        let event = NewKernelEvent::builder(
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
        .source_component(PARALLEL_SWARM_SOURCE_COMPONENT)
        .payload(payload)
        .build()
        .map_err(|error| StateRecoveryError::Kernel(error.to_string()))?;
        let stored = KernelEvent::from_new(event.clone());
        Ok((event, stored))
    }

    #[allow(clippy::too_many_arguments)]
    async fn persist_json(
        &self,
        kind: &str,
        aggregate_id: &str,
        workspace_id: &str,
        wp_id: Option<&str>,
        mt_id: Option<&str>,
        status: &str,
        claim_scope: Option<&ClaimScope>,
        record_json: String,
        expected_event_id: Option<&str>,
        create_only: bool,
        event: NewKernelEvent,
        stored_event: KernelEvent,
    ) -> StateRecoveryResult<AuthorityRow> {
        self.ensure_workspace(workspace_id)?;
        self.ensure_active_access().await?;
        let exact = exact_scope_bindings(&self.scope);
        let (scope_kind, scope_id) = claim_scope
            .map(|scope| (scope.kind_str().to_string(), scope.scope_id()))
            .unwrap_or_else(|| (String::new(), String::new()));
        let record_id = sha256_hex(
            format!(
                "hsk-state-recovery-authority-v1\0{kind}\0{aggregate_id}\0{}\0{}\0{}\0{}\0{}",
                exact.owner_account_id,
                exact.actor_principal_id,
                exact.authenticated_session_id,
                exact.access_space_id,
                exact.workspace_id
            )
            .as_bytes(),
        );
        #[cfg(feature = "surreal-test-support")]
        let fail_after_event = self.fail_after_event.swap(false, Ordering::SeqCst);
        #[cfg(not(feature = "surreal-test-support"))]
        let fail_after_event = false;
        let bindings = AuthorityWriteBindings {
            record_id,
            record_kind: kind.to_string(),
            aggregate_id: aggregate_id.to_string(),
            workspace_id_filter: workspace_id.to_string(),
            wp_id: wp_id.unwrap_or_default().to_string(),
            mt_id: mt_id.unwrap_or_default().to_string(),
            status: status.to_string(),
            scope_kind,
            scope_id,
            record_json,
            expected_event_id: expected_event_id.unwrap_or_default().to_string(),
            create_only,
            event_id: stored_event.event_id.clone(),
            event_version: event.event_version,
            kernel_task_run_id: event.kernel_task_run_id,
            session_run_id: event.session_run_id,
            event_aggregate_type: event.aggregate_type,
            event_aggregate_id: event.aggregate_id,
            idempotency_key: event.idempotency_key,
            event_type: event.event_type.as_str().to_string(),
            actor_kind: event.actor.actor_kind().to_string(),
            actor_id: event.actor.actor_id().to_string(),
            causation_id: event.causation_id,
            correlation_id: event.correlation_id,
            event_payload_hash: event.payload_hash,
            source_component: event.source_component,
            event_payload: event.payload,
            owner_account_id: exact.owner_account_id,
            actor_principal_id: exact.actor_principal_id,
            authenticated_session_id: exact.authenticated_session_id,
            access_space_id: exact.access_space_id,
            workspace_id: exact.workspace_id,
            fail_after_event,
        };
        let rows = self.storage.with_data_operation(|database| Box::pin(async move {
            database.query_values_at::<AuthorityRow, _>(
                "BEGIN TRANSACTION; \
                 LET $lifecycle = (SELECT id FROM authenticated_resource_context_lifecycle \
                   WHERE lifecycle_state = 'active' \
                     AND owner_account_id = $owner_account_id \
                     AND actor_principal_id = $actor_principal_id \
                     AND authenticated_session_id = $authenticated_session_id \
                     AND access_space_id = $access_space_id \
                     AND workspace_id = $workspace_id LIMIT 2); \
                 IF array::len($lifecycle) != 1 { THROW 'STATE_RECOVERY_CONTEXT_NOT_ACTIVE'; }; \
                 LET $record = type::record('parallel_swarm_state_recovery_authority', $record_id); \
                 LET $existing = (SELECT record_json, record::id(event_ledger_event_id) AS event_id, event_payload_hash FROM $record \
                   WHERE workspace_id = $workspace_id_filter \
                     AND owner_account_id = $owner_account_id \
                     AND actor_principal_id = $actor_principal_id \
                     AND authenticated_session_id = $authenticated_session_id \
                     AND access_space_id = $access_space_id \
                     AND workspace_id = $workspace_id \
                     AND event_ledger_event_id.owner_account_id = $owner_account_id \
                     AND event_ledger_event_id.actor_principal_id = $actor_principal_id \
                     AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id \
                     AND event_ledger_event_id.access_space_id = $access_space_id \
                     AND event_ledger_event_id.workspace_id = $workspace_id \
                     AND event_ledger_event_id.event_id = record::id(event_ledger_event_id) \
                     AND event_ledger_event_id.payload_hash = event_payload_hash \
                     AND event_ledger_event_id.source_component = 'parallel_swarm_state_recovery'); \
                 IF $create_only AND array::len($existing) > 0 { RETURN $existing; } ELSE { \
                   IF !$create_only AND (array::len($existing) != 1 OR $existing[0].event_id != $expected_event_id) { \
                     THROW 'STATE_RECOVERY_GENERATION_CONFLICT'; \
                   }; \
                   LET $prior = (SELECT event_id FROM kernel_event_ledger \
                     WHERE id = type::record('kernel_event_ledger', $event_id) \
                       AND owner_account_id = $owner_account_id \
                       AND actor_principal_id = $actor_principal_id \
                       AND authenticated_session_id = $authenticated_session_id \
                       AND access_space_id = $access_space_id \
                       AND workspace_id = $workspace_id LIMIT 2); \
                   IF array::len($prior) != 0 { THROW 'STATE_RECOVERY_EVENT_ID_REUSED'; }; \
                   LET $ledger = CREATE type::record('kernel_event_ledger', $event_id) CONTENT { \
                     event_id: $event_id, event_version: $event_version, \
                     kernel_task_run_id: $kernel_task_run_id, session_run_id: $session_run_id, \
                     aggregate_type: $event_aggregate_type, aggregate_id: $event_aggregate_id, \
                     idempotency_key: $idempotency_key, event_type: $event_type, \
                     actor_kind: $actor_kind, actor_id: $actor_id, causation_id: $causation_id, \
                     correlation_id: $correlation_id, payload_hash: $event_payload_hash, \
                     source_component: $source_component, payload: $event_payload, \
                     owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id, \
                     authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id, \
                     workspace_id: $workspace_id, created_at: time::now() }; \
                   IF $fail_after_event { THROW 'STATE_RECOVERY_TEST_FAILURE_AFTER_EVENT'; }; \
                   LET $stored = IF array::len($existing) = 0 { \
                     CREATE $record CONTENT { record_kind: $record_kind, aggregate_id: $aggregate_id, \
                       workspace_id: $workspace_id_filter, wp_id: $wp_id, mt_id: $mt_id, status: $status, \
                       scope_kind: $scope_kind, scope_id: $scope_id, record_json: $record_json, \
                       event_ledger_event_id: type::record('kernel_event_ledger', $event_id), \
                       event_payload_hash: $event_payload_hash, owner_account_id: $owner_account_id, \
                       actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, \
                       access_space_id: $access_space_id } \
                   } ELSE { \
                     UPDATE $record CONTENT { record_kind: $record_kind, aggregate_id: $aggregate_id, \
                       workspace_id: $workspace_id_filter, wp_id: $wp_id, mt_id: $mt_id, status: $status, \
                       scope_kind: $scope_kind, scope_id: $scope_id, record_json: $record_json, \
                       event_ledger_event_id: type::record('kernel_event_ledger', $event_id), \
                       event_payload_hash: $event_payload_hash, owner_account_id: $owner_account_id, \
                       actor_principal_id: $actor_principal_id, authenticated_session_id: $authenticated_session_id, \
                       access_space_id: $access_space_id } \
                     WHERE owner_account_id = $owner_account_id \
                       AND actor_principal_id = $actor_principal_id \
                       AND authenticated_session_id = $authenticated_session_id \
                       AND access_space_id = $access_space_id AND workspace_id = $workspace_id \
                   }; \
                   RETURN SELECT record_json, record::id(event_ledger_event_id) AS event_id, event_payload_hash FROM $record \
                     WHERE owner_account_id = $owner_account_id \
                       AND actor_principal_id = $actor_principal_id \
                       AND authenticated_session_id = $authenticated_session_id \
                       AND access_space_id = $access_space_id AND workspace_id = $workspace_id \
                       AND event_ledger_event_id = type::record('kernel_event_ledger', $event_id) \
                       AND event_ledger_event_id.owner_account_id = $owner_account_id \
                       AND event_ledger_event_id.actor_principal_id = $actor_principal_id \
                       AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id \
                       AND event_ledger_event_id.access_space_id = $access_space_id \
                       AND event_ledger_event_id.workspace_id = $workspace_id \
                       AND event_ledger_event_id.payload_hash = $event_payload_hash; \
                 }; \
                 COMMIT TRANSACTION;",
                bindings,
                5,
            ).await
        })).await?;
        match rows.as_slice() {
            [row] => Ok(AuthorityRow {
                record_json: row.record_json.clone(),
                event_id: row.event_id.clone(),
                event_payload_hash: row.event_payload_hash.clone(),
            }),
            [] => Err(StateRecoveryError::AccessLifecycle(
                "atomic authority write returned no exact-scope row".to_string(),
            )),
            _ => Err(StateRecoveryError::AccessLifecycle(
                "atomic authority write returned ambiguous exact-scope rows".to_string(),
            )),
        }
    }
}

impl ParallelSwarmStateRecoveryStore {
    pub async fn claim_work_surface(
        &self,
        request: WorkClaimRequest,
    ) -> StateRecoveryResult<WorkClaimOutcome> {
        validate_ttl(request.ttl_seconds)?;
        validate_claim_scope(&request.workspace_id, &request.scope)?;
        self.ensure_workspace(&request.workspace_id)?;
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

        let now = Utc::now();
        let claim_id = format!("PSR-CLAIM-{}", Uuid::now_v7());
        let lane = request.lane.scrubbed_for_persistence();
        let (event, stored_event) = Self::build_event(
            KernelEventType::SessionClaimed,
            "parallel_swarm_claim",
            &claim_id,
            &lane,
            &request.session_id,
            json!({
                "schema_id": "hsk.parallel_swarm.claim@1",
                "claim_id": &claim_id,
                "workspace_id": &request.workspace_id,
                "wp_id": &request.wp_id,
                "mt_id": &request.mt_id,
                "scope": &request.scope,
                "lane": &lane,
                "reason": &request.reason,
            }),
        )?;
        let candidate = WorkClaimRecord {
            claim_id: claim_id.clone(),
            workspace_id: request.workspace_id.clone(),
            wp_id: request.wp_id.clone(),
            mt_id: request.mt_id.clone(),
            scope: request.scope.clone(),
            lane,
            session_id: request.session_id,
            status: ClaimStatus::Active,
            reason: request.reason,
            claimed_at_utc: now,
            expires_at_utc: now + chrono::Duration::seconds(request.ttl_seconds),
            released_at_utc: None,
            event_ledger_event_id: Some(stored_event.event_id.clone()),
            release_event_ledger_event_id: None,
            reclaim_event_ledger_event_id: None,
        };
        let persisted = self
            .persist_json(
                "claim",
                &claim_id,
                &candidate.workspace_id,
                Some(&candidate.wp_id),
                candidate.mt_id.as_deref(),
                candidate.status.as_str(),
                Some(&candidate.scope),
                serde_json::to_string(&candidate)?,
                None,
                true,
                event,
                stored_event,
            )
            .await;
        match persisted {
            Ok(row) => {
                let record: WorkClaimRecord = serde_json::from_str(&row.record_json)?;
                Ok(WorkClaimOutcome {
                    status: if record.claim_id == claim_id {
                        ClaimStatus::Active
                    } else {
                        ClaimStatus::Held
                    },
                    claim_id: record.claim_id.clone(),
                    active_holder: if record.claim_id == claim_id {
                        None
                    } else {
                        Some(record.lane)
                    },
                    event_ledger_event_id: record.event_ledger_event_id,
                })
            }
            Err(error) => {
                if let Some(holder) = self.active_claim_for_scope(&request.scope).await? {
                    Ok(WorkClaimOutcome {
                        status: ClaimStatus::Held,
                        claim_id: holder.claim_id,
                        active_holder: Some(holder.lane),
                        event_ledger_event_id: holder.event_ledger_event_id,
                    })
                } else {
                    Err(error)
                }
            }
        }
    }

    pub async fn list_active_claims(
        &self,
        workspace_id: &str,
    ) -> StateRecoveryResult<Vec<WorkClaimRecord>> {
        self.ensure_workspace(workspace_id)?;
        let reclaimer = system_reclaimer_lane()?;
        self.reclaim_expired_work_claims(
            &reclaimer,
            "system-expired-claim-reclaim",
            "opportunistic expired claim sweep",
        )
        .await?;
        let now = Utc::now();
        let mut rows = self.load_rows::<WorkClaimRecord>("claim").await?;
        rows.retain(|row| {
            row.workspace_id == workspace_id
                && row.status == ClaimStatus::Active
                && row.released_at_utc.is_none()
                && row.expires_at_utc > now
        });
        rows.sort_by(|left, right| {
            left.claimed_at_utc
                .cmp(&right.claimed_at_utc)
                .then_with(|| left.claim_id.cmp(&right.claim_id))
        });
        Ok(rows)
    }

    async fn active_claim_for_scope(
        &self,
        scope: &ClaimScope,
    ) -> StateRecoveryResult<Option<WorkClaimRecord>> {
        let now = Utc::now();
        let mut rows = self.load_rows::<WorkClaimRecord>("claim").await?;
        rows.retain(|row| {
            &row.scope == scope
                && row.status == ClaimStatus::Active
                && row.released_at_utc.is_none()
                && row.expires_at_utc > now
        });
        rows.sort_by_key(|row| row.claimed_at_utc);
        Ok(rows.into_iter().next())
    }

    pub async fn release_claim(
        &self,
        claim_id: &str,
        lane: &AgentLaneIdentity,
        reason: &str,
    ) -> StateRecoveryResult<bool> {
        self.ensure_active_access().await?;
        let Some(mut claim) = self.load_one::<WorkClaimRecord>("claim", claim_id).await? else {
            return Ok(false);
        };
        if claim.lane.actor_id != lane.actor_id
            || claim.status != ClaimStatus::Active
            || claim.released_at_utc.is_some()
        {
            return Ok(false);
        }
        let previous_event_id = claim
            .event_ledger_event_id
            .as_deref()
            .ok_or_else(|| StateRecoveryError::Kernel("claim receipt is missing".to_string()))?
            .to_string();
        let persistent_lane = lane.scrubbed_for_persistence();
        let (event, stored_event) = Self::build_event(
            KernelEventType::SessionCompleted,
            "parallel_swarm_claim",
            claim_id,
            &persistent_lane,
            &format!("release-{claim_id}"),
            json!({
                "schema_id": "hsk.parallel_swarm.claim_release@1",
                "claim_id": claim_id,
                "workspace_id": &claim.workspace_id,
                "wp_id": &claim.wp_id,
                "mt_id": &claim.mt_id,
                "scope": &claim.scope,
                "lane": &persistent_lane,
                "status": ClaimStatus::Released,
                "reason": reason,
            }),
        )?;
        claim.status = ClaimStatus::Released;
        claim.reason = reason.to_string();
        claim.released_at_utc = Some(Utc::now());
        claim.release_event_ledger_event_id = Some(stored_event.event_id.clone());
        self.persist_json(
            "claim",
            claim_id,
            &claim.workspace_id,
            Some(&claim.wp_id),
            claim.mt_id.as_deref(),
            claim.status.as_str(),
            Some(&claim.scope),
            serde_json::to_string(&claim)?,
            Some(&previous_event_id),
            false,
            event,
            stored_event,
        )
        .await?;
        Ok(true)
    }

    pub async fn reclaim_expired_work_claims(
        &self,
        lane: &AgentLaneIdentity,
        session_id: &str,
        reason: &str,
    ) -> StateRecoveryResult<Vec<WorkClaimRecord>> {
        require_capability(lane, AgentCapability::ClaimWorktree)?;
        self.ensure_active_access().await?;
        let now = Utc::now();
        let mut reclaimed = Vec::new();
        for mut claim in self.load_rows::<WorkClaimRecord>("claim").await? {
            if claim.status != ClaimStatus::Active
                || claim.released_at_utc.is_some()
                || claim.expires_at_utc > now
            {
                continue;
            }
            let previous_event_id = claim
                .event_ledger_event_id
                .as_deref()
                .ok_or_else(|| StateRecoveryError::Kernel("claim receipt is missing".to_string()))?
                .to_string();
            let reclaimer = lane.scrubbed_for_persistence();
            let (event, stored_event) = Self::build_event(
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
            )?;
            claim.status = ClaimStatus::Reclaimed;
            claim.released_at_utc = Some(now);
            claim.reason = reason.to_string();
            claim.reclaim_event_ledger_event_id = Some(stored_event.event_id.clone());
            self.persist_json(
                "claim",
                &claim.claim_id,
                &claim.workspace_id,
                Some(&claim.wp_id),
                claim.mt_id.as_deref(),
                claim.status.as_str(),
                Some(&claim.scope),
                serde_json::to_string(&claim)?,
                Some(&previous_event_id),
                false,
                event,
                stored_event,
            )
            .await?;
            reclaimed.push(claim);
        }
        Ok(reclaimed)
    }
}

impl ParallelSwarmStateRecoveryStore {
    pub async fn record_quiet_background_work(
        &self,
        request: QuietBackgroundWorkRequest,
    ) -> StateRecoveryResult<QuietBackgroundWorkRecord> {
        require_capability(&request.lane, AgentCapability::RunQuietBackgroundWork)?;
        self.ensure_workspace(&request.workspace_id)?;
        ensure_safe_token("wp_id", &request.wp_id)?;
        ensure_safe_token("mt_id", &request.mt_id)?;
        ensure_safe_token("subject_id", &request.subject_id)?;
        ensure_safe_token("session_id", &request.session_id)?;
        validate_quiet_background_policy(request.work_kind, &request.policy)?;
        ensure_bounded_text("evidence_ref", &request.evidence_ref, 512)?;
        let receipt_id = format!("PSR-QUIET-{}", Uuid::now_v7());
        let lane = request.lane.scrubbed_for_persistence();
        let (event, stored_event) = Self::build_event(
            KernelEventType::KnowledgeQuietBackgroundWorkRecorded,
            "parallel_swarm_quiet_background_work",
            &receipt_id,
            &lane,
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
        let record = QuietBackgroundWorkRecord {
            receipt_id: receipt_id.clone(),
            workspace_id: request.workspace_id,
            wp_id: request.wp_id,
            mt_id: request.mt_id,
            work_kind: request.work_kind,
            subject_id: request.subject_id,
            lane,
            session_id: request.session_id,
            policy: request.policy,
            evidence_ref: request.evidence_ref,
            event_ledger_event_id: stored_event.event_id.clone(),
            created_at_utc: Utc::now(),
        };
        let row = self
            .persist_json(
                "quiet_background_work",
                &receipt_id,
                &record.workspace_id,
                Some(&record.wp_id),
                Some(&record.mt_id),
                record.work_kind.as_str(),
                None,
                serde_json::to_string(&record)?,
                None,
                true,
                event,
                stored_event,
            )
            .await?;
        Ok(serde_json::from_str(&row.record_json)?)
    }

    pub(crate) async fn record_quiet_background_work_tx<T>(
        &self,
        _outer_transaction: &mut T,
        request: QuietBackgroundWorkRequest,
    ) -> StateRecoveryResult<QuietBackgroundWorkRecord> {
        self.record_quiet_background_work(request).await
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
        let resolved = NavigationCommandSet.resolve(command, params)?;
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

    pub async fn record_role_mailbox_handoff(
        &self,
        request: RoleMailboxHandoffRequest,
    ) -> StateRecoveryResult<RoleMailboxHandoffRecord> {
        require_capability(&request.from_lane, AgentCapability::WriteMailbox)?;
        ensure_safe_token("to_role", &request.to_role)?;
        ensure_safe_token("wp_id", &request.wp_id)?;
        ensure_safe_token("mt_id", &request.mt_id)?;
        ensure_sha256(&request.body_sha256)?;
        let workspace_id = if let Some(claim_id) = request.claim_id.as_deref() {
            let claim = self
                .load_one::<WorkClaimRecord>("claim", claim_id)
                .await?
                .ok_or_else(|| {
                    StateRecoveryError::InvalidInput(
                        "mailbox handoff claim_ref is absent from exact scope".to_string(),
                    )
                })?;
            if claim.wp_id != request.wp_id || claim.mt_id.as_deref() != Some(&request.mt_id) {
                return Err(StateRecoveryError::InvalidInput(
                    "mailbox handoff claim_ref does not match WP/MT".to_string(),
                ));
            }
            claim.workspace_id
        } else {
            self.scope.workspace_id.as_str().to_string()
        };
        let handoff_id = format!("PSR-HANDOFF-{}", Uuid::now_v7());
        let lane = request.from_lane.scrubbed_for_persistence();
        let (event, stored_event) = Self::build_event(
            KernelEventType::HbrHandoffGate,
            "parallel_swarm_handoff",
            &handoff_id,
            &lane,
            &format!("handoff-{handoff_id}"),
            json!({
                "schema_id": "hsk.parallel_swarm.mailbox_handoff@1",
                "handoff_id": &handoff_id,
                "workspace_id": &workspace_id,
                "wp_id": &request.wp_id,
                "mt_id": &request.mt_id,
                "claim_id": &request.claim_id,
                "to_role": &request.to_role,
                "mailbox_thread_id": &request.mailbox_thread_id,
                "mailbox_message_id": &request.mailbox_message_id,
                "status": request.status,
                "summary": &request.summary,
                "body_sha256": &request.body_sha256,
            }),
        )?;
        let record = RoleMailboxHandoffRecord {
            handoff_id: handoff_id.clone(),
            wp_id: request.wp_id,
            mt_id: request.mt_id,
            claim_id: request.claim_id,
            from_lane: lane,
            to_role: request.to_role,
            mailbox_thread_id: request.mailbox_thread_id,
            mailbox_message_id: request.mailbox_message_id,
            status: request.status,
            summary: request.summary,
            body_sha256: request.body_sha256,
            event_ledger_event_id: stored_event.event_id.clone(),
            created_at_utc: Utc::now(),
        };
        let row = self
            .persist_json(
                "mailbox_handoff",
                &handoff_id,
                &workspace_id,
                Some(&record.wp_id),
                Some(&record.mt_id),
                record.status.as_str(),
                None,
                serde_json::to_string(&record)?,
                None,
                true,
                event,
                stored_event,
            )
            .await?;
        Ok(serde_json::from_str(&row.record_json)?)
    }
}

impl ParallelSwarmStateRecoveryStore {
    pub async fn record_checkpoint(
        &self,
        request: RecoveryCheckpointRequest,
    ) -> StateRecoveryResult<RecoveryCheckpointRecord> {
        require_capability(&request.lane, AgentCapability::RecordCheckpoint)?;
        self.ensure_workspace(&request.workspace_id)?;
        ensure_safe_token("session_id", &request.session_id)?;
        ensure_safe_token("wp_id", &request.wp_id)?;
        ensure_safe_token("mt_id", &request.mt_id)?;
        ensure_bounded_text("next_step_context", &request.next_step_context, 20_000)?;
        ensure_bounded_text("compaction_reason", &request.compaction_reason, 512)?;
        ensure_bounded_text("git_head", &request.git_head, 256)?;
        if let Some(claim_id) = request.claim_id.as_deref() {
            let claim = self
                .load_one::<WorkClaimRecord>("claim", claim_id)
                .await?
                .ok_or_else(|| {
                    StateRecoveryError::InvalidInput(
                        "checkpoint claim_ref is absent from exact scope".to_string(),
                    )
                })?;
            if claim.workspace_id != request.workspace_id || claim.wp_id != request.wp_id {
                return Err(StateRecoveryError::InvalidInput(
                    "checkpoint claim_ref does not match workspace/WP".to_string(),
                ));
            }
        }
        if let Some(handoff_id) = request.mailbox_handoff_id.as_deref() {
            let handoff = self
                .load_one::<RoleMailboxHandoffRecord>("mailbox_handoff", handoff_id)
                .await?
                .ok_or_else(|| {
                    StateRecoveryError::InvalidInput(
                        "checkpoint handoff_ref is absent from exact scope".to_string(),
                    )
                })?;
            if handoff.wp_id != request.wp_id || handoff.mt_id != request.mt_id {
                return Err(StateRecoveryError::InvalidInput(
                    "checkpoint handoff_ref does not match WP/MT".to_string(),
                ));
            }
        }
        let checkpoint_id = format!("PSR-CHKPT-{}", Uuid::now_v7());
        let lane = request.lane.scrubbed_for_persistence();
        let payload_sha256 = sha256_hex(canonical_json(&request.payload).as_bytes());
        let (event, stored_event) = Self::build_event(
            KernelEventType::SessionCheckpointed,
            "parallel_swarm_checkpoint",
            &checkpoint_id,
            &lane,
            &request.session_id,
            json!({
                "schema_id": "hsk.parallel_swarm.checkpoint@1",
                "checkpoint_id": &checkpoint_id,
                "workspace_id": &request.workspace_id,
                "wp_id": &request.wp_id,
                "mt_id": &request.mt_id,
                "claim_id": &request.claim_id,
                "mailbox_handoff_id": &request.mailbox_handoff_id,
                "navigation_command_id": &request.navigation_command_id,
                "resume_pointer": &request.resume_pointer,
                "payload_sha256": &payload_sha256,
                "git_head": &request.git_head,
            }),
        )?;
        let record = RecoveryCheckpointRecord {
            checkpoint_id: checkpoint_id.clone(),
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
            event_ledger_event_id: stored_event.event_id.clone(),
            created_at_utc: Utc::now(),
        };
        let row = self
            .persist_json(
                "checkpoint",
                &checkpoint_id,
                &record.workspace_id,
                Some(&record.wp_id),
                Some(&record.mt_id),
                "checkpointed",
                None,
                serde_json::to_string(&record)?,
                None,
                true,
                event,
                stored_event,
            )
            .await?;
        Ok(serde_json::from_str(&row.record_json)?)
    }

    pub async fn recover_from_checkpoint(
        &self,
        checkpoint_id: &str,
        new_lane: AgentLaneIdentity,
        new_session_id: &str,
    ) -> StateRecoveryResult<RecoveredCheckpoint> {
        require_capability(&new_lane, AgentCapability::RecordCheckpoint)?;
        ensure_safe_token("new_session_id", &new_session_id)?;
        let checkpoint = self
            .load_one::<RecoveryCheckpointRecord>("checkpoint", checkpoint_id)
            .await?
            .ok_or_else(|| StateRecoveryError::CheckpointNotFound(checkpoint_id.to_string()))?;
        let found = sha256_hex(canonical_json(&checkpoint.payload).as_bytes());
        if found != checkpoint.payload_sha256 {
            return Err(StateRecoveryError::PayloadHashMismatch {
                checkpoint_id: checkpoint_id.to_string(),
                expected: checkpoint.payload_sha256.clone(),
                found,
            });
        }
        let recovery_key = format!("{checkpoint_id}:{new_session_id}");
        if let Some(receipt) = self
            .load_one::<RecoveryReceiptRecord>("recovery_receipt", &recovery_key)
            .await?
        {
            return Ok(RecoveredCheckpoint {
                resume_pointer: receipt.resume_pointer.clone(),
                checkpoint,
                receipt,
            });
        }
        let receipt_id = format!("PSR-RECOVER-{}", Uuid::now_v7());
        let lane = new_lane.scrubbed_for_persistence();
        let (event, stored_event) = Self::build_event(
            KernelEventType::KnowledgeCrdtRecoveryReceiptRecorded,
            "parallel_swarm_recovery",
            &receipt_id,
            &lane,
            new_session_id,
            json!({
                "schema_id": "hsk.parallel_swarm.recovery@1",
                "receipt_id": &receipt_id,
                "checkpoint_id": checkpoint_id,
                "prior_session_id": &checkpoint.session_id,
                "new_session_id": &new_session_id,
                "new_lane": &lane,
                "resume_pointer": &checkpoint.resume_pointer,
                "checkpoint_payload_sha256": &checkpoint.payload_sha256,
            }),
        )?;
        let receipt = RecoveryReceiptRecord {
            receipt_id,
            checkpoint_id: checkpoint_id.to_string(),
            prior_session_id: checkpoint.session_id.clone(),
            new_session_id: new_session_id.to_string(),
            new_lane: lane,
            resume_pointer: checkpoint.resume_pointer.clone(),
            event_ledger_event_id: stored_event.event_id.clone(),
            recovered_at_utc: Utc::now(),
        };
        let row = self
            .persist_json(
                "recovery_receipt",
                &recovery_key,
                &checkpoint.workspace_id,
                Some(&checkpoint.wp_id),
                Some(&checkpoint.mt_id),
                "recovered",
                None,
                serde_json::to_string(&receipt)?,
                None,
                true,
                event,
                stored_event,
            )
            .await?;
        let receipt: RecoveryReceiptRecord = serde_json::from_str(&row.record_json)?;
        Ok(RecoveredCheckpoint {
            resume_pointer: receipt.resume_pointer.clone(),
            checkpoint,
            receipt,
        })
    }

    pub async fn build_handoff_compression_template(
        &self,
        request: HandoffCompressionRequest,
    ) -> StateRecoveryResult<HandoffCompressionTemplateV1> {
        require_capability(&request.requested_by_lane, AgentCapability::NavigateBackend)?;
        let max_chars = bounded_handoff_body_chars(request.max_chars)?;
        let checkpoint = self
            .load_one::<RecoveryCheckpointRecord>("checkpoint", &request.checkpoint_id)
            .await?
            .ok_or_else(|| StateRecoveryError::CheckpointNotFound(request.checkpoint_id.clone()))?;
        let found = sha256_hex(canonical_json(&checkpoint.payload).as_bytes());
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
            body_sha256: sha256_hex(body.as_bytes()),
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
}

impl ParallelSwarmStateRecoveryStore {
    pub async fn enqueue_indexing_lease(
        &self,
        request: IndexingLeaseRequest,
    ) -> StateRecoveryResult<IndexingLeaseRecord> {
        validate_ttl(request.ttl_seconds)?;
        validate_quiet_background_policy(QuietBackgroundWorkKind::Indexing, &request.quiet_policy)?;
        require_capability(&request.lane, AgentCapability::WriteLocalIndex)?;
        self.ensure_workspace(&request.workspace_id)?;
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
            .insert_indexing_lease(&request, status, blocked_by)
            .await
        {
            Ok(record) => Ok(record),
            Err(error) if status == IndexLeaseStatus::Acquired => {
                if let Some(active) = self.active_index_writer_for_scope(&request.scope).await? {
                    self.insert_indexing_lease(
                        &request,
                        IndexLeaseStatus::Queued,
                        Some(active.lease_id),
                    )
                    .await
                } else {
                    Err(error)
                }
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
        self.ensure_workspace(&request.workspace_id)?;
        self.reclaim_orphaned_indexing_leases().await?;
        if self
            .active_index_writer_for_scope(&request.scope)
            .await?
            .is_some()
            || self
                .queued_index_writer_for_scope(&request.scope)
                .await?
                .is_some()
        {
            return Ok(None);
        }
        match self
            .insert_indexing_lease(&request, IndexLeaseStatus::Acquired, None)
            .await
        {
            Ok(record) => Ok(Some(record)),
            Err(_)
                if self
                    .active_index_writer_for_scope(&request.scope)
                    .await?
                    .is_some() =>
            {
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }

    pub(crate) async fn try_acquire_indexing_lease_tx<T>(
        &self,
        _outer_transaction: &mut T,
        request: &IndexingLeaseRequest,
    ) -> StateRecoveryResult<Option<IndexingLeaseRecord>> {
        self.try_acquire_indexing_lease(request.clone()).await
    }

    async fn insert_indexing_lease(
        &self,
        request: &IndexingLeaseRequest,
        status: IndexLeaseStatus,
        blocked_by_lease_id: Option<String>,
    ) -> StateRecoveryResult<IndexingLeaseRecord> {
        let now = Utc::now();
        let lease_id = format!("PSR-IDXLEASE-{}", Uuid::now_v7());
        let lane = request.lane.scrubbed_for_persistence();
        let event_type = if status == IndexLeaseStatus::Queued {
            KernelEventType::SessionQueued
        } else {
            KernelEventType::KnowledgeIndexRunStarted
        };
        let (event, stored_event) = Self::build_event(
            event_type,
            "parallel_indexing_lease",
            &lease_id,
            &lane,
            &request.session_id,
            json!({
                "schema_id": "hsk.parallel_swarm.indexing_lease@1",
                "lease_id": &lease_id,
                "workspace_id": &request.workspace_id,
                "wp_id": &request.wp_id,
                "mt_id": &request.mt_id,
                "scope": &request.scope,
                "index_run_id": &request.index_run_id,
                "status": status,
                "blocked_by_lease_id": &blocked_by_lease_id,
                "quiet_policy": &request.quiet_policy,
            }),
        )?;
        let record = IndexingLeaseRecord {
            lease_id: lease_id.clone(),
            workspace_id: request.workspace_id.clone(),
            wp_id: request.wp_id.clone(),
            mt_id: request.mt_id.clone(),
            scope: request.scope.clone(),
            lane,
            session_id: request.session_id.clone(),
            index_run_id: request.index_run_id.clone(),
            priority: request.priority,
            ttl_seconds: request.ttl_seconds,
            status,
            blocked_by_lease_id,
            quiet_policy: request.quiet_policy.clone(),
            event_ledger_event_id: stored_event.event_id.clone(),
        };
        let _lease_times = if status == IndexLeaseStatus::Acquired {
            Some((now, now + chrono::Duration::seconds(request.ttl_seconds)))
        } else {
            None
        };
        let row = self
            .persist_json(
                "indexing_lease",
                &lease_id,
                &record.workspace_id,
                Some(&record.wp_id),
                Some(&record.mt_id),
                record.status.as_str(),
                Some(&record.scope),
                serde_json::to_string(&record)?,
                None,
                true,
                event,
                stored_event,
            )
            .await?;
        Ok(serde_json::from_str(&row.record_json)?)
    }

    pub async fn active_index_writer_for_scope(
        &self,
        scope: &ClaimScope,
    ) -> StateRecoveryResult<Option<IndexingLeaseRecord>> {
        let now = Utc::now();
        let mut rows = self
            .load_rows::<IndexingLeaseRecord>("indexing_lease")
            .await?;
        rows.retain(|row| {
            &row.scope == scope
                && row.status == IndexLeaseStatus::Acquired
                && lease_expiry(row) > now
        });
        rows.sort_by(|left, right| left.lease_id.cmp(&right.lease_id));
        Ok(rows.into_iter().next())
    }

    async fn queued_index_writer_for_scope(
        &self,
        scope: &ClaimScope,
    ) -> StateRecoveryResult<Option<IndexingLeaseRecord>> {
        let mut rows = self
            .load_rows::<IndexingLeaseRecord>("indexing_lease")
            .await?;
        rows.retain(|row| &row.scope == scope && row.status == IndexLeaseStatus::Queued);
        rows.sort_by(|left, right| {
            right
                .priority
                .cmp(&left.priority)
                .then_with(|| left.lease_id.cmp(&right.lease_id))
        });
        Ok(rows.into_iter().next())
    }

    pub async fn complete_indexing_lease(
        &self,
        lease_id: &str,
        lane: &AgentLaneIdentity,
    ) -> StateRecoveryResult<bool> {
        let Some(mut lease) = self
            .load_one::<IndexingLeaseRecord>("indexing_lease", lease_id)
            .await?
        else {
            return Ok(false);
        };
        if lease.lane.actor_id != lane.actor_id || lease.status != IndexLeaseStatus::Acquired {
            return Ok(false);
        }
        let previous_event_id = lease.event_ledger_event_id.clone();
        let (event, stored_event) = Self::build_event(
            KernelEventType::KnowledgeIndexRunCompleted,
            "parallel_indexing_lease",
            lease_id,
            lane,
            &lease.session_id,
            json!({
                "schema_id": "hsk.parallel_swarm.indexing_lease@1",
                "lease_id": lease_id,
                "workspace_id": &lease.workspace_id,
                "scope": &lease.scope,
                "index_run_id": &lease.index_run_id,
                "status": IndexLeaseStatus::Completed,
            }),
        )?;
        lease.status = IndexLeaseStatus::Completed;
        lease.event_ledger_event_id = stored_event.event_id.clone();
        self.persist_json(
            "indexing_lease",
            lease_id,
            &lease.workspace_id,
            Some(&lease.wp_id),
            Some(&lease.mt_id),
            lease.status.as_str(),
            Some(&lease.scope),
            serde_json::to_string(&lease)?,
            Some(&previous_event_id),
            false,
            event,
            stored_event,
        )
        .await?;
        Ok(true)
    }

    pub async fn acquire_next_indexing_lease(
        &self,
        scope: &ClaimScope,
    ) -> StateRecoveryResult<Option<IndexingLeaseRecord>> {
        if self.active_index_writer_for_scope(scope).await?.is_some() {
            return Ok(None);
        }
        let Some(mut lease) = self.queued_index_writer_for_scope(scope).await? else {
            return Ok(None);
        };
        let previous_event_id = lease.event_ledger_event_id.clone();
        let (event, stored_event) = Self::build_event(
            KernelEventType::KnowledgeIndexRunStarted,
            "parallel_indexing_lease",
            &lease.lease_id,
            &lease.lane,
            &lease.session_id,
            json!({
                "schema_id": "hsk.parallel_swarm.indexing_lease@1",
                "lease_id": &lease.lease_id,
                "workspace_id": &lease.workspace_id,
                "scope": &lease.scope,
                "index_run_id": &lease.index_run_id,
                "status": IndexLeaseStatus::Acquired,
            }),
        )?;
        lease.status = IndexLeaseStatus::Acquired;
        lease.blocked_by_lease_id = None;
        lease.event_ledger_event_id = stored_event.event_id.clone();
        let row = self
            .persist_json(
                "indexing_lease",
                &lease.lease_id,
                &lease.workspace_id,
                Some(&lease.wp_id),
                Some(&lease.mt_id),
                lease.status.as_str(),
                Some(&lease.scope),
                serde_json::to_string(&lease)?,
                Some(&previous_event_id),
                false,
                event,
                stored_event,
            )
            .await?;
        Ok(Some(serde_json::from_str(&row.record_json)?))
    }

    pub async fn reclaim_orphaned_indexing_leases(
        &self,
    ) -> StateRecoveryResult<Vec<IndexingLeaseRecord>> {
        let now = Utc::now();
        let mut reclaimed = Vec::new();
        for mut lease in self
            .load_rows::<IndexingLeaseRecord>("indexing_lease")
            .await?
        {
            if lease.status != IndexLeaseStatus::Acquired || lease_expiry(&lease) > now {
                continue;
            }
            let previous_event_id = lease.event_ledger_event_id.clone();
            let (event, stored_event) = Self::build_event(
                KernelEventType::KnowledgeIndexRunCancelled,
                "parallel_indexing_lease",
                &lease.lease_id,
                &lease.lane,
                &lease.session_id,
                json!({
                    "schema_id": "hsk.parallel_swarm.indexing_lease@1",
                    "lease_id": &lease.lease_id,
                    "workspace_id": &lease.workspace_id,
                    "scope": &lease.scope,
                    "index_run_id": &lease.index_run_id,
                    "status": IndexLeaseStatus::Reclaimed,
                }),
            )?;
            lease.status = IndexLeaseStatus::Reclaimed;
            lease.event_ledger_event_id = stored_event.event_id.clone();
            self.persist_json(
                "indexing_lease",
                &lease.lease_id,
                &lease.workspace_id,
                Some(&lease.wp_id),
                Some(&lease.mt_id),
                lease.status.as_str(),
                Some(&lease.scope),
                serde_json::to_string(&lease)?,
                Some(&previous_event_id),
                false,
                event,
                stored_event,
            )
            .await?;
            reclaimed.push(lease);
        }
        Ok(reclaimed)
    }
}

fn lease_expiry(lease: &IndexingLeaseRecord) -> DateTime<Utc> {
    let id = lease
        .lease_id
        .strip_prefix("PSR-IDXLEASE-")
        .and_then(|value| Uuid::parse_str(value).ok());
    let started = id
        .and_then(|value| value.get_timestamp())
        .and_then(|timestamp| {
            let (seconds, nanos) = timestamp.to_unix();
            DateTime::<Utc>::from_timestamp(seconds as i64, nanos)
        })
        .unwrap_or_else(Utc::now);
    started + chrono::Duration::seconds(lease.ttl_seconds)
}

impl ParallelSwarmStateRecoveryStore {
    pub async fn record_cloud_fallback_basis(
        &self,
        request: CloudFallbackBasisRequest,
    ) -> StateRecoveryResult<CloudFallbackBasisReceiptV1> {
        require_capability(&request.lane, AgentCapability::NavigateBackend)?;
        if !matches!(
            request.lane.lane_kind,
            AgentLaneKind::Local | AgentLaneKind::System
        ) {
            return Err(StateRecoveryError::InvalidInput(
                "cloud fallback basis requires a local or system lane".to_string(),
            ));
        }
        self.ensure_workspace(&request.workspace_id)?;
        ensure_safe_token("wp_id", &request.wp_id)?;
        ensure_safe_token("mt_id", &request.mt_id)?;
        ensure_safe_token("claim_id", &request.claim_id)?;
        ensure_safe_token("parent_session_id", &request.parent_session_id)?;
        ensure_safe_token("session_id", &request.session_id)?;
        ensure_sha256(&request.prompt_sha256)?;
        ensure_sha256(&request.evidence_sha256)?;
        ensure_bounded_text("local_attempt_ref", &request.local_attempt_ref, 512)?;
        ensure_bounded_text("summary", &request.summary, 512)?;
        let claim = self
            .load_one::<WorkClaimRecord>("claim", &request.claim_id)
            .await?
            .ok_or_else(|| {
                StateRecoveryError::InvalidInput(
                    "cloud fallback basis claim is absent from exact scope".to_string(),
                )
            })?;
        if claim.workspace_id != request.workspace_id
            || claim.wp_id != request.wp_id
            || claim.mt_id.as_deref() != Some(&request.mt_id)
            || claim.status != ClaimStatus::Active
        {
            return Err(StateRecoveryError::InvalidInput(
                "cloud fallback basis claim is inactive or mismatched".to_string(),
            ));
        }
        let basis_id = format!("PSR-FALLBACK-{}", Uuid::now_v7());
        let lane = request.lane.scrubbed_for_persistence();
        let (event, stored_event) = Self::build_event(
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
        let receipt = CloudFallbackBasisReceiptV1 {
            schema_id: PARALLEL_SWARM_CLOUD_FALLBACK_BASIS_SCHEMA_ID.to_string(),
            basis_id: basis_id.clone(),
            fallback_basis_event_id: stored_event.event_id.clone(),
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
        };
        let row = self
            .persist_json(
                "cloud_fallback_basis",
                &basis_id,
                &receipt.workspace_id,
                Some(&receipt.wp_id),
                Some(&receipt.mt_id),
                "recorded",
                None,
                serde_json::to_string(&receipt)?,
                None,
                true,
                event,
                stored_event,
            )
            .await?;
        Ok(serde_json::from_str(&row.record_json)?)
    }

    pub async fn record_cloud_assistance_output(
        &self,
        request: CloudAssistanceRequest,
    ) -> StateRecoveryResult<CloudAssistanceReceiptV1> {
        require_capability(&request.from_lane, AgentCapability::WriteMailbox)?;
        ensure_cloud_assistance_lane(&request.from_lane)?;
        self.ensure_workspace(&request.workspace_id)?;
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
        let claim = self
            .load_one::<WorkClaimRecord>("claim", &request.claim_id)
            .await?
            .ok_or_else(|| {
                StateRecoveryError::InvalidInput(
                    "cloud assistance claim is absent from exact scope".to_string(),
                )
            })?;
        if claim.status != ClaimStatus::Active
            || claim.workspace_id != request.workspace_id
            || claim.wp_id != request.wp_id
            || claim.mt_id.as_deref() != Some(&request.mt_id)
            || claim.lane.actor_id != request.from_lane.actor_id
            || claim.lane.lane_kind != AgentLaneKind::Cloud
        {
            return Err(StateRecoveryError::InvalidInput(
                "cloud assistance claim is inactive, mismatched, or not cloud-owned".to_string(),
            ));
        }
        let basis = self
            .load_rows::<CloudFallbackBasisReceiptV1>("cloud_fallback_basis")
            .await?
            .into_iter()
            .find(|basis| basis.fallback_basis_event_id == request.fallback_basis_event_id)
            .ok_or_else(|| {
                StateRecoveryError::InvalidInput(
                    "cloud fallback basis is absent from exact scope".to_string(),
                )
            })?;
        if basis.claim_id != request.claim_id
            || basis.workspace_id != request.workspace_id
            || basis.wp_id != request.wp_id
            || basis.mt_id != request.mt_id
            || basis.parent_session_id != request.parent_session_id
            || basis.prompt_sha256 != request.prompt_sha256
            || basis.fallback_reason != request.fallback_reason
        {
            return Err(StateRecoveryError::InvalidInput(
                "cloud fallback basis does not match the assistance request".to_string(),
            ));
        }
        let handoff = self
            .record_role_mailbox_handoff(RoleMailboxHandoffRequest {
                from_lane: request.from_lane.clone(),
                to_role: request.to_role.clone(),
                wp_id: request.wp_id.clone(),
                mt_id: request.mt_id.clone(),
                claim_id: Some(request.claim_id.clone()),
                mailbox_thread_id: request.mailbox_thread_id.clone(),
                mailbox_message_id: request.mailbox_message_id.clone(),
                status: SwarmReceiptStatus::Progress,
                summary: request.summary.clone(),
                body_sha256: request.body_sha256.clone(),
            })
            .await?;
        let receipt_id = format!("PSR-CLOUD-{}", Uuid::now_v7());
        let lane = request.from_lane.scrubbed_for_persistence();
        let (event, stored_event) = Self::build_event(
            KernelEventType::HbrHandoffGate,
            "parallel_swarm_cloud_assistance",
            &receipt_id,
            &lane,
            &request.session_id,
            json!({
                "schema_id": PARALLEL_SWARM_CLOUD_ASSISTANCE_SCHEMA_ID,
                "receipt_id": &receipt_id,
                "workspace_id": &request.workspace_id,
                "wp_id": &request.wp_id,
                "mt_id": &request.mt_id,
                "claim_id": &request.claim_id,
                "handoff_id": &handoff.handoff_id,
                "fallback_basis_event_id": &request.fallback_basis_event_id,
                "parent_session_id": &request.parent_session_id,
                "prompt_sha256": &request.prompt_sha256,
                "fallback_reason": request.fallback_reason,
                "output_kind": request.output_kind,
                "output_sha256": &request.output_sha256,
                "target_ref": &request.target_ref,
                "non_authoritative": true,
                "requires_promotion": true,
            }),
        )?;
        let receipt = CloudAssistanceReceiptV1 {
            schema_id: PARALLEL_SWARM_CLOUD_ASSISTANCE_SCHEMA_ID.to_string(),
            receipt_id: receipt_id.clone(),
            workspace_id: request.workspace_id,
            wp_id: request.wp_id,
            mt_id: request.mt_id,
            claim_id: request.claim_id,
            handoff_id: handoff.handoff_id,
            handoff_event_ledger_event_id: handoff.event_ledger_event_id,
            cloud_assistance_event_id: stored_event.event_id.clone(),
            fallback_basis_event_id: request.fallback_basis_event_id,
            parent_session_id: request.parent_session_id,
            prompt_sha256: request.prompt_sha256,
            lane_id: lane.lane_id,
            actor_id: lane.actor_id,
            provider: lane.attribution.provider,
            model_label: lane.attribution.model_label,
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
        let row = self
            .persist_json(
                "cloud_assistance",
                &receipt_id,
                &receipt.workspace_id,
                Some(&receipt.wp_id),
                Some(&receipt.mt_id),
                &receipt.review_state,
                None,
                serde_json::to_string(&receipt)?,
                None,
                true,
                event,
                stored_event,
            )
            .await?;
        Ok(serde_json::from_str(&row.record_json)?)
    }
}

impl ParallelSwarmStateRecoveryStore {
    pub async fn inspect_swarm_evidence(
        &self,
        request: SwarmEvidenceInspectionRequest,
    ) -> StateRecoveryResult<SwarmEvidenceInspectionSnapshot> {
        require_capability(&request.lane, AgentCapability::InspectEvidence)?;
        self.ensure_workspace(&request.workspace_id)?;
        let limit = bounded_inspection_limit(request.limit)? as usize;
        let mut claims = self.load_rows::<WorkClaimRecord>("claim").await?;
        let claim_ids = claims
            .iter()
            .map(|row| row.claim_id.clone())
            .collect::<BTreeSet<_>>();
        let mut mailbox_handoffs = self
            .load_rows::<RoleMailboxHandoffRecord>("mailbox_handoff")
            .await?;
        mailbox_handoffs.retain(|row| {
            row.claim_id
                .as_ref()
                .is_some_and(|claim_id| claim_ids.contains(claim_id))
        });
        let mut checkpoints = self
            .load_rows::<RecoveryCheckpointRecord>("checkpoint")
            .await?;
        let checkpoint_ids = checkpoints
            .iter()
            .map(|row| row.checkpoint_id.clone())
            .collect::<BTreeSet<_>>();
        let mut recovery_receipts = self
            .load_rows::<RecoveryReceiptRecord>("recovery_receipt")
            .await?;
        recovery_receipts.retain(|row| checkpoint_ids.contains(&row.checkpoint_id));
        let mut indexing_leases = self
            .load_rows::<IndexingLeaseRecord>("indexing_lease")
            .await?;
        let mut quiet_background_work = self
            .load_rows::<QuietBackgroundWorkRecord>("quiet_background_work")
            .await?;
        claims.sort_by(|left, right| right.claimed_at_utc.cmp(&left.claimed_at_utc));
        mailbox_handoffs.sort_by(|left, right| right.created_at_utc.cmp(&left.created_at_utc));
        checkpoints.sort_by(|left, right| right.created_at_utc.cmp(&left.created_at_utc));
        recovery_receipts.sort_by(|left, right| right.recovered_at_utc.cmp(&left.recovered_at_utc));
        indexing_leases.sort_by(|left, right| right.lease_id.cmp(&left.lease_id));
        quiet_background_work.sort_by(|left, right| right.created_at_utc.cmp(&left.created_at_utc));
        claims.truncate(limit);
        mailbox_handoffs.truncate(limit);
        checkpoints.truncate(limit);
        recovery_receipts.truncate(limit);
        indexing_leases.truncate(limit);
        quiet_background_work.truncate(limit);
        Ok(SwarmEvidenceInspectionSnapshot {
            workspace_id: request.workspace_id,
            claims,
            mailbox_handoffs,
            checkpoints,
            recovery_receipts,
            indexing_leases,
            quiet_background_work,
        })
    }

    pub async fn project_swarm_dashboard(
        &self,
        request: SwarmDashboardProjectionRequest,
    ) -> StateRecoveryResult<ParallelSwarmDashboardProjectionV1> {
        require_capability(&request.lane, AgentCapability::InspectEvidence)?;
        self.ensure_workspace(&request.workspace_id)?;
        if let Some(wp_id) = request.wp_id.as_deref() {
            ensure_safe_token("wp_id", wp_id)?;
        }
        if let Some(mt_id) = request.mt_id.as_deref() {
            ensure_safe_token("mt_id", mt_id)?;
        }
        let limit = bounded_inspection_limit(request.limit)?;
        let generated_at_utc = Utc::now();
        let mut claims = self.load_rows::<WorkClaimRecord>("claim").await?;
        let mut handoffs = self
            .load_rows::<RoleMailboxHandoffRecord>("mailbox_handoff")
            .await?;
        let mut checkpoints = self
            .load_rows::<RecoveryCheckpointRecord>("checkpoint")
            .await?;
        let mut recoveries = self
            .load_rows::<RecoveryReceiptRecord>("recovery_receipt")
            .await?;
        let mut leases = self
            .load_rows::<IndexingLeaseRecord>("indexing_lease")
            .await?;
        let mut quiet = self
            .load_rows::<QuietBackgroundWorkRecord>("quiet_background_work")
            .await?;
        let matches = |wp: &str, mt: Option<&str>| {
            request.wp_id.as_deref().map_or(true, |wanted| wanted == wp)
                && request
                    .mt_id
                    .as_deref()
                    .map_or(true, |wanted| mt == Some(wanted))
        };
        claims.retain(|row| matches(&row.wp_id, row.mt_id.as_deref()));
        handoffs.retain(|row| matches(&row.wp_id, Some(&row.mt_id)));
        checkpoints.retain(|row| matches(&row.wp_id, Some(&row.mt_id)));
        let checkpoint_map = checkpoints
            .iter()
            .map(|row| {
                (
                    row.checkpoint_id.clone(),
                    (row.wp_id.clone(), row.mt_id.clone()),
                )
            })
            .collect::<BTreeMap<_, _>>();
        recoveries.retain(|row| {
            checkpoint_map
                .get(&row.checkpoint_id)
                .is_some_and(|(wp, mt)| matches(wp, Some(mt)))
        });
        leases.retain(|row| matches(&row.wp_id, Some(&row.mt_id)));
        quiet.retain(|row| matches(&row.wp_id, Some(&row.mt_id)));
        let totals = self.dashboard_totals_from_rows(
            &claims,
            &handoffs,
            &checkpoints,
            &recoveries,
            &leases,
            &quiet,
        );
        claims.sort_by(|left, right| right.claimed_at_utc.cmp(&left.claimed_at_utc));
        handoffs.sort_by(|left, right| right.created_at_utc.cmp(&left.created_at_utc));
        checkpoints.sort_by(|left, right| right.created_at_utc.cmp(&left.created_at_utc));
        recoveries.sort_by(|left, right| right.recovered_at_utc.cmp(&left.recovered_at_utc));
        leases.sort_by(|left, right| right.lease_id.cmp(&left.lease_id));
        quiet.sort_by(|left, right| right.created_at_utc.cmp(&left.created_at_utc));
        let cap = limit as usize;
        claims.truncate(cap);
        handoffs.truncate(cap);
        checkpoints.truncate(cap);
        recoveries.truncate(cap);
        leases.truncate(cap);
        quiet.truncate(cap);
        let mut event_ids = BTreeSet::new();
        collect_projection_event_ids(
            &claims,
            &handoffs,
            &checkpoints,
            &recoveries,
            &leases,
            &quiet,
            &mut event_ids,
        );
        let source_watermark = self
            .dashboard_event_watermark(event_ids.into_iter().collect())
            .await?;
        let mut warnings = Vec::new();
        for missing in &source_watermark.missing_event_refs {
            warnings.push(SwarmDashboardWarningV1 {
                code: "missing_event_ledger_ref".to_string(),
                detail: format!("projection source referenced missing EventLedger row {missing}"),
            });
        }
        let claim_rows = claims
            .iter()
            .map(|row| dashboard_claim_row(row, generated_at_utc))
            .collect::<Vec<_>>();
        let handoff_rows = handoffs
            .iter()
            .map(dashboard_handoff_row)
            .collect::<Vec<_>>();
        let checkpoint_rows = checkpoints
            .iter()
            .map(dashboard_checkpoint_row)
            .collect::<Vec<_>>();
        let recovery_rows = recoveries
            .iter()
            .map(dashboard_recovery_receipt_row)
            .collect::<Vec<_>>();
        let lease_rows = leases
            .iter()
            .map(dashboard_indexing_lease_row)
            .collect::<Vec<_>>();
        let quiet_rows = quiet
            .iter()
            .map(dashboard_quiet_work_row)
            .collect::<Vec<_>>();
        add_truncation_warning(&mut warnings, "claims", claim_rows.len(), totals.claims);
        add_truncation_warning(
            &mut warnings,
            "mailbox_handoffs",
            handoff_rows.len(),
            totals.mailbox_handoffs,
        );
        add_truncation_warning(
            &mut warnings,
            "recovery_checkpoints",
            checkpoint_rows.len(),
            totals.recovery_checkpoints,
        );
        add_truncation_warning(
            &mut warnings,
            "recovery_receipts",
            recovery_rows.len(),
            totals.recovery_receipts,
        );
        add_truncation_warning(
            &mut warnings,
            "indexing_leases",
            lease_rows.len(),
            totals.indexing_leases,
        );
        add_truncation_warning(
            &mut warnings,
            "quiet_background_work",
            quiet_rows.len(),
            totals.quiet_background_work,
        );
        let lanes = dashboard_lane_rows(
            &claims,
            &handoffs,
            &checkpoints,
            &recoveries,
            &leases,
            &quiet,
        );
        let mut totals = dashboard_totals(totals);
        totals.events = source_watermark.event_count;
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

    fn dashboard_totals_from_rows(
        &self,
        claims: &[WorkClaimRecord],
        handoffs: &[RoleMailboxHandoffRecord],
        checkpoints: &[RecoveryCheckpointRecord],
        recoveries: &[RecoveryReceiptRecord],
        leases: &[IndexingLeaseRecord],
        quiet: &[QuietBackgroundWorkRecord],
    ) -> SwarmDashboardAuthorityTotals {
        let now = Utc::now();
        let mut totals = SwarmDashboardAuthorityTotals {
            claims: claims.len() as i64,
            active_claims: claims
                .iter()
                .filter(|row| row.status == ClaimStatus::Active)
                .count() as i64,
            stale_active_claims: claims
                .iter()
                .filter(|row| row.status == ClaimStatus::Active && row.expires_at_utc <= now)
                .count() as i64,
            mailbox_handoffs: handoffs.len() as i64,
            recovery_checkpoints: checkpoints.len() as i64,
            recovery_receipts: recoveries.len() as i64,
            indexing_leases: leases.len() as i64,
            acquired_indexing_leases: leases
                .iter()
                .filter(|row| row.status == IndexLeaseStatus::Acquired)
                .count() as i64,
            quiet_background_work: quiet.len() as i64,
            ..SwarmDashboardAuthorityTotals::default()
        };
        for row in claims {
            *totals
                .claims_by_status
                .entry(row.status.as_str().to_string())
                .or_insert(0) += 1;
        }
        for row in handoffs {
            *totals
                .handoffs_by_status
                .entry(row.status.as_str().to_string())
                .or_insert(0) += 1;
        }
        for row in leases {
            *totals
                .leases_by_status
                .entry(row.status.as_str().to_string())
                .or_insert(0) += 1;
        }
        for row in quiet {
            *totals
                .quiet_work_by_kind
                .entry(row.work_kind.as_str().to_string())
                .or_insert(0) += 1;
        }
        totals
    }

    async fn dashboard_event_watermark(
        &self,
        event_ids: Vec<String>,
    ) -> StateRecoveryResult<SwarmDashboardSourceWatermarkV1> {
        self.ensure_active_access().await?;
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
        let exact = exact_scope_bindings(&self.scope);
        let bindings = EventLookupBindings {
            event_ids: event_ids.clone(),
            owner_account_id: exact.owner_account_id,
            actor_principal_id: exact.actor_principal_id,
            authenticated_session_id: exact.authenticated_session_id,
            access_space_id: exact.access_space_id,
            workspace_id: exact.workspace_id,
        };
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values::<EventWatermarkRow, _>(
                            "SELECT event_id, source_component, aggregate_type, aggregate_id, created_at \
                             FROM kernel_event_ledger \
                             WHERE event_id IN $event_ids \
                               AND source_component = 'parallel_swarm_state_recovery' \
                               AND owner_account_id = $owner_account_id \
                               AND actor_principal_id = $actor_principal_id \
                               AND authenticated_session_id = $authenticated_session_id \
                               AND access_space_id = $access_space_id \
                               AND workspace_id = $workspace_id \
                               AND array::len((SELECT VALUE id FROM authenticated_resource_context_lifecycle \
                                 WHERE lifecycle_state = 'active' \
                                   AND owner_account_id = $owner_account_id \
                                   AND actor_principal_id = $actor_principal_id \
                                   AND authenticated_session_id = $authenticated_session_id \
                                   AND access_space_id = $access_space_id \
                                   AND workspace_id = $workspace_id)) = 1 \
                             ORDER BY created_at DESC, event_id DESC;",
                            bindings,
                        )
                        .await
                })
            })
            .await?;
        let mut found = BTreeSet::new();
        let mut counts = BTreeMap::<String, i64>::new();
        let mut max_event_created_at_utc = None;
        let mut events = Vec::new();
        for row in rows {
            let created_at = row.created_at.into_inner();
            found.insert(row.event_id.clone());
            *counts.entry(row.aggregate_type.clone()).or_insert(0) += 1;
            if max_event_created_at_utc.map_or(true, |current| created_at > current) {
                max_event_created_at_utc = Some(created_at);
            }
            events.push(SwarmDashboardEventRefV1 {
                event_id: row.event_id,
                source_component: row.source_component,
                aggregate_type: row.aggregate_type,
                aggregate_id: row.aggregate_id,
                created_at_utc: created_at,
            });
        }
        let missing_event_refs = event_ids
            .into_iter()
            .filter(|event_id| !found.contains(event_id))
            .collect::<Vec<_>>();
        Ok(SwarmDashboardSourceWatermarkV1 {
            source_component: PARALLEL_SWARM_SOURCE_COMPONENT.to_string(),
            event_count: found.len() as i64,
            max_event_created_at_utc,
            events,
            aggregate_counts: counts
                .into_iter()
                .map(|(aggregate_type, count)| SwarmDashboardAggregateCountV1 {
                    aggregate_type,
                    count,
                })
                .collect(),
            missing_event_refs,
        })
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
        STATE_RECOVERY_AUTHORITY_TABLE,
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
        STATE_RECOVERY_AUTHORITY_TABLE,
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
        STATE_RECOVERY_AUTHORITY_TABLE,
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
        STATE_RECOVERY_AUTHORITY_TABLE,
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
        STATE_RECOVERY_AUTHORITY_TABLE,
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
        STATE_RECOVERY_AUTHORITY_TABLE,
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
        STATE_RECOVERY_AUTHORITY_TABLE,
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
            STATE_RECOVERY_AUTHORITY_TABLE,
            &claim.claim_id,
            Some(event_id),
            Some("parallel_swarm_claim"),
            Some(&claim.claim_id),
        ));
    }
    if let Some(event_id) = claim.reclaim_event_ledger_event_id.as_deref() {
        source_refs.push(dashboard_source_ref(
            STATE_RECOVERY_AUTHORITY_TABLE,
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
            STATE_RECOVERY_AUTHORITY_TABLE,
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
            STATE_RECOVERY_AUTHORITY_TABLE,
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
            STATE_RECOVERY_AUTHORITY_TABLE,
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
            STATE_RECOVERY_AUTHORITY_TABLE,
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
            STATE_RECOVERY_AUTHORITY_TABLE,
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
        "authority=embedded SurrealDB checkpoint row plus EventLedger receipt; this compressed handoff is a projection only".to_string(),
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
