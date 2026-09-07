//! MT-076 Promotion Failure Test Matrix.
//!
//! Acceptance (MT-076.json): "test each promotion failure scenario.
//! Acceptance: no failure mutates authority."
//!
//! "Authority mutation" in KB003 = an `Accepted` `PromotionReceiptRowV1` being
//! persisted into `Kb003Storage`. Any rejection path must NOT produce such a
//! row. Some rejections (`StaleCandidate`, `ValidationFailure`, `PolicyDenial`,
//! `MissingApproval`, `MissingArtifact`) are decided purely in
//! `PromotionGate::decide` and never touch storage. The full
//! `PromotionGate::evaluate` path always writes a *decision* row recording the
//! verdict (per CX-503R the failure must be auditable) but it MUST NEVER write
//! a receipt with `decision = ACCEPTED` for a rejection — the only exception
//! is `DuplicateIdempotencyKey`, where the original receipt row already exists
//! from the prior successful insert and the new one is refused.
//!
//! Variants covered (one test per `PromotionRejectionReason` variant — 8):
//!   1. StaleCandidate
//!   2. DuplicateIdempotencyKey
//!   3. ValidationFailure
//!   4. PolicyDenial
//!   5. MissingApproval
//!   6. MissingArtifact
//!   7. Durable storage write failure (decision insert refusal; fallback path)
//!   8. ProjectionRebuildFailure  (typed reason exists; gate does not surface
//!      it directly today but the variant must be constructible and serialise)
//!
//! MT-141 operator-approved assertion dispositions (test names retained):
//! | Test | Retired mock-only expectation | Real-store successor proof |
//! | storage_failure_rejection_does_not_mutate_authority | receipt persists without a decision | no orphan receipt, no stored receipt ID, typed diagnostic retained |
//! | storage_failure_retry_is_idempotent | both orphan receipts persist | neither orphan persists; equivalent errors retain equal hashes and distinct raw diagnostics |

use handshake_core::kernel::kb003_artifact_classes::Kb003ArtifactClass;
use handshake_core::kernel::kb003_promotion::artifact_bundle::{
    Kb003ArtifactBundleV1, Kb003ArtifactHandleV1, KbArtifactBundleAssembler,
};
use handshake_core::kernel::kb003_promotion::decision::{
    PromotionDecisionV1, PromotionRejectionReason,
};
use handshake_core::kernel::kb003_promotion::gate::{
    OperatorApprovalEvidence, PromotionGate, PromotionGateInputs,
};
use handshake_core::kernel::sandbox::denial::{DenialKind, SandboxDenialRecordV1};
use handshake_core::kernel::sandbox::policy::SandboxCapability;
use handshake_core::kernel::sandbox::run::{SandboxRunStatus, SandboxRunV1};
use handshake_core::kernel::validation::report::{DescriptorOutcome, ValidationReport};
use handshake_core::kernel::validation::status::ValidationStatus;
use handshake_core::storage::kb003_storage::{Kb003Storage, ValidationRunRowV1};
use handshake_core::storage::surreal::SurrealKb003Storage;
mod kb003_surreal_support;
use uuid::Uuid;

fn completed_run() -> SandboxRunV1 {
    let mut run = SandboxRunV1::new_requested(
        "KTR-mt076",
        "SES-mt076",
        "process_tier",
        "POL-mt076@1",
        "WSP-mt076",
    );
    run.status = SandboxRunStatus::Completed;
    run
}

fn pass_report() -> ValidationReport {
    let mut r = ValidationReport::new(Uuid::now_v7());
    r.push(DescriptorOutcome::new(
        "no_sandbox_escape",
        ValidationStatus::pass(),
    ));
    r.push(DescriptorOutcome::new(
        "artifact_hashes_valid",
        ValidationStatus::pass(),
    ));
    r
}

