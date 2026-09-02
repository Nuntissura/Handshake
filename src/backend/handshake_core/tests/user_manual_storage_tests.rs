//! WP-KERNEL-009 UserManual storage proof against embedded SurrealDB:
//! * MT-193 UserManualNamingMigrationPlan — plan coverage of every legacy
//!   `model_manual` file in the crate + deterministic alias resolution.
//! * MT-194 UserManualStorageModel — migration 0310 tables, idempotent seed,
//!   receipts, version metadata, ordered sections, tampered-child healing.
//! * MT-195 UserManualBuildUpdateRule — every declared WP-009 surface has
//!   manual coverage in the DATABASE, and removing coverage is DETECTED.
//!
//! No mocks: every test runs in a fresh isolated embedded store with the
//! canonical SurrealDB schema applied.

#[allow(dead_code)]
mod user_manual_support;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use handshake_core::kernel::KernelEventType;
use handshake_core::model_manual::{model_manual, render_model_manual_markdown};
use handshake_core::storage::surreal::SurrealDatabase;
use handshake_core::storage::Database;
use handshake_core::user_manual::freshness::{check_freshness, FreshnessVerdictKind};
use handshake_core::user_manual::migration_plan::naming_migration_plan;
use handshake_core::user_manual::registry::wp009_surface_registry;
use handshake_core::user_manual::seed::{corpus_hash, ensure_seeded, seed_corpus};
use handshake_core::user_manual::store::UserManualStore;
use handshake_core::user_manual::USER_MANUAL_VERSION;
use user_manual_support::manual_test_backend;

async fn manual_receipt_count(db: &SurrealDatabase) -> usize {
    let corpus = seed_corpus();
    let subjects = corpus
        .pages
        .iter()
        .map(|page| page.slug.as_str())
        .chain(std::iter::once(USER_MANUAL_VERSION));
    let mut count = 0;
    for subject in subjects {
        count += db
            .list_kernel_events_for_aggregate("user_manual_entry", subject)
            .await
            .expect("list manual receipt events")
            .iter()
            .filter(|event| event.event_type == KernelEventType::KnowledgeUserManualEntryRecorded)
            .count();
    }
    count
}

async fn manual_receipt_exists(db: &SurrealDatabase, event_id: &str) -> bool {
    let corpus = seed_corpus();
    let subjects = corpus
        .pages
        .iter()
        .map(|page| page.slug.as_str())
        .chain(std::iter::once(USER_MANUAL_VERSION));
    for subject in subjects {
        if db
            .list_kernel_events_for_aggregate("user_manual_entry", subject)
            .await
            .expect("list manual receipt events")
            .iter()
            .any(|event| {
                event.event_id == event_id
                    && event.event_type == KernelEventType::KnowledgeUserManualEntryRecorded
            })
        {
            return true;
        }
    }
    false
}

async fn remove_route_anchor_via_typed_store(db: &SurrealDatabase, route: &str) -> bool {
    // MT-141 disposition: the public typed store has no child-anchor delete
    // operation. Re-seed one canonical page through its typed API with this
    // route anchor removed; freshness still proves uncovered-surface detection
    // and the subsequent seed proves healing.
    let mut corpus = seed_corpus();
    let page = corpus
        .pages
        .iter_mut()
        .find(|page| {
            page.anchors
                .iter()
                .any(|anchor| anchor.anchor_kind == "http_route" && anchor.anchor_value == route)
        })
        .expect("route anchor exists in seed corpus");
    let before = page.anchors.len();
    page.anchors
        .retain(|anchor| !(anchor.anchor_kind == "http_route" && anchor.anchor_value == route));
    assert_eq!(page.anchors.len(), before - 1);
    let store = UserManualStore::new(db);
    let (_, changed) = store
        .upsert_page(page, USER_MANUAL_VERSION, "current")
        .await
        .expect("remove route anchor through typed store");
    changed
}

// ---------------------------------------------------------------------------
// MT-193: the naming migration plan is complete and deterministic.
// ---------------------------------------------------------------------------

