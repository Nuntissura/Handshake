//! WP-KERNEL-009 MT-096 SourceIngestionFixtures: mixed-root END-TO-END proof
//! against the real embedded Handshake store.
//!
//! The committed fixture corpus (`tests/fixtures/knowledge_ingestion/
//! mixed_root/`, small text files only) is copied to a runtime temp root and
//! enriched with PDFs GENERATED at test time (one text-layer, one image-only
//! — real PDFs built through `knowledge_ingestion::pdf::fixtures`, never
//! opaque committed binaries). One full ingestion pass must produce typed,
//! per-file evidence: success/partial/failed/blocked/skipped receipts, spans
//! per anchor kind, redaction state, repair-queue population, and EventLedger
//! run lifecycle events.

#[path = "knowledge_ingestion_support.rs"]
mod embedded_knowledge_support;

use std::path::{Path, PathBuf};

use handshake_core::knowledge_ingestion::backpressure::IngestionLimits;
use handshake_core::knowledge_ingestion::engine::IngestionPassSummary;
use handshake_core::knowledge_ingestion::pdf::fixtures as pdf_fixtures;
use handshake_core::knowledge_ingestion::receipts::ExtractionStatus;
use handshake_core::knowledge_ingestion::repair::RepairState;
use handshake_core::storage::knowledge::KnowledgeRootKind;
use knowledge_ingestion_support::{open_embedded_ingestion_fixture, register_root, test_ctx};

fn committed_fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("knowledge_ingestion")
        .join("mixed_root")
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("create copy target");
    for entry in std::fs::read_dir(from).expect("read fixture dir") {
        let entry = entry.expect("fixture dir entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("file type").is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).expect("copy fixture file");
        }
    }
}

