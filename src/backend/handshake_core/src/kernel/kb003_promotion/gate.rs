//! MT-045/046/047/048: `PromotionGate` — the single `evaluate(...)` entry
//! point that produces a typed `PromotionDecisionV1` and persists the
//! corresponding `PromotionReceiptV1` through `Kb003Storage`.
//!
//! Acceptance composite:
//! - MT-045 "Deterministic Check Batch: blocking check failure prevents
//!   promotion" — `ValidationReport::aggregate_blocks_promotion()` short-
//!   circuits to `PromotionRejectionReason::ValidationFailure` before any
//!   approval or artifact check runs.
//! - MT-046 "Validation Evidence Bundle: validation report can be inspected
//!   offline" — the gate carries `report_artifact_ref` into the rejection
//!   reason and into the receipt's `artifact_refs`, so the bundle is the
//!   single offline-inspectable source for failed promotions.
//! - MT-047 "Finding Normalization: raw logs are not the only finding source"
//!   — the gate records each blocking descriptor name verbatim into the
//!   rejection reason, normalised from `ValidationReport::counts_by_tag`.
//! - MT-048 "Advisory vs Blocking Rules: advisory failure is visible but does
//!   not block unless configured" — the gate uses
//!   `ValidationReport::aggregate_blocks_promotion()` (which already
//!   classifies advisory as non-blocking by default) but also exposes
//!   `PromotionGateInputs::treat_advisory_as_blocking` so an operator can opt
//!   into strict mode without changing the descriptor classification.
//!
//! Idempotency mechanism (MT-014 / packet acceptance):
//! - The caller supplies `idempotency_key`. The gate computes the receipt
//!   payload hash via `PromotionReceiptV1::compute_payload_hash`. The storage
//!   layer keys on `(idempotency_key, payload_hash)`. Retrying with the same
//!   payload returns the original receipt id; retrying with a different
//!   payload surfaces as `PromotionRejectionReason::DuplicateIdempotencyKey`
//!   (mapped from `Kb003StorageError::IdempotencyConflict`).

use crate::kernel::kb003_promotion::artifact_bundle::Kb003ArtifactBundleV1;
use crate::kernel::kb003_promotion::decision::{
    PromotionDecisionV1, PromotionOutcome, PromotionRejectionReason,
};
use crate::kernel::kb003_promotion::receipt::PromotionReceiptV1;
use crate::kernel::sandbox::denial::SandboxDenialRecordV1;
// H-B1 fix: `SandboxCapability` is only referenced inside #[cfg(test)] mod tests
// (which has its own `use`). Top-level import would warn under -D warnings.
use crate::kernel::sandbox::run::{SandboxRunStatus, SandboxRunV1};
use crate::kernel::validation::report::ValidationReport;
use crate::kernel::validation::status::ValidationStatus;
use crate::storage::kb003_storage::{
    Kb003Storage, Kb003StorageError, PromotionDecisionRowV1, PromotionReceiptRowV1,
};

/// Operator approval evidence the gate requires before issuing an Accepted
/// decision. Constructed from an operator-review receipt; the gate refuses
/// fixture-looking evidence so the proof surface cannot be spoofed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorApprovalEvidence {
    pub operator_id: String,
    pub review_receipt_id: String,
    pub approval_source: String,
    pub reason: String,
}

