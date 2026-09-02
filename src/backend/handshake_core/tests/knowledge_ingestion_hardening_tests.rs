//! WP-KERNEL-009 SourceIngestionAndEvidence HARDENING proofs against the real
//! Handshake-managed embedded SurrealDB store (MT-081/082/085/086/087/091/094
//! #1-#10).
//!
//! Every active test drives the real `IngestionEngine` into a fresh isolated
//! embedded store and re-reads typed durable state. No mocks, fallback
//! authority, or raw database connections are involved. The fixtures below
//! carry FAKE credentials shaped like real ones.
//!
//! Coverage:
//! * #1/#4 (MT-091/096): a MEDIUM secret on the 120-line code-window seam is
//!   absent from ALL stored span content (whole-file redaction path). NOTE on
//!   the genuine boundary-SPLIT proof: the engine windows code by LINE, and a
//!   line is never cut across windows, while the MEDIUM secret regexes are all
//!   single-line -- so a secret whose BYTES straddle the seam (and would
//!   defeat a per-span rescan) cannot also be detected by the whole-file scan
//!   at the integration level. The byte-level split that genuinely breaks the
//!   per-span rescan is proved by the unit test
//!   `secrets::tests::whole_file_redaction_catches_boundary_split_secret`
//!   (manual byte-split: each fragment alone matches nothing, whole-file
//!   findings still excise both halves). This integration test proves the
//!   persisted outcome: the secret at the seam is redacted out of every
//!   stored span row.
//! * #2 (MT-091): each new pattern (github_pat_, xapp-, headerless base64) is
//!   caught and redacted; raw bytes never stored.
//! * #3 (MT-091): marker-bearing redaction is enforced before span replacement.
//! * #5 (MT-086/087): a garbage PDF degrades to ONE failed file, the pass
//!   completes and other files still ingest (catch_unwind guard).
//! * #6 (MT-086): an image-only PDF with tiny/invisible text -> NO_TEXT_LAYER.
//! * #8 (MT-085/094): persisted receipt and repair event references are observed
//!   through the inspector, and absent references reject before write.
//! * #7 (MT-094): re-failing after dead-letter REOPENS the terminal row for
//!   the same source+reason instead of inserting a new one.
//! * #10 (MT-091): .env / .pem paths are denied root registration.

#[path = "knowledge_ingestion_support.rs"]
mod knowledge_ingestion_support;

use std::path::Path;

use handshake_core::knowledge_ingestion::backpressure::IngestionLimits;
use handshake_core::knowledge_ingestion::engine::{
    FileIngestOutcome, IngestionContext, RootRegistrationRequest,
};
use handshake_core::knowledge_ingestion::pdf::fixtures as pdf_fixtures;
use handshake_core::knowledge_ingestion::receipts::{
    ExtractionStatus, IngestionErrorClass, NewExtractionReceipt,
};
use handshake_core::knowledge_ingestion::repair::{NewRepairEntry, RepairReason, RepairState};
use handshake_core::knowledge_ingestion::spans::{ExtractedSpan, SpanAnchor, SpanRedaction};
use handshake_core::knowledge_ingestion::IngestionError;
use handshake_core::storage::knowledge::{KnowledgeRootKind, KnowledgeSourceRoot, KnowledgeStore};
use handshake_core::storage::surreal::RowFilter;
use handshake_core::storage::Database;
use knowledge_ingestion_support::{
    open_embedded_ingestion_fixture, register_root, test_ctx, EmbeddedIngestionFixture,
};

async fn ingest(
    env: &EmbeddedIngestionFixture,
    ctx: &IngestionContext,
    root: &KnowledgeSourceRoot,
    rel_path: &str,
    bytes: &[u8],
) -> FileIngestOutcome {
    env.engine
        .ingest_file_bytes(
            ctx,
            root,
            rel_path,
            bytes,
            "KIRUN-hardening",
            &IngestionLimits::default(),
            false,
        )
        .await
        .expect("ingest file bytes")
}

/// Re-read the canonical embedded EventLedger rows for one ingestion session.
async fn persisted_event_payloads(
    env: &EmbeddedIngestionFixture,
    ctx: &IngestionContext,
) -> Vec<String> {
    env.engine
        .knowledge()
        .list_kernel_events_for_session(&ctx.session_run_id)
        .await
        .expect("read embedded EventLedger rows")
        .into_iter()
        .map(|event| event.payload.to_string())
        .collect()
}

fn write(dir: &Path, rel: &str, content: &[u8]) {
    let path = dir.join(rel);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir fixture tree");
    std::fs::write(path, content).expect("write runtime fixture file");
}

