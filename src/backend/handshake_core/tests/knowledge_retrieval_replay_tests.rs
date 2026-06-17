//! MT-238 real-PostgreSQL replay proof for persisted context bundles.
//!
//! A replay starts from a stored RetrievalTrace and must reconstruct the
//! ContextBundle, QueryPlan, selected evidence rows, source/span anchors, and
//! EventLedger receipts from durable PostgreSQL state.

#[path = "knowledge_memory_fixtures.rs"]
mod knowledge_memory_fixtures;

use handshake_core::kernel::KernelEventType;
use handshake_core::knowledge_retrieval::budget::PriorityTier;
use handshake_core::knowledge_retrieval::compiler::{
    BundleCandidate, BundleTargetKind, CompiledBundle, ContextBundleCompilerV2,
};
use handshake_core::knowledge_retrieval::executor::execute_retrieval;
use handshake_core::knowledge_retrieval::graph_planner::GraphTraversalPolicy;
use handshake_core::knowledge_retrieval::plan::RetrievalTrace;
use handshake_core::knowledge_retrieval::planner::{
    AuthoritativeHandle, CheapestAuthoritativePathPlanner, RetrievalRequest,
};
use handshake_core::knowledge_retrieval::snippet::assemble_span_snippet;
use handshake_core::storage::knowledge::{
    KnowledgeBundleItemDecision, KnowledgeBundleItemRefKind, KnowledgeCompactionPolicy,
    KnowledgeContextBundleItem, KnowledgeEdgeType, KnowledgePassageEvidenceRef,
    KnowledgeRetrievalMode, KnowledgeRetrievalTrace, KnowledgeStore, NewKnowledgeEdge,
    NewKnowledgeMemoryPassage,
};
use handshake_core::storage::knowledge_retrieval::{
    replay_context_bundle_from_trace, traces_for_bundle,
};
use sqlx::PgPool;
use std::collections::BTreeSet;

use knowledge_memory_fixtures::{pool_for, MemoryFixture};

macro_rules! skip_if_no_pg {
    ($opt:expr, $name:literal) => {
        match $opt {
            Some(value) => value,
            None => {
                panic!(concat!(
                    "PostgreSQL unavailable for required MT-238 replay proof: ",
                    $name
                ));
            }
        }
    };
}

struct SpanReplaySetup {
    fx: MemoryFixture,
    compiled: CompiledBundle,
    stored_trace: KnowledgeRetrievalTrace,
    pool: PgPool,
    unsupported_span_id: String,
}

async fn span_replay_setup(name: &str) -> Option<SpanReplaySetup> {
    let fx = MemoryFixture::setup().await?;

    let entity_id = fx
        .entity("symbol", &format!("mt238_target_{name}"), "Mt238Target")
        .await;
    let planner = CheapestAuthoritativePathPlanner::new(&fx.pg.db);
    let request = RetrievalRequest::discovery(&fx.workspace_id, "replay MT-238 bundle")
        .with_handle(AuthoritativeHandle::EntityId(entity_id.clone()));
    let planned = planner.plan(&request).await.expect("plan");
    let mut trace = RetrievalTrace::for_plan(&planned.plan);
    let snippet = assemble_span_snippet(&fx.pg.db, &fx.span_id)
        .await
        .expect("snippet");
    let unsupported_span_id = "KSP-00000000000000000000000000000000".to_string();
    let unsupported_snippet = assemble_span_snippet(&fx.pg.db, &unsupported_span_id)
        .await
        .expect("unsupported snippet");
    assert!(
        !unsupported_snippet.supported,
        "missing-span citation must produce typed unsupported evidence"
    );

    let compiled = ContextBundleCompilerV2::new(&fx.pg.db)
        .compile(
            &fx.workspace_id,
            "ktr-mt238-replay",
            "sr-mt238-replay",
            BundleTargetKind::Symbol,
            &entity_id,
            &planned.plan,
            &mut trace,
            &[
                BundleCandidate {
                    ref_kind: KnowledgeBundleItemRefKind::Span,
                    ref_id: fx.span_id.clone(),
                    tier: PriorityTier::Authoritative,
                    token_count: 12,
                    relevance_score: 0.88,
                    source_id: fx.source_id.clone(),
                    snippet: Some(snippet),
                },
                BundleCandidate {
                    ref_kind: KnowledgeBundleItemRefKind::Span,
                    ref_id: unsupported_span_id.clone(),
                    tier: PriorityTier::Authoritative,
                    token_count: 8,
                    relevance_score: 0.8,
                    source_id: "ghost-source".to_string(),
                    snippet: Some(unsupported_snippet),
                },
            ],
            None,
            None,
        )
        .await
        .expect("compile with generated receipts");

    let stored_trace = traces_for_bundle(&fx.pg.db, &compiled.bundle_id)
        .await
        .expect("traces")
        .pop()
        .expect("trace");
    let pool = pool_for(&fx.pg).await;

    Some(SpanReplaySetup {
        fx,
        compiled,
        stored_trace,
        pool,
        unsupported_span_id,
    })
}

