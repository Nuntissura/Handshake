use super::types::{
    MarkdownMirrorContractV1, MirrorSyncState, ProjectProfileKind,
    StructuredCollaborationRecordFamily, StructuredCollaborationValidationCode,
    StructuredCollaborationValidationResult, TaskBoardStatus, WorkflowQueueReasonCode,
    WorkflowStateFamily,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskBoardEntry {
    pub wp_id: String,
    pub token: String,
    pub status: TaskBoardStatus,
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
