//! WP-KERNEL-009 MT-062 TransactionalIdempotencyKeys integration tests
//! against REAL Handshake-managed PostgreSQL.
//!
//! Proof targets (the migration-documented discipline, 0142):
//!   * replayed request (same key + same payload) returns the prior result
//!     and writes NOTHING (no double-write, proven by row counts);
//!   * same key + different payload is a typed Conflict;
//!   * two racing writers on one key produce exactly one write;
//!   * the editor-save surface replays the promoted revision instead of a
//!     version conflict.

mod knowledge_pg_support;

use handshake_core::storage::knowledge::{
    KnowledgeCompactionPolicy, KnowledgeIndexingEligibility, KnowledgePassageEvidenceRef,
    KnowledgePermissionScope, KnowledgeRedactionState, KnowledgeRetrievalMode, KnowledgeRootKind,
    KnowledgeSourceKind, KnowledgeSpanKind, KnowledgeStore, NewKnowledgeMemoryPassage,
    NewKnowledgeRichDocument, NewKnowledgeSource, NewKnowledgeSourceRoot, NewKnowledgeSpan,
};
use handshake_core::storage::StorageError;
use knowledge_pg_support::{knowledge_pg, KnowledgePg};
use serde_json::json;
use uuid::Uuid;

const HASH_SRC: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const HASH_SPAN: &str = "2222222222222222222222222222222222222222222222222222222222222222";

/// Real workspace -> root -> source -> span so passages have honest lineage
/// (same fixture shape as knowledge_claims_tests).
async fn span_fixture(pg: &KnowledgePg) -> (String, String, String) {
    let workspace_id = pg.create_workspace().await;
    let root = pg
        .db
        .create_knowledge_source_root(NewKnowledgeSourceRoot {
            workspace_id: workspace_id.clone(),
            display_name: "core".to_string(),
            root_kind: KnowledgeRootKind::ProjectRepo,
            repo_relative_path: format!("src/{}", Uuid::now_v7().simple()),
            allowlist_policy: json!({"include": ["**/*"], "exclude": []}),
            indexing_eligibility: KnowledgeIndexingEligibility::Eligible,
        })
        .await
        .expect("root");
    let source = pg
        .db
        .upsert_knowledge_source(NewKnowledgeSource {
            workspace_id: workspace_id.clone(),
            root_id: Some(root.root_id),
            source_kind: KnowledgeSourceKind::File,
            relative_path: Some("storage/knowledge.rs".to_string()),
            asset_id: None,
            loom_block_id: None,
            document_id: None,
            content_hash: HASH_SRC.to_string(),
            size_bytes: Some(1024),
            provenance: json!({"discovered_by": "idempotency_test"}),
            permission_scope: KnowledgePermissionScope::Workspace,
            redaction_state: KnowledgeRedactionState::None,
            source_modified_at: None,
        })
        .await
        .expect("source");
    let span = pg
        .db
        .create_knowledge_span(NewKnowledgeSpan {
            source_id: source.source_id.clone(),
            span_kind: KnowledgeSpanKind::Text,
            range_start: 0,
            range_end: 240,
            line_start: Some(1),
            line_end: Some(6),
            section_path: None,
            content_sha256: HASH_SPAN.to_string(),
            parser_version: "text_v1".to_string(),
            extraction_receipt_event_id: None,
            index_run_id: None,
            display_snippet: Some("module docs".to_string()),
        })
        .await
        .expect("span");
    (workspace_id, source.source_id, span.span_id)
}

fn passage(workspace_id: &str, span_id: &str, text: &str) -> NewKnowledgeMemoryPassage {
    NewKnowledgeMemoryPassage {
        workspace_id: workspace_id.to_string(),
        passage_text: text.to_string(),
        token_count: Some(12),
        ocr_transcript_metadata: None,
        extraction_confidence: 0.9,
        ranking_features: json!({"recency_score": 0.5}),
        retrieval_mode: KnowledgeRetrievalMode::HybridRag,
        compaction_policy: KnowledgeCompactionPolicy::Keep,
        failure_receipt_event_id: None,
        derived_in_run: None,
        evidence: vec![KnowledgePassageEvidenceRef::Span {
            span_id: span_id.to_string(),
        }],
    }
}

