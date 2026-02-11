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
