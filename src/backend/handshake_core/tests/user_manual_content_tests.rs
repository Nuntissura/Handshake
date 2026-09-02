//! WP-KERNEL-009 UserManual content accuracy proof (ACCURACY IS LAW): the
//! seeded manual text is checked against the LIVE product — runtime behavior
//! first, source-of-truth files where runtime introspection is impossible.
//!
//! * MT-196 — purpose/workflow pages: kernel002 topic import is
//!   deterministic; documented startup constants match the product source.
//! * MT-197/MT-198 — documented failure modes are TRIGGERED at runtime and
//!   the observed typed errors must fall inside the documented vocabulary
//!   (identity 400, permission 403 reasons, save 409 conflict, repair-action
//!   vocabulary).
//! * MT-202 — a REAL compiled context bundle cites a UserManual page with
//!   version + source anchor + drift hash.
//! * MT-206 — the state-recovery guide covers the four contract scenarios.
//! * MT-207 — every spec-enrichment seed row targets an anchor that exists
//!   in the CURRENT Master Spec bundle.
//! * MT-208 — missing-page / legacy-redirect / orphan-navigation fixtures
//!   drive their negative verdicts.

mod surreal_test_store_support;
#[allow(dead_code)]
mod user_manual_support;

use handshake_core::api;
use handshake_core::kernel::model_manual::kernel002_no_context_model_manual;
use handshake_core::knowledge_document::embed::{BrokenEmbedRepair, EmbedRefKind, EmbedTarget};
#[cfg(feature = "legacy-postgres-superseded")]
use handshake_core::knowledge_retrieval::budget::PriorityTier;
#[cfg(feature = "legacy-postgres-superseded")]
use handshake_core::user_manual::bundle_bridge::manual_bundle_candidate;
use handshake_core::user_manual::fixtures::{delete_page, insert_orphan_page, unreachable_pages};
use handshake_core::user_manual::freshness::{check_freshness, FreshnessVerdictKind};
use handshake_core::user_manual::seed::seed_corpus;
use handshake_core::user_manual::spec_seed::spec_enrichment_seed;
use handshake_core::user_manual::store::UserManualStore;
use serde_json::Value;
use std::path::PathBuf;
use user_manual_support::{start_server, UserManualTestScope};

async fn seeded_scope() -> UserManualTestScope {
    let scope = UserManualTestScope::create().await;
    scope.store().ensure_seeded().await.expect("seed corpus");
    scope
}

// ---------------------------------------------------------------------------
// MT-196.
// ---------------------------------------------------------------------------

/// MT-196 (+ UMMIG-002 mapping law): every kernel002 manual topic and every
/// instruction line is present, verbatim, on the seeded
/// kernel-write-governance page — the legacy struct maps deterministically
/// into UserManual authority.
#[tokio::test]
async fn mt196_kernel002_manual_topics_are_seeded_as_pages() {
    let scope = seeded_scope().await;
    let store = scope.store();
    let (_, sections, _) = store
        .get_page_by_slug("kernel-write-governance")
        .await
        .expect("page query")
        .expect("kernel-write-governance seeded");

    let kernel_manual = kernel002_no_context_model_manual();
    for kernel_section in kernel_manual.sections {
        let seeded = sections
            .iter()
            .find(|s| s.title == kernel_section.title)
            .unwrap_or_else(|| panic!("kernel002 topic '{}' not seeded", kernel_section.title));
        for instruction in kernel_section.instructions {
            assert!(
                seeded.body_md.contains(instruction),
                "kernel002 instruction missing from '{}': {instruction}",
                kernel_section.title
            );
        }
    }
}

/// MT-196: documented startup and embedded-storage constants match product
/// source. A source change forces the canonical manual to move with it.
#[tokio::test]
async fn mt196_documented_startup_constants_match_product_source() {
    let scope = seeded_scope().await;
    let store = scope.store();
    let (_, sections, _) = store
        .get_page_by_slug("startup-and-run-commands")
        .await
        .expect("page query")
        .expect("startup page seeded");
    let page_text: String = sections
        .iter()
        .map(|s| s.body_md.clone())
        .collect::<Vec<_>>()
        .join("\n");

    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let main_rs = std::fs::read_to_string(crate_root.join("src/main.rs")).expect("read main.rs");
    let embedded = std::fs::read_to_string(crate_root.join("src/storage/surreal.rs"))
        .expect("read embedded storage source");

    assert!(page_text.contains("127.0.0.1:37501"));
    assert!(
        main_rs.contains("37501"),
        "main.rs no longer binds 37501 — update startup-and-run-commands"
    );
    assert!(page_text.contains("HANDSHAKE_DATA_DIR"));
    assert!(embedded.contains("HANDSHAKE_DATA_DIR_ENV"));
    assert!(embedded.contains("DEFAULT_STORE_DIRECTORY"));
    // The documented mounts exist in main.rs (`/api` nest + merge at root).
    assert!(main_rs.contains(".nest(\"/api\", api_routes)"));
}

