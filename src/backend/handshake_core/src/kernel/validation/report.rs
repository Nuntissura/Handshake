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

use super::descriptor::{DescriptorKind, ValidationDescriptor};
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
    /// Classification of the descriptor that produced this outcome. Defaults
    /// to `Gating` for the `new(name, status)` constructor (conservative —
    /// unknown descriptors are treated as gating so an `Unsupported` outcome
    /// blocks promotion under `aggregate_blocks_promotion_for_kind`). Use
    /// `from_descriptor` to capture the descriptor's real kind.
    #[serde(default = "default_descriptor_kind")]
    pub descriptor_kind: DescriptorKind,
    pub status: ValidationStatus,
    pub evidence: Vec<EvidenceItem>,
    pub artifact_refs: Vec<String>,
}

fn default_descriptor_kind() -> DescriptorKind {
    DescriptorKind::Gating
}

impl DescriptorOutcome {
    pub fn new(descriptor_name: impl Into<String>, status: ValidationStatus) -> Self {
        Self {
            descriptor_name: descriptor_name.into(),
            descriptor_kind: DescriptorKind::Gating,
            status,
            evidence: Vec::new(),
            artifact_refs: Vec::new(),
        }
    }

    /// Construct an outcome capturing the descriptor's real kind. Preferred
    /// over `new` for adapter dispatch so `Unsupported` on a gating descriptor
    /// can be aggregated as blocking (see
    /// `ValidationReport::aggregate_blocks_promotion_for_kind`).
    pub fn from_descriptor(
        descriptor: &dyn ValidationDescriptor,
        status: ValidationStatus,
    ) -> Self {
        Self {
            descriptor_name: descriptor.name().to_string(),
            descriptor_kind: descriptor.kind(),
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
    pub schema_version: String,
    pub event_type: String,
    pub artifact_class: Kb003ArtifactClass,
    pub run_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub outcomes: Vec<DescriptorOutcome>,
    /// MT-049: present when the originating `ValidationRun` is a replay.
    /// Mirrors `ValidationRun.original_run_id` so the event-side projection
    /// carries the replay linkage even when the report is exchanged
    /// out-of-band from the run record.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_run_id: Option<Uuid>,
}

impl ValidationReport {
    pub fn new(run_id: Uuid) -> Self {
        Self {
            schema_version: SCHEMA_KERNEL_VALIDATION_RUN_V1.to_string(),
            event_type: EVENT_KB003_VALIDATION_RUN_COMPLETED.to_string(),
            artifact_class: Kb003ArtifactClass::ValidationReport,
            run_id,
            created_at: Utc::now(),
            outcomes: Vec::new(),
            original_run_id: None,
        }
    }

    /// MT-049: construct a report from a `ValidationRun`, propagating the
    /// replay linkage (`original_run_id`) when the run is a replay so the
    /// event-side projection captures the source-run reference.
    pub fn for_run(run: &crate::kernel::validation::run::ValidationRun) -> Self {
        Self {
            schema_version: SCHEMA_KERNEL_VALIDATION_RUN_V1.to_string(),
            event_type: EVENT_KB003_VALIDATION_RUN_COMPLETED.to_string(),
            artifact_class: Kb003ArtifactClass::ValidationReport,
            run_id: run.run_id,
            created_at: Utc::now(),
            outcomes: Vec::new(),
            original_run_id: run.original_run_id,
        }
    }

    /// MT-049: explicit setter for callers constructing a report via
    /// `ValidationReport::new(run_id)` who later resolve the source run.
    pub fn with_original_run_id(mut self, original: Uuid) -> Self {
        self.original_run_id = Some(original);
        self
    }

    pub fn push(&mut self, outcome: DescriptorOutcome) {
        self.outcomes.push(outcome);
    }

    /// Default-policy promotion-gate projection: blocks if any descriptor
    /// outcome is FAIL/BLOCKED/ERROR. Advisory/Unsupported/Skipped do not
    /// block by default.
    pub fn aggregate_blocks_promotion(&self) -> bool {
        self.outcomes.iter().any(|o| o.status.blocks_promotion())
    }

    /// Kind-aware promotion-gate projection (KB003 remediation H3):
    /// blocks when ANY outcome satisfies:
    ///   - `status.blocks_promotion()` (FAIL / BLOCKED / ERROR), OR
    ///   - `descriptor_kind == Gating && status == Unsupported`
    ///
    /// This closes the silent-skip mode forbidden by MT-044: a gating
    /// descriptor that returns `Unsupported` (declared adapter not available)
    /// must NOT silently pass.
    pub fn aggregate_blocks_promotion_for_kind(&self) -> bool {
        self.aggregate_blocks_promotion_for_kind_with_flag(false)
    }

    /// Strict-mode variant of `aggregate_blocks_promotion_for_kind`. When
    /// `treat_unsupported_as_blocking_for_advisory` is true, advisory
    /// descriptors returning `Unsupported` also block.
    pub fn aggregate_blocks_promotion_for_kind_with_flag(
        &self,
        treat_unsupported_as_blocking_for_advisory: bool,
    ) -> bool {
        self.outcomes.iter().any(|o| {
            o.status.blocks_promotion()
                || (matches!(o.status, ValidationStatus::Unsupported { .. })
                    && (o.descriptor_kind == DescriptorKind::Gating
                        || treat_unsupported_as_blocking_for_advisory))
        })
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
        let r = ValidationReport::new(Uuid::now_v7());
        assert_eq!(r.schema_version, SCHEMA_KERNEL_VALIDATION_RUN_V1);
        assert_eq!(r.event_type, EVENT_KB003_VALIDATION_RUN_COMPLETED);
        assert_eq!(r.artifact_class, Kb003ArtifactClass::ValidationReport);
    }

    #[test]
    fn aggregate_blocks_on_any_blocking_descriptor() {
        let mut r = ValidationReport::new(Uuid::now_v7());
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
    fn replay_report_propagates_original_run_id() {
        // MT-049 acceptance: when the report is built from a replay run, the
        // event-side projection MUST carry the original_run_id linkage.
        use crate::kernel::validation::run::ValidationRun;
        let first = ValidationRun::new("cand-1", "sess-1", "task-1").unwrap();
        let replay = ValidationRun::replay_of(&first, "sess-2", "task-2").unwrap();
        let report = ValidationReport::for_run(&replay);
        assert_eq!(report.run_id, replay.run_id);
        assert_eq!(report.original_run_id, Some(first.run_id));

        // Fresh run produces a report with no original linkage.
        let fresh_report = ValidationReport::for_run(&first);
        assert_eq!(fresh_report.original_run_id, None);
    }

    #[test]
    fn validation_report_serde_round_trips_original_run_id() {
        use crate::kernel::validation::run::ValidationRun;
        let first = ValidationRun::new("c", "s", "t").unwrap();
        let replay = ValidationRun::replay_of(&first, "s2", "t2").unwrap();
        let report = ValidationReport::for_run(&replay);
        let json = serde_json::to_string(&report).expect("serialize");
        let back: ValidationReport = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.original_run_id, Some(first.run_id));
        assert_eq!(back.run_id, replay.run_id);
    }

    // KB003 remediation H3: kind-aware aggregation.

    fn outcome_with_kind(
        name: &str,
        kind: DescriptorKind,
        status: ValidationStatus,
    ) -> DescriptorOutcome {
        let mut o = DescriptorOutcome::new(name, status);
        o.descriptor_kind = kind;
        o
    }

    #[test]
    fn unsupported_gating_descriptor_blocks_promotion() {
        let mut r = ValidationReport::new(Uuid::now_v7());
        r.push(outcome_with_kind(
            "hard_isolation",
            DescriptorKind::Gating,
            ValidationStatus::unsupported("HardIsolation").unwrap(),
        ));
        // Default aggregator (status-only) MUST NOT block — that's the latent
        // silent-skip defect this remediation closes.
        assert!(!r.aggregate_blocks_promotion());
        // Kind-aware aggregator MUST block: gating + Unsupported is a hard
        // refusal under MT-044.
        assert!(r.aggregate_blocks_promotion_for_kind());
    }

    #[test]
    fn unsupported_advisory_descriptor_does_not_block_by_default() {
        let mut r = ValidationReport::new(Uuid::now_v7());
        r.push(outcome_with_kind(
            "lint_advisory",
            DescriptorKind::Advisory,
            ValidationStatus::unsupported("LintAdapter").unwrap(),
        ));
        assert!(!r.aggregate_blocks_promotion_for_kind());
    }

    #[test]
    fn unsupported_advisory_descriptor_blocks_when_strict_flag_set() {
        let mut r = ValidationReport::new(Uuid::now_v7());
        r.push(outcome_with_kind(
            "lint_advisory",
            DescriptorKind::Advisory,
            ValidationStatus::unsupported("LintAdapter").unwrap(),
        ));
        assert!(r.aggregate_blocks_promotion_for_kind_with_flag(true));
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
