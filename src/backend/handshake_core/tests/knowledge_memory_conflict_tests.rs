//! WP-KERNEL-009 MemoryGraphAndClaims MT-121 (ConflictCandidateSearch),
//! MT-122 (ConflictDetectionAgentJob), MT-123 (ConflictResolutionAgentJob)
//! integration tests against REAL Handshake-managed PostgreSQL.
//!
//! Proof: two facts that assert the same (subject, predicate) but disagree on
//! the object are found by the deterministic candidate search; the detection
//! job records a committed claim conflict (moving both backing claims to
//! `conflicted`) and is idempotent on re-run; a resolution job records a
//! discard outcome with a receipt; and the negative paths (resolution without a
//! kept claim, discard without a discarded claim) fail closed.

mod knowledge_memory_fixtures;

use handshake_core::kernel::{KernelActor, KernelEventType, NewKernelEvent};
use handshake_core::knowledge_memory::conflict::{
    list_conflict_resolution_jobs, record_conflict_resolution_job, run_symbolic_conflict_detection,
    ConflictResolutionOutcome,
};
use handshake_core::storage::knowledge::{KnowledgeClaimState, KnowledgeStore};
use handshake_core::storage::knowledge_memory::{
    create_memory_fact, find_fact_conflict_candidates, list_conflict_detection_findings,
    MemoryClaimAuthorityLabel, MemoryFactObject, NewMemoryFact,
};
use handshake_core::storage::postgres::PostgresDatabase;
use handshake_core::storage::{Database, StorageError};
use knowledge_memory_fixtures::{pool_for, MemoryFixture};
use serde_json::json;
use uuid::Uuid;

