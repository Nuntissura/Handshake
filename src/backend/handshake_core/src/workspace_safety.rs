use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str;
use uuid::Uuid;

use crate::flight_recorder::{FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType};

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

    pub fn iter(&self) -> impl Iterator<Item = (&String, &PathBuf)> {
        self.allocations.iter()
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

/// INV-WS-002: Fail-closed isolation enforcement.
/// If the session has no worktree allocation in the registry, execution MUST be denied.
pub fn validate_session_has_isolation<'a>(
    registry: &'a SessionWorktreeRegistry,
    session_id: &str,
) -> Result<&'a PathBuf, IsolationDenial> {
    registry
        .get(session_id)
        .ok_or_else(|| IsolationDenial::NoIsolation {
            session_id: session_id.to_string(),
        })
}

/// Result of a cross-session access check.
/// When the path falls inside another session's worktree, this captures the
/// owning session id so callers can emit FR events regardless of approval.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CrossSessionCheckResult {
    /// Path does not belong to any other session -- no restriction.
    NoConflict,
    /// Path belongs to another session but operator approved access.
    ApprovedOverride {
        target_session: String,
    },
    /// Path belongs to another session and access is denied (no approval).
    Denied {
        target_session: String,
    },
}

/// INV-WS-003: Cross-session file access denial.
/// A session MUST NOT access paths inside another session's worktree unless
/// explicit operator approval is provided.
pub fn validate_no_cross_session_access(
    registry: &SessionWorktreeRegistry,
    session_id: &str,
    target_path: &Path,
    operator_approved: bool,
) -> Result<CrossSessionCheckResult, IsolationDenial> {
    for (other_session, worktree_path) in registry.iter() {
        if other_session == session_id {
            continue;
        }
        if target_path.starts_with(worktree_path) {
            if operator_approved {
                return Ok(CrossSessionCheckResult::ApprovedOverride {
                    target_session: other_session.clone(),
                });
            }
            return Err(IsolationDenial::CrossSessionAccess {
                source_session: session_id.to_string(),
                target_session: other_session.clone(),
                target_path: target_path.to_path_buf(),
            });
        }
    }
    Ok(CrossSessionCheckResult::NoConflict)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IsolationDenial {
    /// INV-WS-002: No worktree allocation exists for this session.
    NoIsolation { session_id: String },
    /// INV-WS-003: Path belongs to another session's worktree.
    CrossSessionAccess {
        source_session: String,
        target_session: String,
        target_path: PathBuf,
    },
}

impl std::fmt::Display for IsolationDenial {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IsolationDenial::NoIsolation { session_id } => {
                write!(
                    f,
                    "INV-WS-002: no isolation established for session {session_id}; execution denied"
                )
            }
            IsolationDenial::CrossSessionAccess {
                source_session,
                target_session,
                target_path,
            } => {
                write!(
                    f,
                    "INV-WS-003: session {source_session} denied access to {} (owned by session {target_session})",
                    target_path.display()
                )
            }
        }
    }
}

impl std::error::Error for IsolationDenial {}

impl IsolationDenial {
    /// Build a Flight Recorder event for this denial.
    pub fn to_fr_event(&self, trace_id: Uuid) -> FlightRecorderEvent {
        match self {
            IsolationDenial::NoIsolation { session_id } => FlightRecorderEvent::new(
                FlightRecorderEventType::WorkspaceIsolationDenied,
                FlightRecorderActor::System,
                trace_id,
                json!({
                    "invariant": "INV-WS-002",
                    "session_id": session_id,
                    "reason": self.to_string(),
                }),
            ),
            IsolationDenial::CrossSessionAccess {
                source_session,
                target_session,
                target_path,
            } => FlightRecorderEvent::new(
                FlightRecorderEventType::WorkspaceCrossSessionDenied,
                FlightRecorderActor::System,
                trace_id,
                json!({
                    "invariant": "INV-WS-003",
                    "source_session": source_session,
                    "target_session": target_session,
                    "target_path": target_path.display().to_string(),
                    "reason": self.to_string(),
                }),
            ),
        }
    }
}

