use std::{
    io,
    path::{Component, Path, PathBuf},
};

use crate::storage::artifacts::resolve_workspace_root;

pub const RUNTIME_GOVERNANCE_ROOT_ENV: &str = "HANDSHAKE_GOVERNANCE_ROOT";
pub const RUNTIME_GOVERNANCE_DEFAULT_ROOT: &str = ".handshake/gov";
pub const RUNTIME_TASK_BOARD_FILE: &str = "TASK_BOARD.md";
pub const RUNTIME_SPEC_CURRENT_FILE: &str = "SPEC_CURRENT.md";
pub const RUNTIME_ROLE_MAILBOX_DIR: &str = "ROLE_MAILBOX";
pub const RUNTIME_GOVERNANCE_DECISIONS_DIR: &str = "governance_decisions";
pub const RUNTIME_GOVERNANCE_AUTO_SIGNATURES_DIR: &str = "auto_signatures";
pub const RUNTIME_WORK_PACKETS_DIR: &str = "work_packets";
pub const RUNTIME_MICRO_TASKS_DIR: &str = "micro_tasks";
pub const RUNTIME_TASK_BOARD_STRUCTURED_DIR: &str = "task_board";

#[derive(Debug, Clone)]
pub struct RuntimeGovernancePaths {
    workspace_root: PathBuf,
    governance_root: PathBuf,
}

