//! WP-KERNEL-009 MT-209..216 ParallelSwarmStateRecovery backend foundations.
//!
//! This module is intentionally backend-only. It gives local/cloud model lanes
//! typed identity, claim leases over shared worktrees/workspaces, role-mailbox
//! handoff receipts, deterministic backend navigation commands, restartable
//! compaction checkpoints, recovery receipts, and a serial lease queue for
//! parallel index writers. PostgreSQL tables from migration 0311 are authority;
//! EventLedger rows provide the receipt trail.

use std::sync::Arc;

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
                WriteLocalIndex,
                WriteMailbox,
                NavigateBackend,
                RecordCheckpoint,
            ],
            AgentLaneKind::Cloud => {
                vec![
                    ClaimWorkspace,
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
                    WriteLocalIndex,
                    NavigateBackend,
                    RecordCheckpoint,
                ]
            }
            AgentLaneKind::Editor => vec![
                ClaimWorkspace,
                EditRichDocument,
                MutateGraph,
                NavigateBackend,
                RecordCheckpoint,
            ],
            AgentLaneKind::System => vec![
                ClaimWorktree,
                ClaimWorkspace,
                EditRichDocument,
                MutateGraph,
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
    pub checkpoints: Vec<RecoveryCheckpointRecord>,
    pub recovery_receipts: Vec<RecoveryReceiptRecord>,
    pub indexing_leases: Vec<IndexingLeaseRecord>,
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

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ResolvedNavigationCommand {
    pub command: BackendNavigationCommand,
    pub command_id: &'static str,
    pub route: &'static str,
    pub params: Value,
    pub deterministic_cache_key: String,
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

        Ok(SwarmEvidenceInspectionSnapshot {
            workspace_id: request.workspace_id,
            claims: claim_rows
                .into_iter()
                .map(work_claim_from_row)
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
                index_run_id, priority, ttl_seconds, status, blocked_by_lease_id,
                acquired_at_utc, expires_at_utc
            )
            VALUES (
                $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,
                CASE WHEN $15 = 'acquired' THEN NOW() ELSE NULL END,
                CASE WHEN $15 = 'acquired' THEN NOW() + ($14::BIGINT * INTERVAL '1 second') ELSE NULL END
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
