//! WP-KERNEL-009 SourceIngestionAndEvidence extraction-pipeline proofs
//! against REAL Handshake-managed PostgreSQL: MT-084 (ContentHashStrategy),
//! MT-085 (ExtractionReceiptModel), MT-086 (PdfTextLayerDetector), MT-087
//! (PdfTranscriptImportPath), MT-088 (MediaTranscriptIngestion), MT-089
//! (GovernanceArtifactIngestion), MT-090 (OperatorResearchNoteIngestion),
//! MT-091 (SecretRedactionPreflight).
//!
//! No mocks, no SQLite: every test drives the real `IngestionEngine` into a
//! fresh isolated schema and asserts durable receipts, spans, EventLedger
//! events, and per-source rollups.

mod knowledge_ingestion_support;
mod knowledge_pg_support;

use handshake_core::knowledge_ingestion::backpressure::IngestionLimits;
use handshake_core::knowledge_ingestion::engine::FileIngestOutcome;
use handshake_core::knowledge_ingestion::pdf::fixtures as pdf_fixtures;
use handshake_core::knowledge_ingestion::receipts::{ExtractionStatus, IngestionErrorClass};
use handshake_core::knowledge_ingestion::spans::SpanAnchor;
use handshake_core::storage::knowledge::KnowledgeRootKind;
use knowledge_ingestion_support::{ingestion_pg, register_root, test_ctx, IngestionPg};
use sqlx::Row;

async fn ingest(
    env: &IngestionPg,
    ctx: &handshake_core::knowledge_ingestion::engine::IngestionContext,
    root: &handshake_core::storage::knowledge::KnowledgeSourceRoot,
    rel_path: &str,
    bytes: &[u8],
) -> FileIngestOutcome {
    env.engine
        .ingest_file_bytes(
            ctx,
            root,
            rel_path,
            bytes,
            "KIRUN-test-token",
            &IngestionLimits::default(),
            false,
        )
        .await
        .expect("ingest file bytes")
}

// ---------------------------------------------------------------------------
// MT-084 ContentHashStrategy
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt084_same_content_two_paths_shares_fidelity_hash_and_records_text_hash() {
    let Some(env) = ingestion_pg().await else {
        eprintln!("SKIP mt084_same_content_two_paths: no PostgreSQL");
        return;
    };
    let workspace_id = env.pg.create_workspace().await;
    let ctx = test_ctx("mt084");
    let root = register_root(
        &env,
        &ctx,
        &workspace_id,
        "src",
        KnowledgeRootKind::ProjectRepo,
    )
    .await;

    let content = b"# Same\n\nidentical bytes at two paths\n";
    let a = ingest(&env, &ctx, &root, "docs_a.md", content).await;
    let b = ingest(&env, &ctx, &root, "docs_b.md", content).await;

    // Dedupe detection: identical raw bytes => identical fidelity hash on two
    // DISTINCT source rows (path is identity, hash is content).
    assert_ne!(a.source.source_id, b.source.source_id);
    assert_eq!(a.source.content_hash, b.source.content_hash);
    assert_eq!(a.receipt.content_hash, a.source.content_hash);

    // Line-ending flips keep the normalized text hash stable while the raw
    // fidelity hash changes (change-detection vs authority, MT-084).
    let crlf = ingest(
        &env,
        &ctx,
        &root,
        "docs_crlf.md",
        b"# Same\r\n\r\nline endings differ\r\n",
    )
    .await;
    let lf = ingest(
        &env,
        &ctx,
        &root,
        "docs_lf.md",
        b"# Same\n\nline endings differ\n",
    )
    .await;
    assert_ne!(crlf.source.content_hash, lf.source.content_hash);
    let crlf_text_hash = crlf.source.provenance["normalized_text_sha256"]
        .as_str()
        .expect("normalized text hash recorded in provenance")
        .to_string();
    let lf_text_hash = lf.source.provenance["normalized_text_sha256"]
        .as_str()
        .expect("normalized text hash recorded in provenance")
        .to_string();
    assert_eq!(crlf_text_hash, lf_text_hash);
}