impl OperatorApprovalEvidence {
    /// Build operator evidence. Returns `MissingApproval` rejection-eligible
    /// values via `validate_field_present` so the gate can route them through
    /// the typed rejection-reason taxonomy instead of panicking.
    pub fn new(
        operator_id: impl Into<String>,
        review_receipt_id: impl Into<String>,
        approval_source: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            operator_id: operator_id.into(),
            review_receipt_id: review_receipt_id.into(),
            approval_source: approval_source.into(),
            reason: reason.into(),
        }
    }

    /// Returns the first missing field name, if any.
    pub fn missing_field(&self) -> Option<&'static str> {
        if self.operator_id.trim().is_empty() {
            return Some("operator_id");
        }
        if self.review_receipt_id.trim().is_empty() {
            return Some("review_receipt_id");
        }
        if self.approval_source.trim().is_empty() {
            return Some("approval_source");
        }
        if self.reason.trim().is_empty() {
            return Some("reason");
        }
        None
    }

    /// Approval source must be `operator_review_receipt` and the
    /// review_receipt_id / reason must not look fixture-like. This mirrors
    /// the legacy `kernel::promotion::PromotionGate::decide` check.
    pub fn looks_fixture(&self) -> bool {
        let needle = |s: &str| {
            let lower = s.to_ascii_lowercase();
            lower.contains("fixture") || lower.contains("kernel-proof")
        };
        needle(&self.operator_id)
            || needle(&self.review_receipt_id)
            || needle(&self.approval_source)
            || needle(&self.reason)
    }
}

/// All inputs the promotion gate inspects in one call. Bundled into a struct
/// so the signature stays stable when new evidence fields are added.
pub struct PromotionGateInputs<'a> {
    pub sandbox_run: &'a SandboxRunV1,
    pub validation_report: &'a ValidationReport,
    pub validation_run_id: String,
    pub artifact_bundle: &'a Kb003ArtifactBundleV1,
    pub approval: OperatorApprovalEvidence,
    pub idempotency_key: String,
    pub required_artifact_refs: Vec<String>,
    pub latest_known_run_id: Option<String>,
    pub denial: Option<&'a SandboxDenialRecordV1>,
    pub policy_version_id: String,
    pub report_artifact_ref: Option<String>,
    pub treat_advisory_as_blocking: bool,
}

