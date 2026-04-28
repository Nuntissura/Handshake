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

// ── Project-profile workflow extension [v02.171] ──

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkflowFamilyDisplayLabel {
    pub family: WorkflowStateFamily,
    pub display_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkflowReasonDisplayLabel {
    pub reason: WorkflowQueueReasonCode,
    pub display_label: String,
}

/// Project-profile extension for workflow display labels and action narrowing.
/// Per v02.171: extensions MAY relabel families for display but MUST NOT change
/// the base semantic meaning. Unknown extensions MUST degrade to the base
/// workflow-state families, reason codes, and governed action ids.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectProfileWorkflowExtensionV1 {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub family_display_labels: Vec<WorkflowFamilyDisplayLabel>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reason_display_labels: Vec<WorkflowReasonDisplayLabel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub narrowed_reason_codes: Option<Vec<WorkflowQueueReasonCode>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub narrowed_action_ids: Option<Vec<String>>,
}

impl ProjectProfileWorkflowExtensionV1 {
    /// Resolve the display label for a workflow state family.
    /// Degrades to the base family label for unmapped families per v02.171.
    pub fn display_label_for_family(&self, family: WorkflowStateFamily) -> String {
        self.family_display_labels
            .iter()
            .find(|l| l.family == family)
            .map(|l| l.display_label.clone())
            .unwrap_or_else(|| base_workflow_family_label(family).to_string())
    }

    /// Resolve the display label for a queue reason code.
    /// Degrades to the base reason label for unmapped reasons per v02.171.
    pub fn display_label_for_reason(&self, reason: WorkflowQueueReasonCode) -> String {
        self.reason_display_labels
            .iter()
            .find(|l| l.reason == reason)
            .map(|l| l.display_label.clone())
            .unwrap_or_else(|| base_queue_reason_label(reason).to_string())
    }

    /// Filter action_ids to only those narrowed by this profile.
    /// Returns the full base set unchanged if no narrowing is defined.
    pub fn narrow_action_ids(&self, base_action_ids: &[String]) -> Vec<String> {
        match &self.narrowed_action_ids {
            Some(narrowed) => base_action_ids
                .iter()
                .filter(|id| narrowed.contains(id))
                .cloned()
                .collect(),
            None => base_action_ids.to_vec(),
        }
    }
}

// ── Governed action registry [v02.171] ──

/// Returns all registered governed action descriptors for a workflow state family.
/// This is the single source of truth for action legality per family.
/// All action-resolution paths MUST resolve through this registry.
pub fn governed_action_descriptors_for_family(
    family: WorkflowStateFamily,
) -> Vec<GovernedActionDescriptorV1> {
    match family {
        WorkflowStateFamily::Intake => vec![
            GovernedActionDescriptorV1 {
                action_id: "triage".into(),
                title: "Triage".into(),
                description: Some("Classify and route the record for processing".into()),
            },
            GovernedActionDescriptorV1 {
                action_id: "prioritize".into(),
                title: "Prioritize".into(),
                description: Some("Set or adjust priority level".into()),
            },
            GovernedActionDescriptorV1 {
                action_id: "refine_work_packet".into(),
                title: "Refine Work Packet".into(),
                description: Some("Refine a work packet before execution".into()),
            },
        ],
        WorkflowStateFamily::Ready => vec![
            GovernedActionDescriptorV1 {
                action_id: "start".into(),
                title: "Start".into(),
                description: Some("Begin work on the record".into()),
            },
            GovernedActionDescriptorV1 {
                action_id: "assign".into(),
                title: "Assign".into(),
                description: Some("Assign to an executor".into()),
            },
            GovernedActionDescriptorV1 {
                action_id: "assign_work_packet".into(),
                title: "Assign Work Packet".into(),
                description: Some("Assign a work packet to an executor".into()),
            },
            GovernedActionDescriptorV1 {
                action_id: "assign_micro_task".into(),
                title: "Assign Micro-Task".into(),
                description: Some("Assign a micro-task to an executor".into()),
            },
        ],
        WorkflowStateFamily::Active => vec![
            GovernedActionDescriptorV1 {
                action_id: "update".into(),
                title: "Update".into(),
                description: Some("Record progress on active work".into()),
            },
            GovernedActionDescriptorV1 {
                action_id: "complete".into(),
                title: "Complete".into(),
                description: Some("Mark work as complete".into()),
            },
            GovernedActionDescriptorV1 {
                action_id: "pause".into(),
                title: "Pause".into(),
                description: Some("Pause active work".into()),
            },
            GovernedActionDescriptorV1 {
                action_id: "continue_work_packet".into(),
                title: "Continue Work Packet".into(),
                description: Some("Continue active work-packet execution".into()),
            },
            GovernedActionDescriptorV1 {
                action_id: "continue_micro_task".into(),
                title: "Continue Micro-Task".into(),
                description: Some("Continue active micro-task execution".into()),
            },
        ],
        WorkflowStateFamily::Waiting => vec![
            GovernedActionDescriptorV1 {
                action_id: "resume".into(),
                title: "Resume".into(),
                description: Some("Resume after wait condition clears".into()),
            },
            GovernedActionDescriptorV1 {
                action_id: "escalate".into(),
                title: "Escalate".into(),
                description: Some("Escalate the wait to a higher authority".into()),
            },
        ],
        WorkflowStateFamily::Review => vec![
            GovernedActionDescriptorV1 {
                action_id: "review".into(),
                title: "Review".into(),
                description: Some("Perform review of the record".into()),
            },
            GovernedActionDescriptorV1 {
                action_id: "request_changes".into(),
                title: "Request Changes".into(),
                description: Some("Request changes after review".into()),
            },
        ],
        WorkflowStateFamily::Approval => vec![
            GovernedActionDescriptorV1 {
                action_id: "approve".into(),
                title: "Approve".into(),
                description: Some("Grant approval".into()),
            },
            GovernedActionDescriptorV1 {
                action_id: "reject".into(),
                title: "Reject".into(),
                description: Some("Reject the approval request".into()),
            },
            GovernedActionDescriptorV1 {
                action_id: "await_gate_review".into(),
                title: "Await Gate Review".into(),
                description: Some("Wait for a gate review before proceeding".into()),
            },
        ],
        WorkflowStateFamily::Validation => vec![
            GovernedActionDescriptorV1 {
                action_id: "validate".into(),
                title: "Validate".into(),
                description: Some("Run validation checks".into()),
            },
            GovernedActionDescriptorV1 {
                action_id: "repair".into(),
                title: "Repair".into(),
                description: Some("Repair validation failures".into()),
            },
        ],
        WorkflowStateFamily::Blocked => vec![
            GovernedActionDescriptorV1 {
                action_id: "unblock".into(),
                title: "Unblock".into(),
                description: Some("Resolve the blocking condition".into()),
            },
            GovernedActionDescriptorV1 {
                action_id: "escalate".into(),
                title: "Escalate".into(),
                description: Some("Escalate the blocker".into()),
            },
            GovernedActionDescriptorV1 {
                action_id: "resolve_blocker".into(),
                title: "Resolve Blocker".into(),
                description: Some("Resolve the blocking condition".into()),
            },
            GovernedActionDescriptorV1 {
                action_id: "retry_micro_task".into(),
                title: "Retry Micro-Task".into(),
                description: Some("Retry a blocked or failed micro-task".into()),
            },
            GovernedActionDescriptorV1 {
                action_id: "resolve_micro_task_blocker".into(),
                title: "Resolve Micro-Task Blocker".into(),
                description: Some("Resolve a micro-task blocker".into()),
            },
        ],
        WorkflowStateFamily::Done => vec![
            GovernedActionDescriptorV1 {
                action_id: "archive".into(),
                title: "Archive".into(),
                description: Some("Archive the completed record".into()),
            },
            GovernedActionDescriptorV1 {
                action_id: "reopen".into(),
                title: "Reopen".into(),
                description: Some("Reopen for further work".into()),
            },
            GovernedActionDescriptorV1 {
                action_id: "archive_work_packet".into(),
                title: "Archive Work Packet".into(),
                description: Some("Archive a completed work packet".into()),
            },
            GovernedActionDescriptorV1 {
                action_id: "archive_micro_task".into(),
                title: "Archive Micro-Task".into(),
                description: Some("Archive a completed micro-task".into()),
            },
        ],
        WorkflowStateFamily::Canceled => vec![
            GovernedActionDescriptorV1 {
                action_id: "archive".into(),
                title: "Archive".into(),
                description: Some("Archive the cancelled record".into()),
            },
            GovernedActionDescriptorV1 {
                action_id: "reopen".into(),
                title: "Reopen".into(),
                description: Some("Reopen the cancelled record".into()),
            },
            GovernedActionDescriptorV1 {
                action_id: "close_work_packet".into(),
                title: "Close Work Packet".into(),
                description: Some("Close a canceled work packet".into()),
            },
            GovernedActionDescriptorV1 {
                action_id: "review_skipped_micro_task".into(),
                title: "Review Skipped Micro-Task".into(),
                description: Some("Review a skipped micro-task before closure".into()),
            },
        ],
        WorkflowStateFamily::Archived => vec![],
    }
}

/// Returns all registered action IDs for a workflow state family.
pub fn governed_action_ids_for_family(family: WorkflowStateFamily) -> Vec<String> {
    governed_action_descriptors_for_family(family)
        .into_iter()
        .map(|d| d.action_id)
        .collect()
}

// ── Base labels for degradation [v02.171] ──

/// Base human-readable label for a workflow state family.
/// Used as degradation target when project-profile extensions are unknown.
pub fn base_workflow_family_label(family: WorkflowStateFamily) -> &'static str {
    match family {
        WorkflowStateFamily::Intake => "Intake",
        WorkflowStateFamily::Ready => "Ready",
        WorkflowStateFamily::Active => "Active",
        WorkflowStateFamily::Waiting => "Waiting",
        WorkflowStateFamily::Review => "Review",
        WorkflowStateFamily::Approval => "Approval",
        WorkflowStateFamily::Validation => "Validation",
        WorkflowStateFamily::Blocked => "Blocked",
        WorkflowStateFamily::Done => "Done",
        WorkflowStateFamily::Canceled => "Canceled",
        WorkflowStateFamily::Archived => "Archived",
    }
}

/// Base human-readable label for a queue reason code.
/// Used as degradation target when project-profile extensions are unknown.
pub fn base_queue_reason_label(reason: WorkflowQueueReasonCode) -> &'static str {
    match reason {
        WorkflowQueueReasonCode::NewUntriaged => "new, untriaged",
        WorkflowQueueReasonCode::DependencyWait => "dependency wait",
        WorkflowQueueReasonCode::ReadyForLocalSmallModel => "ready for local model",
        WorkflowQueueReasonCode::ReadyForCloudModel => "ready for cloud model",
        WorkflowQueueReasonCode::ReadyForHuman => "ready for human",
        WorkflowQueueReasonCode::ReviewWait => "review wait",
        WorkflowQueueReasonCode::ApprovalWait => "approval wait",
        WorkflowQueueReasonCode::ValidationWait => "validation wait",
        WorkflowQueueReasonCode::MailboxResponseWait => "mailbox response wait",
        WorkflowQueueReasonCode::TimerWait => "timer wait",
        WorkflowQueueReasonCode::BlockedMissingContext => "blocked: missing context",
        WorkflowQueueReasonCode::BlockedPolicy => "blocked: policy",
        WorkflowQueueReasonCode::BlockedCapability => "blocked: capability",
        WorkflowQueueReasonCode::BlockedError => "blocked: error",
    }
}

// ── Mailbox-aware queue reason resolution [v02.171] ──

/// Resolves queue_reason_code accounting for mailbox-linked wait state.
/// Per v02.171: mailbox-linked waits MUST remain visible as
/// queue_reason_code=mailbox_response_wait, but the mailbox thread
/// MUST NOT become the authority for the linked record's state family.
pub fn resolve_queue_reason_with_mailbox_context(
    base_reason: WorkflowQueueReasonCode,
    has_pending_mailbox_wait: bool,
) -> WorkflowQueueReasonCode {
    if has_pending_mailbox_wait {
        WorkflowQueueReasonCode::MailboxResponseWait
    } else {
        base_reason
    }
}