impl RuntimeGovernancePaths {
    pub fn resolve() -> Result<Self, io::Error> {
        let workspace_root = resolve_workspace_root()
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string()))?;
        let configured = std::env::var(RUNTIME_GOVERNANCE_ROOT_ENV)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .map(PathBuf::from);
        Self::from_workspace_root_with_override(workspace_root, configured)
    }

    pub fn from_workspace_root(workspace_root: PathBuf) -> Result<Self, io::Error> {
        Self::from_workspace_root_with_override(workspace_root, None)
    }

    fn from_workspace_root_with_override(
        workspace_root: PathBuf,
        override_root: Option<PathBuf>,
    ) -> Result<Self, io::Error> {
        let workspace_root = absolutize(workspace_root)?;
        let configured =
            override_root.unwrap_or_else(|| PathBuf::from(RUNTIME_GOVERNANCE_DEFAULT_ROOT));
        if has_parent_dir(&configured) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "runtime governance root must not contain '..' segments",
            ));
        }

        let governance_root = if configured.is_absolute() {
            configured
        } else {
            workspace_root.join(configured)
        };
        let governance_root = absolutize(governance_root)?;

        ensure_runtime_boundary(&workspace_root, &governance_root)?;

        Ok(Self {
            workspace_root,
            governance_root,
        })
    }

    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    pub fn governance_root(&self) -> &Path {
        &self.governance_root
    }

    pub fn governance_root_display(&self) -> String {
        ensure_trailing_slash(display_path(&self.workspace_root, &self.governance_root))
    }

    pub fn spec_current_path(&self) -> PathBuf {
        self.governance_root.join(RUNTIME_SPEC_CURRENT_FILE)
    }

    pub fn spec_current_display(&self) -> String {
        display_path(&self.workspace_root, &self.spec_current_path())
    }

    pub fn task_board_path(&self) -> PathBuf {
        self.governance_root.join(RUNTIME_TASK_BOARD_FILE)
    }

    pub fn task_board_display(&self) -> String {
        display_path(&self.workspace_root, &self.task_board_path())
    }

    pub fn role_mailbox_export_dir(&self) -> PathBuf {
        self.governance_root.join(RUNTIME_ROLE_MAILBOX_DIR)
    }

    pub fn role_mailbox_export_dir_display(&self) -> String {
        ensure_trailing_slash(display_path(
            &self.workspace_root,
            &self.role_mailbox_export_dir(),
        ))
    }

    pub fn work_packets_dir(&self) -> PathBuf {
        self.governance_root.join(RUNTIME_WORK_PACKETS_DIR)
    }

    pub fn work_packet_dir(&self, wp_id: &str) -> io::Result<PathBuf> {
        Ok(self.work_packets_dir().join(safe_runtime_segment(wp_id)?))
    }

    pub fn work_packet_packet_path(&self, wp_id: &str) -> io::Result<PathBuf> {
        Ok(self.work_packet_dir(wp_id)?.join("packet.json"))
    }

    pub fn work_packet_packet_display(&self, wp_id: &str) -> io::Result<String> {
        Ok(display_path(
            &self.workspace_root,
            &self.work_packet_packet_path(wp_id)?,
        ))
    }

    pub fn work_packet_summary_path(&self, wp_id: &str) -> io::Result<PathBuf> {
        Ok(self.work_packet_dir(wp_id)?.join("summary.json"))
    }

    pub fn work_packet_summary_display(&self, wp_id: &str) -> io::Result<String> {
        Ok(display_path(
            &self.workspace_root,
            &self.work_packet_summary_path(wp_id)?,
        ))
    }

    pub fn micro_tasks_dir(&self) -> PathBuf {
        self.governance_root.join(RUNTIME_MICRO_TASKS_DIR)
    }

    pub fn micro_task_dir(&self, wp_id: &str, mt_id: &str) -> io::Result<PathBuf> {
        Ok(self
            .micro_tasks_dir()
            .join(safe_runtime_segment(wp_id)?)
            .join(safe_runtime_segment(mt_id)?))
    }

    pub fn micro_task_packet_path(&self, wp_id: &str, mt_id: &str) -> io::Result<PathBuf> {
        Ok(self.micro_task_dir(wp_id, mt_id)?.join("packet.json"))
    }

    pub fn micro_task_packet_display(&self, wp_id: &str, mt_id: &str) -> io::Result<String> {
        Ok(display_path(
            &self.workspace_root,
            &self.micro_task_packet_path(wp_id, mt_id)?,
        ))
    }

    pub fn micro_task_summary_path(&self, wp_id: &str, mt_id: &str) -> io::Result<PathBuf> {
        Ok(self.micro_task_dir(wp_id, mt_id)?.join("summary.json"))
    }

    pub fn micro_task_summary_display(&self, wp_id: &str, mt_id: &str) -> io::Result<String> {
        Ok(display_path(
            &self.workspace_root,
            &self.micro_task_summary_path(wp_id, mt_id)?,
        ))
    }

    pub fn task_board_structured_dir(&self) -> PathBuf {
        self.governance_root.join(RUNTIME_TASK_BOARD_STRUCTURED_DIR)
    }

    pub fn task_board_index_path(&self) -> PathBuf {
        self.task_board_structured_dir().join("index.json")
    }

    pub fn task_board_index_display(&self) -> String {
        display_path(&self.workspace_root, &self.task_board_index_path())
    }

    pub fn task_board_view_path(&self, view_id: &str) -> io::Result<PathBuf> {
        Ok(self
            .task_board_structured_dir()
            .join("views")
            .join(format!("{}.json", safe_runtime_segment(view_id)?)))
    }

    pub fn task_board_view_display(&self, view_id: &str) -> io::Result<String> {
        Ok(display_path(
            &self.workspace_root,
            &self.task_board_view_path(view_id)?,
        ))
    }

    pub fn is_runtime_artifact_display_path(&self, value: &str) -> bool {
        let normalized = normalize_display_like(value);
        if normalized.is_empty() {
            return false;
        }

        let governance_root = normalize_display_like(&self.governance_root_display());
        let role_mailbox_root = normalize_display_like(&self.role_mailbox_export_dir_display());
        let task_board_root = normalize_display_like(&ensure_trailing_slash(display_path(
            &self.workspace_root,
            &self.task_board_structured_dir(),
        )));

        normalized.starts_with(&governance_root)
            || normalized.starts_with(&role_mailbox_root)
            || normalized.starts_with(&task_board_root)
    }

    pub fn invalid_runtime_authority_refs<'a>(&self, refs: &'a [String]) -> Vec<&'a str> {
        refs.iter()
            .filter_map(|value| {
                if self.is_runtime_artifact_display_path(value) {
                    None
                } else {
                    Some(value.as_str())
                }
            })
            .collect()
    }

    pub fn governance_decisions_dir(&self) -> PathBuf {
        self.governance_root.join(RUNTIME_GOVERNANCE_DECISIONS_DIR)
    }

    pub fn governance_decisions_dir_display(&self) -> String {
        ensure_trailing_slash(display_path(
            &self.workspace_root,
            &self.governance_decisions_dir(),
        ))
    }

    pub fn governance_decision_path(&self, decision_id: &str) -> PathBuf {
        self.governance_decisions_dir()
            .join(format!("{decision_id}.json"))
    }

    pub fn auto_signatures_dir(&self) -> PathBuf {
        self.governance_root
            .join(RUNTIME_GOVERNANCE_AUTO_SIGNATURES_DIR)
    }

    pub fn auto_signatures_dir_display(&self) -> String {
        ensure_trailing_slash(display_path(
            &self.workspace_root,
            &self.auto_signatures_dir(),
        ))
    }

    pub fn auto_signature_path(&self, auto_signature_id: &str) -> PathBuf {
        self.auto_signatures_dir()
            .join(format!("{auto_signature_id}.json"))
    }
}

