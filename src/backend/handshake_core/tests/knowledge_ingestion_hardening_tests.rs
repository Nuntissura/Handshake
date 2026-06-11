//! WP-KERNEL-009 SourceIngestionAndEvidence HARDENING proofs against REAL
//! Handshake-managed PostgreSQL (MT-081/082/085/086/087/091/094 #1-#10).
//!
//! Every test drives the real `IngestionEngine` into a fresh isolated schema
//! (full migration chain incl. 0210 redaction guard + 0211 receipt-event
//! NOT NULL) and asserts durable DB state. No mocks, no SQLite. The fixtures
//! below carry FAKE credentials shaped like real ones.
//!
//! Coverage:
//! * #1/#4 (MT-091/096): a MEDIUM secret on the 120-line code-window seam is
//!   absent from ALL stored span content (whole-file redaction path). NOTE on
//!   the genuine boundary-SPLIT proof: the engine windows code by LINE, and a
//!   line is never cut across windows, while the MEDIUM secret regexes are all
//!   single-line -- so a secret whose BYTES straddle the seam (and would
//!   defeat a per-span rescan) cannot also be detected by the whole-file scan
//!   at the integration level. The byte-level split that genuinely breaks the
//!   OLD per-span rescan is proved by the unit test
//!   `secrets::tests::whole_file_redaction_catches_boundary_split_secret`
//!   (manual byte-split: each fragment alone matches nothing, whole-file
//!   findings still excise both halves). This integration test proves the
//!   end-to-end DB outcome: the secret at the seam is redacted out of every
//!   stored span row.
//! * #2 (MT-091): each new pattern (github_pat_, xapp-, headerless base64) is
//!   caught and redacted; raw bytes never stored.
//! * #3 (MT-091): the 0210 DB CHECK refuses a redacted span with no marker.
//! * #5 (MT-086/087): a garbage PDF degrades to ONE failed file, the pass
//!   completes and other files still ingest (catch_unwind guard).
//! * #6 (MT-086): an image-only PDF with tiny/invisible text -> NO_TEXT_LAYER.
//! * #8 (MT-085/094): the 0211 NOT NULL pins refuse a receipt / repair row
//!   with a NULL ledger-event ref.
//! * #7 (MT-094): re-failing after dead-letter REOPENS the terminal row for
//!   the same source+reason instead of inserting a new one.
//! * #10 (MT-091): .env / .pem paths are denied root registration.

mod knowledge_ingestion_support;
mod knowledge_pg_support;

use std::path::Path;

use handshake_core::knowledge_ingestion::backpressure::IngestionLimits;
use handshake_core::knowledge_ingestion::engine::{
    FileIngestOutcome, IngestionContext, RootRegistrationRequest,
};
use handshake_core::knowledge_ingestion::pdf::fixtures as pdf_fixtures;
use handshake_core::knowledge_ingestion::receipts::{ExtractionStatus, IngestionErrorClass};
use handshake_core::knowledge_ingestion::repair::RepairState;
use handshake_core::storage::knowledge::{KnowledgeRootKind, KnowledgeSourceRoot};
use knowledge_ingestion_support::{ingestion_pg, register_root, test_ctx, IngestionPg};
use sqlx::Row;

