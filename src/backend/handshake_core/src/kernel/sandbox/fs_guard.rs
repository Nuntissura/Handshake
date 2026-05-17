//! MT-022 Filesystem Scope Guard.
//!
//! Acceptance (MT-022.json): "enforce read/write roots and prevent path escape.
//! Acceptance: all path escape attempts return typed denial evidence."
//!
//! `FilesystemScopeGuard` resolves an attempted access against the
//! `FilesystemScopeV1` carried in the policy bundle. Resolution is purely
//! lexical — the guard never touches the host filesystem — so the result is
//! deterministic across replays.
//!
//! Acceptance is enforced by emitting a typed `SandboxDenialRecordV1` with
//! `DenialKind::WorkspaceBoundaryViolation` for every escape attempt, including:
//!   * `..` traversal segments
//!   * absolute paths when the policy only allows relative roots
//!   * Windows drive prefixes / UNC paths
//!   * paths outside any listed read/write root
//!
//! The guard does NOT pretend to substitute for OS-level FS isolation; that is
//! the hard-isolation tier's job. The guard exists to make Process-tier and
//! stub adapters surface denials *before* spawning a child.

use serde::{Deserialize, Serialize};

use super::denial::{DenialKind, SandboxDenialRecordV1};
use super::policy_default_deny::FilesystemScopeV1;
use super::run::SandboxRunV1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FsAccessMode {
    Read,
    Write,
}

impl FsAccessMode {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Read => "READ",
            Self::Write => "WRITE",
        }
    }
}

pub struct FilesystemScopeGuard<'a> {
    scope: &'a FilesystemScopeV1,
}

impl<'a> FilesystemScopeGuard<'a> {
    pub fn new(scope: &'a FilesystemScopeV1) -> Self {
        Self { scope }
    }

    /// Decide whether `candidate_relative` is allowed under `mode`. Returns
    /// `Ok(())` if allowed; otherwise a typed denial record built against
    /// `run`'s identity.
    pub fn check(
        &self,
        run: &SandboxRunV1,
        candidate: &str,
        mode: FsAccessMode,
    ) -> Result<(), SandboxDenialRecordV1> {
        // 1. Reject obvious host-absolute / drive prefixes before normalisation.
        if is_absolute_or_drive(candidate) {
            return Err(self.denial(
                run,
                mode,
                candidate,
                "absolute or drive-prefixed path is outside any declared workspace root",
            ));
        }

        // 2. Reject UNC / network path shapes.
        if candidate.starts_with("\\\\") || candidate.starts_with("//") {
            return Err(self.denial(
                run,
                mode,
                candidate,
                "UNC / network path is outside any declared workspace root",
            ));
        }

        // 3. Lexical normalisation. If escape sentinel is present after
        //    normalisation, the path escapes.
        let normalised = lexical_normalise(candidate);
        if normalised.starts_with("..") || normalised.contains("/..") {
            return Err(self.denial(
                run,
                mode,
                candidate,
                "path traversal `..` segment escapes the workspace root",
            ));
        }

        // 4. Empty roots = deny-by-default for that mode.
        let roots = match mode {
            FsAccessMode::Read => &self.scope.read_roots,
            FsAccessMode::Write => &self.scope.write_roots,
        };
        if roots.is_empty() {
            return Err(self.denial(
                run,
                mode,
                candidate,
                &format!(
                    "no {} roots declared in policy; default-deny applies",
                    mode.as_str()
                ),
            ));
        }

        // 5. Path must sit under at least one declared root.
        if roots.iter().any(|r| path_is_inside(&normalised, r)) {
            Ok(())
        } else {
            Err(self.denial(
                run,
                mode,
                candidate,
                "path is not inside any declared root for this access mode",
            ))
        }
    }

    fn denial(
        &self,
        run: &SandboxRunV1,
        mode: FsAccessMode,
        candidate: &str,
        reason: &str,
    ) -> SandboxDenialRecordV1 {
        SandboxDenialRecordV1::new(
            run.run_id.0.clone(),
            run.policy_version_id.clone(),
            DenialKind::WorkspaceBoundaryViolation,
            None,
            format!("{} `{}`", mode.as_str(), candidate),
            reason.to_string(),
        )
    }
}

fn is_absolute_or_drive(p: &str) -> bool {
    if p.starts_with('/') || p.starts_with('\\') {
        return true;
    }
    // Windows drive prefix `C:` / `C:/...`.
    let bytes = p.as_bytes();
    if bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic() {
        return true;
    }
    false
}