/// Persisted output of `PromotionGate::evaluate`.
#[derive(Debug, Clone)]
// M-B1 fix: stored_receipt_id is Option<String>. `None` means the rejection
// receipt was not persisted (e.g. PostgresFailure where storage refused
// both the decision row AND the receipt row). Callers MUST treat `None` as
// a signal that the rejection only exists in the returned value and they
// own logging it elsewhere (out-of-band event ledger, monitoring, etc.).
pub struct PromotionGateOutput {
    pub decision: PromotionDecisionV1,
    pub receipt: PromotionReceiptV1,
    pub stored_receipt_id: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum PromotionGateError {
    #[error("decision recorded but storage rejected the row: {0}")]
    DecisionStorage(Kb003StorageError),
}

pub struct PromotionGate;

impl PromotionGate {
    /// Evaluate inputs and persist the resulting receipt. The returned
    /// `PromotionGateOutput.decision.outcome` tells the caller whether the
    /// candidate was Accepted or Rejected (with a typed reason). The receipt
    /// is always issued — even for rejections — so the operator surface can
    /// show what was decided and why.
    pub fn evaluate<S: Kb003Storage>(
        inputs: PromotionGateInputs<'_>,
        storage: &mut S,
    ) -> Result<PromotionGateOutput, PromotionGateError> {
        let decision = Self::decide(&inputs);
        let receipt = PromotionReceiptV1::new(
            decision.clone(),
            inputs.idempotency_key.clone(),
            Some(inputs.artifact_bundle.bundle_id),
            Some(inputs.artifact_bundle.bundle_sha256.clone()),
            Self::receipt_artifact_refs(&inputs),
        );

        // Persist the decision row first; the receipt insert is what enforces
        // idempotency.
        let decision_row = PromotionDecisionRowV1 {
            decision_id: decision.decision_id.clone(),
            validation_run_id: inputs.validation_run_id.clone(),
            decision: decision.outcome.tag().to_string(),
            rationale_short: match &decision.outcome {
                PromotionOutcome::Accepted => "accepted".to_string(),
                PromotionOutcome::Rejected { reason } => reason.rationale_short(),
            },
            decided_at_utc: decision.decided_at_utc.to_rfc3339(),
        };
        if let Err(e) = storage.insert_promotion_decision(&decision_row) {
            // Storage failure folds into a typed rejection so callers still get
            // a receipt explaining what happened.
            let fallback = PromotionDecisionV1::rejected(
                decision.sandbox_run_id.clone(),
                inputs.validation_run_id.clone(),
                PromotionRejectionReason::PostgresFailure {
                    storage_error: e.to_string(),
                },
            );
            let fallback_receipt = PromotionReceiptV1::new(
                fallback.clone(),
                inputs.idempotency_key.clone(),
                Some(inputs.artifact_bundle.bundle_id),
                Some(inputs.artifact_bundle.bundle_sha256.clone()),
                Self::receipt_artifact_refs(&inputs),
            );
            // H-B2 fix: try to persist the rejection receipt anyway. The
            // decision-row insert failed, but the receipt row table may still
            // be writable (e.g. partial Postgres outage scoped to one table).
            // If the receipt insert also fails, we surface `None` for
            // `stored_receipt_id` so the caller knows the rejection only exists
            // in-memory and owns out-of-band logging.
            let fallback_row = PromotionReceiptRowV1 {
                receipt_id: fallback_receipt.receipt_id.clone(),
                decision_id: fallback.decision_id.clone(),
                idempotency_key: fallback_receipt.idempotency_key.clone(),
                payload_hash: fallback_receipt.payload_hash.clone(),
                artifact_ref: inputs.report_artifact_ref.clone(),
                issued_at_utc: fallback_receipt.issued_at_utc.to_rfc3339(),
            };
            let stored_receipt_id = storage
                .insert_promotion_receipt(&fallback_row)
                .ok();
            return Ok(PromotionGateOutput {
                decision: fallback,
                receipt: fallback_receipt,
                stored_receipt_id,
            });
        }

        let row = PromotionReceiptRowV1 {
            receipt_id: receipt.receipt_id.clone(),
            decision_id: decision.decision_id.clone(),
            idempotency_key: receipt.idempotency_key.clone(),
            payload_hash: receipt.payload_hash.clone(),
            artifact_ref: inputs.report_artifact_ref.clone(),
            issued_at_utc: receipt.issued_at_utc.to_rfc3339(),
        };

        match storage.insert_promotion_receipt(&row) {
            Ok(stored_receipt_id) => Ok(PromotionGateOutput {
                decision,
                receipt,
                stored_receipt_id: Some(stored_receipt_id),
            }),
            Err(Kb003StorageError::IdempotencyConflict {
                key,
                existing_hash,
                new_hash,
            }) => {
                let dup_reason = PromotionRejectionReason::DuplicateIdempotencyKey {
                    idempotency_key: key,
                    existing_payload_hash: existing_hash,
                    new_payload_hash: new_hash,
                };
                let dup_decision = PromotionDecisionV1::rejected(
                    decision.sandbox_run_id.clone(),
                    inputs.validation_run_id.clone(),
                    dup_reason,
                );
                let dup_receipt = PromotionReceiptV1::new(
                    dup_decision.clone(),
                    inputs.idempotency_key.clone(),
                    Some(inputs.artifact_bundle.bundle_id),
                    Some(inputs.artifact_bundle.bundle_sha256.clone()),
                    Self::receipt_artifact_refs(&inputs),
                );
                Ok(PromotionGateOutput {
                    decision: dup_decision,
                    receipt: dup_receipt,
                    stored_receipt_id: None,
                })
            }
            Err(other) => Err(PromotionGateError::DecisionStorage(other)),
        }
    }

