//! MT-050: Validation Report Projection (kernel-side, MTE layer).
//!
//! Acceptance (MT-050.json): "expose validation summaries to DCC/projection
//! layer. Acceptance: operator/model can inspect validation without reading raw
//! files first."
//!
//! This module produces a compact, deterministic projection of a
//! `ValidationReport` that downstream surfaces (DCC, validator HUD, MTE
//! summary aggregator) can render directly. The DCC surface itself is owned
//! by Batch G; this file is the **kernel-side** producer only — Batch G
//! consumes via `MteValidationReportProjectionV1`.
//!
//! Hand-off note for Batch G: read this projection through
//! `MteValidationReportProjectionV1::from_report(&report, ...)` and treat
//! `blocking_descriptor_names` as the authoritative "why validation blocks
//! promotion" surface; do not re-derive it by inspecting outcome statuses.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

use crate::kernel::validation::report::ValidationReport;
use crate::kernel::validation::status::ValidationStatus;

/// Compact projection of a `ValidationReport`. Schema id
/// `hsk.kernel.mte.validation_report_projection@1`.
///
/// Field layout is intentionally narrow so the DCC surface stays cheap to
/// render and the bytes stay small enough for inclusion in MTE per-MT
/// summaries without inflating receipt payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MteValidationReportProjectionV1 {
    pub schema_version: &'static str,
    pub validation_run_id: Uuid,
    pub total_outcomes: usize,
    pub counts_by_tag: BTreeMap<String, usize>,
    pub blocking_descriptor_names: Vec<String>,
    pub advisory_descriptor_names: Vec<String>,
    pub unsupported_descriptor_names: Vec<String>,
    pub blocks_promotion_default: bool,
    pub report_artifact_ref: Option<String>,
}

impl MteValidationReportProjectionV1 {
    pub const SCHEMA_VERSION: &'static str =
        "hsk.kernel.mte.validation_report_projection@1";

    /// Reduce a `ValidationReport` into the projection. `report_artifact_ref`
    /// is the bundle-relative handle that the DCC surface can use to fetch
    /// the raw report if the operator drills down.
    pub fn from_report(
        report: &ValidationReport,
        report_artifact_ref: Option<String>,
    ) -> Self {
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        let mut blocking = Vec::new();
        let mut advisory = Vec::new();
        let mut unsupported = Vec::new();
        for outcome in &report.outcomes {
            *counts.entry(outcome.status.tag().to_string()).or_insert(0) += 1;
            if outcome.status.blocks_promotion() {
                blocking.push(outcome.descriptor_name.clone());
            }
            match &outcome.status {
                ValidationStatus::AdvisoryOnly { .. } => {
                    advisory.push(outcome.descriptor_name.clone())
                }
                ValidationStatus::Unsupported { .. } => {
                    unsupported.push(outcome.descriptor_name.clone())
                }
                _ => {}
            }
        }
        // Determinism: sort blocking outcomes alphabetically so the projection
        // hash is stable when descriptor evaluation order changes.
        blocking.sort();
        advisory.sort();
        unsupported.sort();

        Self {
            schema_version: Self::SCHEMA_VERSION,
            validation_run_id: report.run_id,
            total_outcomes: report.outcomes.len(),
            counts_by_tag: counts,
            blocking_descriptor_names: blocking,
            advisory_descriptor_names: advisory,
            unsupported_descriptor_names: unsupported,
            blocks_promotion_default: report.aggregate_blocks_promotion(),
            report_artifact_ref,
        }
    }

    /// True when there is at least one blocking descriptor under default
    /// policy. Mirrors `ValidationReport::aggregate_blocks_promotion` so the
    /// DCC surface does not have to load the full report.
    pub fn blocks_promotion(&self) -> bool {
        self.blocks_promotion_default
    }