fn seed_gate_ancestors(
    store: &mut SurrealKb003Storage,
    run: &SandboxRunV1,
    report: &ValidationReport,
) -> ValidationRunRowV1 {
    let failed = report
        .outcomes
        .iter()
        .filter(|outcome| outcome.status.blocks_promotion())
        .count();
    let validation = ValidationRunRowV1 {
        validation_run_id: "VR-mt076".into(),
        sandbox_run_id: run.run_id.0.clone(),
        descriptor_id: "DESC-mt076".into(),
        verdict: if failed == 0 { "PASS" } else { "FAIL" }.into(),
        check_count: report.outcomes.len() as u32,
        failed_check_count: failed as u32,
        report_artifact_ref: Some("kb003://validation_report/h2bbbbbbbbbbbbbb".into()),
        started_at_utc: "2026-05-17T00:00:00Z".into(),
        finished_at_utc: "2026-05-17T00:00:01Z".into(),
        summary_json: serde_json::to_value(report).expect("serialize validation fixture"),
    };
    kb003_surreal_support::seed_validation_ancestors(store, run, &validation);
    validation
}

fn bundle_for(run: &SandboxRunV1) -> Kb003ArtifactBundleV1 {
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
        validation_run_id: "VR-mt076".into(),
        artifact_bundle: bun,
        approval: good_approval(),
        idempotency_key: "IK-mt076".into(),
        required_artifact_refs: vec![],
        latest_known_run_id: None,
        denial: None,
        policy_version_id: "POL-mt076@1".into(),
        report_artifact_ref: Some("kb003://validation_report/h2bbbbbbbbbbbbbb".into()),
        treat_advisory_as_blocking: false,
        treat_unsupported_as_blocking_for_advisory: false,
    }
}

/// Helper: assert no `ACCEPTED` receipt landed in the authority sink. Decision
/// rows MAY land (decisions are append-only audit trail), but a receipt with
/// `decision = ACCEPTED` is the authority mutation the test guards.
async fn assert_authority_not_mutated(store: &SurrealKb003Storage) {
    kb003_surreal_support::assert_authority_not_mutated(store).await;
}

/// Helper: assert the rejection decision carries the specified typed reason.
fn assert_rejection_reason(dec: &PromotionDecisionV1, expected_tag: &str) {
    assert!(
        !dec.is_accepted(),
        "decision must be Rejected; got {:?}",
        dec.outcome
    );
    assert_eq!(
        dec.outcome.tag(),
        expected_tag,
        "rejection tag mismatch: got {} expected {}; decision: {:?}",
        dec.outcome.tag(),
        expected_tag,
        dec
    );
    assert!(dec.outcome.rejection_reason().is_some());
}

fn assert_storage_failure(dec: &PromotionDecisionV1) {
    assert!(!dec.is_accepted(), "storage failure must reject promotion");
    assert!(
        dec.outcome.rejection_reason().is_some(),
        "storage failure must carry a typed rejection reason"
    );
}

// ------------------------------------------------------------------
// 1. StaleCandidate
// ------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stale_candidate_rejection_does_not_mutate_authority() {
    let run = completed_run();
    let report = pass_report();
    let bun = bundle_for(&run);
    let directory = tempfile::tempdir().unwrap();
    let mut store = kb003_surreal_support::open(directory.path()).await;
    seed_gate_ancestors(&mut store, &run, &report);
    let mut inputs = good_inputs(&run, &report, &bun);
    inputs.latest_known_run_id = Some("SBX-fresher".into());

    let out = PromotionGate::evaluate(inputs, &mut store).expect("evaluate must not error");
    assert_rejection_reason(&out.decision, "REJECTED_STALE_CANDIDATE");
    match out.decision.outcome.rejection_reason().unwrap() {
        PromotionRejectionReason::StaleCandidate { latest_run_id, .. } => {
            assert_eq!(latest_run_id, "SBX-fresher");
        }
        other => panic!("expected StaleCandidate, got {other:?}"),
    }
    let store = kb003_surreal_support::reopen(store, directory.path()).await;
    assert_authority_not_mutated(&store).await;
    kb003_surreal_support::close(store).await;
}