// ── Workflow transition matrix [v02.172] ──

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkflowTransitionRuleV1 {
    pub rule_id: String,
    pub from_family: WorkflowStateFamily,
    pub to_family: WorkflowStateFamily,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Returns all valid transition rules originating from the given family.
/// Portable across Work Packets, Micro-Tasks, Task Board rows, and Role Mailbox-linked waits.
pub fn transition_rules_for_family(family: WorkflowStateFamily) -> Vec<WorkflowTransitionRuleV1> {
    let pairs: &[(WorkflowStateFamily, &str)] = match family {
        WorkflowStateFamily::Intake => &[
            (WorkflowStateFamily::Ready, "triage completes"),
            (WorkflowStateFamily::Blocked, "blocked during triage"),
            (WorkflowStateFamily::Canceled, "canceled before start"),
        ],
        WorkflowStateFamily::Ready => &[
            (WorkflowStateFamily::Active, "work started"),
            (WorkflowStateFamily::Blocked, "blocked before start"),
            (WorkflowStateFamily::Canceled, "canceled before start"),
        ],
        WorkflowStateFamily::Active => &[
            (WorkflowStateFamily::Waiting, "awaiting dependency or response"),
            (WorkflowStateFamily::Review, "submitted for review"),
            (WorkflowStateFamily::Blocked, "blocked during work"),
            (WorkflowStateFamily::Done, "work completed"),
            (WorkflowStateFamily::Canceled, "canceled during work"),
        ],
        WorkflowStateFamily::Waiting => &[
            (WorkflowStateFamily::Active, "wait resolved"),
            (WorkflowStateFamily::Blocked, "blocked while waiting"),
            (WorkflowStateFamily::Canceled, "canceled while waiting"),
        ],
        WorkflowStateFamily::Review => &[
            (WorkflowStateFamily::Active, "changes requested"),
            (WorkflowStateFamily::Approval, "review approved"),
            (WorkflowStateFamily::Blocked, "blocked during review"),
        ],
        WorkflowStateFamily::Approval => &[
            (WorkflowStateFamily::Active, "approval rejected"),
            (WorkflowStateFamily::Validation, "approval granted"),
            (WorkflowStateFamily::Blocked, "blocked during approval"),
        ],
        WorkflowStateFamily::Validation => &[
            (WorkflowStateFamily::Done, "validation passed"),
            (WorkflowStateFamily::Active, "validation failed, repair"),
            (WorkflowStateFamily::Blocked, "blocked during validation"),
        ],
        WorkflowStateFamily::Blocked => &[
            (WorkflowStateFamily::Ready, "unblocked to ready"),
            (WorkflowStateFamily::Active, "unblocked to active"),
            (WorkflowStateFamily::Canceled, "canceled while blocked"),
        ],
        WorkflowStateFamily::Done => &[
            (WorkflowStateFamily::Archived, "archived after completion"),
            (WorkflowStateFamily::Active, "reopened from done"),
        ],
        WorkflowStateFamily::Canceled => &[
            (WorkflowStateFamily::Ready, "reopened from canceled"),
            (WorkflowStateFamily::Archived, "archived after cancellation"),
        ],
        WorkflowStateFamily::Archived => &[],
    };
    pairs
        .iter()
        .map(|(to, desc)| WorkflowTransitionRuleV1 {
            rule_id: format!(
                "transition:{}_{}", family_id_segment(family), family_id_segment(*to)
            ),
            from_family: family,
            to_family: *to,
            description: Some(desc.to_string()),
        })
        .collect()
}

pub fn transition_rule_ids_for_family(family: WorkflowStateFamily) -> Vec<String> {
    transition_rules_for_family(family)
        .into_iter()
        .map(|r| r.rule_id)
        .collect()
}

// ── Queue automation rules [v02.172] ──

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QueueAutomationTrigger {
    DependencyCleared,
    MailboxResponseReceived,
    ValidationPassed,
    RetryTimerElapsed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QueueAutomationRuleV1 {
    pub rule_id: String,
    pub trigger: QueueAutomationTrigger,
    pub from_reason: WorkflowQueueReasonCode,
    pub to_reason: WorkflowQueueReasonCode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

pub fn queue_automation_rules() -> Vec<QueueAutomationRuleV1> {
    vec![
        QueueAutomationRuleV1 {
            rule_id: "automation:dependency_cleared".to_string(),
            trigger: QueueAutomationTrigger::DependencyCleared,
            from_reason: WorkflowQueueReasonCode::DependencyWait,
            to_reason: WorkflowQueueReasonCode::ReadyForHuman,
            description: Some("dependency resolved, return to ready queue".to_string()),
        },
        QueueAutomationRuleV1 {
            rule_id: "automation:mailbox_response_received".to_string(),
            trigger: QueueAutomationTrigger::MailboxResponseReceived,
            from_reason: WorkflowQueueReasonCode::MailboxResponseWait,
            to_reason: WorkflowQueueReasonCode::ReadyForHuman,
            description: Some("mailbox response received, return to ready queue".to_string()),
        },
        QueueAutomationRuleV1 {
            rule_id: "automation:validation_passed".to_string(),
            trigger: QueueAutomationTrigger::ValidationPassed,
            from_reason: WorkflowQueueReasonCode::ValidationWait,
            to_reason: WorkflowQueueReasonCode::ReadyForHuman,
            description: Some("validation passed, return to ready queue for closeout".to_string()),
        },
        QueueAutomationRuleV1 {
            rule_id: "automation:retry_timer_elapsed".to_string(),
            trigger: QueueAutomationTrigger::RetryTimerElapsed,
            from_reason: WorkflowQueueReasonCode::TimerWait,
            to_reason: WorkflowQueueReasonCode::ReadyForLocalSmallModel,
            description: Some("retry timer elapsed, return to model queue".to_string()),
        },
    ]
}

pub fn queue_automation_rule_ids_for_reason(
    reason: WorkflowQueueReasonCode,
) -> Vec<String> {
    queue_automation_rules()
        .into_iter()
        .filter(|r| r.from_reason == reason)
        .map(|r| r.rule_id)
        .collect()
}

// ── Executor eligibility policies [v02.172] ──

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutorKind {
    Operator,
    LocalSmallModel,
    CloudModel,
    WorkflowEngine,
    Reviewer,
    Governance,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutorEligibilityPolicyV1 {
    pub policy_id: String,
    pub executor_kind: ExecutorKind,
    pub eligible_families: Vec<WorkflowStateFamily>,
    pub requires_compact_summary: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

pub fn executor_eligibility_policies() -> Vec<ExecutorEligibilityPolicyV1> {
    vec![
        ExecutorEligibilityPolicyV1 {
            policy_id: "eligibility:operator".to_string(),
            executor_kind: ExecutorKind::Operator,
            eligible_families: vec![
                WorkflowStateFamily::Intake, WorkflowStateFamily::Ready,
                WorkflowStateFamily::Active, WorkflowStateFamily::Waiting,
                WorkflowStateFamily::Review, WorkflowStateFamily::Approval,
                WorkflowStateFamily::Validation, WorkflowStateFamily::Blocked,
                WorkflowStateFamily::Done, WorkflowStateFamily::Canceled,
                WorkflowStateFamily::Archived,
            ],
            requires_compact_summary: false,
            description: Some("operator can act on any workflow state".to_string()),
        },
        ExecutorEligibilityPolicyV1 {
            policy_id: "eligibility:local_small_model".to_string(),
            executor_kind: ExecutorKind::LocalSmallModel,
            eligible_families: vec![WorkflowStateFamily::Ready],
            requires_compact_summary: true,
            description: Some(
                "local small model requires ready-family state and compact summary".to_string(),
            ),
        },
        ExecutorEligibilityPolicyV1 {
            policy_id: "eligibility:cloud_model".to_string(),
            executor_kind: ExecutorKind::CloudModel,
            eligible_families: vec![
                WorkflowStateFamily::Ready, WorkflowStateFamily::Active,
                WorkflowStateFamily::Waiting, WorkflowStateFamily::Review,
            ],
            requires_compact_summary: false,
            description: Some("cloud model can act on ready through review states".to_string()),
        },
        ExecutorEligibilityPolicyV1 {
            policy_id: "eligibility:workflow_engine".to_string(),
            executor_kind: ExecutorKind::WorkflowEngine,
            eligible_families: vec![
                WorkflowStateFamily::Intake, WorkflowStateFamily::Ready,
                WorkflowStateFamily::Active, WorkflowStateFamily::Waiting,
                WorkflowStateFamily::Review, WorkflowStateFamily::Approval,
                WorkflowStateFamily::Validation, WorkflowStateFamily::Blocked,
                WorkflowStateFamily::Done, WorkflowStateFamily::Canceled,
                WorkflowStateFamily::Archived,
            ],
            requires_compact_summary: false,
            description: Some("workflow engine handles automated transitions".to_string()),
        },
        ExecutorEligibilityPolicyV1 {
            policy_id: "eligibility:reviewer".to_string(),
            executor_kind: ExecutorKind::Reviewer,
            eligible_families: vec![
                WorkflowStateFamily::Review, WorkflowStateFamily::Approval,
                WorkflowStateFamily::Validation,
            ],
            requires_compact_summary: false,
            description: Some("reviewer can act on review, approval, validation".to_string()),
        },
        ExecutorEligibilityPolicyV1 {
            policy_id: "eligibility:governance".to_string(),
            executor_kind: ExecutorKind::Governance,
            eligible_families: vec![
                WorkflowStateFamily::Approval, WorkflowStateFamily::Validation,
                WorkflowStateFamily::Blocked,
            ],
            requires_compact_summary: false,
            description: Some("governance acts on approval, validation, blocked".to_string()),
        },
    ]
}

pub fn executor_eligibility_policy_ids_for_family(
    family: WorkflowStateFamily,
) -> Vec<String> {
    executor_eligibility_policies()
        .into_iter()
        .filter(|p| p.eligible_families.contains(&family))
        .map(|p| p.policy_id)
        .collect()
}

/// Checks whether a local small model executor is eligible.
/// Per v02.172: local-small-model eligibility MUST require a ready-family state
/// AND a compact summary being available.
pub fn is_local_small_model_eligible(
    family: WorkflowStateFamily,
    has_compact_summary: bool,
) -> bool {
    family == WorkflowStateFamily::Ready && has_compact_summary
}

fn family_id_segment(family: WorkflowStateFamily) -> &'static str {
    match family {
        WorkflowStateFamily::Intake => "intake",
        WorkflowStateFamily::Ready => "ready",
        WorkflowStateFamily::Active => "active",
        WorkflowStateFamily::Waiting => "waiting",
        WorkflowStateFamily::Review => "review",
        WorkflowStateFamily::Approval => "approval",
        WorkflowStateFamily::Validation => "validation",
        WorkflowStateFamily::Blocked => "blocked",
        WorkflowStateFamily::Done => "done",
        WorkflowStateFamily::Canceled => "canceled",
        WorkflowStateFamily::Archived => "archived",
    }
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub transition_rule_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub queue_automation_rule_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub executor_eligibility_policy_ids: Vec<String>,
    pub status: String,
    pub title_or_objective: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blockers: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_action: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary_ref: Option<String>,
}

/// Compact summary contract for DCC and Role Mailbox triage (6.3).
/// Extends the base StructuredCollaborationSummaryV1 envelope with
/// DCC-specific stable identifiers and routing hints so local-small-model
/// routing and operator views can default to this payload first, loading
/// canonical detail records or Markdown sidecars only on demand.
///
/// Both DccCompactSummaryV1 and the canonical detail record MUST share the
/// same record_id, project_profile_kind, and authoritative references so
/// deterministic joins remain possible without transcript reconstruction.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DccCompactSummaryV1 {
    // ── Base envelope (shared with StructuredCollaborationSummaryV1) ──
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
    // ── DCC stable identifiers ──
    pub work_packet_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_board_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_session_id: Option<String>,
    // ── Routing hints for triage ──
    #[serde(default)]
    pub pending_wait_count: u32,
    #[serde(default)]
    pub active_thread_count: u32,
    #[serde(default)]
    pub session_bound: bool,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub transition_rule_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub queue_automation_rule_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub executor_eligibility_policy_ids: Vec<String>,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub transition_rule_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub queue_automation_rule_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub executor_eligibility_policy_ids: Vec<String>,
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
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WorkflowMirrorVerdict {
    Pass,
    Fail,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkflowMirrorGateSummary {
    pub task_board_id: String,
    pub gate_state_ref: String,
    pub pre_work: GateStatus,
    pub post_work: GateStatus,
    #[serde(default)]
    pub verdict: Option<WorkflowMirrorVerdict>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub check_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkflowMirrorGateArtifactV1 {
    pub schema_id: String,
    pub schema_version: String,
    pub wp_id: String,
    pub task_board_id: String,
    pub gate_state_ref: String,
    pub gate_summary: WorkflowMirrorGateSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkflowMirrorActivationSummary {
    pub base_wp_id: String,
    pub work_packet_id: String,
    pub task_board_id: String,
    pub active_packet_ref: String,
    pub traceability_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkflowMirrorActivationArtifactV1 {
    pub schema_id: String,
    pub schema_version: String,
    pub base_wp_id: String,
    pub work_packet_id: String,
    pub task_board_id: String,
    pub active_packet_ref: String,
    pub traceability_ref: String,
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
    #[serde(default)]
    pub gate_summary: Option<WorkflowMirrorGateSummary>,
    #[serde(default)]
    pub activation_summary: Option<WorkflowMirrorActivationSummary>,
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
pub const DCC_COMPACT_SUMMARY_SCHEMA_ID_V1: &str = "hsk.dcc_compact_summary@1";
pub const GOVERNANCE_ARTIFACT_REGISTRY_SCHEMA_ID_V1: &str =
    "hsk.governance_artifact_registry@1";
pub const GOVERNANCE_ARTIFACT_REGISTRY_EXTENSION_SCHEMA_ID_V1: &str =
    "hsk.ext.software_delivery.governance_artifact_registry@1";
pub const GOVERNANCE_ARTIFACT_REGISTRY_SCHEMA_VERSION_V1: &str = "1";
pub const STRUCTURED_COLLABORATION_SCHEMA_VERSION_V1: &str = "1";
pub const ROLE_MAILBOX_EXPORT_SCHEMA_VERSION_V1: &str = "role_mailbox_export_v1";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StructuredCollaborationRecordFamily {
    WorkPacketPacket,
    WorkPacketSummary,
    MicroTaskPacket,
    MicroTaskSummary,
    GovernanceArtifactRegistry,
    TaskBoardEntry,
    TaskBoardIndex,
    TaskBoardView,
    RoleMailboxIndex,
    RoleMailboxThreadLine,
    DccCompactSummary,
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
        StructuredCollaborationRecordFamily::GovernanceArtifactRegistry => {
            StructuredCollaborationSchemaDescriptor {
                family,
                schema_id: GOVERNANCE_ARTIFACT_REGISTRY_SCHEMA_ID_V1,
                schema_version: GOVERNANCE_ARTIFACT_REGISTRY_SCHEMA_VERSION_V1,
                record_kind: "governance_artifact_registry",
                summary_family: None,
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
        StructuredCollaborationRecordFamily::DccCompactSummary => {
            StructuredCollaborationSchemaDescriptor {
                family,
                schema_id: DCC_COMPACT_SUMMARY_SCHEMA_ID_V1,
                schema_version: STRUCTURED_COLLABORATION_SCHEMA_VERSION_V1,
                record_kind: "dcc_compact_summary",
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
    let project_profile_kind = ProjectProfileKind::parse(
        obj.get("project_profile_kind").and_then(Value::as_str).unwrap_or_default(),
    );
    require_non_empty_string(obj.get("updated_at"), "updated_at", &mut result);
    validate_mirror_state(obj.get("mirror_state"), &mut result);
    require_string_array(obj.get("authority_refs"), "authority_refs", &mut result);
    require_string_array(obj.get("evidence_refs"), "evidence_refs", &mut result);
    validate_profile_extension(
        obj.get("profile_extension"),
        project_profile_kind,
        &mut result,
    );
    validate_governed_action_fields(obj, &mut result);

    match family {
        StructuredCollaborationRecordFamily::WorkPacketPacket
        | StructuredCollaborationRecordFamily::MicroTaskPacket => {
            if let Some(summary_path) = obj.get("summary_record_path") {
                require_non_empty_string(Some(summary_path), "summary_record_path", &mut result);
            }
        }
        StructuredCollaborationRecordFamily::GovernanceArtifactRegistry => {}
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
            if let Some(next_action_str) = obj.get("next_action").and_then(Value::as_str) {
                if !next_action_str.is_empty() {
                    if !is_registered_governed_action_id(next_action_str) {
                        result.push_issue(
                            StructuredCollaborationValidationCode::InvalidFieldValue,
                            "next_action",
                            None,
                            Some(next_action_str.to_string()),
                            "next_action must be a registered governed action id",
                        );
                    } else if let Some(family_str) =
                        obj.get("workflow_state_family").and_then(Value::as_str)
                    {
                        if let Ok(family) = serde_json::from_value::<WorkflowStateFamily>(
                            Value::String(family_str.to_string()),
                        ) {
                            if !is_governed_action_id_allowed_for_workflow_family(
                                family,
                                next_action_str,
                            ) {
                                result.push_issue(
                                    StructuredCollaborationValidationCode::InvalidFieldValue,
                                    "next_action",
                                    None,
                                    Some(next_action_str.to_string()),
                                    "next_action must be allowed for the workflow_state_family",
                                );
                            }
                        }
                    }
                }
            }
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
        StructuredCollaborationRecordFamily::DccCompactSummary => {
            require_non_empty_string(obj.get("status"), "status", &mut result);
            require_non_empty_string(
                obj.get("title_or_objective"),
                "title_or_objective",
                &mut result,
            );
            require_non_empty_string(obj.get("work_packet_id"), "work_packet_id", &mut result);
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

pub fn is_registered_governed_action_id(action_id: &str) -> bool {
    [
        WorkflowStateFamily::Intake,
        WorkflowStateFamily::Ready,
        WorkflowStateFamily::Active,
        WorkflowStateFamily::Waiting,
        WorkflowStateFamily::Review,
        WorkflowStateFamily::Approval,
        WorkflowStateFamily::Validation,
        WorkflowStateFamily::Blocked,
        WorkflowStateFamily::Done,
        WorkflowStateFamily::Canceled,
    ]
    .into_iter()
    .flat_map(governed_action_ids_for_family)
    .any(|registered| registered == action_id)
}

pub fn is_governed_action_id_allowed_for_workflow_family(
    family: WorkflowStateFamily,
    action_id: &str,
) -> bool {
    governed_action_ids_for_family(family)
        .iter()
        .any(|allowed| allowed == action_id)
}

pub fn preferred_governed_next_action_for_family(
    family: WorkflowStateFamily,
) -> Option<&'static str> {
    match family {
        WorkflowStateFamily::Intake => Some("triage"),
        WorkflowStateFamily::Ready => Some("start"),
        WorkflowStateFamily::Active => Some("update"),
        WorkflowStateFamily::Waiting => Some("resume"),
        WorkflowStateFamily::Review => Some("review"),
        WorkflowStateFamily::Approval => Some("approve"),
        WorkflowStateFamily::Validation => Some("validate"),
        WorkflowStateFamily::Blocked => Some("unblock"),
        WorkflowStateFamily::Done => Some("archive"),
        WorkflowStateFamily::Canceled => Some("archive"),
        WorkflowStateFamily::Archived => None,
    }
}

fn workflow_state_family_from_obj(
    obj: &serde_json::Map<String, Value>,
) -> Option<WorkflowStateFamily> {
    obj.get("workflow_state_family")
        .cloned()
        .and_then(|family| serde_json::from_value(family).ok())
}

fn validate_governed_action_fields(
    obj: &serde_json::Map<String, Value>,
    result: &mut StructuredCollaborationValidationResult,
) {
    let family = workflow_state_family_from_obj(obj);

    if let Some(allowed_action_ids) = obj.get("allowed_action_ids").and_then(Value::as_array) {
        for (index, action_id_value) in allowed_action_ids.iter().enumerate() {
            let Some(action_id) = action_id_value.as_str() else {
                continue;
            };
            let field = format!("allowed_action_ids[{index}]");
            if !is_registered_governed_action_id(action_id) {
                result.push_issue(
                    StructuredCollaborationValidationCode::InvalidFieldValue,
                    field,
                    Some("registered governed action id".to_string()),
                    Some(action_id.to_string()),
                    "allowed_action_ids must contain registered governed action ids",
                );
                continue;
            }
            if let Some(family) = family {
                if !is_governed_action_id_allowed_for_workflow_family(family, action_id) {
                    result.push_issue(
                        StructuredCollaborationValidationCode::InvalidFieldValue,
                        field,
                        Some(format!(
                            "action allowed for workflow_state_family={family:?}"
                        )),
                        Some(action_id.to_string()),
                        "allowed_action_ids must match workflow_state_family",
                    );
                }
            }
        }
    }

    if let Some(action_id) = obj.get("next_action").and_then(Value::as_str) {
        if !is_registered_governed_action_id(action_id) {
            result.push_issue(
                StructuredCollaborationValidationCode::InvalidFieldValue,
                "next_action",
                Some("registered governed action id".to_string()),
                Some(action_id.to_string()),
                "next_action must be a registered governed action id",
            );
            return;
        }
        if let Some(family) = family {
            if !is_governed_action_id_allowed_for_workflow_family(family, action_id) {
                result.push_issue(
                    StructuredCollaborationValidationCode::InvalidFieldValue,
                    "next_action",
                    Some(format!(
                        "action allowed for workflow_state_family={family:?}"
                    )),
                    Some(action_id.to_string()),
                    "next_action must match workflow_state_family",
                );
            }
        }
    }
}

fn validate_profile_extension(
    value: Option<&Value>,
    project_profile_kind: Option<ProjectProfileKind>,
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

    let extension_schema_id = obj.get("extension_schema_id").and_then(Value::as_str);
    let extension_schema_version = obj.get("extension_schema_version").and_then(Value::as_str);

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
    if let Some(extension_schema_id) = extension_schema_id {
        if !is_registered_profile_extension_schema(project_profile_kind, extension_schema_id) {
            result.push_issue(
                StructuredCollaborationValidationCode::InvalidFieldValue,
                "profile_extension.extension_schema_id",
                Some("registered profile extension schema id".to_string()),
                Some(extension_schema_id.to_string()),
                "profile_extension schema id must be registered for the project profile kind",
            );
        }
    }
    if extension_schema_id == Some(GOVERNANCE_ARTIFACT_REGISTRY_EXTENSION_SCHEMA_ID_V1) {
        if !matches!(project_profile_kind, Some(ProjectProfileKind::SoftwareDelivery)) {
            result.push_issue(
                StructuredCollaborationValidationCode::IncompatibleProfileExtension,
                "profile_extension.extension_schema_id",
                Some(ProjectProfileKind::SoftwareDelivery.as_str().to_string()),
                project_profile_kind
                    .map(|kind| kind.as_str().to_string())
                    .or(Some("unknown".to_string())),
                "governance artifact registry extension is only compatible with software_delivery profiles",
            );
        }
        if extension_schema_version != Some(GOVERNANCE_ARTIFACT_REGISTRY_SCHEMA_VERSION_V1) {
            result.push_issue(
                StructuredCollaborationValidationCode::SchemaVersionMismatch,
                "profile_extension.extension_schema_version",
                Some(GOVERNANCE_ARTIFACT_REGISTRY_SCHEMA_VERSION_V1.to_string()),
                extension_schema_version.map(str::to_string),
                "governance artifact registry extension_schema_version does not match expected schema version",
            );
        }
    }
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

fn is_registered_profile_extension_schema(
    project_profile_kind: Option<ProjectProfileKind>,
    extension_schema_id: &str,
) -> bool {
    if extension_schema_id == GOVERNANCE_ARTIFACT_REGISTRY_EXTENSION_SCHEMA_ID_V1 {
        return true;
    }

    match project_profile_kind {
        Some(ProjectProfileKind::SoftwareDelivery) => matches!(
            extension_schema_id,
            "hsk.profile.software_delivery@1" | "handshake.project_profile.software_delivery"
        ),
        Some(ProjectProfileKind::Research) => matches!(
            extension_schema_id,
            "hsk.profile.research@1" | "hsk.profile.research.exploratory@1"
        ),
        _ => false,
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

// ── MT-001: v02.181 software-delivery projection-surface discipline ──────────
//
// Projection-surface discipline forbids Dev Command Center, Task Board mirrors,
// and Role Mailbox chronology from acting as authoritative work meaning for the
// `software_delivery` profile. All authoritative fields MUST be lifted from the
// canonical `StructuredCollaborationSummaryV1` (the runtime-backed record). The
// surface MAY carry advisory display state (mirror sync, board lane, mailbox
// thread refs) but those fields MUST NOT shadow the canonical fields used by
// validators, queues, or governed-action eligibility.
//
// `SoftwareDeliveryProjectionSurfaceV1` is the compact projection record. It
// is specialized for `ProjectProfileKind::SoftwareDelivery`; the derivation
// function rejects other profiles to keep this surface profile-scoped.

pub const SOFTWARE_DELIVERY_PROJECTION_SURFACE_SCHEMA_ID_V1: &str =
    "hsk.ext.software_delivery.projection_surface@1";
pub const SOFTWARE_DELIVERY_PROJECTION_SURFACE_SCHEMA_VERSION_V1: &str = "1";
pub const SOFTWARE_DELIVERY_PROJECTION_SURFACE_RECORD_KIND: &str =
    "software_delivery_projection_surface";

pub const GOVERNED_ACTION_PREVIEW_SCHEMA_ID_V1: &str =
    "hsk.ext.software_delivery.governed_action_preview@1";
pub const GOVERNED_ACTION_PREVIEW_SCHEMA_VERSION_V1: &str = "1";
pub const GOVERNED_ACTION_PREVIEW_RECORD_KIND: &str = "governed_action_preview";

/// Eligibility verdict for a governed-action preview. UI surfaces (DCC quick
/// actions, Task Board row actions, Role Mailbox escalation controls) MUST
/// surface ineligibility reasons before allowing operator escalation, so a
/// preview never silently gates around policy/approval/evidence checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GovernedActionEligibility {
    /// Action belongs to the canonical workflow_state_family registry AND no
    /// canonical blockers are present.
    Eligible,
    /// Action is not registered in `governed_action_ids_for_family` for the
    /// canonical workflow_state_family. UI surfaces MUST refuse to escalate.
    IneligibleOutOfFamily,
    /// Action belongs to the canonical workflow_state_family registry but
    /// canonical blockers exist; UI surfaces MUST surface the blockers and
    /// require resolution before escalation.
    IneligibleBlocked,
}

/// Read-only preview of a governed action that DCC quick actions, Task Board
/// row actions, and Role Mailbox escalation controls inspect before requesting
/// any mutation. Constructing a preview MUST NOT mutate canonical runtime
/// state, evidence, or overlay records: it lifts authoritative refs verbatim
/// from `StructuredCollaborationSummaryV1`.
///
/// Per Master Spec v02.181 projection-surface discipline (packet contract row
/// "Governed action preview payload"): every actionable surface that may
/// trigger a governed action SHOULD render with a preview payload carrying
/// `action_request_id`, target record refs, eligibility, blockers, and
/// required evidence refs so policy/approval/evidence gates remain explicit
/// before any state mutation runs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GovernedActionPreviewV1 {
    pub schema_id: String,
    pub schema_version: String,
    pub record_kind: String,
    /// Stable correlation id callers reuse if they decide to escalate the
    /// preview into a real governed action request. Deterministic from the
    /// canonical record_id, workflow_state_family, and action_id so repeated
    /// preview construction does not desync ids across surfaces.
    pub action_request_id: String,
    /// Stable id of the work item this preview targets. Mirrors
    /// `StructuredCollaborationSummaryV1::record_id` for software_delivery.
    pub work_packet_id: String,
    /// Stable id of the canonical workflow run, when bound by the producer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_run_id: Option<String>,
    /// Governed action id from `governed_action_ids_for_family` for the
    /// canonical workflow_state_family.
    pub action_id: String,
    /// Workflow state family the preview was derived under (lifted from the
    /// canonical summary).
    pub workflow_state_family: WorkflowStateFamily,
    /// Canonical record refs the action would touch on resolution; lifted
    /// from `StructuredCollaborationSummaryV1::authority_refs` so previews
    /// never invent fresh authority paths.
    #[serde(default)]
    pub target_record_refs: Vec<String>,
    /// Eligibility verdict at preview-construction time. UI surfaces MUST
    /// surface ineligibility reasons before allowing operator escalation.
    pub eligibility: GovernedActionEligibility,
    /// Canonical blockers that drive `IneligibleBlocked`; empty when
    /// canonical reports no blockers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blockers: Vec<String>,
    /// Canonical evidence refs that the validator/approver gate would
    /// require; lifted from `StructuredCollaborationSummaryV1::evidence_refs`
    /// so previews stay aligned with the canonical evidence contract.
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

/// Compact projection-surface payload for one software-delivery work item.
///
/// The fields above the `// advisory display state` divider are the
/// authoritative join surface; they MUST always reflect the canonical
/// `StructuredCollaborationSummaryV1`. The fields below the divider are
/// advisory display state borrowed from board mirrors and mailbox chronology;
/// they MUST NOT be consumed as authority by validators, queue automation, or
/// governed-action eligibility logic.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SoftwareDeliveryProjectionSurfaceV1 {
    pub schema_id: String,
    pub schema_version: String,
    pub record_id: String,
    pub record_kind: String,
    pub project_profile_kind: ProjectProfileKind,
    pub updated_at: String,
    /// Stable identifier for the underlying work item (mirrors
    /// `StructuredCollaborationSummaryV1::record_id` for software_delivery).
    pub work_packet_id: String,
    /// Stable identifier for the canonical workflow run, when bound.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_run_id: Option<String>,
    /// Stable identifier for the bound model session, when bound.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_session_id: Option<String>,
    /// Stable identifier for the linked task board, when projected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_board_id: Option<String>,
    /// Authoritative workflow state family (lifted from canonical summary).
    pub workflow_state_family: WorkflowStateFamily,
    /// Authoritative queue reason code (lifted from canonical summary).
    pub queue_reason_code: WorkflowQueueReasonCode,
    /// Authoritative governed-action ids (lifted from canonical summary).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_action_ids: Vec<String>,
    /// Authoritative human-readable status (lifted from canonical summary).
    pub status: String,
    /// Authoritative title or objective (lifted from canonical summary).
    pub title_or_objective: String,
    /// Authoritative blockers (lifted from canonical summary).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blockers: Vec<String>,
    /// Authoritative next-action hint (lifted from canonical summary).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_action: Option<String>,
    /// Authoritative summary ref (lifted from canonical summary).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary_ref: Option<String>,
    /// Authoritative authority refs (lifted from canonical summary).
    #[serde(default)]
    pub authority_refs: Vec<String>,
    /// Authoritative evidence refs (lifted from canonical summary).
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    /// Governed-action previews exposed to DCC quick actions, Task Board row
    /// actions, and Role Mailbox escalation controls. One preview per entry
    /// in `allowed_action_ids` (when registered for the canonical workflow
    /// state family); each preview carries `action_request_id`, target record
    /// refs, eligibility, blockers, and evidence refs so policy/approval/
    /// evidence gates remain explicit before any mutation. Constructing
    /// previews MUST NOT mutate canonical runtime state.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub governed_action_previews: Vec<GovernedActionPreviewV1>,
    // ── MT-004: software-delivery overlay extension surfacing ────────────────
    /// Stable id of the bound workflow binding (when one is bound).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_binding_id: Option<String>,
    /// Derived workflow binding lifecycle state. Always lifted from canonical
    /// runtime truth via
    /// `derive_software_delivery_workflow_binding_state`; never edited from
    /// packet prose, board lane, or mailbox chronology.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_binding_state: Option<SoftwareDeliveryWorkflowBindingState>,
    /// Stable record id of the bound canonical claim/lease overlay record
    /// (when ownership is held). MUST resolve to a canonical
    /// `<gov_root>/claim_leases/<wp_id>/<claim_id>.json` path under
    /// `claim_lease_record_ref`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claim_lease_record_id: Option<String>,
    /// Canonical runtime path for the bound claim/lease overlay record.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claim_lease_record_ref: Option<String>,
    /// Stable record ids of canonical queued-instruction overlay records
    /// (deferred steering intent), sorted and deduped.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub queued_instruction_record_ids: Vec<String>,
    /// Canonical runtime paths for queued-instruction overlay records,
    /// aligned 1:1 with `queued_instruction_record_ids`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub queued_instruction_record_refs: Vec<String>,
    // ── advisory display state (MUST NOT carry authority) ────────────────────
    /// Sync state of the human-readable Markdown mirror, advisory only.
    #[serde(default)]
    pub advisory_mirror_state: MirrorSyncState,
    /// Task-board lane the row currently sits in, advisory only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub advisory_board_lane_id: Option<String>,
    /// Task-board display token (e.g. status string), advisory only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub advisory_board_status_text: Option<String>,
    /// Stable thread ids linked to this work item, advisory only.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub advisory_role_mailbox_thread_ids: Vec<String>,
}

/// True when a mirror sync state is advisory and MUST NOT be treated as
/// authoritative for runtime decisions. Per v02.181: stale, advisory-edit, and
/// normalization-required mirrors are advisory display state.
pub fn mirror_state_is_advisory_only(state: MirrorSyncState) -> bool {
    matches!(
        state,
        MirrorSyncState::Stale
            | MirrorSyncState::AdvisoryEdit
            | MirrorSyncState::NormalizationRequired
    )
}

/// Build a deterministic `action_request_id` from canonical inputs so repeated
/// preview construction across surfaces produces identical correlation ids.
fn build_governed_action_request_id(
    record_id: &str,
    workflow_state_family: WorkflowStateFamily,
    action_id: &str,
) -> String {
    let family_token = match workflow_state_family {
        WorkflowStateFamily::Intake => "intake",
        WorkflowStateFamily::Ready => "ready",
        WorkflowStateFamily::Active => "active",
        WorkflowStateFamily::Waiting => "waiting",
        WorkflowStateFamily::Review => "review",
        WorkflowStateFamily::Approval => "approval",
        WorkflowStateFamily::Validation => "validation",
        WorkflowStateFamily::Blocked => "blocked",
        WorkflowStateFamily::Done => "done",
        WorkflowStateFamily::Canceled => "canceled",
        WorkflowStateFamily::Archived => "archived",
    };
    format!("preview:{record_id}:{family_token}:{action_id}")
}

/// Build a single governed-action preview for `action_id` against the canonical
/// summary. Returns `None` when `canonical.project_profile_kind` is not
/// `SoftwareDelivery` or when `action_id` is not registered in any
/// workflow_state_family allowlist (`is_registered_governed_action_id`). The
/// preview reads only canonical inputs and MUST NOT mutate them.
///
/// Eligibility resolves as:
/// - `IneligibleOutOfFamily` when `action_id` is not allowed for
///   `canonical.workflow_state_family` (UI surfaces MUST refuse to escalate).
/// - `IneligibleBlocked` when canonical reports unresolved blockers.
/// - `Eligible` otherwise.
pub fn derive_governed_action_preview(
    canonical: &StructuredCollaborationSummaryV1,
    workflow_run_id: Option<&str>,
    action_id: &str,
) -> Option<GovernedActionPreviewV1> {
    if canonical.project_profile_kind != ProjectProfileKind::SoftwareDelivery {
        return None;
    }
    if !is_registered_governed_action_id(action_id) {
        return None;
    }
    let family_allowed = is_governed_action_id_allowed_for_workflow_family(
        canonical.workflow_state_family,
        action_id,
    );
    let eligibility = if !family_allowed {
        GovernedActionEligibility::IneligibleOutOfFamily
    } else if !canonical.blockers.is_empty() {
        GovernedActionEligibility::IneligibleBlocked
    } else {
        GovernedActionEligibility::Eligible
    };
    Some(GovernedActionPreviewV1 {
        schema_id: GOVERNED_ACTION_PREVIEW_SCHEMA_ID_V1.to_string(),
        schema_version: GOVERNED_ACTION_PREVIEW_SCHEMA_VERSION_V1.to_string(),
        record_kind: GOVERNED_ACTION_PREVIEW_RECORD_KIND.to_string(),
        action_request_id: build_governed_action_request_id(
            &canonical.record_id,
            canonical.workflow_state_family,
            action_id,
        ),
        work_packet_id: canonical.record_id.clone(),
        workflow_run_id: workflow_run_id.map(|s| s.to_string()),
        action_id: action_id.to_string(),
        workflow_state_family: canonical.workflow_state_family,
        target_record_refs: canonical.authority_refs.clone(),
        eligibility,
        blockers: canonical.blockers.clone(),
        evidence_refs: canonical.evidence_refs.clone(),
    })
}

/// Build the full set of governed-action previews for a canonical summary.
/// Sources action ids from `canonical.allowed_action_ids` (the authoritative
/// runtime allowlist) so previews never invent actions the canonical summary
/// has not authorized. Unregistered ids are skipped (filtered through
/// `derive_governed_action_preview`'s `is_registered_governed_action_id`
/// guard) so a tampered canonical cannot smuggle an unregistered action into
/// the preview list. Callers that need to preview a candidate action outside
/// the canonical allowlist (e.g. UI exploring a transition) should use
/// `derive_governed_action_preview` directly per id.
pub fn derive_governed_action_previews(
    canonical: &StructuredCollaborationSummaryV1,
    workflow_run_id: Option<&str>,
) -> Vec<GovernedActionPreviewV1> {
    canonical
        .allowed_action_ids
        .iter()
        .filter_map(|id| derive_governed_action_preview(canonical, workflow_run_id, id))
        .collect()
}

/// Resolves `queue_reason_code` accounting for mailbox-linked wait state.
/// Per Master Spec v02.171: when a linked Role Mailbox thread is awaiting a
/// response, `queue_reason_code` MUST surface as `MailboxResponseWait`, but
/// the mailbox thread MUST NOT become the authority for the linked record's
/// state family — `workflow_state_family` is preserved by the caller.
/// Derive the software-delivery projection surface from canonical runtime
/// truth, with optional advisory display state from a Task Board row and
/// linked Role Mailbox thread ids.
///
/// Authoritative fields (workflow_state_family, queue_reason_code,
/// allowed_action_ids, status, title_or_objective, blockers, next_action,
/// summary_ref, authority_refs, evidence_refs) are lifted from `canonical`.
/// Board entry mirror_state, lane id, and status text are passed through as
/// advisory only. Mailbox thread ids are advisory only.
///
/// Returns `None` when `canonical.project_profile_kind` is not
/// `SoftwareDelivery`, or when the optional `task_board_entry` does not refer
/// to the same canonical `record_id` (stable-id join must hold or we refuse
/// to project a misaligned surface).
pub fn derive_software_delivery_projection_surface(
    canonical: &StructuredCollaborationSummaryV1,
    workflow_run_id: Option<&str>,
    model_session_id: Option<&str>,
    task_board_entry: Option<&super::task_board::TaskBoardEntryRecordV1>,
    mailbox_thread_ids: &[String],
) -> Option<SoftwareDeliveryProjectionSurfaceV1> {
    if canonical.project_profile_kind != ProjectProfileKind::SoftwareDelivery {
        return None;
    }
    if let Some(entry) = task_board_entry {
        if entry.work_packet_id != canonical.record_id {
            return None;
        }
        if entry.project_profile_kind != ProjectProfileKind::SoftwareDelivery {
            return None;
        }
    }

    let (advisory_mirror_state, advisory_board_lane_id, advisory_board_status_text, task_board_id) =
        match task_board_entry {
            Some(entry) => (
                entry.mirror_state,
                Some(entry.lane_id.clone()),
                Some(entry.status.clone()),
                Some(entry.task_board_id.clone()),
            ),
            None => (canonical.mirror_state, None, None, None),
        };

    let mut advisory_role_mailbox_thread_ids: Vec<String> = mailbox_thread_ids
        .iter()
        .filter(|id| !id.trim().is_empty())
        .cloned()
        .collect();
    advisory_role_mailbox_thread_ids.sort();
    advisory_role_mailbox_thread_ids.dedup();

    let governed_action_previews =
        derive_governed_action_previews(canonical, workflow_run_id);

    Some(SoftwareDeliveryProjectionSurfaceV1 {
        schema_id: SOFTWARE_DELIVERY_PROJECTION_SURFACE_SCHEMA_ID_V1.to_string(),
        schema_version: SOFTWARE_DELIVERY_PROJECTION_SURFACE_SCHEMA_VERSION_V1.to_string(),
        record_id: canonical.record_id.clone(),
        record_kind: SOFTWARE_DELIVERY_PROJECTION_SURFACE_RECORD_KIND.to_string(),
        project_profile_kind: ProjectProfileKind::SoftwareDelivery,
        updated_at: canonical.updated_at.clone(),
        work_packet_id: canonical.record_id.clone(),
        workflow_run_id: workflow_run_id.map(|s| s.to_string()),
        model_session_id: model_session_id.map(|s| s.to_string()),
        task_board_id,
        workflow_state_family: canonical.workflow_state_family,
        queue_reason_code: canonical.queue_reason_code,
        allowed_action_ids: canonical.allowed_action_ids.clone(),
        status: canonical.status.clone(),
        title_or_objective: canonical.title_or_objective.clone(),
        blockers: canonical.blockers.clone(),
        next_action: canonical.next_action.clone(),
        summary_ref: canonical.summary_ref.clone(),
        authority_refs: canonical.authority_refs.clone(),
        evidence_refs: canonical.evidence_refs.clone(),
        governed_action_previews,
        workflow_binding_id: None,
        workflow_binding_state: None,
        claim_lease_record_id: None,
        claim_lease_record_ref: None,
        queued_instruction_record_ids: Vec::new(),
        queued_instruction_record_refs: Vec::new(),
        advisory_mirror_state,
        advisory_board_lane_id,
        advisory_board_status_text,
        advisory_role_mailbox_thread_ids,
    })
}

/// Validate that a projection surface keeps the canonical authority contract:
/// authoritative fields MUST equal the canonical summary's; advisory display
/// state MUST NOT mutate them. Returns issues that callers (validators,
/// projection builders, board renderers) can surface to operators.
pub fn validate_software_delivery_projection_surface_authority(
    projection: &SoftwareDeliveryProjectionSurfaceV1,
    canonical: &StructuredCollaborationSummaryV1,
) -> StructuredCollaborationValidationResult {
    let mut result = StructuredCollaborationValidationResult::success(
        StructuredCollaborationRecordFamily::WorkPacketSummary,
    );

    if projection.project_profile_kind != ProjectProfileKind::SoftwareDelivery {
        result.push_issue(
            StructuredCollaborationValidationCode::InvalidFieldValue,
            "project_profile_kind",
            Some(ProjectProfileKind::SoftwareDelivery.as_str().to_string()),
            Some(projection.project_profile_kind.as_str().to_string()),
            "software-delivery projection surface only applies to software_delivery profiles",
        );
    }
    if canonical.project_profile_kind != ProjectProfileKind::SoftwareDelivery {
        result.push_issue(
            StructuredCollaborationValidationCode::InvalidFieldValue,
            "canonical.project_profile_kind",
            Some(ProjectProfileKind::SoftwareDelivery.as_str().to_string()),
            Some(canonical.project_profile_kind.as_str().to_string()),
            "canonical summary must be software_delivery for this projection surface",
        );
    }

    if projection.work_packet_id != canonical.record_id {
        result.push_issue(
            StructuredCollaborationValidationCode::SummaryJoinMismatch,
            "work_packet_id",
            Some(canonical.record_id.clone()),
            Some(projection.work_packet_id.clone()),
            "projection surface must join to canonical summary by stable record_id",
        );
    }

    if projection.workflow_state_family != canonical.workflow_state_family {
        result.push_issue(
            StructuredCollaborationValidationCode::SummaryJoinMismatch,
            "workflow_state_family",
            serde_json::to_string(&canonical.workflow_state_family).ok(),
            serde_json::to_string(&projection.workflow_state_family).ok(),
            "projection workflow_state_family must equal canonical summary",
        );
    }
    if projection.queue_reason_code != canonical.queue_reason_code {
        result.push_issue(
            StructuredCollaborationValidationCode::SummaryJoinMismatch,
            "queue_reason_code",
            serde_json::to_string(&canonical.queue_reason_code).ok(),
            serde_json::to_string(&projection.queue_reason_code).ok(),
            "projection queue_reason_code must equal canonical summary",
        );
    }
    if projection.allowed_action_ids != canonical.allowed_action_ids {
        result.push_issue(
            StructuredCollaborationValidationCode::SummaryJoinMismatch,
            "allowed_action_ids",
            serde_json::to_string(&canonical.allowed_action_ids).ok(),
            serde_json::to_string(&projection.allowed_action_ids).ok(),
            "projection allowed_action_ids must equal canonical summary",
        );
    }
    if projection.status != canonical.status {
        result.push_issue(
            StructuredCollaborationValidationCode::SummaryJoinMismatch,
            "status",
            Some(canonical.status.clone()),
            Some(projection.status.clone()),
            "projection status must equal canonical summary",
        );
    }
    if projection.title_or_objective != canonical.title_or_objective {
        result.push_issue(
            StructuredCollaborationValidationCode::SummaryJoinMismatch,
            "title_or_objective",
            Some(canonical.title_or_objective.clone()),
            Some(projection.title_or_objective.clone()),
            "projection title_or_objective must equal canonical summary",
        );
    }
    if projection.summary_ref != canonical.summary_ref {
        result.push_issue(
            StructuredCollaborationValidationCode::SummaryJoinMismatch,
            "summary_ref",
            canonical.summary_ref.clone(),
            projection.summary_ref.clone(),
            "projection summary_ref must equal canonical summary",
        );
    }
    if projection.blockers != canonical.blockers {
        result.push_issue(
            StructuredCollaborationValidationCode::SummaryJoinMismatch,
            "blockers",
            serde_json::to_string(&canonical.blockers).ok(),
            serde_json::to_string(&projection.blockers).ok(),
            "projection blockers must equal canonical summary",
        );
    }
    if projection.next_action != canonical.next_action {
        result.push_issue(
            StructuredCollaborationValidationCode::SummaryJoinMismatch,
            "next_action",
            canonical.next_action.clone(),
            projection.next_action.clone(),
            "projection next_action must equal canonical summary",
        );
    }
    if projection.authority_refs != canonical.authority_refs {
        result.push_issue(
            StructuredCollaborationValidationCode::SummaryJoinMismatch,
            "authority_refs",
            serde_json::to_string(&canonical.authority_refs).ok(),
            serde_json::to_string(&projection.authority_refs).ok(),
            "projection authority_refs must equal canonical summary",
        );
    }
    if projection.evidence_refs != canonical.evidence_refs {
        result.push_issue(
            StructuredCollaborationValidationCode::SummaryJoinMismatch,
            "evidence_refs",
            serde_json::to_string(&canonical.evidence_refs).ok(),
            serde_json::to_string(&projection.evidence_refs).ok(),
            "projection evidence_refs must equal canonical summary",
        );
    }

    result
}

/// MT-002 v02.181: Software-delivery overlay runtime truth specialization.
///
/// Validate that a Task Board entry projection for a software_delivery work
/// item cannot override canonical runtime authority. Authority-carrying
/// fields (`workflow_state_family`, `queue_reason_code`, `allowed_action_ids`)
/// MUST equal the canonical `StructuredCollaborationSummaryV1`. Advisory
/// display state (`mirror_state`, `lane_id`, `status` text) is explicitly
/// allowed to differ and is NOT flagged here -- it is acknowledged as
/// advisory by the v02.181 projection-surface discipline.
///
/// The board entry must join the canonical summary by stable identifier
/// (`board_entry.work_packet_id == canonical.record_id`); a join mismatch
/// is itself an authority issue.
pub fn validate_software_delivery_task_board_projection_against_canonical(
    board_entry: &super::task_board::TaskBoardEntryRecordV1,
    canonical: &StructuredCollaborationSummaryV1,
) -> StructuredCollaborationValidationResult {
    let mut result = StructuredCollaborationValidationResult::success(
        StructuredCollaborationRecordFamily::TaskBoardEntry,
    );

    if board_entry.project_profile_kind != ProjectProfileKind::SoftwareDelivery {
        result.push_issue(
            StructuredCollaborationValidationCode::InvalidFieldValue,
            "project_profile_kind",
            Some(ProjectProfileKind::SoftwareDelivery.as_str().to_string()),
            Some(board_entry.project_profile_kind.as_str().to_string()),
            "this validator only applies to software_delivery board entries",
        );
    }
    if canonical.project_profile_kind != ProjectProfileKind::SoftwareDelivery {
        result.push_issue(
            StructuredCollaborationValidationCode::InvalidFieldValue,
            "canonical.project_profile_kind",
            Some(ProjectProfileKind::SoftwareDelivery.as_str().to_string()),
            Some(canonical.project_profile_kind.as_str().to_string()),
            "canonical summary must be software_delivery for board projection check",
        );
    }

    if board_entry.work_packet_id != canonical.record_id {
        result.push_issue(
            StructuredCollaborationValidationCode::SummaryJoinMismatch,
            "work_packet_id",
            Some(canonical.record_id.clone()),
            Some(board_entry.work_packet_id.clone()),
            "task-board entry must join canonical summary by stable record_id",
        );
    }

    if board_entry.workflow_state_family != canonical.workflow_state_family {
        result.push_issue(
            StructuredCollaborationValidationCode::SummaryJoinMismatch,
            "workflow_state_family",
            serde_json::to_string(&canonical.workflow_state_family).ok(),
            serde_json::to_string(&board_entry.workflow_state_family).ok(),
            "task-board projection must preserve canonical workflow_state_family",
        );
    }
    if board_entry.queue_reason_code != canonical.queue_reason_code {
        result.push_issue(
            StructuredCollaborationValidationCode::SummaryJoinMismatch,
            "queue_reason_code",
            serde_json::to_string(&canonical.queue_reason_code).ok(),
            serde_json::to_string(&board_entry.queue_reason_code).ok(),
            "task-board projection must preserve canonical queue_reason_code",
        );
    }
    if board_entry.allowed_action_ids != canonical.allowed_action_ids {
        result.push_issue(
            StructuredCollaborationValidationCode::SummaryJoinMismatch,
            "allowed_action_ids",
            serde_json::to_string(&canonical.allowed_action_ids).ok(),
            serde_json::to_string(&board_entry.allowed_action_ids).ok(),
            "task-board projection must preserve canonical allowed_action_ids",
        );
    }

    result
}

// ── MT-003: v02.181 software-delivery closeout derivation ────────────────────
//
// Per Master Spec v02.181 sec 2.6.8.8: for `project_profile_kind=software_delivery`,
// authoritative closeout MUST be derived from canonical workflow state,
// validator-gate posture, governed-action resolutions, owner authority, and
// evidence references rather than from packet-local checklist surgery, board
// reshuffling, or manual side-ledger convergence. The derive function below
// REFUSES to produce a closeout posture when the canonical summary lacks
// validator-gate evidence or owner authority -- this is the runtime tripwire
// for "closeout requires gate evidence and owner truth".

pub const SOFTWARE_DELIVERY_CLOSEOUT_POSTURE_SCHEMA_ID_V1: &str =
    "hsk.ext.software_delivery.closeout_posture@1";
pub const SOFTWARE_DELIVERY_CLOSEOUT_POSTURE_SCHEMA_VERSION_V1: &str = "1";
pub const SOFTWARE_DELIVERY_CLOSEOUT_POSTURE_RECORD_KIND: &str =
    "software_delivery_closeout_posture";

/// Coarse closeout-eligibility classification derived from canonical truth.
/// `NotEligible` covers profiles where the work item is not yet in a
/// closeout-relevant family; `PendingGate` covers Validation/Approval/Review
/// states where a validator-gate decision is still pending; `PendingBlockers`
/// covers Done with unresolved blockers; `ReadyToClose` is the only state
/// from which control-plane close transitions are legal.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SoftwareDeliveryCloseoutState {
    NotEligible,
    PendingGate,
    PendingBlockers,
    ReadyToClose,
}

/// Compact closeout-posture payload for one software-delivery work item.
///
/// All fields are derived from canonical runtime truth; nothing here is
/// editable from packet prose, board lane, or mailbox chronology. The payload
/// records the validator-gate evidence ref, owner-authority ref, optional
/// checkpoint lineage, and governed-action resolution refs that the
/// closeout/recovery contract requires (Master Spec v02.181 sec 2.6.8.8).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SoftwareDeliveryCloseoutPostureV1 {
    pub schema_id: String,
    pub schema_version: String,
    pub record_id: String,
    pub record_kind: String,
    pub project_profile_kind: ProjectProfileKind,
    pub work_packet_id: String,
    pub workflow_state_family: WorkflowStateFamily,
    pub queue_reason_code: WorkflowQueueReasonCode,
    pub closeout_state: SoftwareDeliveryCloseoutState,
    /// First canonical validator-gate evidence ref. Required for closeout
    /// legality. Format: `<gov_root>/validator_gates/<wp_id>.json`.
    pub gate_record_ref: String,
    /// First canonical work-packet packet authority ref. Required for
    /// closeout legality (owner-of-record). Format:
    /// `<gov_root>/work_packets/<wp_id>/packet.json`.
    pub owner_authority_ref: String,
    /// Optional canonical checkpoint record ref propagated when the runtime
    /// recovery posture has a bound checkpoint. Format:
    /// `<gov_root>/checkpoints/<id>.json`. Spoofed paths are rejected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checkpoint_record_ref: Option<String>,
    /// Optional canonical checkpoint id (mirrors the trailing path segment
    /// of `checkpoint_record_ref` minus the `.json` suffix).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checkpoint_id: Option<String>,
    /// Governed-action resolution refs that contributed to closeout (e.g.
    /// approve/validate/archive resolutions). Sorted + deduped on derive.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub governed_action_resolution_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub authority_refs: Vec<String>,
    pub unresolved_blockers: Vec<String>,
    pub updated_at: String,
}

fn first_canonical_validator_gate_ref(
    refs: &[String],
    runtime_paths: &crate::runtime_governance::RuntimeGovernancePaths,
    expected_wp_id: &str,
) -> Option<String> {
    refs.iter()
        .find(|reference| runtime_paths.is_canonical_validator_gate_ref(reference, expected_wp_id))
        .cloned()
}

fn first_canonical_owner_packet_ref(
    refs: &[String],
    runtime_paths: &crate::runtime_governance::RuntimeGovernancePaths,
    expected_wp_id: &str,
) -> Option<String> {
    refs.iter()
        .find(|reference| {
            runtime_paths.is_canonical_work_packet_packet_ref(reference, expected_wp_id)
        })
        .cloned()
}

fn first_canonical_checkpoint_ref(
    candidate: Option<&str>,
    runtime_paths: &crate::runtime_governance::RuntimeGovernancePaths,
) -> Option<String> {
    let value = candidate?;
    if runtime_paths.is_canonical_checkpoint_record_ref(value) {
        Some(value.to_string())
    } else {
        None
    }
}

fn checkpoint_id_from_record_ref(
    record_ref: &str,
    runtime_paths: &crate::runtime_governance::RuntimeGovernancePaths,
) -> Option<String> {
    let prefix = runtime_paths.checkpoints_dir_display();
    let normalized_prefix = prefix.trim_end_matches('/').to_string() + "/";
    let normalized = record_ref.replace('\\', "/");
    let after_prefix = normalized.strip_prefix(&normalized_prefix)?;
    let id = after_prefix.strip_suffix(".json")?;
    if id.is_empty() || id.contains('/') {
        return None;
    }
    Some(id.to_string())
}

fn derive_closeout_state_from_canonical(
    canonical: &StructuredCollaborationSummaryV1,
) -> SoftwareDeliveryCloseoutState {
    match canonical.workflow_state_family {
        WorkflowStateFamily::Done => {
            if canonical.blockers.is_empty() {
                SoftwareDeliveryCloseoutState::ReadyToClose
            } else {
                SoftwareDeliveryCloseoutState::PendingBlockers
            }
        }
        WorkflowStateFamily::Validation
        | WorkflowStateFamily::Approval
        | WorkflowStateFamily::Review => SoftwareDeliveryCloseoutState::PendingGate,
        _ => SoftwareDeliveryCloseoutState::NotEligible,
    }
}

/// Derive a software-delivery closeout posture from canonical runtime truth.
///
/// Returns `None` when:
/// - `canonical.project_profile_kind` is not `SoftwareDelivery`, OR
/// - `canonical.evidence_refs` does not include a CANONICAL validator-gate
///   ref under `<gov_root>/validator_gates/<wp_id>.json` (substring spoofs
///   such as `/notes/validator_gates/...` are rejected), OR
/// - `canonical.authority_refs` does not include a CANONICAL work-packet
///   `packet.json` ref under `<gov_root>/work_packets/<wp_id>/packet.json`.
///
/// `checkpoint_record_id_candidate` is the runtime's bound checkpoint id (if
/// any). When supplied, the function builds a canonical checkpoint record
/// ref via `RuntimeGovernancePaths::checkpoint_record_display` and stores it
/// alongside the checkpoint id; non-canonical or empty candidates are
/// dropped. `governed_action_resolution_refs` is the slice of governed-action
/// records that contributed to closeout; the slice is sorted and deduped
/// before storage.
///
/// All authority-carrying fields (workflow_state_family, queue_reason_code,
/// blockers, evidence_refs, authority_refs, updated_at) are lifted verbatim
/// from canonical; nothing here is editable from packet prose, board lane,
/// or mailbox chronology.
pub fn derive_software_delivery_closeout_posture(
    canonical: &StructuredCollaborationSummaryV1,
    runtime_paths: &crate::runtime_governance::RuntimeGovernancePaths,
    checkpoint_record_id_candidate: Option<&str>,
    governed_action_resolution_refs: &[String],
) -> Option<SoftwareDeliveryCloseoutPostureV1> {
    if canonical.project_profile_kind != ProjectProfileKind::SoftwareDelivery {
        return None;
    }
    if canonical.record_id.is_empty() {
        return None;
    }
    // MT-003 stable-id binding: gate evidence and owner authority MUST refer
    // to the SAME canonical record_id; foreign-WP refs (e.g. WP-A summary
    // pointing at WP-B validator gate) cannot satisfy closeout truth.
    let gate_record_ref = first_canonical_validator_gate_ref(
        &canonical.evidence_refs,
        runtime_paths,
        &canonical.record_id,
    )?;
    let owner_authority_ref = first_canonical_owner_packet_ref(
        &canonical.authority_refs,
        runtime_paths,
        &canonical.record_id,
    )?;

    let (checkpoint_record_ref, checkpoint_id) = match checkpoint_record_id_candidate
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(checkpoint_id) => {
            let display = runtime_paths.checkpoint_record_display(checkpoint_id);
            let canonical_ref = first_canonical_checkpoint_ref(Some(&display), runtime_paths);
            match canonical_ref {
                Some(reference) => {
                    let id = checkpoint_id_from_record_ref(&reference, runtime_paths)
                        .unwrap_or_else(|| checkpoint_id.to_string());
                    (Some(reference), Some(id))
                }
                None => (None, None),
            }
        }
        None => (None, None),
    };

    let mut governed_actions: Vec<String> = governed_action_resolution_refs
        .iter()
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .collect();
    governed_actions.sort();
    governed_actions.dedup();

    Some(SoftwareDeliveryCloseoutPostureV1 {
        schema_id: SOFTWARE_DELIVERY_CLOSEOUT_POSTURE_SCHEMA_ID_V1.to_string(),
        schema_version: SOFTWARE_DELIVERY_CLOSEOUT_POSTURE_SCHEMA_VERSION_V1.to_string(),
        record_id: canonical.record_id.clone(),
        record_kind: SOFTWARE_DELIVERY_CLOSEOUT_POSTURE_RECORD_KIND.to_string(),
        project_profile_kind: ProjectProfileKind::SoftwareDelivery,
        work_packet_id: canonical.record_id.clone(),
        workflow_state_family: canonical.workflow_state_family,
        queue_reason_code: canonical.queue_reason_code,
        closeout_state: derive_closeout_state_from_canonical(canonical),
        gate_record_ref,
        owner_authority_ref,
        checkpoint_record_ref,
        checkpoint_id,
        governed_action_resolution_refs: governed_actions,
        evidence_refs: canonical.evidence_refs.clone(),
        authority_refs: canonical.authority_refs.clone(),
        unresolved_blockers: canonical.blockers.clone(),
        updated_at: canonical.updated_at.clone(),
    })
}

// ── MT-004: v02.181 software-delivery overlay extension records ─────────────
//
// Per Master Spec v02.181 sec 2.6.8.8 "Software-delivery overlay extension
// records and lifecycle semantics": canonical runtime state SHOULD expose
// `GovernanceClaimLeaseRecord` and `GovernanceQueuedInstructionRecord` (or
// equivalent stable overlay records) keyed by stable identifiers
// (work_packet_id, workflow_run_id, workflow_binding_id, model_session_id).
// The records carry bounded temporary ownership and queued steering intent so
// the projection surface can expose them by stable id without falling back to
// transcript reconstruction or mailbox chronology. Software-delivery workflow
// bindings SHOULD preserve explicit lifecycle states (`created`, `queued`,
// `claimed`, `node_active`, `approval_wait`, `validation_wait`,
// `closeout_pending`, `settled`, `failed`, `canceled`).

pub const SOFTWARE_DELIVERY_CLAIM_LEASE_SCHEMA_ID_V1: &str =
    "hsk.ext.software_delivery.claim_lease@1";
pub const SOFTWARE_DELIVERY_CLAIM_LEASE_SCHEMA_VERSION_V1: &str = "1";
pub const SOFTWARE_DELIVERY_CLAIM_LEASE_RECORD_KIND: &str = "software_delivery_claim_lease";

pub const SOFTWARE_DELIVERY_QUEUED_INSTRUCTION_SCHEMA_ID_V1: &str =
    "hsk.ext.software_delivery.queued_instruction@1";
pub const SOFTWARE_DELIVERY_QUEUED_INSTRUCTION_SCHEMA_VERSION_V1: &str = "1";
pub const SOFTWARE_DELIVERY_QUEUED_INSTRUCTION_RECORD_KIND: &str =
    "software_delivery_queued_instruction";

/// Takeover policy for a bounded software-delivery claim/lease. Fixed enum so
/// projection surfaces and validators can compare semantics rather than parse
/// free-form prose.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SoftwareDeliveryClaimTakeoverPolicy {
    /// Only the holder may release. No takeover is permitted.
    HolderRelease,
    /// Operator authority may take over even before the lease expires.
    OperatorOverride,
    /// Lease auto-expires; another actor may claim once expired.
    AutoExpire,
}

impl SoftwareDeliveryClaimTakeoverPolicy {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::HolderRelease => "holder_release",
            Self::OperatorOverride => "operator_override",
            Self::AutoExpire => "auto_expire",
        }
    }
}

/// Stable overlay record describing bounded temporary ownership of a
/// software-delivery work item. Keyed by stable identifiers (work_packet_id,
/// workflow_run_id, workflow_binding_id, model_session_id) so projection
/// surfaces and mailbox triage rows can refer to ownership by record_id
/// without inferring it from comments or mailbox order.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GovernanceClaimLeaseRecordV1 {
    pub schema_id: String,
    pub schema_version: String,
    pub record_id: String,
    pub record_kind: String,
    pub project_profile_kind: ProjectProfileKind,
    pub work_packet_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_binding_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_session_id: Option<String>,
    /// Stable id of the actor session that holds the claim/lease.
    pub claim_actor_session_id: String,
    /// RFC3339 timestamp when the claim/lease was opened.
    pub claim_started_at: String,
    /// Optional RFC3339 timestamp when the lease auto-expires (if any).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lease_expires_at: Option<String>,
    pub takeover_policy: SoftwareDeliveryClaimTakeoverPolicy,
    pub updated_at: String,
}

/// Bounded action vocabulary for queued steering instructions. Fixed enum so
/// projections and validators do not parse free-form prose.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SoftwareDeliveryQueuedInstructionAction {
    SteerNext,
    Pause,
    Resume,
    RequestValidator,
    RequestOperator,
    Cancel,
}

impl SoftwareDeliveryQueuedInstructionAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SteerNext => "steer_next",
            Self::Pause => "pause",
            Self::Resume => "resume",
            Self::RequestValidator => "request_validator",
            Self::RequestOperator => "request_operator",
            Self::Cancel => "cancel",
        }
    }
}

/// Stable overlay record describing a queued steering instruction for a
/// software-delivery work item. Keyed by stable identifiers so deferred
/// steering intent stays inspectable without reading transcript order.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GovernanceQueuedInstructionRecordV1 {
    pub schema_id: String,
    pub schema_version: String,
    pub record_id: String,
    pub record_kind: String,
    pub project_profile_kind: ProjectProfileKind,
    pub work_packet_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_binding_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_model_session_id: Option<String>,
    pub instruction_id: String,
    pub requested_action: SoftwareDeliveryQueuedInstructionAction,
    pub queued_at: String,
    pub updated_at: String,
}

/// Software-delivery workflow binding lifecycle state. Mirrors Master Spec
/// v02.181 sec 2.6.8.8: `approval_wait` requires unresolved governed actions,
/// `validation_wait` requires active validator-gate records, `closeout_pending`
/// is derived from canonical runtime truth (not packet prose).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SoftwareDeliveryWorkflowBindingState {
    Created,
    Queued,
    Claimed,
    NodeActive,
    ApprovalWait,
    ValidationWait,
    CloseoutPending,
    Settled,
    Failed,
    Canceled,
}

impl SoftwareDeliveryWorkflowBindingState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Queued => "queued",
            Self::Claimed => "claimed",
            Self::NodeActive => "node_active",
            Self::ApprovalWait => "approval_wait",
            Self::ValidationWait => "validation_wait",
            Self::CloseoutPending => "closeout_pending",
            Self::Settled => "settled",
            Self::Failed => "failed",
            Self::Canceled => "canceled",
        }
    }
}