    /// Convenience accessor for surfaces that want a single human line.
    pub fn one_line_summary(&self) -> String {
        let mut parts: Vec<String> = self
            .counts_by_tag
            .iter()
            .map(|(tag, n)| format!("{tag}:{n}"))
            .collect();
        parts.sort();
        format!(
            "total={} blocks={} ({})",
            self.total_outcomes,
            self.blocks_promotion_default,
            parts.join(",")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::validation::report::DescriptorOutcome;

    fn report_with_mix() -> ValidationReport {
        let mut r = ValidationReport::new(Uuid::new_v4());
        r.push(DescriptorOutcome::new("d_pass", ValidationStatus::pass()));
        r.push(DescriptorOutcome::new(
            "d_fail",
            ValidationStatus::fail("hash mismatch").unwrap(),
        ));
        r.push(DescriptorOutcome::new(
            "d_advisory",
            ValidationStatus::advisory("style note").unwrap(),
        ));
        r.push(DescriptorOutcome::new(
            "d_unsupported",
            ValidationStatus::unsupported("HardIsolation").unwrap(),
        ));
        r
    }

    // MT-050 acceptance: operator can inspect validation without reading raw
    // files first — the projection carries blocking descriptor names.
    #[test]
    fn projection_exposes_blocking_descriptor_names() {
        let report = report_with_mix();
        let proj = MteValidationReportProjectionV1::from_report(
            &report,
            Some("kb003://validation_report/abc".into()),
        );
        assert!(proj.blocks_promotion());
        assert_eq!(proj.blocking_descriptor_names, vec!["d_fail".to_string()]);
        assert_eq!(
            proj.advisory_descriptor_names,
            vec!["d_advisory".to_string()]
        );
        assert_eq!(
            proj.unsupported_descriptor_names,
            vec!["d_unsupported".to_string()]
        );
        assert_eq!(proj.total_outcomes, 4);
        assert_eq!(proj.counts_by_tag.get("PASS").copied(), Some(1));
        assert_eq!(proj.counts_by_tag.get("FAIL").copied(), Some(1));
        assert_eq!(
            proj.report_artifact_ref.as_deref(),
            Some("kb003://validation_report/abc")
        );
    }

    #[test]
    fn empty_report_does_not_block() {
        let r = ValidationReport::new(Uuid::new_v4());
        let proj = MteValidationReportProjectionV1::from_report(&r, None);
        assert!(!proj.blocks_promotion());
        assert!(proj.blocking_descriptor_names.is_empty());
        assert_eq!(proj.total_outcomes, 0);
    }

    #[test]
    fn projection_is_deterministic_in_blocking_order() {
        // Evaluate descriptors in two different orders; projection's blocking
        // list must be identical.
        let mut a = ValidationReport::new(Uuid::new_v4());
        a.push(DescriptorOutcome::new(
            "zzz_fail",
            ValidationStatus::fail("x").unwrap(),
        ));
        a.push(DescriptorOutcome::new(
            "aaa_fail",
            ValidationStatus::fail("y").unwrap(),
        ));
        let mut b = ValidationReport::new(a.run_id);
        b.push(DescriptorOutcome::new(
            "aaa_fail",
            ValidationStatus::fail("y").unwrap(),
        ));
        b.push(DescriptorOutcome::new(
            "zzz_fail",
            ValidationStatus::fail("x").unwrap(),
        ));
        let pa = MteValidationReportProjectionV1::from_report(&a, None);
        let pb = MteValidationReportProjectionV1::from_report(&b, None);
        assert_eq!(pa.blocking_descriptor_names, pb.blocking_descriptor_names);
        assert_eq!(pa.blocking_descriptor_names, vec!["aaa_fail", "zzz_fail"]);
    }

    #[test]
    fn one_line_summary_is_self_describing() {
        let r = report_with_mix();
        let s = MteValidationReportProjectionV1::from_report(&r, None)
            .one_line_summary();
        assert!(s.contains("total=4"));
        assert!(s.contains("blocks=true"));
        assert!(s.contains("FAIL:1"));
        assert!(s.contains("PASS:1"));
    }

    #[test]
    fn serde_round_trip_keeps_projection_stable() {
        let r = report_with_mix();
        let proj = MteValidationReportProjectionV1::from_report(
            &r,
            Some("kb003://validation_report/h".into()),
        );
        let j = serde_json::to_string(&proj).unwrap();
        let back: MteValidationReportProjectionV1 = serde_json::from_str(&j).unwrap();
        assert_eq!(back, proj);
    }
}