// ---------------------------------------------------------------------------
// #1 / #4 (MT-091 / MT-096): MEDIUM secret straddling the 120-line boundary.
// ---------------------------------------------------------------------------

/// A MEDIUM JWT on the 120-line code-window seam (line 120 of a 150-line
/// file). The engine windows code by 120 lines and redacts each byte-anchored
/// span using WHOLE-FILE findings (the #1 fix). This test proves the secret
/// is absent from EVERY stored span's content row across BOTH windows, the
/// source is marked partially redacted, and the redaction marker is present.
/// (The genuine byte-SPLIT-across-windows case is covered at the unit level --
/// see the module note above.)
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt091_mt096_boundary_seam_medium_secret_absent_from_all_spans() {
    let Some(env) = open_embedded_ingestion_fixture().await else {
        eprintln!("SKIP mt091_mt096_boundary_straddle: embedded store unavailable");
        return;
    };
    let workspace_id = env.store.create_workspace().await;
    let ctx = test_ctx("mt096-boundary");
    let root = register_root(
        &env,
        &ctx,
        &workspace_id,
        "src",
        KnowledgeRootKind::ProjectRepo,
    )
    .await;

    // FAKE JWT (medium severity -> redact, not block).
    let jwt = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";

    // Build a 150-line Rust file with the JWT on line 120 (the last line of
    // window 0, which holds lines 1..=120; window 1 holds 121..=150). A
    // per-window rescan that ever truncated the seam, or a redaction pass that
    // skipped a window, would leave the JWT (or a fragment) in a stored row;
    // the whole-file redaction must excise it from every window.
    // The secret is bound to a NON-keyword identifier (`session_jwt`, not
    // `api_token`/`secret`/...) so the JWT pattern is what fires -- a
    // secret-keyword assignment would instead match the broader
    // generic-high-entropy rule and overlap-merge would keep THAT kind's
    // marker, obscuring which detector caught the boundary token.
    let mut file = String::new();
    for i in 1..=119 {
        file.push_str(&format!("// filler comment line {i:03}\n"));
    }
    file.push_str(&format!(
        "let session_jwt = \"{jwt}\"; // sensitive line 120 on the window seam\n"
    ));
    for i in 121..=150 {
        file.push_str(&format!("// filler comment line {i:03}\n"));
    }

    let outcome = ingest(&env, &ctx, &root, "boundary_secret.rs", file.as_bytes()).await;

    // Code files succeed; the secret region is rewritten in place.
    assert_eq!(outcome.receipt.status, ExtractionStatus::Success);
    assert!(
        outcome.receipt.redaction_count >= 1,
        "the boundary JWT must have been redacted at least once"
    );
    assert_eq!(outcome.source.redaction_state.as_str(), "partial");

    // More than one span: the file genuinely crossed the 120-line window.
    assert!(
        outcome.spans.len() >= 2,
        "a >120-line file must produce multiple windows: {} span(s)",
        outcome.spans.len()
    );
    // The JWT pattern fired specifically (no secret-keyword overlap), so the
    // marker names json_web_token; the redacted span carries it.
    let redacted_span = outcome
        .spans
        .iter()
        .find(|s| s.content.contains("[REDACTED:json_web_token]"))
        .unwrap_or_else(|| {
            panic!(
                "a span must carry the JWT redaction marker; span markers seen: {:?}",
                outcome
                    .spans
                    .iter()
                    .map(|s| (s.span_index, s.redaction_state.as_str()))
                    .collect::<Vec<_>>()
            )
        });
    assert_eq!(redacted_span.redaction_state.as_str(), "redacted");

    // CORE assertion: re-read the canonical embedded rows and prove the raw
    // JWT and each contiguous fragment are absent from every durable surface.
    let stored_spans = env
        .engine
        .store()
        .list_source_spans(&outcome.source.source_id)
        .await
        .expect("re-read embedded span rows");
    assert_eq!(stored_spans.len(), outcome.spans.len());
    for fragment in [jwt, &jwt[..30], &jwt[30..60], &jwt[60..]] {
        assert!(
            stored_spans
                .iter()
                .all(|span| !span.content.contains(fragment)),
            "boundary JWT material leaked into an embedded span content row: {fragment}"
        );
    }

    let inspector = env.store.storage.test_inspector();
    let span_table = inspector
        .table_selector("knowledge_ingestion_spans")
        .await
        .expect("select ingestion span table");
    let span_content = span_table.field("content").expect("select span content");
    let span_redaction = span_table
        .field("redaction_state")
        .expect("select span redaction state");
    let inspected_spans = inspector
        .project(&span_table, &[span_content, span_redaction], RowFilter::All)
        .await
        .expect("inspect persisted ingestion spans");
    assert_eq!(inspected_spans.len(), outcome.spans.len());
    for row in &inspected_spans {
        let content = row
            .values
            .get("content")
            .and_then(serde_json::Value::as_str)
            .expect("inspected span content");
        for fragment in [jwt, &jwt[..30], &jwt[30..60], &jwt[60..]] {
            assert!(
                !content.contains(fragment),
                "boundary JWT material leaked into inspector-observed span {}: {fragment}",
                row.record_id.key_string().unwrap_or("<non-string-id>")
            );
        }
        if row.values.get("redaction_state") == Some(&serde_json::json!("redacted")) {
            assert!(
                content.contains("[REDACTED:"),
                "inspector observed redacted span without a marker: {:?}",
                row.record_id
            );
        }
    }

    let receipts = env
        .engine
        .store()
        .list_extraction_receipts(&outcome.source.source_id, 10)
        .await
        .expect("re-read embedded extraction receipt rows");
    assert!(
        receipts
            .iter()
            .all(|receipt| !serde_json::to_string(receipt)
                .expect("serialize embedded receipt")
                .contains(jwt)),
        "raw boundary JWT leaked into an embedded receipt row"
    );

    let source = env
        .engine
        .knowledge()
        .get_knowledge_source(&outcome.source.source_id)
        .await
        .expect("re-read embedded source row")
        .expect("source row exists");
    assert!(
        !source.provenance.to_string().contains(jwt),
        "raw boundary JWT leaked into embedded source provenance"
    );

    for payload in persisted_event_payloads(&env, &ctx).await {
        assert!(
            !payload.contains(jwt),
            "raw boundary JWT leaked into an embedded EventLedger payload"
        );
    }
}