/// Inputs that describe the ApprovalWait/ValidationWait gate posture for the
/// binding-state derivation. Callers populate these from canonical runtime
/// authority (governed-action registry resolutions, validator-gate records);
/// the derivation function then enforces the spec invariants.
#[derive(Debug, Clone, Copy, Default)]
pub struct SoftwareDeliveryBindingGatePosture {
    /// True when at least one governed action targeting this work item is
    /// unresolved. Required for `approval_wait`.
    pub has_unresolved_governed_actions: bool,
    /// True when the canonical validator-gate record exists for this work
    /// item. Required for `validation_wait`.
    pub has_active_validator_gate: bool,
    /// True when canonical closeout-derivation produced a posture for this
    /// work item (i.e. canonical gate evidence + owner authority truth holds
    /// AND the work-state family is closeout-relevant). Required for
    /// `closeout_pending`/`settled` outcomes.
    pub has_closeout_posture: bool,
    /// True when the underlying workflow run/binding has been marked failed.
    pub workflow_failed: bool,
    /// True when the underlying workflow run/binding has been canceled.
    pub workflow_canceled: bool,
    /// True when the canonical runtime has marked the binding as settled
    /// (final post-close success).
    pub workflow_settled: bool,
}

/// Derive the explicit software-delivery workflow binding lifecycle state
/// from canonical runtime truth. Returns `None` for non-software_delivery
/// profiles. Per Master Spec v02.181 sec 2.6.8.8:
/// - `approval_wait` requires `has_unresolved_governed_actions`,
/// - `validation_wait` requires `has_active_validator_gate`,
/// - `closeout_pending` is derived from canonical runtime truth
///   (`has_closeout_posture` + Done family).
///
/// `claim_lease_present` indicates whether a canonical claim/lease record is
/// bound; in `Ready` family this disambiguates `Queued` vs `Claimed`.
pub fn derive_software_delivery_workflow_binding_state(
    canonical: &StructuredCollaborationSummaryV1,
    gate_posture: SoftwareDeliveryBindingGatePosture,
    claim_lease_present: bool,
) -> Option<SoftwareDeliveryWorkflowBindingState> {
    if canonical.project_profile_kind != ProjectProfileKind::SoftwareDelivery {
        return None;
    }
    if gate_posture.workflow_canceled {
        return Some(SoftwareDeliveryWorkflowBindingState::Canceled);
    }
    if gate_posture.workflow_failed {
        return Some(SoftwareDeliveryWorkflowBindingState::Failed);
    }
    Some(match canonical.workflow_state_family {
        WorkflowStateFamily::Intake => SoftwareDeliveryWorkflowBindingState::Created,
        WorkflowStateFamily::Ready => {
            if claim_lease_present {
                SoftwareDeliveryWorkflowBindingState::Claimed
            } else {
                SoftwareDeliveryWorkflowBindingState::Queued
            }
        }
        WorkflowStateFamily::Active => SoftwareDeliveryWorkflowBindingState::NodeActive,
        WorkflowStateFamily::Waiting => {
            if gate_posture.has_active_validator_gate {
                SoftwareDeliveryWorkflowBindingState::ValidationWait
            } else if gate_posture.has_unresolved_governed_actions {
                SoftwareDeliveryWorkflowBindingState::ApprovalWait
            } else if claim_lease_present {
                SoftwareDeliveryWorkflowBindingState::Claimed
            } else {
                SoftwareDeliveryWorkflowBindingState::Queued
            }
        }
        WorkflowStateFamily::Review | WorkflowStateFamily::Validation => {
            // Spec invariant: `validation_wait` requires an active validator
            // gate record. Without one we cannot promote the binding into
            // `validation_wait`; fall back to the active-node lifecycle so
            // operators see "still working" rather than a phantom gate.
            if gate_posture.has_active_validator_gate {
                SoftwareDeliveryWorkflowBindingState::ValidationWait
            } else {
                SoftwareDeliveryWorkflowBindingState::NodeActive
            }
        }
        WorkflowStateFamily::Approval => {
            // Spec invariant: `approval_wait` requires unresolved governed
            // actions. Without one we hold the binding at NodeActive rather
            // than misrepresent the gate state.
            if gate_posture.has_unresolved_governed_actions {
                SoftwareDeliveryWorkflowBindingState::ApprovalWait
            } else {
                SoftwareDeliveryWorkflowBindingState::NodeActive
            }
        }
        WorkflowStateFamily::Blocked => {
            if gate_posture.has_active_validator_gate {
                SoftwareDeliveryWorkflowBindingState::ValidationWait
            } else if gate_posture.has_unresolved_governed_actions {
                SoftwareDeliveryWorkflowBindingState::ApprovalWait
            } else {
                SoftwareDeliveryWorkflowBindingState::NodeActive
            }
        }
        WorkflowStateFamily::Done => {
            if gate_posture.workflow_settled {
                SoftwareDeliveryWorkflowBindingState::Settled
            } else if gate_posture.has_closeout_posture {
                SoftwareDeliveryWorkflowBindingState::CloseoutPending
            } else {
                SoftwareDeliveryWorkflowBindingState::NodeActive
            }
        }
        WorkflowStateFamily::Canceled => SoftwareDeliveryWorkflowBindingState::Canceled,
        WorkflowStateFamily::Archived => {
            if gate_posture.workflow_settled {
                SoftwareDeliveryWorkflowBindingState::Settled
            } else {
                SoftwareDeliveryWorkflowBindingState::Canceled
            }
        }
    })
}

