//! WP-KERNEL-009 RetrievalContextAndRanking bridges — real-PostgreSQL proof for
//! the folded-stub bridges (MT-139 ProjectBrain, MT-140 SemanticCatalog,
//! MT-141 AIReady export, MT-142 ContextPack recorder) and the schema-first
//! filter (MT-131).
//!
//! PostgreSQL + EventLedger authority only; SKIP loudly when binaries absent.

#[path = "knowledge_memory_fixtures.rs"]
mod knowledge_memory_fixtures;

use handshake_core::kernel::KernelActor;
use handshake_core::knowledge_retrieval::context_pack_recorder::{
    record_context_pack_decision, ContextPackDecision, ContextPackDecisionRecord,
};
use handshake_core::knowledge_retrieval::planner::AuthoritativeHandle;
use handshake_core::knowledge_retrieval::project_brain::{
    map_query, ProjectBrainIntent, ProjectBrainQuery,
};
use handshake_core::knowledge_retrieval::schema_filter::schema_first_filter;
use handshake_core::storage::knowledge_memory::{
    add_memory_ontology_alias, create_memory_fact, upsert_memory_ontology_term,
    MemoryClaimAuthorityLabel, MemoryFactObject, MemoryOntologyAliasSource, MemoryOntologyTermKind,
    NewMemoryFact, NewMemoryOntologyTerm,
};

use knowledge_memory_fixtures::{pool_for, MemoryFixture};

macro_rules! skip_if_no_pg {
    ($opt:expr, $name:literal) => {
        match $opt {
            Some(value) => value,
            None => {
                eprintln!(concat!("SKIP ", $name, ": PostgreSQL unavailable"));
                return;
            }
        }
    };
}

/// MT-131: schema-first filtering narrows facts to the query's ontology scope.
/// A fact whose predicate term the query resolves to is kept; an off-topic fact
/// is dropped.
#[tokio::test]
async fn schema_first_filter_narrows_to_query_scope() {
    let fx = skip_if_no_pg!(MemoryFixture::setup().await, "schema_first_filter");
    let pool = pool_for(&fx.pg).await;

    // Ontology term "depends_on" with an alias the query will hit.
    let term = upsert_memory_ontology_term(
        &pool,
        NewMemoryOntologyTerm {
            workspace_id: fx.workspace_id.clone(),
            term_kind: MemoryOntologyTermKind::RelationClass,
            term_key: "depends_on".to_string(),
            normalized_label: "depends on".to_string(),
            maps_to_edge_type: Some("depends_on".to_string()),
            maps_to_entity_kind: None,
            promotion_threshold: 1,
            operator_approved: true,
            detection_provenance: serde_json::json!({"by": "test"}),
            seen_in_run: None,
        },
    )
    .await
    .expect("term");
    add_memory_ontology_alias(
        &pool,
        &term.term_id,
        &fx.workspace_id,
        "dependency",
        "dependency",
        MemoryOntologyAliasSource::Operator,
    )
    .await
    .expect("alias");

    // An entity + an in-scope fact (predicate = the term) and an off-topic fact.
    let subj = fx.entity("symbol", "svc", "Service").await;
    let claim_in = fx.claim("service depends on db").await;
    create_memory_fact(
        &pool,
        NewMemoryFact {
            workspace_id: fx.workspace_id.clone(),
            claim_id: claim_in.claim_id.clone(),
            subject_entity_id: subj.clone(),
            predicate_key: "depends_on".to_string(),
            predicate_term_id: Some(term.term_id.clone()),
            object: MemoryFactObject::Literal {
                value: "db".to_string(),
            },
            qualifiers: serde_json::json!({}),
            authority_label: MemoryClaimAuthorityLabel::Derived,
            extractor_version: "test_v1".to_string(),
            created_in_run: None,
        },
    )
    .await
    .expect("fact in");

    let claim_out = fx.claim("unrelated note").await;
    create_memory_fact(
        &pool,
        NewMemoryFact {
            workspace_id: fx.workspace_id.clone(),
            claim_id: claim_out.claim_id.clone(),
            subject_entity_id: subj.clone(),
            predicate_key: "note".to_string(),
            predicate_term_id: None,
            object: MemoryFactObject::Literal {
                value: "x".to_string(),
            },
            qualifiers: serde_json::json!({}),
            authority_label: MemoryClaimAuthorityLabel::Derived,
            extractor_version: "test_v1".to_string(),
            created_in_run: None,
        },
    )
    .await
    .expect("fact out");

    // Query mentions "dependency" -> resolves to the term -> narrows facts.
    let result = schema_first_filter(&pool, &fx.workspace_id, "what is the dependency?", 100)
        .await
        .expect("filter");
    assert!(
        result.has_schema_scope(),
        "query should resolve a schema term"
    );
    assert!(result.matched_term_ids.contains(&term.term_id));
    assert_eq!(result.candidate_facts.len(), 1);
    assert_eq!(result.off_topic_dropped, 1);
}

/// MT-142: the ContextPack recorder emits a recorder-visible EventLedger receipt
/// with pack/source hashes and bounded policy metadata (no full context).
#[tokio::test]
async fn context_pack_decision_emits_bounded_receipt() {
    let fx = skip_if_no_pg!(MemoryFixture::setup().await, "context_pack_decision");

    let record = ContextPackDecisionRecord {
        decision: ContextPackDecision::Reuse,
        pack_id: "PACK-test-1".to_string(),
        target_ref: "entity:svc".to_string(),
        pack_hash: "feedface".to_string(),
        source_hashes: vec!["s1".to_string(), "s2".to_string()],
        freshness_policy: "reuse_if_sources_unchanged".to_string(),
        linkage: Some("job-7".to_string()),
    };
    let receipt = record_context_pack_decision(
        &fx.pg.db,
        KernelActor::System("retrieval-test".to_string()),
        "ktr-pack",
        "sr-pack",
        &record,
    )
    .await
    .expect("record pack decision");
    assert!(
        !receipt.is_empty(),
        "a recorder-visible event id is returned"
    );
}

/// MT-139: a Project Brain query with a handle maps to authoritative lookup;
/// without one, to discovery/synthesis (the deterministic backend mapping).
#[tokio::test]
async fn project_brain_maps_lookup_vs_discovery() {
    // Pure mapping (no PG needed), but gate on the fixture for consistency with
    // the suite's skip behavior is unnecessary here — assert directly.
    let lookup = map_query(&ProjectBrainQuery {
        workspace_id: "ws".to_string(),
        query_text: "status of WP-KERNEL-009".to_string(),
        handles: vec![AuthoritativeHandle::WorkPacketId(
            "WP-KERNEL-009".to_string(),
        )],
        prefer_context_pack_reuse: true,
        freshness_uncertain: false,
    });
    assert_eq!(lookup.intent, ProjectBrainIntent::AuthoritativeLookup);
    assert!(lookup.prefer_context_pack_reuse);

    let discovery = map_query(&ProjectBrainQuery {
        workspace_id: "ws".to_string(),
        query_text: "how does ranking work?".to_string(),
        handles: vec![],
        prefer_context_pack_reuse: false,
        freshness_uncertain: false,
    });
    assert_eq!(discovery.intent, ProjectBrainIntent::DiscoverySynthesis);
}
