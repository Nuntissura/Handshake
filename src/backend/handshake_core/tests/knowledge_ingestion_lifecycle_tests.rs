//! WP-KERNEL-009 SourceIngestionAndEvidence lifecycle proofs against REAL
//! Handshake-managed PostgreSQL: MT-092 (LargeFileBackpressure), MT-093
//! (DeletedMovedSourceHandling), MT-094 (SourceRepairQueue).
//!
//! These tests run full ingestion passes over runtime temp directories (the
//! machine-local fs anchor is runtime input, never stored), simulate file
//! moves/deletes between passes, and drive the durable repair queue through
//! failure -> queued -> retry -> resolved and through dead-letter exhaustion.

mod knowledge_ingestion_support;
mod knowledge_pg_support;

use std::path::Path;

use handshake_core::knowledge_ingestion::backpressure::IngestionLimits;
use handshake_core::knowledge_ingestion::receipts::{ExtractionStatus, IngestionErrorClass};
use handshake_core::knowledge_ingestion::repair::RepairState;
use handshake_core::storage::knowledge::{KnowledgeRootKind, KnowledgeStore};
use knowledge_ingestion_support::{ingestion_pg, register_root, test_ctx};

fn write(dir: &Path, rel: &str, content: &[u8]) {
    let path = dir.join(rel);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir fixture tree");
    std::fs::write(path, content).expect("write runtime fixture file");
}

// ---------------------------------------------------------------------------
// MT-092 LargeFileBackpressure
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt092_oversize_files_defer_typed_with_streamed_hash_never_loaded() {
    let Some(env) = ingestion_pg().await else {
        eprintln!("SKIP mt092_oversize_files_defer: no PostgreSQL");
        return;
    };
    let workspace_id = env.pg.create_workspace().await;
    let ctx = test_ctx("mt092");
    let root = register_root(
        &env,
        &ctx,
        &workspace_id,
        "",
        KnowledgeRootKind::ProjectRepo,
    )
    .await;

    // Generate the large fixture AT TEST TIME (never committed): 3 MiB of
    // text against the 2 MiB default ceiling.
    let temp = tempfile::tempdir().expect("temp dir");
    let big = "x".repeat(3 * 1024 * 1024);
    write(temp.path(), "big.md", big.as_bytes());
    write(temp.path(), "small.md", b"# Small\n\nfits fine\n");

    let limits = IngestionLimits::default();
    let summary = env
        .engine
        .run_ingestion_pass(&ctx, &root.root_id, temp.path(), &limits)
        .await
        .expect("ingestion pass");

    let big_outcome = summary
        .outcomes
        .iter()
        .find(|o| o.source.relative_path.as_deref() == Some("big.md"))
        .expect("big file outcome");
    assert_eq!(big_outcome.receipt.status, ExtractionStatus::Deferred);
    assert_eq!(
        big_outcome.receipt.error_class,
        Some(IngestionErrorClass::Oversize)
    );
    let detail = big_outcome.receipt.error_detail.as_ref().expect("detail");
    assert_eq!(detail["reason"], "oversize_bytes");
    assert_eq!(detail["limit"], 2 * 1024 * 1024);
    assert_eq!(detail["actual"], 3 * 1024 * 1024);
    assert!(big_outcome.spans.is_empty());
    // Deferral is repairable: raise the limit and retry.
    assert!(big_outcome.repair.is_some());
    // The oversize path STREAM-hashes: hash must equal the full-content hash.
    let expected_hash = {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(big.as_bytes());
        hex::encode(hasher.finalize())
    };
    assert_eq!(big_outcome.source.content_hash, expected_hash);
    // Deferred sources stay PENDING (not failed, not extracted).
    assert_eq!(big_outcome.source.parser_status.as_str(), "pending");

    let small_outcome = summary
        .outcomes
        .iter()
        .find(|o| o.source.relative_path.as_deref() == Some("small.md"))
        .expect("small file outcome");
    assert_eq!(small_outcome.receipt.status, ExtractionStatus::Success);

    // Line-count ceiling defers typed too (content-stage check).
    let many_lines = "line\n".repeat(60_000);
    let outcome = env
        .engine
        .ingest_file_bytes(
            &ctx,
            &root,
            "long.md",
            many_lines.as_bytes(),
            "KIRUN-mt092",
            &IngestionLimits {
                max_bytes: 10 * 1024 * 1024,
                ..IngestionLimits::default()
            },
            false,
        )
        .await
        .expect("ingest long file");
    assert_eq!(outcome.receipt.status, ExtractionStatus::Deferred);
    assert_eq!(
        outcome.receipt.error_detail.as_ref().expect("detail")["reason"],
        "oversize_lines"
    );
}