/// Build a Flight Recorder event for an operator-approved cross-session access.
pub fn cross_session_approved_fr_event(
    trace_id: Uuid,
    source_session: &str,
    target_session: &str,
    target_path: &Path,
) -> FlightRecorderEvent {
    FlightRecorderEvent::new(
        FlightRecorderEventType::WorkspaceCrossSessionApproved,
        FlightRecorderActor::Human,
        trace_id,
        json!({
            "invariant": "INV-WS-003",
            "source_session": source_session,
            "target_session": target_session,
            "target_path": target_path.display().to_string(),
            "operator_approved": true,
        }),
    )
}

/// INV-WS-002 enforcement entry point with FR event emission.
/// On denial, returns the `IsolationDenial` paired with a ready-to-record
/// `FlightRecorderEvent` so the caller can persist the audit trail.
pub fn enforce_workspace_isolation(
    registry: &SessionWorktreeRegistry,
    session_id: &str,
    trace_id: Uuid,
) -> Result<PathBuf, (IsolationDenial, FlightRecorderEvent)> {
    match validate_session_has_isolation(registry, session_id) {
        Ok(path) => Ok(path.clone()),
        Err(denial) => {
            let fr_event = denial.to_fr_event(trace_id);
            Err((denial, fr_event))
        }
    }
}

