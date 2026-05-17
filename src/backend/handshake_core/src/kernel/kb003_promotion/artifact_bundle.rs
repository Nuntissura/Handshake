//! MT-040: Artifact Store Integration.
//!
//! Acceptance (MT-040.json): "store sandbox artifacts through validated
//! artifact system. Every artifact has stable handle and hash."
//!
//! The assembler takes a sandbox run + the validation evidence emitted during
//! that run and turns them into a `Kb003ArtifactBundleV1` whose entries each
//! carry:
//!
//! - stable handle  = (class, retention_root, content_sha256)
//! - hash policy    = the class's declared `HashPolicy` (per MT-009 table)
//! - content_sha256 = supplied by the caller (real backend computes it after
//!   bytes/JSON are written; in-memory tests pass a precomputed hash)
//!
//! The assembler refuses to emit a handle without a content hash, so promotion
//! cannot reference an artifact that has not been hashed-and-stored. Two
//! invocations on the same `(run_id, members)` produce the same bundle hash.
//!
//! This module sits *above* `kernel::validation::artifact_bundle::ArtifactBundleManifest`
//! (which is the per-validation-run hash projection) and reuses that
//! deterministic hash policy so promotion can refer to one bundle hash for a
//! complete sandbox run.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::kernel::kb003_artifact_classes::{
    metadata_for, HashPolicy, Kb003ArtifactClass, Kb003ArtifactMetadata,
};
use crate::kernel::sandbox::run::SandboxRunV1;
use crate::kernel::validation::artifact_bundle::{ArtifactBundleManifest, BundleMember};

/// Stable handle into the artifact store for a single artifact KB003 produced.
///
/// `handle` is the canonical durable reference (deterministic, content
/// addressed). The retention root is repo-relative under
/// `handshake-product/kb003/...` per MT-009; absolute resolution is the
/// storage layer's job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Kb003ArtifactHandleV1 {
    pub handle: String,
    pub class: Kb003ArtifactClass,
    pub content_sha256: String,
    pub hash_policy: HashPolicy,
    pub retention_root: String,
    pub exportable_by_default: bool,
}

impl Kb003ArtifactHandleV1 {
    /// Build a handle from a class + content hash. The handle string format is
    /// `kb003://<class_tag>/<sha256_prefix16>` so it is human-recognisable in
    /// receipts without leaking the full retention root.
    pub fn new(
        class: Kb003ArtifactClass,
        content_sha256: impl Into<String>,
    ) -> Result<Self, ArtifactBundleError> {
        let meta = metadata_for(class)
            .ok_or(ArtifactBundleError::UnknownArtifactClass)?;
        let content_sha256 = content_sha256.into();
        if content_sha256.trim().is_empty() {
            return Err(ArtifactBundleError::MissingContentHash);
        }
        let class_tag = class_tag(class);
        let prefix = content_sha256.chars().take(16).collect::<String>();
        let handle = format!("kb003://{class_tag}/{prefix}");
        Ok(Self {
            handle,
            class,
            content_sha256,
            hash_policy: meta.hash_policy,
            retention_root: meta.retention_root.to_string(),
            exportable_by_default: meta.exportable_by_default,
        })
    }

    /// Convert this handle to a `BundleMember` for hashing into the canonical
    /// per-bundle hash via `ArtifactBundleManifest`.
    pub fn as_bundle_member(&self) -> BundleMember {
        BundleMember::new(self.handle.clone(), self.class, self.content_sha256.clone())
    }
}

/// Canonical artifact-bundle record for one sandbox run. Schema id is
/// `hsk.kernel.sandbox_artifact_bundle@1`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Kb003ArtifactBundleV1 {
    pub bundle_id: Uuid,
    pub sandbox_run_id: String,
    pub handles: Vec<Kb003ArtifactHandleV1>,
    pub bundle_sha256: String,
}

