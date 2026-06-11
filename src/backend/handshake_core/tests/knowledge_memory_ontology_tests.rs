//! WP-KERNEL-009 MemoryGraphAndClaims MT-113 (MemoryOntologySchema),
//! MT-119 (ProbationaryExtractionLifecycle), MT-120 (StableSchemaPromotionRule)
//! integration tests against REAL Handshake-managed PostgreSQL.
//!
//! Proof path: the ontology layer's promotion lifecycle and alias resolution
//! are exercised end-to-end on the managed cluster (auto-discovered via
//! `knowledge_pg`), including the negative paths where promotion authority and
//! the lifecycle transition guard are enforced at the DB layer.

mod knowledge_pg_support;

use handshake_core::kernel::{KernelActor, KernelEventType, NewKernelEvent};
use handshake_core::storage::knowledge_memory::{
    add_memory_ontology_alias, get_memory_ontology_term, list_memory_ontology_aliases,
    list_memory_ontology_terms, promote_memory_ontology_term, resolve_memory_ontology_alias,
    retire_memory_ontology_term, upsert_memory_ontology_term, MemoryOntologyAliasSource,
    MemoryOntologyLifecycle, MemoryOntologyRetirementReason, MemoryOntologyTermKind,
    NewMemoryOntologyTerm,
};
use handshake_core::storage::{Database, StorageError};
use knowledge_pg_support::{knowledge_pg, KnowledgePg};
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use uuid::Uuid;

/// Open a pool pinned to the test's isolated schema (the managed-PG
/// auto-discovery path; storage free-functions take `&PgPool`).
async fn pool_for(pg: &KnowledgePg) -> PgPool {
    PgPoolOptions::new()
        .max_connections(4)
        .connect(&pg.schema_url)
        .await
        .expect("open pool into isolated knowledge schema")
}

fn new_term(workspace_id: &str, key: &str, threshold: i32) -> NewMemoryOntologyTerm {
    NewMemoryOntologyTerm {
        workspace_id: workspace_id.to_string(),
        term_kind: MemoryOntologyTermKind::RelationClass,
        term_key: key.to_string(),
        normalized_label: key.to_string(),
        maps_to_edge_type: Some("depends_on".to_string()),
        maps_to_entity_kind: None,
        promotion_threshold: threshold,
        operator_approved: false,
        detection_provenance: json!({"extractor": "ontology_test", "extractor_version": "v1"}),
        seen_in_run: None,
    }
}

/// MT-113 + MT-119 + MT-120: an ontology term starts probationary, accrues
/// observations on re-upsert, becomes promotable at threshold, and only
/// promotes to stable with an EventLedger receipt.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ontology_term_probationary_then_promoted_with_receipt() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP ontology_term_probationary_then_promoted_with_receipt: no PostgreSQL");
        return;
    };
    let pool = pool_for(&pg).await;
    let workspace_id = pg.create_workspace().await;

    // First sighting: probationary, observation_count = 1, threshold 3.
    let term = upsert_memory_ontology_term(&pool, new_term(&workspace_id, "depends_on", 3))
        .await
        .expect("upsert term");
    assert!(term.term_id.starts_with("KMO-"));
    assert_eq!(term.lifecycle_state, MemoryOntologyLifecycle::Probationary);
    assert_eq!(term.observation_count, 1);
    assert!(!term.is_promotable(), "1 < threshold 3");

    // Two more sightings on the same identity accrue observations in place.
    for _ in 0..2 {
        let again = upsert_memory_ontology_term(&pool, new_term(&workspace_id, "depends_on", 3))
            .await
            .expect("re-upsert term");
        assert_eq!(
            again.term_id, term.term_id,
            "re-upsert keeps the stable identity"
        );
    }
    let promotable = get_memory_ontology_term(&pool, &term.term_id)
        .await
        .expect("get term")
        .expect("term exists");
    assert_eq!(promotable.observation_count, 3);
    assert!(promotable.is_promotable(), "3 >= threshold 3");

    // Promotion needs a real EventLedger receipt.
    let suffix = Uuid::now_v7();
    let receipt = pg
        .db
        .append_kernel_event(
            NewKernelEvent::builder(
                format!("KTR-ONTO-{suffix}"),
                format!("SR-ONTO-{suffix}"),
                KernelEventType::ValidationRecorded,
                KernelActor::PromotionGate("ontology-test".to_string()),
            )
            .aggregate("knowledge_memory_ontology_term", term.term_id.clone())
            .idempotency_key(format!("idem-onto-promote-{suffix}"))
            .payload(json!({"verdict": "promote"}))
            .build()
            .expect("event"),
        )
        .await
        .expect("append receipt");

    let stable = promote_memory_ontology_term(&pool, &term.term_id, &receipt.event_id)
        .await
        .expect("promote term");
    assert_eq!(stable.lifecycle_state, MemoryOntologyLifecycle::Stable);
    assert_eq!(
        stable.promotion_receipt_event_id.as_deref(),
        Some(receipt.event_id.as_str())
    );
}

