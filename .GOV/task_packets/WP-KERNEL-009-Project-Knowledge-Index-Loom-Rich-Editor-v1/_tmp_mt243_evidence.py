import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))
NOW = "2026-06-12T14:20:00Z"

p = os.path.join(HERE, "MT-243.json")
d = json.load(open(p, encoding="utf-8"))
lc = d["lifecycle"]
lc["implementation_commit"] = "8c9a1ef6"
lc["evidence"] = [
  {
    "at_utc": NOW,
    "kind": "implementation",
    "summary": "WikiIncrementalIngestFanOut: WikiFanOutEngine::run takes ONE changed source (kind source|entity|loom_block|rich_document + id), probes the GIN-indexed compile stamps for pages citing it, hash-diffs JUST that citation against current authority (same hash functions as MT-242 - the per-source stale set equals the drift stale set by construction AND is proven by independent computation in tests), regenerates exactly that set from each page's compile_recipe against CURRENT authority (module/concept recipes re-read symbols/concepts/hashes for their source set; entity/decision re-read their record; loom_topic re-compiles+restamps via the MT-184 path), refreshes page_links + the index/catalog page in the same pass, and emits EventLedger receipts: wiki_fanout_started / per-page wiki_page_fanout_regenerated (page rows reference them) / wiki_fanout_truncated / wiki_fanout_completed (LM-PWIKI-012; payload detail capped at 100 entries + totals so hot sources cannot bloat the ledger). Fan-out budget explicit, clamped 1..200 (default 25); truncated pages are durably marked rebuild_status=stale and a re-run RESUMES the remainder; a completed re-run is a no-op (no duplicate pages/links/receipts). Pages whose cited record vanished are reported orphaned and stay stale - never silently rebuilt. TRIGGER SURFACE: manual route POST /workspaces/:ws/loom/wiki/fanout {source_kind, source_id, budget?}; programmatic WikiFanOutEngine::run for save/ingest callers (deliberately NOT auto-wired into the index engine so ingestion latency stays decoupled - documented in fanout.rs module docs); POST .../wiki/drift-check marks, fan-out applies.",
    "files_changed": [
      "src/backend/handshake_core/src/knowledge_wiki/fanout.rs - WikiFanOutEngine/WikiFanOutRequest/WikiFanOutOutcome/WikiRegeneratedPage + per-recipe regeneration + budget/truncation/receipts",
      "src/backend/handshake_core/src/api/loom.rs - POST /workspaces/:ws/loom/wiki/fanout handler (committed under MT-241 commit; engine logic under this MT)",
      "src/backend/handshake_core/migrations/0301_loom_wiki_overlays_soft_ref.sql (+.down.sql) - drops the 0295 loom_wiki_overlays->projections CASCADE FK (pre-existing projection-isolation violation: deleting a projection destroyed AUTHORITY overlay rows and broke the zero-inbound-FK catalog law test); projection_id is now a soft ref, operator annotations survive projection churn",
      "src/backend/handshake_core/src/storage/postgres.rs - delete_loom_wiki_projection comment corrected (overlays survive)",
      "src/backend/handshake_core/tests/project_wiki_fanout_tests.rs + Cargo.toml [[test]] entry; project_wiki_bootstrap_tests.rs byte-identity test extended (loom_wiki_overlays in the authority fingerprint + overlay survives deleting the page it annotated)"
    ]
  },
  {
    "at_utc": NOW,
    "kind": "test_proof",
    "command": "cargo test -p handshake_core --features runtime-full,test-utils --test project_wiki_fanout_tests",
    "result": "ok. 4 passed; 0 failed (real Handshake-managed PostgreSQL). Regression suites after ALL changes: project_wiki_bootstrap_tests 4/4, project_wiki_drift_tests 4/4, loom_wiki_boundary_tests 5/5, knowledge_documents_tests 7/7 (incl. the previously-red no_authority_table_references_the_projection_table, fixed by 0301)",
    "tests": [
      "mt243_single_source_edit_regenerates_exactly_the_stale_set - REAL staleness.rs edited+re-indexed; fan-out stale set == independently computed MT-242 drift stale set == regenerated set (set equality both directions, LM-PWIKI-010); regenerated module page contains the NEW probe symbol (current-authority rebuild); all wikilinks resolved same pass; index refreshed; per-page receipts exist in kernel_event_ledger and page rows reference them; drift after = zero stale",
      "mt243_budget_truncation_is_loud_and_resumable - budget=1: one regenerated, remainder truncated with wiki_fanout_truncated ledger receipt naming skipped pages (skipped_total + capped detail); skipped pages durably stale; second run resumes EXACTLY the truncated remainder; third run finds nothing",
      "mt243_rerun_is_idempotent_no_duplicates - completed pass re-run: empty stale set, zero regenerated, index untouched, page count/ids/content/stamps unchanged, link sets duplicate-free, ledger counts for wiki_page_fanout_regenerated and wiki_fanout_truncated unchanged (no duplicate receipts)",
      "mt243_loom_block_change_fans_out_to_topic_pages - renaming ONE cited Loom block regenerates exactly the MT-184 topic page citing it (loom_topic recipe arm), content reflects the rename, idempotent re-run clean"
    ]
  }
]
lc["adversarial_code_review"] = {
  "at_utc": NOW,
  "reviewer": "KERNEL_BUILDER backend delegate (self-review)",
  "commit": "8c9a1ef6",
  "findings": [
    "FAN-OUT EXPLOSION (handled+tested): explicit budget clamped 1..200; truncation is LOUD (ledger receipt + durable stale marks) and resumable; receipt payload detail capped (first 100 + totals) so a hot source cited by thousands of pages cannot write multi-MB ledger events.",
    "SET-EQUALITY DRIFT vs PER-SOURCE PROBE (handled+tested): concept pages cite their file SOURCES in addition to entities (added during build) so a source-triggered fan-out reaches every page the drift checker flags; equality proven by independent computation in the test, not by construction alone.",
    "IDEMPOTENCY (handled+tested): regeneration restamps with current hashes, so a re-run resolves an empty stale set; upserts are identity-stable; page_links replaced wholesale; per-page receipts only emitted when a page actually regenerates - ledger counts proven unchanged on re-run.",
    "CRASH MID-FANOUT (handled by design + equivalent path tested): per-page receipt-then-upsert; a crash between leaves the page stale and a re-run resumes it - the truncation-resume test exercises exactly this partial-completion recovery shape.",
    "RECIPE FORGERY (handled): compile_recipe is server-written only (no API accepts a recipe); unknown recipe kinds fail with a typed error instead of guessing; loom_topic regeneration routes through the storage compile which fail-closes on missing blocks.",
    "ORPHANED PAGES (handled+surfaced): a cited record deleted from authority cannot be rebuilt - the page is reported orphaned in outcome+receipt and stays visibly stale; never a silent fake regeneration.",
    "BUDGET=0 STARVATION (handled): clamp floor of 1 prevents a request that can never make progress.",
    "AUTHORITY DESTRUCTION VIA PROJECTION DELETE (FIXED, pre-existing): 0295's overlay CASCADE FK deleted authority annotation rows on projection delete and broke the zero-inbound-FK law test (red since 0295, predates this MT); migration 0301 makes it a soft ref - overlay survival now proven in the byte-identity test."
  ],
  "fixes": [
    "Concept-page source citations (set-equality), receipt detail caps, and migration 0301 all landed and re-tested green (4/4 fanout, 4/4 bootstrap, 4/4 drift, 5/5 boundary, 7/7 documents)."
  ]
}
json.dump(d, open(p, "w", encoding="utf-8"), indent=2, ensure_ascii=False)
print("MT-243.json updated")