// ── MT-004: v02.181 software-delivery workflow run lifecycle record ─────────
//
// Per Master Spec v02.181 sec 2.6.8.8 "Software-delivery overlay extension
// records and lifecycle semantics": canonical runtime state SHOULD expose
// the workflow run/binding lifecycle inputs (failed/canceled/settled posture
// and unresolved governed-action posture) as a stable-id-keyed canonical
// record. The projection surface lifecycle helper reads this record from
// `<gov_root>/workflow_runs/<wp_id>.json` so the emitted projection can
// surface explicit `approval_wait`, `failed`, and `settled` binding states
// without falling back to packet prose, board lane, or mailbox chronology.

pub const SOFTWARE_DELIVERY_WORKFLOW_RUN_LIFECYCLE_SCHEMA_ID_V1: &str =
    "hsk.ext.software_delivery.workflow_run_lifecycle@1";
pub const SOFTWARE_DELIVERY_WORKFLOW_RUN_LIFECYCLE_SCHEMA_VERSION_V1: &str = "1";
pub const SOFTWARE_DELIVERY_WORKFLOW_RUN_LIFECYCLE_RECORD_KIND: &str =
    "software_delivery_workflow_run_lifecycle";

/// Canonical workflow run lifecycle posture for one software-delivery work
/// item. The record is keyed by `work_packet_id` and exposes the stable
/// canonical identifiers (`workflow_run_id`, `workflow_binding_id`,
/// `model_session_id`) plus the lifecycle inputs the binding-state derivation
/// requires (`workflow_failed`, `workflow_canceled`, `workflow_settled`,
/// `has_unresolved_governed_actions`).
///
/// Authority discipline: nothing here is editable from packet prose, board
/// lane, or mailbox chronology; the runtime workflow engine writes this
/// record when canonical lifecycle events fire and the production projection
/// surface lifecycle helper reads it to derive the emitted binding state.
/// Absence of the record is legal -- callers default the lifecycle inputs
/// to `false` when the record is missing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SoftwareDeliveryWorkflowRunLifecycleV1 {
    pub schema_id: String,
    pub schema_version: String,
    pub record_id: String,
    pub record_kind: String,
    pub project_profile_kind: ProjectProfileKind,
    pub work_packet_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_binding_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_session_id: Option<String>,
    #[serde(default)]
    pub workflow_failed: bool,
    #[serde(default)]
    pub workflow_canceled: bool,
    #[serde(default)]
    pub workflow_settled: bool,
    #[serde(default)]
    pub has_unresolved_governed_actions: bool,
    pub updated_at: String,
}