    /// Pure decision logic separated from storage so unit tests can exercise
    /// the rejection taxonomy without a backend.
    pub fn decide(inputs: &PromotionGateInputs<'_>) -> PromotionDecisionV1 {
        // 1. Stale candidate: caller knows a newer run id.
        if let Some(latest) = &inputs.latest_known_run_id {
            if latest != &inputs.sandbox_run.run_id.0 {
                return PromotionDecisionV1::rejected(
                    inputs.sandbox_run.run_id.0.clone(),
                    inputs.validation_run_id.clone(),
                    PromotionRejectionReason::StaleCandidate {
                        candidate_run_id: inputs.sandbox_run.run_id.0.clone(),
                        latest_run_id: latest.clone(),
                    },
                );
            }
        }

        // 2. Policy denial: sandbox produced a denial record.
        if let Some(denial) = inputs.denial {
            return PromotionDecisionV1::rejected(
                inputs.sandbox_run.run_id.0.clone(),
                inputs.validation_run_id.clone(),
                PromotionRejectionReason::PolicyDenial {
                    denial_id: denial.denial_id.clone(),
                    policy_version_id: inputs.policy_version_id.clone(),
                    capability: denial.capability.map(|c| c.as_str().to_string()),
                },
            );
        }

        // 3. Candidate must be in Completed state. Rejected sandbox runs go to
        // PolicyDenial above when a denial exists; otherwise treat as stale.
        if !matches!(inputs.sandbox_run.status, SandboxRunStatus::Completed) {
            return PromotionDecisionV1::rejected(
                inputs.sandbox_run.run_id.0.clone(),
                inputs.validation_run_id.clone(),
                PromotionRejectionReason::StaleCandidate {
                    candidate_run_id: inputs.sandbox_run.run_id.0.clone(),
                    latest_run_id: format!(
                        "{} (status {})",
                        inputs.sandbox_run.run_id.0,
                        inputs.sandbox_run.status.as_str()
                    ),
                },
            );
        }

        // 4. Validation gate: aggregate first, then promote advisory if strict
        // mode opted in.
        let blocking = inputs.validation_report.aggregate_blocks_promotion()
            || (inputs.treat_advisory_as_blocking
                && inputs
                    .validation_report
                    .outcomes
                    .iter()
                    .any(|o| matches!(o.status, ValidationStatus::AdvisoryOnly { .. })));
        if blocking {
            let blocking_outcomes: Vec<String> = inputs
                .validation_report
                .outcomes
                .iter()
                .filter(|o| {
                    o.status.blocks_promotion()
                        || (inputs.treat_advisory_as_blocking
                            && matches!(o.status, ValidationStatus::AdvisoryOnly { .. }))
                })
                .map(|o| o.descriptor_name.clone())
                .collect();
            return PromotionDecisionV1::rejected(
                inputs.sandbox_run.run_id.0.clone(),
                inputs.validation_run_id.clone(),
                PromotionRejectionReason::ValidationFailure {
                    validation_run_id: inputs.validation_run_id.clone(),
                    blocking_outcomes,
                    report_artifact_ref: inputs.report_artifact_ref.clone(),
                },
            );
        }

        // 5. Approval evidence present + well-formed + not fixture-like.
        if let Some(missing) = inputs.approval.missing_field() {
            return PromotionDecisionV1::rejected(
                inputs.sandbox_run.run_id.0.clone(),
                inputs.validation_run_id.clone(),
                PromotionRejectionReason::MissingApproval {
                    missing_field: missing.to_string(),
                },
            );
        }
        if inputs.approval.looks_fixture()
            || inputs.approval.approval_source != "operator_review_receipt"
        {
            return PromotionDecisionV1::rejected(
                inputs.sandbox_run.run_id.0.clone(),
                inputs.validation_run_id.clone(),
                PromotionRejectionReason::MissingApproval {
                    missing_field: "approval_source".to_string(),
                },
            );
        }

        // 6. Required artifacts present in the bundle.
        for required in &inputs.required_artifact_refs {
            let present = inputs
                .artifact_bundle
                .handles
                .iter()
                .any(|h| &h.handle == required);
            if !present {
                return PromotionDecisionV1::rejected(
                    inputs.sandbox_run.run_id.0.clone(),
                    inputs.validation_run_id.clone(),
                    PromotionRejectionReason::MissingArtifact {
                        expected_artifact_ref: required.clone(),
                        bundle_id: Some(inputs.artifact_bundle.bundle_id),
                    },
                );
            }
        }

        // All gates passed.
        PromotionDecisionV1::accepted(
            inputs.sandbox_run.run_id.0.clone(),
            inputs.validation_run_id.clone(),
        )
    }

