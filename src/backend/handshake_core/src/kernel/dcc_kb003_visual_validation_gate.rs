//! MT-073 Visual Validation Gate Descriptor.
//!
//! Acceptance (MT-073.json): "visual evidence can block or advise according
//! to descriptor posture."
//!
//! Binds visual evidence (browser screenshots, DOM dumps, axe-core results)
//! to a ValidationStatus posture: BLOCKING or ADVISORY_ONLY.
//!
//! - In BLOCKING posture, a visual check failure must surface as
//!   `ValidationStatus::Fail` and prevent promotion.
//! - In ADVISORY_ONLY posture, a visual check failure must surface as
//!   `ValidationStatus::AdvisoryOnly` and *not* prevent promotion.
//!
//! Frontend renders via existing dcc-* IPC surface; no app/** edits.
//! Reuses `VisualEvidenceItem` from MT-038 (`validation/visual_evidence.rs`)
//! and `ValidationStatus` from MT-031 (`validation/status.rs`).

use serde::{Deserialize, Serialize};

use crate::kernel::validation::status::ValidationStatus;
use crate::kernel::validation::visual_evidence::VisualEvidenceItem;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VisualGatePosture {
    Blocking,
    AdvisoryOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualValidationGateDescriptorV1 {
    pub schema_version: &'static str,
    pub descriptor_id: String,
    pub posture: VisualGatePosture,
    pub evidence: Vec<VisualEvidenceItem>,
    pub pass_criterion_short: String,
}

impl VisualValidationGateDescriptorV1 {
    pub const SCHEMA_VERSION: &'static str = "hsk.kernel.kb003_visual_validation_gate@1";

    pub fn new(
        descriptor_id: impl Into<String>,
        posture: VisualGatePosture,
        evidence: Vec<VisualEvidenceItem>,
        pass_criterion_short: impl Into<String>,
    ) -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION,
            descriptor_id: descriptor_id.into(),
            posture,
            evidence,
            pass_criterion_short: pass_criterion_short.into(),
        }
    }

    /// Evaluate the descriptor against a boolean `visual_passed` from the
    /// underlying check (e.g. axe-core returns 0 violations).
    /// Maps to the typed ValidationStatus per posture.
    pub fn evaluate(&self, visual_passed: bool) -> ValidationStatus {
        if visual_passed {
            return ValidationStatus::Pass;
        }
        let reason = format!(
            "visual descriptor {} failed: {}",
            self.descriptor_id, self.pass_criterion_short
        );
        match self.posture {
            VisualGatePosture::Blocking => ValidationStatus::fail(reason)
                .expect("non-empty reason"),
            VisualGatePosture::AdvisoryOnly => ValidationStatus::advisory(reason)
                .expect("non-empty note"),
        }
    }

    /// Returns whether the descriptor will block promotion when the visual
    /// check fails.
    pub fn blocks_promotion_on_failure(&self) -> bool {
        matches!(self.posture, VisualGatePosture::Blocking)
    }
}

/// DCC descriptor row for the operator visual lane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccKb003VisualValidationGateRowV1 {
    pub descriptor_id: String,
    pub posture: VisualGatePosture,
    pub evidence_count: usize,
    pub pass_criterion_short: String,
    pub blocks_promotion_on_failure: bool,
}

impl DccKb003VisualValidationGateRowV1 {
    pub fn from_descriptor(d: &VisualValidationGateDescriptorV1) -> Self {
        Self {
            descriptor_id: d.descriptor_id.clone(),
            posture: d.posture,
            evidence_count: d.evidence.len(),
            pass_criterion_short: d.pass_criterion_short.clone(),
            blocks_promotion_on_failure: d.blocks_promotion_on_failure(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pass_maps_to_validation_pass_regardless_of_posture() {
        let blocking = VisualValidationGateDescriptorV1::new(
            "VVG-1",
            VisualGatePosture::Blocking,
            vec![VisualEvidenceItem::screenshot("ART-1")],
            "0 axe violations",
        );
        assert_eq!(blocking.evaluate(true), ValidationStatus::Pass);
        let advisory = VisualValidationGateDescriptorV1::new(
            "VVG-2",
            VisualGatePosture::AdvisoryOnly,
            vec![VisualEvidenceItem::screenshot("ART-2")],
            "0 axe violations",
        );
        assert_eq!(advisory.evaluate(true), ValidationStatus::Pass);
    }

    #[test]
    fn blocking_descriptor_fails_promotes_to_validation_fail() {
        let d = VisualValidationGateDescriptorV1::new(
            "VVG-1",
            VisualGatePosture::Blocking,
            vec![VisualEvidenceItem::screenshot("ART-1")],
            "0 axe violations",
        );
        let status = d.evaluate(false);
        assert!(matches!(status, ValidationStatus::Fail { .. }));
        assert!(d.blocks_promotion_on_failure());
    }

    #[test]
    fn advisory_only_descriptor_fails_to_validation_advisory_only() {
        let d = VisualValidationGateDescriptorV1::new(
            "VVG-2",
            VisualGatePosture::AdvisoryOnly,
            vec![VisualEvidenceItem::screenshot("ART-2")],
            "0 axe violations",
        );
        let status = d.evaluate(false);
        assert!(matches!(status, ValidationStatus::AdvisoryOnly { .. }));
        assert!(!d.blocks_promotion_on_failure());
    }

    #[test]
    fn dcc_row_projects_descriptor_posture() {
        let d = VisualValidationGateDescriptorV1::new(
            "VVG-3",
            VisualGatePosture::Blocking,
            vec![VisualEvidenceItem::screenshot("ART-3"), VisualEvidenceItem::dom_dump("ART-4")],
            "DOM sanity",
        );
        let row = DccKb003VisualValidationGateRowV1::from_descriptor(&d);
        assert_eq!(row.posture, VisualGatePosture::Blocking);
        assert_eq!(row.evidence_count, 2);
        assert!(row.blocks_promotion_on_failure);
    }
}