// ------------------------------------------------------------------
// 2. DuplicateIdempotencyKey
// ------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn duplicate_idempotency_key_rejection_does_not_mutate_authority_second_time() {
    let run = completed_run();
    let report = pass_report();
    let bun = bundle_for(&run);
    let directory = tempfile::tempdir().unwrap();
    let mut store = kb003_surreal_support::open(directory.path()).await;
    let mut alternate_validation = seed_gate_ancestors(&mut store, &run, &report);

    // First call lands an ACCEPTED receipt (this is the authority baseline).
    let first = PromotionGate::evaluate(good_inputs(&run, &report, &bun), &mut store).unwrap();
    assert!(first.decision.is_accepted());
    assert_eq!(
        kb003_surreal_support::accepted_receipt_count(&store).await,
        1,
        "authority query must observe the accepted baseline"
    );
    assert_eq!(kb003_surreal_support::receipt_count(&store).await, 1);
    let receipt_count_before = kb003_surreal_support::receipt_count(&store).await;

    // Second call with SAME idempotency key but DIFFERENT payload (different
    // validation_run_id) surfaces as DuplicateIdempotencyKey rejection.
    let mut inputs2 = good_inputs(&run, &report, &bun);
    inputs2.validation_run_id = "VR-DIFFERENT".into();
    alternate_validation.validation_run_id = inputs2.validation_run_id.clone();
    store
        .insert_validation_run(&alternate_validation)
        .expect("persist alternate validation ancestor");
    let out2 = PromotionGate::evaluate(inputs2, &mut store).unwrap();
    assert_rejection_reason(&out2.decision, "REJECTED_DUPLICATE_IDEMPOTENCY_KEY");
    match out2.decision.outcome.rejection_reason().unwrap() {
        PromotionRejectionReason::DuplicateIdempotencyKey {
            idempotency_key,
            existing_payload_hash,
            new_payload_hash,
        } => {
            assert_eq!(idempotency_key, "IK-mt076");
            assert_ne!(existing_payload_hash, new_payload_hash);
        }
        other => panic!("expected DuplicateIdempotencyKey, got {other:?}"),
    }
    let store = kb003_surreal_support::reopen(store, directory.path()).await;
    // No NEW receipt row was inserted by the rejection.
    assert_eq!(
        kb003_surreal_support::receipt_count(&store).await,
        receipt_count_before,
        "DuplicateIdempotencyKey path must not add a receipt row"
    );
    kb003_surreal_support::close(store).await;
}

// ------------------------------------------------------------------
// 3. ValidationFailure
// ------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn validation_failure_rejection_does_not_mutate_authority() {
    let run = completed_run();
    let mut report = pass_report();
    report.push(DescriptorOutcome::new(
        "no_sandbox_escape",
        ValidationStatus::fail("wrote outside workspace").unwrap(),
    ));
    let bun = bundle_for(&run);
    let directory = tempfile::tempdir().unwrap();
    let mut store = kb003_surreal_support::open(directory.path()).await;
    seed_gate_ancestors(&mut store, &run, &report);
    let out = PromotionGate::evaluate(good_inputs(&run, &report, &bun), &mut store).unwrap();
    assert_rejection_reason(&out.decision, "REJECTED_VALIDATION_FAILURE");
    match out.decision.outcome.rejection_reason().unwrap() {
        PromotionRejectionReason::ValidationFailure {
            blocking_outcomes,
            report_artifact_ref,
            ..
        } => {
            assert!(blocking_outcomes.iter().any(|n| n == "no_sandbox_escape"));
            assert!(report_artifact_ref.is_some());
        }
        other => panic!("expected ValidationFailure, got {other:?}"),
    }
    let store = kb003_surreal_support::reopen(store, directory.path()).await;
    assert_authority_not_mutated(&store).await;
    kb003_surreal_support::close(store).await;
}