    fn receipt_artifact_refs(inputs: &PromotionGateInputs<'_>) -> Vec<String> {
        let mut refs: Vec<String> = inputs
            .artifact_bundle
            .handles
            .iter()
            .map(|h| h.handle.clone())
            .collect();
        if let Some(r) = &inputs.report_artifact_ref {
            if !refs.contains(r) {
                refs.push(r.clone());
            }
        }
        refs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::kb003_artifact_classes::Kb003ArtifactClass;
    use crate::kernel::kb003_promotion::artifact_bundle::{
        Kb003ArtifactHandleV1, KbArtifactBundleAssembler,
    };
    use crate::kernel::sandbox::denial::{DenialKind, SandboxDenialRecordV1};
    use crate::kernel::sandbox::policy::SandboxCapability;
    use crate::kernel::sandbox::run::{SandboxRunStatus, SandboxRunV1};
    use crate::kernel::validation::report::{DescriptorOutcome, ValidationReport};
    use crate::kernel::validation::status::ValidationStatus;
    use crate::storage::kb003_storage::InMemoryKb003Storage;
    use uuid::Uuid;

    fn completed_run() -> SandboxRunV1 {
        let mut run = SandboxRunV1::new_requested("KTR-1", "SES-1", "process_tier", "POL-1@1", "WSP-1");
        run.status = SandboxRunStatus::Completed;
        run
    }

    fn pass_report() -> ValidationReport {
        let mut r = ValidationReport::new(Uuid::new_v4());
        r.push(DescriptorOutcome::new("no_sandbox_escape", ValidationStatus::pass()));
        r.push(DescriptorOutcome::new("artifact_hashes_valid", ValidationStatus::pass()));
        r
    }

    fn bundle(run: &SandboxRunV1) -> Kb003ArtifactBundleV1 {
        let handles = vec![
            Kb003ArtifactHandleV1::new(Kb003ArtifactClass::SandboxLog, "h1aaaaaaaaaaaaaa").unwrap(),
            Kb003ArtifactHandleV1::new(Kb003ArtifactClass::ValidationReport, "h2bbbbbbbbbbbbbb")
                .unwrap(),
        ];
        KbArtifactBundleAssembler::assemble(run, handles).unwrap()
    }

    fn good_approval() -> OperatorApprovalEvidence {
        OperatorApprovalEvidence::new(
            "op-ilja",
            "OPR-deadbeef-1234-5678-9abc-def012345678",
            "operator_review_receipt",
            "validation passed, promoting",
        )
    }

    fn good_inputs<'a>(
        run: &'a SandboxRunV1,
        report: &'a ValidationReport,
        bun: &'a Kb003ArtifactBundleV1,
    ) -> PromotionGateInputs<'a> {
        PromotionGateInputs {
            sandbox_run: run,
            validation_report: report,
            validation_run_id: "VR-1".into(),
            artifact_bundle: bun,
            approval: good_approval(),
            idempotency_key: "IK-1".into(),
            required_artifact_refs: vec![],
            latest_known_run_id: None,
            denial: None,
            policy_version_id: "POL-1@1".into(),
            report_artifact_ref: Some("kb003://validation_report/h2bbbbbbbbbbbbbb".into()),
            treat_advisory_as_blocking: false,
        }
    }

    // MT-045: blocking check failure prevents promotion.
    #[test]
    fn validation_failure_rejects_with_typed_reason() {
        let run = completed_run();
        let mut report = pass_report();
        report.push(DescriptorOutcome::new(
            "no_sandbox_escape",
            ValidationStatus::fail("wrote outside workspace").unwrap(),
        ));
        let bun = bundle(&run);
        let inputs = good_inputs(&run, &report, &bun);
        let dec = PromotionGate::decide(&inputs);
        match dec.outcome {
            PromotionOutcome::Rejected {
                reason: PromotionRejectionReason::ValidationFailure { blocking_outcomes, report_artifact_ref, .. },
            } => {
                assert!(blocking_outcomes.iter().any(|n| n == "no_sandbox_escape"));
                assert!(report_artifact_ref.is_some(), "MT-046 anchors report artifact");
            }
            other => panic!("expected ValidationFailure, got {other:?}"),
        }
    }