/// Derive a software-delivery projection surface that ALSO exposes overlay
/// extension records (claim/lease and queued steering instructions) and the
/// derived workflow binding lifecycle state, keyed by stable identifiers.
///
/// The overlay parameters are advisory inputs into the projection surface;
/// stable-id discipline is enforced. The function returns `None` when:
/// - the canonical summary is not software_delivery,
/// - the optional task-board entry breaks the stable-id join,
/// - the supplied claim/lease record's `work_packet_id` does not match
///   `canonical.record_id` (foreign-WP overlay refused), OR
/// - any supplied queued-instruction record's `work_packet_id` does not
///   match `canonical.record_id` (foreign-WP overlay refused).
///
/// The returned projection surface carries:
/// - `claim_lease_record_id`/`claim_lease_record_ref` (canonical runtime path),
/// - `queued_instruction_record_ids`/`queued_instruction_record_refs` sorted
///   and deduped by stable id (canonical runtime paths), and
/// - `workflow_binding_state`/`workflow_binding_id` derived from canonical
///   runtime truth.
#[allow(clippy::too_many_arguments)]
pub fn derive_software_delivery_projection_surface_with_overlay(
    canonical: &StructuredCollaborationSummaryV1,
    workflow_run_id: Option<&str>,
    workflow_binding_id: Option<&str>,
    model_session_id: Option<&str>,
    task_board_entry: Option<&super::task_board::TaskBoardEntryRecordV1>,
    mailbox_thread_ids: &[String],
    claim_lease: Option<&GovernanceClaimLeaseRecordV1>,
    queued_instructions: &[GovernanceQueuedInstructionRecordV1],
    gate_posture: SoftwareDeliveryBindingGatePosture,
    runtime_paths: &crate::runtime_governance::RuntimeGovernancePaths,
) -> Option<SoftwareDeliveryProjectionSurfaceV1> {
    let mut projection = derive_software_delivery_projection_surface(
        canonical,
        workflow_run_id,
        model_session_id,
        task_board_entry,
        mailbox_thread_ids,
    )?;

    if let Some(claim) = claim_lease {
        if claim.project_profile_kind != ProjectProfileKind::SoftwareDelivery {
            return None;
        }
        if claim.work_packet_id != canonical.record_id {
            return None;
        }
        if claim.record_id.is_empty() || claim.record_id.contains('/') {
            return None;
        }
    }

    for instruction in queued_instructions {
        if instruction.project_profile_kind != ProjectProfileKind::SoftwareDelivery {
            return None;
        }
        if instruction.work_packet_id != canonical.record_id {
            return None;
        }
        if instruction.record_id.is_empty() || instruction.record_id.contains('/') {
            return None;
        }
    }

    let mut queued_pairs: Vec<(String, String)> = queued_instructions
        .iter()
        .map(|instruction| {
            let record_id = instruction.record_id.clone();
            let record_ref = runtime_paths
                .queued_instruction_record_display(&canonical.record_id, &record_id);
            (record_id, record_ref)
        })
        .collect();
    queued_pairs.sort_by(|a, b| a.0.cmp(&b.0));
    queued_pairs.dedup_by(|a, b| a.0 == b.0);

    let queued_instruction_record_ids: Vec<String> =
        queued_pairs.iter().map(|(id, _)| id.clone()).collect();
    let queued_instruction_record_refs: Vec<String> =
        queued_pairs.into_iter().map(|(_, r)| r).collect();

    let claim_lease_record_id = claim_lease.map(|c| c.record_id.clone());
    let claim_lease_record_ref = claim_lease.map(|c| {
        runtime_paths.claim_lease_record_display(&canonical.record_id, &c.record_id)
    });

    let claim_lease_present = claim_lease.is_some();
    let workflow_binding_state =
        derive_software_delivery_workflow_binding_state(canonical, gate_posture, claim_lease_present);

    projection.workflow_binding_id = workflow_binding_id.map(|s| s.to_string());
    projection.workflow_binding_state = workflow_binding_state;
    projection.claim_lease_record_id = claim_lease_record_id;
    projection.claim_lease_record_ref = claim_lease_record_ref;
    projection.queued_instruction_record_ids = queued_instruction_record_ids;
    projection.queued_instruction_record_refs = queued_instruction_record_refs;

    Some(projection)
}