// ---------------------------------------------------------------------------
// #2 (MT-091): new secret patterns are caught + redacted, never stored raw.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt091_new_patterns_github_pat_slack_app_headerless_blob_redacted() {
    let Some(env) = open_embedded_ingestion_fixture().await else {
        eprintln!("SKIP mt091_new_patterns: embedded store unavailable");
        return;
    };
    let workspace_id = env.store.create_workspace().await;
    let ctx = test_ctx("mt091-newpat");
    let root = register_root(
        &env,
        &ctx,
        &workspace_id,
        "src",
        KnowledgeRootKind::ProjectRepo,
    )
    .await;

    // (a) Fine-grained GitHub PAT (MEDIUM -> redact). FAKE shape:
    // github_pat_ + 22 + _ + 59 base62.
    let pat_body22 = "A1b2C3d4E5f6G7h8I9j0K1";
    let pat_body59 = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456";
    let github_pat = format!("github_pat_{pat_body22}_{pat_body59}");
    let md_pat = format!("# Notes\n\nCI reads token = {github_pat} on startup.\n");
    let out_pat = ingest(&env, &ctx, &root, "docs/pat.md", md_pat.as_bytes()).await;
    assert_eq!(out_pat.receipt.status, ExtractionStatus::Success);
    assert!(out_pat.receipt.redaction_count >= 1, "PAT must be redacted");
    assert!(out_pat
        .spans
        .iter()
        .any(|s| s.content.contains("[REDACTED:github_fine_grained_pat]")));
    let persisted_pat_spans = env
        .engine
        .store()
        .list_source_spans(&out_pat.source.source_id)
        .await
        .expect("re-read embedded PAT span rows");
    assert!(
        persisted_pat_spans
            .iter()
            .all(|span| !span.content.contains(pat_body59)),
        "raw github_pat body leaked into an embedded span content row"
    );

    // (b) Slack app-level token xapp- (MEDIUM -> redact). FAKE.
    let xapp_body = "A012BCDEFGH-1234567890-abcdef0123456789";
    let xapp = format!("xapp-1-{xapp_body}");
    let md_xapp = format!("# Slack\n\nslack_app_token = {xapp}\n");
    let out_xapp = ingest(&env, &ctx, &root, "docs/slack.md", md_xapp.as_bytes()).await;
    assert_eq!(out_xapp.receipt.status, ExtractionStatus::Success);
    assert!(
        out_xapp.receipt.redaction_count >= 1,
        "xapp must be redacted"
    );
    assert!(out_xapp
        .spans
        .iter()
        .any(|s| s.content.contains("[REDACTED:slack_app_token]")));
    let persisted_xapp_spans = env
        .engine
        .store()
        .list_source_spans(&out_xapp.source.source_id)
        .await
        .expect("re-read embedded Slack span rows");
    assert!(
        persisted_xapp_spans
            .iter()
            .all(|span| !span.content.contains(xapp_body)),
        "raw xapp token leaked into an embedded span content row"
    );

    // (c) Headerless base64 key blob (HIGH -> BLOCK; no spans stored at all).
    // A standalone high-entropy base64 line with no PEM armor (FAKE).
    let blob = "MIIBVAIBADANBgkqhkiG9w0BAQEFAASCAT4wggE6AgEAAkEA3Tn7HkQxZpLm9KvR4tNw8YbD3cFgH6sJaUePq7Xz2pLm9KvR4tNwQ==";
    let md_blob = format!("key dump:\n{blob}\nend of dump\n");
    let out_blob = ingest(&env, &ctx, &root, "ops/keydump.md", md_blob.as_bytes()).await;
    assert_eq!(
        out_blob.receipt.status,
        ExtractionStatus::Blocked,
        "headerless key blob is HIGH severity -> block"
    );
    assert_eq!(
        out_blob.receipt.error_class,
        Some(IngestionErrorClass::SecretBlocked)
    );
    assert!(
        out_blob.spans.is_empty(),
        "blocked file stores no span content"
    );
    // The finding kind is recorded WITHOUT leaking the blob bytes.
    let detail = out_blob.receipt.error_detail.as_ref().expect("detail");
    assert!(detail.to_string().contains("headerless_key_blob"));
    assert!(!detail.to_string().contains(blob));
    // The blob never lands anywhere durable. Re-read each canonical typed
    // surface available through the embedded support.
    let persisted_blob_spans = env
        .engine
        .store()
        .list_source_spans(&out_blob.source.source_id)
        .await
        .expect("re-read embedded blocked-file spans");
    assert!(persisted_blob_spans.is_empty());
    let persisted_blob_receipts = env
        .engine
        .store()
        .list_extraction_receipts(&out_blob.source.source_id, 10)
        .await
        .expect("re-read embedded blocked-file receipts");
    assert!(
        persisted_blob_receipts
            .iter()
            .all(|receipt| !serde_json::to_string(receipt)
                .expect("serialize embedded blocked-file receipt")
                .contains(blob)),
        "raw headerless key blob leaked into an embedded receipt row"
    );
    for payload in persisted_event_payloads(&env, &ctx).await {
        assert!(
            !payload.contains(blob),
            "raw headerless key blob leaked into an embedded EventLedger payload"
        );
    }

    let inspector = env.store.storage.test_inspector();
    let span_table = inspector
        .table_selector("knowledge_ingestion_spans")
        .await
        .expect("select ingestion span table");
    let span_content = span_table.field("content").expect("select span content");
    let span_redaction = span_table
        .field("redaction_state")
        .expect("select span redaction state");
    let inspected_spans = inspector
        .project(&span_table, &[span_content, span_redaction], RowFilter::All)
        .await
        .expect("inspect persisted pattern-test spans");
    for row in inspected_spans {
        let content = row
            .values
            .get("content")
            .and_then(serde_json::Value::as_str)
            .expect("inspected pattern-test span content");
        for secret in [pat_body59, xapp_body, blob] {
            assert!(
                !content.contains(secret),
                "secret material leaked into inspector-observed span {}",
                row.record_id.key_string().unwrap_or("<non-string-id>")
            );
        }
        if row.values.get("redaction_state") == Some(&serde_json::json!("redacted")) {
            assert!(
                content.contains("[REDACTED:"),
                "inspector observed redacted span without a marker: {:?}",
                row.record_id
            );
        }
    }
}