impl Kb003ArtifactBundleV1 {
    /// Returns the metadata row backing each handle (1:1 with `handles`).
    pub fn metadata(&self) -> Vec<&'static Kb003ArtifactMetadata> {
        self.handles
            .iter()
            .map(|h| metadata_for(h.class).expect("class table covers all variants"))
            .collect()
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ArtifactBundleError {
    #[error("artifact class not registered in MT-009 metadata table")]
    UnknownArtifactClass,
    #[error("artifact handle requires a non-empty content sha256")]
    MissingContentHash,
    #[error("sandbox run id is required")]
    MissingRunId,
    #[error("artifact bundle must contain at least one handle")]
    EmptyBundle,
}

/// Builds a deterministic `Kb003ArtifactBundleV1` from a `SandboxRun` plus the
/// artifact handles produced during the run.
///
/// The assembler is intentionally storage-agnostic: it does not write bytes,
/// it only consumes already-hashed content references and arranges them into
/// the canonical bundle shape that promotion (MT-041..MT-049) consumes.
pub struct KbArtifactBundleAssembler;

impl KbArtifactBundleAssembler {
    pub fn assemble(
        run: &SandboxRunV1,
        handles: Vec<Kb003ArtifactHandleV1>,
    ) -> Result<Kb003ArtifactBundleV1, ArtifactBundleError> {
        if run.run_id.0.trim().is_empty() {
            return Err(ArtifactBundleError::MissingRunId);
        }
        if handles.is_empty() {
            return Err(ArtifactBundleError::EmptyBundle);
        }
        // Use the canonical manifest builder to compute a deterministic hash
        // that is identical across reorderings of the input set.
        let members: Vec<BundleMember> = handles.iter().map(|h| h.as_bundle_member()).collect();
        let manifest = ArtifactBundleManifest::build(members);
        let mut sorted_handles = handles;
        sorted_handles.sort_by(|a, b| {
            (a.handle.as_str(), a.content_sha256.as_str())
                .cmp(&(b.handle.as_str(), b.content_sha256.as_str()))
        });
        Ok(Kb003ArtifactBundleV1 {
            bundle_id: Uuid::now_v7(),
            sandbox_run_id: run.run_id.0.clone(),
            handles: sorted_handles,
            bundle_sha256: manifest.bundle_sha256,
        })
    }
}

fn class_tag(class: Kb003ArtifactClass) -> &'static str {
    match class {
        Kb003ArtifactClass::SandboxLog => "sandbox_log",
        Kb003ArtifactClass::SandboxDiff => "sandbox_diff",
        Kb003ArtifactClass::SandboxManifest => "sandbox_manifest",
        Kb003ArtifactClass::SandboxScreenshot => "sandbox_screenshot",
        Kb003ArtifactClass::ValidationReport => "validation_report",
        Kb003ArtifactClass::RedactionNote => "redaction_note",
        Kb003ArtifactClass::PromotionReceipt => "promotion_receipt",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::sandbox::run::{SandboxRunStatus, SandboxRunV1};

    fn fresh_run() -> SandboxRunV1 {
        SandboxRunV1::new_requested("KTR-1", "SES-1", "process_tier", "POL-1@1", "WSP-1")
    }

    fn h(class: Kb003ArtifactClass, hash: &str) -> Kb003ArtifactHandleV1 {
        Kb003ArtifactHandleV1::new(class, hash).unwrap()
    }

    // MT-040 acceptance: every artifact has stable handle and hash.
    #[test]
    fn handle_is_stable_and_includes_hash_prefix() {
        let a = h(Kb003ArtifactClass::SandboxLog, "abcdef0123456789aabbccddeeff0011");
        let b = h(Kb003ArtifactClass::SandboxLog, "abcdef0123456789aabbccddeeff0011");
        assert_eq!(a.handle, b.handle, "same class+hash => same handle");
        assert!(a.handle.starts_with("kb003://sandbox_log/"));
        assert!(a.handle.contains("abcdef0123456789"));
    }

    #[test]
    fn handle_refuses_empty_hash() {
        let err = Kb003ArtifactHandleV1::new(Kb003ArtifactClass::ValidationReport, "")
            .unwrap_err();
        assert_eq!(err, ArtifactBundleError::MissingContentHash);
    }

    #[test]
    fn handle_propagates_class_hash_policy() {
        let report = h(Kb003ArtifactClass::ValidationReport, "0123456789abcdef");
        assert_eq!(report.hash_policy, HashPolicy::CanonicalJsonSha256);
        assert!(report.exportable_by_default);

        let shot = h(Kb003ArtifactClass::SandboxScreenshot, "0123456789abcdef");
        assert_eq!(shot.hash_policy, HashPolicy::BinarySha256);
        assert!(!shot.exportable_by_default, "screenshots are export-gated");

        let red = h(Kb003ArtifactClass::RedactionNote, "0123456789abcdef");
        assert!(!red.exportable_by_default, "redaction notes are export-gated");
    }

    #[test]
    fn bundle_hash_is_deterministic_across_input_order() {
        let mut run = fresh_run();
        run.status = SandboxRunStatus::Completed;
        let handles = vec![
            h(Kb003ArtifactClass::SandboxLog, "h1aaaaaaaaaaaaaa"),
            h(Kb003ArtifactClass::SandboxDiff, "h2bbbbbbbbbbbbbb"),
            h(Kb003ArtifactClass::ValidationReport, "h3cccccccccccccc"),
        ];
        let bundle_a = KbArtifactBundleAssembler::assemble(&run, handles.clone()).unwrap();
        let mut reordered = handles;
        reordered.reverse();
        let bundle_b = KbArtifactBundleAssembler::assemble(&run, reordered).unwrap();
        assert_eq!(bundle_a.bundle_sha256, bundle_b.bundle_sha256);
        // Bundle id is fresh per call (no contract that they must match).
        assert_ne!(bundle_a.bundle_id, bundle_b.bundle_id);
        assert_eq!(bundle_a.handles, bundle_b.handles);
    }

    #[test]
    fn bundle_refuses_empty_handle_set() {
        let run = fresh_run();
        let err = KbArtifactBundleAssembler::assemble(&run, vec![]).unwrap_err();
        assert_eq!(err, ArtifactBundleError::EmptyBundle);
    }

    #[test]
    fn bundle_metadata_lookup_aligns_with_handles() {
        let run = fresh_run();
        let handles = vec![
            h(Kb003ArtifactClass::PromotionReceipt, "deadbeefcafebabe"),
            h(Kb003ArtifactClass::SandboxLog, "abad1deadeadc0de"),
        ];
        let bundle = KbArtifactBundleAssembler::assemble(&run, handles).unwrap();
        let meta = bundle.metadata();
        assert_eq!(meta.len(), bundle.handles.len());
        for (m, h) in meta.iter().zip(bundle.handles.iter()) {
            assert_eq!(m.class, h.class);
            assert_eq!(m.retention_root, h.retention_root);
        }
    }
}