/// Validate that a software-delivery projection surface exposes overlay
/// extension records (claim/lease, queued instructions) by canonical
/// stable-id paths. The base authority validation
/// (`validate_software_delivery_projection_surface_authority`) MUST also
/// pass; this function adds overlay-specific checks:
/// - claim/lease ref shape and stable-id join,
/// - queued-instruction ref shape, stable-id join, and id/ref alignment,
/// - workflow binding state consistency with canonical truth invariants.
pub fn validate_software_delivery_projection_surface_overlay(
    projection: &SoftwareDeliveryProjectionSurfaceV1,
    canonical: &StructuredCollaborationSummaryV1,
    runtime_paths: &crate::runtime_governance::RuntimeGovernancePaths,
    gate_posture: SoftwareDeliveryBindingGatePosture,
) -> StructuredCollaborationValidationResult {
    let mut result = StructuredCollaborationValidationResult::success(
        StructuredCollaborationRecordFamily::WorkPacketSummary,
    );
    if projection.project_profile_kind != ProjectProfileKind::SoftwareDelivery
        || canonical.project_profile_kind != ProjectProfileKind::SoftwareDelivery
    {
        result.push_issue(
            StructuredCollaborationValidationCode::InvalidFieldValue,
            "project_profile_kind",
            Some(ProjectProfileKind::SoftwareDelivery.as_str().to_string()),
            Some(projection.project_profile_kind.as_str().to_string()),
            "overlay validation only applies to software_delivery projection surfaces",
        );
        return result;
    }

    match (
        projection.claim_lease_record_id.as_deref(),
        projection.claim_lease_record_ref.as_deref(),
    ) {
        (Some(id), Some(reference)) => {
            if !runtime_paths.is_canonical_claim_lease_record_ref(
                reference,
                &canonical.record_id,
                id,
            ) {
                result.push_issue(
                    StructuredCollaborationValidationCode::InvalidFieldValue,
                    "claim_lease_record_ref",
                    Some(runtime_paths.claim_lease_record_display(&canonical.record_id, id)),
                    Some(reference.to_string()),
                    "claim/lease record ref must be a canonical \
                     <gov_root>/claim_leases/<wp_id>/<claim_id>.json path \
                     bound to the canonical record_id and claim record_id",
                );
            }
        }
        (Some(_), None) => {
            result.push_issue(
                StructuredCollaborationValidationCode::InvalidFieldValue,
                "claim_lease_record_ref",
                None,
                None,
                "claim/lease record id present but record ref is missing",
            );
        }
        (None, Some(_)) => {
            result.push_issue(
                StructuredCollaborationValidationCode::InvalidFieldValue,
                "claim_lease_record_id",
                None,
                None,
                "claim/lease record ref present but record id is missing",
            );
        }
        (None, None) => {}
    }

    if projection.queued_instruction_record_ids.len()
        != projection.queued_instruction_record_refs.len()
    {
        result.push_issue(
            StructuredCollaborationValidationCode::InvalidFieldValue,
            "queued_instruction_record_refs",
            Some(projection.queued_instruction_record_ids.len().to_string()),
            Some(projection.queued_instruction_record_refs.len().to_string()),
            "queued_instruction_record_ids and queued_instruction_record_refs \
             must have equal length and aligned indices",
        );
    } else {
        for (idx, id) in projection.queued_instruction_record_ids.iter().enumerate() {
            let reference = &projection.queued_instruction_record_refs[idx];
            if !runtime_paths.is_canonical_queued_instruction_record_ref(
                reference,
                &canonical.record_id,
                id,
            ) {
                result.push_issue(
                    StructuredCollaborationValidationCode::InvalidFieldValue,
                    "queued_instruction_record_refs",
                    Some(
                        runtime_paths
                            .queued_instruction_record_display(&canonical.record_id, id),
                    ),
                    Some(reference.clone()),
                    "queued-instruction record ref must be a canonical \
                     <gov_root>/queued_instructions/<wp_id>/<instruction_id>.json path \
                     bound to the canonical record_id and instruction record_id",
                );
            }
        }
        let mut sorted = projection.queued_instruction_record_ids.clone();
        sorted.sort();
        sorted.dedup();
        if sorted != projection.queued_instruction_record_ids {
            result.push_issue(
                StructuredCollaborationValidationCode::InvalidFieldValue,
                "queued_instruction_record_ids",
                Some(serde_json::to_string(&sorted).unwrap_or_default()),
                Some(
                    serde_json::to_string(&projection.queued_instruction_record_ids)
                        .unwrap_or_default(),
                ),
                "queued_instruction_record_ids must be sorted and deduped by stable id",
            );
        }
    }

    let claim_present = projection.claim_lease_record_id.is_some();
    let expected_state =
        derive_software_delivery_workflow_binding_state(canonical, gate_posture, claim_present);
    if projection.workflow_binding_state != expected_state {
        result.push_issue(
            StructuredCollaborationValidationCode::SummaryJoinMismatch,
            "workflow_binding_state",
            expected_state.map(|s| s.as_str().to_string()),
            projection
                .workflow_binding_state
                .map(|s| s.as_str().to_string()),
            "workflow_binding_state must equal the value derived from canonical \
             runtime truth and the supplied gate posture",
        );
    }

    result
}