// ---------------------------------------------------------------------------
// MT-197 / MT-198: documented failure modes triggered at runtime.
// ---------------------------------------------------------------------------

struct RouterFixture {
    _scope: UserManualTestScope,
    store: UserManualStore,
    base: String,
    _server: tokio::task::JoinHandle<()>,
    http: reqwest::Client,
}

async fn router_fixture() -> RouterFixture {
    let scope = seeded_scope().await;
    let store = scope.store();
    let (base, server) = start_server(api::user_manual::routes_for_test(scope.storage())).await;
    RouterFixture {
        _scope: scope,
        store,
        base,
        _server: server,
        http: reqwest::Client::new(),
    }
}

/// MT-197/MT-198: the documented failure table is LIVE-VERIFIED — each
/// triggered failure must answer with a code the failure-modes page
/// documents for that surface (doc-vs-runtime in the direction that
/// matters: observed behavior ∈ documented vocabulary).
#[tokio::test]
async fn mt198_documented_failure_modes_match_runtime() {
    let fx = router_fixture().await;
    let (_, sections, _) = fx
        .store
        .get_page_by_slug("failure-modes-and-recovery")
        .await
        .expect("page query")
        .expect("failure page seeded");
    let vocab: Value = sections
        .iter()
        .find_map(|s| s.body_json.clone())
        .expect("failure page carries the machine-readable vocabulary");
    let documented = |family: &str, code: &str| -> bool {
        vocab[family]
            .as_array()
            .map(|codes| codes.iter().any(|c| c == code))
            .unwrap_or(false)
    };

    let manual_missing = fx
        .http
        .get(format!("{}/usermanual/pages/zzz-missing", fx.base))
        .send()
        .await
        .expect("manual missing");
    assert_eq!(manual_missing.status(), 404);
    let manual_body: Value = manual_missing.json().await.expect("manual 404 json");
    assert!(documented(
        "usermanual",
        manual_body["error"].as_str().unwrap()
    ));

    let denied = fx
        .http
        .post(format!("{}/usermanual/resync", fx.base))
        .header("x-hsk-actor-kind", "cloud_model")
        .send()
        .await
        .expect("cloud resync denial");
    assert_eq!(denied.status(), 403);
    let denied_body: Value = denied.json().await.expect("denial json");
    assert_eq!(denied_body["error"], "forbidden");
    assert_eq!(denied_body["reason"], "cloud_model_resync_denied");
}

/// MT-198: the documented embed law and repair-action vocabulary match the
/// live types exactly (4 typed constructor errors; relink|reresolve|remove).
#[tokio::test]
async fn mt198_embed_law_and_repair_vocabulary_match_types() {
    let scope = seeded_scope().await;
    let store = scope.store();
    let (_, sections, _) = store
        .get_page_by_slug("rich-documents-surface")
        .await
        .expect("page query")
        .expect("rich documents page seeded");
    let page_text: String = sections
        .iter()
        .map(|s| s.body_md.clone())
        .collect::<Vec<_>>()
        .join("\n");

    // Documented repair actions == the live enum's offers.
    let repair = BrokenEmbedRepair::new(
        "KBL-doc-test",
        EmbedTarget::new(EmbedRefKind::Media, "missing-media-id").expect("typed target"),
        "media id not found",
    );
    assert_eq!(repair.available_actions.len(), 3);
    for action in &repair.available_actions {
        let name = format!("{action:?}").to_lowercase();
        assert!(
            page_text.to_lowercase().contains(&name),
            "repair action '{name}' is offered by the product but not documented"
        );
    }

    // Documented embed-target rejections == the live constructor behavior.
    assert!(EmbedTarget::new(EmbedRefKind::Media, "").is_err());
    assert!(EmbedTarget::new(EmbedRefKind::Media, "C:\\evil\\path.png").is_err());
    assert!(EmbedTarget::new(EmbedRefKind::Url, "ftp://nope").is_err());
    assert!(EmbedTarget::new(EmbedRefKind::Media, "javascript:alert(1)").is_err());
    let safety_text: String = {
        let (_, safety_sections, _) = store
            .get_page_by_slug("permissions-and-safety")
            .await
            .expect("safety page query")
            .expect("safety page seeded");
        safety_sections
            .iter()
            .map(|s| s.body_md.clone())
            .collect::<Vec<_>>()
            .join("\n")
    };
    for documented_reason in ["empty", "absolute path", "non-http url", "scheme not"] {
        assert!(
            safety_text.to_lowercase().contains(documented_reason),
            "embed rejection reason '{documented_reason}' undocumented"
        );
    }
}