fn lexical_normalise(input: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    let mut had_escape = false;
    for seg in input.split(['/', '\\']) {
        match seg {
            "" | "." => continue,
            ".." => {
                if parts.pop().is_none() {
                    had_escape = true;
                    parts.push("..");
                }
            }
            other => parts.push(other),
        }
    }
    if had_escape && parts.first() != Some(&"..") {
        parts.insert(0, "..");
    }
    parts.join("/")
}

fn path_is_inside(candidate: &str, root: &str) -> bool {
    let root_norm = lexical_normalise(root);
    if candidate == root_norm {
        return true;
    }
    let prefix = if root_norm.ends_with('/') {
        root_norm
    } else {
        format!("{}/", root_norm)
    };
    candidate.starts_with(&prefix)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::sandbox::policy_default_deny::FilesystemScopeV1;

    fn run() -> SandboxRunV1 {
        SandboxRunV1::new_requested("KTR-1", "SES-1", "fs-guard", "POL-1@1", "WSP-1")
    }

    fn scope() -> FilesystemScopeV1 {
        FilesystemScopeV1 {
            read_roots: vec!["handshake-product/kb003/work/x".into()],
            write_roots: vec!["handshake-product/kb003/work/x/out".into()],
        }
    }

    #[test]
    fn allowed_read_path_passes() {
        let s = scope();
        let g = FilesystemScopeGuard::new(&s);
        g.check(
            &run(),
            "handshake-product/kb003/work/x/sub/file.txt",
            FsAccessMode::Read,
        )
        .expect("path inside declared read root must pass");
    }

    #[test]
    fn dotdot_traversal_is_typed_denial() {
        let s = scope();
        let g = FilesystemScopeGuard::new(&s);
        let den = g
            .check(
                &run(),
                "handshake-product/kb003/work/x/../../secrets",
                FsAccessMode::Read,
            )
            .expect_err("escape must deny");
        assert_eq!(den.kind, DenialKind::WorkspaceBoundaryViolation);
        assert!(den.reason.contains("traversal"));
    }

    #[test]
    fn absolute_unix_path_is_typed_denial() {
        let s = scope();
        let g = FilesystemScopeGuard::new(&s);
        let den = g
            .check(&run(), "/etc/passwd", FsAccessMode::Read)
            .expect_err("absolute path must deny");
        assert_eq!(den.kind, DenialKind::WorkspaceBoundaryViolation);
        assert!(den.reason.contains("absolute"));
    }

    #[test]
    fn windows_drive_path_is_typed_denial() {
        let s = scope();
        let g = FilesystemScopeGuard::new(&s);
        let den = g
            .check(&run(), "C:/Windows/system32/cmd.exe", FsAccessMode::Read)
            .expect_err("drive prefix must deny");
        assert_eq!(den.kind, DenialKind::WorkspaceBoundaryViolation);
    }

    #[test]
    fn unc_path_is_typed_denial() {
        let s = scope();
        let g = FilesystemScopeGuard::new(&s);
        let den = g
            .check(&run(), "\\\\server\\share\\file", FsAccessMode::Read)
            .expect_err("UNC path must deny");
        assert_eq!(den.kind, DenialKind::WorkspaceBoundaryViolation);
        assert!(den.reason.contains("UNC"));
    }

    #[test]
    fn write_to_read_only_root_is_typed_denial() {
        let s = scope();
        let g = FilesystemScopeGuard::new(&s);
        // Path is inside read root but not write root.
        let den = g
            .check(
                &run(),
                "handshake-product/kb003/work/x/sub/file.txt",
                FsAccessMode::Write,
            )
            .expect_err("write outside write_roots must deny");
        assert_eq!(den.kind, DenialKind::WorkspaceBoundaryViolation);
        assert!(den.action_description.contains("WRITE"));
    }

    #[test]
    fn empty_roots_deny_by_default() {
        let s = FilesystemScopeV1::default();
        let g = FilesystemScopeGuard::new(&s);
        let den = g
            .check(&run(), "anything", FsAccessMode::Read)
            .expect_err("empty roots must deny everything");
        assert_eq!(den.kind, DenialKind::WorkspaceBoundaryViolation);
        assert!(den.reason.contains("no READ roots"));
    }

    #[test]
    fn root_path_itself_is_allowed() {
        let s = scope();
        let g = FilesystemScopeGuard::new(&s);
        g.check(
            &run(),
            "handshake-product/kb003/work/x",
            FsAccessMode::Read,
        )
        .expect("root itself must pass");
    }
}