// ---------------------------------------------------------------------------
// #3 (MT-091): persisted redaction-marker invariant.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt091_markerless_redacted_span_is_rejected_without_replacing_durable_spans() {
    let Some(env) = open_embedded_ingestion_fixture().await else {
        eprintln!("SKIP mt091_markerless_redacted_span: embedded store unavailable");
        return;
    };
    let workspace_id = env.store.create_workspace().await;
    let ctx = test_ctx("mt091-markerless");
    let root = register_root(
        &env,
        &ctx,
        &workspace_id,
        "src",
        KnowledgeRootKind::ProjectRepo,
    )
    .await;
    let outcome = ingest(&env, &ctx, &root, "docs/plain.md", b"# Plain\n\nbody\n").await;
    let before = env
        .engine
        .store()
        .list_source_spans(&outcome.source.source_id)
        .await
        .expect("read original spans");

    let mut invalid = ExtractedSpan::new(
        SpanAnchor::LineRange {
            line_start: 1,
            line_end: 1,
            heading_path: Vec::new(),
        },
        "api_key = raw-secret-material",
    );
    invalid.redaction = SpanRedaction::Redacted;
    let error = env
        .engine
        .store()
        .replace_source_spans(
            &workspace_id,
            &outcome.source.source_id,
            &outcome.receipt.receipt_id,
            &[invalid],
        )
        .await
        .expect_err("markerless redacted span must be rejected");
    assert!(matches!(error, IngestionError::Validation(_)));

    let after = env
        .engine
        .store()
        .list_source_spans(&outcome.source.source_id)
        .await
        .expect("re-read original spans");
    assert_eq!(
        after, before,
        "validation must precede destructive replacement"
    );
}

