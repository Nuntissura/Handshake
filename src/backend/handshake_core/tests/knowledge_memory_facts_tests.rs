//! WP-KERNEL-009 MemoryGraphAndClaims MT-114 (MemoryFactSchema) and
//! MT-125 (ClaimAuthorityLabels) integration tests against REAL
//! Handshake-managed PostgreSQL.
//!
//! A MemoryFact is a structured subject/predicate/object record backed 1:1 by a
//! knowledge_claims row. These tests prove: a fact round-trips with its backing
//! claim's evidence; a relationship fact (entity object) and an attribute fact
//! (literal object) are both representable and the object-shape CHECK rejects
//! neither-or-both; authority labels enforce a legal transition table (an
//! operator-approved fact cannot silently drop back to model_suggested);
//! deleting the backing claim cascades the fact away (the claim is authority).

mod knowledge_memory_fixtures;

use handshake_core::storage::knowledge_memory::{
    create_memory_fact, get_memory_fact, get_memory_fact_by_claim, list_memory_facts,
    set_memory_fact_authority_label, MemoryClaimAuthorityLabel, MemoryFactObject, NewMemoryFact,
};
use handshake_core::storage::StorageError;
use knowledge_memory_fixtures::{pool_for, MemoryFixture};
use serde_json::json;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn relationship_fact_roundtrip_backed_by_claim() {
    let Some(fx) = MemoryFixture::setup().await else {
        eprintln!("SKIP relationship_fact_roundtrip_backed_by_claim: no PostgreSQL");
        return;
    };
    let pool = pool_for(&fx.pg).await;

    // subject entity, object entity, and a backing claim (with evidence span).
    let subject = fx.entity("symbol", "crate::a::Foo", "Foo").await;
    let object = fx.entity("symbol", "crate::b::Bar", "Bar").await;
    let claim = fx.claim("Foo depends on Bar").await;

    let fact = create_memory_fact(
        &pool,
        NewMemoryFact {
            workspace_id: fx.workspace_id.clone(),
            claim_id: claim.claim_id.clone(),
            subject_entity_id: subject.clone(),
            predicate_key: "depends_on".to_string(),
            predicate_term_id: None,
            object: MemoryFactObject::Entity {
                entity_id: object.clone(),
            },
            qualifiers: json!({"source_system": "code_index"}),
            authority_label: MemoryClaimAuthorityLabel::Source,
            extractor_version: "mem_fact_v1".to_string(),
            created_in_run: None,
        },
    )
    .await
    .expect("create relationship fact");
    assert!(fact.fact_id.starts_with("KMF-"));
    assert_eq!(fact.object_entity_id.as_deref(), Some(object.as_str()));
    assert!(fact.object_literal.is_none());

    let fetched = get_memory_fact(&pool, &fact.fact_id)
        .await
        .expect("get fact")
        .expect("fact exists");
    assert_eq!(fetched, fact);

    let by_claim = get_memory_fact_by_claim(&pool, &claim.claim_id)
        .await
        .expect("get by claim")
        .expect("fact exists");
    assert_eq!(by_claim.fact_id, fact.fact_id);

    // Deleting the backing claim cascades the fact away (claim is authority).
    let mut conn = fx.pg.raw_connection().await;
    sqlx::query("DELETE FROM knowledge_claims WHERE claim_id = $1")
        .bind(&claim.claim_id)
        .execute(&mut conn)
        .await
        .expect("delete backing claim");
    assert!(
        get_memory_fact(&pool, &fact.fact_id)
            .await
            .expect("get fact after claim delete")
            .is_none(),
        "fact must cascade away when its backing claim is deleted"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn attribute_fact_and_object_shape_check() {
    let Some(fx) = MemoryFixture::setup().await else {
        eprintln!("SKIP attribute_fact_and_object_shape_check: no PostgreSQL");
        return;
    };
    let pool = pool_for(&fx.pg).await;
    let subject = fx
        .entity("api", "managed_postgres", "ManagedPostgres")
        .await;
    let claim = fx.claim("ManagedPostgres default port is 5544").await;

    // Attribute fact: literal object.
    let fact = create_memory_fact(
        &pool,
        NewMemoryFact {
            workspace_id: fx.workspace_id.clone(),
            claim_id: claim.claim_id.clone(),
            subject_entity_id: subject.clone(),
            predicate_key: "default_port".to_string(),
            predicate_term_id: None,
            object: MemoryFactObject::Literal {
                value: "5544".to_string(),
            },
            qualifiers: json!({}),
            authority_label: MemoryClaimAuthorityLabel::Derived,
            extractor_version: "mem_fact_v1".to_string(),
            created_in_run: None,
        },
    )
    .await
    .expect("create attribute fact");
    assert_eq!(fact.object_literal.as_deref(), Some("5544"));
    assert!(fact.object_entity_id.is_none());

    // DB object-shape CHECK: a fact with NEITHER object form is rejected.
    let mut conn = fx.pg.raw_connection().await;
    let err = sqlx::query(
        "INSERT INTO knowledge_memory_facts
            (fact_id, workspace_id, claim_id, subject_entity_id, predicate_key,
             extractor_version)
         VALUES ('KMF-00000000000000000000000000000001', $1, $2, $3, 'p', 'v')",
    )
    .bind(&fx.workspace_id)
    .bind(&claim.claim_id)
    .bind(&subject)
    .execute(&mut conn)
    .await
    .expect_err("a fact with no object form must violate the object-shape CHECK");
    assert!(
        err.to_string()
            .contains("chk_knowledge_memory_facts_object_shape"),
        "unexpected: {err}"
    );
}

/// MT-125: authority label transitions enforce a legal table; an
/// operator-approved fact cannot silently drop to a weaker source label.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn authority_label_transition_table_is_enforced() {
    let Some(fx) = MemoryFixture::setup().await else {
        eprintln!("SKIP authority_label_transition_table_is_enforced: no PostgreSQL");
        return;
    };
    let pool = pool_for(&fx.pg).await;
    let subject = fx
        .entity("concept", "retrieval_mode", "RetrievalMode")
        .await;
    let claim = fx.claim("hybrid_rag is the default retrieval mode").await;

    let fact = create_memory_fact(
        &pool,
        NewMemoryFact {
            workspace_id: fx.workspace_id.clone(),
            claim_id: claim.claim_id.clone(),
            subject_entity_id: subject.clone(),
            predicate_key: "default_mode".to_string(),
            predicate_term_id: None,
            object: MemoryFactObject::Literal {
                value: "hybrid_rag".to_string(),
            },
            qualifiers: json!({}),
            authority_label: MemoryClaimAuthorityLabel::ModelSuggested,
            extractor_version: "mem_fact_v1".to_string(),
            created_in_run: None,
        },
    )
    .await
    .expect("create model-suggested fact");

    // model_suggested -> operator_approved is legal (an operator confirmed it).
    let approved = set_memory_fact_authority_label(
        &pool,
        &fact.fact_id,
        MemoryClaimAuthorityLabel::OperatorApproved,
    )
    .await
    .expect("promote to operator_approved");
    assert_eq!(
        approved.authority_label,
        MemoryClaimAuthorityLabel::OperatorApproved
    );

    // operator_approved -> model_suggested is ILLEGAL (no silent downgrade).
    let err = set_memory_fact_authority_label(
        &pool,
        &fact.fact_id,
        MemoryClaimAuthorityLabel::ModelSuggested,
    )
    .await
    .expect_err("operator approval must not silently downgrade");
    assert!(matches!(err, StorageError::Conflict(_)), "got {err:?}");

    // operator_approved -> deprecated IS legal (an explicit retirement).
    let deprecated = set_memory_fact_authority_label(
        &pool,
        &fact.fact_id,
        MemoryClaimAuthorityLabel::Deprecated,
    )
    .await
    .expect("deprecate operator-approved fact");
    assert_eq!(
        deprecated.authority_label,
        MemoryClaimAuthorityLabel::Deprecated
    );

    // The fact still lists in the workspace.
    let facts = list_memory_facts(&pool, &fx.workspace_id, 50)
        .await
        .expect("list facts");
    assert_eq!(facts.len(), 1);
}
