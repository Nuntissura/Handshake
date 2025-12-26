use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::str::FromStr;
use thiserror::Error;
use uuid::Uuid;

pub mod postgres;
pub mod retention;
pub mod sqlite;

// Test utilities - exposed for integration tests.
// The helper function `run_storage_conformance` uses Result-based error handling.
pub mod tests;

pub type StorageResult<T> = Result<T, StorageError>;

/// Unified storage error type so callers don't leak provider-specific details.
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("not found: {0}")]
    NotFound(&'static str),
    #[error("conflict: {0}")]
    Conflict(&'static str),
    #[error("validation failed: {0}")]
    Validation(&'static str),
    #[error("mutation guard blocked: {0}")]
    Guard(&'static str),
    #[error("not implemented: {0}")]
    NotImplemented(&'static str),
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Migration(#[from] sqlx::migrate::MigrateError),
}

impl From<serde_json::Error> for StorageError {
    fn from(value: serde_json::Error) -> Self {
        StorageError::Serialization(value.to_string())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct NewWorkspace {
    pub name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub workspace_id: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct NewDocument {
    pub workspace_id: String,
    pub title: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Block {
    pub id: String,
    pub document_id: String,
    pub kind: String,
    pub sequence: i64,
    pub raw_content: String,
    pub display_content: String,
    pub derived_content: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct NewBlock {
    pub id: Option<String>,
    pub document_id: String,
    pub kind: String,
    pub sequence: i64,
    pub raw_content: String,
    pub display_content: Option<String>,
    pub derived_content: Option<Value>,
}

#[derive(Clone, Debug)]
pub struct BlockUpdate {
    pub kind: Option<String>,
    pub sequence: Option<i64>,
    pub raw_content: Option<String>,
    pub display_content: Option<String>,
    pub derived_content: Option<Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Canvas {
    pub id: String,
    pub workspace_id: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CanvasNode {
    pub id: String,
    pub canvas_id: String,
    pub kind: String,
    pub position_x: f64,
    pub position_y: f64,
    pub data: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CanvasEdge {
    pub id: String,
    pub canvas_id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    pub kind: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct NewCanvas {
    pub workspace_id: String,
    pub title: String,
}

#[derive(Clone, Debug)]
pub struct NewCanvasNode {
    pub id: Option<String>,
    pub kind: String,
    pub position_x: f64,
    pub position_y: f64,
    pub data: Option<Value>,
}

#[derive(Clone, Debug)]
pub struct NewCanvasEdge {
    pub id: Option<String>,
    pub from_node_id: String,
    pub to_node_id: String,
    pub kind: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CanvasGraph {
    pub canvas: Canvas,
    pub nodes: Vec<CanvasNode>,
    pub edges: Vec<CanvasEdge>,
}

/// [HSK-GC-001] Artifact classification for retention policies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArtifactKind {
    /// Flight Recorder traces (.jsonl)
    Log,
    /// AI Job outputs / EngineResults
    Result,
    /// Context snapshots (ACE-RAG)
    Evidence,
    /// Web/Model cache
    Cache,
    /// Durable workflow snapshots
    Checkpoint,
}

impl std::fmt::Display for ArtifactKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArtifactKind::Log => write!(f, "log"),
            ArtifactKind::Result => write!(f, "result"),
            ArtifactKind::Evidence => write!(f, "evidence"),
            ArtifactKind::Cache => write!(f, "cache"),
            ArtifactKind::Checkpoint => write!(f, "checkpoint"),
        }
    }
}

/// [HSK-GC-001] Report produced after a prune operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PruneReport {
    pub timestamp: DateTime<Utc>,
    pub items_scanned: u32,
    pub items_pruned: u32,
    pub items_spared_pinned: u32,
    pub items_spared_window: u32,
    pub total_bytes_freed: u64,
}

impl PruneReport {
    pub fn new() -> Self {
        Self {
            timestamp: Utc::now(),
            items_scanned: 0,
            items_pruned: 0,
            items_spared_pinned: 0,
            items_spared_window: 0,
            total_bytes_freed: 0,
        }
    }
}

/// [HSK-GC-001] Retention policy configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetentionPolicy {
    pub kind: ArtifactKind,
    /// Number of days to retain items. Default: 30 for Logs, 7 for Cache.
    pub window_days: u32,
    /// Minimum versions to keep even if expired. Default: 3.
    pub min_versions: u32,
}

impl RetentionPolicy {
    /// Default policy for logs: 30 days, keep min 3 versions.
    pub fn default_log() -> Self {
        Self {
            kind: ArtifactKind::Log,
            window_days: 30,
            min_versions: 3,
        }
    }

    /// Default policy for AI job results: 30 days, keep min 3 versions.
    pub fn default_result() -> Self {
        Self {
            kind: ArtifactKind::Result,
            window_days: 30,
            min_versions: 3,
        }
    }

    /// Default policy for cache: 7 days, keep min 3 versions.
    pub fn default_cache() -> Self {
        Self {
            kind: ArtifactKind::Cache,
            window_days: 7,
            min_versions: 3,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EntityRef {
    pub entity_id: String,
    pub entity_kind: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationType {
    Read,
    Write,
    Plan,
    Execute,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlannedOperation {
    pub op_type: OperationType,
    pub target: EntityRef,
    pub description: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Queued,
    Running,
    Stalled,
    AwaitingValidation,
    AwaitingUser,
    Completed,
    CompletedWithIssues,
    Failed,
    Cancelled,
    Poisoned,
}

impl JobState {
    pub fn as_str(&self) -> &'static str {
        match self {
            JobState::Queued => "queued",
            JobState::Running => "running",
            JobState::Stalled => "stalled",
            JobState::AwaitingValidation => "awaiting_validation",
            JobState::AwaitingUser => "awaiting_user",
            JobState::Completed => "completed",
            JobState::CompletedWithIssues => "completed_with_issues",
            JobState::Failed => "failed",
            JobState::Cancelled => "cancelled",
            JobState::Poisoned => "poisoned",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobKind {
    DocEdit,
    SheetTransform,
    CanvasCluster,
    AsrTranscribe,
    WorkflowRun,
    /// Backward-compatible terminal execution job kind.
    TerminalExec,
    /// Backward-compatible document summarize/test kinds.
    DocSummarize,
    DocTest,
}

impl JobKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            JobKind::DocEdit => "doc_edit",
            JobKind::SheetTransform => "sheet_transform",
            JobKind::CanvasCluster => "canvas_cluster",
            JobKind::AsrTranscribe => "asr_transcribe",
            JobKind::WorkflowRun => "workflow_run",
            JobKind::TerminalExec => "term_exec",
            JobKind::DocSummarize => "doc_summarize",
            JobKind::DocTest => "doc_test",
        }
    }
}

impl FromStr for JobKind {
    type Err = StorageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "doc_edit" => Ok(JobKind::DocEdit),
            "sheet_transform" => Ok(JobKind::SheetTransform),
            "canvas_cluster" => Ok(JobKind::CanvasCluster),
            "asr_transcribe" => Ok(JobKind::AsrTranscribe),
            "workflow_run" => Ok(JobKind::WorkflowRun),
            "term_exec" | "terminal_exec" => Ok(JobKind::TerminalExec),
            "doc_summarize" => Ok(JobKind::DocSummarize),
            "doc_test" => Ok(JobKind::DocTest),
            _ => Err(StorageError::Validation("invalid job kind")),
        }
    }
}

impl TryFrom<&str> for JobState {
    type Error = StorageError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "queued" => Ok(JobState::Queued),
            "running" => Ok(JobState::Running),
            "stalled" => Ok(JobState::Stalled),
            "awaiting_validation" => Ok(JobState::AwaitingValidation),
            "awaiting_user" => Ok(JobState::AwaitingUser),
            "completed" => Ok(JobState::Completed),
            "completed_with_issues" => Ok(JobState::CompletedWithIssues),
            "failed" => Ok(JobState::Failed),
            "cancelled" => Ok(JobState::Cancelled),
            "poisoned" => Ok(JobState::Poisoned),
            _ => Err(StorageError::Validation("invalid job state")),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AccessMode {
    AnalysisOnly,
    PreviewOnly,
    ApplyScoped,
}

impl AccessMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            AccessMode::AnalysisOnly => "analysis_only",
            AccessMode::PreviewOnly => "preview_only",
            AccessMode::ApplyScoped => "apply_scoped",
        }
    }
}

impl TryFrom<&str> for AccessMode {
    type Error = StorageError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "analysis_only" => Ok(AccessMode::AnalysisOnly),
            "preview_only" => Ok(AccessMode::PreviewOnly),
            "apply_scoped" => Ok(AccessMode::ApplyScoped),
            _ => Err(StorageError::Validation("invalid access mode")),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SafetyMode {
    Strict,
    Normal,
    Experimental,
}

impl SafetyMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            SafetyMode::Strict => "strict",
            SafetyMode::Normal => "normal",
            SafetyMode::Experimental => "experimental",
        }
    }
}

impl TryFrom<&str> for SafetyMode {
    type Error = StorageError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "strict" => Ok(SafetyMode::Strict),
            "normal" => Ok(SafetyMode::Normal),
            "experimental" => Ok(SafetyMode::Experimental),
            _ => Err(StorageError::Validation("invalid safety mode")),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JobMetrics {
    #[serde(default)]
    pub duration_ms: u64,
    #[serde(default)]
    pub total_tokens: u32,
    #[serde(default)]
    pub input_tokens: u32,
    #[serde(default)]
    pub output_tokens: u32,
    #[serde(default)]
    pub tokens_planner: u32,
    #[serde(default)]
    pub tokens_executor: u32,
    #[serde(default)]
    pub entities_read: u32,
    #[serde(default)]
    pub entities_written: u32,
    #[serde(default)]
    pub validators_run_count: u32,
}

impl JobMetrics {
    pub fn zero() -> Self {
        Self {
            duration_ms: 0,
            total_tokens: 0,
            input_tokens: 0,
            output_tokens: 0,
            tokens_planner: 0,
            tokens_executor: 0,
            entities_read: 0,
            entities_written: 0,
            validators_run_count: 0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AiJob {
    pub job_id: Uuid,
    pub trace_id: Uuid,
    pub workflow_run_id: Option<Uuid>,
    pub job_kind: JobKind,
    pub state: JobState,
    pub error_message: Option<String>,
    pub protocol_id: String,
    pub profile_id: String,
    pub capability_profile_id: String,
    pub access_mode: AccessMode,
    pub safety_mode: SafetyMode,
    pub entity_refs: Vec<EntityRef>,
    pub planned_operations: Vec<PlannedOperation>,
    pub metrics: JobMetrics,
    pub status_reason: String,
    pub job_inputs: Option<Value>,
    pub job_outputs: Option<Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct NewAiJob {
    pub trace_id: Uuid,
    pub job_kind: JobKind,
    pub protocol_id: String,
    pub profile_id: String,
    pub capability_profile_id: String,
    pub access_mode: AccessMode,
    pub safety_mode: SafetyMode,
    pub entity_refs: Vec<EntityRef>,
    pub planned_operations: Vec<PlannedOperation>,
    pub status_reason: String,
    pub metrics: JobMetrics,
    pub job_inputs: Option<Value>,
}

#[derive(Clone, Debug)]
pub struct JobStatusUpdate {
    pub job_id: Uuid,
    pub state: JobState,
    pub error_message: Option<String>,
    pub status_reason: String,
    pub metrics: Option<JobMetrics>,
    pub workflow_run_id: Option<Uuid>,
    pub trace_id: Option<Uuid>,
    pub job_outputs: Option<Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkflowRun {
    pub id: Uuid,
    pub job_id: Uuid,
    pub status: JobState,
    pub last_heartbeat: chrono::DateTime<chrono::Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkflowNodeExecution {
    pub id: Uuid,
    pub workflow_run_id: Uuid,
    pub node_id: String,
    pub node_type: String,
    pub status: JobState,
    pub sequence: i64,
    pub input_payload: Option<Value>,
    pub output_payload: Option<Value>,
    pub error_message: Option<String>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct NewNodeExecution {
    pub workflow_run_id: Uuid,
    pub node_id: String,
    pub node_type: String,
    pub status: JobState,
    pub sequence: i64,
    pub input_payload: Option<Value>,
    pub started_at: DateTime<Utc>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum WriteActorKind {
    Human,
    Ai,
    System,
}

impl WriteActorKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            WriteActorKind::Human => "HUMAN",
            WriteActorKind::Ai => "AI",
            WriteActorKind::System => "SYSTEM",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MutationMetadata {
    pub actor_kind: WriteActorKind,
    pub actor_id: Option<String>,
    pub job_id: Option<Uuid>,
    pub workflow_id: Option<Uuid>,
    pub edit_event_id: Uuid,
    pub resource_id: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct WriteContext {
    pub actor_kind: WriteActorKind,
    pub actor_id: Option<String>,
    pub job_id: Option<Uuid>,
    pub workflow_id: Option<Uuid>,
}

impl WriteContext {
    pub fn human(actor_id: Option<String>) -> Self {
        Self {
            actor_kind: WriteActorKind::Human,
            actor_id,
            job_id: None,
            workflow_id: None,
        }
    }

    pub fn system(actor_id: Option<String>) -> Self {
        Self {
            actor_kind: WriteActorKind::System,
            actor_id,
            job_id: None,
            workflow_id: None,
        }
    }

    pub fn ai(actor_id: Option<String>, job_id: Option<Uuid>, workflow_id: Option<Uuid>) -> Self {
        Self {
            actor_kind: WriteActorKind::Ai,
            actor_id,
            job_id,
            workflow_id,
        }
    }
}

#[derive(Debug, Error)]
pub enum GuardError {
    #[error("HSK-403-SILENT-EDIT")]
    SilentEdit,
    #[error(transparent)]
    Storage(#[from] StorageError),
}

impl From<GuardError> for StorageError {
    fn from(value: GuardError) -> Self {
        match value {
            GuardError::SilentEdit => StorageError::Guard("HSK-403-SILENT-EDIT"),
            GuardError::Storage(err) => err,
        }
    }
}

#[async_trait]
pub trait StorageGuard: Send + Sync {
    /// Validates the write request against the "No Silent Edits" policy.
    async fn validate_write(
        &self,
        ctx: &WriteContext,
        resource_id: &str,
    ) -> Result<MutationMetadata, GuardError>;
}

pub struct DefaultStorageGuard;

#[async_trait]
impl StorageGuard for DefaultStorageGuard {
    async fn validate_write(
        &self,
        ctx: &WriteContext,
        resource_id: &str,
    ) -> Result<MutationMetadata, GuardError> {
        if ctx.actor_kind == WriteActorKind::Ai
            && (ctx.job_id.is_none() || ctx.workflow_id.is_none())
        {
            return Err(GuardError::SilentEdit);
        }

        Ok(MutationMetadata {
            actor_kind: ctx.actor_kind,
            actor_id: ctx.actor_id.clone(),
            job_id: ctx.job_id,
            workflow_id: ctx.workflow_id,
            edit_event_id: Uuid::new_v4(),
            resource_id: resource_id.to_string(),
            timestamp: Utc::now(),
        })
    }
}

#[async_trait]
pub trait Database: Send + Sync + std::any::Any {
    // Health check
    async fn ping(&self) -> StorageResult<()>;

    // Workspace operations
    async fn list_workspaces(&self) -> StorageResult<Vec<Workspace>>;
    async fn create_workspace(
        &self,
        ctx: &WriteContext,
        workspace: NewWorkspace,
    ) -> StorageResult<Workspace>;
    async fn delete_workspace(&self, ctx: &WriteContext, id: &str) -> StorageResult<()>;
    async fn get_workspace(&self, id: &str) -> StorageResult<Option<Workspace>>;

    // Document operations
    async fn list_documents(&self, workspace_id: &str) -> StorageResult<Vec<Document>>;
    async fn get_document(&self, doc_id: &str) -> StorageResult<Document>;
    async fn create_document(
        &self,
        ctx: &WriteContext,
        doc: NewDocument,
    ) -> StorageResult<Document>;
    async fn delete_document(&self, ctx: &WriteContext, doc_id: &str) -> StorageResult<()>;

    // Block operations
    async fn get_blocks(&self, doc_id: &str) -> StorageResult<Vec<Block>>;
    async fn get_block(&self, block_id: &str) -> StorageResult<Block>;
    async fn create_block(&self, ctx: &WriteContext, block: NewBlock) -> StorageResult<Block>;
    async fn update_block(
        &self,
        ctx: &WriteContext,
        block_id: &str,
        data: BlockUpdate,
    ) -> StorageResult<()>;
    async fn delete_block(&self, ctx: &WriteContext, block_id: &str) -> StorageResult<()>;
    async fn replace_blocks(
        &self,
        ctx: &WriteContext,
        document_id: &str,
        blocks: Vec<NewBlock>,
    ) -> StorageResult<Vec<Block>>;

    // Canvas operations
    async fn create_canvas(&self, ctx: &WriteContext, canvas: NewCanvas) -> StorageResult<Canvas>;
    async fn list_canvases(&self, workspace_id: &str) -> StorageResult<Vec<Canvas>>;
    async fn get_canvas_with_graph(&self, canvas_id: &str) -> StorageResult<CanvasGraph>;
    async fn update_canvas_graph(
        &self,
        ctx: &WriteContext,
        canvas_id: &str,
        nodes: Vec<NewCanvasNode>,
        edges: Vec<NewCanvasEdge>,
    ) -> StorageResult<CanvasGraph>;
    async fn delete_canvas(&self, ctx: &WriteContext, canvas_id: &str) -> StorageResult<()>;

    // AI Job operations (CX-DBP-021)
    async fn get_ai_job(&self, job_id: &str) -> StorageResult<AiJob>;
    async fn create_ai_job(&self, job: NewAiJob) -> StorageResult<AiJob>;
    async fn update_ai_job_status(&self, update: JobStatusUpdate) -> StorageResult<AiJob>;
    async fn set_job_outputs(&self, job_id: &str, outputs: Option<Value>) -> StorageResult<()>;

    // Workflow runs
    async fn create_workflow_run(
        &self,
        job_id: Uuid,
        status: JobState,
        last_heartbeat: Option<DateTime<Utc>>,
    ) -> StorageResult<WorkflowRun>;
    async fn update_workflow_run_status(
        &self,
        run_id: Uuid,
        status: JobState,
        error_message: Option<String>,
    ) -> StorageResult<WorkflowRun>;
    async fn heartbeat_workflow(&self, run_id: Uuid, at: DateTime<Utc>) -> StorageResult<()>;
    async fn create_workflow_node_execution(
        &self,
        exec: NewNodeExecution,
    ) -> StorageResult<WorkflowNodeExecution>;
    async fn update_workflow_node_execution_status(
        &self,
        exec_id: Uuid,
        status: JobState,
        output: Option<Value>,
        error_message: Option<String>,
    ) -> StorageResult<WorkflowNodeExecution>;
    async fn list_workflow_node_executions(
        &self,
        run_id: Uuid,
    ) -> StorageResult<Vec<WorkflowNodeExecution>>;
    async fn find_stalled_workflows(&self, threshold_secs: u64) -> StorageResult<Vec<WorkflowRun>>;

    // Mutation guard
    async fn validate_write_with_guard(
        &self,
        ctx: &WriteContext,
        resource_id: &str,
    ) -> StorageResult<MutationMetadata>;

    // AI Job Pruning [§2.3.11]
    async fn prune_ai_jobs(
        &self,
        cutoff: DateTime<Utc>,
        min_versions: u32,
        dry_run: bool,
    ) -> StorageResult<PruneReport>;

    fn as_any(&self) -> &dyn std::any::Any;
}