// ---------------------------------------------------------------------------
// #8 (MT-085 / MT-094): ledger-event refs on embedded engine paths.
// ---------------------------------------------------------------------------

/// Positive-path coverage only: the real engine persists event references on
/// receipt and repair rows, and the inspector independently resolves both
/// references to the exact EventLedger records. The negative absent-reference
/// invariant remains separately dispositioned below.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt085_mt094_engine_paths_persist_event_refs_observed_by_inspector() {
    let Some(env) = open_embedded_ingestion_fixture().await else {
        eprintln!("SKIP mt085_mt094_ledger_refs: embedded store unavailable");
        return;
    };
    let workspace_id = env.store.create_workspace().await;
    let ctx = test_ctx("mt085-notnull");
    let root = register_root(
        &env,
        &ctx,
        &workspace_id,
        "src",
        KnowledgeRootKind::ProjectRepo,
    )
    .await;
    // A real source to satisfy FKs.
    let outcome = ingest(&env, &ctx, &root, "docs/plain.md", b"# Plain\n\nbody\n").await;

    // Positive receipt path: the real engine mints and stores the event before
    // the typed receipt row is written.
    assert!(outcome.receipt.receipt_event_id.is_some());

    // Positive repair path: a failed extraction still receives a receipt event
    // and the repair row stores the same event reference.
    let failed = ingest(
        &env,
        &ctx,
        &root,
        "docs/stuck.srt",
        b"garbage\nwithout timing\n",
    )
    .await;
    assert_eq!(failed.receipt.status, ExtractionStatus::Failed);
    assert!(failed.receipt.receipt_event_id.is_some());
    assert!(
        failed
            .repair
            .as_ref()
            .and_then(|repair| repair.enqueue_event_id.as_ref())
            .is_some(),
        "repair enqueue must carry the receipt EventLedger reference"
    );

    let inspector = env.store.storage.test_inspector();
    let event_table = inspector
        .table_selector("kernel_event_ledger")
        .await
        .expect("select EventLedger table");
    let references = inspector
        .references_to(&event_table)
        .await
        .expect("inspect EventLedger references");
    let receipt_event_reference = references
        .iter()
        .find(|reference| {
            reference.source_table() == "knowledge_ingestion_receipts"
                && reference.source_field() == "receipt_event_id"
        })
        .expect("ingestion receipt event reference")
        .clone();
    let repair_event_reference = references
        .iter()
        .find(|reference| {
            reference.source_table() == "knowledge_ingestion_repair_queue"
                && reference.source_field() == "enqueue_event_id"
        })
        .expect("repair enqueue event reference")
        .clone();

    for receipt in [&outcome.receipt, &failed.receipt] {
        let referenced = inspector
            .referenced_ids(
                &receipt_event_reference,
                RowFilter::IdEquals(receipt.receipt_id.clone()),
            )
            .await
            .expect("inspect persisted receipt event reference");
        assert_eq!(referenced.len(), 1);
        assert_eq!(
            referenced[0].key_string(),
            receipt.receipt_event_id.as_deref(),
            "persisted receipt must reference its exact EventLedger event"
        );
    }
    let repair = failed.repair.as_ref().expect("failed-file repair row");
    let referenced = inspector
        .referenced_ids(
            &repair_event_reference,
            RowFilter::IdEquals(repair.repair_id.clone()),
        )
        .await
        .expect("inspect persisted repair event reference");
    assert_eq!(referenced.len(), 1);
    assert_eq!(
        referenced[0].key_string(),
        repair.enqueue_event_id.as_deref(),
        "persisted repair must reference its exact EventLedger event"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt085_mt094_absent_event_references_are_rejected_before_write() {
    let Some(env) = open_embedded_ingestion_fixture().await else {
        eprintln!("SKIP mt085_mt094_absent_event_refs: embedded store unavailable");
        return;
    };
    let workspace_id = env.store.create_workspace().await;
    let ctx = test_ctx("mt085-absent-event");
    let root = register_root(
        &env,
        &ctx,
        &workspace_id,
        "src",
        KnowledgeRootKind::ProjectRepo,
    )
    .await;
    let outcome = ingest(&env, &ctx, &root, "docs/plain.md", b"# Plain\n\nbody\n").await;
    let receipt_count = env
        .engine
        .store()
        .list_extraction_receipts(&outcome.source.source_id, 100)
        .await
        .expect("count receipts before invalid write")
        .len();

    let receipt_error = env
        .engine
        .store()
        .record_extraction_receipt(
            NewExtractionReceipt {
                workspace_id: workspace_id.clone(),
                source_id: outcome.source.source_id.clone(),
                ingestion_run_token: Some("KIRUN-absent-event".to_owned()),
                extractor_id: "hardening-proof".to_owned(),
                extractor_version: "1".to_owned(),
                status: ExtractionStatus::Success,
                error_class: None,
                error_detail: None,
                spans_produced: 0,
                spans_failed: 0,
                redaction_count: 0,
                content_hash: outcome.source.content_hash.clone(),
                duration_ms: 1,
            },
            None,
        )
        .await
        .expect_err("receipt without event reference must be rejected");
    assert!(matches!(receipt_error, IngestionError::Validation(_)));
    assert_eq!(
        env.engine
            .store()
            .list_extraction_receipts(&outcome.source.source_id, 100)
            .await
            .expect("count receipts after invalid write")
            .len(),
        receipt_count
    );

    let repairs_before = env
        .engine
        .store()
        .list_repair_entries(&workspace_id, None, 100)
        .await
        .expect("count repairs before invalid write")
        .len();
    let repair_error = env
        .engine
        .store()
        .enqueue_repair(NewRepairEntry {
            workspace_id: workspace_id.clone(),
            source_id: outcome.source.source_id,
            receipt_id: Some(outcome.receipt.receipt_id),
            reason_class: RepairReason::StaleHash,
            reason_detail: serde_json::json!({"proof":"missing-event"}),
            max_attempts: 1,
            enqueue_event_id: None,
        })
        .await
        .expect_err("repair without event reference must be rejected");
    assert!(matches!(repair_error, IngestionError::Validation(_)));
    assert_eq!(
        env.engine
            .store()
            .list_repair_entries(&workspace_id, None, 100)
            .await
            .expect("count repairs after invalid write")
            .len(),
        repairs_before
    );
}

// ---------------------------------------------------------------------------
// #5 (MT-086 / MT-087): a garbage PDF degrades to ONE failed file, not a dead
// pass (lopdf catch_unwind guard). The pass completes and good files ingest.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt086_garbage_pdf_degrades_to_one_failed_file_pass_survives() {
    let Some(env) = open_embedded_ingestion_fixture().await else {
        eprintln!("SKIP mt086_garbage_pdf_degrades: embedded store unavailable");
        return;
    };
    let workspace_id = env.store.create_workspace().await;
    let ctx = test_ctx("mt086-garbage");
    let root = register_root(
        &env,
        &ctx,
        &workspace_id,
        "",
        KnowledgeRootKind::ProjectRepo,
    )
    .await;

    let temp = tempfile::tempdir().expect("temp dir");
    // A PDF magic header followed by truncated/garbage structure: the kind of
    // input that can drive a parser to panic. The catch_unwind guard converts
    // any panic into a typed failure for THIS file only.
    write(
        temp.path(),
        "docs/poison.pdf",
        b"%PDF-1.5\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\ntrailer<< /Root 1 0 R >>\n%%EOF\x00\x01\x02truncated",
    );
    // A healthy file in the same pass must still ingest.
    write(
        temp.path(),
        "docs/good.md",
        b"# Good\n\nthis file is fine\n",
    );
    // A valid text PDF too, to prove PDF ingestion itself still works after
    // the poison file.
    write(
        temp.path(),
        "docs/ok.pdf",
        &pdf_fixtures::text_pdf(&["Healthy page one"]),
    );

    let summary = env
        .engine
        .run_ingestion_pass(
            &ctx,
            &root.root_id,
            temp.path(),
            &IngestionLimits::default(),
        )
        .await
        .expect("pass must COMPLETE despite the poison PDF (no abort)");

    let poison = summary
        .outcomes
        .iter()
        .find(|o| o.source.relative_path.as_deref() == Some("docs/poison.pdf"))
        .expect("poison.pdf outcome");
    // Exactly ONE failed file, typed -- never a process abort, never a silent
    // empty success.
    assert_eq!(
        poison.receipt.status,
        ExtractionStatus::Failed,
        "poison PDF must be a typed failure"
    );
    assert!(
        matches!(
            poison.receipt.error_class,
            Some(IngestionErrorClass::ParseError) | Some(IngestionErrorClass::Internal)
        ),
        "poison PDF error_class must be typed (PARSE_ERROR or INTERNAL), got {:?}",
        poison.receipt.error_class
    );
    assert!(poison.spans.is_empty());

    // The pass survived: the good files ingested normally.
    let good = summary
        .outcomes
        .iter()
        .find(|o| o.source.relative_path.as_deref() == Some("docs/good.md"))
        .expect("good.md outcome");
    assert_eq!(good.receipt.status, ExtractionStatus::Success);
    let ok_pdf = summary
        .outcomes
        .iter()
        .find(|o| o.source.relative_path.as_deref() == Some("docs/ok.pdf"))
        .expect("ok.pdf outcome");
    assert_eq!(ok_pdf.receipt.status, ExtractionStatus::Success);
    assert!(!ok_pdf.spans.is_empty());
}

