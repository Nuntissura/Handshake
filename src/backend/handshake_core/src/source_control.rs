use std::{
    collections::BTreeSet,
    ffi::OsStr,
    io,
    path::{Component, Path, PathBuf},
    process::Command,
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct SourceControlRepository {
    root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceControlStatus {
    pub repo_root: PathBuf,
    pub branch: Option<String>,
    pub entries: Vec<StatusEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusEntry {
    pub path: String,
    pub index: Option<StatusCode>,
    pub worktree: Option<StatusCode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusCode {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    Untracked,
    Ignored,
    Unmerged,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffScope {
    Worktree,
    Staged,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceControlDiff {
    pub path: String,
    pub scope: DiffScope,
    pub patch: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceControlReceipt {
    pub operation: String,
    pub paths: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_ledger_event_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceControlCommit {
    pub id: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_ledger_event_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceControlLog {
    pub entries: Vec<SourceControlLogEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceControlLogEntry {
    pub id: String,
    pub author: String,
    pub timestamp: i64,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceControlBranch {
    pub name: String,
    pub current: bool,
    pub commit_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceControlBlame {
    pub path: String,
    pub lines: Vec<SourceControlBlameLine>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceControlBlameLine {
    pub line_number: usize,
    pub commit_id: String,
    pub author: String,
    pub content: String,
}

#[derive(Debug, Error)]
pub enum SourceControlError {
    #[error("git process failed: {0}")]
    GitIo(#[from] io::Error),
    #[error("git command failed in {repo} with args {args:?}: {stderr}")]
    GitCommandFailed {
        repo: String,
        args: Vec<String>,
        code: Option<i32>,
        stderr: String,
    },
    #[error("invalid git repository {path}: {reason}")]
    InvalidRepository { path: String, reason: String },
    #[error("invalid source-control path {path}: {reason}")]
    InvalidPath { path: String, reason: String },
    #[error("discard requires explicit confirmation")]
    DiscardRequiresConfirmation,
    #[error("commit message must not be empty")]
    EmptyCommitMessage,
    #[error("branch name must not be empty")]
    EmptyBranchName,
    #[error("invalid branch name {name}: {reason}")]
    InvalidBranchName { name: String, reason: String },
}

impl SourceControlRepository {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, SourceControlError> {
        let path = path.as_ref();
        let output = run_git_bytes(path, ["rev-parse", "--show-toplevel"])?;
        let root = String::from_utf8_lossy(&output).trim().to_string();
        if root.is_empty() {
            return Err(SourceControlError::InvalidRepository {
                path: path.display().to_string(),
                reason: "git returned an empty repository root".to_string(),
            });
        }
        Ok(Self {
            root: PathBuf::from(root),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn status(&self) -> Result<SourceControlStatus, SourceControlError> {
        let stdout = self.git_bytes(["status", "--porcelain=v1", "-z"])?;
        let branch = self.current_branch()?;
        Ok(SourceControlStatus {
            repo_root: self.root.clone(),
            branch,
            entries: parse_status_entries(&stdout),
        })
    }

    pub fn diff(
        &self,
        path: &str,
        scope: DiffScope,
    ) -> Result<SourceControlDiff, SourceControlError> {
        let normalized = normalize_repo_path(path)?;
        let mut args = vec!["diff".to_string()];
        if scope == DiffScope::Staged {
            args.push("--cached".to_string());
        }
        args.push("--".to_string());
        args.push(literal_pathspec(&normalized));
        let patch = String::from_utf8_lossy(&self.git_bytes(args)?).to_string();
        Ok(SourceControlDiff {
            path: normalized,
            scope,
            patch,
        })
    }

    pub fn stage(&self, paths: &[&str]) -> Result<SourceControlReceipt, SourceControlError> {
        let paths = normalize_paths(paths)?;
        let mut args = vec!["add".to_string(), "--".to_string()];
        args.extend(paths.iter().map(|path| literal_pathspec(path)));
        self.git_bytes(args)?;
        Ok(SourceControlReceipt {
            operation: "stage".to_string(),
            paths,
            event_ledger_event_id: None,
        })
    }

    pub fn unstage(&self, paths: &[&str]) -> Result<SourceControlReceipt, SourceControlError> {
        let paths = normalize_paths(paths)?;
        let mut args = vec![
            "restore".to_string(),
            "--staged".to_string(),
            "--".to_string(),
        ];
        args.extend(paths.iter().map(|path| literal_pathspec(path)));
        self.git_bytes(args)?;
        Ok(SourceControlReceipt {
            operation: "unstage".to_string(),
            paths,
            event_ledger_event_id: None,
        })
    }

    pub fn discard(
        &self,
        paths: &[&str],
        confirmed: bool,
    ) -> Result<SourceControlReceipt, SourceControlError> {
        if !confirmed {
            return Err(SourceControlError::DiscardRequiresConfirmation);
        }

        let paths = normalize_paths(paths)?;
        let status = self.status()?;
        let untracked: BTreeSet<String> = status
            .entries
            .iter()
            .filter(|entry| {
                entry.index == Some(StatusCode::Untracked)
                    && entry.worktree == Some(StatusCode::Untracked)
            })
            .flat_map(|entry| {
                let trimmed = entry.path.trim_end_matches('/').to_string();
                [entry.path.clone(), trimmed]
            })
            .collect();

        let mut tracked_paths = Vec::new();
        let mut untracked_paths = Vec::new();
        for path in &paths {
            if untracked.contains(path) {
                untracked_paths.push(path.clone());
            } else {
                tracked_paths.push(path.clone());
            }
        }

        if !tracked_paths.is_empty() {
            let mut args = vec![
                "restore".to_string(),
                "--staged".to_string(),
                "--worktree".to_string(),
                "--".to_string(),
            ];
            args.extend(tracked_paths.iter().map(|path| literal_pathspec(path)));
            self.git_bytes(args)?;
        }

        if !untracked_paths.is_empty() {
            let mut args = vec!["clean".to_string(), "-f".to_string(), "--".to_string()];
            args.extend(untracked_paths.iter().map(|path| literal_pathspec(path)));
            self.git_bytes(args)?;
        }

        Ok(SourceControlReceipt {
            operation: "discard".to_string(),
            paths,
            event_ledger_event_id: None,
        })
    }

    pub fn commit(&self, message: &str) -> Result<SourceControlCommit, SourceControlError> {
        let message = message.trim();
        if message.is_empty() {
            return Err(SourceControlError::EmptyCommitMessage);
        }
        self.git_bytes(["commit", "--no-gpg-sign", "-m", message])?;
        let id = String::from_utf8_lossy(&self.git_bytes(["rev-parse", "HEAD"])?)
            .trim()
            .to_string();
        Ok(SourceControlCommit {
            id,
            message: message.to_string(),
            event_ledger_event_id: None,
        })
    }

    pub fn log(&self, limit: usize) -> Result<SourceControlLog, SourceControlError> {
        let limit = limit.clamp(1, 100).to_string();
        let format = "%H%x00%an%x00%at%x00%s%x00";
        let output = match self.git_bytes([
            "log",
            "-n",
            limit.as_str(),
            "--pretty=format:%H%x00%an%x00%at%x00%s%x00",
        ]) {
            Ok(output) => output,
            Err(SourceControlError::GitCommandFailed { stderr, .. })
                if stderr.contains("does not have any commits yet")
                    || stderr.contains("your current branch")
                    || stderr.contains("unknown revision or path not in the working tree") =>
            {
                Vec::new()
            }
            Err(error) => return Err(error),
        };
        Ok(SourceControlLog {
            entries: parse_log_entries(&output, format),
        })
    }

    pub fn branches(&self) -> Result<Vec<SourceControlBranch>, SourceControlError> {
        let current = self.current_branch()?.unwrap_or_default();
        let output = self.git_bytes([
            "for-each-ref",
            "--format=%(refname:short)%00%(objectname)",
            "refs/heads",
        ])?;
        let text = String::from_utf8_lossy(&output);
        let mut branches = Vec::new();
        for line in text.lines() {
            let mut fields = line.split('\0');
            let Some(name) = fields.next() else {
                continue;
            };
            let Some(commit_id) = fields.next() else {
                continue;
            };
            branches.push(SourceControlBranch {
                name: name.to_string(),
                current: name == current,
                commit_id: commit_id.to_string(),
            });
        }
        Ok(branches)
    }

    pub fn create_branch(&self, name: &str) -> Result<SourceControlReceipt, SourceControlError> {
        let name = validate_branch_name(name)?;
        self.git_bytes(["check-ref-format", "--branch", name.as_str()])?;
        self.git_bytes(["branch", name.as_str()])?;
        Ok(SourceControlReceipt {
            operation: "create_branch".to_string(),
            paths: vec![name],
            event_ledger_event_id: None,
        })
    }

    pub fn switch_branch(&self, name: &str) -> Result<SourceControlReceipt, SourceControlError> {
        let name = validate_branch_name(name)?;
        self.git_bytes(["check-ref-format", "--branch", name.as_str()])?;
        self.git_bytes(["switch", name.as_str()])?;
        Ok(SourceControlReceipt {
            operation: "switch_branch".to_string(),
            paths: vec![name],
            event_ledger_event_id: None,
        })
    }

    pub fn blame(&self, path: &str) -> Result<SourceControlBlame, SourceControlError> {
        let normalized = normalize_repo_path(path)?;
        let output = self.git_bytes(["blame", "--line-porcelain", "--", normalized.as_str()])?;
        Ok(SourceControlBlame {
            path: normalized,
            lines: parse_blame_lines(&output),
        })
    }

    fn current_branch(&self) -> Result<Option<String>, SourceControlError> {
        let output = self.git_bytes(["branch", "--show-current"])?;
        let branch = String::from_utf8_lossy(&output).trim().to_string();
        Ok((!branch.is_empty()).then_some(branch))
    }

    fn git_bytes<I, S>(&self, args: I) -> Result<Vec<u8>, SourceControlError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        run_git_bytes(&self.root, args)
    }
}

fn run_git_bytes<I, S>(repo: &Path, args: I) -> Result<Vec<u8>, SourceControlError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let args: Vec<String> = args
        .into_iter()
        .map(|arg| arg.as_ref().to_string_lossy().to_string())
        .collect();
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["-c", "core.longpaths=true"])
        .args(&args)
        .output()?;

    if output.status.success() {
        return Ok(output.stdout);
    }

    Err(SourceControlError::GitCommandFailed {
        repo: repo.display().to_string(),
        args,
        code: output.status.code(),
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
    })
}

fn parse_status_entries(stdout: &[u8]) -> Vec<StatusEntry> {
    let mut entries = Vec::new();
    let mut records = stdout.split(|byte| *byte == 0);
    while let Some(record) = records.next() {
        if record.len() < 4 {
            continue;
        }
        let index = record[0] as char;
        let worktree = record[1] as char;
        let path = String::from_utf8_lossy(&record[3..]).replace('\\', "/");
        if matches!(index, 'R' | 'C') {
            let _old_path = records.next();
        }
        entries.push(StatusEntry {
            path,
            index: status_code(index),
            worktree: status_code(worktree),
        });
    }
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    entries
}

fn status_code(code: char) -> Option<StatusCode> {
    match code {
        ' ' => None,
        'A' => Some(StatusCode::Added),
        'M' => Some(StatusCode::Modified),
        'D' => Some(StatusCode::Deleted),
        'R' => Some(StatusCode::Renamed),
        'C' => Some(StatusCode::Copied),
        '?' => Some(StatusCode::Untracked),
        '!' => Some(StatusCode::Ignored),
        'U' => Some(StatusCode::Unmerged),
        _ => Some(StatusCode::Unknown),
    }
}

fn parse_log_entries(stdout: &[u8], _format: &str) -> Vec<SourceControlLogEntry> {
    let text = String::from_utf8_lossy(stdout);
    let fields: Vec<&str> = text.split('\0').filter(|field| !field.is_empty()).collect();
    let mut entries = Vec::new();
    let mut index = 0;
    while index + 3 < fields.len() {
        entries.push(SourceControlLogEntry {
            id: fields[index].to_string(),
            author: fields[index + 1].to_string(),
            timestamp: fields[index + 2].parse().unwrap_or_default(),
            message: fields[index + 3].to_string(),
        });
        index += 4;
    }
    entries
}

fn parse_blame_lines(stdout: &[u8]) -> Vec<SourceControlBlameLine> {
    let text = String::from_utf8_lossy(stdout);
    let mut lines = Vec::new();
    let mut commit_id = String::new();
    let mut author = String::new();
    let mut line_number = 0usize;

    for line in text.lines() {
        if let Some(content) = line.strip_prefix('\t') {
            lines.push(SourceControlBlameLine {
                line_number,
                commit_id: commit_id.clone(),
                author: author.clone(),
                content: content.to_string(),
            });
            continue;
        }

        if let Some(next_author) = line.strip_prefix("author ") {
            author = next_author.to_string();
            continue;
        }

        let mut fields = line.split_whitespace();
        let Some(first) = fields.next() else {
            continue;
        };
        if first.len() == 40 && first.chars().all(|ch| ch.is_ascii_hexdigit()) {
            let _original = fields.next();
            if let Some(final_line) = fields.next() {
                commit_id = first.to_string();
                line_number = final_line.parse().unwrap_or_default();
                author.clear();
            }
        }
    }

    lines
}

pub(crate) fn normalize_paths(paths: &[&str]) -> Result<Vec<String>, SourceControlError> {
    let mut normalized = Vec::new();
    for path in paths {
        normalized.push(normalize_repo_path(path)?);
    }
    if normalized.is_empty() {
        return Err(SourceControlError::InvalidPath {
            path: String::new(),
            reason: "at least one path is required".to_string(),
        });
    }
    Ok(normalized)
}

fn normalize_repo_path(path: &str) -> Result<String, SourceControlError> {
    let raw = path.trim();
    if raw.is_empty() {
        return Err(SourceControlError::InvalidPath {
            path: path.to_string(),
            reason: "path must not be empty".to_string(),
        });
    }

    let candidate = Path::new(raw);
    if candidate.is_absolute() {
        return Err(SourceControlError::InvalidPath {
            path: path.to_string(),
            reason: "absolute paths are not allowed".to_string(),
        });
    }

    let mut parts = Vec::new();
    for component in candidate.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_string_lossy().to_string()),
            Component::CurDir => {}
            Component::ParentDir => {
                return Err(SourceControlError::InvalidPath {
                    path: path.to_string(),
                    reason: "parent-directory traversal is not allowed".to_string(),
                });
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(SourceControlError::InvalidPath {
                    path: path.to_string(),
                    reason: "rooted paths are not allowed".to_string(),
                });
            }
        }
    }

    if parts.is_empty() {
        return Err(SourceControlError::InvalidPath {
            path: path.to_string(),
            reason: "path must name a file or directory".to_string(),
        });
    }

    Ok(parts.join("/"))
}

fn literal_pathspec(path: &str) -> String {
    format!(":(literal){path}")
}

pub(crate) fn validate_branch_name(name: &str) -> Result<String, SourceControlError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(SourceControlError::EmptyBranchName);
    }
    if trimmed.starts_with('-') {
        return Err(SourceControlError::InvalidBranchName {
            name: name.to_string(),
            reason: "branch names must not start with '-'".to_string(),
        });
    }
    if trimmed.starts_with("refs/") {
        return Err(SourceControlError::InvalidBranchName {
            name: name.to_string(),
            reason: "full refs are not accepted; pass a local branch name".to_string(),
        });
    }
    if trimmed == "@" || trimmed.starts_with("@{") || trimmed.contains("@{") {
        return Err(SourceControlError::InvalidBranchName {
            name: name.to_string(),
            reason: "git branch shorthand is not accepted".to_string(),
        });
    }
    Ok(trimmed.to_string())
}
