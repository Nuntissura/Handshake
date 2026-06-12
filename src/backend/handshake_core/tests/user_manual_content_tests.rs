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

mod knowledge_pg_support;
#[allow(dead_code)]
mod user_manual_support;

use handshake_core::api;
use handshake_core::kernel::model_manual::kernel002_no_context_model_manual;
use handshake_core::knowledge_document::embed::{BrokenEmbedRepair, EmbedRefKind, EmbedTarget};
use handshake_core::knowledge_retrieval::budget::PriorityTier;
use handshake_core::knowledge_retrieval::compiler::{BundleTargetKind, ContextBundleCompilerV2};
use handshake_core::knowledge_retrieval::plan::RetrievalTrace;
use handshake_core::knowledge_retrieval::planner::{
    AuthoritativeHandle, CheapestAuthoritativePathPlanner, RetrievalRequest,
};
use handshake_core::storage::knowledge::KnowledgeStore;
use handshake_core::user_manual::bundle_bridge::{
    ensure_manual_page_entity, manual_bundle_candidate,
};
use handshake_core::user_manual::fixtures::{delete_page, insert_orphan_page, unreachable_pages};
use handshake_core::user_manual::freshness::{check_freshness, FreshnessVerdictKind};
use handshake_core::user_manual::seed::ensure_seeded;
use handshake_core::user_manual::spec_seed::spec_enrichment_seed;
use handshake_core::user_manual::store::UserManualStore;
use serde_json::{json, Value};
use std::path::PathBuf;
use user_manual_support::{app_state_for, start_server};

// ---------------------------------------------------------------------------
// MT-196.
// ---------------------------------------------------------------------------