    // MT-048: advisory does not block by default; opting into strict mode flips it.
    #[test]
    fn advisory_outcome_blocks_only_in_strict_mode() {
        let run = completed_run();
        let mut report = pass_report();
        report.push(DescriptorOutcome::new(
            "lint_advisory",
            ValidationStatus::advisory("style nit").unwrap(),
        ));
        let bun = bundle(&run);

        let lenient = good_inputs(&run, &report, &bun);
        assert!(PromotionGate::decide(&lenient).is_accepted());

        let mut strict = good_inputs(&run, &report, &bun);
        strict.treat_advisory_as_blocking = true;
        match PromotionGate::decide(&strict).outcome {
            PromotionOutcome::Rejected {
                reason: PromotionRejectionReason::ValidationFailure { blocking_outcomes, .. },
            } => {
                assert!(blocking_outcomes.iter().any(|n| n == "lint_advisory"));
            }
            other => panic!("expected strict-mode ValidationFailure, got {other:?}"),
        }
    }

    // Missing approval rejection records the missing field name.
    #[test]
    fn missing_approval_records_field_name() {
        let run = completed_run();
        let report = pass_report();
        let bun = bundle(&run);
        let mut inputs = good_inputs(&run, &report, &bun);
        inputs.approval = OperatorApprovalEvidence::new("", "OPR-1", "operator_review_receipt", "r");
        match PromotionGate::decide(&inputs).outcome {
            PromotionOutcome::Rejected {
                reason: PromotionRejectionReason::MissingApproval { missing_field },
            } => assert_eq!(missing_field, "operator_id"),
            other => panic!("expected MissingApproval, got {other:?}"),
        }
    }

    // Fixture-looking approval evidence is refused.
    #[test]
    fn fixture_like_approval_is_refused() {
        let run = completed_run();
        let report = pass_report();
        let bun = bundle(&run);
        let mut inputs = good_inputs(&run, &report, &bun);
        inputs.approval = OperatorApprovalEvidence::new(
            "kernel-proof-fixture",
            "OPR-1",
            "operator_review_receipt",
            "fixture for tests",
        );
        let dec = PromotionGate::decide(&inputs);
        assert!(!dec.is_accepted());
        assert_eq!(dec.outcome.tag(), "REJECTED_MISSING_APPROVAL");
    }

    // Missing required artifact rejection records the artifact ref.
    #[test]
    fn missing_required_artifact_rejects_with_ref() {
        let run = completed_run();
        let report = pass_report();
        let bun = bundle(&run);
        let mut inputs = good_inputs(&run, &report, &bun);
        inputs.required_artifact_refs = vec!["kb003://promotion_receipt/never".into()];
        match PromotionGate::decide(&inputs).outcome {
            PromotionOutcome::Rejected {
                reason: PromotionRejectionReason::MissingArtifact { expected_artifact_ref, bundle_id },
            } => {
                assert_eq!(expected_artifact_ref, "kb003://promotion_receipt/never");
                assert_eq!(bundle_id, Some(bun.bundle_id));
            }
            other => panic!("expected MissingArtifact, got {other:?}"),
        }
    }

    // Stale candidate: latest_known_run_id differs from sandbox.run_id.
    #[test]
    fn stale_candidate_rejection() {
        let run = completed_run();
        let report = pass_report();
        let bun = bundle(&run);
        let mut inputs = good_inputs(&run, &report, &bun);
        inputs.latest_known_run_id = Some("SBX-newer".into());
        match PromotionGate::decide(&inputs).outcome {
            PromotionOutcome::Rejected {
                reason: PromotionRejectionReason::StaleCandidate { latest_run_id, .. },
            } => assert_eq!(latest_run_id, "SBX-newer"),
            other => panic!("expected StaleCandidate, got {other:?}"),
        }
    }

