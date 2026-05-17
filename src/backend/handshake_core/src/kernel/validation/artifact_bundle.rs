//! MT-034: Artifact bundle manifest (canonical bundle format).
//!
//! Acceptance: bundle hash is deterministic for same inputs. The manifest
//! canonicalizes members by (artifact_ref, content_sha256) and hashes the
//! sorted, canonical-JSON projection so two bundles built from the same set
//! of (ref, hash) pairs in any order produce the same `bundle_sha256`.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::kernel::kb003_artifact_classes::Kb003ArtifactClass;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BundleMember {
    pub artifact_ref: String,
    pub artifact_class: Kb003ArtifactClass,
    pub content_sha256: String,
}

impl BundleMember {
    pub fn new(
        artifact_ref: impl Into<String>,
        artifact_class: Kb003ArtifactClass,
        content_sha256: impl Into<String>,
    ) -> Self {
        Self {
            artifact_ref: artifact_ref.into(),
            artifact_class,
            content_sha256: content_sha256.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactBundleManifest {
    pub members: Vec<BundleMember>,
    pub bundle_sha256: String,
}

impl ArtifactBundleManifest {
    /// Build a manifest with deterministic ordering and a deterministic bundle hash.
    pub fn build(mut members: Vec<BundleMember>) -> Self {
        // Sort by (artifact_ref, content_sha256) so equal sets in any input
        // order produce the same bundle hash.
        members.sort_by(|a, b| {
            (a.artifact_ref.as_str(), a.content_sha256.as_str())
                .cmp(&(b.artifact_ref.as_str(), b.content_sha256.as_str()))
        });
        let mut hasher = Sha256::new();
        for m in &members {
            hasher.update(m.artifact_ref.as_bytes());
            hasher.update(b"\x1f");
            hasher.update(format!("{:?}", m.artifact_class).as_bytes());
            hasher.update(b"\x1f");
            hasher.update(m.content_sha256.as_bytes());
            hasher.update(b"\x1e");
        }
        let out = hasher.finalize();
        let mut bundle_sha256 = String::with_capacity(out.len() * 2);
        for b in out.iter() {
            use std::fmt::Write as _;
            let _ = write!(bundle_sha256, "{:02x}", b);
        }
        Self {
            members,
            bundle_sha256,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn members_alpha() -> Vec<BundleMember> {
        vec![
            BundleMember::new("a://1", Kb003ArtifactClass::SandboxLog, "h1"),
            BundleMember::new("a://2", Kb003ArtifactClass::SandboxDiff, "h2"),
            BundleMember::new("a://3", Kb003ArtifactClass::ValidationReport, "h3"),
        ]
    }

    #[test]
    fn same_inputs_produce_same_bundle_hash_independent_of_order() {
        let a = ArtifactBundleManifest::build(members_alpha());
        let mut reordered = members_alpha();
        reordered.reverse();
        let b = ArtifactBundleManifest::build(reordered);
        assert_eq!(a.bundle_sha256, b.bundle_sha256);
        assert_eq!(a.members, b.members);
    }

    #[test]
    fn different_inputs_produce_different_bundle_hash() {
        let a = ArtifactBundleManifest::build(members_alpha());
        let mut other = members_alpha();
        other.push(BundleMember::new(
            "a://4",
            Kb003ArtifactClass::SandboxLog,
            "h4",
        ));
        let b = ArtifactBundleManifest::build(other);
        assert_ne!(a.bundle_sha256, b.bundle_sha256);
    }

    #[test]
    fn hash_is_hex64() {
        let m = ArtifactBundleManifest::build(members_alpha());
        assert_eq!(m.bundle_sha256.len(), 64);
        assert!(m.bundle_sha256.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
