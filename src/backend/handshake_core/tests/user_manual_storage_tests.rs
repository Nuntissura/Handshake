//! WP-KERNEL-009 UserManual storage proof against embedded SurrealDB:
//! * MT-193 UserManualNamingMigrationPlan — plan coverage of every legacy
//!   `model_manual` file in the crate + deterministic alias resolution.
//! * MT-194 UserManualStorageModel — product schema, idempotent seed,
//!   receipts, version metadata, ordered sections, tampered-child healing.
//! * MT-195 UserManualBuildUpdateRule — every declared WP-009 surface has
//!   manual coverage in the DATABASE, and removing coverage is DETECTED.
//!
//! Every test runs in a fresh product-global database scope with the shipped
//! embedded schema applied.

mod surreal_test_store_support;
#[allow(dead_code)]
mod user_manual_support;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use handshake_core::model_manual::{model_manual, render_model_manual_markdown};
use handshake_core::storage::surreal::bootstrap_schema;
use handshake_core::user_manual::fixtures::{
    delete_page_sections, delete_route_anchor, inject_page_receipt_without_mutation, receipt_count,
    receipt_exists,
};
use handshake_core::user_manual::freshness::{check_freshness, FreshnessVerdictKind};
use handshake_core::user_manual::migration_plan::naming_migration_plan;
use handshake_core::user_manual::registry::wp009_surface_registry;
use handshake_core::user_manual::seed::{corpus_hash, seed_corpus};
use handshake_core::user_manual::store::{NewManualSection, NewUserManualPage, UserManualStore};
use handshake_core::user_manual::USER_MANUAL_VERSION;
use surreal_test_store_support::EmbeddedSurrealTestScope;
use user_manual_support::UserManualTestScope;