// ---------------------------------------------------------------------------
// MT-085 ExtractionReceiptModel + MT-090 OperatorResearchNoteIngestion
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt085_mt090_markdown_note_persists_spans_receipt_ledger_event_and_rollup() {
    let Some(env) = ingestion_pg().await else {
        eprintln!("SKIP mt085_mt090_markdown_note: no PostgreSQL");
        return;
    };
    let workspace_id = env.pg.create_workspace().await;
    let ctx = test_ctx("mt085-note");
    let root = register_root(
        &env,
        &ctx,
        &workspace_id,
        "notes",
        KnowledgeRootKind::OperatorFolder,
    )
    .await;

    let note = "Intro paragraph.\n\n# Research\n\nSee [[Project Roadmap]] and [[WP-009|the packet]].\n\n## Details\n\nNested paragraph.\n";
    let outcome = ingest(&env, &ctx, &root, "research/pdf_crates.md", note.as_bytes()).await;

    // Receipt: typed success, extractor identity, no error class.
    assert_eq!(outcome.receipt.status, ExtractionStatus::Success);
    assert_eq!(outcome.receipt.error_class, None);
    assert_eq!(outcome.receipt.extractor_id, "operator_research_note");
    assert_eq!(outcome.receipt.spans_produced as usize, outcome.spans.len());
    assert!(outcome.receipt.duration_ms >= 0);

    // MT-090: heading-path anchored spans + wikilink candidates recorded.
    assert!(outcome.spans.len() >= 3);
    let research_span = outcome
        .spans
        .iter()
        .find(|s| s.content.contains("Project Roadmap"))
        .expect("research paragraph span");
    match &research_span.anchor {
        SpanAnchor::LineRange { heading_path, .. } => {
            assert_eq!(heading_path, &vec!["Research".to_string()]);
        }
        other => panic!("expected line_range anchor, got {other:?}"),
    }
    let links = research_span
        .link_candidates
        .as_array()
        .expect("links array");
    assert_eq!(links.len(), 2);
    assert_eq!(links[0]["target"], "Project Roadmap");
    assert_eq!(links[1]["target"], "WP-009");
    assert_eq!(links[1]["label"], "the packet");

    // MT-090: notes are non-normative context until promoted.
    assert_eq!(
        outcome.source.provenance["authority"],
        "non_normative_context"
    );

    // MT-085: the receipt is bound to a replayable EventLedger event carrying
    // actor/session/correlation identity.
    let event_id = outcome
        .receipt
        .receipt_event_id
        .as_deref()
        .expect("receipt must carry a ledger event");
    let mut conn = env.pg.raw_connection().await;
    let row = sqlx::query(
        "SELECT actor_id, session_run_id, correlation_id, payload
         FROM kernel_event_ledger WHERE event_id = $1",
    )
    .bind(event_id)
    .fetch_one(&mut conn)
    .await
    .expect("ledger event exists");
    assert_eq!(row.get::<String, _>("actor_id"), ctx.actor.actor_id());
    assert_eq!(row.get::<String, _>("session_run_id"), ctx.session_run_id);
    assert_eq!(
        row.get::<Option<String>, _>("correlation_id"),
        ctx.correlation_id
    );
    let payload: serde_json::Value = row.get("payload");
    assert_eq!(payload["kind"], "extraction_receipt");
    assert_eq!(payload["status"], "success");

    // Per-source rollup advanced (spec 2.3.13.11 last-index receipt).
    assert_eq!(outcome.source.parser_status.as_str(), "parsed");
    assert_eq!(outcome.source.extraction_status.as_str(), "extracted");
    assert_eq!(
        outcome.source.last_index_receipt_event_id.as_deref(),
        Some(event_id)
    );
}