/// Validate that a canonical software-delivery summary carries the truth
/// required for closeout derivation: a CANONICAL validator-gate evidence ref
/// AND a CANONICAL owner-of-record packet authority ref under the product
/// runtime governance root. Substring-only spoofs are rejected. For
/// non-software_delivery profiles or for software_delivery WPs not yet in a
/// closeout-relevant family (Validation/Approval/Review/Done), this returns
/// success with no issues (the truth is only required when closeout becomes
/// legal).
pub fn validate_software_delivery_closeout_canonical_truth(
    canonical: &StructuredCollaborationSummaryV1,
    runtime_paths: &crate::runtime_governance::RuntimeGovernancePaths,
) -> StructuredCollaborationValidationResult {
    let mut result = StructuredCollaborationValidationResult::success(
        StructuredCollaborationRecordFamily::WorkPacketSummary,
    );
    if canonical.project_profile_kind != ProjectProfileKind::SoftwareDelivery {
        return result;
    }
    let closeout_relevant = matches!(
        canonical.workflow_state_family,
        WorkflowStateFamily::Validation
            | WorkflowStateFamily::Approval
            | WorkflowStateFamily::Review
            | WorkflowStateFamily::Done
    );
    if !closeout_relevant {
        return result;
    }
    // Stable-id binding required: gate evidence and owner authority refs
    // MUST refer to the SAME canonical record_id, not a foreign WP id.
    let expected_wp_id = canonical.record_id.as_str();
    if first_canonical_validator_gate_ref(
        &canonical.evidence_refs,
        runtime_paths,
        expected_wp_id,
    )
    .is_none()
    {
        result.push_issue(
            StructuredCollaborationValidationCode::InvalidFieldValue,
            "evidence_refs",
            Some(runtime_paths.validator_gate_record_display(expected_wp_id)),
            Some(serde_json::to_string(&canonical.evidence_refs).unwrap_or_default()),
            "software-delivery closeout requires a canonical validator-gate evidence ref \
             bound to the canonical record_id",
        );
    }
    if first_canonical_owner_packet_ref(
        &canonical.authority_refs,
        runtime_paths,
        expected_wp_id,
    )
    .is_none()
    {
        result.push_issue(
            StructuredCollaborationValidationCode::InvalidFieldValue,
            "authority_refs",
            Some(runtime_paths.work_packet_packet_display(expected_wp_id)),
            Some(serde_json::to_string(&canonical.authority_refs).unwrap_or_default()),
            "software-delivery closeout requires a canonical owner-of-record packet authority ref \
             bound to the canonical record_id",
        );
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn governance_artifact_registry_record(project_profile_kind: &str, extension_schema_id: &str) -> Value {
        json!({
            "schema_id": GOVERNANCE_ARTIFACT_REGISTRY_SCHEMA_ID_V1,
            "schema_version": GOVERNANCE_ARTIFACT_REGISTRY_SCHEMA_VERSION_V1,
            "record_id": "00000000-0000-0000-0000-000000000001",
            "record_kind": "governance_artifact_registry",
            "project_profile_kind": project_profile_kind,
            "updated_at": "2026-04-05T00:00:00Z",
            "mirror_state": "canonical_only",
            "authority_refs": [],
            "evidence_refs": [],
            "profile_extension": {
                "extension_schema_id": extension_schema_id,
                "extension_schema_version": GOVERNANCE_ARTIFACT_REGISTRY_SCHEMA_VERSION_V1,
                "compatibility": {
                    "mode": "non-breaking"
                }
            }
        })
    }

    #[test]
    fn governance_artifact_registry_has_schema_descriptor() {
        let descriptor = structured_collaboration_schema_descriptor(
            StructuredCollaborationRecordFamily::GovernanceArtifactRegistry,
        );
        assert_eq!(descriptor.schema_id, GOVERNANCE_ARTIFACT_REGISTRY_SCHEMA_ID_V1);
        assert_eq!(
            descriptor.schema_version,
            GOVERNANCE_ARTIFACT_REGISTRY_SCHEMA_VERSION_V1
        );
        assert_eq!(descriptor.record_kind, "governance_artifact_registry");
        assert_eq!(descriptor.summary_family, None);
    }

    #[test]
    fn profile_extension_validation_allows_governance_artifact_registry_extension_for_software_delivery() {
        let record = governance_artifact_registry_record(
            ProjectProfileKind::SoftwareDelivery.as_str(),
            GOVERNANCE_ARTIFACT_REGISTRY_EXTENSION_SCHEMA_ID_V1,
        );
        let result = validate_structured_collaboration_record(
            StructuredCollaborationRecordFamily::GovernanceArtifactRegistry,
            &record,
        );
        assert!(result.ok);
        assert!(
            !result
                .issues
                .iter()
                .any(|issue| issue.code == StructuredCollaborationValidationCode::IncompatibleProfileExtension)
        );
    }

    #[test]
    fn profile_extension_validation_rejects_governance_artifact_registry_extension_for_non_software_delivery() {
        let record = governance_artifact_registry_record(
            ProjectProfileKind::Research.as_str(),
            GOVERNANCE_ARTIFACT_REGISTRY_EXTENSION_SCHEMA_ID_V1,
        );
        let result = validate_structured_collaboration_record(
            StructuredCollaborationRecordFamily::GovernanceArtifactRegistry,
            &record,
        );
        assert!(!result.ok);
        assert!(
            result
                .issues
                .iter()
                .any(|issue| issue.code == StructuredCollaborationValidationCode::IncompatibleProfileExtension)
        );
    }
}