fn absolutize(path: PathBuf) -> Result<PathBuf, io::Error> {
    if path.is_absolute() {
        return Ok(path);
    }
    Ok(std::env::current_dir()?.join(path))
}

fn has_parent_dir(path: &Path) -> bool {
    path.components()
        .any(|component| matches!(component, Component::ParentDir))
}

fn ensure_runtime_boundary(workspace_root: &Path, governance_root: &Path) -> Result<(), io::Error> {
    if !governance_root.starts_with(workspace_root) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "runtime governance root must stay under workspace root",
        ));
    }

    let relative = governance_root.strip_prefix(workspace_root).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid runtime governance root",
        )
    })?;

    for component in relative.components() {
        let Component::Normal(segment) = component else {
            continue;
        };
        let segment = segment.to_string_lossy();
        if segment.eq_ignore_ascii_case(".GOV") {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "runtime governance root must not use .GOV directory",
            ));
        }
        if segment.eq_ignore_ascii_case("docs") {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "runtime governance root must not use docs directory",
            ));
        }
    }

    Ok(())
}

fn display_path(workspace_root: &Path, path: &Path) -> String {
    let shown = path.strip_prefix(workspace_root).unwrap_or(path);
    shown
        .to_string_lossy()
        .replace('\\', "/")
        .trim_start_matches("./")
        .to_string()
}

fn ensure_trailing_slash(value: String) -> String {
    if value.ends_with('/') {
        value
    } else {
        format!("{value}/")
    }
}

fn safe_runtime_segment(value: &str) -> Result<String, io::Error> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.contains("..")
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "runtime path segment must be non-empty and must not contain path separators or '..'",
        ));
    }
    Ok(trimmed.to_string())
}

fn normalize_display_like(value: &str) -> String {
    value.trim().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::RuntimeGovernancePaths;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn defaults_to_handshake_gov_under_workspace() -> std::io::Result<()> {
        let dir = tempdir()?;
        let workspace_root = dir.path().to_path_buf();
        let paths = RuntimeGovernancePaths::from_workspace_root(workspace_root.clone())?;
        assert_eq!(
            paths.governance_root(),
            workspace_root.join(".handshake").join("gov")
        );
        assert_eq!(
            paths.task_board_path(),
            workspace_root
                .join(".handshake")
                .join("gov")
                .join("TASK_BOARD.md")
        );
        assert_eq!(
            paths.governance_decisions_dir(),
            workspace_root
                .join(".handshake")
                .join("gov")
                .join("governance_decisions")
        );
        assert_eq!(
            paths.auto_signatures_dir(),
            workspace_root
                .join(".handshake")
                .join("gov")
                .join("auto_signatures")
        );
        Ok(())
    }

    #[test]
    fn rejects_docs_runtime_root() {
        let err = RuntimeGovernancePaths::from_workspace_root_with_override(
            PathBuf::from("/tmp/hsk"),
            Some(PathBuf::from("docs")),
        )
        .expect_err("docs root must be rejected");
        assert!(err.to_string().contains("docs directory"));
    }

    #[test]
    fn rejects_dot_gov_runtime_root() {
        let err = RuntimeGovernancePaths::from_workspace_root_with_override(
            PathBuf::from("/tmp/hsk"),
            Some(PathBuf::from(".GOV")),
        )
        .expect_err(".GOV root must be rejected");
        assert!(err.to_string().contains(".GOV directory"));
    }
}
