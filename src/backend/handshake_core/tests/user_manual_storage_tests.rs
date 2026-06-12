//! WP-KERNEL-009 UserManual storage proof against REAL PostgreSQL:
//! * MT-193 UserManualNamingMigrationPlan — plan coverage of every legacy
//!   `model_manual` file in the crate + deterministic alias resolution.
//! * MT-194 UserManualStorageModel — migration 0310 tables, idempotent seed,
//!   receipts, version metadata, ordered sections, tampered-child healing.
//! * MT-195 UserManualBuildUpdateRule — every declared WP-009 surface has
//!   manual coverage in the DATABASE, and removing coverage is DETECTED.
//!
//! No SQLite, no mocks: every test runs in a fresh isolated schema on the
//! managed cluster with the full migration chain applied.

mod knowledge_pg_support;
#[allow(dead_code)]
mod user_manual_support;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use handshake_core::user_manual::freshness::{check_freshness, FreshnessVerdictKind};
use handshake_core::user_manual::migration_plan::naming_migration_plan;
use handshake_core::user_manual::registry::wp009_surface_registry;
use handshake_core::user_manual::seed::{corpus_hash, ensure_seeded, seed_corpus};
use handshake_core::user_manual::store::UserManualStore;
use handshake_core::user_manual::USER_MANUAL_VERSION;
use sqlx::{Connection, Row};

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
                || row.legacy_path.trim_end_matches('/').ends_with(rel.as_str())
        });
        assert!(
            covered,
            "legacy file {repo_rel} has NO naming-migration plan row (MT-193): \
             add a PlanRow before introducing new model_manual surfaces"
        );
    }
}

/// MT-193 + MT-203 (data layer): seeded aliases resolve deterministically —
/// route aliases point at registered /usermanual surfaces, page aliases point
/// at stored pages. An alias that resolves to nothing is split-brain.
#[tokio::test]
async fn mt193_every_legacy_alias_resolves_to_canonical() {
    let kpg = skip_if_no_pg!(
        knowledge_pg_support::knowledge_pg().await,
        "mt193_alias_resolution"
    );
    ensure_seeded(&kpg.db).await.expect("seed corpus");
    let store = UserManualStore::new(&kpg.db);

    let aliases = store.list_legacy_aliases().await.expect("list aliases");
    assert!(
        aliases.len() >= 6,
        "expected at least the 3 Tauri + 3 IPC legacy aliases, got {}",
        aliases.len()
    );
    let registered_routes: BTreeSet<&str> = wp009_surface_registry()
        .iter()
        .map(|s| s.route)
        .collect();
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
        assert!(alias.deprecation_note.contains("Deprecated since UserManual"));
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
    let kpg = skip_if_no_pg!(
        knowledge_pg_support::knowledge_pg().await,
        "mt194_migration_tables"
    );
    let mut conn = kpg.raw_connection().await;
    for table in [
        "user_manual_pages",
        "user_manual_sections",
        "user_manual_anchors",
        "user_manual_tool_entries",
        "user_manual_feature_entries",
        "user_manual_versions",
        "user_manual_legacy_aliases",
    ] {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM information_schema.tables \
             WHERE table_schema = current_schema() AND table_name = $1)",
        )
        .bind(table)
        .fetch_one(&mut conn)
        .await
        .expect("schema query");
        assert!(exists, "migration 0310 did not create {table}");
    }
    conn.close().await.ok();
}

/// MT-194: seeding is idempotent (hash short-circuit), receipts are appended
/// per changed page on the FIRST run and NOT spammed on re-seed.
#[tokio::test]
async fn mt194_seed_is_idempotent_and_receipted() {
    let kpg = skip_if_no_pg!(
        knowledge_pg_support::knowledge_pg().await,
        "mt194_seed_idempotent"
    );
    let first = ensure_seeded(&kpg.db).await.expect("first seed");
    assert_eq!(first.pages_changed, first.pages_total, "first seed writes all pages");
    assert!(first.tools_total > 100, "registry + legacy catalog expected (got {})", first.tools_total);
    assert!(first.version_receipt_event_id.is_some());

    let mut conn = kpg.raw_connection().await;
    let receipts_after_first: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger WHERE event_type = 'KNOWLEDGE_USER_MANUAL_ENTRY_RECORDED'",
    )
    .fetch_one(&mut conn)
    .await
    .expect("ledger count");
    // One receipt per seeded page + one corpus summary receipt.
    assert_eq!(
        receipts_after_first,
        (first.pages_total + 1) as i64,
        "expected one receipt per page plus the corpus receipt"
    );

    let second = ensure_seeded(&kpg.db).await.expect("second seed");
    assert_eq!(second.pages_changed, 0, "re-seed must short-circuit on content hash");
    assert_eq!(second.tools_changed, 0);
    assert_eq!(second.features_changed, 0);
    assert!(second.version_receipt_event_id.is_none(), "no-change reseed must not receipt");

    let receipts_after_second: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger WHERE event_type = 'KNOWLEDGE_USER_MANUAL_ENTRY_RECORDED'",
    )
    .fetch_one(&mut conn)
    .await
    .expect("ledger count 2");
    assert_eq!(receipts_after_first, receipts_after_second, "idempotent reseed appended receipts");
    conn.close().await.ok();
}