fn collect_model_manual_files(dir: &Path, hits: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_model_manual_files(&path, hits);
        } else if path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.contains("model_manual"))
        {
            hits.push(path);
        }
    }
}

/// MT-193: every `*model_manual*` source file in this crate is covered by a
/// migration-plan row. A NEW legacy-named file without a plan row fails here
/// — the split-brain door stays closed.
#[test]
fn mt193_every_legacy_model_manual_file_is_plan_covered() {
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    collect_model_manual_files(&crate_root.join("src"), &mut files);
    collect_model_manual_files(&crate_root.join("tests"), &mut files);
    assert!(
        !files.is_empty(),
        "expected legacy model_manual files during the bridge phase"
    );

    let plan = naming_migration_plan();
    for file in files {
        let rel = file
            .strip_prefix(&crate_root)
            .expect("file under crate root")
            .to_string_lossy()
            .replace('\\', "/");
        let repo_rel = format!("src/backend/handshake_core/{rel}");
        let covered = plan.rows.iter().any(|row| {
            repo_rel.starts_with(row.legacy_path.trim_end_matches('/'))
                || row
                    .legacy_path
                    .trim_end_matches('/')
                    .ends_with(rel.as_str())
        });
        assert!(
            covered,
            "legacy file {repo_rel} has NO naming-migration plan row (MT-193): \
             add a PlanRow before introducing new model_manual surfaces"
        );
    }
}

/// MT-193: the legacy generated projection may remain only as a compatibility
/// projection. It must not tell no-context readers that ModelManual is still
/// the canonical authority.
#[test]
fn mt193_generated_model_manual_projection_names_usermanual_authority() {
    fn has_stale_manual_version_phrase(haystack: &str, phrase: &str) -> bool {
        let mut offset = 0;
        while let Some(relative) = haystack[offset..].find(phrase) {
            let index = offset + relative;
            let has_user_prefix = index >= 5 && &haystack[index - 5..index] == "USER_";
            if !has_user_prefix {
                return true;
            }
            offset = index + phrase.len();
        }
        false
    }

    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = crate_root
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .expect("crate is under repo/src/backend/handshake_core");
    let projection = std::fs::read_to_string(repo_root.join("app/MODEL_MANUAL.md"))
        .expect("read generated legacy projection");

    assert!(
        projection.contains(
            "This legacy ModelManual projection is a compatibility artifact. UserManual is canonical."
        ),
        "legacy projection must name UserManual as canonical authority"
    );
    assert!(
        !projection.contains("The Rust ModelManual manifest remains canonical."),
        "legacy projection still claims ModelManual is canonical"
    );
    assert!(
        projection
            .contains("HBR-MAN-001 requires every wired surface diff to update UserManual content"),
        "legacy HBR-MAN wording must point at UserManual"
    );
    let stale_hbr_phrases = [
        "without a MANUAL_VERSION bump",
        "wired-surface diff without a MANUAL_VERSION bump",
        "MANUAL_VERSION not bumped",
        "MANUAL_VERSION not bumped after a wired-surface diff",
        "Bump MANUAL_VERSION in the same commit as the wired-surface diff",
        "current MANUAL_VERSION",
        "Confirm MANUAL_VERSION was bumped on the wired-surface diff",
    ];
    for phrase in stale_hbr_phrases {
        assert!(
            !has_stale_manual_version_phrase(&projection, phrase),
            "legacy projection still carries stale HBR-MAN-001 guidance: {phrase}"
        );
    }

    let rendered = render_model_manual_markdown(model_manual());
    assert!(
        rendered.contains(
            "This legacy ModelManual projection is a compatibility artifact. UserManual is canonical."
        ),
        "legacy projection generator must name UserManual as canonical authority"
    );
    assert!(
        !rendered.contains("The Rust ModelManual manifest remains canonical."),
        "legacy projection generator still claims ModelManual is canonical"
    );
    assert!(
        rendered
            .contains("HBR-MAN-001 requires every wired surface diff to update UserManual content"),
        "legacy source constraint must point at UserManual before regeneration"
    );
    for phrase in stale_hbr_phrases {
        assert!(
            !has_stale_manual_version_phrase(&rendered, phrase),
            "legacy projection generator still carries stale HBR-MAN-001 guidance: {phrase}"
        );
    }
}

