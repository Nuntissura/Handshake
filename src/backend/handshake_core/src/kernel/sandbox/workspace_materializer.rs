//! MT-028 Sandbox Workspace Materializer.
//!
//! Acceptance (MT-028.json): "materialize candidate inputs into isolated root.
//! Acceptance: no undeclared project files appear in sandbox input manifest."
//!
//! `WorkspaceMaterializer` takes a `CandidateInputSet` (relative paths declared
//! by the upstream packet) and produces a `SandboxInputManifestV1` listing the
//! files that landed in the sandbox root. The materializer:
//!
//!   * Refuses to add any entry whose source path is not in the declared
//!     candidate set (no undeclared files).
//!   * Refuses any path that escapes the workspace root (delegates to
//!     `FilesystemScopeGuard`-style lexical normalisation).
//!   * Emits a deterministic manifest order (sorted by sandbox-relative path)
//!     so replays are bit-stable.
//!
//! The materializer is filesystem-pure: it accepts a `MaterialisationHasher`
//! callback that returns the file digest. Tests inject a deterministic hasher.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::denial::{DenialKind, SandboxDenialRecordV1};
use super::run::SandboxRunV1;
use super::workspace::SandboxWorkspaceV1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateInputEntry {
    pub source_relative_path: String,
    pub declared_purpose: String,
    pub declared_digest_hint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CandidateInputSet {
    pub entries: Vec<CandidateInputEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxInputManifestEntryV1 {
    pub sandbox_relative_path: String,
    pub source_relative_path: String,
    pub declared_purpose: String,
    pub digest: String,
    pub byte_length: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxInputManifestV1 {
    pub manifest_id: String,
    pub workspace_id: String,
    pub created_at_utc: DateTime<Utc>,
    pub entries: Vec<SandboxInputManifestEntryV1>,
}

pub type MaterialisationHasher<'a> =
    &'a dyn Fn(&str) -> Result<(String, u64), MaterializationError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaterializationError {
    SourceUnreadable { source: String, reason: String },
    SourceNotDeclared { source: String },
    SandboxPathEscapes { sandbox_relative: String },
}

pub struct WorkspaceMaterializer<'a> {
    workspace: &'a SandboxWorkspaceV1,
    candidates: &'a CandidateInputSet,
}

impl<'a> WorkspaceMaterializer<'a> {
    pub fn new(workspace: &'a SandboxWorkspaceV1, candidates: &'a CandidateInputSet) -> Self {
        Self {
            workspace,
            candidates,
        }
    }

    /// Materialise declared inputs. `mapping` is the caller-supplied mapping
    /// from source-relative path to sandbox-relative path (so the candidate's
    /// on-disk layout can differ from the in-sandbox layout). Any sandbox path
    /// that escapes the workspace root, OR any source path that is not in the
    /// declared candidate set, returns a typed denial.
    pub fn materialise(
        &self,
        run: &SandboxRunV1,
        mapping: &BTreeMap<String, String>,
        hasher: MaterialisationHasher<'_>,
    ) -> Result<SandboxInputManifestV1, SandboxDenialRecordV1> {
        let declared: std::collections::HashSet<&str> = self
            .candidates
            .entries
            .iter()
            .map(|e| e.source_relative_path.as_str())
            .collect();

        // Pre-check the mapping: every key MUST be declared. Any undeclared
        // entry yields a typed denial naming the offending source path.
        for source in mapping.keys() {
            if !declared.contains(source.as_str()) {
                return Err(SandboxDenialRecordV1::new(
                    run.run_id.0.clone(),
                    run.policy_version_id.clone(),
                    DenialKind::WorkspaceBoundaryViolation,
                    None,
                    format!("materialise undeclared source `{}`", source),
                    "source path is not in CandidateInputSet; manifest would leak undeclared files"
                        .to_string(),
                ));
            }
        }

        let mut entries: Vec<SandboxInputManifestEntryV1> = Vec::new();
        for (source, sandbox_rel) in mapping {
            if !self.workspace.contains_relative(sandbox_rel) {
                return Err(SandboxDenialRecordV1::new(
                    run.run_id.0.clone(),
                    run.policy_version_id.clone(),
                    DenialKind::WorkspaceBoundaryViolation,
                    None,
                    format!("materialise into `{}`", sandbox_rel),
                    "sandbox-relative path escapes workspace root".to_string(),
                ));
            }
            let (digest, length) = hasher(source).map_err(|e| match e {
                MaterializationError::SourceUnreadable { source, reason } => {
                    SandboxDenialRecordV1::new(
                        run.run_id.0.clone(),
                        run.policy_version_id.clone(),
                        DenialKind::WorkspaceBoundaryViolation,
                        None,
                        format!("hash source `{}`", source),
                        format!("source unreadable: {}", reason),
                    )
                }
                other => SandboxDenialRecordV1::new(
                    run.run_id.0.clone(),
                    run.policy_version_id.clone(),
                    DenialKind::WorkspaceBoundaryViolation,
                    None,
                    format!("materialise `{}`", source),
                    format!("hasher rejected: {:?}", other),
                ),
            })?;
            // H-A3 fix: replace `.expect()` with typed denial propagation. The
            // invariant ("we already checked the declared set") holds today,
            // but a logic regression should surface as a structured denial,
            // not a sandbox-runtime panic.
            let declared_entry = match self
                .candidates
                .entries
                .iter()
                .find(|e| e.source_relative_path == *source)
            {
                Some(e) => e,
                None => {
                    return Err(SandboxDenialRecordV1::new(
                        run.run_id.0.clone(),
                        run.policy_version_id.clone(),
                        DenialKind::WorkspaceBoundaryViolation,
                        None,
                        format!("materialise `{}`", source),
                        "internal invariant: source missing from declared set after \
                         pre-check (regression — would have panicked before H-A3 fix)"
                            .to_string(),
                    ));
                }
            };
            entries.push(SandboxInputManifestEntryV1 {
                sandbox_relative_path: sandbox_rel.clone(),
                source_relative_path: source.clone(),
                declared_purpose: declared_entry.declared_purpose.clone(),
                digest,
                byte_length: length,
            });
        }
        entries.sort_by(|a, b| a.sandbox_relative_path.cmp(&b.sandbox_relative_path));

        Ok(SandboxInputManifestV1 {
            manifest_id: format!("MAN-{}", Uuid::now_v7()),
            workspace_id: self.workspace.workspace_id.clone(),
            created_at_utc: Utc::now(),
            entries,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run() -> SandboxRunV1 {
        SandboxRunV1::new_requested("KTR-1", "SES-1", "mat", "POL-1@1", "WSP-1")
    }

    fn workspace() -> SandboxWorkspaceV1 {
        SandboxWorkspaceV1::new_default("kb003", "handshake-product/kb003/work/x")
    }

    fn candidate_set() -> CandidateInputSet {
        CandidateInputSet {
            entries: vec![
                CandidateInputEntry {
                    source_relative_path: "src/lib.rs".into(),
                    declared_purpose: "compile.input".into(),
                    declared_digest_hint: None,
                },
                CandidateInputEntry {
                    source_relative_path: "Cargo.toml".into(),
                    declared_purpose: "compile.manifest".into(),
                    declared_digest_hint: None,
                },
            ],
        }
    }

    fn fake_hasher(
        path: &str,
    ) -> Result<(String, u64), MaterializationError> {
        Ok((format!("sha256:fake:{}", path), path.len() as u64))
    }

    #[test]
    fn undeclared_source_returns_typed_denial() {
        let ws = workspace();
        let cset = candidate_set();
        let mat = WorkspaceMaterializer::new(&ws, &cset);
        let mut mapping = BTreeMap::new();
        mapping.insert(
            "src/secret_leak.rs".to_string(),
            "handshake-product/kb003/work/x/src/secret_leak.rs".to_string(),
        );
        let den = mat
            .materialise(&run(), &mapping, &fake_hasher)
            .expect_err("undeclared source must deny");
        assert_eq!(den.kind, DenialKind::WorkspaceBoundaryViolation);
        assert!(den.reason.contains("CandidateInputSet"));
    }

    #[test]
    fn sandbox_path_escape_returns_typed_denial() {
        let ws = workspace();
        let cset = candidate_set();
        let mat = WorkspaceMaterializer::new(&ws, &cset);
        let mut mapping = BTreeMap::new();
        mapping.insert("src/lib.rs".to_string(), "../../../etc/passwd".to_string());
        let den = mat
            .materialise(&run(), &mapping, &fake_hasher)
            .expect_err("escape must deny");
        assert_eq!(den.kind, DenialKind::WorkspaceBoundaryViolation);
        assert!(den.reason.contains("escapes"));
    }

    #[test]
    fn happy_path_emits_deterministic_manifest() {
        let ws = workspace();
        let cset = candidate_set();
        let mat = WorkspaceMaterializer::new(&ws, &cset);
        let mut mapping = BTreeMap::new();
        mapping.insert(
            "src/lib.rs".into(),
            "handshake-product/kb003/work/x/src/lib.rs".into(),
        );
        mapping.insert(
            "Cargo.toml".into(),
            "handshake-product/kb003/work/x/Cargo.toml".into(),
        );
        let manifest = mat.materialise(&run(), &mapping, &fake_hasher).unwrap();
        assert_eq!(manifest.entries.len(), 2);
        // Sorted by sandbox path.
        assert!(
            manifest.entries[0].sandbox_relative_path
                <= manifest.entries[1].sandbox_relative_path
        );
        // No undeclared source landed.
        for e in &manifest.entries {
            let in_declared = cset
                .entries
                .iter()
                .any(|c| c.source_relative_path == e.source_relative_path);
            assert!(in_declared, "manifest entry not in declared set: {:?}", e);
        }
        assert!(manifest.manifest_id.starts_with("MAN-"));
        assert_eq!(manifest.workspace_id, ws.workspace_id);
    }

    #[test]
    fn empty_mapping_produces_empty_manifest() {
        let ws = workspace();
        let cset = candidate_set();
        let mat = WorkspaceMaterializer::new(&ws, &cset);
        let mapping = BTreeMap::new();
        let manifest = mat.materialise(&run(), &mapping, &fake_hasher).unwrap();
        assert!(manifest.entries.is_empty());
    }
}