// ---------------------------------------------------------------------------
// MT-093 DeletedMovedSourceHandling
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt093_reindex_marks_deleted_and_moved_sources_stale_without_hard_delete() {
    let Some(env) = ingestion_pg().await else {
        eprintln!("SKIP mt093_reindex_marks_stale: no PostgreSQL");
        return;
    };
    let workspace_id = env.pg.create_workspace().await;
    let ctx = test_ctx("mt093");
    let root = register_root(
        &env,
        &ctx,
        &workspace_id,
        "",
        KnowledgeRootKind::ProjectRepo,
    )
    .await;

    let temp = tempfile::tempdir().expect("temp dir");
    write(temp.path(), "keep.md", b"# Keep\n\nstays in place\n");
    write(temp.path(), "gone.md", b"# Gone\n\nwill be deleted\n");
    write(
        temp.path(),
        "old_name.md",
        b"# Moved\n\nsame bytes, new path\n",
    );

    let limits = IngestionLimits::default();
    let pass1 = env
        .engine
        .run_ingestion_pass(&ctx, &root.root_id, temp.path(), &limits)
        .await
        .expect("pass 1");
    assert_eq!(pass1.outcomes.len(), 3);
    assert!(pass1.stale_marked.is_empty());
    let gone_source_id = pass1
        .outcomes
        .iter()
        .find(|o| o.source.relative_path.as_deref() == Some("gone.md"))
        .expect("gone.md outcome")
        .source
        .source_id
        .clone();
    let moved_source_id = pass1
        .outcomes
        .iter()
        .find(|o| o.source.relative_path.as_deref() == Some("old_name.md"))
        .expect("old_name.md outcome")
        .source
        .source_id
        .clone();

    // Simulate the move + delete between passes.
    std::fs::rename(
        temp.path().join("old_name.md"),
        temp.path().join("new_name.md"),
    )
    .expect("rename file");
    std::fs::remove_file(temp.path().join("gone.md")).expect("delete file");

    let pass2 = env
        .engine
        .run_ingestion_pass(&ctx, &root.root_id, temp.path(), &limits)
        .await
        .expect("pass 2");

    // Typed dispositions: same content at a new path = moved (with target),
    // vanished content = deleted.
    assert_eq!(pass2.stale_marked.len(), 2);
    let moved_mark = pass2
        .stale_marked
        .iter()
        .find(|m| m.source_id == moved_source_id)
        .expect("moved mark");
    assert_eq!(moved_mark.disposition, "moved");
    assert_eq!(moved_mark.moved_to.as_deref(), Some("new_name.md"));
    let deleted_mark = pass2
        .stale_marked
        .iter()
        .find(|m| m.source_id == gone_source_id)
        .expect("deleted mark");
    assert_eq!(deleted_mark.disposition, "deleted");
    assert_eq!(deleted_mark.moved_to, None);

    // Stable id survival: the moved CONTENT got a new source row at the new
    // path while the old row is stale — never hard-deleted, receipts intact.
    let old_row = env
        .engine
        .knowledge()
        .get_knowledge_source(&moved_source_id)
        .await
        .expect("get old source")
        .expect("old source row must still exist");
    assert!(old_row.stale, "old path row is stale, not deleted");
    let receipts = env
        .engine
        .store()
        .list_extraction_receipts(&moved_source_id, 10)
        .await
        .expect("old receipts");
    assert!(!receipts.is_empty(), "history stays citable");

    // The stale markers are EventLedger-backed.
    let mut conn = env.pg.raw_connection().await;
    let payload: serde_json::Value =
        sqlx::query_scalar("SELECT payload FROM kernel_event_ledger WHERE event_id = $1")
            .bind(&moved_mark.event_id)
            .fetch_one(&mut conn)
            .await
            .expect("stale-mark ledger event");
    assert_eq!(payload["kind"], "source_stale_marked");
    assert_eq!(payload["disposition"], "moved");
    assert_eq!(payload["moved_to"], "new_name.md");

    // Pass 3 with no changes: stale rows are not re-marked.
    let pass3 = env
        .engine
        .run_ingestion_pass(&ctx, &root.root_id, temp.path(), &limits)
        .await
        .expect("pass 3");
    assert!(pass3.stale_marked.is_empty(), "stale marking is idempotent");
}

