//! WP-KERNEL-009 MT-064 PostgresUnavailableFailClosed.
//!
//! Negative tests with an unreachable database URL: every knowledge storage
//! API must fail CLOSED with a typed `StorageError` when PostgreSQL is
//! unavailable. There is no in-memory, SQLite, fixture, or cache fallback to
//! observe — `KnowledgeStore` is implemented for `PostgresDatabase` only,
//! and `PostgresDatabase` is constructible only over a real PgPool
//! (`connect`/`connect_with_guard`/`new`). These tests prove the runtime
//! behavior of that boundary:
//!   * eager connection to an unreachable URL is a typed error, not a panic
//!     and not a silent success;
//!   * a lazily-pooled handle (connection deferred to first use) surfaces a
//!     typed `StorageError::Database` from EVERY representative knowledge
//!     API — reads, writes, audits, and idempotent writes alike — instead of
//!     inventing state.
//!
//! No PostgreSQL is required to run these tests; the URL points at a closed
//! port on localhost, so they are true negatives and always execute.

use std::time::Duration;

use handshake_core::storage::knowledge::{
    KnowledgeCompactionPolicy, KnowledgePassageEvidenceRef, KnowledgeProjectionKind,
    KnowledgeRetrievalMode, KnowledgeStore, NewKnowledgeMemoryPassage, NewKnowledgeRetrievalTrace,
    NewKnowledgeRichDocument, NewKnowledgeWikiProjection,
};
use handshake_core::storage::postgres::PostgresDatabase;
use handshake_core::storage::StorageError;
use serde_json::json;
use sqlx::postgres::PgPoolOptions;

/// Nothing listens on TCP port 1 (tcpmux) on localhost; connections are
/// refused immediately, which is exactly the "PostgreSQL unavailable" shape.
const UNREACHABLE_URL: &str = "postgres://hsk:invalid@127.0.0.1:1/hsk_unreachable";

fn assert_fails_closed<T: std::fmt::Debug>(api: &str, result: Result<T, StorageError>) {
    match result {
        Ok(value) => panic!(
            "{api} must fail closed when PostgreSQL is unavailable, got Ok({value:?}) — \
             a success here means some non-PostgreSQL state answered"
        ),
        Err(StorageError::Database(_)) => {}
        Err(other) => panic!(
            "{api} must surface the infrastructure failure as the typed \
             StorageError::Database, got {other:?}"
        ),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn eager_connect_to_unreachable_postgres_is_a_typed_error() {
    let result = PostgresDatabase::connect(UNREACHABLE_URL, 1).await;
    match result {
        Ok(_) => panic!("connecting to an unreachable PostgreSQL must fail"),
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
async fn every_knowledge_api_fails_closed_when_postgres_is_unavailable() {
    // Lazy pool: construction succeeds, the connection failure surfaces at
    // FIRST USE — the dangerous window where a fallback could hide. The
    // short acquire timeout only bounds retry time; the refused connection
    // errors immediately.
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_millis(700))
        .connect_lazy(UNREACHABLE_URL)
        .expect("lazy pool construction parses the URL");
    let db = PostgresDatabase::new(pool);

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

    // Writes (payloads pass Rust-side validation so the call genuinely
    // reaches the unavailable pool).
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
    // results without PostgreSQL would be an in-memory authority violation.
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
        )
        .await,
    );
}