// ---------------------------------------------------------------------------
// #6 (MT-086): image-only PDF with tiny/invisible text -> NO_TEXT_LAYER.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt086_invisible_text_pdf_is_no_text_layer_not_empty_success() {
    let Some(env) = open_embedded_ingestion_fixture().await else {
        eprintln!("SKIP mt086_invisible_text_pdf: embedded store unavailable");
        return;
    };
    let workspace_id = env.store.create_workspace().await;
    let ctx = test_ctx("mt086-invisible");
    let root = register_root(
        &env,
        &ctx,
        &workspace_id,
        "docs",
        KnowledgeRootKind::ProjectRepo,
    )
    .await;

    // An image-only page whose ONLY text run is invisible (`3 Tr`): a reader
    // sees only the image. A naive text-operator check would mis-classify it
    // as a text layer; the detector must call it NO_TEXT_LAYER.
    let bytes = pdf_fixtures::invisible_text_pdf("hidden overlay words a reader never sees");
    let outcome = ingest(&env, &ctx, &root, "overlay.pdf", &bytes).await;

    assert_eq!(
        outcome.receipt.status,
        ExtractionStatus::Failed,
        "invisible-text image PDF must not empty-succeed"
    );
    assert_eq!(
        outcome.receipt.error_class,
        Some(IngestionErrorClass::NoTextLayer)
    );
    assert!(outcome.spans.is_empty());
    let detail = outcome.receipt.error_detail.as_ref().expect("detail");
    assert!(
        detail.to_string().contains("OCR_NEEDED"),
        "image-only must carry OCR guidance: {detail}"
    );
    // Repairable -> queued (OCR the page, re-import).
    let repair = outcome.repair.as_ref().expect("repair entry");
    assert_eq!(repair.reason_class.as_str(), "NO_TEXT_LAYER");

    // A genuinely tiny visible text run (< MIN_TEXT_LAYER_CHARS) is also not a
    // usable layer.
    let tiny = pdf_fixtures::build_pdf(&[pdf_fixtures::FixturePage::Text("a".to_string())]);
    let tiny_outcome = ingest(&env, &ctx, &root, "tiny.pdf", &tiny).await;
    assert_eq!(tiny_outcome.receipt.status, ExtractionStatus::Failed);
    assert_eq!(
        tiny_outcome.receipt.error_class,
        Some(IngestionErrorClass::NoTextLayer)
    );
}