/// MT-193 + MT-203 (data layer): seeded aliases resolve deterministically —
/// route aliases point at registered /usermanual surfaces, page aliases point
/// at stored pages. An alias that resolves to nothing is split-brain.
#[tokio::test]
async fn mt193_every_legacy_alias_resolves_to_canonical() {
    let backend = manual_test_backend().await.expect("open embedded backend");
    ensure_seeded(&backend.db).await.expect("seed corpus");
    let store = UserManualStore::new(&backend.db);

    let aliases = store.list_legacy_aliases().await.expect("list aliases");
    assert!(
        aliases.len() >= 6,
        "expected at least the 3 Tauri + 3 IPC legacy aliases, got {}",
        aliases.len()
    );
    let registered_routes: BTreeSet<&str> =
        wp009_surface_registry().iter().map(|s| s.route).collect();
    for alias in &aliases {
        match alias.canonical_kind.as_str() {
            "route" => assert!(
                registered_routes.contains(alias.canonical_ref.as_str()),
                "alias {} -> route {} is not a registered surface",
                alias.alias,
                alias.canonical_ref
            ),
            "page" => assert!(
                store
                    .get_page_by_slug(&alias.canonical_ref)
                    .await
                    .expect("page lookup")
                    .is_some(),
                "alias {} -> page {} does not exist",
                alias.alias,
                alias.canonical_ref
            ),
            "tool" => assert!(
                store
                    .get_tool_entry(&alias.canonical_ref)
                    .await
                    .expect("tool lookup")
                    .is_some(),
                "alias {} -> tool {} does not exist",
                alias.alias,
                alias.canonical_ref
            ),
            other => panic!("unknown canonical_kind {other}"),
        }
        assert!(alias
            .deprecation_note
            .contains("Deprecated since UserManual"));
    }
    // The exact legacy callables stay mapped.
    let names: BTreeSet<&str> = aliases.iter().map(|a| a.alias.as_str()).collect();
    for expected in [
        "model_manual_get",
        "model_manual_list_commands",
        "model_manual_search",
        "kernel.model_manual.get",
        "kernel.model_manual.list_commands",
        "kernel.model_manual.search",
    ] {
        assert!(names.contains(expected), "missing alias row {expected}");
    }
}

// ---------------------------------------------------------------------------
// MT-194: storage model.
// ---------------------------------------------------------------------------

/// MT-194: migration 0310 creates the seven user_manual_* tables.
#[tokio::test]
async fn mt194_migration_0310_creates_user_manual_tables() {
    let backend = manual_test_backend().await.expect("open embedded backend");
    ensure_seeded(&backend.db).await.expect("seed");
    let store = UserManualStore::new(&backend.db);
    let pages = store
        .list_pages(None, None, 500)
        .await
        .expect("read user_manual_pages through typed store");
    let page_id = pages.first().expect("seeded page").page_id.clone();
    assert!(
        store.sections_for(&page_id).await.is_ok(),
        "user_manual_sections is not readable through typed store"
    );
    assert!(
        store.anchors_by_kind("http_route").await.is_ok(),
        "user_manual_anchors is not readable through typed store"
    );
    assert!(
        store.list_tool_entries(None, None, 500).await.is_ok(),
        "user_manual_tool_entries is not readable through typed store"
    );
    assert!(
        store.list_feature_entries(500).await.is_ok(),
        "user_manual_feature_entries is not readable through typed store"
    );
    assert!(
        store.get_version(USER_MANUAL_VERSION).await.is_ok(),
        "user_manual_versions is not readable through typed store"
    );
    assert!(
        store.list_legacy_aliases().await.is_ok(),
        "user_manual_legacy_aliases is not readable through typed store"
    );
}