#[tokio::test]
async fn replay_context_bundle_from_trace_reconstructs_plan_evidence_and_receipts() {
    let setup = skip_if_no_pg!(
        span_replay_setup("positive").await,
        "replay_context_bundle_from_trace"
    );
    let replay = replay_context_bundle_from_trace(&setup.fx.pg.db, &setup.stored_trace.trace_id)
        .await
        .expect("replay");

    assert_eq!(replay.bundle.bundle_id, setup.compiled.bundle_id);
    assert_eq!(
        replay.query_plan.plan_id,
        replay.retrieval_trace.query_plan_id
    );
    assert_eq!(replay.items.len(), 2);
    assert!(replay
        .items
        .iter()
        .any(|item| item.ref_id == setup.fx.span_id));
    assert!(replay
        .items
        .iter()
        .any(|item| item.ref_id == setup.unsupported_span_id && !item.supported));
    assert_eq!(replay.evidence.len(), 2);

    let supported_evidence = replay
        .evidence
        .iter()
        .find(|evidence| evidence.ref_id == setup.fx.span_id)
        .expect("supported evidence");
    assert_eq!(
        supported_evidence.span_id.as_deref(),
        Some(setup.fx.span_id.as_str())
    );
    assert_eq!(
        supported_evidence.source_id.as_deref(),
        Some(setup.fx.source_id.as_str())
    );
    assert_eq!(
        supported_evidence.source_path.as_deref(),
        Some("memory/graph.rs")
    );
    assert_eq!(
        supported_evidence.retrieval_decision,
        KnowledgeBundleItemDecision::Included
    );

    let unsupported_evidence = replay
        .evidence
        .iter()
        .find(|evidence| evidence.ref_id == setup.unsupported_span_id)
        .expect("unsupported evidence");
    assert!(!unsupported_evidence.supported);
    assert_eq!(
        unsupported_evidence.unsupported_reason.as_deref(),
        Some("span not found in index")
    );
    assert_eq!(unsupported_evidence.span_id, None);
    assert_eq!(unsupported_evidence.source_id, None);
    assert_eq!(
        unsupported_evidence.retrieval_decision,
        KnowledgeBundleItemDecision::Included
    );

    assert_eq!(replay.receipt_events.len(), 2);
    let build_receipt = replay
        .receipt_events
        .iter()
        .find(|event| event.event_type == KernelEventType::ContextBundleRecorded.as_str())
        .expect("build receipt");
    assert_eq!(build_receipt.aggregate_type, "context_bundle");
    assert_eq!(build_receipt.aggregate_id, setup.compiled.bundle_id);
    let trace_receipt = replay
        .receipt_events
        .iter()
        .find(|event| event.event_type == KernelEventType::KnowledgeRetrievalTraceRecorded.as_str())
        .expect("trace receipt");
    assert_eq!(trace_receipt.aggregate_type, "knowledge_retrieval_trace");
    assert_eq!(trace_receipt.aggregate_id, setup.stored_trace.trace_id);
    for receipt in &replay.receipt_events {
        assert_eq!(receipt.kernel_task_run_id, "ktr-mt238-replay");
        assert_eq!(receipt.session_run_id, "sr-mt238-replay");
    }

    sqlx::query("UPDATE kernel_event_ledger SET event_type = $1 WHERE event_id = $2")
        .bind(KernelEventType::ModelResponseRecorded.as_str())
        .bind(&trace_receipt.event_id)
        .execute(&setup.pool)
        .await
        .expect("corrupt trace receipt type");
    let err = replay_context_bundle_from_trace(&setup.fx.pg.db, &setup.stored_trace.trace_id)
        .await
        .expect_err("wrong receipt event type must fail replay");
    assert!(
        err.to_string().contains("receipt event_type"),
        "unexpected receipt-type error: {err}"
    );

    sqlx::query("UPDATE kernel_event_ledger SET event_type = $1 WHERE event_id = $2")
        .bind(KernelEventType::KnowledgeRetrievalTraceRecorded.as_str())
        .bind(&trace_receipt.event_id)
        .execute(&setup.pool)
        .await
        .expect("restore trace receipt type");
    let wrong_workspace_id = setup.fx.pg.create_workspace().await;
    sqlx::query("UPDATE knowledge_retrieval_traces SET workspace_id = $1 WHERE trace_id = $2")
        .bind(wrong_workspace_id)
        .bind(&setup.stored_trace.trace_id)
        .execute(&setup.pool)
        .await
        .expect("corrupt trace workspace");
    let err = replay_context_bundle_from_trace(&setup.fx.pg.db, &setup.stored_trace.trace_id)
        .await
        .expect_err("trace/bundle workspace drift must fail replay");
    assert!(
        err.to_string().contains("workspace"),
        "unexpected workspace-drift error: {err}"
    );
}