// ---------------------------------------------------------------------------
// MT-086 PdfTextLayerDetector + MT-087 PdfTranscriptImportPath
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt086_mt087_pdf_text_layer_extracts_page_spans() {
    let Some(env) = ingestion_pg().await else {
        eprintln!("SKIP mt086_mt087_pdf_text_layer: no PostgreSQL");
        return;
    };
    let workspace_id = env.pg.create_workspace().await;
    let ctx = test_ctx("mt087-pdf");
    let root = register_root(
        &env,
        &ctx,
        &workspace_id,
        "docs",
        KnowledgeRootKind::ProjectRepo,
    )
    .await;

    // Real PDF generated through the same library the detector parses.
    let bytes = pdf_fixtures::text_pdf(&["Alpha page one content", "Beta page two content"]);
    let outcome = ingest(&env, &ctx, &root, "manual.pdf", &bytes).await;

    assert_eq!(outcome.receipt.status, ExtractionStatus::Success);
    assert_eq!(outcome.receipt.extractor_id, "pdf_text_layer");
    assert_eq!(outcome.spans.len(), 2);
    for (index, expected_page) in [(0usize, 1u32), (1, 2)] {
        match &outcome.spans[index].anchor {
            SpanAnchor::PdfPage { page } => assert_eq!(*page, expected_page),
            other => panic!("expected pdf_page anchor, got {other:?}"),
        }
    }
    assert!(outcome.spans[0].content.contains("Alpha page one content"));
    assert!(outcome.spans[1].content.contains("Beta page two content"));
    assert!(outcome.repair.is_none(), "success must not enqueue repair");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt086_image_only_pdf_yields_typed_no_text_layer_and_repair_entry() {
    let Some(env) = ingestion_pg().await else {
        eprintln!("SKIP mt086_image_only_pdf: no PostgreSQL");
        return;
    };
    let workspace_id = env.pg.create_workspace().await;
    let ctx = test_ctx("mt086-imageonly");
    let root = register_root(
        &env,
        &ctx,
        &workspace_id,
        "docs",
        KnowledgeRootKind::ProjectRepo,
    )
    .await;

    let bytes = pdf_fixtures::image_only_pdf(2);
    let outcome = ingest(&env, &ctx, &root, "scanned.pdf", &bytes).await;

    // Image-only is NEVER an empty success: typed failure + OCR guidance.
    assert_eq!(outcome.receipt.status, ExtractionStatus::Failed);
    assert_eq!(
        outcome.receipt.error_class,
        Some(IngestionErrorClass::NoTextLayer)
    );
    assert!(outcome.spans.is_empty());
    let detail = outcome
        .receipt
        .error_detail
        .as_ref()
        .expect("repairable detail");
    assert!(
        detail.to_string().contains("OCR_NEEDED"),
        "detail must carry OCR guidance: {detail}"
    );
    assert_eq!(outcome.source.parser_status.as_str(), "failed");

    // MT-094 linkage: a repairable failure lands in the durable queue.
    let repair = outcome.repair.as_ref().expect("repair entry enqueued");
    assert_eq!(repair.state.as_str(), "queued");
    assert_eq!(repair.reason_class.as_str(), "NO_TEXT_LAYER");
    assert_eq!(
        repair.receipt_id.as_deref(),
        Some(outcome.receipt.receipt_id.as_str())
    );

    // Garbage bytes under .pdf: typed parse failure, never a panic.
    let garbage = ingest(&env, &ctx, &root, "broken.pdf", b"not a pdf at all").await;
    assert_eq!(garbage.receipt.status, ExtractionStatus::Failed);
    assert_eq!(
        garbage.receipt.error_class,
        Some(IngestionErrorClass::ParseError)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt087_mixed_pdf_extracts_partially_with_explicit_page_failures() {
    let Some(env) = ingestion_pg().await else {
        eprintln!("SKIP mt087_mixed_pdf: no PostgreSQL");
        return;
    };
    let workspace_id = env.pg.create_workspace().await;
    let ctx = test_ctx("mt087-mixed");
    let root = register_root(
        &env,
        &ctx,
        &workspace_id,
        "docs",
        KnowledgeRootKind::ProjectRepo,
    )
    .await;

    let bytes = pdf_fixtures::build_pdf(&[
        pdf_fixtures::FixturePage::Text("Readable first page".to_string()),
        pdf_fixtures::FixturePage::ImageOnly,
        pdf_fixtures::FixturePage::Text("Readable third page".to_string()),
    ]);
    let outcome = ingest(&env, &ctx, &root, "mixed.pdf", &bytes).await;

    // Partial is explicit: spans for readable pages, receipt names the loss.
    assert_eq!(outcome.receipt.status, ExtractionStatus::Partial);
    assert_eq!(
        outcome.receipt.error_class,
        Some(IngestionErrorClass::NoTextLayer)
    );
    assert_eq!(outcome.spans.len(), 2);
    assert_eq!(outcome.receipt.spans_failed, 1);
    let detail = outcome.receipt.error_detail.as_ref().expect("detail");
    assert_eq!(detail["failed_pages"][0]["page"], 2);
    // Partial extraction is repairable (OCR the failed pages).
    assert!(outcome.repair.is_some());
}

// ---------------------------------------------------------------------------
// MT-088 MediaTranscriptIngestion
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt088_transcripts_become_time_coded_spans_with_partial_on_malformed_cues() {
    let Some(env) = ingestion_pg().await else {
        eprintln!("SKIP mt088_transcripts: no PostgreSQL");
        return;
    };
    let workspace_id = env.pg.create_workspace().await;
    let ctx = test_ctx("mt088");
    let root = register_root(
        &env,
        &ctx,
        &workspace_id,
        "media",
        KnowledgeRootKind::MediaLibrary,
    )
    .await;

    // SRT: well-formed.
    let srt = "1\n00:00:01,000 --> 00:00:04,000\nFirst cue\n\n2\n00:00:04,500 --> 00:00:06,000\nSecond cue\n";
    let outcome = ingest(&env, &ctx, &root, "episode1.srt", srt.as_bytes()).await;
    assert_eq!(outcome.receipt.status, ExtractionStatus::Success);
    assert_eq!(outcome.spans.len(), 2);
    match &outcome.spans[0].anchor {
        SpanAnchor::MediaTime {
            start_ms,
            end_ms,
            cue_index,
        } => {
            assert_eq!((*start_ms, *end_ms, *cue_index), (1000, 4000, 0));
        }
        other => panic!("expected media_time anchor, got {other:?}"),
    }

    // VTT with one malformed cue: partial receipt naming the loss.
    let vtt = "WEBVTT\n\n00:01.000 --> 00:04.000\nGood cue\n\nbroken timing line\nLost cue\n";
    let outcome = ingest(&env, &ctx, &root, "episode2.vtt", vtt.as_bytes()).await;
    assert_eq!(outcome.receipt.status, ExtractionStatus::Partial);
    assert_eq!(
        outcome.receipt.error_class,
        Some(IngestionErrorClass::MalformedCue)
    );
    assert_eq!(outcome.spans.len(), 1);
    assert_eq!(outcome.receipt.spans_failed, 1);
    let detail = outcome.receipt.error_detail.as_ref().expect("detail");
    assert_eq!(detail["malformed_cues"].as_array().expect("cues").len(), 1);
    assert!(outcome.repair.is_some(), "malformed cues are repairable");

    // JSON transcript artifact (whisper-style shape).
    let json_artifact = r#"{"segments": [{"start": 0.5, "end": 2.25, "text": "hello"}]}"#;
    let outcome = ingest(
        &env,
        &ctx,
        &root,
        "episode3.transcript.json",
        json_artifact.as_bytes(),
    )
    .await;
    assert_eq!(outcome.receipt.status, ExtractionStatus::Success);
    assert_eq!(outcome.spans.len(), 1);
    match &outcome.spans[0].anchor {
        SpanAnchor::MediaTime {
            start_ms, end_ms, ..
        } => {
            assert_eq!((*start_ms, *end_ms), (500, 2250));
        }
        other => panic!("expected media_time anchor, got {other:?}"),
    }

    // Fully malformed artifact: typed whole-file failure.
    let outcome = ingest(&env, &ctx, &root, "garbage.srt", b"no cues at all here").await;
    assert_eq!(outcome.receipt.status, ExtractionStatus::Failed);
    assert_eq!(
        outcome.receipt.error_class,
        Some(IngestionErrorClass::ParseError)
    );
}

// ---------------------------------------------------------------------------
// MT-089 GovernanceArtifactIngestion
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt089_governance_artifacts_get_json_pointer_spans_and_sub_kinds() {
    let Some(env) = ingestion_pg().await else {
        eprintln!("SKIP mt089_governance_artifacts: no PostgreSQL");
        return;
    };
    let workspace_id = env.pg.create_workspace().await;
    let ctx = test_ctx("mt089");
    let root = register_root(
        &env,
        &ctx,
        &workspace_id,
        "governance",
        KnowledgeRootKind::Governance,
    )
    .await;

    // Tiny SYNTHETIC contract-shaped artifact (never the live .GOV).
    let artifact = r#"{
        "mt_id": "MT-TEST",
        "scope": {"constraints": ["no sqlite", "no docker"], "title": "synthetic"},
        "lifecycle": {"status": "PENDING"}
    }"#;
    let outcome = ingest(
        &env,
        &ctx,
        &root,
        "task_packets/WP-TEST/MT-TEST.json",
        artifact.as_bytes(),
    )
    .await;

    assert_eq!(outcome.receipt.status, ExtractionStatus::Success);
    assert_eq!(
        outcome.source.provenance["governance_sub_kind"],
        "mt_contract"
    );
    let pointers: Vec<String> = outcome
        .spans
        .iter()
        .map(|s| match &s.anchor {
            SpanAnchor::JsonPointer { pointer, .. } => pointer.clone(),
            other => panic!("expected json_pointer anchor, got {other:?}"),
        })
        .collect();
    for expected in [
        "/mt_id",
        "/scope",
        "/scope/constraints",
        "/lifecycle/status",
    ] {
        assert!(
            pointers.iter().any(|p| p == expected),
            "missing pointer {expected}: {pointers:?}"
        );
    }

    // JSONL: line-anchored documents, malformed lines counted not fatal.
    let jsonl = "{\"receipt\": 1}\nnot json\n{\"receipt\": 2}\n";
    let outcome = ingest(&env, &ctx, &root, "receipts/ledger.jsonl", jsonl.as_bytes()).await;
    assert_eq!(outcome.receipt.status, ExtractionStatus::Partial);
    assert_eq!(outcome.spans.len(), 2);
    match &outcome.spans[1].anchor {
        SpanAnchor::JsonPointer { jsonl_line, .. } => assert_eq!(*jsonl_line, Some(2)),
        other => panic!("expected json_pointer anchor, got {other:?}"),
    }

    // Malformed JSON artifact: typed parse failure + repair entry.
    let outcome = ingest(&env, &ctx, &root, "task_packets/broken.json", b"{broken").await;
    assert_eq!(outcome.receipt.status, ExtractionStatus::Failed);
    assert_eq!(
        outcome.receipt.error_class,
        Some(IngestionErrorClass::ParseError)
    );
    assert!(outcome.repair.is_some());
}

// ---------------------------------------------------------------------------
// MT-091 SecretRedactionPreflight
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt091_high_severity_secrets_block_the_file_and_store_no_raw_bytes() {
    let Some(env) = ingestion_pg().await else {
        eprintln!("SKIP mt091_high_severity_secrets: no PostgreSQL");
        return;
    };
    let workspace_id = env.pg.create_workspace().await;
    let ctx = test_ctx("mt091-block");
    let root = register_root(
        &env,
        &ctx,
        &workspace_id,
        "src",
        KnowledgeRootKind::ProjectRepo,
    )
    .await;

    // FAKE fixture credentials (AWS documented example key + dummy key block).
    let fake_aws_key = "AKIAIOSFODNN7EXAMPLE";
    let secret_file = format!(
        "config:\n  aws_key = {fake_aws_key}\n-----BEGIN RSA PRIVATE KEY-----\nMIIfake+material\n-----END RSA PRIVATE KEY-----\n"
    );
    let outcome = ingest(&env, &ctx, &root, "ops/config.md", secret_file.as_bytes()).await;

    // Blocked, typed, counted.
    assert_eq!(outcome.receipt.status, ExtractionStatus::Blocked);
    assert_eq!(
        outcome.receipt.error_class,
        Some(IngestionErrorClass::SecretBlocked)
    );
    assert!(outcome.spans.is_empty(), "blocked file must store NO spans");
    assert!(outcome.receipt.redaction_count >= 2);
    assert_eq!(outcome.source.redaction_state.as_str(), "redacted");
    // SECRET_BLOCKED is not machine-repairable: no repair-queue entry.
    assert!(outcome.repair.is_none());

    // The raw secret never reaches ANY durable ingestion row.
    let mut conn = env.pg.raw_connection().await;
    for (table, column) in [
        ("knowledge_ingestion_spans", "content"),
        ("knowledge_ingestion_receipts", "error_detail::text"),
        ("knowledge_sources", "provenance::text"),
        ("kernel_event_ledger", "payload::text"),
    ] {
        let count: i64 = sqlx::query_scalar(&format!(
            "SELECT count(*) FROM {table} WHERE {column} LIKE $1"
        ))
        .bind(format!("%{fake_aws_key}%"))
        .fetch_one(&mut conn)
        .await
        .expect("probe for raw secret");
        assert_eq!(count, 0, "raw secret leaked into {table}");
    }

    // Receipt detail records findings WITHOUT content (kind + location only).
    let detail = outcome.receipt.error_detail.as_ref().expect("detail");
    let findings = detail["findings"].as_array().expect("findings");
    assert!(findings.iter().any(|f| f["kind"] == "aws_access_key_id"));
    assert!(!detail.to_string().contains(fake_aws_key));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt091_medium_severity_secrets_redact_span_content_not_the_file() {
    let Some(env) = ingestion_pg().await else {
        eprintln!("SKIP mt091_medium_severity_secrets: no PostgreSQL");
        return;
    };
    let workspace_id = env.pg.create_workspace().await;
    let ctx = test_ctx("mt091-redact");
    let root = register_root(
        &env,
        &ctx,
        &workspace_id,
        "src",
        KnowledgeRootKind::ProjectRepo,
    )
    .await;

    let fake_token = "q7Xz2pLm9KvR4tNw8YbD3cFgH6sJaUeP";
    let file = format!("# Config notes\n\nThe service reads api_key = {fake_token} at startup.\n");
    let outcome = ingest(&env, &ctx, &root, "docs/config.md", file.as_bytes()).await;

    // Extraction succeeds; the secret region is rewritten, never stored raw.
    assert_eq!(outcome.receipt.status, ExtractionStatus::Success);
    assert!(outcome.receipt.redaction_count >= 1);
    assert_eq!(outcome.source.redaction_state.as_str(), "partial");
    let redacted_span = outcome
        .spans
        .iter()
        .find(|s| s.content.contains("[REDACTED:"))
        .expect("redacted span exists");
    assert_eq!(redacted_span.redaction_state.as_str(), "redacted");
    assert!(!redacted_span.content.contains(fake_token));

    let mut conn = env.pg.raw_connection().await;
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM knowledge_ingestion_spans WHERE content LIKE $1")
            .bind(format!("%{fake_token}%"))
            .fetch_one(&mut conn)
            .await
            .expect("probe spans for raw token");
    assert_eq!(
        count, 0,
        "raw medium-severity secret leaked into span content"
    );
}

// ---------------------------------------------------------------------------
// MT-082/MT-085 cross-check: unsupported formats skip typed, never guess.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unsupported_binary_is_skipped_typed_with_receipt() {
    let Some(env) = ingestion_pg().await else {
        eprintln!("SKIP unsupported_binary_is_skipped: no PostgreSQL");
        return;
    };
    let workspace_id = env.pg.create_workspace().await;
    let ctx = test_ctx("skip");
    let root = register_root(
        &env,
        &ctx,
        &workspace_id,
        "src",
        KnowledgeRootKind::ProjectRepo,
    )
    .await;

    let outcome = ingest(&env, &ctx, &root, "build/output.bin", &[0u8, 255, 19, 55]).await;
    assert_eq!(outcome.receipt.status, ExtractionStatus::Skipped);
    assert_eq!(
        outcome.receipt.error_class,
        Some(IngestionErrorClass::UnsupportedFormat)
    );
    assert!(outcome.spans.is_empty());
    assert!(
        outcome.repair.is_none(),
        "unsupported is permanent, not repairable"
    );
    // The source row still exists with hash + provenance (registered, not lost).
    assert_eq!(
        outcome.source.relative_path.as_deref(),
        Some("build/output.bin")
    );
}