// ---------------------------------------------------------------------------
// MT-202: bundle candidate cites a manual page.
// ---------------------------------------------------------------------------

/// MT-202: the UserManual bridge emits a candidate whose citation source,
/// drift hash, and EventLedger linkage bind the selected page and section.
#[cfg(feature = "legacy-postgres-superseded")]
#[tokio::test]
async fn mt202_bundle_cites_manual_page_with_version_and_anchor() {
    let scope = seeded_scope().await;
    let store = scope.store();
    let (page, sections, _) = store
        .get_page_by_slug("state-recovery-guide")
        .await
        .expect("page query")
        .expect("state-recovery-guide seeded");
    let section = &sections[0];

    let candidate = manual_bundle_candidate(
        &page,
        section,
        "fixture-user-manual-entity",
        PriorityTier::Authoritative,
        40,
        0.95,
    );
    assert_eq!(candidate.ref_id, "fixture-user-manual-entity");
    let snippet = candidate
        .snippet
        .expect("manual candidate carries citation");
    let citation = snippet
        .source_path
        .as_deref()
        .expect("manual citation source");
    assert!(
        citation.contains("usermanual:state-recovery-guide@"),
        "citation must carry slug+version: {citation}"
    );
    assert!(
        citation.contains(&page.manual_version),
        "citation must carry the manual version: {citation}"
    );
    assert!(
        citation.contains(&format!("#{}-{}", section.section_kind, section.position)),
        "citation must carry the section source anchor: {citation}"
    );
    assert!(
        snippet.content_sha256 == page.content_hash,
        "candidate must carry the full page drift hash"
    );
    assert_eq!(snippet.extraction_receipt_event_id, page.ledger_event_id);
}

// ---------------------------------------------------------------------------
// MT-206 / MT-207.
// ---------------------------------------------------------------------------

/// MT-206: the state-recovery guide covers all four contract scenarios.
#[tokio::test]
async fn mt206_state_recovery_guide_covers_contract_scenarios() {
    let scope = seeded_scope().await;
    let store = scope.store();
    let (page, sections, _) = store
        .get_page_by_slug("state-recovery-guide")
        .await
        .expect("page query")
        .expect("state-recovery-guide seeded");
    assert_eq!(page.page_kind, "state_recovery");
    assert!(sections.len() >= 4, "four recovery scenarios expected");
    let all_text: String = sections
        .iter()
        .map(|s| format!("{}\n{}", s.title, s.body_md))
        .collect::<Vec<_>>()
        .join("\n")
        .to_lowercase();
    for scenario in [
        "compaction",
        "interrupted microtask",
        "failed build",
        "validation reentry",
    ] {
        assert!(
            all_text.contains(scenario),
            "state-recovery guide missing scenario '{scenario}'"
        );
    }
    for section in &sections {
        assert_eq!(section.section_kind, "recovery");
    }
}