#[tokio::test]
async fn replay_context_bundle_from_trace_fails_without_eventledger_receipts() {
    let setup = skip_if_no_pg!(
        span_replay_setup("missing_receipts").await,
        "replay_context_bundle_from_trace_fails_without_eventledger_receipts"
    );

    sqlx::query(
        "UPDATE knowledge_context_bundles SET build_receipt_event_id = NULL WHERE bundle_id = $1",
    )
    .bind(&setup.compiled.bundle_id)
    .execute(&setup.pool)
    .await
    .expect("remove build receipt");
    let err = replay_context_bundle_from_trace(&setup.fx.pg.db, &setup.stored_trace.trace_id)
        .await
        .expect_err("missing build receipt must fail replay");
    assert!(
        err.to_string().contains("build_receipt_event_id"),
        "unexpected missing-build-receipt error: {err}"
    );

    let setup = span_replay_setup("missing_trace_receipt")
        .await
        .expect("PostgreSQL fixture remains available");
    sqlx::query(
        "UPDATE knowledge_retrieval_traces SET trace_receipt_event_id = NULL WHERE trace_id = $1",
    )
    .bind(&setup.stored_trace.trace_id)
    .execute(&setup.pool)
    .await
    .expect("remove trace receipt");
    let err = replay_context_bundle_from_trace(&setup.fx.pg.db, &setup.stored_trace.trace_id)
        .await
        .expect_err("missing trace receipt must fail replay");
    assert!(
        err.to_string().contains("trace_receipt_event_id")
            || err.to_string().contains("kernel event receipt"),
        "unexpected missing-trace-receipt error: {err}"
    );
}

#[tokio::test]
async fn replay_context_bundle_from_trace_fails_on_receipt_aggregate_id_mismatch() {
    let setup = skip_if_no_pg!(
        span_replay_setup("aggregate_mismatch").await,
        "replay_context_bundle_from_trace_fails_on_receipt_aggregate_id_mismatch"
    );

    let replay = replay_context_bundle_from_trace(&setup.fx.pg.db, &setup.stored_trace.trace_id)
        .await
        .expect("initial replay");
    let build_receipt = replay
        .receipt_events
        .iter()
        .find(|event| event.aggregate_type == "context_bundle")
        .expect("build receipt");

    sqlx::query("UPDATE kernel_event_ledger SET aggregate_id = $1 WHERE event_id = $2")
        .bind("wrong-context-bundle")
        .bind(&build_receipt.event_id)
        .execute(&setup.pool)
        .await
        .expect("corrupt build aggregate id");
    let err = replay_context_bundle_from_trace(&setup.fx.pg.db, &setup.stored_trace.trace_id)
        .await
        .expect_err("wrong aggregate_id must fail replay");
    assert!(
        err.to_string().contains("aggregate_id"),
        "unexpected aggregate-id error: {err}"
    );
}