// ---------------------------------------------------------------------------
// #7 (MT-094): re-failing after dead-letter REOPENS the terminal row for the
// same source+reason instead of inserting a new one.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt094_refail_after_dead_letter_reopens_row_not_new_one() {
    let Some(env) = open_embedded_ingestion_fixture().await else {
        eprintln!("SKIP mt094_reopen_dead_letter: embedded store unavailable");
        return;
    };
    let workspace_id = env.store.create_workspace().await;
    let ctx = test_ctx("mt094-reopen");
    let root = register_root(
        &env,
        &ctx,
        &workspace_id,
        "",
        KnowledgeRootKind::MediaLibrary,
    )
    .await;

    let temp = tempfile::tempdir().expect("temp dir");
    // A transcript with no well-formed cue: a whole-file PARSE_ERROR every pass.
    write(temp.path(), "stuck.srt", b"garbage\nwithout timing\n");
    let limits = IngestionLimits::default();

    // First pass enqueues the repair entry.
    let pass1 = env
        .engine
        .run_ingestion_pass(&ctx, &root.root_id, temp.path(), &limits)
        .await
        .expect("pass 1");
    let repair_id = pass1.outcomes[0]
        .repair
        .as_ref()
        .expect("queued repair")
        .repair_id
        .clone();

    // Exhaust the retry budget so the entry dead-letters (it never resolves --
    // the file stays broken).
    for _ in 0..3 {
        let _ = env
            .engine
            .retry_repair(&ctx, &repair_id, temp.path(), &limits)
            .await;
    }
    let dead = env
        .engine
        .store()
        .get_repair_entry(&repair_id)
        .await
        .expect("get entry")
        .expect("entry exists");
    assert_eq!(
        dead.state,
        RepairState::DeadLetter,
        "entry must be dead-lettered before the reopen test"
    );

    // Total rows for this source right now (the dead-letter row).
    let total_before = env
        .engine
        .store()
        .list_repair_entries(&workspace_id, None, 50)
        .await
        .expect("list all")
        .iter()
        .filter(|e| e.source_id == dead.source_id)
        .count();
    assert_eq!(total_before, 1, "exactly one row before the re-fail");

    // The source FAILS AGAIN (same source, same PARSE_ERROR reason). The pass
    // must REOPEN the dead-letter row, not insert a new one.
    let pass2 = env
        .engine
        .run_ingestion_pass(&ctx, &root.root_id, temp.path(), &limits)
        .await
        .expect("pass 2");
    let reopened = pass2.outcomes[0]
        .repair
        .as_ref()
        .expect("repair entry on re-fail");

    // SAME row id, reopened to queued with a fresh attempt budget.
    assert_eq!(
        reopened.repair_id, repair_id,
        "re-fail must REOPEN the dead-letter row, not create a new one"
    );
    assert_eq!(reopened.state, RepairState::Queued);
    assert_eq!(reopened.attempts, 0, "reopen resets the retry budget");

    // And still EXACTLY one row for this source -- no growth.
    let total_after = env
        .engine
        .store()
        .list_repair_entries(&workspace_id, None, 50)
        .await
        .expect("list all")
        .iter()
        .filter(|e| e.source_id == dead.source_id)
        .count();
    assert_eq!(
        total_after, 1,
        "dead-letter + re-fail must not multiply rows for the same source+reason"
    );
}