// ---------------------------------------------------------------------------
// MT-094 SourceRepairQueue
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt094_failure_queues_then_retry_after_fix_resolves() {
    let Some(env) = ingestion_pg().await else {
        eprintln!("SKIP mt094_failure_queue_retry_resolve: no PostgreSQL");
        return;
    };
    let workspace_id = env.pg.create_workspace().await;
    let ctx = test_ctx("mt094-resolve");
    let root = register_root(
        &env,
        &ctx,
        &workspace_id,
        "",
        KnowledgeRootKind::MediaLibrary,
    )
    .await;

    let temp = tempfile::tempdir().expect("temp dir");
    // A transcript with NO well-formed cue: whole-file PARSE_ERROR.
    write(temp.path(), "broken.srt", b"garbage\nwithout any timing\n");

    let limits = IngestionLimits::default();
    let pass = env
        .engine
        .run_ingestion_pass(&ctx, &root.root_id, temp.path(), &limits)
        .await
        .expect("pass over broken transcript");
    let outcome = &pass.outcomes[0];
    assert_eq!(outcome.receipt.status, ExtractionStatus::Failed);
    let repair = outcome.repair.as_ref().expect("repair entry queued");
    assert_eq!(repair.state, RepairState::Queued);
    assert_eq!(repair.attempts, 0);
    assert_eq!(repair.max_attempts, 3);
    assert_eq!(repair.reason_class.as_str(), "PARSE_ERROR");

    // Queue is backend-visible and filterable.
    let queued = env
        .engine
        .store()
        .list_repair_entries(&workspace_id, Some(RepairState::Queued), 10)
        .await
        .expect("list queued repairs");
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].repair_id, repair.repair_id);

    // Operator fixes the artifact, then retries the entry.
    write(
        temp.path(),
        "broken.srt",
        b"1\n00:00:01,000 --> 00:00:02,000\nNow well-formed\n",
    );
    let (settled, attempt) = env
        .engine
        .retry_repair(&ctx, &repair.repair_id, temp.path(), &limits)
        .await
        .expect("retry repair");
    let attempt = attempt.expect("retry produced an ingest attempt");
    assert_eq!(attempt.receipt.status, ExtractionStatus::Success);
    assert_eq!(settled.state, RepairState::Resolved);
    assert_eq!(settled.attempts, 1);
    assert_eq!(
        settled.resolved_receipt_id.as_deref(),
        Some(attempt.receipt.receipt_id.as_str())
    );
    assert_eq!(attempt.spans.len(), 1, "fixed transcript produced spans");

    // Terminal entries refuse further retries (typed conflict).
    let err = env
        .engine
        .retry_repair(&ctx, &repair.repair_id, temp.path(), &limits)
        .await
        .expect_err("resolved entry must not retry");
    assert!(
        err.to_string().contains("terminal"),
        "unexpected error: {err}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt094_exhausted_attempts_dead_letter_and_missing_assets_requeue() {
    let Some(env) = ingestion_pg().await else {
        eprintln!("SKIP mt094_dead_letter: no PostgreSQL");
        return;
    };
    let workspace_id = env.pg.create_workspace().await;
    let ctx = test_ctx("mt094-dlq");
    let root = register_root(
        &env,
        &ctx,
        &workspace_id,
        "",
        KnowledgeRootKind::MediaLibrary,
    )
    .await;

    let temp = tempfile::tempdir().expect("temp dir");
    write(temp.path(), "stubborn.srt", b"never\na transcript\n");

    let limits = IngestionLimits::default();
    let pass = env
        .engine
        .run_ingestion_pass(&ctx, &root.root_id, temp.path(), &limits)
        .await
        .expect("pass");
    let repair_id = pass.outcomes[0]
        .repair
        .as_ref()
        .expect("queued repair")
        .repair_id
        .clone();

    // Retry 1: file vanished -> MISSING_ASSET, entry re-queues with the
    // attempt counted (file-system reality is typed, never a panic).
    std::fs::remove_file(temp.path().join("stubborn.srt")).expect("delete fixture");
    let (after_first, attempt) = env
        .engine
        .retry_repair(&ctx, &repair_id, temp.path(), &limits)
        .await
        .expect("retry against missing file");
    assert!(attempt.is_none(), "missing file produces no ingest attempt");
    assert_eq!(after_first.state, RepairState::Queued);
    assert_eq!(after_first.attempts, 1);
    assert_eq!(after_first.reason_detail["reason"], "MISSING_ASSET");

    // Retries 2..3: still failing -> budget exhausts -> dead_letter.
    write(temp.path(), "stubborn.srt", b"still\nnot a transcript\n");
    let (after_second, _) = env
        .engine
        .retry_repair(&ctx, &repair_id, temp.path(), &limits)
        .await
        .expect("retry 2");
    assert_eq!(after_second.state, RepairState::Queued);
    assert_eq!(after_second.attempts, 2);

    let (after_third, _) = env
        .engine
        .retry_repair(&ctx, &repair_id, temp.path(), &limits)
        .await
        .expect("retry 3");
    assert_eq!(after_third.state, RepairState::DeadLetter);
    assert_eq!(after_third.attempts, 3);

    // Budget gone: the next claim is a typed conflict, not a fourth attempt.
    let err = env
        .engine
        .retry_repair(&ctx, &repair_id, temp.path(), &limits)
        .await
        .expect_err("dead-lettered entry must not retry");
    assert!(
        err.to_string().contains("terminal"),
        "unexpected error: {err}"
    );

    // Dead-letter queue is backend-visible.
    let dead = env
        .engine
        .store()
        .list_repair_entries(&workspace_id, Some(RepairState::DeadLetter), 10)
        .await
        .expect("list dead letters");
    assert_eq!(dead.len(), 1);
    assert_eq!(dead[0].repair_id, repair_id);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt094_refailing_source_updates_open_entry_instead_of_multiplying_rows() {
    let Some(env) = ingestion_pg().await else {
        eprintln!("SKIP mt094_refailing_source: no PostgreSQL");
        return;
    };
    let workspace_id = env.pg.create_workspace().await;
    let ctx = test_ctx("mt094-dedupe");
    let root = register_root(
        &env,
        &ctx,
        &workspace_id,
        "",
        KnowledgeRootKind::MediaLibrary,
    )
    .await;

    let temp = tempfile::tempdir().expect("temp dir");
    write(temp.path(), "loop.srt", b"broken\nforever\n");

    let limits = IngestionLimits::default();
    for _ in 0..3 {
        env.engine
            .run_ingestion_pass(&ctx, &root.root_id, temp.path(), &limits)
            .await
            .expect("pass");
    }
    let open = env
        .engine
        .store()
        .list_repair_entries(&workspace_id, Some(RepairState::Queued), 10)
        .await
        .expect("list open repairs");
    assert_eq!(
        open.len(),
        1,
        "one OPEN entry per source, refreshed in place (partial unique index)"
    );
}
