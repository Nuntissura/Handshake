//! MT-033: Diff capture.
//!
//! Acceptance: identical candidate produces identical diff artifact hash. The
//! `DiffArtifact` carries the raw diff bytes and a deterministic
//! `content_sha256` digest. Two calls to `DiffArtifact::capture` with the
//! same input produce equal hashes; any byte-level difference produces a
//! different hash. The artifact is classified as
//! `Kb003ArtifactClass::SandboxDiff` (content-hash policy).

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::kernel::kb003_artifact_classes::Kb003ArtifactClass;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffArtifact {
    pub artifact_class: Kb003ArtifactClass,
    pub diff_bytes: Vec<u8>,
    pub content_sha256: String,
}

impl DiffArtifact {
    /// Capture a candidate diff. The hash is deterministic over `diff_bytes`.
    pub fn capture(diff_bytes: impl Into<Vec<u8>>) -> Self {
        let diff_bytes = diff_bytes.into();
        let content_sha256 = sha256_hex(&diff_bytes);
        Self {
            artifact_class: Kb003ArtifactClass::SandboxDiff,
            diff_bytes,
            content_sha256,
        }
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let out = hasher.finalize();
    let mut s = String::with_capacity(out.len() * 2);
    for b in out.iter() {
        use std::fmt::Write as _;
        let _ = write!(s, "{:02x}", b);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_input_produces_identical_hash() {
        let a = DiffArtifact::capture(b"diff --git a/x b/x\n@@ -1 +1 @@\n-old\n+new\n".to_vec());
        let b = DiffArtifact::capture(b"diff --git a/x b/x\n@@ -1 +1 @@\n-old\n+new\n".to_vec());
        assert_eq!(a.content_sha256, b.content_sha256);
        assert_eq!(a.content_sha256.len(), 64);
        assert_eq!(a.artifact_class, Kb003ArtifactClass::SandboxDiff);
    }

    #[test]
    fn different_input_produces_different_hash() {
        let a = DiffArtifact::capture(b"diff a".to_vec());
        let b = DiffArtifact::capture(b"diff b".to_vec());
        assert_ne!(a.content_sha256, b.content_sha256);
    }
}