#[tokio::test]
async fn replay_context_bundle_from_trace_fails_when_selected_trace_items_are_missing() {
    let setup = skip_if_no_pg!(
        span_replay_setup("selected_missing").await,
        "replay_context_bundle_from_trace_fails_when_selected_trace_items_are_missing"
    );

    sqlx::query(
        r#"
        UPDATE knowledge_retrieval_traces
        SET decisions = jsonb_set(decisions, '{retrieval_trace,selected}', '[]'::jsonb)
        WHERE trace_id = $1
        "#,
    )
    .bind(&setup.stored_trace.trace_id)
    .execute(&setup.pool)
    .await
    .expect("remove selected evidence");

    let err = replay_context_bundle_from_trace(&setup.fx.pg.db, &setup.stored_trace.trace_id)
        .await
        .expect_err("empty selected trace must fail replay");
    assert!(
        err.to_string().contains("selected evidence"),
        "unexpected selected-missing error: {err}"
    );
}

#[tokio::test]
async fn replay_context_bundle_from_trace_fails_when_selected_candidate_is_missing() {
    let setup = skip_if_no_pg!(
        span_replay_setup("selected_candidate_missing").await,
        "replay_context_bundle_from_trace_fails_when_selected_candidate_is_missing"
    );

    sqlx::query(
        r#"
        UPDATE knowledge_retrieval_traces
        SET decisions = jsonb_set(decisions, '{retrieval_trace,candidates}', '[]'::jsonb)
        WHERE trace_id = $1
        "#,
    )
    .bind(&setup.stored_trace.trace_id)
    .execute(&setup.pool)
    .await
    .expect("remove candidate evidence");

    let err = replay_context_bundle_from_trace(&setup.fx.pg.db, &setup.stored_trace.trace_id)
        .await
        .expect_err("selected evidence missing its ranked candidate must fail replay");
    assert!(
        err.to_string().contains("candidate"),
        "unexpected selected-candidate-missing error: {err}"
    );
}

#[tokio::test]
async fn replay_context_bundle_from_trace_reconstructs_executor_passage_span_anchors() {
    let fx = skip_if_no_pg!(
        MemoryFixture::setup().await,
        "replay_context_bundle_from_trace_reconstructs_executor_passage_span_anchors"
    );

    let passage = fx
        .pg
        .db
        .create_knowledge_memory_passage(NewKnowledgeMemoryPassage {
            workspace_id: fx.workspace_id.clone(),
            passage_text: "passage candidate with span lineage".to_string(),
            token_count: Some(7),
            ocr_transcript_metadata: None,
            extraction_confidence: 0.99,
            ranking_features: serde_json::json!({"fixture": "MT-238"}),
            retrieval_mode: KnowledgeRetrievalMode::HybridRag,
            compaction_policy: KnowledgeCompactionPolicy::Compactable,
            failure_receipt_event_id: None,
            derived_in_run: None,
            evidence: vec![KnowledgePassageEvidenceRef::Span {
                span_id: fx.span_id.clone(),
            }],
        })
        .await
        .expect("passage");
    let target_entity_id = fx
        .entity("symbol", "mt238_passage_target", "Mt238PassageTarget")
        .await;
    let planner = CheapestAuthoritativePathPlanner::new(&fx.pg.db);
    let request = RetrievalRequest::discovery(&fx.workspace_id, "replay MT-238 passage bundle")
        .with_handle(AuthoritativeHandle::EntityId(target_entity_id.clone()));
    let planned = planner.plan(&request).await.expect("plan");
    let mut trace = RetrievalTrace::for_plan(&planned.plan);
    let snippet = assemble_span_snippet(&fx.pg.db, &fx.span_id)
        .await
        .expect("snippet");

    let compiled = ContextBundleCompilerV2::new(&fx.pg.db)
        .compile(
            &fx.workspace_id,
            "ktr-mt238-passage",
            "sr-mt238-passage",
            BundleTargetKind::Symbol,
            &target_entity_id,
            &planned.plan,
            &mut trace,
            &[BundleCandidate {
                ref_kind: KnowledgeBundleItemRefKind::Passage,
                ref_id: passage.passage_id.clone(),
                tier: PriorityTier::Supplementary,
                token_count: 7,
                relevance_score: 0.72,
                source_id: fx.source_id.clone(),
                snippet: Some(snippet),
            }],
            None,
            None,
        )
        .await
        .expect("compile passage bundle");

    let stored_trace = traces_for_bundle(&fx.pg.db, &compiled.bundle_id)
        .await
        .expect("traces")
        .pop()
        .expect("trace");
    let replay = replay_context_bundle_from_trace(&fx.pg.db, &stored_trace.trace_id)
        .await
        .expect("passage replay");
    let passage_item: &KnowledgeContextBundleItem = replay
        .items
        .iter()
        .find(|item| item.ref_id == passage.passage_id)
        .expect("passage item");
    assert_eq!(passage_item.ref_kind, KnowledgeBundleItemRefKind::Passage);
    let evidence = replay
        .evidence
        .iter()
        .find(|evidence| evidence.ref_id == passage.passage_id)
        .expect("passage evidence");
    assert_eq!(evidence.span_id.as_deref(), Some(fx.span_id.as_str()));
    assert_eq!(evidence.source_id.as_deref(), Some(fx.source_id.as_str()));
    assert_eq!(evidence.source_path.as_deref(), Some("memory/graph.rs"));
}