/// MT-196 (+ UMMIG-002 mapping law): every kernel002 manual topic and every
/// instruction line is present, verbatim, on the seeded
/// kernel-write-governance page — the legacy struct maps deterministically
/// into UserManual authority.
#[tokio::test]
async fn mt196_kernel002_manual_topics_are_seeded_as_pages() {
    let kpg = skip_if_no_pg!(
        knowledge_pg_support::knowledge_pg().await,
        "mt196_kernel002_import"
    );
    ensure_seeded(&kpg.db).await.expect("seed");
    let store = UserManualStore::new(&kpg.db);
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

/// MT-196: the documented startup constants (bind address, managed-PG port,
/// data dir) match the product SOURCE. Runtime introspection of `main`'s
/// hardcoded bind is impossible from a test, so the consistency check pins
/// the documented values against `src/main.rs` and `src/managed_postgres.rs`
/// — if an operator changes the port, this test forces the manual update.
#[tokio::test]
async fn mt196_documented_startup_constants_match_product_source() {
    let kpg = skip_if_no_pg!(
        knowledge_pg_support::knowledge_pg().await,
        "mt196_startup_constants"
    );
    ensure_seeded(&kpg.db).await.expect("seed");
    let store = UserManualStore::new(&kpg.db);
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
    let managed = std::fs::read_to_string(crate_root.join("src/managed_postgres.rs"))
        .expect("read managed_postgres.rs");

    assert!(page_text.contains("127.0.0.1:37501"));
    assert!(
        main_rs.contains("37501"),
        "main.rs no longer binds 37501 — update startup-and-run-commands"
    );
    assert!(page_text.contains("5544"));
    assert!(
        managed.contains("5544"),
        "managed_postgres.rs default port changed — update the manual"
    );
    assert!(page_text.contains("managed_pgdata"));
    assert!(managed.contains("managed_pgdata"));
    // The documented mounts exist in main.rs (`/api` nest + merge at root).
    assert!(main_rs.contains(".nest(\"/api\", api_routes)"));
}

// ---------------------------------------------------------------------------
// MT-197 / MT-198: documented failure modes triggered at runtime.
// ---------------------------------------------------------------------------

struct RouterFixture {
    kpg: knowledge_pg_support::KnowledgePg,
    base: String,
    _server: tokio::task::JoinHandle<()>,
    http: reqwest::Client,
}

async fn router_fixture() -> Option<RouterFixture> {
    let kpg = knowledge_pg_support::knowledge_pg().await?;
    ensure_seeded(&kpg.db).await.expect("seed");
    let state = app_state_for(&kpg.schema_url).await;
    let (base, server) = start_server(api::routes(state)).await;
    Some(RouterFixture {
        kpg,
        base,
        _server: server,
        http: reqwest::Client::new(),
    })
}

fn doc_headers(req: reqwest::RequestBuilder, actor_kind: &str) -> reqwest::RequestBuilder {
    req.header("x-hsk-actor-id", "um-content-test")
        .header("x-hsk-kernel-task-run-id", "UM-CONTENT")
        .header("x-hsk-session-run-id", "UMS-CONTENT-1")
        .header("x-hsk-actor-kind", actor_kind)
}

/// MT-197/MT-198: the documented failure table is LIVE-VERIFIED — each
/// triggered failure must answer with a code the failure-modes page
/// documents for that surface (doc-vs-runtime in the direction that
/// matters: observed behavior ∈ documented vocabulary).
#[tokio::test]
async fn mt198_documented_failure_modes_match_runtime() {
    let fx = skip_if_no_pg!(router_fixture().await, "mt198_failure_modes");
    let store = UserManualStore::new(&fx.kpg.db);
    let (_, sections, _) = store
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

    // 1) Identity law (code-nav): missing headers -> 400 bad_request.
    let nav = fx
        .http
        .get(format!("{}/knowledge/code/symbols?workspace_id=x&name=y", fx.base))
        .send()
        .await
        .expect("nav probe");
    assert_eq!(nav.status(), 400);
    let nav_body: Value = nav.json().await.expect("nav json");
    let nav_code = nav_body["error"].as_str().unwrap();
    assert!(
        documented("code_nav", nav_code),
        "code-nav answered '{nav_code}' which the failure page does not document"
    );

    // 2) Permission law (documents): cloud_model write -> 403 forbidden with
    //    the documented stable reason.
    let workspace_id = fx.kpg.create_workspace().await;
    let create = doc_headers(
        fx.http.post(format!("{}/knowledge/documents", fx.base)),
        "operator",
    )
    .json(&json!({"workspace_id": workspace_id, "title": "doc-vs-runtime"}))
    .send()
    .await
    .expect("create document");
    assert_eq!(create.status(), 200, "operator create must succeed");
    let created: Value = create.json().await.expect("create json");
    let document_id = created["document"]["rich_document_id"]
        .as_str()
        .expect("rich_document_id in create response")
        .to_string();

    let denied = doc_headers(
        fx.http
            .put(format!("{}/knowledge/documents/{document_id}/save", fx.base)),
        "cloud_model",
    )
    .json(&json!({"expected_version": 1, "content_json": {"type": "doc", "content": []}}))
    .send()
    .await
    .expect("cloud_model save");
    assert_eq!(denied.status(), 403, "cloud_model write must be denied (documented)");
    let denied_body: Value = denied.json().await.expect("denied json");
    let denied_code = denied_body["error"].as_str().unwrap();
    assert!(
        documented("documents", denied_code),
        "documents surface answered '{denied_code}' undocumented"
    );
    assert!(
        denied_body.to_string().contains("cloud_model"),
        "denial must carry the documented stable cloud_model reason: {denied_body}"
    );

    // 3) Concurrency law (documents): stale expected_version -> 409 conflict.
    let conflict = doc_headers(
        fx.http
            .put(format!("{}/knowledge/documents/{document_id}/save", fx.base)),
        "operator",
    )
    .json(&json!({"expected_version": 999, "content_json": {"type": "doc", "content": []}}))
    .send()
    .await
    .expect("stale save");
    assert_eq!(conflict.status(), 409, "stale expected_version must 409 (documented)");
    let conflict_body: Value = conflict.json().await.expect("conflict json");
    assert!(documented("documents", conflict_body["error"].as_str().unwrap()));

    // 4) UserManual surface: unknown slug -> 404 not_found (documented).
    let manual_missing = fx
        .http
        .get(format!("{}/usermanual/pages/zzz-missing", fx.base))
        .send()
        .await
        .expect("manual missing");
    assert_eq!(manual_missing.status(), 404);
    let manual_body: Value = manual_missing.json().await.expect("manual 404 json");
    assert!(documented("usermanual", manual_body["error"].as_str().unwrap()));
}

/// MT-198: the documented embed law and repair-action vocabulary match the
/// live types exactly (4 typed constructor errors; relink|reresolve|remove).
#[tokio::test]
async fn mt198_embed_law_and_repair_vocabulary_match_types() {
    let kpg = skip_if_no_pg!(
        knowledge_pg_support::knowledge_pg().await,
        "mt198_embed_vocab"
    );
    ensure_seeded(&kpg.db).await.expect("seed");
    let store = UserManualStore::new(&kpg.db);
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
// MT-202: real compiled bundle cites a manual page.
// ---------------------------------------------------------------------------

/// MT-202: compile a REAL context bundle (real planner, real compiler, real
/// rows) whose candidate is a UserManual page; the persisted item citation
/// carries slug + manual version + section anchor + drift hash, and the
/// bundle target kind is user_manual_page.
#[tokio::test]
async fn mt202_bundle_cites_manual_page_with_version_and_anchor() {
    let kpg = skip_if_no_pg!(knowledge_pg_support::knowledge_pg().await, "mt202_bundle");
    ensure_seeded(&kpg.db).await.expect("seed");
    let workspace_id = kpg.create_workspace().await;
    let store = UserManualStore::new(&kpg.db);
    let (page, sections, _) = store
        .get_page_by_slug("state-recovery-guide")
        .await
        .expect("page query")
        .expect("state-recovery-guide seeded");
    let section = &sections[0];

    let entity = ensure_manual_page_entity(&kpg.db, &workspace_id, &page)
        .await
        .expect("manual page entity");
    assert_eq!(entity.entity_kind.as_str(), "user_manual_page");

    // Idempotent: re-mirroring keeps the entity id stable.
    let entity_again = ensure_manual_page_entity(&kpg.db, &workspace_id, &page)
        .await
        .expect("re-mirror");
    assert_eq!(entity.entity_id, entity_again.entity_id);

    let planner = CheapestAuthoritativePathPlanner::new(&kpg.db);
    let request = RetrievalRequest::discovery(&workspace_id, "how do I recover state")
        .with_handle(AuthoritativeHandle::EntityId(entity.entity_id.clone()));
    let planned = planner.plan(&request).await.expect("plan");
    let mut trace = RetrievalTrace::for_plan(&planned.plan);

    let candidate = manual_bundle_candidate(
        &page,
        section,
        &entity.entity_id,
        PriorityTier::Authoritative,
        40,
        0.95,
    );
    let compiled = ContextBundleCompilerV2::new(&kpg.db)
        .compile(
            &workspace_id,
            "ktr-um-202",
            "sr-um-202",
            BundleTargetKind::UserManualPage,
            &page.slug,
            &planned.plan,
            &mut trace,
            &[candidate],
            None,
            None,
        )
        .await
        .expect("compile bundle");

    let (bundle, items) = kpg
        .db
        .get_knowledge_context_bundle(&compiled.bundle_id)
        .await
        .expect("load bundle")
        .expect("bundle persisted");
    assert_eq!(items.len(), 1);
    let citation = items[0].citation.as_deref().expect("manual citation persisted");
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
        citation.contains(&page.content_hash[..8]),
        "citation must carry the drift hash prefix: {citation}"
    );
    // The bundle itself is targeted at the manual page.
    assert!(bundle.allowed_context.to_string().contains("user_manual_page"));
}

// ---------------------------------------------------------------------------
// MT-206 / MT-207.
// ---------------------------------------------------------------------------

/// MT-206: the state-recovery guide covers all four contract scenarios.
#[tokio::test]
async fn mt206_state_recovery_guide_covers_contract_scenarios() {
    let kpg = skip_if_no_pg!(
        knowledge_pg_support::knowledge_pg().await,
        "mt206_state_recovery"
    );
    ensure_seeded(&kpg.db).await.expect("seed");
    let store = UserManualStore::new(&kpg.db);
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

// ---------------------------------------------------------------------------
// MT-208 fixtures (the families not already driven by the API tests).
// ---------------------------------------------------------------------------

/// MT-208: missing-page fixture — deleting a seeded page yields the
/// missing_page freshness verdict, and reseeding restores it.
#[tokio::test]
async fn mt208_missing_page_fixture_detected_and_healed() {
    let kpg = skip_if_no_pg!(
        knowledge_pg_support::knowledge_pg().await,
        "mt208_missing_page"
    );
    ensure_seeded(&kpg.db).await.expect("seed");
    assert_eq!(delete_page(&kpg.db, "quickstart-editor").await.expect("delete"), 1);

    let report = check_freshness(&kpg.db).await.expect("freshness");
    assert!(!report.fresh);
    assert!(
        report.verdicts.iter().any(|v| {
            v.kind == FreshnessVerdictKind::MissingPage && v.subject == "quickstart-editor"
        }),
        "missing page must be detected"
    );

    ensure_seeded(&kpg.db).await.expect("healing reseed");
    let store = UserManualStore::new(&kpg.db);
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
    let kpg = skip_if_no_pg!(
        knowledge_pg_support::knowledge_pg().await,
        "mt208_legacy_redirect"
    );
    ensure_seeded(&kpg.db).await.expect("seed");
    let store = UserManualStore::new(&kpg.db);
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
    let kpg = skip_if_no_pg!(knowledge_pg_support::knowledge_pg().await, "mt208_orphan");
    ensure_seeded(&kpg.db).await.expect("seed");
    assert!(
        unreachable_pages(&kpg.db).await.expect("audit").is_empty(),
        "seeded corpus must have no orphans"
    );
    let orphan_slug = insert_orphan_page(&kpg.db).await.expect("insert orphan");
    let orphans = unreachable_pages(&kpg.db).await.expect("audit 2");
    assert!(
        orphans.contains(&orphan_slug),
        "navigation audit must flag the orphan (got {orphans:?})"
    );
}