async fn ingest(
    env: &IngestionPg,
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

/// Count rows in `table` whose `content` column matches `LIKE %needle%`.
async fn count_like(env: &IngestionPg, table: &str, column: &str, needle: &str) -> i64 {
    let mut conn = env.pg.raw_connection().await;
    sqlx::query_scalar(&format!(
        "SELECT count(*) FROM {table} WHERE {column} LIKE $1"
    ))
    .bind(format!("%{needle}%"))
    .fetch_one(&mut conn)
    .await
    .expect("count_like probe")
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
    let Some(env) = ingestion_pg().await else {
        eprintln!("SKIP mt091_mt096_boundary_straddle: no PostgreSQL");
        return;
    };
    let workspace_id = env.pg.create_workspace().await;
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

    // CORE assertion: the raw JWT is absent from EVERY stored span content row
    // (LIKE probe straight against the DB), and from receipts/sources/ledger.
    assert_eq!(
        count_like(&env, "knowledge_ingestion_spans", "content", jwt).await,
        0,
        "raw boundary JWT leaked into a span content row"
    );
    // Probe each contiguous third of the JWT too: a boundary split could leave
    // a fragment behind even if the whole token is gone.
    for fragment in [&jwt[..30], &jwt[30..60], &jwt[60..]] {
        assert_eq!(
            count_like(&env, "knowledge_ingestion_spans", "content", fragment).await,
            0,
            "a JWT fragment leaked into a span content row: {fragment}"
        );
    }
    for (table, column) in [
        ("knowledge_ingestion_receipts", "error_detail::text"),
        ("knowledge_sources", "provenance::text"),
        ("kernel_event_ledger", "payload::text"),
    ] {
        assert_eq!(
            count_like(&env, table, column, jwt).await,
            0,
            "raw boundary JWT leaked into {table}.{column}"
        );
    }
}

// ---------------------------------------------------------------------------
// #2 (MT-091): new secret patterns are caught + redacted, never stored raw.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt091_new_patterns_github_pat_slack_app_headerless_blob_redacted() {
    let Some(env) = ingestion_pg().await else {
        eprintln!("SKIP mt091_new_patterns: no PostgreSQL");
        return;
    };
    let workspace_id = env.pg.create_workspace().await;
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
    assert_eq!(
        count_like(&env, "knowledge_ingestion_spans", "content", pat_body59).await,
        0,
        "raw github_pat body leaked into span content"
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
    assert_eq!(
        count_like(&env, "knowledge_ingestion_spans", "content", xapp_body).await,
        0,
        "raw xapp token leaked into span content"
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
    // The blob never lands anywhere durable.
    assert_eq!(
        count_like(&env, "knowledge_ingestion_spans", "content", blob).await,
        0
    );
    assert_eq!(
        count_like(&env, "kernel_event_ledger", "payload::text", blob).await,
        0
    );
}

// ---------------------------------------------------------------------------
// #3 (MT-091): the 0210 DB CHECK refuses a redacted span with no marker.
// ---------------------------------------------------------------------------

/// Schema-enforced redaction invariant: a row claiming
/// redaction_state='redacted' whose content carries NO '[REDACTED:' marker
/// (i.e. raw secret bytes could still be present) is refused at the DB layer,
/// independent of any application code path.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt091_db_guard_refuses_redacted_span_without_marker() {
    let Some(env) = ingestion_pg().await else {
        eprintln!("SKIP mt091_db_guard_redacted_marker: no PostgreSQL");
        return;
    };
    let workspace_id = env.pg.create_workspace().await;
    let ctx = test_ctx("mt091-dbguard");
    let root = register_root(
        &env,
        &ctx,
        &workspace_id,
        "src",
        KnowledgeRootKind::ProjectRepo,
    )
    .await;

    // Create a real source + receipt to satisfy the span FKs.
    let outcome = ingest(
        &env,
        &ctx,
        &root,
        "docs/plain.md",
        b"# Plain\n\nno secrets here\n",
    )
    .await;
    let source_id = outcome.source.source_id.clone();
    let receipt_id = outcome.receipt.receipt_id.clone();

    let mut conn = env.pg.raw_connection().await;

    // A 'redacted' span whose content has NO marker must be REFUSED by the
    // 0210 CHECK. The content here is a raw secret that was (incorrectly) not
    // redacted -- exactly the leak the guard exists to stop. span_index 900
    // is chosen well clear of the real spans the plain.md ingest already wrote
    // on this receipt, so the ONLY constraint this row can violate is the
    // redaction-marker CHECK (never the (receipt_id, span_index) unique index).
    let bad = sqlx::query(
        "INSERT INTO knowledge_ingestion_spans
            (span_id, workspace_id, source_id, receipt_id, span_index, anchor_kind,
             anchor, content, content_hash, redaction_state)
         VALUES ($1, $2, $3, $4, 900, 'line_range',
                 '{\"anchor_kind\":\"line_range\",\"line_start\":1,\"line_end\":1}'::jsonb,
                 $5, $6, 'redacted')",
    )
    .bind(format!("KISP-{}", "0".repeat(32)))
    .bind(&workspace_id)
    .bind(&source_id)
    .bind(&receipt_id)
    .bind("api_key = AKIAIOSFODNN7EXAMPLE still raw")
    .bind("a".repeat(64))
    .execute(&mut conn)
    .await;
    let err = bad.expect_err("redacted span without a marker must be refused by the DB");
    assert!(
        err.to_string()
            .contains("chk_knowledge_ingestion_spans_redaction_marker")
            || err.to_string().to_lowercase().contains("check"),
        "unexpected error (want check_violation): {err}"
    );

    // The SAME content shape but WITH a proper marker IS accepted (positive
    // control), at a likewise-clear span_index.
    let good = sqlx::query(
        "INSERT INTO knowledge_ingestion_spans
            (span_id, workspace_id, source_id, receipt_id, span_index, anchor_kind,
             anchor, content, content_hash, redaction_state)
         VALUES ($1, $2, $3, $4, 901, 'line_range',
                 '{\"anchor_kind\":\"line_range\",\"line_start\":1,\"line_end\":1}'::jsonb,
                 $5, $6, 'redacted')",
    )
    .bind(format!("KISP-{}", "1".repeat(32)))
    .bind(&workspace_id)
    .bind(&source_id)
    .bind(&receipt_id)
    .bind("api_key = [REDACTED:aws_access_key_id] now safe")
    .bind("b".repeat(64))
    .execute(&mut conn)
    .await;
    assert!(
        good.is_ok(),
        "a redacted span WITH a marker must be accepted: {good:?}"
    );
}

// ---------------------------------------------------------------------------
// #8 (MT-085 / MT-094): 0211 NOT NULL pins on the ledger-event refs.
// ---------------------------------------------------------------------------

/// Every ingestion mutation must carry an EventLedger receipt: a receipt or a
/// repair-queue row inserted with a NULL ledger-event ref is refused by the
/// 0211 NOT NULL constraints, schema-enforcing the receipt law.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt085_mt094_db_guard_requires_ledger_event_ref() {
    let Some(env) = ingestion_pg().await else {
        eprintln!("SKIP mt085_mt094_ledger_ref_not_null: no PostgreSQL");
        return;
    };
    let workspace_id = env.pg.create_workspace().await;
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
    let source_id = outcome.source.source_id.clone();

    let mut conn = env.pg.raw_connection().await;

    // (a) receipt with NULL receipt_event_id -> refused.
    let bad_receipt = sqlx::query(
        "INSERT INTO knowledge_ingestion_receipts
            (receipt_id, workspace_id, source_id, extractor_id, extractor_version,
             status, content_hash, receipt_event_id)
         VALUES ($1, $2, $3, 'x', '1', 'success', $4, NULL)",
    )
    .bind(format!("KIRC-{}", "0".repeat(32)))
    .bind(&workspace_id)
    .bind(&source_id)
    .bind("c".repeat(64))
    .execute(&mut conn)
    .await;
    let err = bad_receipt.expect_err("receipt without a ledger event must be refused");
    assert!(
        err.to_string().to_lowercase().contains("null")
            || err.to_string().to_lowercase().contains("not-null")
            || err.to_string().to_lowercase().contains("not null"),
        "unexpected error (want not-null violation): {err}"
    );

    // (b) repair entry with NULL enqueue_event_id -> refused.
    let bad_repair = sqlx::query(
        "INSERT INTO knowledge_ingestion_repair_queue
            (repair_id, workspace_id, source_id, reason_class, enqueue_event_id)
         VALUES ($1, $2, $3, 'PARSE_ERROR', NULL)",
    )
    .bind(format!("KIRQ-{}", "0".repeat(32)))
    .bind(&workspace_id)
    .bind(&source_id)
    .execute(&mut conn)
    .await;
    let err = bad_repair.expect_err("repair row without a ledger event must be refused");
    assert!(
        err.to_string().to_lowercase().contains("null"),
        "unexpected error (want not-null violation): {err}"
    );

    // Positive control: the real engine path (which always mints the event)
    // already produced a receipt WITH a ledger event for the source above.
    assert!(outcome.receipt.receipt_event_id.is_some());
}

// ---------------------------------------------------------------------------
// #5 (MT-086 / MT-087): a garbage PDF degrades to ONE failed file, not a dead
// pass (lopdf catch_unwind guard). The pass completes and good files ingest.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt086_garbage_pdf_degrades_to_one_failed_file_pass_survives() {
    let Some(env) = ingestion_pg().await else {
        eprintln!("SKIP mt086_garbage_pdf_degrades: no PostgreSQL");
        return;
    };
    let workspace_id = env.pg.create_workspace().await;
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
    let Some(env) = ingestion_pg().await else {
        eprintln!("SKIP mt086_invisible_text_pdf: no PostgreSQL");
        return;
    };
    let workspace_id = env.pg.create_workspace().await;
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
    let Some(env) = ingestion_pg().await else {
        eprintln!("SKIP mt094_reopen_dead_letter: no PostgreSQL");
        return;
    };
    let workspace_id = env.pg.create_workspace().await;
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
    let Some(env) = ingestion_pg().await else {
        eprintln!("SKIP mt091_denied_paths: no PostgreSQL");
        return;
    };
    let workspace_id = env.pg.create_workspace().await;
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