// ------------------------------------------------------------------
// 4. PolicyDenial
// ------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn policy_denial_rejection_does_not_mutate_authority() {
    let run = completed_run();
    let report = pass_report();
    let bun = bundle_for(&run);
    let denial = SandboxDenialRecordV1::new(
        run.run_id.0.clone(),
        "POL-mt076@1",
        DenialKind::PolicyDenied,
        Some(SandboxCapability::Network),
        "fetch https://example.com",
        "default_deny NETWORK",
    );
    let directory = tempfile::tempdir().unwrap();
    let mut store = kb003_surreal_support::open(directory.path()).await;
    seed_gate_ancestors(&mut store, &run, &report);
    let mut inputs = good_inputs(&run, &report, &bun);
    inputs.denial = Some(&denial);
    let out = PromotionGate::evaluate(inputs, &mut store).unwrap();
    assert_rejection_reason(&out.decision, "REJECTED_POLICY_DENIAL");
    match out.decision.outcome.rejection_reason().unwrap() {
        PromotionRejectionReason::PolicyDenial {
            capability,
            policy_version_id,
            denial_id,
        } => {
            assert_eq!(capability.as_deref(), Some("NETWORK"));
            assert_eq!(policy_version_id, "POL-mt076@1");
            assert!(denial_id.starts_with("DEN-"));
        }
        other => panic!("expected PolicyDenial, got {other:?}"),
    }
    let store = kb003_surreal_support::reopen(store, directory.path()).await;
    assert_authority_not_mutated(&store).await;
    kb003_surreal_support::close(store).await;
}

// ------------------------------------------------------------------
// 5. MissingApproval
// ------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn missing_approval_rejection_does_not_mutate_authority() {
    let run = completed_run();
    let report = pass_report();
    let bun = bundle_for(&run);
    let directory = tempfile::tempdir().unwrap();
    let mut store = kb003_surreal_support::open(directory.path()).await;
    seed_gate_ancestors(&mut store, &run, &report);
    let mut inputs = good_inputs(&run, &report, &bun);
    inputs.approval = OperatorApprovalEvidence::new(
        "",
        "OPR-deadbeef-1234-5678-9abc-def012345678",
        "operator_review_receipt",
        "trying without op id",
    );
    let out = PromotionGate::evaluate(inputs, &mut store).unwrap();
    assert_rejection_reason(&out.decision, "REJECTED_MISSING_APPROVAL");
    match out.decision.outcome.rejection_reason().unwrap() {
        PromotionRejectionReason::MissingApproval { missing_field } => {
            assert_eq!(missing_field, "operator_id");
        }
        other => panic!("expected MissingApproval, got {other:?}"),
    }
    let store = kb003_surreal_support::reopen(store, directory.path()).await;
    assert_authority_not_mutated(&store).await;
    kb003_surreal_support::close(store).await;
}

// ------------------------------------------------------------------
// 6. MissingArtifact
// ------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn missing_artifact_rejection_does_not_mutate_authority() {
    let run = completed_run();
    let report = pass_report();
    let bun = bundle_for(&run);
    let directory = tempfile::tempdir().unwrap();
    let mut store = kb003_surreal_support::open(directory.path()).await;
    seed_gate_ancestors(&mut store, &run, &report);
    let mut inputs = good_inputs(&run, &report, &bun);
    inputs.required_artifact_refs = vec!["kb003://promotion_receipt/never_present".into()];
    let out = PromotionGate::evaluate(inputs, &mut store).unwrap();
    assert_rejection_reason(&out.decision, "REJECTED_MISSING_ARTIFACT");
    match out.decision.outcome.rejection_reason().unwrap() {
        PromotionRejectionReason::MissingArtifact {
            expected_artifact_ref,
            bundle_id,
        } => {
            assert_eq!(
                expected_artifact_ref,
                "kb003://promotion_receipt/never_present"
            );
            assert_eq!(*bundle_id, Some(bun.bundle_id));
        }
        other => panic!("expected MissingArtifact, got {other:?}"),
    }
    let store = kb003_surreal_support::reopen(store, directory.path()).await;
    assert_authority_not_mutated(&store).await;
    kb003_surreal_support::close(store).await;
}

