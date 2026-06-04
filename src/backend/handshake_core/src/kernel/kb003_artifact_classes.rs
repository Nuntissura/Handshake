//! KB003 sandbox / validation / promotion artifact taxonomy.
//!
//! MT-009 acceptance: each artifact class has content type, hash policy,
//! exportability default, and retention/default root. Classes are looked up by
//! variant; metadata is const-table so callers can match without runtime
//! allocation.
//!
//! Retention roots resolve under the external artifact root
//! `../Handshake_Artifacts/` per CX-212E. The default root listed here is the
//! repo-relative subfolder; absolute resolution is the storage layer's job.

use serde::{Deserialize, Serialize};

/// Stable taxonomy of artifact classes KB003 produces or links.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Kb003ArtifactClass {
    SandboxLog,
    SandboxDiff,
    SandboxManifest,
    SandboxScreenshot,
    ValidationReport,
    RedactionNote,
    PromotionReceipt,
}

/// How the hash for a stored artifact is computed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HashPolicy {
    /// Hash content bytes as-is (logs, diffs, manifests, reports).
    ContentSha256,
    /// Hash the canonicalized JSON form before storing (receipts/notes).
    CanonicalJsonSha256,
    /// Hash the raw bytes of a binary asset (screenshots).
    BinarySha256,
}

/// Metadata describing how an artifact class is stored, hashed, and exported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Kb003ArtifactMetadata {
    pub class: Kb003ArtifactClass,
    pub content_type: &'static str,
    pub hash_policy: HashPolicy,
    pub exportable_by_default: bool,
    pub retention_root: &'static str,
}

pub const KB003_ARTIFACT_CLASSES: &[Kb003ArtifactMetadata] = &[
    Kb003ArtifactMetadata {
        class: Kb003ArtifactClass::SandboxLog,
        content_type: "text/plain; charset=utf-8",
        hash_policy: HashPolicy::ContentSha256,
        exportable_by_default: true,
        retention_root: "handshake-product/kb003/sandbox/logs",
    },
    Kb003ArtifactMetadata {
        class: Kb003ArtifactClass::SandboxDiff,
        content_type: "text/x-diff",
        hash_policy: HashPolicy::ContentSha256,
        exportable_by_default: true,
        retention_root: "handshake-product/kb003/sandbox/diffs",
    },
    Kb003ArtifactMetadata {
        class: Kb003ArtifactClass::SandboxManifest,
        content_type: "application/json",
        hash_policy: HashPolicy::CanonicalJsonSha256,
        exportable_by_default: true,
        retention_root: "handshake-product/kb003/sandbox/manifests",
    },
    Kb003ArtifactMetadata {
        class: Kb003ArtifactClass::SandboxScreenshot,
        content_type: "image/png",
        hash_policy: HashPolicy::BinarySha256,
        exportable_by_default: false,
        retention_root: "handshake-product/kb003/sandbox/screenshots",
    },
    Kb003ArtifactMetadata {
        class: Kb003ArtifactClass::ValidationReport,
        content_type: "application/json",
        hash_policy: HashPolicy::CanonicalJsonSha256,
        exportable_by_default: true,
        retention_root: "handshake-product/kb003/validation/reports",
    },
    Kb003ArtifactMetadata {
        class: Kb003ArtifactClass::RedactionNote,
        content_type: "application/json",
        hash_policy: HashPolicy::CanonicalJsonSha256,
        exportable_by_default: false,
        retention_root: "handshake-product/kb003/validation/redactions",
    },
    Kb003ArtifactMetadata {
        class: Kb003ArtifactClass::PromotionReceipt,
        content_type: "application/json",
        hash_policy: HashPolicy::CanonicalJsonSha256,
        exportable_by_default: true,
        retention_root: "handshake-product/kb003/promotion/receipts",
    },
];

/// Lookup metadata for a class; panics only at test time on missing rows.
pub fn metadata_for(class: Kb003ArtifactClass) -> Option<&'static Kb003ArtifactMetadata> {
    KB003_ARTIFACT_CLASSES.iter().find(|m| m.class == class)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_variant_has_metadata() {
        let all = [
            Kb003ArtifactClass::SandboxLog,
            Kb003ArtifactClass::SandboxDiff,
            Kb003ArtifactClass::SandboxManifest,
            Kb003ArtifactClass::SandboxScreenshot,
            Kb003ArtifactClass::ValidationReport,
            Kb003ArtifactClass::RedactionNote,
            Kb003ArtifactClass::PromotionReceipt,
        ];
        for class in all {
            let m = metadata_for(class).expect("metadata row missing");
            assert!(!m.content_type.is_empty(), "{:?} content_type empty", class);
            assert!(
                !m.retention_root.is_empty(),
                "{:?} retention_root empty",
                class
            );
            assert!(
                m.retention_root.starts_with("handshake-product/"),
                "{:?} retention_root must live under external artifact root (CX-212E)",
                class
            );
        }
    }

    #[test]
    fn screenshots_and_redactions_are_export_gated() {
        // MT-009 risk: confidential evidence should not export by default.
        assert!(
            !metadata_for(Kb003ArtifactClass::SandboxScreenshot)
                .unwrap()
                .exportable_by_default
        );
        assert!(
            !metadata_for(Kb003ArtifactClass::RedactionNote)
                .unwrap()
                .exportable_by_default
        );
    }
}