/// MT-194: seeding is idempotent (hash short-circuit), receipts are appended
/// per changed page on the FIRST run and NOT spammed on re-seed.
#[tokio::test]
async fn mt194_seed_is_idempotent_and_receipted() {
    let backend = manual_test_backend().await.expect("open embedded backend");
    let first = ensure_seeded(&backend.db).await.expect("first seed");
    assert_eq!(
        first.pages_changed, first.pages_total,
        "first seed writes all pages"
    );
    assert!(
        first.tools_total > 100,
        "registry + legacy catalog expected (got {})",
        first.tools_total
    );
    assert!(first.version_receipt_event_id.is_some());

    let receipts_after_first = manual_receipt_count(&backend.db).await;
    // One receipt per seeded page + one corpus summary receipt.
    assert_eq!(
        receipts_after_first,
        first.pages_total + 1,
        "expected one receipt per page plus the corpus receipt"
    );

    let second = ensure_seeded(&backend.db).await.expect("second seed");
    assert_eq!(
        second.pages_changed, 0,
        "re-seed must short-circuit on content hash"
    );
    assert_eq!(second.tools_changed, 0);
    assert_eq!(second.features_changed, 0);
    assert!(
        second.version_receipt_event_id.is_none(),
        "no-change reseed must not receipt"
    );

    let receipts_after_second = manual_receipt_count(&backend.db).await;
    assert_eq!(
        receipts_after_first, receipts_after_second,
        "idempotent reseed appended receipts"
    );
}

/// MT-194: version metadata row carries the corpus hash, counts, and a
/// resolvable EventLedger receipt id.
#[tokio::test]
async fn mt194_seed_records_version_metadata() {
    let backend = manual_test_backend().await.expect("open embedded backend");
    let report = ensure_seeded(&backend.db).await.expect("seed");
    let store = UserManualStore::new(&backend.db);
    let version = store
        .get_version(USER_MANUAL_VERSION)
        .await
        .expect("version query")
        .expect("version row exists");
    assert_eq!(version.seed_content_hash, corpus_hash(&seed_corpus()));
    assert_eq!(version.page_count as usize, report.pages_total);
    assert_eq!(version.tool_count as usize, report.tools_total);
    assert_eq!(version.feature_count as usize, report.features_total);
    let receipt_id = version.ledger_event_id.expect("version receipt id");

    let exists = manual_receipt_exists(&backend.db, &receipt_id).await;
    assert!(
        exists,
        "version receipt {receipt_id} not in kernel_event_ledger"
    );
}

/// MT-194: page reads return ordered sections and anchors; typed tampered child
/// rows are healed by reseed.
#[tokio::test]
async fn mt194_sections_ordered_and_tampered_children_heal() {
    let backend = manual_test_backend().await.expect("open embedded backend");
    ensure_seeded(&backend.db).await.expect("seed");
    let store = UserManualStore::new(&backend.db);
    let (page, sections, anchors) = store
        .get_page_by_slug("manual-toc")
        .await
        .expect("toc query")
        .expect("manual-toc seeded");
    assert_eq!(page.manual_version, USER_MANUAL_VERSION);
    assert!(!sections.is_empty());
    for (index, section) in sections.iter().enumerate() {
        assert_eq!(
            section.position as usize, index,
            "sections must come back ordered"
        );
    }
    assert!(!anchors.is_empty());

    // MT-141 disposition: no public typed child-delete operation exists. Use
    // the typed page upsert to alter one child while preserving the healing
    // proof; this supported path also updates the parent content hash.
    let mut tampered_page = seed_corpus()
        .pages
        .into_iter()
        .find(|candidate| candidate.slug == "manual-toc")
        .expect("manual-toc exists in seed corpus");
    tampered_page.sections[0].title = "tampered child section".to_owned();
    assert_eq!(
        store
            .upsert_page(&tampered_page, USER_MANUAL_VERSION, "current")
            .await
            .expect("tamper child through typed store")
            .1,
        true,
        "typed tamper must rewrite the page and child rows"
    );

    let report = ensure_seeded(&backend.db).await.expect("healing reseed");
    assert!(
        report.pages_changed >= 1,
        "reseed must heal the gutted page"
    );
    let (_, healed_sections, _) = store
        .get_page_by_slug("manual-toc")
        .await
        .expect("re-query")
        .expect("page still there");
    assert_eq!(healed_sections.len(), sections.len(), "sections restored");
}