// ---------------------------------------------------------------------------
// #10 (MT-091): .env / .pem (and friends) are denied root registration.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt091_secret_bearing_paths_are_denied_root_registration() {
    let Some(env) = open_embedded_ingestion_fixture().await else {
        eprintln!("SKIP mt091_denied_paths: embedded store unavailable");
        return;
    };
    let workspace_id = env.store.create_workspace().await;
    let ctx = test_ctx("mt091-deny");

    // Each of these secret-bearing shapes must be DENIED by the default deny
    // patterns (#10), with a durable decision row recording the matched
    // pattern -- never a silent skip.
    for denied_path in [
        "app/.env",
        "deploy/.env.production",
        "certs/server.pem",
        "keys/id_rsa",
        "home/.aws/credentials",
        "project/.npmrc",
    ] {
        let result = env
            .engine
            .register_root(
                &ctx,
                RootRegistrationRequest {
                    workspace_id: workspace_id.clone(),
                    display_name: format!("deny test {denied_path}"),
                    root_kind: KnowledgeRootKind::ProjectRepo,
                    repo_relative_path: denied_path.to_string(),
                    file_allowlist_policy: serde_json::json!({"include": ["**/*"], "exclude": []}),
                    operator_approved: false,
                },
            )
            .await;
        let err = result.expect_err(&format!("{denied_path} must be denied"));
        let msg = err.to_string();
        assert!(
            msg.contains("denied_pattern") || msg.to_lowercase().contains("denied"),
            "{denied_path} should be denied_pattern, got: {msg}"
        );
    }

    // A legitimately-named file (environment.rs) is NOT caught by the .env
    // shape (control): the deny is anchored at the path-segment dot.
    let ok = register_root(
        &env,
        &ctx,
        &workspace_id,
        "src/environment.rs",
        KnowledgeRootKind::ProjectRepo,
    )
    .await;
    assert_eq!(ok.repo_relative_path, "src/environment.rs");

    // Durable decisions recorded the denials (backend-visible, not silent).
    let decisions = env
        .engine
        .store()
        .list_policy_decisions(&workspace_id, 50)
        .await
        .expect("list decisions");
    let denied_count = decisions
        .iter()
        .filter(|d| d.verdict.as_str() == "denied_pattern")
        .count();
    assert!(
        denied_count >= 6,
        "all six secret-path denials must be durable decisions: {denied_count}"
    );
}
