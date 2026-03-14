use serde::{Deserialize, Serialize};

use super::types::{
    structured_collaboration_schema_descriptor, validate_structured_collaboration_record,
    MirrorSyncState, ProjectProfileKind, StructuredCollaborationRecordFamily,
    StructuredCollaborationValidationResult, TaskBoardStatus,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskBoardEntry {
    pub wp_id: String,
    pub token: String,
    pub status: TaskBoardStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StructuredTaskBoardEntryRecord {
    pub schema_id: String,
    pub schema_version: String,
    pub record_id: String,
    pub record_kind: String,
    pub project_profile_kind: ProjectProfileKind,
    pub updated_at: String,
    pub mirror_state: MirrorSyncState,
    pub authority_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub task_board_id: String,
    pub work_packet_id: String,
    pub lane_id: String,
    pub display_order: u64,
    #[serde(default)]
    pub view_ids: Vec<String>,
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StructuredTaskBoardIndexRecord {
    pub schema_id: String,
    pub schema_version: String,
    pub record_id: String,
    pub record_kind: String,
    pub project_profile_kind: ProjectProfileKind,
    pub updated_at: String,
    pub mirror_state: MirrorSyncState,
    pub authority_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub task_board_id: String,
    pub view_ids: Vec<String>,
    pub rows: Vec<StructuredTaskBoardEntryRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StructuredTaskBoardViewRecord {
    pub schema_id: String,
    pub schema_version: String,
    pub record_id: String,
    pub record_kind: String,
    pub project_profile_kind: ProjectProfileKind,
    pub updated_at: String,
    pub mirror_state: MirrorSyncState,
    pub authority_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub task_board_id: String,
    pub view_id: String,
    pub lane_ids: Vec<String>,
    pub rows: Vec<StructuredTaskBoardEntryRecord>,
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

pub fn task_board_lane_id(status: TaskBoardStatus) -> &'static str {
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

pub fn build_structured_task_board_entry(
    task_board_id: &str,
    updated_at: &str,
    authority_refs: &[String],
    evidence_refs: &[String],
    display_order: u64,
    view_ids: &[String],
    entry: &TaskBoardEntry,
) -> StructuredTaskBoardEntryRecord {
    let descriptor = structured_collaboration_schema_descriptor(
        StructuredCollaborationRecordFamily::TaskBoardEntry,
    );
    StructuredTaskBoardEntryRecord {
        schema_id: descriptor.schema_id.to_string(),
        schema_version: descriptor.schema_version.to_string(),
        record_id: entry.wp_id.clone(),
        record_kind: descriptor.record_kind.to_string(),
        project_profile_kind: ProjectProfileKind::SoftwareDelivery,
        updated_at: updated_at.to_string(),
        mirror_state: MirrorSyncState::Synchronized,
        authority_refs: authority_refs.to_vec(),
        evidence_refs: evidence_refs.to_vec(),
        task_board_id: task_board_id.to_string(),
        work_packet_id: entry.wp_id.clone(),
        lane_id: task_board_lane_id(entry.status).to_string(),
        display_order,
        view_ids: view_ids.to_vec(),
        token: entry.token.clone(),
    }
}

pub fn build_structured_task_board_index(
    task_board_id: &str,
    updated_at: &str,
    authority_refs: &[String],
    evidence_refs: &[String],
    view_ids: &[String],
    rows: Vec<StructuredTaskBoardEntryRecord>,
) -> StructuredTaskBoardIndexRecord {
    let descriptor = structured_collaboration_schema_descriptor(
        StructuredCollaborationRecordFamily::TaskBoardIndex,
    );
    StructuredTaskBoardIndexRecord {
        schema_id: descriptor.schema_id.to_string(),
        schema_version: descriptor.schema_version.to_string(),
        record_id: "task_board_index".to_string(),
        record_kind: descriptor.record_kind.to_string(),
        project_profile_kind: ProjectProfileKind::SoftwareDelivery,
        updated_at: updated_at.to_string(),
        mirror_state: MirrorSyncState::Synchronized,
        authority_refs: authority_refs.to_vec(),
        evidence_refs: evidence_refs.to_vec(),
        task_board_id: task_board_id.to_string(),
        view_ids: view_ids.to_vec(),
        rows,
    }
}

pub fn build_structured_task_board_view(
    task_board_id: &str,
    view_id: &str,
    updated_at: &str,
    authority_refs: &[String],
    evidence_refs: &[String],
    lane_ids: &[String],
    rows: Vec<StructuredTaskBoardEntryRecord>,
) -> StructuredTaskBoardViewRecord {
    let descriptor = structured_collaboration_schema_descriptor(
        StructuredCollaborationRecordFamily::TaskBoardView,
    );
    StructuredTaskBoardViewRecord {
        schema_id: descriptor.schema_id.to_string(),
        schema_version: descriptor.schema_version.to_string(),
        record_id: view_id.to_string(),
        record_kind: descriptor.record_kind.to_string(),
        project_profile_kind: ProjectProfileKind::SoftwareDelivery,
        updated_at: updated_at.to_string(),
        mirror_state: MirrorSyncState::Synchronized,
        authority_refs: authority_refs.to_vec(),
        evidence_refs: evidence_refs.to_vec(),
        task_board_id: task_board_id.to_string(),
        view_id: view_id.to_string(),
        lane_ids: lane_ids.to_vec(),
        rows,
    }
}

pub fn validate_structured_task_board_index(
    record: &StructuredTaskBoardIndexRecord,
) -> StructuredCollaborationValidationResult {
    let mut result = validate_structured_collaboration_record(
        StructuredCollaborationRecordFamily::TaskBoardIndex,
        &serde_json::to_value(record).unwrap_or_default(),
    );
    for row in &record.rows {
        result.merge(validate_structured_task_board_entry(row));
    }
    result
}

pub fn validate_structured_task_board_view(
    record: &StructuredTaskBoardViewRecord,
) -> StructuredCollaborationValidationResult {
    let mut result = validate_structured_collaboration_record(
        StructuredCollaborationRecordFamily::TaskBoardView,
        &serde_json::to_value(record).unwrap_or_default(),
    );
    for row in &record.rows {
        result.merge(validate_structured_task_board_entry(row));
    }
    result
}

pub fn validate_structured_task_board_entry(
    record: &StructuredTaskBoardEntryRecord,
) -> StructuredCollaborationValidationResult {
    validate_structured_collaboration_record(
        StructuredCollaborationRecordFamily::TaskBoardEntry,
        &serde_json::to_value(record).unwrap_or_default(),
    )
}
