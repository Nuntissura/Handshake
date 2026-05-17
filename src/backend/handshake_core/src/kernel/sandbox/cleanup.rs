//! MT-029 Sandbox Cleanup and Retention.
//!
//! Acceptance (MT-029.json): "clean temp roots while preserving artifacts.
//! Acceptance: cleanup never deletes project files or authority rows."
//!
//! `CleanupPlan` is the deterministic plan a cleanup pass would execute against
//! a sandbox workspace once a run is terminal. Plans never touch filesystem
//! directly; the runner produces a plan and the storage layer applies it. This
//! lets tests and audits inspect every deletion before it happens.
//!
//! Three safety rails:
//!   1. Targets MUST resolve inside the workspace temp root. Anything outside
//!      the temp root is `RejectedTarget` with reason "not under temp root".
//!   2. Targets MUST NOT overlap any preserved artifact path (output roots,
//!      explicitly preserved files).
//!   3. Authority row identifiers (`SBX-...`, `DEN-...`, `MAN-...`, `POL-...`,
//!      `WSP-...`) are never included; the planner refuses any caller-supplied
//!      target that looks like an authority row id.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::workspace::SandboxWorkspaceV1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CleanupAction {
    DeleteFile,
    DeleteDir,
    TruncateFile,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedTarget {
    pub action: CleanupAction,
    pub workspace_relative_path: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RejectedTarget {
    pub workspace_relative_path: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CleanupPlanV1 {
    pub workspace_id: String,
    pub planned: Vec<PlannedTarget>,
    pub rejected: Vec<RejectedTarget>,
    pub preserved_paths: Vec<String>,
    pub created_at_utc: DateTime<Utc>,
}

pub struct CleanupPlanner<'a> {
    workspace: &'a SandboxWorkspaceV1,
    temp_root_relative: String,
    preserved_paths: Vec<String>,
}

impl<'a> CleanupPlanner<'a> {
    pub fn new(
        workspace: &'a SandboxWorkspaceV1,
        temp_root_relative: impl Into<String>,
    ) -> Self {
        let temp = temp_root_relative.into();
        // Preserved by default: every declared output root.
        let preserved = workspace.output_roots_relative.clone();
        Self {
            workspace,
            temp_root_relative: temp,
            preserved_paths: preserved,
        }
    }

    pub fn preserve(&mut self, path: impl Into<String>) {
        self.preserved_paths.push(path.into());
    }

    pub fn plan(&self, targets: &[(CleanupAction, String)]) -> CleanupPlanV1 {
        let mut planned: Vec<PlannedTarget> = Vec::new();
        let mut rejected: Vec<RejectedTarget> = Vec::new();
        for (action, raw) in targets {
            let path = raw.clone();
            // Authority row ids never appear as filesystem targets.
            if is_authority_id_like(&path) {
                rejected.push(RejectedTarget {
                    workspace_relative_path: path,
                    reason: "looks like an authority row id; refusing to delete authority data"
                        .into(),
                });
                continue;
            }
            // Workspace containment.
            if !self.workspace.contains_relative(&path) {
                rejected.push(RejectedTarget {
                    workspace_relative_path: path,
                    reason: "target escapes workspace root; refusing".into(),
                });
                continue;
            }
            // Must sit under the declared temp root.
            if !path_is_inside(&path, &self.temp_root_relative) {
                rejected.push(RejectedTarget {
                    workspace_relative_path: path,
                    reason: "target not under workspace temp root".into(),
                });
                continue;
            }
            // Must not overlap a preserved path.
            if self.preserved_paths.iter().any(|p| paths_overlap(&path, p)) {
                rejected.push(RejectedTarget {
                    workspace_relative_path: path,
                    reason: "target overlaps a preserved artifact path".into(),
                });
                continue;
            }
            planned.push(PlannedTarget {
                action: *action,
                workspace_relative_path: path,
                reason: "under temp root, no preserved overlap".into(),
            });
        }
        CleanupPlanV1 {
            workspace_id: self.workspace.workspace_id.clone(),
            planned,
            rejected,
            preserved_paths: self.preserved_paths.clone(),
            created_at_utc: Utc::now(),
        }
    }
}

fn is_authority_id_like(s: &str) -> bool {
    const PREFIXES: &[&str] = &[
        "SBX-", "DEN-", "MAN-", "POL-", "WSP-", "KTR-", "SES-", "ART-", "PRM-", "BLK-",
    ];
    let trimmed = s.trim();
    PREFIXES.iter().any(|p| trimmed.starts_with(p))
}

fn path_is_inside(candidate: &str, root: &str) -> bool {
    if candidate == root {
        return true;
    }
    let prefix = if root.ends_with('/') {
        root.to_string()
    } else {
        format!("{}/", root)
    };
    candidate.starts_with(&prefix)
}

fn paths_overlap(a: &str, b: &str) -> bool {
    path_is_inside(a, b) || path_is_inside(b, a)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ws() -> SandboxWorkspaceV1 {
        // root: handshake-product/kb003/work/x ; output_roots: .../x/out
        SandboxWorkspaceV1::new_default("kb003", "handshake-product/kb003/work/x")
    }

    #[test]
    fn temp_files_under_temp_root_are_planned() {
        let w = ws();
        let p = CleanupPlanner::new(&w, "handshake-product/kb003/work/x/tmp");
        let plan = p.plan(&[(
            CleanupAction::DeleteFile,
            "handshake-product/kb003/work/x/tmp/scratch.txt".to_string(),
        )]);
        assert_eq!(plan.planned.len(), 1);
        assert!(plan.rejected.is_empty());
    }

    #[test]
    fn output_root_paths_are_preserved() {
        let w = ws();
        let p = CleanupPlanner::new(&w, "handshake-product/kb003/work/x/tmp");
        let plan = p.plan(&[(
            CleanupAction::DeleteDir,
            "handshake-product/kb003/work/x/out".to_string(),
        )]);
        assert!(plan.planned.is_empty());
        assert_eq!(plan.rejected.len(), 1);
        assert!(plan.rejected[0].reason.contains("preserved"));
    }

    #[test]
    fn out_of_workspace_target_is_rejected() {
        let w = ws();
        let p = CleanupPlanner::new(&w, "handshake-product/kb003/work/x/tmp");
        let plan = p.plan(&[(
            CleanupAction::DeleteFile,
            "../../../etc/passwd".to_string(),
        )]);
        assert!(plan.planned.is_empty());
        assert_eq!(plan.rejected.len(), 1);
    }

    #[test]
    fn authority_id_target_is_rejected() {
        let w = ws();
        let p = CleanupPlanner::new(&w, "handshake-product/kb003/work/x/tmp");
        let plan = p.plan(&[
            (CleanupAction::DeleteFile, "SBX-abc".to_string()),
            (CleanupAction::DeleteFile, "DEN-xyz".to_string()),
            (CleanupAction::DeleteFile, "MAN-1".to_string()),
        ]);
        assert!(plan.planned.is_empty());
        assert_eq!(plan.rejected.len(), 3);
        for r in &plan.rejected {
            assert!(r.reason.contains("authority"));
        }
    }

    #[test]
    fn target_not_under_temp_root_is_rejected() {
        let w = ws();
        let p = CleanupPlanner::new(&w, "handshake-product/kb003/work/x/tmp");
        let plan = p.plan(&[(
            CleanupAction::DeleteFile,
            "handshake-product/kb003/work/x/src/lib.rs".to_string(),
        )]);
        assert!(plan.planned.is_empty());
        assert_eq!(plan.rejected.len(), 1);
        assert!(plan.rejected[0].reason.contains("temp root"));
    }

    #[test]
    fn explicit_preserved_path_blocks_cleanup() {
        let w = ws();
        let mut p = CleanupPlanner::new(&w, "handshake-product/kb003/work/x/tmp");
        p.preserve("handshake-product/kb003/work/x/tmp/keepme");
        let plan = p.plan(&[(
            CleanupAction::DeleteDir,
            "handshake-product/kb003/work/x/tmp/keepme".to_string(),
        )]);
        assert!(plan.planned.is_empty());
        assert_eq!(plan.rejected.len(), 1);
        assert!(plan.rejected[0].reason.contains("preserved"));
    }

    #[test]
    fn plan_carries_workspace_id_and_preserved_paths() {
        let w = ws();
        let p = CleanupPlanner::new(&w, "handshake-product/kb003/work/x/tmp");
        let plan = p.plan(&[]);
        assert_eq!(plan.workspace_id, w.workspace_id);
        assert!(plan
            .preserved_paths
            .iter()
            .any(|p| p.ends_with("/out")));
    }
}
