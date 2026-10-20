use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::str::FromStr;
use thiserror::Error;
use uuid::Uuid;

use crate::workspace_safety::MergeBackArtifact;

use crate::kernel::{
    crdt::{persistence::CrdtUpdateRecordV1, snapshot::CrdtSnapshotRecordV1},
    KernelEvent, KernelSessionLease, NewKernelEvent, SessionRun, SessionRunState,
};

use crate::ai_ready_data::records::{
    BronzeRecord, EmbeddingModelRecord, EmbeddingRegistry, NewBronzeRecord, NewSilverRecord,
    SilverRecord,
};

pub(crate) mod block_view_outbox;
/// Embedded-SurrealDB persistence for the block-view outbox, and the reference
/// pattern for porting the remaining outer modules onto the widened
/// `SurrealDataContext` seam.
pub(crate) mod block_view_outbox_surreal;
pub mod calendar;
pub mod calendar_activity;
pub mod fems_memory;
pub mod kb003_storage;
pub mod knowledge;
pub mod knowledge_crdt;
pub mod knowledge_memory;
pub mod knowledge_retrieval;
pub mod loom;
pub mod loom_ai;
pub mod retention;
pub mod stage_artifacts;
pub mod surreal;

pub use calendar::*;
pub use calendar_activity::*;
pub use loom::*;
pub use stage_artifacts::*;

// Test utilities - exposed for integration tests.
// The helper function `run_storage_conformance` uses Result-based error handling.
pub mod tests;

pub type StorageResult<T> = Result<T, StorageError>;

pub const WORKBENCH_LAYOUT_SCHEMA_ID: &str = "hsk.workbench_layout_state@1";
pub const WORKSPACE_SETTINGS_SCHEMA_ID: &str = "hsk.workspace_settings_state@1";
pub const WORKSPACE_SEARCH_BOOKMARK_SCHEMA_ID: &str = "hsk.workspace_search_bookmark_state@1";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkbenchLayoutStateInput {
    pub layout_state: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkbenchLayoutState {
    pub workspace_id: String,
    pub layout_state: Value,
    pub updated_at: DateTime<Utc>,
    pub event_ledger_event_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkspaceSettingsStateInput {
    pub settings_state: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkspaceSettingsState {
    pub workspace_id: String,
    pub settings_state: Value,
    pub updated_at: DateTime<Utc>,
    pub event_ledger_event_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkspaceSearchBookmarkStateInput {
    pub bookmark_state: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkspaceSearchBookmarkState {
    pub workspace_id: String,
    pub bookmark_state: Value,
    pub updated_at: DateTime<Utc>,
    pub event_ledger_event_id: String,
}

/// MT-254 DebugAdapterCore: one durable per-document breakpoint input.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DebugBreakpointInput {
    pub source_url: String,
    pub line: i32,
    #[serde(default)]
    pub condition: Option<String>,
    #[serde(default)]
    pub verified: bool,
}

/// MT-254 DebugAdapterCore: a persisted per-document breakpoint.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DebugBreakpoint {
    pub breakpoint_id: String,
    pub rich_document_id: String,
    pub workspace_id: String,
    pub source_url: String,
    pub line: i32,
    pub condition: Option<String>,
    pub verified: bool,
    pub updated_at: DateTime<Utc>,
    pub event_ledger_event_id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageBackendKind {
    Surreal,
}

pub const HANDSHAKE_STORAGE_MODE_ENV: &str = "HANDSHAKE_STORAGE_MODE";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlPlaneStorageMode {
    /// The Handshake-native embedded SurrealDB store. It is opened and closed by
    /// the Handshake process itself, so no operator-launched database, external
    /// service, or container runtime is part of core operation [CX-503R/CX-503S].
    SurrealEmbedded,
}

impl ControlPlaneStorageMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SurrealEmbedded => "surreal_embedded",
        }
    }

    pub fn is_control_plane_authority(&self) -> bool {
        matches!(self, Self::SurrealEmbedded)
    }

    pub fn authority_label(&self) -> &'static str {
        match self {
            Self::SurrealEmbedded => "primary_authority",
        }
    }

    pub fn freshness_label(&self) -> &'static str {
        match self {
            Self::SurrealEmbedded => "current_source_of_truth",
        }
    }
}

impl std::fmt::Display for ControlPlaneStorageMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ControlPlaneStorageMode {
    type Err = StorageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "surreal_embedded" | "surreal" => Ok(Self::SurrealEmbedded),
            _ => Err(StorageError::Validation("unsupported storage mode")),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlPlaneStorageConfig {
    pub mode: ControlPlaneStorageMode,
    /// Directory that owns the embedded store. Resolved from `HANDSHAKE_DATA_DIR`
    /// when set, otherwise from the platform-local application data directory, so
    /// no machine-local absolute path is baked into the product [CX-109B].
    pub data_dir: Option<PathBuf>,
}

impl ControlPlaneStorageConfig {
    pub fn from_env() -> Result<Self, StorageError> {
        let mode = std::env::var(HANDSHAKE_STORAGE_MODE_ENV).ok();
        let data_dir = std::env::var(surreal::HANDSHAKE_DATA_DIR_ENV).ok();

        Self::resolve(mode.as_deref(), data_dir.as_deref())
    }

    /// Resolves the control-plane storage configuration.
    ///
    /// SurrealDB is the only accepted authority, so an absent mode resolves to it
    /// rather than failing; an explicitly named unsupported mode still errors so a
    /// stale `postgres_primary` setting fails closed instead of being ignored.
    pub fn resolve(mode: Option<&str>, data_dir: Option<&str>) -> Result<Self, StorageError> {
        let mode = match non_empty(mode) {
            Some(value) => ControlPlaneStorageMode::from_str(value)?,
            None => ControlPlaneStorageMode::SurrealEmbedded,
        };

        Ok(Self {
            mode,
            data_dir: non_empty(data_dir).map(PathBuf::from),
        })
    }

    /// Builds the embedded-store configuration this control plane opens.
    pub fn surreal_config(&self) -> Result<surreal::SurrealStorageConfig, StorageError> {
        let config = match self.data_dir.as_ref() {
            Some(dir) => surreal::SurrealStorageConfig::for_data_dir(dir),
            None => surreal::SurrealStorageConfig::from_env(),
        };
        config.map_err(|_| StorageError::Validation("invalid embedded storage data directory"))
    }
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageCapabilitySnapshot {
    pub backend: StorageBackendKind,
    pub supports_structured_collab_artifacts: bool,
    pub supports_loom_graph_filtering: bool,
}

impl StorageCapabilitySnapshot {
    pub fn loom_search_observability_tier(&self) -> u8 {
        match self.backend {
            StorageBackendKind::Surreal => 2,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructuredCollabWorkPacketRow {
    pub wp_id: String,
    pub version: i64,
    pub title: String,
    pub description: String,
    pub status: String,
    pub priority: i64,
    pub phase: String,
    pub routing: String,
    pub task_packet_path: Option<String>,
    pub task_board_status: String,
    pub assignee: Option<String>,
    pub reporter: String,
    pub created_at: String,
    pub updated_at: String,
    pub vector_clock: String,
    pub metadata: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructuredCollabTaskBoardProjectionRow {
    pub wp_id: String,
    pub task_board_status: String,
    pub metadata: String,
    pub updated_at: String,
}

/// Unified storage error type so callers don't leak provider-specific details.
pub use handshake_storage_support::StorageError;

// The former relational-driver error conversions
// are gone with PostgreSQL. The embedded store converts through
// `From<surreal::SurrealStorageError>` below, which serves the same purpose:
// it keeps the driver's message while keeping the driver type out of the
// public API.
impl From<surreal::SurrealStorageError> for StorageError {
    fn from(err: surreal::SurrealStorageError) -> Self {
        StorageError::Database(err.to_string())
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

/// Document block with content and classification metadata.
///
/// Classification fields support ACE runtime validators [HSK-ACE-VAL-100]:
/// - `sensitivity`: Content sensitivity level ("low"|"medium"|"high"|"unknown")
/// - `exportable`: Whether content can be sent to cloud models
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
    /// Content sensitivity level [HSK-ACE-VAL-100]
    /// Values: "low", "medium", "high", "unknown" (NULL treated as "unknown")
    pub sensitivity: Option<String>,
    /// Whether content can be exported to cloud models [HSK-ACE-VAL-100]
    /// NULL or true = exportable, false = local-only
    pub exportable: Option<bool>,
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
    /// Content sensitivity level [HSK-ACE-VAL-100]
    pub sensitivity: Option<String>,
    /// Whether content can be exported to cloud models [HSK-ACE-VAL-100]
    pub exportable: Option<bool>,
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

impl Default for PruneReport {
    fn default() -> Self {
        Self::new()
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

/// Artifact system foundations (Phase 1): on-disk artifact store + manifests + hashing + LocalFile
/// materialize helper.
///
/// Spec anchors:
/// - 2.3.10.6 Artifact manifests + on-disk layout (normative)
/// - 2.3.10.7 Bundles + canonical hashing (normative)
/// - 2.3.11.2 Materialize rules (atomic + traversal-safe + no bypass)
pub mod artifacts;

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
    DocRewrite,
    SheetTransform,
    CanvasCluster,
    AsrTranscribe,
    ModelRun,
    WorkflowRun,
    MediaDownloader,
    MicroTaskExecution,
    SpecRouter,
    LocusOperation,
    LoomPreviewGenerate,
    /// Backward-compatible terminal execution job kind.
    TerminalExec,
    /// Document summarization job kind.
    DocSummarize,
    /// Debug bundle export job [§10.5.6.8]
    DebugBundleExport,
    DocIngest,
    DistillationEval,
}

impl JobKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            JobKind::DocEdit => "doc_edit",
            JobKind::DocRewrite => "doc_rewrite",
            JobKind::SheetTransform => "sheet_transform",
            JobKind::CanvasCluster => "canvas_cluster",
            JobKind::AsrTranscribe => "asr_transcribe",
            JobKind::ModelRun => "model_run",
            JobKind::WorkflowRun => "workflow_run",
            JobKind::MediaDownloader => "media_downloader",
            JobKind::MicroTaskExecution => "micro_task_execution",
            JobKind::SpecRouter => "spec_router",
            JobKind::LocusOperation => "locus_operation",
            JobKind::LoomPreviewGenerate => "loom_preview_generate",
            JobKind::TerminalExec => "terminal_exec",
            JobKind::DocSummarize => "doc_summarize",
            JobKind::DebugBundleExport => "debug_bundle_export",
            JobKind::DocIngest => "doc_ingest",
            JobKind::DistillationEval => "distillation_eval",
        }
    }
}

impl FromStr for JobKind {
    type Err = StorageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "doc_edit" => Ok(JobKind::DocEdit),
            "doc_rewrite" => Ok(JobKind::DocRewrite),
            "sheet_transform" => Ok(JobKind::SheetTransform),
            "canvas_cluster" => Ok(JobKind::CanvasCluster),
            "asr_transcribe" => Ok(JobKind::AsrTranscribe),
            "model_run" => Ok(JobKind::ModelRun),
            "workflow_run" => Ok(JobKind::WorkflowRun),
            "media_downloader" => Ok(JobKind::MediaDownloader),
            "micro_task_execution" => Ok(JobKind::MicroTaskExecution),
            "spec_router" => Ok(JobKind::SpecRouter),
            "locus_operation" => Ok(JobKind::LocusOperation),
            "loom_preview_generate" => Ok(JobKind::LoomPreviewGenerate),
            "term_exec" | "terminal_exec" => Ok(JobKind::TerminalExec),
            "doc_summarize" => Ok(JobKind::DocSummarize),
            "debug_bundle_export" => Ok(JobKind::DebugBundleExport),
            "doc_ingest" => Ok(JobKind::DocIngest),
            "distillation_eval" => Ok(JobKind::DistillationEval),
            _ => Err(StorageError::Validation("invalid job kind")),
        }
    }
}

pub fn validate_job_contract(
    job_kind: &JobKind,
    profile_id: &str,
    protocol_id: &str,
) -> StorageResult<()> {
    const MICRO_TASK_EXECUTOR_V1_ID: &str = "micro_task_executor_v1";
    const MD_BATCH_PROTOCOL_ID_V0: &str = "hsk.media_downloader.batch.v0";
    const MD_CONTROL_PROTOCOL_ID_V0: &str = "hsk.media_downloader.control.v0";
    const MD_COOKIE_IMPORT_PROTOCOL_ID_V0: &str = "hsk.media_downloader.cookie_import.v0";
    const LOOM_PREVIEW_GENERATE_PROTOCOL_ID_V1: &str = "hsk.loom.preview_generate@v1";

    let is_mte_profile = profile_id == MICRO_TASK_EXECUTOR_V1_ID;
    let is_mte_protocol = protocol_id == MICRO_TASK_EXECUTOR_V1_ID;
    let is_mte_kind = matches!(job_kind, JobKind::MicroTaskExecution);

    if is_mte_kind && (!is_mte_profile || !is_mte_protocol) {
        return Err(StorageError::Validation(
            "invalid job contract: micro_task_execution requires micro_task_executor_v1 profile_id and protocol_id",
        ));
    }

    if (is_mte_profile || is_mte_protocol) && (!is_mte_profile || !is_mte_protocol) {
        return Err(StorageError::Validation(
            "invalid job contract: micro_task_executor_v1 requires profile_id and protocol_id to match",
        ));
    }

    if (is_mte_profile || is_mte_protocol) && !is_mte_kind {
        return Err(StorageError::Validation(
            "invalid job contract: micro_task_executor_v1 requires job_kind micro_task_execution",
        ));
    }

    let is_md_kind = matches!(job_kind, JobKind::MediaDownloader);
    let is_md_protocol = protocol_id == MD_BATCH_PROTOCOL_ID_V0
        || protocol_id == MD_CONTROL_PROTOCOL_ID_V0
        || protocol_id == MD_COOKIE_IMPORT_PROTOCOL_ID_V0;

    if is_md_kind && !is_md_protocol {
        return Err(StorageError::Validation(
            "invalid job contract: media_downloader requires protocol_id hsk.media_downloader.{batch|control|cookie_import}.v0",
        ));
    }

    let is_loom_preview_kind = matches!(job_kind, JobKind::LoomPreviewGenerate);
    let is_loom_preview_protocol = protocol_id == LOOM_PREVIEW_GENERATE_PROTOCOL_ID_V1;
    if is_loom_preview_kind && !is_loom_preview_protocol {
        return Err(StorageError::Validation(
            "invalid job contract: loom_preview_generate requires protocol_id hsk.loom.preview_generate@v1",
        ));
    }
    if is_loom_preview_protocol && !is_loom_preview_kind {
        return Err(StorageError::Validation(
            "invalid job contract: hsk.loom.preview_generate@v1 requires job_kind loom_preview_generate",
        ));
    }

    Ok(())
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ModelSessionState {
    Created,
    Active,
    Paused,
    Blocked,
    Completed,
    Failed,
    Cancelled,
}

impl ModelSessionState {
    pub fn as_str(&self) -> &'static str {
        match self {
            ModelSessionState::Created => "CREATED",
            ModelSessionState::Active => "ACTIVE",
            ModelSessionState::Paused => "PAUSED",
            ModelSessionState::Blocked => "BLOCKED",
            ModelSessionState::Completed => "COMPLETED",
            ModelSessionState::Failed => "FAILED",
            ModelSessionState::Cancelled => "CANCELLED",
        }
    }
}

impl TryFrom<&str> for ModelSessionState {
    type Error = StorageError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "CREATED" | "created" => Ok(ModelSessionState::Created),
            "ACTIVE" | "active" => Ok(ModelSessionState::Active),
            "PAUSED" | "paused" => Ok(ModelSessionState::Paused),
            "BLOCKED" | "blocked" => Ok(ModelSessionState::Blocked),
            "COMPLETED" | "completed" => Ok(ModelSessionState::Completed),
            "FAILED" | "failed" => Ok(ModelSessionState::Failed),
            "CANCELLED" | "cancelled" => Ok(ModelSessionState::Cancelled),
            _ => Err(StorageError::Validation("invalid model session state")),
        }
    }
}

impl ModelSessionState {
    /// MT-142: whether this state closes the session. Only terminal states may
    /// carry close metadata (close reason / closing actor / closed-at).
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            ModelSessionState::Completed | ModelSessionState::Failed | ModelSessionState::Cancelled
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SessionMessageRole {
    System,
    User,
    Assistant,
    ToolCall,
    ToolResult,
}

impl SessionMessageRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            SessionMessageRole::System => "SYSTEM",
            SessionMessageRole::User => "USER",
            SessionMessageRole::Assistant => "ASSISTANT",
            SessionMessageRole::ToolCall => "TOOL_CALL",
            SessionMessageRole::ToolResult => "TOOL_RESULT",
        }
    }
}

