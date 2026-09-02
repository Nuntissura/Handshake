//! WP-KERNEL-009 MT-064 EmbeddedStoreFailClosed.
//!
//! Negative tests after the embedded store is closed: every knowledge storage
//! API must fail CLOSED with a typed `StorageError`. There is no in-memory,
//! fixture, or cache fallback to observe. These tests prove the runtime
//! behavior of that boundary across reads, writes, audits, and idempotent
//! writes alike instead of inventing state.

use handshake_core::storage::knowledge::{
    KnowledgeCompactionPolicy, KnowledgePassageEvidenceRef, KnowledgeProjectionKind,
    KnowledgeRetrievalMode, KnowledgeStore, NewKnowledgeMemoryPassage, NewKnowledgeRetrievalTrace,
    NewKnowledgeRichDocument, NewKnowledgeWikiProjection,
};
use handshake_core::storage::surreal::SurrealDatabase;
use handshake_core::storage::{Database, StorageError};
use serde_json::json;

#[path = "knowledge_ingestion_support.rs"]
mod knowledge_embedded_support;
use knowledge_embedded_support::open_embedded_store as embedded_knowledge;

fn assert_fails_closed<T: std::fmt::Debug>(api: &str, result: Result<T, StorageError>) {
    match result {
        Ok(value) => {
            panic!("{api} must fail closed when the embedded store is closed, got Ok({value:?})")
        }
        Err(StorageError::Database(_)) => {}
        Err(other) => panic!(
            "{api} must surface the closed-store failure as the typed \
             StorageError::Database, got {other:?}"
        ),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn closed_embedded_store_ping_is_a_typed_error() {
    let store = embedded_knowledge()
        .await
        .expect("embedded knowledge store must be available for the negative probe");
    store
        .shutdown()
        .await
        .expect("closing the embedded knowledge store");
    let result = store.db.ping().await;
    match result {
        Ok(_) => panic!("pinging a closed embedded store must fail"),
        Err(StorageError::Database(message)) => {
            assert!(
                !message.is_empty(),
                "the typed error must carry diagnosable detail"
            );
        }
        Err(other) => panic!("expected the typed StorageError::Database, got {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn every_knowledge_api_fails_closed_when_embedded_store_is_closed() {
    let store = embedded_knowledge()
        .await
        .expect("embedded knowledge store must be available for the negative probe");
    store
        .shutdown()
        .await
        .expect("closing the embedded knowledge store");
    let db = SurrealDatabase::new(store.storage.clone());

    // Reads and audits.
    assert_fails_closed(
        "list_knowledge_schema_registry",
        db.list_knowledge_schema_registry().await,
    );
    assert_fails_closed(
        "audit_knowledge_namespace",
        db.audit_knowledge_namespace().await,
    );
    assert_fails_closed(
        "get_knowledge_memory_passage",
        db.get_knowledge_memory_passage("KMP-00000000000000000000000000000000")
            .await,
    );
    assert_fails_closed(
        "get_knowledge_rich_document",
        db.get_knowledge_rich_document("KRD-00000000000000000000000000000000")
            .await,
    );
    assert_fails_closed(
        "get_knowledge_context_bundle",
        db.get_knowledge_context_bundle("CTX-0000000000000000")
            .await,
    );
    assert_fails_closed(
        "get_knowledge_wiki_projection",
        db.get_knowledge_wiki_projection("KWP-00000000000000000000000000000000")
            .await,
    );

    // Writes pass Rust-side validation so the call genuinely reaches the
    // closed store.
    let passage = NewKnowledgeMemoryPassage {
        workspace_id: "ws-unreachable".to_string(),
        passage_text: "fail closed".to_string(),
        token_count: Some(2),
        ocr_transcript_metadata: None,
        extraction_confidence: 1.0,
        ranking_features: json!({}),
        retrieval_mode: KnowledgeRetrievalMode::DirectLoad,
        compaction_policy: KnowledgeCompactionPolicy::Keep,
        failure_receipt_event_id: None,
        derived_in_run: None,
        evidence: vec![KnowledgePassageEvidenceRef::Span {
            span_id: "KSP-00000000000000000000000000000000".to_string(),
        }],
    };
    assert_fails_closed(
        "create_knowledge_memory_passage",
        db.create_knowledge_memory_passage(passage.clone()).await,
    );
    assert_fails_closed(
        "create_knowledge_rich_document",
        db.create_knowledge_rich_document(NewKnowledgeRichDocument {
            workspace_id: "ws-unreachable".to_string(),
            document_id: None,
            title: "Unreachable".to_string(),
            schema_version: "hsk_richdoc_v1".to_string(),
            content_json: json!({"type": "doc", "content": []}),
            crdt_document_id: None,
            crdt_snapshot_id: None,
            promotion_receipt_event_id: None,
            // MT-145 identity fields default (no project/folder/owner,
            // 'promoted' label).
            ..Default::default()
        })
        .await,
    );
    assert_fails_closed(
        "upsert_knowledge_wiki_projection",
        db.upsert_knowledge_wiki_projection(NewKnowledgeWikiProjection {
            workspace_id: "ws-unreachable".to_string(),
            projection_kind: KnowledgeProjectionKind::WikiPage,
            title: "Unreachable".to_string(),
            source_records: json!([]),
            rendered_content: String::new(),
            staleness_hash: "0".repeat(64),
        })
        .await,
    );
    assert_fails_closed(
        "record_knowledge_retrieval_trace",
        db.record_knowledge_retrieval_trace(NewKnowledgeRetrievalTrace {
            workspace_id: "ws-unreachable".to_string(),
            retrieval_mode: KnowledgeRetrievalMode::None,
            mode_reason: "fail-closed probe".to_string(),
            query_text: None,
            bundle_id: None,
            decisions: json!([]),
            trace_receipt_event_id: None,
        })
        .await,
    );

    // Idempotent writes fail closed too — a replay engine that "remembers"
    // results after shutdown would be an in-memory authority violation.
    assert_fails_closed(
        "create_knowledge_memory_passage_idempotent",
        db.create_knowledge_memory_passage_idempotent("idem-unreachable-1", passage)
            .await,
    );
    assert_fails_closed(
        "save_knowledge_rich_document_version_idempotent",
        db.save_knowledge_rich_document_version_idempotent(
            "idem-unreachable-2",
            "KRD-00000000000000000000000000000000",
            1,
            json!({"type": "doc", "content": []}),
            None,
            None,
            None,
        )
        .await,
    );
}