async fn seeded_scope() -> UserManualTestScope {
    let scope = UserManualTestScope::create().await;
    scope.store().ensure_seeded().await.expect("seed corpus");
    scope
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
    let scope = seeded_scope().await;
    let store = scope.store();

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

/// MT-194: the shipped embedded schema materializes every UserManual record
/// family used by the canonical corpus.
#[tokio::test]
async fn mt194_embedded_schema_materializes_user_manual_records() {
    let scope = seeded_scope().await;
    let store = scope.store();
    assert!(!store
        .list_pages(None, None, 500)
        .await
        .expect("pages")
        .is_empty());
    assert!(!store
        .anchors_by_kind("page_link")
        .await
        .expect("anchors")
        .is_empty());
    assert!(!store
        .list_tool_entries(None, None, 500)
        .await
        .expect("tools")
        .is_empty());
    assert!(!store
        .list_feature_entries(500)
        .await
        .expect("features")
        .is_empty());
    assert!(!store
        .list_legacy_aliases()
        .await
        .expect("aliases")
        .is_empty());
    assert!(store
        .get_version(USER_MANUAL_VERSION)
        .await
        .expect("version")
        .is_some());
}

/// MT-194: seeding is idempotent (hash short-circuit), receipts are appended
/// per changed page on the FIRST run and NOT spammed on re-seed.
#[tokio::test]
async fn mt194_seed_is_idempotent_and_receipted() {
    let scope = UserManualTestScope::create().await;
    let store = scope.store();
    let first = store.ensure_seeded().await.expect("first seed");
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

    let receipts_after_first = receipt_count(&store).await.expect("ledger count");
    // One receipt per seeded page + one corpus summary receipt.
    assert_eq!(
        receipts_after_first,
        first.pages_total + 1,
        "expected one receipt per page plus the corpus receipt"
    );

    let second = store.ensure_seeded().await.expect("second seed");
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

    let receipts_after_second = receipt_count(&store).await.expect("ledger count 2");
    assert_eq!(
        receipts_after_first, receipts_after_second,
        "idempotent reseed appended receipts"
    );
}

#[tokio::test]
async fn mt022_page_receipt_retry_is_stable_and_orphan_receipt_fails_closed() {
    let scope = UserManualTestScope::create().await;
    let store = scope.store();
    let page = NewUserManualPage {
        slug: "fixture-receipt-retry".to_owned(),
        title: "Receipt retry fixture".to_owned(),
        page_kind: "surface_guide",
        audience: "model",
        spec_anchors: Vec::new(),
        sections: vec![NewManualSection {
            section_kind: "purpose",
            title: "Purpose".to_owned(),
            body_md: "Prove stable mutation evidence.".to_owned(),
            body_json: None,
        }],
        anchors: Vec::new(),
    };

    let (page_id, changed) = store
        .upsert_page(&page, USER_MANUAL_VERSION, "current")
        .await
        .expect("commit page mutation");
    assert!(changed);
    let first = store
        .get_page_by_slug(&page.slug)
        .await
        .expect("read committed page")
        .expect("committed page exists")
        .0;
    let first_receipt = first
        .ledger_event_id
        .clone()
        .expect("committed page carries receipt evidence");
    let receipts_before_retry = receipt_count(&store).await.expect("receipt count");

    let retried = store
        .upsert_page(&page, USER_MANUAL_VERSION, "current")
        .await
        .expect("retry identical committed mutation");
    assert_eq!(retried, (page_id, false));
    let after_retry = store
        .get_page_by_slug(&page.slug)
        .await
        .expect("read retried page")
        .expect("retried page exists")
        .0;
    assert_eq!(
        after_retry.ledger_event_id.as_deref(),
        Some(first_receipt.as_str())
    );
    assert_eq!(
        receipt_count(&store)
            .await
            .expect("receipt count after retry"),
        receipts_before_retry,
        "identical committed retry must not append evidence"
    );

    let orphan_page = NewUserManualPage {
        slug: "fixture-orphan-receipt".to_owned(),
        title: "Orphan receipt fixture".to_owned(),
        page_kind: "surface_guide",
        audience: "model",
        spec_anchors: Vec::new(),
        sections: vec![NewManualSection {
            section_kind: "purpose",
            title: "Purpose".to_owned(),
            body_md: "Prove fail-closed receipt consistency.".to_owned(),
            body_json: None,
        }],
        anchors: Vec::new(),
    };
    let orphan_receipt = inject_page_receipt_without_mutation(&store, &orphan_page)
        .await
        .expect("inject bounded orphan receipt counterfactual");
    assert!(receipt_exists(&store, &orphan_receipt)
        .await
        .expect("orphan receipt canonical re-read"));
    let error = store
        .upsert_page(&orphan_page, USER_MANUAL_VERSION, "current")
        .await
        .expect_err("receipt without mutation must fail closed");
    assert!(
        error
            .to_string()
            .contains("receipt already exists without this mutation"),
        "unexpected fail-closed error: {error}"
    );
    assert!(store
        .get_page_by_slug(&orphan_page.slug)
        .await
        .expect("read absent orphan-receipt page")
        .is_none());
}

fn mt022_transition_page(variant: &str) -> NewUserManualPage {
    NewUserManualPage {
        slug: "fixture-predecessor-bound-reopen".to_owned(),
        title: format!("Predecessor-bound manual {variant}"),
        page_kind: "surface_guide",
        audience: "model",
        spec_anchors: vec![format!("MT-022-AC9-{variant}")],
        sections: vec![NewManualSection {
            section_kind: "purpose",
            title: "Purpose".to_owned(),
            body_md: format!("Product-global UserManual transition {variant}."),
            body_json: None,
        }],
        anchors: Vec::new(),
    }
}

#[tokio::test]
async fn mt022_product_global_a_b_a_retry_and_predecessor_receipts_survive_reopen() {
    let mut isolated = EmbeddedSurrealTestScope::create()
        .await
        .expect("allocate isolated embedded UserManual store");
    let namespace = isolated.namespace().to_owned();
    let database = isolated.database().to_owned();
    let storage = isolated
        .activate_storage()
        .await
        .expect("activate exact product-global storage");
    assert_eq!(storage.namespace(), namespace);
    assert_eq!(storage.database(), database);
    bootstrap_schema(&storage)
        .await
        .expect("bootstrap product-global UserManual schema");
    let store = UserManualStore::new(storage.clone());
    let page_a = mt022_transition_page("A");
    let page_b = mt022_transition_page("B");

    let (page_id, changed) = store
        .upsert_page(&page_a, USER_MANUAL_VERSION, "current")
        .await
        .expect("commit first A");
    assert!(changed);
    let first_a = store
        .get_page_by_slug(&page_a.slug)
        .await
        .expect("read first A")
        .expect("first A exists")
        .0;
    let first_a_receipt = first_a
        .ledger_event_id
        .clone()
        .expect("first A has canonical receipt");
    let after_first_a = receipt_count(&store).await.expect("count after first A");
    assert_eq!(
        store
            .upsert_page(&page_a, USER_MANUAL_VERSION, "current")
            .await
            .expect("retry first A"),
        (page_id.clone(), false)
    );
    assert_eq!(
        receipt_count(&store)
            .await
            .expect("count after first A retry"),
        after_first_a
    );

    assert_eq!(
        store
            .upsert_page(&page_b, USER_MANUAL_VERSION, "current")
            .await
            .expect("commit B"),
        (page_id.clone(), true)
    );
    let b = store
        .get_page_by_slug(&page_b.slug)
        .await
        .expect("read B")
        .expect("B exists")
        .0;
    let b_receipt = b.ledger_event_id.clone().expect("B has canonical receipt");
    assert_ne!(b_receipt, first_a_receipt);
    let after_b = receipt_count(&store).await.expect("count after B");
    assert_eq!(after_b, after_first_a + 1);
    assert_eq!(
        store
            .upsert_page(&page_b, USER_MANUAL_VERSION, "current")
            .await
            .expect("retry B"),
        (page_id.clone(), false)
    );
    assert_eq!(
        receipt_count(&store).await.expect("count after B retry"),
        after_b
    );

    assert_eq!(
        store
            .upsert_page(&page_a, USER_MANUAL_VERSION, "current")
            .await
            .expect("commit predecessor-bound second A"),
        (page_id.clone(), true)
    );
    let second_a = store
        .get_page_by_slug(&page_a.slug)
        .await
        .expect("read second A")
        .expect("second A exists")
        .0;
    let second_a_receipt = second_a
        .ledger_event_id
        .clone()
        .expect("second A has canonical receipt");
    assert_ne!(second_a_receipt, first_a_receipt);
    assert_ne!(second_a_receipt, b_receipt);
    let after_second_a = receipt_count(&store).await.expect("count after second A");
    assert_eq!(after_second_a, after_b + 1);
    assert_eq!(
        store
            .upsert_page(&page_a, USER_MANUAL_VERSION, "current")
            .await
            .expect("retry second A"),
        (page_id.clone(), false)
    );
    assert_eq!(
        receipt_count(&store)
            .await
            .expect("count after second A retry"),
        after_second_a
    );

    drop(store);
    drop(storage);
    isolated
        .shutdown_storage_for_reopen()
        .await
        .expect("close exact product-global storage");
    isolated
        .reopen()
        .await
        .expect("reopen exact namespace/database");
    let reopened_storage = isolated
        .activate_storage()
        .await
        .expect("reactivate exact namespace/database");
    assert_eq!(reopened_storage.namespace(), namespace);
    assert_eq!(reopened_storage.database(), database);
    let reopened = UserManualStore::new(reopened_storage.clone());
    let persisted = reopened
        .get_page_by_slug(&page_a.slug)
        .await
        .expect("read A after reopen")
        .expect("A survives reopen")
        .0;
    assert_eq!(persisted.page_id, page_id);
    assert_eq!(persisted.content_hash, page_a.content_hash());
    assert_eq!(
        persisted.ledger_event_id.as_deref(),
        Some(second_a_receipt.as_str())
    );
    assert_eq!(
        reopened
            .upsert_page(&page_a, USER_MANUAL_VERSION, "current")
            .await
            .expect("retry A after reopen"),
        (persisted.page_id, false)
    );
    assert_eq!(
        receipt_count(&reopened)
            .await
            .expect("count after reopen retry"),
        after_second_a
    );

    drop(reopened);
    drop(reopened_storage);
    isolated.cleanup().await.expect("clean exact test scope");
}

/// MT-194: version metadata row carries the corpus hash, counts, and a
/// resolvable EventLedger receipt id.
#[tokio::test]
async fn mt194_seed_records_version_metadata() {
    let scope = UserManualTestScope::create().await;
    let store = scope.store();
    let report = store.ensure_seeded().await.expect("seed");
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

    let exists = receipt_exists(&store, &receipt_id)
        .await
        .expect("receipt lookup");
    assert!(
        exists,
        "version receipt {receipt_id} not in kernel_event_ledger"
    );
}

/// MT-194: page reads return ordered sections and anchors; tampered child
/// rows are healed by reseed even when the page hash still matches.
#[tokio::test]
async fn mt194_sections_ordered_and_tampered_children_heal() {
    let scope = seeded_scope().await;
    let store = scope.store();
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

    // Tamper: delete the page's sections WITHOUT touching the page hash.
    delete_page_sections(&store, &page.page_id)
        .await
        .expect("tamper sections");

    let report = store.ensure_seeded().await.expect("healing reseed");
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

#[tokio::test]
async fn model_lane_user_manual_entries_persist_in_embedded_store() {
    let scope = seeded_scope().await;
    let store = scope.store();

    for slug in [
        "model-lane-schema",
        "model-lane-launch-adapters",
        "model-lane-promotion",
        "model-lane-context-bundle-handoff",
        "model-lane-cloud-projection-consent",
        "model-lane-recovery",
        "model-lane-diagnostics",
        "model-lane-navigation",
        "model-lane-validation-harness",
    ] {
        let (page, sections, anchors) = store
            .get_page_by_slug(slug)
            .await
            .expect("model-lane page query")
            .unwrap_or_else(|| panic!("model-lane UserManual page {slug} is persisted"));
        assert_eq!(page.manual_version, USER_MANUAL_VERSION);
        assert_eq!(page.source_kind, "builtin_seed");
        assert!(
            page.ledger_event_id.is_some(),
            "{slug} must carry EventLedger seed receipt"
        );
        assert!(!sections.is_empty(), "{slug} must persist ordered sections");
        assert!(
            !anchors.is_empty(),
            "{slug} must persist navigation anchors"
        );
    }

    for tool_id in [
        "model_lane_launch_tests",
        "model_lane_launch_tests",
        "worktree_model_lane_live_surreal_tests",
        "worktree_model_lane_live_surreal_tests",
        "cloud_model_lane_policy_surreal_tests",
        "model_lane_cloud_consent_scope_surreal_tests",
        "worktree_model_lane_live_surreal_tests",
        "swarm_lane_diagnostics_runtime_proof",
        "model_lane_navigation_api_tests",
        "worktree_model_lane_live_surreal_tests",
    ] {
        let tool = store
            .get_tool_entry(tool_id)
            .await
            .expect("model-lane tool query")
            .unwrap_or_else(|| panic!("model-lane UserManual tool {tool_id} is persisted"));
        assert_eq!(tool.origin, "wp1_model_lane");
        assert_eq!(tool.manual_version, USER_MANUAL_VERSION);
        assert!(
            tool.schema_fields
                .iter()
                .any(|field| field.contains("Flight Recorder") || field.contains("EventLedger")),
            "{tool_id} must persist Flight Recorder/EventLedger proof fields"
        );
    }

    let mixed_tool = store
        .get_tool_entry("worktree_model_lane_live_surreal_tests")
        .await
        .expect("mixed tool query")
        .expect("mixed model-lane validation tool is persisted");
    assert!(
        mixed_tool
            .schema_fields
            .iter()
            .any(|field| field == "hsk.user_manual_behavior_coverage@1"),
        "mixed validation tool must persist hsk.user_manual_behavior_coverage@1 schema"
    );
    assert!(
        mixed_tool
            .expected_output
            .contains("hsk.user_manual_behavior_coverage@1"),
        "mixed validation tool output must cite the behavior coverage matrix"
    );

    let version = store
        .get_version(USER_MANUAL_VERSION)
        .await
        .expect("version query")
        .expect("UserManual version row exists");
    assert_eq!(version.seed_content_hash, corpus_hash(&seed_corpus()));
    assert!(
        version.ledger_event_id.is_some(),
        "UserManual version row must carry EventLedger receipt"
    );
}

// ---------------------------------------------------------------------------
// MT-195: the build-update rule.
// ---------------------------------------------------------------------------

/// MT-195: in a seeded database, EVERY declared WP-009 surface has at least
/// one http_route anchor on a manual page (build-rule law, spec 10.15.8).
#[tokio::test]
async fn mt195_every_registry_surface_has_db_coverage() {
    let scope = seeded_scope().await;
    let store = scope.store();
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
    let scope = seeded_scope().await;
    let storage = scope.storage();
    let store = scope.store();

    let victim = "/knowledge/code/symbols";
    let deleted = delete_route_anchor(&store, victim)
        .await
        .expect("remove coverage");
    assert!(deleted > 0, "fixture must remove real route coverage");

    let report = check_freshness(&storage).await.expect("freshness");
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
    store.ensure_seeded().await.expect("healing reseed");
    let healed = check_freshness(&storage)
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
    let scope = seeded_scope().await;
    let storage = scope.storage();
    let store = scope.store();

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
        .expect("tool lookup")
        .expect("tool exists");
    tool.description = "tampered visible tool description".to_owned();
    store
        .upsert_tool_entry(&tool)
        .await
        .expect("tamper tool content");
    let mut feature = store
        .get_feature_entry(&feature_id)
        .await
        .expect("feature lookup")
        .expect("feature exists");
    feature.description = "tampered visible feature description".to_owned();
    store
        .upsert_feature_entry(&feature)
        .await
        .expect("tamper feature content");
    let mut alias_row = store
        .get_legacy_alias(&alias)
        .await
        .expect("alias lookup")
        .expect("alias exists");
    alias_row.canonical_ref = "tampered-alias-target".to_owned();
    store
        .upsert_legacy_alias(&alias_row)
        .await
        .expect("tamper alias target");

    let report = check_freshness(&storage).await.expect("freshness");
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

    let healed = store.ensure_seeded().await.expect("healing reseed");
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
    let fresh = check_freshness(&storage)
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
    let scope = seeded_scope().await;
    let store = scope.store();

    let hits = store
        .search("embedded SurrealDB", 25)
        .await
        .expect("search");
    assert!(!hits.is_empty(), "seeded corpus names embedded SurrealDB");
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