impl TryFrom<&str> for SessionMessageRole {
    type Error = StorageError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "SYSTEM" | "system" => Ok(SessionMessageRole::System),
            "USER" | "user" => Ok(SessionMessageRole::User),
            "ASSISTANT" | "assistant" => Ok(SessionMessageRole::Assistant),
            "TOOL_CALL" | "tool_call" => Ok(SessionMessageRole::ToolCall),
            "TOOL_RESULT" | "tool_result" => Ok(SessionMessageRole::ToolResult),
            _ => Err(StorageError::Validation("invalid session message role")),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelSession {
    pub session_id: String,
    pub parent_session_id: Option<String>,
    pub spawn_depth: i32,
    pub state: ModelSessionState,
    pub model_id: String,
    pub backend: String,
    pub parameter_class: String,
    pub role: String,
    pub wp_id: Option<String>,
    pub mt_id: Option<String>,
    pub work_profile_id: Option<String>,
    pub execution_mode: String,
    pub memory_policy: String,
    pub consent_receipt_id: Option<String>,
    pub capability_grants: Vec<String>,
    pub capability_token_ids: Option<Vec<String>>,
    pub job_id: Option<Uuid>,
    pub checkpoint_artifact_id: Option<String>,
    pub last_checkpoint_at: Option<DateTime<Utc>>,
    pub checkpoint_count: i64,
    pub merge_back_artifact: Option<MergeBackArtifact>,
    /// MT-142: the agent identity that owns the session (e.g. a model/session
    /// principal), durable across restarts.
    #[serde(default)]
    pub agent: Option<String>,
    /// MT-142: why the session exists (operator/work intent), durable across
    /// restarts.
    #[serde(default)]
    pub purpose: Option<String>,
    /// MT-142: why the session was closed. Set only by
    /// [`Database::close_model_session`] together with a terminal state.
    #[serde(default)]
    pub close_reason: Option<String>,
    /// MT-142: the actor that closed the session.
    #[serde(default)]
    pub closed_by_actor: Option<String>,
    /// MT-142: when the session was closed.
    #[serde(default)]
    pub closed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct NewModelSession {
    pub session_id: String,
    pub parent_session_id: Option<String>,
    pub spawn_depth: i32,
    pub state: ModelSessionState,
    pub model_id: String,
    pub backend: String,
    pub parameter_class: String,
    pub role: String,
    pub wp_id: Option<String>,
    pub mt_id: Option<String>,
    pub work_profile_id: Option<String>,
    pub execution_mode: String,
    pub memory_policy: String,
    pub consent_receipt_id: Option<String>,
    pub capability_grants: Vec<String>,
    pub capability_token_ids: Option<Vec<String>>,
    pub job_id: Option<Uuid>,
    pub checkpoint_artifact_id: Option<String>,
    pub last_checkpoint_at: Option<DateTime<Utc>>,
    pub checkpoint_count: i64,
    /// MT-142: durable agent identity for the session.
    pub agent: Option<String>,
    /// MT-142: durable session purpose.
    pub purpose: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionCheckpoint {
    pub checkpoint_id: String,
    pub session_id: String,
    pub timestamp: DateTime<Utc>,
    pub session_state_json: String,
    pub message_thread_tail_id: String,
    pub pending_tool_calls_json: String,
    pub checkpoint_artifact_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionMessage {
    pub message_id: String,
    pub session_id: String,
    pub role: SessionMessageRole,
    pub content_hash: String,
    pub content_artifact_id: String,
    pub token_count: Option<i64>,
    pub redacted: bool,
    pub tool_call_id: Option<String>,
    pub attachments: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct NewSessionMessage {
    pub message_id: Option<String>,
    pub session_id: String,
    pub role: SessionMessageRole,
    pub content_hash: String,
    pub content_artifact_id: String,
    pub token_count: Option<i64>,
    pub redacted: bool,
    pub tool_call_id: Option<String>,
    pub attachments: Vec<String>,
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

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AiJobMcpFields {
    pub mcp_server_id: Option<String>,
    pub mcp_call_id: Option<String>,
    pub mcp_progress_token: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct AiJobMcpUpdate {
    pub mcp_server_id: Option<String>,
    pub mcp_call_id: Option<String>,
    pub mcp_progress_token: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct AiJobListFilter {
    pub status: Option<JobState>,
    pub job_kind: Option<JobKind>,
    pub wsid: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
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

#[derive(Debug, Error)]
pub enum JobAdmissionError {
    #[error("job admission storage failure")]
    Storage(#[from] StorageError),
    #[error("job admission creation basis is invalid")]
    Basis(#[from] handshake_job_admission::CreationCodecError),
    #[error("job admission startup recovery gate failed")]
    StartupRecovery(#[from] crate::workflows::StartupRecoveryError),
    #[error("HSK-403-PROTECTED-RESOURCE")]
    Denied,
    #[error("invalid job admission command: {0}")]
    InvalidCommand(&'static str),
    #[error("job creation conflicts with the original immutable command")]
    Conflict,
    #[error("job creation integrity failure: {0}")]
    Integrity(&'static str),
    #[error("job admission outcome is unknown: {0}")]
    Unknown(&'static str),
    #[error("job admission already has an initial submission")]
    AlreadySubmitted,
}

impl From<surreal::SurrealStorageError> for JobAdmissionError {
    fn from(error: surreal::SurrealStorageError) -> Self {
        Self::Storage(StorageError::from(error))
    }
}

#[derive(Clone)]
pub(crate) struct JobCreationCommand {
    checked: handshake_job_admission::CheckedJobCreation,
    input: NewAiJob,
}

impl JobCreationCommand {
    pub(crate) fn from_checked(
        checked: handshake_job_admission::CheckedJobCreation,
    ) -> Result<Self, JobAdmissionError> {
        use handshake_job_admission::CreationOperationType;
        let descriptor = checked.descriptor();
        let job_kind = descriptor.job_kind.parse::<JobKind>()?;
        validate_job_contract(&job_kind, &descriptor.profile_id, &descriptor.protocol_id)?;
        let input = NewAiJob {
            trace_id: checked.identity().initial_trace_id(),
            job_kind,
            protocol_id: descriptor.protocol_id.clone(),
            profile_id: descriptor.profile_id.clone(),
            capability_profile_id: descriptor.capability_profile_id.clone(),
            access_mode: AccessMode::try_from(descriptor.access_mode.as_str())?,
            safety_mode: SafetyMode::try_from(descriptor.safety_mode.as_str())?,
            entity_refs: descriptor.entity_refs.iter().map(|entity| EntityRef {
                entity_id: entity.entity_id.clone(),
                entity_kind: entity.entity_kind.clone(),
            }).collect(),
            planned_operations: descriptor.planned_operations.iter().map(|operation| PlannedOperation {
                op_type: match operation.op_type {
                    CreationOperationType::Read => OperationType::Read,
                    CreationOperationType::Write => OperationType::Write,
                    CreationOperationType::Plan => OperationType::Plan,
                    CreationOperationType::Execute => OperationType::Execute,
                },
                target: EntityRef {
                    entity_id: operation.target.entity_id.clone(),
                    entity_kind: operation.target.entity_kind.clone(),
                },
                description: operation.description.clone(),
            }).collect(),
            status_reason: "queued".to_owned(),
            metrics: JobMetrics::zero(),
            job_inputs: descriptor.job_inputs.clone(),
        };
        Ok(Self { checked, input })
    }

    pub(crate) fn checked(&self) -> &handshake_job_admission::CheckedJobCreation { &self.checked }
    pub(crate) fn input(&self) -> &NewAiJob { &self.input }
}

pub(crate) struct CanonicalJobCreation {
    job: AiJob,
    creation: handshake_job_admission::CheckedJobCreation,
    reserved_workflow: Option<WorkflowRun>,
}

impl CanonicalJobCreation {
    pub(crate) fn from_verified_rows(
        job: AiJob,
        creation: handshake_job_admission::CheckedJobCreation,
        reserved_workflow: Option<WorkflowRun>,
    ) -> StorageResult<Self> {
        let identity = creation.identity();
        if job.job_id != identity.job_id() {
            return Err(StorageError::Validation("job creation identity mismatch"));
        }
        match (identity.reserved_workflow_run_id(), reserved_workflow.as_ref()) {
            (None, None) => {}
            (Some(expected), Some(workflow))
                if workflow.id == expected && workflow.job_id == identity.job_id() => {}
            _ => return Err(StorageError::Validation("job creation reserved workflow mismatch")),
        }
        Ok(Self { job, creation, reserved_workflow })
    }

    pub(crate) fn job(&self) -> &AiJob { &self.job }
    pub(crate) fn creation(&self) -> &handshake_job_admission::CheckedJobCreation { &self.creation }
    pub(crate) fn reserved_workflow(&self) -> Option<&WorkflowRun> { self.reserved_workflow.as_ref() }
}

pub(crate) enum JobCreateCommitOutcome {
    Created(CanonicalJobCreation),
    AlreadyCreated(CanonicalJobCreation),
}

pub(crate) enum JobCreationObservation {
    Observed(CanonicalJobCreation),
    Unknown,
}

pub(crate) type JobInitialWriter = tokio::task::JoinHandle<Result<JobCreateCommitOutcome, JobAdmissionError>>;
pub(crate) type JobInitialWriterResult = Result<Result<JobCreateCommitOutcome, JobAdmissionError>, tokio::task::JoinError>;

struct RejectedJobInitialWriter {
    reason: handshake_job_admission::SubmissionStateError,
    writer: JobInitialWriter,
    outcome: Option<JobInitialWriterResult>,
}

pub(crate) struct JobInitialObservation<'a> {
    pub primary: Option<&'a JobInitialWriterResult>,
    pub rejected: Option<JobRejectedObservation<'a>>,
}

pub(crate) enum JobRejectedObservation<'a> {
    Pending { reason: handshake_job_admission::SubmissionStateError },
    Observed {
        reason: handshake_job_admission::SubmissionStateError,
        outcome: &'a JobInitialWriterResult,
    },
}

pub(crate) enum JobSubmissionSettlement<'a> {
    NotSubmitted,
    Pending,
    Observed(JobInitialObservation<'a>),
}

#[must_use = "the original caller retains this admission until its actual owners settle"]
pub struct JobAdmission {
    command: JobCreationCommand,
    operation_id: Uuid,
    cancellation: std::sync::Arc<surreal::ContextOperationCancellation>,
    retention: surreal::resource_authority::JobAdmissionTransactionRetention,
    initial: handshake_job_admission::JobCreationState<JobInitialWriter, JobInitialWriterResult>,
    rejected_initial: Option<RejectedJobInitialWriter>,
}

impl JobAdmission {
    pub(crate) fn from_captured_authority(
        operation_id: Uuid,
        cancellation: std::sync::Arc<surreal::ContextOperationCancellation>,
        command: JobCreationCommand,
        retention: surreal::resource_authority::JobAdmissionTransactionRetention,
    ) -> Result<Self, JobAdmissionError> {
        use surreal::resource_authority::ResourceAuthorityError;
        retention.validate_creation(operation_id, command.checked()).map_err(|error| match error {
            ResourceAuthorityError::Storage(error) => JobAdmissionError::Storage(error.into()),
            ResourceAuthorityError::Denied { .. }
            | ResourceAuthorityError::InvalidInput(_)
            | ResourceAuthorityError::Entropy(_) => JobAdmissionError::Denied,
        })?;
        let initial = handshake_job_admission::JobCreationState::new(command.checked().clone());
        Ok(Self { command, operation_id, cancellation, retention, initial, rejected_initial: None })
    }

    pub fn job_id(&self) -> Uuid { self.command.checked().identity().job_id() }
    pub(crate) fn command(&self) -> &JobCreationCommand { &self.command }
    pub(crate) fn operation_id(&self) -> Uuid { self.operation_id }
    pub(crate) fn cancellation(&self) -> &std::sync::Arc<surreal::ContextOperationCancellation> { &self.cancellation }
    pub(crate) fn retention(&self) -> &surreal::resource_authority::JobAdmissionTransactionRetention { &self.retention }

    pub(crate) fn can_submit(&self) -> bool {
        self.rejected_initial.is_none()
            && matches!(self.initial.view(), handshake_job_admission::JobCreationView::Prepared { .. })
    }

    pub(crate) fn install_initial(&mut self, writer: JobInitialWriter) -> Result<(), JobAdmissionError> {
        match self.initial.attach_writer(writer) {
            Ok(()) => Ok(()),
            Err(rejected) => {
                let (reason, writer) = rejected.into_parts();
                self.rejected_initial = Some(RejectedJobInitialWriter { reason, writer, outcome: None });
                Err(JobAdmissionError::Unknown("initial_writer_install_rejected"))
            }
        }
    }

    pub(crate) fn initial_result(&self) -> JobInitialObservation<'_> {
        JobInitialObservation {
            primary: self.initial.outcome(),
            rejected: self.rejected_initial.as_ref().map(|rejected| match &rejected.outcome {
                Some(outcome) => JobRejectedObservation::Observed { reason: rejected.reason, outcome },
                None => JobRejectedObservation::Pending { reason: rejected.reason },
            }),
        }
    }

    pub(crate) async fn settle<'a>(
        &'a mut self,
        original_deadline: tokio::time::Instant,
    ) -> JobSubmissionSettlement<'a> {
        use std::{future::Future, pin::Pin, task::Poll};
        if self.can_submit() {
            return JobSubmissionSettlement::NotSubmitted;
        }
        let completed = {
            let observe = std::future::poll_fn(|context| {
                let primary_done = matches!(self.initial.view(), handshake_job_admission::JobCreationView::Prepared { .. })
                    || self.initial.poll_writer(context).is_ready();
                let rejected_done = match &mut self.rejected_initial {
                    Some(rejected) => {
                        if rejected.outcome.is_none() {
                            if let Poll::Ready(outcome) = Pin::new(&mut rejected.writer).poll(context) {
                                rejected.outcome = Some(outcome);
                            }
                        }
                        rejected.outcome.is_some()
                    }
                    None => true,
                };
                if primary_done && rejected_done { Poll::Ready(()) } else { Poll::Pending }
            });
            tokio::time::timeout_at(original_deadline, observe).await.is_ok()
        };
        if completed {
            JobSubmissionSettlement::Observed(self.initial_result())
        } else {
            JobSubmissionSettlement::Pending
        }
    }
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
pub struct GovernanceCheckRun {
    pub id: Uuid,
    pub check_id: Uuid,
    pub session_id: Uuid,
    pub check_name: String,
    pub check_kind: String,
    pub descriptor_hash: String,
    pub result_status: String,
    pub checks_duration_ms: u64,
    pub evidence_artifact_id: Option<String>,
    pub evidence_artifact_content_hash: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct NewGovernanceCheckRun {
    pub check_id: Uuid,
    pub session_id: Uuid,
    pub check_name: String,
    pub check_kind: String,
    pub descriptor_hash: String,
    pub result_status: String,
    pub checks_duration_ms: u64,
    pub evidence_artifact_id: Option<String>,
    pub evidence_artifact_content_hash: Option<String>,
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

pub type WriteActor = WriteActorKind;

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

#[derive(Clone, Debug)]
pub struct MutationTraceabilityRow {
    pub last_actor_kind: String,
    pub last_actor_id: Option<String>,
    pub last_job_id: Option<String>,
    pub last_workflow_id: Option<String>,
    pub edit_event_id: String,
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
            edit_event_id: Uuid::now_v7(),
            resource_id: resource_id.to_string(),
            timestamp: Utc::now(),
        })
    }
}

pub trait StorageCapabilityStore: Send + Sync {
    fn storage_capabilities(&self) -> StorageCapabilitySnapshot;
}

#[async_trait]
pub trait StructuredCollaborationStore: StorageCapabilityStore + Send + Sync {
    async fn locus_work_packet_exists(&self, wp_id: &str) -> StorageResult<bool>;
    async fn execute_locus_operation(
        &self,
        op: crate::workflows::locus::types::LocusOperation,
    ) -> StorageResult<Value>;
    async fn locus_task_board_get_status_and_metadata(
        &self,
        wp_id: &str,
    ) -> StorageResult<Option<(i64, String, String)>>;
    async fn locus_task_board_update_work_packet(
        &self,
        expected_version: i64,
        status: &str,
        task_board_status: &str,
        updated_at: &str,
        metadata: &str,
        wp_id: &str,
    ) -> StorageResult<()>;
    async fn locus_task_board_list_rows(&self) -> StorageResult<Vec<(String, String, String)>>;
    async fn structured_collab_work_packet_row(
        &self,
        wp_id: &str,
    ) -> StorageResult<Option<StructuredCollabWorkPacketRow>>;
    async fn structured_collab_work_packet_rows(
        &self,
    ) -> StorageResult<Vec<StructuredCollabWorkPacketRow>>;
    async fn structured_collab_micro_task_status_rows(
        &self,
        wp_id: &str,
    ) -> StorageResult<Vec<(String, String)>>;
    async fn structured_collab_micro_task_metadata(
        &self,
        wp_id: &str,
        mt_id: &str,
    ) -> StorageResult<Option<String>>;
    async fn structured_collab_micro_task_rows(
        &self,
        wp_id: &str,
    ) -> StorageResult<Vec<(String, String)>>;
}

#[async_trait]
pub trait Database: Send + Sync {
    fn supports_locus_runtime(&self) -> bool {
        false
    }

    fn supports_structured_collab_artifacts(&self) -> bool {
        false
    }

    fn loom_search_observability_tier(&self) -> u8;

    fn supports_loom_graph_filtering(&self) -> bool {
        false
    }

    fn loom_traverse_graph_perf_target_ms(&self) -> u128;

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

    // Loom (WP-1-Loom-MVP-v1)
    async fn create_asset(&self, ctx: &WriteContext, asset: NewAsset) -> StorageResult<Asset>;
    async fn get_loom_artifact_binding(
        &self,
        _workspace_id: &str,
        _asset_id: &str,
    ) -> StorageResult<Option<LoomArtifactBinding>> {
        Err(StorageError::NotImplemented("get_loom_artifact_binding"))
    }
    async fn reserve_loom_artifact_binding(
        &self,
        _ctx: &WriteContext,
        _expected: &Asset,
        _retention_ttl_days: Option<u32>,
    ) -> StorageResult<LoomArtifactBinding> {
        Err(StorageError::NotImplemented("reserve_loom_artifact_binding"))
    }
    async fn publish_loom_artifact_binding(
        &self,
        _ctx: &WriteContext,
        _verified: VerifiedLoomArtifact,
    ) -> StorageResult<LoomArtifactBinding> {
        Err(StorageError::NotImplemented("publish_loom_artifact_binding"))
    }
    async fn get_asset(&self, workspace_id: &str, asset_id: &str) -> StorageResult<Asset>;
    async fn find_asset_by_content_hash(
        &self,
        workspace_id: &str,
        content_hash: &str,
    ) -> StorageResult<Option<Asset>>;

    // MT-259 MediaCacheTiers (GAP-LM-009): per-asset, per-tier cache state.
    async fn upsert_media_tier(
        &self,
        ctx: &WriteContext,
        upsert: MediaTierUpsert,
    ) -> StorageResult<MediaAssetTier>;
    /// Set a tier's status; on `failed` -> `pending` (retry) bump attempt_count.
    async fn set_media_tier_status(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        asset_id: &str,
        tier: MediaTier,
        status: MediaTierStatus,
        failure_reason: Option<String>,
    ) -> StorageResult<MediaAssetTier>;
    async fn get_media_tier(
        &self,
        workspace_id: &str,
        asset_id: &str,
        tier: MediaTier,
    ) -> StorageResult<Option<MediaAssetTier>>;
    async fn list_media_tiers(
        &self,
        workspace_id: &str,
        asset_id: &str,
    ) -> StorageResult<Vec<MediaAssetTier>>;
    async fn list_failed_media_tiers(
        &self,
        workspace_id: &str,
    ) -> StorageResult<Vec<MediaAssetTier>>;
    /// Delete derived tier rows for an asset. MUST NOT touch the original blob.
    async fn delete_media_tiers(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        asset_id: &str,
    ) -> StorageResult<u64>;

    // MT-259 GAP-LM-244a: backend album/slideshow list-source.
    async fn create_loom_collection(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        title: Option<String>,
    ) -> StorageResult<LoomCollection>;
    async fn get_loom_collection(
        &self,
        workspace_id: &str,
        collection_id: &str,
    ) -> StorageResult<LoomCollectionWithMembers>;
    /// Replace the ordered member list (re-densifies positions 0..n).
    async fn set_loom_collection_order(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        collection_id: &str,
        asset_ids: &[String],
    ) -> StorageResult<LoomCollectionWithMembers>;

    async fn create_loom_block(
        &self,
        ctx: &WriteContext,
        block: NewLoomBlock,
    ) -> StorageResult<LoomBlock>;
    async fn get_or_create_daily_journal_block(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        journal_date: &str,
    ) -> StorageResult<LoomBlock>;
    async fn get_loom_block(&self, workspace_id: &str, block_id: &str) -> StorageResult<LoomBlock>;
    async fn find_loom_block_by_content_hash(
        &self,
        workspace_id: &str,
        content_hash: &str,
    ) -> StorageResult<Option<LoomBlock>>;
    async fn find_loom_block_by_asset_id(
        &self,
        workspace_id: &str,
        asset_id: &str,
    ) -> StorageResult<Option<LoomBlock>>;
    async fn update_loom_block(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        block_id: &str,
        update: LoomBlockUpdate,
    ) -> StorageResult<LoomBlock>;
    async fn set_loom_block_preview(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        block_id: &str,
        preview_status: PreviewStatus,
        thumbnail_asset_id: Option<String>,
        proxy_asset_id: Option<String>,
    ) -> StorageResult<()>;
    async fn delete_loom_block(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        block_id: &str,
    ) -> StorageResult<()>;

    async fn create_loom_edge(
        &self,
        ctx: &WriteContext,
        edge: NewLoomEdge,
    ) -> StorageResult<LoomEdge>;
    async fn delete_loom_edge(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        edge_id: &str,
    ) -> StorageResult<LoomEdge>;
    async fn list_loom_edges_for_block(
        &self,
        workspace_id: &str,
        block_id: &str,
    ) -> StorageResult<Vec<LoomEdge>>;
    async fn get_backlinks(
        &self,
        workspace_id: &str,
        block_id: &str,
    ) -> StorageResult<Vec<LoomEdge>>;
    async fn get_outgoing_edges(
        &self,
        workspace_id: &str,
        block_id: &str,
    ) -> StorageResult<Vec<LoomEdge>>;
    async fn traverse_graph(
        &self,
        workspace_id: &str,
        start_block_id: &str,
        max_depth: u32,
        edge_types: &[LoomEdgeType],
    ) -> StorageResult<Vec<(LoomBlock, u32)>>;
    async fn recompute_block_metrics(
        &self,
        workspace_id: &str,
        block_id: &str,
    ) -> StorageResult<()>;
    async fn recompute_all_metrics(&self, workspace_id: &str) -> StorageResult<()>;

    async fn query_loom_view(
        &self,
        workspace_id: &str,
        view_type: LoomViewType,
        filters: LoomViewFilters,
        limit: u32,
        offset: u32,
    ) -> StorageResult<LoomViewResponse>;
    async fn search_loom_blocks(
        &self,
        workspace_id: &str,
        query: &str,
        filters: LoomSearchFilters,
        limit: u32,
        offset: u32,
    ) -> StorageResult<Vec<LoomBlockSearchResult>>;
    async fn search_loom_graph(
        &self,
        workspace_id: &str,
        query: &str,
        filters: LoomSearchFilters,
        limit: u32,
        offset: u32,
    ) -> StorageResult<Vec<LoomGraphSearchResult>>;

    /// WP-KERNEL-009 MT-264 LoomSearchV2: refresh the derived search-index
    /// projection row for one block (FTS text + optional REAL embedding) inside
    /// the authority write path, EventLedger-receipted. `embedding` is `None`
    /// when no embedding model is configured (keyword/trigram only — never
    /// fabricated). Called on block create/update; delete is handled by the
    /// ON DELETE CASCADE FK so a deleted block can never leave a stale hit.
    async fn reindex_loom_block_search(
        &self,
        _ctx: &WriteContext,
        _workspace_id: &str,
        _block_id: &str,
        _search_text: &str,
        _embedding: Option<&[f32]>,
        _embedding_model: Option<&str>,
    ) -> StorageResult<()> {
        Err(StorageError::NotImplemented(
            "loom search v2 reindex backend",
        ))
    }

    /// WP-KERNEL-009 MT-264 LoomSearchV2: hybrid search fusing full-text rank,
    /// trigram similarity, and vector kNN, with snippet highlight, content_type
    /// facets, and loom_edges graph-blend ranking, resolved against the
    /// canonical loom block rows in one round trip.
    ///
    /// No implementor exists yet: the embedded SurrealDB store is the only
    /// database, and this surface is still pending the SurrealDB port
    /// (WP-KERNEL-012 MT-136). The default body fails closed with
    /// `NotImplemented`.
    async fn loom_search_v2(
        &self,
        _workspace_id: &str,
        _request: LoomSearchV2Request,
    ) -> StorageResult<LoomSearchV2Response> {
        Err(StorageError::NotImplemented("loom search v2 backend"))
    }
    async fn record_quick_switcher_recent(
        &self,
        _workspace_id: &str,
        _input: QuickSwitcherRecentInput,
    ) -> StorageResult<QuickSwitcherRecent> {
        Err(StorageError::NotImplemented(
            "quick switcher recents backend",
        ))
    }
    async fn list_quick_switcher_recents(
        &self,
        _workspace_id: &str,
        _limit: u32,
    ) -> StorageResult<Vec<QuickSwitcherRecent>> {
        Err(StorageError::NotImplemented(
            "quick switcher recents backend",
        ))
    }
    async fn get_workbench_layout_state(
        &self,
        _workspace_id: &str,
    ) -> StorageResult<Option<WorkbenchLayoutState>> {
        Err(StorageError::NotImplemented("workbench layout backend"))
    }
    async fn save_workbench_layout_state(
        &self,
        _workspace_id: &str,
        _input: WorkbenchLayoutStateInput,
    ) -> StorageResult<WorkbenchLayoutState> {
        Err(StorageError::NotImplemented("workbench layout backend"))
    }
    async fn get_workspace_settings_state(
        &self,
        _workspace_id: &str,
    ) -> StorageResult<Option<WorkspaceSettingsState>> {
        Err(StorageError::NotImplemented("workspace settings backend"))
    }
    async fn save_workspace_settings_state(
        &self,
        _workspace_id: &str,
        _input: WorkspaceSettingsStateInput,
    ) -> StorageResult<WorkspaceSettingsState> {
        Err(StorageError::NotImplemented("workspace settings backend"))
    }
    // ---- WP-KERNEL-012 MT-072 Settings & Preferences domain (Master Spec §10.17) ----
    // Canonical typed PreferenceRecord authority in SurrealDB. `entry` is the registry schema entry
    // (default, value_type, namespace, redaction). The value passed to `preference_set` MUST already be
    // typed-validated by the caller (`PreferenceSchemaEntry::validate`) — the store enforces authority,
    // EventLedger receipts, and revisions, not re-validation of user input semantics.
    /// Resolve the record for `entry` in `scope`; returns the registry-default record (revision 0) when
    /// unset (SET-REC-003 — never null for a defined preference).
    async fn preference_get(
        &self,
        _scope: &crate::preferences::PreferenceScope,
        _entry: &crate::preferences::PreferenceSchemaEntry,
    ) -> StorageResult<crate::preferences::PreferenceRecord> {
        Err(StorageError::NotImplemented("preference store"))
    }
    /// Set the value, bumping revision, emitting the EventLedger `PreferenceRecordChanged` receipt, and
    /// appending a recoverable change receipt in the same transaction (SET-EVT-001/002/003).
    async fn preference_set(
        &self,
        _scope: &crate::preferences::PreferenceScope,
        _entry: &crate::preferences::PreferenceSchemaEntry,
        _value: Value,
        _source: crate::preferences::PreferenceSource,
        _actor: &str,
    ) -> StorageResult<(
        crate::preferences::PreferenceRecord,
        crate::preferences::PreferenceChangeReceipt,
    )> {
        Err(StorageError::NotImplemented("preference store"))
    }
    /// Reset to the registry default, modeled as an explicit mutation with a receipt (SET-UI-002 — never
    /// a provenance-losing delete).
    async fn preference_reset(
        &self,
        _scope: &crate::preferences::PreferenceScope,
        _entry: &crate::preferences::PreferenceSchemaEntry,
        _actor: &str,
    ) -> StorageResult<(
        crate::preferences::PreferenceRecord,
        crate::preferences::PreferenceChangeReceipt,
    )> {
        Err(StorageError::NotImplemented("preference store"))
    }
    /// The change-history receipts for one preference in `scope`, newest first (SET-UI-003).
    async fn preference_history(
        &self,
        _scope: &crate::preferences::PreferenceScope,
        _preference_id: &str,
    ) -> StorageResult<Vec<crate::preferences::PreferenceChangeReceipt>> {
        Err(StorageError::NotImplemented("preference store"))
    }
    /// The deterministic redacted projection over the given registry entries in `scope` (SET-PROJ-002).
    async fn preference_projection(
        &self,
        _scope: &crate::preferences::PreferenceScope,
        _entries: &[crate::preferences::PreferenceSchemaEntry],
    ) -> StorageResult<Vec<crate::preferences::PreferenceProjectionRow>> {
        Err(StorageError::NotImplemented("preference store"))
    }
    /// MT-258 durable workspace search bookmarks (saved searches). Replaces the
    /// localStorage-only authority with durable store + EventLedger persistence.
    async fn get_workspace_search_bookmark_state(
        &self,
        _workspace_id: &str,
    ) -> StorageResult<Option<WorkspaceSearchBookmarkState>> {
        Err(StorageError::NotImplemented(
            "workspace search bookmark backend",
        ))
    }
    async fn save_workspace_search_bookmark_state(
        &self,
        _workspace_id: &str,
        _input: WorkspaceSearchBookmarkStateInput,
    ) -> StorageResult<WorkspaceSearchBookmarkState> {
        Err(StorageError::NotImplemented(
            "workspace search bookmark backend",
        ))
    }

    // -- MT-261 CanvasBoard ----------------------------------------------------
    /// Create a canvas board: a typed `LoomBlock(content_type=canvas)` plus its
    /// board-state (viewport) row. The block must already exist (created via
    /// `create_loom_block` + bridge). Idempotent on the block id (re-create
    /// returns the existing board / refreshes the viewport receipt).
    async fn create_canvas_board(
        &self,
        _ctx: &WriteContext,
        _workspace_id: &str,
        _block_id: &str,
        _board_state: Value,
    ) -> StorageResult<LoomCanvasBoard> {
        Err(StorageError::NotImplemented("loom canvas board backend"))
    }

    /// Read a canvas board projection: board + placements + visual-only edges.
    async fn get_canvas_board(
        &self,
        _workspace_id: &str,
        _block_id: &str,
    ) -> StorageResult<LoomCanvasBoardView> {
        Err(StorageError::NotImplemented("loom canvas board backend"))
    }

    /// Persist a new viewport (pan/zoom) only when the caller's EventLedger revision still matches,
    /// returning the new authoritative board revision.
    async fn update_canvas_board_state(
        &self,
        _ctx: &WriteContext,
        _workspace_id: &str,
        _block_id: &str,
        _board_state: Value,
        _expected_event_ledger_event_id: &str,
    ) -> StorageResult<LoomCanvasBoard> {
        Err(StorageError::NotImplemented("loom canvas board backend"))
    }

    /// Place an existing block on a canvas as a REFERENCE (never a copy).
    /// Validates same-workspace and that the placed block exists (FK only).
    async fn place_block_on_canvas(
        &self,
        _ctx: &WriteContext,
        _placement: NewLoomCanvasPlacement,
    ) -> StorageResult<LoomCanvasPlacement> {
        Err(StorageError::NotImplemented("loom canvas board backend"))
    }

    /// Atomically import the Stage reference as RichDocument/Loom authority and
    /// place it on a Canvas, serialized by its canonical provenance key.
    async fn create_stage_canvas_card(
        &self,
        _ctx: &WriteContext,
        _card: NewLoomCanvasStageCard,
    ) -> StorageResult<LoomCanvasStageCard> {
        Err(StorageError::NotImplemented(
            "loom Stage canvas card backend",
        ))
    }

    /// Remove only the complete authority tuple created by an exact Stage card
    /// request. The implementation is atomic, idempotent, and fail-closed on
    /// any identity/provenance mismatch.
    async fn compensate_stage_canvas_card(
        &self,
        _ctx: &WriteContext,
        _card: CompensateLoomCanvasStageCard,
    ) -> StorageResult<LoomCanvasStageCompensation> {
        Err(StorageError::NotImplemented(
            "loom Stage canvas card compensation backend",
        ))
    }

    /// Move / resize / (re)group a placement.
    async fn update_canvas_placement(
        &self,
        _ctx: &WriteContext,
        _workspace_id: &str,
        _placement_id: &str,
        _update: LoomCanvasPlacementUpdate,
    ) -> StorageResult<LoomCanvasPlacement> {
        Err(StorageError::NotImplemented("loom canvas board backend"))
    }

    /// Remove a placement and return the exact EventLedger receipt committed with that deletion.
    /// Deletes ONLY the placement row; the referenced LoomBlock is never touched.
    async fn remove_canvas_placement(
        &self,
        _ctx: &WriteContext,
        _workspace_id: &str,
        _placement_id: &str,
    ) -> StorageResult<LoomCanvasPlacementRemovalReceipt> {
        Err(StorageError::NotImplemented("loom canvas board backend"))
    }

    /// Add a visual-only (board-local) edge between two placements. This is NOT
    /// graph authority and never becomes a `loom_edge`.
    async fn add_canvas_visual_edge(
        &self,
        _ctx: &WriteContext,
        _workspace_id: &str,
        _canvas_block_id: &str,
        _from_placement_id: &str,
        _to_placement_id: &str,
        _label: Option<String>,
    ) -> StorageResult<LoomCanvasVisualEdge> {
        Err(StorageError::NotImplemented("loom canvas board backend"))
    }

    /// Delete a visual-only edge (board decoration only).
    async fn remove_canvas_visual_edge(
        &self,
        _ctx: &WriteContext,
        _workspace_id: &str,
        _visual_edge_id: &str,
    ) -> StorageResult<()> {
        Err(StorageError::NotImplemented("loom canvas board backend"))
    }

    // -- MT-262 BlockCollectionViews -------------------------------------------
    /// Create a saved block-collection view: a typed
    /// `LoomBlock(content_type='view_def')` whose query/columns/grouping are
    /// persisted in the dedicated `view_definition_json` column (NOT a
    /// derived_json overload). The block must already exist (created via
    /// `create_loom_block` + bridge); this sets its content_type + definition.
    /// Authority is the embedded SurrealDB store + EventLedger; there is NO
    /// parallel/localStorage store for view definitions.
    async fn create_block_view(
        &self,
        _ctx: &WriteContext,
        _workspace_id: &str,
        _block_id: &str,
        _title: Option<String>,
        _definition: BlockViewDefinition,
    ) -> StorageResult<BlockViewRecord> {
        Err(StorageError::NotImplemented(
            "loom block collection view backend",
        ))
    }

    /// Read a saved view block: the typed LoomBlock plus its decoded
    /// definition. Errors `NotFound` if the block does not exist or is not a
    /// `view_def` block.
    async fn get_block_view(
        &self,
        _workspace_id: &str,
        _block_id: &str,
    ) -> StorageResult<BlockViewRecord> {
        Err(StorageError::NotImplemented(
            "loom block collection view backend",
        ))
    }

    /// Replace a saved view's definition (e.g. a table header click re-sorting
    /// the view persists the new sort), leaving an updated_at bump. Authority is
    /// the embedded SurrealDB store; never localStorage.
    async fn update_block_view_definition(
        &self,
        _ctx: &WriteContext,
        _workspace_id: &str,
        _block_id: &str,
        _definition: BlockViewDefinition,
    ) -> StorageResult<BlockViewRecord> {
        Err(StorageError::NotImplemented(
            "loom block collection view backend",
        ))
    }

    /// Execute a saved view's query against the REAL Loom query backend. All
    /// filtering, the typed ORDER BY, and Kanban grouping run server-side in
    /// SQL (never client-side over a partial page). For Kanban (group_by) the
    /// result is split into lanes; for table/calendar it is a single sorted
    /// page.
    async fn query_block_view_results(
        &self,
        _workspace_id: &str,
        _definition: &BlockViewDefinition,
        _limit: u32,
        _offset: u32,
    ) -> StorageResult<BlockViewResults> {
        Err(StorageError::NotImplemented(
            "loom block collection view backend",
        ))
    }

    /// MT-254 DebugAdapterCore: list the durable breakpoints for a RichDocument.
    ///
    /// MT-157: the SurrealDB implementation is the product path; the route calls it inside the
    /// caller's record-user scope, so only rows the account may read (RichDocument read + fs.read)
    /// are returned. This default body is for backends without breakpoint storage.
    async fn list_debug_breakpoints(
        &self,
        _rich_document_id: &str,
    ) -> StorageResult<Vec<DebugBreakpoint>> {
        Err(StorageError::NotImplemented("debug breakpoints backend"))
    }

    /// MT-254 DebugAdapterCore: replace the full breakpoint set for a
    /// RichDocument (PUT semantics), each write leaving an EventLedger receipt.
    ///
    /// MT-157: under a record-user scope the write needs RichDocument update + fs.write, the
    /// receipt carries the session principal, and a write the table predicates silently dropped
    /// returns `StorageError::Guard("HSK-403-PROTECTED-RESOURCE")` with nothing changed.
    async fn set_debug_breakpoints(
        &self,
        _rich_document_id: &str,
        _workspace_id: &str,
        _breakpoints: Vec<DebugBreakpointInput>,
    ) -> StorageResult<Vec<DebugBreakpoint>> {
        Err(StorageError::NotImplemented("debug breakpoints backend"))
    }

    /// MT-191 LoomVisualDebugViews: bounded backend snapshot of graph,
    /// backlink, folder, and search state for one reproducible debug context.
    async fn loom_visual_debug_snapshot(
        &self,
        _workspace_id: &str,
        _start_block_id: &str,
        _query: &str,
        _limit: u32,
    ) -> StorageResult<LoomVisualDebugSnapshot> {
        Err(StorageError::NotImplemented("loom visual-debug backend"))
    }

    // -- MT-177 LoomBlockKnowledgeBridge ---------------------------------------
    /// The single authority backend for the Loom surface. WP-KERNEL-009
    /// §10.12 #9.1.1 forbids any SQLite/cache/offline/sidecar authority path;
    /// the only durable Loom authority is the embedded store + EventLedger.
    fn loom_authority_backend(&self) -> LoomAuthorityBackend {
        LoomAuthorityBackend::SurrealEventLedger
    }

    /// Bridge a `LoomBlock` to the ProjectKnowledgeIndex + EventLedger
    /// (MT-177). Upserts a `knowledge_entities` row (entity_kind=`loom_block`,
    /// entity_key=block_id), appends a `KNOWLEDGE_LOOM_BLOCK_INDEXED`
    /// EventLedger receipt, and upserts the `loom_block_knowledge_bridge` link.
    ///
    /// Idempotent: the knowledge entity's natural identity is
    /// (workspace, `loom_block`, block_id) and the bridge row is keyed on
    /// block_id, so re-bridging the same block updates in place and re-points
    /// to a fresh receipt rather than duplicating authority rows.
    async fn bridge_loom_block_to_knowledge(
        &self,
        _ctx: &WriteContext,
        _workspace_id: &str,
        _block_id: &str,
    ) -> StorageResult<LoomKnowledgeBridge> {
        Err(StorageError::NotImplemented(
            "loom block knowledge bridge backend",
        ))
    }

    /// Read the authority bridge for a LoomBlock, if it has been bridged.
    async fn get_loom_block_knowledge_bridge(
        &self,
        _workspace_id: &str,
        _block_id: &str,
    ) -> StorageResult<Option<LoomKnowledgeBridge>> {
        Err(StorageError::NotImplemented(
            "loom block knowledge bridge backend",
        ))
    }

    /// List every authority bridge in a workspace.
    async fn list_loom_block_knowledge_bridges(
        &self,
        _workspace_id: &str,
    ) -> StorageResult<Vec<LoomKnowledgeBridge>> {
        Err(StorageError::NotImplemented(
            "loom block knowledge bridge backend",
        ))
    }

    // -- MT-178 BacklinkComputation --------------------------------------------
    /// Linked backlinks for a block: every incoming edge (MENTION/TAG/...) with
    /// the referencing source block and a surrounding-text context snippet
    /// (§10.12 [LM-BACK-001]). Ordered oldest-first by edge creation.
    async fn get_backlinks_with_context(
        &self,
        _workspace_id: &str,
        _block_id: &str,
    ) -> StorageResult<Vec<LoomBacklink>> {
        Err(StorageError::NotImplemented("loom backlink backend"))
    }

    /// Unlinked mentions for a block (§10.12 [LM-BACK-002] / [LM-VIEW-002]):
    /// blocks whose searchable text contains the viewed block's title/aliases
    /// on a word boundary but which have NO formal MENTION/TAG edge to it, so
    /// one click converts them to edges. `aliases` may add extra terms to scan
    /// for beyond the block's own title. `limit` bounds the scan result set.
    async fn scan_unlinked_mentions(
        &self,
        _workspace_id: &str,
        _block_id: &str,
        _aliases: &[String],
        _limit: u32,
    ) -> StorageResult<Vec<LoomUnlinkedMention>> {
        Err(StorageError::NotImplemented(
            "loom unlinked mention backend",
        ))
    }

    // -- MT-179 LocalGraphApi --------------------------------------------------
    /// Local graph neighborhood around a block (§10.12 [LM-GRAPH-001]): an
    /// UNDIRECTED BFS over loom_edges to `max_depth`, filtered to `edge_types`
    /// (empty = all), returning nodes (with stale markers + ProjectKnowledge
    /// citations) and the edges among them. `node_limit` bounds the result and
    /// sets `truncated` when exceeded.
    async fn local_graph(
        &self,
        _workspace_id: &str,
        _start_block_id: &str,
        _max_depth: u32,
        _edge_types: &[LoomEdgeType],
        _node_limit: u32,
    ) -> StorageResult<LoomGraph> {
        Err(StorageError::NotImplemented("loom local graph backend"))
    }

    // -- MT-180 GlobalGraphApi -------------------------------------------------
    /// Project-level global graph (§10.12 [LM-GRAPH-002] / §7.1.4.3): all blocks
    /// + edges in the workspace, filtered to `edge_types` (empty = all), with a
    /// performance `node_limit` (sets `truncated`) and hub suppression — nodes
    /// whose total degree exceeds `hub_degree_threshold` are dropped from the
    /// returned graph and listed in `suppressed_hub_ids`.
    async fn global_graph(
        &self,
        _workspace_id: &str,
        _edge_types: &[LoomEdgeType],
        _node_limit: u32,
        _hub_degree_threshold: u32,
    ) -> StorageResult<LoomGraph> {
        Err(StorageError::NotImplemented("loom global graph backend"))
    }

    // -- MT-182 TagsAndTagHubs -------------------------------------------------
    /// List all tag-hub blocks in a workspace (content_type=tag_hub), newest
    /// first, paginated. The tag-as-first-class-block surface ([LM-TAG-002]).
    async fn list_tag_hubs(
        &self,
        _workspace_id: &str,
        _limit: u32,
        _offset: u32,
    ) -> StorageResult<Vec<LoomBlock>> {
        Err(StorageError::NotImplemented("loom tag hub backend"))
    }

    /// The tag-hub surface for one tag block: the block, its direct sub-tags
    /// (SUB_TAG children), the blocks tagged with it (direct TAG sources), and
    /// its backlink count. Fails closed if the block is not a tag_hub.
    async fn get_tag_hub(
        &self,
        _workspace_id: &str,
        _tag_block_id: &str,
    ) -> StorageResult<LoomTagHub> {
        Err(StorageError::NotImplemented("loom tag hub backend"))
    }

    /// The blocks tagged with a tag (search filter by tag, [LM-TAG-001]). When
    /// `include_subtags` is true, blocks tagged with any DESCENDANT sub-tag are
    /// included too (nested-tag membership, [LM-TAG-003]). Paginated.
    async fn list_blocks_for_tag(
        &self,
        _workspace_id: &str,
        _tag_block_id: &str,
        _include_subtags: bool,
        _limit: u32,
        _offset: u32,
    ) -> StorageResult<Vec<LoomBlock>> {
        Err(StorageError::NotImplemented("loom tag hub backend"))
    }

    // -- MT-183 PinsFavoritesAndUnlinked ---------------------------------------
    /// Set or clear a block's Pins-grid ordinal ([LM-VIEW-004] reorderable
    /// grid). `pin_order = None` clears the ordinal (the dedicated reorder path,
    /// which the COALESCE-based patch cannot express). Returns the updated block.
    async fn set_loom_block_pin_order(
        &self,
        _ctx: &WriteContext,
        _workspace_id: &str,
        _block_id: &str,
        _pin_order: Option<i32>,
    ) -> StorageResult<LoomBlock> {
        Err(StorageError::NotImplemented("loom pin order backend"))
    }

    /// WP-KERNEL-012 MT-024 FAIL_V2: atomically remove a pin — clear the pin_order
    /// ordinal AND unpin the block in ONE transaction alongside the durable
    /// EventLedger receipt. Collapses the old two-call remove flow (PUT
    /// /pin-order(null) THEN PATCH {pinned:false}) so a pin removal can never
    /// leave partial persisted state. Returns the updated block and exact committed EventLedger row.
    async fn remove_loom_block_pin(
        &self,
        _ctx: &WriteContext,
        _workspace_id: &str,
        _block_id: &str,
    ) -> StorageResult<LoomBlockMutationReceipt> {
        Err(StorageError::NotImplemented("loom pin removal backend"))
    }

    // -- MT-181 FolderTreeAndColorLabels ---------------------------------------
    /// Create a Loom folder (optionally nested under a parent). The parent, when
    /// given, must exist in the workspace; sibling names are unique.
    async fn create_loom_folder(
        &self,
        _workspace_id: &str,
        _folder: NewLoomFolder,
    ) -> StorageResult<LoomFolder> {
        Err(StorageError::NotImplemented("loom folder backend"))
    }

    /// Read one folder by id.
    async fn get_loom_folder(
        &self,
        _workspace_id: &str,
        _folder_id: &str,
    ) -> StorageResult<LoomFolder> {
        Err(StorageError::NotImplemented("loom folder backend"))
    }

    /// List every folder in a workspace (the full tree, parent-before-child
    /// order). The caller builds the nesting from parent_folder_id.
    async fn list_loom_folders(&self, _workspace_id: &str) -> StorageResult<Vec<LoomFolder>> {
        Err(StorageError::NotImplemented("loom folder backend"))
    }

    /// Update a folder: rename, recolor, change sort mode/order, or move it
    /// (re-parent). Moving into the folder's own subtree fails closed (cycle).
    async fn update_loom_folder(
        &self,
        _workspace_id: &str,
        _folder_id: &str,
        _update: LoomFolderUpdate,
    ) -> StorageResult<LoomFolder> {
        Err(StorageError::NotImplemented("loom folder backend"))
    }

    /// Delete a folder and its subtree (membership rows cascade; the LoomBlocks
    /// themselves are untouched — folders are an overlay, not ownership).
    async fn delete_loom_folder(&self, _workspace_id: &str, _folder_id: &str) -> StorageResult<()> {
        Err(StorageError::NotImplemented("loom folder backend"))
    }

    /// Add a block to a folder (idempotent). Optional manual ordinal.
    async fn add_block_to_loom_folder(
        &self,
        _workspace_id: &str,
        _folder_id: &str,
        _block_id: &str,
        _sort_order: Option<i32>,
    ) -> StorageResult<()> {
        Err(StorageError::NotImplemented("loom folder backend"))
    }

    /// Remove a block from a folder (idempotent).
    async fn remove_block_from_loom_folder(
        &self,
        _workspace_id: &str,
        _folder_id: &str,
        _block_id: &str,
    ) -> StorageResult<()> {
        Err(StorageError::NotImplemented("loom folder backend"))
    }

    /// List the blocks in a folder, ordered by the folder's sort_mode
    /// (name/created/updated/manual). Paginated.
    async fn list_loom_folder_blocks(
        &self,
        _workspace_id: &str,
        _folder_id: &str,
        _limit: u32,
        _offset: u32,
    ) -> StorageResult<Vec<LoomBlock>> {
        Err(StorageError::NotImplemented("loom folder backend"))
    }

    // -- MT-184 WikiPageProjectionCompiler -------------------------------------
    /// Compile (or recompile) a Loom wiki/topic page projection from a set of
    /// LoomBlocks. Renders deterministic wiki markdown with per-block citations,
    /// stamps a staleness hash over the source blocks' identity+content, and
    /// stores it as a NEVER-authority projection (§10.12 §9.1.1). Returns the
    /// compiled projection.
    async fn compile_loom_wiki_projection(
        &self,
        _workspace_id: &str,
        _title: &str,
        _block_ids: &[String],
    ) -> StorageResult<LoomWikiProjection> {
        Err(StorageError::NotImplemented("loom wiki projection backend"))
    }

    /// Read a Loom wiki projection by id.
    async fn get_loom_wiki_projection(
        &self,
        _workspace_id: &str,
        _projection_id: &str,
    ) -> StorageResult<LoomWikiProjection> {
        Err(StorageError::NotImplemented("loom wiki projection backend"))
    }

    /// True when the projection's stored staleness hash no longer matches a
    /// freshly computed hash over its source blocks (authority drifted).
    async fn loom_wiki_projection_is_stale(
        &self,
        _workspace_id: &str,
        _projection_id: &str,
    ) -> StorageResult<bool> {
        Err(StorageError::NotImplemented("loom wiki projection backend"))
    }

    /// Regenerate a projection's content from current authority and clear its
    /// stale marker. Returns the refreshed projection.
    async fn regenerate_loom_wiki_projection(
        &self,
        _workspace_id: &str,
        _projection_id: &str,
    ) -> StorageResult<LoomWikiProjection> {
        Err(StorageError::NotImplemented("loom wiki projection backend"))
    }

    /// Delete a wiki projection. Projections are regenerable; deleting one MUST
    /// NOT mutate any authority record (the projection-never-authority law).
    async fn delete_loom_wiki_projection(
        &self,
        _workspace_id: &str,
        _projection_id: &str,
    ) -> StorageResult<()> {
        Err(StorageError::NotImplemented("loom wiki projection backend"))
    }

    // -- MT-185 WikiPageEditableOverlay ----------------------------------------
    /// Add an operator annotation overlay to a wiki projection. The overlay is
    /// its OWN authority row; the projection it annotates stays non-authority
    /// and regenerable.
    async fn add_loom_wiki_overlay(
        &self,
        _workspace_id: &str,
        _projection_id: &str,
        _annotation: &str,
        _anchor: Option<&str>,
    ) -> StorageResult<LoomWikiOverlay> {
        Err(StorageError::NotImplemented("loom wiki overlay backend"))
    }

    /// List the operator annotation overlays on a projection.
    async fn list_loom_wiki_overlays(
        &self,
        _workspace_id: &str,
        _projection_id: &str,
    ) -> StorageResult<Vec<LoomWikiOverlay>> {
        Err(StorageError::NotImplemented("loom wiki overlay backend"))
    }

    /// Delete an operator annotation overlay (does not touch the projection).
    async fn delete_loom_wiki_overlay(
        &self,
        _workspace_id: &str,
        _overlay_id: &str,
    ) -> StorageResult<()> {
        Err(StorageError::NotImplemented("loom wiki overlay backend"))
    }

    // -- MT-187 ObsidianImportBoundary -----------------------------------------
    /// Import a markdown-like note into Loom authority: parse it to a block
    /// tree, create a RichDocument (authority) + a LoomBlock (note) bridged to
    /// the ProjectKnowledgeIndex, and return the new LoomBlock plus any import
    /// warnings. The markdown SOURCE is NEVER recorded as authority — only the
    /// parsed authority records in the durable store (MT-187). A vault/file layout cannot
    /// become the source of truth.
    async fn import_markdown_to_loom(
        &self,
        _ctx: &WriteContext,
        _workspace_id: &str,
        _title: &str,
        _markdown: &str,
    ) -> StorageResult<LoomMarkdownImport> {
        Err(StorageError::NotImplemented("loom markdown import backend"))
    }

    // -- MT-188 NavigationBreadcrumbs ------------------------------------------
    /// Build the breadcrumb trail for a LoomBlock across the entity spine:
    /// workspace -> (project) -> folder ancestry -> block -> ProjectKnowledge
    /// entity. Root-first. When the block lives in multiple folders, the first
    /// folder branch (deterministic order) is used. Fails closed if the block
    /// does not exist.
    async fn loom_block_breadcrumbs(
        &self,
        _workspace_id: &str,
        _block_id: &str,
    ) -> StorageResult<LoomBreadcrumbTrail> {
        Err(StorageError::NotImplemented("loom breadcrumb backend"))
    }

    // Calendar storage (WP-1-Calendar-Storage-v1)
    async fn upsert_calendar_source(
        &self,
        ctx: &WriteContext,
        source: CalendarSourceUpsert,
    ) -> StorageResult<CalendarSource>;
    async fn list_calendar_sources(&self, workspace_id: &str)
        -> StorageResult<Vec<CalendarSource>>;
    async fn get_calendar_source(
        &self,
        workspace_id: &str,
        source_id: &str,
    ) -> StorageResult<Option<CalendarSource>>;
    async fn upsert_calendar_event(
        &self,
        ctx: &WriteContext,
        event: CalendarEventUpsert,
    ) -> StorageResult<CalendarEvent>;
    async fn query_calendar_events(
        &self,
        query: CalendarEventWindowQuery,
    ) -> StorageResult<Vec<CalendarEvent>>;
    async fn delete_calendar_data_by_source(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        source_id: &str,
    ) -> StorageResult<()>;

    // Canvas operations
    async fn create_canvas(&self, ctx: &WriteContext, canvas: NewCanvas) -> StorageResult<Canvas>;
    async fn list_canvases(&self, workspace_id: &str) -> StorageResult<Vec<Canvas>>;
    async fn get_canvas_with_graph(&self, canvas_id: &str) -> StorageResult<CanvasGraph>;
    /// Rename a canvas, optionally guarded by the caller's last-seen `updated_at`. A stale timestamp
    /// returns [`StorageError::Conflict`] instead of overwriting a concurrent edit.
    async fn rename_canvas(
        &self,
        ctx: &WriteContext,
        canvas_id: &str,
        title: &str,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> StorageResult<Canvas>;
    async fn update_canvas_graph(
        &self,
        ctx: &WriteContext,
        canvas_id: &str,
        nodes: Vec<NewCanvasNode>,
        edges: Vec<NewCanvasEdge>,
    ) -> StorageResult<CanvasGraph>;
    async fn delete_canvas(&self, ctx: &WriteContext, canvas_id: &str) -> StorageResult<()>;

    // AI-Ready Data Architecture (Â§2.3.14)
    async fn create_ai_bronze_record(
        &self,
        ctx: &WriteContext,
        record: NewBronzeRecord,
    ) -> StorageResult<BronzeRecord>;
    async fn get_ai_bronze_record(&self, bronze_id: &str) -> StorageResult<Option<BronzeRecord>>;
    async fn list_ai_bronze_records(&self, workspace_id: &str) -> StorageResult<Vec<BronzeRecord>>;
    async fn mark_ai_bronze_deleted(
        &self,
        ctx: &WriteContext,
        bronze_id: &str,
    ) -> StorageResult<()>;

    async fn create_ai_silver_record(
        &self,
        ctx: &WriteContext,
        record: NewSilverRecord,
    ) -> StorageResult<SilverRecord>;
    async fn get_ai_silver_record(&self, silver_id: &str) -> StorageResult<Option<SilverRecord>>;
    async fn list_ai_silver_records_by_bronze(
        &self,
        bronze_id: &str,
    ) -> StorageResult<Vec<SilverRecord>>;
    async fn list_ai_silver_records(&self, workspace_id: &str) -> StorageResult<Vec<SilverRecord>>;
    async fn supersede_ai_silver_record(
        &self,
        ctx: &WriteContext,
        superseded_silver_id: &str,
        new_silver_id: &str,
    ) -> StorageResult<()>;

    async fn upsert_ai_embedding_model(
        &self,
        ctx: &WriteContext,
        model: EmbeddingModelRecord,
    ) -> StorageResult<()>;
    async fn list_ai_embedding_models(&self) -> StorageResult<Vec<EmbeddingModelRecord>>;
    async fn set_ai_embedding_default_model(
        &self,
        ctx: &WriteContext,
        model_id: &str,
        model_version: &str,
    ) -> StorageResult<()>;
    async fn get_ai_embedding_registry(&self) -> StorageResult<Option<EmbeddingRegistry>>;

    // AI Job operations (CX-DBP-021)
    async fn get_ai_job(&self, job_id: &str) -> StorageResult<AiJob>;
    async fn list_ai_jobs(&self, filter: AiJobListFilter) -> StorageResult<Vec<AiJob>>;
    async fn create_ai_job(&self, job: NewAiJob) -> StorageResult<AiJob>;
    async fn update_ai_job_status(&self, update: JobStatusUpdate) -> StorageResult<AiJob>;
    async fn set_job_outputs(&self, job_id: &str, outputs: Option<Value>) -> StorageResult<()>;
    async fn upsert_model_session(&self, session: NewModelSession) -> StorageResult<ModelSession>;
    async fn get_model_session(&self, session_id: &str) -> StorageResult<ModelSession>;
    async fn get_model_session_by_job_id(&self, job_id: Uuid) -> StorageResult<ModelSession>;
    async fn update_model_session_state(
        &self,
        session_id: &str,
        state: ModelSessionState,
        job_id: Option<Uuid>,
    ) -> StorageResult<ModelSession> {
        self.update_model_session_state_with_merge_back_artifact(session_id, state, job_id, None)
            .await
    }

    async fn update_model_session_state_with_merge_back_artifact(
        &self,
        session_id: &str,
        state: ModelSessionState,
        job_id: Option<Uuid>,
        merge_back_artifact: Option<MergeBackArtifact>,
    ) -> StorageResult<ModelSession>;
    /// MT-142: close a durable model session with close metadata.
    ///
    /// `state` must be terminal ([`ModelSessionState::is_terminal`]);
    /// `close_reason` and `actor` must be non-empty. Sets `close_reason`,
    /// `closed_by_actor` and `closed_at` on the durable row so the close
    /// metadata survives restart.
    async fn close_model_session(
        &self,
        session_id: &str,
        state: ModelSessionState,
        close_reason: &str,
        actor: &str,
    ) -> StorageResult<ModelSession>;
    async fn create_session_checkpoint(
        &self,
        checkpoint: SessionCheckpoint,
    ) -> StorageResult<SessionCheckpoint>;
    async fn get_latest_session_checkpoint(
        &self,
        session_id: &str,
    ) -> StorageResult<SessionCheckpoint>;
    async fn append_session_message(
        &self,
        message: NewSessionMessage,
    ) -> StorageResult<SessionMessage>;
    async fn list_session_messages(&self, session_id: &str) -> StorageResult<Vec<SessionMessage>>;
    async fn append_kernel_event(&self, event: NewKernelEvent) -> StorageResult<KernelEvent>;
    async fn append_kernel_events_atomic(
        &self,
        events: Vec<NewKernelEvent>,
    ) -> StorageResult<Vec<KernelEvent>>;
    async fn append_kernel_event_pair_atomic_with_causation(
        &self,
        first: NewKernelEvent,
        second: NewKernelEvent,
    ) -> StorageResult<Vec<KernelEvent>>;
    /// WP-KERNEL-009 authority-hardening #2: promote a graph proposal into a
    /// durable `knowledge_crdt_promoted_facts` row in ONE transaction with the
    /// causation-linked PROMOTION_REQUESTED/PROMOTION_ACCEPTED event pair and
    /// the proposal's `approved -> promoted` state flip. Either the ledger
    /// pair AND the fact AND the flip all commit, or none do — closing the
    /// crash window where the ledger said "promoted" but no fact row existed
    /// (passive replay then never converged). Idempotent on `proposal_id`:
    /// a re-promotion returns the existing fact untouched.
    async fn promote_graph_fact_atomic(
        &self,
        requested: NewKernelEvent,
        accepted: NewKernelEvent,
        fact: knowledge_crdt::NewPromotedFact,
    ) -> StorageResult<knowledge_crdt::PromotedFactRow>;
    async fn list_kernel_events_for_session(
        &self,
        session_run_id: &str,
    ) -> StorageResult<Vec<KernelEvent>>;
    async fn list_kernel_events_for_aggregate(
        &self,
        aggregate_type: &str,
        aggregate_id: &str,
    ) -> StorageResult<Vec<KernelEvent>>;
    /// Return the bounded durable native-editor mirror backlog in ledger order. A pending row is
    /// excluded once the same aggregate has a completion receipt, so the periodic reconciler never
    /// rescans all historical native-editor events after every backend restart.
    async fn list_pending_native_editor_mirrors(
        &self,
        after_event_sequence: i64,
        limit: i64,
    ) -> StorageResult<Vec<KernelEvent>>;
    async fn append_kernel_crdt_update(
        &self,
        record: CrdtUpdateRecordV1,
        update_bytes: Vec<u8>,
    ) -> StorageResult<CrdtUpdateRecordV1>;
    async fn list_kernel_crdt_updates(
        &self,
        workspace_id: &str,
        document_id: &str,
        crdt_document_id: &str,
    ) -> StorageResult<Vec<CrdtUpdateRecordV1>>;
    async fn read_kernel_crdt_update_bytes(&self, update_bytes_ref: &str)
        -> StorageResult<Vec<u8>>;
    async fn append_kernel_crdt_snapshot(
        &self,
        record: CrdtSnapshotRecordV1,
        snapshot_bytes: Vec<u8>,
    ) -> StorageResult<CrdtSnapshotRecordV1>;
    async fn list_kernel_crdt_snapshots(
        &self,
        workspace_id: &str,
        document_id: &str,
        crdt_document_id: &str,
    ) -> StorageResult<Vec<CrdtSnapshotRecordV1>>;
    async fn read_kernel_crdt_snapshot_bytes(
        &self,
        snapshot_bytes_ref: &str,
    ) -> StorageResult<Vec<u8>>;
    async fn enqueue_kernel_session_run(&self, session: SessionRun) -> StorageResult<SessionRun>;
    async fn enqueue_kernel_session_run_and_record_event(
        &self,
        session: SessionRun,
        causation_id: Option<String>,
        correlation_id: String,
    ) -> StorageResult<(SessionRun, KernelEvent)>;
    async fn claim_kernel_session_run(
        &self,
        session_run_id: &str,
        claimed_by: &str,
        lease_seconds: i64,
    ) -> StorageResult<Option<KernelSessionLease>>;
    async fn claim_kernel_session_run_and_record_event(
        &self,
        session_run_id: &str,
        claimed_by: &str,
        lease_seconds: i64,
        causation_id: Option<String>,
        correlation_id: String,
    ) -> StorageResult<Option<(KernelSessionLease, KernelEvent)>>;
    async fn update_kernel_session_run_state(
        &self,
        session_run_id: &str,
        state: SessionRunState,
    ) -> StorageResult<KernelSessionLease>;
    async fn update_kernel_session_run_state_and_record_event(
        &self,
        session_run_id: &str,
        state: SessionRunState,
        causation_id: Option<String>,
        correlation_id: String,
    ) -> StorageResult<(KernelSessionLease, KernelEvent)>;

    async fn update_ai_job_mcp_fields(
        &self,
        job_id: Uuid,
        update: AiJobMcpUpdate,
    ) -> StorageResult<()> {
        let _ = (job_id, update);
        Err(StorageError::NotImplemented("update_ai_job_mcp_fields"))
    }

    async fn get_ai_job_mcp_fields(&self, job_id: Uuid) -> StorageResult<AiJobMcpFields> {
        let _ = job_id;
        Err(StorageError::NotImplemented("get_ai_job_mcp_fields"))
    }

    async fn find_ai_job_id_by_mcp_progress_token(
        &self,
        progress_token: &str,
    ) -> StorageResult<Option<Uuid>> {
        let _ = progress_token;
        Err(StorageError::NotImplemented(
            "find_ai_job_id_by_mcp_progress_token",
        ))
    }

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

    async fn create_governance_check_run(
        &self,
        ctx: &WriteContext,
        run: NewGovernanceCheckRun,
    ) -> StorageResult<GovernanceCheckRun> {
        let _ = (ctx, run);
        Err(StorageError::NotImplemented("create_governance_check_run"))
    }
    async fn list_governance_check_runs(
        &self,
        session_id: Uuid,
    ) -> StorageResult<Vec<GovernanceCheckRun>> {
        let _ = session_id;
        Err(StorageError::NotImplemented("list_governance_check_runs"))
    }

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

    /// Run database migrations.
    async fn run_migrations(&self) -> StorageResult<()>;

    /// Returns the current embedded SurrealDB schema revision.
    async fn migration_version(&self) -> StorageResult<i64>;

    async fn execute_locus_operation(
        &self,
        op: crate::workflows::locus::types::LocusOperation,
    ) -> StorageResult<Value> {
        let _ = op;
        Err(StorageError::NotImplemented("locus runtime"))
    }

    async fn locus_task_board_update_work_packet(
        &self,
        expected_version: i64,
        status: &str,
        task_board_status: &str,
        updated_at: &str,
        metadata: &str,
        wp_id: &str,
    ) -> StorageResult<()> {
        let _ = (
            expected_version,
            status,
            task_board_status,
            updated_at,
            metadata,
            wp_id,
        );
        Err(StorageError::NotImplemented("locus runtime"))
    }

    async fn structured_collab_work_packet_row(
        &self,
        wp_id: &str,
    ) -> StorageResult<Option<StructuredCollabWorkPacketRow>> {
        let _ = wp_id;
        Err(StorageError::NotImplemented(
            "structured collaboration artifacts",
        ))
    }

    async fn structured_collab_work_packet_rows(
        &self,
    ) -> StorageResult<Vec<StructuredCollabWorkPacketRow>> {
        Err(StorageError::NotImplemented(
            "structured collaboration artifacts",
        ))
    }

    async fn structured_collab_micro_task_metadata(
        &self,
        wp_id: &str,
        mt_id: &str,
    ) -> StorageResult<Option<String>> {
        let _ = (wp_id, mt_id);
        Err(StorageError::NotImplemented(
            "structured collaboration artifacts",
        ))
    }

    async fn structured_collab_micro_task_status_rows(
        &self,
        wp_id: &str,
    ) -> StorageResult<Vec<(String, String)>> {
        let _ = wp_id;
        Err(StorageError::NotImplemented(
            "structured collaboration artifacts",
        ))
    }

    async fn structured_collab_micro_task_rows(
        &self,
        wp_id: &str,
    ) -> StorageResult<Vec<(String, String)>> {
        let _ = wp_id;
        Err(StorageError::NotImplemented(
            "structured collaboration artifacts",
        ))
    }

    #[cfg(any(test, feature = "surreal-test-support"))]
    async fn test_overwrite_loom_block_metrics(
        &self,
        workspace_id: &str,
        block_id: &str,
        mention_count: i64,
        tag_count: i64,
        backlink_count: i64,
    ) -> StorageResult<()> {
        let _ = (
            workspace_id,
            block_id,
            mention_count,
            tag_count,
            backlink_count,
        );
        Err(StorageError::NotImplemented("test loom metrics backend"))
    }

    #[cfg(any(test, feature = "surreal-test-support"))]
    async fn test_zero_workspace_loom_metrics(&self, workspace_id: &str) -> StorageResult<()> {
        let _ = workspace_id;
        Err(StorageError::NotImplemented("test loom metrics backend"))
    }

    #[cfg(any(test, feature = "test-utils", feature = "surreal-test-support"))]
    async fn test_insert_loom_traversal_perf_fixture(
        &self,
        workspace_id: &str,
        total_blocks: usize,
    ) -> StorageResult<String> {
        let _ = (workspace_id, total_blocks);
        Err(StorageError::NotImplemented(
            "loom traversal performance backend",
        ))
    }

    #[cfg(any(test, feature = "surreal-test-support"))]
    async fn test_update_ai_job_metadata(
        &self,
        job_id: Uuid,
        status: &str,
        created_at: DateTime<Utc>,
        is_pinned: bool,
    ) -> StorageResult<()> {
        let _ = (job_id, status, created_at, is_pinned);
        Err(StorageError::NotImplemented("test ai job metadata backend"))
    }

    #[cfg(any(test, feature = "surreal-test-support"))]
    async fn test_fetch_mutation_traceability_row(
        &self,
        table: &str,
        id: &str,
    ) -> StorageResult<MutationTraceabilityRow> {
        let _ = (table, id);
        Err(StorageError::NotImplemented(
            "test mutation traceability backend",
        ))
    }
}

impl<T> StorageCapabilityStore for T
where
    T: Database + ?Sized,
{
    fn storage_capabilities(&self) -> StorageCapabilitySnapshot {
        // SurrealDB is the only backend kind, so this is not a choice. The
        // match this replaced branched on the observability tier and the
        // graph-filtering flag but returned the SAME value from all three arms,
        // which made it read like a capability decision while being a constant.
        // Reporting the constant directly keeps the snapshot honest; the two
        // capability flags below are the fields that actually vary.
        let backend = StorageBackendKind::Surreal;

        StorageCapabilitySnapshot {
            backend,
            supports_structured_collab_artifacts: self.supports_structured_collab_artifacts(),
            supports_loom_graph_filtering: self.supports_loom_graph_filtering(),
        }
    }
}

#[async_trait]
impl<T> StructuredCollaborationStore for T
where
    T: Database + ?Sized,
{
    async fn locus_work_packet_exists(&self, wp_id: &str) -> StorageResult<bool> {
        let row = Database::structured_collab_work_packet_row(self, wp_id).await?;
        Ok(row.is_some())
    }

    async fn execute_locus_operation(
        &self,
        op: crate::workflows::locus::types::LocusOperation,
    ) -> StorageResult<Value> {
        Database::execute_locus_operation(self, op).await
    }

    async fn locus_task_board_get_status_and_metadata(
        &self,
        wp_id: &str,
    ) -> StorageResult<Option<(i64, String, String)>> {
        let row = Database::structured_collab_work_packet_row(self, wp_id).await?;
        Ok(row.map(|row| (row.version, row.task_board_status, row.metadata)))
    }

    async fn locus_task_board_update_work_packet(
        &self,
        expected_version: i64,
        status: &str,
        task_board_status: &str,
        updated_at: &str,
        metadata: &str,
        wp_id: &str,
    ) -> StorageResult<()> {
        Database::locus_task_board_update_work_packet(
            self,
            expected_version,
            status,
            task_board_status,
            updated_at,
            metadata,
            wp_id,
        )
        .await
    }

    async fn locus_task_board_list_rows(&self) -> StorageResult<Vec<(String, String, String)>> {
        let rows = Database::structured_collab_work_packet_rows(self).await?;
        Ok(rows
            .into_iter()
            .map(|row| (row.wp_id, row.task_board_status, row.metadata))
            .collect())
    }

    async fn structured_collab_work_packet_row(
        &self,
        wp_id: &str,
    ) -> StorageResult<Option<StructuredCollabWorkPacketRow>> {
        Database::structured_collab_work_packet_row(self, wp_id).await
    }

    async fn structured_collab_work_packet_rows(
        &self,
    ) -> StorageResult<Vec<StructuredCollabWorkPacketRow>> {
        Database::structured_collab_work_packet_rows(self).await
    }

    async fn structured_collab_micro_task_status_rows(
        &self,
        wp_id: &str,
    ) -> StorageResult<Vec<(String, String)>> {
        Database::structured_collab_micro_task_status_rows(self, wp_id).await
    }

    async fn structured_collab_micro_task_metadata(
        &self,
        wp_id: &str,
        mt_id: &str,
    ) -> StorageResult<Option<String>> {
        Database::structured_collab_micro_task_metadata(self, wp_id, mt_id).await
    }

    async fn structured_collab_micro_task_rows(
        &self,
        wp_id: &str,
    ) -> StorageResult<Vec<(String, String)>> {
        Database::structured_collab_micro_task_rows(self, wp_id).await
    }
}

use std::sync::Arc;

/// [CX-DBP-041] Initialize the storage backend based on environment configuration.
/// Resolves the explicit control-plane storage mode before opening the backend.
pub async fn init_storage() -> Result<Arc<dyn Database>, StorageError> {
    let config = ControlPlaneStorageConfig::from_env()?;
    init_storage_with_config(&config).await
}

#[derive(Clone)]
pub struct ControlPlaneStorage {
    pub database: Arc<dyn Database>,
    /// The embedded store itself, kept so the process that opened it can also
    /// close it. SurrealDB runs in-process, so its lifetime is Handshake's
    /// lifetime: there is no external service that outlives the binary and no
    /// operator step to stop one [CX-503R/CX-503S].
    pub surreal: surreal::SurrealStorage,
}

pub async fn init_control_plane_storage() -> Result<ControlPlaneStorage, StorageError> {
    let config = ControlPlaneStorageConfig::from_env()?;
    init_control_plane_storage_with_config(&config).await
}

pub async fn init_control_plane_storage_with_config(
    config: &ControlPlaneStorageConfig,
) -> Result<ControlPlaneStorage, StorageError> {
    match config.mode {
        ControlPlaneStorageMode::SurrealEmbedded => {
            // The embedded store is opened and owned by this process: no
            // operator-launched database, external service, or container
            // runtime is part of core operation [CX-503R/CX-503S].
            let surreal_config = config.surreal_config()?;
            let storage = surreal::SurrealStorage::open(surreal_config)
                .await
                .map_err(|err| StorageError::Database(err.to_string()))?;
            let db = surreal::SurrealDatabase::new(storage.clone());
            db.run_migrations().await?;
            Ok(ControlPlaneStorage {
                database: Arc::new(db),
                surreal: storage,
            })
        }
    }
}

/// Configured concurrency budget for bounded embedded-SurrealDB operations.
///
/// This controls application work admitted concurrently; it is not a database
/// connection-pool size. Invalid or out-of-range values fail closed to the
/// conservative default instead of reviving a legacy PostgreSQL setting.
pub fn configured_surreal_operation_concurrency() -> usize {
    std::env::var("HANDSHAKE_SURREAL_OPERATION_CONCURRENCY")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| (1..=32).contains(value))
        .unwrap_or(4)
}

pub async fn init_storage_with_config(
    config: &ControlPlaneStorageConfig,
) -> Result<Arc<dyn Database>, StorageError> {
    Ok(init_control_plane_storage_with_config(config)
        .await?
        .database)
}
