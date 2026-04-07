use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str;

/// Tracks active session-level workspace allocations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionWorktreeAllocation {
    pub session_id: String,
    pub worktree_path: PathBuf,
}

impl SessionWorktreeAllocation {
    pub fn new(session_id: impl Into<String>, worktree_path: impl Into<PathBuf>) -> Self {
        Self {
            session_id: session_id.into(),
            worktree_path: worktree_path.into(),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct SessionWorktreeRegistry {
    allocations: HashMap<String, PathBuf>,
}

impl SessionWorktreeRegistry {
    pub fn new() -> Self {
        Self {
            allocations: HashMap::new(),
        }
    }

    pub fn put(&mut self, allocation: SessionWorktreeAllocation) {
        self.allocations
            .insert(allocation.session_id, allocation.worktree_path);
    }

    pub fn get(&self, session_id: &str) -> Option<&PathBuf> {
        self.allocations.get(session_id)
    }

    pub fn remove(&mut self, session_id: &str) -> Option<SessionWorktreeAllocation> {
        self.allocations
            .remove(session_id)
            .map(|worktree_path| SessionWorktreeAllocation::new(session_id, worktree_path))
    }

    pub fn len(&self) -> usize {
        self.allocations.len()
    }

    pub fn is_empty(&self) -> bool {
        self.allocations.is_empty()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MergeBackConflictReport {
    pub conflicting_files: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MergeBackArtifact {
    pub session_id: String,
    pub worktree_path: String,
    pub produced_at: DateTime<Utc>,
    pub diff_patch: String,
    pub conflict_report: Option<MergeBackConflictReport>,
}

impl MergeBackArtifact {
    pub fn has_conflicts(&self) -> bool {
        self.conflict_report
            .as_ref()
            .is_some_and(|report| !report.conflicting_files.is_empty())
    }
}

pub fn extract_conflicting_files_from_git_status_porcelain_z(
    status_output: &[u8],
) -> Vec<String> {
    let mut files = Vec::new();
    let mut dedupe = HashSet::new();

    for segment in status_output.split(|byte| *byte == b'\0') {
        if segment.len() < 4 {
            continue;
        }

        let status = match segment.get(0..2).and_then(|value| str::from_utf8(value).ok()) {
            Some(value) => value,
            None => continue,
        };

        if !matches!(status, "DD" | "AU" | "UA" | "AA" | "UD" | "DU" | "UU") {
            continue;
        }

        let path = segment
            .get(3..)
            .and_then(|value| str::from_utf8(value).ok())
            .map(|path| path.split('\t').next().unwrap_or_default().to_string())
            .filter(|path| !path.is_empty());

        if let Some(path) = path {
            if dedupe.insert(path.clone()) {
                files.push(path);
            }
        }
    }

    files
}

pub fn collect_merge_back_artifact(
    worktree_path: &Path,
    session_id: impl Into<String>,
) -> io::Result<MergeBackArtifact> {
    let worktree_path = worktree_path.to_string_lossy().to_string();

    let status_output = run_git_text_command(
        &worktree_path,
        &["status", "--short", "--porcelain", "-z"],
    )?;
    let diff_patch = run_git_text_command(&worktree_path, &["diff", "--binary"])?;
    let conflicting_files = extract_conflicting_files_from_git_status_porcelain_z(status_output.as_bytes());

    Ok(MergeBackArtifact {
        session_id: session_id.into(),
        worktree_path,
        produced_at: Utc::now(),
        diff_patch,
        conflict_report: if conflicting_files.is_empty() {
            None
        } else {
            Some(MergeBackConflictReport { conflicting_files })
        },
    })
}

fn run_git_text_command(cwd: &str, args: &[&str]) -> io::Result<String> {
    let output = Command::new("git").arg("-C").arg(cwd).args(args).output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "git command failed for {cwd} with args {:?}: {stderr}",
                args
            ),
        ));
    }

    String::from_utf8(output.stdout).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "git output was not valid UTF-8",
        )
    })
}

#[cfg(test)]
mod tests {
    use super::{
        extract_conflicting_files_from_git_status_porcelain_z, SessionWorktreeAllocation,
        SessionWorktreeRegistry,
    };

    #[test]
    fn registry_put_get_remove_roundtrip() {
        let mut registry = SessionWorktreeRegistry::new();
        let session_id = "session-1".to_string();

        registry.put(SessionWorktreeAllocation::new(
            &session_id,
            "tmp/session-1-worktree",
        ));

        assert_eq!(
            registry.get(&session_id).unwrap().to_string_lossy().as_ref(),
            "tmp/session-1-worktree"
        );
        assert_eq!(registry.len(), 1);

        let removed = registry
            .remove(&session_id)
            .expect("session allocation should be present");
        assert_eq!(removed.session_id, session_id);
        assert_eq!(
            removed.worktree_path.to_string_lossy().as_ref(),
            "tmp/session-1-worktree"
        );
        assert!(registry.get(&removed.session_id).is_none());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn extract_conflicting_files_from_git_status_porcelain_z_detects_unmerged() {
        let status_output = b"UU model.txt\0M  clean.txt\0DD rename old.txt\tnew.txt\0";

        assert_eq!(
            extract_conflicting_files_from_git_status_porcelain_z(status_output),
            vec!["model.txt".to_string(), "rename old.txt".to_string()]
        );
    }

    #[test]
    fn extract_conflicting_files_from_git_status_porcelain_z_ignores_non_conflicts() {
        let status_output = b"  src/main.rs\0A  src/new.rs\0D  removed.txt\0";

        assert!(extract_conflicting_files_from_git_status_porcelain_z(status_output).is_empty());
    }
}