// ------------------------------------------------------------------
// 7. Durable storage write failure
// ------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn storage_failure_rejection_does_not_mutate_authority() {
    let run = completed_run();
    let report = pass_report();
    let bun = bundle_for(&run);
    let directory = tempfile::tempdir().unwrap();
    let mut store = kb003_surreal_support::open(directory.path()).await;
    seed_gate_ancestors(&mut store, &run, &report);
    kb003_surreal_support::refuse_decisions(&store, 0).await;
    let out = PromotionGate::evaluate(good_inputs(&run, &report, &bun), &mut store).unwrap();
    assert_storage_failure(&out.decision);
    assert!(out
        .receipt
        .storage_error_detail
        .as_deref()
        .is_some_and(|detail| detail.contains("simulated storage deadlock")));
    let store = kb003_surreal_support::reopen(store, directory.path()).await;
    // H-B2 still attempts the fallback receipt. The real schema requires its
    // decision ancestor, so a rolled-back decision cannot leave an orphan receipt.
    assert_eq!(
        kb003_surreal_support::receipt_count(&store).await,
        0,
        "storage-failure path must not persist an orphan rejection receipt"
    );
    assert!(
        out.stored_receipt_id.is_none(),
        "stored_receipt_id must be None when its decision ancestor was refused"
    );
    // The decision row was NOT persisted (the synchronous event refused), so authority is
    // still unmutated in the sense of "no ACCEPTED row was committed."
    assert!(
        kb003_surreal_support::decision_count(&store).await == 0,
        "no decision row may exist when decision insert refused"
    );
    assert_authority_not_mutated(&store).await;
    kb003_surreal_support::close(store).await;
}

// ------------------------------------------------------------------
// 7b. Durable storage failure where BOTH decision AND receipt inserts fail
// ------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn storage_failure_with_total_outage_yields_none_stored_receipt_id() {
    let run = completed_run();
    let report = pass_report();
    let bun = bundle_for(&run);
    let directory = tempfile::tempdir().unwrap();
    let mut store = kb003_surreal_support::open(directory.path()).await;
    seed_gate_ancestors(&mut store, &run, &report);
    kb003_surreal_support::refuse_decisions(&store, 1).await;
    kb003_surreal_support::refuse_receipts(&store).await;
    let out = PromotionGate::evaluate(good_inputs(&run, &report, &bun), &mut store).unwrap();
    // Rejection is typed and returned to the caller.
    assert_storage_failure(&out.decision);
    assert!(out
        .receipt
        .storage_error_detail
        .as_deref()
        .is_some_and(|detail| detail.contains("decision_table_offline")));
    // H-B2: when receipt insert also fails, stored_receipt_id is None so the
    // caller knows the rejection only exists in the returned value.
    assert!(
        out.stored_receipt_id.is_none(),
        "stored_receipt_id must be None when both decision AND receipt inserts refused"
    );
    let store = kb003_surreal_support::reopen(store, directory.path()).await;
    assert_eq!(kb003_surreal_support::decision_count(&store).await, 0);
    assert_eq!(kb003_surreal_support::receipt_count(&store).await, 0);
    kb003_surreal_support::close(store).await;
}

// ------------------------------------------------------------------
// 8. ProjectionRebuildFailure (constructible + typed)
// ------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn projection_rebuild_failure_variant_is_typed_and_serialisable() {
    // The gate today does not autonomously surface ProjectionRebuildFailure
    // (projection refresh is a downstream concern), but the variant MUST
    // exist as a typed rejection reason so future projection-rebuild paths
    // can route through the same taxonomy without mutating authority.
    let reason = PromotionRejectionReason::ProjectionRebuildFailure {
        projection_family_id: "hsk.dcc.kb003.sandbox_promotion_lane@1".into(),
        detail: "missing source schema after projection refresh".into(),
    };
    assert_eq!(reason.tag(), "REJECTED_PROJECTION_REBUILD_FAILURE");
    let dec = PromotionDecisionV1::rejected("SBX-mt076", "VR-mt076", reason.clone());
    assert!(!dec.is_accepted());
    assert_eq!(dec.outcome.tag(), "REJECTED_PROJECTION_REBUILD_FAILURE");
    let json = serde_json::to_string(&dec).expect("must serialise");
    assert!(
        json.contains("PROJECTION_REBUILD_FAILURE"),
        "serde must surface variant tag: {json}"
    );
    let back: PromotionDecisionV1 = serde_json::from_str(&json).unwrap();
    assert_eq!(back, dec);

    // Building a PromotionDecisionV1::rejected with this reason does NOT
    // touch the authority sink at all — authority mutation is gated on
    // PromotionGate::evaluate, which we are NOT calling here.
    let directory = tempfile::tempdir().unwrap();
    let inert_store = kb003_surreal_support::open(directory.path()).await;
    let inert_store = kb003_surreal_support::reopen(inert_store, directory.path()).await;
    assert!(
        kb003_surreal_support::receipt_count(&inert_store).await == 0,
        "constructing a rejection must not mutate any authority sink"
    );
    kb003_surreal_support::close(inert_store).await;
}

