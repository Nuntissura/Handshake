use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub type Iso8601Timestamp = DateTime<Utc>;
pub type VectorClock = Value;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProjectProfileKind {
    SoftwareDelivery,
    Research,
    Worldbuilding,
    Design,
    #[default]
    Generic,
    Custom,
}

impl ProjectProfileKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SoftwareDelivery => "software_delivery",
            Self::Research => "research",
            Self::Worldbuilding => "worldbuilding",
            Self::Design => "design",
            Self::Generic => "generic",
            Self::Custom => "custom",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "software_delivery" => Some(Self::SoftwareDelivery),
            "research" => Some(Self::Research),
            "worldbuilding" => Some(Self::Worldbuilding),
            "design" => Some(Self::Design),
            "generic" => Some(Self::Generic),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum MirrorSyncState {
    #[default]
    CanonicalOnly,
    Synchronized,
    Stale,
    AdvisoryEdit,
    NormalizationRequired,
}

impl MirrorSyncState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::CanonicalOnly => "canonical_only",
            Self::Synchronized => "synchronized",
            Self::Stale => "stale",
            Self::AdvisoryEdit => "advisory_edit",
            Self::NormalizationRequired => "normalization_required",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "canonical_only" => Some(Self::CanonicalOnly),
            "synchronized" => Some(Self::Synchronized),
            "stale" => Some(Self::Stale),
            "advisory_edit" => Some(Self::AdvisoryEdit),
            "normalization_required" => Some(Self::NormalizationRequired),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MarkdownAuthorityMode {
    DerivedReadonly,
    AdvisoryEditable,
    NotesSidecarOnly,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MirrorReconciliationAction {
    None,
    RegenerateMirror,
    PromoteAdvisoryNote,
    ManualResolutionRequired,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MarkdownMirrorContractV1 {
    pub authority_mode: MarkdownAuthorityMode,
    pub markdown_mirror_path: String,
    pub template_id: String,
    pub canonical_content_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mirror_content_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_reconciled_at: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub manual_edit_zones: Vec<String>,
    pub reconciliation_action: MirrorReconciliationAction,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStateFamily {
    Intake,
    Ready,
    Active,
    Waiting,
    Review,
    Approval,
    Validation,
    Blocked,
    Done,
    Canceled,
    Archived,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowQueueReasonCode {
    NewUntriaged,
    DependencyWait,
    ReadyForLocalSmallModel,
    ReadyForCloudModel,
    ReadyForHuman,
    ReviewWait,
    ApprovalWait,
    ValidationWait,
    MailboxResponseWait,
    TimerWait,
    BlockedMissingContext,
    BlockedPolicy,
    BlockedCapability,
    BlockedError,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GovernedActionDescriptorV1 {
    pub action_id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StructuredCollaborationSummaryV1 {
    pub schema_id: String,
    pub schema_version: String,
    pub record_id: String,
    pub record_kind: String,
    pub project_profile_kind: ProjectProfileKind,
    pub updated_at: String,
    pub mirror_state: MirrorSyncState,
    #[serde(default)]
    pub authority_refs: Vec<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mirror_contract: Option<MarkdownMirrorContractV1>,
    pub workflow_state_family: WorkflowStateFamily,
    pub queue_reason_code: WorkflowQueueReasonCode,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_action_ids: Vec<String>,
    pub status: String,
    pub title_or_objective: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blockers: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_action: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrackedWorkPacketArtifactV1 {
    pub schema_id: String,
    pub schema_version: String,
    pub record_id: String,
    pub record_kind: String,
    pub project_profile_kind: ProjectProfileKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_extension: Option<Value>,
    pub updated_at: String,
    pub mirror_state: MirrorSyncState,
    #[serde(default)]
    pub authority_refs: Vec<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mirror_contract: Option<MarkdownMirrorContractV1>,
    pub workflow_state_family: WorkflowStateFamily,
    pub queue_reason_code: WorkflowQueueReasonCode,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_action_ids: Vec<String>,
    pub summary_ref: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub note_refs: Vec<String>,
    pub wp_id: String,
    pub version: u64,
    pub title: String,
    pub description: String,
    pub status: WorkPacketStatus,
    pub priority: u8,
    pub governance: WorkPacketGovernance,
    #[serde(rename = "type")]
    pub kind: WorkPacketType,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,
    pub reporter: String,
    pub micro_tasks: MicroTaskSummary,
    pub created_at: Iso8601Timestamp,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<Iso8601Timestamp>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<Iso8601Timestamp>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_at: Option<Iso8601Timestamp>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<WorkNote>,
    #[serde(default)]
    pub metadata: Value,
    #[serde(default)]
    pub vector_clock: VectorClock,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tombstone: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrackedMicroTaskArtifactV1 {
    pub schema_id: String,
    pub schema_version: String,
    pub record_id: String,
    pub record_kind: String,
    pub project_profile_kind: ProjectProfileKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_extension: Option<Value>,
    pub updated_at: String,
    pub mirror_state: MirrorSyncState,
    #[serde(default)]
    pub authority_refs: Vec<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mirror_contract: Option<MarkdownMirrorContractV1>,
    pub workflow_state_family: WorkflowStateFamily,
    pub queue_reason_code: WorkflowQueueReasonCode,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_action_ids: Vec<String>,
    pub summary_ref: String,
    pub mt_id: String,
    pub wp_id: String,
    pub name: String,
    pub scope: String,
    pub files: MicroTaskFiles,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub done_criteria: Vec<String>,
    pub status: MicroTaskStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub active_session_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub iterations: Vec<MicroTaskIterationRecord>,
    pub current_iteration: u32,
    pub max_iterations: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validation_result: Option<MicroTaskValidationResult>,
    pub escalation: MicroTaskEscalation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<Iso8601Timestamp>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<Iso8601Timestamp>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkPacketStatus {
    #[serde(rename = "stub")]
    Unknown,
    Ready,
    InProgress,
    Blocked,
    Gated,
    Done,
    Cancelled,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TaskBoardStatus {
    #[serde(rename = "STUB")]
    Unknown,
    Ready,
    InProgress,
    Blocked,
    Gated,
    Done,
    Cancelled,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GateStatusKind {
    Pending,
    Pass,
    Fail,
    Skip,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GateStatus {
    pub status: GateStatusKind,
    #[serde(default)]
    pub validated_at: Option<Iso8601Timestamp>,
    #[serde(default)]
    pub validated_by: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub validation_report_ref: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GateStatuses {
    pub pre_work: GateStatus,
    pub post_work: GateStatus,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkPacketPhase {
    #[serde(rename = "0")]
    Phase0,
    #[serde(rename = "0.5")]
    Phase0_5,
    #[serde(rename = "1")]
    Phase1,
    #[serde(rename = "2")]
    Phase2,
    #[serde(rename = "3")]
    Phase3,
    #[serde(rename = "4")]
    Phase4,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RoutingPolicy {
    #[serde(rename = "GOV_STRICT")]
    GovStrict,
    #[serde(rename = "GOV_STANDARD")]
    GovStandard,
    #[serde(rename = "GOV_LIGHT")]
    GovLight,
    #[serde(rename = "GOV_NONE")]
    GovNone,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkPacketGovernance {
    pub phase: WorkPacketPhase,
    pub routing: RoutingPolicy,
    #[serde(default)]
    pub spec_session_id: Option<String>,
    pub gates: GateStatuses,
    #[serde(default)]
    pub task_packet_path: Option<String>,
    pub task_board_status: TaskBoardStatus,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkPacketType {
    Feature,
    Bug,
    Refactor,
    Docs,
    Test,
    Chore,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MicroTaskSummary {
    pub total: u32,
    pub completed: u32,
    pub failed: u32,
    pub in_progress: u32,
    #[serde(default)]
    pub mt_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkNoteType {
    User,
    System,
    Gate,
    MtExecutor,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkNote {
    pub note_id: String,
    pub author: String,
    pub content: String,
    pub created_at: Iso8601Timestamp,
    pub note_type: WorkNoteType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrackedWorkPacket {
    #[serde(default = "default_work_packet_schema_id")]
    pub schema_id: String,
    #[serde(default = "default_work_packet_schema_version")]
    pub schema_version: String,
    #[serde(default)]
    pub record_id: String,
    #[serde(default = "default_work_packet_record_kind")]
    pub record_kind: String,
    #[serde(default)]
    pub project_profile_kind: ProjectProfileKind,
    #[serde(default)]
    pub mirror_state: MirrorSyncState,
    #[serde(default)]
    pub authority_refs: Vec<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub summary_record_path: Option<String>,
    #[serde(default)]
    pub profile_extension: Option<Value>,
    pub wp_id: String,
    pub version: u64,
    pub title: String,
    pub description: String,
    pub status: WorkPacketStatus,
    pub priority: u8,
    pub governance: WorkPacketGovernance,
    #[serde(rename = "type")]
    pub kind: WorkPacketType,
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default)]
    pub assignee: Option<String>,
    pub reporter: String,
    pub micro_tasks: MicroTaskSummary,
    pub created_at: Iso8601Timestamp,
    pub updated_at: Iso8601Timestamp,
    #[serde(default)]
    pub started_at: Option<Iso8601Timestamp>,
    #[serde(default)]
    pub completed_at: Option<Iso8601Timestamp>,
    #[serde(default)]
    pub due_at: Option<Iso8601Timestamp>,
    #[serde(default)]
    pub notes: Vec<WorkNote>,
    #[serde(default)]
    pub metadata: Value,
    #[serde(default)]
    pub vector_clock: VectorClock,
    #[serde(default)]
    pub tombstone: Option<Value>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DependencyType {
    #[serde(rename = "blocks")]
    Blocks,
    #[serde(rename = "blocked_by")]
    BlockedBy,
    #[serde(rename = "related")]
    Related,
    #[serde(rename = "parent-child")]
    ParentChild,
    #[serde(rename = "discovered-from")]
    DiscoveredFrom,
    #[serde(rename = "duplicate-of")]
    DuplicateOf,
    #[serde(rename = "depends-on")]
    DependsOn,
    #[serde(rename = "implements")]
    Implements,
    #[serde(rename = "tests")]
    Tests,
    #[serde(rename = "documents")]
    Documents,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrackedDependency {
    pub dependency_id: String,
    pub from_wp_id: String,
    pub to_wp_id: String,
    #[serde(rename = "type")]
    pub kind: DependencyType,
    #[serde(default)]
    pub description: Option<String>,
    pub created_at: Iso8601Timestamp,
    pub created_by: String,
    pub vector_clock: VectorClock,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MicroTaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Blocked,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MicroTaskFiles {
    #[serde(default)]
    pub read: Vec<String>,
    #[serde(default)]
    pub modify: Vec<String>,
    #[serde(default)]
    pub create: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MicroTaskValidationResult {
    pub passed: bool,
    #[serde(default)]
    pub blocking_failures: Vec<String>,
    #[serde(default)]
    pub warnings: Vec<String>,
    pub evidence_ref: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EscalationLevel {
    pub model_id: String,
    #[serde(default)]
    pub lora_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MicroTaskEscalation {
    pub current_level: u32,
    #[serde(default)]
    pub escalation_chain: Vec<EscalationLevel>,
    pub escalations_count: u32,
    pub drop_backs_count: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MicroTaskIterationOutcome {
    #[serde(rename = "SUCCESS")]
    Success,
    #[serde(rename = "RETRY")]
    Retry,
    #[serde(rename = "ESCALATE")]
    Escalate,
    #[serde(rename = "BLOCKED")]
    Blocked,
    #[serde(rename = "SKIPPED")]
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MicroTaskIterationRecord {
    pub iteration: u32,
    pub model_id: String,
    #[serde(default)]
    pub lora_id: Option<String>,
    pub escalation_level: u32,
    pub started_at: Iso8601Timestamp,
    pub completed_at: Iso8601Timestamp,
    pub duration_ms: u64,
    pub tokens_prompt: u32,
    pub tokens_completion: u32,
    pub claimed_complete: bool,
    #[serde(default)]
    pub validation_passed: Option<bool>,
    pub outcome: MicroTaskIterationOutcome,
    pub output_artifact_ref: Value,
    #[serde(default)]
    pub validation_artifact_ref: Option<Value>,
    #[serde(default)]
    pub error_summary: Option<String>,
    #[serde(default)]
    pub failure_category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrackedMicroTask {
    #[serde(default = "default_micro_task_schema_id")]
    pub schema_id: String,
    #[serde(default = "default_micro_task_schema_version")]
    pub schema_version: String,
    #[serde(default)]
    pub record_id: String,
    #[serde(default = "default_micro_task_record_kind")]
    pub record_kind: String,
    #[serde(default)]
    pub project_profile_kind: ProjectProfileKind,
    #[serde(default = "default_structured_updated_at")]
    pub updated_at: Iso8601Timestamp,
    #[serde(default)]
    pub mirror_state: MirrorSyncState,
    #[serde(default)]
    pub authority_refs: Vec<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub summary_record_path: Option<String>,
    #[serde(default)]
    pub profile_extension: Option<Value>,
    pub mt_id: String,
    pub wp_id: String,
    pub name: String,
    pub scope: String,
    pub files: MicroTaskFiles,
    #[serde(default)]
    pub done_criteria: Vec<String>,
    pub status: MicroTaskStatus,
    #[serde(default)]
    pub active_session_ids: Vec<String>,
    #[serde(default)]
    pub iterations: Vec<MicroTaskIterationRecord>,
    pub current_iteration: u32,
    pub max_iterations: u32,
    #[serde(default)]
    pub validation_result: Option<MicroTaskValidationResult>,
    pub escalation: MicroTaskEscalation,
    #[serde(default)]
    pub started_at: Option<Iso8601Timestamp>,
    #[serde(default)]
    pub completed_at: Option<Iso8601Timestamp>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocusCreateWpParams {
    pub wp_id: String,
    pub title: String,
    pub description: String,
    pub priority: u8,
    #[serde(rename = "type")]
    pub kind: WorkPacketType,
    pub phase: WorkPacketPhase,
    pub routing: RoutingPolicy,
    #[serde(default)]
    pub task_packet_path: Option<String>,
    #[serde(default)]
    pub assignee: Option<String>,
    #[serde(default)]
    pub labels: Option<Vec<String>>,
    #[serde(default)]
    pub spec_session_id: Option<String>,
    pub reporter: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocusUpdateWpParams {
    pub wp_id: String,
    pub updates: BTreeMap<String, Value>,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LocusGateKind {
    PreWork,
    PostWork,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocusGateWpParams {
    pub wp_id: String,
    pub gate: LocusGateKind,
    pub result: GateStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocusCloseWpParams {
    pub wp_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocusDeleteWpParams {
    pub wp_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocusRegisterMtsParams {
    pub wp_id: String,
    pub micro_tasks: Vec<TrackedMicroTask>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocusStartMtParams {
    pub wp_id: String,
    pub mt_id: String,
    pub model_id: String,
    #[serde(default)]
    pub lora_id: Option<String>,
    pub escalation_level: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocusRecordIterationParams {
    pub wp_id: String,
    pub mt_id: String,
    pub iteration: MicroTaskIterationRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocusCompleteMtParams {
    pub wp_id: String,
    pub mt_id: String,
    pub final_iteration: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocusBindSessionParams {
    pub wp_id: String,
    pub mt_id: String,
    pub session_id: String,
    #[serde(default)]
    pub model_id: Option<String>,
    #[serde(default)]
    pub lora_id: Option<String>,
    pub escalation_level: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocusUnbindSessionParams {
    pub wp_id: String,
    pub mt_id: String,
    pub session_id: String,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocusAddDependencyParams {
    pub dependency_id: String,
    pub from_wp_id: String,
    pub to_wp_id: String,
    #[serde(rename = "type")]
    pub kind: DependencyType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocusRemoveDependencyParams {
    pub dependency_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocusQueryReadyParams {
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocusGetWpStatusParams {
    pub wp_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocusGetMtProgressParams {
    pub mt_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocusSyncTaskBoardParams {
    #[serde(default)]
    pub dry_run: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "operation", content = "params", rename_all = "snake_case")]
pub enum LocusOperation {
    CreateWp(LocusCreateWpParams),
    UpdateWp(LocusUpdateWpParams),
    GateWp(LocusGateWpParams),
    CloseWp(LocusCloseWpParams),
    DeleteWp(LocusDeleteWpParams),
    RegisterMts(LocusRegisterMtsParams),
    StartMt(LocusStartMtParams),
    BindSession(LocusBindSessionParams),
    UnbindSession(LocusUnbindSessionParams),
    RecordIteration(LocusRecordIterationParams),
    CompleteMt(LocusCompleteMtParams),
    AddDependency(LocusAddDependencyParams),
    RemoveDependency(LocusRemoveDependencyParams),
    QueryReady(LocusQueryReadyParams),
    GetWpStatus(LocusGetWpStatusParams),
    GetMtProgress(LocusGetMtProgressParams),
    SyncTaskBoard(LocusSyncTaskBoardParams),
}

pub const TRACKED_WORK_PACKET_SCHEMA_ID_V1: &str = "hsk.tracked_work_packet@1";
pub const TRACKED_MICRO_TASK_SCHEMA_ID_V1: &str = "hsk.tracked_micro_task@1";
pub const STRUCTURED_COLLABORATION_SUMMARY_SCHEMA_ID_V1: &str =
    "hsk.structured_collaboration_summary@1";
pub const TASK_BOARD_ENTRY_SCHEMA_ID_V1: &str = "hsk.task_board_entry@1";
pub const TASK_BOARD_INDEX_SCHEMA_ID_V1: &str = "hsk.task_board_index@1";
pub const TASK_BOARD_VIEW_SCHEMA_ID_V1: &str = "hsk.task_board_view@1";
pub const ROLE_MAILBOX_INDEX_SCHEMA_ID_V1: &str = "hsk.role_mailbox_index@1";
pub const ROLE_MAILBOX_THREAD_LINE_SCHEMA_ID_V1: &str = "hsk.role_mailbox_thread_line@1";
pub const STRUCTURED_COLLABORATION_SCHEMA_VERSION_V1: &str = "1";
pub const ROLE_MAILBOX_EXPORT_SCHEMA_VERSION_V1: &str = "role_mailbox_export_v1";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StructuredCollaborationRecordFamily {
    WorkPacketPacket,
    WorkPacketSummary,
    MicroTaskPacket,
    MicroTaskSummary,
    TaskBoardEntry,
    TaskBoardIndex,
    TaskBoardView,
    RoleMailboxIndex,
    RoleMailboxThreadLine,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct StructuredCollaborationSchemaDescriptor {
    pub family: StructuredCollaborationRecordFamily,
    pub schema_id: &'static str,
    pub schema_version: &'static str,
    pub record_kind: &'static str,
    pub summary_family: Option<StructuredCollaborationRecordFamily>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StructuredCollaborationSummaryRecord {
    pub schema_id: String,
    pub schema_version: String,
    pub record_id: String,
    pub record_kind: String,
    pub project_profile_kind: ProjectProfileKind,
    pub mirror_state: MirrorSyncState,
    pub status: String,
    pub title_or_objective: String,
    #[serde(default)]
    pub blockers: Vec<String>,
    pub next_action: String,
    pub updated_at: String,
    #[serde(default)]
    pub authority_refs: Vec<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StructuredCollaborationValidationCode {
    UnknownSchemaId,
    SchemaVersionMismatch,
    MissingField,
    InvalidFieldType,
    InvalidFieldValue,
    IncompatibleProfileExtension,
    SummaryJoinMismatch,
    AuthorityScopeMismatch,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StructuredCollaborationValidationIssue {
    pub code: StructuredCollaborationValidationCode,
    pub field: String,
    #[serde(default)]
    pub expected: Option<String>,
    #[serde(default)]
    pub actual: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StructuredCollaborationValidationResult {
    pub ok: bool,
    pub family: StructuredCollaborationRecordFamily,
    pub schema_id: String,
    pub schema_version: String,
    #[serde(default)]
    pub issues: Vec<StructuredCollaborationValidationIssue>,
}

impl StructuredCollaborationValidationResult {
    pub fn success(family: StructuredCollaborationRecordFamily) -> Self {
        let descriptor = structured_collaboration_schema_descriptor(family);
        Self {
            ok: true,
            family,
            schema_id: descriptor.schema_id.to_string(),
            schema_version: descriptor.schema_version.to_string(),
            issues: Vec::new(),
        }
    }

    pub fn push_issue(
        &mut self,
        code: StructuredCollaborationValidationCode,
        field: impl Into<String>,
        expected: Option<String>,
        actual: Option<String>,
        message: impl Into<String>,
    ) {
        self.ok = false;
        self.issues.push(StructuredCollaborationValidationIssue {
            code,
            field: field.into(),
            expected,
            actual,
            message: message.into(),
        });
    }

    pub fn merge(&mut self, other: Self) {
        if !other.ok {
            self.ok = false;
        }
        self.issues.extend(other.issues);
    }
}

pub fn structured_collaboration_schema_descriptor(
    family: StructuredCollaborationRecordFamily,
) -> StructuredCollaborationSchemaDescriptor {
    match family {
        StructuredCollaborationRecordFamily::WorkPacketPacket => {
            StructuredCollaborationSchemaDescriptor {
                family,
                schema_id: TRACKED_WORK_PACKET_SCHEMA_ID_V1,
                schema_version: STRUCTURED_COLLABORATION_SCHEMA_VERSION_V1,
                record_kind: "work_packet",
                summary_family: Some(StructuredCollaborationRecordFamily::WorkPacketSummary),
            }
        }
        StructuredCollaborationRecordFamily::WorkPacketSummary => {
            StructuredCollaborationSchemaDescriptor {
                family,
                schema_id: STRUCTURED_COLLABORATION_SUMMARY_SCHEMA_ID_V1,
                schema_version: STRUCTURED_COLLABORATION_SCHEMA_VERSION_V1,
                record_kind: "work_packet",
                summary_family: Some(StructuredCollaborationRecordFamily::WorkPacketPacket),
            }
        }
        StructuredCollaborationRecordFamily::MicroTaskPacket => {
            StructuredCollaborationSchemaDescriptor {
                family,
                schema_id: TRACKED_MICRO_TASK_SCHEMA_ID_V1,
                schema_version: STRUCTURED_COLLABORATION_SCHEMA_VERSION_V1,
                record_kind: "micro_task",
                summary_family: Some(StructuredCollaborationRecordFamily::MicroTaskSummary),
            }
        }
        StructuredCollaborationRecordFamily::MicroTaskSummary => {
            StructuredCollaborationSchemaDescriptor {
                family,
                schema_id: STRUCTURED_COLLABORATION_SUMMARY_SCHEMA_ID_V1,
                schema_version: STRUCTURED_COLLABORATION_SCHEMA_VERSION_V1,
                record_kind: "micro_task",
                summary_family: Some(StructuredCollaborationRecordFamily::MicroTaskPacket),
            }
        }
        StructuredCollaborationRecordFamily::TaskBoardEntry => {
            StructuredCollaborationSchemaDescriptor {
                family,
                schema_id: TASK_BOARD_ENTRY_SCHEMA_ID_V1,
                schema_version: STRUCTURED_COLLABORATION_SCHEMA_VERSION_V1,
                record_kind: "task_board_entry",
                summary_family: None,
            }
        }
        StructuredCollaborationRecordFamily::TaskBoardIndex => {
            StructuredCollaborationSchemaDescriptor {
                family,
                schema_id: TASK_BOARD_INDEX_SCHEMA_ID_V1,
                schema_version: STRUCTURED_COLLABORATION_SCHEMA_VERSION_V1,
                record_kind: "task_board_index",
                summary_family: None,
            }
        }
        StructuredCollaborationRecordFamily::TaskBoardView => {
            StructuredCollaborationSchemaDescriptor {
                family,
                schema_id: TASK_BOARD_VIEW_SCHEMA_ID_V1,
                schema_version: STRUCTURED_COLLABORATION_SCHEMA_VERSION_V1,
                record_kind: "task_board_view",
                summary_family: None,
            }
        }
        StructuredCollaborationRecordFamily::RoleMailboxIndex => {
            StructuredCollaborationSchemaDescriptor {
                family,
                schema_id: ROLE_MAILBOX_INDEX_SCHEMA_ID_V1,
                schema_version: ROLE_MAILBOX_EXPORT_SCHEMA_VERSION_V1,
                record_kind: "generic",
                summary_family: None,
            }
        }
        StructuredCollaborationRecordFamily::RoleMailboxThreadLine => {
            StructuredCollaborationSchemaDescriptor {
                family,
                schema_id: ROLE_MAILBOX_THREAD_LINE_SCHEMA_ID_V1,
                schema_version: ROLE_MAILBOX_EXPORT_SCHEMA_VERSION_V1,
                record_kind: "role_mailbox_message",
                summary_family: None,
            }
        }
    }
}

pub fn validate_structured_collaboration_record(
    family: StructuredCollaborationRecordFamily,
    value: &Value,
) -> StructuredCollaborationValidationResult {
    let descriptor = structured_collaboration_schema_descriptor(family);
    let mut result = StructuredCollaborationValidationResult::success(family);

    let Some(obj) = value.as_object() else {
        result.push_issue(
            StructuredCollaborationValidationCode::InvalidFieldType,
            "$",
            Some("object".to_string()),
            Some(json_type_name(value).to_string()),
            "structured collaboration record must be a JSON object",
        );
        return result;
    };

    validate_expected_string(
        obj.get("schema_id"),
        "schema_id",
        descriptor.schema_id,
        &mut result,
    );
    validate_expected_string(
        obj.get("schema_version"),
        "schema_version",
        descriptor.schema_version,
        &mut result,
    );
    require_non_empty_string(obj.get("record_id"), "record_id", &mut result);
    validate_expected_string(
        obj.get("record_kind"),
        "record_kind",
        descriptor.record_kind,
        &mut result,
    );
    validate_project_profile_kind(obj.get("project_profile_kind"), &mut result);
    require_non_empty_string(obj.get("updated_at"), "updated_at", &mut result);
    validate_mirror_state(obj.get("mirror_state"), &mut result);
    require_string_array(obj.get("authority_refs"), "authority_refs", &mut result);
    require_string_array(obj.get("evidence_refs"), "evidence_refs", &mut result);
    validate_profile_extension(obj.get("profile_extension"), &mut result);

    match family {
        StructuredCollaborationRecordFamily::WorkPacketPacket
        | StructuredCollaborationRecordFamily::MicroTaskPacket => {
            if let Some(summary_path) = obj.get("summary_record_path") {
                require_non_empty_string(Some(summary_path), "summary_record_path", &mut result);
            }
        }
        StructuredCollaborationRecordFamily::WorkPacketSummary
        | StructuredCollaborationRecordFamily::MicroTaskSummary => {
            require_non_empty_string(obj.get("status"), "status", &mut result);
            require_non_empty_string(
                obj.get("title_or_objective"),
                "title_or_objective",
                &mut result,
            );
            require_string_array(obj.get("blockers"), "blockers", &mut result);
            require_non_empty_string(obj.get("next_action"), "next_action", &mut result);
        }
        StructuredCollaborationRecordFamily::TaskBoardEntry => {
            require_non_empty_string(obj.get("task_board_id"), "task_board_id", &mut result);
            require_non_empty_string(obj.get("work_packet_id"), "work_packet_id", &mut result);
            require_non_empty_string(obj.get("lane_id"), "lane_id", &mut result);
            require_non_empty_string(obj.get("token"), "token", &mut result);
            require_u64_like(obj.get("display_order"), "display_order", &mut result);
            if let Some(view_ids) = obj.get("view_ids") {
                require_string_array(Some(view_ids), "view_ids", &mut result);
            }
        }
        StructuredCollaborationRecordFamily::TaskBoardIndex => {
            require_non_empty_string(obj.get("task_board_id"), "task_board_id", &mut result);
            require_string_array(obj.get("view_ids"), "view_ids", &mut result);
            require_value_array(obj.get("rows"), "rows", &mut result);
        }
        StructuredCollaborationRecordFamily::TaskBoardView => {
            require_non_empty_string(obj.get("task_board_id"), "task_board_id", &mut result);
            require_non_empty_string(obj.get("view_id"), "view_id", &mut result);
            require_string_array(obj.get("lane_ids"), "lane_ids", &mut result);
            require_value_array(obj.get("rows"), "rows", &mut result);
        }
        StructuredCollaborationRecordFamily::RoleMailboxIndex => {
            require_non_empty_string(obj.get("generated_at"), "generated_at", &mut result);
            require_value_array(obj.get("threads"), "threads", &mut result);
        }
        StructuredCollaborationRecordFamily::RoleMailboxThreadLine => {
            require_non_empty_string(obj.get("message_id"), "message_id", &mut result);
            require_non_empty_string(obj.get("thread_id"), "thread_id", &mut result);
            require_non_empty_string(obj.get("created_at"), "created_at", &mut result);
            require_non_empty_string(obj.get("from_role"), "from_role", &mut result);
            require_string_array(obj.get("to_roles"), "to_roles", &mut result);
            require_non_empty_string(obj.get("message_type"), "message_type", &mut result);
            require_non_empty_string(obj.get("body_ref"), "body_ref", &mut result);
            require_non_empty_string(obj.get("body_sha256"), "body_sha256", &mut result);
            require_string_array(obj.get("attachments"), "attachments", &mut result);
            require_value_array(
                obj.get("transcription_links"),
                "transcription_links",
                &mut result,
            );
        }
    }

    result
}

pub fn validate_structured_collaboration_summary_join(
    detail_family: StructuredCollaborationRecordFamily,
    detail: &Value,
    summary_family: StructuredCollaborationRecordFamily,
    summary: &Value,
) -> StructuredCollaborationValidationResult {
    let mut result = StructuredCollaborationValidationResult::success(detail_family);
    result.merge(validate_structured_collaboration_record(
        detail_family,
        detail,
    ));
    result.merge(validate_structured_collaboration_record(
        summary_family,
        summary,
    ));

    let detail_descriptor = structured_collaboration_schema_descriptor(detail_family);
    if detail_descriptor.summary_family != Some(summary_family) {
        result.push_issue(
            StructuredCollaborationValidationCode::SummaryJoinMismatch,
            "summary_family",
            detail_descriptor
                .summary_family
                .map(|family| format!("{family:?}")),
            Some(format!("{summary_family:?}")),
            "detail and summary families do not form a valid structured collaboration pair",
        );
        return result;
    }

    let Some(detail_obj) = detail.as_object() else {
        return result;
    };
    let Some(summary_obj) = summary.as_object() else {
        return result;
    };

    compare_string_field(detail_obj, summary_obj, "record_id", &mut result);
    compare_string_field(detail_obj, summary_obj, "record_kind", &mut result);
    compare_string_field(detail_obj, summary_obj, "project_profile_kind", &mut result);
    compare_string_arrays(detail_obj, summary_obj, "authority_refs", &mut result);
    compare_string_arrays(detail_obj, summary_obj, "evidence_refs", &mut result);

    result
}

pub fn seed_tracked_micro_task_registry_fields(
    tracked_mt: &mut TrackedMicroTask,
    packet_path: impl Into<String>,
    summary_path: impl Into<String>,
    authority_refs: Vec<String>,
    evidence_refs: Vec<String>,
) {
    tracked_mt.schema_id = TRACKED_MICRO_TASK_SCHEMA_ID_V1.to_string();
    tracked_mt.schema_version = STRUCTURED_COLLABORATION_SCHEMA_VERSION_V1.to_string();
    tracked_mt.record_id = tracked_mt.mt_id.clone();
    tracked_mt.record_kind = structured_collaboration_schema_descriptor(
        StructuredCollaborationRecordFamily::MicroTaskPacket,
    )
    .record_kind
    .to_string();
    tracked_mt.project_profile_kind = ProjectProfileKind::SoftwareDelivery;
    tracked_mt.updated_at = Utc::now();
    tracked_mt.mirror_state = MirrorSyncState::CanonicalOnly;
    tracked_mt.authority_refs = authority_refs;
    tracked_mt.evidence_refs = evidence_refs;
    tracked_mt.summary_record_path = Some(summary_path.into());
    tracked_mt.metadata["structured_collaboration_packet_path"] = Value::String(packet_path.into());
}

pub fn seed_tracked_work_packet_registry_fields(
    tracked_wp: &mut TrackedWorkPacket,
    summary_path: impl Into<String>,
    authority_refs: Vec<String>,
    evidence_refs: Vec<String>,
) {
    tracked_wp.schema_id = TRACKED_WORK_PACKET_SCHEMA_ID_V1.to_string();
    tracked_wp.schema_version = STRUCTURED_COLLABORATION_SCHEMA_VERSION_V1.to_string();
    tracked_wp.record_id = tracked_wp.wp_id.clone();
    tracked_wp.record_kind = structured_collaboration_schema_descriptor(
        StructuredCollaborationRecordFamily::WorkPacketPacket,
    )
    .record_kind
    .to_string();
    tracked_wp.project_profile_kind = ProjectProfileKind::SoftwareDelivery;
    tracked_wp.updated_at = Utc::now();
    tracked_wp.mirror_state = MirrorSyncState::CanonicalOnly;
    tracked_wp.authority_refs = authority_refs;
    tracked_wp.evidence_refs = evidence_refs;
    tracked_wp.summary_record_path = Some(summary_path.into());
}

pub fn backfill_tracked_work_packet_registry_fields(
    tracked_wp: &mut TrackedWorkPacket,
    summary_path: impl Into<String>,
    authority_refs: Vec<String>,
    evidence_refs: Vec<String>,
) {
    let summary_path = summary_path.into();
    if tracked_wp.schema_id.trim().is_empty() {
        tracked_wp.schema_id = TRACKED_WORK_PACKET_SCHEMA_ID_V1.to_string();
    }
    if tracked_wp.schema_version.trim().is_empty() {
        tracked_wp.schema_version = STRUCTURED_COLLABORATION_SCHEMA_VERSION_V1.to_string();
    }
    if tracked_wp.record_id.trim().is_empty() {
        tracked_wp.record_id = tracked_wp.wp_id.clone();
    }
    if tracked_wp.record_kind.trim().is_empty() {
        tracked_wp.record_kind = structured_collaboration_schema_descriptor(
            StructuredCollaborationRecordFamily::WorkPacketPacket,
        )
        .record_kind
        .to_string();
    }
    if tracked_wp.authority_refs.is_empty() {
        tracked_wp.authority_refs = authority_refs;
    }
    if tracked_wp.evidence_refs.is_empty() {
        tracked_wp.evidence_refs = evidence_refs;
    }
    if tracked_wp
        .summary_record_path
        .as_ref()
        .map(|value| value.trim().is_empty())
        .unwrap_or(true)
    {
        tracked_wp.summary_record_path = Some(summary_path);
    }
}

pub fn backfill_tracked_micro_task_registry_fields(
    tracked_mt: &mut TrackedMicroTask,
    packet_path: impl Into<String>,
    summary_path: impl Into<String>,
    authority_refs: Vec<String>,
    evidence_refs: Vec<String>,
) {
    let packet_path = packet_path.into();
    let summary_path = summary_path.into();
    if tracked_mt.schema_id.trim().is_empty() {
        tracked_mt.schema_id = TRACKED_MICRO_TASK_SCHEMA_ID_V1.to_string();
    }
    if tracked_mt.schema_version.trim().is_empty() {
        tracked_mt.schema_version = STRUCTURED_COLLABORATION_SCHEMA_VERSION_V1.to_string();
    }
    if tracked_mt.record_id.trim().is_empty() {
        tracked_mt.record_id = tracked_mt.mt_id.clone();
    }
    if tracked_mt.record_kind.trim().is_empty() {
        tracked_mt.record_kind = structured_collaboration_schema_descriptor(
            StructuredCollaborationRecordFamily::MicroTaskPacket,
        )
        .record_kind
        .to_string();
    }
    tracked_mt.updated_at = Utc::now();
    if tracked_mt.authority_refs.is_empty() {
        tracked_mt.authority_refs = authority_refs;
    }
    if tracked_mt.evidence_refs.is_empty() {
        tracked_mt.evidence_refs = evidence_refs;
    }
    if tracked_mt
        .summary_record_path
        .as_ref()
        .map(|value| value.trim().is_empty())
        .unwrap_or(true)
    {
        tracked_mt.summary_record_path = Some(summary_path);
    }
    if !tracked_mt.metadata.is_object() {
        tracked_mt.metadata = Value::Object(Default::default());
    }
    let packet_path_present = tracked_mt
        .metadata
        .get("structured_collaboration_packet_path")
        .and_then(Value::as_str)
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);
    if !packet_path_present {
        tracked_mt.metadata["structured_collaboration_packet_path"] = Value::String(packet_path);
    }
}

pub fn default_structured_collaboration_summary_record(
    family: StructuredCollaborationRecordFamily,
    record_id: impl Into<String>,
    title_or_objective: impl Into<String>,
    status: impl Into<String>,
    next_action: impl Into<String>,
    authority_refs: Vec<String>,
    evidence_refs: Vec<String>,
    updated_at: impl Into<String>,
    project_profile_kind: ProjectProfileKind,
    mirror_state: MirrorSyncState,
) -> StructuredCollaborationSummaryRecord {
    let summary_family = match family {
        StructuredCollaborationRecordFamily::WorkPacketPacket
        | StructuredCollaborationRecordFamily::WorkPacketSummary => {
            StructuredCollaborationRecordFamily::WorkPacketSummary
        }
        StructuredCollaborationRecordFamily::MicroTaskPacket
        | StructuredCollaborationRecordFamily::MicroTaskSummary => {
            StructuredCollaborationRecordFamily::MicroTaskSummary
        }
        _ => StructuredCollaborationRecordFamily::WorkPacketSummary,
    };
    let descriptor = structured_collaboration_schema_descriptor(summary_family);
    StructuredCollaborationSummaryRecord {
        schema_id: descriptor.schema_id.to_string(),
        schema_version: descriptor.schema_version.to_string(),
        record_id: record_id.into(),
        record_kind: descriptor.record_kind.to_string(),
        project_profile_kind,
        mirror_state,
        status: status.into(),
        title_or_objective: title_or_objective.into(),
        blockers: Vec::new(),
        next_action: next_action.into(),
        updated_at: updated_at.into(),
        authority_refs,
        evidence_refs,
    }
}

fn default_work_packet_schema_id() -> String {
    TRACKED_WORK_PACKET_SCHEMA_ID_V1.to_string()
}

fn default_work_packet_schema_version() -> String {
    STRUCTURED_COLLABORATION_SCHEMA_VERSION_V1.to_string()
}

fn default_work_packet_record_kind() -> String {
    structured_collaboration_schema_descriptor(
        StructuredCollaborationRecordFamily::WorkPacketPacket,
    )
    .record_kind
    .to_string()
}

fn default_micro_task_schema_id() -> String {
    TRACKED_MICRO_TASK_SCHEMA_ID_V1.to_string()
}

fn default_micro_task_schema_version() -> String {
    STRUCTURED_COLLABORATION_SCHEMA_VERSION_V1.to_string()
}

fn default_micro_task_record_kind() -> String {
    structured_collaboration_schema_descriptor(StructuredCollaborationRecordFamily::MicroTaskPacket)
        .record_kind
        .to_string()
}

fn default_structured_updated_at() -> Iso8601Timestamp {
    Utc::now()
}

fn json_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn validate_expected_string(
    value: Option<&Value>,
    field: &str,
    expected: &str,
    result: &mut StructuredCollaborationValidationResult,
) {
    match value {
        Some(Value::String(actual)) => {
            if actual != expected {
                result.push_issue(
                    if field == "schema_id" {
                        StructuredCollaborationValidationCode::UnknownSchemaId
                    } else {
                        StructuredCollaborationValidationCode::SchemaVersionMismatch
                    },
                    field,
                    Some(expected.to_string()),
                    Some(actual.clone()),
                    format!("{field} does not match the canonical registry"),
                );
            }
        }
        Some(other) => result.push_issue(
            StructuredCollaborationValidationCode::InvalidFieldType,
            field,
            Some("string".to_string()),
            Some(json_type_name(other).to_string()),
            format!("{field} must be a string"),
        ),
        None => result.push_issue(
            StructuredCollaborationValidationCode::MissingField,
            field,
            Some(expected.to_string()),
            None,
            format!("{field} is required"),
        ),
    }
}

fn require_non_empty_string(
    value: Option<&Value>,
    field: &str,
    result: &mut StructuredCollaborationValidationResult,
) {
    match value {
        Some(Value::String(actual)) if !actual.trim().is_empty() => {}
        Some(Value::String(actual)) => result.push_issue(
            StructuredCollaborationValidationCode::InvalidFieldValue,
            field,
            Some("non-empty string".to_string()),
            Some(actual.clone()),
            format!("{field} must not be empty"),
        ),
        Some(other) => result.push_issue(
            StructuredCollaborationValidationCode::InvalidFieldType,
            field,
            Some("string".to_string()),
            Some(json_type_name(other).to_string()),
            format!("{field} must be a string"),
        ),
        None => result.push_issue(
            StructuredCollaborationValidationCode::MissingField,
            field,
            Some("string".to_string()),
            None,
            format!("{field} is required"),
        ),
    }
}

fn require_string_array(
    value: Option<&Value>,
    field: &str,
    result: &mut StructuredCollaborationValidationResult,
) {
    match value {
        Some(Value::Array(items)) => {
            for (index, item) in items.iter().enumerate() {
                if !item.is_string() {
                    result.push_issue(
                        StructuredCollaborationValidationCode::InvalidFieldType,
                        format!("{field}[{index}]"),
                        Some("string".to_string()),
                        Some(json_type_name(item).to_string()),
                        format!("{field}[{index}] must be a string"),
                    );
                }
            }
        }
        Some(other) => result.push_issue(
            StructuredCollaborationValidationCode::InvalidFieldType,
            field,
            Some("array".to_string()),
            Some(json_type_name(other).to_string()),
            format!("{field} must be an array of strings"),
        ),
        None => result.push_issue(
            StructuredCollaborationValidationCode::MissingField,
            field,
            Some("array".to_string()),
            None,
            format!("{field} is required"),
        ),
    }
}

fn require_value_array(
    value: Option<&Value>,
    field: &str,
    result: &mut StructuredCollaborationValidationResult,
) {
    match value {
        Some(Value::Array(_)) => {}
        Some(other) => result.push_issue(
            StructuredCollaborationValidationCode::InvalidFieldType,
            field,
            Some("array".to_string()),
            Some(json_type_name(other).to_string()),
            format!("{field} must be an array"),
        ),
        None => result.push_issue(
            StructuredCollaborationValidationCode::MissingField,
            field,
            Some("array".to_string()),
            None,
            format!("{field} is required"),
        ),
    }
}

fn require_u64_like(
    value: Option<&Value>,
    field: &str,
    result: &mut StructuredCollaborationValidationResult,
) {
    match value {
        Some(Value::Number(number)) if number.is_u64() || number.is_i64() => {}
        Some(other) => result.push_issue(
            StructuredCollaborationValidationCode::InvalidFieldType,
            field,
            Some("integer".to_string()),
            Some(json_type_name(other).to_string()),
            format!("{field} must be an integer"),
        ),
        None => result.push_issue(
            StructuredCollaborationValidationCode::MissingField,
            field,
            Some("integer".to_string()),
            None,
            format!("{field} is required"),
        ),
    }
}

fn validate_project_profile_kind(
    value: Option<&Value>,
    result: &mut StructuredCollaborationValidationResult,
) {
    match value {
        Some(Value::String(actual)) => {
            if ProjectProfileKind::parse(actual).is_none() {
                result.push_issue(
                    StructuredCollaborationValidationCode::InvalidFieldValue,
                    "project_profile_kind",
                    Some("known project profile kind".to_string()),
                    Some(actual.clone()),
                    "project_profile_kind is not one of the registry-supported profile kinds",
                );
            }
        }
        Some(other) => result.push_issue(
            StructuredCollaborationValidationCode::InvalidFieldType,
            "project_profile_kind",
            Some("string".to_string()),
            Some(json_type_name(other).to_string()),
            "project_profile_kind must be a string",
        ),
        None => result.push_issue(
            StructuredCollaborationValidationCode::MissingField,
            "project_profile_kind",
            Some("string".to_string()),
            None,
            "project_profile_kind is required",
        ),
    }
}

fn validate_mirror_state(
    value: Option<&Value>,
    result: &mut StructuredCollaborationValidationResult,
) {
    match value {
        Some(Value::String(actual)) => {
            if MirrorSyncState::parse(actual).is_none() {
                result.push_issue(
                    StructuredCollaborationValidationCode::InvalidFieldValue,
                    "mirror_state",
                    Some("known mirror_state".to_string()),
                    Some(actual.clone()),
                    "mirror_state is not one of the registry-supported values",
                );
            }
        }
        Some(other) => result.push_issue(
            StructuredCollaborationValidationCode::InvalidFieldType,
            "mirror_state",
            Some("string".to_string()),
            Some(json_type_name(other).to_string()),
            "mirror_state must be a string",
        ),
        None => result.push_issue(
            StructuredCollaborationValidationCode::MissingField,
            "mirror_state",
            Some("string".to_string()),
            None,
            "mirror_state is required",
        ),
    }
}

fn validate_profile_extension(
    value: Option<&Value>,
    result: &mut StructuredCollaborationValidationResult,
) {
    let Some(value) = value else {
        return;
    };
    if value.is_null() {
        return;
    }

    let Some(obj) = value.as_object() else {
        result.push_issue(
            StructuredCollaborationValidationCode::InvalidFieldType,
            "profile_extension",
            Some("object".to_string()),
            Some(json_type_name(value).to_string()),
            "profile_extension must be an object when present",
        );
        return;
    };

    require_non_empty_string(
        obj.get("extension_schema_id"),
        "profile_extension.extension_schema_id",
        result,
    );
    require_non_empty_string(
        obj.get("extension_schema_version"),
        "profile_extension.extension_schema_version",
        result,
    );
    if obj.get("compatibility").is_none() {
        result.push_issue(
            StructuredCollaborationValidationCode::IncompatibleProfileExtension,
            "profile_extension.compatibility",
            Some("compatibility metadata".to_string()),
            None,
            "profile_extension must declare compatibility metadata",
        );
    } else if profile_extension_is_breaking(obj.get("compatibility")) {
        result.push_issue(
            StructuredCollaborationValidationCode::IncompatibleProfileExtension,
            "profile_extension.compatibility",
            Some("non-breaking compatibility".to_string()),
            compatibility_repr(obj.get("compatibility")),
            "profile_extension declares breaking compatibility and must be rejected deterministically",
        );
    }
}

fn profile_extension_is_breaking(value: Option<&Value>) -> bool {
    match value {
        Some(Value::String(raw)) => raw.trim().eq_ignore_ascii_case("breaking"),
        Some(Value::Object(obj)) => {
            obj.get("breaking")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                || obj
                    .get("mode")
                    .and_then(Value::as_str)
                    .map(|mode| mode.eq_ignore_ascii_case("breaking"))
                    .unwrap_or(false)
        }
        _ => false,
    }
}

fn compatibility_repr(value: Option<&Value>) -> Option<String> {
    value.map(Value::to_string)
}

fn compare_string_field(
    detail_obj: &serde_json::Map<String, Value>,
    summary_obj: &serde_json::Map<String, Value>,
    field: &str,
    result: &mut StructuredCollaborationValidationResult,
) {
    let detail_value = detail_obj.get(field).and_then(Value::as_str);
    let summary_value = summary_obj.get(field).and_then(Value::as_str);
    if detail_value != summary_value {
        result.push_issue(
            StructuredCollaborationValidationCode::SummaryJoinMismatch,
            field,
            detail_value.map(ToOwned::to_owned),
            summary_value.map(ToOwned::to_owned),
            format!("{field} must match between detail and summary records"),
        );
    }
}

fn compare_string_arrays(
    detail_obj: &serde_json::Map<String, Value>,
    summary_obj: &serde_json::Map<String, Value>,
    field: &str,
    result: &mut StructuredCollaborationValidationResult,
) {
    let detail_values = normalized_string_array(detail_obj.get(field));
    let summary_values = normalized_string_array(summary_obj.get(field));
    if detail_values != summary_values {
        result.push_issue(
            StructuredCollaborationValidationCode::SummaryJoinMismatch,
            field,
            Some(detail_values.join(",")),
            Some(summary_values.join(",")),
            format!("{field} must match between detail and summary records"),
        );
    }
}

fn normalized_string_array(value: Option<&Value>) -> Vec<String> {
    let Some(Value::Array(items)) = value else {
        return Vec::new();
    };
    let mut values = items
        .iter()
        .filter_map(Value::as_str)
        .map(|item| item.to_string())
        .collect::<Vec<_>>();
    values.sort();
    values.dedup();
    values
}