/// INV-WS-003 enforcement entry point with FR event emission.
/// On denial, returns the denial paired with a `FlightRecorderEvent`.
/// On approved override, returns the check result paired with the
/// operator-approved FR event so the override is auditable.
pub fn enforce_cross_session_access(
    registry: &SessionWorktreeRegistry,
    session_id: &str,
    target_path: &Path,
    operator_approved: bool,
    trace_id: Uuid,
) -> Result<(CrossSessionCheckResult, Option<FlightRecorderEvent>), (IsolationDenial, FlightRecorderEvent)>
{
    match validate_no_cross_session_access(registry, session_id, target_path, operator_approved) {
        Ok(CrossSessionCheckResult::ApprovedOverride { ref target_session }) => {
            let fr_event = cross_session_approved_fr_event(
                trace_id,
                session_id,
                target_session,
                target_path,
            );
            Ok((
                CrossSessionCheckResult::ApprovedOverride {
                    target_session: target_session.clone(),
                },
                Some(fr_event),
            ))
        }
        Ok(result) => Ok((result, None)),
        Err(denial) => {
            let fr_event = denial.to_fr_event(trace_id);
            Err((denial, fr_event))
        }
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
    use super::*;
    use std::path::PathBuf;
    use uuid::Uuid;

    fn two_session_registry() -> SessionWorktreeRegistry {
        let mut registry = SessionWorktreeRegistry::new();
        registry.put(SessionWorktreeAllocation::new(
            "session-a",
            "/worktrees/session-a",
        ));
        registry.put(SessionWorktreeAllocation::new(
            "session-b",
            "/worktrees/session-b",
        ));
        registry
    }

    // ---------------------------------------------------------------
    // MT-001 regression: registry basics
    // ---------------------------------------------------------------

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

    // ---------------------------------------------------------------
    // MT-003 regression: conflict extraction
    // ---------------------------------------------------------------

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

    // ---------------------------------------------------------------
    // MT-005: INV-WS-002 fail-closed isolation enforcement
    // ---------------------------------------------------------------

    #[test]
    fn validate_session_has_isolation_returns_path_when_registered() {
        let registry = two_session_registry();
        let result = validate_session_has_isolation(&registry, "session-a");
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            &PathBuf::from("/worktrees/session-a")
        );
    }

    #[test]
    fn validate_session_has_isolation_fails_closed_when_missing() {
        let registry = two_session_registry();
        let result = validate_session_has_isolation(&registry, "session-unknown");
        assert!(result.is_err());
        let denial = result.unwrap_err();
        assert_eq!(
            denial,
            IsolationDenial::NoIsolation {
                session_id: "session-unknown".to_string(),
            }
        );
        assert!(
            denial.to_string().contains("INV-WS-002"),
            "denial message must reference INV-WS-002"
        );
    }

    #[test]
    fn validate_session_has_isolation_empty_registry_denies() {
        let registry = SessionWorktreeRegistry::new();
        let result = validate_session_has_isolation(&registry, "any-session");
        assert!(result.is_err());
    }

    #[test]
    fn validate_session_has_isolation_removed_session_denies() {
        let mut registry = two_session_registry();
        registry.remove("session-a");
        let result = validate_session_has_isolation(&registry, "session-a");
        assert!(result.is_err());
        // session-b still works
        assert!(validate_session_has_isolation(&registry, "session-b").is_ok());
    }

    #[test]
    fn isolation_denial_no_isolation_fr_event_has_correct_type() {
        let denial = IsolationDenial::NoIsolation {
            session_id: "sess-42".to_string(),
        };
        let event = denial.to_fr_event(Uuid::nil());
        assert_eq!(
            event.event_type,
            FlightRecorderEventType::WorkspaceIsolationDenied
        );
        let payload = &event.payload;
        assert_eq!(payload["invariant"], "INV-WS-002");
        assert_eq!(payload["session_id"], "sess-42");
    }

    // ---------------------------------------------------------------
    // MT-006: INV-WS-003 cross-session access denial
    // ---------------------------------------------------------------

    #[test]
    fn cross_session_access_denied_without_approval() {
        let registry = two_session_registry();
        let target = Path::new("/worktrees/session-b/src/main.rs");
        let result =
            validate_no_cross_session_access(&registry, "session-a", target, false);
        assert!(result.is_err());
        let denial = result.unwrap_err();
        match &denial {
            IsolationDenial::CrossSessionAccess {
                source_session,
                target_session,
                target_path,
            } => {
                assert_eq!(source_session, "session-a");
                assert_eq!(target_session, "session-b");
                assert_eq!(target_path, target);
            }
            other => panic!("expected CrossSessionAccess, got {other:?}"),
        }
        assert!(
            denial.to_string().contains("INV-WS-003"),
            "denial message must reference INV-WS-003"
        );
    }

    #[test]
    fn cross_session_access_allowed_with_operator_approval() {
        let registry = two_session_registry();
        let target = Path::new("/worktrees/session-b/src/main.rs");
        let result =
            validate_no_cross_session_access(&registry, "session-a", target, true);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            CrossSessionCheckResult::ApprovedOverride {
                target_session: "session-b".to_string(),
            }
        );
    }

    #[test]
    fn own_session_path_is_always_allowed() {
        let registry = two_session_registry();
        let target = Path::new("/worktrees/session-a/src/lib.rs");
        let result =
            validate_no_cross_session_access(&registry, "session-a", target, false);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), CrossSessionCheckResult::NoConflict);
    }

    #[test]
    fn path_outside_all_worktrees_is_allowed() {
        let registry = two_session_registry();
        let target = Path::new("/home/user/documents/notes.txt");
        let result =
            validate_no_cross_session_access(&registry, "session-a", target, false);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), CrossSessionCheckResult::NoConflict);
    }

    #[test]
    fn cross_session_denial_fr_event_has_correct_type() {
        let denial = IsolationDenial::CrossSessionAccess {
            source_session: "sess-a".to_string(),
            target_session: "sess-b".to_string(),
            target_path: PathBuf::from("/worktrees/sess-b/file.rs"),
        };
        let event = denial.to_fr_event(Uuid::nil());
        assert_eq!(
            event.event_type,
            FlightRecorderEventType::WorkspaceCrossSessionDenied
        );
        let payload = &event.payload;
        assert_eq!(payload["invariant"], "INV-WS-003");
        assert_eq!(payload["source_session"], "sess-a");
        assert_eq!(payload["target_session"], "sess-b");
    }

    #[test]
    fn cross_session_approved_fr_event_has_correct_type() {
        let event = cross_session_approved_fr_event(
            Uuid::nil(),
            "sess-a",
            "sess-b",
            Path::new("/worktrees/sess-b/file.rs"),
        );
        assert_eq!(
            event.event_type,
            FlightRecorderEventType::WorkspaceCrossSessionApproved
        );
        assert_eq!(event.payload["operator_approved"], true);
        assert_eq!(event.payload["invariant"], "INV-WS-003");
    }

    #[test]
    fn empty_registry_allows_any_path_no_cross_session() {
        let registry = SessionWorktreeRegistry::new();
        let target = Path::new("/worktrees/session-a/src/main.rs");
        let result =
            validate_no_cross_session_access(&registry, "session-x", target, false);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), CrossSessionCheckResult::NoConflict);
    }

    #[test]
    fn exact_worktree_root_path_triggers_cross_session_check() {
        let registry = two_session_registry();
        // Accessing the exact root of another session's worktree
        let target = Path::new("/worktrees/session-b");
        let result =
            validate_no_cross_session_access(&registry, "session-a", target, false);
        assert!(result.is_err());
    }

    // ---------------------------------------------------------------
    // MT-005/MT-006 wiring: enforce_* entry points with FR emission
    // ---------------------------------------------------------------

    #[test]
    fn enforce_workspace_isolation_returns_path_on_success() {
        let registry = two_session_registry();
        let result = enforce_workspace_isolation(&registry, "session-a", Uuid::nil());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), PathBuf::from("/worktrees/session-a"));
    }

    #[test]
    fn enforce_workspace_isolation_emits_fr_event_on_denial() {
        let registry = two_session_registry();
        let result = enforce_workspace_isolation(&registry, "unknown", Uuid::nil());
        assert!(result.is_err());
        let (denial, fr_event) = result.unwrap_err();
        assert!(matches!(denial, IsolationDenial::NoIsolation { .. }));
        assert_eq!(
            fr_event.event_type,
            FlightRecorderEventType::WorkspaceIsolationDenied
        );
        assert_eq!(fr_event.payload["invariant"], "INV-WS-002");
    }

    #[test]
    fn enforce_cross_session_access_no_conflict_no_fr_event() {
        let registry = two_session_registry();
        let target = Path::new("/worktrees/session-a/src/lib.rs");
        let result = enforce_cross_session_access(
            &registry, "session-a", target, false, Uuid::nil(),
        );
        assert!(result.is_ok());
        let (check_result, fr_event) = result.unwrap();
        assert_eq!(check_result, CrossSessionCheckResult::NoConflict);
        assert!(fr_event.is_none());
    }

    #[test]
    fn enforce_cross_session_access_emits_fr_event_on_denial() {
        let registry = two_session_registry();
        let target = Path::new("/worktrees/session-b/src/main.rs");
        let result = enforce_cross_session_access(
            &registry, "session-a", target, false, Uuid::nil(),
        );
        assert!(result.is_err());
        let (denial, fr_event) = result.unwrap_err();
        assert!(matches!(denial, IsolationDenial::CrossSessionAccess { .. }));
        assert_eq!(
            fr_event.event_type,
            FlightRecorderEventType::WorkspaceCrossSessionDenied
        );
        assert_eq!(fr_event.payload["invariant"], "INV-WS-003");
    }

    #[test]
    fn enforce_cross_session_access_emits_approved_fr_event_on_override() {
        let registry = two_session_registry();
        let target = Path::new("/worktrees/session-b/src/main.rs");
        let result = enforce_cross_session_access(
            &registry, "session-a", target, true, Uuid::nil(),
        );
        assert!(result.is_ok());
        let (check_result, fr_event) = result.unwrap();
        assert!(matches!(
            check_result,
            CrossSessionCheckResult::ApprovedOverride { .. }
        ));
        let fr_event = fr_event.expect("approved override must emit FR event");
        assert_eq!(
            fr_event.event_type,
            FlightRecorderEventType::WorkspaceCrossSessionApproved
        );
        assert_eq!(fr_event.payload["operator_approved"], true);
    }
}
