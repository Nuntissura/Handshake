//! WP-KERNEL-009 MemoryGraphAndClaims MT-115 (MemoryPassageSchema read side),
//! MT-116 (OntologyGraphProjection), MT-117 (FactGraphProjection),
//! MT-118 (PassageEvidenceGraphProjection) integration tests against REAL
//! Handshake-managed PostgreSQL.
//!
//! Projections are NEVER authority: every node/edge carries a stable ref back
//! into an authority row, and `authority_class` is "projection". These tests
//! prove each projection is built from authority rows and that trusted-only
//! filtering excludes non-retrieval-trusted facts from the stable fact graph.

mod knowledge_memory_fixtures;

use handshake_core::kernel::{KernelActor, KernelEventType, NewKernelEvent};
use handshake_core::knowledge_memory::passage::{
    list_passages_citing_claim, load_passage_with_evidence,
};
use handshake_core::knowledge_memory::projection::{
    build_fact_graph, build_ontology_graph, build_passage_evidence_graph,
};
use handshake_core::knowledge_memory::visual_debug::{
    build_memory_graph_visual_debug, MEMORY_GRAPH_VISUAL_DEBUG_SCHEMA_ID,
};
use handshake_core::storage::knowledge::{
    KnowledgeClaimState, KnowledgeCompactionPolicy, KnowledgePassageEvidenceRef,
    KnowledgeRetrievalMode, KnowledgeStore, NewKnowledgeMemoryPassage,
};
use handshake_core::storage::knowledge_memory::{
    add_memory_ontology_alias, create_memory_fact, set_memory_fact_authority_label,
    upsert_memory_ontology_term, MemoryClaimAuthorityLabel, MemoryFactObject,
    MemoryOntologyAliasSource, MemoryOntologyTermKind, NewMemoryFact, NewMemoryOntologyTerm,
};
use handshake_core::storage::postgres::PostgresDatabase;
use handshake_core::storage::{Database, StorageError};
use knowledge_memory_fixtures::{pool_for, MemoryFixture};
use serde_json::json;
use uuid::Uuid;

async fn accept_claim(db: &PostgresDatabase, claim_id: &str, label: &str) -> String {
    let suffix = Uuid::now_v7();
    let receipt = db
        .append_kernel_event(
            NewKernelEvent::builder(
                format!("KTR-{label}-{suffix}"),
                format!("SR-{label}-{suffix}"),
                KernelEventType::ValidationRecorded,
                KernelActor::ValidationRunner(label.to_string()),
            )
            .aggregate("knowledge_claim", claim_id.to_string())
            .idempotency_key(format!("idem-{label}-{suffix}"))
            .payload(json!({"accepted_claim": claim_id}))
            .build()
            .expect("event"),
        )
        .await
        .expect("append claim acceptance receipt");
    db.transition_knowledge_claim(
        claim_id,
        KnowledgeClaimState::Accepted,
        None,
        Some(&receipt.event_id),
    )
    .await
    .expect("accept claim");
    receipt.event_id
}

// ---------------------------------------------------------------------------
// MT-116 OntologyGraphProjection
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ontology_graph_projection_includes_terms_aliases_and_class() {
    let Some(fx) = MemoryFixture::setup().await else {
        eprintln!("SKIP ontology_graph_projection_includes_terms_aliases_and_class: no PostgreSQL");
        return;
    };
    let pool = pool_for(&fx.pg).await;

    let term = upsert_memory_ontology_term(
        &pool,
        NewMemoryOntologyTerm {
            workspace_id: fx.workspace_id.clone(),
            term_kind: MemoryOntologyTermKind::RelationClass,
            term_key: "depends_on".to_string(),
            normalized_label: "depends on".to_string(),
            maps_to_edge_type: Some("depends_on".to_string()),
            maps_to_entity_kind: None,
            promotion_threshold: 3,
            operator_approved: false,
            detection_provenance: json!({}),
            seen_in_run: None,
        },
    )
    .await
    .expect("term");
    add_memory_ontology_alias(
        &pool,
        &term.term_id,
        &fx.workspace_id,
        "requires",
        "requires",
        MemoryOntologyAliasSource::Operator,
    )
    .await
    .expect("alias");

    let graph = build_ontology_graph(&pool, &fx.workspace_id, false, 50)
        .await
        .expect("ontology graph");
    assert_eq!(graph.authority_class, "projection");
    assert_eq!(graph.nodes.len(), 1);
    let node = &graph.nodes[0];
    assert_eq!(node.term_id, term.term_id);
    assert_eq!(node.aliases, vec!["requires".to_string()]);
    assert_eq!(node.lifecycle_state, "probationary");

    // stable_only filter excludes the probationary term.
    let stable = build_ontology_graph(&pool, &fx.workspace_id, true, 50)
        .await
        .expect("stable ontology graph");
    assert!(stable.nodes.is_empty(), "no stable terms yet");
}

