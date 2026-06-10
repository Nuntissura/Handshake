//! Source-evidence and anchor-verification matrix (WP-KERNEL-005 MT-001/MT-002).
//!
//! This is a product/runtime surface for no-context models, not repo-governance
//! paperwork. It records which legacy-source facts have verified product
//! anchors, what maturity state each source fact is in, and which expected
//! anchors are explicitly blocked as `BLOCKED_MISSING_ANCHOR`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;

use super::{reject_legacy_runtime_ref, AtelierError, AtelierResult, AtelierStore};

pub mod source_evidence_event_family {
    pub const SOURCE_EVIDENCE_MATRIX_RECORDED: &str = "atelier.source_evidence.matrix_recorded";

    pub const ALL: &[&str] = &[SOURCE_EVIDENCE_MATRIX_RECORDED];
}

pub use source_evidence_event_family::SOURCE_EVIDENCE_MATRIX_RECORDED;

pub const CORE_DATA_SOURCE_EVIDENCE_MATRIX_ID: &str = "wp-kernel-005.core-data.source-evidence@1";
pub const POSE_COMFY_SOURCE_EVIDENCE_MATRIX_ID: &str = "wp-kernel-005.pose-comfy.source-evidence@1";
pub const POSE_MEDIA_ANCHOR_VERIFICATION_MATRIX_ID: &str =
    "wp-kernel-005.pose-media.anchor-verification@1";

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SourceMaturityStatus {
    Done,
    Review,
    Blocked,
}