/// Build two facts sharing (subject, predicate) but with different literal
/// objects (a symbolic conflict). Returns (subject_entity_id, claim_a, claim_b).
async fn conflicting_facts(fx: &MemoryFixture, pool: &sqlx::PgPool) -> (String, String, String) {
    let subject = fx
        .entity("api", "managed_postgres", "ManagedPostgres")
        .await;
    let claim_a = fx.claim("managed PG port is 5544").await;
    let claim_b = fx.claim("managed PG port is 5432").await;

    for (claim, port) in [(&claim_a, "5544"), (&claim_b, "5432")] {
        create_memory_fact(
            pool,
            NewMemoryFact {
                workspace_id: fx.workspace_id.clone(),
                claim_id: claim.claim_id.clone(),
                subject_entity_id: subject.clone(),
                predicate_key: "default_port".to_string(),
                predicate_term_id: None,
                object: MemoryFactObject::Literal {
                    value: port.to_string(),
                },
                qualifiers: json!({}),
                authority_label: MemoryClaimAuthorityLabel::ModelSuggested,
                extractor_version: "mem_fact_v1".to_string(),
                created_in_run: None,
            },
        )
        .await
        .expect("create fact");
    }
    (subject, claim_a.claim_id, claim_b.claim_id)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn symbolic_candidate_search_finds_object_mismatch() {
    let Some(fx) = MemoryFixture::setup().await else {
        eprintln!("SKIP symbolic_candidate_search_finds_object_mismatch: no PostgreSQL");
        return;
    };
    let pool = pool_for(&fx.pg).await;
    let (subject, claim_a, claim_b) = conflicting_facts(&fx, &pool).await;

    let candidates = find_fact_conflict_candidates(&pool, &fx.workspace_id, 50)
        .await
        .expect("candidate search");
    assert_eq!(candidates.len(), 1, "exactly one conflicting pair");
    let candidate = &candidates[0];
    assert_eq!(candidate.subject_entity_id, subject);
    assert_eq!(candidate.predicate_key, "default_port");
    // The pair is ordered (claim ids follow fact id ordering); both claims are
    // present regardless of order.
    let claims = [candidate.claim_id_a.as_str(), candidate.claim_id_b.as_str()];
    assert!(claims.contains(&claim_a.as_str()) && claims.contains(&claim_b.as_str()));
    assert_ne!(candidate.object_a, candidate.object_b);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn detection_job_records_conflict_and_is_idempotent() {
    let Some(fx) = MemoryFixture::setup().await else {
        eprintln!("SKIP detection_job_records_conflict_and_is_idempotent: no PostgreSQL");
        return;
    };
    let pool = pool_for(&fx.pg).await;
    let db = PostgresDatabase::new(pool.clone());
    let (_subject, claim_a, claim_b) = conflicting_facts(&fx, &pool).await;

    // First detection pass: records one conflict, both claims -> conflicted.
    let result = run_symbolic_conflict_detection(&db, &pool, &fx.workspace_id, 50, None)
        .await
        .expect("detection pass");
    assert_eq!(result.candidates.len(), 1);
    assert_eq!(result.conflict_ids.len(), 1);
    assert_eq!(result.job.conflicts_found, 1);
    assert_eq!(result.job.candidates_scanned, 1);

    for claim_id in [&claim_a, &claim_b] {
        let state = db
            .get_knowledge_claim(claim_id)
            .await
            .expect("get claim")
            .expect("claim exists")
            .lifecycle_state;
        assert_eq!(state, KnowledgeClaimState::Conflicted);
    }

    let findings = list_conflict_detection_findings(&pool, &result.job.job_id)
        .await
        .expect("findings");
    assert_eq!(findings, result.conflict_ids);

    // Second pass: the pair is already conflicting, so no NEW conflict is
    // recorded (idempotent) and the pass does not error.
    let again = run_symbolic_conflict_detection(&db, &pool, &fx.workspace_id, 50, None)
        .await
        .expect("idempotent re-run");
    assert_eq!(
        again.conflict_ids.len(),
        0,
        "re-run must not duplicate the conflict"
    );
    assert_eq!(again.candidates.len(), 1, "the candidate still surfaces");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resolution_job_records_outcome_with_receipt() {
    let Some(fx) = MemoryFixture::setup().await else {
        eprintln!("SKIP resolution_job_records_outcome_with_receipt: no PostgreSQL");
        return;
    };
    let pool = pool_for(&fx.pg).await;
    let db = PostgresDatabase::new(pool.clone());
    let (_subject, claim_a, claim_b) = conflicting_facts(&fx, &pool).await;

    let result = run_symbolic_conflict_detection(&db, &pool, &fx.workspace_id, 50, None)
        .await
        .expect("detection pass");
    let conflict_id = result.conflict_ids[0].clone();

    // A discard resolution: claim_a wins, claim_b discarded. Receipt-backed.
    let suffix = Uuid::now_v7();
    let receipt = db
        .append_kernel_event(
            NewKernelEvent::builder(
                format!("KTR-RES-{suffix}"),
                format!("SR-RES-{suffix}"),
                KernelEventType::ValidationRecorded,
                KernelActor::ValidationRunner("conflict-res".to_string()),
            )
            .aggregate("knowledge_claim_conflict", conflict_id.clone())
            .idempotency_key(format!("idem-res-{suffix}"))
            .payload(json!({"resolution": "discard claim_b"}))
            .build()
            .expect("event"),
        )
        .await
        .expect("append receipt");

    let job = record_conflict_resolution_job(
        &pool,
        &fx.workspace_id,
        &conflict_id,
        ConflictResolutionOutcome::Discard,
        Some(&claim_a),
        Some(&claim_b),
        json!({"rationale": "5544 is the managed default"}),
        &receipt.event_id,
    )
    .await
    .expect("record resolution job");
    assert!(job.job_id.starts_with("KCRJ-"));
    assert_eq!(job.outcome, ConflictResolutionOutcome::Discard);
    assert_eq!(job.resolution_receipt_event_id, receipt.event_id);

    let jobs = list_conflict_resolution_jobs(&pool, &conflict_id)
        .await
        .expect("list resolution jobs");
    assert_eq!(jobs.len(), 1);

    // Negative: a discard WITHOUT a discarded claim fails closed.
    let err = record_conflict_resolution_job(
        &pool,
        &fx.workspace_id,
        &conflict_id,
        ConflictResolutionOutcome::Discard,
        Some(&claim_a),
        None,
        json!({}),
        &receipt.event_id,
    )
    .await
    .expect_err("discard requires a discarded claim");
    assert!(matches!(err, StorageError::Validation(_)), "got {err:?}");

    // Negative: a resolution with no kept claim fails closed.
    let err = record_conflict_resolution_job(
        &pool,
        &fx.workspace_id,
        &conflict_id,
        ConflictResolutionOutcome::Refine,
        None,
        None,
        json!({}),
        &receipt.event_id,
    )
    .await
    .expect_err("resolution requires a kept claim");
    assert!(matches!(err, StorageError::Validation(_)), "got {err:?}");
}