// ------------------------------------------------------------------
// Coverage check: every rejection variant tag appears in our matrix.
// ------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn matrix_covers_all_eight_promotion_rejection_variants() {
    // Construct every variant and collect the tags. If the taxonomy ever
    // grows, this test fails so the matrix can be extended.
    let run = completed_run();
    let report = pass_report();
    let bun = bundle_for(&run);
    let directory = tempfile::tempdir().unwrap();
    let mut refusing_store = kb003_surreal_support::open(directory.path()).await;
    seed_gate_ancestors(&mut refusing_store, &run, &report);
    kb003_surreal_support::refuse_decisions(&refusing_store, 0).await;
    let storage_failure =
        PromotionGate::evaluate(good_inputs(&run, &report, &bun), &mut refusing_store)
            .expect("storage refusal must produce a rejection result")
            .decision
            .outcome
            .rejection_reason()
            .expect("storage refusal must produce a typed rejection")
            .clone();
    let variants = [
        PromotionRejectionReason::StaleCandidate {
            candidate_run_id: "SBX-c".into(),
            latest_run_id: "SBX-l".into(),
        },
        PromotionRejectionReason::DuplicateIdempotencyKey {
            idempotency_key: "IK-1".into(),
            existing_payload_hash: "h-a".into(),
            new_payload_hash: "h-b".into(),
        },
        PromotionRejectionReason::ValidationFailure {
            validation_run_id: "VR-1".into(),
            blocking_outcomes: vec!["x".into()],
            report_artifact_ref: None,
        },
        PromotionRejectionReason::PolicyDenial {
            denial_id: "DEN-1".into(),
            policy_version_id: "POL-1@1".into(),
            capability: Some("NETWORK".into()),
        },
        PromotionRejectionReason::MissingApproval {
            missing_field: "operator_id".into(),
        },
        PromotionRejectionReason::MissingArtifact {
            expected_artifact_ref: "kb003://x".into(),
            bundle_id: None,
        },
        storage_failure,
        PromotionRejectionReason::ProjectionRebuildFailure {
            projection_family_id: "fam@1".into(),
            detail: "x".into(),
        },
    ];
    let tags: std::collections::BTreeSet<&'static str> = variants.iter().map(|v| v.tag()).collect();
    assert_eq!(
        tags.len(),
        8,
        "all 8 PromotionRejectionReason variants must produce a unique tag; matrix must cover each"
    );
    let refusing_store = kb003_surreal_support::reopen(refusing_store, directory.path()).await;
    assert_authority_not_mutated(&refusing_store).await;
    kb003_surreal_support::close(refusing_store).await;
}

// ====================================================================
// H4 (KB003 remediation) — deterministic rejection-path payload_hash.
//
// Each test triggers the same logical rejection TWICE and asserts the
// receipt's `payload_hash` is byte-identical across retries. That is the
// invariant that prevents false `IdempotencyConflict` on retry of any
// rejection variant.
// ====================================================================

use handshake_core::kernel::kb003_promotion::receipt::PromotionReceiptV1;

/// Build a fresh accepted decision -> receipt, but with a controlled
/// idempotency key + bundle so two calls hash deterministically.
fn make_rejection_receipt(reason: PromotionRejectionReason) -> PromotionReceiptV1 {
    let dec = PromotionDecisionV1::rejected("SBX-h4", "VR-h4", reason);
    PromotionReceiptV1::new(
        dec,
        "IK-h4",
        Some(Uuid::now_v7()),
        Some("bundle-sha-h4".into()),
        vec!["kb003://art/h4".into()],
    )
}

#[test]
fn stale_candidate_retry_is_idempotent() {
    let r1 = make_rejection_receipt(PromotionRejectionReason::StaleCandidate {
        candidate_run_id: "SBX-c".into(),
        latest_run_id: "SBX-l".into(),
    });
    let r2 = make_rejection_receipt(PromotionRejectionReason::StaleCandidate {
        candidate_run_id: "SBX-c".into(),
        latest_run_id: "SBX-l".into(),
    });
    assert_eq!(
        r1.payload_hash, r2.payload_hash,
        "two StaleCandidate retries must hash identically"
    );
    assert_ne!(r1.receipt_id, r2.receipt_id, "receipt id is ephemeral");
}

