use super::types::TaskBoardStatus;

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