async fn passage_count(pg: &KnowledgePg, workspace_id: &str) -> i64 {
    let mut conn = pg.raw_connection().await;
    sqlx::query_scalar("SELECT COUNT(*) FROM knowledge_memory_passages WHERE workspace_id = $1")
        .bind(workspace_id)
        .fetch_one(&mut conn)
        .await
        .expect("count passages")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn replayed_passage_write_returns_prior_result_without_double_write() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!(
            "SKIP replayed_passage_write_returns_prior_result_without_double_write: no PostgreSQL"
        );
        return;
    };
    let (workspace_id, _source_id, span_id) = span_fixture(&pg).await;
    let key = format!("idem-passage-{}", Uuid::now_v7());
    let payload = passage(&workspace_id, &span_id, "managed PG listens on 5544");

    let first = pg
        .db
        .create_knowledge_memory_passage_idempotent(&key, payload.clone())
        .await
        .expect("first write");
    assert!(!first.replayed, "first write must be a real write");

    let second = pg
        .db
        .create_knowledge_memory_passage_idempotent(&key, payload.clone())
        .await
        .expect("replay");
    assert!(second.replayed, "replay must be flagged as replayed");
    assert_eq!(
        second.value, first.value,
        "replay must return the prior result"
    );
    assert_eq!(
        passage_count(&pg, &workspace_id).await,
        1,
        "replay must not double-write"
    );

    // The key ledger holds exactly one row for the key.
    let mut conn = pg.raw_connection().await;
    let keys: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM knowledge_idempotency_keys WHERE idempotency_key = $1",
    )
    .bind(&key)
    .fetch_one(&mut conn)
    .await
    .expect("count keys");
    assert_eq!(keys, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn same_key_with_divergent_payload_is_a_typed_conflict() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP same_key_with_divergent_payload_is_a_typed_conflict: no PostgreSQL");
        return;
    };
    let (workspace_id, _source_id, span_id) = span_fixture(&pg).await;
    let key = format!("idem-divergent-{}", Uuid::now_v7());

    pg.db
        .create_knowledge_memory_passage_idempotent(
            &key,
            passage(&workspace_id, &span_id, "original payload"),
        )
        .await
        .expect("first write");

    let err = pg
        .db
        .create_knowledge_memory_passage_idempotent(
            &key,
            passage(&workspace_id, &span_id, "DIFFERENT payload"),
        )
        .await
        .expect_err("divergent duplicate must be rejected");
    assert!(matches!(err, StorageError::Conflict(_)), "got {err:?}");
    assert_eq!(
        passage_count(&pg, &workspace_id).await,
        1,
        "the divergent duplicate must not write"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn racing_writers_on_one_key_produce_exactly_one_write() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP racing_writers_on_one_key_produce_exactly_one_write: no PostgreSQL");
        return;
    };
    let (workspace_id, _source_id, span_id) = span_fixture(&pg).await;
    let key = format!("idem-race-{}", Uuid::now_v7());
    let payload = passage(&workspace_id, &span_id, "raced payload");

    let (left, right) = tokio::join!(
        pg.db
            .create_knowledge_memory_passage_idempotent(&key, payload.clone()),
        pg.db
            .create_knowledge_memory_passage_idempotent(&key, payload.clone()),
    );
    let left = left.expect("left racer");
    let right = right.expect("right racer");

    assert_eq!(
        left.value.passage_id, right.value.passage_id,
        "both racers must converge on one result"
    );
    assert_eq!(
        passage_count(&pg, &workspace_id).await,
        1,
        "a race must never double-write"
    );
    assert!(
        !(left.replayed && right.replayed),
        "at least one racer performed the real write"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn editor_save_replay_returns_promoted_revision_without_double_write() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!(
            "SKIP editor_save_replay_returns_promoted_revision_without_double_write: no PostgreSQL"
        );
        return;
    };
    let workspace_id = pg.create_workspace().await;
    let document = pg
        .db
        .create_knowledge_rich_document(NewKnowledgeRichDocument {
            workspace_id: workspace_id.clone(),
            document_id: None,
            title: "Idempotent Runbook".to_string(),
            schema_version: "hsk_richdoc_v1".to_string(),
            content_json: json!({"type": "doc", "content": []}),
            crdt_document_id: None,
            crdt_snapshot_id: None,
            promotion_receipt_event_id: None,
        })
        .await
        .expect("create doc");

    let key = format!("idem-save-{}", Uuid::now_v7());
    let v2 = json!({
        "type": "doc",
        "content": [{"type": "paragraph", "content": [{"type": "text", "text": "v2"}]}]
    });

    let first = pg
        .db
        .save_knowledge_rich_document_version_idempotent(
            &key,
            &document.rich_document_id,
            1,
            v2.clone(),
            None,
        )
        .await
        .expect("first save");
    assert!(!first.replayed);
    assert_eq!(first.value.doc_version, 2);

    // The EXACT same request again: a non-idempotent path would now fail
    // with a version conflict; the idempotent path replays the result.
    let second = pg
        .db
        .save_knowledge_rich_document_version_idempotent(
            &key,
            &document.rich_document_id,
            1,
            v2.clone(),
            None,
        )
        .await
        .expect("replayed save");
    assert!(second.replayed);
    assert_eq!(second.value.doc_version, 2);
    assert_eq!(second.value.content_sha256, first.value.content_sha256);

    let versions = pg
        .db
        .list_knowledge_rich_document_versions(&document.rich_document_id)
        .await
        .expect("history");
    assert_eq!(versions.len(), 2, "replay must not append to the history");

    // A DIFFERENT save reusing the key stays a typed Conflict.
    let err = pg
        .db
        .save_knowledge_rich_document_version_idempotent(
            &key,
            &document.rich_document_id,
            2,
            json!({"type": "doc", "content": [{"type": "paragraph"}]}),
            None,
        )
        .await
        .expect_err("key reuse with a different save must be rejected");
    assert!(matches!(err, StorageError::Conflict(_)), "got {err:?}");
}
