//! `ValidationReport` — the per-descriptor outcome bundle stored from a run.
//!
//! Each evaluated descriptor produces one `DescriptorOutcome` carrying its
//! typed `ValidationStatus`, the evidence the descriptor reported, and any
//! artifact_refs that back the evidence (logs, diffs, screenshots).
//!
//! Hand-off note for Batch E (promotion + artifact bundle):
//! - `ValidationReport.aggregate_blocks_promotion()` is the single source of
//!   truth for whether any descriptor outcome blocks promotion under default
//!   policy.
//! - `artifact_class` for the canonical export is
//!   `Kb003ArtifactClass::ValidationReport` (hash policy
//!   `CanonicalJsonSha256`, exportable by default).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::kernel::kb003_artifact_classes::Kb003ArtifactClass;
use crate::kernel::kb003_schemas::{
    EVENT_KB003_VALIDATION_RUN_COMPLETED, SCHEMA_KERNEL_VALIDATION_RUN_V1,
};

use super::status::ValidationStatus;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceItem {
    pub key: String,
    pub value: String,
}

impl EvidenceItem {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DescriptorOutcome {
    pub descriptor_name: String,
    pub status: ValidationStatus,
    pub evidence: Vec<EvidenceItem>,
    pub artifact_refs: Vec<String>,
}

impl DescriptorOutcome {
    pub fn new(descriptor_name: impl Into<String>, status: ValidationStatus) -> Self {
        Self {
            descriptor_name: descriptor_name.into(),
            status,
            evidence: Vec::new(),
            artifact_refs: Vec::new(),
        }
    }

    pub fn with_evidence(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.evidence.push(EvidenceItem::new(key, value));
        self
    }

    pub fn with_artifact(mut self, artifact_ref: impl Into<String>) -> Self {
        self.artifact_refs.push(artifact_ref.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationReport {
    pub schema_version: &'static str,
    pub event_type: &'static str,
    pub artifact_class: Kb003ArtifactClass,
    pub run_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub outcomes: Vec<DescriptorOutcome>,
}

impl ValidationReport {
    pub fn new(run_id: Uuid) -> Self {
        Self {
            schema_version: SCHEMA_KERNEL_VALIDATION_RUN_V1,
            event_type: EVENT_KB003_VALIDATION_RUN_COMPLETED,
            artifact_class: Kb003ArtifactClass::ValidationReport,
            run_id,
            created_at: Utc::now(),
            outcomes: Vec::new(),
        }
    }

    pub fn push(&mut self, outcome: DescriptorOutcome) {
        self.outcomes.push(outcome);
    }

    /// Default-policy promotion-gate projection: blocks if any descriptor
    /// outcome is FAIL/BLOCKED/ERROR. Advisory/Unsupported/Skipped do not
    /// block by default.
    pub fn aggregate_blocks_promotion(&self) -> bool {
        self.outcomes
            .iter()
            .any(|o| o.status.blocks_promotion())
    }

    /// Count by status tag for fast surface rendering.
    pub fn counts_by_tag(&self) -> std::collections::BTreeMap<&'static str, usize> {
        let mut acc = std::collections::BTreeMap::new();
        for o in &self.outcomes {
            *acc.entry(o.status.tag()).or_insert(0) += 1;
        }
        acc
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_wires_schema_event_and_artifact_class() {
        let r = ValidationReport::new(Uuid::new_v4());
        assert_eq!(r.schema_version, SCHEMA_KERNEL_VALIDATION_RUN_V1);
        assert_eq!(r.event_type, EVENT_KB003_VALIDATION_RUN_COMPLETED);
        assert_eq!(r.artifact_class, Kb003ArtifactClass::ValidationReport);
    }

    #[test]
    fn aggregate_blocks_on_any_blocking_descriptor() {
        let mut r = ValidationReport::new(Uuid::new_v4());
        r.push(DescriptorOutcome::new("d1", ValidationStatus::pass()));
        r.push(DescriptorOutcome::new(
            "d2",
            ValidationStatus::advisory("info").unwrap(),
        ));
        assert!(!r.aggregate_blocks_promotion());
        r.push(DescriptorOutcome::new(
            "d3",
            ValidationStatus::fail("oops").unwrap(),
        ));
        assert!(r.aggregate_blocks_promotion());
    }

    #[test]
    fn outcomes_carry_evidence_and_artifact_refs() {
        let outcome = DescriptorOutcome::new(
            "artifact_hashes_valid",
            ValidationStatus::fail("hash mismatch").unwrap(),
        )
        .with_evidence("expected", "sha256:aaa")
        .with_evidence("actual", "sha256:bbb")
        .with_artifact("bundle://run/abc/diff.patch");
        assert_eq!(outcome.evidence.len(), 2);
        assert_eq!(outcome.artifact_refs.len(), 1);
    }
}