#[tokio::test]
async fn replay_context_bundle_from_trace_reconstructs_executor_graph_edge_span_anchors() {
    let fx = skip_if_no_pg!(
        MemoryFixture::setup().await,
        "replay_context_bundle_from_trace_reconstructs_executor_graph_edge_span_anchors"
    );
    let pool = pool_for(&fx.pg).await;

    let hub = fx
        .entity("symbol", "mt238_graph_hub", "Mt238GraphHub")
        .await;
    let target = fx
        .entity("symbol", "mt238_graph_target", "Mt238GraphTarget")
        .await;
    let edge = fx
        .pg
        .db
        .upsert_knowledge_edge(NewKnowledgeEdge {
            workspace_id: fx.workspace_id.clone(),
            edge_type: KnowledgeEdgeType::DependsOn,
            source_entity_id: hub.clone(),
            target_entity_id: target,
            extractor_version: "test_v1".to_string(),
            confidence: 0.96,
            detected_in_run: None,
            evidence_span_ids: vec![fx.span_id.clone()],
        })
        .await
        .expect("edge");

    let mut request = RetrievalRequest::discovery(&fx.workspace_id, "replay MT-238 graph bundle");
    request.graph_neighborhood_expected = true;
    let executed = execute_retrieval(
        &fx.pg.db,
        &pool,
        "ktr-mt238-graph",
        "sr-mt238-graph",
        BundleTargetKind::Symbol,
        &hub,
        &request,
        &BTreeSet::from([hub.clone()]),
        GraphTraversalPolicy::default(),
    )
    .await
    .expect("execute graph retrieval");
    assert_eq!(executed.ranked[0].kind, "entity_ref");

    let stored_trace = traces_for_bundle(&fx.pg.db, &executed.compiled.bundle_id)
        .await
        .expect("traces")
        .pop()
        .expect("trace");
    let replay = replay_context_bundle_from_trace(&fx.pg.db, &stored_trace.trace_id)
        .await
        .expect("graph edge replay");
    let graph_item: &KnowledgeContextBundleItem = replay
        .items
        .iter()
        .find(|item| item.ref_id == edge.relationship_id)
        .expect("graph edge item");
    assert_eq!(graph_item.ref_kind, KnowledgeBundleItemRefKind::Entity);
    let evidence = replay
        .evidence
        .iter()
        .find(|evidence| evidence.ref_id == edge.relationship_id)
        .expect("graph edge evidence");
    assert_eq!(evidence.span_id.as_deref(), Some(fx.span_id.as_str()));
    assert_eq!(evidence.source_id.as_deref(), Some(fx.source_id.as_str()));
    assert_eq!(evidence.source_path.as_deref(), Some("memory/graph.rs"));
}