// ---------------------------------------------------------------------------
// MT-195: the build-update rule.
// ---------------------------------------------------------------------------

/// MT-195: in a seeded database, EVERY declared WP-009 surface has at least
/// one http_route anchor on a manual page (build-rule law, spec 10.15.8).
#[tokio::test]
async fn mt195_every_registry_surface_has_db_coverage() {
    let backend = manual_test_backend().await.expect("open embedded backend");
    ensure_seeded(&backend.db).await.expect("seed");
    let store = UserManualStore::new(&backend.db);
    let anchors = store
        .anchors_by_kind("http_route")
        .await
        .expect("route anchors");
    let covered: BTreeSet<(String, String)> = anchors
        .iter()
        .map(|a| (a.http_method.clone(), a.anchor_value.clone()))
        .collect();
    for surface in wp009_surface_registry() {
        assert!(
            covered.contains(&(surface.method.to_string(), surface.route.to_string())),
            "surface {} {} ({}) has no manual coverage in the database",
            surface.method,
            surface.route,
            surface.surface_id
        );
    }
    // The tool catalog covers the registry too.
    for surface in wp009_surface_registry() {
        assert!(
            store
                .get_tool_entry(surface.surface_id)
                .await
                .expect("tool lookup")
                .is_some(),
            "surface {} missing from the tool catalog",
            surface.surface_id
        );
    }
}

/// MT-195 negative: removing a surface's coverage is DETECTED (the gate can
/// actually fail) — freshness flips to uncovered_surface for that route.
#[tokio::test]
async fn mt195_uncovered_surface_detection_fires() {
    let backend = manual_test_backend().await.expect("open embedded backend");
    ensure_seeded(&backend.db).await.expect("seed");

    let victim = "/knowledge/code/symbols";
    assert_eq!(
        remove_route_anchor_via_typed_store(&backend.db, victim).await,
        true,
        "typed fixture must remove one route-coverage anchor"
    );

    let report = check_freshness(&backend.db).await.expect("freshness");
    assert!(!report.fresh, "gutted coverage must not report fresh");
    assert!(
        report.verdicts.iter().any(|v| {
            v.kind == FreshnessVerdictKind::UncoveredSurface && v.subject.contains(victim)
        }),
        "expected uncovered_surface verdict for {victim}; got {:?}",
        report
            .verdicts
            .iter()
            .filter(|v| v.kind.is_problem())
            .collect::<Vec<_>>()
    );

    // Healing: reseed restores coverage after the typed page upsert changed
    // the page hash and removed the route anchor.
    ensure_seeded(&backend.db).await.expect("healing reseed");
    let healed = check_freshness(&backend.db)
        .await
        .expect("freshness after heal");
    assert!(
        !healed
            .verdicts
            .iter()
            .any(|v| v.kind == FreshnessVerdictKind::UncoveredSurface),
        "reseed must restore registry coverage"
    );
}