impl SourceMaturityStatus {
    pub fn as_token(self) -> &'static str {
        match self {
            SourceMaturityStatus::Done => "DONE",
            SourceMaturityStatus::Review => "REVIEW",
            SourceMaturityStatus::Blocked => "BLOCKED",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "DONE" => Ok(SourceMaturityStatus::Done),
            "REVIEW" => Ok(SourceMaturityStatus::Review),
            "BLOCKED" => Ok(SourceMaturityStatus::Blocked),
            other => Err(AtelierError::Validation(format!(
                "unknown source maturity status: {other}"
            ))),
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AnchorVerificationStatus {
    Verified,
    BlockedMissingAnchor,
}

impl AnchorVerificationStatus {
    pub fn as_token(self) -> &'static str {
        match self {
            AnchorVerificationStatus::Verified => "VERIFIED",
            AnchorVerificationStatus::BlockedMissingAnchor => "BLOCKED_MISSING_ANCHOR",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "VERIFIED" => Ok(AnchorVerificationStatus::Verified),
            "BLOCKED_MISSING_ANCHOR" => Ok(AnchorVerificationStatus::BlockedMissingAnchor),
            other => Err(AtelierError::Validation(format!(
                "unknown anchor verification status: {other}"
            ))),
        }
    }
}

#[derive(Clone, Debug)]
pub struct NewSourceEvidenceRecord {
    pub source_id: String,
    pub source_label: String,
    pub source_ref: String,
    pub product_area: String,
    pub maturity_status: SourceMaturityStatus,
    pub implementation_status: String,
    pub evidence_refs: Vec<String>,
    pub proof_refs: Vec<String>,
    pub gap_reason: Option<String>,
}

#[derive(Clone, Debug)]
pub struct NewAnchorVerificationRecord {
    pub anchor_id: String,
    pub source_id: String,
    pub anchor_label: String,
    pub expected_product_path: String,
    pub verification_status: AnchorVerificationStatus,
    pub verified_product_paths: Vec<String>,
    pub blocking_reason: Option<String>,
}

#[derive(Clone, Debug)]
pub struct NewSourceEvidenceMatrix {
    pub matrix_id: String,
    pub recorded_by: String,
    pub sources: Vec<NewSourceEvidenceRecord>,
    pub anchors: Vec<NewAnchorVerificationRecord>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceEvidenceRecord {
    pub source_id: String,
    pub matrix_id: String,
    pub source_label: String,
    pub source_ref: String,
    pub product_area: String,
    pub maturity_status: SourceMaturityStatus,
    pub implementation_status: String,
    pub evidence_refs: Vec<String>,
    pub proof_refs: Vec<String>,
    pub gap_reason: Option<String>,
    pub updated_at_utc: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AnchorVerificationRecord {
    pub anchor_id: String,
    pub matrix_id: String,
    pub source_id: String,
    pub anchor_label: String,
    pub expected_product_path: String,
    pub verification_status: AnchorVerificationStatus,
    pub verified_product_paths: Vec<String>,
    pub blocking_reason: Option<String>,
    pub updated_at_utc: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceEvidenceMatrix {
    pub matrix_id: String,
    pub sources: Vec<SourceEvidenceRecord>,
    pub anchors: Vec<AnchorVerificationRecord>,
}

pub fn core_data_source_evidence_matrix(recorded_by: impl Into<String>) -> NewSourceEvidenceMatrix {
    NewSourceEvidenceMatrix {
        matrix_id: CORE_DATA_SOURCE_EVIDENCE_MATRIX_ID.to_string(),
        recorded_by: recorded_by.into(),
        sources: vec![
            NewSourceEvidenceRecord {
                source_id: "MT-006.character-identity".to_string(),
                source_label: "Stable character identity".to_string(),
                source_ref: "source://legacy/character-identity".to_string(),
                product_area: "atelier.core".to_string(),
                maturity_status: SourceMaturityStatus::Done,
                implementation_status: "verified_product_path".to_string(),
                evidence_refs: vec![
                    "src/backend/handshake_core/src/atelier/core.rs".to_string(),
                    "src/backend/handshake_core/migrations/0030_atelier_foundation.sql"
                        .to_string(),
                ],
                proof_refs: vec![
                    "src/backend/handshake_core/tests/atelier_foundation_tests.rs".to_string(),
                    "src/backend/handshake_core/tests/atelier_core_data_tests.rs".to_string(),
                ],
                gap_reason: None,
            },
            NewSourceEvidenceRecord {
                source_id: "MT-008.sheet-template-parser".to_string(),
                source_label: "Typed sheet template parser".to_string(),
                source_ref: "source://legacy/sheet-template-parser".to_string(),
                product_area: "atelier.sheet".to_string(),
                maturity_status: SourceMaturityStatus::Review,
                implementation_status: "verified_product_path_needs_validator_refresh"
                    .to_string(),
                evidence_refs: vec![
                    "src/backend/handshake_core/src/atelier/sheet.rs".to_string(),
                    "src/backend/handshake_core/migrations/0037_atelier_sheet_parser_ast.sql"
                        .to_string(),
                ],
                proof_refs: vec![
                    "src/backend/handshake_core/tests/atelier_sheet_parser_tests.rs".to_string(),
                ],
                gap_reason: Some(
                    "Integration ledger predates current typed parser implementation proof"
                        .to_string(),
                ),
            },
            NewSourceEvidenceRecord {
                source_id: "MT-014.bulk-operations".to_string(),
                source_label: "Bulk tag/field/export/trash operations".to_string(),
                source_ref: "source://legacy/bulk-operations".to_string(),
                product_area: "atelier.bulk".to_string(),
                maturity_status: SourceMaturityStatus::Review,
                implementation_status: "partial_product_path".to_string(),
                evidence_refs: vec![
                    "src/backend/handshake_core/src/atelier/bulk.rs".to_string(),
                    "src/backend/handshake_core/migrations/0039_atelier_bulk_operation_receipts.sql"
                        .to_string(),
                ],
                proof_refs: vec![
                    "src/backend/handshake_core/tests/atelier_bulk_operations_tests.rs"
                        .to_string(),
                ],
                gap_reason: Some(
                    "Bulk receipt path exists; validator must still confirm all contract operations"
                        .to_string(),
                ),
            },
            NewSourceEvidenceRecord {
                source_id: "MT-018.media-review-metadata".to_string(),
                source_label: "Media review metadata".to_string(),
                source_ref: "source://legacy/media-review-metadata".to_string(),
                product_area: "atelier.review_metadata".to_string(),
                maturity_status: SourceMaturityStatus::Review,
                implementation_status: "verified_product_path_needs_validator_refresh"
                    .to_string(),
                evidence_refs: vec![
                    "src/backend/handshake_core/src/atelier/media.rs".to_string(),
                    "src/backend/handshake_core/migrations/0043_atelier_media_review_metadata.sql"
                        .to_string(),
                ],
                proof_refs: vec![
                    "src/backend/handshake_core/tests/atelier_media_artifact_tests.rs".to_string(),
                ],
                gap_reason: None,
            },
        ],
        anchors: vec![
            NewAnchorVerificationRecord {
                anchor_id: "ANCHOR-MT-006-core-rs".to_string(),
                source_id: "MT-006.character-identity".to_string(),
                anchor_label: "Character identity product module".to_string(),
                expected_product_path: "src/backend/handshake_core/src/atelier/core.rs".to_string(),
                verification_status: AnchorVerificationStatus::Verified,
                verified_product_paths: vec![
                    "src/backend/handshake_core/src/atelier/core.rs".to_string(),
                    "src/backend/handshake_core/tests/atelier_foundation_tests.rs".to_string(),
                ],
                blocking_reason: None,
            },
            NewAnchorVerificationRecord {
                anchor_id: "ANCHOR-MT-008-sheet-parser".to_string(),
                source_id: "MT-008.sheet-template-parser".to_string(),
                anchor_label: "Typed parser product module and migration".to_string(),
                expected_product_path: "src/backend/handshake_core/src/atelier/sheet.rs"
                    .to_string(),
                verification_status: AnchorVerificationStatus::Verified,
                verified_product_paths: vec![
                    "src/backend/handshake_core/src/atelier/sheet.rs".to_string(),
                    "src/backend/handshake_core/migrations/0037_atelier_sheet_parser_ast.sql"
                        .to_string(),
                    "src/backend/handshake_core/tests/atelier_sheet_parser_tests.rs".to_string(),
                ],
                blocking_reason: None,
            },
            NewAnchorVerificationRecord {
                anchor_id: "ANCHOR-MT-018-review-metadata".to_string(),
                source_id: "MT-018.media-review-metadata".to_string(),
                anchor_label: "Media review metadata product module".to_string(),
                expected_product_path: "src/backend/handshake_core/src/atelier/media.rs"
                    .to_string(),
                verification_status: AnchorVerificationStatus::Verified,
                verified_product_paths: vec![
                    "src/backend/handshake_core/src/atelier/media.rs".to_string(),
                    "src/backend/handshake_core/migrations/0043_atelier_media_review_metadata.sql"
                        .to_string(),
                    "src/backend/handshake_core/tests/atelier_media_artifact_tests.rs".to_string(),
                ],
                blocking_reason: None,
            },
        ],
    }
}

pub fn pose_comfy_source_evidence_matrix(
    recorded_by: impl Into<String>,
) -> NewSourceEvidenceMatrix {
    NewSourceEvidenceMatrix {
        matrix_id: POSE_COMFY_SOURCE_EVIDENCE_MATRIX_ID.to_string(),
        recorded_by: recorded_by.into(),
        sources: vec![
            NewSourceEvidenceRecord {
                source_id: "MT-081.posekit".to_string(),
                source_label: "PoseKit pose rig storage".to_string(),
                source_ref: "source://legacy/posekit".to_string(),
                product_area: "atelier.pose".to_string(),
                maturity_status: SourceMaturityStatus::Done,
                implementation_status: "verified_product_path".to_string(),
                evidence_refs: vec![
                    "src/backend/handshake_core/src/atelier/pose.rs".to_string(),
                    "src/backend/handshake_core/migrations/0032_atelier_pose_diagnostics.sql"
                        .to_string(),
                ],
                proof_refs: vec![
                    "src/backend/handshake_core/tests/atelier_pose_tests.rs".to_string()
                ],
                gap_reason: None,
            },
            NewSourceEvidenceRecord {
                source_id: "MT-081.openpose".to_string(),
                source_label: "OpenPose sidecar payload contract".to_string(),
                source_ref: "source://legacy/openpose".to_string(),
                product_area: "atelier.pose".to_string(),
                maturity_status: SourceMaturityStatus::Done,
                implementation_status: "verified_product_path".to_string(),
                evidence_refs: vec![
                    "src/backend/handshake_core/src/atelier/pose.rs".to_string(),
                    "src/backend/handshake_core/src/atelier/media.rs".to_string(),
                    "src/backend/handshake_core/migrations/0047_atelier_media_sidecars.sql"
                        .to_string(),
                ],
                proof_refs: vec![
                    "src/backend/handshake_core/tests/atelier_pose_tests.rs".to_string(),
                    "src/backend/handshake_core/tests/atelier_media_artifact_tests.rs".to_string(),
                ],
                gap_reason: None,
            },
            NewSourceEvidenceRecord {
                source_id: "MT-081.identity".to_string(),
                source_label: "Pose identity profile history".to_string(),
                source_ref: "source://legacy/identity-profile".to_string(),
                product_area: "atelier.pose".to_string(),
                maturity_status: SourceMaturityStatus::Done,
                implementation_status: "verified_product_path".to_string(),
                evidence_refs: vec![
                    "src/backend/handshake_core/src/atelier/pose.rs".to_string(),
                    "src/backend/handshake_core/migrations/0032_atelier_pose_diagnostics.sql"
                        .to_string(),
                ],
                proof_refs: vec![
                    "src/backend/handshake_core/tests/atelier_pose_tests.rs".to_string()
                ],
                gap_reason: None,
            },
            NewSourceEvidenceRecord {
                source_id: "MT-081.comfyui".to_string(),
                source_label: "ComfyUI custom-node intake bridge".to_string(),
                source_ref: "source://legacy/comfyui-custom-node".to_string(),
                product_area: "atelier.comfy".to_string(),
                maturity_status: SourceMaturityStatus::Done,
                implementation_status: "verified_product_path".to_string(),
                evidence_refs: vec![
                    "src/backend/handshake_core/src/atelier/comfy.rs".to_string(),
                    "src/backend/handshake_core/migrations/0032_atelier_pose_diagnostics.sql"
                        .to_string(),
                ],
                proof_refs: vec![
                    "src/backend/handshake_core/tests/atelier_comfy_tests.rs".to_string()
                ],
                gap_reason: None,
            },
            NewSourceEvidenceRecord {
                source_id: "MT-081.workflow-registry".to_string(),
                source_label: "Workflow registry command-corpus bridge".to_string(),
                source_ref: "source://legacy/workflow-registry".to_string(),
                product_area: "atelier.command_corpus".to_string(),
                maturity_status: SourceMaturityStatus::Done,
                implementation_status: "verified_product_path".to_string(),
                evidence_refs: vec![
                    "src/backend/handshake_core/src/atelier/command_corpus.rs".to_string(),
                    "src/backend/handshake_core/migrations/0032_atelier_pose_diagnostics.sql"
                        .to_string(),
                ],
                proof_refs: vec![
                    "src/backend/handshake_core/tests/atelier_command_corpus_tests.rs".to_string(),
                ],
                gap_reason: None,
            },
            NewSourceEvidenceRecord {
                source_id: "MT-081.image-sourcing-adapter".to_string(),
                source_label: "Image sourcing adapter and handler matrix".to_string(),
                source_ref: "source://legacy/image-sourcing-adapter".to_string(),
                product_area: "atelier.sourcing".to_string(),
                maturity_status: SourceMaturityStatus::Done,
                implementation_status: "verified_product_path".to_string(),
                evidence_refs: vec![
                    "src/backend/handshake_core/src/atelier/sourcing.rs".to_string(),
                    "src/backend/handshake_core/migrations/0032_atelier_pose_diagnostics.sql"
                        .to_string(),
                ],
                proof_refs: vec![
                    "src/backend/handshake_core/tests/atelier_sourcing_tests.rs".to_string()
                ],
                gap_reason: None,
            },
        ],
        anchors: vec![
            NewAnchorVerificationRecord {
                anchor_id: "ANCHOR-MT-081-posekit".to_string(),
                source_id: "MT-081.posekit".to_string(),
                anchor_label: "PoseKit pose rig storage module".to_string(),
                expected_product_path: "src/backend/handshake_core/src/atelier/pose.rs".to_string(),
                verification_status: AnchorVerificationStatus::Verified,
                verified_product_paths: vec![
                    "src/backend/handshake_core/src/atelier/pose.rs".to_string(),
                    "src/backend/handshake_core/tests/atelier_pose_tests.rs".to_string(),
                ],
                blocking_reason: None,
            },
            NewAnchorVerificationRecord {
                anchor_id: "ANCHOR-MT-081-openpose".to_string(),
                source_id: "MT-081.openpose".to_string(),
                anchor_label: "OpenPose payload and sidecar relation".to_string(),
                expected_product_path: "src/backend/handshake_core/src/atelier/pose.rs".to_string(),
                verification_status: AnchorVerificationStatus::Verified,
                verified_product_paths: vec![
                    "src/backend/handshake_core/src/atelier/pose.rs".to_string(),
                    "src/backend/handshake_core/src/atelier/media.rs".to_string(),
                    "src/backend/handshake_core/tests/atelier_pose_tests.rs".to_string(),
                    "src/backend/handshake_core/tests/atelier_media_artifact_tests.rs".to_string(),
                ],
                blocking_reason: None,
            },
            NewAnchorVerificationRecord {
                anchor_id: "ANCHOR-MT-081-identity".to_string(),
                source_id: "MT-081.identity".to_string(),
                anchor_label: "Identity profile append-only pose records".to_string(),
                expected_product_path: "src/backend/handshake_core/src/atelier/pose.rs".to_string(),
                verification_status: AnchorVerificationStatus::Verified,
                verified_product_paths: vec![
                    "src/backend/handshake_core/src/atelier/pose.rs".to_string(),
                    "src/backend/handshake_core/tests/atelier_pose_tests.rs".to_string(),
                ],
                blocking_reason: None,
            },
            NewAnchorVerificationRecord {
                anchor_id: "ANCHOR-MT-081-comfyui".to_string(),
                source_id: "MT-081.comfyui".to_string(),
                anchor_label: "ComfyUI governed intake bridge".to_string(),
                expected_product_path: "src/backend/handshake_core/src/atelier/comfy.rs"
                    .to_string(),
                verification_status: AnchorVerificationStatus::Verified,
                verified_product_paths: vec![
                    "src/backend/handshake_core/src/atelier/comfy.rs".to_string(),
                    "src/backend/handshake_core/tests/atelier_comfy_tests.rs".to_string(),
                ],
                blocking_reason: None,
            },
            NewAnchorVerificationRecord {
                anchor_id: "ANCHOR-MT-081-workflow-registry".to_string(),
                source_id: "MT-081.workflow-registry".to_string(),
                anchor_label: "Workflow registry command-corpus record".to_string(),
                expected_product_path: "src/backend/handshake_core/src/atelier/command_corpus.rs"
                    .to_string(),
                verification_status: AnchorVerificationStatus::Verified,
                verified_product_paths: vec![
                    "src/backend/handshake_core/src/atelier/command_corpus.rs".to_string(),
                    "src/backend/handshake_core/tests/atelier_command_corpus_tests.rs".to_string(),
                ],
                blocking_reason: None,
            },
            NewAnchorVerificationRecord {
                anchor_id: "ANCHOR-MT-081-image-sourcing-adapter".to_string(),
                source_id: "MT-081.image-sourcing-adapter".to_string(),
                anchor_label: "Image sourcing adapter handler matrix".to_string(),
                expected_product_path: "src/backend/handshake_core/src/atelier/sourcing.rs"
                    .to_string(),
                verification_status: AnchorVerificationStatus::Verified,
                verified_product_paths: vec![
                    "src/backend/handshake_core/src/atelier/sourcing.rs".to_string(),
                    "src/backend/handshake_core/tests/atelier_sourcing_tests.rs".to_string(),
                ],
                blocking_reason: None,
            },
        ],
    }
}

/// MT-082 Product Anchor Verification for the pose + media product surfaces.
///
/// Where MT-081's [`pose_comfy_source_evidence_matrix`] records pose/comfy
/// *source maturity*, MT-082 records an explicit per-domain **anchor
/// verification** artifact: for each pose/media product anchor the contract
/// names (pose, media, artifact, workflow, external tools, diagnostics) it
/// asserts either `VERIFIED` with the real product paths that back the anchor,
/// or `BLOCKED_MISSING_ANCHOR` with a blocking_reason. All paths cited here
/// resolve to modules/migrations/tests that exist in the product source tree,
/// so every anchor in this matrix is `VERIFIED`.
pub fn pose_media_anchor_verification_matrix(
    recorded_by: impl Into<String>,
) -> NewSourceEvidenceMatrix {
    NewSourceEvidenceMatrix {
        matrix_id: POSE_MEDIA_ANCHOR_VERIFICATION_MATRIX_ID.to_string(),
        recorded_by: recorded_by.into(),
        sources: vec![
            NewSourceEvidenceRecord {
                source_id: "MT-082.pose-anchor".to_string(),
                source_label: "Pose product surface anchor".to_string(),
                source_ref: "source://legacy/pose-product-anchor".to_string(),
                product_area: "atelier.pose".to_string(),
                maturity_status: SourceMaturityStatus::Done,
                implementation_status: "verified_product_path".to_string(),
                evidence_refs: vec![
                    "src/backend/handshake_core/src/atelier/pose.rs".to_string(),
                    "src/backend/handshake_core/migrations/0090_atelier_pose_sidecars.sql"
                        .to_string(),
                ],
                proof_refs: vec![
                    "src/backend/handshake_core/tests/atelier_pose_tests.rs".to_string()
                ],
                gap_reason: None,
            },
            NewSourceEvidenceRecord {
                source_id: "MT-082.media-anchor".to_string(),
                source_label: "Media product surface anchor".to_string(),
                source_ref: "source://legacy/media-product-anchor".to_string(),
                product_area: "atelier.media".to_string(),
                maturity_status: SourceMaturityStatus::Done,
                implementation_status: "verified_product_path".to_string(),
                evidence_refs: vec![
                    "src/backend/handshake_core/src/atelier/media.rs".to_string(),
                    "src/backend/handshake_core/migrations/0043_atelier_media_review_metadata.sql"
                        .to_string(),
                ],
                proof_refs: vec![
                    "src/backend/handshake_core/tests/atelier_media_artifact_tests.rs".to_string(),
                ],
                gap_reason: None,
            },
            NewSourceEvidenceRecord {
                source_id: "MT-082.artifact-anchor".to_string(),
                source_label: "Media artifact manifest product surface anchor".to_string(),
                source_ref: "source://legacy/artifact-product-anchor".to_string(),
                product_area: "atelier.media".to_string(),
                maturity_status: SourceMaturityStatus::Done,
                implementation_status: "verified_product_path".to_string(),
                evidence_refs: vec![
                    "src/backend/handshake_core/src/atelier/media.rs".to_string(),
                    "src/backend/handshake_core/migrations/0040_atelier_media_artifact_manifest.sql"
                        .to_string(),
                ],
                proof_refs: vec![
                    "src/backend/handshake_core/tests/atelier_media_artifact_tests.rs".to_string(),
                ],
                gap_reason: None,
            },
            NewSourceEvidenceRecord {
                source_id: "MT-082.workflow-anchor".to_string(),
                source_label: "Workflow command-corpus product surface anchor".to_string(),
                source_ref: "source://legacy/workflow-product-anchor".to_string(),
                product_area: "atelier.command_corpus".to_string(),
                maturity_status: SourceMaturityStatus::Done,
                implementation_status: "verified_product_path".to_string(),
                evidence_refs: vec![
                    "src/backend/handshake_core/src/atelier/command_corpus.rs".to_string(),
                ],
                proof_refs: vec![
                    "src/backend/handshake_core/tests/atelier_command_corpus_tests.rs".to_string(),
                ],
                gap_reason: None,
            },
            NewSourceEvidenceRecord {
                source_id: "MT-082.external-tools-anchor".to_string(),
                source_label: "External tools (ComfyUI) product surface anchor".to_string(),
                source_ref: "source://legacy/external-tools-product-anchor".to_string(),
                product_area: "atelier.comfy".to_string(),
                maturity_status: SourceMaturityStatus::Done,
                implementation_status: "verified_product_path".to_string(),
                evidence_refs: vec![
                    "src/backend/handshake_core/src/atelier/comfy.rs".to_string()
                ],
                proof_refs: vec![
                    "src/backend/handshake_core/tests/atelier_comfy_tests.rs".to_string()
                ],
                gap_reason: None,
            },
            NewSourceEvidenceRecord {
                source_id: "MT-082.diagnostics-anchor".to_string(),
                source_label: "Pose diagnostics product surface anchor".to_string(),
                source_ref: "source://legacy/diagnostics-product-anchor".to_string(),
                product_area: "atelier.pose".to_string(),
                maturity_status: SourceMaturityStatus::Done,
                implementation_status: "verified_product_path".to_string(),
                evidence_refs: vec![
                    "src/backend/handshake_core/src/atelier/pose.rs".to_string(),
                    "src/backend/handshake_core/migrations/0032_atelier_pose_diagnostics.sql"
                        .to_string(),
                ],
                proof_refs: vec![
                    "src/backend/handshake_core/tests/atelier_pose_tests.rs".to_string()
                ],
                gap_reason: None,
            },
        ],
        anchors: vec![
            NewAnchorVerificationRecord {
                anchor_id: "ANCHOR-MT-082-pose".to_string(),
                source_id: "MT-082.pose-anchor".to_string(),
                anchor_label: "Pose product module and pose-sidecar migration".to_string(),
                expected_product_path: "src/backend/handshake_core/src/atelier/pose.rs".to_string(),
                verification_status: AnchorVerificationStatus::Verified,
                verified_product_paths: vec![
                    "src/backend/handshake_core/src/atelier/pose.rs".to_string(),
                    "src/backend/handshake_core/migrations/0090_atelier_pose_sidecars.sql"
                        .to_string(),
                    "src/backend/handshake_core/tests/atelier_pose_tests.rs".to_string(),
                ],
                blocking_reason: None,
            },
            NewAnchorVerificationRecord {
                anchor_id: "ANCHOR-MT-082-media".to_string(),
                source_id: "MT-082.media-anchor".to_string(),
                anchor_label: "Media review-metadata product module and migration".to_string(),
                expected_product_path: "src/backend/handshake_core/src/atelier/media.rs"
                    .to_string(),
                verification_status: AnchorVerificationStatus::Verified,
                verified_product_paths: vec![
                    "src/backend/handshake_core/src/atelier/media.rs".to_string(),
                    "src/backend/handshake_core/migrations/0043_atelier_media_review_metadata.sql"
                        .to_string(),
                    "src/backend/handshake_core/tests/atelier_media_artifact_tests.rs".to_string(),
                ],
                blocking_reason: None,
            },
            NewAnchorVerificationRecord {
                anchor_id: "ANCHOR-MT-082-artifact".to_string(),
                source_id: "MT-082.artifact-anchor".to_string(),
                anchor_label: "Media artifact manifest product module and migration".to_string(),
                expected_product_path:
                    "src/backend/handshake_core/migrations/0040_atelier_media_artifact_manifest.sql"
                        .to_string(),
                verification_status: AnchorVerificationStatus::Verified,
                verified_product_paths: vec![
                    "src/backend/handshake_core/src/atelier/media.rs".to_string(),
                    "src/backend/handshake_core/migrations/0040_atelier_media_artifact_manifest.sql"
                        .to_string(),
                    "src/backend/handshake_core/tests/atelier_media_artifact_tests.rs".to_string(),
                ],
                blocking_reason: None,
            },
            NewAnchorVerificationRecord {
                anchor_id: "ANCHOR-MT-082-workflow".to_string(),
                source_id: "MT-082.workflow-anchor".to_string(),
                anchor_label: "Workflow command-corpus product module".to_string(),
                expected_product_path: "src/backend/handshake_core/src/atelier/command_corpus.rs"
                    .to_string(),
                verification_status: AnchorVerificationStatus::Verified,
                verified_product_paths: vec![
                    "src/backend/handshake_core/src/atelier/command_corpus.rs".to_string(),
                    "src/backend/handshake_core/tests/atelier_command_corpus_tests.rs".to_string(),
                ],
                blocking_reason: None,
            },
            NewAnchorVerificationRecord {
                anchor_id: "ANCHOR-MT-082-external-tools".to_string(),
                source_id: "MT-082.external-tools-anchor".to_string(),
                anchor_label: "External tools (ComfyUI) governed intake product module"
                    .to_string(),
                expected_product_path: "src/backend/handshake_core/src/atelier/comfy.rs"
                    .to_string(),
                verification_status: AnchorVerificationStatus::Verified,
                verified_product_paths: vec![
                    "src/backend/handshake_core/src/atelier/comfy.rs".to_string(),
                    "src/backend/handshake_core/tests/atelier_comfy_tests.rs".to_string(),
                ],
                blocking_reason: None,
            },
            NewAnchorVerificationRecord {
                anchor_id: "ANCHOR-MT-082-diagnostics".to_string(),
                source_id: "MT-082.diagnostics-anchor".to_string(),
                anchor_label: "Pose diagnostics product module and migration".to_string(),
                expected_product_path:
                    "src/backend/handshake_core/migrations/0032_atelier_pose_diagnostics.sql"
                        .to_string(),
                verification_status: AnchorVerificationStatus::Verified,
                verified_product_paths: vec![
                    "src/backend/handshake_core/src/atelier/pose.rs".to_string(),
                    "src/backend/handshake_core/migrations/0032_atelier_pose_diagnostics.sql"
                        .to_string(),
                    "src/backend/handshake_core/tests/atelier_pose_tests.rs".to_string(),
                ],
                blocking_reason: None,
            },
        ],
    }
}

impl AtelierStore {
    pub async fn record_source_evidence_matrix(
        &self,
        input: &NewSourceEvidenceMatrix,
    ) -> AtelierResult<SourceEvidenceMatrix> {
        validate_matrix(input)?;
        let evidence_count = input.sources.len();
        let verified_anchor_count = input
            .anchors
            .iter()
            .filter(|anchor| anchor.verification_status == AnchorVerificationStatus::Verified)
            .count();
        let blocked_anchor_count = input
            .anchors
            .iter()
            .filter(|anchor| {
                anchor.verification_status == AnchorVerificationStatus::BlockedMissingAnchor
            })
            .count();

        let mut tx = self.pool().begin().await?;
        let source_ids = input
            .sources
            .iter()
            .map(|source| source.source_id.clone())
            .collect::<Vec<_>>();
        let anchor_ids = input
            .anchors
            .iter()
            .map(|anchor| anchor.anchor_id.clone())
            .collect::<Vec<_>>();
        sqlx::query(
            r#"DELETE FROM atelier_anchor_verification_record
               WHERE matrix_id = $1
                 AND NOT (anchor_id = ANY($2::text[]))"#,
        )
        .bind(&input.matrix_id)
        .bind(&anchor_ids)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            r#"DELETE FROM atelier_source_evidence_record
               WHERE matrix_id = $1
                 AND NOT (source_id = ANY($2::text[]))"#,
        )
        .bind(&input.matrix_id)
        .bind(&source_ids)
        .execute(&mut *tx)
        .await?;
        for source in &input.sources {
            let evidence_refs = serde_json::to_value(&source.evidence_refs)
                .map_err(|err| AtelierError::Validation(err.to_string()))?;
            let proof_refs = serde_json::to_value(&source.proof_refs)
                .map_err(|err| AtelierError::Validation(err.to_string()))?;
            sqlx::query(
                r#"INSERT INTO atelier_source_evidence_record (
                       source_id, matrix_id, source_label, source_ref, product_area,
                       maturity_status, implementation_status, evidence_refs, proof_refs,
                       gap_reason, updated_at_utc
                   )
                   VALUES ($1, $2, $3, $4, $5, $6, $7, $8::jsonb, $9::jsonb, $10, NOW())
                   ON CONFLICT (matrix_id, source_id) DO UPDATE SET
                       source_label = EXCLUDED.source_label,
                       source_ref = EXCLUDED.source_ref,
                       product_area = EXCLUDED.product_area,
                       maturity_status = EXCLUDED.maturity_status,
                       implementation_status = EXCLUDED.implementation_status,
                       evidence_refs = EXCLUDED.evidence_refs,
                       proof_refs = EXCLUDED.proof_refs,
                       gap_reason = EXCLUDED.gap_reason,
                       updated_at_utc = NOW()"#,
            )
            .bind(&source.source_id)
            .bind(&input.matrix_id)
            .bind(&source.source_label)
            .bind(&source.source_ref)
            .bind(&source.product_area)
            .bind(source.maturity_status.as_token())
            .bind(&source.implementation_status)
            .bind(evidence_refs)
            .bind(proof_refs)
            .bind(source.gap_reason.as_deref())
            .execute(&mut *tx)
            .await?;
        }

        for anchor in &input.anchors {
            let verified_product_paths = serde_json::to_value(&anchor.verified_product_paths)
                .map_err(|err| AtelierError::Validation(err.to_string()))?;
            sqlx::query(
                r#"INSERT INTO atelier_anchor_verification_record (
                       anchor_id, matrix_id, source_id, anchor_label, expected_product_path,
                       verification_status, verified_product_paths, blocking_reason,
                       updated_at_utc
                   )
                   VALUES ($1, $2, $3, $4, $5, $6, $7::jsonb, $8, NOW())
                   ON CONFLICT (matrix_id, anchor_id) DO UPDATE SET
                       source_id = EXCLUDED.source_id,
                       anchor_label = EXCLUDED.anchor_label,
                       expected_product_path = EXCLUDED.expected_product_path,
                       verification_status = EXCLUDED.verification_status,
                       verified_product_paths = EXCLUDED.verified_product_paths,
                       blocking_reason = EXCLUDED.blocking_reason,
                       updated_at_utc = NOW()"#,
            )
            .bind(&anchor.anchor_id)
            .bind(&input.matrix_id)
            .bind(&anchor.source_id)
            .bind(&anchor.anchor_label)
            .bind(&anchor.expected_product_path)
            .bind(anchor.verification_status.as_token())
            .bind(verified_product_paths)
            .bind(anchor.blocking_reason.as_deref())
            .execute(&mut *tx)
            .await?;
        }

        self.record_event_in_tx(
            &mut tx,
            source_evidence_event_family::SOURCE_EVIDENCE_MATRIX_RECORDED,
            "atelier_source_evidence_matrix",
            &input.matrix_id,
            serde_json::json!({
                "matrix_id": input.matrix_id,
                "recorded_by": input.recorded_by,
                "source_count": evidence_count,
                "verified_anchor_count": verified_anchor_count,
                "blocked_missing_anchor_count": blocked_anchor_count,
                "schema": "hsk.atelier.source_evidence_matrix@1",
            }),
        )
        .await?;
        tx.commit().await?;

        self.get_source_evidence_matrix(&input.matrix_id).await
    }

