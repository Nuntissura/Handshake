//! Sandbox workspace boundary descriptor (schema id
//! `hsk.kernel.sandbox_workspace@1`).
//!
//! A workspace declares the root directory the sandbox is allowed to read and
//! write inside, plus output roots where post-run artifacts may be promoted.
//! Both are bound to the external artifact root `../Handshake_Artifacts/` per
//! CX-212E; the storage layer resolves absolute paths.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxWorkspaceV1 {
    pub workspace_id: String,
    pub root_relative_path: String,
    pub output_roots_relative: Vec<String>,
    pub label: String,
    pub allow_write: bool,
    pub allow_subprocess_cwd: bool,
}

impl SandboxWorkspaceV1 {
    pub fn new_default(label: impl Into<String>, root_relative: impl Into<String>) -> Self {
        let root = root_relative.into();
        Self {
            workspace_id: format!("WSP-{}", Uuid::new_v4()),
            root_relative_path: root.clone(),
            output_roots_relative: vec![format!("{}/out", root)],
            label: label.into(),
            allow_write: true,
            allow_subprocess_cwd: false,
        }
    }

    /// Returns `true` only if `candidate_relative` is inside the workspace root
    /// (no `..` escapes after lexical normalisation). Path resolution is
    /// lexical to keep the workspace boundary deterministic across replays.
    pub fn contains_relative(&self, candidate_relative: &str) -> bool {
        let normalised = lexical_normalise(candidate_relative);
        normalised.starts_with(&self.root_relative_path) && !normalised.contains("..")
    }
}

fn lexical_normalise(input: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for seg in input.split(['/', '\\']) {
        match seg {
            "" | "." => continue,
            ".." => {
                if parts.pop().is_none() {
                    // Escape attempt — preserve `..` so contains_relative returns false.
                    return format!("../{}", input);
                }
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_default_carries_output_root() {
        let ws = SandboxWorkspaceV1::new_default("kb003-mvp", "handshake-product/kb003/work/abc");
        assert!(ws.workspace_id.starts_with("WSP-"));
        assert_eq!(ws.output_roots_relative.len(), 1);
        assert!(ws.output_roots_relative[0].ends_with("/out"));
    }

    #[test]
    fn contains_relative_rejects_escape() {
        let ws = SandboxWorkspaceV1::new_default("kb003", "handshake-product/kb003/work/abc");
        assert!(ws.contains_relative("handshake-product/kb003/work/abc/sub/file.txt"));
        assert!(!ws.contains_relative("handshake-product/kb003/work/abc/../../secrets"));
        assert!(!ws.contains_relative("/etc/passwd"));
    }
}