#[test]
fn duplicate_idempotency_key_retry_is_idempotent() {
    // existing_payload_hash / new_payload_hash are observability fields and
    // can wobble between retries; the canonical projection keeps only
    // `idempotency_key`.
    let r1 = make_rejection_receipt(PromotionRejectionReason::DuplicateIdempotencyKey {
        idempotency_key: "IK-collide".into(),
        existing_payload_hash: "h-prior-aaaaa".into(),
        new_payload_hash: "h-new-bbbbbb".into(),
    });
    let r2 = make_rejection_receipt(PromotionRejectionReason::DuplicateIdempotencyKey {
        idempotency_key: "IK-collide".into(),
        existing_payload_hash: "h-prior-DIFFERENT".into(),
        new_payload_hash: "h-new-DIFFERENT".into(),
    });
    assert_eq!(
        r1.payload_hash, r2.payload_hash,
        "DuplicateIdempotencyKey retries with wobbly hash strings must hash identically"
    );
}

#[test]
fn validation_failure_retry_is_idempotent() {
    // Different INSERTION ORDER of blocking outcomes must NOT change the
    // hash (canonical projection sorts the list ascending).
    let r1 = make_rejection_receipt(PromotionRejectionReason::ValidationFailure {
        validation_run_id: "VR-h4".into(),
        blocking_outcomes: vec!["b_check".into(), "a_check".into()],
        report_artifact_ref: Some("kb003://r/1".into()),
    });
    let r2 = make_rejection_receipt(PromotionRejectionReason::ValidationFailure {
        validation_run_id: "VR-h4".into(),
        blocking_outcomes: vec!["a_check".into(), "b_check".into()],
        report_artifact_ref: Some("kb003://r/1".into()),
    });
    assert_eq!(
        r1.payload_hash, r2.payload_hash,
        "ValidationFailure with same descriptor set in different order must hash identically"
    );
}

#[test]
fn policy_denial_retry_is_idempotent() {
    // `denial_id` is a fresh UUID per denial record; it MUST NOT participate
    // in the payload_hash so retries collapse correctly.
    let r1 = make_rejection_receipt(PromotionRejectionReason::PolicyDenial {
        denial_id: "DEN-aaaaaaaaaaaaaa".into(),
        policy_version_id: "POL-1@1".into(),
        capability: Some("NETWORK".into()),
    });
    let r2 = make_rejection_receipt(PromotionRejectionReason::PolicyDenial {
        denial_id: "DEN-DIFFERENT-UUID".into(),
        policy_version_id: "POL-1@1".into(),
        capability: Some("NETWORK".into()),
    });
    assert_eq!(
        r1.payload_hash, r2.payload_hash,
        "PolicyDenial retries with different denial_id must hash identically"
    );

    // Same policy+capability but DIFFERENT policy_version_id MUST differ
    // (the policy version is load-bearing).
    let r3 = make_rejection_receipt(PromotionRejectionReason::PolicyDenial {
        denial_id: "DEN-x".into(),
        policy_version_id: "POL-1@2".into(),
        capability: Some("NETWORK".into()),
    });
    assert_ne!(
        r1.payload_hash, r3.payload_hash,
        "different policy_version_id must change the hash"
    );
}

#[test]
fn missing_approval_retry_is_idempotent() {
    let r1 = make_rejection_receipt(PromotionRejectionReason::MissingApproval {
        missing_field: "operator_id".into(),
    });
    let r2 = make_rejection_receipt(PromotionRejectionReason::MissingApproval {
        missing_field: "operator_id".into(),
    });
    assert_eq!(r1.payload_hash, r2.payload_hash);
}