/// MT-120 negative: a probationary term that has NOT met its threshold (and is
/// not operator-approved) cannot be promoted.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn under_threshold_term_cannot_be_promoted() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP under_threshold_term_cannot_be_promoted: no PostgreSQL");
        return;
    };
    let pool = pool_for(&pg).await;
    let workspace_id = pg.create_workspace().await;

    // threshold 5, only 1 observation, not operator-approved.
    let term = upsert_memory_ontology_term(&pool, new_term(&workspace_id, "implements", 5))
        .await
        .expect("upsert term");

    let suffix = Uuid::now_v7();
    let receipt = pg
        .db
        .append_kernel_event(
            NewKernelEvent::builder(
                format!("KTR-UND-{suffix}"),
                format!("SR-UND-{suffix}"),
                KernelEventType::ValidationRecorded,
                KernelActor::PromotionGate("under-test".to_string()),
            )
            .aggregate("knowledge_memory_ontology_term", term.term_id.clone())
            .idempotency_key(format!("idem-under-{suffix}"))
            .payload(json!({"verdict": "promote"}))
            .build()
            .expect("event"),
        )
        .await
        .expect("append receipt");

    let err = promote_memory_ontology_term(&pool, &term.term_id, &receipt.event_id)
        .await
        .expect_err("under-threshold promotion must be rejected");
    assert!(matches!(err, StorageError::Validation(_)), "got {err:?}");

    // The DB itself refuses a stable row without a receipt (raw SQL attack).
    let mut conn = pg.raw_connection().await;
    let err = sqlx::query(
        "UPDATE knowledge_memory_ontology_terms SET lifecycle_state='stable' WHERE term_id=$1",
    )
    .bind(&term.term_id)
    .execute(&mut conn)
    .await
    .expect_err("DB must refuse stable without a promotion receipt");
    assert!(
        err.to_string()
            .contains("MUST carry a promotion_receipt_event_id"),
        "unexpected: {err}"
    );
}