/// MT-224: the state-recovery guide must cover the live parallel-swarm
/// state-recovery APIs by symbol, not just prose. The source-symbol check
/// keeps the page tied to runtime code instead of an invented manual.
#[tokio::test]
async fn mt224_parallel_swarm_manual_patch_covers_live_runtime_symbols() {
    let scope = seeded_scope().await;
    let store = scope.store();
    let (_, sections, anchors) = store
        .get_page_by_slug("state-recovery-guide")
        .await
        .expect("page query")
        .expect("state-recovery-guide seeded");
    let section = sections
        .iter()
        .find(|s| s.title == "Parallel swarm operation and recovery")
        .expect("state-recovery-guide must include the MT-224 parallel swarm section");

    let expected_symbols = [
        "AgentLaneIdentity",
        "claim_work_surface",
        "record_role_mailbox_handoff",
        "resolve_backend_navigation_quiet",
        "record_checkpoint",
        "recover_from_checkpoint",
        "enqueue_indexing_lease",
        "try_acquire_indexing_lease",
        "record_quiet_background_work",
        "project_swarm_dashboard",
        "build_handoff_compression_template",
    ];
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let runtime =
        std::fs::read_to_string(crate_root.join("src/swarm_orchestration/state_recovery.rs"))
            .expect("read state_recovery.rs");
    let body_json = section
        .body_json
        .as_ref()
        .expect("parallel swarm section must carry machine-readable runtime symbols");
    for symbol in expected_symbols {
        assert!(
            runtime.contains(symbol),
            "runtime no longer exposes {symbol}; update MT-224 manual coverage"
        );
        assert!(
            section.body_md.contains(symbol),
            "manual section does not name live symbol {symbol}"
        );
        assert!(
            body_json["runtime_symbols"]
                .as_array()
                .expect("runtime_symbols array")
                .iter()
                .any(|value| value.as_str() == Some(symbol)),
            "manual body_json runtime_symbols missing {symbol}"
        );
    }

    let expected_negative_tests = [
        "mt223_interrupted_indexing_start_failure_leaves_no_swarm_or_kir_receipts",
        "mt223_quiet_receipt_failure_rolls_back_index_run_and_lease",
        "mt223_stale_indexing_lease_enqueue_does_not_leapfrog_queued_writer",
        "mt223_restart_after_crash_reconstructs_swarm_state_from_surrealdb",
    ];
    let test_source =
        std::fs::read_to_string(crate_root.join("tests/parallel_swarm_state_recovery_tests.rs"))
            .expect("read parallel swarm tests");
    for test_name in expected_negative_tests {
        assert!(
            test_source.contains(test_name),
            "runtime proof {test_name} missing; update MT-224 manual evidence"
        );
        assert!(
            section.body_md.contains(test_name),
            "manual section does not cite runtime proof {test_name}"
        );
    }

    for target in [
        "backend-navigation-and-identity",
        "quickstart-state-recovery",
    ] {
        assert!(
            anchors
                .iter()
                .any(|a| a.anchor_kind == "page_link" && a.anchor_value == target),
            "state-recovery-guide missing page link to {target}"
        );
    }
}

/// MT-207: every spec-enrichment seed row targets an anchor that EXISTS in
/// the current Master Spec bundle — the prepared wording can be lifted
/// without archaeology. SKIPs only when the spec bundle is not checked out.
#[test]
fn mt207_spec_seed_anchors_exist_in_current_bundle() {
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let modules_dir = crate_root
        .ancestors()
        .nth(3)
        .map(|root| root.join(".GOV/spec/master-spec-v02.193/spec-modules"));
    let Some(modules_dir) = modules_dir.filter(|d| d.is_dir()) else {
        eprintln!("SKIP mt207_spec_seed_anchors: spec bundle not present in this checkout");
        return;
    };
    for row in spec_enrichment_seed() {
        let module_path = modules_dir.join(row.target_module);
        assert!(
            module_path.is_file(),
            "{}: spec module {} does not exist",
            row.seed_id,
            row.target_module
        );
        let content = std::fs::read_to_string(&module_path).expect("read spec module");
        assert!(
            content.contains(row.target_anchor),
            "{}: anchor '{}' not found in {} — seed row points at a vanished anchor",
            row.seed_id,
            row.target_anchor,
            row.target_module
        );
    }
}

