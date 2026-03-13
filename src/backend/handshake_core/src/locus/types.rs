use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub type Iso8601Timestamp = DateTime<Utc>;
pub type VectorClock = Value;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectProfileKind {
    SoftwareDelivery,
    Research,
    Worldbuilding,
    Design,
    Generic,
    Custom,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MirrorSyncState {
    CanonicalOnly,
    Synchronized,
    Stale,
    AdvisoryEdit,
    NormalizationRequired,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authority_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
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
    pub updated_at: String,
    pub mirror_state: MirrorSyncState,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authority_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
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
    pub updated_at: String,
    pub mirror_state: MirrorSyncState,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authority_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
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