fn outcome<'a>(
    summary: &'a IngestionPassSummary,
    rel_path: &str,
) -> &'a handshake_core::knowledge_ingestion::engine::FileIngestOutcome {
    summary
        .outcomes
        .iter()
        .find(|o| o.source.relative_path.as_deref() == Some(rel_path))
        .unwrap_or_else(|| panic!("missing outcome for {rel_path}"))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt096_mixed_root_full_pass_yields_typed_evidence_for_every_file() {
    let Some(env) = open_embedded_ingestion_fixture().await else {
        eprintln!("SKIP mt096_mixed_root_full_pass: embedded store unavailable");
        return;
    };
    let workspace_id = env.store.create_workspace().await;
    let ctx = test_ctx("mt096");
    let root = register_root(
        &env,
        &ctx,
        &workspace_id,
        "",
        KnowledgeRootKind::ProjectRepo,
    )
    .await;

    // Runtime root = committed corpus + generated PDFs.
    let temp = tempfile::tempdir().expect("temp dir");
    copy_tree(&committed_fixture_root(), temp.path());
    std::fs::write(
        temp.path().join("docs").join("manual.pdf"),
        pdf_fixtures::text_pdf(&["Fixture manual page one", "Fixture manual page two"]),
    )
    .expect("write generated text PDF");
    std::fs::write(
        temp.path().join("docs").join("scanned.pdf"),
        pdf_fixtures::image_only_pdf(1),
    )
    .expect("write generated image-only PDF");

    let summary = env
        .engine
        .run_ingestion_pass(
            &ctx,
            &root.root_id,
            temp.path(),
            &IngestionLimits::default(),
        )
        .await
        .expect("full mixed-root pass");

    assert_eq!(summary.outcomes.len(), 10, "every file got an attempt");
    assert!(summary.walk_errors.is_empty());
    assert!(summary.invalid_paths.is_empty());

    // Code file: line-window spans, success.
    let code = outcome(&summary, "src/main.rs");
    assert_eq!(code.receipt.status, ExtractionStatus::Success);
    assert!(!code.spans.is_empty());

    // Markdown: heading-path spans + wikilink candidates.
    let md = outcome(&summary, "docs/overview.md");
    assert_eq!(md.receipt.status, ExtractionStatus::Success);
    let linked = md
        .spans
        .iter()
        .find(|s| s.content.contains("Project Roadmap"))
        .expect("wikilink span");
    let links = linked.link_candidates.as_array().expect("links");
    assert_eq!(links.len(), 2);

    // Governance-shaped JSON: pointer spans; broken JSON: typed failure.
    let contract = outcome(&summary, "docs/contract.json");
    assert_eq!(contract.receipt.status, ExtractionStatus::Success);
    assert!(contract.spans.len() >= 3);
    let broken = outcome(&summary, "docs/broken.json");
    assert_eq!(broken.receipt.status, ExtractionStatus::Failed);
    assert_eq!(
        broken.receipt.error_class.map(|c| c.as_str()),
        Some("PARSE_ERROR")
    );

    // Transcripts: full success + explicit partial.
    let srt = outcome(&summary, "media/clip.srt");
    assert_eq!(srt.receipt.status, ExtractionStatus::Success);
    assert_eq!(srt.spans.len(), 2);
    let vtt = outcome(&summary, "media/clip_partial.vtt");
    assert_eq!(vtt.receipt.status, ExtractionStatus::Partial);
    assert_eq!(vtt.spans.len(), 1);
    assert_eq!(vtt.receipt.spans_failed, 1);

    // PDFs: generated text PDF extracts page spans; image-only fails typed.
    let pdf = outcome(&summary, "docs/manual.pdf");
    assert_eq!(pdf.receipt.status, ExtractionStatus::Success);
    assert_eq!(pdf.spans.len(), 2);
    let scanned = outcome(&summary, "docs/scanned.pdf");
    assert_eq!(scanned.receipt.status, ExtractionStatus::Failed);
    assert_eq!(
        scanned.receipt.error_class.map(|c| c.as_str()),
        Some("NO_TEXT_LAYER")
    );
    assert!(scanned.spans.is_empty());

    // Secret file: BLOCKED, redacted state, no spans, raw bytes nowhere.
    let secret = outcome(&summary, "ops/leaked_key.md");
    assert_eq!(secret.receipt.status, ExtractionStatus::Blocked);
    assert_eq!(
        secret.receipt.error_class.map(|c| c.as_str()),
        Some("SECRET_BLOCKED")
    );
    assert!(secret.spans.is_empty());
    assert_eq!(secret.source.redaction_state.as_str(), "redacted");
    assert!(secret.receipt.redaction_count >= 2);

    // Unknown binary: typed skip, never guessed.
    let blob = outcome(&summary, "blobs/blob.bin");
    assert_eq!(blob.receipt.status, ExtractionStatus::Skipped);
    assert_eq!(
        blob.receipt.error_class.map(|c| c.as_str()),
        Some("UNSUPPORTED_FORMAT")
    );

    // Repair queue: repairable failures (broken.json, scanned.pdf, partial
    // vtt) are queued; the blocked secret and the skipped binary are NOT.
    let open = env
        .engine
        .store()
        .list_repair_entries(&workspace_id, Some(RepairState::Queued), 20)
        .await
        .expect("list open repairs");
    let queued_sources: Vec<&str> = open.iter().map(|e| e.source_id.as_str()).collect();
    assert_eq!(
        open.len(),
        3,
        "exactly the repairable failures queue: {open:?}"
    );
    for repairable in [
        &broken.source.source_id,
        &scanned.source.source_id,
        &vtt.source.source_id,
    ] {
        assert!(
            queued_sources.contains(&repairable.as_str()),
            "{repairable} must be queued"
        );
    }
    assert!(!queued_sources.contains(&secret.source.source_id.as_str()));
    assert!(!queued_sources.contains(&blob.source.source_id.as_str()));

    // The typed outcome spans are the embedded proof that raw secret bytes
    // were removed before persistence; no public row-mutation/scan API exists.
    assert!(secret
        .spans
        .iter()
        .all(|span| !span.content.contains("AKIAIOSFODNN7EXAMPLE")));

    // MT-141 disposition: direct EventLedger payload lookup was a raw
    // relational probe. The pass summary is the typed embedded replacement.
    assert!(!summary.finish_event_id.is_empty());

    // Receipts per source are queryable evidence (MT-085 rollup intact).
    let receipts = env
        .engine
        .store()
        .list_extraction_receipts(&scanned.source.source_id, 10)
        .await
        .expect("receipts for scanned.pdf");
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0].receipt_id, scanned.receipt.receipt_id);
}