/// MT-204/MT-207: the model-facing manual and the prepared spec seed must
/// name the full freshness verdict vocabulary. Non-page corpus rows are now
/// freshness authority too, so stale docs here would teach models to ignore
/// tool, feature, or legacy-alias drift.
#[test]
fn mt204_freshness_docs_name_every_verdict_kind() {
    let verdicts = [
        FreshnessVerdictKind::Current.as_str(),
        FreshnessVerdictKind::StaleContent.as_str(),
        FreshnessVerdictKind::MissingPage.as_str(),
        FreshnessVerdictKind::UncoveredSurface.as_str(),
        FreshnessVerdictKind::DanglingAnchor.as_str(),
        FreshnessVerdictKind::UnseededVersion.as_str(),
        FreshnessVerdictKind::MissingToolEntry.as_str(),
        FreshnessVerdictKind::StaleToolEntry.as_str(),
        FreshnessVerdictKind::MissingFeatureEntry.as_str(),
        FreshnessVerdictKind::StaleFeatureEntry.as_str(),
        FreshnessVerdictKind::MissingLegacyAlias.as_str(),
        FreshnessVerdictKind::StaleLegacyAlias.as_str(),
    ];

    let corpus = seed_corpus();
    let manual = corpus
        .pages
        .iter()
        .find(|page| page.slug == "usermanual-surface")
        .expect("usermanual surface page seeded");
    let manual_text = manual
        .sections
        .iter()
        .map(|section| section.body_md.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let spec_text = spec_enrichment_seed()
        .iter()
        .find(|row| row.seed_id == "UMSPEC-002")
        .expect("UMSPEC-002 seed row")
        .proposed_wording_md;

    for verdict in verdicts {
        assert!(
            manual_text.contains(verdict),
            "seeded usermanual-surface page must document freshness verdict {verdict}"
        );
        assert!(
            spec_text.contains(verdict),
            "UMSPEC-002 must document freshness verdict {verdict}"
        );
    }

    assert!(
        manual_text.contains("changed pages, tool entries, feature entries, and legacy aliases"),
        "recovery guidance must say resync covers non-page corpus rows"
    );
    assert!(
        !manual_text.contains("only changed pages are written"),
        "recovery guidance must not claim resync writes only pages"
    );
}

// ---------------------------------------------------------------------------
// MT-208 fixtures (the families not already driven by the API tests).
// ---------------------------------------------------------------------------

/// MT-208: missing-page fixture — deleting a seeded page yields the
/// missing_page freshness verdict, and reseeding restores it.
#[tokio::test]
async fn mt208_missing_page_fixture_detected_and_healed() {
    let scope = seeded_scope().await;
    let storage = scope.storage();
    let store = scope.store();
    assert_eq!(
        delete_page(&store, "quickstart-editor")
            .await
            .expect("delete"),
        1
    );

    let report = check_freshness(&storage).await.expect("freshness");
    assert!(!report.fresh);
    assert!(
        report.verdicts.iter().any(|v| {
            v.kind == FreshnessVerdictKind::MissingPage && v.subject == "quickstart-editor"
        }),
        "missing page must be detected"
    );

    store.ensure_seeded().await.expect("healing reseed");
    assert!(
        store
            .get_page_by_slug("quickstart-editor")
            .await
            .expect("re-query")
            .is_some(),
        "reseed restores the deleted page"
    );
}

/// MT-208: legacy redirect fixture — known aliases resolve, unknown aliases
/// do not (no fuzzy/implicit resolution).
#[tokio::test]
async fn mt208_legacy_redirect_fixture() {
    let scope = seeded_scope().await;
    let store = scope.store();
    let alias = store
        .get_legacy_alias("model_manual_get")
        .await
        .expect("alias query")
        .expect("model_manual_get maps");
    assert_eq!(alias.canonical_kind, "route");
    assert_eq!(alias.canonical_ref, "/usermanual/legacy/model-manual");
    assert!(store
        .get_legacy_alias("model_manual_get_v2_definitely_unknown")
        .await
        .expect("unknown alias query")
        .is_none());
}

/// MT-208: visual-navigation fixture — an orphan page (nothing links to it)
/// is DETECTED by the reachability audit; the seeded corpus itself has no
/// orphans.
#[tokio::test]
async fn mt208_orphan_page_fixture_detected() {
    let scope = seeded_scope().await;
    let store = scope.store();
    assert!(
        unreachable_pages(&store).await.expect("audit").is_empty(),
        "seeded corpus must have no orphans"
    );
    let orphan_slug = insert_orphan_page(&store).await.expect("insert orphan");
    let orphans = unreachable_pages(&store).await.expect("audit 2");
    assert!(
        orphans.contains(&orphan_slug),
        "navigation audit must flag the orphan (got {orphans:?})"
    );
}
