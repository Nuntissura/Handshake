//! H2 (KB003 remediation) — `AuthorityBoundaryCheck` is now wired into the
//! actual `Kb003Storage::insert_promotion_decision` /
//! `insert_promotion_receipt` write path. Before H2, the boundary check
//! existed in `kernel::mte_authority_mutation_boundary` but no storage
//! call site invoked it, so any caller that could construct a row could
//! mutate authority — defeating the MT-057 contract.
//!
//! These tests exercise the runtime boundary by calling the storage
//! methods directly with a non-PromotionGate actor and asserting the
//! call returns `Kb003StorageError::AuthorityBoundaryDenied`. The
//! PromotionGate actor is asserted to succeed so the happy path stays
//! green.

use handshake_core::kernel::mte_authority_mutation_boundary::AuthorityMutationActor;
use handshake_core::kernel::sandbox::run::SandboxRunV1;
use handshake_core::storage::kb003_storage::{
    Kb003Storage, Kb003StorageError, PromotionDecisionRowV1, PromotionReceiptRowV1,
    ValidationRunRowV1,
};

mod kb003_surreal_support;

fn sample_decision_row() -> PromotionDecisionRowV1 {
    PromotionDecisionRowV1 {
        decision_id: "PD-h2-boundary".into(),
        validation_run_id: "VR-h2-boundary".into(),
        decision: "ACCEPTED".into(),
        rationale_short: "h2 boundary runtime test".into(),
        decided_at_utc: "2026-05-17T00:00:00Z".into(),
    }
}

fn sample_receipt_row() -> PromotionReceiptRowV1 {
    PromotionReceiptRowV1 {
        receipt_id: "PR-h2-boundary".into(),
        decision_id: "PD-h2-boundary".into(),
        idempotency_key: "IK-h2-boundary".into(),
        payload_hash: "h-h2-boundary-aaaa".into(),
        artifact_ref: Some("kb003://art/h2".into()),
        issued_at_utc: "2026-05-17T00:00:01Z".into(),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn non_gate_actor_denied_at_storage_boundary_for_decision() {
    let directory = tempfile::tempdir().expect("isolated KB003 directory");
    let mut store = kb003_surreal_support::open(directory.path()).await;
    for actor in [
        AuthorityMutationActor::Coder,
        AuthorityMutationActor::Validator,
        AuthorityMutationActor::DccProjection,
        AuthorityMutationActor::ExternalActor,
        AuthorityMutationActor::SandboxAdapter,
        AuthorityMutationActor::ValidationRunner,
        AuthorityMutationActor::MteScheduler,
        AuthorityMutationActor::UnknownActor,
    ] {
        let result = store.insert_promotion_decision(&sample_decision_row(), actor);
        match result {
            Err(Kb003StorageError::AuthorityBoundaryDenied(denial)) => {
                assert_eq!(denial.actor, actor);
            }
            other => panic!(
                "actor {:?} should be denied at the PromotionDecision boundary; got {:?}",
                actor, other
            ),
        }
    }
    let store = kb003_surreal_support::reopen(store, directory.path()).await;
    // No row should have landed despite every attempt.
    assert!(
        kb003_surreal_support::decision_count(&store).await == 0,
        "denied actors must not produce decision rows"
    );
    kb003_surreal_support::close(store).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn non_gate_actor_denied_at_storage_boundary_for_receipt() {
    let directory = tempfile::tempdir().expect("isolated KB003 directory");
    let mut store = kb003_surreal_support::open(directory.path()).await;
    for actor in [
        AuthorityMutationActor::Coder,
        AuthorityMutationActor::Validator,
        AuthorityMutationActor::DccProjection,
        AuthorityMutationActor::ExternalActor,
        AuthorityMutationActor::SandboxAdapter,
        AuthorityMutationActor::ValidationRunner,
        AuthorityMutationActor::MteScheduler,
        AuthorityMutationActor::UnknownActor,
    ] {
        let result = store.insert_promotion_receipt(&sample_receipt_row(), actor);
        match result {
            Err(Kb003StorageError::AuthorityBoundaryDenied(denial)) => {
                assert_eq!(denial.actor, actor);
            }
            other => panic!(
                "actor {:?} should be denied at the PromotionReceipt boundary; got {:?}",
                actor, other
            ),
        }
    }
    let store = kb003_surreal_support::reopen(store, directory.path()).await;
    assert!(
        kb003_surreal_support::receipt_count(&store).await == 0,
        "denied actors must not produce receipt rows"
    );
    kb003_surreal_support::close(store).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn promotion_gate_actor_permitted_at_storage_boundary() {
    let directory = tempfile::tempdir().expect("isolated KB003 directory");
    let mut store = kb003_surreal_support::open(directory.path()).await;
    let run = SandboxRunV1::new_requested("KTR-h2", "SES-h2", "local", "POL-h2@1", "WSP-h2");
    kb003_surreal_support::seed_validation_ancestors(
        &mut store,
        &run,
        &ValidationRunRowV1 {
            validation_run_id: sample_decision_row().validation_run_id,
            sandbox_run_id: run.run_id.0.clone(),
            descriptor_id: "DESC-h2".into(),
            verdict: "PASS".into(),
            check_count: 1,
            failed_check_count: 0,
            report_artifact_ref: None,
            started_at_utc: "2026-05-17T00:00:00Z".into(),
            finished_at_utc: "2026-05-17T00:00:00Z".into(),
            summary_json: serde_json::json!({"checks": ["boundary"]}),
        },
    );
    let dec_result = store.insert_promotion_decision(
        &sample_decision_row(),
        AuthorityMutationActor::PromotionGate,
    );
    assert!(
        dec_result.is_ok(),
        "PromotionGate actor must succeed at the decision boundary; got {:?}",
        dec_result
    );
    let rec_result = store
        .insert_promotion_receipt(&sample_receipt_row(), AuthorityMutationActor::PromotionGate);
    assert!(
        rec_result.is_ok(),
        "PromotionGate actor must succeed at the receipt boundary; got {:?}",
        rec_result
    );
    let store = kb003_surreal_support::reopen(store, directory.path()).await;
    assert_eq!(kb003_surreal_support::decision_count(&store).await, 1);
    assert_eq!(kb003_surreal_support::receipt_count(&store).await, 1);
    kb003_surreal_support::close(store).await;
}