    // Policy denial path: denial record present.
    #[test]
    fn policy_denial_rejection() {
        let run = completed_run();
        let report = pass_report();
        let bun = bundle(&run);
        let denial = SandboxDenialRecordV1::new(
            run.run_id.0.clone(),
            "POL-1@1",
            DenialKind::PolicyDenied,
            Some(SandboxCapability::Network),
            "fetch https://x",
            "default_deny NETWORK",
        );
        let mut inputs = good_inputs(&run, &report, &bun);
        inputs.denial = Some(&denial);
        match PromotionGate::decide(&inputs).outcome {
            PromotionOutcome::Rejected {
                reason: PromotionRejectionReason::PolicyDenial { capability, policy_version_id, .. },
            } => {
                assert_eq!(capability.as_deref(), Some("NETWORK"));
                assert_eq!(policy_version_id, "POL-1@1");
            }
            other => panic!("expected PolicyDenial, got {other:?}"),
        }
    }

    // Happy path: accepted decision through storage; receipt persisted.
    #[test]
    fn happy_path_emits_accepted_receipt_through_storage() {
        let run = completed_run();
        let report = pass_report();
        let bun = bundle(&run);
        let inputs = good_inputs(&run, &report, &bun);
        let mut store = InMemoryKb003Storage::new_postgres_primary();
        let out = PromotionGate::evaluate(inputs, &mut store).unwrap();
        assert!(out.decision.is_accepted());
        assert_eq!(out.stored_receipt_id.as_deref(), Some(out.receipt.receipt_id.as_str()));
        assert_eq!(store.promotion_receipts.len(), 1);
        assert_eq!(store.promotion_decisions.len(), 1);
        assert_eq!(store.promotion_decisions[0].decision, "ACCEPTED");
    }

    // Idempotency: same key + same payload returns the same receipt id (no
    // duplicate row).
    #[test]
    fn idempotency_same_payload_returns_same_receipt() {
        let run = completed_run();
        let report = pass_report();
        let bun = bundle(&run);
        let mut store = InMemoryKb003Storage::new_postgres_primary();

        let inputs1 = good_inputs(&run, &report, &bun);
        let out1 = PromotionGate::evaluate(inputs1, &mut store).unwrap();
        assert!(out1.decision.is_accepted());

        // Second call: same idempotency key, same payload (decision is fresh
        // but the receipt body's canonical projection matches).
        let inputs2 = good_inputs(&run, &report, &bun);
        let out2 = PromotionGate::evaluate(inputs2, &mut store).unwrap();
        // The new decision row is added (gate does not dedup decisions), but
        // the receipt insert is idempotent and returns the original receipt id.
        assert_eq!(
            out2.stored_receipt_id.as_deref(),
            Some(out1.receipt.receipt_id.as_str())
        );
        assert_eq!(store.promotion_receipts.len(), 1, "no duplicate receipt row");
    }

    // Idempotency conflict: same key + different payload surfaces as a typed
    // DuplicateIdempotencyKey rejection (mapped from storage error).
    #[test]
    fn idempotency_conflict_surfaces_as_typed_rejection() {
        let run = completed_run();
        let report = pass_report();
        let bun = bundle(&run);
        let mut store = InMemoryKb003Storage::new_postgres_primary();

        let inputs1 = good_inputs(&run, &report, &bun);
        let _ = PromotionGate::evaluate(inputs1, &mut store).unwrap();

        // Same key but different validation_run_id changes the payload hash.
        let mut inputs2 = good_inputs(&run, &report, &bun);
        inputs2.validation_run_id = "VR-DIFFERENT".into();
        let out2 = PromotionGate::evaluate(inputs2, &mut store).unwrap();
        match out2.decision.outcome {
            PromotionOutcome::Rejected {
                reason: PromotionRejectionReason::DuplicateIdempotencyKey { idempotency_key, .. },
            } => assert_eq!(idempotency_key, "IK-1"),
            other => panic!("expected DuplicateIdempotencyKey, got {other:?}"),
        }
    }
}