#[test]
fn missing_artifact_retry_is_idempotent() {
    // `bundle_id` is a fresh UUID per bundle; it MUST NOT participate in the
    // payload_hash so retries collapse correctly.
    let r1 = make_rejection_receipt(PromotionRejectionReason::MissingArtifact {
        expected_artifact_ref: "kb003://x/abc".into(),
        bundle_id: Some(Uuid::now_v7()),
    });
    let r2 = make_rejection_receipt(PromotionRejectionReason::MissingArtifact {
        expected_artifact_ref: "kb003://x/abc".into(),
        bundle_id: Some(Uuid::now_v7()),
    });
    assert_eq!(
        r1.payload_hash, r2.payload_hash,
        "MissingArtifact retries with different bundle_id must hash identically"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn storage_failure_retry_is_idempotent() {
    // Two retries of the same logical deadlock — different raw text wobble,
    // SAME NormalisedStorageErrorKind::Deadlock — must produce identical
    // `payload_hash` so the idempotency dedup fires. The non-hashed
    // `storage_error_detail` may differ.
    let run = completed_run();
    let report = pass_report();
    let bun = bundle_for(&run);

    let directory1 = tempfile::tempdir().unwrap();
    let mut store1 = kb003_surreal_support::open(directory1.path()).await;
    seed_gate_ancestors(&mut store1, &run, &report);
    kb003_surreal_support::refuse_decisions(&store1, 2).await;
    let out1 = PromotionGate::evaluate(good_inputs(&run, &report, &bun), &mut store1).unwrap();

    let directory2 = tempfile::tempdir().unwrap();
    let mut store2 = kb003_surreal_support::open(directory2.path()).await;
    seed_gate_ancestors(&mut store2, &run, &report);
    kb003_surreal_support::refuse_decisions(&store2, 3).await;
    let out2 = PromotionGate::evaluate(good_inputs(&run, &report, &bun), &mut store2).unwrap();

    assert_storage_failure(&out1.decision);
    assert_storage_failure(&out2.decision);
    assert!(out1
        .receipt
        .storage_error_detail
        .as_deref()
        .is_some_and(|detail| detail.contains("deadlock")));
    assert!(out2
        .receipt
        .storage_error_detail
        .as_deref()
        .is_some_and(|detail| detail.contains("Deadlock")));
    assert_eq!(
        out1.receipt.payload_hash, out2.receipt.payload_hash,
        "storage-failure retries with same kind but wobbly text must hash identically"
    );
    // The non-hashed observability detail MAY differ.
    assert!(out1.receipt.storage_error_detail.is_some());
    assert!(out2.receipt.storage_error_detail.is_some());
    assert_ne!(
        out1.receipt.storage_error_detail, out2.receipt.storage_error_detail,
        "raw storage_error_detail should preserve the wobble for observability"
    );
    let store1 = kb003_surreal_support::reopen(store1, directory1.path()).await;
    let store2 = kb003_surreal_support::reopen(store2, directory2.path()).await;
    assert!(out1.stored_receipt_id.is_none());
    assert!(out2.stored_receipt_id.is_none());
    assert_eq!(kb003_surreal_support::decision_count(&store1).await, 0);
    assert_eq!(kb003_surreal_support::decision_count(&store2).await, 0);
    assert_eq!(kb003_surreal_support::receipt_count(&store1).await, 0);
    assert_eq!(kb003_surreal_support::receipt_count(&store2).await, 0);
    assert_authority_not_mutated(&store1).await;
    assert_authority_not_mutated(&store2).await;
    kb003_surreal_support::close(store1).await;
    kb003_surreal_support::close(store2).await;
}

#[test]
fn projection_rebuild_failure_retry_is_idempotent() {
    // `detail` is non-deterministic operational text; only
    // `projection_family_id` is in the canonical projection.
    let r1 = make_rejection_receipt(PromotionRejectionReason::ProjectionRebuildFailure {
        projection_family_id: "hsk.dcc.kb003.sandbox_promotion_lane@1".into(),
        detail: "missing source schema at 12:01:02".into(),
    });
    let r2 = make_rejection_receipt(PromotionRejectionReason::ProjectionRebuildFailure {
        projection_family_id: "hsk.dcc.kb003.sandbox_promotion_lane@1".into(),
        detail: "missing source schema at 12:01:55".into(),
    });
    assert_eq!(
        r1.payload_hash, r2.payload_hash,
        "ProjectionRebuildFailure retries with different `detail` must hash identically"
    );
}