// ---------------------------------------------------------------------------
// MT-117 FactGraphProjection
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fact_graph_projection_trusted_only_filter() {
    let Some(fx) = MemoryFixture::setup().await else {
        eprintln!("SKIP fact_graph_projection_trusted_only_filter: no PostgreSQL");
        return;
    };
    let pool = pool_for(&fx.pg).await;
    let db = PostgresDatabase::new(pool.clone());

    let subject = fx.entity("symbol", "crate::a::Foo", "Foo").await;
    let object = fx.entity("symbol", "crate::b::Bar", "Bar").await;

    // A SOURCE-labelled (trusted) relationship fact.
    let claim_trusted = fx.claim("Foo depends on Bar (source)").await;
    accept_claim(&db, &claim_trusted.claim_id, "fact-trusted").await;
    create_memory_fact(
        &pool,
        NewMemoryFact {
            workspace_id: fx.workspace_id.clone(),
            claim_id: claim_trusted.claim_id.clone(),
            subject_entity_id: subject.clone(),
            predicate_key: "depends_on".to_string(),
            predicate_term_id: None,
            object: MemoryFactObject::Entity {
                entity_id: object.clone(),
            },
            qualifiers: json!({}),
            authority_label: MemoryClaimAuthorityLabel::Source,
            extractor_version: "v1".to_string(),
            created_in_run: None,
        },
    )
    .await
    .expect("trusted fact");

    // An UNSUPPORTED (untrusted) attribute fact on the same subject.
    let claim_untrusted = fx.claim("Foo has unknown owner (unsupported)").await;
    create_memory_fact(
        &pool,
        NewMemoryFact {
            workspace_id: fx.workspace_id.clone(),
            claim_id: claim_untrusted.claim_id.clone(),
            subject_entity_id: subject.clone(),
            predicate_key: "owner".to_string(),
            predicate_term_id: None,
            object: MemoryFactObject::Literal {
                value: "unknown".to_string(),
            },
            qualifiers: json!({}),
            authority_label: MemoryClaimAuthorityLabel::Unsupported,
            extractor_version: "v1".to_string(),
            created_in_run: None,
        },
    )
    .await
    .expect("untrusted fact");

    // SOURCE labels are not enough for trusted retrieval: conflicting backing
    // claims must stay out of trusted projections until resolved and accepted.
    let claim_conflicted_a = fx.claim("Foo default port 5544 (source)").await;
    let conflicted_fact_a = create_memory_fact(
        &pool,
        NewMemoryFact {
            workspace_id: fx.workspace_id.clone(),
            claim_id: claim_conflicted_a.claim_id.clone(),
            subject_entity_id: subject.clone(),
            predicate_key: "default_port".to_string(),
            predicate_term_id: None,
            object: MemoryFactObject::Literal {
                value: "5544".to_string(),
            },
            qualifiers: json!({}),
            authority_label: MemoryClaimAuthorityLabel::Source,
            extractor_version: "v1".to_string(),
            created_in_run: None,
        },
    )
    .await
    .expect("conflicted fact a");
    let claim_conflicted_b = fx.claim("Foo default port 5432 (source)").await;
    let conflicted_fact_b = create_memory_fact(
        &pool,
        NewMemoryFact {
            workspace_id: fx.workspace_id.clone(),
            claim_id: claim_conflicted_b.claim_id.clone(),
            subject_entity_id: subject.clone(),
            predicate_key: "default_port".to_string(),
            predicate_term_id: None,
            object: MemoryFactObject::Literal {
                value: "5432".to_string(),
            },
            qualifiers: json!({}),
            authority_label: MemoryClaimAuthorityLabel::Source,
            extractor_version: "v1".to_string(),
            created_in_run: None,
        },
    )
    .await
    .expect("conflicted fact b");
    db.record_knowledge_claim_conflict(
        &claim_conflicted_a.claim_id,
        &claim_conflicted_b.claim_id,
        "MT-231 trusted projection contradiction fixture",
        None,
    )
    .await
    .expect("record source-labelled conflict");

    // trusted_only=false: all facts present (4 edges).
    let all = build_fact_graph(&db, &pool, &fx.workspace_id, false, 50)
        .await
        .expect("all fact graph");
    assert_eq!(all.edges.len(), 4);
    assert_eq!(all.authority_class, "projection");

    // trusted_only=true: only the accepted SOURCE fact survives (1 edge).
    let trusted = build_fact_graph(&db, &pool, &fx.workspace_id, true, 50)
        .await
        .expect("trusted fact graph");
    assert_eq!(trusted.edges.len(), 1);
    assert_eq!(trusted.edges[0].authority_label, "source");
    assert!(
        !trusted
            .edges
            .iter()
            .any(|edge| edge.fact_id == conflicted_fact_a.fact_id
                || edge.fact_id == conflicted_fact_b.fact_id),
        "conflicted facts must not leak into trusted projections"
    );
    // The trusted edge is the relationship fact; its endpoints are both nodes.
    assert_eq!(trusted.edges[0].subject_entity_id, subject);
    assert_eq!(
        trusted.edges[0].object_entity_id.as_deref(),
        Some(object.as_str())
    );
    assert!(trusted.nodes.iter().any(|n| n.entity_id == subject));
    assert!(trusted.nodes.iter().any(|n| n.entity_id == object));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fact_graph_trusted_only_blocks_raw_sql_unresolved_conflicts() {
    let Some(fx) = MemoryFixture::setup().await else {
        eprintln!(
            "SKIP fact_graph_trusted_only_blocks_raw_sql_unresolved_conflicts: no PostgreSQL"
        );
        return;
    };
    let pool = pool_for(&fx.pg).await;
    let db = PostgresDatabase::new(pool.clone());

    let subject = fx.entity("symbol", "crate::raw::Server", "Server").await;

    let claim_a = fx.claim("Server default port is 5544 (source)").await;
    accept_claim(&db, &claim_a.claim_id, "raw-conflict-a").await;
    let fact_a = create_memory_fact(
        &pool,
        NewMemoryFact {
            workspace_id: fx.workspace_id.clone(),
            claim_id: claim_a.claim_id.clone(),
            subject_entity_id: subject.clone(),
            predicate_key: "default_port".to_string(),
            predicate_term_id: None,
            object: MemoryFactObject::Literal {
                value: "5544".to_string(),
            },
            qualifiers: json!({}),
            authority_label: MemoryClaimAuthorityLabel::Source,
            extractor_version: "v1".to_string(),
            created_in_run: None,
        },
    )
    .await
    .expect("raw conflict fact a");

    let claim_b = fx.claim("Server default port is 5432 (source)").await;
    accept_claim(&db, &claim_b.claim_id, "raw-conflict-b").await;
    let fact_b = create_memory_fact(
        &pool,
        NewMemoryFact {
            workspace_id: fx.workspace_id.clone(),
            claim_id: claim_b.claim_id.clone(),
            subject_entity_id: subject.clone(),
            predicate_key: "default_port".to_string(),
            predicate_term_id: None,
            object: MemoryFactObject::Literal {
                value: "5432".to_string(),
            },
            qualifiers: json!({}),
            authority_label: MemoryClaimAuthorityLabel::Source,
            extractor_version: "v1".to_string(),
            created_in_run: None,
        },
    )
    .await
    .expect("raw conflict fact b");

    let conflict_id = format!("KCC-{}", Uuid::now_v7().simple());
    let mut conn = fx.pg.raw_connection().await;
    sqlx::query(
        r#"
        INSERT INTO knowledge_claim_conflicts
            (conflict_id, claim_id, conflicting_claim_id, conflict_reason)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(&conflict_id)
    .bind(&claim_a.claim_id)
    .bind(&claim_b.claim_id)
    .bind("raw SQL MT-231 unresolved contradiction")
    .execute(&mut conn)
    .await
    .expect("raw conflict insert");

    for claim_id in [&claim_a.claim_id, &claim_b.claim_id] {
        let claim = db
            .get_knowledge_claim(claim_id)
            .await
            .expect("claim lookup")
            .expect("claim exists");
        assert_eq!(
            claim.lifecycle_state,
            KnowledgeClaimState::Conflicted,
            "raw conflict insertion must move accepted claims back to conflicted"
        );
    }

    let trusted = build_fact_graph(&db, &pool, &fx.workspace_id, true, 50)
        .await
        .expect("trusted fact graph");
    assert!(
        !trusted
            .edges
            .iter()
            .any(|edge| edge.fact_id == fact_a.fact_id || edge.fact_id == fact_b.fact_id),
        "raw unresolved conflicts must not leak accepted facts into trusted projections"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unsupported_fact_cannot_be_relabeled_into_trusted_stable_graph() {
    let Some(fx) = MemoryFixture::setup().await else {
        eprintln!(
            "SKIP unsupported_fact_cannot_be_relabeled_into_trusted_stable_graph: no PostgreSQL"
        );
        return;
    };
    let pool = pool_for(&fx.pg).await;
    let db = PostgresDatabase::new(pool.clone());

    let subject = fx
        .entity("symbol", "crate::unsupported::Rumor", "Rumor")
        .await;
    let claim = fx
        .claim("Rumor has a stable owner but the citation is unsupported")
        .await;
    accept_claim(&db, &claim.claim_id, "unsupported-stable-claim").await;
    let fact = create_memory_fact(
        &pool,
        NewMemoryFact {
            workspace_id: fx.workspace_id.clone(),
            claim_id: claim.claim_id.clone(),
            subject_entity_id: subject,
            predicate_key: "owner".to_string(),
            predicate_term_id: None,
            object: MemoryFactObject::Literal {
                value: "unknown".to_string(),
            },
            qualifiers: json!({}),
            authority_label: MemoryClaimAuthorityLabel::Unsupported,
            extractor_version: "mt233_bad_citation_fixture_v1".to_string(),
            created_in_run: None,
        },
    )
    .await
    .expect("create unsupported fact");

    let trusted = build_fact_graph(&db, &pool, &fx.workspace_id, true, 50)
        .await
        .expect("trusted graph before relabel");
    assert!(
        !trusted
            .edges
            .iter()
            .any(|edge| edge.fact_id == fact.fact_id),
        "unsupported facts must start outside trusted stable projections"
    );

    let api_err = set_memory_fact_authority_label(
        &pool,
        &fact.fact_id,
        MemoryClaimAuthorityLabel::OperatorApproved,
    )
    .await
    .expect_err("unsupported fact must not become operator-approved without re-grounding");
    assert!(
        matches!(api_err, StorageError::Conflict(_)),
        "unexpected API error: {api_err:?}"
    );

    let mut conn = fx.pg.raw_connection().await;
    let raw_err = sqlx::query(
        r#"
        UPDATE knowledge_memory_facts
           SET authority_label = 'source', updated_at = NOW()
         WHERE fact_id = $1
        "#,
    )
    .bind(&fact.fact_id)
    .execute(&mut conn)
    .await
    .expect_err("raw SQL must not relabel unsupported facts into trusted labels");
    assert!(
        raw_err
            .to_string()
            .contains("unsupported facts cannot become retrieval-trusted"),
        "unexpected raw SQL error: {raw_err}"
    );

    let trusted_after = build_fact_graph(&db, &pool, &fx.workspace_id, true, 50)
        .await
        .expect("trusted graph after rejected relabel");
    assert!(
        !trusted_after
            .edges
            .iter()
            .any(|edge| edge.fact_id == fact.fact_id),
        "failed relabel attempts must not leak unsupported facts into stable projections"
    );
}

// ---------------------------------------------------------------------------
// MT-115 + MT-118 PassageEvidenceGraphProjection
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn passage_evidence_graph_and_citation_resolution() {
    let Some(fx) = MemoryFixture::setup().await else {
        eprintln!("SKIP passage_evidence_graph_and_citation_resolution: no PostgreSQL");
        return;
    };
    let pool = pool_for(&fx.pg).await;
    let db = PostgresDatabase::new(pool.clone());

    // A claim that backs a fact, then a passage that cites that claim + the span.
    let subject = fx
        .entity("api", "managed_postgres", "ManagedPostgres")
        .await;
    let claim = fx.claim("managed PG default port is 5544").await;
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
            authority_label: MemoryClaimAuthorityLabel::Source,
            extractor_version: "v1".to_string(),
            created_in_run: None,
        },
    )
    .await
    .expect("fact");

    let passage = db
        .create_knowledge_memory_passage(NewKnowledgeMemoryPassage {
            workspace_id: fx.workspace_id.clone(),
            passage_text: "The managed PostgreSQL cluster listens on port 5544.".to_string(),
            token_count: Some(10),
            ocr_transcript_metadata: None,
            extraction_confidence: 0.95,
            ranking_features: json!({"recency_score": 0.9}),
            retrieval_mode: KnowledgeRetrievalMode::HybridRag,
            compaction_policy: KnowledgeCompactionPolicy::Keep,
            failure_receipt_event_id: None,
            derived_in_run: None,
            evidence: vec![
                KnowledgePassageEvidenceRef::Claim {
                    claim_id: claim.claim_id.clone(),
                },
                KnowledgePassageEvidenceRef::Span {
                    span_id: fx.span_id.clone(),
                },
            ],
        })
        .await
        .expect("passage");

    // MT-118: the evidence graph has the passage node and its 2 evidence edges.
    let graph = build_passage_evidence_graph(&db, &pool, &fx.workspace_id, 50)
        .await
        .expect("passage evidence graph");
    assert_eq!(graph.authority_class, "projection");
    assert_eq!(graph.nodes.len(), 1);
    assert_eq!(graph.nodes[0].passage_id, passage.passage_id);
    assert_eq!(graph.edges.len(), 2, "claim + span evidence edges");
    assert!(graph
        .edges
        .iter()
        .any(|e| e.ref_kind == "claim" && e.target_id == claim.claim_id));
    assert!(graph
        .edges
        .iter()
        .any(|e| e.ref_kind == "span" && e.target_id == fx.span_id));

    // MT-115: loading the passage resolves the facts it cites (through claims).
    let with_evidence = load_passage_with_evidence(&db, &pool, &passage.passage_id)
        .await
        .expect("load passage with evidence")
        .expect("passage exists");
    assert_eq!(with_evidence.cited_facts.len(), 1);
    assert_eq!(with_evidence.cited_facts[0].fact_id, fact.fact_id);

    // MT-115 reverse: the passage shows up as citing the claim.
    let citing = list_passages_citing_claim(&pool, &claim.claim_id)
        .await
        .expect("citing passages");
    assert_eq!(citing, vec![passage.passage_id]);
}

// ---------------------------------------------------------------------------
// MT-127 MemoryGraphVisualDebugPayload
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn visual_debug_payload_composes_graphs_and_counts() {
    let Some(fx) = MemoryFixture::setup().await else {
        eprintln!("SKIP visual_debug_payload_composes_graphs_and_counts: no PostgreSQL");
        return;
    };
    let pool = pool_for(&fx.pg).await;
    let db = PostgresDatabase::new(pool.clone());

    // One ontology term, one source fact, one passage citing the claim.
    upsert_memory_ontology_term(
        &pool,
        NewMemoryOntologyTerm {
            workspace_id: fx.workspace_id.clone(),
            term_kind: MemoryOntologyTermKind::RelationClass,
            term_key: "depends_on".to_string(),
            normalized_label: "depends on".to_string(),
            maps_to_edge_type: Some("depends_on".to_string()),
            maps_to_entity_kind: None,
            promotion_threshold: 3,
            operator_approved: false,
            detection_provenance: json!({}),
            seen_in_run: None,
        },
    )
    .await
    .expect("term");
    let subject = fx.entity("symbol", "crate::a::Foo", "Foo").await;
    let object = fx.entity("symbol", "crate::b::Bar", "Bar").await;
    let claim = fx.claim("Foo depends on Bar").await;
    create_memory_fact(
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
            qualifiers: json!({}),
            authority_label: MemoryClaimAuthorityLabel::Source,
            extractor_version: "v1".to_string(),
            created_in_run: None,
        },
    )
    .await
    .expect("fact");
    db.create_knowledge_memory_passage(NewKnowledgeMemoryPassage {
        workspace_id: fx.workspace_id.clone(),
        passage_text: "Foo depends on Bar.".to_string(),
        token_count: Some(5),
        ocr_transcript_metadata: None,
        extraction_confidence: 1.0,
        ranking_features: json!({}),
        retrieval_mode: KnowledgeRetrievalMode::DirectLoad,
        compaction_policy: KnowledgeCompactionPolicy::Keep,
        failure_receipt_event_id: None,
        derived_in_run: None,
        evidence: vec![KnowledgePassageEvidenceRef::Claim {
            claim_id: claim.claim_id.clone(),
        }],
    })
    .await
    .expect("passage");

    let payload = build_memory_graph_visual_debug(&db, &pool, &fx.workspace_id, false, 50)
        .await
        .expect("visual debug payload");

    assert_eq!(payload.schema_id, MEMORY_GRAPH_VISUAL_DEBUG_SCHEMA_ID);
    assert_eq!(payload.authority_class, "projection");
    // All three subgraphs are populated and each is itself a projection.
    assert_eq!(payload.ontology_graph.nodes.len(), 1);
    assert_eq!(payload.ontology_graph.authority_class, "projection");
    assert_eq!(payload.fact_graph.edges.len(), 1);
    assert_eq!(payload.passage_evidence_graph.nodes.len(), 1);
    // Counts: one proposed claim, one source fact, no open conflicts.
    assert_eq!(payload.claim_state_counts.proposed, 1);
    assert_eq!(payload.fact_label_counts.source, 1);
    assert_eq!(payload.open_conflict_count, 0);
}
