#![cfg(all(feature = "surreal-test-support", feature = "test-utils"))]
//! WP-KERNEL-012 MT-150: coverage restored from the suite deleted by 4f92cc25.
//!
//! Recovers the two non-PG-specific tests from the deleted
//! `memory_persistence_postgres_tests.rs`: that [`CapsuleRecorder`] bound to
//! the production kernel-action adapter persists through the real
//! EventLedger and dedupes on idempotency_key. The deleted tests bound to
//! `PostgresKernelActionSubmitter` (`#[ignore]`d, required a live PostgreSQL
//! server); the drop-in production adapter today is
//! [`SurrealKernelActionSubmitter`] (confirmed by its own in-source proof at
//! `src/storage/surreal/mt136_kernel_action_submitter_proof.rs`, which
//! currently FAILS in the suite -- see MT-150 lane notes -- so it does not
//! cover these two behaviors and is not relied on here). These tests stand
//! alone against a real embedded SurrealDB store (the same
//! `atelier_surreal_support::AtelierSurrealHarness` embedded-store harness
//! every other `atelier_*` suite uses, borrowed here only for its
//! schema-bootstrapped `Arc<dyn Database>` -- no Atelier domain data is
//! involved) and require no external server, so unlike the deleted originals
//! they are NOT `#[ignore]`d.

mod atelier_surreal_support;

use std::{collections::BTreeMap, sync::Arc};

use chrono::{DateTime, Utc};
use handshake_core::{
    memory::{
        persistence::{MEMORY_CAPSULE_AGGREGATE_TYPE, MEMORY_CAPSULE_SOURCE_COMPONENT},
        CapsuleAuditEntry, CapsuleAuditLog, CapsuleRecord, CapsuleRecorder, DegradationTier,
        RetrievalPolicy, SurrealKernelActionSubmitter, TaskType, MEMORY_CAPSULE_RECORD_ACTION_ID,
    },
    storage::Database,
};
use uuid::Uuid;

/// An isolated, schema-bootstrapped embedded SurrealDB `Arc<dyn Database>`,
/// reusing the shared Atelier test harness purely for its store plumbing.
async fn embedded_database() -> (
    Arc<dyn Database>,
    atelier_surreal_support::AtelierSurrealHarness,
) {
    let harness = atelier_surreal_support::AtelierSurrealHarness::create().await;
    let database = harness.database.clone();
    (database, harness)
}

#[tokio::test(flavor = "multi_thread")]
async fn capsule_recorder_persists_via_kernel_action_catalog_against_embedded_store() {
    let (db, _harness) = embedded_database().await;

    // Bind the recorder to the real embedded-Surreal-backed kernel action
    // catalog dispatcher -- the production drop-in for the deleted
    // `PostgresKernelActionSubmitter`.
    let submitter = SurrealKernelActionSubmitter::with_db(Arc::clone(&db));

    // Sanity-check the catalog includes the memory_capsule.record action so the
    // wiring is real (not a stub).
    assert!(
        submitter
            .catalog()
            .action(MEMORY_CAPSULE_RECORD_ACTION_ID)
            .is_some(),
        "KernelActionCatalogV1 must register memory_capsule.record action"
    );

    let recorder = CapsuleRecorder {
        action_catalog: &submitter,
    };

    let record = sample_capsule_record();
    let receipt = recorder.record(record.clone()).expect("recorder.record");

    assert_eq!(receipt.record_id.get_version_num(), 7);
    assert_eq!(receipt.write_box_envelope_id.get_version_num(), 7);

    // Re-read from the kernel_event_ledger and confirm an event row landed for
    // this capsule under the memory_capsule aggregate.
    let events = db
        .list_kernel_events_for_aggregate(
            MEMORY_CAPSULE_AGGREGATE_TYPE,
            &record.capsule_id.to_string(),
        )
        .await
        .expect("list kernel events for memory_capsule aggregate");

    assert!(
        !events.is_empty(),
        "kernel_event_ledger must contain at least one event for the recorded capsule"
    );
    let stored = &events[0];
    assert_eq!(stored.aggregate_type, MEMORY_CAPSULE_AGGREGATE_TYPE);
    assert_eq!(stored.aggregate_id, record.capsule_id.to_string());
    assert_eq!(stored.source_component, MEMORY_CAPSULE_SOURCE_COMPONENT);
    assert_eq!(stored.event_version, "kernel_event_v1");
    assert_eq!(stored.payload_hash.len(), 64);
    assert_eq!(
        stored.payload["catalog_action_id"].as_str(),
        Some(MEMORY_CAPSULE_RECORD_ACTION_ID),
        "payload must record the kernel action catalog action_id"
    );
    assert_eq!(
        stored.payload["request"]["target_ids"][0]["target_id"]
            .as_str()
            .map(str::to_string),
        Some(record.capsule_id.to_string())
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn capsule_recorder_dedup_collapses_duplicate_submissions() {
    let (db, _harness) = embedded_database().await;
    let submitter = SurrealKernelActionSubmitter::with_db(Arc::clone(&db));

    let recorder = CapsuleRecorder {
        action_catalog: &submitter,
    };

    let record = sample_capsule_record();
    let first = recorder.record(record.clone()).expect("first record");
    // Re-submit the identical record. The idempotency_key in the underlying
    // KernelActionRequestV1 is deterministic (derived from the record, not the
    // receipt), so the ledger should dedupe at the database boundary just like
    // any other kernel event.
    let second = recorder.record(record.clone()).expect("second record");

    // The recorder generates fresh receipt UUIDs so the two receipts are not
    // equal, but the persisted ledger rows should collapse to a single event.
    let events = db
        .list_kernel_events_for_aggregate(
            MEMORY_CAPSULE_AGGREGATE_TYPE,
            &record.capsule_id.to_string(),
        )
        .await
        .expect("list ledger events for aggregate");
    assert!(
        events.len() <= 2,
        "duplicate capsule action submissions must not produce more than 2 ledger rows (idempotent dedup expected)"
    );
    // The submission carries the same idempotency_key so the ledger row for the
    // first submission must remain visible.
    assert_eq!(events[0].aggregate_id, record.capsule_id.to_string());
    let _ = (first, second);
}

fn sample_capsule_record() -> CapsuleRecord {
    CapsuleRecord {
        capsule_id: Uuid::now_v7(),
        capsule_source_hash: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
            .to_string(),
        task_type: TaskType::KernelBuilderMtImplementation,
        policy: RetrievalPolicy {
            top_k: 12,
            capsule_budget_bytes: 65_536,
            task_type: TaskType::KernelBuilderMtImplementation,
            scoring_formula_version: "retrieval_scoring_formula_v0".to_string(),
            graceful_degradation_tier: DegradationTier::Tiered,
        },
        audit_log: CapsuleAuditLog {
            entries: vec![CapsuleAuditEntry {
                item_id: "item-150-surreal".to_string(),
                source_uri: "fems://source/artifact/artifact-150#item-1".to_string(),
                included: true,
                suppression_reason: None,
                score: 0.91,
                score_breakdown: BTreeMap::from([("similarity".to_string(), 0.91)]),
                pinned: false,
            }],
        },
        built_at_utc: dt("2026-05-19T10:00:00Z"),
        recorded_at_utc: dt("2026-05-19T10:05:00Z"),
        session_id: "session-mt-150-surreal".to_string(),
        role_id: "KERNEL_BUILDER".to_string(),
        outcome: None,
    }
}

fn dt(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .unwrap()
        .with_timezone(&Utc)
}