/// MT-119 negative: the lifecycle transition guard refuses an illegal raw-SQL
/// transition (stable -> probationary backward jump) and resurrection of a
/// retired term.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ontology_lifecycle_guard_blocks_illegal_transitions() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP ontology_lifecycle_guard_blocks_illegal_transitions: no PostgreSQL");
        return;
    };
    let pool = pool_for(&pg).await;
    let workspace_id = pg.create_workspace().await;

    // Operator-approved term promotes immediately (threshold bypass).
    let mut payload = new_term(&workspace_id, "validates", 3);
    payload.operator_approved = true;
    let term = upsert_memory_ontology_term(&pool, payload)
        .await
        .expect("upsert term");
    assert!(term.is_promotable(), "operator approval bypasses threshold");

    let suffix = Uuid::now_v7();
    let receipt = pg
        .db
        .append_kernel_event(
            NewKernelEvent::builder(
                format!("KTR-GRD-{suffix}"),
                format!("SR-GRD-{suffix}"),
                KernelEventType::ValidationRecorded,
                KernelActor::PromotionGate("guard-test".to_string()),
            )
            .aggregate("knowledge_memory_ontology_term", term.term_id.clone())
            .idempotency_key(format!("idem-grd-{suffix}"))
            .payload(json!({"verdict": "promote"}))
            .build()
            .expect("event"),
        )
        .await
        .expect("append receipt");
    let stable = promote_memory_ontology_term(&pool, &term.term_id, &receipt.event_id)
        .await
        .expect("promote");
    assert_eq!(stable.lifecycle_state, MemoryOntologyLifecycle::Stable);

    let mut conn = pg.raw_connection().await;
    // stable -> probationary is not a legal transition.
    let err = sqlx::query("UPDATE knowledge_memory_ontology_terms SET lifecycle_state='probationary' WHERE term_id=$1")
        .bind(&term.term_id)
        .execute(&mut conn)
        .await
        .expect_err("stable -> probationary must be refused");
    assert!(
        err.to_string().contains("illegal lifecycle transition"),
        "unexpected: {err}"
    );

    // Retire it, then attempt resurrection.
    let retired = retire_memory_ontology_term(
        &pool,
        &term.term_id,
        MemoryOntologyRetirementReason::OperatorRetired,
        None,
    )
    .await
    .expect("retire");
    assert_eq!(retired.lifecycle_state, MemoryOntologyLifecycle::Retired);
    let err = sqlx::query(
        "UPDATE knowledge_memory_ontology_terms SET lifecycle_state='stable' WHERE term_id=$1",
    )
    .bind(&term.term_id)
    .execute(&mut conn)
    .await
    .expect_err("retired is terminal");
    assert!(
        err.to_string().contains("retired is terminal"),
        "unexpected: {err}"
    );
}

/// MT-113 aliases: alternate spellings resolve to one canonical term, and a
/// duplicate normalized key across the workspace is rejected (no ambiguous
/// alias graph).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ontology_aliases_resolve_to_canonical_term() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP ontology_aliases_resolve_to_canonical_term: no PostgreSQL");
        return;
    };
    let pool = pool_for(&pg).await;
    let workspace_id = pg.create_workspace().await;

    let term = upsert_memory_ontology_term(&pool, new_term(&workspace_id, "depends_on", 3))
        .await
        .expect("upsert term");

    add_memory_ontology_alias(
        &pool,
        &term.term_id,
        &workspace_id,
        "dependsOn",
        "dependson",
        MemoryOntologyAliasSource::Extraction,
    )
    .await
    .expect("alias 1");
    add_memory_ontology_alias(
        &pool,
        &term.term_id,
        &workspace_id,
        "requires",
        "requires",
        MemoryOntologyAliasSource::Operator,
    )
    .await
    .expect("alias 2");

    let resolved = resolve_memory_ontology_alias(&pool, &workspace_id, "dependson")
        .await
        .expect("resolve")
        .expect("alias resolves");
    assert_eq!(resolved.term_id, term.term_id);

    let aliases = list_memory_ontology_aliases(&pool, &term.term_id)
        .await
        .expect("list aliases");
    assert_eq!(aliases.len(), 2);

    // Duplicate normalized key in the same workspace is rejected.
    let err = add_memory_ontology_alias(
        &pool,
        &term.term_id,
        &workspace_id,
        "DependsOn",
        "dependson",
        MemoryOntologyAliasSource::Import,
    )
    .await
    .expect_err("duplicate alias norm key must violate the unique constraint");
    assert!(
        err.to_string()
            .contains("uq_knowledge_memory_ontology_aliases_norm"),
        "unexpected: {err}"
    );

    // Filtered listing: only probationary terms exist so far.
    let probationary = list_memory_ontology_terms(
        &pool,
        &workspace_id,
        Some(MemoryOntologyTermKind::RelationClass),
        Some(MemoryOntologyLifecycle::Probationary),
        50,
    )
    .await
    .expect("list probationary");
    assert_eq!(probationary.len(), 1);
}