    pub async fn get_source_evidence_matrix(
        &self,
        matrix_id: &str,
    ) -> AtelierResult<SourceEvidenceMatrix> {
        let matrix_id = validate_token("matrix_id", matrix_id)?;
        let source_rows = sqlx::query(
            r#"SELECT source_id, matrix_id, source_label, source_ref, product_area,
                      maturity_status, implementation_status, evidence_refs, proof_refs,
                      gap_reason, updated_at_utc
               FROM atelier_source_evidence_record
               WHERE matrix_id = $1
               ORDER BY source_id"#,
        )
        .bind(&matrix_id)
        .fetch_all(self.pool())
        .await?;
        let anchor_rows = sqlx::query(
            r#"SELECT anchor_id, matrix_id, source_id, anchor_label, expected_product_path,
                      verification_status, verified_product_paths, blocking_reason, updated_at_utc
               FROM atelier_anchor_verification_record
               WHERE matrix_id = $1
               ORDER BY anchor_id"#,
        )
        .bind(&matrix_id)
        .fetch_all(self.pool())
        .await?;

        if source_rows.is_empty() {
            return Err(AtelierError::NotFound(format!(
                "source evidence matrix_id={matrix_id}"
            )));
        }

        let mut sources = Vec::with_capacity(source_rows.len());
        for row in source_rows {
            sources.push(SourceEvidenceRecord {
                source_id: row.get("source_id"),
                matrix_id: row.get("matrix_id"),
                source_label: row.get("source_label"),
                source_ref: row.get("source_ref"),
                product_area: row.get("product_area"),
                maturity_status: SourceMaturityStatus::from_token(row.get("maturity_status"))?,
                implementation_status: row.get("implementation_status"),
                evidence_refs: jsonb_string_array(row.get("evidence_refs"))?,
                proof_refs: jsonb_string_array(row.get("proof_refs"))?,
                gap_reason: row.get("gap_reason"),
                updated_at_utc: row.get("updated_at_utc"),
            });
        }

        let mut anchors = Vec::with_capacity(anchor_rows.len());
        for row in anchor_rows {
            anchors.push(AnchorVerificationRecord {
                anchor_id: row.get("anchor_id"),
                matrix_id: row.get("matrix_id"),
                source_id: row.get("source_id"),
                anchor_label: row.get("anchor_label"),
                expected_product_path: row.get("expected_product_path"),
                verification_status: AnchorVerificationStatus::from_token(
                    row.get("verification_status"),
                )?,
                verified_product_paths: jsonb_string_array(row.get("verified_product_paths"))?,
                blocking_reason: row.get("blocking_reason"),
                updated_at_utc: row.get("updated_at_utc"),
            });
        }

        Ok(SourceEvidenceMatrix {
            matrix_id,
            sources,
            anchors,
        })
    }
}

fn validate_matrix(input: &NewSourceEvidenceMatrix) -> AtelierResult<()> {
    validate_token("matrix_id", &input.matrix_id)?;
    validate_token("recorded_by", &input.recorded_by)?;
    if input.sources.is_empty() {
        return Err(AtelierError::Validation(
            "source evidence matrix must include at least one source".into(),
        ));
    }
    if input.anchors.is_empty() {
        return Err(AtelierError::Validation(
            "source evidence matrix must include at least one anchor".into(),
        ));
    }
    let source_ids: std::collections::HashSet<&str> = input
        .sources
        .iter()
        .map(|source| source.source_id.as_str())
        .collect();
    if source_ids.len() != input.sources.len() {
        return Err(AtelierError::Validation(
            "source evidence matrix source_id values must be unique".into(),
        ));
    }

    for source in &input.sources {
        validate_token("source_id", &source.source_id)?;
        validate_token("source_label", &source.source_label)?;
        validate_ref("source_ref", &source.source_ref)?;
        validate_token("product_area", &source.product_area)?;
        validate_token("implementation_status", &source.implementation_status)?;
        validate_ref_list("evidence_refs", &source.evidence_refs)?;
        validate_ref_list("proof_refs", &source.proof_refs)?;
        if source.maturity_status == SourceMaturityStatus::Blocked
            && source
                .gap_reason
                .as_deref()
                .map(str::trim)
                .unwrap_or_default()
                .is_empty()
        {
            return Err(AtelierError::Validation(
                "BLOCKED source evidence rows must include gap_reason".into(),
            ));
        }
    }

    let anchor_ids: std::collections::HashSet<&str> = input
        .anchors
        .iter()
        .map(|anchor| anchor.anchor_id.as_str())
        .collect();
    if anchor_ids.len() != input.anchors.len() {
        return Err(AtelierError::Validation(
            "anchor verification anchor_id values must be unique".into(),
        ));
    }
    for anchor in &input.anchors {
        validate_token("anchor_id", &anchor.anchor_id)?;
        if !source_ids.contains(anchor.source_id.as_str()) {
            return Err(AtelierError::Validation(format!(
                "anchor {} references unknown source_id {}",
                anchor.anchor_id, anchor.source_id
            )));
        }
        validate_token("anchor_label", &anchor.anchor_label)?;
        validate_ref("expected_product_path", &anchor.expected_product_path)?;
        validate_ref_list("verified_product_paths", &anchor.verified_product_paths)?;
        match anchor.verification_status {
            AnchorVerificationStatus::Verified if anchor.verified_product_paths.is_empty() => {
                return Err(AtelierError::Validation(
                    "VERIFIED anchors must include verified_product_paths".into(),
                ));
            }
            AnchorVerificationStatus::BlockedMissingAnchor
                if anchor
                    .blocking_reason
                    .as_deref()
                    .map(str::trim)
                    .unwrap_or_default()
                    .is_empty() =>
            {
                return Err(AtelierError::Validation(
                    "BLOCKED_MISSING_ANCHOR anchors must include blocking_reason".into(),
                ));
            }
            _ => {}
        }
    }

    Ok(())
}

fn validate_token(field: &str, value: &str) -> AtelierResult<String> {
    if value.trim().is_empty() || value.trim() != value {
        return Err(AtelierError::Validation(format!(
            "{field} must not be empty or padded"
        )));
    }
    Ok(value.to_string())
}

fn validate_ref(field: &str, value: &str) -> AtelierResult<()> {
    reject_legacy_runtime_ref(field, value)?;
    if value.to_ascii_lowercase().contains("candidate") {
        return Err(AtelierError::Validation(format!(
            "{field} must cite a verified product/source ref, not a candidate name"
        )));
    }
    Ok(())
}

fn validate_ref_list(field: &str, values: &[String]) -> AtelierResult<()> {
    for value in values {
        validate_ref(field, value)?;
    }
    Ok(())
}

fn jsonb_string_array(value: serde_json::Value) -> AtelierResult<Vec<String>> {
    serde_json::from_value(value)
        .map_err(|err| AtelierError::Validation(format!("expected JSON string array: {err}")))
}