/// MT-204: freshness covers the full seed corpus, not just page rows. Tool,
/// feature, and legacy-alias row tampering must be visible because those rows
/// are operator/model navigation authority too.
#[tokio::test]
async fn mt204_freshness_detects_non_page_corpus_tampering() {
    let backend = manual_test_backend().await.expect("open embedded backend");
    ensure_seeded(&backend.db).await.expect("seed");
    let store = UserManualStore::new(&backend.db);

    let corpus = seed_corpus();
    let tool_id = corpus
        .tools
        .first()
        .expect("seed has tool entries")
        .tool_id
        .clone();
    let feature_id = corpus
        .features
        .first()
        .expect("seed has feature entries")
        .feature_id
        .clone();
    let alias = corpus
        .aliases
        .first()
        .expect("seed has legacy aliases")
        .alias
        .clone();

    let mut tool = store
        .get_tool_entry(&tool_id)
        .await
        .expect("read tool before tamper")
        .expect("seeded tool exists");
    tool.description = "tampered visible tool description".to_owned();
    assert_eq!(
        store
            .upsert_tool_entry(&tool)
            .await
            .expect("tamper tool through typed store"),
        true,
        "typed fixture must update one tool entry"
    );

    let mut feature = store
        .get_feature_entry(&feature_id)
        .await
        .expect("read feature before tamper")
        .expect("seeded feature exists");
    feature.description = "tampered visible feature description".to_owned();
    assert_eq!(
        store
            .upsert_feature_entry(&feature)
            .await
            .expect("tamper feature through typed store"),
        true,
        "typed fixture must update one feature entry"
    );

    let mut alias_row = store
        .get_legacy_alias(&alias)
        .await
        .expect("read alias before tamper")
        .expect("seeded alias exists");
    alias_row.canonical_ref = "tampered-alias-target".to_owned();
    assert_eq!(
        store
            .upsert_legacy_alias(&alias_row)
            .await
            .expect("tamper alias through typed store"),
        true,
        "typed fixture must update one legacy alias"
    );

    let report = check_freshness(&backend.db).await.expect("freshness");
    assert!(
        !report.fresh,
        "non-page corpus tampering must make the manual stale"
    );
    assert!(
        report
            .verdicts
            .iter()
            .any(|v| v.kind == FreshnessVerdictKind::StaleToolEntry && v.subject == tool_id),
        "tampered tool entry must yield stale_tool_entry; got {:?}",
        report.verdicts
    );
    assert!(
        report.verdicts.iter().any(|v| {
            v.kind == FreshnessVerdictKind::StaleFeatureEntry && v.subject == feature_id
        }),
        "tampered feature entry must yield stale_feature_entry; got {:?}",
        report.verdicts
    );
    assert!(
        report
            .verdicts
            .iter()
            .any(|v| v.kind == FreshnessVerdictKind::StaleLegacyAlias && v.subject == alias),
        "tampered alias row must yield stale_legacy_alias; got {:?}",
        report.verdicts
    );

    let healed = ensure_seeded(&backend.db).await.expect("healing reseed");
    assert!(
        healed.tools_changed >= 1,
        "reseed must heal visible tool row drift even when content_hash was unchanged"
    );
    assert!(
        healed.features_changed >= 1,
        "reseed must heal visible feature row drift even when content_hash was unchanged"
    );
    assert!(
        healed.aliases_changed >= 1,
        "reseed must heal visible alias row drift"
    );
    let fresh = check_freshness(&backend.db)
        .await
        .expect("freshness after heal");
    assert!(
        fresh.fresh,
        "reseed must restore full corpus freshness: {:?}",
        fresh.verdicts
    );
}

/// MT-194: bounded search finds seeded content; LIKE wildcards are escaped
/// (a '%' query is literal, not match-everything).
#[tokio::test]
async fn mt194_search_is_bounded_and_literal() {
    let backend = manual_test_backend().await.expect("open embedded backend");
    ensure_seeded(&backend.db).await.expect("seed");
    let store = UserManualStore::new(&backend.db);

    let hits = store.search("SurrealDB", 25).await.expect("search");
    assert!(!hits.is_empty(), "seeded corpus mentions SurrealDB");
    assert!(hits.len() <= 25);

    let nonsense = store
        .search("zzz-no-such-term-anywhere", 25)
        .await
        .expect("nonsense search");
    assert!(nonsense.is_empty());

    let pages_total = store
        .list_pages(None, None, 500)
        .await
        .expect("pages")
        .len();
    let wildcard = store.search("%", 500).await.expect("wildcard search");
    assert!(
        wildcard.len() < pages_total,
        "'%' must be a literal character, not a match-everything pattern"
    );
}
