use super::types::{
    executor_eligibility_policy_ids_for_family, queue_automation_rule_ids_for_reason,
    transition_rule_ids_for_family, validate_software_delivery_task_board_projection_against_canonical,
    MarkdownMirrorContractV1, MirrorSyncState, ProjectProfileKind,
    SoftwareDeliveryCloseoutPostureV1, SoftwareDeliveryCloseoutState,
    SoftwareDeliveryWorkflowBindingState, StructuredCollaborationRecordFamily,
    StructuredCollaborationSummaryV1,
    StructuredCollaborationValidationCode, StructuredCollaborationValidationResult, TaskBoardStatus,
    WorkflowQueueReasonCode, WorkflowStateFamily, STRUCTURED_COLLABORATION_SCHEMA_VERSION_V1,
    STRUCTURED_COLLABORATION_SUMMARY_SCHEMA_ID_V1,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskBoardEntry {
    pub wp_id: String,
    pub token: String,
    pub status: TaskBoardStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SoftwareDeliveryCloseoutProjectionBadgeV1 {
    pub work_packet_id: String,
    pub label: String,
    pub advisory_only: bool,
    pub projection_surface_ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closeout_posture_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closeout_state: Option<SoftwareDeliveryCloseoutState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_binding_state: Option<SoftwareDeliveryWorkflowBindingState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate_record_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_authority_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_action: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_record_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authority_refs: Vec<String>,
}

fn closeout_badge_label(closeout_state: Option<SoftwareDeliveryCloseoutState>) -> &'static str {
    match closeout_state {
        Some(SoftwareDeliveryCloseoutState::ReadyToClose) => "ready_to_close",
        Some(SoftwareDeliveryCloseoutState::PendingBlockers) => "closeout_blocked",
        Some(SoftwareDeliveryCloseoutState::PendingGate) => "closeout_blocked_gate",
        Some(SoftwareDeliveryCloseoutState::NotEligible) => "not_closeout_ready",
        None => "closeout_blocked_no_posture",
    }
}

pub fn build_software_delivery_closeout_projection_badge(
    work_packet_id: &str,
    projection_surface_ref: String,
    closeout_posture_ref: Option<String>,
    closeout_posture: Option<&SoftwareDeliveryCloseoutPostureV1>,
    workflow_binding_state: Option<SoftwareDeliveryWorkflowBindingState>,
) -> SoftwareDeliveryCloseoutProjectionBadgeV1 {
    let closeout_state = closeout_posture.map(|posture| posture.closeout_state);
    let mut source_record_refs = vec![projection_surface_ref.clone()];
    if let Some(reference) = closeout_posture_ref.as_ref() {
        source_record_refs.push(reference.clone());
    }
    source_record_refs.sort();
    source_record_refs.dedup();

    SoftwareDeliveryCloseoutProjectionBadgeV1 {
        work_packet_id: work_packet_id.to_string(),
        label: closeout_badge_label(closeout_state).to_string(),
        advisory_only: true,
        projection_surface_ref,
        closeout_posture_ref,
        closeout_state,
        workflow_binding_state,
        gate_record_ref: closeout_posture.map(|posture| posture.gate_record_ref.clone()),
        owner_authority_ref: closeout_posture.map(|posture| posture.owner_authority_ref.clone()),
        next_action: closeout_posture.and_then(|posture| posture.next_action.clone()),
        source_record_refs,
        evidence_refs: closeout_posture
            .map(|posture| posture.evidence_refs.clone())
            .unwrap_or_default(),
        authority_refs: closeout_posture
            .map(|posture| posture.authority_refs.clone())
            .unwrap_or_default(),
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TaskBoardSections {
    pub stub: Vec<TaskBoardEntry>,
    pub ready: Vec<TaskBoardEntry>,
    pub in_progress: Vec<TaskBoardEntry>,
    pub blocked: Vec<TaskBoardEntry>,
    pub gated: Vec<TaskBoardEntry>,
    pub done: Vec<TaskBoardEntry>,
    pub cancelled: Vec<TaskBoardEntry>,
}

/// Task board entry projection row.
///
/// For `project_profile_kind = software_delivery` (v02.181): the row's
/// `mirror_state`, `lane_id`, and `status` text are advisory display state
/// only. Authoritative work meaning (`workflow_state_family`,
/// `queue_reason_code`, `allowed_action_ids`) MUST be lifted from the
/// canonical `StructuredCollaborationSummaryV1`. See
/// `derive_software_delivery_projection_surface` in `locus::types`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskBoardEntryRecordV1 {
    pub schema_id: String,
    pub schema_version: String,
    pub record_id: String,
    pub record_kind: String,
    pub project_profile_kind: ProjectProfileKind,
    #[serde(default)]
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
    pub task_board_id: String,
    pub work_packet_id: String,
    pub lane_id: String,
    pub display_order: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub view_ids: Vec<String>,
    pub token: String,
    pub status: String,
    pub summary_ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closeout_badge: Option<SoftwareDeliveryCloseoutProjectionBadgeV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskBoardIndexV1 {
    pub schema_id: String,
    pub schema_version: String,
    pub record_id: String,
    pub record_kind: String,
    pub project_profile_kind: ProjectProfileKind,
    #[serde(default)]
    pub profile_extension: Option<Value>,
    pub updated_at: String,
    pub mirror_state: MirrorSyncState,
    #[serde(default)]
    pub authority_refs: Vec<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mirror_contract: Option<MarkdownMirrorContractV1>,
    pub task_board_id: String,
    pub generated_at: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub view_ids: Vec<String>,
    #[serde(default)]
    pub rows: Vec<TaskBoardEntryRecordV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskBoardViewV1 {
    pub schema_id: String,
    pub schema_version: String,
    pub record_id: String,
    pub record_kind: String,
    pub project_profile_kind: ProjectProfileKind,
    #[serde(default)]
    pub profile_extension: Option<Value>,
    pub updated_at: String,
    pub mirror_state: MirrorSyncState,
    #[serde(default)]
    pub authority_refs: Vec<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mirror_contract: Option<MarkdownMirrorContractV1>,
    pub task_board_id: String,
    pub view_id: String,
    pub generated_at: String,
    #[serde(default)]
    pub lane_ids: Vec<String>,
    #[serde(default)]
    pub rows: Vec<TaskBoardEntryRecordV1>,
}

pub fn validate_task_board_entry_authoritative_fields(
    entry: &TaskBoardEntryRecordV1,
    expected_work_packet_id: &str,
    expected_workflow_state_family: WorkflowStateFamily,
    expected_queue_reason_code: WorkflowQueueReasonCode,
    expected_allowed_action_ids: &[String],
) -> StructuredCollaborationValidationResult {
    let mut result = StructuredCollaborationValidationResult::success(
        StructuredCollaborationRecordFamily::TaskBoardEntry,
    );

    if entry.work_packet_id != expected_work_packet_id {
        result.push_issue(
            StructuredCollaborationValidationCode::InvalidFieldValue,
            "work_packet_id",
            Some(expected_work_packet_id.to_string()),
            Some(entry.work_packet_id.clone()),
            "task-board entry must stay linked to the authoritative work packet id",
        );
    }

    if entry.workflow_state_family != expected_workflow_state_family {
        result.push_issue(
            StructuredCollaborationValidationCode::InvalidFieldValue,
            "workflow_state_family",
            Some(
                serde_json::to_string(&expected_workflow_state_family)
                    .unwrap_or_else(|_| format!("{expected_workflow_state_family:?}")),
            ),
            Some(
                serde_json::to_string(&entry.workflow_state_family)
                    .unwrap_or_else(|_| format!("{:?}", entry.workflow_state_family)),
            ),
            "task-board row must preserve the authoritative workflow_state_family",
        );
    }

    if entry.queue_reason_code != expected_queue_reason_code {
        result.push_issue(
            StructuredCollaborationValidationCode::InvalidFieldValue,
            "queue_reason_code",
            Some(
                serde_json::to_string(&expected_queue_reason_code)
                    .unwrap_or_else(|_| format!("{expected_queue_reason_code:?}")),
            ),
            Some(
                serde_json::to_string(&entry.queue_reason_code)
                    .unwrap_or_else(|_| format!("{:?}", entry.queue_reason_code)),
            ),
            "task-board row must preserve the authoritative queue_reason_code",
        );
    }

    if entry.allowed_action_ids != expected_allowed_action_ids {
        result.push_issue(
            StructuredCollaborationValidationCode::InvalidFieldValue,
            "allowed_action_ids",
            Some(
                serde_json::to_string(expected_allowed_action_ids)
                    .unwrap_or_else(|_| format!("{expected_allowed_action_ids:?}")),
            ),
            Some(
                serde_json::to_string(&entry.allowed_action_ids)
                    .unwrap_or_else(|_| format!("{:?}", entry.allowed_action_ids)),
            ),
            "task-board row must preserve the authoritative allowed_action_ids",
        );
    }

    result
}

/// MT-002 v02.181: Production wiring for the software-delivery overlay
/// runtime-truth specialization.
///
/// Synthesizes a minimal `StructuredCollaborationSummaryV1` from the
/// already-extracted authoritative truths produced by the runtime
/// (`work_packet_workflow_state` + `allowed_action_ids`) and dispatches to
/// `validate_software_delivery_task_board_projection_against_canonical`.
/// Non-software_delivery entries return success with no issues so this
/// helper is safe to call unconditionally inside the production task-board
/// materialization loop.
pub fn enforce_software_delivery_task_board_projection_authority(
    entry: &TaskBoardEntryRecordV1,
    expected_record_id: &str,
    expected_workflow_state_family: WorkflowStateFamily,
    expected_queue_reason_code: WorkflowQueueReasonCode,
    expected_allowed_action_ids: Vec<String>,
) -> StructuredCollaborationValidationResult {
    if entry.project_profile_kind != ProjectProfileKind::SoftwareDelivery {
        return StructuredCollaborationValidationResult::success(
            StructuredCollaborationRecordFamily::TaskBoardEntry,
        );
    }
    let canonical_proxy = StructuredCollaborationSummaryV1 {
        schema_id: STRUCTURED_COLLABORATION_SUMMARY_SCHEMA_ID_V1.to_string(),
        schema_version: STRUCTURED_COLLABORATION_SCHEMA_VERSION_V1.to_string(),
        record_id: expected_record_id.to_string(),
        record_kind: "work_packet".to_string(),
        project_profile_kind: ProjectProfileKind::SoftwareDelivery,
        updated_at: entry.updated_at.clone(),
        mirror_state: MirrorSyncState::CanonicalOnly,
        authority_refs: Vec::new(),
        evidence_refs: Vec::new(),
        mirror_contract: None,
        workflow_state_family: expected_workflow_state_family,
        queue_reason_code: expected_queue_reason_code,
        allowed_action_ids: expected_allowed_action_ids,
        transition_rule_ids: transition_rule_ids_for_family(expected_workflow_state_family),
        queue_automation_rule_ids: queue_automation_rule_ids_for_reason(expected_queue_reason_code),
        executor_eligibility_policy_ids: executor_eligibility_policy_ids_for_family(
            expected_workflow_state_family,
        ),
        status: String::new(),
        title_or_objective: String::new(),
        blockers: Vec::new(),
        next_action: None,
        summary_ref: None,
    };
    validate_software_delivery_task_board_projection_against_canonical(entry, &canonical_proxy)
}

impl TaskBoardSections {
    pub fn entries_for_status_mut(&mut self, status: TaskBoardStatus) -> &mut Vec<TaskBoardEntry> {
        match status {
            TaskBoardStatus::Unknown => &mut self.stub,
            TaskBoardStatus::Ready => &mut self.ready,
            TaskBoardStatus::InProgress => &mut self.in_progress,
            TaskBoardStatus::Blocked => &mut self.blocked,
            TaskBoardStatus::Gated => &mut self.gated,
            TaskBoardStatus::Done => &mut self.done,
            TaskBoardStatus::Cancelled => &mut self.cancelled,
        }
    }

    pub fn entries_for_status(&self, status: TaskBoardStatus) -> &Vec<TaskBoardEntry> {
        match status {
            TaskBoardStatus::Unknown => &self.stub,
            TaskBoardStatus::Ready => &self.ready,
            TaskBoardStatus::InProgress => &self.in_progress,
            TaskBoardStatus::Blocked => &self.blocked,
            TaskBoardStatus::Gated => &self.gated,
            TaskBoardStatus::Done => &self.done,
            TaskBoardStatus::Cancelled => &self.cancelled,
        }
    }
}

// ── Ready-query filter extensions (MT-004) ─────────────────────────────

impl TaskBoardIndexV1 {
    /// Return entries whose workflow_state_family matches the given family.
    pub fn entries_by_state_family(
        &self,
        family: WorkflowStateFamily,
    ) -> Vec<&TaskBoardEntryRecordV1> {
        self.rows
            .iter()
            .filter(|e| e.workflow_state_family == family)
            .collect()
    }

    /// Return entries whose queue_reason_code matches the given reason.
    pub fn entries_by_queue_reason(
        &self,
        reason: WorkflowQueueReasonCode,
    ) -> Vec<&TaskBoardEntryRecordV1> {
        self.rows
            .iter()
            .filter(|e| e.queue_reason_code == reason)
            .collect()
    }

    /// Return work-packet IDs for entries matching a given state family.
    /// Generalises the DCC ready_queue to any WorkflowStateFamily.
    pub fn work_packet_ids_by_state_family(
        &self,
        family: WorkflowStateFamily,
    ) -> Vec<String> {
        self.rows
            .iter()
            .filter(|e| e.workflow_state_family == family)
            .map(|e| e.work_packet_id.clone())
            .collect()
    }

    /// Return work-packet IDs for entries matching a given queue reason code.
    pub fn work_packet_ids_by_queue_reason(
        &self,
        reason: WorkflowQueueReasonCode,
    ) -> Vec<String> {
        self.rows
            .iter()
            .filter(|e| e.queue_reason_code == reason)
            .map(|e| e.work_packet_id.clone())
            .collect()
    }

    /// Return entries that expose a specific allowed action ID.
    pub fn entries_by_allowed_action(
        &self,
        action_id: &str,
    ) -> Vec<&TaskBoardEntryRecordV1> {
        self.rows
            .iter()
            .filter(|e| e.allowed_action_ids.iter().any(|a| a == action_id))
            .collect()
    }
}

impl TaskBoardViewV1 {
    /// Return entries whose workflow_state_family matches the given family.
    pub fn entries_by_state_family(
        &self,
        family: WorkflowStateFamily,
    ) -> Vec<&TaskBoardEntryRecordV1> {
        self.rows
            .iter()
            .filter(|e| e.workflow_state_family == family)
            .collect()
    }

    /// Return entries whose queue_reason_code matches the given reason.
    pub fn entries_by_queue_reason(
        &self,
        reason: WorkflowQueueReasonCode,
    ) -> Vec<&TaskBoardEntryRecordV1> {
        self.rows
            .iter()
            .filter(|e| e.queue_reason_code == reason)
            .collect()
    }
}

fn status_for_heading(heading: &str) -> Option<TaskBoardStatus> {
    let heading = heading.trim();
    let heading = heading.strip_prefix("## ")?;
    let title = heading.trim();

    if title.eq_ignore_ascii_case("Ready for Dev") {
        return Some(TaskBoardStatus::Ready);
    }
    if title.eq_ignore_ascii_case("In Progress") {
        return Some(TaskBoardStatus::InProgress);
    }
    if title.eq_ignore_ascii_case("Blocked") {
        return Some(TaskBoardStatus::Blocked);
    }
    if title.eq_ignore_ascii_case("Done") {
        return Some(TaskBoardStatus::Done);
    }
    if title.to_ascii_lowercase().starts_with("stub backlog") {
        return Some(TaskBoardStatus::Unknown);
    }
    if title.to_ascii_lowercase().starts_with("superseded") {
        return Some(TaskBoardStatus::Cancelled);
    }

    None
}

pub fn lane_id_for_status(status: TaskBoardStatus) -> &'static str {
    match status {
        TaskBoardStatus::Unknown => "stub",
        TaskBoardStatus::Ready => "ready",
        TaskBoardStatus::InProgress => "in_progress",
        TaskBoardStatus::Blocked => "blocked",
        TaskBoardStatus::Gated => "gated",
        TaskBoardStatus::Done => "done",
        TaskBoardStatus::Cancelled => "cancelled",
    }
}

pub fn default_view_id() -> &'static str {
    "default"
}

fn parse_entry_line(line: &str) -> Option<(String, String)> {
    let line = line.trim();
    if !line.starts_with("- **[") {
        return None;
    }
    let after_prefix = line.strip_prefix("- **[")?;
    let (wp_id, rest) = after_prefix.split_once("]**")?;
    let rest = rest.trim_start();
    let rest = rest.strip_prefix("-")?.trim_start();
    let token_part = rest.strip_prefix("[")?;
    let (token, _rest) = token_part.split_once(']')?;
    Some((wp_id.trim().to_string(), token.trim().to_string()))
}

pub fn parse_task_board(markdown: &str) -> TaskBoardSections {
    let mut sections = TaskBoardSections::default();
    let mut current: Option<TaskBoardStatus> = None;

    for raw_line in markdown.lines() {
        let line = raw_line.trim_end();
        if let Some(status) = status_for_heading(line) {
            current = Some(status);
            continue;
        }

        let Some(status) = current else { continue };
        let Some((wp_id, token)) = parse_entry_line(line) else {
            continue;
        };

        sections
            .entries_for_status_mut(status)
            .push(TaskBoardEntry {
                wp_id,
                token,
                status,
            });
    }

    sections
}

fn format_entry(entry: &TaskBoardEntry) -> String {
    format!("- **[{}]** - [{}]", entry.wp_id, entry.token)
}

#[derive(Debug, Clone, Copy)]
struct SectionWindow {
    status: TaskBoardStatus,
    start: usize,
}

fn find_section_windows(lines: &[String]) -> Vec<SectionWindow> {
    let mut headings: Vec<(usize, TaskBoardStatus)> = Vec::new();
    for (idx, line) in lines.iter().enumerate() {
        if let Some(status) = status_for_heading(line) {
            headings.push((idx, status));
        }
    }

    let mut windows: Vec<SectionWindow> = Vec::new();
    for (start, status) in headings {
        windows.push(SectionWindow { status, start });
    }
    windows
}

pub fn rewrite_task_board(markdown: &str, canonical: &TaskBoardSections) -> String {
    let mut lines: Vec<String> = markdown.lines().map(|l| l.to_string()).collect();

    let mut windows = find_section_windows(&lines);
    windows.sort_by(|a, b| b.start.cmp(&a.start));

    for window in windows {
        let mut end = lines.len();
        for i in (window.start + 1)..lines.len() {
            if lines[i].trim_start().starts_with("## ") {
                end = i;
                break;
            }
        }

        let mut entries: Vec<TaskBoardEntry> = canonical
            .entries_for_status(window.status)
            .iter()
            .cloned()
            .collect();
        entries.sort_by(|a, b| a.wp_id.cmp(&b.wp_id).then_with(|| a.token.cmp(&b.token)));
        let mut new_lines: Vec<String> = entries.iter().map(format_entry).collect();

        let mut insert_at = end;
        for i in (window.start + 1)..end {
            if parse_entry_line(&lines[i]).is_some() {
                insert_at = i;
                break;
            }
        }

        let mut i = window.start + 1;
        while i < end && i < lines.len() {
            if lines[i].trim_start().starts_with("## ") {
                break;
            }
            if parse_entry_line(&lines[i]).is_some() {
                lines.remove(i);
                end = end.saturating_sub(1);
                if i < insert_at {
                    insert_at = insert_at.saturating_sub(1);
                }
                continue;
            }
            i += 1;
        }

        if !new_lines.is_empty() && insert_at == end {
            if insert_at > 0 && !lines[insert_at.saturating_sub(1)].trim().is_empty() {
                new_lines.insert(0, String::new());
            }
        }

        lines.splice(insert_at..insert_at, new_lines);
    }

    let mut out = lines.join("\n");
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}