/// MT-194: version metadata row carries the corpus hash, counts, and a
/// resolvable EventLedger receipt id.
#[tokio::test]
async fn mt194_seed_records_version_metadata() {
    let kpg = skip_if_no_pg!(
        knowledge_pg_support::knowledge_pg().await,
        "mt194_version_metadata"
    );
    let report = ensure_seeded(&kpg.db).await.expect("seed");
    let store = UserManualStore::new(&kpg.db);
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

    let mut conn = kpg.raw_connection().await;
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM kernel_event_ledger WHERE event_id = $1)",
    )
    .bind(&receipt_id)
    .fetch_one(&mut conn)
    .await
    .expect("receipt lookup");
    assert!(exists, "version receipt {receipt_id} not in kernel_event_ledger");
    conn.close().await.ok();
}

/// MT-194: page reads return ordered sections and anchors; tampered child
/// rows are healed by reseed even when the page hash still matches.
#[tokio::test]
async fn mt194_sections_ordered_and_tampered_children_heal() {
    let kpg = skip_if_no_pg!(
        knowledge_pg_support::knowledge_pg().await,
        "mt194_sections_ordered"
    );
    ensure_seeded(&kpg.db).await.expect("seed");
    let store = UserManualStore::new(&kpg.db);
    let (page, sections, anchors) = store
        .get_page_by_slug("manual-toc")
        .await
        .expect("toc query")
        .expect("manual-toc seeded");
    assert_eq!(page.manual_version, USER_MANUAL_VERSION);
    assert!(!sections.is_empty());
    for (index, section) in sections.iter().enumerate() {
        assert_eq!(section.position as usize, index, "sections must come back ordered");
    }
    assert!(!anchors.is_empty());

    // Tamper: delete the page's sections WITHOUT touching the page hash.
    let mut conn = kpg.raw_connection().await;
    sqlx::query("DELETE FROM user_manual_sections WHERE page_id = $1")
        .bind(&page.page_id)
        .execute(&mut conn)
        .await
        .expect("tamper sections");
    conn.close().await.ok();

    let report = ensure_seeded(&kpg.db).await.expect("healing reseed");
    assert!(report.pages_changed >= 1, "reseed must heal the gutted page");
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
    let kpg = skip_if_no_pg!(
        knowledge_pg_support::knowledge_pg().await,
        "mt195_registry_coverage"
    );
    ensure_seeded(&kpg.db).await.expect("seed");
    let store = UserManualStore::new(&kpg.db);
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
    let kpg = skip_if_no_pg!(
        knowledge_pg_support::knowledge_pg().await,
        "mt195_uncovered_detection"
    );
    ensure_seeded(&kpg.db).await.expect("seed");

    let victim = "/knowledge/code/symbols";
    let mut conn = kpg.raw_connection().await;
    sqlx::query(
        "DELETE FROM user_manual_anchors WHERE anchor_kind = 'http_route' AND anchor_value = $1",
    )
    .bind(victim)
    .execute(&mut conn)
    .await
    .expect("remove coverage");
    conn.close().await.ok();

    let report = check_freshness(&kpg.db).await.expect("freshness");
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

    // Healing: reseed restores coverage (page hash unchanged but anchors
    // missing -> child-count check forces the rewrite).
    ensure_seeded(&kpg.db).await.expect("healing reseed");
    let healed = check_freshness(&kpg.db).await.expect("freshness after heal");
    assert!(
        !healed
            .verdicts
            .iter()
            .any(|v| v.kind == FreshnessVerdictKind::UncoveredSurface),
        "reseed must restore registry coverage"
    );
}

/// MT-194: bounded search finds seeded content; LIKE wildcards are escaped
/// (a '%' query is literal, not match-everything).
#[tokio::test]
async fn mt194_search_is_bounded_and_literal() {
    let kpg = skip_if_no_pg!(knowledge_pg_support::knowledge_pg().await, "mt194_search");
    ensure_seeded(&kpg.db).await.expect("seed");
    let store = UserManualStore::new(&kpg.db);

    let hits = store.search("PostgreSQL", 25).await.expect("search");
    assert!(!hits.is_empty(), "seeded corpus mentions PostgreSQL");
    assert!(hits.len() <= 25);

    let nonsense = store
        .search("zzz-no-such-term-anywhere", 25)
        .await
        .expect("nonsense search");
    assert!(nonsense.is_empty());

    let pages_total = store.list_pages(None, None, 500).await.expect("pages").len();
    let wildcard = store.search("%", 500).await.expect("wildcard search");
    assert!(
        wildcard.len() < pages_total,
        "'%' must be a literal character, not a match-everything pattern"
    );
}
